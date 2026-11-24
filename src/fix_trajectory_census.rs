//! Issue 256 T0/T1 — the fix-trajectory early-read census (report-only
//! core; the `fix_trajectory_census` example is the thin printer).
//!
//! Over the `.refine/fixseq/` per-attempt rings
//! ([`crate::horizon_report::load_runs`], the ONE loader):
//!
//! - **T0** — the `DoM` early-read: fit the shipped 9-feature ridge
//!   ([`crate::horizon_report::ridge_fit`] verbatim) on each run's FIRST
//!   `early = max(1, len / 10)` attempt(s) (the "first 5–10%" discipline at
//!   ring granularity) and score held-out recall/precision of "this run
//!   ends non-clean", leave-one-run-out.
//! - **T1** — the macrostate audit: `katgpt_core::closure_audit::run` per
//!   fold (feature `fix_trajectory_census` forwards
//!   `katgpt-core/closure_audit`, Issue 1244 / Bench 994), summary = the
//!   41-D state rows, microstate = the 32-D `span_embed` half; the typed
//!   verdict prints as-is (the primitive's own pinned bars).
//!
//! Refusals are typed ([`CensusRefusal`]) and the caller refuses LOUD —
//! a census over nothing is a failure, never a pass (the horizon-report
//! law). Nothing here gates or wires; no fixer behavior is touched.
//!
//! # Determinism (the Issue-940 family)
//!
//! Runs in file/line order, one sequential f64 fold per score, LORO folds
//! in group order — the same rings yield the same stdout.

use crate::horizon_report::{self, RingLine, RunRing};
#[cfg(feature = "fix_trajectory_census")]
use crate::span_embed::SPAN_EMBED_DIMS;

/// The early prefix fraction denominator — "first ~10%" (issue 256's `DoM`
/// window). `early = max(1, len / EARLY_DENOM)`.
pub const EARLY_DENOM: usize = 10;

/// Why the census refused to measure. Typed, never folded into a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CensusRefusal {
    /// Zero `run-*.jsonl` files across every scanned root.
    NoRings { roots: usize },
    /// Every loaded run carries a single attempt (no early→later pair).
    NoMultiAttempt { runs: usize },
    /// No run ended non-clean — the target class is empty.
    SingleClass { runs: usize },
}

impl std::fmt::Display for CensusRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRings { roots } => write!(
                f,
                "NO RINGS: no .refine/fixseq/run-*.jsonl across {roots} root(s) — \
                 the per-attempt history was never dumped here (the rings appear \
                 after `cargo refine --fix-compile --write` runs; see the capture \
                 note in .benchmarks/165)"
            ),
            Self::NoMultiAttempt { runs } => write!(
                f,
                "NO MULTI-ATTEMPT: all {runs} run(s) carry a single attempt — \
                 no early→later transition exists to read"
            ),
            Self::SingleClass { runs } => write!(
                f,
                "SINGLE CLASS: all {runs} run(s) ended clean — \"will verify \
                 fail\" has no positive labels to measure recall against"
            ),
        }
    }
}

/// One run's census row: the early-window feature block, the label, and
/// the later state rows the T1 audit consumes.
#[derive(Debug, Clone)]
pub struct EarlyRun {
    /// The run's file name (the LORO grouping key).
    pub source: String,
    /// Early prefix lines (the first `early` attempts).
    pub early: Vec<RingLine>,
    /// Every later line as a 41-D state row (`features_at` 9 + the decoded
    /// `span_embed` 32). `None` embed → the run is arm-skipped upstream.
    pub later_states: Vec<[f64; horizon_report::N_FEAT_EMBED]>,
    /// True when the run's LAST attempt reverted (the run "ends non-clean").
    pub ends_bad: bool,
}

/// The early-window count for a run: `max(1, len / 10)`.
#[must_use]
pub fn early_count(run_len: usize) -> usize {
    (run_len / EARLY_DENOM).max(1).min(run_len)
}

/// Fold one run into an [`EarlyRun`]. `None` when the run has no
/// transition to read (fewer than 2 attempts) or any later line lacks the
/// embed column (pre-T2 rings — the arm-skip discipline, never zero-fill).
#[must_use]
pub fn early_run(ring: &RunRing) -> Option<EarlyRun> {
    if ring.lines.len() < 2 {
        return None;
    }
    let early_n = early_count(ring.lines.len());
    let mut later_states = Vec::with_capacity(ring.lines.len() - early_n);
    for line in &ring.lines[early_n..] {
        later_states.push(horizon_report::features_at_embed(line, ring.lines.len())?);
    }
    let ends_bad = ring.lines.last().and_then(RingLine::kept)?;
    Some(EarlyRun {
        source: ring.source.clone(),
        early: ring.lines[..early_n].to_vec(),
        later_states,
        ends_bad: !ends_bad,
    })
}

