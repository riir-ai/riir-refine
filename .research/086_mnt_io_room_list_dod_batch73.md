# Batch 73 — mnt.io Matrix-RSDK room-list DoD distill (rust_perf external)

**Status:** RECORD (Batch 73 landed 2026-08-26; Plan 029; consumes Research 085's queued verdict)

**Classification:** in-scope amplification (Issue 469 T2.1's gate — the rust_perf
domain + Pareto bench substrate already exist; the batch only adds corpus entries).

**Source:** mnt.io "Memory pressure, lock contention, and Data-oriented Design"
(2026-02-23) — verified in-source at mine time (Research 085's requirement):
`matrix-rust-sdk` `matrix_sdk_ui::room_list` `RoomListItem` (6 cached fields +
`refresh_cached_data` on every `RoomInfoNotableUpdate`; doc comment carries the
rationale verbatim: "RoomInfo is behind a lock… By caching the data, it avoids
hitting the lock") + the sorter Warning comments (recency.rs / latest_event.rs:
"Using `Room::new_latest_event` would be dramatic as it returns a clone of the
`LatestEventValue`") + `eyeball-im-util` `vector::sort` (`SortBy` re-invokes the
comparator from `binary_search_by` on EVERY diff; every filter change does a
fresh full sort in the `stream!` loop — the sorts-per-update > 1 regime is
structural, not incidental).

## Yield

rust_perf corpus **76 → 78** (caching 2→3, concurrency 6→7); benchable 66→68;
annotated 18→20 (both the measured-winner caveat class — ParetoDominant verdict
+ applicability condition, the B65 precedent). Zero proposers (both rules are
architectural/suggestion-only — Research 085's own honest caveat: type-blind
for a parse-only proposer; the `soa-hot-field` precedent).

1. **`sort-key-cache-precompute-out-of-lock`** (caching) — comparator lock+clone
   per invocation → maintained sort-key cache refreshed once per data update.
   **Measured ParetoDominant 24.3-37.2x** across the sorts-per-update axis
   (1/4/16 sorts per refresh cycle, N=512 rooms) — the amortization curve is
   visible in the axis itself. Allocs 164497→17 + bytes 8.1MB→532KB @16 sorts.
2. **`lock-guard-narrow-field-accessor`** (concurrency) — whole-payload clone
   under read-guard → narrow accessor returning only the consumed sub-field.
   **Measured ParetoDominant 11.5-12.7x** at 1K-100K calls; allocs 100000→0,
   bytes 4.7MB→0 @100K.

## GOAT (all PASS)

- **G1**: both harnesses produce real measurements; in-run debug_asserts
  (position-weighted sort-order checksum vs a make_input-computed expectation /
  expected local-count) hold in timing + alloc runs; 2 corpus-level G1 tests
  (`sort_key_cache_order_matches_lock_clone_comparator`,
  `lock_guard_narrow_accessor_values_match_wide_clone` — the B65 per-rule
  pattern).
- **G2**: lib tests 679/0 default + 697/0 rust_perf_bench state.
- **G3**: clippy 0 warnings in default/bench/all-targets states; retrieval
  floors HELD zero decay (gate PASS; perf@1 96.2, clippy@1 89.6); CI guard
  ALL LAYERS.
- **G5 + G-det**: both ParetoDominant; alloc-det stable; verdict-stable across
  2 invocations (rule 1: 24-37x band, rule 2: 11.5-12.7x band — margins far
  from 1.0x under load ~7.1).

## Honest findings (recorded in the annotations)

1. **The source's numbers are end-to-end; our bench is the isolated mechanism.**
   78×/−98.7% + 7718.5% throughput (their room_list bench, mobile allocator
   relief included) vs our 24-37×; +18% (their narrow-accessor patch alone,
   one of several stacked optimizations) vs our 11-13×. Both annotations
   carry "never cite the source number as our bench claim."
2. **The regime is two-sided and annotated**: sorts-per-update ≈ 0 (data
   updated, never re-sorted) makes the maintained cache pure refresh overhead
   — N narrow reads per Reset with no consumer.
3. **Bench-protocol discovery (the load-bearing one): a plain reverse() before
   each sort is ONE detectable run for the adaptive driftsort** — measured
   ~512 comparisons/sort at N=512 (alloc evidence: 1024 clones ≈ 2×N) vs the
   ~5100 the shuffle protocol pays. The first draft used reverse() and
   silently under-loaded BOTH arms ~10×. Fix: a fixed deterministic
   Fisher-Yates swap list (in-place, no allocs, the SAME permutation in both
   arms) applied before every sort — ~N/2 runs → full ~N·log N comparisons.
   The comparison count is the axis the rule lives on; a sort-bench that
   accidentally benches single-run reversals measures the wrong thing.
4. **Derived `Clone` does NOT count as a field read for rustc's dead_code** —
   the width-only payload fields (which only the buggy arm's clone ever
   touches) need `#[allow(dead_code)]` in both the harness mods and the G1
   test structs. Confirmed empirically twice (harness + test compile).
5. **Rule interaction**: the two rules COMPOSE — the production
   `RoomListItem.refresh_cached_data` reads through narrow accessors
   (`new_latest_event_timestamp`), not whole-payload clones. The sort-key-cache
   entry documents this in-rule.

## Count-pin sites updated (the 19th-site class)

`seed_corpus_has_seventy_eight_entries_across_twelve_categories` (renamed,
76→78, category comment caching 2→3 / concurrency 6→7),
`sixty_eight_rules_have_bench_specs` (66→68),
`twenty_annotated_rules_carry_context_notes` (18→20 + 2 ids + doc),
`unannotated_rules_have_no_context_notes` (message text 78-20),
`context_notes_backward_compatible_serde` (18→20 — found by the failing run,
NOT by grep: pre-existing site the batch records keep warning about),
`drafter_propagates_context_notes_from_perf_corpus` (18→20 + doc comment).
`ten_rules_are_not_benchable` unchanged (both new rules benchable).

## Box-constraint re-assessment (the Research 085 deferral, resolved)

Research 085 queued the batch with "GOAT builds defer until the p335 arm
exits." Re-assessed LIVE at plan-open: p335 = single-threaded deterministic
accuracy arm (~1 core of 12+, ~21 GB RSS of 64 GB, 83% memory free, no
wall-clock timeout in its command — CPU steal lengthens it, never corrupts
it); load 7.13. The Plan 337 trigger ("a measurement saturates the machine")
does not fire. GOAT ran under `CARGO_TARGET_DIR=/tmp/riir-clippy-b73` with
verdict-stability checks; the p335 arm was untouched and still running at
batch close. Swap was 4 GB residual throughout (untouched).

## Generator/process lessons

- The corpus-entry edit that dropped a `bench_spec:` line (old_text included
  it, new_text didn't) — caught by grep-verifying the entry structure after
  every corpus edit. Checklist item: after every PerfRule edit, grep the
  rule_id + assert bench_spec/context_notes lines are present.
- The raw-string terminator surgery (`}"#` → `}\n"#` for fmt) temporarily
  produced an INVALID `}"` terminator — caught by immediate re-compile. Never
  hand-edit raw-string boundaries for formatting; match the file's existing
  `"#\n        .to_string(),` shape from the start.

## Records

- Corpus: `src/domains/rust_perf/corpus.rs` (2 entries + 2 bench specs + 2 G1
  tests + 6 count-pin updates).
- Plan: `.plans/029_mnt_io_room_list_dod_batch73.md` (all tasks `[x]`).
- Queue: `.distill/001_mining_queue_snapshot.md` updated same-commit.
