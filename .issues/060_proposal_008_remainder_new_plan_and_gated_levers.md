# Issue 060 — Proposal 008 remainder: autonomy re-open, dopamine levers, advisory soak

Status: **RESOLVED (2026-09-04) by owner decision — see
[Issue 068](068_kat_launch_tier_decision.md). Advisory-only is 008's SHIPPED END STATE,
not a waiting room.** The G5 verdict is FAIL (wash), measured and pinned; the decision is
to **not commission** the v2-candidates plan on a calendar, to **kill** the dopamine
levers rather than keep them deferred, and to move the advisory soak onto the devnet rung
(068 D7/D8) that unblocks it. This file is kept rather than removed because sibling repos
cite it by number; it is now a record, not a wait-state tracker.
Branch: `develop` (per global rule — no feature branches)
Proposal: [`.proposals/008_adaptive_tokenomics_riir_dao.md`](../.proposals/008_adaptive_tokenomics_riir_dao.md)
Decision: [Issue 068](068_kat_launch_tier_decision.md) · Related:
[Issue 059](059_proposal_007_remainder_ops_signer_and_counsel_gate.md),
[Issue 061](061_proposal_011_product_loop_rollout.md), riir-dapps `Issue 006` (CLOSED —
the decay-saturation resolution), Proposal 010 (the landed re-open half).

## Decisions applied (2026-09-04)

- [x] **The NEW plan (the only path to autonomy) — DO NOT COMMISSION** (068 D4).
      Commissioning a v2-candidates campaign today buys a re-measurement whose only
      evidenced outcome class is another wash, and it would need an owner **supply-ratio**
      decision made on no data. The measurements, re-verified at source on 2026-09-04:
      `riir-dao/.benchmarks/001_g5_simulator.md` reads *"**FAIL — no metric where adaptive
      strictly beats frozen (wash)**"*, carrying a regression pin that asserts the verdict
      stays exactly that; Bench 004 axis-E adds that no agent-reachable `decay_factor`
      keeps mining alive to a 1000-epoch horizon at launch scale under any cap.
      **So the landed system is the answer, not a shortfall:** it is bounded, audited
      (`dao-audit`), and a frozen constant performs as well as anything adaptive we have
      tried. That is a recorded negative result — treating it as unfinished work is how
      someone re-runs a campaign we already paid for.
      **Re-open trigger — a MEASUREMENT, not a calendar:** the Tier-0 soak (below) shows a
      metric where a frozen constant is measurably leaving value on the table. Let the
      data commission the plan.
- [x] **T3.1 `FairRoll` jackpot table as a governed lever — WON'T-DO** (068 D5), not
      deferred. Two concrete reasons: (1) the guard layer already pins
      `REWARD_EV_EPSILON_BP = 0` — *"EV ≤ base exactly, the strictest reading"*
      (`riir-dao/crates/riir-dao/src/guard.rs`, asserted in
      `tests/dapps_contract_pins.rs:177`, with `REWARD_MAX_MULTIPLIER_X = 10` at :176), so
      a jackpot table with **zero EV edge is a presentation layer over a constant** with
      no economic function by construction; (2) gambling-adjacent mechanics and copy on a
      token we have just decided to keep **non-transferable and closed-loop** (068 D1) add
      exposure to precisely the surface 005 caveat #15 calls "most likely to be
      screenshotted". The guard-side caps STAY (they bound anything future); the levers do
      not get wired.
      **Re-open trigger:** only inside a future proposal that first shows, measured, a
      retention metric the levers are supposed to move — with the copy posture re-checked.
- [x] **T3.2 streak bonuses — WON'T-DO**, same gate and same reasoning as T3.1.
- [x] **T2.3 — advisory-mode soak. UNBLOCKED and RE-HOMED to
      [Issue 061](061_proposal_011_product_loop_rollout.md)** (068 D7/D8). It was blocked
      on live telemetry → blocked on the genesis ceremony → an owner act. The devnet rung
      breaks that chain: a local ceremony produces a real cluster emitting real epochs, so
      the soak runs there. A devnet soak is **not** a substitute for a testnet one for
      *launch* purposes, but it is a full substitute for the purpose this row stated —
      proving the advisory loop runs N epochs clean and the guard layer does not trip on
      real telemetry. It is also the measurement that could re-open the v2 plan above.
- [x] **T4.1/T4.2 — the autonomy ramp. CLOSED (terminally foreclosed)** (068 D6). Already
      terminal-as-specified by the G5 verdict; recorded as closed rather than left
      perpetually "open, gated". Re-opens only inside a future proposal, never inside 008.

## Landed (record — do not re-do)

- T0.1 dao scaffold + boundary gate (`1de05ba`, `4f24d8d`, registered in the riir-ai
  matrix `e51040213`); T0.2 dapps `EpochParams` + apply-time bounds + governance-intent
  signing (`80a9aef`, 21 G6 tests); T0.3 telemetry rows (same commit, golden-pinned
  both sides).
- T1.1–T1.4: signals, BetaPosterior/Elo strategy selection over `katgpt_core::rating`,
  guard layer, simulator + G5 campaign (FAIL-wash verdict, adversarial half PASSED,
  5 defects fixed en route).
- T2.1 GLM advisory transport + guard re-check proven by test (`1ae80d8`); T2.2
  decision artifacts + G1 bit-identity + `dao-audit` CLI (same commit). The live GLM
  smoke remains an OPS step (creds) — the mode + the model-change-requires-re-smoke
  rule shipped.
- T2.4 shallow-DAO voting: dapps lifecycle `4cae5f2` + clippy CLI `3346470`
  ([Bench 005](../../riir-dapps/.benchmarks/005_008_t24_shallow_dao_voting_g9_gov.md),
  G9-GOV PASS, the constitutional pin — a passed ban never moved a balance byte) + the
  dao ban-proposal composer `b1af3f8`.
- The decay-bound conflict resolved with the bound UNCHANGED (riir-dapps `3577986`,
  Issue 006 CLOSED): genesis saturates the lever's safe end; the agent cannot inflate
  through decay at all — pinned by tests on both sides of the contract.
- Proposal 010 (the ledger-budget-rule re-open half) LANDED end-to-end: row v2, the
  yield ceiling, treasury-first grants, activation.

## Summary

008 is a landed, bounded, audited advisory system, and **advisory-only is its shipped end
state by measurement**. The v2 plan is not commissioned (re-opens on a measurement, not a
date), the dopamine levers are killed rather than deferred, the autonomy ramp is closed,
and the one row with a real path — the advisory soak — moved to
[Issue 061](061_proposal_011_product_loop_rollout.md) behind the devnet ceremony rung.
Full reasoning: [Issue 068](068_kat_launch_tier_decision.md).
