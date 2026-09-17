#!/usr/bin/env python3
"""Bench 094 Extend protocol driver — the default-level fired histogram sampler.

Methodology owner: `.benchmarks/094_default_level_fired_histogram.md`.
Wave 1 (2026-09-16, `data/094_t01`) was captured by a throwaway driver; this
tracked script re-implements the SAME methodology (the Extend protocol is
explicitly rerunnable) and adds the wave-merge step.

Subcommands:
  sample — fetch crates.io pages, filter, interleave, dedup vs the base wave,
           download+extract each `.crate`, run plain `cargo clippy
           --quiet --message-format=json` (DEFAULT lint level) in a bounded
           worker pool, tally clippy::* codes, append per-crate results to a
           partial JSONL (resume-safe: done crates are skipped on re-run).
  merge  — read the base manifest + the partial JSONL, emit the merged
           manifest (same schema v1) with dataset_blake3 = blake3 over the
           canonical JSON of the manifest minus the blake3 field itself.

Merge math is EXACT because crate sets are disjoint after dedup: per lint,
count' = count1 + count2 and spread' = spread1 + spread2; per-crate verdicts
concatenate; only verdict=="ok" crates contribute to the histogram.

Population floors live in the bench doc, not here (compiled_clean ~150+).
"""

from __future__ import annotations

import argparse
import concurrent.futures
import io
import json
import os
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone

API = "https://crates.io/api/v1/crates"
STATIC = "https://static.crates.io/crates/{name}/{name}-{ver}.crate"
UA = "riir-clippy-bench094-extend (gist.rs workspace instrumentation)"
PFX = "clippy::"


def _out() -> None:
    for s in (sys.stdout, sys.stderr):
        try:
            s.reconfigure(encoding="utf-8", errors="backslashreplace")
        except Exception:
            pass


def _fetch_json(url: str, tries: int = 3) -> dict:
    last = None
    for i in range(tries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=60) as r:
                return json.load(io.TextIOWrapper(r, encoding="utf-8", errors="replace"))
        except Exception as e:  # noqa: BLE001 — retry any transport failure
            last = e
            time.sleep(2 * (i + 1))
    raise SystemExit(f"fetch failed after {tries} tries: {url}: {last}")


def fetch_candidates(pages: list[int], per_page: int, min_downloads: int) -> tuple[list[dict], int]:
    """Round-robin interleave of the per-page filtered rows.

    Returns (interleaved_passing_rows, rows_seen) — rows_seen counts every
    passing row BEFORE the target cut (the population.candidates half).
    """
    bands: list[list[dict]] = []
    seen_names: set[str] = set()
    rows_seen = 0
    for p in pages:
        d = _fetch_json(f"{API}?page={p}&per_page={per_page}&sort=downloads")
        band = []
        for c in d.get("crates", []):
            if c.get("yanked") or not c.get("max_stable_version"):
                continue
            if c.get("downloads", 0) < min_downloads:
                continue
            if c["name"] in seen_names:
                continue
            seen_names.add(c["name"])
            band.append({"name": c["name"], "ver": c["max_stable_version"], "downloads": c["downloads"]})
        rows_seen += len(band)
        bands.append(band)
        print(f"  page {p}: {len(band)} passing", flush=True)
        time.sleep(1.0)
    out: list[dict] = []
    i = 0
    while any(bands):
        for b in bands:
            if b:
                out.append(b.pop(0))
        i += 1
    return out, rows_seen


def download_crate(name: str, ver: str, dest_dir: str) -> str:
    os.makedirs(dest_dir, exist_ok=True)
    path = os.path.join(dest_dir, f"{name}-{ver}.crate")
    if os.path.exists(path) and os.path.getsize(path) > 0:
        return path
    last = None
    for i in range(3):
        try:
            req = urllib.request.Request(STATIC.format(name=name, ver=ver), headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=120) as r, open(path + ".part", "wb") as f:
                while True:
                    chunk = r.read(1 << 16)
                    if not chunk:
                        break
                    f.write(chunk)
            os.replace(path + ".part", path)
            return path
        except Exception as e:  # noqa: BLE001
            last = e
            time.sleep(2 * (i + 1))
    raise SystemExit(f"download failed: {name}-{ver}: {last}")


