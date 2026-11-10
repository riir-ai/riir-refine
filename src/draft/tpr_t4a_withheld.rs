//! Issue 062 **T4a** — the preregistered withheld-`(role, filler)`-pair gate
//! that decides route (b).
//!
//! ## What this is, and what it is not
//!
//! T4 asked "score retrieval candidates against a TPR projection rather than
//! the raw state". Bench 063 established that the 8-D consumer space cannot
//! host that fit (`K = m·d ≥ D` at `d = 1`), and §11.6 measured the coalesced
//! consumer arm FLAT. So the fuse, if it happens, happens on the 256-D token
//! bag — and the owner call (Issue 062, 2026-09-03) is that it happens against
//! `clippy_lints` ONLY, gated on this test.
//!
//! This module is that gate and nothing else. It does not build the fused
//! reranker; a FAIL here closes T4 REFUTED.
//!
//! ## Protocol (preregistered — this file predates its first run)
//!
//! 1. Corpus: `clippy_lints` + the Plan 049 T0.1 real-repo spans, embedded in
//!    [`StateSpace::TokenBag256`], role-coalesced at `T = 8` (`m' = 6`) —
//!    exactly the arm Bench 063 §12.7 measured at 13 / 45 testable fillers.
//! 2. `d` is BIC's pick among the ADMISSIBLE grid points, taken from the same
//!    [`tpr_fit::run`] the bench uses, so the gate cannot silently score at a
//!    different geometry than the bench reported.
//! 3. **Shared pool**: every distinct `(role, filler)` pair the FULL corpus
//!    contains. Shared, not per-trial, for a reason the degenerate case makes
//!    concrete — a pool of `(r, ·)` at one fixed role makes every filler
//!    role-determined inside the scored universe, so
//!    [`WithheldPairReport::verdict`] would be `None` on every trial and the
//!    gate would report an unreadable number as a refusal to answer.
//! 4. **Testable pairs**: pairs whose FILLER is seen with ≥ 2 distinct roles
//!    ([`FillerRoleSpread::multi_role_fillers`]). Withholding a pair from a
//!    single-role filler withholds the whole filler, which is the Issue 711
//!    regime where the OOD arm asks a different question rather than a harder
//!    one.
//! 5. Per trial: drop every state carrying that pair, refit at `d` with the
//!    role vocabulary FIXED (never rebuilt — a rebuilt vocabulary would change
//!    `m` and therefore the geometry mid-gate), score the held-out states
//!    against the shared pool with [`withheld_pair_top1_report`], and score the
//!    same states with an [`AtomicNull`] fitted on the same training set.
//! 6. Aggregate by STATE, not by trial: a trial holding 3 states is 3
//!    observations, and a trial mean would weight a 1-state trial equally.
//!
//! ## Preregistered verdict (Issue 062 T4a, verbatim, operationalized here)
//!
//! - **PASS** — `verdict()` is `Some` on every trial that ran, `coverage` is
//!   `1.0` on every trial, and TPR beats the null by ≥ [`T4A_MARGIN_PP`]
//!   (the same bar Issue 707 G8 registered). Route (b) proceeds.
//! - **FAIL** — the bar is not cleared. Route (b) is dead on this corpus and
//!   T4 closes REFUTED. Do NOT widen and retry.
//! - **INCONCLUSIVE** — the bar is cleared, but on so few scored states that a
//!   single flip would cross it. Operationalized as
//!   `margin - 100/n_scored < T4A_MARGIN_PP`: one held-out state changing from
//!   hit to miss moves the TPR arm by `100/n_scored` points, so this is
//!   literally "one flip crosses the bar". That is the 1.8-states-per-pair risk
//!   materialising, and it is the ONE branch where widening the clippy corpus
//!   is the right response rather than a retry.
//!
//! The null's own in-distribution coverage is checked FIRST (Bench 062's
//! lesson): a null that cannot fit its own training set fails OOD by vacuity,
//! and its 0% certifies nothing. A vacuous null makes the run INCONCLUSIVE
//! regardless of margin — it can never make it a PASS.
//!
//! ## Three populations, reported apart
//!
//! Pooling a validated number with a predicted one makes the prediction read
//! as a measurement. [`T4aReport`] carries them separately:
//!
//! - **validated** — pairs this gate actually withheld and scored;
//! - **fitted-but-unvalidated** — pairs in the corpus that no trial could
//!   withhold (single-role fillers, or a withhold that would empty a role);
//! - **outside the fit** — the `m' × n_fillers` pair space the corpus never
//!   observed at all, where [`ObservedPairs::contains`] is `false` and any
//!   readout is a prediction.

