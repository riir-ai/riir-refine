#!/bin/sh
# Issue 191 bar 5 — the G5 utility + ON-posture cost leg (the Bench-124
# disclosed deferral, measured as its own bench).
#
# Runs the census+latency harness (examples/kernel_goat_g5.rs) in BOTH
# postures over the pinned corpus, computes the per-file medians and the
# ON-OFF added latency, and adjudicates the PRE-REGISTERED bars:
#   - added latency: p50 <= 1.0 ms/file, p95 <= 5.0 ms/file
#   - utility floor: >= 1 span fired through >= 3 distinct category experts
#
# Usage: sh scripts/g5_utility_cost.sh [--root src] [--top-k 5] [--out-dir <dir>]
# No cargo in this script beyond the two example builds (isolated target
# dir so a sibling build never holds the lock).

set -eu

ROOT="src"
TOP_K="5"
OUT_DIR=""
while [ $# -gt 0 ]; do
    case "$1" in
        --root) ROOT="$2"; shift 2 ;;
        --top-k) TOP_K="$2"; shift 2 ;;
        --out-dir) OUT_DIR="$2"; shift 2 ;;
        *) echo "unknown arg $1" >&2; exit 2 ;;
    esac
done

TD="${OUT_DIR:-$(mktemp -d)}"
mkdir -p "$TD"
echo "g5 work dir: $TD"
echo "box: $(uname -s) $(nproc 2>/dev/null || echo '?') cpus | free mem: $(vm_stat >/dev/null 2>&1 && echo darwin || wmic OS get FreePhysicalMemory /value 2>/dev/null | grep = || cat /proc/meminfo 2>/dev/null | grep MemAvailable || echo 'n/a')"

# ── the OFF posture (latent_retrieval for the matcher; kernel_opt OFF) ──
CARGO_TARGET_DIR="$TD/t_off" cargo build --release --example kernel_goat_g5 --features latent_retrieval 2>"$TD/off_build.log"
"$TD/t_off/release/examples/kernel_goat_g5.exe" --root "$ROOT" --top-k "$TOP_K" --out "$TD/g5_off.json"

# ── the ON posture (+kernel_opt) ─────────────────────────────────────────
CARGO_TARGET_DIR="$TD/t_on" cargo build --release --example kernel_goat_g5 --features latent_retrieval,kernel_opt 2>"$TD/on_build.log"
"$TD/t_on/release/examples/kernel_goat_g5.exe" --root "$ROOT" --top-k "$TOP_K" --out "$TD/g5_on.json"

# ── the adjudication ─────────────────────────────────────────────────────
python3 - "$TD/g5_off.json" "$TD/g5_on.json" <<'PYEOF'
import json
import statistics
import sys

off = json.load(open(sys.argv[1], encoding="utf-8"))
on = json.load(open(sys.argv[2], encoding="utf-8"))

if off["files"] != on["files"] or off["corpus_root"] != on["corpus_root"]:
    raise SystemExit("REFUSE: posture tables disagree on the corpus walk")

def medians(tbl):
    rows = tbl["per_pass_us"]
    paths = [p for p, _ in rows[0]]
    return paths, [statistics.median([rows[k][i][1] for k in range(len(rows))]) for i in range(len(paths))]

off_paths, off_med = medians(off)
on_paths, on_med = medians(on)
assert off_paths == on_paths, "file order drifted between postures"

added_us = [o - f for o, f in zip(on_med, off_med)]
added_ms = sorted(u / 1000.0 for u in added_us)
n = len(added_ms)

def pct(sorted_xs, p):
    # nearest-rank on the pre-sorted list; tail support disclosed
    idx = min(n - 1, max(0, round(p * n) - 1))
    return sorted_xs[idx], n - idx

p50, sup50 = pct(added_ms, 0.50)
p95, sup95 = pct(added_ms, 0.95)

print(f"files: {n}")
print(f"added latency ms/file: p50 {p50:.3f} (tail support {sup50}) | p95 {p95:.3f} (tail support {sup95})")
print(f"ceilings: p50 <= 1.0 ms | p95 <= 5.0 ms")

lat_ok = p50 <= 1.0 and p95 <= 5.0
print(f"cost leg: {'PASS' if lat_ok else 'FAIL'}")

if "utility_distinct_rank1_experts" in on:
    experts = on["utility_distinct_rank1_experts"]
    kspans = on["kernel_spans_last_pass"]
    r1 = on["rank1_by_expert"]
    print(f"utility: {experts} distinct rank-1 experts over {kspans} kernel-cited spans (rank-1 by expert: {r1})")
    util_ok = experts >= 3 and kspans >= 1
    print(f"utility leg (>= 1 span through >= 3 experts): {'PASS' if util_ok else 'FAIL'}")
    print(f"rule census: {on['cited_rules']}/{on['total_rules']} cited at least once, {on['zero_match_rules']} zero-match (disclosed, never auto-demoted)")
    print(f"cited_by_expert: {on['cited_by_expert']}")
else:
    print("utility leg: N/A (OFF posture table)")

print(f"\nverdict jsons: {sys.argv[1]} {sys.argv[2]}")
PYEOF
