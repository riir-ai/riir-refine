# Research 387 — sccache cache-layer re-distill: the tiered-hit backfill family

**Status:** DISTILLED — pending owner decision (verdict YES B−; batch staged as Plan 355, nominal Batch 343 behind 342)

## Source

mozilla/sccache @ `c1d7204f90a9ed1d88e6a8be39e2b8dab7ecfcd6` (origin/main, 2026-11-08; Apache-2.0). Full clone at `.raw/mozilla-heal/sccache` — **pinned sha is the provenance; the clone is throwaway** (campaign-ephemeral, removed at batch landing). License verified in-tree.

**Provenance chain:** this is a RE-distill of a source the corpus already mined once — Batch 324 (entries_58) took the spawn-path/io family (`pre-exec-disables-posix-spawn`, `mtime-encoded-lru-recency-persistence`, boot-identity/hash-read-once). This pass reads the FILES B324 did not: the multi-level tiering layer (`src/cache/multilevel.rs`) and the LruDiskCache staging path (`src/lru_disk_cache/mod.rs`), read in full this session while the #2968 lane had the clone open. Zero file overlap with B324's mined surface.

**Inspection trigger:** the 2026-11-08 mozilla-org wide survey (Bench 147) found zero SENDABLE perf fixes across seven fresh repos — "their repo too perfect." The owner's redirect: distill the perfection instead. sccache is the deepest-read of the surveyed repos (the #2968 lane required full comprehension of the cache stack).

## Why the corpus wants this family

The corpus's caching family is strong on single-tier recency (LRU, mtime-persistence, admission, eviction ladders) but has ZERO entries on **hierarchical multi-tier propagation** — the laws of moving a hit between tiers. sccache's multi-level layer is a production, Mozilla-shipped instance of exactly that (disk + redis/s3 chains, per the MultiLevel docs), with the three load-bearing mechanisms read and quoted below.

## Ranked candidates

| # | candidate | grade | source | notes |
|---|---|---|---|---|
| 1 | **raw-bytes-with-hit-backfill-once** | **B** | `multilevel.rs` `backfill_raw_hit` (:491-526) + `get` (:571-601) + `disk.rs::get_with_raw` (:126-136) | see law below |
| 2 | **advertised-size-staged-write-reconciles-at-commit** | **B−** | `lru_disk_cache/mod.rs` `prepare_add`/`commit` (:367-409) | see law below |
| 3 | **vanished-entry-tolerant-eviction-panic-else** | **B−** | `lru_disk_cache/mod.rs` `make_space` (:257-288) + upstream issue 2092 referenced in-tree | see law below |
| 4 | per-tier-lockfree-stat-slots (`AtomicLevelStats` + `inc_stat!` macro) | C, expected-drop | `multilevel.rs` :42-103 | "count stats with per-tier AtomicU64 slots" is close to ambient — the corpus's `debug-stat-full-scan-out-of-hot-path` (entries_05) already owns the hot-path-stats axis; the per-TIER keying is a parameterization, not a new law |
| — | KILL: mtime-sorted-walk LRU rebuild | — | `lru_disk_cache/mod.rs` `get_all_files` (:75-95) | **positive instance of `mtime-encoded-lru-recency-persistence` (B324, entries_58)** — the walk+sort-by-mtime feeding the LRU is exactly that rule's boot-time init half. Kill recorded as dedup evidence. |
| — | KILL: per-request mtime re-stat validation (`server.rs` :1409-1417), `box_clone` per request (c.rs :385) | — | | correctness-load-bearing (binary-swap detection) / ~2 allocs per compile — neither a law nor a cost |
| — | KILL: per-request compiler-info memo, two-phase `prepare_add` capacity accounting as PERF | — | | the memo shape is universal (positive instance of the memoization family); the capacity accounting folds INTO candidate 2 |

### Law 1 — raw-bytes-with-hit-backfill-once (B)

When a hierarchical cache takes a hit at a SLOWER tier and faster tiers need the entry, read the entry ONCE in a form both consumers share: the parse and the backfill copy consume the SAME raw bytes (`get_with_raw` returns `(Cache::Hit, Option<Bytes>)`; `CacheRead::from(Cursor::new(data.clone()))` parses a view while the `Bytes` handle rides to the backfill). The write-back is spawned DETACHED (`tokio::spawn`, `multilevel.rs:508`) so the read path never waits on it, and a hit at the FASTEST tier skips backfill entirely (`hit_level == 0 → return`, :492-494) — there is nothing faster to fill. The caller-side split: only levels with a faster neighbor use `get_with_raw` (`get` :571-581 — `idx > 0` gets raw, `idx == 0` plain `get`), so the single-tier deployment never pays the raw-copy arm.

Regimes: multi-tier-with-slower-remote (law applies) vs single-tier (the `idx == 0` half — never request raw). Anti-regime: a tier whose entries are too big to duplicate in memory — then the streaming copy must go disk-to-disk and the same-bytes trick is unavailable (bounded fix-space states this).

