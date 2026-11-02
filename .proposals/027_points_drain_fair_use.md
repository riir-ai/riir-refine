# Proposal 027 — the points drain: the hosted-lane fair-use budget (rates, cap, refill, the O(1) gate)

**Status:** Proposed — the §4 follow-up [Proposal 026](026_points_plane_riir_ledger.md) defers (issue 192 GAP 4); this proposal owns the numeric fair-use policy (drain rate, balance cap, refill paths) and the gate mechanism. The numbers below are the **PROPOSED defaults** — one owner ratification act pins them alongside the earn table + rank half-life (026 §3/§5.2), and nothing drains before that act.
**Claimed:** zed-m3-drain, 1793593066, expires 2026-11-04T04:17:46+00:00

## TL;DR

Hosted lanes in cargo refine — the `seat_rerank` escalation seat (ARMED in the clippy domain manifest) and the `hosted_expert` L4 generate (armed when a manifest row arms it) — meter under a **free daily allowance + post-hoc billable-day fee + capped spendable balance, refilled only by proven contribution**. The gate is a client-side cached **O(1) integer check** on the escalation path only (the modelless hot path never executes it — met by construction, per the Proposal 025 bar); exhaustion declines typed (`DrainExhausted` → the existing `StoodByModelless` path) and **local `fix` is never metered** (binding law 1). Mechanism family: textbook token-bucket/GCRA fair-use (cited-only); the contribution-only refill + the rank-decoupled debit are ours. Ship the shape behind an opt-in feature that is **inert until the owner ratifies the constants**; the drain itself is the owner-gated act.

## The problem this solves

Proposal 026 shipped the points plane W0 with the drain deliberately out (§4: "v1 public copy leaves the drain OUT entirely"), and Plan 061 + Plan 325 landed the wire, the routes, and the client. Meanwhile Plan 225 T5.2 (`2623f4d09`) wired the escalation dispatch at the production serve boundary — **hosted `seat_rerank` calls are real usage today**, unmetered. Without the drain policy:

1. "points meter hosted lanes as fair use" (the only sanctioned public sentence, Plan 324 G-B law) is copy with no mechanism behind it.
2. Hosted cost grows unbounded with adoption — the one plane where a user can extract continuous spend without contributing anything.
3. The honest-targets discipline (shape ships, numbers ratified) has no vehicle for the drain half: 026 §8 names this proposal as the gate, and no vehicle exists yet.

## What meters (and what never does)

| Surface | Metered? | Why |
|---|---|---|
| `seat_rerank` escalation decisions | **YES** | the hosted instinct seat — real usage today (armed manifest row) |
| `hosted_expert` L4 generations | **YES** (when armed) | same plane, same counter; promote drain-on BEFORE expert-on (caveat 4) |
| local `fix` (the modelless path itself) | **never** | binding law 1 — local healing is always free, never blocked |
| sync, `/points/balance`, `/points/rank`, `/points/info`, join/leave | never | reads and identity plumbing; metering them would punish contribution |
| the `cargo refine upgrade` / corpus-lease lanes | never | the KAT plane, frozen and separate (issue 192 T16.7) |
| the served reflex/rethink products | never | separate product lane with its own economics (Proposal 014) |

## The proposed design

### The gate (client) — the Proposal 025 bar, met by construction

One machine-local day state beside `client_id.rs`'s store:
`{ utc_day, hosted_calls_today, billable_day: bool, cached_balance: Option<u64> }`.

The check sits in the escalation dispatch path (`escalation_dispatch.rs`), **after** the
modelless MISS is established and **before** the seat/expert call:

```text
allow_hosted():
    roll utc_day (reset counters on day change)
    if hosted_calls_today < A            -> allow (free allowance), hosted_calls_today += 1
    else if cached_balance > Some(0)     -> allow (billable), billable_day = true
    else                                 -> decline DrainExhausted
```

Two integer compares; no allocation, no network, no I/O on the decision path — the
≤ 1 ms p99 bar is met by construction and measured anyway (GOAT G2). The modelless
hot path (the fix itself, the drafter, the pruner) never executes this code.

