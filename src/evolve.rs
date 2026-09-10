//! Latent skill evolution — EMA direction-vector update on fix outcomes.
//!
//! Borrows the modelless EMA direction-update pattern from
//! `CommittedFieldBlend` (katgpt-rs Plan 321) + `PersonalityWeightedComposition`
//! (Plan 297). The update is pure vector arithmetic — no loss function, no
//! gradient, no backprop.
//!
//! ## The evolution rule
//!
//! Each lint has an evolved "preference direction" `[f32; 8]` that represents
//! what kind of fix tends to work for it. On each fix outcome:
//!
//! ```text
//! On success: direction += α · (fix_direction - direction)   // move toward
//! On fail:    direction -= β · (fix_direction - direction)   // move away
//! ```
//!
//! where `α` (success learning rate) > `β` (fail unlearning rate), because
//! successes are more informative than failures (a success confirms the fix
//! works; a failure only says "this fix didn't work this time", which could
//! be due to many causes).
//!
//! ## Freeze/thaw invariant
//!
//! The base drafter weights are NEVER mutated. This evolution layer is a
//! reader-side overlay: the evolved direction is used to *score* candidates,
//! not to modify the frozen ternary weights. This preserves the modelless
//! mandate (no weight mutation) + the freeze/thaw invariant (base snapshot
//! is immutable; the evolution is a versioned overlay).
//!
//! ## Alloc-free hot path
//!
//! `score_candidate_into` writes into a caller-provided buffer. The
//! `HashMap<String, Direction>` allocates per-lint on first insert, but the
//! read path (lookup + arithmetic) is alloc-free.

use std::collections::HashMap;

use crate::embedder::{Direction, EMBED_DIM};

/// Configuration for the EMA evolution rule.
#[derive(Debug, Clone, Copy)]
pub struct EvolveConfig {
    /// Success learning rate (how much to move toward a successful fix's
    /// direction). Default 0.1 — same as `CommittedFieldBlend`'s α.
    pub alpha: f32,
    /// Fail unlearning rate (how much to move away from a failed fix's
    /// direction). Default 0.05 — smaller than α because failures are less
    /// informative.
    pub beta: f32,
    /// Temperature for the sigmoid scoring. Lower = sharper preference.
    /// Default 1.0.
    pub temperature: f32,
}

impl Default for EvolveConfig {
    fn default() -> Self {
        Self {
            alpha: 0.1,
            beta: 0.05,
            temperature: 1.0,
        }
    }
}

/// The latent skill evolution layer — maintains per-lint evolved directions.
///
/// Each entry maps `lint_name → Direction`. The direction starts as the
/// BLAKE3-embedded identity of the lint's "typical good fix" and evolves via
/// EMA as outcomes are reported.
pub struct LatentSkillEvolution {
    /// Per-lint evolved direction overlays.
    directions: HashMap<String, Direction>,
    /// Evolution hyperparameters.
    config: EvolveConfig,
}

impl Default for LatentSkillEvolution {
    fn default() -> Self {
        Self::new(EvolveConfig::default())
    }
}

/// One EMA outcome step over a caller-owned direction — the pure
/// arithmetic core of [`LatentSkillEvolution::record_outcome`], shared
/// with the durable per-rule learn rows ([`crate::traj_store`], Issue 037
/// E3) so the persisted state evolves under the SAME rule the in-process
/// layer uses. Returns the L2 magnitude of the applied (pre-clamp) delta —
/// the "direction deltas ≠ 0" evidence the E3 gate asks for.
///
/// Success moves `dir` toward `fix_dir` by `α`; fail moves it away by `β`.
/// The clamp keeps components in `[-1, 1]` (the embedder's range — EMA
/// accumulation could otherwise drift past it).
pub fn ema_step(
    dir: &mut Direction,
    fix_dir: &Direction,
    success: bool,
    config: &EvolveConfig,
) -> f32 {
    let rate = if success { config.alpha } else { -config.beta };
    let mut sq = 0.0f32;
    for i in 0..EMBED_DIM {
        let applied = rate * (fix_dir[i] - dir[i]);
        dir[i] = (dir[i] + applied).clamp(-1.0, 1.0);
        sq += applied * applied;
    }
    sq.sqrt()
}

impl LatentSkillEvolution {
    /// Construct with the given config.
    #[must_use]
    pub fn new(config: EvolveConfig) -> Self {
        Self {
            directions: HashMap::new(),
            config,
        }
    }

    /// Pre-allocate capacity for `n` lints.
    #[must_use]
    pub fn with_capacity(n: usize) -> Self {
        Self {
            directions: HashMap::with_capacity(n),
            config: EvolveConfig::default(),
        }
    }

