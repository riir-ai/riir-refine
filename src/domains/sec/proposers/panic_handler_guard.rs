//! `panic-handler-nested-catch-unwind` — SUGGEST-ONLY detector
//! (Batch 325, the mozilla/uniffi-rs FFI-hygiene distill; Research 361 /
//! Plan 327).
//!
//! An error-REPORTING handler that itself does fallible work (`format!`
//! / alloc) must carry its OWN `catch_unwind` + graceful degradation —
//! skip the out-param set and let the empty-buffer fallback speak. A
//! panic escaping the handler path during an active unwind double-panics,
//! and a panic from the reporting path masks the ORIGINAL error. The
//! handler path (the fn invoked to render/report/record errors) is
//! unowned by existing rules: `ffi-extern-fn-catch-unwind` owns the
//! extern BODY wrapper; `drop-impl-no-panic` owns Drop bodies.
//!
//! ## Why suggest-only (never a fix)
//!
//! The graceful-degradation choice (what the fallback does — skip the
//! write? log-and-continue?) is a protocol decision. NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval and review.
//!
//! ## The bounded form
//!
//! ```text
//! fn report_error(err: &Error, out: &mut Buffer) {
//!     let _ = std::panic::catch_unwind(|| format!("{err}")); // the handler guards ITSELF
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Handler vocabulary (gate a)**: the fn name carries
//!   error/report/fault/panic/failure vocabulary (`report` / `handler` /
//!   `dump` segments; the compound `on_error` / `handle_error` /
//!   `write_error` / `render_error` / `fmt_error` names arrive through
//!   their `error` segment), OR the SOLE param's type is named
//!   `*Error*` / `Panic*` / `Fault*`. A fn named `compute_total` stays
//!   silent whatever its body does.
//! - **Fallible work without a guard (gates b+c)**: the body contains a
//!   `format!` / `write!` / `writeln!` macro, a `.to_string()` call, or
//!   a `collect::<String>` — AND no `catch_unwind` anywhere in the fn.
//! - **Handler shape (gate d, conservative proxy)**: the fn takes a
//!   `&mut` buffer/out param (or `&mut self`), or returns a
//!   `String`/`bool` success flag, or its body carries `set_out`/
//!   out-param-write vocabulary.
//! - **Suppression screens** (run on the whole-body token stream — the
//!   conservative direction; detection itself never descends into macro
//!   tokens): any `catch_unwind` / `panic::set_hook` / pause-spawner
//!   token in the fn stays silent; a body that is a single `log::`/
//!   `eprintln!` call stays silent (logging paths already degrade).
//! - `#[cfg(test)]` mods are skipped; `Stmt::Macro` is never descended
//!   into (house discipline).
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **Per-fn only**: the ACTUAL unwind context (is the handler invoked
//!   during a panic?) is invisible — the rule flags the hardening
//!   QUESTION, not a definite bug (suggest-only is the blast-radius
//!   control).
//! - Handlers that cannot alloc (no `format!`/`to_string` in the body)
//!   are out by gate b.
//! - Cross-fn handler chains (report in a helper, the guard in the
//!   caller) unconnected.
//! - Distinct-from `ffi-extern-fn-catch-unwind` (owns the extern body
//!   wrapper, unit-return; this owns the handler PATH by vocabulary) and
//!   `drop-impl-no-panic` (Drop bodies).
//! - Batch 325 / Plan 327 / Research 361 (mozilla/uniffi-rs @
//!   ef9625b68dd51207f55f024a27649e58edb52b09, MPL-2.0 — cited as class
//!   provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, FnArg, Item, Stmt, Type};

/// One detected unguarded fallible site on an error-handler path.
#[derive(Debug, Clone)]
pub struct HandlerGuardFinding {
    /// The handler fn doing fallible work without its own guard.
    pub fn_name: String,
    /// Source line of the fallible site (`format!` / `.to_string()` / ..).
    pub line: usize,
    /// The flagged shape (retrieval token).
    pub kind: &'static str,
}

/// The flagged shape (single source, tested).
pub const KIND: &str = "handler-path-unguarded-fallible-work";

/// Handler vocabulary — `_`-segments of the fn name (compound
/// `on_error` / `handle_error` / `write_error` / `render_error` /
/// `fmt_error` names arrive through their `error` segment).
const HANDLER_VOCAB: &[&str] = &[
    "report", "handler", "dump", "error", "panic", "fault", "failure",
];

/// Macro paths counting as fallible reporting work (gate b).
const FALLIBLE_MACROS: &[&str] = &["format", "write", "writeln"];

/// Whole-body tokens meaning the fn already guards itself or is panic
/// infrastructure (gate c's suppression screen, conservative direction).
const GUARD_TOKENS: &[&str] = &["catch_unwind", "set_hook", "pause"];

/// Macro paths that are plain logging — such a body already degrades.
const LOG_MACROS: &[&str] = &[
    "eprintln", "eprint", "println", "print", "info", "warn", "debug", "trace", "error",
];

/// Detect unguarded fallible work on error-handler paths (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<HandlerGuardFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<HandlerGuardFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.sig, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.sig, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production handler paths.
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// Fallible-site scan state for one fn body.
#[derive(Default)]
struct FallibleScan {
    /// Line of the first fallible site.
    line: Option<usize>,
    /// Byte end of that site (span sanity vs the source).
    end: usize,
}

fn scan_fn(
    source: &str,
    sig: &syn::Signature,
    block: &syn::Block,
    findings: &mut Vec<HandlerGuardFinding>,
) {
    let name = sig.ident.to_string();
    if single_log_body(&block.stmts) {
        return; // a logging-only body already degrades gracefully
    }
    let body_ts = block.to_token_stream().to_string();
    if GUARD_TOKENS.iter().any(|w| contains_word(&body_ts, w)) {
        return; // the handler already guards itself (or is panic infra)
    }
    if !name_vocab(&name) && !sole_param_errorish(sig) {
        return; // gate a: handler vocabulary, or none of this rule's business
    }
    let mut scan = FallibleScan::default();
    scan_stmts(&block.stmts, &mut scan);
    let Some(line) = scan.line else {
        return; // gate b: no alloc-class fallible work
    };
    if scan.end > source.len() {
        return;
    }
    let shaped = takes_mut_param(sig) || returns_flag(sig) || contains_word(&body_ts, "set_out");
    if !shaped {
        return; // gate d: not shaped as a handler
    }
    findings.push(HandlerGuardFinding {
        fn_name: name,
        line,
        kind: KIND,
    });
}

/// A body that is ONLY `log::`-class / `eprintln!` macro statements —
/// logging paths already degrade (spec suppression).
fn single_log_body(stmts: &[Stmt]) -> bool {
    let mut count = 0;
    for stmt in stmts {
        match stmt {
            Stmt::Macro(m) => {
                if !last_seg_in(&m.mac.path, LOG_MACROS) {
                    return false;
                }
            }
            Stmt::Expr(e, _) => {
                let Expr::Macro(m) = e else {
                    return false;
                };
                if !last_seg_in(&m.mac.path, LOG_MACROS) {
                    return false;
                }
            }
            _ => return false,
        }
        count += 1;
    }
    count > 0
}

fn last_seg_in(path: &syn::Path, set: &[&str]) -> bool {
    path.segments
        .last()
        .is_some_and(|s| set.contains(&s.ident.to_string().as_str()))
}

fn name_vocab(name: &str) -> bool {
    name.to_ascii_lowercase()
        .split('_')
        .any(|seg| HANDLER_VOCAB.contains(&seg))
}

/// The SOLE param's type named `*Error*` / `Panic*` / `Fault*`.
fn sole_param_errorish(sig: &syn::Signature) -> bool {
    let mut typed = 0;
    let mut errorish = false;
    for arg in &sig.inputs {
        let FnArg::Typed(p) = arg else {
            continue;
        };
        typed += 1;
        errorish |= type_name_vocab(&p.ty);
    }
    typed == 1 && errorish
}

fn type_name_vocab(ty: &Type) -> bool {
    let Type::Path(p) = ty else {
        return false;
    };
    p.path.segments.last().is_some_and(|s| {
        let n = s.ident.to_string();
        n.contains("Error") || n.contains("Panic") || n.contains("Fault")
    })
}

/// Gate d proxy: a `&mut` param (or `&mut self`).
fn takes_mut_param(sig: &syn::Signature) -> bool {
    sig.inputs.iter().any(|arg| match arg {
        FnArg::Typed(p) => matches!(&*p.ty, Type::Reference(r) if r.mutability.is_some()),
        FnArg::Receiver(r) => r.reference.is_some() && r.mutability.is_some(),
    })
}

/// Gate d proxy: a `String`/`bool` success-flag return.
fn returns_flag(sig: &syn::Signature) -> bool {
    match &sig.output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ty) => matches!(
            &**ty,
            Type::Path(p)
                if p.path.segments.last().is_some_and(|s| s.ident == "String" || s.ident == "bool")
        ),
    }
}

