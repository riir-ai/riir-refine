//! Issue 062 T3 / Bench 063 — TPR structure fit admissibility on the healer
//! corpus.
//!
//! Preregistered in `.benchmarks/063_tpr_structure_admissibility.md`
//! (scheme decision, two-sided admissibility bound, P1–P6 and the four-way
//! verdict rule were committed before this ran).
//!
//! ```bash
//! CARGO_TARGET_DIR=/tmp/riir_clippy_063 cargo test --release \
//!   --features tpr_structure --test t063_tpr_structure -- --nocapture
//! ```
//!
//! Like Bench 062's harness this pins **no measured number**: the assertions
//! are instrument-integrity ones only (determinism, the `param_count`
//! cross-check, the structured-implies-degraded consistency check, and the
//! refusal to read a probe off an inadmissible fit). Pinning a residual here
//! would convert an honest re-measurement into a red suite.

#![cfg(all(feature = "tpr_structure", feature = "latent_retrieval"))]

use riir_clippy::draft::tpr_fit::{ProbeStatus, StateSpace, print_space_report, run};
use riir_clippy::draft::withheld_pair_ood::{
    Binding, clippy_population, rustc_errors_population,
};

/// The Plan 049 T0.1 committed real-repo dataset — the only source of genuine
/// shape diversity in the clippy population (Bench 062 §10). Absent → the run
/// proceeds on the hand-authored corpus alone and says so, because a missing
/// fixture must not read as a corpus finding.
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

/// `d` grid. Powers of two up to the 8-D consumer state's width — beyond that
/// every arm is inadmissible on the core-width bound by construction, so a
/// wider grid would only add rows that cannot be read.
const DS: [usize; 4] = [1, 2, 4, 8];

