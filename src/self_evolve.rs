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
}

/// Weight on the evolution-score axis in candidate selection.
const W_EVO: f32 = 0.6;
/// Weight on the success-rate axis in candidate selection.
const W_RATE: f32 = 0.4;

/// The self-evolving heal pipeline.
///
/// Composes the frozen L0-L2 pipeline (drafter + pruner) with the latent
/// memory + evolution layers. The frozen layers are NEVER mutated — the
/// evolution layer is a reader-side overlay.
pub struct HealPipeline {
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
}

impl HealPipeline {
    /// Construct a new pipeline with the given drafter + pruner + empty memory.
    #[must_use]
    pub fn new(drafter: TernaryDrafter, pruner: RustAnalyzerPruner) -> Self {
        Self {
            drafter,
            pruner,
            memory: LatentFixMemory::default(),
            evolution: LatentSkillEvolution::default(),
            adaptive: AdaptiveDrafter::new(),
            retrieve_scratch: Vec::new(),
            retrieve_out: Vec::new(),
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
            candidates.push(Candidate {
                code: draft.code,
                direction: dir,
                source: HealSource::Drafter,
                success_rate,
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
        let mut best = None::<(usize, f32)>;
        for (i, cand) in candidates.iter().enumerate() {
            let evo_score = self.evolution.score_candidate(rule_id, &cand.direction);
            let combined = W_EVO * evo_score + W_RATE * cand.success_rate;
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
        })
    }

    /// Report the outcome of a heal attempt (success or fail) and feed back
    /// into the memory + evolution layers.
    ///
    /// This is the "`L_evolve`" step — it records the trajectory (if new),
    /// updates the trajectory's success/fail stats, and applies the EMA
    /// direction update.
    pub fn report_outcome(
        &mut self,
        rule_id: &str,
        buggy_code: &str,
        fixed_code: &str,
        success: bool,
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

        // 3. Update the trajectory's success/fail stats.
        self.memory.record_outcome(rule_id, fixed_code, success);

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
        assert_eq!(r.evolution_score, 0.5, "unseen lint → neutral evolution score");
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
        assert_eq!(r1.evolution_score, 0.5);

        // Report a success with the correct fix.
        p.report_outcome("needless_return", "return 42;", "fn foo() -> i32 { 42 }", true);

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
}
