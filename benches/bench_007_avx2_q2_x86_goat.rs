//! Bench 007 — the 4090-lane x86 GOAT for the P53 rider on the kernel_opt
//! rule `lut-free-bit-spread-vadd4-unpack` (entries_gpu_33), the llama.cpp
//! #30447 AVX2 Q2_0 vec_dot distill (pin `0a731cce`, MIT; batch 231 claim
//! `68be73c34`, plan 233 `11e1a8ae9`).
//!
//! The rule lands NotBenchable-on-M3 (the M3 box is aarch64; every x86
//! intrinsic compiles to nothing there), so its x86 GOAT runs HERE (x86_64
//! with avx2 and fma only, loud refusal anywhere else). The exact mirror of
//! the bench_005 lane split (Batch 204's NEON rule: NotBenchable-on-4090,
//! M3-Max GOAT). Three gates, the same staged spec:
//!
//! 1. **G1 (integer-exactness)** — power-of-two-scale vectors: the AVX2
//!    packed kernel is BIT-IDENTICAL to the dequant-then-scalar-dot
//!    reference, and the reference is the NAIVE per-element codec form
//!    (the B62 bench-design law: comparing against a transposed-lane
//!    multi-accumulator scalar form measures LAYOUT, not codec). A panic
//!    here fails the gate permanently; it is a correctness claim, not a
//!    timing claim.
//! 2. **G2 (GEMV sweep, provenance-gated)** — rows {32..512} x K
//!    {256..4096}, AVX2 vs the naive scalar, interleaved best-of-N passes
//!    (contention adds time to SOME runs; the minimum separates it),
//!    correctness-coupled sinks (each arm's GEMV must equal the scalar
//!    reference bit-for-bit at setup, so an optimiser deleting the measured
//!    loop FAILS instead of reporting a fast zero — the timed-region
//!    loud-zero doctrine). Every citable number quotes the preflight
//!    PROVENANCE line; `AVX2_Q2_BENCH_REQUIRE_PROVENANCE=1` is the exit-2
//!    gate posture for the citable run.
//! 3. **G4 (allocation-free)** — a counting global allocator armed around
//!    the measured vec_dot path: zero allocations, by construction (the
//!    constant vectors hoist; nothing in the loop allocs).
//!
//! The kernel is the PR's three-facet pattern, transcribed verbatim:
//! - **OR-spread bit unpack** (`q2_0_unpack_32_avx2`): four consecutive
//!   Q2_0 weights per byte spread into byte lanes via `v |= v<<12;
//!   v |= v<<6; v &= 0x03030303` — OR, NEVER ADD (the overlapping copies
//!   only touch masked-off bits);
//! - **maddubs int algebra** (`q2_0_dot_32_avx2`): `sum((v-1)*y)` computed
//!   as `maddubs(v,y) - maddubs(1,y)`, exact for every int8 y (no i16
//!   saturation: |8*3*127| = 3048 < 32767);
//! - **two FMA-chain accumulators** (the main loop): interleaved block
//!   pairs each into their own `__m256` chain (latency hiding), odd tail
//!   onto chain 0, one final hsum.
//!
//! Verdict shape (the rust_perf Pareto validator, bespoke): G1 pass AND
//! AVX2 strictly faster at EVERY sweep shape AND zero allocs both arms =
//! `PARETO-PASS` (exit 0). Any other measured outcome = `PARETO-FAIL`
//! (exit 1) — a failed GOAT is a real outcome, recorded as such.
//!
//! ```bash
//! # the whole gate (G1 + G4 + provisional-labeled G2):
//! cargo bench --bench bench_007_avx2_q2_x86_goat
//! # the GATE posture (exit 2 unless preflight PASSED — the citable run):
//! AVX2_Q2_BENCH_REQUIRE_PROVENANCE=1 cargo bench --bench bench_007_avx2_q2_x86_goat
//! ```

#![allow(clippy::pedantic)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

// ── the codec (the rule's own shapes; the PR's kernels transcribed —
//    the #30447 diff was read in full at the pin) ─────────────────────────

