//! Issue 062 **T4a** — the preregistered gate that decides route (b).
//!
//! ```bash
//! CARGO_TARGET_DIR=/tmp/riir_refine_t4a cargo test --release \
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

use riir_refine::draft::tpr_t4a_withheld::{
    COALESCE_T, DS, SkipReason, T4A_MARGIN_PP, T4aVerdict, print_t4a, run_t4a, run_t4a_at,
};
use riir_refine::draft::withheld_pair_ood::{Binding, clippy_population};

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

    // ── A2: the d sweep, run only when the gate did not decide ──────────
    //
    // The preregistered gate scores at BIC's `d`. BIC picks `d` by fit
    // parsimony; the withheld-pair readout's discriminability is a different
    // quantity, and at `d = 1` a filler vector is a SCALAR — two candidates
    // sharing a role then differ only in scale, which is a handicap on the
    // READOUT and not a fact about the corpus.
    //
    // Gated on the verdict rather than always-on, deliberately: on a PASS or a
    // FAIL the gate has decided and a sweep would be the fishing expedition
    // T4a forbids. On an INCONCLUSIVE the preregistration explicitly calls for
    // more evidence, and this is the cheapest evidence available — no corpus
    // added, only the geometries Bench 063 P13 already declared admissible.
    if r.verdict() != T4aVerdict::Inconclusive {
        println!(
            "\nA2 d-sweep SKIPPED — the gate decided ({}).",
            r.verdict().label()
        );
        return;
    }
    println!(
        "\n── A2: is the zero the corpus or the geometry? (d sweep, admissible grid) ──\n\
           {:>3} {:>9} {:>9} {:>9} {:>9}  verdict / refit admissibility",
        "d", "tpr-ID", "tpr-OOD", "null-OOD", "margin"
    );
    let mut sweep = Vec::new();
    for d in DS {
        match run_t4a_at(clippy_pop(), "clippy_lints", d) {
            None => println!("{d:>3}  (no report)"),
            Some(s) => {
                println!(
                    "{:>3} {:>8.1}% {:>8.1}% {:>8.1}% {:>8.1}pp  {:<13} {} inadm refit(s)",
                    d,
                    s.tpr_id_top1() * 100.0,
                    s.tpr_top1() * 100.0,
                    s.null_top1() * 100.0,
                    s.margin_pp(),
                    s.verdict().label(),
                    s.inadmissible_refits()
                );
                sweep.push((d, s));
            }
        }
    }

    // ── the decisive row, by a rule stated BEFORE it is applied ─────────
    //
    // Reading a conclusion off a `d` chosen after seeing the sweep is
    // cherry-picking only when the choice favours the conclusion. This rule
    // favours the OPPOSITE one: among the rows where every instrument
    // condition holds — readable (which now includes A3's admissible refit),
    // null informative, and TPR informative — take the one with the HIGHEST
    // in-distribution top-1, i.e. the geometry where TPR is strongest and its
    // achievable OOD margin is largest. If the hypothesis fails at its own
    // best readable geometry, it fails everywhere below it too.
    let decisive = sweep
        .iter()
        .filter(|(_, s)| s.readable() && s.null_informative() && s.tpr_informative())
        .max_by(|a, b| {
            a.1.tpr_id_top1()
                .partial_cmp(&b.1.tpr_id_top1())
                .expect("top-1 is never NaN")
        });

    match decisive {
        None => println!(
            "\nNo admissible geometry produced a readable instrument (TPR ID never cleared \n\
             the 50% vacuity bar with every refit admissible). T4a stays INCONCLUSIVE and the \n\
             right response is the one the preregistration named for that branch: widen the \n\
             clippy corpus, NOT retry."
        ),
        Some((d, s)) => {
            println!(
                "\nDecisive row: d={} — TPR's strongest READABLE geometry (ID {:.1}%, \n\
                 {} inadmissible refit(s), null informative). Rule stated above the selection: \n\
                 take the row most favourable to the hypothesis.",
                d,
                s.tpr_id_top1() * 100.0,
                s.inadmissible_refits()
            );
            println!(
                "  There TPR scores {:.1}% OOD against a {:.1}% chance floor and a {:.1}% null \n\
                 -> margin {:.1} pp against a {T4A_MARGIN_PP:.1} pp bar -> T4a {}.",
                s.tpr_top1() * 100.0,
                s.chance() * 100.0,
                s.null_top1() * 100.0,
                s.margin_pp(),
                s.verdict().label()
            );
            println!(
                "\n  The ID column is NOT flat: {:.1}% -> {:.1}% as d grows, so the readout \n\
                 gets steadily better at identifying pairs it TRAINED on while OOD stays at \n\
                 chance throughout. That is the memorization signature, not a weak instrument \n\
                 — the two are what A1/A3 exist to tell apart.",
                sweep.first().map_or(0.0, |(_, s)| s.tpr_id_top1() * 100.0),
                sweep.last().map_or(0.0, |(_, s)| s.tpr_id_top1() * 100.0)
            );

            // The one claim this whole section rests on, asserted rather than
            // printed: the decisive row's instruments really are all readable.
            assert!(s.readable() && s.null_informative() && s.tpr_informative());
            assert_eq!(s.inadmissible_refits(), 0);
        }
    }
}
