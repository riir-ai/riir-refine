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
}

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
        SelectionMode::BetaPosterior => {
            katgpt_core::best_belief_score(counts.0, counts.1, BELIEF_EPSILON)
        }
        SelectionMode::Entropic => {
            warn_entropic_degraded_once();
            katgpt_core::best_belief_score(counts.0, counts.1, BELIEF_EPSILON)
        }
    }
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
        }
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
        // Look up rule-level context_notes from the corpus (the source of truth
        // for rule metadata). This ensures the annotation is surfaced on ALL
        // paths (drafter / memory / Stage2) — the annotation describes the
        // RULE, not a specific fix, so it's the same regardless of which path
        // serves the fix.
        let rule_context_notes: Option<&str> = corpus
            .iter()
            .find(|e| e.rule_id() == rule_id)
            .and_then(HealEntry::context_notes);

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
            return None;
        }

        // Step 4: score each candidate.
        // Combined score = W_EVO * evolution_score + W_RATE * success_rate.
        // W_EVO = 0.6 (the evolved direction is the primary signal).
        // W_RATE = 0.4 (historical reliability is a strong secondary signal).
        // For unseen rules (evolution returns 0.5), success_rate dominates —
        // this means the first fix attempt picks the most reliable past fix.
        //
        // Entropic (Issue 026): the reliability axis is scored from the
        // POOL's shape instead of per-candidate history alone — the
        // KL-budgeted Boltzmann tilt over the pool's belief scores
        // (max-seeking; adaptive β per pool shape). Pools of size ≤ 1 have
        // no shape and fall through to the per-candidate path, which
        // degrades loudly to BetaPosterior there.
        let entropic_pool =
            matches!(self.selection_mode, SelectionMode::Entropic) && candidates.len() > 1;
        if entropic_pool {
            self.tilt_rewards.clear();
            for cand in &candidates {
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

        let mut best = None::<(usize, f32)>;
        for (i, cand) in candidates.iter().enumerate() {
            let evo_score = self.evolution.score_candidate(rule_id, &cand.direction);
            let reliability = if entropic_pool {
                katgpt_core::sigmoid(self.tilt_advantages[i])
            } else {
                reliability_score(self.selection_mode, cand.counts, cand.success_rate)
            };
            let combined = W_EVO * evo_score + W_RATE * reliability;
            if best.is_none_or(|(_, best_score)| combined > best_score) {
                best = Some((i, combined));
            }
        }

        let (best_idx, best_score) = best.expect("candidates is non-empty, so best exists");
        let cand = &candidates[best_idx];

        // Step 5: validate with the L2 pruner.
        let validation = self.pruner.validate(&cand.code, rule_id);

        Some(HealResult {
            fixed_code: cand.code.clone(),
            parses: validation.parses,
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
}
