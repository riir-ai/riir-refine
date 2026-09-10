//! Issue 035 T2 + Plan 019 Phase 6 — the G-floor + GOAT measurement runner.
//!
//! ```bash
//! # everything (the full Phase 6 measurement session, ~30-40 min):
//! cargo run --release --example rustc_errors_floor --features rustc_errors
//!
//! # one section at a time (chunked runs; sections: probe floor driver null ab):
//! cargo run --release --example rustc_errors_floor --features rustc_errors -- --only driver
//! cargo run --release --example rustc_errors_floor --features "rustc_errors,latent_retrieval" -- --only ab
//! ```
//!
//! Sections (each prints its own table; all measured against the live
//! toolchain):
//!
//! 1. **probe** — per error code, does rustc ship a machine-applicable
//!    suggestion, and does `cargo fix --broken-code` apply it? The
//!    contested/uncontested split that bounds what `rustc_errors` can claim.
//! 2. **floor** — `cargo fix --broken-code` over the FULL known-inverse
//!    corpus (the 12 committed anchors + the N≥200 generated cases):
//!    per-code fix rate AND, for every landed repair, whether it restored
//!    the original meaning or diverged.
//! 3. **driver** — the production `--fix-compile` arm over the same corpus
//!    (verbose: per-iteration timings), plus the G1 semantics pass
//!    (`cargo test` post-fix on every repaired case — the corpus's
//!    ground-truth behavior tests) and the G2 latency table (p50/p99).
//! 4. **null** — the no-op arm self-check: must score exactly 0% on the
//!    full corpus (corpus integrity at scale).
//! 5. **ab** — the G6/G7 ordering A/B (needs `latent_retrieval`): a
//!    learning pass warms a temp trajectory store, then static min-delta
//!    vs flat `(code,strategy)` BetaPosterior vs shape-conditional
//!    ordering are compared on candidates-tried + reverts over the
//!    template-covered stratum.

use riir_clippy::domains::rustc_errors::corpus_gen::{generate_corpus, Territory};
use riir_clippy::domains::rustc_errors::floor::{
    CargoFixBaseline, DriverFixer, FixVerdict, FloorHarness, FloorOutcome, FloorReport, NullFixer,
    MUTANT_CASES,
};
use riir_clippy::domains::rustc_errors::probe::PremiseProbe;

fn main() {
    let only: Vec<String> = std::env::args()
        .skip_while(|a| a != "--only")
        .nth(1)
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let want = |s: &str| only.is_empty() || only.iter().any(|x| x == s);

    println!("=== Plan 019 Phase 6 — premise probe + G-floor + GOAT ===\n");
    println!("rustc: {}", rustc_version());

    if want("probe") {
        probe_section();
    }
    if want("floor") {
        floor_section();
    }
    if want("driver") {
        driver_section();
    }
    if want("null") {
        null_section();
    }
    if want("seeds") {
        seeds_section();
    }
    if want("ab") {
        #[cfg(feature = "latent_retrieval")]
        ordering_ab_section();
        #[cfg(not(feature = "latent_retrieval"))]
        println!("\n── 5. SKIP ab: needs --features latent_retrieval ──");
    }
}

fn case_count() -> usize {
    MUTANT_CASES.len() + generate_corpus().len()
}

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn probe_section() {
    println!("\n── 1. Premise probe: does rustc already fix this? ──\n");
    let probe = PremiseProbe::new();
    if !probe.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    println!(
        "  {:<30} {:>6}  {:>5} {:>5}  {:>9}  {:<12} replacement",
        "case", "code", "sugg", "auto", "fix-moved", "territory"
    );
    for result in probe.probe_all() {
        match result {
            Err(e) => println!("  ERROR: {e}"),
            Ok(o) => {
                let territory = match o.is_uncontested() {
                    true => "UNCONTESTED",
                    false => "contested",
                };
                let repl = match o.replacements.is_empty() {
                    true => "-".to_string(),
                    false => format!("{:?}", o.replacements),
                };
                let mislabel = match o.code_as_declared() {
                    true => "",
                    false => "  << MIS-LABELED",
                };
                println!(
                    "  {:<30} {:>6}  {:>5} {:>5}  {:>9}  {:<12} {repl}{mislabel}",
                    o.name,
                    o.expect_code,
                    o.suggestion_spans,
                    o.machine_applicable,
                    o.cargo_fix_changed,
                    territory
                );
            }
        }
    }
    probe.cleanup();
}