/// Q2_0 block: 64 weights x 2 bits + one f16 scale. The scale rides as f16
/// BITS — std `f16` is still unstable on the pinned 1.98.1 (the corpus rule
/// text is string data, so it never had to compile); the fixtures use
/// power-of-two normals (exactly representable) decoded by
/// [`f16_bits_to_f32`].
#[derive(Clone, Copy)]
pub struct Q2Block {
    pub d: u16,       // f16 bit pattern
    pub qs: [u8; 16], // 64 weights x 2 bits, LSB pair first
}

/// Q8_0 block: 32 int8 activations + one f16 scale; ONE Q2 block (the
/// 64-weight whole) consumes TWO Q8 blocks (the two 32-weight halves).
#[derive(Clone, Copy)]
pub struct Q8Block {
    pub d: u16, // f16 bit pattern
    pub qs: [i8; 32],
}

/// Exact f16 -> f32 (normals, subnormals, zero; Inf/NaN propagate bit-wise
/// through the mantissa path but the fixtures never produce them).
fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0f32 } else { 1.0 };
    let exp = ((bits >> 10) & 0x1F) as i32;
    let man = (bits & 0x03FF) as f32;
    match exp {
        0 => sign * man * 2.0f32.powi(-24), // zero + subnormals
        0x1F => sign * f32::INFINITY,       // Inf/NaN boundary (unused)
        e => sign * (1.0 + man / 1024.0) * 2.0f32.powi(e - 15),
    }
}

/// f16 bit patterns of the power-of-two scales the fixtures use.
/// 1.0=0x3C00 2.0=0x4000 0.5=0x3800 0.25=0x3400 4.0=0x4400
const POW2_F16: [u16; 5] = [0x3C00, 0x4000, 0x3800, 0x3400, 0x4400];

// ── the NAIVE per-element scalar reference (the rule's buggy_code form,
//    UNCHANGED — the B62 law) ─────────────────────────────────────────────

pub fn vec_dot_scalar(xs: &[Q2Block], ys: &[Q8Block]) -> f32 {
    let mut sumf = 0.0f32;
    for (i, x) in xs.iter().enumerate() {
        let y0 = &ys[2 * i];
        let y1 = &ys[2 * i + 1];
        let mut acc0 = 0i32;
        for (k, &byte) in x.qs[..8].iter().enumerate() {
            for j in 0..4 {
                let code = (byte >> (2 * j)) & 0x03; // one code at a time
                acc0 += (code as i32 - 1) * y0.qs[4 * k + j] as i32;
            }
        }
        let mut acc1 = 0i32;
        for (k, &byte) in x.qs[8..].iter().enumerate() {
            for j in 0..4 {
                let code = (byte >> (2 * j)) & 0x03;
                acc1 += (code as i32 - 1) * y1.qs[4 * k + j] as i32;
            }
        }
        sumf += f16_bits_to_f32(x.d) * f16_bits_to_f32(y0.d) * acc0 as f32;
        sumf += f16_bits_to_f32(x.d) * f16_bits_to_f32(y1.d) * acc1 as f32;
    }
    sumf
}

// ── the AVX2 packed kernel (the PR's fixed_code form, with the avx2+fma
//    target_feature the intrinsics require) ───────────────────────────────

#[cfg(target_arch = "x86_64")]
pub mod avx2 {
    use super::{Q2Block, Q8Block, f16_bits_to_f32};
    use std::arch::x86_64::*;

    /// Four consecutive Q2_0 weights per byte spread into byte lanes:
    /// `v |= v<<12; v |= v<<6; v &= 0x03030303` — OR, NEVER ADD (the
    /// overlapping copies only touch masked-off bits). Verbatim from
    /// #30447 `q2_0_unpack_32_avx2`.
    ///
    /// # Safety
    ///
    /// The caller MUST have runtime-detected `avx2` before calling; `qs`
    /// MUST point at 8 readable bytes (one half of a Q2 block).
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn q2_0_unpack_32(qs: *const u8) -> __m256i {
        let v = _mm256_cvtepu8_epi32(_mm_loadl_epi64(qs.cast::<__m128i>()));
        let v = _mm256_or_si256(v, _mm256_slli_epi32::<12>(v));
        let v = _mm256_or_si256(v, _mm256_slli_epi32::<6>(v));
        _mm256_and_si256(v, _mm256_set1_epi32(0x0303_0303))
    }

