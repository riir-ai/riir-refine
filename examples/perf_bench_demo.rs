//! GOAT gate for the `rust_perf` bench Pareto validator (Proposal 001 caveat 3).
//!
//! This is the **promotion gate** for the `rust_perf` domain. It runs the
//! bench validator across all corpus rules + reports:
//! - How many rules are Pareto-dominant (faster on majority of sizes)
//! - How many are NotBenchable (honest — need external runtimes)
//! - Per-rule Pareto verdicts with speedup ratios
//! - Per-rule allocation axis (count + bytes, Issue 014 T1) + determinism
//!   proof across 3 repeat runs (Issue 014 T1.4)
//!
//! Gates:
//! - G1 (correctness): benchable rules produce real measurements (ran=true,
//!   non-empty results, non-zero timings)
//! - G2 (latency): document honestly (seconds per rule — I/O bound)
//! - G3 (no-regression): default features (no `rust_perf_bench`) still pass
//! - G4 (alloc): the alloc axis is REPORTED per rule; determinism across
//!   repeats is asserted as G-det below (Issue 014 T1.4)
//! - G5 (Pareto verdict): benchable rules are Pareto-dominant (faster on
//!   majority of sizes) — this is the promotion bar
//! - G-det (alloc determinism, Issue 014 T1.4): the alloc axis (count + bytes
//!   per size) is bit-identical across 3 repeat runs for every benchable rule
//!
//! Run with:
//! ```text
//! cargo run --example perf_bench_demo --features rust_perf_bench --release
//! ```

#![allow(clippy::pedantic)]

#[cfg(feature = "rust_perf_bench")]
use riir_clippy::{
    domains::rust_perf::RustPerfDomain,
    domain::HealDomain,
    PerfBenchValidator, SizeOutcome,
};

/// How many times each benchable rule's binary runs for the alloc-determinism
/// proof (Issue 014 T1.4: "identical across 3 repeat runs").
#[cfg(feature = "rust_perf_bench")]
const DETERMINISM_REPEATS: usize = 3;

