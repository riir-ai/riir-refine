//! Issue 105 T3 — the strand-keep credit-repair gate.
//!
//! The poison (Bench 089/090): a KEEP that ends its run at a typeck-masked
//! E0614 scores as a full verified success, because every selection surface
//! consumed the TERMINAL verdict. The repair (PC-ALM credit, katgpt-rs
//! Research 554): credit = the integral of the residual history plus the
//! terminal residual, never the terminal verdict alone.
//!
//! Three tiers, honestly separated:
//! 1. **Ring replay** (no store) — the three canonical bench populations
//!    fold into JourneyCredit values that rank strand > clean and taint
//!    only the masked-end runs (unit-pinned in driver.rs; asserted here
//!    end-to-end against the ring the CLI would dump).
//! 2. **Store replay** (the mis-attribution metric) — two keys with
//!    IDENTICAL outcome counts (10k/0r) but different journey histories:
//!    all-clean vs strand-tainted. Before 105: `shape_conditional_score`
//!    returns the same value (mis-attribution = 1.0 — indistinguishable).
//!    After: the tainted key scores strictly lower. PASS = strictly lower
//!    mis-attribution at equal counts — the resolved rate is untouched
//!    (the keep rule never changed; parity is pinned by the A/B lane).
//! 3. **Decode stability** — pre-105 rows/rings decode as (0, 0) ⇒ factor
//!    exactly 1.0 ⇒ every legacy store reads bit-identically.

#![cfg(all(feature = "rustc_errors", feature = "latent_retrieval"))]

use riir_clippy::domains::rustc_errors::driver::{
    AttemptOutcome, AttemptRecord, JourneyCredit, JOURNEY_ALPHA, JOURNEY_RHO,
};
use riir_clippy::journey::journey_factor;
use riir_clippy::traj_store::TrajStore;

fn rec(seq: usize, code: &str, strat: &str, outcome: AttemptOutcome, errors: (usize, usize), masking: (usize, usize)) -> AttemptRecord {
    AttemptRecord {
        seq,
        code: code.to_string(),
        span_hash: format!("h{seq}"),
        strategy_id: strat.to_string(),
        outcome,
        errors,
        masking,
        journey_residual_sum: 0,
        terminal_residual: 0,
    }
}

/// Tier 1 — the F4 clean ladder vs the pre-100 F3 strand (the exact rings
/// Benches 089/090 recorded), through the dump-side stamping path.
#[test]
fn ring_replay_strand_ranks_above_clean() {
    let f4 = vec![
        rec(0, "E0614", "e0614-deref-at-use", AttemptOutcome::Kept, (1, 1), (1, 0)),
        rec(1, "E0499", "e0499-swap-second-decl", AttemptOutcome::Kept, (1, 0), (0, 0)),
    ];
    let strand = vec![
        // The pre-100 poison: count-only rule KEPT the clone-out (2→1)
        // while masking went 0→1; the run ends masked, nothing left to
        // propose.
        rec(0, "E0502", "e0502-deref-removal", AttemptOutcome::RevertedNoProgress, (2, 2), (0, 0)),
        rec(1, "E0502", "e0502-clone-out", AttemptOutcome::Kept, (2, 1), (0, 1)),
    ];
    let c_clean = JourneyCredit::from_attempts(&f4);
    let c_strand = JourneyCredit::from_attempts(&strand);
    assert!(!c_clean.tainted(), "F4 ends clean");
    assert!(c_strand.tainted(), "the strand run ends masked");
    let p_clean = c_clean.poverty(JOURNEY_ALPHA, JOURNEY_RHO);
    let p_strand = c_strand.poverty(JOURNEY_ALPHA, JOURNEY_RHO);
    assert!(
        p_strand > p_clean,
        "strand poverty {p_strand} must exceed clean {p_clean}"
    );
    // The composite the store stamps (×256 fixed-point) is only produced
    // for tainted runs and is > 0 for the strand.
    assert!(c_strand.taint_composite_256() > 0);
}

/// Tier 2 — the mis-attribution repair on the REAL store + REAL scorer.
/// One key, three stores (the same key ⇒ the same embedding ⇒ the same
/// retrieval weights — comparable): identical counts (10 successes, 0
/// fails, same shape); the journeys differ — tainted / clean / none.
/// Before 105 the three scores are IDENTICAL (the poison:
/// indistinguishable). After: the tainted store scores strictly lower,
/// the clean-journey store is BIT-IDENTICAL to the no-journey legacy
/// store (factor exactly 1.0), at untouched counts (the keep rule and the
/// outcome counters never changed — resolved-rate parity is pinned by the
/// A/B lane, not by this test).
#[test]
fn store_replay_tainted_key_scores_strictly_lower() {
    let shape = "let head = v[0]; let tail = v[1..];";
    let fix = "let (head, tail) = v.split_first().unwrap();";
    let key = "E0502:e0502-clone-out";
    let strand_composite = {
        let strand = vec![rec(0, "E0502", "e0502-clone-out", AttemptOutcome::Kept, (2, 1), (0, 1))];
        JourneyCredit::from_attempts(&strand).taint_composite_256()
    };
    assert!(strand_composite > 0);

    let score_in_store = |taint: Option<Option<u32>>| {
        let root = std::env::temp_dir().join(format!(
            "riir_105_gate_{}_{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (store, bad) = TrajStore::open(&root).expect("store opens");
        assert_eq!(bad, 0);
        for i in 0..10 {
            let _ = store.record(key, shape, fix, true, true, &format!("run-{i}"));
            if let Some(t) = taint {
                store
                    .record_journey(key, shape, fix, &format!("run-{i}"), t)
                    .expect("journey record");
            }
        }
        let score = riir_clippy::traj_store::shape_conditional_score(&store, "E0502", "e0502-clone-out", shape);
        let _ = std::fs::remove_dir_all(&root);
        score
    };

    let score_tainted = score_in_store(Some(Some(strand_composite)));
    let score_clean = score_in_store(Some(None));
    let score_legacy = score_in_store(None);

    assert!(
        score_clean > score_tainted,
        "clean {score_clean} must strictly exceed tainted {score_tainted} at identical counts"
    );
    assert!(
        score_legacy > score_tainted,
        "legacy {score_legacy} must strictly exceed tainted {score_tainted} — the mis-attribution is repaired"
    );
    assert_eq!(
        score_clean, score_legacy,
        "clean journeys must be BIT-IDENTICAL to no-journey legacy (factor exactly 1.0)"
    );
}

/// Tier 3 — decode stability: a pre-105 TrajRowDoc (no journey fields)
/// deserializes as (0, 0) and scores at factor exactly 1.0.
#[test]
fn pre105_rows_decode_neutral() {
    let legacy = r#"{"lint_key":"E0502:e0502-deref-removal","shape":"s","fix":"f","direction":[0.1,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"success_count":7,"fail_count":2,"consecutive_fails":0,"tier":2,"last_run_id":"r","last_seen_at":1,"elo":1200.0,"span_elo":1200.0}"#;
    let row: riir_clippy::traj_store::TrajRowDoc = serde_json::from_str(legacy).expect("decodes");
    assert_eq!((row.journey_runs, row.journey_taint_sum), (0, 0));
    assert_eq!(journey_factor(row.journey_runs, row.journey_taint_sum), 1.0);
}
