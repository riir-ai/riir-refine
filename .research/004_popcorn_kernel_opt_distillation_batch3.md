# Research 004 — Popcorn kernel_opt distillation: Batch 3 (linear-attention recurrence, 6 rules)

**Status:** COMPLETE (batch 3 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** Research 002-003 follow-up. Directly relevant to the DeltaNet work in
riir-ai (Issues 652, 658, 659 — DeltaNet prefill performance).
**Source:** `delta_rule.py`, `gsa.py`, `gated_delta_product.py`, `lightning_attn.py`,
`rwkv7.py`, `hgrn.py`.

## What linear-attention recurrence is

Standard softmax attention is O(N²) in sequence length. Linear attention replaces
the softmax with a kernel feature map, making it O(N) — but introduces a
**state-carrying sequential recurrence**:

```
S_t = f(S_{t-1}, k_t, v_t)    # state update (sequential — each step needs S_{t-1})
o_t = q_t^T S_t                # state query
```

This sequential dependency is the fundamental bottleneck (Issue 652, Issue 658).
The optimization patterns for state-carrying loops are distinct from the
flash-attention patterns in Batch 2 — they're about **chunkwise parallelism**,
not block streaming.

## Batch 3 — 6 rules (linear-attention recurrence)

| Rule ID | GPU Concept | Source |
|---|---|---|
| `state-forget-gate` | exponential decay gate on recurrent state | gsa.py, gated_delta_product.py |
| `householder-rank1-update` | delta rule as rank-1 Householder reflection | delta_rule.py |
| `chunkwise-parallel-recurrence` | sequential loop → chunk-parallel within blocks | fla chunk_delta_rule |
| `recurrent-vs-chunk-dispatch` | short-seq recurrent vs long-seq chunk backend | fla fused_recurrent vs chunk |
| `multi-write-per-step` | N Householder writes per timestep | gated_delta_product.py |
| `chained-two-pass-recurrence` | two sequential recurrences with softmax bridge | gsa.py (K-state then V-state) |

## GOAT gate

Same as batches 1-2: corpus expansion, no new substrate. G1 (compile) + G3 (no-regression).

## Next steps

- [x] Batch 1: 19 rules (memory, compute, fusion, dispatch, parallelism) — Research 002
- [x] Batch 2: 7 rules (flash-attention tiling) — Research 003
- [x] Batch 3: 6 rules (linear-attention recurrence) — this note
- [ ] Batch 4: MoE dispatch + expert routing (momoe_tl.py, 43KB)
- [ ] Batch 5: NSA (block-sparse selection, nsa_tl.py)
- [ ] Batch 6+: remaining kernels (losses, activations, normalization variants)

## References

1. **popcorn** — `/Users/katopz/git/popcorn/src/popcorn/kernels/{delta_rule,gsa,gated_delta_product}.py`
2. **DeltaNet** — Yang et al., 2024 ([arxiv 2406.06484](https://arxiv.org/abs/2406.06484))
3. **DeltaProduct** — Siems et al., 2025 ([arxiv 2502.10297](https://arxiv.org/abs/2502.10297))
4. **Gated Slot Attention** — Zhang et al., 2024 ([arxiv 2409.07146](https://arxiv.org/abs/2409.07146))
5. **Issue 652** — `riir-ai/.issues/652_deltanet_chunkwise_parallel_prefill.md` (CLOSED — chunkwise
   parallel failed the 3× gate because the recurrence IS the Delta rule, not standard linear attention)
