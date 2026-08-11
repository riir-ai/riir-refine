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
   Lint resolution verification via `cargo clippy` re-run is Phase 3 (deferred).
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
```

## Feature Flag Discipline

- `default`: L0 (corpus) + L1 (ternary drafter) + L2 (native rust-analyzer pruner)
- `ruliology_search` (opt-in): L3 Pareto-front candidate enumeration
- `wasm_pruner` (opt-in, deferred): L2 WASM rust-analyzer variant (blocked on rubrc)
- `clippy_lora` (opt-in, deferred): L4 adaptive LoRA fallback (blocked on riir-train integration)

The modelless path (L0-L3) is PRIMARY. LoRA (L4) is the conservative last resort.

## Numbering Discipline

Issue, plan, doc, benchmark, and research numbers are **monotonic and never
reused**. Before creating a new `.issues/` file, read `.issues/.highwater`, use
`value + 1`, write back. Same for `.plans/`, `.docs/`, `.benchmarks/`, `.research/`.

## Branch

`develop` is the working branch. Commit directly on `develop` per the global rule.
