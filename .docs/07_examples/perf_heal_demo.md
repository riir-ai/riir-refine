# perf_heal_demo — rust_perf domain demo (Plan 001 Phase 2)

**Source:** [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs)
**Feature:** `rust_perf` (or `rust_perf_verify` for cargo test verification)
**Run:** `cargo run --example perf_heal_demo --features rust_perf`

## What it demonstrates

The `rust_perf` domain end-to-end (Plan 001 Phase 2):

1. **Build corpus**: `build_perf_seed_corpus()` → 34 `PerfRule` entries across
   11 categories (complexity is deepest with 8 rules;
   allocation has 8; simd has 4).
2. **List rules**: prints all 34 rules grouped by category.
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

## The 22 seed rules

| Category | Rule | Pattern |
|---|---|---|
| complexity | `o1-lookup` | O(n) `Vec` scan → O(1) `HashMap` lookup |
| complexity | `bsearch-sorted-slice` | O(n) `.contains()` on sorted → O(log n) `binary_search()` |
| complexity | `dedupe-hashset-vs-vec` | O(n²) `Vec::contains` dedup loop → O(n) `HashSet` collect |
| complexity | `entry-vs-contains-insert` | `contains_key()` + `insert()` → `entry().or_insert()` (merge HashMap lookups) |
| complexity | `loop-fusion` | Two iterator passes over same data → one fused `for` loop (merge loops) |
| complexity | `prefix-sum-2d` | O(n) 2D region-sum scan → O(1) prefix sum (boundary-vs-volume) |
| complexity | `swap-remove` | `Vec::remove(i)` O(n) shift → `Vec::swap_remove(i)` O(1) swap |
| allocation | `static-str` | `String` → `&'static str` for constant labels |
| allocation | `vec-with-capacity` | `Vec::new()` + push → `Vec::with_capacity(n)` |
| allocation | `format-concat-loop` | `format!("{s}{x}")` in loop → `String::push_str` |
| allocation | `ok-or-else` | `ok_or(expensive())` → `ok_or_else(\|\| expensive())` (lazy error eval) |
| layout | `layout-reorder` | Field reorder to eliminate padding |
| layout | `repr-u8-enum` | Field-less enum → `#[repr(u8)]` for 1-byte size |
| concurrency | `mutex-to-atomic` | `Mutex<u32>` counter → `AtomicU32::fetch_add` |
| concurrency | `papaya-lockfree` | `Arc<RwLock<HashMap>>` → `papaya` lock-free map |
| simd | `simd-chunk` | Scalar loop → 4-wide chunked loop |
| simd | `bool-to-bitpack` | `Vec<bool>` count → packed `Vec<u64>` + `count_ones()` |
| simd | `branchless-conditional` | Data-dependent `if` in hot loop → `val * cond as u64` branchless multiply |
| simd | `chunks-exact` | `chunks` (variable last) → `chunks_exact` (uniform, vectorizes) |
| arithmetic | `f32-counter-to-int` | f32 accumulator for integer tally → u64 accumulator |
| iterators | `collect-avoidance` | Unnecessary `.collect()` intermediate → chained iterator |
| dispatch | `dyn-to-enum` | `Box<dyn Trait>` dispatch → enum dispatch (devirtualize) |
| caching | `precompute-table` | Recompute-per-call → `const` lookup table |
| bevy | `bevy-reactive-loop` | `WinitSettings::default()` → `reactive_low_power` |
| wasm | `wasm-batch-ffi` | Per-call FFI → batch invocation |

## Output shape

```text
=== rust_perf domain — Phase 2 scaffold demo ===

Seed corpus: 34 entries
Commitment (BLAKE3): <updated on rebuild — see `build_perf_seed_corpus()`>

Rules by category:
  [allocation] format-concat-loop — format!("{s}{x}") concat in loop → String::push_str
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
  build_corpus() returned 20 entries (matches seed: true)

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
