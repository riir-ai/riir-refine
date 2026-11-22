// Batch 401 — the mlx-serve lane-split bar story's `rust_perf` half (Plan
// 401 / Research 434's staged candidate #2, ddalcu/mlx-serve @ 96311b26,
// MIT; quotes re-verified at the pin at this batch's T0).
//
// rust_perf 389 → 390: one measurement-honesty transcription rule:
// - `lane-split-self-consistency-bar-straddle-blind` — a structural split
//   (parallel lanes, chunked conversion jobs) whose pieces cross a
//   converter with an alignment stride passes every self-consistency
//   comparison while the converter corrupts exactly the boundary class the
//   stride discriminates (the source's `convert` left the first rows of
//   every job after one whose rows were not a multiple of 16 at zero).
//   Self-consistency is necessary-not-sufficient: symmetric corruption
//   cancels in any comparison whose both sides inherit it; the bar needs
//   an absolute leg (each piece vs the unsplit source processed alone)
//   over straddling fixture sizes (both sides of the stride: 24 and 8,
//   not 32 and 16).
//
// Fixtures land in src/domains/rust_perf/corpus/tests.rs (the shape arms).
// NOTHING INVENTED — every quote read at the clone this session.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 401 — the mlx-serve lane-split bar story's `rust_perf` half.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b401_rules() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "lane-split-self-consistency-bar-straddle-blind".to_string(),
            name: "A structural split whose pieces cross a converter with an alignment stride is \
                   NOT certified by self-consistency alone: symmetric corruption cancels in any \
                   comparison whose both sides inherit it, so the bar needs an ABSOLUTE leg (each \
                   piece vs the unsplit source processed alone) over STRADDLING fixture sizes — \
                   both sides of the stride, the remainder class included"
                .to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: the split's bar is entirely internal-agreement —
// one lane's output vs two lanes' output, slots interleaved vs
// alone. The converter chunks pieces at a 16-row stride and its
// index mapping zeroes the first rows of every job AFTER one whose
// row count is not a multiple of 16. Both lanes' jobs carry the
// same straddling sizes, so the corruption is identical on every
// side of the split — and every self-consistency comparison passes.
struct SplitBar;

impl SplitBar {
    /// every comparison is configuration-vs-configuration of the SAME split
    fn certified(&self, one_lane: &[i8], two_lane: &[i8]) -> bool {
        one_lane == two_lane // symmetric defect cancels here BY CONSTRUCTION
    }
}

// the boundary class never appears in any fixture: all jobs are
// stride multiples (16/32), so the zeroed-rows shape never runs
const FIXTURE_JOB_ROWS: [usize; 2] = [16, 32];

fn run_converter(jobs: &[usize]) -> Vec<i8> {
    let mut out = Vec::new();
    let mut prev_was_straddler = false;
    for &rows in jobs {
        if prev_was_straddler && rows % 16 != 0 {
            out.extend(std::iter::repeat(0i8).take(rows * 4)); // the defect
        } else {
            out.extend(std::iter::repeat(1i8).take(rows * 4));
        }
        prev_was_straddler = rows % 16 != 0;
    }
    out
}

#[cfg(test)]
mod naive_tests {
    use super::*;

    #[test]
    fn lanes_agree_so_the_split_is_certified() {
        let a = run_converter(&FIXTURE_JOB_ROWS);
        let b = run_converter(&FIXTURE_JOB_ROWS);
        assert!(SplitBar.certified(&a, &b)); // green — and blind
    }
}"#
            .to_string(),
            fixed_code: r#"// FIXED: the bar carries BOTH an absolute leg and straddling
// fixtures. Absolute: each piece's converted output compared
// element-wise against the UNSPLIT source processed alone (the
// source's own oracle — 24+8+8 rows back-to-back vs the whole
// linear converted in one call). Straddling: fixture sizes on
// both sides of the stride — the remainder class (8) and the
// non-multiple (24), not just multiples (16/32). Now the same
// defect reads red at the absolute leg.
struct SplitBar;

impl SplitBar {
    /// config-vs-config agreement is the NECESSARY half …
    fn self_consistent(&self, one_lane: &[i8], two_lane: &[i8]) -> bool {
        one_lane == two_lane
    }

    /// … the SUFFICIENT half is the absolute straddling oracle
    fn certified(&self, pieces: &[i8], unsplit_alone: &[i8]) -> bool {
        pieces == unsplit_alone
    }
}

// both sides of the stride in one job list: non-multiple + remainder
const FIXTURE_JOB_ROWS: [usize; 3] = [24, 8, 8];

fn run_converter(jobs: &[usize]) -> Vec<i8> {
    let mut out = Vec::new();
    let mut prev_was_straddler = false;
    for &rows in jobs {
        if prev_was_straddler && rows % 16 != 0 {
            out.extend(std::iter::repeat(0i8).take(rows * 4)); // the defect, now visible
        } else {
            out.extend(std::iter::repeat(1i8).take(rows * 4));
        }
        prev_was_straddler = rows % 16 != 0;
    }
    out
}

