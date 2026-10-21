# riir-kat-housekeeping (Issue 045)

Hourly `$0` housekeeping Worker: **container cost tracking + KAT mint
anchor**. Deployed at `riir-kat-housekeeping.<account>.workers.dev`.

- **Cron** `0 * * * *` — pulls CF GraphQL (`containersUsageAdaptiveGroups`
  billing dataset + `containersMetricsAdaptiveGroups` awake seconds +
  `aiInferenceAdaptiveGroups` day neurons), computes MARGINAL est cost
  (usage × rate from unit zero — never a bill quantity, Issue 065), writes KV
  rows (`hour:*`, `day:*`, `latest`), and records the **KAT mint anchor**.
  When `MONTHLY_BUDGET_USD` is set it additionally reconciles the account-wide,
  **period-scoped CF bill** (`src/bill.js`, Issue 065 T7) for alerting.
- **Mint:** records the local anchor for the in-progress UTC day
  (`row.mint_anchor`, `status: "accruing"`; `row.mint: "anchor-only"`).
  The ledger shell landed 2026-08-29 (riir-dapps Plan 013 kat-service):
  when BOTH `KAT_SERVICE_URL` + `KAT_MINT_TOKEN` are set, every cron run
  also POSTs the most recent COMPLETED UTC day's burn to
  `${KAT_SERVICE_URL}/mint-anchor` (bearer auth; the server answers
  `already-recorded` on replay, so hourly re-posts are idempotent) —
  Issue 045 T6 / Issue 044 T5, landed 2026-09-01. Unset → inert (the
  deployed-but-inert pattern); no KAT math in the worker.
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
node cloudflare/housekeeping/verify_token.mjs <token>     # or CF_OPS_API_TOKEN=... node ...
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
# From the repo root. Worker-side binding name stays CF_API_TOKEN (deployed
# secret); locally it is CF_OPS_API_TOKEN.
grep '^CF_OPS_API_TOKEN=' .env | cut -d= -f2- | (cd cloudflare/housekeeping && npx wrangler secret put CF_API_TOKEN)
curl -s https://riir-kat-housekeeping.foxfox.workers.dev/status | jq '{token_set, ts, ok}'
```

> **Sequencing:** deploy the T8 fix *before* setting this secret. Until it is
> deployed the worker still collides its hour keys, and its rows — which start
> carrying the per-app cost breakdown the moment this token exists — are still
> served unredacted on `/hours` and `/status`. (As of §T9e `/hours` is
> token-gated outright, so on the *current* build the exposure is `/status`
> alone; the sequencing advice stands because the DEPLOYED worker is the old
> one.)

### "Why not just use wrangler? It already authenticates."

It does — **on the M3**. `scripts/gen_dashboard.py:_cf_token()` prefers
`CF_OPS_API_TOKEN` but falls back to the wrangler OAuth file, so the generator and
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
npx wrangler secret put ALERT_WEBHOOK_URL   # Discord-compatible webhook URL
npx wrangler secret put MONTHLY_BUDGET_USD  # optional monthly container budget (USD)
```

