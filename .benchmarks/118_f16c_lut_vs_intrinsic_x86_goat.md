# Bench 118 — the Batch-237 staged x86 F16C GOAT (bench_009 / plan 249)

**Status:** COMPLETE — PARETO-PASS (exit 0), 2026-10-16, the i7-13700K 4090 box
(shikuwa). Provenance PASSED: `PROVENANCE: power=AC scheme=high load=2.64-3.12
swap=919MB gpu=100 %, 11732 MiB canary=skipped` (gpu = the LOWO sibling's
finetune chain; the bench is CPU-side, CPU-quiet at measurement — interleaved
best-of-5 pairs the contention defense).

## What this is

The `f16-to-f32-accumulator-lut-breaks-conversion-serialization` rule (rust_perf,
simd, Batch 237 / P58) landed NotBenchable-on-this-box with the explicit stage
marker: "the x86 F16C GOAT lane is staged, not this batch — the i7-13700K 4090
box was unreachable; the numbers are SOURCE-REPORTED, message-only". This bench
is that staged lane, executed on the box it was staged for — the B231/116
precedent (the lut-free x86 GOAT) and the B242/117 precedent (this lane's own
prescale bench), third of the shape.

## T0 — the lowering reproduced on THIS toolchain (the design evidence)

Standalone `rustc 1.98.1 -O --emit asm` of the scalar F16C widening through the
packed instruction (the F16C ISA has NO scalar FP16→FP32 form; the C
`_cvtsh_ss` builtin and our honest Rust spelling — `_mm_cvtsi32_si128` +
`_mm_cvtph_ps` + `_mm_cvtss_f32` under `#[target_feature(enable = "f16c")]` —
lower to a lane-0 insert + convert + extract):

```asm
        vpxor   %xmm0, %xmm0, %xmm0      ; acc lives in xmm0
.LBB0_9:
        vpinsrw $0, (%rcx,%rax,2), %xmm0, %xmm1   ; merge src = THE ACCUMULATOR
        vcvtph2ps       %xmm1, %xmm1
        vpinsrw $0, (%r8,%rax,2), %xmm0, %xmm2    ; merge src = THE ACCUMULATOR
        vcvtph2ps       %xmm2, %xmm2
        vmulss  %xmm2, %xmm1, %xmm1
        vaddss  %xmm1, %xmm0, %xmm0                ; acc updated
        vpinsrw $0, 2(%rcx,%rax,2), %xmm0, %xmm1   ; unrolled ×2, same aliasing
```

**The Clang 20.1.8 lowering the source measured is reproduced by rustc/LLVM
1.98.1 verbatim** — every `vpinsrw` reads the register holding the live
accumulator. The indexed and the clippy-clean zip loop spellings lower
identically (both verified at T0). T0 trap ledger: bare rustc defaults to
edition 2015 (`use core` fails — `std::arch`); `_cvtsh_ss`/`_cvtss_sh` do not
exist in stdarch (the `_mm_cvtsh_ss` that exists is AVX512FP16 — absent on
Raptor Lake); `#[inline(always)]` is rejected with `#[target_feature]` on 1.98
(`#[inline]` suffices — the probe asm shows the inlining happened).

## The measured record (benches/bench_009_f16c_lut_vs_intrinsic_goat.rs, release)

- **G1 widen — full 65536-pattern sweep: PASS.** `lut == soft` by construction
  (verified bit-for-bit); `hardware == soft` on EVERY non-NaN pattern
  (normals, subnormals, zeros, infs — bit-identical). NaN inputs 2046 (sNaN
  1024): 100% class-match; payload diffs 1022 DISCLOSED — hardware NaN
  canonicalization vs the manual payload-preserving form.
- **G1 store — 132,922 cases: PASS.** `_mm_cvtps_ph` imm 0 (RNE) vs an exact
  software RNE reference (truncation candidate + exact-f64 midpoint selection,
  ties to even) — every non-NaN value bit-identical, including 256 explicit
  f16-grid midpoint ties, all 255 exponent classes (64 random mantissas × 2
  signs each), the class edges (±0, ±inf, 65504 / 65519.996 / 65520 / 65520.004
  / 65536, 2^-14/2^-15/2^-24/2^-25/2^-26, f32 subnormals), and 100k random
  full-width patterns. NaN cases 2: class-match, 0 payload diffs.
- **G4 — zero allocations** in all five measured arms (counting allocator; the
  256 KiB table + fixtures are setup, outside every measured path).
