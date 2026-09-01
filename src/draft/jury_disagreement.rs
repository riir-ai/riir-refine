//! Issue 036 — jury-disagreement verify escalation POC (T1–T3).
//!
//! ## The transfer being tested
//!
//! arXiv:2608.12645 §6.2 ("Jagged Judges"): baseline jury majority strength —
//! one cheap cross-judge vote snapshot — is the single best predictor of which
//! verdicts flip under pressure (|ρ| = 0.59, all 84 cells negative), beating
//! repeat-consistency and positional invariance. The judges there are 9
//! sampled LLMs; ours are deterministic proposers whose disagreement reflects
//! corpus geometry rather than sampling noise. The *principle* under test is
//! whether baseline disagreement forecasts instability — here, whether a
//! span's **retrieval-pool contention** predicts **fix failure** (compile-gate
//! revert or verifier lint-not-resolved) before any fix is applied.
//!
//! ## The two signals (T1, measured independently)
//!
//! - `pool_spread` — distinct rule ids in the top-`RERANK_POOL` retrieval pool
//!   BEFORE re-ranking. The index holds one chunk per whole pattern PLUS one
//!   per significant line, so a strongly-matching rule floods the pool with
//!   its own line-chunks (low spread = consensus) while a contested span pulls
//!   many different rules (high spread = no majority).
//! - `rerank_margin` — ranked[0].score − ranked[1].score AFTER instantiation
//!   re-rank. Thin (or negative — a categorical override) margins mark spans
//!   where the ordering decision was close.
//!
//! ## The instability join (T2)
//!
//! Per span: apply the top-1 rule's proposer fix to the span, then ask real
//! `cargo clippy` (the T3.5.5 oracle — no connection to our corpus or
//! proposers) what happened. `CompileFail` is what `--fix --verify`'s compile
//! gate would revert; `LintUnresolved` is what `ClippyVerifier` would report.
//! `NoFix` (proposer silent — nothing to verify) is excluded from the join the
//! same way the paper excludes items without a valid L0 verdict: there is no
//! verdict to be unstable.
//!
//! ## Read-only by construction (G3)
//!
//! Nothing here touches the retrieval path: [`LatentMatcher::pool_and_ranked`]
//! replicates `suggest_all_domains`'s per-domain steps on a COPY of the pool
//! and every existing entry point is untouched. The Issue 017 floors cannot
//! move because of this module.

// Spearman/contingency arithmetic over fixture-population counts (≤ a few
// hundred): usize→f64 precision loss is not reachable at these magnitudes.
#![allow(clippy::cast_precision_loss)]

use std::collections::BTreeSet;

use super::clippy_oracle::ClippyOracle;
use super::corpus_index::ScoredRule;
use super::domain_router::DomainId;
use super::latent_matcher::LatentMatcher;

/// Per-span disagreement snapshot (T1). All fields are passive reads of the
/// pre-commit retrieval state — no fix was applied to produce them.
#[derive(Debug, Clone)]
pub struct DisagreementSignals {
    /// Pool size actually retrieved (≤ `RERANK_POOL`; a tiny corpus may
    /// return fewer).
    pub pool_len: usize,
    /// Distinct rule ids among those pool entries. Low = one rule's
    /// line-chunks dominate (jury majority); high = many rules tie
    /// (contested).
    pub pool_spread: usize,
    /// How many pool rules' proposers fire on the span (diagnostic — the
    /// paper's "how many judges speak at all").
    pub instantiating: usize,
    /// Post-rerank top-1 minus top-2 similarity score. Negative when the
    /// instantiation flag overrode similarity. `INFINITY` when fewer than
    /// two candidates exist (uncontested by emptiness).
    pub rerank_margin: f32,
    /// The top-1 rule id after re-ranking — the fix candidate whose outcome
    /// joins against these signals.
    pub top1_rule: String,
}

