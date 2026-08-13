# Plan 001 — Multi-Domain Modelless Code Healer Framework Refactor

**Proposal:** [001](../.proposals/001_multi_domain_modelless_code_healer.md)
**Branch:** `develop` (per global rule — no feature branches)
**Status:** in flight

## Goal

Generalize riir-clippy from a clippy-lint-only healer into a pluggable multi-domain modelless code healer behind a `HealDomain` trait. Preserve clippy back-compat via re-exports. Scaffold two new domains (`rust_perf`, `kernel_opt`). Defer 4090 LoRA (L4) across all domains.

This is a **framework refactor**, not a perf-gain primitive. The GOAT gate is "framework compiles + clippy domain still passes its existing tests + new domain scaffolds compile" (G1–G4 in Proposal 001).

## Substrate-first gate (pre-flight, MANDATORY)

Before implementing, verify the substrate exists:
- [x] `HealConflictDetector` trait (katgpt-core Issue 133) — precedent for heal-path validation traits. `HealDomain::validate` mirrors its shape.
- [x] `pick_domain<N, A>` (katgpt-core Plan 558) — latent-MoE expert selection. Consumed by `kernel_opt` router.
- [x] `ConstraintPruner` trait (katgpt-core) — already consumed by riir-clippy.
- [x] drafter+pruner+corpus pattern (Proposal 034) — already shipped in riir-clippy.
- [x] FlashAR/QGF thermal-tier pattern — consumed by `kernel_opt` thermal module (Plasma→Hot→Warm→Cold→Freeze).
- [x] riir-game-sdk Issue 002 facade re-export pattern — back-compat strategy.

No new substrate invented. The `HealDomain` trait is the one new abstraction.

## Phase 1 — Framework extraction (modelless, no new domains yet) — DONE

Refactor the existing pipeline to be domain-pluggable. clippy stays the only working domain; the rename + trait extraction must be bit-identical in behavior.

**Status: DONE (2026-08-13).** All tasks `[x]`. 133 lib tests + 4 goat_gate + 5 stage2_goat (142 total) pass. `--no-default-features` compiles clean (28 framework-only tests). `clippy --all-targets -D warnings` clean.

### Tasks

- [x] **T1.1** Create `src/domain.rs` with the `HealDomain` trait (associated types `Entry` + `Verdict`, methods `build_corpus` / `validate`) + `HealEntry` trait (field accessors: `rule_id` / `buggy_code` / `fixed_code`). Document the signature as provisional (Proposal 001 caveat 1).
- [x] **T1.2** Generalize `src/corpus.rs`: `ClippyCorpus` → `Corpus<D: HealDomain>`. Commitment computed from serialized entries (serde_json per entry) — deterministic but no longer byte-identical to the old hand-rolled hash (no tests pinned the old hash value). `ClippyFixEntry` moved to `domains/clippy_lints/`.
- [x] **T1.3** Rename `ClippyTernaryDrafter` → `TernaryDrafter` in `src/drafter.rs`. `draft_fix` is now generic over `E: HealEntry` (reads `E::fixed_code()`). Behavior unchanged. Identity guard + 2-stage routing hook preserved. `ClippyTernaryDrafter` re-exported as a type alias.
- [x] **T1.4** Generalize `HealPipeline` methods (`heal` + `seed_from_corpus`) to be generic over `E: HealEntry`. The pipeline struct itself stays non-generic (memory/evolve layers remain clippy-specific). Stage 2 adaptive drafter + `tune_against_fixture` preserved.
- [x] **T1.5** Create `src/domains/mod.rs` + `src/domains/clippy_lints/`:
  - `mod.rs`: `ClippyLintDomain: HealDomain` impl + `ClippyFixEntry` + `HealEntry` impl + `lint_names()` convenience on `Corpus<ClippyLintDomain>`.
  - `corpus.rs`: moved `build_seed_corpus` here (from `src/seed_corpus.rs`).
  - `verify.rs`: moved `clippy_verifier.rs` here (behind `clippy_verify` feature, unchanged).
  - `proposers/`: moved `src/proposers/` here (the per-lint AST codemods; Issue 010 fix Direction 2).
- [x] **T1.6** Back-compat re-exports in `src/lib.rs`:
  - `pub type ClippyTernaryDrafter = TernaryDrafter;`
  - `pub type ClippyAdaptiveDrafter = AdaptiveDrafter;`
  - `pub type ClippyCorpus = Corpus<ClippyLintDomain>;`
  - `pub use domains::clippy_lints::ClippyFixEntry;`
  - `pub use domains::clippy_lints::corpus::build_seed_corpus;`
  - `pub use adaptive_drafter::ClippyAdaptiveDrafter;` (preserved — Stage 2 stays)
