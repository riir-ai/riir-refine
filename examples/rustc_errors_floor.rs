//! Issue 035 T2 — run the G-floor measurement and print the published tables.
//!
//! ```bash
//! cargo run --release --example rustc_errors_floor --features rustc_errors
//! ```
//!
//! Two measurements, both against the live toolchain:
//!
//! 1. **The premise probe** — per error code, does rustc ship a
//!    machine-applicable suggestion, and does `cargo fix --broken-code`
//!    apply it? This is the contested/uncontested split that bounds what the
//!    `rustc_errors` domain can claim.
//! 2. **The floor** — `cargo fix --broken-code` scored over the
//!    known-inverse mutation corpus: fix rate AND, for every repair that
//!    landed, whether it restored the original meaning or diverged from it.
//!
//! The second number is the one that matters. A fixer that repairs a mutant
//! with a diff that means something different has not "fixed" it — it has
//! moved the problem somewhere a compiler cannot see.

use riir_clippy::domains::rustc_errors::floor::{
    CargoFixBaseline, FixVerdict, FloorHarness, FloorReport, NullFixer, MUTANT_CASES,
};
use riir_clippy::domains::rustc_errors::probe::PremiseProbe;

fn main() {
    println!("=== Issue 035 — rustc_errors premise probe + G-floor ===\n");
    println!("rustc: {}", rustc_version());

    probe_section();
    let baseline = floor_section();
    null_arm_section();

    println!("\n── Verdict ──");
    match baseline {
        None => println!("  INCONCLUSIVE — cargo unavailable, nothing measured."),
        Some(report) => {
            println!(
                "  Floor arm `{}`: {:.1}% of the corpus repaired.",
                report.arm,
                report.total_fix_rate() * 100.0
            );
            let diverged: Vec<&str> = report
                .outcomes
                .iter()
                .filter(|o| o.verdict == FixVerdict::Diverged)
                .map(|o| o.name)
                .collect();
            match diverged.is_empty() {
                true => println!(
                    "  Every repair matched the known inverse — on this corpus the floor \
                     is not just free, it is right."
                ),
                false => println!(
                    "  {} repair(s) compiled but DIVERGED from the known inverse: {}\n  \
                     These are where a compile-only oracle cannot tell you the meaning \
                     changed — the review surface a ranked-alternatives fixer would own.",
                    diverged.len(),
                    diverged.join(", ")
                ),
            }
            println!(
                "  Any `--fix-compile` template must beat this per-code floor before it \
                 ships (Plan 019 T6.9)."
            );
        }
    }
}

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn probe_section() {
    println!("\n── 1. Premise probe: does rustc already fix this? ──\n");
    let probe = PremiseProbe::new();
    if !probe.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    println!(
        "  {:<30} {:>6}  {:>5} {:>5}  {:>9}  {:<12} replacement",
        "case", "code", "sugg", "auto", "fix-moved", "territory"
    );
    for result in probe.probe_all() {
        match result {
            Err(e) => println!("  ERROR: {e}"),
            Ok(o) => {
                let territory = match o.is_uncontested() {
                    true => "UNCONTESTED",
                    false => "contested",
                };
                let repl = match o.replacements.is_empty() {
                    true => "-".to_string(),
                    false => format!("{:?}", o.replacements),
                };
                let mislabel = match o.code_as_declared() {
                    true => "",
                    false => "  << MIS-LABELED",
                };
                println!(
                    "  {:<30} {:>6}  {:>5} {:>5}  {:>9}  {:<12} {repl}{mislabel}",
                    o.name,
                    o.expect_code,
                    o.suggestion_spans,
                    o.machine_applicable,
                    o.cargo_fix_changed,
                    territory
                );
            }
        }
    }
    probe.cleanup();
}

fn floor_section() -> Option<FloorReport> {
    println!(
        "\n── 2. G-floor: `cargo fix --broken-code` over {} known-inverse mutants ──\n",
        MUTANT_CASES.len()
    );
    let harness = FloorHarness::new();
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return None;
    }
    let report = match harness.measure(&CargoFixBaseline::new()) {
        Ok(r) => r,
        Err(e) => {
            println!("  ERROR: {e}");
            return None;
        }
    };
    print_report(&report);

    if !report.invalid_mutants.is_empty() {
        println!(
            "\n  CORPUS INTEGRITY FAILURES ({}):",
            report.invalid_mutants.len()
        );
        for bad in &report.invalid_mutants {
            println!("    - {bad}");
        }
    }

    println!("\n  Per-mutant detail:");
    for o in &report.outcomes {
        let verdict = match o.verdict {
            FixVerdict::MatchedInverse => "matched-inverse",
            FixVerdict::Diverged => "DIVERGED",
            FixVerdict::Unfixed => "unfixed",
        };
        println!(
            "    {:<34} {:>6} {:<18} {:<15} valid={}",
            o.name,
            o.expect_code,
            o.mutator.label(),
            verdict,
            o.mutant_valid
        );
        if let Some(src) = &o.diverged_source {
            for line in src.lines().filter(|l| !l.trim().is_empty()) {
                println!("        | {line}");
            }
        }
    }
    harness.cleanup();
    Some(report)
}

/// The falsification arm: an arm that repairs nothing must score 0%.
/// If it scores above zero the harness is measuring the mutation, not the fixer.
fn null_arm_section() {
    println!("\n── 3. Harness self-check: the no-op arm must score 0% ──\n");
    let harness = FloorHarness::new().with_work_root(
        std::env::temp_dir().join(format!("riir_floor_null_arm_{}", std::process::id())),
    );
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    match harness.measure(&NullFixer) {
        Err(e) => println!("  ERROR: {e}"),
        Ok(report) => {
            let rate = report.total_fix_rate();
            println!(
                "  arm `{}`: {:.1}% fixed — {}",
                report.arm,
                rate * 100.0,
                match rate == 0.0 {
                    true => "PASS (every mutant genuinely broken)",
                    false => "FAIL — the harness is scoring something other than the fixer",
                }
            );
        }
    }
    harness.cleanup();
}

fn print_report(report: &FloorReport) {
    println!("  arm: {}\n", report.arm);
    println!(
        "  {:<8} {:>8} {:>9} {:>9} {:>8} {:>10} {:>11}",
        "code", "mutants", "matched", "diverged", "unfixed", "fix-rate", "divergence"
    );
    for row in &report.rows {
        println!(
            "  {:<8} {:>8} {:>9} {:>9} {:>8} {:>9.0}% {:>10.0}%",
            row.code,
            row.mutants,
            row.matched,
            row.diverged,
            row.unfixed,
            row.fix_rate() * 100.0,
            row.divergence_rate() * 100.0
        );
    }
    println!(
        "\n  TOTAL fix rate: {:.1}% ({} mutants)",
        report.total_fix_rate() * 100.0,
        report.outcomes.len()
    );
}
