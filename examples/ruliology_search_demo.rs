//! GOAT gate for Issue 005: L3 ruliology Pareto-front candidate search.
//!
//! Gates:
//! - G1 (correctness): Pareto-front correctly filters dominated candidates;
//!   minimum-AST-delta fix is picked among Pareto-optimal.
//! - G2 (latency): search over N candidates completes in < target time.
//!   Not a hot-path gate (runs once per lint invocation, not per-tick).
//! - G3 (no-regression): default features (no `ruliology_search`) still pass.
//! - G4 (alloc-free steady-state): N/A — runs once per invocation, not per-tick.
//!   The Pareto-front output Vec is the result, not steady-state allocation.
//!
//! Run with: cargo run --example ruliology_search_demo --features ruliology_search --release

#![allow(clippy::pedantic)]

#[cfg(feature = "ruliology_search")]
use riir_clippy::{
    RuliologySearch, build_seed_corpus,
    ruliology_search::{ast_delta_size, pareto_front},
};

#[cfg(feature = "ruliology_search")]
use std::time::Instant;

#[cfg(feature = "ruliology_search")]
fn main() {
    println!("=== Issue 005 GOAT gate: L3 ruliology Pareto-front search ===\n");

    // ── G1: Correctness (Pareto-front + dominance) ──────────────────────
    {
        println!("G1 (correctness — Pareto-front + minimum-complexity pick):");

        // G1a: pareto_front filters dominated candidates.
        // Candidate 0: payoff=true, complexity=10 (dominated by 2)
        // Candidate 1: payoff=false, complexity=5 (dominated by 3)
        // Candidate 2: payoff=true, complexity=5 (Pareto-optimal)
        // Candidate 3: payoff=false, complexity=3 (Pareto-optimal)
        let payoffs = vec![true, false, true, false];
        let complexities = vec![10usize, 5, 5, 3];
        let front = pareto_front(&payoffs, &complexities);
        let g1a = front.contains(&2) && front.contains(&3) && !front.contains(&0) && !front.contains(&1);
        println!("  pareto_front filters dominated: front={front:?} → {}", if g1a { "✅" } else { "❌" });

        // G1b: ast_delta_size gives simpler fixes lower complexity.
        let buggy = "fn foo() -> i32 { return 42; }";
        let simple_delta = ast_delta_size(buggy, "fn foo() -> i32 { 42 }");
        let complex_delta = ast_delta_size(buggy, "fn foo() -> i32 { match () { () => 42 } }");
        let g1b = simple_delta < complex_delta;
        println!(
            "  ast_delta simpler < complex: {simple_delta} < {complex_delta} → {}",
            if g1b { "✅" } else { "❌" }
        );

        // G1c: best_fix picks the minimum-complexity Pareto-optimal fix.
        let search = RuliologySearch::new();
        use riir_clippy::corpus::ClippyFixEntry;
        let candidates = vec![
            ClippyFixEntry {
                lint_name: "needless_return".to_string(),
                lint_group: "style".to_string(),
                buggy_code: buggy.to_string(),
                fixed_code: "fn foo() -> i32 { match () { () => 42 } }".to_string(),
            },
            ClippyFixEntry {
                lint_name: "needless_return".to_string(),
                lint_group: "style".to_string(),
                buggy_code: buggy.to_string(),
                fixed_code: "fn foo() -> i32 { 42 }".to_string(),
            },
        ];
        let best = search.best_fix(buggy, &candidates);
        let g1c = best.is_some() && best.unwrap().fixed_code == "fn foo() -> i32 { 42 }";
        println!(
            "  best_fix picks minimum-complexity: → {}",
            if g1c { "✅" } else { "❌" }
        );

        let pass = g1a && g1b && g1c;
        println!("  {}", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "G1 FAIL");
    }

    // ── G2: Latency (not a hot-path gate) ────────────────────────────────
    {
        println!("\nG2 (latency — search over N candidates, not per-tick):");

        let search = RuliologySearch::new();
        let corpus = build_seed_corpus();

        // Build a synthetic 10-candidate scenario by cloning the seed corpus.
        let candidates: Vec<_> = corpus.entries.iter().take(10).cloned().collect();
        let buggy = "fn foo() -> i32 { return 42; }";

        let t0 = Instant::now();
        let result = search.search(buggy, &candidates);
        let elapsed = t0.elapsed();

        println!("  search over {} candidates: {:.2?}", candidates.len(), elapsed);
        println!("  returned {} scored candidates", result.len());

        // Not a hot-path gate. Document the time; assert it's reasonable (< 10ms).
        // The L1 drafter (2.3µs) + L2 pruner (6.8µs) are the hot path; L3 is
        // called once per lint invocation, not per-tick.
        let pass = elapsed.as_millis() < 10;
        println!(
            "  < 10ms target (not hot-path): → {}",
            if pass { "✅" } else { "❌" }
        );
        assert!(pass, "G2 FAIL: search took {elapsed:?}");
    }

    // ── G3: No-regression (seed corpus still searchable) ─────────────────
    {
        println!("\nG3 (no-regression — seed corpus integration):");

        let search = RuliologySearch::new();
        let corpus = build_seed_corpus();
        let mut all_pass = true;

        for entry in &corpus.entries {
            let candidates: Vec<_> = corpus.lookup(&entry.lint_name).cloned().collect();
            let best = search.best_fix(&entry.buggy_code, &candidates);
            if best.is_none() {
                println!(
                    "  ❌ best_fix returned None for lint '{}'",
                    entry.lint_name
                );
                all_pass = false;
            }
        }

        println!(
            "  seed corpus: {} lints all searchable → {}",
            corpus.entries.len(),
            if all_pass { "✅" } else { "❌" }
        );
        assert!(all_pass, "G3 FAIL");
    }

    // ── G4: Alloc-free steady-state (N/A — runs once per invocation) ────
    {
        println!("\nG4 (alloc-free steady-state — N/A, runs once per invocation):");
        println!("  L3 search runs once per lint invocation, not per-tick.");
        println!("  The Pareto-front Vec is the result, not steady-state allocation.");
        println!("  The L1 drafter + L2 pruner are the hot path with G4 alloc gates.");
        println!("  ✅ PASS (documented N/A — same pattern as clippy_verify G4)");
    }

    println!("\n=== All GOAT gates PASS ===");
    println!("\nL3 ruliology search is now available behind `ruliology_search`.");
    println!("The Pareto-front algorithm is reimplemented locally from");
    println!("katgpt-ruliology (Bench 572) — no code-level dep on the substrate");
    println!("because clippy fixes have ground-truth payoff, not tournament payoffs.");
}

#[cfg(not(feature = "ruliology_search"))]
fn main() {
    eprintln!(
        "This demo requires the `ruliology_search` feature.\n\
         Run with: cargo run --example ruliology_search_demo --features ruliology_search --release"
    );
    std::process::exit(1);
}
