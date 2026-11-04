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
//!
//! T3 (the `--misses` generalization): [`domain_queue`] +
//! [`render_domain_queue`] extend the Plan 091 frontier ordering beyond its
//! `rustc_errors`-only roots — one frontier-ordered RULE queue per retrieval
//! domain (clippy/perf/kernel/sec/docker), attribution by corpus membership.
//! The per-CODE rustc queue is kept verbatim (additive, not a replacement);
//! the per-RULE band above stays this module's admission instrument.

use std::collections::BTreeMap;

use katgpt_core::best_belief_score;
use serde::{Deserialize, Serialize};

use crate::dispersion::{PoolEntry, Regime, from_counts, frontier_order};

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

/// Issue 139 T3 — the censoring disclosure on [`FrontierSummary::band_mass`].
///
/// `band_mass` is a FILTERED readout: it is conditioned on `C` = "a verdict
/// landed" (`n ≥ 1`), and the unseen rows are `¬C`. Three quantities, from
/// the same counts:
///
/// - the censoring decomposition `population_mass = P(C) · E[band | C]` —
///   the band mass over the whole scanned population;
/// - `bias_bound = filter_bias_bound(½, P(¬C)) = P(¬C)` — the distribution-
///   free budget (katgpt-rs Issue 913 T2) on how far `E[band | C]` can sit
///   from the population's band rate, the band indicator centred on `[0, 1]`
///   so `|R| ≤ Γ = ½`;
/// - the exact count `hull` `[band, band + unseen] / population` — the
///   tight interval for THIS population if every unseen row resolved either
///   way. It is never wider than the budget (`max(b, 1−b) · P(¬C) ≤ P(¬C)`).
#[cfg(feature = "noise_weighted_evidence")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterDisclosure {
    /// `P(¬C)` — the unseen fraction of the scanned population.
    pub p_not_c: f32,
    /// `P(C) · E[band | C]` — band mass over the whole population.
    pub population_mass: f32,
    /// `filter_bias_bound(½, P(¬C))`.
    pub bias_bound: f32,
    /// `[band / population, (band + unseen) / population]`.
    pub hull: (f32, f32),
}

/// Compute the [`FilterDisclosure`] for a summary. `None` for an empty
/// population (the green-zero law: nothing was measured, so nothing is
/// disclosed either).
#[cfg(feature = "noise_weighted_evidence")]
#[must_use]
pub fn filter_disclosure(summary: &FrontierSummary) -> Option<FilterDisclosure> {
    let population = summary.population;
    if population == 0 {
        return None;
    }
    let unseen = population - summary.seen;
    #[allow(clippy::cast_precision_loss)] // fractions over usize counts
    let (pop, unseen_f, band) = (population as f32, unseen as f32, summary.band_count as f32);
    let p_not_c = unseen_f / pop;
    Some(FilterDisclosure {
        p_not_c,
        population_mass: (1.0 - p_not_c) * summary.band_mass,
        bias_bound: katgpt_core::grouped_evidence::filter_bias_bound(0.5, p_not_c),
        hull: (band / pop, (band + unseen_f) / pop),
    })
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
    #[cfg(feature = "noise_weighted_evidence")]
    if let Some(d) = filter_disclosure(summary) {
        let _ = writeln!(
            out,
            "filter disclosure (Issue 139 T3): band-mass is conditioned on seen — P(unseen) {pnc:.3} | population band-mass = P(seen)·band-mass = {pm:.1}% | bias budget 2Γ·P(unseen) (Γ=½) ±{bb:.1}pp | exact hull [{lo:.1}%, {hi:.1}%]",
            pnc = d.p_not_c,
            pm = d.population_mass * 100.0,
            bb = d.bias_bound * 100.0,
            lo = d.hull.0 * 100.0,
            hi = d.hull.1 * 100.0,
        );
    }
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

// ── Issue 089 T3: the per-domain frontier rule queue (`--misses` generalization) ──

/// One per-domain queue row: a corpus rule with observed store outcomes,
/// frontier-ordered (the Plan 091 control law applied at rule granularity).
#[derive(Clone, Debug, PartialEq)]
pub struct DomainQueueRow {
    pub rank: usize,
    pub rule: String,
    pub kept: usize,
    pub reverted: usize,
    pub entropy: f64,
    pub regime: Regime,
}

/// Build one domain's frontier-ordered rule queue from its corpus rule ids
/// and the store's per-rule `(kept, reverted)` counters.
///
/// Population = corpus rules that have store rows (**observed only** — an
/// unobserved rule has no queue entry; [`render_domain_queue`] carries the
/// unobserved count as a coverage line instead of hundreds of empty rows).
/// Pool outcomes are `from_counts(kept, reverted, 0)` — the `Missed` class
/// exists only where a miss FILE exists (the rustc queue); here a no-candidate
/// lint lands in the store as a fail, reading `regenerate` (the corpus-gap
/// signal). Ordering is the shared control law: informative (mixed evidence)
/// first, then empty (a row with zero verdicts — one probe is cheap), then
/// regenerate, saturated last.
///
/// Deterministic: `frontier_order` ties break by rule id (the corpus house
/// rule). Store keys outside the corpus are ignored — attribution is the
/// CLI's job (it knows which domains are enabled).
#[must_use]
pub fn domain_queue(
    corpus_rules: &[String],
    outcomes: &BTreeMap<String, (usize, usize)>,
) -> Vec<DomainQueueRow> {
    let pool: Vec<PoolEntry> = corpus_rules
        .iter()
        .filter_map(|rule| {
            outcomes.get(rule).map(|(kept, reverted)| {
                PoolEntry::new(rule.clone(), from_counts(*kept, *reverted, 0))
            })
        })
        .collect();
    frontier_order(&pool)
        .into_iter()
        .enumerate()
        .map(|(i, verdict)| DomainQueueRow {
            rank: i + 1,
            kept: outcomes.get(verdict.id.as_str()).map_or(0, |(k, _)| *k),
            reverted: outcomes.get(verdict.id.as_str()).map_or(0, |(_, r)| *r),
            rule: verdict.id,
            entropy: verdict.entropy,
            regime: verdict.regime,
        })
        .collect()
}

/// Render one domain's section for the `--misses` output (house style:
/// `String` + `fmt::Write`, deterministic). An all-empty domain renders its
/// loud `(empty …)` marker — exit 0, the rustc empty-queue precedent (a
/// quiet domain is a measurement, not an error).
#[must_use]
pub fn render_domain_queue(domain: &str, rows: &[DomainQueueRow], corpus_size: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(256 + rows.len() * 96);

    if rows.is_empty() {
        let _ = writeln!(
            out,
            "# {domain} — frontier rule queue (0 observed / {corpus_size} corpus rules)\n\
             (empty: no store row attributes to this domain's corpus)\n"
        );
        return out;
    }

    let _ = writeln!(
        out,
        "# {domain} — frontier rule queue ({} observed / {corpus_size} corpus rules)\n",
        rows.len()
    );
    out.push_str(
        "Plan 091 control law at rule granularity: informative pools first (mixed \
         kept/reverted — resolving those converts the most per corpus pass), then \
         empty (a row with zero verdicts), regenerate (all-fail: the fix family is \
         wrong — author/refine here), saturated last.\n\n",
    );
    out.push_str("| rank | rule | kept | reverted | entropy | regime |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for r in rows {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {:.3} | {} |",
            r.rank,
            r.rule,
            r.kept,
            r.reverted,
            r.entropy,
            r.regime.label()
        );
    }
    let unobserved = corpus_size.saturating_sub(rows.len());
    let _ = writeln!(
        out,
        "\n(corpus coverage: {} of {corpus_size}; {unobserved} rule(s) unobserved — no store rows)",
        rows.len()
    );
    out
}

// ── Issue 089 T5: provenance weighting — pair population + yield split ──

/// One provenance class's pair-population row (the `pair:` namespace —
/// construction pairs today, organic pairs the day a miner lands).
#[derive(Clone, Debug, PartialEq)]
pub struct ProvenanceRow {
    pub provenance: String,
    pub pairs: usize,
    /// `pairs / total` — the class's share of the pair namespace.
    pub share: f32,
    /// [`crate::frontier_pairs::provenance_weight`] of the class.
    pub weight: f32,
    /// `pairs × weight` — the class's priced contribution to weighted yield.
    pub weighted: f32,
}

