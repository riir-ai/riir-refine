# Proposal 027 — the points drain: the hosted-lane fair-use budget (rates, cap, refill, the O(1) gate)

**Status:** Proposed — the §4 follow-up [Proposal 026](026_points_plane_riir_ledger.md) defers (issue 192 GAP 4); this proposal owns the numeric fair-use policy (drain rate, balance cap, refill paths) and the enforcement mechanism. The numbers below are the **PROPOSED defaults** — one owner ratification act pins them alongside the earn table + rank half-life (026 §3/§5.2), and nothing enforces before that act. Delegated verdict AGREE r3 (session `f64f7d90`, 2026-11-02 — three rounds: r1/r2 REVISE drove the meter server-side, the `B` ceiling, the unjoined consistency law, and the tiered fail).
**Lease closed:** zed-m3-drain, 1793593066, claimed 2026-11-02, expired 2026-11-04T04:17:46+00:00 — filed + AGREE'd within the window (`24cee313c` r1, `a382e3563` r2, `fefce49f5` r3); execution plan: [Plan 326](../.plans/326_points_drain_phase12.md).

## TL;DR

Hosted lanes in cargo refine — the `seat_rerank` escalation seat (ARMED in the clippy domain manifest) and the `hosted_expert` L4 generate (armed when a manifest row arms it) — meter under a **free daily allowance + a billable-day fee + a hard daily billable ceiling + a capped spendable balance, refilled only by proven contribution**. **Enforcement is SERVER-side at the seat** — the one ledger is the seat's per-`(account, day)` count of signed per-call attestations — because a client-reported meter cannot bind a patched client. The client keeps a **UX pre-check only**: a local cached O(1) integer compare on the escalation path (after `min_pool`, before the transport) that declines early to save the round trip — never the enforcement (the Proposal 025 bar: the client decision path stays network-free). Exhaustion declines typed (`StandReason::DrainExhausted` → the existing `StoodByModelless` answer); **local `fix` is never metered** (binding law 1). Mechanism family: textbook token-bucket/GCRA fair-use (cited-only); the contribution-only refill, the rank-decoupled debit, and the attestation-carried seat meter are ours. Ship the shape behind an opt-in feature that is **inert until the owner ratifies the constants**; enforcement itself is the owner-gated act.

## The problem this solves

Proposal 026 shipped the points plane W0 with the drain deliberately out (§4: "v1 public copy leaves the drain OUT entirely"), and Plan 061 + Plan 325 landed the wire, the routes, and the client. Meanwhile Plan 225 T5.2 (`2623f4d09`) wired the escalation dispatch at the production serve boundary — **hosted `seat_rerank` calls are real usage today**, unmetered. Without the drain policy:

1. "points meter hosted lanes as fair use" (the only sanctioned public sentence, Plan 324 G-B law) is copy with no mechanism behind it.
2. Hosted cost grows unbounded with adoption — the one plane where a user can extract continuous spend without contributing anything.
3. The honest-targets discipline (shape ships, numbers ratified) has no vehicle for the drain half: 026 §8 names this proposal as the gate, and no vehicle exists yet.

## What meters (and what never does)

| Surface | Metered? | Why |
|---|---|---|
| `seat_rerank` escalation decisions reaching the seat | **YES** | the hosted instinct seat — real usage today (armed manifest row) |
| `hosted_expert` L4 generations reaching the hosted lane | **YES** (when armed) | same plane, same counters; drain promotes BEFORE expert arms (caveat 4) |
| local `fix` (the modelless path itself) | **never** | binding law 1 — local healing is always free, never blocked |
| escalation cases that never reach the seat (`Worthy`, `RateLatched`, `BelowMinPool`) | **never** | no hosted call was made; consuming allowance for a non-call would be a defect (the counter-order law below) |
| sync, `/points/balance`, `/points/rank`, `/points/info`, join/leave | never | reads and identity plumbing; metering them would punish contribution |
| the `upgrade` / corpus-lease lanes | never | the KAT plane, frozen and separate (issue 192 T16.7) |
| the served reflex/rethink products | never | separate product lane with its own economics (Proposal 014) |

## The proposed design

### The enforcement point is the seat (server-side) — the one ledger

A client-reported meter cannot bind a patched client (the client is open source;
`RIIR_REFINE_OFFLINE=1` or a dropped report would zero it). So the meter of record
lives where the spend happens: **the hosted instinct seat counts what it serves**.

