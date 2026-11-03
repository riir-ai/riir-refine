//! `clippy::manual_slice_fill` — `for v in &mut place { *v = val; }` →
//! `place.fill(val);`.
//!
//! Style lint (new in clippy 1.98). Span-preserving (T6.4): the whole
//! for-loop expression is replaced by a `fill` call; every byte outside the
//! loop stays identical.
//!
//! Guards (the probe-mapped subset of real clippy's behavior — probe crate,
//! rustc 1.98.1, 2026-09-05):
//! - iterable must be the `&mut place` REFERENCE form — `for v in
//!   t.iter_mut()` is silent in real clippy and silent here (firing both
//!   forms would exceed the corpus-truthful domain);
//! - body = exactly one statement, an assignment whose LHS is the loop
//!   binding (`*v = val` for the by-value pat, `v = val` for the
//!   `for &mut v in &mut place` ref-pat form);
//! - the value must be PURE ([`is_pure_expr`]): `fill` evaluates its
//!   argument ONCE while the loop evaluates it PER ITERATION — an impure
//!   value (`*v = next()`) changes evaluation count, so calls/macros/indexing
//!   stay silent; a path value is safe because the original loop only
//!   compiles when the value is `Copy`, which subsumes `fill`'s `Clone`
//!   bound;
//! - the value must not BE the loop binding (`*v = v` is meaningless, not a
//!   fill);
//! - no comments inside the loop span (the replacement would displace them).
//!
//! `place` is evaluated once by the for-loop and once by `fill` — identical.
//! Vec/slice/array all deref to `[T]::fill`.

