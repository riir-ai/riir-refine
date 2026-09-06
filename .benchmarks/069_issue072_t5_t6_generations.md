# Bench 069 — Issue 072 T5/T6: the generation-v2 freeze measurement + the M5 admission verdict

**Status:** RECORD — freeze complete, admission honestly REFUSED (pre-registered branch B),
the full ratchet machinery live + exercised end-to-end. Plan 092; issue 072 T5/T6.

**Box:** 4090 / Windows, CPU-only (the oracle spawns cargo+clippy; no GPU anywhere).
**Command:**
`cargo test --release --features score_bench --test generation_v2_freeze -- --ignored --test-threads=1 --nocapture`

## The freeze (M4 exactness ladder)

The atom factory is construction-first: per vocabulary lint, the first
(`min buggy_code`, sorted-(lint, code)) seed-corpus entry whose `buggy_code`
parses standalone as a file. The ladder: k ∈ {1,2,3,4}, deterministic
stride-window selection over the sorted lint set, composites wrap each atom
in its own `mod gen_{i}` (zero item-name collisions, zero cross-atom
interference by construction).

| Measure | Value |
|---|---|
| Atoms | 38 (of ~44 vocabulary lints — the rest are fragment-shaped) |
| Bad atoms | **2** — `needless_match` also fires `manual_map`; `uninlined_format_args` also fires `useless_format`. Not valid single-lint atoms. |
| Specs generated | 66 (k=1: 38, k=2: 16, k=3: 12, k=4: 8) |
| Union-identity failures | **8** — the 2 bad k=1 atoms + the 6 composites containing them (cascade). **Zero independent interference.** |
| Verified composites | 58, oracle labels ≡ union gold 58/58 (the G1 construction-vs-oracle fence, real clippy) |
| Freeze record | `FROZEN_LABELS` (58) + `EXCLUDED_IDS` (8) landed in `src/score_bench/generations.rs`; the always-on coverage + drift gates pin it |

## Per-k solve rates — the M4 difficulty measurement

| k | resolved/before | rate |
|---|---|---|
| 1 | 36/36 | **100.0%** |
| 2 | 28/28 | **100.0%** |
| 3 | 30/30 | **100.0%** |
| 4 | 24/24 | **100.0%** |

The monotone-decay gate passes vacuously (flat). **This is pre-registered
branch (B) of `.plans/092`: the k-ladder over the corpus's own shapes cannot
be harder than the proposers trained on that corpus.** The v1 bench's 3.8%
persist comes from real-repo shapes the corpus does not carry (second-level
macro nesting, named-arg-with-rename, array-literal defaults — the
documented divergence classes); corpus atoms sit entirely inside the
proposers' covered distribution, so composite width does not create
difficulty.

## The M5 admission verdict (exercised live)

`cargo run --release --features score_bench --bin cargo-heal -- --score-bench --generation 2`:

```
generation v2 admission REFUSED (non-vacuity, M5): v2 heal_rate 1.0000 >= v1 0.9623.
A new generation that is not harder is not admitted; nothing appended.
```

Exit 1, **nothing appended** (`score_history.jsonl` length unchanged at 45
across the refusal) — the non-vacuity gate refuses a generation that is not
harder, exactly per issue 072 T6's spec. The frozen generation is
deterministic, so the failed admission is reproducible; the verdict lives
here instead of in the JSONL.

The v1 path ran clean in the same session (run #46 appended, exit 0) and
the saturation telemetry read the flat tail correctly:

```
saturation: gen-1 SATURATED (marginal +0.0000 < 0.005 over 5 runs) — consider `--generation 2` (v1 floors stay pinned)
```

The ratchet loop is live end-to-end: detector fires → ratchet suggested →
candidate generation measured → admission honestly refused → finding
recorded. `--misses-report` exits 0 on the (currently empty) queue with the
honest line; the frontier-order rendering is unit-pinned against a planted
queue (informative → low-entropy informative → regenerate, conversion
column rendered).

## What actually remains (the difficulty axis)

The mechanism (detector + admission + per-generation series + freeze
discipline) is landed and pinned; what the first ladder lacks is hardness.
The reopen candidates, in order of expected yield:

1. **Persist-shape stratification** — build composites from the v1 persist
   map's actual failure shapes (the bench's own per-fixture diagnostics
   name them) instead of corpus atoms. These are the shapes the proposers
   demonstrably miss; k-width then composes real difficulty.
