"""Batch 296 count pins (Plan 301 T4): bump the kernel corpus count value
981 -> 982 at every assert site, prepend the batch-296 history clause to each
site's message, bump the DtypeAlignment category row 85 -> 86, and bump the
router expert-total pin.

Shape-asserted: every rewrite matches an exact expected form; any mismatch
aborts loudly BEFORE any write."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TESTS = ROOT / "src" / "domains" / "kernel_opt" / "corpus" / "tests.rs"
ROUTER = ROOT / "src" / "domains" / "kernel_opt" / "router.rs"

HISTORY = (
    "batch 296 re-pin: 982 (981 + 1: N vector-width-admission-checks-every-"
    "outer-stride -- the cubecl #1922 all-strides vector-admission law, "
    "merge 5bd6d17aa883, Plan 301 / Research 337); prior: "
)


def main() -> None:
    lines = TESTS.read_text(encoding="utf-8").split("\n")
    assert not any("batch 296 re-pin" in ln for ln in lines), "already re-pinned"

    val_re = re.compile(r"^(\s*)981,\s*$")
    msg_re = re.compile(r'^(\s*)"(.*)$')

    out: list[str] = []
    n_values = 0
    n_heads = 0
    i = 0
    while i < len(lines):
        line = lines[i]
        m = val_re.match(line)
        if m:
            n_values += 1
            out.append(f"{m.group(1)}982,")
            # find the message-start line: the next non-empty line must open
            # a string literal ("...). Prepend the history clause inside it.
            j = i + 1
            while j < len(lines) and not lines[j].strip():
                out.append(lines[j])
                j += 1
            assert j < len(lines), f"no message line after value at line {i + 1}"
            mm = msg_re.match(lines[j])
            assert mm, f"line {j + 1} does not open a message string"
            indent, rest = mm.groups()
            out.append(f'{indent}"{HISTORY}{rest}')
            n_heads += 1
            i = j + 1
            continue
        out.append(line)
        i += 1

    assert n_values == 35, f"expected 35 value sites, found {n_values}"
    assert n_heads == 35, f"expected 35 message heads, found {n_heads}"
    text = "\n".join(out)

    # the DtypeAlignment category row 85 -> 86 with a comment
    cat_old = "(KernelCategory::DtypeAlignment, 85), //"
    cat_new = (
        "(KernelCategory::DtypeAlignment, 86), // Batch 296 +1: "
        "vector-width-admission-checks-every-outer-stride (the cubecl #1922 "
        "all-strides admission law, Plan 301 / Research 337) //"
    )
    assert text.count(cat_old) == 1, "expected 1 DtypeAlignment row"
    text = text.replace(cat_old, cat_new)

    TESTS.write_text(text, encoding="utf-8")
    print(f"tests.rs: 35 values bumped, 35 heads prefixed, category 85 -> 86")

    # the router expert-total pin
    rtext = ROUTER.read_text(encoding="utf-8")
    r_old = "            total, 981,\n"
    assert rtext.count(r_old) == 1, "expected 1 router value pin"
    rtext = rtext.replace(r_old, "            total, 982,\n", 1)
    r_head_old = (
        '            "Plan 003: experts cover all 981 corpus rules (Batch 295 +4: '
    )
    assert rtext.count(r_head_old) == 1, "expected 1 router head pin"
    r_head_new = (
        '            "Plan 003: experts cover all 982 corpus rules (Batch 296 +1: '
        "N vector-width-admission-checks-every-outer-stride, the cubecl #1922 "
        "all-strides admission law, merge 5bd6d17aa883, Plan 301 / Research 337; "
        "prior: Batch 295 +4: "
    )
    rtext = rtext.replace(r_head_old, r_head_new, 1)
    ROUTER.write_text(rtext, encoding="utf-8")
    print("router.rs: expert-total 981 -> 982 + history head")


if __name__ == "__main__":
    main()
