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

    let mut arms = Vec::new();
    for space in StateSpace::ALL {
        arms.push(run(clippy_pop(), space, "clippy_lints", &DS));
        arms.push(run(rustc_errors_population(), space, "rustc_errors", &DS));
    }

    for r in &arms {
        print_space_report(r);
    }

    // ── instrument integrity ────────────────────────────────────────────
    for r in &arms {
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
    for r in &arms {
        println!(
            "  {:<14} {:<12} admissible_d={:?} structured={:?} probes={:?}",
            r.domain,
            r.space.label(),
            r.admissible_ds(),
            r.structured(),
            r.probe_status
        );
    }
    let consumer: Vec<_> = arms.iter().filter(|r| r.space.is_consumer()).collect();
    let control: Vec<_> = arms.iter().filter(|r| !r.space.is_consumer()).collect();
    let consumer_ok = consumer.iter().any(|r| r.structured() == Some(true));
    let control_ok = control.iter().any(|r| r.structured() == Some(true));
    println!(
        "  consumer admissible anywhere: {}",
        consumer.iter().any(|r| r.any_admissible())
    );
    let verdict = match (consumer_ok, control_ok, control.iter().any(|r| r.any_admissible())) {
        (true, _, _) => "T4 PROCEEDS — the consumer state is admissible AND structured",
        (false, true, _) => {
            "T4 BLOCKED-ON-STATE-WIDTH — control is structured, consumer state too narrow"
        }
        (false, false, true) => "T4 DEMOTED — admissible control carries no binding structure",
        (false, false, false) => "INCONCLUSIVE — no arm admissible; widen the grid or the corpus",
    };
    println!("\nVERDICT: {verdict}");
}
