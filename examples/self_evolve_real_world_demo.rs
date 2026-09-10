//! Real-world validation of the self-evolution loop via cargo clippy.
//!
//! This example validates two gates:
//!
//! - **G6 (corpus truthfulness)**: verifies that each seed corpus entry's
//!   `buggy_code` triggers `clippy::<lint_name>` AND `fixed_code` resolves
//!   it. This is the factual prerequisite for any evolution test.
//!
//! - **G7 (self-evolution with real clippy feedback)**: runs the self-evolution
//!   loop with `cargo clippy` as the ground-truth verifier instead of an
//!   oracle. Measures whether the healing rate improves after feedback.
//!
//! Run with:
//! ```sh
//! cargo run --example self_evolve_real_world_demo --features "self_evolve clippy_verify" --release
//! ```
//!
//! NOTE: this is SLOW — it spawns cargo clippy dozens of times. Expect
//! minutes, not seconds. Each call is BLAKE3-cached, so repeat calls on
//! the same code are faster.

#![allow(clippy::pedantic, clippy::type_complexity)]

#[cfg(all(feature = "self_evolve", feature = "clippy_verify"))]
use riir_clippy::{
    build_seed_corpus, ClippyFixEntry, ClippyTernaryDrafter, ClippyVerifier, HealPipeline,
    RustAnalyzerPruner,
};

