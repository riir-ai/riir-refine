//! `clippy::deref_addrof` — `*&x` → `x` (also `*&mut x`, Issue 016).
//!
//! Pattern: a deref unary whose operand is a reference expression —
//! EXACTLY one `*` wrapping one `&` (any mutability). In syn 2 the
//! reference is `Expr::Reference` (there is no `UnOp::Ref`). The whole
//! outer unary's span is replaced by the ORIGINAL source text of the
//! reference's inner expr. The reversed order (`&*x`, a reborrow) never
//! fires — it is a borrow of the pointee, not a cancellation.

use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprUnary, File, UnOp};

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

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_unary(&mut self, u: &'ast ExprUnary) {
        syn::visit::visit_expr_unary(self, u);
        if !matches!(u.op, UnOp::Deref(_)) {
            return;
        }
        let Expr::Reference(r) = &*u.expr else {
            return;
        };
        if let Some(text) = src(self.source, &r.expr) {
            self.edits.push(edit(u, text));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn deref_of_ref_collapses() {
        let out = propose_fix("deref_addrof", "fn f(x: u32) -> u32 { let y = *&x; y }")
            .expect("pattern matched");
        assert_eq!(out, "fn f(x: u32) -> u32 { let y = x; y }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn deref_of_mut_ref_collapses() {
        let out = propose_fix("deref_addrof", "fn f(v: u32) -> u32 { let y = *&mut v; y }")
            .expect("pattern matched");
        assert_eq!(out, "fn f(v: u32) -> u32 { let y = v; y }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn plain_deref_unchanged() {
        let out = propose_fix("deref_addrof", "fn f(x: &u32) -> u32 { *x }");
        assert!(out.is_none(), "`*x` alone is a real deref: {out:?}");
    }

    #[test]
    fn reborrow_unchanged() {
        // REVERSED order — `&*x` is a reborrow, not a cancellation.
        let out = propose_fix("deref_addrof", "fn f(x: &u32) -> &u32 { &*x }");
        assert!(out.is_none(), "reborrow must not fire: {out:?}");
    }

    #[test]
    fn double_deref_of_ref_removes_one_level() {
        // `**&x` is `*x` after one pass — correct, and it parses.
        let out = propose_fix("deref_addrof", "fn f(x: &u32) -> u32 { let y = **&x; y }")
            .expect("pattern matched");
        assert!(out.contains("*x"), "one level removed: {out}");
        assert!(!out.contains("**&x"), "no residue: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }
}
