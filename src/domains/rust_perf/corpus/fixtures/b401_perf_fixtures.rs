//! Batch 401 rust_perf fixture preflight — the lane-split bar law
//! (Plan 401 / Research 434 staged candidate #2, ddalcu/mlx-serve @
//! 96311b26 MIT) as ONE standalone file (1 rule, 4 arms).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is Zig: `src/engines/ane/pack.zig`). Compiled AND executed
//! BEFORE entry:
//! `rustc --edition 2021 --test b401_perf_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1a the defect arm: a converter that zeroes the first rows of every
//!   job after a non-stride-multiple job, validated ONLY by config-vs-config
//!   agreement — the naive self-consistency bar passes (the bug class
//!   exists, reproduced).
//! - F1b the fixed arm: the same defect under a bar carrying the absolute
//!   per-piece leg (pieces vs the unsplit source processed alone) — red.
//! - F1c the straddle arm: stride-multiple-only fixtures (16/32) with the
//!   same defect mapped to whole jobs — the straddle-blind fixture misses
//!   it; the straddling fixture (24/8/8 back-to-back, the source's own
//!   24/8 lane shares) catches it.
//! - F1d the clean arm: a correct converter — all bars green, byte-equal
//!   to per-piece-alone conversion (the expectEqualSlices shape).

// ────────────────────────── the model ──────────────────────────

const STRIDE: usize = 16;

/// the defect shape from the source: the first rows of every job AFTER
/// one whose row count is not a multiple of the stride land at zero.
/// (in the real engine the mechanism was a chunk-index mapping bug; here
/// the same observable is produced deterministically.)
fn convert(jobs: &[usize], buggy: bool) -> Vec<i8> {
    let mut out = Vec::new();
    let mut prev_was_straddler = false;
    for &rows in jobs {
        if buggy && prev_was_straddler && rows % STRIDE != 0 {
            out.extend(std::iter::repeat(0i8).take(rows));
        } else {
            out.extend(std::iter::repeat(1i8).take(rows));
        }
        prev_was_straddler = rows % STRIDE != 0;
    }
    out
}

/// the source's straddling lane shares: a 32-row in_proj_b halved per
/// lane yields 24- and 8-row jobs — neither a multiple of the 16-row step.
const LANE_SHARES: [usize; 3] = [24, 8, 8];
/// the straddle-blind fixture: stride multiples only
const MULTIPLES_ONLY: [usize; 2] = [16, 32];

// ────────────────────────── the arms ──────────────────────────

#[test]
fn f1a_self_consistency_passes_over_a_symmetric_defect() {
    // the BUGGY arm: every comparison is configuration-vs-configuration
    // of the SAME split. The defect zeroes the same rows on every side,
    // so the sides agree BY CONSTRUCTION.
    let one_lane = convert(&LANE_SHARES, true);
    let two_lane = convert(&LANE_SHARES, true);
    assert_eq!(
        one_lane, two_lane,
        "self-consistency: symmetric defect cancels"
    );
    // interleaved-slots-like-alone also agrees
    let slots = convert(&LANE_SHARES, true);
    assert_eq!(slots, one_lane);
    // yet the output is corrupted exactly at the boundary class:
    // job 2 (rows 24..32, the first job after the 24-row straddler)
    let job2 = &one_lane[24..32];
    assert!(job2.iter().all(|&v| v == 0), "the boundary class is zeroed");
    // and the harness cannot see it — this is the green-zero shape
    assert_eq!(
        one_lane[0..24],
        vec![1i8; 24],
        "job 1 is fine (the partial zero)"
    );
}

#[test]
fn f1b_absolute_leg_catches_what_agreement_cannot() {
    // the FIXED arm: the same defect, the bar now carries the absolute
    // leg — pieces vs the UNSPLIT source converted alone (40 rows in one
    // call, the source's own oracle shape).
    let pieces = convert(&LANE_SHARES, true);
    let unsplit_alone = convert(&[40], false);
    assert_eq!(unsplit_alone, vec![1i8; 40]);
    assert_ne!(pieces, unsplit_alone, "the absolute leg reads red");
    // and the pieces' own concatenation disagrees element-wise exactly
    // where the defect zeroed (every job AFTER the 24-row straddler: rows
    // 24..40 — the source's defect zeroed "the first rows of every job
    // after one whose rows were not a multiple of 16"), nowhere else
    let diffs: Vec<usize> = (0..40).filter(|&i| pieces[i] != unsplit_alone[i]).collect();
    assert_eq!(diffs, (24..40).collect::<Vec<usize>>());
}

#[test]
fn f1c_straddle_blind_fixtures_miss_the_class_multiples_only() {
    // stride-multiple-only fixtures: no job is ever a straddler, so
    // prev_was_straddler never arms — the defect shape never runs and
    // the naive bar stays green even against the absolute leg.
    let a = convert(&MULTIPLES_ONLY, true);
    let oracle = convert(&[48], false);
    assert_eq!(a, oracle, "multiple-only fixtures cannot excite the defect");
    assert_eq!(a, convert(&MULTIPLES_ONLY, false));

    // the straddling fixture (the source's 24/8/8 back-to-back) DOES:
    let got = convert(&LANE_SHARES, true);
    let want = convert(&[40], false);
    assert_ne!(got, want, "straddling sizes excite the defect");
    // the fixture sizes themselves must straddle: non-multiple + remainder
    assert_eq!(LANE_SHARES[0] % STRIDE, 8, "24: the non-multiple");
    assert_eq!(LANE_SHARES[1] % STRIDE, 8, "8: the remainder class");
    assert_ne!(LANE_SHARES[0] % STRIDE, 0);
}

#[test]
fn f1d_clean_converter_is_byte_equal_everywhere() {
    // the CLEAN arm: a correct converter — every bar green, and the
    // pieces-vs-alone oracle is byte-equal at the straddling sizes too
    // (the expectEqualSlices shape the source's own test pins).
    let pieces = convert(&LANE_SHARES, false);
    let alone = convert(&[40], false);
    assert_eq!(pieces, alone, "correct conversion is split-invariant");
    // self-consistency still holds (necessary half)
    assert_eq!(pieces, convert(&LANE_SHARES, false));
}
