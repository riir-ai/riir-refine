#!/usr/bin/env bash
# standalone_dep_gate.sh — pin the set of path dependencies that ESCAPE this
# repository.
#
# ## Why this exists
#
# `riir-clippy` is not standalone-buildable: `katgpt-core` is a NON-optional
# `path = "../katgpt-rs…"` dependency, and cargo reads every path dependency's
# manifest when it loads the workspace graph — `optional = true` governs
# *linking*, not *resolution* — so with the siblings absent, no cargo command
# runs at all, not even `cargo metadata`.
#
# Closing that gap would mean publishing the siblings to a registry. What IS a
# one-session change is making the gap **non-growable**: this gate enumerates
# the escaping deps lexically, from text, and fails if the set differs from the
# pinned inventory.
#
# Precedent: `riir-neuron-db/scripts/standalone_dep_gate.sh` (3 entries, one
# public sibling) and `riir-chain/scripts/standalone_dep_gate.sh` (twelve
# entries, three siblings — its Issue 028). Here the surface is nine entries
# across THREE siblings, two of them private:
#
#   katgpt-rs      (public)   katgpt-core, katgpt-device-verify
#   riir-ai        (private)  riir-engine, riir-gpu, riir-rag
#                             + cubecl-runtime, cubecl-wgpu, wgpu-hal
#                             (the [patch.crates-io] vendor paths)
#   riir-neuron-db (private)  riir-neuron-db
#
# Provisioning them is `.github/workflows/rust.yml`'s job (the feature-guard
# job; the two private siblings need `SIBLING_REPOS_TOKEN`).
#
# ## Why it is the only cargo-shaped check that can run sibling-free
#
# It never invokes cargo. Every other gate in this repo
# (`scripts/ci_feature_guard.sh`) needs `katgpt-rs` — and for --all-features
# `riir-ai` + `riir-neuron-db` too — on disk before cargo will even parse the
# manifest. This one needs a checkout and `bash`, so it runs unconditionally
# in CI with no secret — see the `standalone` job in
# `.github/workflows/rust.yml`.
#
# ## Adding or removing an escaping dep
#
# Deliberate changes update `EXPECTED` below in the same commit. An
# *accidental* one fails here, which is the whole job. Removing one is just
# as gated, so a fix that shrinks the surface has to record the shrink.
set -euo pipefail

cd "$(dirname "$0")/.."

# The pinned inventory: "<manifest-relative-path> <dep-name>", sorted.
#
# 9 entries as of 2026-09-02. `[patch]` entries resolve through the same
# lexical scan (their lines are `name = { path = "…" }` exactly like
# dependency lines) and are pinned — a new vendor patch is a new repo CI must
# provision. `katgpt-core` appears twice in the root manifest (the dependency
# table and `[patch."https://github.com/katopz/katgpt-rs"]`) and is pinned
# once; the scan dedupes the same way.
EXPECTED=$(
    cat <<'EOF'
Cargo.toml cubecl-runtime
Cargo.toml cubecl-wgpu
Cargo.toml katgpt-core
Cargo.toml katgpt-device-verify
Cargo.toml riir-engine
Cargo.toml riir-gpu
Cargo.toml riir-neuron-db
Cargo.toml riir-rag
Cargo.toml wgpu-hal
EOF
)

# Lexical resolution — deliberately NOT `realpath`/`readlink -f`, which need the
# target to exist. The whole point is to answer "does this escape?" in a
# checkout where the sibling is absent, so the answer must come from the text.
#
# Returns 0 (escapes) when normalising <manifest-dir>/<dep-path> walks above the
# repository root.
escapes() {
    local manifest_dir="$1" dep_path="$2"
    local depth=0 component
    local IFS=/

    for component in $manifest_dir; do
        case "$component" in '' | . | ..) ;; *) depth=$((depth + 1)) ;; esac
    done

    for component in $dep_path; do
        case "$component" in
            '' | .) ;;
            ..)
                [ "$depth" -eq 0 ] && return 0
                depth=$((depth - 1))
                ;;
            *) depth=$((depth + 1)) ;;
        esac
    done
    return 1
}

ACTUAL=""
while IFS= read -r manifest; do
    manifest="${manifest#./}"
    manifest_dir="$(dirname "$manifest")"
    [ "$manifest_dir" = "." ] && manifest_dir=""

    # One dep per line, `name = { … path = "…" … }`. Cargo also accepts a
    # multi-line inline table; none exist here and the sibling-present guard
    # would catch one that did. The `[package]`/`[[bin]]` bare `path = "…"`
    # lines do not match the grep (they carry no second `path = "` after the
    # first `=`), so package-relative paths are correctly ignored.
    while IFS= read -r line; do
        case "$line" in \#*) continue ;; esac
        name="${line%%=*}"
        name="$(printf '%s' "$name" | tr -d '[:space:]')"
        [ -n "$name" ] || continue
        dep_path="${line#*path = \"}"
        dep_path="${dep_path%%\"*}"
        [ -n "$dep_path" ] || continue
        if escapes "$manifest_dir" "$dep_path"; then
            ACTUAL="${ACTUAL}${manifest} ${name}"$'\n'
        fi
    done < <(grep -E '^[A-Za-z0-9_-]+ *=.*path *= *"' "$manifest" || true)
done < <(find . -name Cargo.toml -not -path './target/*' -not -path './.git/*' | sort)

ACTUAL="$(printf '%s' "$ACTUAL" | sort -u)"
EXPECTED="$(printf '%s\n' "$EXPECTED" | sort -u)"

echo "── escaping path dependencies ──"
printf '%s\n' "$ACTUAL" | sed 's/^/  /'
echo

if [ "$ACTUAL" = "$EXPECTED" ]; then
    n=$(printf '%s\n' "$ACTUAL" | grep -c . || true)
    echo "✓ standalone-dep gate PASSED — $n escaping path deps, matching the pin"
    echo "  (this repo is still NOT standalone-buildable; the gate pins the gap,"
    echo "   it does not close it. CI provisions all three siblings —"
    echo "   .github/workflows/rust.yml, feature-guard job.)"
    exit 0
fi

echo "✗ standalone-dep gate FAILED — the escaping-dep set changed."
echo
echo "── diff (expected < / actual >) ──"
diff <(printf '%s\n' "$EXPECTED") <(printf '%s\n' "$ACTUAL") || true
echo
echo "A NEW escaping dep makes the standalone gap worse: every cargo command in"
echo "a checkout without that sibling dies during manifest loading, before any"
echo "code is compiled — and it adds a repo the CI workflow must provision."
echo "Either consume substrate that already lives in this repo, or update"
echo "EXPECTED in scripts/standalone_dep_gate.sh in the same commit AND teach"
echo ".github/workflows/rust.yml to check the sibling out."
exit 1
