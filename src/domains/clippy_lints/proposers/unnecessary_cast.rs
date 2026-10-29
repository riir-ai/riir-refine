//! `clippy::unnecessary_cast` — `x as T` → `x` on a proven-same-type operand;
//! `5 as u64` → `5_u64` on an unsuffixed literal (Batch 167 / Plan 158 — the
//! coverage-report re-arm pack XI; the family fixture verified to fire
//! exactly this lint under all+pedantic+nursery with a clean fixed form,
//! 2026-09-21, `scripts/plan158_frontier_probe.py`).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! `expr as T` where the operand's static type IS `T` is the identity cast:
//! the expression's value, type, and place-expression status are unchanged by
//! a transmute-free same-type cast, so dropping it rewrites nothing but the
//! syntax. On an UNSIZED integer/float literal the cast is what types the
//! literal — but the literal can carry that type itself (`5 as u64` IS
//! `5_u64`, bit-and-type-identical by the reference's literal-suffix rule),
//! so the suffix substitution is the same construction under a second
//! spelling.
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - **v1 provenance is PARAM-only**: the operand must be a BARE IDENT proven
//!   by the enclosing fn's signature (`scalar_param_heads_over` — the
//!   never-rebound guard makes the annotation a construction fact), with the
//!   proven type text EXACTLY equal to the cast target. Annotated-let
//!   provenance (which upstream also fires on) is a later rider — widening
//!   needs a let-annotation collector; the params-only derivation is the
//!   sound v1 (the documented-divergence posture).
//! - The cast target must be a single-segment path type with no arguments
//!   (`usize`, `u8`, `f64`, `char`, ...). References, pointers (`*const` /
//!   `*mut` — raw-pointer territory is OUT), multi-segment paths, and
//!   generics all refuse.
//! - The **literal sub-arm** accepts only int/float targets (the suffix form
//!   exists for those alone): `char` / `bool` targets refuse (no suffix
//!   spelling exists), a direct `Expr::Lit` operand only (unary-minus and
//!   parenthesized literals refuse — v1 keeps the emit to the canonical
//!   suffix form), and an EXISTING suffix (either spelling, `5u64` or
//!   `5_u64`) is stripped before the canonical `5_u64` emit — clippy's own
//!   suggestion for `5u64 as u64` is `5_u64`, verified in the probe.
//!   **P42 soundness guards** (the rewrite must hold standalone — the fix
//!   lane can fire without an oracle span): a float token takes only a
//!   float target; int→float requires a DECIMAL token (in hex, `f` is a
//!   digit — `0x8000_f32` lexes as the integer `0x8000f32`); int→int
//!   requires the value to FIT the target (`300_u8` is E0301). The probe
//!   also caught TWO upstream 1.98 broken suggestions this guard set
//!   refuses to mirror: the out-of-range `300_u8` on `300 as u8` and the
//!   prefix-dropping `FF_u64` on `0xFF_u64 as u64` (ours re-emits
//!   `0xFF_u64`).
//! - Cross-type casts REFUSE by construction (the proven map stores the
//!   exact type text; `x as u64` with `x: u32` is not a match). The probe
//!   showed those fire `cast_lossless` / `needless_type_cast` /
//!   `cast_possible_wrap` — DIFFERENT lints, different seams, never ours.
//! - Distinctness: `cast_lossless` rewrites a REMOVABLE-but-real widening
//!   (`u8 as u64` → `u64::from(x)`); this rule only drops casts that carry
//!   no conversion at all. Comments inside the cast span refuse.

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprCast, File, ImplItemFn, ItemFn, Lit};

