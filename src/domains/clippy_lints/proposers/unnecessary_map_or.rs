//! `clippy::unnecessary_map_or` — `x.map_or(false, f)` → `x.is_some_and(f)`
//! (P15 post-mining intake, 2026-09-14 — the born-lint closure's consumer).
//!
//! The healer's own `map_unwrap_or` fallback emits `map_or(false, f)` at
//! type-UNKNOWN receivers (the always-compiling choice), and real clippy
//! immediately re-flags that output — the gen-8 shaped-verdict family's
//! fourth measured member. This rule closes the loop for every receiver the
//! shared [`super::receiver_kind`] substrate can prove.
//!
//! Probe evidence (clippy 1.98, `.raw/mapor_probe`, 2026-09-14 — all
//! `MachineApplicable` method-name suggestions):
//!
//! | receiver | default | fires? | terminal |
//! |---|---|---|---|
//! | Option | `false` | yes | `is_some_and(f)` |
//! | Option | `true` | yes | `is_none_or(f)` — `map_or(true, f) ≡ is_none_or(f)` exactly (no negation requirement) |
//! | Result | `false` | yes | `is_ok_and(f)` |
//! | Result | `true` | **SILENT** | no std terminal form (`is_err_or` does not exist) — `map_or(true, f)` IS terminal |
//! | any | `map_or_else` | **SILENT** | out of clippy's envelope (neither `unnecessary_map_or` nor `unnecessary_lazy_evaluations`) — we mirror |
//!
//! Terminal-form semantics are exact: `map_or` and the terminal twins take
//! `self`/`f` identically (same move, same lazy `f`, literal default), so
//! the rewrite is semantics-preserving for EVERY compiling input with a
//! classified receiver. MSRV floors: `is_some_and`/`is_ok_and` 1.70,
//! `is_none_or` 1.82 (the workspace floats 1.98.1; real clippy is
//! MSRV-aware on the suggest path).
//!
//! Documented divergences (real clippy fires, we refuse):
//!
//! - **Unknown receivers** — a bare local from a foreign call
//!   (`lookup(k).map_or(false, ..)`) or an unclassified initializer:
//!   `is_some_and` on a `Result` is a hard E0599 (the `bench_440` class), so
//!   the type-blind standalone NEVER guesses. CLOSED (P21, 2026-09-14): in
//!   `--clippy-json` mode the `--fix` lane consumes the diagnostic's own
//!   `MachineApplicable` suggestion children verbatim as one atomic edit
//!   (clippy's suggestion is type-aware — `clippy_feed::oracle_edits`).
//! - **Result + `true`** — clippy itself is silent; we refuse too
//!   (`map_or(true, f)` is already terminal for Result).
//! - **Macro-argument sites outside the format family** — syn cannot
//!   descend into `assert!(x.map_or(false, f))` token streams (the
//!   standing P13 divergence class); format-family macro args ARE scanned
//!   via [`super::helpers::macro_expr_args`].

use std::collections::HashMap;

use super::helpers::{bool_lit, is_postfix_safe, macro_expr_args};
use super::receiver_kind::{classify_receiver, collect_fn_rets, ReceiverKind, ScopeKinds};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        scopes: Vec::new(),
        fn_rets: collect_fn_rets(file),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Enclosing-fn scope stack (params + safe-lets) — the Issue 031 H1
    /// enclosing-fn-stack pattern, shared substrate since P15.
    scopes: Vec<ScopeKinds>,
    fn_rets: HashMap<String, ReceiverKind>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.scopes.push(ScopeKinds::from_fn(
            &f.sig,
            &f.block,
            self.source,
            &self.fn_rets,
        ));
        syn::visit::visit_item_fn(self, f);
        self.scopes.pop();
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.scopes.push(ScopeKinds::from_fn(
            &f.sig,
            &f.block,
            self.source,
            &self.fn_rets,
        ));
        syn::visit::visit_impl_item_fn(self, f);
        self.scopes.pop();
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) =
            try_unnecessary_map_or(self.source, m, self.scopes.last(), Some(&self.fn_rets))
        {
            self.edits.push(edit(m, replacement));
        }
    }

    /// Macro arguments are OPAQUE token streams to syn — the format-family
    /// scan (the `map_unwrap_or` macro lane, shared via helpers): parse the
    /// post-template expression args and run the same matcher over them.
    fn visit_expr_macro(&mut self, m: &'ast syn::ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.scan_macro_args(&m.mac);
    }

    fn visit_stmt_macro(&mut self, m: &'ast syn::StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.scan_macro_args(&m.mac);
    }
}

