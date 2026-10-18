//! Entries slice 34 — Batch 250 slice 1 (Plan 254 / Research 284): the
//! llama.cpp envelope pool's rust_perf candidate — `3dbcb7f` "cpu: reduce
//! AVX2 tiled accumulators together" (#30296). Every quote verbatim at
//! the pin this claim (llama.cpp @ 62318cd; the commit also an ancestor
//! of tip 797de63b). The rule is x86_64-only (`#[cfg(target_arch =
//! "x86_64")]` in both halves): on the aarch64 M3 the bench fixture
//! COMPILES but the AVX2 intrinsics are absent — the Pareto measurement
//! rides an x86_64 host (the 4090 box), the B204 staging precedent.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── simd: a bank of register-resident accumulator vectors is
        // reduced by an intra-register hadd butterfly — never N scalar
        // hsum round-trips through the stack ───────────────────────────
        // Source: ggml-org/llama.cpp @ 3dbcb7f (#30296),
        // ggml/src/ggml-cpu/tiled/tiled-kernel.cpp.
        PerfRule {
            rule_id: "avx2-tiled-accumulator-butterfly-epilogue".to_string(),
            name: "A fixed bank of SIMD accumulator vectors (one per output column) is reduced at the epilogue → an intra-register hadd butterfly over the BANK (pairwise _mm256_hadd_epi32, then _mm256_permute2x128 lo/hi, then one add) — never N per-vector scalar horizontal-sums each spilling through a stack store + vector reload".to_string(),
            category: "simd".to_string(),
            buggy_code: "// NAIVE: the microtile keeps one __m256i accumulator per output\n// column (8 of them, register-resident through the main loop) and\n// reduces each INDEPENDENTLY at the epilogue: a scalar hsum helper\n// per vector — extract hi lane, add, shuffle, add, shuffle, add,\n// store to a stack array — then reload the whole array as one\n// vector. Eight round-trips the butterfly never pays.\n\n#[cfg(target_arch = \"x86_64\")]\nuse std::arch::x86_64::*;\n\n#[cfg(target_arch = \"x86_64\")]\n#[target_feature(enable = \"avx2\")]\nunsafe fn epilogue_naive(acc: &mut Vec<__m256i>) -> __m256i {\n    // Horizontal sum helper: converts 8x32-bit int lane inside a YMM\n    // register to scalar int32 (the incumbent's own helper, verbatim\n    // shape).\n    unsafe fn hsum256_epi32(v: __m256i) -> i32 {\n        let low = _mm256_castsi256_si128(v);\n        let high = _mm256_extracti128_si256(v, 1);\n        let mut sum128 = _mm_add_epi32(low, high);\n        sum128 = _mm_add_epi32(sum128, _mm_shuffle_epi32(sum128, 0b10_11_00_01));\n        sum128 = _mm_add_epi32(sum128, _mm_shuffle_epi32(sum128, 0b01_00_11_10));\n        _mm_cvtsi128_si32(sum128)\n    }\n    let mut s1_vals = [0i32; 8];\n    for t in 0..8 {\n        s1_vals[t] = hsum256_epi32(acc[t]);\n    }\n    unsafe { _mm256_load_si256(s1_vals.as_ptr().cast()) }\n}".to_string(),
            fixed_code: "// OPT: the bank is ONE reduction problem, not eight. Reduce the\n// eight accumulators pairwise IN-REGISTER (hadd pairs adjacent\n// accumulators, halving the bank per level), then splice the two\n// surviving halves' 128-bit lanes with permute2x128 and add once —\n// zero stack stores, zero reloads, ~5 instructions for the whole\n// bank. (The source's own warning stands: this is a LOCAL epilogue\n// reduction over one microtile's registers — column sums, additions\n// modulo 2^32 — never a reassociation of a synced/order-pinned\n// fold.)\n\n#[cfg(target_arch = \"x86_64\")]\nuse std::arch::x86_64::*;\n\n#[cfg(target_arch = \"x86_64\")]\n#[target_feature(enable = \"avx2\")]\nunsafe fn epilogue_butterfly(acc: &[__m256i; 8]) -> __m256i {\n    // Reduce the eight accumulators to column sums in acc[0]..acc[7]\n    // order, with additions modulo 2^32 (the source comment).\n    let ab = unsafe { _mm256_hadd_epi32(acc[0], acc[1]) };\n    let cd = unsafe { _mm256_hadd_epi32(acc[2], acc[3]) };\n    let ef = unsafe { _mm256_hadd_epi32(acc[4], acc[5]) };\n    let gh = unsafe { _mm256_hadd_epi32(acc[6], acc[7]) };\n    // Each 128-bit half now holds four column sums over input lanes\n    // 0..3 or 4..7 (the source comment).\n    let abcd = unsafe { _mm256_hadd_epi32(ab, cd) };\n    let efgh = unsafe { _mm256_hadd_epi32(ef, gh) };\n    let lo = unsafe { _mm256_permute2x128_si256(abcd, efgh, 0x20) };\n    let hi = unsafe { _mm256_permute2x128_si256(abcd, efgh, 0x31) };\n    unsafe { _mm256_add_epi32(lo, hi) }\n}".to_string(),
            context_notes: Some("BENCH-VERDICT-PENDING-x86 (NotBenchable on THIS aarch64 host by construction — the rule is x86_64-only, the Pareto validator's fixture compiles empty on the M3 which has no AVX2; the measured verdict lands with the landing commit from an x86_64-host run); the bench's arms share the SAME accumulators and make_input SAMPLE-PARITY-ASSERTS the two epilogues' column sums agree element-wise before any timing. SOURCE (ggml-org/llama.cpp @ 3dbcb7f, #30296 'cpu: reduce AVX2 tiled accumulators together', MIT; verified an ancestor of tip 797de63b; every quote verbatim at the pin this claim). THE INCUMBENT DELETED (ggml/src/ggml-cpu/tiled/tiled-kernel.cpp, verbatim): the per-vector scalar helper 'auto hsum256_epi32 = [](const __m256i v) -> int32_t {' with its '_mm_add_epi32(low, high)' + two '_mm_shuffle_epi32' folds + '_mm_cvtsi128_si32(sum128)', and its call site 's1_vals[t] = hsum256_epi32(acc[t]);' under '#pragma GCC unroll 8' storing to 'alignas(32) int32_t s1_vals[GROUP];' then '__m256i s1_vec = _mm256_load_si256((const __m256i *) s1_vals);' — eight store+reload round-trips the butterfly replaces. THE BUTTERFLY LANDED (verbatim): 'const __m256i ab = _mm256_hadd_epi32(acc[0], acc[1]);' through 'gh' (four pairwise hadds), 'const __m256i abcd = _mm256_hadd_epi32(ab, cd);' + 'efgh' (two more), 'const __m256i lo = _mm256_permute2x128_si256(abcd, efgh, 0x20);' + 'hi' with '0x31', '__m256i s1_vec = _mm256_add_epi32(lo, hi);' — five vector ops total for the whole bank; the source comments carry the lane algebra ('Each 128-bit half now holds four column sums over input lanes 0..3 or 4..7') and the arithmetic contract ('with additions modulo 2^32'). WHY hadd IS THE RIGHT PRIMITIVE HERE (the dedup hinge): hadd_epi32(a, b) produces [a0+a1, a2+a3, a4+a5, a6+a7 | b0+b1, ...] — it reduces WITHIN each accumulator's lanes AND pairs the two accumulators in one op, so a log-depth butterfly over the bank falls out of repeated hadds; the incumbent's shuffle tree instead reduces ONE vector's lanes to a SCALAR (a different endpoint), forcing the per-vector stack loop. REGIME: a FIXED, SMALL bank of register-resident accumulator vectors reduced once per tile/iteration (GEMM microtile epilogues, multi-accumulator dot products, per-column partial sums); AVX2 integer or float (the hadd lattice has a ps twin). ANTI-REGIMES: a SINGLE accumulator vector (the plain in-vector shuffle-reduce is the answer — there is no bank to butterfly; hadd's cross-vector pairing buys nothing); accumulators that must be REPORTED individually (the butterfly destroys the per-vector values); reductions whose ORDER crosses the sync/quorum boundary (the workspace determinism law — this epilogue reassociates; keep it LOCAL to the tile, the source's own column sums are per-microtile state, never a synced fold); tiny GROUP=2 banks (one hadd, the ladder is overkill but harmless — the law bites at 4+); aarch64 hosts (NEON has no cross-vector hadd — the pairwise shape becomes addp ladders, the same LAW different intrinsics, not this rule's x86 form). DEDUP ADJUDICATION: distinct from the broadcast-FMA GEMV family (entries_03-class: removes the horizontal reduce from the main LOOP via transposed layout — THIS owns the epilogue WHEN per-column accumulators are already live in registers); distinct from the fwht transform butterflies (kernel_opt B166: a many-to-one EXCHANGE network where every lane emerges holding different outputs — this is a many-to-one CONVERGENCE to one column-sum vector, the B166 distinct-from set's own words); distinct from horizontal-sum-of-one-vector helpers (the incumbent being deleted); distinct from bsearch-sorted-slice / reduction-order rules (no searching, no order contract). Grep 'hadd|horizontal.*bank|accumulator.*butterfly' — zero rule owners. Batch 250 slice 1 / Plan 254 / Research 284.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(avx2_tiled_accumulator_butterfly_bench_spec()),
        },
    ]
}

