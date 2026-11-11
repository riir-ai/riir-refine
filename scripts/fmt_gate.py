#!/usr/bin/env python3
"""rustfmt-clean, with a pinned drain list — Issue 223 T4 (delegated-owner
verdicts: AGREE session 95ae0195 — the decision of record, 7 binding
conditions; REVISE session e2c0b216 — its three corrections are folded here).
Ported from riir-reflex/scripts/fmt_gate.py — itself the katgpt-rs Issue 946
canonical gate, ported for a root-package workspace.

⛔ NEVER run `cargo fmt --all` in this repo: cargo-fmt's --all formats
"all packages, and also their local path-based dependencies", and the
root-package workspace (issue 200 C1) auto-resolves the sibling path deps
(../katgpt-rs, ../riir-kat, ../riir-infer, ...) into that closure — a bare
--all would rewrite sibling files as uncommitted working-tree changes under
concurrent sessions (measured in reflex 2026-10-27 from a detached worktree;
measured here 2026-11-11 the same way). Equally never `cargo fmt -p X --
file.rs` — everything after `--` is a rustfmt OPTION and the whole package
formats. The walker is bare `rustfmt --check`, batched over THIS repo's own
git-tracked .rs (the rustfmt.toml (T1) pins edition = "2021" +
style_edition = "2024", so bare `rustfmt <file>` — and this scan — parse at
2021 and format at style 2024, byte-identical to what `cargo fmt` would
produce in-repo).

The edition passed to rustfmt is READ from the root Cargo.toml — never
hard-coded: an `--edition 2024` override would parse at 2024 while cargo fmt
parses at 2021, re-creating the two-formatter split that caused Issue 223.

Exclusions are the ONE shared machine map — `scripts/fmt_exclusions.txt`
(condition 1: the gate AND any sweep read it; the issue table is the human
record, the map is authoritative). Row shape: `glob [tripwire|notripwire] #
reason`; a tagged row without a reason is a config error. TRIPWIRE rows
(condition 3) are checked the INVERTED way: an excluded record file reading
rustfmt-CLEAN is a FINDING — a byte-exact/executed-provenance artifact whose
entry state was unformatted must never read clean, because clean means the
bytes were formatted after entry and the record no longer matches what was
executed/embedded. A glob measured entry-CLEAN at adoption carries
`notripwire` with the measured reason instead (measured 2026-11-11: the
corpus-fixtures and sec-fixtures authors formatted BEFORE executing, so
entry-clean is legitimate there; the generated dot_per_row pair is entry-
DIRTY, so its tripwire is armed).

Timing (measured on the quiet-box calibration worktree, 2026-11-11): one
spawn per file ≈ 0.6–1.5 s warm ⇒ per-file over ~830 files ≈ 6–17 min —
the predecessor's 5-minute wall, confirmed. The scan is therefore BATCHED:
one rustfmt spawn per 64-file chunk with `-l` for attribution
(~10–45 s warm whole-tree, measured 42.8 s all-852 single-batch); a chunk
whose exit ∉ {0, 1} or whose stderr carries `error:` is REDONE per-file so
the refusal (and any parse error) is attributed to its file, never pooled.

Verdict, by MEMBERSHIP, both directions (a count is green on a swap):

- **UNPINNED** — a diverging walked file no row names: a NEW drift. Run
  `rustfmt <file>` (bare — the toml carries edition + style) and commit.
  NEVER pin new drift; the list only shrinks (condition 6: rows only join
  with a toolchain bump + reason).
- **STALE** — a drain-list row whose file is now clean (or gone): delete the
  row in the same commit.
- **FORMATTED-RECORD** — a tripwire row whose file reads rustfmt-clean: the
  record's bytes moved after entry. Restore the executed bytes from git
  history (never re-format), then adjudicate the row's tag.

Refusals (exit 2, NO VERDICT — never a pass): a rustfmt spawn exiting
outside {0, 1} (condition 4), a walk below the map's `min_files_checked`
floor, a missing rustfmt binary (LOUD — blocking, never a silent skip), a
missing pins/map file, or a reasonless/tagless map row.

Runs as Layer 8 of scripts/ci_feature_guard.sh — zero cargo, warm ~1 min on
a quiet box; the guard's completion sentinel is untouched (condition 5).
"""

from __future__ import annotations

import fnmatch
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# A non-UTF-8 console (cp874) would kill this script on its first
# non-ASCII glyph; degrade the glyph, never the verdict (katgpt-rs
# Issue 804). A captured stream without reconfigure is left alone.
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(errors="backslashreplace")
    except (AttributeError, ValueError):
        pass

ROOT = Path(__file__).resolve().parent.parent
MAP = ROOT / "scripts" / "fmt_exclusions.txt"
PINS = ROOT / "scripts" / "fmt_pending_expected.txt"
BATCH = 64