**Decline shape:** a new `EscalationDecline::DrainExhausted` arm → the existing
`EscalatedFix::StoodByModelless` answer (the caller serves the modelless fix; nothing
panics, nothing blocks), plus ONE run-log disclosure line naming the two refuel paths
("contribute for hosted runway — the issue-075 soft-gate shape"). Local `fix` output
is byte-identical whether the gate allows, declines, or is compiled out.

**Cache semantics (deliberate):** `cached_balance` refreshes on any successful
`/points/balance` read or usage ack. It may be stale-positive by up to the
staleness window; the server reconciles the true balance post-hoc. The leak is
bounded by `staleness × local escalation rate` — bounded in practice by the advisory
ladder's own ceilings (`max_rate` sticky latch) — and DISCLOSED here, not hidden
(caveat 2). Anonymous or never-synced callers have `cached_balance = None`:
allowance-only.

### The accounting (server)

- **`points_usage` row family** keyed `(account, utc_day)` in the Plan 061 points row
  family: `hosted_calls`, `billable`. Reported by `POST /points/usage` (account-key
  signed; **idempotent by `(account, day)`** — the burn-sync monotone-watermark
  posture: a replayed day is a no-op, never a double debit). Rides a best-effort tail
  with the burn-sync posture exactly: ~5 s budget, never blocks the run, consults the
  offline gate + consent.
- **Debit at processing:** a billable day debits `R` from the balance, **floored at 0**
  — points never go negative-locked (issue 192 GAP 4 law; the floor is an invariant
  with a property test, not a convention). Integer math only; no floats anywhere near
  the money path (the dao law).
- **Balance cap `C`:** the mint clamps `minted = min(earned, C − balance)`; the excess
  stays in the PERMANENT contribution rows. **Drains and the cap never touch rank
  credit** — the rank view (026 §5) reads contribution history, not the spendable
  balance, so heavy hosted usage cannot collapse a contributor's rank.
- **`/points/balance` gains `drain_state`** (additive serde, the `pay` precedent):
  `{ allowance_remaining_today, billable_today }`. Refusals keep the
  `ErrorEnvelope` shape.

### The numbers — ratio logic (why these constants)

