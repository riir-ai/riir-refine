#!/usr/bin/env bash
# riir-refine feature isolation guard.
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
#   5b2. retrieval gate      → the regression gates compiled OUT of 5b (the
#                              whole `draft` tree is latent_retrieval-only);
#                              named run at the posture its floors pin
#                              (Issue 170)
#   5c. integration + bins   → EVERY tests/*.rs and bin, all-features, no
#                              allowlist (an allowlist rots silently)
#   5d. zero-test detector   → a cfg-gated test file that compiles empty
#                              is a pass that proves nothing
#   6. deploy-token guard    → the local dev-lane credential law (runbook
#                              ../riir-deployer/.docs/02_runbooks/deploy_tokens.md):
#                              only CLOUDFLARE_DEPLOY_API_TOKEN in local env
#                              files; prod-name tracked refs carry the local
#                              name or a comment (skips loud without the
#                              deployer checkout — a deferral, never a pass)
#   7. corpus-pins manifest  → the committed count manifest matches the
#                              compiled corpus + the live prose rows
#                              (plan 329 P1, issue 197; --write is the
#                              count-pin landing pass, never run-to-pass)
#   8. fmt gate              → rustfmt-clean per scripts/fmt_gate.py (issue
#                              223 T4): map-driven exclusions + tripwires
#                              (scripts/fmt_exclusions.txt), batched
#                              per-chunk rustfmt at the toml's edition+style,
#                              drain list fmt_pending_expected.txt (empty,
#                              only shrinks). Zero cargo; missing rustfmt =
#                              loud NO VERDICT, never a silent skip.
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
echo "=== Layer 5b2: retrieval regression gate (the latent_retrieval posture) ==="
# Issue 170: the whole `draft` tree — the retrieval-precision regression gate
# included — compiles ONLY under `latent_retrieval`, so Layer 5b (default
# features) never sees it. The corpus batches that move its floors land
# through lanes that run clippy gates, so the router-annotation floor sat RED
# (85/141 vs the 102/141 pin) across Batches 248–271 with every standing lane
# green. Corpus growth is exactly what this gate exists to catch, so it runs
# HERE, named, at the posture its floors are pinned at (default features +
# latent_retrieval: clippy_lints + rust_perf + docker + dist).
#
# The PASS line is asserted by name: a name filter that matches nothing
# prints `test result: ok` with 0 tests — the green-zero trap. The grep
# below refuses that shape (Layer 5d's law, one layer up).
GATELOG="$(mktemp "${TMPDIR:-/tmp}/ci_feature_guard_5b2.XXXXXX")"
set +e
cargo test --lib --features latent_retrieval retrieval_ 2>&1 | tee "$GATELOG"
_5B2_RC=${PIPESTATUS[0]}
set -e
if [ "$_5B2_RC" != 0 ]; then
    echo "✗ Layer 5b2: the retrieval gate suite FAILED (rc=$_5B2_RC) — see the panic above."
    echo "  Corpus batches shift the router vocab + centroids; the Issue-020"
    echo "  contract is re-measure + re-pin with the delta recorded, never silently."
    exit "$_5B2_RC"
fi
if ! grep -q "draft::retrieval_eval::tests::retrieval_precision_regression_gate ... ok" "$GATELOG"; then
    echo "✗ Layer 5b2: the named gate never RAN — its filter matched nothing or"
    echo "  the test compiled out (the green-zero shape: 'ok' with 0 tests)."
    echo "  Check the latent_retrieval posture and the test's name."
    exit 1
fi
grep -q "^test draft::retrieval_eval::tests::eval_report_is_deterministic ... ok" "$GATELOG" \
    || { echo "✗ Layer 5b2: eval_report_is_deterministic did not run — the filter drifted"; exit 1; }
