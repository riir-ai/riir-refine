//! Issue 134 T2 — the pool-depth allocation gate's WIRING gates (the
//! matcher seam; the pure policy tests live ungated in
//! `src/demonstrability_gate.rs`).
//!
//! Three assertions, one per posture of the gate:
//!
//! 1. **identity = byte-identical** — an installed identity policy
//!    reproduces the default matcher's suggest output exactly (the A/B
//!    control's premise, and the unarmed feature-ON posture);
//! 2. **low demonstrability shallows** — a low-constant domain's served
//!    list is capped below the caller's `top_k` (a deterministic UPPER
//!    bound, holding regardless of pool richness);
//! 3. **high demonstrability deepens** — a high-constant domain serves
//!    MORE than the caller's `top_k` when the pool has the candidates
//!    (neutral serves exactly `top_k`; the frozen in-repo fixture +
//!    corpus make the pool richness deterministic — the same premise the
//!    `instantiation_rerank` pool tests rely on).
//!
//! ```bash
//! cargo test --features demonstrability_gate --test demonstrability_gate_wiring
//! ```

#![cfg(feature = "demonstrability_gate")]

use riir_refine::demonstrability::bench_row;
use riir_refine::demonstrability_gate::DepthPolicy;
use riir_refine::draft::corpus_index::ScoredRule;
use riir_refine::draft::domain_router::DomainId;
use riir_refine::draft::eval_fixtures_composite::COMPOSITE_SPANS;
use riir_refine::draft::eval_fixtures_perf::PERF_SPANS;
use riir_refine::draft::LatentMatcher;

/// clippy 0.2 / rust_perf 1.0 → factors ≈ 0.571 / 1.429 (the
/// `spread_constants_shallow_and_deepen` unit-test pin).
fn spread_policy() -> DepthPolicy {
    DepthPolicy::from_rows(&[
        bench_row("clippy", None, 20, 80, 0, 0),
        bench_row("rust_perf", None, 100, 0, 0, 0),
    ])
}

fn rules_for(by_domain: &[(DomainId, Vec<ScoredRule>)], want: DomainId) -> &[ScoredRule] {
    by_domain
        .iter()
        .find(|(d, _)| *d == want)
        .map_or(&[], |(_, rules)| rules.as_slice())
}

#[test]
fn identity_policy_is_byte_identical() {
    let mut plain = LatentMatcher::from_enabled_domains().expect("matcher builds");
    let mut ident = LatentMatcher::from_enabled_domains()
        .expect("matcher builds")
        .with_depth_policy(DepthPolicy::identity());
    // The attached policy is visible for the posture pin.
    assert!(ident.depth_policy().is_identity());

    for (_, code) in COMPOSITE_SPANS {
        let (routed_a, by_a) = plain.suggest_span(code, 3);
        let (routed_b, by_b) = ident.suggest_span(code, 3);
        assert_eq!(routed_a, routed_b);
        assert_eq!(by_a, by_b, "identity policy must reproduce the default exactly");
    }
    for (_, code) in PERF_SPANS {
        let (routed_a, by_a) = plain.suggest_span(code, 3);
        let (routed_b, by_b) = ident.suggest_span(code, 3);
        assert_eq!(routed_a, routed_b);
        assert_eq!(by_a, by_b);
    }
}

#[test]
fn armed_policy_shallows_the_low_demonstrability_domain() {
    // clippy factor ≈ 0.571: K 6 → round(3.43) = 3 — the served clippy
    // list is capped BELOW the caller's top_k no matter how rich the pool
    // (an upper bound is deterministic; pool richness cannot break it).
    let mut m = LatentMatcher::from_enabled_domains()
        .expect("matcher builds")
        .with_depth_policy(spread_policy());
    let (_, code) = COMPOSITE_SPANS[0];
    let (_, by) = m.suggest_span(code, 6);
    let clippy = rules_for(&by, DomainId::Clippy);
    assert!(
        clippy.len() <= 3,
        "low-demonstrability clippy must shallow to <=3, served {}",
        clippy.len()
    );
}

#[test]
fn armed_policy_deepens_the_high_demonstrability_domain() {
    // rust_perf factor ≈ 1.429: K 2 → round(2.86) = 3. Neutral at K 2
    // serves exactly 2 (a rich pool truncated); armed serves 3.
    let (_, code) = PERF_SPANS[0];

    let mut neutral = LatentMatcher::from_enabled_domains().expect("matcher builds");
    let (_, by_neutral) = neutral.suggest_span(code, 2);
    let perf_neutral = rules_for(&by_neutral, DomainId::Perf);
    assert_eq!(
        perf_neutral.len(),
        2,
        "premise: the frozen perf fixture retrieves >=2 (rich pool)"
    );

    let mut armed = LatentMatcher::from_enabled_domains()
        .expect("matcher builds")
        .with_depth_policy(spread_policy());
    let (_, by_armed) = armed.suggest_span(code, 2);
    let perf_armed = rules_for(&by_armed, DomainId::Perf);
    assert_eq!(
        perf_armed.len(),
        3,
        "high-demonstrability perf must deepen 2 -> 3"
    );
    // NOTE: no ordering pin against the neutral head — a deeper POOL can
    // legitimately surface a new instantiating rule above the old head
    // (that is the point of depth); the gate scales DEPTH, not ranking.
}
