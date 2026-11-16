# 411 — Distill verdict: hydra-db/hydradb (object-store graph DB)

**Status:** DISTILLED + BOTH TRANCHES LANDED (Batch 364 / Plan 379 first tranche 2026-11-16; Batch 365 / Plan 379 T3 SECOND storage tranche 2026-11-16: candidates 9/10/11/3/5 + #6 of the table below — R3 `existential-witness-early-exit-scan` BENCHABLE GOAT PASS ParetoDominant 3/0/0 two-run stable (5.98x/5.70x/3.78x then 6.17x/5.85x/3.76x, `.benchmarks/155_batch365_goat_cells.md`), R9 `wal-watermark-pinned-before-snapshot` + R10 `tail-walk-cost-cap-declines-to-rebuild` (the two-sided regime carried: the xlog no-gate counter-example in-rule) + R11 `frozen-artifact-plus-delta-tail-overlay` landed NotBenchable-by-domain, R5 landed as `pending-claim-registry-take-consumes-drop-clears` NotBenchable-by-model, R6 `one-shared-disk-cache-not-even-split` NotBenchable-by-domain (the cache-QUALITY ordering fixture-pinned); CITE-CORRECTION row 5: the table cited `graph-slate/write.rs:80-87` with a bloom-negative clause — the verified site at that cite (src/shard/write.rs:74-103 at the pin) is the PENDING WRITE-BACK SWEEP REGISTRY take/Drop; the lifetime-scoped no-invalidation law IS verified there, the bloom clause is NOT verifiable at the cite and was dropped from the entry; the rule landed under the mechanism's own name; rust_perf 347 -> 353, pins 353/15/138). Was: DISTILLED + FIRST TRANCHE LANDED (Batch 364 / Plan 379, 2026-11-16: candidates 1, 2, 7, 8, 4 of the table below — R1 `degree-gated-probe-with-doubling-retry` GOAT PASS ParetoDominant 3/0/0 two-run stable, R2 `bfs-depth-epoch-marker-dedup` FAILED the bar (Tie at every size) DEMOTED to NotBenchable-by-measurement per the ungoat law, 7/8 the honesty pair, 4 `derived-index-write-touches-only-changed-keys`; the SECOND storage tranche (candidates 9/10/11/3/5 + #6) DEFERRED to the follow-up batch; `count-before-fault-injection` (the face candidate, fault_store.rs:186) an honest T0 DROP — no fault_store.rs exists at the pin; candidate 14 adjudicated as the consume_budget rider on adaptive-yield-consecutive-ready; GOAT record: `.benchmarks/154_batch364_goat_cells.md`; the cite corrections: 'graph-slate/write.rs' = src/shard/write.rs, 'graph-slate/config.rs' = crates/graph-slate/src/core/config.rs, 'graph-slate/state.rs' = crates/graph-slate/src/core/state.rs — all content byte-exact). Was: DISTILLED — pending owner decision (YES B+ corpus; batch staged as nominal Batch 365 / Plan 379)

## Source

- URL: https://github.com/hydra-db/hydradb — object-store-native distributed graph DB (SlateDB durable storage, snapshot-consistent reads = compiled CSC generation + visible WAL-tail overlay, atomic object-pointer index publication, CAS-lease writer fencing, SuiteSparse GraphBLAS + pure-Rust CSC kernel, Neo4j Bolt + HTTPS planes).
- Pin: `916637477f3f34766b4905b270bd4292109d77e6` (tag v0.4.0, 2026-11-06; 13.1k stars, 84 commits, fresh open-source drop).
- License: **Elastic License 2.0 (ELv2)** — source-available, not OSI. Corpus-safe under the standing transcription law (fixtures are always our original reproductions; never paste source), but the license note belongs in every entry's provenance row.
- Inspection: 2026-11-15, M3, two parallel subagent lanes (A: graph-slate/object-publication/buddy-wal/src-engine/src-core; B: sparse_kernel/shard/pg-graph-csr/client/placement) + coordinator spot-verification of 7 critical quotes at the pin (all byte-exact) + coordinator dedup greps.
- Completeness: ZERO `todo!`/`unimplemented!` in non-test code across both lanes; v0.4.0 tagged, Keep-a-Changelog, doc-comment density exceptional — most constants carry their own regime justification and measured provenance (prod pathology numbers quoted in-file: 1,566 reads/30 s scan, 48 GiB/64-scope cache split, 17–38 s prepare times, 54 fences/s starving flushes).

## Domain routing

`rust_perf` — io / caching / concurrency / measurement-honesty categories + 1 tokio-family candidate. The tempting WRONG domains: `kernel_opt` (sparse_kernel is CPU sparse traversal, not GPU), `sec` (writer fencing looks security-shaped but the fix-space is concurrency/storage semantics, not a security class). Everything here is the source's **landed decision content** — no outbound-PR leads (P113 trivially "present" throughout; this repo IS the upstream).

## GOAT test

Lead: **`degree-gated-probe-with-doubling-retry`** (src/shard/query.rs:8634–8646, verified). Benchable: integer-exact (probes + neighbor-reads counted), small fixture (probe-admission function over a synthetic degree stream with injected overflow watermarks), magnitude provable (failed probes cost ≤ ~2× the largest degree, vs per-source full cost; hubs still get their probe), deterministic. Co-lead: `bfs-depth-epoch-marker-dedup` (src/sparse_kernel/graphblas.rs:370–384, verified) — N BFS levels cost one `vec![0u8; V]` + zero memsets instead of N memset passes; integer-exact (allocations + memset bytes), magnitude scales hops × V. Four further CPU-benchable candidates in the ranked table.

## Ranked candidates (post-dedup)

| # | Rule | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `degree-gated-probe-with-doubling-retry` | query.rs:8634–8646 | B+ (GOAT lead) | scan-vs-lookup crossover at degree ≥ 64 + failed speculative access-path switch retried only at 2× the failure watermark. DISTINCT (no corpus owner; batch-retry-ladder is transport retries). Benchable. |
| 2 | `bfs-depth-epoch-marker-dedup` | graphblas.rs:370–384 | B+ | monotone epoch marker makes per-iteration dedup-array clearing unnecessary; u8 marker w/ generation-overflow reset path. DISTINCT vs `generation-counter-stale-timer-invalidation` (timer staleness) + entries_03 memset rule (single-buffer alloc). Benchable. |
| 3 | `existential-witness-early-exit-scan` | write.rs:3157–3166 | B | ∃ predicate reads to first witness; ∀ predicate reads all — predicate arity decides the scan shape. DISTINCT (kinship with sorted-input-previous-row "stop early" but different axis). Benchable. |
| 4 | `derived-index-write-touches-only-changed-keys` | graph-slate/write.rs:182–190 | B+ | unchanged-value re-puts pay full LSM amplification (WAL→memtable→L0→compactions); diff `previous` vs `next`, write only deltas. DISTINCT. Benchable (bytes-written w/ declared amplification constant). |
| 5 | `catalog-memo-scoped-to-the-snapshot-no-invalidation` | graph-slate/write.rs:80–87 | B | memo lifetime = snapshot/txn lifetime ⇒ cross-txn correctness by construction, invalidation machinery deleted; negatives memoized (bloom-filtered point miss per name). ADJACENT-DISTINCT vs `batch-local-memoization-negative-cache-tombstone` (that owns the negative-tombstone mechanism; this owns the lifetime-equals-consistency-scope law). Benchable. |
| 6 | `one-shared-disk-cache-not-even-split` | engine/shared_disk_cache.rs:3–12 | B | even budget split over max-concurrency slots starves the skewed tenant while the budget idles (48 GiB/64 scopes measured prod); one pool + one eviction order, prefix-keyed tenancy. ADJACENT-DISTINCT vs `cache-share-capped-at-measured-distinct-working-set` (per-tenant caps — opposite mechanism, same budget family). Benchable (trace-driven LRU sim). |
| 7 | `spawn-blocking-queue-wait-split-from-compute` | query.rs:157–170 | B | aggregate latency over spawned work must split queue-wait from execution or a saturated pool presents contention as kernel slowness. Measurement-honesty family. DISTINCT. NotBenchable-by-domain. |
| 8 | `latency-histogram-split-on-structural-populations` | client/service.rs:865–997 | B+ (honesty co-lead) | one sum fed two structurally different populations (bimodal mean "names neither"); split at the record site on the discriminator the code already holds + label-from-data-not-export + gap-not-guess ("a guess in a metric is worse than a gap") + derived totals recomputed. DISTINCT. NotBenchable-by-domain. |
| 9 | `wal-watermark-pinned-before-snapshot` | engine/index_store.rs:114–120 | B+ (ndb lead) | read-ORDER invariant: capture the durable WAL id BEFORE pinning the snapshot or every tail walk is unbounded (worst case whole WAL history, partially GC'd). DISTINCT. NotBenchable-by-domain. |
| 10 | `tail-walk-cost-cap-declines-to-rebuild` | graph-slate/config.rs:113–121 | B | incremental path whose input cost grows on a different axis than its output (activity vs delta) needs a hard cap whose action is DECLINE-to-full-rebuild (rebuild + snapshot adjacency cheaper than the refused walk). Regime two-sided IN-SOURCE: the xlog delta path deliberately has no gate (index_store.rs:219–223) — the axis decides. DISTINCT. NotBenchable-by-domain. |
| 11 | `frozen-artifact-plus-delta-tail-overlay` | query.rs:7571–7657 | B | cache the immutable compiled artifact at its base epoch, compose freshness by bounded WAL-tail overlay; the movable pointer is re-read every call. Direct ndb freeze/thaw + WalBackend analog. DISTINCT. NotBenchable-by-domain. |
| 12 | `cheap-proof-existence-probe-with-gc-window` | graph-slate/state.rs:1530–1543 | B− | one GET of `n+1` proves "nothing newer" only while a periodic full listing re-establishes the window and GC never deletes younger than a pinned min-age (`interval < min_age` load-bearing). DISTINCT. NotBenchable. |
| 13 | `boot-probe-gates-write-authority-fail-closed` | core/create_only.rs:3–140 | B− | race-probe the remote CAS primitive at boot (N concurrent create-only PUTs, one winner + echo); fail ⇒ keep reads, refuse writes, /readyz reads the gate. ADJACENT-DISTINCT vs `write-vectored-capability-flag` (compile-time flag vs runtime probe + authority degradation). NotBenchable. |
| 14 | `consume-budget-yield-before-ready-await` | query.rs:8406–8410, verified | B− ⚠ | plain `.await` never yields when the future is already ready (cache-hit scans); `tokio::task::consume_budget()` per iteration/chunk. ⚠ MINE-TIME ADJACENCY: `adaptive-yield-consecutive-ready` was NOT in lane B's adjudication list — close kin (adaptive K-consecutive vs fixed budget); adjudicate before landing. |
| 15 | `warm-guarded-reads-before-exclusive-permit` | write.rs:6–23 | B− | hold the exclusive writer permit for compute, not IO; pre-warm reads under TWO gates (row cap matched to cache size + derived read budget, one row fans to ~7 keys); seeks charge budget even when empty. DISTINCT. NotBenchable. |
| 16 | `final-state-changelog-canonical-overlay` | graph-slate/keys.rs:60–64 + index_store.rs:332–336 | B− | changelog-as-overlay stores post-state (order-independent replay) + canonical reconstruction because the generation id is content-addressed (equal states must encode byte-identically or the identity oracle breaks). Correctness law. |
| 17 | `remote-clock-remaining-cached-by-fingerprint` | writer_lease.rs:537–586 | C+ | server-clock oracle (probe-write + HEAD of a private object) + remaining-time cached keyed on the lease's FULL fingerprint + timestamp-resolution guard. DISTINCT. |

**Faces (fold onto existing rules, count-neutral):** histogram derived-count-never-stored + torn-pair bound + bounds-pinned-to-thresholds → face on the histogram/align-64 family; `small-bytes-copy-out-of-pinned-block` (write.rs:3144) → face on raw-pointer-traversal family; `statistics-carry-coverage-provenance` (planner_statistics.rs:56–159) → conceptually kin to katgpt-rs Issue 983 coverage_manifest, annotation-grade; count-before-fault-injection (fault_store.rs:186) → face on the measurement-honesty family; `lease-window-starts-at-write-not-response` + `abandon-not-release-parked-lease` (writer_lease.rs) → one face each, writer-lease family if it lands.

**COLLISION (the gate working):** `contended-counter-array-cache-line-aligned` (histogram.rs:93–101 `#[repr(align(64))]`) = positive instance of existing **`align-64-false-sharing`** (entries_53; named as kin in entries_18's context_notes). Dropped from the candidate table; the rule cites hydradb as a second independent instance if the entry ever gains a sources row.

## Dedup evidence (coordinator greps, this session)

- `align(64)`/`false.shar` → entries_53:80 (`align-64-false-sharing`) + entries_18 kinship mention — COLLISION, recorded above.
- `generation.*timer` → entries_56 (`generation-counter-stale-timer-invalidation`, Batch 318) — timers, not array-dedup; candidates 2 stays DISTINCT.
- `memset` → entries_03 (alloc_zeroed vs resize) — single-buffer allocation law; candidate 2's N-iteration invalidation axis unowned.
- `backoff` → entries_63/06 (transport retry ladder; blocking-sleep fixture text) — no speculative-switch owner; candidate 1 stays DISTINCT.
- `witness` → entries_58/39 (Drop witness / cache-reuse witness — different senses) — candidate 3's predicate-arity axis unowned; re-verify at authoring.
- `overlay` → entries_20 only (different sense); `spawn_blocking` → entries_06 only (blocking-sleep). Candidates 7/10/11 unowned.
- ⚠ One adjudication owed at mine time: candidate 14 vs `adaptive-yield-consecutive-ready` (omitted from the lane adjacency list — my error, flagged, not hidden).

## Lane-intel axis (ndb — user-directed question)

The league scoreboard is LLM-inference; no storage lane exists there — the mapped lane is **riir-neuron-db substrate** (the owner's ask). Four concrete mappings, none tied to an open ndb issue (ndb's issue files are compacted to HISTORY.md):

1. `wal-watermark-pinned-before-snapshot` + `frozen-artifact-plus-delta-tail-overlay` → ndb `WalBackend` seam (`R2Wal` sealed segments, `resume_after` cold-boot replay, snapshot+GC compaction) and freeze/thaw lineage envelopes: capture the sealed-segment id BEFORE taking the freeze snapshot; the frozen artifact + warm WAL tail overlay is exactly the ndb freeze+replay shape.
2. `tail-walk-cost-cap-declines-to-rebuild` → ndb `scan_after_prefix` cursor sweeps + Bm25Index rebuilds: cap the sweep, decline to a full index rebuild past the cap.
3. `derived-index-write-touches-only-changed-keys` → ndb LocalKv projected/derived-row writes + Bm25 posting updates (and the consumers' Repo-style save paths that rewrite whole rows on save).
4. `catalog-memo-scoped-to-the-snapshot` → Warm-tier scoped-row lanes: txn-scoped memo of scope-counter/catalog lookups.

Also of ndb-adjacent interest (not rules): buddy-wal's per-writer frontier tracking (frontier.rs — within an epoch every field only moves forward) is the bounded-bookkeeping shape behind the ndb `chunked_root` anchor chain and dapps' hourly points epoch; `generation-switched-index-rebuild-not-in-place` (keys.rs:196–269) is the catalog-flip shape the dapps corpus-days sync embodies.

**Verdict: LANE INTEL MODERATE (ndb)** — mappings recorded; no open lane forces them now. If ndb ever opens a WAL-overlay or scan-cap lane, this note is the reference design (pin + file:line).

## Product-coverage axis

Seeds **the LSM/storage-engine family — currently a GAP** (the corpus's io family owns page-cache/log-write rules; nothing owns write amplification, WAL-tail overlays, or index-generation GC) + **tokio family +1** (candidate 14). Follow-up pool if the family grows: SlateDB itself, the RocksDB write-amplification wiki, sled's io docs. Accumulation clause: pool sources read before the batch window fold into the same batch.

## Yield estimate

First batch (Plan 379): **4–6 rules + 2–3 faces** — lead `degree-gated-probe-with-doubling-retry` (benchable) + `bfs-depth-epoch-marker-dedup` (benchable) + the measurement-honesty pair (7/8) + 1–2 of the storage-transcription tier (9/10/11). Total across the source: **6–9 rules** over 1–2 batches after coordinator falsification (~50% of the 17 table rows + honorable mentions will die at fixture/adjacency/regime checks — the usual rate).

## Honest caveats

- ELv2 license — transcription-only posture (standard, but the note is per-entry).
- Unread remainders (both lanes disclosed): graph-slate `provider/mutation.rs`, `core/metrics.rs` (1,578 lines), `core/model.rs`, `engine/placement.rs`, bulk of `write.rs`/`codec.rs`/`cluster.rs` bodies; shard/query.rs + write.rs read selectively (~1.5k of 11.8k + ~1.2k of 8.9k lines). A second pass (placement rendezvous, provider mutation session) could add candidates.
- Heavy third-party substrate: SlateDB owns the storage engine; the WAL/lease/publication LAWS are hydradb's (in their doc comments), the ENGINE is SlateDB's — mine the laws, cite both.
- Candidate 14's adaptive-yield adjudication owed (above).
- The .raw clone was deleted at verdict close (pin + quotes in this note are the provenance); re-clone at the pin for authoring per Protocol A.

## Queue slot

Behind the InferenceForge verdict (nominal Batch 364 / Plan 375) → **nominal Batch 365 / Plan 379**. Nothing in this note is invented — every quote verified at the pin this session; dedup greps run live against the corpus at `97e2ea863`.
