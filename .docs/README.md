# riir-clippy `.docs/` book

> **Purpose:** architectural narrative reference for the clippy-healing
> drafter+pruner pipeline. Numbered folders for sort order, unnumbered
> `.md` files inside each. Each folder's `README.md` is the local index.

This book documents the **modelless-first clippy-healing pipeline** that
distills clippy rules into a ruliology-style search space, drafts fixes
via a ternary SIMD matvec, validates with a `syn`-based syntax pruner,
and refines via Pareto-front candidate search. LoRA (L4) is the
conservative last-resort fallback, not the co-equal path.

## Folder index

| # | Folder | Scope |
|---|---|---|
| 01 | [`01_orientation/`](01_orientation/README.md) | Crate role, sibling layout, boundary rationale, build commands |
| 02 | [`02_corpus/`](02_corpus/README.md) | L0 rule distillation: `ClippyCorpus` + BLAKE3 commitment + seed corpus |
| 03 | [`03_drafter/`](03_drafter/README.md) | L1 ternary SIMD matvec drafter + BLAKE3 modelless embedder |
| 04 | [`04_pruner/`](04_pruner/README.md) | L2 `syn`-based syntax validator |
| 05 | [`05_clippy_verify/`](05_clippy_verify/README.md) | Phase 3 lint resolution via real `cargo clippy` (opt-in) |
| 06 | [`06_ruliology/`](06_ruliology/README.md) | L3 Pareto-front candidate search (opt-in) |
| 07 | [`07_examples/`](07_examples/README.md) | The 3 in-tree examples |
| 08 | [`08_benchmarks/`](08_benchmarks/README.md) | GOAT gate methodology + measured results |
| 09 | [`09_lessons/`](09_lessons/README.md) | Canonical patterns + honest POC caveats |
| 10 | [`10_self_evolve/`](10_self_evolve/README.md) | Issue 004: latent-first self-evolution loop (`HealPipeline` + memory + evolution) |

## Pipeline overview

```text
Clippy corpus (BLAKE3-committed freeze/thaw)       ← L0 (02_corpus)
  (lint_name, buggy_code, fixed_code, lint_group) tuples
         │
         ▼
L1 — MODELLESS DRAFTER (ternary SIMD matvec)        ← L1 (03_drafter)
  Frozen clippy-fix direction vectors (.bits files)
  BLAKE3-deterministic embedding (lint_name, buggy_code) → [f32; 8]
  Template fallback when no weights loaded
  Zero-alloc hot path
         │
         ▼
L2 — VALIDATOR (always-on)                          ← L2 (04_pruner)
  `syn` in-process parse check (fast, modelless)
  Phase 3: `cargo clippy` re-run for lint resolution (05_clippy_verify)
         │
         ▼
L3 — RULIOLOGY SEARCH (opt-in)                      ← L3 (06_ruliology)
  Pareto-front of (lint_satisfied, AST delta size)
  Picks minimum-complexity fix among Pareto-optimal set
         │
         ▼
  [ validated clippy fix ]
```

## Resolved validation

- **Issue 008** (resolved + removed per noise-reduction) — real-world
  validation of `self_evolve` via `cargo clippy` feedback (G6 corpus
  truthfulness + G7 self-evolution with real clippy). Both gates PASS.
  See `.benchmarks/008_real_world_validation.md`.

## See also

- [`README.md`](../README.md) — build surface (Status + Quick Start + Usage)
- [`AGENTS.md`](../AGENTS.md) — repo-local context + boundary rationale
- [`.benchmarks/001_goat_consolidated.md`](../.benchmarks/001_goat_consolidated.md) — measured GOAT gate results
