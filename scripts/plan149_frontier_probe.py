#!/usr/bin/env python3
"""Plan 149 T1 — probe the Batch-158 re-arm candidates with REAL clippy.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case.

Re-arm discipline (Plan 146 T0's): seed must be clean under the FULL oracle
(incl. the missing_const_for_fn nursery discipline — `&mut` params no longer
block const-ability since const_mut_refs stabilized, so seeds dodge via
non-const OPS: format!/Vec::push/String::push/iterator adapters), broken must
fire EXACTLY its target lint.

Extra arm this round: the `grp-*` cases run DEFAULT clippy (no -W groups) to
pin the lint GROUP of the two Batch-158 leads for the corpus entries
(`to_string_in_format_args` is expected perf/default-on; `manual_range_contains`
style/default-on — a default-arm hit confirms default-level).
"""
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# Every verdict below prints non-ASCII glyphs. On a console whose encoding is
# not UTF-8 — the Windows workstation that runs these is cp874 — print() raises
# UnicodeEncodeError and this script dies with NO output at all, so its
# findings are not *unknown* but *unlooked at* (katgpt-rs Issue 804).
# backslashreplace degrades the glyph to a visible escape and keeps every ASCII
# character exact, so a verdict line stays greppable; forcing UTF-8 instead
# gives mojibake, which is the silent direction.
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(errors="backslashreplace")
    except (AttributeError, ValueError):
        pass  # not a TextIOWrapper (embedded / detached); keep old behavior


CASES = {
    # ── single_char_add_str (style, default): the Batch 156/157 verified
    #    spare, re-verified for the record. String::push is non-const
    #    (interior mutability), seed form clean. ──
    "sca-seed": (
        "fn mark(s: &mut String) { s.push('!'); }"
    ),
    "sca-broken": (
        "fn mark(s: &mut String) { s.push_str(\"!\"); }"
    ),
    # ── unit_arg (style, default; Bench 094 wave-4 evidence 46/2): shape
    #    candidates — the explicit-`()`-to-unit-param form and the
    #    unit-expr-block form. ──
    "ua1-seed": (
        "fn touch(x: ()) {}\nfn go() { touch(()) }"
    ),
    "ua1-broken": (
        "fn touch(x: ()) {}\nfn go() { touch({ 1; }) }"
    ),
    "ua2-broken": (
        "fn touch(x: ()) {}\nfn go() { let u = { 1; }; touch(u) }"
    ),
    # ── manual_clamp (style, default; fallback): `x.max(lo).min(hi)` ↔
    #    `x.clamp(lo, hi)`. Vec::push keeps the fn non-const-able. ──
    "mc-seed": (
        "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.clamp(3, 5) }"
    ),
    "mc-broken": (
        "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(3).min(5) }"
    ),
    # ── from_str_radix_10 (style, default; fallback): parse via radix 10.
    #    A Result return + ? keeps things non-const-able. ──
    "fsr-seed": (
        "fn num(s: &str) -> Result<u32, std::num::ParseIntError> { s.parse::<u32>() }"
    ),
    "fsr-broken": (
        "fn num(s: &str) -> Result<u32, std::num::ParseIntError> { u32::from_str_radix(s, 10) }"
    ),
    # ── negative control: known-clean shape under the full oracle. ──
    "neg-ctl": (
        "fn plain(x: u32, hits: &mut Vec<u32>) -> bool { hits.push(x); x > 0 }"
    ),
}

# Default-group arms (no -W flags): a hit here = default-level lint.
GROUP_CASES = {
    "grp-tsifa-broken": (
        "fn show(x: u32) -> String { format!(\"n={}\", x.to_string()) }"
    ),
    "grp-mrc-broken": (
        "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && x <= 5 }"
    ),
    "grp-sca-broken": (
        "fn mark(s: &mut String) { s.push_str(\"!\"); }"
    ),
}


def probe(name: str, code: str, default_groups: bool = False) -> list[str]:
    root = Path(tempfile.mkdtemp(prefix=f"probe149_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe149\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
            "[lib]\npath = \"src/lib.rs\"\n",
            encoding="utf-8",
        )
        (root / "src").mkdir()
        # mirror the oracle wrap: dead-code/unused silences, no clippy change
        (root / "src" / "lib.rs").write_text(
            "#![allow(dead_code, unused)]\n" + code + "\n",
            encoding="utf-8",
        )
        cmd = ["cargo", "clippy", "--message-format=json", "--"]
        if not default_groups:
            cmd += ["-W", "clippy::all", "-W", "clippy::pedantic", "-W", "clippy::nursery"]
        out = subprocess.run(
            cmd,
            cwd=root, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=120,
        )
        lints = set()
        for line in out.stdout.splitlines():
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if msg.get("reason") == "compiler-message":
                codev = msg.get("message", {}).get("code", {})
                if codev and codev.get("code", "").startswith("clippy::"):
                    lints.add(codev["code"])
        return sorted(lints)
    finally:
        shutil.rmtree(root, ignore_errors=True)


def main() -> int:
    for name, code in CASES.items():
        try:
            lints = probe(name, code)
        except Exception as e:  # noqa: BLE001
            print(f"{name:28} PROBE-ERROR {e}")
            continue
        print(f"{name:28} {lints}")
    print("── default-group arms (no -W flags; a hit = default-level) ──")
    for name, code in GROUP_CASES.items():
        try:
            lints = probe(name, code, default_groups=True)
        except Exception as e:  # noqa: BLE001
            print(f"{name:28} PROBE-ERROR {e}")
            continue
        print(f"{name:28} {lints}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
