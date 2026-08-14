//! The GPU kernel optimization healing domain.
//!
//! This is the third `HealDomain` impl (Plan 001 Phase 3). It absorbs
//! Proposal 012's design (thermal-tier + latent-MoE + numeric validator) into
//! the multi-domain framework. The seed corpus is distilled from popcorn
//! (Batches 1-13, COMPLETE) + riir-gpu production kernels (Batches 14-26,
//! OPEN — ~177 rules and growing).
//!
//! ## What ships now vs deferred (Proposal 012 §Phase 0)
//!
//! | Component | Ships now | Deferred |
//! |---|---|---|
//! | `KernelOptDomain: HealDomain` | ✅ | |
//! | Seed corpus | ✅ 177 rules (popcorn 1-13 + riir-gpu 14-26) | riir-gpu mining continues |
//! | `KernelExpertRouter` (`pick_domain`) | ✅ (8-dim activity, 15/15 reachable, Plan 004) | Mined centroids via `CommittedFieldBlend` |
//! | Routed heal pipeline (`kernel_heal_demo` example) | ✅ (Plan 003/004 composition) | |
//! | `KernelThermalRouter` (5-tier stub) | ✅ (returns Plasma) | Warm/Cold `LoRA` (4090, riir-train) |
//! | `KernelVerifier` (compile-only) | ✅ (behind `kernel_verify`) | Full numeric compare vs reference |
//! | L4 `LoRA` fallback | | ✅ Deferred across all domains |
//!
//! ## Domain classification (per AGENTS.md)
//!
//! - Kernel **shapes** (M, N, K, dtype, hw) = raw physical (deterministic,
//!   replayable — needed for quorum sync of kernel selection decisions).
//! - Expert **directions** = latent (dot-projection onto learned centroids,
//!   projected to a scalar expert index via `pick_domain`). NOT synced — only
//!   the resulting expert index crosses the sync boundary.
//! - Thermal tier = raw categorical (Plasma/Hot/Warm/Cold/Freeze — enum, 1 byte).
//!
//! ## Modelless-first mandate
//!
//! The Plasma tier (modelless cache hit on M3 Metal) is the primary path.
//! Warm/Cold tiers (`LoRA` draft + full validate on 4090 CUDA) are the
//! conservative last resort, **deferred** to riir-train. The scaffold returns
//! Plasma for all calls until the 4090 path lands.

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod corpus;
pub mod router;
pub mod thermal;

#[cfg(feature = "kernel_verify")]
pub mod verify;

/// The 15 corpus categories (A–O), one per expert in [`router::KernelExpertRouter`].
///
/// `#[repr(u8)]` for 1-byte sync efficiency (matches the [`thermal::ThermalTier`]
/// convention). The discriminant order matches the corpus section order in
/// [`corpus::build_kernel_seed_corpus`] — category A is memory access (idx 0),
/// category O is IR strength reduction (idx 14).
///
/// See [`corpus::KERNEL_CATEGORIES`] for the category → `rule_ids` mapping.
/// See [Plan 003](../../../.plans/003_kernel_opt_router_corpus_wiring.md) for the
/// design rationale (one expert per category, not one per rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum KernelCategory {
    /// A: Memory access patterns (coalescing, vectorized load, grid-stride, tiling, masking).
    #[default]
    MemoryAccess = 0,
    /// B: Compute patterns (autotune config, precision upcast, branchless, unroll, inline).
    Compute = 1,
    /// C: Fusion patterns (norm+act+linear, quant+matmul, masked softmax, cross-entropy).
    Fusion = 2,
    /// D: Dispatch patterns (SM-aware block size, predicate-gated backend, pow2 alignment).
    Dispatch = 3,
    /// E: Parallelism patterns (multi-dim grid, block-count clamp, backward-as-conjugate).
    Parallelism = 4,
    /// F: Flash-attention tiling (online softmax, exp2/log2, block pointers, causal split).
    FlashAttnTiling = 5,
    /// G: Linear-attention recurrence (forget gate, Householder update, chunkwise parallel).
    LinearAttnRecurrence = 6,
    /// H: `MoE` dispatch + backward pass (grouped launch, fused matmul, atomic scatter).
    MoeDispatchBackward = 7,
    /// I: NSA block-sparse selection (block gather, scalar reduction, compression, sliding).
    NsaBlockSparse = 8,
    /// J: Varlen packing + chunked reductions (`cu_seqlens`, chunk-reset cumsum, additive bias).
    VarlenChunkedReduction = 9,
    /// K: Autotune dispatch (panic-safe candidates, multi-priority groups, shape-kind, vector-width).
    AutotuneDispatch = 10,
    /// L: Conv-as-GEMM + memory layout (1×1 matmul, contiguity guard, batch pack, stride align).
    ConvGemmLayout = 11,
    /// M: Kernel parametric polymorphism (op-family trait, in-place alias, layout permute).
    KernelPolymorphism = 12,
    /// N: dtype + alignment specialization (`FloatKind` intrinsic, alignment-gated zero-copy view).
    DtypeAlignment = 13,
    /// O: IR strength reduction (mul/div/mod by pow2 → shift/mask).
    IrStrengthReduction = 14,
}

