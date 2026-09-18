//! `clippy::manual_clamp` — `x.max(3).min(5)` → `x.clamp(3, 5)`
//! (+ the mirrored `x.min(5).max(3)` order twin)
//! (Batch 159 / Plan 150 — the frontier-lead mining pack V; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery AND the
//! default-groups arm, 2026-09-18).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `min(max(x,lo),hi)` ≡ `max(min(x,hi),lo)` ≡ `clamp(x,lo,hi)` for every
//! TOTAL order with `lo <= hi` — the lattice absorption law. On the
//! admitted scalars (integers + `char`), `Ord::max`/`Ord::min`/`Ord::clamp`
//! are all defined through the SAME `partial_cmp` impl, so value-identity
//! is construction, not approximation. TWO real divergences REFUSE by
//! design:
//!
//! 1. **FLOATS — the NaN anti-regime witness.** `f64::max`/`min` IGNORE
//!    NaN (`NaN.max(3.0) == 3.0`) while `clamp` PROPAGATES it
//!    (`NaN.clamp(3.0, 5.0) == NaN`) — the manual chain and clamp differ
//!    on NaN input, so `f32`/`f64` receivers refuse (documented
//!    narrower-than-clippy divergence).
//! 2. **PANIC — the `lo > hi` guard.** `clamp` asserts `min <= max`
//!    (release too) while the manual chain silently returns `hi`; closed
//!    by LITERAL-bounds-only with a decidable `lo <= hi` comparison (int
//!    digits as i128 / char as u32). An inverted or equal-kind-mixed pair
//!    refuses.
//!
//! Shape: the outer `.min(hi)` / `.max(lo)` call whose receiver is the
//! inner `.max(lo)` / `.min(hi)` call (either order), whose receiver is
//! the bare proven ident. Bounds are literals (sign-wrapped ints, char
//! literals — optional int suffixes ride). Guards: turbofish on either
//! call refuses; attrs refuse; comments inside the deleted glue refuse
//! (keep-list = the three operand spans); variable bounds refuse (the
//! `manual_range_contains` divergence precedent); `bool` receivers refuse
//! (derivable but zero real-world shape — outside `CLAMP_SCALARS`).

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, ImplItemFn, ItemFn};

use super::helpers::{byte_range, drops_interior_comment, scalar_param_heads_over};
use super::{edit, SourceEdit};

/// The scalar annotations `manual_clamp` admits — the TOTAL-order set
/// (ints + char). `f32`/`f64` refuse (the NaN divergence); `bool` refuses
/// (no real-world shape).
const CLAMP_SCALARS: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    "char",
];

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
    /// Signature-proven scalar bindings of the enclosing fn (type text).
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

    fn visit_expr_method_call(&mut self, c: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, c);
        if let Some(replacement) = self.try_rewrite(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    /// `c` is the OUTER call (`.min(hi)` or `.max(lo)`); its receiver must
    /// be the inner call of the complementary method.
    fn try_rewrite(&self, c: &ExprMethodCall) -> Option<String> {
        if c.method != "min" && c.method != "max" {
            return None;
        }
        if c.args.len() != 1 || c.turbofish.is_some() || !c.attrs.is_empty() {
            return None;
        }
        let outer_is_min = c.method == "min";
        let outer_arg = c.args.first()?;
        // The inner call: the complementary method on the proven ident.
        let Expr::MethodCall(inner) = unwrap_parens(&c.receiver) else {
            return None;
        };
        if inner.turbofish.is_some() || !inner.attrs.is_empty() {
            return None;
        }
        let inner_is_max = inner.method == "max";
        if inner.method != "max" && inner.method != "min" {
            return None;
        }
        // Complementary pair only: min(max(..)) or max(min(..)).
        if outer_is_min != inner_is_max {
            return None;
        }
        let inner_arg = inner.args.first()?;
        if inner.args.len() != 1 {
            return None;
        }
        // The innermost receiver: the bare proven ident.
        let x_text = ident_text(unwrap_parens(&inner.receiver))?;
        if !self
            .proven
            .get(x_text.as_str())
            .is_some_and(|ty| CLAMP_SCALARS.contains(&ty.as_str()))
        {
            return None;
        }
        // (lo, hi) by chain role: inner max binds lo; outer min binds hi.
        let (lo, hi) = if inner_is_max {
            (inner_arg, outer_arg)
        } else {
            (outer_arg, inner_arg)
        };
        let lo_lit = bound_literal(lo)?;
        let hi_lit = bound_literal(hi)?;
        if !bounds_ordered(&lo_lit, &hi_lit)? {
            return None;
        }
        // Comments inside the deleted glue refuse; the three operand texts
        // ride verbatim — their spans are the keep-list.
        let x_range = byte_range(unwrap_parens(&inner.receiver));
        let lo_range = byte_range(lo);
        let hi_range = byte_range(hi);
        if drops_interior_comment(
            self.source,
            byte_range(c),
            &[x_range, lo_range, hi_range],
        ) {
            return None;
        }
        let lo_text = self.source.get(byte_range(lo))?;
        let hi_text = self.source.get(byte_range(hi))?;
        Some(format!("{x_text}.clamp({lo_text}, {hi_text})"))
    }
}

