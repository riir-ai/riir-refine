//! Issue 257 T4 — the oracle-quiet BACKFILL census runner (report-only).
//!
//! Opens a `.refine/trajectories.kv` store read-only-in-spirit (the store's
//! own open path; the census WRITES NOTHING) and classifies every stored
//! trajectory row by the span-free shapes (attribute-insertion /
//! stub-replacement) on the row's own stored texts. Prints the per-rule
//! table + the corrected-figure baseline (PRIMARY exact subtraction,
//! Noeri's ratio cross-check) — the verdict's pre-registered demotion
//! threshold reads THIS output, never the store.
//!
//! ```text
//! cargo run --example oracle_quiet_backfill -- [path-to-refine-root]
//! ```

use std::path::PathBuf;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let (store, _bad) = riir_refine::traj_store::TrajStore::open(&root).unwrap_or_else(|e| {
        eprintln!(
            "backfill: cannot open {}/.refine/trajectories.kv: {e}",
            root.display()
        );
        std::process::exit(2);
    });
    let (judged, rows) = store.backfill_census();
    let total_hacked: usize = rows.iter().map(|(_, _, h)| h).sum();
    let credited_note = "the store carries verdict counters, not credits — credited_nonhack is a FIX-RUN figure and is not backfillable (disclosed)";
    println!(
        "oracle-quiet backfill (issue 257 T4, report-only): {judged} row(s) judged, {total_hacked} hacked"
    );
    if rows.is_empty() {
        println!("(no rows)");
    }
    for (rule, j, h) in &rows {
        println!("  {rule:<40} judged {j:>5}  hacked {h:>5}");
    }
    println!("note: {credited_note}");
    // The verdict's pre-registered demotion threshold (set BEFORE this
    // run, in issue 257): a rule with ≥ 30% hacked share over ≥ 5 judged
    // rows is a demotion CANDIDATE — hand-checked, then the existing
    // active-set demote command; never a blanket action.
    const SHARE_PCT: f64 = 30.0;
    const MIN_JUDGED: usize = 5;
    let candidates: Vec<&(String, usize, usize)> = rows
        .iter()
        .filter(|(_, j, h)| *j >= MIN_JUDGED && (*h as f64) / (*j as f64) * 100.0 >= SHARE_PCT)
        .collect();
    if candidates.is_empty() {
        println!(
            "demotion candidates (≥{SHARE_PCT}% share over ≥{MIN_JUDGED} judged): none — audited, 0 demotions"
        );
    } else {
        for (rule, j, h) in candidates {
            println!(
                "demotion candidate: {rule} ({h}/{j} = {:.1}% — hand-check a sample before the active-set demote)",
                *h as f64 / *j as f64 * 100.0
            );
        }
    }
}
