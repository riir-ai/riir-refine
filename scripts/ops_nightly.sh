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
# Regenerate without touching git or Cloudflare:
#   OPS_NIGHTLY_NO_COMMIT=1 OPS_NIGHTLY_NO_DEPLOY=1 scripts/ops_nightly.sh

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${REPO}/.benchmarks/ops_nightly.log"
LOCK="${REPO}/.benchmarks/.ops_nightly.lock"
ARTIFACTS=(".benchmarks/dashboard_coverage.json" "cloudflare/dashboard/public/index.html")

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
trap release_lock EXIT

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

# A missing tool must STOP this runner, never downgrade it. A gate that
# silently stops gating is worse than no gate, because the log still reads
# green — which is precisely how the Issue 045 T8 hour-key bug survived two
# days of "live-verified" checks.
for _tool in python3 node npx git; do
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
# --sweep shells out to cargo-heal per repo; 20 min is ~4x the measured cost.
if timeout 1200 python3 "${REPO}/scripts/gen_dashboard.py" --sweep >> "${LOG}" 2>&1; then
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
        commit -q -m "chore(ops): nightly dashboard refresh (issue 044 T7)" \
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
DEPLOY_FAILED=0
if [ "${OPS_NIGHTLY_NO_DEPLOY:-0}" != "1" ]; then
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
  if (cd "${REPO}/cloudflare/dashboard" && env -u CLOUDFLARE_API_TOKEN -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL timeout 600 npx --yes "wrangler@${WRANGLER_VERSION}" deploy >"${_dout}" 2>&1); then
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
    _why="$(grep -iE 'error|denied|not authorized|permission|unauthorized|expired|missing' "${_dout}" \
            | head -3 | tr '\n' ' ' | cut -c1-400 || true)"
    log "WARN deploy FAILED — production still serves the PREVIOUS dashboard. wrangler said: ${_why:-<no error line matched; see full output above>}"
  fi
  rm -f "${_dout}"
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
