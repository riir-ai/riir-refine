//! Issue 256 T0/T1 — the fix-trajectory early-read census (thin printer
//! over [`riir_refine::fix_trajectory_census`], the report-only core).
//!
//! REPORT-ONLY and read-only: loads the `.refine/fixseq/` per-attempt
//! rings (paths default to `.` then the sibling repos `corpus_diagnose`
//! sweeps), prints the corpus census, the T0 early-read LORO readout
//! (recall/precision + AUC of "ends non-clean" from the first ~10% of
//! each run), and the T1 macrostate audit's typed verdicts per fold.
//! Exits 1 on a typed refusal (no rings / no multi-attempt runs / single
//! class) — never a green zero. Nothing here gates or wires.
//!
//! ```text
//! cargo run --release --features fix_trajectory_census \
//!     --example fix_trajectory_census -- [PATHS…]
//! ```

use std::path::PathBuf;

use riir_refine::fix_trajectory_census::{
    CensusRefusal, early_run, recall_precision_at_median, t0_scores, t1_audits,
};
use riir_refine::horizon_report::{self, RunRing};

const PROG: &str = "fix-trajectory-census";

fn default_roots() -> Vec<PathBuf> {
    let mut roots = vec![std::env::current_dir().expect("cwd")];
    if let Some(parent) = roots[0].parent().map(PathBuf::from) {
        for name in [
            "katgpt-rs",
            "riir-ai",
            "riir-chain",
            "riir-dapps",
            "riir-game-sdk",
            "riir-infer",
            "riir-instinct",
            "riir-kat",
            "riir-neuron-db",
            "riir-reflex",
            "riir-reflexer",
            "riir-rethink",
            "riir-train",
            "seal-game-editor",
        ] {
            let p = parent.join(name);
            if p.join(".git").exists() {
                roots.push(p);
            }
        }
    }
    roots
}

fn refuse(r: &CensusRefusal) -> ! {
    println!("{PROG}: {r}");
    std::process::exit(1);
}

fn main() {
    let mut paths: Vec<PathBuf> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .collect();
    if paths.is_empty() {
        paths = default_roots();
    }

    let mut all_runs: Vec<RunRing> = Vec::new();
    let mut bad_lines = 0_usize;
    let mut files = 0_usize;
    for root in &paths {
        let (runs, bad, n_files) = horizon_report::load_runs(root);
        files += n_files;
        bad_lines += bad;
        if !runs.is_empty() {
            println!(
                "{PROG}: {}: {} run(s), {bad} bad line(s)",
                root.display(),
                runs.len()
            );
        }
        all_runs.extend(runs);
    }

    if all_runs.is_empty() {
        refuse(&CensusRefusal::NoRings { roots: paths.len() });
    }
    if bad_lines > 0 {
        println!("{PROG}: {bad_lines} unparseable ring line(s) skipped (counted, never folded)");
    }

    let early_runs: Vec<_> = all_runs.iter().filter_map(early_run).collect();
    if early_runs.is_empty() {
        refuse(&CensusRefusal::NoMultiAttempt {
            runs: all_runs.len(),
        });
    }
    let positives = early_runs.iter().filter(|r| r.ends_bad).count();
    if positives == 0 {
        refuse(&CensusRefusal::SingleClass {
            runs: early_runs.len(),
        });
    }

    println!();
    println!("## corpus");
    println!("- files scanned: {files} across {} root(s)", paths.len());
    println!(
        "- runs: {} (multi-attempt: {})",
        all_runs.len(),
        early_runs.len()
    );
    println!("- ends-bad labels: {positives} / {} runs", early_runs.len());
    let mut early_sizes: Vec<(usize, usize)> = early_runs
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut m, r| {
            *m.entry(r.early.len()).or_default() += 1;
            m
        })
        .into_iter()
        .collect();
    early_sizes.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    println!("- early-window sizes: {early_sizes:?}");

    println!();
    println!("## T0 — early-read (LORO ridge over the first ~10% of each run)");
    if let Some(scores) = t0_scores(&early_runs) {
        let n_pos = scores.iter().filter(|(_, y)| *y).count();
        let n_neg = scores.len() - n_pos;
        let auc = horizon_report::auc(&scores);
        let (recall, precision) = recall_precision_at_median(&scores);
        println!(
            "- support: {} folds (pos {n_pos} / neg {n_neg})",
            scores.len()
        );
        println!(
            "- AUC: {}",
            auc.map_or_else(
                || "UNDEFINED (single class)".to_string(),
                |a| format!("{a:.3}")
            )
        );
        println!(
            "- recall@median: {} · precision@median: {}",
            recall.map_or_else(|| "n/a".to_string(), |r| format!("{r:.3}")),
            precision.map_or_else(|| "n/a".to_string(), |p| format!("{p:.3}"))
        );
        println!(
            "- floors: base AUC 0.5 (class prior) · history = the per-strategy \
             Laplace aggregate (the horizon-report's own floor; the early-state \
             score must beat it to credit the early STATE, not the strategy prior)"
        );
    } else {
        println!("- NO measurable fold (every LORO fit degenerate)");
    }

    println!();
    println!(
        "## T1 — macrostate audit (katgpt-core closure_audit, 41-D summary vs 32-D embed microstate)"
    );
    let audits = t1_audits(&early_runs);
    if audits.is_empty() {
        println!("- no fold reached 2 transition windows — the audit is unserved");
    } else {
        for (source, report) in &audits {
            match report {
                Ok(r) => println!(
                    "- {source}: {:?} · summary R² {:.3} · aug {} · gap {} · windows {}",
                    r.verdict,
                    r.summary_best_r2,
                    r.augmented_best_r2
                        .map_or_else(|| "n/a".to_string(), |v| format!("{v:.3}")),
                    r.gap
                        .map_or_else(|| "n/a".to_string(), |v| format!("{v:.3}")),
                    r.support.windows
                ),
                Err(e) => println!("- {source}: REFUSED — {e}"),
            }
        }
        let closed = audits
            .iter()
            .filter(|(_, r)| {
                matches!(
                    r,
                    Ok(x) if matches!(x.verdict, katgpt_core::closure_audit::ClosureVerdict::Closed { .. })
                )
            })
            .count();
        let lossy = audits
            .iter()
            .filter(|(_, r)| {
                matches!(
                    r,
                    Ok(x) if matches!(x.verdict, katgpt_core::closure_audit::ClosureVerdict::Lossy { .. })
                )
            })
            .count();
        let undecidable = audits.len() - closed - lossy;
        println!(
            "- pooled: {closed} Closed · {lossy} Lossy · {undecidable} Undecidable/refused \
             (per-fold reports above; the primitive's pinned bars decide each)"
        );
    }

    println!();
    println!("{PROG}: census complete (report-only — nothing gated, nothing wired)");
}
