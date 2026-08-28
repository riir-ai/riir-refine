# Issue 051 — groq-first LLM routing: the free tier absorbs the reasoning window before any CF neuron burns

**Filed:** 2026-08-28
**Status:** OPEN — T1-T6 LANDED; **T9 LANDED 2026-08-28: the free leg is no longer gated behind the paid leg's credentials** (the fix below — `LlmRouter::from_env` built `CfClient` FIRST, so the owner's groq-only `.env` made `kat_drain` exit 3 without spending one free token: the exact opposite of this issue's purpose). Live-proven on the real API in a groq-only env (`live_router_groq_only`: served by groq, 21+2 tok, **0 neurons**, pricing basis `@cf/zai-org/glm-4.7-flash` intact). Tests 16/16 groq + 14/14 kat_drain + 1051/1051 lib, clippy clean. T7 (first live window on real backlog) STILL blocked — but on BACKLOG only (measured 0 eligible of 41 store rows), no longer on credentials; fixer_regate stays CF-pinned BY DESIGN
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
- **Either leg may be absent; at least ONE must be armed** (revised by T9;
  the original text below was the defect). `GROQ_API_KEY` absent →
  byte-identical to pre-051 (pure-CF window). CF creds absent → a groq-only
  window: the ladder has no fallback, so an exhausted free tier is terminal
  for the window (`FREE_TIER_EXHAUSTED`, a clean stop that still writes the
  ledger row) rather than a crash. Both absent → refused at construction.
  A CF leg that is present-but-misconfigured still fails hard.

  > ~~The fallback is mandatory: `CfClient::from_env` still fails hard
  > without CF creds — free tier alone is not the contract.~~ **WRONG, and
  > load-bearing wrong** — this gated the FREE leg behind the PAID leg's
  > credentials, which made the whole issue a no-op on the owner's actual
  > `.env`. Kept struck-through as the lesson: "the fallback must exist" is
  > a statement about the LADDER's shape, not a licence to require the
  > fallback's credentials in order to use the primary.

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
- [x] T9 — **the mandatory-CF defect** (found + fixed 2026-08-28). The
      design below made the CF leg mandatory, so `LlmRouter::from_env` ran
      `CfClient::from_env(cf_model)?` FIRST and the free leg never armed
      without PAID credentials. The owner's `.env` holds exactly one key
      (`GROQ_API_KEY`) → `kat_drain` exited 3 (FATAL) before any free token
      was spent. The headline of this issue was unreachable in the only
      environment it was written for. Fixed:
      - `cf: Option<CfClient>`; a new pure `LlmRouter::arm_plan(groq, cf)`
        decides the legs (env-free → unit-testable without `set_var`).
        At least ONE leg must be armed; both absent → a loud refusal naming
        both vars.
      - **Unarmed ≠ misconfigured**: CF creds PRESENT but the client refuses
        (unpriced model, …) still fails HARD — a config bug must never
        degrade silently to groq-only.
      - `cf_model: String` resolved even when CF is unarmed (the model that
        WOULD have served), so `usd_avoided`'s pricing basis and the ledger
        row's `model` field survive a groq-only window. `model_id()` keeps
        its `&str` signature — zero call-site churn.
      - New terminal verdict `FREE_TIER_EXHAUSTED = "free-tier exhausted"`,
        classified `StopDay` by `kat_drain::classify_error` alongside the USD
        cap: a groq-only window whose quota runs out stops CLEANLY and still
        writes its ledger row. Deliberately a DISTINCT prefix from
        `"LLM daily cap reached"` — conflating them would misreport the cause.
      - A transient groq **transport** failure with no CF armed declines only
        that span (groq stays enabled, next span retries); only the windowed
        **rate limit** is terminal.
      - `stats.fallbacks` now increments only when an attempt ACTUALLY
        reached CF — on a groq-only window `fallbacks_to_cf` stays 0 instead
        of implying paid traffic that never happened.
      - `cf()` → `Option<&CfClient>`; the drain's spend-state / server-meter
        / spend-summary reads are Option-aware and print an explicit
        `$0.0000 metered spend (groq-only window)` line.
- [-] T7 — first LIVE window on Groq (quota absorption measured on real
      backlog) — **no longer credential-blocked (T9)**; now waiting on a
      non-empty backlog. Measured 2026-08-28 via `--dry-run`: 0 eligible of
      41 store rows (eligibility = `fail_count > 0 AND tier != Withdrawn`),
      so the router is never even constructed. Standing pre-flights, both
      passing on the real API: `live_smoke` (T5) and `live_router_groq_only`
      (T9, proves the groq-only ladder end-to-end).
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
- Numbering: allocated 051 (highwater 50 → 51). T9 is a task number inside
  this issue, not a new issue number.
- **The T9 lesson, generalized:** a fallback leg's CREDENTIALS must never
  gate the primary leg's use. The tell was in the issue's own status line —
  "T7 deferred … needs CF creds" — a free-tier feature that cannot run
  without paid credentials is not deferred, it is broken. Read a "deferred,
  needs X" status as a claim to verify against the environment, not a fact.
