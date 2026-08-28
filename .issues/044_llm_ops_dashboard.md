# Issue 044 — ops dashboard: one URL tracking LLM spend + healer adoption + coverage

**Filed:** 2026-08-26
**Status:** OPEN — T1+T8+T11 SHIPPED; T2/T3/T6/T9/T10 SHIPPED 2026-08-27 (live-deployed, see §2026-08-27 session); T4 mechanism SHIPPED (paging inert until owner secrets); T7 workflow SHIPPED (inert until repo var + secrets); T5 blocker REWORDED 2026-08-28 — Proposal 005 Phase 1 landed 2026-08-27 (T1.5 dapps `84dd049` / T1.6 clippy `0a7cfd8`), so ledger-row KAT denomination awaits the deferred Worker/DO HTTP ledger shell (phase-2 defer / Plan 031 Phase B gate / phase-3 owner gate); ledger rows DO carry a `kat` field today, present-and-null on all 19 rows (the earlier "NO `kat` field at all (grep-verified)" claim was WRONG — corrected 2026-08-28 by re-parsing the file rather than grepping it; what is missing is a non-null DENOMINATION, not the field). **T12 SHIPPED 2026-08-28** — the "all remaining work is owner-gated activation" claim was also FALSE: the per-day neuron series was silently WRONG (see T12). Remaining work is now genuinely owner-gated activation (T4 paging / T5 ledger shell / T7 nightly)
**Repo:** riir-clippy
**URL:** https://riir-ops-dashboard.foxfox.workers.dev/

## The problem

Operational truth about the LLM/healer stack is scattered across five
incompatible surfaces, none of them visual:

| truth | where it lives | shape |
|---|---|---|
| LLM spend + incidents | `.benchmarks/llm_cost_ledger.jsonl` | JSONL, append-only |
| healer quality | `.benchmarks/score_history.jsonl` | JSONL per score-bench run |
| heal adoption (verdicts/tiers) | per-repo `.heal/trajectories.kv` stores | neuron-db KV (binary) |
| heal adoption (cadence) | git logs, 11 repos | word-boundary grep |
| daily neurons (billing truth) | CF GraphQL `aiInferenceAdaptiveGroups` | live API, short retention |
| unhealed remainder | healer dry-run per repo | not persisted anywhere |

The owner asked for: **one URL + one self-contained HTML (embedded CSS)
showing graphs of everything — usage, spending, training, ... — and a
fill-in of what is missing for prod grade.**

## T1 — v1 SHIPPED (2026-08-26)

- `scripts/gen_dashboard.py` — one-command generator, python3 stdlib only.
  All six sources above, each degrading gracefully. `--sweep` re-measures
  coverage (healer dry-run per repo; cached in
  `.benchmarks/dashboard_coverage.json` otherwise).
- `cloudflare/dashboard/` — `wrangler deploy` config (`riir-ops-dashboard`,
  assets-only) + `public/index.html` (the single generated file).
- Deployed + verified: charts render (node DOM-stub runtime check), 27 KiB,
  zero external deps, `noindex,nofollow`, no secrets (generator asserts the
  OAuth token never enters the HTML).

### What the first generation already caught

- **CAP EXCEEDED 2026-08-26: 10,329.78 / 10,000 neurons (103.3%)** — the
  GLM arms + llama reconciliation pushed the account day over the free cap.
  On the Free plan new generations hard-fail until 00:00 UTC. This is
  exactly the class of event the dashboard exists to surface *before* it
  bites a run.
- Coverage snapshot (fresh `--sweep`, 2026-08-26): riir-ai 336f/2619e ·
  riir-train 272f/2340e · game-sdk 45f/341e · katgpt-rs 15f/33e ·
  chain 11f/29e · clippy 5f/5e · ndb 2f/4e · mmorpg 1f/2e ·
  viewbridge/dapps 0.
- Trajectory stores exist in 9/11 repos (game-sdk's 8,117 rows are
  pre-adoption imports — Heuristic tier; the adoption-era repos are
  Certified).

## CF cleanup executed same session (context)

Leftovers found + removed (owner ask: "any active on cloudflare leftover?"):

- `riir-mmorpg-examples-authority-authoritycontainer` — Container,
  **1 live instance billing since 2026-08-14** (Bench 012 e2e deploy, never
  torn down). DELETED via `wrangler containers delete`.
