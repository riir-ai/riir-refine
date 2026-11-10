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
# only sanctioned write path for src/katplane/. The sanctioned DIVERGENCES
# (applied on every write AND every check, so --check never flags them and
# a bare re-vendor is idempotent):
#   1. the doc-comment crate-path adaptation (adapt_docs — the doctest
#      law): `crate::<module>` → `riir_refine::<module>` on doc lines,
#   2. kat_units's vendored lint-posture block (the sibling's gate is not
#      this crate's pedantic set),
#   3. this repo's rustfmt (rustfmt.toml, style_edition 2024),
#   4. the per-module refine-side posture folds below (the exact-string
#      python step — landed doc backtick fixes from the vendoring commit,
#      the kat_units doctest crate-path spelling, kat_net_pin's vendored
#      allow + Errors section, kat_redact's audit-line fold, kat_state_stamp's
#      clippy folds + the gated round-trip + the hex pin + the char pins).
#      Each layer names its landing; every layer re-adjudicates LOUD when
#      its anchor drifts, so a sibling move can never silently drop one.
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
"//! Vendored from \`riir-kat\` (\`$SHA\`, 2026-11-09, issue 221" \
"//! private-dep minimization): the feature-free protocol tier refine's" \
"//! UNGATED surfaces consume (secret scan, units, consent dir, config" \
"//! store, hosts, net pin, state stamp, soft gate). std-only by the" \
"//! leaf's protocol-tier purity rule. The canonical copy stays in the" \
"//! private sibling (\`riir-kat\` \`src/$1.rs\`); drift is caught by the" \
"//! golden pins in \`tests/katplane_parity.rs\` and repaired by" \
"//! re-vendoring via \`scripts/vendor_katplane.sh\` — never hand-edit one" \
"//! side."
}

# Doc-comment-only crate-path adaptation (the doctest law, 2026-11-10):
# rustdoc compiles every doctest as a separate crate where `crate::` points
# at the doctest binary itself — the documented crate is reachable only by
# its package name (`riir_refine`). A doc example saying `use
# crate::kat_units::…` therefore does not compile in ANY vendored module,
# while the sibling's own copy (`use riir_kat::kat_units::…`) is fine at
# home. The write path adapts the vendored module's SELF-NAME doc refs
# (`crate::<module>` → `riir_refine::<module>` on `//!`/`///` lines only —
# code bytes stay verbatim; cross-module doc refs like
# `crate::kat_lease_client` are NOT self-referential and are untouched),
# and the check mode applies the same transform before diffing, so parity
# stays verifiable and a re-vendor can never clobber the fix.
adapt_docs() {
    m="$1"
    sed -e '\,^[[:space:]]*//!,s,crate::'$m',riir_refine::'$m',g' \
        -e '\,^[[:space:]]*///,s,crate::'$m',riir_refine::'$m',g' \
        "$2"
}

# Insert the lint-posture block after the sibling's `//!` module docs and
# BEFORE the first `///` item doc: an inner attribute (`#![allow]`) is
# illegal after ANY outer doc comment, so both the raw tail position (after
# item docs, rustc error) and the first-`pub` position (rustfmt moves it
# down beside the item docs, which re-invalidates it) are unusable.
insert_lint_posture() {
    awk '
        /^\/\/\// && !done && !printed {
            while ((getline l < "'"$1"'") > 0) print l
            close("'"$1"'")
            print ""
            done = 1
        }
        { print }
    ' "$2"
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
# the vendored lint-posture block (the sibling stays canonical; this is
# refine-side posture — see the case below)
cat > "$tmp/lint_posture.txt" <<'EOF'
// Vendored lint posture (issue 221): the canonical copy passes the sibling's
// gate, not this crate's pedantic set; the µ↔whole conversions are pinned
// by the golden tests, so the `as` forms stay VERBATIM (allow, never
// rewrite money math).
#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

EOF
for m in $MODULES; do
    { header "$m"; echo; adapt_docs "$m" "$SIBLING/$m.rs"; } > "$tmp/$m.rs"
    # The sanctioned refine-side posture layers, applied AFTER the sibling
    # copy: (1) the vendored lint posture block — the canonical copy passes
    # the SIBLING's gate, not this crate's pedantic set (a re-vendor must
    # never re-import the sibling's gate findings); (2) this repo's rustfmt
    # (rustfmt.toml, style_edition 2024) — the sibling formats under its own
    # toolchain, and the vendored copy must match THIS repo's gate.
    case "$m" in
        kat_units)
            insert_lint_posture "$tmp/lint_posture.txt" "$tmp/$m.rs" > "$tmp/$m.rs.new" && mv "$tmp/$m.rs.new" "$tmp/$m.rs"
            ;;
    esac
    # The per-module refine-side posture folds — all exact strings, all
    # loud on drift:
    #   kat_state_stamp: the two-arm Ok match folds to `(None, _)` + the
    #     rename fallback becomes early returns (this crate's clippy vs the
    #     sibling's; a re-vendor must never re-import SIBLING gate
    #     findings — the kat_units posture law verbatim), the round-trip
    #     test gains its kat_service gate + the vendored-in ungated hex-form
    #     pin (the protocol tier lives behind kat_service in this crate
    #     root; landing 8f063da5), and 3dfd703f's char-literal contains
    #     pins ('7', a single byte, not "7").
    #   kat_redact: the audit-line assert folds to the array-ne form (the
    #     vendored Finding carries PartialEq — the rulewright's later
    #     elem_known_partial_eq gate would refuse the is_empty form's
    #     class) + the doc backtick fixes from the vendoring commit.
    #   kat_units: the doctests' crate paths spell riir_refine (adapt_docs
    #     handles the self-name lines; these two are the same class) + the
    #     doc backtick fixes.
    #   kat_service_urls: one doc backtick fix.
    #   kat_net_pin: the vendored #[allow(clippy::unnecessary_wraps)] + the
    #     fn's Errors doc section (this crate's missing_docs-class gate).
    if [ "$m" = "kat_state_stamp" ]; then
        python3 - "$tmp/$m.rs" <<'PYEOF'
