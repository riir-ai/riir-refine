#!/usr/bin/env python3
"""Plan 151 T1 — probe the Batch-160 leads' GROUPS + the proposer shape
boundaries + the re-arm candidates with REAL clippy.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case.

Group-pin arms (the leads being LANDED as rules this batch): the plan150
probe verified both leads' fixtures under the FULL oracle (-W all
-W pedantic -W nursery) — which proves the lint FIRES but not WHICH group
it belongs to (the union makes pedantic vs default ambiguous). A hit in
the bare arm (no -W flags) = default-level; the corpus `lint_group` pin
is owed for both entries.

Shape arms (the proposers' guard decisions):
  - aop-float / aop-bitop / aop-shift / aop-param: does the lint fire on
    float receivers, bitwise/shift ops, and mut-PARAM shapes (the
    derivation's admitted set must track clippy's coverage).
  - aop-swapped: `y = 1 + y` — expected misrefactored_assign_op (the
    family boundary our proposer refuses).
  - fsr-signed / fsr-suffixed: signed receivers + a suffixed radix
    literal.

Re-arm candidates (seed clean / broken fires EXACTLY its target, both
under the full oracle — the re-arm discipline):
  - ua (unit_arg — the plan150-verified spare, side-effect-block dodge)
  - slb (string_lit_as_bytes), msn (manual_string_new),
    bc (bool_comparison), bctl (bytes_count_to_len),
    nli (needless_late_init)
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

# ── shape arms (full oracle) ──
CASES = {
    # assign_op_pattern coverage: float receiver.
    "aop-float": (
        "fn z(x: f64, hits: &mut Vec<f64>) -> f64 { hits.push(x); let mut y = x; y = y + 1.0; y }"
    ),
    # bitwise + shift ops.
    "aop-bitop": (
        "fn m(x: u8, hits: &mut Vec<u8>) -> u8 { hits.push(x); let mut y = x; y = y | 3; y }"
    ),
    "aop-shift": (
        "fn sh(x: u32, hits: &mut Vec<u32>) -> u32 { hits.push(x); let mut y = x; y = y << 2; y }"
    ),
    # mut-PARAM shape (derivation leg (a)).
    "aop-param": (
        "fn grow(mut p: i32, hits: &mut Vec<i32>) -> i32 { hits.push(p); p = p + 5; p }"
    ),
    # The swapped operand — the misrefactored_assign_op family boundary.
    "aop-swapped": (
        "fn s(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = 1 + y; y }"
    ),
    # from_str_radix_10 coverage: signed + suffixed radix.
    "fsr-signed": (
        "fn num(s: &str, hits: &mut Vec<i8>) -> Result<i8, std::num::ParseIntError>"
        " { hits.push(0); i8::from_str_radix(s, 10) }"
    ),
    "fsr-suffixed": (
        "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError>"
        " { hits.push(0); u32::from_str_radix(s, 10u32) }"
    ),
    # re-arm candidate: unit_arg (the verified spare — side-effect-block
    # dodge; both dirt classes from plan149 cleared).
    "ua-seed": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch((), hits) }"
    ),
    "ua-broken": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }"
    ),
    # re-arm candidate: string_lit_as_bytes — `"abc".as_bytes()` → `b"abc"`.
    "slb-seed": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); b\"abc\" }"
    ),
    "slb-broken": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"abc\".as_bytes() }"
    ),
    # re-arm candidate: manual_string_new — `String::from("")` → `String::new()`.
    "msn-seed": (
        "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::new() }"
    ),
    "msn-broken": (
        "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from(\"\") }"
    ),
    # re-arm candidate: bool_comparison — `x == true` → `x`.
    "bc-seed": (
        "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x }"
    ),
    "bc-broken": (
        "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x == true }"
    ),
    # re-arm candidate: bytes_count_to_len — `s.bytes().count()` → `s.len()`.
    "bctl-seed": (
        "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.len() }"
    ),
    "bctl-broken": (
        "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes().count() }"
    ),
    # re-arm candidate: needless_late_init — `let x; x = 5;` → `let x = 5;`.
    "nli-seed": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x = 5u32; x }"
    ),
    "nli-broken": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 5u32; x }"
    ),
    # negative control: known-clean shape under the full oracle.
    "neg-ctl": (
        "fn plain(x: u32, hits: &mut Vec<u32>) -> bool { hits.push(x); x > 0 }"
    ),
}

# Group-pin arms (no -W flags): a hit here = default-level lint — the corpus
# `lint_group` pins for the two Batch-160 leads + the re-arm record.
GROUP_CASES = {
    "grp-fsr-broken": (
        "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError>"
        " { hits.push(0); u32::from_str_radix(s, 10) }"
    ),
    "grp-aop-broken": (
        "fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = y + 1; y }"
    ),
    "grp-ua-broken": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }"
    ),
}


def probe(name: str, code: str, default_groups: bool = False) -> list[str]:
    root = Path(tempfile.mkdtemp(prefix=f"probe151_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe151\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
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
    print("-- default-group arms (no -W flags; a hit = default-level) --")
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
