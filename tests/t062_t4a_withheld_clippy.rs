//! Issue 062 **T4a** — the preregistered gate that decides route (b).
//!
//! ```bash
//! CARGO_TARGET_DIR=/tmp/riir_clippy_t4a cargo test --release \
//!   --features tpr_structure --test t062_t4a_withheld_clippy -- --nocapture
//! ```
//!
//! Like Bench 062's and 063's harnesses this pins **no measured number** — the
//! assertions are instrument-integrity ones only. The verdict is computed and
//! PRINTED; it is deliberately not asserted, because a FAIL is a legitimate
//! preregistered outcome (route (b) closes REFUTED) and turning it into a red
//! suite would create the incentive to widen and retry that T4a exists to
//! forbid.
//!
//! What IS asserted: the gate scored the arm it claims to have scored, at the
//! geometry the bench chose, on a pool that can answer, with a verdict that
//! cannot be reached by the two ways this measurement fails silently.

#![cfg(all(feature = "tpr_structure", feature = "latent_retrieval"))]

use riir_clippy::draft::tpr_t4a_withheld::{
    print_t4a, run_t4a, SkipReason, T4aVerdict, COALESCE_T, DS, T4A_MARGIN_PP,
};
use riir_clippy::draft::withheld_pair_ood::{clippy_population, Binding};

/// The Plan 049 T0.1 committed real-repo dataset — the only source of genuine
/// shape diversity in the clippy population (Bench 062 §10). Absent → the run
/// proceeds on the hand-authored corpus alone and says so, because a missing
/// fixture must not read as a corpus finding. Same contract as `t063`.
const REAL_DATASET: &str = ".benchmarks/data/049_t01/spans.jsonl";

#[derive(serde::Deserialize)]
struct Row {
    code: String,
    label: Option<String>,
}

fn real_repo_bindings() -> Vec<Binding> {
    let Ok(bytes) = std::fs::read(REAL_DATASET) else {
        eprintln!("NOTE: {REAL_DATASET} missing — running without the real-repo bindings");
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    text.lines()
        .filter_map(|l| serde_json::from_str::<Row>(l).ok())
        .filter_map(|r| r.label.map(|lab| Binding::clippy(lab, r.code, "real")))
        .collect()
}

fn clippy_pop() -> Vec<Binding> {
    let mut p = clippy_population();
    p.extend(real_repo_bindings());
    p
}

#[test]
fn t062_t4a_withheld_pair_gate() {
    println!(
        "\n=== Issue 062 T4a: withheld-(role, filler)-pair gate (route (b) kill-switch) ===\n\
         preregistered: PASS = every trial readable + null informative + margin >= {T4A_MARGIN_PP:.1} pp\n\
         with a one-flip cushion; FAIL = bar not cleared -> T4 REFUTED, no widen-and-retry;\n\
         INCONCLUSIVE = cleared but one flip crosses, or an instrument was unreadable"
    );

    let r = run_t4a(clippy_pop(), "clippy_lints")
        .expect("clippy/tokenbag256 [coalesced] has an admissible d — Bench 063 §12.7 P13");
    print_t4a(&r);

    // ── instrument integrity ────────────────────────────────────────────

    // The arm. Bench 063 §12.9's rule: a row travels, a heading does not — so
    // the gate must be able to say which corpus it scored, not inherit it.
    assert_eq!(r.domain, "clippy_lints");
    assert_eq!(r.coalesce_threshold, COALESCE_T);
    assert!(
        DS.contains(&r.d),
        "d={} is outside the declared grid {DS:?}",
        r.d
    );
    assert!(r.n_states > 0, "empty population");
    assert!(
        r.m > 1,
        "m'={} — a single role makes the role axis vacuous",
        r.m
    );

    // The pool is the shared one and every truth pair is in it, so `coverage`
    // is a real ceiling rather than an artifact of a pool built per trial.
    assert_eq!(
        r.pool, r.spread.distinct_pairs,
        "the shared pool must BE the observed-pair set: pool {} vs distinct_pairs {}",
        r.pool, r.spread.distinct_pairs
    );
    assert!(
        r.pool <= r.pair_space,
        "pool {} exceeds the {}-pair space",
        r.pool,
        r.pair_space
    );

    // The three populations partition the corpus's pairs. Asserted because
    // reporting them apart is worthless if they do not add up — a pair lost
    // between the loops would shrink the validated set silently.
    assert_eq!(
        r.trials.len() + r.unvalidated(),
        r.pool,
        "validated {} + unvalidated {} != {} corpus pairs",
        r.trials.len(),
        r.unvalidated(),
        r.pool
    );
    assert_eq!(
        r.outside_fit() + r.pool,
        r.pair_space,
        "outside-the-fit + observed != pair space"
    );

    // Testability is the covariate, not an accident of the loop: every trial
    // must come from a multi-role filler, and the count of trials cannot
    // exceed what the corpus can host.
    assert!(
        r.trials.len() <= r.spread.distinct_pairs,
        "more trials than distinct pairs"
    );
    let single_role_skips = r
        .skipped
        .iter()
        .filter(|(_, _, why)| *why == SkipReason::SingleRoleFiller)
        .count();
    assert_eq!(
        r.spread.fillers - r.spread.multi_role_fillers,
        single_role_skips,
        "every single-role filler contributes exactly one pair, so the skip count must \
         equal fillers - multi_role_fillers ({} - {})",
        r.spread.fillers,
        r.spread.multi_role_fillers
    );

    // Each trial partitions the corpus: held + train == n_states, with a
    // non-empty held set. A trial that held nothing would score 0 states and
    // silently dilute the aggregate toward the null.
    for t in &r.trials {
        assert!(t.held > 0, "{}/{}: empty held set", t.role, t.filler);
        assert_eq!(
            t.held + t.train,
            r.n_states,
            "{}/{}: held {} + train {} != {} states",
            t.role,
            t.filler,
            t.held,
            t.train,
            r.n_states
        );
        assert!(
            t.report.top1 <= t.report.coverage + 1e-6,
            "{}/{}: top1 {} exceeds its ceiling {}",
            t.role,
            t.filler,
            t.report.top1,
            t.report.coverage
        );
    }

    // The aggregate is state-weighted, and re-derived here from the trials so
    // a change to the accessor cannot drift from what the table shows.
    let scored: usize = r.trials.iter().map(|t| t.held).sum();
    assert_eq!(r.scored(), scored);
    assert!(
        scored > 0,
        "no state was scored — the gate measured nothing"
    );

    // Two-sided on the verdict machinery: PASS is unreachable when either
    // instrument is unreadable, and that is the property that stops a
    // vacuous null from certifying route (b). Checked as an implication on
    // the measured report, so it is a live assertion rather than a unit test
    // of a constant.
    if !r.readable() || !r.null_informative() {
        assert_eq!(
            r.verdict(),
            T4aVerdict::Inconclusive,
            "an unreadable instrument must not produce PASS or FAIL"
        );
    }
    if r.verdict() == T4aVerdict::Pass {
        assert!(r.readable() && r.null_informative());
        assert!(r.margin_pp() - r.one_flip_pp() >= T4A_MARGIN_PP);
    }

    println!(
        "\nT4a verdict: {} — margin {:.1} pp over {} scored state(s), \
         one flip worth {:.1} pp",
        r.verdict().label(),
        r.margin_pp(),
        scored,
        r.one_flip_pp()
    );
}
