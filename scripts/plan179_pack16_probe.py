#!/usr/bin/env python3
"""Plan 179 probe — the Batch 184 pack XVI candidates (2026-09-28).

The staged leads from Plan 169's pack XV adjudication land here:
LEAD-1 `ptr_offset_with_cast` (the standing IOU — "lands in a later pack";
clippy's own two-part MachineApplicable splice measured) and LEAD-2
`needless_borrowed_reference` (the plan158 RETIRE overturned on plan160's
measured firing shapes). This probe measures the PROPOSER's guard matrix —
the shapes the guards admit, the shapes they refuse, and the upstream
suggestion text (replacement + applicability + span) each fix must match
or diverge-from-measured.

One clippy run PER FIXTURE (each fixture alone in the probe crate) —
attribution is exact by construction; no line arithmetic. UTF-8 writes,
ASCII output — the plan163 envelope.

NEW this pack (beyond plan169's rows):
  - suggestion APPLICABILITY is dumped (MachineApplicable vs
    MaybeIncorrect — the nbr deletion's usage-site hazard lives here);
  - a FIXED-FORMS section: every hand-written fixed form is compiled and
    must produce ZERO error-level diagnostics AND stay silent on its lint
    (the batch-183 latent-E0308 class — the pipeline never compiles
    fixed_code, so the probe closes it at probe time);
  - `powc_custom_offset`: does upstream TYPECK the receiver? If clippy
    fires on a custom `offset` method our `.as_ptr()`/`*T` provenance
    guard is a documented narrower-than-clippy divergence; if silent,
    the guard is pure parity;
  - the negative arm `-(i as isize)` (sub?) and `wrapping_offset`
    (wrapping_add?) — the method-mapping table measured, not guessed;
  - `&ref mut` for nbr (the mut-variant staged or admitted on evidence).

Confirmed present + warn-by-default in 0.1.98 (plan169 measured both via
`-W help`; re-confirmed this run through the fixtures themselves):
  ptr-offset-with-cast        "unneeded pointer offset cast"
  needless-borrowed-reference "destructuring a reference and borrowing the
                               inner value"

Run from this repo's root:

    py scripts/plan179_pack16_probe.py

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

# -- ptr_offset_with_cast (LEAD-1) -------------------------------------------
# `p.offset(i as isize)` -> `p.add(i)`. unsafe lives inside a safe fn (the
# bench_guard harness class: no undocumented_unsafe_blocks by default).
POWC_PLAIN = (
    "pub fn powc_a(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.offset(i as isize) }\n"
    "}\n"
)
# POWC_PARAM_PTR: the receiver IS a raw-pointer fn param — the proposer's
# second provenance arm (the scalar_param_heads_over analog for *T).
POWC_PARAM_PTR = (
    "pub fn powc_p(p: *const u8, i: usize) -> u8 {\n"
    "    unsafe { *p.offset(i as isize) }\n"
    "}\n"
)
# POWC_WRAPPING: the wrapping arm — method mapping measured, not guessed.
POWC_WRAPPING = (
    "pub fn powc_w(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.wrapping_offset(i as isize) }\n"
    "}\n"
)
# POWC_NEG: the negative arm — does upstream suggest `sub`?
POWC_NEG = (
    "pub fn powc_n(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.offset(-(i as isize)) }\n"
    "}\n"
)
# POWC_U64: a u64 operand cast to isize — FIRES upstream; our v1 guard
# REFUSES (the `.add` arg must be usize; keeping the cast moved is a
# bigger edit). FIXTURES firing + divergence recorded.
POWC_U64 = (
    "pub fn powc_u(b: &[u8], i: u64) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.offset(i as isize) }\n"
    "}\n"
)

# POWC_CUSTOM_OFFSET: does upstream typeck the receiver? A custom type's
# own `offset(isize)` with a cast-to-isize arg — if clippy FIRES here the
# `.add` suggestion would not compile (no such method), and our pointer-
# provenance guard is the documented narrower-than-clippy divergence; if
# SILENT the guard is pure parity.
POWC_CUSTOM_OFFSET = (
    "pub struct Pz { pub v: u8 }\n"
    "impl Pz {\n"
    "    pub fn offset(self, by: isize) -> Pz {\n"
    "        let _ = by;\n"
    "        self\n"
    "    }\n"
    "}\n"
    "pub fn powc_z(p: Pz, i: usize) -> u8 {\n"
    "    p.offset(i as isize).v\n"
    "}\n"
)

# silences
POWC_ADD = (
    "pub fn powc_b(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.add(i) }\n"
    "}\n"
)
POWC_ISIZE = (
    "pub fn powc_c(b: &[u8], i: isize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.offset(i) }\n"
    "}\n"
)
# POWC_ISIZE_CAST: identity cast of an already-isize operand — MEASURED
# GUESS upstream stays silent (the cast is a no-op it may still flag).
POWC_ISIZE_CAST = (
    "pub fn powc_i(b: &[u8], i: isize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.offset(i as isize) }\n"
    "}\n"
)
POWC_WRAPPING_ADD = (
    "pub fn powc_wa(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.wrapping_add(i) }\n"
    "}\n"
)
POWC_SUB = (
    "pub fn powc_s(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.sub(i) }\n"
    "}\n"
)
POWC_OFFSET_FROM = (
    "pub fn powc_f(b: &[u8]) -> usize {\n"
    "    let a = b.as_ptr();\n"
    "    let c = unsafe { b.as_ptr().add(3) };\n"
    "    unsafe { a.offset_from(c) as usize }\n"
    "}\n"
)

# -- needless_borrowed_reference (LEAD-2) -------------------------------------
# The plan169-measured let shape (FIRES, one deletion span) and the match
# shape (FIRES + CO-FIRES match_ref_pats — our refusal arm). NEW: the mut
# variant, and the fixed-form compile check (dropping `&` moves the
# binding one reference level — the usage site must tolerate it, so the
# corpus atom's usage is an auto-deref method call).
NBR_LET = (
    "pub fn nbr_b(a: u64) -> String {\n"
    "    let &ref y = &a;\n"
    "    y.to_string()\n"
    "}\n"
)
NBR_LET_MUT = (
    "pub fn nbr_m(a: u64) -> u64 {\n"
    "    let mut b = a;\n"
    "    {\n"
    "        let &ref mut y = &mut b;\n"
    "        *y += 1;\n"
    "    }\n"
    "    b\n"
    "}\n"
)
NBR_MATCH = (
    "pub fn nbr_a(r: &Option<u64>) -> u64 {\n"
    "    match r {\n"
    "        &Some(ref x) => *x,\n"
    "        &None => 0,\n"
    "    }\n"
    "}\n"
)
NBR_REF_BIND_OWNED = (
    "pub fn nbr_c(o: Option<u64>) -> u64 {\n"
    "    match o {\n"
    "        Some(ref v) => *v,\n"
    "        None => 0,\n"
    "    }\n"
    "}\n"
)
NBR_ERGONOMIC = (
    "pub fn nbr_d(r: &Option<u64>) -> u64 {\n"
    "    match r {\n"
    "        Some(v) => *v,\n"
    "        None => 0,\n"
    "    }\n"
    "}\n"
)

# name -> (source, {lint: min_count}); every expectation must hold
FIXTURES = {
    "powc_plain": (POWC_PLAIN, {"ptr_offset_with_cast": 1}),
    "powc_param_ptr": (POWC_PARAM_PTR, {"ptr_offset_with_cast": 1}),
    "powc_wrapping": (POWC_WRAPPING, {"ptr_offset_with_cast": 1}),
    "powc_neg": (POWC_NEG, {"ptr_offset_with_cast": 1}),
    "powc_u64": (POWC_U64, {"ptr_offset_with_cast": 1}),
    "nbr_let": (NBR_LET, {"needless_borrowed_reference": 1}),
    "nbr_match": (NBR_MATCH, {"needless_borrowed_reference": 1}),
}

# name -> (source, lint) — the lint must NOT fire on the shape (measured
# upstream silences the proposer's guards mirror)
SILENT_SHAPES = {
    "powc_add": (POWC_ADD, "ptr_offset_with_cast"),
    "powc_isize": (POWC_ISIZE, "ptr_offset_with_cast"),
    "powc_isize_cast": (POWC_ISIZE_CAST, "ptr_offset_with_cast"),
    "powc_wrapping_add": (POWC_WRAPPING_ADD, "ptr_offset_with_cast"),
    "powc_sub": (POWC_SUB, "ptr_offset_with_cast"),
    "powc_offset_from": (POWC_OFFSET_FROM, "ptr_offset_with_cast"),
    "nbr_ref_bind_owned": (NBR_REF_BIND_OWNED, "needless_borrowed_reference"),
    "nbr_ergonomic": (NBR_ERGONOMIC, "needless_borrowed_reference"),
}

# MEASURED GUESSES, kept OUT of the verdict: the probe REPORTS these rows
# but never fails on them — their measured value re-adjudicates the guard
# set (the plan169 nbr_let lesson: a silent-guess that fires moves to
# FIXTURES, never silently passes).
GUESS_SHAPES = {
    "powc_custom_offset": (POWC_CUSTOM_OFFSET, "ptr_offset_with_cast"),
    "nbr_let_mut": (NBR_LET_MUT, "needless_borrowed_reference"),
}

# The hand-written FIXED forms: each must compile with ZERO error-level
# diagnostics AND stay silent on its lint (the batch-183 latent-E0308
# class closed at probe time — the pipeline never compiles fixed_code).
FIXED_FORMS = {
    "powc_add_fixed": (POWC_ADD, "ptr_offset_with_cast"),
    "powc_sub_fixed": (POWC_SUB, "ptr_offset_with_cast"),
    "powc_wrapping_add_fixed": (POWC_WRAPPING_ADD, "ptr_offset_with_cast"),
    "nbr_let_fixed": (NBR_LET.replace("let &ref y", "let ref y"), "needless_borrowed_reference"),
}

WATCH_LINTS = (
    # co-fire / near-miss watches (the Batch-169 near-miss-name lesson)
    "match_ref_pats",
    "needless_borrow",
    "needless_borrows_for_generic_args",
    "unnecessary_cast",
    "cast_abs_to_unsigned",
    "erasing_op",
    "identity_op",
    "unused_variables",
)

LIB_HEAD = "// plan179 probe fixture (generated - do not edit)\n"


def run_clippy(lib_rs: Path, target: Path) -> tuple[dict[str, list[dict]], list[dict]]:
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(target)
    proc = subprocess.run(
        ["cargo", "clippy", "--message-format=json", "--quiet"],
        cwd=lib_rs.parent.parent,
        env=env,
        capture_output=True,
        text=True,
        encoding="utf-8",
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
        for child in d.get("children", []):
            for sp in child.get("spans", []):
                if sp.get("suggested_replacement") is not None:
                    suggestions.append(
                        {
                            "lint": lint,
                            "replacement": sp["suggested_replacement"],
                            "line": sp.get("line_start"),
                            "applicability": sp.get("suggestion_applicability"),
                        }
                    )
    return found, suggestions


def main() -> int:
    crate = Path(tempfile.mkdtemp(prefix="plan179_probe_"))
    target = Path(tempfile.mkdtemp(prefix="plan179_probe_t_"))
    ok = True
    try:
        (crate / "src").mkdir()
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "plan179probe"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n',
            encoding="utf-8",
        )
        lib = crate / "src" / "lib.rs"

        all_shapes: dict[str, str] = {}
        for name, (frag, _) in FIXTURES.items():
            all_shapes[name] = frag
        for name, (frag, _l) in SILENT_SHAPES.items():
            all_shapes[name] = frag
        for name, (frag, _l) in GUESS_SHAPES.items():
            all_shapes[name] = frag

        print("-- firings (one clippy run per fixture; exact attribution) --")
        for name, frag in all_shapes.items():
            lib.write_text(LIB_HEAD + frag, encoding="utf-8")
            found, suggestions = run_clippy(lib, target)
            lints = sorted(found)
            print(f"  {name}: {lints}")
            for s in suggestions:
                rep = s["replacement"].replace("\n", "\\n").replace("\t", "\\t")
                print(f"      [{s['lint']}] L{s['line']} -> {rep!r} ({s['applicability']})")
            if name in FIXTURES:
                for lint, want in FIXTURES[name][1].items():
                    got = len(found.get(lint, []))
                    if got < want:
                        ok = False
                        print(f"      DRIFT: expected {lint} >= {want}, got {got}")
            elif name in SILENT_SHAPES:
                lint = SILENT_SHAPES[name][1]
                got = len(found.get(lint, []))
                if got != 0:
                    ok = False
                    print(f"      DRIFT: expected {lint} SILENT, got {got}")
            else:
                lint = GUESS_SHAPES[name][1]
                got = len(found.get(lint, []))
                state = "FIRES" if got else "SILENT"
                print(f"      [guess adjudication] {lint} {state} (count {got}) — guards re-derive on this")

        print("-- fixed forms (compile + silence) --")
        for name, (frag, lint) in FIXED_FORMS.items():
            lib.write_text(LIB_HEAD + frag, encoding="utf-8")
            found, _s = run_clippy(lib, target)
            errs = [
                d
                for msgs in found.values()
                for d in msgs
                if d.get("level") == "error"
            ]
            lint_hits = len(found.get(lint, []))
            status = "OK" if not errs and lint_hits == 0 else "FAIL"
            if status == "FAIL":
                ok = False
            print(f"  {name}: {status} (errors={len(errs)}, {lint}={lint_hits})")
            for d in errs:
                loc = (d.get("spans") or [{}])[0]
                print(f"      [error] L{loc.get('line_start')} {d.get('message')}")

        print("probe verdict:", "PASS" if ok else "FAIL")
        return 0 if ok else 1
    finally:
        shutil.rmtree(crate, ignore_errors=True)
        shutil.rmtree(target, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
