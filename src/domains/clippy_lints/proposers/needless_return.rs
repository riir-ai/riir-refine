//! `clippy::needless_return` — strip `return <expr>;` to `<expr>` in fn bodies.
//!
//! Pattern: a fn body whose last statement is a `return` statement.
//! With a returned expression the stmt is replaced by the expression as a
//! tail expr; with a BARE `return;` (unit tail) the whole stmt is deleted
//! (real clippy's own fix deletes it too — probe-verified 2026-08-19,
//! the run-#2 miss map's `unit_return_plus_needless_return` fixture).
//!
//! Span-preserving (T6.4): the stmt's byte range (which covers the trailing
//! `;` when present) is replaced by the ORIGINAL source text of the returned
//! expression — everything else in the file is untouched. The bare-return
//! deletion goes through `delete_stmt_edit` so line ownership (own-line vs
//! mid-line) is respected.

use super::{delete_stmt_edit, edit, for_each_fn, src, SourceEdit};
use syn::{Expr, File, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |_sig, block| {
        let Some(last) = block.stmts.last() else { return };
        if let Stmt::Expr(Expr::Return(ret), _semi) = last {
            match ret.expr.as_ref() {
                // `return <expr>` — the expr becomes the tail expression.
                Some(inner) => {
                    if let Some(text) = src(source, inner) {
                        edits.push(edit(last, text));
                    }
                }
                // Bare `return;` as the LAST stmt of a unit fn — control
                // flow reaches the end anyway; delete the stmt. A bare return
                // that is NOT last is an early return (control flow) and never
                // matches this pattern.
                None => edits.push(delete_stmt_edit(source, last)),
            }
        }
    });
    edits
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn strips_trailing_return_with_expr() {
        let out = propose_fix("needless_return", "fn bar(x: i32) -> i32 { return x + 1; }")
            .expect("pattern matched");
        assert!(!out.contains("return"), "return not stripped: {out}");
        assert!(out.contains("x + 1"), "expr lost: {out}");
        assert!(out.contains("fn bar(x: i32) -> i32 { x + 1 }"), "tail expr not in place: {out}");
    }

    #[test]
    fn strips_trailing_return_no_semi() {
        let out =
            propose_fix("needless_return", "fn foo() -> u8 { return 42 }").expect("pattern matched");
        assert!(!out.contains("return"), "return not stripped: {out}");
        assert_eq!(out, "fn foo() -> u8 { 42 }");
    }

    #[test]
    fn no_return_leaves_unchanged() {
        let out = propose_fix("needless_return", "fn foo() -> u8 { 42 }");
        assert!(out.is_none(), "should not have changed: {out:?}");
    }

    #[test]
    fn bare_return_tail_deleted() {
        // Real clippy's needless_return DOES fire on a bare trailing
        // `return;` (probe-verified vs clippy 1.97 pedantic — the fix replaces
        // the stmt with nothing). The old "unit fns keep it" note was wrong.
        let out = propose_fix("needless_return", "fn foo() { return; }").expect("pattern matched");
        assert!(!out.contains("return"), "bare return not deleted: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn bare_return_with_prior_stmts_deleted() {
        // The run-#2 bench fixture shape: `-> ()` (removed by unused_unit in
        // an earlier pass) + a trailing bare return after real work.
        let input = "fn foo(t: u32) { let _ = t; return; }";
        let out = propose_fix("needless_return", input).expect("pattern matched");
        assert!(out.contains("let _ = t;"), "prior stmt lost: {out}");
        assert!(!out.contains("return"), "bare return not deleted: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn mid_body_return_untouched() {
        // Only the TRAILING return is the pattern — an early `return` inside
        // an `if` is control flow, not needless.
        let input = "fn f(x: i32) -> i32 { if x < 0 { return 0; } x }";
        let out = propose_fix("needless_return", input);
        assert!(out.is_none(), "early return must not be touched: {out:?}");
    }

    #[test]
    fn impl_method_rewrites() {
        let input = "struct S;\nimpl S {\n    fn get(&self) -> u8 { return 7; }\n}";
        let out = propose_fix("needless_return", input).expect("pattern matched");
        assert!(out.contains("fn get(&self) -> u8 { 7 }"), "{out}");
    }
}
