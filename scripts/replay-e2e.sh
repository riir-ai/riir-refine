#!/usr/bin/env bash
# replay-e2e.sh — the repeatable K=2 replay-lane e2e (riir-dapps Issue 091's
# "setup a mode and do the replay" item; the manual env-override invocations
# recorded there are now THIS script).
#
# Choreography (two identities, one machine, against the configured service
# — default devnet via `cargo heal config set --url devnet`):
#   verifier 1 = the DEFAULT account (the fleet identity on this machine)
#   verifier 2 = a throwaway SSH ed25519 key imported into an ISOLATED
#                account dir (RIIR_AUTH_ACCOUNT_DIR — the substrate-level
#                override; RIIR_HEAL_KAT_ACCOUNT_DIR isolates only the
#                consent/burn ack state, so BOTH are set) — removed on exit
#
# Both run `cargo heal replay` (lease → local clippy-oracle replay → signed
# verdict submit). PASS requires ≥1 "confirmed — K=2 quorum reached" across
# the two runs; a lane with no leasable rows is an honest SKIP (exit 3),
# never a pass; a lease/toolchain/skew refusal is a FAIL (exit 1).
#
# Requirements: a cargo-heal binary built with the replay feature set:
#   cargo build --features kat_service,kat_transport,latent_retrieval,\
#   rust_perf,clippy_lints --bin cargo-heal
# (or override $HEAL_BIN with an existing binary path).
#
# Usage: scripts/replay-e2e.sh [--keep]   (--keep retains the temp account)
set -euo pipefail

# ── the completion sentinel (the katgpt-rs trap-launder lesson: an
#    aborting bash-3.2 script must not report success through its trap) ──
COMPLETED=0
cleanup() {
    rc=$?
    if [ "$COMPLETED" -ne 1 ] && [ "$rc" -ne 0 ]; then
        echo "replay-e2e: INCOMPLETE (died at rc=$rc) — treat as a failure" >&2
        rc=1
    fi
    if [ "${KEEP:-0}" != "1" ]; then
        rm -rf "$TMPDIR_E2E"
    fi
    exit "$rc"
}
trap cleanup EXIT

KEEP=0
[ "${1:-}" = "--keep" ] && KEEP=1

# The e2e PINS the service explicitly (env > config > default): the
# throwaway account dir carries NO service_url.json, and the default
# (apex/mainnet) refuses the replay lane with a 503 — pinning devnet keeps
# both verifiers on the armed lane regardless of local config state.
SERVICE_URL="${RIIR_HEAL_KAT_SERVICE_URL:-https://devnet.ai.gist.rs}"
export RIIR_HEAL_KAT_SERVICE_URL="$SERVICE_URL"
echo "── service: $SERVICE_URL"

HEAL_BIN="${HEAL_BIN:-./target/debug/cargo-heal}"
if [ ! -x "$HEAL_BIN" ]; then
    echo "replay-e2e: no binary at $HEAL_BIN — build with the replay feature set (see header) or set HEAL_BIN" >&2
    exit 2
fi

TMPDIR_E2E="$(mktemp -d /tmp/kat-replay-e2e.XXXXXX)"
KEY_SRC="$TMPDIR_E2E/import"
ACCT="$TMPDIR_E2E/account"
mkdir -p "$KEY_SRC" "$ACCT"
ssh-keygen -q -t ed25519 -N "" -f "$KEY_SRC/id_ed25519" -C kat-replay-e2e

echo "── verifier 2: import the throwaway identity into $ACCT"
RIIR_AUTH_ACCOUNT_DIR="$ACCT" RIIR_HEAL_KAT_ACCOUNT_DIR="$ACCT" \
    "$HEAL_BIN" login --key "$KEY_SRC/id_ed25519" >/dev/null

V2_OUT="$TMPDIR_E2E/verdicts-v2.log"
V1_OUT="$TMPDIR_E2E/verdicts-v1.log"

echo "── verifier 2 (throwaway): lease → replay → submit"
# Exit 1 from replay = every verdict failed to submit or none submitted —
# for the e2e that is a FAIL only if verifier 1 also cannot run; capture
# and judge below (a skew/toolchain refusal is a real failure).
RIIR_AUTH_ACCOUNT_DIR="$ACCT" RIIR_HEAL_KAT_ACCOUNT_DIR="$ACCT" \
    "$HEAL_BIN" replay >"$V2_OUT" 2>&1 || true

echo "── verifier 1 (default fleet account): lease → replay → submit"
"$HEAL_BIN" replay >"$V1_OUT" 2>&1 || true

echo "── verdicts (verifier 2):"
sed 's/^/  v2| /' "$V2_OUT"
echo "── verdicts (verifier 1):"
sed 's/^/  v1| /' "$V1_OUT"

# Honest adjudication, in order of severity:
#   build-gate / skew / unconfigured / no-account → FAIL (fix the rig)
#   no leasable rows (both runs)      → SKIP (exit 3; nothing to verify)
#   ≥1 quorum reached                 → PASS
if grep -qE "requires a build|SKEW|503|not configured|no account" "$V2_OUT" "$V1_OUT"; then
    echo "replay-e2e: FAIL — the lane refused (build gate/skew/unconfigured/no-account); see the logs above" >&2
    exit 1
fi
if grep -q "carries no items" "$V2_OUT" && grep -q "carries no items" "$V1_OUT"; then
    echo "replay-e2e: SKIP — no leasable rows for either verifier (queue proven/settled); run again after the next sync push" >&2
    exit 3
fi
if grep -q "K=2 quorum reached" "$V2_OUT" "$V1_OUT"; then
    echo "replay-e2e: PASS — K=2 quorum reached (the recording lane is live end-to-end)"
    COMPLETED=1
    exit 0
fi
echo "replay-e2e: FAIL — verdicts submitted but no quorum confirmed in this run (rows may already be confirmed by earlier pairs; check the accept lines above)" >&2
COMPLETED=1
exit 1
