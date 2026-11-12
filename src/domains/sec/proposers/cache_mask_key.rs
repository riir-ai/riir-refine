//! `position-keyed-additive-cache-mask` — SUGGEST-ONLY detector
//! (Batch 352, the `LoNE` distill; Research 395 / Plan 362).
//!
//! A KV cache preallocated to full context holds EMPTY rows past the
//! written cursor; an additive mask over `past || new` keys that bounds
//! validity by BUFFER INDEX (`i < past_len`) is right only while the
//! buffer stays compact — the first ring rotation makes row i a slot
//! whose absolute position is NOT i. `LoNE`'s law (AJ-Gonzalez/LoNE @
//! `4806171d`, Apache-2.0, verbatim at the pin): "Past rows are valid
//! only up to `pos`; the rest of the preallocated buffer is empty, and
//! the new block is causal within itself. Mask by absolute position,
//! never by buffer index." The additive sentinel is fp16's −65504.
//!
//! Why suggest-only: whether the buffer is compacted-by-construction is
//! a design property the detector cannot see, and the mask dtype/sentinel
//! is a wire decision. NO `FIXABLE_SEC` membership, NO `collect_edits`
//! arm.
//!
//! Detector: `proposers::cache_mask_key`. Fixture:
//! `fixtures/b352_fixtures.rs` F4. Batch 352 / Plan 362 / Research 395.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, File, Item};

/// One detected index-keyed mask over a preallocated cache.
#[derive(Debug, Clone)]
pub struct MaskKeyFinding {
    /// Byte range of the mask-building fn body.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the absolute-position law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a preallocated KV cache holds empty rows past the \
written cursor; an additive mask that bounds validity by BUFFER INDEX is \
wrong the moment the buffer rotates (row i's slot position is no longer i). \
Carry each slot's absolute position and bound validity by \
slot_pos <= query_pos (+ the sliding-window floor); the fp16 additive \
sentinel is -65504, never -inf (Batch 352, suggest-only)";

/// The mask-building vocabulary (the class-shape gate).
const MASK_TOKENS: &[&str] = &["mask", "allowed", "valid"];

/// The preallocated-cache vocabulary (the regime gate).
const CACHE_TOKENS: &[&str] = &["cache", "kv", "ring", "past", "ctx", "window"];

/// The additive-sentinel vocabulary (the additive-mask shape).
const SENTINEL_TOKENS: &[&str] = &["65504", "neg_infinity", "f32::min"];

/// The buffer-index validity shape: a `<`/`<=` comparison whose LEFT is
/// a loop-index name and whose RIGHT is a count/cursor name.
const INDEX_NAMES: &[&str] = &["i", "idx", "index", "j", "row"];

/// The count/cursor names the index is wrongly compared against.
const CURSOR_NAMES: &[&str] = &[
    "pos", "past", "past_len", "n_past", "n_seen", "cursor", "count", "len", "n",
];

/// The position-carrying vocabulary whose PRESENCE suppresses (the fix
/// already applied).
const POSITION_TOKENS: &[&str] = &["slot_pos", "past_pos", "slots["];

/// Detect index-keyed masks over preallocated caches (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<MaskKeyFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<MaskKeyFinding>) {
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
            // fixture habitat, not production mask sites.
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

fn scan_block(source: &str, block: &Block, findings: &mut Vec<MaskKeyFinding>) {
    let ts = quote::ToTokens::to_token_stream(block).to_string();
    let compact: String = ts.chars().filter(|c| !c.is_whitespace()).collect();
    let lower = compact.to_lowercase();

    // Class-shape gate: mask vocabulary + cache vocabulary + the
    // additive sentinel in the body (case-insensitive: the constants
    // are conventionally UPPER).
    if !MASK_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    if !CACHE_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    if !SENTINEL_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // Suppression: the positions are carried — the fix already applied.
    if POSITION_TOKENS.iter().any(|t| lower.contains(t)) {
        return;
    }
    // The bug shape: an index-vs-count validity comparison in the body.
    if !index_keyed_site(block) {
        return;
    }

    let span = block.span().byte_range();
    findings.push(MaskKeyFinding {
        site_span: span.clone(),
        site_text: source.get(span).unwrap_or("").to_string(),
        advice: ADVICE,
    });
}

/// An index-vs-count comparison: `i < n_seen` / `idx <= cursor` — a
/// short identifier compared against a cursor/count-named operand.
fn index_keyed_site(block: &Block) -> bool {
    let mut walk = IndexKeyScan { hit: false };
    walk.visit_block(block);
    walk.hit
}

struct IndexKeyScan {
    hit: bool,
}

impl<'ast> syn::visit::Visit<'ast> for IndexKeyScan {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if !self.hit {
            if let Expr::Binary(b) = e {
                if matches!(b.op, syn::BinOp::Lt(_) | syn::BinOp::Le(_)) {
                    let left = operand_name(&b.left);
                    let right = operand_name(&b.right);
                    if let (Some(l), Some(r)) = (left, right) {
                        if INDEX_NAMES.contains(&l.as_str()) && CURSOR_NAMES.contains(&r.as_str()) {
                            self.hit = true;
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

/// The bare-identifier name of an operand, when it is one.
fn operand_name(e: &Expr) -> Option<String> {
    match e {
        Expr::Path(p) => p.path.get_ident().map(std::string::ToString::to_string),
        _ => None,
    }
}
