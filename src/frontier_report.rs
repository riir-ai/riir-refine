//! Issue 089 T1 + T2 — the learnability-frontier band report (REPORT-FIRST,
//! gate-later per the R135/Bench 047 law: at real pool sparsity selection
//! sophistication measured ≤0.04pp — evidence volume is the bottleneck, so
//! nothing here GATES anything yet; it makes the band measurable).
//!
//! Pure arithmetic over per-rule outcome counters `(rule, successes, fails)`:
//!
//! - `p̂ = r/n` — the empirical resolve rate;
//! - `LCB₅ = best_belief_score(r, f, 0.05)` — the ε-quantile of
//!   `Beta(1+r, 1+f)`, the conservative lower bound on the true rate (the
//!   same primitive the shipped `BetaPosterior` selection consumes via
//!   [`crate::traj_store::shape_conditional_score`], at the frontier's
//!   documented ε=0.05 — `FRONTIER_EPSILON` in katgpt-core, not the
//!   strategy-ordering 0.32);
//! - **band** = `0 < p̂ ≤ 0.5` (the admission band — hard-but-learnable);
//! - **saturated** = `p̂ = 1 ∧ n ≥ W=50` — the demote-to-suggest-only
//!   CANDIDATE list. Honest proxy: the store's counters are cumulative
//!   (per-attempt ORDER is unreconstructible — the Issue 91 T1 mining
//!   verdict), so "W trailing perfect encounters" is measured as its
//!   conservative superset `p̂=1 ∧ n≥W` (all-time-perfect with deep
//!   evidence ⟹ the trailing W were perfect). A rule that failed once
//!   long ago then won 100 in a row is MISSED — conservative is correct
//!   for a report-only list; deletion is never a report decision.
//!
//! The CLI half (bin/heal.rs `--frontier-report`) feeds `learn_rows()` +
//! `score_runs()` from the trajectory store into [`summarize`]; this module
//! stays store-free so the stats are unit-testable at default features.

use katgpt_core::best_belief_score;

/// The frontier LCB ε — 0.05, the 95% lower bound (katgpt-core's documented
/// `FRONTIER_EPSILON` convention; NOT the 0.32 strategy-ordering ε).
pub const FRONTIER_LCB_EPSILON: f32 = 0.05;

/// The admission band's upper edge (inclusive): `0 < p̂ ≤ 0.5`.
pub const ADMISSION_BAND_MAX_P_HAT: f32 = 0.5;

/// The saturation window W — trailing oracle encounters at `p̂=1` that make a
/// rule a demote-to-suggest-only CANDIDATE (report-only; never deletes).
pub const SATURATION_WINDOW: u32 = 50;

/// One report row: a rule's frontier stats.
#[derive(Clone, Debug, PartialEq)]
pub struct FrontierRow {
    pub rule: String,
    pub r: u32,
    pub f: u32,
    pub n: u32,
    pub p_hat: f32,
    pub lcb5: f32,
    pub class: BandClass,
}

/// The band classification (the T4 admission vocabulary, measured not wired).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BandClass {
    /// `0 < p̂ ≤ 0.5` — the admission band: hard but learnable.
    InBand = 0,
    /// `p̂ = 0` with `n ≥ 1` — the expressiveness-gap fork (T4: fires>0 ∧
    /// resolves=0 → L3 ruliology or `decline`, never admission).
    BelowBand = 1,
    /// `0.5 < p̂ < 1`, or `p̂ = 1` with thin evidence (`n < W`).
    AboveBand = 2,
    /// `p̂ = 1 ∧ n ≥ W` — the saturation demotion CANDIDATE list.
    Saturated = 3,
    /// `n = 0` — a learn row exists but no verdicts landed (unseen).
    Unseen = 4,
}

impl BandClass {
    /// The stable render label (report tables + tests needle on it).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::InBand => "band",
            Self::BelowBand => "below",
            Self::AboveBand => "above",
            Self::Saturated => "saturated",
            Self::Unseen => "unseen",
        }
    }
}

/// The score-history context line (T1's "+ score history" half) — plain
/// fields so this module stays free of the `latent_retrieval`-gated store
/// types; the CLI adapter fills it from `TrajStore::score_runs()`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScoreContext {
    pub runs: usize,
    pub latest_generation: u8,
    pub latest_heal_rate: f32,
}

