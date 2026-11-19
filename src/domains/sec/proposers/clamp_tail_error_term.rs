//! `clamp-emits-tail-error-term` — SUGGEST-ONLY detector
//! (Batch 370, the mlx-learn certified-KNN + compensated-numerics
//! transcription; Research 415 / Plan 383).
//!
//! An approximation that deliberately clamps its input domain must emit a
//! MEASURABLE ERROR TERM for the clamped mass, and the caller must fold the
//! emitted terms — under a conservative multiplier for the summation's own
//! rounding — into a REPORTED bound. The source's shape (mlx-learn
//! `_compensated.py`): `exp2x` clamps at −80, the kernel emits per-row
//! `tail_error[row] = (abs(u.x)>80.0f) ? weight * feature_max *
//! 1.8048524e-35f : 0.0f` (1.8048524e-35 ≈ e^−80, the clamp-point residual),
//! and the caller reports `tail_bound = 2.0 * mx.sum(tail_errors)` with the
//! in-source comment "Factor 2 accounts conservatively for float32 rounding
//! in this particular tail sum. This is not a bound on all other numerical
//! errors." Consumers treat the bound as "a numerical check, not a proof"
//! (the module docstring) — it covers only the clamped tail, never all
//! arithmetic error.
//!
//! Why suggest-only: which sites NEED a tail term is a numerics-design
//! decision (the clamp may be a deliberate saturation the caller prices
//! elsewhere); parse-only cannot see the caller's bound bookkeeping. The
//! detector FLAGS the shape for review, never rewrites.
//!
//! Detector vocabulary discipline (IDENT + body-shape, never string
//! literals beyond the math constants): the detector looks for clamped
//! approximation bodies (a comparison against a large-negative constant
//! selecting a zero return) that carry NO tail/error-term emission in the
//! same fn, and names the clamp-point constant when it matches e^-80-class
//! residuals.
//!
//! Detector: `proposers::clamp_tail_error_term`. Fixture:
//! `fixtures/b370_fixtures.rs` F1. Batch 370 / Plan 383 / Research 415
//! (tabulai/mlx-learn @ 07054ce3, Apache-2.0; the fixture is an original
//! synthesis, standalone-compiled AND executed before entry).

use std::ops::Range;

use quote::ToTokens;
use syn::File;
use syn::spanned::Spanned;

/// One detected unbounded-clamp site.
#[derive(Debug, Clone)]
pub struct ClampTailFinding {
    /// Byte range of the fn body.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the emitted tail term + the conservative fold).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a deliberately clamped approximation emits a \
measurable error term for the clamped mass (per-row tail_error over the \
clamp-point residual, e.g. weight * e^-80 at a -80 clamp) and the caller \
sums the emitted terms under a conservative multiplier for the summation's \
own rounding (the source's factor 2) into a REPORTED bound — a check, \
never a claimed proof of all arithmetic error (Batch 370, suggest-only)";

/// Clamp-point constants of the e^-80 class (f32 spellings of the residual
/// at the clamp).
const CLAMP_POINT_CONSTS: &[&str] = &[
    "1.8048524e-35",
    "1.80485e-35",
    "1.8049e-35",
    "1.8048524e-35f32",
];

/// Detection vocabulary (fn-name/body-shape gate, never string literals
/// beyond the math constants above).
const APPROX_TOKENS: &[&str] = &["exp", "approx", "sigmoid", "soft", "tail"];

/// Detect clamped approximation fns with no tail/error emission
/// (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<ClampTailFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        let fn_items = match item {
            syn::Item::Fn(f) => vec![f],
            syn::Item::Mod(m) => {
                // one level of inline module (the corpus convention)
                let mut out = Vec::new();
                if let Some(content) = &m.content {
                    for nested in &content.1 {
                        if let syn::Item::Fn(f) = nested {
                            out.push(f);
                        }
                    }
                }
                out
            }
            _ => Vec::new(),
        };
        for f in fn_items {
            visit_fn(source, f, &mut findings);
        }
    }
    findings
}

fn visit_fn(source: &str, f: &syn::ItemFn, findings: &mut Vec<ClampTailFinding>) {
    let name = f.sig.ident.to_string();
    let lower = name.to_ascii_lowercase();
    // the class-shape gate: an approximation-ish fn name...
    let name_is_approx = APPROX_TOKENS.iter().any(|t| lower.contains(t));
    if !name_is_approx {
        return;
    }
    let body_text = f.block.to_token_stream().to_string();
    // ...whose body clamps a large-negative threshold to a zero return...
    let clamps_to_zero = body_text.contains("< -")
        && (body_text.contains("return float2(0")
            || body_text.contains("return 0.0")
            || body_text.contains("0.0f,0.0f")
            || (body_text.contains("if") && body_text.contains("-80")));
    if !clamps_to_zero {
        return;
    }
    // ...and emits NO tail/error term anywhere in the fn.
    let emits_tail = body_text.contains("tail_error")
        || body_text.contains("tail_bound")
        || body_text.contains("error_term")
        || CLAMP_POINT_CONSTS.iter().any(|c| body_text.contains(c));
    if emits_tail {
        return;
    }
    let span = f.span();
    findings.push(ClampTailFinding {
        site_span: span.byte_range(),
        site_text: source[span.byte_range()].to_string(),
        advice: ADVICE,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_on_unbounded_clamp() {
        let src = r"
fn cheap_exp_approx(z: f32) -> f32 {
    if z < -80.0 {
        return 0.0;
    }
    z.exp()
}
";
        let file: File = syn::parse_str(src).unwrap();
        let found = detect(src, &file);
        assert_eq!(found.len(), 1, "the unbounded clamp must fire");
    }

    #[test]
    fn silent_on_tail_emitting_clamp() {
        let src = r"
fn exp_approx_tail(z: f32, w: f32) -> (f32, f32) {
    if z < -80.0 {
        return (0.0, w * 1.8048524e-35);
    }
    (z.exp(), 0.0)
}
";
        let file: File = syn::parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "tail emission suppresses");
    }

    #[test]
    fn silent_on_unrelated_fns() {
        let src = r"
fn clamp01_saturate(x: f32) -> f32 {
    if x < 0.0 {
        return 0.0;
    }
    x
}
";
        let file: File = syn::parse_str(src).unwrap();
        assert!(detect(src, &file).is_empty(), "no large-negative clamp");
    }
}
