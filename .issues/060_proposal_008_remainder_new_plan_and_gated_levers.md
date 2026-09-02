# Issue 060 — Proposal 008 remainder: autonomy re-open needs a NEW plan; dopamine levers G5-gated; soak async

Status: **OPEN (2026-09-01) — advisory-only is TERMINAL for this proposal.** The G5
verdict is FAIL (wash) ([riir-dao Bench 001](../../riir-dao/.benchmarks/001_g5_simulator.md),
re-confirmed at the ratified genesis in [Bench 002](../../riir-dao/.benchmarks/002_longevity_cross_bench.md)):
adaptive ≈ frozen constants, so the agent stays advisory-only — a recorded defend-wrong
outcome, not a re-roll. Phases 0–2 LANDED (scaffold, bounds + telemetry, signals/
strategies/guards, GLM advisory transport, decision artifacts + audit CLI, shallow-DAO
ban/unban voting G9-GOV PASS + the dao-side ban composer `b1af3f8`). Everything below is
blocked on owner decisions, a NEW plan, or live-telemetry accumulation — nothing here is
executable without one of those. Issue filed while converting the KAT proposals
(007–011) to tracked issues.
Branch: `develop` (per global rule — no feature branches)
Proposal: [`.proposals/008_adaptive_tokenomics_riir_dao.md`](../.proposals/008_adaptive_tokenomics_riir_dao.md)
Related: [Issue 059](059_proposal_007_remainder_ops_signer_and_counsel_gate.md),
[Issue 061](061_proposal_011_product_loop_rollout.md), riir-dapps
[Issue 006](../../riir-dapps/.issues/006_freeze_ratification_gate_and_decay_bound_conflict.md)
(CLOSED — the decay-saturation resolution), Proposal 010 (the landed re-open half).

## Open items

- [-] **The NEW plan (the only path to autonomy).** The owner's "auto-live tokenomics"
      ask is NOT deliverable by 008 as specified — measured, not opinions: the G5
      verdict (adaptive ≈ frozen) plus Bench 004's axis-E finding (no agent-reachable
      `decay_factor` keeps mining alive to a 1000-epoch horizon at launch scale under
      any cap). The proposal's own re-open clause: "a ledger budget-rule change ... or
      v2 candidates require a NEW plan" — and the cap/initial-supply ratio change is an
      OWNER act (a supply decision), never an agent move. **Unblock: owner commissions
      the v2-candidates plan** (contents per Proposal 008 T1.4: v2 strategy candidates
      incl. decay/Inflate directions + the supply-ratio decision framed for owner
      sign-off). Do NOT re-roll the G5 campaign with the same candidates.
- [-] **T3.1 — `FairRoll` jackpot table as a governed lever, end-to-end.** GATED on
      G5-class evidence (the phase gate: "gated on G5 evidence" — which FAILED).
      Partially landed: the guard-side caps ship (`REWARD_EV_EPSILON_BP = 0`,
      `REWARD_MAX_MULTIPLIER_X = 10`, riir-dao guard.rs); what is open is the FairRoll
      table itself wired as a governed lever end-to-end. Unblock: the NEW plan above
      carries it, or owner explicitly de-gates it.
- [-] **T3.2 — streak bonuses (bounded, capped; retention metric wired into signals).**
      Same gate as T3.1.
- [-] **T2.3 — advisory-mode soak (operator applies by hand; N epochs clean).** ASYNC
      OPERATOR WORK — the mechanism landed (T2.1/T2.2); the soak accumulates when live
      telemetry flows. Unblock: the service plane runs live epochs (Proposal 011's
      Phase 2 genesis ceremony is the precondition — no live network, no telemetry).
- [-] **T4.1/T4.2 — the autonomy ramp. TERMINALLY FORECLOSED by the G5 verdict** for
      this proposal; re-opens only inside the NEW plan. Recorded here so the
      checkboxes in the proposal never look like ordinary backlog.

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

## Wait-state re-verification (2026-09-02, source-level)

Re-verified this date against the recorded benchmark rather than against this file's
own status line.

- **The G5 FAIL verdict is real and pinned, not stale prose.**
  `riir-dao/.benchmarks/001_g5_simulator.md:36` reads "**FAIL — no metric where
  adaptive strictly beats frozen (wash)**", and the doc records a regression pin
  asserting the verdict stays EXACTLY that. So advisory-only remains terminal for
  this proposal by measurement, not by opinion.
- **The only path out is still an owner act.** The v2-candidates plan requires an
  owner-commissioned supply decision (the cap/initial-supply ratio is never an agent
  move). Re-rolling the G5 campaign with the same candidates is explicitly NOT the
  path — that would re-measure a known wash.
- **T3.1/T3.2 stay gated** on G5-class evidence that failed; **T2.3's soak** stays
  blocked on live telemetry, which needs Proposal 011's genesis ceremony
  (Issue 061 T2.2 — an owner act). No engineering unblock exists for any of the four.

## Summary

008 is a landed, terminal-as-specified advisory system with one open path: a NEW plan
carrying v2 candidates + an owner supply decision. The dopamine levers and the soak are
gated/async respectively. This issue tracks those four wait-states so they surface in
issue sweeps instead of hiding in the proposal's checkboxes.
