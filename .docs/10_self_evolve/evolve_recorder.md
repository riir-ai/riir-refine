# EvolveRecorder — the oracle-split recorder (Issue 018)

`src/recorder.rs`. The standalone consumer shape of the self-evolve layer:
tools that apply fixes through a path OTHER than the `HealPipeline` (the
`cargo heal --fix` proposers + edit emitter) still feed the same memory +
learning — with a **graceful oracle degradation** instead of a hard feature
dependency.

## The oracle split

| Half | Oracle? | Behavior without one |
|---|---|---|
| **Memory** (`record_applied` / `record_reviewed`) | None needed — every applied fix is a trajectory; a reverted fix is a definitive fail verdict | Runs in EVERY build |
| **Learning** (EMA direction update) | Needs trustworthy verdicts (`clippy_verify` compiled in, or a caller-owned sound source) | No-op + **exactly ONE** log line: `self-evolve: memory-only (learning disabled: …)` |

The split is a policy of the recorder, not of the primitive — callers with
their own sound verdict path can still drive
`LatentSkillEvolution::record_outcome` directly.

## API surface

```rust
EvolveRecorder::new()                 // learning = cfg!(feature = "clippy_verify")
EvolveRecorder::from_env()            // + RIIR_HEAL_SELF_EVOLVE opt-out
EvolveRecorder::with_opt_out(bool)    // explicit (test-friendly)
rec.record_applied(rule, buggy, fixed)          // memory only
rec.record_reviewed(&[(rule, line, repl, restore)])  // the --fix review shape
rec.record_outcome(rule, buggy, fixed, success) // memory stats always; EMA gated
rec.disable_learning(reason)                    // runtime oracle failure
rec.load(&path) / rec.save(&path)               // persistence (see latent_fix_memory.md)
```

## The `cargo heal --fix` wiring

- **Non-verify mode**: only WRITTEN edits are recorded
  (`record_reviewed`) — review-only runs record nothing.
- **`--verify` (fix_verify)**: kept edits join the memory as
  memory-only entries (a rustc pass proves non-breakage, NOT lint
  resolution — no verdict claimed); reverted edits record
  `success=false` (a compile failure IS a definitive verdict — sound to
  learn from when an oracle build enabled learning).
- **Why not the clippy verifier on span edits?**
  `ClippyVerifier` wraps code as a whole temp-crate lib — a span
  replacement referencing outer-scope names won't compile, and a
  non-compiling crate reports "lint absent" = false success. Span-level
  learning therefore rides the compile gate (sound verdicts only).
- **Persist once per run** when anything was recorded; the path resolves
  via `RIIR_HEAL_MEMORY` or the platform cache dir.

## Opt-out

`RIIR_HEAL_SELF_EVOLVE=0` (or `off`/`false`/`no`) — fully inert: no
recording, no persistence, no log lines. The pure heal loop.

## GOAT

Bench 023: contracts (G1), real-bin e2e accumulation + one-line + inert
opt-out (G2), 5-state clippy-clean + CI guard all-layers (G3), empty
`cargo tree` diff (G4), oracle-present mechanism regression-tested +
`HealPipeline` code paths untouched (G7 route).

## See also

- [`latent_fix_memory.md`](latent_fix_memory.md) — the memory + persistence format
- [`latent_skill_evolution.md`](latent_skill_evolution.md) — the EMA learning primitive
- [Bench 023](../../.benchmarks/023_self_evolve_promotion_goat.md)
