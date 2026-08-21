//! The playbook seam (Plan 019 Phase 1) — per-diagnostic candidate
//! generators for the `--fix-compile` driver.
//!
//! A playbook receives one [`RustcErrorDiag`] plus the CURRENT source of
//! the file its primary span points at, and returns zero or more
//! [`FixCandidate`]s. The driver owns everything mechanical: ordering,
//! byte-splicing, rechecking, keeping, and reverting. A playbook never
//! touches disk and never runs cargo — that division is what makes the
//! loop's invariants (one candidate per iteration, exact revert) hold.
//!
//! ## Granularity
//!
//! Deliberately PER-DIAGNOSTIC, not per-crate: the driver's value over
//! `cargo fix` is candidate arbitration (try best → recheck → revert →
//! try next), which needs editable units smaller than "the whole crate".
//! The G-floor harness's whole-crate [`super::floor::CompileFixer`]
//! (Plan 019 Phase 6) adapts the driver into that granularity by running
//! it over a crate dir — one adapter, not two shapes of playbook.
//!
//! ## Phase status
//!
//! Phase 1 ships the SEAM only — [`Playbooks::empty()`]. The registry
//! iterates empty, so `--fix-compile` today reports every coded error as
//! a miss (`no playbook matched`). The E0597 family lands in Phase 2
//! (`e0597.rs`), the rest of borrowck in Phase 3.

use std::path::{Path, PathBuf};

use super::diag::RustcErrorDiag;

/// One proposed edit: replace `source[byte_span]` with `replacement`.
///
/// The span is against the source text the playbook was handed (the
/// driver re-proposes every iteration, so spans never go stale across
/// kept fixes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixCandidate {
    /// Strategy identity for the report + the Phase 4 memory key
    /// (`(error_code, strategy_id)` — the `EvolveRecorder` bucketing).
    pub strategy_id: String,
    /// The `E….` code this candidate addresses.
    pub code: String,
    pub file: PathBuf,
    /// 1-based line of the primary span (reporting).
    pub line: usize,
    /// Byte range to replace in the CURRENT file content.
    pub byte_span: (usize, usize),
    pub replacement: String,
}

/// A playbook: proposes candidates for one diagnostic against the current
/// source of the file it points at.
pub trait CompileProposer {
    /// Stable identity for reports (`e0597-clone`, `rustc-suggestion`...).
    fn name(&self) -> &'static str;
    /// Propose candidates. Empty = this playbook has no move here (the
    /// driver records the diagnostic as a miss if NO playbook proposes).
    fn propose(&self, diag: &RustcErrorDiag, file: &Path, source: &str) -> Vec<FixCandidate>;
}

/// The registry the driver iterates. Order is registration order; the
/// driver's candidate ordering is independent of registry position.
pub struct Playbooks {
    entries: Vec<Box<dyn CompileProposer>>,
}

impl Default for Playbooks {
    fn default() -> Self {
        Self::empty()
    }
}

impl Playbooks {
    /// Phase 1 ships empty — templates land Phase 2 (`e0597.rs`) and
    /// Phase 3 (the rest of borrowck).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a playbook (builder).
    #[must_use]
    pub fn with(mut self, p: Box<dyn CompileProposer>) -> Self {
        self.entries.push(p);
        self
    }

    /// Number of registered playbooks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the registry empty?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Propose for one diagnostic by iterating every registered playbook.
    #[must_use]
    pub fn propose(&self, diag: &RustcErrorDiag, file: &Path, source: &str) -> Vec<FixCandidate> {
        let mut out = Vec::new();
        for p in &self.entries {
            out.extend(p.propose(diag, file, source));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::rustc_errors::diag::SuggestionSpan;

    struct FixedProposer(&'static str);

    impl CompileProposer for FixedProposer {
        fn name(&self) -> &'static str {
            self.0
        }

        fn propose(&self, diag: &RustcErrorDiag, _file: &Path, _source: &str) -> Vec<FixCandidate> {
            vec![FixCandidate {
                strategy_id: self.0.to_string(),
                code: diag.code.clone(),
                file: PathBuf::from("src/lib.rs"),
                line: 1,
                byte_span: (0, 0),
                replacement: String::new(),
            }]
        }
    }

    fn diag() -> RustcErrorDiag {
        RustcErrorDiag {
            code: "E0597".to_string(),
            message: String::new(),
            primary: Some(("src/lib.rs".to_string(), 1)),
            suggestions: vec![SuggestionSpan {
                file: "src/lib.rs".to_string(),
                line_start: 1,
                byte_start: None,
                byte_end: None,
                replacement: String::new(),
                applicability: crate::domains::rustc_errors::diag::Applicability::Unknown,
                message: String::new(),
            }],
            rendered: None,
        }
    }

    #[test]
    fn empty_registry_proposes_nothing() {
        let p = Playbooks::empty();
        assert!(p.is_empty());
        assert!(p.propose(&diag(), Path::new("src/lib.rs"), "").is_empty());
    }

    #[test]
    fn registry_iterates_every_playbook() {
        let p = Playbooks::empty()
            .with(Box::new(FixedProposer("a")))
            .with(Box::new(FixedProposer("b")));
        assert_eq!(p.len(), 2);
        let out = p.propose(&diag(), Path::new("src/lib.rs"), "");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].strategy_id, "a");
        assert_eq!(out[1].strategy_id, "b");
    }
}