def extract_crate(path: str, dest_root: str) -> str:
    os.makedirs(dest_root, exist_ok=True)
    with tarfile.open(path, "r:gz") as t:
        t.extractall(dest_root)  # noqa: S202 — measured third-party corpus, sandboxed scratch
    # the tarball's single top-level dir is the crate root
    entries = [e for e in os.listdir(dest_root) if os.path.isdir(os.path.join(dest_root, e))]
    for e in entries:
        if os.path.exists(os.path.join(dest_root, e, "Cargo.toml")):
            return os.path.join(dest_root, e)
    # fallback: nested one level (some packs wrap)
    for e in entries:
        sub = os.path.join(dest_root, e)
        for s in os.listdir(sub):
            if os.path.exists(os.path.join(sub, s, "Cargo.toml")):
                return os.path.join(sub, s)
    raise SystemExit(f"no Cargo.toml after extract: {path}")


def run_clippy(crate_dir: str, target_dir: str, timeout_s: int) -> dict:
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = target_dir
    env.setdefault("CARGO_TERM_COLOR", "never")
    t0 = time.time()
    try:
        proc = subprocess.run(
            ["cargo", "clippy", "--quiet", "--message-format=json"],
            cwd=crate_dir,
            env=env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_s,
        )
    except subprocess.TimeoutExpired:
        return {"verdict": "timeout", "exit_code": None, "stderr_tail": "", "lints": {}, "wall_s": round(time.time() - t0, 1)}
    lints: dict[str, int] = {}
    for line in (proc.stdout or "").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if obj.get("reason") != "compiler-message":
            continue
        # cargo's JSON schema: the diagnostic lives under "message"
        # ({"reason":"compiler-message","message":{...},"package_id":...});
        # "msg" was the wrong key and tallied ZERO lints (wave-2 bug, caught
        # by the akin-0.4.0 known-dirty control before any data was used).
        msg = obj.get("message") or {}
        code = (msg.get("code") or {}).get("code") or ""
        if code.startswith(PFX) and str(msg.get("level", "")).startswith("warning"):
            lint = code[len(PFX):]
            lints[lint] = lints.get(lint, 0) + 1
    verdict = "ok" if proc.returncode == 0 else "compile_error"
    return {
        "verdict": verdict,
        "exit_code": proc.returncode,
        "stderr_tail": (proc.stderr or "")[-240:],
        "lints": lints,
        "wall_s": round(time.time() - t0, 1),
    }


