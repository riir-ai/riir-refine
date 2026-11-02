//! Issue 191 bar 5 — the G5 utility + ON-posture cost leg harness.
//!
//! Measures the kernel_opt domain's utility and scan cost on a pinned real
//! corpus (this repo's `src/**/*.rs` unless a root is passed):
//!
//! 1. **Match census** — per-span kernel-domain suggestions from the
//!    production scan path ([`riir_refine::draft::LatentMatcher::suggest_file`]),
//!    aggregated per category expert (rank-1 counts = the strict "fires"
//!    reading; full top-k citations disclosed alongside) plus the zero-match
//!    rule census (rules never cited anywhere — disclosed, never auto-demoted;
//!    retirement stays the `--necessity-bench` lane).
//! 2. **Scan latency** — wall time of `suggest_file` per file, one untimed
//!    warmup pass then three timed passes (per-file medians are the runner
//!    script's job). The pre-registered ceiling applies to the ON−OFF DIFF:
//!    p50 ≤ 1.0 ms/file, p95 ≤ 5.0 ms/file.
//!
//! One source, two postures: built WITH `kernel_opt` the kernel domain is
//! indexed and the census half runs (cfg-gated); built WITHOUT it the same
//! walk times the identical matcher minus the kernel index (the OFF leg) —
//! kernel citations are structurally zero there, which the JSON records.
//!
//! ```sh
//! cargo run --release --example kernel_goat_g5 -- --out /tmp/g5_off.json
//! cargo run --release --example kernel_goat_g5 --features kernel_opt -- --out /tmp/g5_on.json
//! cargo run --release --example kernel_goat_g5 --features kernel_route_narrow -- --out /tmp/g5_narrow.json
//! ```
//!
//! The posture label is cfg-derived (`posture` field) — the flat ON run and
//! the route-narrowed run emit the same schema so the runner script can
//! diff any two (Issue 196 R2/R3: the census re-run + recall delta at the
//! narrowed posture).
//!
//! No exit-code verdict here — the runner script owns the ceilings (the
//! goat_g1/g2 precedent: the harness measures, the record adjudicates).

use std::time::Instant;

#[cfg(feature = "kernel_opt")]
use riir_refine::draft::DomainId;
use riir_refine::draft::LatentMatcher;

const PASSES: usize = 3;

fn main() {
    let mut out_path: Option<String> = None;
    let mut root = std::path::PathBuf::from("src");
    let mut top_k = 5usize;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out_path = Some(args.next().unwrap_or_default()),
            "--root" => root = std::path::PathBuf::from(args.next().unwrap_or_default()),
            "--top-k" => {
                top_k = args.next().and_then(|v| v.parse().ok()).unwrap_or(5);
            }
            other => {
                eprintln!(
                    "unknown arg {other}; usage: kernel_goat_g5 [--out <json>] [--root <dir>] [--top-k <n>]"
                );
                std::process::exit(2);
            }
        }
    }

    // The pinned corpus walk (sorted + deduped, the heal.rs collect precedent).
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!("no .rs files under {}", root.display());
        std::process::exit(2);
    }

    let build_started = Instant::now();
    let mut matcher = LatentMatcher::from_enabled_domains().expect("matcher builds");
    let build_ms = build_started.elapsed().as_millis() as u64;

    // Warmup pass (untimed): the first file's cold caches never skew a median.
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        let _ = matcher.suggest_file(&src, &f.display().to_string(), top_k);
    }

    report_and_emit(
        &mut matcher,
        &files,
        top_k,
        root.display().to_string(),
        build_ms,
        out_path.as_deref(),
    );
}

