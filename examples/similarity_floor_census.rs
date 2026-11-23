//! Issue 250 T1 — the retrieval-similarity floor census (REPORT-ONLY, exit 0
//! on every successful read; `--self-test` pins the arithmetic and fails loud).
//!
//! T1b's per-edit event log (`.refine/edit_events.jsonl`, landed 2026-11-20)
//! records one row per edit the fix-verify batch attempted. This instrument
//! reads one or more of those logs, groups rows into runs by (source, run_id),
//! splits runs into FIT / EVAL at the median timestamp, and scores every EVAL
//! edit's (restore → replacement) pair against the FIT accepted fixes by
//! char-trigram Jaccard — the modelless similarity floor the issue runs FIRST
//! (if it clears the derived G3 bar, the trained arms never run).
//!
//! Policy per τ: decline an edit iff its best similarity < τ. Unscoreable
//! edits (no applicable reference) and degenerate rows (both texts blank) are
//! never declined. The cost simulation reuses T1a's semantics so the numbers
//! are comparable with Bench 162: a run whose every remaining edit is declined
//! costs 0 checks; one whose remaining edits all historically survived costs
//! the canonical 2 (baseline + one good round); otherwise the run's actual
//! `run_checks`. The headline reduction excludes runs spending more than 5×
//! the median `run_checks` (T1a's outlier-excluded bar posture; both columns
//! print).
//!
//! Two reference variants, never pooled: GLOBAL (every accepted fix) and RULE
//! (same-rule accepted fixes only — rules unseen in FIT are never declined,
//! the T1a law carried over).
//!
//! Bars (pre-registered, Bench 162): G3 ≥ 10% realized check reduction ·
//! false declines ≤ 1% of kept edits · and the floor must beat the 3.9% the
//! per-rule prior realized. PASS ⇒ the modelless floor ships and the trained
//! arms (T2/T3) stay unopened. The honest verdicts (FAIL / no-τ-adjudicable)
//! are first-class outcomes, never a pass.
//!
//! Data gate: the log accrues only from real fix runs since 2026-11-20 —
//! below `--min-runs` (default 4) the census prints the accumulation note and
//! exits 0. `--self-test` runs the whole policy on synthetic rows with a
//! planted separation and asserts the exact expected numbers (both variants
//! FAIL there, by design — it pins the arithmetic, not a pass).
//!
//! Run:
//! ```text
//! cargo run --release --example similarity_floor_census -- [--self-test] \
//!   [--events <path/to/.refine/edit_events.jsonl>]... [--min-runs <n>] \
//!   [--tau <t>] [REPO_ROOT]...
//! ```
//! A bare positional ROOT reads `<ROOT>/.refine/edit_events.jsonl` (missing
//! per-root logs are a loud skip); a missing `--events` file refuses exit 2.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use riir_refine::blueprint::{EDIT_EVENT_VERSION, EditEventRow};

/// τ sweep, ascending; ties keep the first τ (the fixed tie rule).
const TAU_SWEEP: [f64; 11] = [0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.75, 0.9];
/// T0-derived G3 bar: realized check reduction (Bench 162).
const G3_BAR: f64 = 0.10;
/// False-decline bar: fraction of kept edits a wrong decline may burn.
const FALSE_DECLINE_BAR: f64 = 0.01;
/// T1a's realized number (Bench 162 addendum) — the floor this arm must beat.
const T1A_PRIOR: f64 = 0.039;
/// Below this many runs the FIT/EVAL split is not adjudicable (honest zero).
const MIN_RUNS: usize = 4;
/// A run spending more than OUTLIER_MULT × the median `run_checks` leaves the
/// headline reduction (T1a's outlier-excluded posture); both columns print.
const OUTLIER_MULT: f64 = 5.0;
/// T1a's canonical surviving-batch cost (baseline + one good round).
const CANONICAL_SURVIVE_CHECKS: u64 = 2;