TAGS = ("tripwire", "notripwire")


def load_map(path: Path) -> tuple[dict, list[str]]:
    """Parse fmt_exclusions.txt: directives + exclusion rows.

    Returns ({min_files_checked: int, rows: [(pattern, tag, reason)]}, errors).
    Row shape: `<glob> [tripwire|notripwire] # reason`; a tagged row without a
    reason is a config error; an untagged row is a plain exclusion (never
    tripwired). Glob semantics are fnmatch (`*` crosses `/`).
    """
    cfg: dict = {"min_files_checked": None, "rows": []}
    errors: list[str] = []
    for n, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.split()[0] == "min_files_checked":
            try:
                cfg["min_files_checked"] = int(line.split()[1])
            except (IndexError, ValueError):
                errors.append(f"{path.name}:{n}: bad min_files_checked: {line!r}")
            continue
        rest, sep, reason = line.partition("#")
        if not sep or not reason.strip():
            errors.append(f"{path.name}:{n}: row has no reason: {line!r}")
            continue
        parts = rest.strip().split()
        if len(parts) == 2 and parts[1] in TAGS:
            pattern, tag = parts
        elif len(parts) == 1:
            pattern, tag = parts[0], ""
        else:
            errors.append(f"{path.name}:{n}: bad row shape: {line!r}")
            continue
        if not pattern.strip():
            errors.append(f"{path.name}:{n}: empty pattern: {line!r}")
            continue
        cfg["rows"].append((pattern, tag, reason.strip()))
    if cfg["min_files_checked"] is None:
        errors.append(f"{path.name}: missing `min_files_checked <N>` directive")
    return cfg, errors


def load_pins(path: Path) -> tuple[set[str], list[str]]:
    """Rows are `<relpath> # reason`. A reasonless row is refused."""
    pins: set[str] = set()
    errors: list[str] = []
    for n, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        rel, sep, reason = line.partition("#")
        if not sep or not reason.strip():
            errors.append(f"{path.name}:{n}: row has no reason: {line!r}")
            continue
        pins.add(rel.strip())
    return pins, errors


def edition(root: Path) -> str | None:
    """The root [package] edition — rustfmt parses at it; the repo
    rustfmt.toml supplies style_edition. NEVER hard-code an edition here."""
    manifest = root / "Cargo.toml"
    for line in manifest.read_text(encoding="utf-8").splitlines():
        m = re.match(r'^\s*edition\s*=\s*"([^"]+)"', line)
        if m:
            return m.group(1)
    return None


def classify_map(rows: list[tuple[str, str, str]], rel: str) -> tuple[str, str] | None:
    """(tag, reason) of the FIRST map row matching `rel`, else None.

    Exact rows win over globs regardless of order; among globs, first match
    in file order wins (fnmatch semantics — `*` crosses `/`).
    """
    for pattern, tag, reason in rows:
        if ( "*" not in pattern and "?" not in pattern and "[" not in pattern
             and pattern == rel ):
            return (tag, reason)
    for pattern, tag, reason in rows:
        if "*" in pattern or "?" in pattern or "[" in pattern:
            if fnmatch.fnmatch(rel, pattern):
                return (tag, reason)
    return None


def walk(root: Path, rows: list[tuple[str, str, str]]) -> tuple[list[str], dict[str, int], dict[str, int]]:
    """(walked files, exclusion counts per reason, tripwire counts per reason)
    — git is the truth."""
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "*.rs"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if out.returncode != 0:
        return [], {}, {}
    files = [f for f in out.stdout.splitlines() if f.strip()]
    kept: list[str] = []
    excluded: dict[str, int] = {}
    tripwires: dict[str, int] = {}
    for f in files:
        hit = classify_map(rows, f)
        if hit is None:
            kept.append(f)
            continue
        tag, reason = hit
        excluded[reason] = excluded.get(reason, 0) + 1
        if tag == "tripwire":
            tripwires[reason] = tripwires.get(reason, 0) + 1
    return kept, excluded, tripwires


def _spawn(root: Path, rels: list[str], edition_: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["rustfmt", "--edition", edition_, "--check", "--config", "skip_children=true", "-l"]
        + rels,
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )


def scan(root: Path, files: list[str], edition_: str) -> tuple[set[str], str | None]:
    """(diverging files, first refusal reason) over the whole walk.

    Batched fast path: one spawn per BATCH-file chunk with `-l`; the chunk's
    stdout lists diverging files. A chunk exiting outside {0, 1} or carrying
    `error:` on stderr is REDONE per-file so the refusal is attributed to its
    file — a parse error or an unrecognized option must never pool into a
    batch-level verdict, and the files after the bad one in the chunk must
    still be checked.
    """
    diverging: set[str] = set()
    for i in range(0, len(files), BATCH):
        chunk = files[i : i + BATCH]
        try:
            proc = _spawn(root, chunk, edition_)
        except OSError as exc:
            return set(), f"rustfmt not runnable: {exc} — NO VERDICT, blocking (condition 4)"
        if "error:" in proc.stderr or proc.returncode not in (0, 1):
            for rel in chunk:
                try:
                    single = _spawn(root, [rel], edition_)
                except OSError as exc:
                    return set(), f"rustfmt not runnable: {exc} — NO VERDICT, blocking (condition 4)"
                if "error:" in single.stderr:
                    first = single.stderr.strip().splitlines()[0]
                    return set(), f"rustfmt failed on {rel}: {first} — NO VERDICT, never a classification"
                if single.returncode not in (0, 1):
                    return set(), f"rustfmt exited {single.returncode} on {rel} — NO VERDICT, never a classification"
                if single.returncode == 1:
                    diverging.add(rel)
            continue
        if proc.returncode == 1:
            listed = [ln.strip() for ln in proc.stdout.splitlines() if ln.strip()]
            if not listed:
                return set(), (
                    "rustfmt exited 1 but listed no file — NO VERDICT "
                    "(the founding phantom: a phantom exit must never read as drift or as clean)"
                )
            for ln in listed:
                diverging.add(Path(ln).resolve().as_posix())
    # -l prints absolute (or cwd-joined) paths; map back to repo-relative.
    root_posix = root.resolve().as_posix()
    resolved: set[str] = set()
    for d in diverging:
        rel = d
        if rel.startswith(root_posix + "/"):
            rel = rel[len(root_posix) + 1 :]
        resolved.add(rel)
    return resolved, None


def tripwire_scan(root: Path, tripwire_files: dict[str, list[str]], edition_: str) -> tuple[list[tuple[str, str]], str | None]:
    """(findings, refusal) over the tripwire classes — checked INVERTED:
    a record file reading rustfmt-CLEAN is a finding (condition 3)."""
    findings: list[tuple[str, str]] = []
    for reason, rels in tripwire_files.items():
        for rel in rels:
            try:
                proc = _spawn(root, [rel], edition_)
            except OSError as exc:
                return [], f"rustfmt not runnable: {exc} — NO VERDICT, blocking (condition 4)"
            if "error:" in proc.stderr:
                first = proc.stderr.strip().splitlines()[0]
                return [], f"rustfmt failed on {rel}: {first} — NO VERDICT, never a classification"
            if proc.returncode not in (0, 1):
                return [], f"rustfmt exited {proc.returncode} on {rel} — NO VERDICT, never a classification"
            if proc.returncode == 0:
                findings.append((rel, reason))
    return findings, None


def verdict(diverging: set[str], pins: set[str]) -> tuple[list[str], list[str]]:
    """(unpinned, stale) — the whole decision, extracted so the arms reach it."""
    return sorted(diverging - pins), sorted(pins - diverging)


def selftest() -> list[str]:
    fails: list[str] = []
    if verdict({"a.rs"}, {"a.rs"}) != ([], []):
        fails.append("exact match must be clean")
    if verdict({"a.rs", "n.rs"}, {"a.rs"}) != (["n.rs"], []):
        fails.append("a new drift must be UNPINNED")
    if verdict(set(), {"a.rs"}) != ([], ["a.rs"]):
        fails.append("a drained row must be STALE")
    if verdict({"x.rs"}, {"y.rs"}) != (["x.rs"], ["y.rs"]):
        fails.append("a swap must red both ways (membership, not count)")
    # load_pins refusals are exercised against a temp file, not a fixture path.
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False, encoding="utf-8") as tf:
        tf.write("ok.rs # a reason\nbad.rs\n")
        tmp = Path(tf.name)
    pins, errors = load_pins(tmp)
    tmp.unlink()
    if not errors or "no reason" not in errors[0]:
        fails.append(f"a reasonless row must be refused: {errors=}")
    if pins != {"ok.rs"}:
        fails.append(f"the good row must still pin while the bad row refuses: {pins=}")
    # load_map arms: directive, tags, reasons, exact-vs-glob precedence.
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False, encoding="utf-8") as tf:
        tf.write(
            "min_files_checked 10\n"
            "vendor/** # vendored\n"
            "tests/fixtures/dot_per_row_*.rs tripwire # byte-exact include_str inputs\n"
            "src/domains/sec/fixtures/b342_fixtures.rs notripwire # measured entry-clean\n"
            "src/domains/sec/fixtures/*.rs tripwire # executed provenance\n"
            "badrow\n"
            "tagged-noreason tripwire\n"
        )
        tmp = Path(tf.name)
    cfg, errors = load_map(tmp)
    tmp.unlink()
    if cfg["min_files_checked"] != 10:
        fails.append(f"directive must parse: {cfg['min_files_checked']=}")
    if len(cfg["rows"]) != 4:
        fails.append(f"four good rows expected, got {len(cfg['rows'])}")
    if not any("no reason" in e for e in errors):
        fails.append(f"a reasonless tagged row must be refused: {errors=}")
    if not any("no reason" in e for e in errors):
        fails.append("a reasonless untagged row must be refused")
    rows = cfg["rows"]
    hit = classify_map(rows, "src/domains/sec/fixtures/b342_fixtures.rs")
    if hit is None or hit[0] != "notripwire":
        fails.append(f"exact row must win over glob: {hit=}")
    hit = classify_map(rows, "src/domains/sec/fixtures/b350_fixtures.rs")
    if hit is None or hit[0] != "tripwire":
        fails.append(f"glob row must match non-exact sec fixture: {hit=}")
    hit = classify_map(rows, "vendor/foo/lib.rs")
    if hit is None or hit[0] != "":
        fails.append(f"untagged row must be a plain exclusion: {hit=}")
    if classify_map(rows, "src/lib.rs") is not None:
        fails.append("a walked file must not classify into the map")
    return fails


