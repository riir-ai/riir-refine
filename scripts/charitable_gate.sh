#!/bin/sh
# ─────────────────────────────────────────────────────────────────────────────
# Issue 190 T14 — the charitable (Posture A) build sweep, as a NAMED posture.
#
# Builds cargo-refine with the charitable feature set EXACTLY as verified by
# T14a-min (`e7cf72ea`): default + latent_retrieval,rust_perf,clippy_lints.
# The release set is NEVER touched here — `release_set_matches_workflow` pins
# it; this is the second posture, built BY NAME, never a silent change.
#
# Bars (issue 190 T14; runnable since T14a-min + T14b landed 2026-10-31):
#   (1) CLI output grep — `--help` / `--version` / a bare dry-run / a scan
#       run, case-insensitive, PAYMENT/TOKEN TERMS ONLY (issue 192 GAP 2
#       re-scope, 2026-11-01):
#       kat|payout|epoch settle|lease|pack|usdc|wallet|topup
#       The earlier wide pattern (mine|earn|epoch|settle|credit|join|service)
#       collided with the PRODUCT surfaces the charitable story now OWNS —
#       join / contribution / corpus / rank copy is the public good, not the
#       credit plane; it left the bar. "mine" specifically is still covered:
#       bar 3 hard-zeroes the mining machinery (/mining/ wire,
#       mining_consent, kat_meter, …) and the removed-surface check
#       below stays (--mine/--unmine are UNKNOWN flags since the
#       2026-11-06 vocabulary law — issue 192 GAP 10). Word boundaries on single terms (the T14a record: a
#       naive substring trips the tagline's "learns"); "epoch settle" is a
#       PHRASE (optional space/punct between). grep rc is adjudicated:
#       1 = clean PASS, 0 = hits FAIL, >=2 = grep error FAIL — a broken
#       grep is never a pass.
#       ⛔ POSITIVE ASSERTIONS (join/points/corpus surfaces PRESENT and
#       consent-gated) are DEFERRED to T16.5's implementation task row — the
#       surfaces do not exist in this build yet; asserting them today would
#       red on the correct posture.
#       DISCLOSURE-CLASS EXEMPTION (T14a's recorded carve-out): the build
#       stamp's release-set STALE disclosure + rebuild hint name the missing
#       features BY DESIGN (posture honesty). Exempt lines are printed
#       verbatim, counted, and CAPPED — the class growing is a loud FAIL,
#       never a silent pass.
#   (2) `cargo tree -e features` — riir-kat resolves at its redact + join
#       identity-chain tier ONLY (Plan 325 T6 re-scope); no
#       transport/service/billing feature resolved. Issue 221: the UNGATED
#       protocol tier is vendored in-repo (src/katplane/), so the DEFAULT
#       build resolves ZERO private deps — riir-kat re-enters only through
#       points' identity chain, which is exactly the tier this bar blesses.
#       Bar 3's machinery tokens stay zero: the linker strips the unused
#       vendored soft_gate/mining_consent bodies (uses are kat-gated).
#   (3) binary strings scan — hard-zero classes (X-Kat- headers, KAT host
#       literals, /mining/ wire, soft-gate/burn/meter machinery tokens) plus
#       two disclosed classes:
#         R1  help/reference/refusal text carrying kat_service|kat_transport
#             (the --help-all maintenance listings + typed-refusal texts —
#             none reachable from the bar-1 surfaces); disclosed + CAPPED.
#         R2  RIIR_REFINE_KAT_* env-name strings, pinned BY NAME (census:
#             any name outside the pin fails — a new name never rides in).
#   (+) `--mine` refuses (the flag was REMOVED 2026-11-06 — unknown flag,
#       rc != 0) — the removal pin.
#   (+) scripts/binary_leak_scan.sh on the artifact (the shipping gate).
#
# ⛔ The STRICT bar (zero kat_* crates in the resolved graph) is OWNER-GATED
# residue (the kat_redact decision, T14a row) — deliberately NOT asserted
# here. Check (2)'s feature-free+redact shape is the blessed regular bar.
#
# Usage: scripts/charitable_gate.sh
# Env:
#   CHARITABLE_TARGET_DIR  build dir. Default: a fresh mktemp -d per run
#                          (hermetic; the repo target/ is deliberately NOT
#                          shared — a charitable build must never pollute or
#                          contend with the release-set cache). Override for
#                          iteration; rm it when done.
#   CHARITABLE_KEEP=1      keep the build dir even on success.
# Exit: 0 all bars pass · 1 any bar fails · 2 usage/environment
# ─────────────────────────────────────────────────────────────────────────────
set -u

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root" || exit 2