const USAGE: &str = "usage: similarity_floor_census [--self-test] \
[--events <edit_events.jsonl>]... [--min-runs <n>] [--tau <t>] [REPO_ROOT]...";

/// FNV-1a over three chars' UTF-32 little-endian bytes (deterministic, no
/// external dep; collisions only thin the trigram signal, never invert it).
fn tri_key(a: char, b: char, c: char) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for ch in [a, b, c] {
        for byte in u32::from(ch).to_le_bytes() {
            h ^= u32::from(byte);
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h
}

fn trigrams(text: &str) -> BTreeSet<u32> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 3 {
        return BTreeSet::new();
    }
    chars
        .windows(3)
        .map(|w| tri_key(w[0], w[1], w[2]))
        .collect()
}

fn jaccard(a: &BTreeSet<u32>, b: &BTreeSet<u32>) -> f64 {
    let inter = a.intersection(b).count();
    let union = a.len() + b.len() - inter;
    if union == 0 {
        return 0.0;
    }
    f64::from(u32::try_from(inter).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(union).unwrap_or(u32::MAX))
}

/// The similarity arm's input: the diff pair, delimited (NUL cannot appear in
/// source text, so the join is injective).
fn pair_text(row: &EditEventRow) -> String {
    format!("{}\u{0}{}", row.restore, row.replacement)
}

fn degenerate(row: &EditEventRow) -> bool {
    row.restore.trim().is_empty() && row.replacement.trim().is_empty()
}

fn median(sorted: &[f64]) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        Some(sorted[mid])
    } else {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    }
}

// ── loading ──────────────────────────────────────────────────────────────────

struct SourceLog {
    name: String,
    rows: Vec<EditEventRow>,
    malformed: usize,
    other_version: usize,
}

fn load_file(path: &Path) -> std::io::Result<SourceLog> {
    let text = std::fs::read_to_string(path)?;
    let mut rows = Vec::new();
    let mut malformed = 0usize;
    let mut other_version = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<EditEventRow>(line) {
            Ok(row) if row.v == EDIT_EVENT_VERSION => rows.push(row),
            Ok(_) => other_version += 1,
            Err(_) => malformed += 1,
        }
    }
    Ok(SourceLog {
        name: path.display().to_string(),
        rows,
        malformed,
        other_version,
    })
}

/// Positional roots resolve to `<ROOT>/.refine/edit_events.jsonl`; a missing
/// per-root log is a loud skip (the normal posture for repos without fix
/// runs). Explicit `--events` files must exist (exit 2 otherwise).
fn collect_logs(events: &[PathBuf], roots: &[PathBuf]) -> Vec<SourceLog> {
    let mut logs = Vec::new();
    for path in events {
        match load_file(path) {
            Ok(log) => logs.push(log),
            Err(e) => {
                eprintln!("refusal: --events {} unreadable: {e}", path.display());
                std::process::exit(2);
            }
        }
    }
    for root in roots {
        let path = root.join(".refine").join("edit_events.jsonl");
        if !path.exists() {
            println!("skip: {} (no .refine/edit_events.jsonl)", root.display());
            continue;
        }
        match load_file(&path) {
            Ok(log) => logs.push(log),
            Err(e) => println!("skip: {} unreadable: {e}", path.display()),
        }
    }
    logs
}

// ── runs ─────────────────────────────────────────────────────────────────────

/// One verify batch. Run identity is (source, run_id) — run_ids are only
/// unique inside one log. Rows are sorted for a deterministic edit order.
struct Run {
    key: String,
    ts: u64,
    edits: Vec<EditEventRow>,
    checks_drift: usize,
}

