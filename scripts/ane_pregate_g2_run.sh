#!/bin/sh
# Plan 377 P2.1 — the ane_pregate G2 cost cell runner.
# Interleaves the three arm invocations (A,B,C × 3 rounds) over the
# registered walk, adjudicates added p50/p95 (nearest-rank, per-file
# medians, tail support) against the pre-registered 1.0/5.0 ms/file
# ceilings, and reads the utility floor. Quiet-box law: bench_preflight
# must PASS first — the PROVENANCE line is quoted in the record.
#
# Usage: scripts/ane_pregate_g2_run.sh <baseline-bin> <full-bin> [root]
#   <baseline-bin>  the default+latent_retrieval build of ane_pregate_g2
#   <full-bin>      the latent_retrieval,ane,ane_pregate build
# Exits 1 on a preflight refusal or a pre-registered bar FAIL; a
# MARGINAL pass (p95 within 2x of 5.0, or p50 within 2x of 1.0) exits 0
# but prints MARGINAL — re-run alone before any green (Bench-131 law).
set -eu

BASELINE=${1:?usage: ane_pregate_g2_run.sh <baseline-bin> <full-bin> [root]}
FULL=${2:?usage: ane_pregate_g2_run.sh <baseline-bin> <full-bin> [root]}
ROOT=${3:-src}
OUT=$(mktemp -d /tmp/i377g2.XXXXXX)

# ── the quiet-box gate ──
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFLIGHT="$SCRIPT_DIR/../../riir-reflex/scripts/bench_preflight.sh"
if [ ! -x "$PREFLIGHT" ] || [ ! -f "$PREFLIGHT" ]; then
    PREFLIGHT="$SCRIPT_DIR/../../../riir-reflex/scripts/bench_preflight.sh"
fi
if [ ! -f "$PREFLIGHT" ]; then
    echo "REFUSE: bench_preflight.sh not found (../riir-reflex expected)" >&2
    exit 2
fi
PROV=$("$PREFLIGHT" 2>&1 | grep '^PROVENANCE:' || true)
PASS_LINE=$("$PREFLIGHT" 2>&1 | grep -c 'preflight PASSED' || true)
if [ "$PASS_LINE" != "1" ]; then
    echo "REFUSE: bench_preflight did not PASS — never measure a BUSY box" >&2
    echo "$PROV" >&2
    exit 1
fi
echo "PROVENANCE: $PROV"

# ── the interleaved rounds (A,B,C × 3 — the sequential-A/B class) ──
for r in 1 2 3; do
    "$BASELINE" "$ROOT" > "$OUT/base_r$r.json" 2>"$OUT/base_r$r.log"
    RIIR_ANE_PREGATE=0 "$FULL" "$ROOT" > "$OUT/ungated_r$r.json" 2>"$OUT/ungated_r$r.log"
    "$FULL" "$ROOT" > "$OUT/pregate_r$r.json" 2>"$OUT/pregate_r$r.log"
    echo "round $r done"
done

# ── adjudication ──
python3 - "$OUT" "$PROV" <<'EOF'
import json, sys, statistics

out, prov = sys.argv[1], sys.argv[2]

def load(name, r):
    with open(f"{out}/{name}_r{r}.json") as f:
        for line in f:
            if line.startswith("{"):
                return json.loads(line)
    raise SystemExit(f"{name}_r{r}: no JSON doc")

def median(v):
    return statistics.median(v)

def nearest_rank(sorted_v, p):
    # nearest-rank percentile index (ceiling), tail support = n - idx
    import math
    n = len(sorted_v)
    idx = max(0, math.ceil(p * n) - 1)
    return sorted_v[idx], n - idx

arms = {}
for name in ("base", "ungated", "pregate"):
    docs = [load(name, r) for r in (1, 2, 3)]
    files = docs[0]["per_file_us"].keys()
    assert all(set(d["per_file_us"].keys()) == set(files) for d in docs), "walk drifted between rounds"
    # per_file_us[f] is the list of TIMED-PASS samples within one round
    # (one untimed warmup + 3 timed passes — the example header's law),
    # so the per-file population is 3 rounds × 3 passes = 9 samples;
    # flatten BEFORE the median (the old spelling fed median() a list of
    # lists — TypeError on the first subtraction, the bmo run-2 crash).
    med = {f: median([s for r in range(3) for s in docs[r]["per_file_us"][f]]) for f in files}
    arms[name] = {
        "docs": docs,
        "med": med,
        "floor": docs[0]["utility_floor"],
        "files": len(files),
    }

base = arms["base"]["med"]
ceil_p50, ceil_p95 = 1.0, 5.0  # ms/file — pre-registered, never re-registered

verdict = {}
for name in ("ungated", "pregate"):
    added = sorted((arms[name]["med"][f] - base[f]) / 1000.0 for f in base)  # µs → ms
    p50, ts50 = nearest_rank(added, 0.50)
    p95, ts95 = nearest_rank(added, 0.95)
    floor = arms[name]["floor"]
    p50_ok = p50 <= ceil_p50
    p95_ok = p95 <= ceil_p95
    marginal = (p95_ok and p95 > ceil_p95 / 2) or (p50_ok and p50 > ceil_p50 / 2)
    verdict[name] = {
        "added_p50_ms": round(p50, 3), "tail_support_50": ts50,
        "added_p95_ms": round(p95, 3), "tail_support_95": ts95,
        "p50_ok": p50_ok, "p95_ok": p95_ok, "marginal": marginal,
        "floor_holds": floor.get("holds") is True,
        "floor_detail": floor,
        "files": arms[name]["files"],
    }

doc = {
    "leg": "ane_pregate G2 cost cell (Plan 377 P2.1) — adjudicated",
    "provenance": prov,
    "rounds": 3,
    "ceiling_p50_ms": ceil_p50, "ceiling_p95_ms": ceil_p95,
    "baseline_files": arms["base"]["files"],
    "matcher_build_ms": {n: arms[n]["docs"][0]["matcher_build_ms"] for n in arms},
    "verdict": verdict,
    "raw_dir": out,
}
print(json.dumps(doc, indent=2))

ok = True
for name, v in verdict.items():
    tag = "PASS" if (v["p50_ok"] and v["p95_ok"] and v["floor_holds"]) else "FAIL"
    marg = " MARGINAL(re-run-alone)" if v["marginal"] and tag == "PASS" else ""
    print(f"{name}: {tag}{marg} — added p50 {v['added_p50_ms']} ms (<= {ceil_p50}), "
          f"added p95 {v['added_p95_ms']} ms (<= {ceil_p95}), floor {'HOLDS' if v['floor_holds'] else 'MISS'}")
    if tag == "FAIL":
        ok = False
sys.exit(0 if ok else 1)
EOF
