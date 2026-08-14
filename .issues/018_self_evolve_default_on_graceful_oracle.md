# Issue 018 — Promote `self_evolve` to default-on with graceful oracle degradation

**Filed:** 2026-08-15, out of the improvement-priorities session.
**Status:** OPEN.
**Type:** feature promotion (learn axis of the flywheel).

## Background

The `self_evolve` layer (Issues 004 + 008) is fully validated:
`LatentFixMemory` (3-axis retrieval over fix trajectories) +
`LatentSkillEvolution` (EMA direction-vector updates). GOAT **G1–G7 ALL
PASS**, including G6 (10/10 corpus truthfulness) and G7 (**50% → 100%
healing rate** on the failing class) against real `cargo clippy` feedback.
~730 LOC, reimplemented locally, zero new deps.

It stays opt-in for ONE reason: its feedback loop consumes `clippy_verify`,
which is itself opt-in (requires cargo+clippy on PATH). That is a
**deployment-environment constraint, not a correctness concern** — the
mechanism is proven.

The split to fix: the memory (store/retrieve fix trajectories) needs NO
oracle; only the *learning feedback* does. Bundling both behind the
`clippy_verify`-shaped gate starves the memory in every default deployment.

## Proposal

Split mechanism from oracle, then promote:

1. `LatentFixMemory` active whenever healing runs (default build): every
   applied/reverted fix is recorded locally. Modelless, no external deps.
2. Learning feedback (`LatentSkillEvolution` updates) hooks into the
   verifier when present; **no-ops with one clear log line** when
   `clippy_verify` is unavailable (graceful degradation — same shape as
   Issue 015's verify-absent behavior).
3. Explicit opt-out flag/env for consumers who want the pure loop.
4. Promote `self_evolve` to default.

## Acceptance

- [ ] Memory records fix trajectories in the default build (no feature
      flags) — test proves entries accumulate after a `--fix` run.
- [ ] Oracle-absent path tested: learning no-ops, memory still works, one
      log line, zero panics/errors.
- [ ] Oracle-present path unchanged: G7 behavior reproduced (regression
      test or documented re-run).
- [ ] Default build dep surface unchanged (`cargo tree` diff empty).
- [ ] GOAT G3: default `cargo test --lib` green + `ci_feature_guard.sh`
      passes with the promoted default.
- [ ] Feature-gate docs updated (AGENTS.md `self_evolve` entry → default).

## Non-goals

- No new learning mechanisms — this is promotion + degradation plumbing
  only.
- No change to `clippy_verify`'s own opt-in status (Issue 015 owns the
  verify path).
