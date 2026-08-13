//! Modelless ternary drafter for code fixes.
//!
//! Mirrors the `TernaryDraftModel` pattern from
//! `riir-games-quest/quest_grammar/plasma_draft.rs`. Uses
//! `katgpt_core::TernaryWeights` for multiplication-free SIMD matvec with
//! zero-alloc hot path.
//!
//! # Hot-path contract
//!
//! `draft_fix` reuses two pre-allocated scratch buffers (`input_scratch`,
//! `output_scratch`) sized at `with_weights` time. This makes the SIMD matvec
//! hot path **zero-alloc** — the same pattern as the quest grammar drafter.
//!
//! # Domain-agnostic (Plan 001 Phase 1)
//!
//! Renamed from `ClippyTernaryDrafter` → `TernaryDrafter`. The `draft_fix`
//! method is now generic over `E: HealEntry` — it reads `E::fixed_code()` from
//! each candidate, so it works with any domain's entry type. The old
//! `ClippyTernaryDrafter` name is preserved as a type alias via re-export.

use crate::domain::HealEntry;

/// The modelless ternary drafter for code fixes.
///
/// When ternary weight files (`.bits`) are available, uses them for fast
/// draft generation via multiplication-free SIMD matvec from katgpt-core.
/// Otherwise, falls back to corpus-based template lookup.
#[derive(Debug, Clone)]
pub struct TernaryDrafter {
    /// Frozen ternary weights for the drafter (L1 hot path).
    /// `None` = fallback mode (use corpus lookup).
    weights: Option<katgpt_core::TernaryWeights>,
    /// Pre-allocated input buffer for the SIMD matvec hot path.
    /// Sized to `weights.cols`. Reused across `draft_fix` calls.
    input_scratch: Vec<f32>,
    /// Pre-allocated output buffer for the SIMD matvec hot path.
    /// Sized to `weights.rows`. Reused across `draft_fix` calls.
    output_scratch: Vec<f32>,
    /// Whether to enforce the identity guard (Stage 1 decline path).
    /// When true, the drafter returns `None` instead of emitting a fix whose
    /// identifier set is disjoint from the buggy code (Issue 010).
    identity_guard: bool,
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

impl TernaryDrafter {
    /// Create in fallback mode (no ternary weights — corpus lookup only).
    #[must_use]
    pub fn fallback() -> Self {
        Self {
            weights: None,
            input_scratch: Vec::new(),
            output_scratch: Vec::new(),
            identity_guard: true, // Issue 010: decline-by-default (Stage 2 replaces)
        }
    }

    /// Create with ternary weights (L1 hot path enabled).
    ///
    /// Pre-allocates the SIMD matvec scratch buffers to `(weights.cols,
    /// weights.rows)` so the hot path is zero-alloc.
    #[must_use]
    pub fn with_weights(weights: katgpt_core::TernaryWeights) -> Self {
        let cols = weights.cols;
        let rows = weights.rows;
        Self {
            weights: Some(weights),
            input_scratch: vec![0.0f32; cols],
            output_scratch: vec![0.0f32; rows],
            identity_guard: true, // Issue 010: decline-by-default (Stage 2 replaces)
        }
    }

    /// Enable or disable the identity guard (Stage 1 decline path).
    ///
    /// When enabled (default), the drafter returns `None` if the emitted fix
    /// shares no identifiers with the buggy code. This is the Issue 010 fix
    /// direction: turn the silent-miss into an honest decline so the caller
    /// routes to Stage 2.
    pub fn set_identity_guard(&mut self, enabled: bool) {
        self.identity_guard = enabled;
    }

    /// Whether the identity guard is currently enabled.
    #[must_use]
    #[inline]
    pub fn identity_guard_enabled(&self) -> bool {
        self.identity_guard
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
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        self.weights.is_none()
    }

    /// Current capacity of the input scratch buffer (for G4 alloc-free checks).
    #[inline]
    #[must_use]
    pub fn input_scratch_capacity(&self) -> usize {
        self.input_scratch.capacity()
    }

    /// Current capacity of the output scratch buffer (for G4 alloc-free checks).
    #[inline]
    #[must_use]
    pub fn output_scratch_capacity(&self) -> usize {
        self.output_scratch.capacity()
    }

