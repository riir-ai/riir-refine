//! Self-evolving heal pipeline — composes drafter + pruner + memory + evolution.
//!
//! This is the `L_memory` + `L_evolve` layer from Research 001. It wraps the
//! existing L0-L2 pipeline (corpus + drafter + pruner) with:
//!
//! 1. **`LatentFixMemory`** — retrieves past fix trajectories for the current lint.
//! 2. **`LatentSkillEvolution`** — scores candidates using evolved per-lint directions.
//!
//! The pipeline is **additive**: when no memory exists, it degenerates to the
//! existing static-corpus path (drafter + pruner only). Evolution only biases
//! candidate selection when past outcomes have been recorded.
//!
//! ## Domain-agnostic methods (Plan 001 Phase 1)
//!
//! `heal` and `seed_from_corpus` are generic over `E: HealEntry` — they work
//! with any domain's corpus entry type. The memory + evolution layers remain
//! clippy-specific (they use `lint_name` vocabulary) until a second domain
//! needs the self-evolution layer.
//!
//! ## Flow
//!
//! ```text
//! heal_with_evolution(lint, code):
//!   1. Query memory for past trajectories (top-K).
//!   2. Draft a fix using the existing L1 drafter.
//!   3. Collect candidates: [drafter output] + [memory's past fixes].
//!   4. Score each candidate via LatentSkillEvolution (dot + sigmoid).
//!   5. Validate the top-scoring candidate with the L2 pruner.
//!   6. Return the result. Caller reports outcome → feeds back to memory + evolution.
//! ```

use crate::adaptive_drafter::AdaptiveDrafter;
use crate::domain::HealEntry;
use crate::drafter::TernaryDrafter;
use crate::embedder::{Direction, EMBED_DIM, embed_into};
use crate::evolve::LatentSkillEvolution;
use crate::memory::{FixTrajectory, LatentFixMemory, LatentQuery};
use crate::pruner::RustAnalyzerPruner;

/// The result of a heal attempt with the self-evolving pipeline.
#[derive(Debug, Clone)]
pub struct HealResult {
    /// The proposed fixed code (highest-scoring candidate that passed the pruner).
    pub fixed_code: String,
    /// Whether the fixed code passed the L2 syntax pruner.
    pub parses: bool,
    /// The score assigned by the evolution layer (0.0-1.0). Higher = the
    /// evolved direction favored this candidate.
    pub evolution_score: f32,
    /// Whether this fix came from memory (past trajectory) or the drafter.
    pub source: HealSource,
    /// Rule-level `context_notes` looked up from the corpus at heal time
    /// (Plan 001 post-promotion, 2026-08-13). `Some` signals the rule is
    /// context-dependent — not an unconditional default-config win. The
    /// pipeline surfaces the annotation on ALL paths (drafter / memory /
    /// Stage2) because the annotation is rule-level metadata, not fix-level.
    /// The pipeline does NOT skip or downgrade annotated rules; it surfaces
    /// the annotation so the consumer decides whether their context matches.
    pub context_notes: Option<String>,
}

/// Where the proposed fix came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealSource {
    /// The L1 ternary drafter (existing path).
    Drafter,
    /// A past fix trajectory retrieved from memory.
    Memory,
    /// The Stage 2 adaptive drafter — a verified fix served from the pool
    /// (Issue 010 fix direction). Highest-confidence source: the fix was
    /// validated by the domain validator during the tuning phase.
    Stage2Adaptive,
    /// The L4 generative fallback (Plan 336). Reached ONLY when the modelless
    /// L0-L3 path could not serve a parsing fix — see
    /// [`crate::l4_fallback`] for the four-invariant contract.
    #[cfg(feature = "l4_lora")]
    L4Lora,
}

/// The `evolution_score` reported for an L4 fix.
///
/// L4 answers carry no evolution evidence — the layer has never scored this
/// direction and the fix has no trajectory history. 0.5 is the pipeline's
/// existing "no evidence" value: an unseen rule scores `W_EVO * 0.5 +
/// W_RATE * 0.5 = 0.5` on the modelless path too, so L4 lands on the same
/// neutral point rather than inventing confidence it has not earned.
#[cfg(feature = "l4_lora")]
const L4_EVOLUTION_SCORE: f32 = 0.5;

/// Internal candidate for the heal pipeline's selection step.
struct Candidate {
    code: String,
    direction: Direction,
    source: HealSource,
    /// The trajectory's historical success rate (if from memory).
    /// Drafter candidates have no trajectory → neutral 0.5.
    success_rate: f32,
    /// Beta-Bernoulli counts backing `success_rate` (Plan 005 T9.3).
    ///
    /// `success_rate` alone cannot distinguish 1/1 from 100/100 — both are
    /// 1.0 — so a single lucky attempt outranks a long reliable history. The
    /// counts preserve the sample size that the rate throws away.
    counts: (u32, u32),
}

/// Weight on the evolution-score axis in candidate selection.
const W_EVO: f32 = 0.6;
/// Weight on the reliability axis in candidate selection.
const W_RATE: f32 = 0.4;

/// Conservatism of the Beta lower bound in [`SelectionMode::BetaPosterior`].
///
/// `best_belief_score` returns the ε-quantile of `Beta(1+S, 1+F)`; 0.05 is the
/// primitive's own documented default and matches the 95% convention used by
/// the conformal layer (Issue 023).
const BELIEF_EPSILON: f32 = 0.05;

