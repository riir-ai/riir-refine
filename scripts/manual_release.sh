#!/bin/sh
# ─────────────────────────────────────────────────────────────────────────────
# Manual (workstation) release — the CI-release-pipeline twin, run from local.
#
# WHY THIS EXISTS: the Actions billing block (2026-09-08) refused every job of
# the tag-triggered pipeline; the owner's standing instruction is "manual
# deploy from local" while billing is broken. This script IS the pipeline,
# locally: same feature set, same packaging, same G2 leak scan, same SHA256
# discipline — then `gh release create` + the tap/scoop manifest bumps.
#
# Usage:  scripts/manual_release.sh v0.1.3
#
# Prereqs (checked): gh auth, cargo-zigbuild, zig, rustup musl targets,
# shasum. Windows builds via zig's gnu target (msvc needs CI — the release
# notes say so; scoop follows the gnu asset here).
#
# Steps this does NOT do: the G1 clean-machine install probes (run those by
# hand against the published assets — Plan 105 T3.1); LICENSE acceptance for
# NEW deps (extend .github/about/about.toml after an actual read first).
#
# The build is --profile dist (strip + fat LTO + opt-level s) — the SAME
# profile release.yml builds; plain --release measured 11.0 MB vs the dist
# block's 4.3 MB on the arm64 shipping set (Cargo.toml [profile.dist]).
# The feature set is PINNED by the release_set_matches_the_manual_script
# test (src/build_stamp.rs) — editing FEATURES reds the lib tests until it
# matches RELEASE_FEATURES again.
# ─────────────────────────────────────────────────────────────────────────────
set -eu

TAG="${1:?usage: $0 <tag>  (e.g. v0.1.3)}"
case "$TAG" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "tag must look like v0.1.3" >&2; exit 2 ;;
esac
[ -f .github/about/about.toml ] || { echo "run from the riir-refine repo root" >&2; exit 2; }

command -v gh > /dev/null || { echo "gh missing" >&2; exit 2; }
command -v cargo-zigbuild > /dev/null || { echo "cargo-zigbuild missing (cargo install cargo-zigbuild --locked)" >&2; exit 2; }
command -v zig > /dev/null || { echo "zig missing (brew install zig)" >&2; exit 2; }
command -v shasum > /dev/null || command -v sha256sum > /dev/null || { echo "shasum missing" >&2; exit 2; }

FEATURES="latent_retrieval,points,net-mainnet,fix_verify,update_check,kernel_pregate"

echo "== release body (required: the corpus: line per Issue 153 T5 + the T15 exposure-bound note)"
NOTES="release/notes-$TAG.md"
[ -f "$NOTES" ] || { echo "missing $NOTES — draft the release body first (a `corpus:` per-domain rule-delta line + the honest straggler-exposure bound; see riir-refine .plans/223_refine_mainnet_release_checklist.md)" >&2; exit 1; }
grep -q '^corpus:' "$NOTES" || { echo "$NOTES carries no `corpus:` line — the release-note convention (Issue 153 T5) requires one so the update alert quotes real numbers" >&2; exit 1; }

echo "== licenses (cargo-about, fails on any non-accepted license)"
cargo about generate --config .github/about/about.toml \
    --features "$FEATURES" .github/about/about.hbs > /tmp/manual_release_licenses.md
echo "   ok: $(grep -c '^### ' /tmp/manual_release_licenses.md) license section(s)"

echo "== build 5 targets (dist profile per release.yml; remap-path-prefix per host; strip comes from [profile.dist])"
export RUSTFLAGS="--remap-path-prefix $HOME=/build"
cargo build --profile dist --target aarch64-apple-darwin --features "$FEATURES" --bin cargo-refine
cargo build --profile dist --target x86_64-apple-darwin --features "$FEATURES" --bin cargo-refine
cargo zigbuild --profile dist --target aarch64-unknown-linux-musl --features "$FEATURES" --bin cargo-refine
cargo zigbuild --profile dist --target x86_64-unknown-linux-musl --features "$FEATURES" --bin cargo-refine
cargo zigbuild --profile dist --target x86_64-pc-windows-gnu --features "$FEATURES" --bin cargo-refine

