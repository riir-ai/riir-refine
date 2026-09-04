# Issue 065 — the ops dashboard's cost model did not model the bill

**Status:** T1/T2/T4/T5/T8 DONE (2026-09-03; DEPLOYED + auth-gated) · T3 DONE (owner-resolved 2026-09-03: zed agent bug — see T3's inline note) · T6 DONE (verdict PASS 2026-09-03T18:23 UTC) · **T7 DONE (2026-09-04; deployed + live-verified — see T7's resolution note)**
**Evidence:** [`.benchmarks/067_cf_bill_reconciliation.md`](../.benchmarks/067_cf_bill_reconciliation.md) ·
bill snapshot [`.benchmarks/cf_bill_pin.json`](../.benchmarks/cf_bill_pin.json)
**Surface:** `scripts/gen_dashboard.py`, `cloudflare/dashboard/public/index.html` (generated)

## What was wrong

Asked why app `a03a70ff-88e9-45c5-af80-339ab7707fbe` showed 234,298 GB-s of container disk with
no matching cost on Cloudflare. The app is **not** missing from the bill — it is 95.1% of the
disk line and 100% of container egress. Its **cost** is missing, because 246.33k GB-s is 34% of
the 720,000 GB-s (200 GB-hour) monthly allowance and the line is billable 0.

Three defects, in increasing order of how much money they hid:

1. **Allowances were never subtracted.** `container_cost_usd` and its JS twin `contCost` computed
   `usage × rate` from unit zero, while the card sub-text claimed "beyond the Workers-Paid
   included allowances (25 GiB-h mem / 375 vCPU-min / 200 GB-h disk per month)". The prose
   described a subtraction the arithmetic never performed: $0.3780 reported against a real
   container bill of $0.08 (**4.7×**). One legend on the same page already said "allowances not
   subtracted" — the page contradicted itself and nothing failed.
2. **The allowance is monthly and account-wide**, so per-app / per-day dollars are not billing
   quantities *in principle*. The per-app column, the 14d daily bars and the 4w weekly bars were
   all structurally unbillable, and all three were labelled "est cost".
3. **KV was not modelled at all — and KV was 99.0% of the bill.** $8.00 of $8.08 was KV list +
   read ops. The dashboard tracked the $0.08 family and the free neuron family, and was blind to
   the only line being charged for. KV also rounds the billable remainder **up to a whole
   million**, so 301.97k list ops cost the full $5.00 block — the one rule under which a naive
   linear model reads too *low*.

## Tasks

- [x] **T1** — model the bill properly: `BILL_LINES` (included allowance, unit price, block
      granularity per line) + `bill_line()` + `bill_reconciliation()`, account-wide and
      period-scoped. Verified 6/6 lines and the total to the cent against the transcribed bill.
- [x] **T2** — collect KV: `collect_kv()` over `kvOperationsAdaptiveGroups` by
      namespace × action × day, degrading like the container collector. New per-namespace card.
- [x] **T4** — stop calling marginal cost "cost": `contCost` → `contMarginal`,
      `contDayCost` → `contDayMarginal`, headers and card titles relabelled **MARGINAL**, and the
      false sub-text replaced with the worked a03a70ff example. New KPIs lead with
      `CF billed $ (period)` and `top billed line`.
- [x] **T5** — make drift detectable instead of self-agreeing: `.benchmarks/cf_bill_pin.json`
      carries Cloudflare's own `billable`/`usd` figures (never recomputed) and the reconciliation
      card renders a per-line Δ against it, with `not pinned` distinct from `✓`.
- [x] **T3** — **owner call: identify who owns KV namespace `d2cdb46dee30430b96dbf5b439ed318b`.**
      99.98% of every KV op on the account (17,026,180 ops: 15.29M read / 1.32M list / 417.6k
      write) and it appears in **no wrangler config under `/Users/katopz/git`**. Its 11.6
      reads-per-list is a list-then-read-each loop; caching or a single `list` + batched reads
      would take the $5.00 list block to $0.00 (billable would fall under the 1M allowance) and
      the read line from 6 blocks to 5. Worker CPU agrees: `scriptName __unknown__` is 522,185
      requests / 4,034,105 cpu-ms = 97.9% of the account's Workers CPU. riir-clippy's own two
      workers are 381 requests combined. **Not this repo's spend — needs the account owner.** 
      // it's zed agent bug, has been resolved.