impl Collector<'_> {
    fn scan_macro_args(&mut self, mac: &syn::Macro) {
        let scope = self.scopes.last().cloned();
        for expr in macro_expr_args(mac) {
            scan_expr_for_matches(
                self.source,
                &mut self.edits,
                &expr,
                scope.as_ref(),
                &self.fn_rets,
            );
        }
    }
}

/// Walk one macro-arg expression for `map_or(bool-literal, f)` matches (the
/// `map_unwrap_or` `Sub` pattern; spans are file-relative by token round-trip,
/// so `src`/`edit` operate on the WHOLE-file source).
fn scan_expr_for_matches(
    source: &str,
    edits: &mut Vec<SourceEdit>,
    expr: &Expr,
    scope: Option<&ScopeKinds>,
    fn_rets: &HashMap<String, ReceiverKind>,
) {
    struct Sub<'a> {
        source: &'a str,
        edits: &'a mut Vec<SourceEdit>,
        scope: Option<&'a ScopeKinds>,
        fn_rets: &'a HashMap<String, ReceiverKind>,
    }
    impl<'ast> Visit<'ast> for Sub<'ast> {
        fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
            syn::visit::visit_expr_method_call(self, m);
            if let Some(replacement) =
                try_unnecessary_map_or(self.source, m, self.scope, Some(self.fn_rets))
            {
                self.edits.push(edit(m, replacement));
            }
        }
    }
    let mut sub = Sub {
        source,
        edits,
        scope,
        fn_rets,
    };
    sub.visit_expr(expr);
}