    /// 32 weights against one Q8_0 block: `sum((v-1) * y) = sum(v*y) -
    /// sum(y)`, exact for every int8 y (no i16 saturation at these ranges).
    /// Verbatim from #30447 `q2_0_dot_32_avx2` (the Rust names are the
    /// stdarch spellings of the same intrinsics).
    ///
    /// # Safety
    ///
    /// The caller MUST have runtime-detected `avx2`; `qs` MUST point at 8
    /// readable bytes and `qy` at 32 readable i8.
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn q2_0_dot_32(qs: *const u8, qy: *const i8) -> __m256 {
        let v = q2_0_unpack_32(qs);
        let y = _mm256_loadu_si256(qy.cast::<__m256i>());
        let ones = _mm256_set1_epi8(1);
        let p = _mm256_sub_epi16(_mm256_maddubs_epi16(v, y), _mm256_maddubs_epi16(ones, y));
        _mm256_cvtepi32_ps(_mm256_madd_epi16(p, _mm256_set1_epi16(1)))
    }

    /// One Q2 block (two 32-weight halves) FMA'd into `acc` under its two
    /// half-scales. Verbatim from #30447 `q2_0_acc_block_avx2`.
    ///
    /// # Safety
    ///
    /// The caller MUST have runtime-detected `avx2` + `fma` (the fma arm is
    /// the PR's `__FMA__` posture — every x86_64 chip with avx2 in practice).
    #[inline]
    #[target_feature(enable = "avx2,fma")]
    unsafe fn q2_0_acc_block(x: &Q2Block, y0: &Q8Block, y1: &Q8Block, acc: __m256) -> __m256 {
        let dx = f16_bits_to_f32(x.d);
        let d0 = _mm256_set1_ps(dx * f16_bits_to_f32(y0.d));
        let d1 = _mm256_set1_ps(dx * f16_bits_to_f32(y1.d));
        let p0 = q2_0_dot_32(x.qs.as_ptr(), y0.qs.as_ptr());
        let p1 = q2_0_dot_32(x.qs[8..].as_ptr(), y1.qs.as_ptr());
        let acc = _mm256_fmadd_ps(d0, p0, acc);
        _mm256_fmadd_ps(d1, p1, acc)
    }

    /// Horizontal sum of 8 f32 lanes (the PR's `hsum_float_8`).
    ///
    /// # Safety
    ///
    /// The caller MUST have runtime-detected `avx2`.
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn hsum8(v: __m256) -> f32 {
        let hi = _mm256_extractf128_ps::<1>(v);
        let sum128 = _mm_add_ps(_mm256_castps256_ps128(v), hi);
        let shuf = _mm_movehdup_ps(sum128);
        let sums = _mm_add_ps(sum128, shuf);
        let res = _mm_add_ss(sums, _mm_movehl_ps(sums, sums));
        _mm_cvtss_f32(res)
    }

    /// The whole vec_dot: two FMA-chain accumulators over interleaved block
    /// pairs (latency hiding), odd tail onto chain 0, one final hsum —
    /// verbatim from #30447 `ggml_vec_dot_q2_0_q8_0`.
    ///
    /// # Safety
    ///
    /// The caller MUST have runtime-detected `avx2` AND `fma` before
    /// calling — `std::arch::is_x86_feature_detected!("avx2")` /
    /// `("fma")` — the accumulation contract has no scalar fallback by
    /// design (a fallback would measure a DIFFERENT kernel).
    pub unsafe fn vec_dot_q2_q8(xs: &[Q2Block], ys: &[Q8Block]) -> f32 {
        debug_assert_eq!(
            ys.len(),
            2 * xs.len(),
            "the Q2/Q8 pairing (2 y-blocks per x-block)"
        );
        let mut acc0 = _mm256_setzero_ps();
        let mut acc1 = _mm256_setzero_ps();
        let nb = xs.len();
        let mut i = 0usize;
        while i + 1 < nb {
            acc0 = q2_0_acc_block(&xs[i], &ys[2 * i], &ys[2 * i + 1], acc0);
            acc1 = q2_0_acc_block(&xs[i + 1], &ys[2 * i + 2], &ys[2 * i + 3], acc1);
            i += 2;
        }
        if i < nb {
            acc0 = q2_0_acc_block(&xs[i], &ys[2 * i], &ys[2 * i + 1], acc0);
        }
        hsum8(_mm256_add_ps(acc0, acc1))
    }
}

// ── the counting allocator (G4) ──────────────────────────────────────────

