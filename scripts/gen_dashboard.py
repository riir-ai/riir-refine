#!/usr/bin/env python3
"""gen_dashboard.py — one-command ops dashboard generator (Issue 044).

Collects every scattered truth source about LLM spend + healer adoption +
heal coverage into ONE self-contained HTML file (embedded CSS + JS, inline
SVG charts, zero external deps — works offline, no CDN).

Data sources (ALL optional — each degrades gracefully):
  1. .benchmarks/llm_cost_ledger.jsonl   — committed LLM spend ledger
  2. .benchmarks/score_history.jsonl     — clippy-bench score curve
  3. .refine/trajectories.kv per repo      — self-evolve adoption (via the
                                            healer binary's --dump-trajectories)
  4. git heal-commit cadence per repo    — 30d word-boundary grep (A1 gate
                                            method; 'health' must not match)
  5. CF GraphQL aiInferenceAdaptiveGroups — server/billing truth, daily
                                            neurons (wrangler OAuth token,
                                            read from the wrangler config;
                                            NEVER embedded in the HTML)
  6. coverage snapshot JSON              — unhealed-edit counts per repo
                                            (--sweep re-measures via healer
                                            dry-run; otherwise cached)
  7. CF GraphQL containersUsage/Metrics  — container sleep/awake/usage +
                                            est cost, hourly/daily/weekly
                                            (Issue 045)
  8. riir-train/.benchmarks/runs.jsonl   — training/GPU run log (T6)
  9. .benchmarks/fixer_quality_history   — fixer-arm quality history (T8)
                                            .jsonl (src/bin/fixer_regate.rs)
 10. housekeeping worker GET /latest     — LIVE staleness badge + live cap
                                            (T3/T9; fetched client-side,
                                            the only network call)

Env overrides (Issue 044, the CI workflow depends on both):
  RIIR_GIT_ROOT  — sibling-repo root (default /Users/katopz/git)
  CF_API_TOKEN   — CF API token; wins over the wrangler OAuth file

Usage:
  python3 scripts/gen_dashboard.py                  # regenerate HTML
  python3 scripts/gen_dashboard.py --sweep          # + re-measure coverage
                                                     # (slow: healer dry-run
                                                     # per repo)
  python3 scripts/gen_dashboard.py --settle         # + settle completed
                                                     # account days: append a
                                                     # settlement ledger row
                                                     # per drifting day
                                                     # (issue 057; runs before
                                                     # the data build)
  python3 scripts/gen_dashboard.py --out FILE.html  # override output path

Output default: cloudflare/dashboard/index.html
Deploy: cd cloudflare/dashboard && npx wrangler deploy
"""

from __future__ import annotations

import json
import math
import os
import re
import subprocess
import sys
import time
import urllib.request
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

# Every verdict below prints ✓ / ✗ / ⛔ and em-dashes. On a console whose
# encoding is not UTF-8 — the Windows workstation that runs these is cp874 —
# print() raises UnicodeEncodeError and this script dies with NO output at all,
# so its findings are not *unknown* but *unlooked at* (katgpt-rs Issue 804).
# backslashreplace degrades the glyph to a visible escape and keeps every ASCII
# character exact, so a verdict line stays greppable; forcing UTF-8 instead
# gives mojibake, which is the silent direction.
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(errors="backslashreplace")
    except (AttributeError, ValueError):
        pass  # not a TextIOWrapper (embedded / detached); keep old behavior

REPO_ROOT = Path(__file__).resolve().parent.parent
LEDGER = REPO_ROOT / ".benchmarks" / "llm_cost_ledger.jsonl"
SCORE_HISTORY = REPO_ROOT / ".benchmarks" / "score_history.jsonl"
COVERAGE_CACHE = REPO_ROOT / ".benchmarks" / "dashboard_coverage.json"
# Issue 065 T6 — the coverage phase gets a BOUNDED slice of the nightly regen
# budget (ops_nightly.sh gives the whole gen_dashboard run `timeout 1200`).
# Per-repo dry-run timeouts are drawn from the phase budget's REMAINDER, never
# a fixed 600s: one slow repo (riir-ai) used to be able to eat half the run
# budget, and two slow repos exhausted the whole 1200s (the 09-01/09-02
# exit-124 nights — dashboard pinned stale, no deploy).
COVERAGE_BUDGET_S = int(os.environ.get("COVERAGE_BUDGET_S", "600"))
COVERAGE_MIN_PER_REPO_S = 30
DEFAULT_OUT = REPO_ROOT / "cloudflare" / "dashboard" / "public" / "index.html"

# Mirrors scripts/self_evolve_audit.sh REPOS.
REPOS = [
    "katgpt-rs", "riir-ai", "riir-train", "riir-neuron-db", "riir-chain",
    "riir-refine", "riir-game-sdk", "riir-mmorpg-examples",
    "riir-viewbridge", "riir-dapps",
]
GIT_ROOT = Path(os.environ.get("RIIR_GIT_ROOT", "/Users/katopz/git"))

CF_ACCOUNT_ID = "7e11517c4dd4f6e9cede7da9b60d66eb"
CF_GRAPHQL = "https://api.cloudflare.com/client/v4/graphql"
# Issue 044 T3/T9 — housekeeping worker (CORS-enabled). Fetched client-side
# by the generated HTML (the ONLY network call) for the staleness badge +
# live cap reading; never contacted by this generator.
HOUSEKEEPING_BASE = "https://riir-kat-housekeeping.foxfox.workers.dev"
NEURON_USD = 0.011 / 1000.0  # $0.011 per 1k neurons (pinned 2026-08-18)
DAILY_FREE_CAP = 10_000.0

HEAL_BIN = Path.home() / ".cargo" / "bin" / "cargo-refine"
WRANGLER_TOKEN_FILE = Path.home() / "Library/Preferences/.wrangler/config/default.toml"


def log(msg: str) -> None:
    print(f"[gen-dashboard] {msg}", file=sys.stderr)


# ── 1. ledger ──────────────────────────────────────────────────────────────

def load_ledger() -> list[dict]:
    rows: list[dict] = []
    if not LEDGER.exists():
        log(f"ledger missing: {LEDGER}")
        return rows
    for line in LEDGER.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            log(f"ledger skip bad line: {line[:60]}")
    # `ts_unix` is when the row was APPENDED; `covers_unix` (optional) is the
    # usage window it describes. They differ for settlement/backfill rows, so
    # every time-axis consumer below uses the effective timestamp instead —
    # bucketing a reconciliation row by its append time credits its neurons to
    # the wrong account day (Issue 044: 2026-08-27 read 0.00 while 2026-08-28
    # carried 1,129.90 it never spent).
    for r in rows:
        r["eff_ts"] = r.get("covers_unix") or r.get("ts_unix")
    rows.sort(key=lambda r: r.get("eff_ts") or 0)
    return rows


def ledger_summary(rows: list[dict]) -> dict:
    per_model: dict[str, dict] = {}
    per_day: dict[str, dict] = {}
    total_usd_client = 0.0
    total_neurons_server = 0.0
    total_req = 0
    incidents = []
    # Issue 051 — the groq-first free tier's absorption (per-row
    # `providers.groq` block; legacy rows without the key are unaffected).
    groq = {
        "calls": 0, "ok": 0, "prompt_tokens": 0, "completion_tokens": 0,
        "usd_avoided": 0.0, "rate_limited": 0, "fallbacks": 0,
        "windows": 0,
    }
    for r in rows:
        model = r.get("model") or "?"
        cl = r.get("client") or {}
        sv = r.get("server") or {}
        n = r.get("n") or 0
        gq = (r.get("providers") or {}).get("groq") or {}
        if gq.get("enabled") or gq.get("calls") or gq.get("rate_limited"):
            groq["windows"] += 1
            groq["calls"] += gq.get("calls") or 0
            groq["ok"] += gq.get("ok") or 0
            groq["prompt_tokens"] += gq.get("prompt_tokens") or 0
            groq["completion_tokens"] += gq.get("completion_tokens") or 0
            groq["usd_avoided"] += gq.get("usd_avoided_est") or 0.0
            groq["rate_limited"] += gq.get("rate_limited") or 0
            groq["fallbacks"] += gq.get("fallbacks_to_cf") or gq.get("fallbacks") or 0
        # Issue 051 T13 — attribute a GROQ-ONLY window to the free provider,
        # not to the CF model whose id the row carries as its PRICING BASIS.
        # Without this, such a row renders as `<cf-model> 0` ($0, 0 tokens)
        # AND credits the CF model with `n` samples groq actually served —
        # the mis-render hand-reconciled in `7bdb710`, which the T9 groq-only
        # window turned from an anomaly into a routine row shape.
        #
        # Deliberately narrow. Rows that must stay on the CF model:
        #   - pure-CF / mixed windows   → client tokens > 0
        #   - settlement/backfill rows  → server neurons > 0, client 0
        #   - a CF smoke + groq window  → drain.smoke_neurons > 0
        # Only "groq did work AND CF demonstrably did none" reattributes.
        # `served_by` (emitted from T13 on) is authoritative; the token
        # heuristic is the fallback that keeps pre-T13 rows correct.
        cf_used = bool(
            (cl.get("prompt_tokens") or 0)
            or (cl.get("completion_tokens") or 0)
            or (sv.get("neurons") or 0)
            or ((r.get("drain") or {}).get("smoke_neurons") or 0)
        )
        gq_active = bool(gq.get("enabled") or gq.get("calls") or gq.get("rate_limited"))
        served_by = r.get("served_by")
        if served_by == "groq" or (served_by is None and gq_active and not cf_used):
            model = "groq:" + (gq.get("model") or "?")
        m = per_model.setdefault(model, {
            "usd": 0.0, "neurons": 0.0, "requests": 0, "n": 0,
            "prompt_tokens": 0, "completion_tokens": 0,
        })
        m["usd"] += cl.get("usd_est") or 0.0
        m["neurons"] += sv.get("neurons") or 0.0
        m["requests"] += sv.get("requests") or 0
        m["n"] += n
        m["prompt_tokens"] += cl.get("prompt_tokens") or 0
        m["completion_tokens"] += cl.get("completion_tokens") or 0
        ts = r.get("eff_ts") or r.get("ts_unix")
        if ts:
            day = datetime.fromtimestamp(ts, tz=timezone.utc).strftime("%Y-%m-%d")
            d = per_day.setdefault(day, {"usd": 0.0, "neurons": 0.0, "n": 0})
            d["usd"] += cl.get("usd_est") or 0.0
            d["neurons"] += sv.get("neurons") or 0.0
            d["n"] += n
        total_usd_client += cl.get("usd_est") or 0.0
        total_neurons_server += sv.get("neurons") or 0.0
        total_req += sv.get("requests") or 0
        note = r.get("note") or ""
        if "incident" in note.lower() or "wasted" in note.lower():
            incidents.append({
                "ts": r.get("eff_ts") or r.get("ts_unix"), "model": model,
                "note": note[:160],
                "neurons": sv.get("neurons") or 0.0,
            })
    return {
        "per_model": per_model,
        "per_day": per_day,
        "total_usd_client": total_usd_client,
        "total_neurons_server": total_neurons_server,
        "total_requests": total_req,
        "incidents": incidents,
        "groq": groq,
        "rows": rows,
    }


# ── 2. score history ───────────────────────────────────────────────────────

def load_score_history() -> list[dict]:
    jsonl_rows: list[dict] = []
    if SCORE_HISTORY.exists():
        for line in SCORE_HISTORY.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                jsonl_rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    series = store_series()
    if series is None:
        return jsonl_rows
    # Store-first merge (Issue 050 T2): dedupe by (ts, sha), store wins;
    # jsonl-only rows survive. The scores jsonl is NOT rewritten here —
    # append-time dual-write keeps it in lockstep with the store.
    scores = series.get("scores") or []
    merged: dict[tuple, dict] = {}
    for r in scores:
        merged[(r.get("ts"), r.get("sha"))] = r
    for r in jsonl_rows:
        merged.setdefault((r.get("ts"), r.get("sha")), r)
    return [merged[k] for k in sorted(merged, key=lambda kv: (str(kv[0]), str(kv[1])))]


# ── 2b. the store-first reader (Issue 050 T2) ─────────────────────────────

# One-shot memo: the corpus + score loaders both need the same payload, and
# each store_series() call opens the healer's KV store. The sentinel
# distinguishes not-yet-computed from computed-unavailable (None).
_STORE_SERIES_UNSET = object()
_STORE_SERIES_CACHE: object = _STORE_SERIES_UNSET


def store_series() -> dict | None:
    """The Warm-tier series via `cargo-refine --corpus-stats --json`.

    Runs the healer binary with RIIR_REFINE_TRAJ_ROOT=REPO_ROOT and parses
    ONLY the last JSON object on stdout (with --json the Rust side prints
    the human table to stderr, so stdout is one JSON line). Returns
    {"today":…, "corpus":[…], "scores":[…]} or None on any failure
    (missing binary, non-zero exit, bad/absent JSON, timeout) — every
    caller degrades to the jsonl readers.
    """
    global _STORE_SERIES_CACHE
    if _STORE_SERIES_CACHE is not _STORE_SERIES_UNSET:
        return _STORE_SERIES_CACHE  # type: ignore[return-value]
    result: dict | None = None
    if not HEAL_BIN.exists():
        log(f"healer binary missing: {HEAL_BIN} — jsonl fallback for corpus/score series")
    else:
        env = dict(os.environ)
        env["RIIR_REFINE_TRAJ_ROOT"] = str(REPO_ROOT)
        try:
            proc = subprocess.run(
                [str(HEAL_BIN), "--corpus-stats", "--json"],
                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60, env=env,
            )
        except subprocess.TimeoutExpired:
            log("corpus-stats timeout — jsonl fallback for corpus/score series")
        else:
            if proc.returncode != 0:
                log(f"corpus-stats exit {proc.returncode} — jsonl fallback for corpus/score series")
            else:
                obj = None
                for line in proc.stdout.splitlines():
                    line = line.strip()
                    if line.startswith("{"):
                        try:
                            obj = json.loads(line)
                        except json.JSONDecodeError:
                            obj = None
                if (isinstance(obj, dict) and isinstance(obj.get("today"), dict)
                        and isinstance(obj.get("corpus"), list)
                        and isinstance(obj.get("scores"), list)):
                    result = obj
                else:
                    log("corpus-stats: no parseable JSON payload — jsonl fallback")
    _STORE_SERIES_CACHE = result
    return result


