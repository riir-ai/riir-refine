# Issue 045 — container cost observability + hourly housekeeping worker + smallest-sizing verdict

**Filed:** 2026-08-26
**Status:** OPEN — T1 verdict recorded, T2+T3 SHIPPED same day; live-mint half gated on Proposal 005 Phase 1. Sizing verdict feeds Proposal 005 T2.0/T2.1 **+ T2.6 / [Plan 031](../.plans/031_kat_sweeper_temp_container_r2_tier.md)** (2026-08-26 later-session sync). **2026-08-27: the T3 Worker was EXTENDED + redeployed (`ab79bd20`) — CORS on all read endpoints (dashboard live-fetch), Issue-044-T4 budget alerting (cap thresholds 50/90/100% + optional monthly budget, KV-dedupe, webhook via `ALERT_WEBHOOK_URL`), extended `/status` (`cap`/`thresholds`/`has_webhook`/`has_monthly_budget`/`alerts_today`).** **2026-08-28 truth-refresh: Proposal 005 Phase 1 LANDED 2026-08-27 (T1.5 dapps `84dd049` / T1.6 clippy `0a7cfd8`) — T6's blocker is NO LONGER Phase 1; the live-mint swap-in awaits the deferred Worker/DO HTTP ledger shell (Proposal 005 phase-2 defer / Plan 031 Phase B gate / phase-3 owner gate). Worker mint status strings refreshed `pending-phase-1` → `pending-worker-shell` + redeployed (version `f93ae507`, live-verified on the 04:00 UTC tick). T4 unchanged (owner-gated secrets). T5 still blocked — CLI exposes no id→name mapping (see checkbox).**
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

`containersUsageAdaptiveGroups` shows **3 container-family apps running
TODAY** (a03038cf…, a034b17e…, a035c2cc… — app ids, not names; the GraphQL
schema exposes no name mapping and `wrangler containers list` shows none).
Derived shapes: one ≈ standard-1, one ≈ standard-1, one ≈ lite; combined
burn ≈ **$0.01–0.03/day**. 2026-08-14 rows match the Bench 331 deploy pair
(deleted). Likely Cloudflare-Agents/Sandboxes-family workloads from other
tooling on this account. The dashboard (T2) makes them visible; naming them
needs dashboard-side correlation — left as a follow-up, not a blocker.

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
- [ ] T5 name the 3 live container-family apps (a03038cf…, a034b17e…,
      a035c2cc… — app ids, not names) — cosmetic, blocked on a
      name-bearing API surface. **2026-08-28 re-probe (read-only, from
      `cloudflare/housekeeping`, OAuth'd): `npx wrangler containers list`
      → `No containers found.`; `npx wrangler containers info a03038cf`
      → `[ERROR] Response body when an Application is not found` (exit 1)
      — the CLI neither lists the 3 GraphQL app ids nor maps id→name;
      needs dashboard-side correlation or a CF-dashboard lookup.**
- [ ] T6 live-mint swap-in — Proposal 005 Phase 1 (the sync surface)
      landed 2026-08-27 (T1.5 dapps `84dd049` / T1.6 clippy `0a7cfd8`),
      so the blocker is NO LONGER Phase 1: the swap awaits the deferred
      Worker/DO HTTP ledger shell (Proposal 005 phase-2 defer / Plan 031
      Phase B gate / phase-3 owner gate). Then swap `recordMintAnchor`
      (this object is already the designed mint payload) for the real
      mint call (`ledger DO` / `chain_prog_kat`). Worker status strings
      refreshed to `pending-worker-shell` 2026-08-28 + deployed
      (`f93ae507`).

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
