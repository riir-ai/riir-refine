#!/bin/sh
# ─────────────────────────────────────────────────────────────────────────────
# Proposal 003 G2 / Plan 105 T2.5 — the binary leak-scan gate.
#
# Usage: scripts/binary_leak_scan.sh <binary> [<binary> ...]
#
# Fails loud (exit 1) if ANY scanned binary has:
#   1. a symbol table        — `nm` must fail: [profile.release] strip = true
#                              did not run, or someone shipped a dev build
#   2. absolute machine paths — /Users/... /home/... or a drive letter
#                              (D:\, D:/) — --remap-path-prefix did not cover
#                              the build host (measured 2026-09-06: the
#                              unremapped shipping binary carried 61 such
#                              strings: registry + sibling-source paths)
#   3. secret-shaped strings  — ghp_ / github_pat_ / AKIA / sk- / xox[baprs]-
#
# Deliberately NOT checked (proposal-ACCEPTABLE surfaces, Proposal 003
# §Binary leak-surface checklist): CWD-relative src/... panic paths (module
# names only, no source) and corpus capability texts (--suggest output is
# the WHAT, not the HOW). Do not add checks for them here without amending
# Proposal 003 first.
# ─────────────────────────────────────────────────────────────────────────────
set -u

[ $# -ge 1 ] || { echo "usage: $0 <binary> [<binary> ...]" >&2; exit 2; }

fail=0
for f in "$@"; do
    if [ ! -f "$f" ]; then
        echo "LEAK-SCAN FAIL ($f): missing file"
        fail=1
        continue
    fi
    echo "== $f"

    # 1. Symbol table — a stripped binary carries NO defined symbols. macOS
    #    keeps a few harmless artifacts even after `strip = true`: the dyld
    #    undefined-import list (` U _write` etc. — libc ABI, not our
    #    architecture), the `__mh_execute_header` linker marker at the
    #    fixed image base, and (on 1.99 toolchain builds since the dep-tree
    #    re-resolve of 2026-11-04) std's own `_rust_eh_personality` abort
    #    shim — a compiler symbol with no user data (proven: the v0.4.0 TAG
    #    rebuild reproduces it; the published v0.4.0 predates the delta).
    #    All three are allowlisted; anything else with an
    #    address (T/D/B/S/...) means the internal architecture is exposed.
    defined=$(nm "$f" 2>/dev/null | grep -E '^[0-9a-fA-F]{8,} [a-zA-Z] ' | grep -v -F '__mh_execute_header' | grep -v -F '_rust_eh_personality' || true)
    if [ -n "$defined" ]; then
        echo "  FAIL: symbol table present ($(printf '%s\n' "$defined" | wc -l | tr -d ' ') defined symbols) — strip did not run"
        printf '%s\n' "$defined" | sed 's/^/       /' | sed -n '1,8p'
        fail=1
    else
        echo "  ok: no defined symbols (imports/marker only)"
    fi

    # 2. Build-machine paths. Present tense = the remap prefix missed this
    #    host's layout. The drive-letter arm requires an UPPERCASE drive
    #    (MSVC/cargo bake C:\, D:\) + two path segments: measured on run
    #    34073335938, a lowercase form false-positives on the generic
    #    default-PATH constant baked by the musl runtime (`…/bin:/bin:/usr/bin`
    #    → `n:/usr/bin`), and a preceding-char boundary would REJECT the real
    #    leak (it occurs mid-string, e.g. `valueC:\Users\runneradmin`).
    #    Samples capped so a failure stays readable.
    if grep -a -q -E '/(Users|home)/' "$f"; then
        echo "  FAIL: absolute machine paths found, samples:"
        grep -a -o -E '/(Users|home)/[A-Za-z0-9._/-]+' "$f" | sort -u | sed 's/^/       /' | sed -n '1,8p'
        fail=1
    else
        echo "  ok: no /Users/ or /home/ paths"
    fi
    if grep -a -q -E '[A-Z]:[\\/][A-Za-z0-9_. -]{1,}[\\/][A-Za-z0-9_. -]{1,}' "$f"; then
        echo "  FAIL: drive-letter paths found, samples:"
        grep -a -o -E '[A-Z]:[\\/][A-Za-z0-9_. -]{1,}[\\/][A-Za-z0-9_. -]{1,}' "$f" | sort -u | sed 's/^/       /' | sed -n '1,8p'
        fail=1
    else
        echo "  ok: no drive-letter paths"
    fi

    # 3. Secret shapes. Any hit is a stop-the-line event, never a warning.
    #    Benign false positive, documented: the redactor's OWN prefix table
    #    (`PREFIX_RULES` in kat_redact.rs) lays its literal entries adjacent
    #    in .rodata — `xoxb-xoxa-xoxp-xoxr-xoxs-sk-` is the TABLE, not a
    #    token (a real Slack token has a random body after ONE prefix). The
    #    negative lookahead (no engine in POSIX grep -E) is emulated by
    #    excluding the exact table adjacency, and any hit that is not
    #    exactly that adjacency still fails.
    leaks=$(grep -a -o -E 'ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|sk-[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}' "$f" \
        | grep -v -E '^xoxb-xoxa-xoxp-xoxr-xoxs-sk-' || true)
    if [ -n "$leaks" ]; then
        echo "  FAIL: secret-shaped string(s) found"
        printf '%s\n' "$leaks" | sort -u | sed 's/^/       /' | sed -n '1,8p'
        fail=1
    else
        echo "  ok: no secret-shaped strings"
    fi
done

if [ "$fail" -ne 0 ]; then
    echo "LEAK-SCAN: FAIL — do not publish these artifacts"
    exit 1
fi
echo "LEAK-SCAN: PASS ($# binary arg(s) clean)"