/// What happened when the top-1 fix candidate was actually applied (T2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixOutcome {
    /// Top-1's proposer emitted no edit — nothing to apply or verify.
    /// Excluded from the join (no verdict to be unstable).
    NoFix,
    /// The ORIGINAL span does not compile (Issue 022: 5 of the 48 original
    /// fixtures reference undeclared items). Blaming the fix for a broken
    /// baseline would fabricate instability — excluded like `NoFix`.
    FixtureBroken,
    /// The applied edit breaks compilation — `--fix --verify` would revert
    /// this edit. UNSTABLE.
    CompileFail,
    /// Compiles, but real clippy still reports the top-1's own lint —
    /// `ClippyVerifier` would report lint-not-resolved. UNSTABLE.
    LintUnresolved,
    /// Compiles and (for clippy spans) the top-1's lint is gone. Stable.
    Resolved,
    /// cargo unreachable — the row must be dropped, not scored.
    OracleDown,
}

impl FixOutcome {
    /// The paper's "wiggle" analog: did the applied fix fail its verification?
    /// `NoFix`/`FixtureBroken`/`OracleDown` are not verdicts and are not
    /// unstable.
    #[must_use]
    pub fn is_unstable(self) -> bool {
        matches!(self, FixOutcome::CompileFail | FixOutcome::LintUnresolved)
    }

    /// Is this a verdict at all (vs an exclusion)?
    #[must_use]
    pub fn is_verdict(self) -> bool {
        !matches!(
            self,
            FixOutcome::NoFix | FixOutcome::FixtureBroken | FixOutcome::OracleDown
        )
    }
}

/// One measured span: signals + joined outcome + report metadata.
#[derive(Debug, Clone)]
pub struct JuryRow {
    pub tag: String,
    pub domain: DomainId,
    /// The fixture's labeled rule ("composite:<n>" for multi-lint spans) —
    /// context for the report, not used by the correlation.
    pub label: String,
    pub signals: DisagreementSignals,
    pub outcome: FixOutcome,
}

/// Reduce a (pool, ranked) pair to the disagreement signals.
#[must_use]
pub fn disagreement_signals(pool: &[ScoredRule], ranked: &[ScoredRule]) -> DisagreementSignals {
    let distinct: BTreeSet<&str> = pool.iter().map(|r| r.rule_id.as_str()).collect();
    let rerank_margin = if ranked.len() >= 2 {
        ranked[0].score - ranked[1].score
    } else {
        f32::INFINITY
    };
    DisagreementSignals {
        pool_len: pool.len(),
        pool_spread: distinct.len(),
        instantiating: ranked.iter().filter(|r| r.instantiates).count(),
        rerank_margin,
        top1_rule: ranked.first().map_or_else(String::new, |r| r.rule_id.clone()),
    }
}

/// Apply `top1`'s proposer fix to `code` and ask the oracle what happened.
///
/// The baseline is checked FIRST (Issue 022: some original fixtures do not
/// compile) — a fix cannot be blamed for a span that never compiled.
///
/// The `rust_perf` proposers are only reachable when that feature is on;
/// without it, perf spans report `NoFix` (conservative — absence of a
/// proposer is not evidence of anything).
#[must_use]
pub fn fix_outcome(
    oracle: &ClippyOracle,
    domain: DomainId,
    code: &str,
    top1: &str,
) -> FixOutcome {
    let Ok(file) = syn::parse_file(code) else {
        // The eval fixtures all parse; an unparseable span has no fix path.
        return FixOutcome::NoFix;
    };
    let edits = collect_domain_edits(domain, top1, code, &file);
    if edits.is_empty() {
        return FixOutcome::NoFix;
    }
    let baseline = oracle.lints_for(code);
    if !baseline.ran {
        return FixOutcome::OracleDown;
    }
    if !baseline.compiles {
        return FixOutcome::FixtureBroken;
    }
    let fixed = crate::edit_emitter::apply_edits(code, edits);
    let verdict = oracle.lints_for(&fixed);
    if !verdict.ran {
        return FixOutcome::OracleDown;
    }
    if !verdict.compiles {
        return FixOutcome::CompileFail;
    }
    if domain == DomainId::Clippy && verdict.lints.contains(top1) {
        return FixOutcome::LintUnresolved;
    }
    FixOutcome::Resolved
}