- **Per-call attestation (identity on the seat request).** The client attaches three
  HTTP headers — `X-Points-Account: <account_id hex>` + `X-Points-Pubkey: <ed25519
  pubkey hex>` + `X-Points-Sig: <ed25519 sig>` — over
  `account_id ‖ utc_day ‖ call_seq ‖ blake3(request_body)`. Headers, not JSON
  fields: the seat's `/decide` envelope is the **frozen v1 decision_wire contract**
  (riir-reflex issue 074 R7); unknown headers are ignored by today's readers, so
  identity is backward-compatible (the `kat_ua` `X-Kat-Totp` precedent — the one
  header-setter law applies client-side). The pubkey rides WITH the signature
  because a hash cannot be reversed into a key: the seat checks
  `blake3(pubkey) == account_id` (the id is bound to the key that presents it —
  no registry lookup needed to VERIFY; a fresh unregistered key verifies fine),
  then verifies the signature (anti-spoof: nobody can attribute calls to someone
  else's account and drain THEIR points; the body hash ties each signature to the
  exact request it authenticates). `call_seq` is a per-process counter checked at
  the seat through a **replay window** — a fixed per-`(account, day)` bitmap over
  the last N sequence numbers (the IPsec anti-replay shape): unseen-and-in-window
  accepts, below-window rejects as replay — because several concurrent `cargo
  refine` runs (worktrees, parallel agents — this workspace's ordinary posture)
  share one account and day, and a strictly-increasing check would drop their
  out-of-order calls to the anonymous posture. Account EXISTENCE (joined? has
  balance?) is a seat-side lookup that fails gracefully to the unjoined posture —
  verification and enforcement are independent steps.
- **Seat-side day counters** keyed `(account, utc_day)`: `free_calls ≤ A`,
  `billable_calls ≤ B` (billable only while the seat's cached balance for the
  account is `≥ R`). At `A + B` the seat refuses further hosted calls that day →
  the call declines → the caller serves the modelless answer. The account id is
  `BLAKE3(pubkey)` (the riir-auth account-key substrate — pseudonymous, not
  hardware-derived, nothing about the machine), consistent with the T16.1 privacy
  law; the seat learns no more than the id the caller chose to attest.
- **Two caller classes, two budgets — signing alone multiplies nothing.**
  **JOINED accounts** (GitHub-anchored — one account per GitHub id, the Sybil
  anchor) draw their own `A`/day from the seat, plus the billable tier (`B` calls
  at balance ≥ `R`). **UNJOINED keys — signed or not — share the same per-source
  budget** as anonymous callers. This is the consistency law: a throwaway keygen
  is free, so an unjoined key must never out-draw an un-attested caller; what
  join buys is the personal allowance and the billable tier, and join is the one
  identity axis a farmer cannot freely rotate. The per-source key is a
  **salted, daily-rotating hash of the client's /24 (IPv4) or /64 (IPv6) prefix**
  — the salt rotates at the UTC day boundary, the hash is kept for THE DAY ONLY
  and never persisted past it, and the practice is disclosed beside the T16.1
  privacy law (it is the free tier's main enforcer and must be said in plain
  words, like the client-UUID disclosure).
- **Availability is tiered fail-open/fail-closed.** If the seat's counter store is
  down: anonymous and per-source traffic fails OPEN (the coarse allowance family
  serves — the caller is never broken by the meter), while the BILLABLE tier fails
  CLOSED (billable callers get the anonymous posture for the outage — spend is
  never unbounded because the enforcement store is gone). An uncountable seat is
  a degraded meter, never a client failure and never an open bar.
- **The aggregate settles to the points plane server-to-server.** The seat exports
  daily `(account, day, free_calls, billable_calls)` rows to the kat-service points
  plane (authenticated service route, **idempotent by `(account, day)`** — the
  burn-sync monotone-watermark posture: a replayed day is a no-op, never a double
  debit). HTTP data crossing, zero cargo deps in either direction — the
  dao↔dapps telemetry-as-data precedent (the 005 Issue-041 one-way rule shape).
  Debit at processing: a day with `billable_calls > 0` debits `R`, **floored at 0**
  — points never go negative-locked (issue 192 GAP 4 law; property-test invariant).
  Integer math only; no floats anywhere near the money path (the dao law).
  **The client sends NO usage report.** The original draft's `POST /points/usage`
  from the client is dropped — it was the self-reported hole. The client gate
  (below) is pure-local; its only network contact is the `/points/balance` reads
  the user already runs.

### The client gate — UX pre-check, the 025 bar met by construction

One machine-local day state beside `client_id.rs`'s store:
`{ utc_day, hosted_calls_today, cached_balance: Option<u64> }`. The check sits in
`escalation_dispatch.rs` **after the `min_pool` gate passes and immediately before
the transport call** — `Worthy` / `RateLatched` / `BelowMinPool` cases consume
nothing (a non-call never burns allowance; a test pins this order):

```text
allow_hosted():                       # UX pre-check only — the seat enforces
    roll utc_day (reset counter on day change)
    if hosted_calls_today < A_served  -> allow, hosted_calls_today += 1
    else if cached_balance >= R_served -> allow (billable), count += 1 (≤ B locally)
    else                              -> decline (skip the transport)
```

Two integer compares; no allocation, no network, no I/O on the decision path — the
≤ 1 ms p99 bar is met by construction and measured anyway (GOAT G2). The modelless
hot path never executes this code.

**Decline shape:** a new `StandReason::DrainExhausted` arm (the real enum at
`escalation_dispatch.rs:80` — not a `Declined(String)` class: exhaustion is a
POLICY verdict like `SeatAbstained`, and the audit plane counts reasons by variant)
→ `EscalatedFix::StoodByModelless` → the caller serves the modelless fix, plus ONE
run-log disclosure line naming the refuel path ("contribute for hosted runway" —
the issue-075 soft-gate shape). Local `fix` output is byte-identical whether the
gate allows, declines, or is compiled out. A deliberately wrong client clock only
skews the LOCAL pre-check (early-decline or over-allow UX); the seat's server clock
is authoritative for enforcement, and anonymous callers have no gate to skew.

**Cache semantics (deliberate):** `cached_balance` refreshes on any successful
`/points/balance` read. Staleness here costs UX accuracy only — the seat's own
balance cache has a TTL and is the enforced side (a stale-positive SEAT cache can
admit a bounded number of billable days past true exhaustion; the TTL bounds it and
the settle reconciliation reports it — caveat 2).

### The numbers — ratio logic (why these constants)

| Constant | Proposed | Logic |
|---|---|---|
| `A` — free daily hosted allowance | **200 / UTC-day** per JOINED account | `seat_rerank` fires only on modelless-MISS ∩ pool ≥ `min_pool`; real heavy usage is tens/day. 200 is generous fair use; unjoined keys never see it (they share the per-source budget — the consistency law above), and join is one-account-per-GitHub-id, so the personal allowance is not freely rotatable |
| `B` — hard daily billable ceiling | **200 calls / UTC-day** | closes the 1-point-unlimited-day hole: max hosted calls/day = `A + B = 400` regardless of balance; the day-flag fee then bounds COST, `B` bounds CALLS |
| `R` — billable-day fee | **2 points / billable day** | day-flag, not per-call: un-gameable, trivially deterministic. Under any plausible ratified earn (10–30/epoch ≈ 1.4–4.3/day), a hosted-heavy contributor pays ≈ half their earn rate — contributing stays above water; pure consumption drains |
| `C` — spendable balance cap | **90 points** | this proposal's own bounded-drain-window rule, **[30 days, 2 years]** at the sustained billable rate — 90 / 2 = 45 days ✓. The window is ours (looser than the KAT free-grant's [7 days, 2 years] envelope-gate doctrine, `../riir-dao/crates/riir-dao/tests/envelope_gate.rs` + the riir-dao AGENTS envelope row, cited as lineage): `C` caps CONTRIBUTED balance, not a free grant — a contributor's earned runway should outlast a giveaway's |
| `P` — per-source daily budget (the unjoined/anonymous free tier) | **SIZED AT RATIFICATION from the report-only counts** | the free tier's main enforcer (the consistency law) — deliberately NOT pre-numbered: the seat's pre-ratification report-only counting measures the real per-source distribution, and the ratified `P` must clear the shared-NAT edge case (a campus/carrier /24 is many humans; sizing from measured counts is the honest input, a guess is not) |
| join grant | **none** | points come only from proven contribution; no faucet, no packs, no purchase — the D4 doctrine shape: contribute or the modelless path (which is the product's primary path anyway) |

The window bounds [30 days, 2 years] are the INVARIANT; the absolutes are
provisional and re-derived at the joint ratification if the earn table lands
elsewhere. The band-fee alternative (`R × ceil(calls / A)`, finer cost-causation)
is the named revisit lever — `B` + day-fee is v1 because it is two integers and a
flag.

### Wire changes (additive only)

- Seat request: the three attestation headers (outside the frozen JSON contract;
  old seats ignore them, old clients omit them — byte-compatible both directions).
- `/points/balance` gains `drain_state` (additive serde, the `pay` precedent):
  `{ allowance_remaining_today, billable_ceiling_today, rate_served }` — the
  client's pre-check constants come from the SERVER (`A_served`, `B_served`,
  `R_served`), so a ratified re-tune never needs a client release.
