# Issue 045 — container cost observability + hourly housekeeping worker + smallest-sizing verdict

**Filed:** 2026-08-26
**Status:** OPEN — T4 (owner secrets) + T6 (external ledger shell) remain; T5 RESOLVED (cross-repo correlation) + T7 read-path hardening SHIPPED 2026-08-28. Issue 053 (unattributed app `a03a70ff`) filed **and RESOLVED same day** by cross-repo correlation — it is the `riir-mmorpg-examples` CCU proof bench (Bench 034); resolved-and-removed per the noise rule, record in §T5 + git history. T1 verdict recorded, T2+T3 SHIPPED same day; live-mint half gated on Proposal 005 Phase 1. Sizing verdict feeds Proposal 005 T2.0/T2.1 **+ T2.6 / [Plan 031](../.plans/031_kat_sweeper_temp_container_r2_tier.md)** (2026-08-26 later-session sync). **2026-08-27: the T3 Worker was EXTENDED + redeployed (`ab79bd20`) — CORS on all read endpoints (dashboard live-fetch), Issue-044-T4 budget alerting (cap thresholds 50/90/100% + optional monthly budget, KV-dedupe, webhook via `ALERT_WEBHOOK_URL`), extended `/status` (`cap`/`thresholds`/`has_webhook`/`has_monthly_budget`/`alerts_today`).** **2026-08-28 truth-refresh: Proposal 005 Phase 1 LANDED 2026-08-27 (T1.5 dapps `84dd049` / T1.6 clippy `0a7cfd8`) — T6's blocker is NO LONGER Phase 1; the live-mint swap-in awaits the deferred Worker/DO HTTP ledger shell (Proposal 005 phase-2 defer / Plan 031 Phase B gate / phase-3 owner gate). Worker mint status strings refreshed `pending-phase-1` → `pending-worker-shell` + redeployed (version `f93ae507`, live-verified on the 04:00 UTC tick). T4 unchanged (owner-gated secrets). T5 still blocked — CLI exposes no id→name mapping (see checkbox).**
**Repo:** riir-clippy (dashboard + worker deploy live here; sizing verdict feeds Proposal 005 T2.0/T2.1)
**Owner ask (2026-08-26):**
1. can the container be smaller? smallest?
2. container sleep/awake/usage as weekly/daily/hourly graphs on
   <https://riir-ops-dashboard.foxfox.workers.dev/>
3. hourly scheduled housekeeping — container cost + KAT mint — via Worker
   (cheaper) or the smallest possible trainer-node container; reuse the
   existing one if feasible, else Worker.

---

## T1 — sizing verdict (researched against live docs + API, 2026-08-26)

### The instance-type ladder (CF pricing page, verified 2026-08-26)

| Instance type | vCPU | Memory | Disk |
|---|---|---|---|
| **lite** | 1/16 | 256 MiB | 2 GB |
| basic | 1/4 | 1 GiB | 4 GB |
| standard-1 | 1/2 | 4 GiB | 8 GB |
| standard-2 | 1 | 6 GiB | 12 GB |
| standard-3 | 2 | 8 GiB | 16 GB |
| standard-4 | 4 | 12 GiB | 20 GB |

Custom types (≥1 vCPU) exist since 2026-01. **`lite` is the smallest
predefined type.**

### The billing model (the part that decides "smaller")

Workers Paid ($5/mo) includes monthly: **25 GiB-h memory, 375 vCPU-min CPU,
200 GB-h disk**; beyond that $0.0000025/GiB-s mem, $0.000020/vCPU-s CPU
(active only), $0.00000007/GB-s disk. Billed per 10 ms **while running**;
charges stop at sleep. Memory+disk bill on *provisioned* resources per
wall-second of run; CPU bills on *active usage* only.

- Included allowances ÷ lite footprint ≈ **100 lite-hours/month free** after
  the $5 plan. A daily ≤1 h drain window fits easily.
