# Bench 017 — rust_perf corpus v3: data-structure choice rules (Issue 013)

> **Date:** 2026-08-14
> **Feature gate:** `rust_perf_bench` (opt-in)
> **Toolchain:** rustc 1.97.0 (2026-07-07)
> **Hardware:** Apple M3 Max (native, single-threaded, `--release`)
> **GOAT:** G1 (bench runs) PASS, G5 (Pareto verdict) PASS — 3/3 ParetoDominant

## Context

The `rust_perf` corpus had a **data-structure-choice gap**: zero rules about
choosing the right collection (Vec vs VecDeque, HashMap vs BTreeMap, String vs
Cow). This is the most impactful perf decision a dev makes, yet the corpus only
operated within single data structures. Issue 013 adds 3 std-only rules filling
this gap.

## Rules added (3)

| Rule | Category | Pattern | Bench sizes |
|---|---|---|---|
| `vec-remove-front-to-vecdeque` | complexity | Vec::remove(0) O(n²) → VecDeque::pop_front O(n) | 100, 1K, 10K, 50K |
| `btreemap-sorted-iteration` | complexity | HashMap + sort/query → BTreeMap natural order | 100, 1K, 10K, 50K |
| `cow-conditional-alloc` | allocation | String always-alloc → Cow<str> borrow (90% no-work) | 100, 1K, 10K, 50K |

## Results

All 3 rules are **ParetoDominant** — 4/4 Faster across all sizes. No
`context_notes` annotations needed; all are unconditional wins.

### `vec-remove-front-to-vecdeque`

Vec::remove(0) does a memmove of all remaining elements left by one position.
VecDeque::pop_front just advances the ring-buffer head index. The gap grows
quadratically with N (O(n²) total drain vs O(n)).

| N | Buggy (ns) | Fixed (ns) | Speedup | Outcome |
|---|---:|---:|---:|---|
| 100 | 738 | 202 | 3.65× | FASTER |
| 1,000 | 30,584 | 1,332 | 22.96× | FASTER |
| 10,000 | 2,338,667 | 11,137 | 209.99× | FASTER |
| 50,000 | 71,905,333 | 56,617 | 1,270.03× | FASTER |

**Verdict: ParetoDominant.** Clear quadratic-vs-linear scaling — the textbook
data-structure-choice win.

### `btreemap-sorted-iteration`

HashMap requires collecting keys + sorting_unstable on each sorted-iteration
query (O(n log n) per query). BTreeMap maintains sorted order by construction
(O(n) per query). Bench inserts N pseudo-random keys, then does N/10 sorted
iterations (the common case where sorted order is needed repeatedly).

| N | Buggy (ns) | Fixed (ns) | Speedup | Outcome |
|---|---:|---:|---:|---|
| 100 | 7,723 | 2,073 | 3.73× | FASTER |
| 1,000 | 494,729 | 107,939 | 4.58× | FASTER |
| 10,000 | 62,463,583 | 11,176,584 | 5.59× | FASTER |
| 50,000 | 1,881,243,292 | 272,284,666 | 6.91× | FASTER |

**Verdict: ParetoDominant.** Speedup grows with N (the sort cost dominates more
at larger sizes). The crossover where BTreeMap insert overhead pays off is below
N=100 — even for small maps, sorted iteration favors BTreeMap when repeated.

### `cow-conditional-alloc`

`String` return type forces `to_string()` allocation even when the input needs
no transformation. `Cow<str>` borrows the input in the no-work case (zero
allocation). Bench processes N strings where 90% have no HTML special chars.

| N | Buggy (ns) | Fixed (ns) | Speedup | Outcome |
|---|---:|---:|---:|---|
| 100 | 6,520 | 4,365 | 1.49× | FASTER |
| 1,000 | 61,458 | 44,149 | 1.39× | FASTER |
| 10,000 | 619,125 | 441,854 | 1.40× | FASTER |
| 50,000 | 3,129,833 | 2,362,333 | 1.32× | FASTER |

**Verdict: ParetoDominant.** Consistent ~1.3-1.5× — the 90% borrow path avoids
N heap allocations (each ~40-60ns on macOS). The win is steady, not growing,
because the per-call cost difference is fixed (one allocation vs zero). A higher
borrow ratio (99% no-work) would amplify the gap further.

## GOAT gate

| Gate | Status | Detail |
|---|---|---|
| G1 (correctness) | ✅ PASS | All 3 rules ran successfully (ran=true, non-empty results) |
| G2 (latency) | ✅ documented | ~14s + ~385s + ~2s = ~401s total (I/O bound: cargo build per rule) |
| G3 (no-regression) | ✅ PASS | 91 default lib tests pass (was 91 before) |
| G4 (alloc) | N/A | Subprocess-based validator (same as all bench specs) |
| G5 (Pareto verdict) | ✅ PASS | 3/3 ParetoDominant — promotion bar met |

## Corpus totals after Issue 013

| Metric | Before | After |
|---|---|---|
| Total rules | 43 | **46** |
| Benchable | 35 | **38** |
| ParetoDominant | 29 | **32** |
| ParetoDominated | 2 | 2 |
| Mixed | 4 | 4 |
| NotBenchable | 8 | 8 |
| Annotated (`context_notes`) | 6 | 6 (unchanged — no new annotations) |
| Categories | 12 | 12 (no new category — 2 complexity + 1 allocation) |

## Methodology

Each rule's `BenchSpec.harness` is embedded into a generated temp bin crate,
compiled with `cargo run --release`, and timed via `std::time::Instant` with
dynamic calibration + warmup + median-of-samples (same methodology as Bench 013
+ Bench 016). The runner is `examples/perf_bench_demo.rs` (or for this focused
run, a temporary 3-rule subset runner — same validator, same crates).

## Decision

All 3 rules are unconditional ParetoDominant wins. No `context_notes`
annotations needed. Corpus v3 ships as-is.

Promotion: `rust_perf` stays default-on (no change — already promoted in Bench
013). The 3 new rules inherit the default-on status.

**No demotions.** All 3 new rules are ParetoDominant; no loser to demote.
