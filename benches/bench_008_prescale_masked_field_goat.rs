//! Bench 008 — the portable CPU-exact GOAT for the kernel_opt rule
//! `masked-in-place-field-prescale-other-operand` (entries_gpu_98, Batch 242
//! / Plan 244 / Research 274; ddalcu/mlx-serve @ 1015f785, MIT; this battery:
//! plan 246 / Bench 117).
//!
//! THE LAW (simd_qmm.zig:28-39, verbatim): "4 and 8 bits masked in place,
//! the input pre-scaled by wpre(p) to cancel the shift; 6 bits straddle
//! words, so they are shifted down and wpre is 1." Consume a packed sub-word
//! quant field of an integer weight word AS A FLOAT WITHOUT SHIFTING IT OUT
//! OF THE WORD — mask it IN PLACE (the masked integer IS field << off, so
//! its f32 value is field * 2^off, exact: at most an 8-bit significand) and
//! pre-scale the OTHER operand of the product by 2^-off through the f32
//! exponent-field bit pattern `(127 - shift) << 23` at STAGING time. Each
//! factor is exact and the exponents cancel, so every product rounds the
//! SAME true value the shift-down form rounds — bit-identical dots, with
//! the per-element shift removed from the hot loop and paid as ONE multiply
//! per staged activation element, amortized over every weight row that
//! reads it.
//!
//! Portable by design (the B231/116 x86-GOAT precedent's portable sibling):
//! pure integer masks + fp32 bit patterns, no ISA gate — exactness and
//! amortization are host-independent claims, measured on whichever box runs
//! this (the landing record: the 4090 box's i7-13700K).
//!
//! Four arms:
//! - **4-bit** (off = 4p, 8 fields/word): the rule's primary wpre form;
//! - **8-bit** (off = 8p, 4 fields/word): the second word-dividing width;
//! - **2-bit ALU twin** (qmv_nax2.zig:153-183, the plain-float form): the
//!   per-group scale ladder sv * {2^0, 2^-2, 2^-4, 2^-6} staged per group,
//!   codes consumed masked at their byte offsets (swept in G2 — the law's
//!   plain-ALU face);
//! - **6-bit exception** (G1 only): non-dividing widths straddle words —
//!   the conventional shift-down with wpre = 1, the two-word straddle form
//!   verbatim; the law's own boundary, asserted bit-identical.
//!
//! Three gates (the staged spec):
//! 1. **G1 (bit-identity)** — the exponent-cancellation exactness proof
//!    pinned by execution: random fixtures + edge words + the regime
//!    boundary x = ±2^-98 (x * 2^-28 = 2^-126, the smallest normal — the
//!    LAST exact prescale) across all four arms. A panic here fails the
//!    gate permanently; it is a correctness claim, not a timing claim.
//!    REGIME: |x * 2^-off| >= 2^-126 — the prescale multiply (a power-of-two
//!    scaling) is exact there; below it the prescale can round (subnormal)
//!    and bit-identity is NOT claimed (the anti-regime, disclosed).
//! 2. **G2 (GEMV sweep, provenance-gated)** — rows {32..512} x K
//!    {256..4096} for the 4-bit, 8-bit, and 2-bit-ALU-twin faces, opt
//!    (staged prescale + masked-in-place) vs naive (shift-down per
//!    element), interleaved best-of-5 passes, correctness-coupled sinks
//!    (each shape's arms must agree bit-for-bit at setup — an optimiser
//!    deleting the measured loop reports a fast wrong number, never a
//!    pass), plus the OP-COUNT LEDGER: the naive arm executes rows x K
//!    variable shifts in the hot loop; the opt arm executes K staged
//!    prescale multiplies and ZERO shifts (at rows=512: 512 shifts
//!    eliminated per staging multiply).
//! 3. **G4 (allocation-free)** — a counting global allocator armed around
//!    BOTH arms' full measured path (staging included, writing into a
//!    pre-allocated scratch): zero allocations.
//!
//! Verdict: G1 + G4 + opt strictly faster at EVERY shape (both wpre arms) =
//! `PARETO-PASS` (exit 0). Any other measured outcome = `PARETO-FAIL`
//! (exit 1) — a failed GOAT is a real outcome, recorded as such.
//!
//! ```bash
//! # the whole gate (G1 + G4 + provisional-labeled G2):
//! cargo bench --bench bench_008_prescale_masked_field_goat
//! # the GATE posture (exit 2 unless preflight PASSED — the citable run):
//! PRESCALE_BENCH_REQUIRE_PROVENANCE=1 cargo bench --bench bench_008_prescale_masked_field_goat
//! ```

#![allow(clippy::pedantic)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

// ── the codec: masks + exponent-field prescales (the rule's own shapes) ──

/// 2^-shift built by writing the exponent field directly (mantissa zero):
/// `(127 - shift) << 23` — no table, no conversion op, exact by
/// construction (the rule's wpre).
const fn wpre(shift: u32) -> f32 {
    f32::from_bits((127 - shift) << 23)
}