fn collect_domain_edits(
    domain: DomainId,
    rule_id: &str,
    code: &str,
    file: &syn::File,
) -> Vec<crate::edit_emitter::SourceEdit> {
    let rule_edits = match domain {
        DomainId::Clippy => {
            #[cfg(feature = "clippy_lints")]
            {
                crate::domains::clippy_lints::proposers::collect_edits(rule_id, code, file)
            }
            #[cfg(not(feature = "clippy_lints"))]
            {
                let _ = (rule_id, code, file);
                Vec::new()
            }
        }
        DomainId::Perf => {
            #[cfg(feature = "rust_perf")]
            {
                crate::domains::rust_perf::proposers::collect_edits(rule_id, code, file)
            }
            #[cfg(not(feature = "rust_perf"))]
            {
                let _ = (rule_id, code, file);
                Vec::new()
            }
        }
        // Kernel has no proposers (Plan 005 Phase 5 deferred) and sec is out
        // of this POC's scope (its fixtures are not oracle-verifiable).
        _ => Vec::new(),
    };
    rule_edits.into_iter().map(|re| re.edit).collect()
}

// ── T3: statistics ─────────────────────────────────────────────────────────

/// Spearman's ρ with average ranks for ties.
///
/// The binary instability variable ties massively (every stable span shares
/// one rank), which average-rank assignment handles exactly; distinct-value
/// ranking would silently bias ρ toward 0.
#[must_use]
pub fn spearman_rho(xs: &[f64], ys: &[f64]) -> f64 {
    debug_assert_eq!(xs.len(), ys.len());
    if xs.len() < 2 {
        return 0.0;
    }
    let rx = average_ranks(xs);
    let ry = average_ranks(ys);
    pearson(&rx, &ry)
}

fn average_ranks(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut ranks = vec![0.0f64; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]].total_cmp(&v[idx[i]]) == std::cmp::Ordering::Equal {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for &k in &idx[i..=j] {
            ranks[k] = avg;
        }
        i = j + 1;
    }
    ranks
}

fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len() as f64;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut dx = 0.0;
    let mut dy = 0.0;
    for (&x, &y) in xs.iter().zip(ys) {
        let a = x - mx;
        let b = y - my;
        num += a * b;
        dx += a * a;
        dy += b * b;
    }
    if dx == 0.0 || dy == 0.0 {
        return 0.0;
    }
    num / (dx * dy).sqrt()
}

/// One threshold's 2×2 contingency over the verdict-bearing rows.
#[derive(Debug, Clone)]
pub struct Contingency {
    /// The threshold tested (spread ≥ t, or margin < t).
    pub threshold: f64,
    pub total: usize,
    pub flagged: usize,
    pub unstable_total: usize,
    pub unstable_flagged: usize,
}

impl Contingency {
    /// Fraction of unstable spans the threshold captures (recall).
    #[must_use]
    pub fn capture(&self) -> f64 {
        if self.unstable_total == 0 {
            0.0
        } else {
            self.unstable_flagged as f64 / self.unstable_total as f64
        }
    }

    /// Fraction of all verdict spans flagged (the cost side).
    #[must_use]
    pub fn flagged_frac(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.flagged as f64 / self.total as f64
        }
    }
}

fn contingency_at(rows: &[&JuryRow], flagged: impl Fn(&JuryRow) -> bool, threshold: f64) -> Contingency {
    let mut c = Contingency { threshold, total: 0, flagged: 0, unstable_total: 0, unstable_flagged: 0 };
    for r in rows {
        c.total += 1;
        let f = flagged(r);
        if f {
            c.flagged += 1;
        }
        if r.outcome.is_unstable() {
            c.unstable_total += 1;
            if f {
                c.unstable_flagged += 1;
            }
        }
    }
    c
}

