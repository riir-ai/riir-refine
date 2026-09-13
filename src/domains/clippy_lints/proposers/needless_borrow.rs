//! `clippy::needless_borrow` — `f(&x)` → `f(x)` on bare-path call args, plus
//! the METHOD-RECEIVER shape `(&x).m()` → `x.m()` for a whitelisted std
//! method set (P11 post-mining intake, 2026-09-13).
//!
//! Span-preserving (T6.4): the `&x` argument's byte range (resp. the
//! `(&x)` receiver's) is replaced by the ORIGINAL source text of the
//! stripped expression.
//!
//! **Type-blind on the free-fn shape** (not `--fix` eligible): whether `&x`
//! is needless depends on the callee's parameter type (a fn taking `&T`
//! needs the borrow when `x: T`). Parse-only validation cannot decide that.
//! The `self_evolve` loop (real `cargo clippy` verify) remains the consumer.
//!
//! **Receiver shape (probe-mapped, rustc/clippy 1.98.1, 2026-09-13):** real
//! clippy fires on EVERY `(&expr).m()` receiver whose method resolves
//! through auto-ref — `as_slice`, `len`, `iter`, `as_ptr` (shared), the
//! `as_mut_slice` mut twin, `(&mut v).len()` (mut-shared), `(&&v).len()`
//! (clippy strips ALL borrow layers, suggestion `v`), field/index/call
//! receivers, and non-whitelisted methods (`(&v).to_vec()` also fires).
//! This proposer is deliberately NARROWER — the documented divergences:
//! - method NAME must be in [`RECEIVER_METHODS`] (std container/slice
//!   methods whose receiver is auto-ref'd for every compiling input —
//!   parse-only decidable; `to_vec` etc. stay silent);
//! - the base expression must be postfix-safe (a bare `(&(a + b)).len()`
//!   receiver would re-group on strip);
//! - `((&v)).m()` (double parens) stays silent.
//!
//! The mutability of the stripped borrow is NOT constrained — the strip is
//! resolution-identical for both (probe: both fire, both suggest the same
//! strip). Tier note: the LINT stays self_evolve-only — its free-fn shape
//! over-matches every necessary `f(&local)` borrow parse-only, which is the
//! documented BY-DESIGN exclusion from the FIXABLE/VERIFY structural set
//! (`retrieval_eval::clean_spans_fire_no_lint`).

use super::helpers::is_postfix_safe;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, ExprReference, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

/// Receiver-strip whitelist: std container/slice methods taking `&self`
/// (resp. `&mut self`) — for these, method resolution auto-refs the
/// receiver, so stripping the explicit borrow is resolution-identical by
/// construction (no `impl for &T` shadowing exists in std for them).
const RECEIVER_METHODS: &[&str] = &["as_slice", "as_mut_slice", "len", "iter", "as_ptr"];

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        syn::visit::visit_expr_call(self, call);
        for arg in &call.args {
            if let Expr::Reference(r) = arg {
                if r.mutability.is_none() && matches!(&*r.expr, Expr::Path(..)) {
                    if let Some(text) = src(self.source, &*r.expr) {
                        self.edits.push(edit(arg, text));
                    }
                }
            }
        }
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(text) = self.receiver_strip(m) {
            self.edits.push(edit(m.receiver.as_ref(), text));
        }
    }
}

impl Collector<'_> {
    /// `(&x).m()` → `x` — None when the receiver is not a strippable
    /// parenthesized borrow feeding a whitelisted method.
    fn receiver_strip(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() {
            return None;
        }
        if !RECEIVER_METHODS.contains(&m.method.to_string().as_str()) {
            return None;
        }
        let Expr::Paren(p) = &*m.receiver else {
            return None;
        };
        let Expr::Reference(ExprReference { expr: inner, .. }) = &*p.expr else {
            return None;
        };
        // Peel ALL borrow layers — clippy's own suggestion for `(&&v).m()`
        // is the full strip to `v` (probe, 2026-09-13), and each layer is
        // resolution-identical for the whitelisted methods.
        let base = strip_borrows(inner);
        if !is_postfix_safe(base) {
            return None;
        }
        src(self.source, base)
    }
}

