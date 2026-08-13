# riir-clippy

**Multi-domain modelless code healer** — a pluggable framework behind the
`HealDomain` trait that distills code rules into a ruliology-style search
space, drafts fixes via a modelless ternary SIMD matvec, and validates with a
`syn`-based syntax pruner. Three domains ship (Plan 001):

| Domain | Feature | Status | Corpus |
|---|---|---|---|
| **clippy_lints** (default) | `clippy_lints` | ✅ Production | 10 common lints (style/complexity/perf/nursery) |
| **rust_perf** (default since 2026-08-13) | `rust_perf` | ✅ Phase 2 (GOAT G1+G5 PASS, Bench 013) | 39 rules from `rust-optimize` skill + nnethercote/perf-book + rustc-hash/bevy patterns (complexity/allocation/layout/concurrency/simd/arithmetic/iterators/dispatch/caching/bevy/wasm); 4 carry `context_notes` (2 ParetoDominated + 2 Mixed in default config) |
| **kernel_opt** (opt-in) | `kernel_heal` | ✅ Phase 3 scaffold | 2 placeholder rules (matvec-coalesce, gemm-tiling); real popcorn distillation deferred (Proposal 012 §Phase 0) |

Heavyweight LoRA is the last-resort fallback, not the co-equal path.

## Status

**POC — multi-domain framework (Plan 001) + Phase 0 (rule distillation) +
L1 (drafter) + L2 (syntax validator) + additive self-evolution loop done.**

The clippy-healing pipeline that distills clippy rules into a ruliology-style
search space, drafts fixes via a modelless ternary SIMD matvec, and validates
with a `syn`-based syntax pruner. Heavyweight LoRA is the last-resort
fallback, not the co-equal path. Plan 001 generalized the framework to be
domain-pluggable behind the `HealDomain` trait.

| Layer | Status |
|---|---|
| L0 — Corpus (rule distillation) | ✅ Done (BLAKE3-committed freeze/thaw + seed corpus with 10 common lints + 20 perf rules + 2 kernel rules) |
| L1 — Ternary SIMD drafter | ✅ Done (Issue 001, zero-alloc matvec, p50=2.3µs) — ⚠️ **corpus-resident lints only**, see Issue 010 |
| L2 — Syntax validator | ✅ Done (Issue 002, `syn`-based parse check, p50=6.8µs) |
| L2 — Lint resolution check (clippy) | ✅ Done (Issue 003, `clippy_verify` feature, real `cargo clippy` JSON parsing, ~130ms/verify) |
| L2 — Test pass check (rust_perf) | ✅ Done (Plan 001 Phase 2, `rust_perf_verify` feature, real `cargo test` + compile-error detection via exit status + JSON parsing) |
| L2 — Compile check (kernel_opt) | ✅ Stub (Plan 001 Phase 3, `kernel_verify` feature, delegates to shared `RustAnalyzerPruner`; full CubeCL compile + numeric compare deferred per Proposal 012 §Phase 0) |
| L3 — Ruliology search | ✅ Done (Issue 005, `ruliology_search` feature, Pareto-front from katgpt-ruliology pattern, ~400µs/10-candidate search) |
| SE — Self-evolution loop (additive) | ✅ Done (Issue 004 + Issue 008; `self_evolve` feature, opt-in; G1–G5 + G6/G7 ALL PASS — 50.0pp healing-rate improvement on real `cargo clippy` feedback) |
| Router — kernel_opt expert routing | ✅ Stub (Plan 001 Phase 3, `KernelExpertRouter` wraps `pick_domain<4,5>` from Plan 558; identity directions placeholder) |
| Thermal — kernel_opt tier dispatch | ✅ Stub (Plan 001 Phase 3, `KernelThermalRouter` + `ThermalTier` enum; Plasma→Hot→Warm→Cold→Freeze; returns Plasma for all calls) |
| L4 — LoRA fallback (all domains) | ⏳ Deferred to riir-train |

## Scope limit (measured, Bench 009)

The pipeline is a **fast cache over corpus-resident lint fixes**, not a general
clippy fixer. Measured on 10 lints:

