# 11 — Multi-domain architecture (Plan 001)

> **Purpose:** the domain-pluggable framework behind the `HealDomain` trait
> (Plan 001). Three domains ship: `clippy_lints` (default), `rust_perf`
> (Phase 2 opt-in), `kernel_opt` (Phase 3 opt-in, absorbs Proposal 012). Each
> domain owns its corpus + validator; the domain-agnostic core (drafter +
> pruner + ruliology + self-evolve) operates on the corpus entries this trait
> produces.

## Files

| File | Scope |
|---|---|
| [`heal_domain_trait.md`](heal_domain_trait.md) | `HealDomain` + `HealEntry` traits — the one new abstraction |
| [`rust_perf.md`](rust_perf.md) | Phase 2: `RustPerfDomain` + `PerfRule` + seed corpus (now 158 rules — measured-GOAT + external distills, per-batch ledger inside) + `PerfVerifier` (`cargo test`); Phase 4: top-5 `syn` AST proposers (`--fix` upgrade) |
| [`kernel_opt.md`](kernel_opt.md) | Phase 3: `KernelOptDomain` + `KernelRule` + `KernelExpertRouter` (`pick_domain`) + `KernelThermalRouter` (Plasma→Hot→Warm→Cold→Freeze) + `KernelVerifier` (stub) |
| [`sec.md`](sec.md) | Plan 018: `SecDomain` + `SecRule` + the geiger-taxonomy unsafe counter + the banned-list boundary gate + 16 rules across six intake waves (6 fixable proposers + the Batch-131/140/165 suggest-only detectors) |
| [`docker.md`](docker.md) | Plan 030: `DockerDomain` + `DockerRule` + the stanza scanner (no `syn`) + the hadolint oracle tier (incl. the Tier-2 `docker build --check` oracle) + the DL3059-refusal gate + 31 rules (14 bounded fixes) |
| [`dist.md`](dist.md) | Issue 085: `DistDomain` + `DistRule` + the section-aware manifest scanner (no `syn`, no `toml`) + the refuses gate + 6 suggest-only rules (release-profile size knobs + the workspace-member profile trap + the Batch-143 large-API dependency hint + the Batch-146 PGO training-set hint) |
| [`shader_perf.md`](shader_perf.md) | Plan 134 / Proposal 014: `ShaderPerfDomain` + `ShaderRule` + the block-aware WGSL render-stage scanner (no `syn`, no `regex`) + the naga oracle tier + the refusal gate + 8 rules (1 bounded: the loop-guard proposer; the negative control pins ZERO findings on the 190-file vgpu port corpus) |
| [`cross_domain_routing.md`](cross_domain_routing.md) | Why cross-domain auto-routing is NOT in scope (Proposal 001) |

## Source

- [`src/domain.rs`](../../src/domain.rs) — `HealDomain` + `HealEntry` traits
- [`src/corpus.rs`](../../src/corpus.rs) — `Corpus<D: HealDomain>` (generic)
- [`src/domains/clippy_lints/`](../../src/domains/clippy_lints/) — clippy domain (default)
- [`src/domains/rust_perf/`](../../src/domains/rust_perf/) — rust_perf domain (Phase 2)
- [`src/domains/kernel_opt/`](../../src/domains/kernel_opt/) — kernel_opt domain (Phase 3)

## Feature gates

| Feature | Default | Implies | Domain |
|---|---|---|---|
| `clippy_lints` | ✅ on | — | clippy lint fixing |
| `rust_perf` | ✅ on (since 2026-08-13) | — | rust perf fixing (Phase 2) |
| `rust_perf_verify` | off | `rust_perf` | `cargo test` verifier |
| `kernel_opt` | off (✅ default via `kernel_pregate` since 2026-11-06) | `katgpt-core/committed_field_blend`, `dep:regex` | GPU kernel opt (Phase 3; 1076 rules) |
| `kernel_verify` | off | `kernel_opt` | CubeCL compile verifier (STUB) |
| `sec` | off | — | security-pattern fixing (Plan 018) |
| `docker` | ✅ on (since 2026-08-28) | — | Dockerfile bounded-fix drafting (Plan 030) |
| `docker_verify` | off | `docker` | hadolint agreement oracle |
| `docker_buildcheck` | off | `docker_verify` | Tier-2 `docker build --check` agreement oracle |
| `dist` | ✅ on (since 2026-09-13) | — | shipped-artifact build-config advice (Issue 085; promoted on Bench 082 + §6 evidence + owner pull) |

## Status

- **Phase 1 (framework refactor):** DONE 2026-08-13. `HealDomain` trait
  shipped, clippy domain reorganized behind `clippy_lints`, all 142 existing
  tests pass bit-identical.
- **Phase 2 (rust_perf):** DONE 2026-08-13. 25 seed rules across 11 categories
  (complexity, allocation, layout, concurrency, simd, arithmetic, iterators,
  dispatch, caching, bevy, wasm) — sources: `rust-optimize` skill checklist +
  nnethercote/perf-book. `cargo test` verifier works (compile-error
  detection via exit status + JSON parsing). Perf-bench Pareto validator DONE
  (Bench 013 GOAT G1+G5 PASS) — `rust_perf` promoted to default-on 2026-08-13.
- **Phase 3 (kernel_opt):** DONE 2026-08-13. 34 rules (Batch 1-3: 2 original + 32
  popcorn-distilled per Research 002-004; full corpus ~300-500 rules in ~15-25 batches).
  `KernelExpertRouter` wraps `pick_domain<15,8>` (Plan 558; Plan 004 extended the
  activity vector from 5 to 8 dims with operation-class one-hot axes; 15/15
  categories reachable). `KernelThermalRouter` returns Plasma for all calls
  (modelless-first mandate). 4090 Warm/Cold/Freeze tiers DEFERRED.
- **Phase 4 (L4 LoRA, all domains):** DEFERRED to riir-train. Reopens when a
  domain's modelless path plateaus below its G1 gate.

## See also

- [`../01_orientation/`](../01_orientation/README.md) — crate role + boundary rationale
- [Proposal 001](../../.proposals/001_multi_domain_modelless_code_healer.md) — the design argument
- [Plan 001](../../.plans/001_multi_domain_framework_refactor.md) — execution plan
- [Proposal 012](../../.proposals/) — GPU kernel optimizer (absorbed into Phase 3)
- [Plan 558](../../../katgpt-rs/.plans/) — `pick_domain` substrate
