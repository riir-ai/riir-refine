# Research 007 — Popcorn kernel_opt distillation: Batch 6 (varlen packing + chunked reductions, 3 rules)

**Status:** COMPLETE (batch 6 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** Research 005+006 follow-up. After exhausting the heavy impl files
(momoe_tl, nsa_tl, wall_attn_tl), this batch mines the smaller kernel files for
genuinely novel general techniques not already covered.
**Source:** `attn_varlen.py` (85 lines), `chunk_local_cumsum.py` (51 lines),
`forgetting_attn.py` (~55 lines).

## What this batch adds

The heavy impl files are now mined. The remaining ~90 kernel files mostly fall
into two camps:
1. **Already-covered patterns** (fusion, coalescing, autotune) applied to
   different domains (losses, activations, normalization variants)
2. **Domain-specific math** (Householder chains, specific attention variants)
   that aren't general GPU techniques

This batch extracts the **3 genuinely novel general techniques** found in the
smaller files:

## Batch 6 — 3 rules (varlen + chunked reductions)

| Rule ID | GPU Concept | Source |
|---|---|---|
| `varlen-packing` | pack variable-length sequences via cu_seqlens, no padding | `attn_varlen.py` |
| `chunk-reset-cumsum` | cumsum that resets at chunk boundaries | `chunk_local_cumsum.py` |
| `cumsum-additive-bias` | additive bias via cumulative sum of a gate | `forgetting_attn.py` |

## Per-rule source attribution

### varlen-packing
**Source:** `attn_varlen.py` (L30-58)
**Concept:** Instead of padding variable-length sequences to a common max length
(wasting compute on padding tokens), pack all sequences end-to-end into a single
1D buffer delimited by `cu_seqlens` (cumulative sequence length offsets). Each
sequence is attended independently by slicing `[cu_seqlens[i]:cu_seqlens[i+1]]`.
This eliminates all padding waste — a batch of [128, 200, 64, 300] tokens
becomes a 692-element buffer, not a 4×300 padded matrix (1200 elements, 42% padding).

### chunk-reset-cumsum
**Source:** `chunk_local_cumsum.py` (L25-40)
**Concept:** Cumulative sum that resets at every chunk boundary —
`y_t = sum(g[chunk_start..=t])` where `chunk_start = (t // chunk_size) * chunk_size`.
This is the foundation of chunked linear-attention algorithms: within each chunk,
the cumsum accumulates locally; across chunks, it resets. Implemented by splitting
the tensor into chunks, running cumsum per-chunk, then concatenating.

### cumsum-additive-bias
**Source:** `forgetting_attn.py` (L29-34)
**Concept:** Apply an additive bias to attention scores via the cumulative sum of
a per-position gate: `scores[i,j] += cumsum(g)[i] - cumsum(g)[j]`. This creates
a learned positional decay — earlier positions relative to the query get less
weight. The key insight is that the bias is expressed as a *difference* of two
cumsums, so it's O(N) to compute (one cumsum) rather than O(N²) (a full bias matrix).

## What was NOT included (assessed + dropped)

| Kernel | Pattern | Why dropped |
|---|---|---|
| `path_attn.py` | Householder chain on keys | Domain-specific math (PaTH attention), not a general GPU technique |
| `chunk_global_cumsum.py` | plain cumsum | Already a primitive in every framework; no GPU-specific optimization |
| `group_norm.py` | normalization | Delegates to fla/liger; no inline kernel to distill |
| `kl_div.py`, `jsd.py`, `tvd.py`, `linear_kl_div.py` | loss fusion | Same fusion pattern as `fuse-xent-logsumexp` (Batch 1) |
| `embedding.py` | gather + atomic grad | Gather covered by memory patterns; atomic grad by `atomic-scatter-reduction` (Batch 4) |
| `grpo_offpolicy.py` | PPO clip + KL | Domain-specific RL math; the GPU technique is fusion (already covered) |

## GOAT gate for this batch

Same as batches 1-5: corpus expansion, no new substrate. G1 (compile) + G3
(no-regression).

## Next steps

- [x] Batch 1: 19 rules (memory, compute, fusion, dispatch, parallelism) — Research 002
- [x] Batch 2: 7 rules (flash-attention tiling) — Research 003
- [x] Batch 3: 6 rules (linear-attention recurrence) — Research 004
- [x] Batch 4: 9 rules (MoE dispatch + backward pass) — Research 005
- [x] Batch 5: 4 rules (NSA block-sparse selection) — Research 006
- [x] Batch 6: 3 rules (varlen + chunked reductions) — this note
- [ ] Batch 7+: the heavy impl files are exhausted; remaining kernels are
      mostly already-covered patterns or domain-specific math. Future batches
      would need new impl files or a different source (e.g. FlashInfer,
      xformers). Recommend pausing distillation until a new source lands.
