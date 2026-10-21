//! Issue 126 lane (b) — the Lemma-4.2 likelihood-ratio tier guard.
//!
//! arXiv:2206.07568 (CRL, Eysenbach et al.) Lemma 4.2 prices hindsight
//! credit transfer: relabel a trajectory τ from the goal it reached to the
//! goal it was commanded only when the two trajectory likelihoods agree —
//!
//! ```text
//! exclude  when  | π(τ|s_g) / π(τ|s_j) − 1 | > ε
//! cost     of exclusion:  V^{π'} ≥ V^π − 2γε/(1−γ)
//! ```
//!
//! Otherwise the credit is WRONG-GOAL credit: the trajectory's probability
//! mass belongs to the reached outcome, not the commanded goal, and
//! crediting it under the commanded goal misprices the value by at most
//! the closed-form slack above.
//!
//! # The healer mapping (one line each)
//!
//! - τ — one fix trajectory: the store row `(rule, shape, fix)` and its
//!   keep/revert history.
//! - `s_g` (commanded goal) — "this rule fixes this lint on this shape".
//!   `π(τ|s_g)` = the store's goal-side model, the Elo expected score
//!   [`crate::elo::expected_score`]`(rule_elo, span_elo)` over the two
//!   rating fields every [`crate::traj_store::TrajRowDoc`] durably carries
//!   (Issue 039's batch replay: cross-row evidence — the rule's strength
//!   across ALL its rows, the span's difficulty across ALL rules).
//! - `s_j` (reached goal) — what the keep event actually is: the row's own
//!   realized outcome history. `π(τ|s_j)` = the Laplace posterior mean over
//!   the row's counters INCLUDING the arbitrated event — the event is part
//!   of the trajectory the reached dynamics produced, not a judge of its
//!   own trial.
//! - The certify transition (`success && verified` → Certified) IS the
//!   hindsight relabel: the run-level keep (reached) becomes goal
//!   attainment (commanded). [`certify_allowed`] gates it.
//!
//! # The two exclusion directions (both are real defects)
//!
//! - ratio ≪ 1 — the keep is far more likely under reached dynamics than
//!   the goal model predicts: the free-rider / wrong-goal class (a kept
//!   fix that did not attain its commanded diagnostic; a disfavored rule
//!   certified by one lucky keep).
//! - ratio ≫ 1 — the goal model believes but the row's own outcome
//!   history refutes: the overcertification class (a mostly-failing row
//!   certified on a single keep).
//!
//! # The Issue-033 mandate (read before touching blend constants)
//!
//! Issue 033's TETHER-adaptive selection blend measured NEGATIVE (Bench
//! 042; `5494dbec`, retired `159a0569`): adaptive re-blending lost to the
//! pinned `W_EVO`/`W_RATE` constants. Consequences carried here:
//!
//! 1. This guard is an EXCLUSION filter on one transition — it never
//!    re-weights the selection blend, the reliability axis, or the
//!    Withdrawn ladder.
//! 2. The knobs are pinned constants ([`TIER_GUARD_EPSILON`],
//!    [`TIER_GUARD_GAMMA`]) — no adaptive windows, no online tuning.
//! 3. The cold start is behavior-identical: a fresh row at base ratings
//!    reads `π_g = π_j` = 0.5, ratio exactly 1.0 — every success-only row
//!    certifies exactly as before the guard existed (at base ratings the
//!    guard refuses only rows with `F > 2S + 1`).
//!
//! Non-finite stored ratings fail CLOSED (refuse certification) — an
//! overflowed rating must never sail through on a NaN comparison (the
//! `t < 1e-20`-is-false-for-NaN lesson).

/// The exclusion tolerance: the two probabilities must agree within a
/// factor of `1 + ε`. 0.5 admits the entire success-only population and
/// every row with `F ≤ 2S + 1` at base ratings; it refuses genuinely
/// lopsided evidence (a 3×-failing row's lucky keep; a disfavored rule's
/// free-rider keep). Pinned, not adaptive — the Issue-033 lesson.
pub const TIER_GUARD_EPSILON: f32 = 0.5;

