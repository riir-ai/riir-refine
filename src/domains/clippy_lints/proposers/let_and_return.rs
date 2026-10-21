//! `clippy::let_and_return` — `let x = expr; x` tail → `expr` (Batch 155 /
//! Plan 146 — the frontier-lead mining pack, family fixture verified to fire
//! exactly this lint under all+pedantic+nursery, 2026-09-18).
//!
//! Pattern: a fn body whose last-but-one statement is a SIMPLE
//! `let x = <init>;` and whose tail expression is the bare binding name.
//! The let is deleted and the tail expr is replaced by the ORIGINAL source
//! text of the init expression.
//!
//! Guards (parse-only provable — FIXABLE tier):
//! - the binding pattern is a plain single ident (no `mut`, no `ref`, no
//!   subpattern, no type annotation — an annotation can drive inference the
//!   tail position cannot re-derive);
//! - the local carries NO attributes (a `#[cfg]`'d let is only present on
//!   some cfg views — the `needless_return` divergence class);
//! - no `else` (let-else is a diverging refutable shape);
//! - the ident occurs EXACTLY TWICE in the fn body's source (the let + the
//!   tail): no other use, no reassignment, no shadow, not referenced by the
//!   init itself (`let x = x + 1;` re-binds a PRIOR x — a different program);
//! - the let is the IMMEDIATELY preceding statement of the tail (clippy's
//!   own shape; earlier lets are untouched).
//!
//! Divergences (documented, probe-shaped like clippy's own refusal set):
//! multi-let tails (`let a = ..; let b = ..; b`) fire on the LAST let only —
//! clippy rewrites just that one, and so does this proposer when the
//! last-but-one statement is the let; a `let` whose init borrows a temporary
//! the binding would lifetime-extend is REFUSED by the twice-count guard
//! only when the temp is the same ident — the general temporary-lifetime
//! case (`let x = &v[0]; x` on a local `v`) is inference-identical in tail
//! position (clippy `MachineApplicable` on this shape).

use super::helpers::count_binding_occurrences;
use super::{delete_stmt_edit, edit, for_each_fn, src, SourceEdit};
use syn::{Expr, File, Ident, Pat, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |_sig, block| {
        let Some((local, ident, tail, init)) = tail_let_binding(block) else {
            return;
        };
        // The binding ident must occur exactly twice in the WHOLE fn body
        // (method/field positions — `.ident` — excluded): the `let` itself
        // + the tail expression. Any third occurrence is a use, a
        // reassignment, a shadow, or a self-referential init — every one a
        // different program. Method-position exclusion is load-bearing:
        // the family fixture itself is `let sum = v.iter().sum(); sum` —
        // `.sum()` is a different namespace and must not count.
        let Some(body_text) = src(source, block) else {
            return;
        };
        if count_binding_occurrences(&body_text, &ident.to_string()) != 2 {
            return;
        }
        let Some(init_text) = src(source, &init.expr) else {
            return;
        };
        // The init text re-inserted at the tail position must parse in place
        // of an expression — it already does (it IS an expression), and the
        // let's span (with the trailing `;` when present) is deleted via
        // delete_stmt_edit so line ownership is respected.
        edits.push(edit(tail, init_text));
        edits.push(delete_stmt_edit(source, local));
    });
    edits
}