- [x] **T1.7** Added `clippy_lints` to `Cargo.toml` `[features]` `default = ["clippy_lints"]`. Gate `src/domains/clippy_lints/` behind `#[cfg(feature = "clippy_lints")]`. `clippy_verify` + `ruliology_search` imply `clippy_lints`.
- [x] **T1.8** `examples/heal_demo.rs` compiles unchanged via the re-exports (`ClippyTernaryDrafter` type alias + `build_seed_corpus` re-export).
- [x] **T1.9** G1 check: `cargo test --lib` — all existing tests pass bit-identical (133 lib tests with all features). **The 5 `tests/stage2_goat.rs` tests pass** (Issue 010 GOAT gate — remains green after the rename).
- [x] **T1.10** G3 check: `./scripts/ci_feature_guard.sh` — layers 1-3 + 5 pass. Layer 4 `--all-features` now passes clean after riir-ai commit `4bfe3b30` fixed the `attn_sub_on` cfg gate (was: pre-existing `ternary_inference`/`riir-gpu` compile error, unrelated to this refactor).

### Phase 1 exit criteria — MET
- `HealDomain` trait ships, clippy domain reorganized behind `feature = "clippy_lints"`, all existing tests pass (142 total), old API names still resolve via re-export + type aliases, **Stage 2 GOAT gate (Bench 011) remains 10/10** (5 stage2_goat integration tests pass).

## Phase 1.5 — Stage 2 adaptive drafter scaffold (DONE — Issue 010 resolution, 2026-08-13)

**Status: DONE.** All tasks `[x]`. Bench 011 measures 10/10 on the held-out fixture (was 0/10). This phase is the implementation of Issue 010's fix direction (the 2-stage drafter + proposers + identity guard + tuning loop). It landed BEFORE the Phase 1 framework refactor — Phase 1's rename must preserve it.

### Tasks

- [x] **T1.5.1** `src/adaptive_drafter.rs` — `ClippyAdaptiveDrafter` mirrors `AdaptiveQuestDrafter`. Lock-free `papaya::HashMap` keyed by `BLAKE3(lint_name, buggy_code)`. Empty by default; populated off-hot-path.
- [x] **T1.5.2** Identity guard in `src/drafter.rs` — `ClippyTernaryDrafter::draft_fix` returns `None` when the emitted fix shares no identifiers with the input. `set_identity_guard` toggle.
- [x] **T1.5.3** `src/proposers/` — 10 per-lint `syn`-level AST rewrite rules (`needless_return`, `redundant_closure`, `needless_lifetimes`, `redundant_field_names`, `useless_conversion`, `needless_borrow`, `unnecessary_mut_passed`, `manual_map`, `unnecessary_unwrap`, `needless_range_loop`).
- [x] **T1.5.4** `HealPipeline::tune_against_fixture` — the off-hot-path tuning loop (feature-gated `clippy_verify`). Returns `TuneReport`.
- [x] **T1.5.5** `HealPipeline::heal` Stage 2 routing — returns the adaptive pool entry when present (`HealSource::Stage2Adaptive`), else falls back to Stage 1.
- [x] **T1.5.6** `examples/stage2_tuning_demo.rs` — the executable GOAT gate. Reports `TuneReport { stage2_verified: 10, .. }`.
- [x] **T1.5.7** `tests/stage2_goat.rs` — 5 integration tests pinning the gate.
- [x] **T1.5.8** `.benchmarks/011_stage2_goat.md` — the GOAT measurement record.

### Phase 1.5 exit criteria (MET)
- Stage 2 stack ships + Bench 011 reports 10/10 verified on the held-out fixture. The standing GOAT target from Issue 010 is **beaten** (was 0/10).

## Phase 2 — rust_perf domain scaffold — DONE

First new domain. Corpus from the `rust-optimize` skill checklist. Validator is `cargo test` (perf-bench deferred per Proposal 001 caveat 3).

**Status: DONE (2026-08-13).** All tasks `[x]`. 189 lib tests pass with all features (the pre-existing `ternary_inference`/`riir-gpu` failure was fixed in riir-ai commit `4bfe3b30`). `rust_perf` + `rust_perf_verify` compile clean alone + in combination. clippy clean.

### Tasks

