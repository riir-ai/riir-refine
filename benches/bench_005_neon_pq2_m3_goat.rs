//! Bench 005 — Plan 204 T11: the M3-Max GOAT for the rust_perf rule
//! `neon-tbl-replicate-2bit-codec-sdot-block-rescale` (Batch 204, the
//! PrismML-Eng/llama.cpp #265 NEON PQ2_0 vec_dot distill).
//!
//! The rule is NotBenchable on the 4090 host (x86_64 compiles every
//! aarch64 intrinsic to nothing), so its GOAT runs HERE — aarch64 only,
//! loud refusal anywhere else. Three gates, the staged spec verbatim:
//!
//! 1. **G1 (integer-exactness)** — power-of-two-scale vectors: the NEON
//!    packed kernel is BIT-IDENTICAL to the dequant-then-scalar-dot
//!    reference, and the reference is the NAIVE per-element codec form
//!    (the rule's buggy_code — the B62 bench-design law: comparing
//!    against a transposed-lane multi-accumulator scalar form measures
//!    LAYOUT, not codec). A panic here fails the gate permanently; it is
//!    a correctness claim, not a timing claim.
//! 2. **G2 (GEMV sweep, provenance-gated)** — rows {32..512} x K
//!    {256..4096}, NEON vs the naive scalar, interleaved best-of-N
//!    passes (contention adds time to SOME runs; the minimum separates
//!    it), correctness-coupled sinks (each arm's GEMV sum must equal the
//!    scalar reference computed at setup, so an optimiser deleting the
//!    measured loop FAILS instead of reporting a fast zero — the
//!    timed-region loud-zero doctrine). Every citable number quotes the
//!    `bench_preflight.sh` PROVENANCE line; `NEON_PQ2_BENCH_REQUIRE_
//!    PROVENANCE=1` is the exit-2 gate posture for the citable run.
//! 3. **G4 (allocation-free)** — a counting global allocator armed around
//!    the measured vec_dot path: zero allocations, by construction (the
//!    fixed idx/shift/mask tables hoist; nothing in the loop allocs).
//!
//! Verdict shape (the rust_perf Pareto validator, bespoke): G1 pass AND
//! NEON strictly faster at EVERY sweep shape AND zero allocs both arms =
//! `PARETO-PASS` (exit 0). Any other measured outcome = `PARETO-FAIL`
//! (exit 1) — a failed GOAT is a real outcome, recorded as such.
//!
//! ```bash
//! # the whole gate (G1 + G4 + provisional-labeled G2):
//! cargo bench --bench bench_005_neon_pq2_m3_goat
//! # the GATE posture (exit 2 unless preflight PASSED — the citable run):
//! NEON_PQ2_BENCH_REQUIRE_PROVENANCE=1 cargo bench --bench bench_005_neon_pq2_m3_goat
//! ```

#![allow(clippy::pedantic)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

// ── the codec (the rule's own shapes; fixtures ORIGINAL — the C was never
//    pasted, this is the rule's pseudocode-level transcription) ───────────

/// PQ2_0 block: 128 weights x 2 bits + one f16 scale. The scale rides as
/// f16 BITS — std `f16` is still unstable on the pinned toolchain (the
/// corpus rule text is string data, so it never had to compile); the
/// fixtures use power-of-two normals (exactly representable) decoded by
/// [`f16_bits_to_f32`].
#[derive(Clone, Copy)]
pub struct Pq2Block {
    pub d: u16,       // f16 bit pattern
    pub qs: [u8; 32], // 128 weights x 2 bits, LSB pair first
}

/// Q8_K block: 256 int8 activations + one f32 scale; TWO Pq2 blocks (the
/// two 128-weight halves) share one Q8K block.
#[derive(Clone, Copy)]
pub struct Q8KBlock {
    pub d: f32,
    pub qs: [i8; 256],
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

pub fn vec_dot_scalar(xs: &[Pq2Block], ys: &[Q8KBlock]) -> f32 {
    let mut sumf = 0.0f32;
    for (i, x) in xs.iter().enumerate() {
        let yb = &ys[i >> 1];
        let q8 = &yb.qs[128 * (i & 1)..];
        let mut acc = 0i32;
        for (k, &byte) in x.qs.iter().enumerate() {
            for j in 0..4 {
                let code = (byte >> (2 * j)) & 0x03; // one code at a time
                acc += (code as i32 - 1) * q8[4 * k + j] as i32;
            }
        }
        sumf += f16_bits_to_f32(x.d) * yb.d * acc as f32;
    }
    sumf
}

// ── the NEON packed kernel (the rule's fixed_code form, with the dotprod
//    target_feature the sdot intrinsics require; Rust names ACLE's
//    vsdotq_s32 `vdotq_s32`) ───────────────────────────────────────────────

#[cfg(target_arch = "aarch64")]
pub mod neon {
    use super::{f16_bits_to_f32, Pq2Block, Q8KBlock};
    use std::arch::aarch64::*;

