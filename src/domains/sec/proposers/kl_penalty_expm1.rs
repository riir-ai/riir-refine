//! `kl-penalty-exp-minus-d-cancellation` — SUGGEST-ONLY detector
//! (Batch 366, the rounding-bugs numerics transcription; Research 412 /
//! Plan 380).
//!
//! The k3 KL penalty `e^d − d − 1` collapses to exactly 0.0 at small
//! |d|: at d = 2^−12 f32, `d` is 2048 ulps above 1.0 and `d²/2` is
//! exactly a quarter-ulp, so correctly-rounded `exp(d)` rounds to
//! `1+d` and the form returns 0.0 where the true penalty is ≈ 3.0e−8
//! (George & Rush, live-fetch 2026-11-16; TRL runs this line whenever
//! its GRPO `beta` > 0 — UPSTREAM-NAIVE, transcription only, NEVER a
//! PR). The fix is `expm1(d) − d`.
//!
//! Why suggest-only: the penalty's own gradient survives the collapse
//! (the source says so), so whether the zero MATTERS is a per-lane
//! call. NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! Detector: `proposers::kl_penalty_expm1`. Fixture:
//! `fixtures/b366_fixtures.rs` F2. Batch 366 / Plan 380 / Research 412.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected `e^d − d − 1` cancellation site.
#[derive(Debug, Clone)]
pub struct KlPenaltyFinding {
    /// Byte range of the fn body.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the expm1 rearrangement).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "the k3 KL penalty e^d - d - 1 collapses to exactly 0.0 \
at small |d| (at d = 2^-12 f32 the d^2/2 quarter-ulp is lost when exp(d) rounds \
to 1+d; the true penalty is ~3.0e-8). Rearrange to expm1(d) - d — expm1 keeps \
the small difference the direct subtraction rounds away (Batch 366, suggest-only)";

/// The penalty vocabulary (the class-shape gate).
const PENALTY_TOKENS: &[&str] = &["kl", "penalty", "k3", "divergence", "reward"];

/// Suppression vocabulary: the expm1 rearrangement already applied.
const SUPPRESS_TOKENS: &[&str] = &["exp_m1", "expm1", "log1p", "log_sigmoid"];

/// Detect `e^d − d − 1` bodies (suggest-only; no edits are produced by
/// this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<KlPenaltyFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<KlPenaltyFinding>) {
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

fn scan_block(source: &str, ident: &str, block: &Block, findings: &mut Vec<KlPenaltyFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    // The vocabulary gate reads IDENT + BODY — the penalty name
    // conventionally lives in the fn name (`kl_penalty_k3`).
    let mut ctx = ident.to_string();
    ctx.push_str(&compact);
    let lower = ctx.to_lowercase();

    // Class-shape gate: the penalty vocabulary in the body.
    if !PENALTY_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // Suppression: the expm1 rearrangement is already applied.
    if SUPPRESS_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // The bug shape: `.exp()` minus the same receiver minus literal 1.
    if !exp_minus_self_minus_one(block) {
        return;
    }

    let span = block.span().byte_range();
    findings.push(KlPenaltyFinding {
        site_span: span.clone(),
        site_text: source.get(span).unwrap_or("").to_string(),
        advice: ADVICE,
    });
}

/// The cancellation shape: `x.exp() - x ... - 1.0` (either operand
/// order of the trailing subtraction).
fn exp_minus_self_minus_one(block: &Block) -> bool {
    let mut walk = ExpMinusScan { hit: false };
    walk.visit_block(block);
    walk.hit
}

struct ExpMinusScan {
    hit: bool,
}

impl<'ast> syn::visit::Visit<'ast> for ExpMinusScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if !self.hit {
            // The chained subtraction root: (a - b) - c or a - (b - c).
            if let Expr::Binary(outer) = e {
                if matches!(outer.op, syn::BinOp::Sub(_)) {
                    // Left leg: an exp-call minus an identifier.
                    if let Expr::Binary(inner) = &*outer.left {
                        if matches!(inner.op, syn::BinOp::Sub(_)) {
                            let recv = exp_receiver_ident(&inner.left);
                            let sub_ident = path_ident(&inner.right);
                            if let (Some(r), Some(s)) = (recv, sub_ident) {
                                if r == s && sub_leg_is_one(&outer.right) {
                                    self.hit = true;
                                }
                            }
                        }
                    }
                    // Mirror: an identifier minus an exp-call, minus 1.
                    if let Expr::Binary(inner) = &*outer.right {
                        if matches!(inner.op, syn::BinOp::Sub(_)) {
                            let recv = exp_receiver_ident(&inner.right);
                            let sub_ident = path_ident(&inner.left);
                            if let (Some(r), Some(s)) = (recv, sub_ident) {
                                if r == s && sub_leg_is_one(&outer.left) {
                                    self.hit = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

/// The identifier an `.exp()` method call is invoked on.
fn exp_receiver_ident(e: &Expr) -> Option<String> {
    match e {
        Expr::MethodCall(m) if m.method == "exp" => path_ident(&m.receiver),
        _ => None,
    }
}

fn path_ident(e: &Expr) -> Option<String> {
    match e {
        Expr::Path(p) => p.path.get_ident().map(std::string::ToString::to_string),
        _ => None,
    }
}

/// A literal-1 leg (1, 1.0, `1_f32`, ...).
fn sub_leg_is_one(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(i) => i
                .base10_parse::<f64>()
                .is_ok_and(|v| (v - 1.0).abs() < 1e-9),
            syn::Lit::Float(f) => f
                .base10_parse::<f64>()
                .is_ok_and(|v| (v - 1.0).abs() < 1e-9),
            _ => false,
        },
        Expr::Unary(u) => {
            let Expr::Lit(l) = &*u.expr else {
                return false;
            };
            match &l.lit {
                syn::Lit::Int(i) => i
                    .base10_parse::<f64>()
                    .is_ok_and(|v| (v + 1.0).abs() < 1e-9),
                syn::Lit::Float(f) => f
                    .base10_parse::<f64>()
                    .is_ok_and(|v| (v + 1.0).abs() < 1e-9),
                _ => false,
            }
        }
        _ => false,
    }
}
