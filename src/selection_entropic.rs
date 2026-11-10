//! Entropic (max-seeking) selection tilt — the TTT-Discover within-batch
//! objective, transferred to code-healing candidate selection (Issue 026).
//!
//! **This module is a re-export shim.** The arithmetic was hoisted to
//! [`katgpt_core::entropic_tilt`] on 2026-08-18, when riir-train Plan 341
//! became its second consumer (the GRPO advantage estimator for game
//! self-play). Plan 341's rule is explicit — "do NOT duplicate past one
//! implementation + one import" — so the solver, the tilt, and their tests
//! live in katgpt-core and this module simply re-exports them. Issue 026's
//! public API is unchanged; nothing downstream needs to move.
//!
//! Source: arXiv:2601.16175 "Learning to Discover at Test Time"
//! (TTT-Discover), distilled at
//! `katgpt-rs/.research/019_TTT_Discover_Test_Time_Training.md`. The objective
//! itself is published prior art (RS-GRPO arXiv:2509.24261, RSPO
//! arXiv:2508.01174) — this module transfers it, it does not claim it.
//!
//! ## Mechanism (unchanged — see [`katgpt_core::entropic_tilt`] for detail)
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
//! with β solved per pool by bisection so `KL(q_β ‖ uniform) = γ`.
//!
//! ## Scope notes for THIS consumer (honest)
//!
//! - This selector uses the **batch** form ([`tilt_advantages_into`]), not the
//!   leave-one-out form katgpt-core also exposes. Both are strictly monotone
//!   in `r_i` within a fixed pool, so the ranking they induce is identical —
//!   and the argmax blend is this module's only consumer. The distinction
//!   matters for gradient-scaling consumers (riir-train), not here.
//! - Pools of size ≤ 1 have no shape; callers fall back to the per-candidate
//!   history score ([`crate::SelectionMode::BetaPosterior`]).
//! - Rewards must be finite. The heal path feeds belief scores in (0, 1);
//!   katgpt-core carries a `debug_assert` documenting the contract.
//! - The final mapping into the selection blend IS a sigmoid
//!   (`sigmoid(A_β)`), preserving the house sigmoid-not-softmax rule where it
//!   applies — the Boltzmann normalization here is the published objective's
//!   tilting mechanism, and its output is a signed zero-sum advantage rather
//!   than a competing gate.

pub use katgpt_core::entropic_tilt::{
    KL_BUDGET_LN2, solve_beta, tilt_advantages_into, tilted_weights,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Consumer-contract test (the algorithmic tests live with the
    /// implementation in katgpt-core): the re-exported path is the one
    /// `self_evolve` calls, and it must still produce zero-sum advantages
    /// that upweight the best candidate in a heal pool.
    #[test]
    fn reexported_tilt_still_serves_the_heal_pool_contract() {
        // Belief scores in (0, 1), the shape the heal path feeds.
        let pool = [0.31f32, 0.42, 0.88, 0.29];
        let mut adv = Vec::new();
        let beta = tilt_advantages_into(&pool, KL_BUDGET_LN2, &mut adv);

        assert_eq!(adv.len(), pool.len());
        assert!(beta > 0.0, "a non-degenerate pool must solve a positive β");

        // The strongest candidate wins the tilt.
        let best = adv
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i);
        assert_eq!(best, Some(2), "argmax must track the highest belief score");

        // Zero-sum: the batch form is what this consumer relies on.
        let sum: f32 = adv.iter().sum();
        assert!(sum.abs() < 1e-4, "advantages must be zero-sum, sum {sum}");
    }
}
