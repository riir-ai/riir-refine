#!/usr/bin/env python3
"""gen_dashboard.py — one-command ops dashboard generator (Issue 044).

Collects every scattered truth source about LLM spend + healer adoption +
heal coverage into ONE self-contained HTML file (embedded CSS + JS, inline
SVG charts, zero external deps — works offline, no CDN).

Data sources (ALL optional — each degrades gracefully):
  1. .benchmarks/llm_cost_ledger.jsonl   — committed LLM spend ledger
  2. .benchmarks/score_history.jsonl     — clippy-bench score curve
  3. .heal/trajectories.kv per repo      — self-evolve adoption (via the
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
  python3 scripts/gen_dashboard.py --out FILE.html  # override output path

Output default: cloudflare/dashboard/index.html
Deploy: cd cloudflare/dashboard && npx wrangler deploy
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LEDGER = REPO_ROOT / ".benchmarks" / "llm_cost_ledger.jsonl"
SCORE_HISTORY = REPO_ROOT / ".benchmarks" / "score_history.jsonl"
COVERAGE_CACHE = REPO_ROOT / ".benchmarks" / "dashboard_coverage.json"
DEFAULT_OUT = REPO_ROOT / "cloudflare" / "dashboard" / "public" / "index.html"

# Mirrors scripts/self_evolve_audit.sh REPOS.
REPOS = [
    "katgpt-rs", "riir-ai", "riir-train", "riir-neuron-db", "riir-chain",
    "riir-clippy", "riir-game-sdk", "riir-mmorpg-examples", "riir-unity",
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

HEAL_BIN = Path.home() / ".cargo" / "bin" / "cargo-heal"
WRANGLER_TOKEN_FILE = Path.home() / "Library/Preferences/.wrangler/config/default.toml"


def log(msg: str) -> None:
    print(f"[gen-dashboard] {msg}", file=sys.stderr)


# ── 1. ledger ──────────────────────────────────────────────────────────────

def load_ledger() -> list[dict]:
    rows: list[dict] = []
    if not LEDGER.exists():
        log(f"ledger missing: {LEDGER}")
        return rows
    for line in LEDGER.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            log(f"ledger skip bad line: {line[:60]}")
    rows.sort(key=lambda r: r.get("ts_unix") or 0)
    return rows


def ledger_summary(rows: list[dict]) -> dict:
    per_model: dict[str, dict] = {}
    per_day: dict[str, dict] = {}
    total_usd_client = 0.0
    total_neurons_server = 0.0
    total_req = 0
    incidents = []
    for r in rows:
        model = r.get("model") or "?"
        cl = r.get("client") or {}
        sv = r.get("server") or {}
        n = r.get("n") or 0
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
        ts = r.get("ts_unix")
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
                "ts": r.get("ts_unix"), "model": model,
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
        "rows": rows,
    }


# ── 2. score history ───────────────────────────────────────────────────────

def load_score_history() -> list[dict]:
    if not SCORE_HISTORY.exists():
        return []
    out = []
    for line in SCORE_HISTORY.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return out


# ── 3. trajectory stores per repo ──────────────────────────────────────────

def collect_trajectories() -> list[dict]:
    if not HEAL_BIN.exists():
        log(f"healer binary missing: {HEAL_BIN}")
        return []
    out = []
    for repo in REPOS:
        store = GIT_ROOT / repo / ".heal" / "trajectories.kv"
        if not store.is_dir():
            continue
        env = dict(os.environ)
        env["RIIR_HEAL_TRAJ_ROOT"] = str(GIT_ROOT / repo)
        try:
            proc = subprocess.run(
                [str(HEAL_BIN), "--dump-trajectories"],
                capture_output=True, text=True, timeout=60, env=env,
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
                capture_output=True, text=True, timeout=30,
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


# ── 5b. containers (usage + metrics) ───────────────────────────────────────
# Issue 045: container sleep/awake/usage + est cost, hourly/daily/weekly.
# Units (empirically pinned 2026-08-26 against known instance footprints):
#   allocatedMemory / allocatedDisk = byte-seconds; cpuTimeSec = vCPU-seconds;
#   containerUptime (metrics) = milliseconds per hourly bucket.

CONT_MEM_USD_PER_GIB_S = 0.0000025   # $/GiB-s beyond 25 GiB-h/mo included
CONT_DISK_USD_PER_GB_S = 0.00000007  # $/GB-s beyond 200 GB-h/mo included
CONT_CPU_USD_PER_S = 0.000020       # $/vCPU-s beyond 375 vCPU-min/mo included


def _wrangler_oauth_token():
    if WRANGLER_TOKEN_FILE.exists():
        m = re.search(r'^oauth_token = "(.+)"$',
                      WRANGLER_TOKEN_FILE.read_text(), re.MULTILINE)
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


def container_cost_usd(mem_byte_s: float, disk_byte_s: float, cpu_s: float) -> float:
    """Est cost from billing-dataset sums (Issue 045 pinned constants)."""
    mem_gib_s = (mem_byte_s or 0.0) / (1024.0 ** 3)
    disk_gb_s = (disk_byte_s or 0.0) / 1e9
    return (mem_gib_s * CONT_MEM_USD_PER_GIB_S
            + disk_gb_s * CONT_DISK_USD_PER_GB_S
            + (cpu_s or 0.0) * CONT_CPU_USD_PER_S)


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
    try:
        du = _gql(token, q_usage)
        dm = _gql(token, q_metrics)
    except Exception as e:  # noqa: BLE001 — best-effort source
        log(f"container GraphQL failed: {e}")
        return empty
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
            for i in range(hourly_hours):
                key = t.strftime("%Y-%m-%dT%H")
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
            for i in range(daily_days):
                key = t.strftime("%Y-%m-%d")
                grid.append({"date": key, "apps": seen.get(key, {})})
                t = t + timedelta(days=1)
            daily_list = grid
    return {"daily": daily_list, "hourly": hourly_list,
            "apps": sorted(apps),
            "rates": {"mem": CONT_MEM_USD_PER_GIB_S,
                      "disk": CONT_DISK_USD_PER_GB_S,
                      "cpu": CONT_CPU_USD_PER_S}}


# ── 6. coverage sweep (healer dry-run per repo; cached) ────────────────────

def coverage_snapshot(sweep: bool) -> list[dict]:
    cached = []
    if COVERAGE_CACHE.exists():
        try:
            cached = json.loads(COVERAGE_CACHE.read_text()).get("repos", [])
        except json.JSONDecodeError:
            cached = []
    if not sweep or not HEAL_BIN.exists():
        return cached
    fresh = []
    for repo in REPOS:
        target = None
        for cand in ("crates", "src"):
            p = GIT_ROOT / repo / cand
            if p.is_dir():
                target = str(p)
                break
        if target is None:
            continue
        try:
            proc = subprocess.run(
                [str(HEAL_BIN), "--fix", target],
                capture_output=True, text=True, timeout=600,
            )
        except subprocess.TimeoutExpired:
            log(f"{repo}: coverage dry-run timeout")
            continue
        m = re.search(r"fixed (\d+) file\(s\) / (\d+) edit\(s\)", proc.stdout)
        if m:
            fresh.append({
                "repo": repo,
                "files": int(m.group(1)),
                "edits": int(m.group(2)),
                "measured_at": datetime.now(timezone.utc).isoformat(),
            })
    if fresh:
        COVERAGE_CACHE.write_text(json.dumps(
            {"repos": fresh, "updated": datetime.now(timezone.utc).isoformat()},
            indent=1))
        return fresh
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
    for line in path.read_text().splitlines():
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
    for line in path.read_text().splitlines():
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
  <div class="card"><h2>ledger spend by model (USD est, client)</h2>
    <div id="modelSpend"></div>
  </div>
  <div class="card"><h2>neurons by model — ledger (server)</h2>
    <div id="modelNeurons"></div>
  </div>
  <div class="card"><h2>healer score-bench curve</h2>
    <div id="scoreCurve"></div>
    <div class="legend"><span><i style="background:var(--accent)"></i>heal_rate</span>
      <span><i style="background:var(--warn)"></i>created_rate</span>
      <span><i style="background:var(--accent2)"></i>parse_safety</span></div>
  </div>
</div>

<div class="grid">
  <div class="card wide"><h2>containers — awake hours per hour (48h, per app)</h2>
    <div id="contHourly"></div>
    <div class="legend"><span><i style="background:var(--accent2)"></i>awake (uptime s in hour)</span>
      <span>gap = asleep / no data</span></div>
  </div>
  <div class="card"><h2>containers — daily est cost (14d)</h2>
    <div id="contDaily"></div>
    <div class="legend"><span><i style="background:var(--accent2)"></i>est $/day ×1000</span>
      <span>Workers-Paid allowances not subtracted</span></div>
  </div>
  <div class="card"><h2>containers — weekly est cost (4w)</h2>
    <div id="contWeekly"></div>
    <div class="legend"><span><i style="background:var(--accent)"></i>est $/week ×1000</span></div>
  </div>
  <div class="card wide"><h2>containers — per app (30d, billing dataset)</h2>
    <div id="contTable"></div>
    <div class="sub">cost = mem GiB-s×$2.5e-6 + disk GB-s×$7e-8 + vCPU-s×$2e-5 — beyond the
      Workers-Paid included allowances (25 GiB-h mem / 375 vCPU-min / 200 GB-h disk per month)</div>
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

// ── container cost helpers (issue 045; used by KPIs + containers section) ──
const contCost = (mem,disk,cpu) =>
  (mem||0)/1073741824*(D.containers&&D.containers.rates?D.containers.rates.mem:2.5e-6)
  + (disk||0)/1e9*(D.containers&&D.containers.rates?D.containers.rates.disk:7e-8)
  + (cpu||0)*(D.containers&&D.containers.rates?D.containers.rates.cpu:2e-5);
function contDayCost(dayRow){ let c=0; for(const a in dayRow.apps)
  c+=contCost(dayRow.apps[a].mem,dayRow.apps[a].disk,dayRow.apps[a].cpu); return c; }

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
  ];
  if (D.containers && D.containers.daily && D.containers.daily.length) {
    const days = D.containers.daily;
    const last24 = days.slice(-1).map(contDayCost).reduce((a,b)=>a+b,0);
    const week = days.slice(-7).map(contDayCost).reduce((a,b)=>a+b,0);
    let awakeH = 0;
    (D.containers.hourly||[]).slice(-24).forEach(h=>{ for(const a in h.apps) awakeH += h.apps[a]/3.6e6; });
    k.push(['containers est $ (24h)', fmt.usd(last24),
      fmt.usd(week) + ' / 7d · est'],
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
  const lw = 64; // label width
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
       + '<rect class="'+cls+'" x="'+lw+'" y="'+y+3+'" width="'+w+'" height="'+(rowH-9)+'" rx="2">'
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
  const x = i => padL + (n<=1?0:(i/(n-1))*(W-padL-10));
  const y = v => padT + (1 - v/yMax) * (H-padT-padB);
  let s = '<svg viewBox="0 0 '+W+' '+H+'">';
  for(let g=0; g<=4; g++){
    const gy = padT + g*(H-padT-padB)/4;
    s += '<line class="axis" x1="'+padL+'" y1="'+gy+'" x2="'+W+'" y2="'+gy+'"/>'
       + '<text x="'+(padL-5)+'" y="'+(gy+3)+'" text-anchor="end">'
       + (yMax*(1-g/4)).toFixed(yMax===1?1:0)+'</text>';
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
    Object.keys(D.ledger.per_day).map(k=>k.slice(5)),
    Object.values(D.ledger.per_day).map(v=>v.neurons),
    {cap: D.cap});
}
{
  const models = Object.entries(D.ledger.per_model).sort((a,b)=>b[1].usd-a[1].usd);
  svgBarChart($('modelSpend'), models.map(m=>m[0].split('/').pop().slice(0,22)),
    models.map(m=>m[1].usd));
  const m2 = Object.entries(D.ledger.per_model).sort((a,b)=>b[1].neurons-a[1].neurons);
  svgBarChart($('modelNeurons'), m2.map(m=>m[0].split('/').pop().slice(0,22)),
    m2.map(m=>m[1].neurons));
}
if (D.score.length) {
  const labels = D.score.map(r=>(r.ts||'').slice(5,10));
  svgLineChart($('scoreCurve'), [
    {name:'heal_rate', values:D.score.map(r=>r.heal_rate||0), pct:true, color:'var(--accent)'},
    {name:'created_rate', values:D.score.map(r=>r.created_rate||0), pct:true, color:'var(--warn)', cls:'l3'},
    {name:'parse_safety', values:D.score.map(r=>r.parse_safety||0), pct:true, color:'var(--accent2)', cls:'l2'},
  ], labels, {yMax:1});
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
    const costs = days.map(d=>contDayCost(d));
    svgBarChart($('contDaily'), labels, costs.map(c=>c*1000)); // $ milli-units for visibility
    // weekly cost (4w)
    const wk = {};
    D.containers.daily.forEach(d=>{
      const dt = new Date(d.date+'T00:00:00Z');
      const mon = new Date(dt); mon.setUTCDate(dt.getUTCDate()-dt.getUTCDay()+1);
      const k = mon.toISOString().slice(0,10);
      wk[k]=(wk[k]||0)+contDayCost(d);
    });
    const wkeys = Object.keys(wk).sort().slice(-4);
    svgBarChart($('contWeekly'), wkeys.map(k=>k.slice(5)), wkeys.map(k=>wk[k]*1000));
    // per-app table (30d)
    const agg = {};
    D.containers.daily.forEach(d=>{ for(const a in d.apps){
      const s = agg[a] = agg[a] || {mem:0,disk:0,cpu:0,tx:0,last:d.date};
      s.mem+=d.apps[a].mem; s.disk+=d.apps[a].disk; s.cpu+=d.apps[a].cpu;
      s.tx+=d.apps[a].tx||0; s.last=d.date; }});
    const rows = Object.entries(agg).sort((a,b)=>contCost(b[1].mem,b[1].disk,b[1].cpu)-contCost(a[1].mem,a[1].disk,a[1].cpu));
    $('contTable').innerHTML = tableHTML(
      ['app','mem GiB-s','disk GB-s','cpu vCPU-s','tx MB','est $ (30d)','last seen'],
      rows,
      r => '<td>'+r[0]+'</td><td class="num">'+(r[1].mem/1073741824).toFixed(0)+
        '</td><td class="num">'+(r[1].disk/1e9).toFixed(0)+'</td><td class="num">'+
        r[1].cpu.toFixed(1)+'</td><td class="num">'+(r[1].tx/1e6).toFixed(1)+
        '</td><td class="num">'+fmt.usd(contCost(r[1].mem,r[1].disk,r[1].cpu))+'</td><td>'+
        r[1].last+'</td>');
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
      return '<td>'+fmt.d(r.ts_unix)+'</td><td>'+(r.arm||'—')+'</td><td class="note">'+
        (r.model||'?').split('/').pop()+'</td><td class="num">'+(r.n||0)+
        '</td><td class="num">'+(sv.requests||0)+'</td><td class="num">'+
        fmt.n1(sv.neurons||0)+'</td><td class="num">'+fmt.usd((sv.neurons||0)*D.neuron_usd)+
        '</td><td class="num">'+fmt.usd(cl.usd_est||0)+'</td><td class="note"'+
        (inc?' style="color:var(--crit)"':'')+'>'+esc(r.note||'')+'</td>';
    });
}
function esc(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}
$('missingList').innerHTML = D.missing.map(m=>'<li><b>'+m[0]+'</b> — '+m[1]+'</li>').join('');
$('footer').textContent = 'riir ops dashboard · generated by scripts/gen_dashboard.py (riir-clippy issue 044) · '
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
    coverage = coverage_snapshot(sweep)
    training = collect_training()
    fixer = load_fixer_quality()
    data = {
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M"),
        "neuron_usd": NEURON_USD,
        "cap": DAILY_FREE_CAP,
        "ledger": {
            "rows": ledger_rows,
            "per_model": led["per_model"],
            "per_day": led["per_day"],
            "incidents": led["incidents"],
            "totals": {
                "usd_client": round(led["total_usd_client"], 6),
                "server_neurons": round(led["total_neurons_server"], 2),
                "requests": led["total_requests"],
            },
        },
        "score": score,
        "traj": traj,
        "cadence": cadence,
        "cf_daily": cf_daily,
        "containers": containers,
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
            ["coverage", bool(coverage)],
            ["training", bool(training)],
            ["fixer-quality", bool(fixer)],
        ],
    }
    return data


def main() -> int:
    sweep = "--sweep" in sys.argv
    out = DEFAULT_OUT
    if "--out" in sys.argv:
        out = Path(sys.argv[sys.argv.index("--out") + 1])
    data = build_data(sweep)
    out.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(data, separators=(",", ":"))
    # Never embed anything but aggregated data — assert no token leaked.
    assert "oauth_token" not in payload and "Bearer " not in payload
    html = HTML_TEMPLATE.replace("__DATA__", payload)
    out.write_text(html)
    log(f"wrote {out} ({len(html)//1024} KiB, "
        f"ledger={len(data['ledger']['rows'])} rows, "
        f"cf_days={len(data['cf_daily'])}, traj_repos={len(data['traj'])}, "
        f"coverage={len(data['coverage'])}, training={len(data['training'])}, "
        f"fixer={len(data['fixer'])})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