/// The T0 readout over LORO folds: the run-level early-state score per
/// run (ridge fit on every OTHER run's early blocks, mean margin over the
/// held-out run's own early lines) + the run's label.
///
/// Returns `None` when no fold produced a fit (empty input or every fit
/// degenerate) — the caller refuses, never scores a green zero.
#[must_use]
pub fn t0_scores(runs: &[EarlyRun]) -> Option<Vec<(f64, bool)>> {
    if runs.is_empty() {
        return None;
    }
    let mut scores = Vec::with_capacity(runs.len());
    for (gi, g) in runs.iter().enumerate() {
        let train: Vec<([f64; horizon_report::N_FEAT], bool)> = runs
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != gi)
            .flat_map(|(_, r)| {
                r.early
                    .iter()
                    .map(move |line| (horizon_report::features_at(line, r.early.len()), r.ends_bad))
            })
            .collect();
        if let Some(w) = horizon_report::ridge_fit(&train) {
            let mut acc = 0.0_f64;
            for line in &g.early {
                let x = horizon_report::features_at(line, g.early.len());
                acc += w.iter().zip(x.iter()).map(|(a, b)| a * b).sum::<f64>();
            }
            #[allow(clippy::cast_precision_loss)]
            // usize counter → f64 divisor — the report's quantity
            scores.push((acc / g.early.len() as f64, g.ends_bad));
        }
    }
    if scores.is_empty() {
        None
    } else {
        Some(scores)
    }
}

/// Recall/precision at the score's own median cut (report-only; the census
/// reads separability, never a deployed threshold).
#[must_use]
#[allow(clippy::cast_precision_loss)] // usize counts → f64 rates — the report's quantity
pub fn recall_precision_at_median(scores: &[(f64, bool)]) -> (Option<f64>, Option<f64>) {
    let n_pos = scores.iter().filter(|(_, y)| *y).count();
    let n_neg = scores.len() - n_pos;
    if n_pos == 0 || n_neg == 0 {
        return (None, None);
    }
    let mut sorted: Vec<f64> = scores.iter().map(|(s, _)| *s).collect();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    let cut = sorted[mid];
    let tp = scores.iter().filter(|(s, y)| *y && *s >= cut).count();
    let fp = scores.iter().filter(|(s, y)| !*y && *s >= cut).count();
    let recall = tp as f64 / n_pos as f64;
    let precision = if tp + fp == 0 {
        0.0
    } else {
        tp as f64 / (tp + fp) as f64
    };
    (Some(recall), Some(precision))
}

/// One fold's audit outcome: the run's source + the audit result (typed
/// error, never a panic).
#[cfg(feature = "fix_trajectory_census")]
pub type FoldAudit = (
    String,
    Result<
        katgpt_core::closure_audit::ClosureReport,
        katgpt_core::closure_audit::ClosureAuditError,
    >,
);

