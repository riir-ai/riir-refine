# 154 — Batch 364 GOAT cells: the hydradb distill benchable pair (Plan 379 T7)

**Status:** R1 GOAT PASS (delegated-bar shape: the plan's pre-registered bars, measured); R2 FAILED the bar — DEMOTED same-commit per the ungoat law. Records below.

Box state: m3 (M3 Max 16-core), AC power, load 2.0-2.5 during the cells (Zed + the corpus gates idle between runs), target warm. The validator's own cargo invocation builds release; timing medians, 21 samples, dynamically calibrated ~1 ms/sample.

## Bars (pre-registered in Plan 379 at staging, BEFORE the first run)

- GOAT lead `degree-gated-probe-with-doubling-retry`: Pareto validator, integer-exact (reads counted through a 1M-entry touch table), two-run stable, wins >= 2/3 sizes. Fails => demote same-commit (the ungoat law).
- Co-lead `bfs-depth-epoch-marker-dedup`: memset-count/alloc axis (N memsets -> 0), wins >= 2/3 sizes, G1 bit-identical traversal order.

## Cell 1 — `degree-gated-probe-with-doubling-retry`: PASS

Instrument: `cargo test -p riir-refine --lib --features rust_perf_bench batch364_goat -- --ignored --nocapture` (the validator generates a temp bench crate per rule; `run_bench_with_determinism(rid, spec, 2)` runs the binary twice and asserts the alloc axis bit-identical).

| n | buggy ns | fixed ns | speedup | allocs (buggy->fixed) | outcome |
|---|---|---|---|---|---|
| 1 024 | 12 709 | 11 978 | 1.06x | 0->0 | Faster |
| 16 384 | 182 858 | 173 408 | 1.05x | 0->0 | Faster |
| 262 144 | 3 942 667 | 3 616 041 | 1.09x | 0->0 | Faster |

- Run 1 (same session, minutes earlier): 1.06x / 1.06x / 1.09x — verdict class stable, the alloc axis (0->0 at every size) bit-identical across runs => two-run stable.
- G1 census parity: asserted inside the harness's `make_input` (both arms count the same matched edges) — green on every run.
- Plan bar: wins >= 2/3 sizes MET (3/3 above the tie band); never dominated MET (slower=0).
- The margin is the seek-vs-scan crossover on the 70% low-degree mass; the hub tail is where the LAW's watermark half earns its keep (a much larger hub still probes after an overflow — fixture-pinned).

**Verdict: GOAT PASS** — the rule enters BENCHABLE (pins: benchable 137 = 136 + 1 net; the co-lead's demotion is below).

## Cell 2 — `bfs-depth-epoch-marker-dedup`: FAIL -> DEMOTED

| n | buggy ns | fixed ns | speedup | allocs | outcome |
|---|---|---|---|---|---|
| 1 024 | 5 348 | 5 621 | 0.95x | 0->0 | Tie |
| 16 384 | 148 659 | 148 118 | 1.00x | 0->0 | Tie |
| 131 072 | 2 416 459 | 2 411 625 | 1.00x | 0->0 | Tie |

- Plan bar: wins >= 2/3 sizes **FAILED** (0/3).
- The honest reading: the ring+chords harness (2 random chords/vertex, avg degree 4) has diameter ~log_4(n) — at n=131 072 the walk saturates by level ~9, so the naive pays ~9 fills of n bytes (~1.1 MB, ~25 us) against ~2.4 ms of boundary edge work: the fill is ~1% of the walk. The LAW is real (the fixture censuses the memset-BYTES axis: N clears -> 0, G1 visited-census parity green) but wall-clock on realistic graphs does not reward it at these scales.
- The harness that WOULD make the fill dominate (a pure ring, diameter n/2) is the degenerate-topology trap — the synthetic-model class this batch refused for candidate #4 (a validator would measure OUR graph shape, not a serving walk).
- **Demotion executed same-commit** (the ungoat law): `bench_spec: None`, the rule lands NotBenchable-BY-MEASUREMENT as decision-content, the wall-clock claim WITHDRAWN in its notes, the harness fn kept unreferenced as the record of what was measured. Pins: benchable 137 (136 + R1), notbenchable 210 (206 + R2 demoted + the three honesty/io rules).

## Discrimination note (the memset-count axis is NOT dead)

The corpus keeps the memset-BYTES census in `b364_fixtures` (F2: N clears vs 0 + the 255-wrap reset census + visited-census parity). A future consumer whose walk is genuinely frontier-bound (a ring-class topology, an explicit ladder, a walk where boundary re-scan is cheap) re-opens the benchable claim with THAT posture pinned — never by re-running this cell.

## Where the numbers live

- The test: `domains::rust_perf::corpus::tests::batch364_goat_cells` (ignored by default; the command above).
- The harnesses: `entries_80.rs` (R1's referenced by the live spec; R2's kept as the demoted record).
- Fixtures: `corpus/fixtures/b364_fixtures.rs` (16/16 standalone pre-entry).
- Plan 379 T7 discharge + the queue-snapshot row ride the landing commit.

Session: zed-m3-i379-hydradb@m3, epoch 1794697180 (claim a2b0e83c8).
