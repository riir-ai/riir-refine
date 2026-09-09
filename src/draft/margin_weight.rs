//! Margin-weighted mining substrate (issue 077 T2+T3 — arXiv:2604.26841's
//! margin law `w(M) ∝ e^{−2M}` on the healer's rule-mining order).
//!
//! The seam is the drafter's candidate ranking: [`super::corpus_index`]'s
//! per-rule best-score table sorted score-descending with the LOAD-BEARING
//! `total_cmp` + `rule_id` tie-break (touched nowhere here). The margin of
//! a span is the gap between its top-2 candidate scores under that canon —
//! the same shape [`super::jury_disagreement::disagreement_signals`] already
//! computes as `rerank_margin` on the RERANKED list. The two are deliberately
//! different views and this module consumes the PRE-rerank canon:
//!
//! - pre-rerank (the pool [`super::latent_matcher::LatentMatcher::
//!   pool_and_ranked`] returns first): pure score-desc → margin ≥ 0 by
//!   construction. THIS is the paper's margin (a similarity gap).
//! - post-rerank: instantiation-first ordering can override similarity, so
//!   the gap can go negative (a different, evidence-aware quantity — the
//!   disagreement module's business, not reused here).
//!
//! The weight law: `w(M) = e^{−2M}` — sigmoid-shaped, never softmax (the
//! house rule); a score-gap exponentiation, not a normalized distribution.
//! Smallest margin (decision-boundary span) → largest weight → mined first.
//! Uncontested spans (fewer than two candidates everywhere) carry
//! `INFINITY` margin → weight `e^{−∞} = 0` — no boundary, no review effort,
//! exactly the law's intent.
//!
//! Gated `latent_retrieval` (the matcher is the only consumer); the math
//! itself is pure `f32` and carries zero deps. G1 contract tests live at
//! the bottom; G3 (zero-alloc ranking) applies to the consumers' loops —
//! [`span_min_margin`] allocates nothing beyond the matcher's own scratch.

use super::corpus_index::ScoredRule;
use super::domain_router::DomainId;
use super::latent_matcher::LatentMatcher;

/// The paper's margin: `ranked[0].score − ranked[1].score` over a canonically
/// score-descending slice. `INFINITY` when fewer than two candidates exist
/// (uncontested by emptiness — the [`super::jury_disagreement`]
/// `rerank_margin` convention, applied to the pre-rerank pool).
///
/// Non-negativity is the caller's sort contract (the `total_cmp` + `rule_id`
/// canon guarantees it on the pool); the function itself only subtracts.
#[must_use]
pub fn contested_margin(ranked: &[ScoredRule]) -> f32 {
    if ranked.len() >= 2 {
        ranked[0].score - ranked[1].score
    } else {
        f32::INFINITY
    }
}

/// The margin law's weight: `w(M) = e^{−2M}` ∈ (0, 1], monotone decreasing
/// in M. `INFINITY` margin → `0.0` (uncontested spans get no mining
/// priority). NaN propagates — callers only ever pass subtracted scores of
/// the same retrieval run, which cannot be NaN.
#[must_use]
pub fn margin_weight(margin: f32) -> f32 {
    (-2.0 * margin).exp()
}

/// Per-span margin summary — the min contested margin plus how many
/// enabled domain views were actually contested (≥2 candidates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanMargin {
    /// The most-contested pre-rerank pool margin (min across domains);
    /// `INFINITY` when no domain offers two candidates.
    pub min: f32,
    /// How many enabled domains offered ≥2 candidates for the span.
    pub contested_domains: usize,
}

/// [`span_min_margin`] with the per-domain contest census — one retrieval
/// pass over the span, both numbers at once.
pub fn span_margin(matcher: &mut LatentMatcher, code: &str) -> SpanMargin {
    let mut min = f32::INFINITY;
    let mut contested = 0usize;
    for domain in DomainId::ALL {
        if let Some((pool, _ranked)) = matcher.pool_and_ranked(domain, code) {
            let m = contested_margin(&pool);
            if m.is_finite() {
                contested += 1;
            }
            if m < min {
                min = m;
            }
        }
    }
    SpanMargin { min, contested_domains: contested }
}

