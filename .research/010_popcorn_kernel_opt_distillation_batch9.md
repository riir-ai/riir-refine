# Research 010 — Popcorn kernel_opt distillation: Batch 9 (8 rules — fusion + dtype/quant + dispatch)

**Status:** COMPLETE (batch 9 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** The kernel_opt corpus had 68 rules with categories C (fusion, 4 rules),
D (dispatch, 3 rules), and N (dtype+alignment, 2 rules) being the smallest. This
batch targets those gaps by distilling quantization, fused-normalization, and
fused-loss patterns from popcorn kernels not yet mined.
**Source:** `/Users/katopz/git/popcorn` — 6 kernel files:
- `kernels/int8_int2_matmul.py` (42 lines) — int8×int2 packed matmul
- `kernels/add_rms_norm.py` (28 lines) — fused residual add + RMS norm
- `kernels/group_norm_linear.py` (38 lines) — fused group norm + linear
- `kernels/jsd.py` (44 lines) — Jensen-Shannon divergence
- `kernels/bit_linear.py` (35 lines) — BitNet b1.58 linear
- `kernels/_quant.py` (20 lines) — BitNet fake quantization helpers

## What was mined

### Category C: Fusion patterns (3 new rules → 7 total)

| Rule | Source | Pattern |
|---|---|---|
| `fuse-add-rms-norm` | `add_rms_norm.py` | Fused residual add + RMS norm (avoids 2-pass: separate add then norm) |
| `fuse-group-norm-linear` | `group_norm_linear.py` | Fused group norm + linear projection (avoids intermediate normalized tensor) |
| `fuse-jsd-logsumexp` | `jsd.py` | Fused JSD/KL divergence via log-domain (avoids materializing probability matrices) |

### Category D: Dispatch patterns (1 new rule → 4 total)

| Rule | Source | Pattern |
|---|---|---|
| `predicate-gate-div512` | `int8_int2_matmul.py` `_aligned` | Shape divisibility gate (inner dim % 512 == 0) for the fast packed-int2 path |

### Category N: dtype + alignment patterns (4 new rules → 6 total)

| Rule | Source | Pattern |
|---|---|---|
| `int2-pack-weights` | `int8_int2_matmul.py` `pack_weights` | Pack 4 ternary {-1,0,+1} weights per byte before matmul (4× memory bandwidth reduction) |
| `per-token-activation-quant` | `_quant.py` `activation_quant` | Per-token symmetric int8 fake quant with straight-through gradient |
| `ternary-weight-quant` | `_quant.py` `weight_quant` | Per-tensor ternary {-1,0,+1} fake quant with straight-through gradient |
| `upcast-before-loss` | `jsd.py` | Upcast fp16 inputs to f32 before loss computation to avoid precision loss in logsumexp |

## Per-rule source attribution

### fuse-add-rms-norm
**Source:** `add_rms_norm.py` — `add_rms_norm` reference + `liger` backend.
**Pattern:** The reference computes `s = x + residual` then `rms(s)` in two logical
steps. The liger backend fuses both into one kernel pass, returning the normalized
sum AND the new residual stream (so the next transformer layer doesn't need a
separate residual update). The key optimization: one global memory round-trip
instead of two (add → store → load → norm → store).

### fuse-group-norm-linear
**Source:** `group_norm_linear.py` — `group_norm_linear` reference + `fla` backend.
**Pattern:** The reference computes `h = group_norm(x)` then `linear(h, w)` in two
steps with an intermediate `h` tensor. The fla backend fuses both, eliminating the
intermediate. The `_wide_groups` predicate gate (`channels // num_groups >= 8`)
ensures the fast path is only used when group width is sufficient for numerical
stability.

### fuse-jsd-logsumexp
**Source:** `jsd.py` — `jsd` reference + `liger` backend.
**Pattern:** The reference computes `generalized_jsd` which materializes `p = exp(log_q)`
and `q = exp(log_p)` — two full probability matrices. The liger backend works entirely
in the log domain via `logaddexp`, avoiding the probability materialization entirely.
The `upcast(log_p)` call before the kernel avoids fp16 precision loss in the log
computation.

### predicate-gate-div512
**Source:** `int8_int2_matmul.py` — `_aligned` predicate.
**Pattern:** The liger int8×int2 backend requires `inner_dim % 512 == 0` for its
autotune tiling. The `_aligned` predicate gate ensures the fast path is only
dispatched when the shape is compatible; otherwise the reference path runs. This
is a general pattern: conditional kernel dispatch based on shape divisibility.

### int2-pack-weights
**Source:** `int8_int2_matmul.py` — `kernel.pack_weights(b)`.
**Pattern:** Ternary weights {-1, 0, +1} can be encoded in 2 bits each, packing 4
weights per byte. The naive approach stores each weight as a full int8 (8 bits per
weight, 4× wastage). The `pack_weights` call packs 4 weights per byte before the
matmul, reducing memory bandwidth by 4×. This is directly relevant to katgpt-core's
ternary inference (the same {-1, 0, +1} weight space).

### per-token-activation-quant
**Source:** `_quant.py` — `activation_quant`.
**Pattern:** Per-token symmetric int8 quantization: compute `scale = 127 / max(|x|)`,
then `q = round(x * scale) / scale`. The straight-through gradient trick (`x + (q - x).detach()`)
ensures gradients flow through the rounding step during training. In a pure inference
context (katgpt-rs modelless mandate), the quantization is deterministic (no gradient).

### ternary-weight-quant
**Source:** `_quant.py` — `weight_quant`.
**Pattern:** Per-tensor ternary quantization: compute `scale = 1 / mean(|w|)`, then
`q = round(w * scale).clamp(-1, 1) / scale`. This produces weights in {-1, 0, +1}.
Same straight-through gradient trick as `per-token-activation-quant`.

### upcast-before-loss
**Source:** `jsd.py` — `upcast(log_p)` before the liger kernel.
**Pattern:** fp16 inputs are upcast to f32 before loss computation (log, exp, logaddexp).
fp16 has insufficient precision for the log domain — `log(very_small_number)` underflows
to -inf. The upcast is a one-time cost that prevents catastrophic precision loss.
This is the same pattern as the existing `upcast-before-reduction` rule but applied
to loss computation rather than reduction.

## What was NOT included (assessed + dropped)

- **`bit_linear.py` end-to-end fusion** — The full BitLinear pipeline (RMSNorm →
  int8 quant → ternary quant → matmul) is already partially covered by `fuse-quant-matmul`
  (Category C) + `upcast-before-reduction` (Category B). The new rules
  (`per-token-activation-quant`, `ternary-weight-quant`, `int2-pack-weights`)
  decompose the pipeline into its component patterns, which is more reusable than
  one monolithic rule.
- **JSD beta=0 / beta=1 special-case branches** — These are KL divergence
  special cases. The general JSD formula subsumes them. Not a distinct GPU pattern.
- **`_wide_groups` predicate for group_norm_linear** — This is the same
  predicate-gate pattern as `predicate-gated-backend` (Category D) + the new
  `predicate-gate-div512`. The divisibility threshold differs (>= 8 channels per
  group vs % 512) but the pattern is the same. `predicate-gate-div512` was chosen
  as the representative because the % 512 case is more common (tile alignment).

## How these map to the KernelRule struct

Each rule becomes a `KernelRule { rule_id, gpu_concept, ref_shape, opt_pattern }`
where:
- `rule_id` is the kebab-case identifier above
- `gpu_concept` is a short description of the optimization concept
- `ref_shape` is 1-3 lines of Rust-shaped pseudocode showing the naive version
- `opt_pattern` is 1-3 lines of Rust-shaped pseudocode showing the optimized version

The pseudocode is intentionally Rust-shaped (not Python/Triton) so the
`RustAnalyzerPruner` can syntax-validate it.

## GOAT gate for this batch

This batch is syntax-validated only (no GPU perf measurement):
- G1 (determinism): all rules are deterministic pseudocode
- G2 (perf): not measured (blocked on GPU)
- G3 (no-regression): 261 tests pass (was 253 + 8 new rules' round-trip tests)
- G4 (alloc-free): corpus is built once at boot; no per-tick allocation

The real perf gate (`kernel_perf_bench`) is deferred until the numeric validator
lands (Proposal 012 §Phase 0, blocked on GPU).

## Next steps

- Batch 10+: continue mining popcorn kernels (many remain: `embedding.py`,
  `gelu.py`, `geglu.py`, `hadamard_transform.py`, `comba.py`, `dyt.py`, etc.)
- Centroid mining via `CommittedFieldBlend` (needs real speedup data)
- `kernel_perf_bench` numeric validator (blocked on GPU)

## References

1. **popcorn** — `/Users/katopz/git/popcorn/src/popcorn/kernels/{int8_int2_matmul,add_rms_norm,group_norm_linear,jsd,bit_linear,_quant}.py`
2. **BitNet b1.58** — [Ma et al., 2024](https://arxiv.org/abs/2402.17764)
3. **BitNet** — [Wang et al., 2023](https://arxiv.org/abs/2310.11453)
4. **RMSNorm** — [Zhang & Sennrich, 2019](https://arxiv.org/abs/1910.07467)
5. **Group Normalization** — [Wu & He, 2018](https://arxiv.org/abs/1803.08494)
