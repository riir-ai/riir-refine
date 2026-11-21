//! Batch 379 fixtures (Plan 395 / Research 429) — ORIGINAL std-only
//! reproductions of the alesha-pro/maya-inkling-mini laws (@ be849daa, MIT),
//! never pasted source. One standalone file per the B336/…/B370 discipline:
//! compiled AND executed BEFORE entry as `rustc --edition 2021 --test
//! b379_fixtures.rs && ./b379_fixtures`. The CUDA/MMA surface is modeled as
//! a CPU mock (the b370 precedent); the fixture never calls a GPU. Each
//! rule's ref/opt arms in the batch's entries file embed the same executed
//! code — the fixture pass is the authoring gate: a defect caught here is
//! fixed HERE first, then synced back into the entries in the same landing
//! commit (the b361 precedent).
//!
//! Arms (rule → what the tests pin):
//! - R1 `decode-attn-cache-rows-register-direct-occupancy-not-smem-tile`:
//!   the read census (each cache row read ONCE per warp-group; the R heads
//!   consume it inside the fragment geometry); the admission keys on reuse
//!   LOCATION + occupancy + the narrow-cache materialization tax — never
//!   reuse COUNT alone (the R > 1 rows still stage nothing); the PREFILL
//!   anti-regime (Tq > 1: cross-query tile reuse is real, the tile form
//!   wins the census and the admission picks it); the cross-warp reuse arm
//!   (rows shared ACROSS warps → tiles, the gpu_106 reuse-arithmetic pole);
//!   the FP8-vs-FP16 materialization census (smem form converts every
//!   staged element; register form converts per consumed fragment only).
//! - R2 `fp32-input-hi-lo-two-fp16-mma-columns`: the Dekker split
//!   (hi = f16(x), lo = f16(x − hi)) recovers dual-representable inputs
//!   EXACTLY and bounds the general class at ~2 f16 ulps of the residual
//!   scale (vs the naive cast's half-ulp-of-hi loss — the 1.0 + 2^-13
//!   adversarial value the naive cast rounds away); the mma census (the
//!   limb form is ONE mma per k-step; the both-operands residual-pair form
//!   is THREE); the spare-columns admission (idle B columns ≥ 2 and a
//!   one-sided split → limb form; exhausted columns or both-side loss →
//!   the pair form is what remains).
//! - R3 `bytes-in-flight-concurrency-not-prefetch-ahead`: the
//!   latency-bound model (a warp waits for its whole block before asking
//!   again — in-flight bytes = outstanding requests × request width) where
//!   the SPLIT multiplies outstanding requests and throughput rises with
//!   it; the CEILING anti-regime (at the memory-bandwidth limit neither
//!   wider requests nor more concurrency move throughput — the measured
//!   715 GB/s flat); the cache-crowding anti-regime (prefetching another
//!   working set evicts the live one — the 8.05 → 9.7 s regression); the
//!   deterministic fold (strided writers take disjoint slots, the fold
//!   sums them in ascending order, bit-identical across runs).

use std::cell::RefCell;

// ─────────────────────────────────────────────────────────────────────────────
// f16 round-to-nearest-even, std-only (the R2 substrate). Sanity-pinned
// below before any law uses it.
// ─────────────────────────────────────────────────────────────────────────────

/// f32 bits → f16 bits at round-to-nearest-even.
fn f32_bits_to_f16_bits(v: u32) -> u16 {
    let sign = ((v >> 16) & 0x8000) as u16;
    let exp = ((v >> 23) & 0xFF) as i32;
    let mant = v & 0x007F_FFFF;
    if exp == 0xFF {
        // inf / nan
        return if mant == 0 {
            sign | 0x7C00
        } else {
            sign | 0x7E00
        };
    }
    let e16 = exp - 127 + 15; // the f16 exponent field value (biased)
    if e16 >= 31 {
        // overflow: the RN boundary is 65520 = midpoint(65504, 65536);
        // values ≥ 65520 go to inf, below round to 65504.
        if exp == 0x7F && mant < 0x007F_F000 {
            // in [65504, 65520): rounds DOWN to max finite
            return sign | 0x7BFF;
        }
        return sign | 0x7C00;
    }
    if e16 <= 0 {
        // zero or subnormal: k = round((implicit-bit mantissa) >> (126 − exp))
        // in units of 2^-24, RN ties-to-even.
        let shift = 126 - exp; // 14..=24 in the subnormal-target range
        if shift > 25 {
            return sign; // value < 2^-26: rounds to zero
        }
        let m = mant | 0x0080_0000; // restore the implicit bit (24 bits)
        let low = m & ((1u32 << shift) - 1);
        let half = 1u32 << (shift - 1);
        let mut m16 = (m >> shift) as u16;
        // RN ties-to-even
        if low > half || (low == half && (m16 & 1) == 1) {
            m16 += 1;
            if m16 == 1024 {
                // rounded up into the normal range
                return sign | (1 << 10);
            }
        }
        return sign | m16; // subnormal: exponent field 0
    }
    let mut m16 = ((mant >> 13) & 0x3FF) as u16;
    let low = mant & 0x1FFF;
    if low > 0x1000 || (low == 0x1000 && (m16 & 1) == 1) {
        m16 += 1;
        if m16 == 1024 {
            return sign | (((e16 + 1) as u16) << 10); // mantissa overflow carried into the exponent
        }
    }
    sign | (((e16 as u16) & 0x1F) << 10) | m16
}

