#!/usr/bin/env python3
"""Plan 158 T1 — probe the Batch-167 leads (`unnecessary_cast` +
`needless_borrowed_reference` + `double_ended_iterator_last` [+ `clone_on_copy`
as the spare]) with REAL clippy: firing shapes, suggestion rendering, guard
boundaries, and the default-group pins.

Replicates the ClippyOracle invocation exactly (the plan155 pattern):
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case and the
rendered suggestions for the lead lints (the proposer design input). The
default-group arms run with NO -W flags: a hit there = default-level lint.

Lead provenance: the `--coverage-report` derivation of 2026-09-21 (fresh build
at develop d8a2c0ec, clippy 0.1.98): 822 derived / 86 covered / 736 uncovered /
423 default-firing / 116 with evidence. Both leads carry default-eco evidence
from the Bench 094 manifest (663-crate default-level histogram):
  unnecessary_cast              complexity warn  count 48  (top non-doc row)
  needless_borrowed_reference   complexity warn  count 27
  double_ended_iterator_last    perf       warn  count 17
  clone_on_copy                 complexity warn  count 16  (spare)

Precedent excludes applied here (recorded rejects / skips):
  missing_safety_doc (186, the top row) + missing_errors_doc / missing_panics_doc
    — doc-AUTHORING class, not a bounded mechanical fix (Research 191 skip list).
  match_ref_pats — probed in the plan152 round as `mrp`, SILENT on the natural
    shape; recorded reject, not re-probed.
  len_zero (20) — NOT real debt: covered by our `comparison_to_empty` corpus
    rule under a different name; the name-keyed derivation counts it uncovered.
    Derivation alias gap, report-only (goes in the plan).
  result_unit_err / too_many_arguments — API-shape (architectural), not bounded.
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
    # ── unnecessary_cast ──
    "uc-seed-annot": (
        "fn f(hits: &mut Vec<u8>) -> usize { hits.push(0); "
        "let x: usize = 7; let y = x as usize; y }"
    ),
    "uc-seed-param": (
        "fn f(hits: &mut Vec<u8>, p: u32) -> u32 { hits.push(0); p as u32 }"
    ),
    "uc-literal": (
        "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 5 as u64 }"
    ),
    "uc-literal-annot": (
        "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); "
        "let a: u64 = 5 as u64; a }"
    ),
    "uc-float": (
        "fn f(hits: &mut Vec<u8>) -> f64 { hits.push(0); "
        "let a: f64 = 1.5; a as f64 }"
    ),
    "uc-lossless-crossfire": (
        "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); "
        "let x: u8 = 7; x as u64 }"
    ),
    "uc-samewidth-difftype": (
        "fn f(hits: &mut Vec<u8>, x: u64) -> i64 { hits.push(0); x as i64 }"
    ),
    "uc-const": (
        "const A: u64 = 5u64 as u64;\n"
        "fn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); A }"
    ),
    "uc-expr-call": (
        "fn mk() -> usize { 3 }\n"
        "fn f(hits: &mut Vec<u8>) -> usize { hits.push(0); mk() as usize }"
    ),
    # ── needless_borrowed_reference (pattern-position borrows) ──
    "nbr-match-refparam": (
        "fn f(hits: &mut Vec<u8>, r: &u32) -> u32 { hits.push(0); "
        "match r { &x => x } }"
    ),
    "nbr-iflet-optref": (
        "fn f(hits: &mut Vec<u8>, o: Option<&u32>) -> u32 { hits.push(0); "
        "if let Some(&x) = o { x } else { 0 } }"
    ),
    "nbr-match-optref": (
        "fn f(hits: &mut Vec<u8>, o: Option<&u32>) -> u32 { hits.push(0); "
        "match o { Some(&x) => x, None => 0 } }"
    ),
    "nbr-whilelet-ref": (
        "fn f(hits: &mut Vec<u8>, o: &Option<u32>) -> u32 { hits.push(0); "
        "if let Some(&x) = o { x } else { 0 } }"
    ),
    "nbr-for-legit": (
        "fn f(hits: &mut Vec<u8>, v: &[u32]) -> u32 { hits.push(0); "
        "let mut s = 0u32; for &i in v { s += i } s }"
    ),
    "nbr-noncopy-pattern": (
        "fn f(hits: &mut Vec<u8>, o: Option<&String>) -> usize { hits.push(0); "
        "if let Some(&s) = o { s.len() } else { 0 } }"
    ),
    # ── double_ended_iterator_last ──
    "deil-iter-seed": (
        "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Option<u8> { hits.push(0); "
        "v.iter().last() }"
    ),
    "deil-intoiter": (
        "fn f(hits: &mut Vec<u8>, v: Vec<u8>) -> Option<u8> { hits.push(0); "
        "v.into_iter().last() }"
    ),
    "deil-rev": (
        "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Option<u8> { hits.push(0); "
        "v.iter().rev().last() }"
    ),
    "deil-btree": (
        "fn f(hits: &mut Vec<u8>, m: &std::collections::BTreeMap<u8, u8>) -> Option<&u8> "
        "{ hits.push(0); m.values().last() }"
    ),
    "deil-hashmap-neg": (
        "fn f(hits: &mut Vec<u8>, m: &std::collections::HashMap<u8, u8>) -> Option<&u8> "
        "{ hits.push(0); m.values().last() }"
    ),
    "deil-range-neg": (
        "fn f(hits: &mut Vec<u8>) -> Option<u32> { hits.push(0); (0..10).last() }"
    ),
    "deil-chars-neg": (
        "fn f(hits: &mut Vec<u8>, s: &str) -> Option<char> { hits.push(0); s.chars().last() }"
    ),
    "deil-fixed-iter": (
        "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Option<u8> { hits.push(0); "
        "v.iter().next_back() }"
    ),
    "deil-fixed-rev": (
        "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Option<u8> { hits.push(0); "
        "v.iter().rev().next_back() }"
    ),
    # ── clone_on_copy (the spare) ──
    "coc-seed": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x.clone() }"
    ),
    "coc-noncopy-neg": (
        "fn f(hits: &mut Vec<u8>, s: String) -> String { hits.push(0); s.clone() }"
    ),
    # negative control: known-clean shape under the full oracle.
    "neg-ctl": (
        "fn plain(x: u32, hits: &mut Vec<u32>) -> bool { hits.push(x); x > 0 }"
    ),
}

# Group-pin arms (no -W flags): a hit here = default-level lint — the corpus
# `lint_group` pins for the Batch-167 leads (uc/nbr/coc complexity-warn, deil
# perf-warn — all default-level).
GROUP_CASES = {
    "uc-grp-seed": (
        "fn f(hits: &mut Vec<u8>) -> usize { hits.push(0); "
        "let x: usize = 7; let y = x as usize; y }"
    ),
    "nbr-grp-seed": (
        "fn f(hits: &mut Vec<u8>, r: &u32) -> u32 { hits.push(0); "
        "match r { &x => x } }"
    ),
    "deil-grp-seed": (
        "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Option<u8> { hits.push(0); "
        "v.iter().last() }"
    ),
    "coc-grp-seed": (
        "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x.clone() }"
    ),
}


def probe_full(name: str, code: str, default_groups: bool = False):
    """Returns (sorted lint ids, the rendered suggestions for lead lints)."""
    root = Path(tempfile.mkdtemp(prefix=f"probe158_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe158\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
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


LEAD_KEYS = {"uc", "nbr", "deil", "coc"}


def main() -> int:
    for name, code in CASES.items():
        try:
            lints, suggestions = probe_full(name, code)
        except Exception as e:  # noqa: BLE001
            print(f"{name:28} PROBE-ERROR {e}")
            continue
        line = f"{name:28} {lints}"
        if suggestions and name.split("-")[0] in LEAD_KEYS:
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
