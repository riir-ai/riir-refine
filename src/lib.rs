//! # riir-clippy — Clippy-Healing Drafter+Pruner Pipeline
//!
//! A modelless-first pipeline that heals `cargo clippy` warnings using:
//!
//! 1. **L0 — Rule distillation**: each clippy lint distilled into a named rule
//!    with a bounded fix-space, stored in a BLAKE3-committed corpus.
//! 2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec proposes
//!    fixes using frozen direction vectors. Zero-alloc hot path. Template
//!    fallback when no weights loaded.
//! 3. **L2 — Validator**: `syn`-based syntax parse check (always-on,
//!    modelless). Lint resolution verification via `cargo clippy` re-run is
//!    Phase 3 (feature-gated `clippy_verify`, opt-in).
//! 4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration
//!    when multiple valid fixes exist. Reuses the algorithm pattern from
//!    `katgpt-ruliology` (Bench 572) without a code-level dep — clippy fixes
//!    have ground-truth payoff, not tournament payoffs.
//! 5. **L4 — `LoRA` fallback** (deferred): adaptive learning on the residual
//!    lints L0-L3 miss. Last resort, not co-equal path.
//!
//! ## Architecture
//!
//! This crate borrows the **drafter+pruner+corpus pattern** from
//! `riir-games-quest/quest_grammar` (`SealQuestCorpus` + `TernaryDraftModel` +
//! `ConstraintPruner`) but has **zero game-domain coupling**. It is a developer
//! tool, not game content.
//!
//! ## Modelless-first mandate
//!
//! The primary path (L0-L3) is entirely modelless — no training, no backprop.
//! The `LoRA` fallback (L4) is the conservative last resort, not the co-equal
//! path. This matches the user's core direction: "old-school `LoRA` is really
//! later."

#![warn(clippy::all, clippy::pedantic)]

pub mod adaptive_drafter;
pub mod corpus;
pub mod drafter;
pub mod embedder;
pub mod proposers;
pub mod pruner;
pub mod seed_corpus;

// Phase 3: optional lint resolution verification via real cargo clippy.
// Opt-in because it's slow (seconds) and requires cargo+clippy on PATH.
#[cfg(feature = "clippy_verify")]
pub mod clippy_verifier;

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

pub use corpus::{ClippyCorpus, ClippyFixEntry};
pub use drafter::ClippyTernaryDrafter;
pub use embedder::{Direction, EMBED_DIM, embed, embed_into};
pub use pruner::RustAnalyzerPruner;
pub use seed_corpus::build_seed_corpus;

// Issue 010: Stage 2 adaptive drafter (mirrors `AdaptiveQuestDrafter`).
pub use adaptive_drafter::ClippyAdaptiveDrafter;

#[cfg(feature = "clippy_verify")]
pub use clippy_verifier::ClippyVerifier;

#[cfg(feature = "ruliology_search")]
pub use ruliology_search::{RuliologySearch, ScoredCandidate};

#[cfg(feature = "self_evolve")]
pub use evolve::{EvolveConfig, LatentSkillEvolution};
#[cfg(feature = "self_evolve")]
pub use memory::{FixTrajectory, LatentFixMemory, LatentQuery, ScoredTrajectory};
#[cfg(feature = "self_evolve")]
pub use self_evolve::{HealPipeline, HealResult, HealSource, TuneReport};
