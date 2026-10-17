//! Issue 166 T2 — the ONE bounded expected-null confirmation for the
//! traj-store fused retrieval lane, the (a)-class weighted-sum consumer T1
//! found (`sigmoid_dot_8` fused with max-normalized BM25 before the top-k
//! truncate — `TrajStore::retrieve`).
//!
//! The pre-registered expectation (issue 166, katgpt-rs Research 624 scope
//! law 1): a monotone sigmoid gate at FIXED shape/temperature cannot
//! mediate corpus-growth drift — the gate is hard-coded at the site
//! (`sigmoid(dot)`, no runtime knob, no threshold, no corpus coupling), so
//! any top-k outcome drift across corpus sizes must be pool-tail drift
//! (more rows → more candidates → max-of-M tail), identical in kind under
//! ANY fixed monotone gate. This test runs the shipped `retrieve` at
//! 1×/2×/4× rows, fixed gate, and records the table. It asserts only:
//!
//! - determinism (same-size re-run is byte-identical),
//! - boundedness (every fused pool score's two addends stay in [0, 1]),
//! - the drifting-mass accounting (top-1 recall + k-boundary churn are
//!   REPORTED, never asserted — a null means the numbers show no
//!   gate-mediated effect, which the close-out records verbatim).
//!
//! Whole-file gated (`latent_retrieval`); the `[[test]]` required-features
//! row in Cargo.toml is the reader half of that gate.

#![cfg(feature = "latent_retrieval")]

use std::path::PathBuf;

use riir_refine::traj_store::TrajStore;

const FAMILIES: usize = 8;
const BASE_ROWS_PER_FAMILY: usize = 8; // 1× = 64 rows total
const K: usize = 5;

fn temp_root(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("riir_i166_{tag}_{}", std::process::id()))
}

fn shape_of(family: usize, row: usize) -> String {
    format!("fn example_{family}_{row}(x: u32) -> u32 {{ x.wrapping_add({row}) }}")
}

fn fix_of(family: usize, row: usize) -> String {
    format!("fn example_{family}_{row}(x: u32) -> u32 {{ x.checked_add({row}).unwrap() }}")
}

fn seed(store: &TrajStore, rows_per_family: usize) {
    for family in 0..FAMILIES {
        let lint_key = format!("E{family:04}:s0");
        for row in 0..rows_per_family {
            store
                .record(
                    &lint_key,
                    &shape_of(family, row),
                    &fix_of(family, row),
                    true,
                    false,
                    "i166",
                )
                .expect("record");
        }
    }
}

/// The fixed query set: one query per family, the family's row-0 shape
/// (near-exact) — the top-1 hit lands in the right family iff the fused
/// lane is doing its job at that size.
fn family_query(family: usize) -> String {
    shape_of(family, 0)
}

#[test]
fn issue166_fused_topk_null_across_corpus_sizes() {
    let sizes = [
        ("1x", BASE_ROWS_PER_FAMILY),
        ("2x", BASE_ROWS_PER_FAMILY * 2),
        ("4x", BASE_ROWS_PER_FAMILY * 4),
    ];
    let mut prev_top1: Option<usize> = None;
    let mut prev_pool: Option<Vec<Vec<String>>> = None;

    for (tag, rows_per_family) in sizes {
        let root = temp_root(tag);
        let _ = std::fs::remove_dir_all(&root);
        let (store, bad) = TrajStore::open(&root).expect("open store");
        assert_eq!(bad, 0, "undecodable rows at seed time");
        seed(&store, rows_per_family);

        let mut top1_hits = 0usize;
        let mut pools: Vec<Vec<String>> = Vec::new();
        let mut bound_violations = 0usize;
        for family in 0..FAMILIES {
            let neighbors = store.retrieve(&family_query(family), &format!("E{family:04}:"), K);
            assert!(
                !neighbors.is_empty(),
                "family {family} at {tag}: fused lane returned an empty pool"
            );
            let top1_is_family = neighbors[0].lint_key.starts_with(&format!("E{family:04}:"));
            top1_hits += usize::from(top1_is_family);
            pools.push(neighbors.iter().map(|n| n.lint_key.clone()).collect());
            for n in &neighbors {
                // The fusion's actual inputs are bounded: latent = sigmoid(·)
                // ∈ (0, 1); bm25 enters the comparator max-normalized
                // (bm25/max ≤ 1 — the delivered field is the RAW BM25 score,
                // unbounded by design). Assert the delivered latent + the
                // raw-bm25 non-negativity (the max-norm needs ≥ 0 to be a
                // valid normalization denominator base).
                if !(0.0..=1.0).contains(&n.latent) || n.bm25 < 0.0 {
                    bound_violations += 1;
                }
            }
        }
        assert_eq!(bound_violations, 0, "fused addend escaped [0,1] at {tag}");
        let recall = f64::from(top1_hits as u32) / f64::from(FAMILIES as u32);

        // Determinism: re-run one query at this size, expect the identical pool.
        let again = store.retrieve(&family_query(0), "E0000:", K);
        let again_keys: Vec<String> = again.iter().map(|n| n.lint_key.clone()).collect();
        assert_eq!(again_keys, pools[0], "same-size re-run diverged at {tag}");

        // k-boundary churn vs the previous size (pool-tail evidence, reported).
        if let (Some(prev), Some(prev_top1_n)) = (prev_pool.as_ref(), prev_top1) {
            let churn: usize = pools
                .iter()
                .zip(prev.iter())
                .map(|(cur, old)| cur.iter().filter(|k| !old.contains(k)).count())
                .sum();
            let top1_delta = top1_hits as i64 - prev_top1_n as i64;
            println!(
                "i166 {tag}: rows={} top1={top1_hits}/{} recall={recall:.3} k-boundary-churn={churn} top1-delta-vs-prev={top1_delta:+}",
                rows_per_family * FAMILIES,
                FAMILIES
            );
        } else {
            println!(
                "i166 {tag}: rows={} top1={top1_hits}/{} recall={recall:.3} k-boundary-churn=baseline",
                rows_per_family * FAMILIES,
                FAMILIES
            );
        }

        prev_top1 = Some(top1_hits);
        prev_pool = Some(pools);
        let _ = std::fs::remove_dir_all(&root);
    }
}
