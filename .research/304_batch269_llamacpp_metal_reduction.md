# Research 304 — Batch 269 staging: the llama.cpp Metal reduction window (#30862) + the standing-watch re-poll 10-21

**Status:** DISTILLED — converting as Batch 269 / Plan 272 (this note is the window record; the corpus conversion lives in the plan + entries_gpu_116).

## The window (standing-watch re-poll, 2026-10-20 ~21:5x-22:1xZ, zed-m3/M3)

Enumerated past the B264/B265 pins via ls-remote + the GitHub commits API; collision with the
shikuwa/4090 lane's simultaneous poll resolved by push-order (they keep batch 267 / plan 271 —
the cubecl fold/autotune/barrier wave + vllm MLA-prefill prep fusion; their claim row's own
llama.cpp adjudication reads "rest NULL/server/mtmd/hexagon", which is where this window's lead
fell; the material below is what remained outside their claimed set).

### THE LEAD (converted here)

**ggml-org/llama.cpp `df1170213` #30862 — "metal : optimize argmax and pool_1d"**
(MIT, merged 2026-10-20T13:46Z; author Georgi Gerganov; assisted-by a DeepSeek-V4-Flash
session on Mac Studio M2 Ultra 192GB).

Two independent mechanisms in one commit:

1. **Two-pass partial argmax reduction across dispatches** (`kernels/misc.metal` +134,
   `ggml-metal-ops.cpp` +79, `ggml-metal-impl.h` +7): `kernel_argmax_part_f32` (grid
   npart × nrows; each threadgroup strided-scans its chunk = ceil(ne00/npart) elements,
   two-level simd/shared reduce, thread 0 writes (max, arg) to scratch) then
   `kernel_argmax_final_f32` (one threadgroup per row, nth = next-pow2(npart) capped 256,
   reduces the npart partials). Constants: `N_ARGMAX_CHUNK 4096`, `N_ARGMAX_NPART_MAX 256`.
   Scratch `(8 + 4) × npart × ne01` bytes rides past dst in the same buffer
   (`bid_tmp.offs += ggml_nbytes(op)`), sized by the new `ggml_metal_op_argmax_extra`.
   **The stale-partials law**: the two dispatches share a command buffer and need
   `ggml_metal_op_concurrency_reset(ctx)` (= `ggml_metal_encoder_memory_barrier` +
   mem-ranges reset) between them — the PR body: "A memory barrier is inserted between the
   part and final argmax dispatches to avoid reading stale partials."
   Admission: `ne00 > 4096` only; the single-pass kernel stays for small rows.
   The (value, index) pair reduces value by `simd_max` and index by
   `simd_max(select(-1, larg, lmax == max_val))` — index rides only on max-holding lanes.
2. **pool_1d cooperative threadgroup reduction** (`kernels/pool.metal` +122/−44):
   thread-per-output → one threadgroup per output (`nth = min(256, max(1, k0))`,
   `ntg = np` outputs per grid), strided window loop, simd_max + the shared-memory
   per-simdgroup tree, smem 32 floats (max) / 2×32 (avg — lsum + lcnt, the count
   incremented only for in-bounds elements so padding never skews the mean). The third
   face of the window law (Batch-209's CUDA pool2d warp-radius face is the second) —
   count-neutral rider on `grid-over-output-thread-private-sweep` (entries_gpu_06, E).

**MEASURED (the PR body, test-backend-ops perf; source-reported, never our bar):**

| box | shape | base | PR | × |
|---|---|---:|---:|---:|
| M2 Ultra | POOL_1D 2048,8,1 | 247.51 µs | 6.35 | ~39 |
| M2 Ultra | POOL_1D 2048,8,512 | 2303.73 | 36.70 | ~63 |
| M2 Ultra | POOL_1D 8192,16,512 | 9613.05 | 368.94 | ~26 |
| M2 Ultra | ARGMAX 32000,512 | 453.17 | 78.87 | 5.7 |
| M2 Ultra | ARGMAX 32,10 (small-row admission) | 3.01 | 3.00 | flat |
| M5 Max | POOL_1D 2048,8,1 | 110.52 | 2.79 | ~40 |
| M5 Max | POOL_1D 8192,8,1 | 440.14 | 5.37 | ~82 |

Both boxes are OUR silicon family (M-series Metal); the M3 Max sits between them.

### Dedup (grep over src/domains/kernel_opt/corpus/)

- `block-reduction-separate-scratch-runtime-strides` (B, "partials live once the split
  exists") owns the WITHIN-ONE-KERNEL block reduction with scratch; `row-dot-split-inner-axis-
  block-reduce` (E) owns the split-the-reduction-axis mapping; `tree-reduction-depth-over-
  sequential-accumulate` the tree shape. NONE owns the cross-DISPATCH split (two kernels, one
  command buffer, bounded parts, the inter-dispatch encoder barrier) — the new rule composes
  with all three and names them.
- `grid-over-output-thread-private-sweep` (E) owns thread-per-output + the Batch-209 CUDA
  pool2d warp-radius rider → the pool_1d face rides there (count-neutral).
- Argmax-named rules are decision/selection-surface (argmax-epilogue, committed-argmax,
  argmax-rng-tie-break) — no parallel-geometry owner. GREP EVIDENCE:
  'kernel_argmax_part|argmax_final|N_ARGMAX_CHUNK|concurrency_reset' → ZERO corpus hits.

### Watch rows (no corpus; recorded in the snapshot claim row)

cubecl `aad86f453` #1871 (plane barrier WorkgroupMemory — the gpu_85 family lane; RIDES the
B267 lane's claim), `3a7c0e066` #1879 (AutotuneKey shorthand — proc-macro lint hygiene, C+),
llama.cpp `39da130fc` (ssm_conv k=7 — league arch), `b68449857` (mmf tile-width div-64),
`704366d76` (MoE 9-expert metal — the standing watch row LANDED), `7e3eeee04` (H2O-Lightning-4B
decision model — llama.cpp hosts another decision model after Granite Switch #30676; the
reflex/rethink serving-family intel), vllm `9da5f2d31` #60250 (rms_norm_gated default-on
Qwen3.8-27B — league serving intel), mlx `bff10220c` #4682 (NAX affine_qmm_t_nax split-k
LANDED — M3-lane intel), splash release wave ~26 PRs (STAGED as the B268 pool by the 4090
lane — untouched), bevy `59a4d89f9` + `c01a7fa4a` (watch). Static: prism tip == the bout
scored tag `prism-b10787-73bfe6c` (decode re-arm NO-FIRE), lithos `5485ad6`, TensorFold
`d1018b31`, mlxfast `0c7c392` (rider NOT fired; weekly floor ~10-24 stands), uzu 0.6.11 =
the consumed #994 + release tag. wgpu UNREACHABLE 3× this poll (transient; the B267 lane
read it NULL from their box).

### Tie-rule disclosure

The select-mask index reduction yields last-index-wins across lanes on exact ties (the
sequential reference's strict-`>` scan is first-index-within-chunk; cross-chunk behavior
upstream is the select-mask's max-index). Recorded in-rule as a surface disclosure; not
adjudicated here — ggml's own tie semantics for argmax is the upstream question.

## Routing

kernel_opt, category B (Compute — beside `block-reduction-separate-scratch-runtime-strides`).
NotBenchable on this box lane-wise? NO — actually the honest note: the mechanism is Metal-
kernel geometry; our Metal GOAT lane (rust_perf_bench Pareto validator) benches CPU paths,
and no wall-clock bar is asserted; the decision content is verifiable by inspection of any
multi-dispatch reduction. Annotated NotBenchable (source-reported numbers only).