#[cfg(all(feature = "self_evolve", feature = "clippy_verify"))]
fn main() {
    println!("=== Issue 008: Real-World Validation of self_evolve via clippy_verify ===\n");

    let verifier = ClippyVerifier::new();

    if !verifier.is_cargo_available() {
        eprintln!("❌ cargo not available — cannot run real-world validation");
        std::process::exit(1);
    }
    println!("cargo: available\n");

    let corpus = build_seed_corpus();

    // ── G6: Corpus truthfulness ───────────────────────────────────────────
    println!("── G6: Corpus truthfulness (buggy triggers, fixed resolves) ──\n");

    let mut g6_pass = true;
    let mut verified_lints: Vec<(String, String, String)> = Vec::new(); // (lint, buggy, fixed)

    for entry in &corpus.entries {
        let buggy_result = verifier.verify(&entry.buggy_code, &entry.lint_name);
        let fixed_result = verifier.verify(&entry.fixed_code, &entry.lint_name);

        if !buggy_result.ran {
            eprintln!(
                "  ❌ {:30} cargo clippy failed to run on buggy code: {:?}",
                entry.lint_name, buggy_result.error
            );
            g6_pass = false;
            continue;
        }
        if !fixed_result.ran {
            eprintln!(
                "  ❌ {:30} cargo clippy failed to run on fixed code: {:?}",
                entry.lint_name, fixed_result.error
            );
            g6_pass = false;
            continue;
        }

        let buggy_triggers = !buggy_result.lint_resolved;
        let fixed_resolves = fixed_result.lint_resolved;

        let pass = buggy_triggers && fixed_resolves;
        println!(
            "  {} {:30} buggy_triggers={}, fixed_resolves={}",
            if pass { "✅" } else { "❌" },
            entry.lint_name,
            if buggy_triggers { "Y" } else { "N" },
            if fixed_resolves { "Y" } else { "N" }
        );

        if !fixed_resolves {
            let other_lints: Vec<_> = fixed_result
                .present_lints
                .iter()
                .filter(|l| l.starts_with("clippy::"))
                .collect();
            eprintln!("    ⚠️ fixed_code still triggers: {:?}", other_lints);
        }
        if !buggy_triggers {
            eprintln!(
                "    ⚠️ buggy_code does NOT trigger clippy::{} (lint name may be wrong)",
                entry.lint_name
            );
        }

        if pass {
            verified_lints.push((
                entry.lint_name.clone(),
                entry.buggy_code.clone(),
                entry.fixed_code.clone(),
            ));
        } else {
            g6_pass = false;
        }
    }

    println!(
        "\n  G6: {} ({}/{} entries verified)\n",
        if g6_pass {
            "✅ ALL PASS"
        } else {
            "❌ SOME FAILED"
        },
        verified_lints.len(),
        corpus.entries.len()
    );

    if !g6_pass {
        eprintln!("❌ G6 failed — cannot proceed to G7 with a broken corpus.");
        eprintln!("   Fix the corpus entries above before re-running.");
        std::process::exit(1);
    }

    // ── G7: Self-evolution with real clippy feedback ──────────────────────
    println!("── G7: Self-evolution with real cargo clippy feedback ──\n");

    // Build the fixture: for each verified lint, candidates = [correct, identity].
    // The identity transform (buggy code as the "fix") parses but doesn't
    // resolve the lint. We rotate which candidate the fallback drafter picks
    // first (corpus[0]) so the baseline is non-trivial.
    //
    // Even-indexed lints: drafter picks correct (index 0) → baseline correct
    // Odd-indexed lints:  drafter picks identity (index 0) → baseline wrong
    // Expected baseline: ~50% (5/10 correct).
    type LintFixture = (String, String, Vec<(String, bool)>);
    let fixture: Vec<LintFixture> = verified_lints
        .iter()
        .enumerate()
        .map(|(i, (lint, buggy, correct))| {
            let identity = buggy.clone(); // wrong: same code, lint still triggers
            let candidates = if i % 2 == 0 {
                vec![(correct.clone(), true), (identity, false)]
            } else {
                vec![(identity, false), (correct.clone(), true)]
            };
            (lint.clone(), buggy.clone(), candidates)
        })
        .collect();

    let n_lints = fixture.len();

    // Helper: build corpus entries for a lint from its candidate list.
    let make_corpus = |lint_name: &str, candidates: &[(String, bool)]| -> Vec<ClippyFixEntry> {
        candidates
            .iter()
            .map(|(code, _)| ClippyFixEntry {
                lint_name: lint_name.to_string(),
                lint_group: "test".to_string(),
                buggy_code: format!("buggy_for_{lint_name}"),
                fixed_code: code.clone(),
            })
            .collect()
    };

    // The "all corpus" for seeding memory: every candidate for every lint.
    let all_corpus: Vec<ClippyFixEntry> = fixture
        .iter()
        .flat_map(|(lint, _, candidates)| make_corpus(lint, candidates))
        .collect();

    // ── Phase 1: Baseline (memory seeded, no feedback) ────────────────────
    println!("Phase 1: baseline (seeded memory, no feedback) — verifying with cargo clippy...");
    let mut baseline_pipeline =
        HealPipeline::new(ClippyTernaryDrafter::fallback(), RustAnalyzerPruner::new());
    baseline_pipeline.seed_from_corpus(&all_corpus);

    let mut baseline_resolved = 0usize;
    for (lint_name, buggy_code, candidates) in &fixture {
        let corpus_entries = make_corpus(lint_name, candidates);
        let result = baseline_pipeline.heal(lint_name, buggy_code, &corpus_entries);
        let (resolved, source, notes) = match result {
            Some(r) => {
                let vr = verifier.verify(&r.fixed_code, lint_name);
                (vr.ran && vr.lint_resolved, r.source, r.context_notes)
            }
            None => (false, riir_clippy::HealSource::Drafter, None),
        };
        if resolved {
            baseline_resolved += 1;
        }
        let note_marker = if notes.is_some() { " ⚠️" } else { "" };
        println!(
            "  {:30} → {}{} (source: {:?})",
            lint_name,
            if resolved {
                "✅ resolved"
            } else {
                "❌ not resolved"
            },
            note_marker,
            source
        );
    }
    #[allow(clippy::cast_precision_loss)]
    let baseline_rate = (baseline_resolved as f64) / (n_lints as f64);
    println!(
        "\n  baseline: {}/{} = {:.1}%\n",
        baseline_resolved,
        n_lints,
        baseline_rate * 100.0
    );

    // ── Phase 2: Evolution loop with real clippy feedback ─────────────────
    println!("Phase 2: evolution loop (real cargo clippy feedback)...");
    let mut evolved_pipeline =
        HealPipeline::new(ClippyTernaryDrafter::fallback(), RustAnalyzerPruner::new());
    evolved_pipeline.seed_from_corpus(&all_corpus);

    // Run 3 rounds of feedback: heal → verify → report outcome.
    // Round 1: half the lints fail (identity picked) → fail_count rises →
    //          correct out-scores identity on next heal.
    // Round 2: correct gets picked → success → direction initialized.
    // Round 3: direction favors correct → always correct.
    const N_ROUNDS: usize = 3;
    for round in 0..N_ROUNDS {
        let mut round_resolved = 0usize;
        for (lint_name, buggy_code, candidates) in &fixture {
            let corpus_entries = make_corpus(lint_name, candidates);
            if let Some(result) = evolved_pipeline.heal(lint_name, buggy_code, &corpus_entries) {
                let vr = verifier.verify(&result.fixed_code, lint_name);
                let success = vr.ran && vr.lint_resolved;
                // Issue 021: the verdict came from the real verifier — a
                // resolved lint is a certification, not a heuristic success.
                evolved_pipeline.report_outcome(
                    lint_name,
                    buggy_code,
                    &result.fixed_code,
                    success,
                    vr.ran,
                );
                if success {
                    round_resolved += 1;
                }
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let round_rate = (round_resolved as f64) / (n_lints as f64);
        println!(
            "  round {}/{}: {}/{} = {:.1}% resolved",
            round + 1,
            N_ROUNDS,
            round_resolved,
            n_lints,
            round_rate * 100.0
        );
    }

    // ── Phase 3: Measure evolved rate ─────────────────────────────────────
    println!("\nPhase 3: measuring evolved rate (no more feedback)...");
    let mut evolved_resolved = 0usize;
    for (lint_name, buggy_code, candidates) in &fixture {
        let corpus_entries = make_corpus(lint_name, candidates);
        let result = evolved_pipeline.heal(lint_name, buggy_code, &corpus_entries);
        let (resolved, source, notes) = match result {
            Some(r) => {
                let vr = verifier.verify(&r.fixed_code, lint_name);
                (vr.ran && vr.lint_resolved, r.source, r.context_notes)
            }
            None => (false, riir_clippy::HealSource::Drafter, None),
        };
        if resolved {
            evolved_resolved += 1;
        }
        let note_marker = if notes.is_some() { " ⚠️" } else { "" };
        println!(
            "  {:30} → {}{} (source: {:?})",
            lint_name,
            if resolved {
                "✅ resolved"
            } else {
                "❌ not resolved"
            },
            note_marker,
            source
        );
    }
    #[allow(clippy::cast_precision_loss)]
    let evolved_rate = (evolved_resolved as f64) / (n_lints as f64);
    println!(
        "\n  evolved: {}/{} = {:.1}%\n",
        evolved_resolved,
        n_lints,
        evolved_rate * 100.0
    );

    // ── G7 verdict ────────────────────────────────────────────────────────
    let improvement_pp = (evolved_rate - baseline_rate) * 100.0;
    let g7_pass = improvement_pp >= 5.0;
    println!("── G7 verdict ──\n");
    println!(
        "  baseline:  {:.1}% ({}/{})",
        baseline_rate * 100.0,
        baseline_resolved,
        n_lints
    );
    println!(
        "  evolved:   {:.1}% ({}/{})",
        evolved_rate * 100.0,
        evolved_resolved,
        n_lints
    );
    println!("  improvement: {:.1}pp", improvement_pp);
    println!("  target: ≥5.0pp");
    println!("  G7: {}\n", if g7_pass { "✅ PASS" } else { "❌ FAIL" });

    // ── Summary ───────────────────────────────────────────────────────────
    println!("═══ Summary ═══");
    println!(
        "  G6 (corpus truthfulness): {}",
        if g6_pass { "✅ PASS" } else { "❌ FAIL" }
    );
    println!(
        "  G7 (real-world evolution): {}",
        if g7_pass { "✅ PASS" } else { "❌ FAIL" }
    );
    println!();
    if g6_pass && g7_pass {
        println!(
            "✅ All gates PASS. The self-evolution loop generalizes to real cargo clippy feedback."
        );
        println!("   The controlled G5 fixture result (oracle feedback) is confirmed by G7 (ground-truth feedback).");
    } else {
        println!("❌ Some gates FAILED. See details above.");
    }
}

#[cfg(not(all(feature = "self_evolve", feature = "clippy_verify")))]
fn main() {
    eprintln!(
        "This demo requires both the `self_evolve` and `clippy_verify` features.\n\
         Run with: cargo run --example self_evolve_real_world_demo --features \"self_evolve clippy_verify\" --release"
    );
    std::process::exit(1);
}
