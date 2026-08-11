# 02 — Corpus (L0 rule distillation)

> **Purpose:** the BLAKE3-committed corpus of clippy lint fix examples.
> Each lint is distilled into a named rule with a bounded fix-space. The
> corpus is the foundation (L0) of the clippy-healing pipeline.

## Files

| File | Scope |
|---|---|
| [`clippy_corpus.md`](clippy_corpus.md) | `ClippyCorpus` + `ClippyFixEntry` + BLAKE3 commitment + freeze/thaw |
| [`seed_corpus.md`](seed_corpus.md) | `build_seed_corpus()` — the 10 built-in lint examples |

## Source

- [`src/corpus.rs`](../../src/corpus.rs) — `ClippyCorpus`, `ClippyFixEntry`
- [`src/seed_corpus.rs`](../../src/seed_corpus.rs) — `build_seed_corpus()`

## See also

- [`../03_drafter/`](../03_drafter/README.md) — L1 consumes corpus entries
- [`../09_lessons/freeze_thaw_pattern.md`](../09_lessons/freeze_thaw_pattern.md) — the freeze/thaw convention