    /// Draft a fix for the given rule id and buggy code.
    ///
    /// Generic over `E: HealEntry` — works with any domain's corpus entry type.
    /// The drafter reads `E::fixed_code()` from each candidate to select + emit
    /// the proposed fix.
    ///
    /// - **Fallback mode**: returns the first corpus entry for this rule.
    /// - **With weights (L1)**: runs the ternary SIMD matvec over a
    ///   BLAKE3-deterministic embedding of (`rule_id`, `buggy_code`) to select
    ///   the best fix candidate. Zero-alloc hot path (reuses scratch buffers).
    ///
    /// The actual fix validation happens in the pruner (L2), NOT here.
    /// The drafter proposes; the pruner disposes.
    ///
    /// **Identity guard (Issue 010):** if `identity_guard_enabled` is true (the
    /// default), the drafter returns `None` when the emitted fix shares no
    /// identifiers with the buggy code. This is the Stage 1 decline path — the
    /// caller routes to Stage 2 when this returns `None`.
    pub fn draft_fix<E: HealEntry>(
        &mut self,
        rule_id: &str,
        buggy_code: &str,
        candidates: &[E],
    ) -> Option<DraftedFix> {
        if candidates.is_empty() {
            return None;
        }

        let raw = self.draft_fix_raw(rule_id, buggy_code, candidates);

        // Identity guard (Issue 010): if the emitted fix shares no identifiers
        // with the buggy code, the Stage 1 retriever is producing a confidently-
        // wrong answer. Decline — let Stage 2 (the adaptive drafter / tuning
        // loop) take over.
        if self.identity_guard && !shares_identifier(buggy_code, &raw.code) {
            return None;
        }

        Some(raw)
    }