#[test]
fn t063_tpr_structure_admissibility() {
    println!(
        "\n=== Bench 063: TPR structure fit admissibility (Issue 062 T3) ===\n\
         two-sided bound: params < n_states*D, every role has > d states, and K < D\n\
         scheme: Orthogonal{{arity=m}}, role = shape-class, filler = rule identity"
    );

    // §10.2a: ONE fixed threshold at the grid maximum, so BIC compares `d` at
    // a constant `N`. Per-`d` coalescing would change `n_states` and make the
    // BIC scores incomparable.
    let threshold = DS.iter().copied().max().expect("grid non-empty");

    let mut arms = Vec::new();
    for space in StateSpace::ALL {
        arms.push(run(clippy_pop(), space, "clippy_lints", &DS, None));
        arms.push(run(rustc_errors_population(), space, "rustc_errors", &DS, None));
    }
    let mut coalesced = Vec::new();
    for space in StateSpace::ALL {
        coalesced.push(run(clippy_pop(), space, "clippy_lints", &DS, Some(threshold)));
        coalesced.push(run(
            rustc_errors_population(),
            space,
            "rustc_errors",
            &DS,
            Some(threshold),
        ));
    }

    for r in &arms {
        print_space_report(r);
    }
    println!("\n=== §11 addendum: coalesced arms (T={threshold}) ===");
    for r in &coalesced {
        print_space_report(r);
    }

    // P11: the coalescing path is additive, so the uncoalesced arms must be
    // untouched by it. Asserted rather than eyeballed — a shift would be
    // invisible in a table read one arm at a time.
    for r in &arms {
        assert!(
            r.coalesce.is_none(),
            "{}/{}: uncoalesced arm carries a coalesce report",
            r.domain,
            r.space.label()
        );
    }

    // §10.4's guard was REFUTED by this run and is replaced by the invariant it
    // should have been. It asserted that coalescing can never make the consumer
    // space admissible "because it only shrinks m" — but shrinking m is exactly
    // what takes `K = m'*d` BELOW `D`: clippy coalesces to m'=6, so `K = 6 < 8`
    // at d=1 IS admissible. The prediction assumed m stayed 8.
    //
    // What must actually hold is the bound itself, on every arm: admissible
    // implies all three conditions. Asserting that catches a bound that has
    // drifted from the verdict it decides, which is what the guard was reaching
    // for.
    for r in arms.iter().chain(coalesced.iter()) {
        for g in &r.grid {
            let a = &g.adm;
            if a.admissible() {
                assert!(
                    a.k < a.dim,
                    "{}/{} d={}: ADMISSIBLE with K={} >= D={}",
                    r.domain,
                    r.space.label(),
                    a.d,
                    a.k,
                    a.dim
                );
                assert!(
                    a.params < a.observations,
                    "{}/{} d={}: ADMISSIBLE with params {} >= obs {}",
                    r.domain,
                    r.space.label(),
                    a.d,
                    a.params,
                    a.observations
                );
                assert!(
                    a.starved_roles.is_empty(),
                    "{}/{} d={}: ADMISSIBLE with {} starved role(s)",
                    r.domain,
                    r.space.label(),
                    a.d,
                    a.starved_roles.len()
                );
            }
        }
    }

    // ── instrument integrity ────────────────────────────────────────────
    for r in arms.iter().chain(coalesced.iter()) {
        let tag = format!("{}/{}", r.domain, r.space.label());
        assert!(r.n_states > 0, "{tag}: empty population");
        assert!(
            r.n_roles > 1,
            "{tag}: {} role(s) — a single role makes the role axis vacuous",
            r.n_roles
        );

        // Determinism (upstream G1) re-checked at this consumer, on every
        // grid point whose fit ran — including inadmissible ones, because a
        // nondeterministic fit is a defect regardless of admissibility.
        for g in &r.grid {
            if let Some(rep) = &g.report {
                assert_eq!(
                    g.double_fit_identical,
                    Some(true),
                    "{tag} d={}: double fit not bit-identical",
                    g.adm.d
                );
                // NOT `monotone_violations == 0`. Upstream's contract is that
                // an uphill sweep is REJECTED and rolled back, so a violation
                // is the descent guard working, not a defect — and it does
                // occur here (clippy/latent8 d=1, where K == D makes the fit
                // exactly determined and f32 noise trips the guard). What must
                // hold is the rollback certificate: the reported objective is
                // the best sweep's, never a post-violation uphill one.
                let best = rep
                    .ssr_per_sweep
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    rep.final_ssr <= best + best.abs() * 1e-9,
                    "{tag} d={}: rollback certificate violated — final_ssr {} > best sweep {} \
                     ({} rejected sweep(s))",
                    g.adm.d,
                    rep.final_ssr,
                    best,
                    rep.monotone_violations
                );
                assert!(
                    rep.residual_energy_fraction.is_finite(),
                    "{tag} d={}: non-finite residual",
                    g.adm.d
                );
            }
        }

        // A probe verdict may only exist on an admissible fit.
        assert_eq!(
            r.bow.is_some(),
            r.chosen_d.is_some(),
            "{tag}: probe/admissibility disagreement"
        );
        if r.bow.is_some() {
            assert!(
                r.any_admissible(),
                "{tag}: probes ran with no admissible d"
            );
        }

        // The upstream per-state control is a provable identity here (one
        // binding per state ⇒ `for i in (1..1).rev()` never runs), so it can
        // only ever report `degraded = false`. Assert the vacuity rather than
        // reading the verdict — and assert it is EXACTLY an identity, which is
        // what makes the reading unsafe.
        if r.probe_status == ProbeStatus::Ran {
            assert!(
                r.shuffled_vacuous,
                "{tag}: the per-state shuffle stopped being vacuous — this \
                 corpus grew multi-binding states, so re-enable it as a real control"
            );
            let sh = r.shuffled.as_ref().expect("probes ran");
            assert!(
                (sh.r_shuffled - sh.r_true).abs() <= f32::EPSILON,
                "{tag}: per-state shuffle claimed to be vacuous but moved the \
                 residual ({} vs {}) — the vacuity predicate is wrong",
                sh.r_shuffled,
                sh.r_true
            );

            // The replacement control must actually permute something. A
            // control that cannot move is the failure mode it exists to fix.
            let x = r.cross_state.as_ref().expect("probes ran");
            assert!(
                x.moved > 0,
                "{tag}: cross-state permutation moved 0 of {} roles — identity \
                 draw, so its verdict is vacuous too",
                r.n_states
            );
        }
    }

    // ── the preregistered verdict, printed not asserted ─────────────────
    println!("\n=== §6 verdict inputs ===");
    for r in arms.iter().chain(coalesced.iter()) {
        println!(
            "  {:<14} {:<12} admissible_d={:?} structured={:?} probes={:?}",
            r.domain,
            r.space.label(),
            r.admissible_ds(),
            r.structured(),
            r.probe_status
        );
    }
    // Both arm sets are evidence. Scoring only the uncoalesced set was how the
    // first cut printed BLOCKED-ON-STATE-WIDTH while an admissible coalesced
    // consumer arm sat two tables above it.
    let all: Vec<&_> = arms.iter().chain(coalesced.iter()).collect();
    let consumer: Vec<&_> = all
        .iter()
        .copied()
        .filter(|r| r.space.is_consumer())
        .collect();
    let control: Vec<&_> = all
        .iter()
        .copied()
        .filter(|r| !r.space.is_consumer())
        .collect();
    let consumer_ok = consumer.iter().any(|r| r.structured() == Some(true));
    let control_ok = control.iter().any(|r| r.structured() == Some(true));
    let consumer_admissible = consumer.iter().any(|r| r.any_admissible());
    let consumer_measured_flat = consumer.iter().any(|r| r.structured() == Some(false));
    println!("  consumer admissible anywhere: {consumer_admissible}");
    println!("  consumer measured flat:       {consumer_measured_flat}");

    // §6 had four branches. The run produced a fifth: a consumer arm that is
    // ADMISSIBLE and measures NOT structured. That is strictly stronger
    // evidence for the same consequence as branch 2 — the 8-D state does not
    // carry the structure — because it is MEASURED in the consumer space rather
    // than inferred from that space being unmeasurable. It gets its own name
    // instead of being folded into branch 2.
    let verdict = if consumer_ok {
        "T4 PROCEEDS — the consumer state is admissible AND structured"
    } else if consumer_admissible && consumer_measured_flat && control_ok {
        "T4 BLOCKED-ON-STATE-CONTENT (§6 branch 5, unanticipated) — the consumer state \
         IS measurable and carries NO role-filler structure, while the wider control \
         does. Stronger than branch 2: measured, not inferred."
    } else if consumer_admissible && consumer_measured_flat {
        "T4 DEMOTED — the consumer state is measurable and flat, and no control shows \
         structure either"
    } else if control_ok {
        "T4 BLOCKED-ON-STATE-WIDTH (§6 branch 2) — control structured, consumer state \
         never admissible"
    } else {
        "INCONCLUSIVE — no structured arm; widen the grid or the corpus"
    };
    println!("\nVERDICT: {verdict}");
}
