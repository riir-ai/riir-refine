//! `variance-cancellation-two-pass` — SUGGEST-ONLY detector
//! (Batch 366, the rounding-bugs numerics transcription; Research 412 /
//! Plan 380).
//!
//! The one-pass variance `E[x²] − mean²` cancels catastrophically when
//! the inputs share a large mean: at the blog's [4095, 4097, 4097]
//! f32 slice the scatter-mean tie at 2²⁵ loses the 2-ulp tail and the
//! form returns EXACTLY −2.0 where the true variance is 8/9 (George &
//! Rush, live-fetch 2026-11-16; JAX documents the same and ships
//! `algorithm="stable"` — the stable form is decision content; `PyTorch`
//! Geometric's `VarAggregation` is upstream-naive). The fix is the
//! two-pass `mean((x−μ)²)`.
//!
//! Why suggest-only: the two-pass costs an extra pass over the data
//! (streaming variants exist — Welford — but which one applies is a
//! per-lane memory/bandwidth call). NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm.
//!
//! Detector: `proposers::variance_two_pass`. Fixture:
//! `fixtures/b366_fixtures.rs` F3. Batch 366 / Plan 380 / Research 412.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected one-pass variance cancellation site.
#[derive(Debug, Clone)]
pub struct VarianceFinding {
    /// Byte range of the fn body.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the two-pass rearrangement).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "the one-pass variance E[x^2] - mean^2 cancels \
catastrophically at a large shared mean (at [4095, 4097, 4097] f32 it returns \
exactly -2.0 where the true variance is 8/9; JAX documents this and ships \
algorithm=\"stable\"). Rearrange to the two-pass mean((x - mean)^2), or a \
Welford stream when a single pass is required (Batch 366, suggest-only)";

/// The variance vocabulary (the class-shape gate).
const VARIANCE_TOKENS: &[&str] = &["var", "variance", "std", "normalize", "standardize"];

/// Suppression vocabulary: the two-pass (or Welford) already applied.
const SUPPRESS_TOKENS: &[&str] = &["welford", "m2", "(x -", "(x_i -", "centered"];

/// Detect one-pass variance bodies (suggest-only; no edits are produced
/// by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<VarianceFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<VarianceFinding>) {
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

fn scan_block(source: &str, ident: &str, block: &Block, findings: &mut Vec<VarianceFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    // The vocabulary gate reads IDENT + BODY — the variance name
    // conventionally lives in the fn name (`layer_variance`).
    let mut ctx = ident.to_string();
    ctx.push_str(&compact);
    let lower = ctx.to_lowercase();

    // Class-shape gate: the variance vocabulary in the body.
    if !VARIANCE_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // Suppression: the two-pass/Welford rearrangement is already applied.
    if SUPPRESS_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // The bug shape: a subtraction whose right leg is a SQUARED mean
    // (the same expression multiplied by itself).
    if !mean_squared_subtraction(block) {
        return;
    }

    let span = block.span().byte_range();
    findings.push(VarianceFinding {
        site_span: span.clone(),
        site_text: source.get(span).unwrap_or("").to_string(),
        advice: ADVICE,
    });
}

/// `something - mean * mean` (or `something - pow(mean, 2)`): the
/// right leg of the subtraction is the mean times itself.
fn mean_squared_subtraction(block: &Block) -> bool {
    let mut walk = SquaredMeanScan { hit: false };
    walk.visit_block(block);
    walk.hit
}

struct SquaredMeanScan {
    hit: bool,
}

impl<'ast> syn::visit::Visit<'ast> for SquaredMeanScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if !self.hit {
            if let Expr::Binary(sub) = e {
                if matches!(sub.op, syn::BinOp::Sub(_)) && squared_same_operand(&sub.right) {
                    self.hit = true;
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

/// Is this expression `X * X` (identical operands) or
/// `X.powi(2)` / `X.powf(2.0)` / `X * X` under a cast?
fn squared_same_operand(e: &Expr) -> bool {
    match e {
        Expr::Binary(b) if matches!(b.op, syn::BinOp::Mul(_)) => {
            let l = b.left.to_token_stream().to_string();
            let r = b.right.to_token_stream().to_string();
            normalize_ws(&l) == normalize_ws(&r)
        }
        Expr::MethodCall(m) if m.method == "powi" || m.method == "powf" => {
            let ts = m.args.to_token_stream().to_string();
            let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
            compact == "2" || compact == "2.0"
        }
        Expr::Paren(p) => squared_same_operand(&p.expr),
        Expr::Cast(c) => squared_same_operand(&c.expr),
        _ => false,
    }
}

fn normalize_ws(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}