fn group_runs(logs: &[SourceLog]) -> Vec<Run> {
    let mut by: BTreeMap<(String, String), Vec<EditEventRow>> = BTreeMap::new();
    for log in logs {
        for row in &log.rows {
            by.entry((log.name.clone(), row.run_id.clone()))
                .or_default()
                .push(row.clone());
        }
    }
    let mut runs: Vec<Run> = by
        .into_iter()
        .map(|((source, run_id), mut rows)| {
            rows.sort_by(|a, b| (a.line, &a.rule, &a.path).cmp(&(b.line, &b.rule, &b.path)));
            let ts = rows.iter().map(|r| r.ts).min().unwrap_or(0);
            let (re, rc) = rows
                .first()
                .map(|r| (r.run_edits, r.run_checks))
                .unwrap_or((0, 0));
            let checks_drift = rows
                .iter()
                .filter(|r| r.run_edits != re || r.run_checks != rc)
                .count();
            Run {
                key: format!("{source}::{run_id}"),
                ts,
                edits: rows,
                checks_drift,
            }
        })
        .collect();
    runs.sort_by(|a, b| (a.ts, &a.key).cmp(&(b.ts, &b.key)));
    runs
}

// ── the floor policy ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct TauRow {
    tau: f64,
    declines: usize,
    correct: usize,
    false_declines: usize,
    /// None when the kept denominator is 0 (n/a — never folded into 0).
    false_rate: Option<f64>,
    sim_checks: u64,
    actual_checks: u64,
    reduction_all: f64,
    reduction_ex: f64,
    outlier_runs: usize,
}

#[derive(Debug)]
struct Eval {
    fit_runs: usize,
    eval_runs: usize,
    fit_accepted: usize,
    eval_edits: usize,
    eval_kept: usize,
    eval_reverted: usize,
    degenerate: usize,
    unscoreable: usize,
    score_min: Option<f64>,
    score_median: Option<f64>,
    score_max: Option<f64>,
    run_checks_median: f64,
    rows: Vec<TauRow>,
    /// Index into [`TAU_SWEEP`] of the best false-bar-clearing τ (ties keep
    /// the first), or None when no τ is adjudicable.
    best: Option<usize>,
}

fn reduction(sim: u64, actual: u64) -> f64 {
    if actual == 0 {
        return 0.0;
    }
    1.0 - sim as f64 / actual as f64
}

