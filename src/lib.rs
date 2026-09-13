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
//! 5. **L4 — `LoRA` fallback** (opt-in, `l4_lora`): a generative model for the
//!    residual cases L0-L3 miss. Last resort, not co-equal path. The
//!    composition seam ships in [`l4_fallback`]; the adapter itself lives in
//!    riir-train (Plan 336).
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
//! The framework is domain-pluggable behind [`HealDomain`]. The shipped
//! domain set (Plan 001 + Plans 018/030/019/085):
//! - **`clippy_lints`** (default-on) — `clippy::*` lint healing (the original domain)
//! - **`rust_perf`** (default-on, Phase 2) — `rust-optimize` perf healing (promoted 2026-08-13 after Bench 013 GOAT)
//! - **`kernel_heal`** (opt-in, Phase 3) — GPU kernel optimization healing (absorbs `Proposal 012`; 4090 `LoRA` deferred)
//! - **`sec`** (opt-in, Plan 018) — security-pattern bounded fixes
//! - **`docker`** (default-on since 2026-08-28, Plan 030) — Dockerfile healing, the first no-`syn` scanner
//! - **`rustc_errors`** (default-on since 2026-08-29, Plan 019) — `--fix-compile` compile-error repair (driver-shaped, inert until invoked)
//! - **`dist`** (default-on since 2026-09-13, Issue 085) — Cargo.toml build-config advice (suggest-only)
//!
//! Cross-domain auto-routing ships behind the `latent_retrieval` feature
//! (Plan 005): real-file span-level `--suggest` via the riir-rag latent
//! substrate. Each domain is additionally selected by its feature flag for
//! the non-latent paths.
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
pub mod dispersion;
pub mod domain;
pub mod drafter;
pub mod edit_emitter;
pub mod elo;
pub mod embedder;
pub mod fmt;
// Issue 072 PoC — construction-pair factory. Two halves: the TYPE +
// fence-arithmetic half is dependency-free; the corpus half (strategy_for +
// construction_pairs → rustc_errors::corpus_gen + floor::MutatorKind) is
// cfg(rustc_errors) inside the file. UNGATED since Issue 089 T5: the
// provenance vocabulary (tags + weights) lives with the pair types and the
// ungated frontier_report consumes it — the module's own imports stay pure
// std with the corpus half cfg'd, so every feature build compiles it. (The
// 2026-09-10 each-feature sweep caught the latent-only arm importing the
// corpus unconditionally — E0433 on all 8 latent-without-rustc_errors combos;
// the any() gate was the transitional shape of that fix.)
pub mod frontier_pairs;
pub mod modelability;
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

// Issue 004 + 008 + 018: latent-first self-evolution layer (memory + EMA
// direction update + the HealPipeline + the standalone EvolveRecorder).
// UNCONDITIONAL since Issue 018 (promotion with graceful oracle degradation):
// the memory half needs no oracle and records wherever fixes are applied;
// the EMA learning half no-ops with ONE log line when the clippy_verify
// oracle is unavailable (see `recorder`). The `self_evolve` feature stays
// as a default-on back-compat alias (benches/examples required-features).
// Validated: G1-G5 PASS (Issue 004) + G6/G7 real cargo clippy PASS (Issue 008).
pub mod collapse_monitor;
pub mod component;
pub mod evolve;
pub mod gp_acquisition;
pub mod memory;
pub mod recorder;
pub mod selection_entropic;
pub mod selection_tether;
pub mod self_evolve;

// Plan 005: real-file span-level latent retrieval (`--suggest` MVP).
// Composes riir-rag (AstChunker + ModellessEmbedder + NeuronDbRetriever) +
// the katgpt-core routing primitive. Opt-in — pulls the riir-rag +
// riir-neuron-db path deps (heavy; default build stays light).
#[cfg(feature = "latent_retrieval")]
pub mod draft;

// Plan 019 T0.3: the retrieval-free compile-oracle substrate shared by
// `fix_verify` and the `rustc_errors` `--fix-compile` driver — the cargo
// check runner, the rustc-reported-path resolution, and the package /
// workspace root walk. Gated by its own feature so `rustc_errors` can
// consume it WITHOUT pulling `latent_retrieval` (riir-rag + riir-neuron-db)
// into a compile fixer.
#[cfg(feature = "compile_oracle")]
pub mod compile_oracle;

