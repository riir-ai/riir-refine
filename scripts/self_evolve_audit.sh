#!/bin/sh
# self_evolve_audit.sh — one command to judge heal ADOPTION + E2E self-evolve state.
# Answers: "did agents actually use cargo heal?" and "is the self-evolve loop closing?"
# Issue 037 gates read directly from this output. Stateless: run anytime, compare runs.
# Issue 039 T4: §1b renders the ELO leaderboard (rule-vs-shape ratings) from
# --dump-trajectories — the human-readable rating surface (ratings stay in
# storage; only the report is prose).
set -u

HEAL_REPO="/Users/katopz/git/riir-clippy"
MEMORY="${RIIR_HEAL_MEMORY:-$HOME/.cache/riir-clippy/memory.json}"
HISTORY="$HEAL_REPO/.benchmarks/score_history.jsonl"
INTAKE="$HEAL_REPO/.distill/001_mining_queue_snapshot.md"
ADOPTION_DATE="2026-08-24T00:00:00"
REPOS="katgpt-rs riir-ai riir-train riir-neuron-db riir-chain riir-clippy riir-game-sdk riir-mmorpg-examples riir-unity riir-viewbridge riir-dapps"

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║ cargo-heal adoption + self-evolve audit  (Issue 037)         ║"
echo "╚══════════════════════════════════════════════════════════════╝"

echo ""
echo "── 0. Setup health (preconditions for 'agent will use it') ──"
if [ -x "$HOME/.cargo/bin/cargo-heal" ] || [ -L "$HOME/.cargo/bin/cargo-heal" ]; then
  echo "  binary:    OK  (~/.cargo/bin/cargo-heal)"
else
  echo "  binary:    MISSING (~/.cargo/bin/cargo-heal)"
fi
if [ -L "$HOME/.agents/skills/cargo-heal" ]; then
  echo "  skill:     OK  (~/.agents/skills/cargo-heal)"
else
  echo "  skill:     MISSING (~/.agents/skills/cargo-heal)"
fi
if grep -q 'cargo heal --fix --write --verify' "$HOME/.config/zed/AGENTS.md" 2>/dev/null; then
  echo "  global rule: OK  (~/.config/zed/AGENTS.md)"
else
  echo "  global rule: MISSING in ~/.config/zed/AGENTS.md"
fi

echo ""
echo "── 1. Memory (loop state — verdicts & tiers; 038: store-first) ──"
# 038-T4: the repo-local .heal/trajectories.kv store is the ACTIVE backend
# (latent_retrieval builds); the envelope is legacy/fallback. Read BOTH:
# per-repo store rows via the binary's backend-agnostic dump, then the
# envelope if it still exists (or its .migrated tombstone).
HEAL_BIN="${HEAL_REPO}/target/release/cargo-heal"
[ -x "$HEAL_BIN" ] || HEAL_BIN="$HOME/.cargo/bin/cargo-heal"
DUMP_TMP="$(mktemp)"
STORE_REPOS=""
for r in $REPOS; do
  d="/Users/katopz/git/$r"
  [ -d "$d/.heal/trajectories.kv" ] || continue
  STORE_REPOS="$STORE_REPOS $r"
  if [ -x "$HEAL_BIN" ]; then
    RIIR_HEAL_TRAJ_ROOT="$d" "$HEAL_BIN" --dump-trajectories > "$DUMP_TMP" 2>/dev/null
    python3 - "$r" "$DUMP_TMP" <<'EOF'
import json, sys
repo, path = sys.argv[1], sys.argv[2]
rows = succ = fail = cert = 0
for line in open(path):
    line = line.strip()
    if not line.startswith('{'):
        continue
    try:
        v = json.loads(line)
    except Exception:
        continue
    rows += 1
    succ += int(v.get('success_count', 0))
    fail += int(v.get('fail_count', 0))
    if int(v.get('tier', 1)) == 0:
        cert += 1
