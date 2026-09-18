#!/usr/bin/env python3
"""Plan 154 T1 — probe the Batch-163 leads (`manual_string_new` +
`needless_late_init`) with REAL clippy: group pins, suggestion rendering,
and the proposers' guard-boundary matrix.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case, the
rendered suggestions for the lead lints (the proposer design input), and the
default-group arms (a hit with no -W flags = default-level).

Guard arms (design questions the proposers must answer):
  msn-content        String::from("x")          — expect SILENT (empty-only)
  msn-var            String::from(s), s = ""    — expect SILENT (literal-only)
  msn-turbofish      <String as From<&str>>::from("")  — fire? (we refuse)
  msn-method         "".to_string()             — different lint family?
  msn-nested         Some(String::from(""))     — fires in place?
  nli-annotated      let x: u32; x = 5u32;      — suggestion carries the type?
  nli-intervening    let x; push(); x = 5;      — clippy fires between stmts?
  nli-ifelse         let x; if c { x = 1; } else { x = 2; }  — fires?
  nli-call-init      let x; x = mk(hits);       — fires (call rhs)?
  nli-two-assigns    let x; x = 1; x = 2;       — fires on the first?
  nli-question       let x; x = f()?;           — fires? (we refuse)
  nli-mut            let mut x; x = 5;          — fires (mut carried)?
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
    # ── manual_string_new ──
    "msn-seed": (
        'fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::new() }'
    ),
    "msn-broken": (
        'fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from("") }'
    ),
    "msn-content": (
        'fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from("x") }'
    ),
    "msn-var": (
        'fn mk(s: &str, hits: &mut Vec<u8>) -> String { hits.push(0); String::from(s) }'
    ),
    "msn-lit-var": (
        'fn mk(hits: &mut Vec<u8>) -> String { let s = ""; hits.push(0); String::from(s) }'
    ),
    "msn-turbofish": (
        'fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); <String as From<&str>>::from("") }'
    ),
    "msn-method": (
        'fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); "".to_string() }'
    ),
    "msn-nested": (
        'fn mk(hits: &mut Vec<u8>) -> Option<String> { hits.push(0); Some(String::from("")) }'
    ),
    # ── needless_late_init ──
    "nli-seed": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x = 5u32; x + x }"
    ),
    "nli-broken": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 5u32; x + x }"
    ),
    "nli-annotated": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x: u32; x = 5u32; x + x }"
    ),
    "nli-intervening": (
        "fn late(hits: &mut Vec<u8>) -> u32 { let x; hits.push(1); x = 5u32; hits.push(x); x + x }"
    ),
    "nli-ifelse": (
        "fn late(hits: &mut Vec<u8>, c: bool) -> u32 { hits.push(0); let x; "
        "if c { x = 1u32; } else { x = 2u32; } x + x }"
    ),
    "nli-call-init": (
        "fn mk(hits: &mut Vec<u8>) -> u32 { hits.push(9); 7 }\n"
        "fn late(hits: &mut Vec<u8>) -> u32 { let x; x = mk(hits); x + x }"
    ),
    "nli-two-assigns": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 1u32; x = 2u32; x + x }"
    ),
    "nli-question": (
        "fn parse(hits: &mut Vec<u8>) -> Result<u32, String> { hits.push(0); Ok(5) }\n"
        "fn late(hits: &mut Vec<u8>) -> Result<u32, String> { let x; x = parse(hits)?; Ok(x + x) }"
    ),
    "nli-mut": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let mut x; x = 5u32; x + x }"
    ),
    # negative control: known-clean shape under the full oracle.
    "neg-ctl": (
        "fn plain(x: u32, hits: &mut Vec<u32>) -> bool { hits.push(x); x > 0 }"
    ),
}

# Group-pin arms (no -W flags): a hit here = default-level lint — the corpus
# `lint_group` pins for the two Batch-163 leads.
GROUP_CASES = {
    "grp-msn-broken": (
        'fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from("") }'
    ),
    "nli-grp-broken": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 5u32; x + x }"
    ),
}


def probe_full(name: str, code: str, default_groups: bool = False):
    """Returns (sorted lint ids, the rendered suggestions for lead lints)."""
    root = Path(tempfile.mkdtemp(prefix=f"probe154_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe154\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
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
        if suggestions and name.split("-")[0] in {"msn", "nli"}:
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
