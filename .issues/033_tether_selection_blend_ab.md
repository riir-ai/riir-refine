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

- [ ] T1: consume katgpt-core `tether` (Issue 675 T1–T2) once landed; until then inline the 3-accumulator ρ* closed form (delete on primitive landing)
- [ ] T2: `SelectionMode::TetherBlend` — score `(1−ρ)·evo + ρ·reliability`, reliability axis = BetaPosterior unchanged; ρ updated from `record_outcome` events (observe(R, p1, p2) for the SELECTED candidate), window = per-N-heals or per-run (persisted across runs via the memory envelope — pick + pin), EMA + lagged application
- [ ] T3: ρ telemetry (per-window ρ̂ + EMA value in the run log / EvolveRecorder stats) — the drift signature is part of the gate
- [ ] T4: A/B on the existing selection harness (Issue 026 T2/T4 shape, seeded evo-noise regimes, both noise levels): TetherBlend vs BetaPosterior vs LinearBlend, max metric + selection regret; in-sample never-worse asserts on recorded streams
- [ ] T5: verdict — promote if it beats BetaPosterior on the max metric in ≥1 regime without losing elsewhere; else record the honest negative (keep the variant opt-in as the artifact) + write the verdict back to Research 426 §6 R2b
- [ ] T6: G4 check — 3 accumulators, zero steady-state alloc; determinism given identical outcome streams (bit-identical two runs)

## Honest caveats

- The outcome stream density is the open question the A/B exists to answer (Appendix C: continuous ρ needs stable data; if windows are starved, the degenerate guard holds previous ρ — worst case it converges to the fixed-blend behavior, never worse).
- Outcomes are only trustworthy with the `clippy_verify` oracle on (the Issue-018 oracle split) — ρ fitting is learning-half, gated the same way the EMA direction update is.
- Not the retrieval-layer fusion (latent↔BM25) — that lever measured INERT (T3.5 sweep, `DEFAULT_LATENT_WEIGHT` doc) and stays deferred.
