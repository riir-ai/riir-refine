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

FEATURES="latent_retrieval,rust_perf,clippy_lints,kat_service,kat_transport"

echo "== licenses (cargo-about, fails on any non-accepted license)"
cargo about generate --config .github/about/about.toml \
    --features "$FEATURES" .github/about/about.hbs > /tmp/manual_release_licenses.md
echo "   ok: $(grep -c '^### ' /tmp/manual_release_licenses.md) license section(s)"

echo "== build 5 targets (remap-path-prefix per host; strip comes from profile.release)"
export RUSTFLAGS="--remap-path-prefix $HOME=/build"
cargo build --release --target aarch64-apple-darwin --features "$FEATURES" --bin cargo-refine
cargo build --release --target x86_64-apple-darwin --features "$FEATURES" --bin cargo-refine
cargo zigbuild --release --target aarch64-unknown-linux-musl --features "$FEATURES" --bin cargo-refine
cargo zigbuild --release --target x86_64-unknown-linux-musl --features "$FEATURES" --bin cargo-refine
cargo zigbuild --release --target x86_64-pc-windows-gnu --features "$FEATURES" --bin cargo-refine

echo "== package + G2 leak scan"
DIST="/tmp/manual_release_dist"
# The binary's licence (the EULA) rides every archive next to the third-party list.
ROOT_EULA="$(cd "$(dirname "$0")/.." && pwd)/release/LICENSE.md"
[ -f "$ROOT_EULA" ] || { echo "missing $ROOT_EULA" >&2; exit 1; }
rm -rf "$DIST" /tmp/manual_release_stage
mkdir -p "$DIST"
for t in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl x86_64-pc-windows-gnu; do
    stage="/tmp/manual_release_stage/$t"; mkdir -p "$stage"
    if [ -f "target/$t/release/cargo-refine.exe" ]; then
        bin="target/$t/release/cargo-refine.exe"
        cp "$bin" "$stage/cargo-refine.exe"
        cp /tmp/manual_release_licenses.md "$stage/THIRD_PARTY_LICENSES.md"
        cp "$ROOT_EULA" "$stage/LICENSE.md"
        (cd "$stage" && zip -q "$DIST/cargo-refine-${TAG}-${t}.zip" cargo-refine.exe LICENSE.md THIRD_PARTY_LICENSES.md)
    else
        bin="target/$t/release/cargo-refine"
        cp "$bin" "$stage/cargo-refine"
        cp /tmp/manual_release_licenses.md "$stage/THIRD_PARTY_LICENSES.md"
        cp "$ROOT_EULA" "$stage/LICENSE.md"
        tar -czf "$DIST/cargo-refine-${TAG}-${t}.tar.gz" -C "$stage" cargo-refine LICENSE.md THIRD_PARTY_LICENSES.md
    fi
    ./scripts/binary_leak_scan.sh "$bin"
done
cp /tmp/manual_release_licenses.md "$DIST/THIRD_PARTY_LICENSES.md"
cp "$ROOT_EULA" "$DIST/LICENSE.md"
(cd "$DIST" && { if command -v sha256sum > /dev/null; then sha256sum -- *; else shasum -a 256 -- *; fi; } > SHA256SUMS)

echo "== publish"
gh release create "$TAG" -R gist-rs/cargo-refine \
    --title "cargo-refine $TAG" \
    --notes "Prebuilt cargo-refine binaries for $TAG. Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README." \
    "$DIST"/*

echo "== tap/scoop manifest bumps (the CI pipeline opened PRs; manual mode edits main directly)"
echo "   NOTE: bump gist-rs/homebrew-tap Formula/cargo-refine.rb (version + 4 sha256 from"
echo "   $DIST/SHA256SUMS) and gist-rs/scoop-bucket bucket/cargo-refine.json (version +"
echo "   windows-gnu url + hash), commit + push each — or merge the CI PRs when billing"
echo "   is fixed and CI ran instead."
echo "DONE: https://github.com/gist-rs/cargo-refine/releases/tag/$TAG"