| | in-distribution (corpus holds the fix) | held-out (same lints, unseen code) |
|---|---|---|
| exact-match | **10/10** | **0/10** |
| e2e throughput vs Bonsai-27B | ~74,000× tok/s | ~68,000× tok/s |

The L1 drafter returns corpus `fixed_code` **verbatim** — the input code selects
*which* stored fix to return but never influences the text ([Issue 010](.issues/010_drafter_cannot_generalize_heldout_code.md)).
On unseen code it therefore emits a valid-but-wrong function, and L2 reports
`parses = true` on it, so **a cache miss is currently silent**. Deploy it as a
cache with a fallback path, and do not read the throughput number as a
capability claim. For reference, Ternary-Bonsai-27B scored 10/10 on both
fixtures at ~16 tok/s.

## Multi-domain architecture (Plan 001)

The framework is domain-pluggable behind the [`HealDomain`](src/domain.rs)
trait. Each domain owns: (1) what counts as a fixable rule, (2) how to
validate a proposed fix against ground truth. The domain-agnostic core
(drafter + pruner + ruliology + self-evolve) operates on the corpus entries
this trait produces.

```text
src/
├── domain.rs           # HealDomain + HealEntry traits (the one new abstraction)
├── corpus.rs           # Corpus<D: HealDomain> — BLAKE3-committed freeze/thaw
├── drafter.rs          # TernaryDrafter (Stage 1) — generic over E: HealEntry
├── adaptive_drafter.rs # AdaptiveDrafter (Stage 2) — domain-agnostic
├── pruner.rs           # RustAnalyzerPruner — domain-agnostic syntax check
├── ruliology_search.rs # Pareto-front candidate search (opt-in)
├── self_evolve.rs      # HealPipeline + LatentFixMemory + LatentSkillEvolution
└── domains/
    ├── clippy_lints/   # feature "clippy_lints" (DEFAULT) — clippy::* lint healing
    │   ├── corpus.rs   # 10 common clippy lints
    │   ├── verify.rs   # cargo clippy re-run (behind clippy_verify)
    │   └── proposers/  # 10 per-lint AST codemods (Issue 010 Stage 2)
    ├── rust_perf/      # feature "rust_perf" (default-on since 2026-08-13) — rust-optimize perf healing
    │   ├── corpus.rs   # 20 perf rules from the rust-optimize skill checklist
    │   ├── verify.rs   # cargo test re-run (behind rust_perf_verify)
    │   └── bench.rs    # perf-bench Pareto validator (behind rust_perf_bench, the GOAT gate)
    └── kernel_opt/     # feature "kernel_heal" (opt-in, 4090 DEFERRED)
        ├── corpus.rs   # 2 placeholder rules (popcorn distillation DEFERRED)
        ├── router.rs   # KernelExpertRouter wraps pick_domain<4,5> (Plan 558)
        ├── thermal.rs  # Plasma→Hot→Warm→Cold→Freeze tier dispatch (returns Plasma)
        └── verify.rs   # compile-only stub (behind kernel_verify)
```

**Cross-domain auto-routing is NOT in scope** — each domain is selected by
enabling its feature flag. A unified `cargo heal` CLI that auto-detects
domain from code shape is a future design sketch (Proposal 001 §Product
vision), not shipped.

### Feature matrix

| Feature | Default | Implies | Purpose |
|---|---|---|---|
| `clippy_lints` | ✅ on | — | clippy lint healing domain |
| `clippy_verify` | off | `clippy_lints` | real `cargo clippy` lint resolution check |
| `rust_perf` | ✅ on | — | rust perf healing domain (Plan 001 Phase 2; promoted 2026-08-13, Bench 013 G1+G5) |
| `rust_perf_verify` | off | `rust_perf` | real `cargo test` test pass check |
| `rust_perf_bench` | off | `rust_perf` | perf-bench Pareto validator (the GOAT gate; Bench 013) |
| `kernel_heal` | off | `katgpt-core/variable_rank_domain_expert` | GPU kernel opt domain (Plan 001 Phase 3) |
| `kernel_verify` | off | `kernel_heal` | CubeCL compile check (STUB — full validator deferred) |
| `ruliology_search` | off | `clippy_lints` | L3 Pareto-front candidate search |
| `self_evolve` | off | — | additive self-evolution loop (memory + EMA) |
| `ternary_inference` | off | `riir-engine`, `riir-gpu` | Bench 010: katgpt ternary generation on clippy prompts |