/// The full T3 report: ρ per signal + the threshold sweeps.
#[derive(Debug, Clone, Default)]
pub struct JuryPocReport {
    pub rows: Vec<JuryRow>,
    /// (signal name, ρ) over verdict-bearing rows.
    pub rhos: Vec<(&'static str, f64)>,
    /// Spread sweep, thresholds 2..=pool max.
    pub spread_sweep: Vec<Contingency>,
    /// Margin sweep over the observed descending thresholds.
    pub margin_sweep: Vec<Contingency>,
    /// Verdict-bearing row count + unstable count (the join's denominator).
    pub verdict_rows: usize,
    pub unstable_rows: usize,
}

/// Run T1+T2 over the fixture population and compute T3.
///
/// Population: the 48 single-lint clippy fixtures + 28 composite multi-lint
/// spans + (feature-gated) the perf fixture set. Composite spans carry no
/// single label; they join purely through their top-1 fix outcome.
#[must_use]
pub fn run_jury_poc(matcher: &mut LatentMatcher, oracle: &ClippyOracle) -> JuryPocReport {
    let mut rows: Vec<JuryRow> = Vec::new();

    // Tags repeat across fixtures (multiple spans pin the same rule); the
    // occurrence suffix keeps the unstable-row listing unambiguous.
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut push = |matcher: &mut LatentMatcher,
                    tag: &str,
                    domain: DomainId,
                    label: &str,
                    code: &str,
                    rows: &mut Vec<JuryRow>| {
        let Some((pool, ranked)) = matcher.pool_and_ranked(domain, code) else {
            return;
        };
        let signals = disagreement_signals(&pool, &ranked);
        let outcome = fix_outcome(oracle, domain, code, &signals.top1_rule);
        let n = seen.entry(tag.to_string()).or_insert(0);
        *n += 1;
        rows.push(JuryRow {
            tag: format!("{tag}#{n}"),
            domain,
            label: label.to_string(),
            signals,
            outcome,
        });
    };

    #[cfg(feature = "clippy_lints")]
    for (lint, code) in super::eval_fixtures_clippy::CLIPPY_SPANS {
        push(matcher, lint, DomainId::Clippy, lint, code, &mut rows);
    }
    for (tag, code) in super::eval_fixtures_composite::COMPOSITE_SPANS {
        push(matcher, tag, DomainId::Clippy, "composite", code, &mut rows);
    }
    #[cfg(feature = "rust_perf")]
    for (rule, code) in super::eval_fixtures_perf::PERF_SPANS {
        push(matcher, rule, DomainId::Perf, rule, code, &mut rows);
    }

    finalize_report(rows)
}

fn finalize_report(rows: Vec<JuryRow>) -> JuryPocReport {
    let verdict_rows = rows.iter().filter(|r| r.outcome.is_verdict()).count();
    let unstable_rows = rows.iter().filter(|r| r.outcome.is_unstable()).count();

    let verdicts: Vec<&JuryRow> = rows.iter().filter(|r| r.outcome.is_verdict()).collect();
    let mut rhos = Vec::new();
    if unstable_rows > 0 && unstable_rows < verdict_rows {
        let ys: Vec<f64> = verdicts.iter().map(|r| f64::from(u8::from(r.outcome.is_unstable()))).collect();
        let spreads: Vec<f64> = verdicts.iter().map(|r| r.signals.pool_spread as f64).collect();
        let margins: Vec<f64> = verdicts.iter().map(|r| f64::from(r.signals.rerank_margin)).collect();
        rhos.push(("pool_spread", spearman_rho(&spreads, &ys)));
        rhos.push(("rerank_margin", spearman_rho(&margins, &ys)));
    }

    let max_spread = verdicts.iter().map(|r| r.signals.pool_spread).max().unwrap_or(0);
    let spread_sweep: Vec<Contingency> = (2..=max_spread)
        .map(|t| {
            contingency_at(&verdicts, |r| r.signals.pool_spread >= t, t as f64)
        })
        .collect();

    let mut margins: Vec<f64> = verdicts
        .iter()
        .filter(|r| r.signals.rerank_margin.is_finite())
        .map(|r| f64::from(r.signals.rerank_margin))
        .collect();
    margins.sort_by(|a, b| a.total_cmp(b));
    margins.dedup();
    let margin_sweep: Vec<Contingency> = margins
        .iter()
        .map(|&t| contingency_at(&verdicts, |r| f64::from(r.signals.rerank_margin) < t, t))
        .collect();

    JuryPocReport {
        rows,
        rhos,
        spread_sweep,
        margin_sweep,
        verdict_rows,
        unstable_rows,
    }
}

/// The G2 question: does ANY tested threshold capture ≥ 70% of unstable spans
/// while flagging ≤ 40% of verdict spans?
#[must_use]
pub fn best_g2_threshold(sweep: &[Contingency]) -> Option<&Contingency> {
    sweep
        .iter()
        .filter(|c| c.capture() >= 0.70 && c.flagged_frac() <= 0.40)
        .max_by(|a, b| {
            a.capture()
                .partial_cmp(&b.capture())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Human-readable report (the `--nocapture` output of the POC test).
pub fn print_jury_report(report: &JuryPocReport) {
    println!("── Issue 036 jury-disagreement POC ──");
    println!(
        "population: {} measured, {} verdict-bearing, {} unstable (excluded: NoFix/OracleDown)",
        report.rows.len(),
        report.verdict_rows,
        report.unstable_rows
    );
    let count = |f: fn(&FixOutcome) -> bool| report.rows.iter().filter(|r| f(&r.outcome)).count();
    println!(
        "outcomes: resolved={} nofix={} fixture_broken={} compile_fail={} lint_unresolved={} oracle_down={}",
        count(|o| *o == FixOutcome::Resolved),
        count(|o| *o == FixOutcome::NoFix),
        count(|o| *o == FixOutcome::FixtureBroken),
        count(|o| *o == FixOutcome::CompileFail),
        count(|o| *o == FixOutcome::LintUnresolved),
        count(|o| *o == FixOutcome::OracleDown),
    );
    for r in report.rows.iter().filter(|r| r.outcome.is_unstable()) {
        println!(
            "  unstable: {:<32} top1={:<28} spread={:<3} margin={:>9.3} -> {:?}",
            r.tag, r.signals.top1_rule, r.signals.pool_spread, r.signals.rerank_margin, r.outcome
        );
    }
    for (name, rho) in &report.rhos {
        println!("ρ({name}, unstable) = {rho:+.3}");
    }
    println!("── spread sweep (flag: pool_spread ≥ t) ──");
    for c in &report.spread_sweep {
        println!(
            "t={:>4.1}  flagged {:>3}/{:<3} ({: >5.1}%)  capture {:>2}/{:<2} ({: >5.1}%)",
            c.threshold,
            c.flagged,
            c.total,
            c.flagged_frac() * 100.0,
            c.unstable_flagged,
            c.unstable_total,
            c.capture() * 100.0
        );
    }
    println!("── margin sweep (flag: rerank_margin < t) ──");
    for c in &report.margin_sweep {
        println!(
            "t={:>9.3}  flagged {:>3}/{:<3} ({: >5.1}%)  capture {:>2}/{:<2} ({: >5.1}%)",
            c.threshold,
            c.flagged,
            c.total,
            c.flagged_frac() * 100.0,
            c.unstable_flagged,
            c.unstable_total,
            c.capture() * 100.0
        );
    }
    match best_g2_threshold(&report.spread_sweep) {
        Some(c) => println!(
            "G2 (spread): PASS at t={} — capture {:.1}%, flagged {:.1}%",
            c.threshold,
            c.capture() * 100.0,
            c.flagged_frac() * 100.0
        ),
        None => println!("G2 (spread): no threshold meets ≥70% capture at ≤40% flagged"),
    }
    match best_g2_threshold(&report.margin_sweep) {
        Some(c) => println!(
            "G2 (margin): PASS at t={:.3} — capture {:.1}%, flagged {:.1}%",
            c.threshold,
            c.capture() * 100.0,
            c.flagged_frac() * 100.0
        ),
        None => println!("G2 (margin): no threshold meets ≥70% capture at ≤40% flagged"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spearman_is_one_on_perfectly_monotone_pairs() {
        let xs = [1.0, 2.0, 3.0, 4.0];
        let ys = [10.0, 20.0, 30.0, 44.0];
        assert!((spearman_rho(&xs, &ys) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn spearman_is_minus_one_on_perfectly_antitone_pairs() {
        let xs = [1.0, 2.0, 3.0, 4.0];
        let ys = [44.0, 30.0, 20.0, 10.0];
        assert!((spearman_rho(&xs, &ys) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn spearman_handles_massive_ties_in_y_without_collapsing() {
        // Binary y with a real effect: every high-x row is unstable.
        let xs: Vec<f64> = (0..20).map(f64::from).collect();
        let ys: Vec<f64> = (0..20).map(|i| f64::from(u8::from(i >= 12))).collect();
        let rho = spearman_rho(&xs, &ys);
        assert!(rho > 0.7, "a perfect step function must rank-correlate: {rho}");
    }

    #[test]
    fn spearman_is_zero_on_constant_y() {
        let xs = [1.0, 5.0, 3.0];
        let ys = [0.0, 0.0, 0.0];
        let rho = spearman_rho(&xs, &ys);
        assert!(rho.abs() < 1e-12, "constant y must give exactly 0: {rho}");
    }

    #[test]
    fn contingency_math_is_exact() {
        let rows = [
            row("a", 8, 0.1, FixOutcome::Resolved),
            row("b", 8, 0.1, FixOutcome::LintUnresolved),
            row("c", 2, 5.0, FixOutcome::Resolved),
            row("d", 2, 5.0, FixOutcome::Resolved),
            row("e", 8, 0.1, FixOutcome::NoFix), // excluded: not a verdict
        ];
        let verdicts: Vec<&JuryRow> = rows.iter().filter(|r| r.outcome.is_verdict()).collect();
        let c = contingency_at(&verdicts, |r| r.signals.pool_spread >= 8, 8.0);
        assert_eq!(c.total, 4);
        assert_eq!(c.flagged, 2);
        assert_eq!(c.unstable_total, 1);
        assert_eq!(c.unstable_flagged, 1);
        assert!((c.capture() - 1.0).abs() < 1e-9);
        assert!((c.flagged_frac() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn g2_picks_the_meeting_threshold() {
        let mut sweep = Vec::new();
        for t in 2..=8u32 {
            sweep.push(Contingency {
                threshold: f64::from(t),
                total: 10,
                flagged: 8,          // 80% flagged — fails the ≤40% side
                unstable_total: 5,
                unstable_flagged: 5,
            });
        }
        sweep.push(Contingency {
            threshold: 9.0,
            total: 10,
            flagged: 3, // 30% flagged
            unstable_total: 5,
            unstable_flagged: 4, // 80% capture
        });
        let best = best_g2_threshold(&sweep).expect("t=9 must qualify");
        assert!((best.threshold - 9.0).abs() < 1e-12);
    }

    fn row(tag: &str, spread: usize, margin: f32, outcome: FixOutcome) -> JuryRow {
        JuryRow {
            tag: tag.to_string(),
            domain: DomainId::Clippy,
            label: tag.to_string(),
            signals: DisagreementSignals {
                pool_len: 12,
                pool_spread: spread,
                instantiating: 1,
                rerank_margin: margin,
                top1_rule: "x".to_string(),
            },
            outcome,
        }
    }
}
