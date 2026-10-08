//! # riir-refine — Multi-Domain Modelless Code Healer
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
//! - **`kernel_opt`** (opt-in, Phase 3) — GPU kernel optimization healing (absorbs `Proposal 012`; 4090 `LoRA` deferred)
//! - **`sec`** (opt-in, Plan 018) — security-pattern bounded fixes
//! - **`docker`** (default-on since 2026-08-28, Plan 030) — Dockerfile healing, the first no-`syn` scanner
//! - **`rustc_errors`** (default-on since 2026-08-29, Plan 019) — `--fix-compile` compile-error repair (driver-shaped, inert until invoked)
//! - **`dist`** (default-on since 2026-09-13, Issue 085) — Cargo.toml build-config advice (suggest-only)
//! - **`shader_perf`** (opt-in, Plan 134 / Proposal 014) — WGSL render-pipeline bounded-fix healing (`@vertex`/`@fragment`; `@compute` stays `kernel_opt`)
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
// Issue 117: the compiled-feature stamp (`--version`) + the loud refusal a
// compiled-out gated command prints before exit 2. Ungated on purpose — a
// STALE binary is precisely one that has few features, so the module that
// diagnoses staleness must be in every build.
pub mod bench_guard;
pub mod build_stamp;
pub mod corpus;
pub mod dispersion;
pub mod domain;
pub mod drafter;
pub mod edit_emitter;
// Issue 134 T1 — the standing demonstrability column (measurement only,
// no behavior change; Research 585 / SAT arXiv:2609.22682). Pure over its
// inputs; the eval half of its API is cfg(latent_retrieval) and the bench
// half cfg(score_bench), mirroring the two labeled-outcome sources.
pub mod demonstrability;
// Issue 134 T2 — the pool-depth ALLOCATION gate (feature
// `demonstrability_gate`, default off). This module is the PURE policy +
// the standing loader (ungated so the policy tests run at default
// features); the matcher wiring that consumes it is
// cfg(demonstrability_gate) in `draft/latent_matcher.rs`.
pub mod demonstrability_gate;
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
// Issue 126 lane (b): the Lemma-4.2 likelihood-ratio tier guard (arXiv:2206.07568).
// UNGATED — pure math over `elo`'s expected score; its only consumer is the
// latent_retrieval-gated traj_store's certify transition, but the vocabulary
// (π(τ|s_g)/π(τ|s_j) exclusion + the priced 2γε/(1−γ) slack) is crate-level
// evidence law, and the pure tests run at default features.
pub mod tier_guard;

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
// Issue 102 T2/T3: the corpus active-set marker (demotion/re-admit/cap).
// Ungated, default-empty — nothing demotes until the fade-predictive gate
// opens; the fan-out and fix lanes only ever READ this file.
pub mod active_set;
// Plan 202 R1: the escalation manifest — the operator-owned ARMING surface
// for per-domain escalation, consuming the SHARED katgpt-core
// escalation_guard primitive (Issue 923). Manifest-only arming, demote-only
// switches, ALL-EMPTY at ship. Opt-in (`escalation_manifest`).
#[cfg(feature = "escalation_manifest")]
pub mod escalation_manifest;
// Plan 202 R6: the escalation DISPATCH — armed manifest rows joined to their
// shipped transports (the min-pool gate, the deadline-capped readiness
// retry, the sticky cost-ceiling latch). Gated as the manifest AND any of
// its two transports (the code_egress ANY-of-consumers shape): a dispatch
// without a transport is nothing to compile.
#[cfg(all(
    feature = "escalation_manifest",
    any(feature = "seat_rerank", feature = "hosted_expert")
))]
pub mod escalation_dispatch;
// Plan 202 R4: the vessel verify path — ONE artifact protocol across the
// decision stack (A1/A8), via the public format repo's crate. Verify-only
// seam: refine never mints (A8), never embeds vessel bytes (A10). Opt-in
// (`vessel_artifacts`); R2's HostedExpertL4 + the escalation lane's
// artifact pins are the designed consumers.
#[cfg(feature = "vessel_artifacts")]
pub mod vessel_artifacts;
// Issue 130 T1+T3: the admission-time generalization gate's REPORT half —
// per-stratum δ calibration + the corpus-stall predicate. Report-only by
// design (T0): nothing here gates admission until the strata support a δ.
pub mod gate_calibration;
// Issue 136 (reef P1+P3+P5, Research 213): the structural self-mutation
// admission vocabulary — per-kind shape gates, kind-keyed review classes,
// and the two-head commit-chain state machine. UNGATED pure: nothing here
// mutates anything, and no auto-serve kind is activated (owner call
// reserved). The only filesystem surface is the store below, feature-gated.
pub mod structural_admission;
#[cfg(feature = "structural_admission")]
pub mod structural_admission_store;
pub mod gp_acquisition;
pub mod memory;
pub mod recorder;
// Issue 105: the ungated journey-credit factor (PC-ALM consumption half —
// the driver accumulator lives in the rustc_errors domain; both selection
// layers consume this).
pub mod journey;
pub mod selection_entropic;
pub mod selection_tether;
pub mod self_evolve;

