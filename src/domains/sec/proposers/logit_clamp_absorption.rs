//! `logit-clamp-epsilon-absorption` — SUGGEST-ONLY detector
//! (Batch 366, the rounding-bugs numerics transcription; Research 412 /
//! Plan 380).
//!
//! An eps-guarded logit that clamps in the INPUT domain absorbs its own
//! guard: `hi = 1.0 − eps` rounds to 1.0 in f32 for eps = 1e−8
//! (half-ulp below 1 is 2.98e−8), the clamp is a no-op at the top, and
//! `logit(1.0)` returns +∞ (George & Rush, live-fetch 2026-11-16;
//! `PyTorch`'s `torch.special.logit(x, eps=1e-8)` is upstream-naive,
//! #204898). The fix clamps in the OUTPUT domain — clamp the log
//! result to `±ln((1−eps)/eps)` computed in a wider lane.
//!
//! Why suggest-only: the guard's placement is a wire/API decision (the
//! eps contract may be input-domain by spec — then the fix changes the
//! function's documented behavior and needs a versioned bump). NO
//! `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! Detector: `proposers::logit_clamp_absorption`. Fixture:
//! `fixtures/b366_fixtures.rs` F5. Batch 366 / Plan 380 / Research 412.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected input-domain eps-clamped logit site.
#[derive(Debug, Clone)]
pub struct LogitClampFinding {
    /// Byte range of the fn body.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the output-domain clamp).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "an eps-guarded logit that clamps in the INPUT domain \
absorbs its own guard: 1.0 - eps rounds to 1.0 in f32 for eps <= half-ulp-below-1 \
(eps = 1e-8), the clamp is a no-op at the top, and logit(1.0) = +inf. Clamp in \
the OUTPUT domain: bound the log RESULT to +-ln((1-eps)/eps) computed in a \
wider lane (Batch 366, suggest-only)";

/// The logit vocabulary (the class-shape gate).
const LOGIT_TOKENS: &[&str] = &["logit", "log_odds", "ln(", "log("];

/// Suppression vocabulary: the output-domain clamp already applied (the
/// bound is derived FROM eps through the log, not built as `1 - eps`).
const SUPPRESS_TOKENS: &[&str] = &["(1.0-eps)/eps", "(1.0-e)/e", "output_clamp"];

/// Detect input-domain eps-clamped logits (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<LogitClampFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<LogitClampFinding>) {
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

fn scan_block(source: &str, ident: &str, block: &Block, findings: &mut Vec<LogitClampFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    // The vocabulary gate reads IDENT + BODY — the logit name
    // conventionally lives in the fn name (`guarded_logit`).
    let mut ctx = ident.to_string();
    ctx.push_str(&compact);
    let lower = ctx.to_lowercase();

    // Class-shape gate: the logit vocabulary in the body.
    if !LOGIT_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // Suppression: the output-domain clamp is already applied.
    if SUPPRESS_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    // The bug shape: a `1.0 - <eps>` binding used as a clamp bound in a
    // body that takes a log of a probability ratio.
    if !input_domain_eps_bound(block) {
        return;
    }

    let span = block.span().byte_range();
    findings.push(LogitClampFinding {
        site_span: span.clone(),
        site_text: source.get(span).unwrap_or("").to_string(),
        advice: ADVICE,
    });
}

/// The absorbed-guard shape: a let binding whose init is
/// `1.0 - <eps-named ident>` (the vanished upper bound), in a body that
/// clamps (min/max/clamp vocabulary) and takes a log.
fn input_domain_eps_bound(block: &Block) -> bool {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    let lower = compact.to_lowercase();

    let clamps = ["min(", "max(", "clamp("].iter().any(|t| lower.contains(t));
    let logs = [".ln(", ".log(", "f32::ln", "f64::ln"]
        .iter()
        .any(|t| lower.contains(t));
    if !(clamps && logs) {
        return false;
    }

    let mut walk = EpsBoundScan { hit: false };
    walk.visit_block(block);
    walk.hit
}

struct EpsBoundScan {
    hit: bool,
}

impl<'ast> syn::visit::Visit<'ast> for EpsBoundScan {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if !self.hit {
            if let Some(init) = &l.init {
                if let Expr::Binary(b) = &*init.expr {
                    if matches!(b.op, syn::BinOp::Sub(_))
                        && is_lit_one(&b.left)
                        && eps_named(&b.right)
                    {
                        self.hit = true;
                    }
                }
            }
        }
        syn::visit::visit_local(self, l);
    }
}

fn is_lit_one(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Float(f) => f
                .base10_parse::<f64>()
                .is_ok_and(|v| (v - 1.0).abs() < 1e-9),
            syn::Lit::Int(i) => i
                .base10_parse::<f64>()
                .is_ok_and(|v| (v - 1.0).abs() < 1e-9),
            _ => false,
        },
        Expr::Paren(p) => is_lit_one(&p.expr),
        _ => false,
    }
}

/// Is this expression an identifier (or simple path) naming eps?
fn eps_named(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => {
            let ts = p.path.get_ident().map_or_else(
                || p.path.to_token_stream().to_string().to_lowercase(),
                |i| i.to_string().to_lowercase(),
            );
            ts.contains("eps") || ts.contains("epsilon")
        }
        _ => false,
    }
}