- [x] **T8** — **the live dashboard was serving its whole private payload
      unauthenticated, and this change widened it.** Found by *fetching* the
      worker after deploy rather than trusting the deploy's exit code:
      `x-dashboard-auth: OPEN-INSECURE` + `x-dashboard-warning: DASHBOARD_TOKEN
      unset`. The secret had never been set since Issue 044 T13 armed the gate,
      so `UNCONFIGURED_POLICY` defaulted to `serve` on every request. Exposure
      inventory at that moment (all readable with a bare `curl`, no header):
      4 KV namespace IDs **with per-namespace op counts**, the full per-line bill
      breakdown and the `cf_bill_pin.json` account usage across 12 product lines
      (**all three added by this commit**), plus 6 durable-object IDs, 6
      container app UUIDs, 11 private repo names with per-repo unhealed-lint
      counts, and 23 LLM ledger rows — 45,391 bytes of private engineering
      profile. **CLOSED 2026-09-03** by `wrangler secret put DASHBOARD_TOKEN`
      (32 random bytes, url-safe) with the least-privilege deploy credential.
      Verified on the live worker, all four branches, not inferred: anon → **401**
      / 61 bytes / zero payload / namespace id absent; wrong token → **401**, no
      echo; correct token → **301** with the token stripped from `Location` and
      an `HttpOnly; Secure; SameSite=Lax; Max-Age=2592000` cookie planted;
      cookie → **200** with the payload. Token stored in the gitignored `.env`
      as `DASHBOARD_TOKEN`. `UNCONFIGURED_POLICY = "deny"` was offered and
      **declined** as too disruptive — so the fail-open branch still exists and
      is unreachable only while the secret lives. Deleting the secret
      re-publishes the payload silently; that residual is the reason to
      eventually take the `deny` flip.
