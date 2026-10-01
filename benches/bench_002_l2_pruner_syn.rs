//! GOAT gate for Issue 002: L2 pruner `syn`-based syntax validation.
//!
//! Gates:
//! - G1 (correctness): valid Rust → parses=true; invalid Rust → parses=false (deterministic)
//! - G2 (latency): parse check p50 < 100µs for typical snippet sizes
//! - G3 (no-regression): the stub test is gone; real validation works
//! - G4 (alloc-bounded): diagnostics empty on success, bounded on failure

#![allow(clippy::pedantic)]

use riir_refine::RustAnalyzerPruner;
use std::time::Instant;

/// Representative corpus snippets — the shapes the pruner sees in production.
const VALID_SNIPPETS: &[&str] = &[
    "fn foo() -> i32 { 42 }",
    "struct Point { x: f32, y: f32 }",
    "impl Point { fn new(x: f32, y: f32) -> Self { Self { x, y } } }",
    "enum Color { Red, Green, Blue(u8) }",
    "trait Greet { fn hello(&self) -> String; }",
    "const MAX_SIZE: usize = 1024;",
    "static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);",
    "fn add(a: i32, b: i32) -> i32 { a + b }",
];

const INVALID_SNIPPETS: &[&str] = &[
    "fn foo( -> {",
    "struct {",
    "impl Foo { fn bar( }",
    "let x = ;",
    "fn () -> {",
    "}}}{{{",
    "this is not rust!!!",
    "fn broken(->",
];

fn main() {
    println!("=== Issue 002 GOAT gate: L2 pruner syn syntax validation ===\n");

    let pruner = RustAnalyzerPruner::new();

    // ── G1: Correctness (deterministic parse classification) ──────────────
    {
        let mut all_valid_pass = true;
        let mut all_invalid_pass = true;

        println!("G1 (correctness):");
        println!("  valid snippets:");
        for (i, code) in VALID_SNIPPETS.iter().enumerate() {
            // Run twice — must be deterministic.
            let r1 = pruner.validate(code, "test_lint");
            let r2 = pruner.validate(code, "test_lint");
            let deterministic = r1.parses == r2.parses;
            let correct = r1.parses; // should parse
            let pass = deterministic && correct;
            if !pass {
                all_valid_pass = false;
            }
            println!(
                "    [{i}] parses={:>5}  det={:>5}  {}  {:?}",
                r1.parses,
                deterministic,
                if pass { "✅" } else { "❌" },
                if code.len() > 40 { &code[..40] } else { code }
            );
        }

        println!("  invalid snippets:");
        for (i, code) in INVALID_SNIPPETS.iter().enumerate() {
            let r1 = pruner.validate(code, "test_lint");
            let r2 = pruner.validate(code, "test_lint");
            let deterministic = r1.parses == r2.parses;
            let correct = !r1.parses; // should NOT parse
            let pass = deterministic && correct;
            if !pass {
                all_invalid_pass = false;
            }
            println!(
                "    [{i}] parses={:>5}  det={:>5}  {}  {:?}",
                r1.parses,
                deterministic,
                if pass { "✅" } else { "❌" },
                if code.len() > 40 { &code[..40] } else { code }
            );
        }

        let pass = all_valid_pass && all_invalid_pass;
        println!(
            "  valid all correct:   {}\n  invalid all correct: {}\n  {}",
            all_valid_pass,
            all_invalid_pass,
            if pass { "✅ PASS" } else { "❌ FAIL" }
        );
        assert!(pass, "G1 FAIL");
        println!();
    }

    // ── G2: Latency ───────────────────────────────────────────────────────
    {
        let n_iters = 10_000;
        let sample = "fn foo() -> i32 { let x = 42; x + 1 }";

        // Warmup
        for _ in 0..100 {
            let _ = pruner.validate(sample, "needless_return");
        }

        let mut times_ns = Vec::with_capacity(n_iters);
        for _ in 0..n_iters {
            let start = Instant::now();
            let _ = pruner.validate(sample, "needless_return");
            times_ns.push(start.elapsed().as_nanos() as u64);
        }

        times_ns.sort_unstable();
        let p50 = times_ns[n_iters / 2];
        let p99 = times_ns[n_iters * 99 / 100];
        let mean = times_ns.iter().sum::<u64>() / n_iters as u64;

        println!("G2 (latency) @ snippet ~40 bytes, {} iters:", n_iters);
        println!("  p50:  {p50} ns  ({:.3} µs)", p50 as f64 / 1000.0);
        println!("  p99:  {p99} ns  ({:.3} µs)", p99 as f64 / 1000.0);
        println!("  mean: {mean} ns  ({:.3} µs)", mean as f64 / 1000.0);

        let target_ns = 100_000; // 100µs target
        let pass = p50 < target_ns;
        println!(
            "  target: p50 < {target_ns} ns ({:.0} µs)",
            target_ns as f64 / 1000.0
        );
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G2 FAIL: p50 {p50}ns >= {target_ns}ns");
        println!();
    }

    // ── G3: No-regression ─────────────────────────────────────────────────
    {
        // The old stub always returned parses=true. Now we verify the REAL
        // parse check works: the pruner correctly distinguishes valid from
        // invalid. This is the regression guard against re-stubbing.
        let valid = pruner.validate("fn ok() {}", "test");
        let invalid = pruner.validate("fn broken(->", "test");

        let stub_regressed = invalid.parses; // if true, we re-stubbed
        let real_check = valid.parses && !invalid.parses;

        println!("G3 (no-regression — real parse check, not stub):");
        println!("  valid parses:   {} (must be true)", valid.parses);
        println!("  invalid parses: {} (must be false)", invalid.parses);
        println!(
            "  stub regressed: {}",
            if stub_regressed {
                "YES — re-stubbed!"
            } else {
                "no"
            }
        );
        let pass = real_check && !stub_regressed;
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G3 FAIL: parse check regressed to stub");
        println!();
    }

    // ── G4: Alloc-bounded (diagnostics) ───────────────────────────────────
    {
        // On success, diagnostics must be empty (zero alloc).
        // On failure, diagnostics is bounded (typically 1 entry — the parse error).
        let n_iters = 1000;

        let mut success_diag_lens: Vec<usize> = Vec::with_capacity(n_iters);
        let mut failure_diag_lens: Vec<usize> = Vec::with_capacity(n_iters);

        for i in 0..n_iters {
            let valid_code = "fn foo() -> i32 { 42 }";
            let r = pruner.validate(valid_code, "test");
            success_diag_lens.push(r.diagnostics.len());

            let invalid_code = "fn broken( ->";
            let r = pruner.validate(invalid_code, "test");
            failure_diag_lens.push(r.diagnostics.len());

            // Touch i to prevent optimizer from eliding
            if i == 0 {
                let _ = &r;
            }
        }

        let max_success = *success_diag_lens.iter().max().unwrap_or(&0);
        let max_failure = *failure_diag_lens.iter().max().unwrap_or(&0);
        let all_success_empty = success_diag_lens.iter().all(|&n| n == 0);
        let failure_bounded = max_failure <= 2; // at most a couple of error messages

        println!("G4 (alloc-bounded, {n_iters} iterations):");
        println!(
            "  success diagnostics: all empty = {} (max len = {max_success})",
            all_success_empty
        );
        println!(
            "  failure diagnostics: bounded = {} (max len = {max_failure})",
            failure_bounded
        );

        let pass = all_success_empty && failure_bounded;
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G4 FAIL: diagnostics unbounded");
        println!();
    }

    println!("=== All GOAT gates PASS ===");
}
