# Research 025 — riir-gpu Production-Kernel Distillation Batch 24 (cudarc_kernels backward cluster: 64-bit index widening + dual-accumulator reduction sweep + dual-grad same pass + broadcast-uniform-skip-reduce)

**Date:** 2026-08-14
**Source:** riir-ai/crates/riir-gpu/src/cudarc_kernels/backward.rs (1923 lines — the Issue 641 T7.3/T7.5 per-layer backward CUDA kernels for DeltaNet + Attention, explicitly deferred from Batch 22 "to a future batch" in Research 023 §5)
**Target:** `riir-clippy` `kernel_opt` seed corpus (155 → 160 rules after Batch 23 → **164** after this batch)
**Categories touched:** A (+1), B (+1), C (+1), E (+1)

## 1. Context

Batch 23 (Research 024) mined the CubeCL flash-attention decode cluster
(`attention_cubecl/mod.rs`) and grew the corpus to 160 rules across 15
categories (A–O). Research 023 §5 had explicitly deferred
`cudarc_kernels/backward.rs` (1923 lines): "The backward pass contains
gradient-specific patterns (conjugate gradients, outer-product weight grads)
that deserve focused treatment. Some are already covered by
`backward-as-conjugate` (E), `weight-grad-batched-outer-product` (H),
`fused-backward-chain` (H)."

This batch is that focused treatment — the eleventh batch mining the stack's
own kernels and the one that closes out the `cudarc_kernels/` directory
(6/6 files: embedding.rs, attention.rs, mod.rs, deltanet.rs mined in
Batch 22; backward.rs here; lora.rs assessed + dropped as duplicate in
Research 023).

## 2. Source analysis

The file contains 10 CUDA kernels + their Rust launchers + GPU-vs-CPU
cross-validation tests:

| Kernel | Shape | Notable |
|---|---|---|
| `rmsnorm_batched_backward_f32` | one block/head, two-pass | TWO reductions (sum_sq + dot) fused in ONE sweep |
| `swiglu_backward_f32` | 1 thread/element | BOTH partials (grad_gate + grad_up) in one pass, one sigmoid |
| `z_gating_backward_f32` | 1 thread/element | BOTH partials (grad_pre + grad_z), one silu |
| `output_gate_sigmoid_backward_f32` | 1 thread/element | BOTH partials, sigmoid gate (attention analog of z_gating) |
| `conv1d_silu_backward_single_f32` | 1 thread/channel | recomputes forward `raw` from conv_state + weight |
| `conv1d_scatter_backward_f32` | 1 thread/(ch,k) | atomicAdd scatter to overlapping source timesteps |
| `l2_normalize_backward_f32` | 1 block, two-pass | consumes forward-saved `y_hat` + `inv_norm` |
| `beta_decay_backward_f32` | 1 thread/head | consumes forward `decay` values; softplus'=sigmoid identity |
| `softmax_attention_backward_f32` | 1 block/head | recomputes softmax probs; GQA indexing; the T7.2 bug fix |
| `vector_accumulate_f32` | 1 thread/element | trivial in-place `target += src` |

### 2.1 The 64-bit index widening

Every KV cache access in `softmax_attention_backward_f32` uses
`(long)t * kv_stride + kv_off` (L416/L474/L478/L515/L518). int32 index
arithmetic wraps silently in CUDA when position × stride exceeds 2^31 —
long-context KV caches (e.g. t=500_000 × kv_stride=8192 = 4.1e9) produce
wrapped indices → OOB reads / wrong elements with **no error**. The
discipline: keep loop counters + small dims 32-bit (register economy), widen
ONLY the address-producing multiply by casting ONE operand.

### 2.2 The dual-accumulator reduction sweep

`rmsnorm_batched_backward_f32` pass 1 (L61-83): RMSNorm backward needs TWO
reductions over the SAME elements — sum_sq = Σ x[i]² and
dot = Σ grad_y[i]·gamma[i]·x[i]. The kernel computes both partial sums in
ONE strided loop (`x[off+i]` loaded once, feeding both accumulators), then
reduces BOTH shared-memory arrays inside the SAME stride tree
(`s_sum_sq[tid] += s_sum_sq[tid+stride]` AND `s_dot[tid] += s_dot[tid+stride]`
per step). Halves global loads AND halves the `__syncthreads()` count vs two
sequential reductions.

### 2.3 The dual-grad same pass

