//! Issue 035 T2 + Plan 019 Phase 6 — the G-floor + GOAT measurement runner.
//!
//! ```bash
//! # everything (the full Phase 6 measurement session, ~30-40 min):
//! cargo run --release --example rustc_errors_floor --features rustc_errors
//!
//! # one section at a time (chunked runs; sections: probe floor driver null ab):
//! cargo run --release --example rustc_errors_floor --features rustc_errors -- --only driver
//! cargo run --release --example rustc_errors_floor --features "rustc_errors,latent_retrieval" -- --only ab
//! ```
//!
//! Sections (each prints its own table; all measured against the live
//! toolchain):
//!
//! 1. **probe** — per error code, does rustc ship a machine-applicable
//!    suggestion, and does `cargo fix --broken-code` apply it? The
//!    contested/uncontested split that bounds what `rustc_errors` can claim.
//! 2. **floor** — `cargo fix --broken-code` over the FULL known-inverse
//!    corpus (the 12 committed anchors + the N≥200 generated cases):
//!    per-code fix rate AND, for every landed repair, whether it restored
//!    the original meaning or diverged.
//! 3. **driver** — the production `--fix-compile` arm over the same corpus
//!    (verbose: per-iteration timings), plus the G1 semantics pass
//!    (`cargo test` post-fix on every repaired case — the corpus's
//!    ground-truth behavior tests) and the G2 latency table (p50/p99).
//! 4. **null** — the no-op arm self-check: must score exactly 0% on the
//!    full corpus (corpus integrity at scale).
//! 5. **ab** — the G6/G7 ordering A/B (needs `latent_retrieval`): a
//!    learning pass warms a temp trajectory store, then static min-delta
//!    vs flat `(code,strategy)` BetaPosterior vs shape-conditional
//!    ordering are compared on candidates-tried + reverts over the
//!    template-covered stratum.
//!
//! # Issue 126 T0 — `--dump-fixseq <dir>` (label-gen posture)
//!
//! `--only driver --dump-fixseq <dir>` runs the production driver arm over
//! the full corpus and dumps each case's attempt ring to
//! `<dir>/.heal/fixseq/run-<ts>.jsonl` — the hindsight-label corpus
//! (`span_embed` × `strategy_id` × `outcome`) for the sigmoid-NCE
//! rule-embedding trainer (kept = positive, reverted = hard negative).
//! Early-returns before the G1/G2 measurement passes. Pass a DEDICATED root
//! (e.g. `.benchmarks/data/126_t0/rings`), never the repo root — dumping
//! into `<repo>/.heal/fixseq` would pollute the organic-ring readings the
//! `--horizon-report` consumer owns.
//!
//! # Issue 128 T5 — `--dump-fixseq-cfg <chunked|strided>` (the mint configs)
//!
//! With `--dump-fixseq-cfg`, the corpus is REPLACED by a composed
//! multi-error generation config (the T5 two-config law — two seeds of one
//! composer do NOT satisfy T7's disjointness rule):
//!
//! - `chunked` — `corpus_gen::compose_multi(generate_corpus(), 5)`: k
//!   ADJACENT cases per world (same-family error runs share a scaffold).
//! - `strided` — `corpus_gen::compose_multi_strided(generate_corpus(), 5)`:
//!   stride-k across the corpus (families MIX per world; the Bench-088
//!   menu-width-3-4 shape).
//!
//! Both configs append the 4 `interacting_pairs` (the E0614 masking chains
//! — the only E0614 source, without which indicator (c) and the
//! root/cascade attribution would have zero signal). MUTANT_CASES (solo
//! single-error anchors) are EXCLUDED: single-attempt runs carry no
//! transitions and would only dilute the cascade corpus. The run also
//! writes `<dir>/manifest.json` — per-file BLAKE3 + counts, the house data
//! discipline (the 049_t01 stamped-manifest law).

use riir_clippy::domains::rustc_errors::corpus_gen::{generate_corpus, Territory};
use riir_clippy::domains::rustc_errors::floor::{
    CargoFixBaseline, DriverFixer, FixVerdict, FloorHarness, FloorOutcome, FloorReport, NullFixer,
    MUTANT_CASES,
};
use riir_clippy::domains::rustc_errors::probe::PremiseProbe;

fn main() {
    let only: Vec<String> = std::env::args()
        .skip_while(|a| a != "--only")
        .nth(1)
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let want = |s: &str| only.is_empty() || only.iter().any(|x| x == s);

    println!("=== Plan 019 Phase 6 — premise probe + G-floor + GOAT ===\n");
    println!("rustc: {}", rustc_version());

    if want("probe") {
        probe_section();
    }
    if want("floor") {
        floor_section();
    }
    if want("driver") {
        let dump_fixseq_root: Option<String> = std::env::args().position(|a| a == "--dump-fixseq")
            .and_then(|p| std::env::args().nth(p + 1));
        let dump_fixseq_cfg: Option<String> =
            std::env::args().position(|a| a == "--dump-fixseq-cfg")
                .and_then(|p| std::env::args().nth(p + 1));
        // Plan 432 T0.2a (riir-train) — the TEACHER-corpus mint: per case,
        // the self-contained (seed, broken source, attempts-with-shapes+
        // digests) row. The rings alone cannot serve the teacher pass —
        // they carry no case linkage and (pre-140) no trigger text; this
        // dump is the pure-data contract the 4090 teacher harness consumes
        // with ZERO riir-clippy dep (the boundary law — riir-train cannot
        // path-dep this crate). Composable with --dump-fixseq; early-returns
        // like it.
        let dump_teacher_root: Option<String> = std::env::args().position(|a| a == "--dump-teacher")
            .and_then(|p| std::env::args().nth(p + 1));
        // Issue 128 T5 — cfg validation at the door: a cfg without a dump
        // root, or an unknown cfg name, refuses loudly (exit 2) rather than
        // minting an unlabeled corpus.
        match (&dump_fixseq_cfg, &dump_fixseq_root, &dump_teacher_root) {
            (Some(cfg), _, _) if cfg != "chunked" && cfg != "strided" => {
                eprintln!("--dump-fixseq-cfg must be chunked|strided, got '{cfg}'");
                std::process::exit(2);
            }
            (Some(_), None, None) => {
                eprintln!("--dump-fixseq-cfg requires --dump-fixseq <dir>");
                std::process::exit(2);
            }
            _ => {}
        }
        driver_section(dump_fixseq_root, dump_fixseq_cfg, dump_teacher_root);
    }
    if want("null") {
        null_section();
    }
    if want("seeds") {
        seeds_section();
    }
    if want("insight") {
        insight_ab_section();
    }
    if want("ab") {
        #[cfg(feature = "latent_retrieval")]
        ordering_ab_section();
        #[cfg(not(feature = "latent_retrieval"))]
        println!("\n── 5. SKIP ab: needs --features latent_retrieval ──");
    }
}

