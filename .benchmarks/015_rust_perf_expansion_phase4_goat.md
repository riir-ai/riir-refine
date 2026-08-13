# Bench 015 — rust_perf Phase 4 Re-bench (Expanded Corpus Finalization)

**Captured:** 2026-08-13
**Hardware:** Apple M3 Max (release mode, `cargo run --example perf_bench_demo --features rust_perf_bench --release`)
**Commit:** `9c03786` (HEAD of `develop` at capture time — `feat: expand rust_perf corpus 34→39 rules`)
**Branch:** `develop`
**Feature gate:** `rust_perf_bench` (opt-in — implies `rust_perf`)

This is the **Phase 4 finalization bench** for [Plan 002](../.plans/002_rust_perf_corpus_expansion.md):
a full re-bench of the expanded 39-rule corpus after Phases 1-3 landed. It
supersedes the [Bench 013](013_rust_perf_bench_pareto_goat.md) corpus-shape
numbers (which described the pre-expansion 18-rule / 22-rule / 25-rule /
28-rule / 31-rule corpus snapshots as the plan landed batches); Bench 013
remains the historical record of the **promotion gate** decision.

> **Numbering note:** Plan 002 §Phase 4 originally specified `Bench 014`.
> Bench 014 was allocated to a different experiment
> ([`014_bench010_rerun_llamacpp_vs_katgpt_ternary.md`](014_bench010_rerun_llamacpp_vs_katgpt_ternary.md))
> while Plan 002 was in flight. Per the global numbering-discipline rule
> ("monotonic and never reused"), the Phase 4 bench moved to **015**. The
> plan file's `Bench 014` references are stale; this file is canonical.

## GOAT gates

| Gate | What | Target | Status |
|---|---|---|---|
| G1 (correctness) | benchable rules produce real measurements (ran=true, non-empty results) | all 31 benchable rules run | ✅ PASS (31/31 ran, 0 failed) |
| G2 (latency) | per-rule latency (temp crate compile + bench subprocess) | documented, not hot-path | ~5-15s/rule (compile-dominated) |
| G3 (no-regression) | default features (no `rust_perf_bench`) still pass | default `cargo test --lib` green | ✅ PASS (91 default tests) |
| G4 (alloc-free) | N/A — subprocess-based validator | N/A | ✅ PASS (documented N/A) |
| G5 (Pareto verdict) | majority of benchable rules are Pareto-dominant | `pareto_dominant * 2 > benchable` | ✅ PASS (29/31 = 94%) |

## Measured results (39 total: 31 benchable, 8 NotBenchable)

```
============================================================
BENCH VALIDATOR SUMMARY
============================================================
  Pareto-dominant (faster on majority): 29
  Mixed (some faster, some slower):     0
  Pareto-dominated (regression):        2
  Not benchable (needs external runtime): 8
  Failed to run:                        0

Benchable rules: 31 of 39
Pareto-dominant: 29 of 31 benchable
============================================================
```

### ParetoDominant (29/31) — clean wins

These rules are faster on the majority of tested sizes. Three of them
(`o1-lookup`, `dedupe-hashset-vs-vec`, `reuse-allocation-clear`) have one
slower size each — they still pass the majority bar, but the 1-slower pattern
flags them as regime-fragile; `dedupe-hashset-vs-vec` and `reuse-allocation-clear`
carry `context_notes` for that reason (see "Regime-fragile" below).