FEATURES="latent_retrieval,rust_perf,clippy_lints,points,net-mainnet"
# ^ Plan 325 T3/T6 (issue 192): the charitable set gains `points` — the
# public-goods plane IS the public build's network surface (the owner's v4
# pivot). The KAT features stay out (bar 2 keeps kat_transport/kat_service
# hard-zero); the JOIN identity chain (riir-kat provider_login → rebind →
# client → account_key) is now BLESSED — the GAP 5 decision (the existing
# OAuth app compiles as the default so join works out of the box).
# Bar-1 output pattern — payment/token terms ONLY (issue 192 GAP 2 re-scope,
# 2026-11-01): the wide pattern's product-collision half (mine/earn/epoch/
# settle/credit/join/service) left with the v4 pivot; the mining machinery
# stays hard-zeroed at bar 3 + the removed-surface pin stays (the typed
# refusal became an unknown-flag refusal 2026-11-06). Single terms
# word-boundaried; "epoch settle" a phrase with optional separator.
PATTERN='(^|[^A-Za-z])(kat|payout|lease|pack|usdc|wallet|topup)([^A-Za-z]|$)|epoch[^A-Za-z]?settle'
# Bar-1 disclosure class (T14a's recorded carve-out): the build stamp's
# release-set STALE disclosure + its rebuild hint. Anchored to the exact line
# prefixes; anything else matching the wide pattern still fails.
# Issue 221 second class: the `features:` census line names cargo's COMPILED
# feature set verbatim — the optional dep's implicit feature (riir-kat, from
# the `riir-kat/<feat>` feature rows) shows there in every posture that
# resolves the identity chain. Honest metadata, gated by bar 2 (the tree);
# disclosed + CAPPED at 1 (the line is one line — a second one is a lie).
VERSION_EXEMPT='^release set: STALE|^rebuild: cargo build|^features: '
VERSION_EXEMPT_CAP=3
# Bar-3 hard-zero classes: transport wire · KAT-era machinery · mining path ·
# DEVNET hosts. Token-shaped (never a bare `kat`) so the
# katgpt-core dep tree cannot false-positive. Zero hits expected — any hit
# fails with no exemption path.
# HOST-CLASS SPLIT (2026-11-04, the net-mainnet release pin): MAINNET hosts
# (mainnet.riir.ai / tuna.riir.ai / riir.ai + gen-1 aliases) moved OUT of
# hard-zero — the pin's own allowed-set + refusal diagnostics legitimately
# carry them (the pin NAMES what it allows; issue 190 T14's class was the
# KAT-era economy surface, which the pin build compiles out entirely). The
# DEVNET hosts stay HARD-ZERO — the owner's mainnet-only directive: the
# shipped binary carries no dialable devnet host at the byte level
# (compile-to-nothing, riir-kat kat_service_urls).
SYMBOL_HARD='X-Kat-|devnet\.riir\.ai|devnet\.gist\.rs|devnet\.ai\.gist\.rs|/mining/|soft_gate|kat_burn|burn_ack|burn_client|mining_consent|kat_meter|kat_charge|kat_recovery|kat_admin|tuna_admin|KAT_SERVICE|KAT_TRANSPORT'
# MAINNET hosts — disclosed + capped (the net-pin allowed-set diagnostics;
# every occurrence is the pin naming what it allows or the honest refusal
# text). The class growing past the cap is a loud FAIL.
MAINNET_HOST_CAP=12
# R1 — the disclosed help/reference class (reason per row class):
#   --help-all maintenance listings + long-form command help fragments +
#   typed-refusal texts naming the missing features (the same honest-detector
#   class the build-stamp refusals carry) + the build-stamp release-set const blob.
# Disclosed (printed verbatim) + capped; the class growing past the cap is a
# loud FAIL — re-adjudicate before raising it.
R1_PATTERN='kat_service|kat_transport'
R1_CAP=40
# R2 — RIIR_REFINE_KAT_* env-name strings, pinned BY NAME with reasons:
#   RIIR_REFINE_KAT_ACCOUNT_DIR  local config/account path const (no network)
#   RIIR_REFINE_KAT_CRANK_TOKEN  doc fragment from the crank/mine reference
#   RIIR_REFINE_KAT_KEY          the account-key PATH override the join/login
#                                substrate reads (kat_account login_default_in;
#                                a local file path, never a network dial)
#   RIIR_REFINE_KAT_LEASE_DIR    the corpus-lease store's local dir override
#                                (riir-kat/client's LeaseStore — the local
#                                .refine/corpus-lease files the header's
#                                corpus-epoch line reads; a local path, never
#                                a network dial; rode in with the points
#                                feature's riir-kat/client edge, 2026-11-04)
#   ⛔ RIIR_REFINE_KAT_SERVICE_URL stays OUT — deliberately NOT a pin: the
#      points posture's lease read resolves through the points plane's own
#      env (issue 203); the KAT dial-law env name contains KAT_SERVICE as a
#      substring, the bar-3 HARD-ZERO class — a name that can never ride in
#      (measured: pulling the kat dial seam in tripped bar 3, 2026-11-04).
# The census extracts EVERY RIIR_REFINE_KAT_* name in the binary; a name
# outside this pin fails even when its carrying line looks inert.
R2_PINS='RIIR_REFINE_KAT_ACCOUNT_DIR
RIIR_REFINE_KAT_CRANK_TOKEN
RIIR_REFINE_KAT_KEY
RIIR_REFINE_KAT_LEASE_DIR'

