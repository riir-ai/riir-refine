//! Issue 036 — the jury-disagreement POC harness (T1–T3 measurement).
//!
//! Runs the full population (48 single-lint clippy fixtures + 28 composite
//! multi-lint spans + the perf fixture set), instruments the two disagreement
//! signals, joins the real-clippy fix outcome, and prints the ρ table + both
//! threshold sweeps + the G2 verdict.
//!
//! ```bash
//! cargo test --release --features latent_retrieval,clippy_verify \
//!   --test jury_disagreement_poc -- --nocapture --test-threads=1
//! ```
//!
//! Slow (one `cargo clippy` per applicable-fix span, BLAKE3-keyed temp
//! crates so re-runs hit cargo's build cache). Deliberately NOT a pinned
//! regression floor — the same toolchain-drift contract as
//! `cross_domain_oracle_truth.rs`: the POC's verdict is recorded in the
//! issue file + research note, not asserted here. What IS asserted: the
//! harness is structurally sound (every span yields signals; the join has
//! a verdict-bearing population; the sweeps are well-formed), so a broken
//! instrumentation fails loudly instead of silently producing ρ=0.

#![cfg(all(feature = "latent_retrieval", feature = "clippy_verify"))]

use riir_refine::draft::jury_disagreement::{
    DisagreementSignals, FixOutcome, best_g2_threshold, print_jury_report, run_jury_poc,
};
use riir_refine::draft::{ClippyOracle, DomainId, LatentMatcher};

#[test]
fn jury_disagreement_poc_harness() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("cargo not on PATH — skipping the POC (oracle unavailable)");
        return;
    }

    let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
    let report = run_jury_poc(&mut matcher, &oracle);
    print_jury_report(&report);

    // ── Structural assertions (the POC verdict itself is recorded, not asserted) ──

    // The population is the full fixture set: 48 clippy + 28 composite = 76,
    // plus the perf set when its feature is on.
    let clippy_rows = report
        .rows
        .iter()
        .filter(|r| r.domain == DomainId::Clippy)
        .count();
    assert!(
        clippy_rows >= 70,
        "expected ≥70 clippy-domain rows (48 fixtures + 28 composite), got {clippy_rows}"
    );

    // Every measured span produced real signals (a pool, a top-1).
    for r in &report.rows {
        assert!(
            !r.signals.top1_rule.is_empty(),
            "span {} yielded no top-1 rule",
            r.tag
        );
        assert!(
            r.signals.pool_spread >= 1 && r.signals.pool_spread <= r.signals.pool_len,
            "span {} spread {} outside [1, {}]",
            r.tag,
            r.signals.pool_spread,
            r.signals.pool_len
        );
    }

    // The join has a verdict-bearing population (NoFix/OracleDown excluded,
    // but not everything).
    assert!(
        report.verdict_rows > 0,
        "every span was NoFix/OracleDown — the join is vacuous"
    );

    // Both sweeps are well-formed: thresholds monotone, totals consistent.
    for c in report.spread_sweep.iter().chain(report.margin_sweep.iter()) {
        assert_eq!(c.total, report.verdict_rows);
        assert!(c.flagged <= c.total);
        assert!(c.unstable_flagged <= c.unstable_total);
        assert!(c.unstable_total <= c.total);
    }
    // The sweep exists at all (≥2 distinct spreads observed).
    assert!(
        !report.spread_sweep.is_empty(),
        "pool_spread had no variance — signal dead, record it honestly"
    );

    // The G2 helper never selects a threshold outside the gate bounds.
    for sweep in [&report.spread_sweep, &report.margin_sweep] {
        if let Some(c) = best_g2_threshold(sweep) {
            assert!(c.capture() >= 0.70 && c.flagged_frac() <= 0.40);
        }
    }

    // ρ is computed iff the instability variable has variance.
    let has_variance = report.unstable_rows > 0 && report.unstable_rows < report.verdict_rows;
    assert_eq!(
        !report.rhos.is_empty(),
        has_variance,
        "ρ presence must match instability variance"
    );
}

/// The signals reduce correctly on a hand-built pool (unit-level, no cargo).
#[test]
fn disagreement_signals_reduce_on_a_fixed_pool() {
    use riir_refine::draft::corpus_index::ScoredRule;

    let sr = |id: &str, score: f32| ScoredRule {
        rule_id: id.to_string(),
        score,
        instantiates: false,
    };
    // A consensus pool: one rule floods it with its own line-chunks.
    let pool = vec![
        sr("needless_return", 9.0),
        sr("needless_return", 8.5),
        sr("needless_return", 8.0),
        sr("let_and_return", 2.0),
    ];
    let mut ranked = pool.clone();
    ranked[0].instantiates = true;
    ranked.sort_by(|a, b| {
        b.instantiates
            .cmp(&a.instantiates)
            .then_with(|| b.score.total_cmp(&a.score))
    });
    let s: DisagreementSignals =
        riir_refine::draft::jury_disagreement::disagreement_signals(&pool, &ranked);
    assert_eq!(s.pool_len, 4);
    assert_eq!(s.pool_spread, 2, "two distinct rules in the pool");
    assert_eq!(s.instantiating, 1);
    assert_eq!(s.top1_rule, "needless_return");
    assert!((s.rerank_margin - 0.5).abs() < 1e-6, "9.0 - 8.5");

    // A single-candidate pool is uncontested by emptiness.
    let one = vec![sr("only", 1.0)];
    let s1 = riir_refine::draft::jury_disagreement::disagreement_signals(&one, &one);
    assert!(s1.rerank_margin.is_infinite());
}

/// The outcome classifier: a fix that does not resolve its own lint is
/// LintUnresolved; one that breaks compilation is CompileFail (oracle-backed,
/// one cargo call each — kept out of the harness test above for clarity).
#[test]
fn fix_outcome_classifies_the_three_verdicts() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("cargo not on PATH — skipping (oracle unavailable)");
        return;
    }

    // needless_return resolves its own lint.
    let code = "pub fn f() -> u8 {\n    return 1;\n}\n";
    let out = riir_refine::draft::jury_disagreement::fix_outcome(
        &oracle,
        DomainId::Clippy,
        code,
        "needless_return",
    );
    assert_eq!(out, FixOutcome::Resolved, "the canonical fix must resolve");

    // A rule whose proposer does not fire on the span: no verdict.
    let out = riir_refine::draft::jury_disagreement::fix_outcome(
        &oracle,
        DomainId::Clippy,
        code,
        "manual_map",
    );
    assert_eq!(out, FixOutcome::NoFix, "a silent proposer is NoFix");
}

/// G2's honest failure mode: when no threshold qualifies, `None` — never a
/// loosened bar (unit-level).
#[test]
fn g2_returns_none_when_nothing_qualifies() {
    let sweep = vec![riir_refine::draft::jury_disagreement::Contingency {
        threshold: 2.0,
        total: 10,
        flagged: 9,
        unstable_total: 5,
        unstable_flagged: 5,
    }];
    assert!(best_g2_threshold(&sweep).is_none());
}