print(f"  store {repo}: {rows} row(s)  success={succ}  fail={fail}  Certified={cert}")
EOF
  else
    echo "  store $r: present (binary unavailable for the row summary)"
  fi
done
rm -f "$DUMP_TMP"
[ -n "$STORE_REPOS" ] && echo "  repos with stores:$STORE_REPOS  (037-A2 per-repo evidence)"
if [ -f "$MEMORY" ]; then
  python3 - "$MEMORY" <<'EOF'
import json, sys, collections
m = json.load(open(sys.argv[1]))
t = m.get('trajectories', [])
tiers = collections.Counter(str(b.get('tier')) for b in t)
succ = sum(b.get('success_count', 0) for b in t)
fail = sum(b.get('fail_count', 0) for b in t)
cert = tiers.get('Certified', 0)
wd = tiers.get('Withdrawn', 0)
print(f"  envelope buckets: {len(t)}  tiers: {dict(tiers)}")
print(f"  outcome totals: success={succ}  fail={fail}  (Certified={cert}, Withdrawn={wd})")
top = collections.Counter(b.get('lint_name','?') for b in t).most_common(5)
print("  top lints: " + ", ".join(f"{k}={v}" for k, v in top))
# E2 gate verdicts
print(f"  E2 (real verdicts recorded): {'PASS' if succ > 0 or cert > 0 else 'FAIL — positive outcomes never recorded (Issue 037 T2)'}")
EOF
elif [ -f "${MEMORY}.migrated" ]; then
  echo "  envelope: MIGRATED (${MEMORY}.migrated) — the active backend is the repo-local store(s) above"
else
  echo "  memory ABSENT at $MEMORY — no heal run has ever recorded on this box"
fi

echo ""
echo "── 1b. ELO leaderboard (Issue 039 T4 — rule-vs-shape ratings) ──"
# The human-readable rendering of the 039 ELO surface: rules are rated by
# REAL usage (win = kept+verified verdict, loss = reverted/created-warning,
# review-only sighting = draw → stays unrated). Withdrawn rows (tier=2) are
# EXCLUDED from the standings (Issue 021's trust model — ratings themselves
# include every match; this is a render filter). Ratings are as fresh as
# the last `cargo heal --recompute-elo` in that repo.
if [ -n "$STORE_REPOS" ] && [ -x "$HEAL_BIN" ]; then
  ELO_TMP="$(mktemp)"
  for r in $STORE_REPOS; do
    d="/Users/katopz/git/$r"
    RIIR_HEAL_TRAJ_ROOT="$d" "$HEAL_BIN" --dump-trajectories > "$ELO_TMP" 2>/dev/null
    python3 - "$r" "$ELO_TMP" <<'EOF'
import json, sys
repo, path = sys.argv[1], sys.argv[2]
BASE = 1200.0
rules = {}   # lint_key -> {wins, losses, elo(None|float)}
spans = {}   # shape -> {elo(None|float)}
withdrawn = 0
for line in open(path):
    line = line.strip()
    if not line.startswith('{'):
        continue
    try:
        v = json.loads(line)
    except Exception:
        continue
    if int(v.get('tier', 1)) == 2:
        withdrawn += 1
        continue  # Withdrawn rows excluded from the standings (Issue 021)
    e = rules.setdefault(v.get('lint_key', '?'), {'wins': 0, 'losses': 0, 'elo': None})
    e['wins'] += int(v.get('success_count', 0))
    e['losses'] += int(v.get('fail_count', 0))
    elo = float(v.get('elo', BASE))
    if elo != BASE:
        e['elo'] = elo
    s = spans.setdefault(v.get('shape', ''), {'elo': None})
    span_elo = float(v.get('span_elo', BASE))
    if span_elo != BASE:
        s['elo'] = span_elo
rated = sorted(((k, v) for k, v in rules.items() if v['elo'] is not None),
               key=lambda kv: (-kv[1]['elo'], -kv[1]['wins'], kv[0]))
