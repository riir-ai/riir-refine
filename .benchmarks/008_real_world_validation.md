# Benchmark 008 — Real-World Validation of self_evolve via clippy_verify

**Date:** 2026-08-11
**Issue:** 008 (resolved + removed per noise-reduction; resolution captured here)
**Gate:** G6 (corpus truthfulness) + G7 (self-evolution with real cargo clippy)

## Setup

- **Tool:** `examples/self_evolve_real_world_demo.rs`
- **Features:** `self_evolve + clippy_verify`
- **Build:** `cargo run --example self_evolve_real_world_demo --features "self_evolve clippy_verify" --release`
- **Runtime:** ~30s (cargo clippy is cached via BLAKE3-keyed temp dirs)
- **Platform:** macOS (M3 Max)

## G6: Corpus truthfulness

Verifies that each seed corpus entry's `buggy_code` triggers `clippy::<lint_name>`
AND `fixed_code` resolves it.

### Findings (before fix)

Two corpus entries had factual errors caught by G6:

| Lint | Problem | Fix |
|---|---|---|
| `redundant_closure` | `buggy_code` referenced `double` fn without defining it → compilation failure → clippy never ran → false "resolved" | Added `fn double(x: i32) -> i32 { x * 2 }` to both buggy + fixed code |
| `manual_map` | `buggy_code` used `if x.is_some() { Some(x.unwrap()) }` which triggers `unnecessary_unwrap`, not `manual_map` | Changed to `match x { Some(v) => Some(v * 2), None => None }` (the canonical manual_map pattern) |

### Result (after fix)

```
✅ manual_map                     buggy_triggers=Y, fixed_resolves=Y
✅ needless_borrow                buggy_triggers=Y, fixed_resolves=Y
✅ needless_lifetimes             buggy_triggers=Y, fixed_resolves=Y
✅ needless_range_loop            buggy_triggers=Y, fixed_resolves=Y
✅ needless_return                buggy_triggers=Y, fixed_resolves=Y
✅ redundant_closure              buggy_triggers=Y, fixed_resolves=Y
✅ redundant_field_names          buggy_triggers=Y, fixed_resolves=Y
✅ unnecessary_mut_passed         buggy_triggers=Y, fixed_resolves=Y
✅ unnecessary_unwrap             buggy_triggers=Y, fixed_resolves=Y
✅ useless_conversion             buggy_triggers=Y, fixed_resolves=Y

G6: ✅ ALL PASS (10/10 entries verified)
```

## G7: Self-evolution with real cargo clippy feedback

### Design

For each of the 10 verified lints, build a candidate set:
- **Correct fix**: the corpus's `fixed_code` (verified by G6)
- **Identity transform**: the buggy code itself (parses but doesn't resolve the lint)

Rotate which candidate the fallback drafter picks first (corpus[0]):
- Even-indexed lints: drafter picks correct → baseline resolved
- Odd-indexed lints: drafter picks identity → baseline NOT resolved

Expected baseline: ~50% (5/10).

### Evolution loop

3 rounds of: heal → verify with cargo clippy → report outcome (success/fail).

### Mechanism bug found + fixed

During G7 testing, found that 3 of 5 odd-indexed lints didn't converge. Root
cause: the `HealPipeline::heal()` method hardcoded `success_rate: 0.5` for
drafter candidates, ignoring the trajectory's actual fail stats. When the
identity transform failed, its trajectory's `success_rate` dropped to 0.0 in
memory — but the drafter candidate kept its hardcoded 0.5, causing a tie with
the correct candidate (also 0.5), and the drafter won ties.

**Fix:** added `LatentFixMemory::success_rate_for(lint_name, fixed_code)` and
updated `HealPipeline::heal()` to look up the actual trajectory stats for the
drafter candidate. Now, after a fail, the drafter candidate's `success_rate`
drops to 0.0, and the correct candidate (with 0.5 or higher) gets selected.

### Result (after fix)

```
Phase 1: baseline (seeded memory, no feedback)
  baseline: 5/10 = 50.0%

Phase 2: evolution loop (real cargo clippy feedback)
  round 1/3: 7/10 = 70.0% resolved
  round 2/3: 10/10 = 100.0% resolved
  round 3/3: 10/10 = 100.0% resolved

Phase 3: evolved rate (no more feedback)
  evolved: 10/10 = 100.0%

Improvement: 50.0pp (50% → 100%)
Target: ≥5.0pp
G7: ✅ PASS
```

All 5 odd-indexed lints converged to `source: Memory`, confirming the evolution
mechanism works end-to-end with real cargo clippy feedback.

## G5 vs G7 comparison

| Gate | Feedback source | Baseline | Evolved | Improvement |
|---|---|---|---|---|
| G5 (Issue 004) | Oracle (test knows correct answer) | 30.0% | 100.0% | 70.0pp |
| G7 (Issue 008) | Real cargo clippy | 50.0% | 100.0% | 50.0pp |

The gap (70pp vs 50pp) is explained by:
1. G7 uses 2 candidates per lint (not 4) → higher baseline (50% vs 25%)
2. G7's feedback is ground-truth but slower (seconds per verify vs instant oracle)

Both gates confirm the mechanism works. G7 confirms it generalizes to real
cargo clippy feedback.

## Promotion decision

`self_evolve` stays **opt-in** (default-off). Rationale:

1. The G7 fixture is still somewhat artificial (identity transform as the
   wrong candidate). Real-world clippy healing involves many candidate fixes
   per lint.
2. The `clippy_verify` integration is itself opt-in (slow, requires cargo on
   PATH). Promoting `self_evolve` to default-on without `clippy_verify`
   available means the feedback loop can't run.
3. The mechanism now depends on `success_rate_for` lookup in the hot path —
   a linear scan over all trajectories. Acceptable for POC, worth profiling
   at production scale.

The mechanism is validated against real cargo clippy feedback (G7 PASS).
The opt-in gate is a deployment-environment constraint, not a correctness
concern.

## Reproduction

```sh
cargo run --example self_evolve_real_world_demo \
  --features "self_evolve clippy_verify" --release
```

Expected output: G6 ✅ ALL PASS (10/10) + G7 ✅ PASS (50.0pp improvement).

Runtime: ~30s on Apple Silicon (cargo clippy is cached via BLAKE3-keyed dirs).