/// How the reliability axis of candidate selection is scored (Plan 005 T9.3).
///
/// The evolution axis is unchanged in both modes — this replaces only the
/// *reliability* term, because discarding the direction signal wholesale
/// (as a bare `select_best_belief` call would) throws away the blend's
/// primary input and degenerates to first-index whenever every candidate is
/// unseen.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// `W_EVO·evolution + W_RATE·(S/(S+F))` — the raw success rate.
    ///
    /// Overconfident on small samples: 1 success out of 1 attempt scores a
    /// perfect 1.0, tying a candidate with 100 successes out of 100.
    ///
    /// Kept as the A/B baseline and an escape hatch for large-sample regimes
    /// (see [`BetaPosterior`](Self::BetaPosterior)'s crossover note).
    LinearBlend,
    /// `W_EVO·evolution + W_RATE·best_belief_score(S, F, ε)` — the ε-quantile
    /// of `Beta(1+S, 1+F)`, a conservative lower bound on true reliability.
    ///
    /// Sample size is respected: 1/1 scores 0.224 where the raw rate scores a
    /// perfect 1.000, so an 18/20 candidate (Beta 0.729) correctly outranks it.
    ///
    /// **Promoted to default by the T9.3 gate (Bench 028):** mean selection
    /// regret −8.4% over the 1–5-attempt regime, and −23.8% at a single
    /// attempt, across 20 000 randomized trials.
    ///
    /// **Documented crossover:** the advantage shrinks monotonically as
    /// evidence accumulates (−23.8% / −14.4% / −6.8% / −1.9% at max attempts
    /// 1 / 3 / 5 / 10) and *reverses* to +2.9% at 30 attempts, where the raw
    /// rate has converged and the lower bound's conservatism is pure cost.
    /// That reversal is a correctness check on the model, not a defect — and
    /// the healer's regime is the thin-evidence end, where per-(lint, fix)
    /// pairs are seen a handful of times. Switch to
    /// [`LinearBlend`](Self::LinearBlend) if that ever stops being true.
    #[default]
    BetaPosterior,
    /// `W_EVO·evolution + W_RATE·sigmoid(w_β − 1)` — the KL-budgeted
    /// Boltzmann tilt over the POOL's belief scores (Issue 026;
    /// TTT-Discover's within-batch max-seeking objective, β solved by
    /// bisection so `KL(q_β ‖ uniform) = ln 2`).
    ///
    /// Unlike the two history-only modes, the entropic score depends on the
    /// SHAPE of the current candidate pool: consistent-improvement pools
    /// tilt aggressively toward the best candidate (tiny belief gaps get
    /// amplified into decisive selection); outlier-dominated pools flatten,
    /// and the KL budget bounds the maximum weight regardless of gap
    /// magnitude (one lucky 1/1 candidate cannot own selection). The tilt
    /// consumes the belief scores as rewards, so the history axis still
    /// feeds selection — the paper itself stacks PUCT history + the
    /// entropic within-batch objective the same way.
    ///
    /// Degrades loudly to [`BetaPosterior`](Self::BetaPosterior) where no
    /// scored pool exists (pools of size ≤ 1, per-candidate-only call
    /// sites) — exactly one log line, never silent.
    ///
    /// **Measured NEGATIVE on the max metric (Bench 035, Issue 026 T3/T4):**
    /// discovery-within-budget 0.9091/0.9173 (sigmoid mapping) vs
    /// [`BetaPosterior`](Self::BetaPosterior) 0.9240/0.9379 — and ALL four
    /// reward-source × mapping variants lose to both [`LinearBlend`](Self::LinearBlend)
    /// and [`BetaPosterior`](Self::BetaPosterior) across both evo-noise
    /// regimes. The tilt amplifies thin-evidence pool-shape noise (belief
    /// rankings from 0-3 observations) and drowns the correlated evo axis;
    /// the paper's regime (dense reward batches) does not have starved
    /// pools. NOT promoted — [`BetaPosterior`](Self::BetaPosterior) stays
    /// the default. Kept opt-in as the reproducible negative-result
    /// artifact (the `gemma2_directions` precedent); the harness lives in
    /// `tests/t26_entropic_max_metric.rs`.
    Entropic,
    /// `(1−ρ)·evolution + ρ·best_belief_score(S, F, ε)` with **ρ fit from
    /// realized outcomes** instead of pinned at 0.4 (Issue 033;
    /// arXiv:2608.16739's TETHER baseline, see [`crate::selection_tether`]).
    ///
    /// The reliability axis is [`BetaPosterior`](Self::BetaPosterior)'s,
    /// unchanged — only the MIX becomes adaptive. That is the whole
    /// difference from [`Entropic`](Self::Entropic), which replaced the
    /// per-candidate score itself with a pool-shaped one and measured
    /// negative by amplifying thin-evidence noise (Bench 035). Here each
    /// candidate keeps the thin-evidence-correct Beta score and ρ is fit
    /// from a DENSER stream: one global scalar over all realized outcomes in
    /// a window, not a per-candidate quantity estimated from 0-3
    /// observations.
    ///
    /// ρ is fed by [`report_outcome`](HealPipeline::report_outcome) for the
    /// candidate this mode actually selected, applied one window late (the
    /// lag law — see the module docs), and starts at exactly `W_RATE`, so a
    /// pipeline that never reports an outcome is bit-identical to
    /// [`BetaPosterior`](Self::BetaPosterior).
    ///
    /// **Measured NEGATIVE (Bench 042, Issue 033 T4/T5) — NOT promoted.**
    /// Paired discovery under common random numbers: significantly WORSE at
    /// evo-noise 0.6 (0.9207 vs 0.9239, `McNemar` z = −13.86) and only
    /// +0.0007 better at 0.2 (z = +4.29); selection regret worse in all three
    /// attempt budgets. The mechanism is sound — the in-sample guarantee holds
    /// exactly, ρ drifts 0.058 ↔ 0.565 with axis informativeness, the stream
    /// is dense (0 degenerate windows over 18 351) — but ρ* minimizes
    /// PREDICTION error while selection needs a RANKING, and
    /// `best_belief_score` is deliberately biased low (that bias is why T9.3
    /// promoted it). So the fit correctly downweights the reliability axis for
    /// the loss it was given and incorrectly for the metric that pays.
    ///
    /// The fixed-ρ sweep in the same bench closes the question: discovery is
    /// FLAT across ρ ∈ [0, 0.6] (spread 0.0067, SE ≈ 0.0018) and the pinned
    /// 0.4 is the sweep max in the harsh regime. The hand-pinned constant was
    /// never a live lever — "never swept" was a fair criticism with a null
    /// answer.
    ///
    /// Kept opt-in as the reproducible negative-result artifact (the Issue 026
    /// / `gemma2_directions` precedent); harnesses in
    /// `tests/t33_tether_selection_ab.rs` + `tests/t33_tether_g4_alloc.rs`.
    TetherBlend,
}

/// One-time loud degradation notice when `Entropic` is scored per-candidate
/// (no candidate pool at the call site) — mirrors the `self_evolve`
/// oracle-degradation pattern: exactly one line, never silent (Issue 026 T2).
fn warn_entropic_degraded_once() {
    use std::sync::Once;
    static DEGRADE_NOTICE: Once = Once::new();
    DEGRADE_NOTICE.call_once(|| {
        eprintln!(
            "riir-clippy: SelectionMode::Entropic scored without a candidate pool — \
             degrading to BetaPosterior (per-candidate history score)"
        );
    });
}

/// Score the reliability axis for a candidate under `mode`.
///
/// Per-candidate form. `Entropic` has no per-candidate form (its score is
/// pool-shaped) — it degrades loudly to the Beta belief score here; the
/// pool-bearing path in [`HealPipeline::heal`] computes the true tilt.
#[inline]
fn reliability_score(mode: SelectionMode, counts: (u32, u32), rate: f32) -> f32 {
    match mode {
        SelectionMode::LinearBlend => rate,
        // `TetherBlend` shares this axis verbatim — it changes the MIX weight,
        // not the per-candidate score (Issue 033).
        SelectionMode::BetaPosterior | SelectionMode::TetherBlend => {
            katgpt_core::best_belief_score(counts.0, counts.1, BELIEF_EPSILON)
        }
        SelectionMode::Entropic => {
            warn_entropic_degraded_once();
            katgpt_core::best_belief_score(counts.0, counts.1, BELIEF_EPSILON)
        }
    }
}

/// The selection facts needed to attribute a realized outcome back to the
/// [`SelectionMode::TetherBlend`] fit (Issue 033).
///
/// `p1` / `p2` are captured **at selection time**: the ρ fit regresses the
/// return on the endpoint predictions that produced it, so post-outcome values
/// (a reliability score already updated with this very outcome) would leak the
/// return into its own baseline.
///
/// The `String`s are reused buffers (`clear` + `push_str`), so staging a pick
/// is alloc-free after warm-up — G4.
#[derive(Debug, Default)]
struct TetherPending {
    rule_id: String,
    fixed_code: String,
    p1: f32,
    p2: f32,
    /// Whether the buffers hold a live pick (a pick is consumed on report).
    staged: bool,
}

/// The self-evolving heal pipeline.
///
/// Composes the frozen L0-L2 pipeline (drafter + pruner) with the latent
/// memory + evolution layers. The frozen layers are NEVER mutated — the
/// evolution layer is a reader-side overlay.
pub struct HealPipeline {
    /// How the reliability axis is scored (Plan 005 T9.3).
    selection_mode: SelectionMode,
    /// The frozen L1 ternary drafter (existing — not modified by evolution).
    drafter: TernaryDrafter,
    /// The frozen L2 syntax pruner (existing — not modified by evolution).
    pruner: RustAnalyzerPruner,
    /// The latent fix-trajectory memory (grows with use).
    memory: LatentFixMemory,
    /// The latent skill evolution layer (reader-side direction overlay).
    evolution: LatentSkillEvolution,
    /// The Stage 2 adaptive drafter — pool of verified fixes keyed by input
    /// (Issue 010 fix direction). Empty by default; populated by
    /// [`tune_against_fixture`](Self::tune_against_fixture) off the hot path.
    adaptive: AdaptiveDrafter,
    /// Reusable scratch buffer for retrieval index sort (alloc-free after warm-up).
    retrieve_scratch: Vec<usize>,
    /// Reusable scratch buffer for retrieval results (alloc-free after warm-up).
    retrieve_out: Vec<crate::memory::ScoredTrajectory>,
    /// Reusable scratch: per-candidate tilt rewards (belief scores) for the
    /// `Entropic` selection mode (alloc-free after warm-up, Issue 026).
    tilt_rewards: Vec<f32>,
    /// Reusable scratch: tilted advantages for the `Entropic` selection
    /// mode (alloc-free after warm-up, Issue 026).
    tilt_advantages: Vec<f32>,
    /// The TETHER blend-weight fit for
    /// [`SelectionMode::TetherBlend`](crate::SelectionMode) (Issue 033).
    ///
    /// Inert in every other mode: it is only read by the `TetherBlend`
    /// scoring path and only written by [`Self::report_outcome`] after a
    /// `TetherBlend` pick, so the other modes' arithmetic is untouched.
    tether: crate::selection_tether::TetherFit,
    /// The last `TetherBlend` pick awaiting its realized outcome.
    tether_pending: TetherPending,
    /// The L4 generative fallback (Plan 336 T4.4). `None` by default — the
    /// pipeline is then byte-identical to the L0-L3 path. Consulted only on a
    /// modelless miss; see [`crate::l4_fallback`] for the contract.
    #[cfg(feature = "l4_lora")]
    l4: Option<Box<dyn crate::l4_fallback::L4Fallback>>,
    /// The input-scope gate (Bench 669 consumer adoption, Plan 016). `None` by
    /// default — the pipeline is then byte-identical to the pre-adoption
    /// path. When installed, [`HealPipeline::heal`] first scores the INPUT
    /// under the contrastive table: out-of-scope inputs (`D > θ`) decline
    /// immediately (never served, L4 never consulted — declining is a CORRECT
    /// answer off-distribution); in-scope inputs proceed unchanged with the
    /// epistemic haircut applied to `evolution_score` (exactly `1.0` for
    /// strongly in-scope docs, by f32 sigmoid saturation — bit-identical).
    #[cfg(feature = "contrastive_scope")]
    scope: Option<crate::scope_model::ScopeModel>,
    /// Reusable token-id scratch for the scope check (alloc-free after
    /// warm-up — the `mem::take` dance preserves capacity across the split
    /// borrow between `self.scope` and the scratch buffer).
    #[cfg(feature = "contrastive_scope")]
    scope_scratch: Vec<u32>,
    /// Issue 048 T3 — the GP acquisition tie-break (lexicographic, AFTER the
    /// `BetaPosterior` combined score; never overrides payoff). `None` by
    /// default → selection is bit-identical to the pre-048 path. When
    /// installed via [`Self::with_gp_tiebreak`], an EXACT combined-score tie
    /// between two candidates is broken by posterior variance of their
    /// 8-D direction embeddings (higher σ² first — prefer the input the
    /// observed set explains least), and [`Self::report_outcome`] observes
    /// each reported fix direction so σ² collapses where evidence exists.
    gp: Option<crate::gp_acquisition::GpAcquisition>,
}

