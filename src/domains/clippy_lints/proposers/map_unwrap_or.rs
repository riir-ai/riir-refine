//! `clippy::map_unwrap_or` — `x.map(f).unwrap_or(d)` → `x.map_or(d, f)`, or
//! `is_some_and(<F>)` when the default is `false` and the closure provably
//! returns bool (real clippy 1.97's own suggestion for that shape — its
//! `map_or` output would be re-flagged by `unnecessary_map_or`, a style
//! lint, so the fused form is NOT the terminal state there).
//!
//! The `unwrap_or_else` arm (riir-train slice 8, 2026-08-18): real clippy
//! fires the SAME lint on `x.map(f).unwrap_or_else(g)` and suggests
//! `x.map_or_else(g, f)` — default closure FIRST, mapper SECOND. Both
//! evaluate both closures lazily, so the fused form preserves semantics
//! for ANY shapes. The bool-literal special cases do not apply (the
//! default is a closure, never a bool literal).
//!
//! `Option::map_or(default, f)` is the fused form of `map(f).unwrap_or(d)`:
//! both evaluate `d` eagerly and `f` lazily, so the rewrite preserves
//! semantics for ANY default/mapper shapes (no purity requirement).
//! Span-preserving (T6.4): the whole `unwrap_or`/`unwrap_or_else` call's
//! byte range is replaced by `<recv-of-map>.map_or(<D>, <F>)` (or
//! `map_or_else(<G>, <F>)`), each fragment being the ORIGINAL source text
//! of the matched node.
//!
//! Bool-default terminal states (clippy 1.93 probe, 2026-08-18 evening):
//! a provably-bool closure with `false`/`true` defaults emits the TERMINAL
//! form directly — `is_some_and(<f>)` / `is_none_or(<f>)`. The earlier
//! `true`-default SKIP was based on the two-step clippy chain
//! (`map_or(true, f)` → `is_none_or`), but emitting the terminal state
//! directly is one clean step (probe: both outputs lint-clean on 1.93;
//! `is_none_or` stable since 1.82). Closures whose bool-ness cannot be
//! proven syntactically (e.g. a bare path) keep the `map_or` form — a
//! parse-only limitation, not a wrong fix.

use super::helpers::is_postfix_safe;
use super::uninlined_format_args::post_template_expr_args;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, ExprMacro, File, Macro, StmtMacro};
use proc_macro2::TokenStream;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        param_kinds: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Enclosing-fn parameter types (Option/Result classification for
    /// plain-path receivers of the bool-literal terminal forms). Stack of
    /// scopes — the Issue 031 H1 enclosing-fn-stack pattern.
    param_kinds: Vec<std::collections::HashMap<String, ReceiverKind>>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.param_kinds.push(fn_param_kinds(&f.sig));
        syn::visit::visit_item_fn(self, f);
        self.param_kinds.pop();
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.param_kinds.push(fn_param_kinds(&f.sig));
        syn::visit::visit_impl_item_fn(self, f);
        self.param_kinds.pop();
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = try_map_or(self.source, m, self.innermost_params()) {
            self.edits.push(edit(m, replacement));
        }
    }

    /// Macro arguments are OPAQUE token streams to syn — the default
    /// visitor never descends into them, so `.map(f).unwrap_or_else(g)`
    /// inside a `writeln!`/`println!`/`format!` arg (the `bench_241` shape)
    /// is invisible to the method-call visit above. Real clippy matches
    /// there (post-expansion HIR); we parse the expression args of the
    /// known format-family macros + run the same matcher over them. The
    /// cloned tokens keep their file-relative spans, so edits collected
    /// on the parsed args slice the ORIGINAL source correctly.
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.scan_macro_args(&m.mac);
    }

    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.scan_macro_args(&m.mac);
    }
}