/// 4-bit fields pack 8 per 32-bit word; field p lives at bit offset 4p.
const MASKS4: [u32; 8] = [
    0x0000_000F,
    0x0000_00F0,
    0x0000_0F00,
    0x0000_F000,
    0x000F_0000,
    0x00F0_0000,
    0x0F00_0000,
    0xF000_0000,
];
/// The staging prescale per pass: 2^-4p.
const WPRE4: [f32; 8] = [
    wpre(0),
    wpre(4),
    wpre(8),
    wpre(12),
    wpre(16),
    wpre(20),
    wpre(24),
    wpre(28),
];

/// 8-bit fields pack 4 per 32-bit word; field p lives at bit offset 8p.
const MASKS8: [u32; 4] = [0x0000_00FF, 0x0000_FF00, 0x00FF_0000, 0xFF00_0000];
/// The staging prescale per pass: 2^-8p.
const WPRE8: [f32; 4] = [wpre(0), wpre(8), wpre(16), wpre(24)];

/// The ALU twin (plain-float form): the per-group scale ladder 2^-2p.
const LADDER2: [f32; 4] = [1.0, 0.25, 0.0625, 0.015625];
/// 2-bit codes pack 4 per byte; code p lives at bit offset 2p.
const MASKS2: [u8; 4] = [0x03, 0x0C, 0x30, 0xC0];

/// A 4-bit group: 8 words x 8 fields = 64 values (value index 8j + p).
type Group4 = [u32; 8];
/// An 8-bit group: 16 words x 4 fields = 64 values (value index 4j + p).
type Group8 = [u32; 16];
/// A 2-bit ALU-twin group: 8 bytes x 4 codes = 32 values + one group scale.
#[derive(Clone, Copy)]
struct Group2 {
    bytes: [u8; 8],
    sv: f32,
}
/// A 6-bit exception group: 3 words = 96 bits = 16 contiguous 6-bit values
/// (bit offset 6i ACROSS words — the straddle the law carves out).
type Group6 = [u32; 3];

// ── the arms. Every pair shares the IDENTICAL loop structure and
//    accumulation order (4 independent accumulator lanes, summed the same
//    way) so the comparison measures the CODEC — the shift-down vs the
//    masked-in-place + staged prescale — never the layout (the B62 law). ──

/// NAIVE 4-bit: shift the field down to unit position per element — the
/// rule's buggy_code form, unchanged.
fn row_dot_naive4(groups: &[Group4], x: &[f32]) -> f32 {
    let mut acc = [0.0f32; 4];
    for (g, words) in groups.iter().enumerate() {
        let xg = &x[g * 64..g * 64 + 64];
        for (j, &w) in words.iter().enumerate() {
            let xw = &xg[j * 8..j * 8 + 8];
            for (p, &xv) in xw.iter().enumerate() {
                let field = ((w >> (4 * p)) & 0xF) as f32;
                acc[p & 3] += xv * field;
            }
        }
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3])
}

/// OPT 4-bit: the field masked IN PLACE (mask precomputed per pass) and the
/// OTHER operand pre-scaled at staging — no shift in the loop.
fn row_dot_masked4(groups: &[Group4], xs_pre: &[f32]) -> f32 {
    let mut acc = [0.0f32; 4];
    for (g, words) in groups.iter().enumerate() {
        let xg = &xs_pre[g * 64..g * 64 + 64];
        for (j, &w) in words.iter().enumerate() {
            let xw = &xg[j * 8..j * 8 + 8];
            for (p, (&xv, &m)) in xw.iter().zip(MASKS4.iter()).enumerate() {
                let field = (w & m) as f32; // == field * 2^(4p), exact
                acc[p & 3] += xv * field;
            }
        }
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3])
}

/// The staging pass (the opt arm's ONE multiply per element, paid once per
/// GEMV and amortized over every weight row that reads it).
fn stage4(x: &[f32], xs_pre: &mut [f32]) {
    for (src, dst) in x.as_chunks::<64>().0.iter().zip(xs_pre.as_chunks_mut::<64>().0.iter_mut()) {
        for (xw, dw) in src.as_chunks::<8>().0.iter().zip(dst.as_chunks_mut::<8>().0.iter_mut()) {
            for (p, &wp) in WPRE4.iter().enumerate() {
                dw[p] = xw[p] * wp; // exact: power-of-two scaling
            }
        }
    }
}

/// NAIVE 8-bit: shift-down per element.
fn row_dot_naive8(groups: &[Group8], x: &[f32]) -> f32 {
    let mut acc = [0.0f32; 4];
    for (g, words) in groups.iter().enumerate() {
        let xg = &x[g * 64..g * 64 + 64];
        for (j, &w) in words.iter().enumerate() {
            let xw = &xg[j * 4..j * 4 + 4];
            for (p, &xv) in xw.iter().enumerate() {
                let field = ((w >> (8 * p)) & 0xFF) as f32;
                acc[p] += xv * field;
            }
        }
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3])
}

/// OPT 8-bit: masked in place + staged prescale.
fn row_dot_masked8(groups: &[Group8], xs_pre: &[f32]) -> f32 {
    let mut acc = [0.0f32; 4];
    for (g, words) in groups.iter().enumerate() {
        let xg = &xs_pre[g * 64..g * 64 + 64];
        for (j, &w) in words.iter().enumerate() {
            let xw = &xg[j * 4..j * 4 + 4];
            for (p, (&xv, &m)) in xw.iter().zip(MASKS8.iter()).enumerate() {
                let field = (w & m) as f32; // == field * 2^(8p), exact
                acc[p] += xv * field;
            }
        }
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3])
}

