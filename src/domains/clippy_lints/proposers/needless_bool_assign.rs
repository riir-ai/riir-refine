//! `clippy::needless_bool_assign` — `x = if c { true } else { false };` →
//! `x = c;`.
//!
//! Pattern: an assignment whose RHS is an if with one bool-literal statement
//! per block arm and differing literals (the `needless_bool` shape, in
//! assignment position).
//!
//! Span-preserving (Issue 016): the ASSIGN expression's byte range is
//! replaced by `<lhs> = <cond-or-negation>` — both fragments keep their
//! ORIGINAL source text. No trailing `;` is emitted: the statement's own
//! semicolon sits outside the expression span and survives the splice.

use super::helpers::{bool_lit, needs_logic_parens};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Block, Expr, ExprAssign, ExprIf, File, Stmt};

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
    fn visit_expr_assign(&mut self, a: &'ast ExprAssign) {
        syn::visit::visit_expr_assign(self, a);
        if !a.attrs.is_empty() {
            return; // attrs would be dropped by the splice
        }
        let Expr::If(i) = &*a.right else { return };
        let Some(value) = bool_if_value(i) else { return };
        let Some(lhs) = src(self.source, &*a.left) else { return };
        let Some(cond) = src(self.source, &i.cond) else { return };
        let rhs = match value {
            true => cond,
            false if needs_logic_parens(&i.cond) => format!("!({cond})"),
            false => format!("!{cond}"),
        };
        self.edits.push(edit(a, format!("{lhs} = {rhs}")));
    }
}

/// For an `if c { <bool> } else { <bool> }` with differing literals: the
/// value the assignment receives (the then-literal), else `None`.
fn bool_if_value(i: &ExprIf) -> Option<bool> {
    if !i.attrs.is_empty() {
        return None;
    }
    let Some((_, else_e)) = &i.else_branch else { return None };
    let Expr::Block(eb) = &**else_e else { return None };
    if !eb.attrs.is_empty() {
        return None;
    }
    let then_bool = single_bool_lit(&i.then_branch)?;
    let else_bool = single_bool_lit(&eb.block)?;
    (then_bool != else_bool).then_some(then_bool)
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
    fn bool_if_assignment_collapses() {
        let input = "fn set(x: &mut bool, c: bool) { *x = if c { true } else { false }; }";
        let out = propose_fix("needless_bool_assign", input).expect("matched");
        assert!(out.contains("*x = c;"), "assignment not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negated_shape_collapses() {
        let input = "fn set(x: &mut bool, c: bool) { *x = if c { false } else { true }; }";
        let out = propose_fix("needless_bool_assign", input).expect("matched");
        assert!(out.contains("*x = !c;"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comparison_condition_negated_with_parens() {
        let input = "fn set(x: &mut bool, a: u32, b: u32) { *x = if a == b { false } else { true }; }";
        let out = propose_fix("needless_bool_assign", input).expect("matched");
        assert!(out.contains("*x = !(a == b);"), "comparison must be parenthesized: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_bool_rhs_untouched() {
        let input = "fn set(x: &mut u8, c: bool) { *x = if c { 1 } else { 2 }; }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "non-bool rhs must not fire: {out:?}");
    }

    #[test]
    fn same_literal_rhs_untouched() {
        let input = "fn set(x: &mut bool, c: bool) { *x = if c { true } else { true }; }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "constant arms must not fire: {out:?}");
    }

    #[test]
    fn plain_assignment_untouched() {
        let input = "fn set(x: &mut bool) { *x = true; }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "plain assignment must not fire: {out:?}");
    }
}