// Plan 005: real-file span-level latent retrieval (`--suggest` MVP).
// Composes riir-rag (AstChunker + ModellessEmbedder + NeuronDbRetriever) +
// the katgpt-core routing primitive. Opt-in — pulls the riir-rag +
// riir-neuron-db path deps (heavy; default build stays light).
#[cfg(feature = "latent_retrieval")]
pub mod draft;

// Issue 113: the auto-oracle — a scanning run over a cargo project lints it
// with REAL clippy itself. std-only (Path + Command), so it is UNGATED: the
// `cargo-refine` bin calls it from the scan path without a cfg, and its
// required-features do NOT include `compile_oracle`.
//
// ⛔ It was gated on `compile_oracle` from Issue 113's landing (3f0eecb9)
// until Issue 117: the `pub mod clippy_oracle;` line was inserted BETWEEN
// the attribute below and the item that attribute was written for, so the
// gate landed on the wrong module and `compile_oracle` — the one the comment
// describes — lost its gate entirely. Silent under default features (both
// features are on), and a `--no-default-features` build of the bin's own
// required-feature set would not compile.
pub mod clippy_oracle;

// Issue 146 T0.1 / Proposal 020: the hotpath report subset — parse + SHA
// keying + hot-span derivation for the profile-guided-healing oracle lane
// (the `--clippy-json` anchor-lane pattern over runtime profiles). Ungated:
// pure data plumbing, lib-tested at default features; zero new deps.
pub mod hotpath_report;

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
// the cargo-refine bin surface).
#[cfg(feature = "fix_verify")]
pub mod fix_verify;

// Issue 153: the CLI version alert + the verified binary self-update. The
// alert is the module's whole job when the feature is off — compiled out.
#[cfg(feature = "update_check")]
pub mod update_check;

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
// substrates it composes. Issue 124: the wire half of the client (URL
// shape, body builder, response parse, failure classification) rides the
// `riir-llm` leaf. See `.issues/040_*` + Bench 049.
#[cfg(feature = "cf_workers_ai")]
pub mod cf_workers_ai;

// Issue 046 — the USD LLM daily-cap ledger (reserve-then-reconcile): the
// circuit breaker every LLM call through `CfClient::generate` gates through
// BEFORE any unattended loop runs (the Issue 040 169-request burn class).
// Plan 202 R2: the `hosted_expert` transport joins it as a consumer — the
// gate is the ANY of the two features (the ledger itself is feature-free).
#[cfg(any(feature = "cf_workers_ai", feature = "hosted_expert"))]
pub mod llm_spend;

// Plan 202 R2 — HostedExpertL4: the hosted-expert GENERATE transport under
// the L4Fallback seam (the no-candidate miss). OpenAI-compatible wire via
// the riir-llm leaf (the groq_first shape: no direct ureq); llm_spend as
// the cost guard; code-egress consent as its own scope (Unset refuses,
// pre-socket); hard deadline declines; snippet-only payload with the
// no-retention rule on the wire. Endpoint comes from the R1 manifest row
// — never in code. Opt-in (arming is the manifest's, at the R6 wiring).
#[cfg(feature = "hosted_expert")]
pub mod hosted_expert;

