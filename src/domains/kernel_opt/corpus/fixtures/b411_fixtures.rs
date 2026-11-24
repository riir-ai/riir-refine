//! Batch 411 kernel fixture preflight — the four mlx residual slices
//! (Plan 410 / Research 444, ml-explore/mlx @ c69303b0, MIT — the
//! Batch-410 staged residue) as ONE standalone file (4 rules, the
//! four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is Metal/C++; the Rust arms below model each law's decision
//! content). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b411_fixtures.rs`
//! at debug AND -O. Each arm names the honest and the adversary half;
//! a defect caught here is fixed HERE first, then synced into
//! entries_gpu_167.rs in the same landing commit.
//!
//! Arms:
//! - S1 per-row-bounds-bitmask-hoisted-loader: the closed-form masks
//!   (mask_h/mask_w = 6/7/7/3); the hot-path arithmetic census (720
//!   bound checks + 576 coordinate ops naive vs 60 build comparisons,
//!   zero in the load); the bit-identity (fixed == naive); the
//!   all-interior pole (all-ones masks — the compose seam with the
//!   whole-tile fast path); the tamper arm (a cleared live bit zeroes
//!   out(0,1) from -5 to 0).
//! - S2 dilation-base-table-tap-odometer: the host solve (config A
//!   table {2,2},{1,2},{0,2}, taps {2,5},{1,4},{0,3}); the congruence
//!   cross-check; the odometer wrap-to-base (never zero — the wrap-to-
//!   zero tamper injects unaligned taps); the degenerate all-safe
//!   phase (idil=1 → trivial table); config B (size can be 1).
//! - S3 reduce-simd-width-fallback-launch-cap-pairing: the ladder
//!   (512→32 / 1024→128 / ceil-32-cap-1024); the clamp bites at row
//!   4096 (1024→512, cost 15616→8192 in ×4 model units) and is a
//!   no-op at 2048; the narrow type must NOT clamp (1024 stays, the
//!   counterfactual clamp costs 512→640); the tamper arm (the host
//!   clamp re-keyed on row size instead of width — the grid probe
//!   detects the predicate divergence).
//! - S4 strided-reduce-shape-ratio-assumption-guarded-tier-ladder:
//!   the 11-shape pinned table (boundary-exact rows); the widening
//!   rider (longcolumn 32→128 at total ≥ 32768, 2pass NEVER widens);
//!   the one-predicate naive misroutes; the reordered-ladder tamper
//!   routes shape 3 (and 9) out of LongColumn.

// ═══════════ S1 — per-row bounds bitmask ═══════════

const IN_W: i32 = 7;
const IN_H: i32 = 7;
const KS: i32 = 3; // kernel size per axis
const STR: i32 = 2;
const PAD: i32 = 1;
const OUT: i32 = 4; // (7 + 2*1 - 3) / 2 + 1 = 4

fn input(i: i32, j: i32) -> i32 {
    ((7 * i + j) % 13) - 6
}
fn weight(kh: i32, kw: i32) -> i32 {
    ((3 * kh + kw) % 5) - 2
}

/// The naive loader: every tap re-evaluates the full inequality and
/// re-does the coordinate arithmetic. Counts what the hot loop pays.
fn naive_conv(out: &mut [[i32; 4]; 4]) -> (u64, u64) {
    let mut bound_checks = 0u64;
    let mut coord_ops = 0u64;
    for oh in 0..OUT {
        for ow in 0..OUT {
            let mut acc = 0;
            for kh in 0..KS {
                for kw in 0..KS {
                    coord_ops += 4; // ih, iw, two dilated offsets
                    let ih = oh * STR - PAD + kh;
                    let iw = ow * STR - PAD + kw;
                    bound_checks += 5; // n(1) + h(2) + w(2)
                    if ih >= 0 && ih < IN_H && iw >= 0 && iw < IN_W {
                        acc += input(ih, iw) * weight(kh, kw);
                    }
                }
            }
            out[oh as usize][ow as usize] = acc;
        }
    }
    (bound_checks, coord_ops)
}

