#!/bin/sh
# Issue 191 bar 5 — the G5 utility + cost leg; extended for Issue 196
# R2/R3 with the THIRD posture (the route-narrowed kernel index).
#
# Runs the census+latency harness (examples/kernel_goat_g5.rs) in THREE
# postures over the pinned corpus, computes the per-file medians and the
# ON−OFF added latency, and adjudicates the PRE-REGISTERED bars:
#   - added latency: p50 <= 1.0 ms/file, p95 <= 5.0 ms/file (flat AND
#     narrowed — the same ceilings; Issue 196 R3's verdict is the
#     narrow-vs-off diff)
#   - utility floor: >= 1 span fired through >= 3 distinct category experts
#     (Bench 125's record: 15/15 at the flat posture — the narrowed run
#     must HOLD that 15/15, the issue's own floor)
#   - R2 disclosure: the per-expert rank-1 delta + the rank-1 flip census
#     (paired per-span ids) + the cited-set shrinkage, flat vs narrowed.
#
# Usage: sh scripts/g5_utility_cost.sh [--root src] [--top-k 5] [--out-dir <dir>]
# No cargo in this script beyond the three example builds (isolated target
# dirs so a sibling build never holds the lock).

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
echo "box: $(uname -s) $(nproc 2>/dev/null || echo '?') cpus | free mem: $(wmic OS get FreePhysicalMemory /value 2>/dev/null | grep = || cat /proc/meminfo 2>/dev/null | grep MemAvailable || echo 'n/a')"

# ── the OFF posture (latent_retrieval for the matcher; kernel_opt OFF) ──
CARGO_TARGET_DIR="$TD/t_off" cargo build --release --example kernel_goat_g5 --features latent_retrieval 2>"$TD/off_build.log"
"$TD/t_off/release/examples/kernel_goat_g5.exe" --root "$ROOT" --top-k "$TOP_K" --out "$TD/g5_off.json"

# ── the flat ON posture (+kernel_opt, the Bench-125 reference) ──────────
CARGO_TARGET_DIR="$TD/t_on" cargo build --release --example kernel_goat_g5 --features latent_retrieval,kernel_opt 2>"$TD/on_build.log"
"$TD/t_on/release/examples/kernel_goat_g5.exe" --root "$ROOT" --top-k "$TOP_K" --out "$TD/g5_on.json"

# ── the narrowed ON posture (+kernel_route_narrow, Issue 196) ───────────
CARGO_TARGET_DIR="$TD/t_narrow" cargo build --release --example kernel_goat_g5 --features kernel_route_narrow 2>"$TD/narrow_build.log"
"$TD/t_narrow/release/examples/kernel_goat_g5.exe" --root "$ROOT" --top-k "$TOP_K" --out "$TD/g5_narrow.json"

# ── the adjudication ─────────────────────────────────────────────────────
python3 - "$TD/g5_off.json" "$TD/g5_on.json" "$TD/g5_narrow.json" <<'PYEOF'
import json
import statistics
import sys

off = json.load(open(sys.argv[1], encoding="utf-8"))
on = json.load(open(sys.argv[2], encoding="utf-8"))
narrow = json.load(open(sys.argv[3], encoding="utf-8"))

root = off["corpus_root"]
for tbl, name in [(on, "flat"), (narrow, "narrow")]:
    if tbl["files"] != off["files"] or tbl["corpus_root"] != root:
        raise SystemExit(f"REFUSE: {name} posture table disagrees on the corpus walk")

def medians(tbl):
    rows = tbl["per_pass_us"]
    paths = [p for p, _ in rows[0]]
    return paths, [statistics.median([rows[k][i][1] for k in range(len(rows))]) for i in range(len(paths))]

off_paths, off_med = medians(off)
on_paths, on_med = medians(on)
narrow_paths, narrow_med = medians(narrow)
assert off_paths == on_paths == narrow_paths, "file order drifted between postures"

def added_dists(flat_med, tbl_med):
    added_ms = sorted((t - f) / 1000.0 for f, t in zip(flat_med, tbl_med))
    n = len(added_ms)
    def pct(sorted_xs, p):
        # nearest-rank on the pre-sorted list; tail support disclosed
        idx = min(n - 1, max(0, round(p * n) - 1))
        return sorted_xs[idx], n - idx
    p50, sup50 = pct(added_ms, 0.50)
    p95, sup95 = pct(added_ms, 0.95)
    over = sum(1 for x in added_ms if x > 1.0)
    return p50, sup50, p95, sup95, over, n

for name, tbl_med in [("flat (kernel_opt)", on_med), ("narrow (kernel_route_narrow)", narrow_med)]:
    p50, sup50, p95, sup95, over, n = added_dists(off_med, tbl_med)
    print(f"added latency ms/file [{name}]: p50 {p50:.3f} (tail support {sup50}) | p95 {p95:.3f} (tail support {sup95}) | files over 1.0ms: {over}/{n}")
print("ceilings: p50 <= 1.0 ms | p95 <= 5.0 ms (the SAME pre-registered bars both postures)")

def cost_verdict(tbl_med):
    p50, sup50, p95, sup95, _, _ = added_dists(off_med, tbl_med)
    return p50 <= 1.0 and p95 <= 5.0
print(f"cost leg [flat]: {'PASS' if cost_verdict(on_med) else 'FAIL'}")
print(f"cost leg [narrow] (Issue 196 R3 verdict): {'PASS' if cost_verdict(narrow_med) else 'FAIL'}")

def utility(tbl, name):
    if "utility_distinct_rank1_experts" not in tbl:
        return
    experts = tbl["utility_distinct_rank1_experts"]
    kspans = tbl["kernel_spans_last_pass"]
    r1 = tbl["rank1_by_expert"]
    util_ok = experts >= 3 and kspans >= 1
    holds_1515 = experts == 15
    print(f"utility [{name}]: {experts} distinct rank-1 experts over {kspans} kernel-cited spans "
          f"(floor >= 3: {'PASS' if util_ok else 'FAIL'}; the 15/15 Bench-125 floor: {'HOLDS' if holds_1515 else 'LOST'})")
    print(f"  rank1_by_expert: {r1}")
    print(f"  rule census: {tbl['cited_rules']}/{tbl['total_rules']} cited, {tbl['zero_match_rules']} zero-match "
          f"(disclosed, never auto-demoted)")
    print(f"  cited_by_expert: {tbl['cited_by_expert']}")

utility(on, "flat")
utility(narrow, "narrow")

# ── R2 recall disclosure: flat vs narrowed, paired per span ──────────────
if "rank1_rule_ids" in on and "rank1_rule_ids" in narrow:
    f_ids, n_ids = on["rank1_rule_ids"], narrow["rank1_rule_ids"]
    if len(f_ids) != len(n_ids):
        raise SystemExit("REFUSE: rank1_rule_ids length drift between postures")
    flips = sum(1 for a, b in zip(f_ids, n_ids) if a != b)
    lost = sum(1 for a, b in zip(f_ids, n_ids) if a and not b)
    gained = sum(1 for a, b in zip(f_ids, n_ids) if b and not a)
    flat_set, narrow_set = set(f_ids) - {""}, set(n_ids) - {""}
    shrink = sorted(flat_set - narrow_set)
    print(f"R2 recall delta: rank-1 flips {flips}/{len(f_ids)} spans "
          f"(lost rank-1 entirely: {lost}, gained: {gained}); "
          f"cited-set shrinkage: {len(shrink)} rules cited flat but never narrowed")
    if shrink:
        print(f"  shrunk rules: {', '.join(shrink)}")

print(f"\nverdict jsons: {sys.argv[1]} {sys.argv[2]} {sys.argv[3]}")
PYEOF
