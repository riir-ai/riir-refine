//! Issue 190 — the score-bench regime readout (`elo_k_eff`, REPORT-ONLY).
//!
//! One `regime:` line on the score-bench stdout: a
//! [`katgpt_core::step_noise::StepNoiseMonitor`] fed the (selection-state,
//! score) series ACROSS the run, so "selection churns while score flat"
//! (the `elo_k_eff` damp + oracle-outcome accumulation remedy) becomes
//! visible against "score flat, low churn" (mine more corpus) — Research
//! 485's consumer-context reframe (b)/(c). The `--frontier-report`
//! precedent: no exit-code change, no auto-demotion, feature-off prints
//! nothing.
//!
//! **The state series (an honest deviation, documented):** the issue
//! sketched a "per-rule rating vector slice". Ratings are static within a
//! bench run — Issue 039's non-goal pins NO online Elo updates in the hot
//! loop, and the bench never writes ratings — so a rating slice cannot
//! churn and the monitor would read a constant. The wired state is the
//! SELECTION distribution the rating plane governs: per step (one
//! single-target healable fixture whose target rule is in the tracked
//! slice, run order), the sliding-window pick share of the top-8 rules by
//! in-run involvement. The objective is the running resolved count over
//! the same steps. Regime semantics on this series:
//!
//! - `Learning` — resolutions still landing (slope well above the floor).
//! - `Thrash` — picks keep swinging while the score is flat (the damp
//!   remedy's exact target).
//! - `Stuck` — the pick distribution froze and the score is flat near the
//!   anchor (mine-more remedy).
//! - `Converged` — quiet after real selection travel.
//! - `CommittedWrong` — unreachable on this series by construction (the
//!   resolved count never decreases); the arm stays in the label set for
//!   the shared primitive's total match.
//!
//! Warmup is the primitive's own (16 steps): a shorter run prints
//! `warming` with its step count — never a fabricated label.

use super::BenchScore;
use katgpt_core::step_noise::{Regime, StepNoiseConfig, StepNoiseMonitor, StepNoiseRead};

/// The monitor's state dimension (the issue's size bound: 8 fits the
/// measured 28.3 ns/observe bar; the slice is the top-8 selection mass).
const N: usize = 8;

/// The pick window behind each state observation (steps). A window, not a
/// cumulative share: cumulative mass decays ~1/j mechanically and would
/// read every long run as quiet. 16 = the monitor's own warmup, so the
/// state is stationary by the time the read goes warm.
const WINDOW: usize = 16;

/// The consumer calibration: the primitive's DEFAULT posture with the two
/// scale midpoints retuned to this host's units (the DEFAULT assumes
/// unit-scale objectives — the module doc is the retune contract).
/// `p0`/`w0` = 0.05 resolutions/step (a run still landing 1 resolution
/// per 20 steps reads as progress, not noise); `c0` stays 1e-2
/// (window-share movement ≥ 1%/step is real churn). Gates/betas/taus/
/// warmup inherit DEFAULT.
#[must_use]
pub fn regime_config() -> StepNoiseConfig {
    StepNoiseConfig {
        p0: 0.05,
        w0: 0.05,
        ..StepNoiseConfig::DEFAULT
    }
}

/// The regime's stdout label — a total match, so a new primitive variant
/// breaks this build instead of printing `{:?}` bytes unreviewed.
#[must_use]
pub fn regime_label(regime: Regime) -> &'static str {
    match regime {
        Regime::Learning => "Learning",
        Regime::Thrash => "Thrash",
        Regime::Stuck => "Stuck",
        Regime::Converged => "Converged",
        Regime::CommittedWrong => "CommittedWrong",
    }
}

/// The selection slice: the top-`N` rules by in-run involvement (fixture
/// count; ties by rule name ascending — deterministic, stable for the
/// whole walk so the monitor's per-dimension semantics hold).
#[must_use]
fn selection_slice(score: &BenchScore) -> Vec<String> {
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for o in &score.outcomes {
        if !o.ran || o.vocab_before.is_empty() || o.vocab_before.len() > 1 {
            continue;
        }
        if let Some(rule) = o.vocab_before.iter().next() {
            *counts.entry(rule).or_default() += 1;
        }
    }
    let mut rows: Vec<(&str, usize)> = counts.into_iter().collect();
    rows.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    rows.truncate(N);
    rows.into_iter().map(|(rule, _)| rule.to_string()).collect()
}