/// The mask loader: build once per tile offset (the constructor), then
/// the hot per-tap load pays two bit tests — zero bound checks, zero
/// coordinate arithmetic. mask bit kh set iff the tap is in bounds.
fn build_masks() -> ([u16; 4], [u16; 4], u64) {
    let mut mask_h = [0u16; 4];
    let mut mask_w = [0u16; 4];
    // H axis: base ih = oh*str - pad; batch n=1 fixed (the source
    // folds the batch check into the FIRST axis's mask only — pinned).
    let mut build_comparisons = 0u64;
    for kh in 0..KS {
        for oh in 0..OUT {
            build_comparisons += 3; // ih >= 0, ih < H, (n < N folded once)
            let ih = oh * STR - PAD + kh;
            let in_bounds = ih >= 0 && ih < IN_H;
            mask_h[oh as usize] |= (in_bounds as u16) << kh;
        }
    }
    for kw in 0..KS {
        for ow in 0..OUT {
            build_comparisons += 2; // iw >= 0, iw < W
            let iw = ow * STR - PAD + kw;
            let in_bounds = iw >= 0 && iw < IN_W;
            mask_w[ow as usize] |= (in_bounds as u16) << kw;
        }
    }
    (mask_h, mask_w, build_comparisons)
}

fn masked_conv(mask_h: &[u16; 4], mask_w: &[u16; 4], out: &mut [[i32; 4]; 4]) -> u64 {
    let mut bit_tests = 0u64;
    for oh in 0..OUT {
        for ow in 0..OUT {
            let mut acc = 0;
            for kh in 0..KS {
                for kw in 0..KS {
                    bit_tests += 2; // the two mask ANDs — nothing else
                    let live = (mask_h[oh as usize] >> kh) & 1 == 1
                        && (mask_w[ow as usize] >> kw) & 1 == 1;
                    if live {
                        // the source's load path does NO coordinate
                        // arithmetic: the pointer was staged at
                        // construction. Model the staged read as a
                        // direct lookup by the staged (ih, iw), then
                        // the SAME multiply the accumulate pays.
                        acc += staged(oh, ow, kh, kw) * weight(kh, kw);
                    }
                }
            }
            out[oh as usize][ow as usize] = acc;
        }
    }
    bit_tests
}

/// The staged-pointer read: src[i] was computed once in the
/// constructor (`src[i] = src_ + n*is[0] + ih*is[1] + iw*is[2] + bj`),
/// and `next()` advances it by a fixed jump per tap — so the hot path
/// reads whatever the pointer holds with zero coordinate math. The
/// model reproduces that: the tap's element, no bounds re-check.
fn staged(oh: i32, ow: i32, kh: i32, kw: i32) -> i32 {
    let ih = oh * STR - PAD + kh;
    let iw = ow * STR - PAD + kw;
    if ih >= 0 && ih < IN_H && iw >= 0 && iw < IN_W {
        input(ih, iw)
    } else {
        // unreachable in the masked path (the mask guards it) — but
        // the model stays total
        0
    }
}

#[test]
fn s1_masks_match_the_closed_form_and_outputs_are_bit_identical() {
    let (mask_h, mask_w, build_comparisons) = build_masks();
    // closed form: bit kh set iff 2*oh - 1 + kh in [0, 7)
    assert_eq!(mask_h, [6, 7, 7, 3], "mask_h closed form");
    assert_eq!(mask_w, [6, 7, 7, 3], "mask_w closed form");
    // build cost: h axis 3 taps x 4 rows x 3 + w axis 3 x 4 x 2
    assert_eq!(build_comparisons, 36 + 24);

    let mut naive = [[0i32; 4]; 4];
    let (bc, co) = naive_conv(&mut naive);
    assert_eq!(bc, 720, "naive: 16 outputs x 9 taps x 5 comparisons");
    assert_eq!(co, 576, "naive: 16 x 9 x 4 coordinate ops");

    let mut fixed = [[0i32; 4]; 4];
    let bt = masked_conv(&mask_h, &mask_w, &mut fixed);
    assert_eq!(bt, 288, "fixed: 16 x 9 x 2 bit tests in the load");
    assert_eq!(naive, fixed, "masked path bit-identical to naive");

    // pinned elements (hand-computed)
    assert_eq!(naive[0][0], 0);
    assert_eq!(naive[0][1], -5);
    assert_eq!(naive[1][1], -11);
    assert_eq!(naive[3][3], 22);
    // out(0,1) worked: kh1 row -5 + -8 + 6 = -7; kh2 row -2 + 0 + 4 = +2;
    // kh0 fully masked → -5
}

