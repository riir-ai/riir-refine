//! Issue 122 / Plan 145 — the frontier-miner real-oracle e2e (the
//! `interacting_gate.rs` precedent: admissibility is MEASURED, never
//! asserted). Runs both admission modes against REAL `cargo clippy`:
//!
//! 1. **Seed-clean gate** — every family seed must report ZERO clippy lints
//!    under the full all+pedantic+nursery oracle (a dirty seed is a broken
//!    family table; reported, never silently skipped — the
//!    `corpus_interacting` lesson; the seeds are deliberately non-const-able
//!    so nursery `missing_const_for_fn` stays silent).
//! 2. **`HeldOut` pass** — EVERY covered family runs its own held-out loop
//!    (view = the family's rule disabled, solution = re-enable; Batch 155
//!    graduated five frontier leads, Batch 156 one, Batch 157 BOTH pack III
//!    leads, Batch 158 BOTH pack IV leads — nine covered families alongside
//!    the original `needless_return`);
//!    the full invariant loop (unsolved-before /
//!    solved-after via re-enable / regression over all stock rows /
//!    exactly-once vs the committed store) runs with oracle verdicts at
//!    every judgment point.
//! 3. **Frontier pass** — the current uncovered-lint families
//!    (`single_char_add_str` + `manual_clamp`, the Batch 158 re-arm); each
//!    admissible row is
//!    a PENDING mining lead (clause 3 records `solving_rule: None`).
//! 4. **Committed-row verification** — rows already in the store are
//!    RE-VERIFIED against the live oracle (clauses 1–3 re-checked on the
//!    committed fixture, against the ROW's own family rule): a committed row
//!    that no longer holds under the current toolchain is a RED (the
//!    freeze-gate drift discipline), not a silent skip. Reruns are therefore
//!    verify-passes, never stale failures.
//! 5. **Steering report + band fold** — per family: what fired, whether the
//!    target is in the compiled fix set; the band fold covers committed +
//!    fresh rows (the synthetic feed accumulates).
//!
//! Rows commit ONLY after the verdict passes (the first e2e run's lesson:
//! a failed run must leave no rows behind).
//!
//! Exit 0 iff: seeds clean ∧ EVERY covered family's loop holds (fresh or
//! re-verified) ∧ ≥1 frontier lead (fresh or re-verified).
//!
//! ```text
//! cargo run --features frontier_miner --example frontier_miner
//! ```
//!
//! Writes/reads `.benchmarks/frontier_miner_rows.jsonl` relative to CWD
//! (run from the repo root). CPU-only; one clippy spawn per judgment
//! (BLAKE3-keyed temp crates make repeats cheap).

use std::collections::BTreeSet;

use riir_clippy::draft::clippy_oracle::OracleVerdict;
use riir_clippy::frontier_miner::{
    admit, append_rows, apply_solution, band_rows, generate, heal_under_view, load_known,
    AdmittedRow, CorpusView, MinerRow, Mode, Rejection, Solution, MINER_ROWS_JSONL,
};
use riir_clippy::frontier_pairs::PROVENANCE;
use riir_clippy::frontier_report;
use riir_clippy::score_bench::draft_oracle;

