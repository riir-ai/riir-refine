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

### "Why not just use wrangler? It already authenticates."

It does — **on the M3**. `scripts/gen_dashboard.py:_cf_token()` prefers
`CF_API_TOKEN` but falls back to the wrangler OAuth file, so the generator and
`scripts/ops_nightly.sh` need **zero secrets**. That fallback is the whole
reason the nightly lives on the M3 rather than in CI.

This **Worker** is different: it runs on Cloudflare's edge, on a cron, with no
human present. It cannot read `~/Library/Preferences/.wrangler/config/default.toml`
— the only credentials it can see are `env` vars and secrets.

Pasting the OAuth token in fails for two independent reasons (both measured
2026-08-28, not assumed):

1. **It expires.** The config file carries an `expiration_time` ~1 h out plus a
   `refresh_token`; the CLI refreshes silently, a Worker cannot. It would break
   within the hour. A stale OAuth token is exactly what produced the
   `graphql 400` heartbeat row found in KV during the T8 audit.
2. **Blast radius — the stronger reason.** The OAuth token carries **29
   scopes**, including `workers:write`, `workers_kv:write`,
   `workers_scripts:write`, `secrets_store:write`, `containers:write`,
   `cloudchamber:write`, `connectivity:admin`, `d1:write`, `ai:write`,
   `email_sending:write`, `ssl_certs:write`. That is near-total account write
   authority. This job needs exactly one permission: **Account → Analytics :
   Read**. Storing the OAuth token here would let an unattended edge process
   rewrite your Workers, KV, secrets store and containers in order to read
   three analytics tables.

**The zero-secret alternative, and why it is not taken:** `ops_nightly.sh`
could collect on the M3 (wrangler already works there) and push rows in with
`wrangler kv key put`. That trades 24/7 unattended hourly collection — the
entire reason this Worker exists — for a laptop that must be awake, to save a
one-time 30-second step. Not worth it.

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

### Access model (hardened 2026-08-28 — Issue 045 §T7, revised §T8)

There are **two** gate levels, and the distinction is the whole design:

| level | condition | what it guards |
|---|---|---|
| fail-**open** | `READ_TOKEN` unset ⇒ pass | liveness + config posture, so the T4 verification commands work before any token exists |
| **hard** | `READ_TOKEN` set **and** presented | anything that is account telemetry, or that costs a KV `list` op |

| route | anonymous | with `READ_TOKEN` | cache |
|---|---|---|---|
| `/latest` | projection + `redacted:true` | full row | 60 s |
| `/status` | liveness (`worker`, `cron`, `ts`, `ok`) + `latest` **as the projection**; config posture while the gate fails open | + `alerts_today`, `latest` as the full row | 60 s |
| `/hours?n=` | **projection** rows, `n ≤ 168` *(401 once `READ_TOKEN` is set)* | full rows | 300 s + isolate memo |
| `/days?n=` | rows, `n ≤ 90` *(401 once `READ_TOKEN` is set)* | same | 300 s + isolate memo |

- **The projection is applied on every route, not just `/latest` (§T8).** It
  is `ts`, `hour`, `ok`, `mint`, aggregate `containers.est_usd`,
  `ai.neurons_today`, `ai.pct` — by construction everything the dashboard
  consumes, so hard-gating the per-app ids, per-app cost, mint anchor and
  collector notes costs no functionality. T7 applied it only in `/latest`,
  which meant `GET /status` and `GET /hours?n=1` each returned the full row
  to an anonymous caller: two silent bypasses of a redaction documented as
  unconditional. Harmless while the rows are heartbeats — it arms itself the
  moment T4 sets `CF_API_TOKEN` and the rows start carrying the breakdown.
- **`alerts_today` is hard-gated (§T8).** It issues a KV `list` per request
  and sat behind the fail-open gate, so anonymous traffic drove one list op
  each. The free tier allows **1,000 list ops/day against 100,000 reads**
  ([KV pricing](https://developers.cloudflare.com/kv/platform/pricing/)) —
  ~1,000 requests exhaust it, and the two things that then break are both
  list-based: the day-aggregate recompute and the monthly-budget check.
  Cheaper to trip and more damaging than the read amplification T7 fixed.
- `token_set` / `read_gated` stay readable while the gate fails open — that
  is the check that the `CF_API_TOKEN` step landed.
- **CORS is an allowlist**, not `*` — set `ALLOWED_ORIGINS` (`[vars]`,
  comma-separated, no trailing slash) to the origins allowed to read
  cross-origin. A non-allowlisted `Origin` gets no `Access-Control-Allow-Origin`.
- **Fan-out is bounded.** `n` clamps to 168/90 and reads run at concurrency
  16. Before this, `n=720` meant 720 unbounded concurrent KV gets on an
  unauthenticated route — 138 requests exhausted the 100k/day KV read tier,
  which also broke the alert-dedupe read and silenced the cap alarm.
- **Repeat fan-outs are memoised per isolate (§T8)**, 60 s TTL, 64 entries,
  public variant only — an authenticated body is never served from a cache an
  anonymous caller can hit. Deliberately *not* the Cache API: the
  [docs](https://developers.cloudflare.com/workers/runtime-apis/cache/)
  enumerate the contexts where it no-ops (dashboard editor, Playground,
  Access-fronted Workers) and `*.workers.dev` is not among them — but the
  availability sentence names custom domains and Pages and stays silent on
  `workers.dev`. That is an omission, not a grant, and this Worker is on
  `workers.dev`. The memo needs no such premise.
- Non-`GET`/`HEAD` → `405`.

## Tests

```sh
node --test cloudflare/housekeeping/test/*.test.mjs   # 12 tests, no deps
```

A bare directory arg fails `MODULE_NOT_FOUND` — `node --test` wants file
paths. The Worker is dependency-free ESM and every global it touches exists
in Node 18+, so it runs unmodified under `node:test` with a KV stub.

These exist because the §T8 key-collision bug was live for two days while
every "live-verified" check passed: the endpoints returned `200` with
plausible JSON throughout. Asserting on the **shape of the KV keys** is what
catches that class; asserting on HTTP status is not. Verified as a real pin —
8 of the 12 fail against the pre-fix worker (`git show HEAD~1:.../worker.js`).
`scripts/ops_nightly.sh` runs them WARN-only (it does not deploy this
Worker, so it has nothing to gate).

## Re-deploying the dashboard after changes here

The static dashboard reads CF GraphQL directly at generation time
(`scripts/gen_dashboard.py`); the Worker's KV is the running archive +
future live-mode source (`/hours`, `/days`).

## Pricing constants (pinned 2026-08-26)

`MEM_USD_PER_GIB_S=2.5e-6` · `DISK_USD_PER_GB_S=7e-8` ·
`CPU_USD_PER_S=2e-5` — Workers Paid, beyond the included 25 GiB-h mem /
375 vCPU-min / 200 GB-h disk per month. A change is a `[vars]` re-pin +
redeploy (the `CHARGE_VERSION` discipline from Proposal 005).
