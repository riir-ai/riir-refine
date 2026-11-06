# riir-refine `.docs/` book

> **Purpose:** architectural narrative reference for the multi-domain
> modelless rulewright. Numbered folders for sort order, unnumbered
> `.md` files inside each. Each folder's `README.md` is the local index.

This book documents the **modelless-first code-fixing pipeline** that
distills rules into a ruliology-style search space, drafts fixes via a
modelless ternary SIMD matvec, validates with a `syn`-based syntax pruner,
and refines via Pareto-front candidate search. Three domains ship (Plan 001):
`clippy_lints` (default), `rust_perf` (Phase 2, default-on since 2026-08-13), `kernel_opt` (Phase 3
opt-in). LoRA (L4) is the conservative last-resort fallback, not the
co-equal path.

## Folder index

| # | Folder | Scope |
|---|---|---|
| 01 | [`01_orientation/`](01_orientation/README.md) | Crate role, sibling layout, boundary rationale, build commands, KAT billing + TUNA surfaces |
| 02 | [`02_corpus/`](02_corpus/README.md) | L0 rule distillation: `Corpus<D>` (generic) + BLAKE3 commitment + per-domain seed corpora |
| 03 | [`03_drafter/`](03_drafter/README.md) | L1 ternary SIMD matvec drafter (`TernaryDrafter`, generic over `E: HealEntry`) + BLAKE3 modelless embedder + Plan 005 latent retrieval (`RuleIndex`/`DomainRouter`/`LatentMatcher`, the `suggest` MVP) |
| 04 | [`04_pruner/`](04_pruner/README.md) | L2 `syn`-based syntax validator (domain-agnostic) |
| 05 | [`05_clippy_verify/`](05_clippy_verify/README.md) | clippy lint resolution via real `cargo clippy` (opt-in) |
| 06 | [`06_ruliology/`](06_ruliology/README.md) | L3 Pareto-front candidate search (opt-in) |
| 07 | [`07_examples/`](07_examples/README.md) | The 8 in-tree examples + the `cargo-refine` bin (`cargo refine suggest`) |
| 08 | [`08_benchmarks/`](08_benchmarks/README.md) | GOAT gate methodology + measured results |
| 09 | [`09_lessons/`](09_lessons/README.md) | Canonical patterns + honest POC caveats |
| 10 | [`10_self_evolve/`](10_self_evolve/README.md) | Issue 004 + Issue 008: latent-first self-evolution loop (G1–G7 ALL PASS) |
| 11 | [`11_domains/`](11_domains/README.md) | **Plan 001:** multi-domain architecture — `rust_perf` (Phase 2) + `kernel_opt` (Phase 3, absorbs Proposal 012) |
| 12 | [`12_ane/`](12_ane/README.md) | Apple Neural Engine substrate knowledge — private runtime API, MIL/blob formats, M3 Max (h15g) hardware findings, the Rust-bridge negative result + working ObjC substrate, ecosystem map (Issue 726 T0 distillation) |

## Pipeline overview

```text
Real .rs files (Plan 005 — feature `latent_retrieval`, the suggest MVP)
  AstChunker spans → ModellessEmbedder (8-D) → DomainRouter (annotation)
  → RuleIndex fan-out retrieve → ranked rules + pattern snippets   ← 03_drafter/latent_suggest.md
         │
         ▼
Domain corpus (BLAKE3-committed freeze/thaw)        ← L0 (02_corpus)
  clippy_lints: (lint_name, lint_group, buggy_code, fixed_code)
  rust_perf:    (rule_id, name, category, buggy_code, fixed_code)
  kernel_opt:   (rule_id, gpu_concept, ref_shape, opt_pattern)
         │
         ▼
L1 — MODELLESS DRAFTER (ternary SIMD matvec)        ← L1 (03_drafter)
  Frozen direction vectors (.bits files)
  BLAKE3-deterministic embedding (rule_id, buggy_code) → [f32; 8]
  Template fallback when no weights loaded
  Zero-alloc hot path
         │
         ▼
L2 — VALIDATOR (always-on, domain-specific opt-in)  ← L2 (04_pruner)
  `syn` in-process parse check (fast, modelless, always-on)
  clippy_lints: `cargo clippy` re-run (05_clippy_verify)
  rust_perf:    `cargo test` re-run (11_domains/rust_perf)
  kernel_opt:   CubeCL compile (11_domains/kernel_opt, STUB)
         │
         ▼
L3 — RULIOLOGY SEARCH (opt-in)                      ← L3 (06_ruliology)
  Pareto-front of (rule_satisfied, AST delta size)
  Picks minimum-complexity fix among Pareto-optimal set
         │
         ▼
  [ validated fix ]
```

The Plan 005 latent path is **suggest + fix** at the CLI — the Phase 4
rust_perf `propose_fix` proposers + the T6.4 clippy emitter migration are
wired into `cargo refine fix` (span-preserving edits; the proposer visitor
is the gate). See [`07_examples/cargo_refine.md`](07_examples/cargo_refine.md).

## Resolved validation

- **Issue 008** (resolved + removed per noise-reduction) — real-world
  validation of `self_evolve` via `cargo clippy` feedback (G6 corpus
  truthfulness + G7 self-evolution with real clippy). Both gates PASS.
  See `.benchmarks/008_real_world_validation.md`.

## See also

- [`README.md`](../README.md) — build surface (Status + Quick Start + Usage)
- [`AGENTS.md`](../AGENTS.md) — repo-local context + boundary rationale + multi-domain architecture
- [`11_domains/`](11_domains/README.md) — Plan 001 multi-domain framework (Phase 2 + Phase 3)
- [`.benchmarks/001_goat_consolidated.md`](../.benchmarks/001_goat_consolidated.md) — measured GOAT gate results
