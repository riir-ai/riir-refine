//! Issue 191 T5 — the kernel_opt GOAT gate G2 harness runner: the
//! deterministic tranche, per-rule interleaved A/B microbenches, and the
//! pre-registered verdict.
//!
//! ```text
//! cargo run --release --example kernel_goat_g2 --features kernel_opt -- --full
//! cargo run --release --example kernel_goat_g2 --features kernel_opt -- --filter <substr>
//! cargo run --release --example kernel_goat_g2 --features kernel_opt -- --tranche
//! ```
//!
//! Modes:
//! - `--tranche` — print the deterministic tranche selector's output only
//!   (no compiles) — the pre-registration disclosure.
//! - `--full` — the whole tranche (15 files' GREEN rules).
//! - `--filter <substr>` — one rule by id substring (single-rule re-runs).
//! - `--out <path>` — write the JSON report.
//!
//! ⚠ Rig exclusivity is MANDATORY for any published number (T1 bar 2): run
//! via `agent-mail rig run 4090 --exclusive` (or verify the box quiet) and
//! record the PROVENANCE line this runner prints.

use riir_refine::domains::kernel_opt::corpus::goat_g1;
use riir_refine::domains::kernel_opt::corpus::goat_g2;

fn provenance_line() -> String {
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = std::process::Command::new("git")
        .args(["--no-optional-locks", "status", "--porcelain"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .map(|clean| if clean { "clean" } else { "DIRTY" })
        .unwrap_or("unknown");
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    format!(
        "PROVENANCE: commit {sha} [{dirty}] · rustc {} · {} logical cpus · UTC {} · posture: rig exclusivity REQUIRED for published numbers (T1 bar 2)",
        goat_g1_rustc_version(),
        cpus,
        chrono_now(),
    )
}

fn goat_g1_rustc_version() -> String {
    // Reuse the G1 reader through the public surface (the G2 module prints it
    // in its report; this line is the runner's own header).
    String::from_utf8_lossy(
        &std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default(),
    )
    .trim()
    .to_string()
}

fn chrono_now() -> String {
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", "(Get-Date).ToUniversalTime().ToString(\"yyyy-MM-ddTHH:mm:ssZ\")"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut full = false;
    let mut tranche_only = false;
    let mut filter: Option<String> = None;
    let mut out: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--full" => full = true,
            "--tranche" => tranche_only = true,
            "--filter" => {
                i += 1;
                filter = Some(args.get(i).cloned().unwrap_or_default());
            }
            "--out" => {
                i += 1;
                out = Some(args.get(i).cloned().unwrap_or_default());
            }
            other => {
                eprintln!("unknown arg: {other}");
                eprintln!("usage: kernel_goat_g2 [--full|--tranche] [--filter <substr>] [--out <path>]");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    if !full && !tranche_only && filter.is_none() {
        eprintln!("one of --full / --tranche / --filter is required (this runner spawns rustc compiles; it refuses to run uninvoked)");
        std::process::exit(2);
    }
    println!("kernel_opt GOAT gate G2 harness (Issue 191 T5)");
    println!("{}", provenance_line());

    if tranche_only {
        let files = goat_g2::tranche();
        println!(
            "tranche: {} file(s) by GREEN #[test] tokens desc, filename asc tie-break, cap {}",
            files.len(),
            goat_g2::TRANCHE_FILES
        );
        for f in &files {
            println!(
                "  {:<18} green #[test] tokens {:>4} · {} green rule(s)",
                f.file, f.green_test_tokens, f.green_rules
            );
        }
        return;
    }

    if let Some(f) = &filter {
        let mut verdict = None;
        for (file, entries) in goat_g1::per_file_entries_for_tranche() {
            for rule in entries {
                if rule.rule_id.contains(f.as_str()) {
                    println!("bench: {} ({file})…", rule.rule_id);
                    verdict = Some(goat_g2::bench_rule(&rule, &file));
                }
            }
        }
        let Some(r) = verdict else {
            eprintln!("no rule matches filter {f:?}");
            std::process::exit(1);
        };
        if let Some(reason) = &r.not_measured_reason {
            println!("{}: NOT-MEASURED — {reason}", r.rule_id);
        } else if let Some(e) = &r.error {
            println!("{}: ERROR — {e}", r.rule_id);
        } else {
            println!(
                "{}: {}",
                r.rule_id,
                r.pairs
                    .iter()
                    .map(|p| format!(
                        "{} med {:.4}{} ({})",
                        p.fn_name,
                        p.median_speedup,
                        if p.passes() { " PASS" } else { "" },
                        if p.measured { "measured" } else { "ZERO" }
                    ))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
        return;
    }

    let started = std::time::Instant::now();
    let (tranche, rules) = goat_g2::run_g2();
    let report = goat_g2::aggregate(tranche, rules, started.elapsed().as_secs_f64());

    println!("tranche: {} file(s)", report.tranche.len());
    for f in &report.tranche {
        println!("  {file} ({} green rules)", f.green_rules, file = f.file);
    }
    println!(
        "population: {} rule(s) — {} measured / {} passing (>= 1.06) / {} regressing / {} not-measured",
        report.rules.len(),
        report.measured,
        report.passing,
        report.regressing,
        report.not_measured
    );
    println!(
        "tranche median speedup: {:.4} (bar: >= 1.06) in {:.1}s",
        report.tranche_median_speedup, report.seconds
    );
    for r in &report.rules {
        if let Some(reason) = &r.not_measured_reason {
            println!("NOT-MEASURED {} ({}) — {reason}", r.rule_id, r.file);
        } else if let Some(e) = &r.error {
            println!("ERROR {} ({}) — {e}", r.rule_id, r.file);
        } else {
            let best = r.best_speedup().unwrap_or(f64::NAN);
            println!(
                "measured {} ({}) best {:.4}{}{}",
                r.rule_id,
                r.file,
                best,
                if r.passes() { " PASS" } else { "" },
                if r.regresses() { " REGRESSION" } else { "" }
            );
        }
    }

    if let Some(path) = out {
        let json = serde_json::to_string_pretty(&report).expect("report serializes");
        std::fs::write(&path, json).expect("write report");
        println!("report: {path}");
    }

    match report.verdict() {
        Ok(()) => println!("G2 verdict: PASS"),
        Err(e) => {
            println!("G2 verdict: FAIL\n{e}");
            std::process::exit(1);
        }
    }
}
