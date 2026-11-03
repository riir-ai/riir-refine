# Plan 330 — Batch 326: rustc PR #164484 adaptive sparse metadata tables → rust_perf

**Status:** IN EXECUTION — T0 claimed (zed-agent@amo, 1793735100, expires 2026-11-06T14:00:00+00.000000); converting Research 363's verdict into the corpus entry per T1–T8.
**Claimed:** zed-agent@amo, 1793735100, expires 2026-11-06T14:00:00+00:00 Number note: 329 was found double-held on disk at write time (`329_engine_mozilla_methodology_port` + `329_mozilla_outbound_pr_wetrun`, `.highwater` stale at 328) — this plan took the uncontended 330; the collision was RESOLVED owner-directed the same day (first-allocation: the methodology port keeps 329, the wetrun renumbered to `331_mozilla_outbound_pr_wetrun`, highwater → 331).

Source: rust-lang/rust [#164484](https://github.com/rust-lang/rust/pull/164484) "Use adaptive sparse encoding for metadata tables" — merged 2026-11-01T01:40Z, merge `026faeb1afef7581b2e49b2512d361d09819cb69`, head `de21c98c951e91adb4abf353494ba7739f425ab8`, r=nnethercote, milestone 1.101.0, METADATA_VERSION 11→12. Provenance record: `.research/363_rustc_pr164484_adaptive_sparse_metadata_tables.md`.

Landing shape: **1 rust_perf rule + 1 face** (category `layout`, suggestion-tier, `bench_spec: None` — the size-axis precedent class: `box-large-enum-variant`, `narrow-counter-overflow-promotion`; the compiler-internals precedent: B202 rustc #160193 → rust_perf).

## Tasks

- [ ] T0: claim the batch slot per the mining queue (`claim_lease_check.py --item 330`, append the `**Claimed:**` line, push — the push is the arbiter).
- [ ] T1: re-fetch the PR at the pin (files API + thread) and byte-verify every quoted hunk against the landing merge `026faeb1afef` — the table may have moved since 2026-11-01; re-quote or re-pin as found.
- [ ] T2: dedup re-verification at mine time (binds at landing; the verdict-stage greps are leads, never coverage): `sparse|dense|bitmap|popcount|rank|base|occupancy|fill.?ratio` PLUS the verdict-round-1 reviewer's extra set `rank.?select|succinct|elias.?fano|roaring` (its independent pass cleared them — `byte-rank-select-rarest-probe` adjudicated distinct: frequency-rank for search, not popcount-rank; re-run at mine time anyway) over `src/domains/rust_perf/corpus/` + the kernel_opt kin set — `csr-index-via-prefix-sum`, `tile-coo-sparse-tile-aligned-format`, `distinct-index-bitmap-ascending-compaction`, `attention-row-mask-bitmap-per-row-not-per-block` — distinct-from clauses both directions, gate needle asserts the split.
- [ ] T3: author the rust_perf entry `adaptive-sparse-blocked-table-encoding` (final name at authoring), category `layout`, suggestion-tier: buggy = an index-keyed table serialized dense full-width (one slot per key, defaults written); fixed = per-64-entry presence bitmap (u64) + per-block u32 base (cumulative non-default count of preceding blocks) + packed non-default values in table order, encode-time smaller-of gate (defaults > 25% AND `index_bytes + non_defaults*width < len*width` AND non_defaults ≤ u32::MAX), mode flag carried in the header word. **Original fixture code only** — never paste rustc source (license hygiene + the binary-only rule).
- [ ] T4: fold the census-gate face in-rule (count non-defaults only when `index_bytes < len*width` could win — the census pays only when the census can pay) + the 75% occupancy knob note (50% tested upstream, reverted: "didn't show a clear benefit") + the anti-regime (read-latency-critical random access stays dense; upstream accepted +0.3–0.4% mean primary instruction cost for −20.15% metadata size / −3.8% max-RSS, `perf-regression-triaged`).
- [ ] T5: gates — batch parse + needle test first-run PASS; count pin rust_perf 292→293 via `./scripts/pins_manifest.py --write` in the SAME commit (or close the plan as a clean negative if falsification drops the rule); clippy `-D` at default + no-default + the batched postures at an isolated `CARGO_TARGET_DIR`; retrieval floors both postures re-run.
- [ ] T6: score-bench + freeze-table obligations per P30 (rust_perf is default-on: corpus delta MAY owe the gen2/gen3 freeze regen — check `score_bench/generations.rs` triggers at landing).
- [ ] T7: docs — AGENTS.md rust_perf bullet count, README if it carries the count; flip this plan's checkboxes + Status; append the queue-snapshot LANDING row (claim discharge + commit hash).
- [ ] T8: commit named paths only (`docs:`/`feat:` prefix, develop), push.

## Honest caveats carried from the verdict

- The headline figure is **20.15%** (PR thread, comment 6246665077) — the 20.18% in the original prompt is corrected to the thread-verified number.
- `perf-regression-triaged` is an ACCEPTED trade (nnethercote: "cycles and wall-time were neutral. We decided before merging that this trade-off was reasonable") — never quote the label as a defect; it IS the anti-regime evidence.
- METADATA_VERSION 11→12: adopting the shape in any persistent format = version-bump discipline; readers must know the flag bit.
- The delegated-owner verdict round for the verdict was rate-limited at record time (resets 15:10 ICT 2026-11-03; single re-attempt disclosed in the landing row) — the batch landing may re-run the ping on the final entry if the lane wants it.
