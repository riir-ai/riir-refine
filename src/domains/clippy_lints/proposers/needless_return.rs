//! `clippy::needless_return` — strip `return <expr>;` to `<expr>` in fn bodies.
//!
//! Pattern: a fn body whose last statement is `Stmt::Expr(Expr::Return(..))`
//! (when the fn has a non-unit return type). The rewrite replaces that stmt
//! with the returned expression as a tail expr.
//!
//! Span-preserving (T6.4): the stmt's byte range (which covers the trailing
//! `;` when present) is replaced by the ORIGINAL source text of the returned
//! expression — everything else in the file is untouched.

use super::{edit, for_each_fn, src, SourceEdit};
use syn::{Expr, File, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |_sig, block| {
        let Some(last) = block.stmts.last() else { return };
        if let Stmt::Expr(Expr::Return(ret), _semi) = last {
            // Only `return <expr>`; a bare `return;` has no expr to drop into
            // a tail position (unit fns keep it).
            if let Some(inner) = ret.expr.as_ref() {
                if let Some(text) = src(source, inner) {
                    edits.push(edit(last, text));
                }
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
    fn bare_return_no_expr_unchanged() {
        let out = propose_fix("needless_return", "fn foo() { return; }");
        assert!(out.is_none(), "bare return should not be touched: {out:?}");
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
