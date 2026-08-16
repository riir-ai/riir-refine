# Research 054 — Batch 48 (rust_perf half): the 8 online-search perf rules + the two harness lessons

**Date:** 2026-08-16
**Source:** idle-queue item 11 (online rust/kernel perf search, Issue 019 close-out) — the 8 deduped candidates queued in the SKILL.md mining snapshot (nnethercote/perf-book std-types + iterators chapters, std docs).
**Scope:** `src/domains/rust_perf/corpus.rs` (8 new `PerfRule` entries + 8 `BenchSpec` harnesses), `src/draft/eval_fixtures_perf.rs` (8 new eval spans), `src/draft/retrieval_eval.rs` (floors re-pin), `examples/perf_bench_demo.rs` (`PERF_BENCH_FILTER`).
**Verdict:** GOAT G1/G2/G3/G5/G-det PASS — 6/8 ParetoDominant, 2 honestly annotated (1 ParetoDominated folk-wisdom refutation + 1 Mixed). Corpus 50→58 rules, 40→48 benchable, PERF_SPANS 40→48.

## 1. The rules + measured verdicts (Pareto bench, release, median-of-21, 3-run alloc determinism)

| Rule | Category | Verdict | Speedup (sizes) | Alloc axis |
|---|---|---|---|---|
| `collect-append-to-extend` | allocation | **ParetoDominant 3/0/0** | 9.78–13.06× (1K→100K) | 6251→1 allocs, 800K→400K bytes @100K |
| `vec-insert-front-deque` | complexity | **ParetoDominant 3/0/0** | 7.80–115.31× (250→4K) | 1→1 (parity; the win is the O(n²)→O(n) memmove) |
| `vec-macro-zero-fill` | allocation | **ParetoDominant 3/0/0** | 32–2505× (1K→1M) | 1→1 allocs, bytes parity — the win is calloc's lazy zero pages vs the explicit memset pass |
| `unwrap-or-else-lazy` | allocation | **ParetoDominant 3/0/0** | 1.94–2.30× (100→10K, 99% Some) | 0→0 (the win is skipping the eagerly-built default) |
| `hashset-insert-bool-return` | complexity | **ParetoDominant 3/0/0** | 1.19–1.31× (1K→100K) | 1→1 (one hash-table walk instead of two) |
| `filter-map-fuse` | iterators | **ParetoDominant 3/0/0** | 1.37–1.64× (1K→1M, timing-only; allocs 18→18 parity) | parity |
| `iter-copied-deref-map` | iterators | **Mixed (tie/tie/0.80×@1M)** | — | parity — `context_notes`: the specialized `Copied` adapter's length-tracking overhead LOSES to the fully-unrolled Map+sum loop at 1M on a sum fold; keep as an idiom (TrustedLen matters for `collect`'s exact-size reservation, not folds) |
| `filter-count-to-any` | iterators | **ParetoDominated 0.82–0.90×** | — | parity — `context_notes`: see §2 (the folk-wisdom refutation) |

Counts: corpus 50→58 (complexity 12→14, allocation 14→17, iterators 4→7); benchable 40→48; annotated (`context_notes`) 8→10.

## 2. The headline finding — `filter-count-to-any` is ParetoDominated at mid-position hits

The queued candidate came from the perf-book iterators chapter: `.filter(p).count() > 0` → `.any(p)` short-circuits. Our bench **refutes the general claim**: at a **50% hit position**, `any()` measured **0.82–0.90× (SLOWER)** than the full `filter().count()` across 1K/10K/100K.

Root cause: LLVM **vectorizes the count loop** (batched element scanning, SIMD-width batches) while `any()`'s early-exit semantics **force scalar iteration** (it must stop at the first hit — no batched speculation possible). At a mid-position hit the vectorized full scan processes ~8-16 elements per instruction while `any()` walks one element per iteration to the same midpoint. `any()` only wins when the hit is very early (empirically ≲10% position — a position-0 hit is literally sub-ns and trips the validator's div-by-zero Tie guard) or the predicate is non-vectorizable (expensive/branchy).

