# Bench 122 — the item-8 benchmark regression sweep (the five pinned CPU benches, post-i620 quiet box)

**Status:** COMPLETE — ZERO REGRESSIONS, all five verdicts reproduce their baselines (2026-10-28 ~02:2x+07, the shikuwa i7-13700K 4090 box).
**Commit:** measured at the post-merge develop tip (riir-refine synced through the overnight migration-lane batch; riir-infer + riir-ai ff-merged immediately before the runs — the deferred merges discharged at the i620 DONE line 2026-10-28 01:19:32).
**Provenance:** `box: shikuwa i7-13700K+RTX 4090 · power=AC · free RAM 15.5 GiB · gpu idle (4 %, background) · 02:2x+07, ~1 h after the i620 A/B DONE line — the box QUIET by the gate's own exit (no concurrent compute consumer)`. This is the sweep's first fully-quiet-box run: 116/117/118 were all run BESIDE GPU resident work (gpu=99-100 % rows in their records), CPU-quiet via the interleaved best-of-5 defense.

## Scope

The five pinned CPU benches (the x86-relevant GOAT set; bench_005 NEON stays skipped — aarch64 face per the 116 lane split):

| bench | baseline | verdict | key numbers |
|---|---|---|---|
| `bench_001_l1_matvec` | 001 | **ALL GATES PASS** | G1 bit-identical · G2 matvec p50 **2.300 µs** / p99 2.800 µs (bar < 10 µs, 4.3× under) · G3 CorpusTemplate fallback · G4 alloc-free |
| `bench_002_l2_pruner_syn` | 001 | **ALL GATES PASS** | G1 16/16 · G2 parse p50 **7.700 µs** / p99 12.6 µs (bar < 100 µs, 13× under) · G3 not-stub · G4 bounded |
| `bench_007_avx2_q2_x86_goat` | 116 | **PARETO-PASS (exit 0)** | G1 311 bit-identical · G2 25/25 shapes AVX2 strictly faster, **2.13–2.64×** · G4 zero-alloc |
| `bench_008_prescale_masked_field_goat` | 117 | **exit 1 BY DESIGN** (strict all-faces bar kept unsoftened) | G1 988 bit-identical · G4 zero-alloc · G2 split: 2-bit **1.28–1.31× WON** / 8-bit **~1.07× WON** / 4-bit **anti-case LOST** (anchor 0.449 vs 0.398 ms) — the 117 record's split REPRODUCES |
| `bench_009_f16c_lut_vs_intrinsic_goat` | 118 | **PARETO-PASS (exit 0)** | G1 65536/65536 widen + 132922 store cases (NaN disclosed) · G2 LUT strictly faster 9/9 dot cells (**intr/lut 6.14–6.16×**) + store no slower 9/9 (**soft/intr 24.5–33×**) · G4 zero-alloc |

## Comparisons (vs the pinned records)

- **001 (L1/L2, M3-era 2026-08-11):** the M3 p50s (2.250 / 5.459 µs) vs this box (2.300 / 7.700 µs) — in-family cross-host reads, both far under their bars; the bars are the gates, not the cross-host deltas.
- **116 (bench_007, this box 2026-10-14):** speedup band 2.20–2.94× then vs **2.13–2.64×** now — in-band; the verdict (25/25 strictly faster, G1 311 exact, G4 zero) identical. No regression.
- **117 (bench_008, this box):** the per-face split reproduces exactly — two faces win (2-bit ALU twin, 8-bit), one measured anti-case (4-bit, root-caused at the asm level in the record); exit 1 is the bench's own strict verdict discipline, never read as a sweep failure.
- **118 (bench_009, this box 2026-10-16):** PARETO-PASS on both axes at every cell, identical shape.

## What this closes

The item-8 benchmark regression sweep owed since the score-bench re-arm (`f2364275f`, run 205): the latency floors for the healer's hot paths (L1 drafter matvec, L2 parse check) + the three x86 GOAT kernel rules all hold at the post-overnight-merges develop tip. No drift → no bisect owed → the next scored corpus batch starts from a clean measured floor.

## Queue after

- The sweep ran ~1 h after the i620 A/B DONE (amo-i620 pre-registration, four arms STE/k50/k100/k200 — their analysis is THAT lane's, not this record's).
- Batch 298 (B298, bbtree) on the amo claim; walk #19 Wed ~02:15Z this lane; resin 08:28Z 10-28; nnethercote ~11-01.