/// Aggregate pair counts per provenance class (Issue 089 T5).
/// Deterministic: known classes in canonical order (organic first — the
/// class the paper prices 1.0 — then construction), then any unrecognized
/// tag ascending (counted loud at weight 0.0, never silently dropped).
#[must_use]
pub fn provenance_summary(pair_counts: &BTreeMap<String, usize>) -> Vec<ProvenanceRow> {
    let total: usize = pair_counts.values().sum();
    // Canonical classes first (organic — priced 1.0 — then construction),
    // then any unrecognized tag ascending (counted loud at weight 0.0,
    // never silently dropped).
    let mut ordered: Vec<String> = [
        crate::frontier_pairs::PROVENANCE_ORGANIC,
        crate::frontier_pairs::PROVENANCE,
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    let mut unknown: Vec<String> = pair_counts
        .keys()
        .filter(|k| !ordered.iter().any(|c| c == *k))
        .cloned()
        .collect();
    unknown.sort();
    ordered.append(&mut unknown);
    ordered
        .into_iter()
        .map(|p| {
            let pairs = pair_counts.get(&p).copied().unwrap_or(0);
            let weight = crate::frontier_pairs::provenance_weight(&p);
            #[allow(clippy::cast_precision_loss)]
            // fractions over usize counts — the report's quantity
            let share = if total == 0 {
                0.0
            } else {
                pairs as f32 / total as f32
            };
            #[allow(clippy::cast_precision_loss)] // priced count — usize body, f32 price
            let weighted = pairs as f32 * weight;
            ProvenanceRow {
                pairs,
                share,
                weight,
                weighted,
                provenance: p,
            }
        })
        .collect()
}

/// Render the pair-provenance section (the `--frontier-report` tail). An
/// empty namespace renders its loud empty marker — a quiet namespace is a
/// measurement, not an error (the `render_domain_queue` precedent).
#[must_use]
pub fn render_provenance_section(rows: &[ProvenanceRow]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(256 + rows.len() * 96);
    let total: usize = rows.iter().map(|r| r.pairs).sum();
    if total == 0 {
        let _ = writeln!(
            out,
            "pair provenance (Issue 089 T5): EMPTY — 0 pair rows in this namespace (construction pairs import via the rustc_errors A/B; an organic miner does not exist yet)\n"
        );
        return out;
    }
    let _ = writeln!(
        out,
        "pair provenance (Issue 089 T5 — organic=1.0, construction/synthetic=ε={}, unknown=0.0 fail-closed; the weight prices yield arithmetic, gates nothing):",
        crate::frontier_pairs::SYNTHETIC_PAIR_WEIGHT
    );
    let _ = writeln!(
        out,
        "  {:<16} {:>6} {:>8} {:>7} {:>9}",
        "provenance", "pairs", "share", "weight", "weighted"
    );
    for r in rows {
        let _ = writeln!(
            out,
            "  {:<16} {:>6} {:>7.1}% {:>7.2} {:>9.2}",
            r.provenance,
            r.pairs,
            r.share * 100.0,
            r.weight,
            r.weighted
        );
    }
    let _ = writeln!(
        out,
        "  (never inject failures on purpose — a fabricated row priced 1.0 is the worst lie in the table)"
    );
    out
}

/// One outcome-feed's provenance split row (Issue 089 T5). The feed split
/// measures the two evidence pipelines SEPARATELY — per-feed band-mass is
/// what T4's admission gate will actually need (a band living only in the
/// synthetic feed is a curriculum artifact, not a learnable corpus).
#[derive(Clone, Debug, PartialEq)]
pub struct ProvenanceYieldRow {
    /// `"organic"` (real-repo heal outcomes) or `"synthetic"`.
    pub feed: &'static str,
    /// Rules in this feed (learn-row keys attributed by shape).
    pub rules: usize,
    /// Rules with `n ≥ 1` in this feed.
    pub seen: usize,
    /// In-band rules (`0 < p̂ ≤ 0.5`) in this feed.
    pub band: usize,
    /// `band / seen` — the T2 metric, per feed.
    pub band_mass: f32,
    /// Pooled `Σr / Σn` across the feed (the aggregate resolve rate — a
    /// difficulty read, NOT the paper's causal curriculum effect).
    pub resolve_rate: f32,
}

/// The organic feed's label: plain store keys — real-repo heal outcomes
/// (the corpus rules + any real-run row; feature churn evidence).
pub const FEED_ORGANIC: &str = "organic";
/// The synthetic feed's label: `{code}:{strategy}` keys — generated-case
/// rows (the `corpus_gen` mutator planted the break; injected evidence).
pub const FEED_SYNTHETIC: &str = "synthetic";

/// Split the learn rows into the two provenance feeds (Issue 089 T5).
/// Attribution by key shape — the store's documented convention: rustc
/// trajectory keys are `"{code}:{strategy_id}"` and are the ONLY ':'-shaped
/// keys the store carries (generated cases); every other key is a
/// real-repo heal outcome. Returns `[organic, synthetic]` — always both
/// rows, zeros included, so a missing feed reads as measured-empty.
#[must_use]
pub fn provenance_yield_split(entries: &[(String, u32, u32)]) -> [ProvenanceYieldRow; 2] {
    // Per-feed accumulators: (rules, Σr, Σn, seen, band).
    let mut feeds = [(0usize, 0u32, 0u32, 0usize, 0usize); 2];
    for (key, r, f) in entries {
        // ':'-shaped keys are the generated-case convention — the ONLY
        // ':'-shaped keys the store carries (documented; --misses relies on
        // the same attribution).
        let idx = usize::from(key.contains(':'));
        let (rules, sum_r, sum_n, seen, band) = &mut feeds[idx];
        *rules += 1;
        *sum_r += *r;
        *sum_n += *r + *f;
        let n = *r + *f;
        if n >= 1 {
            *seen += 1;
            #[allow(clippy::cast_precision_loss)] // rate over u32 counters — the report's quantity
            let rate = *r as f32 / n as f32;
            if rate > 0.0 && rate <= ADMISSION_BAND_MAX_P_HAT {
                *band += 1;
            }
        }
    }
    let row = |feed: &'static str, (rules, r, n, seen, band): (usize, u32, u32, usize, usize)| {
        #[allow(clippy::cast_precision_loss)] // fractions over usize counts — the report's quantity
        let band_mass = if seen == 0 {
            0.0
        } else {
            band as f32 / seen as f32
        };
        #[allow(clippy::cast_precision_loss)] // rate over u32 counters — the report's quantity
        let resolve_rate = if n == 0 { 0.0 } else { r as f32 / n as f32 };
        ProvenanceYieldRow {
            feed,
            rules,
            seen,
            band,
            band_mass,
            resolve_rate,
        }
    };
    [row(FEED_ORGANIC, feeds[0]), row(FEED_SYNTHETIC, feeds[1])]
}

/// Render the outcome-feed provenance split + the honest per-batch blocker
/// (the T2 precedent: the causal yield — the paper's +2.06 organic vs
/// +0.00 injected — needs per-generation rows the store deliberately does
/// not carry; stated, not silently narrowed).
#[must_use]
pub fn render_provenance_yield(rows: &[ProvenanceYieldRow]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(320 + rows.len() * 96);
    let _ = writeln!(
        out,
        "outcome-feed provenance split (Issue 089 T5 — {FEED_SYNTHETIC} = generated-case '{{code}}:{{strategy}}' keys; every other key = {FEED_ORGANIC} real-repo heal outcomes):"
    );
    let _ = writeln!(
        out,
        "  {:<10} {:>6} {:>5} {:>5} {:>10} {:>12}",
        "feed", "rules", "seen", "band", "band-mass", "Σr/Σn"
    );
    for r in rows {
        let _ = writeln!(
            out,
            "  {:<10} {:>6} {:>5} {:>5} {:>9.1}% {:>12.3}",
            r.feed,
            r.rules,
            r.seen,
            r.band,
            r.band_mass * 100.0,
            r.resolve_rate
        );
    }
    let _ = writeln!(
        out,
        "  (per-BATCH causal yield is BLOCKED: rows carry no generation/batch tag — the T2 snapshot precedent; this split is per-feed health, not the curriculum effect)"
    );
    out
}

// ── Issue 089 T7: the "Beta (half-count)" feed contract ────────────────

/// One row of the shaped half-count fold: what a rule's counters become
/// when the bench's shaped verdicts ("target cleared ∧ new diags
/// introduced", priced at 0.5 — `score_bench::SHAPED_VERDICT` on the
/// feature-gated bench side) are folded in at HALF EVIDENCE — `r += s/2
/// AND f += s/2`, so `p̂` drifts toward 0.5 (exactly half a success and
/// half a failure per shaped outcome). The Elo twin of the same verdict
/// is the TIE ([`crate::elo::update_pair_scored`] at `s = 0.5`).
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedHalfRow {
    pub rule: String,
    /// Shaped-outcome count (each worth +0.5 r AND +0.5 f).
    pub s: u32,
    /// The store's integer counters before the fold.
    pub r: u32,
    pub f: u32,
    /// `r + s/2`.
    pub r_eff: f64,
    /// `f + s/2`.
    pub f_eff: f64,
    /// The store's `p̂` (0.0 when the store has never seen the rule — the
    /// bench-only rule case; the "0.0" here is "no organic evidence", NOT
    /// [`BandClass::BelowBand`]).
    pub p_hat_store: f64,
    /// `(r + s/2) / (r + f + s)` — the half-count-folded rate.
    pub p_hat_with_halves: f64,
}

/// Fold the bench's per-rule shaped counts into the store's integer
/// outcome counters (Issue 089 T7). Rules named by `half_counts` that the
/// store has never seen heal with `r = f = 0` — their folded `p̂` is
/// EXACTLY 0.5 (the admission-band ceiling by construction: a rule whose
/// only evidence is shaped sits at the band's edge, never above it).
///
/// Deliberately NOT wired into [`summarize`]/the LCB math: the u32
/// `best_belief_score` primitive cannot price halves without doubling
/// counts (Beta(1+2r+s, 1+2f+s) fakes concentration), and the admission
/// gate stays on the organic store feed — this fold is the REPORT
/// representation (report-first law), the same standing as the strata.
#[must_use]
pub fn shaped_half_rows(
    entries: &[(String, u32, u32)],
    half_counts: &[(String, u32)],
) -> Vec<ShapedHalfRow> {
    use std::collections::BTreeMap;
    let by_rule: BTreeMap<&str, (u32, u32)> = entries
        .iter()
        .map(|(r, s, f)| (r.as_str(), (*s, *f)))
        .collect();
    let mut rows: Vec<ShapedHalfRow> = half_counts
        .iter()
        .filter(|(_, s)| *s > 0)
        .map(|(rule, s)| {
            let (r, f) = by_rule.get(rule.as_str()).copied().unwrap_or((0, 0));
            let half = f64::from(*s) / 2.0;
            let (r_eff, f_eff) = (f64::from(r) + half, f64::from(f) + half);
            let p_store = if r + f == 0 {
                0.0
            } else {
                f64::from(r) / f64::from(r + f)
            };
            ShapedHalfRow {
                rule: rule.clone(),
                s: *s,
                r,
                f,
                r_eff,
                f_eff,
                p_hat_store: p_store,
                p_hat_with_halves: r_eff / (r_eff + f_eff),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.s.cmp(&a.s).then(a.rule.cmp(&b.rule)));
    rows
}

/// Render the half-count section (EMPTY-Loud on no shaped rows — an absent
/// feed is stated, never rendered as a zero-measurement).
#[must_use]
pub fn render_shaped_half_rows(rows: &[ShapedHalfRow]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(256 + rows.len() * 112);
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "shaped half-count fold (Issue 089 T7): EMPTY — no shaped verdict rows in the score history (generation 8 unrun here)"
        );
        return out;
    }
    let _ = writeln!(
        out,
        "shaped half-count fold (Issue 089 T7 — each shaped verdict = +0.5 r AND +0.5 f; the Elo twin is the tie):"
    );
    let _ = writeln!(
        out,
        "  {:<28} {:>4} {:>9} {:>9} {:>10} {:>10}",
        "rule", "s", "r→r_eff", "f→f_eff", "p̂_store", "p̂_folded"
    );
    for row in rows {
        let _ = writeln!(
            out,
            "  {:<28} {:>4} {:>4}→{:<4.1} {:>4}→{:<4.1} {:>10.3} {:>10.3}",
            row.rule,
            row.s,
            row.r,
            row.r_eff,
            row.f,
            row.f_eff,
            row.p_hat_store,
            row.p_hat_with_halves
        );
    }
    let _ = writeln!(
        out,
        "  (report-first: the admission/LCB math stays on the store's integer feed — doubling counts to fit u32 would fake Beta concentration)"
    );
    out
}