fn scan_stmts(stmts: &[Stmt], scan: &mut FallibleScan) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(&init.expr, scan);
                }
            }
            Stmt::Item(item) => scan_nested_item(item, scan),
            Stmt::Expr(e, _) => scan_expr(e, scan),
            Stmt::Macro(m) => {
                if last_seg_in(&m.mac.path, FALLIBLE_MACROS) {
                    note_site(m, scan);
                }
            }
        }
    }
}

fn scan_nested_item(item: &Item, scan: &mut FallibleScan) {
    match item {
        Item::Fn(f) => scan_stmts(&f.block.stmts, scan),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_stmts(&m.block.stmts, scan);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    scan_nested_item(nested, scan);
                }
            }
        }
        _ => {}
    }
}

fn scan_expr(e: &Expr, scan: &mut FallibleScan) {
    match e {
        Expr::Macro(m) => {
            if last_seg_in(&m.mac.path, FALLIBLE_MACROS) {
                note_site(e, scan);
            }
        }
        Expr::MethodCall(m) => {
            let collect_string = m.method == "collect"
                && m.turbofish
                    .as_ref()
                    .is_some_and(|tf| tf.to_token_stream().to_string().contains("String"));
            if m.method == "to_string" || collect_string {
                note_site(e, scan);
            }
            scan_expr(&m.receiver, scan);
            for arg in &m.args {
                scan_expr(arg, scan);
            }
        }
        Expr::Closure(cl) => scan_expr(&cl.body, scan),
        Expr::Unary(u) => scan_expr(&u.expr, scan),
        Expr::Paren(p) => scan_expr(&p.expr, scan),
        Expr::Reference(r) => scan_expr(&r.expr, scan),
        Expr::Field(f) => scan_expr(&f.base, scan),
        Expr::Binary(b) => {
            scan_expr(&b.left, scan);
            scan_expr(&b.right, scan);
        }
        Expr::Call(c) => {
            scan_expr(&c.func, scan);
            for arg in &c.args {
                scan_expr(arg, scan);
            }
        }
        Expr::Assign(a) => {
            scan_expr(&a.left, scan);
            scan_expr(&a.right, scan);
        }
        Expr::Index(ix) => {
            scan_expr(&ix.expr, scan);
            scan_expr(&ix.index, scan);
        }
        Expr::If(i) => {
            scan_expr(&i.cond, scan);
            scan_stmts(&i.then_branch.stmts, scan);
            if let Some((_, else_e)) = &i.else_branch {
                scan_expr(else_e, scan);
            }
        }
        Expr::Match(m) => {
            scan_expr(&m.expr, scan);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    scan_expr(guard, scan);
                }
                scan_expr(&arm.body, scan);
            }
        }
        Expr::Block(b) => scan_stmts(&b.block.stmts, scan),
        Expr::Unsafe(u) => scan_stmts(&u.block.stmts, scan),
        Expr::Loop(l) => scan_stmts(&l.body.stmts, scan),
        Expr::While(w) => {
            scan_expr(&w.cond, scan);
            scan_stmts(&w.body.stmts, scan);
        }
        Expr::ForLoop(fl) => {
            scan_expr(&fl.expr, scan);
            scan_stmts(&fl.body.stmts, scan);
        }
        Expr::Return(r) => {
            if let Some(v) = &r.expr {
                scan_expr(v, scan);
            }
        }
        Expr::Try(t) => scan_expr(&t.expr, scan),
        Expr::Await(aw) => scan_expr(&aw.base, scan),
        Expr::Let(l) => scan_expr(&l.expr, scan),
        _ => {}
    }
}

