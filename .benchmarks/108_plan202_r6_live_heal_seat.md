# Bench 108 — Plan 202 R6 measurement: arm H over the LIVE heal seat

**Status:** RECORD — 2026-10-08. The escalation chain measured END-TO-END on the real wire
(live seat process, real sockets, armed manifest row, real corpus pools). The quality axis
is **instrument-valid only**: the seat is the R3 rerank STUB (deterministic index-1 pick) —
a known control, not an expert. The R7 one-`heal`-suite decision is exercised for real:
the lane, the manifest grammar's `HEAL_SUITE`, and the v1 decision_wire contract
(`contract_version` echo — instinct `db7490e`) all speak "heal".

## The rig

| half | what ran |
|---|---|
| Seat | `riir-instinct` `ext_seat_dogfood_server` **flipped to suite `heal`** (this unit) — the REAL serve edge, the `LoggingRerank` stub, ENC row, lazy budget, 127.0.0.1:8092. The seat boots through a VEHICLE corpus (sst5 — the suite axis is an opaque routing key; `prepare_seat` has no "heal" dataset row and the rerank backend answers presented options, not a label universe) |
| Client | `fixer_regate --arm H` (Plan 202 R6's escalation harness arm), features `cf_workers_ai,escalation_manifest,seat_rerank`, RELEASE |
| Manifest | `.refine/escalation_manifest_heal_armH.json` — clippy row ARMED: `seat_rerank` → `http://127.0.0.1:8092`, `min_pool` 2 (the frozen default), `modelless_cap` 0.5, `max_rate` 0.9, not demoted |
| Cases | `--source reachable` — the 9 authored fixtures (real L0–L3 pools of 5 candidates each, from the corpus + drafter) |
| Consent | `RIIR_REFINE_CODE_EGRESS=snippet`; deadline 2000 ms/case |
| Box | shikuwa (Windows 11, i7-13700K, AC); seat + client both release, loopback; no GPU involved |

## Measured

| surface | number | note |
|---|---|---|
| chain routing | **9/9 → seat rerank** | cap 0.5 × scores 0.0 (OffCorpus serves — the reachable fixtures are deliberately off-corpus): every case escalation-ELIGIBLE; 0 worthy-stands, 0 declines, 0 latched |
| readiness retry | first call declined `503 loading` (the lazy window), retried under the deadline budget — the Bench 107 shape, now under arm H's policy | |
| seat picks | **9/9 seat_pick, 0 abstains** | pool 5 each, no truncation (cap 8); receipts READ (build + µs + confidence — the R7 tier populating for real after the `input_blake3`/`decision_blake3` spelling alignment) |
| paired keep_s | **escalated wins 1, loses 0, ties 8** | the win is the STUB's fixed index-1 pick landing keep_s on one case — a control outcome, NOT an expert quality signal |
| cost axes | escalated wall 55 ms total / 9 cases (max 7 ms; ~6 ms/case = Bench 107's loopback transport floor); seat-side decision time 378 µs total; modelless arm 630 µs total | $0.00 — the decision-plane carries no LLM spend |
| verdict line | `ESCALATION WINS — demote the modelless incumbent via the manifest cap` | PRINTED, never applied (the demotion is a manifest edit) |

## The honest reading

1. **What this proves (mechanism):** the R6 chain works on real traffic — worthiness
   routing, min-pool, pool presentation (real 5-candidate pools), the deadline-capped
   readiness retry, seat picks + receipt audit, paired per-case scoring, cost axes, and
   the printed verdict. The v1 wire contract held (echo, receipt spellings) under a live
   multi-process run.
2. **What this does NOT prove (quality):** nothing about rerank QUALITY — the stub's pick
   is deterministic. The `ESCALATION WINS` verdict is therefore **instrument-valid,
   quality-pending**: it measures the harness, not an expert. The modelless incumbent is
   NOT demoted — the shipped default manifest stays ALL-EMPTY (arming is a manifest edit
   at deploy time, per the R1 law).
3. **Cap-choice discipline (the en-route fix):** the first run armed `modelless_cap: 0.0`
   — the ALL-EMPTY posture's value ("every fix stands") — and the chain correctly
   escalated NOTHING (9/9 worthy). The measurement gap: the JSONL rows did not record the
   modelless score, so the wrong cap was not quotable against the distribution it routed
   on. Arm H rows now carry `modelless.score` (the worthiness gate's INPUT); the score
   distribution on this corpus is 0.0 across all 9 (OffCorpus serves), which is what
   justifies cap 0.5 for the live run.

## Consequences

- The R6 measurement unit is COMPLETE: live heal-suite seat (suite flipped this unit) +
  armed row + the run recorded here. The demote-the-loser manifest edit: **declined on
  stub evidence** — revisit when a real hosted heal expert exists (HOSTED-ONLY tier,
  riir-train mint lane).
- The `ext_seat_dogfood_server` example is now the standing `heal`-suite rig (Bench 107's
  sst5 borrow retired); the gates (`ext_seat_rerank_gates`) still exercise `sst5` as the
  R3-proven vehicle — both spellings are pinned, the suite key is data.
- Arm H cannot silently drop out (Plan 202 §11): this bench is the arm-H run record.

## Artifacts

- `/tmp/armh_live` run rows (9 JSONL, this box) — the committed evidence is the table +
  numbers above; the rows carry machine-local paths and stay uncommitted.
- Seat log: live process stderr (`[seat-call]` per wire call: state_bytes, options,
  option_sizes, pick).