| Constant | Proposed | Logic |
|---|---|---|
| `A` — free daily hosted allowance | **200 / UTC-day** per client | `seat_rerank` fires only on modelless-MISS ∩ pool ≥ `min_pool`; real heavy usage is tens/day. 200 is generous fair use AND bounds a UUID farm to 200/day/identity (the honest limit: `reset-client-id` exists as a privacy right and will not be revoked — caveat 3) |
| `R` — billable-day fee | **2 points / billable day** | day-flag, not per-call: un-gameable, trivially deterministic. Under any plausible ratified earn (10–30/epoch ≈ 1.4–4.3/day), a hosted-heavy contributor pays ≈ half their earn rate — contributing stays above water; pure consumption drains |
| `C` — spendable balance cap | **90 points** | the envelope doctrine (riir-dao Bench 005 shape): the cap must drain in **[30 days, 2 years]** at the sustained billable rate — 90 / 2 = 45 days ✓ |
| join grant | **none** | points come only from proven contribution; no faucet, no packs, no purchase — the D4 doctrine shape: contribute or the modelless path (which is the product's primary path anyway) |

The envelope bounds [30 days, 2 years] are the INVARIANT; the absolutes are
provisional and re-derived at the joint ratification if the earn table lands
elsewhere.

### Wire changes (riir-ledger, additive)

`UsageReport`, `DrainState` DTOs; `FORMAT_VERSION` bump rides the additive-default
law (the `pay` mirror precedent — absence of new fields decodes byte-identically on
today's readers). No boundary changes: riir-ledger (wire), riir-dapps (service rows +
routes), riir-refine (client gate + tail) each already own their surface from 026 /
Plan 061 / Plan 325.

### Feature flag + promotion

`points_drain` — opt-in, implied by nothing, never in a default or release set
initially. **Fail-closed absent-config:** the gate compiles but refuses ALL debits and
allows the free allowance only when the ratified `drain_config` (A/R/C) is absent —
the `quest_caps` shape (malformed/absent config refuses the metered behavior, never
invents numbers). The constants ship as DEPLOYMENT CONFIG on the service, not code —
a ratified re-tune never needs a client release (the client needs only `A`, served in
`drain_state`; `R`/`C` are server-side). Demote-only env `RIIR_REFINE_POINTS_DRAIN=0`
(the promoted-arm env law). Promotion requires: the joint owner ratification + the
GOAT gate below.

## Honest caveats — READ BEFORE IMPLEMENTING

1. **The earn numbers do not exist yet.** `R` and `C` are ratios to an unratified
   table; this proposal's absolutes are placeholders pending the joint act. If the
   earn table lands at a very different scale, re-derive `R`/`C` from the envelope
   bounds — never ship the placeholders by inertia.
2. **The stale-cache leak is deliberate, not an oversight.** Post-hoc accounting is
   the pinned bar (Proposal 025); the cost is a bounded over-admission through a
   stale-positive cache. If dispatch receipts ever show abuse, the tightening lever
   is a max-staleness bound on `cached_balance` (refuse when older than N hours) at
   the cost of more balance reads — a config change, not a redesign.
3. **The anonymous allowance is farmable per-UUID.** `config reset-client-id` is a
   privacy right (T16.1: resettable by design) and stays. The mitigation is the bound
   itself (200/day/UUID) + the dispatch receipts as the detector; the named
   escalation is a joined-only-allowance amendment — REFUSED for v1 (it would couple
   basic utility to identity; the privacy law prefers allowance-for-all).
4. **Promotion ordering is load-bearing:** `points_drain` must promote BEFORE (or
   with) any `hosted_expert` manifest arming. The meter must never ship after the
   spend it meters.
5. **One meter, one ledger.** The usage day row is the ONLY hosted-usage meter; the
   KAT burn tail stays untouched (separate plane, the one-way rule, T16.7 frozen).
   A second meter would fork the accounting and the audit story.

## Fusion lineage

- [Proposal 025](025_mozilla_human_scale_ai_incubator.md) §perf/sec posture (the
  cached-balance O(1) pre-check bar) × [Proposal 026](026_points_plane_riir_ledger.md)
  §4 (the deferral mandate) → this design.
- **Issue 075's depleted-account soft gate** (refuse + name the refuel paths,
  server-truth-only, kill-switches) → the decline UX shape, reused verbatim.
- **riir-dao envelope gate** ([Bench 005](../riir-dao/.benchmarks/005_g16_post_d1_grant_remeasure.md),
  the [7 days, 2 years] drain doctrine) → the cap parameterization.
- **`escalation_dispatch::StoodByModelless`** (Plan 202 R6, wired by
  [Plan 225](../.plans/225_heal_seat_expert_bench.md) T5.2) → the typed decline the
  drain reuses; zero new refusal machinery.
- GCRA / token-bucket (references 1–2, cited-only) → the O(1) constant-space
  enforcement family.

## GOAT gate

- **G1 correctness (property tests):** the balance floor is never crossed (no debit
  can produce a negative); drain-off boots are byte-identical; an exhausted-points
  client's local `fix` output is byte-identical to no-gate; a replayed usage day is a
  no-op; absent/malformed `drain_config` meters nothing.
- **G2 perf:** the gate adds zero network + zero alloc to the escalation path;
  measured dispatch overhead delta ≤ 1 ms p99 against the pre-gate posture (the 025
  bar) — expected µs-class. The modelless hot path is untouched by construction and
  the G3 suite re-pins its counts.
- **G3 no regression:** full suites at drain-off byte-identical (the flag-off
  posture is a tested posture, never assumed); `cargo refine --score-bench` if any
  proposer-adjacent file is touched.
- **G4 alloc-free:** the gate path allocates nothing (counting-allocator canary).
- **Accounting determinism:** the same usage rows produce bit-identical debits
  (integer math only); the dapps-side recompute is a gate, in the
  `dapps_contract_pins` shape.

## What ships now (riir-refine + riir-ledger + riir-dapps) vs deferred