static ALLOC_ARM: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ALLOC_ARM.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ALLOC_ARM.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOC: Counting = Counting;

fn alloc_armed<T>(f: impl FnOnce() -> T) -> (T, usize) {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    ALLOC_ARM.store(true, Ordering::Relaxed);
    let out = f();
    ALLOC_ARM.store(false, Ordering::Relaxed);
    (out, ALLOC_COUNT.load(Ordering::Relaxed))
}

// ── the deterministic fixture generator (identical on every host) ────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    /// xorshift64* — deterministic, no deps.
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// One Q2 block's worth of random fixtures: one Q2 block (packed
/// uniform-random 2-bit codes) + its TWO Q8 blocks (i8 full range). Scales
/// are power-of-two (f16 bits) — the G1 exactness contract's precondition.
fn gen_fixture(
    rng: &mut Rng,
    xd_bits: u16,
    y0d_bits: u16,
    y1d_bits: u16,
) -> (Q2Block, [Q8Block; 2]) {
    let mut x = Q2Block {
        d: xd_bits,
        qs: [0; 16],
    };
    for b in x.qs.iter_mut() {
        *b = (rng.next() & 0xFF) as u8;
    }
    let mut ys = [
        Q8Block {
            d: y0d_bits,
            qs: [0; 32],
        },
        Q8Block {
            d: y1d_bits,
            qs: [0; 32],
        },
    ];
    for y in &mut ys {
        for q in y.qs.iter_mut() {
            *q = (rng.below(256) as i32 - 128) as i8;
        }
    }
    (x, ys)
}

/// Edge fixture: every 2-bit code value exercised (the byte 0b11_10_01_00
/// carries codes {3,2,1,0} LSB-pair-first), q8 at one extreme.
fn edge_fixtures(xd_bits: u16, yd_bits: u16, q8_extreme: i8) -> (Q2Block, [Q8Block; 2]) {
    let x = Q2Block {
        d: xd_bits,
        qs: [
            0b11_10_01_00,
            0b00_01_10_11,
            0b11_10_01_00,
            0b00_01_10_11,
            0b11_10_01_00,
            0b00_01_10_11,
            0b11_10_01_00,
            0b00_01_10_11,
            0b11_10_01_00,
            0b00_01_10_11,
            0b11_10_01_00,
            0b00_01_10_11,
            0b11_10_01_00,
            0b00_01_10_11,
            0b11_10_01_00,
            0b00_01_10_11,
        ],
    };
    let ys = [
        Q8Block {
            d: yd_bits,
            qs: [q8_extreme; 32],
        },
        Q8Block {
            d: yd_bits,
            qs: [q8_extreme; 32],
        },
    ];
    (x, ys)
}

// ── the GEMV (one row = one vec_dot; the sweep's measured unit) ──────────

fn gemv_scalar(rows: &[Vec<Q2Block>], ys: &[Q8Block]) -> f32 {
    let mut acc = 0.0f32;
    for r in rows {
        acc += vec_dot_scalar(r, ys);
    }
    acc
}

#[cfg(target_arch = "x86_64")]
fn gemv_avx2(rows: &[Vec<Q2Block>], ys: &[Q8Block]) -> f32 {
    let mut acc = 0.0f32;
    // SAFETY: avx2+fma runtime-detected before any call (main refuses
    // loudly on a host without them — the i7-13700K carries both).
    for r in rows {
        acc += unsafe { avx2::vec_dot_q2_q8(r, ys) };
    }
    acc
}

// ── provenance (the bench_005 pattern; the Windows sibling gate is the
//    reflex .ps1 — Issue 065 T3(b), the .sh is the unix half) ─────────────

