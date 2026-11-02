#!/bin/sh
# Mozilla outbound fix-PR wetrun — send mechanics (Plan 329 T5)
# Prereq: `gh auth login` completed by the owner (device flow).
set -eu
echo "== 1. auth check =="
gh auth status
echo "== 2. fork mozilla/sccache into riir-ai =="
gh repo fork mozilla/sccache --org riir-ai --clone=false
echo "== 3. push the branch =="
cd "$(dirname "$0")/../../../.raw/mozilla-heal/sccache"
git remote add fork git@github.com:riir-ai/sccache.git 2>/dev/null || true
git push -u fork refine/cachedir-tag
echo "== 4. open the PR =="
cd "$OLDPWD"
gh pr create --repo mozilla/sccache --head riir-ai:refine/cachedir-tag \
  --title "Tag the disk cache directory with CACHEDIR.TAG" \
  --body-file .benchmarks/127_mozilla_wetrun/pr_body.md
echo "== 5. record the PR URL in .benchmarks/127_mozilla_wetrun/README.md ledger =="