    /// Inner drafter — produces a fix from corpus/weights WITHOUT applying
    /// the identity guard. The guard wraps this in [`draft_fix`]. Kept private
    /// + separate so the guard logic stays testable in isolation.
    ///
    /// Returns `DraftedFix` directly (not `Option`) — both branches always
    /// produce a fix. The caller (`draft_fix`) handles the empty-candidates +
    /// identity-guard `None` cases.
    fn draft_fix_raw<E: HealEntry>(
        &mut self,
        rule_id: &str,
        buggy_code: &str,
        candidates: &[E],
    ) -> DraftedFix {
        // (Path split from original `draft_fix` body — original behavior preserved.)

        if let Some(tw) = self.weights.as_ref() {
            // L1: ternary SIMD matvec path (zero-alloc, reuses scratch buffers).
            // 1. BLAKE3-hash (rule_id, buggy_code) → fill input_scratch with
            //    normalized hash bytes (cycling for cols > 32). Hash bytes give
            //    better signal diversity than raw code bytes (ASCII-biased).
            // 2. simd_ternary_matvec → output_scratch.
            // 3. Sigmoid(mean(output)) → confidence (feature-dependent, not hardcoded).
            // 4. FNV-1a hash of output scores → candidate index (same proven
            //    pattern as quest_grammar::plasma_draft::generate_with_weights).
            let input = &mut self.input_scratch;
            debug_assert_eq!(
                input.len(),
                tw.cols,
                "input_scratch len {} != weights.cols {}",
                input.len(),
                tw.cols
            );

            // BLAKE3 hash of (rule_id \0 buggy_code) -> 32 bytes of high-entropy seed.
            let mut hasher = blake3::Hasher::new();
            hasher.update(rule_id.as_bytes());
            hasher.update(b"\x00");
            hasher.update(buggy_code.as_bytes());
            let hash = hasher.finalize();
            let hb = hash.as_bytes(); // 32 bytes

            // Cycle the 32 hash bytes through input_scratch (cols may be > 32).
            // Normalize byte [0, 255] → f32 [-0.5, 0.5] for a zero-centered input.
            for (i, slot) in input.iter_mut().enumerate() {
                *slot = f32::from(hb[i % 32]) / 255.0 - 0.5;
            }

            let output = &mut self.output_scratch;
            debug_assert_eq!(
                output.len(),
                tw.rows,
                "output_scratch len {} != weights.rows {}",
                output.len(),
                tw.rows
            );
            katgpt_core::simd_ternary_matvec(tw, input, output);

            // Confidence: sigmoid(mean(output)) → scalar in (0, 1).
            #[allow(clippy::cast_precision_loss)] // output.len() is small (<256 rows)
            let mean = output.iter().sum::<f32>() / output.len() as f32;
            let confidence = sigmoid(mean);

            // Candidate selection: FNV-1a fold over output scores → index.
            // Same deterministic reduction as quest_grammar's plasma_draft.
            let output_hash = output
                .iter()
                .enumerate()
                .fold(0xcbf2_9ce4_8422_2325_u64, |h, (i, &v)| {
                    let bits = v.to_bits();
                    h ^ (u64::from(bits).wrapping_add((i as u64).wrapping_mul(0x0100_0000_01b3)))
                });
            #[allow(clippy::cast_possible_truncation)] // intentional: index via modulo
            let idx = (output_hash % candidates.len() as u64) as usize;

            let entry = &candidates[idx];
            DraftedFix {
                code: entry.fixed_code().to_string(),
                confidence,
                source: DraftSource::TernaryMatvec,
            }
        } else {
            // L0: corpus template lookup
            let entry = &candidates[0];
            DraftedFix {
                code: entry.fixed_code().to_string(),
                confidence: 0.5, // lower confidence for template
                source: DraftSource::CorpusTemplate,
            }
        }
    }
}

/// Logistic sigmoid (never softmax per AGENTS.md rule).
#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Whether `buggy` + `fix` share at least one Rust identifier token.
///
/// Used by the identity guard (Issue 010): if a Stage 1 emission shares no
/// identifiers with the input, the retriever is producing a confidently-wrong
/// function (a different code's fix). Declining is strictly better than
/// emitting the wrong answer.
///
/// An identifier here is a non-keyword alphanumeric Rust token longer than 1
/// char. We exclude common keywords (`fn`, `return`, `let`, etc.) so the
/// guard is not fooled by structural similarity alone.
#[must_use]
pub(crate) fn shares_identifier(buggy: &str, fix: &str) -> bool {
    let buggy_idents = identifier_set(buggy);
    if buggy_idents.is_empty() {
        // Defensive: if the input has no identifiers at all, the guard can't
        // fire. Permit the emission (the pruner is the next safety net).
        return true;
    }
    identifier_set(fix).intersection(&buggy_idents).next().is_some()
}

/// Collect the set of non-keyword Rust identifiers in `s`.
fn identifier_set(s: &str) -> std::collections::HashSet<&str> {
    use std::collections::HashSet;
    let mut out = HashSet::new();
    for token in s.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if token.len() < 2 {
            continue;
        }
        // Skip if it's purely numeric.
        if token.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        // Skip common Rust keywords.
        if is_rust_keyword(token) {
            continue;
        }
        out.insert(token);
    }
    out
}

/// Common Rust keywords we exclude from identifier-overlap checks.
fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "fn" | "let" | "mut" | "return" | "if" | "else" | "match"
        | "for" | "while" | "loop" | "in" | "ref" | "move" | "struct"
        | "enum" | "impl" | "trait" | "pub" | "self" | "Self"
        | "crate" | "use" | "mod" | "as" | "where" | "type"
        | "const" | "static" | "unsafe" | "true" | "false" | "i8"
        | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
        | "usize" | "isize" | "f32" | "f64" | "bool" | "char"
        | "str" | "String" | "Vec" | "Option" | "Some" | "None"
        | "Result" | "Ok" | "Err"
    )
}

