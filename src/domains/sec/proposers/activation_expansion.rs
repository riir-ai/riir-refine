//! `fused-activation-approximation-expansion` — SUGGEST-ONLY detector
//! (Batch 352, the `LoNE` distill; Research 395 / Plan 362).
//!
//! When the backend runtime substitutes an APPROXIMATE fused activation
//! for the composed form, expand the activation to its primitive
//! composition to keep the op off that path — `LoNE`'s measured case
//! (AJ-Gonzalez/LoNE @ `4806171d`, Apache-2.0, verbatim at the pin):
//! "Core ML maps `aten::silu` (and `x * sigmoid(x)`) to an NE activation
//! with ~1e-3 relative error; the exp form keeps the activation off that
//! path (measured 0.99935 -> 0.999991 on a real LFM3 FFN)."
//!
//! THE ADJUDICATION (load-bearing, BOTH directions, carried in-rule):
//! DISTINCT-FROM `fuse-gate-up-gemv-activation` and the GLU-fusion
//! family (`kernel_opt` `gpu_96/35/126/147`) — THOSE fuse sibling GEMVs and
//! their activation into one dispatch for THROUGHPUT; THIS rule EXPANDS
//! a fused activation into primitive ops for PARITY. The axes are
//! opposite and never merge (the source itself composes both).
//!
//! Why suggest-only: the expansion costs extra backend ops — wanted
//! only where parity is the goal, never in a throughput-only serving
//! path. NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! Detector: `proposers::activation_expansion`. Fixture:
//! `fixtures/b352_fixtures.rs` F3. Batch 352 / Plan 362 / Research 395.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected fused-approximation activation site.
#[derive(Debug, Clone)]
pub struct ActivationFinding {
    /// Byte range of the activation definition or call site.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the exp-form composition).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "when the backend runtime substitutes an approximate \
fused activation for the composed form (~1e-3 relative error measured: gate \
0.99935 -> 0.999991), expand to the primitive composition in parity-critical \
paths — silu as x / (1 + exp(-x)), gelu in its erf form — so the op stays off \
the approximation path. Opposite axis of the GLU-fusion family: they fuse for \
dispatch economy, this expands for parity; the two compose, never merge \
(Batch 352, suggest-only)";

/// The fused-activation vocabulary (the class-shape gate).
const ACTIVATION_TOKENS: &[&str] = &["silu", "swish", "gelu"];

/// The mul-composition marker: the backend also maps `x * sigmoid(x)`
/// to the approximate NE op (detected as a sigmoid call in a
/// silu/swish-named definition).
const MUL_MARKER: &str = "sigmoid";

/// The exp-form markers — the fix already applied (both spellings: the
/// free-fn form `exp(-x)` and the method-call form `(-x).exp()`; the
/// compact form of the latter is `(-x).exp()`).
const EXP_MARKER_FREE: &str = "exp(-";
const EXP_MARKER_METHOD: &str = ".exp()";

fn has_exp_marker(compact: &str) -> bool {
    compact.contains(EXP_MARKER_FREE) || compact.contains(EXP_MARKER_METHOD)
}

/// The parity-critical vocabulary (the regime gate — where the expansion
/// is wanted at all).
const PARITY_TOKENS: &[&str] = &[
    "cosine",
    "parity",
    "golden",
    "tolerance",
    "max_abs",
    "abs_err",
    "reference",
    "ref_out",
    "drift",
    "gate_threshold",
];

/// Detect fused-approximation activation sites in parity-critical
/// contexts (suggest-only; no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<ActivationFinding> {
    let mut findings = Vec::new();
    // File-level state: an exp-form silu/swish definition anywhere in
    // the file means the file has adopted the fix — its call sites are
    // not re-flagged (the definition is the fix, the calls just use it).
    let file_has_exp_form = file_has_exp_form_silu(&file.items);
    for item in &file.items {
        collect_in_item(source, item, file_has_exp_form, &mut findings);
    }
    findings
}