Four kernels (`swiglu_bwd` L108-126, `z_gating_bwd` L137-155,
`output_gate_bwd` L169-186, `beta_decay_bwd` L345-369) share one shape: a
product-rule activation `out = f(gate)·g(other)` yields TWO partial
derivatives that share the same inputs AND the same transcendental
evaluation. `grad_a = grad_out·f'(a)·b` and `grad_b = grad_out·f(a)` — the
sigmoid is evaluated ONCE and feeds both. One kernel writes both grad
buffers; two kernels would re-read all inputs + re-evaluate exp + launch
twice.

### 2.4 The broadcast-uniform bug fix (T7.2)

`softmax_attention_backward_f32` L492-501 carries the documented war story:

> "z is the same for all threads (gp was already reduced to s_reduce[0], so
> every thread computed the same z_local). No cross-thread reduction needed —
> just broadcast from thread 0 via shared memory. BUG FIX (T7.2 validation
> catch): the prior code reduced z_local across threads, which multiplied z
> by blockDim.x since every thread had the same value."

When a per-thread value is derived ENTIRELY from already-reduced broadcasts,
it is thread-uniform: re-reducing it sums the identical value blockDim.x
times — **a correctness bug, not just wasted barriers**.

## 3. Four new rules distilled

### 3.1 `widen-index-multiply-to-64bit` (Category A — Memory access)

Cast ONE operand of an address-producing multiply to 64-bit when position ×
stride can exceed 2^31; keep loop counters + small dims 32-bit for register
economy. Distinct from `upcast-before-reduction` (fp32 precision) — this is
integer address width.

### 3.2 `dual-accumulator-single-reduction-sweep` (Category B — Compute)

Fuse two block reductions over the same data into one sweep: two partial
accumulators in the same strided loop (each element loaded once, feeding
both), both shared-memory arrays reduced inside the SAME stride tree.
Distinct from `split-sign-accumulators-defer-subtraction` (ILP chains within
one dot) — this is barrier + load sharing across two different reductions.

### 3.3 `dual-grad-same-pass` (Category C — Fusion)

Both partials of a product-rule activation emitted from ONE kernel sharing
input reads + one sigmoid/silu evaluation. Applies to every gated backward.
Distinct from `fused-backward-chain` (producer→consumer chain stays in
registers) and `fuse-gate-up-gemv-activation` (forward sibling GEMVs) — this
is the backward fan-out: sibling grads sharing one evaluation.

### 3.4 `broadcast-uniform-skip-reduce` (Category E — Parallelism)

Thread-uniform values (derived entirely from already-reduced broadcasts)
must be broadcast from thread 0, not re-reduced — re-reducing multiplies the
result by blockDim.x (correctness bug). Distinct from
`single-plane-sufficient-reduction` (dispatch-level redundant planes) — this
is intra-block uniformity after a reduction.

## 4. Substrate-first duplicate detection (4 checks)

| Candidate | Closest existing rule | Verdict |
|---|---|---|
| `widen-index-multiply-to-64bit` | `upcast-before-reduction` (B — fp32 accumulator precision); `block-local-reference-frame-for-overflow` (F — fp32 exp overflow); no integer-address-width rule exists | **Novel** |
| `dual-accumulator-single-reduction-sweep` | `split-sign-accumulators-defer-subtraction` (B — pos/neg ILP chains within ONE dot); `warp-shuffle-intrinsics-for-sub-warp-reduction` (B — reduction mechanism); `single-plane-sufficient-reduction` (E — redundant planes); no multi-reduction-sweep rule exists | **Novel** |
| `dual-grad-same-pass` | `fused-backward-chain` (H — producer→consumer chain); `fuse-gate-up-gemv-activation` (C — forward sibling GEMV pair); `dual-mode-gate-transform` (D — gate formula dispatch); no backward fan-out rule exists | **Novel** |
| `broadcast-uniform-skip-reduce` | `single-plane-sufficient-reduction` (E — N planes recomputing the same reduction at dispatch level; pure waste); this is intra-block uniformity post-reduction where re-reducing is WRONG (×blockDim.x), not just wasteful | **Novel** |

## 5. Candidates assessed and dropped (5 drops)

1. **Recompute-or-save forward state in backward** — softmax bwd recomputes
   probs from `q + k_cache`; conv1d bwd recomputes `raw` from `conv_state +
   weight`; l2norm bwd consumes forward-saved `inv_norm`/`y_hat`; beta_decay
   bwd consumes forward `decay`. **DROP:** the save-vs-recompute tradeoff is
   `activation-recompute-budget` (H, Batch 4) + the cheap-scalar pass-through
   is `precompute-scalar-reduction` (I, Batch 5) + `split-reduction-scalar-prepass`
   (E, Batch 14). Both sides adequately covered.