- `riir-mmorpg-examples-alarm-jitter-test` + `riir-mmorpg-examples-ws-catchup-test`
  — Issue 030 experiment workers. DELETED.
- KEPT: `riir-mmorpg-examples` (the intentional static game deploy).
- Not touched, not this stack: `seal-poc-container` (seal POC, 2026-05-02),
  `agent-board-worker` (2026-08-24, unknown owner), ~25 personal workers
  predating riir.

## Prod-grade backlog (the "what you miss" fill-in)

- [x] **T2 auth gating** — SHIPPED 2026-08-27: `cloudflare/dashboard/src/worker.js`
      token gate deployed in front of the assets (`run_worker_first = true`,
      declared `ASSETS` binding). Fails OPEN (`x-dashboard-auth: open`)
      until the one-time `npx wrangler secret put DASHBOARD_TOKEN` (from
      `cloudflare/dashboard/`); then `?token=…` exchanges for a 30d HttpOnly
      cookie, wrong/absent credentials → 401. Constant-time-ish compare.
- [x] **T3 auto-refresh** — SHIPPED 2026-08-27 (live-data half): the HTML
      fetches the housekeeping worker `/latest` (CORS-enabled) on load →
      `live · cron HH:MM` badge, live cap-alert when generation-time
      analytics were empty. Full regen stays nightly/manual (T7).
- [-] **T4 budget alerting** — MECHANISM SHIPPED 2026-08-27 (housekeeping
      worker, deployed): hourly neurons vs cap thresholds 50/90/100% +
      optional monthly container budget, KV-dedupe `alert:<date>:<level>`,
      Discord-shape webhook POST. Paging INERT until the owner secrets
      `CF_API_TOKEN` + `ALERT_WEBHOOK_URL` (optional `MONTHLY_BUDGET_USD`);
      without the webhook, crossings still land in KV + `/status`
      `alerts_today`. Today's 103.3% event would have paged at all three
      levels once live.
- [-] **T5 KAT rail** — PARTIAL 2026-08-26 (Issue 045); blocker REWORDED
      2026-08-28: the housekeeping Worker records the daily mint anchor
      (`neurons_today`, the mint function's input per Proposal 005
      §Economics — a policy, not a formula). Proposal 005 Phase 1 landed
      2026-08-27 (T1.5/T1.6), so the blocker is now the deferred
      Worker/DO HTTP ledger shell (phase-2 defer / Plan 031 Phase B gate
      / phase-3 owner gate); ledger rows carry NO `kat` field at all
      today — CORRECTED 2026-08-28: the field IS present on all 19 rows,
      uniformly `null`. The producers (`kat_drain.rs`, `fixer_regate.rs`)
      already emit it; only the VALUE awaits that shell. The prior
      "grep-verified" claim came from grepping for a populated pattern and
      reading zero hits as an absent field — a parse, not a grep, settles
      field presence.
- [x] **T6 training/GPU axis** — SHIPPED 2026-08-27: collector
      (`collect_training()`) + card live (recent-runs table + tok/s curve;
      placeholder until rows); the convention landed in riir-train
      `c536b47a` (`.benchmarks/runs.README.md` + `runs.example.jsonl`).
      Rows appear as runs complete.
- [-] **T7 coverage auto-measure** — WORKFLOW SHIPPED 2026-08-27
      (`.github/workflows/ops_dashboard.yml`): nightly 18:23 UTC `--sweep`
      regen + commit `[skip ci]` + conditional wrangler deploy; guarded on
      repo var `OPS_DASHBOARD_ENABLED=true` + secrets `GH_PAT` /
      `CF_API_TOKEN` / `CLOUDFLARE_API_TOKEN` (owner activation — inert by
      default, no nightly failure spam). The generator gained
      `RIIR_GIT_ROOT` + `CF_API_TOKEN` env overrides so CI needs no path
      shims.
- [x] **T8 fixer quality history** — SHIPPED 2026-08-27 (commit `50da11a`):
      `.benchmarks/fixer_quality_history.jsonl` — one append-only row per arm
      run (arms A/B via `run_cf` + arm C via `run_daemon`): keep_oracle/
      keep_strict/vetoed/divergent/regressions + the reachable-miss pair +
      EM/parse/merge axes + tokens + latency. The dashboard's fixer-quality
      card reads this file (live 2026-08-27; placeholder until first rows).
