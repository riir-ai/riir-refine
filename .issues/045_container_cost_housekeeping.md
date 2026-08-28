# Issue 045 — container cost observability + hourly housekeeping worker + smallest-sizing verdict

**Filed:** 2026-08-26
**Status:** OPEN — T4 (owner secrets) + T6 (external ledger shell) remain, and **T8 + T9 are code-complete, dry-run-VALIDATED, but still NOT DEPLOYED (owner-gated; one command: `cd cloudflare/housekeeping && npx wrangler deploy`)**. **2026-08-29 T9 + T9b + T9c + T9d + T9e SHIPPED (code+tests, undeployed). T9e (perf/sec pass): `/hours` + `/days` were the last fail-open FAN-OUT routes (168 KV gets per request, memo is per-ISOLATE so colo-spread traffic misses it; ~595 anonymous requests exhaust the 100k/day read tier and what breaks is the day aggregate + the alert-dedupe get, i.e. anonymous traffic could silently disable budget alerting). Hard-gated to 403, which costs ZERO functionality because the consumer was checked first: the ops dashboard makes exactly one cross-origin call, `/latest`, and charts from a statically baked blob. RUNBOOK ORDER CHANGED — `secret put READ_TOKEN` now precedes the verification curls. Plus: the "timing-safe" compare leaked the secret's LENGTH (looped max(len) times) → SHA-256 digests, fixed 64-char loop, digest doubles as the memo salt so the authenticated fan-out is memoised for the first time; `/status` + `/latest` no longer 500 on a corrupt KV value (safeJson) and `fetch` gained the top-level catch `scheduled` got in T9c; `latest` reads collapse 10→1. 36/36 tests, 5 of 6 new/changed RED against the prior worker (the 2 green-on-both recorded as non-discriminating, not as evidence). T9d (KV inventory): the namespace is 8 keys — 7 legacy collided hour rows + `latest`, and ZERO `day:` rows, because the heartbeat path returned before the aggregate. `/days` has served `[]` since deploy and the aggregate code never ran in prod. Fixed, plus `hours` split from `hours_present` so a heartbeat day cannot report a false zero. DEPLOY WARNING: `/hours` returns `[]` right after the deploy (the fixed read path looks for keys that do not exist yet) — run the backfill immediately after. T9c (from reading PROD, not code): `/status` says `token_set:false` — the token has never been set, so no cost data has EVER been collected and the "~6-8x undercount is live" framing above describes an undercount of nothing; what T8 loses is heartbeats. That also broke T9's own design: its exists-based protect rule would have declared the all-heartbeat archive untouchable and repaired nothing, so the rule is now data-based (`ok===true && containers`, decided free from `list()` metadata). Plus `[observability]` (this worker's every failure mode is silence) and a `scheduled()` catch. T9b: T9b: authenticated read responses were `Cache-Control: public` — RFC 9111 §3.5 makes that the exact directive granting shared caches storage of an `Authorization`-bearing response that they would otherwise refuse; now `private, no-store`. Plus the monthly-budget scan re-ran hourly after its alert had already fired (~32 subrequests/run to a guaranteed no-op), and the backfill could 524 instead of returning its resume cursor (45 s wall-clock guard). 22/22 tests. T9: `POST /backfill` re-derives the hours T8 lost — the "unrecoverable" verdict was wrong, CF GraphQL retains ~31 d and exposes `datetimeHour`, so KV is a cache of a still-live source. 19/19 tests (7 new). En route it fixed a defect in T8's own test suite (the GraphQL stub omitted `datetimeHour`, so no test could have caught an hour-grouping bug) and a hot-path KV read-your-writes hole in the day aggregate.** Prior status: (2026-08-28: `hourKey()` was collapsing 24 hourly rows into 3, understating the daily cost ~8×; fixed + pinned by 12 regression tests, deploy blocked by the session permission classifier — run `cd cloudflare/housekeeping && npx wrangler deploy`); T5 RESOLVED (cross-repo correlation) + T7 read-path hardening SHIPPED 2026-08-28. Issue 053 (unattributed app `a03a70ff`) filed **and RESOLVED same day** by cross-repo correlation — it is the `riir-mmorpg-examples` CCU proof bench (Bench 034); resolved-and-removed per the noise rule, record in §T5 + git history. T1 verdict recorded, T2+T3 SHIPPED same day; live-mint half gated on Proposal 005 Phase 1. Sizing verdict feeds Proposal 005 T2.0/T2.1 **+ T2.6 / [Plan 031](../.plans/031_kat_sweeper_temp_container_r2_tier.md)** (2026-08-26 later-session sync). **2026-08-27: the T3 Worker was EXTENDED + redeployed (`ab79bd20`) — CORS on all read endpoints (dashboard live-fetch), Issue-044-T4 budget alerting (cap thresholds 50/90/100% + optional monthly budget, KV-dedupe, webhook via `ALERT_WEBHOOK_URL`), extended `/status` (`cap`/`thresholds`/`has_webhook`/`has_monthly_budget`/`alerts_today`).** **2026-08-28 truth-refresh: Proposal 005 Phase 1 LANDED 2026-08-27 (T1.5 dapps `84dd049` / T1.6 clippy `0a7cfd8`) — T6's blocker is NO LONGER Phase 1; the live-mint swap-in awaits the deferred Worker/DO HTTP ledger shell (Proposal 005 phase-2 defer / Plan 031 Phase B gate / phase-3 owner gate). Worker mint status strings refreshed `pending-phase-1` → `pending-worker-shell` + redeployed (version `f93ae507`, live-verified on the 04:00 UTC tick). T4 unchanged (owner-gated secrets). T5 still blocked — CLI exposes no id→name mapping (see checkbox).** *(That last clause is SUPERSEDED later the same day — T5 was resolved by cross-repo correlation, not by the CLI; see §T5.)*
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
- [ ] T8 hourly-archive correctness + posture pass (§T8 below) — `hourKey()`
      collapsed 24 h into 3 keys (day cost understated ~8×), the `/latest`
      redaction had two bypasses (`/status`, `/hours`), and anonymous
      `/status` spent a KV `list` per request against a 1,000/day cap. Fixed
      + pinned by 12 `node:test` regression tests (8 fail pre-fix).
      **CODE + TESTS LANDED; DEPLOY PENDING — OWNER-GATED.** One command
      closes it: `cd cloudflare/housekeeping && npx wrangler deploy`.

- [ ] T9d the KV inventory (2026-08-29) — **the whole namespace is 8 keys**:
      7 legacy collided `hour:YYYYMMDDT<digit>` rows + `latest`, and **zero
      `day:` rows** after three days of hourly runs.
      **Finding — the day aggregate has NEVER executed in production.** The
      heartbeat path `return`ed before it, so `/days` has served `[]` since
      deploy (indistinguishable from "nothing ran", the same silent-blank
      class as T8/T9c) and every fix to that code — T7 CORRECT-1's idempotent
      recompute, T9's `recomputeDay` extraction — was only ever exercised
      against a stub. Fixed: the heartbeat path falls through to the
      aggregate, and `hours` (rows that COLLECTED) is now split from
      `hours_present` (rows that exist). Conflating them would have reported a
      heartbeat day as "24 hours cost $0" — a false zero, which is worse than
      an absent row because it looks like a measurement.
      **Operational warning for the deploy (the thing most likely to look like
      breakage):** the deployed worker uses the buggy `hourKey()` on BOTH the
      write and the read path, so its reads currently land on the same
      collided keys — hence 24 rows / 4 distinct pairs. The fixed worker
      constructs `hour:YYYYMMDDHH`, none of which exist, so **`/hours` returns
      `[]` immediately after deploy** and fills one row per hour thereafter.
      Run the backfill right after to populate it. The 7 legacy keys become
      permanent orphans — unreachable by every read path, filtered from the
      aggregate by `HOUR_KEY_RE`, inert. Leave them; deleting is a destructive
      op on the only record of those heartbeats for no gain. Pinned by a test
      built from the exact live key shapes.
      31/31 tests (3 new, all 3 RED against the pre-T9d worker);
      bundle 23.73 KiB / 7.15 KiB gzip.
- [ ] T9c the live-state read that broke T9's own design (2026-08-29) —
      **found by curling prod, not by reading code.** `GET /status` reports
      `token_set: false`: CF_API_TOKEN has NEVER been set, so the worker has
      been in heartbeat mode since deploy and **no cost data has ever been
      collected**. Two consequences, one of them a bug in T9:
      **(a) The issue's urgency framing was wrong.** "The daily cost is
      understated ~6-8x and the longer the deploy waits the worse it gets"
      describes an undercount of data that does not exist. The T8 collision is
      real and reproduces exactly (live `/hours?n=24` → 24 rows, **4** distinct
      `(ts,hour)` pairs, re-measured 2026-08-28T17:16Z) but what it is losing
      is HEARTBEATS. The real stake is different and still urgent: the moment
      T4 sets the token, collection begins — and it must begin on a fixed
      worker, or it corrupts from hour one. Cron itself is healthy (last run
      16 min before the probe).
      **(b) T9's protect rule was exists-based, which would have repaired
      NOTHING.** "Never overwrite an existing hour row" was written to protect
      `ai` + `mint_anchor`. But every archived row is a heartbeat, so an
      exists-based rule declares the entire archive protected — and every
      placeholder written between the deploy and T4 would wedge its hour shut
      permanently. Fixed: the discriminator is now `hasCollectedData(row)` =
      `row.ok === true && row.containers`. `ok` is load-bearing — an
      all-collectors-failed row DOES get `containers:{est_usd:0,apps:{}}`
      because the collectors return empty maps rather than throwing, and that
      false zero must not read as data. Merge-forward preserves anything the
      placeholder did carry (e.g. a neuron count whose collector succeeded).
      **Decided free, not cheap:** protect/replace is read from `list()`
      metadata (the KV docs recommend this over `list()`+`get()`), via a
      two-bit marker `d` (real collection) + `m` (worth merging). Both bits
      are needed — without `m`, skipping the `get` also discards the body the
      merge requires. Pure heartbeats, i.e. 100% of today's archive, cost zero
      gets. Pre-marker rows are classified by a `get`; assuming either way is
      wrong in a different direction.
      **Also in this pass:** `[observability] enabled = true` added to
      `wrangler.toml` — every failure mode of this worker is silence and the
      docs are explicit that Workers Logs requires the setting; and
      `scheduled()` now catches, because `runHousekeeping`'s internal `try`
      covers collection while the KV writes after it do not, so a KV outage
      rejected into `waitUntil()` leaving no row and no log. Doc drift fixed:
      `wrangler.toml` claimed a `mint_anchor:<date>` KV key that is never
      written (the anchor lives inside the hour row) — the T6 swap-in would
      have gone looking for it.
      28/28 tests (6 new, 5 confirmed RED against the pre-T9c worker);
      bundle 23.65 KiB / 7.11 KiB gzip.
- [ ] T9b prod-grade pass on the same pending deploy (2026-08-29) — three
      findings, each pinned by a test confirmed RED against the pre-T9b
      worker:
      **SEC-1 — authenticated bodies were marked publicly cacheable.**
      `cors()` stamped `Cache-Control: public, max-age=60/300` on every read
      route including the token-bearing full rows (per-app ids, per-app cost,
      mint anchor). Per RFC 9111 §3.5 a shared cache MUST NOT reuse a response
      to an `Authorization`-bearing request *unless* it carries
      `public`/`s-maxage`/`must-revalidate` — so `public` was the exact
      directive GRANTING storage the default would have refused.
      `Vary: Authorization` was present and does keep a conformant cache from
      cross-serving, but it is a mitigation on an unnecessary grant. Notably
      the isolate memo already encoded the correct rule ("an authenticated
      body must never be served from a cache an unauthenticated caller can
      also hit") and the HTTP header contradicted it one layer down — the
      same class T7/T8 kept finding. Authenticated responses are now
      `private, no-store`; the redacted anonymous projection stays cacheable.
      **PERF-1 — the monthly-budget scan ran forever after firing.** The
      month-to-date scan (1 KV `list` + up to 31 `get`s) sat ABOVE its dedupe
      check, so once the alert had fired it re-ran every hour for the rest of
      the month to recompute a number that could only re-fire an alert already
      sent — ~32 subrequests/run to reach a guaranteed no-op. Dedupe now gates
      the scan. (Workers count every KV binding call as a subrequest.)
      **RESIL-1 — the backfill could 524 instead of yielding its cursor.** 14
      days is 28 sequential GraphQL round-trips, which can outlive the edge's
      client-connection timeout. The timeout is SAFE (writes committed, route
      idempotent) but indistinguishable from a broken worker, and it loses the
      resume cursor. A 45 s wall-clock guard now stops on a day boundary and
      returns `truncated_by:"wall_clock"` + `next_end`.
      Test-instrument note: the wall-clock pin needed a new
      `atUtcAdvancing()` helper — the existing fixed-instant clock makes any
      elapsed-time guard measure zero and pass vacuously.
      22/22 green; bundle re-verified (22.13 KiB / 6.68 KiB gzip, 7 bindings).
- [ ] T9 backfill the hours T8 lost — **CODE + TESTS LANDED 2026-08-29;
      rides the same owner-gated deploy as T8.** `POST /backfill?days=&end=`
      re-derives missing `hour:YYYYMMDDHH` rows from the GraphQL billing
      dataset (~31 d retention) and recomputes the affected day aggregates.
      Never overwrites an existing row (a cron row carries `ai` +
      `mint_anchor`, which billing cannot rebuild); idempotent (one KV `list`
      per day decides existence, so a repeat run issues zero puts AND zero
      gets); bounded (`days <= 14`, 700-subrequest budget, stops on a day
      boundary and returns a `next_end` cursor — Workers count every KV
      binding call as a subrequest); `POST`-only and hard-gated (an unset
      `READ_TOKEN` is a 403, not a fail-open). 7 new tests, 19/19 green,
      bundle dry-run-verified (21.72 KiB / 6.53 KiB gzip, 7 bindings resolve).
      **Owner runbook, after the T8 deploy and after T4 sets `CF_API_TOKEN`:**
      `curl -sX POST -H "Authorization: Bearer $READ_TOKEN" "$H/backfill?days=14" | jq`
      then repeat with `?end=<next_end>` while `truncated:true`.
      En route, T9 found and fixed a **defect in the T8 test suite itself**:
      the GraphQL stub omitted `datetimeHour`, the dimension the query asks
      for, so a whole-day window was indistinguishable from a one-hour window
      and **no test in the suite could have caught an hour-grouping bug** —
      precisely the class the suite exists to pin. Also fixed on the hot path:
      the cron listed `hour:<dk>*` immediately after `put`-ing that hour's
      row, and KV `list` is not read-your-writes, so the day total could
      silently omit the very hour it had just collected. `recomputeDay(env,
      dk, seed)` — now the single shared definition of a day aggregate, used
      by both the cron and the backfill — unions the in-memory seed with the
      listed names and closes that window.

      **2026-08-28 validation pass (owner chose dry-run over a live deploy).**
      The bundle was validated WITHOUT changing live behaviour, and the defect
      was re-measured on the running Worker:
      - `wrangler deploy --dry-run --outdir` builds clean (15.83 KiB / 5.01
        KiB gzip); all 7 bindings resolve (`HOUSEKEEPING_KV`, `CF_ACCOUNT_ID`,
        the 3 price vars, `NEURON_CAP`, `ALLOWED_ORIGINS`).
      - The emitted bundle CARRIES the fix — `hourKey` = `u.slice(0,8) +
        u.slice(9,11)` (10-digit `YYYYMMDDHH`) plus the `HOUR_KEY_RE
        /^hour:\d{10}$/` legacy-collision filter. Verifying the artifact that
        WOULD deploy, not just the source it came from.
      - Regression suite re-run: **12/12 pass** (`node --test
        test/worker.test.mjs`). *(`node --test test/` — the directory form —
        reports a bogus 1-test failure; invoke the FILE.)*
      - Live deployment list: production is still `ca93c8de`, the **T7**
        deploy. The T8 commit `216257a` is NOT live.
      - **Live defect re-measured:** `/hours?n=24` returns 24 rows carrying
        only **4 distinct** `(ts, hour)` pairs — `2026-08-27T18/22`,
        `2026-08-28T08/15`. The day aggregate is summing **4 of 24 hours**.
      - **The understatement factor is NOT a constant** — 3 distinct keys
        when first measured, 4 now (~8x then, ~6x today). It tracks how many
        collision buckets happen to be populated, so the archived rows cannot
        be repaired by scaling a fixed multiplier.
      - ~~The longer the deploy waits, the more unrecoverable hours accrue.~~
        **CORRECTED 2026-08-29 by T9.** "Unrecoverable" was true of the KV
        *rows* and false of the *data*. KV here is a cache of a source that is
        still live: CF's GraphQL analytics datasets retain **~31 days**, and
        `datetimeHour` — the dimension the collectors were already requesting
        and then summing away — is the per-hour truth the archive lost. The
        hours ARE re-derivable, and `POST /backfill` (§T9) re-derives them.
        The deploy urgency is unchanged (rows past ~31 d really are gone, and
        the live undercount continues meanwhile) but the loss window is now
        bounded by GraphQL retention, not by how long the deploy waits.

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


---

## T8 — the hourly archive was collapsing 24 hours into 3 keys (2026-08-28)

**Status: code + tests on `develop`; NOT DEPLOYED.** `npx wrangler deploy` was
declined by this session's permission classifier. Production is still running
the pre-fix worker (`ca93c8de`) and still collapsing its hour keys until an
owner runs `cd cloudflare/housekeeping && npx wrangler deploy`.

A prod-grade perf/sec pass over the T3 Worker's own state, prompted by the
sibling Issue 044 T13 finding ("the fail-open rationale was false"): the same
question, asked of this Worker's fail-open routes, found one correctness bug
and two more posture defects. All are fixed, tested and pinned.

### BUG-1 (headline) — `hourKey()` produced 3 distinct keys per day, not 24

```js
function utc(d)     { return d.toISOString().replace(/[-:]/g,"").replace(/\.\d{3}Z$/,""); }
function hourKey(d) { return utc(d).slice(0, 10); }   // "YYYYMMDDHH" — it is not
```

`utc()` yields `YYYYMMDD"T"HHMMSS`. `slice(0, 10)` therefore keeps the literal
`T` **plus one hour digit**:

| wall hour | key written | |
|---|---|---|
| 00–09 | `hour:20260828T0` | 10 runs, one surviving row |
| 10–19 | `hour:20260828T1` | 10 runs, one surviving row |
| 20–23 | `hour:20260828T2` | 4 runs, one surviving row |

**Measured in production before the fix**, not inferred:

- `wrangler kv key list` returned **3 keys total** for the namespace:
  `day:20260828`, `hour:20260828T1`, `latest`.
- `day:20260828` = `{"date":"2026-08-28","est_usd":0,"hours":1,...}` — after
  **13 cron runs that day**. The day aggregate sums `hour:<YYYYMMDD>*`, so it
  was summing ≤3 of 24 hours: **the daily cost number this issue exists to
  produce was understated ~8×.**
- `GET /hours?n=24` returned **24 rows with 4 distinct timestamps** — the same
  row repeated, rendering as a flat hourly chart that looked like real data.

The bug was live from T3's deploy (2026-08-26) and survived three rounds of
"live-verified" checks, because **every endpoint returned `200` with plausible
JSON the entire time**. Nothing in an HTTP-status check can see this.

**Blast radius, scoped honestly.** The *dashboard's* container charts are NOT
affected: `scripts/gen_dashboard.py` queries CF GraphQL directly at generation
time and never reads the Worker's KV (verified — the only network call the
dashboard makes is `GET /latest`, `gen_dashboard.py:1535`, and `/latest` reads
the single `latest` key, which no collision can corrupt). What the bug
destroyed is the **Worker's own KV archive**: the `hour:` rows, the `day:`
aggregates, `/hours`, `/days`, and — the part that would have mattered most —
the future live mode and the mint-anchor history that Proposal 005's economics
loop is meant to read. CF GraphQL retains ~14 days; this archive is supposed to
be the durable record past that horizon, and for its first two days it kept
3 rows a day instead of 24.

Two related off-by-ones fixed in the same pass:

- **Rows were filed under the run time, not the hour they describe.** The
  12:00 run collects 11:00–12:00 and wrote it under `12`, so every point was
  one slot off its own `row.hour` field. Now keyed on `hourStart`.
- **The 00:00 run orphaned the previous day's last hour.** It collects the
  23:00 hour of day *N* but aggregated `dayKey(now)` = day *N+1*, so day *N*
  never got its 24th hour. Now aggregates `dayKey(hourStart)`.

Migration: legacy `hour:YYYYMMDDT<digit>` keys still match the aggregate's
`hour:YYYYMMDD` prefix scan, so they would have been summed *alongside* the
correct rows — a double count layered on the undercount. The aggregate now
filters to `/^hour:\d{10}$/`. The pre-fix hours of 2026-08-28 are
unrecoverable (they were overwritten in place); collection is correct from
the first post-deploy tick.

### SEC-1 — the `/latest` redaction had two silent bypasses

T7 documented the minimised projection as **unconditional** on `/latest` and
applied it in exactly one place. Both other read routes returned the **full
row** to an anonymous caller:

- `GET /status` echoed `latest: parsed` — the complete row.
- `GET /hours?n=1` returned full rows (the route fails open until `READ_TOKEN`
  is set, which it is not — T4 is still open).

Currently harmless only because the rows are heartbeats. It **arms itself the
moment T4 lands**: from that tick the rows carry `containers.apps` (per-app
id → awake seconds → est cost), the mint anchor and the collector notes — the
exact account telemetry `/latest` refuses to serve. Fixed by hoisting the
projection into one `publicRow()` used by all three routes.

### PERF-1 — an anonymous KV `list` per `/status` request

`alerts_today` issues a `kv.list()` on **every** request and sat behind the
fail-open gate, so anonymous traffic drove one list op each. Free-tier KV
allows **1,000 list ops/day against 100,000 reads**
([pricing](https://developers.cloudflare.com/kv/platform/pricing/), confirmed
against the live doc — the more commonly cited *limits* page omits list and
delete entirely). So:

- **~1,000 unauthenticated requests exhaust the day's list quota**, versus the
  138 that exhausted the read tier in T7 — but list ops are **100× scarcer**,
  and the two operations that then fail are both list-based: the day-aggregate
  recompute and the monthly-budget check. A cheaper trip with a worse blast
  radius than the bug T7 shipped to fix.

Fixed: `alerts_today` (and the full-row `latest`) now require a **presented**
token. Liveness and `token_set`/`read_gated` stay in the fail-open tier so the
T4 verification commands still work before any token exists.

### PERF-2 — repeat fan-outs are memoised per isolate

`/hours?n=168` issues 168 KV gets, uncached, per request. Added a 60 s
isolate-local memo (64 entries, **public variant only** — an authenticated
body must never be served from a cache an anonymous caller can hit). Past hour
rows are immutable once written, so the TTL is safe.

Deliberately **not** the Cache API. Checked the docs rather than assuming:
[runtime-apis/cache](https://developers.cloudflare.com/workers/runtime-apis/cache/)
enumerates the three contexts where Cache API ops no-op (dashboard editor,
Playground previews, Access-fronted Workers) and `*.workers.dev` is not among
them — **but** the availability sentence names custom domains and Pages and
stays silent on `workers.dev`. That is an omission, not a grant, and this
Worker is on `workers.dev`. An isolate memo needs no such premise. *(This
correction matters: the first draft of the fix asserted a `workers.dev` Cache
API prohibition as fact in a source comment. It was not fact — same failure
mode as the false fail-open rationale that started this pass.)*

### The regression pin (`cloudflare/housekeeping/test/worker.test.mjs`)

12 tests, zero dependencies, `node --test cloudflare/housekeeping/test/*.test.mjs`.
The Worker is dependency-free ESM and every global it touches exists in
Node 18+, so it runs unmodified under `node:test` against a KV stub and a
pinned clock.

**Verified as a real pin, not a decorative one** — run against the pre-fix
worker (`git show HEAD:...`), **8 of 12 fail**, each with the measured number:

| test | pre-fix | post-fix |
|---|---|---|
| every hourly run writes a distinct key | 3 rows | 24 rows |
| the key names the hour the row describes | `hour:20260828T1` | `hour:2026082811` |
| the day aggregate counts every hour | `hours: 2` | `hours: 12` |
| the 00:00 run aggregates the previous day | orphaned | filed under the 28th |
| anonymous `/hours` is redacted | full rows | `redacted: true` |
| anonymous `/status` echoes the projection | full row | projection |
| anonymous `/status` spends zero list ops | 10 ops / 10 reqs | 0 |
| `/hours` repeat is memoised | 168 gets | 0 gets |

The 4 that pass on both are the invariants the fix must **not** break
(authenticated detail, wrong-token rejection, CORS allowlist).
`scripts/ops_nightly.sh` runs the suite WARN-only — it does not deploy this
Worker, so it has nothing to gate; it exists so the next regression is caught
by a machine rather than by an audit two days later.

## T9e — the fail-open window on the fan-out routes, and a timing-leaky compare (2026-08-28)

Two prod-grade findings from a perf/sec pass over the (still undeployed) T9d
worker. Both were reachable only by first establishing a fact about the
CONSUMER, not by reading the worker.

### The fact that decided it

`/hours` and `/days` have **no consumer**. The ops dashboard makes exactly one
cross-origin call — `/latest` — and renders its hourly/daily container charts
from the statically baked `<script id="ds">` blob that
`scripts/gen_dashboard.py` writes into `public/index.html`. Every `/hours` /
`/days` invocation in the housekeeping README already passes
`Authorization: Bearer $READ_TOKEN`. The dashboard worker's own CSP comment
says the same thing from the other side ("makes exactly ONE cross-origin call:
the housekeeping worker's /latest live badge").

That is what made the fix free. Without checking it, the obvious move was the
weaker one (cap the anonymous `n`), because hard-gating *looks* like it costs
the weekly chart.

### SEC/PERF-1 — `/hours` + `/days` were the last fail-open fan-out

They are the only fan-out routes on the worker: one request issues up to **168
KV gets**. The isolate memo meant to blunt that is **per-isolate**, and
Workers spreads traffic across colos, so realistic anonymous traffic misses it
nearly every time. ~**595 anonymous requests exhaust the 100k/day KV read
tier** — and what breaks when reads run out is not the chart, it is the
day-aggregate recompute and the alert-dedupe `get`. So unauthenticated traffic
could **silently disable budget alerting**: the same exhaustion class T7
capped, one route over, and only partially mitigated.

Redaction (the T8 answer for this route) does not help — producing a redacted
row costs the identical fan-out. So the routes now **403 while `READ_TOKEN` is
unset**, matching `/backfill`'s existing rule verbatim ("a costly route does
not fail open"). Zero functionality lost per the consumer fact above; the
diagnostic path the fail-open posture existed for (`token_set`, `read_gated`,
liveness) is on `/status` + `/latest`, untouched.

**Operational consequence for the runbook:** `wrangler secret put READ_TOKEN`
now moves **ahead of** the post-deploy verification curls. Not an extra step —
the backfill already required it — but the ordering changed, and an anonymous
`curl .../hours` returning 403 is the gate working, not the deploy failing.

### SEC-2 — the "timing-safe" compare leaked the secret's length

`timingSafeEqual` XOR-ed the raw encodings and looped
`max(len(guess), len(secret))` times. Folding the length difference into
`diff` makes the **verdict** length-independent — but not the **timing**,
which is the property it was named for: a 1-char guess still ran
`len(READ_TOKEN)` iterations. Both sides are now SHA-256'd first, so every
comparison is a fixed 64-hex-char loop.

The accepted digest doubles as the memo salt. That is not incidental: the old
memo deliberately skipped the authenticated path, so hard-gating the route
would otherwise have left **nothing** memoised and made every owner poll pay
the full 168-get fan-out. Salting with a digest of an already-ACCEPTED token
restores the collapse without weakening the rule it was protecting — a
rejected caller cannot name a live key.

**Honest limit:** the tests pin the compare's **verdicts** across secret
lengths 1 / 6 / 200, not its **timing**. A wall-clock timing assertion would
be flaky. The timing property rests on the loop being fixed-width by
construction — a code-shape argument, not a measured one.

### ROBUST-3 — the two public routes 500'd on a corrupt KV value

Every `JSON.parse` here is over a value KV returned, i.e. over data a previous
deploy or partial write could have left malformed. The aggregate and backfill
paths were already tolerant; `/status` and `/latest` parsed bare, so one bad
`latest` value turned the dashboard's liveness badge into an opaque runtime
500. Now `safeJson` → reports as absent. `fetch` also gained the top-level
catch `scheduled` got in T9c, for the same reason (this worker's every failure
mode is silence): a throw now logs one line of ours and answers
`{"error":"internal error"}` as `private, no-store`.

### PERF-4 — `latest` is the one key anonymous traffic can drive

`/status` and `/latest` both read it, one get per request each. Now one
memoised **parsed** read shared by both (60 s TTL, matching the `max-age=60`
both already advertise): 10 public reads cost 1 KV get, down from 10. The row
is projected/gated per-request, so the memo is a KV-read cache, not a response
cache — no auth coupling.

Memoising module state also exposed a **test defect I introduced**: the memo's
keys carry no namespace identity, so one test's `latest` was servable to the
next test's differently-seeded stub — a suite that passes *in this order*.
Hence `__resetMemoForTests()` + a `beforeEach`. The Workers runtime only ever
reads `default`, so the extra named export is inert in production.

### The false green worth recording

Hard-gating `/hours` made the existing test *"/hours fan-out is bounded and
memoised per isolate"* **pass for the wrong reason**: a 403 issues zero KV
gets, so both "bounded" and "memoised from the memo" held trivially and the
assertions measured nothing. It now presents a token and asserts
`first > 0` before asserting `first <= 168` — the shape of every gate that
can be satisfied by the measured path not running.

### Gate

**36/36 tests** (4 new, 2 rewritten). Verified **5 of the 6 new/changed tests
are RED against the prior worker**. The 2 that are green on both are recorded
as non-discriminating rather than as evidence: the wrong-token-vs-memo test
(the old worker also 401s, having no authenticated memo to leak) and the
token-length test (the old compare returned the same *verdicts*; only its
timing leaked). Bundle 25.46 KiB / 7.78 KiB gzip, up from 23.73 / 7.15.
Undeployed — `npx wrangler deploy` is still classifier-blocked.