- The seat→plane daily-aggregate route (authenticated, idempotent by
  `(account, day)`).
- `FORMAT_VERSION` bumps ride the additive-default law. No boundary changes:
  riir-ledger (wire), riir-dapps (plane rows + routes + debit), riir-instinct
  (seat counting + enforcement — the seat host already owns its serving posture),
  riir-refine (client pre-check) each act inside their existing surfaces. The
  instinct↔dapps edge is HTTP data crossing, zero cargo deps (the one-way rule).

### Feature flag + promotion

`points_drain` — opt-in, implied by nothing, never in a default or release set
initially. **Absent/malformed ratified config = fully inert, byte-identical to
today**: the client pre-check does nothing (it has no `A_served` to apply — the
constants are served, not compiled), and the seat COUNTS REPORT-ONLY (no
enforcement, no debits) — which is deliberate: pre-ratification report-only
counting is exactly the measured-usage data the ratification act should size
`A`/`B`/`R` from, replacing guesswork with the dispatch receipts. Demote-only env
`RIIR_REFINE_POINTS_DRAIN=0` after promotion (the promoted-arm env law). Promotion
requires: the joint owner ratification + the GOAT gate below, and the ORDERING LAW:
the drain promotes BEFORE (or with) any `hosted_expert` manifest arming.

