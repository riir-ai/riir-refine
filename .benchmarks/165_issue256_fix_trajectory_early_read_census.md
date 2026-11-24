# 165 — Issue 256 T0/T1: the fix-trajectory early-read census (instrument + honest NO-VERDICT + the capture that unblocks it)

**Status:** CENSUS EXECUTED 2026-11-25 (m3, this box's stores + history); **verdict NO VERDICT — typed, never folded** — the repo's attempt history carries no per-attempt state sequence and no negative-class support; the closure-audit arm is likewise unserved. The instrument (`examples/fix_trajectory_census.rs`) is landed, gate-tested over committed fixtures, and armed on the go-forward capture (T0c below). Claim lease: zed-8895fce6@m3 (0e166bb9).

## The question (issue 256)

Fit the shipped DoM/direction probe on EARLY fix-attempt state and measure held-out recall/precision of "will verify fail"; then run `closure_audit` (katgpt-rs 1244, Bench 994) over the chosen fix-trajectory summary. Census only — no fixer behavior touched.

## What the stores actually hold (the census half, per repo, this box)

| surface | rows | attempts (succ+fail) | fails | reverts | negative support | sequence? |
|---|---|---|---|---|---|---|
| traj store (aggregate rows, `--dump-trajectories`, this repo) | 433 | 1019 | 4 | — | 4 rows, all W-class, 3 rows | no |
| traj stores (22 sibling repos' WALs, byte-presence scan) | 22 non-empty | — | — | — | same class | no |
| `.refine/blueprint_history.jsonl` (12 repos) | 96 campaigns | 317 rule-cells | 5 reverted edits | 5 | 5 events | per-run only |
| `.refine/edit_events.jsonl` | 2 repos, 12 events | 12 | 12 | 12 | all `reverted: true` | one run each |
| `.refine/fixseq/run-*.jsonl` (the per-attempt rings) | **0 files, every repo** | 0 | — | — | 0 | **the missing surface** |
| `.benchmarks/necessity_history.jsonl` | 52 | 52 | 22 FLIPPED | — | 22, but arm-perturbation verdicts, not fix attempts | no |
| `.refine/adoption_outcomes.jsonl` | 2 | 2 | 0 | 0 | 0 | no |

**Why NO VERDICT (typed, per arm):**

- **T0 (early-read probe):** the only per-attempt outcome sequence (the fixseq ring) was never dumped on this box — zero `run-*.jsonl` in any of the 24 stores scanned; the driver dumps them on every `--fix-compile --write` run (heal.rs Phase 4) and no such run has executed here (no history shell hits; the archive boxes not scanned). The aggregate substitutes (traj rows keyed `(rule, shape, fix)` with cumulative counters; blueprint per-rule strata with 1–2 attempted edits per rule-cell) cannot express "the first ~10% of an attempt's state" — a per-cell aggregate is not a trajectory. And the negative class: 4 fails in 1019 attempts (0.4%); held-out recall over a 4-row negative class is a coin-flip statistic, not a census.
- **T1 (closure audit):** the audit needs ≥ 2 transition windows per trajectory (it needs a recorded state SEQUENCE); the aggregate stores have none. The window count would be 0 — `ClosureAuditError::NoWindows` — before any support floor.

The honest reading: the lane's premise ("trajectories + outcomes already persist") holds for OUTCOMES and aggregates; it does not hold for the per-attempt STATE sequence the DoM probe and the closure audit both consume. Bench 087 already measured the K=1 ring shape (0% multi-attempt corpora) as the horizon lane's founding refusal — this census is the store-side confirmation at box scale.

## The instrument (what landed so the census re-runs itself when the data arrives)

`examples/fix_trajectory_census.rs` (report-only, exit 0 unless a REFUSAL fires — the horizon-report law):

- **T0 arm:** LORO ridge readout over the first `max(1, len/10)` ring lines per run (the DoM "first 5–10%" discipline at ring granularity) → held-out recall/precision of "the run ends non-clean" + the two floors (constant 0.5-class + per-strategy Laplace aggregate), reusing `horizon_report::{load_runs, features_at}` verbatim (one loader, one feature law — no second copy).
- **T1 arm:** `katgpt_core::closure_audit::run` per fold — summary = the 10-D ring-native features per attempt, microstate = the 32-D `span_embed` (both columns exist in every post-T2 ring line); typed verdict `Closed | Lossy | Undecidable{reason}` read as-is (support floor 128/ε 0.05/R² floor 0.5 — the primitive's own pinned bars).
- **Typed refusals** (exit 1, never a green zero): `NoRings` (zero run files), `NoMultiAttempt` (every run is one attempt), `SingleClass` (no negative labels), plus the per-arm support columns.

Gates: `tests/fix_trajectory_census_gates.rs` — synthetic rings drive (a) the T0 probe to a known-signal answer, (b) the T1 arm through `Closed` on an orthogonal-dynamics ring and a `Lossy`/`Undecidable` on an injected one, (c) every typed refusal, (d) determinism (bit-identical stdout digest across two runs).

## The capture that unblocks the measurement (T0c — added to the issue)

The fixseq rings only appear after `cargo refine --fix-compile --write` runs (the compile-error repair lane — rustc_errors domain, invoked explicitly). The census's go-forward capture is therefore:

1. **Organic:** any future `--fix-compile --write` run on any repo auto-dumps its ring; the census re-runs across sibling stores as rings accumulate. No code needed — the dump has shipped since Issue 91 T1.5.
2. **Deliberate:** run the repair lane against the workspace's own known-compile-error fixtures (the playbook G1 fixtures exist in-tree) under `--write` in throwaway worktrees, generating multi-attempt rings honestly (real cargo checks, real keep/revert verdicts). This is the T0c follow-up, not part of this census.

## What this census is NOT

- NOT a negative result about the RET primitive — the closure audit is untested against fix-trajectory data because no such sequence data exists yet; Bench 994's fixture battery (G1 15/15) is its evidence.
- NOT a wiring decision: T2/T3 of issue 256 stay gated behind a census that clears its pre-registered recall floor, and no such census has run.

## Box state

m3 (M3 Max, AC, `powermode 0`), load 3.8/16, 28.8/64 GiB RAM; all reads against this box's on-disk stores (`~/.cargo/bin/cargo-refine --dump-trajectories` 0.6.0 @ 6ff647380a0c; the sibling WALs byte-presence-scanned, never decrypted). The capture soak probe (amo, zone-1 vessel, 2 triggers) landed exactly 1 crossing in ~55 s of sim — consistent with the driven lane's ~2 s/crossing settle + the hero's post-crossing sleep drives (bench 053's 0-of-8 organic finding, same cause), and confirms the sink plumbing end-to-end (runetrace belt + crossing line) for the T0c lane.

## Provenance

- Primitive: katgpt-rs `c80cf19e7` + `8adb663a9`, Bench 994 (G2 PASS 38.776 ms ≤ 50 ms, amo `2e226a91`).
- Issue: riir-refine `.issues/256` (claim `0e166bb9`); sibling consumer riir-ai `.issues/1259` (claimed by zed-31415479@m3).
- Research: katgpt-rs `.research/692_RET_Macrostate_Closure_Audit.md` (arXiv:2605.09294 v3); riir-refine Research 168 (probe-gated verification gap).
