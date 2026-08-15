//! `clippy::double_parens` — `((x))` → `(x)` (Issue 016).
//!
//! Pattern: a paren expr whose inner expr is ALSO a paren expr. The outer
//! node's span is replaced by the ORIGINAL source text of the inner paren
//! node, keeping exactly one paren level. Triple nesting fires once per
//! pass (the inner edit overlaps the outer's replacement span and is
//! dropped by the emitter's outer-first selection — the bounded
//! fixpoint loop converges).

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprParen, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
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
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn nested_parens_collapsed_one_level() {
        let out = propose_fix("double_parens", "fn f(x: u32) -> u32 { let y = ((x + 1)); y }")
            .expect("pattern matched");
        assert_eq!(out, "fn f(x: u32) -> u32 { let y = (x + 1); y }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn call_argument_double_parens_collapsed() {
        // `f((x))` is a SINGLE paren arg — that's unused_parens territory and
        // stripping it would break tuple args (`f((a, b))`). The double-paren
        // shape inside a call arg is `f(((x)))`, collapsing one level.
        let input = "fn id(v: u32) -> u32 { v }\nfn g(x: u32) -> u32 { id(((x))) }";
        let out = propose_fix("double_parens", input).expect("pattern matched");
        assert_eq!(out, "fn id(v: u32) -> u32 { v }\nfn g(x: u32) -> u32 { id((x)) }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn single_parens_unchanged() {
        let out = propose_fix("double_parens", "fn f(x: u32) -> u32 { let y = (x + 1); y }");
        assert!(out.is_none(), "one paren level is not redundant: {out:?}");
    }

    #[test]
    fn triple_nesting_fires_and_parses() {
        // One level per pass is acceptable; the output must still parse.
        let out = propose_fix("double_parens", "fn f(x: u32) -> u32 { let y = (((x))); y }")
            .expect("pattern matched");
        assert!(out.contains("((x))"), "one level removed: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }
}
