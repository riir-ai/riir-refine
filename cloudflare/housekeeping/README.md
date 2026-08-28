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

### 1. Create the token

Go to the **Account API tokens** page — <https://dash.cloudflare.com/?to=/:account/api-tokens>
— and **Create Token → Custom token**.

Use the **account-owned** surface, not `My Profile → API Tokens`. Account-owned
tokens are documented as
"[durable integrations that can act as service principals with their own
specific set of permissions](https://developers.cloudflare.com/fundamentals/api/get-started/account-owned-tokens/)",
whereas a user token "acts on behalf of a particular user and inherits a subset
of that user's permissions" — i.e. it dies with the user. An unattended cron is
exactly the durable-service-principal case. *(Creating one requires Super
Administrator on the account.)*

| field | value |
|---|---|
| Token name | `riir-kat-housekeeping-analytics` |
| Permissions | `Account` → **`Account Analytics`** → **`Read`** |
| Account Resources | Include → this account (`CF_ACCOUNT_ID` in `wrangler.toml`) |
| TTL | optional start/end **date pair**; there is no default expiry |
| Client IP filtering | optional; a Worker has no stable egress IP, so leave it off |

**One permission. Nothing else.** Do **not** add `Containers : Read` or
`Workers AI : Read` — those govern the Containers / Workers-AI *management*
REST APIs, not the analytics pipeline, and they widen the blast radius of a
token whose whole justification (see below) is that it is narrow.

Two dashboard gotchas, both doc-sourced:

- The
  [Analytics walkthrough](https://developers.cloudflare.com/analytics/graphql-api/getting-started/authentication/api-token-auth/)
  tells you to configure **Zone Resources**. `Account Analytics` is an
  **account**-scoped permission and binds via **Account Resources**; the zone
  step reads as stale copy carried over from zone-analytics tokens. The docs
  do not explain the discrepancy.
- The token is **shown once**. "Anyone who has the token can use it to access
  your data."

### 2. Verify it BEFORE setting it — not optional here

```sh
node cloudflare/housekeeping/verify_token.mjs <token>     # or CF_API_TOKEN=... node ...
```

This is a hard requirement rather than a nicety, because **Cloudflare does not
document the permission these queries need**:

- `aiInferenceAdaptiveGroups` is **undocumented on developers.cloudflare.com** —
  not the dataset, not its fields, not its permission. It exists in the live
  GraphQL schema and nowhere in the product docs.
- Neither Containers dataset appears on the
  [Datasets (tables)](https://developers.cloudflare.com/analytics/graphql-api/features/data-sets/)
  reference; they exist only in a
  [tutorial](https://developers.cloudflare.com/analytics/graphql-api/tutorials/querying-container-metrics/),
  which delegates permissions to the generic Account-Analytics page rather than
  stating a per-dataset requirement.

So `Account Analytics : Read` is the best-supported reading of the docs, not a
documented guarantee. The script closes that gap empirically by issuing the
exact three queries `worker.js` issues.

Reading its output:

- `token/verify` is **informational**. Account-owned and user tokens verify at
  *different* endpoints (`/accounts/{id}/tokens/verify` vs
  `/user/tokens/verify`) and each 401s on the other's token — measured: the
  wrangler OAuth token 401s at **both** while running all three GraphQL
  queries successfully. Gating on it would reject a working token.
- **`0 row(s)` is a PASS.** "Authorised, nothing billed in this window" is the
  normal state; nothing has run containers on this account since
  2026-08-26T22Z. Only an error means the token cannot do the job.
- A wrong-permission token surfaces as `code 9106 "Authentication failed"` on
  the GraphQL call — *not* at `token/verify`.
- **The blast-radius probe is part of the gate, not advice.** Proving a token
  can read the three datasets says nothing about what else it can do. Measured
  2026-08-28: a token created for this job also carried **Account API Tokens
  Write** (permission group `5bc3f8b21c554832afc660159ab75fa4`) — which can
  mint further tokens with arbitrary permissions, i.e. account takeover — and
  the script printed PASS, because every analytics query worked. It now GETs
  four surfaces an analytics-only token must not reach (account tokens, Workers
  scripts, KV namespaces, Containers control plane) and FAILs on any 200.
  Self-test: run it against the wrangler OAuth token and it trips 3 of 4.

If it reports over-privilege, fix the token rather than the script: dashboard →
Account API tokens → **Edit** → leave only `Account Analytics : Read`. Editing
permissions does **not** change the token value, so nothing downstream needs
re-pasting.

### 3. Set it

```sh
cd cloudflare/housekeeping
npx wrangler secret put CF_API_TOKEN   # paste at the prompt
curl -s https://riir-kat-housekeeping.foxfox.workers.dev/status | jq '{token_set, ts, ok}'
```

> **Sequencing:** deploy the T8 fix *before* setting this secret. Until it is
> deployed the worker still collides its hour keys, and its rows — which start
> carrying the per-app cost breakdown the moment this token exists — are still
> served unredacted on `/hours` and `/status`.

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

| route | anonymous | with `READ_TOKEN` | cache (anon / authed) |
|---|---|---|---|
| `/latest` | projection + `redacted:true` | full row | `public, max-age=60` / **`private, no-store`** |
| `/status` | liveness (`worker`, `cron`, `ts`, `ok`) + `latest` **as the projection**; config posture while the gate fails open | + `alerts_today`, `latest` as the full row | `public, max-age=60` / **`private, no-store`** |
| `/hours?n=` | **projection** rows, `n ≤ 168` *(401 once `READ_TOKEN` is set)* | full rows | `public, max-age=300` + isolate memo / **`private, no-store`**, no memo |
| `/days?n=` | rows, `n ≤ 90` *(401 once `READ_TOKEN` is set)* | same | as above |
| `POST /backfill` | **403** — a write route never fails open | repairs missing hour rows (§T9) | none |

**Why authenticated responses are `private, no-store` (§T9b).** RFC 9111 §3.5
says a shared cache MUST NOT reuse a response to a request carrying an
`Authorization` header *unless* the response says `public` / `s-maxage` /
`must-revalidate`. So `public` is not a neutral hint on this worker — it is
the exact directive that **grants** shared caches storage the default would
refuse, and it was being stamped on the token-bearing full rows (per-app ids,
per-app cost, mint anchor) on all four read routes. `Vary: Authorization`
keeps a *conformant* cache from serving one caller's body to another, but
that is a mitigation layered on an unnecessary grant and it assumes every
intermediary honours `Vary`. The isolate memo already had the right rule —
"an authenticated body must never be served from a cache an unauthenticated
caller can also hit" — and then the HTTP header handed the same body to every
cache on the path. Same bug class, one layer down.

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

## Repairing lost hours — `POST /backfill` (Issue 045 §T9)

§T8 recorded the hours its `hourKey()` bug collapsed as **unrecoverable**.
That is true of the KV *rows* and false of the *data*: KV here is a cache of
a source that is still live. Cloudflare's GraphQL analytics datasets retain
**~31 days**, and `datetimeHour` — the dimension the collectors were already
requesting and then summing away — is exactly the per-hour truth the archive
lost. So the same deploy that stops the bleeding also repairs the past.

```sh
H=https://riir-kat-housekeeping.foxfox.workers.dev
# rebuild the last 7 days (default), walking back from today
curl -sX POST -H "Authorization: Bearer $READ_TOKEN" "$H/backfill" | jq
# a specific window; `end` is inclusive and the walk goes backwards
curl -sX POST -H "Authorization: Bearer $READ_TOKEN" "$H/backfill?days=14&end=2026-08-27" | jq
# if it reports truncated:true, resume from the cursor it hands back
curl -sX POST -H "Authorization: Bearer $READ_TOKEN" "$H/backfill?days=14&end=<next_end>" | jq
```

Response: `{written, skipped, truncated, truncated_by, next_end, subrequests_spent,
`elapsed_ms`, per_day:[…]}`.

Four invariants, in the order they would cost to get wrong:

- **It protects rows that carry a real collection — not rows that merely
  exist.** A genuine cron row has `ai` + `mint_anchor`, neither of which
  billing can reconstruct per-hour, so clobbering one would destroy
  information in the name of repairing it. But *existence* is the wrong test,
  and getting that wrong was a live bug (§T9c): production reports
  `token_set:false`, so **every archived row today is a heartbeat**. An
  exists-based rule would have declared the whole archive protected and
  repaired nothing — and each placeholder written between the deploy and T4
  would have wedged its hour shut permanently. The discriminator is
  `row.ok === true && row.containers`; `ok` is load-bearing, because an
  all-collectors-failed row *does* get `containers:{est_usd:0,apps:{}}` (the
  collectors return empty maps rather than throwing) and that false zero must
  not read as data. Anything worth keeping on a replaced placeholder — a
  neuron count whose collector succeeded while the container ones failed — is
  merged forward, and the displaced row is recorded under `replaced`.
  Backfilled rows are tagged `source:"backfill"` so they are never mistaken
  for cron rows.

  The protect/replace decision is made from **`list()` metadata**, which the
  KV docs recommend over a `list()`+`get()` pair. Each hour row is written
  with a two-bit marker — `d` (carries a real collection) and `m` (carries
  something worth merging forward if replaced). Both bits are needed: without
  `m` the shortcut is self-defeating, since skipping the `get` also discards
  the body the merge needs. The common case — a pure heartbeat — costs zero
  gets. Rows written before this deploy carry no marker and are classified by
  a `get`; assuming either way is wrong (assume-data and the repair silently
  no-ops, assume-placeholder and a real collection is clobbered).
- **Idempotent.** Existence is decided by **one KV `list` per day**, not 24
  `get`s. A repeat run writes nothing and costs 3 subrequests per day —
  pinned by a test that asserts zero puts *and* zero gets on the second pass.
- **Bounded.** Workers count *every* KV binding call as a subrequest (50 free
  / 1,000+ paid), so an unbounded "repair the month" call would die mid-write
  leaving a half-written day that looks complete to the aggregate. Hence
  `days ≤ 14`, a 700-subrequest budget, and a stop that only ever lands on a
  day boundary and returns a `next_end` cursor.
- **`POST`, and it never fails open.** The read routes fail open while
  `READ_TOKEN` is unset so the owner keeps a diagnostic path; this one spends
  upstream GraphQL quota and writes KV, so an unset token is a `403`. `GET` is
  a `405` — a link prefetch or crawler must not be able to trigger it.
- **A 45 s wall-clock guard**, on top of the subrequest budget. 14 days is 28
  sequential GraphQL round-trips, which can outlive the edge's client-connection
  timeout (`524`). A `524` here is *safe* — every KV write has committed and the
  route is idempotent — but it is indistinguishable from a broken worker and it
  loses the cursor. Stopping ourselves returns a clean
  `truncated:true, truncated_by:"wall_clock", next_end:…` instead.

Two things it deliberately does **not** do:

- **No TTL / pruning on `hour:` rows.** Tempting (they grow ~9k keys/yr, ~10 MB
  against a 1 GB tier) and wrong: past ~31 days GraphQL can no longer rebuild
  them, so expiring a row would make the archive strictly worse than the
  source it caches. The `/hours?n=` cap is a *read* bound, not a retention one.
- **No per-hour `ai` reconstruction.** `aiInferenceAdaptiveGroups` is a daily
  rollup; the honest answer is `null`, not a divided-by-24 fiction.

## Tests

```sh
node --test cloudflare/housekeeping/test/*.test.mjs   # 28 tests, no deps
```

A bare directory arg fails `MODULE_NOT_FOUND` — `node --test` wants file
paths. The Worker is dependency-free ESM and every global it touches exists
in Node 18+, so it runs unmodified under `node:test` with a KV stub.

These exist because the §T8 key-collision bug was live for two days while
every "live-verified" check passed: the endpoints returned `200` with
plausible JSON throughout. Asserting on the **shape of the KV keys** is what
catches that class; asserting on HTTP status is not. Verified as a real pin —
8 of the original 12 fail against the pre-fix worker
(`git show HEAD~1:.../worker.js`); §T9 added 7 more, §T9b 3, and §T9c 6 — the
behavioural ones in each batch confirmed to fail against the worker they fix — a test
that has never been seen red is not a pin.

The KV stub also models per-key `list()` metadata, because the backfill's
protect/replace decision reads it. A stub that dropped metadata would have
left the legacy `get` fallback as the only path under test — the same
unfaithful-instrument problem as the missing `datetimeHour`.

One of them needed a purpose-built clock. `atUtcAdvancing()` advances
`Date.now()` on every read, because the fixed-instant `atUtc()` makes any
elapsed-time guard measure zero and pass vacuously.

One of those 12 was itself unfaithful and was fixed in the §T9 pass: the
GraphQL stub **omitted `datetimeHour`**, the dimension the query asks for. A
whole-day window therefore looked identical to a one-hour window, so no test
in the suite could have caught an hour-grouping defect — the exact class of
bug the suite exists for. The stub now echoes one group per hour, and
`omitHour:true` reproduces the old shape on purpose to pin the cron's
fallback behaviour.
`scripts/ops_nightly.sh` runs them WARN-only (it does not deploy this
Worker, so it has nothing to gate).

## Re-deploying the dashboard after changes here

The static dashboard reads CF GraphQL directly at generation time
(`scripts/gen_dashboard.py`); the Worker's KV is the running archive +
future live-mode source (`/hours`, `/days`).

## Observability (added 2026-08-29, §T9c)

`wrangler.toml` now carries `[observability] enabled = true,
head_sampling_rate = 1`. Every documented failure mode of this worker is
**silence** — the §T8 key collision ran two days behind green checks, and the
collectors degrade to an `ok:false` row nobody reads — so an unlogged throw is
the worst possible shape. The cron's own error path was also unguarded:
`runHousekeeping`'s internal `try` covers *collection*, but the KV writes
after it are outside it, so a KV outage rejected straight into `waitUntil()`
and the run vanished leaving no row and no log. `scheduled()` now catches and
`console.error`s. Cost is nil at 24 cron runs/day against the 20M
logs/month included on Paid.

## Pricing constants (pinned 2026-08-26)

`MEM_USD_PER_GIB_S=2.5e-6` · `DISK_USD_PER_GB_S=7e-8` ·
`CPU_USD_PER_S=2e-5` — Workers Paid, beyond the included 25 GiB-h mem /
375 vCPU-min / 200 GB-h disk per month. A change is a `[vars]` re-pin +
redeploy (the `CHARGE_VERSION` discipline from Proposal 005).
