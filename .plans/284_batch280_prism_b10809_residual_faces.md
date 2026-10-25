# Plan 284 — Batch 280: the prism-b10809 +22 residual rider faces

**Status:** IN PROGRESS — claim `PENDING` (claim row lands with this plan in the same commit)
**Claimed:** shikuwa/4090 riir-refine idle loop, 2026-10-25, expires 2026-10-27T~14:xx +07

Batch 280 converts the four sm_89-relevant residual faces staged detection-only in the
Bench-999 bout row (riir-refine `.distill/001` BOUT DISCHARGED row, 2026-10-25) into
count-neutral rider marks. Sources: the LOCAL prism clone at the scored tag
`prism-b10809-b3ebc47` (detached HEAD `b3ebc47c28` — verified this session; zero fetches).

## Faces + routing (decided at T0/T1, quotes machine-verified at the pin)

| # | Fork PR | Commit | Decision content | Owner rule |
|---|---------|--------|------------------|------------|
| 1 | #308 | `4c3c02b9d0` | fattn-mma KV-tile residency: nbatch_fa 64→32 for the head-256 8-col Ampere/Ada config row — 2→5 blocks/SM, 58%→84% DRAM, 540→370 µs/layer @120k keys (RTX 4070); decode 41.2→46.0 tok/s; no change at 2-3 queries / depth 0 | gpu_43 `occupancy-before-async-consumer-ada` (the saved-SMEM purchase order, bandwidth-bound class + stage-window route) |
| 2 | #309 | `f386da27b7` | transposing concat kernel `concat_dim0_transpose_u32` gated `cc == GGML_CUDA_CC_DGX_SPARK` — stranded on every other device; shape predicate alone (dim0, 2D, ≤8 u32/row, aligned); 8.8→2.5 µs/call on RTX 4070; +1.2% MTP decode e2e; measure-then-remove env switch (2nd commit deletes `GGML_CUDA_CONCAT_TRANSPOSE`) | gpu_16 `dispatcher-eligibility-predicate-lags-kernel-capability` (the stale device conjunct, deleted; engagement measured by launch/perf deltas) |
| 3 | #209 | `0493f9517f` | TWO independent unreachability gates: (a) the same GB10 device gate on `add_rms_norm_f32` ("uses no architecture-specific instructions") → `GGML_CUDA_CC_IS_NVIDIA`; (b) the fusion range check rejecting EXACT aliases (in-place residual add + norm output reusing an input buffer) → opt-in `allow_exact_alias` with the kernel-level proof (elementwise-within-row, reads before the block-synced write); partial overlaps stay rejected; broadcast weight must not alias. Measured: 96 fused launches per 32 decode tokens; throughput unchanged within noise at batch 1 (honest negative) | (a) rides gpu_16 with face 2 (one rider, two instances — the B116 collection pattern); (b) rides popcorn `fuse-add-rms-norm` (the aliasing carve-out + evidence + negative) |
| 4 | #380 | `23f6463d2c` | thin-F32 matmul admission widening: mmvf takes thin src0 (≤512 rows) up to `MMVF_MAX_BATCH_SIZE` (8) on Ampere+ instead of TF32 tiled mul_mat_f — "gives the tiled mul_mat_f only a block or two; the vector kernel is several times faster up to its batch limit"; exact FP32 (better than the lossy TF32 incumbent); KLD-vs-F16 within noise; +6% MTP K=3 decode on H200 | popcorn `shape-kind-dispatch` (the family-kind key must include the output extent — thin-output starvation) |

**Same-face adjudication vs B274's #30804 twin: REFUTED** — the B274 SIZE-PLACED-CUT face
(upstream `280141e` #30804) is graph chunking by node budget (rides gpu_08
`identity-keyed-graph-retention-stable-arena`); fork #308 is a KV-stage residency knob.
Different axes; no collision. The staged "twin" hypothesis is closed by this plan.

Count-neutral: corpus stays **966**. All four land as RIDER marks with verbatim quotes
+ a batch280 gate pinning (rule_id, mark text, provenance) triples.

## Tasks

- [x] T0: sources at the pin — local prism clone verified at `b3ebc47c28` (scored tag), all four commits read in full
- [x] T1: dedup greps — gpu_43 (B277 91-layout rider), gpu_16 (B116/B204/B225 riders), popcorn `fuse-add-rms-norm` (bare-thin; Research 091 flagged the rider home), popcorn `shape-kind-dispatch` (bare-thin); #30804 same-face REFUTED; gpu_49's mmvq family = layout lane, NOT admission (cite-only kinship for #380)
- [x] T2: claim row in `.distill/001_mining_queue_snapshot.md` + this plan, one commit, PUSH before authoring
- [ ] T3: author the four rider marks (verbatim quotes at the pin; cross-cites between the #209 halves; the nbatch_fa-is-not-the-compute-tile boundary named explicitly in face 1)
- [ ] T4: extend the corpus gate — `batch280_rules_and_riders_landed`: rider (rule_id, mark, provenance) triples + count witness 966
- [ ] T5: gates — kernel_opt lib (floor 3225+), default lib (floor 2729), clippy −D ×3 postures (default / all-features / no-default)
- [ ] T6: Research 317 rider paragraph + snapshot landing row + plan ticks
- [ ] T7: commit + push (`feat: batch 280: ...`)

## Defers

None.
