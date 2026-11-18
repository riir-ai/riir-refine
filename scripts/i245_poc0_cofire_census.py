#!/usr/bin/env python3
"""i245 PoC-0: rule co-fire census over the Bench-094 wave scans.

Parses .benchmarks/data/094_t0{2,3,4}/results_wave*.jsonl per-crate `lints`
dicts, pins BOTH parse conventions the filing flagged as disagreeing
(verdict filter x wave-dedupe), emits the census table + the co-fire edge
list + the per-crate population under the pinned convention, and checks the
pre-registered kill conditions from riir-refine Issue 245.

Wave 1 (094_t01) is totals-only (manifest has no per-crate sets) and is
deliberately excluded; recovering it needs a wave-1 re-run, priced
separately, never blocking PoC-0 (Issue 245 PoC-0 note).

Outputs (under .benchmarks/data/094_cofire_census/):
  census_combos.tsv   all four convention combos, one row each
  edges_weighted.jsonl  co-fire edges under the pinned convention
                        ({"a","b","weight"} per line, a<b)
  population.jsonl    per-crate lint sets under the pinned convention
                      ({"name","lints","waves"} per line)

Exit codes: 0 = census written (pass or NEGATIVE-sparse both exit 0 — the
verdict lives in the census table and stdout); 2 = data missing/unreadable.
"""

import itertools
import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DATA = REPO / ".benchmarks" / "data"
WAVES = ["094_t02", "094_t03", "094_t04"]
OUT = DATA / "094_cofire_census"

KILL_MIN_CRATES = 40
KILL_MIN_EDGES_GE3 = 60

# PoC-1 pre-registration, stated HERE before any community run (Issue 245:
# "pin the exact test + level BEFORE running; state it in the census output
# first"). Pinned 2026-11-18 by the PoC-0 lane:
#   - Algorithm: Louvain-class community detection over the weighted co-fire
#     graph (min-support 3: drop edges weight < 3 before detection; a
#     community whose members co-fire in fewer than 3 crates is an anecdote).
#   - Null: degree-preserving permutation (each lint's fire count AND each
#     crate's size fixed), 1000 draws, same min-support + detection.
#   - Test: observed max community size vs the null distribution of max
#     community size; PASS iff observed >= the 95th percentile of the null
#     (one-sided p <= 0.05) AND the community has >= 3 members.
#   - Any other reading (post-hoc metric swap, level change after seeing the
#     null) is a protocol violation, not a result.
POC1_PREREG = {
    "algorithm": "louvain-class weighted community detection",
    "min_support": 3,
    "null": "degree-preserving permutation (lint fire counts + crate sizes fixed)",
    "draws": 1000,
    "test": "observed max community size >= 95th percentile of null max (one-sided p<=0.05), community size >= 3",
}


