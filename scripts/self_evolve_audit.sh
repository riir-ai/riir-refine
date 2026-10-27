#!/bin/sh
# self_evolve_audit.sh — one command to judge heal ADOPTION + E2E self-evolve state.
# Answers: "did agents actually use cargo refine?" and "is the self-evolve loop closing?"
# Issue 037 gates read directly from this output. Stateless: run anytime, compare runs.
# Issue 039 T4: §1b renders the ELO leaderboard (rule-vs-shape ratings) from
# --dump-trajectories — the human-readable rating surface (ratings stay in
# storage; only the report is prose).
set -u

# Portable: repo root + sibling root derived from THIS script's location
# (works unchanged on the M3 checkout and the 4090/Windows checkout).
HEAL_REPO="$(cd "$(dirname "$0")/.." && pwd)"
GIT_ROOT="$(dirname "$HEAL_REPO")"
# Python is OPTIONAL: probe by EXECUTION, not command existence — Windows ships
# a Microsoft-Store `python3` stub that resolves on PATH but only prints
# "Python was not found" (measured 2026-08-31). Sections that need JSON math
# degrade to a one-line note instead of spewing stub errors.
PY_BIN=""
for _c in python3 python py; do
  if command -v "$_c" >/dev/null 2>&1 && "$_c" -c 'import sys' >/dev/null 2>&1; then
    PY_BIN="$_c"
    break
  fi
done
run_py() {
  if [ -n "$PY_BIN" ]; then
    # Windows consoles default python's stdio to the LOCALE codec (cp874 on the
    # 4090 box) — trajectories/scores are UTF-8 (…, box-drawing) → force UTF-8.
    PYTHONIOENCODING=utf-8 "$PY_BIN" "$@"
  else
    cat >/dev/null
    echo "  (python unavailable on this box — section skipped)"
  fi
}
# Epoch → readable date, BSD (`date -r EPOCH`) first, GNU (`date -d @EPOCH`)
# second, epoch fallback. Probe by execution — GNU's `-r` means REFERENCE FILE
# and fails on a number, which is exactly the fallback signal.
bin_age_fmt() {
  _bf_d="$(date -r "$1" '+%m-%d %H:%M' 2>/dev/null || true)"
  if [ -n "$_bf_d" ]; then echo "$_bf_d"; return 0; fi
  _bf_d="$(date -u -d "@$1" '+%m-%d %H:%M' 2>/dev/null || true)"
  if [ -n "$_bf_d" ]; then echo "$_bf_d"; return 0; fi
  echo "$1"
}
MEMORY="${RIIR_REFINE_MEMORY:-$HOME/.cache/riir-refine/memory.json}"
HISTORY="$HEAL_REPO/.benchmarks/score_history.jsonl"
INTAKE="$HEAL_REPO/.distill/001_mining_queue_snapshot.md"
INTAKE_HISTORY="$HEAL_REPO/.research/108_mining_queue_snapshot_history.md"
ADOPTION_DATE="2026-08-24T00:00:00"
REPOS="katgpt-rs riir-ai riir-train riir-neuron-db riir-chain riir-refine riir-game-sdk riir-mmorpg-examples riir-viewbridge riir-dapps"

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║ cargo-refine adoption + self-evolve audit  (Issue 037)         ║"
echo "╚══════════════════════════════════════════════════════════════╝"

echo ""
echo "── 0. Setup health (preconditions for 'agent will use it') ──"
if [ -x "$HOME/.cargo/bin/cargo-refine" ] || [ -L "$HOME/.cargo/bin/cargo-refine" ]; then
  echo "  binary:    OK  (~/.cargo/bin/cargo-refine)"
else
  echo "  binary:    MISSING (~/.cargo/bin/cargo-refine)"
