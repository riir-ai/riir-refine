# Research 174 — the Bakhshaliyev RTX 4090 FP8 GEMM series: source VERIFIED, staged for the next kernel_opt batch window

**Status:** RECORD — consumed by Batch 148 (Plan 133, landed 2026-09-16: 2 rules + 5
riders admitted, corpus 614 → 616; the admission-time corrections are appended at the
bottom).

## Provenance (both posts fetched FULL-TEXT 2026-09-16, direct LinkedIn read — no .raw clone needed; the source is prose, not a repo)

- **Part 1:** Vugar Bakhshaliyev, "Unlocking 473 TFLOPS on RTX 4090: Reverse-Engineering
  NVIDIA's Undocumented FP8 Tensor Core Layout", LinkedIn Pulse, published **Feb 18, 2026**
  (linkedin.com/pulse/unlocking-473-tflops-rtx-4090-reverse-engineering-fp8-bakhshaliyev-qttee).
- **Part 2:** "Continue: Unlocking 587 TFLOPS on the RTX 4090: A Story of Hardware Reverse
  Engineering and Kernel Optimization", published **Feb 21, 2026**
  (linkedin.com/pulse/continue-unlocking-587-tflops-rtx-4090-story-hardware-bakhshaliyev-nf4te).
- Hardware/software envelope, both parts: NVIDIA GeForce RTX 4090 (AD102, **SM 8.9**, 128 SMs),
  CUDA 12.x, nvcc/cuobjdump/Nsight Compute; boost 2535 MHz stock, +510 MHz core / +1000 MHz
  memory offset for the 587 numbers (3045 MHz).
- Access caveat for the next session: LinkedIn serves the full text to an anonymous fetch
  (verified today) but rate-limits aggressively — jina's anonymous lane was blocked for
  linkedin.com on an unrelated abuse report the same hour. If a refetch fails, this note is
  the quote record; the intake lead in `.distill/001` carries the distilled decision content.

## Verification verdict: every intake claim checked against the source — ALL CONFIRMED, with sharpening corrections

The 2026-09-16 intake lead (`.distill/001`, the unchecked row) was read against the full text.
Item by item:

- **(f) FP8-f16-acc = 2× FP8-f32-acc — CONFIRMED, exact figures:** Part 1 §2.1 table:
  "FP8 + FP16 acc **642.51 TFLOPS** 97.3% · FP8 + FP32 acc **329.33 TFLOPS** 99.8% ·
  FP16 + FP16 acc 324.84 TFLOPS 98.4%" (% of theory). Part 2 repeats "642 TFLOPS (0.209 ms) /
  329 (0.408) / 325 (0.207)". INT8 note: the intake's "INT8 s32-acc already full-rate" is
  **NOT in either fetched text** — treat that sub-claim as intake-only provenance (the writer's
  own account, unverifiable here) unless a third source appears. The mechanism quote (Part 1
  §1.1): "each thread produces two FP16 values packed into a single 32-bit register instead of
  two separate 32-bit FP32 results, allowing the tensor core to issue results twice as fast."
- **The software-lock finding — CONFIRMED:** Part 1 §1.2: cuBLASLt with
  `CUBLASLT_MATMUL_DESC_COMPUTE_TYPE` FP16 on a 4090 returns `CUBLAS_STATUS_NOT_SUPPORTED`;
  §6.2: "NVIDIA's datacenter/consumer GPU segmentation for FP8 capabilities is enforced
  entirely through software (cuBLASLt library checks), not hardware fuses." Part 2 adds the
  FAST_ACCUM nuance: "Those flags affect the epilogue, not the MMA instruction itself."
- **QMMA.16832 opcode + the a1↔a2 swap — CONFIRMED with the precise map:** Part 1 §4.2 SASS:
  `QMMA.16832.F16.E4M3.E4M3 R42, R28.reuse.ROW, R52.COL, R4`. §8 summary table (Part 1):
  PTX ISA says a1 = row g / k-high, a2 = row g+8 / k-low; SM89 hardware reads a1 = **row g+8,
  k-low**, a2 = **row g, k-high** — "Only A-fragment registers 1 and 2 are swapped. B and D
  fragments follow the PTX ISA specification." Part 2 §Phase-2 states the same swap.
  ⚠ **Correction vs the intake:** the intake called the discovery "not previously documented
  anywhere in NVIDIA's official documentation, CUTLASS source code, or academic literature" —
  that is the AUTHOR's claim (Part 1 abstract), not an independent finding; quote it attributed.
