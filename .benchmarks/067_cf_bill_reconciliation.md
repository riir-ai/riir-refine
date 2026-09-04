# Bench 067 — the CF bill vs the ops dashboard: what $8.08 was actually made of

**Status:** ALIGNED (2026-09-03) — 6/6 collected bill lines reproduce to the cent, total $8.08 = $8.08
**Issue:** riir-clippy ``.issues/065``
**Harness:** `scripts/gen_dashboard.py` (`bill_line` / `bill_reconciliation` / `collect_kv`) against
`.benchmarks/cf_bill_pin.json` — the bill transcribed from Cloudflare's Billing → Usage page
**Run:** `set -a; . ./.env; set +a; python3 scripts/gen_dashboard.py --out /tmp/dash.html`
→ `bill=$8.08 over 6 lines since 2026-08-16`

## The question

The dashboard reported container spend and Workers-AI neurons. Cloudflare reported $8.08.
Asked of one row specifically: app `a03a70ff-88e9-45c5-af80-339ab7707fbe` showed **234,298 GB-s
of disk** on the dashboard and no corresponding disk cost on the bill. Was the app missing from
Cloudflare, or the cost?

## Answer: the COST was missing, and the app is 95% of the line

The app is not absent from Cloudflare — it dominates every container line it touches:

| quantity | a03a70ff | account (bill) | share |
|---|---|---|---|
| container egress | 17.66 GB | 17.66 GB | **100.0%** |
| container memory | 117,149 GiB-s | 122.94k GiB-s | 95.2% |
| container disk | 234,298 GB-s | 246.33k GB-s | 95.1% |
| container vCPU | 998.5 vCPU-s | 1.02k vCPU-s | 97.9% |

What is absent is a *charge*: 246.33k GB-s is **34% of the 720,000 GB-s (200 GB-hour) monthly
allowance**, so the disk line is billable 0 → **$0.00**. The dashboard was pricing it at
$0.0164 for that app because its cost model started at unit zero.

All of the app's usage lands on a single date, **2026-08-26** (32.5 GiB-hours of memory, 16.6
vCPU-minutes, 17.66 GB egress). That is not a date-attribution bug — the hourly metrics dataset
returns zero rows for the last 48h, consistent with nothing having run since.

## The three rules the model was missing

`billable = max(0, PERIOD_ACCOUNT_total − included)`; `usd = ceil(billable / block) × price`.

1. **The allowance is subtracted.** It never was. Measured: $0.3780 of "container cost" against
   a real container bill of $0.08 — **4.7×**.
2. **The allowance is monthly and ACCOUNT-WIDE.** Not per app, not per namespace, not per day.
   So a per-app or per-day dollar figure is not a billing quantity at all, and the per-app table,
   the 14d daily bars and the 4w weekly bars were all structurally unbillable. They are now
   labelled **marginal** — what a slice adds *once* the account is past the allowance.
3. **The remainder rounds UP to a whole block.** KV is sold by the million. 301.97k billable list
   ops cost the full **$5.00** of a block, not $1.51. This is the only direction that makes a
   naive model too **LOW**, so it cannot be waved through as conservative.

## The reconciliation (live GraphQL vs the transcribed bill)

| line | used | included | billable | blocks | model $ | bill $ | Δ |
|---|---|---|---|---|---|---|---|
| KV list ops | 1,319,010 | 1M | 319,010 | 1 × $5.00 | 5.0000 | 5.00 | ✓ |
| KV read ops | 15,292,220 | 10M | 5,292,220 | 6 × $0.50 | 3.0000 | 3.00 | ✓ |
| container memory | 123,074 GiB-s | 25 GiB-h | 33,074 | — | 0.0827 | 0.08 | ✓ |
| container disk | 246,598 GB-s | 200 GB-h | 0 | — | 0.0000 | 0.00 | ✓ |
| container vCPU | 1,019 vCPU-s | 375 vCPU-min | 0 | — | 0.0000 | 0.00 | ✓ |
| KV write ops | 418,140 | 1M | 0 | 0 × $5.00 | 0.0000 | 0.00 | ✓ |
| **total** | | | | | **$8.08** | **$8.08** | **✓** |

