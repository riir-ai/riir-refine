# Issue 118 — Bench-target guard scope: `--fix-compile` edits on bench targets are measurement-altering
**Status:** OPEN — P2, unblocked. Filed 2026-09-16 from the room-to-improve triage (session record: HISTORY.md same-day entry).

## Why

P30 (2026-09-16) landed DECLINE-OR-FLAG for **rust_perf** rules on bench
targets (`src/domains/rust_perf/bench_guard.rs`): a bench file's numbers ARE
its artifact, and `--verify` proves compilation, never measurement validity.
The recorded scope was deliberate: "PERF-domain only — clippy lints,
`--fix-compile`, sec/docker/dist are out of scope (the recorded defect
class)."

`--fix-compile` is the next domain over with the same exposure: a compile
repair that rewrites a bench target's code can change WHAT the bench
measures (hoisting a loop, changing a type's size/copy semantics, removing a
"dead" result that was the timing subject — the exact
`dead-result-timing-loop-folded-under-lto` specimen that motivated P30, which
was itself a compile-shape edit class). The rustc_errors driver's write path
currently has no bench-target awareness.

Clippy-lint fixes on benches are *mostly* measurement-safe (style rewrites),
but the honest boundary is per-strategy, not per-domain: some rustc_errors
strategies are pure syntax while others move code.

## Shape of the fix

- Reuse `rust_perf::bench_guard::is_bench_target(path, src)` as-is (it is
  domain-agnostic by construction: path axis + harness markers).
- Classify the rustc_errors strategy menu into measurement-safe vs
  measurement-altering (import-only / rename-in-place vs anything that moves
  statements, changes types, or deletes bindings). Start conservative:
  decline ALL driver writes on bench targets with the loud `bench-guard:`
  line (the P30 precedent), then admit the provably-safe subset if a real
  corpus need appears — a loud false decline beats a silent measurement
  break (P30's own construction principle).
- The `--clippy-json` oracle-edits lane (P21) on bench targets rides the
  same guard: MachineApplicable suggestions at bench spans are consumed
  today; they should hit the same decline-or-flag door.

## Honest caveats

- A compile ERROR in a bench target still must be fixable — the guard gates
  the routine write path, not a user-forced `--fix-compile <bench path>`
  invocation; an explicit-path override prints the flag and proceeds (the
  P30 "apply manually only" posture).
- Record the strategy classification in the issue on landing — it is the
  adjudication artifact, the P30 SKILL.md-row precedent.

## Tasks
- [ ] T1 — Wire `is_bench_target` into the `--fix-compile` write path:
      decline + loud line + run-summary total (the P30 lane shape).
- [ ] T2 — Oracle-edits lane (P21) on bench spans: same door, same line.
- [ ] T3 — Explicit-path override posture: named bench path prints the flag
      and proceeds (never a silent hard-refusal on a user-forced target).
- [ ] T4 — Fixture tests: synthetic `benches/` compile-error specimen —
      routine path declines, explicit path flags-and-proceeds; identical
      error in `src/` heals unaffected.

## Related
- P30 / `src/domains/rust_perf/bench_guard.rs` (the guard + its detection),
  HISTORY.md §P30 (the intake: katgpt-rs `bench_775_adjoint_goat.rs`).