/// The discount parameter of the priced bound `2γε/(1−γ)` — the
/// documented cost of exclusion, never a behavior input. Paper-typical 0.9.
pub const TIER_GUARD_GAMMA: f32 = 0.9;

/// `π(τ|s_g)` — the goal-conditioned probability: the store's own belief
/// that this rule attains this goal on this span, from the two logged
/// rating fields. Delegates to the Issue-039 Elo expected score (DRY —
/// one win-probability formula in the crate).
#[must_use]
#[inline]
pub fn goal_probability(rule_elo: f32, span_elo: f32) -> f32 {
    crate::elo::expected_score(rule_elo, span_elo)
}

/// `π(τ|s_j)` — the reached-outcome probability: the Laplace posterior mean
/// over the row's own counters (Beta(1,1) prior — no extra knob). Callers
/// pass the counters INCLUDING the arbitrated event (see the module doc).
#[must_use]
#[inline]
#[allow(clippy::cast_precision_loss)] // u32 counters < 2^24: f32 is exact (elo.rs precedent)
pub fn outcome_probability(successes: u32, fails: u32) -> f32 {
    (successes as f32 + 1.0) / (successes as f32 + fails as f32 + 2.0)
}

/// The paper's exclusion test, verbatim: `|π_g/π_j − 1| > ε`.
///
/// Both inputs are strictly positive by construction (the sigmoid is in
/// (0,1); the Laplace mean is ≥ 1/(S+F+2) > 0), so no division hazard —
/// but NaN inputs compare false and would silently ADMIT; the composition
/// entry point checks finiteness instead (fail-closed).
#[must_use]
#[inline]
pub fn lemma_4_2_excludes(goal: f32, outcome: f32, epsilon: f32) -> bool {
    ((goal / outcome) - 1.0).abs() > epsilon
}

/// The documented cost of one exclusion: the closed-form slack
/// `2γε/(1−γ)` — the most value the filter can forfeit, in the paper's
/// units. Report-only; nothing in the heal path consumes it.
#[must_use]
#[inline]
pub fn priced_slack(gamma: f32, epsilon: f32) -> f32 {
    2.0 * gamma * epsilon / (1.0 - gamma)
}

