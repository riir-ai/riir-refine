//! Issue 195 — the escalation-chain PLACEBO pick policy (the `JevAct`
//! Table-3 falsification arm).
//!
//! The paper's lesson (arXiv:2610.38620 §5.1): a gate feature grants a
//! revision opportunity, and the opportunity alone can be the whole
//! measured effect — for gpt-oss-120b, RANDOM flags at the gate's rate
//! reproduced the full gate's gain (32.0% → 42.0%). The placebo answers
//! the only question that matters for attribution: does the SEAT'S
//! JUDGMENT contribute anything beyond the escalation opportunity?
//!
//! The placebo is measurement-side, never production: it escalates at
//! the SAME trigger rate as the real arm (the row's worthiness cap +
//! `min_pool`, evaluated on the same inputs) and picks a DETERMINISTIC
//! SEEDED-RANDOM candidate from the SAME pool the seat saw — no
//! judgment, no wire. `real > placebo` (paired LB95 > 0) is the only
//! gain figure attributable to the seat; `real > baseline` is the
//! harness's (seat + revision opportunity together).
//!
//! Posture bound: [`triggered`] replicates the FRESH-PER-CASE dispatch
//! posture's trigger (the `run_frozen` harness arms a new dispatch per
//! case, so the rolling-rate latch's 200-observe window never fills —
//! Bench 119's pre-registered posture). A long-lived dispatch posture
//! additionally carries the sticky cost-ceiling latch; a measurement
//! that runs the placebo against such a posture must mirror the latch
//! explicitly or disclose the mismatch — never silently.
//!
//! Gated at `escalation_manifest` ALONE by design: a placebo consumes
//! the row's trigger constants (cap + `min_pool`), never a transport —
//! which is also what lets the manifest-alone posture compile a real
//! instrument instead of nothing.

/// The deterministic placebo pick index over a presented pool of
/// `pool_len` candidates: `BLAKE3(case_id)`, first 8 bytes as LE u64,
/// modulo the pool length. Same case + same pool size → the same pick,
/// forever (the determinism law — no unseeded-global draws; a re-run is
/// bit-reproducible and the ledger's `pick_idx` is auditable against
/// this function).
///
/// `pool_len` is the PRESENTED length — the caller truncates to
/// [`crate::escalation_dispatch::PRESENTATION_CAP`] first when the pool
/// exceeds the seat's presentation cap, because the placebo must pick
/// from the pool the seat actually saw.
///
/// # Panics
///
/// Panics when `pool_len == 0` — a trigger with an empty pool is a
/// caller contract bug (the dispatch's `min_pool ≥ 2` floor makes it
/// unreachable from the gate path); the placebo refuses loud rather
/// than folding into a silent stand.
#[must_use]
pub fn pick_idx(case_id: &str, pool_len: usize) -> usize {
    assert!(
        pool_len > 0,
        "placebo pick over an empty pool — trigger gate bug"
    );
    let digest = blake3::hash(case_id.as_bytes());
    let seed = u64::from_le_bytes(digest.as_bytes()[..8].try_into().expect("8 bytes"));
    // The modulo bounds the result by pool_len — always a valid usize on
    // every target (pool_len itself came from a usize), so the conversion
    // is infallible by construction.
    usize::try_from(seed % pool_len as u64).unwrap_or(0)
}

/// The placebo's trigger replication at the fresh-per-case posture:
/// the row's worthiness gate (`modelless_score < cap`) AND the
/// `min_pool` floor — the same two conditions
/// [`crate::escalation_dispatch::EscalationDispatch::escalate_rerank`]
/// evaluates before the seat, on the same inputs. The latch is absent
/// by posture (see the module doc).
#[must_use]
pub fn triggered(modelless_score: f32, pool_len: usize, cap: f64, min_pool: usize) -> bool {
    f64::from(modelless_score) < cap && pool_len >= min_pool
}

#[cfg(test)]
mod tests {
    use super::{pick_idx, triggered};

    #[test]
    fn pick_is_deterministic_and_in_range() {
        for pool_len in 1..=12usize {
            for case in ["case-a", "case-b", "case-c", "lint:needless_return:mod7"] {
                let first = pick_idx(case, pool_len);
                for _ in 0..8 {
                    assert_eq!(pick_idx(case, pool_len), first, "{case} x{pool_len}");
                }
                assert!(first < pool_len, "{case} pick {first} ≥ pool {pool_len}");
            }
        }
    }

    #[test]
    fn pick_spreads_across_cases() {
        // 64 synthetic case ids over a pool of 8: determinism is pinned
        // above; this asserts the seed is not degenerate (a constant
        // pick for every id would make the placebo a fixed-arm, not a
        // random one). Not a uniformity claim — a spread bound only.
        let picks: std::collections::BTreeSet<usize> =
            (0..64).map(|i| pick_idx(&format!("case-{i}"), 8)).collect();
        assert!(picks.len() >= 4, "degenerate seed: {picks:?}");
    }

    #[test]
    fn pick_matches_the_documented_formula() {
        // The ledger-auditability contract: an auditor recomputes the
        // pick from the case id with the documented formula.
        for case in ["x", "case-17", "lint:merge_bool:shape"] {
            let seed = u64::from_le_bytes(
                blake3::hash(case.as_bytes()).as_bytes()[..8]
                    .try_into()
                    .unwrap(),
            );
            assert_eq!(pick_idx(case, 5), (seed % 5) as usize);
        }
    }

    #[test]
    #[should_panic(expected = "empty pool")]
    fn empty_pool_refuses_loud() {
        let _ = pick_idx("case", 0);
    }

    #[test]
    fn trigger_is_the_row_gate_and_min_pool() {
        // Worthiness: at/above the cap stands, below is a miss.
        assert!(!triggered(1.0, 4, 1.0, 2));
        assert!(!triggered(0.9, 4, 0.8, 2));
        assert!(triggered(0.7, 4, 0.8, 2));
        // min_pool floor: a miss with a small pool still stands.
        assert!(!triggered(0.7, 1, 0.8, 2));
        assert!(triggered(0.7, 2, 0.8, 2));
        // Cap 1.0 (the Bench-119 bench posture): every sub-1.0 score is a
        // miss. Cap 0.0 is the manifest's "every fix stands" floor (the
        // doc: `0.0` = no case is a miss, nothing triggers).
        assert!(triggered(0.5, 2, 1.0, 2));
        assert!(!triggered(1.0, 2, 1.0, 2));
        assert!(!triggered(0.5, 2, 0.0, 2));
    }
}
