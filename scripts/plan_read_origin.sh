#!/usr/bin/env bash
# plan_read_origin.sh — origin-first plan reading for the idle-loop dispatch.
#
# WHY (the Plan-346 stale-copy trap, 2026-08-27): the shared checkout of a
# repo can ride a SIBLING branch (e.g. riir-train on sync/p348-*) whose
# `.plans/*.md` differ from the canonical branch — plans landed, plans
# missing, checkbox states wrong. Any dispatcher that reads `.plans/` from
# the working tree (Zed auto_prompt's read_plan_files, or an agent following
# skill queue item 2) then ranks plans against STALE data and can dispatch a
# plan that is already closed on origin, or miss one that is open there.
#
# This script is the canonical read path: fetch origin, resolve the repo's
# default branch from the remote itself, and read `.plans/` ONLY from
# `origin/<default>` — never from the working tree. It never checks out,
# resets, or writes any file; `git fetch` updating remote-tracking refs is
# the only mutation (the same discipline as the boundary-guard "parser
# replay via git show origin/<default>:BOUNDARY.md" technique).
#
# Usage:
#   scripts/plan_read_origin.sh <repo-path> --list          # plan filenames on origin
#   scripts/plan_read_origin.sh <repo-path> <plan-file>     # plan content on origin
#   scripts/plan_read_origin.sh --self-test                 # guard: dirty non-main
#                                                          # checkout still yields
#                                                          # origin contents
#
# Output: requested data on stdout; one log line on stderr:
#   [plan-read] <repo> @ <branch> = <commit-hash> (<what was read>)
# The commit hash is the provenance of every read — cite it when acting on a
# plan ("read plan 346 @ 2a7d5fb1").
#
# Exit codes: 0 ok · 1 no .plans/ on origin · 2 plan not on origin ·
#             3 not a git repo / no remote / fetch failed · 4 self-test failed
#
# Default-branch resolution (first that answers):
#   1. git ls-remote --symref origin HEAD   — authoritative, from the remote
#   2. git symbolic-ref refs/remotes/origin/HEAD — local remote-tracking HEAD
#   3. existence fallback: origin/main, then origin/develop
# (the whole workspace works on develop since the 2026-09-04 riir-train
# flip — before that riir-train worked on main; the remote's own HEAD is
# the one source of truth that needs no per-repo table.)

set -uo pipefail

log() { echo "[plan-read] $*" >&2; }

die() { log "ERROR: $*"; exit 3; }

# Resolve the default branch of repo $1. Echoes the bare branch name.
default_branch() {
    local repo=$1 branch
    # 1. Ask the remote directly (works with a stale/missing local origin/HEAD).
    branch=$(git -C "$repo" ls-remote --symref origin HEAD 2>/dev/null \
        | awk '$1 == "ref:" {sub(/^refs\/heads\//, "", $2); print $2; exit}')
    if [ -n "$branch" ]; then echo "$branch"; return 0; fi
    # 2. Local remote-tracking HEAD.
    branch=$(git -C "$repo" symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null \
        | sed 's|^origin/||')
    if [ -n "$branch" ]; then echo "$branch"; return 0; fi
    # 3. Existence fallback.
    for branch in main develop; do
        if git -C "$repo" show-ref --verify --quiet "refs/remotes/origin/$branch"; then
            echo "$branch"; return 0
        fi
    done
    return 1
}

# Plans directory on origin: prefer .plans/, fall back to .plan/ (Zed's
# read_plan_files checks both; this workspace uses .plans/).
plans_dir_on() { # <repo> <origin-ref>
    local repo=$1 ref=$2 dir
    for dir in .plans .plan; do
        if git -C "$repo" ls-tree --name-only "$ref" -- "$dir/" 2>/dev/null | grep -q .; then
            echo "$dir"; return 0
        fi
    done
    return 1
}

read_plans() { # <repo>
    local repo=$1
    git -C "$repo" rev-parse --is-inside-work-tree >/dev/null 2>&1 \
        || die "$repo is not a git repo"
    git -C "$repo" remote get-url origin >/dev/null 2>&1 \
        || die "$repo has no 'origin' remote"
    git -C "$repo" fetch origin --quiet >/dev/null 2>&1 \
        || die "git fetch origin failed in $repo"
    local branch hash dir
    branch=$(default_branch "$repo") || die "cannot resolve origin default branch for $repo"
    hash=$(git -C "$repo" rev-parse --short "origin/$branch") \
        || die "cannot resolve origin/$branch"
    dir=$(plans_dir_on "$repo" "origin/$branch") \
        || { log "$repo @ $branch = $hash — no .plans/ (or .plan/) on origin"; exit 1; }
    REPO=$repo BRANCH=$branch HASH=$hash DIR=$dir
}

REPO="" BRANCH="" HASH="" DIR=""