fail=0
note() { printf '%s\n' "$*"; }
bar_fail() { fail=1; note "  FAIL: $*"; }

# ── adjudicated surface grep with a capped disclosure-class exemption ────────
scan_surface() {
    label=$1
    outfile=$2
    exempt_regex=$3
    exempt_cap=$4
    hits_file=$(mktemp) || { bar_fail "$label: mktemp failed"; return; }
    grep -i -E "$PATTERN" "$outfile" >"$hits_file"
    rc=$?
    if [ "$rc" -eq 1 ]; then
        note "  ok: $label — zero pattern hits"
        rm -f "$hits_file"
        return
    fi
    if [ "$rc" -gt 1 ]; then
        bar_fail "$label — grep itself failed (rc=$rc); never a silent pass"
        rm -f "$hits_file"
        return
    fi
    # rc 0 — hits present; apply the disclosure class.
    exempt_file=$(mktemp) || { bar_fail "$label: mktemp failed"; return; }
    net_file=$(mktemp) || { bar_fail "$label: mktemp failed"; return; }
    grep -i -E "$exempt_regex" "$hits_file" >"$exempt_file"
    grep -i -v -E "$exempt_regex" "$hits_file" >"$net_file"
    exempt_n=$(wc -l <"$exempt_file" | tr -d ' ')
    if [ -s "$net_file" ]; then
        bar_fail "$label — pattern hits outside the disclosure class:"
        sed 's/^/       /' "$net_file" | sed -n '1,8p'
    elif [ "$exempt_n" -gt "$exempt_cap" ]; then
        bar_fail "$label — disclosure class grew to $exempt_n lines (cap $exempt_cap); re-adjudicate"
    else
        note "  ok: $label — $exempt_n disclosure-class line(s) exempt (disclosed below)"
        sed 's/^/       EXEMPT: /' "$exempt_file"
    fi
    rm -f "$hits_file" "$exempt_file" "$net_file"
}

# ── build dir ────────────────────────────────────────────────────────────────
if [ "${CHARITABLE_TARGET_DIR:-}" = "" ]; then
    CHARITABLE_TARGET_DIR=$(mktemp -d) || exit 2
    built_fresh=1
else
    built_fresh=0
fi
cleanup() {
    if [ "$built_fresh" -eq 1 ] && [ "$fail" -eq 0 ] && [ "${CHARITABLE_KEEP:-}" != "1" ]; then
        rm -rf "$CHARITABLE_TARGET_DIR"
    fi
}

