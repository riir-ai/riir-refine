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
//!
//! Divergence (2026-09-04): a trailing `return` that carries ANY attribute is
//! refused. A `#[cfg]`-attributed one is a target-arch/-feature switch — the
//! parse-view "last" statement is only last on the arches whose cfg keeps it;
//! on the others it vanishes and a DIFFERENT statement trails (real clippy
//! runs post-expansion, so its verdict is per-view while ours is parse-view;
//! probe-verified SILENT on the dual-`#[cfg]` return switch). The rewrite
//! span covers the attribute, so emitting the bare expr would silently drop
//! it. Live misfire: seal-remake `fps_bench.rs::present_mode_from_env` (the
//! wasm32 arm got promoted to an unguarded tail → `unreachable_code` on
//! native, caught by the consumer's `-D warnings` clippy).

use super::{SourceEdit, delete_stmt_edit, edit, for_each_fn, src};
use syn::{Expr, File, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |_sig, block| {
        let Some(last) = block.stmts.last() else {
            return;
        };
        if let Stmt::Expr(Expr::Return(ret), _semi) = last {
            // Attribute guard: the edit span covers the statement's
            // attributes, so any rewrite would silently drop them. cfg is
            // the load-bearing case (see the module doc for the misfire).
            if !ret.attrs.is_empty() {
                return;
            }
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
        assert!(
            out.contains("fn bar(x: i32) -> i32 { x + 1 }"),
            "tail expr not in place: {out}"
        );
    }

    #[test]
    fn strips_trailing_return_no_semi() {
        let out = propose_fix("needless_return", "fn foo() -> u8 { return 42 }")
            .expect("pattern matched");
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

    #[test]
    fn cfg_switched_trailing_return_refused() {
        // The live misfire (seal-remake fps_bench.rs, minimized): the parse-view
        // last stmt is a cfg-attributed return — only last on the arches whose
        // cfg keeps it. Real clippy is SILENT on the whole switch (probe 2026-09-04,
        // clippy 1.93); the old rewrite promoted the wasm32 arm to an unguarded
        // tail AND dropped the attribute, creating `unreachable_code` on native.
        let input = "fn f(e: Option<u8>) -> u8 {\n    #[cfg(not(target_arch = \"wasm32\"))]\n    return e.unwrap_or(0);\n    #[cfg(target_arch = \"wasm32\")]\n    return 0;\n}";
        let out = propose_fix("needless_return", input);
        assert!(
            out.is_none(),
            "cfg-switched return must be refused: {out:?}"
        );
    }

    #[test]
    fn single_cfg_trailing_return_refused() {
        // Same rule one arm short: cfg'd trailing return with no sibling arm —
        // on other arches the stmt vanishes and an earlier stmt trails.
        let input = "fn f() -> u8 {\n    let x = 1;\n    #[cfg(unix)]\n    return 7;\n}";
        let out = propose_fix("needless_return", input);
        assert!(
            out.is_none(),
            "cfg'd trailing return must be refused: {out:?}"
        );
    }

    #[test]
    fn plain_trailing_return_after_cfg_stmt_still_fires() {
        // A cfg'd NON-return statement earlier in the body never moves after
        // the parse-view last on any view — the trailing return stays trailing,
        // so the fix is safe and must keep firing (the guard is on the target
        // statement's attributes, not on cfg anywhere in the body).
        let input = "fn f() -> u8 {\n    #[cfg(unix)]\n    let _ = 1;\n    return 42;\n}";
        let out = propose_fix("needless_return", input).expect("pattern matched");
        assert!(out.contains("42"), "{out}");
        assert!(
            out.contains("#[cfg(unix)]"),
            "unrelated cfg stmt must survive: {out}"
        );
    }
}
