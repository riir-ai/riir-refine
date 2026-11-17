//! Issue 243 — the pre-registered frame-validation pass (T1 retention-hit ×
//! T2 window-mass × T3 flip-identity) over the COMMITTED Bench-150 windows
//! (`../.benchmarks/150_plan633_t33_crossover_alpha0_growth_data/windows/`).
//!
//! REPORT-ONLY: reads windows, computes `riir_refine::trinity_frame`
//! frames, prints the three arms' tables plus the beyond-census support
//! line. It recomputes NO executed bar of Benches 148/149/150 and touches
//! no gate code; the executed instrument (`adoption_g3_cell`) is untouched.
//!
//! Falsifier (issue 243): if the trinity surfaces no verdict-relevant fact
//! beyond Bar C's existing per-consult census (nothing the aggregate
//! lines + Bar C don't already say), the issue closes negative. T1 runs on
//! the PROXY arm here (committed windows carry no forward rows) — a
//! negative close reads "T2/T3 add nothing", never "T1 adds nothing".
//!
//! Defaults are the executed Bench-150 posture (α₀ = 0.5, noise arms
//! 0.1/0.2, 32 seeds, bar 0.90) so every stability number is directly
//! comparable to the record's disclosed fractions.

use std::path::PathBuf;
use std::process::ExitCode;

use riir_refine::adoption_gate::AdoptionWindow;
use riir_refine::trinity_frame::{ConsultRow, TrinityConfig, compute_frame};

const MIN_STABILITY_DEFAULT: f64 = 0.90;

fn parse_floats(csv: &str, what: &str) -> Result<Vec<f64>, String> {
    csv.split(',')
        .map(|s| s.trim().parse::<f64>().map_err(|e| format!("{what}: {e}")))
        .collect()
}

struct Args {
    windows: Vec<PathBuf>,
    noises: Vec<f64>,
    seeds: usize,
    min_stability: f64,
}

fn parse_args<I: Iterator<Item = String>>(mut it: I) -> Result<Args, String> {
    let mut windows = Vec::new();
    let mut noises = vec![0.1, 0.2];
    let mut seeds = 32usize;
    let mut min_stability = MIN_STABILITY_DEFAULT;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--windows" => {
                let v = it.next().ok_or("--windows needs a value")?;
                windows.push(PathBuf::from(v));
            }
            "--noises" => {
                let v = it.next().ok_or("--noises needs a value")?;
                noises = parse_floats(&v, "--noises")?;
            }
            "--seeds" => {
                let v = it.next().ok_or("--seeds needs a value")?;
                seeds = v.parse::<usize>().map_err(|e| format!("--seeds: {e}"))?;
            }
            "--min-stability" => {
                let v = it.next().ok_or("--min-stability needs a value")?;
                min_stability = v
                    .parse::<f64>()
                    .map_err(|e| format!("--min-stability: {e}"))?;
            }
            other => windows.push(PathBuf::from(other)),
        }
    }
    if windows.is_empty() {
        return Err("no windows given (positional paths or --windows)".to_string());
    }
    Ok(Args {
        windows,
        noises,
        seeds,
        min_stability,
    })
}

/// The executed record's Bar-C census set: accepted-carrier demote
/// REFUSALS. The beyond-census support line counts fragile consults
/// outside it — the quantitative novelty surface the record's falsifier
/// adjudicates against.
fn bar_c_member(c: &ConsultRow) -> bool {
    c.id.kind == "demote" && c.accepted_carrier && c.verdict == "REFUSE"
}

