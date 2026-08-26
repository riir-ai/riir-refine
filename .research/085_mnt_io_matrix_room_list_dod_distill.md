# Distill verdict — mnt.io "Memory pressure, lock contention, and Data-oriented Design" (Matrix Rust SDK room-list)

**Status:** DISTILLED — pending owner decision (queued as the next `rust_perf` external batch; plan opens at scheduling, the Research 082 → Batch 71 precedent)

**Source:** https://mnt.io/articles/about-memory-pressure-lock-contention-and-data-oriented-design/
(inspected 2026-08-26, read in full — 28 min; production code = `matrix-rust-sdk`
`matrix_sdk_ui::room_list` + `eyeball-im-util` `SortBy`). Article date 2026-02-23.

**Verdict:** YES, B+ — route **`rust_perf`** (concurrency + the sort/cache family),
NOT kernel_opt (pure CPU Rust, UI-SDK domain). Two candidates after dedup + drops.

## The story (why the mechanisms are real)

A "frozen room list" in Element X (Android): not frozen, but taking up to 5 minutes
to yield initial values. Memory analysis: **322,042 allocations / 743 MiB** inside
`eyeball_im_util::vector::sort::SortBy` for ~500-600 rooms. Root cause chain:
(1) each sorter comparison called `Room::latest_event()` which does
`self.info.read().latest_event.clone()` — a read-lock acquire + a **144-byte clone**
per call, twice per comparison, across 2 of 3 sorters; (2) `Vector::sort_by` is a
PRNG-pivot quicksort, so the allocation count varied per run (the randomness
red herring); (3) allocation pressure on the mobile allocator compounded it.

## Ranked candidates

