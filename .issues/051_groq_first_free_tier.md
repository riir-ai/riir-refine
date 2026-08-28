# Issue 051 — groq-first LLM routing: the free tier absorbs the reasoning window before any CF neuron burns

**Filed:** 2026-08-28
**Status:** OPEN — T1-T6 LANDED; **T9 LANDED 2026-08-28: the free leg is no longer gated behind the paid leg's credentials** (the fix below — `LlmRouter::from_env` built `CfClient` FIRST, so the owner's groq-only `.env` made `kat_drain` exit 3 without spending one free token: the exact opposite of this issue's purpose). Live-proven on the real API in a groq-only env (`live_router_groq_only`: served by groq, 21+2 tok, **0 neurons**, pricing basis `@cf/zai-org/glm-4.7-flash` intact). Tests 16/16 groq + 14/14 kat_drain + 1051/1051 lib, clippy clean. **T13 (ledger attribution — a groq-only window no longer credits the CF model with samples it never served) + T12 (the same hardening applied to the CF fallback leg + bounded response-body dumps) + T10 (prod-grade hardening: split connect timeout, single-allocation auth header, token scrub on server-controlled strings) + T11 (`--preflight`: ladder check on an empty backlog, zero metered cost by construction) LANDED same day.** **T14 (backlog census -- "queue empty" no longer hides a withheld backlog) LANDED 2026-08-28**, and its first real-store reading (`51 clean * 0 eligible * 0 withdrawn`) instrument-VERIFIES that T7 (first live window on real backlog) is STILL blocked — but on BACKLOG only (measured 0 eligible of 41 store rows), no longer on credentials; fixer_regate stays CF-pinned BY DESIGN
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
- [x] T10 — **prod-grade hardening of the free leg** (2026-08-28):
      - **`CONNECT_TIMEOUT` (10s) separate from `CALL_TIMEOUT` (3 min).**
        Availability defect: one overall timeout cannot tell a blackholed TCP
        connect from a slow reasoning model, so a dead network stalled the
        FULL 3 min — per span. A `--limit 25` window could hang ~75 minutes.
        Connecting is fast or broken; nothing in between. Pinned by
        `connect_timeout_is_bounded_well_under_the_call_timeout` (also
        asserts 25 × connect ≤ 5 min, so a future edit can't reopen it).
      - **`auth: String` pre-formatted once.** Perf: drops one `format!`
        allocation per call. Security: the secret occupies ONE heap location
        for the client's lifetime instead of being copied into a fresh
        allocation per request and dropped un-zeroed — it can't be smeared
        across freed heap pages.
      - **`GroqClient::scrub`** on every server-controlled error string (the
        HTTP error body, ureq transport errors, the not-JSON path). Defense
        in depth: the API doesn't echo `Authorization` today and all error
        paths are hand-audited, but a leaked key in
        `.benchmarks/llm_cost_ledger.jsonl` would be a COMMITTED secret. Cost
        is one `contains` on a path already doing network I/O; the clean path
        early-returns without allocating.
      - Tests: token absent from `scrub` output / `Debug` / the header
        contract (3 new, built without `from_env` so they never touch the
        real key and stay parallel-safe).
- [x] T11 — **`kat_drain --preflight`** (2026-08-28): arms the ladder, proves
      the free leg SERVES, exits — BEFORE the TrajStore opens, so it works on
      an empty backlog. Closes a real ops gap: the daily window returns at
      "queue empty" before the router is ever constructed, so on a quiet day
      an operator had NO way to validate credentials or the ladder — which is
      why T7's pre-flight existed only as an `#[ignore]`d unit test.
      **Zero metered cost by construction**: the probe runs ONLY when the
      free leg is armed. A pure-CF preflight is deliberately REFUSED (it
      would burn ~50-120 neurons that this path does not ledger, while
      reporting nothing the window's own ledgered smoke doesn't). No path may
      spend behind the ledger's back. Exits 0 / 3 (no provider) / 5 (probe
      failed). Verified live both ways: groq-only → `probe OK via groq ·
      21+2 tok · 0.0 neurons · 632ms · ladder READY`; neither armed → exit 3
      with the both-vars refusal.