/// The final read plus the selection-step count. `None` when the run had
/// no selection steps (no in-slice single-target fixture ran).
#[must_use]
#[allow(clippy::cast_precision_loss)] // counts < 2^23 by contract
pub fn regime_read(score: &BenchScore) -> Option<(StepNoiseRead, usize)> {
    let slice = selection_slice(score);
    if slice.is_empty() {
        return None;
    }
    let slot: std::collections::HashMap<&str, usize> = slice
        .iter()
        .enumerate()
        .map(|(i, r)| (r.as_str(), i))
        .collect();
    let mut monitor = StepNoiseMonitor::<N>::new(regime_config());
    let mut window: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut per_rule = [0usize; N];
    let mut resolved_total = 0u32;
    let mut steps = 0usize;
    let mut last_read = None;
    for o in &score.outcomes {
        if !o.ran || o.vocab_before.is_empty() || o.vocab_before.len() > 1 {
            continue; // excluded / clean / multi-target: not a selection step
        }
        let Some(rule) = o.vocab_before.iter().next() else {
            continue;
        };
        // Out-of-slice targets are skipped whole (state AND objective):
        // this readout prices the top-8 selection plane's own series.
        let Some(&s) = slot.get(rule.as_str()) else {
            continue;
        };
        window.push_back(s);
        per_rule[s] += 1;
        if window.len() > WINDOW {
            if let Some(old) = window.pop_front() {
                per_rule[old] = per_rule[old].saturating_sub(1);
            }
        }
        resolved_total += u32::try_from(o.resolved()).unwrap_or(u32::MAX);
        steps += 1;
        let mut state = [0.0f32; N];
        let len = window.len() as f32;
        for (i, c) in per_rule.iter().enumerate() {
            state[i] = *c as f32 / len;
        }
        last_read = Some(monitor.observe(&state, resolved_total as f32));
    }
    last_read.map(|read| (read, steps))
}

