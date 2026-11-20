//! Plan 392 Phase 1 (Research 426) — the mining-gain census instrument.
//!
//! REPORT-ONLY and read-only: loads the committed
//! `.benchmarks/mining_gain_panel.jsonl` fixture, fits the katgpt-core
//! `epiplexity_gain::GainEstimator` (Plan 641 — consumed, never forked) via
//! `riir_refine::epiplexity_census::run_census`, and prints:
//!
//! 1. the saturation verdict line (arm + rank census) against the Plan 392
//!    Phase 0 prediction (eff10 = 1 vs bar 7 → SPARSE on this fixture);
//! 2. the per-domain table (counts, curve state, ε floor, headroom,
//!    marginal gain, sigmoid weight at τ = 0.5), sorted by domain name;
//! 3. the GAIN-ORDERED queue (weight desc, ties → name; `NoCurve` abstains
//!    last);
//! 4. the saturation gate (`DEMOTED (<name>)` — headroom under
//!    `SATURATION_DEMOTE_HEADROOM` = 0.005);
//! 5. the dominance refusal (`SKIP-SELECTION` at ≥ 0.85 top weight);
//! 6. with `--history`: the queue's top-two at every 16th panel row.
//!
//! It exits 0 on EVERY successful read and never writes anything — the mining
//! queue file itself is never touched. Refusals are typed and exit 2.
//!
//! ```text
//! cargo run --features epiplexity_census --example mining_gain_census \
//!     [--panel <path>] [--history]
//! ```

use std::path::PathBuf;

use riir_refine::epiplexity_census::{
    CensusReport, CurveState, DOMINANCE_SKIP_WEIGHT, SATURATION_DEMOTE_HEADROOM, run_census,
};

/// The Phase 0 prediction this census checks against (Research 426, CORRECTED
/// 2026-11-20 — the first pass's divergent Jacobi pairing read 2; the
/// converged spectrum reads 1, trace invariant rel err 2.3e-16): eff10 = 1
/// (the σ ≥ 10%·σmax share) against bar 7 → the Sparse arm.
const PHASE0_PREDICTED_EFF10: usize = 1;
/// The Phase 0 predicted arm.
const PHASE0_PREDICTED_ARM: &str = "Sparse";

fn fmt_opt_f64(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{x:.6}"),
        None => "-".to_string(),
    }
}

fn fmt_opt_gain(v: Option<f32>) -> String {
    match v {
        Some(x) => format!("{x:.3e}"),
        None => "-".to_string(),
    }
}

fn print_report(report: &CensusReport) {
    println!("== mining-gain census — riir-refine Plan 392 Phase 1 (Research 426) ==");
    println!(
        "panel: rows={} domains={} (τ = {})",
        report.rows,
        report.domains.len(),
        report.tau
    );
    let c = &report.census;
    println!(
        "saturation: {} — rank census: domains={} bar={} eff10={} eff1={} participation_ratio={:.3}",
        report.arm, c.domains, c.bar, c.eff10, c.eff1, c.participation_ratio
    );
    let arm_matches = report.arm == PHASE0_PREDICTED_ARM;
    let rank_delta = if c.eff10 == PHASE0_PREDICTED_EFF10 {
        "measured eff10 matches the Phase 0 figure".to_string()
    } else {
        format!(
            "measured eff10={} vs the Phase 0 figure {} — same verdict, both far under bar {}",
            c.eff10, PHASE0_PREDICTED_EFF10, c.bar
        )
    };
    println!(
        "  Plan 392 Phase 0 prediction (rank {} vs bar {} → {}): arm {} ({})",
        PHASE0_PREDICTED_EFF10,
        c.bar,
        PHASE0_PREDICTED_ARM,
        if arm_matches { "MATCHES" } else { "DIFFERS" },
        rank_delta
    );

    println!();
    println!(
        "{:<14} {:>10}  {:<8} {:>10} {:>10} {:>12} {:>8}",
        "domain", "counts_now", "curve", "eps_floor", "headroom", "marginal_gain", "weight"
    );
    for row in &report.table {
        let curve = match row.curve {
            CurveState::Fit => "Fit",
            CurveState::NoCurve => "NoCurve",
        };
        println!(
            "{:<14} {:>10.1}  {:<8} {:>10} {:>10} {:>12} {:>8.4}",
            row.name,
            row.counts_now,
            curve,
            fmt_opt_f64(row.epsilon_floor),
            fmt_opt_f64(row.headroom),
            fmt_opt_gain(row.marginal_gain),
            row.weight
        );
    }

    println!();
    println!("queue (gain-ordered: weight desc, ties → name; NoCurve abstains last):");
    for (i, entry) in report.queue.iter().enumerate() {
        if entry.allocable {
            println!(
                "{:>3}. {:<14} weight={:.4}",
                i + 1,
                entry.name,
                entry.weight
            );
        } else {
            println!("{:>3}. {:<14} NoCurve — not allocable", "-", entry.name);
        }
    }

    println!();
    if report.demoted.is_empty() {
        println!("saturation gate (headroom < {SATURATION_DEMOTE_HEADROOM}): none demoted");
    } else {
        for name in &report.demoted {
            println!("saturation gate: DEMOTED ({name})");
        }
    }
    match &report.dominance_skip {
        Some(name) => println!(
            "SKIP-SELECTION: {name} dominates the allocation integral — the machinery refuses to run \
             (top weight ≥ {DOMINANCE_SKIP_WEIGHT})"
        ),
        None => println!("dominance gate (top weight ≥ {DOMINANCE_SKIP_WEIGHT}): none"),
    }

    if !report.history.is_empty() {
        println!();
        println!(
            "history (every 16th row, weights recomputed at τ = {}):",
            report.tau
        );
        for line in &report.history {
            println!(
                "t={} {} top={} weight={:.4} second={} weight={:.4}",
                line.idx, line.ts, line.top, line.top_weight, line.second, line.second_weight
            );
        }
    }
    println!();
    println!("report-only: exit 0 — the mining queue file was never touched.");
}

fn main() {
    let mut panel = PathBuf::from(".benchmarks/mining_gain_panel.jsonl");
    let mut history = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--panel" => match args.next() {
                Some(p) => panel = PathBuf::from(p),
                None => {
                    eprintln!("--panel needs a path");
                    std::process::exit(2);
                }
            },
            "--history" => history = true,
            other => {
                eprintln!("unknown argument {other:?} (usage: [--panel <path>] [--history])");
                std::process::exit(2);
            }
        }
    }
    let report = match run_census(&panel, history) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("mining-gain census refused: {e}");
            std::process::exit(2);
        }
    };
    print_report(&report);
}
