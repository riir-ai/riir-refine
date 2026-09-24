//! `clippy::cast_abs_to_unsigned` — `x.abs() as uN` → `x.unsigned_abs()`
//! (+ the kept cast on a width change) (Batch 175 / Plan 169 — the
//! coverage-report re-arm pack XV; the family fixtures verified to fire
//! under the plain oracle, 2026-09-24, `scripts/plan169_pack15_probe.py`).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! `iN::unsigned_abs()` returns the unsigned twin width `uN`, and for every
//! input its value is `x.abs() as uN`: release-mode `abs` of `iN::MIN` is
//! `MIN` (wrapping), `MIN as uN` is `2^(N-1)`, and `unsigned_abs()` returns
//! `2^(N-1)` — values IDENTICAL for every input in release. In debug the
//! fix is strictly BETTER: `iN::MIN.abs()` panics before the cast while
//! `unsigned_abs()` returns the value — that "can panic" term is the lint's
//! own motivation. On a width CHANGE (narrowing `i64→u8` or widening
//! `i32→u64`) the cast carries the conversion, so only the `abs` half is
//! rewritten and the cast survives — the three-regime rule is: whole-expr
//! replacement iff `unsigned_abs()`'s return width equals the cast target
//! width (probe span-dumped for all three regimes).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - **Receiver provenance is PARAM-only**: the receiver must be a bare
//!   ident proven by the enclosing fn's signature
//!   (`scalar_param_heads_over` — the never-rebound guard makes the
//!   annotation a construction fact) with a PRIMITIVE SIGNED int type
//!   (`i8..i128`/`isize`; `u*` has no `.abs()` — E0599 — and `f64::abs`
//!   is a different method). Parenthesized receivers, field/call
//!   receivers, and annotated-let operands refuse (the `unnecessary_cast`
//!   v1 posture).
//! - The method must be exactly `abs`, zero-arg, no turbofish, no attrs.
//! - The cast target must be a single-segment UNSIGNED path type
//!   (`u8..u128`/`usize`); signed targets are other lints' territory
//!   (`cast_lossless` / `unnecessary_cast`), pointer/reference targets
//!   refuse by construction.
//! - Comments inside a replaced span refuse (the emitter would delete
//!   them outright).
//! - Nested casts resolve at the INNER cast (`x.abs() as u8 as u16` with
//!   `x: i64` rewrites the inner call, both casts survive) — the outer
//!   cast's operand is a Cast, not a method call, and matching it would
//!   double-count the same panic bound.

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprCast, ExprMethodCall, File, ImplItemFn, ItemFn};