/// The similarity floor over `runs` (FIT = first half, EVAL = rest, split at
/// the run median — never mid-run, so labels cannot leak across the split).
fn evaluate(runs: &[Run], rule_gated: bool) -> Eval {
    let fit = &runs[..runs.len() / 2];
    let eval = &runs[runs.len() / 2..];

    let reference: Vec<(String, BTreeSet<u32>)> = fit
        .iter()
        .flat_map(|r| r.edits.iter())
        .filter(|e| !e.reverted && !degenerate(e))
        .map(|e| (e.rule.clone(), trigrams(&pair_text(e))))
        .collect();
    let fit_accepted = reference.len();

    // Per-eval-run best similarities (None = unscoreable / degenerate → kept).
    let mut eval_scores: Vec<Vec<Option<f64>>> = Vec::new();
    let mut sims_all: Vec<f64> = Vec::new();
    let mut degenerate_rows = 0usize;
    let mut unscoreable = 0usize;
    for run in eval {
        let mut sims: Vec<Option<f64>> = Vec::with_capacity(run.edits.len());
        for e in &run.edits {
            let score = if degenerate(e) {
                degenerate_rows += 1;
                None
            } else {
                let tri = trigrams(&pair_text(e));
                let best = reference
                    .iter()
                    .filter(|(rule, _)| !rule_gated || *rule == e.rule)
                    .map(|(_, t)| jaccard(&tri, t))
                    .fold(None::<f64>, |m, s| Some(m.unwrap_or(0.0).max(s)));
                if best.is_none() {
                    unscoreable += 1;
                }
                best
            };
            if let Some(s) = score {
                sims_all.push(s);
            }
            sims.push(score);
        }
        eval_scores.push(sims);
    }

    let mut checks: Vec<f64> = eval
        .iter()
        .map(|r| f64::from(r.edits.first().map(|e| e.run_checks).unwrap_or(0)))
        .collect();
    checks.sort_by(|a, b| a.total_cmp(b));
    let run_checks_median = median(&checks).unwrap_or(0.0);

    let eval_edits: usize = eval.iter().map(|r| r.edits.len()).sum();
    let eval_kept = eval
        .iter()
        .flat_map(|r| r.edits.iter())
        .filter(|e| !e.reverted)
        .count();
    let eval_reverted = eval_edits - eval_kept;

    let mut rows = Vec::with_capacity(TAU_SWEEP.len());
    for tau in TAU_SWEEP {
        let mut sim_checks = 0u64;
        let mut actual_checks = 0u64;
        let mut sim_checks_ex = 0u64;
        let mut actual_checks_ex = 0u64;
        let mut outlier_runs = 0usize;
        let mut declines = 0usize;
        let mut correct = 0usize;
        let mut false_declines = 0usize;
        for (run, sims) in eval.iter().zip(&eval_scores) {
            let act = u64::from(run.edits.first().map(|e| e.run_checks).unwrap_or(0));
            let mut remaining = 0usize;
            let mut remaining_all_survive = true;
            for (e, sim) in run.edits.iter().zip(sims) {
                let decline = matches!(sim, Some(s) if *s < tau);
                if decline {
                    declines += 1;
                    if e.reverted {
                        correct += 1;
                    } else {
                        false_declines += 1;
                    }
                } else {
                    remaining += 1;
                    if e.reverted {
                        remaining_all_survive = false;
                    }
                }
            }
            let simc = if remaining == 0 {
                0
            } else if remaining_all_survive {
                CANONICAL_SURVIVE_CHECKS
            } else {
                act
            };
            let outlier = run_checks_median > 0.0 && act as f64 > OUTLIER_MULT * run_checks_median;
            if outlier {
                outlier_runs += 1;
            } else {
                sim_checks_ex += simc;
                actual_checks_ex += act;
            }
            sim_checks += simc;
            actual_checks += act;
        }
        rows.push(TauRow {
            tau,
            declines,
            correct,
            false_declines,
            false_rate: if eval_kept == 0 {
                None
            } else {
                Some(false_declines as f64 / eval_kept as f64)
            },
            sim_checks,
            actual_checks,
            reduction_all: reduction(sim_checks, actual_checks),
            reduction_ex: reduction(sim_checks_ex, actual_checks_ex),
            outlier_runs,
        });
    }

    // Best τ: the false-bar-clearing rows, max excluded reduction, ties keep
    // the first sweep entry.
    let best = rows.iter().enumerate().fold(None::<usize>, |acc, (i, r)| {
        if !r.false_rate.is_some_and(|fr| fr <= FALSE_DECLINE_BAR) {
            return acc;
        }
        match acc {
            None => Some(i),
            Some(j) if r.reduction_ex > rows[j].reduction_ex => Some(i),
            _ => acc,
        }
    });

    sims_all.sort_by(|a, b| a.total_cmp(b));
    Eval {
        fit_runs: fit.len(),
        eval_runs: eval.len(),
        fit_accepted,
        eval_edits,
        eval_kept,
        eval_reverted,
        degenerate: degenerate_rows,
        unscoreable,
        score_min: sims_all.first().copied(),
        score_median: median(&sims_all),
        score_max: sims_all.last().copied(),
        run_checks_median,
        rows,
        best,
    }
}

