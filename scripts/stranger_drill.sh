#!/usr/bin/env bash
# stranger_drill.sh — the Plan 045 Phase A8 stranger drill (Proposal 008):
# a clean-install stranger joins devnet through the NEW first-join prompt
# (A1), claims the trial grant, heals a throwaway sample crate, pushes the
# mining batch, and syncs — while the refusal paths that are cheaply
# assertable are asserted against a SECOND stranger who joined MAINNET
# (A3: the posture copy names every unarmed plane up front; the top-up URL
# is named while /pay is still 404 — expected until Plan 045 B1 lands).
#
# Pattern: replay_e2e.sh (completion sentinel, devnet guard, per-leg logs,
# honest verdicts). Exit codes: 0 PASS · 1 FAIL · 2 usage/preflight refusal.
#
# Env:
#   STRANGER_DRILL_URL          service URL for the DEVNET preflight only.
#                               Default https://devnet.ai.gist.rs. Must
#                               contain "devnet" unless
#                               STRANGER_DRILL_ALLOW_ANY_URL=1 — the client
#                               commands resolve their service through the
#                               A1-recorded config, never this var.
#   STRANGER_DRILL_TARGET_DIR   cargo target dir for the build (isolate
#                               when a sibling holds the build lock).
#   STRANGER_DRILL_SKIP_BUILD   =1 reuse the existing binary without building.
#
# Settle/vest legs are DEFERRED by design (an epoch settle cannot complete
# in-session); `--with-settle` runs the pipeline reads and prints the
# observed state instead. Neither can force a settle — that is stated,
# never faked.
#
# The client build needs the loop's feature set (NOT the full oracle set —
# the drill heals a sample crate, it never replays):
#   latent_retrieval,kat_service,kat_transport,fix_verify
set -euo pipefail

# ── completion sentinel (the trap-launder defense, katgpt-rs Issue 734) ──
COMPLETED=0
# shellcheck disable=SC2154  # rc=$? IS the assignment; trap bodies trip SC2154
trap 'rc=$?; if [ "$COMPLETED" -ne 1 ] && [ $rc -eq 0 ]; then
  echo "stranger-drill: INCOMPLETE — the script aborted before its verdict" >&2
  exit 1
fi
exit $rc' EXIT

URL="${STRANGER_DRILL_URL:-https://devnet.ai.gist.rs}"
FEATURES="latent_retrieval,kat_service,kat_transport,fix_verify"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WITH_SETTLE=0
if [ "${1:-}" = "--with-settle" ]; then
  WITH_SETTLE=1
fi

if [ "${STRANGER_DRILL_ALLOW_ANY_URL:-0}" != "1" ]; then
  case "$URL" in
    *devnet*) ;;
    *)
      echo "stranger-drill: REFUSED — URL '$URL' is not a devnet host (the apex IS" >&2
      echo "  mainnet; a stranger drill's throwaway traffic belongs on devnet only)." >&2
      echo "  Override consciously with STRANGER_DRILL_ALLOW_ANY_URL=1." >&2
      exit 2
      ;;
  esac
fi

for tool in cargo ssh-keygen curl git; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "stranger-drill: REFUSED — $tool is not on PATH" >&2
    exit 2
  }
done

# ── preflight: the drill never fakes a pass on an unreachable network ────
health="$(curl -fsS -o /dev/null -w '%{http_code}' --max-time 15 "$URL/health" 2>/dev/null || true)"
if [ "$health" != "200" ]; then
  echo "stranger-drill: REFUSED — $URL/health answered '${health:-nothing}' (want 200)." >&2
  echo "  devnet is unreachable from this box; the drill left the stranger" >&2
  echo "  unexecuted rather than fake a pass. Re-run when the network answers." >&2
  exit 2
fi
echo "stranger-drill: preflight OK — $URL/health answered 200"
# The MAINNET legs (stranger B, the A3 refusal arms) preflight separately:
# an unreachable apex DEFERS them loud — it is not a pass and not a fail.
apex_health="$(curl -fsS -o /dev/null -w '%{http_code}' --max-time 15 "https://ai.gist.rs/health" 2>/dev/null || true)"
MAINNET_OK=0
if [ "$apex_health" = "200" ]; then
  MAINNET_OK=1
  echo "stranger-drill: preflight OK — https://ai.gist.rs/health answered 200 (mainnet legs armed)"
else
  echo "stranger-drill: mainnet unreachable ('${apex_health:-nothing}') — the A3 legs DEFER loud"
fi

# ── build (isolated-optional; the repo target usually carries the loop) ──
if [ -n "${STRANGER_DRILL_TARGET_DIR:-}" ]; then
  export CARGO_TARGET_DIR="$STRANGER_DRILL_TARGET_DIR"
fi
BIN="$ROOT/target/debug/cargo-refine"
if [ "${CARGO_TARGET_DIR:-}" != "" ]; then
  BIN="$CARGO_TARGET_DIR/debug/cargo-refine"
fi

if [ "${STRANGER_DRILL_SKIP_BUILD:-0}" != "1" ]; then
  echo "stranger-drill: building the drill client ($FEATURES) → ${BIN}"
  (cd "$ROOT" && cargo build --features "$FEATURES" --bin cargo-refine)
elif [ ! -x "$BIN" ]; then
  echo "stranger-drill: REFUSED — STRANGER_DRILL_SKIP_BUILD=1 but no binary at $BIN" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/riir-stranger-drill.XXXXXX")"
SAMPLE="$WORK/sample"
echo "stranger-drill: work dir $WORK (kept for forensics)"
echo "stranger-drill: the client resolves its service through the A1 config record — no env override anywhere"

fail() {
  echo "stranger-drill: FAIL — $1" >&2
  echo "stranger-drill: logs under $WORK" >&2
  exit 1
}

# run_leg <log> <acct-dir> <cwd> [KEY=VALUE ...] -- <cargo-refine args...>
# The extra env is a bash-3.2-safe expansion (an empty set must not trip
# `set -u`); no billing-plane kill switches are set — the drill IS the
# real metered loop.
run_leg() {
  local log="$1" acct="$2" cwd="$3"
  shift 3
  local envs=()
  while [ "${1:-}" != "--" ]; do
    envs+=("$1")
    shift
  done
  shift
  (cd "$cwd" \
    && env RIIR_REFINE_KAT_ACCOUNT_DIR="$acct" RIIR_AUTH_ACCOUNT_DIR="$acct" \
      ${envs[@]+"${envs[@]}"} "$BIN" "$@") >"$log" 2>&1
}

show() { # show <log> <prefix>
  sed "s/^/  $2| /" "$1"
}

# ══ Stranger A: the clean install, joining devnet through the A1 prompt ══
echo "stranger-drill: [A] fresh key + join via the first-join prompt (--yes → devnet)"
ACCT_A="$WORK/acct-a"
ssh-keygen -q -t ed25519 -N "" -C "stranger-drill-a" -f "$WORK/id_a"
LOG_A_JOIN="$WORK/a-join.log"
if ! run_leg "$LOG_A_JOIN" "$ACCT_A" "$WORK" -- login --key "$WORK/id_a" --yes; then
  show "$LOG_A_JOIN" A-join
  fail "stranger A login exited nonzero"
fi
show "$LOG_A_JOIN" A-join
grep -q "recorded to config (cargo refine config set --url devnet)" "$LOG_A_JOIN" \
  || fail "A1: the join did not record devnet through the config-set record"
if grep -q "Join which kat network" "$LOG_A_JOIN"; then
  fail "A1: the prompt block printed under --yes (the default must answer silently)"
fi
[ -f "$ACCT_A/service_url.json" ] || fail "A1: no service_url.json in the account dir"
grep -q "https://devnet.ai.gist.rs" "$ACCT_A/service_url.json" \
  || fail "A1: the record does not hold the devnet URL"