impl HealPipeline {
    /// Construct a new pipeline with the given drafter + pruner + empty memory.
    /// Set how the reliability axis is scored — the Plan 005 T9.3 A/B knob.
    /// Defaults to [`SelectionMode::BetaPosterior`].
    #[must_use]
    pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    #[must_use]
    pub fn new(drafter: TernaryDrafter, pruner: RustAnalyzerPruner) -> Self {
        Self {
            selection_mode: SelectionMode::default(),
            drafter,
            pruner,
            memory: LatentFixMemory::default(),
            evolution: LatentSkillEvolution::default(),
            adaptive: AdaptiveDrafter::new(),
            retrieve_scratch: Vec::new(),
            retrieve_out: Vec::new(),
            tilt_rewards: Vec::new(),
            tilt_advantages: Vec::new(),
            tether: crate::selection_tether::TetherFit::new(),
            tether_pending: TetherPending::default(),
            #[cfg(feature = "l4_lora")]
            l4: None,
            #[cfg(feature = "contrastive_scope")]
            scope: None,
            #[cfg(feature = "contrastive_scope")]
            scope_scratch: Vec::new(),
            gp: None,
        }
    }

    /// Issue 048 T3 — install the GP acquisition tie-break, seeded with
    /// prior observation directions (e.g. trajectory directions from
    /// memory or the durable store).
    ///
    /// Lexicographic ONLY: the tie-break fires on exact `f32` equality of
    /// the combined score (unseen candidates sharing belief 0.32 and a tied
    /// evolution score are the common real-pool shape). Default pipeline
    /// (not installed) is bit-identical — pinned by test.
    #[must_use]
    pub fn with_gp_tiebreak(
        mut self,
        observed: impl IntoIterator<Item = Direction>,
    ) -> Self {
        let mut gp = crate::gp_acquisition::GpAcquisition::new();
        for dir in observed {
            gp.observe(&dir);
        }
        self.gp = Some(gp);
        self
    }

    /// Whether the GP acquisition tie-break is installed (Issue 048 T3).
    #[must_use]
    pub fn has_gp_tiebreak(&self) -> bool {
        self.gp.is_some()
    }

    /// Install the input-scope gate (Bench 669 consumer adoption, Plan 016).
    ///
    /// Typically built via [`ScopeModel::build_for_corpus`] over the same
    /// corpus the pipeline serves. Without this the pipeline behaves exactly
    /// as before the adoption — the gate is opt-in at BOTH the feature level
    /// and the builder level.
    #[cfg(feature = "contrastive_scope")]
    #[must_use]
    pub fn with_scope_model(mut self, model: crate::scope_model::ScopeModel) -> Self {
        self.scope = Some(model);
        self
    }

    /// Whether an input-scope model is installed (for inspection / benches).
    #[cfg(feature = "contrastive_scope")]
    #[must_use]
    pub fn has_scope_model(&self) -> bool {
        self.scope.is_some()
    }

    /// Install the L4 generative fallback (Plan 336 T4.4).
    ///
    /// Without this the pipeline never leaves the modelless path. With it, L4
    /// is consulted **only** when L0-L3 fail to produce a parsing fix, and its
    /// answer is kept only if it parses — see [`crate::l4_fallback`].
    #[cfg(feature = "l4_lora")]
    #[must_use]
    pub fn with_l4_fallback(mut self, l4: Box<dyn crate::l4_fallback::L4Fallback>) -> Self {
        self.l4 = Some(l4);
        self
    }

    /// Whether an L4 fallback is installed (for inspection / benchmarks).
    #[cfg(feature = "l4_lora")]
    #[must_use]
    pub fn has_l4_fallback(&self) -> bool {
        self.l4.is_some()
    }

    /// Step 4 of [`HealPipeline::heal`]: score each candidate and pick the
    /// best.
    ///
    /// Combined score = `W_EVO`·evolution + `W_RATE`·reliability. `W_EVO =
    /// 0.6` (the evolved direction is the primary signal); `W_RATE = 0.4`
    /// (historical reliability is a strong secondary signal). For unseen
    /// rules (evolution returns 0.5), reliability dominates — the first fix
    /// attempt picks the most reliable past fix.
    ///
    /// Entropic (Issue 026): the reliability axis is scored from the POOL's
    /// shape instead of per-candidate history alone — the KL-budgeted
    /// Boltzmann tilt over the pool's belief scores (max-seeking; adaptive β
    /// per pool shape). Pools of size ≤ 1 have no shape and fall through to
    /// the per-candidate path, which degrades loudly to `BetaPosterior` there.
    fn select_best_candidate(&mut self, rule_id: &str, candidates: &[Candidate]) -> (usize, f32) {
        let entropic_pool =
            matches!(self.selection_mode, SelectionMode::Entropic) && candidates.len() > 1;
        if entropic_pool {
            self.tilt_rewards.clear();
            for cand in candidates {
                self.tilt_rewards.push(katgpt_core::best_belief_score(
                    cand.counts.0,
                    cand.counts.1,
                    BELIEF_EPSILON,
                ));
            }
            crate::selection_entropic::tilt_advantages_into(
                &self.tilt_rewards,
                crate::selection_entropic::KL_BUDGET_LN2,
                &mut self.tilt_advantages,
            );
        }

        // ρ is read ONCE for the whole pool: the weight that scores a pool
        // must not shift between its candidates, and the fit is published
        // only at window boundaries anyway (the Issue 033 lag law).
        let tether = matches!(self.selection_mode, SelectionMode::TetherBlend);
        let rho = self.tether.rho();

        let mut best = None::<(usize, f32)>;
        // The winner's endpoint pair, carried out for the TETHER observation.
        let mut best_axes = (0.0f32, 0.0f32);
        for (i, cand) in candidates.iter().enumerate() {
            let evo_score = self.evolution.score_candidate(rule_id, &cand.direction);
            let reliability = if entropic_pool {
                katgpt_core::sigmoid(self.tilt_advantages[i])
            } else {
                reliability_score(self.selection_mode, cand.counts, cand.success_rate)
            };
            let combined = if tether {
                (1.0 - rho) * evo_score + rho * reliability
            } else {
                W_EVO * evo_score + W_RATE * reliability
            };
            // Issue 048 T3 — GP acquisition tie-break: lexicographic AFTER
            // the combined score. Fires ONLY on exact f32 equality (the
            // unseen-candidate shape: belief(0,0) both sides + a tied
            // evolution score); prefers the higher posterior-variance
            // direction. Never overrides payoff — a strictly greater
            // combined score always wins regardless of σ². The strict `==`
            // is the DESIGN: an epsilon compare would let near-ties trigger
            // the σ² override, which is exactly what "never overriding
            // payoff" forbids.
            #[allow(clippy::float_cmp)]
            let takes = match best {
                None => true,
                Some((best_idx, best_score)) => {
                    if combined > best_score {
                        true
                    } else if combined == best_score {
                        match &self.gp {
                            Some(gp) => {
                                gp.sigma_sq(&cand.direction)
                                    > gp.sigma_sq(&candidates[best_idx].direction)
                            }
                            None => false,
                        }
                    } else {
                        false
                    }
                }
            };
            if takes {
                best = Some((i, combined));
                best_axes = (evo_score, reliability);
            }
        }
        let (best_idx, best_score) = best.expect("candidates is non-empty, so best exists");
        if tether {
            self.stage_tether_pick(rule_id, &candidates[best_idx].code, best_axes);
        }
        (best_idx, best_score)
    }

