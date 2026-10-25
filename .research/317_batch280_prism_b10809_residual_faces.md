# Research 317 — Batch 280: the prism-b10809 +22 residual rider faces

**Status:** RECORD — landed count-neutral, corpus stands 966 (Plan 284)

## Provenance

Source: the PrismML llama.cpp fork, LOCAL clone at the scored league tag
`prism-b10809-b3ebc47` (detached HEAD `b3ecb47c28` family — verified `b3ec...` =
`b3ebc47c28` this session; zero fetches; the clone already carried the tag from the
Bench-999 bout). All four commits read in full from the local pin; every rider quote
is a same-session verbatim read. Licence MIT (the fork's own PRs; the base repo's
licence verified in B251/B274 lanes).

The batch converts the four sm_89-relevant residual faces staged detection-only in the
Bench-999 bout staging row (`.distill/001`, 2026-10-25) — the +22 scored window's
sm_89-relevant remainder AFTER B274's riders (#378/#379 already landed there;
#373 graded thin in B272's pool; #374/#381/#382/#376 Hopper-gated, not our silicon).

## The four faces and their laws

### 1. #308 `4c3c02b9d0` — fattn-mma KV-tile residency → rider on gpu_43 `occupancy-before-async-consumer-ada`

The config row for `flash_attn_ext_f16<256,256,1,8>` (one query, GQA 6, head-256) halved
`nbatch_fa` 64 → 32: "fits only 2 blocks per SM because of shared memory and reaches 58%
of the DRAM bandwidth at 120k keys. nbatch_fa 32 fits 5 blocks per SM (84% DRAM)."
Per-layer 32k keys 122 → 87 µs, 120k keys 540 → 370 µs (RTX 4070, Bonsai 2 27B q4_0
K/V); decode 54.4 → 56.4 tok/s @32k, 41.2 → 46.0 @120k; "No change with 2-3 queries
(different config), no change at depth 0."

Law: the THIRD smem-saving purchase route joins gpu_43's list (beside XOR-swizzle and
format-pack) — the STAGE-WINDOW shrink. The load-bearing boundary the rider names:
`nbatch_fa` is how many keys stream through smem per iteration, NOT the compute tile —
so the halving is bytes-per-block saved (the sanctioned route), never the forbidden
v12-class tile shrink (per-block useful mma work). Class note: this kernel is
BANDWIDTH-bound at low residency (58% DRAM = the starvation signal), so the bought
blocks pay in DRAM utilization, not MMA-latency hiding. The knob lives in a config
table row → residency re-tunes are per-(head-dim, ncols2, queries) row, never global.