fn case_count() -> usize {
    MUTANT_CASES.len()
        + generate_corpus().len()
        + riir_clippy::domains::rustc_errors::corpus_interacting::interacting_pairs().len()
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

/// One measurement arm over the full corpus — shared scaffold/floor logic
/// for the floor + driver sections (the null arm reuses it too).
fn measure_full(
    harness: &FloorHarness,
    label: &str,
    measure: impl Fn(&FloorHarness) -> Result<FloorReport, String>,
) -> Option<FloorReport> {
    println!(
        "\n── {label}: over {n} known-inverse mutants (12 anchors + {gen} generated) ──\n",
        n = case_count(),
        gen = case_count() - MUTANT_CASES.len()
    );
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return None;
    }
    let report = match measure(harness) {
        Ok(r) => r,
        Err(e) => {
            println!("  ERROR: {e}");
            return None;
        }
    };
    print_report(&report);
    print_integrity(&report);
    Some(report)
}

fn floor_section() {
    let harness = FloorHarness::new();
    let report = measure_full(&harness, "2. G-floor: `cargo fix --broken-code`", |h| {
        h.measure_cases(&CargoFixBaseline::new(), &floor_case_list())
    });
    if let Some(report) = report {
        let diverged: Vec<&str> = report
            .outcomes
            .iter()
            .filter(|o| o.verdict == FixVerdict::Diverged)
            .map(|o| o.name.as_str())
            .collect();
        if !diverged.is_empty() {
            println!(
                "\n  {} repair(s) compiled but DIVERGED from the known inverse — the \
                 review surface a ranked-alternatives fixer owns.",
                diverged.len()
            );
        }
    }
    harness.cleanup();
}

fn driver_section(
    dump_fixseq_root: Option<String>,
    dump_fixseq_cfg: Option<String>,
    dump_teacher_root: Option<String>,
) {
    let harness = FloorHarness::new();
    let fixer = DriverFixer::new();
    // Issue 128 T5 — a mint config REPLACES the stock floor list with the
    // composed multi-error worlds (+ the interacting pairs); the flag-less
    // path is byte-identical to before (G3: the stock measurement session
    // is untouched).
    let cases = match &dump_fixseq_cfg {
        Some(cfg) => composed_case_list(cfg).unwrap_or_else(|| {
            eprintln!("unknown --dump-fixseq-cfg '{cfg}'");
            std::process::exit(2);
        }),
        None => floor_case_list(),
    };
    let cfg_note = dump_fixseq_cfg
        .as_deref()
        .map(|c| format!(" [mint cfg: {c}, Issue 128 T5]"))
        .unwrap_or_default();
    println!(
        "\n── 3. The `--fix-compile` arm (verbose): over {n} known-inverse mutants{cfg_note} ──\n",
        n = cases.len()
    );
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    let (report, driver_reports) = match harness.measure_driver(&fixer, &cases) {
        Ok(r) => r,
        Err(e) => {
            println!("  ERROR: {e}");
            return;
        }
    };
    print_report(&report);
    print_integrity(&report);
    dump_diverged(&cases, &report);

    // Plan 432 T0.2a — the teacher-corpus mint. One self-contained row per
    // ATTEMPTED case: the seed (ground truth), the broken source (the task
    // the driver saw — the teacher re-derives diagnostics from it in a
    // fresh scaffold), and the FULL attempt ring — the T3 fields ride every
    // attempt now (shape/strategy/outcome/shape_class/insight), so the
    // fail→resolve scaffold has the failed EDIT and the corrected EDIT, not
    // just their ids. Unattempted cases (first-check clean) carry nothing
    // the teacher needs.
    if let Some(root) = dump_teacher_root {
        use std::path::PathBuf;
        let root = PathBuf::from(root);
        let _ = std::fs::create_dir_all(&root);
        let by_name: std::collections::HashMap<&str, &riir_clippy::domains::rustc_errors::floor::HarnessCase<'_>> =
            cases.iter().map(|c| (c.name, c)).collect();
        let mut rows = 0usize;
        let mut attempts_total = 0usize;
        let path = root.join("teacher_cases.jsonl");
        let mut out = String::new();
        for (name, rep) in &driver_reports {
            if rep.attempts.is_empty() {
                continue;
            }
            let Some(case) = by_name.get(name.as_str()) else { continue };
            // The edit-level legs (Plan 432's scaffold wants the FAILED edit
            // and the CORRECTED edit, not just their strategy ids): the
            // applied/reverted records carry shape + fix text verbatim.
            let applied: Vec<_> = rep
                .applied
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "code": a.code, "strategy_id": a.strategy_id,
                        "line": a.line, "shape": a.shape, "fix": a.fix,
                        "errors": a.errors,
                    })
                })
                .collect();
            let reverted: Vec<_> = rep
                .reverted
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "code": r.code, "strategy_id": r.strategy_id,
                        "line": r.line, "shape": r.shape, "fix": r.fix,
                        "reason": r.reason.label(),
                    })
                })
                .collect();
            let row = serde_json::json!({
                "case": case.name,
                "expect_code": case.expect_code,
                "seed": case.seed,
                "broken": case.broken,
                "attempts": rep.attempts,
                "applied": applied,
                "reverted": reverted,
            });
            out.push_str(&serde_json::to_string(&row).unwrap_or_default());
            out.push('\n');
            rows += 1;
            attempts_total += rep.attempts.len();
        }
        match std::fs::write(&path, out) {
            Ok(()) => println!(
                "\n  teacher dump: {rows} case row(s) ({attempts_total} attempts, T3 digests \
                 stamped) -> {}",
                path.display()
            ),
            Err(e) => eprintln!("teacher dump: {e}"),
        }
        // The house-data manifest (per-file BLAKE3 + counts — the 128-T5 law).
        let bytes = std::fs::read(&path).unwrap_or_default();
        let manifest = serde_json::json!({
            "plan": "432-T0.2a",
            "rustc": rustc_version(),
            "rows": rows,
            "attempts": attempts_total,
            "blake3": blake3::hash(&bytes).to_hex().to_string(),
            "created_unix": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
        let mpath = root.join("manifest.json");
        let _ = std::fs::write(&mpath, serde_json::to_string_pretty(&manifest).unwrap_or_default());
        println!("  manifest: {}", mpath.display());
        harness.cleanup();
        return;
    }

    // Issue 126 T0 — optional fixseq ring dump: the hindsight-label corpus
    // (`span_embed` × `strategy_id` × `outcome`) the sigmoid-NCE rule-embedding
    // trainer trains on. Early-returns BEFORE the G1 semantics / G2 latency
    // passes: the label-gen posture wants the rings, not the measurement
    // tables (the flag-less path is unchanged — the committed corpus run
    // keeps its full measurement session).
    if let Some(root) = dump_fixseq_root {
        use riir_clippy::domains::rustc_errors::driver::AttemptOutcome;
        use riir_clippy::domains::rustc_errors::evolve::dump_fixseq;
        use std::path::PathBuf;
        let root = PathBuf::from(root);
        let mut runs = 0usize;
        let mut attempts_total = 0usize;
        let mut kept = 0usize;
        let mut reverted = 0usize;
        let mut with_embed = 0usize;
        let mut codes = std::collections::BTreeSet::new();
        let mut strategies = std::collections::BTreeSet::new();
        for (name, rep) in &driver_reports {
            if rep.attempts.is_empty() {
                continue;
            }
            let (written, warnings) = dump_fixseq(&root, &rep.attempts);
            for w in &warnings {
                eprintln!("fixseq dump {name}: {w}");
            }
            if written.is_none() {
                continue;
            }
            runs += 1;
            attempts_total += rep.attempts.len();
            for a in &rep.attempts {
                match a.outcome {
                    AttemptOutcome::Kept => kept += 1,
                    AttemptOutcome::RevertedNoProgress | AttemptOutcome::RevertedIo => reverted += 1,
                }
                if a.span_embed.is_some() {
                    with_embed += 1;
                }
                codes.insert(a.code.clone());
                strategies.insert(a.strategy_id.clone());
            }
        }
        println!(
            "\n  fixseq dump: {runs} run file(s) -> {}/.heal/fixseq/ ; attempts {attempts_total} \
             (kept {kept} POSITIVES, reverted {reverted} HARD-NEGATIVES); span_embed stamped \
             {with_embed}/{attempts_total}; {c} error code(s), {s} strategy(ies)",
            root.display(),
            c = codes.len(),
            s = strategies.len()
        );
        // Issue 128 T5 — the house-data manifest: per-file BLAKE3 + counts,
        // stamped with the mint config (the 049_t01 law).
        if let Some(cfg) = &dump_fixseq_cfg {
            match write_ring_manifest(&root, cfg) {
                Ok(p) => println!("  manifest: {}", p.display()),
                Err(e) => eprintln!("  manifest: {e}"),
            }
        }
        harness.cleanup();
        return;
    }

    // G1 semantics pass: every repaired case's scaffold holds the final
    // source; run its ground-truth behavior test.
    println!("\n  G1 semantics pass: `cargo test` post-fix on every repaired case");
    let mut tested = 0usize;
    let mut green = 0usize;
    let mut red: Vec<String> = Vec::new();
    for o in &report.outcomes {
        if o.verdict == FixVerdict::Unfixed {
            continue;
        }
        let dir = harness.work_root().join(&o.name);
        match harness.cargo_test_passes(&dir) {
            Err(e) => println!("    {} cargo-test error: {e}", o.name),
            Ok(false) => red.push(o.name.clone()),
            Ok(true) => green += 1,
        }
        tested += 1;
    }
    println!(
        "    {green}/{tested} repaired cases keep the seed's behavior; {} broke it{}",
        red.len(),
        match red.is_empty() {
            true => String::new(),
            false => format!(": {}", red.join(", ")),
        }
    );

    // G2 latency: per-case fixer wall time + per-iteration driver time.
    let mut fix_ms: Vec<f64> = report.outcomes.iter().map(|o| o.fix_ms).collect();
    fix_ms.sort_by(|a, b| a.total_cmp(b));
    if !fix_ms.is_empty() {
        println!(
            "\n  G2 latency, per CASE (ms): p50 {:.0}  p99 {:.0}  max {:.0}",
            percentile(&fix_ms, 0.50),
            percentile(&fix_ms, 0.99),
            fix_ms[fix_ms.len() - 1]
        );
    }
    let mut iter_ms: Vec<f64> = driver_reports
        .iter()
        .flat_map(|(_, r)| r.iter_ms.iter().copied())
        .collect();
    iter_ms.sort_by(|a, b| a.total_cmp(b));
    if !iter_ms.is_empty() {
        println!(
            "  G2 latency, per ITERATION (ms): n {}  p50 {:.0}  p99 {:.0}  max {:.0}",
            iter_ms.len(),
            percentile(&iter_ms, 0.50),
            percentile(&iter_ms, 0.99),
            iter_ms[iter_ms.len() - 1]
        );
    }
    harness.cleanup();
}

