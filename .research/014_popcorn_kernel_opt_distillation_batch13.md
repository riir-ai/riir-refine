# Research 014 — Popcorn Kernel-Opt Distillation, Batch 13 (final unmined pass)

**Date:** 2026-08-14
**Prior session:** Batch 12 (Research 013, 113 rules). Corpus: 99 → 113 → **119**.
**Source:** `tilde-research/popcorn/src/popcorn/kernels/` — final unmined files.

## 1. Motivation

Batch 12 closed with 113 rules across 15 categories. A `comm -23` diff of the
popcorn kernel directory (96 files) against the mined-files list surfaced
**9 unmined files** (the prior-session summary listed 8; `sqrelu.py` was an
inadvertent omission — it carries the canonical squared-ReLU activation that
`rwkv7_channel_mixing.py` also consumes). This batch mines the remaining 9
files and is the **final popcorn corpus pass**: every file in the popcorn
kernels directory has now been read and either yielded a rule or been
documented as covered.

## 2. Unmined files (9) — per-file analysis

| File | Lines | Outcome |
|---|---|---|
| `grpo_offpolicy.py` | 105 | **+1 rule** (`expm1-for-near-zero-exp-minus-one`). Batch 6 had deferred this file as "RL math, fusion covered" — the numerical-stability aspect (`expm1` for KL cancellation near 0) was overlooked. |
| `llama4_rope.py` | 47 | **+1 rule** (`complex-mul-via-paired-reals`). Paired-real complex multiplication avoids the complex dtype. |
| `qwen2vl_mrope.py` | 45 | **+1 rule** (`cyclic-modality-table-selection`). Multimodal RoPE: split head_dim into sections, cyclic gather from N modality tables. |
| `rwkv7_channel_mixing.py` | 56 | **+2 rules** (`stateful-shift-via-cat-prefix`, `squared-relu-activation`). Token-shift with cross-batch carry + the squared-ReLU activation. |
| `rwkv7_gate_output.py` | 36 | **+1 rule** (`rkv-receptance-correction`). Fused reduce-then-broadcast-multiply. |
| `sqrelu.py` | 17 | (source attribution only — the squared-ReLU rule cites both `sqrelu.py` as canonical source + `rwkv7_channel_mixing.py` as consumer) |
| `rwkv7_k_update.py` | 31 | **No new rule.** `k.addcmul(k * (a - 1), ka)` is a fused multiply-add — structurally covered by existing fusion rules. Same verdict as the prior-session `rwkv7_addcmul` deferral ("too simple"). |
| `sigmoid.py` | 17 | **No new rule.** Single-element `torch.sigmoid(x)`. Already reviewed in Batch 10 (Research 011) as covered by `forceinline-device-fn`. |
| `tvd.py` | 36 | **No new rule.** `0.5 * (p - q.detach()).abs()` — the detach-target + upcast-before-reduce patterns are already covered (`upcast-before-reduction`). |

**Total: +6 rules from 7 files** (9 read, 2 yielded nothing new).

## 3. Substrate-first duplicate detection (3 checks)

Before writing each rule, the existing corpus was grepped for the concept.
All 6 candidates passed:

| Candidate rule | Grep pattern | Existing match? | Verdict |
|---|---|---|---|
| `expm1-for-near-zero-exp-minus-one` | `expm1\|kl_stab\|numerical.stab` | No (2 "numerical stability" hits are for `wkv-logsumexp` + `dual-mode-gate`, different rules) | **Novel** |
| `complex-mul-via-paired-reals` | `complex\|paired.real\|real.*imag\|rotate_half` | No | **Novel** |
| `cyclic-modality-table-selection` | `modality\|cyclic\|mrope\|section` | No | **Novel** |
| `stateful-shift-via-cat-prefix` | `token.shift\|cat.shift\|roll\|x_prev` | `shift-via-pad-subtract` exists (Batch 10, `token_shift.py`) — but that's pad+subtract for a DIFFERENCE; this is cat+carry for cross-batch state | **Novel** (carefully framed) |
| `squared-relu-activation` | `squared.relu\|relu.*square\|sqrelu` | No | **Novel** |
| `rkv-receptance-correction` | `keepdim\|broadcast.multiply\|reduce.then\|dot.product.then` | No | **Novel** |

## 4. Rules shipped (6)

### Category B (Compute) — +4 → 18 total