    /// sdot (v8.2 asimddp — NOT i8mm) requires the dotprod feature at the
    /// call site; the caller MUST runtime-detect first (M-series has it).
    /// # Safety
    ///
    /// The caller MUST have runtime-detected `dotprod` (v8.2 asimddp) —
    /// `std::arch::is_aarch64_feature_detected!("dotprod")` — before calling;
    /// the sdot accumulation contract has no scalar fallback by design (a
    /// fallback would measure a DIFFERENT kernel). Only valid on aarch64.
    #[target_feature(enable = "dotprod")]
    pub unsafe fn vec_dot_pq2_q8k(xs: &[Pq2Block], ys: &[Q8KBlock]) -> f32 {
        let idx_lo: [u8; 16] = [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3];
        let idx_hi: [u8; 16] = [4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7];
        let shifts: [i8; 16] = [0, -2, -4, -6, 0, -2, -4, -6, 0, -2, -4, -6, 0, -2, -4, -6];
        let (t_lo, t_hi, sh) = unsafe {
            (
                vld1q_u8(idx_lo.as_ptr()),
                vld1q_u8(idx_hi.as_ptr()),
                vld1q_s8(shifts.as_ptr()),
            )
        };
        let (m2, one) = (vdupq_n_u8(0x03), vdupq_n_s8(1));
        let mut sumf = 0.0f32;
        for (i, x) in xs.iter().enumerate() {
            let yb = &ys[i >> 1];
            let q8 = &yb.qs[128 * (i & 1)..];
            let mut acc = vdupq_n_s32(0);
            for k in 0..4 {
                let (qv0, qv1) = unsafe {
                    let raw = vld1_u8(x.qs[8 * k..].as_ptr());
                    let raw16 = vcombine_u8(raw, raw); // one replication, no stores
                    let b0 = vqtbl1q_u8(raw16, t_lo);
                    let b1 = vqtbl1q_u8(raw16, t_hi);
                    let d0 = vsubq_s8(vreinterpretq_s8_u8(vandq_u8(vshlq_u8(b0, sh), m2)), one);
                    let d1 = vsubq_s8(vreinterpretq_s8_u8(vandq_u8(vshlq_u8(b1, sh), m2)), one);
                    (d0, d1)
                };
                let (y0, y1) = unsafe {
                    (
                        vld1q_s8(q8[32 * k..].as_ptr()),
                        vld1q_s8(q8[32 * k + 16..].as_ptr()),
                    )
                };
                acc = vdotq_s32(acc, qv0, y0); // exact int32 lane sums
                acc = vdotq_s32(acc, qv1, y1);
            }
            let block_dot = vaddvq_s32(acc) as f32;
            sumf += f16_bits_to_f32(x.d) * yb.d * block_dot; // ONE rescale per block
        }
        sumf
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

/// One Q8K block's worth of random fixtures: two Pq2 blocks (packed
/// uniform-random 2-bit codes) + one Q8K block (i8 full range). Scales are
/// power-of-two (x: f16 bits from the table, y: f32) — the G1 exactness
/// contract's precondition.
fn gen_fixture(rng: &mut Rng, xd_bits: u16, yd: f32) -> ([Pq2Block; 2], Q8KBlock) {
    let mut xs = [
        Pq2Block {
            d: xd_bits,
            qs: [0; 32],
        },
        Pq2Block {
            d: xd_bits,
            qs: [0; 32],
        },
    ];
    for x in &mut xs {
        for b in x.qs.iter_mut() {
            *b = (rng.next() & 0xFF) as u8;
        }
    }
    let mut yb = Q8KBlock {
        d: yd,
        qs: [0; 256],
    };
    for q in yb.qs.iter_mut() {
        *q = (rng.below(256) as i32 - 128) as i8;
    }
    (xs, yb)
}

/// Edge fixture: every 2-bit code value exercised (the byte 0b11_10_01_00
/// carries codes {3,2,1,0} LSB-pair-first), q8 at one extreme.
fn edge_fixtures(xd_bits: u16, yd: f32, q8_extreme: i8) -> ([Pq2Block; 2], Q8KBlock) {
    let xs = [
        Pq2Block {
            d: xd_bits,
            qs: [0b11_10_01_00; 32],
        },
        Pq2Block {
            d: xd_bits,
            qs: [0b00_01_10_11; 32],
        }, // reversed pairs
    ];
    let yb = Q8KBlock {
        d: yd,
        qs: [q8_extreme; 256],
    };
    (xs, yb)
}

// ── the GEMV (one row = one vec_dot; the sweep's measured unit) ──────────

fn gemv_scalar(rows: &[Vec<Pq2Block>], ys: &[Q8KBlock]) -> f32 {
    let mut acc = 0.0f32;
    for r in rows {
        acc += vec_dot_scalar(r, ys);
    }
    acc
}

#[cfg(target_arch = "aarch64")]
fn gemv_neon(rows: &[Vec<Pq2Block>], ys: &[Q8KBlock]) -> f32 {
    let mut acc = 0.0f32;
    // SAFETY: dotprod runtime-detected before any call (main refuses
    // loudly on a host without it — M-series Max chips carry it).
    for r in rows {
        acc += unsafe { neon::vec_dot_pq2_q8k(r, ys) };
    }
    acc
}

// ── provenance (the bench_004 pattern — the instrument is cross-repo) ────

fn preflight() -> (String, String) {
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../riir-reflex/scripts/bench_preflight.sh");
    if !script.exists() {
        return (
            "UNAVAILABLE".to_string(),
            format!(
                "preflight script absent at {} — provenance never skipped",
                script.display()
            ),
        );
    }
    match Command::new(&script).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let prov = stdout
                .lines()
                .find(|l| l.starts_with("PROVENANCE:"))
                .map(|l| l.to_string())
                .unwrap_or_else(|| {
                    stdout
                        .lines()
                        .find(|l| l.contains("REFUSED") || l.contains("FAIL"))
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

// ── the sweep grid (the staged spec: 256..4096 K x 32..512 rows) ─────────

const ROWS_GRID: [usize; 5] = [32, 64, 128, 256, 512];
const K_GRID: [usize; 5] = [256, 512, 1024, 2048, 4096];
/// Best-of-N total-wall passes, interleaved per shape (the sequential-A/B
/// box-noise lesson: contention adds time to SOME runs; the minimum of
/// interleaved pairs separates it).
const MEASURED_PASSES: usize = 5;
const FIXTURE_SEED: u64 = 0x2040_0500_6B00_0001;

/// rows x K PQ2-packed matrix + the K-wide Q8_K activation vector, random
/// with a fixed seed. K must be a multiple of 256 (2 Pq2 blocks per Q8K).
struct Sweep {
    rows: Vec<Vec<Pq2Block>>,
    ys: Vec<Q8KBlock>,
}

fn gen_sweep(rows: usize, k: usize, seed_offset: u64) -> Sweep {
    assert!(
        k.is_multiple_of(256),
        "K must be a multiple of 256 (the Q8_K layout)"
    );
    let mut rng = Rng::new(FIXTURE_SEED ^ seed_offset);
    let n_pq2_per_row = k / 128;
    let n_q8k = k / 256;
    let mut out = Sweep {
        rows: Vec::with_capacity(rows),
        ys: Vec::with_capacity(n_q8k),
    };
    for _ in 0..rows {
        let mut r = Vec::with_capacity(n_pq2_per_row);
        for _ in 0..(n_pq2_per_row / 2) {
            let xd = POW2_F16[rng.below(3) as usize]; // {1.0, 2.0, 0.5} x-scales
            let yd = 2.0f32.powi(rng.below(3) as i32 - 1); // {0.5, 1.0, 2.0}
            let (pair, _) = gen_fixture(&mut rng, xd, yd);
            r.push(pair[0]);
            r.push(pair[1]);
        }
        out.rows.push(r);
    }
    for _ in 0..n_q8k {
        let yd = 2.0f32.powi(rng.below(3) as i32 - 1);
        let (_, yb) = gen_fixture(&mut rng, POW2_F16[0], yd);
        out.ys.push(yb);
    }
    out
}

/// The host gate: aarch64 + dotprod, or a LOUD refusal. On a non-aarch64
/// host every aarch64 intrinsic compiles to nothing — refuse rather than
/// silently measure a different (scalar) kernel.
fn host_gate() {
    #[cfg(not(target_arch = "aarch64"))]
    {
        say("REFUSED — this bench prices an aarch64-only kernel (NEON/sdot) and");
        say("every aarch64 intrinsic compiles to NOTHING on this host. Run it on");
        say("the M3 (the Plan-204 T11 lane). Nothing was measured.");
        std::process::exit(2);
    }
    #[cfg(target_arch = "aarch64")]
    {
        if !std::arch::is_aarch64_feature_detected!("dotprod") {
            say("REFUSED — dotprod (v8.2 asimddp) not detected; sdot is the rule's");
            say("accumulation contract and a fallback would measure a DIFFERENT");
            say("kernel. Nothing was measured.");
            std::process::exit(2);
        }
    }
}

fn main() {
    say("── bench_005: the NEON PQ2_0 M3-Max GOAT (Plan 204 T11) ──");

    // ── the host gate: aarch64 + dotprod, or a LOUD refusal ──────────────
    // In its own fn so the not-aarch64 arm's exit(2) divergence stays INSIDE
    // it — inline in main, that arm made every following statement
    // unreachable_code on an x86_64 host under -D warnings (found by the
    // Plan 202 R4 gate triad, 2026-10-08). Refusal text byte-identical.
    host_gate();

    // ── provenance (before any timing) ───────────────────────────────────
    let (prov_verdict, prov_line) = preflight();
    say(&format!("provenance: {prov_verdict} — {prov_line}"));
    if prov_verdict != "PASSED"
        && std::env::var("NEON_PQ2_BENCH_REQUIRE_PROVENANCE").is_ok_and(|v| v == "1")
    {
        say("GATE POSTURE: provenance not PASSED and NEON_PQ2_BENCH_REQUIRE_PROVENANCE=1 — exit 2");
        std::process::exit(2);
    }
    let citable = prov_verdict == "PASSED";

    // ══ G1 — integer exactness (power-of-two scales, bit-identical) ══
    say("── G1: integer-exact vs the naive per-element scalar reference ──");
    let mut g1_cases = 0usize;
    let mut rng = Rng::new(FIXTURE_SEED);
    // random fixtures across all five power-of-two scale pairs (both axes)
    for _ in 0..256 {
        let xd = POW2_F16[rng.below(POW2_F16.len() as u64) as usize];
        let yd = 2.0f32.powi(rng.below(5) as i32 - 2); // {0.25..4.0} pow2
        let (xs, ys_single) = gen_fixture(&mut rng, xd, yd);
        let ys = vec![ys_single];
        let s = vec_dot_scalar(&xs, &ys);
        #[cfg(target_arch = "aarch64")]
        {
            let n = unsafe { neon::vec_dot_pq2_q8k(&xs, &ys) };
            assert_eq!(
                s.to_bits(),
                n.to_bits(),
                "G1 FAIL: NEON {n} != scalar {s} at xd={xd:#06x} yd={yd}"
            );
        }
        let _ = s;
        g1_cases += 1;
    }
    // edge fixtures: every code value, q8 at BOTH extremes, all scale pairs
    for &xd in &POW2_F16 {
        for yi in 0..5 {
            let yd = 2.0f32.powi(yi - 2);
            for &q8 in &[-128i8, 127] {
                let (xs, ys_single) = edge_fixtures(xd, yd, q8);
                let ys = vec![ys_single];
                let s = vec_dot_scalar(&xs, &ys);
                #[cfg(target_arch = "aarch64")]
                {
                    let n = unsafe { neon::vec_dot_pq2_q8k(&xs, &ys) };
                    assert_eq!(
                        s.to_bits(),
                        n.to_bits(),
                        "G1 FAIL (edge): NEON {n} != scalar {s} at xd={xd:#06x} yd={yd} q8={q8}"
                    );
                }
                let _ = s;
                g1_cases += 1;
            }
        }
    }
    // multi-block rows (the sumf accumulation chain; the odd tail exercises
    // a Q8K block whose SECOND half is never indexed — both arms see the
    // same memory, so exactness holds regardless)
    for &n_blocks in &[2usize, 3, 5, 17, 33] {
        let mut xs = Vec::with_capacity(n_blocks);
        let mut ys = Vec::with_capacity(n_blocks.div_ceil(2));
        while xs.len() < n_blocks {
            let want = n_blocks - xs.len();
            let (pair, yb) = gen_fixture(&mut rng, 0x3800, 1.0);
            xs.push(pair[0]);
            if want > 1 {
                xs.push(pair[1]);
            }
            ys.push(yb);
        }
        let s = vec_dot_scalar(&xs, &ys);
        #[cfg(target_arch = "aarch64")]
        {
            let n = unsafe { neon::vec_dot_pq2_q8k(&xs, &ys) };
            assert_eq!(
                s.to_bits(),
                n.to_bits(),
                "G1 FAIL (multi-block n={n_blocks}): NEON {n} != scalar {s}"
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
        let sweep = gen_sweep(32, 256, 0xC0FFEE);
        let row = &sweep.rows[0];
        let ys = &sweep.ys;
        let (_, n_allocs) = alloc_armed(|| vec_dot_scalar(row, ys));
        assert_eq!(
            n_allocs, 0,
            "G4 FAIL: the SCALAR arm allocates ({n_allocs})"
        );
        #[cfg(target_arch = "aarch64")]
        {
            let (_, n_allocs) = alloc_armed(|| unsafe { neon::vec_dot_pq2_q8k(row, ys) });
            assert_eq!(n_allocs, 0, "G4 FAIL: the NEON arm allocates ({n_allocs})");
        }
    }
    say("G4 PASS — zero allocations, both arms (tables hoisted, no heap in the loop)");

    // ══ G2 — the GEMV sweep, interleaved best-of-N ══
    say("── G2: GEMV sweep (rows x K), NEON vs naive scalar ──");
    say(&format!(
        "    (interleaved best-of-{MEASURED_PASSES} per shape per arm; correctness-coupled sinks)"
    ));
    let mut all_neon_win = true;
    for &rows in &ROWS_GRID {
        for &k in &K_GRID {
            let sweep = gen_sweep(rows, k, (rows as u64) << 32 | k as u64);
            // correctness-coupled sinks: the reference is the SCALAR arm at
            // setup; the NEON arm must match it bit-for-bit or the bench
            // FAILS (a deleted loop reports a fast wrong number, never a pass).
            // On x86_64 the NEON reads are cfg'd out — the binding is dead
            // there BY CONSTRUCTION (host_gate refused above); not a latent
            // defect, just unreachable. Same Plan 202 R4 gate-triad find.
            #[cfg_attr(not(target_arch = "aarch64"), allow(unused_variables))]
            let reference = {
                let (r, a) = alloc_armed(|| gemv_scalar(&sweep.rows, &sweep.ys));
                assert_eq!(a, 0, "G2 setup: scalar GEMV allocated ({a})");
                r
            };
            #[cfg(target_arch = "aarch64")]
            {
                let n = gemv_neon(&sweep.rows, &sweep.ys);
                assert_eq!(
                    reference.to_bits(),
                    n.to_bits(),
                    "G2 sink mismatch at rows={rows} k={k}: scalar {reference} vs NEON {n}"
                );
            }
            // interleaved best-of-N (min per arm)
            let mut best_neon = f64::INFINITY;
            let mut best_scalar = f64::INFINITY;
            for _ in 0..MEASURED_PASSES {
                let t0 = Instant::now();
                #[cfg(target_arch = "aarch64")]
                {
                    let sink = gemv_neon(&sweep.rows, &sweep.ys);
                    std::hint::black_box(sink);
                }
                best_neon = best_neon.min(t0.elapsed().as_secs_f64());
                let t0 = Instant::now();
                let sink = gemv_scalar(&sweep.rows, &sweep.ys);
                std::hint::black_box(sink);
                best_scalar = best_scalar.min(t0.elapsed().as_secs_f64());
            }
            let speedup = best_scalar / best_neon;
            if best_neon >= best_scalar {
                all_neon_win = false;
            }
            say(&format!(
                "  rows={rows:>3} k={k:>4}  neon={:>9.3} ms  scalar={:>9.3} ms  speedup={speedup:>6.2}x{}",
                best_neon * 1e3,
                best_scalar * 1e3,
                if citable { "" } else { "  (PROVISIONAL)" }
            ));
        }
    }

    // ══ the verdict (the rust_perf Pareto validator, bespoke) ══
    say("── VERDICT ──");
    if all_neon_win {
        say(&format!(
            "PARETO-PASS — G1 exact, G4 zero-alloc, NEON strictly faster at every one of {} shapes{}",
            ROWS_GRID.len() * K_GRID.len(),
            if citable { "" } else { " (TIMING PROVISIONAL — quote the PROVENANCE line)" }
        ));
        std::process::exit(0);
    } else {
        say("PARETO-FAIL — the NEON arm lost or tied at least one shape (a real measured outcome; record it)");
        std::process::exit(1);
    }
}