/// The store-facing composition: may the `success && verified` event
/// certify its row? Fail-closed on non-finite STORED RATINGS — checked at
/// the INPUT, because an infinite rating yields a FINITE expected score
/// through the sigmoid (`expected_score(+∞, base) = 1.0`), so an
/// output-only check would silently admit a corrupt store (the Batch-54
/// NaN guard corollary, applied one step earlier).
///
/// `successes`/`fails` are the row's counters AFTER the event was
/// recorded — the arbitrated trajectory includes its own outcome.
#[must_use]
pub fn certify_allowed(rule_elo: f32, span_elo: f32, successes: u32, fails: u32) -> bool {
    if !rule_elo.is_finite() || !span_elo.is_finite() {
        return false;
    }
    let goal = goal_probability(rule_elo, span_elo);
    let outcome = outcome_probability(successes, fails);
    if !goal.is_finite() || !outcome.is_finite() {
        return false;
    }
    !lemma_4_2_excludes(goal, outcome, TIER_GUARD_EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elo::default_elo;

    // ── the cold-start contract (Issue-033 lesson 3) ────────────────────

    /// A fresh row at base ratings: `π_g = π_j` = 0.5 exactly, ratio 1.0 —
    /// the guard is behavior-identical on the dominant historical path.
    #[test]
    #[allow(clippy::float_cmp)] // base equality IS the assertion (traj_store test precedent)
    fn fresh_row_ratio_is_exactly_one_and_certifies() {
        assert_eq!(goal_probability(default_elo(), default_elo()), 0.5);
        assert_eq!(outcome_probability(0, 0), 0.5);
        assert!(!lemma_4_2_excludes(0.5, 0.5, TIER_GUARD_EPSILON));
        assert!(certify_allowed(default_elo(), default_elo(), 1, 0));
    }

    /// Every success-only row certifies at base ratings, whatever the
    /// sample size (the historical population the guard must not move).
    #[test]
    fn success_only_rows_certify_at_base_ratings() {
        for s in [1u32, 2, 3, 10, 100] {
            assert!(
                certify_allowed(default_elo(), default_elo(), s, 0),
                "S={s} refused at base ratings"
            );
        }
    }

    // ── the two exclusion directions ─────────────────────────────────────

    /// G1 poisoned shape (pure half): the goal model disfavors the rule
    /// while the row's keeps are strong — wrong-goal / free-rider credit.
    #[test]
    fn poisoned_wrong_goal_credit_is_excluded() {
        // disfavored rule (p_goal 0.05) + strong local keeps (p_outcome 0.8)
        assert!(lemma_4_2_excludes(0.05, 0.8, TIER_GUARD_EPSILON));
        assert!(!certify_allowed(
            default_elo() - 760.0, // ≈ p_goal 0.05 vs base span
            default_elo(),
            3,
            0
        ));
    }

    /// The mirror direction: the goal model believes but the row's own
    /// history refutes — the overcertification class.
    #[test]
    fn overcertified_lucky_keep_is_excluded() {
        // favored rule (p_goal 0.95) + a mostly-failing row (p_outcome 0.2)
        assert!(lemma_4_2_excludes(0.95, 0.2, TIER_GUARD_EPSILON));
        assert!(!certify_allowed(default_elo() + 760.0, default_elo(), 1, 4));
    }

    // ── boundary + arithmetic ─────────────────────────────────────────────

    /// At exactly the tolerance the event passes (strict `>`), a hair
    /// outside it is excluded. (The boundary itself is float-fragile at
    /// 1.5; the test sits clearly on each side.)
    #[test]
    fn exclusion_is_strictly_outside_the_tolerance() {
        let goal = 0.6;
        let outcome = 0.5;
        // ratio 1.2 — inside the factor-1.5 band.
        assert!(!lemma_4_2_excludes(goal, outcome, TIER_GUARD_EPSILON));
        // ratio 1.7 — outside it.
        assert!(lemma_4_2_excludes(0.85, 0.5, TIER_GUARD_EPSILON));
    }

    /// Laplace values: (S+1)/(S+F+2) — pinned so a formula regression is
    /// a named failure, not a shifted band.
    #[test]
    #[allow(clippy::float_cmp)] // exact same-expression equality IS the pin
    fn outcome_probability_matches_laplace() {
        assert_eq!(outcome_probability(0, 0), 0.5);
        assert_eq!(outcome_probability(3, 0), 0.8); // 4/5
        assert_eq!(outcome_probability(0, 4), 1.0 / 6.0); // (0+1)/(0+4+2)
        assert_eq!(outcome_probability(2, 3), 3.0 / 7.0);
    }

    /// The goal side delegates to the ONE Elo expected-score formula.
    #[test]
    #[allow(clippy::float_cmp)] // bit-identity of the delegation IS the assertion
    fn goal_probability_delegates_to_elo_expected_score() {
        for (a, b) in [(1200.0f32, 1200.0), (924.0, 1476.0), (1500.0, 900.0)] {
            assert_eq!(goal_probability(a, b), crate::elo::expected_score(a, b));
        }
        assert!(goal_probability(default_elo(), default_elo() + 400.0) < 0.11);
    }

    /// The priced bound: 2·0.9·0.5/0.1 = 9 — the documented cost. (f32
    /// reads 8.999998; the gate is a tolerance, not an exact-equality
    /// knife edge.)
    #[test]
    fn priced_slack_matches_the_closed_form() {
        assert!((priced_slack(0.9, 0.5) - 9.0).abs() < 1e-5);
        assert!((priced_slack(TIER_GUARD_GAMMA, TIER_GUARD_EPSILON) - 9.0).abs() < 1e-5);
    }

    // ── fail-closed on corrupt evidence ──────────────────────────────────

    /// NaN/overflowed ratings REFUSE certification (a NaN comparison is
    /// false and would silently admit — the Batch-54 guard corollary).
    #[test]
    fn non_finite_ratings_fail_closed() {
        assert!(!certify_allowed(f32::NAN, default_elo(), 1, 0));
        assert!(!certify_allowed(default_elo(), f32::NAN, 1, 0));
        assert!(!certify_allowed(f32::INFINITY, default_elo(), 1, 0));
    }
}