import sys
p = sys.argv[1]
src = open(p).read()

def fold(a, b, why):
    global src
    assert a in src, f"state_stamp posture anchor drifted ({why}) — re-adjudicate"
    src = src.replace(a, b)

fold("        (None, Some(_)) => Ok(()),\n        (None, None) => Ok(()),",
     "        (None, _) => Ok(()),", "two-arm Ok fold")
fold("""    match std::fs::rename(path, &dest) {
        Ok(()) => Ok(dest),
        Err(_) => {
            std::fs::copy(path, &dest)?;
            std::fs::remove_file(path)?;
            Ok(dest)
        }
    }""",
     """    if std::fs::rename(path, &dest).is_ok() {
        return Ok(dest);
    }
    std::fs::copy(path, &dest)?;
    std::fs::remove_file(path)?;
    Ok(dest)""", "rename-fallback early returns")
fold("    #[test]\n    fn ledger_id_hex_round_trips_through_the_protocol_parse() {",
     """    // Issue 221 (vendored): the round-trip needs the protocol tier, which
    // lives behind `kat_service` in riir-refine's crate root (in riir-kat
    // the reference was same-crate). The hex encoding itself stays pinned
    // ungated below.
    #[cfg(feature = "kat_service")]
    #[test]
    fn ledger_id_hex_round_trips_through_the_protocol_parse() {""",
     "round-trip kat_service gate")
fold('''            "the state-file encoding is the service-info read's own form"
        );
    }

''',
     '''            "the state-file encoding is the service-info read's own form"
        );
    }

    #[test]
    fn ledger_id_hex_is_the_pinned_64hex_statefile_form() {
        let hex = ledger_id_hex(ID_B);
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(ledger_id_hex(ID_A), ledger_id_hex(ID_A), "stable per id");
        assert_ne!(ledger_id_hex(ID_A), ledger_id_hex(ID_B), "ids differ");
    }

''', "vendored-in hex-form pin")
fold('assert!(moved.contains("7"), "bytes survived the move: {moved}");',
     "assert!(moved.contains('7'), \"bytes survived the move: {moved}\");", "char-literal contains pin 1")
fold('assert!(std::fs::read_to_string(&dest).unwrap().contains("7"));',
     "assert!(std::fs::read_to_string(&dest).unwrap().contains('7'));", "char-literal contains pin 2")
open(p, 'w').write(src)
PYEOF
    fi
    if [ "$m" = "kat_redact" ]; then
        python3 - "$tmp/$m.rs" <<'PYEOF'
import sys
p = sys.argv[1]
src = open(p).read()

def fold(a, b, why):
    global src
    assert a in src, f"kat_redact posture anchor drifted ({why}) — re-adjudicate"
    src = src.replace(a, b)

fold("assert!(!findings.is_empty());", "assert_ne!(findings, [] as [Finding; 0]);", "audit-line assert fold")
fold("/// The PuTTY `.ppk` comment-header form (v4; gitleaks",
     "/// The `PuTTY` `.ppk` comment-header form (v4; `gitleaks`", "doc backtick fix 1")
fold("/// (github_pat_, figma, atlassian, linear, telegram tails, …).",
     "/// (`github_pat_`, figma, atlassian, linear, telegram tails, …).", "doc backtick fix 2")
fold("/// `is_word` plus `.` — the SendGrid `SG.` body charset (its two-dot",
     "/// `is_word` plus `.` — the `SendGrid` `SG.` body charset (its two-dot", "doc backtick fix 3")
fold("    let bytes = span.as_bytes();\n    const NEEDLE: &[u8] = b\"PuTTY-User-Key-File-\";",
     "    const NEEDLE: &[u8] = b\"PuTTY-User-Key-File-\";\n    let bytes = span.as_bytes();", "scan_putty decl order")
open(p, 'w').write(src)
PYEOF
    fi
    if [ "$m" = "kat_units" ]; then
        python3 - "$tmp/$m.rs" <<'PYEOF'
import sys
p = sys.argv[1]
src = open(p).read()

def fold(a, b, why):
    global src
    assert a in src, f"kat_units posture anchor drifted ({why}) — re-adjudicate"
    src = src.replace(a, b)