def cmd_sample(a: argparse.Namespace) -> None:
    base = json.load(open(a.base_manifest, encoding="utf-8")) if a.base_manifest else {"crates": [], "compile_error_crates": []}
    # Cross-wave dedup uses the CONSTRUCTED key f"{name}-{ver}" — the base
    # manifest's per-crate rows carry exactly that key (cmd_sample line ~242
    # / cmd_merge line ~387 both build it the same way). The wave-4 fix: this
    # used to compare the candidate's BARE name against those versioned keys
    # — never a match, so the dedup was dead code and every cross-wave
    # collision (rank drift at adjacent-band boundaries) would have been
    # re-measured and DOUBLE-COUNTED by cmd_merge (whose exactness premise is
    # disjoint crate sets). Same class as the wave-2/wave-3 resume-key
    # incidents — exact keys close it. Residual, accepted: a same-crate-
    # NEW-version re-entry creates a distinct-version row (a genuinely
    # distinct published artifact), not a duplicate count; cmd_merge's
    # base-collision refusal is the backstop for exact re-measures.
    base_keys = frozenset(c["name"] for c in base.get("crates", [])) | frozenset(
        c["name"] for c in base.get("compile_error_crates", [])
    )
    pages = [int(p) for p in a.pages.split(",")]
    print(f"fetching pages {pages} ...", flush=True)
    rows, rows_seen = fetch_candidates(pages, a.per_page, a.min_downloads)
    deduped, skipped_dup = [], 0
    for r in rows:
        key = f"{r['name']}-{r['ver']}"
        key = f"{r['name']}-{r['ver']}"
        if key in base_keys or any(d["name"] == r["name"] for d in deduped):
            skipped_dup += 1
            continue
        deduped.append(r)
    sample = deduped[: a.target]
    print(f"candidates(fetch)={rows_seen} dedup_skipped={skipped_dup} sampled={len(sample)}", flush=True)
    json.dump({"candidates": rows_seen, "sampled": len(sample), "pages": pages}, open(a.meta_out, "w", encoding="utf-8"))

    done: dict[str, dict] = {}
    if os.path.exists(a.out):  # resume
        for line in open(a.out, encoding="utf-8"):
            try:
                r = json.loads(line)
                done[r["name"]] = r
            except json.JSONDecodeError:
                pass
        print(f"resume: {len(done)} already measured", flush=True)
    # Row keys are exactly f"{name}-{ver}" and the candidate carries BOTH
    # parts, so the resume skip matches the CONSTRUCTED key exactly. The
    # wave-2 bug was the BARE name never matching versioned keys (re-measured
    # 122 crates, caught by the per-lint crate lists printing each crate
    # twice); the wave-3 bug was PREFIX-matching — done key gsk4-sys-0.11.4
    # startswith("gsk4-") silently skipped the gsk4 crate (caught by the
    # reconstruction diff before any number was used). Exact keys close both.
    done_keys = frozenset(done.keys())

    extract_root = os.path.join(a.scratch, "extract")
    os.makedirs(extract_root, exist_ok=True)

    def work(idx_r):
        idx, r = idx_r
        name, ver = r["name"], r["ver"]
        if f"{name}-{ver}" in done_keys:
            return None
        crate_dir = None
        try:
            path = download_crate(name, ver, os.path.join(a.scratch, "dl"))
            crate_dir = extract_crate(path, os.path.join(extract_root, f"{name}-{ver}"))
            res = run_clippy(crate_dir, os.path.join(a.scratch, f"tgt-{idx % a.workers}"), a.timeout)
            row = {
                "name": f"{name}-{ver}",
                "verdict": res["verdict"],
                "warnings": sum(res["lints"].values()),
                "exit_code": res["exit_code"],
                "stderr_tail": res["stderr_tail"],
                "lints": res["lints"],
                "wall_s": res["wall_s"],
            }
        except SystemExit as e:
            row = {"name": f"{name}-{ver}", "verdict": "fetch_error", "warnings": 0, "error": str(e)}
        except Exception as e:  # noqa: BLE001 — record, never abort the sweep
            row = {"name": f"{name}-{ver}", "verdict": "fetch_error", "warnings": 0, "error": repr(e)}
        with open(a.out, "a", encoding="utf-8") as f:
            f.write(json.dumps(row, sort_keys=True) + "\n")
            f.flush()
        n = row.get("warnings", 0)
        print(f"  [{len(done) + 1}/{len(sample)}] {row['name']}: {row['verdict']} w={n} ({row.get('wall_s', '?')}s)", flush=True)
        return row

    t0 = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.workers) as ex:
        for _ in ex.map(work, list(enumerate(sample))):
            pass
    print(f"sample done in {round((time.time() - t0) / 60, 1)} min -> {a.out}", flush=True)


def cmd_retally(a: argparse.Namespace) -> None:
    """Re-run clippy for every measured row against the WARM per-worker target
    dirs and overwrite lints/warnings/verdict with the fixed parser.

    Used when the lint tally was wrong but the builds already happened: cargo
    replays cached diagnostics on a warm target dir (verified on the
    akin-0.4.0 known-dirty control), so the re-run costs seconds, not a
    rebuild. Rows whose extract/download failed keep their original verdict
    with a `retally_error` note and are never silently dropped."""
    rows = []
    for line in open(a.results, encoding="utf-8"):
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    extract_root = os.path.join(a.scratch, "extract")
    out = []
    done = 0

    def one(idx_row):
        idx, row = idx_row
        if row.get("verdict") == "fetch_error":
            return row
        name = row["name"]
        try:
            path = os.path.join(a.scratch, "dl", f"{name}.crate")
            crate_dir = extract_crate(path, os.path.join(extract_root, name))
            res = run_clippy(crate_dir, os.path.join(a.scratch, f"tgt-{idx % a.workers}"), a.timeout)
            return {
                "name": name,
                "verdict": res["verdict"],
                "warnings": sum(res["lints"].values()),
                "exit_code": res["exit_code"],
                "stderr_tail": res["stderr_tail"],
                "lints": res["lints"],
                "wall_s": res["wall_s"],
            }
        except Exception as e:  # noqa: BLE001 — keep the original row, marked
            row2 = dict(row)
            row2["retally_error"] = repr(e)[:200]
            return row2

    with concurrent.futures.ThreadPoolExecutor(max_workers=a.workers) as ex:
        for r in ex.map(one, list(enumerate(rows))):
            out.append(r)
            done += 1
            if done % 25 == 0:
                print(f"  retally {done}/{len(rows)}", flush=True)
    with open(a.results + ".retallied", "w", encoding="utf-8") as f:
        for r in out:
            f.write(json.dumps(r, sort_keys=True) + "\n")
    errs = [r for r in out if r.get("retally_error")]
    print(f"retally done: {len(out)} rows, {len(errs)} retally_error -> {a.results}.retallied", flush=True)