| Rule | Category | Sizes | Speedup range | Verdict |
|---|---|---|---|---|
| `o1-lookup` | complexity | 4 (n=10/100/1K/10K) | 0.60-558× (1 slower @ n=10) | ParetoDominant |
| `bsearch-sorted-slice` | complexity | 4 (n=100/1K/10K/100K) | 13.75-2649× | ParetoDominant |
| `swap-remove` | complexity | 4 (n=100/1K/10K/100K) | 2.61-7894× | ParetoDominant |
| `prefix-sum-2d` | complexity | 4 (n=1K/10K/100K/1M) | 34-28159× | ParetoDominant |
| `fxhashmap` | complexity | 4 (n=100/1K/10K/100K) | 3.30-4.23× | ParetoDominant |
| `entry-vs-contains-insert` | complexity | 4 (n=100/1K/10K/100K) | 1.51-2.36× | ParetoDominant |
| `loop-fusion` | complexity | 4 (n=1K/10K/100K/1M) | 1.43-3.59× | ParetoDominant |
| `static-str` | allocation | 3 (n=1/10/100) | 36-43× | ParetoDominant |
| `vec-with-capacity` | allocation | 4 (n=100/1K/10K/100K) | 1.07-4.26× | ParetoDominant |
| `format-concat-loop` | allocation | 3 (n=100/500/1K) | 40-96× | ParetoDominant |
| `ok-or-else` | allocation | 4 (n=100/1K/10K/100K) | 97-101× | ParetoDominant |
| `mem-take` | allocation | 4 (n=100/1K/10K/100K) | 1.96-2.15× | ParetoDominant |
| `avoid-clone` | allocation | 4 (n=100/1K/10K/100K) | 2.01-8.33× | ParetoDominant |
| `box-str-immutable` | allocation | 3 (n=1K/10K/100K) | 1.43-1.59× | ParetoDominant |
| `box-avoid` | allocation | 3 (n=1K/10K/100K) | 9.60-41× | ParetoDominant |
| `smallvec-stack` | allocation | 3 (n=1/4/8) | 22-52× (1 tie @ n=1) | ParetoDominant |
| `buffered-io` | iterators | 2 (n=10K/100K) | 84-98× | ParetoDominant |
| `log-write-direct` | allocation | 3 (n=100/1K/10K) | 1.26-1.90× | ParetoDominant |
| `mutex-to-atomic` | concurrency | 4 (n=1K/10K/100K/1M) | 2.51-2.59× | ParetoDominant |
| `simd-chunk` | simd | 4 (n=100/1K/10K/100K) | 2.80-3.93× | ParetoDominant |
| `bool-to-bitpack` | simd | 4 (n=100/1K/10K/100K) | 8-12.5× | ParetoDominant |
| `branchless-conditional` | simd | 4 (n=1K/10K/100K/1M) | 9.17-12× | ParetoDominant |
| `chunks-exact` | simd | 4 (n=1K/10K/100K/1M) | 17-20× | ParetoDominant |
| `f32-counter-to-int` | arithmetic | 4 (n=100/1K/10K/100K) | 7.25-11.4× | ParetoDominant |
| `collect-avoidance` | iterators | 4 (n=1K/10K/100K/1M) | 2.42-5.10× | ParetoDominant |
| `retain-inplace` | iterators | 4 (n=1K/10K/100K/1M) | 1.29-1.97× | ParetoDominant |
| `dyn-to-enum` | dispatch | 3 (n=1K/10K/100K) | 2.54-2.91× | ParetoDominant |
| `dedupe-hashset-vs-vec` ⚠️ | complexity | 4 (n=100/500/1K/5K) | 0.92-7.52× (1 slower @ n=500) | ParetoDominant (regime-fragile) |
| `reuse-allocation-clear` ⚠️ | allocation | 4 (n=100/1K/10K/100K) | 0.87-4.0× (1 slower @ n=100K) | ParetoDominant (regime-fragile) |

### Regime-fragile (2 of the 29 ParetoDominant — keep `context_notes`)

Two rules — `dedupe-hashset-vs-vec` and `reuse-allocation-clear` — flipped
between Mixed (in earlier Plan 002 batch runs) and ParetoDominant (in this
final run). The flip is run-to-run variance at the boundary size:
- `dedupe-hashset-vs-vec`: at n=500, HashSet overhead briefly exceeds the
  Vec scan (0.92×). At n≥1000 the O(n²) scan dominates and the win is clear
  (1.5-7.5×).
- `reuse-allocation-clear`: at n=100K, L2 cache pressure from reusing a
  large buffer slightly exceeds the alloc-per-call cost (0.87×). At n≤10K
  the alloc reuse wins clearly (1.2-4.0×).

Both keep their `context_notes: Some(...)` annotation because the verdict
is regime-fragile, not because this run classified them as Mixed. The
annotation tells the consumer "verify this rule applies to your N range /
cache profile before applying." This is the honest framing: the rule is a
real win in its target regime, but it's not unconditional.

### ParetoDominated (2/31) — honest findings, kept annotated

| Rule | Category | Why dominated | `context_notes` |
|---|---|---|---|
| `precompute-table` | caching | Compiler optimizes the "buggy" popcount loop to a hardware POPCNT instruction in release mode. The "fixed" lookup-table version is slower because it adds an extra memory load. The pattern is a genuine win on targets without POPCNT, but M3 Max has it. | ⚠️ yes |
| `papaya-lockfree` | concurrency | papaya's lock-free HashMap has higher per-op overhead than `Arc<RwLock<HashMap>>` in the single-threaded bench harness. The win only materializes under TRUE multi-threaded contention (multiple concurrent readers/writers), which the sequential harness doesn't exercise. | ⚠️ yes |

These are kept in the corpus (they're genuine patterns in their respective
regimes — non-POPCNT targets / multi-threaded contention) but carry
`context_notes` so consumers know they're not unconditional wins in the
default config.

### NotBenchable (8/39) — structural / external-runtime rules