impl KernelCategory {
    /// Human-readable category label (for logging / debugging).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MemoryAccess => "A: memory access",
            Self::Compute => "B: compute",
            Self::Fusion => "C: fusion",
            Self::Dispatch => "D: dispatch",
            Self::Parallelism => "E: parallelism",
            Self::FlashAttnTiling => "F: flash-attn tiling",
            Self::LinearAttnRecurrence => "G: linear-attn recurrence",
            Self::MoeDispatchBackward => "H: MoE dispatch + backward",
            Self::NsaBlockSparse => "I: NSA block-sparse",
            Self::VarlenChunkedReduction => "J: varlen + chunked reduction",
            Self::AutotuneDispatch => "K: autotune dispatch",
            Self::ConvGemmLayout => "L: conv-as-GEMM + layout",
            Self::KernelPolymorphism => "M: kernel polymorphism",
            Self::DtypeAlignment => "N: dtype + alignment",
            Self::IrStrengthReduction => "O: IR strength reduction",
        }
    }
}

/// A single GPU kernel optimization rule — the corpus entry type for
/// [`KernelOptDomain`].
///
/// Each entry pairs a **reference shape** (the slow/reference kernel) with an
/// **optimized pattern** (the hand-tuned / auto-tuned kernel) + metadata (rule
/// id, GPU concept, shape signature). The corpus is currently a **stub** with
/// placeholder rules; real distillation from popcorn is deferred (Proposal 012
/// §Phase 0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelRule {
    /// Short kebab-case rule id (e.g., "`matvec-coalesce`", "`gemm-tiling`").
    pub rule_id: String,
    /// The GPU optimization concept (e.g., "coalesced memory access",
    /// "shared-memory tiling", "register blocking").
    pub gpu_concept: String,
    /// The reference (slow / naive) kernel code.
    pub ref_shape: String,
    /// The optimized kernel code pattern.
    pub opt_pattern: String,
}

impl HealEntry for KernelRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.ref_shape
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.opt_pattern
    }
}

/// The GPU kernel optimization healing domain — `HealDomain` impl.
///
/// Corpus entry type is [`KernelRule`]. Validator verdict is a
/// [`KernelVerdict`]:
/// - Without `kernel_verify`: parse-only verdict (syntax check via the shared
///   `RustAnalyzerPruner` — the kernel code is Rust-shaped pseudocode in the
///   scaffold; real CUDA/WGSL compile lands with the full validator).
/// - With `kernel_verify`: compile-only verdict (`CubeCL` compile check). Full
///   numeric compare vs reference is deferred (Proposal 012 §Phase 0).
#[derive(Debug, Clone, Copy, Default)]
pub struct KernelOptDomain;

/// The parse-only verdict (always available — no feature gate needed).
///
/// When `kernel_verify` is enabled, the richer `verify::KernelVerifyResult`
/// is the domain's `Verdict` type instead.
#[derive(Debug, Clone)]
pub struct KernelVerdict {
    /// Whether the optimized pattern parses as valid Rust (scaffold) or
    /// compiles via `CubeCL` (with `kernel_verify`).
    pub parses: bool,
}

#[cfg(not(feature = "kernel_verify"))]
impl HealDomain for KernelOptDomain {
    type Entry = KernelRule;
    type Verdict = KernelVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_kernel_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        // The L2 pruner is domain-agnostic syntax validation. We pass the
        // rule_id as the "lint name" slot — the pruner doesn't interpret it.
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        let result = pruner.validate(proposed_fix, rule_id);
        KernelVerdict {
            parses: result.parses,
        }
    }
}

#[cfg(feature = "kernel_verify")]
impl HealDomain for KernelOptDomain {
    type Entry = KernelRule;
    type Verdict = verify::KernelVerifyResult;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_kernel_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        verify::KernelVerifier::new().verify(proposed_fix, rule_id)
    }
}
