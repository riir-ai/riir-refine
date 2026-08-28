# Issue 051 — groq-first LLM routing: the free tier absorbs the reasoning window before any CF neuron burns

**Filed:** 2026-08-28
**Status:** OPEN — T1-T6 LANDED same day (client + router + kat_drain wiring + ledger `providers` rows + dashboard free-tier card + tests); **live smoke PASSED on the real API** (`qwen/qwen3.8-27b` on groq replied exact "OK", usage 21+2 tok metered, quota headers 999 req/7,947 tok); dashboard card live-deployed (version `40bea5b3`); T7 first live drain window deferred to the owner's next daily run (needs CF creds + non-empty backlog); fixer_regate stays CF-pinned BY DESIGN
**Repo:** riir-clippy
**URL:** https://riir-ops-dashboard.foxfox.workers.dev/

## The problem

The daily trainer window (`kat_drain`, Plan 031 A3) and every other LLM call
in this repo go STRAIGHT to Cloudflare Workers AI — a metered free tier
(10k neurons/UTC-day, the Issue 044 cap-exceeded incident proved the ceiling
is real and hard). The owner provisioned a **Groq** API key (`.env`,
gitignored): an OpenAI-compatible chat API with a free tier of its own.

Owner direction: **call Groq first for reasoning; fall back to Cloudflare
when the free limit hits.** The free tier absorbs the cost; CF remains the
metered fallback its USD cap (Issue 046) already guards. And the usage must
land on the ops dashboard so what the free tier absorbed is visible.

## Design (three-test-rule clean: this is dev tooling, zero game coupling)

**Provider ladder, per attempt:**

```text
attempt → Groq (free; GROQ_API_KEY) ── ok ──→ done (usd = 0)
              │ 429 / quota headers exhausted
              │   → RateLimited: DISABLE groq for the rest of the window
              │     (limits are windowed; hammering a 429 is waste)
              │ other transport error
              │   → retry groq next attempt (transient), fall back this attempt
              ▼
          Cloudflare CfClient (unchanged: USD cap + smoke + ledger)
```

- **Overflow (`finish_reason=length`, null content) on Groq does NOT fall
  back** — a budget problem overflows on CF too; falling back would burn
  paid neurons on a deterministic failure. The error surfaces with the same
  `REASONING_OVERFLOW` prefix so the drain's existing budget-escalation
  retry (`2× --max-new`) re-enters the ladder at Groq with the bigger budget.
- **qwen3 mitigation carried over**: `/no_think` suffix for qwen3-family ids
  (the same serving-family soft switch CfClient applies). If Groq's stack
  ignores it, the overflow classification + budget escalation still bound
  the cost (free).
- **GROQ_API_KEY absent → byte-identical to today** (pure-CF window). The
  fallback is mandatory: `CfClient::from_env` still fails hard without CF
  creds — free tier alone is not the contract.

**Accounting honesty (the load-bearing rule):** Groq tokens are FREE — they
must never inflate the CF cost estimate. `ArmSummary` (the `client` ledger
block) stays CF-only; Groq stats ride a new `providers.groq` ledger block.
`usd_avoided` is an ESTIMATE (groq tokens × the CF fallback model's per-M
price) and is labeled as one everywhere it renders.

## Tasks

- [x] T1 — `src/groq.rs`: `GroqClient` (ureq, OpenAI-compat chat completions
      at `https://api.groq.com/openai/v1/chat/completions`; `GROQ_MODEL`
      override, default `qwen/qwen3.8-27b` per the owner's .env probe; token
      NEVER logged/Debug'd — the CfClient rule) + pure parse/classify fns +
      quota-header extraction. Feature `groq_first = ["dep:ureq"]` (no new
      packages — ureq is already the cf_workers_ai dep).
- [x] T2 — `LlmRouter` (same file, gated `all(groq_first, cf_workers_ai)`):
      groq-first ladder above, per-window stats (calls/tokens/rate-limited/
      fallbacks/errors), `RouterGen` carrying `provider` + `usd_est` (0.0 for
      groq). CF errors keep their exact message prefixes so
      `kat_drain::classify_error` needs zero changes.
- [x] T3 — `kat_drain` wiring: smoke + per-span attempts through the router;
      groq cases count in the quality summary but contribute ZERO tokens to
      `client`; `DrainStats` + ledger row gain `providers.groq` +
      `smoke_provider` + `groq_model`; `usd_avoided` estimate in the row.
      Bin required-features: `["cf_workers_ai", "groq_first"]`.
- [x] T4 — dashboard (`scripts/gen_dashboard.py`): ledger summary gains the
      groq absorption axis (reads `providers.groq` per row; legacy rows
      unchanged); new card "free-tier absorption (groq-first)" + 2 header
      KPIs (tokens absorbed / est USD avoided) — regenerated + **live-
      deployed** (version `40bea5b3`, card verified in the served HTML).
- [x] T5 — tests: request-body build (qwen3 `/no_think`, non-qwen clean),
      OpenAI-shape parse + usage + reasoning chars, overflow classification,
      429-vs-5xx classification, quota-header parse, router disable-once
      behavior, ledger `providers.groq` row shape. Live smoke
      (`live_smoke`, `#[ignore]`d) — **PASSED 2026-08-28 on the real API**:
      exact `OK` reply, usage reported (21+2 tok), quota headers read
      (999 req / 7,947 tok remaining).
- [x] T6 — docs: this issue + README feature-matrix row + the kat_drain
      module doc header (the CF-only run instructions updated).
- [-] T7 — first LIVE window on Groq (quota absorption measured on real
      backlog) — deferred to the next daily drain run (needs the owner's CF
      creds for the mandatory fallback leg + a non-empty backlog); the live
      smoke (T5) passed 2026-08-28 and is the standing pre-flight.
- [-] T8 — `fixer_regate` arms stay CF-pinned BY DESIGN (arm rows are
      MODEL comparisons — mixing a free router would confound the comparison
      the arms exist to make). Re-open only with an explicit free-tier arm.

## Notes

- `.env` is gitignored — `GROQ_API_KEY` never enters the repo; the issue,
  code, and errors never print it (the CfClient token rule, inherited).
- The housekeeping worker tracks CF neurons via GraphQL; Groq has no
  server-side meter we can read — the client-metered ledger rows ARE the
  usage record (the ledger-as-archive convention, Issue 044 §Notes).
- Scaling direction (owner): this ladder is the TEMPLATE for more free
  tiers — a provider list + per-provider quota state, not a hardcoded pair.
  Filed as the follow-up shape in T8's re-open clause; do not build the
  general ladder until a second free provider exists (the
  no-premature-abstraction precedent, riir-ai Issue 528).
- Numbering: allocated 051 (highwater 50 → 51).
