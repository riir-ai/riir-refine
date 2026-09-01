#!/usr/bin/env bash
# riir-clippy feature isolation guard.
#
# Runs five cargo layers to catch feature-combo regressions:
#   1. --no-default-features  → zero-dep baseline (only katgpt-core cross-repo)
#   2. default features       → the production path
#   3. --each-feature         → every single-feature combo, ~30 of them as of
#                               2026-09-02 (--keep-going, so
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
# riir-neuron-db AGENTS.md). The feature set is ~30 (2026-09-02) and mostly
# independent, but `self_evolve` + `ruliology_search` both touch the
# drafter/pruner surface.
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

# Layer 5c was the one UNBOUNDED step in this guard, and unbounded is exactly
# how it failed. `--all-features` is a SUPERSET switch: it turns on every
# feature gate, including ones guarding tests that load model weights or run
# for hours. One such binary (`bonsai_retrieval_probe`: 6.7 GB GGUF, ~50 min in
# RELEASE, run here in DEBUG with both tests loading the model concurrently at
# 15.5 GB RSS) consumed the ENTIRE run, so Layer 5c had never once completed
# since it landed. It was misdiagnosed three separate times as an environment
# problem — a killed process, nohup vs a process-group SIGKILL — because a run
# that simply never returns names no cause. Measured: 4 binaries finished in
# the ~30 min before that probe; 26 finished in the 3 min after it was killed.
#
# `#[ignore]` is the fix for any individual offender (see Bench 055), but
# nothing ENFORCES the annotation, and the next feature to gate a heavy test
# enters this layer silently. So bound the layer and, on expiry, name the
# target it died on — turning "hangs forever, cause unknown" into a diagnosis.
# The budget is deliberately generous: it is a wedge detector, not a perf gate.
LAYER_5C_TIMEOUT_SECS="${LAYER_5C_TIMEOUT_SECS:-3600}"
_TIMEOUT_BIN="$(command -v timeout || command -v gtimeout || true)"
if [ -n "$_TIMEOUT_BIN" ]; then
    set +e
    "$_TIMEOUT_BIN" "$LAYER_5C_TIMEOUT_SECS" cargo test --all-features --tests 2>&1 | tee "$RUNLOG"
    _5C_RC=${PIPESTATUS[0]}
    set -e
    if [ "$_5C_RC" = 124 ]; then
        echo
        echo "✗ Layer 5c exceeded ${LAYER_5C_TIMEOUT_SECS}s and was killed."
        echo "  last target it started:"
        grep -E '^ +Running ' "$RUNLOG" | tail -1 | sed 's/^/      /'
        echo
        echo "  --all-features enables EVERY feature gate, so a test that loads"
        echo "  model weights or runs for hours joins this layer silently. If the"
        echo "  target above is a measurement probe rather than a gate, mark it"
        echo "  #[ignore] with the cost in the reason (Bench 055) — do not raise"
        echo "  this budget to accommodate it. Raise it only when the SWEEP has"
        echo "  legitimately grown."
        exit 1
    fi
    [ "$_5C_RC" = 0 ] || exit "$_5C_RC"
else
    # Never fail the guard over a missing coreutils; just refuse to imply a
    # bound that is not being enforced.
    echo "  WARN: no timeout(1)/gtimeout(1) — Layer 5c runs UNBOUNDED;"
    echo "        a wedged test binary will hang this guard with no diagnosis."
    cargo test --all-features --tests 2>&1 | tee "$RUNLOG"
fi
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
# cargo's human output renders paths with the OS separator (`tests\x.rs` on
# Windows, `tests/x.rs` elsewhere) — the class must accept both or the
# instrument silently counts 0 on Windows and fails its own consistency
# check (measured 2026-08-31, the first Layer 5c completion on this box).
OBSERVED_TARGETS="$(grep -cE '^ +Running tests[\\/]' "$RUNLOG" || true)"
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
    /^ +Running tests[\\/]/ { t = $2 }
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
