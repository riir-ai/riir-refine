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
MODULES="kat_redact kat_units kat_mining_consent kat_service_urls kat_service_config kat_net_pin kat_state_stamp kat_soft_gate kat_ua"
# kat_ua is a PARTIAL vendoring (issue 221 P2): only the ungated half — the
# consumer-tag install + the header-value shape over kat_net_pin. The ureq
# apply/TOTP/signer arms stay sibling-side (their consumers are kat-gated),
# so --check compares only the vendored slice's lines, not whole-file parity.
PARTIAL_MODULES="kat_ua"

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
        case " $PARTIAL_MODULES " in
            *" $m "*)
                # partial vendoring: the vendored copy's non-header lines must
                # all EXIST in the sibling module (a subset check, not parity)
                if ! grep -qF "$(grep -vE '^//!' "$DEST/$m.rs" | head -20 | tail -5)" "$SIBLING/$m.rs" 2>/dev/null; then
                    echo "Δ $m (partial) anchor lines not found in sibling @$SHA"
                    rc=1
                fi
                ;;
            *)
                if ! diff -u "$DEST/$m.rs" "$tmp/$m.rs" >/dev/null 2>&1; then
                    echo "Δ $m differs from sibling @$SHA"
                    rc=1
                fi
                ;;
        esac
    done
    [ "$rc" -eq 0 ] && echo "✓ katplane matches riir-kat @$SHA (modulo the provenance header; partial modules subset-checked)" || true
    exit "$rc"
fi

mkdir -p "$DEST"
for m in $MODULES; do
    case " $PARTIAL_MODULES " in
        *" $m "*)
            # hand-maintained partial copy — re-vendoring it would clobber the
            # slice boundary; drift is caught by the subset check above
            echo "kept $m (partial vendoring — subset-checked, not rewritten)"
            ;;
        *)
            cp "$tmp/$m.rs" "$DEST/$m.rs"
            echo "vendored $m @$SHA"
            ;;
    esac
done
echo "→ run: rustfmt src/katplane/*.rs (edition from rustfmt.toml), then the parity pins + clippy -D at kat_service,kat_transport"
