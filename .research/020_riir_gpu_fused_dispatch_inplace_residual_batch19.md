# Research 020 — riir-gpu Fused-Dispatch + In-Place Residual Distillation (Batch 19)

**Date:** 2026-08-14
**Source:** riir-ai/crates/riir-gpu/src/ (`ternary_ffn_fused.rs`, `gemv_ternary_residual_cubecl.rs`, `deltanet_input_proj_fused.rs`)
**Outcome:** 2 new rules distilled (Category C: fusion); 4 sibling files assessed + dropped.
**Corpus:** 138 → **140 rules** across 15 categories (A–O).

---

## 1. Candidate Assessment

Batch 19 targets the **fused-dispatch** family — the cluster of files that fuse
multi-kernel pipelines into single command-buffer dispatches or single-kernel
fusions. Three files were the primary candidates:

| File | Lines | Verdict | Rules |
|---|---|---|---|
| `ternary_ffn_fused.rs` | 511 | **MINED** — command-buffer-level dispatch chaining | +1 |
| `gemv_ternary_residual_cubecl.rs` | 736 | **MINED** — in-place output array for parameter-count bypass | +1 |
| `deltanet_input_proj_fused.rs` | 590 | **DROPPED** — same pattern as `ternary_ffn_fused.rs` (confirms generality, no new rule) | 0 |
| `deltanet_chunked_cubecl.rs` | 2057 | **DROPPED** — chunked recurrence is the implementation of `recurrent-vs-chunk-dispatch` (Batch 11, Category G); covered | 0 |
| `deltanet_cubecl.rs` | 2548 | **DROPPED** — base recurrence kernel; chunked sibling is the richer pattern source | 0 |

---

## 2. Per-File Analysis

### `ternary_ffn_fused.rs` (511 lines — MINED)

The fused ternary FFN GPU dispatcher (Issue 600). Chains 4 dispatches — gate
GEMV + up GEMV + SwiGLU elementwise + down GEMV — into a single GPU command
buffer, eliminating the per-matvec CPU↔GPU sync that made the naive Issue 599
integration slower than CPU parallel.

**Rule: `chain-dispatches-single-sync-point`** (Category C — fusion). The
naive path runs 4 separate dispatches with a `read_one` sync after each. The
fused path queues all 4 dispatches into one command buffer and only syncs
ONCE at the end (the final output readback). At most 2 intermediate buffers
are live at any point (the dependency chain is linear: gate+up → SwiGLU →
inter → down GEMV → out), so the GPU command scheduler can reuse buffer
memory between dependent dispatches.

**Key distinction from Bench 437's failed deferred-readback**: Bench 437 kept
Q/K/V/gate/up outputs ALL alive simultaneously (5 concurrent buffers), which
INCREASED Metal memory pressure → 16% slower. This pattern works because the
dependency chain is **linear** (max 2 live intermediates), not **fan-out** (N
concurrent). The rule's applicability condition is the linear-dependency
constraint — fanning out N independent intermediates that are all consumed
later is the known-bad case.

**Distinct from `fuse-gate-up-gemv-activation`** (Batch 15, Category C):
that rule is **single-kernel fusion** (collapse 3 ops into 1 kernel). This
rule is **command-buffer-level fusion** (N kernels, 1 sync point). The
tradeoff: single-kernel fusion saves dispatch overhead AND global memory
bandwidth; command-buffer chaining saves only dispatch overhead (sync latency
~55-97 µs per `read_one` on Metal), but works when the ops can't legally be
collapsed into one kernel (different work-group shapes, different weight
tensors, or the ops are independent rather than elementwise-fusable).

### `gemv_ternary_residual_cubecl.rs` (736 lines — MINED)

Fused ternary GEMV + in-place ResidualAdd CubeCL kernel (Issue 616).
Single-dispatch kernel that computes
`residual_output[row] += dot(weight_row, input)` for ternary bit-plane
weights, eliminating one GPU dispatch per FFN down-projection in the Qwen3.5
DeltaNet forward. Collapses 2 dispatches × 64 layers = **128 dispatches/token**
into **64 dispatches/token**.

**Rule: `fuse-residual-via-in-place-output-array`** (Category C — fusion).
CubeCL v0.10 has a hard **5 Array parameter limit** per kernel. The plain
ternary GEMV already uses 5 (pos_bits, neg_bits, scales, input, output).
Adding a 6th `residual` array is impossible.

The fix is to make the kernel **in-place**: the residual lives in the output
buffer and is overwritten by the GEMV result + residual. Same 5 args, but the
output buffer is read-then-written. Lane 0 of each plane reads
`residual_output[row]` before writing
`residual_output[row] = residual[row] + gemv_result[row]`.

**Distinct from `fuse-add-rms-norm`** (Batch 9, Category C): that rule fuses
two ops (add + norm) into one pass for memory-bandwidth savings. This rule
fuses two ops (GEMV + add) by **repurposing the output buffer as the residual
input** to bypass a hard parameter-count constraint. The mechanism is the
in-place aliasing, not the pass-count reduction. General pattern: when a kernel
needs N+1 array parameters but the runtime limits to N, and one of the N
outputs is the accumulation target of the (N+1)th input, alias the output to
the accumulation input (read-then-write semantics).

### `deltanet_input_proj_fused.rs` (590 lines — DROPPED)