// ── Issue 089 T4's evidence clock: band-mass snapshots ──────────────────
//
// T2's honest blocker, made accumulable: the store holds CUMULATIVE
// counters, so historical band-mass at generation boundaries cannot be
// reconstructed retroactively — and T4's GO evidence is the band-mass ↔
// next-generation yield correlation over ≥3 generations. Every
// `--frontier-report` run appends the just-measured band-mass to a
// COMMITTED append-only jsonl (the score-history law: the committed file
// is the projection), deduped by state change — an unchanged store at the
// same generation appends nothing. Since the bench-tail hardening
// (2026-09-13) every score-bench run appends the same snapshot at its tail
// (see [`append_snapshot_at_frontier`]) — a generation boundary is created
// by a bench run, so the boundary's snapshot is taken by the same run; a
// boundary nobody reports at is a boundary the correlation never samples.
// The report-first law is unchanged: the file gates nothing; it makes the
// evidence exist.

/// The committed append-only snapshot log (relative to the report's cwd).
pub const SNAPSHOTS_JSONL: &str = ".benchmarks/frontier_snapshots.jsonl";

/// One band-mass snapshot — the T2 metric frozen at a point in time
/// (Issue 089 T4's x-axis: band-mass AT a generation boundary, correlated
/// later against that generation's yield).

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrontierSnapshot {
    /// UTC ISO-8601 timestamp of the report run.
    pub ts: String,
    /// Best-effort git sha of the report's cwd. NOT the store discriminator
    /// (that reading was measured wrong 2026-09-21, Issue 102 verdict): the
    /// sha keys the cwd's HEAD, which moves with every commit, and two
    /// checkouts at one commit collide — the file's x-axis was non-monotone
    /// on two unnamed stores because the store itself was never persisted.
    /// `sha` stays for provenance (which tree produced the row); [`Self::store`]
    /// discriminates.
    pub sha: String,
    /// The generation FRONTIER — the max score-history generation ever run
    /// in this cwd (0 = no history yet). Deliberately NOT the latest lane:
    /// lanes interleave (v1 reruns after v8 landings) and the T4 x-axis
    /// (band-mass at a generation boundary) must be monotone.
    pub generation: u8,
    /// Learn rows scanned (the population — the green-zero defense's axis).
    pub population: usize,
    /// Rows with `n ≥ 1` (p̂ defined).
    pub seen: usize,
    /// In-band rules (`0 < p̂ ≤ 0.5`).
    pub band_count: usize,
    /// `band_count / seen` — the T2 health metric.
    pub band_mass: f32,
    /// `p̂=1 ∧ n≥W` rows — the demotion-candidate count.
    pub saturated: usize,
    /// The T5 outcome-feed split, per-feed band-mass (a band living only
    /// in the synthetic feed is a curriculum artifact; T4's gate needs the
    /// split, not just the pooled mass).
    pub feeds: Vec<FeedSnapshot>,
    /// Short id of the store this snapshot measured ([`store_id`] —
    /// blake3 of the canonicalized store root, first 8 hex; machine paths
    /// never enter the file). Empty on rows written before the field
    /// existed (2026-09-21): legacy rows are unattributed by construction,
    /// never retro-guessed from population clusters. Joins
    /// [`FrontierSnapshot::same_state`] — two stores with identical stats
    /// are two states, not one.
    #[serde(default)]
    pub store: String,
}

// ── Issue 105 T4: the journey integral axis (INSTRUMENTATION, not scored) ──
// PC-ALM credit (Research 554): successes accumulated inside tainted
// journeys (runs that ended at a masking-stage error — the strand-keep
// poison) carry reduced selection credit via
// [`crate::journey::journey_factor`]. This section surfaces the per-key
// evidence so the discount is OBSERVABLE from the store — report-only,
// never scored, the same standing as the provenance strata.

/// One key's journey-evidence row (rendered, pre-sorted).
pub struct JourneyRow {
    /// The trajectory key (`"{code}:{strategy_id}"` or a clippy rule id).
    pub lint_key: String,
    /// Runs the key kept an attempt in.
    pub runs: u32,
    /// Σ of tainted-run composites (×256 fixed-point).
    pub taint_sum: u32,
    /// The discount the selection layers apply (`1.0` = clean/neutral).
    pub factor: f32,
}

/// Build the journey section rows: keys with journey evidence, most
/// discounted first, bounded to `top`.
#[must_use]
pub fn journey_summary(rows: &[(String, u32, u32)], top: usize) -> Vec<JourneyRow> {
    let mut out: Vec<JourneyRow> = rows
        .iter()
        .map(|(k, runs, taint)| JourneyRow {
            lint_key: k.clone(),
            runs: *runs,
            taint_sum: *taint,
            factor: crate::journey::journey_factor(*runs, *taint),
        })
        .collect();
    out.sort_by(|a, b| {
        a.factor
            .total_cmp(&b.factor)
            .then(b.taint_sum.cmp(&a.taint_sum))
            .then(a.lint_key.cmp(&b.lint_key))
    });
    out.truncate(top);
    out
}

/// Render the journey section (empty input ⇒ a one-line absent note, the
/// honest no-evidence state — never a zero table).
#[must_use]
pub fn render_journey_section(rows: &[JourneyRow], total_keys: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "## Journey credit (Issue 105, instrumentation — not scored)"
    );
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "no journey evidence in the store (no --fix-compile keeps recorded yet)\n"
        );
        return out;
    }
    let tainted = rows.iter().filter(|r| r.taint_sum > 0).count();
    let _ = writeln!(
        out,
        "{total_keys} key(s) with journey evidence, {tainted} tainted; most-discounted first:\n"
    );
    let _ = writeln!(
        out,
        "| key | runs | taint×256 | factor |\n|---|---:|---:|---:|"
    );
    for r in rows {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.3} |",
            r.lint_key, r.runs, r.taint_sum, r.factor
        );
    }
    out
}

// ── Issue 102 T1: the contribution-fade section (report-first) ──────────
// Library Drift (arXiv:2605.19576) over the healer corpus: a rule that
// keeps getting retrieval-exposed on full-corpus sweeps and resolves
// nothing is a FADE CANDIDATE — report-only, never a demotion (T2/T3 stay
// gated on this report proving predictive over ≥3 FUTURE generations; the
// clock started 2026-09-21 at this landing).
//
// ⛔ The unit is a DISTINCT FULL-CORPUS SWEEP, never a distinct row and
// never a distinct sha (the verdict that revised the first cut,
// 2026-09-21): the 44-row gen-1 series has ZERO variance — one distinct
// outcome tuple per rule across 37 shas, because the routine v1 lane
// samples 4–5 rules of a ~90-rule corpus — so keying fade on rows would
// land an assertion that runs and cannot fail, and would flag
// `map_unwrap_or` (persist=1, resolved 1/2, constant) FADED on day one
// with nothing having faded. A sweep row is a row whose attribution
// covers the corpus.
//
// ⛔ FADED ≠ NEVER-CONTRIBUTED (the verdict's round-3 finding): fade
// means CONTRIBUTED, THEN STOPPED — a rule with zero resolutions across
// the whole series never contributed, which is 089 T4's `p̂ = 0`
// expressiveness-gap fork (L3 ruliology or `decline`) — the OPPOSITE of
// demotion. Two buckets, printed separately; T2 may only ever read the
// FADED list.

/// Minimum rules a score-history row must attribute to count as a
/// full-corpus sweep row. The routine v1 lane attributes 4–5; the gen-3
/// persist-stratified sweeps attributed 36 (2026-09-13) → 47 (2026-09-18).
/// 25 = 5× the routine lane and above half the smallest observed sweep —
/// pinned, not tuned.
pub const SWEEP_MIN_RULES: usize = 25;

/// Fade candidacy needs this many sweep rows in a rule's series (Issue 102
/// T1: "≥N generation rows, N a floor not a vibe").
pub const MIN_FADE_SWEEPS: usize = 3;

/// One score-history row the fade scan consumes — the light ungated shape.
/// The CLI maps `serde_json::Value` → this (the T7 `half_counts` precedent:
/// the `score_bench` types stay feature-gated).
#[derive(Clone, Debug)]
pub struct FadeRow {
    /// The bench generation the row ran.
    pub generation: u8,
    /// UTC ISO ts of the run.
    pub ts: String,
    /// Per-rule outcomes attributed by this row (single-target only —
    /// multi-target composites are skipped loud at bench time and absent
    /// here by construction).
    pub outcomes: Vec<FadeOutcome>,
}

/// One rule's outcome inside a [`FadeRow`].
#[derive(Clone, Debug)]
pub struct FadeOutcome {
    /// The rule id.
    pub rule: String,
    /// Fixtures the rule resolved cleanly.
    pub resolved: u32,
    /// Fixtures the rule persisted (unresolved, output ok).
    pub persist: u32,
    /// Edits the rule applied (the R(n) log-length axis, 089 T8).
    pub edits: u32,
}

