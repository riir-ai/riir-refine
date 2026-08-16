# Issue 026 — Entropic (max-seeking) selection mode for discovery-shaped healing loops

> **Source paper:** [arXiv:2601.16175](https://arxiv.org/abs/2601.16175) "Learning to Discover at Test Time" (TTT-Discover) — distilled at `katgpt-rs/.research/019_TTT_Discover_Test_Time_Training.md` (+ 2026-08-16 addendum).
> **Prior art on the mechanism:** RS-GRPO (arXiv:2509.24261, ICLR 2026), RSPO (arXiv:2508.01174) — the objective itself is published; this issue transfers it to code-healing selection.
> **Filed:** 2026-08-16

## Context

TTT-Discover's core claim: in **discovery** settings the goal is the **single best outcome**, not average performance — so the learning/selection objective should tilt toward the max, not the mean. Its entropic objective reweights a batch of outcomes with Boltzmann weights

```
w_β(a) = exp(β·R(a)) / E[exp(β·R)]        A_β(a) = w_β(a) − 1
```

where β is set **per batch** by bisection so the tilted distribution stays within a KL budget: `KL(q_β ‖ uniform) = γ` (paper fixes γ = ln 2). Properties (paper Appendix A.1):

- **Invariant to reward shift/scale** (`r → w·r + b` yields identical advantages).
- Auto-adapts: consistent-small-improvement batches permit larger β (more aggressive); outlier-dominated batches force smaller β (prevents one lucky rollout from owning the update).
- Works on binary rewards (rare successes get upweighted harder — the RS-GRPO exploration-dilemma result).

## The gap in this repo

`SelectionMode::{LinearBlend, BetaPosterior}` (Plan 005 T9.3, Bench 028) both score the reliability axis from **history counts (S, F)**. Neither consumes the **current candidate pool's outcome distribution**. Bench 028's own honest caveat documents the failure mode: BetaPosterior pays **+2.9% at 30 attempts** — it is the wrong objective in large-sample / max-seeking regimes, and the heal loop IS max-seeking whenever the goal is "best fix found for this span/codebase" (the G6/G7 heal-rate framing, the `--fix --verify` escalation tier, ruliology candidate search).

Signal-diff (defend-wrong §3.6, one read each): `best_belief_score(S, F)` = history-only ε-quantile; entropic tilt = within-batch distribution shape. Complementary, not redundant — the paper itself stacks both (PUCT buffer carries history; entropic objective carries within-batch).

## Proposal

Third `SelectionMode` variant `Entropic` (a.k.a. `Tilted`): where a scored candidate pool exists at selection time, score candidates by the KL-budgeted Boltzmann tilt over the pool's outcome scores instead of (or blended with, behind the same knob) the history-based rate. The β solver is ~40 LOC zero-alloc (fixed-iteration bisection, `r_max` subtraction for overflow safety, stack scratch) — mirrors the paper's batch estimator incl. the leave-one-out form.

Honest scope: in the thin-evidence regime BetaPosterior stays correct (Bench 028's −8.4% selection-regret result stands, untouched). This issue experiments with the complementary regime only. Binary verifier outcomes are a valid reward signal for the tilt — no new instrumentation needed for the eval harness.

## Tasks

- [ ] **T1** — `tilted_weights(rewards: &[f32], gamma: f32) -> SmallVec`-free β solver + weight fn in `src/self_evolve.rs` (or a small `selection/entropic.rs`): bisection on β ≥ 0 to hit `KL(q_β ‖ u) = γ` within tolerance; `r_max`-subtracted exponentials; unit tests for (a) shift/scale invariance, (b) exact KL budget at convergence, (c) degenerate pools (all-equal rewards → uniform weights, advantages 0), (d) rare-success binary pool upweights successes.
- [ ] **T2** — `SelectionMode::Entropic` variant wired where a scored pool exists: the Stage-2 tuning loop's adaptive pool + the ruliology candidate ranking. Where no pool exists (plain history-only path), the variant must degrade loudly to `BetaPosterior` (one log line, mirroring the oracle-degradation pattern) — never silently.
- [ ] **T3** — Falsifiable A/B on the uncontaminated composite eval (T3.5.5 harness): metric = **per-span best outcome found within a fixed attempt budget** (the max metric — NOT average heal rate), plus the standing Issue 017 regression floors must not regress. Arms: `BetaPosterior` vs `Entropic`, same budget, ≥3 repeat runs (determinism pinned).
- [ ] **T4** — Verdict: if `Entropic` wins the max metric without regressing floors → promote as the default for pool-bearing paths only (document the split), keep `BetaPosterior` default elsewhere; if it loses → record the negative result in the bench doc (Bench 024 precedent) and close.
- [ ] **T5** — Only if T4 wins: follow-up candidate (separate issue) — PUCT-style optimistic reuse in `LatentFixMemory` retrieval (max-child-Q + visitation exploration bonus over trajectories; the moka-wasm PUCT is the in-tree reference). Not in scope here.

## GOAT gate (G-lite, no new feature flag — rides the existing `selection_mode` knob)

| Gate | Criterion |
|---|---|
| G1 correctness | Unit tests (T1) + determinism (identical selection across repeat runs) |
| G2 perf | Selection stays O(pool) with ~30 bisection iters; no heap alloc on the steady-state path (CountingAllocator spot-check) |
| G3 no-regression | Issue 017 retrieval floors + full lib test suite, both selection states |
| G4 discovery | T3 max-metric A/B — the load-bearing gate |
