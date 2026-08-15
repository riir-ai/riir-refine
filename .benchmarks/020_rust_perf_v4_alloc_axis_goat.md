# Bench 020 — rust_perf corpus v4: deterministic alloc-count axis + profiling-guide rules (Issue 014)

> **Date:** 2026-08-16
> **Feature gate:** `rust_perf_bench` (opt-in)
> **Toolchain:** rustc 1.97.0 (2026-07-07)
> **Hardware:** Apple M3 Max (native, single-threaded, `--release`)
> **Sweep cost:** 1,606 s (~26.8 min) — 40 rules × 3 determinism repeats (120 executions)
> **GOAT:** G1 PASS, G5 PASS, **G-det PASS** (alloc axis bit-identical across 3 repeat runs on ALL 40 benchable rules)

## Context

[Research 032](../.research/032_hotpath_profiling_guide_distillation.md) (hotpath-rs
profiling guide) transferred two things: (1) **deterministic signals (alloc
count/bytes) over noisy wall-clock** — the Pareto validator measured timing
only and documented its own ±5% Tie-band noise; (2) **4 rule candidates**
with no shipped corpus equivalent (grep-verified). Issue 014 executes both.

The alloc-axis substrate (T1.1/T1.2 — counting `#[global_allocator]` injected
into the generated bench crate, `SizeResult` alloc fields,
`run_bench_with_determinism`) landed in commit `2ace442` but **incomplete**:
the `alloc_axis` comparison function was called but never defined, and the
injected allocator block's `use` statements collided with harness imports
(E0252 on `Ordering` — `mutex-to-atomic`'s harness failed to compile). This
session completed + repaired the substrate (this bench is the proof):

1. **`alloc_axis()` implemented** — the `(size, buggy_allocs, buggy_alloc_bytes,
   fixed_allocs, fixed_alloc_bytes)` tuple vector; equality across runs is the
   determinism proof. Unit-tested (identical-input equality, timing-irrelevance,
   any-integer-difference inequality).
2. **Allocator block made collision-proof** — fully-qualified paths, zero `use`
   statements. The injected block sits at module scope next to the harness's
   own imports; a `use` there collides with any harness importing the same
   name (`Ordering` is common in atomics benches). The first T1.3 sweep run
   caught this live (`mutex-to-atomic` FAILED with E0252); the fix re-ran clean.
3. **`perf_bench_demo` extended** — per-size alloc columns, 3× determinism
   repeats per rule, a G-det gate, machine-readable `REPORT_ROWS` output.

## The alloc axis (T1.2/T1.3) — every benchable rule, both axes

Timing = run 1 (median-of-21 samples, ±5% Tie band). Allocs/bytes = steady-state
per-call integers after warmup (one-time init excluded), at the LARGEST size.
`det=stable` = identical integers across all 3 repeat runs.

| Rule | Timing verdict | allocs b→f | bytes b→f | det |
|---|---|---:|---:|---|
| arc-str-payload | PD 3/0/0 | 1001→2 | 1,056,000→25,040 | stable |
| avoid-clone | PD 4/0/0 | 1→0 | 100,000→0 | stable |
| bool-to-bitpack | PD 3/0/1 | 0→0 | 0→0 | stable |
| box-avoid | PD 3/0/0 | 100,001→1 | 1,600,000→800,000 | stable |
| box-str-immutable | PD 2/0/1 | 0→0 | 0→0 | stable |
| branchless-conditional | PD 4/0/0 | 0→0 | 0→0 | stable |
| bsearch-sorted-slice | PD 4/0/0 | 0→0 | 0→0 | stable |
| btreemap-sorted-iteration | PD 4/0/0 | 5000→5000 | 1.0e9→1.0e9 | stable |
| buffered-io | PD 2/0/0 | 0→1 | 0→8192 | stable |
| bufwriter-batched | PD 3/0/0 | 2→3 | 279→8471 | stable |
| chunks-exact | PD 4/0/0 | 0→0 | 0→0 | stable |
| collect-avoidance | PD 4/0/0 | 1→0 | 8,000,000→0 | stable |
| copy-from-slice | PD 4/0/0 | 1→1 | 4096→4096 | stable |
| cow-conditional-alloc | PD 4/0/0 | 75,000→30,000 | 3,085,002→1,835,001 | stable |
| dedupe-hashset-vs-vec | PD 3/1/0 | 11→2 | 32,752→50,968 | stable |
| dyn-to-enum | PD 3/0/0 | 3→1 | 64→48 | stable |
| entry-vs-contains-insert | PD 4/0/0 | 1→1 | 1,179,656→1,179,656 | stable |
| f32-counter-to-int | PD 4/0/0 | 0→0 | 0→0 | stable |
| format-concat-loop | PD 3/0/0 | 1999→1 | 14,985,010→10,000 | stable |
| fxhashmap | PD 4/0/0 | 0→0 | 0→0 | stable |
| log-write-direct | PD 3/0/0 | 10,015→15 | 382,136→262,136 | stable |
| loop-fusion | PD 4/0/0 | 0→0 | 0→0 | stable |
| mem-take | PD 4/0/0 | 200,002→100,001 | 6,777,780→3,388,890 | stable |
| mutex-to-atomic | PD 4/0/0 | 1→0 | 64→0 | stable |
| o1-lookup | PD 3/1/0 | 0→0 | 0→0 | stable |
| ok-or-else | PD 4/0/0 | 3,700,000→37,000 | 294,400,000→2,944,000 | stable |
| papaya-lockfree | **PDom** 1/2/0 | 10,000→10,000 | 78,890→78,890 | stable |
| precompute-table | **PDom** 0/3/0 | 0→0 | 0→0 | stable |
| prefix-sum-2d | PD 4/0/0 | 0→0 | 0→0 | stable |
| regex-hoist-lazylock | PD 3/0/0 | 1,300,000→0 | 563,601,000→0 | stable |
| retain-inplace | PD 4/0/0 | 19→1 | 8,388,592→4,000,000 | stable |
| reuse-allocation-clear | PD 3/1/0 | 16→1 | 1,048,560→400,000 | stable |
| simd-chunk | PD 4/0/0 | 0→0 | 0→0 | stable |
| smallvec-stack | PD 3/0/0 | 2→0 | 96→0 | stable |
| sort-by-key | **Mixed** 1/0/3 | 100,002→100,002 | 8,401,642→8,401,642 | stable |
| sort-unstable | PD 4/0/0 | 2→1 | 800,000→400,000 | stable |
| static-str | PD 3/0/0 | 10,000→0 | 50,000→0 | stable |
| swap-remove | PD 4/0/0 | 1→1 | 400,000→400,000 | stable |
| vec-remove-front-to-vecdeque | PD 4/0/0 | 1→1 | 200,000→200,000 | stable |
| vec-with-capacity | PD 4/0/0 | 16→1 | 1,048,560→400,000 | stable |

**G-det: 40/40 stable.** The alloc integers are deterministic exactly as
claimed — zero variance across 3 full re-executions per rule. Any future
non-stability is a harness bug, not noise.

### Issue 014 T1.3 predictions vs measured

| Prediction | Verdict |
|---|---|
| `vec-with-capacity` shows alloc delta | ✅ 16→1 allocs, 1,048,560→400,000 B |
| `format-concat-loop` | ✅ 1999→1 allocs, **1,498×** bytes |
| `box-str-immutable` | ⚠️ **no steady-state delta** (0→0) — the harness reuses pre-built Vecs per call, so the width win is cache-density-only in timing; honest finding, annotation unchanged |
| `mem-take` | ✅ exactly 2× (200,002→100,001; 6,777,780→3,388,890 B) |
| unplanned: `ok-or-else` | ✅ 100× (3.7M→37K allocs) — eager error construction dominates |
| unplanned: `regex-hoist-lazylock` | ✅ 1.3M→0 allocs, 563.6 MB-class→0 B per 1000 calls |
| unplanned: `static-str`/`collect-avoidance`/`avoid-clone` | ✅ deterministic N→0 deltas |

### Timing-verdict comparison vs Bench 017 (T1.4 no-flip check)

- `papaya-lockfree` ParetoDominated → ParetoDominated ✅ (no flip)
- `precompute-table` ParetoDominated → ParetoDominated ✅ (no flip)
- `sort-by-key` Mixed → Mixed ✅ (no flip)
- `copy-from-slice` Mixed → ParetoDominant 4/0/0 — **fragile-boundary flip (favorable)**
- `dedupe-hashset-vs-vec` Mixed → ParetoDominant 3/1/0 — **fragile-boundary flip (favorable)**
- `reuse-allocation-clear` Mixed → ParetoDominant 3/1/0 — **fragile-boundary flip (favorable)**

**No adverse flips.** The three flips are exactly the regime-fragile annotated
rules oscillating across the ±5% Tie band between sessions — the documented
fragility is real, so the `context_notes` annotations are RETAINED (they warn
consumers the win is regime-dependent, which remains true even when one session
measures all-faster). Within this session's two sweeps the same oscillation
appeared (`o1-lookup` 3/0/1 ↔ 3/1/0, `bool-to-bitpack` 4/0/0 ↔ 3/0/1,
`box-str-immutable` 3/0/0 ↔ 2/0/1) — this is precisely the wall-clock noise
the deterministic alloc axis exists to complement.

## T2 — rule candidates: bench, then add-or-reject

| Candidate | Gate | Measured | Verdict |
|---|---|---|---|
| T2.1 `regex-hoist-lazylock` | ParetoDominant expected | **PD 3/0/0; 5,443×–5,675×** (guide prior ~555× was conservative); allocs 13,000→0 per 10 calls; bytes 5,636,010→0 | ✅ **ADD** (caching, 2nd rule) |
| T2.2 `arc-str-payload` | BOTH axes (timing may Tie; alloc must show the delta) | **PD 3/0/0; 5.38×–11.38×** (beat the ~1.3× prior); allocs 1001→2; bytes 1,056,000→25,040 (**42×**) at n=1000 | ✅ **ADD** (allocation, 14th rule) |
| T2.3 `tokio-guard-across-await` | NotBenchable pattern rule (metric mismatch) | n/a — cost is OTHER tasks' wait latency under contention; same class as `park-vs-spin` rejection + `bevy-reactive-loop` precedent; clippy `await_holding_lock` covers std guards only | ✅ **ADD** (concurrency; `context_notes` mandatory — recorded) |
| T2.4 `bounded-channel-backpressure` | NotBenchable pattern rule (metric mismatch + semantics-changing) | n/a — win is bounded queue depth; the buggy path is often FASTER on wall-clock precisely because it buffers unboundedly | ✅ **ADD** (concurrency; `context_notes` mandatory — recorded) |
| T2.5 honest drop | any candidate failing its gate is dropped | none failed | n/a — nothing dropped |

### `regex-hoist-lazylock` detail

| N | Buggy (ns) | Fixed (ns) | Speedup | allocs b→f | bytes b→f |
|---|---:|---:|---:|---|---|
| 10 | 653,208 | 120 | 5,443× | 13,000→0 | 5,636,010→0 |
| 100 | 6,467,542 | 1,144 | 5,653× | 130,000→0 | 56,360,100→0 |
| 1,000 | 64,605,084 | 11,384 | 5,675× | 1,300,000→0 | 563,601,000→0 |

Regex compilation is runtime NFA construction (~65 µs + ~5.6 KB-class of
allocation per call on this pattern) — real work the compiler cannot elide
(unlike `precompute-table`'s POPCNT shape). The largest deterministic win in
the corpus.

### `arc-str-payload` detail

| N | Buggy (ns) | Fixed (ns) | Speedup | allocs b→f | bytes b→f |
|---|---:|---:|---:|---|---|
| 10 | 393 | 73 | 5.38× | 11→2 | 10,560→1,280 |
| 100 | 3,853 | 494 | 7.80× | 101→2 | 105,600→3,440 |
| 1,000 | 46,195 | 4,061 | 11.38× | 1,001→2 | 1,056,000→25,040 |

1 KiB payload, N recipients. The fixed side's residual 2 allocs + ~25 KB are
the output `Vec` scaffolding (identical on both sides); the delta is exactly
the eliminated payload copies. Timing grew with N (memcpy-bound) — the
~1.3×-Tie prior from the guide did not materialize at this payload size.

### `mutex-to-atomic` repair verification

Bench 013 recorded 2.5–2.6×; this sweep measures 2.52×–3.58× (4/4 Faster) —
consistent, no regression from the counting-allocator overhead (amortized, as
designed). Alloc axis: 1→0 allocs (the Mutex heap allocation on first lock in
the buggy path).

## GOAT gate

| Gate | Status | Detail |
|---|---|---|
| G1 (correctness) | ✅ PASS | 40/40 benchable rules ran (ran=true, non-empty results, non-zero timings) — including the repaired `mutex-to-atomic` |
| G2 (latency) | ✅ documented | 1,606 s total (40 rules × 3 repeats; I/O + build bound; ~13.4 s/rule) |
| G3 (no-regression) | ✅ PASS | 219 lib tests under `rust_perf_bench`; default suite re-verified post-change (see commit) |
| G4 (alloc) | ✅ **G-det PASS** | The alloc axis reported for every rule + bit-identical across 3 repeat runs on 40/40 — the axis IS the gate here |
| G5 (Pareto verdict) | ✅ PASS | 37/40 ParetoDominant (must be > half); 2 honestly ParetoDominated + 1 Mixed — annotations retained |

## Corpus totals after Issue 014

| Metric | Before | After |
|---|---|---|
| Total rules | 46 | **50** |
| Benchable | 38 | **40** |
| ParetoDominant (this run) | 32 | **37** |
| ParetoDominated | 2 | 2 (unchanged — `precompute-table`, `papaya-lockfree`) |
| Mixed | 4 | 1 measured (`sort-by-key`; the other 3 measured PD this run — annotations retained, see flip note) |
| NotBenchable | 8 | **10** (8 external-runtime/size-class + 2 metric-mismatch) |
| Annotated (`context_notes`) | 6 | **8** (6 verdict-class + 2 metric-mismatch-class) |
| Categories | 12 | 12 (caching 1→2, allocation 13→14, concurrency 2→4) |

## Decision

All 4 T2 candidates land (2 benchable ParetoDominant + 2 honest NotBenchable
pattern rules). The alloc axis ships as the deterministic second axis:
timing stays the Pareto verdict axis; the alloc integers are reported for
every rule + gate Tie-class candidates (per the `arc-str-payload` policy —
which this run rendered unnecessary, the timing verdict being clearly
dominant). No demotions: no rule newly measured dominated; the 2 known
dominated rules keep their annotations.

The `context_notes` surface now has two documented classes: **verdict**
(ParetoDominated/Mixed in default config) and **metric-mismatch**
(NotBenchable-with-reason) — the drafter propagates both identically.

**Stale-header fix:** the corpus header claimed complexity 14 + io 3; actual
counts were complexity 12 + io 1 (sum check). Corrected in the same commit.
