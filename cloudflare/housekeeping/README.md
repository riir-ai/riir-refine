# riir-kat-housekeeping (Issue 045)

Hourly `$0` housekeeping Worker: **container cost tracking + KAT mint
anchor**. Deployed at `riir-kat-housekeeping.<account>.workers.dev`.

- **Cron** `0 * * * *` — pulls CF GraphQL (`containersUsageAdaptiveGroups`
  billing dataset + `containersMetricsAdaptiveGroups` awake seconds +
  `aiInferenceAdaptiveGroups` day neurons), computes est cost, writes KV
  rows (`hour:*`, `day:*`, `latest`), and records the **KAT mint anchor**.
- **Mint:** records the anchor (`neurons_today`, `kat: null`,
  `status: "pending-phase-1"`). The live mint (riir-chain `chain_prog_kat`
  + the riir-dapps ledger DO) is Proposal 005 Phase 1 — swap-in point is
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

## Deploy / read

```sh
npx wrangler deploy
curl https://riir-kat-housekeeping.<account>.workers.dev/status
curl '.../hours?n=24'   # hourly rows (containers est_usd + awake_s per app)
curl '.../days?n=14'    # day aggregates
```

## Re-deploying the dashboard after changes here

The static dashboard reads CF GraphQL directly at generation time
(`scripts/gen_dashboard.py`); the Worker's KV is the running archive +
future live-mode source (`/hours`, `/days`).

## Pricing constants (pinned 2026-08-26)

`MEM_USD_PER_GIB_S=2.5e-6` · `DISK_USD_PER_GB_S=7e-8` ·
`CPU_USD_PER_S=2e-5` — Workers Paid, beyond the included 25 GiB-h mem /
375 vCPU-min / 200 GB-h disk per month. A change is a `[vars]` re-pin +
redeploy (the `CHARGE_VERSION` discipline from Proposal 005).
