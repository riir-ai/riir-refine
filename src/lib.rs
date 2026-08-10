//! # riir-clippy — Clippy-Healing Drafter+Pruner Pipeline
//!
//! A modelless-first pipeline that heals `cargo clippy` warnings using:
//!
//! 1. **L0 — Rule distillation**: each clippy lint distilled into a named rule
//!    with a bounded fix-space, stored in a BLAKE3-committed corpus.
//! 2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec proposes
//!    fixes using frozen direction vectors. Zero-alloc hot path. Template
//!    fallback when no weights loaded.
//! 3. **L2 — Validator**: native rust-analyzer pruner validates parse +
//!    type-check. WASM variant (opt-in) for sandboxed distribution.
//! 4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration
//!    when multiple valid fixes exist.
//! 5. **L4 — LoRA fallback** (deferred): adaptive learning on the residual
//!    lints L0-L3 miss. Last resort, not co-equal path.
//!
//! ## Architecture
//!
//! This crate borrows the **drafter+pruner+corpus pattern** from
//! `riir-games-quest/quest_grammar` (SealQuestCorpus + TernaryDraftModel +
//! ConstraintPruner) but has **zero game-domain coupling**. It is a developer
//! tool, not game content.
//!
//! ## Modelless-first mandate
//!
//! The primary path (L0-L3) is entirely modelless — no training, no backprop.
//! The LoRA fallback (L4) is the conservative last resort, not the co-equal
//! path. This matches the user's core direction: "old-school LoRA is really
//! later."

#![warn(clippy::all, clippy::pedantic)]

pub mod corpus;
pub mod drafter;
pub mod pruner;

pub use corpus::ClippyCorpus;
pub use drafter::ClippyTernaryDrafter;
pub use pruner::RustAnalyzerPruner;