### Ships now — the shape, inert until ratification
- `riir-ledger`: the additive drain DTOs + `FORMAT_VERSION` bump (repo is local;
  push waits for the owner's `gist-rs/riir-ledger` remote).
- `riir-refine`: the day-state + `allow_hosted()` gate behind `points_drain`
  (fail-closed absent-config), the `DrainExhausted` decline arm, the usage tail.
- `riir-dapps`: `points_usage` rows + `/points/usage` + `drain_state` on
  `/points/balance` (config-carried constants, absent = inert).

### Deferred — to the joint owner ratification
- The live posture: ratified constants deployed, the feature promoted, the public
  copy un-roadmap-framed (Plan 324 G-B law gates that sentence, not this proposal).

### Explicitly NOT shipped by this proposal
Any KAT-plane change; TUNA marketing-plane changes; any purchase/transfer/faucet
path for points; chain anchoring changes (026 decision 6 stands); rank-rule changes;
the served reflex/rethink product economics.

## Phased rollout (sketch — a plan would expand this)

### Phase 1 — wire + client gate (riir-ledger, riir-refine)
- `points_drain` feature; day-state; `allow_hosted()`; the decline arm + disclosure
  line; the usage tail (offline/consent-gated, 5 s budget).
- G1 property tests + G4 canary at this phase.

### Phase 2 — service accounting (riir-dapps)
- `points_usage` rows; `/points/usage` (idempotent by `(account, day)`);
  `drain_state` on `/points/balance`; config-carried A/R/C with absent = inert;
  the debit-at-processing path + determinism test.

### Phase 3 — gates, ratification, promotion
- G2 bench both postures; G3 full suites; the joint owner ratification act
  (earn table + H + A/R/C in one sitting); promote; publish the numbers on the
  fronts (Plan 324 G-B un-block).

## Risks

1. **Perf:** none on the hot path by construction; G2 verifies the escalation-path
   delta. The usage tail is a new network dial in the fix run — bounded by the
   burn-sync posture (budget, offline, consent) and never on the decision path.
2. **Correctness:** negative-lock is impossible by invariant (property test);
   replay idempotency is the watermark posture; the stale-cache leak is bounded and
   disclosed (caveat 2).
3. **Architectural:** the meter spans three repos — the wire is the contract and
   `riir-ledger` stays a zero-dep leaf; no game crate ever names any of it
   (026's leaf-law check carries over unchanged).
4. **Adoption-optics:** a metered hosted lane must never read as "the product got
   worse" — the decline keeps the modelless answer (the primary path), and the
   disclosure line says contribute, not pay. The public copy stays roadmap-framed
   until ratification.

## Out of scope

Rank rules (026 §5 stands, `H` ratifies alongside); the earn table's numbers;
TUNA marketing issuance; the KAT plane and its frozen lanes; crates.io publication
(T8, owner); the `gist-rs/riir-ledger` remote creation (owner); chain-anchored epoch
roots / witness mirrors (owner-optional hardening, 026 §6).

## References

1. *ID-based rate limiting* — slashid engineering blog, GCRA as the
   memory-efficient token-bucket-family algorithm (cited-only):
   <https://www.slashid.dev/blog/id-based-rate-limiting>
2. Token bucket / leaky bucket surveys — the AWS/Stripe/GitHub-adopted rate-limit
   family (cited-only): <https://bytebytego.com/courses/system-design-interview/design-a-rate-limiter>
3. In-workspace: Proposal 025 (the bar), Proposal 026 (the deferral + plane), issue
   192 (GAP 4 laws), Plan 061 (the service plane), Plan 325 (the client), Plan 225
   T5.2 (the production wiring), riir-dao Bench 005 (the envelope doctrine), issue
   075 (the soft-gate shape, closed — git history).

## TL;DR

Ship the drain SHAPE behind opt-in `points_drain`, fail-closed and inert until the
joint owner ratification pins A/R/C with the earn table; the gate is an O(1) client
check that only ever declines to `StoodByModelless`; local `fix` is never metered.
Next action: a plan pair (riir-refine + riir-dapps phases above) after the owner
ratifies — or immediately for Phases 1–2, which need no numbers.
