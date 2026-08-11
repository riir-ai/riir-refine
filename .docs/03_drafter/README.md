# 03 — Drafter (L1 ternary SIMD matvec)

> **Purpose:** the modelless ternary drafter that proposes clippy fixes
> via a multiplication-free SIMD matvec over frozen direction vectors.
> Zero-alloc hot path. Template fallback when no weights loaded.

## Files

| File | Scope |
|---|---|
| [`clippy_ternary_drafter.md`](clippy_ternary_drafter.md) | `ClippyTernaryDrafter` + `DraftedFix` + `DraftSource` |
| [`embedder.md`](embedder.md) | BLAKE3-deterministic modelless embedder (`embed` / `embed_into`) |

## Source

- [`src/drafter.rs`](../../src/drafter.rs) — `ClippyTernaryDrafter`
- [`src/embedder.rs`](../../src/embedder.rs) — `embed`, `embed_into`, `embed_fix_into`

## See also

- [`../02_corpus/`](../02_corpus/README.md) — L0 provides the fix candidates
- [`../04_pruner/`](../04_pruner/README.md) — L2 validates the drafted fix
