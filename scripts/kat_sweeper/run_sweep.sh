#!/usr/bin/env bash
# run_sweep.sh — the KAT own-repo sweeper runner (Plan 031 Phase A / A2)
# + the daily trainer-drain wrapper (A3/A4 share the cron surface).
#
# Modes:
#   run_sweep.sh                 — sweep every target due by cadence
#   run_sweep.sh --only <repo>   — sweep one target (cadence still honored)
#   run_sweep.sh --drain [...]   — the daily trainer window (A3): builds +
#                                  runs `kat_drain`, extra args pass through
#   run_sweep.sh --list          — print the parsed target table, exit
#
# THE REVIEW-BRANCH RULE (never-wrong at the repo level): findings land on
# `heal/<repo>-<yyyymmdd>` branches, NEVER committed to develop, NEVER
# pushed. The loop's job is to surface reviewable mechanical fixes; merging
# is a human act.
#
# Safety: a target with a dirty working tree is SKIPPED (recorded, never
# stashed — the global rule: sibling WIP is not ours to sweep aside).
#
# Measurement (A4): every run row carries wall + CPU (user+sys) + peak RSS
# via /usr/bin/time -l (macOS reports BYTES — not KB like Linux), appended to
#   <sweeper-home>/.refine/kat_sweeper_runs.jsonl   (version-stamped)
#
# Environment:
#   KAT_SWEEPER_HEAL_BIN   healer binary (default ~/.cargo/bin/cargo-refine,
#                           must be built with fix_verify+clippy_verify)
#   KAT_SWEEPER_CONFIG     config path (default beside this script)
#   KAT_SWEEPER_DRY        1 = parse + print targets, no heal/commit
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWEEPER_HOME="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG="${KAT_SWEEPER_CONFIG:-$SCRIPT_DIR/kat_sweeper.toml}"
RUNS_FILE="$SWEEPER_HOME/.refine/kat_sweeper_runs.jsonl"
RUNS_VERSION=1
HEAL_BIN="${KAT_SWEEPER_HEAL_BIN:-$HOME/.cargo/bin/cargo-refine}"
LOG_DIR="$SWEEPER_HOME/.refine/kat_sweeper_logs"
DRAIN_TARGET_DIR="${KAT_SWEEPER_DRAIN_TARGET:-/tmp/kat_clippy_p31a}"
mkdir -p "$SWEEPER_HOME/.refine" "$LOG_DIR"

MODE="sweep"
ONLY=""
PASS_THROUGH=()
while [ $# -gt 0 ]; do
  case "$1" in
    --drain) MODE="drain" ;;
    --only)  ONLY="${2:-}"; shift ;;
    --list)  MODE="list" ;;
    *)       PASS_THROUGH+=("$1") ;;
  esac
  shift
done

now_unix() { date +%s; }
today_ymd() { date +%Y%m%d; }
today_iso() { date -u +%Y-%m-%dT%H:%M:%SZ; }