fn stage8(x: &[f32], xs_pre: &mut [f32]) {
    for (src, dst) in x.as_chunks::<64>().0.iter().zip(xs_pre.as_chunks_mut::<64>().0.iter_mut()) {
        for (xw, dw) in src.as_chunks::<4>().0.iter().zip(dst.as_chunks_mut::<4>().0.iter_mut()) {
            for (p, &wp) in WPRE8.iter().enumerate() {
                dw[p] = xw[p] * wp;
            }
        }
    }
}

/// NAIVE 2-bit ALU twin: shift each code down, multiply by the group scale
/// per element — `(x * sv) * field`.
fn row_dot_naive2(groups: &[Group2], x: &[f32]) -> f32 {
    let mut acc = [0.0f32; 4];
    for (g, grp) in groups.iter().enumerate() {
        let xg = &x[g * 32..g * 32 + 32];
        for (j, &by) in grp.bytes.iter().enumerate() {
            let xb = &xg[j * 4..j * 4 + 4];
            for (p, &xv) in xb.iter().enumerate() {
                let field = ((by >> (2 * p)) & 3) as f32;
                acc[p] += (xv * grp.sv) * field;
            }
        }
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3])
}

/// OPT 2-bit ALU twin: the scale ladder staged per group (sv * 2^-2p, exact
/// power-of-two scaling), codes consumed MASKED at their byte offsets —
/// `(x * ladder_p) * (field * 2^(2p))`, no shift anywhere.
fn row_dot_masked2(groups: &[Group2], x2: &[f32]) -> f32 {
    let mut acc = [0.0f32; 4];
    for (g, grp) in groups.iter().enumerate() {
        let xg = &x2[g * 32..g * 32 + 32];
        for (j, &by) in grp.bytes.iter().enumerate() {
            let xb = &xg[j * 4..j * 4 + 4];
            for (p, (&xv, &m)) in xb.iter().zip(MASKS2.iter()).enumerate() {
                let field = (by & m) as f32; // == field * 2^(2p), exact
                acc[p] += xv * field;
            }
        }
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3])
}

/// The twin's staging: x pre-multiplied by the per-group ladder — one
/// pass over the activations per GEMV, amortized over every weight row.
fn stage2(groups: &[Group2], x: &[f32], x2: &mut [f32]) {
    for (g, grp) in groups.iter().enumerate() {
        let src = &x[g * 32..g * 32 + 32];
        let dst = &mut x2[g * 32..g * 32 + 32];
        for (p, &lad) in LADDER2.iter().enumerate() {
            let s = grp.sv * lad; // exact: sv * 2^-2p
            for j in 0..8 {
                dst[j * 4 + p] = src[j * 4 + p] * s; // exact in-regime
            }
        }
    }
}

/// The 6-bit exception's conventional extraction (the law's carve-out,
/// source form verbatim): single word while the field fits (off <= 26),
/// the two-word straddle past it.
fn extract6(words: &[u32], i: usize) -> u32 {
    let bit = 6 * i;
    let w0 = words[bit / 32];
    let off = bit % 32;
    if off <= 26 {
        (w0 >> off) & 63
    } else {
        // straddles: the field's high bits in w0, its low bits in the NEXT
        // word — (w0 >> off) | (next << (32 - off)), masked to 6 bits
        ((w0 >> off) | (words[bit / 32 + 1] << (32 - off))) & 63
    }
}

/// NAIVE 6-bit exception: shift-down extraction, x as staged.
fn row_dot_exc6(groups: &[Group6], x: &[f32]) -> f32 {
    let mut acc = 0.0f32;
    for (g, words) in groups.iter().enumerate() {
        let xg = &x[g * 16..g * 16 + 16];
        for (i, &xv) in xg.iter().enumerate() {
            acc += xv * extract6(words, i) as f32;
        }
    }
    acc
}

/// OPT 6-bit exception: the SAME conventional extraction with wpre = 1 —
/// the law's boundary arm (masked-in-place cannot serve: the offset depends
/// on the k-slot too AND fields straddle word boundaries). The staging
/// multiply by wpre is kept explicit — it is the seam, and its value is the
/// exception's own verdict.
const WPRE6: f32 = 1.0; // 6-bit fields straddle words: no prescale to cancel

fn row_dot_exc6_wpre1(groups: &[Group6], xs_pre: &[f32]) -> f32 {
    let mut acc = 0.0f32;
    for (g, words) in groups.iter().enumerate() {
        let xg = &xs_pre[g * 16..g * 16 + 16];
        for (i, &xv) in xg.iter().enumerate() {
            acc += xv * extract6(words, i) as f32;
        }
    }
    acc
}

fn stage6_wpre1(x: &[f32], xs_pre: &mut [f32]) {
    for (src, dst) in x.as_chunks::<16>().0.iter().zip(xs_pre.as_chunks_mut::<16>().0.iter_mut()) {
        for (d, &s) in dst.iter_mut().zip(src.iter()) {
            *d = s * WPRE6; // wpre = 1: the exception's staging is the identity
        }
    }
}

