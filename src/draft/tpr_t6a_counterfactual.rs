//! Issue 062 **T6a** — the preregistered counterfactual-surgery gate that
//! decides whether T6 (zero-recompute miss attribution on
//! [`LatentFixMemory`] trajectories) may ship at all.
//!
//! Bench 065 measured TPR's withheld-pair readout at **chance on unseen
//! `(role, filler)` pairs** and 63.3% on seen ones (best readable `d`). T6
//! as written would feed the mining intake with attributions built on that
//! readout, so the owner re-scope (Issue 062, 2026-09-03) gates T6 on THIS
//! measurement: does the readout interpolate usefully on the population it
//! would actually serve — persist-miss spans?
//!
//! ## Protocol (preregistered in `.plans/070` BEFORE the first run)
//!
//! 1. **Fit** — the T4a corpus (`clippy_lints` + single-lint fixtures + the
//!    Plan 049 real-repo spans, [`StateSpace::TokenBag256`], role-coalesced
//!    at `T = 8`), BIC's `d` among admissible grid points — the SAME
//!    geometry Bench 065 scored, so the two benches are comparable.
//! 2. **Queries** — the frozen score-bench fixtures freshly run at HEAD,
//!    embedded with the same `TokenBag256.embed_into`. A fixture with ≥1
//!    resolved AND ≥1 persisted vocabulary lint enters the gate (the mixed
//!    population: gold healers exist AND hard negatives competed). One
//!    query per (fixture, healed rule): the counterfactual pair
//!    `(shape_of_span, g)`.
//! 3. **Readout** — recover the span's fitted filler at its shape role
//!    ([`unbind_state_into`]) and score candidate rule `Y` by
//!    `‖f_hat − f_Y‖`. This is the closed-form surgery magnitude: for
//!    orthogonal schemes the surgery edit norm `‖W_r(f_Y − f_hat)‖` is
//!    monotone in `‖f_Y − f_hat‖` (`W_rᵀ W_r = I_d`), so the surgery
//!    phrasing and the distance phrasing produce the IDENTICAL ranking —
//!    and the distance form is the one already validated by Bench 065's
//!    machinery.
//! 4. **Candidates** — per fixture, the fillers with an OBSERVED pair at
//!    the span's shape role. The readout is never asked about an
//!    unobserved pair; unobserved candidates do not dilute the ranking.
//! 5. **Split** — gold `g` is an observed query iff `(s_f, g)` is a corpus
//!    pair; otherwise it is counted and NEVER enters the gate (Bench 065
//!    measured that readout at chance). Role-absent fixtures (shape class
//!    missing from the coalesced vocabulary) and out-of-vocabulary golds
//!    are counted the same way.
//! 6. **Null** — the frequency prior over rules: corpus state count per
//!    filler, outcome-blind. "Guess the most common healer." Ties break by
//!    lower filler id; the distance argmin breaks ties with
//!    [`f32::total_cmp`] + lower filler id (deterministic).
//!
//! ## Preregistered verdict
//!
//! - **PASS** — `margin ≥ T6A_MARGIN_PP` **and** `margin − 100/n ≥
//!   T6A_MARGIN_PP` (the one-flip cushion, same shape as T4a's). T6 may be
//!   wired, on the observed half only, with the refusal contract for
//!   unobserved pairs.
//! - **FAIL** — the bar is not cleared on a readable instrument. T6 closes
//!   REFUTED the way T4a closed T4. No widen-and-retry.
//! - **INCONCLUSIVE** — margin clears the bar but one flip crosses it; or
//!   the gate population is empty; or the instrument is unreadable (no
//!   admissible `d`, fit error). Instrument failure can never make a FAIL
//!   — the same one-directional rule T4a's amendment A1 encoded.
//!
//! `T6A_MARGIN_PP = 20.0` — deliberately the same bar as T4a's
//! [`T4A_MARGIN_PP`] and katgpt-rs Issue 707 G8, so this gate is not a
//! softer version of its family.
//!
//! [`LatentFixMemory`]: crate::self_evolve::LatentFixMemory
//! [`T4A_MARGIN_PP`]: super::tpr_t4a_withheld::T4A_MARGIN_PP
//! [`unbind_state_into`]: katgpt_core::tpr::unbind_state_into

use std::collections::{BTreeMap, BTreeSet};

