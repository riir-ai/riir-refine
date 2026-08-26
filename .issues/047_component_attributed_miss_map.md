# Issue 047: Component-attributed miss map — E/ρ/W/C taxonomy for heal failures

**Filed:** 2026-08-26
**Source:** [riir-ai Research 347](../riir-ai/.research/347_recuris_verified_goal_ledger_invocation_conditional_memory.md) (Recuris, arXiv:2608.24876 — localization-by-construction + component-scoped memory evolution)
**Repo:** riir-clippy
**Kind:** POC / optimization (post-mining improvement — feeds the self-evolve loop)

## Problem

The healer records failures but never attributes them to a *component*. Three failure streams already land on disk with no component tag:

1. `.heal/misses/` — spans where no rule fired or the wrong rule won.
2. Memory trajectories with `success=false` — verify-gate reverts (compile breaks).
3. Created-warnings / score-bench deltas — fixes that introduced new diagnostics.

Recuris's measured finding (their Table 4): failure localization from structured traces reaches **64.8%** vs **13.0%** from outcome alone — and the gain is concentrated exactly on the faults a transcript cannot show (a non-fired invocation is invisible in output; only the *decision-time record* shows it). Our own precedent: **Issue 024** (cross-domain switch-cost) was a hand-run component localization — the correct rule was crowded to raw rank 9 by loop-shaped rules (a ρ-class failure), fixed by `RERANK_POOL` 8→12, verified 0.0pp both directions afterward. That investigation took a full session; the taxonomy below would have flagged ρ from the recorded artifacts alone.

## Proposal

Tag every recorded failure with exactly one component class (the Recuris E/W/ρ/C ontology, healer-translated):

| Class | Healer meaning | Existing evidence |
|---|---|---|
| **E** (corpus) | rule missing / buggy_code fixture wrong / proposer lacks the lint shape | miss with zero plausible candidates; fixture that fails its own proposer |
| **ρ** (retrieval/invocation) | right rule existed, wrong rule won — crowding, domain mis-route, pool depth, rerank miss | miss where the correct rule appears outside top-1 but inside top-k (`retrieval_eval` miss taxonomy already names crowding/coverage classes); cross-domain pairs |
| **W** (drafter context) | right rule retrieved, wrong fix drafted — context/annotation starvation, span state insufficient | parse failures; rule fired but edit reverted as semantically wrong (not compile-broken) |
| **C** (verifier/gate) | fix was good but gate rejected / fix was bad but gate passed | created-warnings (gate too loose); unnecessary reverts (gate too strict) |

Then surface the per-class counts in `self_evolve_audit.sh` §1b alongside the ELO leaderboard, so the *next* improvement targets the dominant component instead of guessing.

## Tasks

- [ ] T1 — Schema: add `component: Option<ComponentClass>` to the miss record + trajectory failure records (backwards-compatible decode = None).
- [ ] T2 — Classifier: derive the tag at record time from existing signals (rank of correct rule if known from eval fixtures; parse result; revert reason; created-warning presence). Zero-LLM, deterministic.
- [ ] T3 — Retro-validation (the POC gate): re-analyze the Issue 024 switch-cost artifacts + the Issue 027/028 false-positive clusters under the taxonomy; confirm the classes they were eventually hand-diagnosed as (ρ, E-proposer-shape, C-comment-guard respectively) are what the classifier emits. If the classifier cannot reproduce those three known answers, the taxonomy is wrong — fix before shipping.
- [ ] T4 — Audit surfacing: per-class counts + top representative spans per class in `self_evolve_audit.sh`.
- [ ] T5 — (deferred `[-]` unless T3 passes cleanly) anchor-gated patch admission: when a proposer/retriever change lands, auto-run the score-bench frozen fixtures as the no-regression half of a Recuris-style gate (repair ∧ no-anchor-regression) instead of the current manual run-after-change convention.

## Non-goals

- No LLM Meta-Agent — the classifier is deterministic pattern-matching over recorded artifacts (the paper's Meta-Agent is replaceable wholesale per their §3.4.3; our fixed procedure is the auditor script).
- No new retrieval mechanics in this issue — ρ-class counts are the *input* to a future tuning decision (pool depth, fan-out weights), not an automatic parameter search.

## Close condition

T1–T4 merged with T3's retro-validation passing; note the outcome in Research 347's consumer line.