pending = sorted(k for k, v in rules.items() if v['elo'] is None and v['wins'] + v['losses'] > 0)
draws = sorted(k for k, v in rules.items() if v['elo'] is None and v['wins'] + v['losses'] == 0)
tough = sorted(((k, v['elo']) for k, v in spans.items() if v['elo'] is not None),
                key=lambda kv: -kv[1])[:3]
print(f"  {repo}: {len(rated)} rated | {len(draws)} draw(s, review-only) | {len(pending)} pending-recompute | {withdrawn} Withdrawn row(s) excluded")
for name, v in rated[:8]:
    print(f"    {v['elo']:7.1f}  {name[:44]:<44}  W={v['wins']:<4} L={v['losses']}")
if draws:
    shown = ", ".join(draws[:6])
    tail = " …" if len(draws) > 6 else ""
    print(f"    draws (review-only, unrated): {shown}{tail}")
if pending:
    print(f"    NOTE: {len(pending)} rule(s) with verdicts but no rating — run: cargo heal --recompute-elo")
if tough:
    def clip(s):
        s = s.replace("\n", " ")
        return s if len(s) <= 38 else s[:37] + "…"
    print("    toughest shapes (span_elo): " + ", ".join(f'"{clip(k)}" {v:.0f}' for k, v in tough))
EOF
  done
  rm -f "$ELO_TMP"
else
  echo "  (needs ≥1 store repo + the readable binary — see §1)"
fi

echo ""
echo "── 2. Score bench history (learning quality over time) ──"
if [ -f "$HISTORY" ]; then
  python3 - "$HISTORY" <<'EOF'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
print(f"  runs: {len(rows)}")
for r in rows:
    ts = (r.get('ts') or r.get('timestamp') or '?')[:19]
    print(f"    {ts}  heal={float(r.get('heal_rate',0))*100:5.1f}%  "
          f"created={float(r.get('created_rate',0))*100:4.1f}%  "
          f"safety={float(r.get('parse_safety',0))*100:5.1f}%  edits={r.get('edits_total','?')}")
last = rows[-1] if rows else {}
hr = float(last.get('heal_rate', 0)) * 100
cr = float(last.get('created_rate', 0)) * 100
print(f"  E4 (no decay vs 98.1/1.9): {'see latest run above — gate: heal>=98.1 created<=1.9' if rows else 'NO RUNS'}")
EOF
else
  echo "  score_history.jsonl ABSENT — run: cargo heal --score-bench"
fi

echo ""
echo "── 3. Post-mining intake (mining report progress) ──"
if [ -f "$INTAKE" ]; then
  done_n=$(grep -c '^- \[x\]' "$INTAKE" 2>/dev/null); done_n=${done_n:-0}
  open_n=$(grep -c '^- \[ \]' "$INTAKE" 2>/dev/null); open_n=${open_n:-0}
  echo "  intake items: done=${done_n:-0} open=${open_n:-0}"
  echo "  E5 (>=1 class closed e2e): $([ "${done_n:-0}" -ge 1 ] && echo PASS || echo 'FAIL — no post-mining class closed yet')"
else
  echo "  intake ABSENT at $INTAKE"
fi

echo ""
echo "── 4. Adoption (did agents actually run it? durable trail) ──"
total=0
for r in $REPOS; do
  d="/Users/katopz/git/$r"
  [ -d "$d/.git" ] || continue
  n=$(git -C "$d" log --oneline --since="$ADOPTION_DATE" --grep='heal' 2>/dev/null | wc -l | tr -d ' ')
  [ "$n" = "0" ] || echo "  $r: $n heal-commit(s) since $ADOPTION_DATE"
  total=$((total + n))
done
echo "  total heal-related commits since $ADOPTION_DATE: $total"
echo "  A2 gate (>=2 repos with heal commits): see above"
echo ""
echo "NOTE: envelope rows carry no timestamps, but 038 store rows do"
echo "(last_seen_at) — per-day adoption cadence (A1) becomes measurable from"
echo "--dump-trajectories once stores accumulate. Envelope: git-commit proxy only."
