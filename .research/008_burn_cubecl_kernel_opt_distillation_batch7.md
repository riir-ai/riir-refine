# Research 008 — burn-cubecl kernel_opt distillation: Batch 7 (autotune dispatch + conv-as-GEMM + layout, 9 rules)

**Status:** COMPLETE (batch 7 of the ongoing kernel_opt corpus expansion)
**Date:** 2026-08-14
**Trigger:** Research 007 paused popcorn distillation ("heavy impl files exhausted;
future batches need new source impl files"). This batch mines a **new source**:
`burn-cubecl` (`/Users/katopz/git/burn/crates/burn-cubecl/`) — the Rust-native
GPU compute backend of the Burn deep-learning framework, written in CubeCL.
**Why burn-cubecl:** unlike popcorn's Python/Triton/CUDA patterns, burn-cubecl
patterns are **Rust-native** (CubeCL is the same compute layer `riir-gpu` uses),
so the distilled patterns apply directly to the riir-stack's own GPU kernels.

## Source files

| File | Lines | Patterns distilled |
|---|---|---|
| `kernel/matmul/tune/base.rs` | 510 | panic-safe autotune, multi-priority tune groups, shape-kind dispatch, vector-width sweep |
| `kernel/reduce/tune.rs` | 291 | vector-width pow2 sweep, reduce-count routine selection |
| `kernel/conv/im2col.rs` | 189 | 1×1 conv as matmul, contiguity guard, cube-count batch pack |
| `kernel/conv/forward/tune.rs` | 186 | stride-align vectorization key, anchor shape bucketing, conv-as-GEMM family dispatch |

## What this batch adds

9 new rules across 2 new categories (K: autotune dispatch, L: conv-as-GEMM +
memory layout). Corpus: 50 → 59.

## Batch 7 — 9 rules

| Rule ID | GPU Concept | Category | Source |
|---|---|---|---|
| `panic-safe-autotune-candidate` | panic-resilient autotune candidate | K | matmul/tune/base.rs `launch_matmul_safe` |
| `multi-priority-tune-groups` | shape-keyed priority-ranked autotune groups | K | matmul/tune/base.rs `TuneGroup` + `PRIORITY_*` |
| `shape-kind-dispatch` | matmul-kind-keyed kernel family dispatch | K | matmul/tune/base.rs `MatmulKind` |
| `vector-width-pow2-sweep` | vectorization width as an autotune dimension | K | reduce/tune.rs `sum_one_shot<C>` |
| `conv-1x1-as-matmul` | 1×1 convolution as direct matmul (skip im2col) | L | conv/im2col.rs `conv_im2col_1x1` |
| `contiguity-guard-before-reshape` | stride-contiguity guard skips unnecessary relayout copy | L | conv/im2col.rs `is_spatial_contiguous` + `reshape_input` |
| `cube-count-bounded-batch-pack` | grid-limit-aware batch packing | L | conv/im2col.rs `batches_per_run` |
| `stride-align-vectorization-key` | stride-byte-alignment determines vectorization width | L | conv/forward/tune.rs `stride_align` + `pow2_factor` |
| `anchor-shape-bucketing` | shape bucketing for autotune cache reuse | L | conv/forward/tune.rs `anchor(shape, ...)` |

## Per-rule source attribution

### panic-safe-autotune-candidate
**Source:** `matmul/tune/base.rs` L18-40 (`launch_matmul_safe`)
**Concept:** Autotune candidates can panic on unsupported shapes (e.g.,
divide-by-zero in tiling blueprint when `K` is smaller than the tile). Wrapping
the kernel launch in `catch_unwind` converts the panic into `Err`, so the
autotune system **skips** the failed kernel instead of crashing the entire
search. This makes the autotune set robust to shape-specific kernel limitations
without the caller needing to know each candidate's constraints upfront.

### multi-priority-tune-groups
**Source:** `matmul/tune/base.rs` L83-118 (`TuneGroup` + `PRIORITY_MAX/HIGH/MEDIUM/MIN/NEVER`)
**Concept:** Instead of a flat candidate list benchmarked equally, candidates
are grouped by `TuneGroup` with **priority predicates** keyed on the autotune
key. A candidate assigned `PRIORITY_NEVER (-1)` is skipped entirely for that
shape; higher priority candidates are benchmarked first. The TMA group is
`PRIORITY_MAX` only when `skew_factor < 4 && stride_factor >= 4`; the GEMV
group is `PRIORITY_MAX` only for `MatmulKind::MatVec`. This prunes the search
space per shape — the tuner never benchmarks a kernel that can't apply.

### shape-kind-dispatch
**Source:** `matmul/tune/base.rs` `MatmulKind::{General, MatVec, VecMat, InnerProduct}`
**Concept:** A single general GEMM kernel wastes 99% of compute on matvec (one
row or column). Dispatching on `MatmulKind` routes to a **dedicated kernel
family**: MatVec → plane-parallel or unit-perpendicular GEMV kernels; General →
tiled GEMM. The GEMV kernels are O(M) not O(M×N), a fundamental complexity
reduction for the degenerate case.

### vector-width-pow2-sweep
**Source:** `reduce/tune.rs` L150-157 (`sum_one_shot<C>` with C ∈ {1,2,4,8,16,32,64})
**Concept:** Vector width (data-level parallelism per thread) is itself an
**autotune dimension**, not a hardcoded constant. Registering 7 candidates
(C = 1,2,4,8,16,32,64) lets the tuner pick the best vectorization per
(element_count, dtype). Small reductions benefit from wide vectors (fewer
iterations); large ones may saturate register pressure at width 4. The tuner
resolves this empirically rather than guessing.

### conv-1x1-as-matmul
**Source:** `conv/im2col.rs` `conv_im2col_1x1`
**Concept:** A 1×1 convolution (kernel all ones) is mathematically identical to
a matrix multiply: input reshaped to `[(N,H,W), C_in]` × weight `[C_in, C_out]`.
The im2col expansion (materializing a `KH*KW*C` column matrix) is pure overhead
when KH=KW=1 — it allocates O(N*C_in) extra memory for a transformation that's
the identity. Reshaping to 2D + calling matmul directly skips the expansion
entirely.

### contiguity-guard-before-reshape
**Source:** `conv/im2col.rs` `is_spatial_contiguous` + `reshape_input`
**Concept:** Before doing a free metadata reshape (e.g., `[(N,H,W), C]`), check
that the tensor is already **spatially contiguous** (`stride[C] == 1` and each
spatial dim nests correctly: `stride[i] == stride[i+1] * shape[i+1]`). Only
call `into_contiguous_pitched` (a real data-movement kernel) when the layout
doesn't already support the reshape. This avoids an unnecessary full copy when
the reshape is already valid as a metadata-only operation.

### cube-count-bounded-batch-pack
**Source:** `conv/im2col.rs` `batches_per_run`
**Concept:** The GPU's cube (block) count is bounded (e.g., `u16::MAX = 65535`).
For small-batch workloads, launching one batch at a time underutilizes the grid.
`batches_per_run` computes `max_simultaneous = min(u16::MAX / cubes_per_batch,
batch_size)`, then finds the largest **divisor** of `batch_size` ≤ max (so the
pack divides evenly). This saturates the grid without overflowing the cube-count
limit — and the divisibility constraint avoids tail-fragmentation.

### stride-align-vectorization-key
**Source:** `conv/forward/tune.rs` `stride_align` + `pow2_factor`
**Concept:** The achievable vectorization width is determined by the **byte
alignment** of the tensor's strides, not just the element count.
`stride_align(strides, dtype) = min(trailing_zeros(stride × bytes_per_elem))`
capped at 2^10 (the 128-byte swizzle repeat). A stride of 3 f32-elements (12
bytes) has `trailing_zeros(12) = 2` → max vector width 4. A stride of 1 (4
bytes) has `trailing_zeros(4) = 2` → also 4. A stride of 8 f32 (32 bytes) → 32
→ 8-wide possible. This feeds the autotune key so the vector width is chosen
based on what the memory layout actually supports.

### anchor-shape-bucketing
**Source:** `conv/forward/tune.rs` `anchor(shape, None, None, None)`
**Concept:** Without shape bucketing, every distinct spatial dimension (H=224 vs
H=225) produces a separate autotune cache key → the tuner re-tunes from scratch
for a 1-pixel difference. `anchor` buckets spatial dims so shapes within the
same bucket (e.g., nearest 16) share a cache entry. This dramatically reduces
the number of autotune runs in production (where input shapes vary slightly per
batch) while preserving config quality (a 224-tile config is fine for 225).

## What was NOT included (assessed + dropped)

| Pattern | Source | Why dropped |
|---|---|---|
| Pitched-contiguous (TMA-stride-aware copy) | `into_contiguous_aligned` | Overlaps with `contiguity-guard-before-reshape` — both about avoiding relayout. The guard (skip the copy) is the more general + cleaner rule; the pitched-copy variant is CubeCL/TMA-specific. |
| Conv family trichotomy (direct vs im2col vs implicit-GEMM) | `conv/forward/tune.rs` 8 candidates | The trichotomy is covered compositionally: `conv-1x1-as-matmul` (the im2col→matmul case) + `shape-kind-dispatch` (the general dispatch principle). Enumerating 8 concrete conv kernels is burn-specific, not a general GPU technique. |
| Double-buffering tune decision | `should_tune_double_buffering` | Genuinely novel (overlapping compute+memory) but hard to express as a clean ref→opt pair in Rust pseudocode without CubeCL pipeline abstractions. Deferred to a future batch with a cleaner framing. |
| Reduce-count routine selection (unit/plane/cube by vector_count) | `reduce/tune.rs` `ReduceProps` | Novel, but conceptually the same shape-keyed dispatch family as `multi-priority-tune-groups` + `shape-kind-dispatch`. Included indirectly via those two rules. |

## Deduplication vs existing corpus (Batches 1-6)

| New rule | Closest existing rule | Why distinct |
|---|---|---|
| `multi-priority-tune-groups` | `predicate-gated-backend` (D) | predicate-gated is a **binary** gate (fast vs reference); tune-groups is a **multi-priority** ranked system with 5 levels (MAX/HIGH/MEDIUM/MIN/NEVER). |
| `shape-kind-dispatch` | `predicate-gated-backend` (D) | predicate-gated checks shape safety; shape-kind **routes to a different kernel family** (GEMV vs GEMM) — a complexity-class change, not a safety gate. |
| `vector-width-pow2-sweep` | `autotune-warps-stages` (B) | warps-stages sweeps thread-config (compute parallelism); vector-width sweeps **data-level parallelism** (elements per lane). Different axis. |
| `stride-align-vectorization-key` | `pow2-block-alignment` (D) | pow2-block-alignment aligns the **block dimension** to hardware; stride-align keys vectorization on **stride byte-alignment**. Different target. |
| `autotune-aggregate-workload-key` (Batch 4) | `multi-priority-tune-groups` (K) | aggregate-workload keys autotune on total work Msum; tune-groups is a **priority-ranking system** over candidate groups. Different mechanism. |

## GOAT gate for this batch

Same as batches 1-6: corpus expansion, no new substrate. G1 (compile + all
pseudocode parses via `RustAnalyzerPruner`) + G3 (no-regression: 128 lib tests
pass with `--features kernel_opt`, 91 default, clippy clean).

## Next steps

- [x] Batch 1: 19 rules (memory, compute, fusion, dispatch, parallelism) — Research 002
- [x] Batch 2: 7 rules (flash-attention tiling) — Research 003
- [x] Batch 3: 6 rules (linear-attention recurrence) — Research 004
- [x] Batch 4: 9 rules (MoE dispatch + backward pass) — Research 005
- [x] Batch 5: 4 rules (NSA block-sparse selection) — Research 006
- [x] Batch 6: 3 rules (varlen + chunked reductions) — Research 007
- [x] Batch 7: 9 rules (burn-cubecl autotune dispatch + conv-as-GEMM + layout) — this note
- [ ] Batch 8+: burn-cubecl still has unmined files (comparison.rs 442 lines,
      binary.rs 336 lines, pool/, index/, deform_conv). The matmul reduce + conv
      kernels are the richest; remaining files are likely to yield 2-4 more
      general techniques. cubecl itself has optimization passes
      (`cubecl-opt/src/passes/reduce_strength.rs`) that may distill into
      compiler-optimization rules. Recommend pausing until the corpus is
      consumed end-to-end (router wiring or a real heal invocation).
