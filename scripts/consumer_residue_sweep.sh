#!/usr/bin/env bash
# Consumer residue sweep — the missing half of the guard-landing cycle.
#
# WHY THIS EXISTS (mining-queue item P-a, filed 2026-08-31)
#
# Landing a proposer guard stops the healer EMITTING a broken class. It does
# nothing about the code the un-guarded healer already emitted and that already
# landed in consumer repos. Today the cycle ends when the fixture + positive
# control go green in this repo, which leaves every swept consumer carrying
# whatever came before the guard — with no record of how much.
#
# That gap is measured, not hypothetical. The `227637e` guards
# (`manual_is_multiple_of` on ambiguous `{integer}` receivers, `redundant_closure`
# over a deref coercion) landed 2026-08-30 and were CORRECT: every one of the
# 32 sites riir-ai found the next day would have been suppressed by them. The
# sites were pre-guard residue. One day separated the landing from an audit
# that found 99 errors across 50 targets, 32 of them this repo's classes.
#
# THE SCREEN IS THE CLAIM (item P-b, closed the same day)
#
#   cargo clippy --workspace --all-features --all-targets --keep-going
#
# Not `cargo check` — both of those classes are accepted by check's typeck and
# rejected by clippy's, so the healer's own `--verify` compile gate is blind to
# them by construction. Not default features — ALL 32 sites sat behind
# non-default features, and the immediately preceding fix of the same E0689
# class recorded `--workspace --all-targets: exit 0` while 16 more live sites
# of that class existed at the same commit.
#
# USAGE
#
#   scripts/consumer_residue_sweep.sh <lint> [<lint> ...]      # full screen
#   scripts/consumer_residue_sweep.sh --grep <regex> <lint>    # tier 1 first
#
# Prints a markdown table for the landing note and exits non-zero when residue
# exists. "Fix or file, but never silently leave it unmeasured."
#
# Tier 1 (`--grep`) is a cheap textual pre-filter that narrows the repo set
# before paying for the compile. It is a NARROWING aid only: a zero-hit grep
# does NOT clear a repo, because the classes worth guarding are exactly the
# ones whose signature is a type relationship rather than a token. When in
# doubt, run without `--grep`.
#
# TWO TRAPS THIS SCRIPT WAS CAUGHT BY AND NOW HANDLES
#
# 1. `cargo clippy` on an up-to-date build emits NOTHING. The first version of
#    this script reported "0 sites" for riir-neuron-db while a live
#    `manual_is_multiple_of` sat in `src/` — the units were fresh from an
#    earlier run, so no lint pass ran and no diagnostic was replayed. A quiet
#    zero from a cached build is indistinguishable from a clean repo. Fixed by
#    `cargo clean -p <each workspace member>` in a sweep-local target dir
#    before screening: local crates re-check, dependency builds stay cached.
#
# 2. The JSON key is `"code":{"code":"clippy::<lint>"}`, not `"code":"<lint>"`,
#    and rustc's own lints carry no `clippy::` prefix. Matching the wrong shape
#    also returns a confident zero.
#
# Both failures pointed the same way — toward "clean" — which is why the
# script now prints the raw per-repo diagnostic total alongside the per-class
# counts. If the total is 0 for a repo that has ANY warning, the instrument is
# broken, not the repo.

set -uo pipefail

REPO_LIST="$(cd "$(dirname "$0")" && pwd)/swept_repos.txt"
GREP_RE=""

case "${1:-}" in
    --grep)
        GREP_RE="${2:?--grep needs a regex}"
        shift 2
        ;;
esac

if [ "$#" -eq 0 ]; then
    echo "usage: $0 [--grep <regex>] <lint> [<lint> ...]" >&2
    exit 2
fi

LINTS=("$@")
[ -f "$REPO_LIST" ] || { echo "missing repo list: $REPO_LIST" >&2; exit 2; }

# ── Repo set ───────────────────────────────────────────────────────────────
REPOS=()
while IFS= read -r line; do
    line="${line%%#*}"
    line="$(printf '%s' "$line" | tr -d '[:space:]')"
    [ -n "$line" ] || continue
    if [ -d "$line" ]; then
        REPOS+=("$line")
    else
        echo "note: skipping absent repo $line" >&2
    fi
done < "$REPO_LIST"

echo "── consumer residue sweep ──"
echo "classes: ${LINTS[*]}"
echo "repos:   ${#REPOS[@]}"
[ -n "$GREP_RE" ] && echo "tier 1:  /$GREP_RE/ (narrowing only — a zero-hit grep does NOT clear a repo)"
echo

