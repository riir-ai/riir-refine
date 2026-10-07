# Bench 107 — Plan 202 R2+R3 dogfood spike: the first REAL seat-rerank traffic

**Status:** RECORD — spike complete 2026-10-08; findings feed the R7 ledger freeze (refine's manifest grammar half). R2's generate half stays loopback-evidenced; real-endpoint arming is R6 + owner-gated spend.

## The exercise

Two new example binaries — the first cross-process, cross-repo traffic the R3 wire has ever carried (the gates are in-process; each side had never spoken to the other's real binary):

- Seat half: `../riir-instinct/examples/ext_seat_dogfood_server.rs` — the REAL `serve_edge` with the rerank stub seat (byte-identical contract to `ext_seat_rerank_gates`), serving until killed on 127.0.0.1:8092, suite `sst5`, ENC row, lazy budget.
- Client half: `examples/seat_rerank_dogfood.rs` (opt-in `seat_rerank`) — REAL corpus candidate pools through `SeatRerankClient`, JSONL ledger out. Two disclosed passes + one probe: `corpus_pool` (the real fresh-install shape), `wire_scale` (8 real fixes per body — sibling rules' fixes, disclosed as NOT same-site candidates: payload/echo scale at the 8-label cap, never a quality claim), `abstain_probe`.

Postures: both sides RELEASE builds; loopback; box = shikuwa (Windows 11, i7-13700K, idle otherwise, AC); no GPU involved. Artifacts: `107_plan202_r2r3_dogfood/dogfood_ledger.jsonl` (25 warm rows) + `107_plan202_r2r3_dogfood/dogfood_cold.jsonl` (7 rows, cold start) beside this record.

## Measured

| surface | number | provenance |
|---|---|---|
| warm pick, pool=1 | **0.8–6.2 ms** round trip | first warm run 4.3–6.2 ms (cold OS sockets); second warm run 0.9–1.4 ms — OS-warm variance is real, quote a range not a point |
| warm pick, pool=8 | **1.4–8.9 ms** | echo bytes scale it (~1 KB body); same two-run spread |
| 503 decline (lane loading) | **0.22–0.42 ms** per decline | fast-fail; the client declines `Wire("loading")` one-shot |
| cold start to first pick | **53.8–109.4 ms total, 2–3 attempts** | lazy load window ~0.1 s + budgeted retries (50 ms backoff); observed in BOTH cold captures (the committed warm ledger's row 1 carries one too) |
| abstain round trip | 0.8–4.6 ms | abstention is first-class over the real wire |
| wire round trips | **57 total across three runs, zero contract violations** | echo, pick index, abstain, receipt fields all held |

## The findings (R7 ledger inputs)

1. **The corpus is 1-fix-per-rule — EVERYWHERE.** Verified across clippy_lints (100 rules, 0 with >1 fix), rust_perf, kernel_opt (777+, all unique rule ids). Consequence: the candidate pool on ANY fresh install is size ≤1 → **rerank traffic exists only on warm installs** (self-evolve memory accumulation is the only pool≥2 source today). The escalation chain's real default traffic: pool=0 → generate (R2), pool=1 → serve modellessly (a 1-option rerank is a paid socket for a non-choice), pool≥2 → rerank. **R7 freeze: the manifest grammar carries a min-pool gate (rerank arms fire at pool≥2 only).**
2. **The lazy 503 window is the first-customer experience.** The seat's first decision after boot (or eviction, or swap) declines `Wire("loading")` — one-shot, by contract (decline keeps the modelless answer; correct). The driver's budgeted retry (the `post_decide_until_ready` shape) recovered every time: 2 declines + ~100 ms. **R7 freeze: R6's arm H wiring owns a budgeted readiness retry (deadline-capped); eager-load rows are the seat-side alternative for latency-critical suites.**
3. **Transport floor ~0.8–6 ms/decision** (loopback, release, pool=1; OS-socket-warm vs cold spread). The edge is one-request-per-connection (`Connection: close`) and spawns per-connection state; the stub's decision time is 0 — the floor is pure transport. vs the DaemonL4 precedent (24 s/fix) this is noise, and decision-plane traffic is escalation-rate by design. **R7 freeze: v1 accepts the floor; keep-alive/pooled connections are the v2 lever (with `contract_version`, additive).**
4. **The client drops the receipt.** The reply carries `receipt {build, features, input_blake3, decision_blake3}`, `us`, `confidence`, `arm`, `lane_load` — `SeatAnswer` keeps only pick/abstain. The heal-side audit trail has no seat-side evidence. **R7 freeze: surface a receipt tier on `SeatAnswer` (at minimum decision_blake3 + build) — additive, shippable as v2 with contract_version; v1 ships without only if the break budget says so.**
5. **Consent is local-only — freeze it that way.** `RIIR_REFINE_CODE_EGRESS=snippet` gates every socket pre-connect; nothing consent-shaped rides the wire. The spike's refusal lane exercised clean (the driver exits loud on Egress refusal). **R7 freeze: no consent fields on the wire in v1; the endpoint's trust anchor is the transport + the receipt.**
6. **The suite axis is dataset-bound on the seat.** The seat routes by suite→lane; suites are dataset suites (sst5 needs its t20k labels at seat prep). A healing domain has no dataset row — the dogfood borrowed sst5 (the R3-proven vehicle). **R7 freeze: name the healing suite vocabulary (one `heal` suite vs suite-per-domain) before the first real hosted seat; the manifest grammar's `suite` field is the selection key, so the grammar freezes the naming too.**
7. **The v1 envelope is SUFFICIENT for rerank.** `{suite, state, options}` carried everything the contract needs — no wire change forced by traffic. The rule-id context (which lint produced the site) is an expert-QUALITY lever, not a contract need: additive v2 candidate under `contract_version`.

## R2 (generate) half — scoping record

`HostedExpertL4` evidence stays loopback (the 10 R2 tests incl. the consent-silent-listener pin). Real-endpoint arming needs: an R1 manifest row with a real endpoint + `RIIR_HOSTED_EXPERT_TOKEN` + egress consent + llm_spend pricing rows — **R6 + owner-gated spend**, not a spike unit. No payload unknowns surfaced beyond what the R2 tests pin (snippet-only builder, retention rule on the wire, deadline decline, cap decline).

## Consequences

- Plan 202 R7 can now freeze: the ledger rows above ARE the freeze inputs (refine's manifest grammar half: min-pool gate + suite naming + receipt tier decision; the decision_wire half lives in reflex's filed issue).
- R6 arm H gains a named prerequisite: expose the candidate pool (today `heal()`'s pool is internal — a `heal_with_pool`-shaped seam or the rerank hook), plus the readiness-retry policy.
- The examples stay as the standing dogfood rig: boot the server example, run the driver, diff the ledger.

## Addendum — the freeze landed (2026-10-08, same day)

The refine manifest grammar half of the freeze is in the tree (see Plan 202 R7's "Freeze LANDED" note):

1. `EscalateTarget.min_pool` — absent → `DEFAULT_MIN_POOL = 2`; seat_rerank rows may raise it, never lower it (validated 2..=8); refused as a dead knob on generate rows.
2. `SeatReceipt` — the seat's decision receipt surfaces on both `SeatAnswer` variants, lenient parse (an envelope without a receipt still answers — the additive-on-the-wire law); the dogfood driver prints it per pick.
3. **The suite vocabulary is DECIDED: one `heal` suite for v1.** One seat lane hosts the healing rerank expert; per-domain lanes wait for measured need — the hosted tier scales experts independently (R5's LRU), and multiplying seats before traffic is the premature-axis class. The suite is an opaque routing key on the wire, so this decision needs no wire change; it freezes the manifest grammar's `suite` field vocabulary for our first customer.
