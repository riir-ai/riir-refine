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
//! Phase 1 shipped the seam; Phase 2 shipped the E0597 family ([`e0597`]);
//! **Phase 3 ships [`borrowck`]** — the rest of the uncontested territory
//! (E0502, E0499) plus the one measured contested opening (E0505's
//! aliasing-divergence, where `cargo fix` rewrites the borrow site and we
//! clone at the move site, matching the known inverse). E0596/E0382/E0507/
//! E0373 ship NO template by measurement (Bench 041: `cargo fix` scores
//! 100% matched there — no headroom).

pub mod borrowck;
pub mod e0597;

pub use borrowck::family as borrowck_family;
pub use e0597::family;

use std::path::{Path, PathBuf};

use super::diag::{LabeledSpan, RustcErrorDiag};

/// A labeled span in the same file as the diagnostic's primary span —
/// the shared anchor lookup every playbook family keys on (the oracle's
/// own location knowledge, never guessed).
pub(crate) fn anchor(diag: &RustcErrorDiag, pred: impl Fn(&str) -> bool) -> Option<&LabeledSpan> {
    let (pf, _) = diag.primary.as_ref()?;
    diag.labeled_spans
        .iter()
        .find(|ls| &ls.file == pf && pred(&ls.label))
}

/// Label "binding `x` declared here" (or any label whose first backtick
/// pair is a plain ident) → the binding name `x`.
pub(crate) fn binding_name(label: &str) -> Option<&str> {
    let start = label.find('`')? + 1;
    let end = start + label[start..].find('`')?;
    let name = &label[start..end];
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(name)
}

/// Build a single-edit candidate (the common shape).
pub(crate) fn cand(
    diag: &RustcErrorDiag,
    file: &Path,
    line: usize,
    byte_span: (usize, usize),
    replacement: String,
    strategy_id: &str,
) -> FixCandidate {
    FixCandidate {
        strategy_id: strategy_id.to_string(),
        code: diag.code.clone(),
        file: file.to_path_buf(),
        line,
        byte_span,
        replacement,
        shape: String::new(),
        semantic_note: None,
        extra_edits: Vec::new(),
    }
}

/// One atomic edit of a [`FixCandidate`]: replace `source[byte_span]`
/// with `replacement`.
///
/// A candidate may carry SEVERAL edits ([`FixCandidate::extra_edits`]) —
/// they are applied as ONE unit and reverted as ONE unit. The E0502
/// deref-removal is the motivating case: deleting the `&` at the binding
/// without also deleting the `*` at the uses trades E0502 for E0614
/// (no progress → revert), so the pair must land atomically or not at
/// all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanEdit {
    /// Byte range to replace in the CURRENT file content.
    pub byte_span: (usize, usize),
    pub replacement: String,
}

/// One proposed edit: replace `source[byte_span]` with `replacement`,
/// plus optional further edits in the SAME file applied atomically with
/// it.
///
/// The primary span is against the source text the playbook was handed
/// (the driver re-proposes every iteration, so spans never go stale across
/// kept fixes). All edits of one candidate must be non-overlapping; the
/// driver applies them in one pass and reverts all-or-nothing.
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
    /// Trigger text at `byte_span` (filled by the DRIVER at proposal
    /// time — the T4.2 shape-conditional ordering retrieves neighbors by
    /// it). Playbooks leave it empty; the driver overwrites.
    pub shape: String,
    /// The T6.10 semantic annotation: `Some` when this strategy's fix is in
    /// the semantics-changing class (clone-instead-of-borrow, drop-order
    /// change, ownership transfer, lifetime-contract loosening). The
    /// driver surfaces it on every record so the diff NAMES the change —
    /// "compiles + tests pass" is not "means the same thing".
    pub semantic_note: Option<&'static str>,
    /// Further edits applied atomically with the primary one (same file,
    /// non-overlapping with it and each other). Empty for the common
    /// single-edit candidate.
    pub extra_edits: Vec<SpanEdit>,
}

impl FixCandidate {
    /// Total replaced bytes across all edits — the min-delta ordering key
    /// (shorter total wins).
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.replacement.len()
            + self
                .extra_edits
                .iter()
                .map(|e| e.replacement.len())
                .sum::<usize>()
    }

    /// Attach the T6.10 semantic annotation (builder form — playbook
    /// templates whose fixes sit in the semantics-changing class name the
    /// change instead of leaving it silent).
    #[must_use]
    pub fn with_note(mut self, note: &'static str) -> Self {
        self.semantic_note = Some(note);
        self
    }
}

/// A playbook: proposes candidates for one diagnostic against the current
/// source of the file it points at.
pub trait CompileProposer {
    /// Stable identity for reports (`e0597-clone`, `rustc-suggestion`...).
    fn name(&self) -> &'static str;
    /// The single `E….` code this playbook addresses — the propose
    /// guard's source of truth AND the conversion-ledger inventory key
    /// (Plan 091 T1.3). The ledger derives from this, never from name
    /// prefixes.
    fn code(&self) -> &'static str;
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

    /// The production registry (Phase 2's E0597 family + Phase 3's borrowck
    /// family — E0502/E0499/E0505). What `--fix-compile` runs with.
    #[must_use]
    pub fn production() -> Self {
        let mut pb = self::e0597::family();
        for p in self::borrowck::family().into_entries() {
            pb = pb.with(p);
        }
        pb
    }

    /// The registered playbooks, as boxed entries (used by
    /// [`Playbooks::production`] to compose families).
    #[must_use]
    pub fn into_entries(self) -> Vec<Box<dyn CompileProposer>> {
        self.entries
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

        fn code(&self) -> &'static str {
            "" // test helper — never in production(); proposes for any code
        }

        fn propose(&self, diag: &RustcErrorDiag, _file: &Path, _source: &str) -> Vec<FixCandidate> {
            vec![FixCandidate {
                strategy_id: self.0.to_string(),
                code: diag.code.clone(),
                file: PathBuf::from("src/lib.rs"),
                line: 1,
                byte_span: (0, 0),
                replacement: String::new(),
                shape: String::new(),
                semantic_note: None,
                extra_edits: Vec::new(),
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
            labeled_spans: Vec::new(),
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