## Honest caveats — READ BEFORE IMPLEMENTING

1. **The earn numbers do not exist yet.** `R` and `C` are ratios to an unratified
   table; this proposal's absolutes are placeholders pending the joint act. If the
   earn table lands at a very different scale, re-derive from the window bounds —
   never ship the placeholders by inertia.
2. **Two caches, two bounded leaks — both deliberate, both disclosed.** The client
   cache staleness costs UX accuracy only (the seat enforces). The SEAT's balance
   cache TTL is the enforced-side leak: a stale-positive seat admits billable days
   past true exhaustion, bounded by the TTL and surfaced by the settle
   reconciliation. Tightening is a config change (shorter TTL), never a redesign.
3. **Rotation is bounded, not eliminated — and signing buys nothing.** Unjoined
   keys (signed or not) share the per-source budget, so throwaway keygen draws no
   more than staying anonymous; the per-source hash rotates daily and is kept for
   the day only (a privacy cost, disclosed beside T16.1, accepted as the free
   tier's enforcer). Joined rotation is bounded by join itself (one account per
   GitHub id). If seat receipts ever show allowance farming at scale (many
   unjoined sources per human, or join-farm attempts), the named escalations are
   per-source tuning (a config act) or a joined-only-allowance amendment — the
   latter REFUSED for v1 (it would couple basic utility to identity; the privacy
   law prefers allowance-for-all).
4. **Promotion ordering is load-bearing:** `points_drain` must promote BEFORE (or
   with) any `hosted_expert` manifest arming. The meter must never ship after the
   spend it meters.
5. **One meter, one ledger.** The seat's per-`(account, day)` count is the ONLY
   hosted-usage meter; the client pre-check is explicitly NOT a ledger (it exists
   to save a doomed round trip and to decline politely); the KAT burn tail stays
   untouched (separate plane, the one-way rule, T16.7 frozen). A second meter
   would fork the accounting and the audit story.

## Fusion lineage

- [Proposal 025](025_mozilla_human_scale_ai_incubator.md) §perf/sec posture (the
  cached-balance O(1) pre-check bar — here satisfied by the CLIENT-side pre-check;
  enforcement moves server-side where a client-reported meter cannot bind) ×
  [Proposal 026](026_points_plane_riir_ledger.md) §4 (the deferral mandate) →
  this design.
- **Issue 075's depleted-account soft gate** (refuse + name the refuel path,
  server-truth-only — the property this design finally honors by putting the
  ledger server-side) → the decline UX shape, reused verbatim.