impl Collector<'_> {
    fn scan_macro_args(&mut self, mac: &Macro) {
        let Some(args) = post_template_expr_args(mac) else {
            return;
        };
        let params = self.innermost_params().cloned();
        for arg in args {
            let stream: TokenStream = arg.toks.iter().cloned().collect();
            let Ok(expr) = syn::parse2::<Expr>(stream) else {
                continue; // named args / non-expression tokens — skip
            };
            scan_expr_for_matches(self.source, &mut self.edits, &expr, params.as_ref());
        }
    }

    fn innermost_params(&self) -> Option<&std::collections::HashMap<String, ReceiverKind>> {
        self.param_kinds.last()
    }
}

/// Walk one macro-arg expression for `map(..).unwrap_or(_or_else)(..)`
/// matches. The lifetimes tie the AST borrow to the source/edits borrows
/// (all share the caller's frame); spans are file-relative by token
/// round-trip, so `src`/`edit` operate on the WHOLE-file source.
fn scan_expr_for_matches(
    source: &str,
    edits: &mut Vec<SourceEdit>,
    expr: &Expr,
    params: Option<&std::collections::HashMap<String, ReceiverKind>>,
) {
    struct Sub<'a> {
        source: &'a str,
        edits: &'a mut Vec<SourceEdit>,
        params: Option<&'a std::collections::HashMap<String, ReceiverKind>>,
    }
    impl<'ast> Visit<'ast> for Sub<'ast> {
        fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
            syn::visit::visit_expr_method_call(self, m);
            if let Some(replacement) = try_map_or(self.source, m, self.params) {
                self.edits.push(edit(m, replacement));
            }
        }
    }
    let mut sub = Sub { source, edits, params };
    sub.visit_expr(expr);
}

/// Extract Option/Result classification from a fn signature's parameter
/// types. Only simple `ident: Type` patterns (incl. `&`-wrapped) are
/// recorded — complex destructuring patterns are skipped (Unknown).
fn fn_param_kinds(sig: &syn::Signature) -> std::collections::HashMap<String, ReceiverKind> {
    let mut out = std::collections::HashMap::new();
    for arg in sig.inputs.iter() {
        let syn::FnArg::Typed(pt) = arg else { continue };
        let syn::Pat::Ident(id) = &*pt.pat else { continue };
        let kind = param_type_kind(&pt.ty);
        if let Some(kind) = kind {
            out.insert(id.ident.to_string(), kind);
        }
    }
    out
}

fn param_type_kind(ty: &syn::Type) -> Option<ReceiverKind> {
    let ty = match ty {
        syn::Type::Reference(r) => &*r.elem,
        _ => ty,
    };
    let syn::Type::Path(tp) = ty else { return None };
    // Last path segment: `Option` / `Result` bare or `std::option::Option`.
    // Generic args are ignored — `Option<T>` is OptionLike for every T.
    match tp.path.segments.last().map(|s| s.ident.to_string()).as_deref() {
        Some("Option") => Some(ReceiverKind::OptionLike),
        Some("Result") => Some(ReceiverKind::ResultLike),
        _ => None,
    }
}