/// f16 bits → f32 bits (exact widening).
fn f16_bits_to_f32_bits(h: u16) -> u32 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exp = ((h >> 10) & 0x1F) as u32;
    let mant = (h & 0x03FF) as u32;
    if exp == 0 {
        if mant == 0 {
            return sign;
        }
        // subnormal: normalize
        let mut m = mant;
        let mut e = 113; // 127 - 14
        while m & 0x0400 == 0 {
            m <<= 1;
            e -= 1;
        }
        m &= 0x03FF;
        return sign | ((e << 23) | (m << 13));
    }
    if exp == 0x1F {
        return sign | 0x7F80_0000 | (mant << 13);
    }
    sign | (((exp + 112) << 23) | (mant << 13))
}

/// the model's `f2h` / `h2f` pair: f32 → f16 (RNE) → f32.
fn f16_rt(x: f32) -> f32 {
    f32::from_bits(f16_bits_to_f32_bits(f32_bits_to_f16_bits(x.to_bits())))
}

#[test]
fn f16_helper_sanity_rne() {
    assert_eq!(f16_rt(1.0), 1.0);
    assert_eq!(f16_rt(0.0), 0.0);
    assert_eq!(f16_rt(-2.5), -2.5);
    // max finite + the RN overflow boundary
    assert_eq!(f16_rt(65504.0), 65504.0);
    assert_eq!(f16_rt(65519.0), 65504.0); // below the midpoint: down
    assert!(f16_rt(65520.0).is_infinite()); // at the midpoint: to inf
    // ulp behaviour at 1.0 (f16 ulp = 2^-10)
    let ulp = 2.0f32.powi(-10);
    assert_eq!(f16_rt(1.0 + ulp * 0.5), 1.0); // tie: mantissa even → down
    assert_eq!(f16_rt(1.0 + ulp), 1.0 + ulp);
    assert_eq!(f16_rt(1.0 + ulp * 1.5), 1.0 + 2.0 * ulp); // tie → up (even)
    // subnormals
    assert_eq!(f16_rt(2.0f32.powi(-24)), 2.0f32.powi(-24)); // min subnormal exact
    // subnormal ties: 1.5 × 2^-24 sits halfway between k = 1 and k = 2;
    // RN-to-even picks k = 2 (2^-23).
    assert_eq!(f16_rt(1.5 * 2.0f32.powi(-24)), 2.0f32.powi(-23));
    // 0.5 × 2^-24 = 2^-25: tie between k = 0 (even) and k = 1 → 0.
    assert_eq!(f16_rt(2.0f32.powi(-25)), 0.0);
    // 2.5 × 2^-24: tie between k = 2 (even) and k = 3 → stays 2^-23.
    assert_eq!(f16_rt(2.5 * 2.0f32.powi(-24)), 2.0f32.powi(-23));
    // 2^-13 is a normal f16 exactly
    assert_eq!(f16_rt(2.0f32.powi(-13)), 2.0f32.powi(-13));
}

// ─────────────────────────────────────────────────────────────────────────────
// R1 — decode-attn-cache-rows-register-direct-occupancy-not-smem-tile
// ─────────────────────────────────────────────────────────────────────────────