- **Free plan: Containers are N/A** — deploying any container (trainer or
  housekeeping) requires the $5/mo Workers Paid upgrade. Owner gate.

### Verdict per surface

1. **Hourly housekeeping → a WORKER, not a container.** Worker cron triggers
   are free (3/worker on Free plan; scheduled runs don't consume the
   100k/day request budget), KV writes 24/day ≪ 1k/day free. A container
   would cost the $5 plan + uptime for zero capability gain — the
   housekeeping loop is 2 GraphQL reads + 1 KV write. **T3 ships this.**
2. **Trainer container (Proposal 005 T2.0/T2.1) → `basic` floor, not lite.**
   The drain's verify half needs `rustc` (`fix_verify` compile gates):
   - lite's 256 MiB cannot hold a rustc + target dir for our repos —
     OOM-class, not slow-class.
   - lite's 1/16 vCPU is the exact throttle the mmorpg authority measured
     (12–16× intended period, Issue 030) — CPU-bound work is disqualified
     by measurement, not opinion.
   - **The counter-intuitive cost fact:** billable CPU-seconds are roughly
     *work-constant* (same compile ≈ same vCPU-seconds at any size), while
     memory+disk bill per *wall-second* — so a larger instance that
     shortens the window can be **cheaper overall** when mem/disk dominate.
     For compile-bound windows, sizing down to save money is exactly
     backwards unless CPU dominates.
   - Recommendation pinned into Proposal 005 T2.0/T2.1: start at `basic`
     (1/4 vCPU, 1 GiB) for a verify-light drain; step to `standard-1`
     (4 GiB) if real compile gates run in-window; measure cpuTimeSec vs
     mem GiB-s from this dashboard's graphs and re-pin.
3. **"We already have one" — checked, honestly: no.** `wrangler containers
   list` → *No containers found*; the two Workers with container bindings
   (`hello-cloudflare-container`, `seal-poc-container`) have **no live
   Application** behind them (`containers info` → not found; those are
   deleted-app leftovers from 2026-08-14 Bench 331 / earlier POCs — their
   bindings are inert declarations). Nothing to reuse: T3's executor is a
   **Worker** anyway (cheaper than any container for hourly housekeeping).

### Live finding the research surfaced (owner should see)

> **SUPERSEDED 2026-08-28 — this paragraph was measured on 2026-08-26 with
> the truncating collector; three of its numbers are wrong. Corrected
> restatement follows below. Kept for provenance.**

`containersUsageAdaptiveGroups` shows **3 container-family apps running
TODAY** (a03038cf…, a034b17e…, a035c2cc… — app ids, not names; the GraphQL
schema exposes no name mapping and `wrangler containers list` shows none).
Derived shapes: one ≈ standard-1, one ≈ standard-1, one ≈ lite; combined
burn ≈ **$0.01–0.03/day**. 2026-08-14 rows match the Bench 331 deploy pair
(deleted). Likely Cloudflare-Agents/Sandboxes-family workloads from other
tooling on this account. The dashboard (T2) makes them visible; naming them
needs dashboard-side correlation — left as a follow-up, not a blocker.

### Corrected live finding (2026-08-28, full-id collector)

Three corrections, all measured against the same GraphQL datasets:

1. **Count: 6 apps over 30 d, not 3.** The 08-26 window has **4**
   (a03038cf, a034b17e, a035c2cc, **a03a70ff**); 08-14 has 2
   (a03c634b, a033f790 — the Bench 331 pair). The original list of 3
   **omitted `a03a70ff`, the single largest consumer.**
2. **Cost: ~11–35× understated.** Not "$0.01–0.03/day" — the measured
   **2026-08-26 burn is $0.3453**, and `a03a70ff` alone is **$0.3292
   (94% of all container spend)**: 117,149 GiB-s memory, 998.5 vCPU-s,
   **17.7 GB egress** in one day, colo `yyz04`, 3 deployments. The
   30 d total across all 6 is **$0.3780**. Still small in absolute
   terms — but the observability claim "we can see our container
   spend" was false while the dominant line item was missing.
