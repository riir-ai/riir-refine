//! Plan 377 P2.1 — the ane_pregate G2 cost leg + the utility floor
//! (report-only, exit 0 — the `calibrated_dispatch_g2` shape).
//!
//! **Timing half:** the production `suggest_file` path timed per .rs
//! file over the registered walk (this repo's `src/**/*.rs`; the
//! Bench-131 registered-walk precedent), one untimed warmup pass then
//! [`TIMED_PASSES`] timed passes. One process = ONE arm; the runner
//! (`scripts/ane_pregate_g2_run.sh`) interleaves the three arm
//! invocations (A,B,C,A,B,C — 3 rounds) on a quiet box and adjudicates
//! (per-file medians, nearest-rank added p50/p95 vs the pre-registered
//! 1.0/5.0 ms ceilings, tail support disclosed; a MARGINAL pass —
//! < 2× headroom over either ceiling — must be re-run alone before any
//! green, the Bench-131 law).
//!
//! **The arms (the pre-registration's three postures, two builds):**
//! - baseline — default features + `latent_retrieval`, NO `ane`: the
//!   ane domain compiles out (`DomainId::Ane is_compiled ==
//!   cfg!(feature = "ane")`); this is the "added" reference.
//! - ane ungated — the full build with `RIIR_ANE_PREGATE=0` (the
//!   demote env from birth): the ane domain fans out on every span,
//!   byte-identical to a build without the pregate feature (the
//!   `AnePlan::Off` posture).
//! - ane+pregate — the full build, env unset: the first-pass needle
//!   scan gates the ane fan-out to signal-bearing files/spans.
//!
//! **Utility floor (the pre-registration's floor, verbatim):** every
//! `eval_fixtures_ane::ANE_SPANS` fixture span still cites ≥ 1 ane
//! suggestion under the arm's posture (ane-bearing builds; the
//! baseline prints n/a — no ane domain exists there). A floor MISS is
//! a hard line in the doc: the gate may not hide an ANE span the
//! ungated posture cites.
//!
//! ```text
//! cargo run --release --features latent_retrieval \
//!   --example ane_pregate_g2 -- [root]              # baseline
//! RIIR_ANE_PREGATE=0 cargo run --release --features latent_retrieval,ane,ane_pregate \
//!   --example ane_pregate_g2 -- [root]              # ane ungated
//! cargo run --release --features latent_retrieval,ane,ane_pregate \
//!   --example ane_pregate_g2 -- [root]              # ane+pregate
//! ```

use riir_refine::build_stamp::COMPILED_FEATURES;
use riir_refine::draft::eval_fixtures_ane::ANE_SPANS;
use riir_refine::draft::latent_matcher::LatentMatcher;

/// The pre-registered probe width (Bench-125/129/131 verbatim).
const TOP_K: usize = 5;
/// Timed passes per invocation (medians are the runner's; 3 rounds of
/// interleaving happen ACROSS processes — the sequential-A/B class).
const TIMED_PASSES: usize = 3;

fn main() {
    // One optional positional arg: the walk root (default `src`, the
    // registered-walk root).
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("src"));

    // The arm identity, from the build AND the runtime env (the doc's
    // adjudicator keys on this; never infer an arm from file names).
    let compiled_ane = cfg!(feature = "ane");
    let compiled_pregate = cfg!(feature = "ane_pregate");
    let demoted =
        std::env::var("RIIR_ANE_PREGATE").as_deref() == Ok("0");
    let arm = if !compiled_ane {
        "baseline"
    } else if !compiled_pregate || demoted {
        "ane_ungated"
    } else {
        "ane_pregate"
    };

    // The registered walk: recursive .rs under the root, sorted.
    let mut files = Vec::new();
    collect_rs(std::path::Path::new(&root), &mut files);
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!("no .rs files under {root}");
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
        "# ane_pregate_g2 arm={arm} compiled_ane={compiled_ane} compiled_pregate={compiled_pregate} \
         demote_env={} files={} roots={root}",
        demoted,
        srcs.len(),
    );

    let build_started = std::time::Instant::now();
    let mut matcher = LatentMatcher::from_enabled_domains().expect("matcher builds");
    let build_ms = build_started.elapsed().as_millis() as u64;
    println!("# matcher_build_ms={build_ms}");

    // Warmup, untimed (cold caches never skew a median).
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

    // Utility floor: every ANE_SPANS fixture still cites >= 1 ane
    // suggestion under THIS arm's posture (ane-bearing builds only).
    let floor = if compiled_ane {
        let mut misses: Vec<String> = Vec::new();
        for (idx, (src, path)) in ANE_SPANS.iter().enumerate() {
            let spans = matcher.suggest_file(src, path, TOP_K);
            let cited = spans
                .iter()
                .any(|s| s.by_domain.iter().any(|(d, _)| *d == riir_refine::draft::DomainId::Ane));
            if !cited {
                misses.push(format!("fixture#{idx} ({path})"));
            }
        }
        serde_json::json!({
            "fixtures": ANE_SPANS.len(),
            "misses": misses,
            "holds": misses.is_empty(),
        })
    } else {
        serde_json::json!({ "fixtures": ANE_SPANS.len(), "holds": "n/a (no ane domain in this build)" })
    };

    // One JSON document (the runner adjudicates).
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
        "leg": "ane_pregate G2 cost leg + utility floor (Plan 377 P2.1)",
        "compiled_features": COMPILED_FEATURES,
        "arm": arm,
        "compiled_ane": compiled_ane,
        "compiled_pregate": compiled_pregate,
        "demote_env": demoted,
        "root": root,
        "files": srcs.len(),
        "top_k": TOP_K,
        "timed_passes": TIMED_PASSES,
        "matcher_build_ms": build_ms,
        "pass_totals_us": pass_totals,
        "per_file_us": per_file,
        "utility_floor": floor,
    });
    println!("{doc}");
}

fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().as_ref().is_some_and(|e| *e == "rs") {
            out.push(p);
        }
    }
}
