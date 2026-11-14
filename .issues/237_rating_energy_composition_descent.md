# Issue 237: Selection → composition descent — the rating-energized candidate landscape cell

> **Status:** CLOSED-NEGATIVE 2026-11-15 (bench 151 `4a35b11b`, zed-07a26ed5@amo — the cell ran; **NO-GO**: arm-a compose-all 0.9643 = arm-b order-only 0.9643 byte-identical (the ordering axis is a measured NO-OP); arm-c fitted-level subset 0.3929 (Δ−32 resolutions to save one created lint); falsification HOLDS; the damage concentrates in the 24 multi-target fixtures — composition breadth is the fixpoint's fuel, not over-composition; `K_ij` = 0 disclosed, re-open only on a pair-effect-pricing consumer). Instrumentation stays report-only (`--composition-ab`, bench-only behind `score_bench`; default heal path byte-identical). The lease folded with the verdict.
> **Lease closed:** zed-07a26ed5@amo, 1794710000, expired 2026-11-17T03:00:00+00:00 — folded 2026-11-15 with the NO-GO verdict (bench 151)
> **Research:** katgpt-rs [Research 670](../../katgpt-rs/.research/670_CRAM_Pseudolikelihood_Energy_Descent.md) + [Issue 1211](../../katgpt-rs/.issues/1211_cram_score_landscape_descent.md) (the primitive)
> **Source:** arXiv:2611.17309 "CRAM"; the owner-directed fusion: "Elo score while bandit MCTS adjusts up/down trajectory at test time"

## Why

`HealPipeline::select_best_candidate` (`src/self_evolve.rs` L507; blend consts `W_EVO` = 0.6 / `W_RATE` = 0.4 at L132–136; the `TetherBlend` ρ-combine `(1−ρ)·evo + ρ·rel` at L552–566) is ONE-SHOT argmax. Ratings are refreshed by batch replay only (`TrajStore::recompute_elo`), never online. CRAM's shape adds what one-shot selection cannot produce: candidate COMPOSITIONS — descend a multi-rule candidate set one element at a time on a landscape whose input costs come from rule ratings (Elo/Beta → B_i) and whose pair costs come from co-outcome statistics in the trajectory store (→ K_ij), stopping at a fitted level, abstaining on budget. The realtime half of the owner's ask splits honestly: online ELO refresh is absent (batch replay only), but online ADAPTATION of the selection blend was already tried and lost — Bench 042.

## Named confounds (the kill-switches)

1. **Co-outcome ≠ interaction:** co-applied rules share application-order effects; the fixture ablation must disentangle pair-interaction from ordering, or the cell dies at its own gate.
2. **The incumbent is strong AND the adaptive-blend record is already negative:** (a) the blend-constant sweep ALREADY RAN — Bench 042 (2026-08-21, `src/selection_tether.rs` + `tests/t33_tether_selection_ab.rs`, Issue 033's resolution): fixed-ρ sweep over 0.0–1.0 (20k trials/cell) found no headroom, online adaptive ρ (`TetherFit`) loses elsewhere (selection regret at all three attempt budgets, paired max-metric significantly worse at evo-noise 0.6) — `TetherBlend` stayed opt-in, `BetaPosterior` the default. This cell is NOT another blend-parameter search: it changes the SHAPE (one-shot argmax pick → composition descent), which Bench 042's within-blend A/B never tested — but its prior is the Bench-042 record, never a strawman. (b) `BetaPosterior` also beat every other measured alternative (entropic Boltzmann tilt NEGATIVE, Bench 035; opus softmax banned by the sigmoid law). (c) test-time statefulness has LOST in this fleet before (pick_latch, reflex Bench 176). Loss = demote, cheap by pre-registration.

## Proof path (pre-registered; feature flag)

- A/B on the existing score-bench (the vehicle exists): incumbent one-shot vs composition-descent at equal verify budget; per-family results, never pooled; seeded + deterministic fold order.
- Falsification arm: a mis-fitted stop threshold → abstains, never wrong certifications (the P104 posture: unverified output is never certified).
- The primitive lands in katgpt-rs first (Issue 1211) as a `katgpt-assign` EXTENSION — the local-search driver already ships (Research 607/Plan 620); this repo consumes, never forks (goat-audit watches the edge).
- **Win base (owner reframe 2026-11-14; verdict-revised session 5eca383a):** this cell extends the consumed win stack — the rating substrate (BetaPosterior default; Bench 042's sweep makes this a LOW-HEADROOM prior: the cell must win by SHAPE change, not weight re-tuning), decision_wire's first-class abstention, and the conformal floor's fitting rule. Law precedent: the conformal-UQ candidate's Bench 564 (calibrated τ beats fixed τ — nearest occupied neighbor, signal-diffed in Research 670 §2.3a) and bet_search's exhaustive win. **Never-wrong here comes from the compile VERIFY step, not the stop** — the rating-shaped landscape is a noisy proxy, so the stop's claim on this surface is compute-shape (abstain/budget), never accuracy. Pillar wiring (Fourier MCTS, frame budget) is UNFILED design argument. Tracked: `../riir-ai/.docs/04_supergoat_candidates/rating_energized_descent.md`.

## Echo (no new file)

riir-dao's G5 re-open requires "a v2 candidate set or a ledger-side rule change" — a descent-composer GENERATES candidates instead of ranking proposals; if dao adopts, it files its own plan.

## Verdict (2026-11-15, bench 151 — the cell ran; NO-GO)

See `.benchmarks/151_issue237_composition_descent_cell.md` for the rows. The short form: the CRAM composition shape does not transfer from cell-to-value assignment (where Issue 1211's `descent` earned best-of-K) to rule-composition over a fixpoint — compose-all dominates the ordering axis (byte-identical) and the fitted-level axis (−57% resolutions for one created lint). Named prior (Bench 042's within-blend negative) confirmed one level up, on the SHAPE. The never-wrong property (falsification arm) holds. Closed negative; noise-reduction removal rides the next HISTORY sweep.
