//! `out-param-manuallydrop-not-maybeuninit` — SUGGEST-ONLY detector
//! (Batch 325, the mozilla/uniffi-rs FFI-hygiene distill; Research 361 /
//! Plan 327).
//!
//! A caller-initialized `#[repr(C)]` out-param crossing an FFI boundary
//! is `&mut ManuallyDrop<T>` — safe reads, explicit drop discipline,
//! set-twice = a visible single-writer leak — NOT `&mut MaybeUninit<T>`,
//! where every read is `unsafe` and the initialized-contract is
//! implicit. Two-regime: when the ABI guarantees caller init,
//! `ManuallyDrop` is the honest contract; when the CALLEE owns
//! initialization (the caller passes uninitialized scratch), `MaybeUninit`
//! is correct there. This rule flags the CALLER-INITIALIZED-CONTRACT
//! sites that chose `MaybeUninit` anyway — the uniffi `rustcalls.rs:42-53`
//! shape: the repr(C) out-param taken as `&mut MaybeUninit` while the
//! caller contract initializes it.
//!
//! ## Why suggest-only (never a fix)
//!
//! Switching the param type changes the ABI contract shared with foreign
//! callers — a protocol decision, never mechanical. NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval and review.
//!
//! ## The bounded form
//!
//! ```text
//! unsafe extern "C" fn call(out: &mut ManuallyDrop<RustBuffer>) { ... } // caller-init contract, safe reads
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Extern gate (a)**: an `unsafe extern "C"` fn (or a
//!   `#[no_mangle] pub extern "C"` fn) carrying a `&mut MaybeUninit<..>`
//!   parameter; a plain (non-extern) fn stays silent.
//! - **Full-write gate (b)**: the fn body WRITES the whole value — an
//!   assignment `*out = ...` through the param, or `out.write(...)` on
//!   it — i.e. the callee fully initializes, so the caller-side scratch
//!   could have been `ManuallyDrop` with the init-contract stated. Only
//!   PARTIAL writes (field assigns, `(*p).len = ...` through a raw
//!   pointer hop) stay silent — partial-init is `MaybeUninit`'s legitimate
//!   home (and a `MaybeUninit::uninit()` / `assume_init` reader body is
//!   silent by the same gate).
//! - **Literal type-name gate (c)**: the param's type name must be
//!   literally `MaybeUninit` (last path segment) — non-repr(C) generics
//!   narrowed by construction.
//! - **Incremental-fill suppression**: any `for`/`while`/`loop` whose
//!   body mentions the param suppresses (array-ish fills write
//!   element-by-element; `MaybeUninit` is correct there). The screen runs
//!   on the loop body's token stream (the conservative direction);
//!   detection itself never descends into macro tokens (`Stmt::Macro`
//!   is skipped — house discipline).
//! - `#[cfg(test)]` mods are skipped — fixture habitat, not production
//!   FFI surfaces.
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **Per-fn only**: the FOREIGN caller's actual init behavior is
//!   invisible from the callee body — the rule flags the CONTRACT
//!   question, not a definite bug (suggest-only is the blast-radius
//!   control).
//! - `MaybeUninit::write` through an ALIASED pointer (a raw pointer
//!   taken from `as_mut_ptr`) and the free-fn form
//!   `MaybeUninit::write(out, v)` are out — only the `*out = ...` assign
//!   and the `out.write(...)` method shape.
//! - Cross-fn contracts (init in a helper, the extern signature here)
//!   unconnected.
//! - Distinct-from the inline-const lifetime-retarget rule (the corpus's
//!   only ManuallyDrop/MaybeUninit neighbor — a different decision:
//!   init-ownership vs lifetime) and `slice-from-raw-parts-len-guard`
//!   (owns slice len, not out-param contracts).
//! - Batch 325 / Plan 327 / Research 361 (mozilla/uniffi-rs @
//!   ef9625b68dd51207f55f024a27649e58edb52b09, MPL-2.0 — cited as class
//!   provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, FnArg, Item, Pat, Stmt, Type, UnOp};

/// One detected `MaybeUninit` out-param on a caller-initialized FFI contract.
#[derive(Debug, Clone)]
pub struct OutparamInitFinding {
    /// The extern fn carrying the `&mut MaybeUninit` out-param.
    pub fn_name: String,
    /// Source line of the full-write site (`*out = ..` / `out.write(..)`).
    pub line: usize,
    /// The flagged shape (retrieval token).
    pub kind: &'static str,
}

/// The flagged shape (single source, tested).
pub const KIND: &str = "caller-init-repr-c-outparam-maybeuninit";

/// Detect `MaybeUninit` out-params on caller-init FFI contracts
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<OutparamInitFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<OutparamInitFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.attrs, &f.sig, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.attrs, &m.sig, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production FFI surfaces.
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

/// Gate (a): an `unsafe extern "C"` fn (or a `#[no_mangle] extern "C"` fn).
fn extern_c_surface(attrs: &[syn::Attribute], sig: &syn::Signature) -> bool {
    let abi_c = sig
        .abi
        .as_ref()
        .is_some_and(|a| a.name.as_ref().is_some_and(|n| n.value() == "C"));
    if !abi_c {
        return false;
    }
    sig.unsafety.is_some() || attrs.iter().any(|a| a.path().is_ident("no_mangle"))
}

/// The first `&mut MaybeUninit<..>` param's binding name, if any.
fn maybeuninit_out_param(sig: &syn::Signature) -> Option<String> {
    sig.inputs.iter().find_map(|arg| {
        let FnArg::Typed(p) = arg else {
            return None;
        };
        let name = pat_ident(&p.pat)?;
        is_mut_ref_maybeuninit(&p.ty).then_some(name)
    })
}

fn pat_ident(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(pi) => Some(pi.ident.to_string()),
        _ => None,
    }
}