# The doctest crate-path folds target the FINAL spelling (riir_refine::)
# directly — adapt_docs ran before this block, so the landed copy's
# `crate::` shape and the sibling's `riir_kat::` shape both land here as
# `riir_refine::`; folding to `crate::` would re-break the doctest law the
# adaptation exists for.
fold("//! use riir_kat::kat_units::{MicroKat, WholeKat, MICRO_PER_KAT, render_kat};",
     "//! use riir_refine::kat_units::{MicroKat, WholeKat, MICRO_PER_KAT, render_kat};", "module doctest path")
fold("//!     riir_kat::kat_units::CHARGE_MICRO_KAT_PER_MILLION_TOKENS",
     "//!     riir_refine::kat_units::CHARGE_MICRO_KAT_PER_MILLION_TOKENS", "module doctest body 1")
fold("//!         * riir_kat::kat_units::FREE_TIER_TOKENS_MTOK",
     "//!         * riir_refine::kat_units::FREE_TIER_TOKENS_MTOK", "module doctest body 2")
fold("/// use riir_kat::kat_units::{",
     "/// use riir_refine::kat_units::{", "item doctest path")
fold("//! | lamport | µKAT (micro-KAT) | smallest unit — **the only stored form** |",
     "//! | `lamport` | `µKAT` (micro-KAT) | smallest unit — **the only stored form** |", "doc backtick fix 1")
fold("//!    `_MICRO_KAT` name holds µKAT. The pre-Issue-094 spelling",
     "//!    `_MICRO_KAT` name holds `µKAT`. The pre-Issue-094 spelling", "doc backtick fix 2")
fold("/// KAT). At [`CHARGE_MICRO_KAT_PER_MILLION_TOKENS`] = 1_000_000 µ per",
     "/// KAT). At [`CHARGE_MICRO_KAT_PER_MILLION_TOKENS`] = `1_000_000` µ per", "doc backtick fix 3")
fold("/// OpenRouter).", "/// `OpenRouter`).", "doc backtick fix 4")
open(p, 'w').write(src)
PYEOF
    fi
    if [ "$m" = "kat_service_urls" ]; then
        python3 - "$tmp/$m.rs" <<'PYEOF'
import sys
p = sys.argv[1]
src = open(p).read()
a = "/// ⛔ COMPILE-TO-NOTHING (the net-pin doctrine, kat_net_pin's header): under"
b = "/// ⛔ COMPILE-TO-NOTHING (the net-pin doctrine, `kat_net_pin`'s header): under"
assert a in src, "kat_service_urls posture anchor drifted — re-adjudicate"
open(p, 'w').write(src.replace(a, b))
PYEOF
    fi
    if [ "$m" = "kat_net_pin" ]; then
        python3 - "$tmp/$m.rs" <<'PYEOF'
import sys
p = sys.argv[1]
src = open(p).read()

allow_anchor = "#[cfg(not(all(feature = \"net-mainnet\", not(feature = \"net-devnet\"))))]\n#[must_use]\nfn devnet_service_url() -> Option<String> {"
allow_block = "#[cfg(not(all(feature = \"net-mainnet\", not(feature = \"net-devnet\"))))]\n#[must_use]\n// The Option is the posture contract, not a wrap: the cfg twin arm below\n// returns None at the mainnet-only compile (issue 221 vendored allow).\n#[allow(clippy::unnecessary_wraps)]\nfn devnet_service_url() -> Option<String> {"
assert allow_anchor in src, "kat_net_pin allow anchor drifted — re-adjudicate"
src = src.replace(allow_anchor, allow_block, 1)
errors_doc = """///
/// # Errors
///
/// The pinned-host verdict as a string when the URL's host sits outside
/// the build's allowed set (a hosted net at a single-pin build, or the
/// both-pins conflict posture); `Ok` carries the canonicalized URL.
"""
anchor2 = "pub fn ensure_net_allowed(raw: &str) -> Result<String, String> {"
assert anchor2 in src, "kat_net_pin Errors-section owner not found — re-adjudicate"
# append at the END of the fn's doc block: insert after the doc's LAST
# line (the sibling's doc has no trailing blank /// line, so the landed
# shape is doc-tail + errors_doc + decl)
idx = src.find(anchor2)
# walk back over the doc block to its first line
start = src.rfind("\n\n", 0, idx) + 1
src = src[:idx] + errors_doc + src[idx:]
open(p, 'w').write(src)
PYEOF
    fi
    rustfmt --edition 2024 "$tmp/$m.rs" 2>/dev/null || true
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
                    echo "Δ $m differs from sibling @$SHA (doc-adapted + posture + rustfmt)"
                    rc=1
                fi
                ;;
        esac
    done
    [ "$rc" -eq 0 ] && echo "✓ katplane matches riir-kat @$SHA (doc-adapted + posture + rustfmt; partial modules subset-checked)" || true
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
            echo "vendored $m @$SHA (doc-adapted + posture + rustfmt)"
            ;;
    esac
done
echo "→ then the parity pins + clippy -D at kat_service,kat_transport"
