//! Issue 072 T2 — the non-circularity fence, executed against the REAL
//! oracle (rustc). The always-on unit gates pin the fence ARITHMETIC
//! (`fence_verdicts`/`FenceReport`); this e2e proves the CONSTRUCTION
//! PREMISE on a deterministic sample: the good arm compiles clean, the bad
//! premise holds (the case is a valid resolution instance).
//!
//! A buggy generator poisons every pair it emits (the Issue-022
//! corpus-drift class) — the fence is the mandatory gate BEFORE pair
//! ingestion feeds ratings, and ≥95% agreement is the pass bar.
//!
//! `#[ignore]`d: spawns rustc ~2×K times (~40 × 0.1–0.3 s). Run explicitly:
//!
//! ```bash
//! cargo test --release --features rustc_errors --test frontier_fence_e2e -- --ignored --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use std::path::Path;
use std::process::Command;

use riir_refine::frontier_pairs::{PairVerdict, construction_pairs, fence_sample, fence_verdicts};

/// Compile one source with plain rustc (the `behavioural_equivalence`
/// house pattern — no cargo, no dep graph, no shared target dir) and
/// report whether the named error code appears in the diagnostics.
fn compile_has_code(source: &str, workdir: &Path, expect_code: &str) -> Result<bool, String> {
    std::fs::create_dir_all(workdir).map_err(|e| format!("mkdir {}: {e}", workdir.display()))?;
    let src = workdir.join("probe_case.rs");
    std::fs::write(&src, source).map_err(|e| format!("write {}: {e}", src.display()))?;
    let out = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "--out-dir",
        ])
        .arg(workdir)
        .arg(&src)
        .output()
        .map_err(|e| format!("rustc not runnable: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_file(&src);
    Ok(stderr.contains(expect_code))
}

#[test]
#[ignore = "spawns rustc ~40x (0.1-0.3 s each); run with --ignored — the T2 fence e2e"]
fn fence_sample_agrees_with_real_rustc_at_95pct() -> Result<(), Box<dyn std::error::Error>> {
    if Command::new("rustc").arg("--version").output().is_err() {
        panic!("rustc not on PATH — the fence e2e cannot run where it is asked to run");
    }
    let workdir = std::env::temp_dir().join(format!("riir_fence_{}", std::process::id()));
    let pairs = construction_pairs();
    let sample = fence_sample(&pairs, 20);
    let mut verdicts: Vec<(&str, PairVerdict)> = Vec::with_capacity(sample.len());
    for p in &sample {
        // good arm: the case's own seed must compile WITHOUT expect_code.
        let good_clean = !compile_has_code(&p.good_fix, &workdir, p.code)?;
        // bad premise: the BROKEN case must still fail WITH expect_code.
        let broken = {
            // The pair row carries only the seeds; the broken form comes
            // from the corpus case the pair anchors (regenerate + match).
            let cases = riir_refine::domains::rustc_errors::corpus_gen::generate_corpus();
            cases
                .iter()
                .find(|c| c.name == p.case_name)
                .map(|c| c.broken.clone())
                .ok_or_else(|| format!("case {} vanished from the corpus", p.case_name))?
        };
        let bad_premise = compile_has_code(&broken, &workdir, p.code)?;
        verdicts.push((
            Box::leak(p.case_name.clone().into_boxed_str()),
            PairVerdict {
                good_resolves: good_clean,
                bad_premise_holds: bad_premise,
            },
        ));
    }
    let report = fence_verdicts(&verdicts);
    println!(
        "fence: {}/{} agree ({:.1}%) — disagreements: {:?}",
        report.agreements,
        report.pairs_checked,
        report.rate() * 100.0,
        report.disagreements
    );
    let _ = std::fs::remove_dir_all(&workdir);
    assert!(
        report.passes(0.95),
        "construction fence FAILED at {:.1}% (<95%): the generator disagrees with the real oracle on {:?}",
        report.rate() * 100.0,
        report.disagreements
    );
    Ok(())
}