/// `x.map(f).unwrap_or(d)` → `x.map_or(d, f)` / `is_some_and(f)`;
/// `x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)`.
fn try_map_or(
    source: &str,
    m: &ExprMethodCall,
    params: Option<&std::collections::HashMap<String, ReceiverKind>>,
) -> Option<String> {
        let else_form = if m.method == "unwrap_or" {
            false
        } else if m.method == "unwrap_or_else" {
            true
        } else {
            return None;
        };
        if !m.attrs.is_empty() || m.turbofish.is_some() {
            return None;
        }
        if m.args.len() != 1 {
            return None;
        }
        let Expr::MethodCall(map) = &*m.receiver else {
            return None; // receiver isn't a method call
        };
        if !map.attrs.is_empty() || map.turbofish.is_some() || map.method != "map" {
            return None;
        }
        if map.args.len() != 1 {
            return None;
        }
        let recv = src(source, &*map.receiver)?;
        let default = src(source, &m.args[0])?;
        let mapper = src(source, &map.args[0])?;
        // Array-literal defaults (`&[...]`, incl. `&[]`) must stay on the
        // `map(f).unwrap_or(&[...])` form: `map_or`'s U unification needs the
        // CALL SITE to force the type for the unsize coercion `&[T; N] ->
        // &[T]`. In inference-transparent positions (plain `let`, generic
        // args) rustc fixes U from the array literal and the closure's slice
        // return E0308s — probe-verified 2026-08-18: `map_or(&[], |m|
        // m.as_slice())` AND `map_or(&[1.0], ...)` both fail in `let`
        // position while both compile context-forced; the original always
        // compiles. The proposer cannot see call-site context, so skip the
        // shape entirely (joins the documented we-stay-silent class).
        if is_array_lit_default(&m.args[0], else_form) {
            return None;
        }
        let recv = if is_postfix_safe(&map.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        if else_form {
            // `x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)` — real
            // clippy's own suggestion for the else arm; default closure
            // FIRST, mapper SECOND. No bool-literal special cases (the
            // default is always a closure here).
            return Some(format!("{recv}.map_or_else({default}, {mapper})"));
        }
        let bool_default = bool_literal(&m.args[0]);
        let mapper_is_bool = closure_body_is_bool(&map.args[0]);
        if let Some(default) = bool_default {
            if mapper_is_bool {
                // Terminal forms fire only on PROVABLY-typed receivers:
                // `is_some_and`/`is_none_or` are Option-only, `is_ok_and` is
                // Result-only — the wrong pick is a COMPILE error (E0599, the
                // bench_440 env::var catch, 2026-08-18). Unknown receivers
                // fall back to the always-compiling `map_or` (one step short
                // of real clippy's terminal suggestion — the documented
                // SAFER-than-clippy divergence, same class as the type-blind
                // lint demotions).
                match classify_receiver(&recv, params) {
                    ReceiverKind::OptionLike => {
                        if default {
                            // clippy chain: `map_or(true, f)` → `is_none_or`
                            // (probe-verified lint-clean on 1.93).
                            return Some(format!("{recv}.is_none_or({mapper})"));
                        }
                        // Real clippy 1.93's own suggestion for this shape:
                        // the `map_or` output would be re-flagged by
                        // `unnecessary_map_or` (a default-group lint).
                        return Some(format!("{recv}.is_some_and({mapper})"));
                    }
                    ReceiverKind::ResultLike => {
                        if default {
                            // Real clippy is SILENT on Result + true default
                            // (probe 2026-08-18: no `is_err_or` suggestion —
                            // not stable); `map_or(true, f)` IS the terminal
                            // state for this receiver kind.
                            return Some(format!("{recv}.map_or(true, {mapper})"));
                        }
                        // Real clippy suggests `is_ok_and` directly for
                        // `Result.map(f).unwrap_or(false)` (probe-verified:
                        // skips the map_or intermediate entirely).
                        return Some(format!("{recv}.is_ok_and({mapper})"));
                    }
                    ReceiverKind::Unknown => {
                        // Cannot prove Option vs Result — emit the form that
                        // compiles on BOTH. May leave an `unnecessary_map_or`
                        // warning for Option receivers, but never breaks the
                        // build (compile-safety over terminal completeness).
                        return Some(format!("{recv}.map_or({default}, {mapper})"));
                    }
                }
            }
        }
        Some(format!("{recv}.map_or({default}, {mapper})"))
}

/// Parse-only receiver classification for the bool-literal terminal forms.
/// Last-match-wins over the receiver text: a chain like
/// `env::var(k).ok()` is Option-producing (`.ok()` converts to Option), so
/// the OUTERMOST (last) known-shape call decides. Evidence-based allowlists:
/// Option shapes are the dominant `.get(`/`.find(` accessors; Result shapes
/// are the std APIs seen in production heals (`env::var`, `.parse`).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ReceiverKind {
    OptionLike,
    ResultLike,
    Unknown,
}

