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
from datetime import datetime, timezone
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
GIT_ROOT = Path("/Users/katopz/git")

CF_ACCOUNT_ID = "7e11517c4dd4f6e9cede7da9b60d66eb"
CF_GRAPHQL = "https://api.cloudflare.com/client/v4/graphql"
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
    token = None
    if WRANGLER_TOKEN_FILE.exists():
        m = re.search(r'^oauth_token = "(.+)"$',
                      WRANGLER_TOKEN_FILE.read_text(), re.MULTILINE)
        if m:
            token = m.group(1)
    if not token:
        log("no wrangler OAuth token — skipping live CF analytics")
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
</script>
</body>
</html>
"""

MISSING_FOR_PROD = [
    ["auth gating", "unauthenticated workers.dev URL — add a token query gate "
     "(tiny Worker script in front of assets) or Cloudflare Access"],
    ["auto-refresh", "data is a snapshot at generation time — schedule "
     "regeneration (cron on this box, GitHub Action, or a Worker cron that "
     "re-runs collection into KV/R2)"],
    ["budget alerting", "notify (Discord/email/webhook) at 50/90/100% of the "
     "10k/day cap + monthly USD budget; the bar turns red but nobody is paged"],
    ["KAT rail", "ledger rows carry kat:null — per-fix cost in KAT tokens per "
     "Proposal 005 Phase 1; wire when the rail lands"],
    ["training/GPU axis", "riir-train runs (plan, steps, tok/s, GPU-hours, "
     "M3 vs 4090) are not ingested — needs a run-log convention + collector"],
    ["coverage auto-measure", "--sweep is manual + slow; schedule it (nightly) "
     "so the unhealed-remainder table stays honest"],
    ["fixer quality history", "keeps/vetoes per arm per session (Bench 049/053 "
     "shape) as a time series, not prose in .md files"],
    ["retention", "CF GraphQL served ~14d here; the committed ledger is the "
     "archive — keep appending, never rewrite"],
    ["incident timeline", "ledger notes carry incidents (qwen 7.8k) — render "
     "as an annotated timeline strip once there are >3"],
    ["heartbeat", "the dashboard cannot tell you the GENERATOR died — add a "
     "generated-at staleness check + alert when >48h"],
]


def build_data(sweep: bool) -> dict:
    ledger_rows = load_ledger()
    led = ledger_summary(ledger_rows)
    score = load_score_history()
    traj = collect_trajectories()
    cadence = collect_git_cadence()
    cf_daily = collect_cf_daily()
    coverage = coverage_snapshot(sweep)
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
        "coverage": coverage,
        "missing": MISSING_FOR_PROD,
        "source_flags": [
            ["ledger", bool(ledger_rows)],
            ["score-history", bool(score)],
            ["trajectories", bool(traj)],
            ["git-cadence", bool(cadence)],
            ["cf-analytics", bool(cf_daily)],
            ["coverage", bool(coverage)],
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
        f"coverage={len(data['coverage'])})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
