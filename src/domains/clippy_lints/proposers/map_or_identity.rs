//! `clippy::map_or_identity` — `.map_or(default, |x| x)` →
//! `.unwrap_or(default)` (identity closure folded into the terminal form;
//! the rca specimen, rust-code-analysis-cli main.rs:67 —
//! `globset.build().map_or(GlobSet::empty(), |g| g)` →
//! `globset.build().unwrap_or(GlobSet::empty())`).
//!
//! Probe matrix (clippy 1.99.0, `/tmp/probe_cm` + the rca capture, plan 353):
//! - method-call form fires on BOTH Option and Result (the message names
//!   `Option::unwrap_or()` / `Result::unwrap_or()`); UFCS
//!   (`Option::map_or(o, ..)`) is SILENT — the method-call matcher here is
//!   upstream-exact.
//! - non-identity closures are silent (`unnecessary_map_or`'s territory);
//!   ref-pattern and deref/move spellings (`|&x| x`, `|x| *x`, `|x| &x`)
//!   have the same VALUE on `Copy` types but a different `FnOnce` bound — the
//!   upstream identity check is typeck-driven; the parse-only matcher takes
//!   the exact `|p| p` spelling only.
//! - the receiver classification rides the shared [`super::receiver_kind`]
//!   substrate (the `unnecessary_map_or` P15 precedent): a bare `.unwrap_or`
//!   only exists on Option/Result in std, and a custom type with both
//!   `map_or` and an UNRELATED `unwrap_or` would make the rewrite semantic —
//!   type-blind standalone is E0599-unsafe, unknown receivers refuse (the
//!   documented divergence; the --clippy-json oracle lane is the close).
//! - the rewrite is evaluation-order-identical: `unwrap_or(default)`
//!   evaluates `default` eagerly exactly where `map_or` did (`map_or`'s
//!   default argument is eager too — the lazy twin is
//!   `unnecessary_lazy_evaluations`' `unwrap_or_else` shape).

use std::collections::HashMap;

use super::helpers::is_postfix_safe;
use super::receiver_kind::{ReceiverKind, ScopeKinds, classify_receiver, collect_fn_rets};
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, ItemFn};

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
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
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
            try_map_or_identity(self.source, m, self.scopes.last(), Some(&self.fn_rets))
        {
            self.edits.push(edit(m, replacement));
        }
    }
}

/// The identity closure shape: `|x| x` (single bare-ident param, body exactly
/// that binding), optionally parenthesized `(|x| x)`.
fn identity_binding(expr: &Expr) -> bool {
    let Expr::Closure(c) = expr else { return false };
    if !c.attrs.is_empty() || c.constness.is_some() || c.movability.is_some() {
        return false;
    }
    // The identity closure has EXACTLY one param — a 2-param closure is a
    // different fn shape (and `|x, y| x` is not the identity fn of anything).
    if c.inputs.len() != 1 {
        return false;
    }
    let syn::Pat::Ident(id) = &c.inputs[0] else {
        return false;
    };
    // `mut`/`ref`/`&`-pattern forms are not the bare identity binding.
    if id.mutability.is_some() || id.by_ref.is_some() || id.subpat.is_some() {
        return false;
    }
    let Expr::Path(body) = c.body.as_ref() else {
        return false;
    };
    body.path.is_ident(&id.ident)
}