fi
# P3 discipline mechanized (2026-09-02): a stale installed binary silently serves a
# stale corpus/proposer set — measured TWICE (the 08-31 incident sat through five
# src/ commits despite a "rebuilt" claim; again 09-02 through Batch 98's corpus).
# Rule: trust the mtime, not the message — flag when the binary predates the last
# commit touching src/ or Cargo.toml. Sibling path-dep state is deliberately NOT
# tracked here (a dirty sibling tree has no "current" to compare against); gates
# grep `bin-age:.*STALE`.
BIN_AGE_PATH="$HOME/.cargo/bin/cargo-refine"
BIN_AGE_MTIME=""
if [ -e "$BIN_AGE_PATH" ] || [ -L "$BIN_AGE_PATH" ]; then
  # FOLLOW the symlink: macOS/BSD stat does NOT dereference by default (-L
  # does), GNU stat DOES by default (-L is explicit dereference). The integer
  # case-guard filters whichever flavor misparses (GNU `-f` is filesystem
  # mode and does not print an epoch).
  BIN_AGE_MTIME="$(stat -L -f %m "$BIN_AGE_PATH" 2>/dev/null || true)"
  case "$BIN_AGE_MTIME" in ''|*[!0-9]*) BIN_AGE_MTIME="" ;; esac
  if [ -z "$BIN_AGE_MTIME" ]; then
    BIN_AGE_MTIME="$(stat -L -c %Y "$BIN_AGE_PATH" 2>/dev/null || true)"
    case "$BIN_AGE_MTIME" in ''|*[!0-9]*) BIN_AGE_MTIME="" ;; esac
  fi
  if [ -z "$BIN_AGE_MTIME" ]; then
    BIN_AGE_MTIME="$(stat -f %m "$BIN_AGE_PATH" 2>/dev/null || true)"
    case "$BIN_AGE_MTIME" in ''|*[!0-9]*) BIN_AGE_MTIME="" ;; esac
  fi
  if [ -z "$BIN_AGE_MTIME" ]; then
    BIN_AGE_MTIME="$(stat -c %Y "$BIN_AGE_PATH" 2>/dev/null || true)"
    case "$BIN_AGE_MTIME" in ''|*[!0-9]*) BIN_AGE_MTIME="" ;; esac
  fi
fi
BIN_AGE_LAST_CT="$(git -C "$HEAL_REPO" log -1 --format=%ct -- src/ Cargo.toml 2>/dev/null || true)"
BIN_AGE_LAST_H="$(git -C "$HEAL_REPO" log -1 --format=%h -- src/ Cargo.toml 2>/dev/null || true)"
[ -n "$BIN_AGE_LAST_H" ] || BIN_AGE_LAST_H="?"
if [ -n "$BIN_AGE_MTIME" ] && [ -n "$BIN_AGE_LAST_CT" ]; then
  if [ "$BIN_AGE_MTIME" -ge "$BIN_AGE_LAST_CT" ]; then
    echo "  bin-age:   OK  (binary $(bin_age_fmt "$BIN_AGE_MTIME") >= last src commit $BIN_AGE_LAST_H $(bin_age_fmt "$BIN_AGE_LAST_CT"))"
  else
    echo "  bin-age:   STALE — binary $(bin_age_fmt "$BIN_AGE_MTIME") predates last src commit $BIN_AGE_LAST_H $(bin_age_fmt "$BIN_AGE_LAST_CT"); adoption runs serve a stale corpus"
    echo "             rebuild: cd $HEAL_REPO && cargo build --release --features latent_retrieval,kat_service,kat_transport,fix_verify,update_check --bin cargo-refine (the Issue-153 pinned release set, byte-identical to release.yml's build line — latent_retrieval alone compiles the refill/sync/burn-push surfaces to nothing, and a set missing update_check honestly self-reports STALE at --version)"
  fi
else
  echo "  bin-age:   UNKNOWN (mtime/git comparison unavailable on this box — broken symlink? no git?)"
fi
# Skill + global-rule checks are drift-aware and multi-platform:
#  - Mac: the global skill is a SYMLINK (the discipline)
#  - Windows: native symlinks need admin/Developer Mode (Operation not
#    permitted, measured 2026-08-31) — a CONTENT-VERIFIED copy is accepted;
#    a drifted copy is named loudly
if [ -L "$HOME/.agents/skills/cargo-refine" ]; then
  echo "  skill:     OK  (symlink)"
