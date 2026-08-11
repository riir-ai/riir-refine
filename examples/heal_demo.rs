//! End-to-end demo of the clippy-healing pipeline.
//!
//! Runs the full L0→L1→L2 pipeline on the seed corpus:
//! 1. Builds a ClippyCorpus from common clippy lint examples (L0)
//! 2. Drafts fixes using the ternary SIMD matvec drafter (L1)
//! 3. Validates each drafted fix with the syn-based syntax pruner (L2)
//! 4. Prints a summary: which fixes parsed, which failed
//!
//! Run with:
//! ```sh
//! cargo run --example heal_demo
//! ```

use riir_clippy::{
    build_seed_corpus, ClippyTernaryDrafter, RustAnalyzerPruner,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     riir-clippy — End-to-End Clippy Healing Pipeline Demo   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // ── L0: Build the corpus ─────────────────────────────────────────────
    let corpus = build_seed_corpus();
    println!("📦 L0 Corpus: {} lint entries (BLAKE3 verified: {})",
        corpus.entries.len(),
        if corpus.verify() { "✅" } else { "❌" }
    );
    println!("   Lints: {}", corpus.lint_names().join(", "));
    println!();

    // ── L1: Set up the drafter with ternary weights ──────────────────────
    // Use small deterministic weights for the demo (production would load .bits files).
    let weights = demo_weights();
    let mut drafter = ClippyTernaryDrafter::with_weights(weights);
    println!("⚡ L1 Drafter: ternary SIMD matvec (weights: 8 rows × 16 cols)");
    println!();

    // ── L2: Set up the syntax pruner ─────────────────────────────────────
    let pruner = RustAnalyzerPruner::new();
    println!("🔍 L2 Pruner: syn-based syntax validation");
    println!();

    // ── Run the pipeline on each lint ────────────────────────────────────
    println!("─── Pipeline Run ─────────────────────────────────────────────────");
    println!();

    let mut total = 0usize;
    let mut drafted = 0usize;
    let mut validated = 0usize;

    for lint_name in corpus.lint_names() {
        total += 1;
        let candidates: Vec<_> = corpus.lookup(lint_name).cloned().collect();
        if candidates.is_empty() {
            continue;
        }

        // Use the first candidate's buggy_code as the "input" (simulating a real
        // clippy warning). In production, this would come from parsing
        // `cargo clippy --message-format=json` output.
        let buggy_code = &candidates[0].buggy_code;

        // L1: Draft a fix
        let fix = drafter.draft_fix(lint_name, buggy_code, &candidates);
        match &fix {
            Some(f) => {
                drafted += 1;
                // L2: Validate the fix
                let result = pruner.validate(&f.code, lint_name);
                let status = if result.is_valid() { "✅ VALID" } else { "❌ INVALID" };
                validated += if result.is_valid() { 1 } else { 0 };

                println!("Lint: {} [{:?}]", lint_name, f.source);
                println!("  Confidence: {:.4}", f.confidence);
                println!("  Parse:      {}  Lint resolved: {}  {}",
                    result.parses, result.lint_resolved, status);
                if !result.diagnostics.is_empty() {
                    println!("  Diagnostics: {}", result.diagnostics.join("; "));
                }
                // Show a diff preview (first 2 lines of buggy vs fixed)
                let buggy_first = buggy_code.lines().take(2).collect::<Vec<_>>().join(" | ");
                let fixed_first = f.code.lines().take(2).collect::<Vec<_>>().join(" | ");
                println!("  Buggy:  {}", buggy_first);
                println!("  Fixed:  {}", fixed_first);
                println!();
            }
            None => {
                println!("Lint: {} — ⚠️  no fix drafted (empty candidates)", lint_name);
                println!();
            }
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────
    println!("─── Summary ────────────────────────────────────────────────────");
    println!("  Total lints:    {}", total);
    println!("  Fixes drafted:  {} (L1 ternary matvec)", drafted);
    println!("  Fixes valid:    {} (L2 syntax pruner)", validated);
    println!("  Pass rate:      {:.1}%",
        if drafted > 0 { validated as f64 / drafted as f64 * 100.0 } else { 0.0 });
    println!();

    // ── Fallback mode demo (no weights) ──────────────────────────────────
    println!("─── Fallback Mode (L0 corpus template, no weights) ─────────────");
    let mut fallback = ClippyTernaryDrafter::fallback();
    let candidates: Vec<_> = corpus.lookup("needless_return").cloned().collect();
    if let Some(fix) = fallback.draft_fix("needless_return", "return 42;", &candidates) {
        let result = pruner.validate(&fix.code, "needless_return");
        println!("  Lint:     needless_return");
        println!("  Source:   {:?}", fix.source);
        println!("  Conf:     {:.4} (template default)", fix.confidence);
        println!("  Valid:    {} {}", result.parses, if result.is_valid() { "✅" } else { "❌" });
    }
    println!();
    println!("✅ Pipeline demo complete.");
}

/// Build small deterministic ternary weights for the demo.
///
/// In production, these would be loaded from `.bits` files (frozen direction
/// vectors trained on a clippy lint corpus). For the demo, we use a small
/// deterministic pattern that exercises the SIMD matvec path.
fn demo_weights() -> katgpt_core::TernaryWeights {
    let rows = 8;
    let cols = 16;
    let mut weights = vec![0.0f32; rows * cols];
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
