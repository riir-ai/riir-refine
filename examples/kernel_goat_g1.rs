//! Issue 191 T2 — the kernel_opt GOAT gate G1 harness runner.
//!
//! The standing fixture validator: extraction equality against the
//! independent structural census, the downward-only floors, and per-rule
//! standalone `rustc --edition 2021 --test` compile + run of every
//! fixture-bearing rule's `ref_shape` / `opt_pattern` strings.
//!
//! ```text
//! cargo run --release --example kernel_goat_g1 --features kernel_opt -- --full
//! cargo run --release --example kernel_goat_g1 --features kernel_opt -- --smoke
//! cargo run --release --example kernel_goat_g1 --features kernel_opt -- --filter <substr> --out report.json
//! cargo run --release --example kernel_goat_g1 --features kernel_opt -- --full --emit-status src/domains/kernel_opt/corpus/g1_status.tsv
//! ```
//!
//! Modes:
//! - `--full` — the whole fixture population (every fixture-bearing rule;
//!   parse-only rules recorded as disclosed, not executed).
//! - `--smoke` — census + floors + the first fixture-bearing rule by
//!   name-ascending order (deterministic, pre-registered pick).
//! - `--filter <substr>` — rules whose id contains the substring.
//! - `--limit N` — cap the executed fixture-bearing rules (the census
//!   assertions bind regardless — the population law).
//! - `--out <path>` — write the JSON report (default: stdout only).
//! - `--emit-status <path>` — write the digest-keyed verdict store (T3's
//!   annotation channel; WHOLE-population only — refuses sampled runs).
//!
//! The gate verdict (exit 0/1) is [`riir_refine::domains::kernel_opt::corpus::goat_g1::G1Report::verdict`]:
//! census equality + floors + every executed fixture rule green. A FAIL names
//! the rules — that is the gate doing its job (the entry discipline was
//! manual; drift shows here first).

use riir_refine::domains::kernel_opt::corpus::goat_g1;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut full = false;
    let mut smoke = false;
    let mut filter: Option<String> = None;
    let mut limit: Option<usize> = None;
    let mut out: Option<String> = None;
    let mut emit_status: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--full" => full = true,
            "--smoke" => smoke = true,
            "--filter" => {
                i += 1;
                filter = Some(args.get(i).cloned().unwrap_or_default());
            }
            "--limit" => {
                i += 1;
                limit = args.get(i).and_then(|v| v.parse().ok());
            }
            "--out" => {
                i += 1;
                out = Some(args.get(i).cloned().unwrap_or_default());
            }
            "--emit-status" => {
                i += 1;
                emit_status = Some(args.get(i).cloned().unwrap_or_default());
            }
            other => {
                eprintln!("unknown arg: {other}");
                eprintln!("usage: kernel_goat_g1 [--full|--smoke] [--filter <substr>] [--limit N] [--out <path>] [--emit-status <path>]");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    if full && smoke {
        eprintln!("--full and --smoke are mutually exclusive");
        std::process::exit(2);
    }
    if emit_status.is_some() && (smoke || filter.is_some() || limit.is_some()) {
        eprintln!(
            "--emit-status requires the WHOLE population: it refuses --smoke, --filter, --limit \
             (a sampled run must never emit a partial verdict store)"
        );
        std::process::exit(2);
    }
    if emit_status.is_some() {
        full = true;
    }
    if !full && !smoke && filter.is_none() {
        eprintln!("one of --full / --smoke / --filter is required (default IS the dry review: this runner executes compiles, so it refuses to run uninvoked)");
        std::process::exit(2);
    }
    if smoke {
        // Deterministic pre-registered pick: the first fixture-bearing rule in
        // name-ascending module order. The census + floors still bind fully.
        limit = Some(1);
    }

    println!("kernel_opt GOAT gate G1 harness (Issue 191 T2)");
    let report = goat_g1::run_g1(filter.as_deref(), limit);

    println!(
        "census: extracted == independent structural census: {}",
        match &report.census_check {
            Ok(()) => "OK".to_string(),
            Err(e) => format!("MISMATCH\n{e}"),
        }
    );
    println!(
        "population: {} fixture-bearing rules across {} files (floors: >= {}, >= {})",
        report.fixture_rules,
        report.fixture_files,
        goat_g1::MIN_FIXTURE_RULES,
        goat_g1::MIN_FIXTURE_FILES
    );
    println!(
        "executed: {} green / {} FAIL / {} parse-only (disclosed) in {:.1}s [{}]",
        report.green, report.fail, report.parse_only, report.seconds, report.rustc_version
    );
    for rule in &report.rules {
        if rule.outcome() == goat_g1::G1Outcome::Fail {
            println!("FAIL {} ({})", rule.rule_id, rule.file);
            for arm in &rule.arms {
                if !(arm.compile_ok && arm.run_ok) {
                    if let Some(e) = &arm.error {
                        let first = e.lines().next().unwrap_or("");
                        println!("  arm {}: {}", arm.arm, first);
                    }
                }
            }
        }
    }

    if let Some(path) = out {
        let json = serde_json::to_string_pretty(&report)
            .expect("report serializes");
        std::fs::write(&path, json).expect("write report");
        println!("report: {path}");
    }

    if let Some(path) = emit_status {
        let rows = goat_g1::status_rows(&report);
        goat_g1::emit_status_table(&report, std::path::Path::new(&path))
            .unwrap_or_else(|e| {
                eprintln!("emit-status write failed: {e}");
                std::process::exit(1);
            });
        println!(
            "status store: {path} — {} row(s) (digest-keyed; the corpus annotation \
             channel fails closed to unverified on drift)",
            rows.len()
        );
    }

    match report.verdict() {
        Ok(()) => {
            println!("G1 verdict: PASS");
        }
        Err(e) => {
            println!("G1 verdict: FAIL\n{e}");
            std::process::exit(1);
        }
    }
}
