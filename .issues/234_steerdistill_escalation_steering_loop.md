# Issue 234: SteerDistill steering loop for the escalation chain (steer, don't take over)

**Status:** Open — unclaimed
**Source:** arXiv:2611.13410 SteerDistill via [riir-rethink/.research/025](../riir-rethink/.research/025_SteerDistill_Closed_Loop_Teacher_Steering.md)
**Related:** `src/hosted_expert.rs` (R2 payload: snippet + no-retention system message — GENERATE/takeover shape), `src/seat_rerank.rs`, opt-in `ternary_inference`, `fix_verify` compile gate, corpus-gap intake lane

## The gap

SteerDistill's Q2/Q3 ablations: guidance that lets the student **revise its own response** beats teacher takeover (+1.78) and unguided retry (+2.38); structured prompts (discrepancy / applicable conditions / corrective rule) beat free-form (+2.06). Our escalation chain has the execution-feedback environment the paper requires (the `fix_verify` compile gate ≈ its LocalVerify) but the hosted arm is takeover-shaped: it generates a replacement fix instead of structured guidance, and nothing retries the student under refined guidance.

## Scope law (the honest part)

The production modelless drafter is **not promptable** — the steer loop cannot touch it. The only promptable student arm in the chain's neighborhood is the opt-in `ternary_inference` inference arm. This issue is scoped there, measurement-only, never the release set.

## The actionable cell

- [ ] **T1** Structured guidance payload builder (discrepancy / conditions / corrective rule / expected outcome) beside the existing snippet builder in `hosted_expert.rs` — opt-in flag, no default-posture change.
- [ ] **T2** Steer-retry loop (budget R=3 — a budget choice below the paper's R=4 to bound hosted-token spend per failure; results are a budgeted probe, not a full reproduction): ternary arm generates a revision conditioned on the guidance → compile gate verifies → on fail, feed the failure category back as guidance refinement → verified revisions route into the existing content-addressed corpus-gap intake (internalization = corpus rows, our medium).
- [ ] **T3** Measure via the `fixer_regate` arm: steered-ternary vs takeover vs plain-modelless on the labeled eval fixtures. Decline is a correct answer — if takeover wins or costs are dominated, record NEGATIVE and close.

## Non-goals

- No hosted_expert default-posture or payload-format change without the measurement in T3.
- No new hosted-token spend lane beyond the R2 budget rules (cap latch, deadline, snippet-only payload).

**Claimed:** zed-4430bf70@4090, 1794579705806, expires 2026-11-13T16:21:45Z — housekeeping routing (2026-11-13): routed to the amo lane (4090 cannot host refine builds right now — 2/32 GB RAM free, the sibling t679 job holds the box). Order: T1 (structured guidance payload builder in hosted_expert.rs, opt-in flag, no default-posture change), T2 (steer-retry loop, budget R=3, verified revisions feed the corpus-gap intake), T3 (fixer_regate measurement — decline is a correct answer; takeover-wins or cost-dominated records NEGATIVE and closes). Measurement-only, never the release set. Claim expected from the taking agent on pickup.
