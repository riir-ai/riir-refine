# Bench 042: TETHER-adaptive selection blend — A/B vs the pinned `W_EVO/W_RATE`

**Date:** 2026-08-21
**Issue:** 033 (riir-clippy) — TETHER-adaptive selection blend; T1-T6 all closed, issue resolved-and-removed in commit `5494dbe` (this note is the artifact)
**Source:** [Research 426](../../riir-train/.research/426_Le_Critique_PVF_TETHER.md) §6 R2b — arXiv:2608.16739 "Le Critique" (TETHER baseline). Primitive home: katgpt-rs Issue 675 (`tether`).
**Harnesses:** `tests/t33_tether_selection_ab.rs` (A/B, 8 tests), `tests/t33_tether_g4_alloc.rs` (G4 + determinism, 3 tests), `src/selection_tether.rs` (8 unit fixtures)

```bash
cargo test --release --test t33_tether_selection_ab -- --nocapture --test-threads=1
cargo test --release --test t33_tether_g4_alloc   -- --nocapture --test-threads=1
cargo test --release --lib selection_tether
```

## VERDICT: honest NEGATIVE — `TetherBlend` stays opt-in, `BetaPosterior` stays the default

The mechanism is correct and its guarantee holds exactly. It optimizes the
**wrong loss for this consumer**, and the fixed-ρ sweep shows there was no
headroom to win in the first place.

| Gate | Result |
|---|---|
| In-sample never-worse (T4) | **PASS, exact** — `SSE(ρ*) ≤ min(evo-only, rel-only)` AND `≤ SSE(0.4)` on every recorded stream |
| ρ drift (T3) | **PASS** — ρ 0.058 (oracle evo) vs 0.565 (noise evo); a fixed 0.4 cannot express either |
| Max metric (T4/T5, paired) | **FAIL** — significantly WORSE at evo-noise 0.6 (z = −13.86), marginally better at 0.2 (z = +4.29, +0.0007 absolute) |
| Selection regret (T4) | **FAIL** — worse in all three attempt budgets |
| G4 alloc (T6) | **PASS** — fit allocates 0; pipeline steady-state delta −1 vs default |
| Determinism (T6) | **PASS** — bit-identical ρ across repeat runs, harness and pipeline seam |

T5's promotion rule was "beats `BetaPosterior` on the max metric in ≥1 regime
**without losing elsewhere**". It loses elsewhere, significantly. Not promoted.

## The numbers

### Fixed-ρ sweep — the sweep the constant never got (20 000 trials/cell, BUDGET = 3)

Discovery = `P(≥1 success within 3 attempts)`; ρ = 0.4 is the production blend
(bit-identical to `BetaPosterior`, pinned by `t33_fixed_rho_04_is_the_production_blend`).

| ρ | discovery @ evo-noise 0.6 | discovery @ evo-noise 0.2 |
|---|---|---|
| 0.0 (evo only) | 0.9173 | **0.9387** |
| 0.2 | 0.9215 | 0.9385 |
| **0.4 (production)** | **0.9240** | 0.9379 |
| 0.6 | 0.9227 | 0.9355 |
| 0.8 | 0.9100 | 0.9217 |
| 1.0 (reliability only) | 0.8512 | 0.8512 |
| adaptive (TETHER) | 0.9210 (final ρ 0.163) | 0.9387 (final ρ 0.063) |
| `LinearBlend` (raw rate) | 0.9170 | 0.9278 |

**The metric is flat in ρ across [0, 0.6]** (spread 0.0067 / 0.0032, SE ≈ 0.0018)
and collapses only as ρ → 1. The hand-pinned 0.4 is the sweep MAX in the harsh
regime and within noise of the max in the mild one — **the blend weight was
never a live lever here.** That is the finding that decides the issue: the
"never swept" criticism was fair, the sweep was worth running, and the answer
is that the constant was already fine.

### Paired discovery, common random numbers (200 000 trials/cell)

Unpaired ±0.003 deltas cannot beat SE ≈ 0.0018, so the verdict cell pre-draws
the pool AND a per-candidate outcome table shared by both arms — candidate `c`'s
`k`-th attempt succeeds identically whichever arm attempted it. McNemar on the
discordant pairs:

| evo-noise | adaptive | default | adaptive-only | default-only | z | verdict |
|---|---|---|---|---|---|---|
| 0.6 | 0.9207 | 0.9239 | 716 | 1345 | **−13.86** | adaptive significantly WORSE |
| 0.2 | 0.9417 | 0.9410 | 517 | 388 | **+4.29** | adaptive significantly better (+0.0007) |