fn preflight() -> (String, String) {
    #[cfg(windows)]
    let (spawn, script) = {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../riir-reflex/scripts/bench_preflight.ps1");
        let ps = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&script)
            .output();
        (ps, script)
    };
    #[cfg(not(windows))]
    let (spawn, script) = {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../riir-reflex/scripts/bench_preflight.sh");
        let sh = Command::new(&script).output();
        (sh, script)
    };
    if !script.exists() {
        return (
            "UNAVAILABLE".to_string(),
            format!(
                "preflight script absent at {} — provenance never skipped",
                script.display()
            ),
        );
    }
    match spawn {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let haystack = if stdout.lines().any(|l| l.starts_with("PROVENANCE:")) {
                stdout
            } else {
                // powershell writes refusals on stderr
                stderr
            };
            let prov = haystack
                .lines()
                .find(|l| l.starts_with("PROVENANCE:"))
                .map(|l| l.to_string())
                .unwrap_or_else(|| {
                    haystack
                        .lines()
                        .find(|l| {
                            l.contains("REFUSED") || l.contains("FAIL") || l.contains("refused")
                        })
                        .unwrap_or("preflight refused (no reason line parsed)")
                        .to_string()
                });
            let verdict = if out.status.success() && prov.starts_with("PROVENANCE:") {
                "PASSED".to_string()
            } else {
                "REFUSED".to_string()
            };
            (verdict, prov)
        }
        Err(e) => (
            "UNAVAILABLE".to_string(),
            format!("preflight spawn failed: {e}"),
        ),
    }
}

