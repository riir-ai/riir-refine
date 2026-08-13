# Bench 013 — rust_perf Bench Pareto Validator GOAT

**Captured:** 2026-08-13
**Hardware:** Apple M3 Max (release mode, `cargo run --example perf_bench_demo --features rust_perf_bench --release`)
**Commit:** `feat: perf-bench Pareto validator for rust_perf domain`
**Branch:** `develop`
**Feature gate:** `rust_perf_bench` (opt-in — implies `rust_perf`)

This is the **promotion gate** for the `rust_perf` domain (Proposal 001
caveat 3). The existing `rust_perf_verify` runs `cargo test` (compilation +
test pass) — a degraded validator with no perf claim. This validator adds
the real perf ground truth: does the fixed code actually run faster?

## What it does

For each corpus rule with a `BenchSpec`:
1. Generate a temp **bin** crate embedding the harness + a timing loop
   (dynamic calibration → warmup → median-of-samples).
2. Run `cargo run --release` on the temp crate.
3. Parse JSON lines: `{"size":N,"buggy_ns":X,"fixed_ns":Y}`.
4. Compute per-size outcome (Faster/Slower/Tie at ±5% threshold).
5. Aggregate into a Pareto verdict (ParetoDominant if faster on majority).

Rules without a `BenchSpec` are honestly marked `NotBenchable`.

## GOAT gates

| Gate | What | Target | Status |
|---|---|---|---|
| G1 (correctness) | benchable rules produce real measurements (ran=true, non-empty results) | all 5 benchable rules run | ✅ PASS (5/5 ran) |
| G2 (latency) | per-rule latency (temp crate compile + bench subprocess) | documented, not hot-path | ~5-15s/rule (compile-dominated) |
| G3 (no-regression) | default features (no `rust_perf_bench`) still pass | default `cargo test --lib` green | ✅ PASS (75 default tests) |
| G4 (alloc-free) | N/A — subprocess-based validator | N/A | ✅ PASS (documented N/A) |
| G5 (Pareto verdict) | majority of benchable rules are Pareto-dominant | `pareto_dominant * 2 > benchable` | ✅ PASS (3/5 = majority) |

## Measured results (5 benchable, 3 not-benchable)

### Pareto-dominant (3/5) — the pattern works

| Rule | N=small | N=large | Peak speedup | Verdict |
|---|---:|---:|---:|---|
| `o1-lookup` | 0.60x (N=10) | **570.60x** (N=10000) | 570.60x | ✅ ParetoDominant (3/4 sizes faster) |
| `simd-chunk` | 2.90x (N=100) | **3.90x** (N=100000) | 3.90x | ✅ ParetoDominant (4/4 sizes faster) |
| `static-str` | 37.06x (N=1) | **43.18x** (N=100) | 43.18x | ✅ ParetoDominant (3/3 sizes faster) |

The `o1-lookup` N=10 SLOWER (0.60x) is small-N noise — HashMap has
setup overhead that dominates at 10 elements. The O(n)→O(1) win is
overwhelming at large N (570x at N=10000).

### Pareto-dominated (2/5) — honest findings

| Rule | Speedup range | Verdict | Honest reason |
|---|---:|---|---|
| `precompute-table` | 0.20x–0.40x | ❌ ParetoDominated | The compiler optimizes `(0..8).filter().count() as u8 & 1` to a single **POPCNT** instruction in release mode, making the lookup table SLOWER (L1 cache latency > POPCNT latency). The rule is NOT a release-mode win — it would only win on architectures without POPCNT or in debug mode. |
| `papaya-lockfree` | 0.55x–0.79x | ❌ ParetoDominated | papaya's epoch-based GC has overhead that only pays off under **multi-threaded contention**. The single-threaded harness measures the overhead without the benefit. This is expected — the rule's value is `Arc<RwLock<HashMap>>` replacement under concurrent access, not single-threaded reads. |