// Issue 015: `--fix --verify` — the compile gate as tooling for the edit
// path (apply → cargo check → rustc-JSON error attribution → span-level
// revert → re-check → report). Opt-in; implies latent_retrieval (extends
// the cargo-heal bin surface).
#[cfg(feature = "fix_verify")]
pub mod fix_verify;

// Plan 336 T4.4: the L0-L3 → L4 composition seam. Opt-in and inert without an
// installed fallback; the adapter lives in riir-train (needs the 27B GGUF + a
// GPU). Splitting the contract from the adapter is what makes G4 measurable
// here without the model — see the module docs for the four invariants.
#[cfg(feature = "l4_lora")]
pub mod l4_fallback;

// Plan 336 T4.4 transport client: talks to riir-train's
// `bonsai_clippy_l4_eval --serve` daemon (load once, serve many). std-only —
// no model dependency reaches this crate.
#[cfg(feature = "l4_lora")]
pub mod l4_daemon;

// Issue 040 — the fixer re-gate POC harness: partial-credit metrics (EM /
// fragment-parse / oracle-verified / verify-filtered acceptance) + the
// modelless pre-pass prompt + the CF Workers AI REST client. Opt-in; pulls
// `ureq` (third-party HTTP, not boundary-gated) + the retrieval + oracle
// substrates it composes. See `.issues/040_*` + Bench 049.
#[cfg(feature = "cf_workers_ai")]
pub mod cf_workers_ai;

// Issue 046 — the USD LLM daily-cap ledger (reserve-then-reconcile): the
// circuit breaker every LLM call through `CfClient::generate` gates through
// BEFORE any unattended loop runs (the Issue 040 169-request burn class).
// Same feature gate as the client it guards; no new deps.
#[cfg(feature = "cf_workers_ai")]
pub mod llm_spend;

// Issue 051 — groq-first free-tier LLM routing: the Groq chat client
// (OpenAI-compatible REST, reuses the optional ureq dep) + the groq→CF
// provider ladder (the `router` submodule, additionally gated on
// `cf_workers_ai` — it composes CfClient as the paid fallback leg).
// `GROQ_API_KEY` absent → pure-CF behavior, byte-identical to pre-051.
// See `.issues/051_groq_first_free_tier.md`.
#[cfg(feature = "groq_first")]
pub mod groq;

// Bench 669 consumer adoption (Plan 016): the input-scope gate at the heal()
// seam. `ScopeModel` wraps katgpt-core's `contrastive_scope` primitive — the
// 2D extension of the Issue 030 relevance gate (rule coverage × input scope).
// Opt-in and inert without an installed model.
#[cfg(feature = "contrastive_scope")]
pub mod scope_model;

// ── issue 088: the kat service plane moved to the riir-kat repo ──────────
// The whole plane (protocol tier + client plane + transports) now lives in
// the sibling leaf crate; these re-exports preserve EVERY historical path
// (`riir_clippy::kat_meter`, `crate::kat_redact`, `bin/heal.rs`'s ~40 refs,
// the integration tests) bit-identically. Gate-match = the historical root
// availability, mapped onto the member's tier features (root kat_service →
// member client tier; root kat_transport → member kat_transport tier;
// refill/consent need BOTH). Keepers below (sync_payload, pending
// estimator, promotion) stay healer-side: they import traj_store/score_bench.
#[cfg(feature = "kat_service")]
pub use riir_kat::{
    kat_account, kat_account_client, kat_burn_client, kat_lease_client, kat_meter,
    kat_payreq_client, kat_protocol_account, kat_protocol_burn, kat_protocol_lease,
    kat_protocol_payreq, kat_protocol_stake, kat_protocol_vote, kat_protocol_wetrun,
    kat_redact, kat_service_config, kat_stake_client, kat_vote_client, kat_wetrun,
};
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
pub use riir_kat::{kat_consent_client, kat_protocol_consent, kat_refill};
#[cfg(feature = "kat_service")]
pub use riir_kat::kat_units;
// The ungated machine-local trio + the free-tier protocol plumbing: the
// member keeps them behind `client`, but riir-clippy's contract is
// UNGATED (--unmine must work in every feature build; the consent-module
// rule) — the member compiles them for us at default features, and root
// consumers reach them through these ungated re-exports.
pub use riir_kat::{kat_mining_consent, kat_service_urls, kat_soft_gate};

