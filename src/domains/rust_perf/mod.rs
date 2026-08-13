//! The rust performance healing domain.
//!
//! This is the second `HealDomain` impl (Plan 001 Phase 2). **Promoted to
//! default-on 2026-08-13** after the perf-bench Pareto validator GOAT gate
//! passed (Bench 013 G1+G5). It owns:
//! - [`PerfRule`] — the corpus entry type (rule id, category, buggy, fixed,
//!   optional bench spec)
//! - [`RustPerfDomain`] — the `HealDomain` impl (`build_corpus`, validate)
//! - The seed corpus distilled from the `rust-optimize` skill checklist
//! - The verifier (behind `rust_perf_verify`): runs `cargo test` as subprocess
//! - The bench validator (behind `rust_perf_bench`): runs generated
//!   microbenchmarks across multiple sizes + reports a Pareto verdict — the
//!   **promotion gate** for the domain (Proposal 001 caveat 3)
//!
//! ## Validator hierarchy (strongest last)
//!
//! 1. **L2 pruner** (always-on, modelless): `syn`-based syntax parse check.
//! 2. **`rust_perf_verify`** (opt-in): `cargo test` re-run (compiles + tests pass).
//! 3. **`rust_perf_bench`** (opt-in): generated microbenchmarks at multiple
//!    input sizes → Pareto verdict (faster on N of M sizes). This is the
//!    **real perf ground truth** — the gate that can promote `rust_perf`
//!    to default-on.
//!
//! Rules without a [`BenchSpec`] are honestly marked `NotBenchable` — they
//! need external runtimes (Bevy, WASM) a temp crate cannot provide.

#![allow(clippy::doc_markdown)]
#![allow(clippy::needless_raw_string_hashes)]
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

#[cfg(feature = "rust_perf_bench")]
pub mod bench;

#[cfg(feature = "rust_perf_verify")]
pub mod verify;

/// A single rust performance rule — the corpus entry type for [`RustPerfDomain`].
///
/// Distilled from the `rust-optimize` skill checklist. Each entry pairs a
/// **buggy pattern** (the slow form) with a **fixed pattern** (the fast form)
/// plus metadata (rule id, display name, category). The L1 drafter reads the
/// `fixed_code` field, the L2 pruner parses it, the verifier (behind
/// `rust_perf_verify`) runs `cargo test`, and the bench validator (behind
/// `rust_perf_bench`) runs generated microbenchmarks if a `bench_spec` is
/// present.
///
/// The `bench_spec` field is **optional** — rules needing external runtimes
/// (Bevy, WASM) are honestly left without one and reported as `NotBenchable`
/// by the validator.
///
/// The `context_notes` field documents rules where the bench validator found
/// the "fix" is **not a default-configuration win** (e.g. the compiler already
/// optimizes the buggy form, or the benefit only appears under a specific
/// runtime regime like multi-threaded contention). When `Some`, consumers of
/// the corpus should treat the rule as **context-dependent** — present it as
/// a documented pattern, not an unconditional suggestion. The two rules
/// flagged in [Bench 013](../../../../.benchmarks/013_rust_perf_bench_pareto_goat.md)
/// (`precompute-table`, `papaya-lockfree`) carry this annotation.
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
    /// Honest annotation for rules where the bench validator found the "fix"
    /// is not a default-configuration win (Bench 013). `None` for
    /// unconditional wins + honestly `NotBenchable` rules.
    ///
    /// Backward-compatible via `#[serde(default)]` — older freeze/thaw
    /// snapshots (pre-this-field) deserialize as `None`.
    #[serde(default)]
    pub context_notes: Option<String>,
    /// Optional microbenchmark spec for the `rust_perf_bench` validator.
    ///
    /// `None` means the rule is `NotBenchable` (needs an external runtime).
    /// `Some(spec)` means the bench validator can generate + run a
    /// microbenchmark for this rule.
    #[cfg(feature = "rust_perf_bench")]
    pub bench_spec: Option<bench::BenchSpec>,
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

    #[inline]
    fn context_notes(&self) -> Option<&str> {
        self.context_notes.as_deref()
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