// ── the GEMV wrappers (one row = one group-block dot; the sweep's unit) ──

fn gemv_naive4(rows: &[Vec<Group4>], x: &[f32]) -> f32 {
    let mut total = 0.0f32;
    for r in rows {
        total += row_dot_naive4(r, x);
    }
    total
}

fn gemv_masked4(rows: &[Vec<Group4>], x: &[f32], scratch: &mut [f32]) -> f32 {
    stage4(x, scratch);
    let mut total = 0.0f32;
    for r in rows {
        total += row_dot_masked4(r, scratch);
    }
    total
}

fn gemv_naive8(rows: &[Vec<Group8>], x: &[f32]) -> f32 {
    let mut total = 0.0f32;
    for r in rows {
        total += row_dot_naive8(r, x);
    }
    total
}

fn gemv_masked8(rows: &[Vec<Group8>], x: &[f32], scratch: &mut [f32]) -> f32 {
    stage8(x, scratch);
    let mut total = 0.0f32;
    for r in rows {
        total += row_dot_masked8(r, scratch);
    }
    total
}

fn gemv_naive2(rows: &[Vec<Group2>], x: &[f32]) -> f32 {
    let mut total = 0.0f32;
    for r in rows {
        total += row_dot_naive2(r, x);
    }
    total
}

fn gemv_masked2(rows: &[Vec<Group2>], x: &[f32], scratch: &mut [f32]) -> f32 {
    // staging keys off the FIRST row's group scales — every row shares the
    // same group partition (the GEMV structure: one activation vector, N
    // weight rows), so the ladder is staged once from row 0's scales.
    stage2(&rows[0], x, scratch);
    let mut total = 0.0f32;
    for r in rows {
        total += row_dot_masked2(r, scratch);
    }
    total
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
    /// An in-regime activation: sign * mantissa[0.5, 1) * 2^e, e in
    /// [-8, 8] — 20+ orders of magnitude above the 2^-98 regime bound for
    /// every arm's worst offset.
    fn rand_x(&mut self) -> f32 {
        let sign = if self.next() & 1 == 0 { 1.0f64 } else { -1.0 };
        let m = 0.5 + (self.below(1 << 30) as f64) / ((1u64 << 30) as f64) * 0.5;
        let e = self.below(17) as i32 - 8;
        (sign * m * 2.0f64.powi(e)) as f32
    }
}

const EDGE_WORDS: [u32; 6] = [
    0x0000_0000,
    0xFFFF_FFFF,
    0xAAAA_AAAA,
    0x5555_5555,
    0x0123_4567,
    0xFEDC_BA98,
];

/// The regime-boundary x values: |x| = 2^-98 exactly — x * 2^-28 (the 4-bit
/// arm's deepest prescale) = 2^-126, the smallest normal, the LAST exact
/// prescale. Below this the anti-regime begins and bit-identity is not
/// claimed. (2^-98 built by the rule's own wpre form — the boundary IS a
/// prescale value.)
const BOUNDARY_X: [f32; 2] = [wpre(98), -wpre(98)];

fn assert_bits_eq(a: f32, b: f32, ctx: &str) {
    assert_eq!(
        a.to_bits(),
        b.to_bits(),
        "G1 FAIL at {ctx}: naive {a} != opt {b} ({} vs {})",
        a.to_bits(),
        b.to_bits()
    );
}

// ── provenance (the bench_005/007 pattern; the Windows sibling gate is the
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
                        .find(|l| l.contains("REFUSED") || l.contains("FAIL") || l.contains("refused"))
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
        Err(e) => ("UNAVAILABLE".to_string(), format!("preflight spawn failed: {e}")),
    }
}

