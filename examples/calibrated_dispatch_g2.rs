//! Issue 233 T3 — the calibrated-dispatch G2 cost leg + the real-scan
//! pool-cut value read (report-only, exit 0).
//!
//! **Timing half (the G2 cost leg):** the production `suggest_file` path
//! timed per file over a real `.rs` walk — one untimed warmup pass (cold
//! caches never skew a median), then [`TIMED_PASSES`] timed passes. One
//! process = ONE arm; the runner invokes the example TWICE at feature sets
//! differing ONLY in `calibrated_dispatch` and alternates the invocations
//! (A,B,A,B,A,B — the sequential-A/B posture on a quiet box, `bench_preflight`
//! PROVENANCE quoted by the runner). The adjudication (per-file medians, the
//! armed−incumbent diff p50/p95 vs the pre-registered 1.0/5.0 ms bars) is the
//! runner's; this harness only measures and emits (the `kernel_goat_g5`
//! precedent).
//!
//! **Value half (the real-scan pool cut):** the census measured the fan-out
//! cut on STATIC EVAL FIXTURES, where the annotated rules never co-occur in
//! the pools (0.0% dropped — the landing caveat). This walk replicates the
//! chunk walk + the per-chunk shape verdict + the Perf pool exactly as the
//! suggest path runs them (`AstChunker` → `classify` → `pool_and_ranked`) and
//! reports, per non-ALL Perf chunk: the pool, the incumbent-dispatch retain
//! (`rule_admits`), and — on the armed build — what the calibrated consult
//! actually retains (`narrowing_applies` gates the retain). `drops_shared`
//! counts rules DROPPED (pool − retain; the 2026-11-16 first cell emitted the
//! retained count under this name — fixed, and the verdict arithmetic there
//! read the pool/retain sums, which were correct in every cell);
//! `extra_retained` counts the rules the gate keeps on refused chunks (the
//! incumbent's drops there).
//!
//! Run (the two arms; identical except the one feature):
//! ```text
//! cargo run --release --features span_shape_dispatch,latent_retrieval,clippy_lints,rust_perf \
//!   --example calibrated_dispatch_g2 -- <root> [<root>...]          # incumbent
//! cargo run --release --features calibrated_dispatch,latent_retrieval,clippy_lints,rust_perf \
//!   --example calibrated_dispatch_g2 -- <root> [<root>...]          # armed
//! ```

use riir_refine::domain::SpanShapeSet;
use riir_refine::draft::latent_matcher::span_shape::{classify, rule_admits};
use riir_refine::draft::{DomainId, LatentMatcher};
use riir_refine::ragplane::ast_chunker::AstChunker;
use riir_refine::ragplane::chunker::Chunker;

#[cfg(feature = "calibrated_dispatch")]
use riir_refine::draft::calibrated_dispatch::narrowing_applies;

/// The census's probe width (the dispatch never touches truncation — the
/// retain runs BEFORE marking/rerank/truncate — but the timing path must be
/// the production `suggest_file` call, so the width is the real probe's).
const TOP_K: usize = 8;

/// Timed passes per invocation (medians are the runner's).
const TIMED_PASSES: usize = 3;

#[cfg(feature = "calibrated_dispatch")]
const ARMED: bool = true;
#[cfg(not(feature = "calibrated_dispatch"))]
const ARMED: bool = false;

