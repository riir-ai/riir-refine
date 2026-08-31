# Research 083 — ELO selection challenge gate (Issue 039 T5)

**Status:** NEGATIVE (measured no-win; ELO stays report-only)
**Bench:** [047](../.benchmarks/047_elo_selection_challenge_gate.md)
**Harness:** `tests/t39_elo_selection_ab.rs` (kept as the reproducible artifact)

## The question

Issue 039's design verdict allowed ELO to challenge `BetaPosterior` for
candidate selection **only** after beating it on the same selection bench
that killed entropic (Bench 035). T5 built that bench at REAL pool sparsity
(measured from the 4 live stores: 98.5% of candidates unseen, 1.3%
single-observation) with paired-seed common-random-numbers McNemar, and
encoded the promotion rule as code (`promotion_gate`: strict win = discovery
strictly greater + `z > +1.96` + no regret regression, in BOTH evo-noise
cells).

## The measured answer (Bench 047 tables)

- `EloStatic` (the exact batch-loaded store form) — **significantly WORSE**
  (z = −8.94 at evo-noise 0.6). At real sparsity the arms differ almost
  entirely in within-trial learning; frozen ratings cannot learn during the
  3-attempt budget.
- `EloOnline` — a real but microscopic win (+0.0004, z = +3.63) at noise
  0.6, **NS at 0.2** → not a strict win.
- Dense rule-level evidence (the production aggregation asymmetry) — wins in
  the harsh-evo regime (z ≈ +4.1), NS-to-negative in the mild regime →
  regime-dependent, not robust.

**Verdict: ReportOnly.** ELO remains exactly what T2–T4 shipped: a reported
axis (audit §1b leaderboard + ruliology tie-break), no selection wiring.

## Distilled lessons

1. **Starved pools punish every selection sophistication — now with
   store-measured numbers.** The 98.5%-unseen regime makes selection
   differences ≤0.04pp; the bottleneck is evidence volume. Confirms Issue
   026's doctrine on real data.
2. **A frozen rating table is strictly worse than count-updating at thin
   evidence** — the parameterization mattered less than WHO LEARNS DURING
   THE BUDGET. Any future rating-based selection must update online.
3. **Score-bench ≠ selection-bench.** The clippy score-bench's
   `heal_fixpoint` bypasses `select_best_candidate` entirely; selection
   changes cannot move that curve. Selection gates get their own harness
   (this one, t26/t33's lineage).
4. **Ground the regime from the store, not from assumption.** t26's uniform
   0..=3 pre-observations were ~40× denser than the real distribution;
   gates run at assumed densities measure the wrong world.

## Reopen triggers

Store densification (median candidate ≥2 observations) or evo-axis
degradation — recorded in Bench 047 §Reopen condition.

> **PASS-Redirects (synthesis):** Tu, Feng, Wang, Xie, Yan [arXiv:2608.18027 "Chain-of-Experience for Continual LLM Improvement"] — independent external confirmation of lesson 1 at frontier scale: across 8 reasoning LLMs × 6 benchmarks, improvement-from-feedback correlates positively with base ability (task-avg Pearson r=0.50) and most gains land in the first ~20 iterations — the same "evidence volume, not selection math" shape this note measured at store sparsity (98.5% unseen). Also validates the L4 decline posture from the other side: their model-judge feedback only becomes competitive with a correctness oracle when the judge's base ability is strong (a weak judge yields inferior-or-negative lift) — GPT-5-mini could not help GPT-5 on hard tasks, exactly the weak-verifier regime that disarmed the local fallback lane.