elif [ -f "$HOME/.agents/skills/cargo-refine/SKILL.md" ]; then
  if diff -q "$HEAL_REPO/.agents/skills/cargo-refine/SKILL.md" "$HOME/.agents/skills/cargo-refine/SKILL.md" >/dev/null 2>&1; then
    echo "  skill:     OK  (content-verified copy — no native symlink on this box)"
  else
    echo "  skill:     STALE COPY — SKILL.md differs from canonical $HEAL_REPO/.agents/skills/cargo-refine; refresh it"
  fi
else
  echo "  skill:     MISSING (~/.agents/skills/cargo-refine)"
fi
# Zed's global AGENTS.md lives at ~/.config/zed/ on macOS/Linux but at
# %APPDATA%\\Zed on Windows (measured 2026-08-31) — check both.
ZED_AGENTS=""
for cand in "$HOME/.config/zed/AGENTS.md" "${APPDATA:-$HOME/AppData/Roaming}/Zed/AGENTS.md"; do
  [ -f "$cand" ] && ZED_AGENTS="$cand" && break
done
if [ -n "$ZED_AGENTS" ] && grep -q 'cargo refine --fix --write --verify' "$ZED_AGENTS" 2>/dev/null; then
  echo "  global rule: OK  ($ZED_AGENTS)"
else
  echo "  global rule: MISSING (checked ~/.config/zed/AGENTS.md + %APPDATA%/Zed/AGENTS.md)"
fi

echo ""
echo "── 1. Memory (loop state — verdicts & tiers; 038: store-first) ──"
# 038-T4: the repo-local .refine/trajectories.kv store is the ACTIVE backend
# (latent_retrieval builds); the envelope is legacy/fallback. Read BOTH:
# per-repo store rows via the binary's backend-agnostic dump, then the
# envelope if it still exists (or its .migrated tombstone).
HEAL_BIN="${HEAL_REPO}/target/release/cargo-refine"
[ -x "$HEAL_BIN" ] || HEAL_BIN="$HOME/.cargo/bin/cargo-refine"
DUMP_TMP="$(mktemp)"
STORE_REPOS=""
for r in $REPOS; do
  d="$GIT_ROOT/$r"
  [ -d "$d/.refine/trajectories.kv" ] || continue
  STORE_REPOS="$STORE_REPOS $r"
  if [ -x "$HEAL_BIN" ]; then
    RIIR_REFINE_TRAJ_ROOT="$d" "$HEAL_BIN" --dump-trajectories > "$DUMP_TMP" 2>/dev/null
    run_py - "$r" "$DUMP_TMP" <<'EOF'
import json, sys
repo, path = sys.argv[1], sys.argv[2]
rows = succ = fail = cert = 0
for line in open(path, encoding='utf-8', errors='replace'):
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
  run_py - "$MEMORY" <<'EOF'
import json, sys, collections
m = json.load(open(sys.argv[1], encoding='utf-8', errors='replace'))
t = m.get('trajectories', [])
tiers = collections.Counter(str(b.get('tier')) for b in t)
succ = sum(b.get('success_count', 0) for b in t)
fail = sum(b.get('fail_count', 0) for b in t)
cert = tiers.get('Certified', 0)
wd = tiers.get('Withdrawn', 0)
comps = collections.Counter(b.get('component') for b in t if b.get('component'))
print(f"  envelope buckets: {len(t)}  tiers: {dict(tiers)}")
print(f"  outcome totals: success={succ}  fail={fail}  (Certified={cert}, Withdrawn={wd})")
if comps:
    print(f"  component-attributed (047): {dict(comps)}")
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
# the last `cargo refine --recompute-elo` in that repo.
if [ -n "$STORE_REPOS" ] && [ -x "$HEAL_BIN" ]; then
  ELO_TMP="$(mktemp)"
  for r in $STORE_REPOS; do
    d="$GIT_ROOT/$r"
    RIIR_REFINE_TRAJ_ROOT="$d" "$HEAL_BIN" --dump-trajectories > "$ELO_TMP" 2>/dev/null
    run_py - "$r" "$ELO_TMP" "$GIT_ROOT/$r" <<'EOF'
