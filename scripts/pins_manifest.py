#!/usr/bin/env python3
"""The corpus pins manifest gate (plan 329 P1 — issue 197).

One committed file — ``.benchmarks/pins_manifest.json`` — is the single
source of truth for every corpus count the repo pins. The numbers are
DERIVED from the compiled corpus builders (the Rust half:
``src/pins_manifest.rs::dump_pins_manifest``, driven by this script under
``cargo test``), never hand-typed; the Rust count asserts read the
manifest via ``crate::pins_manifest::count`` and the live prose rows are
gated against the same values.

Modes:
  (bare)      re-derive the live counts + compare against the committed
              manifest + gate the prose rows; exit 1 naming every
              diverging site. This is the CI layer.
  --write     refresh the committed manifest from live counts (the
              landing pass; its diff is the landing's own proof).
  --self-test both canary directions on temp copies (never touches repo
              state): (a) a drifted committed row reds naming the key
              (the B308 class); (b) a drifted live-corpus count reds
              naming the key (the stale-manifest direction — ``--write``
              is never run-to-pass, the corpus is the truth).

The build posture is fixed: ``--features kernel_opt,sec,rust_perf_bench``
(clippy_lints + rust_perf are default-on). The Rust dump REFUSES a
partial manifest, so a feature typo fails loud on both sides.

Ratchet: any exemption this gate grows (EXEMPT_PROSE rows) takes
deletions only — a fixed prose site graduates back into PROSE_SITES and
its exemption row is deleted in the same commit (the vet / fmt_pending
drain-list shape).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MANIFEST = REPO / ".benchmarks" / "pins_manifest.json"
FEATURES = "kernel_opt,sec,rust_perf_bench"
DUMP_TEST = "dump_pins_manifest"
REQUIRED_DOMAINS = ("clippy_lints", "rust_perf", "kernel_opt", "sec")


def flatten(value, prefix=""):
    """{(a/b/...): scalar} over nested dicts — the comparison vocabulary."""
    rows = {}
    if isinstance(value, dict):
        for k, v in value.items():
            rows.update(flatten(v, f"{prefix}/{k}"))
    else:
        rows[prefix] = value
    return rows


def run_dump() -> dict:
    """Build + run the Rust dump test; return the live counts it emits."""
    with tempfile.NamedTemporaryFile(
        suffix=".json", prefix="pins_dump_", delete=False
    ) as handle:
        out_path = handle.name
    env = dict(os.environ)
    env["RIIR_REFINE_PINS_DUMP"] = out_path
    cmd = [
        "cargo",
        "test",
        "--lib",
        DUMP_TEST,
        "--features",
        FEATURES,
        "--",
        "--exact",
        "pins_manifest::dump_pins_manifest",
    ]
    proc = subprocess.run(
        cmd, cwd=REPO, env=env, capture_output=True, text=True, encoding="utf-8"
    )
    try:
        if proc.returncode != 0:
            sys.stderr.write(proc.stdout[-4000:] + "\n" + proc.stderr[-4000:] + "\n")
            raise SystemExit(
                f"pins_manifest: the dump test failed (exit {proc.returncode}) — "
                "fix the build before touching the manifest"
            )
        raw = Path(out_path).read_text(encoding="utf-8")
        if not raw.strip():
            raise SystemExit(
                "pins_manifest: the dump test ran but wrote nothing — the "
                "--exact test name and src/pins_manifest.rs must agree"
            )
        live = json.loads(raw)
    finally:
        Path(out_path).unlink(missing_ok=True)
    missing = [d for d in REQUIRED_DOMAINS if d not in live]
    if missing:
        raise SystemExit(
            f"pins_manifest: the dump is missing {missing} — the build posture "
            f"is `{FEATURES}` (+ defaults); a partial manifest is refused"
        )
    return live


def compare(committed: dict, live: dict) -> list[tuple[str, object, object]]:
    """Every diverging leaf as (path, committed, live); sorted for stable output."""
    flat_c, flat_l = flatten(committed), flatten(live)
    rows = []
    for key in sorted(set(flat_c) | set(flat_l)):
        if key not in flat_c:
            rows.append((key, "<absent>", flat_l[key]))
        elif key not in flat_l:
            rows.append((key, flat_c[key], "<absent>"))
        elif flat_c[key] != flat_l[key]:
            rows.append((key, flat_c[key], flat_l[key]))
    return rows


# ── The prose half ────────────────────────────────────────────────────────
# Each site: (repo-relative file, line regex with one capture group per
# manifest leaf, manifest leaves in capture order). Only LIVE rows are
# gated — historical prose ("was 966 at Batch 278", eraWitness pins)
# stays untouched history, never a gate row.
PROSE_SITES = [
    (
        "AGENTS.md",
        r"drafts fixes for `cargo clippy` warnings; (\d+) corpus lints",
        ("clippy_lints/rules",),
    ),
    ("AGENTS.md", r"`rust_perf`[^\n]*?\*\*(\d+) rules\*\* standing", ("rust_perf/rules",)),
    ("AGENTS.md", r"`kernel_opt`[^\n]*?\*\*(\d+) rules\*\* standing", ("kernel_opt/rules",)),
    ("AGENTS.md", r"`sec`[^\n]*?\((\d+) rules;", ("sec/rules",)),
    ("README.md", r"\| \*\*rust_perf\*\*[^\n]*?\| (\d+) rules", ("rust_perf/rules",)),
    (
        "README.md",
        r"\| \*\*clippy_lints\*\*[^\n]*?\| (\d+) corpus lints",
        ("clippy_lints/rules",),
    ),
    ("README.md", r"\| \*\*kernel_opt\*\*[^\n]*?\| (\d+) rules", ("kernel_opt/rules",)),
    ("README.md", r"\| \*\*sec\*\*[^\n]*?\| (\d+) rules", ("sec/rules",)),
    (
        ".docs/11_domains/rust_perf.md",
        r"now (\d+) rules across (\d+) categories",
        ("rust_perf/rules", "rust_perf/categories"),
    ),
]

# THE RATCHET: deletions only. A row lands here ONLY when a prose site is
# deliberately retired (the sentence rewritten not to carry a count), with
# the reason; a fixed site graduates back into PROSE_SITES and the row is
# deleted in the same commit.
EXEMPT_PROSE: list[tuple[str, str]] = []


def prose_divergences(manifest: dict) -> list[str]:
    """Check every PROSE_SITES row against the manifest; name each drift."""
    problems = []
    for rel, pattern, leaves in PROSE_SITES:
        path = REPO / rel
        if not path.exists():
            problems.append(f"{rel}: file missing — the prose gate cannot run")
            continue
        text = path.read_text(encoding="utf-8")
        matches = re.findall(pattern, text)
        if not matches:
            problems.append(
                f"{rel}: live count row not found (/{'/'.join(leaves)}) — "
                "the prose gate's regex and the sentence must agree; fix one"
            )
            continue
        if len(matches) > 1:
            problems.append(
                f"{rel}: {len(matches)} rows match the live pattern — the "
                "gate needs one unambiguous live row, found history rows too"
            )
            continue
        captured = matches[0] if isinstance(matches[0], tuple) else (matches[0],)
        for leaf, got in zip(leaves, captured):
            want = manifest
            for part in leaf.split("/"):
                want = want[part]
            if str(want) != str(got):
                problems.append(
                    f"{rel}: prose says {got}, manifest /{leaf} says {want} — "
                    "run `scripts/pins_manifest.py --write` only after the "
                    "corpus change itself landed, and update this prose row "
                    "in the same commit"
                )
    return problems


def self_test() -> int:
    """Both canary directions + the prose extractor, on copies only."""
    committed = json.loads(MANIFEST.read_text(encoding="utf-8"))
    failures = []

    # Canary (a) — the B308 class: a drifted COMMITTED row must red by name.
    drifted = json.loads(json.dumps(committed))
    domain = REQUIRED_DOMAINS[0]
    first_key = next(iter(drifted[domain]))
    drifted[domain][first_key] += 1
    rows = compare(committed, drifted)
    if len(rows) == 1 and domain in rows[0][0]:
        print(f"canary a PASS: drifted committed row reds naming {rows[0][0]}")
    else:
        failures.append(f"canary a FAIL: expected exactly one named row, got {rows}")

    # Canary (b) — the stale-manifest direction: a live-corpus move without
    # --write must red on the manifest (the corpus is the truth).
    live = json.loads(json.dumps(committed))
    live["rust_perf"]["rules"] += 1
    rows = compare(live, committed)
    if any(r[0] == "/rust_perf/rules" for r in rows):
        print("canary b PASS: stale manifest reds naming /rust_perf/rules")
    else:
        failures.append(f"canary b FAIL: /rust_perf/rules drift not named, got {rows}")

    # The prose extractor: a drifted manifest must name the prose site.
    drifted_prose = json.loads(json.dumps(committed))
    drifted_prose["rust_perf"]["rules"] += 7
    problems = prose_divergences(drifted_prose)
    if any("rust_perf/rules" in p and "AGENTS.md" in p for p in problems):
        print("canary c PASS: drifted manifest names the AGENTS.md prose row")
    else:
        failures.append(f"canary c FAIL: prose drift not named, got {problems}")

    if failures:
        for f in failures:
            print(failures and f)
        return 1
    print("pins_manifest self-test: 3/3 canaries PASS")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--write",
        action="store_true",
        help="refresh the committed manifest from the live compiled counts",
    )
    parser.add_argument(
        "--self-test", action="store_true", help="run the two-direction canaries + prose extractor"
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    live = run_dump()

    if args.write:
        MANIFEST.write_text(json.dumps(live, indent=2) + "\n", encoding="utf-8")
        print(f"pins_manifest: wrote {MANIFEST.relative_to(REPO)} (the landing's own diff)")
        return 0

    if not MANIFEST.exists():
        raise SystemExit(
            "pins_manifest: .benchmarks/pins_manifest.json is missing — run "
            "`scripts/pins_manifest.py --write` once and commit the manifest"
        )
    committed = json.loads(MANIFEST.read_text(encoding="utf-8"))
    rows = compare(committed, live)
    for key, was, now in rows:
        print(f"pins_manifest: DRIFT {key}: manifest {was} vs live corpus {now}")
    if rows:
        print(
            "pins_manifest: the corpus moved without --write — run "
            "`scripts/pins_manifest.py --write` and land the manifest diff "
            "WITH the corpus change (never run-to-pass)"
        )
        return 1

    problems = prose_divergences(committed)
    for p in problems:
        print(f"pins_manifest: PROSE DRIFT {p}")
    for rel, reason in EXEMPT_PROSE:
        print(f"pins_manifest: exempt (deletions-only ratchet): {rel} — {reason}")
    if problems:
        return 1
    print("pins_manifest: manifest + prose agree with the compiled corpus")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
