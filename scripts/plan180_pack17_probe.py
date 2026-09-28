#!/usr/bin/env python3
"""Plan 180 probe — the Batch 185 pack XVII candidates (2026-09-28).

LEAD-1 `multiple_bound_locations` (the Plan-169 staged fresh row —
"defining generic bounds in multiple locations") and LEAD-2
`missing_transmute_annotations` (the strongest unadjudicated
evidence-carrying row left in the fresh derivation's head — 13, default-eco;
everything else above it is skip-listed: doc-authoring, API-shape, alias-gap,
or a recorded REJECT). `derivable_impls` (16) stays DEFER (semantic-eq +
multi-part attr edit) — this probe re-records its suggestion shape only.

One clippy run PER FIXTURE (each fixture alone in the probe crate) —
attribution exact by construction. UTF-8 writes, ASCII output (the plan163
envelope; the plan160 cp874 lesson). NEW beyond plan179's rows:

  - MBL: which location the suggestion edits (inline vs where arm), the
    multi-bound duplicate form, the trait-impl form, the const-generic form;
  - MTA: the Plan-149 source-provenance fixture (`transmute(y)` with `y: f32`
    in the fn head) — the question a syntax-only proposer must answer:
    does annotating the SOURCE only (`transmute::<f32, _>`) silence the
    lint, or does it need the TARGET too (which syntax cannot know)?
  - DI: suggestion shape re-record (report-only — no verdict row).

Confirmed present + warn-by-default in 0.1.98 (`-W help`, this run's
derivation): multiple-bound-locations (style), missing-transmute-annotations
(suspicious), derivable-impls (complexity).

Run from this repo's root:

    py scripts/plan180_pack17_probe.py

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

# -- multiple_bound_locations (LEAD-1) ---------------------------------------
# `<T: Default>` inline AND `where T: Default` — remove one. plan169 measured
# plain fires / single-location silent; re-verified + widened this run.
MBL_PLAIN = (
    "pub fn mbl_a<T: Default>(t: T) -> T\n"
    "where\n"
    "    T: Default,\n"
    "{\n"
    "    t\n"
    "}\n"
)
MBL_MULTI_BOUND = (
    "pub fn mbl_m<T: Default + Clone>(t: T) -> T\n"
    "where\n"
    "    T: Default + Clone,\n"
    "{\n"
    "    t.clone()\n"
    "}\n"
)
MBL_TRAIT_IMPL = (
    "pub trait MblT {\n"
    "    fn mbl(&self) -> u8;\n"
    "}\n"
    "impl<T: Default> MblT for T\n"
    "where\n"
    "    T: Default,\n"
    "{\n"
    "    fn mbl(&self) -> u8 {\n"
    "        0\n"
    "    }\n"
    "}\n"
)
MBL_STRUCT = (
    "pub struct MblS<T: Default>\n"
    "where\n"
    "    T: Default,\n"
    "{\n"
    "    t: T,\n"
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

# -- missing_transmute_annotations (LEAD-2 candidate — REJECTED) -------------
# MEASURED VERDICT (2 probe runs, the probe-first law):
#   run 1: naked `transmute(y)` SILENT — the lint does not fire on unannotated
#          calls whose context is locally visible.
#   run 2: partial turbofish (`<_, u32>` / `<f32, _>`) SILENT — and fires
#          `unnecessary_transmutes` instead (the annotation is REDUNDANT when
#          inference already knows both sides). `transmute::<_, _>` is a
#          compile ERROR (E0282), so the "missing both" shape cannot exist.
#   The count-13 default-eco row must fire on inference-across-items shapes —
#   typeck territory, beyond any syntax-only proposer. REJECTED as a rule
#   lead; the matrix below is the permanent negative record.
MTA_NAKED = (
    "pub fn mta_n(y: f32) -> u32 {\n"
    "    let bits: u32 = unsafe { std::mem::transmute(y) };\n"
    "    bits\n"
    "}\n"
)
MTA_PARTIAL_SRC = (
    "pub fn mta_a(y: f32) -> u32 {\n"
    "    let bits: u32 = unsafe { std::mem::transmute::<_, u32>(y) };\n"
    "    bits\n"
    "}\n"
)
MTA_PARTIAL_TGT = (
    "pub fn mta_b(y: f32) -> u32 {\n"
    "    let bits: u32 = unsafe { std::mem::transmute::<f32, _>(y) };\n"
    "    bits\n"
    "}\n"
)
MTA_EXPLICIT = (
    "pub fn mta_d(y: f32) -> u32 {\n"
    "    let bits: u32 = unsafe { std::mem::transmute::<f32, u32>(y) };\n"
    "    bits\n"
    "}\n"
)

# -- derivable_impls (DEFER maintained — suggestion-shape record only) -------
DI_PLAIN = (
    "pub struct DiP {\n"
    "    a: u32,\n"
    "    b: Option<String>,\n"
    "}\n"
    "impl Default for DiP {\n"
    "    fn default() -> Self {\n"
    "        Self { a: 0, b: None }\n"
    "    }\n"
    "}\n"
)

# name -> (source, {lint: min_count}); every expectation must hold
FIXTURES = {
    "mbl_plain": (MBL_PLAIN, {"multiple_bound_locations": 1}),
    "mbl_multi_bound": (MBL_MULTI_BOUND, {"multiple_bound_locations": 1}),
}

# name -> (source, lint) — the lint must NOT fire on the shape
SILENT_SHAPES = {
    "mbl_where_only": (MBL_WHERE_ONLY, "multiple_bound_locations"),
    "mbl_inline_only": (MBL_INLINE_ONLY, "multiple_bound_locations"),
    "mbl_trait_impl": (MBL_TRAIT_IMPL, "multiple_bound_locations"),
    "mbl_struct": (MBL_STRUCT, "multiple_bound_locations"),
    "mta_naked": (MTA_NAKED, "missing_transmute_annotations"),
    "mta_partial_src": (MTA_PARTIAL_SRC, "missing_transmute_annotations"),
    "mta_partial_tgt": (MTA_PARTIAL_TGT, "missing_transmute_annotations"),
    "mta_explicit": (MTA_EXPLICIT, "missing_transmute_annotations"),
}

# MEASURED GUESSES, kept OUT of the verdict: the probe REPORTS these rows
# but never fails on them (the plan169 nbr_let lesson).
#   mta_partial_src_utes: the raw-cargo run (2026-09-28, outside the probe
#   envelope) fired `unnecessary_transmutes` on the partial shape — that
#   was a stale-target replay artifact of the /tmp scratch crate; inside
#   the probe's clean single-fixture envelope the partial shape fires
#   NOTHING (count=0 measured). The row is kept as a watch only.
#   di_plain: DI stays DEFER (semantic-eq + cross-item attr edit); this
#   row records today's suggestion shape for the ledger: whole-impl-block
#   primary span, suggested_replacement None — the synthesized two-item
#   edit class.
GUESS_SHAPES = {
    "mta_partial_src_utes": (MTA_PARTIAL_SRC, "unnecessary_transmutes"),
    "di_plain": (DI_PLAIN, "derivable_impls"),
}

# The hand-written FIXED forms: each must compile with ZERO error-level
# diagnostics AND stay silent on its lint.
FIXED_FORMS = {
    "mbl_inline_dropped": (
        MBL_PLAIN.replace("<T: Default>", "<T>").replace("where\n    T: Default,\n", ""),
        "multiple_bound_locations",
    ),
    "mbl_where_dropped": (MBL_PLAIN.replace("where\n    T: Default,\n", ""), "multiple_bound_locations"),
}

WATCH_LINTS = (
    "derivable_impls",
    "unused_variables",
    "dead_code",
    "clippy::multiple_bound_locations",
)

LIB_HEAD = "// plan180 probe fixture (generated - do not edit)\n"


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
        "path = \"src/lib.rs\"\n",
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
        found.setdefault(lint, []).extend(spans)
    return found, errors


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    clippy_driver = cargo_home / "bin"
    print("plan180 probe — pack XVII candidates (MBL + MTA; DI record-only)")
    if not clippy_driver.exists():
        print(f"  cargo bin not found at {clippy_driver}")
        return 2
    target = Path(tempfile.mkdtemp(prefix="plan180_probe_"))
    failures: list[str] = []
    try:
        for name, (frag, wants) in list(FIXTURES.items()) + list(GUESS_SHAPES.items()):
            found, errors = run_clippy(frag, target)
            wants = {wants: 1} if isinstance(wants, str) else wants
            if errors:
                print(f"  [E] {name}: fixture itself failed to compile:")
                for e in errors[:3]:
                    print(f"      {e['message']}")
                failures.append(name)
                continue
            for lint, want in wants.items():
                got = len(found.get(lint, []))
                kind = "FIXTURE" if name in FIXTURES else "SILENT" if name in SILENT_SHAPES else "GUESS"
                ok = got >= want if kind == "FIXTURE" else got == 0
                mark = "ok" if ok else "FAIL"
                print(f"  [{kind}] {mark} {name}: {lint} count={got} (want {'>=' + str(want) if kind == 'FIXTURE' else '0'})")
                if kind != "GUESS" and not ok:
                    failures.append(name)
                for sp in found.get(lint, []):
                    print(
                        f"      L{sp['line']} primary={sp['text']!r}"
                        f" -> {sp['suggested']!r} ({sp['applicability']})"
                    )
        # report watch-lint co-fires
            for wl in WATCH_LINTS:
                short = wl.removeprefix("clippy::")
                if short in found and short not in wants:
                    print(f"      co-fire: {short} x{len(found[short])}")
        # SILENT_SHAPES: the lint must NOT fire (simple string form)
        for name, (frag, lint) in SILENT_SHAPES.items():
            found, errors = run_clippy(frag, target)
            got = len(found.get(lint, []))
            mark = "ok" if got == 0 else "FAIL"
            print(f"  [SILENT] {mark} {name}: {lint} count={got} (want 0)")
            if got:
                failures.append(name)
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
        print(f"VERDICT: FAIL — {len(failures)} drifted row(s): {sorted(set(failures))}")
        print("  re-adjudicate before authoring (the probe-first law).")
        return 1
    print("VERDICT: PASS — every expectation held.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
