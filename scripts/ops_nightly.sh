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

log() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >> "${LOG}"; }

# mkdir is atomic on macOS (no flock in base BSD userland). A stale lock from a
# killed run would wedge the schedule forever, so treat >6 h as abandoned.
if ! mkdir "${LOCK}" 2>/dev/null; then
  if [ -n "$(find "${LOCK}" -maxdepth 0 -mmin +360 2>/dev/null)" ]; then
    log "WARN stale lock >6h — reclaiming"
    rmdir "${LOCK}" 2>/dev/null || true
    mkdir "${LOCK}" 2>/dev/null || { log "SKIP could not reclaim lock"; exit 0; }
  else
    log "SKIP another ops_nightly run holds the lock"
    exit 0
  fi
fi
trap 'rmdir "${LOCK}" 2>/dev/null || true' EXIT

log "=== start (pid $$) ==="

# ── 1. regenerate (re-measures healer coverage across the sibling repos) ──
# --sweep shells out to cargo-heal per repo; 20 min is ~4x the measured cost.
if timeout 1200 python3 "${REPO}/scripts/gen_dashboard.py" --sweep >> "${LOG}" 2>&1; then
  log "regen OK"
else
  log "FAIL regen (exit $?) — leaving artifacts untouched, no deploy"
  exit 1
fi

# ── 2. render gate: never deploy or commit an HTML that does not render ──
if [ -f "${REPO}/cloudflare/dashboard/verify_dom.mjs" ] && command -v node >/dev/null; then
  if timeout 120 node "${REPO}/cloudflare/dashboard/verify_dom.mjs" >> "${LOG}" 2>&1; then
    log "verify_dom PASS"
  else
    log "FAIL verify_dom — refusing to deploy/commit a broken dashboard"
    exit 1
  fi
else
  log "WARN verify_dom.mjs or node absent — deploying unverified"
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
  if (cd "${REPO}/cloudflare/dashboard" && timeout 600 npx --yes wrangler deploy >> "${LOG}" 2>&1); then
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
if [ -d "${REPO}/cloudflare/housekeeping/test" ] && command -v node >/dev/null; then
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