use super::helpers::{byte_range, drops_interior_comment, scalar_param_heads_over};
use super::{SourceEdit, edit};

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
        if let Some(replacement) = self.try_rewrite(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, c: &ExprCast) -> Option<String> {
        // The target must be a plain single-segment path type (no args): the
        // scalar spelling the proven map stores. References / pointers /
        // multi-segment / generic targets refuse.
        let syn::Type::Path(tp) = &*c.ty else {
            return None;
        };
        if tp.qself.is_some() || tp.path.leading_colon.is_some() {
            return None;
        }
        if tp.path.segments.len() != 1 || !tp.path.segments[0].arguments.is_none() {
            return None;
        }
        let target = tp.path.segments[0].ident.to_string();

        if let Some(replacement) = self.rewrite_proven_ident(c, &target) {
            return Some(replacement);
        }
        self.rewrite_typed_literal(c, &target)
    }

    /// Arm (a): `x as T` where `x` is a proven bare ident typed exactly `T`
    /// — the identity cast drops.
    fn rewrite_proven_ident(&self, c: &ExprCast, target: &str) -> Option<String> {
        let Expr::Path(p) = &*c.expr else { return None };
        if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
            return None;
        }
        if !p.path.segments[0].arguments.is_none() {
            return None;
        }
        let ident = p.path.segments[0].ident.to_string();
        if self.proven.get(ident.as_str())? != target {
            return None;
        }
        if drops_interior_comment(self.source, byte_range(c), &[byte_range(&c.expr)]) {
            return None;
        }
        Some(ident)
    }

    /// Arm (b): `5 as u64` → `5_u64` — the unsized literal carries the type
    /// itself. Int/float targets only; a direct `Expr::Lit` operand only.
    ///
    /// P42 guards (the literal rewrite must be SOUND STANDALONE — the fix
    /// lane can fire without an oracle span, which is exactly how the hex
    /// breakage shipped, katgpt-rs recovery `877d477cc`):
    /// - **kind**: a float-token operand takes only a float target (`2.0 as
    ///   u64` → `2.0_u64` is not a literal at all; upstream 1.98 is SILENT
    ///   there — probe-verified).
    /// - **base**: int→float requires a DECIMAL token — float literals are
    ///   decimal-only, and in a hex literal `f` is a DIGIT, so `0x8000_f32`
    ///   lexes as the hex INTEGER `0x8000f32` (the P42 breakage; upstream is
    ///   silent on non-decimal int→float — parity).
    /// - **value fit**: int→int requires the value to FIT the target —
    ///   `300 as u8` → `300_u8` is E0301. Upstream 1.98 FIRES here with that
    ///   broken suggestion (probe-verified; likewise its suffixed-hex
    ///   suggestion `FF_u64` drops the `0x`) — the bounded fix-space is
    ///   narrower than clippy by design, and our strip-then-reemit keeps the
    ///   prefix upstream loses.
    fn rewrite_typed_literal(&self, c: &ExprCast, target: &str) -> Option<String> {
        if !SUFFIX_TARGETS.contains(&target) {
            return None;
        }
        let Expr::Lit(l) = &*c.expr else { return None };
        let digits = match &l.lit {
            Lit::Int(li) => li.token().to_string(),
            Lit::Float(fl) => fl.token().to_string(),
            _ => return None,
        };
        let is_float_lit = matches!(&l.lit, Lit::Float(_));
        let is_float_target = target == "f32" || target == "f64";
        if is_float_lit && !is_float_target {
            // `2.0_u64` is not a literal spelling — refuse.
            return None;
        }
        if is_float_target && !is_float_lit && !is_decimal_int_token(&digits) {
            // Non-decimal int → float: no float spelling exists for the
            // token (float literals are decimal-only).
            return None;
        }
        // Strip an existing integer/float suffix (either spelling) and emit
        // the canonical `digits_target` form — the probe-verified upstream
        // suggestion for `5u64 as u64` is `5_u64`.
        let bare = strip_numeric_suffix(&digits);
        if !is_float_target {
            let value = parse_int_value(&bare)?;
            if !fits_int_target(value, target) {
                return None;
            }
        }
        if drops_interior_comment(self.source, byte_range(c), &[byte_range(&c.expr)]) {
            return None;
        }
        Some(format!("{bare}_{target}"))
    }
}

/// The int/float spellings the literal sub-arm accepts — exactly the types
/// that HAVE a literal suffix form (`char`/`bool` have none and refuse).
const SUFFIX_TARGETS: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64",
];

/// True when the numeric token is DECIMAL — no `0x`/`0o`/`0b` radix prefix
/// (P42's base guard: float literals are decimal-only).
fn is_decimal_int_token(tok: &str) -> bool {
    !(tok.starts_with("0x")
        || tok.starts_with("0X")
        || tok.starts_with("0o")
        || tok.starts_with("0O")
        || tok.starts_with("0b")
        || tok.starts_with("0B"))
}