LOG_A_CFG="$WORK/a-config-get.log"
if ! run_leg "$LOG_A_CFG" "$ACCT_A" "$WORK" -- config get; then
  show "$LOG_A_CFG" A-cfg
  fail "config get exited nonzero"
fi
show "$LOG_A_CFG" A-cfg
grep -q "source: config" "$LOG_A_CFG" \
  || fail "A1: config get does not report the joined record (source: config)"
grep -q "https://devnet.ai.gist.rs" "$LOG_A_CFG" \
  || fail "A1: config get does not show the devnet URL"
echo "stranger-drill: [A] join recorded (devnet, config source) — config get agrees"

echo "stranger-drill: [A] trial grant claim (read-first, via \`account\`)"
LOG_A_CLAIM="$WORK/a-claim.log"
if ! run_leg "$LOG_A_CLAIM" "$ACCT_A" "$WORK" -- account; then
  show "$LOG_A_CLAIM" A-claim
  fail "stranger A account (claim) exited nonzero"
fi
show "$LOG_A_CLAIM" A-claim
grep -q "grant:" "$LOG_A_CLAIM" \
  || fail "A: no grant line — the claim plane did not answer"
if grep -q "not claimed" "$LOG_A_CLAIM"; then
  fail "A: the devnet grant was NOT claimed (the claim answered a refusal)"
fi
echo "stranger-drill: [A] grant claimed on devnet"

# ── the throwaway sample crate the stranger heals (a real git repo — a ──
# stranger's own project is one, and the own-repo sync origin needs it) ───
mkdir -p "$SAMPLE/src"
cat >"$SAMPLE/Cargo.toml" <<'TOML'
[package]
name = "drill_sample"
version = "0.1.0"
edition = "2021"

[workspace]
TOML
cat >"$SAMPLE/src/lib.rs" <<'RS'
pub fn early(x: u32) -> u32 {
    if x > 0 {
        return 1;
    }
    return 0;
}

pub fn is_on(flag: bool) -> bool {
    if flag == true {
        true
    } else {
        false
    }
}
RS
git -C "$SAMPLE" init -q
git -C "$SAMPLE" -c user.email=drill@example.invalid -c user.name="stranger drill" add -A
git -C "$SAMPLE" -c user.email=drill@example.invalid -c user.name="stranger drill" commit -qm "drill sample"

echo "stranger-drill: [A] heal the sample crate (--fix; the real metered loop)"
LOG_A_FIX="$WORK/a-fix.log"
if ! run_leg "$LOG_A_FIX" "$ACCT_A" "$SAMPLE" -- --fix --yes "$SAMPLE"; then
  show "$LOG_A_FIX" A-fix
  fail "stranger A --fix exited nonzero"
fi
show "$LOG_A_FIX" A-fix
# The TAIL return is the needless_return shape (a mid-function early
# return is idiomatic and clippy stays silent on it — the first drill run
# proved that the hard way); bool_comparison's `== true` must be gone too.
if grep -q "return 0;" "$SAMPLE/src/lib.rs" || grep -q "== true" "$SAMPLE/src/lib.rs"; then
  fail "A: the sample was not healed (needless_return / bool_comparison survived)"
fi
echo "stranger-drill: [A] sample healed (needless_return + bool_comparison applied)"

echo "stranger-drill: [A] --mine (the explainer + consent + the auto-sync push)"
LOG_A_MINE="$WORK/a-mine.log"
if ! run_leg "$LOG_A_MINE" "$ACCT_A" "$SAMPLE" -- --mine; then
  show "$LOG_A_MINE" A-mine
  fail "stranger A --mine exited nonzero"
fi
show "$LOG_A_MINE" A-mine
grep -q "consent: recorded" "$LOG_A_MINE" \
  || fail "A: --mine did not record mining consent"

echo "stranger-drill: [A] sync status (idempotent re-push after --mine's tail)"
LOG_A_SYNC="$WORK/a-sync.log"
if ! run_leg "$LOG_A_SYNC" "$ACCT_A" "$SAMPLE" -- sync; then
  show "$LOG_A_SYNC" A-sync
  fail "stranger A sync exited nonzero"
