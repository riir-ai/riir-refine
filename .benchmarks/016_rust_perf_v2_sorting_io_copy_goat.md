# Bench 016 — rust_perf corpus v2: sorting + IO-write + bulk-copy gaps (Issue 012)

**Date:** 2026-08-13
**Issue:** `012`
**Feature gate:** `rust_perf_bench` (opt-in)
**Machine:** Apple M3 Max (macOS)
**GOAT gate:** G1 (correctness) + G5 (Pareto)

## Scope

Issue 012 identified three category gaps in the `rust_perf` corpus (39 rules,
Bench 015 finalization): sorting (0 rules), IO write path (0 rules — only
read-side `buffered-io` existed), and bulk memory copy (0 rules). This bench
adds 4 new rules to fill those gaps + measures each one's Pareto verdict.

## Methodology

Each new rule's bench spec runs at multiple input sizes (release mode, median
of samples with dynamic calibration + warmup). The Pareto validator classifies
each size as Faster/Slower/Tie at ±5% threshold, then aggregates into an
overall verdict (ParetoDominant / Mixed / ParetoDominated).

Runner: `tests/issue012_bench_check.rs` (temporary integration test, removed
after measurement — results preserved here).

## Results

### `sort-unstable` — `sort()` → `sort_unstable()`

**Verdict: ✅ ParetoDominant** (4/4 Faster, 0 Slower, 0 Tie)

| n | buggy ns | fixed ns | speedup | outcome |
|---|---:|---:|---:|---|
| 100 | 388 | 328 | 1.18× | Faster |
| 1,000 | 5,002 | 4,358 | 1.15× | Faster |
| 10,000 | 62,005 | 57,642 | 1.08× | Faster |
| 100,000 | 969,000 | 773,250 | 1.25× | Faster |

Stable sort allocates a temporary buffer + uses mergesort; unstable sort is
in-place pdqsort. Consistent 1.08-1.25× win across all sizes.

### `sort-by-key` — `sort_by(closure)` → `sort_by_key(closure)`

**Verdict: ❌ Mixed** (0/4 Faster, 0 Slower, 4 Tie) → annotated `context_notes`

| n | buggy ns | fixed ns | speedup | outcome |
|---|---:|---:|---:|---|
| 1,000 | 33,112 | 32,365 | 1.02× | Tie |
| 10,000 | 411,646 | 416,062 | 0.99× | Tie |
| 50,000 | 2,262,750 | 2,253,667 | 1.00× | Tie |
| 100,000 | 4,885,875 | 4,915,083 | 0.99× | Tie |

For integer keys on `(String, u32)` tuples, the compiler optimizes `sort_by`'s
comparator to avoid redundant key reads. `sort_by_key` is a correctness/idiom
win (avoids writing the comparator twice), not a throughput win for trivial
keys. The win materializes only when the key function is non-trivial (hash
computation, nested dereference, or expensive computation that the compiler
cannot CSE across comparator calls).

**`context_notes` annotation applied.** Pattern kept in corpus as documented
context-dependent rule.

### `bufwriter-batched` — per-record `File::write` → `BufWriter`

**Verdict: ✅ ParetoDominant** (3/3 Faster, 0 Slower, 0 Tie)

| n | buggy ns | fixed ns | speedup | outcome |
|---|---:|---:|---:|---|
| 100 | 278,180 | 112,151 | 2.48× | Faster |
| 1,000 | 1,404,834 | 201,958 | 6.96× | Faster |
| 10,000 | 14,998,000 | 695,459 | 21.57× | Faster |

Massive win — syscall amortization scales linearly with record count. At
10,000 records the buggy path spends ~15ms in syscalls; the buffered path
coalesces into ~128 writes (8KB buffer / 64-byte records). This is the
write-side analog of `buffered-io` (BufReader for reads, ~97× at Bench 013).

### `copy-from-slice` — index-loop copy → `copy_from_slice`

**Verdict: ❌ Mixed** (2/4 Faster, 1 Slower, 1 Tie) → annotated `context_notes`

| n | buggy ns | fixed ns | speedup | outcome |
|---|---:|---:|---:|---|
| 64 | 43 | 34 | 1.26× | Faster |
| 256 | 31 | 31 | 1.00× | Tie |
| 1,024 | 58 | 53 | 1.09× | Faster |
| 4,096 | 111 | 117 | 0.95× | Slower |

LLVM auto-vectorizes the manual index loop effectively for `u8` slices. At
medium sizes (256-1024) performance is indistinguishable; at n=4096 the manual
loop is marginally faster (likely memcpy setup overhead amortized differently).
`copy_from_slice` is still the correct idiom (eliminates bounds checks,
guarantees SIMD lowering, less code) but the throughput difference is small
for `u8` in release mode. The win is clearer for larger element types (`u64`,
structs) where the manual loop's per-element bounds check is more expensive
relative to the copy.

**`context_notes` annotation applied.** Pattern kept in corpus as documented
context-dependent rule.

## GOAT gate summary

| Rule | G1 (compiles) | G5 (Pareto) | Annotation |
|---|---|---|---|
| `sort-unstable` | ✅ | ✅ ParetoDominant | None |
| `sort-by-key` | ✅ | ❌ Mixed (4 Tie) | `context_notes: ParetoDominated` |
| `bufwriter-batched` | ✅ | ✅ ParetoDominant | None |
| `copy-from-slice` | ✅ | ❌ Mixed (2F/1S/1T) | `context_notes: Mixed` |

**Overall GOAT:** 2/4 ParetoDominant, 2/4 honestly annotated. The corpus
expansion is valid — 2 unconditional wins + 2 documented context-dependent
patterns. The corpus now has 43 rules (35 benchable, 6 annotated).

## Corpus totals after this bench

| Metric | Before (Bench 015) | After (Bench 016) |
|---|---|---|
| Total rules | 39 | 43 |
| Benchable | 31 | 35 |
| NotBenchable | 8 | 8 |
| ParetoDominant | 29 | 31 |
| ParetoDominated | 2 | 2 |
| Mixed | 2 | 4 |
| Annotated (`context_notes: Some`) | 4 | 6 |
| Categories | 11 | 12 (added `io`) |

## Decision

The 4 new rules ship to the corpus (default-on `rust_perf` domain). The 2
ParetoDominant rules (`sort-unstable`, `bufwriter-batched`) are unconditional
suggestions. The 2 Mixed rules (`sort-by-key`, `copy-from-slice`) carry
`context_notes` annotations recording the bench finding — consumers see the
annotation via the drafter's `DraftedFix::context_notes` field and can decide
whether their context matches the winning regime.

No feature-gate change needed — `rust_perf` was already promoted to default-on
(Bench 013 GOAT G1+G5 PASS). This bench extends the corpus; it does not
re-open the promotion gate.
