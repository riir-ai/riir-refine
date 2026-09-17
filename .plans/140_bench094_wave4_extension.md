# Plan 140 — Bench 094 wave-4 default-eco extension (+ the cross-wave base-dedup fix it exposed)

**Status:** COMPLETE 2026-09-17 — all tasks done; flip 110/442 → 129/442 (beating wave 3's +16 with +19); the pre-launch dead-dedup audit was the unit's GOAT. Record: HISTORY.md 2026-09-17 wave-4 entry; commit ref below.

## Why this wave runs (and what it must fix first)

Auditing the sampler for wave 4 found the cross-wave base-dedup in
`cmd_sample` is DEAD CODE: it compares the candidate's BARE name
(`r["name"]`) against the base manifest's VERSIONED per-crate keys
(`"actix-macros-0.2.4"`), which can never match. Wave 3's "0 base
collisions" was a fetch with no boundary overlap, not the dedup working.
Wave 4 samples pages ADJACENT to wave 3's (14/34/54/79/99 vs 13/33/53/78/98)
across three prior fetches taken at different times — rank drift makes a
boundary overlap realistic, and an undeduped re-measure would be
DOUBLE-COUNTED by `cmd_merge` (whose exactness premise is "crate sets are
disjoint"). Same defect class as the wave-2/wave-3 resume-key incidents,
one layer over: the third place names meet. The merged manifest's own
`selection_rule` text CLAIMS the dedup ("crates already in earlier waves
deduped by name and skipped — disjoint by construction") — currently a
false claim; this plan makes it true.

## Tasks

- [x] T0 pre-flight controls (the wave-3 protocol): toolchain byte-check
      (`clippy 0.1.98 (48a229ceae 2026-09-01)` / `cargo 1.98.1 (797e8a9bc
      2026-08-05)` — must equal t03's manifest strings) + known-dirty control
      `akin-0.4.0` → exactly its 3 recorded warnings, end-to-end through the
      tracked driver's own functions. **DONE** — both green.
- [x] T1 the dedup fix: candidate skip by the CONSTRUCTED key
      `f"{name}-{ver}"` against the base's versioned keys (the same exact-key
      law the wave-3 resume fix landed); `--base-manifest` help text updated
      ("prior-waves accumulated manifest — pass the latest tNN"); PLUS a
      merge-time guard: `cmd_merge` HARD-REFUSES (exit 1, naming rows) if a
      partial row's name already exists in the base crates — the exactness
      premise enforced, not assumed. Residual documented: a same-crate-
      NEW-version re-entry (rank drift + a release bump inside the window)
      creates a distinct-version row, not a duplicate count — accepted, note
      in the incident-ledger comment. **DONE.**
- [x] T2 fix validation BEFORE launch: synthetic probe against t03 as base
      (a known base crate at its recorded version must be skipped; a fresh
      name must pass) — the dead-dedup proof, inverted. **DONE** — both arms
      PASS on the real code paths (dedup probe + merge-refusal probe).
- [x] T3 wave-4 sample: `--pages 14,34,54,79,99` (ranks 1301-1400, 3301-3400,
      5301-5400, 7801-7900, 9801-9900), `--base-manifest
      .benchmarks/data/094_t03/manifest.json`, isolated scratch, workers 6,
      timeout default; resume-safe JSONL. Post-run controls: the printed
      `dedup_skipped` count read honestly (names verifiable in t03 if > 0) +
      the reconstruction diff (re-fetch the pages, replay the target cut,
      diff vs the file — the wave-3 `gsk4` lesson). **DONE** — 200 crates in
      10.3 min (162 ok / 38 compile_error); reconstruction: 499 fetched,
      base_skip=0, expected=file=200, no missing/extra.
- [x] T4 merge → `.benchmarks/data/094_t04/manifest.json` (`--wave 4`, base
      t03, meta from the sample run); verify population arithmetic
      (candidates/sampled monotone, compiled_clean = 501 + wave-4 ok,
      warnings/distinct recompute); archive `results_wave4.jsonl` beside it
      (t02/t03 dir precedent); record the new `dataset_blake3`. **DONE** —
      1994/780/663/116/1/2402/186 exact; blake3 `296cf4d7…`.
- [x] T5 loader repoint `bin/heal.rs` REL `094_t03` → `094_t04` +
      `src/corpus_coverage.rs` docstring counts (501/1801/164 → measured).
      **DONE** — verified live: the rebuilt report prints t04 (186 rows over
      663 crates, blake3 `296cf4d7…`).
- [x] T6 coverage-report re-run: the flip (110/442 → N/442), head-row
      movements (`needless_borrow` 92/30 → ?), any NEW mining-grade rows
      (count/spread) noted for the `.distill/001` intake. **DONE** — flip
      **110/442 → 129/442** (+19); `needless_borrow` 150/39 (left the
      uncovered head — Batch 151 covers it); new rows staged in `.distill/001`
      (`io_other_error` 57/8, `unit_arg` 46/2, `wrong_self_convention` 19/6,
      `redundant_static_lifetimes` 33/6, `double_ended_iterator_last` 17 perf,
      `match_ref_pats` 17).
- [x] T7 docs + validation + land: bench doc §Extend wave-4 section
      (+incident-ledger entry: the base-dedup corpse), HISTORY.md entry,
      AGENTS.md coverage numbers, `py_compile` the script, `cargo test --lib`
      (corpus_coverage tests), `cargo check` the touched bin/lib set, scratch
      cleanup, commit `feat: bench 094 wave 4 …` + push, plan COMPLETE.
      **DONE** — corpus_coverage 37/37; bin rebuilt at the required feature
      set (the default-features `cargo build --bin cargo-heal` SKIP silently —
      the stale-exe trap, caught by grepping the binary for the new path); all
      docs landed.

## Honest scope

- Diminishing returns are EXPECTED and recorded as such (wave 3: +16
  evidenced rows; wave 4 estimates +5-15). The value is (a) spread
  strengthening on the mining-grade head, (b) one more honest increment on
  compiled_clean, (c) the dedup fix itself — which wave 4 needed to run
  safely at all.
- Mining-batch staging from the strengthened rows stays owner-adjudicated
  (unchanged posture from the handoff).
