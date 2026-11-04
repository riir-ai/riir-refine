# Issue 205: Campaign failure decomposition — grammar record + recovery vocabulary + verify-referee prune (LEAP × Research 649 fusion)

**Status:** open — fusion idea (Research 649 × LEAP Plan 215; sweep record `katgpt-rs/.research/650_Probe_Grammar_Consumer_Fusion_Sweep.md`), novelty TBD, PoC unrun; verdict round 1 REVISE addressed (base-rate + seed counts pinned, held-out arm added)
**Date:** 2026-11-04
**Repo:** riir-refine (the fixer lane) — refine-side only; the rethink ESC mapping was judged weak (receipts-only, `esc_voi` value legs blind) and is excluded from scope
**Filing analysis:** 10-analyst fusion sweep, 2026-11-04; load-bearing facts coordinator-verified (`RefusalTally` in `src/fix_verify.rs`, run-end roll-up in `src/bin/heal.rs` Plan 329 P2, `EscalationDispatch::arm` in `src/escalation_dispatch.rs`)
**Sweep record:** `katgpt-rs/.research/650_Probe_Grammar_Consumer_Fusion_Sweep.md` @ katgpt-rs `e7a746274` (2026-11-04)

## The idea (what × what)

LEAP's AND-OR strategy decomposition + δ-mem cached sub-strategies + acceptance-rate
dead-end detection, applied to the fixer lane's FAILURE space, structured by Research
649's three-stage shape (probe → grammar record → behavior-verified discrete emission):

- A failed `cargo refine --fix` campaign decomposes AND-OR into sub-failures keyed by
  the shipped `RefusalTally` taxonomy (`spawn_failure / cargo_refusal / incomplete_check
  / baseline_broken / no_manifest / write_failed / loop_exhausted` — `src/fix_verify.rs`,
  printed at run end per Plan 329 P2).
- Each sub-failure gets a 649-style **grammar record**:
  `fail ↚ drive[rule | stage-gate] —wire(recovery-word, span)→ gap[rejected edit]` —
  BLAKE3-digested, ADDITIVE row (traj-store extension or a blueprint-history v2 mode
  row; the artifact format is additive-by-design).
- **Recovery vocabulary** (discrete words, not free-form retries): `SplitEdit /
  NarrowSpan / AddContextNotes / EscalateSeatRerank` — the last delegates to the
  existing `EscalationDispatch::arm`; no new escalation machinery.
- **Emission** = ascending word sweep, first compile-gated clear (mirror 649's
  `emit_word`) — **grammar proposes; verify disposes**: the fix_verify loop IS the
  dose-sweep referee, already shipped.
- LEAP's dead-end signal lands as a **per-campaign verify pass-rate EMA prune** —
  in-campaign only (the mutated state is the in-memory branch ledger; campaign end
  appends ONLY to ledgers that already exist: blueprint history row, traj store).
  Stationary discipline: no EMA actuator outside a multi-attempt campaign.
- δ-mem analog: solved sub-repairs cached keyed `(rule × stage × shape_hash)`, read via
  the existing `shape_conditional_score` — consume, never fork.

## Pre-registered demotion law (runs BEFORE the PoC)

- [ ] **Base-rate gate (pinned):** over the most recent ≥20 fix campaigns in
      `--blueprint-report` history (+ traj store): if fewer than **5%** of campaigns
      record ≥1 `RefusalTally` event or a loop-exhausted outcome, the lane folds
      pre-PoC, recorded negative. No PoC on a below-floor population.
- [ ] **Held-out coverage arm (from the gate's harvest):** the real refused /
      loop-exhausted campaigns the base-rate gate collected are scored by the FRACTION
      any recovery word clears — a coverage measurement, reported honestly with no
      pass bar (it quantifies vocabulary reach on real failures, not fixture-authored
      ones, and feeds the attribution claim).
- [ ] **Fixture:** one corpus entry seeded to fail each targeted `RefusalTally` class
      (P104 unverified-kept, E0433-class path spelling, baseline-broken); assert flat
      retry fails and exactly ONE recovery word clears. This arm proves the MACHINERY
      (parse → record → sweep → referee); vocabulary coverage on REAL failures is the
      held-out arm's job, never this one.
- [ ] Grammar-record writer + stale-refusal test (campaign-epoch/digest keyed).
- [ ] Recovery-vocabulary enum + ascending sweep + determinism assertion.
- [ ] Campaign EMA prune GOAT bar (seed counts pinned): ≥12 recoverable + ≥6
      unrecoverable seeded campaigns; **0 false prunes across ALL recoverable seeds**;
      detection ≥50% of unrecoverable seeds (≥3 of 6); overhead ≤2× flat-retry verify
      cost.
- [ ] Blind baseline: flat-retry vs decompose on the seed set (649 P3 framing — report
      eval cost + attribution honestly; any headline multiple is order-dependent).
- [ ] Probe-half honesty note in the record: clippy spans/verify diagnostics are a
      COARSE proxy for 649's activation taps — what transfers is the emission
      structure, not the probes.

## Proof path

`tests/campaign_decomposition_gates.rs` (fixture + referee arms + canary) + one
`.benchmarks/` record; `cargo refine --fix` e2e over the fixture; word-sweep
determinism asserted.

## Closest cousins (signal-diff'd)

- `--blueprint-report` campaign audit — reads ledgers, never mutates; this issue adds
  the in-campaign writer.
- `RollingRateLatch` (consumed here — imported from `katgpt_core::escalation_guard`) /
  rethink `EscRateGuard` — latch on escalation FREQUENCY; this prunes on
  verify-OUTCOME rate — different state, different actuator, no collision.
- blueprint `demotion_suggestions` — report-only, names the command; never executed.

## Prior art

LLM-APR diagnosis-driven retry is pervasive (arXiv:2506.23749 taxonomy; Xia & Wei
ISSTA 2024) — adjacent, not killing; nothing found naming a verify-stage-gated discrete
recovery vocabulary with acceptance-rate dead-end pruning of the decomposition.