#[cfg(all(test, feature = "clippy_lints"))]
mod tests {
    use super::*;
    use crate::domains::clippy_lints::ClippyFixEntry;

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
        let mut drafter = TernaryDrafter::fallback();
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
        let mut drafter = TernaryDrafter::fallback();
        let empty: [ClippyFixEntry; 0] = [];
        let fix = drafter.draft_fix("needless_return", "return 42;", &empty);
        assert!(fix.is_none());
    }

    // ── L1 ternary matvec path tests ──────────────────────────────────────

    /// Build small deterministic ternary weights for testing the L1 path.
    /// Uses `quantize_from_f32` — the same constructor as `quest_grammar`'s tests.
    fn mock_weights() -> katgpt_core::TernaryWeights {
        // 4 rows × 8 cols — small enough to reason about, large enough to exercise matvec.
        let weights: &[f32] = &[
            1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 1.0, -1.0,
            -1.0, 1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0,
            0.0, 0.0, 1.0, -1.0, 1.0, -1.0, 1.0, 0.0,
            1.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 1.0,
        ];
        katgpt_core::TernaryWeights::quantize_from_f32(weights, 4, 8)
    }

    #[test]
    fn l1_drafter_runs_matvec() {
        let mut drafter = TernaryDrafter::with_weights(mock_weights());
        assert!(!drafter.is_fallback());

        let entries = mock_entries();
        let fix = drafter.draft_fix("needless_return", "return 42;", &entries);
        assert!(fix.is_some());
        let fix = fix.unwrap();
        // Must claim TernaryMatvec (not CorpusTemplate) — it actually ran the matvec.
        assert_eq!(fix.source, DraftSource::TernaryMatvec);
        assert!(!fix.code.is_empty());
        // Confidence is now derived from sigmoid(mean(output)) — must be in (0, 1).
        assert!(fix.confidence > 0.0 && fix.confidence < 1.0);
    }

    #[test]
    fn l1_drafter_is_deterministic() {
        // G1: same (lint, code, weights, candidates) → identical output.
        let entries = mock_entries();
        let mut drafter = TernaryDrafter::with_weights(mock_weights());

        let fix1 = drafter.draft_fix("needless_return", "return 42;", &entries).unwrap();
        let fix2 = drafter.draft_fix("needless_return", "return 42;", &entries).unwrap();

        assert_eq!(fix1.code, fix2.code);
        // G1 determinism: bit-identical confidence (not approximate — the matvec
        // is deterministic, so the same input MUST produce the same f32 bits).
        assert_eq!(fix1.confidence.to_bits(), fix2.confidence.to_bits());
        assert_eq!(fix1.source, fix2.source);
    }

    #[test]
    fn l1_different_input_can_produce_different_output() {
        // Different (lint_name, buggy_code) → different BLAKE3 hash → different
        // matvec input → potentially different candidate selection. We only
        // check that the path processes both without panic; the exact selection
        // is deterministic but weight-dependent.
        let entries = mock_entries();
        let mut drafter = TernaryDrafter::with_weights(mock_weights());

        let _fix1 = drafter.draft_fix("needless_return", "return 42;", &entries).unwrap();
        let _fix2 = drafter.draft_fix("redundant_closure", "|x| f(x)", &entries).unwrap();
        // Both should produce valid fixes — the point is no panic + valid source.
    }

    #[test]
    fn l1_confidence_in_valid_range() {
        // Confidence = sigmoid(mean(output)) must always be in (0, 1),
        // never NaN, never exactly 0 or 1 (sigmoid asymptote).
        let entries = mock_entries();
        let mut drafter = TernaryDrafter::with_weights(mock_weights());

        for lint in &["needless_return", "redundant_closure", "unused_var"] {
            for code in &["return 42;", "let x = 1;", "|x| f(x)"] {
                let fix = drafter.draft_fix(lint, code, &entries).unwrap();
                assert!(fix.confidence > 0.0, "confidence must be > 0 for {lint}/{code}");
                assert!(fix.confidence < 1.0, "confidence must be < 1 for {lint}/{code}");
                assert!(!fix.confidence.is_nan());
            }
        }
    }

    #[test]
    fn load_weights_resizes_scratch() {
        let mut drafter = TernaryDrafter::with_weights(mock_weights());
        // Initial scratch: input=8, output=4
        assert_eq!(drafter.input_scratch.len(), 8);
        assert_eq!(drafter.output_scratch.len(), 4);

        // Swap to bigger weights
        let big = katgpt_core::TernaryWeights::quantize_from_f32(&[1.0; 32], 2, 16);
        drafter.load_weights(big);
        assert_eq!(drafter.input_scratch.len(), 16);
        assert_eq!(drafter.output_scratch.len(), 2);

        // Draft still works with new dims
        let entries = mock_entries();
        let fix = drafter.draft_fix("needless_return", "return 42;", &entries).unwrap();
        assert_eq!(fix.source, DraftSource::TernaryMatvec);
    }
}
