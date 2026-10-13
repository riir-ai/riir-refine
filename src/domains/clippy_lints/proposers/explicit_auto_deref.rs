//! `clippy::explicit_auto_deref` — an explicit deref that the ambient
//! auto-deref/deref-coercion would perform anyway: `f(&*e)` → `f(&e)` and
//! `f(&**e)` → `f(e)` (P51a, 2026-10-14; complexity group per the pinned
//! 1.98.1 driver; probe-verified against the driver's own suggestions —
//! BOTH shapes `MachineApplicable`, the deref-COUNT selects the rewrite:
//! one deref collapses to `&e`, two to `e`).
//!
//! ## The deref-count law
//!
//! The two production specimens (cycle 71, both healed by the driver's
//! suggestions): `bare_ident(&*m.receiver)` → `&m.receiver` (the receiver
//! is an owned `Box<Expr>` — one deref, keep the `&`) and
//! `self.visit_expr(&**else_e)` → `else_e` (`else_e` is a `&Box<Expr>`
//! match-ergonomics binding — two derefs, drop the whole `&**`). The
//! driver has type info to know WHICH collapse is right; this proposer
//! takes the deterministic syntactic law — count the derefs under the
//! reference and splice accordingly:
//!
//! - `&*e`  → `&e`   (one `*` deleted; `e`'s bytes verbatim)
//! - `&**e` → `e`    (the `&**` prefix deleted; `e`'s bytes verbatim)
//!
//! Any deeper chain (`&***e` — three derefs exist for `&&Box<T>`-class
//! operands) refuses: the collapse target stops being derivable from the
//! count alone at the shapes that did not fire upstream.
//!
//! ## Why it is safe when it compiles (the VERIFY bar)
//!
//! For a CONCRETE reference-taking callee the rewrite is a no-op
//! adjustment: the original `f(&*e)` compiling proves the callee's param
//! accepts `&Target(e::Deref)`; the rewrite passes `&e` (or `e`) and the
//! arg-position deref coercion walks the SAME one-step (or two-step)
//! Deref chain — same pointer, same borrow, no move of `e` itself (`&e`
//! borrows; `&**e`→`e` where the two-deref shape proves `e` is a
//! reference place — references are `Copy`).
//!
//! ## Documented divergences
//!
//! - **The generic-callee class (SILENT)**: `fn g<T>(x: T)` binds `T =
//!   &Target` for the original and `T = &e`-type for the rewrite — both
//!   compile, the callee receives a different type. Upstream's typeck
//!   never suggests there; a parse-only proposer cannot see the callee.
//!   Disclosed: the `--verify` compile gate catches every concrete-callee
//!   mis-fire (E0308) and this silent class is the residual risk, bounded
//!   by how rare `&*`/`&**` args to GENERIC callees are (the specimens
//!   were concrete-typed helpers).
//! - **Let-init positions refuse**: `let x: &u8 = &*b;` — the annotation
//!   makes the coercion site explicit and the rewrite would change the
//!   inferred binding type on UN-annotated lets (`let x = &*b` binds
//!   `&u8`, `let x = &b` binds `&Box<u8>` — silently different). Only the
//!   argument-position shape (call, method-call, and macro-expression
//!   args) fires — positions where a callee (and its coercion site)
//!   exists.
//! - Receiver position (`(*e).method()` — no `&`) is a DIFFERENT upstream
//!   shape and stays silent (v1 scope).
//! - `&mut *e` and any `&mut` form refuse (reborrow semantics differ).
//! - Attributes on the reference or a comment under the splice refuse
//!   (only `e`'s bytes survive).
//!
//! Interior-comment guard: the operand's bytes are the only survivors.

use syn::visit::Visit;
use syn::{Expr, File};

use super::helpers::{byte_range, drops_interior_comment};
use super::{src, SourceEdit};

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
    /// Only ARGUMENT positions: the parent visit hands us every call /
    /// method-call; we inspect each argument for the reference-deref shape.
    /// (A top-down `visit_expr` on the argument itself cannot know it sits
    /// in argument position — the eligibility is the PARENT's property.)
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        for a in &c.args {
            self.try_argument(a);
        }
        syn::visit::visit_expr_call(self, c);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        for a in &m.args {
            self.try_argument(a);
        }
        // The receiver position is the DIFFERENT upstream shape — silent.
        syn::visit::visit_expr_method_call(self, m);
    }
}

