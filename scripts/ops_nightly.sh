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
if [ "${OPS_NIGHTLY_NO_DEPLOY:-0}" != "1" ]; then
  if (cd "${REPO}/cloudflare/dashboard" && timeout 600 npx --yes "wrangler@${WRANGLER_VERSION}" deploy >> "${LOG}" 2>&1); then
    log "deployed"
  else
    log "WARN deploy failed (wrangler OAuth may need refresh: npx wrangler login)"
  fi
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

log "=== done ==="