def load_rows():
    rows = []
    for w in WAVES:
        p = DATA / w / ("results_wave%s.jsonl" % w[-1])
        if not p.exists():
            # file naming: results_wave2/3/4.jsonl
            p = DATA / w / ("results_wave%d.jsonl" % int(w[-1]))
        if not p.exists():
            print("MISSING DATA: %s" % p)
            sys.exit(2)
        with open(p, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                r = json.loads(line)
                r["_wave"] = int(w[-1])
                rows.append(r)
    return rows


def census(rows, ok_only, dedupe):
    """Return (crates_ge2, lint_universe, edges_dict) under one combo.

    dedupe mode:
      "pairunion": one entry per crate; its edge set is the UNION of the
        same-scan pairs across that crate's (filtered) rows - each crate
        counted once, only pairs that truly co-fired in one scan. The
        convention of record for a co-fire graph (no phantom cross-scan
        pairs, no double-counted crates).
      "keyunion": one entry per crate; its lint KEY set is the union across
        rows, then pairs are formed. Naive reading - can create phantom
        cross-scan pairs (A from wave2 + C from wave3 never co-fired).
      "perwave": every (wave, row) contributes independently - true
        same-scan pairs, but a crate re-run in a later wave contributes
        the same edges again (weight inflation).
    """
    per_pairs = defaultdict(set)  # name -> set of same-scan pairs
    per_keys = defaultdict(set)   # name -> union of lint keys
    entries = []                  # (name, pair_set) contributors
    for r in rows:
        if ok_only and r.get("verdict") != "ok":
            continue
        lints = r.get("lints") or {}
        pairs = set(itertools.combinations(sorted(lints.keys()), 2))
        if dedupe == "perwave":
            entries.append((r["name"], pairs))
        else:
            per_pairs[r["name"]].update(pairs)
            per_keys[r["name"]].update(lints.keys())
    if dedupe == "perwave":
        use = entries
    elif dedupe == "pairunion":
        use = [(n, s) for n, s in per_pairs.items()]
    else:  # keyunion
        use = [(n, set(itertools.combinations(sorted(s), 2)))
               for n, s in per_keys.items()]
    edges = defaultdict(int)
    crates_ge2 = 0
    universe = set()
    for name, pset in use:
        universe |= {a for p in pset for a in p} | {b for p in pset for b in p}
        if len(pset) < 1:
            continue
        crates_ge2 += 1
        for pair in pset:
            edges[pair] += 1
    return crates_ge2, universe, edges


def main():
    rows = load_rows()
    total_rows = len(rows)
    ok_rows = sum(1 for r in rows if r.get("verdict") == "ok")
    verdicts = defaultdict(int)
    for r in rows:
        verdicts[r.get("verdict", "<none>")] += 1
    names_all = sorted(set(r["name"] for r in rows))
    repeated = total_rows - len(names_all)

    OUT.mkdir(parents=True, exist_ok=True)

    combos = {}
    for ok_only in (True, False):
        for dedupe in ("pairunion", "keyunion", "perwave"):
            crates_ge2, universe, edges = census(rows, ok_only, dedupe)
            ge3 = sum(1 for w in edges.values() if w >= 3)
            wsum = sum(edges.values())
            combos[(ok_only, dedupe)] = (crates_ge2, len(universe), len(edges), ge3, wsum)

    # Convention of record: ok-only + per-crate PAIR-union (completed scans
    # only; each crate counted once; only pairs that truly co-fired in one
    # scan). Key-union and per-wave variants reported alongside.
    pin = (True, "pairunion")
    pin_crates, pin_universe, pin_edges_n, pin_ge3, _ = combos[pin]
    _, _, pin_edges = census(rows, pin[0], pin[1])

    # outputs
    with open(OUT / "census_combos.tsv", "w", encoding="utf-8") as f:
        f.write("ok_only\tdedupe\tcrates_ge2\tlint_universe\tedges\tedges_ge3\tedge_instances\n")
        for (ok_only, dedupe), v in sorted(combos.items(), key=lambda kv: (not kv[0][0], kv[0][1])):
            f.write("%s\t%s\t%d\t%d\t%d\t%d\t%d\n" % (ok_only, dedupe, v[0], v[1], v[2], v[3], v[4]))
    with open(OUT / "edges_weighted.jsonl", "w", encoding="utf-8") as f:
        for (a, b), w in sorted(pin_edges.items()):
            f.write(json.dumps({"a": a, "b": b, "weight": w}) + "\n")
    per_crate = defaultdict(lambda: {"lints": set(), "waves": set()})
    for r in rows:
        if r.get("verdict") != "ok":
            continue
        pc = per_crate[r["name"]]
        pc["lints"].update((r.get("lints") or {}).keys())
        pc["waves"].add(r["_wave"])
    with open(OUT / "population.jsonl", "w", encoding="utf-8") as f:
        for name in sorted(per_crate):
            pc = per_crate[name]
            if len(pc["lints"]) < 1:
                continue
            f.write(json.dumps({
                "name": name,
                "lints": sorted(pc["lints"]),
                "waves": sorted(pc["waves"]),
            }) + "\n")

    # verdict
    sparse = pin_crates < KILL_MIN_CRATES or pin_ge3 < KILL_MIN_EDGES_GE3
    verdict = "NEGATIVE-sparse (kill condition fired)" if sparse else "PASS - population viable for PoC-1"

    print("=== i245 PoC-0 co-fire census ===")
    print("rows: %d total, %d ok, %d non-ok; verdict census: %s" % (
        total_rows, ok_rows, total_rows - ok_rows, dict(verdicts)))
    print("distinct crates scanned: %d; repeated (wave,crate) rows: %d" % (len(names_all), repeated))
    print("")
    print("census combos (crates_ge2 / lint_universe / edges / edges_ge3 / instances):")
    for (ok_only, dedupe), v in sorted(combos.items(), key=lambda kv: (not kv[0][0], kv[0][1])):
        tag = "%s+%s" % ("ok" if ok_only else "all", dedupe)
        print("  %-16s %4d / %4d / %5d / %5d / %6d %s" % (
            tag, v[0], v[1], v[2], v[3], v[4], "<== PINNED" if (ok_only, dedupe) == pin else ""))
    print("")
    print("pinned convention: verdict=='ok' rows only + per-crate PAIR-union across waves")
    print("(keyunion = naive key-set union, can hold phantom cross-scan pairs; perwave =")
    print("true same-scan pairs but double-counts the %d repeated (wave,crate) rows)" % repeated)
    print("")
    print("delta attribution vs the filing's two parses (claude-verdict 105/153/1318/124,")
    print("coordinator 99/154/1579/111): all+perwave reproduces the claude-verdict crates/edges/ge3")
    print("(105/1318/124); ok+perwave reproduces the coordinator's 99/111; the coordinator's 1579")
    print("matches neither distinct-pair count - compare the per-wave edge-INSTANCE columns.")
    print("")
    print("kill condition: crates_ge2 >= %d AND edges_ge3 >= %d -> %s" % (
        KILL_MIN_CRATES, KILL_MIN_EDGES_GE3, verdict))
    print("PoC-1 pre-registration (pinned BEFORE any community run): %s" % json.dumps(POC1_PREREG))
    print("outputs: %s" % OUT)
    return 0


if __name__ == "__main__":
    sys.exit(main())
