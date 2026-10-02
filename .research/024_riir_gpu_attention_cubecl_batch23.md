# Research 024 — riir-gpu Production-Kernel Distillation Batch 23 (CubeCL flash-attention decode cluster: combined KV buffer + GQA head sharing + softcap-tanh + fold-factor occupancy + hardcoded fast-path JIT specialization)

**Date:** 2026-08-14
**Source:** riir-ai/crates/riir-gpu/src/attention_cubecl/mod.rs (1573 lines — flash-attention decode + folded dispatch + block-causal + generic LLaMA-family variants)
**Outcome:** 5 new rules distilled (2 Category A — Memory access, 1 Category B — Compute, 2 Category D — Dispatch)
**Corpus:** 155 → **160 rules** across 15 categories (A–O).

## 1. Context

Batch 22 (Research 023) grew the corpus to **155 rules** by mining the
`cudarc_kernels/` CUDA cluster (the full raw-CUDA forward port for Issue 615).
That directory is now fully covered (5 files assessed, `backward.rs` deferred).

This batch mines the **`attention_cubecl/` directory** — the `CubeCL`
portable-path counterpart to the CUDA raw port. `mod.rs` (1573 lines) contains
the flash-attention decode kernel family that ships in the default
`cubecl_runtime` build:

- `attention_decode_f32` — hardcoded Gemma 2 2B decode (n_head=8, n_kv_head=4,
  head_dim=256, softcap=50.0)
- `attention_decode_folded_f32` — folded dispatch (Plan 179 D1), gated
  `fold_dispatch`
- `attention_decode_llama_f32` — generic parametric variant for any
  LLaMA-family architecture
- `attention_block_causal_f32` — block-causal masking (Plan 108 T2), gated
  `gemma2_d2f`

Reading the file revealed the patterns split into two groups:

1. **Already covered** — the online softmax triple (running max/sum/out with
   per-tile correction factors) is `online-softmax-streaming` (Batch 2,
   Category F). The unrolled 256→1 smem tree reduction (8 `sync_cube()`
   barriers) is the portable-path form already mentioned as context in
   `warp-shuffle-intrinsics-for-sub-warp-reduction` (Batch 22, Category B).
   The block-causal `t_n` boundary computation is mask logic specific to the
   d2f feature, not a generalizable kernel opt.
2. **Novel** — the KV buffer layout, GQA head sharing, softcap-tanh score
   bounding, fold-factor occupancy, and hardcoded-vs-parametric JIT
   specialization. Five genuinely new rules.

This is the **tenth batch mining the stack's own kernels** (Batches 14–23).

## 2. Source analysis

### 2.1 Combined KV buffer (`[keys | values]`)

All four kernels take a single `kv: &[f32]` buffer laid out as
`[2 × n_positions × kv_stride]` — keys occupy the first half, values the
second. The launcher's `AttentionParams::combine_kv` concatenates the two
arrays. Inside the kernel, `kv_half = kv.len() / 2` is computed once; a key
read is `kv[kv_off + d]` and a value read is `kv[kv_half + kv_off + d]` —
the same `kv_off` index serves both, offset by `kv_half` for values.

One buffer binding instead of two; one stride variable instead of two. On
the dispatch side this halves the descriptor pressure; on the memory side
both K and V for the same position share a contiguous stride.

### 2.2 GQA head sharing

Every kernel computes `kv_group = head_idx * n_kv_head / n_head` then
`kv_off = kv_group * head_dim` ONCE per workgroup. Multiple query heads
mapped to the same KV group read the identical KV cache slice. Gemma 2 2B:
8 query heads, 4 KV heads → 2× KV cache memory + bandwidth reduction
vs MHA (8:8). The sharing is an architectural choice (model design), but
the kernel-level optimization is that the shared slice is computed once
per workgroup and reused for all K/V reads.

### 2.3 Softcap-tanh score bounding

Gemma 2's attention logits pass through `softcap * tanh(raw / softcap)`
(softcap=50.0) before the online softmax. This bounds scores to
[-softcap, +softcap] with a smooth, everywhere-differentiable function —
unlike clamp, whose gradient is discontinuous (zero) at the boundary.
The generic LLaMA-family kernel handles `softcap == 0.0` by skipping the
softcap (raw scaled dot product) — the LLaMA/MiniCPM path. This dual-path
is a per-model-config branch inside one kernel.

### 2.4 Fold-factor occupancy (Plan 179 D1)

During speculative decode (`seq_len_q > 1`), the unfolded dispatch gives
each head its own workgroup: `CubeCount::Static(n_head, 1, 1)`. For small
models (Gemma 2 2B, 8 heads) on GPUs with many SMs (80+), most SMs idle.
The folded dispatch processes `fold_factor` query tokens per workgroup:
`fold_factor(n_head, seq_len_q, workgroup_size)` returns the largest
divisor of `seq_len_q` ≤ `workgroup_size / n_head`. Example: (8, 4, 256) →
4; (32, 8, 256) → 8; (8, 1, 256) → 1 (nothing to fold).

