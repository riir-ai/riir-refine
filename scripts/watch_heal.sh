#!/usr/bin/env bash
# Plan 019 T5.1 — the real-use watch loop over `cargo heal --fix-compile`.
#
# USAGE:
#   ./scripts/watch_heal.sh                    # watch session-touched .rs files
#   ./scripts/watch_heal.sh src/foo.rs ...     # watch an explicit file list
#   HEAL_FEATURES="--features a,b" ./scripts/watch_heal.sh
#
# REQUIREMENTS encoded (T5.2 — these are the three recorded caveats as
# REQUIREMENTS of any wrapper, not suggestions):
#
# 1. WRITE SEMANTICS: `--fix-compile` never writes without `--write`
#    (pinned by `review_mode_plans_without_writing`). This script passes
#    --write deliberately — every run applies what the oracle keeps.
#
# 2. FEATURE-GATE BLINDNESS: the oracle is `cargo check`; a fix that
#    compiles under default features can break a gated config (the
#    Issue 031 H3 class). `--verify-args` MUST carry the repo's real
#    feature set. Set HEAL_FEATURES (default: --all-features, the safe
#    superset; narrow it for speed once you know your repo's shape).
#
# 3. SIBLING AGENTS: NEVER whole-repo scoping on a shared checkout
#    (the recorded sibling-collision discipline). Default scope =
#    SESSION-TOUCHED files only: `git diff --name-only HEAD` +
#    untracked .rs files. Pass an explicit list to narrow further.
#
# ── CARGO_TARGET_DIR isolation (T5.3) ─────────────────────────────────────
# If any sibling agent/cargo is already running against this repo, rustc
# will block on the target-dir lock. Export a private target dir BEFORE
# launching:
#   export CARGO_TARGET_DIR=/tmp/riir_heal_watch_$$
# (This script does NOT set it for you: the heal oracle shares the target
# dir with your other builds by default, which is the fast path when idle
# and the contention point when not. The global rule: check for running
# cargo first; isolate per-session when concurrent.)
#
# ── Store paths (the Phase 4 amendment) ───────────────────────────────────
# Self-evolution state lives at <repo>/.heal/:
#   trajectories.kv/  — the durable neuron-db store (latent_retrieval
#                       builds; encrypted WAL + BM25 + 8-D axis)
#   memory.json       — the JSON-envelope fallback (default builds)
#   misses/           — the template-authoring queue (T4.4)
# There is NO fix_log.jsonl append — the store is the one source of truth;
# `cargo heal --dump-trajectories` is the export surface (T4.3b).

set -euo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# The heal command. Default: the installed `cargo-heal` binary (the
# distribution surface). Override for a dev build, e.g.:
#   HEAL_BIN="cargo run --features rustc_errors --bin cargo-heal --"
# (split on words deliberately — it is invoked unquoted below.)
HEAL_BIN="${HEAL_BIN:-cargo-heal}"
# --verify-args carries CARGO args (appended to `cargo check --all-targets
# --message-format=json`), not a command — the feature set only.
HEAL_FEATURES="${HEAL_FEATURES-}"
[ -n "$HEAL_FEATURES" ] || HEAL_FEATURES="--all-features"
HEAL_ARGS=(--fix-compile --write --verify-args "$HEAL_FEATURES")

# ── Scope: session-touched files, NEVER the whole repo (caveat 3) ─────────
# (portable collection — macOS still ships bash 3.2, no mapfile)
WATCH_FILES=()
if [ "$#" -gt 0 ]; then
    for f in "$@"; do WATCH_FILES+=("$f"); done
else
    while IFS= read -r f; do
        [ -n "$f" ] && WATCH_FILES+=("$f")
    done < <(
        { git diff --name-only HEAD -- '*.rs' 2>/dev/null || true
          git ls-files --others --exclude-standard -- '*.rs' 2>/dev/null || true
        } | sort -u
    )
fi

if [ "${#WATCH_FILES[@]}" -eq 0 ]; then
    echo "watch_heal: no session-touched .rs files (clean tree) — nothing to watch."
    echo "  Pass an explicit file list to force: ./scripts/watch_heal.sh path/to/file.rs"
    exit 0
fi

echo "watch_heal: ${#WATCH_FILES[@]} file(s) in scope:"
printf '  %s\n' "${WATCH_FILES[@]}"
echo "oracle args: ${HEAL_ARGS[*]}"
echo

run_heal() {
    echo "── $(date '+%H:%M:%S') heal pass ──"
    # HEAL_BIN is word-split deliberately (it may carry cargo-run prefixes).
    # shellcheck disable=SC2086
    $HEAL_BIN "${HEAL_ARGS[@]}" "${WATCH_FILES[@]}" || true
}

# ── First pass immediately, then watch (watchexec preferred) ──────────────
run_heal

# The WATCH TRIGGER may be the tree (that's what watchexec/cargo-watch
# do); the HEAL SCOPE stays the session-touched list above — that is the
# sibling-safety requirement. A change outside the scope re-runs heal on
# the SAME scoped files, never the whole repo.
HEAL_CMDLINE="$HEAL_BIN $(printf '%q ' "${HEAL_ARGS[@]}" "${WATCH_FILES[@]}")"

if command -v watchexec >/dev/null 2>&1; then
    echo "watching (watchexec, scope locked to the list above) — Ctrl-C to stop"
    exec watchexec --restart -e rs -- bash -c "$HEAL_CMDLINE || true"
elif command -v cargo-watch >/dev/null 2>&1 \
    || cargo watch --version >/dev/null 2>&1; then
    echo "watching (cargo watch, scope locked to the list above) — Ctrl-C to stop"
    exec cargo watch --quiet --shell "$HEAL_CMDLINE || true"
else
    cat <<'EOF'
watch_heal: neither watchexec nor cargo-watch found — one-shot mode only.

Install one of:
  cargo install watchexec    # preferred
  cargo install cargo-watch
EOF
    exit 0
fi