note "== charitable_gate: building posture default+$FEATURES"
note "   target dir: $CHARITABLE_TARGET_DIR"
RUSTFLAGS="--remap-path-prefix $HOME=/build" \
    CARGO_TARGET_DIR="$CHARITABLE_TARGET_DIR" \
    cargo build --profile dist --features "$FEATURES" --bin cargo-refine || {
        note "charitable_gate: FAIL — charitable build does not compile (build dir kept: $CHARITABLE_TARGET_DIR)"
        exit 1
    }
BIN="$CHARITABLE_TARGET_DIR/dist/cargo-refine"
[ -f "$BIN" ] || { note "charitable_gate: FAIL — binary missing at $BIN"; exit 1; }

work=$(mktemp -d) || exit 2

# ── bar 1: the four CLI output surfaces ─────────────────────────────────────
note "== bar 1: CLI output grep (word-boundaried, case-insensitive)"
"$BIN" --help >"$work/help.out" 2>&1
scan_surface "--help" "$work/help.out" '__never_matches__' 0
"$BIN" --version >"$work/version.out" 2>&1
scan_surface "--version" "$work/version.out" "$VERSION_EXEMPT" "$VERSION_EXEMPT_CAP"
printf 'fn main() { println!("hello"); }\n' >"$work/sample.rs"
( cd "$work" && RIIR_REFINE_AUTO_ORACLE=0 "$BIN" ) >"$work/dryrun.out" 2>&1
scan_surface "bare dry-run" "$work/dryrun.out" '__never_matches__' 0
( cd "$work" && RIIR_REFINE_AUTO_ORACLE=0 "$BIN" --fix --no-verify sample.rs ) >"$work/scan.out" 2>&1
scan_surface "--fix scan" "$work/scan.out" '__never_matches__' 0

# ── bar 2: cargo tree — riir-kat at redact + the JOIN identity chain ONLY ───
note "== bar 2: cargo tree -e features (charitable set)"
CARGO_TARGET_DIR="$CHARITABLE_TARGET_DIR" \
    cargo tree -e features -i riir-kat --features "$FEATURES" >"$work/tree_inv.out" 2>&1
inv_rc=$?
if [ "$inv_rc" -ne 0 ]; then
    bar_fail "cargo tree -i riir-kat failed (rc=$inv_rc):"
    sed 's/^/       /' "$work/tree_inv.out" | sed -n '1,8p'
else
    kat_features=$(grep -o 'riir-kat feature "[A-Za-z_]*"' "$work/tree_inv.out" | sort -u)
    # Plan 325 T6 re-scope (issue 192 GAP 5): redact + the join identity
    # chain are the blessed tier — the points plane's identity surface. The
    # KAT wire/billing features (transport/service) stay hard-zero below.
    blessed=$(printf '%s\n' "$kat_features" | grep -oE '"[a-z_]+"' | tr -d '"' | sort | tr '\n' ' ')
    case " $blessed " in
        *" redact "*) ;;
        *) bar_fail "riir-kat resolves without redact (got: $blessed)" ;;
    esac
    for f in client account_key rebind provider_login; do
        case " $blessed " in
            *" $f "*) ;;
            *) bar_fail "the join identity chain is INCOMPLETE — riir-kat feature '$f' missing (got: $blessed)" ;;
        esac
    done
    # The forbidden tier: anything the KAT wire/billing plane owns. Pinned
    # BY NAME (a new kat feature lands here only by an explicit edit).
    # lease_http (issue 203): the corpus-lease READ transport — GET-only
    # (/lease + /segment), no /sync, no /claim, no signing, no QR; the
    # money surface stays kat_transport-only by construction.
    forbidden_tier=""
    for f in $blessed; do
        case "$f" in
            redact|client|account_key|rebind|provider_login|github_login|lease_http) ;;
            *) forbidden_tier="$forbidden_tier $f" ;;
        esac
    done
    if [ -n "$forbidden_tier" ]; then
        bar_fail "riir-kat resolves features beyond redact + the identity chain:"
        printf '%s\n' $forbidden_tier | sed 's/^/       /'
    else
        note "  ok: riir-kat resolves at redact + the join identity chain only"
    fi
fi
CARGO_TARGET_DIR="$CHARITABLE_TARGET_DIR" \
    cargo tree -e features --features "$FEATURES" >"$work/tree_fwd.out" 2>&1
