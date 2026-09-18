#!/usr/bin/env python3
"""Plan 150 T8 — splice the regenerated freeze tables into generations.rs.

The deterministic regen (examples/freeze_regen_tmp.rs) printed the new
FROZEN_LABELS + EXCLUDED_IDS tables from the CURRENT specs() derivation +
the known bad-atom sets. This script replaces the two v2 tables and the two
v3 tables in place, keeping the surrounding docs/comments intact.

The freeze e2e (`--ignored`, real clippy) verifies the labels afterward —
0 drift expected (labels ARE expect_lints; exclusions ARE bad-atom windows).
"""
import re
import sys
from pathlib import Path

SRC = Path("src/score_bench/generations.rs")
OUT = Path("freeze_regen_out.txt")


def parse_sections() -> dict[str, list[str]]:
    data = OUT.read_text(encoding="utf-8", errors="replace").splitlines()
    sections: dict[str, list[str]] = {}
    current: str | None = None
    for line in data:
        if "EXCLUDED" in line or "LABELS" in line:
            m = re.search(r"(v[23]) (EXCLUDED|LABELS)", line)
            current = f"{m.group(1)}_{m.group(2)}" if m else None
            sections[current] = []
        elif current and line.strip():
            sections[current].append(line)
    return sections


def build_static(rows: list[str], tuple_form: bool) -> list[str]:
    out = ["&["]
    for row in rows:
        body = row.strip().rstrip(",")
        out.append(f"    {body},")
    out.append("];")
    return out


def replace_static(src: str, header: str, new_lines: list[str]) -> str:
    """Replace `header` .. matching `];` with header + rows + `];`."""
    start = src.index(header)
    end = src.index("];", start) + len("];")
    replacement = header + "\n" + "\n".join(new_lines[1:])
    return src[:start] + replacement + src[end:]


def main() -> int:
    sec = parse_sections()
    src = SRC.read_text(encoding="utf-8")
    # v2 tables
    src = replace_static(src, "pub static EXCLUDED_IDS: &[&str] = &[", build_static(sec["v2_EXCLUDED"], False))
    src = replace_static(src, "pub static FROZEN_LABELS: &[(&str, &[&str])] = &[", build_static(sec["v2_LABELS"], True))
    # v3 tables
    src = replace_static(src, "pub static EXCLUDED_IDS_V3: &[&str] = &[", build_static(sec["v3_EXCLUDED"], False))
    src = replace_static(src, "pub static FROZEN_LABELS_V3: &[(&str, &[&str])] = &[", build_static(sec["v3_LABELS"], True))
    SRC.write_text(src, encoding="utf-8", newline="")
    n = {k: len(v) for k, v in sec.items()}
    print(f"spliced: {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