fn try_map_or_identity(
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
    let mapper = match m.args[1] {
        Expr::Paren(ref p) => p.expr.as_ref(),
        ref e => e,
    };
    if !identity_binding(mapper) {
        return None;
    }
    let recv_text = src(source, m.receiver.as_ref())?;
    // Type-proof leg: `.unwrap_or` is std-Option/Result territory; a custom
    // type carrying its own `map_or` but no matching `unwrap_or` would make
    // the rewrite an E0599. Unknown receivers refuse (the documented
    // divergence — the oracle lane covers those sites).
    match classify_receiver(&recv_text, scope, fn_rets) {
        ReceiverKind::OptionLike | ReceiverKind::ResultLike => {}
        ReceiverKind::Unknown => return None,
    }
    let default_text = src(source, &m.args[0])?;
    let recv = if is_postfix_safe(m.receiver.as_ref()) {
        recv_text
    } else {
        format!("({recv_text})")
    };
    Some(format!("{recv}.unwrap_or({default_text})"))
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn result_param_identity_becomes_unwrap_or() {
        // The rca specimen's exact shape (Result receiver).
        let input = concat!(
            "fn f(r: Result<u8, String>) -> u8 {\n",
            "    r.map_or(0, |x| x)\n",
            "}\n",
        );
        let out = propose_fix("map_or_identity", input).expect("matched");
        assert!(
            out.contains("r.unwrap_or(0)"),
            "unwrap_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn option_param_identity_becomes_unwrap_or() {
        let input = "fn f(o: Option<u8>) -> u8 { o.map_or(7, |x| x) }";
        let out = propose_fix("map_or_identity", input).expect("matched");
        assert!(
            out.contains("o.unwrap_or(7)"),
            "unwrap_or not produced: {out}"
        );
    }

    #[test]
    fn chain_receiver_becomes_unwrap_or() {
        // The rca site: `globset.build().map_or(GlobSet::empty(), |g| g)` —
        // a chain receiver (`.build(` classifies OptionLike via nothing —
        // a bare call IS classified by the same-file fn-ret map; here the
        // chain's `.build(` has no allowlist shape so the PARAM route is the
        // proof: `g: Result<..>` param.
        let input = concat!(
            "fn f(g: Result<u8, String>) -> u8 {\n",
            "    g.map_err(|e| e).map_or(0, |x| x)\n",
            "}\n",
        );
        // `.map_err(` has no allowlist shape → Unknown → refused (the
        // documented divergence; the oracle lane is the close).
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "unclassifiable chain must refuse: {out:?}");
    }

    #[test]
    fn same_file_fn_receiver_becomes_unwrap_or() {
        let input = concat!(
            "fn pick(k: u8) -> Option<u8> { None }\n",
            "fn f(k: u8) -> u8 { pick(k).map_or(3, |x| x) }\n",
        );
        let out = propose_fix("map_or_identity", input).expect("matched");
        assert!(
            out.contains("pick(k).unwrap_or(3)"),
            "fn-ret terminal: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn safe_let_receiver_becomes_unwrap_or() {
        // The P15 safe-let route: `.find(` initializer proves Option.
        let input = concat!(
            "fn f(v: &[u8]) -> u8 {\n",
            "    let best = v.iter().find(|x| **x > &1);\n",
            "    best.copied().map_or(0, |x| x)\n",
            "}\n",
        );
        // `.copied(` is not a known shape → Unknown → refused (divergence).
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "unclassifiable post-let chain: {out:?}");
    }

    #[test]
    fn unknown_receiver_refused() {
        let input = "fn f(k: u8) -> u8 { lookup(k).map_or(0, |x| x) }";
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "unknown receiver must not fire: {out:?}");
    }

    #[test]
    fn non_identity_closure_refused() {
        let input = "fn f(o: Option<u8>) -> u8 { o.map_or(0, |x| x + 1) }";
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "non-identity closure fired: {out:?}");
    }

    #[test]
    fn deref_closure_refused() {
        // `|x| *x` is value-equal on Copy but NOT the identity fn (the
        // probe's exact-boundary posture) — refused.
        let input = "fn f(o: Option<u8>) -> u8 { o.map_or(0, |x| *x) }";
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "deref closure fired: {out:?}");
    }

    #[test]
    fn ref_pattern_closure_refused() {
        let input = "fn f(o: Option<u8>) -> u8 { o.map_or(0, |&x| x) }";
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "ref-pattern closure fired: {out:?}");
    }

    #[test]
    fn parenthesized_identity_accepted() {
        let input = "fn f(o: Option<u8>) -> u8 { o.map_or(7, (|x| x)) }";
        let out = propose_fix("map_or_identity", input).expect("matched");
        assert!(out.contains("o.unwrap_or(7)"), "paren form: {out}");
    }

    #[test]
    fn turbofish_refused() {
        let input = "fn f(o: Option<u8>) -> u8 { o.map_or::<u8>(7, |x| x) }";
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "turbofish fired: {out:?}");
    }

    #[test]
    fn bool_twin_stays_with_unnecessary_map_or() {
        // `map_or(false, f)` is unnecessary_map_or's shape — the two lints
        // partition the map_or surface by the mapper's shape.
        let input = "fn f(o: Option<u8>) -> bool { o.map_or(false, |x| x > 1) }";
        let out = propose_fix("map_or_identity", input);
        assert!(out.is_none(), "bool-default shape fired: {out:?}");
    }
}