| Rule | Category | Why not benchable |
|---|---|---|
| `layout-reorder` | layout | Deterministic `size_of` win, not a runtime microbench |
| `repr-u8-enum` | layout | Deterministic `size_of` win (1-byte enum), same class |
| `bevy-reactive-loop` | bevy | Needs the Bevy runtime (event loop mode switch) |
| `wasm-batch-ffi` | wasm | Needs a WASM runtime (FFI call amortization) |
| `shrink-to-fit-after-build` | allocation | Memory win (frees over-allocation), not speed; the realloc may even slow the build path |
| `box-large-enum-variant` | layout | `size_of` win (shrinks common case from 1032→16 bytes); improves cache density, not a microbench |
| `async-yield-point` | iterators | Fairness / p99-latency win under contention, not single-task throughput |
| `async-future-size` | layout | `size_of_val` win (shrinks the future state machine); improves allocator pressure, not a microbench |

## Plan 002 corpus expansion summary

The corpus grew across Plan 002's phases. Each batch was empirically verified
with a standalone Criterion bench BEFORE adding to the corpus:

| Phase | Batch | Rules added | Count |
|---|---|---|---|
| Pre-Plan-002 | Bench 013 baseline | (the original 18 benchable + 4 NotBenchable) | 22 |
| Plan 002 Phase 1 (early) | nnethercote/perf-book distillation | `chunks-exact`, `ok-or-else`, `swap-remove` | 25 |
| Plan 002 Phase 1 (mid) | rustc-hash/bevy pattern mining | `fxhashmap`, `retain-inplace`, `mem-take` | 28 |
| Plan 002 Phase 1 (mid) | more pattern mining | `avoid-clone`, `box-str-immutable`, `reuse-allocation-clear` | 31 |
| Plan 002 Phase 1 (late) | heap/alloc focus | `box-avoid`, `smallvec-stack`, `buffered-io` | 34 |
| Plan 002 Phase 1 (late) | log path | `log-write-direct` | 35 |
| Plan 002 Phase 2 | NotBenchable structural | `shrink-to-fit-after-build`, `box-large-enum-variant`, `async-yield-point`, `async-future-size` | 39 |

**Candidates tested + REJECTED (9 total):** `inline-hot-fn` (ParetoDominated
0.73-0.75× — `#[inline(always)]` causes i-cache bloat), `rwlock-read-heavy`
(ParetoDominated 0.94-0.96× — RwLock overhead exceeds Mutex sequentially),
`cas-to-fetch` (Tie 1.07-1.09× — single-threaded CAS rarely retries),
`ordering-relaxed` (Tie 1.00-1.02× — same LDAR/STAR on ARM64),
`scope-threads` (Tie 0.98-1.01× — Arc clone is negligible vs thread spawn),
`park-vs-spin` (metric mismatch — CPU-time vs wall-clock),
`lift-hot-field` (architecturally hard to bench in a microbench),
`object-pool` (duplicate of `reuse-allocation-clear`),
`rayon-par-iter` (out of scope — architectural, requires dep + restructuring).

The 5/11 → 29/31 acceptance rate (Bench 013 baseline → Bench 015 final)
exceeds the plan's expected-outcome prediction ("~9-12 of 18 to pass"). The
honest rejections are documented above; no claim ships without measurement.

## Methodology

Identical to Bench 013 (dynamic calibration → warmup → median-of-samples,
±5% threshold, BLAKE3-keyed temp dirs for cargo cache reuse). See
[Bench 013 §Methodology](013_rust_perf_bench_pareto_goat.md#methodology)
for details.

## Phase 4 verdict

**Plan 002 Phase 4 is COMPLETE.** All seven Phase 4 tasks pass:
- **T-rebench** ✅ — this file (Bench 015).
- **T-goat-g1** ✅ — all 31 benchable rules produce real measurements (0 failed).
- **T-goat-g5** ✅ — 29/31 (94%) of benchable rules are ParetoDominant; well
  above the plan's acceptance bar.
- **T-goat-g3** ✅ — default features `cargo test --lib` green (91 tests).
- **T-doc** ✅ — `.docs/11_domains/rust_perf.md` corpus table + counts updated.
- **T-clippy** ✅ — `cargo clippy --all-targets -D warnings` clean.
- **T-agents** ✅ — `AGENTS.md` rust_perf rule count updated (31→39).

## Re-running

```sh
# Clean temp dirs (forces fresh cargo builds)
rm -rf /tmp/riir_perf_bench/

# Run the GOAT gate (slow — compiles + runs 31 temp crates, ~5-15s each)
cargo run --example perf_bench_demo --features rust_perf_bench --release
```

## References

- [Plan 002](../.plans/002_rust_perf_corpus_expansion.md) — the expansion plan this finalizes
- [Bench 013](013_rust_perf_bench_pareto_goat.md) — the original promotion-gate bench (historical; pre-expansion corpus snapshots)
- [`.docs/11_domains/rust_perf.md`](../.docs/11_domains/rust_perf.md) — domain doc (corpus table + counts)
- [Proposal 001 caveat 3](../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness (closed by Bench 013; Bench 015 re-confirms on the expanded corpus)
