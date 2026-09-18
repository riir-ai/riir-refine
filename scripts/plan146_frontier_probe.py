#!/usr/bin/env python3
"""Plan 146 T0 — probe new frontier-family candidates with REAL clippy.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case.
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


CASES = {
    # ── candidate: io_other_error ─────────────────────────────────────
    "io_other_error-seed": (
        "fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }"
    ),
    "io_other_error-broken": (
        "fn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, e) }"
    ),
    # ── candidate: assigning_clones (v2 — no impl-typed param, no use_self) ──
    "assigning_clones-seed": (
        "struct Box2 { label: String }\nimpl Box2 {\n    fn relabel(&mut self, fresh: String) { self.label.clone_from(&fresh); }\n}"
    ),
    "assigning_clones-broken": (
        "struct Box2 { label: String }\nimpl Box2 {\n    fn relabel(&mut self, fresh: String) { self.label = fresh.clone(); }\n}"
    ),
    # ── candidate: unit_arg ───────────────────────────────────────────
    "unit_arg-seed": (
        "fn log_finish() {}\nfn run() { log_finish(); }"
    ),
    "unit_arg-broken": (
        "fn log_finish(u: ()) {}\nfn run() { log_finish(()); }"
    ),
    # ── candidate: wrong_self_convention (v2 — seed reads self, non-const) ──
    "wsc-seed": (
        "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(self) -> String { format!(\"sig-{}\", self.code) }\n}"
    ),
    "wsc-broken": (
        "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}"
    ),
}

def probe(name: str, code: str) -> list[str]:
    root = Path(tempfile.mkdtemp(prefix=f"probe146_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe146\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
            "[lib]\npath = \"src/lib.rs\"\n",
            encoding="utf-8",
        )
        (root / "src").mkdir()
        # mirror the oracle wrap: dead-code/unused silences, no clippy change
        (root / "src" / "lib.rs").write_text(
            "#![allow(dead_code, unused)]\n" + code + "\n",
            encoding="utf-8",
        )
        out = subprocess.run(
            ["cargo", "clippy", "--message-format=json", "--",
             "-W", "clippy::all", "-W", "clippy::pedantic", "-W", "clippy::nursery"],
            cwd=root, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=120,
        )
        lints = set()
        for line in out.stdout.splitlines():
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if msg.get("reason") == "compiler-message":
                codev = msg.get("message", {}).get("code", {})
                if codev and codev.get("code", "").startswith("clippy::"):
                    lints.add(codev["code"])
        return sorted(lints)
    finally:
        shutil.rmtree(root, ignore_errors=True)

def main() -> int:
    for name, code in CASES.items():
        try:
            lints = probe(name, code)
        except Exception as e:  # noqa: BLE001
            print(f"{name:28} PROBE-ERROR {e}")
            continue
        print(f"{name:28} {lints}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