`kv_delete` is reported as **uncollected**, not as zero — an unmeasured line and an unused one
are different facts, and conflating them is the error this whole bench is about.

The container *usage* rows carry a 0.11% residual (246,598 vs 246,330 GB-s) because the billing
period start is a **pin**, not a measurement — see below. Both lines are billable 0, so the
residual does not reach the dollars.

## The finding that matters more than the fix: the dashboard was modelling 1% of the bill

| family | bill | dashboard modelled it? |
|---|---|---|
| KV list + read ops | **$8.00 (99.0%)** | **no — not at all** |
| container memory | $0.08 (1.0%) | yes (4.7× too high) |
| everything else | $0.00 | partly |

And the attribution is not this repo's: **one namespace is 99.98% of every KV op on the account.**

| namespace | total ops | share | read:list |
|---|---|---|---|
| `d2cdb46dee30430b96dbf5b439ed318b` | 17,026,180 | 99.98% | 11.6× |
| `a3a0add9325f4078a16e32eb26f54932` (riir-clippy `HOUSEKEEPING_KV`) | 2,750 | 0.02% | 9.0× |
| `98a0ccc465034a1a994c4485591dd2f0` | 400 | 0.00% | — |
| `31b4238227184be1acc15581c1e449c5` | 40 | 0.00% | — |

`d2cdb46d…` appears in **no wrangler config anywhere under `/Users/katopz/git`**. Its 11.6
reads-per-list is the signature of a list-then-read-each loop — the shape that converts a 1M-op
allowance into a $5.00 block. Worker CPU points the same way: `scriptName __unknown__` is
522,185 requests and 4,034,105 cpu-ms, **97.9%** of the account's 4.12M Workers CPU ms, while
riir-clippy's own two workers are 381 requests combined (`riir-kat-housekeeping` 290,
`riir-ops-dashboard` 91). **The $8.00 is not riir-clippy's and its owner is unidentified** —
Issue 065 T3.

## What could not be measured, and why it is a pin rather than a number

The **billing period start** is not derivable with either credential in this repo: `/billing/*`,
`/subscriptions`, `/accounts/{id}/storage/kv/namespaces` and `/workers/scripts` all answer
`10000 Authentication error` for both `CF_API_TOKEN` and the wrangler OAuth token — they carry
analytics GraphQL only. Bound established by arithmetic instead: the bill's container lines
reproduce from the 2026-08-26 rows **alone** to within 0.11% and **exclude** the 2026-08-14 rows,
so the period starts in **2026-08-15 .. 2026-08-26**. Every day in that bound gives the identical
reconciliation over present data, because no usage row falls between 08-15 and 08-25 — so the pin
is currently **unfalsifiable**, which is exactly why the card labels it `day PINNED` instead of
trusting it. Override with `CF_BILLING_ANNIVERSARY_DAY`.

Two more instrument limits found while measuring, both of the "silent zero" class:

- `containersUsageAdaptiveGroups` / `kvOperationsAdaptiveGroups` **reject** a window wider than
  4w4d wholesale (`quota` error) and return **zero rows** — indistinguishable from an idle
  account. `collect_kv` clamps to 30d for this reason.
- The wrangler OAuth token in `~/Library/Preferences/.wrangler/config/default.toml` **expired
  2026-09-01T01:30Z**. Anything relying on the OAuth fallback rather than `.env`'s
  `CF_API_TOKEN` silently collects nothing.

## Verification

- 6/6 collected lines and the total reproduce the transcribed bill to the cent (table above).
- `.benchmarks/cf_bill_pin.json` records Cloudflare's **own** `billable` and `usd` figures; they
  are not recomputed. They are what pins the two non-obvious rules. Where Cloudflare prints
  `used` at 3 significant figures and `billable` exactly, `used` is reconstructed as
  `billable + included` and flagged `used_reconstructed` — transcribing the displayed `1.3M`
  instead of `1,301,970` puts a 1,970-op error into the one line whose block rounding is 30% of
  a block.
- The renderer is **executed**, not just parsed: `node --check` on the extracted script, then the
  script run against a DOM stub — `billTable` 1995 chars, `kvTable` 1304, `contTable` 2238,
  `kpis` 1397, no throw. A syntax check would not have caught a renderer that reads a payload
  key that does not exist.