    /// Stage the `TetherBlend` pick's `(rule_id, fixed_code, p1, p2)` so a later
    /// [`report_outcome`](Self::report_outcome) can attribute the realized
    /// return to it (Issue 033 T2).
    ///
    /// Attribution is exact-match on `(rule_id, fixed_code)`: the heal path can
    /// still decline the pick or replace it with an L4 answer, and an outcome
    /// for a fix this mode did not choose must not enter the fit.
    fn stage_tether_pick(&mut self, rule_id: &str, fixed_code: &str, axes: (f32, f32)) {
        let p = &mut self.tether_pending;
        p.rule_id.clear();
        p.rule_id.push_str(rule_id);
        p.fixed_code.clear();
        p.fixed_code.push_str(fixed_code);
        p.p1 = axes.0;
        p.p2 = axes.1;
        p.staged = true;
    }

    /// The TETHER blend telemetry — applied ρ, last closed window's raw fit,
    /// window counts (Issue 033 T3).
    ///
    /// The ρ *trajectory* is the mechanism-of-action signal: a ρ pinned at its
    /// [`DEFAULT_RHO`](crate::selection_tether::DEFAULT_RHO) start means the
    /// fit never saw a window with two disagreeing axes, i.e. the mode is
    /// inert on this stream — a finding, not a pass.
    #[must_use]
    pub fn tether_stats(&self) -> crate::selection_tether::TetherStats {
        self.tether.stats()
    }

    /// Close the TETHER window early — for a process ending mid-window
    /// (`cargo heal` is short-lived) or a benchmark that wants the tail folded
    /// in. A no-op-safe flush: a degenerate tail holds the previous ρ.
    pub fn flush_tether_window(&mut self) {
        self.tether.close_window();
    }

    /// Score the input under the installed scope model and fold the verdict
    /// into a haircut multiplier. `None` ⇒ DECLINE the heal entirely.
    ///
    /// The 2D complement of the Issue 030 relevance gate: rule coverage ×
    /// INPUT scope. An out-of-scope input (`D > θ`: non-Rust code, config
    /// files, prose) paired with an in-corpus rule id would otherwise be
    /// SERVED confidently — the drafter serves the corpus template, it parses,
    /// the rule is covered. A Rust fix for a Python input is a wrong answer
    /// served with a straight face, and on the L4 path the same garbage burns
    /// a ~48 s GPU call. Declining is the CORRECT answer off-distribution.
    ///
    /// No-regression: without a model installed this returns `Some(1.0)`;
    /// with one, strongly in-scope docs saturate f32 sigmoid to exactly `1.0`
    /// so the haircut is bit-identical (pinned by test).
    #[cfg(feature = "contrastive_scope")]
    fn scope_haircut_for(&mut self, buggy_code: &str) -> Option<f32> {
        // Split-borrow workaround: the model is behind `&self.scope` while
        // the scratch buffer is `&mut self.scope_scratch` — take the scratch
        // out, use, put it back (capacity preserved — G4 alloc-free warm-up).
        let mut scratch = std::mem::take(&mut self.scope_scratch);
        let verdict = self
            .scope
            .as_ref()
            .map(|m| m.verdict_str(buggy_code, 1.0, &mut scratch));
        self.scope_scratch = scratch;
        match verdict {
            Some(v) if v.declined => None,
            Some(v) => Some(v.haircut),
            None => Some(1.0),
        }
    }

    /// Consult the L4 fallback for a fix the modelless path could not serve.
    ///
    /// Returns `Some` only when a fallback is installed, it produced a fix,
    /// AND that fix passes the L2 pruner — invariant 3 (never degrade a
    /// modelless answer) is enforced here, at the single point where an L4
    /// result can enter the pipeline.
    #[cfg(feature = "l4_lora")]
    fn consult_l4(
        &mut self,
        rule_id: &str,
        buggy_code: &str,
        rule_context_notes: Option<&str>,
    ) -> Option<HealResult> {
        let fix = self.l4.as_mut()?.generate_fix(rule_id, buggy_code)?;
        // The pruner is the same L2 gate every other source passes through.
        // A non-parsing L4 answer is discarded, not returned with parses=false:
        // the caller already has (or has not) a modelless answer, and replacing
        // it with an equally-broken generated one buys nothing for ~48 s.
        let validation = self.pruner.validate(&fix, rule_id);
        if !validation.parses {
            return None;
        }
        Some(HealResult {
            fixed_code: fix,
            parses: true,
            evolution_score: L4_EVOLUTION_SCORE,
            source: HealSource::L4Lora,
            context_notes: rule_context_notes.map(str::to_string),
        })
    }

    /// Resolve a modelless miss: offer it to L4 first, then decide what to do
    /// when L4 is absent or declines.
    ///
    /// `relevant` separates the two miss kinds (Issue 030):
    /// - **relevant but non-parsing** — hand the answer back anyway, exactly as
    ///   the pipeline did before L4 existed, so the worst case is unchanged.
    /// - **irrelevant** — DECLINE. Serving unrelated code that merely parses
    ///   looks like a fix, and makes the caller's verifier pay to discover it is
    ///   not; `None` lets the caller escalate honestly.
    ///
    /// The non-parsing answer is built lazily so the decline path allocates
    /// nothing (`heal_is_alloc_free_after_warmup` covers the hot path).
    fn resolve_miss(
        &mut self,
        rule_id: &str,
        buggy_code: &str,
        rule_context_notes: Option<&str>,
        candidate: (&str, f32, HealSource),
        relevant: bool,
    ) -> Option<HealResult> {
        if let Some(l4) = self.consult_l4(rule_id, buggy_code, rule_context_notes) {
            return Some(l4);
        }
        if !relevant {
            return None;
        }
        let (code, evolution_score, source) = candidate;
        Some(HealResult {
            fixed_code: code.to_string(),
            parses: false,
            evolution_score,
            source,
            context_notes: rule_context_notes.map(str::to_string),
        })
    }

