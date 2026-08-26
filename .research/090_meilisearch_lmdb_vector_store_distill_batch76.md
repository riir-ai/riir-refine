# Research 090 — Batch 76: the Meilisearch LMDB/vector-store distill

**Status:** RECORD (batch landed same day; closed)

**Source:** "Patching LMDB: how we made Meilisearch's vector store 3× faster"
— Clément Renault (Meilisearch CTO), meilisearch.com/blog, 2026-03-18
(16-min engineering-deep article; production-verified on a 19M-embedding
customer trace: insert throughput ~6/s → ~20/s, and one identified step
559s → 2–3s). Found via the idle-loop online perf search (queue item 11 —
the B75-validated "one engineering-deep article > many listicles" path).

**Pool (same pass):** Qdrant "Introducing Gridstore" (qdrant.tech,
2025-02-05) — assessed **C+ for rule-mining**: an architecture piece
(tracker/mask/gaps layers, model testing, WAL idempotence, 2× ingestion).
Its one rule-shaped candidate — HashMap with dense sequential integer keys
→ Vec direct index — KILLED at dedup as adjacent to `o1-lookup` (the
Vec↔HashMap neighbor) + `dedupe-hashset-vs-vec` (the dense-int → Vec
marking neighbor). The gaps-layer per-region summary index is data-structure
design (segment-tree shaped), not a bounded fix-space. Also dropped: the
Meilisearch TLS read-transaction pooling (architectural — no bounded
fix-space; the `+1` main-thread transaction is a design anecdote, not a
rule) and the LMDB nested-read-txn C patch itself (upstream library API —
out of corpus scope by definition).

## The two rules (rust_perf 83 → 85; complexity 16→17, caching 3→4)

### 1. `debug-stat-full-scan-out-of-hot-path` (complexity)

- **Source mechanism** ("Stop computing useless stats"): the mean-degree
  sanity statistic — "added to Hannoy very early on as a sanity check" —
  was computed during item-ID retrieval, full-scanning the vector store on
  every insert batch. "Removing this reduces the time from 559s to about
  2–3s and brings the complexity down to O(n), where n is the number of
  newly inserted or removed embeddings."
- **Decision content:** a diagnostic eagerly computed in a hot path is a
  hidden O(collection) tax that GROWS with total store size — the hot path
  pays for the whole past, not the new work. Fix shapes: remove it
  (the source's fix — CI now guards the invariant), gate behind
  debug/opt-in, or maintain the aggregate incrementally.
- **Measured:** ParetoDominant 3/0/0 — 168.91×/1929.27×/5851.00× at
  N=10K/100K/300K (B=100 per op; verdict-stable ×2: 179×/1884×/6294×;
  allocs 0/0 equal by design; alloc-det stable). The win GROWS linearly
  with store size — the regime axis is N/B.
- **Dedup:** distinct from `sort-key-cache-precompute-out-of-lock`
  (per-comparison recompute under a lock; this is per-operation
  whole-collection statistics) and `filter-count-to-any` (predicate
  scan vs short-circuit; this is an aggregate whose result isn't even
  consumed in production).

### 2. `eager-pointer-collect-lazy-fetch` (caching)

- **Source mechanism** ("Full scans are harmful" + "But what's the new
  trick?"): the ImmutableItems/ImmutableLinks structures did "a full scan
  of all the entries we will need to access", collecting pointers into a
  HashMap — "we were polluting the cache with useless, unnecessary disk
  pages just by iterating over all of them to fetch pointers." Replaced by
  FrozenReader: "read uncommitted graph nodes and connections lazily, on
  demand" so "only the necessary pages are loaded into the cache — not all
  available pages in LMDB. We avoid evicting important pages."
- **Decision content:** building an everything-index to serve a subset of
  accesses pays the scan TWICE — once in time (O(all) build) and once in
  cache (the scan's own traffic evicts the hot pages the subset will need).
  When the underlying store offers a direct access path, index only on
  demand.
- **Measured:** ParetoDominant 3/0/0 — 1903.70×/5414.62×/21353.90× at
  N=100K/300K/1M, K=1,000 touched ids (verdict-stable ×2:
  2218×/5962×/25736×; allocs 1→0, bytes 35.7 MB → 0 at N=1M; alloc-det
  stable). REGIME K/N ≪ 1 + one-shot access; at K ≈ N the build amortizes
  to nothing; repeated-same-subset wants the persistent-index rule
  (`sort-key-cache-precompute-out-of-lock`) instead.
- **Dedup:** distinct from `collect-avoidance` (intermediate-Vec staging,
  alloc axis; the load-bearing half here is working-set residency) and
  `encoded-layout-borrowed-view` (representation re-expansion inside an
  accessor; this is index materialization BEFORE a subset read).

Both suggestion-only (type-blind/architectural — the soa-hot-field class).

## GOAT

- G1: both benches run + measure; the debug-stat maintained-aggregate
  equivalence gate (structured + LCG inputs, mean-bits equality) + the
  eager-pointer checksum ground truth both PASS; batch76_rules_present.
- G2: 696 default + 714 rust_perf_bench lib tests.
- G3: clippy 0 in both states `--all-targets`.
- G-det: alloc determinism stable across 3 runs (both rules).
- G5: 2/2 benchable ParetoDominant, verdict-stable ×2 invocations.
- Retrieval floors: 14/14 HELD (`kernel_heal,latent_retrieval` — no
  re-pins; corpus growth did not shift any floor).

## Generator lessons (this batch)

1. **The edit_file JSON layer strips one escape level** — `\\\"` in the
   edit becomes a bare `"` in the file, silently un-terminating Rust
   string literals. The robust route for string-heavy corpus entries:
   python here-doc with `rustlit()`-style explicit escaping (backslash,
   quote, newline each replaced once), never the structured-edit tool.
   (Same family as B75's "the part-1 insertion anchor must re-emit the
   closing `},`" — tool-layer hazards, not corpus-content hazards.)
2. **The bench compile caught the u64/usize checksum mix in harness 1**
   (B75's "pseudocode_parses is necessary, not sufficient" re-confirmed —
   syn cannot see integer type mismatches).
3. Count-pin census for a +2-rule batch: entries (83→85), annotated (26→28
   ×3 sites incl. the serde round-trip + the drafter test name itself),
   benchable (73→75), module doc header (was stale at 76/18/15 from an
   earlier era — rewritten to the true 85/20/17), category comments
   (complexity 16→17, caching 3→4).

## Honest scope notes

- The eager-pointer magnitude (1900×–21000×) is larger than the source's
  end-to-end 3× because the bench ISOLATES the mechanism (a full-scan
  pointer-map build vs none) on synthetic data sized past L2 — the
  source's number stacks graph-building work that now runs longer per
  their trace. Never cite 21353× as "what Meilisearch got"; cite it as
  the isolated mechanism's bound at N=1M/K=1K.
- The debug-stat bench's fixed arm still pays O(B) per call (the batch's
  own link sum) — the honest floor of the maintained-aggregate form; the
  source's REMOVAL fix (no statistic at all) would be O(0), noted in the
  context_notes as the primary fix shape.
