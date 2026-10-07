# 239 — PrismML-Eng NEON PQ2_0 vec_dot `6bfcd79a2` (distill verdict)

**Status:** DISTILLED — batch-ready, no owner gate (M3 CPU-side work; plan-first at
conversion per the Research-236 PRISMML-intake precedent wording — no `.plans/`
number allocated now). ⚠ Numbering note: allocated 238 at write time, RENUMBERED
238→239 on rebase — Batch 202 (landed concurrently from shikuwa/4090) took
`.research/238` first; the renumber-not-reuse fix per the numbering discipline.

Date: 2026-10-07 · graded by the riir-refine idle loop (queue item 5 intake row
2026-10-06, the riir-ai Issue-1034 watch handoff `distill lead`).

## Verdict line

**YES B− corpus (rust_perf simd ×1 + kernel_opt riders ×3) / lane intel MODERATE
(CPU decode class — no scored league lane today) / product coverage: seeds the
rust_perf NEON family (first codec rule; the family is bench-spec-only today).**

## Source + provenance

- `PrismML-Eng/llama.cpp` @ `6bfcd79a2d426abcd2b50e3c2d09ae2225e70a17` (MIT;
  org-name trap re-confirmed: `PrismML` 404s, Batch-127 lesson). Commit
  "arm: NEON vec_dot for PQ2_0 (#265)", 2026-10-05. Files:
  `ggml/src/ggml-cpu/arch/arm/quants.c` (+66), `ggml/src/ggml-cpu/arch-fallback.h`
  (+6/−5). Fresh blob-filtered clone into `.raw/`, quotes verified at the pin by
  `git show`, clone removed at landing.