# ── Tier 1: textual narrowing ──────────────────────────────────────────────
if [ -n "$GREP_RE" ]; then
    NARROWED=()
    for repo in "${REPOS[@]}"; do
        hits=$(grep -rIlE --include='*.rs' "$GREP_RE" "$repo" 2>/dev/null | grep -vc '/target/' || true)
        printf 'tier1  %-28s %s candidate file(s)\n' "$(basename "$repo")" "${hits:-0}"
        [ "${hits:-0}" -gt 0 ] && NARROWED+=("$repo")
    done
    REPOS=("${NARROWED[@]:-}")
    echo
    [ "${#REPOS[@]}" -eq 0 ] && { echo "tier 1 found no candidates — run without --grep to confirm."; exit 0; }
fi

# ── Tier 2: the screen ─────────────────────────────────────────────────────
#
# A sweep-local CARGO_TARGET_DIR keeps this out of the repo's own target dir,
# so a concurrent session in that repo is neither slowed nor invalidated. It
# persists between sweeps (dependency builds stay warm); only the workspace's
# own crates are invalidated, which is what forces the lint pass to re-run.
SWEEP_TARGET_ROOT="${SWEEP_TARGET_ROOT:-/tmp/riir-clippy-residue-sweep}"
mkdir -p "$SWEEP_TARGET_ROOT"

TOTAL=0
ROWS=""
for repo in "${REPOS[@]}"; do
    [ -n "$repo" ] || continue
    name="$(basename "$repo")"
    target="$SWEEP_TARGET_ROOT/$name"
    echo "screening $name …" >&2

    # Trap 1: invalidate the workspace's OWN crates so clippy actually runs.
    members=$(cd "$repo" && cargo metadata --no-deps --format-version 1 2>/dev/null \
        | grep -oE '"name":"[^"]+"' | sed 's/"name":"//; s/"//' | sort -u)
    for m in $members; do
        (cd "$repo" && CARGO_TARGET_DIR="$target" cargo clean -p "$m" >/dev/null 2>&1)
    done

    json=$(cd "$repo" && CARGO_TARGET_DIR="$target" cargo clippy --workspace \
        --all-features --all-targets --keep-going --message-format=json 2>/dev/null)

    # Instrument liveness: a repo with zero diagnostics of ANY kind after a
    # forced re-check is suspicious, not reassuring.
    any=$(printf '%s' "$json" | grep -c '"level":"warning"' || true)
    printf 'screen  %-28s %s diagnostic(s) emitted\n' "$name" "${any:-0}" >&2

    for lint in "${LINTS[@]}"; do
        # Trap 2: the code lives at `"code":{"code":"clippy::<lint>"}`, and
        # rustc's own lints carry no `clippy::` prefix — accept both. Matched
        # with awk's index() rather than a regex so no shell/regex escaping
        # layer can silently fail to match and report a confident zero.
        #
        # Identity is (first file_name, first byte_start) on the diagnostic's
        # own line: `byte_start` precedes `file_name` in the span object (an
        # adjacency the first version of this script guessed backwards), the
        # primary span precedes any `children` spans, and de-duplicating on
        # the pair collapses the same site re-reported once per compilation
        # unit — a real double-count under --all-features --all-targets.
        n=$(printf '%s' "$json" | awk -v lint="$lint" '
            index($0, "\"code\":{\"code\":\"clippy::" lint "\"") ||
            index($0, "\"code\":{\"code\":\"" lint "\"") {
                f = ""; b = "";
                if (match($0, /"file_name":"[^"]+"/)) f = substr($0, RSTART, RLENGTH);
                if (match($0, /"byte_start":[0-9]+/))  b = substr($0, RSTART, RLENGTH);
                if (f != "") print f b
            }' | sort -u | wc -l | tr -d '[:space:]')
        n="${n:-0}"
        TOTAL=$((TOTAL + n))
        ROWS="${ROWS}| ${name} | \`${lint}\` | ${n} |"$'\n'
    done
done

echo
echo "### Consumer residue — landing note table"
echo
echo "| repo | class | live sites |"
echo "|---|---|---|"
printf '%s' "$ROWS"
echo
echo "screen: \`cargo clippy --workspace --all-features --all-targets --keep-going\`"
echo

if [ "$TOTAL" -gt 0 ]; then
    echo "✗ $TOTAL live site(s) of the guarded class remain in consumer repos."
    echo "  Fix them or file an issue — do NOT close the guard landing with this"
    echo "  count unrecorded. That is the P-a failure mode."
    exit 1
fi

echo "✓ no residue — the guard landing can close with this table in its note."