fn classify_receiver(recv: &str, params: Option<&std::collections::HashMap<String, ReceiverKind>>) -> ReceiverKind {
    // A plain-path receiver typed by an enclosing fn parameter: the parse
    // tree's own type annotation is authoritative (`fn f(x: Option<u8>)`).
    if !recv.contains('.') && !recv.contains('(') {
        if let Some(kind) = params.and_then(|p| p.get(recv.trim())) {
            return *kind;
        }
        return ReceiverKind::Unknown;
    }
    const OPTION_SHAPES: [&str; 7] = [
        ".get(", ".find(", ".first(", ".last(", ".pop()", ".next()", ".ok()",
    ];
    const RESULT_SHAPES: [&str; 3] = ["env::var(", ".parse(", ".parse::<"];
    let mut best: Option<(usize, ReceiverKind)> = None;
    for shape in OPTION_SHAPES {
        if let Some(pos) = recv.rfind(shape) {
            if best.is_none_or(|(p, _)| pos > p) {
                best = Some((pos, ReceiverKind::OptionLike));
            }
        }
    }
    for shape in RESULT_SHAPES {
        if let Some(pos) = recv.rfind(shape) {
            if best.is_none_or(|(p, _)| pos > p) {
                best = Some((pos, ReceiverKind::ResultLike));
            }
        }
    }
    best.map(|(_, kind)| kind).unwrap_or(ReceiverKind::Unknown)
}

/// True for `&[...]` / `&mut [...]` defaults (recursing through parens).
/// Casts are safe — `as` ascribes the target type, resolving the coercion
/// locally.
fn is_array_lit_ref(e: &Expr) -> bool {
    match e {
        Expr::Reference(r) => matches!(&*r.expr, Expr::Array(_)),
        Expr::Paren(p) => is_array_lit_ref(&p.expr),
        _ => false,
    }
}

/// Array-literal default check across both rewrite forms: direct for
/// `unwrap_or` (`&[...]`), inside the default closure's body for
/// `unwrap_or_else` (`|| &[...]`).
fn is_array_lit_default(arg: &Expr, else_form: bool) -> bool {
    if else_form {
        match arg {
            Expr::Closure(c) => is_array_lit_ref(&c.body),
            _ => false,
        }
    } else {
        is_array_lit_ref(arg)
    }
}

/// `Some(false)`/`Some(true)` for a `false`/`true` literal, else `None`.
fn bool_literal(e: &Expr) -> Option<bool> {
    match e {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Bool(b) => Some(b.value),
            _ => None,
        },
        Expr::Paren(p) => bool_literal(&p.expr),
        _ => None,
    }
}

/// Syntactically PROVE the expression is bool-typed. Conservative: only
/// shapes that are bool whenever the code compiles (comparisons, `!`,
/// `&&`/`||`, bool literals) plus a small allowlist of bool-returning method
/// names. Unknown shapes return false (the proposer then fires `map_or`,
/// which is real clippy's suggestion for everything it cannot see through).
fn expr_is_bool(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(&l.lit, syn::Lit::Bool(_)),
        Expr::Paren(p) => expr_is_bool(&p.expr),
        Expr::Unary(u) => matches!(u.op, syn::UnOp::Not(_)),
        Expr::Binary(b) => matches!(
            b.op,
            syn::BinOp::And(_)
                | syn::BinOp::Or(_)
                | syn::BinOp::Eq(_)
                | syn::BinOp::Ne(_)
                | syn::BinOp::Lt(_)
                | syn::BinOp::Le(_)
                | syn::BinOp::Gt(_)
                | syn::BinOp::Ge(_)
        ),
        Expr::MethodCall(m) => is_bool_method(&m.method),
        // `matches!(e, p)` evaluates to a bool by definition (post-expansion
        // it's an `is`-match expression) — found as live residue in katgpt
        // percepta's `.map(|d| matches!(d.kind, ..)).unwrap_or(true)`.
        Expr::Macro(mac) => mac.mac.path.is_ident("matches"),
        _ => false,
    }
}