/// The epilogue bench: both arms hold the SAME eight register-resident
/// accumulators (built once per input) and reduce them N times; the arms
/// differ ONLY in the epilogue shape. Sample parity asserts the two
/// epilogues' column sums agree element-wise (i32 wrapping equality) on
/// 256 samples before any timing. x86_64-only by construction (the
/// rule's own cfg law); on aarch64 the validator's generated crate
/// compiles both arms to empty shims and the bench refuses — the
/// measurement is an x86_64-host act.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn avx2_tiled_accumulator_butterfly_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// The microtile-epilogue shape: GROUP=8 accumulator vectors, each
// holding 8 i32 lanes (a deterministic pseudo-GEMM fills them), reduced
// once per "tile" — n tiles per input. Both arms share the SAME
// accumulators; make_input sample-parity-asserts the two epilogues
// agree element-wise (wrapping i32) on 256 samples.
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    // [tile][bank][lane] — deterministic accumulator contents
    tiles: Vec<Vec<[[i32; 8]; 8]>>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0xC0FF_EE12_3456_789A;
    let per_tile_groups = 64; // tiles amortize the fixed bank
    let tiles_count = (n / per_tile_groups).max(1);
    let mut tiles = Vec::with_capacity(tiles_count);
    for _ in 0..tiles_count {
        let mut groups = Vec::with_capacity(per_tile_groups);
        for _ in 0..per_tile_groups {
            let mut bank = [[0i32; 8]; 8];
            for g in bank.iter_mut() {
                for lane in g.iter_mut() {
                    *lane = (lcg(&mut state) % 2001) as i32 - 1000;
                }
            }
            groups.push(bank);
        }
        tiles.push(groups);
    }
    let input = Input { tiles };
    // ---- SAMPLE PARITY (the B245 lesson, embedded at construction) ----
    let tile0 = &input.tiles[0];
    for i in 0..256 {
        let (t, g) = ((i * 13) % tile0.len(), (i * 7) % tile0[0].len());
        let bank = tile0[t][g];
        let a = unsafe { buggy::epilogue(&bank) };
        let b = unsafe { fixed::epilogue(&bank) };
        let mut av = [0i32; 8];
        let mut bv = [0i32; 8];
        unsafe {
            _mm256_storeu_si256(av.as_mut_ptr().cast(), a);
            _mm256_storeu_si256(bv.as_mut_ptr().cast(), b);
        }
        assert_eq!(av, bv, "epilogue parity failure at sample {i}");
    }
    input
}

