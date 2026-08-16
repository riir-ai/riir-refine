//! Entropic (max-seeking) selection tilt — the TTT-Discover within-batch
//! objective, transferred to code-healing candidate selection (Issue 026).
//!
//! Source: arXiv:2601.16175 "Learning to Discover at Test Time"
//! (TTT-Discover), distilled at
//! `katgpt-rs/.research/019_TTT_Discover_Test_Time_Training.md`. The objective
//! itself is published prior art (RS-GRPO arXiv:2509.24261, RSPO
//! arXiv:2508.01174) — this module transfers it, it does not claim it.
//!
//! ## Mechanism
//!
//! Where [`SelectionMode::BetaPosterior`](crate::SelectionMode) scores each
//! candidate's reliability from its OWN history counts (mean-seeking — the
//! ε-quantile estimates average reliability), the entropic tilt scores
//! candidates from the SHAPE of the current pool:
//!
//! ```text
//! q_β(i) = exp(β·r_i) / Σ_j exp(β·r_j)      (tilted distribution)
//! w_β(i) = n · q_β(i)                        (weights, mean 1 over the pool)
//! A_β(i) = w_β(i) − 1                        (advantages, sum 0)
//! ```
//!
//! with β solved per pool by bisection so the tilt stays within a KL budget:
//! `KL(q_β ‖ uniform) = γ` (the paper's default `γ = ln 2`).
//!
//! Auto-adaptation is the point: a pool of consistent small improvements
//! permits a large β (aggressive max-seeking — tiny belief gaps get amplified
//! into decisive selection); an outlier-dominated pool forces small β, and
//! the budget bounds the maximum weight regardless of gap magnitude (one
//! lucky candidate cannot own selection). The tilt is shift/scale invariant
//! in the rewards (`r → a·r + b` yields identical advantages — paper
//! Appendix A.1).
//!
//! ## Why a softmax normalization here does not violate the house
//! sigmoid-not-softmax rule
//!
//! The rule targets ROUTING/GATING decisions where softmax forces exclusive
//! competition between additive gates. Here the Boltzmann normalization is
//! the published objective's tilting mechanism, its output is an ADVANTAGE
//! (signed, zero-sum over the pool — not a competing gate), and the final
//! mapping into the selection blend IS a sigmoid (`sigmoid(A_β)`), preserving
//! the rule where it applies.
//!
//! ## Scope notes (honest)
//!
//! - The paper's leave-one-out estimator is NOT implemented: both the batch
//!   and LOO forms are strictly monotone in `r_i` within a fixed pool, so
//!   the ranking they induce is identical — and the argmax blend is this
//!   module's only consumer.
//! - Pools of size ≤ 1 have no shape; callers fall back to the per-candidate
//!   history score ([`crate::SelectionMode::BetaPosterior`]).
//! - Rewards must be finite. The heal path feeds belief scores in (0, 1);
//!   a `debug_assert` documents the contract.

/// KL budget γ — the paper's default `ln 2`.
pub const KL_BUDGET_LN2: f32 = core::f32::consts::LN_2;

/// Bisection tolerance on the achieved KL.
const KL_TOL: f32 = 1e-4;

/// Bisection iterations (halves the β bracket 60× — far past f32 precision).
const BISECT_ITERS: usize = 60;

/// Upper β bracket. Rewards here are belief scores in (0, 1); β = 1e3
/// already saturates any pool this selector sees to one-hot, so the bracket
/// spans the full uniform→argmax range.
const BETA_MAX: f32 = 1e3;

/// `KL(q_β ‖ uniform)` for the pool, computed streaming (zero allocation).
///
/// `KL = Σ q_i · ln(n · q_i)` — with the `r_max`-subtracted
/// parameterization, `ln(n·q_i) = ln n + β(r_i − r_max) − ln Z` where
/// `Z = Σ exp(β(r_j − r_max)) ≥ 1` (the max element contributes `exp(0)`).
#[inline]
#[allow(clippy::cast_precision_loss)] // pool sizes ≤ ~10²
fn kl_vs_uniform(rewards: &[f32], beta: f32, r_max: f32) -> f32 {
    let n = rewards.len() as f32;
    let ln_n = n.ln();
    let mut z = 0.0f32;
    for &r in rewards {
        z += (beta * (r - r_max)).exp();
    }
    let ln_z = z.ln();
    let mut kl = 0.0f32;
    for &r in rewards {
        let q = (beta * (r - r_max)).exp() / z;
        // q·ln(n·q): an underflowed q = 0 contributes 0, not NaN.
        if q > 0.0 {
            kl += q * (ln_n + beta * (r - r_max) - ln_z);
        }
    }
    kl
}

