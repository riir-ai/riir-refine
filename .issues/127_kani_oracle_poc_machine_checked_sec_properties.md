# Issue 127 — Kani oracle POC: machine-checked property evidence for sec rules

**Status:** OPEN — POC EXECUTED 2026-09-20, **verdict POSITIVE** (G1+G2 land, cost floor measured); ONE item remains: the owner call on the corpus doc column. Harnesses: [`.proofs/kani_sec_poc/`](../.proofs/kani_sec_poc/) (standalone, detached, WSL-only lane).

## Why

Research 078 graded Kani **B+** on the sec ORACLE ladder ("bit-precise overflow/panic/UB; needs harnesses") — and it has never been RUN. Sec rules today carry probe evidence only ("no default or restriction lint fires on the buggy form", clippy 1.93/1.97 — the T0.4 protocol). A machine-checked property harness adds a second, stronger evidence axis: prove the buggy form violates the property and the fixed form satisfies it.

## POC (bounded, 4 tasks)

- [x] G1 — harness `nan-unsafe-sort-comparator` (the cleanest concrete property): Kani must exhibit a violating triple for the `partial_cmp(..).unwrap_or(Equal)` comparator (a non-total order) and verify totality + NaN-determinism for `total_cmp`. **DONE 2026-09-20: violating triple EXHIBITED — x=`-inf`, y=`+NaN`, z=`+inf` (cmp(x,y)=Equal, cmp(y,z)=Equal, cmp(x,z)=Less; "comparator not total" assertion FAILED with witness via `--concrete-playback print`); `total_cmp` VERIFIED transitive-on-Equal + total + antisymmetric + NaN-self-Equal (3 SUCCESS harnesses). Solver 0.03–0.07 s each.**
- [x] G2 — harness the wire-arithmetic lead from Research 193 (`from_le_bytes` operands flowing into `+`/`*`): the buggy form must reach an overflow on some input (existence check), the checked form must prove impossibility. Pre-validates the corpus candidate before any batch. **DONE 2026-09-20: existence WITNESSED (a=b=`0xFFFFFFFF`, Kani's arithmetic-overflow check FAILED inside the buggy callee + a playback witness); checked form VERIFIED two-sided (Some ⟺ exact u64 sum, None ⟺ true sum > u32::MAX). The lead's property claims are machine-checkable — pre-validation PASSED.**
- [x] Measure cost: cargo-kani install + toolchain pin, per-harness wall time, unwinding config needed, workstation-only feasibility (never a per-push gate). **MEASURED: install 8.6 s (warm cache) + setup 51.2 s (pinned nightly-2026-08-21 download); per-harness 0.15–0.45 s wall / 0.03–0.07 s solver; NO unwinding config (loop-free fixtures); lane = WSL2 Ubuntu 22.04 on the 4090 box (Windows-native unsupported — the WSL requirement is itself part of the feasibility verdict). Toolchain pin: Kani 0.68.0 / CBMC 6.11.0 / rustc 1.100.0-nightly(2026-08-21), fully detached from the workspace stable pins (fixtures carry zero deps). CAVEAT: this is the floor, not the ceiling — the minutes-per-fixture regime (loops/unwind, heap, real code) was never approached and stays unmeasured.**
- [ ] Verdict + record: G1+G2 landed → **OWNER CALL pending**: does the sec corpus doc table gain a "property evidence" column (harness id + verdict) beside "killed neighbours (evidence)"? (All POC evidence + the pending-question framing recorded here, in [`.proofs/kani_sec_poc/README.md`](../.proofs/kani_sec_poc/README.md), Research 193's status delta, and HISTORY.md 2026-09-20.)

## Explicit falsifier

Kani requires its own pinned nightly snapshot (not the workspace's `rust-toolchain.toml` stable) and bounded unwinding; if either makes harness cost exceed the value (minutes per fixture, toolchain friction), the POC records NEGATIVE — that is a valid outcome, not a failure.

## Non-goals

- No Verus adoption (rustc fork — refused in Research 193 Track B).
- No corpus edits under this issue; a wire-arithmetic batch (if the owner pulls the lead) opens its own plan-first batch.