// Plan 202 R3a — the code-egress consent scope moved here from
// `hosted_expert` (R2): the seat-rerank client joins as the second
// consumer of the SAME scope (one env, one spelling, one taxonomy —
// the `llm_spend` ANY-of-consumers gate shape). Pure std.
#[cfg(any(feature = "hosted_expert", feature = "seat_rerank"))]
pub mod code_egress;

// Plan 202 R3a — SeatRerankClient: the RERANK half of the escalation
// chain (the candidates-exist arm). The L0–L3 candidates become the
// seat's `options`; the hosted instinct `Ext` seat picks or abstains;
// abstain => the modelless answer stands. Std-only HTTP/1.1 over the
// serve edge's `/decide` envelope (the riir-agents decision_client
// shape — a decision-plane edge, not an LLM leg; no riir-llm, no ureq).
// The echo-is-the-trust-anchor contract + L4 invariant 3 (a pick can
// only ever name a candidate we sent) + the shared code-egress consent
// + a required deadline. Opt-in (arming is the manifest's, at the R6
// wiring). The instinct-side gate: riir-instinct
// tests/ext_seat_rerank_gates.rs (the seat half of the contract).
#[cfg(feature = "seat_rerank")]
pub mod seat_rerank;

// Issue 051 — groq-first free-tier LLM routing + the groq→infron→CF
// provider ladder (the `router` submodule, additionally gated on
// `cf_workers_ai` — it composes CfClient as the paid fallback leg).
// Issue 124: the wire mechanics (OpenAI-compatible URL/body/parse, the
// qwen3 `/no_think` mitigation, failure classification, both timeout
// ceilings, bearer hygiene) are the `riir-llm` leaf — this module keeps
// env resolution + the WINDOWED ladder policy (rate-limited legs disable
// for the whole window; the leaf's ladder is stateless per attempt) +
// the providers ledger contract. GROQ_API_KEY absent → the ladder starts
// at the next free leg (infron, Issue 106) or pure-CF when no free key
// is set — byte-identical legacy paths. See `.issues/051_groq_first_
// free_tier.md` (removed; git history) + `.issues/106_infron_free_leg.md`
// + `.issues/124_riir_llm_leaf_repoint.md`.
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
// (`riir_refine::kat_meter`, `crate::kat_redact`, `bin/heal.rs`'s ~40 refs,
// the integration tests) bit-identically. Gate-match = the historical root
// availability, mapped onto the member's tier features (root kat_service →
// member client tier; root kat_transport → member kat_transport tier;
// refill/consent need BOTH). Keepers below (sync_payload, pending
// estimator, promotion) stay healer-side: they import traj_store/score_bench.
// Issue 088 re-export plane (see the block comment above). `kat_redact`
// moved OUT of the kat_service-gated list (Issue 151 T1, verdict R1): the
// secret scanner is std+blake3-only compute on riir-kat's own `redact` tier
// (which this dep now enables at the BASE — never the client tier), so the
// secret-blind surface is available at DEFAULT features and the
// structural-admission reef scan consumes the ONE canonical table ungated.
// Public path unchanged (`riir_refine::kat_redact`).
pub use riir_kat::kat_redact;
#[cfg(feature = "kat_service")]
pub use riir_kat::{
    kat_account, kat_account_client, kat_adoption, kat_burn_client, kat_fixstat_client,
    kat_lease_client, kat_meter, kat_pay_client, kat_payreq_client, kat_protocol_account,
    kat_protocol_burn, kat_protocol_fixstat, kat_protocol_lease, kat_protocol_pay,
    kat_protocol_payreq, kat_protocol_replay, kat_protocol_stake, kat_protocol_v2,
    kat_protocol_vote, kat_protocol_wetrun, kat_replay_client, kat_service_config,
    kat_stake_client, kat_vote_client, kat_wetrun,
};
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
pub use riir_kat::{kat_consent_client, kat_protocol_consent, kat_refill};
// The unit vocabulary (MicroKat/WholeKat + the µ constants) is feature-free
// by construction (kat_units' own module doc) — and the Plan 035 T3
// newtype adoption makes the UNGATED footer a consumer, so the root
// re-export is ungated too (the machine-local trio's posture).
pub use riir_kat::kat_units;
// The ungated machine-local trio + the free-tier protocol plumbing: the
// member keeps them behind `client`, but riir-refine's contract is
// UNGATED (--unmine must work in every feature build; the consent-module
// rule) — the member compiles them for us at default features, and root
// consumers reach them through these ungated re-exports.
pub use riir_kat::{kat_mining_consent, kat_service_urls, kat_soft_gate};
// The state-stamp module (riir-kat Issue 003 T1/T2): feature-free protocol
// tier — the per-net dir seam + the stamp judge are consumed by the ungated
// consent plane (--unmine's outbox dir) and the gated money plane alike.
pub use riir_kat::kat_state_stamp;

