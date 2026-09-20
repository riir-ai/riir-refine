# Issue 127 — Kani oracle POC: machine-checked property evidence for sec rules

**Status:** OPEN — POC (poc/proof task, intake 2026-09-20; ref `.research/193_verus_program_verifier_distill.md`)

## Why

Research 078 graded Kani **B+** on the sec ORACLE ladder ("bit-precise overflow/panic/UB; needs harnesses") — and it has never been RUN. Sec rules today carry probe evidence only ("no default or restriction lint fires on the buggy form", clippy 1.93/1.97 — the T0.4 protocol). A machine-checked property harness adds a second, stronger evidence axis: prove the buggy form violates the property and the fixed form satisfies it.

## POC (bounded, 4 tasks)

- [ ] G1 — harness `nan-unsafe-sort-comparator` (the cleanest concrete property): Kani must exhibit a violating triple for the `partial_cmp(..).unwrap_or(Equal)` comparator (a non-total order) and verify totality + NaN-determinism for `total_cmp`.
- [ ] G2 — harness the wire-arithmetic lead from Research 193 (`from_le_bytes` operands flowing into `+`/`*`): the buggy form must reach an overflow on some input (existence check), the checked form must prove impossibility. Pre-validates the corpus candidate before any batch.
- [ ] Measure cost: cargo-kani install + toolchain pin, per-harness wall time, unwinding config needed, workstation-only feasibility (never a per-push gate).
- [ ] Verdict + record: if G1+G2 land, decide (owner call) whether the sec corpus doc table gains a "property evidence" column (harness id + verdict) beside "killed neighbours (evidence)"; if Kani cannot decide these small fixtures in bounded time, record NEGATIVE in the Research 193 note + here and close — the ladder stays probe-only.

## Explicit falsifier

Kani requires its own pinned nightly snapshot (not the workspace's `rust-toolchain.toml` stable) and bounded unwinding; if either makes harness cost exceed the value (minutes per fixture, toolchain friction), the POC records NEGATIVE — that is a valid outcome, not a failure.

## Non-goals

- No Verus adoption (rustc fork — refused in Research 193 Track B).
- No corpus edits under this issue; a wire-arithmetic batch (if the owner pulls the lead) opens its own plan-first batch.
