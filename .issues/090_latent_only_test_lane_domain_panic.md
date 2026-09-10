# 090 — latent-only test lane: 17 draft tests panic (need ≥1 healing domain)

**Status:** OPEN — found by the 2026-09-10 each-feature rot-gate sweep; compile half fixed same day, runtime half tracked here.

## The lane

`cargo test --lib --no-default-features --features latent_retrieval` — the
latent-only arm of the feature matrix. No guard layer runs it (5b runs
default features, 5c runs `--all-features`; layer 3 is check-only), so it
was never exercised until the 2026-09-10 full `ci_feature_guard.sh` sweep
prompted a manual pass.

## State after the compile fix

The sweep's E0433 class (frontier_pairs importing the rustc_errors corpus
under latent-only combos — 8 failing `--each-feature` checks) is FIXED
same-day: the module now splits into the dependency-free type/fence half and
the `cfg(rustc_errors)` corpus half. `collapse_monitor::loop_tests` also
gained its missing `clippy_lints` gate. The lane COMPILES and 232 tests
pass (including the two fence-arithmetic tests, newly runnable here).

## What remains (this issue)

17 tests fail at RUNTIME with
`LatentMatcher: no domains enabled (need at least one of
clippy_lints/rust_perf/kernel_heal/sec/docker/dist)` — correct production
behavior (the matcher refuses a domain-less build), tested by tests that
conceptually require ≥1 enabled domain:

- `draft::latent_matcher::tests` — 5 (matcher construction/routing over
  enabled domains)
- `draft::headroom_probe::tests` — 6 (same matcher dependency)
- `draft::retrieval_eval::tests` — 6 (fixtures derive from the clippy
  corpus → needs `clippy_lints` specifically)

## The fix shape (when picked up)

Gate each test module on what it actually needs:
`any(clippy_lints, rust_perf, kernel_heal, sec, docker, dist)` for the
matcher/probe halves; `clippy_lints` for retrieval_eval. No guard-layer
count moves (those lanes always have the domains on) — verify by
count-identical `cargo test --lib` at default features before/after.
