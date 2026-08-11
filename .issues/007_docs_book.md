# Issue 007 — `.docs/` book (sibling-repo parity)

**Filed:** 2026-08-11
**Status:** RESOLVED

## Problem

Every sibling repo in the 7-repo stack ships a numbered `.docs/` book:
`riir-ai` (10 folders), `riir-neuron-db` (10 folders), `riir-game-sdk`
(9 folders), `riir-chain` (referenced in AGENTS.md).

`riir-clippy`'s `.docs/` folder was **empty**. This is a hygiene gap: the
crate has 7 source modules + 3 examples + 2 benches + 5 feature flags,
but the architectural narrative lived only in `README.md` + `AGENTS.md` +
scattered module doc-comments.

## Scope

Created a numbered `.docs/` book following the sibling-repo format:
- Numbered folders for sort order
- Unnumbered `.md` files inside (filenames are bare slugs)
- Each folder has a `README.md` index with a purpose blockquote + file table

## Structure (10 folders shipped)

| Folder | Scope | Files |
|---|---|---|
| `01_orientation/` | Crate role, sibling layout, boundary rationale, build commands, numbering | 5 docs + README |
| `02_corpus/` | L0 rule distillation: `ClippyCorpus` + BLAKE3 commitment + seed corpus | 2 docs + README |
| `03_drafter/` | L1 ternary SIMD matvec drafter + BLAKE3 modelless embedder | 2 docs + README |
| `04_pruner/` | L2 `syn`-based syntax validator + honesty contract | 2 docs + README |
| `05_clippy_verify/` | Phase 3 lint resolution via real `cargo clippy` (opt-in) | 1 doc + README |
| `06_ruliology/` | L3 Pareto-front candidate search + ast_delta_size | 3 docs + README |
| `07_examples/` | The 3 in-tree examples | 3 docs + README |
| `08_benchmarks/` | GOAT gate methodology + measured results | 2 docs + README |
| `09_lessons/` | Canonical patterns + honest POC caveats | 5 docs + README |
| `10_self_evolve/` | Issue 004: `HealPipeline` + memory + evolution (committed mid-task) | 3 docs + README |

**`10_self_evolve/` added mid-task:** the sibling agent committed Issue 004
(`bf342f1`) while this issue was in progress. The `self_evolve` module is
now shipped code, so it gets a dedicated folder. Follows the riir-neuron-db
precedent of appending new layers at the end (their `10_local_kv/`).

## Tasks

- [x] T1: Create `.docs/README.md` (top-level index)
- [x] T2: Create `01_orientation/` (role, layout, boundary, build, numbering)
- [x] T3: Create `02_corpus/` (L0 substrate)
- [x] T4: Create `03_drafter/` (L1 substrate + embedder)
- [x] T5: Create `04_pruner/` (L2 substrate)
- [x] T6: Create `05_clippy_verify/` (Phase 3 substrate)
- [x] T7: Create `06_ruliology/` (L3 substrate)
- [x] T8: Create `07_examples/` (the 3 examples)
- [x] T9: Create `08_benchmarks/` (GOAT methodology)
- [x] T10: Create `09_lessons/` (canonical patterns)
- [x] T11: Update `AGENTS.md` (document the `.docs/` book format)
- [x] T12: Verify all source claims against actual code (no fabrication)

## Non-goals

- Documenting Issue 004 (self_evolve) — ships when that work commits
- A `.plans/` file — this is a hygiene task, not a multi-phase plan
- Restructuring `src/` — docs only, zero code changes