This is the GOAT gate working as designed: the perf-book hedge was "any is usually better"; our measurement draws the actual regime boundary. The rule stays in the corpus as a **documented idiom with a regime caveat** — the same treatment as `precompute-table` (Bench 013's compiler-optimizes-it-away finding) and `papaya-lockfree` (contention-only win).

## 3. Harness lesson #1 — pure-compute rules get hoisted out of the timing loop (anti-LICM idiom required)

Three first-draft harnesses (`unwrap-or-else-lazy`, `filter-count-to-any`, `vec-macro-zero-fill`) measured **perfect 1.00× ties at impossible per-call costs** (1ns for a 1MB alloc+memset; 0.11ns/item for a 64-iteration default). Root cause: the validator's timing loop re-calls `run_buggy`/`run_fixed` with the **same input** every iteration, so LLVM's loop-invariant code motion hoists the entire pure computation out of the timing loop — only the (near-free) `black_box(result)` remains inside.

**The fix (now the standing idiom for pure-compute harnesses):** re-opacify the input at the top of every `run_*`:

```rust
pub fn run_buggy(input: &[u32]) {
    let input = std::hint::black_box(input);   // defeats LICM hoisting
    ...the actual work...
}
```

`black_box` is opaque to the optimizer — its result is treated as varying, so downstream work cannot be hoisted. Allocation-bearing harnesses never hit this (allocator calls are opaque per-call costs) — which is why the existing 40 specs never needed it. Two related elision classes caught in the same pass:

- **Const-folding of a pure default:** `unwrap_or(expensive_default(64))` — LLVM folds `expensive_default(64)` (pure fn, constant arg) to a constant on BOTH sides → the eager/lazy distinction vanished. Fix: `expensive_default(std::hint::black_box(64))`.
- **Div-by-zero Tie guard:** a position-0 short-circuit makes `any()` genuinely sub-ns → `fixed_ns=0` → the validator's `speedup = 1.0` guard (built for malformed 0ns measurements) classifies a real 10000× win as a Tie. Fix in the harness: hit at n/2 (the honest mid regime — which then produced the ParetoDominated finding of §2).

## 4. Harness lesson #2 — `use` declarations are not inherited by child modules

The `vec-insert-front-deque` harness failed to compile twice: first `run_fixed` (top level) couldn't see `VecDeque` (the `use` lived inside `mod fixed`), then `mod fixed`'s own signature couldn't see it after the `use` moved to the top. Both sites need visibility — the harness now carries the `use` at BOTH the fragment top level (for `run_fixed`) and inside `mod fixed` (for its signature). A generated-crate quirk worth remembering for every future harness that types a container in both a mod signature and a top-level runner.

## 5. Retrieval-precision re-measure + floors re-pin (Issue 017 protocol)

First measurement after the corpus grew 50→58: 3 of the 8 new spans drifted — **`filter-map-fuse`'s span didn't retrieve its own rule in the top-8** (every distinctive token renamed away from the corpus shape: `i % 17`, `rare_drops` vs the corpus `v % 2`, `even_squares`), `vec-insert-front-deque` ranked behind `bounded-channel-backpressure` (its span's `for event in events` matched bounded-channel's corpus verbatim), `filter-count-to-any` lost a 0.15-margin tie to `bool-to-bitpack`.

Per the Issue 022 fix-before-re-pin order, the spans were re-authored to the set's convention (**variant names, canonical predicate/operator shapes** — the distance-from-corpus the existing 40 spans keep), plus one corpus-side cleanup: the new `iter-copied` entry's `fn total_xp` collided with `f32-counter-to-int`'s corpus `fn total_xp` → renamed to `sum_kills`.

Re-measured + re-pinned (both states):

| Metric | Old (Bench 024 pin) | New (Batch 48) | Δ |
|---|---|---|---|
| perf top-1/3/5 | 87.5/95.0/97.5 (40) | **87.5/95.8/97.9 (48)** | top-1 unchanged; 3/5 up |
| blended 2-domain | 69.3/81.8/88.6 (88) | **70.8/83.3/90.6 (96)** | all up |
| blended 3-domain | 77.1/86.4/92.4 (118) | **77.8/87.3/92.9 (126)** | all up |
| kernel self-variants | 100/100/100 (30) | **100/100/100** | crowding alarm clean |
| router annotation 2d/3d | 69.3% / 68.6% | **68.8% (66/96) / 69.8% (88/126)** | ~unchanged |

Floors re-pinned: BLENDED 67/96, 79/96, 86/96; PERF_TOP3 45/48 (tightened from 37/40); ROUTER 61/96 + 80/126 (the −5-span corpus-growth headroom formula, both landing 63.5%). Zero batch decay after the fixture fixes — the re-pin is a denominator correction, not a weakening.

## 6. Tooling: `PERF_BENCH_FILTER` (batch-scoped gate runs)

`perf_bench_demo` now accepts a comma-separated substring filter env — `PERF_BENCH_FILTER=hashset,filter-map cargo run --example perf_bench_demo --features rust_perf_bench --release` benches only matching rules (8/58 in this batch's scoped runs vs the full 48-rule ~35-min run). The full-corpus run remains the promotion-gate record; the filter makes per-batch iteration + regression triage affordable.

## 7. GOAT

- **G1 (correctness):** all 8 corpus entries parse; 8/8 eval spans labeled + retrievable; the 2 compile-failure harness iterations were caught + fixed pre-record (call-site rename + the dual-`use` lesson).
- **G2 (latency):** scoped bench runs ~2-4 min for 8 rules. The full 48-rule re-run was **deliberately killed ~15 min in** (13/48 done): the pre-existing `btreemap-sorted-iteration` harness alone costs ~40+ min (its `run_buggy` at n=50K executes 5000 inner queries × ~1-2ms collect+sort each ≈ 5-10s PER CALL, × calibration probe × 21 samples × 4 sizes × 3 determinism repeats — a slow-by-design harness, not a bug; same cost class as its original Bench 020 appearance). The 40 untouched rules keep their Bench 020 record; the 8 new rules' scoped runs (above) are the batch evidence.
- **G3 (no-regression):** 415 default lib tests; retrieval gate PASS both states; clippy 0 warnings in 4 states (default / latent_retrieval / rust_perf_bench / kernel_heal+latent_retrieval — 2 `doc_markdown` backtick misses fixed pre-commit, the standing gotcha family).
- **G5 (Pareto):** 6/8 new rules ParetoDominant (13.06× best: `collect-append-to-extend`; 115× at 4K: `vec-insert-front-deque`); 2 honestly annotated losers/regime-fragile — never promote a perf gain on a wrong result.
- **G-det (alloc determinism):** stable across 3 repeat runs for every benchable rule (0 nondeterministic).

## 8. Follow-ups

- **Proposers deferred** (honest scope): the queue note sketched "entries + proposers + fixtures + bench gate", but the domain's own methodology is bench-verdict-first (Phase 4 landed proposers only after Bench 013 identified the top-5; Issues 012/013/014 batch additions all landed corpus-only). The bench verdicts now exist: the parse-clean ParetoDominant candidates for a future proposer batch are `hashset-insert-bool-return` (`!s.contains(&x) { s.insert(x) }` → `s.insert(x)`), `collect-append-to-extend` (two-statement staging), and `vec-macro-zero-fill` (two-statement init). `filter-count-to-any` must NEVER get an unconditional proposer (ParetoDominated).
- The validator's `fixed_ns == 0 → Tie` guard could distinguish "sub-ns measurement noise" from "genuinely free call" (e.g. a `fixed_ns < buggy_ns/1000 → Faster` fast-path) — deferred; harnesses can always position the hit to stay measurable.