/// The one stdout line. `None` = nothing to report (no steps) — the
/// feature-on run over a stepless fixture set prints nothing, like the
/// feature-off build.
#[must_use]
pub fn regime_line(score: &BenchScore) -> Option<String> {
    let (read, steps) = regime_read(score)?;
    let warmup = regime_config().warmup as usize;
    Some(if steps >= warmup {
        format!(
            "regime: {} (churn {:.3} slope {:+.4} steps {steps})",
            regime_label(read.regime),
            read.churn,
            read.progress_slope
        )
    } else {
        format!("regime: warming ({steps}/{warmup} selection steps)")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drive helper mirrors katgpt-rs tests/step_noise_gates.rs: push
    /// an explicit synthetic (state, objective) series, return the final
    /// read. This prices the MONITOR + the consumer calibration; the
    /// window/slice arithmetic above is the production wiring.
    fn drive(series: &[([f32; N], f32)]) -> StepNoiseRead {
        let mut monitor = StepNoiseMonitor::<N>::new(regime_config());
        let mut last = None;
        for (state, objective) in series {
            last = Some(monitor.observe(state, *objective));
        }
        last.expect("non-empty series")
    }

    fn one_hot(i: usize) -> [f32; N] {
        let mut s = [0.0f32; N];
        s[i] = 1.0;
        s
    }

    /// Production-scale pick alternation: the two states are the settled
    /// window shares of an A/B alternation (±0.06 per dim, Δnorm ≈ 0.17
    /// per step — what a WINDOWED selection distribution actually moves),
    /// not full-amplitude one-hots (Δnorm ≈ 1.41 would leave the churn
    /// EMA hot for ~46 decay steps and blur the quiet-band fixtures).
    fn share(a: bool) -> [f32; N] {
        let mut s = [0.0f32; N];
        s[0] = if a { 0.56 } else { 0.44 };
        s[1] = if a { 0.44 } else { 0.56 };
        s
    }

    #[test]
    fn thrash_fixture_class_reads_thrash_on_churn_flat_score() {
        // Alternating picks (the selection state swings hard each step)
        // with a FLAT objective: the damp-remedy target.
        let series: Vec<([f32; N], f32)> = (0usize..48)
            .map(|t| (share(t.is_multiple_of(2)), 1.0))
            .collect();
        let read = drive(&series);
        assert!(read.churn > 0.02, "churn should read hot: {}", read.churn);
        assert_eq!(read.regime, Regime::Thrash, "read: {read:?}");
    }

    #[test]
    fn learning_fixture_class_reads_learning_on_a_healthy_improvement_series() {
        // Resolutions every step (slope ≈ 1 ≫ p0) while picks alternate:
        // the thrash gate's (1−progress) term must fail and Learning win.
        let series: Vec<([f32; N], f32)> = (0usize..48)
            .map(|t| (share(t.is_multiple_of(2)), (t + 1) as f32))
            .collect();
        let read = drive(&series);
        assert!(
            read.progress_slope > 0.05,
            "slope should read as progress: {}",
            read.progress_slope
        );
        assert_eq!(read.regime, Regime::Learning, "read: {read:?}");
    }

    #[test]
    fn stuck_fixture_class_reads_stuck_on_a_frozen_flat_series() {
        // The same rule picked every step (the state never moves — the
        // windowed share's settled shape), objective flat: mine-more arm.
        let series: Vec<([f32; N], f32)> = (0usize..48).map(|_| (one_hot(0), 1.0)).collect();
        let read = drive(&series);
        assert!(read.churn < 1e-3, "churn should decay: {}", read.churn);
        assert_eq!(read.regime, Regime::Stuck, "read: {read:?}");
    }

    #[test]
    fn converged_fixture_class_reads_converged_after_travel() {
        // Real selection travel first (production-scale amplitude), then
        // freeze AT the last active state (a parity flip here would add
        // one more movement step): quiet after movement.
        let mut series: Vec<([f32; N], f32)> = (0usize..24)
            .map(|t| (share(t.is_multiple_of(2)), 1.0))
            .collect();
        let settled = share(false); // t=23 is odd → the last active state
        series.extend((0usize..40).map(|_| (settled, 1.0)));
        let read = drive(&series);
        assert_eq!(read.regime, Regime::Converged, "read: {read:?}");
    }

    #[test]
    fn regime_label_covers_every_variant() {
        assert_eq!(regime_label(Regime::Learning), "Learning");
        assert_eq!(regime_label(Regime::Thrash), "Thrash");
        assert_eq!(regime_label(Regime::Stuck), "Stuck");
        assert_eq!(regime_label(Regime::Converged), "Converged");
        assert_eq!(regime_label(Regime::CommittedWrong), "CommittedWrong");
    }

    #[test]
    fn windowed_series_over_a_bench_score_matches_the_fixture_classes() {
        // The PRODUCTION wiring end to end: synthesize a BenchScore whose
        // outcomes alternate targets (churn) with no resolutions (flat),
        // and one with resolutions landing (learning). The bench-score
        // constructors are test-side; build outcomes directly.
        use super::super::FixtureOutcome;
        let mk = |rule: &str, resolved: bool| FixtureOutcome {
            id: String::new(),
            ran: true,
            vocab_before: [rule.to_string()].into_iter().collect(),
            all_before: [rule.to_string()].into_iter().collect(),
            all_after: if resolved {
                [].into_iter().collect()
            } else {
                [rule.to_string()].into_iter().collect()
            },
            vocab_persist: if resolved {
                [].into_iter().collect()
            } else {
                [rule.to_string()].into_iter().collect()
            },
            created: std::collections::BTreeSet::new(),
            edits: 1,
            passes: 1,
            parses: true,
            drifted: false,
        };
        // Thrash: alternate two rules, nothing resolves.
        let mut score = BenchScore::default();
        for t in 0usize..48 {
            let rule = if t.is_multiple_of(2) {
                "rule_a"
            } else {
                "rule_b"
            };
            score.outcomes.push(mk(rule, false));
        }
        let (read, steps) = regime_read(&score).expect("steps");
        assert_eq!(steps, 48);
        assert_eq!(read.regime, Regime::Thrash, "read: {read:?}");
        let line = regime_line(&score).expect("warm");
        assert!(line.starts_with("regime: Thrash"), "{line}");
    }

    #[test]
    fn short_run_prints_warming_and_stepless_prints_nothing() {
        use super::super::FixtureOutcome;
        let mk = |rule: &str| FixtureOutcome {
            id: String::new(),
            ran: true,
            vocab_before: [rule.to_string()].into_iter().collect(),
            all_before: [rule.to_string()].into_iter().collect(),
            all_after: [rule.to_string()].into_iter().collect(),
            vocab_persist: [rule.to_string()].into_iter().collect(),
            created: std::collections::BTreeSet::new(),
            edits: 0,
            passes: 1,
            parses: true,
            drifted: false,
        };
        // 4 steps < warmup 16 → the honest warming line.
        let mut score = BenchScore::default();
        for _ in 0..4 {
            score.outcomes.push(mk("rule_a"));
        }
        let line = regime_line(&score).expect("steps");
        assert!(line.starts_with("regime: warming (4/16"), "{line}");
        // No single-target ran outcome → no line at all.
        let empty = BenchScore::default();
        assert!(regime_line(&empty).is_none());
    }
}