use super::helpers::{drops_interior_comment, is_postfix_safe, is_pure_expr};
use super::{edit, src, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprAssign, ExprForLoop, ExprPath, File, Pat, Stmt, UnOp};

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
    fn visit_expr_for_loop(&mut self, f: &'ast ExprForLoop) {
        syn::visit::visit_expr_for_loop(self, f);
        if let Some(replacement) = self.try_fill(f) {
            self.edits.push(edit(f, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_fill(&self, f: &ExprForLoop) -> Option<String> {
        if !f.attrs.is_empty() {
            return None;
        }
        // `for <pat> in &mut <place> { .. }` — the reference form only.
        let Expr::Reference(r) = &*f.expr else {
            return None;
        };
        r.mutability?;
        let [stmt] = f.body.stmts.as_slice() else {
            return None;
        };
        let Stmt::Expr(Expr::Assign(assign), _) = stmt else {
            return None;
        };
        let binding = Self::assigns_loop_binding(assign, &f.pat)?;
        let val = &*assign.right;
        if !is_pure_expr(val) {
            return None; // fill evaluates the value ONCE, the loop per-iteration
        }
        // The value must not mention the loop binding AT ALL (`*v = *v;` and
        // `*v = *v * 2.0;` are per-element reads — `fill` cannot express
        // them; `fill(<v>)` would not even compile).
        if contains_ident(val, &binding) {
            return None;
        }
        // Whole-span replacement — any interior comment would be displaced.
        if drops_interior_comment(self.source, f.span().byte_range(), &[]) {
            return None;
        }
        let place_text = src(self.source, &r.expr)?;
        let val_text = src(self.source, val)?;
        let place = if is_postfix_safe(&r.expr) {
            place_text
        } else {
            format!("({place_text})")
        };
        Some(format!("{place}.fill({val_text});"))
    }

    /// If `assign.left` writes the loop binding, return the binding's name.
    /// Two shapes: `for v in &mut p { *v = x; }` (deref of the by-value
    /// ident pat) and `for &mut v in &mut p { v = x; }` (ref pat, direct
    /// path write).
    fn assigns_loop_binding(assign: &ExprAssign, pat: &Pat) -> Option<String> {
        if let Pat::Ident(pi) = pat {
            if pi.by_ref.is_some() || pi.subpat.is_some() {
                return None;
            }
            let name = pi.ident.to_string();
            // `*v = x` — unary deref of a bare path naming `v`.
            if let Expr::Unary(u) = &*assign.left {
                if matches!(u.op, UnOp::Deref(_)) {
                    if let Expr::Path(p) = &*u.expr {
                        if p.path.get_ident().is_some_and(|id| id == &pi.ident) {
                            return Some(name);
                        }
                    }
                }
            }
            return None;
        }
        if let Pat::Reference(pr) = pat {
            pr.mutability?;
            let Pat::Ident(pi) = &*pr.pat else {
                return None;
            };
            if pi.by_ref.is_some() || pi.subpat.is_some() {
                return None;
            }
            if let Expr::Path(ExprPath { path, .. }) = &*assign.left {
                if path.get_ident().is_some_and(|id| id == &pi.ident) {
                    return Some(pi.ident.to_string());
                }
            }
        }
        None
    }
}

fn contains_ident(e: &Expr, name: &str) -> bool {
    struct Seeker<'a> {
        name: &'a str,
        found: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for Seeker<'_> {
        fn visit_path(&mut self, p: &'ast syn::Path) {
            if p.get_ident().is_some_and(|id| id == self.name) {
                self.found = true;
            }
            syn::visit::visit_path(self, p);
        }
    }
    let mut s = Seeker { name, found: false };
    syn::visit::Visit::visit_expr(&mut s, e);
    s.found
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn fills_field_place() {
        let input = "struct T { data: Vec<f32> }\nfn reset(t: &mut T) {\n    for v in &mut t.data {\n        *v = 0.5;\n    }\n}";
        let out = propose_fix("manual_slice_fill", input).expect("matched");
        assert!(out.contains("t.data.fill(0.5);"), "no fill: {out}");
        assert!(!out.contains("for "), "loop remains: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn fills_index_slice_place() {
        let input = "fn f(buf: &mut [f32], n: usize) { for v in &mut buf[..n] { *v = 0.0; } }";
        let out = propose_fix("manual_slice_fill", input).expect("matched");
        assert!(out.contains("buf[..n].fill(0.0);"), "no fill: {out}");
    }

    #[test]
    fn ref_pat_form_fires() {
        let input = "fn f(buf: &mut Vec<u8>) { for &mut v in &mut buf[..] { v = 3; } }";
        let out = propose_fix("manual_slice_fill", input).expect("matched");
        assert!(out.contains("buf[..].fill(3);"), "no fill: {out}");
    }

    #[test]
    fn iter_mut_form_silent() {
        // Real clippy is silent on this shape — we stay corpus-truthful.
        let input = "fn f(t: &mut Vec<f32>) { for v in t.iter_mut() { *v = 0.0; } }";
        let out = propose_fix("manual_slice_fill", input);
        assert!(out.is_none(), "iter_mut must not fire: {out:?}");
    }

    #[test]
    fn impure_value_silent() {
        // fill evaluates ONCE, the loop per-iteration — calls stay silent.
        let input = "fn g() -> f32 { 1.0 }\nfn f(buf: &mut Vec<f32>) { for v in &mut buf[..] { *v = g(); } }";
        let out = propose_fix("manual_slice_fill", input);
        assert!(out.is_none(), "impure value must not fire: {out:?}");
    }

    #[test]
    fn value_is_binding_silent() {
        let input = "fn f(buf: &mut Vec<u8>) { for v in &mut buf[..] { *v = *v; } }";
        let out = propose_fix("manual_slice_fill", input);
        assert!(out.is_none(), "self-assign must not fire: {out:?}");
    }

    #[test]
    fn value_mentioning_binding_silent() {
        // A per-element read the loop var participates in — fill cannot
        // express it (and `fill(<v>)` would not even compile).
        let input = "fn f(buf: &mut [f32]) { for v in &mut buf[..] { *v = *v * 2.0; } }";
        let out = propose_fix("manual_slice_fill", input);
        assert!(
            out.is_none(),
            "binding-mentioning value must not fire: {out:?}"
        );
    }

    #[test]
    fn multi_stmt_body_silent() {
        let input = "fn f(buf: &mut Vec<f32>) { for v in &mut buf[..] { *v = 0.0; log(v); } }";
        let out = propose_fix("manual_slice_fill", input);
        assert!(out.is_none(), "multi-stmt body must not fire: {out:?}");
    }

    #[test]
    fn non_assign_body_silent() {
        let input = "fn f(buf: &mut Vec<f32>) { for v in &mut buf[..] { *v += 1.0; } }";
        let out = propose_fix("manual_slice_fill", input);
        assert!(out.is_none(), "compound assign must not fire: {out:?}");
    }

    #[test]
    fn interior_comment_refused() {
        let input = "fn f(buf: &mut Vec<f32>) { for v in &mut buf[..] { /* c */ *v = 0.0; } }";
        let out = propose_fix("manual_slice_fill", input);
        assert!(out.is_none(), "interior comment must refuse: {out:?}");
    }

    #[test]
    fn unary_minus_literal_fires() {
        let input = "fn f(buf: &mut [f32]) { for v in &mut buf[..] { *v = -1.0; } }";
        let out = propose_fix("manual_slice_fill", input).expect("matched");
        assert!(out.contains("buf[..].fill(-1.0);"), "no fill: {out}");
    }
}
