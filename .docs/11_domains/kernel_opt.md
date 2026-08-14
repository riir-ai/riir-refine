# `kernel_opt` domain (Plan 001 Phase 3, absorbs Proposal 012)

> The third `HealDomain` impl. Heals GPU kernel optimization. Absorbs
> Proposal 012's design (thermal-tier + latent-MoE + numeric validator) into
> the multi-domain framework. **4090 LoRA deferred** across all tiers above
> Plasma.

## What ships vs deferred (Proposal 012 §Phase 0)

| Component | Ships now | Deferred |
|---|---|---|
| `KernelOptDomain: HealDomain` | ✅ | |
| Seed corpus (Batch 1-8: 68 rules) | ✅ | Full corpus (~300-500 rules, ~15-25 batches) |
| `KernelExpertRouter` (`pick_domain`) | ✅ (identity directions) | Per-expert centroids from popcorn |
| `KernelThermalRouter` (5-tier stub) | ✅ (returns Plasma) | Warm/Cold LoRA (4090, riir-train) |
| `KernelVerifier` (compile-only) | ✅ (behind `kernel_verify`) | Full numeric compare vs reference |
| L4 LoRA fallback | | ✅ Deferred across all domains |

## Corpus

Batch 1 (Research 002): 21 rules — 2 original + 19 popcorn-distilled across 5 categories.
Batch 2 (Research 003): 7 rules — flash-attention tiling patterns.
Batch 3 (Research 004): 6 rules — linear-attention recurrence patterns.
Batch 4 (Research 005): 9 rules — MoE dispatch + backward-pass patterns.
Batch 5 (Research 006): 4 rules — NSA block-sparse selection patterns.
Batch 6 (Research 007): 3 rules — varlen packing + chunked reductions.
Batch 7 (Research 008): 9 rules — burn-cubecl autotune dispatch + conv-as-GEMM + memory layout.
Batch 8 (Research 009): 9 rules — burn-cubecl kernel polymorphism + dtype/alignment specialization + cubecl-opt IR strength reduction.
Total: 68 rules. Full corpus target: ~300-500 rules in ~15-25 batches.

### Category A: Memory access patterns (6 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `matvec-coalesce` | coalesced memory access | Strided read (uncoalesced) → transposed coalesced read |
| `coalesced-row-read` | coalesced memory access | Strided load (H*D apart) → contiguous load (adjacent) |
| `vectorized-pack-load` | vectorized memory access | Scalar load → 4-element packed load (16-byte transactions) |
| `grid-stride-loop` | work distribution | Fixed one-thread-per-element → grid-stride for arbitrary n |
| `gemm-tiling` | shared-memory tiling | Naive GEMM (global mem) → tiled GEMM (shared mem reuse) |
| `masked-boundary-load` | boundary-safe memory access | Unchecked load → masked load with fallback |

### Category B: Compute patterns (5 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `autotune-warps-stages` | autotune dispatch | Hardcoded threads → sweep over (warps, stages) |
| `upcast-before-reduction` | numerical precision | fp16 reduction → fp32 upcast before mean/sum |
| `branchless-select` | branch divergence elimination | Conditional branch → select/if-expr |
| `unroll-fixed-inner` | loop unrolling for ILP | Dynamic loop → fixed-size unrolled |
| `forceinline-device-fn` | function inlining | Regular call → `#[inline(always)]` |

### Category C: Fusion patterns (4 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `fuse-norm-act-linear` | kernel fusion | 3-pass norm+act+linear → single fused pass |
| `fuse-quant-matmul` | quantization fusion | Separate quant+matmul → fused (BitNet b1.58) |
| `fuse-softmax-mask` | mask fusion in softmax | Separate mask+softmax → fused masked softmax |
| `fuse-xent-logsumexp` | cross-entropy fusion | Softmax+NLL → fused logsumexp (no prob matrix) |

### Category D: Dispatch patterns (3 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `sm-count-block-size` | occupancy-aware block sizing | Fixed block → SM-aware `min(128, next_pow2)` |
| `predicate-gated-backend` | conditional kernel dispatch | Unconditional fast path → shape/dtype predicate gate |
| `pow2-block-alignment` | power-of-2 alignment | Arbitrary dim → `next_power_of_two` |

### Category E: Parallelism patterns (3 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `multi-dim-grid-launch` | multi-dimensional parallelism | 1D nested loop → 3D parallel grid (batch×head×time) |
| `block-count-clamp` | grid dimension clamping | Unbounded blocks → clamped to 65535 |
| `backward-as-conjugate` | backward kernel reuse | Separate backward → forward with conjugate flag |

