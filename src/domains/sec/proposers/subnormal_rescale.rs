//! `fp16-subnormal-product-power-of-two-rescale` — SUGGEST-ONLY detector
//! (Batch 352, the `LoNE` distill; Research 395 / Plan 362).
//!
//! An fp16/NE-class linear core loses precision when the input×weight
//! products fall into fp16's subnormal range — `LoNE`'s measured case
//! (AJ-Gonzalez/LoNE @ `4806171d`, Apache-2.0, verbatim at the pin):
//! "operator cosine 0.99916 -> 0.9999998 on real activations." The gate
//! is width: the source scales ONLY reductions wider than 1024
//! (`_WIDE = 1024`; `scaled_weight` scales when `w.shape[1] > _WIDE`).
//!
//! The Rust decision content: at a weights-prepare or wide-gate site for
//! an f16 reduction, when no power-of-two scaling is applied, suggest
//! the ×2^k weights / ×2^-k output pair (2^4 in the source).
//!
//! Why suggest-only: whether the host backend exhibits the subnormal
//! loss is per-device/per-generation, and k is a per-model calibration.
//! NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! Honest recall limits (stated in-rule): weights-prep vocabulary is
//! fn-scope; a deliberate unscaled site (f32 accumulator, a verified
//! subnormal-free backend) fires and review decides.
//!
//! Detector: `proposers::subnormal_rescale`. Fixture:
//! `fixtures/b352_fixtures.rs` F1. Batch 352 / Plan 362 / Research 395.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected unscaled wide f16 reduction weights site.
#[derive(Debug, Clone)]
pub struct SubnormalFinding {
    /// Byte range of the enclosing fn's wide-gate or accumulation site.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the ×2^k / ×2^-k pair).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "an fp16 reduction whose per-term products fall in \
the subnormal band loses accumulated precision (measured: operator cosine \
0.99916 -> 0.9999998 on real activations). Scale the weights by a power of \
two (2^4) at prepare time and divide the output by the same factor — exact \
in fp16 absent overflow, so the rewrite moves the accumulation into the \
normal range without changing any representable value. Gate on the wide \
reductions (k > 1024 in the source) (Batch 352, suggest-only)";

/// The weights-prep vocabulary (one of the two site arms).
const WEIGHT_TOKENS: &[&str] = &["weights", "weight_plane", "w_quant", "dequant"];

/// The reduction compute vocabulary (dot / matvec / matmul / GEMV).
const REDUCE_TOKENS: &[&str] = &["dot", "matvec", "matmul", "gemv", "gemm", "reduce", "proj"];

/// f16 arithmetic vocabulary — the regime gate (NE-class fp16, not f32).
const F16_TOKENS: &[&str] = &["f16", "F16", "half", "Half"];

/// The power-of-two rescale vocabulary whose PRESENCE suppresses
/// (the fix already applied) — matched against the LOWERCASED body.
const SCALE_TOKENS: &[&str] = &["scale", "rescale", "powf", "powi"];

/// The wide-threshold literal the source pins: reductions wider than
/// this are the regime (`_WIDE = 1024`).
const WIDE_THRESHOLD: u64 = 1024;

/// Detect unscaled wide f16 reduction sites (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<SubnormalFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<SubnormalFinding>) {
    match item {
        Item::Fn(f) => scan_block(source, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_block(source, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production weights-prep sites.
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

fn scan_block(source: &str, block: &Block, findings: &mut Vec<SubnormalFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    // Token streams render with spaces — compact before the vocabulary
    // checks; a lowercased copy for the case-insensitive suppression
    // vocabulary.
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    let lower = compact.to_lowercase();

    // Class-shape gate: f16 arithmetic + reduction vocabulary in the body
    // (comments are not token-stream members — the vocabulary must be in
    // code).
    if !F16_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    if !REDUCE_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    // Suppression: the rescale already applied (a scale vocabulary in
    // the body).
    if SCALE_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // The site arms: a weights-prepare vocabulary, or a wide threshold
    // comparison (the `_WIDE` gate shape).
    let weights_site = WEIGHT_TOKENS.iter().any(|t| lower.contains(t));
    let wide_gate = wide_threshold_site(block);
    if !weights_site && wide_gate.is_none() {
        return;
    }

    let span = wide_gate.unwrap_or_else(|| block_span(block));
    let text = source.get(span.clone()).unwrap_or("").to_string();
    findings.push(SubnormalFinding {
        site_span: span,
        site_text: text,
        advice: ADVICE,
    });
}

/// A wide-threshold comparison site: a literal `>= 1024` / `> 1024`
/// (or wider) against any expression — the `_WIDE` gate shape.
fn wide_threshold_site(block: &Block) -> Option<Range<usize>> {
    let mut walk = WideGateScan { hit: None };
    walk.visit_block(block);
    walk.hit
}

struct WideGateScan {
    hit: Option<Range<usize>>,
}

impl<'ast> syn::visit::Visit<'ast> for WideGateScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if self.hit.is_none() {
            if let Expr::Binary(b) = e {
                if matches!(b.op, syn::BinOp::Gt(_) | syn::BinOp::Ge(_)) {
                    let lit_is_wide = match b.right.as_ref() {
                        Expr::Lit(l) => match &l.lit {
                            syn::Lit::Int(i) => {
                                i.base10_parse::<u64>().is_ok_and(|v| v >= WIDE_THRESHOLD)
                            }
                            _ => false,
                        },
                        _ => false,
                    };
                    if lit_is_wide {
                        self.hit = Some(e.span().byte_range());
                    }
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

fn block_span(block: &Block) -> Range<usize> {
    block.span().byte_range()
}
