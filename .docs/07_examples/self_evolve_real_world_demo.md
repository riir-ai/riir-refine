# self_evolve_real_world_demo — self_evolve G6/G7 GOAT gate

**Source:** [`examples/self_evolve_real_world_demo.rs`](../../examples/self_evolve_real_world_demo.rs)
**Features:** `self_evolve + clippy_verify` (both opt-in — requires cargo+clippy on PATH)
**Run:** `cargo run --example self_evolve_real_world_demo --features "self_evolve clippy_verify" --release`
**Runtime:** ~30s on Apple Silicon (cargo clippy is BLAKE3-cached)

## What it demonstrates

The real-world GOAT gate for the self-evolution loop, using `cargo clippy` as
ground truth instead of an oracle:

- **G6 (corpus truthfulness)**: for each seed corpus entry, verifies that
  `buggy_code` triggers `clippy::<lint_name>` AND `fixed_code` resolves it.
  This is the factual prerequisite for any evolution test — a corpus entry
  whose `buggy_code` doesn't actually trigger the named lint, or whose
  `fixed_code` doesn't resolve it, makes every downstream measurement
  meaningless. G6 caught two such bugs in the original corpus
  (`redundant_closure` referenced an undefined fn; `manual_map` triggered
  `unnecessary_unwrap` instead).

- **G7 (self-evolution with real clippy feedback)**: seeds memory with
  `[correct_fix, identity_transform]` per lint, rotates which candidate the
  drafter picks first (even-indexed → correct, odd-indexed → identity), then
  runs 3 rounds of fix → verify-with-cargo-clippy → record-outcome. The
  baseline is ~50% (5/10); the evolved rate after feedback should be ≥5pp
  higher. Measured: **50.0pp improvement (50% → 100%)**.

## Why this is a separate example (not part of goat_gate_004)

The Issue 004 GOAT gate (`tests/goat_gate_004.rs`) uses an **oracle** — the
test knows which fix is correct and reports it as a success. That proves the
*mechanism* works (G5: 70pp improvement) but does NOT confirm it generalizes
to real `cargo clippy` feedback, where:

1. The caller doesn't know which fix is correct — only `cargo clippy` does.
2. A fix might parse OK (passes L2 pruner) but not actually resolve the lint.
3. The feedback loop is seconds, not microseconds.

This example closes that gap by using `ClippyVerifier` as the ground truth.

## The mechanism bug it caught

During G7 testing, found that 3 of 5 odd-indexed lints didn't converge. Root
cause: `HealPipeline::heal()` hardcoded `success_rate = 0.5` for drafter
candidates, ignoring the trajectory's actual fail stats. When the identity
transform failed, its trajectory's `success_rate` dropped to 0.0 in memory —
but the drafter candidate kept its hardcoded 0.5, causing a tie with the
correct candidate, and the drafter won ties.

**Fix:** added `LatentFixMemory::success_rate_for(lint_name, fixed_code)` and
updated `HealPipeline::heal()` to look up the actual trajectory stats for
the drafter candidate. After a fail, the drafter candidate's rate drops to
0.0, and the correct candidate wins.

## See also

- [`../10_self_evolve/heal_pipeline.md`](../10_self_evolve/heal_pipeline.md) — the pipeline being validated
- [`../../.benchmarks/008_real_world_validation.md`](../../.benchmarks/008_real_world_validation.md) — the persisted results
- [`clippy_verify_demo.md`](clippy_verify_demo.md) — the Phase 3 verifier (G1–G4)
