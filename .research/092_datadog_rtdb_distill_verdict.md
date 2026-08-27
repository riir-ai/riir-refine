# Research 092 — Datadog RTDB/Monocle distill verdict (Batch 78 source)

**Status:** RECORD — Batch 78 LANDED 2026-08-27 (Plan 036 complete; commit in the same session). The bench OVERTURNED the verdict-time GOAT-lead ranking: `narrow-counter-overflow-promotion` measured **ParetoDominated 0.64-0.66x** (the memory-for-speed trade — honest-annotated, the `filter-count-to-any` class) while `cache-line-block-partitioned-lock` measured **ParetoDominant 1.12/3.91/2.98/3.03x at 2-12t** (verdict-stable x2). Both landed. See the §Landing addendum below.

**Source:** "Evolving our real-time timeseries storage again: Built in Rust for performance at scale" — https://www.datadoghq.com/blog/engineering/rust-timeseries-engine/ (published 2025-08-04; authors Khayyam Guliyev, Duarte Nunes, Ming Chen, Justin Jaffray; read in full via webReader 2026-08-27). Blog-only (no public repo for Monocle) — fixtures are original minimal reproductions per the standing license constraint.

**Provenance chain:** 6th-gen lineage (Cassandra → Redis → MDBM/mmap → Go B+tree → RocksDB sidecar → unified Rust LSM). The article's own upstream cites: Google Slicer (routing), Gorilla/Facebook (timeseries compression), "Constructing and Analyzing the LSM Compaction Design Space", the Linux kernel virtual-memory map (the 4-level radix tree), CoDel (query throttling). Production-verified at Datadog scale: **60x ingestion, 5x high-cardinality queries, 2x cost-efficiency** — all END-TO-END architecture numbers, never citable as isolated mechanisms (the mnt.io 78x precedent, B73).

## Why this source

The validated hunting ground (B75/B76): production-Rust engineering blogs with measured customer traces. Datadog's RTDB is a from-scratch Rust LSM timeseries engine with a 15-year generational lineage behind every design decision — the article carries per-mechanism reasoning (measurement-driven: "production data showed ~75% of DDSketch instances have four or fewer bins") rather than folk wisdom. The Aug-2025 date is 13 months stale but the mechanisms are timeless.

## Ranked candidates (dedup-grep evidence per row)