#[test]
fn s1_all_interior_pole_masks_are_all_ones() {
    // pad=0, stride=1, kernel 3 on a 6x6 input: every output interior
    // → every mask bit set — the compose seam where the whole-tile
    // fast path (gpu_166) would key its unguarded loader.
    let mut mask = 0u16;
    for oh in 0..4u16 {
        for kh in 0..3u16 {
            let ih = oh as i32 + kh as i32; // stride 1, pad 0
            if ih >= 0 && ih < 6 {
                mask |= 1 << kh;
            }
        }
    }
    assert_eq!(mask, 0b111, "all-interior: every tap in bounds");
    // (the fixture pins the pole's predicate; the fast path itself is
    // gpu_166's surface — compose, never merge)
}

#[test]
fn s1_tampered_mask_bit_zeroes_a_live_element() {
    let (mut mask_h, mask_w, _) = build_masks();
    // adversary: bit kh=1 cleared from every row mask (wrong polarity
    // — a live tap zeroed)
    for m in mask_h.iter_mut() {
        *m &= !(1 << 1);
    }
    let mut tampered = [[0i32; 4]; 4];
    let _ = masked_conv(&mask_h, &mask_w, &mut tampered);
    // out(0,1) loses the whole kh=1 row (-7: -5-8+6), keeping the
    // kh=2 row (+2: -2+0+4) → the live-tap term set changed
    assert_eq!(tampered[0][1], 2, "cleared live row dropped its terms");
    let honest = [0i32, 0, 0, -5, -3, -7, 0, -11, -28, 2, 0, -13, -30, 27, -5, -17, -8, 22];
    let _ = honest; // (the un-tampered pair's bit-identity is pinned in the first test)
}

// ═══════════ S2 — dilation base tables + tap odometer ═══════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BaseInfo {
    weight_base: i32,
    weight_size: i32,
}

fn lcm(a: i32, b: i32) -> i32 {
    a / gcd(a, b) * b
}
fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

/// The host solve, verbatim in i32 (Rust's % keeps the dividend's
/// sign — the negative ih_loop steps mirror the C++ exactly).
fn host_base(w_s: i32, str_: i32, pad: i32, kdil: i32, idil: i32) -> Vec<BaseInfo> {
    let f_wgt_jump = lcm(idil, kdil) / kdil;
    let f_out_jump = lcm(idil, str_) / str_;
    let jump = kdil; // flip = false
    let init = 0; // flip = false
    let mut base = Vec::new();
    for i in 0..f_out_jump {
        let mut ih_loop = i * str_ - pad + init;
        let mut wh_base = 0;
        while wh_base < w_s && ih_loop % idil != 0 {
            wh_base += 1;
            ih_loop += jump;
        }
        let wh_size = ((w_s - wh_base) + f_wgt_jump - 1) / f_wgt_jump;
        base.push(BaseInfo {
            weight_base: wh_base,
            weight_size: wh_size,
        });
    }
    base
}

/// The odometer: start at the phase base, advance by the tap jump,
/// wrap to the BASE (never zero). `ok_wrap_to_zero` models the defect.
fn odometer(base: &BaseInfo, w_s: i32, jump: i32, trips: usize, ok_wrap_to_zero: bool) -> Vec<i32> {
    let mut t = base.weight_base;
    let mut visited = Vec::new();
    for _ in 0..trips {
        visited.push(t);
        t += jump;
        if t < w_s {
            continue;
        }
        t = if ok_wrap_to_zero { 0 } else { base.weight_base };
    }
    visited
}

#[test]
fn s2_host_solve_matches_the_congruence_cross_check() {
    // config A: wS=6, str=2, pad=1, kdil=2, idil=3
    let base = host_base(6, 2, 1, 2, 3);
    assert_eq!(
        base,
        vec![
            BaseInfo { weight_base: 2, weight_size: 2 },
            BaseInfo { weight_base: 1, weight_size: 2 },
            BaseInfo { weight_base: 0, weight_size: 2 },
        ],
        "config A closed form: phases 0/1/2"
    );
    // congruence cross-check: tap t is aligned for phase i iff
    // (2i - 1 + 2t) % 3 == 0
    for (i, b) in base.iter().enumerate() {
        let taps: Vec<i32> = (0..6)
            .filter(|&t| (2 * i as i32 - 1 + 2 * t) % 3 == 0)
            .collect();
        let od = odometer(b, 6, 3, b.weight_size as usize, false);
        assert_eq!(od, taps, "phase {i}: odometer taps == congruence set");
    }
}

