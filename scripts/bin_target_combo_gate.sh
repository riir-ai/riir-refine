#!/usr/bin/env bash
# Layer 9 of scripts/ci_feature_guard.sh — bin-target feature-combo compile
# gate (issue 231).
#
# The blind spot this gate closes (the fix(230) class, latent 2026-11-05 →
# 2026-11-12): Layer 3 compiles every SINGLE feature; Layers 4/5a/5c compile
# --all-features. Both mask the same shape — a cfg-union consumer (any(A, B))
# whose producer stayed on one arm (all(A, C)) breaks ONLY at a posture with
# the widened side ON and the producer's arm OFF: --all-features satisfies the
# producer through the extra arm, default/singles compile the consumer out.
# No standing layer compiles the PAIR. The break (points+score_bench in
# bin/heal.rs) sat six days across green gates.
#
# Scope: the ROOT package's bins (`--bins`), clippy -D warnings, at the pinned
# matrix in bin_target_combos.txt. Lib-side combos stay Layers 3/5's job. The
# release SET needs no row: it is a subset of --all-features (5a compiles it)
# and the maintained-binary lane (issue 224) executes it at HEAD.
#
# Modes:
#   (default)  compile every pinned combo, report every failure (--keep-going
#              semantics: one run enumerates all — the Layer-3 lesson), then
#              floor-fail below MIN_COMBOS
#   --list     print the parsed combos, compile nothing
#   --canary   ABSOLUTE floor (999999) — MUST fail; proves the floor is live
#              (the test_gate.sh --canary precedent: never "floor+1", it goes
#              stale and can pass for the wrong reason)
#
# GREEN-ZERO DEFENCE (found live on the gate's first run, 2026-11-12): the
# root bins carry required-features (cargo-refine + corpus_diagnose need
# latent_retrieval; wasm-validator-build needs wasm_validator), and cargo
# SILENTLY SKIPS a target whose required-features the posture does not
# satisfy — a combo can compile NOTHING and exit 0, its only signal the
# `warning: target filter ... no targets matched` line. The gate greps for
# that warning and REDS the combo loud: a row whose posture cannot reach the
# bins is a blind row, never a pass. Every row therefore also names
# latent_retrieval (the bins' requirement), which never dilutes the row's
# negative-space value — the union arm under test stays OFF.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
COMBOS_FILE="$HERE/bin_target_combos.txt"
MANIFEST="$HERE/../Cargo.toml"
MIN_COMBOS=4

MODE="run"
if [ "${1:-}" = "--canary" ]; then
    MIN_COMBOS=999999
elif [ "${1:-}" = "--list" ]; then
    MODE="list"
elif [ -n "${1:-}" ]; then
    echo "usage: bin_target_combo_gate.sh [--list|--canary]"
    exit 2
fi

[ -f "$COMBOS_FILE" ] || { echo "✗ combo matrix missing: $COMBOS_FILE"; exit 1; }
[ -f "$MANIFEST" ] || { echo "✗ root manifest missing: $MANIFEST"; exit 1; }

# The root [features] table — every combo feature must exist here, so a
# renamed/removed feature reds its stale pin row instead of silently dropping
# the posture it guarded.
FEATURES_TABLE="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' "$MANIFEST")"
FEATURE_COUNT="$(printf '%s\n' "$FEATURES_TABLE" | grep -c . || true)"
if [ "${FEATURE_COUNT:-0}" -eq 0 ]; then
    echo "✗ cannot read the [features] table from $MANIFEST — instrument blind, refusing (never a green zero)"
    exit 2
fi

n=0
fails=0
LOG="$(mktemp "${TMPDIR:-/tmp}/bin_target_combo_gate.XXXXXX")"
while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in ''|\#*) continue ;; esac
    feats="${line%%|*}"
    reason="${line#*|}"
    feats_compact="$(printf '%s' "$feats" | tr -d ' ')"
    if [ -z "$feats_compact" ] || [ "$reason" = "$line" ]; then
        echo "✗ malformed combo row (need '<features> | <reason>'): $line"
        exit 1
    fi
    for f in $(printf '%s' "$feats_compact" | tr ',' ' '); do
        if ! printf '%s\n' "$FEATURES_TABLE" | grep -qx "$f"; then
            echo "✗ combo [$feats_compact] names feature '$f' absent from the root [features] table — stale pin, fix the row"
            exit 1
        fi
    done
    n=$((n + 1))
    if [ "$MODE" = "list" ]; then
        printf 'combo %d: %s — %s\n' "$n" "$feats_compact" "$reason"
        continue
    fi
    echo "=== combo [$feats_compact]: $reason"
    if ! cargo clippy --bins --features "$feats_compact" -- -D warnings 2>&1 | tee "$LOG"; then
        echo "✗ combo [$feats_compact] FAILED"
        fails=$((fails + 1))
        continue
    fi
    if grep -q 'no targets matched' "$LOG"; then
        echo "✗ combo [$feats_compact] compiled NOTHING — its bins are required-features-gated at this posture; name the missing requirement (latent_retrieval et al) in the row"
        fails=$((fails + 1))
        continue
    fi
    if ! grep -q 'Finished' "$LOG"; then
        echo "✗ combo [$feats_compact] produced no cargo verdict — instrument blind, refusing"
        fails=$((fails + 1))
    fi
done < "$COMBOS_FILE"

if [ "$MODE" = "list" ]; then
    echo "✓ $n combos parsed (matrix unmodified, nothing compiled)"
    exit 0
fi

if [ "$n" -lt "$MIN_COMBOS" ]; then
    echo "✗ floor: $n combos compiled < MIN_COMBOS=$MIN_COMBOS — an empty or shrunk matrix is a blind gate, never a pass"
    exit 1
fi
if [ "$fails" -ne 0 ]; then
    echo "✗ Layer 9 bin-target combo gate FAILED ($fails of $n combos red)"
    exit 1
fi
echo "✓ Layer 9 bin-target combo gate PASS ($n combos, floor $MIN_COMBOS)"