# ── config parsing (the documented minimal-TOML subset) ────────────────────
# Emits one TSV line per [[target]]: repo \t path \t scope \t features \t
# branch \t cadence. `version` is validated, not carried.
parse_targets() {
  awk '
    BEGIN { FS = "="; intarget = 0 }
    /^[[:space:]]*version[[:space:]]*=/ {
      v = $2; gsub(/[ "]/, "", v)
      if (v != 1) { print "BAD_VERSION:" v > "/dev/stderr"; exit 3 }
      next
    }
    /^[[:space:]]*\[\[target\]\]/ {
      if (intarget) emit()
      repo = path = scope = features = branch = cadence = ""
      intarget = 1
      next
    }
    intarget && $1 ~ /^[[:space:]]*(repo|path|scope|features|branch|cadence)[[:space:]]*$/ {
      k = $1; gsub(/[ \t]/, "", k)
      v = $0; sub(/^[^=]*=/, "", v); gsub(/^[ \t"]+|[ \t"]+$/, "", v)
      if (k == "repo") repo = v
      else if (k == "path") path = v
      else if (k == "scope") scope = v
      else if (k == "features") features = v
      else if (k == "branch") branch = v
      else if (k == "cadence") cadence = v
      next
    }
    END { if (intarget) emit() }
    function emit() {
      if (repo == "" || path == "" || scope == "") {
        print "BAD_TARGET (need repo+path+scope): " repo > "/dev/stderr"; exit 3
      }
      if (cadence == "") cadence = "hourly"
      print repo "\t" path "\t" scope "\t" features "\t" branch "\t" cadence
    }
  ' "$CONFIG"
}

# ── run-row writer (append-only JSONL; every value is script-controlled —
#    no free-form text ever enters a row) ────────────────────────────────────
record_row() {
  # $1 kind  $2 repo  $3 branch  $4 scope  $5 exit  $6 wall_s  $7 cpu_s
  # $8 rss_bytes  $9 findings  $10 keeps  $11 declines/reverts  $12 commit
  printf '{"v":%d,"ts":"%s","ts_unix":%s,"kind":"%s","repo":"%s","branch":"%s","scope":"%s","exit":%s,"wall_sec":%s,"cpu_sec":%s,"peak_rss_bytes":%s,"findings":%s,"keeps":%s,"reverts_or_declines":%s,"commit":"%s"}\n' \
    "$RUNS_VERSION" "$(today_iso)" "$(now_unix)" "$1" "$2" "$3" "$4" \
    "$5" "$6" "$7" "$8" "$9" "${10}" "${11}" "${12}" >> "$RUNS_FILE"
}

# Parse a `/usr/bin/time -l` report file into TIME_WALL / TIME_CPU / TIME_RSS
# (globals). macOS layout (fields on ONE line: value real, user-value user,
# sys-value sys — so user is $3 + sys is $5 on the `real` line):
#     22.36 real   19.12 user   3.11 sys
#     12345678  maximum resident set size   (BYTES)
parse_time_report() {
  TIME_WALL="$(awk '$2 == "real" {print $1; exit}' "$1")"
  TIME_CPU="$(awk '$2 == "real" {printf "%.2f", $3 + $5; exit}' "$1")"
  TIME_RSS="$(awk '/maximum resident set size/ {print $1; exit}' "$1")"
  TIME_WALL="${TIME_WALL:-0}"; TIME_CPU="${TIME_CPU:-0}"; TIME_RSS="${TIME_RSS:-0}"
}

# ── cadence gate: `daily` targets run once per UTC day (last row wins) ──────
due_by_cadence() {
  # $1 repo  $2 cadence → 0 if due, 1 if already run today
  [ "$2" = "hourly" ] && return 0
  local last last_ts today_start
  last="$(grep "\"repo\":\"$1\"" "$RUNS_FILE" 2>/dev/null | tail -1)"
  [ -z "$last" ] && return 0
  last_ts="$(printf '%s' "$last" | sed -n 's/.*"ts_unix":\([0-9]*\).*/\1/p')"
  [ -z "$last_ts" ] && return 0
  today_start="$(date -u +%s | awk '{printf "%d", int($1/86400)*86400}')"
  [ "$last_ts" -lt "$today_start" ] && return 0
  return 1
}

# ── one sweep ───────────────────────────────────────────────────────────────
sweep_one() {
  local repo="$1" path="$2" scope="$3" features="$4" branch_prefix="$5" cadence="$6"
  local branch="${branch_prefix}-$(today_ymd)"
  local stamp; stamp="$(today_ymd)-$(now_unix)"
  local log="$LOG_DIR/${repo}-${stamp}.log"

  if ! due_by_cadence "$repo" "$cadence"; then
    echo "[sweep] $repo: cadence=$cadence already run today — skipping"
    return 0
  fi
  echo "[sweep] $repo: scope=$scope branch=$branch"

  if ! cd "$path" 2>/dev/null; then
    echo "[sweep] $repo: SKIP — cannot cd to $path"
    record_row sweep "$repo" "$branch" "$scope" 2 0 0 0 0 0 0 ""
    return 0
  fi
  # Dirty tree = someone's WIP. NEVER stash (global rule). Skip + record.
  if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    echo "[sweep] $repo: SKIP — working tree dirty (WIP protection; never stashed)"
    record_row sweep "$repo" "$branch" "$scope" 3 0 0 0 0 0 0 ""
    cd "$SWEEPER_HOME"
    return 0
  fi
  # Update develop first (the base every review branch grows from).
  git checkout -q develop 2>>"$log"
  if ! git pull --rebase origin develop >>"$log" 2>&1; then
    echo "[sweep] $repo: SKIP — git pull --rebase failed (see $log)"
    record_row sweep "$repo" "$branch" "$scope" 4 0 0 0 0 0 0 ""
    cd "$SWEEPER_HOME"
    return 0
  fi
  # Review branch: reuse today's if a prior run created it, else create.
  if git show-ref --verify --quiet "refs/heads/$branch"; then
    git checkout -q "$branch" >>"$log" 2>&1
  else
    git checkout -q -b "$branch" >>"$log" 2>&1
  fi

  # The healer, scoped + compile-gated. /usr/bin/time -l writes its report
  # to stderr — the same file as the healer's, separated below.
  local feat_args=()
  [ -n "$features" ] && feat_args=("--verify-args" "$features")
  /usr/bin/time -l "$HEAL_BIN" --fix --write --verify "${feat_args[@]}" "$scope" \
    >"$log" 2>"$log.err"
  local heal_rc=$?
  parse_time_report "$log.err"
  cat "$log.err" >>"$log"; rm -f "$log.err"

  # Parse the healer's verify summary lines (src/bin/heal.rs):
  #   "verify: N file(s) / M edit(s) — baseline + compile-gated apply"
  #   "verify: K cargo check(s) — kept X file(s) / Y edit(s), reverted Z edit(s)"
  local findings kept reverted
  findings="$(sed -n 's/.*verify: [0-9]* file(s) \/ \([0-9]*\) edit(s).*/\1/p' "$log" | tail -1)"
  kept="$(sed -n 's/.*kept [0-9]* file(s) \/ \([0-9]*\) edit(s), reverted \([0-9]*\) edit(s).*/\1/p' "$log" | tail -1)"
  reverted="$(sed -n 's/.*kept [0-9]* file(s) \/ \([0-9]*\) edit(s), reverted \([0-9]*\) edit(s).*/\2/p' "$log" | tail -1)"
  findings="${findings:-0}"; kept="${kept:-0}"; reverted="${reverted:-0}"

  # Commit kept fixes ON THE REVIEW BRANCH (never develop, never pushed).
  local commit=""
  if git status --porcelain -- "$scope" | grep -q .; then
    git add -- "$scope"
    if git commit -q -m "fix: kat-sweeper heal — $scope (${kept} kept, ${reverted} reverted) [$(today_ymd)]" >>"$log" 2>&1; then
      commit="$(git rev-parse --short HEAD)"
      echo "[sweep] $repo: committed $commit on $branch (${kept} kept / ${reverted} reverted)"
    fi
  else
    echo "[sweep] $repo: clean scope (${findings} finding(s), nothing kept)"
  fi

  record_row sweep "$repo" "$branch" "$scope" "$heal_rc" "$TIME_WALL" "$TIME_CPU" \
    "$TIME_RSS" "$findings" "$kept" "$reverted" "$commit"
  cd "$SWEEPER_HOME"
}

# ── the drain (A3): build + run kat_drain, wrap with /usr/bin/time (A4) ─────
run_drain() {
  local log="$LOG_DIR/drain-$(today_ymd)-$(now_unix).log"
  echo "[drain] building kat_drain (release, trainer tier: cf_workers_ai + groq_first) → $DRAIN_TARGET_DIR"
  if ! CARGO_TARGET_DIR="$DRAIN_TARGET_DIR" cargo build --release -p riir-refine-trainer \
        --features cf_workers_ai,groq_first --bin kat_drain >>"$log" 2>&1; then
    echo "[drain] BUILD FAILED (see $log)"
    record_row drain riir-refine - kat_drain 5 0 0 0 0 0 0 ""
    return 1
  fi
  /usr/bin/time -l "$DRAIN_TARGET_DIR/release/kat_drain" "${PASS_THROUGH[@]}" \
    >"$log.out" 2>"$log.err"
  local rc=$?
  parse_time_report "$log.err"
  cat "$log.out" "$log.err" >>"$log" && rm -f "$log.out" "$log.err"

  # Pull the drain's own summary numbers for the run row (the authoritative
  # cost/quality row is the llm_cost_ledger.jsonl one kat_drain writes).
  local queued keeps declines
  queued="$(sed -n 's/.*backlog: \([0-9]*\) queued.*/\1/p' "$log" | tail -1)"
  keeps="$(sed -n 's/.*| DRAIN | [0-9]* | .* | \([0-9]*\) | .*/\1/p' "$log" | tail -1)"
  declines="$(sed -n 's/.*declined \([0-9]*\).*/\1/p' "$log" | tail -1)"
  queued="${queued:-0}"; keeps="${keeps:-0}"; declines="${declines:-0}"
  echo "[drain] exit=$rc queued=$queued — log $log"
  record_row drain riir-refine - kat_drain "$rc" "$TIME_WALL" "$TIME_CPU" "$TIME_RSS" \
    "$queued" "$keeps" "$declines" ""
}

# ── dispatch ────────────────────────────────────────────────────────────────
case "$MODE" in
  list)
    echo "repo	path	scope	features	branch	cadence"
    parse_targets
    exit 0
    ;;
  drain)
    run_drain
    exit $?
    ;;
  sweep)
    if [ "${KAT_SWEEPER_DRY:-0}" = "1" ]; then
      echo "[sweep] DRY — targets:"
      parse_targets
      exit 0
    fi
    if [ ! -x "$HEAL_BIN" ]; then
      echo "FATAL: healer binary not executable at $HEAL_BIN (KAT_SWEEPER_HEAL_BIN overrides; build: cargo build --release --features fix_verify,clippy_verify)"
      exit 1
    fi
    while IFS=$'\t' read -r repo path scope features branch cadence; do
      [ -n "$ONLY" ] && [ "$repo" != "$ONLY" ] && continue
      sweep_one "$repo" "$path" "$scope" "$features" "$branch" "$cadence"
    done < <(parse_targets)
    echo "[sweep] done — rows append to $RUNS_FILE"
    ;;
esac