import json, sys
repo, path, repo_root = sys.argv[1], sys.argv[2], sys.argv[3]
BASE = 1200.0
rules = {}   # lint_key -> {wins, losses, elo(None|float)}
spans = {}   # shape -> {elo(None|float)}
withdrawn = 0
for line in open(path, encoding='utf-8', errors='replace'):
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
    print(f"    NOTE: {len(pending)} rule(s) with verdicts but no rating — run: RIIR_REFINE_TRAJ_ROOT={repo_root} cargo refine --recompute-elo (BARE form reads the default root — measured idempotent-0/0 while this row stayed pending, 2026-10-27)")
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
echo "── 1c. Self-collapse monitor (Issue 042 — retrieval-loop source diversity) ──"
# R346 (arXiv:2608.22118) watch axis: as the trajectory stores age, does the
# REAL heal retrieval loop (HealPipeline::heal Step 1/3 + selection) become
# memory-dominated at the expense of pool rule-diversity? Trend signal for
# Issue 037's A-axes: pool memory-fraction rising + top-k rule entropy
# falling as stores age = the collapse signature; entropy RISING is the
# healthy direction (more rules covered). Read-only (same open-replay class
# as --dump-trajectories above; this mode never migrates and never puts).
# Needs a latent_retrieval build of the binary (the fixture set + store
# bridge); a stale binary is detected once and noted.
if [ -n "$STORE_REPOS" ] && [ -x "$HEAL_BIN" ]; then
  CR_PROBE="$(mktemp -d)"
  if RIIR_REFINE_TRAJ_ROOT="$CR_PROBE" "$HEAL_BIN" --collapse-report 2>/dev/null | grep -q 'collapse-report:'; then
    for r in $STORE_REPOS; do
      d="$GIT_ROOT/$r"
      RIIR_REFINE_TRAJ_ROOT="$d" "$HEAL_BIN" --collapse-report 2>/dev/null | grep -E 'fresh\(empty\)|store\('
    done
    echo "  reading: pool mem% = Memory share of the heal candidate pool;"
    echo "           top-k H = normalized rule-entropy of the retrieved top-k;"
    echo "           mem win% = Memory share of served fixes (citation-rate analog)."
  else
    echo "  (binary lacks --collapse-report — rebuild with: cargo build --release --features latent_retrieval -p riir-refine --bin cargo-refine)"
  fi
  rm -rf "$CR_PROBE"
else
  echo "  (needs ≥1 store repo + the readable binary — see §1)"
fi

echo ""
echo "── 1d. Component-attributed failures (Issue 047 — E/rho/W/C miss map) ──"
# The Recuris localization-by-construction transplant: every recorded
# failure carries the component class derived AT RECORD TIME (never
# post-hoc). E = corpus/proposer gap, rho = retrieval (crowding/pool
# depth), W = drafter (defective fix the gate caught), C = verifier/gate
# (passed a bad fix / rejected without judging). The dominant class is
# the next improvement target instead of a guess.
if [ -n "$STORE_REPOS" ] && [ -x "$HEAL_BIN" ]; then
  COMP_TMP="$(mktemp)"
  for r in $STORE_REPOS; do
    d="$GIT_ROOT/$r"
    RIIR_REFINE_TRAJ_ROOT="$d" "$HEAL_BIN" --dump-trajectories > "$COMP_TMP" 2>/dev/null
    run_py - "$r" "$COMP_TMP" <<'EOF'
import json, sys, collections
repo, path = sys.argv[1], sys.argv[2]
rows = []
for line in open(path, encoding='utf-8', errors='replace'):
    line = line.strip()
    if not line.startswith('{'):
        continue
    try:
        v = json.loads(line)
    except Exception:
        continue
    rows.append(v)
tagged = [v for v in rows if v.get('component')]
if not tagged:
    print(f"  {repo}: 0 attributed row(s) ({len(rows)} row(s) — pre-047 rows decode unattributed)")
    raise SystemExit(0)