/// A comparison bound: a numeric or char literal, sign-wrapped numerics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bound {
    Int(i128),
    Char(u32),
}

/// The literal value of a bound + its verbatim-text guarantee. Everything
/// else refuses (variables, calls, fields — the `manual_range_contains`
/// divergence precedent).
fn bound_literal(e: &Expr) -> Option<Bound> {
    match unwrap_parens(e) {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(i) => i.base10_digits().parse::<i128>().ok().map(Bound::Int),
            syn::Lit::Char(c) => Some(Bound::Char(c.value() as u32)),
            _ => None,
        },
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => match &*u.expr {
            Expr::Lit(l) => match &l.lit {
                syn::Lit::Int(i) => i
                    .base10_digits()
                    .parse::<i128>()
                    .ok()
                    .and_then(i128::checked_neg)
                    .map(Bound::Int),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// The `lo <= hi` panic guard (clamp asserts `min <= max`, release too —
/// the manual chain never panics, so an inverted pair is a behavior
/// change and refuses). Mixed kinds refuse (int vs char cannot both
/// unify with one receiver scalar anyway).
fn bounds_ordered(lo: &Bound, hi: &Bound) -> Option<bool> {
    match (lo, hi) {
        (Bound::Int(a), Bound::Int(b)) => Some(a <= b),
        (Bound::Char(a), Bound::Char(b)) => Some(a <= b),
        _ => None,
    }
}

fn unwrap_parens(mut e: &Expr) -> &Expr {
    while let Expr::Paren(p) = e {
        e = &p.expr;
    }
    e
}

/// The bare ident of a single-segment plain path (the per-file ident
/// helper pattern).
fn ident_text(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn max_then_min_fires() {
        let out = propose_fix(
            "manual_clamp",
            "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(3).min(5) }",
        )
        .expect("matched");
        assert!(
            out.contains("x.clamp(3, 5)"),
            "not rewritten to clamp: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn min_then_max_fires() {
        let out = propose_fix(
            "manual_clamp",
            "fn roof(n: i32, hits: &mut Vec<i32>) -> i32 { hits.push(n); n.min(9).max(7) }",
        )
        .expect("matched");
        assert!(
            out.contains("n.clamp(7, 9)"),
            "mirrored order not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unsigned_and_char_receivers_fire() {
        let out = propose_fix(
            "manual_clamp",
            "fn cap(v: u8, hits: &mut Vec<u8>) -> u8 { hits.push(v); v.max(1).min(9) }",
        )
        .expect("matched");
        assert!(
            out.contains("v.clamp(1, 9)"),
            "unsigned receiver not rewritten: {out}"
        );
        let out = propose_fix(
            "manual_clamp",
            "fn zone(c: char, hits: &mut Vec<char>) -> char { hits.push(c); c.max('a').min('z') }",
        )
        .expect("matched");
        assert!(
            out.contains("c.clamp('a', 'z')"),
            "char receiver not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negative_and_suffixed_bounds_ride_verbatim() {
        let out = propose_fix(
            "manual_clamp",
            "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(-5).min(5) }",
        )
        .expect("matched");
        assert!(
            out.contains("x.clamp(-5, 5)"),
            "negative bound must ride verbatim: {out}"
        );
        let out = propose_fix(
            "manual_clamp",
            "fn cap(v: u8, hits: &mut Vec<u8>) -> u8 { hits.push(v); v.max(1u8).min(9u8) }",
        )
        .expect("matched");
        assert!(
            out.contains("v.clamp(1u8, 9u8)"),
            "suffixed bounds must ride verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn equal_bounds_fire() {
        // lo == hi passes the panic guard (assert min <= max is <=, not <).
        let out = propose_fix(
            "manual_clamp",
            "fn pin(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(4).min(4) }",
        )
        .expect("matched");
        assert!(
            out.contains("x.clamp(4, 4)"),
            "equal bounds must fire: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_derivation() {
        let out = propose_fix(
            "manual_clamp",
            "struct W { n: u8 }\nimpl W {\n    fn band(&self, x: i32) -> i32 { x.max(3).min(5) }\n}",
        )
        .expect("matched");
        assert!(
            out.contains("x.clamp(3, 5)"),
            "impl-method param not derived: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_larger_expr_fires() {
        let out = propose_fix(
            "manual_clamp",
            "fn calc(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(3).min(5) + 1 }",
        )
        .expect("matched");
        assert!(
            out.contains("x.clamp(3, 5) + 1"),
            "sub-expression not rewritten in place: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn float_receiver_refuses() {
        // The NaN anti-regime witness: NaN.max(3.0).min(5.0) == 3.0 but
        // NaN.clamp(3.0, 5.0) == NaN — a real behavior change.
        assert!(propose_fix(
            "manual_clamp",
            "fn band(x: f64, hits: &mut Vec<f64>) -> f64 { hits.push(x); x.max(3.0).min(5.0) }",
        )
        .is_none());
    }

    #[test]
    fn inverted_bounds_refuse() {
        // clamp asserts min <= max (release too); the manual chain never
        // panics — an inverted pair is a behavior change.
        assert!(propose_fix(
            "manual_clamp",
            "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(5).min(3) }",
        )
        .is_none());
    }

    #[test]
    fn variable_and_unproven_bounds_refuse() {
        assert!(propose_fix(
            "manual_clamp",
            "fn band(x: i32, lo: i32, hi: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(lo).min(hi) }",
        )
        .is_none());
        // Unproven receiver (generic param — the derivation's soundness leg).
        assert!(propose_fix(
            "manual_clamp",
            "fn band<T: Ord>(x: T) -> T { x.max(3).min(5) }",
        )
        .is_none());
    }

    #[test]
    fn non_ident_receiver_refuses() {
        assert!(propose_fix(
            "manual_clamp",
            "fn at(v: Vec<i32>) -> i32 { v[0].max(3).min(5) }",
        )
        .is_none());
    }

    #[test]
    fn same_method_pair_and_triple_chain_refuse() {
        // max+max is not the clamp shape.
        assert!(propose_fix(
            "manual_clamp",
            "fn band(x: i32) -> i32 { x.max(3).max(5) }",
        )
        .is_none());
        // A three-deep chain — the outer rewrite would leave a half-shape;
        // the outermost pair matches only if it IS a clamp pair.
        let out = propose_fix(
            "manual_clamp",
            "fn odd(x: i32) -> i32 { x.max(3).min(5).max(1) }",
        );
        assert!(out.is_some());
        assert!(
            out.as_ref().is_some_and(|o| o.contains("x.clamp(3, 5).max(1)")),
            "sub-chain must rewrite in place: {}",
            out.unwrap_or_default()
        );
    }

    #[test]
    fn comment_in_glue_refuses() {
        assert!(propose_fix(
            "manual_clamp",
            "fn band(x: i32) -> i32 { x.max(/* lo */ 3).min(5) }",
        )
        .is_none());
    }
}