- [x] **T7** — the **same defect ships in the housekeeping Worker**, latently.
      `costUsd()` (`cloudflare/housekeeping/src/worker.js:296`) is the identical
      from-unit-zero model, and the `MONTHLY_BUDGET_USD` alert sums *per-day*
      `est_usd` rows — a quantity whose allowance cannot be apportioned to a day
      even in principle. A budget set from the real bill would alarm at ~1/4.7 of
      the intended spend, and it watches only the family that is 1.0% of the
      bill. **Currently inert**: `/status` reports `has_monthly_budget: false`
      and `has_webhook: false`, so nothing fires. Deliberately NOT fixed here —
      fixing it means redeploying a live Worker for a path that cannot currently
      execute, and the trap is now documented at the point of enablement
      (`cloudflare/housekeeping/README.md`, budget bullet). Port `bill_line()` /
      `bill_reconciliation()` before setting the secret.
      **RESOLVED 2026-09-04** (deployed + live-verified; `/status` answers
      `bill_model: "issue065-t7"`):
      * **`cloudflare/housekeeping/src/bill.js` (new)** — the bill model ported
        from `scripts/gen_dashboard.py`: `BILL_LINES` / `billLine()` /
        `billReconciliation()` / `billingPeriodStart()` / `KV_ACTION_TO_LINE`.
        Wrangler bundles the ESM import; still no build step.
      * **`marginalUsd()`** — `costUsd()` renamed + doc-commented with the
        marginal-vs-billed contract (the dashboard's `container_marginal_usd`
        wording); `marginal: true` added to hour-row `containers` and day
        aggregates so the semantics are labelled at the read site.
      * **The alert is bill-shaped**: account-wide, period-scoped (the
        `CF_BILLING_ANNIVERSARY_DAY` `[vars]` pin, default 16 = the
        dashboard's), allowances subtracted, KV's round-up applied, and **KV
        ops collected** (`kvOperationsAdaptiveGroups` by actionType — the
        family that was $8.00 of $8.08). Two new ungrouped-period-total query
        shapes (the `aiNeuronsToday` shape proves the dataset supports it);
        degraded collectors leave their lines OUT and the alert names them as
        unwatched (uncollected ≠ zero — the Issue 065 defect-3 lesson). Dedupe
        `alert:bill:<periodStartYYYYMMDD>` checked BEFORE the 2 GraphQL calls
        (cost discipline kept: post-alert runs spend zero bill subrequests).
        Dedupe row records `bill_usd` / `marginal_usd` / `uncollected`.
      * **Pins**: `test/bill.test.mjs` (new) reproduces the 2026-09-03 bill to
        the cent from `.benchmarks/cf_bill_pin.json` (incl. KV's 301,970 →
        one whole $5 block), pins `billingPeriodStart` off UTC fields, and
        pins the `[vars]` marginal rates == the BILL_LINES prices (two price
        sources exist on purpose; drift is a red test). The worker budget
        tests replace the day-row-sum test with: bill-shaped firing + period
        window in the queries, stop-after-alert (0 bill fetches post-dedupe,
        3 hourly collectors still run), KV-collector-failure → uncollected
        named in the alert, and no-budget → collectors never run. Both
        mutation checks bite (allowance subtraction dropped → 6 red; linear
        instead of ceil → 4 red).
      * 56/56 tests green (`node --test cloudflare/housekeeping/test/*.test.mjs`).
      * **Still true after the fix**: `MONTHLY_BUDGET_USD` and
        `ALERT_WEBHOOK_URL` remain UNSET, so the alert stays inert until the
        owner sets them — but setting them is now SAFE: the number the alert
        compares is the bill. The trap documentation in the README budget
        bullet is now the description of the fixed behaviour (pre-T7 model
        preserved as a do-not-resurrect note).
- [x] **T6** — the M3 nightly has failed `exit 124` on **2026-09-01 and 2026-09-02**, so the
      deployed dashboard is pinned at the 2026-09-01 03:44 artifact. Structural, not flaky: the
      whole-run budget is `timeout 1200` (`scripts/ops_nightly.sh:223`) while the **per-repo**
      coverage dry-run budget is `timeout=600` (`scripts/gen_dashboard.py:1089`) across 11 repos —
      one slow repo can consume half the run. Measured: 09-01 logged
      `riir-ai: coverage dry-run timeout` and both nights died at exactly 20:00 elapsed. The
      per-repo budget must be a fraction of the run budget, not half of it. **FIX LANDED
      2026-09-04 (~17:55 UTC, ahead of the 18:23 UTC scheduled run):** the coverage phase is
      now internally bounded — `COVERAGE_BUDGET_S` (default/env 600s = half the run budget),
      per-repo timeouts drawn from the phase budget's REMAINDER (`max(30, remaining)`),
      budget-exhaustion keeps cached rows, every repo logs its duration, and the cache write
      MERGES instead of REPLACES (pre-fix, any repo that did not complete was silently dropped
      from the coverage table — a second, latent data-loss defect found in the same function).
      Also removed `riir-unity` from both mirrored REPOS lists (`gen_dashboard.py` +
      `self_evolve_audit.sh`) — the repo moved to `obsolete/` in the 2026-09-04 owner move.
      Validation: `py_compile` + `bash -n` clean; smoke at `COVERAGE_BUDGET_S=1` exercises the
      exhaustion branch and returns all 10 cached rows merged. End-to-end verdict = the next
      scheduled 18:23 UTC run (this file's checkbox flips on its PASS).
      **VERDICT PASS 2026-09-03T18:23 UTC (run of record):** `regen OK` at 18:33:36 (10.5 min
      inside the 1200s budget; the 09-01/09-02 runs died at exactly 20:00), per-repo duration
      lines present (katgpt-rs 156.1s, riir-ai 244.7s, riir-train 164.9s under sibling-agent
      load), the **budget-exhaustion branch fired in production** — `riir-clippy: coverage
      budget exhausted (600s phase), keeping cached` — and the run CONTINUED to completion
      instead of dying, with `coverage=10` proving the merge-not-replace path (the skipped
      repo's cached row survived). verify_dom PASS → artifacts committed (`9bc1aec`, FF to
      main) → deployed → **verified: deployment landed 18:34:10Z** → housekeeping worker
      tests PASS → `=== done ===`. First fresh dashboard artifact since 09-01 03:44. The
      pre-run de-risk (00:52 +0700, creds-less, `--out /tmp`) had already validated the
      quiet-box path (10/10 repos, 111s total, EXIT=0).

## Rules that now hold, and the ones that do not

Holds: `billable = max(0, PERIOD_ACCOUNT_total − included)`, `usd = ceil(billable / block) × price`.
Verified against every line Cloudflare charged.

Does **not** hold, and is labelled rather than believed: the **billing period start**. Neither
credential in this repo can read it — `/billing/*`, `/subscriptions`, the KV namespace list and
`/workers/scripts` all answer `10000 Authentication error` for `CF_API_TOKEN` *and* the wrangler
OAuth token, which carry analytics GraphQL only. Bounded by arithmetic to
**2026-08-15 .. 2026-08-26**, and every day in that bound yields the same reconciliation because
no usage row falls between 08-15 and 08-25 — so the pin is currently **unfalsifiable**. The card
prints `day PINNED`; override with `CF_BILLING_ANNIVERSARY_DAY`.

Two silent-zero hazards found in the instruments, both of the class this workspace keeps
re-finding: the container and KV GraphQL datasets **reject** a window wider than 4w4d and return
**zero rows** rather than an error the caller notices (a 45d ask reads identically to an idle
account — `collect_kv` clamps to 30d), and the wrangler OAuth token **expired 2026-09-01T01:30Z**,
so any path relying on the OAuth fallback instead of `.env`'s `CF_API_TOKEN` collects nothing and
says so only in a log line.