counts = collections.Counter(v['component'] for v in tagged)
order = ['E', 'rho', 'W', 'C']
long = {'E': 'corpus/proposer', 'rho': 'retrieval', 'W': 'drafter', 'C': 'verifier/gate'}
parts = "  ".join(f"{k}({long[k]})={counts.get(k, 0)}" for k in order if counts.get(k, 0))
print(f"  {repo}: {len(tagged)} attributed row(s)  {parts}")
for k in order:
    reps = [v for v in tagged if v['component'] == k][:3]
    if not reps:
        continue
    def clip(s):
        s = (s or '').replace("\n", " ")
        return s if len(s) <= 42 else s[:41] + "…"
    shown = ", ".join(f"{v.get('lint_key','?')} [\"{clip(v.get('shape',''))}\"] L={v.get('fail_count',0)}" for v in reps)
    print(f"    {k}: {shown}")
EOF
  done
  rm -f "$COMP_TMP"
else
  echo "  (needs ≥1 store repo + the readable binary — see §1)"
fi

echo ""
echo "── 2. Score bench history (learning quality over time) ──"
if [ -f "$HISTORY" ]; then
  run_py - "$HISTORY" <<'EOF'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1], encoding='utf-8', errors='replace') if l.strip()]
print(f"  runs: {len(rows)}")
for r in rows:
    ts = (r.get('ts') or r.get('timestamp') or '?')[:19]
    print(f"    {ts}  heal={float(r.get('heal_rate',0))*100:5.1f}%  "
          f"created={float(r.get('created_rate',0))*100:4.1f}%  "
          f"safety={float(r.get('parse_safety',0))*100:5.1f}%  edits={r.get('edits_total','?')}")
last = rows[-1] if rows else {}
# E4 population discipline (2026-09-30): the post-P9 floor is calibrated
# on the STANDARD generation-1 score-bench population (n_ran≈35, edits≈69).
# score_history also accumulates generation-2/3 rows — the freeze-regen e2e
# and the adopted per-batch gen-N full-corpus sweep cadence (Issues 089/102;
# n_ran≈98, edits≈150+, created≈10% = the documented closure-birth class
# priced at shaped_rate in gen-8). Adjudicating those against this floor is
# population mixing: after every batch landing the sweep row would read
# "FAIL — DECAY" over a healthy standard population (measured 2026-09-30:
# sweep row #174 read 95.7/10.1 while the gen-1 row an hour earlier read
# 96.4/1.8, floor holding). E4 adjudicates on the latest gen-1 row and NAMES
# that row; non-gen-1 rows stay printed above but never gate this floor.
# Rows predating the `generation` field were gen-1 by construction.
gen1 = [r for r in rows if str(r.get('generation', '1')) == '1'] or rows
last = gen1[-1] if gen1 else {}
hr = float(last.get('heal_rate', 0)) * 100
cr = float(last.get('created_rate', 0)) * 100
# E4 floor re-pinned 2026-09-03 (post-mining triage): the old 98.1 reference
# predates the P9 fixture `riir-ai-ruliology-player-001` (added 2026-08-25),
# whose map_unwrap_or persist is a CORRECT decline — the trait-object-cast
# mapper guard (probe re-verified 2026-09-03 on the current toolchain: the
# map_or rewrite still E0308s — expected `&fn(..)`, found `&dyn Fn(..)`; the
# original compiles). The rate step 98.1→96.2 was denominator growth from a
# deliberately-declined lint, not healing decay; every run since 08-25 is
# stable at 96.2/1.9. The remaining persist (useless_format, String-typed) +
# the 1 created (single_option_map on manual_map's output — clippy's own
# cascade) are the Run-#3 documented boundaries. Floor = post-P9 stable
# state; a drop BELOW it (or created above) is decay.
E4_HEAL_FLOOR = 96.1   # 96.2 stable − tie slack
E4_CREATED_CEIL = 2.0  # 1 created = 1.9%
verdict = 'PASS' if (hr >= E4_HEAL_FLOOR and cr <= E4_CREATED_CEIL) else 'FAIL — DECAY (investigate the miss map: cargo refine --score-bench)'
adj_ts = (last.get('ts') or last.get('timestamp') or '?')[:19]
print(f"  E4 (no decay vs post-P9 floor 96.2/<=1.9): {verdict} — latest heal={hr:.1f}% created={cr:.1f}%")
print(f"    adjudicated on the gen-1 row {adj_ts} (n_ran={last.get('n_ran','?')}, edits={last.get('edits_total','?')}); non-gen-1 sweep/regen rows do not gate this floor")
EOF
else
  echo "  score_history.jsonl ABSENT — run: cargo refine --score-bench"