## Architecture

```text
Domain corpus (BLAKE3-committed freeze/thaw)
  clippy_lints: (lint_name, lint_group, buggy_code, fixed_code) tuples
  rust_perf:    (rule_id, name, category, buggy_code, fixed_code) tuples
  kernel_opt:   (rule_id, gpu_concept, ref_shape, opt_pattern) tuples
         │
         ▼
L1 — MODELLESS DRAFTER (PlasmaPath ternary SIMD matvec)
  Frozen direction vectors (.bits files)
  Template fallback when no weights loaded
  Zero-alloc hot path
         │
         ▼
L2 — VALIDATOR (always-on, domain-specific verifier opt-in)
  `syn` in-process parse check (fast, modelless, always-on)
  clippy_lints: `cargo clippy` re-run (feature `clippy_verify`)
  rust_perf:    `cargo test` re-run (feature `rust_perf_verify`)
  kernel_opt:   CubeCL compile (feature `kernel_verify`, STUB)
         │
         ▼
  [ validated fix ]
         │
         ▼ (opt-in, additive — feature `self_evolve`)
SE — SELF-EVOLUTION LOOP (composes on top of L0–L2)
  LatentFixMemory   : 3-axis retrieval over past fix trajectories
                     (latent dot+sigmoid, BM25 lexical, identical-triple dedup)
  LatentSkillEvolution: EMA per-lint direction-vector update
  Heals + records outcome → next-call retrieval benefits
  Degenerates to static-corpus path when memory is empty
```

### kernel_opt-specific layers (Plan 001 Phase 3)

```text
Kernel signature (M, N, K, dtype, hw)
         │
         ▼
Router — KERNEL EXPERT ROUTER (pick_domain<4,5>, Plan 558)
  Activity vector · domain_directions → expert index
  Placeholder: identity directions (real centroids deferred)
         │
         ▼
Thermal — KERNEL THERMAL ROUTER (5-tier escalation)
  Plasma (modelless cache, M3 Metal) → Hot (Stage 2 adaptive) →
  Warm (LoRA draft, 4090 CUDA, DEFERRED) → Cold (full validate, DEFERRED) →
  Freeze (archive, DEFERRED)
  Scaffold: always returns Plasma (modelless-first mandate)
```

## L3 — Ruliology candidate search

When the L1 drafter proposes multiple candidates and L2 accepts more than one,
L3 enumerates the Pareto-front of (lint_satisfied, AST delta size) and picks
the minimum-complexity fix among the Pareto-optimal set.

The Pareto-front algorithm is reimplemented locally (~30 LOC) from
`katgpt-rs/crates/katgpt-ruliology/src/types.rs::WinMatrix::pareto_front`
(GOAT PASS, Bench 572). The substrate's version is coupled to `WinMatrix`
(2-player tournament semantics) + non-optionally pulls `katgpt-pruners`
(g_zero feature) — too heavy for clippy's needs. Clippy fixes have ground-truth
payoff (lint_satisfied: bool), not pairwise tournament payoffs, so the
`WinMatrix` type doesn't fit. This mirrors the repo's existing pattern:
`ClippyTernaryDrafter` reuses the `TernaryDraftModel` pattern from
`quest_grammar` without a code-level dep.

## Modelless-first mandate

The primary path (L0–L3) is entirely modelless — no training, no backprop:
- **L0**: rule distillation (named rules + bounded fix-space)
- **L1**: ternary SIMD matvec drafter (frozen direction vectors)
- **L2**: rust-analyzer validation
- **L3** (opt-in): ruliology Pareto-front candidate search
- **SE** (opt-in, additive): self-evolution loop — latent retrieval + EMA
  direction update. No gradient descent; the only state mutations are
  append-only memory + EMA direction refresh, matching the global
  modelless-first mandate.