Fused DeltaNet input projections GPU dispatch (Issue 602). Chains the 4
DeltaNet input-projection ternary GEMVs — `qkv`, `z`, `a`, `b` — into a single
GPU command buffer. All 4 projections share the same input `x` (the
RMSNorm'd layer input), so they can be dispatched back-to-back without any
inter-kernel data dependency. ONE `read_one` at the end drains all 4.

**Verdict:** This is the **same pattern** as `ternary_ffn_fused.rs`
(`chain-dispatches-single-sync-point`) applied to a different layer. The
independence case (4 projections, no inter-dependency, all share input) vs
the FFN's linear-dependency case — both fit the rule. The file confirms the
pattern is general (not FFN-specific) and provides the counterexample to
Bench 437 (small intermediates → low memory pressure → safe). **No new rule
beyond what `ternary_ffn_fused.rs` already captured.**

### `deltanet_chunked_cubecl.rs` (2057 lines — DROPPED)

Chunkwise parallel prefill kernels (Issue 652 / Plan 533). The sequential
per-token prefill path dispatches P recurrence + P conv1d kernels per layer
(4096 sequential dispatches at P=128, 32 layers) — this module collapses those
to O(P/C) chunk-boundary transitions + intra-chunk batched matmuls.

**Verdict:** This is the **implementation** of the chunk path selected by
`recurrent-vs-chunk-dispatch` (Batch 11, Category G). The chunking algorithm
itself (Yang & Wang 2024) is math, not a kernel optimization technique. The
dispatch-collapsing aspect (P sequential → O(P/C) batched) is a generalization
of `chain-dispatches-single-sync-point` to the recurrence case, but the
existing rule covers it (command-buffer-level fusion, batched dispatches, no
inter-dispatch sync). **No new rule.**

### `deltanet_cubecl.rs` (2548 lines — DROPPED)

Base DeltaNet linear recurrence kernel (Plan 182, Phase 2). Implements the
state update `S_t = diag(β)·S_{t-1} + v⊗k^T` and state read `y = S_t·q`.

**Verdict:** The recurrence math (rank-1 outer product update + matvec read)
is captured by existing Category G rules. The conv1d/gating elementwise
kernels are standard pointwise patterns. The chunked sibling
(`deltanet_chunked_cubecl.rs`) is the richer pattern source and was assessed
separately above. **No new rule.**

---

## 3. Substrate-First Duplicate Detection

Each candidate rule was grepped against the existing 138-rule corpus using
distinctive vocabulary:

| Candidate rule | Grep pattern | Closest existing rule | Verdict |
|---|---|---|---|
| `chain-dispatches-single-sync-point` | `chain.*dispatch\|single.*sync\|defer.*read\|command.buffer\|noread\|launch.amortiz` | `fuse-gate-up-gemv-activation` (different: single-KERNEL fusion collapsing N ops into 1 kernel; this is command-BUFFER-level fusion keeping N kernels but 1 sync point — the tradeoff is dispatch-overhead-only savings, not bandwidth savings, and it applies when ops can't legally collapse into one kernel); `recurrent-vs-chunk-dispatch` (different: backend SELECTION by seq length; this is dispatch BATCHING regardless of length) | **Novel** |
| `fuse-residual-via-in-place-output-array` | `in.place\|in_place\|residual.*output\|output.*residual\|5.*array\|parameter.*limit\|read.then.write` | `fuse-add-rms-norm` (different: fuses add+norm into one PASS for bandwidth; this fuses GEMV+add by ALIASING the output buffer as the residual input to bypass a hard parameter-count constraint — the mechanism is in-place aliasing, not pass-count reduction) | **Novel** |

Both candidates passed novelty verification.

---

## 4. Corpus Totals

| Category | Before | Delta | After |
|---|---|---|---|
| A (Memory access) | 12 | 0 | 12 |
| B (Compute) | 24 | 0 | 24 |
| C (Fusion) | 12 | +2 (`chain-dispatches-single-sync-point`, `fuse-residual-via-in-place-output-array`) | **14** |
| D–O (all other) | 90 | 0 | 90 |
| **Total** | **138** | **+2** | **140** |

---

## 5. GOAT Gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 2 rules valid Rust pseudocode; 0 duplicates (2 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1); lib tests run in <1s | ✅ PASS |
| G3 (no-regression) | All existing tests pass (count assertion updates only) | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |

Full 5-layer CI guard PASS. Routed heal demo maintains candidate reduction
(140 flat → ~9.3 avg per category).

---

## 6. riir-gpu Corpus Status — OPEN

With Batch 19, **16 of ~200 riir-gpu files** have been mined (~8%). The
fused-dispatch + residual cluster is now substantially covered.

### Richest unmined candidates for Batch 20+

- `gemm_ternary_metal_tensor.rs` / `gemm_ternary_metal_wgpu.rs` / `gemm_ternary_metal_zero_copy.rs` — Metal tensor core paths (different from CubeCL cmma).
- `gemv_ternary_cuda_raw.rs` / `gemv_ternary_cubecl.rs` — CUDA raw + base ternary GEMV (CUDA dp4a + `__byte_perm` instructions may be new hardware-instruction rules).
- `edge_lora/` directory — LoRA training + self-play (may be riir-train concerns).
- `gemv_ternary_block_contiguous_cubecl.rs` — block-contiguous variant (Issue 650 Phase 2; may overlap Batch 15's `block-contiguous-weight-layout`).
- `gemma2_q4k_weights.rs` — Q4_K weight loading (CPU-side, likely not kernel opt).
- `gemma2_d2f` / `gemma2_d2f_sc.rs` — D2F paths.
- Directory clusters: `attention_cubecl/`, `backward/`, `forward/`, `gemma2_cubecl/`, `gemma2_cubecl_train/`.

### Blocked items (unchanged)

- **Centroid mining** via `CommittedFieldBlend` — still blocked on real kernel benchmarking data.
- **`kernel_perf_bench` numeric validator** — still blocked on GPU (pseudocode rules aren't runnable kernels).