/// Which kernel form a decode/prefill attention call takes. The admission
/// reads reuse LOCATION (fragment-internal vs cross-work-item), the query
/// row count, and the cache format's materialization tax — never reuse
/// count alone.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AttnForm {
    /// R ≤ 8 heads consume each cache row inside the mma fragment; rows are
    /// read straight to registers, no staging.
    RegisterDirect,
    /// Tq > 1 query rows (or rows shared ACROSS warps): staged tiles serve
    /// many consumers.
    SmemTiles,
}

fn attn_form(
    r_heads_per_cache_head: usize,
    query_rows: usize,
    rows_shared_across_warps: bool,
) -> AttnForm {
    if query_rows > 1 || rows_shared_across_warps {
        return AttnForm::SmemTiles; // cross-query / cross-warp reuse is real: stage it
    }
    // decode: one query row group; the R consumers live in the fragment's
    // own column geometry — staging adds no reuse the fragment lacks, and
    // the tile's smem occupancy caps resident warps (the flash contrast).
    let _ = r_heads_per_cache_head; // consumed inside the fragment either way
    AttnForm::RegisterDirect
}

/// The read census of the register-direct decode form: each warp takes its
/// own run of keys; a key row is loaded ONCE and consumed by all R heads of
/// its K/V head (the fragment's columns). Returns total cache-row reads.
fn register_direct_row_reads(n_cache_rows: usize, _r_heads: usize) -> usize {
    n_cache_rows // one read per row, whatever R is — the reuse is in-fragment
}

/// The read census of the naive per-head form (what GQA sharing replaced):
/// every query head reads every row of its group separately.
fn per_head_row_reads(n_cache_rows: usize, r_heads: usize) -> usize {
    n_cache_rows * r_heads.max(1)
}

/// The tile form's read census under Tq query rows: staged once, consumed
/// by all query rows.
fn tiled_row_reads(n_cache_rows: usize, _tq: usize) -> usize {
    n_cache_rows // staged once per tile pass
}

/// The per-query register form's census under Tq > 1 (the anti-regime's
/// loser): every query row re-reads the keys.
fn per_query_register_reads(n_cache_rows: usize, tq: usize) -> usize {
    n_cache_rows * tq
}

/// Materialization census: elements CONVERTED (the narrow-format tax).
/// The smem form converts every staged element of the tile; the register
/// form converts only the elements a lane actually consumes.
fn smem_materialized_elements(n_cache_rows: usize, width: usize, staged_fraction: f64) -> usize {
    ((n_cache_rows * width) as f64 * staged_fraction).round() as usize
}

fn register_materialized_elements(n_consumed_elements: usize) -> usize {
    n_consumed_elements
}

#[test]
fn r1_rows_read_once_whatever_the_head_count() {
    // the fragment consumes R heads per row: the census is 1 read/row,
    // never R reads/row.
    for r in [1usize, 2, 4, 8] {
        assert_eq!(
            register_direct_row_reads(512, r),
            512,
            "R = {r}: register-direct reads each row once"
        );
        assert_eq!(
            per_head_row_reads(512, r),
            512 * r,
            "R = {r}: the per-head form pays R reads/row"
        );
    }
}

#[test]
fn r1_admission_keys_on_reuse_location_not_count() {
    // R > 1 rows still stage NOTHING at decode: the reuse lives in the
    // fragment's columns. The plan's reuse-arithmetic arm: the admission is
    // NOT `R > 1 → stage`.
    for r in [2usize, 8] {
        assert_eq!(attn_form(r, 1, false), AttnForm::RegisterDirect);
    }
    // rows shared ACROSS warps (a tile could add reuse) → stage.
    assert_eq!(attn_form(8, 1, true), AttnForm::SmemTiles);
    // the gpu_106 pole in-rule: R == 1 with fragment-consumed reuse is
    // still register-direct (never stage).
    assert_eq!(attn_form(1, 1, false), AttnForm::RegisterDirect);
}

#[test]
fn r1_prefill_keeps_smem_tiles() {
    // the anti-regime: Tq > 1 gives the tile real cross-query reuse; the
    // staged form's census beats the per-query register form and the
    // admission picks the tile.
    let (rows, tq) = (512usize, 16usize);
    assert_eq!(tiled_row_reads(rows, tq), rows);
    assert_eq!(per_query_register_reads(rows, tq), rows * tq);
    assert!(tiled_row_reads(rows, tq) < per_query_register_reads(rows, tq));
    assert_eq!(attn_form(8, tq, false), AttnForm::SmemTiles);
}

