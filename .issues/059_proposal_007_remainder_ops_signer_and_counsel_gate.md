# Issue 059 — Proposal 007 remainder: the ops signer (T2.2) + the counsel gate (Phase 3)

Status: **RESOLVED (2026-09-04) by owner decision — see
[Issue 068](068_kat_launch_tier_decision.md).** Both open items were blocked on
NON-code decisions, and the decision that unblocked them was that **neither is on the
critical path**: the ops signer and the counsel review both belong to the **USDC custody
rail**, which is now Tier 2 (not proposed, demand-gated), while the shipping arm is the
custody-free **KAT-stake** arm that was already live. This file is kept rather than
removed because ~30 files across five repos cite it by number; it is now a record, not a
wait-state tracker.
Branch: `develop` (per global rule — no feature branches)
Proposal: [`.proposals/007_kat_usdc_staking.md`](../.proposals/007_kat_usdc_staking.md)
Decision: [Issue 068](068_kat_launch_tier_decision.md) · Related:
[Issue 060](060_proposal_008_remainder_new_plan_and_gated_levers.md) (the governance
side, also resolved by 068), [Issue 061](061_proposal_011_product_loop_rollout.md) (the
live product tracker), riir-dapps Plans 008/011.

## Decisions applied (2026-09-04)

- [x] **T2.2 — ops signer binary + daily batch drain + per-day cap + receipt rows.**
      **WON'T-BUILD NOW** ([Issue 068](068_kat_launch_tier_decision.md) D2). The signer
      serves exactly one purpose — outbound USDC unstake refunds — which exist only in
      Tier 2, and it is the highest-blast-radius component in the system (a custody key
      with send authority). The two ops questions this row waited on (RPC provider,
      custody-key posture) are not *missing*, they are **premature**: the right answer
      for a rail we have decided not to operate is "do not choose one yet."
      **Re-open trigger:** Tier 2 authorized — a real inbound USDC deposit intent or an
      operator ask for fiat-in. Re-open as its own plan then, with real load numbers.
      **Accepted cost, recorded:** at Tier 2 the longest-lead item starts from zero.
- [x] **T3.1 — counsel review → pick the USDC arm or the KAT-stake arm.**
      **DECIDED: the KAT-stake arm ships; the USDC arm is not proposed to counsel**
      (068 D1). The fallback arm runs under the *unchanged* 005 caveat-#7 posture (no
      custody, no outbound transfers, no new legal surface) and is **already activated**
      (`staking_share_bp` 500, `yield_ceiling_bp` 22 armed in `EpochParams::DEFAULT`;
      the ordering constraint enforced structurally at `riir-dapps/src/kat/ledger.rs:1350`).
      Choosing it costs nothing because it is the arm that already runs.
      **This is no longer a launch blocker** for Tier 0 (devnet/testnet, operator-owned
      accounts) — that surface needs no new posture. The counsel engagement is
      **rescoped and reduced** to ONE narrow Tier-1 question, unbundled from the custody
      review that was making the cheap question wait on the expensive one:
      *"May a non-transferable, non-redeemable prepaid service credit move between two
      independent account holders inside our own closed loop without reclassifying the
      product?"* Asked when a Tier-0 demand signal exists, not before.
      **Not a legal opinion** — the decision is which surfaces we operate, and the
      ambiguous case (a mainnet operator loop) is deliberately placed in Tier 1 rather
      than argued into Tier 0.
- [x] **T3.2 — user-facing copy review.** **NO LONGER COUNSEL-GATED** (068 D3). It rode
      T3.1 because the disclosure wording depended on which arm shipped; D1 picks the
      arm, and the KAT-stake arm has **no escrow window, no venting slices and no
      refund-to-source story to disclose** — there is no custody. What remains is
      mechanical and already partly enforced (no `$` beside a KAT figure, service-credit
      labeling — 005 caveat #15, test-pinned since T2.1).
      **Residual engineering, tracked in 068 §"What is actually open" item 3:** extend
      the pinned copy set to the KAT-stake surface (the per-tranche 7-day lockup clock,
      the no-promised-rate line). Acceptance: the existing copy-rule test covers the
      stake/unstake strings, canaried by reintroducing a `$`-adjacent rendering.

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

## Wait-state re-verification (2026-09-02, source-level) — historical

Re-verified against source, not against this file's own prose — a tracker that re-reads
itself cannot detect that the thing it describes has changed. Retained because the
activation claim below is what makes 068 D1 free to take.

- **The activation claim holds.** `riir-dapps/src/kat/compose.rs` `EpochParams::DEFAULT`
  carries `staking_share_bp: 500` with `yield_ceiling_bp: 22` armed in the SAME default,
  and `src/kat/ledger.rs:1350` structurally refuses a row with `staking_share_bp > 0`
  while the ceiling is disabled. The ordering constraint (Issue 009 — never open the
  thin-base window uncapped) is enforced by construction, not by convention.
- **Both open items were non-engineering.** That framing was correct and is what 068
  acted on: an item with no engineering unblock is a *scope* question, and the scope
  answer was that neither item is needed for the next surface.
- **The KAT-stake fallback arm still ships**, which is why dropping the USDC rail from
  the critical path drops no feature.

## Summary

The staking feature is functionally complete and live at 500 bp. The outbound-refund
signer is **not built by decision** (Tier 2, re-open on demand) and the legal posture
question is **answered by arm selection** (KAT-stake, under the unchanged posture) with a
narrower Tier-1 review deferred until there is demand for it. Nothing here is waiting
without a named trigger. Full reasoning + the tier table:
[Issue 068](068_kat_launch_tier_decision.md).
