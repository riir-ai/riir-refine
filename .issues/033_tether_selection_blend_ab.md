# Issue 033: TETHER-adaptive selection blend — fit W_EVO/W_RATE from recorded outcomes (A/B)

**Filed:** 2026-08-20 (user challenge — the LOPD rescue class)
**Source:** [Research 426](../../riir-train/.research/426_Le_Critique_PVF_TETHER.md) (riir-train) — arXiv:2608.16739 "Le Critique" (TETHER). Primitive: katgpt-rs Issue 675 (`tether`).
**Class:** A/B experiment on an existing harness (poc/proof tier)

## The hit

`self_evolve.rs::select_best_candidate`:

```rust
const W_EVO: f32 = 0.6;  // "the evolved direction is the primary signal"
const W_RATE: f32 = 0.4; // "historical reliability is a strong secondary signal"
...
let combined = W_EVO * evo_score + W_RATE * reliability;
```

This is arXiv:2608.16739's baseline shape `b(ρ) = (1−ρ)·p1 + ρ·p2` with **ρ hand-pinned at 0.4 and never swept** — p1 = evolution score (learned direction axis), p2 = per-candidate reliability (BetaPosterior). The realized-outcome stream TETHER needs to fit ρ **already fires on every applied fix** (`EvolveRecorder::record_applied` / `record_outcome`), and the lag law is admissible de facto (past heal outcomes are not caused by the current selection decision).

The paper's cure, verbatim applicable: fit ρ per window by closed-form least squares against realized outcomes,

```
ρ* = clip[0,1]( Σ A_B·Δ / Σ Δ² ),  A_B = R − p1,  Δ = p2 − p1
```

EMA-smoothed (d≈0.95), applied to the NEXT window only (lagged = admissible). In-sample guarantee: `SSE(ρ*) ≤ min(SSE(p1-only), SSE(p2-only))`.

**Why this is NOT the Issue-026 entropic repeat:** entropic consumed per-candidate thin beliefs (0–3 observations each — noise amplification, its own doc records the mechanism). TETHER here fits **one global scalar over ALL applied-fix outcomes in a window** — a denser stream — and the per-candidate axis stays BetaPosterior (the thin-evidence-correct shape per the paper's Appendix C, which also retroactively explains the entropic negative result). Only the MIX becomes adaptive; the per-candidate scoring is untouched.

**Falsifiable predictions:**
1. In-sample never-worse than both fixed-weight endpoints on recorded outcome streams (exact assert).
2. ρ drifts over the memory lifecycle (cold memory vs hot trajectories) — a fixed 0.4 cannot express this; the ρ trajectory is the mechanism-of-action telemetry (paper Fig 6 signature).
3. Max-metric / selection-regret A/B vs BetaPosterior (baselines 0.9240/0.9379 from Issue 026 T2/T4) and vs LinearBlend: win, or honest negative result recorded + kept as reproducible artifact (the `gemma2_directions` / entropic precedent).

## Tasks

- [x] T1: consume katgpt-core `tether` (Issue 675 T1–T2) once landed; until then inline the 3-accumulator ρ* closed form (delete on primitive landing) — inlined at `src/selection_tether.rs` (`TetherFit`, `fit_rho`, `sse`; 8 unit fixtures). Deletion trigger recorded in the module docs (the `selection_entropic` re-export-shim precedent).
- [x] T2: `SelectionMode::TetherBlend` — score `(1−ρ)·evo + ρ·reliability`, reliability axis = BetaPosterior unchanged; ρ updated from `record_outcome` events (observe(R, p1, p2) for the SELECTED candidate), window = per-N-heals or per-run (persisted across runs via the memory envelope — pick + pin), EMA + lagged application
- [x] T3: ρ telemetry (per-window ρ̂ + EMA value in the run log / EvolveRecorder stats) — the drift signature is part of the gate
- [x] T4: A/B on the existing selection harness (Issue 026 T2/T4 shape, seeded evo-noise regimes, both noise levels): TetherBlend vs BetaPosterior vs LinearBlend, max metric + selection regret; in-sample never-worse asserts on recorded streams
- [x] T5: verdict — promote if it beats BetaPosterior on the max metric in ≥1 regime without losing elsewhere; else record the honest negative (keep the variant opt-in as the artifact) + write the verdict back to Research 426 §6 R2b
- [x] T6: G4 check — 3 accumulators, zero steady-state alloc; determinism given identical outcome streams (bit-identical two runs)

## Honest caveats

- The outcome stream density is the open question the A/B exists to answer (Appendix C: continuous ρ needs stable data; if windows are starved, the degenerate guard holds previous ρ — worst case it converges to the fixed-blend behavior, never worse).
- Outcomes are only trustworthy with the `clippy_verify` oracle on (the Issue-018 oracle split) — ρ fitting is learning-half, gated the same way the EMA direction update is.
- Not the retrieval-layer fusion (latent↔BM25) — that lever measured INERT (T3.5 sweep, `DEFAULT_LATENT_WEIGHT` doc) and stays deferred.

## VERDICT (2026-08-21): honest NEGATIVE — not promoted, artifact kept

Full numbers: [`.benchmarks/042_tether_selection_blend_ab.md`](../.benchmarks/042_tether_selection_blend_ab.md).

- **Mechanism PASSES:** in-sample never-worse holds exactly on every recorded
  stream (`SSE(ρ*) ≤ min(endpoints)` AND `≤ SSE(0.4)`); ρ drift is large and
  reproducible (0.058 oracle-evo ↔ 0.565 noise-evo — prediction 2 CONFIRMED);
  G4 zero-alloc (fit 0 allocs, pipeline delta −1); bit-exact determinism.
- **Quality FAILS (prediction 3 → negative):** paired discovery under common
  random numbers is significantly WORSE at evo-noise 0.6 (0.9207 vs 0.9239,
  McNemar z = −13.86) and only +0.0007 better at 0.2 (z = +4.29); selection
  regret worse in all three attempt budgets.
- **Why:** ρ* minimizes a PREDICTION loss; selection needs a RANKING.
  `best_belief_score` is deliberately biased low (that bias is the T9.3 gain),
  so it predicts `R` badly and ranks thin evidence well — the fit downweights
  it correctly for the loss and wrongly for the metric.
- **The sweep settles the premise:** discovery is FLAT over ρ ∈ [0, 0.6]
  (spread 0.0067, SE ≈ 0.0018) and the pinned 0.4 is the sweep MAX in the harsh
  regime. "ρ hand-pinned and never swept" was a fair criticism with a null
  answer — the constant was already at/near optimum.
- **Not the Issue 026 repeat, as predicted:** the stream was dense (≈29 000
  samples, 18 351 windows, 0 degenerate) and the fit statistically sound. The
  two negatives share only their conclusion; this one is objective mismatch,
  not noise amplification.
- **Kept opt-in** as the reproducible artifact (`SelectionMode::TetherBlend`,
  cold-start bit-identical to `BetaPosterior`). Feedback written back to
  Research 426 §6 R2b and katgpt-rs Issue 675 (new documented hazard: fit ρ
  against the consumer's own metric; a ranking consumer needs a ranking-aware
  objective).