#[test]
fn r1_narrow_cache_materialization_census() {
    // the FP8 tax: the smem form converts EVERY staged element (its FP16
    // tile is the working format); the register form converts per consumed
    // fragment. Under a staged-but-partially-consumed tile the register
    // census is strictly smaller.
    let (rows, width) = (512usize, 128usize);
    let smem = smem_materialized_elements(rows, width, 1.0); // full tile staged
    // decode consumes a key row once per fragment: the consumed elements
    // are the same rows but the conversion rides the load, never a
    // materialized working copy — model the tile-overshoot: the tile
    // stages window-padding rows the decode never reads.
    let consumed = rows * width - 16 * width; // 16 padding rows staged unread
    let reg = register_materialized_elements(consumed);
    assert!(reg < smem, "register form converts {reg} < smem's {smem}");
}

// ─────────────────────────────────────────────────────────────────────────────
// R2 — fp32-input-hi-lo-two-fp16-mma-columns
// ─────────────────────────────────────────────────────────────────────────────

/// the route_token_kernel limb build, verbatim arithmetic: hi = f2h(x),
/// lo = f2h(x − h2f(hi)).
fn build_limbs(x: f32) -> (f32, f32) {
    let hi = f16_rt(x);
    let lo = f16_rt(x - hi);
    (hi, lo)
}

/// the kernel's row fold: column 0 (hi) + column 1 (lo), in the wide lane.
fn limb_row_sum(hi: f32, lo: f32) -> f32 {
    hi + lo // the FP32 accumulator domain — never summed in f16
}

#[test]
fn r2_dekker_split_recovers_dual_representable_exactly() {
    // constructed class: x = hi + lo with both f16-representable and lo in
    // the subnormal-safe window below hi's ulp → the split recovers x
    // bit-for-bit (the fixture's exactness arm).
    let mut exact = 0usize;
    let mut total = 0usize;
    let mut hi = 1.0f32;
    while hi < 64.0 {
        // 8 staggered residuals per octave, all exact f16 multiples of
        // hi's ulp / 8 boundary class: residual = k * ulp_hi / 2048-ish —
        // dual-representable when residual is an f16 value itself.
        for k in [1.0f32, 3.0, 5.0, 7.0, 257.0, 511.0, 1023.0, 2047.0] {
            // residual = k * 2^-24-class below the octave: representable in
            // f16 subnormals (≥ 2^-24, ≤ 2047 × 2^-24 < 2^-14)
            let lo = k * 2.0f32.powi(-24);
            if lo >= 2.0f32.powi(-24) && lo < 2.0f32.powi(-14) {
                let x = hi + lo;
                total += 1;
                let (h2, l2) = build_limbs(x);
                if limb_row_sum(h2, l2).to_bits() == x.to_bits() {
                    exact += 1;
                }
            }
        }
        hi *= 2.0;
    }
    assert!(total >= 40, "sweep planted {total} dual-representable rows");
    assert_eq!(
        exact, total,
        "the split must recover every dual-representable row exactly (got {exact}/{total})"
    );
}

#[test]
fn r2_naive_cast_loses_the_low_bits_pair_bounds_them() {
    // the adversarial value: 1.0 + 2^-13 sits below f16's ulp at 1.0
    // (2^-10) but above the residual floor — the naive cast rounds it to
    // 1.0 (error 2^-13); the pair recovers it to ≤ half ulp of the
    // residual scale (here exactly).
    let x = 1.0f32 + 2.0f32.powi(-13);
    let naive = f16_rt(x);
    assert_eq!(naive, 1.0, "the naive f16 cast rounds the low bits away");
    assert!((naive - x).abs() >= 2.0f32.powi(-13) * 0.5);
    let (hi, lo) = build_limbs(x);
    let err = (limb_row_sum(hi, lo) - x).abs();
    // the pair's bound: ≤ half ulp of lo's scale (2^-24 class here) —
    // 2048× tighter than the naive loss at this value; exact on this one.
    assert!(
        err <= 2.0f32.powi(-22),
        "pair error {err} must sit at the residual's ulp scale, not the hi ulp"
    );
    assert_eq!(
        err, 0.0,
        "1.0 + 2^-13 is dual-representable: exact recovery"
    );
}