// riir-dapps Proposal 004 Half 1: the work-stats sharing consent — the
// THIRD consent plane beside mining, deny-by-default (Unset refuses; only
// OptedIn pushes — the deliberate inverse of kat_mining_consent's Unset).
// Ungated in-crate (the consent-module rule: writable in every feature
// build); client-LOCAL state, so not in the riir-kat leaf — the record
// resolution delegates to kat_mining_consent::default_dir so the account
// dir cannot drift between planes. Fix-path push half lands with the
// Proposal 004 Half-2 server route.
pub mod kat_stats_consent;

// riir-dapps Plan 045 Phase A (Proposal 008 §2): the first-join environment
// choice — `--mine` / `login` with NO service config offers devnet
// (default) / mainnet once, recorded through the EXISTING `config set`
// record (`service_url.json`). Ungated in-crate (the consent-module rule:
// `--mine` is the join command in every feature build); `testnet` is never
// offered (host torn down 2026-10-03 — the `config set` alias stays valid
// for wire compat); `DEFAULT_SERVICE_URL` never moves (the QR-goldens law).
pub mod kat_join;

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

// Proposal 015 Phase 2 (Plan 127): the replay-endorsement ENGINE — the
// toolchain fingerprint, the pinned single-item rate rule (the determinism
// law two agreeing verifiers depend on), and the step-0 canary hash check
// over one leased item. Gated with the ORACLE's own gates (clippy_verify
// for the instrument + latent_retrieval for the `draft` tree it lives in);
// the wire types come from the riir-kat protocol tier (always compiled —
// the root dep has no default features). The command orchestration
// (`cargo refine replay`) lives in the bin and adds the kat_service +
// kat_transport gates.
#[cfg(all(feature = "clippy_verify", feature = "latent_retrieval"))]
pub mod kat_replay;

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

// Issue 156 — the state-level rescue-trajectory mining filter (FlyBy
// Algorithm 1, Research 609 §2.6): the b0=0 ∧ bd≥3 keep predicate, the
// shallowest-sufficient-depth vocabulary, the keep-set BLAKE3 hash, and the
// `--misses` rescue-band section. UNGATED pure arithmetic over counters
// (the frontier_report precedent — lib tests cover it at default features);
// the durable tallies live on the traj_store row (gated) and the arm-H lane
// (`fixer_regate --arm H`) is the miner. AUGMENT-ONLY: the agreement VETO
// stays the acceptance authority — this can only narrow.
pub mod rescue_mine;

// Issue 134 T1 — the compiled rule→domain table, moved UNGATED from
// kat_sync_payload (which re-exports it at its historical path): the
// demonstrability column + the score-bench lane project through the SAME
// table without the kat_service dep tree. Pure in-memory read; lazily
// built, zero cost when unused.
pub mod rule_domains;

// Issue 122 PoC: the PowerPlay frontier miner — modelless, deterministic,
// zero-LLM generator that constructs the simplest fixtures the current
// fix-rule corpus cannot repair, verifies them with the real clippy oracle
// only, and commits admissible rows (unsolved-before / solved-after /
// prior-rows-green / exactly-once) with band evidence for the Issue-089
// gate. Opt-in (PoC pending its GOAT); implies `score_bench` (the heal
// fixpoint + ComponentOverride substrate) + `clippy_lints` (the stock
// regression rows). Never touches `build_seed_corpus()` (the freeze-table
// law). The real-clippy e2e: `examples/frontier_miner.rs`.
#[cfg(feature = "frontier_miner")]
pub mod frontier_miner;