- [x] **T9 heartbeat/staleness** — SHIPPED 2026-08-27: header badge goes
      `crit` (`housekeeping stale · last run …`) when the worker's `/latest`
      heartbeat is older than 2h, `static snapshot` when unreachable —
      live now (heartbeat rows exist even without CF_API_TOKEN).
- [x] **T10 incident timeline strip** — SHIPPED 2026-08-27: renders ALL
      ledger incidents above the session log (the old ">3" gate dropped —
      rendering now is strictly better); today it shows the single qwen
      7.8k row.
- [x] **T11 container axis** — SHIPPED 2026-08-26 (Issue 045): container
      sleep/awake/usage graphs (hourly 48h / daily 14d / weekly 4w +
      per-app 30d table + est-cost KPIs) from the CF GraphQL container
      datasets, deployed live.
- [x] **T12 per-day attribution + reconciliation guard** — SHIPPED 2026-08-28
      (§T12): rows were bucketed by APPEND time, not the usage window they
      cover, so the per-day neuron series was wrong on 4 days (phantom 2025
      bucket; cap-exceeded day under-reported by 1,118 n; 2026-08-27 read
      0.00). New optional `covers_unix` field + `eff_ts` in the generator;
      the ledger now reconciles with server billing truth to 0.003% and
      reproduces this issue's 10,329.78 headline from the committed archive.
      Plus a ledger↔analytics reconciliation card so it cannot regress
      silently.

## T12 — per-day attribution bug + reconciliation guard (SHIPPED 2026-08-28)

**The dashboard was reporting the wrong neurons-per-day, on the very day it
exists to police.** Found by decoding `ts_unix` on every ledger row instead of
trusting the rendered chart.

Two defect instances, one root cause — **rows were bucketed by the time they
were APPENDED, not the usage window they describe:**

| rows | stamped | actually covers | evidence |
|---|---|---|---|
| 0 (`A-smoke`), 2 (`external`) | 2025-09-29 | 2026-08-26 00:55Z / 01:10Z | their own notes name the windows; CF GraphQL retains ~1 day so a GraphQL-sourced row cannot predate commit `dac4b71` (2026-08-26 01:42Z) by 331 days |
| 17 (glm settlement), 18 (llama settlement) | 2026-08-28 12:02Z | 2026-08-26 04:00Z / 2026-08-27 17:00Z | their own notes name the settled buckets |

Effect on the per-day series — a phantom 2025 bucket, the cap-exceeded day
under-reported by 1,118 neurons, and a day of spend credited to a day that
spent nothing:

| account day | before | after | CF analytics (server truth) |
|---|---|---|---|
| 2025-09-29 | 531.14 | — (phantom, gone) | n/a |
| 2026-08-26 | 9,211.60 | **10,330.10** | 10,329.78 |
| 2026-08-27 | 0.00 | **542.54** | 542.54 |
| 2026-08-28 | 1,129.90 | — (nothing spent) | n/a |

The fix makes the client ledger reconcile with server billing truth to
**0.003%** (residual +0.32 is 2dp rounding in the committed 587.36 figure) and
independently reproduces the **10,329.78** cap-exceeded figure this issue
headlines — which had been reachable only from live analytics, never from the
committed archive.

**The fix (append-only respected):**

- New optional ledger field **`covers_unix`** — the usage window a row
  describes. `ts_unix` keeps its honest meaning (append time). Applied
  surgically by string insertion, not re-serialization: **15/19 lines are
  byte-identical**, the 4 targeted rows changed additively only (asserted
  key-by-key; notes appended, never replaced; no measured value touched).
  A first attempt re-serialized via `json.dumps` and silently re-spaced 13
  untouched archive lines — reverted and redone.
- `gen_dashboard.py` derives `eff_ts = covers_unix or ts_unix` once in
  `load_ledger()` and every time-axis consumer (sort, `per_day`, incidents,
  rows table) uses it. Settlement rows show their covered time with a `*`
  and the append time in a tooltip.
- `per_day` chart keys are now date-**sorted** rather than insertion-ordered
  (the `MM-DD` labels also drop the year, so a cross-year bucket was
  unlabelable as stale).
- Producers need no change: `kat_drain.rs` / `fixer_regate.rs` write live
  rows where append time *is* the usage time. `covers_unix` is for
  hand-appended settlement/backfill rows.