fn main() {
    let oracle = draft_oracle();
    if !oracle.is_available() {
        eprintln!("⛔ cargo not reachable — the e2e needs real clippy (the oracle IS the verdict)");
        std::process::exit(1);
    }
    let probe = |code: &str| oracle.lints_for(code);

    println!("== frontier miner e2e (Issue 122 / Plan 145) ==");
    println!("oracle: real cargo clippy (all+pedantic+nursery), temp-crate spawns\n");

    let all = generate(&[Mode::HeldOut, Mode::Frontier]);
    let path = std::path::Path::new(MINER_ROWS_JSONL);
    let known = load_known(path);

    // ── 1. Seed-clean gate (measured) ────────────────────────────────────
    let mut dirty_seeds = 0usize;
    for c in &all {
        let v = probe(&c.seed);
        if v.ran && v.compiles && !v.lints.is_empty() {
            dirty_seeds += 1;
            eprintln!(
                "⛔ seed NOT clean (family {}): clippy fired {:?}",
                c.family,
                v.lints.iter().collect::<Vec<_>>()
            );
        }
    }
    println!("seed-clean gate: {} seeds checked, {} dirty", all.len(), dirty_seeds);

    // ── 2+3. Fresh admission (both modes, exactly-once vs the store) ─────
    // Batch 155: EVERY covered family runs its own held-out loop (view =
    // its rule disabled, solution = re-enable) — the graduated rules
    // re-prove unsolved-before/solved-after/regression on
    // every e2e rerun, alongside the needless_return original.
    let covered: Vec<_> = all.iter().filter(|c| c.mode == Mode::HeldOut).cloned().collect();
    let heldout_families: Vec<String> = covered
        .iter()
        .map(|c| c.family.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut heldout_fresh = 0usize;
    let mut heldout_rejections = 0usize;
    let mut heldout_admitted: Vec<AdmittedRow> = Vec::new();
    let mut heldout_admitted_families: BTreeSet<String> = BTreeSet::new();
    for fam in &heldout_families {
        let fam_cands: Vec<_> = covered.iter().filter(|c| c.family == fam.as_str()).cloned().collect();
        let sol = Solution::Reenable(fam_target_lint(fam));
        let view = CorpusView::holding_out(&fam_target_lint(fam));
        let pass = admit(&fam_cands, &view, Some(&sol), &known, &probe);
        heldout_rejections += pass.rejections.len();
        heldout_fresh += pass.admitted.len();
        for row in &pass.admitted {
            heldout_admitted_families.insert(row.family.to_string());
        }
        heldout_admitted.extend(pass.admitted);
        if !pass.rejections.is_empty() {
            report(&format!("HeldOut[{fam}]"), &pass.rejections);
        }
    }
    println!(
        "HeldOut: {} family loop(s), {} fresh admission(s), {} rejection(s)",
        heldout_families.len(),
        heldout_fresh,
        heldout_rejections
    );

    let frontier_cands: Vec<_> = all.iter().filter(|c| c.mode == Mode::Frontier).cloned().collect();
    let frontier = admit(&frontier_cands, &CorpusView::default(), None, &known, &probe);
    report("Frontier", &frontier.rejections);

    // ── 4. Committed-row verification (reruns are verify-passes) ─────────
    let committed = load_rows(path);
    let mut verify_failures = 0usize;
    for row in &committed {
        if !verify_committed(row, &probe) {
            verify_failures += 1;
            eprintln!("⛔ committed row {} no longer holds — toolchain drift?", row.blake3);
        }
    }
    println!(
        "committed rows: {} verified, {} drifted",
        committed.len().saturating_sub(verify_failures),
        verify_failures
    );

    // ── 5. Steering + band fold (committed + fresh) ──────────────────────
    let fix_set: BTreeSet<String> = compiled_fix_set();
    println!("\nsteering (target lint → in compiled fix set?):");
    for row in frontier.admitted.iter() {
        println!(
            "  {:<22} lint={:<18} covered={} lints_before={:?}",
            row.family,
            row.target_lint,
            fix_set.contains(row.target_lint),
            row.lints_before
        );
    }

    let mut fold_rows = band_rows(&heldout_admitted);
    for row in committed.iter().filter(|r| r.solving_rule.is_some()) {
        if let Some(rule) = &row.solving_rule {
            fold_rows.push((rule.clone(), 1, 1));
        }
    }
    let summary = frontier_report::summarize(&fold_rows);
    println!(
        "\nband fold: population={} seen={} band_count={} band_mass={:.3} (provenance={})",
        summary.population, summary.seen, summary.band_count, summary.band_mass, PROVENANCE
    );

    // ── Verdict, THEN commit (a failed run leaves no rows) ───────────────
    // Every covered family must hold: a fresh admission THIS run or a
    // re-verified committed row (verify_failures == 0 covers the latter).
    let mut holding_families: BTreeSet<String> = heldout_admitted_families;
    for row in &committed {
        if row.solving_rule.is_some() {
            holding_families.insert(row.family.clone());
        }
    }
    let heldout_ok = holding_families.len() == heldout_families.len();
    let frontier_yield = frontier.admitted.len()
        + committed
            .iter()
            .filter(|r| r.solving_rule.is_none())
            .count();

    let mut fresh = heldout_admitted;
    fresh.extend(frontier.admitted.iter().cloned());
    if heldout_ok && dirty_seeds == 0 && verify_failures == 0 && frontier_yield >= 1 {
        if fresh.is_empty() {
            println!("\nstore: no new rows (families already committed this epoch)");
        } else {
            append_rows(path, &fresh).expect("append miner rows");
            println!("store: appended {} row(s) to {}", fresh.len(), MINER_ROWS_JSONL);
        }
        println!("\n✓ e2e VERDICT: invariant loop holds; frontier yield {frontier_yield} lead(s)");
    } else {
        eprintln!(
            "\n✗ e2e VERDICT: heldout_ok={heldout_ok} dirty_seeds={dirty_seeds} verify_failures={verify_failures} frontier_yield={frontier_yield} — no rows committed"
        );
        std::process::exit(1);
    }
}

/// The dispatch rule a family holds out: the family's target lint (every
/// covered family's rule is dispatched under its lint name).
fn fam_target_lint(family: &str) -> String {
    family.trim_end_matches("-heldout").to_string()
}

/// Re-verify a committed row against the live oracle (the drift gate).
fn verify_committed(row: &MinerRow, probe: &dyn Fn(&str) -> OracleVerdict) -> bool {
    // Re-derive the fixture's candidate form from the current tables.
    let Some(cand) = generate(&[Mode::HeldOut, Mode::Frontier])
        .into_iter()
        .find(|c| c.blake3 == row.blake3)
    else {
        // The fixture text itself changed (table edit) — regenerate by
        // family is the honest fallback: the FAMILY's current canonical
        // first candidate must still be admissible.
        return false;
    };
    let seed_v = probe(&cand.seed);
    if !(seed_v.ran && seed_v.compiles && seed_v.lints.is_empty()) {
        return false;
    }
    let broken_v = probe(&cand.broken);
    if !(broken_v.ran && broken_v.compiles && broken_v.lints.contains(cand.target_lint)) {
        return false;
    }
    if cand.mode == Mode::HeldOut {
        // Unsolved-by-view + solved-by-solution, re-judged against the
        // ROW's own family rule (Batch 155: one loop per covered family).
        let rule = cand.target_lint.to_string();
        let sol = Solution::Reenable(rule.clone());
        let (_healed, cleared) = apply_solution(&cand, &sol, probe);
        let (under_view, _, _) = heal_under_view(&cand, &CorpusView::holding_out(&rule));
        let view_v = probe(&under_view);
        let unsolved = !view_v.ran || !view_v.compiles || view_v.lints.contains(cand.target_lint);
        unsolved && cleared
    } else {
        // Frontier row: still unhealable by the default corpus.
        let (healed_default, _, _) = heal_under_view(&cand, &CorpusView::default());
        healed_default == cand.broken
    }
}

fn load_rows(path: &std::path::Path) -> Vec<MinerRow> {
    std::fs::read_to_string(path)
        .map(|t| t.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default()
}

fn report(mode: &str, rejections: &[Rejection]) {
    use riir_clippy::frontier_miner::Rejection::*;
    println!("\n{mode} rejections ({}):", rejections.len());
    for r in rejections {
        match r {
            Duplicate { family, blake3 } => println!("  {family}: duplicate {blake3}"),
            SeedNotClean { family, saw } => println!("  {family}: seed NOT clean, saw {saw:?}"),
            TargetLintNotFired { family, saw } => println!("  {family}: target lint not fired, saw {saw:?}"),
            BrokenNotVerifiable { family, why } => println!("  {family}: broken not verifiable ({why})"),
            HealedByView { family } => println!("  {family}: healed by the view — not at the frontier"),
            SolutionDidNotClear { family } => println!("  {family}: solution did not clear"),
            RegressionRefused { family, perturbed } => println!("  {family}: REGRESSION refused, perturbed {perturbed:?}"),
        }
    }
}

/// `FIXABLE_LINTS ∪ VERIFY_LINTS` — the compiled fix lane's truth (the
/// `corpus_coverage` vocabulary; steers which frontier leads are genuinely
/// uncovered).
fn compiled_fix_set() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for lint in riir_clippy::domains::clippy_lints::proposers::FIXABLE_LINTS {
        set.insert((*lint).to_string());
    }
    for lint in riir_clippy::domains::clippy_lints::proposers::VERIFY_LINTS {
        set.insert((*lint).to_string());
    }
    set
}
