#!/usr/bin/env python3
"""Plan 150 T1 — probe the Batch-159 leads' GROUPS + the re-arm candidates
with REAL clippy.

Replicates the ClippyOracle invocation exactly:
  cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery
in a temp crate per fixture. Prints the fired clippy lint ids per case.

Group-pin arms (the leads being LANDED as rules this batch):
  - `manual_clamp` had NO default-groups arm in plan149 — the group pin is
    owed for the corpus entry (`lint_group` field).
  - `single_char_add_str` re-pinned for the record (plan149 verified it).

Re-arm discipline (Plan 146 T0's): seed must be clean under the FULL oracle
(incl. the missing_const_for_fn nursery discipline — dodged via non-const
OPS: Vec::push / String::push / mutable locals), broken must fire EXACTLY
its target lint.

Candidates this round (the plan149-recorded spare + 4):
  - `from_str_radix_10` (the recorded spare, now with the Vec::push dodge)
  - `assign_op_pattern` (`y = y + 1` → `y += 1`)
  - `string_extend` (`s.extend("ab")` → `s.push_str("ab")`)
  - `double_neg` (`!!x` → `!x`)
  - `unit_arg` (the round-1 reject RETRIED with a side-effect block — the
    recorded dirt was a const-able seed + the `no_effect` crossfire on
    `{ 1; }`; both dodged via Vec::push ops)
"""
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

CASES = {
    # ── re-arm candidate 1: from_str_radix_10 (style, default; the
    #    plan149-recorded spare, Vec::push const-dodge added). ──
    "fsr-seed": (
        "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError>"
        " { hits.push(0); s.parse::<u32>() }"
    ),
    "fsr-broken": (
        "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError>"
        " { hits.push(0); u32::from_str_radix(s, 10) }"
    ),
    # ── re-arm candidate 2: assign_op_pattern (style, default): `y = y + 1`
    #    ↔ `y += 1`. Mutable local + Vec::push dodge. ──
    "aop-seed": (
        "fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y += 1; y }"
    ),
    "aop-broken": (
        "fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = y + 1; y }"
    ),
    # ── re-arm candidate 3: string_extend (style, default): `s.extend("ab")`
    #    ↔ `s.push_str("ab")` (2-char literal dodges single_char_add_str
    #    both sides; String::push_str non-const). ──
    "se-seed": (
        "fn grow(s: &mut String) { s.push_str(\"ab\"); }"
    ),
    "se-broken": (
        "fn grow(s: &mut String) { s.extend(\"ab\"); }"
    ),
    # ── re-arm candidate 4: double_neg (style, default): `!!x` ↔ `!x`.
    #    Risk: the nonminimal_bool crossfire. ──
    "dn-seed": (
        "fn flip(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); !x }"
    ),
    "dn-broken": (
        "fn flip(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); !!x }"
    ),
    # ── re-arm candidate 5: unit_arg RETRY (the plan149 reject): the seed
    #    was const-able dirty + `{ 1; }` crossfired no_effect. Side-effect
    #    block + Vec::push dodges both. ──
    "ua-seed": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch((), hits) }"
    ),
    "ua-broken": (
        "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\n"
        "fn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }"
    ),
    # ── negative control: known-clean shape under the full oracle. ──
    "neg-ctl": (
        "fn plain(x: u32, hits: &mut Vec<u32>) -> bool { hits.push(x); x > 0 }"
    ),
}

# Default-group arms (no -W flags): a hit here = default-level lint — the
# corpus `lint_group` pin for the two Batch-159 leads.
GROUP_CASES = {
    "grp-sca-broken": (
        "fn mark(s: &mut String) { s.push_str(\"!\"); }"
    ),
    "grp-mc-broken": (
        "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(3).min(5) }"
    ),
}


def probe(name: str, code: str, default_groups: bool = False) -> list[str]:
    root = Path(tempfile.mkdtemp(prefix=f"probe150_{name}_"))
    try:
        (root / "Cargo.toml").write_text(
            "[package]\nname = \"probe150\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"
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
            cwd=root, capture_output=True, text=True, timeout=120,
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
    print("── default-group arms (no -W flags; a hit = default-level) ──")
    for name, code in GROUP_CASES.items():
        try:
            lints = probe(name, code, default_groups=True)
        except Exception as e:  # noqa: BLE001
            print(f"{name:28} PROBE-ERROR {e}")
            continue
        print(f"{name:28} {lints}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
