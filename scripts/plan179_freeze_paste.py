#!/usr/bin/env python3
"""One-shot: paste the freeze e2e's emitted record into generations.rs.

Usage: py scripts/plan179_freeze_paste.py <capture-file> <v2|v3>

Extracts the emitted EXCLUDED_IDS / FROZEN_LABELS blocks from the freeze
run's captured stdout and replaces the corresponding static declarations
in src/score_bench/generations.rs (the v2 pair or the v3 pair).
"""

import re
import sys
from pathlib import Path


def extract_block(text: str, name: str) -> list[str]:
    lines = text.splitlines()
    start = None
    for i, line in enumerate(lines):
        if line.startswith(f"pub static {name}: "):
            start = i
            break
    if start is None:
        raise SystemExit(f"no emitted block for {name}")
    out = []
    for line in lines[start:]:
        out.append(line)
        if line == "];":
            break
    return out


def main() -> int:
    capture, which = Path(sys.argv[1]), sys.argv[2]
    suffix = "_V3" if which == "v3" else ""
    text = capture.read_text(encoding="utf-8", errors="replace")
    exc = extract_block(text, "EXCLUDED_IDS" + suffix)
    lab = extract_block(text, "FROZEN_LABELS" + suffix)
    gen = Path("src/score_bench/generations.rs")
    src = gen.read_text(encoding="utf-8")
    for name, block in (("EXCLUDED_IDS" + suffix, exc), ("FROZEN_LABELS" + suffix, lab)):
        pat = re.compile(
            r"^pub static " + re.escape(name) + r": .*\n(?:.*\n)*?\];$", re.MULTILINE
        )
        matches = pat.findall(src)
        if len(matches) != 1:
            raise SystemExit(f"expected exactly 1 declaration of {name}, found {len(matches)}")
        src = src.replace(matches[0], "\n".join(block))
    gen.write_text(src, encoding="utf-8", newline="\n")
    print(f"pasted {which}: {len(exc)}-line EXCLUDED block, {len(lab)}-line LABELS block")
    return 0


if __name__ == "__main__":
    sys.exit(main())
