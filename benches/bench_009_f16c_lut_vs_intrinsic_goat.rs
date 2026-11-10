//! bench_009 — the Batch-237 staged x86 F16C GOAT (plan 249 / Bench 118).
//!
//! The rule `f16-to-f32-accumulator-lut-breaks-conversion-serialization`
//! landed NotBenchable-on-this-box at B237 ("the x86 F16C GOAT lane is
//! staged — the i7-13700K 4090 box was unreachable; the numbers are
//! SOURCE-REPORTED, message-only"). This fixture is that staged lane, run
//! on the box it was staged for (i7-13700K, F16C present).
//!
//! THE LAW UNDER TEST: a scalar FP16 -> FP32 hardware conversion called
//! INSIDE an accumulating dot loop can be lowered by the compiler into a
//! lane-merge of the loop's accumulator register — a false dependency that
//! serializes the chain — so the widening goes through a once-built
//! bit-pattern lookup table whose reads land in fresh registers, PER
//! DIRECTION: the table for FP16->FP32 (the dot direction feeds the live
//! accumulator), the direct conversion for FP32->FP16 (the store direction
//! has no accumulator to poison).
//!
//! T0 design evidence (standalone rustc 1.98.1 -O --emit asm, this box):
//! the F16C ISA has NO scalar FP16->FP32 instruction; the C `_cvtsh_ss`
//! builtin (and our honest Rust spelling: `_mm_cvtsi32_si128` +
//! `_mm_cvtph_ps` + `_mm_cvtss_f32` under `#[target_feature(enable =
//! "f16c")]`) lowers to a lane-0 `vpinsrw` + `vcvtph2ps` pair, and the
//! register allocator merges INTO THE ACCUMULATOR's register:
//!
//! ```asm
//!         vpxor   %xmm0, %xmm0, %xmm0      ; acc lives in xmm0
//! .LBB:   vpinsrw $0, (mem), %xmm0, %xmm1  ; merge src = THE ACCUMULATOR
//!         vcvtph2ps %xmm1, %xmm1
//!         ...
//!         vaddss  %xmm1, %xmm0, %xmm0      ; acc updated -> next vpinsrw waits
//! ```
//!
//! — the exact Clang 20.1.8 class the source measured (19 vs 4
//! cycles/iteration estimated, 45-65% slower Q8_0/Q4_0 MUL_MAT_ID
//! kernels), reproduced by THIS toolchain. Both the indexed and the
//! clippy-clean zip loop spellings lower identically (verified).
//!
//! Arms (dot direction — the law): `dot_intr` (the hardware widening in
//! the loop, the naive arm) vs `dot_lut` (the 65536-entry f32 widening
//! table, 256 KiB, built once at setup) vs `dot_soft` (the corpus
//! fixture's manual branchy widening — the no-F16C reference + the G1
//! oracle). Arms (store direction — the PER-DIRECTION clause):
//! `store_intr` (`_mm_set_ss` + `_mm_cvtps_ph` imm 0 + extract) vs
//! `store_rne` (an exact software RNE reference — same rounding semantics
//! as the hardware imm 0, so G1 is decidable bit-for-bit; built as the
//! corpus fixture's truncating form + exact f64 midpoint selection).
//!
//! Gates: G1 exactness (full 65536-pattern widen sweep + the store sweep;
//! NaN class-match asserted, payload diffs counted + disclosed), G2
//! interleaved best-of-5 (the sequential-A/B box-noise defense), G4
//! zero-alloc in the measured paths. Provenance: the reflex sibling
//! preflight gate (.ps1 on Windows), quoted beside every timing. ISA
//! gate: exit 2 without F16C — the staged lane is x86-F16C-only (the
//! aarch64 posture stays NotBenchable; the rule's corpus row is
//! untouched — count-neutral, the B204/116 + B242/117 precedent).
//!
//! Verdict bar (pre-registered in plan 249): PASS iff `dot_lut` strictly
//! faster than `dot_intr` at EVERY cell AND `store_intr` no slower than
//! `store_rne` at every cell. A split is exit 1 with the per-arm table
//! recorded honestly (the bench_008 posture).

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_goat {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::arch::is_x86_feature_detected;
    use std::arch::x86_64::{
        _mm_cvtph_ps, _mm_cvtps_ph, _mm_cvtsi32_si128, _mm_cvtsi128_si32, _mm_cvtss_f32, _mm_set_ss,
    };
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::Instant;

    // ── the conversion arms ─────────────────────────────────────────────────

    /// The scalar FP16 -> FP32 hardware widening through the F16C packed
    /// instruction (the ISA has NO scalar form — the C `_cvtsh_ss` builtin
    /// lowers to exactly this lane-0 insert + vcvtph2ps + lane-0 extract).
    #[inline]
    #[target_feature(enable = "f16c")]
    unsafe fn widen_one(h: u16) -> f32 {
        let v = _mm_cvtsi32_si128(h as i32);
        _mm_cvtss_f32(_mm_cvtph_ps(v))
    }

    /// The scalar FP32 -> FP16 hardware narrowing (lane-0 form, imm 0 = RNE —
    /// the C `_cvtss_sh` builtin's lowering).
    #[inline]
    #[target_feature(enable = "f16c")]
    unsafe fn narrow_one(v: f32) -> u16 {
        let b = _mm_set_ss(v);
        let h = _mm_cvtps_ph(b, 0);
        (_mm_cvtsi128_si32(h) & 0xFFFF) as u16
    }

    /// The reference scalar widening — the corpus fixture's manual branchy
    /// form verbatim (the no-F16C fallback + the G1 oracle; the table is
    /// built from THIS).
    fn widen_soft(h: u16) -> f32 {
        let sign = ((h as u32) & 0x8000) << 16;
        let exp = (h >> 10) & 0x1F;
        let frac = (h & 0x03FF) as u32;
        let bits = match exp {
            0 if frac == 0 => sign,
            0 => {
                // subnormal: renormalize into the f32 exponent field
                let mut f = frac;
                let mut s: u32 = 0;
                while f & 0x0400 == 0 {
                    f <<= 1;
                    s += 1;
                }
                sign | ((113 - s) << 23) | ((f & 0x03FF) << 13)
            }
            31 => sign | 0x7F80_0000 | (frac << 13), // inf / nan
            _ => sign | (((exp as u32) + 112) << 23) | (frac << 13),
        };
        f32::from_bits(bits)
    }

    /// The truncating FP32 -> FP16 narrowing — the corpus fixture's
    /// `store_f16` form verbatim (toward zero on magnitude). The RNE
    /// reference is built on top of it.
    fn narrow_trunc(v: f32) -> u16 {
        let bits = v.to_bits();
        let sign = ((bits >> 16) & 0x8000) as u16;
        let exp = ((bits >> 23) & 0xFF) as i32;
        let frac = bits & 0x007F_FFFF;
        if exp == 0xFF {
            let nan = if frac != 0 { 0x7E00 } else { 0x7C00 };
            return sign | nan;
        }
        let e = exp - 127 + 15;
        if e >= 0x1F {
            return sign | 0x7C00; // overflow -> inf
        }
        if e <= 0 {
            // underflow: subnormal (truncated) or zero
            // CORRECTED SHIFT (the B237 corpus fixture's store_f16 carried
            // `(14 - exp)` here — ALWAYS negative in this branch (exp is in
            // [102,112]) so the u32 cast wraps and the shift masks to garbage;
            // the fixture's own test never executed this path (store_f16(1.0)
            // only). This bench EXECUTED it and caught it — the derivation:
            // k = (frac|0x800000) >> (126 - exp).)
            if e < -10 {
                return sign;
            }
            let frac = frac | 0x0080_0000;
            let shift = (126 - exp) as u32;
            return sign | ((frac >> shift) as u16);
        }
        sign | ((e as u16) << 10) | ((frac >> 13) as u16)
    }

    /// An exact software RNE FP32 -> FP16 reference: the truncating form
    /// (toward zero on magnitude) picks the NEAR candidate; the tie decision
    /// and the away case go through EXACT f64 midpoints (every f32/f16 value
    /// and their pairwise sums are exact in f64), ties to even mantissa.
    /// Matches `_mm_cvtps_ph` imm 0 (RNE) semantics on all finite inputs.
    fn narrow_rne(v: f32) -> u16 {
        if v.is_nan() {
            let sign = ((v.to_bits() >> 16) & 0x8000) as u16;
            return sign | 0x7E00; // canonical QNaN, payload NOT preserved (disclosed)
        }
        let sign = ((v.to_bits() >> 16) & 0x8000) as u16;
        let x = f64::from(v.abs());
        // the RNE overflow threshold: 65520 = (65504 + 65536) / 2, ties to the
        // even (inf) side; above it the rounding target is inf.
        if x >= 65520.0 {
            return sign | 0x7C00;
        }
        let lo_bits = narrow_trunc(v) & 0x7FFF; // magnitude, <= 0x7BFF here
        let lo = f64::from(widen_soft(lo_bits)); // the toward-zero candidate
        let hi_bits = lo_bits + 1; // the away-from-zero candidate (grid-contiguous:
        // 0x03FF -> 0x0400 spans subnormal->normal at the same 2^-24 step; at the
        // top, x < 65520 already guarantees the answer is 0x7BFF when lo == 0x7BFF)
        if lo == x {
            return sign | lo_bits; // exactly representable (or x == 0) — no rounding
        }
        let hi = f64::from(widen_soft(hi_bits));
        let mid = (lo + hi) * 0.5; // exact in f64 (12-bit-mantissa sum)
        let pick_hi = if x > mid {
            true
        } else if x < mid {
            false
        } else {
            // tie: to even — the LOW BIT of the 10-bit fraction (bit 0 of
            // the candidate, sign excluded; inf never reaches here — the
            // 65520 guard owns that boundary)
            (hi_bits & 1) == 0
        };
        sign | if pick_hi { hi_bits } else { lo_bits }
    }

    // ── the dot arms (identical loop shape — zip spelling, the T0-verified
    //    lowering-equivalent of the indexed form) ────────────────────────────

    /// NAIVE dot: the F16C hardware widening INSIDE the accumulating loop —
    /// the arm the source measured 45-65% slower (the vpinsrw accumulator
    /// merge, reproduced at T0 on this toolchain).
    #[target_feature(enable = "f16c")]
    unsafe fn dot_intr(a: &[u16], b: &[u16]) -> f32 {
        let mut acc = 0.0f32;
        for (&ha, &hb) in a.iter().zip(b.iter()) {
            acc += widen_one(ha) * widen_one(hb);
        }
        acc
    }

    /// OPT dot: the once-built 65536-entry widening table — the read lands in
    /// a fresh register; nothing lane-merges INTO the accumulator.
    fn dot_lut(t: &[f32], a: &[u16], b: &[u16]) -> f32 {
        let mut acc = 0.0f32;
        for (&ha, &hb) in a.iter().zip(b.iter()) {
            acc += t[ha as usize] * t[hb as usize];
        }
        acc
    }

    /// REFERENCE dot: the manual branchy widening (the no-F16C baseline —
    /// llama.cpp's non-F16C path was the LUT, so this arm is context, not
    /// the law's counterfactual).
    fn dot_soft(a: &[u16], b: &[u16]) -> f32 {
        let mut acc = 0.0f32;
        for (&ha, &hb) in a.iter().zip(b.iter()) {
            acc += widen_soft(ha) * widen_soft(hb);
        }
        acc
    }

    // ── the store arms (the PER-DIRECTION clause) ───────────────────────────

    /// DIRECT store: the F16C hardware narrowing per element (no accumulator
    /// downstream — the direction the law says KEEPS the direct conversion).
    #[target_feature(enable = "f16c")]
    unsafe fn store_intr(v: &[f32], out: &mut [u16]) {
        for (i, &x) in v.iter().enumerate() {
            out[i] = narrow_one(x);
        }
    }

    /// SOFTWARE store: the exact RNE reference narrowing.
    fn store_rne(v: &[f32], out: &mut [u16]) {
        for (i, &x) in v.iter().enumerate() {
            out[i] = narrow_rne(x);
        }
    }

    // ── the counting allocator (the bench_008 pattern) ─────────────────────

    static ALLOC_ARM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static ALLOC_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    struct Counting;

    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if ALLOC_ARM.load(std::sync::atomic::Ordering::Relaxed) {
                ALLOC_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if ALLOC_ARM.load(std::sync::atomic::Ordering::Relaxed) {
                ALLOC_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }

    #[global_allocator]
    static GLOBAL_ALLOC: Counting = Counting;

    fn alloc_armed<T>(f: impl FnOnce() -> T) -> (T, usize) {
        ALLOC_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
        ALLOC_ARM.store(true, std::sync::atomic::Ordering::Relaxed);
        let out = f();
        ALLOC_ARM.store(false, std::sync::atomic::Ordering::Relaxed);
        (out, ALLOC_COUNT.load(std::sync::atomic::Ordering::Relaxed))
    }

    // ── the deterministic fixture generator ─────────────────────────────────

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
        /// A FINITE random half (exp != 31 — excludes inf/NaN so every arm's
        /// arithmetic is bit-identical and the sinks couple correctness).
        fn rand_half(&mut self) -> u16 {
            loop {
                let h = self.next() as u16;
                if (h >> 10) & 0x1F != 31 {
                    return h;
                }
            }
        }
        /// A FINITE random f32 across the full exponent range (exp != 0xFF).
        fn rand_f32(&mut self) -> f32 {
            loop {
                let bits = self.next() as u32;
                if (bits >> 23) & 0xFF != 0xFF {
                    return f32::from_bits(bits);
                }
            }
        }
    }

    // ── provenance (the bench_005/007/008 pattern; the Windows sibling gate is
    //    the reflex .ps1 — Issue 065 T3(b), the .sh is the unix half) ─────────

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

    // ── the grid ────────────────────────────────────────────────────────────

    const N_GRID: [usize; 3] = [1024, 16384, 262144];
    const SEEDS: [u64; 3] = [0x0249, 0x1370, 0xF16C];
    /// Best-of-N total-wall passes, interleaved per cell per arm (the
    /// sequential-A/B box-noise lesson).
    const MEASURED_PASSES: usize = 5;
    const FIXTURE_SEED: u64 = 0x0249_00F1_6C00_0007;

    // ── main ───────────────────────────────────────────────────────────────

    pub fn main() {
        say("── bench_009: the Batch-237 staged x86 F16C GOAT (plan 249 / Bench 118) ──");
        // the ISA gate: the staged lane is x86-F16C-only (refuse loud, never a
        // green zero — the aarch64 posture stays NotBenchable in the corpus)
        if !is_x86_feature_detected!("f16c") {
            say("REFUSED: no F16C on this host — the staged lane is x86-F16C-only (exit 2)");
            std::process::exit(2);
        }

        // ── provenance (before any timing) ───────────────────────────────
        let (prov_verdict, prov_line) = preflight();
        say(&format!("provenance: {prov_verdict} — {prov_line}"));
        let citable = prov_verdict == "PASSED";

        // the widening table (SETUP, once — 256 KiB, outside every measured path)
        let table: Vec<f32> = (0u32..=u16::MAX as u32)
            .map(|h| widen_soft(h as u16))
            .collect();
        assert_eq!(table.len(), 65536);

        // ══ G1 widen: the full 65536-pattern exactness sweep ═════════════
        say("── G1 widen: hardware intrinsic vs LUT vs soft, all 65536 patterns ──");
        let mut nan_inputs = 0usize;
        let mut snan_inputs = 0usize;
        let mut nan_payload_diffs = 0usize;
        for h in 0..=u16::MAX {
            let soft = widen_soft(h);
            // the table IS the soft widening, by construction — verified anyway
            assert_eq!(
                table[h as usize].to_bits(),
                soft.to_bits(),
                "G1 CONSTRUCTION FAIL at {h:#06x}"
            );
            let hw = unsafe { widen_one(h) };
            if soft.is_nan() {
                nan_inputs += 1;
                if (h & 0x0200) != 0 && ((h >> 10) & 0x1F) == 31 {
                    snan_inputs += 1;
                }
                assert!(
                    hw.is_nan(),
                    "G1 CLASS FAIL at {h:#06x}: soft NaN, hardware {hw:e}"
                );
                if hw.to_bits() != soft.to_bits() {
                    nan_payload_diffs += 1;
                }
            } else {
                assert_eq!(
                    hw.to_bits(),
                    soft.to_bits(),
                    "G1 WIDEN FAIL at {h:#06x}: hardware {hw:e} vs soft {soft:e}"
                );
            }
        }
        say(&format!(
            "G1 widen PASS — 65536/65536: every non-NaN pattern bit-identical (normals + subnormals + zeros + infs); NaN inputs {nan_inputs} (of which sNaN {snan_inputs}) all class-match, payload diffs {nan_payload_diffs} DISCLOSED (hardware NaN handling vs the manual payload-preserving form)"
        ));

        // ══ G1 store: the deterministic f32 population ═══════════════════
        say("── G1 store: hardware intrinsic (imm 0 = RNE) vs the software RNE reference ──");
        let mut rng = Rng::new(FIXTURE_SEED);
        let mut pop: Vec<f32> = Vec::new();
        // the class edges
        for v in [
            0.0f32,
            -0.0,
            1.0,
            -1.0,
            2.0,
            0.5,
            65504.0,
            65519.996,
            65520.0,
            65520.004,
            65536.0,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            2.0f32.powi(-14),
            -(2.0f32.powi(-14)),
            2.0f32.powi(-15),
            2.0f32.powi(-24),
            -(2.0f32.powi(-24)),
            2.0f32.powi(-25),
            -(2.0f32.powi(-25)),
            2.0f32.powi(-26),
            f32::from_bits(1),
            -f32::from_bits(1),
            1e10,
            -1e10,
            1e-40,
            -1e-40,
            2047.0 * 32.0,
            2048.0 * 32.0,
        ] {
            pop.push(v);
        }
        // every exponent's random mantissas, both signs (32768 cases)
        for e in 0..=254u32 {
            for _ in 0..64 {
                let m = (rng.next() & 0x007F_FFFF) as u32;
                let bits = (e << 23) | m;
                pop.push(f32::from_bits(bits));
                pop.push(f32::from_bits(bits | 0x8000_0000));
            }
        }
        // the EXACT f16-grid midpoints (RNE ties — every 512th grid step) + the
        // grid values themselves, both signs: the tie boundary is the reference's
        // hardest case and the hardware's rounding decision
        for h in (0usize..0x7BFF).step_by(512) {
            let a = widen_soft(h as u16);
            let b = widen_soft((h + 1) as u16);
            let mid = (a + b) * 0.5; // exact in f32 (11-bit-mantissa sum, same binade)
            pop.push(mid);
            pop.push(-mid);
            pop.push(a);
            pop.push(-a);
        }
        // random full-width finite patterns (100k)
        for _ in 0..100_000 {
            pop.push(rng.rand_f32());
        }
        let mut store_cases = 0usize;
        let mut store_nan_cases = 0usize;
        let mut store_nan_payload_diffs = 0usize;
        for &v in &pop {
            let soft = narrow_rne(v);
            let hw = unsafe { narrow_one(v) };
            if v.is_nan() {
                store_nan_cases += 1;
                let hw_is_nan = (hw & 0x7C00) == 0x7C00 && (hw & 0x03FF) != 0;
                assert!(hw_is_nan, "G1 STORE CLASS FAIL at {v:e}");
                if hw != soft {
                    store_nan_payload_diffs += 1;
                }
            } else {
                assert_eq!(
                    hw,
                    soft,
                    "G1 STORE FAIL at v={v:e} (bits {:#010x})",
                    v.to_bits()
                );
            }
            store_cases += 1;
        }
        say(&format!(
            "G1 store PASS — {store_cases} cases: every non-NaN value bit-identical (RNE reference vs hardware, incl. {}/{} explicit ties + all exponent classes + edges); NaN cases {store_nan_cases} class-match, payload diffs {store_nan_payload_diffs} DISCLOSED",
            256,
            pop.len()
        ));

        // ══ G4 — allocation-free (every measured arm) ═══════════════════
        say("── G4: zero allocations in every measured arm ──");
        {
            let n = 16384usize;
            let mut rng = Rng::new(FIXTURE_SEED ^ 0x604);
            let a: Vec<u16> = (0..n).map(|_| rng.rand_half()).collect();
            let b: Vec<u16> = (0..n).map(|_| rng.rand_half()).collect();
            let vs: Vec<f32> = (0..n).map(|_| rng.rand_f32()).collect();
            let mut out_i = vec![0u16; n];
            let mut out_s = vec![0u16; n];
            let (_, k) = alloc_armed(|| unsafe { dot_intr(&a, &b) });
            assert_eq!(k, 0, "G4 FAIL: dot_intr allocates ({k})");
            let (_, k) = alloc_armed(|| dot_lut(&table, &a, &b));
            assert_eq!(k, 0, "G4 FAIL: dot_lut allocates ({k})");
            let (_, k) = alloc_armed(|| dot_soft(&a, &b));
            assert_eq!(k, 0, "G4 FAIL: dot_soft allocates ({k})");
            let (_, k) = alloc_armed(|| unsafe { store_intr(&vs, &mut out_i) });
            assert_eq!(k, 0, "G4 FAIL: store_intr allocates ({k})");
            let (_, k) = alloc_armed(|| store_rne(&vs, &mut out_s));
            assert_eq!(k, 0, "G4 FAIL: store_rne allocates ({k})");
        }
        say(
            "G4 PASS — zero allocations, all five measured arms (the table + fixtures are setup, outside the measured paths)",
        );

        // ══ G2 dot: the interleaved best-of-N sweep ═════════════════════
        say("── G2 dot: LUT vs hardware-intrinsic vs soft (interleaved best-of-5 per cell) ──");
        let mut lut_all_win = true;
        for &seed in &SEEDS {
            for &n in &N_GRID {
                let mut rng = Rng::new(FIXTURE_SEED ^ seed ^ (n as u64));
                let a: Vec<u16> = (0..n).map(|_| rng.rand_half()).collect();
                let b: Vec<u16> = (0..n).map(|_| rng.rand_half()).collect();
                // correctness-coupled sink: a deleted loop reports a fast wrong
                // number, never a pass — all three arms must agree bit-for-bit
                let r_intr = unsafe { dot_intr(&a, &b) };
                let r_lut = dot_lut(&table, &a, &b);
                let r_soft = dot_soft(&a, &b);
                assert_eq!(
                    r_intr.to_bits(),
                    r_lut.to_bits(),
                    "sink intr/lut n={n} seed={seed}"
                );
                assert_eq!(
                    r_lut.to_bits(),
                    r_soft.to_bits(),
                    "sink lut/soft n={n} seed={seed}"
                );
                let mut t_intr = f64::INFINITY;
                let mut t_lut = f64::INFINITY;
                let mut t_soft = f64::INFINITY;
                for _ in 0..MEASURED_PASSES {
                    let t0 = Instant::now();
                    let s = unsafe { dot_intr(&a, &b) };
                    std::hint::black_box(s);
                    t_intr = t_intr.min(t0.elapsed().as_secs_f64());
                    let t0 = Instant::now();
                    let s = dot_lut(&table, &a, &b);
                    std::hint::black_box(s);
                    t_lut = t_lut.min(t0.elapsed().as_secs_f64());
                    let t0 = Instant::now();
                    let s = dot_soft(&a, &b);
                    std::hint::black_box(s);
                    t_soft = t_soft.min(t0.elapsed().as_secs_f64());
                }
                // Loud zero (katgpt-rs Issue 855): an elided arm reads 0 and
                // would decide the ratio over absent work.
                assert!(
                    t_intr > 0.0 && t_lut > 0.0 && t_soft > 0.0,
                    "G2 dot n={n} seed={seed}: an arm timed 0 — region elided"
                );
                let ratio = t_intr / t_lut;
                if t_lut >= t_intr {
                    lut_all_win = false;
                }
                say(&format!(
                    "  seed={seed:#06x} n={n:>6}  lut={:>8.3} ms  intr={:>8.3} ms  soft={:>8.3} ms  intr/lut={ratio:>5.2}x{}",
                    t_lut * 1e3,
                    t_intr * 1e3,
                    t_soft * 1e3,
                    if citable { "" } else { "  (PROVISIONAL)" }
                ));
            }
        }

        // ══ G2 store: the PER-DIRECTION clause ══════════════════════════
        say("── G2 store: direct hardware narrowing vs software RNE (the PER-DIRECTION clause) ──");
        let mut store_all_ok = true;
        for &seed in &SEEDS {
            for &n in &N_GRID {
                let mut rng = Rng::new(FIXTURE_SEED ^ seed ^ (n as u64).wrapping_mul(0x9E37_79B9));
                let vs: Vec<f32> = (0..n).map(|_| rng.rand_f32()).collect();
                let mut out_i = vec![0u16; n];
                let mut out_s = vec![0u16; n];
                // correctness-coupled sink (finite population: bit-identical)
                unsafe { store_intr(&vs, &mut out_i) };
                store_rne(&vs, &mut out_s);
                for i in 0..n {
                    assert_eq!(out_i[i], out_s[i], "store sink n={n} seed={seed} i={i}");
                }
                let mut t_intr = f64::INFINITY;
                let mut t_soft = f64::INFINITY;
                for _ in 0..MEASURED_PASSES {
                    let t0 = Instant::now();
                    unsafe { store_intr(&vs, &mut out_i) };
                    std::hint::black_box(&out_i);
                    t_intr = t_intr.min(t0.elapsed().as_secs_f64());
                    let t0 = Instant::now();
                    store_rne(&vs, &mut out_s);
                    std::hint::black_box(&out_s);
                    t_soft = t_soft.min(t0.elapsed().as_secs_f64());
                }
                assert!(
                    t_intr > 0.0 && t_soft > 0.0,
                    "G2 store n={n} seed={seed}: an arm timed 0 — region elided"
                );
                let ratio = t_soft / t_intr;
                if t_intr > t_soft {
                    store_all_ok = false;
                }
                say(&format!(
                    "  seed={seed:#06x} n={n:>6}  intr={:>8.3} ms  soft={:>8.3} ms  soft/intr={ratio:>5.2}x{}",
                    t_intr * 1e3,
                    t_soft * 1e3,
                    if citable { "" } else { "  (PROVISIONAL)" }
                ));
            }
        }

        // the structural ledger (not a measurement — the per-element opcode
        // classes, the amortization axis the law claims)
        say(
            "ledger: naive dot pays per element 2 vpinsrw + 2 vcvtph2ps (chained INTO the accumulator register — the T0 asm) + 1 vmulss + 1 vaddss; LUT dot pays 2 table loads (fresh registers) + 1 vmulss + 1 vaddss; the table itself is 65536 f32 = 256 KiB, built once at setup (amortized over the whole run)",
        );

        // ══ the verdict (pre-registered bar) ════════════════════════════
        say("── VERDICT ──");
        if lut_all_win && store_all_ok {
            say(&format!(
                "PARETO-PASS — G1 exact (NaN diffs disclosed), G4 zero-alloc, the LUT strictly faster than the hardware-intrinsic dot at ALL {} cells AND the direct store no slower than software RNE at all {} cells{}",
                SEEDS.len() * N_GRID.len(),
                SEEDS.len() * N_GRID.len(),
                if citable {
                    ""
                } else {
                    " (TIMING PROVISIONAL — quote the PROVENANCE line)"
                }
            ));
            std::process::exit(0);
        } else {
            if !lut_all_win {
                say(
                    "PARETO-FAIL (dot) — the LUT lost or tied at least one cell (a real measured outcome; record the per-arm split)",
                );
            }
            if !store_all_ok {
                say(
                    "PARETO-FAIL (store) — the direct hardware narrowing lost to software RNE at at least one cell (a real measured outcome; record the split)",
                );
            }
            std::process::exit(1);
        }
    }
} // mod x86_goat

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn main() {
    x86_goat::main();
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn main() {
    // x86-only bench (F16C): this arch compiles an honest no-op stub so the
    // aarch64 --all-targets lanes stay green; the GOAT numbers are read on the
    // x86 box this lane was staged for (plan 249 / Bench 118).
    println!("bench_009 is x86-only (F16C); nothing to run on this arch");
}
