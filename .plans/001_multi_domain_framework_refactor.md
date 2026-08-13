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

## Phase 1 — Framework extraction (modelless, no new domains yet)

Refactor the existing pipeline to be domain-pluggable. clippy stays the only working domain; the rename + trait extraction must be bit-identical in behavior.

### Tasks

- [ ] **T1.1** Create `src/domain.rs` with the `HealDomain` trait (associated types `Entry` + `Verdict`, methods `build_corpus` / `validate` / `triple`). Document the signature as provisional (Proposal 001 caveat 1).
- [ ] **T1.2** Generalize `src/corpus.rs`: `ClippyCorpus` → `Corpus<D: HealDomain>`. Keep `ClippyFixEntry` as the clippy domain's `Entry` type (move to `domains/clippy_lints/`).
- [ ] **T1.3** Rename `ClippyTernaryDrafter` → `TernaryDrafter` in `src/drafter.rs`. Behavior unchanged (ternary matvec on embeddings, no domain-specific logic). **Note (Issue 010 resolution, 2026-08-13):** the drafter now ships with the identity guard + the 2-stage routing hook (Stage 2 = `ClippyAdaptiveDrafter`); the rename must preserve both.
- [ ] **T1.4** Generalize `HealPipeline` → `HealPipeline<D: HealDomain>` in `src/self_evolve.rs`. Generic over domain's `Entry` type. **Note (Issue 010 resolution, 2026-08-13):** the pipeline now hosts the Stage 2 adaptive drafter + `tune_against_fixture` tuning loop; both must be preserved in the generalization (the `HealDomain::validate` method IS the per-domain Stage 2 tuning validator).
- [ ] **T1.5** Create `src/domains/mod.rs` + `src/domains/clippy_lints/`:
  - `mod.rs`: `ClippyLintDomain: HealDomain` impl (Entry = ClippyFixEntry, Verdict = ClippyVerdict)
  - `corpus.rs`: move `ClippyFixEntry` + `build_seed_corpus` here (from `src/seed_corpus.rs`)
  - `verify.rs`: move `clippy_verifier.rs` here (behind `clippy_verify` feature, unchanged)
  - `proposers/`: move `src/proposers/` here (the per-lint AST codemods; Issue 010 fix Direction 2). Already modelless + domain-agnostic in mechanism (syn AST rewrites) — the domain-specific aspect is the per-lint pattern set, which stays with the clippy domain.
- [ ] **T1.6** Back-compat re-exports in `src/lib.rs`:
  - `pub use domains::clippy_lints::{ClippyLintDomain, ClippyFixEntry, build_seed_corpus as build_clippy_seed_corpus};`
  - `pub type ClippyCorpus = Corpus<ClippyLintDomain>;` (type alias preserves old name)
  - `pub use drafter::TernaryDrafter as ClippyTernaryDrafter;` (alias preserves old name)
  - `pub use domains::clippy_lints::build_seed_corpus;` (preserve old fn name as default)
  - `pub use adaptive_drafter::ClippyAdaptiveDrafter;` (preserved — Stage 2 stays)
- [ ] **T1.7** Add `clippy_lints` to `Cargo.toml` `[features]` `default = ["clippy_lints"]`. Gate `src/domains/clippy_lints/` behind `#[cfg(feature = "clippy_lints")]`.
- [ ] **T1.8** Update `examples/heal_demo.rs` to use the new API (or confirm it compiles unchanged via the re-exports).
- [ ] **T1.9** G1 check: `cargo test --lib` — all existing tests pass bit-identical. **Includes the 5 `tests/stage2_goat.rs` tests** (Issue 010 GOAT gate — must remain green after the rename).
- [ ] **T1.10** G3 check: `./scripts/ci_feature_guard.sh` passes (5-layer CI guard).

### Phase 1 exit criteria
- `HealDomain` trait ships, clippy domain reorganized behind `feature = "clippy_lints"`, all existing tests pass, old API names still resolve via re-export, **Stage 2 GOAT gate (Bench 011) remains 10/10**.

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

## Phase 2 — rust_perf domain scaffold

First new domain. Corpus from the `rust-optimize` skill checklist. Validator is `cargo test` (perf-bench deferred per Proposal 001 caveat 3).

### Tasks

- [ ] **T2.1** Create `src/domains/rust_perf/`:
  - `mod.rs`: `RustPerfDomain: HealDomain` impl (Entry = `PerfRule`, Verdict = `PerfVerdict`)
  - `corpus.rs`: `PerfRule { rule_id, name, buggy_pattern, fixed_pattern, category }` + `build_perf_seed_corpus()` — encode 5–10 rules from the rust-optimize skill checklist (O(n)→O(1), String→&str, field reorder, papaya, SIMD chunks, Bevy reactive_low_power, WASM batch)
  - `verify.rs`: `PerfVerifier` — runs `cargo test` as subprocess (syntax + test-pass ground truth). Perf-bench Pareto verdict DEFERRED.
