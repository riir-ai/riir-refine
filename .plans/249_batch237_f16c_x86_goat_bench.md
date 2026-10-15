# Plan 249 — Batch 237's staged x86 F16C GOAT (bench_009 / Bench 118)

**Status:** IN_FLIGHT (claimed 2026-10-16, shikuwa/4090, riir-refine idle loop)

The Batch-237 P58 rule `f16-to-f32-accumulator-lut-breaks-conversion-serialization`
(rust_perf, simd) landed NotBenchable-on-this-box with the explicit stage marker:
"the x86 F16C GOAT lane is staged — the i7-13700K 4090 box was unreachable; the
numbers are SOURCE-REPORTED, message-only". This box IS the i7-13700K box.
Benchmark tasks do not defer (the global rule); the B231 precedent (the staged
x86 GOAT ran as bench_007/116) and the B242 precedent (bench_008/117, this
lane's own prior unit) fix the shape: a benches/ GOAT fixture + a Bench doc +
a count-neutral in-rule verdict append. No corpus-count change, no bench_spec
flip (the rule stays NotBenchable in the corpus — the GOAT prices the
mechanism; the B204/116 precedent).

## T0 — the design evidence (DONE, pre-plan)

Standalone `rustc 1.98.1 -O --emit asm` probe of the scalar F16C widening
through the packed instruction (the ISA has NO scalar FP16→FP32 form — the C
`_cvtsh_ss` builtin lowers to lane-0 insert + `vcvtph2ps` + lane-0 extract;
the Rust spelling is `_mm_cvtsi32_si128` + `_mm_cvtph_ps` + `_mm_cvtss_f32`
under `#[target_feature(enable = "f16c")]`):

```asm
        vpxor   %xmm0, %xmm0, %xmm0      ; acc = 0, lives in xmm0
.LBB0_9:
        vpinsrw $0, (%rcx,%rax,2), %xmm0, %xmm1   ; merge src = THE ACCUMULATOR
        vcvtph2ps       %xmm1, %xmm1
        vpinsrw $0, (%r8,%rax,2), %xmm0, %xmm2    ; merge src = THE ACCUMULATOR
        vcvtph2ps       %xmm2, %xmm2
        vmulss  %xmm2, %xmm1, %xmm1
        vaddss  %xmm1, %xmm0, %xmm0                ; acc updated
        vpinsrw $0, 2(%rcx,%rax,2), %xmm0, %xmm1   ; unrolled ×2 — same aliasing
```

**The Clang 20.1.8 lowering is reproduced by this toolchain verbatim**: every
`vpinsrw` reads `%xmm0`, the register the live accumulator occupies — a false
dependency THROUGH the accumulator that serializes the chain. The GOAT's naive
arm is real on this box, not a strawman.

Trap ledger from T0: bare rustc defaults to edition 2015 (`use core` fails —
use `std::arch` or `--edition`); the scalar C spellings `_cvtsh_ss`/`_cvtss_sh`
do not exist in stdarch (the `_mm_cvtsh_ss` that exists is AVX512FP16 — wrong
ISA, absent on Raptor Lake); `#[inline(always)]` is rejected with
`#[target_feature]` on 1.98 (use `#[inline]`).

- [x] T0 probe evidence recorded (the asm above; scratch `.b249_probe/`,
      removed at close)

## The fixture

- **dot direction** (the law): three arms over `&[u16]` halves,
  `acc += widen(a[i]) * widen(b[i])`:
  1. `dot_intr` — the F16C hardware widening inside the loop (the naive arm;
     the T0 lowering).
  2. `dot_lut` — the once-built 65536-entry f32 widening table (the rule's
     fix; 256 KiB, setup-paid, L2-resident).
  3. `dot_soft` — the corpus fixture's manual branchy widening (the no-F16C
     reference + the G1 oracle).