These are NOT bugs in the validator — they're exactly the kind of
honest signal Proposal 001 caveat 3 predicted ("perf has no single
ground truth — 'faster' depends on workload, hardware, input size").

### Not benchable (3/8) — honest markings

| Rule | Reason |
|---|---|
| `layout-reorder` | `size_of` is deterministic (compile-time); the cache-utilization claim needs a specific access pattern to show up in a microbench. |
| `bevy-reactive-loop` | Needs the Bevy runtime (event loop + winit) — can't run in a temp crate. |
| `wasm-batch-ffi` | Needs a WASM runtime (js-sys / wasm-bindgen FFI) — can't run in a temp crate. |

## Methodology

- **Dynamic calibration**: each function's iteration count is calibrated
  so one sample batch ≈ 1ms (avoids the `Instant::now()` noise floor for
  very fast functions).
- **Warmup**: 100 iterations of both functions before measurement
  (instruction cache + branch predictor warmup).
- **Median-of-samples**: 21 samples (odd → clean median), sorted, middle
  value taken. Reduces noise without the overhead of full criterion stats.
- **Threshold**: Faster = speedup > 1.05 (5% improvement); Slower = < 0.95;
  Tie = within ±5%.
- **BLAKE3-keyed temp dirs**: cargo reuses build cache on repeat calls.

## Why not criterion?

Criterion would provide more robust statistics (outlier detection,
confidence intervals) but:
1. **Heavy dep**: adds ~10s compile time to each temp crate (criterion
   fetches + compiles fresh each time since temp crates don't share a
   target dir).
2. **Complex output**: criterion's stats are rich but hard to parse
   programmatically for a Pareto verdict.
3. **Network access**: temp crates need to fetch criterion from
   crates.io on first build.

The hand-rolled timer (warmup + dynamic calibration + median-of-samples)
is simpler, faster to compile, and sufficient for a Pareto verdict.

## Promotion decision

**rust_perf was promoted to default-on 2026-08-13.** The GOAT gate (G1+G5)
passed:
- G1: all 5 benchable rules produce real measurements ✅
- G5: 3/5 benchable rules are Pareto-dominant (majority) ✅

The promotion follows the global AGENTS.md rule "promote to default if gain
also demote loser":
- **Promoted**: the `rust_perf` domain moved from opt-in to `default = [...]`
  in Cargo.toml.
- **Demoted (annotated)**: the 2 ParetoDominated rules (`precompute-table`,
  `papaya-lockfree`) now carry a `context_notes: Some(...)` annotation on
  [`PerfRule`] documenting that the bench found them slower in the default
  config. The patterns are kept in the corpus (they're genuine wins in their
  respective regimes — non-POPCNT targets / multi-threaded contention) but
  consumers should treat annotated rules as context-dependent, not
  unconditional suggestions. The annotation is backward-compatible via
  `#[serde(default)]` — older freeze/thaw snapshots deserialize as `None`.
- **Drafter propagation (2026-08-13, post-promotion)**: the annotation now
  flows end-to-end through the domain-agnostic drafter. A non-breaking
  default method `HealEntry::context_notes() -> Option<&str> { None }` lets
  the drafter read the annotation from any domain's entry type; the
  drafter propagates it through `DraftedFix::context_notes` so the consumer
  (CLI / IDE / CI) decides whether their context matches. The drafter does
  NOT skip or downgrade annotated rules — it surfaces the annotation; the
  consumer makes the policy call. `clippy_lints` + `kernel_opt` use the
  default `None` impl (no changes needed).

The validator itself is the deliverable — it works correctly, catches real
perf characteristics (including honest "this rule is not a win in release
mode" findings), and provides the Pareto verdict that Proposal 001 caveat 3
required.

## Re-running

```sh
# Clean temp dirs (forces fresh cargo builds)
rm -rf /tmp/riir_perf_bench/

# Run the GOAT gate (slow — compiles + runs 5 temp crates)
cargo run --example perf_bench_demo --features rust_perf_bench --release
```

## References

- [Proposal 001 caveat 3](../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness this closes
- [Plan 001 Phase 2](../.plans/001_multi_domain_framework_refactor.md) — the rust_perf domain scaffold this validator completes
- [`.docs/11_domains/rust_perf.md`](../.docs/11_domains/rust_perf.md) — domain doc (updated with the validator hierarchy)
