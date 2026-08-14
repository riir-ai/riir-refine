# 03 — Drafter (L1 ternary SIMD matvec + latent retrieval)

> **Purpose:** the modelless ternary drafter that proposes clippy fixes
> via a multiplication-free SIMD matvec over frozen direction vectors.
> Zero-alloc hot path. Template fallback when no weights loaded. Also
> hosts the Plan 005 latent retrieval draft layer (`src/draft/`) — the
> span-level `--suggest` pipeline that retrieves ranked rules for real
> code spans (zero proposer code).

## Files

| File | Scope |
|---|---|
| [`clippy_ternary_drafter.md`](clippy_ternary_drafter.md) | `ClippyTernaryDrafter` + `DraftedFix` + `DraftSource` |
| [`embedder.md`](embedder.md) | BLAKE3-deterministic modelless embedder (`embed` / `embed_into`) |
| [`latent_suggest.md`](latent_suggest.md) | Plan 005 span-level latent retrieval (`--suggest` MVP): `RuleIndex` + `LatentMatcher` + domain centroids |
| [`latent_suggest.md`](latent_suggest.md) | Plan 005 latent retrieval: `RuleIndex` + `DomainRouter` + `LatentMatcher` (`--suggest` MVP, GOAT G1–G4 PASS, Bench 018) |

## Source

- [`src/drafter.rs`](../../src/drafter.rs) — `ClippyTernaryDrafter`
- [`src/embedder.rs`](../../src/embedder.rs) — `embed`, `embed_into`, `embed_fix_into`
- [`src/draft/`](../../src/draft/) — Plan 005 latent retrieval layer
  (`corpus_index.rs` + `domain_router.rs` + `latent_matcher.rs`, feature
  `latent_retrieval`)

## See also

- [`../02_corpus/`](../02_corpus/README.md) — L0 provides the fix candidates
- [`../04_pruner/`](../04_pruner/README.md) — L2 validates the drafted fix
- [`../07_examples/cargo_heal.md`](../07_examples/cargo_heal.md) — the `cargo-heal` bin wrapping the latent layer
