# Issue 045 — container cost observability + hourly housekeeping worker + smallest-sizing verdict

**Filed:** 2026-08-26
**Status:** OPEN — T1 verdict recorded, T2+T3 SHIPPED same day; live-mint half gated on Proposal 005 Phase 1
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

- **Cron:** `0 * * * *` (hourly). Free-tier compatible (1 trigger).
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
      housekeeping data collection goes live
- [ ] T5 name the 3 live container-family apps (correlate app ids via
      dashboard-side mapping or CF dashboard) — cosmetic, blocked on a
      name-bearing API surface
- [ ] T6 Phase 1 lands → swap `recordMintAnchor` for the real mint call
      (ledger DO / `chain_prog_kat`) — tracked by Proposal 005 T1.1–T1.4

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
