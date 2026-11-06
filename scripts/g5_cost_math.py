"""The G5 cost-leg arithmetic, shared (Issue 212 T1).

One copy of the Issue-191 bar-5 math: per-file medians over the timed
passes, the added (ON - OFF) latency per file, nearest-rank p50/p95 with
tail support. Consumed by scripts/g5_utility_cost.sh (the kernel lane's
three-posture runner) and scripts/g5_domain_cost.sh (the generic
single-feature runner). The ceilings are the caller's, never this module's.

CLI (the generic runner's adjudication):
    python3 scripts/g5_cost_math.py --feature F off_1.json on_1.json [off_2.json on_2.json ...]
Each (off, on) pair is one interleaved round; every round is a full
instance of the pre-registered bar and the verdict is PASS only if EVERY
round passes (conservative — the ceilings are never relaxed).
"""
from __future__ import annotations

import json
import statistics
import sys

P50_CEIL_MS = 1.0
P95_CEIL_MS = 5.0


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def medians(tbl):
    """(paths, per-file median us) over the table's timed passes."""
    rows = tbl["per_pass_us"]
    paths = [p for p, _ in rows[0]]
    for k in range(1, len(rows)):
        if [p for p, _ in rows[k]] != paths:
            raise SystemExit("REFUSE: file order drifted between passes")
    return paths, [statistics.median([rows[k][i][1] for k in range(len(rows))]) for i in range(len(paths))]


def nearest_rank(sorted_xs, p):
    """Nearest-rank percentile on a pre-sorted list + its tail support."""
    n = len(sorted_xs)
    idx = min(n - 1, max(0, round(p * n) - 1))
    return sorted_xs[idx], n - idx


def added_dists(off_med, on_med):
    """(p50, sup50, p95, sup95, files_over_1ms, n) of the added ms/file."""
    added_ms = sorted((t - f) / 1000.0 for f, t in zip(off_med, on_med))
    n = len(added_ms)
    p50, sup50 = nearest_rank(added_ms, 0.50)
    p95, sup95 = nearest_rank(added_ms, 0.95)
    over = sum(1 for x in added_ms if x > 1.0)
    return p50, sup50, p95, sup95, over, n


def check_walk(off, on, name="on"):
    if on["files"] != off["files"] or on["corpus_root"] != off["corpus_root"]:
        raise SystemExit(f"REFUSE: {name} posture table disagrees on the corpus walk")


def feature_delta(off, on):
    """Features compiled ON but not OFF, and OFF but not ON (build stamps)."""
    a = set(filter(None, off.get("compiled_features", "").split(",")))
    b = set(filter(None, on.get("compiled_features", "").split(",")))
    return sorted(b - a), sorted(a - b)


def main(argv):
    if len(argv) < 4 or argv[0] != "--feature" or (len(argv) - 2) % 2:
        raise SystemExit("usage: g5_cost_math.py --feature F off.json on.json [off.json on.json ...]")
    feature = argv[1]
    pairs = [(argv[i], argv[i + 1]) for i in range(2, len(argv), 2)]
    all_pass = True
    marginal = False
    for r, (off_p, on_p) in enumerate(pairs, 1):
        off, on = load(off_p), load(on_p)
        check_walk(off, on)
        added, removed = feature_delta(off, on)
        # the build stamp spells features cargo's way (CARGO_FEATURE_* ->
        # lowercase, `_` -> `-`): compare on that normalisation
        if feature.replace("_", "-") not in {f.replace("_", "-") for f in added} or removed:
            raise SystemExit(
                f"REFUSE: round {r} binaries do not differ by {feature} "
                f"(ON-only {added}, OFF-only {removed})"
            )
        off_paths, off_med = medians(off)
        on_paths, on_med = medians(on)
        if off_paths != on_paths:
            raise SystemExit("REFUSE: file order drifted between postures")
        p50, sup50, p95, sup95, over, n = added_dists(off_med, on_med)
        ok = p50 <= P50_CEIL_MS and p95 <= P95_CEIL_MS
        # headroom = ceiling / observed (inf when the added cost is <= 0)
        h50 = P50_CEIL_MS / p50 if p50 > 0 else float("inf")
        h95 = P95_CEIL_MS / p95 if p95 > 0 else float("inf")
        if ok and min(h50, h95) < 2.0:
            marginal = True
        all_pass &= ok
        off_tot = sum(off_med) / 1000.0
        on_tot = sum(on_med) / 1000.0
        print(
            f"round {r} [{feature}] files {n} | added ms/file p50 {p50:.3f} (support {sup50}, headroom {h50:.1f}x) "
            f"| p95 {p95:.3f} (support {sup95}, headroom {h95:.1f}x) | files > 1.0ms: {over}/{n} "
            f"| walk total OFF {off_tot:.1f} ms ON {on_tot:.1f} ms | ON-only features {added} "
            f"| build ms OFF {off['matcher_build_ms']} ON {on['matcher_build_ms']} | {'PASS' if ok else 'FAIL'}"
        )
    print(f"ceilings: p50 <= {P50_CEIL_MS} ms | p95 <= {P95_CEIL_MS} ms (Issue 191 bar 5, verbatim)")
    verdict = "PASS" if all_pass else "FAIL"
    if all_pass and marginal:
        verdict = "PASS (MARGINAL < 2x headroom: re-run alone)"
    print(f"cost leg [{feature}]: {verdict} over {len(pairs)} interleaved round(s)")
    return 0 if all_pass else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
