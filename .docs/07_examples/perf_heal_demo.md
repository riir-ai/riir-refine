# perf_heal_demo — rust_perf domain demo (Plan 001 Phase 2)

**Source:** [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs)
**Feature:** `rust_perf` (or `rust_perf_verify` for cargo test verification)
**Run:** `cargo run --example perf_heal_demo --features rust_perf`

## What it demonstrates

The `rust_perf` domain end-to-end (Plan 001 Phase 2):

1. **Build corpus**: `build_perf_seed_corpus()` → 16 `PerfRule` entries across
   9 skill-checklist categories (complexity + allocation tie for deepest
   with 3 rules each).
2. **List rules**: prints all 16 rules grouped by category.
3. **Validate**: calls `RustPerfDomain::validate()` on each fixed pattern.
   - Without `rust_perf_verify`: L2 pruner syntax check (`PerfParseVerdict`)
   - With `rust_perf_verify`: `cargo test` re-run (`PerfVerifyResult` with
     pass/fail counts)
4. **Tamper-evidence**: demonstrates `Corpus::verify()` on pristine vs
   tampered corpus.
5. **Trait exercise**: `RustPerfDomain::build_corpus()` via the `HealDomain`
   trait (matches the seed corpus).
6. **Optional cargo test** (with `rust_perf_verify`): runs `PerfVerifier` on
   the first rule, prints `ran`/`test_passes`/`passed`/`failed`.

## The 16 seed rules

| Category | Rule | Pattern |
|---|---|---|
| complexity | `o1-lookup` | O(n) `Vec` scan → O(1) `HashMap` lookup |
| complexity | `bsearch-sorted-slice` | O(n) `.contains()` on sorted → O(log n) `binary_search()` |
| complexity | `dedupe-hashset-vs-vec` | O(n²) `Vec::contains` dedup loop → O(n) `HashSet` collect |
| allocation | `static-str` | `String` → `&'static str` for constant labels |
| allocation | `vec-with-capacity` | `Vec::new()` + push → `Vec::with_capacity(n)` |
| allocation | `format-concat-loop` | `format!("{s}{x}")` in loop → `String::push_str` |
| layout | `layout-reorder` | Field reorder to eliminate padding |
| layout | `repr-u8-enum` | Field-less enum → `#[repr(u8)]` for 1-byte size |
| concurrency | `mutex-to-atomic` | `Mutex<u32>` counter → `AtomicU32::fetch_add` |
| concurrency | `papaya-lockfree` | `Arc<RwLock<HashMap>>` → `papaya` lock-free map |
| simd | `simd-chunk` | Scalar loop → 4-wide chunked loop |
| simd | `bool-to-bitpack` | `Vec<bool>` count → packed `Vec<u64>` + `count_ones()` |
| arithmetic | `f32-counter-to-int` | f32 accumulator for integer tally → u64 accumulator |
| caching | `precompute-table` | Recompute-per-call → `const` lookup table |
| bevy | `bevy-reactive-loop` | `WinitSettings::default()` → `reactive_low_power` |
| wasm | `wasm-batch-ffi` | Per-call FFI → batch invocation |

## Output shape

```text
=== rust_perf domain — Phase 2 scaffold demo ===

Seed corpus: 16 entries
Commitment (BLAKE3): 1887fcc7d1421ba6e81aac1cacd988e604a03ed808dce0a2c6962d083a36a453

Rules by category:
  [allocation] static-str — String → &'static str for constant labels
  [bevy] bevy-reactive-loop — Bevy Continuous → reactive_low_power event loop
  ...

Validating fixed patterns via domain validate():
  [OK] bevy-reactive-loop
  [OK] layout-reorder
  ...

✓ All fixed patterns validated

Tamper-evidence check:
  verify() on pristine corpus: true
  verify() on tampered corpus: false (expected: false)

HealDomain trait impl:
  build_corpus() returned 16 entries (matches seed: true)

=== Phase 2 scaffold demo complete ===
```

## Caveat

This demo validates that fixed patterns **parse as valid Rust** (or **pass
`cargo test`** with `rust_perf_verify`). It does NOT verify the perf claim
(faster on production input). The perf-bench Pareto validator
(`rust_perf_bench` feature, `perf_bench_demo` example) provides the real
perf ground truth.

## See also

- [`../11_domains/rust_perf.md`](../11_domains/rust_perf.md) — the rust_perf domain design
- [`../11_domains/heal_domain_trait.md`](../11_domains/heal_domain_trait.md) — the `HealDomain` trait