2. **Atomic scatter-add for overlapping gradient contributions** —
   `conv1d_scatter_backward_f32` distributes `grad_raw * weight[k]` to
   `grad_x_seq[src_t]` where multiple source timesteps collide on the same
   `(src_t, ch)`. **DROP:** `atomic-scatter-reduction` (H, Batch 4) covers
   lock-free atomic scatter for parallel overlapping writes; the conv1d
   specifics (src_t bounds guard) are just the scatter index computation.
3. **Fold launch-loop into grid_dim** — the file itself contrasts rmsnorm's
   `grid_dim=(n_heads,1,1)` + `blockIdx.x` (+ dropped arg, `let _ =
   n_heads_i32`) against softmax_attn's `for head in 0..n_head` launching
   `grid(1,1,1)` per head. **DROP:** `multi-dim-grid-launch` (E, Batch 1)
   already covers sequential loop → parallel grid. The arg-drop corollary is
   a minor consequence, not a separate pattern.
4. **Compute-then-scatter kernel split** — conv1d backward split into
   grad_raw computation (per-channel) + scatter (per (ch,k)). **DROP:** the
   split exists for caller-side timestep iteration simplicity ("the scatter
   to previous timesteps is handled by the caller (CPU) for simplicity" —
   L211-214), not for a portable perf principle; `chain-dispatches-single-sync-point`
   (C, Batch 19) covers the dispatch composition side.
5. **smem multi-array manual packing** — `extern __shared__ float smem[]`
   carved into probs/grad_probs/grad_scores/s_reduce via manual offsets.
   **DROP:** covered as documented context in `dynamic-shared-memory-runtime-sized`
   (A, Batch 22).

## 6. Categories not mined this batch (and why)

- **F (flash-attn tiling), G (linear-attn recurrence), I (NSA), J (varlen),
  K (autotune), L (conv-GEMM), M (polymorphism), N (dtype), O (IR strength
  reduction)** — the backward.rs kernels are elementwise + small-block
  reduction shapes; no new patterns in these categories appear. The patterns
  that do appear (online softmax recompute, chunkwise recurrence, varlen
  packing) are already covered by the popcorn/burn/CubeCL batches.
- **H (MoE + backward)** — the backward-specific patterns that were novel
  landed in their natural homes instead (dual-grad-same-pass → C fusion,
  broadcast-uniform → E parallelism) because their optimization mechanism is
  fusion/parallelism, not backward-specific scheduling.

## 7. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 4 rules valid pseudocode; 0 duplicates (4 substrate-first checks passed); concept strings + rule shapes internally consistent | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1); 177 lib tests `kernel_heal` in 0.11s; 294 all-features in 0.38s | ✅ PASS |
| G3 (no-regression) | All existing tests pass (294 all-features, 0 failed, 9 ignored; +2 new batch24 tests); clippy clean under `-D warnings` | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |
| G5 (routed demo) | Routed heal pipeline maintains 15.0× candidate reduction (164 flat → 10.9 avg per category) | ✅ PASS |

## 8. Next steps

- **riir-gpu distillation remains OPEN** — 25 of ~200 files mined (~12.5%).
  The `cudarc_kernels/` directory is now FULLY covered (6/6 files assessed
  across Batches 22+24). Richest unmined candidates for Batch 25+:
  - **`gemma2_cubecl/` directory** (5 files, ~265KB) — Gemma 2
    model-specific forward (dispatch, gpu_forward, kv_cache, mod,
    weight_buffers).
  - **`gemma4_cubecl/` directory** (4 files) — Gemma 4 model-specific
    forward.
  - **`forward/` directory** (~9K lines across 10 files) — DeltaNet +
    FlashPrefill forward substrate.
  - **`backward/` + `backward_dflash/` directories** (~8K lines) — backward
    pass kernels (the CubeCL-side backward cluster; expect overlap with this
    batch's cudarc backward rules — the vocabulary translation check will be
    load-bearing).
  - **`deltanet_cubecl.rs`** (99K) + **`deltanet_chunked_cubecl.rs`** (84K) —
    the two largest single files in the crate.
- **Centroid mining** via `CommittedFieldBlend` — blocked on real kernel
  benchmarking data (unchanged).
- **`kernel_perf_bench` numeric validator** — blocked on GPU (unchanged).
