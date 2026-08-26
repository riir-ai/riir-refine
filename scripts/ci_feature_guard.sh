#!/usr/bin/env bash
# riir-clippy feature isolation guard.
#
# Runs five cargo layers to catch feature-combo regressions:
#   1. --no-default-features  → zero-dep baseline (only katgpt-core cross-repo)
#   2. default features       → the production path
#   3. --each-feature         → all 5 single-feature combos (--keep-going, so
#                               one run enumerates every failure — the
#                               riir-chain Issue 034 lesson)
#   4. --all-features         → every feature at once (catches combo-only bugs
#                               like the new_spectral/merkle_root regression
#                               documented in riir-neuron-db AGENTS.md)
#   5. clippy + tests         → lint + lib tests under default + all-features
#
# No Lean proof gate (this repo has no `.proofs/`).
# No wasm32 gate (the `wasm_pruner` feature is deferred on rubrc maturity).
#
# Requires cargo-hack for layer 3: `cargo install cargo-hack --locked`.
set -euo pipefail

echo "=== Layer 1: zero-dep baseline (--no-default-features) ==="
cargo check --no-default-features
echo "✓ --no-default-features passes"

echo
echo "=== Layer 2: default features (the production path) ==="
cargo check
echo "✓ default features compile"

echo
echo "=== Layer 3: per-feature check (--each-feature) ==="
if command -v cargo-hack &>/dev/null; then
    # `--keep-going`: without it the run aborts on the first broken feature, so
    # it reports *a* failure rather than *the* failures (riir-chain Issue 034).
    cargo hack check --each-feature --keep-going
    echo "✓ all single-feature combos pass"
else
    echo "⚠ cargo-hack not installed, skipping per-feature check"
    echo "  install with: cargo install cargo-hack --locked"
fi

echo
echo "=== Layer 4: all-features combination check ==="
# Catches combo-only bugs — the class where no single feature turns on both
# sides of a regression simultaneously (the merkle_root/can_freeze lesson,
# riir-neuron-db AGENTS.md). riir-clippy's 5 features are mostly independent,
# but `self_evolve` + `ruliology_search` both touch the drafter/pruner surface.
cargo check --all-features
echo "✓ --all-features compiles"

echo
echo "=== Layer 5a: clippy (no-default-features + all-features) ==="
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings
echo "✓ clippy clean (both feature extremes)"

echo
echo "=== Layer 5b: lib tests (default + all-features) ==="
cargo test --lib
cargo test --lib --all-features
echo "✓ lib tests pass (both feature sets)"

echo
echo "=== Layer 5c: KAT integration gates (all-features) ==="
# Proposal 005 T0.2 — the redaction G4 fixtures are a STANDING CI gate
# ("zero secrets in any synced payload, by test"), plus the meter/account
# G9 binaries that Layer 5b's --lib scope never reaches. Without this layer
# those tests ran only when invoked by hand (the gate-coverage lesson).
# T0.3 (Plan 027) adds the sync-payload G4 binary — same class, same layer
# (the payload builder is the serialization seam the gate guards).
# T0.4 (Plan 028) adds the pending-estimator G4 binary — the caveat-#15
# framing gate (no $ beside an accrual; blocked never earns) + the
# watermark recovery gate.
cargo test --all-features --test kat_redact_g4 --test kat_sync_payload_g4 --test kat_pending_estimator_g4 --test kat_meter_g9 --test kat_meter_g2_alloc --test kat_account_g9
echo "✓ KAT integration gates pass"

echo
echo "=== All layers PASS ==="
