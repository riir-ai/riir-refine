//! GOAT gate for Issue 001: L1 ternary matvec path in ClippyTernaryDrafter.
//!
//! Gates:
//! - G1 (determinism): same input → identical output across runs
//! - G2 (latency): matvec path p50 < 10µs at cols=256, rows=64
//! - G3 (no-regression): L0 fallback path unchanged
//! - G4 (alloc-free): scratch buffer capacity stable across 1000 calls

#![allow(clippy::pedantic)]

use riir_clippy::corpus::ClippyFixEntry;
use riir_clippy::drafter::{ClippyTernaryDrafter, DraftSource};
use std::time::Instant;

fn mock_entries(n: usize) -> Vec<ClippyFixEntry> {
    (0..n)
        .map(|i| ClippyFixEntry {
            lint_name: "needless_return".to_string(),
            lint_group: "style".to_string(),
            buggy_code: format!("fn f() -> i32 {{ return {i}; }}"),
            fixed_code: format!("fn f() -> i32 {{ {i} }}"),
        })
        .collect()
}

/// Realistic weights: 64 rows × 256 cols (matches quest_grammar scale).
fn realistic_weights() -> katgpt_core::TernaryWeights {
    let rows = 64;
    let cols = 256;
    let mut weights = vec![0.0f32; rows * cols];
    // Deterministic pattern (not all zeros — that would make matvec trivial).
    for i in 0..rows {
        for j in 0..cols {
            let idx = i * cols + j;
            weights[idx] = match (i + j) % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            };
        }
    }
    katgpt_core::TernaryWeights::quantize_from_f32(&weights, rows, cols)
}

fn main() {
    println!("=== Issue 001 GOAT gate: L1 ternary matvec path ===\n");

    let weights = realistic_weights();
    let entries = mock_entries(8);

    // ── G1: Determinism ──────────────────────────────────────────────────
    {
        let mut drafter = ClippyTernaryDrafter::with_weights(weights.clone());
        let r1 = drafter.draft_fix("needless_return", "return 42;", &entries).unwrap();
        let r2 = drafter.draft_fix("needless_return", "return 42;", &entries).unwrap();

        let code_match = r1.code == r2.code;
        let conf_match = r1.confidence.to_bits() == r2.confidence.to_bits();
        let source_match = r1.source == r2.source;

        println!("G1 (determinism):");
        println!("  code identical:      {code_match}");
        println!("  confidence bits eq:  {conf_match}");
        println!("  source identical:    {source_match}");
        assert!(code_match && conf_match && source_match, "G1 FAIL");
        println!("  ✅ PASS\n");
    }

    // ── G2: Latency ──────────────────────────────────────────────────────
    {
        let mut drafter = ClippyTernaryDrafter::with_weights(weights.clone());
        let n_iters = 10_000;

        // Warmup
        for _ in 0..100 {
            let _ = drafter.draft_fix("needless_return", "return 42;", &entries);
        }

        let mut times_ns = Vec::with_capacity(n_iters);
        for _ in 0..n_iters {
            let start = Instant::now();
            let _ = drafter.draft_fix("needless_return", "return 42;", &entries);
            times_ns.push(start.elapsed().as_nanos() as u64);
        }

        times_ns.sort_unstable();
        let p50 = times_ns[n_iters / 2];
        let p99 = times_ns[n_iters * 99 / 100];
        let mean = times_ns.iter().sum::<u64>() / n_iters as u64;

        println!("G2 (latency) @ cols=256, rows=64, {} iters:", n_iters);
        println!("  p50:  {p50} ns  ({:.3} µs)", p50 as f64 / 1000.0);
        println!("  p99:  {p99} ns  ({:.3} µs)", p99 as f64 / 1000.0);
        println!("  mean: {mean} ns  ({:.3} µs)", mean as f64 / 1000.0);

        let target_ns = 10_000; // 10µs target
        let pass = p50 < target_ns;
        println!("  target: p50 < {target_ns} ns ({:.1} µs)", target_ns as f64 / 1000.0);
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G2 FAIL: p50 {p50}ns >= {target_ns}ns");
        println!();
    }

    // ── G3: No-regression (L0 fallback unchanged) ────────────────────────
    {
        let mut fallback = ClippyTernaryDrafter::fallback();
        assert!(fallback.is_fallback());

        let fix = fallback.draft_fix("needless_return", "return 42;", &entries).unwrap();
        let pass = fix.source == DraftSource::CorpusTemplate
            && (fix.confidence - 0.5).abs() < 1e-6
            && !fix.code.is_empty();

        println!("G3 (no-regression L0 fallback):");
        println!("  source:   {:?}", fix.source);
        println!("  conf:     {:.3} (expected 0.5)", fix.confidence);
        println!("  code len: {}", fix.code.len());
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G3 FAIL");
        println!();
    }

    // ── G4: Alloc-free steady state ──────────────────────────────────────
    {
        let mut drafter = ClippyTernaryDrafter::with_weights(weights.clone());

        let input_cap_before = drafter.input_scratch_capacity();
        let output_cap_before = drafter.output_scratch_capacity();

        // Run 1000 calls — capacity must not grow.
        for i in 0..1000u32 {
            let lint = match i % 3 {
                0 => "needless_return",
                1 => "redundant_closure",
                _ => "unused_var",
            };
            let code = format!("expr_{i}");
            let _ = drafter.draft_fix(lint, &code, &entries);
        }

        let input_cap_after = drafter.input_scratch_capacity();
        let output_cap_after = drafter.output_scratch_capacity();

        let input_stable = input_cap_before == input_cap_after;
        let output_stable = output_cap_before == output_cap_after;

        println!("G4 (alloc-free steady state, 1000 calls):");
        println!("  input_scratch  capacity: {input_cap_before} → {input_cap_after}  {}", if input_stable { "✅" } else { "❌" });
        println!("  output_scratch capacity: {output_cap_before} → {output_cap_after}  {}", if output_stable { "✅" } else { "❌" });

        assert!(input_stable && output_stable, "G4 FAIL: scratch buffers grew");
        println!("  ✅ PASS\n");
    }

    println!("=== All GOAT gates PASS ===");
}
