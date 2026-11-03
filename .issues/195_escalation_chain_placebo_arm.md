# Issue 195: Escalation-chain production measurement needs a judgment-free placebo arm (JevAct's Table-3 lesson)

**Status:** OPEN — measurement-gate proposal, filed from `../riir-reflex/.research/013_JevAct_Decision_Gate_Agent_Runtime.md` (arXiv:2610.38620 "Think Before You Act: Decision Gate for LLM Agents", IBM SIL 2026-10-28). Timing is the point: T5.2 landed 2026-11-01 (`2623f4d09`) — the escalation chain (seat rerank over the proposer pool) is production-wired at the serve boundary, and its first production-gain measurement must not be interpretable as revision-opportunity gain.
**Claimed:** zed-riir-refine-195, 1793706957, expires 2026-11-05T12:00:00+00:00

## The paper's finding (why this is not paranoia)

JevAct's write-gate ablation (their Table 3 + §5.1) ran the placebo nobody runs:

- **Placebo: flag every write call once, check nothing** → GPT-4.1: 50.0 pass^1, byte-identical to ReAct's 50.0. On this agent the gate's JUDGMENT contributes nothing; the extra revision round is free at best.
- **For gpt-oss-120b the reversal is worse**: "the same re-check, or random flags at the gate's rate, raise pass^3 as much as the full gate (32.0% to 42.0%)". The ENTIRE measured gain of a sophisticated policy-compiled gate was reproducible by RANDOM flags at the same rate.
- Their own conclusion: "whether the judge's verdicts matter beyond the revision opportunity depends on the agent."

A gate feature grants a revision opportunity (a second attempt, a re-check round). That opportunity alone can be the whole effect, and which consumer shows which outcome is an empirical fact, not a design property.

## The mapping onto the escalation chain

| JevAct arm | Chain arm |
|---|---|
| ReAct baseline (no gate) | baseline: no escalation — modelless answer stands |
| Full gate (code checks + Kev judge) | real: escalation dispatch — seat rerank picks among the `heal_with_pool` candidates (or the hosted generate lane produces one) |
| **Placebo: flag at the same rate, no judgment** | **placebo: escalate at the SAME rate, pick a SEEDED RANDOM candidate from the same pool (seat ignored)** |

The chain's compile-gate is the executable check half — it runs in ALL arms (it is the oracle, not the treatment). The treatment under test is the seat's PICK. Seat wins on the paired read only if `real > placebo` (LB95 > 0), not merely `real > baseline`.

## The proposal

1. The escalation-chain production gate measurement (the `fixer_regate` arm-H successor / any T5.2-wired production read) runs THREE arms: baseline, real, placebo.
2. Placebo pick = deterministic seeded RNG over the same candidate pool the seat saw (seed BLAKE3-stamped per row per the determinism law — no unseeded-global draws; the global_rng_gate class). Escalation triggers copied verbatim from the real arm's rate (`modelless_cap` worthiness + the sticky cost ceiling unchanged) so trigger rate is matched by construction, not approximated.
3. Attribution claim language: "seat gain over placebo" is the only gain figure quoted as the seat's; "gain over baseline" is quoted as the harness's (seat + revision opportunity together).
4. Generalize: any FUTURE gate-shaped feature in this repo (hosted-expert generate lane, per-domain verify extensions, a `seat_rerank` re-arm after manifest changes) carries the same three-arm shape before any gain claim lands in a bench record.

## Proof path (ungoat discipline)

- Fixture: the existing paired-read machinery from Plan 225 T4 (Bench 119) — the frozen pool texts + pin-verified rows; the placebo arm adds one deterministic pass over the same rows, no new data.
- Falsification arm: the placebo itself. If `real ≤ placebo` on the paired LB95, the seat's judgment is NOT the gain — demote the attribution (the chain stays as a revision-opportunity feature; the manifest row's verdict line updates; no silent re-grade).
- Cost: one extra arm per regate run; no chain code changes (the placebo is a measurement-side pick policy).
- Owner-gated: adopting the three-arm shape as the standing production-gate convention.

## Non-goals

- Not a challenge to the T2 Bench-119 result itself (A1 = bit-identical to A0 on the deciding stratum — there the seat's value was cost, not accuracy, and cost has no placebo confusion).
- Not a new escalation policy — the trigger, rate guard, and dispatch are untouched.