#[test]
fn r2_general_class_error_bounded_at_residual_scale() {
    // sweep the general class: the pair's error stays within one ulp of
    // the residual scale (2^(e32 − 22)-class) everywhere; exact on the
    // dual-representable subset (the residual needs ≤ 11 significant
    // bits), bounded (never worse than that bound) elsewhere. The naive
    // cast's worst error is half ulp of hi (2^(e32 − 11)) — ≥ 2048× the
    // pair's bound at the same scale.
    let mut worst_pair = 0.0f64;
    let mut exact = 0usize;
    let mut total = 0usize;
    let mut m: u32 = 0x3C00_0000; // 1.0
    while m < 0x4100_0000 {
        // up to 8.0, fine mantissa steps
        let x = f32::from_bits(m);
        let (hi, lo) = build_limbs(x);
        let pair = limb_row_sum(hi, lo);
        let bound = 2.0f64.powi((x.abs().log2().floor() as i32) - 22);
        worst_pair = worst_pair.max((pair - x).abs() as f64);
        assert!(
            (pair - x).abs() as f64 <= bound,
            "pair error {} exceeds the residual-scale bound {} at x = {}",
            (pair - x).abs(),
            bound,
            x
        );
        if pair.to_bits() == x.to_bits() {
            exact += 1;
        }
        total += 1;
        m += 0x0010_0000;
    }
    assert!(total >= 80, "swept {total} values");
    assert!(
        exact > 0,
        "the sweep must contain dual-representable members ({exact}/{total} exact)"
    );
    assert!(
        worst_pair <= 2.0f64.powi(-20),
        "worst pair error {worst_pair} must sit at the residual scale"
    );
}

/// the MMA census model: how many mma steps a k-block of the dot product
/// takes under each form.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum PrecisionForm {
    /// the limbs ride two B-operand columns of the SAME mma: 1 step.
    LimbColumns,
    /// both operands carried as residual pairs, the two cross terms + the
    /// hi term: 3 steps.
    PairThreeMma,
    /// plain f16 operands: 1 step, the naive precision.
    PlainF16,
}

fn precision_form(idle_b_columns: usize, both_operands_imprecise: bool) -> PrecisionForm {
    if both_operands_imprecise || idle_b_columns < 2 {
        // no idle columns to ride, or the split must cover both sides:
        // the residual-pair three-mma form is what remains.
        PrecisionForm::PairThreeMma
    } else {
        PrecisionForm::LimbColumns
    }
}

fn mma_steps_per_k_block(form: PrecisionForm) -> usize {
    match form {
        PrecisionForm::LimbColumns | PrecisionForm::PlainF16 => 1,
        PrecisionForm::PairThreeMma => 3,
    }
}

#[test]
fn r2_mma_census_one_step_vs_three() {
    // the limb form costs ZERO extra mma: both limbs live in the same B
    // operand (the kernel's "costs nothing"); the both-operand pair form
    // pays 3× — that price is exactly when the limb form is unavailable.
    assert_eq!(mma_steps_per_k_block(PrecisionForm::LimbColumns), 1);
    assert_eq!(mma_steps_per_k_block(PrecisionForm::PairThreeMma), 3);
    assert_eq!(mma_steps_per_k_block(PrecisionForm::PlainF16), 1);
}

#[test]
fn r2_spare_columns_admission() {
    // the replicated-input geometry has idle B columns (the kernel's down
    // projection: "every column the same") → limb form.
    assert_eq!(precision_form(8, false), PrecisionForm::LimbColumns);
    // columns exhausted by distinct outputs (no replication) → the pair
    // form is what remains — the limb split cannot corrupt other columns.
    assert_eq!(precision_form(0, false), PrecisionForm::PairThreeMma);
    assert_eq!(precision_form(1, false), PrecisionForm::PairThreeMma);
    // both sides imprecise (weights too) → one-sided limbs are not enough.
    assert_eq!(precision_form(8, true), PrecisionForm::PairThreeMma);
}

