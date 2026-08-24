#!/bin/sh
# self_evolve_audit.sh — one command to judge heal ADOPTION + E2E self-evolve state.
# Answers: "did agents actually use cargo heal?" and "is the self-evolve loop closing?"
# Issue 037 gates read directly from this output. Stateless: run anytime, compare runs.
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
echo "── 1. Memory (loop state — verdicts & tiers) ──"
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
print(f"  buckets: {len(t)}  tiers: {dict(tiers)}")
print(f"  outcome totals: success={succ}  fail={fail}  (Certified={cert}, Withdrawn={wd})")
top = collections.Counter(b.get('lint_name','?') for b in t).most_common(5)
print("  top lints: " + ", ".join(f"{k}={v}" for k, v in top))
# E2 gate verdicts
print(f"  E2 (real verdicts recorded): {'PASS' if succ > 0 or cert > 0 else 'FAIL — positive outcomes never recorded (Issue 037 T2)'}")
EOF
else
  echo "  memory ABSENT at $MEMORY — no heal run has ever recorded on this box"
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
echo "NOTE: memory.json carries NO timestamps (Issue 037 T3) — per-day adoption"
echo "cadence (A1) is currently proxied by git commits above only."