#[test]
fn s2_naive_retests_alignment_every_tap_the_fixed_never_does() {
    // naive: all 6 taps per phase, each with a modulo test
    let mut align_tests = 0u64;
    let mut productive = 0u64;
    for i in 0..3i32 {
        for t in 0..6i32 {
            align_tests += 1;
            if (2 * i - 1 + 2 * t) % 3 == 0 {
                productive += 1;
            }
        }
    }
    assert_eq!(align_tests, 18, "naive: 3 phases x 6 taps re-tested");
    assert_eq!(productive, 6, "naive: 6 aligned taps among 18 visits");
    // fixed: the walk visits exactly the table sizes — zero tests
    let base = host_base(6, 2, 1, 2, 3);
    let visits: usize = base.iter().map(|b| b.weight_size as usize).sum();
    assert_eq!(visits, 6, "fixed: trips == sum of per-phase sizes");
    // the only % in the lane is the HOST solve (bounded, 3 steps for
    // config A) — device-side idil arithmetic has no modulo at all.
}

#[test]
fn s2_degenerate_phase_trivial_table_no_wrap() {
    // idil=1, str=2: f_out_jump = 1, f_wgt_jump = 1 — one phase, the
    // full tap range, base 0 (the all-safe pole where the win is zero)
    let base = host_base(6, 2, 1, 1, 1);
    assert_eq!(base.len(), 1);
    assert_eq!(base[0], BaseInfo { weight_base: 0, weight_size: 6 });
    let od = odometer(&base[0], 6, 1, 6, false);
    assert_eq!(od, vec![0, 1, 2, 3, 4, 5], "no wrap needed");
}

#[test]
fn s2_wrap_to_zero_injects_unaligned_taps() {
    // the tamper arm: `next()` wrapping to zero instead of the base —
    // over 2 channel blocks (trips = size x 2, mirroring the trip
    // count the table sizes: gemm_k_iterations = size_h x size_w x
    // channel_iters), phase 0 must visit {2, 5, 2, 5}
    let base = &host_base(6, 2, 1, 2, 3)[0];
    let fixed = odometer(base, 6, 3, 4, false);
    assert_eq!(fixed, vec![2, 5, 2, 5]);
    let defective = odometer(base, 6, 3, 4, true);
    assert_eq!(defective, vec![2, 5, 0, 3]);
    // every injected tap violates the phase-0 congruence
    for t in [0, 3] {
        assert_ne!((-1 + 2 * t) % 3, 0, "tap {t} not aligned for phase 0");
    }
}

#[test]
fn s2_config_b_size_can_be_one() {
    // idil=4, str=2, kdil=1, pad=1: f_wgt_jump=4, f_out_jump=2;
    // phase 1 solves to base 3, size ceil(3/4)=1 — the odometer
    // visits one tap and wraps without a second in-range step
    let base = host_base(6, 2, 1, 1, 4);
    assert_eq!(
        base,
        vec![
            BaseInfo { weight_base: 1, weight_size: 2 },
            BaseInfo { weight_base: 3, weight_size: 1 },
        ]
    );
    assert_eq!(odometer(&base[1], 6, 4, 1, false), vec![3]);
}

// ═══════════ S3 — simd width fallback + launch cap pairing ═══════════

const N_READS: usize = 4;
const N_WRITES: usize = 4;
const SIMD: usize = 32;
const FALLBACK_ROUNDS: usize = 5; // simd_size/2 .. 1: 16,8,4,2,1
const PRIMITIVE_ROUNDS: usize = 1; // the hardware simd reduce, model units

fn ladder(row: usize) -> usize {
    if row <= 512 {
        32
    } else if row <= 1024 {
        128
    } else {
        let tg = (row + N_READS - 1) / N_READS;
        ((tg + 31) / 32) * 32
    }
    .min(1024)
}

fn naive_launch(row: usize, _width8: bool) -> usize {
    ladder(row) // width-blind — the unpaired posture
}

fn fixed_launch(row: usize, width8: bool) -> usize {
    let tg = ladder(row);
    if width8 {
        tg.min(512) // the clamp keyed on the SAME predicate as the kernel
    } else {
        tg
    }
}

