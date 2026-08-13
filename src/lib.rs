//! # riir-clippy — Multi-Domain Modelless Code Healer
//!
//! A modelless-first pipeline that heals code (clippy lints, perf issues,
//! kernel opt) using a drafter+pruner+corpus pattern. The framework is
//! domain-pluggable behind the [`HealDomain`] trait (Plan 001).
//!
//! ## Layers
//!
//! 1. **L0 — Rule distillation**: each rule distilled into a named entry
//!    with a bounded fix-space, stored in a BLAKE3-committed corpus.
//! 2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec proposes
//!    fixes using frozen direction vectors. Zero-alloc hot path.
//! 3. **L2 — Validator**: `syn`-based syntax parse check (always-on,
//!    modelless). Domain-specific lint resolution verification is opt-in
//!    (`clippy_verify` for the clippy domain).
//! 4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration
//!    when multiple valid fixes exist.
//! 5. **L4 — `LoRA` fallback** (deferred): adaptive learning on the residual
//!    cases L0-L3 miss. Last resort, not co-equal path.
//!
//! ## Architecture
//!
//! This crate borrows the **drafter+pruner+corpus pattern** from
//! `riir-games-quest/quest_grammar` (`SealQuestCorpus` + `TernaryDraftModel` +
//! `ConstraintPruner`) but has **zero game-domain coupling**. It is a developer
//! tool, not game content.
//!
//! ## Multi-domain (Plan 001)
//!
//! The framework is domain-pluggable behind [`HealDomain`]. Three domains ship:
//! - **`clippy_lints`** (default-on) — `clippy::*` lint healing (the original domain)
//! - **`rust_perf`** (default-on, Phase 2) — `rust-optimize` perf healing (promoted 2026-08-13 after Bench 013 GOAT)
//! - **`kernel_heal`** (opt-in, Phase 3) — GPU kernel optimization healing (absorbs `Proposal 012`; 4090 `LoRA` deferred)
//!
//! Cross-domain auto-routing is NOT in scope — each domain is selected by
//! enabling its feature flag.
//!
//! ## Modelless-first mandate
//!
//! The primary path (L0-L3) is entirely modelless — no training, no backprop.
//! The `LoRA` fallback (L4) is the conservative last resort, not the co-equal
//! path. This matches the user's core direction: "old-school `LoRA` is really
//! later."

#![warn(clippy::all, clippy::pedantic)]

// ── Domain-agnostic core (always compiled) ─────────────────────────────────
pub mod adaptive_drafter;
pub mod corpus;
pub mod domain;
pub mod drafter;
pub mod embedder;
pub mod pruner;

// ── Domain modules (feature-gated) ─────────────────────────────────────────
pub mod domains;

// ── Opt-in layers ──────────────────────────────────────────────────────────

// L3: optional ruliology Pareto-front candidate search.
// Opt-in refinement layer — runs once per lint invocation (not per-tick).
// Reuses the Pareto-front pattern from katgpt-ruliology (Bench 572) without
// a code-level dep (see .issues/005 for the consume-vs-build verdict).
#[cfg(feature = "ruliology_search")]
pub mod ruliology_search;

// Issue 004 + Issue 008: latent-first self-evolution loop (memory + EMA direction update).
// Opt-in (deployment constraint: clippy_verify is itself opt-in).
// Validated: G1-G5 PASS (Issue 004) + G6/G7 real cargo clippy PASS (Issue 008).
#[cfg(feature = "self_evolve")]
pub mod evolve;
#[cfg(feature = "self_evolve")]
pub mod memory;
#[cfg(feature = "self_evolve")]
pub mod self_evolve;

// ── Back-compat re-exports (preserve pre-refactor API names) ───────────────
// These type aliases + re-exports keep existing call sites compiling unchanged
// after the Plan 001 Phase 1 framework refactor.

/// Back-compat: `ClippyTernaryDrafter` = `TernaryDrafter` (Plan 001 rename).
pub type ClippyTernaryDrafter = drafter::TernaryDrafter;

