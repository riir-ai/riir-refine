//! Issue 126 lane (a) — the standing gate for the trained two-tower posture
//! (`rule_embed_poc`): the committed artifact, at the pinned weight, must
//! hold the measured gates. Count-pinned at the 2026-09-20 landing
//! (`.benchmarks/099_rule_embed_sigmoid_nce.md`):
//!
//! - **G2 (floor):** full-rig trained hits-at-1 ≥ 98 = train 67 + withheld
//!   31 (baseline 94 + the 4 rescued misses: `useless_conversion`,
//!   `single_char_pattern`, `comparison_to_empty`, `box-str-immutable`
//!   outranked; `needless_borrow` rescued from not-retrieved to rank 6).
//!   Fires downward on a corpus/trainer regression.
//! - **G2-OOD (floor):** withheld-split trained hits-at-1 = 31 (the baseline
//!   is already perfect there — zero headroom; the split is the
//!   anti-memorization read, no harm off-distribution).
//! - **G1 (wall at 0):** zero top-1 regressions vs baseline.
//! - **G3 (floor):** clean-span fixable top-1 ≤ 26 (baseline's own count —
//!   the FP class must not grow).
//!
//! The BASELINE counts are pinned too (94/31/26): a baseline shift re-pins
//! the trained floors beside it — a silent re-pin is the failure shape the
//! count law exists for.
//!
//! Weight: w=16, pinned by the example's train-split sweep (the argmax;
//! ties → smaller). Runtime ~5-10 s (two matcher builds + 101 spans × 2
//! postures + 12 clean spans × 2).
#![cfg(feature = "rule_embed_poc")]

use riir_clippy::draft::retrieval_eval::run_split_eval;
use riir_clippy::draft::rule_embed::embedded_tables;

const PINNED_WEIGHT: f32 = 16.0;

// Baseline floors (the untrained posture, same run — re-pin on corpus drift).
const BASELINE_TRAIN_HITS: usize = 63;
const BASELINE_WITHHELD_HITS: usize = 31;
const BASELINE_CLEAN_FIXABLE: usize = 26;

// Trained floors at PINNED_WEIGHT (the landing's measured values; the G2
// win floor is their SUM — 67 + 31 = 98 vs baseline 63 + 31 = 94).
const TRAINED_TRAIN_HITS: usize = 67;
const TRAINED_WITHHELD_HITS: usize = 31;
const TRAINED_CLEAN_FIXABLE: usize = 26;

#[test]
fn rule_embed_posture_holds_the_pinned_gates() {
    let tables = std::sync::Arc::new(embedded_tables().clone());
    let rep = run_split_eval(&tables, PINNED_WEIGHT);

    // G1 — wall at 0.
    assert_eq!(
        rep.regressions_vs_baseline, 0,
        "trained posture lost baseline wins: {}",
        rep.regressions_vs_baseline
    );

    // Baseline pins — a drift here is a corpus/retrieval change, and the
    // trained floors BELOW are then stale until re-pinned beside it.
    assert!(
        rep.baseline_train.hits_at_1 >= BASELINE_TRAIN_HITS,
        "baseline train hits {} < floor {BASELINE_TRAIN_HITS}",
        rep.baseline_train.hits_at_1
    );
    assert!(
        rep.baseline_withheld.hits_at_1 >= BASELINE_WITHHELD_HITS,
        "baseline withheld hits {} < floor {BASELINE_WITHHELD_HITS}",
        rep.baseline_withheld.hits_at_1
    );

    // G2 — the win floor (fires downward on trainer/corpus regression).
    assert!(
        rep.trained_train.hits_at_1 >= TRAINED_TRAIN_HITS,
        "trained train hits {} < floor {TRAINED_TRAIN_HITS}",
        rep.trained_train.hits_at_1
    );
    assert!(
        rep.trained_withheld.hits_at_1 >= TRAINED_WITHHELD_HITS,
        "trained withheld hits {} < floor {TRAINED_WITHHELD_HITS} (off-distribution harm)",
        rep.trained_withheld.hits_at_1
    );
    let trained_total = rep.trained_train.hits_at_1 + rep.trained_withheld.hits_at_1;
    let baseline_total = rep.baseline_train.hits_at_1 + rep.baseline_withheld.hits_at_1;
    assert!(
        trained_total > baseline_total,
        "G2 win gate lost: trained {trained_total} vs baseline {baseline_total}"
    );

    // G3 — the clean FP class must not grow.
    assert!(
        rep.clean_top1_fixable_trained <= TRAINED_CLEAN_FIXABLE,
        "clean fixable top-1 {} > pinned {TRAINED_CLEAN_FIXABLE} (false-positive growth)",
        rep.clean_top1_fixable_trained
    );
    assert!(
        rep.clean_top1_fixable_baseline >= BASELINE_CLEAN_FIXABLE,
        "baseline clean fixable top-1 {} < floor {BASELINE_CLEAN_FIXABLE}",
        rep.clean_top1_fixable_baseline
    );

    // The artifact must cover the eval's rule population honestly — report
    // the coverage so a future corpus growth that outruns the table is loud
    // in the test output (not a failure: uncovered rules fall back to the
    // baseline term by design).
    let total_rules = rep.baseline_train.labeled + rep.baseline_withheld.labeled;
    assert_eq!(total_rules, 101, "labeled span population shifted");
}
