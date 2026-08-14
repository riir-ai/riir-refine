# Research 011 — Popcorn kernel_opt distillation: Batch 10 (12 rules — compute + chunked reduction + polymorphism)

**Status:** COMPLETE (batch 10 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** The kernel_opt corpus had 76 rules across 15 categories with three
categories being the smallest: E (parallelism, 3), J (varlen + chunked reduction, 3),
M (kernel polymorphism, 3). This batch grows J and M to 5 each, plus adds genuinely
new patterns to B (compute), C (fusion), D (dispatch), and N (dtype/alignment).
**Source:** `/Users/katopz/git/popcorn` — 12 kernel files not yet mined:
- `kernels/mean_pooling.py` (24 lines) — chunked mean reduction
- `kernels/solve_tril.py` (36 lines) — block triangular inverse
- `kernels/hadamard_transform.py` (26 lines) — Sylvester Hadamard matrix
- `kernels/comba.py` (68 lines) — gated delta rule
- `kernels/dyt.py` (25 lines) — Dynamic Tanh normalization
- `kernels/token_shift.py` (19 lines) — RWKV token shift
- `kernels/sparsemax.py` (30 lines) — sparse softmax projection
- `kernels/selective_log_softmax.py` (35 lines) — selected-token log probs
- `kernels/softmax.py` (44 lines) — softmax with predicate gates
- `kernels/rms_norm.py` (59 lines) — RMS normalization
- `kernels/geglu.py` (28 lines) + `kernels/swiglu.py` (31 lines) — gated linear units
- `kernels/rope.py` (46 lines) — rotary positional embedding
- `kernels/gelu.py` (27 lines) — Gaussian Error Linear Unit

## What was mined

### Category B: Compute patterns (4 new rules → 9 total)

| Rule | Source | Pattern |
|---|---|---|
| `recursive-kronecker-cat` | `hadamard_transform.py` | Recursive `cat` to build a structured (Hadamard) matrix in O(log d) steps |
| `optional-bias-branch` | `dyt.py`, `rms_norm.py` | Optional bias add via `if bias.is_none()` branch — skip the add when bias is absent |
| `shift-via-pad-subtract` | `token_shift.py` | One-pass sequence shift via `pad(x, (...,1,-1)) - x` — avoids a separate write pass |
| `sorted-cumsum-threshold` | `sparsemax.py` | Sort + cumsum + threshold to find the support set for the Euclidean projection |

### Category C: Fusion patterns (1 new rule → 8 total)

| Rule | Source | Pattern |
|---|---|---|
| `gather-selective-reduction` | `selective_log_softmax.py` | `gather(-1, idx)` to read only the selected tokens instead of materializing the full softmax matrix |

### Category D: Dispatch patterns (1 new rule → 5 total)

| Rule | Source | Pattern |
|---|---|---|
| `multi-condition-predicate` | `softmax.py` `_transformer_engine_ready`, `rms_norm.py` `_aligned` | Compound (rows % 32 == 0 AND width in range AND width % 8 == 0) predicate gate |

### Category J: Varlen + chunked reduction (2 new rules → 5 total)

| Rule | Source | Pattern |
|---|---|---|
| `chunk-split-mean-pool` | `mean_pooling.py` | Chunked reduction via `x.split(C, dim).map(|c| c.mean(dim))` — process fixed chunks instead of one large reduction |
| `ragged-chunk-pad` | `solve_tril.py` | Pad the ragged last chunk to `block_size` so the fixed-size kernel works on all chunks uniformly |

### Category M: Kernel polymorphism (2 new rules → 5 total)

| Rule | Source | Pattern |
|---|---|---|
| `reshape-rank-for-backend` | `geglu.py`, `swiglu.py` (unsloth backends) | Reshape input to the backend's required rank (e.g. 3-D), run the kernel, reshape back — preserves the caller's rank contract |
| `detached-broadcast-insert` | `rope.py` | `.detach()` to mark a tensor non-differentiable + insert a broadcast dim — avoids autograd graph cost on the broadcast |

### Category N: dtype + alignment (2 new rules → 8 total)

| Rule | Source | Pattern |
|---|---|---|
| `batched-upcast-map` | `comba.py` | `map(upcast, (q, k, v, p, g, beta))` — batch upcast a tuple of tensors in one call instead of per-tensor |
| `dtype-restricted-backend` | `gelu.py` (fla), `sparsemax.py` (liger) | Register a backend only for precision-stable dtypes (`Float32 | BFloat16`, excluding fp16) — gradient precision varies by dtype |

## Per-rule source attribution

### recursive-kronecker-cat
**Source:** `hadamard_transform.py` — `hadamard_transform` reference.
**Pattern:** The Hadamard matrix H_d is built recursively via the Sylvester
construction: `H_{2d} = [[H_d, H_d], [H_d, -H_d]]`. The reference uses
`torch.cat([torch.cat([matrix, matrix], -1), torch.cat([matrix, -matrix], -1)], 0)`
in a while loop until `matrix.shape[-1] >= dim`. This is O(log d) cat operations
vs O(d²) explicit element assignment. The Kronecker structure means each cat
doubles the matrix size, so only log2(d) iterations are needed. The constraint
`hidden` is a power-of-two is enforced at the dim-type level (separate
`hadamard_dim` type), not by a runtime check.

### optional-bias-branch
**Source:** `dyt.py` — `dyt` reference; `rms_norm.py` — `rms_norm` reference.
**Pattern:** Both kernels accept an optional bias (`bias: Tensor | None`). The
optimized form uses `out if bias.is_none() else out + bias` — skipping the
allocation + elementwise add entirely when bias is absent. The naive form always
allocates a bias tensor (even if zero) and always adds. The branch is cheap
(a single `None` check) and the skipped add is O(n) with a memory round-trip.

### shift-via-pad-subtract
**Source:** `token_shift.py` — `token_shift` reference.
**Pattern:** RWKV token shift is `y_t = x_{t-1} - x_t` with `x_{-1} = 0`. The
reference uses `F.pad(x, (0, 0, 1, -1)) - x` — pad one zero at the start of the
seq dim, drop the last element, then subtract. This is one fused elementwise op
(the pad is a view, the subtract is one kernel launch) vs the naive two-pass
"allocate shifted buffer, then subtract". The pad trick works because the
padding is along the shift axis only; the feature axis padding is `(0, 0)`.

### sorted-cumsum-threshold
**Source:** `sparsemax.py` — `sparsemax` reference.
**Pattern:** Sparsemax projects onto the probability simplex. The algorithm:
(1) sort x descending, (2) compute `cumsum - 1`, (3) find the largest k where
`k * sorted_x[k] > cumsum[k]` (the support set), (4) compute `tau = cumsum[k] / k`,
(5) clamp `x - tau` to non-negative. The sort + cumsum + threshold is a
single-pass reduction structure (vs the naive iterative projection). The key
insight: the support set is determined by the sorted cumulative crossing point,
not by an iterative binary search.

### gather-selective-reduction
**Source:** `selective_log_softmax.py` — `selective_log_softmax` reference + `liger` backend.
**Pattern:** GRPO loss needs log-probabilities of the *selected* tokens only,
not the full vocab distribution. The naive form computes the full softmax then
indexes. The optimized form uses `scaled.gather(-1, input_ids[..., None])` to
read only the selected logits, then subtracts `logsumexp(scaled)` (a single
reduction over the vocab). This avoids materializing the full `(batch, seq, vocab)`
softmax matrix — only the selected `(batch, seq)` log-probs are written. The
gather is O(batch × seq) instead of O(batch × seq × vocab).

### multi-condition-predicate
**Source:** `softmax.py` — `_transformer_engine_ready` predicate; `rms_norm.py` — `_aligned` predicate.
**Pattern:** Backends that require specific alignment launch on a compound
predicate. `softmax.py`'s transformer-engine backend requires
`rows % 32 == 0 AND 16 < width < 16_384 AND width % 8 == 0` (three conditions:
row alignment for tile launch, width bounds for register pressure, width
divisibility for vectorized load). `rms_norm.py`'s quack backend requires
`dtype == f32 OR shape[-1] % 8 == 0` (dtype OR alignment — the cute kernels
miscompile odd widths for 16-bit dtypes). The pattern: encode multiple
shape/dtype conditions into a single predicate function, and only register the
fast backend when all hold. The naive form dispatches the fast backend
unconditionally and miscompiles on the edge cases.

### chunk-split-mean-pool
**Source:** `mean_pooling.py` — `mean_pooling` reference + `fla` backend.
**Pattern:** Mean-pool consecutive non-overlapping chunks of size C. The
reference uses `torch.stack([chunk.mean(1) for chunk in x.split(C, 1)], 1)` —
split the seq dim into C-sized chunks, mean each chunk over its seq axis, stack
back. The naive form would iterate element-by-element accumulating into a
running sum. The split-then-mean pattern exposes the chunk structure to the
compiler (fixed chunk size = unrollable inner reduction) and avoids the
running-sum accumulator.

### ragged-chunk-pad
**Source:** `solve_tril.py` — `solve_tril` reference.
**Pattern:** Block-triangular inverse over chunked sequence: split into
block_size chunks, invert each `(I + tril_{-1}(A_b))` block, cat back. The last
chunk may be shorter than `block_size` (ragged tail). The reference uses
`F.pad(inverse, (0, block_size - length))` to pad the inverse to `block_size`
before the cat — so the cat axis is uniform and no special-casing is needed.
The naive form would branch on "is this the last chunk?" and skip the cat for
the ragged tail, requiring two code paths. The pad trick unifies both paths.

### reshape-rank-for-backend
**Source:** `geglu.py` — `geglu_unsloth` backend; `swiglu.py` — `swiglu_unsloth` backend.
**Pattern:** The unsloth kernel is elementwise but expects a 3-D tensor
`(1, -1, hidden)`. The caller may pass any rank. The backend adapter reshapes:
`kernel(a.reshape(1, -1, hidden), b.reshape(1, -1, hidden)).reshape(a.shape)`.
The reshape is a view (zero-copy) when the input is contiguous; the kernel runs
on the 3-D view; the output is reshaped back to the caller's rank. The naive
form would require the caller to always pass 3-D (leaking the backend's rank
assumption into the API contract).

### detached-broadcast-insert
**Source:** `rope.py` — `rope` reference.
**Pattern:** RoPE applies `q * cos + rotate_half(q) * sin` where `cos`/`sin` are
`(cos_batch, seq, head_dim)`. The query is `(batch, q_heads, seq, head_dim)`.
The reference uses `cos.detach()[:, None]` — detach (no autograd graph for cos/sin,
which are non-differentiable position embeddings) + insert a broadcast dim at
axis 1 (so `(cos_batch, 1, seq, head_dim)` broadcasts against
`(batch, q_heads, seq, head_dim)`). The naive form would build the full broadcast
tensor explicitly (O(batch × heads × seq × dim) allocation). The detach avoids
the autograd overhead; the `[:, None]` view avoids the materialization.

### batched-upcast-map
**Source:** `comba.py` — `comba` reference.
**Pattern:** The comba kernel takes 6 tensors (q, k, v, p, g, beta) all needing
upcast to fp32 before the recurrence. The reference uses
`q32, k32, v32, p32, g32, beta32 = map(upcast, (q, k, v, p, g, beta))` — one
line, one tuple allocation, six upcasts. The naive form would call `upcast`
six times with six separate bindings. The `map` form is both more concise and
signals to the compiler that all six upcasts are independent (parallelizable
elementwise ops). The `upcast` helper itself uses `torch.promote_types(x.dtype,
torch.float32)` for type promotion.

### dtype-restricted-backend
**Source:** `gelu.py` — `gelu_fla` backend; `sparsemax.py` — `sparsemax_liger` backend.
**Pattern:** Some backends only produce correct gradients for a subset of dtypes.
`gelu.py`'s fla backend is registered for `Float32 | BFloat16` (fp16 tanh
gradients land just past tolerance). `sparsemax.py`'s liger backend is registered
for `Float32 | Float16` (bfloat16 ties break the support-set selection
differently than torch). The pattern: encode the dtype restriction in the
backend's type signature (`x: Float32[Tensor, ...] | BFloat16[Tensor, ...]`),
not in a runtime predicate. The dispatcher selects the backend only when the
input dtype matches; otherwise it falls back to the reference. The naive form
would register the backend for all dtypes and produce silently-wrong gradients.

