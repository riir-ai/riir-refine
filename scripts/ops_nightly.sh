#!/usr/bin/env bash
# ops_nightly.sh — Issue 044 T7 (riir-clippy): nightly ops-dashboard refresh.
#
# Runs ON THE M3, not in CI. Rationale (see .github/workflows/ops_dashboard.yml
# header for the full decision): locally the 11 sibling repos are already
# checked out, cargo-heal is already installed and wrangler OAuth already
# authenticates — so this needs ZERO provisioned secrets, finishes in ~5 min
# instead of ~30-50, and can populate the self-evolve adoption card that a
# fresh CI checkout structurally cannot (`.heal/trajectories.kv` is gitignored
# local state).
#
# Install / uninstall the schedule:
#   launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.riir.ops-nightly.plist
#   launchctl bootout   gui/$(id -u)/com.riir.ops-nightly
# Run once by hand (safe, idempotent):
#   scripts/ops_nightly.sh
# Regenerate without touching git and Cloudflare:
#   OPS_NIGHTLY_NO_COMMIT=1 OPS_NIGHTLY_NO_DEPLOY=1 scripts/ops_nightly.sh
#
# DEPLOY POLICY (2026-08-29): production deploys ship from `main` ONLY —
# `develop` is the working branch. §3b resolves a dedicated `main` worktree
# (default ../.riir-clippy-main, override RIIR_OPS_MAIN_WT), fast-forwards it
# to origin/develop, and the wrangler deploy runs from THERE — never from
# whatever branch this checkout happens to sit on. A diverged main skips the
# deploy loudly instead of shipping a tree main did not vet.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${REPO}/.benchmarks/ops_nightly.log"
LOCK="${REPO}/.benchmarks/.ops_nightly.lock"
# Every path the generator WRITES must be listed here, or the nightly dirties a
# tracked file it never commits. `corpus_history.jsonl` was missing (found
# 2026-08-29 by diffing the working tree after a real run, not by reading the
# generator): it is written on BOTH generator paths, independent of --sweep,
# and a new calendar day always appends a row -- so the schedule would leave it
# permanently dirty. That is not cosmetic for an APPEND-ONLY HISTORY file in a
# repo where sibling agents run bulk `git add -A` and `git checkout`: the row
# either lands in someone else's unrelated commit, or is discarded and that
# day is gone from the series with nothing recording the hole.
# Same class, same day (issue 057): the ledger is written whenever --settle
# appends a settlement row -- without it, a settled day would sit uncommitted
# and a later checkout would turn it back into drift (re-settled as a
# duplicate, or lost entirely once analytics retention expires).
ARTIFACTS=(".benchmarks/dashboard_coverage.json" ".benchmarks/corpus_history.jsonl" ".benchmarks/llm_cost_ledger.jsonl" "cloudflare/dashboard/public/index.html")

# PINNED ON PURPOSE. There is no package.json in cloudflare/dashboard, so a
# bare `npx --yes wrangler` resolves the LATEST wrangler from the npm registry
# at run time and executes it UNATTENDED with this account's Cloudflare deploy
# credentials — unpinned remote code holding publish rights, on a schedule,
# with nobody watching. Bump this deliberately after checking the release;
# never let it float.
WRANGLER_VERSION="4.127.1"

log() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >> "${LOG}"; }

