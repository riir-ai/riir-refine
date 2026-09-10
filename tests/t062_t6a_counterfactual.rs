//! Issue 062 **T6a** — the preregistered counterfactual-surgery gate.
//!
//! ```bash
//! CARGO_TARGET_DIR=/tmp/riir_clippy_t6a cargo test --release \
//!   --features tpr_structure,score_bench --test t062_t6a_counterfactual -- --nocapture
//! ```
//!
//! The population is FRESH: the frozen score-bench fixtures re-run at HEAD
//! (real `cargo clippy` per fixture, BLAKE3-keyed temp crates) so the gold
//! "what actually healed" is the healer's behavior NOW, not a history row.
//! The fit corpus is the T4a corpus, at BIC's chosen `d` — the geometry
//! Bench 065 scored.
//!
//! Like the T4a gate this pins **no measured number** — the assertions are
//! instrument-integrity ones only. The verdict is computed and PRINTED; it
//! is deliberately not asserted, because a FAIL is a legitimate
//! preregistered outcome (T6 closes REFUTED) and turning it into a red
//! suite would create the widen-and-retry incentive the gate exists to
//! forbid.

#![cfg(all(
    feature = "tpr_structure",
    feature = "latent_retrieval",
    feature = "score_bench"
))]

use riir_clippy::draft::tpr_t6a_counterfactual::{
    print_t6a, run_t6a, FixtureOutcome, T6A_MARGIN_PP,
};
use riir_clippy::draft::withheld_pair_ood::{clippy_population, Binding};
use riir_clippy::score_bench as sb;

/// The Plan 049 T0.1 committed real-repo dataset — the T4a corpus's real
/// shape-diversity half. Absent → the run proceeds on the hand-authored
/// corpus alone and says so. Same contract as the T4a harness.
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

/// Fresh healer outcomes at HEAD: the score-bench's per-fixture vocabulary
/// sets, reduced to (healed, persisted).
fn fresh_outcomes() -> Vec<FixtureOutcome> {
    let oracle = sb::draft_oracle();
    assert!(
        oracle.is_available(),
        "cargo not on PATH — the gold (what actually healed) cannot be derived"
    );
    let fixture_list = sb::fixtures::fixtures();
    let score = sb::run_bench(&oracle, &fixture_list);
    score
        .outcomes
        .iter()
        .filter(|o| o.ran && !o.vocab_before.is_empty())
        .map(|o| FixtureOutcome {
            id: o.id.clone(),
            code: fixture_list
                .iter()
                .find(|f| f.id == o.id)
                .map_or_else(String::new, |f| f.code.to_string()),
            healed: o
                .vocab_before
                .difference(&o.vocab_persist)
                .cloned()
                .collect(),
            persisted: o.vocab_persist.iter().cloned().collect(),
        })
        .collect()
}

#[test]
fn t062_t6a_counterfactual_surgery_gate() {
    println!(
        "\n=== Issue 062 T6a: counterfactual-surgery gate (T6 kill-switch) ===\n\
         preregistered: PASS = margin >= {T6A_MARGIN_PP:.1} pp over the frequency prior, \
         with a one-flip cushion;\n\
         FAIL = bar not cleared -> T6 REFUTED, no widen-and-retry;\n\
         INCONCLUSIVE = cleared but one flip crosses, or the gate population is empty"
    );

    let outcomes = fresh_outcomes();
    assert!(
        !outcomes.is_empty(),
        "no scored fixtures — nothing to measure"
    );
    let r = run_t6a(clippy_pop(), &outcomes)
        .expect("clippy/tokenbag256 [coalesced] has an admissible d — Bench 063 §12.7 P13");
    print_t6a(&r);

    // ── instrument integrity ────────────────────────────────────────────

    assert!(r.d > 0, "fit ran at d=0");
    assert!(r.n_corpus_states > 0, "empty fit corpus");
    assert!(
        r.m_roles > 1,
        "m'={} — a single role makes the role axis vacuous",
        r.m_roles
    );
    assert!(
        r.n_fillers > 1,
        "n_fillers={} — a single filler makes the ranking vacuous",
        r.n_fillers
    );

    // Every query's gold really is a healer of its fixture, and every
    // counted exclusion names a fixture that exists.
    let ids: std::collections::BTreeSet<&str> = outcomes.iter().map(|o| o.id.as_str()).collect();
    for q in &r.queries {
        let fx = outcomes
            .iter()
            .find(|o| o.id == q.fixture)
            .expect("query names a fixture");
        assert!(
            fx.healed.iter().any(|h| h == &q.gold),
            "query gold {} is not a healed lint of {}",
            q.gold,
            q.fixture
        );
        assert!(
            q.candidates > 0,
            "a gate query with an empty candidate set leaked in"
        );
    }
    for (id, _, _) in &r.exclusions {
        assert!(
            ids.contains(id.as_str()),
            "exclusion names unknown fixture {id}"
        );
    }

    // The populations account: gate fixtures + the two non-gate shapes
    // cover every fixture that had vocabulary lints.
    assert_eq!(
        r.gate_fixtures + r.all_healed_fixtures + r.all_persisted_fixtures,
        outcomes.len(),
        "fixture population does not account"
    );

    // Determinism: a second run is bit-identical (the T2a lesson — the
    // whole report, not just the verdict).
    let r2 = run_t6a(clippy_pop(), &outcomes).expect("second run lost the instrument");
    assert_eq!(r.d, r2.d);
    assert_eq!(r.gate_queries(), r2.gate_queries());
    assert_eq!(r.tpr_top1().to_bits(), r2.tpr_top1().to_bits());
    assert_eq!(r.prior_top1().to_bits(), r2.prior_top1().to_bits());
    assert_eq!(r.margin_pp().to_bits(), r2.margin_pp().to_bits());
    assert_eq!(r.verdict(), r2.verdict());
}