The patterns are Rust-shaped pseudocode so the shared `RustAnalyzerPruner` can
validate them in the fast path. When the full validator lands (CubeCL compile +
numeric compare), the patterns will carry real kernel source.

## Router — `KernelExpertRouter`

Wraps `katgpt_core::variable_rank_domain_expert::pick_domain<4, 5>` (Plan 558):

```rust
pub const NUM_KERNEL_EXPERTS: usize = 4;  // coalesce, tile, fuse, vectorize
pub const ACTIVITY_DIM: usize = 5;        // M, N, K, dtype, hw

pub fn route(&self, activity: &[f32; ACTIVITY_DIM]) -> usize {
    pick_domain::<NUM_KERNEL_EXPERTS, ACTIVITY_DIM>(activity, &self.directions)
}
```

- **Activity vector** = raw physical (kernel signature, deterministic).
- **Domain directions** = latent (placeholder: identity vectors; real
  per-expert centroids from popcorn distillation land later).
- **Expert index** = raw categorical (synced).
- Modelless argmax — zero allocation.

The `kernel_heal` feature enables `katgpt-core/variable_rank_domain_expert`
so `pick_domain` is reachable.

## Thermal — `KernelThermalRouter` + `ThermalTier`

The FlashAR/QGF thermal-tier pattern: Plasma→Hot→Warm→Cold→Freeze escalation.

| Tier | What runs | Where | Latency | Status |
|---|---|---|---|---|
| Plasma | Modelless cache hit (Stage 1 drafter) | M3 Metal (local) | µs | ✅ shipped |
| Hot | Stage 2 adaptive drafter (Issue 010) | M3 Metal (local) | µs | ✅ shipped |
| Warm | LoRA draft + validate | 4090 CUDA | ms | ⏳ deferred (riir-train) |
| Cold | Full validate vs reference | 4090 CUDA | s | ⏳ deferred |
| Freeze | Archive snapshot | cold storage | offline | ⏳ deferred |

**Scaffold behavior:** returns `ThermalTier::Plasma` for ALL calls — the
modelless path is the primary tier per the modelless-first mandate. The
escalation logic (Plasma → Hot on cache miss, Hot → Warm on Stage 2 pool miss,
Warm → Cold on LoRA uncertainty, Cold → Freeze on archive trigger) lands when
the 4090 path is wired (riir-train).

`ThermalTier` is `#[repr(u8)]` (1 byte, sync-efficient):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ThermalTier {
    #[default]
    Plasma = 0,  // modelless cache
    Hot = 1,     // Stage 2 adaptive
    Warm = 2,    // LoRA draft — DEFERRED
    Cold = 3,    // full validate — DEFERRED
    Freeze = 4,  // archive — DEFERRED
}
```

## Verifier — `KernelVerifier` (compile-only stub)

Behind `kernel_verify`. **STUB** — delegates to the shared `RustAnalyzerPruner`
for syntax validation. Full CubeCL compile + numeric compare vs reference is
deferred (Proposal 012 §Phase 0). `is_full_validator_available()` returns
`false` in the stub so callers can detect the deferred state.

## Domain classification (per AGENTS.md)

- Kernel **shapes** (M, N, K, dtype, hw) = raw physical (deterministic,
  replayable — needed for quorum sync of kernel selection decisions).
- Expert **directions** = latent (dot-projection onto learned centroids,
  projected to a scalar expert index via `pick_domain`). NOT synced — only
  the resulting expert index crosses the sync boundary.
- Thermal tier = raw categorical (1 byte, `#[repr(u8)]`).

### Category F: Flash-attention tiling (Batch 2, 7 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `online-softmax-streaming` | flash-attention online softmax | Full matrix materialization → streaming running max+sum+rescale |
| `exp2-log2-base2` | base-2 math | Natural exp/ln → base-2 exp2/log2 (dedicated GPU instruction) |
| `block-pointer-2d` | structured 2D block pointer loads | Manual offset arithmetic → structured block ptr with boundary check |
| `heuristic-compile-dispatch` | compile-time heuristic dispatch | Runtime branch → compile-time specialization (dead branch elimination) |
| `causal-diag-offdiag-split` | causal matrix diagonal/off-diagonal splitting | Uniform precision → diagonal fp32 + off-diagonal bf16-safe |
| `shared-mem-aware-tile` | shared-memory-aware tile sizing | Fixed tile → SM-aware (Hopper larger, Ampere smaller) |
| `kv-block-streaming` | KV block streaming through shared memory | Full KV load → block streaming (O(block) shared mem) |