- **(a) cp.async measured WORSE — CONFIRMED, Part 2 "The Dead Ends":** "cp.async double
  buffering (v5, v16): ... the overhead of managing two SMEM buffers and commit/wait groups
  cost more than the overlap saved. **318-418 TFLOPS.** Worse than the synchronous path."
  (The intake's "318–418 vs the 546 winner" matches the stock-clock ladder 473 → 513 → 546.)
- **(b) mbarrier polling −16/−19% vs BAR.SYNC — CONFIRMED, Part 2 "The mbarrier Experiment":
  "syncthreads: 590 · mbar_poll: 494 (−16%) · mbar_yield: 478 (−19%)"; mechanism: "Software
  polling burns issue slots on test instructions that could be running MMA or loads."
- **(c) ldmatrix −16% — CONFIRMED:** "ldmatrix (v21): ... Actually made things worse:
  **458 TFLOPS**. NCU showed the compiler generated better scheduling with explicit uint32_t
  loads through swizzle4()." (Intake's "458 vs 546" matches.)
- **(d) XOR-swizzle occupancy compounding — CONFIRMED, Part 2:** "Going from stride-144
  padded to stride-128 XOR-swizzled also saved enough shared memory to fit 3 blocks per SM
  instead of 2. That's 24 warps instead of 16, which helps hide the 26-cycle MMA latency."
  Ladder: BK=128 "the single biggest win" 473 → **513**; swizzle 513 → **546**.
- **(e) Persistent tile-loop −7% — CONFIRMED:** "the tile-loop bookkeeping and L2 cache
  interference between blocks doing different tiles cost 7%. **506 TFLOPS**." (Intake said 506;
  Part 2's stock-clock table separately lists 546 at 8192³ — 506 is the persistent-kernel
  variant's number, do not merge the two cells.)
- **TMA probe — CONFIRMED:** working on SM89: mbarrier.init / arrive / arrive.noComplete /
  test_wait; partial: cp.async.bulk (ptxas expects shared::cluster); Error 715:
  mbarrier.try_wait.parity, mbarrier.expect_tx; Error 700: ld.shared::cluster, %cluster_ctaid;
  driver attribute 95 (cluster launch) = 1 while the runtime property = 0. Part 2's reading:
  "AD102 has the barrier infrastructure ... but lacks the actual DMA engine."
- **Other dead ends (intake did not itemize; carried here for the batch):** register-level
  double buffering v7 = 438 (spills erased the gains); `launch_bounds(256, 2/4)` v8/v22b
  −20% ("The compiler obeys launch_bounds as a maximum, not a target"); 128×256 tile v13 =
  525 (occupancy 3→2 lost more than the ratio gained); warp specialization v15 = 378
  ("Our implementation was broken — 0 effective blocks"); warp 4×2 layout v22 = 469.
- **The wall — CONFIRMED (three walls):** smem (32 KB × 3.125 → 3 blocks), registers
  (~76/thread × 256 × 4 > 65,536), barrier (BAR.SYNC already optimal). NCU at the ceiling:
  tensor pipe ~89%, memory 73%, 3 blocks/SM.
- **Singlesync (the last win, intake omitted it) — CONFIRMED:** paired K-step fragment
  loading under ONE `__syncthreads()` per K-iteration; "1-2% on large matrices, up to 2.5%
  on 2048³"; 587 = 89% of the ~790 peak at 3045 MHz.
- **(g) size-scaling — CONFIRMED (Part 1 stock-clock; the intake's ÷1.077 OC-derivation is
  the intake's own arithmetic, label it as such):** Part 1 §5: 2048³ = 282.4 · 4096³ = 412.4 ·
  8192³ = 448.4 · LLM rectangles 423.8 / 430.9 / **472.6** (2048×4096×11008) — "72% of the
  theoretical 660 TFLOPS peak".
- **The SASS/dead-code methodology lesson — CONFIRMED:** §7.1: a CUTLASS 4.4/CuTe build
  "compiled cleanly and reported 171 TFLOPS — but cuobjdump -sass revealed zero HMMA/QMMA
  instructions. The 'kernel' was executing dead code, and the measurement was meaningless."
  (This is the same instrument lesson as the intake's grep-the-whole-warning-stream note.)

## Staged for the batch window (the delta this batch would land)

1. **NEW rule `occupancy-before-async-consumer-ada`** (B: compute or E: parallelism —
   router-adjudicated at admission): cp.async multistage is a WINNER only in the
   latency/occupancy-starved regime; at high occupancy on compute-bound sm_89 the
   commit/wait-group overhead exceeds the overlap (318–418 vs 546). The rider lands on
   `cpasync-multistage-tile-pipeline` (entries_gpu_09) as the measured LOSER side of its own
   regime boundary; the NEW rule carries the "+1 block/SM buys more than a deeper pipeline"
   decision content. Distinct-from: `register-double-buffer-operand-pipeline` (register
   pressure, ALSO a measured loser here: 438), `smem-budget-single-buffer-refill-at-kill-points`
   (the budget-clamped staging family this contradicts at high occupancy).
2. **NEW rule `hw-barrier-beats-mbarrier-polling`** (B: compute): software mbarrier polling
   burned −16/−19% vs BAR.SYNC on sm_89 — the hardware barrier is optimal; anti-pattern for
   any warp-specialization design on this silicon. Distinct-from:
   `spin-barrier-needs-co-residency-capped-launch` (co-residency vs polling cost),
   `plane-specialization-load-compute-role-split` (the design this hardens).
3. **Riders (evidence additions to existing rules, no new ids):**
   `xor-swizzle-smem-static-index-tables` (+ the occupancy COMPOUNDING face: swizzle wins
   twice — conflict elimination AND the +1 block/SM the saved smem unlocks; 513 → 546),
   `ldmatrix-mma-layout-loads` (+ the sm_89 loser cell: 458, compiler-scheduled u32 loads
   through swizzle4() beat it), `stream-k-persistent-cta-mac-segments` (+ the compute-bound
   sm_89 loser cell: 506, −7%, L2 interference between tiles), `cpasync-multistage-tile-pipeline`
   (+ the occupancy-starved loser regime, per (1) above — the rider that motivates the new
   rule), `mma-fragment-layout-m16n8k16` (+ the SM89 FP8 A-fragment a1↔a2 swap, the
   probe-kernel methodology, attributed to the author as undocumented-elsewhere).
4. **Hardware-facts row (f, minus the unverified INT8 sub-claim)** rides the rule notes:
   FP8-f16-acc = 2× FP8-f32-acc on consumer Ada (642.51 vs 329.33 microbench; cuBLASLt ships
   f32-acc hence its 330); TMA DMA physically absent (barrier primitives present, bulk-copy
   absent, driver/runtime cluster-launch split).
5. **NOT staged:** the 587 overclock numbers (OC-derived, regime-external to our pinned-cell
   methodology — cite stock-clock figures only); FlashAttention follow-ups (different arch,
   sm_120).

Yield at admission: **2 new rules + 5 riders**. Benchability: `kernel_verify` is a STUB —
these land as measured-source records (the numbers ARE the decision content), the
Batch-129/145 convention; the GOAT candidate (occupancy-before-async) is benchable only
through riir-ai Issue 918's rung-A lane on the 4090 and should be re-graded against that
lane's actual results when they land (a sibling agent was mid-run on that lane at staging
time — cargo processes on the box, 2026-09-16).

## Routing

kernel_opt only. No rust_perf content (host-side Go/CUDA sources; the only Rust-adjacent
lesson is the SASS-verification one, already doctrine). riir-ai Issue 918 addendum 2 already
consumes (g); this note supersedes the intake row as the batch input.

## Admission record (Batch 148 / Plan 133, 2026-09-16)

Yield exactly as staged: 2 new rules (`occupancy-before-async-consumer-ada` E,
`hw-barrier-beats-mbarrier-polling` B) + 5 riders (xor-swizzle occupancy-compounding,
ldmatrix sm_89 loser cell, stream-k compute-bound loser cell, cpasync regime-split rider,
mma-fragment-layout QMMA swap + hardware-facts row). Corpus 614 → 616 (E 54 → 55,
B 64 → 65). All GOAT gates green: G1 parse+presence+rider-marks test
(`batch148_pseudocode_parses`), G2 lib tests at default (1564) + kernel_opt (1911) + the
retrieval floors re-measured (15/15 incl. `retrieval_precision_regression_gate`), G3 clippy 0
at default `--all-targets` + kernel_opt + rust_perf_bench states.

**The GOAT re-grade EXECUTED** (the charter's open precondition): the rung-A lane landed as
riir-ai Bench 933 the same window this note was staged, and the re-grade SPLIT the rule's
own headline — "+1 block/SM buys more than a deeper pipeline" is TRUE for the SMEM route
(XOR-swizzle 513 → 546 external) and FALSE for the register route (MINB=3 `__launch_bounds__`
spills: 0.516×/0.340× our lane; −20% external; launch_bounds-forcing 20×-class on the
CloudRift FFMA cell) and for the tile-shrink route (v12 spill-free at 75 regs, still
0.648× — tile economics beat occupancy). The rule ships with the route split as its PRIMARY
decision content; the unsplit staged form would have been a corpus defect (citing a win
whose losing half we measured ourselves). Category adjudication: E not "B or E" (the four
existing occupancy rules all live in E); hw-barrier → B per charter, consistent with
`masked-branchless-wait-collapse`. Cell hygiene: Bench 933 figures quote the primary tables
(§4 0.516×, §6 0.340×, §7 v12 0.648×), never §7's summary roundings (0.522×/0.330×).
