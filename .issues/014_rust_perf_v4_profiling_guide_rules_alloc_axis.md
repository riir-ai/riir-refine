# Issue 014 — rust_perf corpus v4: profiling-guide rule candidates + deterministic alloc-count bench axis

**Source:** [Research 032](../.research/032_hotpath_profiling_guide_distillation.md) (hotpath-rs profiling guide distillation, 2026-08-14)
**Domain:** `rust_perf` (corpus + bench validator)
**Precedent:** Issue 012 (Bench 016) / Issue 013 (Bench 017) — gap-filling batches with empirical Pareto gate BEFORE corpus addition. Bench 013 rejection discipline applies: candidates that don't verify are honestly dropped, not added.

## Context

The hotpath-rs guide's two actionable transfers: (1) **deterministic signals (alloc count/bytes) over noisy wall-clock** — our Pareto validator measures timing only and documents its own noise (±5% Tie band, regime-fragile annotations in Benches 015/016); (2) **4 rule candidates** with no shipped corpus equivalent (grep-verified). Two are benchable, two are NotBenchable pattern rules (metric mismatch — contention latency / queue depth, same class as `park-vs-spin` rejection + `bevy-reactive-loop` NotBenchable precedent).

## T1 — Deterministic alloc-count axis in the bench validator

- [ ] T1.1 Inject a thread-local counting `GlobalAllocator` into the generated bench `main.rs` (`prepare_crate`/`generate_main_rs`); count allocations + bytes per buggy/fixed run. Precedent: riir-mmorpg-examples G4 `CountingAllocator`, dhat-rs heap-usage testing.
- [ ] T1.2 Report `alloc_count`/`alloc_bytes` as a deterministic second axis: exact integers, assert run-to-run equality across samples (variance = harness bug). Timing stays the Pareto verdict axis; alloc is reported + recorded.
- [ ] T1.3 Re-run the 38 benchable rules with the axis; record per-rule alloc deltas in the bench doc. Expected: `vec-with-capacity`, `format-concat-loop`, `box-str-immutable`, `mem-take` show deterministic alloc deltas; timing verdicts unchanged.
- [ ] T1.4 GOAT: G1 axis values identical across 3 repeat runs (determinism proof); no timing-verdict flips vs Bench 017.

## T2 — Rule candidates: bench, then add-or-reject

- [ ] T2.1 `regex-hoist-lazylock` (caching): `Regex::new` per call → `static LazyLock<Regex>`. Harness dep via `// [dep: regex = "1"]` marker. Expected ParetoDominant (guide prior: ~555× per call; compilation is runtime work the compiler cannot elide — unlike `precompute-table`'s POPCNT shape).
- [ ] T2.2 `arc-str-payload` (allocation): clone-per-message immutable payload → `Arc<str>`. Gate on BOTH axes (timing prior ~1.3× may Tie; alloc prior ~190,000× bytes is deterministic). Distinct from `avoid-clone` + `box-str-immutable`.
- [ ] T2.3 `tokio-guard-across-await` (concurrency, NotBenchable): guard held across slow await → fetch-before-guard. `context_notes`: metric is other-task wait latency (not wall-clock); clippy `await_holding_lock` covers std guards only — this targets tokio/parking_lot/async-lock.
- [ ] T2.4 `bounded-channel-backpressure` (concurrency, NotBenchable): unbounded channel + send loop → bounded. `context_notes` mandatory: semantics-changing (backpressure vs buffering), bounds memory.
- [ ] T2.5 Honest drop path: any candidate that fails its gate is recorded in the bench doc as rejected (Bench 013 §rejected-candidates table format), NOT added.

## T3 — Bench doc + bookkeeping

- [ ] T3.1 Write `.benchmarks/020_rust_perf_v4_alloc_axis_goat.md` — methodology, per-rule results (timing + alloc axes), add/reject table, corpus totals before/after.
- [ ] T3.2 Update README feature-matrix corpus counts + AGENTS.md rust_perf bullet if rules land.
- [ ] T3.3 Update `.docs/11_domains/rust_perf.md` (+ `03_drafter`/`08_benchmarks` if the axis changes validator behavior notes).

## Acceptance

- Alloc axis deterministic (identical integers across runs) on all benchable rules.
- Each T2 candidate has an explicit add-or-reject verdict backed by our own bench numbers, not the guide's.
- `cargo check` + `cargo test --lib` + `cargo clippy --all-targets` clean; `rust_perf_bench` feature gates the axis (default build unchanged).
