//! Plan 005 T9.4 — pre-flight for K=8 drafts + CLR voting.
//!
//! T9.4 proposes: *"Generate K=8 drafted fixes (currently 1) via QMC
//! perturbation of the ternary matvec, cluster by syntactic similarity
//! (Levenshtein on AST node-kind sequence), run L2 pruner on each, `clr_vote`
//! picks the most-reliable cluster's winner."*
//!
//! Three of that sentence's load-bearing assumptions look wrong on inspection.
//! This measures them before any of it is built, because the last four tasks
//! that were measured first (T3.5.1, T7.1, T7.3, and the Phase 9 pre-flight)
//! each turned out to rest on a premise that no longer held.
//!
//! **A1 — "generate K=8 drafted fixes."** `TernaryDrafter` does not generate
//! code. It BLAKE3-hashes `(rule_id, buggy_code)`, runs a ternary matvec,
//! FNV-1a folds the output into an *index*, and returns that corpus entry's
//! `fixed_code` verbatim (`src/drafter.rs`). A "draft" is a selected corpus
//! row, so K=8 drafts are 8 corpus rows.
//!
//! **A2 — "QMC perturbation" yields useful diversity.** The index comes from
//! an FNV fold over output *bits*. If that is chaotic under small input
//! perturbations, K=8 perturbed drafts are 8 near-uniform random corpus draws,
//! not 8 considered alternatives — sampling noise dressed as an ensemble.
//!
//! **A3 — "run L2 pruner on each" filters anything.** L2 is
//! `RustAnalyzerPruner::validate`: a `syn` parse, with `lint_resolved` hardcoded
//! `true` on the fast path. Every corpus `fixed_code` is valid Rust by
//! construction, so L2 may accept all 8 and contribute zero discrimination.
//!
//! ```bash
//! cargo test --release --test t94_clr_vote_preflight -- --nocapture
//! ```

use std::collections::BTreeSet;

use riir_clippy::domain::{HealDomain, HealEntry};
use riir_clippy::domains::clippy_lints::ClippyLintDomain;
use riir_clippy::drafter::TernaryDrafter;
use riir_clippy::domains::clippy_lints::ClippyFixEntry;
use riir_clippy::pruner::RustAnalyzerPruner;

/// K perturbed drafts for one (rule, code), emulating the QMC sweep by
/// perturbing the drafter's input the only way its public surface allows:
/// varying the seed text. This is generous to the proposal — a real QMC
/// perturbation of the matvec input would be *smaller*, hence at least as
/// chaotic through the FNV fold.
fn perturbed_drafts<E: HealEntry>(
    drafter: &mut TernaryDrafter,
    rule_id: &str,
    buggy: &str,
    corpus: &[E],
    k: usize,
) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut declined = 0usize;
    for i in 0..k {
        let seeded = format!("{buggy}\n// qmc:{i}");
        match drafter.draft_fix(rule_id, &seeded, corpus) {
            Some(d) => out.push(d.code),
            None => declined += 1,
        }
    }
    (out, declined)
}

#[test]
fn t94_preflight() {
    println!("\n=== T9.4 pre-flight — can K=8 drafts + CLR voting work here? ===");

    let corpus = ClippyLintDomain::build_corpus();
    // The L1 matvec path (not the L0 template fallback) — that is the path
    // T9.4 proposes to perturb.
    let weights: &[f32] = &[
        1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 1.0, -1.0,
        -1.0, 1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0,
        0.0, 0.0, 1.0, -1.0, 1.0, -1.0, 1.0, 0.0,
        1.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 1.0,
    ];
    let mut drafter = TernaryDrafter::with_weights(
        katgpt_core::TernaryWeights::quantize_from_f32(weights, 4, 8),
    );
    let pruner = RustAnalyzerPruner::new();
    const K: usize = 8;

    // Probe over real fixtures so the answer is about this system, not a toy.
    // Probe on the corpus's own buggy_code shapes — available without the
    // `latent_retrieval` feature gate, and they are the real inputs the
    // drafter sees.
    let probe: Vec<(String, String)> = corpus
        .iter()
        .take(24)
        .map(|e: &ClippyFixEntry| (e.rule_id().to_string(), e.buggy_code().to_string()))
        .collect();

    let (mut total_drafts, mut total_distinct, mut total_correct_lint) = (0usize, 0usize, 0usize);
    let mut l2_accepted = 0usize;
    let mut cases = 0usize;
    let mut total_declined = 0usize;
    let mut total_attempts = 0usize;

    // Map each corpus fixed_code back to its owning lint, so we can ask whether
    // a draft is even for the RIGHT rule.
    let owner: Vec<(String, String)> = corpus
        .iter()
        .map(|e| (e.fixed_code().to_string(), e.rule_id().to_string()))
        .collect();

    for (lint, code) in &probe {
        let (drafts, declined) = perturbed_drafts(&mut drafter, lint, code, &corpus, K);
        total_declined += declined;
        total_attempts += K;
        if drafts.is_empty() {
            continue; // identity guard declined every perturbation
        }
        cases += 1;
        let distinct: BTreeSet<&String> = drafts.iter().collect();
        total_drafts += drafts.len();
        total_distinct += distinct.len();

        for d in &drafts {
            // A3: does L2 accept it?
            if pruner.validate(d, lint).parses {
                l2_accepted += 1;
            }
            // A2/A1: is this draft even the right lint's fix?
            if owner.iter().any(|(fixed, l)| fixed == d && l == lint) {
                total_correct_lint += 1;
            }
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize, d: usize| if d == 0 { 0.0 } else { k as f32 / d as f32 * 100.0 };
    println!("  probed {} fixtures x K={K} perturbations = {total_attempts} attempts",
             probe.len());
    println!(
        "  IDENTITY GUARD declined  : {total_declined}/{total_attempts} ({:.1}%)  \
         <- collapses the ensemble",
        pct(total_declined, total_attempts)
    );
    println!("  fixtures yielding any draft: {cases}/{}", probe.len());
    println!("  drafts produced          : {total_drafts}");
    println!(
        "  DISTINCT drafts per case : {:.2} of {K}  (A2: diversity)",
        total_distinct as f32 / cases.max(1) as f32
    );
    println!(
        "  L2 pruner ACCEPTED       : {l2_accepted}/{total_drafts} ({:.1}%)  (A3: does L2 filter?)",
        pct(l2_accepted, total_drafts)
    );
    println!(
        "  drafts that are the RIGHT lint's fix: {total_correct_lint}/{total_drafts} ({:.1}%)",
        pct(total_correct_lint, total_drafts)
    );

    println!(
        "  effective K              : {:.2} (proposal asks for 8)",
        total_drafts as f32 / probe.len().max(1) as f32
    );

    println!("\n  -- verdicts --");
    let a3_no_filter = l2_accepted == total_drafts;
    println!(
        "  A3 (L2 filters the ensemble): {}",
        if a3_no_filter {
            "REFUTED — L2 accepts EVERY draft. It is a syn parse and every corpus \
             fixed_code is valid Rust, so the proposed filter step is a no-op by \
             construction."
        } else {
            "holds — L2 rejects some drafts"
        }
    );
    let right_rate = pct(total_correct_lint, total_drafts);
    println!(
        "  A1/A2 (drafts are considered alternatives): {}",
        if right_rate < 50.0 {
            "REFUTED — a majority of drafts are not even the target lint's fix. \
             The index is an FNV fold, so perturbation draws near-uniformly from \
             the corpus; an ensemble over these is sampling noise, and voting \
             over noise cannot beat picking once."
        } else {
            "holds — drafts are mostly on-target"
        }
    );
}