## Corpus totals

| Category | Before | After | Δ |
|---|---:|---:|---:|
| A: memory access | 6 | 6 | 0 |
| B: compute | 5 | 9 | +4 |
| C: fusion | 7 | 8 | +1 |
| D: dispatch | 4 | 5 | +1 |
| E: parallelism | 3 | 3 | 0 |
| F: flash-attn tiling | 7 | 7 | 0 |
| G: linear-attn recurrence | 6 | 6 | 0 |
| H: MoE dispatch + backward | 9 | 9 | 0 |
| I: NSA block-sparse | 4 | 4 | 0 |
| J: varlen + chunked reduction | 3 | 5 | +2 |
| K: autotune dispatch | 4 | 4 | 0 |
| L: conv-as-GEMM + layout | 5 | 5 | 0 |
| M: kernel polymorphism | 3 | 5 | +2 |
| N: dtype + alignment | 6 | 8 | +2 |
| O: IR strength reduction | 4 | 4 | 0 |
| **Total** | **76** | **88** | **+12** |

## GOAT gate

- **G1 (correctness):** all 12 rules are valid Rust-shaped pseudocode (validated
  by `RustAnalyzerPruner` in the existing test suite). PASS.
- **G2 (perf):** the corpus build is O(n) in the rule count; 88 rules vs 76 is
  negligible. The router's `rules_for_expert()` is O(1) lookup. PASS.
