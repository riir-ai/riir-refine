#!/bin/sh
# Mozilla outbound fix-PR wetrun — send mechanics (Plan 329 T5)
# Prereq: `gh auth login` completed by the owner (device flow).
# Idempotent: safe to re-run (fork/remote/PR steps detect prior state).
set -eu

REFINE_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCCACHE_CLONE="$REFINE_ROOT/.raw/mozilla-heal/sccache"

echo "== 1. auth check =="
gh auth status

echo "== 2. fork mozilla/sccache into riir-ai (idempotent) =="
# Already-forked returns a message and exits 0; an org-permission failure stops here.
gh repo fork mozilla/sccache --org riir-ai --clone=false

echo "== 3. push the branch =="
cd "$SCCACHE_CLONE"
if ! git remote get-url fork >/dev/null 2>&1; then
    git remote add fork git@github.com:riir-ai/sccache.git
fi
git push -u fork refine/cachedir-tag

echo "== 4. open the PR =="
cd "$REFINE_ROOT"
PR_URL="$(gh pr create --repo mozilla/sccache --head riir-ai:refine/cachedir-tag \
  --title "Tag the disk cache directory with CACHEDIR.TAG" \
  --body-file "$REFINE_ROOT/.benchmarks/127_mozilla_wetrun/pr_body.md")"
echo "PR_URL=$PR_URL"

echo "== 5. done — record the URL in .benchmarks/127_mozilla_wetrun/README.md ledger + issue 198 acceptance =="