- **The envelope-gate doctrine**
  (`../riir-dao/crates/riir-dao/tests/envelope_gate.rs`, Plan 035 T5;
  the riir-dao AGENTS envelope row — the [7 days, 2 years] free-grant drain window)
  → the bounded-drain-window parameterization; this proposal sets its OWN window
  ([30 days, 2 years]) for a contributed-balance cap.
- **`escalation_dispatch::StoodByModelless` + `StandReason`** (Plan 202 R6, wired
  by [Plan 225](../.plans/225_heal_seat_expert_bench.md) T5.2) → the typed decline
  the drain extends with one arm; zero new refusal machinery.
- **The `kat_ua` header seam** (X-Kat-Totp on POSTs, the one header-setter law) →
  the attestation-headers-outside-the-frozen-contract pattern.
- GCRA / token-bucket (references 1–2, cited-only) → the O(1) constant-space
  enforcement family.

## GOAT gate

- **G1 correctness (property tests):** the balance floor is never crossed (no debit
  can produce a negative); drain-off boots are byte-identical (absent config ⇒ the
  gate does nothing client-side AND the seat enforces nothing); an
  exhausted-points client's local `fix` output is byte-identical to no-gate; a
  replayed seat-aggregate day is a no-op; **counter-order pin** — `Worthy` /
  `RateLatched` / `BelowMinPool` cases consume no allowance and make no seat call;
  **day-rollover pin** — a UTC-day change mid-run re-rolls the local counter and
  the seat counts the call to ITS day (the server's), so no straddling call is
  unreported or double-charged (the seat count is the ledger; the client count is
  never reconciled against for money); **attestation pins** — a wrong-key sig, a
  reuse `call_seq` within the replay window, and a missing header all fall back to
  the unjoined posture, never an error; an out-of-order `call_seq` from concurrent
  runs ACCEPTS (the window bitmap is the point — only below-window replays
  reject); the counter-store outage arms the tiered fail (open for per-source
  traffic, closed for the billable tier).
- **G2 perf:** the client pre-check adds zero network + zero alloc to the
  escalation path; measured dispatch overhead delta ≤ 1 ms p99 against the
  pre-gate posture (the 025 bar) — expected µs-class. The modelless hot path is
  untouched by construction and the G3 suite re-pins its counts. The seat-side
  check is one hash-map read + two compares on the seat's serve path (the seat is
  a remote service; its latency budget is its own, unaffected at µs scale).
- **G3 no regression:** full suites at drain-off byte-identical (the flag-off
  posture is a tested posture, never assumed); `cargo refine --score-bench` if any
  proposer-adjacent file is touched.
- **G4 alloc-free:** the pre-check path allocates nothing (counting-allocator
  canary).
- **Accounting determinism:** the same seat-aggregate rows produce bit-identical
  debits (integer math only); the settle reconciliation (seat counts vs plane
  debits) is a gate in the `dapps_contract_pins` shape.

## What ships now (riir-refine + riir-ledger + riir-instinct + riir-dapps) vs deferred

