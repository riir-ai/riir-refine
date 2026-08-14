# Research 006 — Popcorn kernel_opt distillation: Batch 5 (NSA block-sparse selection, 4 rules)

**Status:** COMPLETE (batch 5 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** Research 002 batch 1 follow-up. Native Sparse Attention (NSA) has a
distinct pattern: block-sparse selection + two-level attention (coarse compression
→ fine selection). This batch distills the patterns that are NOT already covered
by Batch 2 (flash-attention tiling) — specifically the routing + hierarchy
patterns.
**Source:** `/Users/katopz/git/popcorn/src/popcorn/impls/nsa_tl.py` (586 lines,
Native Sparse Attention — block-sparse attention with compression-based top-k
block selection).

## What NSA adds beyond flash-attention

Flash-attention (Batch 2) streams K/V blocks through shared memory, attending to
ALL blocks in sequence. NSA goes further: it doesn't attend to all blocks. It
compresses K/V into coarse blocks, scores block relevance on the compressed reps,
then attends only to the top-k selected blocks at full resolution. This is
**block-sparse attention** — the routing decision is made on cheap compressed
representations, then the expensive full-res attention only runs on the winners.

This batch distills the 4 patterns that are unique to NSA (not in Batch 2):

## Batch 5 — 4 rules (block-sparse selection + attention hierarchy)

| Rule ID | GPU Concept | Source |
|---|---|---|
| `block-sparse-gather` | block-sparse indexed gather via selection array | `_sel_attn_fwd_kernel` (L100-145) |
| `precompute-scalar-reduction` | hoist invariant row reduction out of per-block loop | `_sel_attn_bwd_preprocess_kernel` (L186-232) |
| `block-compression-before-routing` | pool to coarse blocks, route on compressed reps | `compression_attention` (L27-67) + `nsa` (L561-562) |
| `sliding-window-plus-sparse` | decompose attention into local + global sparse | `nsa` (L573-577) |

## Per-rule source attribution

### block-sparse-gather
**Source:** `_sel_attn_fwd_kernel` (L100-145)
**Concept:** Instead of iterating over all N key/value positions, iterate over a
precomputed `block_indices` array that selects only the relevant blocks. Each
index addresses a contiguous block, enabling coalesced memory access. The routing
decision (which blocks) is made outside the kernel; the kernel just consumes the
index array.

### precompute-scalar-reduction
**Source:** `_sel_attn_bwd_preprocess_kernel` (L186-232)
**Concept:** The backward kernel needs `delta = sum(o * do)` per query row, which
is invariant across all KV blocks. Precompute it in a separate lightweight kernel
pass and store it, rather than recomputing the dot product inside the per-block
inner loop. A general pattern: any invariant row reduction should be hoisted
out of a tiled loop.

### block-compression-before-routing
**Source:** `compression_attention` (L27-67) + `nsa` (L561-562)
**Concept:** Before running expensive selection over all N fine-grained positions,
compress K/V via mean-pooling into coarse blocks, run cheap attention on the
compressed representations to score block relevance, then attend only to the
top-k selected blocks at full resolution. This is a two-level hierarchy: cheap
coarse pass decides what the expensive fine pass works on.

### sliding-window-plus-sparse
**Source:** `nsa` (L573-577)
**Concept:** Decompose attention into two independent mechanisms — a local sliding
window (handles nearby context, computed by a dedicated dense kernel) and a
global block-sparse path (handles long-range, computed by the selection kernel).
Combine via a gated weighted sum, allowing the model to learn the balance between
local and global patterns. This is the "hybrid attention" pattern used by NSA,
Longformer, BigBird, etc.

## What was NOT included (deduplication vs Batch 2 + Batch 4)

Three patterns were identified in NSA but dropped as duplicates of earlier
batches:

| Pattern | Duplicate of | Why dropped |
|---|---|---|
| `online-softmax-one-pass` | `online-softmax-streaming` (Batch 2) | Same algorithm — running max + rescale |
| `exp2-base-conversion` | `exp2-log2-base2` (Batch 2) | Same technique — fold log2(e) into scale |
| `atomic-scatter-grad` | `atomic-scatter-reduction` (Batch 4) | Same technique — lock-free atomic add |

This deduplication keeps the corpus DRY. The same GPU technique should appear
once, with source attribution noting all the places it was found.

## How these map to KernelRule

Each rule is a `ref_shape` (naive Rust pseudocode) + `opt_pattern` (optimized
Rust pseudocode). `block-sparse-gather` shows a dense loop vs an index-array
loop; `block-compression-before-routing` shows per-position scoring vs
pooled-block scoring; `sliding-window-plus-sparse` shows a single attention pass
vs a two-path decomposition.

## GOAT gate for this batch

Same as batches 1-4: corpus expansion, no new substrate. G1 (compile) + G3
(no-regression).

## Next steps

- [x] Batch 1: 19 rules (memory, compute, fusion, dispatch, parallelism) — Research 002
- [x] Batch 2: 7 rules (flash-attention tiling) — Research 003
- [x] Batch 3: 6 rules (linear-attention recurrence) — Research 004
- [x] Batch 4: 9 rules (MoE dispatch + backward pass) — Research 005
- [x] Batch 5: 4 rules (NSA block-sparse selection) — this note
- [ ] Batch 6+: remaining kernels (losses, activations, normalization variants)