| Pattern | Grade | Dedup grep (both vocabularies) | Disposition |
|---|---|---|---|
| `narrow-counter-overflow-promotion` — hot counter arrays stored u32, checked-promotion to u64 on overflow (Rust's overflow checks ARE the detector) | **B+ (GOAT lead)** | `overflow.*promot\|promot.*overflow\|narrow.*counter\|squeeze.*u32` → 0 hits in `src/domains/` | KEEP — two-regime (common case never overflows → half-width storage; rare overflow → promote), bounded fix-space, integer-exact |
| `cache-line-block-partitioned-lock` — K consecutive items per `#[repr(align)]` cache-line block under ONE mutex (8 points/block at Datadog) | B | `cache.line.*block\|fine.grained.*lock` → hits only in kernel_opt GPU corpora (smem staging — different substrate); rust_perf concurrency has `align-64-false-sharing` (B65) — ADJACENT but mechanistically distinct | KEEP with explicit differentiation: B65 pads to eliminate FALSE sharing between independent slots; this partitions TRUE sharing of one logical array at line granularity. Both live on the coherence-granularity axis |
| single-flight concurrent load — "only one thread loads the missing data — others wait and reuse the result" | C+ | `single.flight\|request.*coalesc\|inflight.*dedup` → 0 hits | LEAD only — 1-sentence treatment; the parse-only detection shape (concurrent identical fetches) is hard to write; expect death at verification |
| inline-common-case-bins — ≤4 bin counts stored directly in the summary struct (75% of production sketches) | DROP | `smallvec` → `smallvec-stack` rule exists (27-75x, B59-era) | Positive instance of an existing rule — not a new rule |
| 4-level radix tree (fanout 16) for sparse-clustered keys | DROP | `radix` → only the SWAR "radix folding" comment (arithmetic, different concept) | Data-structure architecture, 65,536-bin-specific; no bounded fix-space |
| shard-per-core partitioned concurrency (no locks/atomics on write path) | DROP | — | Architectural (the TLS read-txn pooling drop class, B76) |
| MPSC-per-shard message passing (tachyonix) + cooperative yield | DROP | — | Architectural |
| CoDel cost-based query throttling (FIFO↔LIFO) | DROP | — | System-level scheduling |
| Tiered compaction + time-range pruning + per-file bloom | DROP | — | Database internals |
| Unified series cache (per-timeseries, time-segmented LRU, proactive population on flush for cached-only series) | DROP | — | Cache-design architecture; the "populate-on-flush only for already-cached series" detail is nice but the shape is system-level |
| SIMD codec for compression (replacing Gorilla) | DROP | — | No specifics (no codec named, no numbers) — unmineable |

## GOAT candidate — `narrow-counter-overflow-promotion` (the four tests)

1. **Benchable**: rust_perf Pareto validator, memory + throughput axes — a counter-array aggregation sweep at u64 vs u32+promotion; the alloc/bytes axis shows the halving exactly; throughput shows the cache-density gain.
2. **Exact semantics**: integer counting with promote-on-overflow preserves every observable total exactly → bit-identical G1 by construction (the promoted values equal the u64 totals at every step).
3. **Magnitude story**: half-width counters = 2x counters per cache line → ~2x fewer line touches on a counter-array sweep; Datadog's number (the radix buffer incl. this: +70% aggregation throughput) is the embedded evidence — the ISOLATED mechanism needs our own bench (honest expectation 1.2-2x on counter-heavy sweeps, regime-gated: pays on LARGE counter arrays where density matters; a lone counter pays a branch for nothing).
4. **Deterministic**: integer counting, no timing dependence.

**Regime conditions (the rule's decision content):** counter ARRAYS (many slots, sweep access) where measured/assumed counts stay under 2^32; NOT single counters; NOT counters expected to overflow (the promotion path would then run constantly — worse than u64). Datadog's own framing: "in practice most counts never approach the 32-bit limit" — the measurement claim rides in the regime note.

**Adjacent-rule differentiation required at authoring time:** `usize-index-to-u32` (B61) narrows INDEX/OFFSET fields where the domain bounds below 2^32 (a layout claim); this narrows VALUE counters whose common case stays low but whose domain does NOT bound them (a representation claim with an escape hatch). Different detection shape (index fields vs accumulation targets), different guard (checked promotion vs domain bound).

## Drops recorded, honesty ledger

- The headline 60x/5x/2x are architecture-end-to-end — never cite as mechanism evidence (mnt.io precedent).
- The u32-promotion + radix-tree + inline-bins + shared-buffer together measured +70% aggregation throughput and "orders of magnitude" memory — the compound is theirs; the isolation is ours to bench.
- No public code — every quote above is from the article text; fixtures must be original.
- Batch GOAT gates on a free box (sibling Issue-762 bench iterations live at verdict time — Plan 337 discipline).

## Queue slot

Batch 78 (Plan 036), rust_perf, expected landing 1-2 rules. Cited by the queue snapshot line in the same commit.

## Landing addendum (2026-08-27, Batch 78)

- **The yield estimate held** (2 rules landed, both keep-candidates survived falsification — the rare 100% batch).
- **The GOAT-lead ranking inverted at measurement**: the u32+sidecar form is a MEMORY trade, not a speed win on update-heavy shapes (flat 0.64-0.66x across 100K/1M/4M — branch-bound, not bandwidth-bound; Datadog's +70% was the compound, exactly as the caveat predicted). The honest-annotation class caught it: never promote a perf gain on a wrong result — the rule now documents the trade with the measured cost.
- **`cache-line-block-partitioned-lock` is a clean winner** (2.5-4x at 4-12t) and forced the validator's first `ALLOC_AXIS_EXEMPT` (bench.rs): contended `std::sync::Mutex` on macOS allocates nondeterministically in the pthread slow path — probed 41/44/42/41/41 allocs across 5 runs (global-mutex 12t) vs EXACTLY 513 for uncontended construction of 512 locks. The exemption is probe-evidenced + per-rule, never a silent skip.
- **The single-flight C+ lead died as predicted** (parse-only detection shape — recorded in Plan 036 T6).
- **Harness lessons** (the B75 class, twice): `std::thread::Scope` takes TWO lifetime params; it is INVARIANT over `'scope` — helper fns taking `&Scope` fight the borrow checker; the spawn-body-inline shape (the align-64 convention) is the only smooth form.
- **B77's stale benchable header caught en route** (74 vs true 76→78) — the count-pin-lesson class, fixed in the same walk.