/// One measurement arm over the full corpus — shared scaffold/floor logic
/// for the floor + driver sections (the null arm reuses it too).
fn measure_full(
    harness: &FloorHarness,
    label: &str,
    measure: impl Fn(&FloorHarness) -> Result<FloorReport, String>,
) -> Option<FloorReport> {
    println!(
        "\n── {label}: over {n} known-inverse mutants (12 anchors + {gen} generated) ──\n",
        n = case_count(),
        gen = case_count() - MUTANT_CASES.len()
    );
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return None;
    }
    let report = match measure(harness) {
        Ok(r) => r,
        Err(e) => {
            println!("  ERROR: {e}");
            return None;
        }
    };
    print_report(&report);
    print_integrity(&report);
    Some(report)
}

fn floor_section() {
    let harness = FloorHarness::new();
    let report = measure_full(&harness, "2. G-floor: `cargo fix --broken-code`", |h| {
        h.measure_cases(&CargoFixBaseline::new(), &floor_case_list())
    });
    if let Some(report) = report {
        let diverged: Vec<&str> = report
            .outcomes
            .iter()
            .filter(|o| o.verdict == FixVerdict::Diverged)
            .map(|o| o.name.as_str())
            .collect();
        if !diverged.is_empty() {
            println!(
                "\n  {} repair(s) compiled but DIVERGED from the known inverse — the \
                 review surface a ranked-alternatives fixer owns.",
                diverged.len()
            );
        }
    }
    harness.cleanup();
}

fn driver_section() {
    let harness = FloorHarness::new();
    let fixer = DriverFixer::new();
    let cases = floor_case_list();
    println!(
        "\n── 3. The `--fix-compile` arm (verbose): over {n} known-inverse mutants ──\n",
        n = cases.len()
    );
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    let (report, driver_reports) = match harness.measure_driver(&fixer, &cases) {
        Ok(r) => r,
        Err(e) => {
            println!("  ERROR: {e}");
            return;
        }
    };
    print_report(&report);
    print_integrity(&report);

    // G1 semantics pass: every repaired case's scaffold holds the final
    // source; run its ground-truth behavior test.
    println!("\n  G1 semantics pass: `cargo test` post-fix on every repaired case");
    let mut tested = 0usize;
    let mut green = 0usize;
    let mut red: Vec<String> = Vec::new();
    for o in &report.outcomes {
        if o.verdict == FixVerdict::Unfixed {
            continue;
        }
        let dir = harness.work_root().join(&o.name);
        match harness.cargo_test_passes(&dir) {
            Err(e) => println!("    {} cargo-test error: {e}", o.name),
            Ok(false) => red.push(o.name.clone()),
            Ok(true) => green += 1,
        }
        tested += 1;
    }
    println!(
        "    {green}/{tested} repaired cases keep the seed's behavior; {} broke it{}",
        red.len(),
        match red.is_empty() {
            true => String::new(),
            false => format!(": {}", red.join(", ")),
        }
    );

    // G2 latency: per-case fixer wall time + per-iteration driver time.
    let mut fix_ms: Vec<f64> = report.outcomes.iter().map(|o| o.fix_ms).collect();
    fix_ms.sort_by(|a, b| a.total_cmp(b));
    if !fix_ms.is_empty() {
        println!(
            "\n  G2 latency, per CASE (ms): p50 {:.0}  p99 {:.0}  max {:.0}",
            percentile(&fix_ms, 0.50),
            percentile(&fix_ms, 0.99),
            fix_ms[fix_ms.len() - 1]
        );
    }
    let mut iter_ms: Vec<f64> = driver_reports
        .iter()
        .flat_map(|(_, r)| r.iter_ms.iter().copied())
        .collect();
    iter_ms.sort_by(|a, b| a.total_cmp(b));
    if !iter_ms.is_empty() {
        println!(
            "  G2 latency, per ITERATION (ms): n {}  p50 {:.0}  p99 {:.0}  max {:.0}",
            iter_ms.len(),
            percentile(&iter_ms, 0.50),
            percentile(&iter_ms, 0.99),
            iter_ms[iter_ms.len() - 1]
        );
    }
    harness.cleanup();
}

fn null_section() {
    let harness = FloorHarness::new();
    let measured = measure_full(&harness, "4. Harness self-check: the no-op arm", |h| {
        h.measure_cases(&NullFixer, &floor_case_list())
    });
    if let Some(report) = measured {
        let rate = report.total_fix_rate();
        println!(
            "  no-op arm fix rate: {:.1}% — {}",
            rate * 100.0,
            match rate == 0.0 {
                true => "PASS (every mutant genuinely broken)",
                false => "FAIL — the harness is scoring something other than the fixer",
            }
        );
    }
    harness.cleanup();
}