- **G3 (no-regression):** all existing tests pass; the category-count test is
  updated to the new totals. PASS.
- **G4 (alloc-free):** the new rules are static `KernelRule` entries built once
  at corpus construction; zero hot-path allocation. PASS.

## Notes on what was NOT mined (and why)

- **`sigmoid.py`, `swish.py`** (17 + 21 lines): single-element activations
  already covered by `forceinline-device-fn` (sigmoid) and `fuse-norm-act-linear`
  (swish variant). No new pattern.
- **`embedding.py`** (27 lines): a thin wrapper around `F.embedding` with no
  GPU-specific optimization in the reference — the liger backend just calls
  `kernel(weight, indices)`. The gather pattern is already covered by
  `gather-selective-reduction`.
- **`layer_norm.py`** (41 lines): the normalization reduction is already
  covered by `upcast-before-reduction`. The `forward_only=True` flag for
  unsloth is an instance of the frozen-weight assumption, not a new pattern.
- **`l2_norm.py`** (17 lines): `x * torch.rsqrt(x.pow(2).sum + eps)` is the
  standard rsqrt-fused reduction — already covered by `upcast-before-reduction`.
- **`logsumexp.py`** (30 lines): the upcast + scale pattern is already covered
  by `upcast-before-reduction` + `upcast-before-loss`.

These files were read but did not yield a rule distinct from the existing
corpus. Future batches should look at the attention-family kernels
(`gla.py`, `based.py`, `linear_attn.py`, `retention.py`, `rwkv7.py`) and the
cumsum family (`chunk_global_cumsum.py`, `gated_oja_rule.py`) for new recurrence
and scan patterns.
