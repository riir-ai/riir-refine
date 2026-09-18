#!/usr/bin/env python3
"""Plan 147 T1 — probe the Batch-156 re-arm candidates with REAL clippy.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case.

Re-arm discipline (Plan 146 T0's): seed must be clean under the FULL oracle
(incl. the missing_const_for_fn nursery discipline — seeds are non-const-able
by construction), broken must fire EXACTLY its target lint.
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
    # ── round 2 (2026-09-18): refined + fresh re-arm candidates ──
    # match_ref_pats v2: ALL arms ref-patterns (binding form — exhaustive,
    # no wildcard; the round-1 wildcard form measured SILENT).
    "mrp2-seed": (
        "fn label(v: u8) -> String { match v { x => format!(\"val-{x}\") } }"
    ),
    "mrp2-broken": (
        "fn label(v: u8) -> String { match &v { &x => format!(\"val-{x}\") } }"
    ),
    # redundant_closure_for_method_calls v2 (String::len — implicit_clone-free).
    "rcmc2-seed": (
        "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(String::len).collect() }"
    ),
    "rcmc2-broken": (
        "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }"
    ),
    # manual_strip (perf, default): starts_with+slice → strip_prefix.
    "strip-seed": (
        "fn cut(s: &str, p: &str) -> &str { if let Some(rest) = s.strip_prefix(p) { rest } else { s } }"
    ),
    "strip-broken": (
        "fn cut(s: &str, p: &str) -> &str { if s.starts_with(p) { &s[p.len()..] } else { s } }"
    ),
    # iter_skip_next (style, default): .skip(n).next() → .nth(n).
    "isn-seed": (
        "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().nth(n) }"
    ),
    "isn-broken": (
        "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(n).next() }"
    ),
    # to_string_in_format_args (pedantic, default-firing).
    "tsifa-seed": (
        "fn show(x: u32) -> String { format!(\"{}\", x) }"
    ),
    "tsifa-broken": (
        "fn show(x: u32) -> String { format!(\"{}\", x.to_string()) }"
    ),
    # ── round 3 (2026-09-18): finalize the re-arm picks ──
    # iter_skip_next v2: `.nth(n)` (clippy's own suggestion) fires iter_nth
    # on slice iters — the ORACLE-CLEAN form is `.get(n)`.
    "isn2-seed": (
        "fn at(v: &[u8], n: usize) -> Option<&u8> { v.get(n) }"
    ),
    "isn2-broken": (
        "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(n).next() }"
    ),
    # to_string_in_format_args v3: literal prefix dodges useless_format;
    # inlined seed dodges uninlined_format_args.
    "tsifa3-seed": (
        "fn show(x: u32) -> String { format!(\"n={x}\") }"
    ),
    "tsifa3-broken": (
        "fn show(x: u32) -> String { format!(\"n={}\", x.to_string()) }"
    ),
    # len_zero (style, default): v.len() == 0 → v.is_empty(); &mut param
    # blocks const-ability (missing_const_for_fn is nursery).
    "lz-seed": (
        "fn empty(v: &[u8], out: &mut usize) -> bool { *out = v.len(); v.is_empty() }"
    ),
    "lz-broken": (
        "fn empty(v: &[u8], out: &mut usize) -> bool { *out = v.len(); v.len() == 0 }"
    ),
    # iter_count_len (fresh probe): v.iter().count() → v.len().
    "icl-seed": (
        "fn n(v: &[u8]) -> usize { v.len() }"
    ),
    "icl-broken": (
        "fn n(v: &[u8]) -> usize { v.iter().count() }"
    ),
}

def probe(name: str, code: str) -> list[str]:
    root = Path(tempfile.mkdtemp(prefix=f"probe147_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe147\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
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
    return 0

if __name__ == "__main__":
    sys.exit(main())
