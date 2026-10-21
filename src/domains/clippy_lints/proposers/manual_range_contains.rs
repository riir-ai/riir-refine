//! `clippy::manual_range_contains` — `x >= 3 && x <= 5` →
//! `(3..=5).contains(&x)` (+ the exclusive `x > lo && x < hi` →
//! `(lo..hi).contains(&x)` twin and both mirrored operand orders)
//! (Batch 158 / Plan 149 — the frontier-lead mining pack IV; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery, 2026-09-18).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `lo <= x && x <= hi` ≡ `(lo..=hi).contains(&x)` for EVERY `PartialOrd`
//! by std's own definition — `RangeInclusive::contains` is
//! `matches!(start.partial_cmp(item), Less | Equal) &&
//! matches!(item.partial_cmp(end), Less | Equal))`, i.e. exactly
//! `!(start > item) && !(item > end)`… the same two leg comparisons
//! (NaN: both sides `None`→false on both forms; ±∞: equal both sides).
//! The exclusive twin `lo < x && x < hi` ≡ `(lo..hi).contains(&x)`
//! identically (`Range::contains` = strictly-less both legs).
//!
//! The COMPILE-DIRECTION hole — a custom `E: PartialOrd<L>` (mixed types)
//! compiles the original but needs `L: PartialOrd<E>` for the range form —
//! is closed by SIGNATURE DERIVATION (the `slice_param_heads_over`
//! pattern): the shared operand must be a fn/impl-method param with an
//! EXPLICIT numeric/char/bool annotation, never rebound, and the bounds
//! must be LITERALS (int/float/char, optional sign). Then both range ends
//! unify with the operand's scalar type and every comparison impl in play
//! is same-type — the rewrite compiles and is value-identical whenever
//! the input did. Documented narrower-than-clippy divergence: variable
//! bounds (`x >= lo && x <= hi` with `lo`/`hi` bindings — clippy fires,
//! type-aware) refuse pending the same derivation for both bound idents.
//!
//! Shape: the `&&` of EXACTLY two comparison sub-expressions whose shared
//! operand text is byte-identical (the `identity_op` operand-text
//! precedent) and appears on the LEFT of both (`x >= lo && x <= hi` /
//! `x > lo && x < hi`) or the RIGHT of both (`lo <= x && hi >= x` /
//! `lo < x && hi > x`). Mixed >=/< pairs (`x >= lo && x < hi`) refuse —
//! no std range type is half-closed. Guards: leading-`&` operands refuse
//! (the emit would need `&&x`); attrs/turbofish N/A (plain comparisons);
//! comments inside the deleted glue (the operators + `&&`) refuse — the
//! three operand texts ride VERBATIM.
//!
//! The rewritten expr has the same `bool` type in every context the
//! original appeared in (if/while conditions, tail expressions, args,
//! nested `&&`/`||` chains — the emit replaces the whole `&&` node).

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, File, ImplItemFn, ItemFn};

use super::helpers::{byte_range, drops_interior_comment, scalar_param_heads_over};
use super::{edit, SourceEdit};