use katgpt_core::tpr::{AlsConfig, ObservedPairs, TprScratch, als_fit, unbind_state_into};

use super::tpr_fit::{OTHER_ROLE, StateSpace, build_corpus, coalesce_for, run};
use super::tpr_t4a_withheld::{COALESCE_T, DS, T4A_MARGIN_PP, T4aVerdict};
use super::withheld_pair_ood::{Binding, classify_shape};

/// The bar, in percentage points, the counterfactual readout must beat the
/// frequency prior by. Aliased to T4a's constant — one family, one bar.
pub const T6A_MARGIN_PP: f32 = T4A_MARGIN_PP;

/// One score-bench fixture, reduced to what the gate needs. The harness
/// (the test) derives `healed`/`persisted` from a fresh
/// `crate::score_bench` run; this module never depends on the bench so the
/// gate math stays separately testable.
#[derive(Debug, Clone)]
pub struct FixtureOutcome {
    pub id: String,
    pub code: String,
    /// Vocabulary lints the healer RESOLVED — the gold healer(s).
    pub healed: Vec<String>,
    /// Vocabulary lints that SURVIVED healing — the hard negatives.
    pub persisted: Vec<String>,
}

/// Why a (fixture, gold) query cannot enter the gate. Counted, never
/// silently dropped — the same discipline as T4a's `SkipReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryExclusion {
    /// The gold pair `(shape, gold)` is not a corpus pair at this role —
    /// Bench 065 measured the readout at chance there; a number would be a
    /// prediction, not a readout.
    UnobservedPair,
    /// The fixture's shape class is absent from the coalesced vocabulary —
    /// no role block to unbind at.
    RoleAbsent,
    /// The gold lint is not a filler of the fit corpus (cannot happen when
    /// the healer's vocabulary IS the corpus vocabulary — counted to prove
    /// it didn't).
    GoldOutOfVocabulary,
}

/// One gate query's outcome.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub fixture: String,
    pub gold: String,
    /// Candidate-set size at this fixture's role (observed pairs only).
    pub candidates: usize,
    /// The argmin candidate is a gold rule.
    pub tpr_hit: bool,
    /// The most frequent candidate is a gold rule.
    pub prior_hit: bool,
    /// Best gold's 1-based rank under the readout (1 = top) — the MRR
    /// diagnostic.
    pub tpr_rank: usize,
}

/// The gate's output.
#[derive(Debug, Clone)]
pub struct T6aReport {
    /// BIC's `d` among the admissible grid points — the geometry Bench 065
    /// scored.
    pub d: usize,
    pub n_corpus_states: usize,
    pub m_roles: usize,
    pub n_fillers: usize,
    /// Fixtures that entered the gate population (mixed: ≥1 healed AND ≥1
    /// persisted).
    pub gate_fixtures: usize,
    /// Their ids — the gate population is small enough that the record
    /// should name it (Bench 066's starvation finding lives or dies on
    /// these rows).
    pub gate_fixture_ids: Vec<String>,
    /// Fixtures reported but not gated, by shape.
    pub all_healed_fixtures: usize,
    pub all_persisted_fixtures: usize,
    /// Gold queries that entered the gate.
    pub queries: Vec<QueryResult>,
    /// Gold queries excluded, by reason.
    pub exclusions: Vec<(String, String, QueryExclusion)>,
    /// Persisted-lint census over ALL scored fixtures — what the healer
    /// left behind (the documented divergence classes should dominate it).
    /// This is the population T6's attribution would serve.
    pub persist_census: Vec<(String, usize)>,
}

