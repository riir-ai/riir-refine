# Issue 012 — rust_perf corpus v2: fill sorting + IO-write + bulk-copy gaps

Filed 2026-08-13. Optimization task (per global rule: optimization → issue, not plan).

## Problem

The `rust_perf` corpus (39 rules, Plan 002 Phase 4 finalization complete) has
three clear category gaps that a real-world perf audit would flag immediately:

1. **Sorting: 0 rules.** `Vec::sort()` vs `Vec::sort_unstable()` + `sort_by` vs
   `sort_by_key` are among the most common Rust perf wins. The corpus has
   complexity rules for lookups, dedup, prefix-sum, and binary search — but
   sorting is completely missing.

2. **IO write path: 0 rules.** The corpus has `buffered-io` (BufReader for
   reads) but no write-side equivalent. `File::write` per-record → `BufWriter`
   batched writes is the mirror image and equally common.

3. **Bulk memory copy: 0 rules.** Manual index-loop element copy vs
   `slice::copy_from_slice` — the latter is SIMD-eligible and eliminates bounds
   checking on the fixed-length path. Overlaps with `simd-chunk` conceptually
   but is a distinct, more specific pattern.

## Proposal

Add 4 new rules to the `rust_perf` corpus:

| rule_id | category | buggy → fixed | expected verdict |
|---|---|---|---|
| `sort-unstable` | complexity | `sort()` → `sort_unstable()` | ParetoDominant (no alloc + faster algo) |
| `sort-by-key` | complexity | `sort_by(\|a,b\| a.x.cmp(&b.x))` → `sort_by_key(\|x\| x.x)` | ParetoDominant (N key extractions vs N log N) |
| `bufwriter-batched` | io | per-record `File::write` → `BufWriter` | ParetoDominant (syscall reduction) |
| `copy-from-slice` | simd | index-loop copy → `copy_from_slice` | ParetoDominant (SIMD + no bounds check) |

New corpus total: 43 rules (39 + 4). 35 benchable (31 + 4).

## Tasks

- [x] Add 4 `PerfRule` entries to `build_perf_seed_corpus()`
- [x] Add 4 `BenchSpec` functions behind `rust_perf_bench`
- [x] Update count assertions (39→43, 31→35 benchable, 4→6 annotated)
- [x] Update category counts in the module doc comment
- [x] Run the GOAT bench (Bench 016) — 2 ParetoDominant + 2 Mixed (annotated)
- [x] Commit + push

## Resolution

Bench 016 measured 4 new rules:
- `sort-unstable` ✅ ParetoDominant (1.08-1.25×)
- `sort-by-key` ❌ Mixed (4 Tie) → `context_notes` annotated
- `bufwriter-batched` ✅ ParetoDominant (2.48-21.57×)
- `copy-from-slice` ❌ Mixed (2F/1S/1T) → `context_notes` annotated

Corpus: 39→43 rules, 31→35 benchable, 4→6 annotated, 11→12 categories (added `io`).
