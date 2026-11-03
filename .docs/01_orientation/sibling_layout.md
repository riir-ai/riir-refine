# Sibling-repo layout

Path dependencies assume this on-disk layout:

```
/git/riir-refine          ← this repo (the rulewright pipeline)
/git/katgpt-rs            ← katgpt-core (NON-OPTIONAL: ConstraintPruner trait + ternary matvec)
/git/riir-ai              ← OPTIONAL dep (feature-gated arms only — see below; default build: NOT a dep)
/git/riir-train           ← OPTIONAL dep (only for L4 LoRA fallback, deferred)
/git/riir-chain           ← NOT a dep
/git/riir-neuron-db       ← transitive only (via riir-rag under `latent_retrieval`)
/git/riir-game-sdk        ← NOT a dep
```

## Dependency graph

```text
katgpt-rs (katgpt-core)
    ↑
    │ path dep (non-optional)
    │
riir-refine
    │
    ├── (optional) riir-ai crates, per feature:
    │     • ternary_inference  → riir-engine + riir-gpu (Bench 010 inference arm)
    │     • latent_retrieval   → riir-rag (Plan 005 retrieval substrate)
    │
    └── (optional, deferred) riir-train — only if L4 clippy_lora ships
```

**Default build: one code-level dependency** — `katgpt-core`. This provides:

- `katgpt_core::TernaryWeights` — the ternary weight container (rows ×
  cols × i8 quantized bits)
- `katgpt_core::simd_ternary_matvec` — the multiplication-free SIMD
  matvec (zero-alloc hot path)
- `katgpt_core::ConstraintPruner` trait — the pruner contract (L2
  `RustAnalyzerPruner` is conceptually aligned, though it implements
  its own `validate` API rather than the trait — see
  [04_pruner](../04_pruner/README.md))

## What is NOT a dependency

- **Game crates** (`riir-games*`, `riir-game-sdk`) — game domain. Wrong
  domain, unconditionally. This is the load-bearing part of the boundary
  rule.
- **riir-chain** — chain consensus. Wrong domain.
- **riir-train** — training. Only relevant for L4 LoRA fallback, which
  is deferred.

**The riir-ai nuance (updated 2026-08-14, Plan 005):** the boundary rule
is about **game-domain coupling**, not the riir-ai workspace per se. The
*default* build takes zero riir-ai deps (patterns are reimplemented
locally — drafter+pruner+corpus from `quest_grammar`, embedder from
`riir-rag`, dot+sigmoid from `riir-agents`). Two opt-in feature-gated
arms consume specific **non-game** sibling crates where reimplementation
would be absurd duplication:

- `ternary_inference` (Bench 010) → `riir-engine` + `riir-gpu`, to
  benchmark against our own inference stack.
- `latent_retrieval` (Plan 005) → `riir-rag` (`AstChunker` +
  `ModellessEmbedder` + `NeuronDbRetriever`), the retrieval substrate
  behind `cargo refine --suggest`.

No game crate is reachable from any feature combination.

## If you move this repo

Update the path entry in `Cargo.toml`:

```toml
katgpt-core = { path = "../katgpt-rs/crates/katgpt-core" }
```

## See also

- [`boundary_rationale.md`](boundary_rationale.md) — why the strict
  one-dep boundary