// Proposal 005 Phase 0 T0.3 (Plan 027): the sync payload schema — the
// versioned NDJSON serialization contract over TrajStore rows (header +
// gated rows + BLAKE3 batch commitment + the account-key Ed25519 signature
// over the header — v2, riir-dapps Issue 008; the receiver's redact-version
// floor gates the ATTESTED version and the submitter is derived from the
// signed key). Behind BOTH kat_service AND latent_retrieval: it consumes the
// redaction gate (kat_service) and the TrajRowDoc wire source
// (latent_retrieval — the store's riir-neuron-db dep). kat_service alone
// stays dep-light; CI covers the combo standing (Layer 5b --lib
// --all-features + Layer 5c's integration binary).
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
pub mod kat_sync_payload;

// Proposal 005 Phase 0 T0.4 (Plan 028): the pending-record estimator — the
// local, deterministic partition of TrajStore rows by proof reachability
// (blocked / pending-unsynced / synced-awaiting) against the last-synced
// watermark, + the caveat-#15 service-credit renders (never a $ beside an
// accrual). Same dual gate as the payload module: it consumes the gate
// (kat_service) + TrajRowDoc/PayloadRow (latent_retrieval). Phase 1 T1.6's
// `sync` command composes `SyncWatermark::from_payload` + `estimate_pending`;
// nothing here touches the network or the clock (pure by construction).
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
pub mod kat_pending_estimator;

// (issue 088: `kat_lease_client` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_stake_client` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_vote_client` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_payreq_client` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_account_client` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_consent_client` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_burn_client` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_soft_gate` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_wetrun` moved to the riir-kat repo — see the
// re-export block above.)

// (issue 088: `kat_refill` moved to the riir-kat repo — see the
// re-export block above.)

// Proposal 005 Phase 2 T2.4 (Plan 042): the score-bench promotion gate
// for network-learned corpus candidates — the frozen bench runs BEFORE any
// candidate enters the local overlay (Certified on pass; Withdrawn is
// ABSORBING on post-hoc regression — the §Mining #4 poisoning defense),
// with trial runs strictly isolated from the score history. Behind BOTH
// kat_service (the candidate channel's surface) AND score_bench (the
// frozen bench + oracle); both are opt-in — the default build is
// untouched.
#[cfg(all(feature = "kat_service", feature = "score_bench"))]
pub mod kat_promotion;

// Plan 019 T4.3: the neuron-db trajectory store — the DURABLE backend for
// `--fix-compile` self-evolution (LocalKvStore encrypted WAL + BM25 + the
// 8-D sigmoid-dot latent axis). Behind `latent_retrieval` (the dep-light
// default keeps the memory.json envelope — one active backend per build,
// no dual-write drift).
#[cfg(feature = "latent_retrieval")]
pub mod traj_store;

// Issue 089 T1+T2: the learnability-frontier band report — per-rule
// r/f/n/p̂/LCB₅ + band-mass + saturation candidates, PURE arithmetic over
// outcome counters (store-free, ungated so lib tests cover it at default
// features; the `--frontier-report` CLI half in bin/heal.rs feeds it
// `learn_rows()` under `latent_retrieval`). Report-first, gate-later
// (R135/Bench 047 law: no admission gate wires until the report proves
// band-mass is predictive).
pub mod frontier_report;

// Plan 124: the config-hygiene audit — intake classes A + B as a report
// lane (`--config-audit` in bin/heal.rs). Pure syn analysis over the scanned
// file set, zero new deps, ungated so lib tests cover it at default
// features. Report-first, gates nothing (the frontier-report law).
pub mod config_audit;
pub mod orphan_report;
pub mod scope_hint;
pub mod downstream_hint;

// The KAT run footer (2026-09-13 redesign): the glanceable 3-line billing
// readout that replaced the two `summary_line()` walls in bin/heal.rs —
// reading-rounded numbers, a Braille fuel-gauge over the grant projection,
// a state-aware sync line. Pure presentation over ledger primitives, so
// lib tests (incl. the visual `-- --nocapture` print) run at default
// features; the canonical 6-decimal shapes stay in riir-kat's `render_kat`.
pub mod kat_footer;