// Issue 107 T1: the fixseq horizon report — lead-k linear decodability of
// attempt outcomes over the `.refine/fixseq/` ring (AUC(k) vs the base and
// history-aggregate floors, LORO-held-out ridge readout). Pure stats over
// the ring's jsonl wire shape, ungated so lib tests cover it at default
// features. Report-first, gates nothing (the frontier-report law); refuses
// green zero on empty/blind/single-class populations.
pub mod horizon_report;

// Issue 128 (T1-T4): the fixseq cascade-law + derailment-index report — the
// Traverse/Scout long-horizon failure statistics (arXiv:2609.17930, Research
// 200) over the SAME ring jsonl horizon_report loads. Counting over labeled
// steps, no judge, no training; Wilson CIs; the G4 KS-vs-point-mass guard.
// Pure stats, ungated (the horizon_report law); report-first, gates nothing,
// wires nothing (G3); refuses green zero on empty / no-transition
// populations.
pub mod fixseq_stats;

// Issue 107 T2: the span-shape embedding — the producer behind the fixseq
// ring's embedding column. Hashed unigram+bigram bag over the span's
// ORIGINAL trigger text, signed, L2-normalized, i8-quantized. Write-time
// capture (the latent-encode-at-capture rule: the site's bytes drift as
// fixes land — offline re-embed would read the WRONG bytes). Ungated,
// zero new deps (blake3 is unconditional); consumed by the driver
// (rustc_errors, write) and mirrored by horizon_report's RingLine (read).
pub mod span_embed;

// Issue 140 T1+T2 (RLTL;DR, arXiv:2609.37633 — .research/228): the modelless
// insight distiller (deterministic template over the structured verifier
// diagnostic — the paper's generator-identity ablation licenses this: verifier
// access is the signal, not who writes the sentence) + the abstraction-level
// admission predicate (L1: one-sentence GENERIC insights transfer cross-task;
// episode-bound literals are label noise). Pure text functions, std-only,
// ungated so lib tests cover it at default features. T3 (CompileDriver wiring
// + the ordering control arm) consumes this; the riir-train tuple miner
// (Plan 432 T0.4) re-implements the ~50-line predicate locally — boundary-
// forced copy, cross-referenced here.
pub mod insight;

// Plan 124: the config-hygiene audit — intake classes A + B as a report
// lane (`--config-audit` in bin/heal.rs). Pure syn analysis over the scanned
// file set, zero new deps, ungated so lib tests cover it at default
// features. Report-first, gates nothing (the frontier-report law).
pub mod config_audit;
pub mod orphan_report;
// Intake class P22: the platform-dead_code decl-vs-use cfg lattice —
// items whose every use sits inside a platform gate the declaration
// lacks (the NEON_U8 class no single-platform CI matrix can see).
pub mod platform_audit;

// Issue 119: the corpus coverage-debt sweep — clippy's LIVE lint surface
// (derived from the installed driver, never hand-typed) diffed against the
// clippy_lints corpus + the compiled fix set. Report-first, gates nothing
// (the frontier-report law); refuses on a blind derivation or an
// uncompiled corpus rather than reporting a flattering zero. Pure over its
// inputs — the driver spawn + manifest read live in the `--coverage-report`
// CLI half — so lib tests cover it at default features.
pub mod corpus_coverage;

// Plan 134 T2.1: the frame-time budget tier — the measurement VERDICT half
// of the shader_perf domain, consuming riir-shader's capture-side
// `<capture>.frames.json` sidecars (schema riir_shader_frame_time_v1):
// per-effect p50/p90 vs the carried budget + the pre/post A/B diff with
// warmup-parity and pairing-uniqueness evidence guards. Ungated (the
// corpus_coverage/horizon_report law): pure over its inputs, no domain
// machinery, lib-tested at default features, the CLI arm works on every
// feature build. Report-first, gates nothing; refuses exit 1 on a blind
// scan — never a flattering partial (the frontier-report law).
pub mod frame_time_report;