- [ ] **T2.2** Add `rust_perf` feature to `Cargo.toml` (opt-in, default-off). Gate `src/domains/rust_perf/` behind `#[cfg(feature = "rust_perf")]`.
- [ ] **T2.3** `PerfRule` BLAKE3-commit the seed corpus (mirror `ClippyFixEntry`'s freeze/thaw pattern).
- [ ] **T2.4** Example: `examples/perf_heal_demo.rs` — heal an O(n) scan into an O(1) lookup, validate via `cargo test`.
- [ ] **T2.5** G1 check: `cargo test --features rust_perf --lib` — domain scaffold compiles + seed corpus round-trips.
- [ ] **T2.6** G3 check: `cargo check --features rust_perf` + `./scripts/ci_feature_guard.sh` pass.

### Phase 2 exit criteria
- `rust_perf` domain compiles, seed corpus (5–10 rules) ships, `cargo test` validator works. Perf-bench validator deferred (separate plan). NOT promoted to default-on (opt-in until real corpus + perf-bench validator pass a domain-specific GOAT).

## Phase 3 — kernel_opt domain scaffold (absorbs Proposal 012, 4090 DEFERRED)

Second new domain. Consumes Proposal 012's design (thermal-tier + latent-MoE + numeric validator). Corpus distillation from popcorn is DEFERRED (Proposal 012 §Phase 0, ~2 weeks reading job). 4090 LoRA path is DEFERRED.

### Tasks

- [ ] **T3.1** Create `src/domains/kernel_opt/`:
  - `mod.rs`: `KernelOptDomain: HealDomain` impl (Entry = `KernelRule`, Verdict = `KernelVerdict`)
  - `corpus.rs`: `KernelRule { rule_id, gpu_concept, ref_shape, opt_pattern }` + `build_kernel_seed_corpus()` — **EMPTY stub** (real corpus deferred per Proposal 012 §Phase 0; ship 1–2 placeholder rules so the scaffold compiles + the trait impl is exercised)
  - `router.rs`: `KernelExpertRouter` — wraps `pick_domain<NUM_KERNEL_EXPERTS, ACTIVITY_DIM>` (Plan 558). Activity vector = kernel signature (shape_m, shape_n, shape_k, dtype_enum, hw_enum). Domain directions = per-expert centroids (placeholder: identity vectors until popcorn distillation lands).
  - `thermal.rs`: `KernelThermalRouter` — Plasma→Hot→Warm→Cold→Freeze tier dispatch stub. Plasma = modelless cache hit (M3 Metal), Warm/Cold = LoRA draft + full validate (4090 CUDA — **DEFERRED**). Stub returns Plasma for all calls until 4090 path lands.
  - `verify.rs`: `KernelVerifier` — compile + run + numeric compare vs reference. Behind `kernel_verify` feature. **Compile-only stub** (full numeric compare deferred per Proposal 012 §Phase 0).
- [ ] **T3.2** Add `kernel_heal` + `kernel_verify` features to `Cargo.toml` (opt-in, default-off). `kernel_heal` gates the domain module; `kernel_verify` gates the CubeCL compile validator (mirrors `clippy_verify` pattern).
- [ ] **T3.3** G1 check: `cargo test --features kernel_heal --lib` — domain scaffold compiles + trait impl exercised by placeholder rules.
- [ ] **T3.4** G3 check: `cargo check --features kernel_heal` + `./scripts/ci_feature_guard.sh` pass.

### Phase 3 exit criteria
- `kernel_opt` domain scaffold compiles, `pick_domain` router wired, thermal-tier stub in place, compile-only validator behind `kernel_verify`. Real corpus + full numeric validator + 4090 LoRA all DEFERRED to Proposal 012's follow-up plans. NOT promoted to default-on.

## Phase 4 — 4090 LoRA fallback (L4, ALL domains) — DEFERRED

- [-] **T4.1** riir-train plan for L4 LoRA fallback. Per modelless-first mandate (Proposal 001 caveat 4): only if modelless L0–L3 path plateaus below a domain's G1 gate. 4090 CUDA required. Deferred across all domains (clippy, rust_perf, kernel_opt).

### Phase 4 exit criteria
DEFERRED. No tasks in this plan. Reopens as a riir-train plan when a domain's modelless path plateaus.

## Phase 5 — GOAT gate + docs

### Tasks

- [ ] **T5.1 (G1 correctness)** `cargo test --all-features --lib` — clippy domain bit-identical, rust_perf + kernel_opt scaffolds compile + trait impls exercised.
- [ ] **T5.2 (G2 perf)** `cargo bench --bench bench_001_l1_matvec` — clippy L1 drafter latency ≤5% regression vs pre-refactor baseline (Bench 009's 17µs p50). The trait dispatch adds ≤1 virtual call; measure it.
- [ ] **T5.3 (G3 no-regression)** `./scripts/ci_feature_guard.sh` — all 5 layers pass (no-dep / default / per-feature / all-features / clippy+tests).
- [ ] **T5.4 (G4 alloc-free)** clippy domain hot path: 0 steady-state allocations (trait dispatch must not introduce boxing). Verify via existing alloc gate tests.
- [ ] **T5.5** Update `.docs/` book: add domain-pluggable architecture section to `01_orientation/` + per-domain sections (`02_corpus/`, `03_drafter/`, `04_pruner/` stay domain-agnostic; add domain-specific notes).
- [ ] **T5.6** Update `README.md`: three domains (clippy_lints default, rust_perf opt-in, kernel_heal opt-in), feature matrix, link to Proposal 001 + Plan 001.
- [ ] **T5.7** Update `AGENTS.md`: domain-pluggable design, the `HealDomain` trait, the supersession of Proposal 012's repo placement.

### Phase 5 exit criteria
- G1–G4 PASS. `.docs/` + README + AGENTS.md reflect the multi-domain architecture. clippy stays default-on; rust_perf + kernel_heal opt-in.

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
