# `kernel_opt` domain (Plan 001 Phase 3, absorbs Proposal 012)

> The third `HealDomain` impl. Heals GPU kernel optimization. Absorbs
> Proposal 012's design (thermal-tier + latent-MoE + numeric validator) into
> the multi-domain framework. **4090 LoRA deferred** across all tiers above
> Plasma.

## What ships vs deferred (Proposal 012 §Phase 0)

| Component | Ships now | Deferred |
|---|---|---|
| `KernelOptDomain: HealDomain` | ✅ | |
| Seed corpus (Batch 1: 21 rules) | ✅ | Full corpus (~300-500 rules, ~15-25 batches) |
| `KernelExpertRouter` (`pick_domain`) | ✅ (identity directions) | Per-expert centroids from popcorn |
| `KernelThermalRouter` (5-tier stub) | ✅ (returns Plasma) | Warm/Cold LoRA (4090, riir-train) |
| `KernelVerifier` (compile-only) | ✅ (behind `kernel_verify`) | Full numeric compare vs reference |
| L4 LoRA fallback | | ✅ Deferred across all domains |

## Corpus

Batch 1 (Research 002): 21 rules total — 2 original placeholders + 19 popcorn-distilled
across 5 categories. Full corpus target: ~300-500 rules in ~15-25 batches.

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

## Source

- [`src/domains/kernel_opt/mod.rs`](../../src/domains/kernel_opt/mod.rs) — `KernelRule` + `KernelOptDomain`
- [`src/domains/kernel_opt/corpus.rs`](../../src/domains/kernel_opt/corpus.rs) — `build_kernel_seed_corpus()`
- [`src/domains/kernel_opt/router.rs`](../../src/domains/kernel_opt/router.rs) — `KernelExpertRouter` + `pick_domain`
- [`src/domains/kernel_opt/thermal.rs`](../../src/domains/kernel_opt/thermal.rs) — `KernelThermalRouter` + `ThermalTier`
- [`src/domains/kernel_opt/verify.rs`](../../src/domains/kernel_opt/verify.rs) — `KernelVerifier` (stub)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Research 002](../../.research/002_popcorn_kernel_opt_distillation_batch1.md) — Batch 1 popcorn distillation methodology
- [Proposal 012](../../.proposals/) — the absorbed GPU kernel optimizer design
- [Plan 558](../../../katgpt-rs/.plans/) — `pick_domain` substrate (katgpt-core)
- [Research 453](../../../katgpt-rs/.research/) — LatentMoE variable-rank principle