fwd_rc=$?
if [ "$fwd_rc" -ne 0 ]; then
    bar_fail "cargo tree -e features failed (rc=$fwd_rc):"
    sed 's/^/       /' "$work/tree_fwd.out" | sed -n '1,8p'
else
    forbidden=$(grep -E 'riir-kat feature "(kat_transport|kat_service|siwr|se_custody|github_pow|admin|totp|genesis|decstat|purge|pay|migrate|recovery|github_login_pow)"' "$work/tree_fwd.out")
    if [ -z "$forbidden" ]; then
        note "  ok: no KAT wire/billing feature resolved"
    else
        bar_fail "forbidden riir-kat features in the resolved graph:"
        printf '%s\n' "$forbidden" | sed 's/^/       /' | sed -n '1,8p'
    fi
fi

# ── bar 3: binary strings scan ───────────────────────────────────────────────
note "== bar 3: binary strings scan"
strings "$BIN" >"$work/strings.txt"
# Hard-zero classes first — no exemption path.
hard_hits=$(grep -E "$SYMBOL_HARD" "$work/strings.txt" || true)
if [ -n "$hard_hits" ]; then
    bar_fail "hard-zero class strings in the binary (wire/host/machinery/devnet hosts):"
    printf '%s\n' "$hard_hits" | sort -u | sed 's/^/       /' | sed -n '1,12p'
else
    note "  ok: zero hard-zero-class strings (X-Kat- / devnet hosts / /mining/ / machinery)"
fi
# MAINNET host class — disclosed + capped (the net-pin allowed-set + the
# refusal text naming what it allows; the pin build's honest diagnostics).
mainnet_hits=$(grep -cE 'mainnet\.riir\.ai|tuna\.riir\.ai|https://riir\.ai|mainnet\.gist\.rs|ai\.gist\.rs' "$work/strings.txt" || true)
if [ "$mainnet_hits" -gt "$MAINNET_HOST_CAP" ]; then
    bar_fail "mainnet host strings exceed the disclosed cap ($mainnet_hits > $MAINNET_HOST_CAP) — the pin's diagnostics grew; re-adjudicate"
else
    note "  ok: mainnet host strings $mainnet_hits <= cap $MAINNET_HOST_CAP (the pin's allowed-set + refusal text)"
fi
# BUILD-PATH class — disclosed + capped (2026-11-04): panic Location strings
# carry the module's (remapped) source path — /build/git/riir-kat/src/... —
# $HOME is remapped away; the residual names the repo TREE only (no user, no
# host). The class growing past the cap is a loud FAIL. ⛔ the kat_protocol
# match on these lines is the FILE NAME, not machinery wire — the hard-zero
# class dropped kat_protocol for exactly this adjacency (a path is not a
# dial; the machinery classes that MATTER stay hard-zero above).
path_hits=$(grep -E 'git/riir-kat/src/|git/riir-refine/src/' "$work/strings.txt" | sort -u)
if [ -n "$path_hits" ]; then
    n_paths=$(printf '%s\n' "$path_hits" | wc -l | tr -d ' ')
    if [ "$n_paths" -gt 12 ]; then
        bar_fail "build-path strings exceed the disclosed cap ($n_paths > 12) — re-adjudicate"
    else
        note "  ok: build-path strings $n_paths <= cap 12 (remapped panic Locations; no user, no host):"
        printf '%s\n' "$path_hits" | sed 's/^/       /' | sed -n '1,6p'
    fi
else
    note "  ok: zero build-path strings"
fi
# R2 census: every RIIR_REFINE_KAT_* occurrence classified against the pin.
# ⛔ .rodata byte-adjacency merges consts (strings sees
# `RIIR_REFINE_KAT_ACCOUNT_DIRXDG_CONFIG_HOME` — no NUL between them), so a
# row is in-pin when it EQUALS a pin, or begins with a pin and the remainder
# carries no second KAT token (any other RIIR_REFINE_KAT_ name contains one).
# Everything else is extra and fails; every pin must also be PRESENT.
r2_names=$(grep -o 'RIIR_REFINE_KAT_[A-Z_]*' "$work/strings.txt" | sort -u)
r2_pin_file="$work/r2_pins.txt"
printf '%s\n' "$R2_PINS" | grep -v '^$' >"$r2_pin_file"
pins_found=0
pins_expected=0
while IFS= read -r pin; do
    [ -z "$pin" ] && continue
    pins_expected=$((pins_expected + 1))
    if printf '%s\n' "$r2_names" | grep -q -F "$pin"; then
        pins_found=$((pins_found + 1))
    else
        bar_fail "R2 pin missing from the binary: $pin (the binary shape changed, re-adjudicate)"
    fi