/// Innermost expression under any run of `&`/`&mut` borrows.
fn strip_borrows(e: &Expr) -> &Expr {
    match e {
        Expr::Reference(r) => strip_borrows(&r.expr),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn strips_borrow_on_path_arg() {
        let input = "fn show(s: &str) -> usize { s.len() }\nfn go(s: &str) -> usize { show(&s) }";
        let out = propose_fix("needless_borrow", input).expect("matched");
        assert!(
            out.contains("show(s)"),
            "borrow not stripped at call site: {out}"
        );
        assert!(
            out.contains("fn show(s: &str)"),
            "signature borrow must stay: {out}"
        );
    }

    #[test]
    fn strips_receiver_borrow_on_whitelisted_method() {
        let input = "fn go(v: Vec<u8>) -> usize { let s = (&v).as_slice(); s.len() }";
        let out = propose_fix("needless_borrow", input).expect("matched");
        assert!(
            out.contains("v.as_slice()"),
            "receiver borrow not stripped: {out}"
        );
        assert!(
            !out.contains("(&v)"),
            "parenthesized borrow must be gone: {out}"
        );
    }

    #[test]
    fn strips_all_borrow_layers_on_receiver() {
        // Probe shape: clippy's suggestion for `(&&v).len()` is `v` (all
        // layers at once) — byte-matched here.
        let input = "fn go(v: Vec<u8>) -> usize { (&&v).len() }";
        let out = propose_fix("needless_borrow", input).expect("matched");
        assert!(
            out.contains("v.len()"),
            "borrow layers not fully stripped: {out}"
        );
    }

    #[test]
    fn strips_mut_receiver_borrow() {
        // Probe shapes: `(&mut v).as_mut_slice()` and `(&mut v).len()` both
        // fire in real clippy with the same strip — mutability-agnostic.
        let input =
            "fn go(mut v: Vec<u8>) -> usize { let s = (&mut v).as_mut_slice(); s.len() }";
        let out = propose_fix("needless_borrow", input).expect("matched");
        assert!(
            out.contains("v.as_mut_slice()"),
            "mut receiver borrow not stripped: {out}"
        );
    }

    #[test]
    fn leaves_non_whitelisted_method() {
        // Documented divergence: real clippy fires on `(&v).to_vec()` too —
        // the whitelist keeps us parse-only decidable (no std impl shadows
        // the whitelisted set; to_vec is not on it).
        let input = "fn go(v: Vec<u8>) -> Vec<u8> { (&v).to_vec() }";
        let out = propose_fix("needless_borrow", input);
        assert!(out.is_none(), "non-whitelisted receiver touched: {out:?}");
    }

    #[test]
    fn complex_receiver_strips_with_parens_riding() {
        // `(&(a + 1)).len()` — the borrow's inner expr is Paren(a + 1):
        // postfix-safe, so the parens ride verbatim and grouping survives
        // (a non-parenthesized non-postfix-safe base is unparseable inside
        // `&..` — the guard is defense-in-depth, not a reachable branch).
        let input = "fn go(a: u8) -> usize { (&(a + 1)).len() }";
        let out = propose_fix("needless_borrow", input).expect("matched");
        assert!(
            out.contains("(a + 1).len()"),
            "parens must ride through the strip: {out}"
        );
        assert!(
            syn::parse_file(&out).is_ok(),
            "output must re-parse: {out}"
        );
    }

    #[test]
    fn leaves_turbofished_receiver_call() {
        let input = "fn go(v: Vec<u8>) -> usize { (&v).iter::<std::slice::Iter<'_, u8>>().count() }";
        let out = propose_fix("needless_borrow", input);
        assert!(out.is_none(), "turbofished call touched: {out:?}");
    }

    #[test]
    fn leaves_double_paren_receiver() {
        // syn keeps nested Paren nodes; only one layer is the borrow paren.
        let input = "fn go(v: Vec<u8>) -> usize { ((&v)).len() }";
        let out = propose_fix("needless_borrow", input);
        assert!(out.is_none(), "double-paren receiver touched: {out:?}");
    }

    #[test]
    fn leaves_mut_borrow() {
        let input = "fn go(v: &mut Vec<u8>) { foo(&mut v) }";
        let out = propose_fix("needless_borrow", input);
        assert!(out.is_none(), "mut borrow touched: {out:?}");
    }

    #[test]
    fn leaves_borrow_of_complex_expr() {
        let input = "fn go() { foo(&(x + 1)) }";
        let out = propose_fix("needless_borrow", input);
        assert!(out.is_none(), "complex borrow touched: {out:?}");
    }

    #[test]
    fn method_call_args_untouched() {
        // Only free-fn call args + whitelisted receivers are the pattern;
        // method-call ARG positions stay out of scope.
        let input = "fn go(s: String) { s.replace(&s, \"x\"); }";
        let out = propose_fix("needless_borrow", input);
        assert!(
            out.is_none(),
            "method-call arg path is out of scope: {out:?}"
        );
    }
}
