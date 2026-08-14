# Research 005 — Popcorn kernel_opt distillation: Batch 4 (MoE dispatch + backward pass, 9 rules)

**Status:** COMPLETE (batch 4 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** Research 002 batch 1 follow-up. The MoE + backward-pass patterns are
the next high-value domain — they cover grouped GEMM (the foundation of every
sparse-routed architecture) and fused backward (the foundation of every training
loop). This batch also adds the first non-forward kernels to the corpus.
**Source:** `/Users/katopz/git/popcorn/src/popcorn/impls/momoe_tl.py` (1283 lines,
MoMoE — Mixture of Matrices of Experts with SwiGLU expert MLPs, including full
forward + backward autograd passes).

## What MoE dispatch + backward is

Mixture-of-Experts (MoE) replaces a single large MLP with N expert MLPs, each
receiving only a routed subset of tokens. The naive implementation launches one
matmul per expert (N kernel launches, N×launch overhead, no cross-expert L2
reuse). The optimized implementation launches a single grouped GEMM where each
thread block derives its expert membership from a prefix-sum (`cumsums`) array —
no per-expert launch, full L2 reuse across experts.

The backward pass is where MoE gets interesting: gradients must scatter back to
multiple experts (atomic writes), weight gradients accumulate as batched outer
products, and two adjacent backward ops (linear + activation) can be fused into
one kernel. The `momoe_tl.py` file has all of these — it's a complete autograd
Function, not just a forward kernel.

## Batch 4 — 9 rules (grouped GEMM + fused backward + expert routing)

| Rule ID | GPU Concept | Source |
|---|---|---|
| `grouped-launch-cumsum-offsets` | grouped GEMM with prefix-sum offsets | lin1_kernel + apply_experts |
| `l2-grouped-tile-schedule` | swizzled L2-cache-aware tile mapping | lin1/lin2/weights_update |
| `fused-matmul-activation` | pointwise activation in matmul epilogue | lin1_kernel (up/gate + SwiGLU) |
| `fused-backward-chain` | two backward ops fused into one kernel | lin2_and_swiglu_backward |
| `activation-recompute-budget` | partial save + in-kernel recompute branch | save logic + swiglu_backward |
| `atomic-scatter-reduction` | lock-free atomic add for overlapping writes | lin1_backward + scatter_cols |
| `weight-grad-batched-outer-product` | weight grads as batched GEMM | weights_update_kernel |
| `csr-index-via-prefix-sum` | GPU-side CSR layout via prefix sum | scatter_cols + find_used_experts |
| `autotune-aggregate-workload-key` | autotune keyed on aggregate work size | autotune decorators |

## Per-rule source attribution

### grouped-launch-cumsum-offsets
**Source:** `lin1_kernel` (L95-203), `apply_experts` (L314-407)
**Concept:** Instead of launching N separate kernels (one per variable-size group),
launch a single grid where each block derives its group membership and per-group
row offset from a prefix-sum (`cumsums`) array. Each block loads
`cumsums[g+1] - cumsums[g]` to get its group's token count, then maps its local
row into the global token buffer. This is the foundation of grouped GEMM.

### l2-grouped-tile-schedule
**Source:** `lin1_kernel` (L141-149), `lin2_kernel` (L237-245), `weights_update_kernel` (L520-528)
**Concept:** Map linear block IDs to (row-block, col-block) tile coordinates using
a grouped/swizzled scheme rather than simple row-major. Blocks within a
`GROUP_SIZE` window tile along the column dimension before advancing the row, so
adjacent blocks share L2 cache lines of the weight matrix. The modulo-within-group
handles the final partial group.

### fused-matmul-activation
**Source:** `lin1_kernel` (L95-203) — fuses up/gate projection + SwiGLU
**Concept:** Fuse a pointwise activation (or any elementwise post-processing) into
the matmul kernel's epilogue, so the accumulator registers feed directly into the
activation and only the final result is written to global memory. Avoids a
full-matrix write + read round-trip through global memory for the intermediate.

### fused-backward-chain
**Source:** `swiglu_backward_kernel` (L567-711), `lin2_and_swiglu_backward` (L712-832)
**Concept:** When two adjacent backward operations share an intermediate (e.g.,
`grad_h` feeds both the linear backward and the activation backward), fuse them
into one kernel. The first backward's output stays in registers and feeds the
second backward directly — no global memory materialization of the intermediate
gradient.

### activation-recompute-budget
**Source:** `lin1_kernel` save logic (L179-185), `swiglu_backward_kernel` branch (L666-700)
**Concept:** Save only the first `Msave` rows of forward activations to a bounded
buffer; for the remaining rows, recompute the forward projection inline during
the backward pass. This trades extra compute for reduced activation memory, and
the branch (`if offset < Msave { read } else { recompute }`) is evaluated per-tile
inside the backward kernel. A classic activation-checkpointing pattern, distilled.

### atomic-scatter-reduction
**Source:** `lin1_backward_kernel` (L921-924), `scatter_cols_kernel` (L1133-1135)
**Concept:** When multiple parallel blocks contribute to the same output element
(e.g., multiple experts/tiles scatter gradients to the same gate-slot or
token-slot), use `atomic_add` with relaxed ordering rather than serializing the
writes. The output buffer is zero-initialized once (`reset_to_zero`), then
accumulated by concurrent blocks.

### weight-grad-batched-outer-product
**Source:** `weights_update_kernel` (L487-561)
**Concept:** Compute weight gradients as `G_W = A^T @ G_b` (an outer-product
accumulation over the token/reduction dimension) within a single batched tile
loop, rather than looping over individual tokens accumulating one rank-1 update
at a time. The per-group token range is sliced via cumsum offsets, and the full
outer product is tiled as a GEMM.

### csr-index-via-prefix-sum
**Source:** `scatter_cols_kernel` (L1071-1125), `find_used_experts` (L1126-1168)
**Concept:** Replace a host-side `nonzero` + sort with a single GPU kernel that
builds a CSR-style sparse layout (`crow` row pointers, `col` column indices)
using a per-tile prefix sum (`cumsum` of the boolean mask within each tile) and
an atomic counter for the `which` (scatter-slot) index. Eliminates the host
round-trip and produces the dispatch index arrays in-place.

### autotune-aggregate-workload-key
**Source:** autotune decorators (L102-106, L226-230, L580-584) — `key=["Msum", "H", "D"]`
**Concept:** For grouped/irregular workloads where the per-group M varies at
runtime (different experts get different token counts each call), autotune on
the aggregate reduction size (`Msum` = total tokens across all groups) rather
than per-group shapes. The total workload — not any single group's dimension —
determines which tile config wins, so the autotune cache key uses the sum.

## How these map to KernelRule

Each rule is a `ref_shape` (naive Rust pseudocode) + `opt_pattern` (optimized
Rust pseudocode). The GPU concept is encoded in the algorithm structure —
`grouped-launch-cumsum-offsets` shows a single grid loop vs an N-launch loop;
`fused-backward-chain` shows one kernel with register handoff vs two kernels
with global-memory handoff. The pseudocode uses `AtomicF32` / `AtomicUsize` as
conceptual types (the pruner's `syn` parser treats them as user types — it
validates syntax, not symbol resolution).

## What's NOT in this batch (honest deferral)

- **MoE-specific routing math** — the softmax-then-topk + renormalize is
  domain-specific, not a general GPU technique. It's already encoded abstractly
  in the `softmax` + `topk` primitives that any ML framework ships.
- **Expert weight layout transform** — momoe pre-concatenates `up_weight` +
  `gate_weight` into a single tensor to enable one grouped GEMM instead of two.
  This is a host-side preprocessing step, not a kernel technique.
- **`MoEFunction(torch.autograd.Function)` glue** — the `forward`/`backward`
  `ctx.save_for_backward` plumbing is PyTorch autograd mechanics, not a GPU
  optimization pattern.

## GOAT gate for this batch

Same as batches 1-3: corpus expansion, no new substrate. G1 (compile) + G3
(no-regression). The rules are Rust-shaped pseudocode validated by
`RustAnalyzerPruner` (syntax check via `syn`).

## Next steps

- [x] Batch 1: 19 rules (memory, compute, fusion, dispatch, parallelism) — Research 002
- [x] Batch 2: 7 rules (flash-attention tiling) — Research 003
- [x] Batch 3: 6 rules (linear-attention recurrence) — Research 004
- [x] Batch 4: 9 rules (MoE dispatch + backward pass) — this note
- [x] Batch 5: 4 rules (NSA block-sparse selection) — Research 006
- [ ] Batch 6+: remaining kernels (losses, activations, normalization variants,
      GroupNorm, LayerNorm, RMSNorm fused variants, KL-div, JSD, sparsemax,
      log-sigmoid, TVD, Hadamard transform, solve_tril, embedding)
