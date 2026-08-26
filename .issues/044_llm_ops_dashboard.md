# Issue 044 — ops dashboard: one URL tracking LLM spend + healer adoption + coverage

**Filed:** 2026-08-26
**Status:** OPEN — T1 (v1) + T11 SHIPPED; T3/T5/T9 partial (Issue 045); T8 SHIPPED 2026-08-27 (`50da11a`); the remaining backlog is owner-gated (T2 auth choice, T4 webhook target, T6 riir-train convention, T7 scheduling infra) — see per-task notes
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

- [ ] **T2 auth gating** — the URL is unauthenticated workers.dev. Add a
      token query gate (tiny Worker script in front of assets) or
      Cloudflare Access. The HTML carries only aggregated numbers today,
      which is why v1 shipped without it.
- [-] **T3 auto-refresh** — PARTIAL 2026-08-26 ([Issue 045](045_container_cost_housekeeping.md)):
      the `riir-kat-housekeeping` Worker (hourly cron) now archives
      container cost + mint-anchor rows to KV continuously with read
      endpoints (`/hours`, `/days`) — the live-data half of auto-refresh
      exists; wiring the HTML to fetch it (and scheduling the generator)
      remains open.
- [ ] **T4 budget alerting** — notify (Discord/email/webhook) at 50/90/100%
      of the 10k/day cap + a monthly USD budget. The bar turns red today;
      nobody is paged. Today's 103.3% event would have paged twice
      (qwen 77.8% + GLM arms).
- [-] **T5 KAT rail** — PARTIAL 2026-08-26 (Issue 045): the housekeeping
      Worker records the daily mint anchor (`neurons_today`, the mint
      function's input per Proposal 005 §Economics); ledger rows still
      carry `kat: null` until the Phase 1 rail denominates them.
- [ ] **T6 training/GPU axis** — riir-train runs (plan, steps, tok/s,
      GPU-hours, M3 vs 4090) are not ingested. Needs a run-log convention
      in riir-train (e.g. `.benchmarks/runs.jsonl`) + a collector here.
      This is the biggest visible gap vs the owner's "training" ask.
- [ ] **T7 coverage auto-measure** — schedule `--sweep` nightly so the
      unhealed table stays honest (manual today).
- [x] **T8 fixer quality history** — SHIPPED 2026-08-27 (commit `50da11a`):
      `.benchmarks/fixer_quality_history.jsonl` — one append-only row per arm
      run (arms A/B via `run_cf` + arm C via `run_daemon`): keep_oracle/
      keep_strict/vetoed/divergent/regressions + the reachable-miss pair +
      EM/parse/merge axes + tokens + latency. The dashboard's fixer-quality
      axis reads this file.
- [-] **T9 heartbeat/staleness** — PARTIAL 2026-08-26 (Issue 045): the
      housekeeping Worker's hourly heartbeat rows (KV `latest`) prove a
      schedule is alive; the dashboard-side staleness banner + alert
      remain open.
- [ ] **T10 incident timeline strip** — render ledger incident rows as an
      annotated timeline once there are >3 (today: 1, the qwen 7.8k row).
- [x] **T11 container axis** — SHIPPED 2026-08-26 (Issue 045): container
      sleep/awake/usage graphs (hourly 48h / daily 14d / weekly 4w +
      per-app 30d table + est-cost KPIs) from the CF GraphQL container
      datasets, deployed live.

## Notes

- CF GraphQL `aiInferenceAdaptiveGroups` served only ~1 day of rows on this
  account (free-plan retention). The committed ledger is the archive —
  keep appending, never rewrite. GraphQL field: `sum.totalNeurons` (the
  `model` dimension is not exposed to this token; per-model split comes
  from ledger client rows).
- Regenerate + redeploy:
  `python3 scripts/gen_dashboard.py && (cd cloudflare/dashboard && npx wrangler deploy)`
- Numbering: allocated 044 (highwater 43 → 44).