fn null_section() {
    let harness = FloorHarness::new();
    let measured = measure_full(&harness, "4. Harness self-check: the no-op arm", |h| {
        h.measure_cases(&NullFixer, &floor_case_list())
    });
    if let Some(report) = measured {
        let rate = report.total_fix_rate();
        println!(
            "  no-op arm fix rate: {:.1}% — {}",
            rate * 100.0,
            match rate == 0.0 {
                true => "PASS (every mutant genuinely broken)",
                false => "FAIL — the harness is scoring something other than the fixer",
            }
        );
    }
    harness.cleanup();
}

/// Seed-validity: every seed's own ground-truth test must pass (the corpus
/// ground truth is only as good as this check — the first draft shipped 10
/// hand-counted-string-length seeds whose tests failed, caught only by the
/// post-fix pass because unrepaired cases never get tested).
fn seeds_section() {
    println!("\n── Seed validity: every seed's ground-truth test must pass ──\n");
    let harness = FloorHarness::new();
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    let cases = floor_case_list();
    match harness.verify_seed_tests(&cases) {
        Err(e) => println!("  ERROR: {e}"),
        Ok(bad) => match bad.is_empty() {
            true => println!("  PASS — {}/{} seed tests green", cases.len(), cases.len()),
            false => println!(
                "  FAIL — {} seed(s) with failing ground truth:\n    {}",
                bad.len(),
                bad.join("\n    ")
            ),
        },
    }
    harness.cleanup();
}

// ── section 5: the G6/G7 ordering A/B (latent_retrieval builds) ───────

/// The template-covered stratum: codes our playbooks propose for. The
/// delegated contested codes (E0596/E0382/E0507/E0373) have exactly one
/// "candidate" (none) — ordering cannot matter there.
#[cfg(feature = "latent_retrieval")]
fn template_covered<'a>(
    committed: &'a [riir_clippy::domains::rustc_errors::floor::MutantCase],
    generated: &'a [riir_clippy::domains::rustc_errors::corpus_gen::GeneratedCase],
) -> (
    Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'a>>,
    usize,
) {
    let mut cases = Vec::new();
    let mut n = 0;
    for c in committed {
        if matches!(c.expect_code, "E0597" | "E0499" | "E0502" | "E0505") {
            if let Ok(h) = riir_clippy::domains::rustc_errors::floor::HarnessCase::committed(c) {
                cases.push(h);
                n += 1;
            }
        }
    }
    for g in generated {
        if matches!(g.expect_code, "E0597" | "E0499" | "E0502" | "E0505") {
            cases.push(riir_clippy::domains::rustc_errors::floor::HarnessCase::generated(g));
            n += 1;
        }
    }
    (cases, n)
}