/// Back-compat: `ClippyAdaptiveDrafter` = `AdaptiveDrafter` (Plan 001 rename).
pub type ClippyAdaptiveDrafter = adaptive_drafter::AdaptiveDrafter;

/// Back-compat: re-export drafter types at the old paths.
pub use drafter::{DraftSource, DraftedFix, TernaryDrafter};
pub use adaptive_drafter::AdaptiveDrafter;
pub use corpus::Corpus;
pub use domain::{HealDomain, HealEntry};
pub use embedder::{Direction, EMBED_DIM, embed, embed_into};
pub use pruner::RustAnalyzerPruner;

// ── Clippy domain re-exports (behind `clippy_lints` feature, default-on) ──
#[cfg(feature = "clippy_lints")]
pub use domains::clippy_lints::{ClippyFixEntry, ClippyLintDomain};

/// Back-compat: `ClippyCorpus` = `Corpus<ClippyLintDomain>` (Plan 001 rename).
#[cfg(feature = "clippy_lints")]
pub type ClippyCorpus = Corpus<ClippyLintDomain>;

/// Back-compat: `build_seed_corpus` builds the clippy seed corpus.
#[cfg(feature = "clippy_lints")]
pub use domains::clippy_lints::corpus::build_seed_corpus;

/// Back-compat: `ClippyVerifier` (behind `clippy_verify`).
#[cfg(feature = "clippy_verify")]
pub use domains::clippy_lints::verify::{ClippyVerifyResult, ClippyVerifier};

// ── rust_perf domain re-exports (behind `rust_perf` feature, default-on) ──
#[cfg(feature = "rust_perf")]
pub use domains::rust_perf::{PerfParseVerdict, PerfRule, RustPerfDomain};

/// Back-compat: `build_perf_seed_corpus` (behind `rust_perf`).
#[cfg(feature = "rust_perf")]
pub use domains::rust_perf::corpus::build_perf_seed_corpus;

/// `rust_perf` verifier (behind `rust_perf_verify`).
#[cfg(feature = "rust_perf_verify")]
pub use domains::rust_perf::verify::{PerfVerifyResult, PerfVerifier};

/// `rust_perf` bench validator (behind `rust_perf_bench`).
#[cfg(feature = "rust_perf_bench")]
pub use domains::rust_perf::bench::{
    BenchOverall, BenchSpec, BenchVerdict, NotBenchable, ParetoVerdict,
    PerfBenchValidator, SizeOutcome, SizeResult,
};

// ── kernel_opt domain re-exports (behind `kernel_heal` feature, opt-in) ────
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::{KernelOptDomain, KernelRule, KernelVerdict};

/// Back-compat: `build_kernel_seed_corpus` (behind `kernel_heal`).
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::corpus::build_kernel_seed_corpus;

/// `kernel_opt` expert router (behind `kernel_heal`).
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::router::{KernelExpertRouter, KERNEL_EXPERT_NAMES, NUM_KERNEL_EXPERTS, ACTIVITY_DIM};

/// `kernel_opt` thermal router (behind `kernel_heal`).
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::thermal::{KernelThermalRouter, ThermalTier};

/// `kernel_opt` verifier (behind `kernel_verify`).
#[cfg(feature = "kernel_verify")]
pub use domains::kernel_opt::verify::{KernelVerifyResult, KernelVerifier};

#[cfg(feature = "ruliology_search")]
pub use ruliology_search::{RuliologySearch, ScoredCandidate};

#[cfg(feature = "self_evolve")]
pub use evolve::{EvolveConfig, LatentSkillEvolution};
#[cfg(feature = "self_evolve")]
pub use memory::{FixTrajectory, LatentFixMemory, LatentQuery, ScoredTrajectory};
#[cfg(feature = "self_evolve")]
pub use self_evolve::{HealPipeline, HealResult, HealSource, TuneReport};
