#!/usr/bin/env python3
"""Plan 155 T1 — probe the Batch-164 leads (`match_like_matches_macro` +
`excessive_precision`) with REAL clippy: group pins, suggestion rendering,
and the proposers' guard-boundary matrix.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case, the
rendered suggestions for the lead lints (the proposer design input), and the
default-group arms (a hit with no -W flags = default-level).

Lead provenance: the `--coverage-report` derivation of 2026-09-19 (the
recorded-spare queue ran EMPTY at Batch 163 — this is the fresh probe round
the snapshot's NEXT POOL line owes). Both leads carry default-eco evidence
from the Bench 094 manifest (663-crate default-level histogram):
  match_like_matches_macro  style  warn  count 21
  excessive_precision       style  warn  count 56   (rank 2 of the head)

Guard arms (design questions the proposers must answer):
  mlmm-guard        `0 if x > 1 => true`     — upstream fires? (we refuse)
  mlmm-three-arms   3 arms w/ duplicate out  — upstream fires? (we refuse)
  mlmm-binding      `v @ 0..=5 => true`      — binding carried into matches!?
  mlmm-named        `other => false` catchall — fires? (we refuse, `_` only)
  mlmm-nonbool      u32 arm bodies           — expect SILENT
  mlmm-call-scrutinee  `match mk(x)`         — fires (verbatim scrutinee OK)?
  mlmm-nested       `Some(match ...)`        — fires in place?
  ep-nosuffix       `let a: f32 = 2.1415...;` — fires? (we refuse: suffix-only)
  ep-separator      `1.000_000_1f32`         — fires? (we refuse)
  ep-exponent       `3.000...e-10f32`        — fires? suggestion keeps exponent?
  ep-ones           `1.0000000f32`           — suggestion `1f32` or `1.0f32`?
  ep-const          const item               — fires in place?
"""
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# Every verdict below prints non-ASCII glyphs. On a console whose encoding is
# not UTF-8 — print() raises UnicodeEncodeError and this script dies with NO
# output at all (katgpt-rs Issue 804). backslashreplace keeps the run readable.
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(errors="backslashreplace")
    except (AttributeError, ValueError):
        pass  # not a TextIOWrapper (embedded / detached); keep old behavior

CASES = {
    # ── match_like_matches_macro ──
    "mlmm-seed": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { 0 => true, _ => false } }"
    ),
    "mlmm-negated": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { 0 => false, _ => true } }"
    ),
    "mlmm-guard": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { 0 if x > 1 => true, _ => false } }"
    ),
    "mlmm-three-arms": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { 0 => true, 1 => false, _ => false } }"
    ),
    "mlmm-orpat": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { 0 | 1 => true, _ => false } }"
    ),
    "mlmm-binding": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { v @ 0..=5 => true, _ => false } }"
    ),
    "mlmm-call-scrutinee": (
        "fn mk(x: u32) -> u32 { x.wrapping_add(1) }\n"
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match mk(x) { 0 => true, _ => false } }"
    ),
    "mlmm-expr-pos": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "let a = match x { 0 => true, _ => false }; a == a }"
    ),
    "mlmm-named-catchall": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { 0 => true, other => false } }"
    ),
    "mlmm-nonbool": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); "
        "match x { 0 => 1, _ => 2 } }"
    ),
    "mlmm-nested": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> Option<bool> { hits.push(0); "
        "Some(match x { 0 => true, _ => false }) }"
    ),
    # ── excessive_precision ──
    "ep-seed": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); "
        "2.141592653589793238462643f32 }"
    ),
    "ep-f64": (
        "fn f(hits: &mut Vec<u8>) -> f64 { hits.push(0); "
        "2.141592653589793238462643f64 }"
    ),
    "ep-trailing-zeros": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 1.100000f32 }"
    ),
    "ep-ones": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 1.0000000f32 }"
    ),
    "ep-clean": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 0.1f32 }"
    ),
    "ep-nosuffix": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); "
        "let a: f32 = 2.141592653589793238462643; a }"
    ),
    "ep-separator": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 1.000_000_1f32 }"
    ),
    "ep-exponent": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 3.00000000000e-10f32 }"
    ),
    "ep-const": (
        "const A: f32 = 2.14159265358979f32;\n"
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); A }"
    ),
    "ep-tiny": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); "
        "0.12345678901234567890123456789f32 }"
    ),
    # negative control: known-clean shape under the full oracle.
    "neg-ctl": (
        "fn plain(x: u32, hits: &mut Vec<u32>) -> bool { hits.push(x); x > 0 }"
    ),
}

# Group-pin arms (no -W flags): a hit here = default-level lint — the corpus
# `lint_group` pins for the two Batch-164 leads (both style/warn).
GROUP_CASES = {
    "mlmm-grp-seed": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); "
        "match x { 0 => true, _ => false } }"
    ),
    "ep-grp-seed": (
        "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); "
        "2.141592653589793238462643f32 }"
    ),
}


def probe_full(name: str, code: str, default_groups: bool = False):
    """Returns (sorted lint ids, the rendered suggestions for lead lints)."""
    root = Path(tempfile.mkdtemp(prefix=f"probe155_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe155\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
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
        suggestions = []
        for line in out.stdout.splitlines():
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if msg.get("reason") != "compiler-message":
                continue
            m = msg.get("message", {})
            codev = m.get("code", {})
            cid = (codev or {}).get("code", "")
            if cid.startswith("clippy::"):
                lints.add(cid)
                for child in m.get("children", []):
                    for span in child.get("spans", []):
                        if span.get("suggested_replacement") is not None:
                            suggestions.append(
                                f"{cid}: {span['suggested_replacement']!r}"
                            )
        return sorted(lints), suggestions
    finally:
        shutil.rmtree(root, ignore_errors=True)


def main() -> int:
    for name, code in CASES.items():
        try:
            lints, suggestions = probe_full(name, code)
        except Exception as e:  # noqa: BLE001
            print(f"{name:28} PROBE-ERROR {e}")
            continue
        line = f"{name:28} {lints}"
        if suggestions and name.split("-")[0] in {"mlmm", "ep"}:
            line += "  suggestion=" + "; ".join(suggestions[:4])
        print(line)
    print("-- default-group arms (no -W flags; a hit = default-level) --")
    for name, code in GROUP_CASES.items():
        try:
            lints, _ = probe_full(name, code, default_groups=True)
        except Exception as e:  # noqa: BLE001
            print(f"{name:28} PROBE-ERROR {e}")
            continue
        print(f"{name:28} {lints}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