### Category G: Linear-attention recurrence (Batch 3, 6 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `state-forget-gate` | exponential decay gate on recurrent state | Unbounded accumulation → `state *= gate.exp()` |
| `householder-rank1-update` | rank-1 Householder reflection | General update → error-correcting delta rule |
| `chunkwise-parallel-recurrence` | chunkwise parallelism for state-carrying loops | Fully sequential → chunk-parallel (intra-chunk matmul) |
| `recurrent-vs-chunk-dispatch` | sequence-length-aware backend dispatch | Always chunk → recurrent for short, chunk for long |
| `multi-write-per-step` | multiple state writes per timestep | Single write → N Householder writes under one gate (DeltaProduct) |
| `chained-two-pass-recurrence` | chained sequential recurrences with softmax bridge | Single pass → K-state then V-state with softmax bridge (GSA) |

### Category H: MoE dispatch + backward pass (Batch 4, 9 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `grouped-launch-cumsum-offsets` | grouped GEMM with prefix-sum offsets | N separate launches → single grid with cumsum-derived group offsets |
| `l2-grouped-tile-schedule` | swizzled L2-cache-aware tile mapping | Row-major block map → grouped swizzle for L2 reuse |
| `fused-matmul-activation` | pointwise activation in matmul epilogue | Separate matmul+activation passes → fused epilogue |
| `fused-backward-chain` | two backward ops fused into one kernel | Two backward kernels with global handoff → one fused kernel |
| `activation-recompute-budget` | partial activation save with in-kernel recompute | Save all activations → save fraction + recompute rest (checkpointing) |
| `atomic-scatter-reduction` | lock-free atomic scatter for parallel writes | Locked scatter → relaxed atomic add |
| `weight-grad-batched-outer-product` | weight gradients as batched outer product GEMM | Per-token rank-1 updates → batched tiled GEMM |
| `csr-index-via-prefix-sum` | GPU-side CSR sparse layout via prefix sum | Host nonzero+sort → in-kernel prefix sum + atomic counter |
| `autotune-aggregate-workload-key` | autotune keyed on aggregate workload size | Fixed config → autotune on total work (Msum) for irregular batches |

### Category I: NSA block-sparse selection (Batch 5, 4 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `block-sparse-gather` | block-sparse indexed gather via selection array | Dense iteration over all N → index-array block gather |
| `precompute-scalar-reduction` | hoist invariant row reduction out of per-block loop | Recompute delta per block → precompute once in separate pass |
| `block-compression-before-routing` | pool to coarse blocks, route on compressed reps | Score all fine positions → pool to blocks, score compressed, attend top-k |
| `sliding-window-plus-sparse` | decompose attention into local + global sparse, gated sum | Single attention pass → local window + global sparse with gates |

### Category J: Varlen packing + chunked reductions (Batch 6, 3 rules)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `varlen-packing` | pack variable-length sequences via cu_seqlens, no padding | Padded batch (waste on padding) → packed 1D buffer with cumsum offsets |
| `chunk-reset-cumsum` | cumsum that resets at chunk boundaries | Full cumsum → cumsum reset every chunk_size elements |
| `cumsum-additive-bias` | additive bias via cumulative sum of a gate | O(N²) bias matrix → O(N) cumsum difference |

### Category K: Autotune dispatch (Batch 7, 4 rules — burn-cubecl)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `panic-safe-autotune-candidate` | panic-resilient autotune candidate | Direct call (panics on bad shape) → catch_unwind → Err (autotune skips) |
| `multi-priority-tune-groups` | shape-keyed priority-ranked autotune groups | Flat candidate list → priority-ranked groups (MAX/HIGH/MIN/NEVER) keyed on shape |
| `shape-kind-dispatch` | matmul-kind-keyed kernel family dispatch | Single general GEMM for all shapes → GEMV for matvec, GEMM for general |
| `vector-width-pow2-sweep` | vectorization width as an autotune dimension | Fixed vector width → sweep C ∈ {1,2,4,8,16,32,64} as autotune candidates |

### Category L: Conv-as-GEMM + memory layout (Batch 7, 5 rules — burn-cubecl)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `conv-1x1-as-matmul` | 1×1 convolution as direct matmul (skip im2col) | im2col expansion + GEMM → reshape to 2D + matmul (no column expansion) |
| `contiguity-guard-before-reshape` | stride-contiguity guard skips unnecessary relayout copy | Always-copy-into-contiguous → check stride contiguity, skip copy if already compatible |
| `cube-count-bounded-batch-pack` | grid-limit-aware batch packing | One batch per launch → pack max batches ≤ cube-count limit (find largest divisor) |
| `stride-align-vectorization-key` | stride-byte-alignment determines vectorization width | Fixed vectorization → trailing-zeros of (stride × bytes) picks vector width |
| `anchor-shape-bucketing` | shape bucketing for autotune cache reuse | Exact-shape autotune key → bucketed key (nearest N) so similar shapes share cache |