# mkdir is atomic on macOS (no flock in base BSD userland). Two contention
# cases must be told apart, and a wall-clock threshold ALONE cannot do it:
#
#   holder DEAD  (killed run)  — reclaim, or the schedule wedges forever
#   holder ALIVE (just slow)   — respect it
#
# This laptop sleeps — the plist says so in its own comment — so a suspended
# run can exceed ANY age threshold while remaining perfectly alive. The old
# age-only reclaim would then start a SECOND run that git-commits and
# `wrangler deploy`s concurrently with the first. Liveness is the right
# question; age survives only as the fallback for a lock carrying no readable
# pid (a pre-upgrade lock, or a torn write).
acquire_lock() {
  if mkdir "${LOCK}" 2>/dev/null; then
    echo $$ > "${LOCK}/pid"
    return 0
  fi
  local holder=""
  [ -r "${LOCK}/pid" ] && holder="$(cat "${LOCK}/pid" 2>/dev/null || true)"
  case "${holder}" in
    ''|*[!0-9]*)
      if [ -z "$(find "${LOCK}" -maxdepth 0 -mmin +360 2>/dev/null)" ]; then
        log "SKIP lock held (no readable pid, not yet stale)"
        return 1
      fi
      log "WARN lock has no readable pid and is >6h old — reclaiming"
      ;;
    *)
      if kill -0 "${holder}" 2>/dev/null; then
        log "SKIP ops_nightly pid ${holder} is ALIVE and holds the lock"
        return 1
      fi
      log "WARN lock holder pid ${holder} is gone — reclaiming"
      ;;
  esac
  rm -f "${LOCK}/pid" 2>/dev/null || true
  rmdir "${LOCK}" 2>/dev/null || true
  mkdir "${LOCK}" 2>/dev/null || { log "SKIP could not reclaim lock"; return 1; }
  echo $$ > "${LOCK}/pid"
  return 0
}

# Release ONLY a lock we still own. Without the ownership check, a run that
# was (correctly) reclaimed as dead — or one that overran and got displaced —
# would delete its SUCCESSOR's lock on exit, cascading into a third concurrent
# run. That is the same defect as the reclaim, one level up.
release_lock() {
  local holder=""
  [ -r "${LOCK}/pid" ] && holder="$(cat "${LOCK}/pid" 2>/dev/null || true)"
  if [ "${holder}" = "$$" ]; then
    rm -f "${LOCK}/pid" 2>/dev/null || true
    rmdir "${LOCK}" 2>/dev/null || true
  fi
}

acquire_lock || exit 0
# ── Completion sentinel (katgpt-rs Issue 734) ──────────────────────────────
# Measured on macOS `/bin/bash` 3.2.57 — and ONLY there (katgpt-rs Issue
# 735): bash 4.4 through 5.3, dash and busybox ash all preserve the
# status. When 3.2 aborts on an unbound expansion (`set -u`) or an `eval`
# syntax error it enters the EXIT trap with `$?`
# ALREADY 0 — so a cleanup trap whose last command succeeds makes the abort
# exit 0, and everything after it silently did not run while the caller reads
# a PASS. `trap 'rc=$?; cleanup; exit $rc'` does not help: the rc it saves is
# itself 0. Only "did the script reach its own last line?" catches it.
# Every deliberate `exit 1`/`exit 2` below passes through untouched — this arm
# fires only on INCOMPLETE *and* claiming success.
# (`acquire_lock || exit 0` above is deliberate and sits BEFORE the trap, so
# the "another run holds the lock" path is untouched by this.)
NIGHTLY_COMPLETED=0
nightly_cleanup() {
    nightly_st=$?
    release_lock
    if [ "${NIGHTLY_COMPLETED}" != "1" ] && [ "${nightly_st}" = "0" ]; then
        log "FAIL: nightly ABORTED mid-run while reporting success — it never reached its last line; forcing exit 1"
        exit 1
    fi
    exit "${nightly_st}"
}
trap nightly_cleanup EXIT

log "=== start (pid $$) ==="