fn print_window(name: &str, frame: &riir_refine::trinity_frame::WindowFrame, cfg: &TrinityConfig) {
    println!(
        "== window {name} ({} rows: {} accepted / {} rejected; {} tags; {} consults)",
        frame.rows,
        frame.accepted,
        frame.rejected,
        frame.tags,
        frame.consults.len()
    );

    // T2 first (the frame every other arm reads its consults from): the
    // mass table across ALL verdict classes — a REFUSE on a 1-row window
    // and a REFUSE on a 10-row window are different verdicts; nothing
    // pools.
    let mut classes: Vec<(&str, &str, Vec<&ConsultRow>)> = Vec::new();
    for kind in ["demote", "readmit"] {
        for verdict in ["PROCEED", "REFUSE", "ABSTAIN"] {
            let rows: Vec<&ConsultRow> = frame
                .consults
                .iter()
                .filter(|c| c.id.kind == kind && c.verdict == verdict)
                .collect();
            if !rows.is_empty() {
                classes.push((kind, verdict, rows));
            }
        }
    }
    println!(
        "  [T2] verdict class | consults | mass rows min..max | single-row | all-rejected | zero-accepted"
    );
    for (kind, verdict, rows) in &classes {
        let mut min_r = usize::MAX;
        let mut max_r = 0usize;
        for c in rows {
            min_r = min_r.min(c.mass_rows);
            max_r = max_r.max(c.mass_rows);
        }
        let single = rows.iter().filter(|c| c.single_row_carrier).count();
        let all_rej = rows.iter().filter(|c| c.all_rejected_carrier).count();
        let zero_acc = rows.iter().filter(|c| c.mass_accepted == 0).count();
        println!(
            "  [T2] {kind:7} {verdict:7} | {:>8} | {:>4}..{:<4} | {:>10} | {:>12} | {:>13}",
            rows.len(),
            min_r,
            max_r,
            single,
            all_rej,
            zero_acc
        );
    }
    println!(
        "  [T2] seat-swap class: 0 consults (no seat rows in committed windows — \
         the seat leg is UNMEASURED, disclosed)"
    );

    // T1 (PROXY arm — disclosed): per-consult min-across-arms stability
    // against the bar, per verdict class. ABSTAIN carries no winner.
    let hits = frame.t1_hits(cfg);
    let misses = frame.t1_misses(cfg);
    let abstains = frame.t1_abstains();
    println!(
        "  [T1 proxy] winner-carrying consults: hits={} misses={} (bar ≥ {:.2}); \
         abstain-excluded={} (no winner to test)",
        hits, misses, cfg.min_stability, abstains
    );
    for c in frame
        .consults
        .iter()
        .filter(|c| c.verdict != "ABSTAIN" && c.min_stability < cfg.min_stability)
    {
        println!(
            "  [T1 miss] {:7} {:28} {} min-stability {:.3} — no reliably-best winner",
            c.id.kind, c.id.tag, c.verdict, c.min_stability
        );
    }

    // T3: the flip census — WHICH consult flips, moved by WHICH carrier
    // rows (LOO attribution), plus the noise-stability fragile set.
    let fragile = frame.fragile(cfg);
    println!(
        "  [T3] fragile consults (noise below bar OR any LOO move): {}",
        fragile.len()
    );
    for c in &fragile {
        let loo: Vec<String> = c
            .loo_moved_by
            .iter()
            .map(|m| format!("row{}({},{})", m.row_index, m.row_tag, m.row_accepted))
            .collect();
        println!(
            "  [T3] {:7} {:28} {} noise={:.3} loo=[{}]",
            c.id.kind,
            c.id.tag,
            c.verdict,
            c.min_stability,
            if loo.is_empty() {
                "none".to_string()
            } else {
                loo.join(", ")
            }
        );
    }
    // The cross-class join: per tag, which kinds are fragile — the
    // concentration question (does a healthy aggregate hide fragile
    // verdicts on the productive carriers?).
    let mut by_tag: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
    for c in &fragile {
        by_tag.entry(c.id.tag.as_str()).or_default().push(c.id.kind);
    }
    for (tag, kinds) in &by_tag {
        println!("  [T3 join] {tag}: fragile in {}", kinds.join("+"));
    }
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let cfg = TrinityConfig {
        alpha0: 0.5,
        noises: args.noises.clone(),
        seeds: args.seeds,
        min_stability: args.min_stability,
    };
    println!(
        "[trinity posture] α₀={:.2} noises={:?} seeds={} bar={:.2} — PROXY T1 \
         (committed windows carry no forward rows; disclosed)",
        cfg.alpha0, cfg.noises, cfg.seeds, cfg.min_stability
    );

    let mut total_consults = 0usize;
    let mut total_hits = 0usize;
    let mut total_misses = 0usize;
    let mut total_fragile = 0usize;
    let mut beyond_census = 0usize;
    let mut beyond_census_named: Vec<String> = Vec::new();
    for path in &args.windows {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let window = match AdoptionWindow::load_file(path.clone()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        let frame = match compute_frame(&window, &cfg) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        print_window(&name, &frame, &cfg);
        total_consults += frame.consults.len();
        total_hits += frame.t1_hits(&cfg);
        total_misses += frame.t1_misses(&cfg);
        let fragile = frame.fragile(&cfg);
        total_fragile += fragile.len();
        for c in fragile {
            if !bar_c_member(c) {
                beyond_census += 1;
                beyond_census_named.push(format!(
                    "{} {} {} ({:.3})",
                    c.id.kind, c.id.tag, c.verdict, c.min_stability
                ));
            }
        }
        println!();
    }

    println!("== totals across {} windows", args.windows.len());
    println!(
        "[T1 proxy] consults={total_consults} hits={total_hits} misses={total_misses} \
         abstain-excluded={}",
        total_consults - total_hits - total_misses
    );
    println!("[T3] fragile consults total: {total_fragile}");
    println!(
        "[beyond-census] fragile consults OUTSIDE Bar C's accepted-carrier \
         demote-refusal set: {beyond_census} — {}",
        if beyond_census_named.is_empty() {
            "none (the census subsumes the fragile set)".to_string()
        } else {
            beyond_census_named.join("; ")
        }
    );
    ExitCode::SUCCESS
}