#[cfg(feature = "rust_perf_bench")]
fn main() {
    println!("=== rust_perf bench Pareto validator — GOAT gate (Bench 020 run) ===\n");

    let validator = PerfBenchValidator::new();

    if !validator.is_cargo_available() {
        eprintln!("\u{274c} cargo not available — cannot run bench GOAT gate");
        std::process::exit(1);
    }
    println!("cargo: available");
    println!("alloc determinism repeats: {DETERMINISM_REPEATS}\n");

    // Build the corpus (with bench specs, since rust_perf_bench is on).
    let corpus = RustPerfDomain::build_corpus();
    println!("Corpus: {} rules\n", corpus.len());

    // Optional substring filter (Batch 48): bench only matching rules —
    // a batch-scoped gate run without re-running the whole corpus.
    // `PERF_BENCH_FILTER=hashset,filter-map cargo run --example ...`
    let filter = std::env::var("PERF_BENCH_FILTER").ok();
    let selected: Vec<&_> = match filter.as_deref() {
        Some(f) if !f.is_empty() => {
            let parts: Vec<&str> = f.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
            let sel: Vec<_> = corpus
                .iter()
                .filter(|e| parts.iter().any(|p| e.rule_id.contains(p)))
                .collect();
            println!("FILTER {f:?}: {}/{} rules selected\n", sel.len(), corpus.len());
            sel
        }
        _ => corpus.iter().collect(),
    };

    let mut pareto_dominant = 0usize;
    let mut mixed = 0usize;
    let mut dominated = 0usize;
    let mut not_benchable = 0usize;
    let mut failed = 0usize;
    let mut nondeterministic: Vec<String> = Vec::new();
    // Machine-readable per-rule rows for the bench doc (Issue 014 T1.3).
    let mut report_rows: Vec<String> = Vec::new();

    for entry in &selected {
        println!("--- Rule: {} ({}) ---", entry.rule_id, entry.name);

        let Some(spec) = entry.bench_spec.as_ref() else {
            let reason = entry
                .context_notes
                .as_deref()
                .unwrap_or("no BenchSpec (rule needs an external runtime or is structural)");
            println!("  \u{2014} NOT BENCHABLE: {reason}");
            not_benchable += 1;
            report_rows.push(format!("{}|NotBenchable|-|-|-|-|-", entry.rule_id));
            println!();
            continue;
        };

        // Issue 014 T1.4: run DETERMINISM_REPEATS times; the alloc axis must
        // be bit-identical across runs. Timing verdict comes from run 1.
        let (results, deterministic) =
            match validator.run_bench_with_determinism(&entry.rule_id, spec, DETERMINISM_REPEATS) {
                Ok(r) => r,
                Err(e) => {
                    println!("  \u{274c} FAILED to run: {e}");
                    failed += 1;
                    report_rows.push(format!("{}|FAILED|-|-|-|-|-", entry.rule_id));
                    println!();
                    continue;
                }
            };
        if !deterministic {
            nondeterministic.push(entry.rule_id.clone());
            println!("  \u{274c} ALLOC AXIS NONDETERMINISTIC across {DETERMINISM_REPEATS} runs");
        }

        let faster_count = results.iter().filter(|r| r.outcome == SizeOutcome::Faster).count();
        let slower_count = results.iter().filter(|r| r.outcome == SizeOutcome::Slower).count();
        let tie_count = results.iter().filter(|r| r.outcome == SizeOutcome::Tie).count();
        let total = results.len();
        // Majority rule (mirrors the validator's aggregate_outcomes).
        let overall = if total == 0 {
            "Mixed"
        } else if faster_count > slower_count && faster_count * 2 > total {
            "ParetoDominant"
        } else if slower_count > faster_count && slower_count * 2 > total {
            "ParetoDominated"
        } else {
            "Mixed"
        };

        println!("  sizes tested: {total}");
        for r in &results {
            let outcome_str = match r.outcome {
                SizeOutcome::Faster => "FASTER",
                SizeOutcome::Slower => "SLOWER",
                SizeOutcome::Tie => "tie",
            };
            println!(
                "    n={:>6}: buggy={:>6}ns  fixed={:>6}ns  speedup={: >6.2}x  [{}]",
                r.size, r.buggy_ns, r.fixed_ns, r.speedup, outcome_str
            );
            println!(
                "             allocs buggy={:>6} fixed={:>6}  bytes buggy={:>8} fixed={:>8}",
                r.buggy_allocs, r.fixed_allocs, r.buggy_alloc_bytes, r.fixed_alloc_bytes
            );
        }
        println!(
            "  verdict: {faster_count} faster, {slower_count} slower, {tie_count} tie \u{2192} {overall}  (alloc-det: {})",
            if deterministic { "stable" } else { "UNSTABLE" }
        );
        match overall {
            "ParetoDominant" => {
                println!("  \u{2713} PARETO DOMINANT");
                pareto_dominant += 1;
            }
            "Mixed" => {
                println!("  ~ MIXED");
                mixed += 1;
            }
            _ => {
                println!("  \u{2717} PARETO DOMINATED (regression)");
                dominated += 1;
            }
        }

        // Machine-readable row: rule|overall|f/s/t|allocs@max|bytes@max|det
        if let Some(last) = results.last() {
            report_rows.push(format!(
                "{}|{}|{}/{}/{}|{}/{}|{}/{}|{}",
                entry.rule_id,
                overall,
                faster_count,
                slower_count,
                tie_count,
                last.buggy_allocs,
                last.fixed_allocs,
                last.buggy_alloc_bytes,
                last.fixed_alloc_bytes,
                if deterministic { "stable" } else { "UNSTABLE" }
            ));
        }
        println!();
    }

    // ── Summary ──────────────────────────────────────────────────────────
    println!("============================================================");
    println!("BENCH VALIDATOR SUMMARY");
    println!("============================================================");
    println!("  Pareto-dominant (faster on majority): {pareto_dominant}");
    println!("  Mixed (some faster, some slower):     {mixed}");
    println!("  Pareto-dominated (regression):        {dominated}");
    println!("  Not benchable (needs external runtime): {not_benchable}");
    println!("  Failed to run:                        {failed}");
    println!(
        "  Alloc-axis nondeterministic (harness bug class): {}",
        nondeterministic.len()
    );
    println!();

    let total_benchable = pareto_dominant + mixed + dominated;
    println!("Benchable rules: {total_benchable} of {}", corpus.len());
    println!("Pareto-dominant: {pareto_dominant} of {total_benchable} benchable");
    println!();

    // ── G1: benchable rules produce real measurements ────────────────────
    println!("G1 (correctness — benchable rules produce real measurements):");
    let g1_pass = failed == 0 && pareto_dominant + mixed + dominated > 0;
    println!("  failed={failed}, benchable_rules={total_benchable}");
    println!("  {}", if g1_pass { "\u{2713} PASS" } else { "\u{274c} FAIL" });

    // ── G-det: alloc axis determinism (Issue 014 T1.4) ────────────────────
    println!("\nG-det (alloc determinism — Issue 014 T1.4, exact integers across runs):");
    let gdet_pass = nondeterministic.is_empty() && total_benchable > 0;
    if gdet_pass {
        println!("  all {total_benchable} benchable rules stable across {DETERMINISM_REPEATS} runs");
    } else {
        println!("  UNSTABLE rules: {nondeterministic:?}");
    }
    println!("  {}", if gdet_pass { "\u{2713} PASS" } else { "\u{274c} FAIL" });

    // ── G5: Pareto verdict — the promotion bar ───────────────────────────
    println!("\nG5 (Pareto verdict — the promotion bar):");
    // Promotion bar: a MAJORITY of benchable rules should be Pareto-dominant.
    // Not ALL rules — some rules are honestly not wins in every regime
    // (e.g., compiler optimizes the computation, or the benefit needs
    // multi-threaded contention the harness doesn't capture). The bar is
    // "the pattern + corpus are valuable for the majority of cases".
    //
    // `pareto_dominant * 2 > total_benchable` is the majority test.
    let g5_pass = total_benchable > 0 && pareto_dominant * 2 > total_benchable;
    println!(
        "  pareto_dominant={pareto_dominant} (must be > half of {total_benchable} benchable)"
    );
    if dominated > 0 {
        println!("  NOTE: {dominated} dominated rule(s) — honest findings:");
        println!("    (compiler optimizations, regime-dependent benefits, etc.)");
    }
    println!("  {}", if g5_pass { "\u{2713} PASS" } else { "\u{274c} FAIL" });

    // ── Machine-readable rows (paste target for the bench doc) ────────────
    println!("\nREPORT_ROWS (rule|overall|f/s/t|allocs@max b/f|bytes@max b/f|det):");
    for row in &report_rows {
        println!("ROW {row}");
    }

    let all_pass = g1_pass && g5_pass && gdet_pass;
    println!("\n============================================================");
    if all_pass {
        println!("\u{2713} ALL GOAT GATES PASS \u{2014} G1 + G5 + G-det");
    } else {
        println!("\u{274c} SOME GOAT GATES FAILED \u{2014} investigate before promotion");
    }
    println!("============================================================");

    if !all_pass {
        std::process::exit(1);
    }
}

#[cfg(not(feature = "rust_perf_bench"))]
fn main() {
    eprintln!(
        "This benchmark requires the `rust_perf_bench` feature.\n\
         Run with: cargo run --example perf_bench_demo --features rust_perf_bench --release"
    );
    std::process::exit(1);
}