#[cfg(test)]
mod straddle_tests {
    use super::*;

    #[test]
    fn absolute_leg_catches_what_agreement_cannot() {
        // pieces back-to-back …
        let got = run_converter(&FIXTURE_JOB_ROWS);
        // … vs the unsplit source converted alone
        let want = vec![1i8; 40 * 4];
        assert!(!SplitBar.certified(&got, &want)); // red — the defect is seen
    }
}"#
            .to_string(),
            context_notes: Some(r##"Grade B- - NotBenchable-by-domain (the deciding metric is bar STRUCTURE - whether the harness holds an absolute straddling leg - no validator prices that in cycles; a testing-bar gain, not a latency gain). SOURCE, verbatim anchors (at the pin 96311b26): docs/reference.md:456 - "The lane comparison is the bar for a split: two lanes passed every self-consistency test while `convert` left the first rows of every job after one whose rows were not a multiple of 16 at zero (each lane's 8 or 24 `in_proj_b` rows)"; the self-consistency suite that passed (same bullet): "speculative greedy equals plain greedy, a mid-prefill checkpoint restores its position's state, a prompt past the first context decodes, a restored checkpoint continues exactly, two slots interleave like each alone, one lane decodes like two, taps included". THE STRIDE: src/engines/ane/pack.zig:302-304 - "Run every job, split into chunks of 16 rows over `threads` workers." + `const step = 16;` and :316 "The next chunk; a chunk never spans two jobs." THE CATCHING INSTRUMENT (the rule's prescribed absolute leg, in-source): pack.zig:442 test "convert: every row of every job, whatever the jobs' row counts" with the 24/8/8 views "A lane's 24 and 8 rows, as two jobs back to back, against each row converted alone" and `@splat(0)` destinations so unwritten rows read zero - the defect shape exactly. WHY SELF-CONSISTENCY IS STRUCTURALLY BLIND: the comparison (lm.zig:2587 test "pack on the ANE: one lane decodes like two, taps included" - expectEqual(got[1], got[0]) + cosine > 0.999) compares two CONFIGURATIONS OF THE SAME SPLIT; a symmetric defect (both lanes' jobs carry the same straddling sizes) cancels in any such comparison BY CONSTRUCTION - the corrupted model still decoded coherent text and agreed with itself at every position. DECISION CONTENT: (a) self-consistency across a split is NECESSARY-NOT-SUFFICIENT - the doc sentence records what happened, not a bar being met; (b) the bar must add the ABSOLUTE leg (each piece vs the unsplit source processed alone) AND straddling fixtures (piece sizes both sides of the stride: multiples AND the remainder class); (c) the lane share (qwen35.zig:84-86 half() - 32-row in_proj_b halved per lane yields 24- and 8-row jobs, neither a multiple of the 16-row step) is exactly the class the stride discriminates. KINSHIPS (adjudicated): length-keyed-dispatch-cut-invariance (kernel gpu_101) owns the DISPATCH-DESIGN axis (a length boundary selects kernels with different numerics; fix is one-arithmetic-every-length) - THIS owns the TEST-BAR axis (the enemy variable is the harness's comparison set; fix adds the absolute straddling leg; the converter was wrong at every cut, identically) - compose, neither subsumes; verify-parity-position-pure-grid (kernel gpu_152) is the CONSTRUCTION-side law making verify rows bit-COMPARABLE (grid a pure function of position) - comparability is not adjudication: comparable sides can both carry the same corruption - compose; bit-equality-oracle-keyed-on-reference-dispatch (kernel gpu_99) keys the oracle on where the REFERENCE's bits are determinate - THIS keys the bar on the defect class the split's stride discriminates; the GPU cross-lane SIMD lane-* family (~30 hits) is a different "lane" sense entirely (subgroup shuffles vs engine lanes); ane-split-calibration-fit-bandwidth-interference-term (rust_perf entries_74) is the same source family but owns the calibration-fit axis. Regime: any pipeline where split pieces cross a converter, packer, serializer or repack with stride/alignment semantics before consumption (quantized weight conversion, row-chunked encode, column-halved packing). ANTI-REGIME: pure round-trip/idempotence with no alignment invariant in the data path (internal agreement IS the whole claim); zero-copy splits whose pieces are views over the same bytes (a view cannot corrupt); determinism/replay checks (symmetric IS the claim); splits whose every piece size is a stride multiple by construction (no remainder class exists - the absolute leg alone remains). Suggestion-only. Batch 401 / Plan 401 / Research 434 staged candidate (ddalcu/mlx-serve @ `96311b26220f8a71a2a93c1a5418b04729a51ac9`, MIT - quotes re-verified at the pin at this batch's T0; the tip has since moved, the pin is the record). Fixture original."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