def main() -> int:
    fails = selftest()
    if fails:
        for f in fails:
            print(f"✗ fmt gate self-test: {f}")
        return 2

    if not MAP.exists():
        print(f"✗ fmt gate NO VERDICT — exclusion map missing: {MAP}")
        return 2
    cfg, errors = load_map(MAP)
    if errors:
        for e in errors:
            print(f"✗ {e}")
        return 1
    rows = cfg["rows"]
    floor = cfg["min_files_checked"]

    if not PINS.exists():
        print(f"✗ fmt gate NO VERDICT — pins file missing: {PINS}")
        return 2
    pins, errors = load_pins(PINS)
    if errors:
        for e in errors:
            print(f"✗ {e}")
        return 1

    edition_ = edition(ROOT)
    if edition_ is None:
        print('✗ fmt gate NO VERDICT — no `edition = "..."` in the root Cargo.toml')
        return 2
    shutil_which = None
    try:
        import shutil

        shutil_which = shutil.which("rustfmt")
    except ImportError:
        pass
    if shutil_which is None:
        print("✗ fmt gate NO VERDICT — rustfmt not found on PATH (install the rustup component) — blocking, never a silent skip (condition 4)")
        return 2

    files, excluded, tripwires = walk(ROOT, rows)
    excluded_total = sum(excluded.values())
    if len(files) < floor:
        print(
            f"✗ fmt gate BLIND — walk returned {len(files)} file(s) (excluded {excluded_total}) "
            f"< map floor {floor}"
        )
        return 2

    # Tripwire half first — the inverted check is cheap and loudest.
    trip_files: dict[str, list[str]] = {}
    for f in files:
        pass
    all_files_out = subprocess.run(
        ["git", "-C", str(ROOT), "ls-files", "*.rs"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if all_files_out.returncode != 0:
        print("✗ fmt gate NO VERDICT — git ls-files failed (walk regression)")
        return 2
    for f in (x for x in all_files_out.stdout.splitlines() if x.strip()):
        hit = classify_map(rows, f)
        if hit is not None and hit[0] == "tripwire":
            trip_files.setdefault(hit[1], []).append(f)
    findings, refusal = tripwire_scan(ROOT, trip_files, edition_)
    if refusal is not None:
        print(f"✗ fmt gate {refusal}")
        return 2
    for rel, reason in findings:
        print(
            f"  ✗ FORMATTED-RECORD {rel} — tripwire [{reason}]: the record reads rustfmt-CLEAN, "
            f"meaning its bytes were formatted AFTER entry; restore the executed bytes from git "
            f"history (never re-format), then adjudicate the row's tag"
        )

    diverging, refusal = scan(ROOT, files, edition_)
    if refusal is not None:
        print(f"✗ fmt gate {refusal}")
        return 2

    unpinned, stale = verdict(diverging, pins)
    for f in unpinned:
        print(f"  ✗ UNPINNED {f} — run `rustfmt {f}` (bare; the toml carries edition + style) and commit it")
    for f in stale:
        print(f"  ✗ STALE {f} — now rustfmt-clean (or gone); delete its row in {PINS.name}")
    if unpinned or stale or findings:
        print(
            f"✗ fmt gate FAILED — {len(unpinned)} unpinned, {len(stale)} stale, "
            f"{len(findings)} formatted-record over {len(files)} files (excluded {excluded_total})"
        )
        return 1
    detail = "; ".join(f"{c}x [{r[:60]}]" for r, c in sorted(excluded.items()))
    tw = sum(tripwires.values())
    print(
        f"✓ fmt gate PASSED — {len(files)} files checked, {len(pins)} pinned on the drain list, "
        f"{tw} tripwired record files clean-inverted OK "
        f"(excluded {excluded_total}: {detail})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