/// Solve β ≥ 0 by bisection so `KL(q_β ‖ uniform) ≈ gamma`.
///
/// KL is monotonically non-decreasing in β (β = 0 → uniform → KL = 0;
/// β → ∞ → one-hot on argmax → KL = ln n). Returns the converged β.
/// Degenerate pools (all rewards equal) have KL = 0 for every β, so the
/// budget is unreachable and the upper bracket is returned — harmless,
/// because the weights ignore β entirely in that case.
///
/// Zero-allocation: the KL is recomputed streaming per bisection iteration.
#[must_use]
pub fn solve_beta(rewards: &[f32], gamma: f32) -> f32 {
    debug_assert!(
        rewards.iter().all(|r| r.is_finite()),
        "entropic tilt requires finite rewards"
    );
    if rewards.is_empty() {
        return 0.0;
    }
    let r_max = rewards.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut lo = 0.0f32; // KL(0) = 0 ≤ gamma
    let mut hi = BETA_MAX;
    if kl_vs_uniform(rewards, hi, r_max) <= gamma {
        // Budget unreachable (or exactly reachable only at saturation —
        // n = 2 hits KL = ln 2 at one-hot): every β beyond is equivalent.
        return hi;
    }
    for _ in 0..BISECT_ITERS {
        let mid = 0.5 * (lo + hi);
        let kl = kl_vs_uniform(rewards, mid, r_max);
        if (kl - gamma).abs() < KL_TOL {
            return mid;
        }
        if kl < gamma {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Write the tilted weights `w_β(i) = n · q_β(i)` (mean 1 over the pool)
/// into `weights_out` (cleared + refilled — caller-reused scratch).
///
/// `beta` typically comes from [`solve_beta`]. Overflow-safe by
/// construction: every exponent is `β(r − r_max) ≤ 0`.
pub fn tilted_weights(rewards: &[f32], beta: f32, weights_out: &mut Vec<f32>) {
    debug_assert!(
        rewards.iter().all(|r| r.is_finite()),
        "entropic tilt requires finite rewards"
    );
    #[allow(clippy::cast_precision_loss)] // pool sizes ≤ ~10²
    let n = rewards.len() as f32;
    let r_max = rewards.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut z = 0.0f32;
    for &r in rewards {
        z += (beta * (r - r_max)).exp();
    }
    // z ≥ 1 always (the max element contributes exp(0) = 1) — no guard needed.
    weights_out.clear();
    weights_out.reserve(rewards.len());
    for &r in rewards {
        weights_out.push(n * (beta * (r - r_max)).exp() / z);
    }
}

/// Solve β for the pool + write advantages `A_β(i) = w_β(i) − 1` into
/// `advantages_out` (cleared + refilled — caller-reused scratch).
///
/// The one-call form for selection: rewards in, zero-sum advantages out
/// (sum ≈ 0 within f32 tolerance). The caller maps through
/// [`katgpt_core::sigmoid`] for a [0,1]-comparable reliability score.
/// Returns the solved β.
pub fn tilt_advantages_into(rewards: &[f32], gamma: f32, advantages_out: &mut Vec<f32>) -> f32 {
    let beta = solve_beta(rewards, gamma);
    tilted_weights(rewards, beta, advantages_out);
    for a in advantages_out.iter_mut() {
        *a -= 1.0;
    }
    beta
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// (a) Shift/scale invariance — paper Appendix A.1.
    #[test]
    fn tilt_is_shift_scale_invariant() {
        let base = [0.2f32, 0.5, 0.6, 0.9];
        let shifted: Vec<f32> = base.iter().map(|r| 3.0 * r + 7.0).collect();
        let mut a1 = Vec::new();
        let mut a2 = Vec::new();
        tilt_advantages_into(&base, KL_BUDGET_LN2, &mut a1);
        tilt_advantages_into(&shifted, KL_BUDGET_LN2, &mut a2);
        for (i, (x, y)) in a1.iter().zip(&a2).enumerate() {
            assert!(
                (x - y).abs() < 1e-3,
                "advantage {i} drifted under r → 3r+7: {x} vs {y}"
            );
        }
    }

    /// (b) Exact KL budget at convergence.
    #[test]
    fn solved_beta_hits_kl_budget() {
        let rewards = [0.1f32, 0.4, 0.35, 0.8, 0.5];
        let beta = solve_beta(&rewards, KL_BUDGET_LN2);
        let r_max = rewards.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let kl = kl_vs_uniform(&rewards, beta, r_max);
        assert!(
            (kl - KL_BUDGET_LN2).abs() < 1e-3,
            "solved β achieves KL {kl}, budget {KL_BUDGET_LN2}"
        );
    }

    /// (c) Degenerate pools: all-equal rewards → uniform weights, zero
    /// advantages, regardless of β.
    #[test]
    fn all_equal_rewards_yield_uniform_weights() {
        let rewards = [0.5f32; 6];
        let mut adv = Vec::new();
        let beta = tilt_advantages_into(&rewards, KL_BUDGET_LN2, &mut adv);
        assert_eq!(adv.len(), 6);
        for (i, &a) in adv.iter().enumerate() {
            assert!(a.abs() < 1e-5, "advantage {i} should be 0, got {a}");
        }
        // Any β yields uniform here — the returned β is the (harmless)
        // bracket value; weights must not depend on it.
        let mut w = Vec::new();
        tilted_weights(&rewards, beta, &mut w);
        for &x in &w {
            assert!((x - 1.0).abs() < 1e-5, "weight should be 1.0, got {x}");
        }
    }

    /// (c) Degenerate pools: a single candidate has no shape.
    #[test]
    fn single_element_pool_is_neutral() {
        let rewards = [0.7f32];
        let mut adv = Vec::new();
        tilt_advantages_into(&rewards, KL_BUDGET_LN2, &mut adv);
        assert_eq!(adv.len(), 1);
        // w = n·q = 1·1 = 1 → A = 0.
        assert!(adv[0].abs() < 1e-5);
    }

    /// (d) Rare-success binary pool upweights the successes (the RS-GRPO
    /// exploration-dilemma shape) — and stays zero-sum.
    #[test]
    fn rare_successes_get_upweighted() {
        let rewards = [0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let mut adv = Vec::new();
        tilt_advantages_into(&rewards, KL_BUDGET_LN2, &mut adv);
        assert!(adv[7] > 0.0, "the rare success must be upweighted");
        for (i, &a) in adv[..7].iter().enumerate() {
            assert!(a < 0.0, "failure {i} must be downweighted, got {a}");
        }
        let sum: f32 = adv.iter().sum();
        assert!(sum.abs() < 1e-4, "advantages must be zero-sum, sum {sum}");
    }

    /// Auto-adaptation: bigger reward gaps need SMALLER β to hit the same
    /// budget (the outlier pool is prevented from tilting harder just
    /// because its outlier is extreme).
    #[test]
    fn beta_shrinks_as_gaps_grow() {
        let tight = [0.49f32, 0.50, 0.51]; // consistent small differences
        let wide = [0.0f32, 0.5, 1.0]; // outlier-dominated
        let b_tight = solve_beta(&tight, KL_BUDGET_LN2);
        let b_wide = solve_beta(&wide, KL_BUDGET_LN2);
        assert!(
            b_wide < b_tight,
            "wider gaps should need smaller β: wide {b_wide} vs tight {b_tight}"
        );
    }

    /// The budget caps concentration: the max tilted weight is bounded well
    /// below the one-hot value (n), for both moderate and extreme pools.
    #[test]
    #[allow(clippy::cast_precision_loss)] // pool sizes ≤ ~10²
    fn kl_budget_bounds_max_weight() {
        let moderate = [0.4f32, 0.5, 0.6];
        let extreme = [0.0f32, 0.0, 1.0];
        for pool in [moderate, extreme] {
            let beta = solve_beta(&pool, KL_BUDGET_LN2);
            let mut w = Vec::new();
            tilted_weights(&pool, beta, &mut w);
            let w_max = w.iter().copied().fold(0.0f32, f32::max);
            // One-hot at n = 3 would be w = 3.0; the ln-2 budget must keep
            // the max meaningfully below that.
            assert!(
                w_max < 2.9,
                "budget must cap the max weight ({w_max} for {pool:?})"
            );
            // And the mean stays 1 (zero-sum advantages).
            let mean: f32 = w.iter().sum::<f32>() / w.len() as f32;
            assert!((mean - 1.0).abs() < 1e-4);
        }
    }

    /// Determinism: identical inputs → bit-identical outputs.
    #[test]
    fn tilt_is_deterministic() {
        let rewards = [0.3f32, 0.31, 0.9, 0.42, 0.15];
        let (mut a1, mut a2) = (Vec::new(), Vec::new());
        let b1 = tilt_advantages_into(&rewards, KL_BUDGET_LN2, &mut a1);
        let b2 = tilt_advantages_into(&rewards, KL_BUDGET_LN2, &mut a2);
        assert_eq!(b1.to_bits(), b2.to_bits());
        assert_eq!(a1.len(), a2.len());
        for (x, y) in a1.iter().zip(&a2) {
            assert_eq!(x.to_bits(), y.to_bits());
        }
    }

    /// G2 alloc-freedom (the honest equivalent of a `CountingAllocator`
    /// spot-check for a pure-f32 module): with pre-allocated scratch, the
    /// solve+weights path never grows capacity across many calls.
    #[test]
    #[allow(clippy::cast_precision_loss)] // loop counters → f32 reward jitter
    fn tilt_scratch_capacity_stabilizes() {
        let rewards = [0.9f32, 0.4, 0.4, 0.55, 0.3, 0.62];
        let mut out = Vec::new();
        // Warm-up: first call sizes the scratch.
        tilt_advantages_into(&rewards, KL_BUDGET_LN2, &mut out);
        let cap_after_warmup = out.capacity();
        for k in 0..1000u32 {
            // Vary rewards slightly — same length, different values.
            let r: Vec<f32> = rewards
                .iter()
                .enumerate()
                .map(|(i, &x)| (x + (k % 7) as f32 * 0.001 * (i as f32 + 1.0)) % 1.0)
                .collect();
            tilt_advantages_into(&r, KL_BUDGET_LN2, &mut out);
            assert_eq!(out.capacity(), cap_after_warmup, "capacity grew mid-loop");
        }
    }
}
