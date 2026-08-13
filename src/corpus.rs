//! Generic healing corpus — BLAKE3-committed freeze/thaw of domain entries.
//!
//! Generalized in Plan 001 Phase 1 from the clippy-specific `ClippyCorpus` to
//! `Corpus<D: HealDomain>`. The corpus stores domain-typed entries + a BLAKE3
//! commitment. The commitment is computed from the serialized entries (via
//! `serde_json`), which captures ALL fields — not just the `(rule_id, buggy,
//! fixed)` triple.
//!
//! Mirrors the `SealQuestCorpus` pattern from `riir-games-quest/quest_grammar`
//! but is now domain-generic.

use std::marker::PhantomData;

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

/// BLAKE3-committed corpus of healing entries.
///
/// Each domain's entries are stored as `D::Entry`. The commitment is the
/// BLAKE3 hash of the serialized entries (sorted by `rule_id` for determinism).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corpus<D: HealDomain> {
    /// All fix entries for this domain.
    pub entries: Vec<D::Entry>,
    /// BLAKE3 hash of the serialized entries (tamper-evidence).
    pub commitment: [u8; 32],
    #[serde(skip)]
    _marker: PhantomData<D>,
}

impl<D: HealDomain> Corpus<D> {
    /// Build a corpus from entries, computing the BLAKE3 commitment.
    ///
    /// The entries are sorted by `rule_id` (via `D::Entry::rule_id()`) for
    /// deterministic commitment — the same entries in any order produce the
    /// same commitment.
    #[must_use]
    pub fn new(mut entries: Vec<D::Entry>) -> Self {
        entries.sort_by(|a, b| a.rule_id().cmp(b.rule_id()));
        let commitment = Self::compute_commitment(&entries);
        Self {
            entries,
            commitment,
            _marker: PhantomData,
        }
    }

    /// Compute the BLAKE3 hash of the serialized entries.
    ///
    /// Each entry is serialized via `serde_json::to_vec` and concatenated with
    /// a `\x01` separator. This captures ALL entry fields (not just the triple)
    /// for maximum tamper-evidence.
    fn compute_commitment(entries: &[D::Entry]) -> [u8; 32] {
        let mut hasher = Hasher::new();
        for entry in entries {
            // Serialize the entry — captures all fields (rule_id, metadata,
            // buggy_code, fixed_code).
            if let Ok(bytes) = serde_json::to_vec(entry) {
                hasher.update(&bytes);
            }
            hasher.update(b"\x01"); // entry separator
        }
        let hash = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(hash.as_bytes());
        out
    }

    /// Verify the corpus commitment (tamper-evidence check).
    #[must_use]
    pub fn verify(&self) -> bool {
        self.commitment == Self::compute_commitment(&self.entries)
    }

    /// Freeze (serialize) the corpus to bytes.
    ///
    /// # Errors
    ///
    /// Returns `serde_json::Error` if serialization fails.
    pub fn freeze(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Thaw (deserialize) the corpus from bytes.
    ///
    /// A commitment mismatch does NOT return an error — the corpus is returned
    /// so the caller can decide how to handle tamper detection. The commitment
    /// is advisory, not a hard gate (the pruner's validation is ground truth).
    ///
    /// # Errors
    ///
    /// Returns `serde_json::Error` if deserialization fails.
    pub fn thaw(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        let corpus: Self = serde_json::from_slice(bytes)?;
        // Tamper detected — but still return the data so the caller can decide.
        // (The commitment is advisory; the pruner is ground truth.)
        Ok(corpus)
    }

    /// All unique rule ids in the corpus, sorted + deduplicated.
    #[must_use]
    pub fn rule_ids(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.iter().map(HealEntry::rule_id).collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

// ── Back-compat: `Corpus<ClippyLintDomain>` gains clippy-specific
//    convenience methods via an inherent impl in `domains/clippy_lints/mod.rs`.
//    The generic `Corpus<D>` itself stays domain-agnostic.

#[cfg(all(test, feature = "clippy_lints"))]
mod tests {
    use super::*;
    use crate::domains::clippy_lints::{ClippyFixEntry, ClippyLintDomain};

    type TestCorpus = Corpus<ClippyLintDomain>;

    fn make_entries() -> Vec<ClippyFixEntry> {
        vec![ClippyFixEntry {
            lint_name: "needless_return".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn foo() -> i32 { return 42; }".to_string(),
            fixed_code: "fn foo() -> i32 { 42 }".to_string(),
        }]
    }

    #[test]
    fn corpus_round_trip() {
        let entries = make_entries();
        let corpus = TestCorpus::new(entries);
        assert!(corpus.verify());

        let frozen = corpus.freeze().unwrap();
        let thawed = TestCorpus::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert!(thawed.verify());
    }

    #[test]
    fn tamper_detected() {
        let entries = make_entries();
        let corpus = TestCorpus::new(entries);
        let mut tampered = corpus.clone();
        tampered.entries[0].fixed_code = "fn foo() -> i32 { 99 }".to_string();
        assert!(!tampered.verify()); // commitment mismatch
    }

    #[test]
    fn lookup_by_rule_id() {
        let corpus = TestCorpus::new(vec![
            ClippyFixEntry {
                lint_name: "needless_return".to_string(),
                lint_group: "style".to_string(),
                buggy_code: "return 1;".to_string(),
                fixed_code: "1".to_string(),
            },
            ClippyFixEntry {
                lint_name: "redundant_closure".to_string(),
                lint_group: "style".to_string(),
                buggy_code: "|x| f(x)".to_string(),
                fixed_code: "f".to_string(),
            },
        ]);
        assert_eq!(corpus.lookup("needless_return").count(), 1);
        assert_eq!(corpus.lookup("nonexistent").count(), 0);
        assert_eq!(corpus.rule_ids().len(), 2);
    }
}