// Plan 014: clippy-bench — the frozen real-code healing bench + SVG score
// history. Measures the PRODUCTION `--fix` path (bounded-fixpoint proposers)
// against real-`cargo clippy` ground truth on frozen fixtures mined from
// pre-sweep sibling commits; appends to `.benchmarks/score_history.jsonl`
// and renders `.benchmarks/score_history.svg`. Opt-in: needs the oracle
// (clippy_verify) and the cargo-heal fix-path surface (latent_retrieval).
#[cfg(feature = "score_bench")]
pub mod score_bench;

/// Plan 356 A1 — PAV (process-advantage) dataset curation for the L4
/// fixer: (prefix, MC-Q) pairs over the frozen score-bench spans, the
/// `.heal/` store, and extra miss dumps. The GPU half (daemon rollouts)
/// runs via `examples/pav_data_curate.rs`; the module + tests are CPU.
#[cfg(feature = "pav_data")]
pub mod pav_data;

/// Behavioural equivalence for candidate fixes (Plan 336 T5.1/T5.3's named
/// blocker). Always compiled: it is CPU-only, needs no model, and a promotion
/// gate must not be able to skip it via a feature flag.
pub mod behavioural_equivalence;

// ── Back-compat re-exports (preserve pre-refactor API names) ───────────────
// These type aliases + re-exports keep existing call sites compiling unchanged
// after the Plan 001 Phase 1 framework refactor.

/// Back-compat: `ClippyTernaryDrafter` = `TernaryDrafter` (Plan 001 rename).
pub type ClippyTernaryDrafter = drafter::TernaryDrafter;

/// Back-compat: `ClippyAdaptiveDrafter` = `AdaptiveDrafter` (Plan 001 rename).
pub type ClippyAdaptiveDrafter = adaptive_drafter::AdaptiveDrafter;

pub use adaptive_drafter::AdaptiveDrafter;
pub use corpus::Corpus;
pub use domain::{HealDomain, HealEntry};
/// Back-compat: re-export drafter types at the old paths.
pub use drafter::{DraftSource, DraftedFix, TernaryDrafter};
pub use embedder::{embed, embed_into, Direction, EMBED_DIM};
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
pub use domains::clippy_lints::verify::{ClippyVerifier, ClippyVerifyResult};

// ── rust_perf domain re-exports (behind `rust_perf` feature, default-on) ──
#[cfg(feature = "rust_perf")]
pub use domains::rust_perf::{PerfParseVerdict, PerfRule, RustPerfDomain};

/// Back-compat: `build_perf_seed_corpus` (behind `rust_perf`).
#[cfg(feature = "rust_perf")]
pub use domains::rust_perf::corpus::build_perf_seed_corpus;

/// `rust_perf` verifier (behind `rust_perf_verify`).
#[cfg(feature = "rust_perf_verify")]
pub use domains::rust_perf::verify::{PerfVerifier, PerfVerifyResult};

/// `rust_perf` bench validator (behind `rust_perf_bench`).
#[cfg(feature = "rust_perf_bench")]
pub use domains::rust_perf::bench::{
    BenchOverall, BenchSpec, BenchVerdict, NotBenchable, ParetoVerdict, PerfBenchValidator,
    SizeOutcome, SizeResult,
};

// ── kernel_opt domain re-exports (behind `kernel_heal` feature, opt-in) ────
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::{KernelCategory, KernelOptDomain, KernelRule, KernelVerdict};

/// Back-compat: `build_kernel_seed_corpus` (behind `kernel_heal`).
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::corpus::build_kernel_seed_corpus;

/// `kernel_opt` corpus category wiring (Plan 003) — category → `rule_ids` mapping + grouped corpus builder.
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::corpus::{build_corpus_categorized, KERNEL_CATEGORIES};

/// `kernel_opt` expert router (behind `kernel_heal`).
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::router::{
    build_activity, KernelExpertRouter, ACTIVITY_DIM, AXIS_DTYPE, AXIS_HW, AXIS_IS_ATTENTION,
    AXIS_IS_FUSION, AXIS_IS_IR_TRANSFORM, AXIS_K, AXIS_M, AXIS_N, KERNEL_EXPERT_NAMES,
    NUM_KERNEL_EXPERTS,
};

// ── sec domain re-exports (behind `sec` feature, opt-in) ──────────────────
#[cfg(feature = "sec")]
pub use domains::sec::{count_unsafe, SecDomain, SecFamily, SecRule, SecVerdict, UnsafeCounts};