2. **Real-repo interference spans** — mine multi-lint spans from sibling
   repos (the `score_bench_mine` path, v1's provenance discipline) at
   controlled k, where lint interactions are organic.
3. **Overlap-edit chains** — composites whose proposers' edits overlap
   (the fixpoint's drop-and-refire semantics), forcing multi-pass
   convergence past `MAX_FIX_PASSES = 4`.
4. **The rustc-errors-side ladder** (the concurrent session's design,
   salvaged from their superseded plan): compose `corpus_gen`
   `GeneratedCase`s at k ∈ {1..4} and measure the `--fix-compile` driver's
   solve-rate per k — a DIFFERENT oracle (per-sub-lint reduction to
   composed golds, exact by construction) and a different repair domain
   than the clippy bench. Their T2/T3 design notes were the one part of
   the duplicate unit this landing lacked.

**Detector design note (the concurrent session's variant, recorded not
shipped):** their detector compared the last run against the WINDOW MEAN
(not the endpoint) and was SIGNED (a regression reads `Moving`, not
`Saturated` — "is the set still measuring" vs the SVG's regression
surface). The shipped `detect_saturation` is endpoint-based and
per-generation (the admission gate needs per-generation series; the
endpoint form is what run #46's live SATURATED verdict measured). A
regressing series reads Saturated here — correct for the ratchet's
purpose (a regressing baseline must not admit a harder generation anyway;
the refusal compares v2 against v1's LATEST rate). Window-mean/signed
remains a re-pin candidate if the endpoint form misfires on a noisy
window.

**Real-G8 baseline note (the concurrent session's T1.3 ledger, KEPT — the
one surface that composed):** the conversion A/B is era-split, not
frontier-vs-FIFO-rerun: the FIFO arm is HISTORICAL (`WIRED_STRATEGIES`,
authored pre-wiring), the frontier arm appends `FRONTIER_LANDINGS` rows in
the same commit as each authored playbook, and `WIRING_COMMIT` pins where
the baseline window ends (the T39/Bench-051 discipline). The ratchet test
`wired_plus_frontier_equals_the_derived_inventory` makes ledger drift a
same-commit failure. `--misses` prints this plan's frontier table
(conversion column from store outcomes); `--misses --report` prints the
ledger.

Reopen: any candidate lands as a NEW generation number (3, not a silent v2
rewrite) through the same freeze discipline — `FROZEN_LABELS`/`EXCLUDED_IDS`
regenerated by the e2e, the always-on coverage gates re-pinned, and the
admission gate re-armed. v1 floors stay pinned forever either way.
**EXECUTED same day: candidate #1 (persist-shape stratification) landed as
generation 3 and was ADMITTED (87.5% < v1 96.2%) — record:
[`.benchmarks/070_generation3_persist_stratified_ladder.md`](070_generation3_persist_stratified_ladder.md).**
**EXECUTED 2026-09-06 (later session): candidate #4 (the rustc-errors-side ladder)
landed as generation 4 and was ADMITTED (54.3% < v1 96.2%, deterministic ×2;
uncontested 100% / contested 20% — the delegate-only families are the structural
misses) — record:
[`.benchmarks/071_generation4_rustc_error_ladder.md`](071_generation4_rustc_error_ladder.md).**

## Validation

- lib default 1116/0 (1112 + the 4 new evolve gates); lib 4-feature union
  (latent_retrieval, rust_perf, rustc_errors, score_bench) 1293/0/8i;
  `generation_v2_freeze` always-on 1/0/1i.
- clippy `--all-targets` 0 real findings in the union state (one
  line-initial `+ ` doc_lazy_continuation caught pre-commit — the B37
  lesson, now its 14th strike).
- The freeze e2e re-run after landing the record: PASS (zero label drift —
  the drift gate live).
- Bench #46 (v1): 96.2/1.9/100/100 FLAT + the SATURATED verdict line —
  count-identical to #43–45, the heal path unregressed.
