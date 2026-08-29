# Issue 044 — ops dashboard: one URL tracking LLM spend + healer adoption + coverage

**Filed:** 2026-08-26
**Status:** OPEN — T1+T8+T11 SHIPPED; T2/T3/T6/T9/T10 SHIPPED 2026-08-27 (live-deployed, see §2026-08-27 session); T4 collector LIVE 2026-08-29 (the `CF_API_TOKEN` worker secret is set via the Issue-045 rollout — neurons + containers collecting hourly, cap-crossing detection LIVE with `alerts_today` in `/status`; only the paging PUSH awaits `ALERT_WEBHOOK_URL` + optional `MONTHLY_BUDGET_USD`); T7 workflow SHIPPED (inert until repo var + secrets); T5 blocker REWORDED 2026-08-28 — Proposal 005 Phase 1 landed 2026-08-27 (T1.5 dapps `84dd049` / T1.6 clippy `0a7cfd8`), so ledger-row KAT denomination awaits the deferred Worker/DO HTTP ledger shell (phase-2 defer / Plan 031 Phase B gate / phase-3 owner gate); ledger rows DO carry a `kat` field today, present-and-null on all 19 rows (the earlier "NO `kat` field at all (grep-verified)" claim was WRONG — corrected 2026-08-28 by re-parsing the file rather than grepping it; what is missing is a non-null DENOMINATION, not the field). **T12 SHIPPED 2026-08-28** — the "all remaining work is owner-gated activation" claim was also FALSE: the per-day neuron series was silently WRONG (see T12). **T13 SHIPPED 2026-08-28** (security hardening + T7 re-homed). Remaining: T4 paging (needs `CF_API_TOKEN`, unmintable from wrangler OAuth — no `user_tokens:edit` scope), T5 (deferred Worker/DO ledger shell), T7 activation (one `launchctl bootstrap`, files shipped), and **the auth gate is OPEN by owner decision 2026-08-28** — see T13. **T16 SHIPPED 2026-08-29** (`0a8e1ec`): the nightly logged `WARN deploy failed` then `=== done ===` and exited 0 — a failed deploy left production stale while launchd saw success. Now exits 2, quotes wrangler verbatim instead of misdiagnosing it as stale OAuth, and names the stale-prod consequence. **T16.1 same day CORRECTS T16's diagnosis**: the OAuth session already holds `workers_scripts (write)` (the earlier `whoami | tail -12` truncated the list above those rows), production was NEVER stale (deploy landed 17:04:29Z, bit-identical to the committed artifact), and the real cause was an ambient analytics-scoped `CLOUDFLARE_API_TOKEN` SHADOWING OAuth on a MANUAL run — impossible under launchd. Deploy now runs under `env -u CLOUDFLARE_API_TOKEN …`; control-verified. **T4 is NOT on the deploy critical path**
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

- [-] **T2 auth gating** — mechanism SHIPPED 2026-08-27, but the gate is **OPEN** (owner decision 2026-08-28, §T13) — reopened from [x] to [-] because a deployed-but-unset gate is not a closed gate: `cloudflare/dashboard/src/worker.js`
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
      **Blocker sharpened 2026-08-28 (T13): `ALERT_WEBHOOK_URL` alone cannot
      activate this** — the collector needs `CF_API_TOKEN` too, and that token
      **cannot be minted from this machine.** `wrangler whoami` shows OAuth
      scopes `account (read)` / `user (read)` with NO `user_tokens:edit`, so no
      agent or script here can create it; it must be made by hand: CF dashboard
      → My Profile → API Tokens → Create Token → Custom → permission
      `Account · Account Analytics · Read` (account
      `7e11517c4dd4f6e9cede7da9b60d66eb`), then
      `cd cloudflare/housekeeping && npx wrangler secret put CF_API_TOKEN`.
      A long-lived token is unavoidable for the Worker because the wrangler
      OAuth token rotates hourly.
      The asymmetry that makes this survivable: the *generator* reads analytics
      fine (it uses that rotating OAuth token from
      `~/Library/Preferences/.wrangler/config/default.toml`), which is why the
      dashboard's cap chart is live and DID surface the 103.3% event. What is
      missing is only *push* paging — *pull* (look at the dashboard) works, and
      the T12 reconciliation card now cross-checks it against billing truth.
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
- [-] **T13 security hardening + T7 re-homed** — SHIPPED 2026-08-28 (§T13):
      payload audit falsified T2's fail-open rationale (19 raw ledger rows,
      10 private repo names + lint debt, container costs). Deployed CSP,
      `private, no-store`, `no-referrer`, nosniff, DENY, noindex, 405 on
      non-GET, length-independent token compare, and a LOUD
      `x-dashboard-auth: OPEN-INSECURE` marker. Investigated
      `cf-cache-status: HIT` → NOT a bypass (`run_worker_first = true`
      invokes the Worker on all requests). **Gate left OPEN by owner
      decision** — deferred, not done. T7 re-homed to the M3
      (`scripts/ops_nightly.sh` + plist, zero secrets, ~5 min vs ~30-50);
      files shipped, schedule not installed.