### 2.5 Hardcoded fast-path vs parametric JIT specialization

`AttentionCubeCL::launch` checks whether params match the Gemma 2 2B config
(n_head=8, n_kv_head=4, head_dim=256, softcap=50.0). If so, it dispatches the
hardcoded `attention_decode_f32` (constants baked into the kernel body).
Otherwise it allocates a 5-element params buffer and dispatches the generic
`attention_decode_llama_f32` (reads dims from the buffer per dispatch).

The hardcoded path exists because the parametric kernel's buffer reads
prevent the `CubeCL` JIT from constant-folding dimensions into optimized
Metal/WGSL shaders — loop bounds are unknown at JIT time, so no unrolling,
no stride folding, no branch elimination. The hardcoded variant recovers
**~24% decode throughput (31.4 → 41.4 tok/s on M3 Max Metal)**. This is a
measured gain from the launcher comment, not a heuristic.

## 3. Five new rules distilled

### 3.1 `combined-kv-buffer-keys-then-values` (Category A — Memory access)

Store keys and values as a single contiguous buffer `[keys | values]` with
shared stride. One buffer binding (was two); one stride variable (was two);
the same `kv_off` index addresses both K and V via an offset by
`kv_half = kv.len() / 2`. Coalesced because both K and V for the same
position share a contiguous stride.

**Substrate-first check:** no existing rule covers KV buffer layout. Closest
is `block-contiguous-weight-layout` (Batch 15 — packing 3 SoA weight buffers
into 1 AoS). That's about weight buffer packing for GEMM; this is about
attention KV buffer binding shape. Distinct mechanism. **NOVEL.**

### 3.2 `gqa-kv-head-sharing-bandwidth-reduction` (Category A — Memory access)

