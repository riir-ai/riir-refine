# Research 003 — Popcorn kernel_opt distillation: Batch 2 (flash-attention tiling, 7 rules)

**Status:** COMPLETE (batch 2 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** Research 002 batch 1 follow-up. The flash-attention tiling patterns are
the most complex GPU optimization technique and deserve their own batch.
**Source:** `/Users/katopz/git/popcorn/src/popcorn/impls/wall_attn_tl.py` (39KB) +
`nsa_tl.py` (18KB).

## What flash-attention tiling is

Standard attention materializes the full `Q @ K^T` attention matrix (O(N²) memory),
then applies softmax, then multiplies by V. For long sequences this doesn't fit in
GPU memory. Flash-attention (Dao, 2022) streams K/V blocks through shared memory,
computing the online softmax incrementally — never materializing the full attention
matrix. The key insight is the **online softmax**: maintain a running max + running
sum, rescaling the accumulator as each new block arrives.

## Batch 2 — 7 rules (flash-attention + advanced tiling)

### The online softmax algorithm (THE core pattern)

The naive softmax requires the full row to compute `max(scores)` before any
normalization. The online softmax maintains a running max `m` and running sum `l`,
rescaling the output accumulator `o` when a new block's max exceeds the old:

```
For each KV block j:
    s_j = Q @ K_j^T * scale          # block attention scores
    m_new = max(m_old, max(s_j))      # update running max
    r = exp(m_old - m_new)             # rescale factor for old
    p_j = exp(s_j - m_new)             # new probabilities
    l = l * r + sum(p_j)               # update running sum
    o = o * r + p_j @ V_j              # rescale old output + add new
```

This is O(N) memory instead of O(N²). The `exp(m_old - m_new)` rescaling is the
critical trick — it keeps the numerical scale correct across blocks.

| Rule ID | GPU Concept | Source |
|---|---|---|
| `online-softmax-streaming` | flash-attention online softmax | wall_attn_tl lines 149-155 |
| `exp2-log2-base2` | base-2 math (faster than natural exp/ln) | wall_attn_tl RCP_LN2 + exp2 |
| `block-pointer-2d` | structured 2D block loads with boundary check | wall_attn_tl `tl.make_block_ptr` |
| `heuristic-compile-dispatch` | compile-time decisions from input args | wall_attn_tl `@triton.heuristics` |
| `causal-diag-offdiag-split` | split attention matrix into diagonal + off-diagonal | wall_attn_tl off-diag/diag loops |
| `shared-mem-aware-tile` | adapt tile sizes to available shared memory | `check_shared_mem` |
| `kv-block-streaming` | stream K/V blocks through shared memory | wall_attn_tl KV loop |

## How these map to KernelRule

The online softmax is the most complex pattern to encode as Rust-shaped pseudocode
because it's inherently a streaming algorithm. The `ref_shape` shows the naive
full-materialization version; the `opt_pattern` shows the streaming version with
running max + running sum + rescale.

## What's NOT in this batch (honest deferral)

- **Backward pass flash-attention** — the backward kernel recomputes attention
  (dKV, dQ, dK loops). Even more complex than forward; needs careful reading of
  the `bwd_kernel` variants in wall_attn_tl.py.
- **Native sparse attention (NSA)** — nsa_tl.py has a distinct pattern
  (block-sparse selection + top-k routing). Needs its own batch.
- **Gated Oja rule** — wall_attn_tl.py is specifically for the GOR training kernel
  (g_cumsum decay factors). The GOR-specific math is domain-specific, not general.
- **Chunkwise parallel** — the DeltaNet chunkwise pattern (Issue 652) is a different
  tiling strategy for linear attention. Needs its own analysis.

## GOAT gate for this batch

Same as batch 1: corpus expansion, no new substrate. G1 (compile) + G3 (no-regression).

## Next steps

- [x] Batch 1: 19 rules (memory, compute, fusion, dispatch, parallelism) — Research 002
- [x] Batch 2: 7 rules (flash-attention tiling) — this note
- [ ] Batch 3: linear-attention recurrence (DeltaNet, GSA, RWKV7)
- [ ] Batch 4: MoE dispatch + expert routing (momoe_tl.py)
- [ ] Batch 5: NSA (block-sparse selection)
- [ ] Batch 6+: remaining kernels (losses, activations, normalization variants)

## References

1. **popcorn** — `/Users/katopz/git/popcorn/src/popcorn/impls/wall_attn_tl.py`
2. **FlashAttention** — Dao et al., 2022, "FlashAttention: Fast and Memory-Efficient
   Exact Attention with IO-Awareness" ([arxiv 2205.14135](https://arxiv.org/abs/2205.14135))
3. **FlashAttention-2** — Dao, 2023, "FlashAttention-2: Faster Attention with Better
   Parallelism and Work Partitioning" ([arxiv 2307.08691](https://arxiv.org/abs/2307.08691))
4. **Research 002** — `riir-clippy/.research/002_popcorn_kernel_opt_distillation_batch1.md`
