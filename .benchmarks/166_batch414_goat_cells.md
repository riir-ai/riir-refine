# Bench 166 — Batch 414 (nominal) GOAT cells: the relate storage/durability batch

**Plan:** 388 (`.plans/388_relate_storage_durability_batch.md`) · **Research:** 422 · **Date:** 2026-11-25
**Box:** m3 (Mac15,8 / Apple M3 Max 16-core / 64 GiB / macOS 27.2), **isolated target dir** `/tmp/plan388`.

**PROVENANCE (quoted from `scripts/bench_preflight.sh`, run immediately before the cell):**
`PROVENANCE: power=AC Power load=4.84 swap=1294.50M canary=skipped powermode=2(high) ane=ok`
— AC power, load 4.84/16 (a sibling regate test at ~1% CPU the only contender), High Power mode
(powermode=2 — max clocks, STABLE for a relative A/B; the mode is named, never hidden), ANE idle
through the 3 s probe window. Canary=skipped (the preflight's skip note, disclosed).

## The cell — R1 `membership-set-inline-words-tier-promote-at-seventeenth`

Instrument: the crate's own perf-bench Pareto validator
(`PERF_BENCH_FILTER=membership-set-inline-words cargo run --release --example perf_bench_demo
--features rust_perf,rust_perf_bench`, the Batch-48 batch-scoped filter — one rule, its harness
compiled as a standalone bin crate per the validator's contract, black_box at both ends, sizes
[4, 16, 17]).

| Run | Verdict | Faster/Slower/Tie (sizes 4/16/17) | Allocs buggy→fixed | Bytes buggy→fixed | G-det (3 repeats) |
|---|---|---|---|---|---|
| 1 | **ParetoDominant** | 3/0/0 | 0/0 | 0/0 | stable |
| 2 | **ParetoDominant** | 3/0/0 | 0/0 | 0/0 | stable |

**Two-run stable: ParetoDominant 3/0/0 at every size, both runs.**

- **G1 (correctness):** PASS — the harness's `debug_assert_eq!` holds the census equality on
  both arms every invocation (the fixed arm's `inline_contains` answers exactly the `HashSet`
  arm's membership over the mixed 1..=17-word-per-set corpus).
- **G2 (latency):** PASS as the Pareto verdict — the fixed (inline-mask) arm is ≥5% faster at
  ALL THREE sizes; the validator prints its per-size ratios in its own log.
- **G3 (no regression):** PASS — no arm slower at any size (0 in the slower column, both runs).
- **G4 (alloc):** PASS — allocs 0→0 and bytes 0→0 at every size (both arms are probe-path
  alloc-free by construction; the BUILD side is in `make_input`, outside the timed region — the
  harness-shape disclosure: the alloc axis here proves the probe path allocates nothing, the
  set-construction economics live in the fixture's F1 census).
- **G-det (alloc determinism):** PASS — the alloc axis is bit-identical across 3 repeat runs,
  both runs (trivially: 0/0).
- **G5 (Pareto promotion bar):** PASS — `pareto_dominant=1 (must be > half of 1 benchable)`.

Validator summary line (both runs):
`ROW membership-set-inline-words-tier-promote-at-seventeenth|ParetoDominant|3/0/0|0/0|0/0|stable`
`✓ ALL GOAT GATES PASS — G1 + G5 + G-det`

## R2–R6 — the pre-registered NotBenchable posture IS the instrument

Five rules land `bench_spec: None` with per-domain honest annotations
(`NotBenchable-by-domain`): integrity decision tables (R2/R3/R6), an fsync-count law (R4), and a
policy seam with an adversarial-key regime (R5) have no validator host — the Batch-136
precedent. Their fixtures (b414_perf F2–F6) carry the four-arm discipline (incl. tamper arms)
as the executable evidence; fixtures 6/6 at debug AND -O, zero warnings.

## Honest disclosures

- The validator's sizes are the spec's [4, 16, 17] — the inline ceiling and one past it. The
  regime evidence for LARGER sets (the promoted-table arm at scale) is the source's own T2
  design (a presized-32 table), not a measured cell here; the rule's anti-regime section names
  the oscillating-count case as measure-before-adopting.
- High Power mode (powermode=2) inflates absolute clocks; the verdict is RELATIVE (same-box
  interleaved arms through one harness), so the mode names the posture without invalidating
  the A/B. A low-power re-run would shift both arms' absolute numbers, not the ratio's class.
- The `swap=1294.50M` reading is the box's standing swap, not this cell's allocation (the
  alloc axis measured 0/0 through the counting allocator).
