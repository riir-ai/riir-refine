#!/bin/sh
# Issue 212 T1 — the G5 cost leg for an ARBITRARY domain feature.
#
# Consumes the standing harness (examples/kernel_goat_g5.rs — its timed
# region is the production LatentMatcher over every compiled domain) at two
# builds: BASE and BASE+<feature>. The arithmetic is scripts/g5_cost_math.py
# (one copy, shared with g5_utility_cost.sh). The bar is Issue 191 bar 5,
# verbatim: added ON-OFF scan latency per file, per-file medians over the
# harness's 3 timed passes, nearest-rank p50 <= 1.0 ms AND p95 <= 5.0 ms.
#
# Interleaving (the katgpt-rs sequential-A/B law): the two binaries run in
# ROUNDS, alternating order (odd rounds OFF first, even rounds ON first);
# every round is a full instance of the bar and the verdict needs ALL rounds.
# The runner refuses unless the two build stamps differ by the feature.
#
# Usage: sh scripts/g5_domain_cost.sh --feature sec [--base latent_retrieval]
#          [--root src] [--top-k 5] [--rounds 3] [--out-dir DIR] [--skip-build]
# --base: the scan posture the feature joins (default: latent_retrieval on
# top of the crate defaults — the shipped scan surface's matcher).
set -eu
cd "$(dirname "$0")/.."

FEATURE=""
BASE="latent_retrieval"
ROOT="src"
TOP_K="5"
ROUNDS="3"
OUT_DIR=""
SKIP_BUILD=0
while [ $# -gt 0 ]; do
    case "$1" in
        --feature) FEATURE="$2"; shift 2 ;;
        --base) BASE="$2"; shift 2 ;;
        --root) ROOT="$2"; shift 2 ;;
        --top-k) TOP_K="$2"; shift 2 ;;
        --rounds) ROUNDS="$2"; shift 2 ;;
        --out-dir) OUT_DIR="$2"; shift 2 ;;
        --skip-build) SKIP_BUILD=1; shift ;;
        *) echo "unknown arg $1" >&2; exit 2 ;;
    esac
done
[ -n "$FEATURE" ] || { echo "--feature is required" >&2; exit 2; }

TD="${OUT_DIR:-$(mktemp -d)}"
mkdir -p "$TD"
echo "g5 domain cost [$FEATURE] work dir: $TD | base: $BASE | root: $ROOT | rounds: $ROUNDS"
echo "box: $(uname -sm) | $(uptime)"

if [ "$SKIP_BUILD" -eq 0 ]; then
    CARGO_TARGET_DIR="$TD/t_off" cargo build --release --example kernel_goat_g5 \
        --features "$BASE" 2>"$TD/off_build.log"
    CARGO_TARGET_DIR="$TD/t_on" cargo build --release --example kernel_goat_g5 \
        --features "$BASE,$FEATURE" 2>"$TD/on_build.log"
fi
bin_of() {
    if [ -x "$1/release/examples/kernel_goat_g5.exe" ]; then
        echo "$1/release/examples/kernel_goat_g5.exe"
    else
        echo "$1/release/examples/kernel_goat_g5"
    fi
}
OFF_BIN=$(bin_of "$TD/t_off")
ON_BIN=$(bin_of "$TD/t_on")

PAIRS=""
r=1
while [ "$r" -le "$ROUNDS" ]; do
    if [ $((r % 2)) -eq 1 ]; then
        "$OFF_BIN" --root "$ROOT" --top-k "$TOP_K" --out "$TD/off_$r.json" >/dev/null
        "$ON_BIN" --root "$ROOT" --top-k "$TOP_K" --out "$TD/on_$r.json" >/dev/null
    else
        "$ON_BIN" --root "$ROOT" --top-k "$TOP_K" --out "$TD/on_$r.json" >/dev/null
        "$OFF_BIN" --root "$ROOT" --top-k "$TOP_K" --out "$TD/off_$r.json" >/dev/null
    fi
    echo "round $r done: $(uptime)"
    PAIRS="$PAIRS $TD/off_$r.json $TD/on_$r.json"
    r=$((r + 1))
done

# shellcheck disable=SC2086
python3 scripts/g5_cost_math.py --feature "$FEATURE" $PAIRS