**Benchable? No — criterion 1 fails DOMAIN-side:** the win is I/O read-amplification (one physical read serving two consumers) + read-path latency not paying the write-back, neither hostable in the CPU Pareto validator. NotBenchable-by-domain annotation (the B324/B136 standard — decision-content entries are first-class). Criteria 2–4 pass: parse-identity fixtures are bit-identical, deterministic, magnitude story = 2 reads → 1 read on every slower-tier hit.

### Law 2 — advertised-size-staged-write-reconciles-at-commit (B−)

A capacity-bound cache admitting a write whose final size is unknown at open (a stream) must not reserve NOTHING (overshoot evicts wrong victims later) nor BLOCK on a guessed size (deadlock risk, wasted reservation on failure). sccache's shape: reserve on the ADVERTISED size up front (`prepare_add(size)` → `make_space(size)` + `pending_size += size` — the pending list participates in `size()` so concurrent staged writes count against capacity), write to a tempfile OUTSIDE the cache namespace, and at `commit` reconcile to the REAL size (`make_space(real_size.saturating_sub(size))` — only the delta, never re-evicting for bytes already reserved), then `persist` into the namespace and insert into the LRU. Fixed-size writes skip the staging arm entirely (`insert_bytes` — the two-regime split).

Benchable? The accuracy axis (eviction-overshoot count) is hostable in principle but the validator's shape is byte-count/cycle counters, not eviction-quality — NotBenchable-by-domain, suggestion-grade. Magnitude story: reserve-on-advertised bounds peak disk usage to capacity+delta instead of capacity+full-output.

### Law 3 — vanished-entry-tolerant-eviction-panic-else (B−)

An eviction loop over a SHARED directory (multiple daemons, one cache root — sccache's documented deployment) races other owners' evictions: `fs::remove_file` failing `NotFound` means someone else evicted the entry first — TOLERATE and continue (debug log, upstream issue 2092 cited in-tree); ANY other error is our own state corruption — PANIC loud (`unwrap_or_else` with the kind split). The general law: a correctness invariant whose violation may be CAUSED BY A PEER must classify peer-caused failures (tolerate+continue) from self-caused ones (fail loud) instead of flattening both into one error arm — flattening hides real corruption; failing on peers breaks the shared deployment.

Adjacent-distinct from `persistent-cache-teardown-needs-second-witness` (entries_39 — teardown verification, not eviction racing) and from the sec family's TOCTOU entries (path-join/checkout — those gate ACCEPTANCE of attacker paths; THIS classifies Benign concurrent removal during our own eviction). Suggest-only; NotBenchable-by-domain (correctness class).

## Dedup evidence (both vocabularies, zero hits unless noted)

- backfill / write-back / writeback / propagate hit → **zero corpus hits**
- staged add / advertised size / reserve-then-commit → zero (the corpus's `reserve-then-commit-capacity-accounting` is the SCAN-time rule about reserving containers before pushes — same WORDS, different mechanism: container capacity vs cache-capacity accounting; in-row adjudication mandatory at authoring)
- eviction NotFound / peer race → zero (`persistent-cache-teardown-needs-second-witness` read and ruled distinct)
- mtime walk rebuild → **one hit, the KILL above** (B324 positive instance)
- per-tier atomic stats → hits are unrelated (static-str, debug-stat) → candidate 4 thin

## Lane-intel axis

No OPEN workspace lane names hierarchical-cache propagation as a lever. WATCH-class: riir-neuron-db's Warm-tier + shard layers and riir-dapps' ledger DO storage are the natural consumers IF a multi-tier promotion lane ever opens — the corpus entry would then be the design gate the same way B223/B319 serve the Metal lanes. Recorded here; no sibling filing.

## Product-coverage axis

Grows the caching family's MISSING half (hierarchical propagation) on a crate every Rust CI user touches (sccache ~7.9k stars). Follow-up pool for the family, accumulation clause: mozilla/sccache multilevel docs + tests (this note's pool), `lru` crate issue tracker (eviction-race reports), ccache's same-named behaviors (the C ancestor, transcription-eligible), nginx cache proxy loaders (the walk-rebuild at scale).

## Caveats

- Pin `c1d7204` is a fast-moving main (22 commits in the 2 days before 11-07); the batch's T0 re-clones at the pin and byte-verifies every quote (the B340 standard: this note's line numbers were read THIS session, quotes verified once).
- Candidate 3's in-tree issue reference (2092) is upstream's own record — cite both the code and the issue number.
- The clone at `.raw/mozilla-heal/sccache` is shared with the #2968 campaign lane — do NOT delete until that lane closes; the batch may read it but must re-pin quotes at its own T0.

## Yield estimate

2–3 rules landing after ~50% falsification (candidates 1–3; candidate 4 expected drop). All suggestion-grade NotBenchable-by-domain — no score-bench run owed, no freeze-tables (zero clippy_lints delta), release-set binary not owed (corpus-only, the B339/B340 precedent).

## Bottom line

Mine as one batch (Plan 355, nominal Batch 343 — behind the staged 342): lead with **raw-bytes-with-hit-backfill-once**; fixtures original Rust (hierarchical in-memory cache reproductions — NO sccache code pasted); in-row adjudication for the reserve-then-commit name collision. Queue line + plan + this note = the three artifacts, one commit.