### Selection regret (t93's metric, 20 000 trials/cell; lower is better)

| max attempts | `BetaPosterior` | ρ = 0 | ρ = 1 | TETHER |
|---|---|---|---|---|
| 1 | **0.0334** | 0.0373 | 0.1351 | 0.0354 (ρ 0.137) |
| 3 | **0.0295** | 0.0373 | 0.0862 | 0.0337 (ρ 0.147) |
| 5 | **0.0287** | 0.0363 | 0.0704 | 0.0317 (ρ 0.145) |

### In-sample guarantee (the paper's claim, asserted not trusted)

| stream | n | ρ* | SSE(ρ*) | evo-only | rel-only | pinned 0.4 |
|---|---|---|---|---|---|---|
| noise 0.6, default arm | 29 140 | 0.1191 | **6003.9** | 6129.8 | 12893.5 | 6704.5 |
| noise 0.6, tether arm | 29 291 | 0.1276 | **6021.1** | 6172.5 | 13094.8 | 6710.8 |
| noise 0.2, default arm | 28 513 | 0.0162 | **5396.5** | 5398.5 | 12999.8 | 6553.7 |
| noise 0.2, tether arm | 28 395 | 0.0166 | **5342.5** | 5344.8 | 13291.9 | 6550.8 |

## Mechanism of the negative (the reusable lesson)

**ρ* minimizes a calibration loss; selection needs a ranking.** OLS fits the ρ
that best *predicts* the realized outcome. `best_belief_score` is the
ε-quantile of `Beta(1+S, 1+F)` — **deliberately biased low** (that conservatism
is exactly why T9.3 promoted it: it prevents a lucky 1/1 outranking a proven
18/20). A deliberately biased estimator is a bad predictor of `R` and a good
*ranker* of thin-evidence candidates. So the fit does the correct thing for the
loss it was given — it pulls ρ from 0.4 down to 0.02–0.16, off the reliability
axis — and the wrong thing for the metric that pays: 11.8 pp of SSE improvement
buys −0.32% discovery and worse regret.

Note the sign: SSE ranks the axes `evo-only (6129) ≪ rel-only (12893)` while
discovery ranks them the other way at the extremes (`ρ=1` collapses to 0.8512,
but `ρ=0` also loses to `ρ=0.4`). The two objectives do not even agree on
direction, which is why a better in-sample SSE cannot be read as a better
selector.

This is **not** the Issue 026 (entropic) failure repeated. Entropic amplified
thin-evidence pool-shape noise; here the fit is statistically sound, the stream
is dense (≈29 000 samples, 18 351 closed windows, 0 degenerate), the guarantee
holds exactly, and ρ drift is a clean, large, reproducible signal
(0.058 ↔ 0.565 across informativeness regimes). The failure is purely
**objective mismatch**, and the two negatives share only their conclusion.

## Feedback for the primitive (katgpt-rs Issue 675)

1. **Validated by this consumer:** the closed form, the never-worse guarantee,
   the EMA/lag contract, the degenerate guard (fired once in the real pipeline
   stream — previous ρ correctly held), zero-alloc, bit-exact determinism. The
   API shape does prevent same-window application by construction.
2. **New documented hazard for the primitive's docs:** *ρ fit against a
   prediction loss is only a gain where the consumer's metric IS prediction
   error.* A consumer whose metric is a ranking / argmax (selection, routing,
   retrieval order) must fit ρ against THAT metric, or ship a ranking-aware
   objective, or expect exactly this result. `loss_grpo`'s TETHER baseline (the
   other consumer) is a value-prediction consumer, so the ρ* form fits it —
   this negative does not transfer to Plan 345.
3. The ≥2-consumer justification for the primitive now rests on `loss_grpo`
   plus the future `DualLeoMixer` adaptive-α; riir-clippy is a **measured
   negative** consumer, kept as the reproducible artifact.

## What ships

- `src/selection_tether.rs` — the inlined 3-accumulator fit (`TetherFit`,
  `fit_rho`, `sse`), to be replaced by a re-export shim when katgpt-core's
  `tether` lands (the `selection_entropic` precedent).
- `SelectionMode::TetherBlend` — opt-in, never default. Cold start is
  bit-identical to `BetaPosterior` (ρ₀ = `W_RATE`), so an unreported pipeline
  is byte-for-byte the old path.
- Telemetry: `HealPipeline::tether_stats()` (ρ, last window's ρ̂, window +
  degenerate counts) and `flush_tether_window()` for short-lived processes.
- The fit consumes outcomes only when `verified` — the Issue 018 oracle split
  applies to ρ fitting exactly as it does to the EMA direction update.
