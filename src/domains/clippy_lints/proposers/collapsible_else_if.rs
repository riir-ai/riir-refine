//! `clippy::collapsible_else_if` — `if a { x(); } else { if b { y(); } }` →
//! `if a { x(); } else if b { y(); }`.
//!
//! Pattern: an if whose else-branch is a BLOCK containing EXACTLY one
//! statement, itself an if (`Stmt::Expr(Expr::If(..), None)`). Attributes on
//! the else-block or the inner if are guarded out — the splice would drop
//! them or land them in an invalid position. (A block LABEL cannot occur
//! here: syn's else grammar accepts only `if` or `{` — verified empirically.)
//!
//! Issue 028: real clippy skips when a comment would be displaced by the
//! collapse — a comment between the else-block's open brace and the inner
//! `if` (the katgpt-speculative false-positive shape: 8 proposed sites, ZERO
//! real findings; includes the arm a `match_bool` edit composes into an
//! else-block in the fixpoint pass) or a trailing comment before the close
//! brace suppresses the lint. syn has no comment nodes, so the raw gap
//! slices are scanned textually.
//!
//! Span-preserving (Issue 016): ONLY the else-block's byte range is
//! replaced — by the ORIGINAL source text of the inner if (braces and all).
//! The `if`/`else` keywords and the outer arms are untouched, as is any
//! whitespace between `else` and `{` (the else-token ends before the block
//! span starts).

use super::{edit, src, SourceEdit};
use syn::spanned::Spanned;
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
        // Issue 028: a comment in either gap slice would be deleted by the
        // splice — real clippy stays silent on those shapes, so must we.
        let open_end = eb.block.brace_token.span.open().byte_range().end;
        let close_start = eb.block.brace_token.span.close().byte_range().start;
        let inner_start = inner.span().byte_range().start;
        let inner_end = inner.span().byte_range().end;
        let (Some(lead), Some(trail)) = (
            self.source.get(open_end..inner_start),
            self.source.get(inner_end..close_start),
        ) else {
            return; // span bookkeeping broke — refuse rather than displace
        };
        if lead.contains("//") || lead.contains("/*") || trail.contains("//") || trail.contains("/*") {
            return;
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

    #[test]
    fn comment_between_else_brace_and_inner_if_blocks() {
        // Issue 028: real clippy is SILENT when a comment sits between the
        // else-block's open brace and the inner `if` (the merge would
        // displace it) — the katgpt-speculative false-positive shape
        // (dd_tree/tree_builder.rs:287-class, 8 sites / 0 real findings).
        let input = "fn f(a: bool, b: bool) { if a { x(); } else {\n            // Original behavior: seed heap with root's children\n            if b { y(); }\n        } }";
        let out = propose_fix("collapsible_else_if", input);
        assert!(out.is_none(), "interleaved comment must block: {out:?}");
    }

    #[test]
    fn match_bool_composed_arm_comment_blocks() {
        // Issue 028 worst sub-case (katgpt-speculative and_or_builder.rs:228):
        // a `match_bool` edit composes `false => { // c \n if let .. }` into
        // an else-block; the fixpoint re-run must NOT collapse through the
        // comment (the observed composed edit deleted it outright).
        let input = "fn f(x: bool, r: Option<usize>) -> usize { if x { z(); } else {\n            // High relevance — close any open region.\n            if let Some(start) = r { start + 1 }\n        } }";
        syn::parse_file(input).expect("fixture must parse");
        let out = propose_fix("collapsible_else_if", input);
        assert!(out.is_none(), "composed-arm comment must block: {out:?}");
    }

    #[test]
    fn trailing_comment_after_inner_if_blocks() {
        // Issue 028: a comment after the inner if (before the else-block's
        // close brace) would be dropped by the splice — same guard.
        let input = "fn f(a: bool, b: bool) { if a { x(); } else { if b { y(); }\n            // why\n        } }";
        let out = propose_fix("collapsible_else_if", input);
        assert!(out.is_none(), "trailing comment must block: {out:?}");
    }

    #[test]
    fn match_arm_with_inner_if_untouched() {
        // Issue 028: the proposer structurally only visits ExprIf
        // else-branches — a match arm can never fire DIRECTLY (it only
        // becomes an else-block after a `match_bool` edit composes, where
        // the comment guard applies). Pin that invariant.
        let input = "fn f(x: bool) -> u32 { match x {\n            false => {\n                // comment\n                if x { 1 } else { 2 }\n            }\n            true => { 3 }\n        } }";
        let out = propose_fix("collapsible_else_if", input);
        assert!(out.is_none(), "match arm must not fire directly: {out:?}");
    }

    #[test]
    fn comment_above_if_else_still_collapses() {
        // Issue 028 control: a comment OUTSIDE the else block does NOT
        // suppress real clippy — must keep firing.
        let input = "fn f(a: bool, b: bool) {\n            // comment above\n            if a { x(); } else { if b { y(); } }\n        }";
        let out = propose_fix("collapsible_else_if", input).expect("matched");
        assert!(out.contains("else if b { y(); }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
