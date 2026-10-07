# Plan 209 — Batch 209: the 10-08 envelope re-poll — pool2d warp-window crossover rider

**Status:** COMPLETE — Batch 209 landed (rider-only; all gates green: batch209
13-needle gate + batch20x 22/22, kernel_opt+latent lib 3082/0, default lib
2473/0, clippy ×3 clean, retrieval 21/21 held; corpus stays 776).

## Lead

- `ggml-org/llama.cpp` @ `42c787e8c191d49c01c757f4b7029d69ec0bf2c8` —
  "cuda: update uncoalesced memory reads in pool2d (#29425)", merged
  2026-10-07T17:53:43Z, MIT. Thread-per-output pool2d with wide windows
  (MiniMax-M3 indexer block-max: MAX pool, 128x1 windows) scatters — a warp's
  lanes read window starts `sw*4B` apart (512 B at sw=128) = a line per lane.
  Fix: window-size-gated crossover (`kh*kw >= CUDA_POOL2D_WARP_KERNEL_MIN_WINDOW`
  32 = WARP_SIZE) to a warp-per-output kernel whose lanes tile the flattened
  window (`t = lane; t += WARP_SIZE` — coalesced by construction) and finish
  with an op-polymorphic `__shfl_xor_sync` butterfly (max / avg-sum with the
  per-element scale fold); ragged windows skip; `__restrict__` both pointers;
  eval tests cover non-divisible width (4160 = 32.5 windows). No in-source perf
  numbers → NotBenchable.
- Envelope delta otherwise: cubecl `2f0d8e8df5` unchanged, bevy `5cd69c1357`
  unchanged, wgpu feed 404 this pass (last known no-lead), llama.cpp `18b5f8b1`
  server-parse off-lane.
- Disposition: RIDER not rule — the split-axis fix shape is owned by
  `row-dot-split-inner-axis-block-reduce` (this is the work-size-gated regime
  face; that rule owns the output-count-starved face) and the butterfly is
  owned by `warp-shuffle-intrinsics-for-sub-warp-reduction` (B22, xor-butterfly
  riders). New decision content = the per-output WORK-SIZE admission axis
  (`kh*kw >= WARP_SIZE`) + the warp-radius shuffle-only form + the
  pooling-window instance.

## Tasks

- [x] Protocol H (sync 32 repos clean; SYSTEM orphans owner-gated as-found; worktrees kept; disk owner-gated)
- [x] Issue-first gate re-scan (riir-refine zero claimable; Drex cluster = M3 lane)
- [x] R134 re-test 10-08 (blog latest still Sep-30 — nothing new)
- [x] Envelope re-poll (llama.cpp +2 above pin, 1 in-lane lead; cubecl/bevy unchanged)
- [x] Dedup grep (coalesc / pool2d / shfl / restrict families — owners identified above)
- [x] Full commit read at the pin via `.raw/b209-llamacpp` clone
- [x] Plan-first (this file; `.plans/.highwater` 208→209; dual_allocation_gate clean)
- [x] Rider edit: `entries_gpu_06.rs` row-dot rule += the B209 RIDER clause (verbatim quotes)
- [x] `tests.rs`: batch209 rider gate (13 needles; total held by the standing category pin)
- [x] Gates: clippy `-D` x3 postures; default lib 2473/0; kernel_opt+latent_retrieval lib 3082/0; retrieval 21/21 floors (en-route: the pre-existing batch208 too_many_lines pedantic warning split-and-healed)
- [x] Research note 244 (grade + disposition + rider content; `.research/.highwater` 243→244)
- [x] AGENTS.md / README: counts unchanged; batch lineage mention (Batch 1-209)
- [x] Snapshot row prepend (`.distill/001_mining_queue_snapshot.md`)
- [x] Commit + push; `.raw/b209-llamacpp` removed at close
