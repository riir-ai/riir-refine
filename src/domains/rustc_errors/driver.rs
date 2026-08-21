//! The `--fix-compile` oracle driver (Plan 019 Phase 1, T1.2/T1.4) — the
//! bounded repair loop:
//!
//! ```text
//! check → parse+dedupe → propose per error (playbooks) → order
//!       → apply best → recheck → keep | revert → repeat ≤ max_iters
//! ```
//!
//! ## Keep/revert semantics (T1.4)
//!
//! A candidate is **kept** iff the package's coded-error count strictly
//! DECREASED after applying it. Everything else — no change, more errors,
//! the error moved elsewhere — reverts the exact original bytes at the
//! span and marks the candidate tried. The next iteration re-proposes
//! against the fresh diagnostics + fresh source, so byte spans never go
//! stale and the next-best untried candidate is picked. This is
//! deliberately stricter than "compiles": a fix that trades one error for
//! another (the `Box::leak` shape — compiles, wrong) must not survive
//! arbitration.
//!
//! ## Loop bounds
//!
//! `max_iters` (default 5) bounds the loop. The `tried` set bounds it
//! further: when every candidate for the current error set has been tried
//! and reverted, the loop stops (`NoMoreCandidates`) — the misses are the
//! authoring queue for the next playbook templates (Phase 4 T4.4).
//!
//! ## Write semantics
//!
//! Review mode (no `--write`) NEVER writes: one check, one proposal pass,
//! the plan printed from [`FixCompileReport::planned`]. Write mode runs
//! the loop and leaves on disk exactly the kept fixes — a reverted file
//! is byte-identical to its pre-run content (the `--fix --verify`
//! unconditional-write trap does not carry over; Plan 019's founding
//! requirement).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::compile_oracle::{
    owning_package_root, resolve_reported_path, workspace_root, CheckRunner,
};

use super::diag::{dedupe_by_site, parse_error_diags, RustcErrorDiag};
use super::playbooks::{FixCandidate, Playbooks, SpanEdit};

/// One applied-and-kept fix, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedRecord {
    pub file: PathBuf,
    pub line: usize,
    pub code: String,
    pub strategy_id: String,
    /// Coded-error count before → after this fix.
    pub errors: (usize, usize),
}

/// One tried-and-reverted candidate, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevertedRecord {
    pub file: PathBuf,
    pub line: usize,
    pub code: String,
    pub strategy_id: String,
    /// Why it reverted.
    pub reason: RevertReason,
}

/// Why a candidate was reverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertReason {
    /// Error count did not decrease (no-op fix, new error, or the error
    /// moved).
    NoProgress,
    /// The file could not be read back or written (I/O).
    Io,
}

impl RevertReason {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::NoProgress => "no progress",
            Self::Io => "i/o",
        }
    }
}

/// A coded error no playbook proposed for — the template-authoring queue
/// (Phase 4 T4.4 dumps these to `<repo>/.heal/misses/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissRecord {
    pub code: String,
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
}

/// A review-mode plan line (no writes happened).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRecord {
    pub file: PathBuf,
    pub line: usize,
    pub code: String,
    pub strategy_id: String,
    /// First line of the replacement, for the printed plan.
    pub replacement_head: String,
}

/// The outcome of a [`CompileDriver::run`].
#[derive(Debug, Clone, Default)]
pub struct FixCompileReport {
    /// Whether cargo ran at all (false: no packages / cargo missing).
    pub ran: bool,
    /// cargo check invocations.
    pub n_checks: usize,
    /// Loop iterations executed (write mode only).
    pub iterations: usize,
    /// Fixes applied and kept.
    pub applied: Vec<AppliedRecord>,
    /// Candidates tried and reverted.
    pub reverted: Vec<RevertedRecord>,
    /// Errors with no candidate at the final proposal pass.
    pub misses: Vec<MissRecord>,
    /// Review-mode plan (empty in write mode).
    pub planned: Vec<PlannedRecord>,
    /// Coded errors at the final check.
    pub remaining: usize,
    /// Zero coded errors at the final check.
    pub clean: bool,
    /// Why the run could not complete (cargo could not run, unreadable
    /// package...).
    pub error: Option<String>,
}

/// Candidate-ordering key: stable (file, line, code) with the shortest
/// TOTAL replacement first — the Phase 1 "min-delta" heuristic (primary +
/// extra edits; a two-byte pair beats a `.clone()` insertion). Phase 4 T4.2
/// swaps in `BetaPosterior` ordering over `(error_code, strategy_id)`
/// memory if it measures a gain; this function is the swappable seam.
fn order_candidates(cands: &mut [FixCandidate]) {
    cands.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.code.cmp(&b.code))
            .then(a.total_len().cmp(&b.total_len()))
            .then(a.strategy_id.cmp(&b.strategy_id))
    });
}