/// Whether any fn in the item list defines a silu/swish activation in
/// the exp form (the adopted fix).
fn file_has_exp_form_silu(items: &[Item]) -> bool {
    let mut hit = false;
    for item in items {
        walk_items(item, &mut |name: &str, block: &Block| {
            let is_activation = ["silu", "swish"].iter().any(|a| name.contains(a));
            if !is_activation {
                return;
            }
            let compact = compact_of(block);
            if has_exp_marker(&compact) {
                hit = true;
            }
        });
    }
    hit
}

fn collect_in_item(
    source: &str,
    item: &Item,
    file_has_exp_form: bool,
    findings: &mut Vec<ActivationFinding>,
) {
    match item {
        Item::Fn(f) => {
            scan_fn(
                source,
                &f.sig.ident.to_string(),
                &f.block,
                file_has_exp_form,
                findings,
            );
        }
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(
                        source,
                        &m.sig.ident.to_string(),
                        &m.block,
                        file_has_exp_form,
                        findings,
                    );
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production activation sites.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, file_has_exp_form, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_fn(
    source: &str,
    name: &str,
    block: &Block,
    file_has_exp_form: bool,
    findings: &mut Vec<ActivationFinding>,
) {
    let compact = compact_of(block);

    // Arm A: the fn DEFINES the fused activation — a silu/swish-named fn
    // whose body is the mul-by-sigmoid composition (the spelling the
    // backend maps to the approximate NE op). Fires regardless of
    // parity vocabulary: the definition itself is the site.
    let name_is_activation = ["silu", "swish"].iter().any(|a| name.contains(a));
    if name_is_activation {
        if compact.contains(MUL_MARKER) && compact.contains('*') && !has_exp_marker(&compact) {
            let span = block.span().byte_range();
            findings.push(ActivationFinding {
                site_span: span.clone(),
                site_text: source.get(span).unwrap_or("").to_string(),
                advice: ADVICE,
            });
        }
        // The exp-form definition is already the fix — silent.
        return;
    }

    // Arm B: an activation CALL inside a parity-critical fn (the
    // expansion is wanted only where parity is the goal), suppressed
    // when the file already carries the exp-form definition.
    let has_activation = ACTIVATION_TOKENS.iter().any(|t| compact.contains(t));
    let is_parity_context = PARITY_TOKENS.iter().any(|t| compact.contains(t));
    if has_activation && is_parity_context && !file_has_exp_form {
        let span = activation_call_span(block).unwrap_or_else(|| block.span().byte_range());
        findings.push(ActivationFinding {
            site_span: span.clone(),
            site_text: source.get(span).unwrap_or("").to_string(),
            advice: ADVICE,
        });
    }
}

fn compact_of(block: &Block) -> String {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    ts.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Walk fn-like items (fns + impl methods + nested mods), visiting each
/// with its name.
fn walk_items(item: &Item, f: &mut dyn FnMut(&str, &Block)) {
    match item {
        Item::Fn(fn_item) => f(&fn_item.sig.ident.to_string(), &fn_item.block),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    f(&m.sig.ident.to_string(), &m.block);
                }
            }
        }
        Item::Mod(m) if let Some((_, items)) = &m.content => {
            for nested in items {
                walk_items(nested, f);
            }
        }
        _ => {}
    }
}

/// The first silu/swish/gelu METHOD-call site's span in the block.
fn activation_call_span(block: &Block) -> Option<Range<usize>> {
    let mut walk = ActivationCallScan { hit: None };
    walk.visit_block(block);
    walk.hit
}

struct ActivationCallScan {
    hit: Option<Range<usize>>,
}

impl<'ast> syn::visit::Visit<'ast> for ActivationCallScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if self.hit.is_none() {
            if let Expr::MethodCall(m) = e {
                let name = m.method.to_string();
                if ACTIVATION_TOKENS.contains(&name.as_str()) {
                    self.hit = Some(e.span().byte_range());
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}