/// Parse an unsuffixed integer token's VALUE (radix prefixes + `_`
/// separators handled; the token is non-negative — unary-minus operands are
/// refused upstream of this call by the direct-`Expr::Lit` guard).
fn parse_int_value(bare: &str) -> Option<u128> {
    let (digits, radix) =
        if let Some(rest) = bare.strip_prefix("0x").or_else(|| bare.strip_prefix("0X")) {
            (rest, 16)
        } else if let Some(rest) = bare.strip_prefix("0o").or_else(|| bare.strip_prefix("0O")) {
            (rest, 8)
        } else if let Some(rest) = bare.strip_prefix("0b").or_else(|| bare.strip_prefix("0B")) {
            (rest, 2)
        } else {
            (bare, 10)
        };
    let cleaned: String = digits.chars().filter(|&c| c != '_').collect();
    if cleaned.is_empty() {
        return None;
    }
    u128::from_str_radix(&cleaned, radix).ok()
}

/// Does the non-negative value FIT the int target's range (`300_u8` is
/// E0301 — the P42 value-fit guard)? `usize`/`isize` use the 64-bit bound
/// (the conservative platform-independent direction: refuse more).
fn fits_int_target(value: u128, target: &str) -> bool {
    match target {
        "u8" => value <= u128::from(u8::MAX),
        "u16" => value <= u128::from(u16::MAX),
        "u32" => value <= u128::from(u32::MAX),
        "u64" | "usize" => value <= u128::from(u64::MAX),
        "u128" => true,
        "i8" => value <= i8::MAX as u128,
        "i16" => value <= i16::MAX as u128,
        "i32" => value <= i32::MAX as u128,
        "i64" | "isize" => value <= i64::MAX as u128,
        "i128" => value <= i128::MAX as u128,
        _ => false,
    }
}