3. **"Running TODAY" is stale.** Zero rows on 2026-08-27 **and**
   2026-08-28; last container activity anywhere on the account is
   **2026-08-26T22Z**. Nothing is running now.

Correlation material now retained (was discarded by the `[:8]` truncation):
`a035c2cc` and `a034b17e` **share durableObjectId `8d3b8a1d…`** and colo
`atl15` — two billed apps behind one Worker/DO, not two independent
workloads. `/containers/me` reports account container defaults of
**0.0625 vCPU / 256 MiB / 2000 MB disk** — an independent confirmation of
the T1 `lite` row.

---

## T2 — dashboard container graphs (SHIPPED 2026-08-26)

`scripts/gen_dashboard.py` gained `collect_containers()`:

- **Source:** CF GraphQL `containersUsageAdaptiveGroups` (billing-grade;
  byte-second units verified empirically against instance sizes — see
  `.benchmarks/` note in this issue) + `containersMetricsAdaptiveGroups`
  (hourly `containerUptime` for the sleep/awake view, `active` dimension).
- **Cost model (pinned constants):** mem GiB-s × $0.0000025 + disk GB-s ×
  $0.00000007 + vCPU-s × $0.000020; included allowances (25 GiB-h / 375
  vCPU-min / 200 GB-h per month) shown as the free tier.
- **Views:** hourly uptime (last 48 h, awake vs sleep per app), daily
  uptime+cost (14 d), weekly cost (4 w), per-app table (uptime h, cpu s,
  est cost), KPI row (24 h / 7 d est cost + awake hours).
- Degrades gracefully (no token / no rows → honest placeholder), same as
  every other source. Regenerated + redeployed.

## T3 — hourly housekeeping Worker (SHIPPED 2026-08-26)

`cloudflare/housekeeping/` — **`riir-kat-housekeeping`** (deployed):

- **Cron:** `0 * * * *` (hourly). Free-tier compatible (1 of 3 triggers/worker on the
  Free plan; cron invocations DO count as requests — 24/day is noise against
  the 100k/day budget). KV writes ~27/day ≪ 1k/day free.
