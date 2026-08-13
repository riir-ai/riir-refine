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
| [`rust_perf.md`](rust_perf.md) | Phase 2: `RustPerfDomain` + `PerfRule` + 31-rule seed corpus + `PerfVerifier` (`cargo test`) |
| [`kernel_opt.md`](kernel_opt.md) | Phase 3: `KernelOptDomain` + `KernelRule` + `KernelExpertRouter` (`pick_domain`) + `KernelThermalRouter` (Plasma→Hot→Warm→Cold→Freeze) + `KernelVerifier` (stub) |
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
| `clippy_lints` | ✅ on | — | clippy lint healing |
| `rust_perf` | off | — | rust perf healing (Phase 2) |
| `rust_perf_verify` | off | `rust_perf` | `cargo test` verifier |
| `kernel_heal` | off | `katgpt-core/variable_rank_domain_expert` | GPU kernel opt (Phase 3) |
| `kernel_verify` | off | `kernel_heal` | CubeCL compile verifier (STUB) |

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
- **Phase 3 (kernel_opt):** DONE 2026-08-13. 2 placeholder rules (real
  popcorn distillation deferred per Proposal 012 §Phase 0).
  `KernelExpertRouter` wraps `pick_domain<4,5>` (Plan 558, identity directions
  placeholder). `KernelThermalRouter` returns Plasma for all calls
  (modelless-first mandate). 4090 Warm/Cold/Freeze tiers DEFERRED.
- **Phase 4 (L4 LoRA, all domains):** DEFERRED to riir-train. Reopens when a
  domain's modelless path plateaus below its G1 gate.

## See also

- [`../01_orientation/`](../01_orientation/README.md) — crate role + boundary rationale
- [Proposal 001](../../.proposals/001_multi_domain_modelless_code_healer.md) — the design argument
- [Plan 001](../../.plans/001_multi_domain_framework_refactor.md) — execution plan
- [Proposal 012](../../.proposals/) — GPU kernel optimizer (absorbed into Phase 3)
- [Plan 558](../../../katgpt-rs/.plans/) — `pick_domain` substrate
