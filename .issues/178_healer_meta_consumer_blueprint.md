# Issue 178: Healer Meta-Consumer — Blueprint-Level Lever Tuning (the Harness-Evolution Meta Level)

**Status:** Open — P2, pointer recorded; no implementation claimed
**Claimed:** zed-amo-178-meta, 1793036009, expires 2026-10-28T17:33:00Z (scope: the first step — Research 401 §7 + Plan 642 Phase 2 read + the `.proposals/` go/no-go proposal; implementation is NOT in this lease)
**Date:** 2026-10-27
**Origin:** riir-ai [Plan 642](../../riir-ai/.plans/642_harness_evolution_autoplay.md) T4.3 pointer + [Research 401](../../riir-ai/.research/401_Harness_Evolution_Game_Autoplay_Guide.md) §7 row 2 (arXiv:2604.21003 — "The Last Harness You'll Ever Build")

## The observation

The healer ALREADY runs a Level-1 evolution loop: `--fix` proposes,
`fix_verify` compile-gates (improved/regressed verdicts), `self_evolve`
records trajectories. What it lacks is the paper's META level: a
blueprint Λ over the loop's own knobs, tuned ACROSS repair campaigns
under guards.

## The proposed shape (record, not commitment)

Blueprint Λ = (lint-class priorities, verify budget, corpus lease
cadence, escalation-lane thresholds) — the healer analog of the game
runtime's (mutation-arm knobs, n_variants, n_generations, ESC intensity,
lever bounds). Selection machinery would REUSE the shipped substrate —
`hint_regret::beta_lcb_order_into` for arm memory, the arena
generations' arm-table pattern (riir-scenelab `arena_generations::ARM_TABLE`
+ `GenerationHistory`) as the structural template — never a new LCB or
selector.

## The guards (non-negotiable, the dao pattern)

Never-levers: the bounds themselves, the never-lever list, KAT pricing,
consent states. Acceptance margins: a blueprint wins only by a
Wilson-LB/strict-superiority margin over the incumbent on held-out
repair campaigns (families it never tuned) — never by aggregate
volume. Intensity: ESC-gated (regression-streak latching demotes to the
static corpus).

## Why an issue and not a plan

The game runtime is the fusion's priority-#1 surface (Plan 642); the
healer is priority #2. This issue parks the pointer so the meta level's
SECOND consumer is on record — which is also the condition that would
un-block katgpt-rs Plan 631's conditional primitives (`BestKeeper`/
`MutationArms` need ≥2 committed consumers; the healer committing HERE
would be the second).

## First step when picked up

Read Research 401 §7 + Plan 642 Phase 2, then a proposal
(`.proposals/`) — the question is "should the healer run a meta level
at all", not "how".
