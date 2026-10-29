#!/bin/sh
# ─────────────────────────────────────────────────────────────────────────────
# Proposal 003 release lane — the binary size CEILING gate (2026-09-09).
#
# Usage: scripts/binary_size_gate.sh <binary> [<binary> ...]
# Env:   MAX_BINARY_BYTES (default 6815744 = 6.5 MiB)
#
# Fails loud (exit 1) if ANY scanned binary exceeds the ceiling. A CEILING,
# not a pin — it fires UPWARD only: legitimate feature growth bumps the
# constant deliberately, while a silent jump (an accidental heavy dep, a
# lost --profile dist, a dev build in the artifact slot) reds before publish.
# Measured basis (2026-09-09, arm64-apple-darwin, the v0.1.4 shipping feature
# set): [profile.dist] binary = 4,546,448 B; the same set on plain --release
# = 11,903,168 B — the gate exists to catch exactly that regression shape.
#
# The default ceiling is PROVISIONAL: only one of the five release-matrix
# targets is measured so far. 6.5 MiB ≈ measured + ~50% headroom for the
# unmeasured rows (musl is typically smaller, msvc sometimes larger).
# After the first --profile dist release, tighten (or split per-target) from
# the real numbers — do not raise it to silence a red without a measurement.
#
# Style note: set -u WITHOUT errexit and NO EXIT trap — same shape as
# binary_leak_scan.sh; an aborting check must exit non-zero honestly, and
# no trap means no trap-laundering class (katgpt-rs Issue 734).
# ─────────────────────────────────────────────────────────────────────────────
set -u

# Ceiling history (the provisionality is DISCHARGED — all five matrix targets
# measured at the v0.2.0 release, 2026-10-01, from the tree at fdc74823):
#   aarch64-apple-darwin        6,897,024 B  (6.58 MiB)
#   x86_64-apple-darwin         7,900,304 B  (7.53 MiB)
#   x86_64-unknown-linux-musl   8,599,232 B  (8.20 MiB)  ← max
#   aarch64-unknown-linux-musl  7,522,576 B  (7.18 MiB)
#   x86_64-pc-windows-gnu       8,081,408 B  (7.71 MiB)
# The growth from v0.1.4's 4.3 MiB is a month of legitimate content
# (kernel_opt 684→717 rules + the whole kat staking/decstat/TUNA plane).
# 9.0 MiB = max + ~10% corpus-growth headroom, still far under the
# ~11.3 MiB dist-profile-loss signature the gate exists to catch.
# v0.3.2 re-measure (2026-10-17, the five-matrix cut): the +10% corpus
# headroom above the v0.2.0 basis was SPENT — musl measured 9,326,392 B
# (8.89 MiB) at v0.3.1 and 10,024,040 B (9.56 MiB) here (+7.5%, one
# release of legitimate corpus growth: kernel_opt 795→848, rust_perf
# 198→230, clippy 100→112); windows-gnu 8,806,400 → 9,468,928 (same
# +7.5%). Ceiling 10.5 MiB = the v0.3.2 max + ~10% headroom for the next
# release of corpus growth, still far under the dist-loss signature.
# v0.3.4 re-measure (2026-10-29, the gen-2 mainnet client): the 10.5 MiB
# ceiling RED on musl x86_64 at 11,574,072 B (11.04 MiB, +15.5% vs v0.3.3's
# 10,024,168 B) — measured BEFORE raising, per the rule above: the release
# feature set's dependency closure is IDENTICAL (151 crates at v0.3.3, 151
# here, zero added — `cargo tree -e normal --target x86_64-unknown-linux-musl`),
# and every target grew evenly (+15–18%), the shape of code growth, not of a
# lost dist profile (that is ~2.5x). The growth is riir-refine's own code +
# the three riir-kat features kat_transport now enables for the gen-2 client
# (github_pow_fetch, provider_login, se_custody — network-bound v2 signing,
# the account/custody surfaces). Basis: aarch64-apple-darwin 9,625,664 ·
# x86_64-apple-darwin 10,826,224 · x86_64-unknown-linux-musl 11,574,072 (max)
# · aarch64-unknown-linux-musl 10,276,856 · x86_64-pc-windows-gnu 10,974,208.
# Ceiling 12.0 MiB = the v0.3.4 max + ~9% headroom. The dist-loss signature
# is now ~2.5x the dist size (>20 MiB), far above it.
limit="${MAX_BINARY_BYTES:-12582912}"

file_size() {
    # BSD stat (macOS) first, GNU stat (Linux/CI) second; empty on failure —
    # the caller treats an unmeasurable file as FAIL (never silently 0).
    stat -f %z "$1" 2>/dev/null || stat -c %s "$1" 2>/dev/null || true
}

mib() {
    awk -v b="$1" 'BEGIN { printf "%.2f", b / 1048576 }'
}

run_gate() {
    fail=0
    for f in "$@"; do
        if [ ! -f "$f" ]; then
            echo "SIZE-GATE FAIL ($f): missing file"
            fail=1
            continue
        fi
        echo "== $f"
        size=$(file_size "$f")
        if [ -z "$size" ]; then
            echo "  FAIL: cannot determine file size"
            fail=1
        elif [ "$size" -gt "$limit" ]; then
            echo "  FAIL: $(mib "$size") MiB > ceiling $(mib "$limit") MiB — dist profile lost, or a heavy dep landed?"
            fail=1
        else
            echo "  ok: $(mib "$size") MiB <= ceiling $(mib "$limit") MiB"
        fi
    done
    if [ "$fail" -ne 0 ]; then
        echo "SIZE-GATE: FAIL — do not publish these artifacts"
        return 1
    fi
    echo "SIZE-GATE: PASS ($# binary arg(s) within $(mib "$limit") MiB)"
}

# --self-test: prove the detector fires both ways (over-ceiling reds,
# under-ceiling greens) without leaving files behind.
if [ "${1:-}" = "--self-test" ]; then
    tmp=$(mktemp -d) || exit 1
    small="$tmp/small.bin"
    big="$tmp/big.bin"
    : > "$small"
    dd if=/dev/zero of="$big" bs=1048576 count=12 2>/dev/null || { rm -rf "$tmp"; exit 1; }
    if run_gate "$small" >/dev/null 2>&1; then
        echo "self-test: under-ceiling PASS arm ok"
    else
        echo "self-test: FAIL — under-ceiling file reds the gate"; rm -rf "$tmp"; exit 1
    fi
    if run_gate "$big" >/dev/null 2>&1; then
        echo "self-test: FAIL — over-ceiling file passed the gate"; rm -rf "$tmp"; exit 1
    else
        echo "self-test: over-ceiling FAIL arm ok"
    fi
    rm -rf "$tmp"
    echo "self-test: PASS"
    exit 0
fi

[ $# -ge 1 ] || { echo "usage: $0 <binary> [<binary> ...]  (or --self-test)" >&2; exit 2; }
run_gate "$@"
exit $?
