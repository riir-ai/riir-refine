//! `clippy::needless_bool` — `if b { true } else { false }` → `b`;
//! `if b { false } else { true }` → `!b`.
//!
//! Pattern: an if (no attributes) whose else-branch is a BLOCK (a direct
//! else-IF chain member does not fire — see the retraction below) where BOTH
//! blocks hold EXACTLY one statement, a bare boolean literal, and the
//! literals differ.
//!
//! Span-preserving (Issue 016): the whole if's byte range is replaced by the
//! ORIGINAL condition text, or its negation (parenthesized when the
//! condition's form doesn't bind tighter than `!`).
//!
//! Else-chain guard: a bare `b` / `!b` replacement is invalid in direct
//! else-position (`else !b` does not parse). Because the visitor recurses
//! before detecting, a chain member's edit is pushed before its parent is
//! seen — so each if RETRACTS any whole-span edit made on its direct
//! else-child after the fact.

use super::helpers::{bool_lit, needs_logic_parens};
use super::{edit, src, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, ExprIf, File, Stmt};

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
        if let Some(replacement) = self.try_bool(i) {
            self.edits.push(edit(i, replacement));
        }
        self.retract_else_child(i);
    }
}

impl Collector<'_> {
    fn try_bool(&self, i: &ExprIf) -> Option<String> {
        if !i.attrs.is_empty() {
            return None;
        }
        let Some((_, else_e)) = &i.else_branch else { return None };
        let Expr::Block(eb) = &**else_e else { return None };
        if !eb.attrs.is_empty() {
            return None; // else-block attrs would be dropped by the splice
        }
        let then_bool = single_bool_lit(&i.then_branch)?;
        let else_bool = single_bool_lit(&eb.block)?;
        if then_bool == else_bool {
            return None; // constant arms — a different rewrite
        }
        let cond = src(self.source, &i.cond)?;
        Some(match then_bool {
            true => cond,
            false if needs_logic_parens(&i.cond) => format!("!({cond})"),
            false => format!("!{cond}"),
        })
    }

    /// Drop any edit that rewrote this if's direct else-child wholesale.
    /// Only an if-child (an else-if chain member) can produce a whole-child
    /// replacement — and `else <bare-expr>` does not parse. A BLOCK child's
    /// inner edits have strictly smaller spans and survive.
    fn retract_else_child(&mut self, i: &ExprIf) {
        if let Some((_, else_e)) = &i.else_branch {
            let span = (**else_e).span().byte_range();
            self.edits.retain(|ed| ed.start != span.start || ed.end != span.end);
        }
    }
}

/// The block's single statement as a bool literal, if that's its exact shape.
fn single_bool_lit(block: &Block) -> Option<bool> {
    match block.stmts.as_slice() {
        [Stmt::Expr(e, None)] => bool_lit(e),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn true_else_false_collapses_to_condition() {
        let input = "fn f(b: bool) -> bool { if b { true } else { false } }";
        let out = propose_fix("needless_bool", input).expect("matched");
        assert!(out.contains("fn f(b: bool) -> bool { b }"), "not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn false_else_true_collapses_to_negation() {
        let input = "fn f(b: bool) -> bool { if b { false } else { true } }";
        let out = propose_fix("needless_bool", input).expect("matched");
        assert!(out.contains("{ !b }"), "not negated: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comparison_condition_negated_with_parens() {
        let input = "fn f(a: u32, b: u32) -> bool { if a == b { false } else { true } }";
        let out = propose_fix("needless_bool", input).expect("matched");
        assert!(out.contains("!(a == b)"), "comparison must be parenthesized: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_bool_arms_untouched() {
        let input = "fn f(b: bool) -> u8 { if b { 1 } else { 2 } }";
        let out = propose_fix("needless_bool", input);
        assert!(out.is_none(), "non-bool arms must not fire: {out:?}");
    }

    #[test]
    fn same_literal_arms_untouched() {
        let input = "fn f(b: bool) -> bool { if b { true } else { true } }";
        let out = propose_fix("needless_bool", input);
        assert!(out.is_none(), "constant arms must not fire: {out:?}");
    }

    #[test]
    fn else_if_chain_members_untouched() {
        // The middle chain member HAS a block else (`{ true }`) and would
        // otherwise rewrite to `!b` — landing as the invalid `else !b`.
        let input =
            "fn f(a: bool, b: bool) -> bool { if a { true } else if b { false } else { true } }";
        let out = propose_fix("needless_bool", input);
        assert!(out.is_none(), "else-if chain member must not fire: {out:?}");
    }
}
