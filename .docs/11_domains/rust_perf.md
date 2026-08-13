# `rust_perf` domain (Plan 001 Phase 2)

> The second `HealDomain` impl. Heals rust performance issues using rules
> distilled from the `rust-optimize` skill checklist. Validator runs
> `cargo test` (perf-bench Pareto deferred per Proposal 001 caveat 3).

## Corpus

8 seed rules, one per skill-checklist category:

| Rule ID | Category | Pattern |
|---|---|---|
| `o1-lookup` | complexity | O(n) `Vec` scan → O(1) `HashMap` lookup |
| `static-str` | allocation | `String` return → `&'static str` for constant labels |
| `layout-reorder` | layout | Field reorder to eliminate padding (u8 between u64s → u64/u32/u8) |
| `papaya-lockfree` | concurrency | `Arc<RwLock<HashMap>>` → `papaya` lock-free map (per AGENTS.md rule) |
| `simd-chunk` | simd | Scalar loop → 4-wide chunked loop for LLVM auto-vectorization |
| `precompute-table` | caching | Recompute-per-call → `const` lookup table |
| `bevy-reactive-loop` | bevy | `WinitSettings::default()` (Continuous, 100% CPU) → `reactive_low_power` |
| `wasm-batch-ffi` | wasm | Per-call FFI (250ns floor × N×M) → single batch invocation |

## Validator

`PerfVerifier` (behind `rust_perf_verify`):
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

## Caveat (Proposal 001 caveat 3 — the weakest link)

The validator is the **weakest link** across the three domains. clippy has
binary ground truth (lint satisfied: yes/no). kernel has numeric ground truth
(output matches reference within tolerance). **perf has no single ground
truth** — "faster" depends on workload, hardware, input size. The first cut
ships a **degraded validator** (`cargo test` pass + syntax), NOT a perf-bench
Pareto verdict.

The real perf-bench validator (runs `cargo bench` across multiple input sizes,
reports a Pareto verdict: faster on N of M sizes) lands as a separate plan.

## Cargo-in-cargo test constraint

Tests that spawn `cargo test` are `#[ignore]`'d ("slow: runs real cargo test
(seconds); also needs `--test-threads=1` under cargo-in-cargo"). This mirrors
the `clippy_verify` test pattern — concurrent cargo invocations against temp
crates race on cargo's build cache lock.

## Source

- [`src/domains/rust_perf/mod.rs`](../../src/domains/rust_perf/mod.rs) — `PerfRule` + `RustPerfDomain`
- [`src/domains/rust_perf/corpus.rs`](../../src/domains/rust_perf/corpus.rs) — `build_perf_seed_corpus()`
- [`src/domains/rust_perf/verify.rs`](../../src/domains/rust_perf/verify.rs) — `PerfVerifier`
- [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs) — end-to-end demo

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Proposal 001 caveat 3](../../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness
