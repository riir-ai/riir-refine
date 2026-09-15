#!/usr/bin/env bash
# replay_e2e.sh — the repeatable K=2 replay-lane e2e (riir-dapps Issue 091's
# "setup a mode"): two identities × one `cargo heal replay` cycle each,
# against devnet, with honest verdicts.
#
# What it does (the 2026-09-15 manual run, productized):
#   verifier A = the default account dir (the fleet identity on this rig);
#   verifier B = a FRESH throwaway identity (ssh-keygen + `login --key`,
#     never the machine-SSH chain — that derives the fleet account, and one
#     account cannot counter-sign its own verdict twice);
#   both run exactly one lease → judge (real clippy oracle) → submit cycle;
#   the harness classifies the transcripts and prints a verdict.
#
# Exit codes: 0 PASS (exercised | idempotent) · 1 FAIL · 2 usage/refusal ·
#             3 EMPTY (queue drained — the lane answered, nothing was
#               leasable; remedy: push a real mining batch and re-run).
#
# Env:
#   REPLAY_E2E_URL            service URL. Default https://devnet.ai.gist.rs.
#                             Must contain "devnet" unless
#                             REPLAY_E2E_ALLOW_ANY_URL=1 (the apex IS
#                             mainnet — throwaway e2e traffic never goes
#                             there; mainnet is unarmed anyway).
#   REPLAY_E2E_TARGET_DIR     cargo target dir for the oracle build.
#                             Default: the repo's own target/ (the binary
#                             there already carries the lane; set /tmp/... to
#                             isolate when a sibling holds the build lock).
#   REPLAY_E2E_SKIP_BUILD     =1 reuse the existing binary without building.
#
# The client build needs the oracle feature set (the release set does NOT
# carry clippy_verify — a verdict is measured, never guessed):
#   latent_retrieval,kat_service,kat_transport,fix_verify,clippy_verify
#
# Assertions (transcript shapes, per the 2026-09-15 devnet record):
#   "… verdict <accept>"        an accepted submit (stored/duplicate/confirmed)
#   "… verdict SUBMIT FAILED"   a rejected submit
#   "confirmed — K=2 quorum reached (n)"  the confirmation event
#   "carries no items"          empty lease (exit 0 client-side)
#   "toolchain SKEW"            the pin check fired (exit 2 client-side)
set -euo pipefail

# ── completion sentinel (the trap-launder defense, katgpt-rs Issue 734) ──
COMPLETED=0
# shellcheck disable=SC2154  # rc=$? IS the assignment; trap bodies trip SC2154
trap 'rc=$?; if [ "$COMPLETED" -ne 1 ] && [ $rc -eq 0 ]; then
  echo "replay-e2e: INCOMPLETE — the script aborted before its verdict" >&2
  exit 1
fi
exit $rc' EXIT

URL="${REPLAY_E2E_URL:-https://devnet.ai.gist.rs}"
FEATURES="latent_retrieval,kat_service,kat_transport,fix_verify,clippy_verify"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [ "${REPLAY_E2E_ALLOW_ANY_URL:-0}" != "1" ]; then
  case "$URL" in
    *devnet*) ;;
    *)
      echo "replay-e2e: REFUSED — URL '$URL' is not a devnet host (the apex IS" >&2
      echo "  mainnet; e2e throwaway traffic belongs on devnet only)." >&2
      echo "  Override consciously with REPLAY_E2E_ALLOW_ANY_URL=1." >&2
      exit 2
      ;;
  esac
fi

for tool in cargo ssh-keygen; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "replay-e2e: REFUSED — $tool is not on PATH" >&2
    exit 2
  }
done

# ── build (isolated-optional; the repo target usually carries the lane) ──
if [ -n "${REPLAY_E2E_TARGET_DIR:-}" ]; then
  export CARGO_TARGET_DIR="$REPLAY_E2E_TARGET_DIR"
fi
BIN="$ROOT/target/debug/cargo-heal"
if [ "${CARGO_TARGET_DIR:-}" != "" ]; then
  BIN="$CARGO_TARGET_DIR/debug/cargo-heal"
fi

if [ "${REPLAY_E2E_SKIP_BUILD:-0}" != "1" ]; then
  echo "replay-e2e: building the oracle client ($FEATURES) → ${BIN}"
  (cd "$ROOT" && cargo build --features "$FEATURES" --bin cargo-heal)
elif [ ! -x "$BIN" ]; then
  echo "replay-e2e: REFUSED — REPLAY_E2E_SKIP_BUILD=1 but no binary at $BIN" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/riir-replay-e2e.XXXXXX")"
LOG_A="$WORK/verifier-a.log"
LOG_B="$WORK/verifier-b.log"
echo "replay-e2e: work dir $WORK (kept for forensics)"
echo "replay-e2e: target $URL"

# ── verifier A: the default account dir (the fleet identity) ────────────
echo "replay-e2e: [A] one replay cycle, default identity"
rc_a=0
RIIR_HEAL_KAT_SERVICE_URL="$URL" "$BIN" heal replay >"$LOG_A" 2>&1 || rc_a=$?
sed 's/^/  A| /' "$LOG_A"

# ── verifier B: fresh throwaway identity (the second-identity helper) ───
echo "replay-e2e: [B] generating a throwaway identity (never the SSH chain)"
ssh-keygen -q -t ed25519 -N "" -C "replay-e2e" -f "$WORK/id_ed25519"
rc_b=0
RIIR_AUTH_ACCOUNT_DIR="$WORK/acct" \
RIIR_HEAL_KAT_ACCOUNT_DIR="$WORK/acct" \
RIIR_HEAL_KAT_SERVICE_URL="$URL" \
  "$BIN" heal login --key "$WORK/id_ed25519" >"$LOG_B" 2>&1 || rc_b=$?
if [ $rc_b -ne 0 ]; then
  sed 's/^/  B| /' "$LOG_B"
  echo "replay-e2e: FAIL — verifier B login failed (rc $rc_b)" >&2
  exit 1
fi
grep -q "account key imported" "$LOG_B" || {
  sed 's/^/  B| /' "$LOG_B"
  echo "replay-e2e: FAIL — verifier B login did not import the key" >&2
  exit 1
}
RIIR_AUTH_ACCOUNT_DIR="$WORK/acct" \
RIIR_HEAL_KAT_ACCOUNT_DIR="$WORK/acct" \
RIIR_HEAL_KAT_SERVICE_URL="$URL" \
  "$BIN" heal replay >>"$LOG_B" 2>&1 || rc_b=$?
sed 's/^/  B| /' "$LOG_B"

# ── classify ─────────────────────────────────────────────────────────────
count() {
  # grep -c always prints a count when it can read the file (0 on no
  # match, exit 1); on an unreadable file it prints nothing — fold that to
  # 0 so the arithmetic below can never see a missing operand.
  local n
  n=$(grep -Eac "$1" "$2" 2>/dev/null) || true
  echo "${n:-0}"
}

skew=$(( $(count 'toolchain SKEW' "$LOG_A") + $(count 'toolchain SKEW' "$LOG_B") ))
empty=$(( $(count 'carries no items' "$LOG_A") + $(count 'carries no items' "$LOG_B") ))
# accepted verdict lines = "… verdict <accept>" minus submit failures
va_all=$(count '… verdict ' "$LOG_A"); vb_all=$(count '… verdict ' "$LOG_B")
va_bad=$(count '… verdict SUBMIT FAILED' "$LOG_A"); vb_bad=$(count '… verdict SUBMIT FAILED' "$LOG_B")
va=$(( va_all - va_bad )); vb=$(( vb_all - vb_bad ))
quorum=$(( $(count 'quorum reached' "$LOG_A") + $(count 'quorum reached' "$LOG_B") ))
dups=$(( $(count 'duplicate' "$LOG_A") + $(count 'duplicate' "$LOG_B") ))
accepted=$(( va + vb ))

echo
echo "replay-e2e: ── summary ─────────────────────────────────"
echo "replay-e2e: rc A=$rc_a B=$rc_b · skew=$skew · empty-leases=$empty"
echo "replay-e2e: accepted verdicts: A=$va B=$vb (duplicates $dups)"
echo "replay-e2e: K=2 confirmations this run: $quorum"
echo "replay-e2e: logs: $LOG_A · $LOG_B"

if [ "$skew" -gt 0 ]; then
  echo "replay-e2e: FAIL — toolchain SKEW: the devnet pin ≠ blake3(rustc -vV)" >&2
  echo "  remedy: re-arm KAT_REPLAY_TOOLCHAIN_HEX (wrangler secret, devnet env)" >&2
  echo "  or align the local toolchain with the arm." >&2
  exit 1
fi
if [ "$rc_a" -ne 0 ] || [ "$rc_b" -ne 0 ]; then
  echo "replay-e2e: FAIL — a verifier exited nonzero (A=$rc_a B=$rc_b)" >&2
  exit 1
fi
if [ "$empty" -eq 2 ]; then
  echo "replay-e2e: EMPTY — both leases empty: the lane answered, but nothing" >&2
  echo "  was leasable (all open rows confirmed, or the queue is drained)." >&2
  echo "  remedy: push a real mining batch (a real heal --fix run + sync on" >&2
  echo "  the fleet account), then re-run." >&2
  exit 3
fi
if [ "$accepted" -eq 0 ]; then
  echo "replay-e2e: FAIL — no accepted verdict on either side" >&2
  exit 1
fi
if [ "$quorum" -gt 0 ]; then
  echo "replay-e2e: VERDICT: PASS (exercised) — lease + oracle verdicts + K=2"
  echo "  confirmation all live on $URL"
  COMPLETED=1
  exit 0
fi
if [ "$accepted" -eq "$dups" ]; then
  echo "replay-e2e: VERDICT: PASS (idempotent) — write-once held on every"
  echo "  row this run; the lane is alive but no fresh confirmation happened"
  echo "  (expected when the queue holds only rows this account already"
  echo "  verdicted and verifier B disagreed or found none new)."
  COMPLETED=1
  exit 0
fi
echo "replay-e2e: VERDICT: PASS (exercised, partial) — verdicts stored but no"
echo "  K=2 confirmation this round (agreement-dependent; re-run for more)."
COMPLETED=1
exit 0