fn is_mut_ref_maybeuninit(ty: &Type) -> bool {
    let Type::Reference(r) = ty else {
        return false;
    };
    r.mutability.is_some() && last_type_segment(&r.elem).as_deref() == Some("MaybeUninit")
}

fn last_type_segment(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        Type::Paren(p) => last_type_segment(&p.elem),
        Type::Group(g) => last_type_segment(&g.elem),
        _ => None,
    }
}

/// Write-site scan state for one fn body.
struct WriteScan {
    /// The `&mut MaybeUninit` param's binding name.
    param: String,
    /// Line of the first whole-value write (`*out = ..` / `out.write(..)`).
    full_line: Option<usize>,
    /// Byte end of that site (span sanity vs the source).
    full_end: usize,
    /// A loop body mentions the param: an array-ish incremental fill.
    loop_writes_param: bool,
}

impl WriteScan {
    fn new(param: &str) -> Self {
        Self {
            param: param.to_string(),
            full_line: None,
            full_end: 0,
            loop_writes_param: false,
        }
    }
}

fn scan_fn(
    source: &str,
    attrs: &[syn::Attribute],
    sig: &syn::Signature,
    block: &syn::Block,
    findings: &mut Vec<OutparamInitFinding>,
) {
    if !extern_c_surface(attrs, sig) {
        return;
    }
    let Some(param) = maybeuninit_out_param(sig) else {
        return;
    };
    let mut scan = WriteScan::new(&param);
    scan_stmts(&block.stmts, &mut scan);
    let Some(line) = scan.full_line else {
        return; // partial-inits and uninit-readers stay silent
    };
    if scan.full_end > source.len() {
        return;
    }
    if scan.loop_writes_param {
        return; // array-ish incremental fill: MaybeUninit's legitimate home
    }
    findings.push(OutparamInitFinding {
        fn_name: sig.ident.to_string(),
        line,
        kind: KIND,
    });
}

fn scan_stmts(stmts: &[Stmt], scan: &mut WriteScan) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(&init.expr, scan);
                }
            }
            Stmt::Item(item) => scan_nested_item(item, scan),
            Stmt::Expr(e, _) => scan_expr(e, scan),
            Stmt::Macro(_) => {} // no macro descent (house discipline)
        }
    }
}

