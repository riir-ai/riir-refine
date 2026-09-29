#!/usr/bin/env python3
"""Plan 188 probe — the Batch 191 pack XVIII lead (2026-09-30).

`mem_replace_with_default` (perf, warn-by-default, count 4 / spread 3 in
the default-eco histogram) — the one unadjudicated, bounded,
eq-by-construction row left in the fresh derivation's head at corpus 99
(everything else: the Research-191 skip list, the B168/B171/B184/B185
recorded rejects/refuses/defers, the wave-4 anecdotes, or the
receiver-collision refuses — see Plan 188 T0).

The identity: `std::mem::take` IS `std::mem::replace(dest,
T::default())` — the std definition itself. Upstream 0.1.98 emits a
whole-call MachineApplicable `std::mem::take(dst)` suggestion
(smoke-probe measured 2026-09-30: fires on BOTH path spellings, SILENT
on the `<T as Default>::default()` turbofish).

One clippy run PER FIXTURE (each fixture alone in the probe crate) —
attribution exact by construction. UTF-8 writes, ASCII output (the
plan163 envelope; the plan160 cp874 lesson). Arms:

  FIRING (must fire exactly the target):
  - mrw_std_path:   `std::mem::replace(dst, Default::default())`
  - mrw_imported:   `use std::mem;` + `mem::replace(dst, Default::default())`
  - mrw_expr_ctx:   the call nested in a larger expression (not a let-tail)

  SILENT (must NOT fire — the guard matrix):
  - mrw_turbofish:  `<u64 as Default>::default()` second arg
  - mrw_nondefault: a non-default second arg
  - mrw_take_already: `std::mem::take(dst)` (the fixed form's own silence)
  - mrw_local_mem_shadow: a local `mod mem` shadow — the shadow-guard
    class; upstream fires or not, the PROPOSER refuses either way (this
    arm records upstream's posture for the divergence table)
  - mrw_bare_replace: `use std::mem::replace;` + `replace(dst, ...)` —
    upstream posture recorded; proposer refuses v1 either way

  GUESS (reported, never fails the probe — the plan169 nbr lesson):
  - mrw_underscore_default: `<_>::default()` spelling

  FIXED-FORMS (must compile + stay silent):
  - std-path + imported spellings rewritten to take()

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

# -- mem_replace_with_default (the LEAD) --------------------------------------
# Every fixture avoids the `let x = ...; x` let-tail shape (the smoke probe's
# let_and_return co-fire — the corpus atom must fire EXACTLY its target).
MRW_STD_PATH = (
    "pub fn mrw_std(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) {\n"
    "    sink.push(std::mem::replace(dst, Default::default()));\n"
    "}\n"
)
MRW_IMPORTED = (
    "pub fn mrw_imp(dst: &mut String, sink: &mut Vec<String>) {\n"
    "    use std::mem;\n"
    "    sink.push(mem::replace(dst, Default::default()));\n"
    "}\n"
)
MRW_EXPR_CTX = (
    "pub fn mrw_ctx(dst: &mut Vec<u8>) -> usize {\n"
    "    std::mem::replace(dst, Default::default()).len()\n"
    "}\n"
)
MRW_TURBOFISH = (
    "pub fn mrw_tf(dst: &mut u64, sink: &mut Vec<u64>) {\n"
    "    sink.push(std::mem::replace(dst, <u64 as Default>::default()));\n"
    "}\n"
)
MRW_NONDEFAULT = (
    "pub fn mrw_nd(dst: &mut Vec<u32>, src: Vec<u32>, sink: &mut Vec<Vec<u32>>) {\n"
    "    sink.push(std::mem::replace(dst, src));\n"
    "}\n"
)
MRW_TAKE_ALREADY = (
    "pub fn mrw_tk(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) {\n"
    "    sink.push(std::mem::take(dst));\n"
    "}\n"
)
MRW_LOCAL_MEM_SHADOW = (
    "pub mod mem {\n"
    "    pub fn replace<T: Clone>(dst: &mut T, src: T) -> T {\n"
    "        let out = dst.clone();\n"
    "        *dst = src;\n"
    "        out\n"
    "    }\n"
    "}\n"
    "pub fn mrw_shadow(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) {\n"
    "    sink.push(mem::replace(dst, Default::default()));\n"
    "}\n"
)
MRW_BARE_REPLACE = (
    "pub fn mrw_bare(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) {\n"
    "    use std::mem::replace;\n"
    "    sink.push(replace(dst, Default::default()));\n"
    "}\n"
)
MRW_UNDERSCORE_DEFAULT = (
    "pub fn mrw_us(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) {\n"
    "    sink.push(std::mem::replace(dst, <_>::default()));\n"
    "}\n"
)

LINT = "mem_replace_with_default"

# name -> source; every firing expectation must hold
FIXTURES = {
    "mrw_std_path": MRW_STD_PATH,
    "mrw_imported": MRW_IMPORTED,
    "mrw_expr_ctx": MRW_EXPR_CTX,
}

# name -> source; the lint must NOT fire
SILENT_SHAPES = {
    "mrw_turbofish": MRW_TURBOFISH,
    "mrw_nondefault": MRW_NONDEFAULT,
    "mrw_take_already": MRW_TAKE_ALREADY,
}

# name -> source — upstream posture RECORDED, never gated (the proposer
# refuses these classes by construction; the row is the divergence table).
POSTURE_SHAPES = {
    "mrw_local_mem_shadow": MRW_LOCAL_MEM_SHADOW,
    "mrw_bare_replace": MRW_BARE_REPLACE,
    "mrw_underscore_default": MRW_UNDERSCORE_DEFAULT,
}

# The hand-written FIXED forms: compile with ZERO errors + stay silent.
FIXED_FORMS = {
    "mrw_std_fixed": (
        MRW_STD_PATH.replace(
            "std::mem::replace(dst, Default::default())", "std::mem::take(dst)"
        ),
        LINT,
    ),
    "mrw_imported_fixed": (
        MRW_IMPORTED.replace(
            "mem::replace(dst, Default::default())", "mem::take(dst)"
        ),
        LINT,
    ),
}

WATCH_LINTS = (
    "let_and_return",
    "unnecessary_wraps",
    "useless_vec",
    "default_trait_access",
)

LIB_HEAD = "// plan188 probe fixture (generated - do not edit)\n"


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
    print("plan188 probe - pack XVIII lead: mem_replace_with_default")
    if shutil.which("cargo") is None:
        print("  cargo not found on PATH")
        return 2
    target = Path(tempfile.mkdtemp(prefix="plan188_probe_"))
    failures: list[str] = []
    try:
        # FIRING arms: the lint fires AND MachineApplicable whole-call take()
        for name, frag in FIXTURES.items():
            found, errors = run_clippy(frag, target)
            if errors:
                print(f"  [FIXTURE] FAIL {name}: fixture itself failed to compile:")
                for e in errors[:3]:
                    print(f"      {e['message']}")
                failures.append(name)
                continue
            got = len(found.get(LINT, []))
            ok = got >= 1
            mark = "ok" if ok else "FAIL"
            print(f"  [FIXTURE] {mark} {name}: {LINT} count={got} (want >=1)")
            if not ok:
                failures.append(name)
            for sp in found.get(LINT, []):
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
            got = len(found.get(LINT, []))
            mark = "ok" if got == 0 else "FAIL"
            print(f"  [SILENT] {mark} {name}: {LINT} count={got} (want 0)")
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
            got = len(found.get(LINT, []))
            print(f"  [POSTURE] {name}: {LINT} count={got} (recorded)")
            for sp in found.get(LINT, []):
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
    print("VERDICT: PASS - every expectation held; author per Plan 188 T2.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