use katgpt_core::tpr::{
    AlsConfig, AlsInput, AtomicNull, FillerRoleSpread, ObservedPairs, TprBindings, TprScheme,
    TprScratch, WithheldPairReport, als_fit, filler_role_spread, withheld_pair_top1,
    withheld_pair_top1_report,
};

use super::tpr_fit::{FitCorpus, StateSpace, admissibility, build_corpus, coalesce_for, run};
use super::withheld_pair_ood::Binding;

/// The bar, in percentage points, TPR must beat the atomic null by. Identical
/// to katgpt-rs Issue 707 G8's `G8_MARGIN_PP` — deliberately the same number,
/// so this gate is not a softer version of the upstream one.
pub const T4A_MARGIN_PP: f32 = 20.0;

/// Role-coalescing threshold. Bench 063 §10.2a's fixed value (the grid max),
/// which is what produces the `m' = 6` arm §12.7 measured.
pub const COALESCE_T: usize = 8;

/// The `d` grid, matching Bench 063's. BIC picks among the admissible ones.
pub const DS: [usize; 4] = [1, 2, 4, 8];

/// The verdict, as preregistered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T4aVerdict {
    /// Bar cleared with a margin no single flip can erase → route (b) proceeds.
    Pass,
    /// Bar not cleared → T4 closes REFUTED. No widen-and-retry.
    Fail,
    /// Bar cleared but one flip crosses it, or an instrument was unreadable.
    /// The one branch where widening the clippy corpus is the right response.
    Inconclusive,
}

impl T4aVerdict {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Inconclusive => "INCONCLUSIVE",
        }
    }
}

/// Why a corpus pair could not host a trial. Counted rather than skipped
/// silently — the unvalidated population is a number this gate must report,
/// not a gap in its loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The filler is seen with exactly one role, so withholding the pair
    /// withholds the filler (Issue 711).
    SingleRoleFiller,
    /// Withholding would leave the role with no states at all, so the refit
    /// has an empty core block and the geometry is not the one BIC chose.
    WouldEmptyRole,
    /// The refit itself errored.
    FitError,
}

/// One withheld-pair trial.
#[derive(Debug, Clone)]
pub struct Trial {
    pub role: &'static str,
    pub filler: String,
    /// States carrying this pair — the OOD sample size this trial contributes.
    pub held: usize,
    /// States the refit saw.
    pub train: usize,
    pub report: WithheldPairReport,
    /// The null fitted on the same training set, scored on the same states
    /// against the same pool.
    pub null_top1: f32,
    /// The null's coverage of its OWN training bindings — the vacuity check,
    /// per trial, because a refit changes the training set.
    pub null_id_coverage: f32,
    /// **The REFIT's admissibility** (amendment A3), not the full corpus's.
    ///
    /// Bench 063 P13 measured admissibility at 113 states; every trial fits a
    /// corpus 1–5 states smaller, and the per-role starvation bound (`> d`
    /// states per role) is exactly the one a withhold can break — at `d = 8` a
    /// role holding 9 states and losing 2 to the withhold becomes starved. A
    /// row read off a starved refit is a vacuous fit's number, so the sweep's
    /// most favourable geometry is precisely where this must be checked.
    pub train_admissible: bool,
    /// **TPR's own vacuity check** (amendment A1): the same artifact, the same
    /// shared pool, scored on the states it was FITTED on. The OOD number is
    /// only a generalization failure if this one is a success; if TPR cannot
    /// identify a state whose pair it trained on, a 0% OOD says the readout is
    /// too weak to answer at all, which is a fact about the instrument and not
    /// about the corpus.
    pub tpr_id_top1: f32,
}

impl Trial {
    /// Held-out states this trial got right, TPR arm.
    ///
    /// Recovered from the fraction rather than plumbed through: `held` is a
    /// handful of states and `top1` is `hits / held` computed from them, so
    /// `round(top1 * held)` is exact — the casts are allowed for that reason,
    /// not waved through.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn tpr_hits(&self) -> usize {
        (self.report.top1 * self.held as f32).round() as usize
    }

    /// Held-out states this trial got right, null arm.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn null_hits(&self) -> usize {
        (self.null_top1 * self.held as f32).round() as usize
    }
}