fn scan_nested_item(item: &Item, scan: &mut WriteScan) {
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

fn scan_expr(e: &Expr, scan: &mut WriteScan) {
    match e {
        Expr::Assign(a) => {
            classify_write_target(&a.left, scan);
            scan_expr(&a.right, scan);
        }
        Expr::MethodCall(m) => {
            if m.method == "write"
                && m.args.len() == 1
                && scan.full_line.is_none()
                && resolves_to_param(&m.receiver, &scan.param)
            {
                record_full(e, scan);
            }
            scan_expr(&m.receiver, scan);
            for arg in &m.args {
                scan_expr(arg, scan);
            }
        }
        Expr::ForLoop(fl) => {
            note_loop(&fl.body, scan);
            scan_expr(&fl.expr, scan);
            scan_stmts(&fl.body.stmts, scan);
        }
        Expr::While(w) => {
            note_loop(&w.body, scan);
            scan_expr(&w.cond, scan);
            scan_stmts(&w.body.stmts, scan);
        }
        Expr::Loop(l) => {
            note_loop(&l.body, scan);
            scan_stmts(&l.body.stmts, scan);
        }
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
        Expr::Closure(cl) => scan_expr(&cl.body, scan),
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
        Expr::Return(r) => {
            if let Some(v) = &r.expr {
                scan_expr(v, scan);
            }
        }
        Expr::Try(t) => scan_expr(&t.expr, scan),
        Expr::Await(aw) => scan_expr(&aw.base, scan),
        _ => {}
    }
}

/// Gate (b): whole-value writes only. A field assign (`(*out).len = ..`)
/// is a partial init and records nothing.
fn classify_write_target(left: &Expr, scan: &mut WriteScan) {
    let Expr::Unary(u) = left else {
        return;
    };
    if !matches!(u.op, UnOp::Deref(_)) {
        return;
    }
    if scan.full_line.is_some() {
        return;
    }
    if resolves_to_param(&u.expr, &scan.param) {
        record_full(left, scan);
    }
}

fn record_full(site: &Expr, scan: &mut WriteScan) {
    let write_span = site.span();
    scan.full_line = Some(write_span.start().line);
    scan.full_end = write_span.byte_range().end;
}

fn note_loop(body: &syn::Block, scan: &mut WriteScan) {
    let ts = body.to_token_stream().to_string();
    if contains_word(&ts, &scan.param) {
        scan.loop_writes_param = true;
    }
}

fn resolves_to_param(e: &Expr, param: &str) -> bool {
    match e {
        Expr::Path(p) => p.path.segments.last().is_some_and(|s| s.ident == param),
        Expr::Paren(p) => resolves_to_param(&p.expr, param),
        Expr::Reference(r) => resolves_to_param(&r.expr, param),
        _ => false,
    }
}

/// Word-bounded token search (ident-boundary guards: param `out` must
/// not match `output`).
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

    fn findings(src: &str) -> Vec<super::OutparamInitFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    #[test]
    fn buggy_extern_maybeuninit_outparam_fires() {
        let src = "#[no_mangle]\n\
            pub extern \"C\" fn fill(out: &mut MaybeUninit<u64>) {\n\
            \x20   unsafe { *out = MaybeUninit::new(7) };\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the caller-init MaybeUninit out-param fires");
        assert_eq!(f[0].fn_name, "fill");
        assert!(f[0].line >= 2, "the full-write site line is reported");
    }

    #[test]
    fn caller_init_contract_manuallydrop_silent() {
        let src = "#[no_mangle]\n\
            pub extern \"C\" fn fill(out: &mut ManuallyDrop<u64>) {\n\
            \x20   unsafe { *out = ManuallyDrop::new(7) };\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "the ManuallyDrop twin is already the honest contract"
        );
    }

    #[test]
    fn partial_field_writes_silent() {
        let src = "#[repr(C)]\n\
            struct Buf { len: u64, cap: u64 }\n\
            #[no_mangle]\n\
            pub extern \"C\" fn partial(out: &mut MaybeUninit<Buf>) {\n\
            \x20   let p = out.as_mut_ptr();\n\
            \x20   unsafe {\n\
            \x20       (*p).len = 4;\n\
            \x20       (*p).cap = 8;\n\
            \x20   }\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "field-level partial init is MaybeUninit's legitimate home"
        );
    }

    #[test]
    fn non_extern_silent() {
        let src = "fn fill_local(out: &mut MaybeUninit<u64>) {\n\
            \x20   unsafe { *out = MaybeUninit::new(3) };\n\
            }\n";
        assert!(findings(src).is_empty(), "no extern surface, no fire");
    }

    #[test]
    fn test_mod_skipped() {
        let src = "#[cfg(test)]\n\
            mod tests {\n\
            \x20   #[test]\n\
            \x20   fn t() {\n\
            \x20       let mut slot = MaybeUninit::<u64>::uninit();\n\
            \x20       fill(&mut slot);\n\
            \x20   }\n\
            \x20   #[no_mangle]\n\
            \x20   pub extern \"C\" fn fill(out: &mut MaybeUninit<u64>) {\n\
            \x20       unsafe { *out = MaybeUninit::new(7) };\n\
            \x20   }\n\
            }\n";
        assert!(findings(src).is_empty(), "fixture habitat stays silent");
    }
}