/// `(passed, named reasons)` — reasons name the miss; an empty list is PASS.
fn verdict(eval: &Eval) -> (bool, Vec<String>) {
    let Some(idx) = eval.best else {
        let mut reasons = Vec::new();
        if let Some(min_fr) = eval
            .rows
            .iter()
            .filter_map(|r| r.false_rate)
            .min_by(f64::total_cmp)
        {
            if min_fr > FALSE_DECLINE_BAR {
                reasons.push(format!(
                    "no tau clears the false-decline bar (min {:.2}% > {:.0}%)",
                    min_fr * 100.0,
                    FALSE_DECLINE_BAR * 100.0
                ));
            }
        }
        if reasons.is_empty() {
            reasons.push("no tau adjudicable (no reference / no kept edits)".into());
        }
        return (false, reasons);
    };
    let best = &eval.rows[idx];
    let mut reasons = Vec::new();
    if best.reduction_ex < G3_BAR {
        reasons.push(format!(
            "G3 {:.1}% < {:.0}%",
            best.reduction_ex * 100.0,
            G3_BAR * 100.0
        ));
    }
    if best.false_rate.is_some_and(|fr| fr > FALSE_DECLINE_BAR) {
        reasons.push(format!(
            "false-decline {:.2}% > {:.0}%",
            best.false_rate.unwrap_or(0.0) * 100.0,
            FALSE_DECLINE_BAR * 100.0
        ));
    }
    if best.reduction_ex <= T1A_PRIOR {
        reasons.push(format!(
            "at/below the {:.1}% per-rule prior",
            T1A_PRIOR * 100.0
        ));
    }
    (reasons.is_empty(), reasons)
}

fn print_eval(name: &str, eval: &Eval) {
    println!("-- {name} reference --");
    println!(
        "fit runs {} (accepted fixes {}) · eval runs {} · edits {} (kept {}, reverted {}, degenerate {}, unscoreable {})",
        eval.fit_runs,
        eval.fit_accepted,
        eval.eval_runs,
        eval.eval_edits,
        eval.eval_kept,
        eval.eval_reverted,
        eval.degenerate,
        eval.unscoreable,
    );
    if let (Some(lo), Some(med), Some(hi)) = (eval.score_min, eval.score_median, eval.score_max) {
        println!("score spread: min {lo:.3} · median {med:.3} · max {hi:.3}");
    }
    println!(
        "  tau  decl  corr false  false%   sim actual   red%  red_ex%  (outlier = > {:.0}x median {:.0})",
        OUTLIER_MULT, eval.run_checks_median,
    );
    for r in &eval.rows {
        let fr = match r.false_rate {
            Some(v) => format!("{:.2}", v * 100.0),
            None => "n/a".into(),
        };
        println!(
            " {:>5.2} {:>5} {:>5} {:>5} {:>7} {:>5} {:>6} {:>6.1} {:>8.1}",
            r.tau,
            r.declines,
            r.correct,
            r.false_declines,
            fr,
            r.sim_checks,
            r.actual_checks,
            r.reduction_all * 100.0,
            r.reduction_ex * 100.0,
        );
    }
    let (passed, reasons) = verdict(eval);
    match eval.best {
        Some(idx) => {
            let best = &eval.rows[idx];
            println!(
                "best tau {:.2}: red_ex {:.1}% (all-runs {:.1}%, {} outlier run(s)) · false {}",
                best.tau,
                best.reduction_ex * 100.0,
                best.reduction_all * 100.0,
                best.outlier_runs,
                match best.false_rate {
                    Some(v) => format!("{:.2}%", v * 100.0),
                    None => "n/a".into(),
                },
            );
        }
        None => println!("best tau: none adjudicable"),
    }
    if passed {
        println!("floor verdict: PASS — the similarity floor clears the bars");
    } else {
        println!("floor verdict: FAIL — {}", reasons.join("; "));
    }
    println!(
        "prior comparison: T1a per-rule prior realized {:.1}% — this floor's best {:.1}%",
        T1A_PRIOR * 100.0,
        eval.best
            .map(|i| eval.rows[i].reduction_ex * 100.0)
            .unwrap_or(f64::NAN),
    );
}

// ── self-test ────────────────────────────────────────────────────────────────