// Issue 116: the corpus-truth delta surface — lints the compiled FIX lane
// can write that the LEASED suggest corpus cannot rank yet. Names the seam
// (`[no corpus rule]` means "not leased yet" here, "unsupported" there),
// never closes it: the split is load-bearing. Pure over its inputs, silent
// on an empty delta.
pub mod corpus_delta;
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
// (clippy_verify) and the cargo-refine fix-path surface (latent_retrieval).
#[cfg(feature = "score_bench")]
pub mod score_bench;

/// Plan 356 A1 — PAV (process-advantage) dataset curation for the L4
/// fixer: (prefix, MC-Q) pairs over the frozen score-bench spans, the
/// `.refine/` store, and extra miss dumps. The GPU half (daemon rollouts)
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

// ── kernel_opt domain re-exports (behind `kernel_opt` feature, opt-in) ────
#[cfg(feature = "kernel_opt")]
pub use domains::kernel_opt::{KernelCategory, KernelOptDomain, KernelRule, KernelVerdict};

/// Back-compat: `build_kernel_seed_corpus` (behind `kernel_opt`).
#[cfg(feature = "kernel_opt")]
pub use domains::kernel_opt::corpus::build_kernel_seed_corpus;

/// `kernel_opt` corpus category wiring (Plan 003) — category → `rule_ids` mapping + grouped corpus builder.
#[cfg(feature = "kernel_opt")]
pub use domains::kernel_opt::corpus::{build_corpus_categorized, KERNEL_CATEGORIES};

/// `kernel_opt` expert router (behind `kernel_opt`).
#[cfg(feature = "kernel_opt")]
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

// ── shader_perf domain re-exports (behind `shader_perf` feature, opt-in — Plan 134) ────────
#[cfg(feature = "shader_perf")]
pub use domains::shader_perf::{
    refuses as shader_refusals, entry_stages, ShaderFamily, ShaderPerfDomain, ShaderRule,
    ShaderVerdict, StageSet,
};

/// The `shader_perf` seed corpus (behind `shader_perf`).
#[cfg(feature = "shader_perf")]
pub use domains::shader_perf::corpus::build_shader_seed_corpus;

/// `kernel_opt` thermal router (behind `kernel_opt`).
#[cfg(feature = "kernel_opt")]
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
    CHARGE_CHANGELOG, CHARGE_MICRO_KAT_PER_MILLION_TOKENS, CHARGE_VERSION,
    FREE_TIER_GRANT_MICRO_KAT, KAT_WARN_REMAINING, METER_CHANGELOG, METER_VERSION,
};

// Issue 151: the secret-blind surface — scan, warn (suggest-only), blind.
// UNGATED: consumes the canonical scanner (also ungated at the base dep
// since T1) and is local-only compute — the config-audit precedent. The
// shareable-output rule lives in the module doc (no fingerprint, no
// finding bytes on stdout).
pub mod secret_blind;

// Proposal 005 Phase 0 T0.1 (Plan 025): the KAT account surface.
#[cfg(feature = "kat_service")]
pub use kat_account::{
    account_id_for, short_id, AccountState, KatAccount, LoginOutcome, ACCOUNT_CHANGELOG,
    ACCOUNT_VERSION,
};

// Proposal 005 Phase 0 T0.2 (Plan 026): the redaction-gate surface.
// UNGATED since Issue 151 T1 (verdict R1): the scanner is std+blake3-only
// compute on riir-kat's `redact` tier (enabled at the BASE dep — never
// `client`, which would drag the account/crypto plane into every build),
// so the secret-blind report lane and the reef P2 persistence scan consume
// the ONE canonical table at default features. The path set is the
// historical kat_service surface plus the ungated availability.
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

// Plan 199 Phase 3: the WASM validator lane — bounded fix-validators as
// committed wasm32 modules executed in-process at µs class. Opt-in: the
// percepta wasm decoder/lowering dep rides `wasm_validator` only (default
// build stays dependency-light). Admission gate + fuel-bounded interpreter;
// the artifact build lane is the `wasm-validator-build` bin.
#[cfg(feature = "wasm_validator")]
pub mod validators;
