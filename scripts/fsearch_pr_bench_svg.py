#!/usr/bin/env python3
"""Render the fsearch outbound-PR bench SVG (before/after search-latency bars).

Reads the rig JSON outputs (fsearch `examples/rig.rs run` — before.json, and
after.json once the gated fix lands) and renders one deterministic SVG with the
refine.riir.ai footer — the outbound-PR marketing artifact (Plan 224).

Usage:
  python3 scripts/fsearch_pr_bench_svg.py --before <before.json> \
      [--after <after.json>] [--meta <meta.json>] --out <bench.svg>

meta.json (optional): {"pin": ..., "corpus": ..., "entries": ..., "box": ...,
"kinds": ...} — defaults name the recorded Plan-224 fixture.

Categories: the rig's per-(kind, category) rows are aggregated to geomean ms
over all samples of the run (3 rounds per query). Only name + typing kinds are
charted (the scoped snapshot carries no content index, so grep is absent).
"""

import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

# A non-UTF-8 console (cp874) would kill this script on its first
# non-ASCII glyph; degrade the glyph, never the verdict (katgpt-rs
# Issue 804). A captured stream without reconfigure is left alone.
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(errors="backslashreplace")
    except (AttributeError, ValueError):
        pass

# (kind, category) -> chart label; order is the chart order.
ROWS = [
    ("name", "exact", "find a file by name (exact)"),
    ("name", "typo", "name with one typo"),
    ("name", "word", "common short word"),
    ("name", "multi", "folder + name"),
    ("name", "filter", "filtered query (ext:/type:/in:)"),
    ("typing", "typing", "typing a whole filename, all keystrokes"),
]

BAR_H = 22
BAR_GAP = 6
GROUP_GAP = 10
LABEL_W = 300
VALUE_W = 170
LEFT = LABEL_W + 16
PLOT_W = 420
TOP = 116
FOOTER_H = 64


def geomean(rows):
    xs = [x for x in rows if x > 0]
    return math.exp(sum(math.log(x) for x in xs) / len(xs)) if xs else float("nan")


