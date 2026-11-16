//! `gelu-tanh-cancellation-sigmoid-form` — SUGGEST-ONLY detector
//! (Batch 366, the rounding-bugs numerics transcription; Research 412 /
//! Plan 380).
//!
//! The tanh-spelled GELU erases its deep-negative tail: `0.5 * x *
//! (1.0 + z.tanh())` rounds `1.0 + tanh z` to +0.0 once
//! `2·e^(−2|z|) < half-ulp-below-1` (2.98e−8 f32, |z| ≳ 9), so at
//! x = −6 every tested spelling returns −0.0 instead of ≈ −8.4e−11
//! (George & Rush, live-fetch 2026-11-16). The fix is the ALGEBRAIC
//! rearrangement `x·σ(2z)` — the same tanh approximation, NOT the erf
//! switch.
//!
//! Why suggest-only: the rewrite is value-preserving over the reals
//! but the regime (which inputs sit in the band) is per-model, and a
//! fused-kernel backend may already compute the tail in a wider lane.
//! NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! Detector: `proposers::gelu_tanh_sigmoid_form`. Fixture:
//! `fixtures/b366_fixtures.rs` F1. Batch 366 / Plan 380 / Research 412.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected tanh-spelled GELU tail-erasure site.
#[derive(Debug, Clone)]
pub struct GeluTanhFinding {
    /// Byte range of the fn body.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the sigmoid rearrangement).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "the tanh-spelled GELU erases its deep-negative tail: \
1.0 + tanh(z) rounds to +0.0 when 2*e^(-2|z|) < half-ulp-below-1 (2.98e-8 f32, \
|z| >~ 9 — at x=-6 f32 every tested spelling returns -0.0 instead of ~-8.4e-11). \
Rearrange ALGEBRAICALLY to x * sigmoid(2*z) — the same tanh approximation, \
never the erf switch (Batch 366, suggest-only)";

/// The activation vocabulary (the class-shape gate).
const ACTIVATION_TOKENS: &[&str] = &["gelu", "activation", "act_"];

/// Suppression vocabulary: the sigmoid rearrangement (or the erf form)
/// already applied.
const SUPPRESS_TOKENS: &[&str] = &["sigmoid(", "exp(", "erf"];

/// Detect tanh-spelled GELU bodies (suggest-only; no edits are produced
/// by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<GeluTanhFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<GeluTanhFinding>) {
    match item {
        Item::Fn(f) => scan_block(source, &f.sig.ident.to_string(), &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_block(source, &m.sig.ident.to_string(), &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production sites.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_block(source: &str, ident: &str, block: &Block, findings: &mut Vec<GeluTanhFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    // The vocabulary gate reads IDENT + BODY — the activation name
    // conventionally lives in the fn name (`gelu_activation`).
    let mut ctx = ident.to_string();
    ctx.push_str(&compact);
    let lower = ctx.to_lowercase();

    // Class-shape gate: the activation vocabulary in the body.
    if !ACTIVATION_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // Suppression: the sigmoid/erf rearrangement is already applied.
    if SUPPRESS_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // The bug shape: a tanh call feeding a `1.0 + <tanh>` tail under a
    // 0.5 scale.
    if !tanh_one_plus_half(block) {
        return;
    }

    let span = block.span().byte_range();
    findings.push(GeluTanhFinding {
        site_span: span.clone(),
        site_text: source.get(span).unwrap_or("").to_string(),
        advice: ADVICE,
    });
}

/// The tail-erasure shape: `.tanh(...)` called on something, a `1.0 +
/// <tanh result>` addition, and a `0.5 *` scale in the same body.
fn tanh_one_plus_half(block: &Block) -> bool {
    let mut walk = TanhTailScan {
        tanh_call: false,
        one_plus: false,
        half_scale: false,
    };
    walk.visit_block(block);
    walk.tanh_call && walk.one_plus && walk.half_scale
}

struct TanhTailScan {
    tanh_call: bool,
    one_plus: bool,
    half_scale: bool,
}

impl<'ast> syn::visit::Visit<'ast> for TanhTailScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        match e {
            Expr::MethodCall(m) if m.method == "tanh" => {
                self.tanh_call = true;
            }
            Expr::Binary(b) => match b.op {
                syn::BinOp::Add(_) if is_lit_one(&b.left) || is_lit_one(&b.right) => {
                    // `1.0 + x` where the other side is (or wraps) a
                    // tanh call — the tail shape.
                    let other = if is_lit_one(&b.left) {
                        &b.right
                    } else {
                        &b.left
                    };
                    let ts = other.to_token_stream().to_string();
                    if ts.contains("tanh") {
                        self.one_plus = true;
                    }
                }
                syn::BinOp::Mul(_) if is_lit_half(&b.left) || is_lit_half(&b.right) => {
                    self.half_scale = true;
                }
                _ => {}
            },
            _ => {}
        }
        syn::visit::visit_expr(self, e);
    }
}

fn is_lit_one(e: &Expr) -> bool {
    matches!(lit_f32_value(e), Some(v) if (v - 1.0).abs() < f32::EPSILON)
}

fn is_lit_half(e: &Expr) -> bool {
    matches!(lit_f32_value(e), Some(v) if (v - 0.5).abs() < f32::EPSILON)
}

/// The float value of a literal expression, when it is one.
fn lit_f32_value(e: &Expr) -> Option<f32> {
    match e {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Float(f) => f.base10_parse::<f32>().ok(),
            syn::Lit::Int(i) => i.base10_parse::<f32>().ok(),
            _ => None,
        },
        Expr::Unary(u) => {
            let Expr::Lit(l) = &*u.expr else {
                return None;
            };
            match &l.lit {
                syn::Lit::Float(f) => f.base10_parse::<f32>().ok().map(|v| -v),
                syn::Lit::Int(i) => i.base10_parse::<f32>().ok().map(|v| -v),
                _ => None,
            }
        }
        _ => None,
    }
}