/// One rule's contribution series across full-corpus sweeps.
#[derive(Clone, Debug)]
pub struct FadeCandidate {
    /// The rule id.
    pub rule: String,
    /// Sweep rows the rule was attributed in (its series length).
    pub sweeps: usize,
    /// Distinct generations across the series (display only — lanes can
    /// repeat a generation at a new corpus state).
    pub generations: Vec<u8>,
    /// Σ resolved across the series (≥1 — FADED means contributed, then
    /// stopped; a zero here is [`FadeScan::never_contributed`], not a
    /// candidate).
    pub resolved_total: u32,
    /// Sweeps with ≥1 resolution (the prior-contribution term).
    pub resolved_sweeps: usize,
    /// Consecutive zero-resolution sweeps at the series tail (the stop).
    pub trailing_zero_sweeps: usize,
    /// Σ persist across the series (exposed-but-unresolved signal).
    pub persist_total: u32,
    /// Σ edits across the series.
    pub edits_total: u32,
    /// ts of the most recent sweep in the series.
    pub last_sweep_ts: String,
}

/// The fade scan over a score history.
#[derive(Clone, Debug)]
pub struct FadeScan {
    /// Sweep rows found (rows attributing ≥ [`SWEEP_MIN_RULES`] rules).
    pub sweep_rows: usize,
    /// One line per sweep row: "gen 3 @ ts (36 rules)" — the population
    /// the verdict rests on, printed in both the live and refused states.
    pub sweep_notes: Vec<String>,
    /// FADED candidates (contributed, then stopped) — most-persisting
    /// first. EMPTY whenever [`FadeScan::refused`] is set. The ONLY bucket
    /// T2 may ever consume.
    pub candidates: Vec<FadeCandidate>,
    /// Rules attributed in ≥ [`MIN_FADE_SWEEPS`] sweeps with ZERO
    /// resolutions across the whole series — NEVER-CONTRIBUTED, 089 T4's
    /// `p̂ = 0` expressiveness-gap fork territory, NEVER a demotion
    /// candidate. Reported so the two buckets can never be conflated.
    pub never_contributed: Vec<String>,
    /// Loud refusal when the population is below [`MIN_FADE_SWEEPS`]. The
    /// section's live state is a REFUSAL until ≥3 sweeps exist; correctness
    /// rides the planted-fade self-test until then.
    pub refused: Option<String>,
}

/// One rule's running contribution series inside [`fade_scan`] (a named
/// accumulator — the tuple form was a type-complexity finding).
#[derive(Default)]
struct FadeSeries {
    sweeps: usize,
    resolved: u32,
    persist: u32,
    edits: u32,
    resolved_sweeps: usize,
    trailing_zero_sweeps: usize,
    generations: Vec<u8>,
    last_ts: String,
}

/// Scan the score history for contribution fade. Pure — no store, no I/O.
/// Per rule attributed in ≥ [`MIN_FADE_SWEEPS`] full-corpus sweeps:
/// **NEVER-CONTRIBUTED** (zero resolutions in every sweep →
/// [`FadeScan::never_contributed`], the 089 triage fork, never a
/// demotion candidate) vs **FADED** (≥1 sweep with a resolution, then a
/// zero-resolution tail → [`FadeScan::candidates`]; attribution in a
/// sweep IS the exposure — per Issue 102 Caution 4 the per-generation
/// evidence comes from these bench-time rows, never cumulative store
/// deltas).
#[must_use]
pub fn fade_scan(rows: &[FadeRow]) -> FadeScan {
    let sweeps: Vec<&FadeRow> = rows
        .iter()
        .filter(|r| r.outcomes.len() >= SWEEP_MIN_RULES)
        .collect();
    let sweep_notes: Vec<String> = sweeps
        .iter()
        .map(|r| {
            format!(
                "gen {} @ {} ({} rules)",
                r.generation,
                r.ts,
                r.outcomes.len()
            )
        })
        .collect();
    let mut scan = FadeScan {
        sweep_rows: sweeps.len(),
        sweep_notes,
        candidates: Vec::new(),
        never_contributed: Vec::new(),
        refused: None,
    };
    if scan.sweep_rows < MIN_FADE_SWEEPS {
        scan.refused = Some(format!(
            "REFUSED — needs ≥{MIN_FADE_SWEEPS} full-corpus sweep rows (≥{SWEEP_MIN_RULES} rules \
             attributed), have {} — no candidates reported from an under-populated corpus; \
             correctness rides the planted-fade self-test until the population exists",
            scan.sweep_rows
        ));
        return scan;
    }
    let mut series: std::collections::BTreeMap<String, FadeSeries> =
        std::collections::BTreeMap::new();
    for row in &sweeps {
        for o in &row.outcomes {
            let e = series.entry(o.rule.clone()).or_default();
            e.sweeps += 1;
            e.resolved += o.resolved;
            e.persist += o.persist;
            e.edits += o.edits;
            if o.resolved > 0 {
                e.resolved_sweeps += 1;
                e.trailing_zero_sweeps = 0;
            } else {
                e.trailing_zero_sweeps += 1;
            }
            if !e.generations.contains(&row.generation) {
                e.generations.push(row.generation);
            }
            e.last_ts.clone_from(&row.ts);
        }
    }
    for (rule, s) in series {
        classify_series(&mut scan, rule, s);
    }
    scan.never_contributed.sort();
    scan.candidates.sort_by(|a, b| {
        b.persist_total
            .cmp(&a.persist_total)
            .then(b.edits_total.cmp(&a.edits_total))
            .then(a.rule.cmp(&b.rule))
    });
    scan
}

/// Bucket one rule's completed series: thin (< [`MIN_FADE_SWEEPS`]) is
/// skipped, zero-resolution is NEVER-CONTRIBUTED (089's triage fork), a
/// resolution followed by a zero tail is FADED, everything else is still
/// healthy. Deterministic order comes from the final sorts, not the walk.
fn classify_series(scan: &mut FadeScan, rule: String, s: FadeSeries) {
    if s.sweeps < MIN_FADE_SWEEPS {
        return;
    }
    if s.resolved_sweeps == 0 {
        scan.never_contributed.push(rule);
        return;
    }
    if s.trailing_zero_sweeps == 0 {
        return;
    }
    let mut generations = s.generations;
    generations.sort_unstable();
    scan.candidates.push(FadeCandidate {
        rule,
        sweeps: s.sweeps,
        generations,
        resolved_total: s.resolved,
        resolved_sweeps: s.resolved_sweeps,
        trailing_zero_sweeps: s.trailing_zero_sweeps,
        persist_total: s.persist,
        edits_total: s.edits,
        last_sweep_ts: s.last_ts,
    });
}

/// Render the fade section. The refused state is LOUD (population
/// disclosure + no candidates) — never a silent zero table (the
/// frontier-report law). NEVER-CONTRIBUTED prints separately, never
/// folded into the FADED table (the round-3 finding: the two buckets
/// carry opposite remedies).
#[must_use]
pub fn render_fade_section(scan: &FadeScan) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "## Contribution fade (Issue 102 T1, report-first — gates nothing)"
    );
    let _ = writeln!(
        out,
        "full-corpus sweep rows: {} (≥{SWEEP_MIN_RULES} rules attributed)",
        scan.sweep_rows
    );
    for note in &scan.sweep_notes {
        let _ = writeln!(out, "  · {note}");
    }
    if let Some(reason) = &scan.refused {
        let _ = writeln!(out, "fade: {reason}");
        return out;
    }
    if !scan.never_contributed.is_empty() {
        let _ = writeln!(
            out,
            "never-contributed: {} rule(s) with ZERO resolutions across every sweep — 089's \
             p̂=0 triage fork (expressiveness gap), NEVER demotion candidates:",
            scan.never_contributed.len()
        );
        for rule in scan.never_contributed.iter().take(12) {
            let _ = writeln!(out, "  · {rule}");
        }
        let hidden = scan.never_contributed.len().saturating_sub(12);
        if hidden > 0 {
            let _ = writeln!(out, "  · … and {hidden} more");
        }
    }
    if scan.candidates.is_empty() {
        let _ = writeln!(
            out,
            "no FADED candidates over {} sweep rows — every ≥{}-sweep rule either still \
             resolves or never contributed (see above)",
            scan.sweep_rows, MIN_FADE_SWEEPS
        );
        return out;
    }
    let _ = writeln!(
        out,
        "{} FADED candidate(s) — contributed, then stopped (zero-resolution tail; the ONLY \
         bucket T2 may ever read; report-only, T2/T3 stay gated):",
        scan.candidates.len()
    );
    let _ = writeln!(
        out,
        "| rule | sweeps | gens | resolved sweeps | trailing zeros | persist | edits | last sweep |\n|---|---:|---|---:|---:|---:|---:|---|"
    );
    for c in &scan.candidates {
        let gens = c
            .generations
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            c.rule,
            c.sweeps,
            gens,
            c.resolved_sweeps,
            c.trailing_zero_sweeps,
            c.persist_total,
            c.edits_total,
            c.last_sweep_ts
        );
    }
    out
}

/// One feed's band-mass inside a [`FrontierSnapshot`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedSnapshot {
    /// [`FEED_ORGANIC`] / [`FEED_SYNTHETIC`].
    pub feed: String,
    /// Rules in this feed.
    pub rules: usize,
    /// Rules with `n ≥ 1` in this feed.
    pub seen: usize,
    /// In-band rules in this feed.
    pub band: usize,
    /// `band / seen` — the T2 metric, per feed.
    pub band_mass: f32,
}

impl FrontierSnapshot {
    /// Same measured state (everything but `ts` — two runs over an
    /// unchanged store differ only by timestamp, which is not a new row).
    /// `store` is part of the state: two stores with identical stats are
    /// two measurements, not one (the 2026-09-21 non-monotone-axis lesson).
    fn same_state(&self, other: &Self) -> bool {
        self.sha == other.sha
            && self.generation == other.generation
            && self.population == other.population
            && self.seen == other.seen
            && self.band_count == other.band_count
            && self.saturated == other.saturated
            && self.feeds == other.feeds
            && self.store == other.store
    }
}

