# Sibling-repo layout

Path dependencies assume this on-disk layout:

```
/git/riir-clippy          ← this repo (clippy-healing pipeline)
/git/katgpt-rs            ← katgpt-core (NON-OPTIONAL: ConstraintPruner trait + ternary matvec)
/git/riir-ai              ← NOT a dep (game runtime — wrong domain)
/git/riir-train           ← OPTIONAL dep (only for L4 LoRA fallback, deferred)
/git/riir-chain           ← NOT a dep
/git/riir-neuron-db       ← NOT a dep
/git/riir-game-sdk        ← NOT a dep
```

## Dependency graph

```text
katgpt-rs (katgpt-core)
    ↑
    │ path dep (non-optional)
    │
riir-clippy
    │
    └── (optional, deferred) riir-train — only if L4 clippy_lora ships
```

**One code-level dependency**: `katgpt-core`. This provides:

- `katgpt_core::TernaryWeights` — the ternary weight container (rows ×
  cols × i8 quantized bits)
- `katgpt_core::simd_ternary_matvec` — the multiplication-free SIMD
  matvec (zero-alloc hot path)
- `katgpt_core::ConstraintPruner` trait — the pruner contract (L2
  `RustAnalyzerPruner` is conceptually aligned, though it implements
  its own `validate` API rather than the trait — see
  [04_pruner](../04_pruner/README.md))

## What is NOT a dependency

- **riir-ai** — game runtime. Wrong domain. The clippy healer borrows
  *patterns* (drafter+pruner+corpus from `quest_grammar`, embedder from
  `riir-rag`, dot+sigmoid from `riir-agents`) but reimplements them
  locally because it cannot depend on riir-ai (boundary rule).
- **riir-chain** — chain consensus. Wrong domain.
- **riir-neuron-db** — storage. Wrong domain.
- **riir-game-sdk** — game SDK. Wrong domain.
- **riir-train** — training. Only relevant for L4 LoRA fallback, which
  is deferred.

## If you move this repo

Update the path entry in `Cargo.toml`:

```toml
katgpt-core = { path = "../katgpt-rs/crates/katgpt-core" }
```

## See also

- [`boundary_rationale.md`](boundary_rationale.md) — why the strict
  one-dep boundary