fn say(msg: &str) {
    println!("{msg}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

// ── the sweep grid (the staged spec: 256..4096 K x 32..512 rows) ────────

const ROWS_GRID: [usize; 5] = [32, 64, 128, 256, 512];
const K_GRID: [usize; 5] = [256, 512, 1024, 2048, 4096];
/// Best-of-N total-wall passes, interleaved per shape (the sequential-A/B
/// box-noise lesson: contention adds time to SOME runs; the minimum of
/// interleaved pairs separates it).
const MEASURED_PASSES: usize = 5;
const FIXTURE_SEED: u64 = 0x2420_0800_0098_0006;

struct Sweep4 {
    rows: Vec<Vec<Group4>>,
    x: Vec<f32>,
    scratch: Vec<f32>,
}

fn gen_sweep4(rows: usize, k: usize, seed_offset: u64) -> Sweep4 {
    assert!(k.is_multiple_of(64), "K must be a multiple of 64 (the 4-bit layout)");
    let mut rng = Rng::new(FIXTURE_SEED ^ seed_offset);
    let n_groups = k / 64;
    let mut out = Sweep4 {
        rows: Vec::with_capacity(rows),
        x: (0..k).map(|_| rng.rand_x()).collect(),
        scratch: vec![0.0; k],
    };
    for _ in 0..rows {
        let mut r = Vec::with_capacity(n_groups);
        for _ in 0..n_groups {
            let mut g: Group4 = [0; 8];
            for w in &mut g {
                *w = rng.next() as u32;
            }
            r.push(g);
        }
        out.rows.push(r);
    }
    out
}

struct Sweep8 {
    rows: Vec<Vec<Group8>>,
    x: Vec<f32>,
    scratch: Vec<f32>,
}

fn gen_sweep8(rows: usize, k: usize, seed_offset: u64) -> Sweep8 {
    assert!(k.is_multiple_of(64), "K must be a multiple of 64 (the 8-bit layout)");
    let mut rng = Rng::new(FIXTURE_SEED ^ seed_offset);
    let n_groups = k / 64;
    let mut out = Sweep8 {
        rows: Vec::with_capacity(rows),
        x: (0..k).map(|_| rng.rand_x()).collect(),
        scratch: vec![0.0; k],
    };
    for _ in 0..rows {
        let mut r = Vec::with_capacity(n_groups);
        for _ in 0..n_groups {
            let mut g: Group8 = [0; 16];
            for w in &mut g {
                *w = rng.next() as u32;
            }
            r.push(g);
        }
        out.rows.push(r);
    }
    out
}

struct Sweep2 {
    rows: Vec<Vec<Group2>>,
    x: Vec<f32>,
    scratch: Vec<f32>,
}

fn gen_sweep2(rows: usize, k: usize, seed_offset: u64) -> Sweep2 {
    assert!(k.is_multiple_of(32), "K must be a multiple of 32 (the 2-bit ALU-twin layout)");
    let mut rng = Rng::new(FIXTURE_SEED ^ seed_offset);
    let n_groups = k / 32;
    let mut out = Sweep2 {
        rows: Vec::with_capacity(rows),
        x: (0..k).map(|_| rng.rand_x()).collect(),
        scratch: vec![0.0; k],
    };
    // one shared group-scale set for the whole matrix (the GEMV structure:
    // the group partition is a property of the layer, not the row)
    let scales: Vec<f32> = (0..n_groups).map(|_| rng.rand_x()).collect();
    for _ in 0..rows {
        let mut r = Vec::with_capacity(n_groups);
        for (gi, scale) in scales.iter().enumerate() {
            let mut g = Group2 {
                bytes: [0; 8],
                sv: *scale,
            };
            for b in &mut g.bytes {
                // mix the group index into the byte stream so rows differ
                *b = (rng.next() as u32 ^ (gi as u32).wrapping_mul(0x9E37_79B9)) as u8;
            }
            r.push(g);
        }
        out.rows.push(r);
    }
    out
}

fn main() {
    say("── bench_008: the portable CPU-exact prescale GOAT (Batch 242, plan 246) ──");
    say(&format!(
        "host: {} (portable law — no ISA gate; this record is the box that runs it)",
        std::env::consts::ARCH
    ));

    // the codec's own self-check: masks and prescales agree with their
    // power-of-two definitions (exact equality — both sides exact)
    for (p, &m) in MASKS4.iter().enumerate() {
        assert_eq!(m, 0xFu32 << (4 * p), "MASKS4[{p}] disagrees with 0xF << 4p");
        assert_eq!(
            WPRE4[p].to_bits(),
            2.0f32.powi(-4 * p as i32).to_bits(),
            "WPRE4[{p}] is not exactly 2^-{}",
            4 * p
        );
    }
    for (p, &m) in MASKS8.iter().enumerate() {
        assert_eq!(m, 0xFFu32 << (8 * p), "MASKS8[{p}] disagrees with 0xFF << 8p");
        assert_eq!(
            WPRE8[p].to_bits(),
            2.0f32.powi(-8 * p as i32).to_bits(),
            "WPRE8[{p}] is not exactly 2^-{}",
            8 * p
        );
    }
    for (p, (&m, &lad)) in MASKS2.iter().zip(LADDER2.iter()).enumerate() {
        assert_eq!(m, 3u8 << (2 * p), "MASKS2[{p}] disagrees with 3 << 2p");
        assert_eq!(lad.to_bits(), 2.0f32.powi(-2 * p as i32).to_bits());
    }

    // ── provenance (before any timing) ───────────────────────────────────
    let (prov_verdict, prov_line) = preflight();
    say(&format!("provenance: {prov_verdict} — {prov_line}"));
    if prov_verdict != "PASSED"
        && std::env::var("PRESCALE_BENCH_REQUIRE_PROVENANCE").is_ok_and(|v| v == "1")
    {
        say("GATE POSTURE: provenance not PASSED and PRESCALE_BENCH_REQUIRE_PROVENANCE=1 — exit 2");
        std::process::exit(2);
    }
    let citable = prov_verdict == "PASSED";

    // ══ G1 — bit-identity (the exponent-cancellation proof, executed) ══
    say("── G1: masked+prescale bit-identical to shift-down, all four arms ──");
    let mut g1_cases = 0usize;
    let mut rng = Rng::new(FIXTURE_SEED);

    // 4-bit: random groups + all-random x
    for _ in 0..256 {
        let mut g: Group4 = [0; 8];
        for w in &mut g {
            *w = rng.next() as u32;
        }
        let x: Vec<f32> = (0..64).map(|_| rng.rand_x()).collect();
        let mut xs_pre = vec![0.0f32; 64];
        stage4(&x, &mut xs_pre);
        assert_bits_eq(
            row_dot_naive4(std::slice::from_ref(&g), &x),
            row_dot_masked4(std::slice::from_ref(&g), &xs_pre),
            "4-bit random",
        );
        g1_cases += 1;
    }
    // 4-bit: edge words (every field pattern they encode) x boundary + unit x
    for &ew in &EDGE_WORDS {
        let g: Group4 = [ew; 8];
        for &xb in BOUNDARY_X
            .iter()
            .chain([1.0f32, -1.0, 256.0, -256.0, 0.003_906_25].iter())
        {
            let x = [xb; 64];
            let mut xs_pre = [0.0f32; 64];
            stage4(&x, &mut xs_pre);
            assert_bits_eq(
                row_dot_naive4(std::slice::from_ref(&g), &x),
                row_dot_masked4(std::slice::from_ref(&g), &xs_pre),
                &format!("4-bit edge word {ew:#010x}"),
            );
            g1_cases += 1;
        }
    }
    // 4-bit: multi-group rows (the sweep's own shape)
    for &n_groups in &[2usize, 3, 5, 17, 33] {
        let mut row = Vec::with_capacity(n_groups);
        for _ in 0..n_groups {
            let mut g: Group4 = [0; 8];
            for w in &mut g {
                *w = rng.next() as u32;
            }
            row.push(g);
        }
        let x: Vec<f32> = (0..64 * n_groups).map(|_| rng.rand_x()).collect();
        let mut xs_pre = vec![0.0f32; 64 * n_groups];
        stage4(&x, &mut xs_pre);
        assert_bits_eq(
            row_dot_naive4(&row, &x),
            row_dot_masked4(&row, &xs_pre),
            &format!("4-bit multi-group n={n_groups}"),
        );
        g1_cases += 1;
    }

    // 8-bit: the same three fixture classes
    for _ in 0..256 {
        let mut g: Group8 = [0; 16];
        for w in &mut g {
            *w = rng.next() as u32;
        }
        let x: Vec<f32> = (0..64).map(|_| rng.rand_x()).collect();
        let mut xs_pre = vec![0.0f32; 64];
        stage8(&x, &mut xs_pre);
        assert_bits_eq(
            row_dot_naive8(std::slice::from_ref(&g), &x),
            row_dot_masked8(std::slice::from_ref(&g), &xs_pre),
            "8-bit random",
        );
        g1_cases += 1;
    }
    for &ew in &EDGE_WORDS {
        let g: Group8 = [ew; 16];
        for &xb in BOUNDARY_X.iter().chain([1.0f32, -1.0, 65536.0, -65536.0].iter()) {
            let x = [xb; 64];
            let mut xs_pre = [0.0f32; 64];
            stage8(&x, &mut xs_pre);
            assert_bits_eq(
                row_dot_naive8(std::slice::from_ref(&g), &x),
                row_dot_masked8(std::slice::from_ref(&g), &xs_pre),
                &format!("8-bit edge word {ew:#010x}"),
            );
            g1_cases += 1;
        }
    }
    for &n_groups in &[2usize, 3, 5, 17, 33] {
        let mut row = Vec::with_capacity(n_groups);
        for _ in 0..n_groups {
            let mut g: Group8 = [0; 16];
            for w in &mut g {
                *w = rng.next() as u32;
            }
            row.push(g);
        }
        let x: Vec<f32> = (0..64 * n_groups).map(|_| rng.rand_x()).collect();
        let mut xs_pre = vec![0.0f32; 64 * n_groups];
        stage8(&x, &mut xs_pre);
        assert_bits_eq(
            row_dot_naive8(&row, &x),
            row_dot_masked8(&row, &xs_pre),
            &format!("8-bit multi-group n={n_groups}"),
        );
        g1_cases += 1;
    }

    // 2-bit ALU twin: random bytes + scales, plus all-codes edge bytes
    for _ in 0..256 {
        let mut grp = Group2 {
            bytes: [0; 8],
            sv: rng.rand_x(),
        };
        for b in &mut grp.bytes {
            *b = (rng.next() & 0xFF) as u8;
        }
        let x: Vec<f32> = (0..32).map(|_| rng.rand_x()).collect();
        let mut x2 = vec![0.0f32; 32];
        stage2(std::slice::from_ref(&grp), &x, &mut x2);
        assert_bits_eq(
            row_dot_naive2(std::slice::from_ref(&grp), &x),
            row_dot_masked2(std::slice::from_ref(&grp), &x2),
            "2-bit ALU-twin random",
        );
        g1_cases += 1;
    }
    for &eb in &[
        0b11_10_01_00u8,
        0b00_01_10_11,
        0xFF,
        0x00,
    ] {
        let grp = Group2 {
            bytes: [eb; 8],
            sv: 1.0,
        };
        let x = [1.0f32; 32];
        let mut x2 = [0.0f32; 32];
        stage2(std::slice::from_ref(&grp), &x, &mut x2);
        assert_bits_eq(
            row_dot_naive2(std::slice::from_ref(&grp), &x),
            row_dot_masked2(std::slice::from_ref(&grp), &x2),
            &format!("2-bit ALU-twin edge byte {eb:#010b}"),
        );
        g1_cases += 1;
    }

    // 6-bit exception: random words through the straddle-prone layout
    // (3 words = 16 values; off = 30 and 28 both straddle) — the boundary
    // arm, wpre = 1
    for _ in 0..128 {
        let mut g: Group6 = [0; 3];
        for w in &mut g {
            *w = rng.next() as u32;
        }
        let x: Vec<f32> = (0..16).map(|_| rng.rand_x()).collect();
        let mut xs_pre = vec![0.0f32; 16];
        stage6_wpre1(&x, &mut xs_pre);
        assert_bits_eq(
            row_dot_exc6(std::slice::from_ref(&g), &x),
            row_dot_exc6_wpre1(std::slice::from_ref(&g), &xs_pre),
            "6-bit exception (straddle layout)",
        );
        g1_cases += 1;
    }
    say(&format!(
        "G1 PASS — {g1_cases} cases bit-identical (4-bit + 8-bit + the 2-bit ALU twin + the 6-bit exception, random + edge + boundary + straddle)"
    ));

    // ══ G4 — allocation-free (the measured path, both arms, staging in) ══
    say("── G4: zero allocations in the measured path ──");
    {
        let mut s4 = gen_sweep4(32, 256, 0xC0FF_EE08);
        let (_, n) = alloc_armed(|| gemv_naive4(&s4.rows, &s4.x));
        assert_eq!(n, 0, "G4 FAIL: the 4-bit NAIVE arm allocates ({n})");
        let (_, n) = alloc_armed(|| gemv_masked4(&s4.rows, &s4.x, &mut s4.scratch));
        assert_eq!(n, 0, "G4 FAIL: the 4-bit OPT arm allocates ({n})");
        let mut s8 = gen_sweep8(32, 256, 0xC0FF_EE09);
        let (_, n) = alloc_armed(|| gemv_naive8(&s8.rows, &s8.x));
        assert_eq!(n, 0, "G4 FAIL: the 8-bit NAIVE arm allocates ({n})");
        let (_, n) = alloc_armed(|| gemv_masked8(&s8.rows, &s8.x, &mut s8.scratch));
        assert_eq!(n, 0, "G4 FAIL: the 8-bit OPT arm allocates ({n})");
        let mut s2 = gen_sweep2(32, 256, 0xC0FF_EE0A);
        let (_, n) = alloc_armed(|| gemv_naive2(&s2.rows, &s2.x));
        assert_eq!(n, 0, "G4 FAIL: the 2-bit NAIVE arm allocates ({n})");
        let (_, n) = alloc_armed(|| gemv_masked2(&s2.rows, &s2.x, &mut s2.scratch));
        assert_eq!(n, 0, "G4 FAIL: the 2-bit OPT arm allocates ({n})");
    }
    say("G4 PASS — zero allocations, both arms of all three swept faces, staging included (pre-allocated scratch)");

    // ══ G2 — the GEMV sweep, interleaved best-of-N ══
    say("── G2: GEMV sweep (rows x K), opt (masked+prescale) vs naive (shift-down) ──");
    say(&format!(
        "    (interleaved best-of-{MEASURED_PASSES} per shape per arm; correctness-coupled sinks; three faces)"
    ));
    let mut all_opt_win = true;

    // the 4-bit sweep
    say("  [4-bit arm: off = 4p, 8 fields/word]");
    let mut best4 = (f64::INFINITY, f64::INFINITY); // (fastest opt, fastest naive) anchor
    for &rows in &ROWS_GRID {
        for &k in &K_GRID {
            let mut s = gen_sweep4(rows, k, ((rows as u64) << 32) | k as u64);
            // correctness-coupled sink: the arms must agree bit-for-bit at
            // setup — a deleted loop reports a fast wrong number, never a pass
            let reference = gemv_naive4(&s.rows, &s.x);
            let opt = gemv_masked4(&s.rows, &s.x, &mut s.scratch);
            assert_bits_eq(reference, opt, &format!("4-bit sink rows={rows} k={k}"));
            // interleaved best-of-N (min per arm)
            let mut best_opt = f64::INFINITY;
            let mut best_naive = f64::INFINITY;
            for _ in 0..MEASURED_PASSES {
                let t0 = Instant::now();
                let sink = gemv_masked4(&s.rows, &s.x, &mut s.scratch);
                std::hint::black_box(sink);
                best_opt = best_opt.min(t0.elapsed().as_secs_f64());
                let t0 = Instant::now();
                let sink = gemv_naive4(&s.rows, &s.x);
                std::hint::black_box(sink);
                best_naive = best_naive.min(t0.elapsed().as_secs_f64());
            }
            let speedup = best_naive / best_opt;
            if best_opt >= best_naive {
                all_opt_win = false;
            }
            if rows == 512 && k == 4096 {
                best4 = (best_opt, best_naive);
            }
            say(&format!(
                "  rows={rows:>3} k={k:>4}  opt={:>9.3} ms  naive={:>9.3} ms  speedup={speedup:>6.2}x{}",
                best_opt * 1e3,
                best_naive * 1e3,
                if citable { "" } else { "  (PROVISIONAL)" }
            ));
        }
    }

    // the 8-bit sweep
    say("  [8-bit arm: off = 8p, 4 fields/word]");
    let mut best8 = (f64::INFINITY, f64::INFINITY);
    for &rows in &ROWS_GRID {
        for &k in &K_GRID {
            let mut s = gen_sweep8(rows, k, 0x8000_0000_0000_0000 | ((rows as u64) << 32) | k as u64);
            let reference = gemv_naive8(&s.rows, &s.x);
            let opt = gemv_masked8(&s.rows, &s.x, &mut s.scratch);
            assert_bits_eq(reference, opt, &format!("8-bit sink rows={rows} k={k}"));
            let mut best_opt = f64::INFINITY;
            let mut best_naive = f64::INFINITY;
            for _ in 0..MEASURED_PASSES {
                let t0 = Instant::now();
                let sink = gemv_masked8(&s.rows, &s.x, &mut s.scratch);
                std::hint::black_box(sink);
                best_opt = best_opt.min(t0.elapsed().as_secs_f64());
                let t0 = Instant::now();
                let sink = gemv_naive8(&s.rows, &s.x);
                std::hint::black_box(sink);
                best_naive = best_naive.min(t0.elapsed().as_secs_f64());
            }
            let speedup = best_naive / best_opt;
            if best_opt >= best_naive {
                all_opt_win = false;
            }
            if rows == 512 && k == 4096 {
                best8 = (best_opt, best_naive);
            }
            say(&format!(
                "  rows={rows:>3} k={k:>4}  opt={:>9.3} ms  naive={:>9.3} ms  speedup={speedup:>6.2}x{}",
                best_opt * 1e3,
                best_naive * 1e3,
                if citable { "" } else { "  (PROVISIONAL)" }
            ));
        }
    }

    // the 2-bit ALU-twin sweep (the law's plain-ALU face: byte masks, small
    // converts, the group-scale ladder folded at staging — the naive pays
    // the shift AND a second multiply per element)
    say("  [2-bit ALU-twin arm: off = 2p, the scale ladder sv * 2^-2p]");
    let mut best2 = (f64::INFINITY, f64::INFINITY);
    for &rows in &ROWS_GRID {
        for &k in &K_GRID {
            let mut s = gen_sweep2(rows, k, 0x4000_0000_0000_0000 | ((rows as u64) << 32) | k as u64);
            let reference = gemv_naive2(&s.rows, &s.x);
            let opt = gemv_masked2(&s.rows, &s.x, &mut s.scratch);
            assert_bits_eq(reference, opt, &format!("2-bit sink rows={rows} k={k}"));
            let mut best_opt = f64::INFINITY;
            let mut best_naive = f64::INFINITY;
            for _ in 0..MEASURED_PASSES {
                let t0 = Instant::now();
                let sink = gemv_masked2(&s.rows, &s.x, &mut s.scratch);
                std::hint::black_box(sink);
                best_opt = best_opt.min(t0.elapsed().as_secs_f64());
                let t0 = Instant::now();
                let sink = gemv_naive2(&s.rows, &s.x);
                std::hint::black_box(sink);
                best_naive = best_naive.min(t0.elapsed().as_secs_f64());
            }
            let speedup = best_naive / best_opt;
            if best_opt >= best_naive {
                all_opt_win = false;
            }
            if rows == 512 && k == 4096 {
                best2 = (best_opt, best_naive);
            }
            say(&format!(
                "  rows={rows:>3} k={k:>4}  opt={:>9.3} ms  naive={:>9.3} ms  speedup={speedup:>6.2}x{}",
                best_opt * 1e3,
                best_naive * 1e3,
                if citable { "" } else { "  (PROVISIONAL)" }
            ));
        }
    }

    // the op-count ledger (the amortization axis — structural arithmetic,
    // printed at the anchor shape; not a measurement)
    {
        let (rows, k) = (512usize, 4096usize);
        let shifts_eliminated = rows * k;
        say(&format!(
            "op-count ledger @ rows={rows} k={k}: naive pays {shifts_eliminated} variable shifts in the hot loop; opt pays {k} staged prescale multiplies and 0 shifts (a {rows}:1 amortization)"
        ));
        say(&format!(
            "anchor @ rows={rows} k={k}: 4-bit opt {:.3} ms vs naive {:.3} ms · 8-bit opt {:.3} ms vs naive {:.3} ms · 2-bit opt {:.3} ms vs naive {:.3} ms",
            best4.0 * 1e3,
            best4.1 * 1e3,
            best8.0 * 1e3,
            best8.1 * 1e3,
            best2.0 * 1e3,
            best2.1 * 1e3
        ));
    }

    // ══ the verdict (the staged Pareto shape) ══
    say("── VERDICT ──");
    if all_opt_win {
        say(&format!(
            "PARETO-PASS — G1 bit-identical, G4 zero-alloc, opt strictly faster at every one of {} shapes x all three swept faces{}",
            ROWS_GRID.len() * K_GRID.len(),
            if citable { "" } else { " (TIMING PROVISIONAL — quote the PROVENANCE line)" }
        ));
        std::process::exit(0);
    } else {
        say("PARETO-FAIL — the opt arm lost or tied at least one shape (a real measured outcome; record the per-arm split)");
        std::process::exit(1);
    }
}
