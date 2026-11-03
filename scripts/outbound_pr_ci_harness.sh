#!/bin/sh
# outbound-pr CI harness (G-PR1b) — reproduce the target repo's CI surfaces
# locally before pushing, and grep the blast radius of the files the diff
# writes/creates. Contract: exit 0 = all runnable CI surfaces green + blast
# radius accounted; 1 = failures (push waits); 2 = harness blindness.
#
# Usage: outbound_pr_ci_harness.sh <clone-dir> [toolchain]
#   BASE_REF   git ref to diff against (default: origin/main)
#   DIFF_WRITTEN_PATHS  space-separated names the diff CREATES that are not
#                       tracked paths (e.g. "CACHEDIR.TAG")
#
# Exit contract:
#   0 = every runnable CI surface green + blast radius accounted
#   1 = at least one surface failed or an unaccounted blast-radius hit
#   2 = harness blindness (no workflows / no toolchain / parse failure)
set -u

CLONE="${1:-}"
TOOLCHAIN="${2:-}"
BASE_REF="${BASE_REF:-origin/main}"
FAIL=0
HITS=0

[ -d "$CLONE" ] || { echo "harness: no clone dir '$CLONE'"; exit 2; }
cd "$CLONE" || exit 2

echo "== harness: $CLONE =="

# Resolve the target toolchain: arg > rust-toolchain.toml channel > rust-version
if [ -z "$TOOLCHAIN" ]; then
    if [ -f rust-toolchain.toml ]; then
        TOOLCHAIN=$(grep -m1 'channel' rust-toolchain.toml | sed 's/.*= *"\(.*\)"/\1/')
    fi
    if [ -z "$TOOLCHAIN" ] && grep -q '^rust-version' Cargo.toml 2>/dev/null; then
        TOOLCHAIN="rust-$(grep -m1 '^rust-version' Cargo.toml | sed 's/.*= *"//;s/"//')"
    fi
fi
[ -n "$TOOLCHAIN" ] || { echo "harness: BLIND - no toolchain resolvable (pass one or declare MSRV)"; exit 2; }
TC="+$TOOLCHAIN"
echo "toolchain: cargo $TC"

# ── 1. CI inventory (python does the parsing; portable, no sed foot-guns) ──
echo "== 1. CI inventory (their workflow files) =="
INVOKE_FILE=$(mktemp)
python3 - "$INVOKE_FILE" << 'PYEOF'
import glob, re, sys

out = sys.argv[1]
cmds = set()
rx = re.compile(r'cargo\s+(?:\+\S+\s+)?(test|clippy|build|check|fmt|hack)\s+([^"\'\n]*)')
for f in glob.glob('.github/workflows/*.y*ml') + glob.glob('.circleci/config.yml'):
    try:
        text = open(f, encoding='utf-8', errors='replace').read()
    except OSError:
        continue
    for line in text.splitlines():
        for m in rx.finditer(line):
            cmd = ('cargo ' + m.group(1) + ' ' + m.group(2)).strip()
            cmd = re.sub(r'\$\{\{[^}]*\}\}', '', cmd)   # CI template vars out
            cmd = cmd.replace('${{', '').replace('}}', '')  # multi-line leftovers
            cmd = re.sub(r'\s+', ' ', cmd).strip()
            if not cmd or cmd.endswith('cargo'):
                continue
            # a trailing bare flag (template var stripped the value) makes the
            # command unrunnable as-written - drop the dangling flag; if that
            # leaves a bare probe, skip it at run time
            for flag in ('--features', '--bin', '--target', '--package'):
                if cmd.endswith(flag):
                    cmd = cmd[: -len(flag)].strip()
            if cmd in ('cargo', 'cargo build', 'cargo check', 'cargo test', 'cargo clippy'):
                continue
            if any(tok in cmd for tok in ('matrix.', 'env:', '$')):
                continue
            cmds.add(cmd)

with open(out, 'w') as fh:
    for c in sorted(cmds):
        fh.write(c + '\n')
PYEOF
if [ ! -s "$INVOKE_FILE" ]; then
    echo "harness: BLIND - no CI invocations found in workflow files"
    rm -f "$INVOKE_FILE"
    exit 2
fi
cat "$INVOKE_FILE"

# ── 2. Execute the runnable set ─────────────────────────────────────────
echo "== 2. Executing runnable CI surfaces (failures collected, verdict at end) =="
echo "(skipped as not-locally-runnable: matrix-parameterized, cross-target, combinatorial, probes - review those by hand)"
while IFS= read -r cmd; do
    [ -n "$cmd" ] || continue
    case "$cmd" in
        *hack*) echo "SKIP (combinatorial): $cmd"; continue ;;
        *--version*) echo "SKIP (probe): $cmd"; continue ;;
        *--target*) echo "SKIP (cross-target): $cmd"; continue ;;
        *windows*|*msvc*) echo "SKIP (windows): $cmd"; continue ;;
        *--all-features*) echo "SKIP (all-features: OS-gated deps; their CI runs it on their runners - review manually): $cmd"; continue ;;
    esac
    case "$cmd" in
        *"fmt --check"*|*"fmt"*)
            echo "--- $cmd"
            if cargo $TC fmt -- --check 2>&1 | tail -2; then :; else echo "HARNESS-FAIL: fmt"; FAIL=1; fi
            continue ;;
    esac
    echo "--- $cmd"
    OUT=$(timeout 1800 cargo $TC ${cmd#cargo } 2>&1)
    RC=$?
    echo "$OUT" | tail -2
    if [ $RC -ne 0 ]; then
        echo "HARNESS-FAIL (rc=$RC): $cmd"
        FAIL=1
    fi
done < "$INVOKE_FILE"
rm -f "$INVOKE_FILE"

# ── 3. Blast-radius grep ────────────────────────────────────────────────
echo "== 3. Blast radius: other readers of the paths this diff writes =="
DIFF_PATHS=$(git diff --name-only "$BASE_REF...HEAD" 2>/dev/null)
CREATED="${DIFF_WRITTEN_PATHS:-}"
echo "diff vs $BASE_REF: $DIFF_PATHS${CREATED:+  + created: $CREATED}"

for name in $CREATED; do
    [ -n "$name" ] || continue
    DIFF_FILES=$(echo "$DIFF_PATHS" | tr '\n' '|')
    READERS=$(grep -rn --include='*.rs' -F "$name" src/ 2>/dev/null \
        | grep -vE "^($DIFF_FILES):" \
        | head -8)
    if [ -n "$READERS" ]; then
        echo "-- other code referencing '$name':"
        echo "$READERS"
        HITS=1
    fi
done
[ "$HITS" -eq 0 ] && echo "(no unaccounted blast-radius hits)"

# ── verdict ─────────────────────────────────────────────────────────────
echo "== harness verdict =="
if [ "$FAIL" -ne 0 ] || [ "$HITS" -ne 0 ]; then
    echo "NOT READY: review the failures/hits above, fix or annotate, re-run."
    exit 1
fi
echo "READY: all runnable CI surfaces green, blast radius accounted."
exit 0