# ── corpus size per domain (mined from git history of the sources) ─────────
# Counting is deterministic greps over the corpus files at each rev:
#   kernel_opt  rule_id:   over src/domains/kernel_opt/corpus/*.rs   → 393
#   rust_perf   rule_id:   over src/domains/rust_perf/corpus/*.rs    → 126
#   clippy      lint_name: over src/domains/clippy_lints/corpus.rs   → 40
#   sec         rule_id:   over src/domains/sec/corpus.rs            → 6
# (HEAD counts verified against the pinned batch totals 2026-08-28.)
CORPUS_DOMAINS = [
    ("kernel_opt", "src/domains/kernel_opt/corpus", "rule_id:"),
    ("rust_perf", "src/domains/rust_perf/corpus", "rule_id:"),
    ("clippy_lints", "src/domains/clippy_lints/corpus.rs", "lint_name:"),
    ("sec", "src/domains/sec/corpus.rs", "rule_id:"),
    # Docker's corpus uses the positional `rule()` shorthand constructor, so
    # there is no per-entry `rule_id:` text — the old needle counted the
    # constructor's PLUMBING (exactly 2 at every rev) and the git-mined layer
    # recorded docker:2 against 27..31 true entries since the domain landed
    # (Issue 066). `    rule(` matches exactly the 8-space entry calls.
    ("docker", "src/domains/docker/corpus.rs", "    rule("),
    # Plan 134: shader_perf uses the same positional `rule()` shorthand —
    # same needle convention (dist, Issue 085, likewise).
    ("shader_perf", "src/domains/shader_perf/corpus.rs", "    rule("),
]

# The RECORDED series (the bridge to the neuron-db Warm tier — Issue 050).
# Issue 050 T2 flipped the direction: the Warm-tier store is the source of
# truth and this file is a generated PROJECTION (rewritten store-first each
# regen; jsonl-only days survive the merge). It cannot be deleted — it is
# the only cross-clone durable record (`.refine/` is gitignored). The legacy
# append-today's-working-tree behavior remains only as the no-store
# fallback. Recorded rows win over git-mined points for overlapping days;
# git-mining backfills everything before the file existed and stays as the
# drift cross-check.
CORPUS_HISTORY_JSONL = REPO_ROOT / ".benchmarks" / "corpus_history.jsonl"


def _count_working_tree(path: str, needle: str) -> int:
    p = REPO_ROOT / path
    if p.suffix == ".rs":
        return p.read_text(encoding="utf-8").count(needle) if p.exists() else 0
    n = 0
    if p.is_dir():
        for f in sorted(p.glob("*.rs")):
            if f.name != "mod.rs":
                n += f.read_text(encoding="utf-8").count(needle)
    return n


def _count_in_rev(rev: str, path: str, needle: str) -> int:
    """Count `needle` occurrences in the corpus source(s) at `rev`.

    Anchored to REPO_ROOT (Issue 066 T4): an inherited CWD makes every git
    show return b'' -> 0, and the pre-T4 merge then "wins" those zeros over
    real store rows — a mined-layer failure indistinguishable from drift.
    """
    import subprocess
    if path.endswith(".rs"):
        r = subprocess.run(["git", "show", f"{rev}:{path}"],
                           capture_output=True, text=True, encoding="utf-8", errors="replace", cwd=REPO_ROOT)
        return r.stdout.count(needle) if r.returncode == 0 else 0
    # A directory: enumerate its .rs files at that rev (renames/splits safe).
    ls = subprocess.run(["git", "ls-tree", "-r", "--name-only", rev, path],
                        capture_output=True, text=True, encoding="utf-8", errors="replace", cwd=REPO_ROOT)
    n = 0
    for f in ls.stdout.splitlines():
        if f.endswith(".rs") and not f.endswith("mod.rs"):
            r = subprocess.run(["git", "show", f"{rev}:{f}"],
                               capture_output=True, text=True, encoding="utf-8", errors="replace", cwd=REPO_ROOT)
            if r.returncode == 0:
                n += r.stdout.count(needle)
    return n


