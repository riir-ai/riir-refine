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
# Issue 046 adds the USD LLM daily-cap binaries (G1 planted-state + G2
# alloc-free gate path) — same class, same layer.
# Plan 031 A3 adds the trainer-drain bin's unit tests (the offline backlog/
# priority/retry-cap gates; the dry-run itself is offline + CI-able).
# Plan 040 (T1.6) adds the lease-client G10 binary — the std-TCP mock
# server fixtures (304 fast path / atomicity / tamper / reheat delta-billing
# / itemization + the sync/claim wire shapes + G3 offline silence). The
# mock binds 127.0.0.1 ephemeral (no external network; the run-start phase
# itself never runs in CI — no test drives main's network path).
# Proposal 007 T2.1 (Plan 041) adds the staking-client G2S binary —
# the same std-TCP mock pattern for the three staking wire shapes (stake
# request / info fetch / unstake), the honest not-live fallbacks, the
# caveat-#15 copy gate, and the render-budget smoke.
# Proposal 005 Phase 2 T2.4 (Plan 042) adds the promotion-gate G2P binary —
# the real-oracle e2e verdicts (promote-pass / heal-bar / created-bar /
# parse-unsafe / withdrawn-absorbing), determinism ×3, and the
# history-isolation pin (trial runs never append to score_history.jsonl).
# The oracle shells out to real cargo clippy on a one-fixture subset —
# content-cached by the oracle's work root, so the run stays CI-cheap.
cargo test --all-features --test kat_redact_g4 --test kat_sync_payload_g4 --test kat_pending_estimator_g4 --test kat_meter_g9 --test kat_meter_g2_alloc --test kat_account_g9 --test kat_lease_client_g10 --test kat_stake_client_g2s --test kat_promotion_g2p --test llm_spend_gates --test llm_spend_g2_alloc --bin kat_drain
echo "✓ KAT integration gates pass"

echo
echo "=== All layers PASS ==="
