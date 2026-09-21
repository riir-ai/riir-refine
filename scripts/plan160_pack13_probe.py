#!/usr/bin/env python3
"""Plan 160 probe — the Batch 169 pack XIII candidates (2026-09-21).

The two staged alternates from the Batch-168 probe (Plan 159), extended
here with the envelope this batch's proposers need: binding shapes, arm
mixes, receiver spellings, co-fire sets, and clippy's OWN suggestion text
(the fix shape the healer must match or diverge-from-measured).

One clippy run PER FIXTURE (each fixture alone in the probe crate) —
attribution is exact by construction; no line arithmetic.

Run from this repo's root:

    py scripts/plan160_pack13_probe.py

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

# -- match_ref_pats ---------------------------------------------------------
MRF_NO_BINDING = (
    "pub fn mrf_a(f: &Fruit) -> u64 {\n    match f {\n        &Fruit::Apple => 1,\n        &Fruit::Pear => 2,\n    }\n}"
)
MRF_BINDING = (
    "pub fn mrf_b(r: &Option<u64>) -> u64 {\n    match r {\n        &Some(ref x) => *x,\n        &None => 0,\n    }\n}"
)
MRF_WILDCARD_ARM = (
    "pub fn mrf_c(f: &Fruit) -> u64 {\n    match f {\n        &Fruit::Apple => 1,\n        _ => 2,\n    }\n}"
)
MRF_GUARD = (
    "pub fn mrf_d(f: &Fruit, k: u64) -> u64 {\n    match f {\n        &Fruit::Apple if k > 1 => 1,\n        &_ => 2,\n    }\n}"
)
MRF_OR_PAT = (
    "pub fn mrf_e(f: &Fruit) -> u64 {\n    match f {\n        &Fruit::Apple | &Fruit::Pear => 1,\n    }\n}"
)
MRF_MIXED = (
    "pub fn mrf_f(f: &Fruit) -> u64 {\n    match f {\n        &Fruit::Apple => 1,\n        Fruit::Pear => 2,\n    }\n}"
)

# -- into_iter_on_ref -------------------------------------------------------
IIR_COUNT = (
    "pub fn iir_a(v: Vec<u64>) -> usize {\n    (&v).into_iter().count()\n}"
)
IIR_BARE = (
    "pub fn iir_b(v: Vec<u64>) -> u64 {\n    let n = (&v).into_iter().next();\n    n.copied().unwrap_or(0)\n}"
)
IIR_MUT = (
    "pub fn iir_c(mut v: &mut Vec<u64>) -> usize {\n    (&mut v).into_iter().count()\n}"
)
IIR_FOR = (
    "pub fn iir_d(v: Vec<u64>) -> u64 {\n    let mut s = 0u64;\n    for x in (&v).into_iter() {\n        s += *x;\n    }\n    s\n}"
)
IIR_REF_IDENT = (
    "pub fn iir_e(r: &Vec<u64>) -> usize {\n    r.into_iter().count()\n}"
)

# name -> (source, {lint: min_count}); every expectation must hold
FIXTURES = {
    "mrf_no_binding": (MRF_NO_BINDING, {"match_ref_pats": 1}),
    "mrf_binding": (MRF_BINDING, {"match_ref_pats": 1}),
    "mrf_guard": (MRF_GUARD, {"match_ref_pats": 1}),
    "iir_count": (IIR_COUNT, {"into_iter_on_ref": 1}),
    "iir_bare": (IIR_BARE, {"into_iter_on_ref": 1}),
    "iir_mut": (IIR_MUT, {"into_iter_on_ref": 1}),
    "iir_for": (IIR_FOR, {"into_iter_on_ref": 1}),
    "iir_ref_ident": (IIR_REF_IDENT, {"into_iter_on_ref": 1}),
}

# name -> (source, lint) — the lint must NOT fire on the shape (measured
# upstream silences the proposer's guards mirror)
SILENT_SHAPES = {
    "mrf_mixed": (MRF_MIXED, "match_ref_pats"),
    "mrf_wildcard_arm": (MRF_WILDCARD_ARM, "match_ref_pats"),
    "mrf_or_pat": (MRF_OR_PAT, "match_ref_pats"),
}

SUPPORT = "pub enum Fruit { Apple, Pear }\n"
LIB_HEAD = "// plan160 probe fixture (generated - do not edit)\n"

SUGGESTION_LINTS = (
    "match_ref_pats",
    "into_iter_on_ref",
    "iter_count",
    "explicit_into_iter_loop",
    "explicit_iter_loop",
    "needless_borrowed_reference",
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
        errors="replace",
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
    crate = Path(tempfile.mkdtemp(prefix="plan160_probe_"))
    target = Path(tempfile.mkdtemp(prefix="plan160_probe_t_"))
    ok = True
    try:
        (crate / "src").mkdir()
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "plan160probe"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n',
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
            lib.write_text(LIB_HEAD + SUPPORT + frag + "\n", encoding="utf-8")
            found, suggestions = run_clippy(lib, target)
            lints = sorted(found)
            print(f"  {name}: {lints}")
            for s in suggestions:
                rep = s["replacement"].replace("\n", "\\n")
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
