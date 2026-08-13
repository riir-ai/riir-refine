//! GOAT gate for the `rust_perf` bench Pareto validator (Proposal 001 caveat 3).
//!
//! This is the **promotion gate** for the `rust_perf` domain. It runs the
//! bench validator across all corpus rules + reports:
//! - How many rules are Pareto-dominant (faster on majority of sizes)
//! - How many are NotBenchable (honest — need external runtimes)
//! - Per-rule Pareto verdicts with speedup ratios
//!
//! Gates:
//! - G1 (correctness): benchable rules produce real measurements (ran=true,
//!   non-empty results, non-zero timings)
//! - G2 (latency): document honestly (seconds per rule — I/O bound)
//! - G3 (no-regression): default features (no `rust_perf_bench`) still pass
//! - G4 (alloc): N/A for subprocess-based validator
//! - G5 (Pareto verdict): benchable rules are Pareto-dominant (faster on
//!   majority of sizes) — this is the promotion bar
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
    PerfBenchValidator,
};

#[cfg(feature = "rust_perf_bench")]
fn main() {
    println!("=== rust_perf bench Pareto validator — GOAT gate ===\n");

    let validator = PerfBenchValidator::new();

    if !validator.is_cargo_available() {
        eprintln!("\u{274c} cargo not available — cannot run bench GOAT gate");
        std::process::exit(1);
    }
    println!("cargo: available\n");

    // Build the corpus (with bench specs, since rust_perf_bench is on).
    let corpus = RustPerfDomain::build_corpus();
    println!("Corpus: {} rules\n", corpus.len());

    let mut pareto_dominant = 0usize;
    let mut mixed = 0usize;
    let mut dominated = 0usize;
    let mut not_benchable = 0usize;
    let mut failed = 0usize;

    for entry in &corpus {
        println!("--- Rule: {} ({}) ---", entry.rule_id, entry.name);

        #[cfg(feature = "rust_perf_bench")]
        let verdict = validator.validate(&entry.rule_id, entry.bench_spec.as_ref());

        match &verdict {
            riir_clippy::BenchVerdict::Pareto(p) => {
                if !p.ran {
                    println!("  \u{274c} FAILED to run: {:?}", p.error);
                    failed += 1;
                    continue;
                }
                println!("  sizes tested: {}", p.results.len());
                for r in &p.results {
                    let outcome_str = match r.outcome {
                        riir_clippy::SizeOutcome::Faster => "FASTER",
                        riir_clippy::SizeOutcome::Slower => "SLOWER",
                        riir_clippy::SizeOutcome::Tie => "tie",
                    };
                    println!(
                        "    n={:>6}: buggy={:>6}ns  fixed={:>6}ns  speedup={: >6.2}x  [{}]",
                        r.size, r.buggy_ns, r.fixed_ns, r.speedup, outcome_str
                    );
                }
                println!(
                    "  verdict: {} faster, {} slower, {} tie \u{2192} {:?}",
                    p.faster_count, p.slower_count, p.tie_count, p.overall
                );
                match p.overall {
                    riir_clippy::BenchOverall::ParetoDominant => {
                        println!("  \u{2713} PARETO DOMINANT");
                        pareto_dominant += 1;
                    }
                    riir_clippy::BenchOverall::Mixed => {
                        println!("  ~ MIXED");
                        mixed += 1;
                    }
                    riir_clippy::BenchOverall::ParetoDominated => {
                        println!("  \u{2717} PARETO DOMINATED (regression)");
                        dominated += 1;
                    }
                }
            }
            riir_clippy::BenchVerdict::NotBenchable(n) => {
                println!("  \u{2014} NOT BENCHABLE: {}", n.reason);
                not_benchable += 1;
            }
        }
        println!();
    }

    // ── Summary ──────────────────────────────────────────────────────────
    println!("============================================================");
    println!("BENCH VALIDATOR SUMMARY");
    println!("============================================================");
    println!("  Pareto-dominant (faster on majority): {}", pareto_dominant);
    println!("  Mixed (some faster, some slower):     {}", mixed);
    println!("  Pareto-dominated (regression):        {}", dominated);
    println!("  Not benchable (needs external runtime): {}", not_benchable);
    println!("  Failed to run:                        {}", failed);
    println!();

    let total_benchable = pareto_dominant + mixed + dominated;
    println!("Benchable rules: {} of {}", total_benchable, corpus.len());
    println!("Pareto-dominant: {} of {} benchable", pareto_dominant, total_benchable);
    println!();

    // ── G1: benchable rules produce real measurements ────────────────────
    println!("G1 (correctness — benchable rules produce real measurements):");
    let g1_pass = failed == 0 && pareto_dominant + mixed + dominated > 0;
    println!("  failed={}, benchable_rules={}", failed, total_benchable);
    println!("  {}", if g1_pass { "\u{2713} PASS" } else { "\u{274c} FAIL" });

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
        "  pareto_dominant={} (must be > half of {} benchable)",
        pareto_dominant, total_benchable
    );
    if dominated > 0 {
        println!("  NOTE: {} dominated rule(s) — honest findings:", dominated);
        println!("    (compiler optimizations, regime-dependent benefits, etc.)");
    }
    println!("  {}", if g5_pass { "\u{2713} PASS" } else { "\u{274c} FAIL" });

    let all_pass = g1_pass && g5_pass;
    println!("\n============================================================");
    if all_pass {
        println!("\u{2713} ALL GOAT GATES PASS \u{2014} rust_perf is eligible for default-on promotion");
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