| Rule | Source | Concept |
|---|---|---|
| `expm1-for-near-zero-exp-minus-one` | `grpo_offpolicy.py` | Numerically stable `e^x − 1` via `expm1(x)` — avoids catastrophic cancellation when x ≈ 0. The KL term `expm1(delta) − delta` is the canonical case. |
| `complex-mul-via-paired-reals` | `llama4_rope.py` | Represent complex numbers as interleaved `[real, imag]` pairs and multiply via 4 scalar muls `(r1·r2 − i1·i2, r1·i2 + i1·r2)`. Avoids the complex dtype where it maps to a slow library path. |
| `stateful-shift-via-cat-prefix` | `rwkv7_channel_mixing.py` | Shift a sequence by concatenating a carry state as the prefix: `[x_prev, x[:, :-1]]`. Distinct from `shift-via-pad-subtract` — carries real cross-batch state (not zero-pad). |
| `squared-relu-activation` | `sqrelu.py` + `rwkv7_channel_mixing.py` | `relu(x)²` — preserves ReLU's exact sparsity (zeros stay zero) while quadratically sharpening non-zero values. Fused form avoids materializing the intermediate. |

### Category C (Fusion) — +1 → 10 total

| Rule | Source | Concept |
|---|---|---|
| `rkv-receptance-correction` | `rwkv7_gate_output.py` | Fused reduce-then-broadcast: `(r·k·ρ).sum(head_dim, keepdim) · v` added to `o`, gated by `g`. Four elementwise ops + one reduce fused into one kernel pass. The `keepdim` makes the reduce output directly broadcastable. |

### Category D (Dispatch) — +1 → 8 total

| Rule | Source | Concept |
|---|---|---|
| `cyclic-modality-table-selection` | `qwen2vl_mrope.py` | Multimodal RoPE: split `head_dim` into N sections (temporal/height/width), each pulling cos/sin from a different modality table via cyclic index `i % n_modalities`. A per-section gather from N constant tables. |

## 5. Corpus totals (after Batch 13)

| Category | Batch 12 | Batch 13 | Total |
|---|---|---|---|
| A: Memory access | 6 | — | 6 |
| B: Compute | 14 | **+4** | **18** |
| C: Fusion | 9 | **+1** | **10** |
| D: Dispatch | 7 | **+1** | **8** |
| E: Parallelism | 6 | — | 6 |
| F: Flash-attn tiling | 10 | — | 10 |
| G: Linear-attn recurrence | 17 | — | 17 |
| H: MoE dispatch backward | 9 | — | 9 |
| I: NSA block sparse | 4 | — | 4 |
| J: Varlen chunked reduction | 5 | — | 5 |
| K: Autotune dispatch | 4 | — | 4 |
| L: Conv-GEMM layout | 5 | — | 5 |
| M: Kernel polymorphism | 5 | — | 5 |
| N: Dtype alignment | 8 | — | 8 |
| O: IR strength reduction | 4 | — | 4 |
| **Total** | **113** | **+6** | **119** |

Category B (compute) is now the largest cluster at **18 rules**, surpassing
Category G (linear-attn recurrence, 17). This reflects the breadth of compute
patterns (numerical stability, representation choices, shifts, activations)
vs the depth of G (which is dominated by the linear-attention recurrence
family).

## 6. GOAT gate analysis

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 6 rules valid Rust pseudocode; 0 duplicates (3 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1) lookup; 155 lib tests in 0.06s | ✅ PASS |
| G3 (no-regression) | All existing tests pass (64 kernel_opt tests + 91 rust_perf tests) | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path allocation | ✅ PASS |

Full 5-layer CI guard + clippy clean (0 warnings). Routed heal demo maintains
**15.0× candidate reduction** (119 flat → 7.9 avg per category).

## 7. What was NOT mined (and why)

- **`rwkv7_k_update.py`** — `k.addcmul(k * (a - 1), ka)` is a fused multiply-add.
  Structurally covered by existing fusion patterns (`fuse-*` family). Same
  verdict as the prior-session `rwkv7_addcmul` deferral.
- **`sigmoid.py`** — single-element `torch.sigmoid(x)`. Already reviewed in
  Batch 10 (Research 011) as covered by `forceinline-device-fn`.
- **`tvd.py`** — total variation distance. The `q.detach()` (treat target as
  constant) + `upcast(distance).sum()` are covered by existing
  `upcast-before-reduction`.

## 8. Popcorn corpus status — COMPLETE

With Batch 13, **every file** in `popcorn/src/popcorn/kernels/` (96 `.py`
files, excluding `_utils.py` / `_quant.py` / `_varlen.py` / `__init__.py`)
has been read and either yielded a rule or been documented as covered.

The popcorn corpus distillation is **COMPLETE at 119 rules**. Future corpus
growth requires new source material (real burn-cubecl kernels, new popcorn
releases, or other GPU kernel libraries).

## 9. Deferred items (unchanged)

- **Centroid mining** via `CommittedFieldBlend` (Plan 321) — needs
  `(signature, observed_speedup)` tuples the pseudocode corpus doesn't carry.
  Blocked on real kernel benchmarking data.
- **`kernel_perf_bench` numeric validator** — blocked on GPU (pseudocode
  rules aren't runnable kernels).
