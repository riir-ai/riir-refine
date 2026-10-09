#!/usr/bin/env python3
"""Plan 169 probe — the Batch 175 pack XV candidates (2026-09-24).

The coverage-report re-arm's derivation (clippy 0.1.98, 822 lints /
94 covered / 728 uncovered, 415 default-firing) adjudicated against the
Research-191 skip list + corpus dedup down to SIX fresh leads; this probe
measures the envelope the proposers need — firing shapes, upstream
silences, co-fire sets (the Batch-169 needless_borrow lesson), and
clippy's OWN suggestion text (the fix shape the healer must match or
diverge-from-measured).

One clippy run PER FIXTURE (each fixture alone in the probe crate) —
attribution is exact by construction; no line arithmetic.

All six lints confirmed present + warn-by-default in 0.1.98 via
`cargo clippy -- -W help` (dash-spelled there):
  len-without-is-empty    "traits or impls with a public `len` method but no
                           corresponding `is_empty` method"
  cast-abs-to-unsigned    "casting the result of `abs()` to an unsigned
                           integer can panic"
  ptr-offset-with-cast    "unneeded pointer offset cast"
  derivable-impls         "manual implementation of the `Default` trait which
                           is equal to a derive"
  multiple-bound-locations "defining generic bounds in multiple locations"
  needless-borrowed-reference "destructuring a reference and borrowing the
                           inner value" (the plan160 mrf-binding co-fire
                           evidence, corpus.rs:935-939)

Run from this repo's root:

    py scripts/plan169_pack15_probe.py

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

# -- len_without_is_empty (LEAD-1) -------------------------------------------
# The additive-insertion shape (the landed new_without_default precedent):
# a pub `len` with no `is_empty` sibling. Description says "traits or
# impls" — both are probed. Guards to mirror: private len must stay
# silent; the is_empty sibling presence must stay silent.
LWIE_PLAIN = (
    "pub struct LwA { items: Vec<u8> }\n"
    "impl LwA {\n"
    "    pub fn len(&self) -> usize {\n"
    "        self.items.len()\n"
    "    }\n"
    "}\n"
)
LWIE_TRAIT = (
    "pub trait LwT {\n"
    "    fn len(&self) -> usize;\n"
    "}\n"
)
# LWIE_WITH_EMPTY: the both-present shape — expected SILENT. The
# `self.len() == 0` body is a len_zero/comparison_to_empty co-fire WATCH
# (upstream len_zero may refuse inside is_empty itself — measured here,
# the suggestion text informs the LWIE proposer's own is_empty body).
LWIE_WITH_EMPTY = (
    "pub struct LwB { items: Vec<u8> }\n"
    "impl LwB {\n"
    "    pub fn len(&self) -> usize {\n"
    "        self.items.len()\n"
    "    }\n"
    "    pub fn is_empty(&self) -> bool {\n"
    "        self.len() == 0\n"
    "    }\n"
    "}\n"
)
# LWIE_PRIVATE: non-pub len — expected SILENT (the description's "public
# `len` method" term; the proposer's provenance guard mirrors it).
LWIE_PRIVATE = (
    "pub struct LwC { items: Vec<u8> }\n"
    "impl LwC {\n"
    "    fn len(&self) -> usize {\n"
    "        self.items.len()\n"
    "    }\n"
    "    pub fn touch(&self) -> usize {\n"
    "        self.len()\n"
    "    }\n"
    "}\n"
)

# -- cast_abs_to_unsigned (LEAD-2) -------------------------------------------
# `x.abs() as uN` -> `x.unsigned_abs()`. The LINT'S OWN MOTIVATION is the
# divergence note (description: "can panic"): iN::MIN.abs() panics in
# debug before the cast; unsigned_abs() returns 2^(N-1). Release-mode
# values are IDENTICAL for every input (wrapping abs of MIN is MIN, and
# MIN as uN == 2^(N-1) == unsigned_abs) — the fix is
# release-identical + debug-strictly-better, clippy's own reasoning.
CAU_I64 = "pub fn cau_a(x: i64) -> u64 {\n    x.abs() as u64\n}\n"
CAU_I32_LET = (
    "pub fn cau_b(x: i32) -> u32 {\n"
    "    let y: u32 = x.abs() as u32;\n"
    "    y\n"
    "}\n"
)
# CAU_UNSIGNED_ABS: the fixed form — expected SILENT.
CAU_UNSIGNED_ABS = "pub fn cau_c(x: i64) -> u64 {\n    x.unsigned_abs()\n}\n"
# CAU_NO_CAST: abs kept in the signed domain — expected SILENT (no cast,
# not this lint's territory).
CAU_NO_CAST = "pub fn cau_d(x: i64) -> i64 {\n    x.abs()\n}\n"
# CAU_UNSIGNED_IN: RETIRED after the first probe run — `u32` has NO
# `.abs()` method in std (E0599), so the abs operand is signed by
# construction; the shape cannot exist. Replaced by CAU_NARROW below.
# CAU_NARROW: signed->unsigned NARROWING cast of abs — MEASURED GUESS it
# still fires (the panic-on-MIN motivation holds at any width); the
# suggestion text decides the fix shape (unsigned_abs + kept cast?).
CAU_NARROW = "pub fn cau_n(x: i64) -> u8 {\n    x.abs() as u8\n}\n"

# -- ptr_offset_with_cast (STAGED ALTERNATE) ---------------------------------
# `p.offset(i as isize)` with i: usize -> `p.add(i)` (method rename + cast
# removal — the spliced-edit class). unsafe lives inside a safe fn (the
# bench_guard harness class: no undocumented_unsafe_blocks by default).
POWC_PLAIN = (
    "pub fn powc_a(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.offset(i as isize) }\n"
    "}\n"
)
# POWC_ADD: the fixed form — expected SILENT.
POWC_ADD = (
    "pub fn powc_b(b: &[u8], i: usize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.add(i) }\n"
    "}\n"
)
# POWC_ISIZE: offset on an already-isize operand — expected SILENT (no
# cast to remove).
POWC_ISIZE = (
    "pub fn powc_c(b: &[u8], i: isize) -> u8 {\n"
    "    let p = b.as_ptr();\n"
    "    unsafe { *p.offset(i) }\n"
    "}\n"
)

# -- derivable_impls (STAGED ALTERNATE) --------------------------------------
# manual Default equal to a derive -> `#[derive(Default)]`. The edit shape
# is the biggest of the pack (attribute insertion + block deletion).
DI_PLAIN = (
    "pub struct DiA { pub a: u32, pub b: String }\n"
    "impl Default for DiA {\n"
    "    fn default() -> Self {\n"
    "        DiA { a: 0, b: String::new() }\n"
    "    }\n"
    "}\n"
)
# DI_NONZERO: a field init the derive CANNOT produce — MEASURED GUESS that
# upstream stays silent here ("which is equal to a derive"); if it drifts
# the row moves to FIXTURES and the proposer's guard set is re-derived.
DI_NONZERO = (
    "pub struct DiB { pub a: u32 }\n"
    "impl Default for DiB {\n"
    "    fn default() -> Self {\n"
    "        DiB { a: 1 }\n"
    "    }\n"
    "}\n"
)

# -- multiple_bound_locations (STAGED ALTERNATE) -----------------------------
# The same bound inline AND in the where clause -> remove one. The guard
# matrix: single-location spellings must stay silent both ways.
MBL_PLAIN = (
    "pub fn mbl_a<T: Default>(t: T) -> T\n"
    "where\n"
    "    T: Default,\n"
    "{\n"
    "    t\n"
    "}\n"
)
MBL_WHERE_ONLY = (
    "pub fn mbl_b<T>(t: T) -> T\n"
    "where\n"
    "    T: Default,\n"
    "{\n"
    "    t\n"
    "}\n"
)
MBL_INLINE_ONLY = "pub fn mbl_c<T: Default>(t: T) -> T {\n    t\n}\n"

# -- needless_borrowed_reference (STAGED — re-adjudicated at probe) ----------
# The plan160-measured firing shape (corpus.rs:935-939: the mrf binding
# fixture co-fires THIS lint): a pattern that destructures a reference
# (`&`-pattern) AND binds the inner value by `ref`. The B164 skip premise
# ("FIXABLE-only miner loop") no longer applies; the probe re-measures.
NBR_MATCH = (
    "pub fn nbr_a(r: &Option<u64>) -> u64 {\n"
    "    match r {\n"
    "        &Some(ref x) => *x,\n"
    "        &None => 0,\n"
    "    }\n"
    "}\n"
)
# NBR_LET: the `&ref` let-binding spelling — MEASURED GUESS it fires
# ("destructuring a reference and borrowing the inner value" verbatim).
NBR_LET = (
    "pub fn nbr_b() -> u64 {\n"
    "    let a: u64 = 5;\n"
    "    let &ref y = &a;\n"
    "    *y\n"
    "}\n"
)
# NBR_REF_BIND_OWNED: `ref` binding WITHOUT `&`-destructure (owned
# scrutinee) — expected SILENT (no reference being destructured).
NBR_REF_BIND_OWNED = (
    "pub fn nbr_c(o: Option<u64>) -> u64 {\n"
    "    match o {\n"
    "        Some(ref v) => *v,\n"
    "        None => 0,\n"
    "    }\n"
    "}\n"
)
# NBR_ERGONOMIC: match ergonomics (no `&`-pattern, no `ref`) — expected
# SILENT.
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
    "lwie_plain": (LWIE_PLAIN, {"len_without_is_empty": 1}),
    "lwie_trait": (LWIE_TRAIT, {"len_without_is_empty": 1}),
    "cau_i64": (CAU_I64, {"cast_abs_to_unsigned": 1}),
    "cau_i32_let": (CAU_I32_LET, {"cast_abs_to_unsigned": 1}),
    "cau_narrow": (CAU_NARROW, {"cast_abs_to_unsigned": 1}),
    "powc_plain": (POWC_PLAIN, {"ptr_offset_with_cast": 1}),
    "di_plain": (DI_PLAIN, {"derivable_impls": 1}),
    "mbl_plain": (MBL_PLAIN, {"multiple_bound_locations": 1}),
    "nbr_match": (NBR_MATCH, {"needless_borrowed_reference": 1}),
    # nbr_let moved SILENT->FIXTURES by the first probe run (2026-09-24):
    # `let &ref y = &a;` FIRES (count 1, deletion suggestion) — the lint
    # covers let-bindings too, not just match patterns. The re-adjudication
    # the probe-first law prescribes, recorded inline.
    "nbr_let": (NBR_LET, {"needless_borrowed_reference": 1}),
}

# name -> (source, lint) — the lint must NOT fire on the shape (measured
# upstream silences the proposer's guards mirror)
SILENT_SHAPES = {
    "lwie_with_empty": (LWIE_WITH_EMPTY, "len_without_is_empty"),
    "lwie_private": (LWIE_PRIVATE, "len_without_is_empty"),
    "cau_unsigned_abs": (CAU_UNSIGNED_ABS, "cast_abs_to_unsigned"),
    "cau_no_cast": (CAU_NO_CAST, "cast_abs_to_unsigned"),
    "powc_add": (POWC_ADD, "ptr_offset_with_cast"),
    "powc_isize": (POWC_ISIZE, "ptr_offset_with_cast"),
    "di_nonzero": (DI_NONZERO, "derivable_impls"),
    "mbl_where_only": (MBL_WHERE_ONLY, "multiple_bound_locations"),
    "mbl_inline_only": (MBL_INLINE_ONLY, "multiple_bound_locations"),
    "nbr_let": (NBR_LET, "needless_borrowed_reference"),
    "nbr_ref_bind_owned": (NBR_REF_BIND_OWNED, "needless_borrowed_reference"),
    "nbr_ergonomic": (NBR_ERGONOMIC, "needless_borrowed_reference"),
}

# NOTE: cau_narrow is a MEASURED GUESS in FIXTURES (narrowing target may
# stay upstream-silent); if it drifts the guard set is re-derived. nbr_let
# was a silent-set guess that FIRED on the first run — moved to FIXTURES
# with its measured count (re-adjudicate, never assume). The nbr_match
# expectation is NOT a guess: plan160 measured the co-fire (corpus.rs:
# 935-939 records it as the reason the mrf corpus atom is the no-binding
# spelling).
#
# Adjudicated OUT before this probe (prior sessions' negative results,
# recorded for the batch note): missing_*_doc (doc-authoring, not a
# bounded rewrite), result_unit_err / too_many_arguments (architectural),
# len_zero (DUPLICATE — comparison_to_empty Form 2 covers `.len() == 0`),
# manual_div_ceil (REJECTED — `(a+b-1)/b` overflows the intermediate at
# the extremes while `div_ceil` does not), useless_borrows_in_formatting
# (MEASURED INERT in 0.1.98 incl. its own doc example), double_ended_
# iterator_last (type-blind), int_plus_one (the manual_div_ceil overflow
# class, Batch-171 probe).

LIB_HEAD = "// plan169 probe fixture (generated - do not edit)\n"

SUGGESTION_LINTS = (
    # the six leads
    "len_without_is_empty",
    "cast_abs_to_unsigned",
    "ptr_offset_with_cast",
    "derivable_impls",
    "multiple_bound_locations",
    "needless_borrowed_reference",
    # co-fire / near-miss watches (the Batch-169 near-miss-name lesson:
    # track the lint one character away TOO)
    "match_ref_pats",
    "toplevel_ref_arg",
    "unnecessary_cast",
    "needless_borrow",
    "len_zero",
    "comparison_to_empty",
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
    crate = Path(tempfile.mkdtemp(prefix="plan169_probe_"))
    target = Path(tempfile.mkdtemp(prefix="plan169_probe_t_"))
    ok = True
    try:
        (crate / "src").mkdir()
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "plan169probe"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n',
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