## T13 — security hardening + T7 re-homed to the M3 (SHIPPED 2026-08-28)

### The fail-open rationale was falsified by a payload audit

T2 shipped the gate FAIL-OPEN on the written rationale that "the HTML carries
only aggregated numbers". Auditing the embedded `ds` JSON instead of trusting
that sentence: it carries **19 FULL ledger rows** with internal issue/proposal
references, **10 PRIVATE repo names with per-repo unhealed-lint counts**, per-repo
rule-adoption stats + top rule names, container app ids and cost, and
fixer-quality internals. That is a private engineering profile of the whole
workspace, on an unauthenticated public URL.

**OWNER DECISION 2026-08-28: leave the gate OPEN.** Recorded, not silently
worked around. The exposure stands until `DASHBOARD_TOKEN` is set:

    cd cloudflare/dashboard && npx wrangler secret put DASHBOARD_TOKEN

### What DID ship (deployed, version `82c5032d`)

Everything that hardens the surface without risking a lockout ahead of that
secret:

- **`Cache-Control: private, no-store` + `Vary: Cookie`.** The assets pipeline
  served the private payload as `public, max-age=0, must-revalidate` and CF was
  observed returning `cf-cache-status: HIT`. Investigated rather than assumed:
  `run_worker_first = true` means **all** requests invoke the Worker (assets are
  never served directly), and CF's own docs note `CF-Cache-Status` is
  "probabilistically added for privacy reasons — don't rely on it for critical
  routing logic". So it is the internal asset layer behind the auth check, **not**
  an edge-cache bypass. The override still matters for browser/intermediary
  caches holding a private body.
- **CSP** `default-src 'none'` with `connect-src` pinned to the single origin the
  HTML actually calls (the housekeeping `/latest` badge), plus `base-uri 'none'`,
  `form-action 'none'`, `frame-ancestors 'none'`. The HTML is fully self-contained
  (inline style/script, inline SVG, zero external assets), so everything else is
  denied outright. Origin is a declared var `HK_ORIGIN`, not a buried literal.
- **`Referrer-Policy: no-referrer`** — the one-time `?token=…` must never ride a
  Referer header outbound.
- **Length-INDEPENDENT token compare.** The prior `if (A.length !== B.length)
  return false` short-circuited and leaked token length via response timing; the
  length difference now folds into the same XOR accumulator.
