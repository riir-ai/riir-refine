#!/usr/bin/env python3
"""Plan 152 T1 — probe the Batch-161 leads' GROUPS + the proposer shape
boundaries + the re-arm candidates with REAL clippy.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case
(+ the rendered suggestion for the two leads' broken forms — the proposer
design input).

Group-pin arms (the leads being LANDED as rules this batch): the Batch 160
miner comment claimed string_lit_as_bytes was a "default-level perf lint" —
the docs say NURSERY. A hit in the bare arm (no -W flags) = default-level;
no hit = allow-by-default group (nursery/pedantic/restriction). unit_arg is
expected complexity/default-on; the pin is owed either way (the corpus
`lint_group` field is a claim about clippy, not about our oracle).

Shape arms (the proposers' guard decisions):
  - ua-fn-call: a unit-RETURNING CALL as the arg — clippy's coverage vs our
    v1 refusal (call-arg side effects are hoist-incompatible at syn level).
  - ua-idx1: the unit block at arg index 1 — clippy fires (doc example
    shape); our proposer REFUSES (hoisting would reorder evaluation past
    arg 0).
  - ua-empty: `{}` as the arg — the in-place `()` arm.
  - ua-value-tail: a block WITH a tail expression (non-unit) — expected
    SILENT (the syntactic refusal twin: the block's value is live).
  - ua-semi: the canonical broken in statement-with-semicolon position.
  - slb-raw / slb-escape / slb-unicode-escape / slb-nonascii: the literal
    snippet boundary matrix (raw prefix, \\n escape, \\u{41} — ASCII value
    but escape spelling, non-ASCII content).

Re-arm candidates (seed clean / broken fires EXACTLY its target, both
under the full oracle — the re-arm discipline; all four are the plan151
verified-but-not-armed alternates):
  - bctl (bytes_count_to_len), bc (bool_comparison),
    msn (manual_string_new), nli (needless_late_init — the x+x dodge,
    clearing the let_and_return crossfire on the naive seed)
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
    # The Batch-160 committed pair, re-verified (rows 14-15).
    "ua-seed": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch((), hits) }"
    ),
    "ua-broken": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }"
    ),
    # A unit-returning CALL as the arg (no side-effect block).
    "ua-fn-call": (
        "fn unit_fn() {}\n"
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch(unit_fn(), hits) }"
    ),
    # The unit block at arg index 1 — clippy's own doc-example shape.
    "ua-idx1": (
        "fn touch(hits: &mut Vec<u8>, x: ()) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch(hits, { hits.push(1); }) }"
    ),
    # An EMPTY block as the arg — the in-place `()` arm.
    "ua-empty": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch({}, hits) }"
    ),
    # A block WITH a tail expression — the block's value is live; the lint
    # must stay silent (the syntactic refusal twin).
    "ua-value-tail": (
        "fn num(hits: &mut Vec<u32>) -> u32 { hits.push(0); 1 }\n"
        "fn add(x: u32, hits: &mut Vec<u32>) -> u32 { hits.push(x); x }\n"
        "fn go(hits: &mut Vec<u32>) -> u32 { add({ num(hits) }, hits) }"
    ),
    # The canonical broken in statement-with-semicolon position.
    "ua-semi": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits); }"
    ),
    # string_lit_as_bytes pair, re-verified.
    "slb-seed": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); b\"abc\" }"
    ),
    "slb-broken": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"abc\".as_bytes() }"
    ),
    # Raw string receiver.
    "slb-raw": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); r\"a\\nc\".as_bytes() }"
    ),
    # Escaped literal content (byte-string-valid escapes).
    "slb-escape": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"a\\nb\".as_bytes() }"
    ),
    # ASCII VALUE spelled as a unicode escape — the snippet boundary
    # (byte strings cannot spell \\u{..}).
    "slb-unicode-escape": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"\\u{41}\".as_bytes() }"
    ),
    # Non-ASCII literal content — clippy's lint is ASCII-scoped.
    "slb-nonascii": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"h\u00e9llo\".as_bytes() }"
    ),
    # re-arm candidate: bytes_count_to_len — `s.bytes().count()` -> `s.len()`.
    "bctl-seed": (
        "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.len() }"
    ),
    "bctl-broken": (
        "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes().count() }"
    ),
    # re-arm candidate: bool_comparison — `x == true` -> `x`.
    "bc-seed": (
        "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x }"
    ),
    "bc-broken": (
        "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x == true }"
    ),
    # re-arm candidate: manual_string_new — `String::from("")` -> `String::new()`.
    "msn-seed": (
        "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::new() }"
    ),
    "msn-broken": (
        "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from(\"\") }"
    ),
    # re-arm candidate: needless_late_init — `let x; x = 5;` -> `let x = 5;`
    # (the x+x tail dodge clears the let_and_return crossfire on the seed).
    "nli-seed": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x = 5u32; x + x }"
    ),
    "nli-broken": (
        "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 5u32; x + x }"
    ),
    # negative control: known-clean shape under the full oracle.
    "neg-ctl": (
        "fn plain(x: u32, hits: &mut Vec<u32>) -> bool { hits.push(x); x > 0 }"
    ),
}

# Group-pin arms (no -W flags): a hit here = default-level lint — the corpus
# `lint_group` pins for the two Batch-161 leads.
GROUP_CASES = {
    "grp-ua-broken": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }"
    ),
    "grp-slb-broken": (
        "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"abc\".as_bytes() }"
    ),
}


def probe_full(name: str, code: str, default_groups: bool = False):
    """Returns (sorted lint ids, the rendered suggestions for lead lints)."""
    root = Path(tempfile.mkdtemp(prefix=f"probe152_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe152\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
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
        if suggestions and name.split("-")[0] in {"ua", "slb"}:
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
