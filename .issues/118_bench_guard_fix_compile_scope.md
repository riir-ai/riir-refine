# Issue 118 — Bench-target guard scope: `--fix-compile` edits on bench targets are measurement-altering
**Status:** RESOLVED 2026-09-16 — T1–T4 landed (see Landing record below).

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
- [x] T1 — `is_bench_target` wired into the `--fix-compile` write path
      (the candidate filter before the `best` pick in
      `rustc_errors::driver::run`): decline + report fields
      (`bench_declined`, `bench_declined_files`) + CLI loud lines.
- [x] T2 — Oracle-edits lane (P21) on bench spans: same door, same line
      (`n_bench_oracle_declined` + per-file decline print + run total).
- [x] T3 — Explicit-path override posture: named bench path (exact file
      or a `benches/` dir component) prints the flag and proceeds —
      `bench_guard::user_path_forces` + `ForcedPaths` in the driver + the
      `oracle_forced` check in the fix loop.
- [x] T4 — Fixture tests: `tests/fix_compile_bench_guard.rs` (real cargo,
      E0596 in both a lib and a `harness = false` bench target) — routine
      path declines + bench file byte-identical + lib heals unaffected;
      explicit path flags-and-proceeds to CLEAN. Matcher units in
      `src/bench_guard.rs` (8 tests).

## Landing record (2026-09-16)

- **The strategy classification the issue asked for**: shipped the
  CONSERVATIVE blanket decline — ALL driver writes on bench targets are
  declined on the routine path, no per-strategy safe subset admitted.
  Rationale: P30's own construction principle (a loud false decline
  beats a silent measurement break); the per-strategy split
  (import-only / rename-in-place as provably safe) is recorded here as
  the admission path when a REAL corpus need appears — not before
  (reopening trigger: a workspace `--fix-compile` run blocked on a bench
  target whose fix is demonstrably measurement-safe).
- `bench_guard` promoted to the crate top level (ungated) so the
  `rustc_errors` driver consumes it without gating on the `rust_perf`
  feature; `domains::rust_perf::bench_guard` re-exports it (P30 call
  sites unchanged).
- P30's founding scope sentence ("making a bench COMPILE is not
  measurement-altering") is superseded by this issue's re-adjudication
  — the module docs in `src/bench_guard.rs` record the reversal.

## Related
- P30 / `src/domains/rust_perf/bench_guard.rs` (the guard + its detection),
  HISTORY.md §P30 (the intake: katgpt-rs `bench_775_adjoint_goat.rs`).