/// Seed-validity: every seed's own ground-truth test must pass (the corpus
/// ground truth is only as good as this check — the first draft shipped 10
/// hand-counted-string-length seeds whose tests failed, caught only by the
/// post-fix pass because unrepaired cases never get tested).
fn seeds_section() {
    println!("\n── Seed validity: every seed's ground-truth test must pass ──\n");
    let harness = FloorHarness::new();
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    let cases = floor_case_list();
    match harness.verify_seed_tests(&cases) {
        Err(e) => println!("  ERROR: {e}"),
        Ok(bad) => match bad.is_empty() {
            true => println!("  PASS — {}/{} seed tests green", cases.len(), cases.len()),
            false => println!(
                "  FAIL — {} seed(s) with failing ground truth:\n    {}",
                bad.len(),
                bad.join("\n    ")
            ),
        },
    }
    harness.cleanup();
}

// ── section 5: the G6/G7 ordering A/B (latent_retrieval builds) ───────

/// The template-covered stratum: codes our playbooks propose for. The
/// delegated contested codes (E0596/E0382/E0507/E0373) have exactly one
/// "candidate" (none) — ordering cannot matter there.
#[cfg(feature = "latent_retrieval")]
fn template_covered<'a>(
    committed: &'a [riir_clippy::domains::rustc_errors::floor::MutantCase],
    generated: &'a [riir_clippy::domains::rustc_errors::corpus_gen::GeneratedCase],
) -> (
    Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'a>>,
    usize,
) {
    let mut cases = Vec::new();
    let mut n = 0;
    for c in committed {
        if matches!(c.expect_code, "E0597" | "E0499" | "E0502" | "E0505") {
            if let Ok(h) = riir_clippy::domains::rustc_errors::floor::HarnessCase::committed(c) {
                cases.push(h);
                n += 1;
            }
        }
    }
    for g in generated {
        if matches!(g.expect_code, "E0597" | "E0499" | "E0502" | "E0505") {
            cases.push(riir_clippy::domains::rustc_errors::floor::HarnessCase::generated(g));
            n += 1;
        }
    }
    (cases, n)
}

