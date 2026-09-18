//! `clippy::excessive_precision` — `2.141592653589793238462643f32` →
//! `2.1415927f32` (Batch 164 / Plan 155 — the frontier-lead mining pack X;
//! the family fixture verified to fire exactly this lint under
//! all+pedantic+nursery with a clean seed, 2026-09-19
//! (`scripts/plan155_frontier_probe.py`); style/default-on, group pinned by
//! the probe's no-flags arm; default-eco count 56 — rank 2 of the whole
//! evidence head).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! The literal's SUFFIX names its type, so the width is known without any
//! type check (the suffix-only scope is the documented narrower-than-clippy
//! divergence — `ep-nosuffix` proved upstream also fires on
//! annotation-typed literals, which are type-blind here). The replacement is
//! the SHORTEST decimal that round-trips to the same value:
//! `format!("{:?}", v)` is std's shortest-round-trip formatting, and the
//! proposer re-parses the shortened text and compares BITS with the parsed
//! original — the edit lands only when the two literals are the identical
//! f32/f64 value. Same suffix + same value ⇒ inference, const-eval, and
//! every consumer see exactly what they saw before.
//!
//! ## Guards (the bounded fix-space — probe-pinned against clippy 0.1.98)
//!
//! - Explicit `f32`/`f64` suffix REQUIRED (upstream also fires on
//!   `let a: f32 = 2.1415…;` — we refuse: the suffix is the only type-blind
//!   width proof; the `manual_string_new` divergence precedent).
//! - Digit separators refuse (`1.000_000_1f32` — upstream is SILENT there,
//!   `unreadable_literal`'s territory; our refusal is upstream-exact).
//! - Integral values refuse (`1.0000000f32`, `2f32` — upstream is SILENT
//!   there; the shortest form would only strip zeros, nothing excessive).
//!   The check is on the VALUE (`fract() == 0`), not the text shape —
//!   `3.000…e-10f32` has all-zero mantissa fraction digits yet FIRES
//!   upstream (probe-pinned) because the value is not integral.
//! - The shortest form must have STRICTLY fewer mantissa digits than the
//!   original (already-shortest literals like `2.1415927f32` and `0.1f32`
//!   refuse — upstream silent on `ep-clean`).
//! - Exponent-presence must AGREE between original and shortest
//!   (`3.000…e-10f32` → `3e-10f32` fires — probed; a decimal original whose
//!   shortest form is exponential, e.g. `0.00001f32` → `1e-5`, refuses —
//!   unprobed shape, conservative scope).
//! - The bit-identity re-parse is the soundness leg: any shortest text that
//!   does not parse back to the original's exact bits refuses (should never
//!   fire — `{:?}` round-trips by contract — but the edit never lands on
//!   faith).
//! - Sign never enters the edit: `-2.1000000f32` is `Unary(Neg, lit)` —
//!   the minus stays in the source, the lit span re-spells alone.

use syn::visit::Visit;
use syn::{ExprLit, File, Lit};

use super::helpers::byte_range;
use super::{edit, SourceEdit};

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

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_lit(&mut self, l: &'ast ExprLit) {
        syn::visit::visit_expr_lit(self, l);
        if let Some(replacement) = self.try_rewrite(l) {
            self.edits.push(edit(l, replacement));
        }
    }
}

/// A parsed float-literal body: everything before the suffix.
struct LiteralBody<'a> {
    digits: &'a str,
    suffix: &'static str,
}

impl Collector<'_> {
    fn try_rewrite(&self, l: &ExprLit) -> Option<String> {
        let Lit::Float(_) = &l.lit else { return None };
        let text = self.source.get(byte_range(l))?;
        let body = split_suffix(text)?;
        let shortest = shortest_round_trip(body.digits, body.suffix)?;
        Some(format!("{shortest}{}", body.suffix))
    }
}

fn split_suffix(text: &str) -> Option<LiteralBody<'_>> {
    let (digits, suffix) = match text.strip_suffix("f32") {
        Some(d) => (d, "f32"),
        // `None` on the f64 strip = no explicit suffix — the width is a
        // type-check question we cannot answer (the documented
        // narrower-than-clippy divergence); the `?` refuses it.
        None => (text.strip_suffix("f64")?, "f64"),
    };
    if digits.contains('_') {
        // Digit separators — unreadable_literal's territory; upstream is
        // silent here too (probe-pinned).
        return None;
    }
    Some(LiteralBody { digits, suffix })
}