/// The most-contested margin of one span: the MINIMUM pre-rerank pool margin
/// across every enabled domain (a span is as contested as its most contested
/// domain view). `INFINITY` when no enabled domain offers two candidates —
/// weight 0 downstream, per the law.
///
/// Read-only over the retrieval substrate: `pool_and_ranked` replicates the
/// suggest path's per-domain steps on copies (the disagreement module's
/// access pattern, reused verbatim).
pub fn span_min_margin(matcher: &mut LatentMatcher, code: &str) -> f32 {
    span_margin(matcher, code).min
}

/// Mass fraction of the top-`k` weights of a descending-sorted slice — the
/// concentration statistic T2 asks for ("what fraction of the total
/// `e^{−2M}` mass do the smallest-margin items carry"). Weights must be
/// sorted descending (largest first); `k` beyond the slice is clamped.
///
/// By construction the top half (k = ceil(n/2)) always carries ≥ 50%: each
/// of the first-⌈n/2⌉ weights is ≥ each of the rest (descending sort), so
/// their sum dominates the remainder termwise. The G1 test pins that
/// inequality — including the all-tied degenerate case, where the split is
/// exactly 50% at even n and strictly above at odd n.
// Count→ratio casts over population sizes (≤ 10⁵ here); allowed with cause.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn concentration_fraction(weights_desc: &[f32], k: usize) -> f32 {
    let total: f32 = weights_desc.iter().sum();
    if total <= 0.0 || weights_desc.is_empty() {
        return 0.0;
    }
    let k = k.min(weights_desc.len());
    let head: f32 = weights_desc[..k].iter().sum();
    head / total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, score: f32) -> ScoredRule {
        ScoredRule { rule_id: id.to_string(), score, instantiates: false }
    }

    /// G1a — margins are non-negative under the canonical sort: the pool
    /// half of `pool_and_ranked` is `rank_into`'s output (score-desc,
    /// `total_cmp` + `rule_id`), so top1 − top2 ≥ 0; equal scores → 0
    /// (never negative), and the canonical tie-break keeps the order
    /// deterministic. Pinned here on canonical slices; the real-pool
    /// half of the contract is pinned by the integration test below.
    #[test]
    fn margins_are_non_negative_under_the_canonical_sort() {
        // Binary-exact values — the assertion is on the arithmetic, not on
        // decimal rounding (the f32 literals below are exactly representable,
        // and 0.75 - 0.5 is exact binary subtraction; float_cmp is precise
        // here BY CONSTRUCTION, not by tolerance).
        #![allow(clippy::float_cmp)]
        let canon = vec![rule("a-rule", 0.75), rule("b-rule", 0.5), rule("c-rule", 0.25)];
        assert_eq!(contested_margin(&canon), 0.25);
        let tied = vec![rule("a-rule", 0.5), rule("b-rule", 0.5)];
        assert_eq!(contested_margin(&tied), 0.0, "exact tie → zero margin");
        assert!(contested_margin(&tied) >= 0.0);
        // Descending order respected: swapping would be a caller bug, and
        // the function does not paper over it (negative is visible, not
        // clamped) — the law still applies (weight > 1), but the sort
        // contract is what makes non-negativity a caller invariant.
        let swapped = vec![rule("b-rule", 0.4), rule("a-rule", 0.9)];
        assert!(contested_margin(&swapped) < 0.0, "unsorted input is visible");
    }

    /// G1b — fewer than two candidates is uncontested (`INFINITY`), and the
    /// weight law sends it to exactly zero: no boundary, no mining priority.
    #[test]
    fn uncontested_spans_carry_zero_weight() {
        // float_cmp allowed: the weight law sends INFINITY to exactly 0.0
        // (underflow to the limit form) — the equality IS the pinned law.
        #![allow(clippy::float_cmp)]
        let single = vec![rule("a-rule", 0.9)];
        assert_eq!(contested_margin(&single), f32::INFINITY);
        assert_eq!(margin_weight(contested_margin(&single)), 0.0);
        assert_eq!(margin_weight(contested_margin(&[])), 0.0);
    }

    /// G1c — the weight law: `e^{−2M}` is monotone decreasing in M, equals 1
    /// at M = 0, stays in (0, 1] for M ≥ 0, and never goes negative.
    #[test]
    fn margin_weight_is_monotone_decreasing_in_margin() {
        // float_cmp allowed: margin 0.0 → weight exactly 1.0 (e⁰) is the
        // law's anchor point, not a rounded comparison.
        #![allow(clippy::float_cmp)]
        assert_eq!(margin_weight(0.0), 1.0);
        let margins = [0.0, 1e-6, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 100.0];
        for w in margins.iter().map(|&m| margin_weight(m)) {
            // f32 underflows to exactly 0.0 around M ≈ 44 — zero is the
            // correct limit form (an uncontested/large-margin span gets no
            // mining priority), never negative.
            assert!((0.0..=1.0).contains(&w), "weight out of range: {w}");
        }
        for pair in margins.windows(2) {
            assert!(
                margin_weight(pair[0]) >= margin_weight(pair[1]),
                "weight must decrease as margin grows: {} !≥ {}",
                margin_weight(pair[0]),
                margin_weight(pair[1])
            );
        }
    }

    /// G1d — the concentration inequality: the lowest-margin half (top half
    /// of the descending weights) always carries ≥ 50% of the total mass.
    /// True by construction for ANY non-negative descending weights; pinned
    /// on graded, steep, and the all-tied degenerate population.
    #[test]
    fn lowest_margin_half_carries_at_least_half_the_mass() {
        // cast_possible_truncation allowed: i ≤ 100 · 0.01 and i ≤ 20 · 0.5
        // are exact in f32 (small integers × a power-of-two-friendly scale —
        // the multiplication is the test's own fixture shape).
        #![allow(clippy::cast_possible_truncation)]
        let graded: Vec<f32> = (1..=100).map(|i| margin_weight(i as f32 * 0.01)).collect();
        let steep: Vec<f32> = (0..20).map(|i| margin_weight(i as f32 * 0.5)).collect();
        let all_tied: Vec<f32> = vec![margin_weight(0.25); 50];
        let odd_tied: Vec<f32> = vec![margin_weight(0.25); 51];
        for weights in [&graded, &steep, &all_tied, &odd_tied] {
            let half = weights.len().div_ceil(2);
            let frac = concentration_fraction(weights, half);
            // 1e-5 slack: the all-tied even case lands ON the boundary and
            // f32 summation rounds 25w/50w to 0.49999988.
            assert!(
                frac >= 0.5 - 1e-5,
                "lowest-margin half must carry ≥50% (got {frac}, n={})",
                weights.len()
            );
        }
        // The degenerate all-tied even case sits exactly on the boundary.
        assert!((concentration_fraction(&all_tied, 25) - 0.5).abs() < 1e-5);
    }

    /// G1e — the real substrate half of the non-negativity contract: over
    /// the frozen v1 fixtures, every pre-rerank pool margin the matcher
    /// produces is ≥ 0 (the `rank_into` canon at work end-to-end), and at
    /// least one span is genuinely contested (margin < INFINITY) so the
        /// spectrum example has something to measure.
    #[test]
    fn real_pool_margins_are_non_negative_and_sometimes_contested() {
        let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
        let mut saw_contested = false;
        for fx in crate::score_bench::fixtures::fixtures().iter().take(8) {
            let m = span_min_margin(&mut matcher, fx.code);
            assert!(m >= 0.0 || m.is_infinite(), "negative pool margin: {m}");
            if m.is_finite() {
                saw_contested = true;
            }
        }
        assert!(saw_contested, "expected ≥1 contested span in the frozen set");
    }

    /// `concentration_fraction` clamps and never divides by zero.
    #[test]
    fn concentration_fraction_handles_edges() {
        // float_cmp allowed: the clamp arms return literal 0.0/1.0 — the
        // equality is the contract, not a rounded comparison.
        #![allow(clippy::float_cmp)]
        assert_eq!(concentration_fraction(&[], 1), 0.0);
        assert_eq!(concentration_fraction(&[0.0, 0.0], 1), 0.0);
        assert_eq!(concentration_fraction(&[1.0, 0.5], 99), 1.0, "k clamped to len");
        assert_eq!(concentration_fraction(&[1.0, 0.5], 0), 0.0);
    }
}