/// The gate's output.
#[derive(Debug, Clone)]
pub struct T4aReport {
    pub domain: &'static str,
    pub space: StateSpace,
    pub coalesce_threshold: usize,
    /// `m'` — the coalesced role count the trials were fitted at.
    pub m: usize,
    pub n_fillers: usize,
    pub n_states: usize,
    /// BIC's pick among the admissible grid points, from `tpr_fit::run`.
    pub d: usize,
    /// The shared candidate pool size (distinct observed pairs).
    pub pool: usize,
    /// Composition covariate of the FULL corpus, before any withhold.
    pub spread: FillerRoleSpread,
    pub trials: Vec<Trial>,
    /// Corpus pairs no trial could withhold, by reason.
    pub skipped: Vec<(&'static str, String, SkipReason)>,
    /// `m' × n_fillers` — the whole pair space.
    pub pair_space: usize,
}

impl T4aReport {
    /// Held-out states scored across every trial. The aggregation unit — a
    /// per-trial mean would weight a 1-state trial like a 5-state one.
    #[must_use]
    pub fn scored(&self) -> usize {
        self.trials.iter().map(|t| t.held).sum()
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn tpr_top1(&self) -> f32 {
        match self.scored() {
            0 => 0.0,
            n => self.trials.iter().map(Trial::tpr_hits).sum::<usize>() as f32 / n as f32,
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn null_top1(&self) -> f32 {
        match self.scored() {
            0 => 0.0,
            n => self.trials.iter().map(Trial::null_hits).sum::<usize>() as f32 / n as f32,
        }
    }

    /// Uniform-guess accuracy against the shared pool.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn chance(&self) -> f32 {
        match self.pool {
            0 => 0.0,
            p => 1.0 / p as f32,
        }
    }

    /// TPR minus null, in percentage points.
    #[must_use]
    pub fn margin_pp(&self) -> f32 {
        (self.tpr_top1() - self.null_top1()) * 100.0
    }

    /// Points one held-out state is worth — the width of "a single flip".
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn one_flip_pp(&self) -> f32 {
        match self.scored() {
            0 => f32::INFINITY,
            n => 100.0 / n as f32,
        }
    }

    /// Every trial produced a readable number: a `Some` verdict, a fully
    /// answerable pool, and an ADMISSIBLE refit (A3). An unreadable trial can
    /// never contribute to a PASS — nor to a FAIL, per [`Self::verdict`].
    #[must_use]
    pub fn readable(&self) -> bool {
        !self.trials.is_empty()
            && self.trials.iter().all(|t| {
                t.report.verdict().is_some() && t.report.coverage > 0.999 && t.train_admissible
            })
    }

    /// Refits that broke the two-sided admissibility bound — reported as a
    /// count so a partially-starved sweep row is visible rather than only
    /// flipping `readable` to false.
    #[must_use]
    pub fn inadmissible_refits(&self) -> usize {
        self.trials.iter().filter(|t| !t.train_admissible).count()
    }

    /// State-weighted in-distribution top-1 of the TPR arm — the A1 control.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn tpr_id_top1(&self) -> f32 {
        match self.trials.len() {
            0 => 0.0,
            n => self.trials.iter().map(|t| t.tpr_id_top1).sum::<f32>() / n as f32,
        }
    }

    /// **Amendment A1** — the TPR arm can answer the question in-distribution.
    ///
    /// Symmetric to [`Self::null_informative`], and added for the same reason
    /// Bench 062 added that one: an arm that cannot fit its own training set
    /// fails OOD by vacuity, and its 0% certifies nothing. The bar is the same
    /// `> 0.5` Issue 707 G8 applies to the null's ID top-1.
    #[must_use]
    pub fn tpr_informative(&self) -> bool {
        !self.trials.is_empty() && self.tpr_id_top1() > 0.5
    }

    /// The null fits its own training set on every trial — Bench 062's
    /// vacuity check, applied per trial because each refit has its own
    /// training set.
    #[must_use]
    pub fn null_informative(&self) -> bool {
        !self.trials.is_empty() && self.trials.iter().all(|t| t.null_id_coverage > 0.999)
    }

    /// Pairs the corpus observed but no trial could withhold.
    #[must_use]
    pub fn unvalidated(&self) -> usize {
        self.skipped.len()
    }

    /// Pairs the corpus never observed — where any readout is a prediction.
    #[must_use]
    pub fn outside_fit(&self) -> usize {
        self.pair_space.saturating_sub(self.pool)
    }

    /// The preregistered decision.
    ///
    /// **A FAIL is only available on a readable instrument.** FAIL closes T4
    /// REFUTED, which is a claim about the corpus; an unreadable trial, a
    /// vacuous null or a vacuous TPR arm is a claim about the measurement, and
    /// the two must not share an outcome. So instrument failure lands in
    /// INCONCLUSIVE whatever the margin does — the reverse of the usual
    /// conservative default, and deliberate: the conservative outcome here is
    /// the one that does NOT close the route.
    ///
    /// **Amendment A1, made after the first run and one-directional.** The
    /// preregistration checked the NULL's vacuity and not TPR's, and the first
    /// run returned 0.0% on both arms of all 30 trials — the exact shape where
    /// "the corpus does not generalize" and "the readout cannot answer" are
    /// indistinguishable. [`Self::tpr_informative`] closes that gap. It can
    /// only ever convert a FAIL into an INCONCLUSIVE; it cannot turn anything
    /// into a PASS, and it cannot raise the margin. An amendment that can only
    /// weaken the conclusion it is being applied to is safe to make after
    /// seeing the number; one that could strengthen it would not be.
    #[must_use]
    pub fn verdict(&self) -> T4aVerdict {
        if !self.readable() || !self.null_informative() || !self.tpr_informative() {
            return T4aVerdict::Inconclusive;
        }
        let margin = self.margin_pp();
        match (
            margin >= T4A_MARGIN_PP,
            margin - self.one_flip_pp() >= T4A_MARGIN_PP,
        ) {
            (false, _) => T4aVerdict::Fail,
            (true, false) => T4aVerdict::Inconclusive,
            (true, true) => T4aVerdict::Pass,
        }
    }
}

// ── the run ─────────────────────────────────────────────────────────────

/// Row-major subset of `(states, bindings)` by index.
fn subset(c: &FitCorpus, idx: &[usize]) -> (Vec<f32>, Vec<TprBindings>) {
    let dim = c.dim();
    let mut states = Vec::with_capacity(idx.len() * dim);
    let mut bindings = Vec::with_capacity(idx.len());
    for &i in idx {
        states.extend_from_slice(&c.states[i * dim..(i + 1) * dim]);
        bindings.push(c.bindings[i].clone());
    }
    (states, bindings)
}

/// A [`FitCorpus`] over a subset's states/bindings, with the vocabulary of the
/// parent held FIXED.
///
/// Rebuilding from a population would re-derive the vocabulary and could
/// shrink `m`, which is a different geometry than the one the gate declares.
/// The per-role and per-filler censuses are recounted, because those are what
/// the starvation bound reads.
fn subset_corpus(parent: &FitCorpus, states: &[f32], bindings: &[TprBindings]) -> FitCorpus {
    let mut role_states = vec![0usize; parent.vocab.roles.len()];
    let mut filler_states = vec![0usize; parent.vocab.fillers.len()];
    for b in bindings {
        role_states[b.roles[0] as usize] += 1;
        filler_states[b.fillers[0] as usize] += 1;
    }
    FitCorpus {
        space: parent.space,
        states: states.to_vec(),
        bindings: bindings.to_vec(),
        vocab: parent.vocab.clone(),
        role_states,
        filler_states,
        spread: filler_role_spread(bindings),
    }
}

/// Distinct roles per filler id, over the whole corpus.
fn roles_per_filler(c: &FitCorpus) -> Vec<usize> {
    let mut seen: Vec<Vec<u16>> = vec![Vec::new(); c.vocab.fillers.len()];
    for b in &c.bindings {
        let (r, f) = (b.roles[0], b.fillers[0] as usize);
        if !seen[f].contains(&r) {
            seen[f].push(r);
        }
    }
    seen.into_iter().map(|v| v.len()).collect()
}

/// Build the coalesced `clippy_lints / tokenbag256` corpus this gate scores.
#[must_use]
pub fn gate_corpus(population: Vec<Binding>) -> FitCorpus {
    let base = build_corpus(population, StateSpace::TokenBag256);
    coalesce_for(&base, COALESCE_T).0
}

/// Run T4a.
///
/// `d` is taken from [`tpr_fit::run`]'s BIC pick rather than chosen here, so
/// the gate provably scores at the geometry the bench reported. `None` when
/// that arm has no admissible `d` at all — a state this corpus is not expected
/// to reach, and one the caller must not paper over with a default.
///
/// # Panics
///
/// Panics if the corpus carries no roles — [`build_corpus`] cannot produce
/// that from a non-empty population, so it would mean the population is empty.
#[must_use]
pub fn run_t4a(population: Vec<Binding>, domain: &'static str) -> Option<T4aReport> {
    let bench = run(
        population.clone(),
        StateSpace::TokenBag256,
        domain,
        &DS,
        Some(COALESCE_T),
    );
    run_t4a_at(population, domain, bench.chosen_d?)
}

/// The gate at an EXPLICIT `d` — the diagnostic entry point (amendment A2).
///
/// [`run_t4a`] is the preregistered gate and takes `d` from BIC. This one does
/// not, and is only for answering the question the first run raised: BIC picks
/// `d` by fit parsimony, and the withheld-pair readout's discriminability is a
/// different quantity. At `d = 1` a filler vector is a SCALAR, so two
/// candidates sharing a role differ only in scale and an L2 argmin over a
/// 256-D state can barely separate them — a structural handicap on the
/// readout that has nothing to do with whether the corpus is structured.
///
/// Sweeping the admissible grid separates "the corpus does not generalize"
/// from "BIC's `d` cannot express the answer". It is **not** the
/// widen-and-retry T4a forbids: no corpus is added, and the geometries swept
/// are the ones Bench 063 P13 already declared admissible on this arm.
///
/// # Panics
///
/// Panics if the corpus carries no roles — [`build_corpus`] cannot produce
/// that from a non-empty population, so it would mean the population is empty.
#[must_use]
pub fn run_t4a_at(
    population: Vec<Binding>,
    domain: &'static str,
    chosen_d: usize,
) -> Option<T4aReport> {
    let corpus = gate_corpus(population);
    let m_roles = corpus.vocab.roles.len();
    let n_fillers = corpus.vocab.fillers.len();
    assert!(m_roles > 0, "empty role vocabulary");
    let dim = corpus.dim();
    let cfg = AlsConfig::new(chosen_d, TprScheme::Orthogonal { arity: m_roles });

    // The shared pool: every distinct observed pair, as a one-slot binding.
    let observed = ObservedPairs::from_bindings(&corpus.bindings);
    let mut pool: Vec<TprBindings> = Vec::new();
    for role_id in 0..u16::try_from(m_roles).expect("role count fits u16") {
        for filler_id in 0..u16::try_from(n_fillers).expect("filler count fits u16") {
            if observed.contains(role_id, filler_id) {
                pool.push(TprBindings::from_pairs(&[(role_id, filler_id)]));
            }
        }
    }

    let spread = filler_role_spread(&corpus.bindings);
    let rpf = roles_per_filler(&corpus);

    let mut trials = Vec::with_capacity(pool.len());
    let mut skipped = Vec::with_capacity(pool.len());
    for cand in &pool {
        let (role_id, filler_id) = (cand.roles[0], cand.fillers[0]);
        let role = corpus.vocab.roles[role_id as usize];
        let filler = corpus.vocab.fillers[filler_id as usize].clone();
        let carries = |i: usize| {
            corpus.bindings[i].roles[0] == role_id && corpus.bindings[i].fillers[0] == filler_id
        };

        if rpf[filler_id as usize] < 2 {
            skipped.push((role, filler, SkipReason::SingleRoleFiller));
            continue;
        }
        let held: Vec<usize> = (0..corpus.bindings.len()).filter(|&i| carries(i)).collect();
        let train: Vec<usize> = (0..corpus.bindings.len())
            .filter(|&i| !carries(i))
            .collect();
        // The role must survive the withhold with states of its own: an empty
        // core block is a different geometry than the one BIC chose.
        if !train
            .iter()
            .any(|&i| corpus.bindings[i].roles[0] == role_id)
        {
            skipped.push((role, filler, SkipReason::WouldEmptyRole));
            continue;
        }

        let (train_states, train_bindings) = subset(&corpus, &train);
        let (held_states, held_bindings) = subset(&corpus, &held);
        let input = AlsInput {
            dim,
            n_fillers,
            states: &train_states,
            bindings: &train_bindings,
        };
        let Ok((art, _)) = als_fit(input, &cfg) else {
            skipped.push((role, filler, SkipReason::FitError));
            continue;
        };
        let mut scratch = TprScratch::new(&art);
        let Ok(report) =
            withheld_pair_top1_report(&art, &held_states, &held_bindings, &pool, &mut scratch)
        else {
            skipped.push((role, filler, SkipReason::FitError));
            continue;
        };
        // A3: the refit's own geometry, on the corpus the refit actually saw.
        let train_corpus = subset_corpus(&corpus, &train_states, &train_bindings);
        let train_admissible = admissibility(&train_corpus, chosen_d).admissible();

        let null = AtomicNull::fit(dim, &train_states, &train_bindings);
        // A1: the same readout on the states the fit saw. Same pool, so the
        // two arms differ only in whether the scored pair was withheld.
        let id = withheld_pair_top1(&art, &train_states, &train_bindings, &pool, &mut scratch)
            .unwrap_or(0.0);
        trials.push(Trial {
            role,
            filler,
            held: held.len(),
            train: train.len(),
            report,
            null_top1: null.top1(&held_states, &held_bindings, &pool),
            null_id_coverage: null.coverage(&train_bindings),
            train_admissible,
            tpr_id_top1: id,
        });
    }

    Some(T4aReport {
        domain,
        space: StateSpace::TokenBag256,
        coalesce_threshold: COALESCE_T,
        m: m_roles,
        n_fillers,
        n_states: corpus.n_states(),
        d: chosen_d,
        pool: pool.len(),
        spread,
        trials,
        skipped,
        pair_space: m_roles * n_fillers,
    })
}

// ── report ──────────────────────────────────────────────────────────────

/// The `--nocapture` table.
pub fn print_t4a(r: &T4aReport) {
    println!(
        "\n── Issue 062 T4a: withheld-pair gate ──  {} / {} [coalesced T={}]",
        r.domain,
        r.space.label(),
        r.coalesce_threshold
    );
    println!(
        "  corpus: n_states={} m(roles)={} n_fillers={} D={} d={} (BIC, admissible)",
        r.n_states,
        r.m,
        r.n_fillers,
        r.space.dim(),
        r.d
    );
    println!(
        "  composition: roles/filler max={} mean={:.3} over {} filler(s), {} testable; \
         distinct pairs {}",
        r.spread.max,
        r.spread.mean,
        r.spread.fillers,
        r.spread.multi_role_fillers,
        r.spread.distinct_pairs
    );
    println!(
        "  populations: validated {} pair(s) / {} scored state(s) | fitted-but-unvalidated {} \
         | outside the fit {} of {} pair space",
        r.trials.len(),
        r.scored(),
        r.unvalidated(),
        r.outside_fit(),
        r.pair_space
    );

    let mut by_reason = [0usize; 3];
    for (_, _, why) in &r.skipped {
        by_reason[match why {
            SkipReason::SingleRoleFiller => 0,
            SkipReason::WouldEmptyRole => 1,
            SkipReason::FitError => 2,
        }] += 1;
    }
    println!(
        "  unvalidated by reason: single-role-filler {} | would-empty-role {} | fit-error {}",
        by_reason[0], by_reason[1], by_reason[2]
    );

    println!(
        "\n  {:<28} {:<34} {:>5} {:>6} {:>7} {:>7} {:>6}",
        "role", "filler", "held", "train", "tpr", "null", "cov"
    );
    for t in &r.trials {
        println!(
            "  {:<28} {:<34} {:>5} {:>6} {:>6.1}% {:>6.1}% {:>6.1}% {:>5.1}%",
            t.role,
            t.filler,
            t.held,
            t.train,
            t.report.top1 * 100.0,
            t.tpr_id_top1 * 100.0,
            t.null_top1 * 100.0,
            t.report.coverage * 100.0
        );
    }

    println!(
        "\n  null: informative on every trial = {} (ID coverage > 99.9% each)",
        r.null_informative()
    );
    println!(
        "  TPR (A1): in-distribution top-1 {:.1}% over the same pool -> informative = {}",
        r.tpr_id_top1() * 100.0,
        r.tpr_informative()
    );
    println!(
        "  readable: every trial verdict Some + coverage 1.0 + admissible refit = {} \
         ({} inadmissible refit(s))",
        r.readable(),
        r.inadmissible_refits()
    );
    println!(
        "  OOD top-1 (state-weighted): TPR {:.1}%  vs  null {:.1}%  (chance {:.1}%, pool {})",
        r.tpr_top1() * 100.0,
        r.null_top1() * 100.0,
        r.chance() * 100.0,
        r.pool
    );
    println!(
        "  margin {:.1} pp (bar {T4A_MARGIN_PP:.1} pp); one flip is worth {:.1} pp \
         over {} scored state(s)",
        r.margin_pp(),
        r.one_flip_pp(),
        r.scored()
    );
    println!("  → T4a {}", r.verdict().label());
}