use super::helpers::{byte_range, drops_interior_comment, scalar_param_heads_over};
use super::{edit, SourceEdit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        proven: BTreeMap::new(),
        saved: BTreeMap::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Signature-proven bindings of the enclosing fn (type text).
    proven: BTreeMap<String, String>,
    /// The set to restore on fn exit (one-deep save).
    saved: BTreeMap<String, String>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = scalar_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = scalar_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_expr_cast(&mut self, c: &'ast ExprCast) {
        syn::visit::visit_expr_cast(self, c);
        if let Some(e) = self.try_rewrite(c) {
            self.edits.push(e);
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, c: &ExprCast) -> Option<SourceEdit> {
        // The target must be a plain single-segment UNSIGNED path type.
        let target = cast_target_ident(c)?;
        if !UNSIGNED_TARGETS.contains(&target.as_str()) {
            return None;
        }
        // The operand must be exactly `ident.abs()` with the ident proven
        // a primitive SIGNED int by the enclosing fn head.
        let mc = method_abs_call(&c.expr)?;
        let ident = method_ident_receiver(mc)?;
        let proven_ty = self.proven.get(ident.as_str())?;
        let unsigned = signed_unsigned_twin(proven_ty)?;

        // The three-regime rule: whole-expr replacement iff the unsigned
        // twin width equals the cast target width; else the cast carries
        // the conversion and only the `abs` half is rewritten.
        let replacement = format!("{ident}.unsigned_abs()");
        if unsigned == target {
            if drops_interior_comment(self.source, byte_range(c), &[byte_range(&mc.receiver)]) {
                return None;
            }
            Some(edit(c, replacement))
        } else {
            if drops_interior_comment(self.source, byte_range(mc), &[byte_range(&mc.receiver)]) {
                return None;
            }
            Some(edit(mc, replacement))
        }
    }
}

/// The single-segment no-arg path ident of a cast target, else `None`.
fn cast_target_ident(c: &ExprCast) -> Option<String> {
    let syn::Type::Path(tp) = &*c.ty else { return None };
    if tp.qself.is_some() || tp.path.leading_colon.is_some() {
        return None;
    }
    if tp.path.segments.len() != 1 || !tp.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(tp.path.segments[0].ident.to_string())
}

/// The zero-arg `abs()` method-call shape (no turbofish, no attrs).
fn method_abs_call(e: &Expr) -> Option<&ExprMethodCall> {
    let Expr::MethodCall(mc) = e else { return None };
    if mc.method != "abs" || !mc.args.is_empty() || mc.turbofish.is_some() || !mc.attrs.is_empty() {
        return None;
    }
    Some(mc)
}

/// The BARE-ident receiver spelling (`x.abs()`); parenthesized, field,
/// call, and path-expression receivers all refuse (the provenance map
/// speaks only for fn-head params).
fn method_ident_receiver(mc: &ExprMethodCall) -> Option<String> {
    let Expr::Path(p) = &*mc.receiver else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

/// The unsigned twin of a primitive SIGNED int type text; every other
/// type text refuses (`u*` has no `.abs()` to rewrite, `f64::abs` is a
/// different method, non-primitives are unprovable here).
fn signed_unsigned_twin(ty: &str) -> Option<&'static str> {
    match ty {
        "i8" => Some("u8"),
        "i16" => Some("u16"),
        "i32" => Some("u32"),
        "i64" => Some("u64"),
        "i128" => Some("u128"),
        "isize" => Some("usize"),
        _ => None,
    }
}

/// The unsigned spellings the cast target admits.
const UNSIGNED_TARGETS: &[&str] = &["u8", "u16", "u32", "u64", "u128", "usize"];

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> i64 { hits.push(0); x.abs() }",
        )
        .is_none());
    }

    #[test]
    fn same_width_drops_the_cast() {
        let out = propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); x.abs() as u64 }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); x.unsigned_abs() }"), "{out}");
        assert!(!out.contains("as u64"), "cast must drop: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn narrowing_keeps_the_cast() {
        let out = propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> u8 { hits.push(0); x.abs() as u8 }",
        )
        .expect("matched");
        assert!(
            out.contains("hits.push(0); x.unsigned_abs() as u8 }"),
            "cast must survive: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn widening_keeps_the_cast() {
        let out = propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i32) -> u64 { hits.push(0); x.abs() as u64 }",
        )
        .expect("matched");
        assert!(
            out.contains("hits.push(0); x.unsigned_abs() as u64 }"),
            "cast must survive: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn isize_to_usize_drops_the_cast() {
        let out = propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: isize) -> usize { hits.push(0); x.abs() as usize }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); x.unsigned_abs() }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_cast_rewrites_the_inner_call() {
        // The outer cast's operand is a Cast, not a method call — the
        // inner (width-changing) arm fires and BOTH casts survive.
        let out = propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> u16 { hits.push(0); x.abs() as u8 as u16 }",
        )
        .expect("matched");
        assert!(
            out.contains("hits.push(0); x.unsigned_abs() as u8 as u16 }"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_proven() {
        let out = propose_fix(
            "cast_abs_to_unsigned",
            "struct S; impl S { fn f(&self, hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); x.abs() as u64 } }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); x.unsigned_abs() }"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unproven_receiver_refuses() {
        // A call result has no signature provenance.
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn mk() -> i64 { 3 } fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); mk().abs() as u64 }",
        )
        .is_none());
    }

    #[test]
    fn let_bound_receiver_refuses_v1() {
        // Annotated-LET provenance is the documented v1 divergence (params
        // only) — upstream fires here; we refuse until the let-collector.
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x: i32 = -5; x.abs() as u32 }",
        )
        .is_none());
    }

    #[test]
    fn unsigned_receiver_refuses() {
        // `u32` has no `.abs()` — the shape cannot compile, and the
        // provenance gate refuses it regardless.
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x.abs() as u32 }",
        )
        .is_none());
    }

    #[test]
    fn float_receiver_refuses() {
        // `f64::abs` is a different method with a different fix seam.
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: f64) -> u64 { hits.push(0); x.abs() as u64 }",
        )
        .is_none());
    }

    #[test]
    fn signed_target_refuses() {
        // `i64` targets are other lints' territory.
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i32) -> i64 { hits.push(0); x.abs() as i64 }",
        )
        .is_none());
    }

    #[test]
    fn parenthesized_receiver_refuses() {
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); (x).abs() as u64 }",
        )
        .is_none());
    }

    #[test]
    fn non_abs_method_refuses() {
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); x.wrapping_abs() as u64 }",
        )
        .is_none());
    }

    #[test]
    fn argful_abs_refuses() {
        // `.abs()` is zero-arg; anything else is a different method call.
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); x.abs(2) as u64 }",
        )
        .is_none());
    }

    #[test]
    fn field_receiver_refuses() {
        // `self.x` is not a proven param ident.
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "struct S { x: i64 } impl S { fn f(&self, hits: &mut Vec<u8>) -> u64 { hits.push(0); self.x.abs() as u64 } }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "cast_abs_to_unsigned",
            "fn f(hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); x.abs() /* why */ as u64 }",
        )
        .is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let src = "fn f(hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); x.abs() as u64 }";
        let first = propose_fix("cast_abs_to_unsigned", src).expect("matched");
        assert!(propose_fix("cast_abs_to_unsigned", &first).is_none());
    }
}