Same-face adjudication (the staging's owed check): **REFUTED** — B274's #30804 face
(upstream `280141e`) is graph chunking by NODE BUDGET (rides gpu_08
`identity-keyed-graph-retention-stable-arena`); fork #308 is a KV-stage residency knob.
Different axes; the "upstream twin" hypothesis is closed.

### 2. #309 `f386da27b7` + #209 gate-1 `0493f9517f` — the GB10 bring-up-gate pair → rider on gpu_16 `dispatcher-eligibility-predicate-lags-kernel-capability`

Two kernels stranded fleet-wide by the same `cc == GGML_CUDA_CC_DGX_SPARK` conjunct in
one +22 window:

- #309: `concat_dim0_transpose_u32` "was enabled only for cc 12.1 (DGX Spark); other
  GPUs used the generic non-contiguous kernel with one block per output row and about
  6 useful elements per block." The eligibility truth is the SHAPE contract (dim0,
  2D single-slab, ≤8 u32/row, contiguous rows, u32-alignable), not the device. Fix:
  delete the conjunct, zero kernel changes. 8.8 → 2.5 µs/call (RTX 4070 cc 8.9,
  C=10240 f32); +1.2% MTP decode e2e (48 GDN layers); no change at 1 token
  (contiguous case — the shape predicate's own guard). Measure-then-remove rode the
  landing (env switch `GGML_CUDA_CONCAT_TRANSPOSE` shipped then deleted — the gpu_49
  B217 rider's MEASURE-THEM-REMOVE law, cited not re-owned).
- #209 gate 1 of 2: the fused `add_rms_norm_f32` path "was unreachable outside GB10
  for two independent reasons", reason 1 the same conjunct — "although the kernel uses
  no architecture-specific instructions." Fixed to `GGML_CUDA_CC_IS_NVIDIA(cc)`.

Pair law added to gpu_16: a bring-up-DEVICE gate on a kernel whose instruction set is
universal is the defect class's purest form — no regression, no test failure, a whole
fleet silently on the slow path (or a fusion silently never firing). The audit
instrument is REACHABILITY (which devices take the fast path), read from launch counts
and per-call deltas, never from the kernel existing. This is the founding instance's
exact fix shape (delete the stale conjunct — the kernel always served the shape), with
the positive-engagement evidence the PRs themselves ship.

### 3. #209 gate-2 `0493f9517f` — the exact-alias carve-out → rider on popcorn `fuse-add-rms-norm`

Reason 2 of the unreachability: the fusion range check treats an EXACT ALIAS like a
partial overlap — and ggml's allocator produces both: "the residual ADD is in-place
(add->data == add->src[1]->data) and the norm output reuses the other input's buffer
(mul->data == add->src[0]->data). Both are exact aliases - same base pointer, type,
shape and contiguous layout - which the range check treats like a partial overlap."

The carve-out is opt-in per fusion with the kernel-level proof at the enable site:
"An exact alias is a destination that shares the base pointer, type and contiguous
layout of a source... Fusions whose kernels are elementwise within the unit they
parallelize over can opt into accepting those; a partial overlap is still rejected for
everyone." Proof obligation: "elementwise within a row -- each thread reads a[col] and
b[col] before writing sum[col], and block_reduce synchronizes the block before
dst[col] is written." Two guards: partial overlaps stay rejected for ALL callers; the
broadcast weight must not alias any output (three-way pointer guard added).

Honest negative (decision content): "throughput unchanged within noise (tg128 63.88 vs
63.89 t/s, pp512 1704 vs 1701...): at batch 1 the fusion saves one n_embd-row read per
firing, which is negligible against weight traffic" — reachability restored ≠ a
measurable gain at every shape; the win axis is the launch count (96 fused launches
per 32 decode tokens, rms_norm launches drop by 96; ADD_RMS_NORM 25/25). Research 091's
flag ("the two-live-output legality point is a possible one-line rider, not a rule")
is discharged by exactly this rider.

### 4. #380 `23f6463d2c` — thin-F32 matmul admission widening → rider on popcorn `shape-kind-dispatch`

The mmvf (F32 vector-kernel) admission was batch-keyed only (`ne11 <= 3`); the widening
adds the weight's THINNESS as the second key axis: "A thin src0 (e.g. the 32-row GDN
ssm_alpha/ssm_beta projections) gives the tiled mul_mat_f only a block or two; the
vector kernel is several times faster up to its batch limit." →
`if (src0_ne[1] <= 512 && ne11 <= MMVF_MAX_BATCH_SIZE) return true;` (Ampere+).

Mechanism: per-family parallelism scaling — the tiled kernel's grid scales with the
output extent (a 32-row weight starves it at ANY batch), the vector kernel scales over
the reduction axis and holds its win to its own batch limit (8). So the matvec-kind
admission is the CONJUNCTION (batch within the vector limit) AND (output thin enough
that tiled starves). Two free properties: the vector path is EXACT FP32 ("The columns
are batch-invariant and exact FP32" — exactness improves over the lossy TF32 tiled
incumbent; KLD vs F16 within noise, "99.9% KLD better in all cells"), and the regime is
arch-carried (+6% MTP K=3 decode on H200, source-reported).

## Dedup ledger (T1, greps recorded)

- gpu_43's B277 rider = the 91-layout sweep negative (CTAs-per-SM axis) — #308 is the
  stage-window axis, distinct face, same purchase order.
- gpu_16's B116/B204/B225 riders = claim-wider/pairing-migration/surface-reduction —
  #309+#209-g1 are the bring-up-gate pair, a new instance collection (B116 pattern).
- popcorn `fuse-add-rms-norm` was bare-thin; Research 091 named the rider home.
- popcorn `shape-kind-dispatch` was bare-thin; the kind-key-extends face.
- gpu_49's mmvq multi-column family = ACTIVATION LAYOUT lane, not path admission —
  cite-only kinship for #380 (the F32 sibling), no ride.
- `tail-clamped-row-reuse-widens-fast-kernel-admission` (gpu_75) = compile-time partial
  variants, not routing predicates — no ride.
- #30804 same-face: REFUTED (see face 1).

## Gates

- batch280 gate (new): rider (rule_id, mark, provenance) triples + count witness 966 — PASS
- kernel_opt lib: 3226 passed / 0 failed (floor 3225 + the new gate)
- default lib: 2729 passed / 0 failed
- clippy −D ×3 postures (default / kernel_opt / all-features): clean — only the box's
  known incremental-cache hard-link notices (filesystem artifact, not lints)
- batch7 concept pin repaired rider-tolerant (`starts_with`) — the established
  Batch-267/225 pattern, third occurrence
- freeze-table/score-bench NOT owed (count-neutral, zero `build_seed_corpus()` delta);
  retrieval re-measure NOT owed (zero corpus delta — the B277 count-neutral precedent)

Count-neutral: corpus stays **966**. Plan 284 / claim row in the same landing.