fi
show "$LOG_A_SYNC" A-sync

echo "stranger-drill: [A] account (position + the soft-gate verdict line)"
LOG_A_ACCT="$WORK/a-account.log"
if ! run_leg "$LOG_A_ACCT" "$ACCT_A" "$SAMPLE" -- account; then
  show "$LOG_A_ACCT" A-acct
  fail "stranger A account exited nonzero"
fi
show "$LOG_A_ACCT" A-acct
# The grant line prints only when the claim CHANGES state (read-first —
# `Ok(None)` on an already-granted account is silent, the second drill run
# proved it); the post-heal account's live-loop evidence is the metered
# position: the local burn + balance projection.
grep -q "balance projection:" "$LOG_A_ACCT" \
  || fail "A: the post-heal account lost the balance projection line"

# ══ Stranger B: the MAINNET joiner — A3 posture copy + the named top-up ══
B_SKIPPED=""
if [ "$MAINNET_OK" != "1" ]; then
  B_SKIPPED="mainnet unreachable at preflight"
  echo "stranger-drill: [B] DEFERRED — $B_SKIPPED (the A3 arms did not run; disclosed, never faked)"
else
  echo "stranger-drill: [B] fresh key + join mainnet via RIIR_REFINE_JOIN_ENV"
  ACCT_B="$WORK/acct-b"
  ssh-keygen -q -t ed25519 -N "" -C "stranger-drill-b" -f "$WORK/id_b"
  LOG_B_JOIN="$WORK/b-join.log"
  if ! run_leg "$LOG_B_JOIN" "$ACCT_B" "$WORK" RIIR_REFINE_JOIN_ENV=mainnet -- login --key "$WORK/id_b"; then
    show "$LOG_B_JOIN" B-join
    fail "stranger B login exited nonzero"
  fi
  show "$LOG_B_JOIN" B-join
  for needle in "recorded to config (cargo refine config set --url mainnet)" \
    "mainnet posture" "reward pool is 0" "TUNA trial credit is unconfigured" \
    "crank" "settles nothing"; do
    grep -q "$needle" "$LOG_B_JOIN" \
      || fail "A3: the mainnet posture copy is missing '$needle'"
  done
  echo "stranger-drill: [B] joined mainnet — every unarmed plane named up front (A3)"

  echo "stranger-drill: [B] account on mainnet (the TUNA claim answers fail-closed)"
  LOG_B_ACCT="$WORK/b-account.log"
  if ! run_leg "$LOG_B_ACCT" "$ACCT_B" "$WORK" -- account; then
    show "$LOG_B_ACCT" B-acct
    fail "stranger B account exited nonzero"
  fi
  show "$LOG_B_ACCT" B-acct
  grep -q "not claimed" "$LOG_B_ACCT" \
    || fail "A3: mainnet TUNA did NOT answer fail-closed (the claim should refuse)"
  echo "stranger-drill: [B] TUNA refused (unconfigured) — named, not discovered"

  echo "stranger-drill: [B] claim --tx on mainnet (the top-up URL must be NAMED)"
  LOG_B_CLAIM="$WORK/b-claim.log"
  # A shape-valid signature (base58, 64-88 chars) — the client validates
  # shape only; the unarmed plane refuses the rest. `|| true` guards the
  # head-close SIGPIPE (the pipefail-discard law) and the length check
  # below keeps the guard honest.
  SIG="$(head -c 4096 /dev/urandom | LC_ALL=C tr -dc '1-9A-HJ-NP-Za-km-np-z' | head -c 88 || true)"
  if [ "${#SIG}" -lt 64 ]; then
    fail "could not generate a valid-shaped signature (got ${#SIG} chars)"
  fi
  if ! run_leg "$LOG_B_CLAIM" "$ACCT_B" "$WORK" -- claim --tx "$SIG"; then
    show "$LOG_B_CLAIM" B-claim
    fail "stranger B claim exited nonzero"
  fi
  show "$LOG_B_CLAIM" B-claim
  grep -q "payment page:" "$LOG_B_CLAIM" \
    || fail "the claim refusal did not NAME the payment page URL"
  PAY_URL="$(grep -o 'https://[^ ]*/pay?acct=[^ ]*' "$LOG_B_CLAIM" | head -1 || true)"
  if [ -z "$PAY_URL" ]; then
    fail "could not extract the named /pay URL"
  fi
  pay_code="$(curl -fsS -o /dev/null -w '%{http_code}' --max-time 15 "$PAY_URL" 2>/dev/null || true)"
  if [ "$pay_code" != "404" ]; then
    echo "stranger-drill: /pay answered '${pay_code:-nothing}' (want 404)" >&2
    fail "the /pay probe: B1 says 404 until Phase B lands; anything else is drift"
  fi
  echo "stranger-drill: [B] top-up URL named ($PAY_URL) while /pay is 404 — expected until Phase B (Plan 045 B1)"
