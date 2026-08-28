# riir-kat-housekeeping (Issue 045)

Hourly `$0` housekeeping Worker: **container cost tracking + KAT mint
anchor**. Deployed at `riir-kat-housekeeping.<account>.workers.dev`.

- **Cron** `0 * * * *` — pulls CF GraphQL (`containersUsageAdaptiveGroups`
  billing dataset + `containersMetricsAdaptiveGroups` awake seconds +
  `aiInferenceAdaptiveGroups` day neurons), computes est cost, writes KV
  rows (`hour:*`, `day:*`, `latest`), and records the **KAT mint anchor**.
- **Mint:** records the anchor (`neurons_today`, `kat: null`,
  `status: "pending-worker-shell"`). Proposal 005 Phase 1 (the sync
  surface) landed 2026-08-27 (T1.5/T1.6); the live mint awaits the
  deferred Worker/DO HTTP ledger shell (proposal-005 phase-2 defer /
  Plan 031 Phase B gate / phase-3 owner gate) — swap-in point is
  `row.mint_anchor` in `runHousekeeping()`; no cron/topology change.
- **Why a Worker, not a container:** the run is milliseconds of work.
  Worker cron is free-tier (3 triggers/worker), 24 KV writes/day ≪ the
  1k/day free cap. Any container pays the $5/mo Workers Paid gate +
  per-run uptime for nothing (Issue 045 §T1 verdict).

## One-time owner step

```sh
cd cloudflare/housekeeping
npx wrangler secret put CF_API_TOKEN   # dedicated token, Account → Analytics : Read
```

The wrangler OAuth token rotates ~hourly and cannot be a Worker secret.
Until `CF_API_TOKEN` is set the cron still writes heartbeat rows (proves
the schedule is alive).

## Optional secrets (budget alerting, Issue 044 T4)

```sh
npx wrangler secret put ALERT_WEBHOOK_URL     # Discord-compatible webhook URL
npx wrangler secret put MONTHLY_BUDGET_USD    # optional monthly container budget (USD)
```

- **Alert thresholds:** `50% / 90% / 100%` of `NEURON_CAP` (`[vars]`, default
  `10000` neurons/day). One webhook per threshold per day — deduped via KV
  key `alert:<date>:<threshold>`, written **after** the POST attempt
  regardless of success (a failed POST is **not** retried next hour — no
  retry storms; check the KV row's `notified` flag).
- **At 100%:** the message adds `CAP EXCEEDED — new generations hard-fail
  until 00:00 UTC`.
- **Monthly budget:** if `MONTHLY_BUDGET_USD` is set, the run sums the
  month's `day:*` rows and alerts once per month (`alert:<YYYYMM>:budget`)
  when month-to-date est cost reaches the budget.
- **`day:*` rows are recomputed, not accumulated** (fixed 2026-08-28): each
  run sums the authoritative `hour:<day>*` rows. The previous
  read-modify-write double-counted on a cron double-fire or manual invoke and
  could lose an update between overlapping runs.
- Alerting only runs when `CF_API_TOKEN` is set (neurons are only known
  there). Without a webhook, alerts still land in KV (`notified: false`).

## Optional secret (read gating, Issue 045 T7)

```sh
npx wrangler secret put READ_TOKEN            # gates /hours, /days, /status config half
```

## Deploy / read

```sh
npx wrangler deploy
curl https://riir-kat-housekeeping.<account>.workers.dev/status
curl -H "Authorization: Bearer $READ_TOKEN" '.../hours?n=24'   # hourly rows
curl -H "Authorization: Bearer $READ_TOKEN" '.../days?n=14'    # day aggregates
```

### Access model (hardened 2026-08-28 — Issue 045 §T7)

| route | anonymous | with `READ_TOKEN` | cache |
|---|---|---|---|
| `/latest` | minimised projection + `redacted:true` | full row | 60 s |
| `/status` | liveness only (`worker`, `cron`, `ts`, `ok`) | + `cap`, `thresholds`, `has_webhook`, `has_monthly_budget`, `read_gated`, `alerts_today`, `latest` | 60 s |
| `/hours?n=` | **401** *(fails open until `READ_TOKEN` set)* | rows, `n ≤ 168` | 300 s |
| `/days?n=` | **401** *(fails open until `READ_TOKEN` set)* | rows, `n ≤ 90` | 300 s |

- `/latest` redacts **unconditionally** — it does not fail open. The
  projection (`ts`, `hour`, `ok`, `mint`, aggregate `containers.est_usd`,
  `ai.neurons_today`, `ai.pct`) is by construction everything the dashboard
  consumes, so hard-gating the per-app ids and cost breakdown costs no
  functionality. The other routes fail open so `token_set` stays readable as
  the check that the `CF_API_TOKEN` step landed.
- **CORS is an allowlist**, not `*` — set `ALLOWED_ORIGINS` (`[vars]`,
  comma-separated, no trailing slash) to the origins allowed to read
  cross-origin. A non-allowlisted `Origin` gets no `Access-Control-Allow-Origin`.
- **Fan-out is bounded.** `n` clamps to 168/90 and reads run at concurrency
  16. Before this, `n=720` meant 720 unbounded concurrent KV gets on an
  unauthenticated route — 138 requests exhausted the 100k/day KV read tier,
  which also broke the alert-dedupe read and silenced the cap alarm.
- Non-`GET`/`HEAD` → `405`.

## Re-deploying the dashboard after changes here

The static dashboard reads CF GraphQL directly at generation time
(`scripts/gen_dashboard.py`); the Worker's KV is the running archive +
future live-mode source (`/hours`, `/days`).

## Pricing constants (pinned 2026-08-26)

`MEM_USD_PER_GIB_S=2.5e-6` · `DISK_USD_PER_GB_S=7e-8` ·
`CPU_USD_PER_S=2e-5` — Workers Paid, beyond the included 25 GiB-h mem /
375 vCPU-min / 200 GB-h disk per month. A change is a `[vars]` re-pin +
redeploy (the `CHARGE_VERSION` discipline from Proposal 005).