mod buggy {
    use super::*;
    // The incumbent shape: per-vector scalar hsum through the stack,
    // then reload as one vector.
    #[target_feature(enable = "avx2")]
    unsafe fn hsum256_epi32(v: __m256i) -> i32 {
        let low = _mm256_castsi256_si128(v);
        let high = _mm256_extracti128_si256(v, 1);
        let mut sum128 = _mm_add_epi32(low, high);
        sum128 = _mm_add_epi32(sum128, _mm_shuffle_epi32(sum128, 0b10_11_00_01));
        sum128 = _mm_add_epi32(sum128, _mm_shuffle_epi32(sum128, 0b01_00_11_10));
        _mm_cvtsi128_si32(sum128)
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn epilogue(bank: &[[i32; 8]; 8]) -> __m256i {
        let mut acc: [__m256i; 8] = [_mm256_setzero_si256(); 8];
        for t in 0..8 {
            acc[t] = _mm256_loadu_si256(bank[t].as_ptr().cast());
        }
        let mut s1_vals = [0i32; 8];
        for t in 0..8 {
            s1_vals[t] = hsum256_epi32(acc[t]);
        }
        _mm256_load_si256(s1_vals.as_ptr().cast())
    }

    pub fn count(input: &Input) -> i64 {
        let mut sink = 0i64;
        for tile in &input.tiles {
            for bank in tile {
                let v = unsafe { epilogue(bank) };
                let mut out = [0i32; 8];
                unsafe { _mm256_storeu_si256(out.as_mut_ptr().cast(), v) };
                sink += i64::from(out[0]);
            }
        }
        sink
    }
}

mod fixed {
    use super::*;
    // The butterfly: pairwise hadd over the bank, permute2x128 splice,
    // one add.
    #[target_feature(enable = "avx2")]
    pub unsafe fn epilogue(bank: &[[i32; 8]; 8]) -> __m256i {
        let mut acc: [__m256i; 8] = [_mm256_setzero_si256(); 8];
        for t in 0..8 {
            acc[t] = _mm256_loadu_si256(bank[t].as_ptr().cast());
        }
        let ab = _mm256_hadd_epi32(acc[0], acc[1]);
        let cd = _mm256_hadd_epi32(acc[2], acc[3]);
        let ef = _mm256_hadd_epi32(acc[4], acc[5]);
        let gh = _mm256_hadd_epi32(acc[6], acc[7]);
        let abcd = _mm256_hadd_epi32(ab, cd);
        let efgh = _mm256_hadd_epi32(ef, gh);
        let lo = _mm256_permute2x128_si256(abcd, efgh, 0x20);
        let hi = _mm256_permute2x128_si256(abcd, efgh, 0x31);
        _mm256_add_epi32(lo, hi)
    }

    pub fn count(input: &Input) -> i64 {
        let mut sink = 0i64;
        for tile in &input.tiles {
            for bank in tile {
                let v = unsafe { epilogue(bank) };
                let mut out = [0i32; 8];
                unsafe { _mm256_storeu_si256(out.as_mut_ptr().cast(), v) };
                sink += i64::from(out[0]);
            }
        }
        sink
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(input));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(input));
}
"#
            .to_string(),
    }
}