def load(path):
    """rig output dict -> {(kind, cat): geomean_ms} using the rig's own
    aggregation: per query, the MEDIAN of its rounds; then the geomean
    across the category's queries (matches examples/rig.rs console tables)."""
    data = json.load(open(path, encoding="utf-8"))
    acc = defaultdict(list)
    for key, entry in data.items():
        if not isinstance(entry, dict) or "ms" not in entry:
            continue
        kind = entry.get("kind", key.split(" ", 1)[0])
        s = sorted(entry["ms"])
        med = s[len(s) // 2] if len(s) % 2 else (s[len(s) // 2 - 1] + s[len(s) // 2]) / 2
        acc[(kind, entry.get("cat", "?"))].append(med)
    return {k: geomean(v) for k, v in acc.items()}


def fmt(ms):
    if ms >= 10:
        return f"{ms:.1f} ms"
    if ms >= 1:
        return f"{ms:.2f} ms"
    return f"{ms*1000:.0f} µs"


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--before", required=True)
    ap.add_argument("--after")
    ap.add_argument("--meta")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    meta = json.load(open(a.meta, encoding="utf-8")) if a.meta else {}
    before = load(a.before)
    after = load(a.after) if a.after else None

    rows = []
    for kind, cat, label in ROWS:
        b = before.get((kind, cat))
        s = after.get((kind, cat)) if after else None
        if b is None:
            continue
        rows.append((label, b, s))

    peak = max(max(b, s or 0) for _, b, s in rows)
    n_bars = len(rows) * (2 if after else 1)
    height = TOP + n_bars * (BAR_H + BAR_GAP) + len(rows) * GROUP_GAP + FOOTER_H
    width = LEFT + PLOT_W + VALUE_W + 24

    def x(ms):
        return LEFT + (math.log10(max(ms, 1e-4)) - math.log10(1e-4)) / (
            math.log10(peak * 1.2) - math.log10(1e-4)
        ) * PLOT_W

    o = []
    o.append(f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="ui-monospace,Menlo,monospace">')
    o.append(f'<rect width="{width}" height="{height}" fill="#14161a"/>')
    pin = esc(meta.get("pin", "fsearch @ b88f4e9a"))
    corpus = esc(meta.get("corpus", "~/git dev-workspace snapshot (APFS-frozen)"))
    entries = meta.get("entries", 4019254)
    box = esc(meta.get("box", "Apple M3 Max (Mac15,8), AC power, quiet-box"))
    o.append(f'<text x="24" y="32" fill="#f2f2f2" font-size="19" font-weight="bold">fsearch search latency — cargo refine outbound bench</text>')
    o.append(f'<text x="24" y="54" fill="#9aa0a6" font-size="12">{pin} · {entries:,} entries</text>')
    o.append(f'<text x="24" y="72" fill="#9aa0a6" font-size="12">corpus: {corpus} · box: {box}</text>')
    o.append(f'<text x="24" y="90" fill="#9aa0a6" font-size="12">geomean of per-query median (3 rig rounds) · lower is better · log scale</text>')

    # legend
    ly = 104
    if after:
        o.append(f'<rect x="{LEFT}" y="{ly}" width="12" height="12" fill="#5b8def"/><text x="{LEFT+18}" y="{ly+10}" fill="#c9cdd3" font-size="12">before (pin)</text>')
        o.append(f'<rect x="{LEFT+140}" y="{ly}" width="12" height="12" fill="#3fbf7f"/><text x="{LEFT+158}" y="{ly+10}" fill="#c9cdd3" font-size="12">after (cargo refine)</text>')
    else:
        o.append(f'<rect x="{LEFT}" y="{ly}" width="12" height="12" fill="#5b8def"/><text x="{LEFT+18}" y="{ly+10}" fill="#c9cdd3" font-size="12">before (pin) — after pending, gated on mainnet launch</text>')

    y = TOP + 14
    for label, b, s in rows:
        o.append(f'<text x="24" y="{y + BAR_H - 4}" fill="#e8e8e8" font-size="13">{esc(label)}</text>')
        if after:
            o.append(f'<rect x="{LEFT}" y="{y}" width="{max(x(b)-LEFT,2)}" height="{BAR_H}" fill="#5b8def" rx="3"/>')
            o.append(f'<text x="{x(b)+8}" y="{y + BAR_H - 6}" fill="#9db8ef" font-size="12">{fmt(b)}</text>')
            y += BAR_H + BAR_GAP
            if s:
                o.append(f'<rect x="{LEFT}" y="{y}" width="{max(x(s)-LEFT,2)}" height="{BAR_H}" fill="#3fbf7f" rx="3"/>')
                o.append(f'<text x="{x(s)+8}" y="{y + BAR_H - 6}" fill="#8fdcb4" font-size="12">{fmt(s)}  ({b/s:.2f}×)</text>')
            else:
                o.append(f'<text x="{LEFT+8}" y="{y + BAR_H - 6}" fill="#6b7280" font-size="12">after pending</text>')
            y += BAR_H + GROUP_GAP
        else:
            o.append(f'<rect x="{LEFT}" y="{y}" width="{max(x(b)-LEFT,2)}" height="{BAR_H}" fill="#5b8def" rx="3"/>')
            o.append(f'<text x="{x(b)+8}" y="{y + BAR_H - 6}" fill="#9db8ef" font-size="12">{fmt(b)}</text>')
            y += BAR_H + GROUP_GAP

    y += 12
    o.append(f'<line x1="24" y1="{y}" x2="{width-24}" y2="{y}" stroke="#2a2e35"/>')
    o.append(f'<text x="24" y="{y+22}" fill="#c9cdd3" font-size="13">fix + bench by cargo refine · harness: fsearch examples/rig.rs (upstream, identical corpus both arms)</text>')
    o.append(f'<text x="24" y="{y+42}" fill="#5b8def" font-size="14" font-weight="bold">refine.riir.ai — the modelless Rust fixer + bench tool</text>')

    Path(a.out).write_text("\n".join(o) + "\n", encoding="utf-8")
    print(f"wrote {a.out} ({len(rows)} rows, after={'yes' if after else 'pending'})")


if __name__ == "__main__":
    sys.exit(main())
