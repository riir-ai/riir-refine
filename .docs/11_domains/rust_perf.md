# `rust_perf` domain (Plan 001 Phase 2)

> The second `HealDomain` impl. Heals rust performance issues using rules
> distilled from the `rust-optimize` skill checklist.

## Validator hierarchy (strongest last)

1. **L2 pruner** (always-on, modelless): `syn`-based syntax parse check.
2. **`rust_perf_verify`** (opt-in): `cargo test` re-run (compiles + tests pass).
3. **`rust_perf_bench`** (opt-in): generated microbenchmarks at multiple
   input sizes → Pareto verdict (faster on N of M sizes). This is the
   **real perf ground truth** — the promotion gate for the domain
   (Proposal 001 caveat 3). See [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md).

## Corpus

8 seed rules, one per skill-checklist category. 5 are benchable (carry a
`BenchSpec`); 3 are honestly `NotBenchable` (need external runtimes).

| Rule ID | Category | Benchable? | Pattern |
|---|---|---|---|
| `o1-lookup` | complexity | ✅ 570x win | O(n) `Vec` scan → O(1) `HashMap` lookup |
| `static-str` | allocation | ✅ 42x win | `String` return → `&'static str` for constant labels |
| `layout-reorder` | layout | ❌ NotBenchable | Field reorder to eliminate padding (deterministic `size_of`, not a runtime microbench) |
| `papaya-lockfree` | concurrency | ✅ 0.55-0.79x (honest dominated) | `Arc<RwLock<HashMap>>` → `papaya` lock-free map (benefit needs multi-threaded contention) |
| `simd-chunk` | simd | ✅ 3.9x win | Scalar loop → 4-wide chunked loop for LLVM auto-vectorization |
| `precompute-table` | caching | ✅ 0.20-0.40x (honest dominated) | Recompute-per-call → `const` lookup table (compiler optimizes popcount to POPCNT in release) |
| `bevy-reactive-loop` | bevy | ❌ NotBenchable | `WinitSettings::default()` (Continuous, 100% CPU) → `reactive_low_power` (needs Bevy runtime) |
| `wasm-batch-ffi` | wasm | ❌ NotBenchable | Per-call FFI (250ns floor × N×M) → single batch invocation (needs WASM runtime) |

## Validator (cargo test — behind `rust_perf_verify`)

`PerfVerifier`:
1. Create a temp crate under `std::env::temp_dir()/riir_perf_verify/<blake3_hash>/`
2. Write `Cargo.toml` + `src/lib.rs` (the code being verified)
3. Run `cargo test --message-format=json`
4. Parse JSON for `compiler-message` errors + `suite` pass/fail counts
5. `test_passes = exit_ok && failed == 0 && compile_errors == 0`

**Why both exit status AND JSON parsing:** a compilation failure produces NO
`suite` event (the test runner never runs), so parsing pass/fail counts alone
returns `(0, 0)` → `test_passes = true` (vacuously). The exit status +
`compiler-message` error count distinguish "compile failure" from "no tests,
all good".

## Bench validator (Pareto verdict — behind `rust_perf_bench`)

`PerfBenchValidator` (the **promotion gate** for the domain):
1. Generate a temp **bin** crate embedding the rule's `BenchSpec.harness` +
   a timing loop (dynamic calibration → warmup → median-of-samples).
2. Run `cargo run --release` on the temp crate.
3. Parse JSON lines: `{"size":N,"buggy_ns":X,"fixed_ns":Y}`.
4. Compute per-size outcome (Faster/Slower/Tie at ±5% threshold).
5. Aggregate into a `ParetoVerdict` (ParetoDominant if faster on majority).

Rules without a `BenchSpec` are honestly marked `NotBenchable` — they need
external runtimes (Bevy, WASM) a temp crate cannot provide.

See [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) for
the measured GOAT results.

## Caveat (Proposal 001 caveat 3 — closed by Bench 013)

The original caveat called perf "the weakest link" because perf has no
single ground truth. **Bench 013 closes this**: the Pareto validator now
provides the real perf ground truth. 3/5 benchable rules are Pareto-dominant
(clear wins); 2/5 are honestly Pareto-dominated (compiler optimizations,
regime-dependent benefits) — exactly the kind of honest signal the caveat
predicted.

## Cargo-in-cargo test constraint

Tests that spawn `cargo test`/`cargo run` are `#[ignore]`'d ("slow: runs
real cargo subprocess (seconds); also needs `--test-threads=1` under
cargo-in-cargo"). This mirrors the `clippy_verify` test pattern — concurrent
cargo invocations against temp crates race on cargo's build cache lock.

## Source

- [`src/domains/rust_perf/mod.rs`](../../src/domains/rust_perf/mod.rs) — `PerfRule` + `RustPerfDomain`
- [`src/domains/rust_perf/corpus.rs`](../../src/domains/rust_perf/corpus.rs) — `build_perf_seed_corpus()` + 5 `BenchSpec`s
- [`src/domains/rust_perf/verify.rs`](../../src/domains/rust_perf/verify.rs) — `PerfVerifier` (cargo test)
- [`src/domains/rust_perf/bench.rs`](../../src/domains/rust_perf/bench.rs) — `PerfBenchValidator` (Pareto verdict)
- [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs) — scaffold end-to-end demo
- [`examples/perf_bench_demo.rs`](../../examples/perf_bench_demo.rs) — the GOAT gate (Bench 013)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Proposal 001 caveat 3](../../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness (now closed)
- [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the GOAT results
