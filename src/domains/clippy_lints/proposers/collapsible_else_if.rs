//! `clippy::collapsible_else_if` — `if a { x(); } else { if b { y(); } }` →
//! `if a { x(); } else if b { y(); }`.
//!
//! Pattern: an if whose else-branch is a BLOCK containing EXACTLY one
//! statement, itself an if (`Stmt::Expr(Expr::If(..), None)`). Attributes on
//! the else-block or the inner if are guarded out — the splice would drop
//! them or land them in an invalid position.
//!
//! Span-preserving (Issue 016): ONLY the else-block's byte range is
//! replaced — by the ORIGINAL source text of the inner if (braces and all).
//! The `if`/`else` keywords and the outer arms are untouched, as is any
//! whitespace between `else` and `{` (the else-token ends before the block
//! span starts).

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
        let Some((_, else_e)) = &i.else_branch else { return };
        let Expr::Block(eb) = &**else_e else { return };
        let [Stmt::Expr(Expr::If(inner), None)] = eb.block.stmts.as_slice() else { return };
        if !eb.attrs.is_empty() || !inner.attrs.is_empty() {
            return; // attrs would be dropped or misplaced by the splice
        }
        let Some(text) = src(self.source, inner) else { return };
        self.edits.push(edit(eb, text));
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn collapses_else_block_if() {
        let input = "fn f(a: bool, b: bool) { if a { x(); } else { if b { y(); } } }";
        let out = propose_fix("collapsible_else_if", input).expect("matched");
        assert!(out.contains("else if b { y(); }"), "not collapsed: {out}");
        assert!(out.contains("if a { x(); }"), "outer arm must survive: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn else_block_plain_stmt_untouched() {
        let input = "fn f(a: bool) { if a { x(); } else { y(); } }";
        let out = propose_fix("collapsible_else_if", input);
        assert!(out.is_none(), "non-if else-block stmt must not fire: {out:?}");
    }

    #[test]
    fn already_else_if_untouched() {
        let input = "fn f(a: bool, b: bool) { if a { x(); } else if b { y(); } }";
        let out = propose_fix("collapsible_else_if", input);
        assert!(out.is_none(), "already idiomatic else-if must not fire: {out:?}");
    }

    #[test]
    fn else_block_two_stmts_untouched() {
        let input = "fn f(a: bool, b: bool) { if a { x(); } else { z(); if b { y(); } } }";
        let out = propose_fix("collapsible_else_if", input);
        assert!(out.is_none(), "multi-stmt else-block must not fire: {out:?}");
    }
}
