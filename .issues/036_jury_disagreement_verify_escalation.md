# Issue 036: Jury-Disagreement Verify Escalation — retrieval-pool contention as a fix-instability predictor

**Date:** 2026-08-22
**Research:** [katgpt-rs/.research/499_Jagged_Judges_Epistemic_Stability.md](../../katgpt-rs/.research/499_Jagged_Judges_Epistemic_Stability.md)
**Source:** arXiv:2608.12645 "Jagged Judges" §6.2 — baseline jury majority strength is the best single-shot predictor of verdict instability under pressure (mean |ρ| = 0.59, all 84 cells negative, beating repeat-consistency and positional invariance)
**Substrate:** `src/draft/` (retrieval pool + instantiation re-ranker), `src/domains/clippy_lints/verify.rs` (`ClippyVerifier`), `fix_verify` compile gate, composite eval fixtures (`eval_fixtures_composite.rs`), `EvolveRecorder` outcome history
**Status:** OPEN (POC)

## Problem

`clippy_verify` (real `cargo clippy` re-run, ~130ms/verify) and `--fix --verify` (rustc compile gate) are our verification tiers. Today they run **globally per invocation**, not per-span by predicted risk. The Jagged Judges finding suggests a cheap pre-screen: items where an ensemble of judges *disagrees at baseline* are exactly the items whose verdicts are unstable. Translated to the healer:

> When the retrieval pool for a span is **contested** — top-K rules span multiple distinct rule families, or the instantiation re-ranker's top-2 score margin is thin — the eventual fix candidate is predicted to be unstable (compile-gate revert, verifier lint-not-resolved, or later `EvidenceTier` failure), and that span should be the one routed to the expensive oracle tier.

If the correlation holds, disagreement-gated verification cuts oracle invocations on the (large) uncontested fraction without losing coverage where it matters — the same shape as the paper's result that a single cheap snapshot replaces repeated-query protocols.

## POC (falsifiable on existing fixtures — no new harness needed)

1. **Instrument disagreement** per span on the composite eval set (`eval_fixtures_composite.rs`, 28 multi-lint spans) + the committed clippy/perf fixture corpus: two candidate signals, measured independently —
   - `pool_spread`: number of distinct rule families in the top-K=12 retrieval pool (the `RERANK_POOL` depth from Issue 024) before re-ranking;
   - `rerank_margin`: re-ranker top-1 vs top-2 score gap after instantiation re-rank.
2. **Instrument instability** per span: compile-gate reverts (fix_verify e2e harness), `ClippyVerifier` lint-not-resolved outcomes, and (offline) `EvidenceTier`/`FixTrajectory` post-hoc failures from `memory.json` history where available.
3. **Measure** Spearman ρ between each disagreement signal and each instability signal, plus the contingency at threshold (does `pool_spread ≥ 2` or `rerank_margin < ε` capture the majority of unstable spans at a small contested fraction?).

**Gates:**
- **G1:** |ρ| ≥ 0.3 between at least one disagreement signal and at least one instability signal (the LLM-judge analog is 0.59; deterministic syn-parse proposers may carry less ensemble disagreement signal — if ρ ≈ 0 everywhere, the analog is refuted and we record the negative result).
- **G2 (earn the gate):** a disagreement threshold that captures ≥ 70% of unstable spans while flagging ≤ 40% of spans → verification-cost cut with bounded coverage loss; measured on the fixture corpus, not asserted.
- **G3:** no regression on the Issue 017 retrieval floors / T3.5.5 uncontaminated G1 (the instrumentation must be read-only on the retrieval path).

## Notes

- Population caveat (why this is a POC, not a change): the paper's judges are 9 sampled LLMs; ours are deterministic proposers whose disagreement reflects corpus geometry, not sampling noise. The *principle* (baseline disagreement predicts instability) is what is being tested, per the defend-wrong discipline.
- This is explicitly NOT "pressure the judge" — the paper's net-corrupting result (flips under challenge move away from truth in 56–70% of cases) independently justifies never adding a challenge/refine loop to the heal pipeline; verification stays oracle-grounded. The disagreement signal is a passive read of the pre-commit retrieval state.
- If G1–G2 pass: wire `verify_escalation(threshold)` into the `--fix --verify` path behind a feature flag; the `EvolveRecorder` already records outcomes for the follow-up A/B (escalated-vs-global verify coverage/cost).

## Tasks

- [ ] T1: disagreement instrumentation (read-only) on the composite + fixture corpus
- [ ] T2: instability join (fix_verify reverts + verifier outcomes + memory.json history)
- [ ] T3: ρ measurement + threshold contingency table
- [ ] T4: verdict → feature-flagged `verify_escalation` plan, or record negative result and close
