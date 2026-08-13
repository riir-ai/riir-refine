//! The rust performance healing domain.
//!
//! This is the second `HealDomain` impl (Plan 001 Phase 2). It owns:
//! - [`PerfRule`] — the corpus entry type (rule id, category, buggy, fixed)
//! - [`RustPerfDomain`] — the `HealDomain` impl (`build_corpus`, validate)
//! - The seed corpus distilled from the `rust-optimize` skill checklist
//! - The verifier (behind `rust_perf_verify`): runs `cargo test` as subprocess
//!   (perf-bench Pareto verdict deferred per Proposal 001 caveat 3)
//!
//! ## Caveat (Proposal 001 caveat 3)
//!
//! The validator is the **weakest link** across the three domains. clippy has
//! binary ground truth (lint satisfied: yes/no). kernel has numeric ground
//! truth (output matches reference within tolerance). perf has **no single
//! ground truth** — "faster" depends on workload, hardware, input size. The
//! first cut ships a **degraded validator** (`cargo test` pass + syntax), not
//! a perf-bench Pareto verdict. The real perf-bench validator lands as a
//! separate plan.
//!
//! ## Domain classification (per AGENTS.md sync boundary rule)
//!
//! All fields here are **raw physical**: rule identifiers + code bytes. No
//! latent / emotion / affect signal. The validator's verdict (`PerfVerdict`)
//! is a categorical (test pass / fail) + optional numeric (perf delta) — both
//! deterministic + replayable.

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod corpus;

#[cfg(feature = "rust_perf_verify")]
pub mod verify;

/// A single rust performance rule — the corpus entry type for [`RustPerfDomain`].
///
/// Distilled from the `rust-optimize` skill checklist. Each entry pairs a
/// **buggy pattern** (the slow form) with a **fixed pattern** (the fast form)
/// plus metadata (rule id, display name, category). The L1 drafter reads the
/// `fixed_pattern` field, the L2 pruner parses it, and the verifier (behind
/// `rust_perf_verify`) runs `cargo test`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfRule {
    /// Short kebab-case rule id (e.g., "`o1-lookup`", "`layout-reorder`").
    pub rule_id: String,
    /// Human-readable rule name (e.g., "O(n) scan → O(1) lookup").
    pub name: String,
    /// Skill-checklist category: `complexity`, `allocation`, `layout`,
    /// `concurrency`, `simd`, `caching`, `bevy`, `wasm`, `arithmetic`.
    pub category: String,
    /// The slow code pattern.
    pub buggy_code: String,
    /// The fast code pattern.
    pub fixed_code: String,
}

impl HealEntry for PerfRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.buggy_code
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.fixed_code
    }
}

/// The rust performance healing domain — `HealDomain` impl for perf rules.
///
/// Corpus entry type is [`PerfRule`]. Validator verdict is a [`PerfVerdict`]:
/// - Without `rust_perf_verify`: parse-only verdict (syntax check via the
///   shared `RustAnalyzerPruner`).
/// - With `rust_perf_verify`: richer verdict that also records whether `cargo
///   test` passed (perf-bench Pareto deferred).
#[derive(Debug, Clone, Copy, Default)]
pub struct RustPerfDomain;

/// The parse-only verdict (always available — no feature gate needed).
///
/// When `rust_perf_verify` is enabled, the richer `verify::PerfVerifyResult`
/// is the domain's `Verdict` type instead.
#[derive(Debug, Clone)]
pub struct PerfParseVerdict {
    /// Whether the fixed code parses as valid Rust.
    pub parses: bool,
}

#[cfg(not(feature = "rust_perf_verify"))]
impl HealDomain for RustPerfDomain {
    type Entry = PerfRule;
    type Verdict = PerfParseVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_perf_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        // The L2 pruner is domain-agnostic syntax validation. We pass the
        // rule_id as the "lint name" slot — the pruner doesn't interpret it.
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        let result = pruner.validate(proposed_fix, rule_id);
        PerfParseVerdict {
            parses: result.parses,
        }
    }
}

#[cfg(feature = "rust_perf_verify")]
impl HealDomain for RustPerfDomain {
    type Entry = PerfRule;
    type Verdict = verify::PerfVerifyResult;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_perf_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        verify::PerfVerifier::new().verify(proposed_fix, rule_id)
    }
}