    /// No-op when the `l4_lora` feature is off — the miss path compiles to the
    /// same `None` it returned before the stage existed.
    #[cfg(not(feature = "l4_lora"))]
    #[inline]
    #[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn consult_l4(
        &mut self,
        _rule_id: &str,
        _buggy_code: &str,
        _rule_context_notes: Option<&str>,
    ) -> Option<HealResult> {
        None
    }

    /// Borrow the Stage 2 adaptive drafter (for inspection / testing).
    #[must_use]
    pub fn adaptive(&self) -> &AdaptiveDrafter {
        &self.adaptive
    }

    /// Whether the Stage 2 adaptive drafter has any verified fixes in the pool.
    #[must_use]
    pub fn has_stage2_entries(&self) -> bool {
        !self.adaptive.is_empty()
    }

    /// Check the internal `retrieve_out` buffer capacity (for G4 alloc-free
    /// steady-state tests). After warm-up, this capacity must not grow.
    #[must_use]
    pub fn retrieve_out_capacity(&self) -> usize {
        self.retrieve_out.capacity()
    }

    /// Borrow the memory (for seeding / inspection).
    #[must_use]
    pub fn memory(&self) -> &LatentFixMemory {
        &self.memory
    }

    /// Mutably borrow the memory (for seeding).
    pub fn memory_mut(&mut self) -> &mut LatentFixMemory {
        &mut self.memory
    }

    /// Borrow the evolution layer (for inspection / testing).
    #[must_use]
    pub fn evolution(&self) -> &LatentSkillEvolution {
        &self.evolution
    }

    /// Seed the memory with a known-good fix trajectory.
    ///
    /// Used to bootstrap the memory with corpus entries (so the first heal
    /// attempt already has retrieval candidates).
    pub fn seed_trajectory(&mut self, traj: FixTrajectory) {
        self.memory.insert(traj);
    }

    /// Seed the memory from a corpus (each entry becomes a trajectory with
    /// neutral success/fail stats).
    ///
    /// Generic over `E: HealEntry` — works with any domain's entry type.
    pub fn seed_from_corpus<E: HealEntry>(&mut self, entries: &[E]) {
        for entry in entries {
            let traj = FixTrajectory::new(
                entry.rule_id().to_string(),
                entry.buggy_code().to_string(),
                entry.fixed_code().to_string(),
            );
            self.memory.insert(traj);
        }
    }

    /// Attempt to heal a rule using the self-evolving pipeline.
    ///
    /// Generic over `E: HealEntry` — works with any domain's corpus entry type.
    ///
    /// **Stage 2 routing (Issue 010):** if the adaptive drafter has a verified
    /// fix for `(rule_id, buggy_code)`, return it immediately. Otherwise
    /// fall back to the Stage 1 drafter + memory + evolution path. The Stage 2
    /// pool is populated OFF the hot path by [`tune_against_fixture`].
    ///
    /// Returns `None` if no candidate could be produced (empty corpus + empty
    /// memory + no Stage 2 hit).
    ///
    /// The caller is responsible for verifying the fix actually resolves the
    /// rule (e.g., via `report_outcome` after checking with real validation).
    /// For POC / testing, the caller can simulate outcomes.
    ///
    /// [`tune_against_fixture`]: Self::tune_against_fixture
    ///
    /// # Panics
    ///
    /// Panics if the internal candidate-index bookkeeping is inconsistent
    /// (should not happen — guarded by the candidate-collection invariant).
    #[must_use]
    pub fn heal<E: HealEntry>(
        &mut self,
        rule_id: &str,
        buggy_code: &str,
        corpus: &[E],
    ) -> Option<HealResult> {
        // ── Input-scope gate (Bench 669 consumer adoption, Plan 016) ──────
        // `None` ⇒ out-of-scope input (`D > θ`): decline the whole heal —
        // never serve, never consult L4. See `scope_haircut_for` for the
        // rationale. Compiles to nothing without the feature.
        #[cfg(feature = "contrastive_scope")]
        let scope_haircut: f32 = self.scope_haircut_for(buggy_code)?;

        // Look up rule-level context_notes from the corpus (the source of truth
        // for rule metadata). This ensures the annotation is surfaced on ALL
        // paths (drafter / memory / Stage2) — the annotation describes the
        // RULE, not a specific fix, so it's the same regardless of which path
        // serves the fix.
        // One scan serves both the annotation lookup and the relevance gate
        // below (Issue 030) — `rule_in_corpus` is the cheap, exact signal for
        // "has this pipeline ever been given ground truth for this rule".
        let corpus_entry = corpus.iter().find(|e| e.rule_id() == rule_id);
        let rule_in_corpus = corpus_entry.is_some();
        let rule_context_notes: Option<&str> = corpus_entry.and_then(HealEntry::context_notes);

        // Stage 2 routing: if the adaptive drafter has a verified entry, return it.
        // This is the ~µs serve path (hash lookup) after the tuning loop populated
        // the pool. The Stage 1 path below is the always-on fallback.
        if let Some(verified) = self.adaptive.lookup(rule_id, buggy_code) {
            // Stage 2 fixes were already validated by the domain validator during
            // tuning. We still run the L2 pruner (cheap) to set the `parses`
            // field honestly.
            let validation = self.pruner.validate(&verified, rule_id);
            return Some(HealResult {
                fixed_code: verified,
                parses: validation.parses,
                // Verified = highest confidence, scaled by the input-scope
                // haircut (exactly 1.0 for in-scope inputs — saturation).
                #[cfg(feature = "contrastive_scope")]
                evolution_score: scope_haircut,
                #[cfg(not(feature = "contrastive_scope"))]
                evolution_score: 1.0, // verified = highest confidence
                source: HealSource::Stage2Adaptive,
                context_notes: rule_context_notes.map(str::to_string),
            });
        }

        // Step 1: retrieve past trajectories for this rule + code context.
        let query = LatentQuery::new(rule_id.to_string(), buggy_code);
        self.memory.retrieve_into(&query, 5, &mut self.retrieve_scratch, &mut self.retrieve_out);

        // Step 2: draft a fix using the existing L1 drafter.
        let drafted = self.drafter.draft_fix(rule_id, buggy_code, corpus);

        // Step 3: collect candidates (drafter output + memory trajectories).
        // Each candidate carries its code + direction + optional success_rate.
        // The success_rate (from the memory trajectory) lets us prefer fixes
        // that have historically worked, not just fixes that align with the
        // evolved direction.
        let mut candidates: Vec<Candidate> = Vec::new();

        // Candidate from drafter (if it produced one).
        if let Some(draft) = drafted {
            let mut dir = [0.0f32; EMBED_DIM];
            embed_into(rule_id, &draft.code, &mut dir);
            // Look up the ACTUAL trajectory stats for this code — if it has
            // been tried before and failed, its success_rate should reflect
            // that (not a hardcoded 0.5). This lets fail feedback bias
            // selection away from repeatedly-failed drafter picks.
            let success_rate = self.memory.success_rate_for(rule_id, &draft.code);
            let counts = self.memory.counts_for(rule_id, &draft.code);
            candidates.push(Candidate {
                code: draft.code,
                direction: dir,
                source: HealSource::Drafter,
                success_rate,
                counts,
            });
        }

        // Candidates from memory's past trajectories.
        for scored in &self.retrieve_out {
            if let Some(traj) = self.memory.get(scored.idx) {
                // Skip if identical to an already-added candidate (dedup).
                if candidates.iter().any(|c| c.code == traj.fixed_code) {
                    continue;
                }
                candidates.push(Candidate {
                    code: traj.fixed_code.clone(),
                    direction: traj.direction,
                    source: HealSource::Memory,
                    success_rate: traj.success_rate(),
                    counts: (traj.success_count, traj.fail_count),
                });
            }
        }

        if candidates.is_empty() {
            // Modelless miss #1: nothing to serve at all. This is the case L4
            // exists for — an input the corpus, drafter and memory have never
            // seen. Returns None when no fallback is installed.
            return self.consult_l4(rule_id, buggy_code, rule_context_notes);
        }

        // Step 4: score each candidate — see `select_best_candidate` for the
        // weighting rationale + the Entropic pool tilt (Issue 026).
        let (best_idx, best_score) = self.select_best_candidate(rule_id, &candidates);
        let cand = &candidates[best_idx];

        // Step 5: validate with the L2 pruner.
        let validation = self.pruner.validate(&cand.code, rule_id);

        // ── Relevance gate (Issue 030) ────────────────────────────────────
        // `parses` is a SYNTACTIC test, and using it alone as the serve/decline
        // decision breaks silently off-distribution. When the corpus has no
        // entry for `rule_id`, the candidate pool collapses to memory's nearest
        // trajectory — valid Rust belonging to some OTHER rule. It parses, so
        // the old code served it and suppressed L4 entirely. Measured before
        // this gate: miss rate 0/29, including 0/9 on real lints with no corpus
        // entry, where e.g. `ptr_arg` was answered with
        // `fn f() -> String { "done".to_string() }` (Bench 037).
        //
        // Provenance is the cheap, exact signal: a Memory candidate for a rule
        // the corpus has never covered has no claim to relevance. The other
        // sources are unaffected by construction — Stage 2 hits return earlier
        // (verifier-certified during tuning, so the Issue 010 path that reached
        // 10/10 held-out cannot regress), and Drafter candidates are derived
        // from the corpus slice for this very rule.
        let relevant = rule_in_corpus || cand.source != HealSource::Memory;

        // Modelless miss #2: the best candidate does not parse, OR it is not
        // relevant to the rule. L4 gets a chance to beat it — but only a PARSING
        // L4 answer replaces it (invariant 3, enforced inside `consult_l4`).
        //
        // Note the ordering: a modelless fix that both parses AND is relevant
        // never reaches the fallback (invariant 1).
        if !validation.parses || !relevant {
            return self.resolve_miss(
                rule_id,
                buggy_code,
                rule_context_notes,
                (&cand.code, best_score, cand.source),
                relevant,
            );
        }

        Some(HealResult {
            fixed_code: cand.code.clone(),
            parses: validation.parses,
            #[cfg(feature = "contrastive_scope")]
            evolution_score: best_score * scope_haircut,
            #[cfg(not(feature = "contrastive_scope"))]
            evolution_score: best_score,
            source: cand.source,
            context_notes: rule_context_notes.map(str::to_string),
        })
    }

    /// Report the outcome of a heal attempt (success or fail) and feed back
    /// into the memory + evolution layers.
    ///
    /// This is the "`L_evolve`" step — it records the trajectory (if new),
    /// updates the trajectory's success/fail stats, and applies the EMA
    /// direction update.
    ///
    /// `verified` (Issue 021) carries the outcome's verification context:
    /// `true` only when the verdict came from a full validator (a real
    /// clippy re-run proving the lint resolved, the domain validator, the
    /// tuning-loop verifier). A verified success certifies the trajectory's
    /// [`EvidenceTier`](crate::memory::EvidenceTier); a syn-parse pass or a
    /// simulated outcome is `verified: false`.
    pub fn report_outcome(
        &mut self,
        rule_id: &str,
        buggy_code: &str,
        fixed_code: &str,
        success: bool,
        verified: bool,
    ) {
        // 1. Compute the fix's direction.
        let mut fix_dir = [0.0f32; EMBED_DIM];
        embed_into(rule_id, fixed_code, &mut fix_dir);

        // 2. Insert the trajectory if it doesn't exist yet.
        let traj = FixTrajectory::new(
            rule_id.to_string(),
            buggy_code.to_string(),
            fixed_code.to_string(),
        );
        self.memory.insert(traj);

        // 3. Update the trajectory's success/fail stats + evidential tier.
        self.memory.record_outcome(rule_id, fixed_code, success, verified);

        // 4. EMA direction update (the actual "evolution").
        self.evolution.record_outcome(rule_id, &fix_dir, success);

        // 4b. Issue 048 T3 — observe the fix direction into the GP
        //     acquisition model (when installed): σ² collapses where the
        //     observed set already explains the input, so future tie-breaks
        //     steer toward genuinely-unseen directions. Outcome-blind by
        //     design — the VARIANCE model is over inputs, not labels.
        if let Some(gp) = &mut self.gp {
            gp.observe(&fix_dir);
        }

        // 5. TETHER blend-weight fit (Issue 033) — only for an outcome of a
        //    fix THIS mode selected, and only where the outcome is
        //    trustworthy. `verified` is the Issue-018 oracle split at this
        //    seam: an unverified verdict (syn-parse pass, simulated outcome)
        //    would fit ρ against a guess, exactly what the split forbids for
        //    the EMA direction update.
        if matches!(self.selection_mode, SelectionMode::TetherBlend)
            && self.tether_pending.staged
            && verified
            && self.tether_pending.rule_id == rule_id
            && self.tether_pending.fixed_code == fixed_code
        {
            let (p1, p2) = (self.tether_pending.p1, self.tether_pending.p2);
            self.tether_pending.staged = false;
            self.tether.observe(f32::from(u8::from(success)), p1, p2);
        }
    }

    /// **Stage 2 tuning loop (Issue 010 fix direction).**
    ///
    /// For each `(rule_id, buggy_code)` pair, populate the adaptive drafter's
    /// pool with a verified fix. The flow:
    ///
    /// 1. Stage 1 `draft_fix` produces a candidate. If it passes the identity
    ///    guard + domain validation → join the pool.
    /// 2. If Stage 1 declines (identity guard fail) or fails validation →
    ///    ask the per-lint AST proposer (`domains::clippy_lints::proposers`)
    ///    for a candidate. If the proposer's output passes validation → join
    ///    the pool.
    ///
    /// **Re-running the domain validator during tuning is fine** — the ~130 ms
    /// cost (clippy) is 4 orders of magnitude above the Stage 1 hot path (~µs),
    /// but the tuning loop is a batch process that builds the Stage 2 pool. The
    /// hot path stays at Stage 1's ~µs latency; it upgrades to Stage 2's ~µs
    /// serve only when a verified entry exists.
    ///
    /// This method is OFF the hot path — call it once at startup (or as a
    /// periodic background refresh), not per user request.
    ///
    /// # Arguments
    ///
    /// - `cases`: iterator of `(rule_id, buggy_code)` pairs to tune against.
    /// - `verifier`: the `ClippyVerifier` used to verify proposed fixes.
    ///
    /// # Returns
    ///
    /// A [`TuneReport`] summarizing the tuning pass — how many cases were
    /// `stage1_hits`, `stage1_verified`, `stage2_proposed`, `stage2_verified`,
    /// or failed. The pool's `len()` after the call is the cumulative count of
    /// verified fixes admitted.
    #[cfg(feature = "clippy_verify")]
    pub fn tune_against_fixture<'a, I>(
        &mut self,
        cases: I,
        verifier: &crate::domains::clippy_lints::verify::ClippyVerifier,
    ) -> TuneReport
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        use crate::domains::clippy_lints::ClippyFixEntry;

        let mut report = TuneReport::default();
        for (rule_id, buggy_code) in cases {
            report.total += 1;
            if self.adaptive.lookup(rule_id, buggy_code).is_some() {
                report.stage1_hits += 1;
                continue;
            }

            // Stage 1 path — pass empty candidates so the identity guard fires
            // + Stage 2 proposer takes over. (The drafter returns None for
            // empty candidates, so stage1_ok is always false here.)
            let empty: [ClippyFixEntry; 0] = [];
            let stage1_ok = if let Some(draft) = self.drafter.draft_fix(rule_id, buggy_code, &empty) {
                let v = verifier.verify(&draft.code, rule_id);
                if v.ran && v.lint_resolved {
                    // Issue 021: a verifier-passed fix is a certification —
                    // record a Certified trajectory in memory (the tier's
                    // admission rule IS the verified-pool admission).
                    self.memory.record_certified(rule_id, buggy_code, &draft.code);
                    self.adaptive.insert(rule_id, buggy_code, draft.code);
                    report.stage1_verified += 1;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if stage1_ok {
                continue;
            }

            // Stage 2 proposer path.
            if let Some(proposed) =
                crate::domains::clippy_lints::proposers::propose_fix(rule_id, buggy_code)
            {
                report.stage2_proposed += 1;
                let v = verifier.verify(&proposed, rule_id);
                if v.ran && v.lint_resolved {
                    // Issue 021: same certification on the proposer path.
                    self.memory.record_certified(rule_id, buggy_code, &proposed);
                    self.adaptive.insert(rule_id, buggy_code, proposed);
                    report.stage2_verified += 1;
                }
            }
        }
        report
    }

    /// Insert a verified fix directly into the Stage 2 adaptive pool.
    ///
    /// Used by callers that have their own verification path (e.g. they ran
    /// the domain validator manually). The tuning loop above does this
    /// automatically; this method is for direct seeding (e.g. for tests).
    pub fn insert_verified(&self, rule_id: &str, buggy_code: &str, verified_fix: String) {
        self.adaptive.insert(rule_id, buggy_code, verified_fix);
    }
}