| # | Pattern | Grade | Source evidence |
|---|---|---|---|
| 1 | **`sort-key-cache-precompute-out-of-lock`** — comparison closures that acquire a lock + clone per invocation → maintain a compact cached-key struct (the article's 64B `RoomListItem`), refreshed once per data update; sorting/filtering operate on the cache. Regime: **sorts-per-update > 1** (reactive re-sort on every diff). | **A (GOAT)** | 53.027 ms → 676.29 µs (**78× / −98.7%**), throughput 18.8 Kelem/s → 1.477 Melem/s (+7718.5%), `room_list` bench, before/after in article |
| 2 | **`lock-guard-narrow-field-accessor`** — `fn field(&self) -> BigValue { self.info.read().field.clone() }` where consumers read ONE sub-field (a bool, a timestamp) → split into narrow accessors returning stack-sized values (`field_is_local() -> bool`, `field_timestamp() -> Option<Ms>`). | **B+** | **+18% throughput** on the same bench from the narrow-accessor patch alone |
| — | PRNG-pivot randomness | DROP | Ambient narrative (why it was intermittent); no bounded fix-space |
| — | AoS → SoA | DROP | **Positive instance of `soa-hot-field`** — the article's own caveat ("if you're sure only one field in a single loop is used… SoA is cache friendlier") externally validates the corpus rule's regime note; cite as validation, not a new rule |
| — | Latency table / napkin math | DROP | Ambient knowledge (Colin Scott's latency numbers, Simon Eskildsen napkin math) |
| — | `VectorDiff::Set` shallow-clone infinite loop | DROP | Correctness tale domain-locked to eyeball-im reactive-stream semantics; no general-Rust bounded fix-space |

## Dedup gate (both vocabularies grepped, 2026-08-26)

- `sort-by-key` (corpus.rs:888) — its own `context_notes` define the adjacent
  regime ("the win materializes only when the key function is non-trivial …
  expensive computation the compiler cannot CSE") but the FIX-SHAPE is different:
  `sort_by_key` still invokes the key fn N times per sort — with the lock+clone
  inside it. Candidate 1 amortizes the key read to **once per update**, then sorts
  over plain cached fields. `sorts-per-update > 1` is the regime split.
- `papaya-lockfree` (corpus.rs:267) + `mutex-to-atomic` — the replace-the-lock
  family. Candidate 2 keeps the lock and shrinks the guarded payload. Different
  mechanism, different fix-space, composable with both.
- `soa-hot-field` — layout family; candidate 1 is about lock amortization +
  clone elimination, orthogonal to AoA/SoA (the article itself does AoS and says
  SoA might be even better — the corpus rule already carries that caveat).

## GOAT candidate check (`sort-key-cache-precompute-out-of-lock`)

1. **Benchable** — Pareto validator: harness sorts N items whose comparator
   acquires an `RwLock` + clones a 144B struct per call vs. sorting over a
   prebuilt key vec; a second axis varies **sorts-per-update** (1 vs many) to
   expose the amortization regime.
2. **Exact semantics** — sort ORDER is identical (same keys, same comparator);
   G1 bit-identical output trivially. The comparator has no observable side
   effects (locks are not output).
3. **Magnitude story** — lock/unlock ~17ns + 144B heap alloc + clone vs ~1ns
   cached-field read ⇒ mechanism-level 10-30× per comparison; the source's 78×
   end-to-end additionally includes allocation-pressure relief on a mobile
   allocator. **Annotate honestly: our isolated bench will read lower than 78×.**
4. **Deterministic** — no timing-dependent output.

Candidate 2 passes the same four (18% at source end-to-end; isolated comparator
bench expected ~1.5-3×).

## Yield estimate

**1-2 rules** after coordinator falsification. Both carry measured source
evidence (the strongest evidence class after measured-GOAT distillations);
candidate 1's regime split is two-sided (cache loses when sorts-per-update ≈ 1
and the update path outweighs — must annotate or the bench will falsify us).

## Honest caveats

- Source numbers are **end-to-end on their Android workload** (allocation
  pressure included); our validator measures the isolated mechanism — expect
  single-digit to low-double-digit ×, never cite 78× as our bench claim.
- Mine-time verification MUST read the real code (snapshot then):
  `matrix-rust-sdk` `matrix_sdk_ui::room_list::RoomListItem` (the 64B cache
  struct + `refresh_cached_data`) and `eyeball-im-util`
  `eyeball_im_util::vector::sort` (the SortBy comparator path). Article quotes
  may compress the real shapes.
- Both rules are likely **suggestion-only proposers** (architectural fix-shapes:
  add-a-struct + refresh-fn; split-an-accessor) — the `soa-hot-field`
  precedent. Corpus rules + bench specs land regardless of proposer feasibility.
- The narrow-accessor pattern is TYPE-blind-hard for a parse-only proposer
  (cannot see that the clone is 144B or that consumers use one field) — bench
  + suggest only.

## Pool drops from this search round (2026-08-26, recorded for the next pool builder)

- `simd-json` (simd-lite/simd-json) — classic SIMD JSON; repo-structure fetch
  FAILED (MCP "repo not found" — possibly renamed/moved org). **Unverified —
  re-probe at pool time; never graded unread.**
- developerlife.com "High-Performance Flat 2D Arrays in Rust (SIMD, L1 Cache)"
  (2026-07-14, fresh) — predicted heavy dedup vs `soa-hot-field` /
  `loop-order-innermost-contiguous` / `simd-chunk`. Drop unless starved.
- Reintech / Khimananda 2026 perf guides — listicle-shaped, no mechanism density.
- dev.to "Cache Optimization in Rust: HashMap Surprises to 4x Image Processing" —
  possible measured content, LOW priority, unverified.

## Numbering flags for the owner (discovered this session, NOT fixed)

- **`.plans/` 026 dual-allocation LIVE**: `026_dirty_encoding_write_elision_batch72.md`
  vs `026_kat_phase0_redaction.md`. Batch 72's number is baked into Research 084 +
  AGENTS.md rows; the KAT side (plans 023-028 lane, landed 08-26) likely renumbers.
  Same tie-breaker as the `.research/` 083 dual-allocation (already flagged).
- `.research/.highwater` = 84 with 85 files (083 dual); `.plans/.highwater` = 28
  with 29 files (026 dual). Next free: research 085 (this note), plan 029.

## Queue slot

Next `rust_perf` external batch when scheduled — **Plan 029 at open** (plan-FIRST).
Box constraint at verdict time: the p335 LoRA arm resident (~1 core, ~23 GB RSS)
→ GOAT builds defer until it exits; the batch is corpus-edits + benches when free.
