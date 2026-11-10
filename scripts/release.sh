#!/bin/sh
# ─────────────────────────────────────────────────────────────────────────────
# release.sh — the touch-deploy release publisher (ONE fingerprint, no
# handcrafted kat-secret lines).
#
#   scripts/release.sh v0.5.1
#
# Composes the proven lane end to end:
#   1. posture guards (clean tree, HEAD tagged, Cargo.toml == tag, notes ready)
#   2. the full pipeline  = scripts/manual_release.sh "$TAG" --no-publish
#      (licenses, 5-target dist build, G2 leak scan, size gate, SHA256SUMS)
#   3. the smoke          = the built binary's `--version` must name the tag's
#      version with a complete release set (the skill's lesson: smoke BEFORE
#      uploading — manual_release builds the working tree, so the tag guard
#      in step 1 is what makes "the build" and "the tag" the same bytes)
#   4. the publish        = kat-secret exec gh-release-pat --env GH_TOKEN —
#      ONE Touch ID prompt, the token never on argv/in a file/in the log
#
# Prereqs: gh (the CLI binary — auth comes from the sealed token), kat-secret
# with `gh-release-pat` sealed (the owner sealed it; `kat-secret list` shows
# it), cargo-zigbuild + zig + shasum (checked by manual_release.sh), and
# release/notes-<TAG>.md carrying a `corpus:` line.
#
# NOT done here (unchanged from manual_release.sh): the G1 clean-machine
# install probes against the published assets; the homebrew-tap + scoop
# manifest bumps (sha256 come from /tmp/manual_release_dist/SHA256SUMS);
# LICENSE acceptance for NEW deps; the crates.io lane (scripts/
# cratesio_publish.sh under `kat-secret exec p1-cascade`, issue 221).
# ─────────────────────────────────────────────────────────────────────────────
set -eu

TAG="${1:?usage: $0 <tag>  (e.g. v0.5.1)}"
case "$TAG" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "tag must look like v0.5.1" >&2; exit 2 ;;
esac
[ -f .github/about/about.toml ] || { echo "run from the riir-refine repo root" >&2; exit 2; }

VER="${TAG#v}"

echo "== posture (the build is the working tree — it must BE the tag)"
[ -z "$(git status --porcelain)" ] || {
    echo "working tree is dirty — the build would ship someone's WIP; commit (or clean) first" >&2
    exit 1
}
[ "$(git describe --exact-match --tags HEAD 2>/dev/null)" = "$TAG" ] || {
    echo "HEAD is not tagged $TAG — tag the release commit first:" >&2
    echo "  git tag $TAG && git push origin $TAG   (then re-run)" >&2
    exit 1
}
[ "$(sed -n 's/^version = \"\(.*\)\"$/\1/p' Cargo.toml | head -1)" = "$VER" ] || {
    echo "Cargo.toml version != $VER — bump it (and the notes file) before tagging" >&2
    exit 1
}

command -v gh > /dev/null || { echo "gh missing (brew install gh) — the binary is needed; auth comes from the sealed token" >&2; exit 2; }
command -v kat-secret > /dev/null || { echo "kat-secret missing (~/.kat/bin, riir-auth Issue 012)" >&2; exit 2; }
kat-secret list 2>/dev/null | grep -qx 'gh-release-pat' || {
    echo "gh-release-pat is NOT sealed — the owner seals it: kat-secret seal gh-release-pat" >&2
    exit 2
}

NOTES="release/notes-$TAG.md"
[ -f "$NOTES" ] || { echo "missing $NOTES — draft the release body first (a \`corpus:\` per-domain rule-delta line; the Issue 153 T5 convention)" >&2; exit 1; }
grep -q '^corpus:' "$NOTES" || { echo "$NOTES carries no \`corpus:\` line — the update alert quotes it, so it must be real" >&2; exit 1; }

echo "== build + package (manual_release.sh --no-publish: licenses, 5 targets, leak scan, size gate, SHA256SUMS)"
sh scripts/manual_release.sh "$TAG" --no-publish

echo "== smoke the built binary BEFORE uploading (the aarch64 arm the owner runs)"
./target/aarch64-apple-darwin/dist/cargo-refine --version
./target/aarch64-apple-darwin/dist/cargo-refine --version | grep -q "cargo-refine $VER" || {
    echo "the built binary does not report $VER — stop, do not upload" >&2
    exit 1
}
if ./target/aarch64-apple-darwin/dist/cargo-refine --version | grep -q 'STALE'; then
    echo "the built binary reports a STALE release set — rebuild before uploading" >&2
    exit 1
fi

CORPUS_LINE="$(grep '^corpus:' "$NOTES" | head -1)"
echo "== publish — Touch ID prompt incoming for gh-release-pat -> gh release create $TAG"
kat-secret exec gh-release-pat --env GH_TOKEN \
    --why "publish cargo-refine $TAG" -- \
    gh release create "$TAG" -R riir-ai/cargo-refine \
        --title "cargo-refine $TAG — $CORPUS_LINE" \
        --notes-file "$PWD/$NOTES" \
        /tmp/manual_release_dist/*

echo "== next (manual, unchanged): tap/scoop bumps from /tmp/manual_release_dist/SHA256SUMS;"
echo "   crates.io lane = scripts/cratesio_publish.sh under kat-secret exec p1-cascade (issue 221)"
echo "DONE: https://github.com/riir-ai/cargo-refine/releases/tag/$TAG"