fn say(msg: &str) {
    println!("{msg}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

// ── the sweep grid (the same staged spec: 256..4096 K x 32..512 rows) ────

const ROWS_GRID: [usize; 5] = [32, 64, 128, 256, 512];
const K_GRID: [usize; 5] = [256, 512, 1024, 2048, 4096];
/// Best-of-N total-wall passes, interleaved per shape (the sequential-A/B
/// box-noise lesson: contention adds time to SOME runs; the minimum of
/// interleaved pairs separates it).
const MEASURED_PASSES: usize = 5;
const FIXTURE_SEED: u64 = 0x2310_0500_0A31_0007;

/// rows x K Q2-packed matrix + the K-wide Q8 activation vector, random
/// with a fixed seed. K must be a multiple of 64 (2 Q8 blocks per Q2).
struct Sweep {
    rows: Vec<Vec<Q2Block>>,
    ys: Vec<Q8Block>,
}

fn gen_sweep(rows: usize, k: usize, seed_offset: u64) -> Sweep {
    assert!(
        k.is_multiple_of(64),
        "K must be a multiple of 64 (the Q2_0 layout)"
    );
    let mut rng = Rng::new(FIXTURE_SEED ^ seed_offset);
    let n_q2_per_row = k / 64;
    let mut out = Sweep {
        rows: Vec::with_capacity(rows),
        ys: Vec::with_capacity(2 * n_q2_per_row),
    };
    for _ in 0..rows {
        let mut r = Vec::with_capacity(n_q2_per_row);
        for _ in 0..n_q2_per_row {
            let xd = POW2_F16[rng.below(3) as usize]; // {1.0, 2.0, 0.5} x-scales
            let y0d = POW2_F16[rng.below(3) as usize];
            let y1d = POW2_F16[rng.below(3) as usize];
            let (x, ys_pair) = gen_fixture(&mut rng, xd, y0d, y1d);
            r.push(x);
            out.ys.push(ys_pair[0]);
            out.ys.push(ys_pair[1]);
        }
        out.rows.push(r);
    }
    out
}

/// The host gate: x86_64 + avx2 + fma, or a LOUD refusal. On a non-x86_64
/// host every x86 intrinsic compiles to nothing — refuse rather than
/// silently measure a different (scalar) kernel.
fn host_gate() {
    #[cfg(not(target_arch = "x86_64"))]
    {
        say("REFUSED — this bench prices an x86_64-only kernel (AVX2+FMA) and");
        say("every x86 intrinsic compiles to NOTHING on this host. Run it on");
        say("the 4090 (the P53 rider's staged lane). Nothing was measured.");
        std::process::exit(2);
    }
    #[cfg(target_arch = "x86_64")]
    {
        if !std::arch::is_x86_feature_detected!("avx2")
            || !std::arch::is_x86_feature_detected!("fma")
        {
            say("REFUSED — avx2 and/or fma not detected; the maddubs+FMA");
            say("accumulation contract is the rule's mechanism and a fallback");
            say("would measure a DIFFERENT kernel. Nothing was measured.");
            std::process::exit(2);
        }
    }
}

fn main() {
    say("── bench_007: the AVX2 Q2_0 x86 GOAT (the P53 rider, batch 231) ──");

    // ── the host gate: x86_64 + avx2 + fma, or a LOUD refusal ────────────
    // In its own fn so the not-x86_64 arm's exit(2) divergence stays INSIDE
    // it (the bench_005 lesson: inline in main, that arm made every
    // following statement unreachable_code under -D warnings).
    host_gate();

    // ── provenance (before any timing) ───────────────────────────────────
    let (prov_verdict, prov_line) = preflight();
    say(&format!("provenance: {prov_verdict} — {prov_line}"));
    if prov_verdict != "PASSED"
        && std::env::var("AVX2_Q2_BENCH_REQUIRE_PROVENANCE").is_ok_and(|v| v == "1")
    {
        say("GATE POSTURE: provenance not PASSED and AVX2_Q2_BENCH_REQUIRE_PROVENANCE=1 — exit 2");
        std::process::exit(2);
    }
    let citable = prov_verdict == "PASSED";

    // ══ G1 — integer exactness (power-of-two scales, bit-identical) ══
    say("── G1: integer-exact vs the naive per-element scalar reference ──");
    let mut g1_cases = 0usize;
    let mut rng = Rng::new(FIXTURE_SEED);
    // random fixtures across all five power-of-two scale triples (x + both y halves)
    for _ in 0..256 {
        let xd = POW2_F16[rng.below(POW2_F16.len() as u64) as usize];
        let y0d = POW2_F16[rng.below(POW2_F16.len() as u64) as usize];
        let y1d = POW2_F16[rng.below(POW2_F16.len() as u64) as usize];
        let (x, ys_pair) = gen_fixture(&mut rng, xd, y0d, y1d);
        let xs = vec![x];
        let ys = vec![ys_pair[0], ys_pair[1]];
        let s = vec_dot_scalar(&xs, &ys);
        #[cfg(target_arch = "x86_64")]
        {
            let n = unsafe { avx2::vec_dot_q2_q8(&xs, &ys) };
            assert_eq!(
                s.to_bits(),
                n.to_bits(),
                "G1 FAIL: AVX2 {n} != scalar {s} at xd={xd:#06x} y0d={y0d:#06x} y1d={y1d:#06x}"
            );
        }
        let _ = s;
        g1_cases += 1;
    }
    // edge fixtures: every code value, q8 at BOTH extremes, all scale pairs
    for &xd in &POW2_F16 {
        for &yd in &POW2_F16 {
            for &q8 in &[-128i8, 127] {
                let (x, ys_pair) = edge_fixtures(xd, yd, q8);
                let xs = vec![x];
                let ys = vec![ys_pair[0], ys_pair[1]];
                let s = vec_dot_scalar(&xs, &ys);
                #[cfg(target_arch = "x86_64")]
                {
                    let n = unsafe { avx2::vec_dot_q2_q8(&xs, &ys) };
                    assert_eq!(
                        s.to_bits(),
                        n.to_bits(),
                        "G1 FAIL (edge): AVX2 {n} != scalar {s} at xd={xd:#06x} yd={yd:#06x} q8={q8}"
                    );
                }
                let _ = s;
                g1_cases += 1;
            }
        }
    }
    // multi-block rows (the two-chain accumulation + odd-tail shape; the
    // exactness regime is order-free, so chain-vs-sequential accumulation
    // is bit-identical when no addition rounds)
    for &n_blocks in &[2usize, 3, 5, 17, 33] {
        let mut xs = Vec::with_capacity(n_blocks);
        let mut ys: Vec<Q8Block> = Vec::with_capacity(2 * n_blocks);
        for _ in 0..n_blocks {
            let (x, ys_pair) = gen_fixture(&mut rng, 0x3800, 0x3C00, 0x3C00); // 0.5 x, 1.0/1.0 y
            xs.push(x);
            ys.push(ys_pair[0]);
            ys.push(ys_pair[1]);
        }
        let s = vec_dot_scalar(&xs, &ys);
        #[cfg(target_arch = "x86_64")]
        {
            let n = unsafe { avx2::vec_dot_q2_q8(&xs, &ys) };
            assert_eq!(
                s.to_bits(),
                n.to_bits(),
                "G1 FAIL (multi-block n={n_blocks}): AVX2 {n} != scalar {s}"
            );
        }
        let _ = s;
        g1_cases += 1;
    }
    say(&format!(
        "G1 PASS — {g1_cases} cases bit-identical (random + edge + multi-block)"
    ));

    // ══ G4 — allocation-free (the measured path, both arms) ══
    say("── G4: zero allocations in the measured path ──");
    {
        let sweep = gen_sweep(32, 256, 0xC0FF_EE07);
        let row = &sweep.rows[0];
        let ys = &sweep.ys;
        let (_, n_allocs) = alloc_armed(|| vec_dot_scalar(row, ys));
        assert_eq!(
            n_allocs, 0,
            "G4 FAIL: the SCALAR arm allocates ({n_allocs})"
        );
        #[cfg(target_arch = "x86_64")]
        {
            let (_, n_allocs) = alloc_armed(|| unsafe { avx2::vec_dot_q2_q8(row, ys) });
            assert_eq!(n_allocs, 0, "G4 FAIL: the AVX2 arm allocates ({n_allocs})");
        }
    }
    say("G4 PASS — zero allocations, both arms (constants hoisted, no heap in the loop)");

    // ══ G2 — the GEMV sweep, interleaved best-of-N ══
    say("── G2: GEMV sweep (rows x K), AVX2 vs naive scalar ──");
    say(&format!(
        "    (interleaved best-of-{MEASURED_PASSES} per shape per arm; correctness-coupled sinks)"
    ));
    let mut all_avx2_win = true;
    for &rows in &ROWS_GRID {
        for &k in &K_GRID {
            let sweep = gen_sweep(rows, k, (rows as u64) << 32 | k as u64);
            // correctness-coupled sinks: the reference is the SCALAR arm at
            // setup; the AVX2 arm must match it bit-for-bit or the bench
            // FAILS (a deleted loop reports a fast wrong number, never a
            // pass). On non-x86_64 the AVX2 reads are cfg'd out — the
            // binding is dead there BY CONSTRUCTION (host_gate refused
            // above); not a latent defect, just unreachable.
            #[cfg_attr(not(target_arch = "x86_64"), allow(unused_variables))]
            let reference = {
                let (r, a) = alloc_armed(|| gemv_scalar(&sweep.rows, &sweep.ys));
                assert_eq!(a, 0, "G2 setup: scalar GEMV allocated ({a})");
                r
            };
            #[cfg(target_arch = "x86_64")]
            {
                let n = gemv_avx2(&sweep.rows, &sweep.ys);
                assert_eq!(
                    reference.to_bits(),
                    n.to_bits(),
                    "G2 sink mismatch at rows={rows} k={k}: scalar {reference} vs AVX2 {n}"
                );
            }
            // interleaved best-of-N (min per arm)
            let mut best_avx2 = f64::INFINITY;
            let mut best_scalar = f64::INFINITY;
            for _ in 0..MEASURED_PASSES {
                let t0 = Instant::now();
                #[cfg(target_arch = "x86_64")]
                {
                    let sink = gemv_avx2(&sweep.rows, &sweep.ys);
                    std::hint::black_box(sink);
                }
                best_avx2 = best_avx2.min(t0.elapsed().as_secs_f64());
                let t0 = Instant::now();
                let sink = gemv_scalar(&sweep.rows, &sweep.ys);
                std::hint::black_box(sink);
                best_scalar = best_scalar.min(t0.elapsed().as_secs_f64());
            }
            let speedup = best_scalar / best_avx2;
            if best_avx2 >= best_scalar {
                all_avx2_win = false;
            }
            say(&format!(
                "  rows={rows:>3} k={k:>4}  avx2={:>9.3} ms  scalar={:>9.3} ms  speedup={speedup:>6.2}x{}",
                best_avx2 * 1e3,
                best_scalar * 1e3,
                if citable { "" } else { "  (PROVISIONAL)" }
            ));
        }
    }

    // ══ the verdict (the rust_perf Pareto validator, bespoke) ══
    say("── VERDICT ──");
    if all_avx2_win {
        say(&format!(
            "PARETO-PASS — G1 exact, G4 zero-alloc, AVX2 strictly faster at every one of {} shapes{}",
            ROWS_GRID.len() * K_GRID.len(),
            if citable {
                ""
            } else {
                " (TIMING PROVISIONAL — quote the PROVENANCE line)"
            }
        ));
        std::process::exit(0);
    } else {
        say(
            "PARETO-FAIL — the AVX2 arm lost or tied at least one shape (a real measured outcome; record it)",
        );
        std::process::exit(1);
    }
}