/// The report's aggregate verdict over a scanned population.
#[derive(Clone, Debug, PartialEq)]
pub struct FrontierSummary {
    /// Deterministically ordered rows (class rank, then n desc, then rule).
    pub rows: Vec<FrontierRow>,
    /// Rows scanned — the population the verdict was computed over (the
    /// green-zero defense: the CLI REFUSES at 0, never reports "healthy").
    pub population: usize,
    /// Entries with `n ≥ 1` (p̂ defined).
    pub seen: usize,
    /// Entries in the admission band.
    pub band_count: usize,
    /// `band_count / seen` — the T2 band-mass health metric.
    pub band_mass: f32,
}

/// Classify + order the per-rule outcome counters into the frontier summary.
///
/// Input is `(rule, successes, fails)` triples — exactly what
/// `LearnRowDoc { lint_key, successes, fails }` carries. Deterministic:
/// class rank first (band is the actionable head), then evidence depth
/// `n` descending, then rule name ascending (the canonical-rank lesson —
/// never let a backing map's order decide a rank).
#[must_use]
pub fn summarize(entries: &[(String, u32, u32)]) -> FrontierSummary {
    let mut rows: Vec<FrontierRow> = entries
        .iter()
        .map(|(rule, r, f)| {
            let n = r.saturating_add(*f);
            #[allow(clippy::cast_precision_loss)] // rate over u32 counters — the report's quantity
            let (p_hat, seen) = if n == 0 {
                (0.0_f32, false)
            } else {
                (*r as f32 / n as f32, true)
            };
            let class = if !seen {
                BandClass::Unseen
            } else if p_hat == 0.0 {
                BandClass::BelowBand
            } else if p_hat <= ADMISSION_BAND_MAX_P_HAT {
                BandClass::InBand
            } else if p_hat < 1.0 || n < SATURATION_WINDOW {
                BandClass::AboveBand
            } else {
                BandClass::Saturated
            };
            FrontierRow {
                rule: rule.clone(),
                r: *r,
                f: *f,
                n,
                p_hat,
                lcb5: best_belief_score(*r, *f, FRONTIER_LCB_EPSILON),
                class,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.class
            .cmp(&b.class)
            .then(b.n.cmp(&a.n))
            .then(a.rule.cmp(&b.rule))
    });

    let seen = rows.iter().filter(|row| row.n > 0).count();
    let band_count = rows
        .iter()
        .filter(|row| row.class == BandClass::InBand)
        .count();
    #[allow(clippy::cast_precision_loss)] // fraction over usize counts
    let band_mass = if seen == 0 {
        0.0
    } else {
        band_count as f32 / seen as f32
    };
    FrontierSummary {
        population: rows.len(),
        rows,
        seen,
        band_count,
        band_mass,
    }
}

/// Render the summary as the `--frontier-report` text (house style:
/// `String` + `fmt::Write`, inline args, deterministic sections).
///
/// An EMPTY population renders the loud `EMPTY` marker — a report over
/// zero rows must never read as "measured, all healthy" (the green-zero
/// law; the CLI half additionally refuses with exit 1).
#[must_use]
pub fn render(summary: &FrontierSummary, score: Option<&ScoreContext>) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(1024);

    if summary.population == 0 {
        let _ = writeln!(
            out,
            "frontier-report: EMPTY — 0 learn rows scanned; nothing was measured (refusing to report health over an empty population)"
        );
        return out;
    }

    let saturated: Vec<&FrontierRow> = summary
        .rows
        .iter()
        .filter(|row| row.class == BandClass::Saturated)
        .collect();
    let below: usize = summary
        .rows
        .iter()
        .filter(|row| row.class == BandClass::BelowBand)
        .count();
    let above: usize = summary
        .rows
        .iter()
        .filter(|row| row.class == BandClass::AboveBand)
        .count();
    let unseen: usize = summary
        .rows
        .iter()
        .filter(|row| row.class == BandClass::Unseen)
        .count();

    let _ = writeln!(
        out,
        "frontier band report (Issue 089 T1+T2 — report-first, gates nothing)"
    );
    let _ = writeln!(
        out,
        "population: {population} rules scanned | seen (n>=1): {seen} | band (0<p̂<=0.5): {band} ({pct:.1}% band-mass) | below: {below} | above: {above} | saturated: {sat} | unseen: {unseen}",
        population = summary.population,
        seen = summary.seen,
        band = summary.band_count,
        pct = summary.band_mass * 100.0,
        below = below,
        above = above,
        sat = saturated.len(),
        unseen = unseen,
    );
    if let Some(ctx) = score {
        let _ = writeln!(
            out,
            "score history: {runs} runs | latest generation {gen} | latest heal rate {rate:.1}%",
            runs = ctx.runs,
            gen = ctx.latest_generation,
            rate = ctx.latest_heal_rate * 100.0,
        );
    }
    if !saturated.is_empty() {
        // T2: saturation demotion is REPORT-ONLY — a candidate list, never
        // a deletion (coverage is a separate decision).
        let _ = writeln!(
            out,
            "saturation candidates (p̂=1 ∧ n>={SATURATION_WINDOW}; conservative proxy — order unrecoverable, Issue 91 T1; suggest-only CANDIDATES, never deleted):",
        );
        for row in saturated {
            let _ = writeln!(out, "  {}  r={} n={}", row.rule, row.r, row.n);
        }
    }
    let _ = writeln!(
        out,
        "per-rule (LCB₅ = Beta(1+r,1+f) {FRONTIER_LCB_EPSILON} quantile; sorted band-first, then evidence depth):",
    );
    let _ = writeln!(
        out,
        "  {:<34} {:>5} {:>5} {:>5} {:>6} {:>7}  class",
        "rule", "r", "f", "n", "p̂", "LCB₅"
    );
    for row in &summary.rows {
        let _ = writeln!(
            out,
            "  {:<34} {:>5} {:>5} {:>5} {:>6.3} {:>7.3}  {}",
            row.rule,
            row.r,
            row.f,
            row.n,
            row.p_hat,
            row.lcb5,
            row.class.label(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixture population floor — the tests below assert against a
    /// KNOWN population; if fixture edits shrink it below the floor the
    /// asserts below stop meaning what they say (population floors, not
    /// ceilings — the sweep convention).
    const FIXTURE_FLOOR: usize = 12;

    fn fixtures() -> Vec<(String, u32, u32)> {
        // Covers every band class + both saturation edges + the proxy miss.
        vec![
            // band: hard-but-learnable (the admission band's exact interior)
            ("band_mid".into(), 3, 3),          // p̂=0.5 band edge (inclusive)
            ("band_low".into(), 1, 9),          // p̂=0.1
            ("band_thin".into(), 1, 1),         // p̂=0.5, thin
            ("band_deep".into(), 20, 30),       // p̂=0.4, deep — band head
            // below: expressiveness gap (all-fail)
            ("gap_all_fail".into(), 0, 12),
            // above: mid rates + thin-perfect
            ("above_mid".into(), 9, 1),         // p̂=0.9
            ("perfect_thin".into(), 10, 0),     // p̂=1, n=10 < W → above
            ("perfect_edge".into(), 49, 0),     // p̂=1, n=49 < W → above (edge)
            // saturated: all-time-perfect with deep evidence
            ("saturated_edge".into(), 50, 0),   // p̂=1, n=50 = W → saturated (edge)
            ("saturated_deep".into(), 120, 0),
            // the proxy MISS: failed once long ago, then a long win streak
            // — trailing-50 IS perfect but p̂<1, so NOT a candidate
            // (conservative by construction; documented honesty).
            ("streak_after_early_fail".into(), 99, 1),
            // unseen: a learn row with no verdicts
            ("never_verdicted".into(), 0, 0),
        ]
    }

    #[test]
    fn population_floor_and_classification() {
        let entries = fixtures();
        let s = summarize(&entries);
        assert!(
            s.population >= FIXTURE_FLOOR,
            "fixture population {population} shrank below the floor {FIXTURE_FLOOR}",
            population = s.population,
        );
        assert_eq!(s.population, entries.len());
        // seen excludes only the unseen row.
        assert_eq!(s.seen, entries.len() - 1);
        // Exactly the four band_* fixtures classify in-band.
        let band: Vec<&str> = s
            .rows
            .iter()
            .filter(|r| r.class == BandClass::InBand)
            .map(|r| r.rule.as_str())
            .collect();
        assert_eq!(band.len(), 4, "in-band set: {band:?}");
        for name in ["band_mid", "band_low", "band_thin", "band_deep"] {
            assert!(band.contains(&name), "{name} missing from band set");
        }
    }

    #[test]
    fn band_mass_arithmetic() {
        let s = summarize(&fixtures());
        // 4 band of 11 seen (12 rows minus the 1 unseen).
        assert!((s.band_mass - 4.0 / 11.0).abs() < 1e-6, "got {m}", m = s.band_mass);
        assert_eq!(s.band_count, 4);
    }

    #[test]
    fn saturation_edges_and_proxy_miss() {
        let s = summarize(&fixtures());
        let saturated: Vec<&str> = s
            .rows
            .iter()
            .filter(|r| r.class == BandClass::Saturated)
            .map(|r| r.rule.as_str())
            .collect();
        assert_eq!(saturated, vec!["saturated_deep", "saturated_edge"]);
        // n=49 perfect stays ABOVE (thin evidence), n=50 crosses.
        let thin = s.rows.iter().find(|r| r.rule == "perfect_edge").unwrap();
        assert_eq!(thin.class, BandClass::AboveBand);
        // The early-fail streak (p̂=0.99) is above-band, NOT a candidate —
        // the proxy's documented miss.
        let streak = s
            .rows
            .iter()
            .find(|r| r.rule == "streak_after_early_fail")
            .unwrap();
        assert_eq!(streak.class, BandClass::AboveBand);
    }

    #[test]
    fn lcb5_bounds_and_confidence_monotonicity() {
        let s = summarize(&fixtures());
        for row in &s.rows {
            if row.p_hat > 0.0 {
                assert!(
                    row.lcb5 <= row.p_hat + 1e-6,
                    "{rule}: LCB₅ {lcb} exceeds p̂ {p}",
                    rule = row.rule,
                    lcb = row.lcb5,
                    p = row.p_hat,
                );
            } else {
                // Boundary honesty: at p̂=0 (all-fail or unseen) the
                // Beta(1, 1+f) prior mass keeps the ε-quantile POSITIVE —
                // it cannot exceed ε (Beta(1,1)=U(0,1) sets the ceiling
                // at exactly ε, deeper evidence only shrinks it).
                assert!(
                    row.lcb5 <= FRONTIER_LCB_EPSILON + 1e-6,
                    "{rule}: LCB₅ {lcb} exceeds the prior-mass ceiling {eps}",
                    rule = row.rule,
                    lcb = row.lcb5,
                    eps = FRONTIER_LCB_EPSILON,
                );
            }
        }
        // Confidence grows with evidence at the same perfect rate: the
        // deep-perfect LCB beats the thin-perfect one (the whole point of
        // an LCB over a raw rate).
        let thin = s.rows.iter().find(|r| r.rule == "perfect_thin").unwrap();
        let deep = s.rows.iter().find(|r| r.rule == "saturated_deep").unwrap();
        assert!(thin.lcb5 < deep.lcb5);
    }

    #[test]
    fn ordering_is_deterministic_and_canonical() {
        let a = summarize(&fixtures());
        let b = summarize(&fixtures());
        assert_eq!(a.rows, b.rows, "summarize must be deterministic");
        // Class-rank head: the first four rows are exactly the band set,
        // deepest evidence first (n desc), name ascending on ties.
        let head: Vec<&str> = a.rows.iter().take(4).map(|r| r.rule.as_str()).collect();
        assert_eq!(head, vec!["band_deep", "band_low", "band_mid", "band_thin"]);
    }

    #[test]
    fn empty_population_is_loud_never_green() {
        let s = summarize(&[]);
        assert_eq!(s.population, 0);
        // Bit-identity IS the assertion: band-mass over an empty seen set is
        // exactly +0.0 (the float_cmp-idiom — collapse_monitor precedent).
        assert_eq!(s.band_mass.to_bits(), 0.0_f32.to_bits());
        let text = render(&s, None);
        assert!(
            text.contains("EMPTY") && text.contains("0 learn rows"),
            "empty render must carry the loud EMPTY marker: {text}"
        );
    }

    #[test]
    fn render_needles() {
        let s = summarize(&fixtures());
        let text = render(
            &s,
            Some(&ScoreContext {
                runs: 7,
                latest_generation: 4,
                latest_heal_rate: 0.962,
            }),
        );
        assert!(text.contains("band-mass"), "band-mass line: {text}");
        assert!(text.contains("36.4%"), "4/11 band-mass pct: {text}");
        assert!(text.contains("saturation candidates"), "{text}");
        assert!(text.contains("score history: 7 runs"), "{text}");
        assert!(text.contains("generation 4"), "{text}");
        assert!(text.contains("96.2%"), "latest heal rate pct: {text}");
        // Every fixture rule appears in the table.
        for name in [
            "band_deep",
            "gap_all_fail",
            "perfect_edge",
            "saturated_edge",
            "streak_after_early_fail",
            "never_verdicted",
        ] {
            assert!(text.contains(name), "{name} missing from table");
        }
    }
}
