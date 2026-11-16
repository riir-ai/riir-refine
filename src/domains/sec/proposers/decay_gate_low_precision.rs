//! `decay-gate-precision-collapse-low-precision` — SUGGEST-ONLY detector
//! (Batch 366, the rounding-bugs numerics transcription; Research 412 /
//! Plan 380).
//!
//! The EMA update `decay·avg + (1−decay)·param` STOPS MOVING in a
//! low-precision lane: decay 0.999 rounds to 1.0 in bf16 (8-bit
//! significand), the increment `(1−decay)·param` becomes 0.0, and a
//! moving average at 1/64 stays there for 5,000 steps (George & Rush,
//! live-fetch 2026-11-16; the source's second instance: an xLSTM
//! forget-gate decay between nearby steps rounds to 1 instead of
//! ≈ 0.9526). The fix runs the update in a wider lane (f32) — the
//! blog's one-step value 8389263/2^29.
//!
//! ⛔ THE ISSUE-940 CARVE-OUT (carried in-rule): f32 sync-crossing
//! reductions stay ORDER-PINNED — the determinism law is not
//! overturned; this rule is LOW-PRECISION LOCAL LANES ONLY (bf16/f16
//! decay gates), regimes disjoint.
//!
//! Why suggest-only: the wider lane costs memory/bandwidth per
//! parameter and some frameworks keep a bf16 master-copy by design;
//! which lanes widen is a per-lane call. NO `FIXABLE_SEC` membership,
//! NO `collect_edits` arm.
//!
//! Detector: `proposers::decay_gate_low_precision`. Fixture:
//! `fixtures/b366_fixtures.rs` F4. Batch 366 / Plan 380 / Research 412.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected low-precision decay-gate site.
#[derive(Debug, Clone)]
pub struct DecayGateFinding {
    /// Byte range of the fn body.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the wider-lane update).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a decay-gated update decay*avg + (1-decay)*param \
STOPS MOVING in a low-precision lane: decay 0.999 rounds to 1.0 in bf16, the \
(1-decay)*param increment rounds to 0, and the average never moves (measured: \
1/64 held for 5000 steps; an xLSTM forget-gate decay rounds to 1 instead of \
~0.9526). Run the update in a WIDER lane (f32) — low-precision local lanes \
only, never an f32 sync-crossing reduction (those stay order-pinned, Issue-940) \
(Batch 366, suggest-only)";

/// The decay-gate vocabulary (the class-shape gate).
const DECAY_TOKENS: &[&str] = &["decay", "ema", "momentum", "moving_avg", "beta"];

/// Suppression vocabulary: the wider-lane update already applied.
const SUPPRESS_TOKENS: &[&str] = &["to_f32", "f32::from", "as f32"];

/// Detect low-precision decay-gate bodies (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<DecayGateFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<DecayGateFinding>) {
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

fn scan_block(source: &str, ident: &str, block: &Block, findings: &mut Vec<DecayGateFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    // The vocabulary gate reads IDENT + BODY.
    let mut ctx = ident.to_string();
    ctx.push_str(&compact);
    let lower = ctx.to_lowercase();

    // Class-shape gate: the decay vocabulary in the body.
    if !DECAY_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // Suppression: the wider-lane update is already applied (the fix's
    // f32 widening vocabulary is present in the body).
    if SUPPRESS_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // The bug shape: `decay * x + (1.0 - decay) * y` — an addition of
    // two products where one product's factor is `1.0 - <the other
    // product's factor>`.
    if !decay_update_shape(block) {
        return;
    }

    let span = block.span().byte_range();
    findings.push(DecayGateFinding {
        site_span: span.clone(),
        site_text: source.get(span).unwrap_or("").to_string(),
        advice: ADVICE,
    });
}

/// The `decay*x + (1-decay)*y` shape (operand order either way).
fn decay_update_shape(block: &Block) -> bool {
    let mut walk = DecayScan { hit: false };
    walk.visit_block(block);
    walk.hit
}

struct DecayScan {
    hit: bool,
}

impl<'ast> syn::visit::Visit<'ast> for DecayScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if !self.hit {
            if let Expr::Binary(add) = e {
                if matches!(add.op, syn::BinOp::Add(_)) {
                    let terms = mul_factor_names(&add.left, &add.right);
                    if terms.len() == 2 {
                        // Each term names its own factor set; the decay
                        // shape = the two factor sets share one name and
                        // one term carries `1.0 - <that name>`.
                        let (a, b) = (&terms[0], &terms[1]);
                        let shared: Vec<&String> = a.iter().filter(|n| b.contains(n)).collect();
                        for name in shared {
                            let one_minus = format!("1.0-{name}");
                            let one_minus_spelled = format!("1-{name}");
                            let ts = add.to_token_stream().to_string();
                            let compact: String =
                                ts.chars().filter(|c| !c.is_whitespace()).collect();
                            if compact.contains(&one_minus) || compact.contains(&one_minus_spelled)
                            {
                                self.hit = true;
                            }
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

/// The bare-identifier factor names of an addition's two legs (each leg
/// expected to be a product; returns the per-leg name sets, or an empty
/// vec when either leg is not a product).
fn mul_factor_names(left: &Expr, right: &Expr) -> Vec<Vec<String>> {
    match (factor_names(left), factor_names(right)) {
        (Some(a), Some(b)) => vec![a, b],
        _ => Vec::new(),
    }
}

/// The bare-identifier names multiplied together in an expression
/// (recursing through parentheses; an arithmetic GROUP like
/// `(1.0 - decay)` flattens to its idents; a non-product yields None).
fn factor_names(e: &Expr) -> Option<Vec<String>> {
    match e {
        Expr::Paren(p) => factor_names(&p.expr),
        Expr::Binary(b) if matches!(b.op, syn::BinOp::Mul(_)) => {
            let mut names = factor_names(&b.left)?;
            names.extend(factor_names(&b.right)?);
            Some(names)
        }
        // An arithmetic group (the `(1.0 - decay)` factor): flatten its
        // idents — the group contributes the names it binds.
        Expr::Binary(b)
            if matches!(
                b.op,
                syn::BinOp::Sub(_) | syn::BinOp::Add(_) | syn::BinOp::Div(_)
            ) =>
        {
            let mut names = group_idents(&b.left);
            names.extend(group_idents(&b.right));
            Some(names)
        }
        Expr::Path(p) => p.path.get_ident().map(|i| vec![i.to_string()]),
        Expr::Lit(_) => Some(Vec::new()),
        _ => None,
    }
}

/// The bare path idents under an expression (through parens and binary
/// ops; method calls and macros are opaque).
fn group_idents(e: &Expr) -> Vec<String> {
    match e {
        Expr::Paren(p) => group_idents(&p.expr),
        Expr::Binary(b) => {
            let mut names = group_idents(&b.left);
            names.extend(group_idents(&b.right));
            names
        }
        Expr::Path(p) => p
            .path
            .get_ident()
            .map(|i| vec![i.to_string()])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