/// The (let-stmt, ident, tail-expr) triple when the block ends with
/// `let x = <init>; x`.
fn tail_let_binding(block: &syn::Block) -> Option<(&syn::Local, &Ident, &Expr, &syn::LocalInit)> {
    let stmts = block.stmts.as_slice();
    if stmts.len() < 2 {
        return None;
    }
    let (let_stmt, tail) = (&stmts[stmts.len() - 2], &stmts[stmts.len() - 1]);
    let Stmt::Local(local) = let_stmt else {
        return None;
    };
    let Stmt::Expr(tail_expr, None) = tail else {
        return None;
    };
    let Expr::Path(path) = tail_expr else {
        return None;
    };
    // Bare single-segment path only — `a::b` tails are not bindings.
    let seg = path.path.segments.last()?;
    if path.path.segments.len() != 1 || !path.attrs.is_empty() {
        return None;
    }
    let ident = &seg.ident;
    // Simple ident pattern: no mut, no ref, no subpattern, no attrs.
    let Pat::Ident(pat) = &local.pat else {
        return None;
    };
    if pat.mutability.is_some()
        || pat.by_ref.is_some()
        || pat.subpat.is_some()
        || !local.attrs.is_empty()
    {
        return None;
    }
    if pat.ident != *ident {
        return None;
    }
    // No let-else (diverging refutable shape), init must exist. A TYPE
    // annotation is already refused by requiring top-level `Pat::Ident`
    // (a typed let is `Pat::Type(..)` in syn 2 — the annotation can drive
    // inference the tail position cannot re-derive).
    let Some(init) = &local.init else {
        return None;
    };
    if init.diverge.is_some() {
        return None;
    }
    Some((local, ident, tail_expr, init))
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn folds_tail_let_into_tail_expr() {
        let out = propose_fix(
            "let_and_return",
            "fn shout(s: &str) -> String { let out = s.to_uppercase(); out }",
        )
        .expect("matched");
        assert!(
            out.contains("fn shout(s: &str) -> String { s.to_uppercase() }"),
            "let not folded: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn multi_line_shape_folds() {
        let out = propose_fix(
            "let_and_return",
            "fn total(v: &[i32]) -> i32 {\n    let sum = v.iter().sum();\n    sum\n}",
        )
        .expect("matched");
        assert!(
            out.contains("v.iter().sum()\n}"),
            "multi-line let not folded: {out}"
        );
        assert!(!out.contains("sum;\n"), "let stmt must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn earlier_let_stays_adjacent_let_folds() {
        // `let a` is two statements back — only the ADJACENT `let s` folds
        // (clippy's own shape); `a` stays bound for the init that reads it.
        let input =
            "fn f(v: &[i32]) -> i32 { let a = v.len(); let s = v.iter().sum::<i32>() + a as i32; s }";
        let out = propose_fix("let_and_return", input).expect("matched");
        assert!(out.contains("let a = v.len();"), "earlier let lost: {out}");
        assert!(!out.contains("let s"), "adjacent let not folded: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn binding_used_twice_refused() {
        // The third occurrence (the assert/debug use) makes this a different
        // program — folding would recompute or lose the binding.
        let input = "fn f(v: &[i32]) -> i32 { let s = v.iter().sum(); v.len() as i32 + s }";
        let out = propose_fix("let_and_return", input);
        // `s` occurs twice here too (let + use) but the TAIL is `v.len() as
        // i32 + s`, not the bare ident — different shape, no match. A real
        // triple-use shape:
        let input2 =
            "fn f(v: &[i32], log: &mut i32) -> i32 { let s = v.iter().sum(); *log = s; s }";
        let out2 = propose_fix("let_and_return", input2);
        assert!(
            out.is_none() && out2.is_none(),
            "non-tail-ident / multi-use shapes must not fire: {out:?} {out2:?}"
        );
    }

    #[test]
    fn annotated_let_refused() {
        // The annotation drives inference; dropping it can change meaning.
        let input = "fn f(v: &[u8]) -> Vec<u8> { let s: Vec<u8> = v.to_vec(); s }";
        let out = propose_fix("let_and_return", input);
        assert!(out.is_none(), "annotated let must be refused: {out:?}");
    }

    #[test]
    fn mut_let_refused() {
        let input = "fn f(v: &[u8]) -> usize { let mut s = v.len(); s }";
        let out = propose_fix("let_and_return", input);
        assert!(out.is_none(), "mut let must be refused: {out:?}");
    }

    #[test]
    fn self_referential_init_refused() {
        // `let x = x + 1;` re-binds a PRIOR x — folding would lose the prior
        // binding's read. The twice-count guard fires (x occurs 3 times).
        let input = "fn f(x: u8) -> u8 { let x = x + 1; x }";
        let out = propose_fix("let_and_return", input);
        assert!(
            out.is_none(),
            "self-referential init must be refused: {out:?}"
        );
    }

    #[test]
    fn attributed_let_refused() {
        // The needless_return cfg divergence class: the let is only present
        // on some cfg views.
        let input = "fn f(v: &[u8]) -> usize {\n    #[cfg(unix)]\n    let s = v.len();\n    s\n}";
        let out = propose_fix("let_and_return", input);
        assert!(out.is_none(), "attributed let must be refused: {out:?}");
    }

    #[test]
    fn non_let_penultimate_refused() {
        let input = "fn f(v: &[u8]) -> usize { v.len() }";
        let out = propose_fix("let_and_return", input);
        assert!(out.is_none(), "no let to fold: {out:?}");
    }

    #[test]
    fn method_impl_folds() {
        let input = "struct S;\nimpl S {\n    fn tag(s: &str) -> String { let made = s.replace(' ', \"-\"); made }\n}";
        let out = propose_fix("let_and_return", input).expect("matched");
        assert!(
            out.contains("s.replace(' ', \"-\") }"),
            "impl-method let not folded: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}
