//! `clippy::needless_return` — strip `return <expr>;` to `<expr>` in fn bodies.
//!
//! Pattern: a fn body whose last statement is `Stmt::Expr(Expr::Return(..))`
//! (when the fn has a non-unit return type). The rewrite replaces that stmt
//! with the returned expression as a tail expr.

use syn::{Expr, ExprReturn, File, ItemFn, Stmt};

/// Apply the rewrite in place. Returns `true` if anything changed.
pub(crate) fn rewrite(file: &mut File) -> bool {
    let mut changed = false;
    for item in &mut file.items {
        if let syn::Item::Fn(fn_item) = item {
            changed |= rewrite_fn(fn_item);
        }
    }
    changed
}

fn rewrite_fn(fn_item: &mut ItemFn) -> bool {
    let stmts = &mut fn_item.block.stmts;
    if stmts.is_empty() {
        return false;
    }

    // Find a trailing return stmt; replace with the returned expr.
    let last_idx = stmts.len() - 1;
    let last = &mut stmts[last_idx];

    // In syn 2.x, Stmt is `Local | Item | Expr(expr, Option<Semi>) | Macro`.
    // A `return expr;` parses as `Stmt::Expr(Expr::Return(..), Some(_))`.
    // A `return expr` parses as `Stmt::Expr(Expr::Return(..), None)`.
    if let Stmt::Expr(Expr::Return(ret), _semi) = last {
        if let Some(new_stmt) = rewrite_return(ret) {
            *last = new_stmt;
            return true;
        }
    }
    false
}

/// Convert `ExprReturn { expr: Some(e) }` → `Stmt::Expr(e, None)` (tail expr).
/// Returns None for bare `return;` (no expr to drop into).
fn rewrite_return(ret: &ExprReturn) -> Option<Stmt> {
    let inner = ret.expr.as_ref()?;
    let cloned: Expr = (**inner).clone();
    Some(Stmt::Expr(cloned, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(crate::proposers::quote_file(&file))
        } else {
            None
        }
    }

    #[test]
    fn strips_trailing_return_with_expr() {
        let out = propose("fn bar(x: i32) -> i32 { return x + 1; }")
            .expect("pattern matched");
        assert!(!out.contains("return"), "return not stripped: {out}");
        assert!(out.contains("x + 1"), "expr lost: {out}");
    }

    #[test]
    fn strips_trailing_return_no_semi() {
        let out = propose("fn foo() -> u8 { return 42 }").expect("pattern matched");
        assert!(!out.contains("return"), "return not stripped: {out}");
    }

    #[test]
    fn no_return_leaves_unchanged() {
        let out = propose("fn foo() -> u8 { 42 }");
        assert!(out.is_none(), "should not have changed: {out:?}");
    }

    #[test]
    fn bare_return_no_expr_unchanged() {
        let out = propose("fn foo() { return; }");
        assert!(out.is_none(), "bare return should not be touched: {out:?}");
    }
}
