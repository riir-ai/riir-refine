# Bench 151 — Issue 237: the composition-descent cell over the fixpoint's rule composition

**Status:** LANDED — **measured NO-GO** (demote the lane; the instrumentation stays report-only). Code: `4a35b11b` (`src/score_bench/composition.rs`) (+ the `heal_fixpoint_lane` seam in `mod.rs`, + `--composition-ab` CLI). Arm labels: arm-a = `LaneMode::All` (the incumbent compose-all), arm-b = `Ranked` (order-only), arm-c = `Subset` (the fitted level), plus the falsification arm.

**Provenance:** amo (Mac17,15, M5 Ultra), AC, load ~7.9 (sibling agents active — a deterministic oracle-measurement lane; every number below is byte-reproducible on any node with the same toolchain/freeze). `--release --features score_bench`, isolated `CARGO_TARGET_DIR=/tmp/i237`. The frozen v1 set (35 fixtures); the run's fresh clippy verdicts win the three drifted fixtures per the drift law (fresh names `doc_markdown` / `missing_panics_code` where the freeze named none) — disclosed, identical across arms, never a verdict input.

## Design

One landscape, zero extra oracle calls: B_i = `best_belief_score(resolved, persist+broken, 0.05)` per rule from arm-a's OWN single-target attribution on the same frozen set (calibrate on the incumbent's data — disclosed); level = the median belief over rules with data (0.2236 — five rated rules: doc_markdown 0.3684, deref_addrof/filter_next/needless_return 0.2236, map_unwrap_or 0.1354). Arms share ONE before-verdict pass (the Issue-108 cache shape) — verify budget identical by construction. `K_ij` = 0 in T1 (disclosed; the pair-cost axis deferred). Per-rule-global policy (no per-fixture adaptation — that is the `K_ij` axis). Deterministic: `total_cmp` + name tie-breaks, no RNG.

## Rows (heal_rate over Σ vocab-before = 56)

| arm | heal_rate | created | parse_safety | edits | passes |
|---|---|---|---|---|---|
| a — compose-all (incumbent) | **0.9643 (54/56)** | 1 | 1.0000 | 70 | 52 |
| b — ranked (order-only) | **0.9643 (54/56)** | 1 | 1.0000 | 70 | 52 |
| c — fitted-level subset | **0.3929 (22/56)** | 0 | 1.0000 | 27 | 24 |
| falsification (level = MAX) | 0.0000 (0/56) | — | — | 0 | 0 |

## Verdict

1. **The ordering axis is a measured NO-OP**: arm-b is byte-identical to arm-a (same edits, same passes, same rates). The fixpoint converges to the same endpoint regardless of application order; the 4-pass cap never bites on this corpus.
2. **The fitted-level composition restriction is a decisive LOSS**: arm-c resolves 22/56 (Δ−32) to avoid exactly ONE created lint (1 → 0). The below-level and un-evidenced rules' edits are load-bearing for 57% of compose-all's resolutions.
3. **Where the damage lives**: the 24 multi-target fixtures (per-rule unattributable). Every single-target per-rule family shows Δresolved 0 except map_unwrap_or (−1) — pooling would have hidden the collapse's location; the per-family read is what pins it (confound-1's discipline working as designed).
4. **Never-wrong holds**: the falsification arm resolves 0 at an absurd level; the lane cannot emit above its level (exit-1 wired at the CLI).

**Mechanism finding:** the fixer's composition BREADTH is not over-composition — it is the fixpoint's fuel. Unfired and weakly-rated rules carry edits that other rules' resolutions depend on; a rating-gated composition policy priced on single-target attribution cannot see that (the value lives in the unattributable multi-target class, exactly where `K_ij` pair statistics would have to price it — and the `K_ij` axis would inherit the same blind spot: co-outcome statistics for multi-rule compositions are unavailable at bench attribution).

**Decision (pre-registered: loss = demote):** the lane is NOT wired to the product fix path (it never entered it — the default heal path is byte-identical; the seam is bench-only behind `score_bench`). `cargo refine` composition stays compose-all. The CRAM composition shape does not transfer from cell-to-value assignment (katgpt-rs Issue 1211's `descent`, where best-of-K earned its win) to rule-composition over a fixpoint. The instrumentation + falsification row stay for replication (`--composition-ab`); re-open only on a consumer whose composition surface prices pair effects (`K_ij` ≠ 0) — the Bench-042 prior now extends from blend weights to composition shape.
