# Plan 034 — Batch 76: the Meilisearch LMDB/vector-store distill (rust_perf 83→85)

> **Source:** "Patching LMDB: how we made Meilisearch's vector store 3× faster"
> (Clément Renault, Meilisearch CTO, meilisearch.com/blog, 2026-03-18; 16-min
> engineering-deep article, production-verified on a 19M-embedding customer
> trace). Found via the idle-loop online perf search (queue item 11).
> **Pool:** the Qdrant Gridstore article (qdrant.tech, 2025-02-05) read in the
> same pass — assessed **C+ for rule-mining** (an architecture piece: tracker/
> mask/gaps layers, model testing, WAL idempotence; its one rule-shaped
> candidate — HashMap with dense sequential keys → Vec direct index — KILLED at
> dedup as adjacent to `o1-lookup` (the Vec↔HashMap neighbor) +
> `dedupe-hashset-vs-vec` (the dense-int-keys → Vec marking neighbor); the
> gaps-layer summary-index is data-structure design, not a bounded fix-space).

## Why these two rules

The article's headline (patched LMDB nested read txns) is an upstream C-API
change — out of scope. The two mechanism wins that generalize to OUR corpus:

1. **The debug-stat full scan** (559s → 2–3s, ~200× on the step): a mean-degree
   sanity statistic computed EAGERLY in the per-insert path full-scanned the
   vector store on every insert batch — O(collection) per op, growing with
   total size. Removed → O(newly-inserted). The decision content: diagnostics
   are a hidden O(n) tax in hot paths; maintain the aggregate or gate it.
   NOT covered by: `sort-key-cache-precompute-out-of-lock` (comparator
   recompute under lock — different trigger), `filter-count-to-any`
   (predicate short-circuit vs scan — different mechanism), kernel_opt's
   `incremental-centroid-running-sum` (cross-domain, GPU clustering — this is
   the rust_perf storage-iteration form).
2. **Eager pointer-collect vs lazy fetch** (the ImmutableItems/ImmutableLinks
   removal): pre-materializing a HashMap of pointers over ALL entries (full
   scan + cache pollution) to serve a SUBSET of random accesses, replaced by
   on-demand reads through a direct path. Only-necessary pages load; the scan's
   cache eviction of hot pages disappears. NOT covered by: `collect-avoidance`
   (intermediate-Vec allocation, not working-set), `encoded-layout-borrowed-view`
   (representation re-expansion, not index materialization).

## Tasks

- [x] T1 Plan file (this)
- [x] T2 Corpus entries ×2 (`debug-stat-full-scan-out-of-hot-path` complexity,
      `eager-pointer-collect-lazy-fetch` caching) + bench_spec fns, suggestion-only
      (type-blind, no proposer — the B73/B75 precedent) — verified 2026-08-28: 9 rule-id refs in `src/domains/rust_perf/corpus.rs`
- [x] T3 Bench arms in bench.rs (working-set-sized data for rule 2; per-insert
      loop for rule 1) + honest annotation if regime-gated — verdict-stable ×2 recorded in the research note (179×/1884×/6294× re-run)
- [x] T4 Count pins updated everywhere they break (failing-run discovery,
      B75's list: AGENTS header, corpus count comments, serde/drafter hidden
      pins, annotation counts) — AGENTS.md Batch 76 row + "28 of 85" annotation count landed
- [x] T5 GOAT: G1 parse + G2 lib tests (default + rust_perf_bench) + G3 clippy 0
      both states `--all-targets` + retrieval floors HELD + verdict-stable ×2
      + the new benches report measured verdicts — 696/0 default + 714/0 bench-state + full CI guard ALL LAYERS (AGENTS.md record)
- [x] T6 Research note `.research/090_*` (born-complete RECORD per convention) — `090_meilisearch_lmdb_vector_store_distill_batch76.md`
- [x] T7 Docs: AGENTS.md Batch 76 row + corpus counts + `.docs/11_domains/rust_perf.md`
      + `.distill/001_mining_queue_snapshot.md` queue state — both doc rows verified 2026-08-28
- [x] T8 Commit + push (`feat: batch 76: ...`)

> **Bookkeeping note (2026-08-28, idle-loop item-4 pass):** T1–T7 landed with the
> batch on 2026-08-27 but only T8 was ticked at the time; the boxes above were
> ticked retroactively against verified evidence (corpus grep, research note,
> AGENTS.md + rust_perf.md rows). No work remained.
