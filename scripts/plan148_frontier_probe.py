#!/usr/bin/env python3
"""Plan 148 T1 — probe the Batch-157 re-arm candidates with REAL clippy.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case.

Re-arm discipline (Plan 146 T0's): seed must be clean under the FULL oracle
(incl. the missing_const_for_fn nursery discipline — `&mut` params no longer
block const-ability since const_mut_refs stabilized, so seeds dodge via
non-const OPS: format!/Vec::push/iterator adapters), broken must fire
EXACTLY its target lint.
"""
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

CASES = {
    # ── to_string_in_format_args (pedantic; the Batch 156 verified spare,
    #    re-verified for the record): inlined seed dodges
    #    uninlined_format_args; the literal prefix dodges useless_format;
    #    format! keeps the fn non-const-able. ──
    "tsifa-seed": (
        "fn show(x: u32) -> String { format!(\"n={x}\") }"
    ),
    "tsifa-broken": (
        "fn show(x: u32) -> String { format!(\"n={}\", x.to_string()) }"
    ),
    # ── manual_range_contains (style/per; Bench 094 wave-2 evidence 121
    #    warnings / 5 crates): `x >= 3 && x <= 5` → `(3..=5).contains(&x)`.
    #    Vec::push keeps the fn non-const-able (allocation). ──
    "mrc-seed": (
        "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); (3..=5).contains(&x) }"
    ),
    "mrc-broken": (
        "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && x <= 5 }"
    ),
    # mrc v2 (iterator adapter dodge — non-const via .iter().filter):
    "mrc2-seed": (
        "fn band_count(rows: &[u8]) -> usize { rows.iter().filter(|x| (3..=5).contains(x)).count() }"
    ),
    "mrc2-broken": (
        "fn band_count(rows: &[u8]) -> usize { rows.iter().filter(|x| **x >= 3 && **x <= 5).count() }"
    ),
    # ── iter_next_slice (perf; the iter_skip_next companion — `.first()`
    #    instead of `.iter().next()`): format! keeps it non-const-able. ──
    "ins-seed": (
        "fn head(v: &[u8]) -> String { format!(\"{:?}\", v.first()) }"
    ),
    "ins-broken": (
        "fn head(v: &[u8]) -> String { format!(\"{:?}\", v.iter().next()) }"
    ),
    # ── single_char_addstr (style, default): push_str("x") → push('x').
    #    String::push is non-const (interior mutability), seed form clean. ──
    "sca-seed": (
        "fn mark(s: &mut String) { s.push('!'); }"
    ),
    "sca-broken": (
        "fn mark(s: &mut String) { s.push_str(\"!\"); }"
    ),
}

def probe(name: str, code: str) -> list[str]:
    root = Path(tempfile.mkdtemp(prefix=f"probe148_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe148\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
            "[lib]\npath = \"src/lib.rs\"\n",
            encoding="utf-8",
        )
        (root / "src").mkdir()
        # mirror the oracle wrap: dead-code/unused silences, no clippy change
        (root / "src" / "lib.rs").write_text(
            "#![allow(dead_code, unused)]\n" + code + "\n",
            encoding="utf-8",
        )
        out = subprocess.run(
            ["cargo", "clippy", "--message-format=json", "--",
             "-W", "clippy::all", "-W", "clippy::pedantic", "-W", "clippy::nursery"],
            cwd=root, capture_output=True, text=True, timeout=120,
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
    return 0

if __name__ == "__main__":
    sys.exit(main())
