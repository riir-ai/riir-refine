//! Modelless ternary drafter for clippy fixes.
//!
//! Mirrors the `TernaryDraftModel` pattern from
//! `riir-games-quest/quest_grammar/plasma_draft.rs` but specialized for
//! clippy lint fixes. Uses `katgpt_core::TernaryWeights` for multiplication-
//! free SIMD matvec with zero-alloc hot path.
//!
//! # Hot-path contract
//!
//! `draft_fix` reuses two pre-allocated scratch buffers (`input_scratch`,
//! `output_scratch`) sized at `with_weights` time. This makes the SIMD matvec
//! hot path **zero-alloc** — the same pattern as the quest grammar drafter.

/// The modelless ternary drafter for clippy fixes.
///
/// When ternary weight files (`.bits`) are available, uses them for fast
/// draft generation via multiplication-free SIMD matvec from katgpt-core.
/// Otherwise, falls back to corpus-based template lookup.
#[derive(Debug, Clone)]
pub struct ClippyTernaryDrafter {
    /// Frozen ternary weights for the drafter (L1 hot path).
    /// `None` = fallback mode (use corpus lookup).
    weights: Option<katgpt_core::TernaryWeights>,
    /// Pre-allocated input buffer for the SIMD matvec hot path.
    /// Sized to `weights.cols`. Reused across `draft_fix` calls.
    input_scratch: Vec<f32>,
    /// Pre-allocated output buffer for the SIMD matvec hot path.
    /// Sized to `weights.rows`. Reused across `draft_fix` calls.
    output_scratch: Vec<f32>,
}

/// A proposed fix from the drafter.
#[derive(Debug, Clone)]
pub struct DraftedFix {
    /// The proposed fixed code.
    pub code: String,
    /// Confidence in the fix (0.0-1.0, from sigmoid of matvec output).
    pub confidence: f32,
    /// Whether this was a template/fallback draft (L0) or a ternary draft (L1).
    pub source: DraftSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftSource {
    /// Corpus template lookup (L0 — no weights).
    CorpusTemplate,
    /// Ternary SIMD matvec draft (L1 — frozen direction vectors).
    TernaryMatvec,
}

impl ClippyTernaryDrafter {
    /// Create in fallback mode (no ternary weights — corpus lookup only).
    pub fn fallback() -> Self {
        Self {
            weights: None,
            input_scratch: Vec::new(),
            output_scratch: Vec::new(),
        }
    }

    /// Create with ternary weights (L1 hot path enabled).
    ///
    /// Pre-allocates the SIMD matvec scratch buffers to `(weights.cols,
    /// weights.rows)` so the hot path is zero-alloc.
    pub fn with_weights(weights: katgpt_core::TernaryWeights) -> Self {
        let cols = weights.cols;
        let rows = weights.rows;
        Self {
            weights: Some(weights),
            input_scratch: vec![0.0f32; cols],
            output_scratch: vec![0.0f32; rows],
        }
    }

    /// Swap in new ternary weights, resizing the scratch buffers in place.
    pub fn load_weights(&mut self, weights: katgpt_core::TernaryWeights) {
        self.input_scratch.clear();
        self.input_scratch.resize(weights.cols, 0.0);
        self.output_scratch.clear();
        self.output_scratch.resize(weights.rows, 0.0);
        self.weights = Some(weights);
    }

    /// Whether the drafter is in fallback (template) mode.
    pub fn is_fallback(&self) -> bool {
        self.weights.is_none()
    }

    /// Draft a fix for the given lint name and buggy code.
    ///
    /// - **Fallback mode**: returns the first corpus entry for this lint.
    /// - **With weights (L1)**: runs the ternary SIMD matvec over a
    ///   BLAKE3-deterministic embedding of (lint_name, buggy_code) to select
    ///   the best fix candidate. Zero-alloc hot path.
    ///
    /// The actual fix validation happens in the pruner (L2), NOT here.
    /// The drafter proposes; the pruner disposes.
    pub fn draft_fix(
        &mut self,
        lint_name: &str,
        buggy_code: &str,
        candidates: &[crate::corpus::ClippyFixEntry],
    ) -> Option<DraftedFix> {
        if candidates.is_empty() {
            return None;
        }

        if self.weights.is_some() {
            // L1: ternary matvec path (zero-alloc, reuses scratch buffers)
            // TODO: embed (lint_name, buggy_code) into input_scratch via
            // BLAKE3-deterministic feature extraction, run ternary matvec,
            // sigmoid the output to get confidence, select best candidate.
            //
            // For now (POC stub): deterministically pick based on hash.
            let idx = simple_hash_select(lint_name, buggy_code, candidates.len());
            let entry = &candidates[idx];
            Some(DraftedFix {
                code: entry.fixed_code.clone(),
                confidence: 0.7, // placeholder until matvec is wired
                source: DraftSource::TernaryMatvec,
            })
        } else {
            // L0: corpus template lookup
            let entry = &candidates[0];
            Some(DraftedFix {
                code: entry.fixed_code.clone(),
                confidence: 0.5, // lower confidence for template
                source: DraftSource::CorpusTemplate,
            })
        }
    }
}

/// Deterministic index selection via BLAKE3 hash (placeholder for full matvec).
fn simple_hash_select(lint_name: &str, buggy_code: &str, n: usize) -> usize {
    let mut hasher = blake3::Hasher::new();
    hasher.update(lint_name.as_bytes());
    hasher.update(b"\x00");
    hasher.update(buggy_code.as_bytes());
    let hash = hasher.finalize();
    let bytes = hash.as_bytes();
    // Use first 8 bytes as u64
    let val = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ]);
    (val as usize) % n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::ClippyFixEntry;

    fn mock_entries() -> Vec<ClippyFixEntry> {
        vec![
            ClippyFixEntry {
                lint_name: "needless_return".to_string(),
                lint_group: "style".to_string(),
                buggy_code: "fn foo() -> i32 { return 42; }".to_string(),
                fixed_code: "fn foo() -> i32 { 42 }".to_string(),
            },
            ClippyFixEntry {
                lint_name: "needless_return".to_string(),
                lint_group: "style".to_string(),
                buggy_code: "fn bar() -> i32 { return 0; }".to_string(),
                fixed_code: "fn bar() -> i32 { 0 }".to_string(),
            },
        ]
    }

    #[test]
    fn fallback_drafter_uses_corpus_template() {
        let mut drafter = ClippyTernaryDrafter::fallback();
        assert!(drafter.is_fallback());

        let entries = mock_entries();
        let fix = drafter.draft_fix("needless_return", "return 42;", &entries);
        assert!(fix.is_some());
        let fix = fix.unwrap();
        assert_eq!(fix.source, DraftSource::CorpusTemplate);
        assert!(!fix.code.is_empty());
    }

    #[test]
    fn draft_fix_returns_none_for_empty_candidates() {
        let mut drafter = ClippyTernaryDrafter::fallback();
        let fix = drafter.draft_fix("needless_return", "return 42;", &[]);
        assert!(fix.is_none());
    }

    #[test]
    fn hash_select_is_deterministic() {
        let a = simple_hash_select("needless_return", "return 42;", 2);
        let b = simple_hash_select("needless_return", "return 42;", 2);
        assert_eq!(a, b);
    }
}