/// Report from a Stage 2 tuning pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TuneReport {
    /// Total number of `(rule, buggy)` cases the tuning loop processed.
    pub total: u32,
    /// Cases where the adaptive pool already had a hit (skipped).
    pub stage1_hits: u32,
    /// Cases where Stage 1 (the L1 drafter) produced a verified fix.
    pub stage1_verified: u32,
    /// Cases where the Stage 2 proposer produced a candidate.
    pub stage2_proposed: u32,
    /// Cases where the Stage 2 proposer's candidate was verified.
    pub stage2_verified: u32,
}

impl TuneReport {
    /// Cases that ended with a verified fix in the pool.
    #[must_use]
    pub fn verified_count(&self) -> u32 {
        self.stage1_verified + self.stage2_verified
    }

    /// Cases that ended without a verified fix (Stage 1 + Stage 2 both failed).
    #[must_use]
    pub fn failed_count(&self) -> u32 {
        self.total - self.stage1_hits - self.verified_count()
    }
}

#[cfg(all(test, feature = "clippy_lints"))]
#[allow(clippy::float_cmp)] // tests check bit-identical determinism
mod tests {
    use super::*;
    use crate::domains::clippy_lints::ClippyFixEntry;

    fn mock_corpus() -> Vec<ClippyFixEntry> {
        vec![
            ClippyFixEntry {
                lint_name: "needless_return".to_string(),
                lint_group: "style".to_string(),
                buggy_code: "fn foo() -> i32 { return 42; }".to_string(),
                fixed_code: "fn foo() -> i32 { 42 }".to_string(),
            },
            ClippyFixEntry {
                lint_name: "redundant_closure".to_string(),
                lint_group: "style".to_string(),
                buggy_code: "|x| f(x)".to_string(),
                fixed_code: "f".to_string(),
            },
        ]
    }

