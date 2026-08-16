# Bench 028 — Plan 005 T9.3: Beta-posterior candidate selection

**Date:** 2026-08-16
**Plan:** [005](../.plans/005_real_file_span_healing.md) Phase 9, T9.3
**One command:** `cargo test --release --test t93_selection_regret -- --nocapture`

**Verdict: mean selection regret −8.4% in the healer's operating regime
(1–5 attempts), −23.8% at a single attempt. `SelectionMode::BetaPosterior`
PROMOTED to default.** A documented crossover reverses the sign at 30
attempts; that is a correctness check on the model, not a defect.

## What changed, and what deliberately did not

`HealPipeline::heal` selects the winning candidate with
`W_EVO·evolution + W_RATE·reliability` (`src/self_evolve.rs`). T9.3 replaces
only the **reliability** term:

| | reliability score |
|---|---|
| `LinearBlend` (was) | `S / (S + F)` — the raw success rate |
| `BetaPosterior` (now) | `best_belief_score(S, F, ε=0.05)` — the ε-quantile of `Beta(1+S, 1+F)` |

**The evolution axis is untouched.** The plan's literal wording ("replace the
blend with `select_best_belief`") would have discarded it: that primitive
returns an index from counts alone, so with every candidate unseen it
degenerates to first-index and throws away the blend's *primary* signal
(`W_EVO` = 0.6). Swapping the reliability term instead fixes the defect
without losing information — the same reading that made T3.5.5's re-ranker
keep instantiation and similarity as separate keys rather than one number.

## The defect

A raw rate cannot distinguish **1/1 from 100/100** — both are exactly 1.0 — so
a single lucky attempt outranks a long reliable history. Same family as the
small-sample flaw Issue 023 found in the conformal warm-up (a cold pool
emitting a zero-width, 0%-coverage interval).

Measured on the exact pathology (`t93_small_sample_pathology_is_fixed`):

| candidate | raw rate | Beta lower bound |
|---|---:|---:|
| lucky, 1/1 | **1.000** | 0.224 |
| proven, 18/20 | 0.900 | **0.729** |

The rate picks `lucky`; the Beta bound picks `proven`. The test asserts both,
so a regression in either direction is caught.

## Gate — selection regret

Regret is `p(best available) − p(chosen)`, which needs each candidate's TRUE
success probability. On real fixtures that is unknown — it is precisely what
the selector estimates — so measuring there would either assume `p` (circular,
the T3.5.4 mistake) or score a single draw as truth (noise). Here `p` is known
by construction and the counts `(S, F)` are *sampled* from it. Randomized
across 20 000 trials: candidate count (2–5), history depth per candidate, true
quality, and a **noisy** evolution score (±0.3 jitter — an exact one would make
the reliability axis irrelevant and rig the test).

| max attempts | linear regret | beta regret | delta |
|---:|---:|---:|---:|
| 1 | 0.0440 | 0.0335 | **−23.8%** |
| 2 | 0.0392 | 0.0317 | −19.1% |
| 3 | 0.0351 | 0.0300 | −14.4% |
| 5 | 0.0310 | 0.0288 | −6.8% |
| 10 | 0.0259 | 0.0254 | −1.9% |
| 30 | 0.0199 | 0.0205 | **+2.9%** |

**Gate (1–5 attempts): 0.0310 → 0.0284, −8.4%.**

## The crossover is a feature of the measurement

The advantage decays monotonically with evidence and *reverses* at 30
attempts. That is exactly what theory predicts — once the raw rate has
converged on the truth, a conservative lower bound is pure handicap — and its
appearance is evidence the simulation is modelling the right thing rather than
flattering the new mode.

It also bounds the claim honestly: **BetaPosterior is right for the thin-
evidence regime the healer actually occupies** (per-`(lint, fixed_code)` pairs
are seen a handful of times across a 40-lint corpus), not universally.
`SelectionMode::LinearBlend` remains available as the escape hatch, with the
crossover documented at the enum variant.

## GOAT

| gate | result |
|---|---|
| **G1 quality** | **PASS** — −8.4% regret in-regime, −23.8% at n=1 |
| **G2 latency** | **PASS** — `best_belief_score` is a LUT hot path (a few ns) for `S,F ∈ [0,31]`; the blend already ran per candidate |
| **G3 no-regression** | **PASS** — 419 / 465 / 485 / 475 lib tests green (default / `latent_retrieval` / +`clippy_verify` / `fix_verify`), clippy 0 |
| **G4 alloc** | **PASS** — allocation-free; `counts_for` is a borrow-only scan |
| **modelless** | **YES** — closed-form Beta quantile, no training, no weights |

## Behaviour change to be aware of

An **unseen** candidate now scores `0.6·0.5 + 0.4·beta(0,0) = 0.32`, not 0.5:
the Beta bound reports "no evidence" as a conservative 0.05 rather than a
neutral 0.5. That is the intended semantics, and the two `HealResult`
assertions that pinned 0.5 were updated to pin 0.32 deliberately rather than
loosened.