/// The census + the per-pass latency table in one final timed walk (the
/// census reads the LAST pass only — caches hot, posture identical across
/// passes). The runner script computes per-file medians and the ON−OFF diff.
#[cfg(feature = "kernel_opt")]
fn report_and_emit(
    matcher: &mut LatentMatcher,
    files: &[std::path::PathBuf],
    top_k: usize,
    root: String,
    build_ms: u64,
    out_path: Option<&str>,
) {
    use riir_refine::domains::kernel_opt::corpus::build_kernel_seed_corpus;
    use riir_refine::domains::kernel_opt::router::KernelExpertRouter;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Write as _;

    let total_rules = build_kernel_seed_corpus().entries.len();

    let mut per_pass: Vec<Vec<(String, u64)>> = vec![Vec::new(); PASSES];
    let mut rank1_by_expert: BTreeMap<usize, usize> = BTreeMap::new();
    let mut cited_by_expert: BTreeMap<usize, usize> = BTreeMap::new();
    let mut cited_rules: BTreeSet<String> = BTreeSet::new();
    // Issue 196 R2: per-span rank-1 kernel rule ids in walk order (last
    // pass) — the paired disclosure the runner script diffs across ON
    // postures (the rank-1 flip census: flat vs narrowed).
    let mut rank1_rule_ids: Vec<String> = Vec::new();
    let mut kernel_spans = 0usize;
    let mut total_spans = 0usize;

    for (p, pass) in per_pass.iter_mut().enumerate() {
        for f in files {
            let Ok(src) = std::fs::read_to_string(f) else {
                continue;
            };
            let started = Instant::now();
            let spans = matcher.suggest_file(&src, &f.display().to_string(), top_k);
            let path = f
                .display()
                .to_string()
                .replace(std::path::MAIN_SEPARATOR, "/");
            pass.push((path, started.elapsed().as_micros() as u64));
            if p + 1 != PASSES {
                continue; // census reads the LAST pass only
            }
            total_spans += spans.len();
            for span in &spans {
                let Some((_, rules)) = span.by_domain.iter().find(|(d, _)| *d == DomainId::Kernel)
                else {
                    continue;
                };
                if rules.is_empty() {
                    continue;
                }
                kernel_spans += 1;
                // rank-1 = the strict "fires" reading (deterministic, no
                // threshold fitting — the no-min-score law, corpus_index.rs)
                if let Some(top) = rules.first() {
                    if let Some(e) = KernelExpertRouter::expert_for_rule(&top.rule_id) {
                        *rank1_by_expert.entry(e).or_insert(0) += 1;
                    }
                    cited_rules.insert(top.rule_id.clone());
                }
                rank1_rule_ids.push(
                    rules
                        .first()
                        .map_or(String::new(), |r| r.rule_id.clone()),
                );
                for r in rules {
                    if let Some(e) = KernelExpertRouter::expert_for_rule(&r.rule_id) {
                        *cited_by_expert.entry(e).or_insert(0) += 1;
                    }
                    cited_rules.insert(r.rule_id.clone());
                }
            }
        }
    }

    let zero_match = total_rules - cited_rules.len();
    let distinct_rank1_experts = rank1_by_expert.len();
    // The cfg-derived posture label — flat ON vs route-narrowed ON (Issue
    // 196 R2/R3 run the same harness at both ON postures).
    #[cfg(feature = "kernel_route_narrow")]
    let posture = "kernel_opt ON + kernel_route_narrow";
    #[cfg(not(feature = "kernel_route_narrow"))]
    let posture = "kernel_opt ON";

    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"leg\": \"G5 utility/cost (Issue 191 bar 5)\",").unwrap();
    writeln!(json, "  \"posture\": \"{posture}\",").unwrap();
    let root_json = root.replace(std::path::MAIN_SEPARATOR, "/");
    writeln!(json, "  \"corpus_root\": \"{root_json}\",").unwrap();
    writeln!(json, "  \"files\": {},", files.len()).unwrap();
    writeln!(json, "  \"top_k\": {top_k},").unwrap();
    writeln!(json, "  \"matcher_build_ms\": {build_ms},").unwrap();
    writeln!(json, "  \"passes\": {PASSES},").unwrap();
    writeln!(json, "  \"total_spans_last_pass\": {total_spans},").unwrap();
    writeln!(json, "  \"kernel_spans_last_pass\": {kernel_spans},").unwrap();
    writeln!(
        json,
        "  \"utility_distinct_rank1_experts\": {distinct_rank1_experts},"
    )
    .unwrap();
    write!(json, "  \"rank1_by_expert\": {{").unwrap();
    for (i, (e, c)) in rank1_by_expert.iter().enumerate() {
        let comma = if i + 1 == rank1_by_expert.len() {
            ""
        } else {
            ","
        };
        write!(json, "\"{e}\": {c}{comma} ").unwrap();
    }
    writeln!(json, "}},").unwrap();
    write!(json, "  \"cited_by_expert\": {{").unwrap();
    for (i, (e, c)) in cited_by_expert.iter().enumerate() {
        let comma = if i + 1 == cited_by_expert.len() {
            ""
        } else {
            ","
        };
        write!(json, "\"{e}\": {c}{comma} ").unwrap();
    }
    writeln!(json, "}},").unwrap();
    // Issue 196 R2: per-span rank-1 ids, walk order (last pass). Empty
    // string = the span had no kernel rank-1 (no kernel rules surfaced).
    write!(json, "  \"rank1_rule_ids\": [").unwrap();
    for (i, id) in rank1_rule_ids.iter().enumerate() {
        let sep = if i == 0 { "" } else { ", " };
        write!(json, "{sep}\"{id}\"").unwrap();
    }
    writeln!(json, "],").unwrap();
    writeln!(
        json,
        "  \"total_rules\": {total_rules}, \"cited_rules\": {}, \"zero_match_rules\": {zero_match},",
        cited_rules.len()
    )
    .unwrap();
    writeln!(json, "  \"per_pass_us\": [").unwrap();
    for (p, pass) in per_pass.iter().enumerate() {
        write!(json, "    [").unwrap();
        for (i, (path, us)) in pass.iter().enumerate() {
            let sep = if i == 0 { "" } else { ", " };
            write!(json, "{sep}[\"{path}\", {us}]").unwrap();
        }
        let comma = if p + 1 == PASSES { "" } else { "," };
        writeln!(json, "]{comma}").unwrap();
    }
    writeln!(json, "  ]").unwrap();
    writeln!(json, "}}").unwrap();

    println!("{json}");
    if let Some(p) = out_path {
        std::fs::write(p, &json).expect("write report");
    }
}

