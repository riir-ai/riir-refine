# 04 — Pruner (L2 syntax validator)

> **Purpose:** validates drafted clippy fixes. Always-on, modelless.
> Uses `syn` for in-process syntax parsing (the fast path). Optional
> `cargo clippy` re-run for truthful lint resolution (Phase 3, in
> [05_clippy_verify](../05_clippy_verify/README.md)).

## Files

| File | Scope |
|---|---|
| [`rust_analyzer_pruner.md`](rust_analyzer_pruner.md) | `RustAnalyzerPruner` + `ValidationResult` |
| [`honesty_contract.md`](honesty_contract.md) | What `parses` / `lint_resolved` actually mean |

## Source

- [`src/pruner.rs`](../../src/pruner.rs)

## See also

- [`../05_clippy_verify/`](../05_clippy_verify/README.md) — Phase 3 truthful lint resolution
- [`../03_drafter/`](../03_drafter/README.md) — L1 proposes the fix; L2 validates it