/// Identity of a tried candidate — retries across iterations must not
/// re-apply the same failed move at the same site. Includes every edit
/// (primary + extras) so two strategies differing only in their extra
/// edits stay distinct.
type TriedKey = (PathBuf, String, usize, usize, String, Vec<((usize, usize), String)>);

fn tried_key(c: &FixCandidate) -> TriedKey {
    (
        c.file.clone(),
        c.strategy_id.clone(),
        c.byte_span.0,
        c.byte_span.1,
        c.replacement.clone(),
        c.extra_edits
            .iter()
            .map(|e| (e.byte_span, e.replacement.clone()))
            .collect(),
    )
}

/// The bounded fix-compile loop driver (Plan 019 Phase 1).
pub struct CompileDriver {
    runner: CheckRunner,
    max_iters: usize,
}

impl Default for CompileDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl CompileDriver {
    /// Environment-inheriting runner, `max_iters = 5` (the plan default).
    #[must_use]
    pub fn new() -> Self {
        Self {
            runner: CheckRunner::new(),
            max_iters: 5,
        }
    }

    /// Environment-isolated runner (scaffold crates — see
    /// [`CheckRunner::isolated`]).
    #[must_use]
    pub fn isolated() -> Self {
        Self {
            runner: CheckRunner::isolated(),
            max_iters: 5,
        }
    }

    /// Override the iteration bound (`--max-iters`).
    #[must_use]
    pub fn with_max_iters(mut self, n: usize) -> Self {
        self.max_iters = n;
        self
    }

    /// Extra cargo args for every check (`--verify-args` passthrough).
    #[must_use]
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.runner = self.runner.with_extra_args(args);
        self
    }

    /// Run over the packages owning `paths`.
    ///
    /// `write == false` → review mode: one check + one proposal pass,
    /// nothing written, the plan in [`FixCompileReport::planned`].
    /// `write == true` → the bounded loop; disk holds exactly the kept
    /// fixes on return.
    ///
    /// The loop body is intentionally one unit: propose → pick → apply →
    /// recheck → keep/revert share the `tried` set + the diagnostic state,
    /// and splitting it would thread four locals through helper
    /// signatures without clarifying any step (the `fix_verify` precedent).
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn run(&self, paths: &[PathBuf], playbooks: &Playbooks, write: bool) -> FixCompileReport {
        let mut report = FixCompileReport::default();

        // Scope: the owning packages of the given paths (deduped, sorted
        // for deterministic check order).
        let mut pkgs: Vec<PathBuf> = Vec::new();
        for p in paths {
            let Some(pkg) = owning_package_root(p) else {
                report.error = Some(format!("no Cargo.toml above {}", p.display()));
                return report;
            };
            if !pkgs.contains(&pkg) {
                pkgs.push(pkg);
            }
        }
        if pkgs.is_empty() {
            return report;
        }
        pkgs.sort();

        let mut diags = match self.check_all(&pkgs, &mut report) {
            Err(e) => {
                report.error = Some(e);
                return report;
            }
            Ok(d) => d,
        };
        report.ran = true;
        if diags.is_empty() {
            // The T2.3 gate: code that already compiles → nothing to fix,
            // zero edits. This arm is what the founding incident's
            // unnecessary fix would have tripped over.
            report.clean = true;
            return report;
        }

        if !write {
            // Review mode: propose once, plan, done. No writes.
            for d in &diags {
                collect_candidates(playbooks, d, &mut report, true);
            }
            report.remaining = diags.len();
            return report;
        }

        // ── Write mode: the bounded loop ─────────────────────────────────
        let mut tried: HashSet<TriedKey> = HashSet::new();
        for _iter in 0..self.max_iters {
            // Propose for every current diagnostic.
            let mut candidates: Vec<FixCandidate> = Vec::new();
            for d in &diags {
                candidates.extend(collect_candidates(playbooks, d, &mut report, false));
            }
            order_candidates(&mut candidates);
            candidates.retain(|c| !tried.contains(&tried_key(c)));
            let Some(best) = candidates.first().cloned() else {
                break; // no untried candidates left — stop (misses recorded)
            };

            report.iterations += 1;
            let before_errors = diags.len();

            // Apply: splice + write.
            let Ok(original) = std::fs::read_to_string(&best.file) else {
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::Io,
                });
                tried.insert(tried_key(&best));
                continue;
            };
            let Some(applied_text) = apply_edits(
                &original,
                best.byte_span,
                &best.replacement,
                &best.extra_edits,
            ) else {
                // Span drifted (should not happen — proposed against the
                // same read) — treat as no-progress revert.
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::NoProgress,
                });
                tried.insert(tried_key(&best));
                continue;
            };
            if std::fs::write(&best.file, &applied_text).is_err() {
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::Io,
                });
                tried.insert(tried_key(&best));
                continue;
            }

            // Recheck.
            let new_diags = match self.check_all(&pkgs, &mut report) {
                Err(e) => {
                    // cargo broke mid-loop: restore the file, report, stop.
                    let _ = std::fs::write(&best.file, &original);
                    report.error = Some(e);
                    break;
                }
                Ok(d) => d,
            };

            if new_diags.len() < before_errors {
                report.applied.push(AppliedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    errors: (before_errors, new_diags.len()),
                });
                diags = new_diags;
                if diags.is_empty() {
                    report.clean = true;
                    break;
                }
            } else {
                // No progress → exact revert (T1.4).
                let _ = std::fs::write(&best.file, &original);
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::NoProgress,
                });
                tried.insert(tried_key(&best));
            }
        }

        report.remaining = diags.len();
        report.clean = diags.is_empty();
        report
    }

    /// Check every package, parse + dedupe + resolve diagnostics to
    /// absolute paths. `Err` only when cargo could not run.
    fn check_all(
        &self,
        pkgs: &[PathBuf],
        report: &mut FixCompileReport,
    ) -> Result<Vec<ResolvedDiag>, String> {
        let mut out: Vec<ResolvedDiag> = Vec::new();
        for pkg in pkgs {
            let outcome = self.runner.run_check(pkg)?;
            report.n_checks += 1;
            for d in dedupe_by_site(&parse_error_diags(&outcome.json)) {
                out.push(ResolvedDiag {
                    diag: d,
                    pkg: pkg.clone(),
                });
            }
        }
        Ok(out)
    }
}