fi

echo ""
echo "── 3. Post-mining intake (mining report progress) ──"
if [ -f "$INTAKE" ]; then
  run_py - "$INTAKE" "$INTAKE_HISTORY" <<'EOF'
import re, sys, os
# E5's gate text: ">=1 post-mining intake class closed end-to-end: intake
# [x] + proposer-change commit + fixture + score-bench delta". A bare tick
# count is NOT that judge: P5's tick is Issue 037's own T2 (outcome-verdict
# WIRING — "one unit, tracked once" under T2), not a proposer-class closure,
# and a gate's own task completions cannot be evidence for the gate itself.
# A ticked item counts only when its entry carries the full closure chain
# AND is not closed-via-an-Issue-037-task attribution.
text = open(sys.argv[1], encoding='utf-8', errors='replace').read()
parts = text.split('## Post-mining intake', 1)
if len(parts) != 2:
    print("  intake section header missing — cannot parse P-items")
    raise SystemExit(0)
body = parts[1]
# 2026-09-13 fold: closed P-item records moved VERBATIM to the history note
# (.research/108, "Folded 2026-09-13" onward) so the snapshot stays under its
# ~400-line budget — parse them there too, else done/open counts silently
# collapse to zero the first fold and E5 reads "no class closed yet" over a
# corpus of landed closures. Counts are stable across the fold by design.
hist_note = sys.argv[2] if len(sys.argv) > 2 else ''
if hist_note and os.path.exists(hist_note):
    body += '\n' + open(hist_note, encoding='utf-8', errors='replace').read()
items = re.split(r'\n(?=- \[.\] P\d)', body)
ticked, open_n, qualifying = [], 0, []
for it in items:
    m = re.match(r'- \[(x| )\] (P\d+)', it)
    if not m:
        continue
    state, pid = m.groups()
    t = re.match(r'- \[.\] P\d+ \*\*(.+?)\*\*', it)
    label = (t.group(1) if t else pid)[:44]
    if state == 'x':
        ticked.append((pid, label, it))
    else:
        open_n += 1
print(f"  intake items: done={len(ticked)} open={open_n}")
HASH = re.compile(r'\b[0-9a-f]{7,40}\b')
for pid, label, entry in ticked:
    low = entry.lower()
    reasons = []
    if re.search(r'(via|under) 037-t\d|tracked in \[?issue 037', low):
        reasons.append('closed via an Issue-037 task (self-evidence — not eligible)')
    if not HASH.search(entry):
        reasons.append('no proposer-change commit ref')
    if 'fixture' not in low:
        reasons.append('no fixture')
    if 'score-bench' not in low and 'score_bench' not in low:
        reasons.append('no score-bench delta')
    if reasons:
        print(f"    {pid} {label}: tick does NOT count — {'; '.join(reasons)}")
    else:
        qualifying.append(pid)
if qualifying:
    print(f"  E5 (>=1 proposer class closed e2e): PASS — {', '.join(qualifying)}")
elif ticked:
    print("  E5 (>=1 proposer class closed e2e): PENDING — tick(s) above lack the proposer closure chain (commit + fixture + score-bench delta)")
else:
    print("  E5 (>=1 proposer class closed e2e): PENDING — no post-mining class closed yet")
EOF
else
  echo "  intake ABSENT at $INTAKE"
fi