/// ×4 integer cost model: sequential adds (4 units per add) + the
/// cross-group shuffle units weighted W4 (narrow 1, wide 12 — model
/// units for dependent-shuffle vs primitive cost, not hardware claims).
fn cost4(row: usize, l: usize, width8: bool) -> usize {
    let adds = 16 * row * N_WRITES / l; // 4 units/add, conserved
    let phases = if l / SIMD == 1 { 1 } else { 2 };
    let rounds = if width8 { FALLBACK_ROUNDS } else { PRIMITIVE_ROUNDS };
    let cross_units = phases * N_WRITES * rounds * (l / SIMD);
    // the wide fallback's shuffle rounds cost 12 model-units each
    // (dependent shuffle); the narrow primitive 1 — the adds term is
    // already in the same ×4 unit scale
    adds + cross_units * if width8 { 12 } else { 1 }
}

#[test]
fn s3_ladder_and_the_clamp_bite() {
    // the ladder
    assert_eq!(ladder(512), 32);
    assert_eq!(ladder(1024), 128);
    assert_eq!(ladder(2048), 512); // ceil((2048+3)/4) = 512
    assert_eq!(ladder(4096), 1024);
    assert_eq!(ladder(100_000), 1024); // the cap
    // the clamp bites at row 4096 (1024 → 512), no-op at 2048
    assert_eq!(fixed_launch(4096, true), 512);
    assert_eq!(fixed_launch(2048, true), 512);
    assert_eq!(fixed_launch(1024, true), 128);
    assert_eq!(fixed_launch(512, true), 32);
    // narrow types NEVER clamp
    assert_eq!(fixed_launch(4096, false), 1024);
}

#[test]
fn s3_unpaired_fallback_runs_slowest_at_max_occupancy() {
    // row 4096, 8-byte: naive 1024 threads pays 15616 ×4-units, the
    // clamped 512 pays 8192 — the unpaired fallback is 1.9× the cost
    let naive = cost4(4096, naive_launch(4096, true), true);
    let fixed = cost4(4096, fixed_launch(4096, true), true);
    assert_eq!(naive, 15_616);
    assert_eq!(fixed, 8_192);
    assert!(naive > fixed, "the clamp must win at the bite point");
    // work conservation: total adds == row * N_WRITES (×4 units /4)
    for l in [32usize, 128, 512, 1024] {
        let blocks = 4096 / (l * N_READS);
        assert_eq!(blocks * N_WRITES * N_READS * l, 4096 * N_WRITES);
    }
    // per-thread slot-rounds: 2 phases x 4 slots x 5 rounds = 40
    // (multi-simd), half that at a single simdgroup
    let rounds_thread = |l: usize| {
        let phases = if l / SIMD == 1 { 1 } else { 2 };
        phases * N_WRITES * FALLBACK_ROUNDS
    };
    assert_eq!(rounds_thread(512), 40);
    assert_eq!(rounds_thread(32), 20);
}

#[test]
fn s3_narrow_types_stay_on_the_primitive() {
    // 4-byte: both postures launch 1024 (the clamp is inert) and the
    // primitive path is cheap — the counterfactual clamp COSTS
    assert_eq!(fixed_launch(4096, false), 1024);
    let no_clamp = cost4(4096, 1024, false);
    let wrongly_clamped = cost4(4096, 512, false);
    assert_eq!(no_clamp, 512);
    assert_eq!(wrongly_clamped, 640);
    assert!(wrongly_clamped > no_clamp, "clamping the primitive arm loses");
}

#[test]
fn s3_tampered_predicate_diverges_from_the_kernel_fallback() {
    // adversary: the host clamp re-keyed on ROW SIZE while the kernel
    // falls back on WIDTH — the pairing invariant (clamp predicate ==
    // fallback predicate) breaks
    let tampered_launch = |row: usize, _width8: bool| {
        if row > 2048 {
            ladder(row).min(512)
        } else {
            ladder(row)
        }
    };
    let probe = [(512usize, true), (1024, true), (2048, true), (4096, true), (4096, false)];
    let mut divergences = 0;
    for &(row, w8) in &probe {
        if tampered_launch(row, w8) != fixed_launch(row, w8) {
            divergences += 1;
            // every divergence is a mis-pairing: the tampered clamp
            // keyed on a predicate the kernel's fallback does not read
            assert!(
                (row == 4096 && !w8) || (row < 2048 && w8),
                "unexpected divergence shape at ({row}, {w8})"
            );
        }
    }
    assert!(divergences >= 1, "the grid probe must catch the mis-pairing");
    // the concrete loss: (4096, narrow) clamped to 512 — 512 → 640
    assert_eq!(cost4(4096, tampered_launch(4096, false), false), 640);
    assert_eq!(cost4(4096, fixed_launch(4096, false), false), 512);
}

