//! The GPU kernel optimization healing domain.
//!
//! This is the third `HealDomain` impl (Plan 001 Phase 3). It absorbs
//! Proposal 012's design (thermal-tier + latent-MoE + numeric validator) into
//! the multi-domain framework. The corpus distillation from popcorn is
//! **DEFERRED** (Proposal 012 §Phase 0, ~2 weeks reading job); this module
//! ships a scaffold with 1-2 placeholder rules so the trait impl is exercised.
//!
//! ## What ships now vs deferred (Proposal 012 §Phase 0)
//!
//! | Component | Ships now | Deferred |
//! |---|---|---|
//! | `KernelOptDomain: HealDomain` | ✅ | |
//! | Seed corpus (1-2 placeholders) | ✅ | Real popcorn-distilled corpus |
//! | `KernelExpertRouter` (`pick_domain`) | ✅ (identity directions) | Per-expert centroids from popcorn |
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
