# riir-clippy

Clippy-healing drafter+pruner pipeline: modelless ternary drafter × rust-analyzer validator × adaptive LoRA fallback.

## Status

**POC — Phase 0 (rule distillation) + L1 (drafter) + L2 (syntax validator) done.**

The clippy-healing pipeline that distills clippy rules into a ruliology-style
search space, drafts fixes via a modelless ternary SIMD matvec, and validates
with a `syn`-based syntax pruner. Heavyweight LoRA is the last-resort
fallback, not the co-equal path.

| Layer | Status |
|---|---|
| L0 — Corpus (rule distillation) | ✅ Done (BLAKE3-committed freeze/thaw + seed corpus with 10 common lints) |
| L1 — Ternary SIMD drafter | ✅ Done (Issue 001, zero-alloc matvec, p50=2.3µs) |
| L2 — Syntax validator | ✅ Done (Issue 002, `syn`-based parse check, p50=6.8µs) |
| L2 — Lint resolution check | ✅ Done (Issue 003, `clippy_verify` feature, real `cargo clippy` JSON parsing, ~130ms/verify) |
| L3 — Ruliology search | ✅ Done (Issue 005, `ruliology_search` feature, Pareto-front from katgpt-ruliology pattern, ~400µs/10-candidate search) |
| L4 — LoRA fallback | ⏳ Deferred to riir-train |

## Architecture

```text
Clippy corpus (BLAKE3-committed freeze/thaw)
  (lint_name, buggy_code, fixed_code, lint_group) tuples
         │
         ▼
L1 — MODELLESS DRAFTER (PlasmaPath ternary SIMD matvec)
  Frozen clippy-fix direction vectors (.bits files)
  Template fallback when no weights loaded
  Zero-alloc hot path
         │
         ▼
L2 — VALIDATOR (always-on)
  `syn` in-process parse check (fast, modelless)
  Phase 3: `cargo clippy` re-run for lint resolution (feature `clippy_verify`)
         │
         ▼
  [ validated clippy fix ]
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

The primary path (L0-L3) is entirely modelless — no training, no backprop:
- **L0**: rule distillation (named rules + bounded fix-space)
- **L1**: ternary SIMD matvec drafter (frozen direction vectors)
- **L2**: rust-analyzer validation
- **L3** (opt-in): ruliology Pareto-front candidate search

The LoRA fallback (L4) is deferred to riir-train and is the conservative last
resort, not a co-equal path.

## Quick Start

```sh
# Run the end-to-end demo (L0 corpus → L1 drafter → L2 pruner)
cargo run --example heal_demo

# Full CI guard (5 layers: no-dep / default / per-feature / all-features / clippy+tests)
./scripts/ci_feature_guard.sh

# Re-run GOAT benches (results persisted in .benchmarks/001_goat_consolidated.md)
cargo bench --bench bench_001_l1_matvec      # L1 hot path
cargo bench --bench bench_002_l2_pruner_syn   # L2 hot path
cargo run --example ruliology_search_demo --features ruliology_search --release  # L3 opt-in
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

- [Bench 001](.benchmarks/001_goat_consolidated.md) — consolidated GOAT gate results (L1 + L2 + Phase 3 + L3)
- [Proposal 034](../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) — the design argument
- [Plan 331 Phase A](../riir-train/.plans/331_lora_rust_coder_candidates_consolidated.md) — Gemma-2 r16 production (LoRA fallback base)
- [Plan 333](../riir-train/.plans/333_bitnet_ternary_moe_neuro_symbolic_poc.md) — Ternary-Bonsai-27B (LoRA fallback primary base)
- [RustAssistant](https://arxiv.org/abs/2412.16358) (Microsoft Research, 2024) — 75% prior art ceiling
- [Proposal 011](../katgpt-rs/.proposals/011_rust_swe_bench_latent_space_via_wasm_pruner.md) — WASM rustc pruner (rubrc)

## License

MIT OR Apache-2.0