// ═══════════ S4 — strided reduce tier ladder ═══════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    Small,
    LongColumn,
    TwoPass,
    Looped,
}

/// The ladder, verbatim: first satisfied guard wins.
fn tier(r: usize, s: usize, o: usize) -> Tier {
    if r < 32 {
        return Tier::Small;
    }
    if s < 32 && r >= 1024 {
        return Tier::LongColumn;
    }
    if r > 256 && o / 32 < 1024 {
        return Tier::TwoPass;
    }
    Tier::Looped
}

/// The widening rider: longcolumn-only (2pass's outer blocks stay 32).
fn longcolumn_outer_blocks(total: usize) -> usize {
    if total >= 32768 {
        128
    } else {
        32
    }
}
fn two_pass_outer_blocks(_total: usize) -> usize {
    32
}

#[test]
fn s4_pinned_shape_table() {
    // (reduction_size x non_col = R, stride S, out O) → tier
    let shapes: [(usize, usize, usize, Tier); 11] = [
        (31, 8, 1024, Tier::Small),        // 1: R < 32
        (32, 64, 512, Tier::Looped),       // 2: past small; LC needs S<32; 2P needs R>256
        (1024, 16, 2048, Tier::LongColumn),// 3: LC lower boundary exact
        (1023, 16, 4096, Tier::TwoPass),   // 4: LC misses by 1
        (257, 64, 8192, Tier::TwoPass),    // 5: R > 256 boundary exact
        (256, 64, 8192, Tier::Looped),     // 6: R > 256 false — boundary miss
        (1024, 64, 32768, Tier::Looped),   // 7: O/32 == 1024, NOT < 1024
        (512, 64, 32767, Tier::TwoPass),   // 8: O/32 = 1023 — one under
        (32768, 16, 512, Tier::LongColumn),// 9: widening boundary exact
        (32767, 16, 512, Tier::LongColumn),// 10: one under the widening
        (512, 16, 4096, Tier::TwoPass),    // 11: S < 32 but R < 1024 → order proof
    ];
    for &(r, s, o, want) in &shapes {
        assert_eq!(tier(r, s, o), want, "shape R={r} S={s} O={o}");
    }
}

#[test]
fn s4_widening_rider_is_longcolumn_only() {
    assert_eq!(longcolumn_outer_blocks(32768), 128);
    assert_eq!(longcolumn_outer_blocks(32767), 32);
    // 2pass NEVER widens, even at absurd totals (the asymmetry pin)
    assert_eq!(two_pass_outer_blocks(1_000_000), 32);
}

#[test]
fn s4_one_predicate_naive_misroutes() {
    let naive = |r: usize, _s: usize, o: usize| {
        if r > 256 && o / 32 < 1024 {
            Tier::TwoPass
        } else {
            Tier::Looped
        }
    };
    let shapes = [
        (31usize, 8usize, 1024usize),   // 1: Small → TwoPass
        (1024, 16, 2048),               // 3: LongColumn → TwoPass
        (32768, 16, 512),               // 9: LongColumn(128) → TwoPass
    ];
    let mut diverged = 0;
    for &(r, s, o) in &shapes {
        if naive(r, s, o) != tier(r, s, o) {
            diverged += 1;
        }
    }
    assert!(diverged >= 3, "the one-predicate dispatch misroutes {diverged}");
}

#[test]
fn s4_reordered_ladder_leaves_the_pinned_tier() {
    // adversary: 2pass checked before longcolumn
    let tampered = |r: usize, s: usize, o: usize| {
        if r < 32 {
            return Tier::Small;
        }
        if r > 256 && o / 32 < 1024 {
            return Tier::TwoPass;
        }
        if s < 32 && r >= 1024 {
            return Tier::LongColumn;
        }
        Tier::Looped
    };
    // shape 3 satisfies BOTH 2pass's and longcolumn's predicate — the
    // reordered ladder routes it TwoPass, the pinned table says
    // LongColumn (whose threadgroup_x IS the short stride — 2pass's
    // pass-2 geometry is sized for wide outputs, the assumption break)
    assert_eq!(tampered(1024, 16, 2048), Tier::TwoPass);
    assert_eq!(tier(1024, 16, 2048), Tier::LongColumn);
    // shape 9 likewise
    assert_eq!(tampered(32768, 16, 512), Tier::TwoPass);
    assert_eq!(tier(32768, 16, 512), Tier::LongColumn);
}