impl T6aReport {
    #[must_use]
    pub fn gate_queries(&self) -> usize {
        self.queries.len()
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn tpr_top1(&self) -> f32 {
        match self.gate_queries() {
            0 => 0.0,
            n => self.queries.iter().filter(|q| q.tpr_hit).count() as f32 / n as f32,
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn prior_top1(&self) -> f32 {
        match self.gate_queries() {
            0 => 0.0,
            n => self.queries.iter().filter(|q| q.prior_hit).count() as f32 / n as f32,
        }
    }

    /// Mean reciprocal rank of the best gold under the readout — the
    /// diagnostic the binary top-1 hides.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn tpr_mrr(&self) -> f32 {
        match self.gate_queries() {
            0 => 0.0,
            n => {
                self.queries
                    .iter()
                    .map(|q| 1.0 / q.tpr_rank.max(1) as f32)
                    .sum::<f32>()
                    / n as f32
            }
        }
    }

    #[must_use]
    pub fn margin_pp(&self) -> f32 {
        (self.tpr_top1() - self.prior_top1()) * 100.0
    }

    /// Points one query flip is worth — the width of "a single flip".
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn one_flip_pp(&self) -> f32 {
        match self.gate_queries() {
            0 => f32::INFINITY,
            n => 100.0 / n as f32,
        }
    }

    /// The preregistered decision (see the module docs). An empty gate
    /// population is INCONCLUSIVE — there is nothing to measure, which is a
    /// fact about the population and not a refutation.
    #[must_use]
    pub fn verdict(&self) -> T4aVerdict {
        if self.gate_queries() == 0 {
            return T4aVerdict::Inconclusive;
        }
        let margin = self.margin_pp();
        match (
            margin >= T6A_MARGIN_PP,
            margin - self.one_flip_pp() >= T6A_MARGIN_PP,
        ) {
            (false, _) => T4aVerdict::Fail,
            (true, false) => T4aVerdict::Inconclusive,
            (true, true) => T4aVerdict::Pass,
        }
    }
}

/// Run T6a. `None` when the instrument is unreadable (no admissible `d`,
/// or the fit errored) — the caller must not paper over that with a
/// default.
///
/// # Panics
///
/// Panics if the corpus carries no roles — the T4a corpus cannot produce
/// that from a non-empty population.
#[must_use]
pub fn run_t6a(population: Vec<Binding>, fixtures: &[FixtureOutcome]) -> Option<T6aReport> {
    let bench = run(
        population.clone(),
        StateSpace::TokenBag256,
        "clippy_lints",
        &DS,
        Some(COALESCE_T),
    );
    let d = bench.chosen_d?;
    run_t6a_at(population, fixtures, d)
}

/// The gate at an explicit `d` — the diagnostic entry point (mirrors
/// T4a's `run_t4a_at`; BIC's `d` is the preregistered geometry, this one
/// exists to answer follow-up questions without re-litigating the gate).
///
/// # Panics
///
/// Panics if the corpus carries no roles.
#[must_use]
pub fn run_t6a_at(
    population: Vec<Binding>,
    fixtures: &[FixtureOutcome],
    chosen_d: usize,
) -> Option<T6aReport> {
    let base = build_corpus(population, StateSpace::TokenBag256);
    let (corpus, _) = coalesce_for(&base, COALESCE_T);
    let m_roles = corpus.vocab.roles.len();
    assert!(m_roles > 0, "empty role vocabulary");

    let Ok((art, _)) = als_fit(corpus.input(), &AlsConfig::new(chosen_d, corpus.scheme())) else {
        return None;
    };

    // The coalesced role lookup. Shapes merged below the threshold live
    // under `OTHER_ROLE` when the merge kept it; shapes absent from the
    // pre-coalesce vocabulary have no block at all.
    let base_roles: BTreeSet<&str> = base.vocab.roles.iter().copied().collect();
    let role_id: BTreeMap<&str, u16> = corpus
        .vocab
        .roles
        .iter()
        .copied()
        .enumerate()
        .map(|(i, r)| (r, u16::try_from(i).expect("role ids fit u16")))
        .collect();
    let role_of = |shape: &str| -> Option<u16> {
        if let Some(&r) = role_id.get(shape) {
            return Some(r);
        }
        if base_roles.contains(shape) && role_id.contains_key(OTHER_ROLE) {
            return Some(role_id[OTHER_ROLE]);
        }
        None
    };

    let filler_id: BTreeMap<&str, u16> = corpus
        .vocab
        .fillers
        .iter()
        .enumerate()
        .map(|(i, f)| (f.as_str(), u16::try_from(i).expect("filler ids fit u16")))
        .collect();

    let observed = ObservedPairs::from_bindings(&corpus.bindings);
    // Outcome-blind frequency prior: corpus state count per filler.
    let freq: Vec<usize> = corpus.filler_states.clone();

    let mut scratch = TprScratch::new(&art);

    let mut queries: Vec<QueryResult> = Vec::new();
    let mut exclusions: Vec<(String, String, QueryExclusion)> = Vec::new();
    let mut gate_fixtures = 0usize;
    let mut gate_fixture_ids: Vec<String> = Vec::new();
    let mut all_healed_fixtures = 0usize;
    let mut all_persisted_fixtures = 0usize;
    let mut persist_census: BTreeMap<String, usize> = BTreeMap::new();

    for fx in fixtures {
        for p in &fx.persisted {
            *persist_census.entry(p.clone()).or_default() += 1;
        }
        match (fx.healed.is_empty(), fx.persisted.is_empty()) {
            (true, _) => all_persisted_fixtures += 1,
            (false, true) => all_healed_fixtures += 1,
            (false, false) => {
                gate_fixtures += 1;
                gate_fixture_ids.push(fx.id.clone());
            }
        }
        // Not the gate population? All-persist has no gold; all-healed has
        // no competing negative. The persist census above still records
        // their misses — that is the population T6's attribution serves.
        if fx.healed.is_empty() || fx.persisted.is_empty() {
            continue;
        }

        // The span's state + shape role → the per-fixture gate readout.
        match gate_fixture(
            &art,
            &mut scratch,
            fx,
            &role_of,
            &filler_id,
            &observed,
            &freq,
        ) {
            Ok(qs) => queries.extend(qs),
            Err(exs) => exclusions.extend(exs),
        }
    }

    Some(T6aReport {
        d: chosen_d,
        n_corpus_states: corpus.n_states(),
        m_roles,
        n_fillers: corpus.vocab.fillers.len(),
        gate_fixtures,
        gate_fixture_ids,
        all_healed_fixtures,
        all_persisted_fixtures,
        queries,
        exclusions,
        persist_census: persist_census.into_iter().collect(),
    })
}

/// One mixed fixture through the gate: recover its fitted filler at its
/// shape role, rank the observed candidates by `‖f_hat − f_Y‖`, and resolve
/// the golds. `Ok` carries one [`QueryResult`] per gate-eligible gold; `Err`
/// carries the (fixture, gold, reason) exclusions — counted, never dropped
/// silently.
#[allow(clippy::too_many_lines)]
fn gate_fixture(
    art: &katgpt_core::tpr::TprArtifact,
    scratch: &mut TprScratch,
    fx: &FixtureOutcome,
    role_of: &dyn Fn(&str) -> Option<u16>,
    filler_id: &BTreeMap<&str, u16>,
    observed: &ObservedPairs,
    freq: &[usize],
) -> Result<Vec<QueryResult>, Vec<(String, String, QueryExclusion)>> {
    let mut exs: Vec<(String, String, QueryExclusion)> = Vec::new();
    let push_ex = |exs: &mut Vec<(String, String, QueryExclusion)>, g: &str, e: QueryExclusion| {
        exs.push((fx.id.clone(), g.to_string(), e));
    };

    // The span's state + shape role.
    let dim = art.dim;
    let mut state = vec![0.0f32; dim];
    StateSpace::TokenBag256.embed_into(&fx.code, &mut state);
    let shape = classify_shape(&fx.code).label();
    let Some(role) = role_of(shape) else {
        for g in &fx.healed {
            push_ex(&mut exs, g, QueryExclusion::RoleAbsent);
        }
        return Err(exs);
    };

    // The span's fitted filler at its shape role — the closed-form surgery
    // source (`f_old`), recovered from the state, not assumed.
    let mut f_hat = vec![0.0f32; art.d];
    if unbind_state_into(art, &state, role, scratch, &mut f_hat).is_err() {
        for g in &fx.healed {
            push_ex(&mut exs, g, QueryExclusion::RoleAbsent);
        }
        return Err(exs);
    }

    // Candidates: fillers with an OBSERVED pair at this role.
    let mut cands: Vec<u16> = (0..u16::try_from(art.n_fillers).expect("filler count fits u16"))
        .filter(|&v| observed.contains(role, v))
        .collect();
    if cands.is_empty() {
        for g in &fx.healed {
            push_ex(&mut exs, g, QueryExclusion::UnobservedPair);
        }
        return Err(exs);
    }

    // Gold resolution — each gold either enters the gate (observed at this
    // role, hence in the candidate set by construction) or is excluded with
    // its reason.
    let mut gold_in: Vec<u16> = Vec::new();
    let mut gold_in_names: Vec<String> = Vec::new();
    for g in &fx.healed {
        match filler_id.get(g.as_str()) {
            None => push_ex(&mut exs, g, QueryExclusion::GoldOutOfVocabulary),
            Some(&v) if observed.contains(role, v) => {
                gold_in.push(v);
                gold_in_names.push(g.clone());
            }
            Some(_) => push_ex(&mut exs, g, QueryExclusion::UnobservedPair),
        }
    }
    if gold_in.is_empty() {
        return Err(exs); // every gold excluded above, each with its reason
    }

    // Distance ranking. `‖f_hat − f_Y‖` over each candidate's shared filler
    // row; `total_cmp` + lower id breaks ties deterministically. This IS the
    // closed-form surgery magnitude: `‖W_r(f_Y − f_hat)‖` is monotone in
    // `‖f_Y − f_hat‖` for orthogonal schemes (`W_rᵀ W_r = I_d`).
    #[allow(clippy::cast_precision_loss)]
    let dist = |v: u16| -> f32 {
        let row = &art.fillers[v as usize * art.d..(v as usize + 1) * art.d];
        f_hat
            .iter()
            .zip(row.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            .sqrt()
    };
    cands.sort_by(|&a, &b| dist(a).total_cmp(&dist(b)).then(a.cmp(&b)));

    let best = cands[0];
    let tpr_hit = gold_in.contains(&best);
    let tpr_rank = gold_in
        .iter()
        .map(|g| {
            cands
                .iter()
                .position(|&c| c == *g)
                .expect("gold_in ⊆ candidates")
                + 1
        })
        .min()
        .unwrap_or(usize::MAX);
    // Frequency prior: the most corpus-frequent candidate, ties → lower id.
    let prior_best = cands
        .iter()
        .copied()
        .max_by(|&a, &b| freq[a as usize].cmp(&freq[b as usize]).then(b.cmp(&a)))
        .expect("candidates non-empty");
    let prior_hit = gold_in.contains(&prior_best);

    Ok(gold_in_names
        .into_iter()
        .map(|gold| QueryResult {
            fixture: fx.id.clone(),
            gold,
            candidates: cands.len(),
            tpr_hit,
            prior_hit,
            tpr_rank,
        })
        .collect())
}

/// The `--nocapture` table.
pub fn print_t6a(r: &T6aReport) {
    println!(
        "\n── Issue 062 T6a: counterfactual-surgery gate ──  clippy_lints / TokenBag256 [coalesced T={COALESCE_T}]"
    );
    println!(
        "  corpus: n_states={} m(roles)={} n_fillers={} D={} d={} (BIC, admissible)",
        r.n_corpus_states,
        r.m_roles,
        r.n_fillers,
        StateSpace::TokenBag256.dim(),
        r.d
    );
    println!(
        "  fixtures: gate {} {:?} (mixed) | all-healed {} (not gated) | all-persisted {} (no gold)",
        r.gate_fixtures, r.gate_fixture_ids, r.all_healed_fixtures, r.all_persisted_fixtures
    );
    println!(
        "  queries: gate {} | excluded {} (unobserved-pair {} / role-absent {} / gold-oov {})",
        r.gate_queries(),
        r.exclusions.len(),
        r.exclusions
            .iter()
            .filter(|(_, _, e)| matches!(e, QueryExclusion::UnobservedPair))
            .count(),
        r.exclusions
            .iter()
            .filter(|(_, _, e)| matches!(e, QueryExclusion::RoleAbsent))
            .count(),
        r.exclusions
            .iter()
            .filter(|(_, _, e)| matches!(e, QueryExclusion::GoldOutOfVocabulary))
            .count(),
    );
    println!(
        "  TPR top1 {:.1}% | frequency-prior top1 {:.1}% | TPR MRR {:.3} | margin {:+.1} pp \
         (bar ≥ {T6A_MARGIN_PP:.0}, one flip = {:.1} pp)",
        r.tpr_top1() * 100.0,
        r.prior_top1() * 100.0,
        r.tpr_mrr(),
        r.margin_pp(),
        r.one_flip_pp()
    );
    println!("  verdict: {}", r.verdict().label());
    if !r.persist_census.is_empty() {
        println!("  persisted-lint census (the population T6's attribution would serve):");
        for (lint, n) in r.persist_census.iter().take(10) {
            println!("    {lint}: {n}");
        }
    }
}
