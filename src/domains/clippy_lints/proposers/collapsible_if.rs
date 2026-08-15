//! `clippy::collapsible_if` — `if a { if b { work(); } }` →
//! `if a && b { work(); }`.
//!
//! Pattern: an if with no `else` and no attributes whose then-block holds
//! EXACTLY one statement, itself an if with no `else` and no attributes
//! (`Stmt::Expr(Expr::If(..), None)`). Both conditions may be arbitrary
//! expressions; each is parenthesized when its form doesn't bind tighter
//! than `&&` (e.g. comparisons).
//!
//! Span-preserving (Issue 016): the OUTER if's byte range is replaced by
//! `if <c1> && <c2> <inner-then-block>` — every fragment (conditions, body)
//! is the ORIGINAL source text, so the caller's body formatting survives
//! verbatim. Deeper nesting collapses one level per fix pass (the emitter's
//! outer-first overlap rule drops the nested edit; the bounded fixpoint
//! re-runs).

use super::helpers::needs_logic_parens;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprIf, File, Stmt};

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
    fn visit_expr_if(&mut self, i: &'ast ExprIf) {
        syn::visit::visit_expr_if(self, i);
        if let Some(replacement) = self.try_collapse(i) {
            self.edits.push(edit(i, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_collapse(&self, i: &ExprIf) -> Option<String> {
        if i.else_branch.is_some() || !i.attrs.is_empty() {
            return None; // an else can't merge into a plain `&&` join
        }
        let [Stmt::Expr(Expr::If(inner), None)] = i.then_branch.stmts.as_slice() else {
            return None; // then-block must be EXACTLY the inner if
        };
        if inner.else_branch.is_some() || !inner.attrs.is_empty() {
            return None; // inner else has nowhere to go in the merge
        }
        let c1 = self.cond_text(&i.cond)?;
        let c2 = self.cond_text(&inner.cond)?;
        let body = src(self.source, &inner.then_branch)?;
        Some(format!("if {c1} && {c2} {body}"))
    }

    /// Original condition text, parenthesized when it wouldn't bind tighter
    /// than the joining `&&`.
    fn cond_text(&self, cond: &Expr) -> Option<String> {
        let text = src(self.source, cond)?;
        Some(if needs_logic_parens(cond) { format!("({text})") } else { text })
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn collapses_nested_if() {
        let input = "fn f(a: bool, b: bool) { if a { if b { work(); } } }";
        let out = propose_fix("collapsible_if", input).expect("matched");
        assert!(out.contains("if a && b { work(); }"), "not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comparison_condition_gets_parens() {
        let input = "fn f(a: u32, b: bool) { if a == 1 { if b {} } }";
        let out = propose_fix("collapsible_if", input).expect("matched");
        assert!(out.contains("if (a == 1) && b {"), "comparison must be parenthesized: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn outer_else_blocks_the_collapse() {
        let input = "fn f(a: bool, b: bool) { if a { if b { work(); } } else { other(); } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "outer else must block: {out:?}");
    }

    #[test]
    fn inner_else_blocks_the_collapse() {
        let input = "fn f(a: bool, b: bool) { if a { if b { work(); } else { other(); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "inner else must block: {out:?}");
    }

    #[test]
    fn two_stmt_then_blocks_the_collapse() {
        let input = "fn f(a: bool, b: bool) { if a { first(); if b { work(); } } }";
        let out = propose_fix("collapsible_if", input);
        assert!(out.is_none(), "multi-stmt then must block: {out:?}");
    }
}