// ── section 6: Issue 140 T3 — the insight-conditioned retry A/B ─────

/// The paired readout primitive: per-case wins/losses (McNemar's b/c)
/// plus the one-sided 95% lower bound on the mean paired difference
/// (`clean_a − clean_b` ∈ {−1, 0, +1} per case). `n < 2` or zero variance
/// reads UNDEFINED — printed as such, never as a pass.
fn paired_lb95(
    a: &[(String, bool)],
    b: &[(String, bool)],
) -> (usize, usize, f64) {
    let bm: std::collections::HashMap<&str, bool> =
        b.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut diffs: Vec<f64> = Vec::with_capacity(a.len());
    for (k, va) in a {
        let Some(vb) = bm.get(k.as_str()) else { continue };
        let d = f64::from(u8::from(*va)) - f64::from(u8::from(*vb));
        if d > 0.0 {
            wins += 1;
        } else if d < 0.0 {
            losses += 1;
        }
        diffs.push(d);
    }
    let n = diffs.len();
    if n < 2 {
        return (wins, losses, f64::NAN);
    }
    let mean = diffs.iter().sum::<f64>() / n as f64;
    let var = diffs.iter().map(|d| (d - mean) * (d - mean)).sum::<f64>() / (n - 1) as f64;
    let lb = mean - 1.645 * var.sqrt() / (n as f64).sqrt();
    (wins, losses, lb)
}

/// The EFFICIENCY-axis paired LB95: per-case `attempts_b − attempts_a`
/// (POSITIVE ⇒ `a` reached its verdict on fewer apply→recheck decisions —
/// the axis that discriminates when every arm eventually cleans; the
/// clean axis saturates on corpora where all arms converge).
fn paired_attempts_lb95(
    a: &[(String, usize)],
    b: &[(String, usize)],
) -> f64 {
    let bm: std::collections::HashMap<&str, usize> =
        b.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    let mut diffs: Vec<f64> = Vec::with_capacity(a.len());
    for (k, va) in a {
        let Some(vb) = bm.get(k.as_str()) else { continue };
        diffs.push(*vb as f64 - *va as f64);
    }
    let n = diffs.len();
    if n < 2 {
        return f64::NAN;
    }
    let mean = diffs.iter().sum::<f64>() / n as f64;
    let var = diffs.iter().map(|d| (d - mean) * (d - mean)).sum::<f64>() / (n - 1) as f64;
    mean - 1.645 * var.sqrt() / (n as f64).sqrt()
}

/// Issue 140 T3 — the REQUIRED control-arm eval: insight-conditioning vs
/// `with_ordering`-shaped statistics over the SAME (error-code,
/// shape-class) features, paired, at equal attempt budget; plus the
/// negative control (site-keyed raw conditioning) and the static baseline.
///
/// Each arm carries a FRESH policy whose store accumulates DURING the
/// arm's own corpus pass — cross-case transfer inside the eval, the
/// paper's own measurement shape. The corpus flags are the ab section's
/// (`--multi K`, `--strided`, `--interacting`).
fn insight_ab_section() {
    use riir_clippy::domains::rustc_errors::insight_retry::{InsightConditioning, Mode};
    use riir_clippy::domains::rustc_errors::floor::DriverFixer;

    println!("\n── 6. Issue 140 T3 — insight-conditioned retry A/B (static vs control vs insight vs raw) ──\n");
    let harness = FloorHarness::new();
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    let multi: usize = std::env::args()
        .position(|a| a == "--multi")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let strided = std::env::args().any(|a| a == "--strided");
    let interacting = std::env::args().any(|a| a == "--interacting");
    let generated = if interacting {
        riir_clippy::domains::rustc_errors::corpus_interacting::interacting_pairs()
            .iter()
            .map(riir_clippy::domains::rustc_errors::corpus_interacting::InteractingCase::as_generated)
            .collect::<Vec<_>>()
    } else {
        match (multi > 1, strided) {
            (true, true) => riir_clippy::domains::rustc_errors::corpus_gen::compose_multi_strided(
                &generate_corpus(),
                multi,
            ),
            (true, false) => riir_clippy::domains::rustc_errors::corpus_gen::compose_multi(
                &generate_corpus(),
                multi,
            ),
            (false, _) => generate_corpus(),
        }
    };
    let (cases, n): (Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'_>>, usize) =
        match multi > 1 || interacting {
            true => {
                let v: Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'_>> = generated
                    .iter()
                    .map(riir_clippy::domains::rustc_errors::floor::HarnessCase::generated)
                    .collect();
                let n = v.len();
                (v, n)
            }
            false => {
                // The template-covered stratum: codes our playbooks propose
                // for — ordering cannot matter where the menu is empty.
                let (v, n) = template_covered_pub(MUTANT_CASES, &generated);
                (v, n)
            }
        };
    println!(
        "  corpus: {} — {n} cases",
        if interacting {
            "INTERACTING pairs".to_string()
        } else {
            match multi > 1 {
                true => format!("MULTI-{multi} ({})", if strided { "strided" } else { "chunked" }),
                false => "template-covered stratum".to_string(),
            }
        }
    );

    // The headroom instrument (the Bench-088 law, carried over verbatim:
    // `multi-attempt runs == 0` means this corpus cannot measure ANY
    // stateful policy, independent of arm quality).
    type ArmOut = (
        Vec<(String, bool)>,
        Vec<(String, usize)>,
        usize,
        usize,
        usize,
        usize,
    );
    let run_arm = |label: &str, fixer: &DriverFixer| -> ArmOut {
        let (_, reports) = harness.measure_driver(fixer, &cases).expect(label);
        let mut clean_by_case: Vec<(String, bool)> = Vec::new();
        let mut attempts_by_case: Vec<(String, usize)> = Vec::new();
        let (mut total_attempts, mut multi_attempt_runs, mut total_reverts, mut fixed_clean) =
            (0usize, 0usize, 0usize, 0usize);
        for (name, rep) in &reports {
            total_attempts += rep.attempts.len();
            total_reverts += rep.reverted.len();
            if rep.attempts.len() > 1 {
                multi_attempt_runs += 1;
            }
            if rep.clean {
                fixed_clean += 1;
            }
            clean_by_case.push((name.clone(), rep.clean));
            attempts_by_case.push((name.clone(), rep.attempts.len()));
        }
        println!(
            "  {label:<28} clean {fixed_clean}/{n}  reverts {total_reverts}  attempts {total_attempts}  multi-attempt runs {multi_attempt_runs}"
        );
        (
            clean_by_case,
            attempts_by_case,
            fixed_clean,
            total_reverts,
            total_attempts,
            multi_attempt_runs,
        )
    };

    let static_arm = run_arm("static (min-delta)", &DriverFixer::new());

    let policy_arm = |label: &str, mode: Mode| {
        let pol = InsightConditioning::new(mode);
        let fixer = DriverFixer::new()
            .with_ordering(pol.ordering())
            .with_observer(pol.observer());
        let out = run_arm(label, &fixer);
        let st = pol.stats();
        println!(
            "    policy: recorded {} refused-admission {} families {} armed-now {} gate-arms {} gate-releases {}",
            st.recorded, st.refused_admission, st.families, st.armed, st.gate_arms, st.gate_releases
        );
        out
    };
    let control_arm = policy_arm("control (code+shape-class)", Mode::Control);
    let insight_arm = policy_arm("insight-conditioned", Mode::Insight);
    let raw_arm = policy_arm("raw (site-keyed, neg-ctl)", Mode::Raw);

    let verdict = |la: &str, a: &ArmOut, lb: &str, b: &ArmOut| {
        let (wins, losses, lb95) = paired_lb95(&a.0, &b.0);
        let eff = paired_attempts_lb95(&a.1, &b.1);
        let read = if wins + losses == 0 {
            "UNDEFINED (no discordant pairs — McNemar has nothing to test)"
        } else if lb95.is_nan() {
            "UNDEFINED (n < 2)"
        } else if lb95 > 0.0 {
            "EARNED (paired LB95 > 0)"
        } else {
            "NOT EARNED (paired LB95 ≤ 0)"
        };
        let eff_read = if eff.is_nan() {
            "UNDEFINED".to_string()
        } else {
            format!(
                "{eff:+.3} attempts/case ({})",
                match eff > 0.0 {
                    true => format!("{la} cheaper, LB95 > 0"),
                    false => format!("{la} NOT cheaper, LB95 ≤ 0"),
                }
            )
        };
        println!(
            "    {la} vs {lb}: {wins} wins / {losses} losses  paired-LB95 {lb95:+.4} — {read};  efficiency {eff_read}"
        );
        (wins, losses, lb95)
    };

    println!("\n  THE CONTROL ARM (conditioning vs ordering — the claim lives or dies here):");
    let ctl = verdict("insight", &insight_arm, "control", &control_arm);
    println!("\n  NEGATIVE CONTROL (raw conditioning must NOT beat insight):");
    let raw = verdict("insight", &insight_arm, "raw", &raw_arm);
    println!("\n  OVERALL (insight vs static):");
    let stat = verdict("insight", &insight_arm, "static", &static_arm);

    let multi_runs = static_arm.5 + control_arm.5 + insight_arm.5 + raw_arm.5;
    println!(
        "\n  HEADROOM PREDICATE: multi-attempt runs across all arms = {multi_runs} — {}",
        match multi_runs {
            0 => "ZERO ⇒ this corpus cannot measure ANY stateful policy; UNMEASURABLE, not refuted",
            _ => "NON-ZERO ⇒ the gain axis is measurable on this corpus",
        }
    );
    let _ = (ctl, raw, stat);
    harness.cleanup();
}

/// `template_covered` for the insight section — the ab section's own fn is
/// `latent_retrieval`-gated; this section needs no store, so it carries its
/// own copy of the stratum filter (same codes, same order).
fn template_covered_pub<'a>(
    committed: &'a [riir_clippy::domains::rustc_errors::floor::MutantCase],
    generated: &'a [riir_clippy::domains::rustc_errors::corpus_gen::GeneratedCase],
) -> (
    Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'a>>,
    usize,
) {
    let mut cases = Vec::new();
    let mut n = 0;
    for c in committed {
        if matches!(c.expect_code, "E0597" | "E0499" | "E0502" | "E0505") {
            if let Ok(h) = riir_clippy::domains::rustc_errors::floor::HarnessCase::committed(c) {
                cases.push(h);
                n += 1;
            }
        }
    }
    for g in generated {
        if matches!(g.expect_code, "E0597" | "E0499" | "E0502" | "E0505") {
            cases.push(riir_clippy::domains::rustc_errors::floor::HarnessCase::generated(g));
            n += 1;
        }
    }
    (cases, n)
}