done <<R2PINS
$R2_PINS
R2PINS
r2_exact=$(printf '%s\n' "$r2_names" | grep -F -x -f "$r2_pin_file" || true)
printf '%s\n' "$r2_exact" | grep -v '^$' >"$work/r2_exact.txt" || true
r2_merged=$(printf '%s\n' "$r2_names" | grep -E '^RIIR_REFINE_KAT_(ACCOUNT_DIR|CRANK_TOKEN)[A-Z_]*$' || true)
r2_merged=$(printf '%s\n' "$r2_merged" | grep -v -E '^RIIR_REFINE_KAT_[A-Z_]*KAT' || true)
# a merged-candidate row that is ALSO an exact pin is an exact match, not an artifact
if [ -s "$work/r2_exact.txt" ]; then
    r2_merged=$(printf '%s\n' "$r2_merged" | grep -F -v -x -f "$work/r2_exact.txt" || true)
fi
{ [ -n "$r2_exact" ] && printf '%s\n' "$r2_exact"; [ -n "$r2_merged" ] && printf '%s\n' "$r2_merged"; } >"$work/r2_in.txt" 2>/dev/null || true
if [ -s "$work/r2_in.txt" ]; then
    r2_extra=$(printf '%s\n' "$r2_names" | grep -F -v -x -f "$work/r2_in.txt" || true)
else
    r2_extra="$r2_names"
fi
if [ -n "$r2_extra" ]; then
    bar_fail "RIIR_REFINE_KAT_* names outside the R2 pin:"
    printf '%s\n' "$r2_extra" | sed 's/^/       /'
else
    if [ -n "$r2_merged" ]; then
        note "  ok: R2 census — $pins_found/$pins_expected pin(s) present; merged-const artifact(s) disclosed:"
        printf '%s\n' "$r2_merged" | sed 's/^/       MERGED: /'
    else
        note "  ok: R2 census — $pins_found/$pins_expected pin(s) present, exact matches only"
    fi
fi
# R1 disclosed class: what remains after hard-zero + R2 removal.
r1_hits=$(grep -E "$R1_PATTERN" "$work/strings.txt" || true)
r1_n=$(printf '%s\n' "$r1_hits" | grep -c . || true)
if [ "$r1_n" -gt "$R1_CAP" ]; then
    bar_fail "R1 disclosure class grew to $r1_n strings (cap $R1_CAP); re-adjudicate"
else
    note "  ok: R1 disclosure class at $r1_n string(s) (cap $R1_CAP), samples disclosed:"
    printf '%s\n' "$r1_hits" | sort -u | sed 's/^/       R1: /' | sed -n '1,8p'
fi

# ── +: the removed mine surface stays gone (the 2026-11-06 vocabulary
# law — issue 192 GAP 10; replaced the historical "--mine refuses typed"
# arm, whose flag no longer exists).
note "== +: the removed mine surface stays gone"
( cd "$work" && "$BIN" --mine ) >"$work/mine.out" 2>&1
mine_rc=$?
if [ "$mine_rc" -eq 0 ]; then
    bar_fail "--mine exited 0 at the charitable posture — the removed surface is back"
else
    if grep -q 'unknown flag' "$work/mine.out"; then
        note "  ok: --mine is an unknown flag now (rc=$mine_rc) — the surface stays removed"
    else
        # A typed build-stamp refusal would also be acceptable; what must
        # NOT happen is a working command or a crash.
        note "  ok: --mine refuses (rc=$mine_rc, non-typed refusal — still gone)"
    fi
fi