# ── 0. resolve the toolchain, then REFUSE to run degraded ──────────────────
# launchd hands this job a spartan environment. `node`/`npx` are nvm-managed
# and nvm initialises in ~/.zshrc — which `/bin/bash -lc` NEVER sources (only
# ~/.profile exists here, and it does not mention nvm). Measured under a
# launchd-realistic login shell: python3 resolves (/usr/bin/python3) but node
# and npx are NOT FOUND.
#
# Left unhandled, the consequence is not a loud failure but a QUIET one: the
# run regenerates, skips the render gate ("deploying unverified"), commits
# artifacts nothing verified, and fails the deploy with the misleading
# "wrangler OAuth may need refresh" — every one of them a WARN. The schedule
# would look healthy for as long as it ran and would never deploy anything.
if ! command -v node >/dev/null 2>&1 && [ -s "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]; then
  # shellcheck disable=SC1091
  . "${NVM_DIR:-$HOME/.nvm}/nvm.sh" >/dev/null 2>&1 || true
fi
if ! command -v node >/dev/null 2>&1; then
  # `set -euo pipefail` is live: a bare `find` over a MISSING nvm dir exits
  # non-zero, pipefail propagates it through the substitution, and the script
  # dies right here — exit 1 with NO log line, i.e. the silent death this
  # whole section exists to prevent. Guard the directory, and neutralise the
  # pipeline's status so only the preflight below decides the verdict.
  NVM_VERSIONS="${NVM_DIR:-$HOME/.nvm}/versions/node"
  if [ -d "${NVM_VERSIONS}" ]; then
    NVM_BIN_DIR="$(find "${NVM_VERSIONS}" -maxdepth 2 -type d -name bin 2>/dev/null | sort -V | tail -1 || true)"
    if [ -n "${NVM_BIN_DIR}" ]; then
      PATH="${NVM_BIN_DIR}:${PATH}"
      export PATH
      log "resolved node from nvm: ${NVM_BIN_DIR}"
    fi
  fi
fi

# Same spartan-PATH class as nvm above, found live 2026-09-01: `timeout` is
# GNU coreutils, installed here by homebrew (/opt/homebrew/bin on Apple
# silicon), and launchd's PATH (/usr/bin:/bin:/usr/sbin:/sbin) has no
# homebrew — so every SCHEDULED run since install died at step 1 with
# `timeout: command not found` (exit 127) while every manual run from an
# interactive shell passed, because the interactive PATH carries homebrew.
# Three consecutive nightly failures (08-29/30/31, .benchmarks/ops_nightly.log)
# and zero [skip ci] regen commits in history were the measurement. Resolve
# the brew prefix the same way nvm is resolved; the preflight below then
# makes a genuinely-missing timeout LOUD at step 0 instead of a mid-run 127.
if ! command -v timeout >/dev/null 2>&1; then
  for _brew_bin in /opt/homebrew/bin /usr/local/bin; do
    if [ -x "${_brew_bin}/timeout" ]; then
      PATH="${_brew_bin}:${PATH}"
      export PATH
      log "resolved timeout from ${_brew_bin}"
      break
    fi
  done
fi

# A missing tool must STOP this runner, never downgrade it. A gate that
# silently stops gating is worse than no gate, because the log still reads
# green — which is precisely how the Issue 045 T8 hour-key bug survived two
# days of "live-verified" checks.
for _tool in python3 node npx git timeout; do
  if ! command -v "${_tool}" >/dev/null 2>&1; then
    log "FAIL required tool '${_tool}' not on PATH — refusing to run degraded"
    exit 1
  fi
done
log "toolchain OK (node $(node --version 2>/dev/null), $(command -v npx))"

# ── 0b. credentials: load .env, then say plainly when we are running BLIND ──
# This script never sourced .env, and launchd hands it a spartan environment —
# so `gen_dashboard.py` fell through to its wrangler-OAuth fallback. That
# fallback is STALE BY CONSTRUCTION here: the OAuth access token carries a ~1 h
# TTL and is refreshed only when the wrangler CLI runs, which in this script is
# step 4 — AFTER the step-1 regeneration that needs it. On an unattended 03:00
# run the token has long expired.
#
# The failure is quiet, which is the whole problem: both CF collectors log one
# line and return empty, so the dashboard renders its container and neuron
# sections BLANK — indistinguishable from "nothing ran on the account". That is
# the same class as the Issue 045 T8 hour-key bug cited in step 0: output that
# looks like data.
if [ -f "${REPO}/.env" ]; then
  set -a
  # shellcheck disable=SC1091
  . "${REPO}/.env"
  set +a
  log "loaded ${REPO}/.env"
fi
if [ -n "${CF_API_TOKEN:-}" ]; then
  log "CF credential: CF_API_TOKEN from env"
else
  _WT="${HOME}/Library/Preferences/.wrangler/config/default.toml"
  _EXP="$(sed -n 's/^expiration_time = "\(.*\)"$/\1/p' "${_WT}" 2>/dev/null || true)"
  _NOW="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [ -z "${_EXP}" ]; then
    log "WARN no CF_API_TOKEN and no wrangler OAuth token — CF analytics + container sections will render BLANK"
  elif [ "${_EXP}" \< "${_NOW}" ]; then
    log "WARN no CF_API_TOKEN and the wrangler OAuth token EXPIRED at ${_EXP} (now ${_NOW}) — CF analytics + container sections will render BLANK; set CF_API_TOKEN in ${REPO}/.env"
  else
    log "CF credential: wrangler OAuth, valid until ${_EXP}"
  fi
fi

# ── 1. regenerate (re-measures healer coverage across the sibling repos) ──
# --settle first appends settlement rows for completed account days whose
# ledger attribution drifts from CF GraphQL (issue 057), so the regenerated
# dashboard reflects the settled state in the same run. --sweep shells out to
# cargo-heal per repo. Issue 065 T6: the sweep phase is internally bounded to
# COVERAGE_BUDGET_S (600s here = half this run budget, dynamic per-repo
# timeouts drawn from the remainder) — the other half always stays available
# for settle + CF collections + render. (The unbounded 600s-per-repo form
# burned the whole 1200s on the 09-01/09-02 riir-ai timeouts: exit 124, no
# regen, no deploy, stale dashboard.)
if COVERAGE_BUDGET_S=600 timeout 1200 python3 "${REPO}/scripts/gen_dashboard.py" --sweep --settle >> "${LOG}" 2>&1; then
  log "regen OK"
else
  log "FAIL regen (exit $?) — leaving artifacts untouched, no deploy"
  exit 1
fi

# ── 2. render gate: never deploy or commit an HTML that does not render ──
if [ -f "${REPO}/cloudflare/dashboard/verify_dom.mjs" ]; then
  if timeout 120 node "${REPO}/cloudflare/dashboard/verify_dom.mjs" >> "${LOG}" 2>&1; then
    log "verify_dom PASS"
  else
    log "FAIL verify_dom — refusing to deploy/commit a broken dashboard"
    exit 1
  fi
else
  log "FAIL verify_dom.mjs missing — refusing to deploy an unverified dashboard"
  exit 1
fi

# ── 3. commit ONLY the two artifacts, by explicit path ──
# NEVER `git add -A`: sibling agents work in this repo continuously and a bulk
# add sweeps their WIP into this bot commit.
if [ "${OPS_NIGHTLY_NO_COMMIT:-0}" != "1" ]; then
  if git -C "${REPO}" diff --quiet -- "${ARTIFACTS[@]}"; then
    log "no artifact change — nothing to commit"
  else
    git -C "${REPO}" add -- "${ARTIFACTS[@]}"
    git -C "${REPO}" -c user.name="ops-nightly" \
        -c user.email="ops-nightly@localhost" \
        commit -q -m "chore(ops): nightly dashboard refresh (issue 044 T7; settlement per issue 057)" \
        -- "${ARTIFACTS[@]}" && log "committed artifacts"
    # Push is best-effort: a race with a concurrent agent must never wedge the
    # schedule or rewrite anyone's history. No rebase, no force.
    if timeout 120 git -C "${REPO}" pull --rebase=false --no-edit -q origin "$(git -C "${REPO}" branch --show-current)" >> "${LOG}" 2>&1 \
       && timeout 120 git -C "${REPO}" push -q origin "$(git -C "${REPO}" branch --show-current)" >> "${LOG}" 2>&1; then
      log "pushed"
    else
      log "WARN push failed (likely raced a concurrent agent) — commit is local, next run retries"
    fi
  fi
fi

# ── 3b. resolve the DEPLOY source — prod ships from main ONLY (policy) ─────
# See the header. DEPLOY_FAILED is initialized here (before any skip can set
# it) so a skipped deploy degrades the run exactly like a failed one: prod
# keeping the previous build must never read as success.
DEPLOY_FAILED=0
DEPLOY_DIR=""
if [ "${OPS_NIGHTLY_NO_DEPLOY:-0}" != "1" ]; then
  MAIN_WT="${RIIR_OPS_MAIN_WT:-${REPO}/../.riir-clippy-main}"
  # Canonicalize BEFORE comparing against `worktree list` output: porcelain
  # prints the resolved path, and the raw "${REPO}/.." spelling would never
  # string-match it (the worktree would then be re-created every run and the
  # add would fail on "already registered").
  MAIN_WT="$(cd "${MAIN_WT}" 2>/dev/null && pwd || printf '%s' "${MAIN_WT}")"
  if ! git -C "${REPO}" worktree list --porcelain | grep -Fxq "worktree ${MAIN_WT}"; then
    # First run (or the worktree was pruned): create it. Seeded from
    # origin/main when that exists, origin/develop as the bootstrap fallback
    # — main is born FROM develop and is never allowed to diverge silently
    # (the FF-only merge below is the standing enforcement).
    _start="origin/main"
    if ! git -C "${REPO}" rev-parse -q --verify origin/main >/dev/null 2>&1; then
      _start="origin/develop"
    fi
    if ! git -C "${REPO}" worktree add -q -B main "${MAIN_WT}" "${_start}" >> "${LOG}" 2>&1; then
      log "WARN could not create the main worktree at ${MAIN_WT} — deploy SKIPPED, prod keeps the previous build"
      DEPLOY_FAILED=1
      MAIN_WT=""
    fi
  fi
  if [ -n "${MAIN_WT:-}" ]; then
    # Worktrees share this repo's object store, so this fetch refreshes the
    # remote-tracking refs for BOTH checkouts. FF-ONLY: main either already
    # equals origin/develop or fast-forwards to it — a direct main commit or
    # a held-back release makes this FAIL, and a failed FF skips the deploy
    # rather than shipping a tree main did not vet.
    if git -C "${MAIN_WT}" fetch -q origin \
       && git -C "${MAIN_WT}" merge -q --ff-only origin/develop >> "${LOG}" 2>&1; then
      if timeout 120 git -C "${MAIN_WT}" push -q origin main >> "${LOG}" 2>&1; then
        log "deploy source: main @ $(git -C "${MAIN_WT}" rev-parse --short HEAD) (FF to origin/develop, pushed)"
      else
        log "WARN push of main failed (race?) — deploying the fast-forwarded local main"
      fi
      DEPLOY_DIR="${MAIN_WT}"
    else
      log "WARN main is DIVERGED from origin/develop (FF-only merge failed) — deploy SKIPPED; reconcile main by hand. prod keeps the previous build"
      DEPLOY_FAILED=1
    fi
  fi
fi

# ── 4. deploy ──
# This step is the POINT of the run: steps 1-3 refresh + commit the dashboard,
# but until it is deployed production still serves the PREVIOUS build. It used
# to be WARN-only and fall through to `=== done ===` with exit 0, so a failed
# deploy left prod stale FOREVER with a success signal on every surface
# launchd can see (observed 2026-08-29 00:02: deploy failed, run reported
# done, exit 0). The failure is now recorded and reported in the exit status.
#
# The old WARN also GUESSED its own cause ("wrangler OAuth may need refresh"),
# which was wrong on the observed failure -- wrangler had authenticated fine
# via CLOUDFLARE_API_TOKEN and was refused for MISSING SCOPE. A hint that
# names the wrong fix costs more than no hint, so quote wrangler verbatim.
if [ "${OPS_NIGHTLY_NO_DEPLOY:-0}" != "1" ] && [ -n "${DEPLOY_DIR}" ]; then
  # Stamped BEFORE the upload so step 4a can ask "did a deployment land
  # AFTER we started?" -- the question `wrangler deploy`'s exit code does
  # not answer.
  _t0="$(date -u +%Y-%m-%dT%H:%M:%S)"
  _dout="$(mktemp)"
  # `env -u ...` is LOAD-BEARING (2026-08-29). Step 0b loads .env for the CF
  # ANALYTICS credential; wrangler, unrelatedly, auto-adopts any ambient
  # CLOUDFLARE_API_TOKEN and PREFERS it over the OAuth session. So an
  # analytics-scoped token in the invoking shell silently SHADOWED an OAuth
  # session that holds `workers_scripts (write)`, and the deploy was refused
  # for missing scope -- which then read as "the owner must mint a deploy
  # token" when no such gap existed. Two credentials, two jobs: analytics
  # reads use the token, deploys use OAuth. Unsetting here also makes a
  # MANUAL run behave exactly like the launchd run, whose spartan environment
  # never had these set -- so a hand-run can no longer fail in a way the
  # scheduled run would not.
  #
  # CF_API_TOKEN belongs in the same strip list (2026-08-29, Bench 057
  # follow-up): wrangler honours it as a DEPRECATED ALIAS of
  # CLOUDFLARE_API_TOKEN ("Using \"CF_API_TOKEN\" environment variable. This
  # is deprecated."), and step 0b exports exactly that name from .env -- so
  # under launchd the deploy was STILL shadowed by the analytics token after
  # the CLOUDFLARE_API_TOKEN fix. Measured, not assumed: sourcing .env and
  # running `env -u CLOUDFLARE_API_TOKEN wrangler whoami` reported the API
  # token, not OAuth.
  #
  # Deploy-credential resolution (2026-08-29): CF_DEPLOY_API_TOKEN (optional
  # .env entry) is the least-privilege deploy token (Account -> Workers
  # Scripts -> Edit, ideally script-scoped to riir-ops-dashboard). When set
  # it is PREFERRED over the personal OAuth session -- the OAuth file carries
  # 29 scopes of near-total account authority, and an unattended launchd job
  # only ever needs this one worker. It is mapped onto CLOUDFLARE_API_TOKEN
  # for the individual command (wrangler's own var) -- never exported -- and
  # the analytics credentials stay stripped in both arms. GitHub Actions sets
  # the same value as its CLOUDFLARE_API_TOKEN secret, so local + CI deploy
  # paths use one credential.
  if [ -n "${CF_DEPLOY_API_TOKEN:-}" ]; then
    log "deploy credential: CF_DEPLOY_API_TOKEN (least-privilege; overrides OAuth)"
    # `env` MUST come first and the assignment must be an ARGUMENT to it.
    # A leading `VAR=value` is an assignment prefix only when the SHELL PARSES
    # it -- after `"${_w_env[@]}"` expansion it is just the first word, so the
    # shell looked for a command literally named `CLOUDFLARE_API_TOKEN=<tok>`
    # and every deploy died with `command not found`. `.env` HAS this var set,
    # so that was 100% of deploys on the least-privilege path (2026-08-29).
    _w_env=(env -u CF_API_TOKEN -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL "CLOUDFLARE_API_TOKEN=${CF_DEPLOY_API_TOKEN}")
  else
    log "deploy credential: wrangler OAuth (CF_DEPLOY_API_TOKEN unset)"
    _w_env=(env -u CLOUDFLARE_API_TOKEN -u CF_API_TOKEN -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL)
  fi
  if (cd "${DEPLOY_DIR}/cloudflare/dashboard" && "${_w_env[@]}" timeout 600 npx --yes "wrangler@${WRANGLER_VERSION}" deploy >"${_dout}" 2>&1); then
    cat "${_dout}" >> "${LOG}"
    log "deployed"
  else
    cat "${_dout}" >> "${LOG}"
    DEPLOY_FAILED=1
    # Verbatim, from wrangler -- never a guess. `head -3` keeps the log line
    # bounded; the full output is already appended above.
    # `|| true` is LOAD-BEARING: `set -o pipefail` + a no-match grep returns 1,
    # which under `set -e` aborted the run with NO log line at all -- silently
    # worse than the WARN-only original, and on the LIKELIEST unattended
    # failure (a bare `timeout 600` kill prints nothing this pattern matches).
    _why="$(grep -iE 'error|denied|not authorized|permission|unauthorized|expired|missing|not found|no such' "${_dout}" \
            | head -3 | tr '\n' ' ' | cut -c1-400 || true)"
    log "WARN deploy FAILED — production still serves the PREVIOUS dashboard. wrangler said: ${_why:-<no error line matched; see full output above>}"
  fi
  rm -f "${_dout}"
fi

# ── 4a. post-deploy verification: did production actually RECEIVE it? ──
# `wrangler deploy` exiting 0 means the API accepted the upload, NOT that the
# deployment landed. This issue has now been bitten three times by a green
# surface over a wrong fact (T12 wrong per-day series, T15 dropped domain,
# T16 failed deploy reported as success), so the deploy gets an independent
# check rather than a self-report.
#
# Deliberately NOT a fetch-and-byte-compare of the dashboard HTML: that only
# works while the auth gate is OPEN (T13). The moment someone runs
# `wrangler secret put DASHBOARD_TOKEN`, `/` stops serving the payload and a
# body check would go RED on a correct deploy. The deployment list is
# posture-independent, so this gate survives closing the gate.
if [ "${DEPLOY_FAILED}" = "0" ] && [ "${OPS_NIGHTLY_NO_DEPLOY:-0}" != "1" ]; then
  _newest="$(cd "${DEPLOY_DIR}/cloudflare/dashboard" \
    && "${_w_env[@]}" timeout 120 npx --yes "wrangler@${WRANGLER_VERSION}" deployments list --json 2>/dev/null \
    | python3 -c 'import sys,json
try:
    d = json.load(sys.stdin)
    print(max(x["created_on"] for x in d) if d else "")
except Exception:
    print("")' 2>/dev/null || true)"
  # THREE outcomes, not two. "Could not measure" must never be reported as
  # "measured stale" -- a network blip on this read would otherwise turn a
  # correct deploy into a nightly exit 2 (a false alarm is how a gate gets
  # ignored, and then disabled).
  case "${_newest}" in
    "")
      log "NOTE post-deploy verify INCONCLUSIVE — could not read the deployment list; deploy itself reported success" ;;
    *)
      if [ "${_newest}" \> "${_t0}" ]; then
        log "verified: deployment landed at ${_newest} (after ${_t0})"
      else
        DEPLOY_FAILED=1
        log "WARN post-deploy verify FAILED — wrangler exited 0 but the newest deployment is ${_newest}, not newer than ${_t0}: production may still serve the PREVIOUS build"
      fi ;;
  esac
