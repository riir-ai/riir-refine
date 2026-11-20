# Bench 163 — Batch 374 GOAT cells: the laya re-distill's R1 batching lead

**Status:** RECORD — Plan 390 T4. One BENCHABLE rule in the batch (`batch-mixed-length-pads-to-longest-group-by-length`, entries_87); the other three land suggestion-only `NotBenchable` `bench_spec: None` (the metric-mismatch class — decision-content/honesty rules have no validator host, disclosed in-rule).

## Box state (PROVENANCE)

- m3 (Apple M3 Max, 16 cores), 2026-11-20 19:00–19:05 local; **AC power, battery 100% charged; powermode = 2 (High Power)**.
- Load average 11.72/10.17/7.47 at cell start (the box's sibling-agent baseline; **no competing cargo process** — `ps aux | grep cargo` clean apart from this lane); rig reads m3 QUIET (agent-mail).
- Isolated target dir `/tmp/b374_target`; the validator's scratch crate compiles its own temp bin (the probe-identity pin holds — Issue 110/112 class guarded).
- Release build of the example (`cargo build --release --example perf_bench_demo --features rust_perf_bench`, 2m15s cold-ish).

## The cell (PerfBenchValidator, the crate's own validator — not the entries_17 fallback)

`PERF_BENCH_FILTER=batch-mixed-length perf_bench_demo --release`, TWO RUNS, both:

```
ROW batch-mixed-length-pads-to-longest-group-by-length|ParetoDominant|3/0/0|1/1025|2097152/4194304|stable
✓ ALL GOAT GATES PASS — G1 + G5 + G-det
```

| n rows | buggy | fixed | speedup | verdict |
|---|---|---|---|---|
| 1,024 | 383,875 ns | 63,122 ns | **6.08×** | FASTER |
| 16,384 | 6,054,125 ns | 1,030,000 ns | **5.88×** | FASTER |
| 262,144 | 96,592,834 ns | 16,622,750 ns | **5.81×** | FASTER |

- **G1** PASS — the fixture (b374_fixtures.rs F1) pins per-row results BIT-IDENTICAL across arms (reorder-only; the pad is inert), the padded-element census strictly smaller on the mixed distribution (≥25% cut pinned), the UNIFORM-length control arm Ties (the anti-regime is part of the fixture), and the tampered no-restore arm reds.
- **G2** PASS — ParetoDominant **3/0/0 TWO-RUN STABLE** (both runs printed the identical verdict row; the pre-registered bar was ≥2 of 3 sizes dominant, never-slower on the third — every size faster, twice). Speedups 5.8–6.1× vs the source-reported 1.42–2.15×: OUR fixture's bimodal+tail mix is heavier than their serving mix (every 16th row 200–400 tokens beside 4–11-token rows — the pass max is tail-dominated); the source numbers stay provenance, never bars; the fixture's numbers are the claim (pre-registered in plan 390).
- **G3** PASS — the default-feature lib suite green (3,935 passed, the batch's pins re-landed); no regression anywhere in the touched postures.
- **G4 / G-det** PASS — alloc axis bit-identical across 3 repeat runs (the validator's determinism proof). The alloc axis is DISCLOSED, not a win: the fixed arm allocates the sort's index vectors (1 → 5 allocs at 1K, 65 at 16K; bytes 8 KiB → 16 KiB at 1K, 2,097,152 → 4,194,304 at 262K) — the rule's claim is the WALL axis (the pre-registered bars named the padded-work model; the alloc cost is the bounded-window sort's price, carried in-rule).

## Verdict

**GOAT PASS** — the rule enters the corpus BENCHABLE as landed (ParetoDominant, two-run stable). No demotion trigger fired; the anti-regime and the FP-near-threshold caveat ride the rule text.
