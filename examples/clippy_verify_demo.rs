//! GOAT gate for Issue 003: Phase 3 clippy lint resolution verification.
//!
//! Gates:
//! - G1 (correctness): buggy code → lint_resolved=false; fixed code → lint_resolved=true
//! - G2 (latency): document honestly (seconds, not µs — I/O bound, not hot path)
//! - G3 (no-regression): feature off by default → existing tests pass
//! - G4 (alloc): N/A for I/O bound — documented honestly
//!
//! Run with: cargo run --features clippy_verify --bin bench_003_clippy_verify
//!
//! NOTE: this benchmark is SLOW (runs real cargo clippy multiple times).
//! It's behind the `clippy_verify` feature + a separate bin (not a bench
//! target) because it takes seconds, not microseconds.

#![allow(clippy::pedantic)]

#[cfg(feature = "clippy_verify")]
use riir_clippy::{ClippyVerifier, RustAnalyzerPruner};
#[cfg(feature = "clippy_verify")]
use std::time::Instant;

#[cfg(feature = "clippy_verify")]
fn main() {
    println!("=== Issue 003 GOAT gate: Phase 3 clippy lint resolution ===\n");

    let verifier = ClippyVerifier::new();

    if !verifier.is_cargo_available() {
        eprintln!("❌ cargo not available — cannot run Phase 3 GOAT gate");
        std::process::exit(1);
    }
    println!("cargo: available");

    // ── G1: Correctness (real lint detection) ────────────────────────────
    {
        println!("\nG1 (correctness — real cargo clippy lint detection):");

        // Case 1: buggy code (has needless_return) → should NOT resolve.
        let buggy = "fn foo() -> i32 { return 42; }";
        let r_buggy = verifier.verify(buggy, "needless_return");
        println!(
            "  buggy code:  ran={}, lint_resolved={} (expected false)",
            r_buggy.ran, r_buggy.lint_resolved
        );
        if !r_buggy.ran {
            eprintln!("  ❌ cargo clippy did not run: {:?}", r_buggy.error);
            std::process::exit(1);
        }
        let buggy_pass = !r_buggy.lint_resolved;

        // Case 2: fixed code (no return) → should resolve.
        let fixed = "fn foo() -> i32 { 42 }";
        let r_fixed = verifier.verify(fixed, "needless_return");
        println!(
            "  fixed code:  ran={}, lint_resolved={} (expected true)",
            r_fixed.ran, r_fixed.lint_resolved
        );
        let fixed_pass = r_fixed.lint_resolved;

        let pass = buggy_pass && fixed_pass;
        println!(
            "  buggy detected:   {}",
            if buggy_pass { "✅" } else { "❌" }
        );
        println!(
            "  fixed resolved:   {}",
            if fixed_pass { "✅" } else { "❌" }
        );
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G1 FAIL");
    }

    // ── G2: Latency (document honestly) ──────────────────────────────────
    {
        println!("\nG2 (latency — I/O bound, seconds not µs):");

        let code = "fn double(x: i32) -> i32 { x * 2 }";

        // First run (cold — may build cargo deps).
        let t0 = Instant::now();
        let _ = verifier.verify(code, "needless_return");
        let cold = t0.elapsed();

        // Second run (warm — cargo reuses build cache via BLAKE3-keyed dir).
        let t1 = Instant::now();
        let _ = verifier.verify(code, "needless_return");
        let warm = t1.elapsed();

        println!("  cold run: {:.3} s", cold.as_secs_f64());
        println!(
            "  warm run: {:.3} s (cargo cache reuse)",
            warm.as_secs_f64()
        );
        println!("  note: NOT a hot-path gate. The L2 syntax check (6.8µs) is the");
        println!("        fast path. This is thorough verification, called rarely.");
    }

    // ── G3: No-regression (composition with L2) ──────────────────────────
    {
        println!("\nG3 (no-regression — L2 syntax check still works):");

        let pruner = RustAnalyzerPruner::new();

        // The fast path (validate) is unchanged.
        let valid = pruner.validate("fn ok() {}", "test");
        let invalid = pruner.validate("fn broken(->", "test");
        let fast_path_ok = valid.parses && !invalid.parses;

        // The slow path (validate_with_clippy) composes correctly.
        let slow_valid = pruner.validate_with_clippy("fn ok() {}", "needless_return");
        let composition_ok = slow_valid.parses;

        println!(
            "  fast path (validate):     parses valid={}, invalid={} → {}",
            valid.parses,
            invalid.parses,
            if fast_path_ok { "✅" } else { "❌" }
        );
        println!(
            "  slow path (validate_with_clippy): parses={} → {}",
            slow_valid.parses,
            if composition_ok { "✅" } else { "❌" }
        );

        let pass = fast_path_ok && composition_ok;
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G3 FAIL");
    }

    // ── G4: Allocation (N/A — I/O bound) ─────────────────────────────────
    {
        println!("\nG4 (alloc-free — N/A for I/O bound):");
        println!("  This module spawns a process + does temp-dir I/O.");
        println!("  Allocation tracking is not meaningful here (unlike L1/L2 hot paths).");
        println!(
            "  The hot-path alloc gates are G4 for Issue 001 (L1 matvec) + Issue 002 (L2 syn)."
        );
        println!("  ✅ PASS (documented N/A)");
    }

    println!("\n=== All GOAT gates PASS ===");
    println!("\nThe lint_resolved field is now TRUTHFUL when validate_with_clippy is used.");
    println!("The old validate() always returned lint_resolved=true (documented lie).");
    println!("Phase 3 closes the honesty gap: cargo clippy is the ground truth.");
}

#[cfg(not(feature = "clippy_verify"))]
fn main() {
    eprintln!(
        "This benchmark requires the `clippy_verify` feature.\n\
         Run with: cargo run --features clippy_verify --bin bench_003_clippy_verify"
    );
    std::process::exit(1);
}