def load_corpus_history(days: int = 35) -> list[dict]:
    """One {ts, <domain>: n} point per day, oldest-first.

    Three layers, later wins for a given day:
      1. git-mined (each rev that touched a corpus path — the backfill +
         drift cross-check; covers the whole window with zero bookkeeping),
      2. recorded rows (Issue 050 T2: the Warm-tier store is the source of
         truth and the jsonl is REWRITTEN as an exact projection of it;
         jsonl-only days survive the merge — history imported by other
         clones stays),
      3. today's working-tree counts — via the store too: --corpus-stats
         already stamped them, so the store series carries today. Only the
         no-store fallback keeps the legacy upsert-into-jsonl behavior.
    """
    import subprocess
    from datetime import date, timedelta
    since = (date.today() - timedelta(days=days)).isoformat()
    paths = [p for _, p, _ in CORPUS_DOMAINS]
    git_log = subprocess.run(
        ["git", "log", f"--since={since}", "--format=%H %cs", "--", *paths],
        capture_output=True, text=True, encoding="utf-8", errors="replace", cwd=REPO_ROOT,
    )
    revs = [tuple(l.split()) for l in git_log.stdout.splitlines() if l.strip()]
    revs.reverse()  # oldest first
    # Collapse to one point per day (last commit of the day wins).
    by_day: dict[str, dict] = {}
    for sha, day in revs:
        by_day[day] = {"ts": day, **{
            name: _count_in_rev(sha, path, needle)
            for name, path, needle in CORPUS_DOMAINS
        }}

    # Layers 2+3 — the recorded series. Store-first (Issue 050 T2); the
    # jsonl-only path below is the no-store fallback, verbatim legacy.
    recorded: dict[str, dict] = {}
    series = store_series()
    if series is not None:
        for row in series["corpus"]:
            ts = row.get("ts")
            if isinstance(ts, str):
                recorded[ts] = row
        # Union the jsonl under the store rows (store wins on overlap;
        # jsonl-only days kept). Rewrite the file as the exact projection —
        # sorted, compact separators, byte-stable when nothing changed.
        if CORPUS_HISTORY_JSONL.exists():
            for line in CORPUS_HISTORY_JSONL.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                    ts = row.get("ts")
                    if isinstance(ts, str) and ts not in recorded:
                        recorded[ts] = row
                except json.JSONDecodeError:
                    pass
        rows = [recorded[d] for d in sorted(recorded)]
        payload = "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows)
        CORPUS_HISTORY_JSONL.parent.mkdir(parents=True, exist_ok=True)
        if (not CORPUS_HISTORY_JSONL.exists()
                or CORPUS_HISTORY_JSONL.read_text(encoding="utf-8") != payload):
            CORPUS_HISTORY_JSONL.write_text(payload, encoding="utf-8")
            log(f"corpus stamp: store projection written ({len(rows)} day(s))")
    else:
        # Legacy path (no healer binary / store unavailable): jsonl +
        # today's working-tree upsert, exactly as before the store flip.
        if CORPUS_HISTORY_JSONL.exists():
            for line in CORPUS_HISTORY_JSONL.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                    if isinstance(row.get("ts"), str):
                        recorded[row["ts"]] = row
                except json.JSONDecodeError:
                    pass

        today = date.today().isoformat()
        today_row = {"ts": today, **{
            name: _count_working_tree(path, needle)
            for name, path, needle in CORPUS_DOMAINS
        }}
        if recorded.get(today) != today_row:
            recorded[today] = today_row
            rows = [recorded[d] for d in sorted(recorded)]
            CORPUS_HISTORY_JSONL.parent.mkdir(parents=True, exist_ok=True)
            CORPUS_HISTORY_JSONL.write_text(
                "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows), encoding="utf-8")
            log(f"corpus stamp: recorded {today} "
                + " ".join(f"{n}={today_row[n]}" for n, _, _ in CORPUS_DOMAINS))

    # Issue 066 T4: recorded wins, but a store-vs-mined disagreement on an
    # overlapping day IS the drift signal the mined layer exists to surface
    # (the poisoned docker 2-vs-31 week was exactly this shape, and the loss
    # was silent). Log per divergent domain — never fail, never override.
    for day in sorted(set(by_day) & set(recorded)):
        mined_row, stored_row = by_day[day], recorded[day]
        for name, _, _ in CORPUS_DOMAINS:
            m, s = mined_row.get(name), stored_row.get(name)
            if m is not None and s is not None and m != s:
                log(f"corpus drift: {name} {day} store={s} mined={m} "
                    "(recorded wins; if the stamp is the skew, repair via the "
                    "dated writer — Issue 066 precedent)")
    by_day.update(recorded)  # recorded wins over mined for overlapping days
    return [by_day[d] for d in sorted(by_day)]


# ── 3. trajectory stores per repo ──────────────────────────────────────────

def collect_trajectories() -> list[dict]:
    if not HEAL_BIN.exists():
        log(f"healer binary missing: {HEAL_BIN}")
        return []
    out = []
    for repo in REPOS:
        store = GIT_ROOT / repo / ".refine" / "trajectories.kv"
        if not store.is_dir():
            continue
        env = dict(os.environ)
        env["RIIR_REFINE_TRAJ_ROOT"] = str(GIT_ROOT / repo)
        try:
            proc = subprocess.run(
                [str(HEAL_BIN), "--dump-trajectories"],
                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60, env=env,
            )
        except subprocess.TimeoutExpired:
            log(f"{repo}: trajectory dump timeout")
            continue
        rows = []
        for line in proc.stdout.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
        if not rows:
            continue
        tiers = {"certified": 0, "heuristic": 0, "withdrawn": 0}
        succ = fail = 0
        rules: dict[str, int] = {}
        last_seen = 0
        for r in rows:
            t = r.get("tier", 0)
            tiers["certified" if t == 0 else ("heuristic" if t == 1 else "withdrawn")] += 1
            succ += r.get("success_count") or 0
            fail += r.get("fail_count") or 0
            rules[r.get("lint_key") or "?"] = rules.get(r.get("lint_key") or "?", 0) + 1
            last_seen = max(last_seen, r.get("last_seen_at") or 0)
        out.append({
            "repo": repo,
            "rows": len(rows),
            "tiers": tiers,
            "success": succ,
            "fail": fail,
            "top_rules": sorted(rules.items(), key=lambda kv: -kv[1])[:6],
            "last_seen": last_seen,
        })
    return out


# ── 4. git heal-commit cadence ─────────────────────────────────────────────

def collect_git_cadence(days: int = 30) -> list[dict]:
    out = []
    since = f"{days}.days"
    wb = re.compile(r"\bheal\b", re.IGNORECASE)
    for repo in REPOS:
        d = GIT_ROOT / repo
        if not (d / ".git").exists():
            continue
        try:
            proc = subprocess.run(
                ["git", "-C", str(d), "log", f"--since={since}", "--pretty=%s"],
                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=30,
            )
            subjects = [s for s in proc.stdout.splitlines() if s.strip()]
        except subprocess.TimeoutExpired:
            subjects = []
        heal_n = sum(1 for s in subjects if wb.search(s))
        if heal_n or subjects:
            out.append({
                "repo": repo, "heal_commits_30d": heal_n,
                "total_commits_30d": len(subjects),
            })
    out.sort(key=lambda r: -r["heal_commits_30d"])
    return out


# ── 5. CF GraphQL daily neurons ────────────────────────────────────────────

def collect_cf_daily(days: int = 14) -> list[dict]:
    token = _cf_token()
    if not token:
        log("no CF token (CF_API_TOKEN unset + no wrangler OAuth) — "
            "skipping live CF analytics")
        return []
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    start = datetime.fromtimestamp(time.time() - days * 86400,
                                   tz=timezone.utc).strftime("%Y-%m-%d")
    q = ("query { viewer { accounts(filter: {accountTag: \"%s\"}) { "
         "aiInferenceAdaptiveGroups(limit: 60, filter: {date_geq: \"%s\", "
         "date_leq: \"%s\"}, orderBy: [date_ASC]) { dimensions { date } "
         "sum { totalNeurons } } } } }" % (CF_ACCOUNT_ID, start, today))
    body = json.dumps({"query": q}).encode()
    req = urllib.request.Request(
        CF_GRAPHQL, data=body,
        headers={"Authorization": f"Bearer {token}",
                 "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            d = json.load(resp)
    except Exception as e:  # noqa: BLE001 — best-effort source
        log(f"CF GraphQL failed: {e}")
        return []
    errs = d.get("errors")
    if errs:
        log(f"CF GraphQL errors: {str(errs)[:200]}")
        return []
    try:
        groups = d["data"]["viewer"]["accounts"][0]["aiInferenceAdaptiveGroups"]
    except (KeyError, IndexError, TypeError):
        return []
    return [{"date": g["dimensions"]["date"],
             "neurons": g["sum"]["totalNeurons"] or 0.0} for g in groups]


# ── 5a. automated settlement (issue 057) ───────────────────────────────────
# The reconciliation card compares ledger-per-day against CF GraphQL billing
# truth, but nothing ever WROTE the settlement rows — they were hand-appended
# (7bdb710, 83d853c), so every completed account day drifted "100%
# unreconciled" until a human noticed, and analytics retention (~1-3 days on
# this account) silently expired the chance to fix it. This closes the loop:
# for every COMPLETE day (strictly before today UTC — the current day is
# still growing, so settling it would under-count) whose ledger attribution
# disagrees with GraphQL beyond the card's own tolerance, append ONE
# settlement row with covers_unix set to the day it settles (the Issue 044
# T12 rule — never bucket by append time).
#
# Idempotency + convergence: a settled day is skipped only while it MATCHES
# analytics — the tolerance IS the duplicate guard. A settled day that still
# disagrees gets a CORRECTION row instead of a silent skip: a one-shot
# settlement could otherwise lock an under-count in as PERMANENT drift
# (analytics lag at settle time attributes fewer neurons than final billing,
# and the skip made the day un-settleable forever). Corrections converge —
# each row moves the day's ledger total onto the then-current analytics
# value, so the next run finds |diff| <= tol and appends nothing. An
# appended-but-uncommitted row still blocks a duplicate on the next run; the
# nightly's commit merely makes it durable.
SETTLE_ARMS = {"settlement", "reconciliation"}  # "reconciliation" = the two hand-appended T12 rows


def settle_reconciliation() -> int:
    token = _cf_token()
    if not token:
        log("settlement skipped: no CF token (CF_API_TOKEN unset + no wrangler OAuth)")
        return 0
    cf_daily = collect_cf_daily()
    if not cf_daily:
        log("settlement skipped: no analytics days returned")
        return 0
    rows = load_ledger()
    led = ledger_summary(rows)
    settled_days = {
        datetime.fromtimestamp(r["eff_ts"], tz=timezone.utc).strftime("%Y-%m-%d")
        for r in rows if r.get("arm") in SETTLE_ARMS and r.get("eff_ts")
    }
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    appended = 0
    for g in cf_daily:
        day = g["date"]
        if day >= today:
            continue  # incomplete day — analytics still growing
        target = g["neurons"]
        have = (led["per_day"].get(day) or {}).get("neurons") or 0.0
        diff = target - have
        # Same tolerance as the card: committed rows round to 2dp, so exact
        # equality is not achievable — below 0.5% (floor 1 neuron) is noise.
        if abs(diff) <= max(1.0, abs(target) * 0.005):
            log(f"settlement: {day} already reconciled "
                f"(ledger {have:.2f} vs analytics {target:.2f})")
            continue
        correction = day in settled_days
        covers = int(datetime.strptime(day, "%Y-%m-%d")
                     .replace(tzinfo=timezone.utc).timestamp())
        head = (
            "settlement CORRECTION (issue 057 convergent re-settle): a prior "
            "settlement under/over-counted (analytics lag at settle time); the "
            "day is still covered by analytics, so correct rather than lock "
            "the drift in"
            if correction else
            "automated settlement (issue 057): GraphQL account-day total"
        )
        row = {
            "ts_unix": int(time.time()),
            "covers_unix": covers,
            "arm": "settlement",
            # aiInferenceAdaptiveGroups has NO model dimension (probed
            # 7bdb710) — the aggregate remainder cannot be attributed to a
            # model, so name it honestly instead of guessing.
            "model": "account/unattributed-aggregate",
            "n": 0,
            "client": None,
            "server": {
                "requests": 0,
                "input_tokens": 0,
                "output_tokens": 0,
                # The sign is carried honestly: a negative row is a
                # legitimate correction (per-run rows can over-attribute
                # from a laggy snapshot vs final billing).
                "neurons": round(diff, 2),
                "usd_list": round(diff * NEURON_USD, 7),
                "free_alloc_pct_of_daily_10k": round(diff / 10_000.0 * 100.0, 1),
            },
            "kat": None,
            "note": (f"{head} {target:.2f} n vs ledger-attributed {have:.2f} n at settle time; "
                     "aggregate remainder — aiInferenceAdaptiveGroups has NO model "
                     "dimension, per-model attribution is not derivable"),
        }
        with LEDGER.open("a", encoding="utf-8") as f:
            f.write(json.dumps(row) + "\n")
        appended += 1
        log(f"settlement: {day} appended {diff:+.2f} n "
            f"({'CORRECTION' if correction else 'settlement'}: "
            f"analytics {target:.2f} - ledger {have:.2f})")
    if not appended:
        log("settlement: nothing to settle")
    return appended


# ── 5b. containers (usage + metrics) ───────────────────────────────────────
# Issue 045: container sleep/awake/usage + est cost, hourly/daily/weekly.
# Units (empirically pinned 2026-08-26 against known instance footprints):
#   allocatedMemory / allocatedDisk = byte-seconds; cpuTimeSec = vCPU-seconds;
#   containerUptime (metrics) = milliseconds per hourly bucket.

CONT_MEM_USD_PER_GIB_S = 0.0000025   # $/GiB-s beyond 25 GiB-h/mo included
CONT_DISK_USD_PER_GB_S = 0.00000007  # $/GB-s beyond 200 GB-h/mo included
CONT_CPU_USD_PER_S = 0.000020       # $/vCPU-s beyond 375 vCPU-min/mo included

# ── the CF bill model (Issue 065) ─────────────────────────────────────────
# What the bill actually computes, per line:
#
#   billable = max(0, PERIOD_ACCOUNT_total - included)
#   usd      = ceil(billable / gran) * price          # gran > 1
#   usd      = billable * price                       # gran == 1 (continuous)
#
# Three properties of that formula are what the dashboard used to get wrong,
# and each one is load-bearing:
#
#  1. `included` is subtracted. Until 2026-09-03 every container dollar on this
#     dashboard was `usage * rate` from unit ZERO, while the card sub-text
#     claimed "beyond the Workers-Paid included allowances". Measured: $0.3780
#     against a real bill of $0.08 (4.7x), and $0.0189 of "disk cost" against a
#     disk line the bill charges $0.00 for.
#  2. The allowance is MONTHLY and ACCOUNT-WIDE. It is not per app, not per
#     namespace and not per day, so a per-app or per-day dollar figure cannot be
#     reconciled against the bill at ALL -- see `container_marginal_usd`.
#  3. `gran` rounds the remainder UP. KV is sold by the million: 301.97k
#     billable list ops cost the full $5.00 of a million, not $1.51. Pricing KV
#     linearly under-reports; this is the only line where a naive model is too
#     LOW, so it cannot be dismissed as conservative.
#
# Verified against the 2026-09-03 bill snapshot (`.benchmarks/cf_bill_pin.json`)
# to the cent on all 7 lines -- Bench 067.
BILL_LINES: dict[str, dict] = {
    "container_mem":  {"label": "container memory", "unit": "GiB-s",
                       "included": 25 * 3600, "price": CONT_MEM_USD_PER_GIB_S,
                       "gran": 1, "included_label": "25 GiB-h"},
    "container_disk": {"label": "container disk", "unit": "GB-s",
                       "included": 200 * 3600, "price": CONT_DISK_USD_PER_GB_S,
                       "gran": 1, "included_label": "200 GB-h"},
    "container_cpu":  {"label": "container vCPU", "unit": "vCPU-s",
                       "included": 375 * 60, "price": CONT_CPU_USD_PER_S,
                       "gran": 1, "included_label": "375 vCPU-min"},
    "kv_read":        {"label": "KV read ops", "unit": "ops",
                       "included": 10_000_000, "price": 0.50,
                       "gran": 1_000_000, "included_label": "10M"},
    "kv_list":        {"label": "KV list ops", "unit": "ops",
                       "included": 1_000_000, "price": 5.00,
                       "gran": 1_000_000, "included_label": "1M"},
    "kv_write":       {"label": "KV write ops", "unit": "ops",
                       "included": 1_000_000, "price": 5.00,
                       "gran": 1_000_000, "included_label": "1M"},
    "kv_delete":      {"label": "KV delete ops", "unit": "ops",
                       "included": 1_000_000, "price": 5.00,
                       "gran": 1_000_000, "included_label": "1M"},
}

# The billing period is NOT derivable with this dashboard's credential: every
# /billing, /subscriptions and namespace-list endpoint answers `10000
# Authentication error` for both CF_API_TOKEN and the wrangler OAuth token
# (analytics GraphQL is all they carry). So the anniversary day is a PIN, and
# the reconciliation reports it as one rather than implying a measurement.
#
# Measured bound (2026-09-03, Issue 065): the bill's container lines reproduce
# from the 2026-08-26 usage rows ALONE to within 0.11% and EXCLUDE the
# 2026-08-14 rows, so the period starts in 2026-08-15..2026-08-26. Every day in
# that bound gives the SAME reconciliation over the present data, because no
# usage row falls between 08-15 and 08-25 -- the pin is currently unfalsifiable,
# which is why it is labelled instead of trusted. Override once it is known:
#   CF_BILLING_ANNIVERSARY_DAY=<1..28>
CF_BILLING_ANNIVERSARY_DAY = int(
    os.environ.get("CF_BILLING_ANNIVERSARY_DAY", "16"))
BILL_PIN = REPO_ROOT / ".benchmarks" / "cf_bill_pin.json"


def _wrangler_oauth_token():
    if WRANGLER_TOKEN_FILE.exists():
        m = re.search(r'^oauth_token = "(.+)"$',
                      WRANGLER_TOKEN_FILE.read_text(encoding="utf-8"), re.MULTILINE)
        if m:
            return m.group(1)
    return None


def _cf_token():
    """Single CF token source (Issue 044): CF_API_TOKEN env wins, else the
    wrangler OAuth file. The token only ever goes in request headers."""
    tok = os.environ.get("CF_API_TOKEN")
    if tok:
        return tok
    return _wrangler_oauth_token()


def _gql(token: str, query: str):
    req = urllib.request.Request(
        CF_GRAPHQL, data=json.dumps({"query": query}).encode(),
        headers={"Authorization": f"Bearer {token}",
                 "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.load(resp)


def container_marginal_usd(mem_byte_s: float, disk_byte_s: float,
                           cpu_s: float) -> float:
    """MARGINAL cost of one usage slice: usage x rate, allowances NOT applied.

    Correct ONLY as "what this slice would add if the account month is already
    past the allowance for that resource". The slice's actual BILLED cost is
    $0.00 until the account crosses, and the crossing is a property of the whole
    account over the whole month -- so a per-app or per-day figure is not a
    billing quantity and must never be reconciled against the bill. Use
    `bill_reconciliation()` for anything that claims to match Cloudflare.
    """
    mem_gib_s = (mem_byte_s or 0.0) / (1024.0 ** 3)
    disk_gb_s = (disk_byte_s or 0.0) / 1e9
    return (mem_gib_s * CONT_MEM_USD_PER_GIB_S
            + disk_gb_s * CONT_DISK_USD_PER_GB_S
            + (cpu_s or 0.0) * CONT_CPU_USD_PER_S)


def billing_period_start(now: datetime | None = None) -> str:
    """Most recent CF_BILLING_ANNIVERSARY_DAY at or before `now` (UTC date)."""
    now = now or datetime.now(timezone.utc)
    day = max(1, min(28, CF_BILLING_ANNIVERSARY_DAY))
    if now.day >= day:
        return now.replace(day=day).strftime("%Y-%m-%d")
    prev_month_end = now.replace(day=1) - timedelta(days=1)
    return prev_month_end.replace(day=day).strftime("%Y-%m-%d")


def bill_line(key: str, used: float) -> dict:
    """One line of the CF bill from an account-wide, period-scoped total."""
    spec = BILL_LINES[key]
    billable = max(0.0, (used or 0.0) - spec["included"])
    gran = spec["gran"]
    # gran == 1 keeps the line continuous; gran > 1 is sold in whole blocks and
    # the remainder rounds UP, so a 30% block still costs a whole one.
    units = math.ceil(billable / gran) if gran > 1 else billable
    return {"key": key, "label": spec["label"], "unit": spec["unit"],
            "used": used or 0.0, "included": float(spec["included"]),
            "included_label": spec["included_label"], "billable": billable,
            "gran": gran, "billed_units": units, "price": spec["price"],
            "usd": units * spec["price"],
            "marginal_usd": (used or 0.0) * spec["price"] / (gran or 1)}


def bill_reconciliation(usage: dict[str, float], period_start: str) -> dict:
    """Account-wide, period-scoped bill. `usage` maps BILL_LINES keys to totals.

    Lines with no collected total are reported as `present: False` rather than
    as a zero -- an uncollected line and an unused one are different facts, and
    conflating them is how this dashboard reported $0.08 of container spend as
    the whole cloud bill while KV was charging $8.00 (Issue 065).
    """
    lines = [bill_line(k, usage[k]) for k in BILL_LINES if k in usage]
    missing = [k for k in BILL_LINES if k not in usage]
    pin = {}
    if BILL_PIN.exists():
        try:
            pin = json.loads(BILL_PIN.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            log("cf_bill_pin.json is not valid JSON — skipping bill cross-check")
    return {"period_start": period_start,
            "anniversary_day": CF_BILLING_ANNIVERSARY_DAY,
            "anniversary_pinned": "CF_BILLING_ANNIVERSARY_DAY" not in os.environ,
            "lines": lines,
            "usd": sum(x["usd"] for x in lines),
            "marginal_usd": sum(x["marginal_usd"] for x in lines),
            "uncollected": missing,
            "pin": pin}


def collect_containers(daily_days: int = 30, hourly_hours: int = 48) -> dict:
    """Container usage from CF GraphQL (Issue 045). Degrades to {\"daily\": [], \"hourly\": []}."""
    empty = {"daily": [], "hourly": [], "apps": []}
    token = _cf_token()
    if not token:
        log("no CF token (CF_API_TOKEN unset + no wrangler OAuth) — "
            "skipping container analytics")
        return empty
    today = datetime.now(timezone.utc)
    d_start = datetime.fromtimestamp(time.time() - daily_days * 86400,
                                     tz=timezone.utc).strftime("%Y-%m-%d")
    h_start = datetime.fromtimestamp(time.time() - hourly_hours * 3600,
                                     tz=timezone.utc).strftime("%Y-%m-%dT%H:00:00Z")
    acc = today.strftime("%Y-%m-%d")
    q_usage = ("{ viewer { accounts(filter: {accountTag: \"%s\"}) { "
               "containersUsageAdaptiveGroups(limit: 1000, filter: {date_geq: \"%s\"}, "
               "orderBy: [date_ASC]) { dimensions { applicationId date } "
               "sum { allocatedMemory allocatedDisk cpuTimeSec txBytes } } } } }"
               % (CF_ACCOUNT_ID, d_start))
    q_metrics = ("{ viewer { accounts(filter: {accountTag: \"%s\"}) { "
                 "containersMetricsAdaptiveGroups(limit: 2000, "
                 "filter: {datetimeHour_geq: \"%s\"}, orderBy: [datetimeHour_ASC]) { "
                 "dimensions { applicationId datetimeHour active } "
                 "sum { containerUptime cpuTimeSec } } } } }"
                 % (CF_ACCOUNT_ID, h_start))
    # Correlation dimensions (Issue 045 T5): applicationId alone is opaque and
    # the control plane 404s for deleted apps, so carry the identifiers that CAN
    # be joined dashboard-side — the backing DO, the deployment, the colo.
    q_meta = ("{ viewer { accounts(filter: {accountTag: \"%s\"}) { "
              "containersMetricsAdaptiveGroups(limit: 2000, "
              "filter: {date_geq: \"%s\"}) { dimensions { applicationId "
              "durableObjectId deploymentId location } } } } }"
              % (CF_ACCOUNT_ID, d_start))
    try:
        du = _gql(token, q_usage)
        dm = _gql(token, q_metrics)
    except Exception as e:  # noqa: BLE001 — best-effort source
        log(f"container GraphQL failed: {e}")
        return empty
    try:
        dmeta = _gql(token, q_meta)
    except Exception as e:  # noqa: BLE001 — enrichment only, never fatal
        log(f"container meta GraphQL failed (non-fatal): {e}")
        dmeta = {}
    for label, d in (("usage", du), ("metrics", dm)):
        if d.get("errors"):
            log(f"container GraphQL ({label}) errors: {str(d['errors'])[:160]}")
    daily: dict[str, dict[str, dict]] = {}
    apps: set[str] = set()
    try:
        rows = du["data"]["viewer"]["accounts"][0]["containersUsageAdaptiveGroups"] or []
    except (KeyError, IndexError, TypeError):
        rows = []
    for g in rows:
        app = (g["dimensions"].get("applicationId") or "?")[:8]
        date = g["dimensions"]["date"]
        s = g["sum"]
        apps.add(app)
        slot = daily.setdefault(date, {}).setdefault(app,
                                                      {"mem": 0.0, "disk": 0.0,
                                                       "cpu": 0.0, "tx": 0.0})
        slot["mem"] += s.get("allocatedMemory") or 0.0
        slot["disk"] += s.get("allocatedDisk") or 0.0
        slot["cpu"] += s.get("cpuTimeSec") or 0.0
        slot["tx"] += s.get("txBytes") or 0.0
    hourly: dict[str, dict[str, float]] = {}
    try:
        rows = dm["data"]["viewer"]["accounts"][0]["containersMetricsAdaptiveGroups"] or []
    except (KeyError, IndexError, TypeError):
        rows = []
    for g in rows:
        app = (g["dimensions"].get("applicationId") or "?")[:8]
        hour = (g["dimensions"].get("datetimeHour") or "")[:13]
        up_ms = (g["sum"].get("containerUptime") or 0.0)
        slot = hourly.setdefault(hour, {}).setdefault(app, 0.0)
        hourly[hour][app] = slot + up_ms
    # app short-key -> full id + correlation dims. The charts key off the short
    # form (stable, readable); the table shows the full id so it is addressable
    # against the API instead of being a dead 8-char stub (Issue 045 T5).
    meta: dict[str, dict] = {}
    try:
        mrows = dmeta["data"]["viewer"]["accounts"][0][
            "containersMetricsAdaptiveGroups"] or []
    except (KeyError, IndexError, TypeError):
        mrows = []
    for g in mrows:
        dims = g.get("dimensions") or {}
        full = dims.get("applicationId") or ""
        if not full:
            continue
        m = meta.setdefault(full[:8], {"id": full, "do": set(),
                                       "deployments": set(), "locations": set()})
        for key, dim in (("do", "durableObjectId"),
                         ("deployments", "deploymentId"),
                         ("locations", "location")):
            if dims.get(dim):
                m[key].add(dims[dim])
    for short in apps:
        meta.setdefault(short, {"id": short, "do": set(),
                                "deployments": set(), "locations": set()})
    meta_out = {k: {"id": v["id"], "do": sorted(v["do"]),
                    "deployments": sorted(v["deployments"]),
                    "locations": sorted(v["locations"])}
                for k, v in meta.items()}
    daily_list = [{"date": d, "apps": daily[d]} for d in sorted(daily)]
    hourly_list = [{"hour": h, "apps": hourly[h]} for h in sorted(hourly)]
    # Zero-fill continuous grids so the charts show true gaps (sleep) instead
    # of compressing sparse buckets together (Issue 045).
    if hourly_list:
        first_h = hourly_list[0]["hour"]
        try:
            t = datetime.strptime(first_h, "%Y-%m-%dT%H")
        except ValueError:
            t = None
        if t is not None:
            grid = []
            seen = {h["hour"]: h["apps"] for h in hourly_list}
            # Clamp at the current hour — zero-filling a fixed width past `now`
            # painted phantom "asleep" buckets into the future (Issue 045).
            now_h = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H")
            for _ in range(hourly_hours):
                key = t.strftime("%Y-%m-%dT%H")
                if key > now_h:
                    break
                grid.append({"hour": key, "apps": seen.get(key, {})})
                t = t + timedelta(hours=1)
            hourly_list = grid
    if daily_list:
        first_d = daily_list[0]["date"]
        try:
            t = datetime.strptime(first_d, "%Y-%m-%d")
        except ValueError:
            t = None
        if t is not None:
            seen = {h["date"]: h["apps"] for h in daily_list}
            grid = []
            now_d = datetime.now(timezone.utc).strftime("%Y-%m-%d")
            for _ in range(daily_days):
                key = t.strftime("%Y-%m-%d")
                if key > now_d:
                    break
                grid.append({"date": key, "apps": seen.get(key, {})})
                t = t + timedelta(days=1)
            daily_list = grid
    return {"daily": daily_list, "hourly": hourly_list,
            "apps": sorted(apps), "meta": meta_out,
            "rates": {"mem": CONT_MEM_USD_PER_GIB_S,
                      "disk": CONT_DISK_USD_PER_GB_S,
                      "cpu": CONT_CPU_USD_PER_S}}


# ── 5c. KV operations (the line that actually costs money) ────────────────
# Issue 065: the dashboard modelled containers ($0.08/mo) and Workers-AI
# neurons and tracked KV NOT AT ALL -- while KV list + read ops were $8.00 of
# the $8.08 bill, 99.0%. Attribution matters as much as the total: one
# namespace is 99.98% of every KV op on this account, and it is not one of
# riir-refine's (HOUSEKEEPING_KV is 1,980 reads of 15.29M).

KV_ACTION_TO_LINE = {"read": "kv_read", "list": "kv_list",
                     "write": "kv_write", "delete": "kv_delete"}


def collect_kv(days: int = 30) -> dict:
    """KV ops per namespace + action + day from CF GraphQL (Issue 065).

    Degrades to `{"daily": [], "namespaces": []}` exactly like the container
    collector, and for the same reason: no credential must ever mean a broken
    dashboard. The window is clamped to the account's GraphQL limit -- a wider
    `date_geq` is rejected wholesale with a `quota` error and yields ZERO rows,
    which is indistinguishable from an idle account (measured: 45d asks
    "cannot request a time range wider than 4w4d").
    """
    empty = {"daily": [], "namespaces": [], "totals": {}}
    token = _cf_token()
    if not token:
        log("no CF token — skipping KV analytics")
        return empty
    days = min(days, 30)
    start = (datetime.now(timezone.utc) - timedelta(days=days)).strftime("%Y-%m-%d")
    q = ("{ viewer { accounts(filter: {accountTag: \"%s\"}) { "
         "kvOperationsAdaptiveGroups(limit: 10000, filter: {date_geq: \"%s\"}, "
         "orderBy: [date_ASC]) { dimensions { namespaceId actionType date } "
         "sum { requests } } } } }" % (CF_ACCOUNT_ID, start))
    try:
        d = _gql(token, q)
    except Exception as e:  # noqa: BLE001 — best-effort source
        log(f"KV GraphQL failed: {e}")
        return empty
    if d.get("errors"):
        log(f"KV GraphQL errors: {str(d['errors'])[:200]}")
    try:
        rows = d["data"]["viewer"]["accounts"][0]["kvOperationsAdaptiveGroups"] or []
    except (KeyError, IndexError, TypeError):
        rows = []
    if not rows:
        return empty
    per_ns: dict[str, dict[str, float]] = {}
    per_day: dict[str, dict[str, float]] = {}
    totals: dict[str, float] = {}
    for g in rows:
        dims = g["dimensions"]
        act = dims.get("actionType") or "?"
        ns = dims.get("namespaceId") or "?"
        n = g["sum"].get("requests") or 0
        per_ns.setdefault(ns, {}).setdefault(act, 0.0)
        per_ns[ns][act] += n
        per_day.setdefault(dims["date"], {}).setdefault(act, 0.0)
        per_day[dims["date"]][act] += n
        line = KV_ACTION_TO_LINE.get(act)
        if line:
            totals[line] = totals.get(line, 0.0) + n
    # The op mix is the whole diagnosis: 11.6 reads per list is a
    # list-then-read-each loop, which is the shape that turns a 1M-op
    # allowance into a $5.00 line.
    namespaces = sorted(
        ({"id": ns, "ops": acts, "total": sum(acts.values())}
         for ns, acts in per_ns.items()),
        key=lambda x: -x["total"])
    return {"daily": [{"date": k, "ops": per_day[k]} for k in sorted(per_day)],
            "namespaces": namespaces, "totals": totals,
            "window_start": start}


# ── 6. coverage sweep (healer dry-run per repo; cached) ────────────────────

def coverage_snapshot(sweep: bool) -> list[dict]:
    cached = []
    if COVERAGE_CACHE.exists():
        try:
            cached = json.loads(COVERAGE_CACHE.read_text(encoding="utf-8")).get("repos", [])
        except json.JSONDecodeError:
            cached = []
    if not sweep or not HEAL_BIN.exists():
        return cached
    fresh: list[dict] = []
    measured: set[str] = set()
    phase_deadline = time.monotonic() + COVERAGE_BUDGET_S
    for repo in REPOS:
        target = None
        for cand in ("crates", "src"):
            p = GIT_ROOT / repo / cand
            if p.is_dir():
                target = str(p)
                break
        if target is None:
            continue
        remaining = phase_deadline - time.monotonic()
        if remaining < COVERAGE_MIN_PER_REPO_S:
            log(f"{repo}: coverage budget exhausted ({COVERAGE_BUDGET_S}s phase), keeping cached")
            break
        try:
            t0 = time.monotonic()
            proc = subprocess.run(
                [str(HEAL_BIN), "--fix", target],
                capture_output=True, text=True, encoding="utf-8", errors="replace",
                timeout=max(COVERAGE_MIN_PER_REPO_S, int(remaining)),
            )
        except subprocess.TimeoutExpired:
            log(f"{repo}: coverage dry-run timeout after {int(remaining)}s of the {COVERAGE_BUDGET_S}s phase")
            continue
        dur = time.monotonic() - t0
        m = re.search(r"fixed (\d+) file\(s\) / (\d+) edit\(s\)", proc.stdout)
        if m:
            fresh.append({
                "repo": repo,
                "files": int(m.group(1)),
                "edits": int(m.group(2)),
                "measured_at": datetime.now(timezone.utc).isoformat(),
            })
            measured.add(repo)
            log(f"{repo}: coverage {dur:.1f}s, {m.group(1)} file(s) / {m.group(2)} edit(s)")
        else:
            log(f"{repo}: coverage produced no counts ({dur:.1f}s)")
    # Merge, never replace: a timed-out / budget-exhausted / no-count repo
    # keeps its previous row (pre-fix, the cache write silently DROPPED any
    # repo that did not complete — data loss on every partial sweep).
    merged = fresh + [c for c in cached if c.get("repo") not in measured]
    if merged:
        COVERAGE_CACHE.write_text(json.dumps(
            {"repos": merged, "updated": datetime.now(timezone.utc).isoformat()},
            indent=1), encoding="utf-8")
        return merged
    return cached


# ── 7. training / GPU runs (riir-train; Issue 044 T6) ──────────────────────

def collect_training() -> list[dict]:
    """riir-train run log — one JSON row per training/bench run.

    Schema (a README in riir-train mirrors this — keep identical):
      row = {"ts": "2026-08-27T01:02:03Z", "run": "<plan/bench id>",
             "gpu": "m3"|"4090"|str, "steps": int?, "tokens": int?,
             "tok_per_s": float?, "gpu_hours": float?, "note": str?}
    """
    path = GIT_ROOT / "riir-train" / ".benchmarks" / "runs.jsonl"
    rows: list[dict] = []
    if not path.exists():
        return rows
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            log(f"training skip bad line: {line[:60]}")
    return rows


# ── 8. fixer-arm quality history (Issue 044 T8 surfacing) ──────────────────

def load_fixer_quality() -> list[dict]:
    """Fixer-arm quality rows from src/bin/fixer_regate.rs::append_quality_history
    (.benchmarks/fixer_quality_history.jsonl; tolerant — [] when missing).

    Row: {ts_unix, arm, model, n, em:{hits,total}, fragment_parses, merge_ok,
          control_fires, oracle_scoreable, strictly_fewer, keep_oracle,
          keep_strict, vetoed, divergent, regressions, reachable:{n,keep_strict},
          tokens:{prompt,completion}, latency_ms:{total,max}}
    """
    path = REPO_ROOT / ".benchmarks" / "fixer_quality_history.jsonl"
    rows: list[dict] = []
    if not path.exists():
        return rows
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            log(f"fixer-quality skip bad line: {line[:60]}")
    rows.sort(key=lambda r: r.get("ts_unix") or 0)
    return rows


# ── HTML ───────────────────────────────────────────────────────────────────

HTML_TEMPLATE = r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex,nofollow">
<title>riir ops dashboard</title>
<style>
:root{
  --bg:#0b0e14; --panel:#121722; --panel2:#0e1320; --line:#1f2736;
  --fg:#d7dee9; --dim:#7b8794; --accent:#4cc38a; --accent2:#69a6f9;
  --warn:#e5b567; --crit:#e0705f; --mono:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;
}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);
  font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;}
a{color:var(--accent2);text-decoration:none}
.wrap{max-width:1180px;margin:0 auto;padding:24px 20px 80px}
header{display:flex;flex-wrap:wrap;gap:10px;align-items:baseline;margin-bottom:6px}
h1{font:600 22px/1.2 var(--mono);margin:0;letter-spacing:.5px}
h1 .cur{color:var(--accent)}
.sub{color:var(--dim);font-size:12px;font-family:var(--mono)}
.badge{display:inline-block;padding:2px 8px;border-radius:10px;font:11px var(--mono);
  border:1px solid var(--line);color:var(--dim);margin-right:6px}
.badge.ok{color:var(--accent);border-color:var(--accent)}
.badge.warn{color:var(--warn);border-color:var(--warn)}
.badge.crit{color:var(--crit);border-color:var(--crit)}
.kpis{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:12px;margin:18px 0}
.kpi{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:14px 16px}
.kpi .v{font:600 24px/1.1 var(--mono);margin-top:4px}
.kpi .l{color:var(--dim);font-size:11px;text-transform:uppercase;letter-spacing:.8px}
.kpi .s{color:var(--dim);font-size:11px;margin-top:4px;font-family:var(--mono)}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(340px,1fr));gap:14px;margin:14px 0}
.card{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:16px}
.card h2{font:600 13px var(--mono);color:var(--dim);margin:0 0 12px;
  text-transform:uppercase;letter-spacing:.8px}
.card.wide{grid-column:1/-1}
svg{display:block;width:100%;height:auto}
svg text{font:10px var(--mono);fill:var(--dim)}
.axis line{stroke:var(--line)}
.bar{fill:var(--accent2)}
.bar.alt{fill:var(--accent)}
.bar.warn{fill:var(--warn)}
.bar.crit{fill:var(--crit)}
.capline{stroke:var(--crit);stroke-dasharray:5 4;stroke-width:1.2}
.line{fill:none;stroke:var(--accent);stroke-width:2}
.line.l2{stroke:var(--accent2)}
.line.l3{stroke:var(--warn)}
.legend{display:flex;gap:14px;flex-wrap:wrap;margin-top:8px;color:var(--dim);font-size:11px;font-family:var(--mono)}
.legend i{display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:5px;vertical-align:-1px}
table{width:100%;border-collapse:collapse;font:12px var(--mono)}
th{color:var(--dim);text-align:left;font-weight:500;border-bottom:1px solid var(--line);
  padding:6px 8px;white-space:nowrap}
td{border-bottom:1px solid var(--line);padding:6px 8px;vertical-align:top}
tr:hover td{background:var(--panel2)}
.num{text-align:right;white-space:nowrap}
/* Issue 065 bill-reconciliation delta cells: a drifted line must be
   visible without reading the number. */
td.ok{color:var(--accent)}td.bad{color:var(--crit);font-weight:600}
.note{color:var(--dim);max-width:420px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.missing{color:var(--dim)}
.missing li{margin:6px 0}
footer{color:var(--dim);font:11px var(--mono);margin-top:34px;border-top:1px solid var(--line);padding-top:14px}
.alertbar{background:rgba(224,112,95,.12);border:1px solid var(--crit);color:var(--crit);
  border-radius:10px;padding:10px 14px;font:12px var(--mono);margin:12px 0;display:none}
</style>
</head>
<body>
<div class="wrap">
<header>
  <h1>riir<span class="cur">·</span>ops dashboard</h1>
  <span class="sub" id="generated"></span>
  <span class="badge" id="livebadge">…</span>
</header>
<div class="sub" id="sources"></div>
<div class="alertbar" id="capalert"></div>

<div class="kpis" id="kpis"></div>

<div class="grid">
  <div class="card"><h2>daily neurons — server truth (14d, CF GraphQL)</h2>
    <div id="cfDaily"></div>
    <div class="legend"><span><i style="background:var(--accent2)"></i>neurons/day</span>
      <span><i style="background:var(--crit)"></i>10k free cap</span></div>
  </div>
  <div class="card wide"><h2>ledger ↔ analytics reconciliation (per account day)</h2>
    <div id="recon"></div>
    <div class="sub" style="margin-top:8px">the ledger is a CLIENT record; CF GraphQL is
      SERVER billing truth. A day that disagrees means unreconciled rows (per-run
      server fields read 0.0 under analytics lag) or a mis-stamped row — the
      defect class Issue 044 hit when settlement rows were bucketed by append
      time instead of the window they cover. Days present in only one source are
      shown too: analytics retention is ~1 day on this account, so older
      ledger-only days are EXPECTED, not drift.</div>
  </div>
  <div class="card"><h2>ledger spend by model (USD est, client)</h2>
    <div id="modelSpend"></div>
    <div class="sub" style="margin-top:8px">rows are per-run LEDGER HISTORY — closed bench arms included. The live
      serving default is <code>@cf/zai-org/glm-5.3-flash</code> (owner pivot 2026-08-28: no llama, no 4.7) with the
      groq free leg on <code>qwen/qwen3.8-27b</code>; every llama entry here is a finished Bench-049/051/068
      fixer-regate arm, not a serving choice.</div>
  </div>
  <div class="card"><h2>neurons by model — ledger (server)</h2>
    <div id="modelNeurons"></div>
  </div>
  <div class="card"><h2>free-tier absorption (groq-first)</h2>
    <div id="groqAbsorb" class="sub" style="margin-bottom:6px"></div>
    <div id="groqBars"></div>
    <div class="legend"><span>issue 051: the daily drain reasons on groq's free tier FIRST and only falls back to metered CF workers AI when the free limit hits — usd avoided is an ESTIMATE (free groq tokens priced at the CF fallback model's per-M rates)</span></div>
  </div>
  <div class="card"><h2>healer score-bench curve</h2>
    <div id="scoreLatest" class="sub" style="margin-bottom:6px"></div>
    <div id="scoreCurve"></div>
    <div class="legend"><span><i style="background:var(--accent)"></i>heal_rate</span>
      <span><i style="background:var(--warn)"></i>created_rate</span>
      <span><i style="background:var(--accent2)"></i>parse_safety</span></div>
    <div id="createdZoom" style="margin-top:10px"></div>
    <div class="legend"><span><i style="background:var(--warn)"></i>created_rate (own scale — the trust axis; 1.9% reads as ~0 on the 0–1 curve above)</span></div>
  </div>
  <div class="card"><h2>healer corpus — rules per domain</h2>
    <div id="corpusLatest" class="sub" style="margin-bottom:6px"></div>
    <div id="corpusBar"></div>
    <div id="corpusSparks"></div>
    <div class="legend"><span>counts mined from the git history of the corpus sources (rule_id / lint_name greps, verified against the pinned batch totals) — one point per day any corpus changed</span></div>
  </div>
</div>

<div class="grid">
  <div class="card wide"><h2>cloudflare bill — reconciliation (billing period, account-wide)</h2>
    <div id="billTable"></div>
    <div class="sub">billed = <code>ceil(max(0, period_total − included) / block) × price</code>.
      The <b>included allowance is monthly and account-wide</b>, so no per-app,
      per-namespace or per-day figure on this page is a billing quantity — those
      are <b>marginal</b> costs (what a slice adds <i>once</i> the account is past
      the allowance). KV is sold by the million and the remainder rounds
      <b>up</b>: 301.97k billable list ops cost a whole $5.00 block, not $1.51.
      Cross-checked against the transcribed bill in
      <code>.benchmarks/cf_bill_pin.json</code> (Issue 065 / Bench 067).</div>
  </div>
  <div class="card wide"><h2>KV operations — per namespace (30d)</h2>
    <div id="kvTable"></div>
    <div class="sub">The line that actually costs money: KV was <b>$8.00 of the
      $8.08</b> bill while this dashboard modelled only containers ($0.08) and
      neurons. A high read:list ratio is a list-then-read-each loop — the shape
      that turns a 1M-op allowance into a $5.00 block.</div>
  </div>
  <div class="card wide"><h2>containers — awake hours per hour (48h, per app)</h2>
    <div id="contHourly"></div>
    <div class="legend"><span><i style="background:var(--accent2)"></i>awake (uptime s in hour)</span>
      <span>gap = asleep / no data</span></div>
  </div>
  <div class="card"><h2>containers — daily MARGINAL cost (14d)</h2>
    <div id="contDaily"></div>
    <div class="legend"><span><i style="background:var(--accent2)"></i>marginal $/day ×1000</span>
      <span>not billed cost — allowances are monthly + account-wide</span></div>
  </div>
  <div class="card"><h2>containers — weekly MARGINAL cost (4w)</h2>
    <div id="contWeekly"></div>
    <div class="legend"><span><i style="background:var(--accent)"></i>marginal $/week ×1000</span></div>
  </div>
  <div class="card wide"><h2>containers — per app (30d, billing dataset)</h2>
    <div id="contTable"></div>
    <div class="sub">marginal $ = mem GiB-s×$2.5e-6 + disk GB-s×$7e-8 + vCPU-s×$2e-5,
      <b>allowances NOT subtracted</b> — they cannot be, being monthly and
      account-wide rather than per app. Read this column as usage weight, and the
      reconciliation card above as cost. Worked example: the top app's 234,298
      GB-s of disk shows a marginal $0.0164 here and is billed <b>$0.00</b>, because
      the account's 246.33k GB-s is 34% of the 720k GB-s (200 GB-h) allowance.
      The window is also a rolling 30d, <b>not</b> the billing period.</div>
  </div>
</div>

<div class="grid">
  <div class="card"><h2>self-evolve adoption — trajectories per repo</h2>
    <div id="trajTable"></div>
  </div>
  <div class="card"><h2>heal commits — last 30d (word-boundary grep)</h2>
    <div id="gitCadence"></div>
  </div>
  <div class="card wide"><h2>unhealed remainder (healer dry-run snapshot)</h2>
    <div id="coverage"></div>
    <div class="sub">re-measure: <code>python3 scripts/gen_dashboard.py --sweep</code></div>
  </div>
</div>

<div class="grid">
  <div class="card wide"><h2>training / GPU — riir-train runs (runs.jsonl)</h2>
    <div id="training"></div>
  </div>
  <div class="card wide"><h2>fixer quality — LLM arms (fixer_quality_history.jsonl)</h2>
    <div id="fixerQ"></div>
  </div>
</div>

<div class="grid">
  <div class="card wide"><h2>incident timeline (ledger)</h2>
    <div id="incidents"></div>
  </div>
  <div class="card wide"><h2>LLM cost ledger — session log</h2>
    <div id="ledgerTable"></div>
  </div>
  <div class="card wide"><h2>what is MISSING for prod grade (issue 044 backlog)</h2>
    <ul class="missing" id="missingList"></ul>
  </div>
</div>

<footer id="footer"></footer>
</div>

<script id="ds" type="application/json">__DATA__</script>
<script>
const D = JSON.parse(document.getElementById('ds').textContent);
const $ = id => document.getElementById(id);
const fmt = {
  n: x => (x==null?'—':x.toLocaleString(undefined,{maximumFractionDigits:0})),
  n1: x => (x==null?'—':x.toLocaleString(undefined,{maximumFractionDigits:0,minimumFractionDigits:0})),
  f2: x => (x==null?'—':x.toFixed(2)),
  usd: x => (x==null?'—':'$'+x.toFixed(x<0.1?4:2)),
  pct: x => (x==null?'—':(x*100).toFixed(1)+'%'),
  d: ts => new Date(ts*1000).toISOString().slice(0,16).replace('T',' '),
};
$('generated').textContent = 'generated ' + D.generated_utc + ' UTC';

// ── container MARGINAL cost helpers (issue 045; renamed issue 065) ─────────
// usage x rate with NO allowance subtraction -- deliberately, because the
// allowance is monthly and account-wide and so cannot be apportioned to an app
// or a day. Never label the output of these "cost"; the billed number comes
// from D.bill. The old name `contCost` is what let a $0.378 marginal figure be
// read as a bill of $0.08 for a week.
const contMarginal = (mem,disk,cpu) =>
  (mem||0)/1073741824*(D.containers&&D.containers.rates?D.containers.rates.mem:2.5e-6)
  + (disk||0)/1e9*(D.containers&&D.containers.rates?D.containers.rates.disk:7e-8)
  + (cpu||0)*(D.containers&&D.containers.rates?D.containers.rates.cpu:2e-5);
function contDayMarginal(dayRow){ let c=0; for(const a in dayRow.apps)
  c+=contMarginal(dayRow.apps[a].mem,dayRow.apps[a].disk,dayRow.apps[a].cpu); return c; }

// ── alert bar ──
if (D.cf_daily && D.cf_daily.length) {
  const today = D.cf_daily[D.cf_daily.length-1];
  const pctCap = today.neurons / D.cap;
  if (pctCap >= 1) {
    $('capalert').style.display='block';
    $('capalert').textContent = '⚠ CAP EXCEEDED — ' + today.date + ': '
      + fmt.n1(today.neurons) + ' / ' + fmt.n(D.cap) + ' neurons ('
      + fmt.pct(pctCap) + '). Free plan hard-fails new generations until 00:00 UTC.';
  } else if (pctCap >= 0.9) {
    $('capalert').style.display='block';
    $('capalert').textContent = '⚠ ' + fmt.pct(pctCap) + ' of daily free cap used ('
      + today.date + ').';
  }
}

// ── KPIs ──
{
  const srvTotal = (D.cf_daily||[]).reduce((a,b)=>a+b.neurons,0)
    || D.ledger.totals.server_neurons;
  const k = [
    ['server neurons (14d)', fmt.n1(srvTotal),
      fmt.usd(srvTotal * D.neuron_usd) + ' list'],
    ['ledger USD (client est)', fmt.usd(D.ledger.totals.usd_client),
      fmt.n(D.ledger.totals.requests) + ' server reqs'],
    ['incidents', String(D.ledger.incidents.length),
      D.ledger.incidents.length ? 'see ledger notes' : 'clean'],
    ['score-bench runs', String(D.score.length),
      D.score.length ? 'heal ' + fmt.pct(D.score[D.score.length-1].heal_rate) : '—'],
    ['traj rows (all repos)', fmt.n(D.traj.reduce((a,b)=>a+b.rows,0)),
      D.traj.reduce((a,b)=>a+b.tiers.certified,0) + ' certified'],
    ['heal commits 30d', fmt.n(D.cadence.reduce((a,b)=>a+b.heal_commits_30d,0)),
      'across ' + D.cadence.filter(r=>r.heal_commits_30d>0).length + ' repos'],
    ['groq tok absorbed (free)', fmt.n((D.ledger.groq||{prompt_tokens:0}).prompt_tokens
      + (D.ledger.groq||{completion_tokens:0}).completion_tokens),
      fmt.usd((D.ledger.groq||{usd_avoided:0}).usd_avoided) + ' CF avoided (est)'],
  ];
  // The BILLED number leads, because it is the only one Cloudflare charges.
  // It used to be absent entirely while a 4.7x-too-high marginal figure sat in
  // the KPI row under the label "est $" (Issue 065).
  if (D.bill && D.bill.lines && D.bill.lines.length) {
    const top = D.bill.lines.slice().sort((a,b)=>b.usd-a.usd)[0];
    k.push(['CF billed $ (period)', fmt.usd(D.bill.usd),
      'since ' + D.bill.period_start + (D.bill.anniversary_pinned ? ' · day PINNED' : '')],
      ['top billed line', top && top.usd>0 ? top.label : 'all within allowance',
       top && top.usd>0 ? fmt.usd(top.usd) + ' of ' + fmt.usd(D.bill.usd) : 'nothing billable']);
  }
  if (D.containers && D.containers.daily && D.containers.daily.length) {
    const days = D.containers.daily;
    const last24 = days.slice(-1).map(contDayMarginal).reduce((a,b)=>a+b,0);
    const week = days.slice(-7).map(contDayMarginal).reduce((a,b)=>a+b,0);
    let awakeH = 0;
    (D.containers.hourly||[]).slice(-24).forEach(h=>{ for(const a in h.apps) awakeH += h.apps[a]/3.6e6; });
    k.push(['containers MARGINAL $ (24h)', fmt.usd(last24),
      fmt.usd(week) + ' / 7d · not billed'],
      ['container awake h (24h)', awakeH.toFixed(1),
      (D.containers.apps||[]).length + ' apps seen']);
  }
  $('kpis').innerHTML = k.map(x =>
    '<div class="kpi"><div class="l">'+x[0]+'</div><div class="v">'+x[1]+
    '</div><div class="s">'+x[2]+'</div></div>').join('');
  $('sources').textContent = 'sources: '
    + D.source_flags.map(s=>s[0]+(s[1]?'✓':'✗')).join(' · ');
}

// ── chart helpers (inline SVG, no deps) ──
function svgBarChart(el, labels, values, opts={}) {
  const W=560, rowH=26, topPad=8, cap=opts.cap;
  const H = Math.max(120, topPad + values.length*rowH + 30);
  const maxV = Math.max(...values, cap||0, 1);
  // Label width sized to the longest label (svg text clips at x<0 — the
  // fixed 64px ate "clippy_lints" down to "ippy_lints"); capped so the long
  // model-name labels cannot starve the bar area.
  const lw = Math.min(170, Math.max(64,
    Math.ceil(Math.max(...labels.map(l=>String(l).length))*6.6)+14));
  const bw = W - lw - 56;
  let s = '<svg viewBox="0 0 '+W+' '+H+'" preserveAspectRatio="xMidYMid meet">';
  if (cap) {
    const cy = topPad + (1 - cap/maxV) * (values.length*rowH);
    s += '<line class="capline" x1="'+lw+'" y1="'+cy+'" x2="'+W+'" y2="'+cy+'"/>'
       + '<text x="'+(W-4)+'" y="'+(cy-3)+'" text-anchor="end">cap '+fmt.n(cap)+'</text>';
  }
  values.forEach((v,i)=>{
    const y = topPad + i*rowH;
    const w = Math.max(2, (v/maxV)*bw);
    const cls = cap && v>cap ? 'bar crit' : (cap && v>0.9*cap ? 'bar warn' : 'bar alt');
    s += '<text x="'+(lw-6)+'" y="'+(y+13)+'" text-anchor="end">'+labels[i]+'</text>'
       + '<rect class="'+cls+'" x="'+lw+'" y="'+(y+3)+'" width="'+w+'" height="'+(rowH-9)+'" rx="2">'
       + '<title>'+labels[i]+': '+fmt.n1(v)+'</title></rect>'
       + '<text x="'+(lw+w+5)+'" y="'+(y+13)+'">'+fmt.n1(v)+'</text>';
  });
  s += '</svg>';
  el.innerHTML = s;
}
function svgLineChart(el, series, labels, opts={}) {
  const W=560, H=220, padL=44, padB=26, padT=12;
  const n = Math.max(...series.map(s=>s.values.length));
  const yMax = opts.yMax!=null?opts.yMax:1;
  // Scale-aware gridline labels: a 0–1 axis reads fine at 1 decimal, but a
  // 0–0.05 axis needs 3 (toFixed(0) would print "0" on every line — the
  // exact "reads as zero" failure the created_rate zoom panel exists to fix).
  const yDec = yMax===1?1:(yMax<1?3:0);
  const x = i => padL + (n<=1?0:(i/(n-1))*(W-padL-10));
  const y = v => padT + (1 - v/yMax) * (H-padT-padB);
  let s = '<svg viewBox="0 0 '+W+' '+H+'">';
  for(let g=0; g<=4; g++){
    const gy = padT + g*(H-padT-padB)/4;
    s += '<line class="axis" x1="'+padL+'" y1="'+gy+'" x2="'+W+'" y2="'+gy+'"/>'
       + '<text x="'+(padL-5)+'" y="'+(gy+3)+'" text-anchor="end">'
       + (yMax*(1-g/4)).toFixed(yDec)+'</text>';
  }
  series.forEach(sr=>{
    let p = sr.values.map((v,i)=>x(i).toFixed(1)+','+y(v).toFixed(1)).join(' ');
    s += '<polyline class="line'+(sr.cls?' '+sr.cls:'')+'" points="'+p+'">'
       + '</polyline>';
    sr.values.forEach((v,i)=>{
      s += '<circle cx="'+x(i)+'" cy="'+y(v)+'" r="2.6" fill="'+
        (sr.color||'var(--accent)')+'"><title>'+sr.name+' '+labels[i]+': '
        + v.toFixed(sr.pct?3:1)+'</title></circle>';
    });
  });
  labels.forEach((l,i)=>{
    if(n>8 && i%Math.ceil(n/7)!==0 && i!==n-1) return;
    s += '<text x="'+x(i)+'" y="'+(H-8)+'" text-anchor="middle">'+l+'</text>';
  });
  s += '</svg>';
  el.innerHTML = s;
}

// ── charts ──
if (D.cf_daily && D.cf_daily.length) {
  svgBarChart($('cfDaily'),
    D.cf_daily.map(r=>r.date.slice(5)),
    D.cf_daily.map(r=>r.neurons),
    {cap: D.cap});
} else {
  $('cfDaily').innerHTML = '<div class="sub">no live analytics (token/lag) — ledger per-day below</div>';
  svgBarChart($('cfDaily'),
    Object.keys(D.ledger.per_day).sort().map(k=>k.slice(5)),
    Object.keys(D.ledger.per_day).sort().map(k=>D.ledger.per_day[k].neurons),
    {cap: D.cap});
}
// ── ledger ↔ analytics reconciliation (issue 044) ──
{
  const led = D.ledger.per_day || {};
  const ana = {};
  (D.cf_daily||[]).forEach(r=>{ ana[r.date] = r.neurons; });
  const days = [...new Set([...Object.keys(led), ...Object.keys(ana)])].sort().reverse();
  // Tolerance: committed reconciliation rows are rounded to 2dp, so an exact
  // match is not achievable — 0.5% (floor 1 neuron) is the drift threshold.
  const tol = (a) => Math.max(1, Math.abs(a) * 0.005);
  // The settle loop (issue 057) refuses the still-open UTC day, so the newest
  // day can read 0.00-vs-partial at generation time without being drift — its
  // settlement row lands on the NEXT nightly run (observed 2026-09-05: the
  // 18:23 UTC run published ledger 0.00 vs analytics 1232.18 as "DRIFT 100%"
  // for a day that settled green 24h later). Distinguish that timing artifact
  // (warn PENDING) from real drift (crit).
  const genDay = (D.generated_utc||'').slice(0,10);
  const settled = new Set((D.ledger.rows||[])
    .filter(r=>r.arm==='settlement'||r.arm==='reconciliation')
    .map(r=>new Date(((r.covers_unix||r.ts_unix)||0)*1000).toISOString().slice(0,10)));
  if (!days.length) {
    $('recon').innerHTML = '<div class="sub">no ledger or analytics days yet</div>';
  } else {
    let drift = 0, pending = 0;
    $('recon').innerHTML = tableHTML(
      ['account day','ledger n (client)','analytics n (server)','diff','verdict'],
      days,
      d => {
        const l = led[d] ? led[d].neurons : null, a = (d in ana) ? ana[d] : null;
        let v, cls = '';
        if (l == null || a == null) {
          v = (a == null) ? 'ledger only · outside analytics retention'
                          : 'analytics only · unrecorded session';
          if (a == null) cls = 'color:var(--dim)';
          else { cls = 'color:var(--warn)'; drift++; }
        } else if (Math.abs(l - a) <= tol(a)) {
          v = 'RECONCILED'; cls = 'color:var(--accent2)';
        } else if (genDay && d >= genDay && !settled.has(d)) {
          v = 'PENDING · day open at generation — next nightly settles';
          cls = 'color:var(--warn)'; pending++;
        } else {
          v = 'DRIFT · ' + fmt.pct(Math.abs(l - a) / (a || 1)) + ' unreconciled';
          cls = 'color:var(--crit)'; drift++;
        }
        const dv = (l != null && a != null) ? (l - a) : null;
        return '<td>'+d+'</td><td class="num">'+(l==null?'—':fmt.f2(l))+
          '</td><td class="num">'+(a==null?'—':fmt.f2(a))+
          '</td><td class="num">'+(dv==null?'—':(dv>=0?'+':'')+dv.toFixed(2))+
          '</td><td style="'+cls+'">'+v+'</td>';
      });
    if (drift) {
      $('recon').innerHTML += '<div class="sub" style="color:var(--crit);margin-top:8px">'
        + drift + ' day(s) need reconciliation — append a settlement row with '
        + 'covers_unix set to the window it settles.</div>';
    }
    if (pending) {
      $('recon').innerHTML += '<div class="sub" style="color:var(--warn);margin-top:8px">'
        + pending + ' day(s) pending settlement — the UTC day was still open at '
        + 'generation and per-run server fields read 0.0 under analytics lag; '
        + 'the next nightly appends the difference (issue 057).</div>';
    }
  }
}
{
  const models = Object.entries(D.ledger.per_model).sort((a,b)=>b[1].usd-a[1].usd);
  svgBarChart($('modelSpend'), models.map(m=>m[0].split('/').pop().slice(0,22)),
    models.map(m=>m[1].usd));
  const m2 = Object.entries(D.ledger.per_model).sort((a,b)=>b[1].neurons-a[1].neurons);
  svgBarChart($('modelNeurons'), m2.map(m=>m[0].split('/').pop().slice(0,22)),
    m2.map(m=>m[1].neurons));
}
// ── free-tier absorption (issue 051 groq-first) ──
{
  const g = (D.ledger.groq || {calls:0,ok:0,prompt_tokens:0,completion_tokens:0,
    usd_avoided:0,rate_limited:0,fallbacks:0,windows:0});
  if (g.calls || g.rate_limited || g.fallbacks) {
    $('groqAbsorb').innerHTML = '<b>' + fmt.n(g.ok) + '</b> of ' + fmt.n(g.calls)
      + ' groq calls served free across ' + fmt.n(g.windows) + ' window(s) — '
      + fmt.n(g.prompt_tokens + g.completion_tokens) + ' tokens absorbed (est '
      + fmt.usd(g.usd_avoided) + ' of CF avoided) · fell back '
      + fmt.n(g.fallbacks) + '× (rate-limited ' + fmt.n(g.rate_limited) + '×).';
    svgBarChart($('groqBars'), ['groq ok', 'fallback→cf', 'rate-limited'],
      [g.ok, g.fallbacks, g.rate_limited]);
  } else {
    $('groqAbsorb').innerHTML = 'no groq-first windows yet (issue 051) — drain runs land here';
  }
}
if (D.score.length) {
  const labels = D.score.map(r=>(r.ts||'').slice(5,10));
  // Latest values as TEXT — the numbers are the point; on a 0–1 curve a
  // created_rate of 0.019 renders ~3px above the floor and reads as 0.
  const last = D.score[D.score.length-1];
  const pct = v => (v==null?'—':(v*100).toFixed(1)+'%');
  $('scoreLatest').innerHTML = 'latest ('+(last.ts||'?')+'):'
    + ' <b>heal ' + pct(last.heal_rate) + '</b>'
    + ' <b style="color:var(--warn)">created ' + pct(last.created_rate) + '</b>'
    + ' <b style="color:var(--accent2)">safety ' + pct(last.parse_safety) + '</b>'
    + ' <b>decline ' + pct(last.decline_correctness) + '</b>';
  svgLineChart($('scoreCurve'), [
    {name:'heal_rate', values:D.score.map(r=>r.heal_rate||0), pct:true, color:'var(--accent)'},
    {name:'created_rate', values:D.score.map(r=>r.created_rate||0), pct:true, color:'var(--warn)', cls:'l3'},
    {name:'parse_safety', values:D.score.map(r=>r.parse_safety||0), pct:true, color:'var(--accent2)', cls:'l2'},
  ], labels, {yMax:1});
  // created_rate on its OWN scale — the trust axis deserves a readable line.
  const cmax = Math.max(...D.score.map(r=>r.created_rate||0), 0.01);
  const cyMax = Math.min(1, Math.ceil(cmax*20)/20); // round up to 5% steps
  svgLineChart($('createdZoom'), [
    {name:'created_rate', values:D.score.map(r=>r.created_rate||0), pct:true, color:'var(--warn)'},
  ], labels, {yMax:cyMax});
}

// ── corpus per domain (counts mined from git; the coverage-growth axis) ──
if (D.corpus && D.corpus.length) {
  const DOMAINS = __DOMAINS__;
  const lastP = D.corpus[D.corpus.length-1];
  // 7d reference: the latest point at/before now-7d (earliest if history is short).
  const cutoff = new Date(lastP.ts+'T00:00:00Z').getTime() - 7*864e5;
  let ref = D.corpus[0];
  for (const p of D.corpus) {
    if (new Date(p.ts+'T00:00:00Z').getTime() <= cutoff) ref = p; else break;
  }
  const d7 = d => lastP[d] - (ref[d]||0);
  $('corpusLatest').innerHTML = DOMAINS.map(d =>
    '<b>'+d+' '+(lastP[d]||0)+'</b>'
    + ' <span style="color:'+(d7(d)>0?'var(--accent)':'var(--dim)')+'">'
    + (d7(d)>0?'(+'+d7(d)+'/7d)':(d7(d)<0?'('+d7(d)+'/7d)':'(±0/7d)'))+'</span>'
  ).join(' · ');
  // Current totals as labeled bars (values print beside each bar — the
  // 393-vs-6 spread would otherwise repeat the reads-as-zero failure).
  svgBarChart($('corpusBar'), DOMAINS.slice(), DOMAINS.map(d=>lastP[d]||0));
  // Per-domain sparklines — own scale each (kernel_opt 370→393 and sec 6→6
  // cannot share an axis honestly).
  const colors = {kernel_opt:'var(--accent)', rust_perf:'var(--accent2)',
                  clippy_lints:'var(--warn)', sec:'var(--crit)'};
  let rows = '';
  DOMAINS.forEach(d => {
    const vals = D.corpus.map(p=>p[d]||0);
    const w=150, h=26, pad=3;
    const lo = Math.min(...vals), hi = Math.max(...vals);
    const span = (hi-lo)||1;
    const pts = vals.map((v,i)=>
      (pad + i*(w-2*pad)/Math.max(vals.length-1,1)).toFixed(1)+','
      + (pad + (1-(v-lo)/span)*(h-2*pad)).toFixed(1)).join(' ');
    // Explicit px size on the svg: the global svg{width:100%} rule stretches
    // these to the flex-row width, pushing the lo→hi label outside the card
    // (the "number outside the chart" bug).
    rows += '<div style="display:flex;align-items:center;gap:8px;margin:2px 0">'
      + '<span style="width:90px;flex:none;text-align:right;color:var(--dim)">'+d+'</span>'
      + '<svg width="'+w+'" height="'+h+'" style="flex:none;width:'+w+'px;height:'+h+'px"><polyline fill="none" stroke="'
      + colors[d]+'" stroke-width="1.5" points="'+pts+'"/></svg>'
      + '<span class="sub" style="white-space:nowrap">'+lo+' → '+hi+'</span></div>';
  });
  $('corpusSparks').innerHTML = rows;
}

// ── cloudflare bill reconciliation (issue 065) ──
// The card the dashboard was missing: what Cloudflare actually CHARGES, with
// the allowance subtracted and the block rounding applied, cross-checked
// against the transcribed bill so a model that drifts shows a delta instead of
// agreeing with itself.
if (D.bill && D.bill.lines && D.bill.lines.length) {
  const pinL = (D.bill.pin && D.bill.pin.lines) || {};
  const fmtU = (v,unit) => unit==='ops' ? fmt.n(v) : fmt.n1(v);
  const lines = D.bill.lines.slice().sort((a,b)=>b.usd-a.usd || b.billable-a.billable);
  $('billTable').innerHTML = tableHTML(
    ['line','used','included','billable','blocks','billed $','bill says','Δ'],
    lines,
    L => {
      const pin = pinL[L.key];
      // A missing pin row is reported as such, never as a zero delta: "not in
      // the snapshot" and "matches the snapshot" are different facts.
      const d = pin===undefined ? null : L.usd - pin.usd;
      const dCls = d===null ? '' : (Math.abs(d) < 0.005 ? 'ok' : 'bad');
      return '<td>'+L.label+'</td>'+
        '<td class="num">'+fmtU(L.used,L.unit)+' '+L.unit+'</td>'+
        '<td class="num">'+L.included_label+'</td>'+
        '<td class="num">'+fmtU(L.billable,L.unit)+'</td>'+
        '<td class="num">'+(L.gran>1 ? L.billed_units+' × '+fmt.usd(L.price) : '—')+'</td>'+
        '<td class="num"><b>'+fmt.usd(L.usd)+'</b></td>'+
        '<td class="num">'+(pin===undefined ? '—' : fmt.usd(pin.usd))+'</td>'+
        '<td class="num '+dCls+'">'+(d===null ? 'not pinned'
          : (Math.abs(d)<0.005 ? '✓' : (d>0?'+':'')+fmt.usd(d)))+'</td>';
    });
  const marg = D.bill.marginal_usd || 0;
  $('billTable').innerHTML +=
    '<div class="sub">billed total <b>'+fmt.usd(D.bill.usd)+'</b>'+
    (D.bill.pin && D.bill.pin.usage_cost_usd!==undefined
      ? ' · bill snapshot ('+D.bill.pin.snapshot_utc+') <b>'+fmt.usd(D.bill.pin.usage_cost_usd)+'</b>'
      : ' · no bill snapshot pinned') +
    ' · the pre-Issue-065 arithmetic (usage × rate from unit zero, no allowance, no '+
    'block rounding) applied to every line here would say '+fmt.usd(marg)+', '+
    (D.bill.usd>0?(marg/D.bill.usd).toFixed(1)+'×':'∞')+' the truth — and it '+
    'modelled only the container lines, so the figure it actually published was '+
    'the $0.08 family alone' +
    (D.bill.uncollected && D.bill.uncollected.length
      ? ' · <b>uncollected lines:</b> '+D.bill.uncollected.join(', ')+
        ' (absent from this reconciliation — NOT known to be zero)'
      : '') + '</div>';
}

// ── KV operations per namespace (issue 065) ──
if (D.kv && D.kv.namespaces && D.kv.namespaces.length) {
  const grand = D.kv.namespaces.reduce((a,n)=>a+n.total,0);
  const acts = ['read','list','write','delete'];
  $('kvTable').innerHTML = tableHTML(
    ['namespace'].concat(acts).concat(['total','share','read:list']),
    D.kv.namespaces,
    n => {
      const rd = n.ops.read||0, ls = n.ops.list||0;
      return '<td title="'+n.id+'"><code>'+n.id+'</code></td>'+
        acts.map(a=>'<td class="num">'+fmt.n(n.ops[a]||0)+'</td>').join('')+
        '<td class="num"><b>'+fmt.n(n.total)+'</b></td>'+
        '<td class="num">'+fmt.pct(grand?n.total/grand:0)+'</td>'+
        '<td class="num">'+(ls>0 ? (rd/ls).toFixed(1)+'×' : '—')+'</td>';
    });
} else {
  $('kvTable').innerHTML = '<div class="sub">no KV analytics (token/lag) — nothing read, or retention window empty</div>';
}

// ── containers (issue 045) ──
if (D.containers && (D.containers.daily.length || D.containers.hourly.length)) {
  // hourly awake chart: one line per app (uptime seconds in each hour bucket)
  if (D.containers.hourly.length) {
    const hours = D.containers.hourly;
    const labels = hours.map(h=>h.hour.slice(5));
    const series = D.containers.apps.map(app => ({
      name: app,
      values: hours.map(h => (h.apps[app]||0)/1000.0), // ms → s
    }));
    const yMax = Math.max(3600, ...series.map(s=>Math.max(...s.values,0)));
    svgLineChart($('contHourly'), series, labels, {yMax:yMax});
  } else {
    $('contHourly').innerHTML = '<div class="sub">no hourly metrics rows (asleep whole window / retention)</div>';
  }
  // daily: uptime h (derived from mem byte-s ÷ plausible GiB — instead use metrics when present;
  // billing dataset gives cost directly) → bar chart cost + line uptime
  if (D.containers.daily.length) {
    const days = D.containers.daily.slice(-14);
    const labels = days.map(d=>d.date.slice(5));
    const costs = days.map(d=>contDayMarginal(d));
    svgBarChart($('contDaily'), labels, costs.map(c=>c*1000)); // $ milli-units for visibility
    // weekly cost (4w)
    const wk = {};
    D.containers.daily.forEach(d=>{
      const dt = new Date(d.date+'T00:00:00Z');
      const mon = new Date(dt); mon.setUTCDate(dt.getUTCDate()-dt.getUTCDay()+1);
      const k = mon.toISOString().slice(0,10);
      wk[k]=(wk[k]||0)+contDayMarginal(d);
    });
    const wkeys = Object.keys(wk).sort().slice(-4);
    svgBarChart($('contWeekly'), wkeys.map(k=>k.slice(5)), wkeys.map(k=>wk[k]*1000));
    // per-app table (30d)
    const agg = {};
    D.containers.daily.forEach(d=>{ for(const a in d.apps){
      const s = agg[a] = agg[a] || {mem:0,disk:0,cpu:0,tx:0,last:d.date};
      s.mem+=d.apps[a].mem; s.disk+=d.apps[a].disk; s.cpu+=d.apps[a].cpu;
      s.tx+=d.apps[a].tx||0; s.last=d.date; }});
    const rows = Object.entries(agg).sort((a,b)=>contMarginal(b[1].mem,b[1].disk,b[1].cpu)-contMarginal(a[1].mem,a[1].disk,a[1].cpu));
    // Issue 045 T5: show the FULL applicationId (the 8-char stub is not
    // addressable against the API) plus the join keys — backing DO + colo.
    const cmeta = (D.containers.meta)||{};
    $('contTable').innerHTML = tableHTML(
      ['app id','backing DO','colo','mem GiB-s','disk GB-s','cpu vCPU-s','tx MB','marginal $ (30d)','last seen'],
      rows,
      r => { const m = cmeta[r[0]]||{};
        const dobj = (m.do&&m.do.length) ? m.do.map(x=>x.slice(0,12)).join(', ') : '—';
        const loc = (m.locations&&m.locations.length) ? m.locations.join(', ') : '—';
        return '<td title="'+(m.id||r[0])+'"><code>'+(m.id||r[0])+'</code></td>'+
        '<td title="backing durable object"><code>'+dobj+'</code></td><td>'+loc+
        '</td><td class="num">'+(r[1].mem/1073741824).toFixed(0)+
        '</td><td class="num">'+(r[1].disk/1e9).toFixed(0)+'</td><td class="num">'+
        r[1].cpu.toFixed(1)+'</td><td class="num">'+(r[1].tx/1e6).toFixed(1)+
        '</td><td class="num">'+fmt.usd(contMarginal(r[1].mem,r[1].disk,r[1].cpu))+'</td><td>'+
        r[1].last+'</td>'; });
  } else {
    $('contDaily').innerHTML = '<div class="sub">no daily usage rows</div>';
    $('contWeekly').innerHTML = '<div class="sub">no daily usage rows</div>';
    $('contTable').innerHTML = '<div class="sub">no daily usage rows</div>';
  }
} else {
  ['contHourly','contDaily','contWeekly','contTable'].forEach(id=>
    $(id).innerHTML = '<div class="sub">no container analytics (token/lag) — nothing running, or retention window empty</div>');
}

// ── tables ──
function tableHTML(headers, rows, rowFmt) {
  let s = '<table><thead><tr>' + headers.map(h=>'<th>'+h+'</th>').join('') +
          '</tr></thead><tbody>';
  rows.forEach(r=>{ s += '<tr>' + rowFmt(r) + '</tr>'; });
  return s + '</tbody></table>';
}
if (D.traj.length) {
  $('trajTable').innerHTML = tableHTML(
    ['repo','rows','cert','heur','wd','succ','fail','last','top rule'],
    D.traj,
    r => '<td>'+r.repo+'</td><td class="num">'+r.rows+'</td><td class="num">'+
      r.tiers.certified+'</td><td class="num">'+r.tiers.heuristic+
      '</td><td class="num">'+r.tiers.withdrawn+'</td><td class="num">'+
      r.success+'</td><td class="num">'+r.fail+'</td><td>'+
      (r.last_seen?new Date(r.last_seen*1000).toISOString().slice(0,10):'—')+
      '</td><td class="note">'+(r.top_rules[0]?r.top_rules[0][0]+' ×'+r.top_rules[0][1]:'—')+'</td>');
} else {
  $('trajTable').innerHTML = '<div class="sub">no trajectory stores found</div>';
}
if (D.cadence.length) {
  $('gitCadence').innerHTML = tableHTML(['repo','heal ⓗ','total 30d'],
    D.cadence.filter(r=>r.heal_commits_30d>0),
    r => '<td>'+r.repo+'</td><td class="num">'+r.heal_commits_30d+
      '</td><td class="num">'+r.total_commits_30d+'</td>');
} else {
  $('gitCadence').innerHTML = '<div class="sub">no git data</div>';
}
if (D.coverage.length) {
  $('coverage').innerHTML = tableHTML(['repo','files w/ edits','edits','measured'],
    [...D.coverage].sort((a,b)=>b.edits-a.edits),
    r => '<td>'+r.repo+'</td><td class="num">'+r.files+'</td><td class="num">'+
      r.edits+'</td><td>'+(r.measured_at||'').slice(0,10)+'</td>');
} else {
  $('coverage').innerHTML = '<div class="sub">no coverage snapshot — run with --sweep</div>';
}

// ── training / GPU (issue 044 T6) ──
{
  const rows = [...(D.training||[])].sort((a,b)=>(b.ts||'').localeCompare(a.ts||'')).slice(0,12);
  if (rows.length) {
    let html = tableHTML(['ts','run','gpu','steps','tok/s','gpu_h','note'], rows,
      r => '<td>'+esc((r.ts||'').slice(0,16).replace('T',' '))+'</td><td class="note">'+
        esc(r.run||'—')+'</td><td>'+(r.gpu||'—')+'</td><td class="num">'+
        (r.steps!=null?fmt.n(r.steps):'—')+'</td><td class="num">'+
        (r.tok_per_s!=null?r.tok_per_s.toFixed(1):'—')+'</td><td class="num">'+
        (r.gpu_hours!=null?r.gpu_hours.toFixed(1):'—')+'</td><td class="note">'+
        esc(r.note||'')+'</td>');
    const tps = rows.filter(r=>r.tok_per_s!=null);
    if (tps.length >= 2) {
      html += '<div id="trainingChart"></div>';
      $('training').innerHTML = html;
      const c = [...tps].reverse(); // oldest→newest, left→right
      svgLineChart($('trainingChart'),
        [{name:'tok/s', values:c.map(r=>r.tok_per_s), color:'var(--accent)'}],
        c.map(r=>r.run||'?'),
        {yMax:Math.max(1,...c.map(r=>r.tok_per_s))*1.15});
    } else {
      $('training').innerHTML = html;
    }
  } else {
    $('training').innerHTML = '<div class="sub">no runs logged — append rows to '
      + 'riir-train/.benchmarks/runs.jsonl (ts, run, gpu, steps, tokens, '
      + 'tok_per_s, gpu_hours, note — schema in that repo\'s runs.README.md)</div>';
  }
}

// ── fixer quality (issue 044 T8 surfacing) ──
{
  const rows = [...(D.fixer||[])].sort((a,b)=>((b.ts_unix||0)-(a.ts_unix||0))).slice(0,12);
  if (rows.length) {
    $('fixerQ').innerHTML = tableHTML(
      ['utc','arm','model','n','keep o/s','veto','div','regr','em','tokens','avg s'],
      rows,
      r => '<td>'+fmt.d(r.ts_unix)+'</td><td>'+(r.arm||'—')+'</td><td class="note">'+
        (r.model||'?').split('/').pop()+'</td><td class="num">'+(r.n!=null?r.n:'—')+
        '</td><td class="num">'+(r.keep_oracle!=null?r.keep_oracle:'—')+'/'+
        (r.keep_strict!=null?r.keep_strict:'—')+'</td><td class="num">'+
        (r.vetoed!=null?r.vetoed:'—')+'</td><td class="num">'+
        (r.divergent!=null?r.divergent:'—')+'</td><td class="num">'+
        (r.regressions!=null?r.regressions:'—')+'</td><td class="num">'+
        (r.em?(r.em.hits!=null?r.em.hits:'—')+'/'+(r.em.total!=null?r.em.total:'—'):'—')+
        '</td><td class="num">'+(r.tokens?fmt.n((r.tokens.prompt||0)+(r.tokens.completion||0)):'—')+
        '</td><td class="num">'+(r.n&&r.latency_ms&&r.latency_ms.total!=null?
          (r.latency_ms.total/1000/r.n).toFixed(1):'—')+'</td>');
  } else {
    $('fixerQ').innerHTML = '<div class="sub">no arm runs recorded yet — '
      + 'src/bin/fixer_regate.rs appends one row per arm run</div>';
  }
}

// ── incident timeline (issue 044 T10) — every ledger incident, no filter ──
if (D.ledger.incidents.length) {
  $('incidents').innerHTML = tableHTML(['utc','model','neurons','usd','note'],
    D.ledger.incidents,
    r => '<td>'+(r.ts?fmt.d(r.ts):'—')+'</td><td class="note">'+
      (r.model||'?').split('/').pop()+'</td><td class="num">'+
      (r.neurons!=null?fmt.n1(r.neurons):'—')+'</td><td class="num">'+
      (r.usd!=null?fmt.usd(r.usd):'—')+
      '</td><td class="note" style="color:var(--crit)">'+esc(r.note||'')+'</td>');
} else {
  $('incidents').innerHTML = '<div class="sub">no incidents recorded</div>';
}
{
  const rows = [...D.ledger.rows].reverse().slice(0, 40);
  $('ledgerTable').innerHTML = tableHTML(
    ['utc','arm','model','n','reqs','neurons','usd srv','usd cl','note'],
    rows,
    r => {
      const sv=r.server||{}, cl=r.client||{};
      const inc=(r.note||'').match(/incident|wasted/i);
      const ets=r.eff_ts||r.ts_unix;
      return '<td'+(r.covers_unix?' title="appended '+fmt.d(r.ts_unix)+'"':'')+'>'+
        fmt.d(ets)+(r.covers_unix?'*':'')+'</td><td>'+(r.arm||'—')+'</td><td class="note">'+
        (r.model||'?').split('/').pop()+'</td><td class="num">'+(r.n||0)+
        '</td><td class="num">'+(sv.requests||0)+'</td><td class="num">'+
        fmt.n1(sv.neurons||0)+'</td><td class="num">'+fmt.usd((sv.neurons||0)*D.neuron_usd)+
        '</td><td class="num">'+fmt.usd(cl.usd_est||0)+'</td><td class="note"'+
        (inc?' style="color:var(--crit)"':'')+'>'+esc(r.note||'')+'</td>';
    });
}
function esc(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}
$('missingList').innerHTML = D.missing.map(m=>'<li><b>'+m[0]+'</b> — '+m[1]+'</li>').join('');
$('footer').textContent = 'riir ops dashboard · generated by scripts/gen_dashboard.py (riir-refine issue 044) · '
  + 'single self-contained file, no external deps · regenerate + deploy: gen → wrangler deploy';

// ── live mode (issue 044 T3/T9) — housekeeping /latest, the only network call ──
(async () => {
  const stat = txt => { try { $('livebadge').className='badge';
    $('livebadge').textContent=txt; } catch(e){} };
  try {
    const opt = (typeof AbortSignal!=='undefined' && AbortSignal.timeout)
      ? {signal: AbortSignal.timeout(8000)} : {};
    const j = await (await fetch(D.hk_base + '/latest', opt)).json();
    if (j && j.ts) {
      const t = Date.parse(j.ts);
      if (!isNaN(t)) {
        const age = Date.now() - t;
        const hm = new Date(t).toISOString().slice(11,16);
        if (age <= 2*3600e3) {
          $('livebadge').className='badge ok';
          $('livebadge').textContent='live · cron ' + hm + ' UTC';
        } else {
          $('livebadge').className='badge crit';
          $('livebadge').textContent='housekeeping stale · last run ' + hm + ' UTC';
        }
      } else { stat('static snapshot'); }
      // live cap alert when analytics were empty at generation time (T9)
      try {
        const nt = j.ai && j.ai.neurons_today;
        if (nt!=null && !(D.cf_daily && D.cf_daily.length) && nt/D.cap >= 0.9) {
          $('capalert').style.display='block';
          $('capalert').textContent = '⚠ ' + fmt.pct(nt/D.cap)
            + ' of daily free cap used (' + String(j.ts).slice(0,10) + ', live).';
        }
      } catch(e){}
    } else { stat('static snapshot'); }
  } catch (e) { stat('static snapshot'); }
})().catch(()=>{});
</script>
</body>
</html>
"""

MISSING_FOR_PROD = [
    ["owner secrets (one-time)", "CF_API_TOKEN → housekeeping collector + "
     "nightly regen analytics; DASHBOARD_TOKEN → dashboard auth gate "
     "(fails OPEN — aggregates-only HTML — until set); ALERT_WEBHOOK_URL "
     "(+ optional MONTHLY_BUDGET_USD) → budget paging. Each degrades "
     "silently until set."],
    ["auto-regeneration", "live badge reads the housekeeping worker; full "
     "regen is nightly once repo var OPS_DASHBOARD_ENABLED=true + secrets "
     "GH_PAT / CF_API_TOKEN / CLOUDFLARE_API_TOKEN are set "
     "(.github/workflows/ops_dashboard.yml) — else manual: "
     "python3 scripts/gen_dashboard.py --sweep && wrangler deploy"],
    ["KAT rail", "ledger rows carry kat:null — per-fix cost in KAT tokens "
     "per Proposal 005 Phase 1; wire when the rail lands"],
    ["training axis adoption", "collector + card live; rows appear once "
     "riir-train appends .benchmarks/runs.jsonl (schema in that repo)"],
    ["retention", "CF GraphQL served ~14d here; the committed ledger is "
     "the archive — keep appending, never rewrite"],
]


def build_data(sweep: bool) -> dict:
    ledger_rows = load_ledger()
    led = ledger_summary(ledger_rows)
    score = load_score_history()
    traj = collect_trajectories()
    cadence = collect_git_cadence()
    cf_daily = collect_cf_daily()
    containers = collect_containers()
    kv = collect_kv()
    # The bill is account-wide and period-scoped, so it is assembled ONCE from
    # every collected line rather than per-card. Container totals are scoped to
    # the same period as KV -- the 30d rolling window used by the cards is NOT
    # the billing period, and mixing the two is what left a 10% residual when
    # this was first reconciled by hand (Bench 067).
    period_start = billing_period_start()
    bill_usage: dict[str, float] = dict(kv.get("totals") or {})
    cont_period = {"mem": 0.0, "disk": 0.0, "cpu": 0.0}
    for row in containers["daily"]:
        if row["date"] < period_start:
            continue
        for app in row["apps"].values():
            for k in cont_period:
                cont_period[k] += app.get(k) or 0.0
    if containers["daily"]:
        bill_usage["container_mem"] = cont_period["mem"] / (1024.0 ** 3)
        bill_usage["container_disk"] = cont_period["disk"] / 1e9
        bill_usage["container_cpu"] = cont_period["cpu"]
    bill = bill_reconciliation(bill_usage, period_start)
    coverage = coverage_snapshot(sweep)
    training = collect_training()
    fixer = load_fixer_quality()
    corpus = load_corpus_history()
    data = {
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M"),
        "neuron_usd": NEURON_USD,
        "cap": DAILY_FREE_CAP,
        "ledger": {
            "rows": ledger_rows,
            "per_model": led["per_model"],
            "per_day": led["per_day"],
            "incidents": led["incidents"],
            "groq": led["groq"],
            "totals": {
                "usd_client": round(led["total_usd_client"], 6),
                "server_neurons": round(led["total_neurons_server"], 2),
                "requests": led["total_requests"],
            },
        },
        "score": score,
        "corpus": corpus,
        "traj": traj,
        "cadence": cadence,
        "cf_daily": cf_daily,
        "containers": containers,
        "kv": kv,
        "bill": bill,
        "coverage": coverage,
        "training": training,
        "fixer": fixer,
        "hk_base": HOUSEKEEPING_BASE,
        "missing": MISSING_FOR_PROD,
        "source_flags": [
            ["ledger", bool(ledger_rows)],
            ["score-history", bool(score)],
            ["trajectories", bool(traj)],
            ["git-cadence", bool(cadence)],
            ["cf-analytics", bool(cf_daily)],
            ["containers", bool(containers["daily"]) or bool(containers["hourly"])],
            ["kv-ops", bool(kv["namespaces"])],
            ["bill-recon", bool(bill["lines"])],
            ["coverage", bool(coverage)],
            ["training", bool(training)],
            ["fixer-quality", bool(fixer)],
        ],
    }
    return data


def main() -> int:
    sweep = "--sweep" in sys.argv
    # (The `--corpus-json` front-page snapshot lane was REMOVED 2026-09-13 —
    # riir-dapps Plan 026 made the ai.gist.rs corpus card chain-native: the
    # mining DO's own rows aggregated by the crank's corpus-view leg. The
    # committed-JSON refresh step died with the consumer; the dashboard's
    # LOCAL corpus card still uses load_corpus_history below.)
    if "--settle" in sys.argv:
        # BEFORE build_data: the freshly generated dashboard must show the
        # post-settlement state, not one run behind.
        settle_reconciliation()
    out = DEFAULT_OUT
    if "--out" in sys.argv:
        out = Path(sys.argv[sys.argv.index("--out") + 1])
    data = build_data(sweep)
    out.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(data, separators=(",", ":"))
    # Never embed anything but aggregated data — assert no token leaked.
    assert "oauth_token" not in payload and "Bearer " not in payload
    # DOMAINS is emitted FROM CORPUS_DOMAINS, never re-typed. The list used
    # to be duplicated as a JS literal here, so adding a domain (docker,
    # 2026-08-28) mined its counts into the series while the renderer
    # silently dropped them — the coverage card under-reported by a whole
    # domain with nothing failing anywhere.
    html = HTML_TEMPLATE.replace("__DATA__", payload).replace(
        "__DOMAINS__", json.dumps([n for n, _, _ in CORPUS_DOMAINS])
    )
    out.write_text(html, encoding="utf-8")
    log(f"wrote {out} ({len(html)//1024} KiB, "
        f"ledger={len(data['ledger']['rows'])} rows, "
        f"cf_days={len(data['cf_daily'])}, traj_repos={len(data['traj'])}, "
        f"coverage={len(data['coverage'])}, training={len(data['training'])}, "
        f"fixer={len(data['fixer'])}, corpus_days={len(data['corpus'])}, "
        f"kv_ns={len(data['kv']['namespaces'])}, "
        f"bill=${data['bill']['usd']:.2f} over {len(data['bill']['lines'])} lines "
        f"since {data['bill']['period_start']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
