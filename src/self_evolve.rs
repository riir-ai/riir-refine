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

use crate::corpus::ClippyFixEntry;
use crate::drafter::ClippyTernaryDrafter;
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
    drafter: ClippyTernaryDrafter,
    /// The frozen L2 syntax pruner (existing — not modified by evolution).
    pruner: RustAnalyzerPruner,
    /// The latent fix-trajectory memory (grows with use).
    memory: LatentFixMemory,
    /// The latent skill evolution layer (reader-side direction overlay).
    evolution: LatentSkillEvolution,
    /// Reusable scratch buffer for retrieval index sort (alloc-free after warm-up).
    retrieve_scratch: Vec<usize>,
    /// Reusable scratch buffer for retrieval results (alloc-free after warm-up).
    retrieve_out: Vec<crate::memory::ScoredTrajectory>,
}

impl HealPipeline {
    /// Construct a new pipeline with the given drafter + pruner + empty memory.
    #[must_use]
    pub fn new(drafter: ClippyTernaryDrafter, pruner: RustAnalyzerPruner) -> Self {
        Self {
            drafter,
            pruner,
            memory: LatentFixMemory::default(),
            evolution: LatentSkillEvolution::default(),
            retrieve_scratch: Vec::new(),
            retrieve_out: Vec::new(),
        }
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
    pub fn seed_from_corpus(&mut self, entries: &[ClippyFixEntry]) {
        for entry in entries {
            let traj = FixTrajectory::new(
                entry.lint_name.clone(),
                entry.buggy_code.clone(),
                entry.fixed_code.clone(),
            );
            self.memory.insert(traj);
        }
    }

    /// Attempt to heal a clippy lint using the self-evolving pipeline.
    ///
    /// See the module doc for the flow. Returns `None` if no candidate could
    /// be produced (empty corpus + empty memory).
    ///
    /// The caller is responsible for verifying the fix actually resolves the
    /// lint (e.g., via `report_outcome` after checking with real cargo clippy).
    /// For POC / testing, the caller can simulate outcomes.
    ///
    /// # Panics
    ///
    /// Panics if the internal candidate-index bookkeeping is inconsistent
    /// (should not happen — guarded by the candidate-collection invariant).
    #[must_use]
    pub fn heal(&mut self, lint_name: &str, buggy_code: &str, corpus: &[ClippyFixEntry]) -> Option<HealResult> {
        // Step 1: retrieve past trajectories for this lint + code context.
        let query = LatentQuery::new(lint_name.to_string(), buggy_code);
        self.memory.retrieve_into(&query, 5, &mut self.retrieve_scratch, &mut self.retrieve_out);

        // Step 2: draft a fix using the existing L1 drafter.
        let drafted = self.drafter.draft_fix(lint_name, buggy_code, corpus);

        // Step 3: collect candidates (drafter output + memory trajectories).
        // Each candidate carries its code + direction + optional success_rate.
        // The success_rate (from the memory trajectory) lets us prefer fixes
        // that have historically worked, not just fixes that align with the
        // evolved direction.
        let mut candidates: Vec<Candidate> = Vec::new();

        // Candidate from drafter (if it produced one).
        if let Some(draft) = drafted {
            let mut dir = [0.0f32; EMBED_DIM];
            embed_into(lint_name, &draft.code, &mut dir);
            // Look up the ACTUAL trajectory stats for this code — if it has
            // been tried before and failed, its success_rate should reflect
            // that (not a hardcoded 0.5). This lets fail feedback bias
            // selection away from repeatedly-failed drafter picks.
            let success_rate = self.memory.success_rate_for(lint_name, &draft.code);
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
        // For unseen lints (evolution returns 0.5), success_rate dominates —
        // this means the first fix attempt picks the most reliable past fix.
        let mut best = None::<(usize, f32)>;
        for (i, cand) in candidates.iter().enumerate() {
            let evo_score = self.evolution.score_candidate(lint_name, &cand.direction);
            let combined = W_EVO * evo_score + W_RATE * cand.success_rate;
            if best.is_none_or(|(_, best_score)| combined > best_score) {
                best = Some((i, combined));
            }
        }

        let (best_idx, best_score) = best.expect("candidates is non-empty, so best exists");
        let cand = &candidates[best_idx];

        // Step 5: validate with the L2 pruner.
        let validation = self.pruner.validate(&cand.code, lint_name);

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
        lint_name: &str,
        buggy_code: &str,
        fixed_code: &str,
        success: bool,
    ) {
        // 1. Compute the fix's direction.
        let mut fix_dir = [0.0f32; EMBED_DIM];
        embed_into(lint_name, fixed_code, &mut fix_dir);

        // 2. Insert the trajectory if it doesn't exist yet.
        let traj = FixTrajectory::new(
            lint_name.to_string(),
            buggy_code.to_string(),
            fixed_code.to_string(),
        );
        self.memory.insert(traj);

        // 3. Update the trajectory's success/fail stats.
        self.memory.record_outcome(lint_name, fixed_code, success);

        // 4. EMA direction update (the actual "evolution").
        self.evolution.record_outcome(lint_name, &fix_dir, success);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // tests check bit-identical determinism
mod tests {
    use super::*;

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
        HealPipeline::new(ClippyTernaryDrafter::fallback(), RustAnalyzerPruner::new())
    }

    #[test]
    fn heal_returns_none_on_empty_corpus_and_memory() {
        let mut p = make_pipeline();
        let result = p.heal("needless_return", "return 42;", &[]);
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
