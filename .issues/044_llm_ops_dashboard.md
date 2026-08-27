# Issue 044 — ops dashboard: one URL tracking LLM spend + healer adoption + coverage

**Filed:** 2026-08-26
**Status:** OPEN — T1+T8+T11 SHIPPED; T2/T3/T6/T9/T10 SHIPPED 2026-08-27 (live-deployed, see §2026-08-27 session); T4 mechanism SHIPPED (paging inert until owner secrets); T7 workflow SHIPPED (inert until repo var + secrets); T5 blocked on Proposal 005 Phase 1 — all remaining work is owner-gated activation
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
- [-] **T5 KAT rail** — PARTIAL 2026-08-26 (Issue 045): the housekeeping
      Worker records the daily mint anchor (`neurons_today`, the mint
      function's input per Proposal 005 §Economics); ledger rows still
      carry `kat: null` until the Phase 1 rail denominates them.
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
- Numbering: allocated 044 (highwater 43 → 44).
