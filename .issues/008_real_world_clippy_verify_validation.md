# Issue 008 — Real-World Validation of self_evolve via clippy_verify Integration

**Filed:** 2026-08-11
**From:** [Issue 004](004_latent_first_self_evolution_loop.md) "What remains" §1
**Type:** POC — validate the self-evolution loop against real `cargo clippy` feedback
**Status:** resolved (all 5 tasks done, G6 + G7 PASS)

## Problem

Issue 004 shipped the latent-first self-evolution loop with all 5 GOAT gates
PASSing. However, the headline gate (G5) uses a **controlled fixture with
oracle feedback** — the test always knows which fix is correct and reports
it as a success. The 70pp improvement (30% → 100%) proves the *mechanism*
works, but does NOT confirm it *generalizes* to real `cargo clippy` feedback.

Real-world clippy healing has two properties the controlled fixture doesn't
test:

1. **Ground-truth verification is external**: the caller doesn't know which
   fix is correct — only `cargo clippy` does. A fix might parse OK (passes
   L2) but not actually resolve the lint.
2. **The seed corpus's `fixed_code` entries are unverified**: the L2 pruner
   checks they parse as valid Rust, but does NOT check they actually resolve
   their target lints. Some corpus entries might be wrong.

## Scope

Two gates:

### G6 — Corpus truthfulness (does the seed corpus actually work?)

For each seed corpus entry, verify with real `cargo clippy`:
- `buggy_code` triggers `clippy::<lint_name>` (lint IS present)
- `fixed_code` does NOT trigger `clippy::<lint_name>` (lint IS resolved)

This is a factual check on the corpus. If any entry fails, the corpus is
wrong and must be fixed before any evolution test is meaningful.

### G7 — Self-evolution with real clippy feedback (the headline gate)

Run the self-evolution loop with `cargo clippy` as the ground-truth verifier
instead of an oracle. Measure whether the healing rate improves after
feedback.

**Design:**
1. Fixture: for each of the 10 seed corpus lints, build a candidate set:
   - The correct fix (from the corpus, verified by G6)
   - The identity transform (return the buggy code as-is — parses, but
     doesn't resolve the lint)
2. Seed the memory with all candidates (neutral success rate = 0.5)
3. **Baseline pass**: heal each lint without feedback. Verify with cargo
   clippy. Record rate.
4. **Evolution pass**: heal each lint. Verify with cargo clippy. Feed back
   the outcome (success if resolved, fail otherwise). Record rate after
   enough iterations for the direction to converge.
5. **G7**: evolution_rate > baseline_rate by ≥5pp.

**Why this works:** the identity transform always fails (lint still present),
and the correct fix always succeeds (lint resolved). After feedback, the
evolution direction biases toward the correct fix, raising the selection
probability above the baseline.

**Why this is different from G5:** G5 uses an oracle that always knows the
correct answer. G7 uses `cargo clippy` — which means:
- The feedback is real (not simulated)
- The corpus entries are verified against ground truth (G6 prerequisite)
- The end-to-end loop runs with real I/O

## Tasks

- [x] **T1: File this issue**
- [x] **T2: G6 — verify the seed corpus against cargo clippy**
  - Found + fixed 2 corpus bugs: `redundant_closure` had missing `double` fn def → compilation failure; `manual_map` buggy_code triggered `unnecessary_unwrap` instead of `manual_map`
  - Result: **10/10 entries verified** after corpus fix
- [x] **T3: G7 — self-evolution with real clippy feedback**
  - Result: **50.0pp improvement** (50% baseline → 100% evolved), target ≥5pp
  - Found + fixed a mechanism bug: the drafter candidate used a hardcoded `success_rate: 0.5` that masked fail feedback; now looks up the actual trajectory stats via `LatentFixMemory::success_rate_for`
- [x] **T4: Document results in `.benchmarks/008_real_world_validation.md`**
- [x] **T5: Update AGENTS.md with the G6/G7 results + promotion decision**

## GOAT gate

| Gate | What | Target | Status |
|---|---|---|---|
| G6 | Corpus truthfulness | 10/10 entries: buggy triggers, fixed resolves | ✅ **10/10** (after fixing 2 corpus bugs) |
| G7 | Self-evolution with real clippy | improvement ≥5pp after feedback | ✅ **50.0pp** (50% → 100%) |

## Promotion decision (post-G7)

**G7 PASSES with 50.0pp improvement** (50% baseline → 100% evolved), far
exceeding the 5pp target. The self-evolution loop generalizes to real
cargo clippy feedback — the controlled G5 fixture result (oracle feedback)
is confirmed by G7 (ground-truth feedback).

However, `self_evolve` stays **opt-in** for now. Rationale:

1. The G7 fixture is still somewhat artificial (identity transform as the
   wrong candidate). Real-world clippy healing involves many candidate
   fixes per lint, some of which parse OK but don't resolve the lint in
   subtle ways. The 2-candidate fixture is a lower bound on complexity.
2. The `clippy_verify` integration is itself opt-in (slow, requires cargo
   on PATH). Promoting `self_evolve` to default-on without `clippy_verify`
   available means the feedback loop can't run.
3. The mechanism now depends on `success_rate_for` lookup in the hot path —
   a linear scan over all trajectories. At N=1000 trajectories this adds
   ~6µs per heal call (measured in G2). Acceptable, but worth profiling
   at production scale.

**Decision: keep `self_evolve` opt-in, documented as "validated against
real cargo clippy feedback (G7 PASS)".** The mechanism works; the opt-in
gate is a deployment-environment constraint, not a correctness concern.

## Out of scope

- **Multi-file projects** — single-file `src/lib.rs` verification only
- **Lint groups beyond the seed corpus** — 10 lints is sufficient for validation
- **Performance** — cargo clippy is seconds per call; this is not a hot-path gate

## References

- [Issue 004](004_latent_first_self_evolution_loop.md) — the self-evolution loop (RESOLVED, G1-G5 PASS)
- [Issue 003](003_phase3_clippy_lint_resolution_verify.md) — the clippy_verify feature
- [Research 001](../.research/001_self_evolving_latent_first_agent.md) — the distillation