fi

# ══ the settle/vest legs — deferred by design, readable on demand ════════
echo
echo "stranger-drill: ── deferred legs ──────────────────────────────────"
echo "stranger-drill: DEFERRED — proof pass + epoch settle + vest release:"
echo "stranger-drill:   the pushed batch credits at the proof pass, pays at the"
echo "stranger-drill:   next epoch settle (≤7d cadence, the payout clock), and the"
echo "stranger-drill:   vest window releases after LOCKUP_TICKS. A live settle cannot"
echo "stranger-drill:   complete in-session — re-run \`sync\` + \`account --history\`"
echo "stranger-drill:   after the settle to watch the payout land."
if [ "$WITH_SETTLE" = "1" ]; then
  echo "stranger-drill: --with-settle: the pipeline reads (state, not a settle):"
  LOG_A_SSYNC="$WORK/a-settle-sync.log"
  if ! run_leg "$LOG_A_SSYNC" "$ACCT_A" "$SAMPLE" -- sync; then
    show "$LOG_A_SSYNC" A-settle-sync
    fail "with-settle sync exited nonzero"
  fi
  show "$LOG_A_SSYNC" A-settle-sync
  LOG_A_SACCT="$WORK/a-settle-acct.log"
  if ! run_leg "$LOG_A_SACCT" "$ACCT_A" "$SAMPLE" -- account --history; then
    show "$LOG_A_SACCT" A-settle-acct
    fail "with-settle account --history exited nonzero"
  fi
  show "$LOG_A_SACCT" A-settle-acct
  echo "stranger-drill:   (an epoch settle still cannot be forced in-session — the"
  echo "stranger-drill:   reads above are the pipeline's current honest state.)"
fi
echo "stranger-drill: DEFERRED — the soft-gate contribute-now offer on an EMPTIED"
echo "stranger-drill:   account: a fresh grant cannot be emptied in-session (depletion"
echo "stranger-drill:   needs a measured burn at grant scale). The refusal path itself is"
echo "stranger-drill:   contract-pinned by tests/kat_soft_gate_cli.rs; the drill asserts"
echo "stranger-drill:   the plane is LIVE instead (the soft-gate verdict line in \`account\`)."
echo

echo "stranger-drill: ── verdict ────────────────────────────────────────"
echo "stranger-drill: PASS — join (A1 devnet via --yes) → record (config-set"
echo "  shape, config-get-verified) → trial grant claim → fix (needless_return"
echo "  healed, metered) → --mine push → sync"
if [ -n "$B_SKIPPED" ]; then
  echo "stranger-drill:   A3 mainnet legs DEFERRED ($B_SKIPPED) — run again when the apex answers."
else
  echo "stranger-drill:   mainnet joiner (A3): posture copy up front, TUNA fail-closed,"
  echo "  top-up URL named while /pay is 404 (Phase B expected)"
fi
echo "stranger-drill: logs: $WORK"
COMPLETED=1
exit 0