- **store direction** (the PER-DIRECTION clause): `f32 → u16` narrowing,
  2 arms — `store_intr` (`_mm_set_ss` + `_mm_cvtps_ph` imm 0 + extract) vs
  `store_rne` (a software RNE reference narrowing — same rounding semantics
  as hardware imm 0, so exactness is decidable bit-for-bit; the corpus
  fixture's truncating `store_f16` is the rule's illustration, not the
  oracle — vcvtps2ph's imm 0 is RNE).
- **G1**: full 65536-pattern sweep — `lut == soft` (by construction, verified
  bit-for-bit) and `intr == soft` on every non-NaN pattern (loud fail
  otherwise); NaN class-match asserted, payload diffs counted + disclosed
  (pre-registered expectation: hardware quiets sNaN inputs — exp 31, frac
  MSB set — the manual form does not; ~1024 patterns). Store direction the
  same over a deterministic f32 population + class edges.
- **G2**: interleaved best-of-5 per cell, n ∈ {1024, 16384, 262144} × 3
  seeds; correctness-coupled sinks (arms must agree bit-for-bit at setup —
  a deleted loop reports a fast wrong number, never a pass). Timing
  population = finite halves (exp≠31).
- **G4**: zero allocations in every measured arm (counting allocator, the
  bench_008 pattern; the table build is setup, outside the measured path).
- **ISA gate**: refuse exit 2 if `!is_x86_feature_detected!("f16c")` — the
  staged lane is x86-F16C-only (the aarch64 posture stays NotBenchable).
- **Verdict bar** (pre-registered): PASS iff `dot_lut` strictly faster than
  `dot_intr` at EVERY cell AND `store_intr` no slower than `store_rne` at
  every cell (the per-direction clause: the dot direction takes the table,
  the store direction keeps the direct conversion). A split = exit 1 with
  the per-arm table recorded honestly (the bench_008 posture).

## Tasks

- [x] T0 design evidence (above)
- [ ] T1 claim row pushed plan-first (this commit; numbering history:
      `.plans/.highwater` read 246 while `247_batch244_*.md` exists on disk —
      the M3 Batch-244 lane's write-back miss; BOTH lanes repaired it and
      both allocated 248 — the dual allocation caught at push (the M3
      Batch-245 claim `d5e097fbe` landed first); this lane YIELDED and this
      plan is 249 per the push-is-the-arbiter protocol)
- [ ] T2 author `benches/bench_009_f16c_lut_vs_intrinsic_goat.rs` + the
      Cargo.toml `[[bench]]` row (harness = false, no required-features —
      the bench_008 shape)
- [ ] T3 G1 executed (full-sweep exactness + the NaN disclosure counts)
- [ ] T4 G2 executed at a provenance-PASSED window (the reflex .ps1 sibling
      gate; interleaved pairs are the contention defense — the LOWO sibling's
      riir-ai build is the known load, disclosed in the record)
- [ ] T5 G4 executed (zero-alloc, all measured arms)
- [ ] T6 Bench 118 written + `.benchmarks/.highwater` 117 → 118
- [ ] T7 count-neutral in-rule verdict append (entries_25.rs context_notes)
      + the stale "x86 GOAT staged" comment repairs (tests.rs ×2, AGENTS ×2,
      .docs/11_domains/rust_perf.md) — every edit count-neutral, the
      batch237 needle tests re-run green
- [ ] T8 gates: clippy default + the bench surface, batch237 needles
      (`cargo test --features rust_perf --lib batch237`), lib default,
      retrieval floors (the rust_perf+latent_retrieval posture, no re-pin
      expected — text-only append); scratch cleanup; land + push + the
      `.distill/001` landing row

## Non-goals

- No corpus-count change, no bench_spec flip, no new research note (the
  bench_008/117 precedent — the rule's stage marker resolves via the
  in-rule append, not a new number).
- No promotion decision — the rule is already in the corpus; this unit
  prices the staged axis only.