if [ "${1:-}" = "--self-test" ]; then
    # ── Guard test ─────────────────────────────────────────────────────────
    # Proves: on a DIRTY, NON-default-branch checkout whose working tree
    # carries different plan content AND a sibling-only plan, selection via
    # this script still yields exactly origin/<default>'s contents + hash.
    # The negative controls assert the fixture CAN fail (working tree really
    # differs) so a green run means something.
    # ────────────────────────────────────────────────────────────────────────
    self_test() {
        local tmp repo script
        script=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
        tmp=$(mktemp -d /tmp/plan-read-selftest.XXXXXX)
        repo=$tmp/repo

        # Origin: a bare repo whose HEAD is main.
        git init --bare --quiet --initial-branch=main "$tmp/origin.git"
        git init --quiet --initial-branch=main "$repo"
        git -C "$repo" config user.email t@t.t
        git -C "$repo" config user.name t
        mkdir -p "$repo/.plans"
        printf '# plan 001 (origin/main)\n- [ ] MAIN-ONLY-TASK\n' > "$repo/.plans/001_fixture.md"
        git -C "$repo" add -A
        git -C "$repo" commit --quiet -m main
        git -C "$repo" remote add origin "$tmp/origin.git"
        git -C "$repo" push --quiet origin main

        # Sibling branch: different content in 001 + a sibling-only plan 002.
        git -C "$repo" checkout --quiet -b sibling-branch
        printf '# plan 001 (SIBLING)\n- [ ] SIBLING-TASK\n' > "$repo/.plans/001_fixture.md"
        printf '# plan 002 (sibling only)\n- [ ] SIBLING-ONLY-PLAN\n' > "$repo/.plans/002_sibling_only.md"
        git -C "$repo" add -A
        git -C "$repo" commit --quiet -m sibling
        # ...and make the tree dirty on top of the sibling branch.
        printf 'uncommitted noise\n' >> "$repo/.plans/001_fixture.md"

        local fails=0
        check() { # <name> <condition-exit-code>
            if [ "$2" -eq 0 ]; then
                echo "  ok   $1"
            else
                echo "  FAIL $1"; fails=$((fails+1))
            fi
        }

        local out err hash_main
        hash_main=$(git -C "$repo" rev-parse --short origin/main)

        # (1) Read yields origin/main content, not the sibling working tree.
        err=$(cd / && "$script" "$repo" .plans/001_fixture.md 2>&1 >/dev/null)
        out=$(cd / && "$script" "$repo" .plans/001_fixture.md 2>/dev/null)
        grep -q 'MAIN-ONLY-TASK' <<<"$out"; check "read returns origin/main content" $?
        ! grep -q 'SIBLING-TASK' <<<"$out"; check "sibling content absent from read" $?
        ! grep -q 'uncommitted noise' <<<"$out"; check "dirty-tree noise absent from read" $?

        # (2) The log line cites the origin/main commit hash.
        grep -q "\[plan-read\] $repo @ main = $hash_main" <<<"$err"; check "log cites origin/main hash" $?

        # (3) Listing is origin/main's set — sibling-only plan invisible.
        out=$(cd / && "$script" "$repo" --list 2>/dev/null)
        grep -q '001_fixture.md' <<<"$out"; check "list includes origin plan" $?
        ! grep -q '002_sibling_only.md' <<<"$out"; check "sibling-only plan not listed" $?

        # (4) Negative control: the working tree REALLY differs (fixture can fail).
        grep -q 'SIBLING-TASK' "$repo/.plans/001_fixture.md"; check "negative control: working tree is sibling+dirty" $?

        # (5) Reading a sibling-only plan refuses (exit 2), not silently empty.
        (cd / && "$script" "$repo" .plans/002_sibling_only.md >/dev/null 2>&1); [ $? -eq 2 ]
        check "sibling-only plan read exits 2" $?

        rm -rf "$tmp"
        if [ "$fails" -gt 0 ]; then
            log "SELF-TEST FAILED ($fails check(s))"
            return 4
        fi
        log "SELF-TEST PASS (dirty non-main checkout yields origin/main contents)"
        return 0
    }
    self_test
    exit $?
fi

[ $# -ge 2 ] || { echo "usage: $0 <repo-path> (--list | <plan-file>)" >&2; exit 3; }

REPO_ARG=$1
WHAT=$2
read_plans "$REPO_ARG"

case $WHAT in
    --list)
        git -C "$REPO" ls-tree --name-only "origin/$BRANCH" -- "$DIR/" \
            | grep -E '\.md$' || true
        log "$REPO @ $BRANCH = $HASH (list $DIR/)"
        ;;
    *)
        FILE=$WHAT
        case $FILE in
            .plans/*|.plan/*) : ;;
            *) FILE="$DIR/$FILE" ;;
        esac
        git -C "$REPO" cat-file -e "origin/$BRANCH:$FILE" 2>/dev/null || {
            log "$REPO @ $BRANCH = $HASH — $FILE not on origin (working-tree-only? sibling branch?)"
            exit 2
        }
        git -C "$REPO" show "origin/$BRANCH:$FILE"
        log "$REPO @ $BRANCH = $HASH ($FILE)"
        ;;
esac