/// The T1 arm over the census runs: one [`katgpt_core::closure_audit::run`]
/// per run — summary = the 41-D state sequence (early lines then later
/// states), microstate = the 32-D embed half. Runs with no embed column or
/// fewer than 2 states are skipped (the arm-skip discipline), never
/// zero-filled.
#[cfg(feature = "fix_trajectory_census")]
#[must_use]
pub fn t1_audits(runs: &[EarlyRun]) -> Vec<FoldAudit> {
    let mut out = Vec::new();
    for g in runs {
        let mut states: Vec<[f64; horizon_report::N_FEAT_EMBED]> = Vec::new();
        let total_len = g.early.len() + g.later_states.len();
        for line in &g.early {
            let mut row = [0.0_f64; horizon_report::N_FEAT_EMBED];
            row[..horizon_report::N_FEAT]
                .copy_from_slice(&horizon_report::features_at(line, total_len));
            if let Some(e) = line.span_embed {
                for (i, v) in e.iter().enumerate() {
                    row[horizon_report::N_FEAT + i] = f64::from(*v) / 127.0;
                }
                states.push(row);
            }
        }
        states.extend(g.later_states.iter().copied());
        if states.len() < 2 {
            continue;
        }
        let dim = horizon_report::N_FEAT_EMBED;
        let md = SPAN_EMBED_DIMS;
        let mut z = Vec::with_capacity(states.len() * dim);
        let mut m = Vec::with_capacity(states.len() * md);
        for s in &states {
            #[allow(clippy::cast_possible_truncation)]
            // features are O(1)-scale; the audit consumes f32 by contract
            z.extend(s.iter().map(|v| *v as f32));
            #[allow(clippy::cast_possible_truncation)] // decoded embeds are [-1, 1]
            m.extend(s[horizon_report::N_FEAT..].iter().map(|v| *v as f32));
        }
        let report = katgpt_core::closure_audit::run(
            &[&z],
            dim,
            Some((&[&m], md)),
            &katgpt_core::closure_audit::ClosureAuditConfig::default(),
        );
        out.push((g.source.clone(), report));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(seq: usize, strategy: &str, kept: bool, errors: (usize, usize)) -> RingLine {
        RingLine {
            seq,
            code: "E0597".to_string(),
            span_hash: format!("{:016x}", (seq as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
            strategy_id: strategy.to_string(),
            outcome: if kept {
                "Kept".to_string()
            } else {
                "RevertedNoProgress".to_string()
            },
            errors,
            masking: (0, 0),
            journey_residual_sum: 0,
            terminal_residual: 0,
            span_embed: Some(crate::span_embed::span_embed(&format!(
                "{strategy} attempt {seq} trigger text"
            ))),
        }
    }

    #[test]
    fn early_count_is_the_max_one_tenth() {
        assert_eq!(early_count(1), 1);
        assert_eq!(early_count(9), 1);
        assert_eq!(early_count(10), 1);
        assert_eq!(early_count(20), 2);
        assert_eq!(early_count(100), 10);
    }

    /// A 20-attempt run whose first two attempts GROW errors and whose
    /// final attempt reverts — the "doomed run" shape the T0 probe should
    /// rank above a clean run.
    fn doomed(source: &str) -> RunRing {
        let mut lines: Vec<RingLine> = (0..19)
            .map(|i| {
                let grow = i < 2;
                line(
                    i,
                    "e0502-clone-out",
                    true,
                    if grow {
                        (5 + i, 6 + i)
                    } else {
                        (30 - i, 29 - i)
                    },
                )
            })
            .collect();
        lines.push(line(19, "e0502-clone-out", false, (2, 3)));
        RunRing {
            source: source.to_string(),
            lines,
        }
    }

    fn clean(source: &str) -> RunRing {
        RunRing {
            source: source.to_string(),
            lines: (0..20)
                .map(|i| line(i, "e0597-move", true, (30 - i, 29 - i)))
                .collect(),
        }
    }

    #[test]
    fn ends_bad_labels_the_final_revert() {
        let dirty = early_run(&doomed("d")).expect("multi-attempt");
        assert!(dirty.ends_bad);
        assert_eq!(dirty.early.len(), 2); // max(1, 20/10)
        let clean_e = early_run(&clean("c")).expect("multi-attempt");
        assert!(!clean_e.ends_bad);
    }

    #[test]
    fn single_attempt_runs_are_skipped() {
        let single = RunRing {
            source: "one".into(),
            lines: vec![line(0, "s", true, (3, 2))],
        };
        assert!(early_run(&single).is_none());
    }

    #[test]
    fn missing_embed_column_skips_the_run() {
        let mut ring = clean("noembed");
        // index 1 of a 20-line ring is inside the EARLY window (2 lines);
        // early lines skip via t1_audits' embed gate, not early_run's.
        ring.lines[1].span_embed = None;
        assert!(early_run(&ring).is_some());
        ring.lines[10].span_embed = None;
        assert!(early_run(&ring).is_none());
    }

    #[test]
    fn t0_refuses_empty_and_scores_the_toy_signal() {
        assert!(t0_scores(&[]).is_none());
        let mut runs = Vec::new();
        for i in 0..6 {
            runs.push(early_run(&clean(&format!("clean{i}"))).expect("run"));
        }
        for i in 0..4 {
            runs.push(early_run(&doomed(&format!("dirty{i}"))).expect("run"));
        }
        let scores = t0_scores(&runs).expect("folds");
        assert_eq!(scores.len(), runs.len());
        let (recall, precision) = recall_precision_at_median(&scores);
        assert!(recall.is_some() && precision.is_some());
    }

    #[cfg(feature = "fix_trajectory_census")]
    #[test]
    fn t1_audits_return_typed_reports() {
        let runs = [
            early_run(&clean("c0")).expect("run"),
            early_run(&doomed("d0")).expect("run"),
        ];
        let audits = t1_audits(&runs);
        assert_eq!(audits.len(), 2);
        for (source, report) in &audits {
            let r = report.as_ref().expect("audit Ok");
            assert!(r.support.windows >= 1, "{source}: the early→later window");
        }
    }
}