    /// Number of lints with evolved directions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.directions.len()
    }

    /// Whether any directions have evolved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.directions.is_empty()
    }

    /// Get the evolved direction for a lint, if any.
    ///
    /// Returns `None` if no outcome has been recorded for this lint yet
    /// (the caller should use a default direction in that case).
    #[must_use]
    pub fn get(&self, lint_name: &str) -> Option<&Direction> {
        self.directions.get(lint_name)
    }

    /// Record a fix outcome and update the evolved direction.
    ///
    /// - `lint_name`: the clippy lint being fixed.
    /// - `fix_direction`: the latent direction of the fix that was applied
    ///   (embedded from `(lint_name, fixed_code)`).
    /// - `success`: whether the fix resolved the lint.
    ///
    /// ## Initialization rule
    ///
    /// The direction is initialized ONLY on the first SUCCESS. A fail as the
    /// first outcome is a no-op for initialization — a fail tells us nothing
    /// about what a good fix looks like, so we wait for a success to establish
    /// the baseline. This prevents the failure mode where a bad first fix
    /// poisons the direction.
    ///
    /// After initialization, the EMA rule applies:
    /// - success: `dir += α · (fix_dir - dir)` (move toward)
    /// - fail: `dir -= β · (fix_dir - dir)` (move away)
    pub fn record_outcome(&mut self, lint_name: &str, fix_direction: &Direction, success: bool) {
        match self.directions.get_mut(lint_name) {
            Some(dir) => {
                // EMA update: component-wise arithmetic (the shared core —
                // see [`ema_step`]).
                ema_step(dir, fix_direction, success, &self.config);
            }
            None if success => {
                // First outcome is a success: initialize to the fix direction.
                // This fix works, so it's a valid baseline.
                self.directions
                    .insert(lint_name.to_string(), *fix_direction);
            }
            None => {
                // First outcome is a fail: no-op. A fail doesn't tell us what a
                // good fix looks like — wait for a success to initialize.
            }
        }
    }

    /// Score a candidate fix using the evolved direction for this lint.
    ///
    /// Returns `sigmoid(dot(evolved_dir, candidate_dir) / temperature) ∈ (0, 1)`.
    /// Higher = the candidate aligns better with the evolved preference.
    ///
    /// If no evolved direction exists for this lint, returns `0.5` (neutral —
    /// no preference). This makes the evolution layer transparent when no
    /// outcomes have been recorded.
    #[must_use]
    pub fn score_candidate(&self, lint_name: &str, candidate_dir: &Direction) -> f32 {
        match self.directions.get(lint_name) {
            Some(evolved) => {
                let dot = katgpt_core::simd::simd_dot_f32(evolved, candidate_dir, EMBED_DIM);
                katgpt_core::sigmoid(dot / self.config.temperature)
            }
            None => 0.5, // neutral — no preference
        }
    }

    /// Score a candidate, writing the dot-product into `dot_scratch` (for
    /// alloc-free batch scoring). Currently the same as `score_candidate`
    /// (the SIMD dot doesn't need scratch), but the API is reserved for
    /// future zero-alloc batch scoring.
    #[inline]
    pub fn score_candidate_into(
        &self,
        lint_name: &str,
        candidate_dir: &Direction,
        _dot_scratch: &mut [f32],
    ) -> f32 {
        self.score_candidate(lint_name, candidate_dir)
    }

    /// Check if a lint has an evolved direction (for testing / diagnostics).
    #[must_use]
    pub fn contains(&self, lint_name: &str) -> bool {
        self.directions.contains_key(lint_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::embed;

    #[test]
    fn unseen_lint_scores_neutral() {
        let ev = LatentSkillEvolution::default();
        let dir = embed("needless_return", "42");
        let score = ev.score_candidate("needless_return", &dir);
        assert!(
            (score - 0.5).abs() < 1e-5,
            "unseen lint must score 0.5 (neutral), got {score}"
        );
    }

    #[test]
    fn success_moves_direction_toward_fix() {
        let mut ev = LatentSkillEvolution::default();
        let fix_dir = embed("needless_return", "42");

        // Record a success — direction should initialize to fix_dir.
        ev.record_outcome("needless_return", &fix_dir, true);

        // Now the same fix direction should score > 0.5 (it aligns with the
        // evolved direction, which IS this direction).
        let score = ev.score_candidate("needless_return", &fix_dir);
        assert!(
            score > 0.5,
            "after a success, the same fix should score > 0.5, got {score}"
        );
    }

    #[test]
    fn fail_lowers_score_for_that_fix() {
        let mut ev = LatentSkillEvolution::default();
        let bad_fix = embed("needless_return", "return 42;"); // a wrong fix
        let good_fix = embed("needless_return", "42"); // a correct fix

        // First: a fail with the bad fix. Under the new initialization rule,
        // a fail as the first outcome is a NO-OP (we don't know what a good
        // fix looks like yet, so we don't initialize the direction).
        ev.record_outcome("needless_return", &bad_fix, false);
        assert!(
            !ev.contains("needless_return"),
            "first-fail must not initialize direction"
        );

        // Now record a success with the good fix — direction initializes to good_fix.
        ev.record_outcome("needless_return", &good_fix, true);

        // Now record another fail with the bad fix — direction moves AWAY from bad.
        ev.record_outcome("needless_return", &bad_fix, false);

        // The good fix should now score higher than the bad fix.
        let good_score = ev.score_candidate("needless_return", &good_fix);
        let bad_score = ev.score_candidate("needless_return", &bad_fix);
        assert!(
            good_score > bad_score,
            "good fix ({good_score}) should outscore bad fix ({bad_score}) after evolution"
        );
    }

    #[test]
    fn direction_components_stay_bounded() {
        // EMA accumulation could in theory drift outside [-1, 1]; the clamp
        // keeps it bounded. Verify after many updates.
        let mut ev = LatentSkillEvolution::default();
        let fix_dir = embed("needless_return", "42");

        // Record 100 successes (heavy push toward fix_dir).
        for _ in 0..100 {
            ev.record_outcome("needless_return", &fix_dir, true);
        }

        let evolved = ev.get("needless_return").unwrap();
        for &v in evolved {
            assert!(
                (-1.0..=1.0).contains(&v),
                "component {v} drifted out of [-1, 1]"
            );
            assert!(!v.is_nan());
        }
    }

    #[test]
    fn different_lints_have_independent_directions() {
        let mut ev = LatentSkillEvolution::default();
        let fix_a = embed("needless_return", "42");
        let fix_b = embed("redundant_closure", "f");

        ev.record_outcome("needless_return", &fix_a, true);
        ev.record_outcome("redundant_closure", &fix_b, true);

        // Score fix_a under lint_a (should be high) vs under lint_b (neutral,
        // since lint_b's direction is fix_b, not fix_a).
        let score_a_under_a = ev.score_candidate("needless_return", &fix_a);
        let score_a_under_b = ev.score_candidate("redundant_closure", &fix_a);

        assert!(
            score_a_under_a > score_a_under_b,
            "fix should score higher under its own lint: {score_a_under_a} vs {score_a_under_b}"
        );
    }

    #[test]
    fn record_outcome_is_alloc_free_after_warmup() {
        // G4 smoke: after the first outcome (which allocates the HashMap
        // entry), subsequent outcomes for the SAME lint must not allocate.
        let mut ev = LatentSkillEvolution::default();
        let fix_dir = embed("needless_return", "42");

        // Warm up: first outcome allocates the entry.
        ev.record_outcome("needless_return", &fix_dir, true);

        // Subsequent outcomes update in-place (no new allocation).
        // (We can't easily assert "0 allocations" without a counting allocator,
        // but we can verify the HashMap size doesn't grow.)
        let len_after_warmup = ev.len();
        for _ in 0..100 {
            ev.record_outcome("needless_return", &fix_dir, true);
        }
        assert_eq!(ev.len(), len_after_warmup, "HashMap grew — not alloc-free");
    }

    // ── Issue 037 E3: the shared EMA step (the durable store consumes it) ──

    #[test]
    fn ema_step_magnitude_is_rate_times_distance() {
        // dir = origin, fix = unit vector → a success applies α·||fix-dir||
        // per component; the returned magnitude is the L2 norm of that.
        let cfg = EvolveConfig::default();
        let mut dir = [0.0f32; EMBED_DIM];
        let mut fix = [0.0f32; EMBED_DIM];
        fix[0] = 1.0;

        let d = ema_step(&mut dir, &fix, true, &cfg);
        assert!((d - cfg.alpha).abs() < 1e-6, "success delta = α·1, got {d}");
        assert!((dir[0] - cfg.alpha).abs() < 1e-6);

        // Fail from dir=[α,0,..] with fix=[1,0,..]: moves AWAY by β·(1-α).
        let d = ema_step(&mut dir, &fix, false, &cfg);
        let expect = cfg.beta * (1.0 - cfg.alpha);
        assert!(
            (d - expect).abs() < 1e-6,
            "fail delta = β·distance, got {d}"
        );
        assert!((dir[0] - (cfg.alpha - expect)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::float_cmp)] // bit-identity with the layer IS the assertion
    fn ema_step_agrees_with_the_evolution_layer() {
        // The extracted core must be bit-compatible with what
        // `LatentSkillEvolution::record_outcome` does on an initialized
        // entry — the store's persisted rows consume the same rule.
        let fix = embed("needless_return", "42");
        let mut layer = LatentSkillEvolution::default();
        layer.record_outcome("needless_return", &fix, true); // initialize

        let mut dir = *layer.get("needless_return").unwrap();
        let other = embed("needless_return", "return 42;");
        layer.record_outcome("needless_return", &other, false);
        ema_step(&mut dir, &other, false, &EvolveConfig::default());

        assert_eq!(dir, *layer.get("needless_return").unwrap());
    }
}