- **G2 dot (the law)** — n ∈ {1024, 16384, 262144} × 3 seeds, interleaved
  best-of-5, correctness-coupled sinks (all three arms bit-identical at every
  setup):

  | cell class | lut | intr | soft | intr/lut |
  |---|---|---|---|---|
  | n=1024 (L1) | 0.001 ms | 0.003 ms | 0.002 ms | **5.00–5.80×** |
  | n=16384 (L2) | 0.008–0.009 ms | 0.047–0.048 ms | 0.029 ms | **5.60–6.14×** |
  | n=262144 (streaming) | 0.121–0.126 ms | 0.748 ms | 0.498–0.501 ms | **5.94–6.16×** |

  **The LUT is 5–6× faster than the hardware-intrinsic dot at every cell.**
  The source-reported 45–65% (Clang 20.1.8) measures **400–516%** on this
  toolchain/uarch — the false dependency costs MORE under rustc/LLVM 1.98 +
  Raptor Lake than the source's own record. Lane intel: the soft branchy
  reference sits at ~2.5× lut (llama.cpp's non-F16C baseline WAS the LUT —
  consistent with the fix's fallback shape).
- **G2 store (the PER-DIRECTION clause)** — same grid, `_mm_cvtps_ph` vs the
  software RNE reference: **soft/intr = 24.00–35.07× — the direct hardware
  narrowing wins massively at every cell.** Disclosed: the soft arm is the
  f64-midpoint reference, not an optimized bit-twiddling narrowing — the
  magnitude is fixture-specific; the DIRECTION (direct conversion is the right
  choice for the store direction — no accumulator to poison) is the claim and
  it holds unambiguously.
- **VERDICT: PARETO-PASS (exit 0)** — the pre-registered bar (lut strictly
  faster than intr at every dot cell AND store_intr no slower than software at
  every cell) held on both axes.

## The corpus-fixture latent bug the bench caught (executed, then repaired)

The B237 fixed_code's `store_f16` subnormal path carried
`let shift = (14 - exp) as u32;` — in the only branch that reaches it
(`e <= 0`, `e >= -10` ⟹ exp ∈ [102, 112]) `14 - exp` is ALWAYS negative; the
`as u32` wraps to ~4.29e9 and the shift masks to garbage. The fixture's own
test never executed the path (`store_f16(1.0)` only — the normal path). This
bench's G1 EXECUTED it and caught it: hardware returned 0x0200 (= 512 × 2^-24
= 2^-15, correct) where the fixture form produced 1 at v = 2^-15. Correct
derivation: k = (frac | 0x800000) >> (126 − exp). Repaired count-neutral in
`entries_25.rs` with the repair note in-rule; needles unaffected (they pin the
source quotes in context_notes, not the fixture code — verified by the 2/2
needle re-run). Second disclosed wart, left as-is: the `e < -10` guard
implements the RNE halfway threshold under a truncation comment (harmless —
the corrected shift subsumes it).

Two reference-algorithm defects were caught and fixed DURING authoring (the
G1 oracle working as designed — both disclosed for the record): the tie rule
first tested `(hi_bits & 0x03FF) == 0` (a multiple-of-1024 test) instead of
the LSB `(hi_bits & 1) == 0` — caught by the tie case v = −7.341797
(frac32 remainder exactly 0x1000, hardware picked even 856); and the
subnormal shift inherited from the corpus fixture (above).

## Gates

- batch237 needles + parse: **2/2** post-edit (`cargo test --features rust_perf
  --lib batch237`).
- default lib: **2676/2676** (count intact — no corpus-count change).
- clippy `--all-targets -D warnings`: **clean** — after repairing the
  pre-existing `batch244_rules_present` too_many_lines break (the M3 Batch-244
  landing's own allow set missed that fn — the gate-ordering class, the
  B242/explicit_iter_loop precedent; repaired with the honest label here).
- retrieval floors: **20 passed / 0 failed / 1 ignored @45.25s — HELD, no
  re-pin** (rust_perf corpus text delta only).
- freeze/score-bench: NOT owed (zero clippy-corpus count delta, P30).
- The corpus count pins stand untouched (217 rules; this unit is count-neutral
  by design — the B204/116 precedent: the GOAT prices the mechanism, the rule
  keeps `bench_spec: None`).

## Disposition

The staged marker resolves: the rule's claimed axes (exactness of the LUT
widening, the register-dependency cost, the PER-DIRECTION discipline) are all
MEASURED-GOAT on the staged box. The rule stays as-is in the corpus
(NotBenchable in-corpus — the aarch64 posture unchanged); the verdict is
recorded in-rule (context_notes append) + AGENTS + the rust_perf.md ledger
row.