rm -f "$GATELOG"
echo "✓ retrieval regression gate ran + passed at the latent_retrieval posture"

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
# bin unit-test module of the root `cargo-refine` bin that Layer 5b's `--lib`
# scope never reached. No `--bins` needed. (The migrated tier bins' unit
# tests — `kat_drain`'s 25 — are Layer 5t's coverage since issue 200 C2;
# `fixer_regate` carries none, measured.)
#
# Deliberately NOT stating per-bin test counts here: they drift with every
# added test and a stale number in a comment reads as a spec. Nothing in this
# script enforces them — Layer 5d's pin is derived from `tests/*.rs` on disk.
RUNLOG="$(mktemp "${TMPDIR:-/tmp}/ci_feature_guard_5c.XXXXXX")"
# ── Completion sentinel (katgpt-rs Issue 734) ──────────────────────────────
# Measured on macOS `/bin/bash` 3.2.57 — and ONLY there (katgpt-rs Issue
# 735): bash 4.4 through 5.3, dash and busybox ash all preserve the
# status. When 3.2 aborts on an unbound expansion (`set -u`) or an `eval`
# syntax error it enters the EXIT trap with `$?`
# ALREADY 0 — so a cleanup trap whose last command succeeds makes the abort
# exit 0, and everything after it silently did not run while the caller reads
# a PASS. `trap 'rc=$?; cleanup; exit $rc'` does not help: the rc it saves is
# itself 0. Only "did the script reach its own last line?" catches it.
# Every deliberate `exit 1`/`exit 2` below passes through untouched — this arm
# fires only on INCOMPLETE *and* claiming success.
GUARD_COMPLETED=0
guard_cleanup() {
    guard_st=$?
    rm -f "$RUNLOG"
    rm -f "${GATELOG:-}" 2>/dev/null || true
    if [ "$GUARD_COMPLETED" != "1" ] && [ "$guard_st" = "0" ]; then
        echo "✗ guard ABORTED mid-run while reporting success — it did not reach its" >&2
        echo "  own last line, so the layers past the error above ran NOTHING." >&2
        echo "  A premature death must never read as a pass; forcing exit 1." >&2
        exit 1
    fi
    exit "$guard_st"
}
trap guard_cleanup EXIT

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
    # Per-test wall-clock probe: a single test burning a large share of the
    # layer budget is named here, because the layer-level "exceeded Ns" only
    # says WHICH TARGET was last started, not WHICH TEST hung. Measured live
    # (2026-09-05, Docker Desktop cred-helper wedge): docker_tier2_buildcheck's
    # agreement test sat >25 min in ONE `docker build --check` invocation
    # whose CLI blocked forever in `open()` — the in-crate CALL_TIMEOUT now
    # converts that to a loud FAIL; this line names any OTHER test that ever
    # wedges the same way, before the whole layer burns.
    awk '
        /has been running for over/ && /^test / {
            s = $2; for (i = 3; i < NF; i++) s = s " " $i;
            sub(/ has been running for over.*/, "", s);
            sub(/\.\.\.$/, "", s);
            print s
        }
    ' "$RUNLOG" | sort -u | while IFS= read -r slow; do
        echo "  ⚠ slow test (libtest watchdog fired ≥60s): $slow"
    done
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
    /^ +Running tests/ { t = $2 }
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
echo "=== Layer 5t: tier crates (issue 200) at their feature extremes ==="
# The root-package workspace's tier members (crates/riir-refine-fixer +
# crates/riir-refine-trainer) are NOT covered by Layers 1–5d: those are
# default-members-scoped (the root only, by design — the charitable posture
# must not pay the tiers' deps), and --workspace would pull the tiers into
# every root layer (feature unification reaching the default posture is
# exactly what the inner-crate shape exists to prevent). This layer is the
# tiers' coverage owner: clippy at BOTH feature extremes + the full test
# surface at all-features — which compiles + runs the migrated bins'
# unit-test modules (kat_drain's 25 offline tests moved here from Layer 5c
# when the bin left the root package; fixer_regate has none, measured, and
# the fixer lib is a charter stub — a legitimate running-0 there).
# Required-features keep the bins skipped at the bare posture: cargo reports
# the skip, and no target compiles to nothing behind a `#![cfg]` — the
# tier-level green-zero class is the --tests arm's job to surface.
for tier in riir-refine-fixer riir-refine-trainer; do
    cargo clippy -p "$tier" --all-targets --no-default-features -- -D warnings
    cargo clippy -p "$tier" --all-targets --all-features -- -D warnings
    cargo test -p "$tier" --all-features --tests
done
echo "✓ tier crates clean (both feature extremes + all-features tests)"

echo
echo "=== Layer 6: deploy-token guard (the local dev-lane credential law) ==="
# The owner deploy-token law (runbook: ../riir-deployer/.docs/02_runbooks/deploy_tokens.md):
# local env files carry ONLY CLOUDFLARE_DEPLOY_API_TOKEN (dev-lane scoped) —
# CLOUDFLARE_API_TOKEN / CF_API_TOKEN are the prod/CI-only names, forbidden in
# local env files and never an auth source for local scripts; a tracked
# non-comment reference to a prod name must carry the local name on the same
# line (a mapping/refusal line) or be a doc. The static half is cheap and
# CI-includable; a missing deployer checkout is a loud DEFERRAL, never a green.
GUARD="../riir-deployer/scripts/deploy_token_guard.sh"
if [ -f "$GUARD" ]; then sh "$GUARD" --repo "$(pwd)" --static --quiet || exit 1
else echo "  · ../riir-deployer absent — deploy-token guard SKIPPED (deferral, not a pass)"; fi

echo
echo "=== Layer 7: corpus-pins manifest gate (plan 329 P1, issue 197) ==="
# The committed `.benchmarks/pins_manifest.json` is the single count source
# of truth; this layer re-derives the live counts from the compiled corpus
# builders (cargo test drives src/pins_manifest.rs's dump) and refuses any
# drift + any stale live prose row (AGENTS/README/.docs). A red is repaired
# by `--write` ONLY when the corpus genuinely moved; the manifest diff lands
# WITH the corpus change. The pins posture build is warm after Layers 2–5.
python3 scripts/pins_manifest.py || exit 1
echo "✓ pins manifest + prose agree with the compiled corpus"

echo
echo "=== Layer 8: fmt gate (issue 223 T4 — rustfmt-clean, map-driven exclusions + tripwires) ==="
# Zero cargo, warm ~1 min on a quiet box (batched per-chunk rustfmt; the
# per-file fallback only fires on a refusing chunk). Missing rustfmt = loud
# NO VERDICT exit 2 — blocking, never a silent skip (condition 4). The gate
# never writes: it flags; the fix is a bare `rustfmt <file>` + commit.
python3 scripts/fmt_gate.py || exit 1
echo "✓ fmt gate clean (drain list only shrinks — never pin new drift)"

echo
echo "=== All layers PASS ==="
GUARD_COMPLETED=1  # the last line — see guard_cleanup above
