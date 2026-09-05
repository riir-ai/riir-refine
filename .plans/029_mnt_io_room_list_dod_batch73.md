# Plan 029 — Batch 73: mnt.io Matrix-RSDK room-list DoD distill (rust_perf external)

**Status:** COMPLETE — landed 2026-08-26 (all tasks `[x]`; the batch record is [Research 136](../.research/136_mnt_io_room_list_dod_batch73.md)).
Consumes [Research 085](../.research/085_mnt_io_matrix_room_list_dod_distill.md)
(the queued distill verdict: YES B+, 2 candidates, drops pre-adjudicated).

**Source:** mnt.io "Memory pressure, lock contention, and Data-oriented Design"
(Matrix Rust SDK room-list; production code = `matrix-rust-sdk`
`matrix_sdk_ui::room_list` + `eyeball-im-util` `SortBy`). Article 2026-02-23.

**Classification:** in-scope amplification — existing substrate families
(sort/cache, lock-payload) gain 2 rules with measured source evidence; no new
system, no new vocabulary type (Issue 469 T2.1's gate: this is `in-scope
amplification`, NOT boundary expansion — the rust_perf domain + its Pareto
bench substrate already exist; the batch only adds corpus entries).

**Evidence:** Research 085 §Ranked candidates: rule 1 source 78×/−98.7%
(+7718.5% thrpt) on the `room_list` bench; rule 2 +18% from the
narrow-accessor patch alone. Both deterministic, both benchable; honest
caveat — our isolated mechanism benches will read lower (10-30× / 1.5-3×)
than the source's end-to-end numbers (allocation-pressure relief included).

## Box-constraint re-assessment (2026-08-26, this session)

Research 085 queued the batch with "GOAT builds defer until the p335 arm
exits". Re-assessed LIVE before opening: p335 = single-threaded deterministic
accuracy arm (~1 core, ~21 GB RSS; CPU steal lengthens it, never corrupts
it — no wall-clock timeout in its command line); box at load 7.13/12+ cores,
83% memory free (~53 GB available), 4 GB residual swap untouched. The Plan
337 discipline trigger ("a measurement saturates the machine") does NOT fire.
GOAT proceeds with `CARGO_TARGET_DIR=/tmp/riir-clippy-b73`, benches verified
stable across invocations, load conditions recorded in Research 136. If
memory pressure appears (swap growth), abort remaining builds + defer.

## Tasks

- [x] T1 Mine-time verification: read the REAL code via GitHub —
  `matrix-rust-sdk` `matrix_sdk_ui` `RoomListItem` (the 64B cache struct +
  `refresh_cached_data`) + `eyeball-im-util` `vector::sort` (the SortBy
  comparator path). Article quotes may compress real shapes (Research 085's
  honest-caveat requirement). **Verified**: RoomListItem's 6 cached fields +
  refresh on RoomInfoNotableUpdate + the doc-comment rationale verbatim; the
  sorter Warning comments name both the wide-clone and narrow methods;
  SortBy re-invokes the comparator from binary_search_by on every diff.
- [x] T2 Dedup re-verification: grep the corpus for the adjacency claims
  (sort-by-key / papaya-lockfree / mutex-to-atomic / soa-hot-field /
  regex-hoist-lazylock / precompute-table) — confirm both rules remain
  distinct (sorts-per-update regime split; keep-lock-shrink-payload vs
  replace-lock). **Verified** against the full 76-rule id list.
- [x] T3 Author the 2 corpus entries (corpus.rs): `sort-key-cache-precompute-out-of-lock`
  (category: caching) + `lock-guard-narrow-field-accessor` (category:
  concurrency), with source-verified buggy/fixed shapes.
- [x] T4 Bench specs for both (BenchSpec sizes + harness): rule 1 with the
  sorts-per-update axis (1/4/16) exposing the amortization regime; rule 2
  clone-whole-under-read-guard vs return-the-field. Deterministic G1
  assertions (sort order identical; consumed values identical). **Protocol
  fix en route**: plain reverse() = ONE driftsort run (O(N), ~512 comps) —
  replaced with a fixed Fisher-Yates swap list (~5100 comps, the honest
  reactive re-sort cost).
- [x] T5 G1 semantic unit tests (the B65 per-rule-test pattern):
  sort-order identity across both arms + narrow-accessor value identity.
- [x] T6 Count-pin updates: entries 76→78, benchable 66→68, annotated 18→20
  (+ `context_notes_backward_compatible_serde` 18→20 — found by the failing
  run, the 19th-site class), category-count comment (caching 2→3,
  concurrency 6→7).
- [x] T7 GOAT: G2 lib tests (679/0 default + 697/0 bench state), G3 clippy 0
  both states + `--all-targets`, retrieval floors HELD (perf@1 96.2,
  clippy@1 89.6, gate PASS), the Pareto bench run for both rules
  (verdict-stable ×2 invocations: rule 1 24.3-37.2x with the visible
  amortization curve; rule 2 11.5-12.7x), honest `context_notes` from the
  MEASURED verdicts (never cite 78× as our bench claim; regime-annotate the
  sorts-per-update ≈ 0 loser side). CI guard ALL LAYERS.
- [x] T8 Docs: Research 136 (batch record), AGENTS.md batch entry + corpus
  count, `.docs/11_domains/rust_perf.md` row, queue snapshot update (same
  commit).
- [x] T9 Commit `feat: batch 73: ...` + push develop.

## Non-goals

- No proposers (both rules are architectural/suggestion-only — the
  `soa-hot-field` precedent; Research 085's own honest caveat: type-blind
  for a parse-only proposer).
- No clippy-domain changes; no kernel_opt changes.
- Pool drops stay dropped (Research 085 §Pool drops; simd-json re-probe
  belongs to the next pool builder).
