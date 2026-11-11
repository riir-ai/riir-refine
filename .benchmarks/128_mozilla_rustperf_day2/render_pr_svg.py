#!/usr/bin/env python3
"""Render the sccache #2968 outbound-PR bench SVG (wall-time bars per arm).

Shape follows the house renderer (scripts/fsearch_pr_bench_svg.py, Plan 224):
dark bg, mono font, horizontal bars, provenance header, harness footer.
No tool/site tagline anywhere (owner verdict 2026-11-02 — attribution ban).

Data: bench 128 record run 8 re-run (2026-11-11, tip 2b18762 — the shipped binary, post single-probe recovery): the standalone cell
(3 alternating reps per arm + demote) and the multilevel chain cell
(disk,webdav with the backend going unhealthy mid-run). Linear scale.
Run-7 freeze (pre-recovery tip 30f9af0-era): within 2.5% of every cell here (record README).

Usage: python3 render_pr_svg.py --out bench128_backend_backoff.svg
"""

import sys
from pathlib import Path

# (cell divider, label, median seconds, reps string, color)
ROWS = [
    (False, "uncached control (no sccache)", 0.647, "", "#6b7280"),
    (False, "before — main 33d12fb (median of 3)", 33.596, "33.591 / 33.596 / 33.598", "#5b8def"),
    (False, "after — backend backoff (median of 3)", 4.394, "4.394 / 4.383 / 4.398", "#3fbf7f"),
    (False, "after, backoff disabled (THRESHOLD=0)", 33.572, "", "#8a6d3b"),
    (True, "multilevel chain disk,webdav — 48 compiles, backend unhealthy mid-run", 0, "", "#000000"),
    (False, "before — main 33d12fb (median of 3)", 33.883, "33.770 / 33.883 / 34.379", "#5b8def"),
    (False, "after — backend backoff (median of 3)", 4.697, "4.697 / 4.707 / 4.585", "#3fbf7f"),
    (False, "after, backoff disabled (THRESHOLD=0)", 34.046, "", "#8a6d3b"),
]

STANDALONE_PEAK = max(r[2] for r in ROWS[0:4])
CHAIN_PEAK = max(r[2] for r in ROWS[5:9])
TOP = 116
BAR_H = 34
BAR_GAP = 14
FOOTER_H = 64
LABEL_W = 430
VALUE_W = 200
LEFT = 24
PLOT_W = 400


def esc(s):
    return (
        s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def main():
    height = TOP + len(ROWS) * (BAR_H + BAR_GAP) + FOOTER_H + 18
    width = LEFT + LABEL_W + PLOT_W + VALUE_W + 24

    def x(v, peak):
        return LEFT + LABEL_W + v / (peak * 1.05) * PLOT_W

    o = []
    o.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" font-family="ui-monospace,Menlo,monospace">'
    )
    o.append(f'<rect width="{width}" height="{height}" fill="#14161a"/>')
    o.append(
        '<text x="24" y="32" fill="#f2f2f2" font-size="19" font-weight="bold">'
        "sccache wall time against an unhealthy cache backend — standalone and multilevel</text>"
    )
    o.append(
        '<text x="24" y="54" fill="#9aa0a6" font-size="12">'
        "before: sccache main 33d12fb · after: per-level backend-read-timeout backoff (this PR)</text>"
    )
    o.append(
        '<text x="24" y="72" fill="#9aa0a6" font-size="12">'
        "WebDAV GETs stop answering, PUTs succeed · SCCACHE_CACHE_READ_TIMEOUT_SECS=1 · medians of 3 alternating runs</text>"
    )
    o.append(
        '<text x="24" y="90" fill="#9aa0a6" font-size="12">'
        "box: Apple M3 Max, AC power, high power mode, load ~4 · rustc 1.91.0 (sccache MSRV) · lower is better · linear scale</text>"
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

    peak = STANDALONE_PEAK
    y = TOP + 14
    for divider, label, v, reps, color in ROWS:
        if divider:
            y += 10
            o.append(f'<line x1="24" y1="{y}" x2="{width-24}" y2="{y}" stroke="#2a2e35"/>')
            y += 20
            o.append(
                f'<text x="{LEFT}" y="{y}" fill="#e8e8e8" font-size="13" font-weight="bold">{esc(label)}</text>'
            )
            peak = CHAIN_PEAK
            y += 10
            continue
        o.append(
            f'<text x="{LEFT}" y="{y + BAR_H - 11}" fill="#e8e8e8" font-size="13">{esc(label)}</text>'
        )
        bar_w = max(x(v, peak) - LEFT - LABEL_W, 2)
        o.append(
            f'<rect x="{LEFT+LABEL_W}" y="{y}" width="{bar_w}" height="{BAR_H}" fill="{color}" rx="3"/>'
        )
        if reps:
            o.append(
                f'<text x="{x(v,peak)+8}" y="{y + BAR_H - 12}" fill="#9aa0a6" font-size="11">{esc(reps)}</text>'
            )
        o.append(
            f'<text x="{x(v,peak)+8}" y="{y + BAR_H - 2}" fill="#c9cdd3" font-size="12">{v:.3f} s</text>'
        )
        y += BAR_H + BAR_GAP

    y += 6
    o.append(f'<line x1="24" y1="{y}" x2="{width-24}" y2="{y}" stroke="#2a2e35"/>')
    o.append(
        '<text x="24" y="'
        + str(y + 22)
        + '" fill="#c9cdd3" font-size="13">standalone: 33.6 → 4.4 s (7.6×), 29 of 32 waits avoided, 3 backend GETs instead of 32 · chain: 33.9 → 4.7 s (7.2×), the disk tier keeps serving hits while the remote level is skipped</text>'
    )
    o.append(
        '<text x="24" y="'
        + str(y + 42)
        + '" fill="#9aa0a6" font-size="12">workload: sequential, fresh daemon per run · the issue\u0027s parallel cargo -j4 shape is measured separately (reproducer table in the body) · cache_timeouts reads 32 on every arm</text>'
    )
    o.append("</svg>")

    Path(sys.argv[sys.argv.index("--out") + 1]).write_text(
        "\n".join(o) + "\n", encoding="utf-8"
    )
    print(f"wrote {sys.argv[sys.argv.index('--out') + 1]}")


if __name__ == "__main__":
    sys.exit(main())