/// `x.map_or(false, f)` → `x.is_some_and(f)` — classified receivers only.
fn try_unnecessary_map_or(
    source: &str,
    m: &ExprMethodCall,
    scope: Option<&ScopeKinds>,
    fn_rets: Option<&HashMap<String, ReceiverKind>>,
) -> Option<String> {
    if m.method != "map_or" {
        return None;
    }
    if !m.attrs.is_empty() || m.turbofish.is_some() {
        return None;
    }
    if m.args.len() != 2 {
        return None;
    }
    let default_true = bool_lit(&m.args[0])?;
    let recv_text = src(source, &m.receiver)?;
    let mapper = src(source, &m.args[1])?;
    // Classify on the RAW receiver text (pre-paren-wrap; a bare ident is
    // always postfix-safe, so wrapping never changes classification).
    let kind = classify_receiver(&recv_text, scope, fn_rets);
    let recv = if is_postfix_safe(&m.receiver) {
        recv_text
    } else {
        format!("({recv_text})")
    };
    match kind {
        ReceiverKind::OptionLike => {
            if default_true {
                Some(format!("{recv}.is_none_or({mapper})"))
            } else {
                Some(format!("{recv}.is_some_and({mapper})"))
            }
        }
        ReceiverKind::ResultLike if !default_true => Some(format!("{recv}.is_ok_and({mapper})")),
        // ResultLike + true: real clippy is SILENT (no std terminal form) —
        // `map_or(true, f)` is already the terminal state for Result.
        // Unknown: type-blind standalone is E0599-unsafe — the documented
        // divergence; the --clippy-json oracle lane is the close.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn option_param_false_becomes_is_some_and() {
        let input = "fn f(x: Option<u8>) -> bool { x.map_or(false, |v| v > 1) }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|v| v > 1)"),
            "is_some_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn option_param_true_becomes_is_none_or() {
        // Probe L27: the PLAIN true-default shape fires too —
        // map_or(true, f) ≡ is_none_or(f) exactly, no negation needed.
        let input = "fn f(x: Option<u8>) -> bool { x.map_or(true, |v| v > 1) }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("x.is_none_or(|v| v > 1)"),
            "is_none_or not produced: {out}"
        );
    }

    #[test]
    fn result_param_false_becomes_is_ok_and() {
        let input = "fn f(r: Result<u8, ()>) -> bool { r.map_or(false, |v| v > 1) }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("r.is_ok_and(|v| v > 1)"),
            "is_ok_and not produced: {out}"
        );
    }

    #[test]
    fn result_param_true_refused() {
        // Probe L22: real clippy SILENT on Result + true — we mirror.
        let input = "fn f(r: Result<u8, ()>) -> bool { r.map_or(true, |v| v > 1) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "Result+true must not fire: {out:?}");
    }

    #[test]
    fn unknown_receiver_refused() {
        // The documented divergence: type-blind standalone is E0599-unsafe.
        let input = "fn f(k: u32) -> bool { lookup(k).map_or(false, |v| v > 1) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "unknown receiver must not fire: {out:?}");
    }

    #[test]
    fn born_lint_shape_safe_let_becomes_is_some_and() {
        // The intake's production shape (seal-game-editor e2112bce): the
        // receiver is a LOCAL binding — safe-let classification (P15) proves
        // it Option through the `.find(` initializer.
        let input = "fn f(v: &[u8]) -> bool { let best = v.iter().find(|x| **x > 1); best.map_or(false, |x| *x > 2) }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("best.is_some_and(|x| *x > 2)"),
            "safe-let terminal not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_receiver_get_becomes_is_some_and() {
        let input = "fn f(m: std::collections::HashMap<u8, u8>) -> bool { m.get(&1).map_or(false, |v| *v > 1) }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("m.get(&1).is_some_and(|v| *v > 1)"),
            "chain terminal not produced: {out}"
        );
    }

    #[test]
    fn fn_path_mapper_transfers_verbatim() {
        // Probe L68: a fn-reference mapper (not a closure) fires; the mapper
        // text transfers unchanged (identical FnOnce bound). Receiver via
        // `.get(` so the shape allowlist classifies it.
        let input = "fn f(m: std::collections::HashMap<u8, u8>) -> bool { fn pred(v: &u8) -> bool { *v > 1 } m.get(&1).map_or(false, pred) }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("m.get(&1).is_some_and(pred)"),
            "fn-path mapper not transferred: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn as_ref_chain_receiver_refused() {
        // A REAL divergence: `.as_ref()` is kind-preserving on BOTH Option
        // and Result — no text shape can classify it (adding it to
        // OPTION_SHAPES would misclassify `r.as_ref()` on Result → E0599).
        // Unknown → refused; the oracle-fed lane covers these sites.
        let input = "fn f(x: Option<u8>) -> bool { fn pred(v: &u8) -> bool { *v > 1 } x.as_ref().map_or(false, pred) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "as_ref chain must not fire: {out:?}");
    }

    #[test]
    fn same_file_fn_receiver_becomes_is_some_and() {
        // The git_out class through THIS rule's door: pre-existing
        // `map_or(false, ..)` at a same-file-fn receiver.
        let input = concat!(
            "fn pick(k: u32) -> Option<u8> { None }\n",
            "fn f(k: u32) -> bool { pick(k).map_or(false, |v| v > 1) }",
        );
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("pick(k).is_some_and(|v| v > 1)"),
            "same-file fn terminal not produced: {out}"
        );
    }

    #[test]
    fn nested_sites_both_edited() {
        let input = "fn f(a: Option<u8>, b: Result<u8, ()>) -> bool { a.map_or(false, |x| x > 1) && b.map_or(false, |y| y > 2) }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(out.contains("a.is_some_and(|x| x > 1)"), "site 1: {out}");
        assert!(out.contains("b.is_ok_and(|y| y > 2)"), "site 2: {out}");
    }

    #[test]
    fn map_or_else_not_the_method() {
        // Probe L32: clippy is SILENT on map_or_else under both lints —
        // different method name, never matched.
        let input = "fn f(x: Option<u8>) -> bool { x.map_or_else(|| false, |v| v > 1) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "map_or_else must not fire: {out:?}");
    }

    #[test]
    fn non_bool_default_refused() {
        let input = "fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "non-bool default must not fire: {out:?}");
    }

    #[test]
    fn turbofish_call_refused() {
        let input = "fn f(x: Option<u8>) -> bool { x.map_or::<bool>(false, |v| v > 1) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "turbofish must not fire: {out:?}");
    }

    #[test]
    fn three_args_refused() {
        // Not a real map_or shape (compile error in practice) — refuse.
        let input = "fn f(x: Option<u8>) -> bool { x.map_or(false, |v| v > 1, 2) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "3-arg call must not fire: {out:?}");
    }

    #[test]
    fn reassigned_safe_let_refused() {
        // The safe-let soundness guard, exercised through this rule: a
        // reassigned binding keeps Unknown (the `let mut` class).
        let input = "fn f(a: &[u8]) -> bool { let mut best = a.iter().find(|x| **x > 1); best = None; best.map_or(false, |x| *x > 2) }";
        let out = propose_fix("unnecessary_map_or", input);
        assert!(out.is_none(), "reassigned binding must not fire: {out:?}");
    }

    #[test]
    fn format_macro_arg_site_fires() {
        let input = "fn f(x: Option<u8>) { println!(\"{}\", x.map_or(false, |v| v > 1)); }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|v| v > 1)"),
            "macro-arg site not produced: {out}"
        );
    }

    #[test]
    fn impl_method_scope_fires() {
        let input = "struct S; impl S { fn m(&self, x: Option<u8>) -> bool { x.map_or(false, |v| v > 1) } }";
        let out = propose_fix("unnecessary_map_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|v| v > 1)"),
            "impl-fn scope not produced: {out}"
        );
    }
}