- **Each run:** query the two container datasets for the trailing hour +
  `aiInferenceAdaptiveGroups` for the day's neurons → compute cost →
  appendimmutable KV rows (`hour:<YYYYMMDDHH>`, `day:<YYYYMMDD>` aggregate,
  `latest`) → write the **KAT mint anchor** row (`mint_anchor:<date>`:
  neurons-to-date + the ledger-row totals that §Economics of Proposal 005
  names as the mint function's input).
- **Mint:** the *live* mint (chain `chain_prog_kat` + ledger DO) is
  Proposal 005 **Phase 1** (T1.1–T1.4) — not yet built, so v1 records the
  anchor + reports `mint: "pending-phase-1"`. The swap-in point is one
  function (`recordMintAnchor`) — no cron/topology change at Phase 1.
- **Auth:** needs a dedicated CF API token (Account → Analytics : Read) as
  the `CF_API_TOKEN` secret — the wrangler OAuth token rotates hourly and
  cannot be a Worker secret. **One-time owner step:**
  `cd cloudflare/housekeeping && npx wrangler secret put CF_API_TOKEN`.
  Until set, the worker still writes heartbeat rows (proves cron alive;
  doubles as the Issue 044 T9 staleness signal).
- **Read API for the dashboard/future live mode:** `GET /latest`,
  `GET /hours?n=24`, `GET /days?n=14`, `GET /status`.

### Why Worker over "smallest trainer container" for housekeeping

Hourly housekeeping is 2 GraphQL reads + 1 KV write ≈ milliseconds of CPU.
A container — even lite — pays the $5 plan gate + per-run uptime + cold
start (~1–3 s) to do milliseconds of work. The Worker is $0, always-warm,
and is the same executor shape Proposal 005's payment/ingest workers use.
The trainer container remains Phase 2 (T2.0/T2.1) where its sizing verdict
lives (T1 above).

## Tasks

- [x] T1 sizing verdict recorded (this issue §T1) + pinned into Proposal
      005 T2.0/T2.1
- [x] T2 `collect_containers()` + dashboard section (hourly/daily/weekly +
      sleep/awake + est cost) + regenerate + deploy
- [x] T3 housekeeping Worker deployed (cron hourly, KV rows, mint anchor,
      heartbeat fallback, read endpoints)
- [ ] T4 owner: `wrangler secret put CF_API_TOKEN` (Analytics:Read) —
      housekeeping data collection goes live *(2026-08-27: `ALERT_WEBHOOK_URL`
      + optional `MONTHLY_BUDGET_USD` join the same one-time step — the
      alerting mechanism is deployed + inert until then; without the
      webhook, crossings still land in KV + `/status alerts_today`)*
      *(2026-08-28: `READ_TOKEN` joins the same step — optional, gates
      `/hours` + `/days` + the `/status` config half. Those three FAIL OPEN
      until it is set, deliberately, so `token_set` stays readable as the
      check that this very step landed. `/latest` is already hard-gated and
      needs no secret. Verify after setting:
      `curl -s .../status` → `read_gated: true`, and
      `curl -s .../hours` → 401.)*
- [x] T5 name the container-family apps — **RESOLVED 2026-08-28 by
      cross-repo correlation** (see the RESOLVED-BY-CORRELATION block at the
      end of this item). Unachievable by *API* — the apps are deleted by a
      documented teardown, so no server-side name exists — but the apps ARE
      named: `a03a70ff` is the riir-mmorpg-examples CCU proof bench
      (Bench 034), confirmed on colo + egress + instance class + date. The
      root cause of the original misdiagnosis was found and fixed, and the
      correlation surface shipped, en route.
      **2026-08-28 second re-probe corrects the first one's premise.** The
      earlier probe fed `wrangler containers info` an **8-char stub**
      (`a03038cf`) — but that stub was never the id. It is a *display
      artifact* of our own `gen_dashboard.py`, which did
      `(dimensions["applicationId"])[:8]`. The real ids are 36-char UUIDs;
      the "ids are unaddressable" conclusion was an artifact of our
      truncation, not of the CF API. Re-probed correctly (OAuth token
      carries `containers (write)` + `cloudchamber (write)`):
      - `GET /containers/applications` → `[]` (empty, HTTP 200).
      - `GET /containers/applications/<FULL-UUID>` → HTTP 404 **`code 1609
        "Container application not found"`** for all 6 — a *well-formed*
        lookup, not a malformed one. These are **deleted** applications:
        GraphQL retains their billing rows, the control plane has no
        record, so **no name exists server-side to fetch**. That is the
        real, positive reason T5 cannot close — it replaces the wrong one.
      - `label(name:)` **is** usable with an argument (the prior note said
        "unusable bare" — correct, but it takes a key). Probed 20+ keys
        (`name`, `app`, `worker-name`, `script-name`, `class`, `image`,
        `managed-by`, …): **every one empty**. Keys >~22 chars or with dots
        are rejected outright. Labels are not the path.
      - `/cloudchamber/applications` → 401 "Account is not authorized";
        `/containers/deployments` → 7003 no route. No deployments surface.
      - Control plane cross-check: only **2** DO namespaces are
        container-backed (`hello-cloudflare-container_MyContainer`,
        `seal-poc-container_MyContainer`) against **6** billed apps —
        consistent with same-day create/delete (Sandboxes/Agents-family),
        i.e. ephemeral apps that never had a durable name here.
      **What shipped instead** (the "dashboard-side correlation" this
      checkbox always named as the path): `collect_containers()` now emits
      a `meta` map — **full applicationId** + `durableObjectId` +
      `deploymentId` + `location` per app — and the per-app table renders
      full id / backing DO / colo. Live-verified on the dashboard. Reopen
      only if CF adds a name-bearing dimension or a retained-app lookup.
      **2026-08-28 RESOLVED-BY-CORRELATION — the apps are named after all,
      just not by any CF API.** The answer was in a sibling repo's git
      history, not in the control plane. `a03a70ff` is the
      **`riir-mmorpg-examples` CCU proof-bench container** (its Issue 084 /
      [Bench 034](../../riir-mmorpg-examples/.benchmarks/034_ccu_proof_bench.md),
      Proposal 040 T4.3; commits `a10c81c` + `3dcb3d7`, 2026-08-26T23:34 UTC).
      Four independent signals agree:
      | signal | billing (GraphQL) | Bench 034 |
      |---|---|---|
      | colo | `yyz04` | "`yyz04` instance observed" |
      | egress | 17.66 GB | 16.2 GB over the ladder |
      | instance class | standard-2 | `standard-2` |
      | date | 2026-08-26 | 2026-08-26 |
      Billing implies ~19,525 standard-2 instance-seconds against 915 s of
      measured wall uptime → **~21 concurrent instances**, which is just what
      a CCU ladder autoscaling looks like.
      **And this explains the deletion.** Bench 034 §Teardown: "CF worker +
      container app deleted (`wrangler delete` + `wrangler containers
      delete`) immediately after measurement". The `code 1609` 404s, the
      empty `containers list`, and the absent DO namespaces are all one
      cause — **deliberate, documented teardown**, not a missing API. T5 was
      never a CF limitation; it was a cross-repo attribution question, and
      the attribution key is repo git history + bench records.
      *Honest scope:* `a03a70ff` is confirmed on all four signals. The other
      three 08-26 apps (`a03038cf` cmh01, `a035c2cc` + `a034b17e` atl15 —
      the latter two sharing DO `8d3b8a1d…`) are **plausible same-campaign
      deploys** — Bench 034 F2 records a stale-image rebuild/redeploy cycle,
      and CF places attempts across colos — but they are not independently
      confirmed, since only the final deploy's colo is recorded. Ranked
      `lite` (283 s), standard-1/2 (741 s), standard-2 (595 s); together
      **$0.0161**, i.e. noise.
      **Method note for the next agent:** the CF control plane cannot name a
      torn-down app, but our own repos can. Correlate colo + instance shape
      + egress + date against sibling `.benchmarks/` and `git log --since`
      BEFORE concluding an app is unattributable.
- [ ] T6 live-mint swap-in — Proposal 005 Phase 1 (the sync surface)
      landed 2026-08-27 (T1.5 dapps `84dd049` / T1.6 clippy `0a7cfd8`),
      so the blocker is NO LONGER Phase 1: the swap awaits the deferred
      Worker/DO HTTP ledger shell (Proposal 005 phase-2 defer / Plan 031
      Phase B gate / phase-3 owner gate). Then swap `recordMintAnchor`
      (this object is already the designed mint payload) for the real
      mint call (`ledger DO` / `chain_prog_kat`). Worker status strings
      refreshed to `pending-worker-shell` 2026-08-28 + deployed
      (`f93ae507`).
- [x] T7 read-path hardening — prod-grade perf/sec pass on the T3 Worker
      (2026-08-28; §T7 below). Deployed `ca93c8de`, live-verified.

## T7 — read-path hardening (SHIPPED 2026-08-28)

A prod-grade perf/sec pass over the T3 Worker. Seven findings, all fixed and
live-verified (`riir-kat-housekeeping` version `ca93c8de`).

### SEC-1 (headline) — unauthenticated KV read amplification

`/hours?n=` accepted **n=720** and fanned it out as **720 unbounded
concurrent KV gets**, on a route with no auth. Measured blast radius:

- **138 requests exhaust the 100k/day KV read tier** — ~14 s at a trivial
  10 req/s.
- Once exhausted the damage is not just "graphs empty": the alert-dedupe
  `kv.get('alert:<date>:<t>')` throws, is swallowed by the outer catch, and
  the run records `ok:false`. **Budget alerting degrades exactly when it
  matters** — an attacker (or a crawler) can silence the cap alarm for the
  rest of the UTC day for free.

Fixed: window capped (`MAX_HOURS=168`, `MAX_DAYS=90`) and fan-out bounded by
a `mapLimit(…, 16)` helper replacing `Promise.all`. Worst case is now 168
keys at concurrency 16, bounded by construction (`Math.min`), verified over
`n ∈ {720, -5, abc, 0, 1e9, null}`.

### SEC-2 — CORS was `*`

Any page in any browser could read this account's cost telemetry
cross-origin. Now an allowlist (`ALLOWED_ORIGINS` var, defaults to the
dashboard origin); a non-allowlisted `Origin` gets no ACAO header. Verified
live: `evil.example` → absent, dashboard origin → echoed.

### SEC-3 — public routes exposed account telemetry

`/latest` returned the whole row (per-app **ids**, per-app cost, neuron
counts); `/status` advertised config posture (`cap`, `has_webhook`,
`has_monthly_budget`, `alerts_today`) — i.e. whether alerting was even wired.

Split by what each consumer actually needs:

| route | anonymous | with `READ_TOKEN` |
|---|---|---|
| `/latest` | minimised projection (`ts`, `hour`, `ok`, `mint`, aggregate `est_usd`, `neurons_today`, `pct`) + `redacted:true` | full row |
| `/status` | liveness only (`worker`, `cron`, `ts`, `ok`) | + config posture |
| `/hours`, `/days` | 401 *(fails open until `READ_TOKEN` set)* | full rows |

`/latest` redacts **unconditionally** — it does not fail open — because the
projection is by construction everything the dashboard consumes, so
hard-gating costs no functionality. The other routes fail open so the owner
retains a diagnostic path (notably `token_set`) before any token exists.
Token compare is length-independent (`timingSafeEqual`) so a wrong token
cannot be recovered byte-wise from response timing.

### CORRECT-1 — the day aggregate was non-idempotent AND racy

`day:<dk>` did a read-modify-write: `prev.est_usd + this hour`. A cron
double-fire or any manual invoke **double-counted** the hour, and two
overlapping runs lose an update on an eventually-consistent store. Now
**recomputed** by summing the authoritative `hour:<dk>*` rows — idempotent
and convergent. Costs ≤24 bounded KV reads/run (~576/day, noise against the
100k tier).

### RESIL-1 — one failing dataset discarded the other two

`Promise.all` over the three collectors meant a single GraphQL failure threw
away usage + awake + neurons and wrote a bare `ok:false` row, losing an hour
of collectable data. Now `Promise.allSettled` with per-dataset notes; a
partial hour is recorded rather than dropped. `neurons` may now be `null`,
so the cap-alert branch is type-guarded.

### RESIL-2 — no timeouts

Neither the GraphQL fetch nor the alert webhook had one; a hanging endpoint
stalled the cron run. Added `AbortSignal.timeout` — 10 s GraphQL, 5 s webhook.

### PERF-1 — no caching, unbounded MTD scan

Read routes now send `Cache-Control` (60 s `/latest` + `/status`, 300 s
`/hours` + `/days`), cutting repeat KV reads from dashboard loads. The
monthly-budget MTD scan's per-key `get` loop is bounded by the same
`mapLimit`.

### Consistency — full applicationId in the Worker too

The Worker carried the same `applicationId.slice(0, 8)` truncation that
caused the T5 misdiagnosis; its KV rows now record full 36-char ids.
**Schema note:** `hour:*` rows written from this version key
`containers.apps` by full UUID, older rows by 8-char stub. Nothing consumes
that map yet (the dashboard reads GraphQL directly), so no migration is
needed — but a future consumer must tolerate both.

### Verification

`node --check`; unit tests for `mapLimit` (peak concurrency 16, order
preserved, empty-input safe) and `timingSafeEqual` (6 cases incl. unequal
lengths + unicode); `wrangler dev --local` for route behaviour (CORS
allow/deny, 405 on POST, 401 matrix with and without `READ_TOKEN`,
redaction projection against a seeded row); then live curl against the
deployed Worker. The dashboard live badge consumes only `ts` +
`ai.neurons_today`, both retained in the public projection — confirmed
still 200 after deploy.

## Notes

- GraphQL schema discovery (2026-08-26, live introspection): the container
  datasets are `containersUsageAdaptiveGroups` (billing) +
  `containersMetricsAdaptiveGroups` (uptime/active/bytes). The `label`
  dimension requires args and is unusable bare; group by `applicationId`.
  The wrangler OAuth token (refreshed via `wrangler whoami`) reads both —
  `account:read`-class scope suffices, same as the AI dataset.
- Unit derivation is empirical: allocatedMemory/allocatedDisk behaved as
  byte-seconds consistent with known instance footprints (lite/basic/
  standard-1 within ~2%). Cost figures are labeled *est* on the dashboard.
- Numbering: 045 (highwater 44 → 45).
- **2026-08-28 housekeeping status-truth refresh (this session):** worker
  mint status strings `pending-phase-1` → `pending-worker-shell` +
  `mint_note`/`note` fields added (phase-1 sync surface landed
  2026-08-27; live-mint blocker = the deferred Worker/DO ledger shell);
  README mint bullet updated the same way. Deployed live (wrangler
  OAuth, version `f93ae507-4612-4037-8ad4-e6ce75d345e6`) and verified on
  the 04:00:06 UTC tick via `/status` + `/latest` (still heartbeat mode:
  `token_set: false`, `has_webhook: false`, `alerts_today: 0` — owner
  secrets still unset, unchanged). T5 re-probed same day (see checkbox:
  CLI names nothing, ids unaddressable). No KV key or field-name changes;
  `kat` stays `null` (§Economics is a policy, not a formula — no KAT
  math in the worker).
- **2026-08-26 later-session sync:** (a) the T1 sizing verdict now also covers
  the **own-repo sweeper container** (Proposal 005 §The own-repo hourly
  sweeper lane / T2.6 — same `basic` floor / size-UP rule); (b) [Plan 031](../.plans/031_kat_sweeper_temp_container_r2_tier.md)
  Phase B **extends this Worker** with the Cron Container crank duty
  (`scheduled() → getContainer().start()` + poll heartbeat — still
  Worker-class work; this issue's Worker-not-container verdict is exactly
  why); (c) the LLM half of every future window is bounded by
  Issue 046 (`DEFAULT_LLM_DAILY_CAP_USD = 1.0`; resolved + removed
  2026-08-27, commit `8fce5b2` — the gate ships in `CfClient::generate`).
  T4/T5/T6 unchanged.

- **2026-08-28 T5 close-out + collector correctness fix (this session):**
  T5 re-probed properly and **deferred as unachievable by API** — the apps
  are deleted (`code 1609`), so no server-side name exists; the prior
  "ids unaddressable" reasoning was our own `[:8]` truncation, now fixed.
  Two real defects found and fixed in `scripts/gen_dashboard.py`:
  **(a) full-id retention + correlation `meta`** (applicationId /
  durableObjectId / deploymentId / location; per-app table shows full id,
  backing DO, colo) — this is the correlation surface T5 asked for;
  **(b) the zero-fill grid painted phantom buckets into the FUTURE**
  (daily ran to 2026-09-12 and hourly to 2026-08-28T16 while it was
  12:26 UTC), rendering not-yet-existent hours as "asleep" — both grids
  now clamp at the current day/hour. Regenerated + deployed
  (`riir-ops-dashboard` version `e9964b8d`), live-verified serving full
  UUIDs. The §"Corrected live finding" block above supersedes the
  2026-08-26 numbers (6 apps not 3; $0.3453 on 08-26 not $0.01–0.03/day;
  nothing running since 08-26T22Z). No worker/KV change — T3's Worker is
  untouched this session. T4/T6 unchanged.