### Ships now — the shape, inert until ratification
- `riir-ledger`: the additive `drain_state` DTO + `FORMAT_VERSION` bump (repo is
  local; push waits for the owner's `gist-rs/riir-ledger` remote).
- `riir-refine`: the day-state + `allow_hosted()` pre-check behind `points_drain`
  (inert without served constants), the `StandReason::DrainExhausted` arm + the
  disclosure line, the attestation header composer (the one header-setter law).
- `riir-instinct`: the seat's report-only day counters + the attestation verify +
  the enforcement hooks (armed only by ratified config) + the aggregate export.
- `riir-dapps`: the aggregate ingest route (idempotent by `(account, day)`) +
  `drain_state` on `/points/balance` + the debit-at-settle path (config-carried
  R/C, absent = inert).

### Deferred — to the joint owner ratification
- The live posture: ratified constants deployed, the feature promoted, the public
  copy un-roadmap-framed (Plan 324 G-B law gates that sentence, not this proposal).

### Explicitly NOT shipped by this proposal
Any KAT-plane change; TUNA marketing-plane changes; any purchase/transfer/faucet
path for points; chain anchoring changes (026 decision 6 stands); rank-rule changes;
the served reflex/rethink product economics; a client-side enforcement gate (never
— the client is open source).

## Phased rollout (sketch — a plan would expand this)

### Phase 1 — wire + client pre-check (riir-ledger, riir-refine)
- `points_drain` feature; day-state; `allow_hosted()`; `StandReason::DrainExhausted`
  + disclosure line; the attestation header composer; the G1 property tests +
  G4 canary (inert at this phase by construction — no served constants).

### Phase 2 — seat counting + service accounting (riir-instinct, riir-dapps)
- Seat report-only counters + attestation verify; the aggregate export; the plane
  ingest route; `drain_state` on balance; the debit-at-settle path; enforcement
  hooks wired but config-gated OFF; reconciliation test. The replay window is a
  FIXED `N = 128` (the usual IPsec size) pinned by a test — in-window unseen
  accepts, below-window rejects; wrapping it takes 128 distinct seqs per
  `(account, day)`, far above any real day.

### Phase 3 — gates, ratification, promotion
- G2 bench both postures; G3 full suites; read the report-only counts (the
  measured A/B sizing input AND the `P` sizing input — the per-source
  distribution); the joint owner ratification act (earn table + H + A/B/R/C/P in
  one sitting); deploy config; promote; publish the numbers on the fronts
  (Plan 324 G-B un-block).

## Risks

1. **Perf:** none on the modelless hot path by construction; G2 verifies the
   escalation-path delta. The seat-side check is µs-class against a remote service.
   No new network dial exists in the client fix run (the original draft's usage
   tail was dropped with the self-reported meter).
2. **Correctness:** negative-lock is impossible by invariant (property test);
   aggregate idempotency is the watermark posture; the two cache leaks are bounded
   and disclosed (caveat 2); the counter-order and day-rollover pins close the
   reviewer-found defect classes.
3. **Architectural:** the meter spans four repos — the wire is the contract and
   `riir-ledger` stays a zero-dep leaf; the instinct↔dapps edge is HTTP data
   crossing with zero cargo deps (the one-way rule); no game crate ever names any
   of it (026's leaf-law check carries over unchanged).
4. **Adoption-optics:** a metered hosted lane must never read as "the product got
   worse" — the decline keeps the modelless answer (the primary path), and the
   disclosure line says contribute, not pay. The public copy stays roadmap-framed
   until ratification.
5. **Availability:** the seat is already the availability bottleneck of the hosted
   lane (a dead peer surfaces as a decline today); the drain adds no new client
   coupling. The counter-store outage fails OPEN for anonymous/per-source traffic
   and CLOSED for the billable tier (the anonymous posture serves billable
   callers for the outage) — the caller is never broken and spend is never
   unbounded.

## Out of scope

Rank rules (026 §5 stands, `H` ratifies alongside); the earn table's numbers;
TUNA marketing issuance; the KAT plane and its frozen lanes; crates.io publication
(T8, owner); the `gist-rs/riir-ledger` remote creation (owner); chain-anchored
epoch roots / witness mirrors (owner-optional hardening, 026 §6); the band-fee
variant (the named revisit lever).

## References

1. *ID-based rate limiting* — slashid engineering blog, GCRA as the
   memory-efficient token-bucket-family algorithm (cited-only):
   <https://www.slashid.dev/blog/id-based-rate-limiting>
2. Token bucket / leaky bucket surveys — the AWS/Stripe/GitHub-adopted rate-limit
   family (cited-only): <https://bytebytego.com/courses/system-design-interview/design-a-rate-limiter>
3. In-workspace: Proposal 025 (the bar), Proposal 026 (the deferral + plane), issue
   192 (GAP 4 laws), Plan 061 (the service plane), Plan 325 (the client), Plan 225
   T5.2 (the production wiring),
   `../riir-dao/crates/riir-dao/tests/envelope_gate.rs` + the riir-dao
   AGENTS envelope row (the envelope doctrine), issue 075 (the soft-gate shape,
   closed — git history), the v1 decision_wire freeze (riir-reflex issue 074 —
   why identity rides headers, not JSON).

## TL;DR

Enforce at the seat (the one ledger: signed per-call attestations counted
per-`(account, day)`), pre-check on the client (O(1), network-free, policy-verb
decline through `StandReason::DrainExhausted`), never meter local `fix`, close the
unbounded-day hole with `B`, and ship everything inert behind `points_drain` until
the joint owner ratification pins A/B/R/C with the earn table — with pre-ratification
report-only seat counting collecting the measured usage the ratification should be
sized from. Next action: a plan pair (riir-refine + riir-instinct/riir-dapps phases
above) after the owner ratifies — or immediately for Phases 1–2, which need no
numbers.