- [x] T12 — **the same hardening on the FALLBACK leg** (`CfClient`,
      2026-08-28). T10 audited the free leg and found a defect class the
      PAID leg shared — Issue 051's ladder depends on both, so fixing only
      the half this issue introduced would leave the window hanging on the
      other half:
      - `CONNECT_TIMEOUT` (10s) split out of `CALL_TIMEOUT` (3 min) — the
        identical blackholed-connect stall, same test pinning it.
      - `auth: String` pre-formatted once, replacing a per-call `format!` at
        BOTH request sites (generate + the usage GraphQL probe).
      - `CfClient::scrub` on all six server-controlled error strings.
      - **`clip_payload` (512-byte ceiling)** on the two paths that dumped a
        FULL response body into an error string (`"usage GraphQL shape
        unexpected: {v}"`, `"CF response had no result: {v}"`). An unbounded
        dump floods archived cron logs and echoes server content back out of
        the process; a bounded prefix keeps every diagnostic these messages
        are actually read for. Char-boundary safe (a naive byte slice would
        panic on multi-byte UTF-8) — pinned by a test.
      Error PREFIXES are unchanged, so `kat_drain::classify_error` and the
      pinned `"CF request failed"` test needed zero edits.
- [x] T13 — **ledger attribution for a groq-only window** (2026-08-28). The
      defect T9 CREATED, found by following through on my own change: the row's
      `model` field is the CF **pricing basis** (it prices `usd_avoided` and is
      resolved even when the CF leg was never armed), NOT a claim that CF
      served anything. So a groq-only row reads `model: @cf/…glm-4.7-flash`
      with `client` tokens 0, `usd_est` 0.0, `cap` null, `server` null —
      **exactly the `glm-4.7-flash 0` mis-render the owner hand-reconciled in
      `7bdb710`**, except T9 turned it from an anomaly into a ROUTINE row
      shape. Worse than the cosmetic $0: `per_model[…]["n"] += n`
      credited the CF model with every sample groq actually served.
      - **Producer** (`kat_drain`): new top-level `"served_by"` field —
        `groq` / `cloudflare` / `mixed` / `none` — from a pure, unit-tested
        `served_by(groq_served, cf_tokens, smoke_neurons, server_neurons)`.
        A CF-served SMOKE counts as CF usage even when groq served every
        span: those neurons are real, and labelling them free would hide
        metered spend behind the free-tier label — the one thing the ledger
        must never do (pinned by `a_cf_smoke_is_not_attributed_to_the_free_tier`).
      - **Consumer** (`scripts/gen_dashboard.py`): reattribute such a row to
        a `groq:<model>` bucket. Deliberately NARROW — rows that must stay on
        the CF model are all excluded by construction: pure-CF/mixed windows
        (client tokens > 0), settlement/backfill rows (server neurons > 0,
        client 0 — the `7bdb710` shape), and a CF-smoke + groq window
        (`drain.smoke_neurons` > 0). `served_by` is authoritative; the token
        heuristic is the fallback that keeps pre-T13 rows correct.
      - **Verified zero-regression**: the dashboard's entire `ledger` section
        is byte-identical on the real 19-row ledger (no legacy row carries
        groq activity, so the branch never fires), plus a 6-shape synthetic
        harness asserting groq-only + legacy-groq reattribute while pure-CF,
        mixed, settlement, and no-providers rows stay put with neurons and
        client tokens intact.