**New: ledger ↔ analytics reconciliation card** (so this class cannot regress
silently). Per account day it shows ledger n vs analytics n and a verdict:
`RECONCILED` (within 0.5%, floor 1 neuron), `DRIFT · N% unreconciled`,
`analytics only · unrecorded session` (neurons burned outside the ledger — the
`external` row's original condition), or a dim `ledger only · outside
analytics retention`. Verified across 5 rendered scenarios: reconciled, injected
drift, analytics-only day, **no-analytics (raises no false alarm — today's
token-unset state)**, and empty-both.

Deployed + live-verified byte-identical (version `6b8c8a10`). The standing honest
gap is unchanged and now *visible*: per-run `server` fields read 0.0 under
analytics lag, and CF GraphQL exposes no `model` dimension to this token, so
per-model server truth still needs a manual settlement row — the card now names
the days that need one.

## 2026-08-27 session (T2/T3/T6/T9/T10 shipped live; T4 mechanism + T7 workflow)

- **Deployed:** dashboard Worker now runs the token gate in front of the
  assets (`x-dashboard-auth: open` until the secret is set); housekeeping
  Worker gained CORS read endpoints + budget alerting + extended `/status`
  (version `ab79bd20`). Dashboard HTML regenerated (41,964 B): live badge,
  training card, fixer-quality card, incident strip, refreshed
  what's-MISSING list (shipped items removed; owner activation remains).
- **riir-train `c536b47a`** — the `runs.jsonl` convention (T6 producer side).
- **Verified live:** dashboard 200 + all new sections present; CORS headers
  on `/latest`; `/status` → `{cap: 10000, thresholds: [0.5,0.9,1],
  has_webhook, has_monthly_budget, alerts_today}`.
- **One-time owner steps now pending (each fails soft until done):**

      cd cloudflare/housekeeping && npx wrangler secret put CF_API_TOKEN        # Analytics:Read — collector + alerting + CI analytics
      cd cloudflare/housekeeping && npx wrangler secret put ALERT_WEBHOOK_URL   # Discord-shape webhook — paging (T4)
      cd cloudflare/housekeeping && npx wrangler secret put MONTHLY_BUDGET_USD  # optional monthly container budget
      cd cloudflare/dashboard   && npx wrangler secret put DASHBOARD_TOKEN      # close the auth gate (T2)
      GitHub: repo variable OPS_DASHBOARD_ENABLED=true + secrets GH_PAT / CF_API_TOKEN / CLOUDFLARE_API_TOKEN   # nightly T7

- **Generator env overrides (for CI):** `RIIR_GIT_ROOT`, `CF_API_TOKEN`.

## Notes

- CF GraphQL `aiInferenceAdaptiveGroups` served only ~1 day of rows on this
  account (free-plan retention). The committed ledger is the archive —
  keep appending, never rewrite. GraphQL field: `sum.totalNeurons` (the
  `model` dimension is not exposed to this token; per-model split comes
  from ledger client rows).
- Regenerate + redeploy:
  `python3 scripts/gen_dashboard.py && (cd cloudflare/dashboard && npx wrangler deploy)`
- **2026-08-28:** housekeeping worker mint status strings refreshed to
  `pending-worker-shell` (+ `mint_note`/`note`) and redeployed (version
  `f93ae507-4612-4037-8ad4-e6ce75d345e6`); live-verified on the
  04:00:06 UTC tick (`/status` + `/latest` return the new strings).
  Live health at check time: `token_set: false` (heartbeat mode — the
  `CF_API_TOKEN` owner secret is still unset), `cap: 10000`,
  `has_webhook: false`, `has_monthly_budget: false`, `alerts_today: 0`.
- **Ledger schema (2026-08-28):** `ts_unix` = when the row was appended.
  `covers_unix` (optional) = the usage window it describes; set it on any
  hand-appended settlement/backfill row. Consumers must bucket by
  `covers_unix or ts_unix` — never `ts_unix` alone.
- **Lesson (T12):** the dashboard rendered a confident, wrong per-day series
  for two days. A chart cannot audit its own inputs — decode the raw
  timestamps. And "grep-verified" is not verification of field presence
  (the T5 `kat` claim): parse the file.
- Numbering: allocated 044 (highwater 43 → 44).