The LoRA fallback (L4) is deferred to riir-train and is the conservative last
resort, not a co-equal path.

## Quick Start

```sh
# Run the clippy healing demo (L0 corpus → L1 drafter → L2 pruner)
cargo run --example heal_demo

# Run the rust_perf domain demo (Plan 001 Phase 2; default-on since 2026-08-13)
cargo run --example perf_heal_demo

# Full CI guard (5 layers: no-dep / default / per-feature / all-features / clippy+tests)
./scripts/ci_feature_guard.sh

# Re-run GOAT benches (results persisted in .benchmarks/001_goat_consolidated.md)
cargo bench --bench bench_001_l1_matvec      # L1 hot path
cargo bench --bench bench_002_l2_pruner_syn   # L2 hot path
cargo run --example ruliology_search_demo --features ruliology_search --release  # L3 opt-in

# Self-evolution real-world validation (~30s; spawns real cargo clippy)
cargo run --example self_evolve_real_world_demo --features "self_evolve clippy_verify" --release
```

## Usage

```rust
use riir_clippy::{build_seed_corpus, ClippyTernaryDrafter, RustAnalyzerPruner};

// Build the corpus from seed entries (or your own)
let corpus = build_seed_corpus();

// Draft a fix (fallback mode = corpus template lookup)
let mut drafter = ClippyTernaryDrafter::fallback();
let candidates: Vec<_> = corpus.lookup("needless_return").cloned().collect();
let drafted = drafter.draft_fix("needless_return", buggy_code, &candidates)?;

// Validate with the syn-based syntax pruner
let pruner = RustAnalyzerPruner::new();
let result = pruner.validate(&drafted.code, "needless_return");
if result.is_valid() {
    // Apply the fix
}
```

## References

- [Proposal 001](.proposals/001_multi_domain_modelless_code_healer.md) — the multi-domain meta-architecture argument
- [Plan 001](.plans/001_multi_domain_framework_refactor.md) — execution plan (Phases 1–5)
- [Bench 001](.benchmarks/001_goat_consolidated.md) — consolidated GOAT gate results (L1 + L2 + Phase 3 + L3)
- [Bench 008](.benchmarks/008_real_world_validation.md) — real-world validation of `self_evolve` via `cargo clippy` (G6 + G7)
- [Bench 009](.benchmarks/009_bonsai_vs_modelless_e2e_tokps.md) — e2e tok/s vs Ternary-Bonsai-27B on llama.cpp (~74,000× throughput; 0/10 held-out quality)
- [Bench 011](.benchmarks/011_stage2_goat.md) — Stage 2 adaptive drafter GOAT (Issue 010 resolution)
- [`.docs/10_self_evolve/`](.docs/10_self_evolve/) — design notes for the self-evolution loop (HealPipeline, LatentFixMemory, LatentSkillEvolution)
- [Proposal 034](../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) — the original clippy-healing design argument
- [Proposal 012](.proposals/) — GPU kernel optimization (absorbed into Plan 001 Phase 3 as `kernel_opt` domain)
- [Plan 558](../katgpt-rs/.plans/) — `pick_domain` substrate (consumed by `KernelExpertRouter`)
- [Plan 331 Phase A](../riir-train/.plans/331_lora_rust_coder_candidates_consolidated.md) — Gemma-2 r16 production (LoRA fallback base)
- [Plan 333](../riir-train/.plans/333_bitnet_ternary_moe_neuro_symbolic_poc.md) — Ternary-Bonsai-27B (LoRA fallback primary base)
- [RustAssistant](https://arxiv.org/abs/2412.16358) (Microsoft Research, 2024) — 75% prior art ceiling
- [Proposal 011](../katgpt-rs/.proposals/011_rust_swe_bench_latent_space_via_wasm_pruner.md) — WASM rustc pruner (rubrc)

## License

MIT OR Apache-2.0