    fn make_pipeline() -> HealPipeline {
        HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new())
    }

    #[test]
    fn heal_returns_none_on_empty_corpus_and_memory() {
        let mut p = make_pipeline();
        let empty: [ClippyFixEntry; 0] = [];
        let result = p.heal("needless_return", "return 42;", &empty);
        assert!(result.is_none(), "empty corpus + empty memory → no candidates");
    }

    #[test]
    fn heal_works_with_corpus_only_no_memory() {
        // Degeneration: no memory → just drafter + pruner (the existing path).
        let mut p = make_pipeline();
        let corpus = mock_corpus();
        let result = p.heal("needless_return", "return 42;", &corpus);
        assert!(result.is_some(), "corpus-only must produce a candidate");
        let r = result.unwrap();
        assert_eq!(r.source, HealSource::Drafter, "no memory → must come from drafter");
        // T9.3: an unseen candidate now scores 0.6·0.5 + 0.4·beta(0,0) =
        // 0.32, not 0.5. The Beta lower bound reports "no evidence" as a
        // conservative 0.05 rather than a neutral 0.5 — that is the point of
        // the change, so this pins the new value deliberately.
        assert_eq!(r.evolution_score, 0.320_000_02, "unseen lint → no-evidence score");
    }

    #[test]
    fn heal_uses_memory_after_seeding() {
        let mut p = make_pipeline();
        let corpus = mock_corpus();
        p.seed_from_corpus(&corpus);

        let result = p.heal("needless_return", "fn foo() -> i32 { return 42; }", &corpus);
        assert!(result.is_some());
        let r = result.unwrap();
        // With memory seeded, the fix could come from either source — both are valid.
        assert!(!r.fixed_code.is_empty());
    }

    #[test]
    fn report_outcome_evolves_direction() {
        let mut p = make_pipeline();
        let corpus = mock_corpus();

        // Before any outcome: evolution score for this lint is neutral (0.5).
        let r1 = p.heal("needless_return", "return 42;", &corpus).unwrap();
        assert_eq!(r1.evolution_score, 0.320_000_02); // T9.3 no-evidence score

        // Report a success with the correct fix.
        p.report_outcome("needless_return", "return 42;", "fn foo() -> i32 { 42 }", true, false);

        // After a success: the evolved direction should favor the correct fix.
        let r2 = p.heal("needless_return", "return 42;", &corpus).unwrap();
        assert!(
            r2.evolution_score > 0.5,
            "after a success, evolution score should be > 0.5, got {}",
            r2.evolution_score
        );
    }

    #[test]
    fn heal_is_deterministic_without_evolution() {
        // G1: without any report_outcome calls, heal is deterministic.
        let corpus = mock_corpus();
        let mut p1 = make_pipeline();
        let mut p2 = make_pipeline();
        p1.seed_from_corpus(&corpus);
        p2.seed_from_corpus(&corpus);

        let r1 = p1.heal("needless_return", "return 42;", &corpus).unwrap();
        let r2 = p2.heal("needless_return", "return 42;", &corpus).unwrap();

        assert_eq!(r1.fixed_code, r2.fixed_code, "G1: same input → same output");
        assert_eq!(r1.evolution_score.to_bits(), r2.evolution_score.to_bits());
    }

    #[test]
    fn heal_is_alloc_free_after_warmup() {
        // G4 smoke: repeated heal calls should not grow scratch buffer capacity.
        let corpus = mock_corpus();
        let mut p = make_pipeline();
        p.seed_from_corpus(&corpus);

        // Warm up.
        let _ = p.heal("needless_return", "return 42;", &corpus);
        let cap_retrieve_out = p.retrieve_out.capacity();

        // Steady state — capacity must not grow.
        for _ in 0..50 {
            let _ = p.heal("needless_return", "return 42;", &corpus);
        }
        assert_eq!(p.retrieve_out.capacity(), cap_retrieve_out, "retrieve_out grew");
    }

    #[test]
    fn drafter_validation_result_propagates() {
        // The pruner's parse check should propagate to HealResult.parses.
        let mut p = make_pipeline();
        let corpus = mock_corpus();
        let result = p.heal("needless_return", "return 42;", &corpus).unwrap();
        // The corpus's fixed_code ("fn foo() -> i32 { 42 }") should parse.
        assert!(result.parses, "corpus fix must parse");
    }

    // ── context_notes pipeline propagation (Plan 001 post-promotion, 2026-08-13) ──
    // The annotation is rule-level metadata looked up from the corpus at heal
    // time. It surfaces on ALL paths (drafter / memory / Stage2), not just the
    // drafter path — the annotation describes the RULE, not a specific fix.

    /// Test entry that overrides `context_notes()` — simulates an annotated
    /// rule so we can verify the pipeline surfaces the annotation through
    /// `HealResult` on all paths.
    #[derive(Clone)]
    struct PipelineAnnotatedEntry {
        rule_id: String,
        buggy: String,
        fixed: String,
        notes: Option<String>,
    }

    impl HealEntry for PipelineAnnotatedEntry {
        fn rule_id(&self) -> &str {
            &self.rule_id
        }
        fn buggy_code(&self) -> &str {
            &self.buggy
        }
        fn fixed_code(&self) -> &str {
            &self.fixed
        }
        fn context_notes(&self) -> Option<&str> {
            self.notes.as_deref()
        }
    }

    #[test]
    fn heal_propagates_context_notes_via_drafter_path() {
        // The drafter path (source=Drafter) must surface context_notes from
        // the corpus lookup. The annotation is rule-level metadata — the same
        // regardless of which path serves the fix.
        let entries = vec![PipelineAnnotatedEntry {
            rule_id: "precompute-table".to_string(),
            buggy: "let pop = v.iter().filter(|x| **x).count();".to_string(),
            fixed: "let pop = v.iter().filter(|x| **x).count(); // table".to_string(),
            notes: Some("ParetoDominated 0.20-0.40x (Bench 013)".to_string()),
        }];
        let mut p = make_pipeline();
        let result = p.heal("precompute-table", "let pop = v.iter().filter(|x| **x).count();", &entries).unwrap();
        assert_eq!(result.source, HealSource::Drafter, "no memory seeded → must come from drafter");
        assert!(result.context_notes.is_some(), "drafter path must surface context_notes");
        assert_eq!(result.context_notes.as_deref(), Some("ParetoDominated 0.20-0.40x (Bench 013)"));
    }

    #[test]
    fn heal_context_notes_none_for_default_entries() {
        // ClippyFixEntry uses the default context_notes() -> None. The
        // pipeline must propagate None for these entries.
        let mut p = make_pipeline();
        let corpus = mock_corpus();
        let result = p.heal("needless_return", "return 42;", &corpus).unwrap();
        assert!(result.context_notes.is_none(), "default clippy entries should have no context_notes");
    }

    #[test]
    fn heal_propagates_context_notes_via_stage2_path() {
        // The Stage2 adaptive drafter (cached verified fixes) must also surface
        // context_notes — the annotation is rule-level metadata looked up from
        // the corpus, not fix-level data stored in the cache.
        let entries = vec![PipelineAnnotatedEntry {
            rule_id: "precompute-table".to_string(),
            buggy: "let pop = v.iter().filter(|x| **x).count();".to_string(),
            fixed: "let pop = v.iter().filter(|x| **x).count(); // table".to_string(),
            notes: Some("ParetoDominated 0.20-0.40x (Bench 013)".to_string()),
        }];
        let mut p = make_pipeline();
        // Seed the Stage2 pool with a verified fix for this rule.
        p.insert_verified(
            "precompute-table",
            "let pop = v.iter().filter(|x| **x).count();",
            "let pop = v.iter().filter(|x| **x).count(); // table".to_string(),
        );
        let result = p.heal("precompute-table", "let pop = v.iter().filter(|x| **x).count();", &entries).unwrap();
        assert_eq!(result.source, HealSource::Stage2Adaptive, "must come from Stage2 cache");
        assert!(result.context_notes.is_some(), "Stage2 path must surface context_notes from corpus");
        assert_eq!(result.context_notes.as_deref(), Some("ParetoDominated 0.20-0.40x (Bench 013)"));
    }

    #[test]
    fn heal_propagates_context_notes_via_memory_path() {
        // When the memory path wins (a past trajectory beats the drafter
        // candidate), the annotation must still surface from the corpus
        // lookup. The annotation is NOT stored in FixTrajectory.
        let entries = vec![PipelineAnnotatedEntry {
            rule_id: "annotated-rule".to_string(),
            buggy: "let buggy = 1;".to_string(),
            fixed: "let fixed = 1;".to_string(),
            notes: Some("Mixed 2/4 Faster (Bench 013)".to_string()),
        }];
        let mut p = make_pipeline();
        // Seed memory with a trajectory so the memory path is available.
        p.seed_from_corpus(&entries);
        // Report a success on the memory fix so its success_rate > drafter's.
        // Simulated outcome (no validator ran) — verified: false.
        p.report_outcome("annotated-rule", "let buggy = 1;", "let fixed = 1;", true, false);

        let result = p.heal("annotated-rule", "let buggy = 1;", &entries);
        assert!(result.is_some(), "must produce a result");
        let r = result.unwrap();
        // The annotation must surface regardless of source (Drafter or Memory).
        assert!(r.context_notes.is_some(), "memory path must surface context_notes from corpus");
        assert_eq!(r.context_notes.as_deref(), Some("Mixed 2/4 Faster (Bench 013)"));
    }

    // ── Issue 048 T3 — GP acquisition tie-break ─────────────────────────

    fn embed_helper(rule: &str, code: &str) -> Direction {
        let mut d = [0.0f32; EMBED_DIM];
        embed_into(rule, code, &mut d);
        d
    }

    #[test]
    fn gp_tiebreak_default_pipeline_has_none_installed() {
        let p = make_pipeline();
        assert!(!p.has_gp_tiebreak(), "default: no GP installed → bit-identical pre-048");
    }

    #[test]
    fn gp_tiebreak_breaks_exact_ties_toward_higher_sigma() {
        // Two memory candidates with IDENTICAL counts (both unseen → same
        // belief) and directions that tie on evolution score only if the
        // evolution layer scores them equally — the honest way to force an
        // exact combined tie is two identical-stat trajectories whose
        // directions differ. The drafter candidate is absent (rule in
        // corpus, drafter may still fire — instead force the memory-only
        // shape via a rule NOT in the corpus: relevance gate would decline
        // Memory candidates then... so instead use the corpus rule and
        // check the SELECTED candidate differs per GP seeding.
        //
        // Simplest deterministic shape: same rule, two seeded trajectories
        // with identical stats; GP observed set contains trajectory A's
        // direction (σ²_A small) but NOT B's (σ²_B ≈ ‖b‖²) → tie must pick B.
        let mut p = make_pipeline();
        p.seed_trajectory(FixTrajectory::new(
            "needless_return".to_string(),
            "fn a() { return 1; }".to_string(),
            "fn a() { 1 }".to_string(),
        ));
        p.seed_trajectory(FixTrajectory::new(
            "needless_return".to_string(),
            "fn b() { return 2; }".to_string(),
            "fn b() { 2 }".to_string(),
        ));
        // Identical stats (0,0) → identical belief; evolution scores by
        // direction — to force the tie we ALSO need identical evo. The
        // evolution layer starts empty (score 0.5 neutral for unseen
        // rules); with no recorded outcomes both candidates tie exactly:
        // W_EVO·0.5' + W_RATE·belief(0,0) each — wait, evolution scores by
        // direction even unseen. Force BOTH equal via identical counts and
        // check evo equality empirically below instead of assuming.
        let dir_a = embed_helper("needless_return", "fn a() { 1 }");
        let dir_b = embed_helper("needless_return", "fn b() { 2 }");
        let evo_a = p.evolution.score_candidate("needless_return", &dir_a);
        let evo_b = p.evolution.score_candidate("needless_return", &dir_b);
        // On a fresh pipeline the evolution layer's neutral score makes
        // these equal; if not, this test's premise (exact tie) fails loudly.
        assert_eq!(
            evo_a, evo_b,
            "test premise: fresh evolution layer must score both directions equally"
        );

        // GP seeded with A's AND the drafter's direction → both collapse,
        // B stays high → the 3-way tie must pick B. (The drafter candidate
        // — the corpus's own fix — joins the tie pool: same belief(0,0),
        // same neutral evolution score, pushed first.)
        let corpus = mock_corpus();
        let dir_draft = embed_helper("needless_return", corpus[0].fixed_code.as_str());
        let mut p_gp = make_pipeline();
        p_gp.seed_trajectory(FixTrajectory::new(
            "needless_return".to_string(),
            "fn a() { return 1; }".to_string(),
            "fn a() { 1 }".to_string(),
        ));
        p_gp.seed_trajectory(FixTrajectory::new(
            "needless_return".to_string(),
            "fn b() { return 2; }".to_string(),
            "fn b() { 2 }".to_string(),
        ));
        let mut p_gp = p_gp.with_gp_tiebreak([dir_a, dir_draft]);
        let buggy = "fn a() { return 1; }";
        let r0 = p.heal("needless_return", buggy, &corpus).unwrap();
        let r1 = p_gp.heal("needless_return", buggy, &corpus).unwrap();
        // Without GP the tie resolves to first-index (the drafter candidate,
        // pushed first); with GP it must flip to B (the only unobserved
        // direction — highest σ²).
        assert_eq!(r0.fixed_code, corpus[0].fixed_code, "no GP → first-index tie order");
        assert_eq!(r1.fixed_code, "fn b() { 2 }", "GP tie-break must pick the unobserved direction");
    }

    #[test]
    fn gp_tiebreak_never_overrides_payoff() {
        // A strictly-better-scoring candidate must win regardless of σ².
        // Shape: trajectory A with strong success stats (belief high),
        // trajectory B unseen (belief 0.32) + GP seeded with A (σ²_A small,
        // σ²_B large). Combined(A) > Combined(B) → A wins with AND without GP.
        let mut p = make_pipeline();
        let traj_a = FixTrajectory::new(
            "needless_return".to_string(),
            "fn a() { return 1; }".to_string(),
            "fn a() { 1 }".to_string(),
        );
        p.seed_trajectory(traj_a);
        p.seed_trajectory(FixTrajectory::new(
            "needless_return".to_string(),
            "fn b() { return 2; }".to_string(),
            "fn b() { 2 }".to_string(),
        ));
        // 10 successes on A.
        for _ in 0..10 {
            p.report_outcome("needless_return", "fn a() { return 1; }", "fn a() { 1 }", true, false);
        }
        let dir_a = embed_helper("needless_return", "fn a() { 1 }");
        // Rebuild with GP seeded on A (the winner) — payoff must still pick A.
        let mut p2 = make_pipeline();
        p2.seed_trajectory(FixTrajectory::new(
            "needless_return".to_string(),
            "fn a() { return 1; }".to_string(),
            "fn a() { 1 }".to_string(),
        ));
        p2.seed_trajectory(FixTrajectory::new(
            "needless_return".to_string(),
            "fn b() { return 2; }".to_string(),
            "fn b() { 2 }".to_string(),
        ));
        for _ in 0..10 {
            p2.report_outcome("needless_return", "fn a() { return 1; }", "fn a() { 1 }", true, false);
        }
        let mut p2 = p2.with_gp_tiebreak([dir_a]);
        let corpus = mock_corpus();
        let r = p2.heal("needless_return", "fn a() { return 1; }", &corpus).unwrap();
        assert_eq!(r.fixed_code, "fn a() { 1 }", "strictly better score wins regardless of σ²");
    }

    #[test]
    fn gp_tiebreak_observations_grow_on_outcome() {
        let mut p = make_pipeline().with_gp_tiebreak([]);
        p.report_outcome("needless_return", "fn a() { return 1; }", "fn a() { 1 }", true, false);
        // Internal growth is observable via behavior: σ² for the reported
        // direction collapses. We can't read the private field from here —
        // pin via the standalone module instead (gp_acquisition's own tests
        // cover collapse); here just pin that outcomes don't panic + the
        // pipeline still heals.
        let corpus = mock_corpus();
        assert!(p.heal("needless_return", "fn a() { return 1; }", &corpus).is_some());
    }
}