/// A diagnostic resolved against its reporting package.
struct ResolvedDiag {
    diag: RustcErrorDiag,
    pkg: PathBuf,
}

/// Resolve a diagnostic's primary span to an absolute path, read the
/// source, and propose. In review mode the candidates land in
/// `report.planned`; in write mode they're returned for ordering. Records
/// a miss when the primary span is absent, the file is unreadable, or no
/// playbook proposes.
fn collect_candidates(
    playbooks: &Playbooks,
    rd: &ResolvedDiag,
    report: &mut FixCompileReport,
    review: bool,
) -> Vec<FixCandidate> {
    let d = &rd.diag;
    let Some((file_name, line)) = d.primary.clone() else {
        report.misses.push(MissRecord {
            code: d.code.clone(),
            file: PathBuf::from(&file_name_placeholder(&d.message)),
            line: 0,
            message: d.message.clone(),
        });
        return Vec::new();
    };
    let ws = workspace_root(&rd.pkg).unwrap_or_else(|| rd.pkg.clone());
    let file = resolve_reported_path(Path::new(&file_name), &ws, &rd.pkg);
    let Ok(source) = std::fs::read_to_string(&file) else {
        report.misses.push(MissRecord {
            code: d.code.clone(),
            file,
            line,
            message: format!("(unreadable) {}", d.message),
        });
        return Vec::new();
    };
    let cands = playbooks.propose(d, &file, &source);
    if cands.is_empty() {
        report.misses.push(MissRecord {
            code: d.code.clone(),
            file,
            line,
            message: d.message.clone(),
        });
        return Vec::new();
    }
    if review {
        for c in &cands {
            report.planned.push(PlannedRecord {
                file: c.file.clone(),
                line: c.line,
                code: c.code.clone(),
                strategy_id: c.strategy_id.clone(),
                replacement_head: c.replacement.lines().next().unwrap_or("").to_string(),
            });
        }
    }
    cands
}

fn file_name_placeholder(msg: &str) -> String {
    format!("(no primary span) {msg}")
}