### Category M: Kernel parametric polymorphism (Batch 8, 3 rules — burn-cubecl)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `op-family-type-dispatch` | parametric kernel polymorphism over op trait | N copy-pasted kernels (one char differs) → one kernel body + `OpFamily` trait with N associated-type impls |
| `inplace-output-alias` | output buffer reuse via mutability check (skip allocation) | Fresh output alloc every call → `can_mut` check aliases output to disposable input (zero alloc) |
| `layout-permute-for-coalescing` | memory layout permutation for coalesced channel access | NCHW strided channel access → permute to NHWC (channel innermost, coalesced), permute back |

### Category N: dtype + alignment specialization (Batch 8, 2 rules — burn-cubecl)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `dtype-specialized-float-kind` | dtype-keyed intrinsic dispatch for low-precision floats | Generic powf (absent/imprecise for f16/bf16) → match on `FloatKind`, cast to concrete dtype, call its intrinsic |
| `alignment-gated-zero-copy-view` | alignment-gated zero-copy slice view via handle offset | Always-copy slice → if byte offsets are alignment-aligned, return a VIEW (offset-only metadata, zero copy) |

### Category O: IR strength reduction (Batch 8, 4 rules — cubecl-opt)

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `strength-reduce-mul-pow2` | IR strength reduction: multiply by power of 2 → left shift | `x * 16` (mul unit) → `x << 4` (shift unit, frees throughput-limited mul) |
| `strength-reduce-mul-pow2-plus-minus-one` | IR strength reduction: multiply by (2^k ± 1) → shift ± self | `x * 15` (mul) → `(x << 4) - x` (shift + sub, 2 instructions, no mul unit) |
| `strength-reduce-div-pow2` | IR strength reduction: divide by power of 2 → right shift | `x / 8` (slow div unit, tens of cycles) → `x >> 3` (1 cycle, exact for unsigned) |
| `strength-reduce-mod-pow2` | IR strength reduction: modulo by power of 2 → bitwise and mask | `x % 64` (div/mod unit) → `x & 63` (bitwise-and unit, exact for unsigned) |

## Source

- [`src/domains/kernel_opt/mod.rs`](../../src/domains/kernel_opt/mod.rs) — `KernelRule` + `KernelOptDomain`
- [`src/domains/kernel_opt/corpus.rs`](../../src/domains/kernel_opt/corpus.rs) — `build_kernel_seed_corpus()`
- [`src/domains/kernel_opt/router.rs`](../../src/domains/kernel_opt/router.rs) — `KernelExpertRouter` + `pick_domain`
- [`src/domains/kernel_opt/thermal.rs`](../../src/domains/kernel_opt/thermal.rs) — `KernelThermalRouter` + `ThermalTier`
- [`src/domains/kernel_opt/verify.rs`](../../src/domains/kernel_opt/verify.rs) — `KernelVerifier` (stub)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Research 002](../../.research/002_popcorn_kernel_opt_distillation_batch1.md) — Batch 1 popcorn distillation methodology
- [Research 003](../../.research/003_popcorn_kernel_opt_distillation_batch2.md) — Batch 2 flash-attention tiling
- [Research 004](../../.research/004_popcorn_kernel_opt_distillation_batch3.md) — Batch 3 linear-attention recurrence
- [Research 005](../../.research/005_popcorn_kernel_opt_distillation_batch4.md) — Batch 4 MoE dispatch + backward pass
- [Research 006](../../.research/006_popcorn_kernel_opt_distillation_batch5.md) — Batch 5 NSA block-sparse selection
- [Research 007](../../.research/007_popcorn_kernel_opt_distillation_batch6.md) — Batch 6 varlen + chunked reductions
- [Research 008](../../.research/008_burn_cubecl_kernel_opt_distillation_batch7.md) — Batch 7 burn-cubecl autotune dispatch + conv-as-GEMM + layout
- [Research 009](../../.research/009_burn_cubecl_cubecl_opt_kernel_opt_distillation_batch8.md) — Batch 8 burn-cubecl kernel polymorphism + dtype/alignment + cubecl-opt IR strength reduction
- [Proposal 012](../../.proposals/) — the absorbed GPU kernel optimizer design
- [Plan 558](../../../katgpt-rs/.plans/) — `pick_domain` substrate (katgpt-core)
- [Research 453](../../../katgpt-rs/.research/) — LatentMoE variable-rank principle