- `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
  `X-Robots-Tag: noindex, nofollow, noarchive`, `Cross-Origin-Opener-Policy`,
  `Permissions-Policy`, and **405** on non-GET/HEAD. Applied on EVERY branch
  (401/503/405 included) so the posture never depends on which branch answered.
- **The unconfigured branch is now LOUD:** `x-dashboard-auth: OPEN-INSECURE` plus
  an `x-dashboard-warning` naming the exposure — the old value was a mild `open`
  that read as normal to a curl or uptime check.
- **`UNCONFIGURED_POLICY` var** (`serve` default | `deny`). Default `serve` ONLY so
  deploying this hardening could not take the dashboard offline ahead of the
  secret. Target state: set the secret, then flip to `deny` so an accidentally
  deleted secret fails safe instead of silently republishing the payload.

### The gated branches are unit-gated, because production cannot test them

With no secret set, the 401 / token-exchange / cookie-auth paths are
unreachable live — so setting `DASHBOARD_TOKEN` would be their FIRST-EVER
execution. `cloudflare/dashboard/verify_auth.mjs` (`node
cloudflare/dashboard/verify_auth.mjs`) exercises all of them against a stub
env: 10 scenarios covering unconfigured serve/deny, 401 with no payload leak
and no secret echo, 5 wrong-token shapes (empty / short / long / truncated /
case-flipped), wrong cookie, the 301 exchange (token stripped from `Location`,
unrelated query params preserved, `HttpOnly`+`Secure`+`SameSite=Lax`+`Path`+
`Max-Age` all present, empty body), multi-cookie jar parsing, 405 on
POST/PUT/DELETE across all three env shapes, and the hardening headers on
every branch.

It immediately earned its keep by catching a real latent bug: the
`x-dashboard-warning` value contained an **em-dash**, and HTTP header values
are ByteStrings. workerd tolerated it (the live header rendered fine), but
Node/undici throws `Cannot convert argument to a ByteString`, so the header was
unportable and the branch untestable off-workerd. Now ASCII-only.

**NOT touched:** `cloudflare/housekeeping/` — a sibling agent has uncommitted
read-side hardening in flight there (CORS allowlist replacing `*`, `READ_TOKEN`
gating `/hours`/`/days`, KV fan-out caps, minimised `/latest`). Live still shows
`Access-Control-Allow-Origin: *` and an unauthenticated `/hours`, so that work is
theirs to land; duplicating it would have collided.

### T7 re-homed: the M3 is primary, CI is the fallback

The CI design was measured against the local one and loses on every axis:

| | M3 (`scripts/ops_nightly.sh`) | GitHub Actions |
|---|---|---|
| secrets needed | **zero** (repos present, cargo-heal installed, wrangler OAuth works) | 3 (`GH_PAT`, `CF_API_TOKEN`, `CLOUDFLARE_API_TOKEN`) |
| wall clock | ~5 min | ~30-50 min (11 × depth-2000 clones + release build of cargo-heal) |
| adoption card | populated | **structurally always empty** — `.heal/trajectories.kv` is gitignored local state |
| `develop` races | none (no auto-push on conflict) | nightly bot commit races concurrent agents |

Shipped `scripts/ops_nightly.sh` + `scripts/com.riir.ops-nightly.plist`
(`plutil -lint` clean). Prod-grade properties: atomic `mkdir` lock with a 6 h
stale-reclaim (a killed run must not wedge the schedule forever); a **render gate**
— refuses to deploy or commit an HTML that fails `verify_dom.mjs`; commits **only
the 2 artifact paths by explicit name**, never `git add -A` (sibling WIP lives in
this repo continuously); best-effort push that never force-pushes or rebases and
just retries next run on a race; niced/low-IO/Background so it yields to
interactive agent work; self-truncating log. **Files shipped, schedule NOT
installed** (owner action):

    cp scripts/com.riir.ops-nightly.plist ~/Library/LaunchAgents/
    launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.riir.ops-nightly.plist

Also **de-staled the CI workflow**, whose header still warned that two generator
dependencies were unlanded. Both landed (`RIIR_GIT_ROOT` at
`gen_dashboard.py:71`, `CF_API_TOKEN` at `:569`), so two dead shims are gone:
a `sudo mkdir -p /Users/katopz` + `sudo ln -sfn` fabricating a macOS path on a
Linux runner as root, and a step writing the CF token to disk as a fake wrangler
config (the generator reads it straight from env, so the token no longer touches
the runner's filesystem).

## T16 — the nightly reported success while production stayed stale (SHIPPED 2026-08-29)

Commit `0a8e1ec`. Found by reading the 00:02 run's own log rather than its
exit status: it printed `WARN deploy failed`, then `=== done ===`, then
exited 0. Deploying is the POINT of this runner -- steps 1-3 only refresh and
commit; until the deploy lands, production serves the PREVIOUS dashboard. So
WARN-only meant a failed deploy left prod stale indefinitely while every
surface launchd can observe reported success. Same class as T12 (a confident,
wrong per-day series) and T15 (a silently dropped domain): the green surface
was never the thing that was wrong.

Three changes:
- **Record the failure in the exit status** -- exit 2, distinct from the
  exit-1 aborts earlier in the script, which fail BEFORE anything is
  committed or deployed. `com.riir.ops-nightly.plist` has no `KeepAlive`, so
  a non-zero exit cannot respawn-loop.
- **Quote wrangler verbatim instead of guessing.** The old hint blamed
  "wrangler OAuth may need refresh". On the observed failure wrangler had
  authenticated FINE via `CLOUDFLARE_API_TOKEN` and was refused for MISSING
  SCOPE -- so the hint named the wrong fix, which costs more than no hint.
- **Say plainly that prod is serving the previous build.**

Verified on five arms by stubbing `npx` against the extracted block:
`SUCCESS` exit 0 | `SCOPE-ERR` exit 2 (verbatim cause) | `TIMEOUT(124)` exit 2
| `SILENT-FAIL` (no output at all) exit 2 | `NO_DEPLOY` exit 0 with `npx`
never invoked.

**The failure arms caught a defect in the first version of the fix.** With
`set -o pipefail`, a no-match `grep` returns 1, so under `set -e` the run
aborted with NO log line at all -- silently WORSE than the WARN-only original,
and precisely on the likeliest unattended failure (a bare `timeout 600` kill
prints nothing the error pattern matches). Hence the load-bearing `|| true`.
Testing only the success arm would have shipped a silent-death regression.

### T16.1 — the cause above was WRONG, and so was the consequence (2026-08-29)

The paragraph that stood here claimed the deploy needed an owner-minted token
with `Workers Scripts:Edit`, and that production was left stale. **Both are
false**, and the second commit of T16 corrects them. Measured, not reasoned:

- **The OAuth session already holds `workers_scripts (write)`.** The earlier
  read took `wrangler whoami | tail -12`, which cut the list off ABOVE the
  `workers_*` rows -- a truncated listing used as an absence proof, the same
  mistake as the T15 grep. Re-read in full, `workers` / `workers_kv` /
  `workers_routes` / `workers_scripts` are all `(write)`.
- **The real cause is credential SHADOWING, not a missing scope.** Step 0b
  loads `.env` for the CF ANALYTICS credential; wrangler independently
  auto-adopts any ambient `CLOUDFLARE_API_TOKEN` and PREFERS it over OAuth.
  The 00:02 run was hand-started from a shell that exported an
  analytics-scoped token, so wrangler authenticated as that token and was
  refused for missing scope. Note `.env` does NOT set `CLOUDFLARE_API_TOKEN`
  (only `CF_API_TOKEN`, which wrangler ignores) and nothing in the repo maps
  one to the other -- so under launchd, whose environment is spartan, this
  failure could not have occurred at all. It was an artifact of MANUAL
  invocation.
- **Production was never left stale.** A deploy landed 2026-08-28T17:04:29Z,
  75 s after the failure. The deployed bytes are BIT-IDENTICAL to the
  committed artifact (`sha256 b863f933…`, fetched and compared, not assumed).

**Fix:** the deploy step now runs under
`env -u CLOUDFLARE_API_TOKEN -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL`.
Two credentials, two jobs -- analytics reads use the token, deploys use OAuth
-- and a manual run now behaves exactly like the launchd run, so a hand-run
can no longer fail in a way the scheduled run would not.

Verified with a CONTROL, so the test discriminates: with the guard removed and
`CLOUDFLARE_API_TOKEN` exported, the deploy reproduces the 00:02 failure
(exit 2, shadowing-token error); with the guard in place the same arm exits 0.
Full matrix re-run: `SUCCESS` 0 | `SCOPE-ERR` 2 | `TIMEOUT(124)` 2 |
`SILENT-FAIL` 2 | `NO_DEPLOY` 0. shellcheck clean.

### T16.3 — the least-privilege deploy path was 100% broken (2026-08-29)

The `CF_DEPLOY_API_TOKEN` branch added on top of T16.1 could never deploy:

```
_w_env=(CLOUDFLARE_API_TOKEN="${CF_DEPLOY_API_TOKEN}" env -u CF_API_TOKEN ...)
"${_w_env[@]}" timeout 600 npx ... deploy
```

A leading `VAR=value` is an assignment prefix only when the SHELL PARSES it.
After `"${_w_env[@]}"` expansion it is just the first WORD, so the shell looked
for a command literally named `CLOUDFLARE_API_TOKEN=<token>` and every deploy
died with `command not found`. `.env` HAS `CF_DEPLOY_API_TOKEN` set (045
rollout), so this was **100% of deploys on the preferred, least-privilege
path** -- the OAuth fallback, which works, was unreachable whenever the safer
credential was present.

Fix: `env` first, assignment as an ARGUMENT to it (options before assignments):
`_w_env=(env -u CF_API_TOKEN -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL "CLOUDFLARE_API_TOKEN=${CF_DEPLOY_API_TOKEN}")`.

Found by re-running T16's arm matrix against the REFACTORED script rather than
trusting the earlier pass -- the refactor changed the exact invocation the
safety properties hang off, so the prior verification was stale. Proven by
having the stub print what it can actually see: token arm
`TOKEN_SEEN=deploytok CF_API_TOKEN=<stripped>`, OAuth arm `TOKEN_SEEN=<none>`
with an ambient shadowing token present (T16.1 still holds).

Second, smaller fix: the error-extraction pattern did not match `not found` /
`no such`, so this very failure logged `<no error line matched>` -- the loudest
possible break reported with the least possible information. Pattern widened;
the `command-not-found` arm now quotes `bash: wrangler: command not found`.

Full matrix re-verified on the fixed script: `scope-err` (both credential arms)
exit 2 verbatim | `timeout(124)` exit 2 | `command-not-found` exit 2 WITH cause
| `uploaded-not-landed` exit 2 DEGRADED | `list-garbage` exit 0 INCONCLUSIVE
(no false alarm) | `NO_DEPLOY` exit 0. shellcheck clean.

### T16.2 — "deployed" now means production RECEIVED it (2026-08-29)

`wrangler deploy` exiting 0 means the API accepted the upload, not that the
deployment landed. This issue has been bitten three times by a green surface
over a wrong fact (T12 wrong per-day series, T15 dropped domain, T16 failed
deploy reported as success), so step 4a now checks independently: stamp `_t0`
before the upload, then confirm the newest entry in `wrangler deployments
list --json` is newer than it.

Deliberately NOT a fetch-and-byte-compare of the dashboard HTML. That works
only while the auth gate is OPEN (T13) — the moment someone runs `wrangler
secret put DASHBOARD_TOKEN`, `/` stops serving the payload and a body check
would go RED on a perfectly correct deploy. The deployment list is
posture-independent, so this gate survives closing the gate.

**Three outcomes, not two.** "Could not measure" is reported separately from
"measured stale": a network blip on the list read logs `NOTE ... INCONCLUSIVE`
and leaves the exit status alone. Conflating the two would turn a correct
deploy into a nightly exit 2, and a gate that cries wolf gets ignored and then
disabled.

Verified on four arms: landed-after-`_t0` -> `verified`, exit 0 | newest older
than `_t0` -> exit 2 + WARN | list unreadable -> NOTE, exit 0 | list returns
non-JSON -> NOTE, exit 0. The parser was then run against the REAL API, not
just the stub, and returned `2026-08-28T17:04:29.645465Z` — the actual
deployment identified in T16.1. shellcheck clean.

**Auth posture, verified live (not assumed):** `/` serves 200 unauthenticated
and self-announces it — `x-dashboard-auth: OPEN-INSECURE` plus an
`x-dashboard-warning` naming T13. Confirmed to be the origin's real behaviour,
not an edge-cache artifact (a cache-buster request returns `cf-cache-status:
MISS` with the same 200). The payload carries operational telemetry (model
names, neuron counts, groq usage) and NO credentials: account id, `Bearer`,
`api_key` and local paths all grep to zero. Other headers are strict (CSP,
`no-store`, `noindex`, `frame-ancestors 'none'`, `vary: Cookie`). This is the
documented owner decision, one `wrangler secret put DASHBOARD_TOKEN` from
closed — recorded here because "the gate is open" had never been confirmed
against the deployment itself.

**Consequence for T4:** minting `CF_API_TOKEN` is still wanted for ANALYTICS,
but it is **NOT** on the critical path for deploying — that path is open today
via OAuth. T7's schedule install is likewise unblocked on this axis.

## T15 — the domain list was duplicated three times; docker was about to vanish (SHIPPED 2026-08-28)

Found while verifying T14's newly hard-gated `verify_dom.mjs` — if a gate now
blocks the unattended deploy, it had better be right about what "broken" means.

**The list of corpus domains existed in THREE places**, two of them as a bare
count:

1. `gen_dashboard.py:CORPUS_DOMAINS` — the source of truth (mines the counts),
2. a hand-typed JS literal in `HTML_TEMPLATE` (`const DOMAINS = [...4 names]`),
3. `verify_dom.mjs` pins: `ys.length === 4`, `ys === [11,37,63,89]`, and three
   more `=== 4` spark assertions.

A sibling had already added **docker** to (1), so `CORPUS_DOMAINS` has five
entries while the renderer's literal still had four. Consequences, both live:

- **Silently dropped data.** The generator builds each daily point from ALL of
  `CORPUS_DOMAINS` (`gen_dashboard.py:382`), so the next regen mines docker's
  rule counts and the 4-name literal then never renders them. The
  coverage-growth card would under-report by a whole domain with nothing
  failing anywhere — the same "green surface, wrong number" class as 045 T8.
- **A booby-trapped fix.** Correcting the literal to five would then trip
  `verify_dom.mjs`'s `=== 4` pins → gate FAIL → and since T14 made this gate
  hard-block the nightly, the deploy would stop with "the dashboard is broken"
  when it was merely *bigger*.

**Fixed on both sides.** The generator now emits `const DOMAINS = __DOMAINS__`
from `CORPUS_DOMAINS` via the existing `__DATA__` replacement idiom — the list
is never re-typed, so adding a domain is a one-line change in one place. The
gate **derives** the expected count from the page's own `DOMAINS` list instead
of pinning it, preserving the invariant that actually catches defects (one rect
per series, on the 11+26k lattice, inside the viewBox — the "y=83 concat"
class).

**Verified positive AND negative — the negative is the point:**

| case | expected | result |
|---|---|---|
| current 4-domain page | PASS | ✓ `corpus 4 domains ys [11,37,63,89]` |
| simulated 5-domain page (docker in data + list) | PASS | ✓ `corpus 5 domains ys [11,37,63,89,115]` |
| row lattice broken (`rowH` 26→25) | **FAIL** | ✓ `corpusBar rect ys = 11,36,61,86` + per-chart off-lattice |
| DOMAINS claims 5, only 4 render | **FAIL** | ✓ `corpusBar rect count 4 != 5 domains` |

The 5-domain simulation also caught an **incomplete first fix**: deriving only
the corpus-bar count left the three spark `=== 4` pins in place, and the bar
alone reported a clean pass. Running the gate against the grown page — rather
than only the current one — is what surfaced them.

## T14 — pre-install audit of the unattended nightly runner (SHIPPED 2026-08-28)

`scripts/ops_nightly.sh` is about to be installed as a **launchd job that
git-commits and `wrangler deploy`s with nobody watching**. T7 asserted its
prod-grade properties in prose; this task VERIFIED them instead, before the
schedule exists. Three defects, all fixed:

**1. The stale-lock reclaim could run two instances concurrently (correctness).**
The lock was reclaimed on AGE alone (`>6 h → abandoned`). But the plist's own
comment says *"The laptop sleeps"* — a suspended run exceeds any wall-clock
threshold while remaining perfectly alive, so the next night's run would
reclaim a LIVE holder's lock and proceed to `git commit` + `wrangler deploy`
concurrently with it. Age cannot answer the question being asked; **liveness**
can. The lock now carries the holder's pid and is reclaimed only when
`kill -0` says the holder is gone; age survives strictly as the fallback for a
lock with no readable pid (pre-upgrade lock, or a torn write), so a genuinely
killed run still cannot wedge the schedule forever.

**2. The EXIT trap released whatever lock existed, not its own (cascade).**
`trap 'rmdir "${LOCK}"' EXIT` — a run that was correctly reclaimed as dead, or
one that overran and got displaced, would delete its SUCCESSOR's lock on the
way out, admitting a third concurrent run. `release_lock` now compares the pid
file against `$$` and releases only a lock it still owns. Same defect as (1),
one level up.

**3. `npx --yes wrangler` was unpinned remote code holding deploy rights (SECURITY).**
There is no `package.json` in `cloudflare/dashboard`, so `npx --yes wrangler`
resolves the **latest** wrangler from the npm registry **at run time** and
executes it unattended with this account's Cloudflare deploy OAuth. That is a
scheduled, unsupervised, unpinned fetch-and-execute with publish rights — a
supply-chain exposure whose blast radius is the live dashboard. Now pinned
(`WRANGLER_VERSION="4.127.1"`, verified to resolve and to match the version
already in use, so behaviour is unchanged today) with the bump-deliberately
rule stated at the constant.

**4. Repo hygiene (`.gitignore`).** The runner writes `.benchmarks/ops_nightly.log`
and the lock dir INTO the repo, both untracked. That is not just `git status`
noise: sibling agents run bulk `git add -A`, and an untracked file is exactly
what gets swept into someone else's commit. Both are now ignored.

**5. THE HEADLINE — under launchd the runner would have silently never deployed
(availability + a gate that stops gating).** `node` and `npx` are **nvm**-managed
and nvm initialises in `~/.zshrc`, which `/bin/bash -lc` (the plist's own
invocation) NEVER sources — `~/.bash_profile` does not exist here and
`~/.profile` says nothing about nvm. Measured under a launchd-realistic login
shell: `python3` resolves (`/usr/bin/python3`) but **node and npx are NOT
FOUND**. The consequence was not a loud failure but a quiet one — the run would
have:

1. regenerated fine (python3 present),
2. **skipped the render gate** → `WARN verify_dom.mjs or node absent — deploying unverified`,
3. committed artifacts nothing had verified,
4. **failed every deploy** → `WARN deploy failed (wrangler OAuth may need refresh)` — a
   MISLEADING diagnosis; the cause is a missing binary, not OAuth,
5. skipped the housekeeping test gate silently,

every one of them a WARN. The schedule would have looked healthy for as long as
it ran and **never deployed anything**. This is exactly the failure mode the T8
hour-key bug had: a surface that reads green while the mechanism is dead.

Fixed two ways, because either alone is insufficient: the runner now **resolves**
the toolchain (sources `nvm.sh`, else prepends the highest installed
`versions/node/*/bin`), and then **refuses to run degraded** — a preflight over
`python3 node npx git` that names the missing tool and exits 1. The render gate's
`&& command -v node` conjunct is gone (dead weight that could mask a regression
now that the preflight guarantees node), and a missing `verify_dom.mjs` is now a
hard FAIL instead of `deploying unverified`. **A gate that silently stops gating
is worse than no gate, because the log still reads green.**

**6. A defect in fix (5) itself, caught by testing the negative path.** The first
cut resolved nvm with a bare `find` over `${NVM_DIR}/versions/node`. Under the
script's own `set -euo pipefail`, a MISSING nvm dir makes `find` exit non-zero,
`pipefail` propagates it through the command substitution, and the script dies
at that assignment — **exit 1 with no log line**, the very silent death the
section exists to prevent. Now the directory is guarded and the pipeline's
status neutralised, so only the preflight decides the verdict. *Verifying that
the failure path FAILS LOUDLY is what caught this; a success-path-only test
would have passed.*

**Verified empirically, not by reading** — the lock functions were extracted
into an isolated harness (nothing triggers the real 20-minute sweep) and all
six behaviours exercised:

| case | expected | result |
|---|---|---|
| free lock | acquire | ✓ |
| holder ALIVE, lock backdated to 2020 | **SKIP** (old age-only logic would have reclaimed) | ✓ |
| holder DEAD | reclaim (schedule must not wedge) | ✓ |
| no readable pid, fresh | SKIP | ✓ |
| no readable pid, backdated | reclaim (age fallback intact) | ✓ |
| release someone else's lock | **refused** | ✓ |
| release our own lock | released | ✓ |

And the toolchain paths, run under a launchd-realistic environment
(`env -i HOME=… PATH=/usr/bin:/bin:/usr/sbin:/sbin /bin/bash -lc`):

| case | expected | result |
|---|---|---|
| nvm present | resolve node, proceed | ✓ `node v24.10.0`, exit 0 |
| nvm absent | name the missing tool, exit 1 | ✓ `FAIL required tool 'node' not on PATH`, exit 1 |
| *(pre-fix baseline)* | — | gate would be SKIPPED → "deploying unverified" |

`bash -n` + `shellcheck` clean. The schedule itself is still NOT installed —
that remains the owner action documented in T7, and it is now worth installing:
before this pass it would have run degraded and silently deployed nothing.

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