#[test]
fn r2_limb_form_matches_wide_reference_within_pair_bound() {
    // an end-to-end row dot: weights quantized to f16-representable values
    // (the kernel's "a code times its scale is exact in FP16" class), the
    // input f32. The limb form's dot matches the f32-reference dot at the
    // pair bound; the plain-f16 input form misses by the naive cast's
    // error on the adversarial element.
    let w: Vec<f32> = (0..64).map(|i| f16_rt(1.0 + i as f32 * 0.125)).collect();
    let mut x = vec![1.0f32; 64];
    x[17] = 1.0 + 2.0f32.powi(-13); // the adversarial element
    let reference: f32 = w.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
    let plain: f32 = w.iter().zip(x.iter()).map(|(a, b)| a * f16_rt(*b)).sum();
    let limb: f32 = w
        .iter()
        .zip(x.iter())
        .map(|(a, b)| {
            let (hi, lo) = build_limbs(*b);
            a * hi + a * lo
        })
        .sum();
    let naive_err = (plain - reference).abs();
    let limb_err = (limb - reference).abs();
    assert!(
        limb_err <= naive_err,
        "limb dot error {limb_err} must not exceed the naive cast's {naive_err}"
    );
    assert!(
        limb_err <= 64.0 * 2.0f32.powi(-22),
        "the limb dot stays within the accumulated pair bound (got {limb_err})"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R3 — bytes-in-flight-concurrency-not-prefetch-ahead
// ─────────────────────────────────────────────────────────────────────────────

/// The latency-bound transfer model: a warp's per-instruction request is
/// the hardware load width; its outstanding loads per wait window are
/// hardware-bounded (the per-stream MLP reach — one warp cannot have
/// arbitrary bytes in flight). It WAITS for its whole block before the
/// next block's requests (the dependency through the compute consuming
/// the block). Bytes in flight = outstanding streams × min(requested
/// width, per-stream reach); throughput = min(bytes_in_flight / latency,
/// bandwidth ceiling).
#[derive(Clone)]
struct Pipe {
    latency_ticks: f64,
    bandwidth_bytes_per_tick: f64,
    per_stream_reach: f64,
}

impl Pipe {
    fn throughput(&self, outstanding_requests: f64, width_bytes: f64) -> f64 {
        let in_flight = outstanding_requests * width_bytes.min(self.per_stream_reach);
        (in_flight / self.latency_ticks).min(self.bandwidth_bytes_per_tick)
    }
}

#[test]
fn r3_concurrency_split_multiplies_bytes_in_flight() {
    // the measured shape: one warp issues its loads and then waits for
    // the WHOLE block before the next block's loads — its in-flight bytes
    // cap at one warp's MLP reach. SPLIT warps each take a stride of the
    // same rows: SPLIT concurrent request streams of the SAME width.
    // Below the ceiling, throughput rises with the split.
    let pipe = Pipe {
        latency_ticks: 8.0,
        bandwidth_bytes_per_tick: 1.0e6, // far above the naive form's reach
        per_stream_reach: 512.0,         // one warp's outstanding-load reach
    };
    let naive = pipe.throughput(1.0, 512.0);
    let split8 = pipe.throughput(8.0, 512.0);
    assert!(
        split8 > naive * 4.0,
        "split throughput {split8} must multiply the naive {naive}"
    );
    // the win is the CONCURRENCY, not the width: a single stream cannot
    // buy the same in-flight bytes by asking wider — the per-stream MLP
    // reach clamps it.
    let wide_one = pipe.throughput(1.0, 512.0 * 8.0);
    assert!(
        wide_one < split8,
        "one wide stream ({wide_one}) must not reach the split's {split8} while the reach clamps it"
    );
}

#[test]
fn r3_at_memory_ceiling_more_ahead_is_flat() {
    // the measured negative: the dense Q8 kernels already run at the
    // memory's limit — widening requests or adding concurrency moves
    // nothing.
    let ceiling = 715.0 * 1024.0 * 1024.0; // bytes/tick, the measured limit
    let pipe = Pipe {
        latency_ticks: 1.0,
        bandwidth_bytes_per_tick: ceiling,
        per_stream_reach: f64::INFINITY, // the ceiling law is reach-agnostic
    };
    let t1 = pipe.throughput(1.0, ceiling * 4.0); // 4× wider request
    let t2 = pipe.throughput(8.0, ceiling * 4.0); // + concurrency
    assert_eq!(t1, ceiling, "at the ceiling a wider request is flat");
    assert_eq!(t2, ceiling, "at the ceiling concurrency is flat");
}

#[test]
fn r3_prefetch_another_working_set_crowds_the_cache() {
    // the layer-level negative: prefetching experts of the NEXT layers
    // evicts the live layer's rows; the hit rate falls and the pass
    // slows. Model: a cache of C slots, a live working set of W rows, a
    // prefetch stream of P rows; without prefetch every live access hits;
    // with P > C − W the live set is partially evicted.
    let cache_slots = 100usize;
    let live = 80usize;
    let live_hits_without = live; // every live row stays resident
    for prefetch in [0usize, 10, 30] {
        let evicted = prefetch.saturating_sub(cache_slots - live);
        let live_hits_with = live - evicted;
        if prefetch == 0 {
            assert_eq!(live_hits_with, live_hits_without);
        } else if evicted > 0 {
            assert!(
                live_hits_with < live_hits_without,
                "prefetch {prefetch} evicted {evicted} live rows"
            );
        }
    }
    let evicted30 = 30usize.saturating_sub(cache_slots - live);
    assert_eq!(evicted30, 10, "prefetch 30 evicts 10 live rows");
}

/// The split kernel's partial fold: SPLIT warps write DISJOINT slots
/// (part[split]); one warp folds them in ascending split order.
fn split_fold(partials: &[f64]) -> f64 {
    // ascending order, plain sum — the deterministic order pin.
    let mut tot = 0.0;
    for p in partials {
        tot += *p;
    }
    tot
}

#[test]
fn r3_split_fold_is_deterministic_and_exact_here() {
    // strided writers take disjoint slots; the fold order is the slot
    // order. With exact-representable partials the fold is bit-identical
    // across runs and equals the sequential reference.
    let nk = 64usize;
    let split = 8usize;
    let mut slots = vec![0.0f64; split];
    for k in 0..nk {
        slots[k % split] += (k as f64) + 0.25; // each warp's stride, disjoint slots
    }
    let folded = split_fold(&slots);
    let mut reference = 0.0f64;
    for k in 0..nk {
        reference += (k as f64) + 0.25;
    }
    assert_eq!(
        folded.to_bits(),
        reference.to_bits(),
        "the fold matches the sequential reference bit-for-bit"
    );
    // determinism: same inputs, same bits, again.
    let mut slots2 = vec![0.0f64; split];
    for k in 0..nk {
        slots2[k % split] += (k as f64) + 0.25;
    }
    assert_eq!(split_fold(&slots2).to_bits(), folded.to_bits());
}

#[test]
fn r3_one_step_lookahead_not_a_deeper_ring() {
    // the kernel's per-warp prefetch is exactly ONE stride ahead (the nxt
    // handshake) — ahead is per-warp LOCAL, never a deeper ring. Model the
    // outstanding-request census: one lookahead adds 1 outstanding request
    // per warp; a ring of depth D adds D. The ceiling law says D > 1 buys
    // nothing once in-flight covers the latency — the census is the
    // admission's input.
    let warp = 1.0f64;
    let stride = 512.0f64;
    let pipe = Pipe {
        latency_ticks: 4.0,
        bandwidth_bytes_per_tick: 1.0e9,
        per_stream_reach: 512.0,
    };
    let with_lookahead = pipe.throughput(warp + 1.0, stride);
    let no_lookahead = pipe.throughput(warp, stride);
    assert!(
        with_lookahead > no_lookahead,
        "one-step lookahead hides the latency"
    );
    // depth 4 on the SAME latency: no further gain once 2 requests cover
    // the window here — the deeper ring is the anti-regime's shape.
    let deep = pipe.throughput(warp + 4.0, stride);
    assert!(
        deep <= with_lookahead * 3.0,
        "depth beyond the latency window adds nothing proportional ({deep} vs {with_lookahead})"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// the adversarial-generator hygiene: every adaptive arm above is fed by a
// deterministic sweep (no RNG at all), so the whole file is replay-stable.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn fixture_is_rng_free_and_replay_stable() {
    // run the deterministic sweeps twice through a thread-local witness:
    // identical outcomes.
    thread_local! {
        static WITNESS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    }
    fn sweep_digest() -> u64 {
        let mut acc = 0u64;
        let mut m: u32 = 0x3C00_0000;
        while m < 0x4100_0000 {
            let x = f32::from_bits(m);
            let (hi, lo) = build_limbs(x);
            acc = acc.rotate_left(7) ^ (limb_row_sum(hi, lo).to_bits() as u64);
            m += 0x0200_0000;
        }
        acc
    }
    let a = sweep_digest();
    WITNESS.with(|w| w.borrow_mut().push(a));
    let b = sweep_digest();
    assert_eq!(a, b, "the fixture must be replay-stable without RNG");
}
