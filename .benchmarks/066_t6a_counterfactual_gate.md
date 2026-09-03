# Bench 066 — Issue 062 T6a: the counterfactual-surgery gate (INCONCLUSIVE — the gate population is structurally starved)

**Status:** RECORD (2026-09-03)
**Issue:** riir-clippy `.issues/062` T6/T6a · **Plan:** [`.plans/070`](../.plans/070_kat062_t6a_counterfactual_surgery_gate.md) (preregistration written before the first run)
**Harness:** `src/draft/tpr_t6a_counterfactual.rs` + `tests/t062_t6a_counterfactual.rs`
**Run:** `CARGO_TARGET_DIR=/tmp/riir_clippy_t6a cargo test --release --features tpr_structure,score_bench --test t062_t6a_counterfactual -- --nocapture --test-threads=1` — 1 passed (instrument-integrity asserts green, two runs bit-identical)

## The question

T4 asked whether TPR *generalizes* (Bench 065: REFUTED — chance on unseen pairs). T6a asks the
different question the issue named: does it **interpolate usefully** on the population T6 would
actually serve — for a score-bench persist-miss span, does the zero-recompute counterfactual
rank the rule that ACTUALLY healed the span above the rules that did not, better than a
frequency prior?

## Preregistration (Plan 070 — fixed before any number existed)

- **Population:** mixed fixtures only (≥1 resolved AND ≥1 persisted vocabulary lint) — the one
  shape where a healed rule and a persisted rule competed on the same span.
- **Query:** one per (fixture, healed rule); pair = `(shape_of_span, gold)`.
- **Readout:** TPR fit on the T4a corpus at BIC's `d` (same geometry as Bench 065), fixture
  state embedded with the same `TokenBag256`, filler recovered at the shape role
  (`unbind_state_into`), candidates scored by `‖f_hat − f_Y‖` — provably the same ranking as
  the literal surgery magnitude (`‖W_r δf‖` is monotone in `‖δf‖`, `W_rᵀW_r = I_d`).
- **Candidates:** fillers with an observed pair at the span's role only — the readout is never
  asked about an unobserved pair.
- **Null:** corpus frequency per filler (outcome-blind). **Bar:** ≥ 20.0 pp (T4a/G8's number —
  not a softer gate) with the one-flip cushion.

## Result

```
corpus: n_states=113 m(roles)=6 n_fillers=45 D=256 d=1 (BIC, admissible)
fixtures: gate 1 ["composite-range_loop_plus_format_args"] (mixed) | all-healed 27 (not gated) | all-persisted 1 (no gold)
queries: gate 0 | excluded 1 (unobserved-pair 1 / role-absent 0 / gold-oov 0)
TPR top1 0.0% | frequency-prior top1 0.0% | TPR MRR 0.000 | margin +0.0 pp (bar ≥ 20, one flip = inf pp)
verdict: INCONCLUSIVE
persisted-lint census: map_unwrap_or: 1, useless_format: 1
```

**INCONCLUSIVE — the gate population is empty, and the emptiness is structural.** Of the 29
fixtures with vocabulary lints at HEAD (heal rate ~98%, runs #3–#30 of the score series), **27
heal completely** — there is no persisted rule to contrast a healed rule against. The ONE mixed
fixture (`composite-range_loop_plus_format_args`) carries exactly one gate-eligible gold, and
its `(shape, gold)` pair is **unobserved** in the corpus — the readout Bench 065 measured at
chance. A number there would be a prediction, which the preregistration forbids shipping.
Per the preregistered branch table, an empty gate population is INCONCLUSIVE, never FAIL —
instrument/population failure cannot close a route (T4a amendment A1's one-directional rule).

## The finding that outlives the verdict

The persisted-lint census is the sharpest output: the persist population T6's attribution would
serve is **`map_unwrap_or` + `useless_format` — both documented we-stay-silent divergence
classes** (the array-literal-default guard and the String-typed `useless_format` boundary,
Bench 031 era). There is no candidate rule that COULD heal them; the healer is silent on them
by design, with fixtures recording the divergence. T6-as-attribution would be asked to explain
misses the corpus deliberately cannot answer — the counterfactual has no gold to interpolate
toward even in principle on this population.

## Consequence

- **T6 stays UNWIRED** — the gate did not pass, and the measurement says the frozen score-bench
  population cannot host it at current healer quality. The attribution artifact is not built.
- **Reopen condition (recorded, not scheduled):** a mixed-fixture population of useful size —
  either the frozen bench grows real persist-miss fixtures with in-corpus gold pairs, or the
  population moves to the `.heal/misses/` real-repo trajectory store where gold comes from a
  later-healed trajectory of the same span (a different instrument, needing its own
  preregistration — not a widen-and-retry of this one).
- The readout machinery (`run_t6a`, `FixtureOutcome`, the report) ships as the standing
  instrument; re-running it is one command when the population changes.
- katgpt-core `tpr` stays opt-in (unchanged from Bench 065's consequence — T6a was the last
  candidate consumer route).

## Honesty notes

- The all-healed 27 could have been admitted as gold-bearing queries (which would very likely
  have produced a large margin — the healer heals exactly what the corpus describes). That
  amendment could only STRENGTHEN the conclusion, so it is forbidden after the fact (T4a A1's
  rule); it is recorded here so nobody "fixes" the gate into a PASS later.
- `d = 1` (BIC's pick) is the geometry where a filler is a scalar — the same structural handicap
  Bench 065 A2 documented. The diagnostic entry point (`run_t6a_at`) exists for follow-up
  questions; the gate itself runs at BIC's `d` by preregistration.
- The test asserts instrument integrity + determinism only; the verdict is printed, never
  asserted — a FAIL/INCONCLUSIVE must not become a red suite that incentivizes widening.