fi

# ── 4b. housekeeping-Worker regression gate (Issue 045 T8) ──
# This nightly does NOT deploy the housekeeping worker, so this gate cannot
# block anything — it exists because the T8 bug (colliding hour keys) lived in
# production for two days while every "live-verified" check passed: the
# endpoints returned 200 with plausible JSON the whole time. A shape assertion
# on the KEYS is what catches that class, and this is the repo's only
# unattended runner. WARN-only, by design: a housekeeping regression must
# never stop the dashboard refresh.
if [ -d "${REPO}/cloudflare/housekeeping/test" ]; then
  # NB: a bare directory arg makes `node --test` fail MODULE_NOT_FOUND;
  # it wants file paths, so glob them.
  if timeout 120 node --test "${REPO}"/cloudflare/housekeeping/test/*.test.mjs >> "${LOG}" 2>&1; then
    log "housekeeping worker tests PASS"
  else
    log "WARN housekeeping worker tests FAILED — see above (Issue 045 T8)"
  fi
fi

# ── 4c. corpus days sync (riir-dapps Issue 102) — git → the kat-service
# corpus card. Mines THIS repo's corpus git history (the miner runs inside
# this checkout) and POSTs the series to each env's merge door. WARN-only,
# by design: a corpus-sync failure must never stop the dashboard refresh.
# OPS_NIGHTLY_NO_CORPUS_SYNC=1 skips (the run-once-by-hand escape hatch).
if [ "${OPS_NIGHTLY_NO_CORPUS_SYNC:-0}" != "1" ]; then
  if [ -x "${REPO}/../riir-dapps/scripts/corpus_days_sync.sh" ]; then
    log "corpus days sync starting"
    if "${REPO}/../riir-dapps/scripts/corpus_days_sync.sh" >> "${LOG}" 2>&1; then
      log "corpus days sync ok"
    else
      log "WARN corpus days sync FAILED (see above; riir-dapps Issue 102) — continuing"
    fi
  else
    log "WARN corpus days sync skipped — no ../riir-dapps checkout beside this repo"
  fi
fi

# ── 4d. healqual days sync (riir-dapps Plan 037) — the healer's daily
# quality series → the kat-service "Self-evolve" card. Mines THIS repo's
# score-bench + frontier feeds (the miner runs inside this checkout) and
# POSTs the signed series to each env's merge door. WARN-only, by design:
# a healqual-sync failure must never stop the dashboard refresh.
# OPS_NIGHTLY_NO_HEALQUAL_SYNC=1 skips (the run-once-by-hand escape
# hatch). The sync's unauthorized line lands in the log above — that is
# the 401/403-LOUD surface (key rotation or an allowlist drop must
# surface here, never warn-and-continue into a silently stale card).
# Companion (T4.3, riir-clippy Proposal 016): the OPPORTUNISTIC push runs
# on its own launchd interval — com.riir.healqual-sync.plist, 4h,
# `healqual_sync.sh --if-changed` (digest-gated). THIS leg stays the
# always-POST liveness probe; never point it at --if-changed.
if [ "${OPS_NIGHTLY_NO_HEALQUAL_SYNC:-0}" != "1" ]; then
  if [ -x "${REPO}/../riir-dapps/scripts/healqual_sync.sh" ]; then
    log "healqual days sync starting"
    if "${REPO}/../riir-dapps/scripts/healqual_sync.sh" >> "${LOG}" 2>&1; then
      log "healqual days sync ok"
    else
      log "WARN healqual days sync FAILED (see above; riir-dapps Plan 037) — continuing"
    fi
  else
    log "WARN healqual days sync skipped — no ../riir-dapps checkout beside this repo"
  fi
fi

# ── 5. keep the log bounded (this runs unattended, forever) ──
if [ "$(wc -l < "${LOG}")" -gt 5000 ]; then
  tail -n 2000 "${LOG}" > "${LOG}.tmp" && mv "${LOG}.tmp" "${LOG}"
  log "log truncated to last 2000 lines"
fi

# A failed deploy is a DEGRADED run, not a successful one: the artifacts are
# refreshed and committed but production is stale. Exit non-zero so launchd's
# log records a failure instead of the silent exit 0 that hid this for a day.
# Distinct code (2) so a supervisor can tell it apart from the exit-1 aborts
# above, which fail BEFORE anything is committed or deployed.
# (No KeepAlive in com.riir.ops-nightly.plist, so this cannot respawn-loop.)
if [ "${DEPLOY_FAILED:-0}" = "1" ]; then
  log "=== done (DEGRADED: deploy failed — prod dashboard is STALE) ==="
  exit 2
fi

log "=== done ==="
NIGHTLY_COMPLETED=1  # the last line — see nightly_cleanup above