/// Families are chosen so the pair texts are trigram-disjoint across families
/// (A/B/C letters never share a trigram) — similarities are exactly 0.0 or
/// 1.0 and every expectation is hand-computable.
#[allow(clippy::too_many_arguments)]
fn srow(
    ts: u64,
    run_id: &str,
    rule: &str,
    line: usize,
    reverted: bool,
    run_edits: u32,
    run_checks: u32,
    restore: &str,
    replacement: &str,
) -> EditEventRow {
    EditEventRow {
        v: EDIT_EVENT_VERSION,
        ts,
        run_id: run_id.into(),
        path: "src/lib.rs".into(),
        rule: rule.into(),
        line,
        reverted,
        whole_file: false,
        run_edits,
        run_checks,
        restore: restore.into(),
        replacement: replacement.into(),
    }
}

const A_RESTORE: &str = "AAAA BBB";
const A_REPLACE: &str = "AAA CCC";
const B1_RESTORE: &str = "GGG HHH";
const B1_REPLACE: &str = "GGG III";
const B2_RESTORE: &str = "JJJ KKK";
const B2_REPLACE: &str = "JJJ LLL";
const C_RESTORE: &str = "NNN OOO";
const C_REPLACE: &str = "NNN PPP";

fn self_test() {
    let runs = vec![
        // FIT: 3 accepted fixes (A, A, C) — the reference.
        Run {
            key: "t::r1".into(),
            ts: 1,
            edits: vec![
                srow(
                    1,
                    "r1",
                    "dead_code_remove",
                    1,
                    false,
                    2,
                    2,
                    A_RESTORE,
                    A_REPLACE,
                ),
                srow(
                    1,
                    "r1",
                    "dead_code_remove",
                    2,
                    false,
                    2,
                    2,
                    A_RESTORE,
                    A_REPLACE,
                ),
            ],
            checks_drift: 0,
        },
        Run {
            key: "t::r2".into(),
            ts: 2,
            edits: vec![srow(
                2,
                "r2",
                "needless_borrow",
                1,
                false,
                1,
                2,
                C_RESTORE,
                C_REPLACE,
            )],
            checks_drift: 0,
        },
        // EVAL R3: a kept A, a reverted far-B, a kept C (actual 4 checks).
        Run {
            key: "t::r3".into(),
            ts: 3,
            edits: vec![
                srow(
                    3,
                    "r3",
                    "dead_code_remove",
                    1,
                    false,
                    3,
                    4,
                    A_RESTORE,
                    A_REPLACE,
                ),
                srow(
                    3,
                    "r3",
                    "redundant_closure",
                    2,
                    true,
                    3,
                    4,
                    B1_RESTORE,
                    B1_REPLACE,
                ),
                srow(
                    3,
                    "r3",
                    "needless_borrow",
                    3,
                    false,
                    3,
                    4,
                    C_RESTORE,
                    C_REPLACE,
                ),
            ],
            checks_drift: 0,
        },
        // EVAL R4: a kept far-B (the false-decline candidate), a reverted
        // far-B, a reverted far-B under needless_borrow (actual 3 checks).
        Run {
            key: "t::r4".into(),
            ts: 4,
            edits: vec![
                srow(
                    4,
                    "r4",
                    "redundant_closure",
                    1,
                    false,
                    3,
                    3,
                    B1_RESTORE,
                    B1_REPLACE,
                ),
                srow(
                    4,
                    "r4",
                    "redundant_closure",
                    2,
                    true,
                    3,
                    3,
                    B2_RESTORE,
                    B2_REPLACE,
                ),
                srow(
                    4,
                    "r4",
                    "needless_borrow",
                    3,
                    true,
                    3,
                    3,
                    B1_RESTORE,
                    B1_REPLACE,
                ),
            ],
            checks_drift: 0,
        },
    ];

    let global = evaluate(&runs, false);
    assert_eq!(global.fit_accepted, 3);
    assert_eq!(global.eval_edits, 6);
    assert_eq!(global.eval_kept, 3);
    assert_eq!(global.unscoreable, 0);
    assert_eq!(global.degenerate, 0);
    let row = &global.rows[0];
    assert!((row.tau - 0.05).abs() < 1e-9);
    assert_eq!(row.declines, 4);
    assert_eq!(row.correct, 3);
    assert_eq!(row.false_declines, 1);
    assert!((row.false_rate.unwrap_or(f64::NAN) - 1.0 / 3.0).abs() < 1e-9);
    assert_eq!(row.sim_checks, 2);
    assert_eq!(row.actual_checks, 7);
    assert!((row.reduction_all - 5.0 / 7.0).abs() < 1e-9);
    assert!((row.reduction_ex - 5.0 / 7.0).abs() < 1e-9);
    assert_eq!(row.outlier_runs, 0);
    assert!((global.run_checks_median - 3.5).abs() < 1e-9);
    assert_eq!(
        global.best, None,
        "GLOBAL: 1/3 false decline must clear no tau"
    );
    let (passed, reasons) = verdict(&global);
    assert!(!passed);
    assert!(
        reasons.iter().any(|r| r.contains("false-decline")),
        "reasons: {reasons:?}"
    );

    let rule = evaluate(&runs, true);
    assert_eq!(
        rule.unscoreable, 3,
        "redundant_closure has no FIT reference"
    );
    let row = &rule.rows[0];
    assert_eq!(
        row.declines, 1,
        "only E6 (needless_borrow, far text) declines"
    );
    assert_eq!(row.correct, 1);
    assert_eq!(row.false_declines, 0);
    assert!((row.false_rate.unwrap_or(f64::NAN)).abs() < 1e-9);
    assert_eq!(row.sim_checks, 7);
    assert_eq!(row.actual_checks, 7);
    assert!(row.reduction_all.abs() < 1e-9);
    assert_eq!(
        rule.best,
        Some(0),
        "all tau tie at 0.0 — the first sweep tau wins"
    );
    let (passed, reasons) = verdict(&rule);
    assert!(!passed);
    assert!(
        reasons.iter().any(|r| r.contains("G3")),
        "reasons: {reasons:?}"
    );

    // Empty-reference arm: a FIT with no accepted fixes declines nothing and
    // adjudicates nothing.
    let empty_ref = vec![
        Run {
            key: "t::e1".into(),
            ts: 1,
            edits: vec![srow(
                1,
                "e1",
                "dead_code_remove",
                1,
                true,
                1,
                3,
                A_RESTORE,
                A_REPLACE,
            )],
            checks_drift: 0,
        },
        Run {
            key: "t::e2".into(),
            ts: 2,
            edits: vec![srow(
                2,
                "e2",
                "dead_code_remove",
                1,
                true,
                1,
                3,
                A_RESTORE,
                A_REPLACE,
            )],
            checks_drift: 0,
        },
        Run {
            key: "t::e3".into(),
            ts: 3,
            edits: vec![srow(
                3,
                "e3",
                "dead_code_remove",
                1,
                false,
                1,
                2,
                A_RESTORE,
                A_REPLACE,
            )],
            checks_drift: 0,
        },
        Run {
            key: "t::e4".into(),
            ts: 4,
            edits: vec![srow(
                4,
                "e4",
                "dead_code_remove",
                1,
                false,
                1,
                2,
                A_RESTORE,
                A_REPLACE,
            )],
            checks_drift: 0,
        },
    ];
    let er = evaluate(&empty_ref, false);
    assert_eq!(er.fit_accepted, 0);
    assert_eq!(er.unscoreable, 2);
    assert_eq!(er.rows[0].declines, 0);
    assert_eq!(
        er.best,
        Some(0),
        "every tau is adjudicable at reduction 0 — the first wins"
    );
    let (passed, reasons) = verdict(&er);
    assert!(!passed);
    assert!(
        reasons.iter().any(|r| r.contains("G3")),
        "reasons: {reasons:?}"
    );

    println!(
        "self-test PASS: GLOBAL 4/3/1 false-rate 1/3 red 5/7 FAIL · \
RULE 1/1/0 red 0 FAIL-best-tau-0.05 · empty-ref declines 0 FAIL"
    );
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut events: Vec<PathBuf> = Vec::new();
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut do_self_test = false;
    let mut min_runs = MIN_RUNS;
    let mut tau_only: Option<f64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--self-test" => do_self_test = true,
            "--events" => match args.next() {
                Some(p) => events.push(PathBuf::from(p)),
                None => {
                    eprintln!("refusal: --events needs a path\n{USAGE}");
                    std::process::exit(2);
                }
            },
            "--min-runs" => match args.next().and_then(|v| v.parse::<usize>().ok()) {
                Some(n) if n >= 2 => min_runs = n,
                _ => {
                    eprintln!("refusal: --min-runs needs an integer >= 2\n{USAGE}");
                    std::process::exit(2);
                }
            },
            "--tau" => match args.next().and_then(|v| v.parse::<f64>().ok()) {
                Some(t) if (0.0..=1.0).contains(&t) => tau_only = Some(t),
                _ => {
                    eprintln!("refusal: --tau needs a value in [0, 1]\n{USAGE}");
                    std::process::exit(2);
                }
            },
            other if other.starts_with("--") => {
                eprintln!("refusal: unknown flag {other}\n{USAGE}");
                std::process::exit(2);
            }
            other => roots.push(PathBuf::from(other)),
        }
    }

    println!("== i250 T1 retrieval-similarity floor census ==");
    println!(
        "bars: G3 >= {:.0}% realized check reduction · false declines <= {:.0}% of kept · beats the {:.1}% per-rule prior",
        G3_BAR * 100.0,
        FALSE_DECLINE_BAR * 100.0,
        T1A_PRIOR * 100.0,
    );

    if do_self_test {
        self_test();
        return;
    }

    if events.is_empty() && roots.is_empty() {
        eprintln!("refusal: no inputs (pass REPO_ROOTs and/or --events files)\n{USAGE}");
        std::process::exit(2);
    }

    let logs = collect_logs(&events, &roots);
    let total: usize = logs.iter().map(|l| l.rows.len()).sum();
    for log in &logs {
        println!(
            "log {}: {} row(s) (malformed {}, other-version {})",
            log.name,
            log.rows.len(),
            log.malformed,
            log.other_version,
        );
    }
    let runs = group_runs(&logs);
    let drift: usize = runs.iter().map(|r| r.checks_drift).sum();
    if drift > 0 {
        println!("schema drift: {drift} row(s) disagree with their run's run_edits/run_checks");
    }
    println!(
        "inputs: {} log(s) · {total} row(s) · runs {} (fit/eval split needs >= {min_runs})",
        logs.len(),
        runs.len(),
    );

    if total == 0 {
        println!(
            "accumulation note: rows accrue from every `cargo refine fix` run (blueprint lane \
on, RIIR_REFINE_META_BLUEPRINT unset/1); each scanned repo logs its own .refine/edit_events.jsonl"
        );
        return;
    }
    if runs.len() < min_runs {
        println!(
            "floor run GATED: {} run(s) accumulated, need >= {min_runs} for the FIT/EVAL split — \
rows accrue from real fix runs; re-run this census as they land",
            runs.len(),
        );
        return;
    }

    for (name, gated) in [("GLOBAL", false), ("RULE", true)] {
        let mut eval = evaluate(&runs, gated);
        if let Some(t) = tau_only {
            // Single-τ mode: report that row's verdict posture only.
            let idx = TAU_SWEEP.iter().position(|&v| (v - t).abs() < 1e-9);
            if let Some(i) = idx {
                eval.rows = vec![eval.rows[i].clone()];
                eval.best = eval.rows.first().and_then(|r| {
                    r.false_rate
                        .is_some_and(|fr| fr <= FALSE_DECLINE_BAR)
                        .then_some(0usize)
                });
            }
        }
        print_eval(name, &eval);
    }
}
