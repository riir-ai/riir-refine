#!/usr/bin/env python3
"""Plan 163 probe — the Batch 171 pack XIV candidates (2026-09-21).

The coverage-report re-arm's next derivation (clippy 0.1.98, 822 lints /
92 covered) adjudicated down to two leads; this probe measures the
envelope the proposers need — firing shapes, upstream silences, co-fire
sets (the Batch-169 needless_borrow lesson), and clippy's OWN suggestion
text (the fix shape the healer must match or diverge-from-measured).

One clippy run PER FIXTURE (each fixture alone in the probe crate) —
attribution is exact by construction; no line arithmetic.

Run from this repo's root:

    py scripts/plan163_pack14_probe.py

Exit 0 = every expectation held; exit 1 = a row drifted (re-adjudicate
before authoring — the probe-first law).
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

# -- tabs_in_doc_comments ---------------------------------------------------
# TID_PLAIN: the canonical shape — a tab inside a /// line.
TID_PLAIN = (
    "///\tA doc comment with a leading tab.\n"
    "pub fn tid_a() -> u64 {\n    1\n}\n"
)
# TID_MID: a tab mid-text.
TID_MID = (
    "/// A doc comment with a\tmid-text tab.\n"
    "pub fn tid_b() -> u64 {\n    2\n}\n"
)
# TID_MULTI: two tabs.
TID_MULTI = (
    "///\t\tTwo tabs.\n"
    "pub fn tid_c() -> u64 {\n    3\n}\n"
)
# TID_INNER: inner doc comment (//!).
TID_INNER = (
    "//!\tModule doc with a tab.\n"
    "pub fn tid_d() -> u64 {\n    4\n}\n"
)
# TID_BLOCK: block doc comment (/** */).
TID_BLOCK = (
    "/**\tBlock doc with a tab. */\n"
    "pub fn tid_e() -> u64 {\n    5\n}\n"
)
# TID_CODEBLOCK: a tab inside a fenced code block — clippy is expected to
# skip code blocks (the suggestion must not rewrite sample code), measured
# here rather than assumed.
TID_CODEBLOCK = (
    "/// Example:\n"
    "/// ```\n"
    "///\tlet x = 1;\n"
    "/// ```\n"
    "pub fn tid_f() -> u64 {\n    6\n}\n"
)
# TID_REGULAR: a tab in a REGULAR comment (//) — must stay silent.
TID_REGULAR = (
    "//\tA regular comment with a tab.\n"
    "pub fn tid_g() -> u64 {\n    7\n}\n"
)

# -- map_clone ---------------------------------------------------------------
# `iter().map(|x| x.clone())` -> `.cloned()`; the deref variant `|x| *x` ->
# `.copied()`. The element-type knowledge rides the by-ref adapter chain
# (`.iter()` on a container param — the into_iter_on_ref provenance law).
MC_ITER_CLONE = (
    "pub fn mc_a(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x| x.clone()).collect()\n}\n"
)
MC_DEREF_COPY = (
    "pub fn mc_b(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x| *x).collect()\n}\n"
)
MC_KEYS_CLONE = (
    "pub fn mc_c(m: &std::collections::BTreeMap<u8, u8>) -> Vec<u8> {\n    m.values().map(|x| x.clone()).collect()\n}\n"
)
# MC_INTO_ITER: map over an OWNED iterator — cloned() would not exist;
# expected SILENT (the type-blind guard the proposer must mirror).
MC_INTO_ITER = (
    "pub fn mc_d(v: Vec<u8>) -> Vec<u8> {\n    v.into_iter().map(|x| x.clone()).collect()\n}\n"
)
# MC_NESTED: clone inside a larger expression — not the identity closure;
# expected SILENT.
MC_NESTED = (
    "pub fn mc_e(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x| (x.clone(), 1).0).collect()\n}\n"
)
# MC_ARBITRARY: a different method in the closure body — expected SILENT.
MC_ARBITRARY = (
    "pub fn mc_f(v: &Vec<String>) -> Vec<usize> {\n    v.iter().map(|x| x.len()).collect()\n}\n"
)

# name -> (source, {lint: min_count}); every expectation must hold
FIXTURES = {
    "tid_plain": (TID_PLAIN, {"tabs_in_doc_comments": 1}),
    "tid_mid": (TID_MID, {"tabs_in_doc_comments": 1}),
    "tid_multi": (TID_MULTI, {"tabs_in_doc_comments": 1}),
    "tid_inner": (TID_INNER, {"tabs_in_doc_comments": 1}),
    "tid_block": (TID_BLOCK, {"tabs_in_doc_comments": 1}),
    "tid_codeblock": (TID_CODEBLOCK, {"tabs_in_doc_comments": 1}),
    "mc_iter_clone": (MC_ITER_CLONE, {"map_clone": 1}),
    "mc_deref_copy": (MC_DEREF_COPY, {"map_clone": 1}),
    "mc_keys_clone": (MC_KEYS_CLONE, {"map_clone": 1}),
}

# name -> (source, lint) — the lint must NOT fire on the shape (measured
# upstream silences the proposer's guards mirror)
SILENT_SHAPES = {
    "tid_regular": (TID_REGULAR, "tabs_in_doc_comments"),
    "mc_into_iter": (MC_INTO_ITER, "map_clone"),
    "mc_nested": (MC_NESTED, "map_clone"),
    "mc_arbitrary": (MC_ARBITRARY, "map_clone"),
}

# NOTE: mc silent shapes are MEASURED GUESSES (upstream may fire on
# into_iter chains or nested clones) — the probe pins them either way; if
# a guess here is wrong the row moves to FIXTURES with its measured count
# before any proposer is authored (re-adjudicate, never assume).
#
# Rejected en route (the probe-first law's negative results, recorded for
# the batch note): useless_borrows_in_formatting — MEASURED INERT in
# clippy 0.1.98 across ~20 shapes INCLUDING its own doc example (let s:
# &str; println!("{}", &s)); int_plus_one — overflow divergence at the
# extremes (y+1 wraps/pansics at MAX while x > y does not — the
# manual_div_ceil REJECT class).

LIB_HEAD = "// plan163 probe fixture (generated - do not edit)\n"

SUGGESTION_LINTS = (
    "tabs_in_doc_comments",
    "map_clone",
    "uninlined_format_args",  # the format-args co-fire watch
    "iter_count",
    "redundant_closure",  # the near-miss co-fire (Batch-169 lesson)
)


def run_clippy(lib_rs: Path, target: Path) -> tuple[dict[str, list[dict]], list[dict]]:
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(target)
    proc = subprocess.run(
        ["cargo", "clippy", "--message-format=json", "--quiet"],
        cwd=lib_rs.parent.parent,
        env=env,
        capture_output=True,
        text=True,
        errors="replace", encoding="utf-8"
    )
    found: dict[str, list[dict]] = {}
    suggestions: list[dict] = []
    for line in proc.stdout.splitlines():
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        d = msg["message"]
        code = (d.get("code") or {}).get("code", "")
        if not code:
            continue
        lint = code.removeprefix("clippy::") if code.startswith("clippy::") else code
        found.setdefault(lint, []).append(d)
        if lint in SUGGESTION_LINTS:
            for child in d.get("children", []):
                for sp in child.get("spans", []):
                    if sp.get("suggested_replacement") is not None:
                        suggestions.append(
                            {
                                "lint": lint,
                                "replacement": sp["suggested_replacement"],
                                "line": sp.get("line_start"),
                            }
                        )
    if proc.returncode != 0:
        for lint, msgs in found.items():
            for d in msgs:
                if d.get("level") == "error":
                    loc = (d.get("spans") or [{}])[0]
                    print(f"[probe ERROR] {loc.get('file_name')}:{loc.get('line_start')} {d.get('message')}")
    return found, suggestions


def main() -> int:
    crate = Path(tempfile.mkdtemp(prefix="plan163_probe_"))
    target = Path(tempfile.mkdtemp(prefix="plan163_probe_t_"))
    ok = True
    try:
        (crate / "src").mkdir()
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "plan163probe"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n',
            encoding="utf-8",
        )
        lib = crate / "src" / "lib.rs"

        all_shapes: dict[str, str] = {}
        for name, (frag, _) in FIXTURES.items():
            all_shapes[name] = frag
        for name, (frag, _l) in SILENT_SHAPES.items():
            all_shapes[name] = frag

        print("-- firings (one clippy run per fixture; exact attribution) --")
        for name, frag in all_shapes.items():
            lib.write_text(LIB_HEAD + frag, encoding="utf-8")
            found, suggestions = run_clippy(lib, target)
            lints = sorted(found)
            print(f"  {name}: {lints}")
            for s in suggestions:
                rep = s["replacement"].replace("\n", "\\n").replace("\t", "\\t")
                print(f"      [{s['lint']}] L{s['line']} -> {rep!r}")
            if name in FIXTURES:
                for lint, want in FIXTURES[name][1].items():
                    got = len(found.get(lint, []))
                    if got < want:
                        ok = False
                        print(f"      DRIFT: expected {lint} >= {want}, got {got}")
            else:
                lint = SILENT_SHAPES[name][1]
                got = len(found.get(lint, []))
                if got != 0:
                    ok = False
                    print(f"      DRIFT: expected {lint} SILENT, got {got}")

        print("probe verdict:", "PASS" if ok else "FAIL")
        return 0 if ok else 1
    finally:
        shutil.rmtree(crate, ignore_errors=True)
        shutil.rmtree(target, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