- [x] T14 — **backlog census: "queue empty" was diagnostically opaque**
      (2026-08-28). `build_backlog` applies TWO independent exclusions
      (`fail_count == 0`, and `tier == Withdrawn`) and the drain reported only
      their SUM — `0 queued (store 51 row(s))`. That one line cannot
      distinguish the two operational states it collapses:
      - **healthy** — every row is clean, nothing to heal; and
      - **STALLED** — real recorded failures exist, but every one sits in the
        absorbing `Withdrawn` tier (3 strikes) and will NEVER be queued again.
      Both print `queue empty — nothing to drain` and **exit 0**, so the daily
      window looks green either way while a pile of permanent failures grows
      invisibly. Same defect class as 044-T12 (the hourly archive collapsing
      24 rows into 3) and this issue's own T13 (a groq-only row crediting the
      CF model): **a green surface hiding real state.**
      - Pure `census(&[TrajRowDoc]) -> BacklogCensus { total, clean,
        withdrawn, eligible }` — a fold using the SAME two predicates in the
        SAME order as `build_backlog` (a census that could disagree with the
        queue it explains would be worse than none; pinned by
        `census_eligible_matches_what_build_backlog_queues`).
      - Reported on EVERY window, not only empty ones — a non-empty queue can
        coexist with a growing withdrawn pile — plus an explicit `WARN` line
        when `withdrawn > 0`.
      - Pure `empty_queue_reason` separates the THREE causes an empty queue
        actually has: **empty store** (no heal run has recorded anything → run
        the healer), **healthy** (all clean → nothing to do), **STALL** (all
        failures withheld → needs a fresh pattern). Three different operator
        actions that used to share one string.
      - 5 new tests (bin unit tests 16 → **24/24**): bucket disjointness +
        exhaustiveness, census-vs-queue agreement, and one per reason —
        including the negative assertions that a stall never reads `healthy`
        and an empty store never reads `healthy`.
      - **`fail_count == 0` wins over the tier**: a clean-but-`Withdrawn` row
        is `clean`, not `withdrawn` — there is nothing to heal, so counting it
        as a stall would manufacture a false alarm (pinned).
      - **Measured on the real store the same day** — see T7: `51 clean · 0
        eligible · 0 withdrawn`. The census CONFIRMED T7's parked status
        rather than overturning it; the difference is that the claim is now
        *instrument-verified on every window* instead of assumed, and the day
        it becomes a stall the operator is told so.
      - **Follow-through fix, same day — the defect T14 itself created.**
        Auditing my own change (the T13 discipline): the caller's branch is
        `prepared_count == 0`, and `prepared_count == queued -
        modelless_served`, so it is reached FIVE ways — not the three the
        first cut modelled. Two were misreported:
        - **A fully-modelless window** (real backlog, every queued span
          absorbed by the L4 pre-pass — zero LLM cost, the BEST possible
          outcome) printed `BUG: the census and build_backlog disagree`. The
          message screamed *defect* at the mechanism working exactly as the
          L4 contract designs it, and would have sent an operator hunting a
          non-existent bug on the healthiest window the drain can have.
        - **`--limit 0`** (a deliberate operator choice) hit the same
          false `BUG`.
        `empty_queue_reason` now takes `queued` + `modelless_served` — without
        both, "the seam healed everything for free" is genuinely
        indistinguishable from "the backlog is empty". The `BUG` branch was
        narrowed to the only genuinely unexplained shape (eligible > 0,
        limit > 0, queued == 0) and is still pinned firing by its own test, so
        the fix did not simply delete the alarm. 3 new tests (21 -> **24/24**),
        including the negative assertion that a fully-modelless window never
        reads `healthy` either — the backlog was REAL, just served for free.
        *Lesson, same shape as T9 -> T13: a diagnostic added to remove
        ambiguity introduced its own, because it modelled the states it was
        designed for rather than every state its branch can actually be
        reached in.*
      - Deliberately NOT surfaced to the ledger/dashboard: `withdrawn == 0`
        today, so a chart would plot a constant zero while risking the 19-row
        ledger and the T13 attribution logic. Escalate when the census
        measures a non-zero stall — not before (evidence-first, no
        premature surface).
- [-] T7 — first LIVE window on Groq (quota absorption measured on real
      backlog) — **no longer credential-blocked (T9)**; now waiting on a
      non-empty backlog. Measured 2026-08-28 via `--dry-run`: 0 eligible of
      41 store rows (eligibility = `fail_count > 0 AND tier != Withdrawn`),
      so the router is never even constructed. **Re-measured 2026-08-28 with the T14 census: the store has GROWN 41 -> 51 rows and is still `51 clean * 0 eligible * 0 withdrawn`** -- so the backlog is genuinely ABSENT, not withheld in the absorbing tier (the one alternative reading that would have made this a stall to fix rather than a wait). T7's blocker is now instrument-verified, not assumed. Standing pre-flights, both
      passing on the real API: `live_smoke` (T5) and `live_router_groq_only`
      (T9, proves the groq-only ladder end-to-end) — plus `--preflight`
      (T11) as the operator-facing equivalent, runnable any day.
      **Not forceable, deliberately:** backlog rows come from REAL heal
      failures (`TrajStore` `fail_count > 0`), so seeding synthetic failures
      to open the window would manufacture the very measurement T7 exists to
      take. It waits for organic accumulation.
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