fn note_site<T: ToTokens>(site: &T, scan: &mut FallibleScan) {
    if scan.line.is_some() {
        return;
    }
    let site_span = site.span();
    scan.line = Some(site_span.start().line);
    scan.end = site_span.byte_range().end;
}

/// Word-bounded token search (ident-boundary guards: `pause` must not
/// match `paused`).
fn contains_word(hay: &str, needle: &str) -> bool {
    if hay.len() < needle.len() {
        return false;
    }
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(off) = hay[from..].find(needle) {
        let start = from + off;
        let end = start + needle.len();
        let before = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before && after {
            return true;
        }
        from = end;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::HandlerGuardFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    #[test]
    fn buggy_handler_no_catch_unwind_fires() {
        let src = "fn report_error(err: &E, out: &mut String) {\n\
            \x20   out.push_str(&format!(\"{err}\"));\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the unguarded reporting path fires");
        assert_eq!(f[0].fn_name, "report_error");
        assert!(f[0].line >= 1, "the fallible site line is reported");
    }

    #[test]
    fn guarded_handler_silent() {
        let src = "fn report_error(err: &E, out: &mut String) {\n\
            \x20   if let Ok(s) = std::panic::catch_unwind(|| format!(\"{err}\")) {\n\
            \x20       out.push_str(&s);\n\
            \x20   }\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "the handler already guards itself"
        );
    }

    #[test]
    fn plain_logger_silent() {
        let src = "fn report_error(err: &E, out: &mut String) {\n\
            \x20   eprintln!(\"error: {err}\");\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "a single eprintln body already degrades"
        );
    }

    #[test]
    fn unrelated_fn_silent() {
        let src = "fn compute_total(items: &[u64]) -> String {\n\
            \x20   format!(\"{}\", items.len())\n\
            }\n";
        assert!(findings(src).is_empty(), "no handler vocabulary, no fire");
    }

    #[test]
    fn test_mod_skipped() {
        let src = "#[cfg(test)]\n\
            mod tests {\n\
            \x20   fn report_error(err: &E, out: &mut String) {\n\
            \x20       out.push_str(&format!(\"{err}\"));\n\
            \x20   }\n\
            }\n";
        assert!(findings(src).is_empty(), "fixture habitat stays silent");
    }
}