/// Build the snapshot of a just-rendered report (pure — the CLI passes the
/// already-computed summary + feed rows; `ts`/`sha` come from the helpers
/// below so both stay std-only + best-effort).
#[must_use]
pub fn snapshot_from(
    summary: &FrontierSummary,
    feeds: &[ProvenanceYieldRow],
    generation: u8,
    sha: String,
    ts: String,
    store: String,
) -> FrontierSnapshot {
    FrontierSnapshot {
        ts,
        sha,
        generation,
        store,
        population: summary.population,
        seen: summary.seen,
        band_count: summary.band_count,
        band_mass: summary.band_mass,
        saturated: summary
            .rows
            .iter()
            .filter(|r| r.class == BandClass::Saturated)
            .count(),
        feeds: feeds
            .iter()
            .map(|f| FeedSnapshot {
                feed: f.feed.to_string(),
                rules: f.rules,
                seen: f.seen,
                band: f.band,
                band_mass: f.band_mass,
            })
            .collect(),
    }
}

/// Append one snapshot unless the last row carries the SAME measured state
/// (the state-change dedup). `Ok(true)` = appended; `Ok(false)` =
/// unchanged, not appended. A malformed trailing line is loudly skipped
/// (the `load_history` law), never fatal — the file is instrumentation.
///
/// # Errors
///
/// Propagates filesystem errors (create dir / open / append write).
pub fn append_snapshot(path: &std::path::Path, snap: &FrontierSnapshot) -> std::io::Result<bool> {
    use std::io::Write;
    if let Ok(text) = std::fs::read_to_string(path) {
        if let Some(last) = text.lines().rev().find(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<FrontierSnapshot>(last) {
                Ok(prev) if prev.same_state(snap) => return Ok(false),
                Ok(_) => {}
                Err(e) => eprintln!("frontier snapshot: skipping malformed last row: {e}"),
            }
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut line = serde_json::to_string(snap).map_err(std::io::Error::other)?;
    line.push('\n');
    f.write_all(line.as_bytes())?;
    Ok(true)
}

/// The outcome of a [`append_snapshot_at_frontier`] call — one console line
/// for the caller, never an exit-code change (the report-first law: the
/// snapshot is instrumentation; it must never fail the run that records it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotOutcome {
    /// A row was appended (first row, or state changed — the store moved or
    /// the generation frontier rose).
    Appended {
        /// The generation frontier the new row carries.
        generation: u8,
    },
    /// The last row already carries the same measured state — deduped,
    /// nothing appended.
    Unchanged {
        /// The unchanged generation frontier.
        generation: u8,
    },
    /// No trajectory store at the root — nothing to measure (the snapshot
    /// is skipped, never fabricated).
    NoStore,
    /// The store exists but carries zero learn rows — a band-mass row over
    /// an empty population would be a green zero (the report law); skipped.
    EmptyPopulation,
}

impl std::fmt::Display for SnapshotOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Appended { generation } => {
                write!(
                    f,
                    "appended (generation {generation}) — the boundary is recorded"
                )
            }
            Self::Unchanged { generation } => write!(
                f,
                "unchanged since the last row (generation {generation}) — not appended"
            ),
            Self::NoStore => write!(
                f,
                "no trajectory store at the root — skipped (run any --fix in this repo to create it)"
            ),
            Self::EmptyPopulation => write!(
                f,
                "store carries zero learn rows — skipped (never a green zero row)"
            ),
        }
    }
}

/// Issue 089 T4 hardening — the score-bench tail's automatic boundary
/// snapshot.
///
/// T4's GO evidence needs a band-mass snapshot AT every generation
/// boundary, but boundaries are created by score-bench runs: a mechanism
/// where the snapshot exists only if somebody separately remembers to run
/// `--frontier-report` at the right moment loses boundaries to interleaved
/// lanes (v1 reruns after v8 landings). The bench tail calls this right
/// after its history append + store dual-write, so the just-recorded run IS
/// in `score_runs` and a newly admitted generation raises the frontier in
/// the same breath — a boundary can no longer be missed.
///
/// Same semantics as the report path: the caller passes the cwd-relative
/// [`SNAPSHOTS_JSONL`] (deliberately NOT `root`-relative, so both writers
/// share one file when `RIIR_REFINE_TRAJ_ROOT` points elsewhere),
/// state-change dedup, the FRONTIER generation (max ever run, not the
/// latest lane). Zero learn rows and a missing store are skips, never
/// fabricated rows. The file still gates nothing (report-first law).
///
/// # Errors
///
/// Store open failure or snapshot-file filesystem errors — returned for
/// one console line; never a panic, never a bench-run failure.
#[cfg(feature = "latent_retrieval")]
pub fn append_snapshot_at_frontier(
    root: &std::path::Path,
    snapshots_path: &std::path::Path,
) -> std::io::Result<SnapshotOutcome> {
    let kv = root.join(".refine").join("trajectories.kv");
    if !kv.exists() {
        return Ok(SnapshotOutcome::NoStore);
    }
    let (store, _bad) = crate::traj_store::TrajStore::open(root).map_err(std::io::Error::other)?;
    let entries: Vec<(String, u32, u32)> = store
        .learn_rows()
        .into_iter()
        .map(|row| (row.lint_key, row.successes, row.fails))
        .collect();
    let summary = summarize(&entries);
    if summary.population == 0 {
        return Ok(SnapshotOutcome::EmptyPopulation);
    }
    let feeds = provenance_yield_split(&entries);
    let generation = store
        .score_runs()
        .unwrap_or_default()
        .iter()
        .map(|r| r.generation)
        .max()
        .unwrap_or(0);
    let snap = snapshot_from(
        &summary,
        &feeds,
        generation,
        git_sha(),
        local_iso_timestamp(),
        store_id(root),
    );
    if append_snapshot(snapshots_path, &snap)? {
        Ok(SnapshotOutcome::Appended { generation })
    } else {
        Ok(SnapshotOutcome::Unchanged { generation })
    }
}

/// Best-effort short git sha of the current repo (empty outside a repo).
/// Moved UNGATED to this module (Issue 089 T4) from `score_bench::report`
/// so the snapshot writer does not drag the feature-gated bench module
/// into every build; the bench re-exports it (`sb::report::git_sha`) so
/// existing call sites are stable.
#[must_use]
pub fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Stable short id of the store at `root` — blake3 of the canonicalized
/// root path, first 8 hex (the Issue 102 verdict, 2026-09-21: the
/// snapshots file's x-axis was non-monotone on two unnamed stores because
/// `generation` is per-store while `root` was never written to the row;
/// `sha` keys the cwd's HEAD, which moves with every commit, and two
/// checkouts at one commit collide). Machine paths never enter the file —
/// only the digest. Deterministic for a store's life at one path; a moved
/// store reads as a NEW store, which is honest — its history did not move
/// with it.
#[must_use]
pub fn store_id(root: &std::path::Path) -> String {
    let canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let hex = blake3::hash(canon.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    hex[..8].to_string()
}

/// UTC ISO-8601 timestamp, std-only (Howard Hinnant's civil-from-days —
/// same algorithm chrono uses; no dependency for one timestamp per run).
/// UTC deliberately: deterministic across machines. Same move-ungated
/// story as [`git_sha`] (Issue 089 T4; re-exported by the bench report).
#[must_use]
#[allow(clippy::cast_possible_wrap, clippy::many_single_char_names)]
pub fn local_iso_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (h, min, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}")
}

