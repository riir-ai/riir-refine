//! `wide-reduction-precision-split` — SUGGEST-ONLY detector
//! (Batch 352, the `LoNE` distill; Research 395 / Plan 362).
//!
//! ONE wide output projection loses parity against TWO half-width ones
//! summed at the output — `LoNE`'s measured case (AJ-Gonzalez/LoNE @
//! `4806171d`, Apache-2.0, verbatim at the pin): "k=6656 measured at
//! 0.997 cosine versus torch, k=3328 at 0.99999." Thresholds measured,
//! not arbitrary: FFN fan-in > 4096, LM head hidden > 1024.
//!
//! THE ADJUDICATION (load-bearing, carried in-rule): DISTINCT-FROM the
//! split-K family (`kernel_opt` gpu_127-class) — split-K partitions the
//! reduction across workers for THROUGHPUT and re-reduces partials
//! mid-kernel; this rule splits for PARITY, sums the halves AT THE END,
//! and carries no work-partition content. The MOTIVATION is the
//! discriminator.
//!
//! Why suggest-only: the threshold is model-family-measured and the
//! parity gate decides whether the split is wanted at all. NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! Detector: `proposers::wide_reduction_split`. Fixture:
//! `fixtures/b352_fixtures.rs` F2. Batch 352 / Plan 362 / Research 395.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected single-pass wide projection site.
#[derive(Debug, Clone)]
pub struct WideSplitFinding {
    /// Byte range of the enclosing fn's wide-gate or whole fn.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the halves-summed-at-end law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "one wide f16 projection measures worse parity than \
two half-width ones summed at the output (measured: 6656-wide 0.9972 vs two \
3328-wide 0.999994). Split the reduction at the measured threshold (FFN \
fan-in > 4096, LM head hidden > 1024 in the source), compute the halves \
independently, and sum AT THE END — parity motivation, never a throughput \
partition (that is split-K's rule, different decision) (Batch 352, \
suggest-only)";

/// The projection compute vocabulary (the class-shape gate).
const PROJ_TOKENS: &[&str] = &[
    "matvec", "matmul", "gemv", "gemm", "proj", "linear", "ffn", "head",
];

/// f16 arithmetic vocabulary — the regime gate.
const F16_TOKENS: &[&str] = &["f16", "F16", "half", "Half"];

/// The split vocabulary whose PRESENCE suppresses (the fix already
/// applied) — the source's own half-projection naming.
const SPLIT_TOKENS: &[&str] = &["split", "w2a", "w2b", "_a_inv"];

/// The FFN threshold literal the source pins: projections wider than
/// this are the regime (`intermediate_size > 4096`).
const FFN_THRESHOLD: u64 = 4096;

/// Detect single-pass wide f16 projections (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<WideSplitFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<WideSplitFinding>) {
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
            // fixture habitat, not production projection sites.
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

fn scan_block(source: &str, block: &Block, findings: &mut Vec<WideSplitFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();

    // Class-shape gate: f16 + projection vocabulary in the body.
    if !F16_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    if !PROJ_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    // Suppression: the split already applied (the source's naming).
    if SPLIT_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    // The wide site: a literal threshold >= 4096 in a comparison, or an
    // f16-typed accumulator single-loop reduction (the narrow
    // accumulation the parity split is wanted for).
    let wide_gate = wide_threshold_site(block);
    let narrow_accum = f16_accumulator_site(block);
    if wide_gate.is_none() && !narrow_accum {
        return;
    }

    let span = wide_gate.unwrap_or_else(|| block.span().byte_range());
    let text = source.get(span.clone()).unwrap_or("").to_string();
    findings.push(WideSplitFinding {
        site_span: span,
        site_text: text,
        advice: ADVICE,
    });
}

/// A wide-threshold comparison site: a literal `> 4096` / `>= 4096`
/// (or wider) — the `intermediate_size > 4096` gate shape.
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
                                i.base10_parse::<u64>().is_ok_and(|v| v >= FFN_THRESHOLD)
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

/// An f16-typed accumulator in the body (`acc` + an `f16(` construction
/// or `: f16` annotation) — the narrow accumulation the parity split is
/// wanted for.
fn f16_accumulator_site(block: &Block) -> bool {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("acc")
        && (compact.contains("f16(") || compact.contains("F16::") || compact.contains(":f16"))
}