- **Alert thresholds:** `50% / 90% / 100%` of `NEURON_CAP` (`[vars]`, default
  `10000` neurons/day). One webhook per threshold per day — deduped via KV
  key `alert:<date>:<threshold>`, written **after** the POST attempt
  regardless of success (a failed POST is **not** retried next hour — no
  retry storms; check the KV row's `notified` flag).
- **At 100%:** the message adds `CAP EXCEEDED — new generations hard-fail
  until 00:00 UTC`.
- **Monthly budget:** if `MONTHLY_BUDGET_USD` is set, the run reconciles the
  **period CF bill** against the budget and alerts once per billing period
  (`alert:bill:<periodStartYYYYMMDD>`) when the bill's total reaches the
  budget. **Issue 065 T7 (landed 2026-09-04) replaced the old model**, which
  summed per-day `est_usd` rows — the ⚠ note below records what that model
  got wrong, kept as the history of the fix:
  > ⚠ **The pre-T7 model (do not resurrect):** `costUsd()` computed
  > `usage × rate` from unit **zero** — the Workers-Paid included allowances
  > (25 GiB-h mem / 200 GB-h disk / 375 vCPU-min per month) were never
  > subtracted, and the alert summed *per-day* figures whose allowance cannot
  > be apportioned to a day anyway. Measured 2026-09-03: that model says
  > **$0.3780** where Cloudflare billed **$0.08** (4.7×), so a budget set from
  > the real bill would alarm at roughly a fifth of the intended spend. It
  > also modelled containers only, and containers were **1.0%** of the bill —
  > KV was $8.00 of $8.08. It was **inert** (`/status`: `has_monthly_budget:
  > false`, `has_webhook: false`) — the only reason it was latent rather than
  > live.
  >
  > **The T7 model (`src/bill.js`, ported from `scripts/gen_dashboard.py`):**
  > `billable = max(0, period_account_total − included)`;
  > `usd = ceil(billable / block) × price` — allowances subtracted, KV's
  > round-up-to-the-next-million applied (301,970 billable list ops cost the
  > full $5.00, not $1.51), and **KV ops collected** (`kvOperationsAdaptiveGroups`
  > by actionType). Verified against the 2026-09-03 bill to the cent in
  > `test/bill.test.mjs` (`.benchmarks/cf_bill_pin.json`); two of those pins
  > were mutation-checked red (drop the allowance subtraction → 6 red; price
  > linearly → 4 red).
  - **Period start is a PIN.** Neither this worker's credential nor the
    dashboard's can read the billing anniversary (analytics GraphQL only), so
    `CF_BILLING_ANNIVERSARY_DAY` (a `[vars]` entry, default **16** = the
    dashboard's pin, bounded to 2026-08-15..2026-08-26) scopes both bill
    queries and the dedupe key. Override once the real day is known.
  - **Degraded collectors are unwatched, not zero.** A failed bill collector
    leaves its lines OUT of the reconciliation and the alert names them
    (`NOT counted (collector failed): …`) — an uncollected line and an unused
    one are different facts (the Issue 065 defect that hid $8.00 of KV behind
    a $0.08 container total).
  - **Cost discipline unchanged:** the dedupe get runs BEFORE the two bill
    GraphQL calls, so a post-alert run spends zero bill subrequests (pinned
    by test).
- **`day:*` rows are recomputed, not accumulated** (fixed 2026-08-28): each
  run sums the authoritative `hour:<day>*` rows. The previous
  read-modify-write double-counted on a cron double-fire or manual invoke and
  could lose an update between overlapping runs.
- Alerting only runs when `CF_API_TOKEN` is set (neurons are only known
  there). Without a webhook, alerts still land in KV (`notified: false`).

## Optional secret (read gating, Issue 045 T7)

```sh
npx wrangler secret put READ_TOKEN   # gates /hours, /days, /backfill, /status config half
```

## Deploy / read

```sh
npx wrangler deploy
curl https://riir-kat-housekeeping.<account>.workers.dev/status
curl -H "Authorization: Bearer $READ_TOKEN" '.../hours?n=24'   # hourly rows (403 without)
curl -H "Authorization: Bearer $READ_TOKEN" '.../days?n=14'    # day aggregates (403 without)
```

> **Credentials (2026-10-21):** wrangler uses `CLOUDFLARE_API_TOKEN` (the
> deploy token) when set, else your `wrangler login` OAuth session. The
> analytics token lives in `.env` as `CF_OPS_API_TOKEN`, a name wrangler never
> reads, so it cannot shadow a deploy. Only the Worker-side secret binding is
> still called `CF_API_TOKEN`.

### Access model (hardened 2026-08-28 — Issue 045 §T7, revised §T8, §T9e)

There are **two** gate levels, and the distinction is the whole design:

| level | condition | what it guards |
|---|---|---|
| fail-**open** | `READ_TOKEN` unset ⇒ pass | liveness + config posture only, so the T4 verification commands work before any token exists. Costs **one memoised KV get** for the whole anonymous surface. |
| **hard** | `READ_TOKEN` set **and** presented | account telemetry, anything that spends a KV `list`, and every **fan-out** route |

| route | anonymous | with `READ_TOKEN` | cache (anon / authed) |
|---|---|---|---|
| `/latest` | projection + `redacted:true` | full row | `public, max-age=60` / **`private, no-store`** |
| `/status` | liveness (`worker`, `cron`, `ts`, `ok`) + `latest` **as the projection**; config posture while the gate fails open | + `alerts_today`, `latest` as the full row | `public, max-age=60` / **`private, no-store`** |
| `/hours?n=` | **403** — a fan-out route never fails open (§T9e) | full rows, `n ≤ 168` | none / **`private, no-store`** + token-salted isolate memo |
| `/days?n=` | **403** (§T9e) | rows, `n ≤ 90` | as above |
| `POST /backfill` | **403** — a write route never fails open | repairs missing hour rows (§T9) | none |

**Why `/hours` + `/days` stopped failing open (§T9e).** They are the only
fan-out routes here: one request can issue up to **168 KV gets**. The isolate
memo that was supposed to blunt that is *per-isolate*, so traffic spread
across colos misses it every time — roughly **595 anonymous requests exhaust
the 100k/day KV read tier**. What breaks when reads run out is not the chart:
it is the day-aggregate recompute and the alert-dedupe `get`, so
unauthenticated traffic could **silently disable budget alerting**. Redaction
(the §T8 answer) did not help, because producing a redacted row costs the same
fan-out as producing a full one.

Checked before closing it rather than assumed: the ops dashboard makes
**exactly one** cross-origin call — `/latest` — and renders its hourly/daily
container charts from the statically baked `<script id="ds">` blob that
`scripts/gen_dashboard.py` writes. Every `/hours` / `/days` invocation in this
README already passes a token. So the gate costs **zero functionality today**,
and the diagnostic path the fail-open posture existed for (`token_set`,
`read_gated`, liveness) lives on `/status` + `/latest`, which are unchanged.
They return **403, not 401**, while `READ_TOKEN` is unset — the same rule and
the same wording `/backfill` already used.

**Token compare is over digests (§T9e).** The previous constant-time compare
XOR-ed the raw bytes and looped `max(len(guess), len(secret))` times. That
makes the *verdict* length-independent but not the *timing* — the thing it was
named for: a 1-char guess still ran `len(READ_TOKEN)` iterations, so the
response time leaked the secret's length. Both sides are now SHA-256'd first,
making every comparison a fixed 64-hex-char loop. The accepted digest doubles
as the memo salt, so an authenticated fan-out can finally be memoised
(previously the authenticated path had no memo at all, so hard-gating the
route would have meant *nothing* was memoised) without a rejected caller ever
being able to name a live key.

> The tests pin the **verdicts** of that compare across secret lengths 1 / 6 /
> 200, not its **timing** — a wall-clock timing assertion would be flaky in
> CI. The timing property rests on the fixed-width loop being fixed-width by
> construction, which is a code-shape argument, not a measured one.

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

## ⚠ Read this before deploying: `/hours` and `/days` go blank first

Measured live 2026-08-29 — the whole KV namespace is **8 keys**:

```
7  hour:20260826T1, hour:20260826T2, hour:20260827T0 … hour:20260828T1   (legacy collided)
1  latest
0  day:*          ← none, after three days of hourly runs
0  hour:YYYYMMDDHH ← none in the post-fix shape
```

Three consequences of deploying the fix, all of which look like breakage:

0. **`/hours` and `/days` start answering 403 to an anonymous caller** (§T9e).
   They are token-gated now, so *verifying* any of the below requires
   `READ_TOKEN` to be set first. That is not an extra step in practice — the
   backfill in point 1 already required it — but it does mean **`wrangler
   secret put READ_TOKEN` moves ahead of the verification curls**, not after
   them. An anonymous `curl .../hours` returning 403 is the gate working, not
   the deploy failing.

1. **The hour chart empties.** The deployed worker uses the buggy `hourKey()`
   on *both* the write and the read path, so its reads currently land on the
   same 7 collided keys — that is why `/hours?n=24` returns 24 rows with only
   4 distinct `(ts,hour)` pairs. The fixed worker constructs
   `hour:YYYYMMDDHH`, and **none of those exist yet**. So `/hours` returns
   `[]` immediately after deploy, filling one row per hour thereafter. Run the
   backfill to populate it at once. The 7 legacy keys become permanent
   orphans: unreachable by every read path and filtered out of the aggregate
   by `HOUR_KEY_RE`. They are inert — leave them. Do not "repair" them.
2. **`day:` starts existing for the first time.** There were zero, because the
   heartbeat path `return`ed before the aggregate — so `/days` had been
   serving `[]` since deploy, indistinguishable from "nothing ran", and the
   aggregate code was never once exercised in production. Fixed in §T9d: a
   heartbeat day now writes `hours: 0, hours_present: 24`. Those two counters
   are deliberately separate — reporting `hours: 24, est_usd: 0` for a day
   that collected nothing is a false zero, and a false zero looks like a
   measurement.

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
- **No lock on `POST /backfill`.** Two concurrent calls would both run,
  doubling the GraphQL spend; the route is idempotent so the rows converge.
  Deliberately not "fixed" with a KV flag: KV is eventually consistent, so a
  read-then-write lock on it would fail exactly when two calls race — the one
  case it exists for. It would buy false assurance, not mutual exclusion. The
  route needs the owner token, so the realistic caller count is one.
- **No per-hour `ai` reconstruction.** `aiInferenceAdaptiveGroups` is a daily
  rollup; the honest answer is `null`, not a divided-by-24 fiction.

## Tests

```sh
node --test cloudflare/housekeeping/test/*.test.mjs   # 56 tests, no deps
```

`test/bill.test.mjs` (added Issue 065 T7) pins the bill model to the real
2026-09-03 bill (`.benchmarks/cf_bill_pin.json`) and pins the `[vars]`
marginal rates equal to the BILL_LINES prices — two price sources exist on
purpose (marginal vs bill) and silent drift between them is a red test.

A bare directory arg fails `MODULE_NOT_FOUND` — `node --test` wants file
paths. The Worker is dependency-free ESM and every global it touches exists
in Node 18+, so it runs unmodified under `node:test` with a KV stub.

These exist because the §T8 key-collision bug was live for two days while
every "live-verified" check passed: the endpoints returned `200` with
plausible JSON throughout. Asserting on the **shape of the KV keys** is what
catches that class; asserting on HTTP status is not. Verified as a real pin —
8 of the original 12 fail against the pre-fix worker
(`git show HEAD~1:.../worker.js`); §T9 added 7 more, §T9b 3, §T9c 6 and §T9d 3 — the
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
future live-mode source (`/hours`, `/days`). Live mode will have to **present
`READ_TOKEN`** — those routes stopped failing open in §T9e — which means
shipping a token to a browser, i.e. it wants a dashboard-side proxy rather
than a direct `fetch` from the page. The dashboard already proxies (its CSP
`connect-src` names only this origin, and only `/latest` is called today).

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

These are the **MARGINAL** rates. The **BILL** model (allowances subtracted,
KV's round-to-the-next-million, KV ops) lives in `src/bill.js`, mirrored from
`scripts/gen_dashboard.py` and pinned to the real bill by
test/bill.test.mjs — including a test that the `[vars]` rates equal the
BILL_LINES prices. Since Issue 065 T7 the MONTHLY_BUDGET_USD alert uses the
bill model, never these marginal rates.