/// The scalar annotations `manual_range_contains` admits (the clean
/// same-type `PartialOrd` set — no `&str`/`String`: cross-type str
/// comparisons are `PartialOrd<&str>`/`PartialOrd<str>` impl pairs the range
/// form does not mirror).
const RANGE_SCALARS: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64", "bool", "char",
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

    fn visit_expr_binary(&mut self, b: &'ast ExprBinary) {
        syn::visit::visit_expr_binary(self, b);
        if let Some(replacement) = self.try_rewrite(b) {
            self.edits.push(edit(b, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, b: &ExprBinary) -> Option<String> {
        if !matches!(b.op, BinOp::And(_)) {
            return None;
        }
        let (l, lop) = as_cmp(&b.left)?;
        let (r, rop) = as_cmp(&b.right)?;

        // Form A — shared operand on the LEFT of both comparisons:
        // `x >= lo && x <= hi` (inclusive) / `x > lo && x < hi` (exclusive).
        let formed = match (lop, rop) {
            (BinOp::Ge(_), BinOp::Le(_)) => Some((l.0, r.0, l.1, r.1, true)),
            (BinOp::Gt(_), BinOp::Lt(_)) => Some((l.0, r.0, l.1, r.1, false)),
            _ => None,
        };
        let (lx, rx, lo, hi, inclusive) = match formed {
            Some(tuple) => tuple,
            // Form B — shared operand on the RIGHT of both:
            // `lo <= x && hi >= x` (inclusive) / `lo < x && hi > x` (exclusive).
            None => match (lop, rop) {
                (BinOp::Le(_), BinOp::Ge(_)) => (l.1, r.1, l.0, r.0, true),
                (BinOp::Lt(_), BinOp::Gt(_)) => (l.1, r.1, l.0, r.0, false),
                _ => return None,
            },
        };
        self.emit(b, lx, rx, lo, hi, inclusive)
    }

    /// Build the replacement when the shared-operand + bounds checks pass.
    fn emit(
        &self,
        and: &ExprBinary,
        lx: &Expr,
        rx: &Expr,
        lo: &Expr,
        hi: &Expr,
        inclusive: bool,
    ) -> Option<String> {
        let lx_range = byte_range(lx);
        let rx_range = byte_range(rx);
        let x = self.source.get(lx_range.clone())?;
        // One shared value: the operand TEXTS must be byte-identical (the
        // identity_op operand-text precedent).
        if x != self.source.get(rx_range.clone())? {
            return None;
        }
        // A bare ident — paren-wrapped forms refuse (the shape check
        // unwraps parens, but the EMIT requires the bare text so the
        // replacement is exactly `(lo..=hi).contains(&ident)`).
        let x_ident = ident_text(unwrap_parens(lx))?;
        if x != x_ident {
            return None;
        }
        let ty = self.proven.get(x_ident.as_str())?;
        if !RANGE_SCALARS.contains(&ty.as_str()) {
            return None;
        }
        // Bounds: literals only (sign-wrapped numerics + char literals).
        let lo_text = literal_text(lo, self.source)?;
        let hi_text = literal_text(hi, self.source)?;
        // Comments inside the deleted glue (the operators + `&&`) refuse;
        // the three emitted operand texts ride verbatim, so their ranges
        // are the keep-list (the SECOND x occurrence's bytes do not ride —
        // but a bare ident span cannot contain a comment, so it needs no
        // keep entry).
        let low_span = byte_range(lo);
        let high_span = byte_range(hi);
        if drops_interior_comment(
            self.source,
            byte_range(and),
            &[lx_range.clone(), low_span, high_span],
        ) {
            return None;
        }
        let dots = if inclusive { "..=" } else { ".." };
        Some(format!("({lo_text}{dots}{hi_text}).contains(&{x})"))
    }
}

/// A comparison sub-expression: `((lhs, rhs), op)` after paren unwrap.
fn as_cmp(e: &Expr) -> Option<((&Expr, &Expr), BinOp)> {
    let Expr::Binary(b) = unwrap_parens(e) else {
        return None;
    };
    let op @ (BinOp::Ge(_) | BinOp::Gt(_) | BinOp::Le(_) | BinOp::Lt(_)) = b.op else {
        return None;
    };
    Some(((&b.left, &b.right), op))
}

fn unwrap_parens(mut e: &Expr) -> &Expr {
    while let Expr::Paren(p) = e {
        e = &p.expr;
    }
    e
}

/// The bare ident of a single-segment plain path.
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

/// The source text of a bound: a numeric/char literal, optionally
/// sign-wrapped. Everything else refuses (variables, calls, fields — the
/// documented narrower-than-clippy divergence).
fn literal_text(e: &Expr, source: &str) -> Option<String> {
    let r = byte_range(e);
    let text = source.get(r)?;
    match unwrap_parens(e) {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(_) | syn::Lit::Float(_) | syn::Lit::Char(_) => Some(text.to_string()),
            _ => None,
        },
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => match &*u.expr {
            Expr::Lit(l) => match &l.lit {
                syn::Lit::Int(_) | syn::Lit::Float(_) => Some(text.to_string()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn inclusive_left_form_fires() {
        let out = propose_fix(
            "manual_range_contains",
            "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && x <= 5 }",
        )
        .expect("matched");
        assert!(
            out.contains("(3..=5).contains(&x)"),
            "not rewritten to contains: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn inclusive_right_form_fires() {
        let out = propose_fix(
            "manual_range_contains",
            "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); 3 <= x && 5 >= x }",
        )
        .expect("matched");
        assert!(
            out.contains("(3..=5).contains(&x)"),
            "not rewritten to contains: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn exclusive_left_form_fires() {
        let out = propose_fix(
            "manual_range_contains",
            "fn in_open(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x > 3 && x < 5 }",
        )
        .expect("matched");
        assert!(
            out.contains("(3..5).contains(&x)"),
            "not rewritten to exclusive contains: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn float_and_char_params_fire() {
        let out = propose_fix(
            "manual_range_contains",
            "fn band(x: f32, hits: &mut Vec<f32>) -> bool { hits.push(x); x >= 0.0 && x <= 1.0 }",
        )
        .expect("matched");
        assert!(
            out.contains("(0.0..=1.0).contains(&x)"),
            "float band not rewritten: {out}"
        );
        let out = propose_fix(
            "manual_range_contains",
            "fn zone(c: char, hits: &mut Vec<char>) -> bool { hits.push(c); c >= 'a' && c <= 'z' }",
        )
        .expect("matched");
        assert!(
            out.contains("('a'..='z').contains(&c)"),
            "char band not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negative_bounds_ride_verbatim() {
        let out = propose_fix(
            "manual_range_contains",
            "fn band(x: i32, hits: &mut Vec<i32>) -> bool { hits.push(x); x >= -3 && x <= -1 }",
        )
        .expect("matched");
        assert!(
            out.contains("(-3..=-1).contains(&x)"),
            "negative bounds must ride verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn while_condition_fires() {
        let out = propose_fix(
            "manual_range_contains",
            "fn go(x: u8, hits: &mut Vec<u8>) { hits.push(x); while x >= 3 && x <= 5 { break; } }",
        )
        .expect("matched");
        assert!(
            out.contains("while (3..=5).contains(&x)"),
            "while condition not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn tail_of_conjunction_chain_fires() {
        // `a && x>=3 && x<=5` groups as `(a && x>=3) && x<=5` — no pair.
        // But `x>=3 && x<=5 && a` groups as `(x>=3 && x<=5) && a` — the
        // inner pair fires.
        let out = propose_fix(
            "manual_range_contains",
            "fn go(x: u8, a: bool, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && x <= 5 && a }",
        )
        .expect("matched");
        assert!(
            out.contains("(3..=5).contains(&x) && a"),
            "conjunction tail not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mixed_pair_refuses() {
        let out = propose_fix(
            "manual_range_contains",
            "fn half(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && x < 5 }",
        );
        assert!(out.is_none(), "half-open pair must refuse: {out:?}");
    }

    #[test]
    fn differing_operands_refuse() {
        let out = propose_fix(
            "manual_range_contains",
            "fn go(x: u8, y: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && y <= 5 }",
        );
        assert!(out.is_none(), "differing operands must refuse: {out:?}");
    }

    #[test]
    fn unproven_operand_refuses() {
        // A local binding — no signature derivation, refuse.
        let out = propose_fix(
            "manual_range_contains",
            "fn go(hits: &mut Vec<u8>) -> bool { let x = 3u8; hits.push(x); x >= 3 && x <= 5 }",
        );
        assert!(out.is_none(), "unproven operand must refuse: {out:?}");
    }

    #[test]
    fn variable_bounds_refuse() {
        let out = propose_fix(
            "manual_range_contains",
            "fn go(x: u8, lo: u8, hi: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= lo && x <= hi }",
        );
        assert!(out.is_none(), "variable bounds must refuse: {out:?}");
    }

    #[test]
    fn non_numeric_param_refuses() {
        // &str params are in the scalar derivation's output set but NOT in
        // the range set (cross-type str comparisons).
        let out = propose_fix(
            "manual_range_contains",
            "fn zone(s: &str, hits: &mut Vec<u8>) -> bool { hits.push(1); s >= \"a\" && s <= \"z\" }",
        );
        assert!(out.is_none(), "&str operand must refuse: {out:?}");
    }

    #[test]
    fn leading_ref_operand_refuses() {
        let out = propose_fix(
            "manual_range_contains",
            "fn go(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); &x >= &3 && &x <= &5 }",
        );
        assert!(out.is_none(), "leading-& operands must refuse: {out:?}");
    }

    #[test]
    fn comment_in_glue_refuses() {
        let out = propose_fix(
            "manual_range_contains",
            "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 /* lo */ && x <= 5 }",
        );
        assert!(
            out.is_none(),
            "comment in deleted glue must refuse: {out:?}"
        );
    }

    #[test]
    fn or_chain_refuses() {
        let out = propose_fix(
            "manual_range_contains",
            "fn out(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x < 3 || x > 5 }",
        );
        assert!(
            out.is_none(),
            "|| chain is the outside-range form — never ours: {out:?}"
        );
    }

    #[test]
    fn impl_method_param_derivation() {
        let out = propose_fix(
            "manual_range_contains",
            "struct G;\nimpl G {\n    fn band(&self, x: u16, hits: &mut Vec<u16>) -> bool { hits.push(x); x >= 7 && x <= 9 }\n}",
        )
        .expect("matched");
        assert!(
            out.contains("(7..=9).contains(&x)"),
            "impl-method param not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rebound_param_refuses() {
        let out = propose_fix(
            "manual_range_contains",
            "fn go(x: u8, hits: &mut Vec<u8>) -> bool { let x = 1u8; hits.push(x); x >= 3 && x <= 5 }",
        );
        assert!(out.is_none(), "rebound param must refuse: {out:?}");
    }

    #[test]
    fn paren_operand_texts_ok() {
        // Paren-wrapped shared operand: text is `(x)` — the bare-ident
        // check unwraps for shape, but the EMIT requires the bare text,
        // so this refuses (conservative).
        let out = propose_fix(
            "manual_range_contains",
            "fn go(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); (x) >= 3 && (x) <= 5 }",
        );
        assert!(
            out.is_none(),
            "paren-wrapped operand text must refuse: {out:?}"
        );
    }
}