# ── +: the points-plane POSITIVE assertions (Plan 325 T6, issue 192 GAP 2's
# deferred half) — the public-goods surface PRESENT and consent/offline-
# gated. Every arm runs OFFLINE (the env speaks for the user; nothing
# dials) with the account dir pointed INSIDE $work (no machine state is
# touched). Presence + gating asserted; network behavior is P2's.
note "== +: points surfaces present + gated (offline arms)"
export RIIR_REFINE_OFFLINE=1
export RIIR_REFINE_KAT_ACCOUNT_DIR="$work/points-account"
( cd "$work" && "$BIN" join ) >"$work/join.out" 2>&1
join_rc=$?
if [ "$join_rc" -eq 2 ] && grep -q 'offline: join skipped' "$work/join.out"; then
    note "  ok: join present, offline-refused typed (rc=2, names the env)"
else
    bar_fail "join: expected the typed offline refusal (rc=2 + 'offline: join skipped'), got rc=$join_rc:"
    sed 's/^/       /' "$work/join.out" | sed -n '1,6p'
fi
for verb in rank info; do
    ( cd "$work" && "$BIN" "$verb" ) >"$work/$verb.out" 2>&1
    verb_rc=$?
    if [ "$verb_rc" -eq 0 ] && grep -q "offline: $verb skipped" "$work/$verb.out"; then
        note "  ok: $verb present, offline-degraded (rc=0, names the env)"
    else
        bar_fail "$verb: expected the offline degradation (rc=0 + 'offline: $verb skipped'), got rc=$verb_rc:"
        sed 's/^/       /' "$work/$verb.out" | sed -n '1,6p'
    fi
done
( cd "$work" && "$BIN" leave ) >"$work/leave.out" 2>&1
leave_rc=$?
if [ "$leave_rc" -eq 0 ] && grep -q 'local state cleared' "$work/leave.out"; then
    note "  ok: leave present, offline-safe (local state cleared, nothing dialed)"
else
    bar_fail "leave: expected the offline-safe local clear (rc=0), got rc=$leave_rc:"
    sed 's/^/       /' "$work/leave.out" | sed -n '1,6p'
fi
( cd "$work" && "$BIN" version ) >"$work/version_cmd.out" 2>&1
if grep -q 'cargo-refine' "$work/version_cmd.out"; then
    note "  ok: version subcommand present (aliases --version)"
else
    bar_fail "version subcommand produced no version block:"
    sed 's/^/       /' "$work/version_cmd.out" | sed -n '1,6p'
fi
( cd "$work" && "$BIN" corpus ) >"$work/corpus.out" 2>&1
corpus_rc=$?
if [ "$corpus_rc" -eq 0 ] && grep -q 'public good' "$work/corpus.out"; then
    note "  ok: corpus present (points-free framing)"
else
    bar_fail "corpus: expected the points-free info block (rc=0), got rc=$corpus_rc:"
    sed 's/^/       /' "$work/corpus.out" | sed -n '1,6p'
fi
# The consent text (T16.1 §5) — the load-bearing sentences VERBATIM in the
# binary (the fix-path ask is the consent surface; the words are law).
for phrase in 'a random ID we generate on your' 'we never read your hardware'; do
    if grep -q -F "$phrase" "$work/strings.txt"; then
        note "  ok: consent text pinned verbatim ('${phrase}…')"
    else
        bar_fail "consent sentence missing from the binary: '$phrase' (the T16.1 §5 words are law)"
    fi
done
# The anti-Sybil rank sentence (Proposal 026 §5.1).
if grep -q -F 'joined identities only' "$work/strings.txt"; then
    note "  ok: anti-Sybil rank sentence pinned"
else
    bar_fail "anti-Sybil sentence missing ('joined identities only')"
fi
unset RIIR_REFINE_OFFLINE RIIR_REFINE_KAT_ACCOUNT_DIR

# ── +: the shipping leak-scan on the same artifact ───────────────────────────
note "== +: binary_leak_scan on the charitable artifact"
sh "$repo_root/scripts/binary_leak_scan.sh" "$BIN" || bar_fail "binary_leak_scan failed on the charitable artifact"

rm -rf "$work"
cleanup
if [ "$fail" -ne 0 ]; then
    note "CHARITABLE_GATE: FAIL — issue 190 T14 bar not met (build dir kept: $CHARITABLE_TARGET_DIR)"
    exit 1
fi
note "CHARITABLE_GATE: PASS — issue 190 T14 regular bar met (Posture A shape verified)"