/// The shortest-round-trip search. `None` = refuse (not the family, or the
/// literal is already shortest).
fn shortest_round_trip(digits: &str, suffix: &str) -> Option<String> {
    let parts = split_mantissa_exponent(digits)?;
    let orig_digits = digits_of(parts.mantissa_int, parts.frac);
    let (shortest, integral) = if suffix == "f32" {
        digits
            .parse::<f32>()
            .ok()
            .map(|v| (format!("{v:?}"), v.fract() == 0.0))
    } else {
        digits
            .parse::<f64>()
            .ok()
            .map(|v| (format!("{v:?}"), v.fract() == 0.0))
    }?;
    if integral {
        // `1.0000000f32` / `2f32` — the value is integral; the shortest
        // form would only strip zeros, nothing excessive (upstream silent
        // on the shape, probe-pinned).
        return None;
    }
    // Exponent-presence must agree (conservative scope: every fired shape
    // stays inside the probed territory).
    if shortest.contains('e') != digits.contains(['e', 'E']) {
        return None;
    }
    let s_parts = split_mantissa_exponent(&shortest)?;
    if digits_of(s_parts.mantissa_int, s_parts.frac).len() >= orig_digits.len() {
        // Already shortest (0.1f32, 2.1415927f32) — refuse.
        return None;
    }
    // The soundness leg: the shortened text must parse back to the exact
    // same bits. ({:?} guarantees this; the edit never lands on faith.)
    let bits_match = if suffix == "f32" {
        digits
            .parse::<f32>()
            .ok()
            .zip(shortest.parse::<f32>().ok())
            .is_some_and(|(a, b)| a.to_bits() == b.to_bits())
    } else {
        digits
            .parse::<f64>()
            .ok()
            .zip(shortest.parse::<f64>().ok())
            .is_some_and(|(a, b)| a.to_bits() == b.to_bits())
    };
    if !bits_match {
        return None;
    }
    Some(shortest)
}

struct MantissaParts<'a> {
    mantissa_int: &'a str,
    frac: &'a str,
}

/// Split a float-literal body into (mantissa-int, frac) parts, validating
/// the decimal shape. The exponent part (if any) is validated but its
/// digits never count toward the mantissa comparison.
fn split_mantissa_exponent(digits: &str) -> Option<MantissaParts<'_>> {
    let (mantissa, exp) = match digits.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (digits, None),
    };
    if let Some(e) = exp {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        if e.is_empty() || !e.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    let (int_part, frac) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };
    let digits_ok = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !digits_ok(int_part) || !digits_ok(frac) {
        return None;
    }
    if int_part.is_empty() && frac.is_empty() {
        return None;
    }
    // Empty frac (`7f32`, shortest `3e-10`) is a valid shape here — the
    // integrality guard, not the shape, decides the original.
    Some(MantissaParts { mantissa_int: int_part, frac })
}

/// Digit characters of a mantissa, in source order (leading zeros included
/// — the comparison only needs a stable count).
fn digits_of(int_part: &str, frac: &str) -> String {
    format!("{int_part}{frac}")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 0.1f32 }",
        )
        .is_none());
    }

    #[test]
    fn f32_shortens_to_shortest_round_trip() {
        let out = propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 2.141592653589793238462643f32 }",
        )
        .expect("matched");
        assert!(out.contains("2.1415927f32"), "not shortened: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn f64_shortens_to_shortest_round_trip() {
        let out = propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f64 { hits.push(0); 2.141592653589793238462643f64 }",
        )
        .expect("matched");
        assert!(out.contains("2.141592653589793f64"), "not shortened: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn trailing_zeros_shorten() {
        let out = propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 1.100000f32 }",
        )
        .expect("matched");
        assert!(out.contains("1.1f32"), "not shortened: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn all_zero_fraction_refuses() {
        // Upstream is SILENT here (probe-pinned: only unreadable_literal).
        assert!(propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 1.0000000f32 }",
        )
        .is_none());
    }

    #[test]
    fn integer_floated_refuses() {
        assert!(propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 7f32 }",
        )
        .is_none());
    }

    #[test]
    fn no_suffix_refuses() {
        // Upstream FIRES here (probe-pinned); the suffix is our only
        // type-blind width proof — the documented divergence.
        assert!(propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); let a: f32 = 2.141592653589793238462643; a }",
        )
        .is_none());
    }

    #[test]
    fn digit_separator_refuses() {
        // Upstream is SILENT here (unreadable_literal's territory).
        assert!(propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 1.000_000_1f32 }",
        )
        .is_none());
    }

    #[test]
    fn exponent_form_shortens_in_place() {
        let out = propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 3.00000000000e-10f32 }",
        )
        .expect("matched");
        assert!(out.contains("3e-10f32"), "not shortened: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn decimal_to_exponent_converts_refuse() {
        // The shortest form of 1e-5f32 is exponential; a decimal original
        // whose shortest form is exponential is UNPROBED — conservative
        // refusal.
        assert!(propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 0.000010000f32 }",
        )
        .is_none());
    }

    #[test]
    fn already_shortest_refuses() {
        assert!(propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 2.1415927f32 }",
        )
        .is_none());
    }

    #[test]
    fn const_context_fires() {
        let out = propose_fix(
            "excessive_precision",
            "const A: f32 = 2.14159265358979f32;\nfn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); A }",
        )
        .expect("matched");
        assert!(out.contains("2.1415927f32"), "not shortened: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negative_literal_keeps_sign() {
        let out = propose_fix(
            "excessive_precision",
            "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); -2.141592653589793238462643f32 }",
        )
        .expect("matched");
        assert!(out.contains("-2.1415927f32"), "sign lost: {out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