/// The OFF posture: the identical walk minus the kernel index. Emits the
/// latency table only (kernel citations are structurally zero — no index).
#[cfg(not(feature = "kernel_opt"))]
fn report_and_emit(
    matcher: &mut LatentMatcher,
    files: &[std::path::PathBuf],
    top_k: usize,
    root: String,
    build_ms: u64,
    out_path: Option<&str>,
) {
    use std::fmt::Write as _;
    let mut per_pass: Vec<Vec<(String, u64)>> = vec![Vec::new(); PASSES];
    for pass in per_pass.iter_mut() {
        for f in files {
            let Ok(src) = std::fs::read_to_string(f) else {
                continue;
            };
            let started = Instant::now();
            let _ = matcher.suggest_file(&src, &f.display().to_string(), top_k);
            let path = f
                .display()
                .to_string()
                .replace(std::path::MAIN_SEPARATOR, "/");
            pass.push((path, started.elapsed().as_micros() as u64));
        }
    }
    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"leg\": \"G5 utility/cost (Issue 191 bar 5)\",").unwrap();
    writeln!(json, "  \"posture\": \"kernel_opt OFF\",").unwrap();
    let root_json = root.replace(std::path::MAIN_SEPARATOR, "/");
    writeln!(json, "  \"corpus_root\": \"{root_json}\",").unwrap();
    writeln!(json, "  \"files\": {},", files.len()).unwrap();
    writeln!(json, "  \"top_k\": {top_k},").unwrap();
    writeln!(json, "  \"matcher_build_ms\": {build_ms},").unwrap();
    writeln!(json, "  \"passes\": {PASSES},").unwrap();
    writeln!(json, "  \"per_pass_us\": [").unwrap();
    for (p, pass) in per_pass.iter().enumerate() {
        write!(json, "    [").unwrap();
        for (i, (path, us)) in pass.iter().enumerate() {
            let sep = if i == 0 { "" } else { ", " };
            write!(json, "{sep}[\"{path}\", {us}]").unwrap();
        }
        let comma = if p + 1 == PASSES { "" } else { "," };
        writeln!(json, "]{comma}").unwrap();
    }
    writeln!(json, "  ]").unwrap();
    writeln!(json, "}}").unwrap();
    println!("{json}");
    if let Some(p) = out_path {
        std::fs::write(p, &json).expect("write report");
    }
}

fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}
