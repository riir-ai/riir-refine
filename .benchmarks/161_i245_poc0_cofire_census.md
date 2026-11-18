# Bench 161 — i245 PoC-0: rule co-fire census (NEGATIVE-sparse)

**Date:** 2026-11-18 · **Lane:** shikuwa-4090-i245-poc0 (lease `c0f8e83b44c`, scope PoC-0 census ONLY)
**Issue:** riir-refine Issue 245 (closed by this record per its outcome contract) · **Research:** 420 (status demoted GAIN → NEGATIVE-sparse)
**Instrument:** `scripts/i245_poc0_cofire_census.py` (tracked; deterministic, stdlib-only, ~1 s wall)
**Data:** `.benchmarks/data/094_t0{2,3,4}/results_wave{2,3,4}.jsonl` — 702 rows (589 `ok`, 110 `compile_error`, 3 `timeout`), 580 distinct crates, 122 repeated (wave, crate) rows. Wave 1 (`094_t01`) totals-only, excluded per the filing's own note.

## Verdict: NEGATIVE-sparse — the kill condition fires; the PoC chain stops at PoC-0

> **Kill condition (pre-registered in Issue 245):** "< 40 crates with ≥2 fired lints or < 60 edges of weight ≥3 → the graph is too sparse for community discovery at min-support 3 — record NEGATIVE-sparse and stop."

Under the pinned convention (**`verdict == "ok"` rows only + per-crate PAIR-union across waves**):

| quantity | pinned value | bar | reads |
|---|---|---|---|
| crates with ≥2 fired lints | **78** | ≥ 40 | PASS |
| co-fire edges weight ≥ 3 | **57** | ≥ 60 | **FAIL — kill fires** |

57 < 60 by 3 edges. Per the issue's outcome contract ("any gate fails → NEGATIVE recorded in Research 420, tier demoted, this issue closed with the record; no middle posture"), the PoC chain terminates here; PoC-1–4 are never run.

## The census table — all six conventions (the filing's parse-criteria delta, resolved)

| ok_only | dedupe | crates_ge2 | lint_universe | distinct edges | edges_ge3 | edge instances |
|---|---|---|---|---|---|---|
| ok | **pairunion** (PINNED) | 78 | 140 | 1026 | **57** | 1289 |
| ok | keyunion | 78 | 140 | 1026 | 57 | 1289 |
| ok | perwave | 99 | 140 | 1026 | 111 | **1579** |
| all | pairunion | 84 | 153 | 1318 | 68 | 1635 |
| all | keyunion | 84 | 153 | 1318 | 68 | 1635 |
| all | perwave | 105 | 153 | 1318 | 124 | 1925 |

Conventions:
- **pairunion** — one entry per crate; edge set = union of same-scan pairs across the crate's rows. Each crate counted once; only pairs that truly co-fired in one scan. Graph-correct dedupe.
- **keyunion** — union the lint keys per crate, then form pairs. Naive reading; can hold phantom cross-scan pairs (A from wave2 + C from wave3 never co-fired). **Empirically identical to pairunion on this data** (zero crates have disjoint scans whose key-union manufactures cross pairs) — the phantom-pair concern is nil here, reported for completeness.
- **perwave** — every (wave, row) contributes independently. True same-scan pairs, but the 122 repeated rows double-count crates (weight inflation).

### The filing's two parses, fully attributed

| filing parse | its numbers | this census |
|---|---|---|
| claude-verdict parse | 105 crates / 153 lints / 1318 edges / 124 ge3 | = **all+perwave** distinct pairs (crates/edges/ge3 reproduced exactly; lint universe 153 exact) |
| coordinator spot-check | 99 / 154 / 1579 / 111 | crates+ge3 = **ok+perwave** (99/111 exact); **1579 = ok+perwave edge INSTANCES** (with multiplicity), not distinct pairs |

Both prior parses were per-wave readings. The disagreement that motivated PoC-0 is fully explained by (a) the verdict filter and (b) distinct-pairs vs instance counting — and the **viability both parses suggested (111/124 ≥ 60) was the double-counting artifact**: deduplicated to one entry per crate, the ge3 core drops to 57 (ok) / 68 (all).

### Parser validation

The instrument reproduces both independently-written prior parses exactly on every comparable number (105/1318/124 and 99/111/1579) — two parsers, one census, zero unexplained deltas.

## Why the pin stands (disclosed, not bar-shopped)

The ok-only filter was pinned **before** any census ran (rationale in the instrument header + the claim commit `c0f8e83b44c`: `compile_error` scans are truncated runs — clippy emits an arbitrary prefix of its lint pass before dying, so non-ok rows sample a different lint regime; the 110 truncated scans are not population-consistent with the 589 completed ones). The all-rows variant would read 68 ≥ 60 (pass by 8) — recorded here as the sensitivity disclosure, **not** adopted: flipping the pin after seeing 57-vs-68 is exactly the post-hoc bar-shopping the pre-registration exists to prevent. Note the pass margins on offer: +3 needed vs +8 available, both razor-thin on a bar designed as a sparseness floor for min-support-3 community detection.

## PoC-1 pre-registration (pinned, moot — never run)

Stated in the census output before any community run, per the filing's requirement: Louvain-class weighted detection, min-support 3, degree-preserving permutation null (lint fire counts + crate sizes fixed), 1000 draws, PASS iff observed max community size ≥ the null's 95th percentile (one-sided p ≤ 0.05) with size ≥ 3. Recorded so a re-open inherits the protocol rather than re-deriving it.

## Outputs

- `scripts/i245_poc0_cofire_census.py` — the tracked instrument (stdlib-only; explicit utf-8 I/O; ASCII-safe console)
- `.benchmarks/data/094_cofire_census/census_combos.tsv` — the six-combo table
- `.benchmarks/data/094_cofire_census/edges_weighted.jsonl` — 1026 edges under the pin ({"a","b","weight"})
- `.benchmarks/data/094_cofire_census/population.jsonl` — 580-crate ok population (lint sets + waves)

## Outcome contract discharge

- [x] NEGATIVE recorded in Research 420 (status line demoted GAIN → NEGATIVE-sparse + §7 record with the re-open condition)
- [x] Tier demoted in the same landing (Research 420 §4 + §7)
- [x] Issue 245 closed with the record (file removed per the noise-reduction rule; this bench doc + Research 420 §7 carry the record; lease folded at close, git history recovers the claim line)
- Re-open condition (new data, not a re-parse): a **wave-1 per-crate re-run** (`094_t01` is totals-only) would add a fourth wave to the population — the filing itself priced this path separately ("filed separately if wanted, do not block PoC-0"). Until such a re-run lands, the pinned population stands at 57/60.

Box state: shikuwa (4090 Windows) idle-CPU posture, no GPU used, parse-only instrument, zero cargo builds. Session: shikuwa-4090-i245-poc0, epoch 1796000000.