def _blake3_of_file(path: str) -> str:
    try:
        import blake3  # type: ignore

        return blake3.blake3(open(path, "rb").read()).hexdigest()
    except ImportError:
        pass
    code = "import sys,blake3;print(blake3.blake3(open(sys.argv[1],'rb').read()).hexdigest())"
    proc = subprocess.run(
        ["uv", "run", "--with", "blake3", "python", "-c", code, path],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=300,
    )
    if proc.returncode != 0:
        raise SystemExit(f"blake3 unavailable (no module, uv fallback failed): {proc.stderr[-300:]}")
    return proc.stdout.strip().splitlines()[-1]


def tool_versions() -> tuple[str, str]:
    cl = subprocess.run(["cargo", "clippy", "--version"], capture_output=True, text=True, encoding="utf-8", errors="replace")
    ca = subprocess.run(["cargo", "--version"], capture_output=True, text=True, encoding="utf-8", errors="replace")
    return cl.stdout.strip(), ca.stdout.strip()


def cmd_merge(a: argparse.Namespace) -> None:
    base = json.load(open(a.base_manifest, encoding="utf-8"))
    seen: dict[str, dict] = {}
    dup_total = 0
    dup_disagree = 0
    for line in open(a.partial, encoding="utf-8"):
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue
        if r.get("verdict") == "fetch_error":
            continue
        if r["name"] in seen:
            dup_total += 1
            if seen[r["name"]].get("lints") != r.get("lints") or seen[r["name"]]["verdict"] != r["verdict"]:
                dup_disagree += 1
            continue  # keep-first: duplicate measurements never double-count
        seen[r["name"]] = r
    rows = list(seen.values())
    # The merge arithmetic (count/spread add per lint; crates concatenate) is
    # EXACT only for DISJOINT crate sets — the manifest's selection_rule says
    # so explicitly. Enforce it rather than assume it: a partial row whose
    # name already exists in the base means the sample-side cross-wave dedup
    # missed (rank drift at an adjacent-band boundary), and merging it would
    # double-count that crate's lints in every row it touches. Refuse loudly;
    # the operator re-runs `sample` against the latest accumulated manifest
    # (whose fixed dedup skips the crate at fetch time).
    base_names = frozenset(c["name"] for c in base.get("crates", [])) | frozenset(
        c["name"] for c in base.get("compile_error_crates", [])
    )
    collisions = sorted(r["name"] for r in rows if r["name"] in base_names)
    if collisions:
        raise SystemExit(
            f"merge REFUSED: {len(collisions)} partial row(s) already in the base manifest "
            f"(disjoint-set premise broken; would double-count): {', '.join(collisions[:10])}"
            f"{' …' if len(collisions) > 10 else ''} — re-run `sample --base-manifest <latest tNN>`."
        )
    if dup_total:
        print(f"dedup: {dup_total} duplicate row(s) collapsed keep-first; {dup_disagree} DISAGREED on lints/verdict", flush=True)
    ok = [r for r in rows if r["verdict"] == "ok"]
    err = [r for r in rows if r["verdict"] == "compile_error"]
    to = [r for r in rows if r["verdict"] == "timeout"]

    hist: dict[str, list[int]] = {}
    for lint, count, spread in base["default_fired_histogram"]:
        hist[lint] = [count, spread]
    for r in ok:
        for lint, n in r.get("lints", {}).items():
            if lint not in hist:
                hist[lint] = [0, 0]
            hist[lint][0] += n
            hist[lint][1] += 1
    merged = sorted(([lint, c, s] for lint, (c, s) in hist.items()), key=lambda r: (-r[1], r[0]))

    crates = base["crates"] + [
        {"name": r["name"], "verdict": r["verdict"], "warnings": r.get("warnings", 0)} for r in rows
    ]
    cerr = base["compile_error_crates"] + [
        {"name": r["name"], "verdict": r["verdict"], "exit_code": r.get("exit_code"), "stderr_tail": r.get("stderr_tail", "")}
        for r in err
    ]
    meta = json.load(open(a.meta_out, encoding="utf-8")) if os.path.exists(a.meta_out) else {}
    base_pop = base["population"]
    population = {
        "candidates": base_pop["candidates"] + meta.get("candidates", 0),
        "sampled": base_pop["sampled"] + meta.get("sampled", 0),
        "compiled_clean": base_pop["compiled_clean"] + len(ok),
        "compile_error": base_pop["compile_error"] + len(err),
        "timeout": base_pop["timeout"] + len(to),
        "warnings_total": base_pop["warnings_total"] + sum(r.get("warnings", 0) for r in ok),
        "distinct_lints": len(merged),
    }
    clippy_v, cargo_v = tool_versions()
    pages = ",".join(str(p) for p in meta.get("pages", []))
    manifest = {
        "schema_version": 1,
        "created_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S+00:00"),
        "generator": (
            f"riir-clippy scripts/sample_default_fired_histogram.py (tracked driver, "
            f"methodology + Extend protocol in .benchmarks/094_default_level_fired_histogram.md); "
            f"merged from {a.base_manifest} (all prior waves, carried byte-identical) + this "
            f"script's wave-{a.wave} sample; dataset_blake3 = blake3 over the canonical JSON "
            f"(sort_keys, compact separators) of this manifest with dataset_blake3 removed"
        ),
        "clippy_version": clippy_v or base["clippy_version"],
        "cargo_version": cargo_v or base["cargo_version"],
        "bench": base["bench"],
        "selection_rule": (
            f"{base['selection_rule']} || wave {a.wave} (this manifest): "
            f"same filters, fresh pages {pages} of the same top-10k window (adjacent rank "
            f"bands; crates already in earlier waves deduped by name and skipped — the crate "
            f"sets are disjoint by construction, which makes the merged per-lint "
            f"count/spread arithmetic exact)"
        ),
        "lint_invocation": base["lint_invocation"],
        "row_schema": base["row_schema"],
        "population": population,
        "rows": len(merged),
        "default_fired_histogram": merged,
        "crates": crates,
        "compile_error_crates": cerr,
    }
    os.makedirs(os.path.dirname(os.path.abspath(a.out_manifest)), exist_ok=True)
    tmp = a.out_manifest + ".noblake3"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(manifest, f, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    digest = _blake3_of_file(tmp)
    manifest["dataset_blake3"] = digest
    with open(a.out_manifest, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=1, ensure_ascii=False)
        f.write("\n")
    os.remove(tmp)
    print(
        f"merged manifest: {a.out_manifest}\n  population={population}\n  rows={len(merged)}\n  blake3={digest}",
        flush=True,
    )


def main() -> None:
    _out()
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("sample", help="fetch + measure one wave")
    s.add_argument("--pages", required=True, help="comma list of crates.io list-API pages (1-indexed, per_page=100)")
    s.add_argument("--target", type=int, default=200)
    s.add_argument("--per-page", type=int, default=100)
    s.add_argument("--min-downloads", type=int, default=1000)
    s.add_argument("--base-manifest", default=".benchmarks/data/094_t01/manifest.json", help="prior-waves ACCUMULATED manifest: dedup source — pass the latest tNN for every new wave")
    s.add_argument("--scratch", default=os.path.join(tempfile.gettempdir(), "b094wave"))
    s.add_argument("--workers", type=int, default=4)
    s.add_argument("--timeout", type=int, default=480, help="per-crate clippy wall cap (s)")
    s.add_argument("--out", required=True, help="partial results JSONL (resume file)")
    s.add_argument("--meta-out", required=True, help="wave meta JSON (candidates/sampled/pages)")
    s.set_defaults(fn=cmd_sample)

    m = sub.add_parser("merge", help="merge base manifest + partial JSONL into the extended manifest")
    m.add_argument("--base-manifest", default=".benchmarks/data/094_t01/manifest.json", help="prior-waves ACCUMULATED manifest — pass the latest tNN for every new wave")
    m.add_argument("--partial", required=True)
    m.add_argument("--meta-out", required=True)
    m.add_argument("--out-manifest", required=True)
    m.add_argument("--wave", type=int, default=2, help="wave number stamped into generator/selection_rule provenance")
    m.set_defaults(fn=cmd_merge)

    r = sub.add_parser("retally", help="re-parse lints for measured rows via warm-target-dir replay")
    r.add_argument("--results", required=True, help="results JSONL from `sample` (overwritten via .retallied sibling)")
    r.add_argument("--scratch", default=os.path.join(tempfile.gettempdir(), "b094wave"))
    r.add_argument("--workers", type=int, default=4)
    r.add_argument("--timeout", type=int, default=480)
    r.set_defaults(fn=cmd_retally)

    a = p.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
