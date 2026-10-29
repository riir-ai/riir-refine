//! `clippy::needless_bool_assign` — `if c { x = true; } else { x = false; }`
//! → `x = c;`.
//!
//! Pattern (real-clippy shape, verified against clippy 1.93 UI tests — the
//! Issue 017 eval-fixture pass caught the prior mismatch): an if statement
//! whose arms each contain a single assignment of a DIFFERING bool literal
//! to the SAME left-hand side. The assign-RHS form `x = if c { true } else
//! { false };` is real clippy's `needless_bool`, not `_assign` (that shape
//! is covered by this crate's `needless_bool` proposer).
//!
//! Span-preserving (Issue 017 fix): the whole if STATEMENT's byte range is
//! replaced by `<lhs> = <cond-or-negation>;` — both fragments keep their
//! ORIGINAL source text. The trailing `;` is emitted because an
//! if-statement in source carries none, while the collapsed assignment
//! statement requires one (when the source form ends with a stray `;`, it
//! sits inside the statement span and is swallowed with it).

use super::helpers::{bool_lit, contains_let, needs_logic_parens};
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprAssign, ExprIf, File, Stmt};

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

impl Collector<'_> {
    /// Try to collapse `if c { lhs = A; } else { lhs = B; }` (differing bool
    /// literals, same lhs) into `lhs = c;` / `lhs = !c;`.
    fn try_collapse(&mut self, stmt: &Stmt, i: &ExprIf) {
        if !i.attrs.is_empty() {
            return; // attrs would be dropped by the splice
        }
        let Some((_, else_e)) = &i.else_branch else {
            return;
        };
        let Expr::Block(eb) = &**else_e else { return };
        if !eb.attrs.is_empty() {
            return;
        }
        let Some((then_assign, then_value)) = single_bool_assign(&i.then_branch.stmts) else {
            return;
        };
        let Some((else_assign, else_value)) = single_bool_assign(&eb.block.stmts) else {
            return;
        };
        let Some(then_lhs) = src(self.source, &then_assign.left) else {
            return;
        };
        let Some(else_lhs) = src(self.source, &else_assign.left) else {
            return;
        };
        // Same LHS, differing literals — otherwise this is not the lint.
        if then_lhs != else_lhs || then_value == else_value {
            return;
        }
        // A `let` condition has no expression-position form: `!(let a = b)`
        // does not parse and a bare `let` statement is not an expression.
        // Edition-blind guard (see helpers::contains_let).
        if contains_let(&i.cond) {
            return;
        }
        let Some(cond) = src(self.source, &i.cond) else {
            return;
        };
        let rhs = match then_value {
            true => cond,
            false if needs_logic_parens(&i.cond) => format!("!({cond})"),
            false => format!("!{cond}"),
        };
        // The STATEMENT span (not the ExprIf span) so a trailing `;` in the
        // `Stmt::Expr(_, Some(_))` form is swallowed with the if — the
        // replacement carries its own.
        self.edits.push(edit(stmt, format!("{then_lhs} = {rhs};")));
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_stmt(&mut self, s: &'ast Stmt) {
        syn::visit::visit_stmt(self, s);
        let Expr::If(i) = (match s {
            Stmt::Expr(e, _) => e,
            _ => return,
        }) else {
            return;
        };
        self.try_collapse(s, i);
    }
}

/// The statements' single member as an assignment of a bool literal, if
/// that's the block's exact shape: `(assign, literal-value)`.
fn single_bool_assign(stmts: &[Stmt]) -> Option<(&ExprAssign, bool)> {
    let [Stmt::Expr(Expr::Assign(a), Some(_))] = stmts else {
        return None;
    };
    if !a.attrs.is_empty() {
        return None;
    }
    let value = bool_lit(&a.right)?;
    Some((a, value))
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn branch_assignments_collapse() {
        let input = "fn f(x: &mut bool, c: bool) { if c { *x = true; } else { *x = false; } }";
        let out = propose_fix("needless_bool_assign", input).expect("matched");
        assert!(out.contains("*x = c;"), "branch form not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negated_branches_collapse() {
        let input = "fn f(x: &mut bool, c: bool) { if c { *x = false; } else { *x = true; } }";
        let out = propose_fix("needless_bool_assign", input).expect("matched");
        assert!(out.contains("*x = !c;"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comparison_condition_negated_with_parens() {
        let input =
            "fn f(x: &mut bool, a: u32, b: u32) { if a == b { *x = false; } else { *x = true; } }";
        let out = propose_fix("needless_bool_assign", input).expect("matched");
        assert!(
            out.contains("*x = !(a == b);"),
            "comparison must be parenthesized: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn assign_rhs_if_untouched() {
        // `x = if c { true } else { false };` is needless_bool, NOT
        // needless_bool_assign (real clippy semantics, Issue 017 fix).
        let input = "fn f(c: bool) { let mut x = false; x = if c { true } else { false }; }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "assign-RHS form must not fire: {out:?}");
    }

    #[test]
    fn non_bool_literals_untouched() {
        let input = "fn f(x: &mut u8, c: bool) { if c { *x = 1; } else { *x = 2; } }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "non-bool arms must not fire: {out:?}");
    }

    #[test]
    fn differing_lhs_untouched() {
        let input = "fn f(a: &mut bool, b: &mut bool, c: bool) { if c { *a = true; } else { *b = false; } }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "differing lhs must not fire: {out:?}");
    }

    #[test]
    fn same_literal_untouched() {
        let input = "fn f(x: &mut bool, c: bool) { if c { *x = true; } else { *x = true; } }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "constant arms must not fire: {out:?}");
    }

    #[test]
    fn let_cond_stays_silent() {
        // A `let` condition has no expression-position form (`!(let a = b)`
        // does not parse). Edition-blind guard class (helpers::contains_let).
        let input = "fn f(x: &mut bool, o: Option<i32>) { if let Some(v) = o { *x = true; } else { *x = false; } }";
        let out = propose_fix("needless_bool_assign", input);
        assert!(out.is_none(), "let condition must not fire: {out:?}");
    }
}
