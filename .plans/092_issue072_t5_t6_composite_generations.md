# Plan 092 — Issue 072 T5/T6: composite exactness ladder + generationed bench ratchet + frontier-ordered authoring queue

**Status:** COMPLETE (2026-09-06, same session; commit lands with the issue
verdict + Bench 069). All tasks done; the M5 admission exercised live and
honestly REFUSED generation 2 (pre-registered branch B) — the machinery is
landed + pinned, the difficulty-axis reopen is recorded in Bench 069.

**COLLISION ADJUDICATION (Issue-665 class, final):** a concurrent session
executed the same unit from their
`.plans/091_issue072_t5_t6_composite_and_generationed_bench.md`
(three commits on origin mid-unit; their plan file also dual-allocated the
091 number this plan's renumber created). Adjudicated per push-wins + the
superset rule, surface by surface:

- **SUPERSEDED (theirs):** their T1.1 queue listing (`read_misses` +
  `frontier_order_misses` + `--misses` code/verdict/path lines — this
  plan's `frontier_queue_report` is the superset: store outcomes via
  `code_outcomes_from_rows`, entropy/regime/conversion columns, the
  in-queue can-never-be-saturated insight) and their T4 detector
  (window-mean/`Moving` variant — the shipped `detect_saturation` is
  per-generation, which the admission gate requires, and is the form the
  live bench #46 SATURATED verdict measured). Their plan file removed as
  the duplicate; their `--misses-report`-shaped flag never shipped (their
  `--misses` + `--misses --report` surface won the flag collision per
  push-wins and is the unified surface).
- **KEPT (theirs, complementary):** their T1.3 conversion ledger
  (`WIRED_STRATEGIES`/`FRONTIER_LANDINGS`/`conversion_report` + the
  `wired_plus_frontier_equals_the_derived_inventory` ratchet test + the
  playbook `code()` dedup) — the era-split G8 instrument this plan's T6
  described but did not build; composes with the queue report (the
  `--misses` flag prints this plan's frontier table, `--misses --report`
  prints their ledger).
- **SALVAGED (notes only, into Bench 069):** the rustc-errors-side
  composite ladder design; the window-mean/signed detector variant; the
  historical-FIFO baseline discipline.

Source: riir-clippy `.issues/072_frontier_evidence_challenger.md` T5/T6 (J-Zero M4/M5;
arXiv:2608.26582), mechanism specs in riir-train `.research/450_J-Zero_Judge_Co_Evolution_Gain.md`
§4.1, PoC complete in Plan 091. Resolves Plan 014 Phase 3's deferred "real-fraction
growth" — the mechanism it was deferred for lacking now ships.

## Pre-registered branches (measurement-first)

The M5 admission gate is non-vacuity: **v2 is admitted only if it measures harder than
v1** (first v2 heal_rate strictly below the latest v1 rate). Two outcomes are legitimate
and both are pre-registered:

- **(A) Decay holds** — per-k solve-rate is non-increasing → v2 admitted, floors begin,
  the ratchet loop is live end-to-end.
- **(B) Flat** — the fixpoint applies ALL proposer edits per pass (`collect_edits` over
  the whole file), so k-composites of proposer-covered lints may resolve at v1's rate →
  the gate refuses admission, the machinery stays landed + pinned, and the recorded
  finding names the difficulty axis the ladder needs (persist-shape stratification /
  interference-aware composition). This is the honest negative branch; the mechanism
  (detector + admission + per-generation series) is still the deliverable.

## Tasks

- [x] **T0 — Racing pre-check.** Tree clean at the renumber commit `b8d30e73`;
  no sibling hunks on the write set.
- [x] **T1 — M4 k-ladder module** (`src/score_bench/generations.rs`, feature
  `score_bench`). Atom factory = the clippy corpus itself (construction-first, zero new
  authored code): per vocabulary lint, the first corpus entry (sorted-(lint, code)
  order) whose `buggy_code` parses standalone as a file. Composites wrap each atom in
  its own `mod gen_{i}` (zero name collisions, zero cross-atom interference by
  construction); deterministic stride-window selection (no RNG); ids `gen2-k{k}-{lints}`;
  `generation_v2()` + `k_of()` + `v2_freeze_status()`. 6 module gates (determinism,
  wrapping, union identity, id uniqueness, k-parse, freeze-coverage both states).
- [x] **T2 — Freeze/verify e2e** (`tests/generation_v2_freeze.rs`, `#[ignore]`d +
  `required-features = ["score_bench"]`). **RAN: 38 atoms (2 bad), 66 specs, 8 union-
  identity exclusions (all cascading from the 2 bad atoms — zero independent
  interference), 58 verified 58/58 vs real clippy; per-k solve rates 100.0% FLAT at
  every k → branch (B).** Freeze record landed (`FROZEN_LABELS` 58 + `EXCLUDED_IDS` 8);
  the e2e re-run after landing: zero drift. Full measurement: [Bench 069](../.benchmarks/069_issue072_t5_t6_generations.md).
- [x] **T3 — M5 saturation detector** (`report.rs`): `detect_saturation(records,
  generation)` + `SaturationVerdict` + `saturation_line`; WINDOW=5, EPSILON=0.005 strict;
  unit gates: rising → Active, flat → Saturated (exact-0.0 marginal), exactly-ε boundary
  (saturates below, active above), short → InsufficientHistory, per-generation filter,
  all three render shapes.
- [x] **T4 — CLI + record + ratchet**: `HistoryRecord.generation: u8` (serde default 1;
  `ScoreRunRow` mirrors it — the jsonl↔store schema-identity test extended);
  `--generation <1|2>` (else exit 2; requires `--score-bench`); gen-2 runs the ladder
  behind the non-vacuity admission gate; per-generation SVGs
  (`score_history.svg` / `score_history_v2.svg`); saturation line after every run; per-k
  solve-rate line on gen-2. **Exercised live: admission REFUSED (v2 1.0000 ≥ v1 0.9623,
  exit 1, nothing appended — JSONL length verified unchanged); bench #46 (v1) green with
  the SATURATED verdict line.**
- [x] **T5 — `.heal/misses/` frontier wiring**: `evolve::parse_miss_code`,
  `code_outcomes_from_rows` (store/envelope-agnostic), `frontier_queue_report` (pure;
  per-code pools = store kept/reverted + Missed × queue occurrences, ordered via
  `dispersion::frontier_order`, renders rank/code/misses/kept/reverted/entropy/regime/
  conversion); CLI `--misses-report` (store-first, envelope fallback, `rustc_errors`-
  gated); 4 unit gates incl. the planted-queue ordering pin + the honest empty-queue
  line. In-queue insight pinned in-test: a fully-saturated pool can never occur in the
  queue (every entry IS a miss) — all-kept store history reads as LOW-entropy informative.
- [x] **T6 — Real-G8 instrument + gate shape.** The report's conversion column IS the
  per-authored-rule Certified-conversion measure; the A/B gate shape recorded in issue
  072; the measurement DEFERS until the queue has entries + ≥1 template is authored
  (reopen condition in the issue — the queue is empty today, exercised exit-0 honest).
- [x] **T7 — Validation.** lib default 1116/0 (1112 + 4 new evolve gates); 4-feature
  union 1293/0/8i; `generation_v2_freeze` always-on 1/0/1i; clippy `--all-targets` 0
  real findings in the union state; bench #46 count-identical (FLAT 96.2/1.9/100/100);
  the gen-2 admission exercised against the LIVE history (refused, nothing appended);
  fence e2e untouched (frontier_pairs unchanged).
- [x] **T8 — Docs + verdict.** [`.benchmarks/069_issue072_t5_t6_generations.md`](../.benchmarks/069_issue072_t5_t6_generations.md)
  (freeze measurement + per-k table + the live refusal + the difficulty-axis reopen
  candidates); AGENTS.md row; issue 072 T5/T6 checkboxes + verdict; commit + push.

## Non-goals

- No auto-admission — v2 admission is a deliberate operator step (`--generation 2`);
  the gate only refuses un-harder generations.
- v1 `FIXTURE_DATA` untouched — the frozen set stays frozen forever.
- No selection-math changes inside `heal()`; no GRPO; EvidenceTier untouched.
- The cgsp scenario-dispersion twin stays `[-]` (issue 072's own defer).