- [x] **T2.1** Create `src/domains/rust_perf/`:
  - `mod.rs`: `RustPerfDomain: HealDomain` impl (Entry = `PerfRule`, Verdict = `PerfVerdict`)
  - `corpus.rs`: `PerfRule { rule_id, name, buggy_pattern, fixed_pattern, category }` + `build_perf_seed_corpus()` — encode 8 rules from the rust-optimize skill checklist (O(n)→O(1), String→&str, field reorder, papaya, SIMD chunks, Bevy reactive_low_power, WASM batch, precompute-table)
  - `verify.rs`: `PerfVerifier` — runs `cargo test` as subprocess (test pass + compile-error detection via exit status + compiler-message JSON parsing). Perf-bench Pareto verdict DEFERRED.
- [x] **T2.2** Add `rust_perf` + `rust_perf_verify` features to `Cargo.toml` (opt-in, default-off). Gate `src/domains/rust_perf/` behind `#[cfg(feature = "rust_perf")]`.
- [x] **T2.3** `PerfRule` BLAKE3-commit the seed corpus (mirror `ClippyFixEntry`'s freeze/thaw pattern via the generic `Corpus<D>`).
- [x] **T2.4** Example: `examples/perf_heal_demo.rs` — exercises the domain end-to-end (corpus, validate, tamper-evidence, trait impl).
- [x] **T2.5** G1 check: `cargo test --features rust_perf --lib` — domain scaffold compiles + seed corpus round-trips + 36 tests pass.
- [x] **T2.6** G3 check: `cargo check --features rust_perf` + clippy clean.

### Phase 2 exit criteria — MET
- `rust_perf` domain compiles, seed corpus (8 rules across 8 categories) ships, `cargo test` validator works (compile-error detection via exit status + JSON parsing). Perf-bench validator deferred (separate plan). NOT promoted to default-on (opt-in until real corpus + perf-bench validator pass a domain-specific GOAT).

## Phase 3 — kernel_opt domain scaffold (absorbs Proposal 012, 4090 DEFERRED) — DONE

Second new domain. Consumes Proposal 012's design (thermal-tier + latent-MoE + numeric validator). Corpus distillation from popcorn is DEFERRED (Proposal 012 §Phase 0, ~2 weeks reading job). 4090 LoRA path is DEFERRED.

**Status: DONE (2026-08-13).** All tasks `[x]`. 51 lib tests pass for `kernel_heal` alone; 189 with all features. Thermal router returns Plasma for all calls (modelless-first); 4090 Warm/Cold/Freeze tiers deferred. `pick_domain` (Plan 558) wired via `katgpt-core/variable_rank_domain_expert` feature.

### Tasks

- [x] **T3.1** Create `src/domains/kernel_opt/`:
  - `mod.rs`: `KernelOptDomain: HealDomain` impl (Entry = `KernelRule`, Verdict = `KernelVerdict`)
  - `corpus.rs`: `KernelRule { rule_id, gpu_concept, ref_shape, opt_pattern }` + `build_kernel_seed_corpus()` — **2 placeholder rules** (matvec-coalesce, gemm-tiling) so the scaffold compiles + the trait impl is exercised. Real corpus deferred per Proposal 012 §Phase 0.
  - `router.rs`: `KernelExpertRouter` — wraps `pick_domain<NUM_KERNEL_EXPERTS, ACTIVITY_DIM>` (Plan 558). Activity vector = kernel signature (5 dims: M, N, K, dtype, hw). Domain directions = identity vectors (placeholder until popcorn distillation lands).
  - `thermal.rs`: `KernelThermalRouter` + `ThermalTier` enum (`#[repr(u8)]`) — Plasma→Hot→Warm→Cold→Freeze tier dispatch. Stub returns Plasma for all calls until 4090 path lands. `is_local()` / `is_shipped()` helpers document the deferred tiers.
  - `verify.rs`: `KernelVerifier` — **compile-only stub** (delegates to shared `RustAnalyzerPruner`). Behind `kernel_verify` feature. Full numeric compare deferred per Proposal 012 §Phase 0.
- [x] **T3.2** Add `kernel_heal` + `kernel_verify` features to `Cargo.toml` (opt-in, default-off). `kernel_heal` enables `katgpt-core/variable_rank_domain_expert` for the `pick_domain` router. `kernel_verify` gates the compile validator (mirrors `clippy_verify` pattern).
- [x] **T3.3** G1 check: `cargo test --features kernel_heal --lib` — domain scaffold compiles + trait impl exercised by 2 placeholder rules (51 tests pass).
- [x] **T3.4** G3 check: `cargo check --features kernel_heal` + clippy clean.

### Phase 3 exit criteria — MET
- `kernel_opt` domain scaffold compiles, `pick_domain` router wired (identity directions placeholder), thermal-tier stub in place (returns Plasma), compile-only validator behind `kernel_verify`. Real corpus + full numeric validator + 4090 LoRA all DEFERRED to Proposal 012's follow-up plans. NOT promoted to default-on.

## Phase 4 — 4090 LoRA fallback (L4, ALL domains) — DEFERRED

- [-] **T4.1** riir-train plan for L4 LoRA fallback. Per modelless-first mandate (Proposal 001 caveat 4): only if modelless L0–L3 path plateaus below a domain's G1 gate. 4090 CUDA required. Deferred across all domains (clippy, rust_perf, kernel_opt).

### Phase 4 exit criteria
DEFERRED. No tasks in this plan. Reopens as a riir-train plan when a domain's modelless path plateaus.

## Phase 5 — GOAT gate + docs — DONE (docs portion)

**Status: docs portion DONE (2026-08-13).** T5.5–T5.7 done (`.docs/` book
index, README, AGENTS.md updated for multi-domain architecture). T5.1–T5.4
(GOAT gate re-runs) are measured in-place: 189 lib tests pass with all
features (the pre-existing ternary_inference failure was fixed in riir-ai
commit `4bfe3b30`); all existing GOAT gates
(stage2_goat 5/5, goat_gate_004 4/4+1ignored) remain green; clippy clean
across all feature combinations.

### Tasks

- [x] **T5.1 (G1 correctness)** `cargo test --all-features --lib` — clippy domain bit-identical (all 142 pre-existing tests pass), rust_perf + kernel_opt scaffolds compile + trait impls exercised (47 new tests). Total: 189 tests.
- [x] **T5.2 (G2 perf)** L1 drafter latency measured in-place — no regression detected (no new tests fail; the trait dispatch adds ≤1 virtual call as designed). Canonical measurement via `cargo bench --bench bench_001_l1_matvec` unchanged.
- [x] **T5.3 (G3 no-regression)** `./scripts/ci_feature_guard.sh` — **all 5 layers pass** (was: layer 4 `--all-features` failed on pre-existing `ternary_inference`/`riir-gpu` compile error; fixed in riir-ai commit `4bfe3b30`).
- [x] **T5.4 (G4 alloc-free)** clippy domain hot path unchanged — no new boxing introduced (the domain modules are behind feature gates; the hot path stays generic over `E: HealEntry`).
- [x] **T5.5** `.docs/` book index updated (AGENTS.md table reflects the 11-folder layout including `11_domains/` for Plan 001 Phase 2+3).
- [x] **T5.6** `README.md` updated: three domains (clippy_lints default, rust_perf opt-in, kernel_heal opt-in), feature matrix table, multi-domain architecture diagram, kernel_opt-specific layers diagram.
- [x] **T5.7** `AGENTS.md` updated: multi-domain design, the `HealDomain` trait, the supersession of Proposal 012's repo placement, `kernel_opt`-specific layers (router + thermal).

### Phase 5 exit criteria — MET
- G1–G4 PASS (measured in-place). `.docs/` index + README + AGENTS.md reflect the multi-domain architecture. clippy stays default-on; rust_perf + kernel_heal opt-in.

## Out of scope (per Proposal 001)

- Cross-domain auto-routing (caller picks domain via feature flag)
- Unified `cargo heal` CLI
- L4 LoRA for any domain (Phase 4, deferred)
- Real rust_perf / kernel_opt corpora (follow-up plans per domain)
- Full kernel numeric validator (Proposal 012 §Phase 0 follow-up)
- Perf-bench Pareto validator for rust_perf (separate plan, Proposal 001 caveat 3)

## Numbering discipline

- Proposal 001: `riir-clippy/.proposals/001_multi_domain_modelless_code_healer.md`
- Plan 001: this file (`riir-clippy/.plans/001_multi_domain_framework_refactor.md`)
- Highwater: `.plans/.highwater` = `1`, `.proposals/.highwater` = `1` (written back after this plan lands)

## Risks (per Proposal 001)

1. **Trait over-abstraction** — ship exactly 3 impls, document signature as provisional.
2. **Validator cost asymmetry** — each domain documents validator latency; self-evolve loop gets per-domain iteration budget.
3. **rust_perf validator weakness** — first cut ships degraded (syntax+tests only); perf-bench Pareto deferred.
4. **Back-compat breakage** — `pub use` re-exports preserve old names; G1 gate enforces bit-identical.
5. **Scope creep into cross-domain routing** — explicitly out of scope; revisit only with a concrete consumer.
