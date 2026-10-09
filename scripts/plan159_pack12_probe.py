#!/usr/bin/env python3
"""Plan 159 probe — the Batch 168 pack XII candidates (2026-09-21).

Verifies, against REAL clippy (JSON output, plain `cargo clippy`), that the
pack's two leads fire on their natural fixtures and that the one measured
silence the empty-line rule mirrors is real. Run from this repo's root:

    py scripts/plan159_pack12_probe.py

The probe crate is built under /tmp (isolated target dir) and removed on
exit. Exit 0 = every expectation held; exit 1 = a row drifted (the batch
lead is no longer probe-clean — re-adjudicate before landing).
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
# Keep this instrument's verdict printable on a non-UTF-8 console
# (katgpt-rs Issue 804 / the 928 drift census): it prints non-ASCII glyphs,
# and print() raises UnicodeEncodeError on e.g. cp874 — the process then dies
# with NO verdict. backslashreplace degrades the glyph visibly and keeps
# ASCII exact, so a verdict line stays greppable. Best-effort: a detached or
# captured stream is left alone rather than made fatal at import.
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(errors="backslashreplace")
    except (AttributeError, ValueError):
        pass

FIXTURES = {
    # lint -> (source fragment, expected FIRING lint count for it)
    "empty_line_after_doc_comments": ("/// Adds one to the input.\n\nfn eld(x: u8) -> u8 {\n    x + 1\n}", 1),
    "new_without_default": (
        "pub struct Widg { pub x: i32 }\nimpl Widg { pub fn new() -> Self { Widg { x: 1 } } }",
        1,
    ),
    # staged alternates (fire, but bounded/scoped — see Plan 159's table)
    "match_ref_pats": ("pub fn mrf(f: &Fruit) -> u8 {\n    match f {\n        &Fruit::Apple => 1,\n        &Fruit::Pear => 2,\n    }\n}", 1),
    "manual_div_ceil": ("pub fn mdc(a: usize, b: usize) -> usize {\n    (a + b - 1) / b\n}", 1),
    "into_iter_on_ref": ("pub fn iir(v: Vec<u8>) -> usize {\n    (&v).into_iter().count()\n}", 1),
}

SILENT_SHAPES = {
    # the measured upstream silence the eld guard mirrors: an attribute
    # between the doc comment and the item NEVER fires the lint
    "empty_line_after_doc_comments+attr-intervened": "/// Docs for bar.\n\n#[derive(Debug)]\npub struct Bar { pub y: u8 }",
}

# support items the fixtures reference
SUPPORT = "pub enum Fruit { Apple, Pear }\n"

LIB_HEAD = "// plan159 probe fixtures (generated — do not edit)\n"


def run_clippy(lib_rs: Path, target: Path) -> dict[str, list[dict]]:
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(target)
    proc = subprocess.run(
        ["cargo", "clippy", "--message-format=json", "--quiet"],
        cwd=lib_rs.parent.parent,
        env=env,
        capture_output=True, text=True,
    )
    found: dict[str, list[dict]] = {}
    for line in proc.stdout.splitlines():
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        d = msg["message"]
        code = (d.get("code") or {}).get("code", "")
        if not code.startswith("clippy::"):
            continue
        lint = code.removeprefix("clippy::")
        found.setdefault(lint, []).append(d)
    return found


def main() -> int:
    crate = Path(tempfile.mkdtemp(prefix="plan159_probe_"))
    target = Path(tempfile.mkdtemp(prefix="plan159_probe_t_"))
    ok = True
    try:
        (crate / "src").mkdir()
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "plan159probe"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n', encoding="utf-8"
        )
        parts = [LIB_HEAD, SUPPORT]
        for name, (frag, _) in FIXTURES.items():
            parts.append(frag)
        for frag in SILENT_SHAPES.values():
            parts.append(frag)
        parts.append("pub fn use_all() { let _ = eld(1); let _ = mrf(&Fruit::Apple); let _ = mdc(10, 3); let _ = iir(vec![1, 2]); let _ = Widg::new(); }\n")
        lib = crate / "src" / "lib.rs"
        lib.write_text("\n".join(parts), encoding="utf-8")

        found = run_clippy(lib, target)

        for name, (_, expected) in FIXTURES.items():
            got = len(found.get(name, []))
            verdict = "OK" if got == expected else "DRIFT"
            if got != expected:
                ok = False
            print(f"{verdict} {name}: fires {got} (expected {expected})")

        for shape, _ in SILENT_SHAPES.items():
            lint = shape.split("+")[0]
            got = len(found.get(lint, []))
            # the silent fixture must contribute ZERO fires of its lint
            # (the other fixtures of the same lint account for its expected
            # count — any extra fire is the silent shape firing)
            base = dict.fromkeys(FIXTURES, 0)
            base["empty_line_after_doc_comments"] = FIXTURES["empty_line_after_doc_comments"][1]
            verdict = "OK" if got <= base.get(lint, 0) else "DRIFT"
            if got > base.get(lint, 0):
                ok = False
            print(f"{verdict} {shape}: silent holds ({got} total for {lint})")

        print("probe verdict:", "PASS" if ok else "FAIL")
        return 0 if ok else 1
    finally:
        shutil.rmtree(crate, ignore_errors=True)
        shutil.rmtree(target, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
