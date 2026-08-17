# Bench 038 — rust_perf Batch 53: pow/sqrt/hashmap/SoA online-search candidates

> **Date:** 2026-08-17 (evening)
> **Feature gate:** `rust_perf_bench` (opt-in)
> **Toolchain:** rustc 1.93.0 (the M3 box default)
> **Hardware:** Apple M3 Max (native, single-threaded timing, `--release`)
> **Environment:** sibling `bonsai_lora_accuracy_parity` CPU process active
> (~1 core of 14, `/tmp/i715_armc/`) during the run. Discharged empirically:
> the distance-sq rule was re-run as a separate invocation under the same load
> and reproduced the verdict + ratios exactly (1.21-1.24× both runs) — timing
> noise under the single-core sibling load is inside the harness's ±5% band.
> The sibling measurement itself is CPU-deterministic (accuracy parity), so
> this run could not corrupt it.
> **Sweep cost:** 4 rules × 3 determinism repeats (~4 min) + 2 single-rule
> verification re-runs.
> **GOAT:** G1 PASS (4/4 real measurements), G-det PASS (alloc axis
> bit-identical across 3 runs on all 4), G5 batch-scoped **FAIL 2/4** (the
> honest-refutation pattern — the 2 non-dominant rules land with
> `context_notes`, the Batch 48 `filter-count-to-any` precedent).

## Context

Batch 53 = idle-queue item 11 (online rust/kernel perf search), executed
2026-08-17 evening. Four candidates distilled from users.rust-lang.org
("powf is extremely slow" t/56451, "preallocate your HashMaps" t/44322),
baptiste-wicht.com's C++ std::pow study, and the Box2D squared-distance SIMD
convention — dedup-grepped against the 58-rule corpus (no pow / sqrt /
map-capacity / SoA rule existed). Authored + benched same session.

## Results (timing = median-of-21, ±5% Tie band; allocs = steady-state integers)

| Rule | Sizes | Speedups | Verdict | allocs b→f @max | bytes b→f @max |
|---|---|---|---|---:|---:|
| distance-sq-no-sqrt | 1K/100K/1M | **1.21× / 1.23× / 1.24×** | **ParetoDominant 3/0/0** | 0→0 | 0→0 |
| hashmap-with-capacity | 8/64/512/4096 | **2.39× / 2.94× / 3.09× / 2.90×** | **ParetoDominant 4/0/0** | 12→1 | 278,556→139,272 |
| powi-vs-powf | 1K/100K/1M | 1.00× / 1.00× / 1.00× | Mixed 0/0/3 (all Tie) | 0→0 | 0→0 |
| soa-hot-field | 1K/100K/1M | 1.07× / 0.99× / 0.96× | Mixed 1/0/2 | 0→0 | 0→0 |

Alloc determinism: `stable` on all 4 across 3 repeat runs.

## The two honest refutations (the batch's real findings)

1. **powi-vs-powf — the "powf is slow" folk wisdom is FALSE for the
   constant-exponent-2 release case.** LLVM instcombine lowers `pow(x, 2.0)`
   to the same `x * x` multiply as `powi(2)` — measured bit-equal timing at
   every size. The rule lands as an idiom with a scoped note: it still matters
   for debug-profile hot loops (no instcombine; powf stays a libm call), for
   exponents ≥ 3 (`pow(x, 3.0)` is NOT lowered — correctly-rounded pow differs
   from `x*x*x`, so it stays a libm call while `powi(3)` becomes two
   multiplies), and for readability. Mirrors the `filter-count-to-any`
   folk-wisdom refutation (Bench 48).

2. **soa-hot-field — the serial f32 add-reduction chain hides the layout win.**
   A scalar `.sum()` over the hot field is latency-bound on the ~3-4 cycle f32
   add dependency chain on BOTH layouts; the 40-byte-stride load latency is
   fully absorbed by out-of-order execution, so AoS ties SoA at 100K/1M
   (memory bandwidth is not the bottleneck for a serial sum). The SoA win
   materializes on throughput-bound field consumers (count/filter, min/max,
   batch transform — vectorizable, no serial chain) or multi-field scans. The
   note scopes the suggestion to those shapes.

## The two wins

- **distance-sq-no-sqrt** (1.21-1.24×, consistent): dropping the sqrt from a
  compare-only radius check wins even on aarch64 where `fsqrt` is a single
  NEON instruction — the fixed form also vectorizes while the sqrt call
  sequence constrains it. Semantic guards in the rule: keep the sqrt when the
  distance VALUE is consumed; NaN inputs order identically-false under both
  forms in practice.
- **hashmap-with-capacity** (2.4-3.1×, the batch's largest win): the rehash
  ladder re-runs SipHash over every stored element at each of the ~log₂(n)
  resizes; `with_capacity(n)` with a known count pays one table alloc + zero
  rehash passes. The alloc axis nails it independently of timing: 12→1
  allocations, bytes exactly halved (the ladder's cumulative tables vs one).

## Corpus + gates

- Corpus 58 → **62 rules** (allocation 17→18, layout 4→5, arithmetic 1→3).
  Annotated (context_notes) 10 → **12**; unannotated 48 → **50**; benchable
  48 → **52** (NotBenchable stays 10).
- `PERF_SPANS` 48 → **52** (one span per new rule, the Batch 48 convention) —
  all 4 retrieve their rule **top-1**.
- Retrieval floors re-pinned per the measured−1-span convention (2-domain):
  blended 92/96/96 of 100, clippy 42/45/45 of 48 (unchanged), perf top-3
  50/52, router 65/100. 3-domain (kernel state): kernel self-variants
  100/100/100, blended 94.6/97.7/97.7 of 130, router 89/130 → floor 84/130.
- Gates: 496 default lib / 514 `rust_perf_bench` lib / retrieval gate PASS
  both states / clippy 0 on the lib with the feature on.
- G5 batch bar (2/4 > half): **FAIL** — recorded honestly; the 2 Mixed rules
  carry verdict-class `context_notes` (the promotion bar for THIS batch is the
  annotation discipline, not unconditional landing).

## Honest scope notes

- Proposers (`--fix` eligibility) are a follow-up unit (the Batch 48
  precedent: rules landed first, 3 proposers landed the next day). Preliminary
  assessment: `powi-vs-powf` is a clean local rewrite (method-call swap);
  `distance-sq-no-sqrt` is structural but tractable (`Lt(sqrt(a), b)` →
  `Lt(a, b·b)`); `hashmap-with-capacity` needs flow analysis for the
  known-count precondition (safe subset only); `soa-hot-field` is
  architectural — never locally fixable, suggestion-only by design.
- The powi note's exponent-≥3 claim is compiler-behavior knowledge (LLVM
  lowers pow only for exponents where the expansion is exactly rounded:
  0.5→sqrt, 1, 2; 3+ needs fast-math), not measured in this bench.
