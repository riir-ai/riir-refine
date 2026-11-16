# 155 — Batch 365 GOAT cell: the hydradb second-tranche benchable lead (Plan 379 T3)

**Status:** R3 `existential-witness-early-exit-scan` GOAT PASS per the plan's pre-registered bar (the tranche's only benchable rule — the other five landed NotBenchable by domain or by model, the adjudication inline below).

Box state: m3 (M3 Max 16-core), AC power, Zed idle between runs, load ~2 during the cells, target warm. The validator's own cargo invocation builds release; timing medians, 21 samples, dynamically calibrated ~1 ms/sample.

## Adjudication at authoring (which tranche rules get cells)

- `existential-witness-early-exit-scan` — BENCHABLE (predicate arity is an integer-exact scan-shape decision; the read-count axis is modelable honestly through a touch table).
- `wal-watermark-pinned-before-snapshot` / `tail-walk-cost-cap-declines-to-rebuild` / `frozen-artifact-plus-delta-tail-overlay` — NotBenchable-by-domain (a correctness ORDERING law / an economics DECLINE / a freshness-COMPOSITION law — the failure or decision lives outside this process's wall-clock; the dial9/B352 annotation class).
- `pending-claim-registry-take-consumes-drop-clears` — NotBenchable-by-model (a synthetic invalidation constant would measure OUR model, never an engine's sweep — the `derived-index-write-touches-only-changed-keys` class from Batch 364).
- `one-shared-disk-cache-not-even-split` — NotBenchable-by-domain (the win is a cache-QUALITY axis — refetches/hit-rate under a skewed working set; the validator's ns/alloc axes cannot express it — the QUALITY ordering is fixture-pinned in b365_fixtures F6: at the fixture's skewed trace the one-pool design refetches less than half what the even-split design does at the same total budget).

## Bars (pre-registered at claim time, BEFORE the first run)

- `existential-witness-early-exit-scan`: probe-count integer-exact through a 1M-entry touch table, Pareto validator, two-run stable (alloc axis bit-identical), wins >= 2/3 sizes, never dominated. Fails => demote same-commit (the ungoat law).

## Cell — `existential-witness-early-exit-scan`: PASS

Instrument: `cargo test -p riir-refine --lib --features rust_perf_bench batch365_goat -- --ignored --nocapture` (the validator generates a temp bench crate per rule; `run_bench_with_determinism(rid, spec, 2)` runs the binary twice per invocation and asserts the alloc axis bit-identical — green on both runs).

Run 1:

| candidates | buggy ns | fixed ns | speedup | allocs (buggy->fixed) | outcome |
|---|---|---|---|---|---|
| 1 024 | 21 629 | 3 614 | 5.98x | 0->0 | Faster |
| 16 384 | 646 000 | 113 322 | 5.70x | 0->0 | Faster |
| 262 144 | 11 334 416 | 2 996 875 | 3.78x | 0->0 | Faster |

Run 2 (same session, minutes later):

| candidates | buggy ns | fixed ns | speedup | allocs (buggy->fixed) | outcome |
|---|---|---|---|---|---|
| 1 024 | 21 792 | 3 531 | 6.17x | 0->0 | Faster |
| 16 384 | 648 459 | 110 921 | 5.85x | 0->0 | Faster |
| 262 144 | 10 822 833 | 2 874 875 | 3.76x | 0->0 | Faster |

- Verdict class stable across the two runs (5.98/5.70/3.78 vs 6.17/5.85/3.76); the alloc axis (0->0 at every size) bit-identical => two-run stable.
- G1 census parity: asserted inside the harness's `make_input` (both arms report the same witness-hit census AND the same universal-arity reads where the law forbids the early exit) — green on every run.
- Plan bar: wins >= 2/3 sizes MET (3/3); never dominated MET (slower=0).
- The margin is the arity dispatch itself: the 85% existential population stops at its first witness (~1/3 depth by the input model) while the naive reads every prefix; the 15% universal population reads identically in both arms (the law's forbidden-early-exit half is IN the harness — the fixture's F1 anti-regime arm pins it separately).
- The speedup exceeds a pure read-count ratio (~2x expected) because the early exit also skips the touch-table folds' dependent loads — the reads saved are the SECOND half of each candidate's stream (cache-local in the fixture), so the wall-clock wins over-proportionally; the census parity keeps the claim honest.

**Verdict: GOAT PASS** — the rule enters BENCHABLE (pins: rust_perf 347 -> 353 rules; benchable 137 -> 138; the kernel-side witness re-pinned 347 -> 353 in the same commit).