/// days-since-epoch → (y, m, d) — Hinnant's `civil_from_days`.
// Bounded intermediate ranges by construction (the algorithm's own
// comments; the truncation/wrap/sign allows mirror the score_bench report's
// module-level set — the fn moved homes, not semantics).
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
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
            ("band_mid".into(), 3, 3),    // p̂=0.5 band edge (inclusive)
            ("band_low".into(), 1, 9),    // p̂=0.1
            ("band_thin".into(), 1, 1),   // p̂=0.5, thin
            ("band_deep".into(), 20, 30), // p̂=0.4, deep — band head
            // below: expressiveness gap (all-fail)
            ("gap_all_fail".into(), 0, 12),
            // above: mid rates + thin-perfect
            ("above_mid".into(), 9, 1),     // p̂=0.9
            ("perfect_thin".into(), 10, 0), // p̂=1, n=10 < W → above
            ("perfect_edge".into(), 49, 0), // p̂=1, n=49 < W → above (edge)
            // saturated: all-time-perfect with deep evidence
            ("saturated_edge".into(), 50, 0), // p̂=1, n=50 = W → saturated (edge)
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
        assert!(
            (s.band_mass - 4.0 / 11.0).abs() < 1e-6,
            "got {m}",
            m = s.band_mass
        );
        assert_eq!(s.band_count, 4);
    }

    /// Issue 139 T3 — the disclosure arithmetic on the shared fixture
    /// (12 rows, 11 seen, 4 band): P(¬C) = 1/12, population mass = 4/12,
    /// budget = 1/12, hull = [4/12, 5/12]; the hull's distance from the
    /// seen-only figure never exceeds the budget; an empty population
    /// discloses nothing.
    #[cfg(feature = "noise_weighted_evidence")]
    #[test]
    fn filter_disclosure_arithmetic() {
        let s = summarize(&fixtures());
        let d = filter_disclosure(&s).expect("non-empty population");
        assert!((d.p_not_c - 1.0 / 12.0).abs() < 1e-6);
        assert!((d.population_mass - 4.0 / 12.0).abs() < 1e-6);
        assert!((d.bias_bound - 1.0 / 12.0).abs() < 1e-6);
        assert!((d.hull.0 - 4.0 / 12.0).abs() < 1e-6 && (d.hull.1 - 5.0 / 12.0).abs() < 1e-6);
        let far = (s.band_mass - d.hull.0)
            .abs()
            .max((s.band_mass - d.hull.1).abs());
        assert!(
            far <= d.bias_bound + 1e-6,
            "hull must sit inside the budget: {far}"
        );
        assert!(filter_disclosure(&summarize(&[])).is_none());
        let text = render(&s, None);
        assert!(text.contains("filter disclosure (Issue 139 T3)"), "{text}");
        assert!(text.contains("exact hull [33.3%, 41.7%]"), "{text}");
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

    // ── Issue 089 T3: the per-domain frontier rule queue ──────────────────

    fn domain_fixtures() -> (Vec<String>, BTreeMap<String, (usize, usize)>) {
        let corpus = vec![
            "mixed_rule".to_string(),
            "all_fail_rule".to_string(),
            "saturated_rule".to_string(),
            "zero_verdict_row".to_string(),
            "never_touched".to_string(),
        ];
        let mut outcomes = BTreeMap::new();
        outcomes.insert("mixed_rule".to_string(), (2, 2));
        outcomes.insert("all_fail_rule".to_string(), (0, 3));
        outcomes.insert("saturated_rule".to_string(), (5, 0));
        outcomes.insert("zero_verdict_row".to_string(), (0, 0));
        // A key OUTSIDE the corpus — attribution is the CLI's job; the queue
        // must ignore it, never invent a row for it.
        outcomes.insert("not_in_corpus".to_string(), (9, 9));
        (corpus, outcomes)
    }

    #[test]
    fn domain_queue_orders_by_control_law_and_ignores_foreign_keys() {
        let (corpus, outcomes) = domain_fixtures();
        let rows = domain_queue(&corpus, &outcomes);
        // observed only: never_touched (no store row) and not_in_corpus
        // (foreign key) are both absent.
        assert_eq!(rows.len(), 4, "{rows:?}");
        assert!(!rows.iter().any(|r| r.rule == "never_touched"));
        assert!(!rows.iter().any(|r| r.rule == "not_in_corpus"));
        // Control law: informative → empty → regenerate → saturated.
        let order: Vec<&str> = rows.iter().map(|r| r.rule.as_str()).collect();
        assert_eq!(
            order,
            vec![
                "mixed_rule",
                "zero_verdict_row",
                "all_fail_rule",
                "saturated_rule"
            ],
            "frontier order must hold at rule granularity: {rows:?}"
        );
        // Ranks are 1-based and contiguous.
        for (i, r) in rows.iter().enumerate() {
            assert_eq!(r.rank, i + 1);
        }
        // The informative row carries its counts verbatim.
        let mixed = &rows[0];
        assert_eq!((mixed.kept, mixed.reverted), (2, 2));
    }

    #[test]
    fn domain_queue_deterministic() {
        let (corpus, outcomes) = domain_fixtures();
        assert_eq!(
            domain_queue(&corpus, &outcomes),
            domain_queue(&corpus, &outcomes)
        );
    }

    #[test]
    fn render_domain_queue_needles() {
        let (corpus, outcomes) = domain_fixtures();
        let rows = domain_queue(&corpus, &outcomes);
        let text = render_domain_queue("clippy", &rows, corpus.len());
        assert!(
            text.contains("# clippy — frontier rule queue (4 observed / 5 corpus rules)"),
            "{text}"
        );
        assert!(text.contains("| 1 | mixed_rule | 2 | 2 |"), "{text}");
        assert!(text.contains("informative"), "{text}");
        assert!(text.contains("regenerate"), "{text}");
        assert!(text.contains("saturated"), "{text}");
        assert!(
            text.contains("coverage: 4 of 5; 1 rule(s) unobserved"),
            "{text}"
        );
    }

    #[test]
    fn render_domain_queue_empty_is_loud_but_not_an_error() {
        let text = render_domain_queue("sec", &[], 15);
        assert!(
            text.contains("0 observed / 15 corpus rules") && text.contains("(empty"),
            "{text}"
        );
        // No table header on an empty domain — nothing to rank.
        assert!(!text.contains("| rank |"));
    }

    // ── Issue 089 T5: provenance weighting ────────────────────────────────

    fn pair_counts_fixture() -> BTreeMap<String, usize> {
        let mut m = BTreeMap::new();
        m.insert(crate::frontier_pairs::PROVENANCE.to_string(), 200);
        m.insert(crate::frontier_pairs::PROVENANCE_ORGANIC.to_string(), 10);
        m.insert("hand-waved".to_string(), 3);
        m
    }

    #[test]
    fn provenance_summary_orders_prices_and_shares() {
        // Exact f32 equality IS the contract here — the class prices (1.0 /
        // ε / 0.0) and their weighted counts are the pin, not approximations.
        #![allow(clippy::float_cmp)]
        let rows = provenance_summary(&pair_counts_fixture());
        // Canonical order first (organic, construction), unknown tail asc.
        let order: Vec<&str> = rows.iter().map(|r| r.provenance.as_str()).collect();
        assert_eq!(order, ["organic", "construction", "hand-waved"]);
        // Total = 213; shares sum to 1; weights price the classes.
        let organic = &rows[0];
        let construction = &rows[1];
        let unknown = &rows[2];
        assert_eq!(organic.pairs, 10);
        assert_eq!(organic.weight, 1.0);
        assert_eq!(organic.weighted, 10.0);
        assert_eq!(construction.pairs, 200);
        assert_eq!(
            construction.weight,
            crate::frontier_pairs::SYNTHETIC_PAIR_WEIGHT
        );
        assert!(
            (construction.weighted - 200.0 * crate::frontier_pairs::SYNTHETIC_PAIR_WEIGHT).abs()
                < 1e-6
        );
        assert_eq!(unknown.weight, 0.0, "unrecognized tags priced out entirely");
        assert_eq!(unknown.weighted, 0.0);
        let share_sum: f32 = rows.iter().map(|r| r.share).sum();
        assert!((share_sum - 1.0).abs() < 1e-6);
        // Canonical classes with ZERO pairs still render (a missing class
        // reads as measured-empty, not absent).
        let mut only_unknown = BTreeMap::new();
        only_unknown.insert("hand-waved".to_string(), 1);
        let rows2 = provenance_summary(&only_unknown);
        assert_eq!(rows2.len(), 3);
        assert_eq!(rows2[0].pairs, 0);
        assert_eq!(rows2[1].pairs, 0);
        assert_eq!(rows2[2].pairs, 1);
    }

    #[test]
    fn render_provenance_section_needles() {
        let rows = provenance_summary(&pair_counts_fixture());
        let text = render_provenance_section(&rows);
        assert!(text.contains("pair provenance (Issue 089 T5"), "{text}");
        assert!(text.contains("construction"), "{text}");
        assert!(text.contains("never inject failures on purpose"), "{text}");
        assert!(text.contains("weighted"), "{text}");
        // The empty namespace is LOUD, not a green nothing.
        let empty = render_provenance_section(&[]);
        assert!(
            empty.contains("EMPTY") && empty.contains("0 pair rows"),
            "{empty}"
        );
    }

    #[test]
    fn provenance_yield_split_attributes_by_key_shape() {
        let entries = vec![
            // organic (plain keys): one in-band, one above, one unseen.
            ("manual_is_multiple_of".into(), 2, 2), // p̂=0.5 → band
            ("use_self".into(), 172, 0),            // p̂=1 → above/saturated-class feed
            ("never_fired".into(), 0, 0),           // n=0 unseen
            // synthetic (':'-shaped keys): one in-band, one below.
            ("E0597:e0597-scope-hoist".into(), 1, 3), // p̂=0.25 → band
            ("E0507:gen:E0507:clone-at-move".into(), 0, 4), // p̂=0 → below
        ];
        let [organic, synthetic] = provenance_yield_split(&entries);
        assert_eq!(organic.feed, "organic");
        assert_eq!(organic.rules, 3);
        assert_eq!(organic.seen, 2);
        assert_eq!(organic.band, 1);
        assert!((organic.resolve_rate - 174.0 / 176.0).abs() < 1e-6);
        assert_eq!(synthetic.feed, "synthetic");
        assert_eq!(synthetic.rules, 2);
        assert_eq!(synthetic.seen, 2);
        assert_eq!(synthetic.band, 1);
        assert!((synthetic.resolve_rate - 1.0 / 8.0).abs() < 1e-6);
        // Band-mass per feed: 1/2 both here.
        assert!((organic.band_mass - 0.5).abs() < 1e-6);
        assert!((synthetic.band_mass - 0.5).abs() < 1e-6);
        // Both feeds always render — zeros included (measured-empty).
        let [o2, s2] = provenance_yield_split(&[]);
        assert_eq!((o2.rules, o2.seen, s2.rules, s2.seen), (0, 0, 0, 0));
    }

    #[test]
    fn render_provenance_yield_needles() {
        let [o, s] = provenance_yield_split(&[]);
        let text = render_provenance_yield(&[o, s]);
        assert!(text.contains("outcome-feed provenance split"), "{text}");
        assert!(
            text.contains("organic") && text.contains("synthetic"),
            "{text}"
        );
        assert!(text.contains("per-BATCH causal yield is BLOCKED"), "{text}");
        assert!(text.contains("band-mass"), "{text}");
    }

    // ── Issue 089 T7: the half-count fold ───────────────────────────────

    #[test]
    // float_cmp: the fold's arithmetic is exact in f64 at these magnitudes
    // — equality against the hand-computed rationals IS the fixture.
    #[allow(clippy::float_cmp)]
    fn shaped_half_rows_fold_at_half_evidence() {
        let entries = vec![
            ("manual_map".into(), 9u32, 1u32), // p̂_store = 0.9
            ("use_self".into(), 172, 0),       // saturated — untouched below
        ];
        let half_counts = vec![
            ("manual_map".into(), 2u32),   // r_eff = 10, f_eff = 2 → p̂ 10/12
            ("bench_only_rule".into(), 3), // r=f=0 → p̂ EXACTLY 0.5
        ];
        let rows = shaped_half_rows(&entries, &half_counts);
        assert_eq!(rows.len(), 2);
        // Ordered by s desc: bench_only_rule (3) first.
        assert_eq!(rows[0].rule, "bench_only_rule");
        assert!((rows[0].p_hat_with_halves - 0.5).abs() < 1e-9);
        assert_eq!(rows[0].p_hat_store, 0.0);
        let mm = &rows[1];
        assert_eq!((mm.r, mm.f, mm.s), (9, 1, 2));
        assert!((mm.r_eff - 10.0).abs() < 1e-9 && (mm.f_eff - 2.0).abs() < 1e-9);
        assert!((mm.p_hat_with_halves - 10.0 / 12.0).abs() < 1e-9);
        assert!((mm.p_hat_store - 0.9).abs() < 1e-9);
        // p̂ DRIFTED DOWN toward 0.5 — the fold's whole point: shaped
        // outcomes are half evidence AGAINST the rule too.
        assert!(mm.p_hat_with_halves < mm.p_hat_store);
    }

    #[test]
    #[allow(clippy::float_cmp)] // 0.0 comparisons on no-store rows — exact by construction
    fn shaped_half_rows_drop_zero_counts_and_render_empty_loud() {
        let rows = shaped_half_rows(&[("a".into(), 1, 1)], &[("a".into(), 0)]);
        assert!(
            rows.is_empty(),
            "a zero-count half row is not a measurement"
        );
        let text = render_shaped_half_rows(&[]);
        assert!(text.contains("EMPTY"), "{text}");
        assert!(text.contains("generation 8 unrun"), "{text}");
        // The populated render carries the columns + the LCB caveat.
        let rows = shaped_half_rows(&[("manual_map".into(), 4, 1)], &[("manual_map".into(), 1)]);
        let text = render_shaped_half_rows(&rows);
        assert!(text.contains("shaped half-count fold"), "{text}");
        assert!(text.contains("p̂_folded"), "{text}");
        assert!(text.contains("fake Beta concentration"), "{text}");
    }

    // ── Issue 089 T4: band-mass snapshots ──────────────────────────────────

    fn snap_fixtures() -> (FrontierSummary, [ProvenanceYieldRow; 2]) {
        let summary = summarize(&fixtures());
        let feeds = provenance_yield_split(&fixtures());
        (summary, feeds)
    }

    #[test]
    fn snapshot_from_copies_state_and_feeds() {
        let (summary, feeds) = snap_fixtures();
        let snap = snapshot_from(
            &summary,
            &feeds,
            8,
            "deadbee".to_string(),
            "2026-09-12T05:00:00".to_string(),
            "store01".to_string(),
        );
        assert_eq!(snap.ts, "2026-09-12T05:00:00");
        assert_eq!(snap.sha, "deadbee");
        assert_eq!(snap.generation, 8);
        assert_eq!(snap.store, "store01");
        assert_eq!(snap.population, summary.population);
        assert_eq!(snap.seen, summary.seen);
        assert_eq!(snap.band_count, summary.band_count);
        // The fixtures() set carries exactly two saturated rows
        // (saturated_edge n=50, saturated_deep n=120).
        assert_eq!(snap.saturated, 2);
        // The feed split rides verbatim, organic-first.
        assert_eq!(snap.feeds.len(), 2);
        assert_eq!(snap.feeds[0].feed, FEED_ORGANIC);
        assert_eq!(snap.feeds[0].rules, feeds[0].rules);
        assert_eq!(snap.feeds[1].feed, FEED_SYNTHETIC);
    }

    #[test]
    fn snapshot_jsonl_round_trips() {
        let (summary, feeds) = snap_fixtures();
        let snap = snapshot_from(
            &summary,
            &feeds,
            0,
            "abc123".to_string(),
            "t".to_string(),
            "store02".to_string(),
        );
        let line = serde_json::to_string(&snap).unwrap();
        let back: FrontierSnapshot = serde_json::from_str(&line).unwrap();
        assert_eq!(back, snap);
    }

    #[test]
    fn legacy_row_without_store_decodes_empty_and_never_dedups_against_a_named_store() {
        let (summary, feeds) = snap_fixtures();
        let named = snapshot_from(
            &summary,
            &feeds,
            8,
            "aaaa".to_string(),
            "t1".to_string(),
            "store09".to_string(),
        );
        // A pre-2026-09-21 row carries no `store` key — serde default gives
        // the empty string, and the row is honestly unattributed.
        let mut legacy = serde_json::to_value(&named).unwrap();
        legacy.as_object_mut().unwrap().remove("store");
        let legacy: FrontierSnapshot = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy.store, "");
        // ...and the first discriminated snapshot lands even over identical
        // stats: legacy "" vs "store09" are two states, not one.
        assert!(!legacy.same_state(&named));
    }

    #[test]
    fn store_id_is_stable_short_and_path_sensitive() {
        let dir = std::env::temp_dir().join(format!("frontier_storeid_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = store_id(&dir);
        let b = store_id(&dir);
        assert_eq!(a, b, "stable across calls");
        assert_eq!(a.len(), 8, "first 8 hex");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        // A different path is a different store — the whole point.
        let other = store_id(&dir.join("sub"));
        assert_ne!(a, other);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Issue 102 T1: the planted-fade self-test suite (GOAT G2) ────────

    /// A sweep row: `rules` distinct rules, all attributed; per-rule
    /// outcomes overridable via the map.
    fn sweep_row(generation: u8, ts: &str, rules: &[&str], resolved: u32) -> FadeRow {
        FadeRow {
            generation,
            ts: ts.to_string(),
            outcomes: rules
                .iter()
                .map(|r| FadeOutcome {
                    rule: (*r).to_string(),
                    resolved,
                    persist: 1,
                    edits: 2,
                })
                .collect(),
        }
    }

    const SWEEP_RULES: [&str; 30] = [
        "r00",
        "r01",
        "r02",
        "r03",
        "r04",
        "r05",
        "r06",
        "r07",
        "r08",
        "r09",
        "r10",
        "r11",
        "r12",
        "r13",
        "r14",
        "r15",
        "r16",
        "r17",
        "r18",
        "r19",
        "r20",
        "r21",
        "r22",
        "r23",
        "r24",
        "r25",
        "r26",
        "r27",
        "r28",
        "faded_rule",
    ];

    #[test]
    fn planted_fade_needs_prior_contribution_and_never_contributed_is_its_own_bucket() {
        // The round-3 arm: a rule at (1, 0, 0) is FADED (contributed, then
        // stopped); a rule at (0, 0, 0) is NEVER-CONTRIBUTED — 089's
        // p̂=0 triage fork, never a demotion candidate. The old planted
        // arm (persist-only across all sweeps) was the NEVER-CONTRIBUTED
        // shape wearing FADED's name.
        let mut rows = Vec::new();
        let mut rules: Vec<&str> = SWEEP_RULES.to_vec();
        rules.push("true_fade");
        rules.push("never_rule");
        let rules: Vec<&str> = rules;
        for (i, ts) in ["s1", "s2", "s3"].iter().enumerate() {
            let mut row = sweep_row(3, ts, &rules, 0);
            for o in &mut row.outcomes {
                match o.rule.as_str() {
                    // FADED: resolves exactly once, then zero tail.
                    "true_fade" => o.resolved = u32::from(i == 0),
                    // Every other rule resolves every sweep (healthy).
                    "never_rule" => o.resolved = 0,
                    _ => o.resolved = 2,
                }
            }
            rows.push(row);
        }
        let scan = fade_scan(&rows);
        assert!(scan.refused.is_none());
        assert_eq!(scan.sweep_rows, 3);
        assert_eq!(scan.candidates.len(), 1, "exactly the planted fade");
        let c = &scan.candidates[0];
        assert_eq!(c.rule, "true_fade");
        assert_eq!(c.sweeps, 3);
        assert_eq!(c.resolved_total, 1);
        assert_eq!(c.resolved_sweeps, 1);
        assert_eq!(c.trailing_zero_sweeps, 2);
        assert_eq!(scan.never_contributed, vec!["never_rule".to_string()]);
    }

    #[test]
    fn healthy_thin_and_recovered_rules_are_never_flagged() {
        // healthy: resolves from sweep 1 on. thin: only in 2 sweeps.
        // recovered: persist-only for 2 sweeps, then resolves on the 3rd —
        // the tail is a resolution, so neither FADED nor never-contributed
        // (fade ≠ saturation is the classifier's core assertion, G2).
        let rules: Vec<&str> = SWEEP_RULES
            .iter()
            .copied()
            .chain(["healthy_rule", "recovered_rule"])
            .collect();
        let mut rows = Vec::new();
        for (i, ts) in ["s1", "s2", "s3"].iter().enumerate() {
            let mut row = sweep_row(3, ts, &rules, 0);
            for o in &mut row.outcomes {
                match o.rule.as_str() {
                    // healthy_rule rides the `_` arm (2, like every other
                    // healthy rule)
                    "recovered_rule" if i == 2 => o.resolved = 1,
                    "thin_rule" => o.resolved = 0,
                    _ => o.resolved = 2,
                }
            }
            // thin_rule joins only the first two sweeps.
            if i < 2 {
                row.outcomes.push(FadeOutcome {
                    rule: "thin_rule".to_string(),
                    resolved: 0,
                    persist: 1,
                    edits: 1,
                });
            }
            rows.push(row);
        }
        let scan = fade_scan(&rows);
        assert!(scan.refused.is_none());
        assert!(
            scan.candidates.is_empty(),
            "all rules healthy/thin/recovered"
        );
        assert_eq!(scan.never_contributed, Vec::<String>::new());
        // A genuinely all-zero rule lands in NEVER-CONTRIBUTED — printed,
        // triage-fork territory, never in the FADED table.
        for (i, row) in rows.iter_mut().enumerate() {
            row.outcomes.push(FadeOutcome {
                rule: "late_fade".to_string(),
                resolved: 0,
                persist: 1,
                edits: 1,
            });
            let _ = i;
        }
        let scan = fade_scan(&rows);
        assert!(
            scan.candidates.is_empty(),
            "all-zero is NEVER-CONTRIBUTED, not FADED"
        );
        assert_eq!(scan.never_contributed, vec!["late_fade".to_string()]);
    }

    #[test]
    fn zero_variance_routine_lanes_never_reach_the_fade_population() {
        // The measured gen-1 shape: 44 rows attributing 5 rules, zero
        // variance — plus the 2 real sweeps. The routine lanes contribute
        // NOTHING to the sweep population and the section stays REFUSED.
        let mut rows = Vec::with_capacity(44);
        for i in 0..44 {
            rows.push(FadeRow {
                generation: 1,
                ts: format!("routine{i}"),
                outcomes: [
                    ("deref_addrof", 1u32, 0u32),
                    ("filter_next", 1, 0),
                    // The measured tuple: (2,1,1,0,0) — persist=1,
                    // resolved 1/2, constant across all 44 rows (the
                    // day-one false positive a row-keyed fade would flag).
                    ("map_unwrap_or", 1, 1),
                    ("needless_return", 1, 0),
                    ("doc_markdown", 2, 0),
                ]
                .iter()
                .map(|(r, resolved, persist)| FadeOutcome {
                    rule: (*r).to_string(),
                    resolved: *resolved,
                    persist: *persist,
                    edits: 1,
                })
                .collect(),
            });
        }
        rows.push(sweep_row(3, "sweep1", &SWEEP_RULES, 1));
        rows.push(sweep_row(3, "sweep2", &SWEEP_RULES, 2));
        let scan = fade_scan(&rows);
        assert_eq!(scan.sweep_rows, 2);
        assert!(scan.refused.is_some(), "2 sweeps < 3 → loud refusal");
        assert!(
            scan.candidates.is_empty(),
            "no candidates from a refused scan"
        );
        assert!(scan.refused.as_ref().unwrap().contains("REFUSED"));
    }

    #[test]
    fn fade_section_renders_refusal_live_and_never_contributed_states_loudly() {
        // Refused: the reason prints, no table.
        let refused = FadeScan {
            sweep_rows: 2,
            sweep_notes: vec!["gen 3 @ a (36 rules)".into(), "gen 3 @ b (47 rules)".into()],
            candidates: Vec::new(),
            never_contributed: Vec::new(),
            refused: Some("REFUSED — needs ≥3".into()),
        };
        let text = render_fade_section(&refused);
        assert!(text.contains("REFUSED"));
        assert!(text.contains("gen 3 @ a (36 rules)"));
        assert!(
            !text.contains("| rule |"),
            "no candidate table when refused"
        );
        // Live + empty: the honest all-healthy line, never a zero table
        // masquerading as a measurement.
        let live = FadeScan {
            sweep_rows: 3,
            sweep_notes: vec![],
            candidates: Vec::new(),
            never_contributed: Vec::new(),
            refused: None,
        };
        let text = render_fade_section(&live);
        assert!(text.contains("no FADED candidates over 3 sweep rows"));
        // Never-contributed prints its OWN bucket with the triage wording,
        // never inside the FADED table.
        let split = FadeScan {
            sweep_rows: 3,
            sweep_notes: vec![],
            candidates: Vec::new(),
            never_contributed: vec!["p_zero_rule".to_string()],
            refused: None,
        };
        let text = render_fade_section(&split);
        assert!(text.contains("never-contributed: 1 rule(s)"));
        assert!(text.contains("p_zero_rule"));
        assert!(text.contains("triage fork"));
    }

    #[test]
    fn append_snapshot_dedups_unchanged_state_and_persists_change() {
        let dir = std::env::temp_dir().join(format!("frontier_snap_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("snapshots.jsonl");
        let (summary, feeds) = snap_fixtures();

        // First append: state never seen → row #1.
        let a = snapshot_from(
            &summary,
            &feeds,
            8,
            "aaaa".to_string(),
            "t1".to_string(),
            "storeaa".to_string(),
        );
        assert!(append_snapshot(&path, &a).unwrap());
        // Same state, different ts only → deduped, file unchanged.
        let a2 = snapshot_from(
            &summary,
            &feeds,
            8,
            "aaaa".to_string(),
            "t2".to_string(),
            "storeaa".to_string(),
        );
        assert!(!append_snapshot(&path, &a2).unwrap());
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 1);

        // Two stores with identical stats are TWO states, not one — the
        // 2026-09-21 lesson (the pooled file's x-axis was non-monotone on
        // unnamed stores): a different store id appends even at the same
        // generation/population/band.
        let other_store = snapshot_from(
            &summary,
            &feeds,
            8,
            "aaaa".to_string(),
            "t2b".to_string(),
            "storebb".to_string(),
        );
        assert!(append_snapshot(&path, &other_store).unwrap());

        // Changed state (generation bump) → row #3.
        let b = snapshot_from(
            &summary,
            &feeds,
            9,
            "aaaa".to_string(),
            "t3".to_string(),
            "storeaa".to_string(),
        );
        assert!(append_snapshot(&path, &b).unwrap());
        // Unchanged again → deduped against row #3 (the LAST row, not #1).
        assert!(!append_snapshot(&path, &b).unwrap());
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 3);

        // A malformed trailing line never blocks the append (loud skip).
        std::fs::write(&path, "not json\n").unwrap();
        assert!(append_snapshot(&path, &a).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Issue 089 T4 hardening: the bench-tail boundary snapshot ────────

    /// Seed a store the way the bench tail finds one: two learn rules
    /// (`band_rule` 1/1 → p̂=0.5, in-band; `sat_rule` 3/0 → p̂=1, above) +
    /// score runs at the given generations. Verdicts and runs seed
    /// SEPARATELY so a test can raise the generation frontier without
    /// moving the band (the exact boundary-capture case).
    #[cfg(feature = "latent_retrieval")]
    fn seed_bench_tail_verdicts(root: &std::path::Path) {
        let (store, _) = crate::traj_store::TrajStore::open(root).expect("store opens");
        store
            .record("band_rule", "shape-a", "fix-a", true, true, "run-1")
            .expect("record success");
        store
            .record("band_rule", "shape-a", "fix-a", false, false, "run-2")
            .expect("record fail");
        for run in ["run-1", "run-2", "run-3"] {
            store
                .record("sat_rule", "shape-b", "fix-b", true, true, run)
                .expect("record saturated success");
        }
    }

    #[cfg(feature = "latent_retrieval")]
    fn seed_bench_tail_runs(root: &std::path::Path, generations: &[u8]) {
        let (store, _) = crate::traj_store::TrajStore::open(root).expect("store opens");
        for (i, g) in generations.iter().enumerate() {
            store
                .record_score_run(&crate::traj_store::ScoreRunRow {
                    ts: format!("2026-09-13T00:00:{i:02}"),
                    sha: String::new(),
                    n_ran: 1,
                    n_healable: 1,
                    n_clean: 0,
                    heal_rate: 1.0,
                    created_rate: 0.0,
                    parse_safety: 1.0,
                    decline_correctness: 1.0,
                    edits_total: 1,
                    vocab_size: 1,
                    generation: *g,
                    recorded_unix: 1_800_000_000 + i as u64,
                    rustc_version: None,
                    clippy_driver_version: None,
                })
                .expect("record score run");
        }
    }

    #[cfg(feature = "latent_retrieval")]
    fn bench_tail_tmp(name: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("frontier_bench_tail_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn bench_tail_snapshot_missing_store_is_a_skip_not_a_fabricated_row() {
        let dir = bench_tail_tmp("nostore");
        let out = append_snapshot_at_frontier(&dir, &dir.join("snap.jsonl")).unwrap();
        assert_eq!(out, SnapshotOutcome::NoStore);
        assert!(!dir.join("snap.jsonl").exists(), "no row without a store");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn bench_tail_snapshot_zero_learn_rows_is_a_skip_not_a_green_zero_row() {
        let dir = bench_tail_tmp("empty");
        // Opening creates the store — but no verdicts means no learn rows.
        let _ = crate::traj_store::TrajStore::open(&dir).expect("open creates the store");
        let out = append_snapshot_at_frontier(&dir, &dir.join("snap.jsonl")).unwrap();
        assert_eq!(out, SnapshotOutcome::EmptyPopulation);
        assert!(
            !dir.join("snap.jsonl").exists(),
            "no row over an empty population"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn bench_tail_snapshot_appends_dedups_and_raises_with_the_generation_frontier() {
        let dir = bench_tail_tmp("frontier");
        seed_bench_tail_verdicts(&dir);
        seed_bench_tail_runs(&dir, &[1, 8]);
        let path = dir.join("snap.jsonl");

        // First call: appended, at the FRONTIER (8), not the latest lane (1).
        let first = append_snapshot_at_frontier(&dir, &path).unwrap();
        assert_eq!(first, SnapshotOutcome::Appended { generation: 8 });
        let text = std::fs::read_to_string(&path).unwrap();
        let snap: FrontierSnapshot = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(snap.generation, 8);
        assert_eq!(snap.population, 2);
        assert_eq!(snap.seen, 2);
        assert_eq!(snap.band_count, 1); // band_rule p̂=0.5, sat_rule above
        let rows = text.lines().filter(|l| !l.trim().is_empty()).count();
        assert_eq!(rows, 1);

        // Same store, same generation → deduped (the lane-interleaving case:
        // a v1 rerun after the v8 boundary appends nothing).
        let second = append_snapshot_at_frontier(&dir, &path).unwrap();
        assert_eq!(second, SnapshotOutcome::Unchanged { generation: 8 });
        let rows = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        assert_eq!(rows, 1, "no second row for an unchanged boundary");

        // A NEW generation lands in the store → a new row at the new
        // frontier EVEN with an identical band (verdicts untouched — only
        // the score run was added): the boundary-capture law this helper
        // exists for.
        seed_bench_tail_runs(&dir, &[1, 8, 9]);
        let third = append_snapshot_at_frontier(&dir, &path).unwrap();
        assert_eq!(third, SnapshotOutcome::Appended { generation: 9 });
        let text = std::fs::read_to_string(&path).unwrap();
        let last: FrontierSnapshot =
            serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
        assert_eq!(last.generation, 9);
        assert_eq!(last.band_count, snap.band_count, "same band, new boundary");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
