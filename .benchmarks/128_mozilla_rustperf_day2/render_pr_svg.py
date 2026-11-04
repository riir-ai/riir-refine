#!/usr/bin/env python3
"""Render the sccache #2968 outbound-PR bench SVG (wall-time bars per arm).

Shape follows the house renderer (scripts/fsearch_pr_bench_svg.py, Plan 224):
dark bg, mono font, horizontal bars, provenance header, harness footer.
No tool/site tagline anywhere (owner verdict 2026-11-02 — attribution ban).

Data: bench 128 record run (benchrun2, 3 alternating reps per arm) + the
demote arm. Linear scale — same units, 33.5 vs 4.4 reads honestly linear.

Usage: python3 render_pr_svg.py --out bench128_backend_backoff.svg
"""

import sys
from pathlib import Path

# arm, median seconds, all three reps (for the label), color, is_before
ROWS = [
    ("uncached control (no sccache)", 0.590, "", "#6b7280"),
    ("before — pin ba261b8 (median of 3)", 33.527, "33.595 / 33.478 / 33.527", "#5b8def"),
    ("after — backend backoff (median of 3)", 4.409, "4.773 / 4.409 / 4.407", "#3fbf7f"),
    ("after, backoff disabled (THRESHOLD=0, 1 run)", 33.528, "", "#8a6d3b"),
]

TOP = 116
BAR_H = 34
BAR_GAP = 14
FOOTER_H = 64
LABEL_W = 420
VALUE_W = 210
LEFT = 24
PLOT_W = 420


def esc(s):
    return (
        s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def main():
    peak = max(v for _, v, _, _ in ROWS)
    height = TOP + len(ROWS) * (BAR_H + BAR_GAP) + FOOTER_H
    width = LEFT + LABEL_W + PLOT_W + VALUE_W + 24

    def x(ms):
        return LEFT + LABEL_W + ms / (peak * 1.05) * PLOT_W

    o = []
    o.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" font-family="ui-monospace,Menlo,monospace">'
    )
    o.append(f'<rect width="{width}" height="{height}" fill="#14161a"/>')
    o.append(
        '<text x="24" y="32" fill="#f2f2f2" font-size="19" font-weight="bold">'
        "sccache wall time, 32 compile requests against an unhealthy cache backend</text>"
    )
    o.append(
        '<text x="24" y="54" fill="#9aa0a6" font-size="12">'
        "before: sccache main ba261b8 · after: backend-read-timeout backoff (this PR)</text>"
    )
    o.append(
        '<text x="24" y="72" fill="#9aa0a6" font-size="12">'
        "blackhole WebDAV backend (GETs never answer) · SCCACHE_CACHE_READ_TIMEOUT_SECS=1 · medians of 3 alternating runs</text>"
    )
    o.append(
        '<text x="24" y="90" fill="#9aa0a6" font-size="12">'
        "box: Apple M3 Max, AC power, load 3.14 · rustc 1.91.0 (sccache MSRV) · lower is better · linear scale</text>"
    )

    ly = 104
    o.append(
        f'<rect x="{LEFT+LABEL_W}" y="{ly}" width="12" height="12" fill="#5b8def"/>'
        f'<text x="{LEFT+LABEL_W+18}" y="{ly+10}" fill="#c9cdd3" font-size="12">before (main)</text>'
    )
    o.append(
        f'<rect x="{LEFT+LABEL_W+150}" y="{ly}" width="12" height="12" fill="#3fbf7f"/>'
        f'<text x="{LEFT+LABEL_W+168}" y="{ly+10}" fill="#c9cdd3" font-size="12">after (backoff)</text>'
    )

    y = TOP + 14
    for label, v, reps, color in ROWS:
        o.append(
            f'<text x="{LEFT}" y="{y + BAR_H - 11}" fill="#e8e8e8" font-size="13">{esc(label)}</text>'
        )
        bar_w = max(x(v) - LEFT - LABEL_W, 2)
        o.append(
            f'<rect x="{LEFT+LABEL_W}" y="{y}" width="{bar_w}" height="{BAR_H}" fill="{color}" rx="3"/>'
        )
        if reps:
            o.append(
                f'<text x="{x(v)+8}" y="{y + BAR_H - 12}" fill="#9aa0a6" font-size="11">{esc(reps)}</text>'
            )
        tag = f"  {33.527 / 4.409:.2f}x faster" if v == 4.409 else ""
        o.append(
            f'<text x="{x(v)+8}" y="{y + BAR_H - 2}" fill="#c9cdd3" font-size="12">{v:.3f} s{tag}</text>'
        )
        y += BAR_H + BAR_GAP

    y += 6
    o.append(f'<line x1="24" y1="{y}" x2="{width-24}" y2="{y}" stroke="#2a2e35"/>')
    o.append(
        '<text x="24" y="'
        + str(y + 22)
        + '" fill="#c9cdd3" font-size="13">backend GETs: 32 before, 3 after (threshold 3 + skip window) — all 32 compiles and cache writes succeed on every arm</text>'
    )
    o.append(
        '<text x="24" y="'
        + str(y + 42)
        + '" fill="#9aa0a6" font-size="12">setup: see PR description · workload = 32 distinct .c files, fresh daemon per run, uncached control 0.590 s</text>'
    )
    o.append("</svg>")

    Path(sys.argv[sys.argv.index("--out") + 1]).write_text(
        "\n".join(o) + "\n", encoding="utf-8"
    )
    print(f"wrote {sys.argv[sys.argv.index('--out') + 1]}")


if __name__ == "__main__":
    sys.exit(main())
