# Bench 112 — fsearch outbound-PR bench: the BEFORE arm (Plan 224, bench-refine-pr lane)

**Date:** 2026-10-11 · **Repo:** riir-refine @ develop · **Box:** Apple M3 Max (Mac15,8), AC power (96%, charging), quiet-ish window load 6.18–6.37 (1-min avg, disclosed; load-sensitivity quantified below) · **Provenance:** numbers taken in-process by fsearch's own rig; no GPU, no new deps.

**Status:** RECORD — the "before" arm of the bench-refine-pr lane; the "after" arm is GATED on mainnet launch (Plan 224 T5–T8). This file is the fixture + method + numbers the after arm must reproduce against.

## Source + pin

- `noahdunnagan/fsearch` @ `b88f4e9a90703eb7338085b325c2852fcaf131a9`, MIT, 1.6k stars. Release profile at the pin: opt-level 3, fat LTO, codegen-units 1, PGO available upstream (pgo.sh). Zero clippy findings workspace-wide (all-targets) — the tree is clippy-clean; our healer's mechanical surface upstream is EMPTY (verdict: Plan 224).
- Harness: upstream `examples/rig.rs` (deterministic xorshift corpus, in-process timing, per-query result digests) + TWO UNTRACKED harness files of ours (no upstream byte modified): `bench_scope.rs` (scoped index — the stock daemon crawls `/` and stalled >12 min on this box without Full Disk Access) and `gen_names.rs` (name-side corpus = the rig's own sampling minus the content half; names/typing are drawn before the content section, so queries are byte-identical to the stock `rig gen`).

## Fixture (frozen — both arms use THESE bytes)

| artifact | blake3 |
|---|---|
| `/Volumes/SDXC1TB/tmp/fsearch-bench/snap/index.bin` | `50eeff298ccc348f76c35f07882c1988b3649a9c43055713c26676c9a41f99de` |
| `/Volumes/SDXC1TB/tmp/fsearch-bench/corpus.json` | `db232868d6d3b4ef0ee22782ec83e50dc96b62779031f0ba90162dd2cf5f0fb1` |

Index: 4,019,254 entries / 2,531,147 unique names / 291 MB over `/Users/katopz/git` (scan 24.9 s at the pin). Corpus: 200 name queries + 10 typing runs. Sidecars in this directory: `corpus.json`, `before_q1.json` (the published run), `bench_scope.rs`, `gen_names.rs`, `bench_before.svg`.

## Before numbers (the published arm — run q1)

Protocol: quiet window only (load < 8), 3 back-to-back rig runs, publish the clean-window run; stat = geomean over per-query medians (3 rounds per query) — the rig's own aggregation.

| metric | n | geomean | worst |
|---|---|---|---|
| name, exact | 60 | **206 µs** | 0.67 ms |
| name, one typo | 60 | **192 µs** | 0.75 ms |
| name, common word | 30 | **262 µs** | 1.31 ms |
| name, folder + name | 30 | **695 µs** | 6.36 ms |
| name, filtered query | 20 | **469 µs** | 8.05 ms |
| typing, whole filename | 10 | **7.49 ms** | 13.25 ms |

Load-sensitivity (why the window rule exists — same rig, same snapshot, no code change):

| window (1-min load) | name exact | typing | factor vs q1 |
|---|---|---|---|
| 6.2–6.4 (q1, published) | 206 µs | 7.49 ms | 1.0× |
| 7.9–9.0 (q2/q3) | 247–480 µs | 7.43–19.1 ms | 1.2–2.3× |
| 32–40 (5 runs) | 860–941 µs | 29.9–34.5 ms | 2.8–4.6× |

## What the after arm owes (Plan 224 T6)

Re-clone at the pin, restore the harness sidecars, verify both blake3s, land the fix (T5), rebuild, `rig run` the same corpus under the same window rule, assert **every query's result digest byte-identical** to `before_q1.json`, render the before/after SVG (`scripts/fsearch_pr_bench_svg.py --before before_q1.json --after after.json`), chart only rows moved beyond the quiet-window spread (~5%), and carry the box state beside every number.
