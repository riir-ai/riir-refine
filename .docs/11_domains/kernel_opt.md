# `kernel_opt` domain (Plan 001 Phase 3, absorbs Proposal 012)

> The third `HealDomain` impl. Heals GPU kernel optimization. Absorbs
> Proposal 012's design (thermal-tier + latent-MoE + numeric validator) into
> the multi-domain framework. **4090 LoRA deferred** across all tiers above
> Plasma.

## What ships vs deferred (Proposal 012 §Phase 0)

| Component | Ships now | Deferred |
|---|---|---|
| `KernelOptDomain: HealDomain` | ✅ | |
| Seed corpus (2 placeholders) | ✅ | Real popcorn-distilled corpus |
| `KernelExpertRouter` (`pick_domain`) | ✅ (identity directions) | Per-expert centroids from popcorn |
| `KernelThermalRouter` (5-tier stub) | ✅ (returns Plasma) | Warm/Cold LoRA (4090, riir-train) |
| `KernelVerifier` (compile-only) | ✅ (behind `kernel_verify`) | Full numeric compare vs reference |
| L4 LoRA fallback | | ✅ Deferred across all domains |

## Corpus

2 placeholder rules (real corpus deferred):

| Rule ID | GPU concept | Pattern |
|---|---|---|
| `matvec-coalesce` | coalesced memory access | Strided read (uncoalesced) → transposed coalesced read |
| `gemm-tiling` | shared-memory tiling | Naive GEMM (global mem) → tiled GEMM (shared mem reuse) |

The patterns are Rust-shaped pseudocode so the shared `RustAnalyzerPruner` can
validate them in the fast path. When the real corpus lands (real kernel
source), the validator upgrades to CubeCL compile + numeric compare.

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
- [Proposal 012](../../.proposals/) — the absorbed GPU kernel optimizer design
- [Plan 558](../../../katgpt-rs/.plans/) — `pick_domain` substrate (katgpt-core)
- [Research 453](../../../katgpt-rs/.research/) — LatentMoE variable-rank principle
