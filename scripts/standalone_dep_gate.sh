#!/usr/bin/env bash
# standalone_dep_gate.sh — pin the set of path dependencies that ESCAPE this
# repository.
#
# ## Why this exists
#
# `riir-refine` is not standalone-buildable: `katgpt-core` is a NON-optional
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
# entries, three siblings — its Issue 028). Here the surface is twelve
# entries across FIVE siblings, four of them private:
#
#   katgpt-rs      (public)   katgpt-core, katgpt-device-verify,
#                            katgpt-micro-belief (Issue 093 T1 — opt-in
#                            nonergodic_poc; already transitive via
#                            katgpt-core defaults, this is the direct row)
#   riir-infer     (public)   riir-infer-core, riir-infer-gpu (the ternary
#                             lane's inference substrate; T2.3 retarget) +
#                             cubecl-runtime, wgpu-hal (the [patch.crates-io]
#                             vendor paths, now pointing at riir-infer/vendor)
#   riir-ai        (private)  riir-rag-bonsai (bonsai_retrieval) — the ONLY direct
#                             riir-ai edge left (the Bonsai-27B dense-embedder
#                             arm; riir-rag itself carved to riir-neuron-db,
#                             riir-ai Issue 1038, 2026-10-07)
#   riir-neuron-db (private)  riir-neuron-db, riir-rag (latent_retrieval — the
#                             carved facade)
#   riir-kat       (private)  riir-kat (issue 088 — the service-plane leaf;
#                             riir-auth left the set the same day: the row
#                             swapped, not grown — the account substrate is
#                             now riir-kat's dep, resolved transitively)
#   riir-ledger    (private)  riir-ledger (Plan 325 T3 / Proposal 026 P1 — the
#                             points-plane WIRE leaf, opt-in `points`; serde +
#                             serde_json only. Local repo 2026-11-02; the
#                             GitHub remote creation is OWNER-PENDING — CI's
#                             standalone job fails on the bare clone until
#                             the repo exists and this gate's CI step
#                             provisions it.)
#   riir-llm       (private)  riir-llm (issue 124 — the LLM-transport leaf;
#                             optional, `groq_first`/`cf_workers_ai` enable
#                             `riir-llm/transport`)
#   riir-reflexer  (public)   reflexer-vessel (Plan 202 R4 — the vessel verify
#                             path, opt-in `vessel_artifacts`; the public
#                             format repo's leaf crate — blake3 +
#                             ed25519-dalek only, no katgpt dep)
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
# 13 rows as of 2026-10-08 (12 through 2026-10-07 + reflexer-vessel — Plan
# 202 R4: the vessel verify path, opt-in `vessel_artifacts`; the public
# format repo's leaf crate, blake3 + ed25519-dalek only — a new sibling for
# CI to provision, PUBLIC so it rides the token-free checkout lane).
# Earlier: 2026-10-07 + katgpt-percepta — Plan 199 T3.2: the wasm validator
# lane's decoder/lowering substrate, opt-in `wasm_validator`; same
# katgpt-rs checkout as katgpt-core, no new sibling for CI to provision).
# Earlier: Plan 610 S6 / reflex Issue 008 T2.3 — the ternary lane retargeted
# from riir-engine/riir-gpu to riir-infer-core/riir-infer-gpu — same count,
# two rows swapped; the cubecl-wgpu patch row dropped with the old edge,
# arc-swap's genlock fork was a git dep and was never pinned here). `[patch]` entries resolve through the same lexical
# scan (their lines are `name = { path = "…" }` exactly like dependency
# lines) and are pinned — a new vendor patch is a new repo CI must
# provision. `katgpt-core` appears twice in the root manifest (the
# dependency table and `[patch."https://github.com/katopz/katgpt-rs"]`) and
# is pinned once; the scan dedupes the same way.
EXPECTED=$(
    cat <<'EOF'
Cargo.toml cubecl-common
Cargo.toml cubecl-runtime
Cargo.toml cubecl-wgpu
Cargo.toml katgpt-core
Cargo.toml katgpt-device-verify
Cargo.toml katgpt-micro-belief
Cargo.toml katgpt-percepta
Cargo.toml reflexer-vessel
Cargo.toml riir-infer-core
Cargo.toml riir-infer-gpu
Cargo.toml riir-kat
Cargo.toml riir-ledger
Cargo.toml riir-llm
Cargo.toml riir-neuron-db
Cargo.toml riir-rag
Cargo.toml riir-rag-bonsai
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
    echo "   it does not close it. CI provisions every sibling the resolve needs —"
    echo "   eight checkouts: the five pinned siblings + riir-auth (riir-kat's"
    echo "   transitive) + riir-infer (the ternary lane's direct substrate since"
    echo "   the T2.3 retarget — no longer transitive through riir-ai) +"
    echo "   riir-reflexer (Plan 202 R4, the vessel verify path — PUBLIC, so it"
    echo "   rides the token-free lane) — see .github/workflows/rust.yml,"
    echo "   feature-guard job.)"
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
