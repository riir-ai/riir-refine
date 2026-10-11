# Plan 138 — Bench 094 default-eco histogram extension (wave 2 + merged t02)

**Status:** COMPLETE — wave 2 sampled + merged + joined; all tasks done 2026-09-17
**Date:** 2026-09-17
**Bench:** `.benchmarks/094_default_level_fired_histogram.md` (Extend protocol)
**Driver:** `scripts/sample_default_fired_histogram.py` (tracked this time — the
wave-1 generator was a throwaway `.tmp_run_clippy.py`; the Extend protocol is
explicitly rerunnable, so the driver becomes standing instrumentation)

## What

The Bench 094 Extend protocol: re-run the sampler against FRESH crates.io
pages (the API page>100 block still stands, so no data-dump strata — fresh
bands within the top-10k window), merge with wave 1 into
`.benchmarks/data/094_t02/manifest.json` (same schema), re-pin the blake3 in
the doc, re-run `cargo heal --coverage-report`.

Merge math (exact because the crate sets are disjoint after dedup):
`count' = count1 + count2`, `spread' = spread1 + spread2` per lint; new lint
keys union in; per-crate verdicts concatenate; compile-error rows append.
Wave-1 verdicts carried byte-identical (same pinned toolchain, clippy 0.1.98).

## Tasks

- [x] T1 — tracked sampler `scripts/sample_default_fired_histogram.py`:
      fetch pages → mechanical filters (not yanked ∧ max_stable_version ∧
      downloads ≥ 1000) → round-robin interleave → dedup vs wave-1 names →
      download/extract `.crate` → bounded-parallel
      `cargo clippy --quiet --message-format=json` (default lint level) →
      tally clippy::* → manifest; `--resume` via partial results JSONL;
      per-worker isolated CARGO_TARGET_DIR; scratch under system temp
      *(two bugs found + fixed in flight, both in the bench doc: the
      `message`-vs-`msg` parser key — caught by the akin-0.4.0 known-dirty
      control, recovered via the warm-replay `retally` subcommand — and the
      resume bare-name-vs-versioned-key skip; merge dedups keep-first)*
- [x] T2 — wave-2 run: pages 12/32/52/77/97 (ranks 1101-1200, 3101-3200,
      5101-5200, 7601-7700, 9601-9700), 200 candidates *(rank drift between
      the two fetches moved the interleave cut; population = 180 unique
      crates after keep-first dedup — the resume bug re-measured 122, replay
      pairs agreed 119/122, the 3 disagreements are heavy-crate
      timeout↔error flapping, conservative verdicts kept)*
- [x] T3 — merged `.benchmarks/data/094_t02/manifest.json` (schema v1,
      selection_rule documents BOTH waves; dataset_blake3 = blake3 over the
      canonical JSON of the manifest minus the blake3 field — definition
      recorded in the generator string + doc) *(380 sampled / 342 clean /
      1334 warnings / 145 distinct; blake3 9eaab0cd…; raw per-crate per-lint
      record preserved as `094_t02/results_wave2.jsonl`)*
- [x] T4 — loader repoint: `bin/heal.rs` `load_default_fired_histogram` REL
      `094_t01` → `094_t02`; `corpus_coverage.rs` module-doc path mention
- [x] T5 — `cargo heal --coverage-report` re-run: before/after evidenced
      uncovered rows (74 → ?), render sanity *(flip 74/448 → 94/442;
      head renders 145 rows over 342 crates, blake3 line verified)*
- [x] T6 — bench doc: Extend-executed section (wave-2 numbers, merged head
      rows, concentration facts vs wave 1), blake3 re-pin, t01 kept as the
      dated wave-1 record
- [x] T7 — gates: clippy clean on the write set, lib tests green (fixture
      corpus untouched → score-bench flat by construction; NO corpus delta →
      no freeze-table regen per the T7.5 rule) *(corpus_coverage 37/37;
      clippy -D warnings 0 on the bin)*
- [x] T8 — HISTORY.md entry + `.distill/001` note + commit + push
