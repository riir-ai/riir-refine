//! `clippy::needless_borrow` — `f(&x)` → `f(x)` when `x` is a bare path.
//!
//! Span-preserving (T6.4): the `&x` argument's byte range is replaced by the
//! ORIGINAL source text of the path.
//!
//! **Type-blind** (not `--fix` eligible): whether `&x` is needless depends
//! on the callee's parameter type (a fn taking `&T` needs the borrow when
//! `x: T`). Parse-only validation cannot decide that. The `self_evolve`
//! loop (real `cargo clippy` verify) remains the consumer.

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

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
        // Only free-fn call args are the pattern (matches the original
        // proposer's scope); method-call receiver positions stay.
        let input = "fn go(s: String) { s.replace(&s, \"x\"); }";
        let out = propose_fix("needless_borrow", input);
        assert!(
            out.is_none(),
            "method-call arg path is out of scope: {out:?}"
        );
    }
}
