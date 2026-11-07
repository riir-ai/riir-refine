#!/usr/bin/env python3
"""ndb Issue 643 — the latent-leg-skip A/B adjudicator (T2's verdict half).

Reads the `--latent-ab` json from riir-refine's kernel_goat_g5 (one binary,
interleaved on,off,on,off walk passes) and adjudicates against the floor
table pinned in riir-neuron-db .issues/643 T1 (BEFORE any A/B read):

  - Quality (per retrieval-bearing domain): the skip arm's rank-1 must be
    SPAN-IDENTICAL to the incumbent's — every span keeps its top-1 rule id
    (retention 1.0) and no span gains a top-1 it did not have (zero gain).
    Any flip, any gain, or a span-count drift between arms exits 1 and names
    the axis. The vacuous-held domains (docker/dist/shader on a `.rs` walk)
    must be absent from BOTH arms' rank-1 (structural absence, the Issue-145
    contract) — their appearance is a scope drift and exits 1.
  - Cost (the timing half): per-file medians across each arm's passes, the
    skip − incumbent delta per file, and the ADDED p50/p95 (negative = the
    skip is faster). Timing verdicts belong to the pre-registered G2
    protocol on a quiet rig; a wall-clock bar is never adjudicated from a
    busy box — this script only REPORTS the numbers it was handed.

Usage: g5_latent_ab_adjudicate.py AB_JSON [--ab-dir] (legacy multi-file mode
kept for paired per-arm files).
Exits 1 on any quality-floor violation or structural drift; 0 otherwise.
"""
import argparse
import json
import sys


def load(path: str) -> dict:
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def median(vals: list[int]) -> float:
    s = sorted(vals)
    n = len(s)
    if n % 2:
        return float(s[n // 2])
    return (s[n // 2 - 1] + s[n // 2]) / 2


def percentile(vals: list[int], p: float) -> float:
    # Nearest-rank on the sorted list; the caller names the support.
    s = sorted(vals)
    idx = min(len(s) - 1, max(0, round(p * (len(s) - 1))))
    return float(s[idx])


def adjudicate(inc: dict, skip: dict, label: str) -> int:
    i_spans = inc.get("rank1_all")
    s_spans = skip.get("rank1_all")
    if not isinstance(i_spans, list) or not isinstance(s_spans, list):
        print("REFUSE: rank1_all missing", file=sys.stderr)
        return 1
    if len(i_spans) != len(s_spans):
        print(
            f"REFUSE: span-count drift ({label}): incumbent {len(i_spans)} "
            f"vs skip {len(s_spans)}",
            file=sys.stderr,
        )
        return 1

    vacuous = ("docker", "dist", "shader")
    flips: list[str] = []
    gains: list[str] = []
    scope_drift: list[str] = []

    def census(spans: list[list[str]]) -> dict[str, int]:
        c: dict[str, int] = {}
        for span in spans:
            for e in span:
                dom = e.rsplit(":", 1)[0]
                c[dom] = c.get(dom, 0) + 1
        return c

    inc_c = census(i_spans)
    skip_c = census(s_spans)
    for dom in vacuous:
        if inc_c.get(dom, 0) or skip_c.get(dom, 0):
            scope_drift.append(
                f"{dom}: incumbent={inc_c.get(dom, 0)} skip={skip_c.get(dom, 0)}"
                " (vacuous-held domain must be absent BOTH arms)"
            )

    for idx, (a, b) in enumerate(zip(i_spans, s_spans)):
        if a == b:
            continue
        a_only = [x for x in a if x not in b]
        b_only = [x for x in b if x not in a]
        for x in a_only:
            flips.append(f"span {idx}: LOST {x} (had {sorted(a)})")
        for x in b_only:
            gains.append(f"span {idx}: GAINED {x} (had {sorted(a)})")

    print(f"[{label}] spans compared: {len(i_spans)}")
    print(f"[{label}] per-domain rank-1 incumbent: {inc_c}")
    print(f"[{label}] per-domain rank-1 skip:      {skip_c}")
    print(
        f"[{label}] top-1 flips: {len(flips)} · gains: {len(gains)} · "
        f"scope drift: {len(scope_drift)}"
    )
    for row in (flips + gains + scope_drift)[:20]:
        print(f"  {row}")
    if len(flips) + len(gains) + len(scope_drift) > 20:
        hidden = len(flips) + len(gains) + len(scope_drift) - 20
        print(f"  ... and {hidden} more")

    # Cost REPORT (never a verdict from a busy box — the G2 protocol owns bars).
    for arm_name, arm in (("incumbent", inc), ("skip", skip)):
        per_file = arm.get("per_file_us", {})
        med = [median(vals) for vals in per_file.values()] if per_file else []
        totals = arm.get("pass_totals_us", [])
        print(
            f"[{label}] {arm_name}: pass totals {totals}"
            + (f" · per-file median p50 {median(med):.0f} µs" if med else "")
        )
    if inc.get("per_file_us") and skip.get("per_file_us"):
        deltas = []
        for path, ivals in inc["per_file_us"].items():
            svals = skip["per_file_us"].get(path)
            if svals is None:
                continue
            deltas.append(median(svals) - median(ivals))
        if deltas:
            print(
                f"[{label}] skip − incumbent per-file delta: "
                f"p50 {percentile(deltas, 0.5):+.0f} µs · "
                f"p95 {percentile(deltas, 0.95):+.0f} µs"
            )

    verdict = "PASS" if not (flips or gains or scope_drift) else "FAIL"
    print(f"[{label}] quality verdict: {verdict}")
    return 0 if verdict == "PASS" else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("inputs", nargs="+", help="one --latent-ab json, or two paired arm files")
    args = ap.parse_args()

    if len(args.inputs) == 1:
        d = load(args.inputs[0])
        for key in ("incumbent", "skip"):
            if key not in d:
                print(f"REFUSE: missing '{key}' arm", file=sys.stderr)
                return 1
        return adjudicate(d["incumbent"], d["skip"], d.get("leg", "ab")[:40])
    if len(args.inputs) == 2:
        return adjudicate(load(args.inputs[0]), load(args.inputs[1]), "paired")
    print("REFUSE: pass ONE --latent-ab json or TWO paired arm files", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
