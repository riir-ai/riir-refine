# Plan 070 — Issue 062 T6a: the counterfactual-surgery gate (TPR interpolation on persist-miss spans)

**Status:** COMPLETE (2026-09-03) — verdict INCONCLUSIVE (Bench 066); reopen condition recorded
**Issue:** riir-clippy `.issues/062` T6/T6a (T4 closed REFUTED by Bench 065; T6 re-scoped to a measurement before any consumer wiring)
**Predecessors:** Bench 065 (T4a withheld-pair gate), Bench 063 (admissibility), Bench 032 (switch-cost — the re-run-based measurement T6 complements)

## Preregistration (written BEFORE the first run — no number informed these choices)

**The question (T6a):** does a zero-recompute counterfactual readout — fit the TPR
artifact on the healer corpus, then score a persist-miss fixture span against each
candidate rule at the span's own shape role — rank the rule that ACTUALLY healed the
span above the rules that did not, better than a frequency prior?

**Population (fixed here):** the frozen score-bench fixtures, freshly run at HEAD
(`sb::run_bench` — the same outcomes the score history records). A fixture enters the
GATE population iff it has BOTH ≥1 resolved vocabulary lint (the gold healer(s)) AND
≥1 persisted vocabulary lint (the hard negatives). All-healed fixtures are reported
but not gated (their non-gold candidates never competed); all-persist fixtures have no
gold and cannot enter.

**Query (fixed here):** one query per (fixture f, healed rule g): the counterfactual
pair is `(shape_of_span(f), g)` — exactly the pair the Issue 062 T6 text defines
("surgery-swap the span's filler to rule Y, producing the pair (shape_of_span, Y)").

**Readout (fixed here):** fit TPR on the T4a corpus (`clippy_lints` + single-lint
fixtures + Plan 049 real-repo spans, `TokenBag256`, role-coalesced at `T = 8`) at
BIC's chosen `d` among admissible grid points — the SAME geometry Bench 065 scored,
so the two benches are comparable. Embed the fixture span with the same
`TokenBag256.embed_into`. Recover the span's fitted filler at its shape role
(`unbind_state_into`) and score candidate rule Y by `‖f_hat − f_Y‖` — the filler-space
distance. This IS the closed-form surgery magnitude: for orthogonal schemes
`‖surgery edit‖ = ‖W_r(f_Y − f_hat)‖` is monotone in `‖f_Y − f_hat‖` (`W_rᵀW_r = I_d`),
so the surgery phrasing and the distance phrasing produce the IDENTICAL ranking.

**Candidate set (fixed here):** per fixture, the fillers with an OBSERVED pair at the
span's shape role — `(s_f, Y) ∈ corpus pairs`. The readout is never asked about an
unobserved pair; unobserved candidates do not dilute the ranking (and the issue
forbids shipping numbers about them anyway).

**Observed/unobserved split (fixed here, per query):** gold g is an OBSERVED query
iff `(s_f, g)` is a corpus pair (then g is in the candidate set). Gold rules whose
pair the corpus never observed are counted as UNOBSERVED queries, reported with their
count, and NEVER enter the gate — Bench 065 measured that readout at chance.

**Null (fixed here):** the frequency prior over rules — corpus state count per filler
(`filler_states`), outcome-blind (it never sees which rules healed which fixture).
Prior hit = the most frequent candidate is a gold rule. Ties break by lower filler id
(deterministic; `total_cmp` for the distance argmin the same way).

**Gate (fixed here):**
- `margin_pp = 100 × (TPR top1 − prior top1)` over the gate queries (a query hits
  when the argmin candidate is ANY of the fixture's healed rules).
- **PASS** — `margin_pp ≥ T6A_MARGIN_PP` AND `margin_pp − 100/n_queries ≥
  T6A_MARGIN_PP` (the one-flip cushion, same shape as T4a's).
- **FAIL** — `margin_pp < T6A_MARGIN_PP` on a readable instrument. Closes T6 the way
  T4a closed T4: no widen-and-retry.
- **INCONCLUSIVE** — margin clears the bar but one flip crosses it; or the gate
  population is empty (no mixed fixtures, or every gold query unobserved); or the
  instrument itself is unreadable (no admissible `d`, fit error, empty role map).
- `T6A_MARGIN_PP = 20.0` — deliberately the same bar as T4a's `T4A_MARGIN_PP` and
  katgpt-rs Issue 707 G8, so this gate is not a softer version of its family.

**Instrument-integrity asserts (fixed here):** the fixture's shape role exists in the
coalesced vocabulary (counted as role-absent otherwise, never silently dropped); gold
lints outside the fit vocabulary are counted as out-of-vocabulary golds; the three
query populations (gate / unobserved / role-absent) are reported and sum to the gold
query total.

## Tasks

- [x] Plan + preregistration (this file) before any run.
- [x] `src/draft/tpr_t6a_counterfactual.rs` — the gate module (pure: takes
      population + fixture outcomes, no score_bench dep).
- [x] `tests/t062_t6a_counterfactual.rs` — the harness: fresh `sb::run_bench`
      outcomes at HEAD + the T4a population; instrument-integrity asserts; verdict
      PRINTED, never asserted (the T4a contract — a FAIL is a legitimate outcome).
- [x] Cargo.toml test row (`required-features = ["tpr_structure", "score_bench"]`).
- [x] Run it once, record the verdict + numbers in `.benchmarks/066_t6a_counterfactual_gate.md`.
- [x] Update `.issues/062` (T6a row), AGENTS.md, mining-queue snapshot.
- [x] Commit + push. (`9c0807b`)
