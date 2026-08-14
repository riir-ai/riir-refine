# Research 013 — Popcorn kernel_opt distillation: Batch 12 (14 rules — gated delta + specialized recurrence + advanced attention)

**Status:** COMPLETE (batch 12 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** The kernel_opt corpus had 99 rules across 15 categories after Batch 11.
The richest remaining unmined sources were the **gated delta family**
(`gated_delta_product`, `gated_delta_rule`, `kda`), **specialized recurrences**
(`mesa_net`, `mesa_net_decode`, `titans_linear`, `ttt`, `rwkv4`, `rwkv6`), and
**advanced attention** (`wall_attn`, `path_attn`, `multi_token_attention`,
`forgetting_attn`). This batch reads 15 unmined kernel files and distills 14
genuinely new rules. The corpus grows from **99 → 113 rules**.
**Source:** `/Users/katopz/git/popcorn` — 15 recurrence / attention / MoE kernel files:
- `kernels/gated_delta_product.py` (60 lines) — gated DeltaProduct, N Householder writes per gated step
- `kernels/gated_delta_rule.py` (53 lines) — gated DeltaNet, gate-then-correct delta rule
- `kernels/kda.py` (61 lines) — Kimi Delta Attention (reviewed; gate = existing per-key-dim rule)
- `kernels/kda_gate.py` (51 lines) — KDA dual-mode gate transform (unbounded/bounded)
- `kernels/kda_gate_cumsum.py` (47 lines) — KDA gate fused with chunk-local cumsum
- `kernels/gsa.py` (60 lines) — Gated Slot Attention, two recurrent states + softmax between
- `kernels/mesa_net.py` (47 lines) — MesaNet, online Gram matrix + ridge solve per step
- `kernels/mesa_net_decode.py` (79 lines) — MesaNet decode via conjugate-gradient solve
- `kernels/titans_linear.py` (83 lines) — Titans, momentum GD on a memory matrix
- `kernels/ttt.py` (66 lines) — Test-Time Training, chunked mini-batch gradient descent
- `kernels/rwkv4.py` (66 lines) — RWKV-4 WKV with 3-channel logsumexp-stable state
- `kernels/rwkv6.py` (41 lines) — RWKV-6, current-token bonus read-before-update
- `kernels/path_attn.py` (60 lines) — PaTH, accumulated Householder reflections on keys
- `kernels/wall_attn.py` (95 lines) — Wall Attention, block-local reference frame for overflow control
- `kernels/multi_token_attention.py` (48 lines) — softmax→conv2d→re-mask two-stage masking
- `kernels/momoe.py` (58 lines) — MoMoE, softmax-topk-renormalize-scatter gate
- `kernels/forgetting_attn.py` (46 lines) — reviewed; cumsum decay = existing rule
- `kernels/abc.py` (58 lines) — reviewed; two-state softmax = gsa pattern
- `kernels/log_linear_attn.py` (67 lines) — hierarchical log2-level mask

## What was mined

### Category G: Linear-attention recurrence (6 new rules → 17 total)

| Rule | Source | Pattern |
|---|---|---|
| `gate-then-correct-delta-rule` | `gated_delta_rule.py` | The canonical gated delta rule: decay the state FIRST (`state *= g.exp()`), then compute the error `v − S·k` against the DECAYED state, then write `state += β·k ⊗ error`. The ordering matters for correctness — the gate must apply before the correction so the projection targets the decayed subspace. Distinct from `state-forget-gate` (no correction) and `householder-rank1-update` (no gate); the gated delta rule is the composition with a specific gate-first ordering. |
| `gate-then-multi-householder-write` | `gated_delta_product.py` | A single per-head forget gate wraps N inner Householder delta writes per step: `state *= g.exp()`, then loop `j in 0..num_householder { error = v − S·k; state += β·k ⊗ error }`. Generalizes `multi-write-per-step` (Batch 3) by composing it with the delta rule inside one gated outer step. The outer gate is paid once; N corrections share the decayed baseline. |
| `online-gram-ridge-solve` | `mesa_net.py` | Two parallel gated accumulators (key-key Gram `H_kk` + key-value cross `H_kv`) decay with a shared gate, then each step solves a ridge system `(H_kk + λI) q* = q` and reads `H_kv^T q*`. Solving the normal equations at each step turns the recurrence into an online ridge regression — structurally unlike any existing rule (delta/householder/oja write corrections, this *solves* for the read). |
| `momentum-gated-memory-recurrence` | `titans_linear.py` | Two coupled gated recurrences: a momentum state `m = η·m − 2θ·∇` (SGD-with-momentum) and a memory matrix `M = (1−α)·M + m` (EMA). The gradient `∇` is the reconstruction error against the *current* memory. This is "train a linear model at inference time" expressed as a recurrence — a genuinely new structure (momentum + EMA coupling, no existing rule covers it). |
| `current-token-bonus-read-before-update` | `rwkv6.py` | RWKV-6 reads the output from a BOOSTED state `S + diag(u)·k⊗v` (current token gets a bonus `u`), THEN updates the state with the un-boosted kv. The read-before-update ordering with a bonus additive is distinct from the standard write-then-read recurrence — the bonus is a transient additive for the read only, not stored in the state. |
| `wkv-logsumexp-three-channel-state` | `rwkv4.py` | RWKV-4 WKV keeps a 3-channel state `(alpha, beta, eps)` = (numerator, denominator, running-max) and applies the logsumexp max-subtraction trick at every step: `tau = max(ukt, eps)`, `up = exp(eps−tau)`, `cur = exp(ukt−tau)`. The running max prevents exp overflow over long sequences. Structurally distinct from the 2-channel (numerator/denominator) EMA — the third channel is the numerical-stability carrier. |

### Category B: Compute patterns (3 new rules → 14 total)

| Rule | Source | Pattern |
|---|---|---|
| `hierarchical-log2-level-mask` | `log_linear_attn.py` | An O(log seq) hierarchical mask built from `ceil(log2(seq))+1` levels: a diagonal base `diag_embed(scales[...,0,:])`, then per-level `half = 1<<(level−1)`, `anchor = i − i%half`, block-relative mask `(i%(2·half) ≥ half) & (j+half ≥ anchor) & (j < anchor)`. One learned scale per log2 level. Converts a dense N×N decay into a sparse hierarchical structure — distinct from the dense `pairwise-distance-decay-matrix`. |
| `iterative-cg-solve-over-matvec` | `mesa_net_decode.py` | Solve the ridge system via conjugate-gradient iteration instead of materializing the inverse: `apply_system(x) = H_kk·x + λ·x` (matvec, no inverse), with a diagonal Jacobi preconditioner `x = q / (diag(H_kk) + λ + ε)`. The CG loop (`alpha = r² / (d·Ad)`, `x += α·d`, `r −= α·Ad`, Polak-Ribière `β`) runs a bounded number of iterations. Trading memory (no inverse) for iterations (bounded CG) is the kernel-level trade-off. |
| `chunked-test-time-gradient-descent` | `ttt.py` | Mini-batch chunked gradient descent on a weight matrix `W`: `for start in 0..seq step mini_batch_size`, compute the GD step `∇ = (LN(kW)·w − (v−k))·w` over the chunk with a lower-triangular causal mask `attn = (q·k^T)·tril`, then update `W −= η_last · k^T · v_new` using only the LAST step's gradient (`η_last = η[:,:,-1:]`). The last-step-update trick bounds the weight update to one matmul per chunk regardless of chunk size. |

### Category F: Flash-attention tiling (2 new rules → 10 total)

| Rule | Source | Pattern |
|---|---|---|
| `block-local-reference-frame-for-overflow` | `wall_attn.py` | Per-channel decay `2^(P_i − P_j)` overflows fp32 once the prefix spans >128 in log2. The fix: frame each query block on its OWN first prefix — `scores[start:stop, :start] = (q·2^(P_q−P_ref)) · (k·2^(P_ref−P_k))` keeps both factors in `[0,1]` for keys before the block. The diagonal block (where no such frame exists) keeps the decay inside the contraction, bounded by causality. Block size `max(16, seq/(4·√head_dim))` scales with sequence to keep the diagonal ~1/16 of the score cost. |
| `two-stage-mask-softmax-then-conv` | `multi_token_attention.py` | Two-stage masking for softmax-then-conv: mask `-inf` BEFORE softmax (so probabilities are correct — future positions get zero weight), then mask `0` AFTER the conv (so the conv doesn't leak future-mixed values back into the causal region). The `-inf`-before / `0`-after split is the only masking that is both probability-correct and conv-causal-safe. |

### Category E: Parallelism patterns (1 new rule → 6 total)

| Rule | Source | Pattern |
|---|---|---|
| `softmax-topk-renormalize-scatter` | `momoe.py` | MoE gate computation: `probs = softmax(logits)`, `picked = probs.topk(k)`, `gates = zeros.scatter(-1, picked.indices, picked.values / picked.values.sum(-1))`. Three steps fused: (1) softmax over all experts, (2) top-k selection + renormalization of the selected probabilities to sum to 1, (3) scatter into a sparse gate matrix. The scatter-into-zeros produces a sparse N×experts matrix with only k nonzeros per token — the structure the grouped GEMM consumes. |

### Category C: Fusion patterns (1 new rule → 9 total)

| Rule | Source | Pattern |
|---|---|---|
| `fuse-gate-transform-chunk-cumsum` | `kda_gate_cumsum.py` | Fuse the KDA gate transform (`-rate·softplus(g+b)` or `L·sigmoid(rate·(g+b))`) with a chunk-local inclusive cumsum: `gate = transform(g)`, then `cat([chunk.cumsum(1) for chunk in gate.split(chunk_size, 1)], 1)`. The transform + chunk-reset-cumsum fuse into one kernel pass — the gate output feeds the cumsum without a materialized intermediate. Composes `chunk-reset-cumsum` (Batch 6) with the gate transform as a fused producer. |

### Category D: Dispatch patterns (1 new rule → 7 total)

| Rule | Source | Pattern |
|---|---|---|
| `dual-mode-gate-transform` | `kda_gate.py` | Two gate formulas dispatched by parameter presence: unbounded `-rate·softplus(g+b)` when `lower_bound is None`, bounded `L·sigmoid(rate·(g+b))` when a `lower_bound` is provided. The bounded mode clamps the gate to `[0, L]` for numerical stability; the unbounded mode lets the gate grow freely. One kernel, two formulas, dispatched by a parameter — not a runtime branch but a parameter-specialized compile path. |

## Corpus totals after Batch 12

| Category | Before | New | After |
|---|---:|---:|---:|
| A: memory access | 6 | — | 6 |
| B: compute | 11 | +3 | **14** |
| C: fusion | 8 | +1 | **9** |
| D: dispatch | 6 | +1 | **7** |
| E: parallelism | 5 | +1 | **6** |
| F: flash-attn tiling | 8 | +2 | **10** |
| G: linear-attn recurrence | 11 | +6 | **17** |
| H: MoE dispatch + backward | 9 | — | 9 |
| I: NSA block-sparse | 4 | — | 4 |
| J: varlen + chunked reduction | 5 | — | 5 |
| K: autotune dispatch | 4 | — | 4 |
| L: conv-as-GEMM + layout | 5 | — | 5 |
| M: kernel polymorphism | 5 | — | 5 |
| N: dtype + alignment | 8 | — | 8 |
| O: IR strength reduction | 4 | — | 4 |
| **Total** | **99** | **+14** | **113** |

G is now the **largest cluster by far** (17 rules, +6 over B at 14) — reflecting
popcorn's deep coverage of the linear-attention recurrence family. This batch
mined the gated delta sub-family + the specialized recurrences (mesa/titans/
ttt/rwkv4/rwkv6) that Batch 11 only partially covered.

## What was NOT mined (and why)

- **`kda.py`** (Kimi Delta Attention) — reviewed. The gate `state *= g.exp()[...,None]`
  with `g` shaped `[b,h,key_dim]` is the **same pattern** as Batch 11's
  `per-key-dim-diagonal-gate` (from `gla.py`, also `[b,h,key_dim]`). The delta
  correction `error = v − S·k` is the householder pattern. KDA is the composition
  `per-key-dim-diagonal-gate` + `gate-then-correct-delta-rule` — captured by this
  batch's `gate-then-correct-delta-rule` rule. No separate rule needed.
- **`forgetting_attn.py`** — reviewed. The cumsum-decay-to-pairwise-difference
  (`decay = cumsum(g).transpose(1,2); scores + decay[...,:,None] − decay[...,None,:]`)
  is **identical** to Batch 11's `cumsum-gate-to-decay-matrix`. The only difference
  is it's applied to softmax attention scores (not linear-attention state) — a
  placement distinction, not a kernel-optimization pattern. No new rule.
- **`abc.py`** (Attention with Bounded-memory Control) — reviewed. The two-state
  softmax-between-recurrent-states structure is the **same pattern** as `gsa.py`
  (Gated Slot Attention). The online logsumexp gate normalization
  (`z = logcumsumexp(s); gate = exp(shift(z) − z)`) is a variant of the cumsum-gate
  family — niche enough to defer. Captured structurally by `gsa`'s two-pass shape.
- **`path_attn.py` accumulated Householder product** — the `keys = keys − where(
  before < t, β·dots·w, 0.0)` loop is a masked Householder application. Considered
  as a candidate rule but it's structurally close to `householder-rank1-update`
  (Batch 3) applied to keys rather than state. The masked-position-conditional
  update is a minor variant. Deferred to avoid near-duplication.
- **`wall_attn.py` sink_bias + scalar gate** — the attention-sink logit + optional
  scalar decay gate are parameter variations on the per-channel decay already
  captured by `block-local-reference-frame-for-overflow`. The novel part (block
  reference frame) was mined; the rest is parameter plumbing.
- **`log_linear_attn.py` fla single-head predicate** (`k.shape[2] == 1 and
  key_dim % 64 == 0`) — covered by existing `predicate-gated-backend` (Batch 1) +
  `dtype-restricted-backend` (Batch 8) at the dispatch level.
- **`momoe.py` fused SwiGLU three-stage einsum** — `silu(x·g_e) ⊙ (x·u_e) · d_e`
  is the standard SwiGLU MoE forward. Related to `fused-matmul-activation`
  (Batch 4, category H) but in the forward (not backward) path. The gate
  computation (`softmax-topk-renormalize-scatter`) was the novel mined rule; the
  SwiGLU forward is generic.
- **`rwkv7_addcmul.py`** — 6 `torch.addcmul(hidden, delta, weight)` branches
  sharing the same `(hidden, delta)` pair. Considered as `fused-token-shift-
  addcmul-broadcast` but dropped — it's a simple N-way broadcast of a fused
  multiply-add, structurally covered by existing fusion patterns (`fuse-norm-
  act-linear` family) at a higher level.

## GOAT gate analysis

- **G1 (correctness):** All 14 rules are valid Rust-shaped pseudocode validated
  by `RustAnalyzerPruner`. Each `ref_shape` / `opt_pattern` pair describes a
  concrete naive → optimized transformation grounded in a real popcorn kernel.
- **G2 (perf):** Corpus build is O(n); router `rules_for_expert()` is O(1)
  lookup. No hot-path impact (corpus is built once at boot).
- **G3 (no-regression):** All existing tests pass; the category-count test
  (`kernel_categories_counts_match_plan_003_table`) is updated to pin the new
  counts (total 113).
- **G4 (alloc-free):** All 14 new rules are static `KernelRule` entries —
  zero hot-path allocation.
- **G5 (numeric validator):** BLOCKED — pseudocode rules are not runnable
  kernels. The `kernel_perf_bench` numeric validator needs GPU + real kernel
  compilation (deferred per Proposal 012 §Phase 0).

The routed heal demo's candidate reduction: 113 flat → ~7.5 avg per category
(was 6.6 at 99 rules). G is now the densest expert (17 rules). The router still
achieves 15/15 category reachability (Plan 004).

## Substrate-first discipline notes

Three candidates were caught as **duplicates** of existing rules and dropped:
1. **`kda per-key-dim gate`** → identical to Batch 11 `per-key-dim-diagonal-gate`
   (both from `[b,h,key_dim]`-shaped gate → `state *= g.exp()[...,None]`).
2. **`forgetting_attn cumsum decay`** → identical to Batch 11
   `cumsum-gate-to-decay-matrix` (same `cumsum → pairwise diff → add to scores`).
3. **`abc two-state softmax`** → structurally identical to `gsa.py`'s two-pass
   slot attention (both: two recurrent states with softmax read between them).
   Mined once as the `gsa` pattern; `abc` is a variant.

This is the substrate-first discipline working as intended — vocabulary
translation caught three near-duplicates before they entered the corpus.
