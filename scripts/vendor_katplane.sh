#!/bin/sh
# Re-vendor the katplane protocol tier from the private sibling (issue 221).
#
#   scripts/vendor_katplane.sh <riir-kat-sha>     # copy + re-header
#   scripts/vendor_katplane.sh --check <sha>      # diff only, no write
#
# The canonical copies live in the PRIVATE sibling repo (riir-kat/src/);
# refine carries vendored std-only copies in src/katplane/ because the
# ungated product surfaces (offline firewall, settings, config, --info,
# secret scan) must compile at DEFAULT features with zero private deps —
# the crates.io lane (issue 221 / issue 192 GAP 11).
#
# Drift law: BEHAVIOR is pinned by tests/katplane_parity.rs (units, hosts,
# FindingKind census); a re-vendor that moves any pinned value must update
# that test in the same commit. Lint-posture doc fixes may land refine-side
# without touching the sibling.
#
# NEVER hand-edit one side: the sibling stays canonical; this script is the
# only sanctioned write path for src/katplane/.
set -eu

REPO="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
SIBLING="${REPO}/../riir-kat/src"
DEST="${REPO}/src/katplane"
MODULES="kat_redact kat_units kat_mining_consent kat_service_urls kat_service_config kat_net_pin kat_state_stamp kat_soft_gate"

usage() {
    echo "usage: $0 <riir-kat-sha> | $0 --check <riir-kat-sha>" >&2
    exit 2
}

[ $# -ge 1 ] || usage
MODE="write"
case "$1" in
    --check) [ $# -eq 2 ] || usage; SHA="$2"; MODE="check" ;;
    -*) usage ;;
    *) [ $# -eq 1 ] || usage; SHA="$1" ;;
esac

[ -d "$SIBLING" ] || { echo "✗ sibling missing: $SIBLING (read-only source)" >&2; exit 1; }
for m in $MODULES; do
    [ -f "$SIBLING/$m.rs" ] || { echo "✗ sibling module missing: $m.rs" >&2; exit 1; }
done

header() {
    printf '%s\n' \
"//! Vendored from \`riir-kat\` (\`$SHA\`, vendored 2026-11-09, issue 221" \
"//! private-dep minimization): the feature-free protocol tier refine's" \
"//! UNGATED surfaces consume (secret scan, units, consent dir, config" \
"//! store, hosts, net pin, state stamp, soft gate). std-only by the" \
"//! leaf's protocol-tier purity rule. The canonical copy stays in the" \
"//! private sibling (\`riir-kat\` \`src/$1.rs\`); drift is caught by the" \
"//! golden pins in \`tests/katplane_parity.rs\` and repaired by" \
"//! re-vendoring via \`scripts/vendor_katplane.sh\` — never hand-edit one" \
"//! side."
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
for m in $MODULES; do
    { header "$m"; echo; cat "$SIBLING/$m.rs"; } > "$tmp/$m.rs"
done

if [ "$MODE" = "check" ]; then
    rc=0
    for m in $MODULES; do
        if ! diff -u "$DEST/$m.rs" "$tmp/$m.rs" >/dev/null 2>&1; then
            echo "Δ $m differs from sibling @$SHA"
            rc=1
        fi
    done
    [ "$rc" -eq 0 ] && echo "✓ katplane matches riir-kat @$SHA (modulo the provenance header)" || true
    exit "$rc"
fi

mkdir -p "$DEST"
for m in $MODULES; do
    cp "$tmp/$m.rs" "$DEST/$m.rs"
    echo "vendored $m @$SHA"
done
echo "→ run: rustfmt src/katplane/*.rs (edition from rustfmt.toml), then the parity pins + clippy -D at kat_service,kat_transport"
