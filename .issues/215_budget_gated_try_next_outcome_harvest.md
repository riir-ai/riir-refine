# Issue 215: Budget-gated try-next-on-compile-fail + outcome-contrast harvest for the fixer pool

**Status:** Open — G0 measured 2026-11-07 (Bench 142): density PASS but the K≤4 proposal-order mechanism is REFUTED (recovers 2/405; static proposer-first order reaches 341/342 at 1 compile) → G1 BLOCKED pending re-scope · filed 2026-11-07 from rethink Research 020 (BPO free-critic distillation); revised per Claude verdict round 1 (mechanism renamed honestly, ungated try-next control arm added) · pre-registered gates inside · exploratory — the pool census may validly close this negative on mechanical-lint pools

## Problem

`heal_with_pool` (self_evolve.rs:842) builds a candidate pool and ranks it in **proposal-score order**; the hosted seat picks from that order; measured compile outcome enters only as a **gate** — `ServeCompileCheck` (escalation_serve.rs:31) verifies the picked fix and a failure demotes the pick to `StoodByModelless`. Nothing tries the NEXT candidate on a compile failure, and nothing records per-candidate outcomes for the pool (no sort by compile result exists in `fix_verify.rs`; verified this session).

**Honest mechanism name (verdict round 1):** with a binary oracle and a pinned tie rule, "rank candidates by compile outcome" reduces exactly to **try-next-on-compile-fail within a budget** — try candidates in proposal order, keep the first that compiles. That is the mechanism; it is not a sibling critic. The three real deltas over current behavior:

1. **The next-candidate step itself** — today a failed seat pick is demoted to `StoodByModelless` even when candidate #2 would compile. Try-next recovers those picks.
2. **The budget gate** — spend bounded only on **contested pools** (proposal scores within a tight margin = the drafter is uncertain about the RANKING, the fixer-side form of a decision point); uncontested pools keep the incumbent path, bit-identical. Cost is capped: ≤ K candidates per gated pool (default 4).
3. **The harvest** — gated pools where compile outcomes DIFFER feed per-candidate `record_outcome`/`record_outcome_verified` rows; self_evolve's trajectory memory already consumes them, and the contrast labels are the dataset any future selection learning would train on (riir-train Plan 489's neighbor lane). BPO (2607.14171) is the inspiration: the environment's own outcome is a free critic; our binary compile oracle is that critic.

## Pre-registered gates (in order; each blocks the next)

- [x] **G0 — pool census** (Bench 142, `examples/try_next_pool_census.rs`, commit COMMIT_TBD): (a) pool≥2 405/405 v2edit · 29/29 score-bench; (b) DIFFER 342/405 = 0.844 · 28/29 = 0.966 (frozen Bench 114: 1413/1446); (c) ~208 ms/scaffold compile, try-all mean 184 ms/pool. Contrast is STRUCTURAL: 2009/2366 candidates fail to MERGE; in 329/342 differing pools ONLY the proposer member (appended LAST, index 4–5) compiles → K=4 try-next recovers 2/405, unbounded 334/405, static proposer-first (1 compile) 341/342. Not negligible → no letter-close, but G1 as written lacks the dominating arm; re-scope before G1. Miss queue empty (UNSEEN); the production seat-lane pool (`rule_book` fixed_code, no proposer member) NOT measured.
  - original text: **G0 — pool census:** over the miss queue + score-bench fixtures, measure (a) fraction of pools with ≥2 candidates, (b) fraction of those where compile outcomes DIFFER (the signal-density census — expect it LOW on mechanical lints; that is the honest number this lane lives or dies by), (c) compile cost per pool. If (b) is negligible on the dominant pools, close negative — first-pick-then-demote is already optimal for signal-free pools.
- [ ] **G1 — three-arm pick A/B (the control arm added per verdict round 1):** on contested pools, compare (a) incumbent proposal-first + demote-on-fail, (b) **ungated try-next-on-fail** (no K cap — the cheapest competitor; ours must beat or match it, not just (a)), (c) ours: budget-gated try-next (K ≤ 4, contested pools only). Bars: (c) pick-success > (a) with LB95 > 0 (paired, ≥3 seeds/folds); (c) pick-success ≥ (b) at a STRICTLY bounded per-pool cost — (b) is unbounded on pathological pools, and that cost bound is the claim.
- [ ] **G2 — cost ceiling:** branch budget counted and printed per run; wall-clock added ≤ budget × measured compile cost, disclosed in the run summary (the ⏱ phase-blame line gains a branch term only when the branch fired).
- [ ] **G3 — determinism:** same inputs → same outcome walk (compile outcomes decide; among passes the PINNED proposal-order tie rule — never timing-, hash-map-, or concurrency-dependent ordering).
- [ ] **G4 — disclosure:** every try-next pick says so in its receipt (which candidates were tried, what the outcomes were) — no silent behavior change; seat contract unchanged (a seat pick still passes `ServeCompileCheck`).

## Boundaries

- Feature-flagged and OFF by default until G0/G1 pass (`try_next_outcome_pool`, opt-in).
- The compile gate semantics (`FixVerifier::verify`, revert-on-fail, refusal counters) are untouched — this issue adds a bounded retry + harvest, never weakens the gate.
- `bench-guard` law: no perf edits to bench targets; the fixer's own measurement binaries are the harness.
- Training the selection is OUT of scope (that is Plan 489's neighbor lane via the harvested contrasts); this issue is the modelless retry + labels only.

## Cousins (verified this session)

`heal_with_pool` (self_evolve.rs:842, pool-only proposer member, `result` bit-identical to `heal`) · `ServeCompileCheck` (escalation_serve.rs:31) + the seat lane's `StoodByModelless` demotion (bin/heal.rs) — the demote point try-next replaces · `record_outcome`/`record_outcome_verified` (recorder.rs:209/:219 — success + verified + ComponentClass, the harvest sink) · `fixer_regate` arms (G1's runner) · TETHER/selection constants (`selection_tether.rs` DEFAULT_RHO 0.4 — the proposal-score side the tie rule keeps).

## References

- Research 020 §4 Track B2 (pinned claim, revised), §5 census law.
- BPO arXiv:2607.14171 §4 — inspiration (outcome-as-critic; sibling contrast in ITS regime); the compile cost per candidate measured in G0(c) plays its snapshot-cost role (BPO: 0.6–4.2% of rollout).
- APPO arXiv:2606.12384 — dual-group normalization is the K=2 degenerate form here; the pinned tie rule covers ordering among passes.
