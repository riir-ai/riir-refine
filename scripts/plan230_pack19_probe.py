#!/usr/bin/env python3
"""Plan 230 probe — the Batch 228 pack XIX candidates (2026-10-13).

The coverage-report re-arm's derivation (clippy 0.1.98, 822 lints /
100 covered / 722 uncovered / 409 default-firing) adjudicated against the
standing skip ledger down to THREE fresh bounded leads; this probe measures
the envelope the proposers need — firing shapes, upstream silences,
co-fire sets (the Batch-169 needless_borrow lesson), and clippy's OWN
suggestion text (the fix shape the healer must match or diverge-from-measured).

One clippy run PER FIXTURE (each fixture alone in the probe crate) —
attribution is exact by construction; no line arithmetic.

Leads (all confirmed present in the installed 0.1.98 driver's -W help
derivation — the coverage report matched them against that surface):
  manual_repeat_n   "used `repeat().take()` when `repeat_n()` would fit"
  manual_ok_err     "manual implementation of `Option::ok`"
  manual_next_back  "using `.next()` on `std::iter::Rev`, which is more
                     idiomatically done via `.next_back()`"

Run from this repo's root:

    py scripts/plan230_pack19_probe.py

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

# -- manual_repeat_n (the LEAD) ------------------------------------------------
MRN_STD = (
    "pub fn mrn_std(vals: &[u32], n: usize) -> Vec<u32> {\n"
    "    std::iter::repeat(0u32).take(n).collect()\n"
    "}\n"
)
MRN_IMPORTED = (
    "pub fn mrn_imp(n: usize) -> Vec<&'static str> {\n"
    "    use std::iter::repeat;\n"
    "    repeat(\"x\").take(n).collect()\n"
    "}\n"
)
MRN_EXPR_CTX = (
    "pub fn mrn_ctx(seeds: &[u8], n: usize) -> usize {\n"
    "    std::iter::repeat(seeds).take(n).count()\n"
    "}\n"
)
MRN_NO_TAKE = (
    "pub fn mrn_no_take() -> Vec<u32> {\n"
    "    std::iter::repeat(7u32).take(3).collect()\n"
    "}\n"
)
MRN_TAKE_CONST = (
    "pub fn mrn_const(n: usize) -> Vec<u64> {\n"
    "    std::iter::repeat(1u64).take(n + 1).collect()\n"
    "}\n"
)
MRN_TAKE_FIRST = (
    "pub fn mrn_take_first(n: usize) -> Vec<char> {\n"
    "    std::iter::repeat('a').take(n).collect()\n"
    "}\n"
)

# -- manual_ok_err ------------------------------------------------------------
MOE_OK_MATCH = (
    "pub fn moe_ok(r: Result<u32, String>) -> Option<u32> {\n"
    "    match r {\n"
    "        Ok(v) => Some(v),\n"
    "        Err(_) => None,\n"
    "    }\n"
    "}\n"
)
MOE_ERR_MATCH = (
    "pub fn moe_err(r: Result<u32, String>) -> Option<String> {\n"
    "    match r {\n"
    "        Ok(_) => None,\n"
    "        Err(e) => Some(e),\n"
    "    }\n"
    "}\n"
)
MOE_NONIDENTITY = (
    "pub fn moe_nonid(r: Result<u32, String>) -> Option<String> {\n"
    "    match r {\n"
    "        Ok(v) => Some(v.to_string()),\n"
    "        Err(_) => None,\n"
    "    }\n"
    "}\n"
)
MOE_IF_LET = (
    "pub fn moe_iflet(r: Result<u32, String>) -> Option<u32> {\n"
    "    if let Ok(v) = r { Some(v) } else { None }\n"
    "}\n"
)

# -- manual_next_back ---------------------------------------------------------
MNB_REV_NEXT = (
    "pub fn mnb_rev_next(vals: &[u32]) -> Option<u32> {\n"
    "    vals.iter().rev().next().copied()\n"
    "}\n"
)
MNB_REV_NEXT_MUT = (
    "pub fn mnb_rev_mut(vals: &mut [u32]) -> Option<&mut u32> {\n"
    "    vals.iter_mut().rev().next()\n"
    "}\n"
)
MNB_PLAIN_NEXT = (
    "pub fn mnb_plain(vals: &[u32]) -> Option<u32> {\n"
    "    vals.iter().next().copied()\n"
    "}\n"
)
MNB_LAST = (
    "pub fn mnb_last(vals: &[u32]) -> Option<u32> {\n"
    "    vals.iter().last().copied()\n"
    "}\n"
)

MRN_LINT = "manual_repeat_n"
MOE_LINT = "manual_ok_err"
MNB_LINT = "manual_next_back"

# name -> (source, lint); every firing expectation must hold
# (re-adjudicated after the first probe run: manual_repeat_n fires on
# ARBITRARY take-args — `take(n + 1)` included, whole-chain splice,
# eq-by-construction — so the const-expr row is a FIRING shape, not
# silent; manual_ok_err fires on the IF-LET form too (-> r.ok(),
# MachineApplicable), recorded as POSTURE: v1 handles the canonical
# two-arm match, the if-let form is the documented
# narrower-than-clippy divergence, the P21 oracle lane covers it via
# clippy's own suggestion)
FIXTURES = {
    "mrn_std_path": (MRN_STD, MRN_LINT),
    "mrn_imported": (MRN_IMPORTED, MRN_LINT),
    "mrn_expr_ctx": (MRN_EXPR_CTX, MRN_LINT),
    "mrn_take_const_expr": (MRN_TAKE_CONST, MRN_LINT),
    "moe_ok_match": (MOE_OK_MATCH, MOE_LINT),
    "moe_err_match": (MOE_ERR_MATCH, MOE_LINT),
    "mnb_rev_next": (MNB_REV_NEXT, MNB_LINT),
    "mnb_rev_next_mut": (MNB_REV_NEXT_MUT, MNB_LINT),
}

# name -> source; the lint must NOT fire
SILENT_SHAPES = {
    "moe_nonidentity": MOE_NONIDENTITY,
    "mnb_plain_next": MNB_PLAIN_NEXT,
    "mnb_last": MNB_LAST,
}

# name -> source — upstream posture RECORDED, never gated (the proposer
# refuses these classes by construction; the row is the divergence table).
POSTURE_SHAPES = {
    "mrn_no_take_arg": MRN_NO_TAKE,
    "mrn_take_first_arg": MRN_TAKE_FIRST,
    "moe_if_let": MOE_IF_LET,
}

# The hand-written FIXED forms: compile with ZERO errors + stay silent.
FIXED_FORMS = {
    "mrn_std_fixed": (
        MRN_STD.replace(
            "std::iter::repeat(0u32).take(n)", "std::iter::repeat_n(0u32, n)"
        ),
        MRN_LINT,
    ),
    "mrn_imported_fixed": (
        MRN_IMPORTED.replace(
            "use std::iter::repeat;\n    repeat(\"x\").take(n)",
            "use std::iter::repeat_n;\n    repeat_n(\"x\", n)",
        ),
        MRN_LINT,
    ),
    "mrn_const_fixed": (
        MRN_TAKE_CONST.replace(
            "std::iter::repeat(1u64).take(n + 1)", "std::iter::repeat_n(1u64, n + 1)"
        ),
        MRN_LINT,
    ),
    "moe_ok_fixed": (MOE_OK_MATCH.replace("    match r {\n        Ok(v) => Some(v),\n        Err(_) => None,\n    }", "    r.ok()"), MOE_LINT),
    "moe_err_fixed": (MOE_ERR_MATCH.replace("    match r {\n        Ok(_) => None,\n        Err(e) => Some(e),\n    }", "    r.err()"), MOE_LINT),
    "mnb_fixed": (MNB_REV_NEXT.replace("vals.iter().rev().next()", "vals.iter().next_back()"), MNB_LINT),
}

WATCH_LINTS = (
    "let_and_return",
    "redundant_pattern_matching",
    "manual_map",
    "single_match",
    "needless_borrow",
    "manual_unwrap_or",
)

LIB_HEAD = "// plan230 probe fixture (generated - do not edit)\n"


def run_clippy(lib_rs: str, target: Path) -> tuple[dict[str, list[dict]], list[dict]]:
    """One clippy JSON run over a single-fixture crate. Returns
    ({lint: [suggestion-dicts]}, [error-diagnostics])."""
    crate = target / "probe_crate"
    src = crate / "src"
    src.mkdir(parents=True, exist_ok=True)
    (crate / "Cargo.toml").write_text(
        "[package]\n"
        'name = "probe_crate"\n'
        'version = "0.1.0"\n'
        'edition = "2021"\n'
        "[lib]\n"
        'path = "src/lib.rs"\n',
        encoding="utf-8",
    )
    lib = src / "lib.rs"
    lib.write_text(LIB_HEAD + lib_rs, encoding="utf-8")
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(target / "tgt")
    proc = subprocess.run(
        ["cargo", "clippy", "--message-format=json", "--quiet"],
        cwd=crate,
        env=env,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="backslashreplace",
        timeout=600,
    )
    found: dict[str, list[dict]] = {}
    errors: list[dict] = []
    for line in proc.stdout.splitlines():
        if not line.strip():
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        m = msg.get("message", {})
        if m.get("level") == "error":
            errors.append({"message": m.get("message", "")[:200]})
            continue
        code = (m.get("code") or {}).get("code") or ""
        if not code.startswith("clippy::"):
            continue
        lint = code.removeprefix("clippy::")
        spans = []
        for sp in m.get("spans", []):
            if sp.get("is_primary"):
                spans.append(
                    {
                        "line": sp.get("line_start"),
                        "text": (sp.get("text") or "")[:80],
                        "suggested": sp.get("suggested_replacement"),
                        "applicability": sp.get("suggestion_applicability"),
                    }
                )
        for child in m.get("children", []):
            for sp in child.get("spans", []):
                if sp.get("suggested_replacement") is not None:
                    raw_text = sp.get("text") or ""
                    if not isinstance(raw_text, str):
                        raw_text = " ".join(
                            str(line.get("text", "")) for line in raw_text
                        )
                    spans.append(
                        {
                            "line": sp.get("line_start"),
                            "text": "CHILD:" + raw_text[:60],
                            "suggested": sp.get("suggested_replacement"),
                            "applicability": sp.get("suggestion_applicability"),
                        }
                    )
        found.setdefault(lint, []).extend(spans)
    return found, errors


def main() -> int:
    print("plan230 probe - pack XIX leads: manual_repeat_n, manual_ok_err, manual_next_back")
    if shutil.which("cargo") is None:
        print("  cargo not found on PATH")
        return 2
    target = Path(tempfile.mkdtemp(prefix="plan230_probe_"))
    failures: list[str] = []
    try:
        # FIRING arms: the lint fires; dump the suggestion shape
        for name, (frag, lint) in FIXTURES.items():
            found, errors = run_clippy(frag, target)
            if errors:
                print(f"  [FIXTURE] FAIL {name}: fixture itself failed to compile:")
                for e in errors[:3]:
                    print(f"      {e['message']}")
                failures.append(name)
                continue
            got = len(found.get(lint, []))
            ok = got >= 1
            mark = "ok" if ok else "FAIL"
            print(f"  [FIXTURE] {mark} {name}: {lint} count={got} (want >=1)")
            if not ok:
                failures.append(name)
            for sp in found.get(lint, []):
                print(
                    f"      L{sp['line']} {sp['text']!r}"
                    f" -> {sp['suggested']!r} ({sp['applicability']})"
                )
            # co-fire census (the atom must fire EXACTLY its target)
            for wl in WATCH_LINTS:
                if wl in found:
                    print(f"      co-fire: {wl} x{len(found[wl])}")
        # SILENT arms
        for name, frag in SILENT_SHAPES.items():
            found, errors = run_clippy(frag, target)
            if errors:
                print(f"  [SILENT] FAIL {name}: fixture failed to compile:")
                for e in errors[:2]:
                    print(f"      {e['message']}")
                failures.append(name)
                continue
            got = len(found.get(MRN_LINT, [])) + len(found.get(MOE_LINT, [])) + len(found.get(MNB_LINT, []))
            mark = "ok" if got == 0 else "FAIL"
            print(f"  [SILENT] {mark} {name}: target-lint count={got} (want 0)")
            if got:
                failures.append(name)
        # POSTURE arms: recorded, never gated
        for name, frag in POSTURE_SHAPES.items():
            found, errors = run_clippy(frag, target)
            if errors:
                print(f"  [POSTURE] {name}: DOES NOT COMPILE ({len(errors)} errors)")
                for e in errors[:2]:
                    print(f"      {e['message']}")
                continue
            got = len(found.get(MRN_LINT, []))
            print(f"  [POSTURE] {name}: {MRN_LINT} count={got} (recorded)")
            for sp in found.get(MRN_LINT, []):
                print(
                    f"      L{sp['line']} {sp['text']!r}"
                    f" -> {sp['suggested']!r} ({sp['applicability']})"
                )
        # FIXED-FORMS: compile + silence
        print("  fixed-forms:")
        for name, (frag, lint) in FIXED_FORMS.items():
            found, errors = run_clippy(frag, target)
            if errors:
                print(f"  [FIXED] FAIL {name}: does not compile ({len(errors)} errors)")
                for e in errors[:3]:
                    print(f"      {e['message']}")
                failures.append(name)
                continue
            got = len(found.get(lint, []))
            if got:
                print(f"  [FIXED] FAIL {name}: compiles but {lint} still fires x{got}")
                failures.append(name)
            else:
                print(f"  [FIXED] ok {name}: compiles clean + silent")
    finally:
        shutil.rmtree(target, ignore_errors=True)
    print()
    if failures:
        print(f"VERDICT: FAIL - {len(failures)} drifted row(s): {sorted(set(failures))}")
        print("  re-adjudicate before authoring (the probe-first law).")
        return 1
    print("VERDICT: PASS - every expectation held; author per Plan 230 T2.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