/// Split a numeric literal token into its digits, stripping a KNOWN suffix
/// (`5u64`, `5_u64`, `2.0f32`) when present. An unsuffixed token returns the
/// whole text; a token whose tail is not a known suffix (e.g. a hex literal
/// `0xFF` — no `FF` suffix exists) returns it unchanged.
fn strip_numeric_suffix(tok: &str) -> String {
    const SUFFIXES: &[&str] = &[
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
        "f32", "f64",
    ];
    for s in SUFFIXES {
        // The underscore spelling (`5_u64`) and the tight spelling (`5u64`)
        // both strip; the boundary check keeps `0xFF` intact (no suffix
        // starts with `F`).
        for spelling in [format!("_{s}"), s.to_string()] {
            if let Some(digits) = tok.strip_suffix(spelling.as_str()) {
                if !digits.is_empty() {
                    return digits.to_string();
                }
            }
        }
    }
    tok.to_string()
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>, p: u32) -> u32 { hits.push(0); p }",
            )
            .is_none()
        );
    }

    #[test]
    fn same_type_param_cast_drops() {
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>, p: u32) -> u32 { hits.push(0); p as u32 }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); p }"), "cast not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn float_same_type_drops() {
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>, a: f64) -> f64 { hits.push(0); a as f64 }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(0); a }"), "cast not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn int_literal_gains_suffix() {
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 5 as u64 }",
        )
        .expect("matched");
        assert!(out.contains("5_u64"), "suffix not substituted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn existing_suffix_strips_to_canonical() {
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 5u64 as u64 }",
        )
        .expect("matched");
        assert!(out.contains("5_u64"), "suffix not canonicalized: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn cross_type_refuses() {
        // `x as u64` with x: u32 is a REAL conversion — cast_lossless's
        // territory, never ours.
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>, x: u32) -> u64 { hits.push(0); x as u64 }",
            )
            .is_none()
        );
    }

    #[test]
    fn pointer_target_refuses() {
        // Raw-pointer target spelling (`*const T`) is not a single-segment
        // path type — out of the fix-space by construction. `usize` is a
        // DIFFERENT type from `u32`, so this shape refuses on the
        // provenance match too (a genuine conversion).
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>, p: u32) -> usize { hits.push(0); p as usize }",
            )
            .is_none()
        );
    }

    #[test]
    fn unproven_operand_refuses() {
        // The operand is a call result, not a proven param — refuse.
        assert!(propose_fix(
            "unnecessary_cast",
            "fn mk() -> u32 { 3 } fn f(hits: &mut Vec<u8>) -> u32 { hits.push(0); mk() as u32 }",
        )
        .is_none());
    }

    #[test]
    fn let_bound_operand_refuses_v1() {
        // Annotated-LET provenance is the documented v1 divergence (params
        // only) — upstream fires here; we refuse until the let-collector
        // rider.
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>) -> usize { hits.push(0); let x: usize = 7; x as usize }",
            )
            .is_none()
        );
    }

    #[test]
    fn char_target_literal_refuses() {
        // `char` has no literal suffix form — the literal sub-arm refuses.
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>) -> char { hits.push(0); 'a' as char }",
            )
            .is_none()
        );
    }

    #[test]
    fn hex_int_to_float_refuses_p42() {
        // THE P42 regression: in a hex literal `f` is a digit, so
        // `0x8000_f32` lexes as the hex INTEGER `0x8000f32` (E0277 — the
        // katgpt-rs `877d477cc` breakage). No float spelling exists for a
        // non-decimal token; upstream 1.98 is silent here too.
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 0x8000 as f32 }",
            )
            .is_none()
        );
    }

    #[test]
    fn oct_int_to_float_refuses_p42() {
        // Same base-guard class for octal (upstream silent — parity).
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>) -> f64 { hits.push(0); 0o777 as f64 }",
            )
            .is_none()
        );
    }

    #[test]
    fn dec_int_to_float_gains_suffix_p42() {
        // The decimal int→float arm SURVIVES the P42 guards — upstream fires
        // here and suggests exactly this form (probe-verified), and the
        // emitted spelling compiles.
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 32768 as f32 }",
        )
        .expect("matched");
        assert!(out.contains("32768_f32"), "suffix not substituted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn float_lit_to_int_refuses_p42() {
        // `2.0_u64` is not a literal spelling (kind guard); upstream 1.98 is
        // silent on the shape — parity.
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 2.0 as u64 }",
            )
            .is_none()
        );
    }

    #[test]
    fn overflow_int_to_int_refuses_p42() {
        // `300_u8` is E0301 (literal out of range) — the value-fit guard.
        // Upstream 1.98 FIRES here with that broken suggestion (probe); the
        // fix-space is narrower than clippy by design.
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>) -> u8 { hits.push(0); 300 as u8 }",
            )
            .is_none()
        );
    }

    #[test]
    fn hex_int_to_int_gains_suffix_p42() {
        // Hex → INT stays sound (suffix on hex is valid: `u`/`i` are not hex
        // digits) — and our re-emit keeps the `0x` prefix that upstream's own
        // suffixed-hex suggestion drops (its `FF_u64` is E0425, probe).
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 0x8000 as u64 }",
        )
        .expect("matched");
        assert!(
            out.contains("0x8000_u64"),
            "prefix lost or no suffix: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn existing_hex_suffix_canonicalizes_keeps_prefix_p42() {
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 0xFF_u64 as u64 }",
        )
        .expect("matched");
        assert!(out.contains("0xFF_u64"), "canonical form wrong: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(
            propose_fix(
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>, p: u32) -> u32 { hits.push(0); p /* why */ as u32 }",
            )
            .is_none()
        );
    }

    #[test]
    fn impl_method_param_proven() {
        let out = propose_fix(
            "unnecessary_cast",
            "struct S; impl S { fn f(&self, p: u64, hits: &mut Vec<u8>) -> u64 { hits.push(0); p as u64 } }",
        )
        .expect("matched");
        assert!(out.contains("p } }"), "cast not dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_condition_fires() {
        let out = propose_fix(
            "unnecessary_cast",
            "fn f(hits: &mut Vec<u8>, p: usize) -> usize { hits.push(0); if p as usize > 1 { 1 } else { 0 } }",
        )
        .expect("matched");
        assert!(out.contains("if p > 1"), "condition not simplified: {out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