echo "== package + G2 leak scan + binary size gate (release.yml parity)"
DIST="/tmp/manual_release_dist"
# The binary's licence (the EULA) rides every archive next to the third-party list.
ROOT_EULA="$(cd "$(dirname "$0")/.." && pwd)/release/LICENSE.md"
[ -f "$ROOT_EULA" ] || { echo "missing $ROOT_EULA" >&2; exit 1; }
rm -rf "$DIST" /tmp/manual_release_stage
mkdir -p "$DIST"
BINS=""
for t in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl x86_64-pc-windows-gnu; do
    stage="/tmp/manual_release_stage/$t"; mkdir -p "$stage"
    if [ -f "target/$t/dist/cargo-refine.exe" ]; then
        bin="target/$t/dist/cargo-refine.exe"
        BINS="$BINS $bin"
        cp "$bin" "$stage/cargo-refine.exe"
        cp /tmp/manual_release_licenses.md "$stage/THIRD_PARTY_LICENSES.md"
        cp "$ROOT_EULA" "$stage/LICENSE.md"
        (cd "$stage" && zip -q "$DIST/cargo-refine-${TAG}-${t}.zip" cargo-refine.exe LICENSE.md THIRD_PARTY_LICENSES.md)
    else
        bin="target/$t/dist/cargo-refine"
        BINS="$BINS $bin"
        cp "$bin" "$stage/cargo-refine"
        cp /tmp/manual_release_licenses.md "$stage/THIRD_PARTY_LICENSES.md"
        cp "$ROOT_EULA" "$stage/LICENSE.md"
        tar -czf "$DIST/cargo-refine-${TAG}-${t}.tar.gz" -C "$stage" cargo-refine LICENSE.md THIRD_PARTY_LICENSES.md
    fi
    ./scripts/binary_leak_scan.sh "$bin"
done
# The size gate runs on the RAW binaries, exactly as the CI package job does.
./scripts/binary_size_gate.sh $BINS
cp /tmp/manual_release_licenses.md "$DIST/THIRD_PARTY_LICENSES.md"
cp "$ROOT_EULA" "$DIST/LICENSE.md"
# Explicit file list — `*` here would include SHA256SUMS itself on any
# manual re-hash of the dir (redirect truncates it first → empty-string
# self-entry → `sha256sum -c` fails on the release page).
(cd "$DIST" && { if command -v sha256sum > /dev/null; then sha256sum -- "cargo-refine-${TAG}"-*.tar.gz "cargo-refine-${TAG}"-*.zip LICENSE.md THIRD_PARTY_LICENSES.md; else shasum -a 256 -- "cargo-refine-${TAG}"-*.tar.gz "cargo-refine-${TAG}"-*.zip LICENSE.md THIRD_PARTY_LICENSES.md; fi; } > SHA256SUMS)

echo "== publish (body from $NOTES)"
# The update alert displays the release TITLE (falling back to the body's
# first line) — so the title CARRIES the corpus line and every alert quotes
# real numbers (the Issue 153 T5 convention; the shape the tests pin:
# `v0.3.0 — corpus: +12 rust_perf rules`). Derived from the notes file so
# title and body can never disagree.
CORPUS_LINE="$(grep '^corpus:' "$NOTES" | head -1)"
gh release create "$TAG" -R gist-rs/cargo-refine \
    --title "cargo-refine $TAG — $CORPUS_LINE" \
    --notes-file "$NOTES" \
    "$DIST"/*

echo "== tap/scoop manifest bumps (the CI pipeline opened PRs; manual mode edits main directly)"
echo "   NOTE: bump gist-rs/homebrew-tap Formula/cargo-refine.rb (version + 4 sha256 from"
echo "   $DIST/SHA256SUMS) and gist-rs/scoop-bucket bucket/cargo-refine.json (version +"
echo "   windows-gnu url + hash), commit + push each — or merge the CI PRs when billing"
echo "   is fixed and CI ran instead."
echo "DONE: https://github.com/gist-rs/cargo-refine/releases/tag/$TAG"
