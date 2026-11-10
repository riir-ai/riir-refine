//! `clippy::double_parens` — `((x))` → `(x)` (Issue 016).
//!
//! Two shapes (probe-verified vs real clippy 1.97, 2026-08-19):
//!
//! 1. A paren expr whose inner expr is ALSO a paren expr — the outer node's
//!    span is replaced by the ORIGINAL source text of the inner paren node,
//!    keeping exactly one paren level. Triple nesting fires once per pass
//!    (the inner edit overlaps the outer's replacement span and is dropped
//!    by the emitter's outer-first selection — the bounded fixpoint loop
//!    converges).
//! 2. A call/method-call argument that is a single `Expr::Paren` —
//!    `push((v + 1))` — the call's `(` plus the wrap reads as double parens
//!    and real clippy fires (`unnecessary parentheses`, the run-#2 miss
//!    map's `range_loop_plus_double_parens` fixture). The wrap is stripped.
//!    `(a, b)` tuple args are `Expr::Tuple`, NOT `Expr::Paren`, so tuples
//!    never match (the old "stripping would break tuple args" note confused
//!    the token shape with the AST shape — a Paren wraps exactly one
//!    expression by definition).

use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprParen, File};

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
    fn visit_expr_paren(&mut self, p: &'ast ExprParen) {
        syn::visit::visit_expr_paren(self, p);
        let Expr::Paren(inner) = &*p.expr else {
            return;
        };
        if let Some(text) = src(self.source, inner) {
            self.edits.push(edit(p, text));
        }
    }

    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        syn::visit::visit_expr_call(self, c);
        self.strip_wrapped_args(c.args.iter());
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        self.strip_wrapped_args(m.args.iter());
    }
}

impl Collector<'_> {
    /// Shape 2: a single-paren wrap as a direct call/method-call argument —
    /// the call's own paren plus the wrap is the double-paren shape real
    /// clippy flags. The ARG's span is replaced by the inner expression's
    /// original text.
    fn strip_wrapped_args<'a>(&mut self, args: impl Iterator<Item = &'a Expr>) {
        for a in args {
            if let Expr::Paren(wrap) = a {
                // A nested-paren arg (`f(((x)))`) is shape 1's territory —
                // the paren-visitor above handles it one level per pass.
                if matches!(&*wrap.expr, Expr::Paren(_)) {
                    continue;
                }
                if let Some(text) = src(self.source, &wrap.expr) {
                    self.edits.push(edit(wrap, text));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn nested_parens_collapsed_one_level() {
        let out = propose_fix(
            "double_parens",
            "fn f(x: u32) -> u32 { let y = ((x + 1)); y }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(x: u32) -> u32 { let y = (x + 1); y }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn call_argument_double_parens_collapsed() {
        // `f(((x)))` — the nested-paren shape inside a call arg; one level
        // per pass (shape 1).
        let input = "fn id(v: u32) -> u32 { v }\nfn g(x: u32) -> u32 { id(((x))) }";
        let out = propose_fix("double_parens", input).expect("pattern matched");
        assert_eq!(
            out,
            "fn id(v: u32) -> u32 { v }\nfn g(x: u32) -> u32 { id((x)) }"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn single_paren_call_arg_stripped() {
        // `f((x))` — real clippy fires double_parens here (probe L13,
        // 2026-08-19): the call paren + the wrap. `(a, b)` tuples never match
        // (Expr::Tuple ≠ Expr::Paren).
        let input = "fn id(v: u32) -> u32 { v }\nfn g(x: u32) -> u32 { id((x)) }";
        let out = propose_fix("double_parens", input).expect("pattern matched");
        assert_eq!(
            out,
            "fn id(v: u32) -> u32 { v }\nfn g(x: u32) -> u32 { id(x) }"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn method_call_paren_arg_stripped() {
        // The bench fixture shape: `out.push((v + 1))` after the range-loop
        // rewrite leaves exactly this — one wrap inside the call.
        let input =
            "fn f(v: &[u32]) -> Vec<u32> { let mut out = Vec::new(); out.push((v[0] + 1)); out }";
        let out = propose_fix("double_parens", input).expect("pattern matched");
        assert!(
            out.contains("out.push(v[0] + 1)"),
            "wrap not stripped: {out}"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn tuple_argument_unchanged() {
        // `(1, 2)` is Expr::Tuple — passing a tuple needs its parens.
        let input = "fn pair(p: (u8, u8)) -> u8 { p.0 }\nfn g() -> u8 { pair((1, 2)) }";
        let out = propose_fix("double_parens", input);
        assert!(out.is_none(), "tuple arg must not fire: {out:?}");
    }

    #[test]
    fn single_parens_unchanged() {
        let out = propose_fix(
            "double_parens",
            "fn f(x: u32) -> u32 { let y = (x + 1); y }",
        );
        assert!(out.is_none(), "one paren level is not redundant: {out:?}");
    }

    #[test]
    fn triple_nesting_fires_and_parses() {
        // One level per pass is acceptable; the output must still parse.
        let out = propose_fix(
            "double_parens",
            "fn f(x: u32) -> u32 { let y = (((x))); y }",
        )
        .expect("pattern matched");
        assert!(out.contains("((x))"), "one level removed: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }
}
