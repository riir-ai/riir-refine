# Issue 190: `elo_k_eff` — evidence-damped K + thrash/stuck readout on the score-bench line

**Status:** Open — consumer lane filed (source: katgpt-rs Issue 975 T5(c); PoC G1–G4 PASS at katgpt-rs `48fd0be4d`, 2026-10-31)  
**Claimed:** zed-m3-i190, 1793427339, expires 2026-11-02T06:15:39+00:00

## What

The healer's selection plane (elo-based rule/candidate selection) gets an evidence-damped effective K plus a regime readout:

```text
elo_k_eff = K · min(1, n / n_floor)
```

- `n` = the evidence count behind the selection (per-rule outcomes); `n_floor` = the calibrated evidence floor (endpoint, consumer-calibrated — katgpt-core `min_ddr_at`'s schedule shape is the reference).
- A young rule (thin evidence) moves ratings at a damped rate — the same law as the paper's DDR (arXiv:2603.06009: evidence per unit divergence; optimum rises with budget) applied to selection churn.
- The score-bench line gains a **thrash/stuck readout** — one line: `regime: <Learning|Thrash|Stuck|Converged|CommittedWrong>` from a `katgpt_core::step_noise::StepNoiseMonitor` fed the (selection-state, score) series across the bench — so "selection churns while score flat" (damp + accumulate oracle outcomes) becomes visible against "score flat, low churn" (mine more) — Research 485's consumer-context reframe (b)/(c).

## The primitive (consume, never fork)

`katgpt_core::step_noise::{StepNoiseMonitor, Regime, min_ddr_at}` (feature `step_noise`). PoC evidence: katgpt-rs Issue 975 — six-archetype fixture classification on DEFAULT calibration, both causal-toggle arms, 28.3 ns/observe at N=8 (bar ≤100 ns), zero steady-state alloc. Monitor N = the selection-state dimension (per-rule rating vector slice; size-bounded, N=8 fits the measured bar).

## Guards

- `elo_k_eff` is a SELECTION-plane damp only — never a corpus or admission gate; the Necessity-Bench / necessity→demotion path is untouched.
- The readout is REPORT-ONLY on the score-bench line (the `--frontier-report` precedent): no exit-code change, no auto-demotion.
- Feature-off ⇒ byte-identical output (house G1); the readout line appears only with the feature on.

## Gate expectations

1. `elo_k_eff` monotone in n (≥), equals K at n ≥ n_floor, damped below.
2. The thrash fixture class must read `Thrash` on a synthetic churn-flat score series; a healthy improvement series reads `Learning` (fixture shapes: katgpt-rs `tests/step_noise_gates.rs`).
3. Score-bench stdout byte-identical with the feature off.
