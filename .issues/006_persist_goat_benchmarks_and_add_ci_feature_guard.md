# Issue 006 — Persist GOAT Benchmark Records + Add CI Feature Guard

**Filed:** 2026-08-11
**Type:** Repo hygiene — close two standing gaps every sibling repo addresses
**Status:** resolved (T1-T4 all done, 2026-08-11)

## Problem

Two standing gaps in riir-clippy that every other repo in the 7-repo stack
addresses:

1. **`.benchmarks/` is empty.** L1 (Issue 001), L2 (Issue 002), Phase 3
   (Issue 003), + L3 (Issue 005) all shipped GOAT gates, but the results
   were never persisted as markdown records. Per global AGENTS.md: "Dont
   defer benchmark task." A GOAT result that isn't recorded is a one-time
   measurement, not a standing gate (the exact lesson documented in
   `riir-chain/AGENTS.md` §"Gate Coverage").

2. **No CI feature guard script.** Sibling repos (`riir-chain`, `riir-ai`,
   `riir-neuron-db`) all ship `scripts/ci_feature_guard.sh` to catch the
   `merkle_root` / `can_freeze` class of bug: a field-gate that only fires
   under a specific feature combination. riir-clippy has 5 features
   (`clippy_verify`, `ruliology_search`, `wasm_pruner`, `clippy_lora`,
   `self_evolve`) + zero CI gates beyond `cargo check`.

## Scope

### Part A — `.benchmarks/001_goat_consolidated.md`

Persist the measured GOAT results for the four committed layers
(L1/L2/Phase3/L3). Captures: gate definition, target, measured value,
hardware, date, commit hash. The Issue 004 (self_evolve) GOAT results
ship in a separate record once that work commits — this record covers
only what's on `develop` today.

### Part B — `scripts/ci_feature_guard.sh`

Mirror the sibling-repo pattern. Layers:

1. `cargo check --no-default-features` (zero-dep baseline — only katgpt-core)
2. `cargo check` (default features)
3. `cargo check --all-features` (catches combo-only regressions — the
   `merkle_root`/`can_freeze` lesson class)
4. `cargo clippy --all-targets --no-default-features` + `--all-features`
5. `cargo test --lib` (default + all-features)

No Lean proof gate (this repo has no `.proofs/`). No wasm32 gate (this
repo has no wasm32 target — the `wasm_pruner` feature is deferred on
rubrc).

## Tasks

- [x] **T1:** Write `.benchmarks/001_goat_consolidated.md` with measured
      L1/L2/Phase3/L3 results (captured 2026-08-11 on M3 Max, release mode).
- [x] **T2:** Write `scripts/ci_feature_guard.sh` mirroring sibling-repo
      pattern (5 layers, no Lean, no wasm32).
- [x] **T3:** Run the script locally — all 5 layers PASS (35 default + 93
      all-features lib tests, 8 single-feature combos, clippy clean both extremes).
- [x] **T4:** Update README.md + AGENTS.md to reference the new artifacts.

## GOAT gate

This is a hygiene issue, not a perf-gain issue. The "gate" is: does the
CI script pass on a clean tree? If yes, the repo now has the same
feature-combo regression coverage as its siblings.

## Out of scope

- **Issue 004 (self_evolve) benchmark record** — ships when the sibling
  agent commits their work. Not mine to persist.
- **`.docs/` book** — separate hygiene task if desired.
- **Benchmark for `clippy_verify`** — it's a slow (~130ms/verify) external
  process gate, not a hot-path perf gate. The Phase 3 issue already
  documents the timing.

## References

- `riir-chain/scripts/ci_feature_guard.sh` — the canonical 5-layer pattern
- `riir-chain/AGENTS.md` §"Gate Coverage" — the "a gate nobody runs is
  documentation, not a gate" lesson
- `riir-neuron-db/AGENTS.md` §"The `merkle_root` Lesson" — the field-gate
  combo bug class