impl Collector<'_> {
    fn try_argument(&mut self, arg: &Expr) {
        let Some(se) = self.try_collapse(arg) else {
            return;
        };
        self.edits.push(se);
    }

    /// `&*e` → `&e` / `&**e` → `e` on one argument expression.
    fn try_collapse(&self, arg: &Expr) -> Option<SourceEdit> {
        let Expr::Reference(r) = arg else {
            return None;
        };
        if r.mutability.is_some() || !r.attrs.is_empty() {
            return None;
        }
        let collapse = collapse_kind(&r.expr)?;
        // Peel EXACTLY `collapse` derefs to the surviving operand.
        let mut operand: &Expr = &r.expr;
        for _ in 0..collapse {
            let Expr::Unary(u) = operand else {
                return None;
            };
            operand = &*u.expr;
        }
        let kept = [byte_range(operand)];
        if drops_interior_comment(self.source, byte_range(arg), &kept) {
            return None;
        }
        let text = src(self.source, operand)?;
        let replacement = match collapse {
            1 => format!("&{text}"),
            _ => text,
        };
        Some(SourceEdit {
            start: byte_range(arg).start,
            end: byte_range(arg).end,
            replacement,
        })
    }
}

/// How many derefs sit under the reference — 1 or 2 (deeper refuses).
fn collapse_kind(inner: &Expr) -> Option<u8> {
    let Expr::Unary(u1) = inner else {
        return None;
    };
    if !matches!(u1.op, syn::UnOp::Deref(_)) {
        return None;
    }
    if let Expr::Unary(u2) = &*u1.expr {
        if matches!(u2.op, syn::UnOp::Deref(_)) {
            // Three derefs refuse.
            if matches!(&*u2.expr, Expr::Unary(u3) if matches!(u3.op, syn::UnOp::Deref(_))) {
                return None;
            }
            return Some(2);
        }
    }
    Some(1)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn single_deref_collapses_to_ref() {
        // Specimen 1 (assert_is_empty.rs, cycle 71): `&*m.receiver` on an
        // owned Box — the driver's suggestion was `&m.receiver`.
        let input = "fn g(m: &M) -> Option<u8> { bare_ident(&*m.receiver) } fn bare_ident(e: &Expr) -> Option<u8> { None }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(
            out.contains("bare_ident(&m.receiver)"),
            "one deref keeps the &: {out}"
        );
        assert!(!out.contains("&*"), "the deref is gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn double_deref_collapses_to_operand() {
        // Specimen 2 (block_scrutinee.rs, cycle 71): `&**else_e` on a
        // &Box binding — the driver's suggestion was `else_e`.
        let input = "fn f(s: &S) { let Some(e) = &s.opt else { return }; visit(&**e) }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(
            out.contains("visit(e)"),
            "two derefs drop the whole &**: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_call_argument_fires() {
        let input = "fn f(m: &M) { self.visit_expr(&**m.else_e) }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(out.contains("self.visit_expr(m.else_e)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_init_position_refused() {
        // The inference-position divergence: `let x = &*b` binds &u8;
        // `let x = &b` would bind &Box<u8> — silently different.
        let input = "fn f(b: Box<u8>) { let x = &*b; takes(x) } fn takes(e: &u8) {}";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(out.is_none(), "let-init positions refuse: {out:?}");
    }

    #[test]
    fn mut_reference_refused() {
        let input = "fn f(b: &mut Box<u8>) { takes(&mut *b) } fn takes(e: &mut u8) {}";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(out.is_none(), "&mut forms refuse (reborrow semantics): {out:?}");
    }

    #[test]
    fn three_deref_chain_refused() {
        let input = "fn f(r: &&Box<u8>) { takes(&***r) } fn takes(e: &u8) {}";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(out.is_none(), "chains deeper than two derefs refuse: {out:?}");
    }

    #[test]
    fn receiver_position_refused() {
        let input = "fn f(b: Box<Vec<u8>>) { let n = (*b).len(); let _ = n; }";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(out.is_none(), "the receiver shape is v1-silent: {out:?}");
    }
}