fn main() {
    let roots: Vec<String> = std::env::args().skip(1).collect();
    if roots.is_empty() {
        eprintln!("usage: calibrated_dispatch_g2 <root> [<root>...]  (recursive *.rs walk)");
        std::process::exit(2);
    }
    let mut files = Vec::new();
    for root in &roots {
        collect_rs(std::path::Path::new(root), &mut files);
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!("no .rs files under the given roots");
        std::process::exit(2);
    }
    let srcs: Vec<(String, String)> = files
        .iter()
        .filter_map(|f| {
            std::fs::read_to_string(f).ok().map(|s| {
                (
                    f.display()
                        .to_string()
                        .replace(std::path::MAIN_SEPARATOR, "/"),
                    s,
                )
            })
        })
        .collect();
    println!(
        "# calibrated_dispatch_g2 armed={ARMED} files={} (of {} walked) roots={roots:?}",
        srcs.len(),
        files.len(),
    );

    let build_started = std::time::Instant::now();
    let mut matcher = LatentMatcher::from_enabled_domains().expect("matcher builds");
    let build_ms = build_started.elapsed().as_millis() as u64;
    println!("# matcher_build_ms={build_ms}");

    // Warmup, untimed.
    for (path, src) in &srcs {
        let _ = matcher.suggest_file(src, path, TOP_K);
    }

    // TIMED_PASSES timed passes; per-file µs, one row per pass.
    let mut per_file_us: Vec<(String, [u64; TIMED_PASSES])> = Vec::with_capacity(srcs.len());
    for (path, src) in &srcs {
        let mut row = [0_u64; TIMED_PASSES];
        for slot in row.iter_mut() {
            let started = std::time::Instant::now();
            let _ = matcher.suggest_file(src, path, TOP_K);
            *slot = started.elapsed().as_micros() as u64;
        }
        per_file_us.push((path.clone(), row));
    }
    let pass_totals: [u64; TIMED_PASSES] =
        std::array::from_fn(|p| per_file_us.iter().map(|(_, row)| row[p]).sum());
    println!("# pass_totals_ms={:?}", pass_totals.map(|t| t / 1000));

    // The real-scan pool-cut value read (untimed; AFTER the timing so the
    // timing passes stay pure production calls).
    let value = value_read(&mut matcher, &srcs);

    // Emit one JSON document (the runner adjudicates).
    let per_file: serde_json::Map<String, serde_json::Value> = per_file_us
        .into_iter()
        .map(|(path, row)| {
            (
                path,
                serde_json::Value::Array(row.iter().map(|us| (*us).into()).collect()),
            )
        })
        .collect();
    let doc = serde_json::json!({
        "leg": "calibrated_dispatch G2 cost leg + real-scan pool cut (Issue 233 T3)",
        "compiled_features": riir_refine::build_stamp::COMPILED_FEATURES,
        "armed": ARMED,
        "roots": roots,
        "files": srcs.len(),
        "top_k": TOP_K,
        "timed_passes": TIMED_PASSES,
        "matcher_build_ms": build_ms,
        "pass_totals_us": pass_totals,
        "per_file_us": per_file,
        "value_read": value,
    });
    println!("{doc}");
}

/// The chunk-walk value read: every non-ALL Perf chunk's pool vs the two
/// postures' retains. Aggregate-only — per-chunk rows would dwarf the doc.
fn value_read(matcher: &mut LatentMatcher, srcs: &[(String, String)]) -> serde_json::Value {
    let chunker = AstChunker;
    let mut chunks_seen: u64 = 0;
    let mut verdict_non_all: u64 = 0;
    #[cfg(feature = "calibrated_dispatch")]
    let (mut armed_narrowed, mut calibration_refused, mut extra_retained): (u64, u64, u64) =
        (0, 0, 0);
    #[cfg(not(feature = "calibrated_dispatch"))]
    let (armed_narrowed, calibration_refused, extra_retained): (u64, u64, u64) = (0, 0, 0);
    let mut pool_sum: u64 = 0;
    let mut incumbent_retain_sum: u64 = 0;
    let mut armed_actual_sum: u64 = 0;
    let mut drops_shared: u64 = 0;

    for (path, src) in srcs {
        for chunk in chunker.chunk(src, path) {
            chunks_seen += 1;
            let parsed = syn::parse_file(&chunk.content).ok();
            let verdict = classify(&chunk.content, parsed.as_ref());
            if verdict == SpanShapeSet::ALL {
                // Both postures leave ALL chunks alone (the fail-open
                // early-out) — counted, never pooled.
                continue;
            }
            verdict_non_all += 1;
            let Some((pool, _ranked)) = matcher.pool_and_ranked(DomainId::Perf, &chunk.content)
            else {
                continue;
            };
            let incumbent_retain = pool
                .iter()
                .filter(|r| rule_admits(&r.rule_id, verdict))
                .count() as u64;
            pool_sum += pool.len() as u64;
            incumbent_retain_sum += incumbent_retain;
            #[cfg(feature = "calibrated_dispatch")]
            {
                if narrowing_applies(verdict) {
                    armed_narrowed += 1;
                    armed_actual_sum += incumbent_retain;
                    drops_shared += pool.len() as u64 - incumbent_retain;
                } else {
                    calibration_refused += 1;
                    armed_actual_sum += pool.len() as u64;
                    extra_retained += pool.len() as u64 - incumbent_retain;
                }
            }
            #[cfg(not(feature = "calibrated_dispatch"))]
            {
                // The incumbent IS the armed retain here.
                drops_shared += pool.len() as u64 - incumbent_retain;
                armed_actual_sum += incumbent_retain;
            }
        }
    }
    serde_json::json!({
        "chunks": chunks_seen,
        "verdict_non_all": verdict_non_all,
        "armed_narrowed": armed_narrowed,
        "calibration_refused": calibration_refused,
        "pool_sum": pool_sum,
        "incumbent_retain_sum": incumbent_retain_sum,
        "armed_actual_sum": armed_actual_sum,
        "drops_shared": drops_shared,
        "extra_retained": extra_retained,
    })
}

/// Recursive `*.rs` walk; build-output and VCS dirs skipped.
fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | ".raw" | "node_modules") {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