/// Replace `text[span]` with `replacement`, then apply `extra_edits` — all
/// in one atomic pass (spans applied lowest-first with the consumed prefix
/// tracked, so every offset is used exactly once against the ORIGINAL
/// text). `None` when any span is out of bounds, off a char boundary, or
/// OVERLAPS another (the caller treats it as a failed apply; a candidate
/// whose edits collide is malformed and must never half-land).
#[must_use]
fn apply_edits(
    text: &str,
    span: (usize, usize),
    replacement: &str,
    extras: &[SpanEdit],
) -> Option<String> {
    let mut all: Vec<((usize, usize), &str)> = Vec::with_capacity(extras.len() + 1);
    all.push((span, replacement));
    all.extend(extras.iter().map(|e| (e.byte_span, e.replacement.as_str())));
    all.sort_by_key(|((s, _), _)| *s);

    let mut out = String::with_capacity(text.len() + replacement.len());
    let mut cursor = 0usize;
    for ((s, e), repl) in all {
        if s < cursor {
            return None; // overlapping edits
        }
        if !text.is_char_boundary(s) || !text.is_char_boundary(e) || e < s || e > text.len() {
            return None;
        }
        out.push_str(&text[cursor..s]);
        out.push_str(repl);
        cursor = e;
    }
    out.push_str(&text[cursor..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_edits_replaces_exact_span() {
        let text = "let r = v;";
        let at = text.find("r = v").unwrap();
        let out = apply_edits(text, (at, at + 5), "mut r = v", &[]).unwrap();
        assert_eq!(out, "let mut r = v;");
    }

    #[test]
    fn apply_edits_rejects_bad_spans() {
        assert_eq!(apply_edits("héllo", (1, 2), "x", &[]), None, "non-char boundary");
        assert_eq!(apply_edits("abc", (2, 1), "x", &[]), None, "end < start");
        assert_eq!(apply_edits("abc", (0, 9), "x", &[]), None, "out of bounds");
        assert_eq!(
            apply_edits("abc", (0, 3), "x", &[]).as_deref(),
            Some("x")
        );
    }

    #[test]
    fn apply_edits_applies_multi_edits_atomically() {
        // The E0502 deref-removal shape: delete the `&` and the `*` in one
        // candidate — half-landing must be impossible.
        let text = "let first = &v[0];\n    v.push(1);\n    *first\n";
        let amp = text.find('&').unwrap();
        let star = text.find("*first").unwrap();
        let out = apply_edits(
            text,
            (amp, amp + 1),
            "",
            &[SpanEdit {
                byte_span: (star, star + 1),
                replacement: String::new(),
            }],
        )
        .unwrap();
        assert_eq!(out, "let first = v[0];\n    v.push(1);\n    first\n");
    }

    #[test]
    fn apply_edits_rejects_overlapping_edits() {
        assert_eq!(
            apply_edits(
                "abcdef",
                (1, 4),
                "XY",
                &[SpanEdit {
                    byte_span: (3, 5),
                    replacement: "Z".to_string(),
                }],
            ),
            None,
            "an edit starting inside the primary span must fail the whole candidate"
        );
    }

    #[test]
    fn ordering_is_file_line_then_shortest_total_replacement() {
        let mk = |file: &str, line: usize, repl: &str, strat: &str| FixCandidate {
            strategy_id: strat.to_string(),
            code: "E0597".to_string(),
            file: PathBuf::from(file),
            line,
            byte_span: (0, 0),
            replacement: repl.to_string(),
            extra_edits: Vec::new(),
        };
        let mut cands = vec![
            mk("b.rs", 1, "short", "z"),
            mk("a.rs", 9, "longer replacement", "y"),
            mk("a.rs", 2, "xxxxxxxx", "x"),
            mk("a.rs", 2, "yy", "w"),
        ];
        order_candidates(&mut cands);
        assert_eq!(cands[0].file, PathBuf::from("a.rs"));
        assert_eq!(cands[0].line, 2);
        assert_eq!(cands[0].replacement, "yy", "shortest delta first");
        assert_eq!(cands[1].replacement, "xxxxxxxx");
        assert_eq!(cands[2].line, 9);
        assert_eq!(cands[3].file, PathBuf::from("b.rs"));
    }

    #[test]
    fn ordering_counts_extra_edits_in_total_len() {
        let base = |strat: &str, extras: Vec<SpanEdit>| FixCandidate {
            strategy_id: strat.to_string(),
            code: "E0597".to_string(),
            file: PathBuf::from("a.rs"),
            line: 1,
            byte_span: (0, 0),
            replacement: String::new(),
            extra_edits: extras,
        };
        // A two-byte deletion pair must sort BEFORE a `.clone()` insertion
        // even though the primary replacement is empty in both.
        let pair = base(
            "deref-removal",
            vec![
                SpanEdit {
                    byte_span: (1, 2),
                    replacement: String::new(),
                },
                SpanEdit {
                    byte_span: (5, 6),
                    replacement: String::new(),
                },
            ],
        );
        assert_eq!(pair.total_len(), 0, "deletions add no bytes");
        let mut cands = vec![
            base(
                "clone-insert",
                vec![SpanEdit {
                    byte_span: (9, 9),
                    replacement: ".clone()".to_string(),
                }],
            ),
            pair,
        ];
        order_candidates(&mut cands);
        assert_eq!(cands[0].strategy_id, "deref-removal");
    }

    #[test]
    fn tried_key_distinguishes_strategies_at_same_span() {
        let a = FixCandidate {
            strategy_id: "s1".to_string(),
            code: "E0597".to_string(),
            file: PathBuf::from("a.rs"),
            line: 1,
            byte_span: (3, 8),
            replacement: "x".to_string(),
            extra_edits: Vec::new(),
        };
        let b = FixCandidate {
            strategy_id: "s2".to_string(),
            ..a.clone()
        };
        assert_ne!(tried_key(&a), tried_key(&b));
        assert_eq!(tried_key(&a), tried_key(&a.clone()));
    }
}