echo ""
echo "── 4. Adoption (did agents actually run it? durable trail) ──"
total=0
for r in $REPOS; do
  d="$GIT_ROOT/$r"
  [ -d "$d/.git" ] || continue
  # Word-boundary match: "heal" as a standalone word. A bare --grep='heal'
  # substring-matches "health"/"healthy" — measured 2026-08-26 when five
  # unrelated commits ("healthy neutral/tax", "/health endpoint", "GPU-health")
  # fabricated a 3rd activity day and printed a false A1: PASS.
  n=$(git -C "$d" log --oneline -E --since="$ADOPTION_DATE" --grep='(^|[^[:alpha:]])heal($|[^[:alpha:]])' 2>/dev/null | wc -l | tr -d ' ')
  [ "$n" = "0" ] || echo "  $r: $n heal-commit(s) since $ADOPTION_DATE"
  total=$((total + n))
done
echo "  total heal-related commits since $ADOPTION_DATE: $total"
echo "  A2 gate (>=2 repos with heal commits): see above"

echo ""
echo "── 5. Cadence (A1 — distinct heal-activity days, >=3 in any 14-day window) ──"
# Union of the gate's own two evidence halves:
#   store days  — .refine/trajectories.kv rows' last_seen_at (a stamped row
#                 only exists where the tool RAN and recorded — the
#                 memory-growth half)
#   commit days — git heal-mentioning commits since adoption (CONSUMER
#                 repos only: riir-refine is the tool itself, its dev
#                 commits are development, not adoption evidence)
DAYS_TMP="$(mktemp)"
A1_DUMP="$(mktemp)"
for r in $REPOS; do
  d="$GIT_ROOT/$r"
  if [ -d "$d/.refine/trajectories.kv" ] && [ -x "$HEAL_BIN" ]; then
    RIIR_REFINE_TRAJ_ROOT="$d" "$HEAL_BIN" --dump-trajectories > "$A1_DUMP" 2>/dev/null
    run_py - "$r" "$A1_DUMP" >> "$DAYS_TMP" <<'EOF'
import json, sys, datetime
repo, path = sys.argv[1], sys.argv[2]
for line in open(path, encoding='utf-8', errors='replace'):
    line = line.strip()
    if not line.startswith('{'):
        continue
    try:
        v = json.loads(line)
    except Exception:
        continue
    ts = int(v.get('last_seen_at', 0))
    if ts > 0:
        day = datetime.datetime.fromtimestamp(ts).strftime('%Y-%m-%d')
        print(f"{day}\tstore\t{repo}")
EOF
  fi
  if [ -d "$d/.git" ] && [ "$r" != "riir-refine" ]; then
    git -C "$d" log -E --format="%as" --since="$ADOPTION_DATE" --grep='(^|[^[:alpha:]])heal($|[^[:alpha:]])' 2>/dev/null \
      | while read -r day; do printf '%s\tcommit\t%s\n' "$day" "$r"; done >> "$DAYS_TMP"
  fi
done
run_py - "$DAYS_TMP" <<'EOF'
import sys, datetime
ADOPT = datetime.date(2026, 8, 24)  # adoption day; post-T3 activity only
days = {}
for line in open(sys.argv[1], encoding='utf-8', errors='replace'):
    line = line.strip()
    if not line:
        continue
    parts = line.split('\t')
    if len(parts) != 3:
        continue
    try:
        dd = datetime.date.fromisoformat(parts[0])
    except ValueError:
        continue
    if dd < ADOPT:
        continue
    days.setdefault(dd, set()).add((parts[1], parts[2]))
for dd in sorted(days):
    repos = sorted({r for _, r in days[dd]})
    halves = sorted({h for h, _ in days[dd]})
    print(f"  {dd}: {' + '.join(halves)} — {', '.join(repos[:6])}{' …' if len(repos) > 6 else ''}")
distinct = sorted(days)
best = 0
for i, d0 in enumerate(distinct):
    n = sum(1 for d in distinct[i:] if (d - d0).days <= 13)
    best = max(best, n)
print(f"  distinct days: {len(distinct)} | max within any 14-day window: {best}")
if best >= 3:
    print("  A1 (>=3 distinct days in a 14-day window): PASS")
elif distinct:
    nxt = distinct[-1] + datetime.timedelta(days=1)
    print(f"  A1: PENDING — needs {3 - best} more distinct day(s); earliest close {nxt}")
else:
    print("  A1: PENDING — no heal-activity days recorded since adoption")
EOF
rm -f "$DAYS_TMP" "$A1_DUMP"