- Provenance chain: the ARM twin of the fork's x86 SSE work (PR #248, bri-prism);
  the 2-bit codec + `vqtbl1q_u8` replication trick itself predates this PR (the
  fork's NEON `pq2_0_q8_0` kernel, same file — the PR's own comment: "Same 2-bit
  codec and vqtbl1q_u8 replication trick as the existing Q8_0 kernel"). The NEW
  content is (a) the Q8_K-activation form (int32 sub-block accumulation + one
  scale per block), (b) the PTQ1_0 half — LANDED then DROPPED in the same PR on
  two independent real-hardware negatives, (c) the Q8_K row-extent escape hatch,
  (d) the ARMv7 wrapper fix.

## Ranked candidate table

| # | Pattern | Source | Grade | Disposition |
|---|---|---|---|---|
| 1 | neon-tbl-replicate-2bit-codec-sdot-block-rescale | quants.c `ggml_vec_dot_pq2_0_q8_K` | A− content / lands B− | NEW rust_perf simd rule |
| 2 | lane-widening base-3 decode is dp4a-class; NEON regresses; i8mm/SMMLA is the ARM form | the dropped `ggml_vec_dot_ptq1_0_q8_0` + arch-fallback comment | B+ | RIDER → `swar-lane-widened-base3-dp4a-dot` (B116 addendum) |
| 3 | exact-integer sdot + block-boundary rescale, CPU instance | same kernel's accumulation contract + test design | B | RIDER → `per-block-int32-chunk-rescale-fp32-accum` (B127) |
| 4 | pairing-migration orphans kernels; row-extent escape hatch | arch-fallback removal + `ggml_cpu_pq2_needs_q8_0` | B− | RIDER → `dispatcher-eligibility-predicate-lags-kernel-capability` (B103 addendum) |
| 5 | ARMv7 raw-intrinsic wrapper fix | `vqtbl1q_u8` → `ggml_vqtbl1q_u8` | C | DROP (portability anecdote, not perf decision content; caveat) |
| 6 | cross-arch fp32 non-associativity note | commit message, 27B PTQ1_0 divergence | — | DROP (already-owned: our Sync-Boundary/deterministic-replay laws; source reaches the same "integer path for exactness" conclusion) |

### Rule 1 — the new rust_perf simd rule

`ggml_vec_dot_pq2_0_q8_K` decodes 16 2-bit codes per step ENTIRELY in registers:
`vcombine_u8(raw, raw)` replicates the packed 8 bytes across a `uint8x16_t`, a
`vqtbl1q_u8` with index tables `{0,0,0,0,1,1,1,1, 2,2,2,2, 3,3,3,3}` /
`{4,4,4,4,…}` places each byte at all four sub-block positions, and
`vshlq_u8` by `{0,-2,-4,-6}` + `vandq_u8(0x03)` + `vsubq_s8(…, 1)` lands the
{−1,0,+1} weight in the lane that owns it — no gather, no LUT, no per-element
scalar extraction. Four 32-weight sub-block dots accumulate in **int32** via
`ggml_vdotq_s32` (sdot, v8.2 asimddp — NOT i8mm), and the block scale is applied
**once** per 128-weight block: `sumf += (fp16→f32(x[i].d) * yb->d) *
(float)vaddvq_s32(acc)`.

- **Regime:** small-domain integer codes whose CONSUMER can stay integer
  (q8-quantized activations; the Q8_K layout's one-float-scale-per-256 is what
  makes the int32 accumulator + single rescale legal).
- **Anti-regime:** float-domain outputs with arbitrary per-element scales — those
  need a value LUT, and NEON's missing gather forces the scalar-extraction path
  (our own `katgpt-core/src/simd_lut_dequant.rs` NEON backend is the honest
  in-house contrast: "NEON has no native gather — the gather is scalar").
  The rule's selection axis: keep the dot integer and decode in-register; pay
  the scalar gather only when the output domain is float.
- **Exactness:** integer accumulation → bit-exact vs dequant-then-scalar-dot
  with power-of-two scales (the PR's own test ported from #248: "packed SIMD
  vec_dot bit-for-bit equal to dequantize-then-scalar-dot, with power-of-two
  scales so the comparison is exact"). G1 trivially satisfiable on our Pareto
  validator.
- **Magnitude:** mechanism-level — scalar ≈ 4 ops/weight (shift, mask, sub,
  fma) vs ≈ 0.2 ops/weight amortized (one tbl + one shift/mask + one sdot per
  16 weights). Source-reported **~5.1× pp / ~4.6× tg** on Orange Pi 5 Plus
  (RK3588, Cortex-A76/A55, asimddp) — WEAK-CORE scalar-baseline handicapped;
  expect materially less on M3 Max (never cite as ours). Cross-validation:
  8B PQ2_0 greedy output byte-identical between the ARM NEON build and the x86
  SSE build (#248).

### Rider 2 — the lane-widening ISA boundary (the in-PR measured negative)

The PR first landed a NEON PTQ1_0 trit-decode kernel (u16-lane widening:
`floor(3·v/256)` forward-carry, "done 16 lanes at a time in uint16 so 3·v (max
765) never overflows"), then DROPPED it in the same PR: "Two independent
real-hardware reports (bri-prism on Apple M5 Pro, karusrus on a Snapdragon 7
Gen 4 phone) found this kernel slower than the plain scalar loop it was meant
to replace — the u16-lane widening used to decode trits costs more than it
saves." The arch-fallback alias comment now carries the negative in-source.
The named replacement: karusrus's 8-bit lane decode + **i8mm/SMMLA 2×2 tiling**
measuring ~1.9×/1.2× over generic on i8mm silicon (branch
`karusrus/llama.cpp/tree/arm-neon-ptq1_0`; unverified by the PR author — no
i8mm hardware). This is the SAME lane-widening family our B116-addendum rule
`swar-lane-widened-base3-dp4a-dot` landed as a dp4a/x86 WIN — the rider adds
the ISA boundary: lane-widening base-3 decode is dp4a-class; NEON without
i8mm keeps scalar; i8mm silicon takes the SMMLA form. The corpus's rule now
carries all three arms of the decode-ISA decision (dp4a-widen / scalar /
smmla).

### Rider 3 — the exact-integer anchor's CPU instance

The kernel is `per-block-int32-chunk-rescale-fp32-accum` (B127, CUDA wgmma)
re-spelled on CPU NEON: int32 sdot accumulators sized to the weight block,
fp32 outer accumulator, one rescale per block. Positive-instance class → the
rider records the instance + the validation METHODOLOGY (the exact vec_dot
test with power-of-two scales; the PR's property harness "~124k random inputs
per format" + exhaustive edge sweep "every weight byte 0-255 × activation
extremes"; and the review-caught `vpadalq_s16` width bug — "vpaddl_s8+vmovl_s16
produces int32x4_t, not the int16x8_t the intrinsic expects" — caught by the
compile gate before it ever ran, the B116 never-compiled lesson in miniature).

### Rider 4 — pairing migration orphans kernels; the escape hatch

The fork's traits table moved PQ2_0's `vec_dot_type` Q8_0 → Q8_K in an earlier
commit, silently orphaning the NEON q8_0 kernel: "the existing NEON PQ2_0
kernel targets Q8_0 activations, which is no longer what the traits table
dispatches now that PQ2_0 uses Q8_K — so it was dead code for normal
inference." It compiled, it passed nothing-broke tests, and nothing routed to
it — the inverse of B103-addendum's predicate-narrower-than-kernel (there the
gate is narrower than the kernel; here the KERNEL is orphaned by a pairing
migration). The revive also exposed the real edge: Q8_K activations exist only
per QK_K=256 rows, so non-QK_K-multiple rows would "read past the activation
buffer and return garbage" — the fix adds a runtime escape hatch
(`ggml_cpu_pq2_needs_q8_0`: `ne[0] % QK_K != 0` keeps the Q8_0 kernel + its
Q8_0 activation path "they used before the Q8_K kernels landed: correct, and
still the AVX2 kernel rather than the generic scalar dot"). Rider content:
(1) a vec_dot-type pairing migration must grep EVERY arch's kernel inventory,
not x86; (2) a revived pairing needs the row-extent escape hatch or it trades
a dead kernel for an OOB read.

## Dedup gate (both vocabularies, grep evidence)

- `vqtbl|replicat` → kernel_opt `fp4-nibble-gather-lut-replicated-index-width`
  (B194, DeepGEMM-Ascend + TensorFold rider): TABLE replication across index
  width to delete a pre-mask. Kinship noted; different mechanism (source-byte
  replication via a tbl index table vs table-width replication) and different
  corpus — rust_perf: **zero hits**.
- `sdot|int32.*accum` → B127 `per-block-int32-chunk-rescale-fp32-accum` — the
  candidate is the CPU instance (positive-instance kill for a NEW rule; lands
  as rider 3).
- `swar|widened` → B116-addendum `swar-lane-widened-base3-dp4a-dot` — the NEON
  negative arm is NEW (rider 2).
- `dispatcher|orphan` → B103-addendum `dispatcher-eligibility-predicate-lags-
  kernel-capability` (+ its B116 rider's four instances) — instance (rider 4).
- `telescoped|trit` → B96 `telescoped-digit-coefficients-byte-fma-dot` + B116
  float-pipe extraction family — the dropped PTQ1_0 NEON kernel was a THIRD
  decode family (integer u16 widening), now dead; its measured negative feeds
  rider 2, not a new rule.
- rust_perf `neon|simd`: bench-spec mentions only (`distance_sq_no_sqrt`
  aarch64 note, the transposed-lane matvec bench) — **no NEON codec/dequant
  rule exists**; the simd category has no ggml-cpu member. CLEAN.
- Our own tree (consumer-side relevance check, NOT corpus): riir-infer
  `src/quant/{q2_0,ptq1_0}.rs` hold the format readers (verified against this
  fork per Plan 333 / Issue 980); katgpt-core's NEON surface is
  `simd_lut_dequant.rs` (scalar-gather LUT, f32-domain) + the portable
  bit-plane matvec — neither runs a NEON integer-dotted PQ2_0/PTQ1_0 path.

## GOAT candidate

Rule 1, benched on M3 Max (asimddp; no i8mm dependency): rust_perf Pareto
validator, integer-exact G1 (power-of-two scale vectors → bit-identical vs
the dequant-then-scalar reference), sizes a GEMV row sweep (256..4096 K, 32..512
rows). Bench-design warning from the B62 lesson: the scalar reference must be
the naive per-element codec form — comparing against the TRANSPOSED-lane
multi-accumulator form measures layout, not codec. G4: the kernel is
allocation-free by construction (fixed idx/shift/mask tables hoisted).
Deterministic: pure integer accumulate, no timing-dependent verdicts.

## Lane-intel axis (mandatory output)

- **Mapped class:** "Bonsai-8B/27B CPU decode on ARM" (Issue-1034's own label).
  **No scored league lane today** — every scored row is GPU-bound (4090 CUDA
  tg128/pp2048; the M3 matrix is watch-only, Ultra-bound), and the watch row
  itself marks the commit "none (CUDA lane)" for 4090 decode.
- **Opponent intel:** llama.cpp's CPU/NEON PQ2_0 decode just moved ~4.6× (tg).
  Any FUTURE CPU-backend league row inherits a far stronger opponent bar than
  the pre-PR fork's scalar path. Recommendation (NOT applied from this repo):
  if a CPU decode row ever opens, the row's opp posture pins ≥ `6bfcd79a2d`.
  Files named, not edited: `../riir-ai/.docs/09_performance/perf_rematch_league.md`
  (opponents table) + `../riir-ai/scripts/perf_rematch.sh` (the fork already
  carries a `watch_repo` row; no new watch row owed).
- **Our substrate:** the reference designs for a CPU fast lane (if one opens —
  cold start, CPU-only serving) are this kernel (sdot form, v8.2) and the
  karusrus i8mm/SMMLA form for M-series (M3 Max has FEAT_I8MM + asimddp; the
  sdot kernel needs neither the newer feature). No sibling issue filing owed —
  no open lane claims CPU decode.
- **Absolute-numbers caveat:** RK3588 5.1×/4.6× on a weak-core scalar baseline
  is not M3 headroom evidence; only same-box league cells own that comparison.

## Product-coverage axis (mandatory output)

rust_perf's NEON/simd family is bench-spec-only — rule 1 is its first
decode/codec member, seeding the family the ecosystem actually ships (ggml's
CPU quants are in every local-inference stack). **Follow-up pool** (with the
accumulation clause — pool sources read before the batch window fold into the
same batch): the SAME file's `ggml_vec_dot_q2_0_q8_0` (the replication trick's
in-fork origin), ggml-org upstream `ggml-cpu/arch/arm/quants.c` NEON kernels,
`karusrus/llama.cpp` branch `arm-neon-ptq1_0` (the i8mm/SMMLA reference;
branch, not PR — pin before reading), and the fork's SYCL XMX PQ2_0/PTQ1_0 arm
`a14c7de9` (DPAS 16-wide — different ISA class, watch-only).

## Yield estimate

1 rust_perf rule + 3 riders after ~50% coordinator falsification. The riders
are the likely survivors (their content is source-measured or two-device
measured); rule 1's fixture must be original (int-exact, no verbatim paste).

## Honest caveats

- The PR landed AND dropped its PTQ1_0 half in ONE PR — mine the FINAL tree at
  the pin; the dropped kernel exists only in the intermediate commits.
- The ARMv7 wrapper fix is the author's pattern-match, explicitly NOT
  hardware-verified ("I don't have ARMv7 hardware or a cross-toolchain") — a
  no-op on aarch64 by construction, unconfirmed on armv7.
- Cross-arch G1 for PTQ1_0 CPU lanes carries the fp32 non-associativity wall
  (27B x86↔ARM diverges with scalar too — pre-existing, not this PR).
- Commit carries a `Claude-Session:` trailer — provenance noted; content
  verified at the pin regardless of authorship.
- The fork moves fast (Batch-196 freshness lesson) — quotes pinned at
  `6bfcd79a2d426abcd2b50e3c2d09ae2225e70a17`; re-verify freshness at conversion.
- A76/A55 lack i8mm (v8.2 asimddp only) — the measured numbers are sdot-form
  numbers; the SMMLA form's ~1.9×/1.2× is third-party single-device.

## Bottom line

Convert as ONE batch at the next mining slot: rule 1 → rust_perf simd (M3 Max
bench) + riders 2–4 → kernel_opt; plan-first at conversion (Research-236
PRISMML-intake precedent); no owner gate (M3 CPU work). The 4090
mining-partition check runs at the batch unit, not here.
