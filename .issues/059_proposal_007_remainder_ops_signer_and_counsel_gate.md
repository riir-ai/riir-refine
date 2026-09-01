# Issue 059 — Proposal 007 remainder: the ops signer (T2.2) + the counsel gate (Phase 3)

Status: **OPEN (2026-09-01)** — both remaining items are blocked on NON-code decisions
(ops: RPC provider + custody-key posture; legal: counsel review). Every engineering
item in [Proposal 007](../.proposals/007_kat_usdc_staking.md) is DONE: Phase 0
(riir-dapps `7a18690`), Phase 1 economics, T2.1 CLI (riir-clippy `6ee000e`+`ba262a8`,
[Plan 041](../.plans/041_kat_007_t21_staking_cli.md)), T2.3 chaos/soak (riir-dapps
`b4b56c0`, [Bench 004](../../riir-dapps/.benchmarks/004_g7_econ_sim_and_staking_chaos.md)),
and staking itself is ACTIVATED (Proposal 010 Phase 4: `staking_share_bp` 0→500 with the
yield ceiling armed, ordering constraint enforced). This issue exists so the two open
items cannot be silently lost — file created while converting the KAT proposals
(007–011) to tracked issues.
Branch: `develop` (per global rule — no feature branches)
Proposal: [`.proposals/007_kat_usdc_staking.md`](../.proposals/007_kat_usdc_staking.md)
Related: [Issue 060](060_proposal_008_remainder_new_plan_and_gated_levers.md) (the
governance side), [Issue 061](061_proposal_011_product_loop_rollout.md) (the rail this
composes with), riir-dapps Plans 008/011.

## Open items

- [-] **T2.2 — ops signer binary + daily batch drain + per-day cap + receipt rows.**
      **DEFERRED — unblock: ops names the RPC provider + the custody-key posture.**
      The escrow / venting / vent-slice LEDGER semantics landed in Phase 0; what remains
      is the ONE genuinely new construction (outbound USDC transaction signing) plus its
      daily-drain driver. It is deferred on ops decisions, not on engineering risk: the
      RPC provider choice (which endpoint, rate limits, failover) and the custody-key
      posture (where the capped signer key lives, cold-backup, runbook) are operator
      calls that determine the binary's shape (connection pooling, key source, alerting).
      Re-open as its own plan the moment both are named — the proposal's own text: "the
      outbound USDC transaction construction needs ops decisions — re-open as its own
      unit".
- [-] **T3.1 — counsel review of the custody + incentive structure → pick the USDC arm
      or the KAT-stake arm. LAUNCH BLOCKER** (the proposal's caveat #1: "Ship NOTHING
      user-facing before counsel signs the structure"). The KAT-stake fallback arm
      already ships via reward restaking, so a "no" from counsel drops only the USDC
      rail, not the feature. **Unblock: counsel engagement scheduled.** Note the
      envelope is SHARED: this review also covers Proposal 011's KAT transfer rail
      (011 caveat #1 — one review, two surfaces) and the reward-restaking yield class
      (007 caveat #8).
- [-] **T3.2 — user-facing copy review** (queue-delay disclosure, no-APY language).
      Rides T3.1 — the copy depends on which arm counsel picks. The mechanical half of
      the copy rule (no `$` beside a KAT figure, service-credit labeling) is already
      test-enforced from T2.1; what counsel-gated review adds is the honest-disclosure
      wording (escrow window, venting slices, refund-to-source binding).

## Not open (verified landed — do not re-do)

- Stake rows, five-check claim, refund-to-source binding, per-tranche 7-day clocks,
  escrow + venting (riir-dapps `7a18690`, Plan 008-dapps).
- Epoch staking slice, snapshot-age rule, authority-share decay schedule, G7-S sim
  parameter freeze (Phase 1).
- `cargo heal stake/unstake` CLI + QR + copy gate (riir-clippy `6ee000e`+`ba262a8`,
  Plan 041).
- Chaos/soak: 26 tests, 16×4 full-loop sweep green; deep soak committed `#[ignore]`d
  (riir-dapps `b4b56c0`, Plan 011-dapps / Bench 004).
- Activation: `staking_share_bp` 500, `yield_ceiling_bp` 22 armed, thin-base window
  unwritable (Proposal 010 Phase 4).

## Summary

The staking feature is functionally complete and live at 500 bp; what remains is the
outbound-refund signer (ops-gated) and the legal posture decision (counsel-gated).
Neither has an engineering unblock this repo can perform.