#[cfg(feature = "latent_retrieval")]
fn ordering_ab_section() {
    use riir_clippy::domains::rustc_errors::evolve::traj_key;
    use riir_clippy::traj_store::{shape_conditional_order, TrajStore};
    use std::collections::HashMap;
    use std::sync::Arc;

    println!("\n── 5. G6/G7 ordering A/B (static vs flat vs shape-conditional) ──\n");
    let harness = FloorHarness::new();
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    let generated = generate_corpus();
    let (cases, n) = template_covered(MUTANT_CASES, &generated);
    println!("  template-covered stratum: {n} cases");

    // Fresh store for a self-contained measurement.
    let root = std::env::temp_dir().join(format!("riir_floor_ab_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (store, bad) = TrajStore::open(&root).expect("temp traj store");
    assert_eq!(bad, 0);

    // ── Learning pass (static ordering): populate the store ──
    let learning = DriverFixer::new();
    let mut learned = 0usize;
    let (_, reports) = harness
        .measure_driver(&learning, &cases)
        .expect("learning pass");
    for (name, rep) in &reports {
        for a in &rep.applied {
            store
                .record(
                    &traj_key(&a.code, &a.strategy_id),
                    &a.shape,
                    &a.fix,
                    true,
                    true,
                    "floor-learning",
                )
                .expect("record applied");
            learned += 1;
        }
        for r in &rep.reverted {
            store
                .record(
                    &traj_key(&r.code, &r.strategy_id),
                    &r.shape,
                    &r.fix,
                    false,
                    false,
                    "floor-learning",
                )
                .expect("record reverted");
            learned += 1;
        }
        let _ = name;
    }
    println!("  learning pass: {learned} trajectories recorded");

    // ── Arm builders ──
    // Flat: BetaPosterior over (code,strategy) counters, shape-blind —
    // built from the store dump (the JSON-backend fallback's semantics).
    let mut flat_counts: HashMap<String, (u32, u32)> = HashMap::new();
    for row in store.dump() {
        let e = flat_counts.entry(row.lint_key.clone()).or_default();
        e.0 = e.0.saturating_add(row.success_count);
        e.1 = e.1.saturating_add(row.fail_count);
    }
    let flat_ordering = Arc::new(
        move |cands: &mut [riir_clippy::domains::rustc_errors::playbooks::FixCandidate]| {
            let score = |c: &riir_clippy::domains::rustc_errors::playbooks::FixCandidate| {
                match flat_counts.get(&traj_key(&c.code, &c.strategy_id)) {
                    None => 0.32, // Beta(1,1) @ eps — the unseen prior
                    Some(&(s, f)) => katgpt_core::best_belief_score(s, f, 0.32),
                }
            };
            cands.sort_by(|a, b| {
                score(b)
                    .total_cmp(&score(a))
                    .then(a.total_len().cmp(&b.total_len()))
                    .then(a.strategy_id.cmp(&b.strategy_id))
            });
        },
    );

    let arm_stats = |label: &str, fixer: &DriverFixer| {
        let (_, reports) = harness.measure_driver(fixer, &cases).expect(label);
        let mut total_iters = 0usize;
        let mut total_reverts = 0usize;
        let mut fixed_clean = 0usize;
        for (name, rep) in &reports {
            total_iters += rep.iterations;
            total_reverts += rep.reverted.len();
            if rep.clean {
                fixed_clean += 1;
            }
            let _ = name;
        }
        println!(
            "  {label:<20} clean {fixed_clean}/{n}  iterations {total_iters}  reverts {total_reverts}"
        );
        (fixed_clean, total_iters, total_reverts)
    };

    let store_arc = Arc::new(store);
    let shape_fixer = {
        let s = Arc::clone(&store_arc);
        DriverFixer::new().with_ordering(Arc::new(move |cands| {
            shape_conditional_order(cands, &s);
        }))
    };
    let flat_fixer = DriverFixer::new().with_ordering(flat_ordering);

    let static_arm = arm_stats("static (min-delta)", &DriverFixer::new());
    let flat_arm = arm_stats("flat counters", &flat_fixer);
    let shape_arm = arm_stats("shape-conditional", &shape_fixer);

    println!(
        "\n  verdict: shape-conditional reverts {} vs static {} — {}",
        shape_arm.2,
        static_arm.2,
        match shape_arm.2.cmp(&static_arm.2) {
            std::cmp::Ordering::Less => "IMPROVEMENT (T4.2 earns its keep)",
            std::cmp::Ordering::Equal => "no difference on this corpus",
            std::cmp::Ordering::Greater => "WORSE (record honest negative; keep static)",
        }
    );
    let _ = flat_arm;

    harness.cleanup();
    let _ = std::fs::remove_dir_all(&root);
}

// ── shared helpers ────────────────────────────────────────────────────

fn floor_case_list() -> Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'static>> {
    // 'static: committed cases are 'static consts; generated cases leak
    // their Strings once per process (the measurement is a one-shot binary
    // — Box::leak keeps the borrow simple without rebuilding per section).
    use riir_clippy::domains::rustc_errors::floor::HarnessCase;
    let mut cases: Vec<HarnessCase<'static>> = MUTANT_CASES
        .iter()
        .map(|c| HarnessCase::committed(c).expect("committed case integrity"))
        .collect();
    for g in generate_corpus() {
        cases.push(HarnessCase::generated(Box::leak(Box::new(g))));
    }
    cases
}

fn print_report(report: &FloorReport) {
    println!("  arm: {}\n", report.arm);
    println!(
        "  {:<8} {:>8} {:>9} {:>9} {:>8} {:>9} {:>11}",
        "code", "mutants", "matched", "diverged", "unfixed", "fix-rate", "divergence"
    );
    for row in &report.rows {
        println!(
            "  {:<8} {:>8} {:>9} {:>9} {:>8} {:>8.0}% {:>10.0}%",
            row.code,
            row.mutants,
            row.matched,
            row.diverged,
            row.unfixed,
            row.fix_rate() * 100.0,
            row.divergence_rate() * 100.0
        );
    }
    let uncontested: Vec<&FloorOutcome> = report
        .outcomes
        .iter()
        .filter(|o| {
            riir_clippy::domains::rustc_errors::corpus_gen::territory_of(o.expect_code)
                == Territory::Uncontested
        })
        .collect();
    if !uncontested.is_empty() {
        let fixed = uncontested
            .iter()
            .filter(|o| o.verdict != FixVerdict::Unfixed)
            .count();
        println!(
            "\n  UNCONTESTED stratum (the product head): {fixed}/{} fixed",
            uncontested.len()
        );
    }
    println!(
        "\n  TOTAL fix rate: {:.1}% ({} mutants)",
        report.total_fix_rate() * 100.0,
        report.outcomes.len()
    );
}

fn print_integrity(report: &FloorReport) {
    if !report.invalid_mutants.is_empty() {
        println!(
            "\n  CORPUS INTEGRITY FAILURES ({}):",
            report.invalid_mutants.len()
        );
        for bad in &report.invalid_mutants {
            println!("    - {bad}");
        }
    } else {
        println!("  corpus integrity: all mutants provoked their declared code");
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
