# Bench 085 — Issue 093 T2: the nonergodic driver wiring (ordering seam + observer + revive), mechanism gates + e2e no-regression

**Status:** LANDED 2026-09-12 — wiring + gates GREEN; the GAIN axis is an honest NULL on every real surface available (the Bench 045 first-try-perfect class), with the family-error question now testable and the reopen trigger recorded.
**Issue:** [093](../.issues/093_nonergodic_per_run_strategy_posterior.md) T2 · **PoC:** [Bench 084](084_nonergodic_driver_poc.md) · **Kernel:** katgpt-rs Plan 592 `NonergodicFilter<K,D>`
**Feature:** `nonergodic_driver` (opt-in; NOT in the release set) · **Tests:** `tests/nonergodic_driver_ab.rs` (6/6, required-features gated)

## What landed

1. **Driver seams (ungated, generic)** — `CompileDriver` gains `with_observer` (fires at all five attempt-outcome sites with the `AttemptRecord` ring record — the in-process twin of the `.heal/fixseq/` dump) and `with_revive` (consulted for tried candidates in the retain; default unset = the shipped never-revive behavior). `driver.rs` semantics otherwise untouched; default-lane lib suite 1247/0, integration 6+6+4+4.
2. **The policy** — `src/domains/rustc_errors/nonergodic_order.rs` (feature `nonergodic_driver`): the PoC's winning arm as a rank-rotation family — menu frozen (sorted, ≤16) at the first ordering call; base ranking store-seeded (`shape_conditional_score` max per strategy) or incumbent min-delta; component `k` asserts "the strategy RANKED `k` is hot" (`P_HOT·ρ^|rank−k|`, two-state sticky {tractable, messy}); token = `menu_idx·3 + outcome` (Kept=0 / revert=1; io reverts tick nothing but still bookkeep); score = the filter's marginal P(resolved), arm-D composed `(1−λ)·store + λ·filter` with `λ = t/(t+1)` and the **stale gate** (committed non-base rotation with majority weight after ≥2 observations → λ=1 — the "D under a lying store paid 3.5pp vs C" defense). Revive: context-scoped (span-hash + file-diagnostic-set blake3 moved) AND commit moved AND `revive_gate() ≥ 0.35`, one-shot per suppression, cleared on Kept.
3. **CLI install** — `heal.rs` `--fix-compile`: when the feature is on and `RIIR_HEAL_NONERGODIC` is not `0|false|off`, the policy REPLACES the static shape-conditional ordering (installed before it; store still feeds seeding + composition). The release feature set (which does not carry `nonergodic_driver`) is untouched — verified by build.

## The honest family port (weaker than the PoC, on purpose)

Bench 084's bound #1: the PoC's components were the world's own generators (well-specified by construction); real rustc "regimes" are latent and unnamed. The rank-rotation family captures "which strategy wins here" — the ordering-relevant signal — without semantic regime structure. The messy-state transform is a uniform scale (the PoC's blunt-strategy rank shift is unmodeled). **This is the misspec T2 exists to measure.**

## Gates

| gate | verdict | evidence |
|---|---|---|
| Mechanism: determinism | PASS | two identical ordering passes identical; menu freeze stable |
| Mechanism: learning flips order (T6.7 flip-test class) | PASS | incumbent-rank-1 strategy overtakes after 3×Kept vs 2×NoProgress |
| Mechanism: revive negatives | PASS | no-record refuse; same-commit+same-ctx refuse (the failure's own tick already moved the commit — recorded post-tick, PoC order) |
| Mechanism: revive flip-window EXISTS | PASS | grid (j kept, k no-progress) ∈ {0..4}²: fired at [(0,2), (0,3), (0,4), (1,4)] — the PoC's narrow-window design confirmed on the real token arithmetic; one-shot held at every firing cell |
| Mechanism: stale-store gate | PASS | store seeds base ranking; 4 contradicting observations → committed non-base majority → gate fires; store loses its vote (order flips) |
| E2E no-regression (real cargo, equal budget 6) | PASS | borrow + e0597 scaffolds: both arms clean, remaining 0, identical applied counts; `t_observed=1` |
| E2E determinism | PASS | two fresh runs → identical attempt sequences `[E0597/e0597-anon-lifetime/kept]` |
| **G1-class: gain on a real fail surface** | **NULL (no surface)** | 0 wasted attempts on BOTH arms — the scaffolds are first-try-perfect, the Bench 045 T6.7(b) class; the N=212 corpus is the same class (its recorded 126 iterations / 0 reverts) |

## The verdict

The wiring is **inert-by-measurement-lane and complete**: every decision-content piece (learning, revive, stale gate, composition) is pinned live by the mechanism tests, and the e2e proves the driver loop is unchanged where ordering doesn't matter. The PoC's resolved-rate gain (41.7% vs 35.2/36.7) does NOT transfer to a claim here — no real surface with revert dynamics exists to measure it on:

- The playbook menus are shape-gated (one strategy fires per diagnostic in practice), so ordering choice is site-order, and site-order only wastes attempts when fixes interact — a dependency-chain surface this bench did not construct.
- The real fix-compile attempt ring (Issue 91 T1.5, `.heal/fixseq/`) started accumulating 2026-09-12; the real 1%-fail surface is exactly what it will produce.

**Reopen trigger (the family-error test this bench cannot run):** a ring corpus showing runs with ≥2 revert-class attempts — replay the A/B harness's e2e tier over those files with both orderings at equal budget; the rank-rotation family either moves resolved-at-equal-budget (GOAT G1 gain) or it does not (negative → the T3 clause: record verdict + store-densification reopen per R135/Bench 047).

## En route fix (pre-existing, Windows-only)

`orphan_report::GitCli::new` compared canonicalized scan paths (`\\?\C:\…` verbatim on Windows) against git's `C:/…` root — every pathspec dropped, the whole history leg blind; `real_git_e2e_orphan_lifecycle` failed 3/3 deterministic on this box (it landed green from macOS where canonicalize adds no prefix). Fixed by canonicalizing the root for the comparison only. Lib suite 1247/0 on Windows restored.
