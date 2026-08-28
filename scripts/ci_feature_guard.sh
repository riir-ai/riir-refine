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
#   5a. clippy               → lint under both feature extremes
#   5b. lib tests            → default features (all-features is 5c's job)
#   5c. integration + bins   → EVERY tests/*.rs and bin, all-features, no
#                              allowlist (an allowlist rots silently)
#   5d. zero-test detector   → a cfg-gated test file that compiles empty
#                              is a pass that proves nothing
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
echo "=== Layer 5b: lib tests (default features) ==="
# All-features lib tests are NOT run here: Layer 5c's `--tests` selects every
# target with `test = true`, which includes the lib unittest binary, so
# `cargo test --lib --all-features` here would run the identical binary twice.
# Measured 2026-08-28: that binary is 1505 tests / **894.78 s** — by far the
# guard's dominant cost, and it was being paid twice. Layer 5c runs lib
# unittests FIRST (cargo's target order), so the fail-fast property this layer
# provided is preserved. Default features are still exercised here because
# Layer 5c is all-features only.
cargo test --lib
echo "✓ lib tests pass (default features)"

echo
echo "=== Layer 5c: every integration test + bin (all-features, no allowlist) ==="
# This layer used to name each test binary explicitly. That allowlist was an
# omission-SILENT design, and it had already rotted: 39 of 51 test binaries
# sat outside it, including `kat_redact_t32_external` (the Proposal 005 T3.2
# leak-posture suite) and `sec_boundary_gate` (Proposal 004's
# do-not-reimplement non-negotiable). Both fell out with no signal at all —
# adding a test file simply never joined CI, and nothing said so.
#
# The allowlist was also buying nothing. There is no `[[test]]` block in
# Cargo.toml: every test target is auto-discovered and self-gates via an
# inner `#![cfg(feature = "...")]`, and Layer 5a already type-checks all of
# them via `--all-targets --all-features`. So `--tests` costs the RUN of the
# extra binaries, not their compile. Selecting a subset saved little and hid
# a lot.
#
# What stays out of CI is now decided by `#[ignore]` — in the test file, next
# to the reason (network, model weights, GPU, hours-long) — which is the
# mechanism that should carry that decision. An allowlist in a shell script
# is not.
#
# `--tests` is not just `tests/*.rs`: cargo selects EVERY target with
# `test = true`, so this one invocation also covers the lib unittests and the
# bin unit-test modules (`cargo-heal`, `fixer_regate`, `kat_drain`) that Layer
# 5b's `--lib` scope never reached. No `--bins` needed.
#
# Deliberately NOT stating per-bin test counts here: they drift with every
# added test and a stale number in a comment reads as a spec. Nothing in this
# script enforces them — Layer 5d's pin is derived from `tests/*.rs` on disk.
RUNLOG="$(mktemp "${TMPDIR:-/tmp}/ci_feature_guard_5c.XXXXXX")"
trap 'rm -f "$RUNLOG"' EXIT
cargo test --all-features --tests 2>&1 | tee "$RUNLOG"
echo "✓ all integration tests + bin unit tests pass"

echo
echo "=== Layer 5d: no test binary silently ran ZERO tests ==="
# The failure mode Layer 5c's `--tests` cannot catch on its own. A test file
# gated on a feature that `--all-features` does not turn on compiles to an
# EMPTY binary and reports "running 0 tests" — a green PASS that proves
# nothing. The old allowlist could never have caught this either (it names
# targets, not outcomes); parsing the actual run can, and does so without a
# list to maintain.
#
# Scope is `tests/*.rs` only: a bin with no `#[cfg(test)]` module legitimately
# reports zero (`fixer_regate` does, measured), so bins are excluded by the
# match itself rather than by an exemption list that could grow to hide a
# real ghost. Ignored tests still count toward the "running N" total, so a suite
# that is entirely `#[ignore]`d is NOT flagged here.
#
# Verify the instrument BEFORE trusting its verdict. This layer parses cargo's
# human-readable output, which is not a stable API. If that format ever drifts,
# the awk below matches nothing and the layer prints a cheerful green — a
# check that silently stopped checking, which is precisely the bug class this
# layer exists to catch. So: the number of `Running tests/` lines must equal
# the number of top-level `tests/*.rs` files on disk. Nested files
# (`tests/fixtures/*.rs`) are NOT targets — cargo auto-discovers `tests/*.rs`
# and `tests/*/main.rs` only — so the top-level glob is the right expectation.
EXPECTED_TARGETS="$(find tests -maxdepth 1 -name '*.rs' 2>/dev/null | wc -l | tr -d ' ')"
OBSERVED_TARGETS="$(grep -cE '^ +Running tests/' "$RUNLOG" || true)"
if [ "$OBSERVED_TARGETS" != "$EXPECTED_TARGETS" ]; then
    echo "✗ Layer 5d cannot trust its own instrument — counts disagree:"
    echo "      tests/*.rs files on disk : $EXPECTED_TARGETS"
    echo "      'Running tests/' lines   : $OBSERVED_TARGETS"
    echo
    echo "  Either cargo's output format changed (this parser needs updating"
    echo "  before its verdict means anything), or a test target on disk was"
    echo "  never run. Both are hard failures — a green from an instrument"
    echo "  that stopped measuring is worse than a red."
    exit 1
fi
echo "  instrument check: $OBSERVED_TARGETS/$EXPECTED_TARGETS targets observed"

ZERO_TEST_TARGETS="$(awk '
    /^ +Running tests\// { t = $2 }
    /^running 0 tests$/  { if (t != "") { print t; t = "" } }
' "$RUNLOG")"
if [ -n "$ZERO_TEST_TARGETS" ]; then
    echo "✗ these test targets ran ZERO tests under --all-features:"
    echo "$ZERO_TEST_TARGETS" | sed 's/^/      /'
    echo
    echo "  A cfg-gated test file that compiles empty is a PASS that proves"
    echo "  nothing. Either its \`#![cfg(feature = ...)]\` names a feature that"
    echo "  --all-features does not enable, or the file has no #[test] fn."
    echo "  Fix the gate or delete the file — do not exempt it here."
    exit 1
fi
echo "✓ every tests/*.rs target ran at least one test"

echo
echo "=== All layers PASS ==="
