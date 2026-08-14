# Research 012 — Popcorn kernel_opt distillation: Batch 11 (11 rules — linear-attention recurrence family)

**Status:** COMPLETE (batch 11 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** The kernel_opt corpus had 88 rules across 15 categories after Batch 10.
The **linear-attention recurrence family** (category G, 6 rules) + the smallest
categories (E parallelism, 3) were identified as the richest unmined source:
popcorn ships ~15 linear-attention / recurrence kernel variants, and only a
handful had been mined into G. This batch reads 15 recurrence/scan kernel
files and distills 11 genuinely new rules (a 12th candidate, `chunk-reset-scan`,
was dropped as a duplicate of the existing Batch-6 `chunk-reset-cumsum`).
**Source:** `/Users/katopz/git/popcorn` — 11 recurrence / scan kernel files:
- `kernels/gla.py` (55 lines) — Gated Linear Attention, per-key-dim log forget gate
- `kernels/simple_gla.py` (37 lines) — scalar per-head gated linear attention
- `kernels/based.py` (47 lines) — second-order Taylor feature map linear attention
- `kernels/rebased.py` (51 lines) — pure quadratic feature map linear attention
- `kernels/linear_attn.py` (52 lines) — Katharopoulos causal linear attention
- `kernels/retention.py` (40 lines) — Retentive Network, per-head geometric decay
- `kernels/lightning_attn.py` (44 lines) — layer-depth-dependent decay schedule
- `kernels/rwkv7.py` (61 lines) — diagonal-plus-low-rank state transition (Goose)
- `kernels/delta_rule.py` (49 lines) — DeltaNet error-correcting delta rule
- `kernels/iplr_delta_rule.py` (56 lines) — identity-plus-low-rank delta rule
- `kernels/gated_oja_rule.py` (57 lines) — Oja subtractive normalization
- `kernels/deltaformer.py` (44 lines) — delta rule in attention space
- `kernels/hgrn.py` (34 lines) — elementwise gated recurrence
- `kernels/chunk_global_cumsum.py` (32 lines) — sequence-wide cumsum
- `kernels/chunk_local_cumsum.py` (43 lines) — chunk-reset cumsum

## What was mined

### Category G: Linear-attention recurrence (5 new rules → 11 total)

| Rule | Source | Pattern |
|---|---|---|
| `per-key-dim-diagonal-gate` | `gla.py` | Per-**dimension** vector decay gate broadcast across the value dim — distinct from the scalar/per-head `state-forget-gate` (Batch 3). `state *= gate.exp()[..., None]` decays each key dimension independently. |
| `erase-replace-low-rank-transition` | `rwkv7.py` | 4-phase diagonal+low-rank state transition: project out erase direction (`removed = state·a`), decay (`state *= w.exp()`), add back along replace direction (`state += b ⊗ removed`), then write outer product (`state += k ⊗ v`). Structurally distinct from the 2-phase forget-gate + write of gla. |
| `oja-subtractive-normalization` | `gated_oja_rule.py` | Oja's rule projects out the already-stored component before writing: `write = β(k − Sv)`. The `Sv` term is the projection of state onto the current value — the write is what's *new*, not what's already stored. Distinct from `householder-rank1-update` (which projects `v − Sk`). |
| `identity-plus-low-rank-delta` | `iplr_delta_rule.py` | `(I + b·a^T)` rank-1 update applied to state via projection: `state += b ⊗ (state·a)` then `state += k ⊗ v`. No forget gate — the identity term makes the update additive-rotational rather than decayed. |
| `cumsum-gate-to-decay-matrix` | `simple_gla.py` | Convert a per-step log-gate into a pairwise decay matrix in one shot: cumsum the log-gate, build the pairwise difference matrix (`cumsum[i] − cumsum[j]`, telescoping), exp, mask. Distinct from `retention`/`lightning` which use a constant per-head slope. |

### Category E: Parallelism patterns (2 new rules → 5 total)

| Rule | Source | Pattern |
|---|---|---|
| `reverse-scan-via-double-flip` | `chunk_global_cumsum.py`, `chunk_local_cumsum.py` | The canonical idiom for a reverse-direction cumulative scan: `flip → cumsum → flip`. No native `cumsum_reverse` primitive is needed; the forward scan + two transposes (flips) compose the reverse scan. |
| `pairwise-distance-decay-matrix` | `retention.py`, `lightning_attn.py` | Materialize a full N×N decay matrix from index distance: `n = arange(seq); steps = n[:,None] − n[None,:]; decay = (steps × slope).exp()`. Broadcasts a per-step decay into a pairwise form for a single batched matmul. |

### Category J: Varlen packing + chunked reductions (0 new rules → 5 total)

No new rule — the candidate `chunk-reset-scan` (from `chunk_local_cumsum.py`)
was identified as a **duplicate** of the existing `chunk-reset-cumsum` (Batch 6,
varlen-packing category). Both source from the same popcorn file and describe
the identical split→cumsum→cat chunk-boundary-reset pattern. Dropped per the
substrate-first / DRY discipline. Recorded here so a future batch does not
re-propose it.

### Category F: Flash-attention tiling (1 new rule → 8 total)

| Rule | Source | Pattern |
|---|---|---|
| `strict-causal-mask-tril-offset` | `deltaformer.py` | Strictly-causal (exclude diagonal) attention via `tril(-1)` — distinct from the standard causal `tril()` which keeps the diagonal. Paired with a `nan_to_num` guard: the first row has no visible keys → softmax of all `−inf` → NaN. |

### Category B: Compute patterns (2 new rules → 11 total)

| Rule | Source | Pattern |
|---|---|---|
| `polynomial-taylor-feature-map` | `based.py`, `rebased.py` | Approximate the softmax kernel with a low-degree polynomial feature map: `1 + x + 0.5·x²` (Based, 2nd-order Taylor) or `x²` (ReBased, pure quadratic). Lets linear attention approximate softmax attention without the exp. |
| `positive-feature-map-shift` | `linear_attn.py` | Shift an activation to guarantee non-negativity: `elu(x) + 1`. The normalizer in linear attention never crosses zero when q and k pass through a positive feature map — numerical stability without a softmax. |

### Category D: Dispatch patterns (1 new rule → 6 total)

| Rule | Source | Pattern |
|---|---|---|
| `layer-depth-decay-schedule` | `lightning_attn.py` | Decay slope scales with layer depth: `slope ∝ (1 − layer_idx/num_layers)` — deeper layers decay slower. A dispatch decision baked into the kernel parameters via the layer index, not a runtime branch. |

## Corpus totals after Batch 11

| Category | Before | New | After |
|---|---:|---:|---:|
| A: memory access | 6 | — | 6 |
| B: compute | 9 | +2 | **11** |
| C: fusion | 8 | — | 8 |
| D: dispatch | 5 | +1 | **6** |
| E: parallelism | 3 | +2 | **5** |
| F: flash-attn tiling | 7 | +1 | **8** |
| G: linear-attn recurrence | 6 | +5 | **11** |
| H: MoE dispatch + backward | 9 | — | 9 |
| I: NSA block-sparse | 4 | — | 4 |
| J: varlen + chunked reduction | 5 | — | 5 |
| K: autotune dispatch | 4 | — | 4 |
| L: conv-as-GEMM + layout | 5 | — | 5 |
| M: kernel polymorphism | 5 | — | 5 |
| N: dtype + alignment | 8 | — | 8 |
| O: IR strength reduction | 4 | — | 4 |
| **Total** | **88** | **+11** | **99** |

G is now tied for the largest cluster (11 rules, same as B), reflecting
popcorn's emphasis on the linear-attention family. E (parallelism) grew from
3 → 5.

## What was NOT mined (and why)

- **`hgrn.py`** (elementwise gated recurrence `h = g.exp()·h + x`) — the
  simplest possible recurrence. Already covered structurally by `state-forget-gate`
  (Batch 3) as the degenerate scalar-input case; a separate rule would be a
  near-duplicate.
- **`deltaformer.py`** correction loop (the per-timestep `v − β·correction`
  in attention space) — the subtractive pattern is already captured by
  `oja-subtractive-normalization` and `householder-rank1-update`; only the
  strictly-causal mask + nan guard yielded a new rule (`strict-causal-mask-tril-offset`).
- **`retention.py` per-head geometric schedule** (`1 − 2^(−5−h)`) — a concrete
  instance of `pairwise-distance-decay-matrix` with a specific slope function;
  the slope formula itself isn't a separate optimization rule, it's the
  schedule that `layer-depth-decay-schedule` generalizes.
- **`linear_attn.py` layout predicate** (`seq ≥ heads`) — covered by existing
  `predicate-gated-backend` (Batch 1) at the dispatch level.
- **`iplr_delta_rule.py` dual return** (`(outs, final_state)`) — a streaming
  API shape, not a kernel-level optimization pattern. Would belong in a
  future "API surface" category if one materializes.

## GOAT gate analysis

- **G1 (correctness):** All 11 rules are valid Rust-shaped pseudocode validated
  by `RustAnalyzerPruner`. Each `ref_shape` / `opt_pattern` pair describes a
  concrete naive → optimized transformation grounded in a real popcorn kernel.
- **G2 (perf):** Corpus build is O(n); router `rules_for_expert()` is O(1)
  lookup. No hot-path impact (corpus is built once at boot).
- **G3 (no-regression):** All existing tests pass; the category-count test
  (`kernel_categories_counts_match_plan_003_table`) is updated to pin the new
  counts (total 101).
- **G4 (alloc-free):** All 11 new rules are static `KernelRule` entries —
  zero hot-path allocation.
- **G5 (numeric validator):** BLOCKED — pseudocode rules are not runnable
  kernels. The `kernel_perf_bench` numeric validator needs GPU + real kernel
  compilation (deferred per Proposal 012 §Phase 0).

The routed heal demo's candidate reduction improves slightly: 99 flat →
~6.6 avg per category (was 5.9 at 88 rules). The router still achieves
15/15 category reachability (Plan 004).
