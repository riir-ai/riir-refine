# AGENTS.md — riir-clippy

The global `~/.agents/` rules apply; this file documents repo-local context.

## Role

**Clippy-healing drafter+pruner pipeline** — a modelless-first developer tool
that heals `cargo clippy` warnings. This is a **developer tooling repo**, NOT a
game concern, NOT a training repo, NOT a chain repo.

The pipeline:
1. **L0 — Rule distillation**: each clippy lint → named rule + bounded fix-space
2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec, zero-alloc (Issue 001, DONE)
3. **L2 — Validator**: `syn`-based syntax parse check (always-on, modelless — Issue 002, DONE).
   Lint resolution verification via `cargo clippy` re-run is Phase 3 (Issue 003,
   feature-gated `clippy_verify`, DONE).
4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration
5. **L4 — LoRA fallback** (deferred): adaptive learning, last resort

## Why this is a separate repo (not in riir-ai)

The clippy healer borrows the **drafter+pruner+corpus pattern** from
`riir-games-quest/quest_grammar` (TernaryDraftModel + SealQuestCorpus +
ConstraintPruner), but has **zero game-domain coupling**:

- No dependency on `riir-engine`, `riir-games`, or any game crate
- No dependency on `riir-chain`, `riir-neuron-db`
- Only public dep: `katgpt-core` (ConstraintPruner trait + ternary SIMD matvec)
- The `TernaryDraftModel` pattern is reimplemented locally (~200 LOC), not
  imported cross-workspace

Putting it in riir-ai would violate the domain boundary: riir-ai is for **game
runtime + engine**, not developer tools. The prior rejection of extracting
riir-games from riir-ai (Proposal 017 era) was correct because riir-games has
heavy bidirectional coupling with riir-engine. Clippy healing has no such
coupling — it's a clean, self-contained developer tool.

## Sibling-Repo Layout

```
/git/riir-clippy          ← this repo (clippy-healing pipeline)
/git/katgpt-rs            ← katgpt-core (NON-OPTIONAL: ConstraintPruner trait + ternary matvec)
/git/riir-ai              ← NOT a dep (game runtime — wrong domain)
/git/riir-train           ← OPTIONAL dep (only for L4 LoRA fallback, deferred)
/git/riir-chain           ← NOT a dep
/git/riir-neuron-db       ← NOT a dep
```

Path deps in `Cargo.toml` assume the standard `/git/` sibling layout.

## Build Commands

```bash
# Default (modelless path only)
cargo check
cargo clippy --all-targets
cargo test --lib

# With ruliology candidate search (L3)
cargo check --features ruliology_search

# All features
cargo check --all-features

# Full CI guard (5 layers: no-dep / default / per-feature / all-features / clippy+tests)
./scripts/ci_feature_guard.sh

# Re-run GOAT benches (see .benchmarks/001_goat_consolidated.md for measured results)
cargo bench --bench bench_001_l1_matvec
cargo bench --bench bench_002_l2_pruner_syn
cargo run --example ruliology_search_demo --features ruliology_search --release
```

## Feature Flag Discipline

- `default`: L0 (corpus) + L1 (ternary drafter) + L2 (native rust-analyzer pruner)
- `clippy_verify` (opt-in): Phase 3 lint resolution verification via real `cargo clippy` re-run. Slow (~130ms/verify), requires cargo+clippy on PATH.
- `ruliology_search` (opt-in): L3 Pareto-front candidate search. Reuses the algorithm pattern from `katgpt-ruliology` (Bench 572) without a code-level dep. Runs once per lint invocation (~400µs/10 candidates), not per-tick.
- `self_evolve` (opt-in, Issue 004): latent-first self-evolution loop — `LatentFixMemory` (3-axis retrieval over fix trajectories) + `LatentSkillEvolution` (EMA direction-vector update). GOAT G1-G5 ALL PASS (G5: 70pp healing rate improvement in controlled fixture). Borrows `dot+sigmoid` from riir-agents + 3-axis retrieval from riir-rag, reimplemented locally (~730 LOC, boundary-compliant). Stays opt-in until validated against real cargo clippy feedback (Phase 3 `clippy_verify` integration).
- `wasm_pruner` (opt-in, deferred): L2 WASM rust-analyzer variant (blocked on rubrc)
- `clippy_lora` (opt-in, deferred): L4 adaptive LoRA fallback (blocked on riir-train integration)

The modelless path (L0-L3) is PRIMARY. LoRA (L4) is the conservative last resort.
The self-evolution layer (Issue 004) is additive — it composes on top of L0-L2
without modifying the frozen pipeline.

## Documentation Shape

This repo ships a **10-folder numbered `.docs/` book** (opened 2026-08-11,
Issue 007), matching the `riir-ai/.docs/` / `riir-neuron-db/.docs/` /
`riir-game-sdk/.docs/` format: numbered folders for sort order, unnumbered
`.md` files inside, each folder has a `README.md` index with a purpose
blockquote + file table + "See also" cross-refs.

| Folder | Scope |
|---|---|
| `01_orientation/` | Crate role, sibling layout, boundary rationale, build commands, numbering |
| `02_corpus/` | L0 rule distillation: `ClippyCorpus` + BLAKE3 commitment + seed corpus |
| `03_drafter/` | L1 ternary SIMD matvec: `ClippyTernaryDrafter` + BLAKE3 `embedder` |
| `04_pruner/` | L2 syn validator: `RustAnalyzerPruner` + `ValidationResult` + honesty contract |
| `05_clippy_verify/` | Phase 3 lint resolution: `ClippyVerifier` (opt-in `clippy_verify`) |
| `06_ruliology/` | L3 Pareto-front: `RuliologySearch` + `pareto_front` + `ast_delta_size` (opt-in) |
| `07_examples/` | The 3 in-tree examples (`heal_demo`, `clippy_verify_demo`, `ruliology_search_demo`) |
| `08_benchmarks/` | GOAT gate methodology + measured results |
| `09_lessons/` | Canonical patterns: borrow-without-dep, modelless-first, freeze/thaw, honest POC caveats, sigmoid-not-softmax |
| `10_self_evolve/` | Issue 004: latent-first self-evolution loop (`HealPipeline` + `LatentFixMemory` + `LatentSkillEvolution`, opt-in `self_evolve`) |

The top-level `.docs/README.md` is the index. `README.md` (repo root) remains
the build surface (Status + Quick Start + Usage); `AGENTS.md` (this file)
remains the authoritative repo-local context.

For `.docs/`: files inside numbered folders have **no number prefix**.
Filenames are bare slugs (e.g. `crate_role.md`). The folder number provides
sort order; the filename provides identity. Add a new doc by dropping
`slug.md` in the right group folder and adding one line to that folder's
`README.md` index table.

## Numbering Discipline

Issue, plan, doc, benchmark, and research numbers are **monotonic and never
reused**. Before creating a new `.issues/` file, read `.issues/.highwater`, use
`value + 1`, write back. Same for `.plans/`, `.docs/`, `.benchmarks/`, `.research/`.

## Branch

`develop` is the working branch. Commit directly on `develop` per the global rule.