#[cfg(feature = "latent_retrieval")]
fn ordering_ab_section() {
    use riir_clippy::domains::rustc_errors::evolve::traj_key;
    use riir_clippy::traj_store::{shape_conditional_order, TrajStore};
    use std::collections::HashMap;
    use std::sync::Arc;

    println!("\n── 5. G6/G7 ordering A/B (static vs flat vs shape-conditional) ──\n");
    let harness = FloorHarness::new();
    if !harness.is_available() {
        println!("  SKIP — cargo not on PATH.");
        return;
    }
    // `--multi K` composes K single-mutation cases into ONE scaffold (Bench
    // 088; the composition is `corpus_gen::compose_multi`, shared with the
    // Issue 91 T2 prevalence runner's power lever). The stock corpus makes
    // exactly ONE apply→recheck decision per run, and a WITHIN-run posterior
    // needs >= 2 — so the K=1 lane can only ever report a green zero about
    // itself. K >= 2 is the lane that can actually adjudicate the gain axis.
    let multi: usize = std::env::args()
        .position(|a| a == "--multi")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    // `--strided` draws the K members by stride rather than from a
    // contiguous chunk, so one scaffold carries K different FAMILIES. Bench
    // 088 measured why it matters: `generate_corpus()` emits families
    // contiguously, so the chunked form puts a WIDTH-1 strategy menu in
    // front of the ordering seam and every ordering policy is the identity
    // on it no matter how deep the run goes.
    let strided = std::env::args().any(|a| a == "--strided");
    // `--interacting` (Issue 099 T4) swaps the corpus for the INTERACTING
    // pair family: both members' errors share one binding, so repairing
    // one changes the other's diagnostics — the corpus where a candidate
    // ORDER (and a revert) can be load-bearing. Mutually exclusive with
    // `--multi` by construction (the interacting cases ARE composed).
    let interacting = std::env::args().any(|a| a == "--interacting");
    let generated = if interacting {
        riir_clippy::domains::rustc_errors::corpus_interacting::interacting_pairs()
            .iter()
            .map(riir_clippy::domains::rustc_errors::corpus_interacting::InteractingCase::as_generated)
            .collect::<Vec<_>>()
    } else {
        match (multi > 1, strided) {
            (true, true) => riir_clippy::domains::rustc_errors::corpus_gen::compose_multi_strided(
                &generate_corpus(),
                multi,
            ),
            (true, false) => riir_clippy::domains::rustc_errors::corpus_gen::compose_multi(
                &generate_corpus(),
                multi,
            ),
            (false, _) => generate_corpus(),
        }
    };
    let (cases, n) = match multi > 1 || interacting {
        // Composed scaffolds carry several codes at once; the stratum filter
        // (which keys on a single declared code) would drop most of them, so
        // the MULTI lane takes the composed set whole.
        true => {
            let v: Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'_>> = generated
                .iter()
                .map(riir_clippy::domains::rustc_errors::floor::HarnessCase::generated)
                .collect();
            let n = v.len();
            (v, n)
        }
        false => template_covered(MUTANT_CASES, &generated),
    };
    println!(
        "  corpus: {} — {n} cases",
        if interacting {
            "INTERACTING pairs (Issue 099; shared-binding members, admissibility-gated)".to_string()
        } else {
            match multi > 1 {
                true => format!(
                    "MULTI-{multi} ({}, {multi} simultaneous errors per scaffold)",
                    match strided {
                        true => "STRIDED — k distinct families per scaffold",
                        false => "chunked — usually k variants of ONE family",
                    }
                ),
                false => "template-covered stratum (stock, 1 error per scaffold)".to_string(),
            }
        }
    );

    // Fresh store for a self-contained measurement.
    let root = std::env::temp_dir().join(format!("riir_floor_ab_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (store, bad) = TrajStore::open(&root).expect("temp traj store");
    assert_eq!(bad, 0);

    // ── Learning pass (static ordering): populate the store ──
    let learning = DriverFixer::new();
    let mut learned = 0usize;
    let (_, reports) = harness
        .measure_driver(&learning, &cases)
        .expect("learning pass");
    for (name, rep) in &reports {
        for a in &rep.applied {
            store
                .record(
                    &traj_key(&a.code, &a.strategy_id),
                    &a.shape,
                    &a.fix,
                    true,
                    true,
                    "floor-learning",
                )
                .expect("record applied");
            learned += 1;
        }
        for r in &rep.reverted {
            store
                .record(
                    &traj_key(&r.code, &r.strategy_id),
                    &r.shape,
                    &r.fix,
                    false,
                    false,
                    "floor-learning",
                )
                .expect("record reverted");
            learned += 1;
        }
        let _ = name;
    }
    println!("  learning pass: {learned} trajectories recorded");

    // ── Arm builders ──
    // Flat: BetaPosterior over (code,strategy) counters, shape-blind —
    // built from the store dump (the JSON-backend fallback's semantics).
    let mut flat_counts: HashMap<String, (u32, u32)> = HashMap::new();
    for row in store.dump() {
        let e = flat_counts.entry(row.lint_key.clone()).or_default();
        e.0 = e.0.saturating_add(row.success_count);
        e.1 = e.1.saturating_add(row.fail_count);
    }
    let flat_ordering = Arc::new(
        move |cands: &mut [riir_clippy::domains::rustc_errors::playbooks::FixCandidate]| {
            let score = |c: &riir_clippy::domains::rustc_errors::playbooks::FixCandidate| {
                match flat_counts.get(&traj_key(&c.code, &c.strategy_id)) {
                    None => 0.32, // Beta(1,1) @ eps — the unseen prior
                    Some(&(s, f)) => katgpt_core::best_belief_score(s, f, 0.32),
                }
            };
            cands.sort_by(|a, b| {
                score(b)
                    .total_cmp(&score(a))
                    .then(a.total_len().cmp(&b.total_len()))
                    .then(a.strategy_id.cmp(&b.strategy_id))
            });
        },
    );

    // HEADROOM instrument (Issue 093 T2 / Bench 088). `attempts` is the
    // ring length — the number of apply→recheck DECISIONS a run actually
    // made. An ordering only matters from decision 1, and a WITHIN-run
    // posterior only from decision 2, so `multi-attempt runs == 0` means
    // this corpus cannot measure ANY stateful ordering, independent of any
    // arm's quality. Printed next to every arm so a saturated corpus can
    // never read as "no difference" (Bench 045 §G6/G7(b) asserted the
    // saturation from `iterations`; this prints the decision count that
    // makes it mechanical).
    let arm_stats = |label: &str, fixer: &DriverFixer| {
        let (_, reports) = harness.measure_driver(fixer, &cases).expect(label);
        let mut total_iters = 0usize;
        let mut total_reverts = 0usize;
        let mut total_attempts = 0usize;
        let mut multi_attempt_runs = 0usize;
        let mut fixed_clean = 0usize;
        let mut clean_by_case: Vec<(String, bool)> = Vec::new();
        for (name, rep) in &reports {
            total_iters += rep.iterations;
            total_reverts += rep.reverted.len();
            total_attempts += rep.attempts.len();
            if rep.attempts.len() > 1 {
                multi_attempt_runs += 1;
            }
            if rep.clean {
                fixed_clean += 1;
            }
            clean_by_case.push((name.clone(), rep.clean));
        }
        println!(
            "  {label:<22} clean {fixed_clean}/{n}  iterations {total_iters}  reverts {total_reverts}  attempts {total_attempts}  multi-attempt runs {multi_attempt_runs}"
        );
        (
            fixed_clean,
            total_iters,
            total_reverts,
            clean_by_case,
            multi_attempt_runs,
        )
    };

    let store_arc = Arc::new(store);
    let shape_fixer = {
        let s = Arc::clone(&store_arc);
        DriverFixer::new().with_ordering(Arc::new(move |cands| {
            shape_conditional_order(cands, &s);
        }))
    };
    let flat_fixer = DriverFixer::new().with_ordering(flat_ordering);

    let static_arm = arm_stats("static (min-delta)", &DriverFixer::new());
    let flat_arm = arm_stats("flat counters", &flat_fixer);
    let shape_arm = arm_stats("shape-conditional", &shape_fixer);

    // ── Issue 093 T2's policy at THIS harness (Bench 088). The landed
    // wiring is gated by tests/nonergodic_driver_ab.rs (mechanism + a
    // 2-scaffold e2e); this is the G6/G7 corpus lane the issue originally
    // designated, at n = 126. Two arms, because they answer different
    // questions: `shared` keeps ONE policy across the corpus (learning
    // across RUNS — evidence the deployed CLI never has) and `per-run`
    // rebuilds it per case (the deployed shape: one `new()` per
    // invocation, one driver run). ──
    #[cfg(feature = "nonergodic_driver")]
    {
        use riir_clippy::domains::rustc_errors::nonergodic_order::NonergodicOrdering;
        use std::sync::atomic::{AtomicUsize, Ordering as AtomOrd};

        let shared = Arc::new(NonergodicOrdering::new(Some(Arc::clone(&store_arc))));
        let shared_fixer = DriverFixer::new()
            .with_ordering(shared.ordering())
            .with_observer(shared.observer())
            .with_revive(shared.revive());
        let neo_shared_arm = arm_stats("nonergodic shared", &shared_fixer);
        let snap = shared.snapshot();
        println!(
            "    shared policy snapshot: menu_len {} t_observed {} committed {:?} store_stale {}",
            snap.menu_len, snap.t_observed, snap.committed, snap.store_stale
        );

        let per_run_arm = {
            let ticks = Arc::new(AtomicUsize::new(0));
            // MENU-WIDTH instrument: the policy freezes its menu (the set of
            // DISTINCT strategy_ids among the candidates) at the first
            // ordering call. A menu of width 1 leaves the score constant
            // across candidates, so the incumbent min-delta tiebreak decides
            // everything — the seam has no choice to make, for ANY policy.
            let mut menu_hist: std::collections::BTreeMap<usize, usize> =
                std::collections::BTreeMap::new();
            let mut t_observed_total = 0usize;
            let mut total_iters = 0usize;
            let mut total_reverts = 0usize;
            let mut total_attempts = 0usize;
            let mut multi_attempt_runs = 0usize;
            let mut fixed_clean = 0usize;
            let mut clean_by_case: Vec<(String, bool)> = Vec::new();
            for i in 0..cases.len() {
                let pol = Arc::new(NonergodicOrdering::new(Some(Arc::clone(&store_arc))));
                // LIVENESS sentinel: an arm whose observer never fires is
                // INERT, and an inert arm reads exactly like "no difference".
                let inner = pol.observer();
                let tk = Arc::clone(&ticks);
                let counting: riir_clippy::domains::rustc_errors::driver::ObserverFn =
                    Arc::new(move |rec| {
                        tk.fetch_add(1, AtomOrd::Relaxed);
                        inner(rec);
                    });
                let fixer = DriverFixer::new()
                    .with_ordering(pol.ordering())
                    .with_observer(counting)
                    .with_revive(pol.revive());
                let (_, reports) = harness
                    .measure_driver(&fixer, &cases[i..=i])
                    .expect("nonergodic per-run");
                for (name, rep) in &reports {
                    total_iters += rep.iterations;
                    total_reverts += rep.reverted.len();
                    total_attempts += rep.attempts.len();
                    if rep.attempts.len() > 1 {
                        multi_attempt_runs += 1;
                    }
                    if rep.clean {
                        fixed_clean += 1;
                    }
                    clean_by_case.push((name.clone(), rep.clean));
                }
                let sn = pol.snapshot();
                *menu_hist.entry(sn.menu_len).or_insert(0) += 1;
                t_observed_total += sn.t_observed;
            }
            let label = "nonergodic per-run";
            println!(
                "  {label:<22} clean {fixed_clean}/{n}  iterations {total_iters}  reverts {total_reverts}  attempts {total_attempts}  multi-attempt runs {multi_attempt_runs}"
            );
            println!(
                "    observer ticks fired: {} (0 ⇒ the arm is INERT — the policy never saw evidence)",
                ticks.load(AtomOrd::Relaxed)
            );
            println!(
                "    frozen menu width histogram {menu_hist:?}  (width 1 ⇒ score constant ⇒ ordering is the incumbent tiebreak, identically)  t_observed total {t_observed_total}"
            );
            (
                fixed_clean,
                total_iters,
                total_reverts,
                clean_by_case,
                multi_attempt_runs,
            )
        };

        // Paired per-case discordants (McNemar's b/c). b = c = 0 means
        // UNDEFINED, not "not significant" — there is no discordant pair
        // to test.
        let discord = |a: &[(String, bool)], b: &[(String, bool)], la: &str, lb: &str| {
            let bm: std::collections::HashMap<&str, bool> =
                b.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            let (mut aw, mut al) = (0usize, 0usize);
            for (k, va) in a {
                match bm.get(k.as_str()) {
                    Some(vb) if *va && !*vb => aw += 1,
                    Some(vb) if !*va && *vb => al += 1,
                    _ => {}
                }
            }
            println!("    discord {la} vs {lb}: {aw} wins / {al} losses");
        };
        discord(&neo_shared_arm.3, &shape_arm.3, "nonergodic shared", "shape");
        discord(&per_run_arm.3, &shape_arm.3, "nonergodic per-run", "shape");
        discord(&per_run_arm.3, &static_arm.3, "nonergodic per-run", "static");

        let multi = static_arm.4 + shape_arm.4 + neo_shared_arm.4 + per_run_arm.4;
        println!(
            "\n  T2 GAIN AXIS: clean per-run {}/{n} vs shape {}/{n} vs static {}/{n}; reverts {} / {} / {}",
            per_run_arm.0, shape_arm.0, static_arm.0, per_run_arm.2, shape_arm.2, static_arm.2
        );
        println!(
            "  HEADROOM PREDICATE (Issue 093 T4 reopen trigger): multi-attempt runs across all arms = {multi} — {}",
            match multi {
                0 => "ZERO ⇒ this corpus cannot measure ANY stateful ordering; the gain axis is UNMEASURABLE here, NOT null-by-refutation",
                _ => "NON-ZERO ⇒ headroom exists; the gain axis is measurable on this corpus",
            }
        );
    }
    #[cfg(not(feature = "nonergodic_driver"))]
    println!("  (nonergodic arms: build with --features nonergodic_driver)");

    println!(
        "\n  verdict: shape-conditional vs static — clean {} vs {}, reverts {} vs {} — {}",
        shape_arm.0,
        static_arm.0,
        shape_arm.2,
        static_arm.2,
        match (shape_arm.0.cmp(&static_arm.0), shape_arm.2.cmp(&static_arm.2)) {
            // Improvement must hold BOTH axes: Issue 099 T4 measured a
            // corpus where the learned arm reverts less but cleans 12 FEWER
            // cases (keeps that strand the run unfixable) — a reverts-only
            // verdict printed "IMPROVEMENT" over a regression.
            (std::cmp::Ordering::Greater, _) | (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => {
                "IMPROVEMENT (T4.2 earns its keep)"
            }
            (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "no difference on this corpus",
            (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater)
            | (std::cmp::Ordering::Less, _) => {
                "WORSE (record honest negative; keep static)"
            }
        }
    );
    let _ = flat_arm;

    harness.cleanup();
    let _ = std::fs::remove_dir_all(&root);
}

// ── shared helpers ────────────────────────────────────────────────────

/// `--dump-diverged K`: print up to K diverged repairs PER ERROR CODE as a
/// seed-vs-final pair.
///
/// [`FloorOutcome::diverged_source`] has been captured on every diverged case
/// since the harness shipped and read by nothing — its own doc calls it "the
/// review artifact", and no reviewer could see it. A 100%-divergence row
/// (E0597 42/42, E0499 26/26) is ambiguous between a repair-QUALITY defect
/// and a metric artifact (`normalized` is whitespace-collapse, so a leftover
/// empty block or a reordered-but-equivalent program reads as diverged), and
/// the two are only separable by looking. Off by default (0) — this prints
/// whole source files.
fn dump_diverged(cases: &[riir_clippy::domains::rustc_errors::floor::HarnessCase<'static>], report: &FloorReport) {
    let k: usize = std::env::args()
        .position(|a| a == "--dump-diverged")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if k == 0 {
        return;
    }
    let seeds: std::collections::HashMap<&str, &str> =
        cases.iter().map(|c| (c.name, c.seed)).collect();
    println!("\n  --dump-diverged {k}: the review artifact, per code\n");
    let mut shown: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for o in &report.outcomes {
        if o.verdict != FixVerdict::Diverged {
            continue;
        }
        let n = shown.entry(o.expect_code).or_insert(0);
        if *n >= k {
            continue;
        }
        *n += 1;
        let Some(final_source) = o.diverged_source.as_deref() else {
            continue;
        };
        let Some(seed) = seeds.get(o.name.as_str()).copied() else {
            continue;
        };
        // The ground-truth test module is identical in both halves by
        // construction — drop it so the diff is the repair, not the fixture.
        let cut = |s: &str| -> String {
            match s.find("#[cfg(test)]") {
                Some(i) => s[..i].trim_end().to_string(),
                None => s.trim_end().to_string(),
            }
        };
        println!("  ── {} [{}] ──", o.name, o.expect_code);
        println!("    seed:\n{}", indent(&cut(seed)));
        println!("    final:\n{}", indent(&cut(final_source)));
    }
}

fn indent(s: &str) -> String {
    s.lines()
        .map(|l| format!("      {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn floor_case_list() -> Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'static>> {
    // 'static: committed cases are 'static consts; generated cases leak
    // their Strings once per process (the measurement is a one-shot binary
    // — Box::leak keeps the borrow simple without rebuilding per section).
    // Issue 099 T5: the interacting pairs join the stock floor list (the
    // admissibility gate passed 34/34 first — `examples/interacting_gate.rs`).
    use riir_clippy::domains::rustc_errors::floor::HarnessCase;
    let mut cases: Vec<HarnessCase<'static>> = MUTANT_CASES
        .iter()
        .map(|c| HarnessCase::committed(c).expect("committed case integrity"))
        .collect();
    for g in generate_corpus() {
        cases.push(HarnessCase::generated(Box::leak(Box::new(g))));
    }
    for ix in riir_clippy::domains::rustc_errors::corpus_interacting::interacting_pairs() {
        cases.push(HarnessCase::generated(Box::leak(Box::new(ix.as_generated()))));
    }
    cases
}

/// Issue 128 T5 — the composed multi-error mint corpus for one generation
/// config. Two STRUCTURALLY different composers (chunked = adjacent
/// same-family runs per world; strided = cross-family mix per world) — the
/// T7 disjointness rule is satisfied by the CONFIG difference, never by
/// two seeds of one composer. Both configs append the interacting pairs
/// (the E0614 masking chains) and EXCLUDE the solo MUTANT_CASES anchors
/// (single-attempt runs carry no transitions).
fn composed_case_list(cfg: &str) -> Option<Vec<riir_clippy::domains::rustc_errors::floor::HarnessCase<'static>>> {
    use riir_clippy::domains::rustc_errors::corpus_gen::{compose_multi, compose_multi_strided};
    use riir_clippy::domains::rustc_errors::floor::HarnessCase;
    const K: usize = 5;
    let generated = generate_corpus();
    let composed = match cfg {
        "chunked" => compose_multi(&generated, K),
        "strided" => compose_multi_strided(&generated, K),
        _ => return None,
    };
    let mut cases: Vec<HarnessCase<'static>> =
        Vec::with_capacity(composed.len() + 4);
    for g in composed {
        cases.push(HarnessCase::generated(Box::leak(Box::new(g))));
    }
    for ix in riir_clippy::domains::rustc_errors::corpus_interacting::interacting_pairs() {
        cases.push(HarnessCase::generated(Box::leak(Box::new(ix.as_generated()))));
    }
    Some(cases)
}

/// Issue 128 T5 — the mint manifest: per-file BLAKE3 + attempt counts + the
/// config stamp, written beside the rings at `<root>/manifest.json` (the
/// house data discipline — `.benchmarks/data/049_t01/manifest.json`).
fn write_ring_manifest(root: &std::path::Path, cfg: &str) -> std::io::Result<std::path::PathBuf> {
    let dir = root.join(".heal").join("fixseq");
    let mut files = Vec::new();
    let mut total_attempts = 0usize;
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let p = entry.path();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !name.starts_with("run-") || !name.ends_with(".jsonl") {
            continue;
        }
        let bytes = std::fs::read(&p)?;
        let attempts = bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()).count();
        let hash = blake3::hash(&bytes).to_hex().to_string();
        total_attempts += attempts;
        files.push(serde_json::json!({
            "name": name,
            "bytes": bytes.len(),
            "blake3": hash,
            "attempts": attempts,
        }));
    }
    files.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    let manifest = serde_json::json!({
        "issue": "128",
        "task": "T5",
        "cfg": cfg,
        "rustc": rustc_version(),
        "created_unix": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "runs": files.len(),
        "attempts": total_attempts,
        "files": files,
    });
    let out = root.join("manifest.json");
    std::fs::write(&out, serde_json::to_string_pretty(&manifest).unwrap_or_default())?;
    Ok(out)
}

fn print_report(report: &FloorReport) {
    println!("  arm: {}\n", report.arm);
    println!(
        "  {:<8} {:>8} {:>9} {:>9} {:>8} {:>9} {:>11}",
        "code", "mutants", "matched", "diverged", "unfixed", "fix-rate", "divergence"
    );
    for row in &report.rows {
        println!(
            "  {:<8} {:>8} {:>9} {:>9} {:>8} {:>8.0}% {:>10.0}%",
            row.code,
            row.mutants,
            row.matched,
            row.diverged,
            row.unfixed,
            row.fix_rate() * 100.0,
            row.divergence_rate() * 100.0
        );
    }
    let uncontested: Vec<&FloorOutcome> = report
        .outcomes
        .iter()
        .filter(|o| {
            riir_clippy::domains::rustc_errors::corpus_gen::territory_of(o.expect_code)
                == Territory::Uncontested
        })
        .collect();
    if !uncontested.is_empty() {
        let fixed = uncontested
            .iter()
            .filter(|o| o.verdict != FixVerdict::Unfixed)
            .count();
        println!(
            "\n  UNCONTESTED stratum (the product head): {fixed}/{} fixed",
            uncontested.len()
        );
    }
    println!(
        "\n  TOTAL fix rate: {:.1}% ({} mutants)",
        report.total_fix_rate() * 100.0,
        report.outcomes.len()
    );
}

fn print_integrity(report: &FloorReport) {
    if !report.invalid_mutants.is_empty() {
        println!(
            "\n  CORPUS INTEGRITY FAILURES ({}):",
            report.invalid_mutants.len()
        );
        for bad in &report.invalid_mutants {
            println!("    - {bad}");
        }
    } else {
        println!("  corpus integrity: all mutants provoked their declared code");
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