Grouped-Query Attention: share KV heads across multiple query heads
(`n_kv_head < n_head`). Each query head maps to its KV group via
`kv_group = head_idx * n_kv_head / n_head`; the kernel computes
`kv_off = kv_group * head_dim` once per workgroup. Reduces KV cache memory +
bandwidth by `n_head/n_kv_head`× (2× for Gemma 2 2B's 8:4). Distinct from
MHA (1:1) and MQA (all heads share 1 KV head).

**Substrate-first check:** no existing rule covers GQA. The concept tables
have no attention-head-sharing entry. **NOVEL.**

### 3.3 `softcap-tanh-attention-score` (Category B — Compute)

Bound raw attention logits to `[-softcap, +softcap]` via
`softcap * tanh(raw / softcap)`. Smooth + differentiable (unlike clamp's
discontinuous boundary gradient). Prevents extreme Q·K scores from
dominating softmax. Configurable: `softcap == 0.0` disables softcapping →
raw scaled dot product (LLaMA/MiniCPM path). Used by Gemma 2 (softcap=50.0).

**Substrate-first check:** no existing rule covers tanh or softcap. Closest
is `fuse-softmax-mask` (Category C — fusing mask into softmax); that's
fusion, this is score arithmetic. **NOVEL.**

### 3.4 `fold-query-tokens-for-occupancy` (Category D — Dispatch)

When `n_head < SM_count` (small models, speculative decode), fold multiple
query tokens into one workgroup. Each workgroup processes `fold_factor`
query tokens sequentially for one head. `fold_factor(n_head, seq_len_q,
workgroup_size)` picks the largest divisor of `seq_len_q` ≤
`workgroup_size / n_head`. Benefit: `(n_head × fold)` token-heads vs
`n_head` — fills idle SMs during speculative decode.

**Substrate-first check:** closest is `fold-norm-scalar-into-matmul` (Batch
14 — folding a normalization scalar into the GEMV dot). Same word, entirely
different mechanism (scalar algebra vs workgroup work-folding). Also close
to `swap-large-axis-to-x-for-workgroup-distribution` (Batch 16 — axis swap
for SM distribution) — that redistributes which axis gets X-tiles; this
increases work per workgroup. **NOVEL.**

### 3.5 `hardcoded-fast-path-jit-specialization` (Category D — Dispatch)

When kernel params match a known config, dispatch a hardcoded variant that
bakes constants into the kernel body — the JIT compiler then constant-folds
dimensions into optimized shaders (unrolled loops, known stride multiplies,
eliminated per-tile branching). The generic parametric kernel reads dims
from a buffer per dispatch, preventing constant-folding. Measured gain:
~24% decode throughput (31.4 → 41.4 tok/s on M3 Max Metal). Fallback:
parametric kernel for unknown architectures.

**Substrate-first check:** closest is `autotune-warps-stages` (Category D —
sweeps launch configs) and `dtype-specialized-float-kind` (Category N —
dtype-keyed intrinsic dispatch). Autotune *searches* configs; this *matches*
a known config. Dtype specialization dispatches on data type; this
dispatches on architecture dimensions. Also distinct from `heuristic-compile-dispatch`
(Category F — compile-time heuristic dispatch between variants): that's
choosing between algorithm variants; this is choosing between a
JIT-specializable hardcoded variant and a flexible parametric one.
**NOVEL.**

## 4. Substrate-first duplicate detection (5 checks)

| Candidate rule | Closest existing rule | Verdict |
|---|---|---|
| `combined-kv-buffer-keys-then-values` | `block-contiguous-weight-layout` (A, Batch 15) — that's weight SoA→AoS packing for GEMM; this is attention KV buffer binding shape. | **Novel** |
| `gqa-kv-head-sharing-bandwidth-reduction` | (none — no attention-head-sharing entry exists) | **Novel** |
| `softcap-tanh-attention-score` | `fuse-softmax-mask` (C) — that's mask fusion into softmax; this is score arithmetic (tanh bounding). | **Novel** |
| `fold-query-tokens-for-occupancy` | `fold-norm-scalar-into-matmul` (C, Batch 14) — that's scalar algebra folding; this is workgroup work-folding. Also `swap-large-axis-to-x-for-workgroup-distribution` (D, Batch 16) — that redistributes axes; this adds work per workgroup. | **Novel** |
| `hardcoded-fast-path-jit-specialization` | `autotune-warps-stages` (D) — that searches configs; this matches one. Also `dtype-specialized-float-kind` (N) — dtype dispatch, not architecture dims. Also `heuristic-compile-dispatch` (F) — algorithm variant choice, not JIT constant-folding. | **Novel** |

## 5. Candidates assessed and dropped (3 drops)

- **Online softmax three-tuple streaming** — DUPLICATE of
  `online-softmax-streaming` (Batch 2, Category F). The `attention_cubecl`
  kernels' running `(max, sum, out)` state with per-tile correction factors
  is exactly that rule's pattern.
- **Unrolled 256→1 smem tree reduction** — covered as the portable-path
  context in `warp-shuffle-intrinsics-for-sub-warp-reduction` (Batch 22,
  Category B). The explicit `sync_cube()` pyramid is the CubeCL form; the
  warp-shuffle rule documents the smem tree as its complement. Not novel
  enough to warrant a separate entry.
- **Block-causal `t_n` boundary computation** — mask logic specific to the
  `gemma2_d2f` feature (prompt bidirectional + generation block-bidirectional
  + cross-block causal). The boundary arithmetic is model-specific, not a
  generalizable kernel optimization. The two-stage mask+softmax concept is
  already covered by `two-stage-mask-softmax-then-conv` (Category F) and
  `strict-causal-mask-tril-offset` (Category F).

## 6. Categories not mined this batch (and why)

- **Category E (Parallelism), G (Linear-attn recurrence), H (MoE/backward), I
  (NSA), J (Varlen), K (Autotune), L (Conv-GEMM), M (Polymorphism), N
  (Dtype), O (IR strength reduction)** — the attention decode kernels are
  single-workgroup-per-head; no new patterns in these categories.
- **Category C (Fusion)** — the attention kernels don't fuse across op
  boundaries; fusion happens at the launcher level (combine_kv) which is
  host-side concatenation, not kernel fusion.

## 7. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 5 rules valid Rust pseudocode; 0 duplicates (5 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1); 175 lib tests with `kernel_opt` in 0.09s; 292 with all features in 0.35s | ✅ PASS |
| G3 (no-regression) | All existing tests pass; +2 new Batch 23 tests (`batch23_rules_present` + `batch23_attention_cubecl_concepts_covered`) | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |

Routed heal demo maintains **15.0× candidate reduction** (160 flat → 10.7
avg per category).

## 8. Next steps

- **riir-gpu distillation remains OPEN** — 24 of ~200 files mined (~12%).
  `cudarc_kernels/` + `attention_cubecl/` are now fully covered. Richest
  unmined candidates for Batch 24+:
  - **`cudarc_kernels/backward.rs`** (1923 lines) — backward pass gradients.
  - **`gemma2_cubecl/` directory** (5 files, ~265KB) — model-specific
    forward for Gemma 2.
  - **`gemma4_cubecl/` directory** (4 files) — model-specific forward for
    Gemma 4.
  - **`forward/` directory** (~9K lines across 10 files) — DeltaNet +
    FlashPrefill forward substrate.
  - **`backward/` + `backward_dflash/` directories** (~8K lines) — backward
    pass kernels.
- **Centroid mining** via `CommittedFieldBlend` — still blocked on real kernel
  benchmarking data (unchanged).
- **`kernel_perf_bench` numeric validator** — still blocked on GPU (unchanged).
