//! Clippy healing corpus — BLAKE3-committed freeze/thaw of (lint, buggy, fixed) triples.
//!
//! Mirrors the `SealQuestCorpus` pattern from `riir-games-quest/quest_grammar`
//! but specialized for clippy lint fixes. Each entry is a named clippy lint
//! with its buggy code and the known-good fix.

use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// A single clippy lint fix example.
///
/// Domain classification (per AGENTS.md sync boundary rule):
/// - `lint_name` + `lint_group` = physical (categorical, deterministic)
/// - `buggy_code` + `fixed_code` = physical (raw bytes, bit-identical for replay)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippyFixEntry {
    /// The clippy lint name (e.g., "`needless_return`", "`redundant_closure`").
    pub lint_name: String,
    /// The lint group (style, correctness, suspicious, complexity, perf).
    pub lint_group: String,
    /// The buggy code that triggers the lint.
    pub buggy_code: String,
    /// The fixed code that resolves the lint.
    pub fixed_code: String,
}

/// BLAKE3-committed corpus of clippy lint fixes.
///
/// The corpus is the foundation (L0) of the clippy-healing pipeline.
/// Each lint is distilled into a named rule with a bounded fix-space.
/// The ternary drafter (L1) consumes these as frozen direction vectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippyCorpus {
    /// All fix entries, keyed by `lint_name` for dedup.
    pub entries: Vec<ClippyFixEntry>,
    /// BLAKE3 hash of the serialized entries (tamper-evidence).
    pub commitment: [u8; 32],
}

impl ClippyCorpus {
    /// Build a corpus from fix entries, computing the BLAKE3 commitment.
    #[must_use]
    pub fn new(mut entries: Vec<ClippyFixEntry>) -> Self {
        // Sort by lint_name for deterministic commitment
        entries.sort_by(|a, b| a.lint_name.cmp(&b.lint_name));

        let commitment = Self::compute_commitment(&entries);
        Self { entries, commitment }
    }

    /// Compute the BLAKE3 hash of the serialized entries.
    fn compute_commitment(entries: &[ClippyFixEntry]) -> [u8; 32] {
        let mut hasher = Hasher::new();
        for entry in entries {
            hasher.update(entry.lint_name.as_bytes());
            hasher.update(b"\x00");
            hasher.update(entry.lint_group.as_bytes());
            hasher.update(b"\x00");
            hasher.update(entry.buggy_code.as_bytes());
            hasher.update(b"\x00");
            hasher.update(entry.fixed_code.as_bytes());
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
    /// Returns `serde_json::Error` if serialization fails (e.g., the corpus
    /// contains non-serializable data — should not happen with `String` fields).
    pub fn freeze(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Thaw (deserialize) the corpus from bytes, verifying the commitment.
    ///
    /// # Errors
    ///
    /// Returns `serde_json::Error` if deserialization fails (corrupt JSON,
    /// schema mismatch, etc.). A commitment mismatch does NOT return an
    /// error — the corpus is returned so the caller can decide how to handle
    /// tamper detection.
    pub fn thaw(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        let corpus: Self = serde_json::from_slice(bytes)?;
        if !corpus.verify() {
            // Tamper detected — but we still return the data so the caller
            // can decide how to handle it. The commitment is advisory, not
            // a hard gate (the pruner's is_patch_valid is ground truth).
        }
        Ok(corpus)
    }

    /// Look up fix entries by lint name.
    pub fn lookup<'a>(&'a self, lint_name: &'a str) -> impl Iterator<Item = &'a ClippyFixEntry> + 'a {
        self.entries.iter().filter(move |e| e.lint_name == lint_name)
    }

    /// All unique lint names in the corpus.
    #[must_use]
    pub fn lint_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.iter().map(|e| e.lint_name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_round_trip() {
        let entries = vec![ClippyFixEntry {
            lint_name: "needless_return".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn foo() -> i32 { return 42; }".to_string(),
            fixed_code: "fn foo() -> i32 { 42 }".to_string(),
        }];
        let corpus = ClippyCorpus::new(entries);
        assert!(corpus.verify());

        let frozen = corpus.freeze().unwrap();
        let thawed = ClippyCorpus::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert!(thawed.verify());
    }

    #[test]
    fn tamper_detected() {
        let entries = vec![ClippyFixEntry {
            lint_name: "needless_return".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn foo() -> i32 { return 42; }".to_string(),
            fixed_code: "fn foo() -> i32 { 42 }".to_string(),
        }];
        let corpus = ClippyCorpus::new(entries);
        let mut tampered = corpus.clone();
        tampered.entries[0].fixed_code = "fn foo() -> i32 { 99 }".to_string();
        assert!(!tampered.verify()); // commitment mismatch
    }

    #[test]
    fn lookup_by_lint_name() {
        let corpus = ClippyCorpus::new(vec![
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
        assert_eq!(corpus.lint_names().len(), 2);
    }
}