fn is_bool_method(name: &syn::Ident) -> bool {
    let n = name.to_string();
    n.starts_with("is_")
        || matches!(
            n.as_str(),
            "contains" | "contains_key" | "starts_with" | "ends_with" | "any" | "all" | "eq"
                | "ne" | "lt" | "le" | "gt" | "ge"
        )
}

/// The mapper arg must be a closure whose body provably returns bool.
fn closure_body_is_bool(arg: &Expr) -> bool {
    let Expr::Closure(c) = arg else {
        return false;
    };
    expr_is_bool(&c.body)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn fuses_map_unwrap_or() {
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v + 1)"),
            "map_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_receiver_preserved() {
        let input = "fn f(v: Vec<u8>) -> u8 { v.first().map(|x| *x + 1).unwrap_or(9) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("v.first().map_or(9, |x| *x + 1)"),
            "chain receiver text not preserved: {out}"
        );
    }

    #[test]
    fn non_map_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-map receiver must not fire: {out:?}");
    }

    #[test]
    fn other_method_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.filter(|v| *v > 0).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "filter receiver must not fire: {out:?}");
    }

    // Bool-default terminal states (Issue 027 probe class + 2026-08-18
    // evening re-probe on clippy 1.93; live residue in katgpt percepta):
    // - `.map(bool-f).unwrap_or(false)` → `is_some_and(f)` (the `map_or`
    //   output is re-flagged by the DEFAULT-group `unnecessary_map_or`);
    // - `.map(bool-f).unwrap_or(true)` → `is_none_or(f)` — the terminal
    //   form of clippy's two-step chain, emitted directly (the old skip
    //   left the original warned shape in place).
    // Array-literal defaults stay on the unwrap_or form (E0308 class,
    // probe-verified 2026-08-18 — see the guard comment in `try_map_or`).
    // The sibling katgpt-rs heal WIP broke on exactly this shape at
    // katgpt-forward/src/step.rs:196 (`map_or(&[], |m| m.as_slice())`).
    #[test]
    fn empty_array_lit_default_skipped() {
        let input =
            "fn f(m: Vec<Vec<f32>>) -> usize { m.get(1).map(|s| s.as_slice()).unwrap_or(&[]).len() }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "empty `&[]` default must not fire: {out:?}");
    }

    #[test]
    fn nonempty_array_lit_default_skipped() {
        let input =
            "fn f(m: Vec<Vec<f32>>) -> usize { m.first().map(|s| s.as_slice()).unwrap_or(&[1.0]).len() }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-empty `&[1.0]` default must not fire: {out:?}");
    }

    #[test]
    fn or_else_array_lit_body_skipped() {
        let input =
            "fn f(m: Vec<Vec<f32>>) -> usize { m.get(1).map(|s| s.as_slice()).unwrap_or_else(|| &[]).len() }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "`|| &[]` default-closure body must not fire: {out:?}");
    }

    #[test]
    fn bool_closure_false_default_becomes_is_some_and() {
        let input =
            "fn f(x: Option<(bool, bool)>) -> bool { x.map(|s| !s.0 && !s.1).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|s| !s.0 && !s.1)"),
            "is_some_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bool_closure_true_default_becomes_is_none_or() {
        let input =
            "fn f(x: Option<(bool, bool)>) -> bool { x.map(|s| s.0 || s.1).unwrap_or(true) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_none_or(|s| s.0 || s.1)"),
            "is_none_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn contains_key_ref_pattern_closure_becomes_is_some_and() {
        // Live shape from katgpt percepta scheduler.rs:550 — the ref-pattern
        // closure `|&op| k.contains_key(&op)` over `Option<&T>`.
        let input = "fn f(x: Option<u32>, k: std::collections::HashMap<u32, u32>) -> bool { x.map(|&op| k.contains_key(&op)).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|&op| k.contains_key(&op))"),
            "is_some_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn matches_macro_closure_true_default_becomes_is_none_or() {
        // Live shape from katgpt percepta graph/types/mod.rs:1170 —
        // `.map(|d| matches!(d.kind, DimensionKind::Input)).unwrap_or(true)`.
        let input = "enum K { Input } fn f(x: Option<u32>) -> bool { x.map(|d| matches!(d, 1..=3)).unwrap_or(true) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_none_or(|d| matches!(d, 1..=3))"),
            "is_none_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comparison_closure_false_default_becomes_is_some_and() {
        let input = "fn f(x: Option<u8>) -> bool { x.map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|v| v > 3)"),
            "is_some_and not produced: {out}"
        );
    }

    // Result-receiver terminal states (bench_440 catch, 2026-08-18: the
    // heal emitted `is_some_and` on a `std::env::var` Result receiver —
    // E0599 compile break). Probe-verified on clippy 1.97: Result
    // receivers get `is_ok_and` directly; Result + true default is
    // clippy-SILENT (no stable `is_err_or`), so `map_or(true, f)` IS the
    // terminal state there. Unknown receivers (no param type, no known
    // shape) fall back to the always-compiling `map_or`.
    #[test]
    fn env_var_receiver_becomes_is_ok_and() {
        // Live shape from katgpt-core bench_440_lllg_paper_repro.rs:456.
        let input = "fn f() -> bool { std::env::var(\"LLLG_MULTISTEP\").map(|v| v == \"1\" || v.eq_ignore_ascii_case(\"true\")).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("std::env::var(\"LLLG_MULTISTEP\").is_ok_and(|v|"),
            "is_ok_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn parse_receiver_becomes_is_ok_and() {
        let input = "fn f() -> bool { \"5\".parse::<i32>().map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("\"5\".parse::<i32>().is_ok_and(|v| v > 3)"),
            "is_ok_and not produced: {out}"
        );
    }

    #[test]
    fn result_param_true_default_stays_map_or() {
        // Real clippy is SILENT on Result + true default (probe: no
        // `is_err_or` suggestion — not stable); `map_or(true, f)` is the
        // terminal state. Emitting `is_none_or` would be an E0599.
        let input = "fn f(x: Result<u8, u8>) -> bool { x.map(|v| v > 3).unwrap_or(true) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(true, |v| v > 3)"),
            "map_or(true, ..) not produced: {out}"
        );
    }

    #[test]
    fn result_param_false_default_becomes_is_ok_and() {
        let input = "fn f(x: Result<u8, u8>) -> bool { x.map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_ok_and(|v| v > 3)"),
            "is_ok_and not produced: {out}"
        );
    }

    #[test]
    fn unknown_receiver_falls_back_to_map_or() {
        // A receiver the classifier cannot prove Option vs Result (custom
        // fn call, no param type): the always-compiling `map_or` — never a
        // terminal form that could be an E0599 on the wrong receiver kind.
        let input = "fn f(k: u32) -> bool { lookup(k).map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("lookup(k).map_or(false, |v| v > 3)"),
            "map_or fallback not produced: {out}"
        );
        assert!(
            !out.contains("is_some_and"),
            "terminal form on unknown receiver must not be produced: {out}"
        );
    }

    #[test]
    fn bool_method_closure_false_default_becomes_is_some_and() {
        let input = "fn f(x: Option<String>) -> bool { x.map(|s| s.contains('a')).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|s| s.contains('a'))"),
            "is_some_and not produced: {out}"
        );
    }

    #[test]
    fn unprovable_closure_false_default_keeps_map_or() {
        // A bare-path body's bool-ness is invisible parse-only → the
        // conservative `map_or` form (real clippy's suggestion for what it
        // can see through type-independently).
        let input = "fn f(x: Option<bool>, b: bool) -> bool { x.map(|v| v).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(false, |v| v)"),
            "map_or not produced: {out}"
        );
    }

    #[test]
    fn non_bool_default_with_bool_shaped_body_still_fuses() {
        // `0` is not a bool literal → the fused map_or form regardless of
        // the closure body's shape (type-mismatched bool body is impossible).
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v + 1)"),
            "map_or not produced: {out}"
        );
    }

    // `unwrap_or_else` arm (riir-train slice 8): same lint, suggestion
    // `map_or_else(<default-closure>, <mapper>)` — arg ORDER is the
    // load-bearing fact (default first).
    #[test]
    fn fuses_map_unwrap_or_else_default_first() {
        let input =
            "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or_else(|| 0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or_else(|| 0, |v| v + 1)"),
            "map_or_else not produced with default FIRST: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn or_else_chained_receiver_preserved() {
        // The bench_241 shape: a field-chain receiver under the map.
        let input = "fn f(s: S) -> String { s.opt.step.map(|v| v.to_string()).unwrap_or_else(|| \"none\".into()) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.opt.step.map_or_else(|| \"none\".into(), |v| v.to_string())"),
            "chained receiver text not preserved: {out}"
        );
    }

    #[test]
    fn or_else_non_map_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or_else(|| 0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-map receiver must not fire: {out:?}");
    }

    #[test]
    fn or_else_non_closure_default_still_fuses() {
        // A function-path default (`unwrap_or_else(Default::default)` shape)
        // is still the fused form — map_or_else takes any callable.
        let input =
            "fn f(x: Option<Vec<u8>>) -> Vec<u8> { x.map(|v| v.clone()).unwrap_or_else(Vec::new) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or_else(Vec::new, |v| v.clone())"),
            "map_or_else not produced for path default: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    // Macro-argument descent (riir-train slice 8, bench_241 shape): the
    // `.map(f).unwrap_or_else(g)` sites live inside `writeln!` args —
    // opaque tokens to syn's visitor, visible only via the format-family
    // macro scan.
    #[test]
    fn or_else_inside_writeln_args_fires() {
        let input = "fn f(o: impl std::fmt::Write, s: S) { \
            writeln!(o, \"| {} |\", s.step.map(|v| v.to_string()).unwrap_or_else(|| \"never\".into())).unwrap(); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.step.map_or_else(|| \"never\".into(), |v| v.to_string())"),
            "macro-arg match not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn or_else_multiline_writeln_chain_fires() {
        // The exact bench_241 shape: a multi-line field-chain receiver
        // inside a multi-line writeln! statement.
        let input = "fn f(w: W, s: S) { \
            writeln!( \
                w, \
                \"| Convergence | {} |\", \
                s.with.convergence_step \
                    .map(|st| format!(\"step {st}\")) \
                    .unwrap_or_else(|| \"never\".into()) \
            ).unwrap(); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("map_or_else(|| \"never\".into(), |st| format!(\"step {st}\"))"),
            "multiline macro-arg match not produced: {out}"
        );
        // The receiver text survives verbatim (newlines included).
        assert!(
            out.contains("s.with.convergence_step"),
            "receiver text not preserved: {out}"
        );
    }

    #[test]
    fn plain_map_unwrap_or_inside_println_fires() {
        let input = "fn f(s: S) { println!(\"{}\", s.opt.map(|v| v + 1).unwrap_or(0)); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.opt.map_or(0, |v| v + 1)"),
            "macro-arg map_or not produced: {out}"
        );
    }

    #[test]
    fn no_false_fire_on_clean_macro_args() {
        // A clean arg (no map/unwrap chain) + a named arg that doesn't
        // parse as an expression — neither fires, and the named arg does
        // not abort scanning of later args.
        let input = "fn f(s: S) { println!(\"{a} {b}\", a = 1, b = s.v.map(|x| x).unwrap_or(2)); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.v.map_or(2, |x| x)"),
            "arg after a named arg not scanned: {out}"
        );
    }
}