/// Back-compat: `build_sec_seed_corpus` (behind `sec`).
#[cfg(feature = "sec")]
pub use domains::sec::corpus::build_sec_seed_corpus;

// ── docker domain re-exports (behind `docker` feature, default since 2026-08-28) ────────────
#[cfg(feature = "docker")]
pub use domains::docker::{
    refuses as docker_refusals, DockerDomain, DockerFamily, DockerRule, DockerVerdict,
    SyntaxVersion,
};

/// The docker seed corpus (behind `docker`).
#[cfg(feature = "docker")]
pub use domains::docker::corpus::build_docker_seed_corpus;

// ── dist domain re-exports (behind `dist` feature, default since 2026-09-13) ─────────────────
#[cfg(feature = "dist")]
pub use domains::dist::{
    refuses as dist_refusals, DistContext, DistDomain, DistFamily, DistRule, DistVerdict,
    ProfileKnobs,
};

/// The dist seed corpus (behind `dist`).
#[cfg(feature = "dist")]
pub use domains::dist::corpus::build_dist_seed_corpus;

/// `kernel_opt` thermal router (behind `kernel_heal`).
#[cfg(feature = "kernel_heal")]
pub use domains::kernel_opt::thermal::{KernelThermalRouter, ThermalTier};

/// `kernel_opt` verifier (behind `kernel_verify`).
#[cfg(feature = "kernel_verify")]
pub use domains::kernel_opt::verify::{KernelVerifier, KernelVerifyResult};

#[cfg(feature = "ruliology_search")]
pub use ruliology_search::{RuliologySearch, ScoredCandidate};

pub use evolve::{EvolveConfig, LatentSkillEvolution};
pub use memory::{
    CommitmentStatus, EvidenceTier, FixTrajectory, LatentFixMemory, LatentQuery, ScoredTrajectory,
};
pub use recorder::EvolveRecorder;
pub use selection_entropic::{solve_beta, tilt_advantages_into, tilted_weights, KL_BUDGET_LN2};
pub use selection_tether::{
    fit_rho, sse, TetherFit, TetherStats, DEFAULT_EMA_DECAY, DEFAULT_RHO, DEFAULT_WINDOW,
};
pub use self_evolve::{HealPipeline, HealResult, HealSource, SelectionMode, TuneReport};

// Bench 669 consumer adoption (Plan 016).
#[cfg(feature = "contrastive_scope")]
pub use scope_model::{ScopeModel, KAPPA, OUT_OF_SCOPE_DOCS, THETA};

// Proposal 005 Phase 0 T0.5/T0.6 (Plans 023 + 024): the KAT meter + charge surface.
#[cfg(feature = "kat_service")]
pub use kat_meter::{
    count_word_tokens, kat_for_tokens, ChargeState, KatCharge, KatMeter, MeterState,
    CHARGE_CHANGELOG, CHARGE_KAT_PER_MILLION_TOKENS, CHARGE_VERSION, FREE_TIER_GRANT_KAT,
    KAT_WARN_REMAINING, METER_CHANGELOG, METER_VERSION,
};

// Proposal 005 Phase 0 T0.1 (Plan 025): the KAT account surface.
#[cfg(feature = "kat_service")]
pub use kat_account::{
    account_id_for, short_id, AccountState, KatAccount, LoginOutcome, ACCOUNT_CHANGELOG,
    ACCOUNT_VERSION,
};

// Proposal 005 Phase 0 T0.2 (Plan 026): the redaction-gate surface.
#[cfg(feature = "kat_service")]
pub use kat_redact::{
    glob_match, redact, scan_secrets, BlockReason, Finding, FindingKind, GateDecision,
    LoadedPolicy, RedactionGate, RedactionLog, SpanOrigin, SyncPolicy, DEFAULT_MAX_SPAN_BYTES,
    POLICY_FILE_NAME, REDACT_CHANGELOG, REDACT_VERSION,
};

// Plan 005: latent retrieval surface (`--suggest` MVP).
#[cfg(feature = "latent_retrieval")]
pub use draft::{
    DomainId, DomainRouter, LatentMatcher, RuleIndex, RuleInfo, ScoredRule, SpanSuggestion,
    SuggestScratch,
};
