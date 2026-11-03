# The placebo arm — attribution methodology for gate-shaped features

> **Purpose:** how this repo attributes a gain to a gate's JUDGMENT vs
> its REVISION OPPORTUNITY. The JevAct Table-3 lesson (arXiv:2610.38620
> §5.1): a gate feature grants a second attempt, and the opportunity
> alone can be the whole measured effect — for gpt-oss-120b, RANDOM
> flags at the gate's rate reproduced the full gate's gain (32.0% →
> 42.0%). "Whether the judge's verdicts matter beyond the revision
> opportunity depends on the agent." This page is the standing answer.

## The three-arm shape

Any gate-shaped feature's gain claim runs THREE arms on ONE population:

| Arm | What it is |
|---|---|
| **baseline** | no gate — the modelless answer stands |
| **real** | the gate with its judgment (the seat picks among the pool) |
| **placebo** | the gate's trigger, NO judgment — a deterministic seeded-random pick over the pool the seat saw |

- Trigger rate matched **by construction**: the placebo evaluates the
  SAME gates on the SAME inputs (the row's `modelless_cap` worthiness +
  `min_pool`; the fresh-per-case posture the harness arms, latch-free).
- The pick is **auditable**: `blake3(case_id)` first 8 bytes LE, modulo
  the presented pool length (`PRESENTATION_CAP`-truncated — the pool
  the seat actually saw). Same rows → the same picks forever; a re-run
  is bit-identical; the ledger's `pick_idx` recomputes by hand.
- No unseeded-global draws (the `global_rng_gate` class).

## The claim-language law (issue 195 item 3)

- **"seat gain over placebo"** — the ONLY gain figure quoted as the
  seat's judgment. Real > placebo on the paired LB95 > 0, or the
  attribution is REFUTED.
- **"gain over baseline"** — the harness's (seat + revision opportunity
  together), never the seat's.
- **Falsification**: `real ≤ placebo` at LB95 ⇒ the judgment adds
  nothing over a blind pick at the same rate — demote the attribution,
  keep the chain as a revision-opportunity/cost feature. Never re-grade
  silently.

## Where it runs

- `fixer_regate --arm FROZEN --placebo` — the three arms in one pass
  over the frozen population (Plan 225 T4's pre-registered A1-vs-A0
  read is byte-unchanged when `--placebo` is absent). The summary
  prints seat-vs-placebo (judgment) and placebo-vs-A0 (opportunity)
  beside the pre-registered read, then the ATTRIBUTION verdict.
- `fixer_regate --arm PLACEBO` — the placebo half alone, no seat, no
  transport, no verifier (manifest-alone posture; scores against the
  frozen gold). Deterministically identical to the FROZEN pass's
  placebo half, so a seatless run reproduces those rows exactly and the
  judgment read joins offline from the FROZEN ledger. This arm answers
  the OPPORTUNITY axis only — it never saw a seat.
- The policy lives in `src/escalation_placebo.rs` (`pick_idx` +
  `triggered`), gated at `escalation_manifest` alone — a placebo
  consumes the row's trigger constants, never a transport.

## Posture bound

`triggered` replicates the FRESH-PER-CASE dispatch posture (the harness
arms a new dispatch per case, so the rolling-rate latch never fills —
Bench 119's pre-registered posture). A long-lived dispatch posture also
carries the sticky cost-ceiling latch; a placebo against such a posture
must mirror the latch explicitly or disclose the mismatch. Never
silently.

## Generalization (issue 195 item 4 — adoption OWNER-GATED)

Any FUTURE gate-shaped feature in this repo (hosted-expert generate
lane, per-domain verify extensions, a `seat_rerank` re-arm after
manifest changes) carries the same three-arm shape before any gain
claim lands in a bench record. The shape above is the template; the
MANDATE to apply it everywhere is the owner-gated half (issue 190
T-family adjacent) — until adopted, quoting "seat gain over placebo"
is this repo's own discipline for the escalation chain.
