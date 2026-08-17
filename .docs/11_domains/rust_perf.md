# `rust_perf` domain (Plan 001 Phase 2)

> The second `HealDomain` impl. Heals rust performance issues using rules
> distilled from the `rust-optimize` skill checklist.
>
> **Promoted to default-on 2026-08-13** after the perf-bench Pareto
> validator GOAT gate passed (Bench 013 G1+G5). The domain module now
> compiles by default; the validators (`rust_perf_verify`,
> `rust_perf_bench`) remain opt-in (slow subprocess-based checks).

## Validator hierarchy (strongest last)

1. **L2 pruner** (always-on, modelless): `syn`-based syntax parse check.
2. **`rust_perf_verify`** (opt-in): `cargo test` re-run (compiles + tests pass).
3. **`rust_perf_bench`** (opt-in): generated microbenchmarks at multiple
   input sizes → Pareto verdict (faster on N of M sizes). This is the
   **real perf ground truth** — the promotion gate for the domain
   (Proposal 001 caveat 3). See [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md).

## Corpus

62 rules across 12 categories (rust-optimize skill checklist +
nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs
profiling guide, Research 032; Batch 48 added the 8 online-search candidates
from the perf-book std-types + iterators chapters; Batch 53 added the 4
online-search candidates of [Bench 038](../../.benchmarks/038_batch53_rust_perf_online_search_goat.md)).
Allocation is the deepest
category (18 rules). Complexity has 14. Iterators has 7. Simd 5, layout 5;
concurrency has 4 (Issue 014 added the two metric-mismatch rules);
caching has 2 (Issue 014 added `regex-hoist-lazylock`); arithmetic has 3
(Batch 53 added `powi-vs-powf` + `distance-sq-no-sqrt`); the rest have 1
each. 52 are benchable (carry a `BenchSpec`); 10 are honestly
`NotBenchable` — 8 need external runtimes or are structural (size_of,
memory, fairness), 2 are **metric-mismatch** (Issue 014 T2.3/T2.4: the cost
is contention latency / queue depth, not wall-clock). 12 of 62 carry
`context_notes` — 10 verdict-class (3 `ParetoDominated` + 7
regime-fragile/Mixed — Batch 48 added the `filter-count-to-any` folk-wisdom
refutation: the VECTORIZED count loop beats `any()`'s scalar early-exit at
mid-position hits; Batch 53 added the `powi-vs-powf` refutation — LLVM
lowers `powf(2.0)` to the same multiply as `powi(2)` in release, the folk
wisdom only holds for debug profiles / exponents ≥ 3 — and the
`soa-hot-field` latency-bound scoping — a serial f32 add-reduction hides the
layout win; SoA pays on throughput-bound field consumers) + 2
metric-mismatch-class. Consumers should treat
annotated rules as context-dependent, not unconditional suggestions.

| Rule ID | Category | Bench verdict | Context? | Pattern |
|---|---|---|---|---|
| `o1-lookup` | complexity | ✅ 570x ParetoDominant | — | O(n) `Vec` scan → O(1) `HashMap` lookup |
| `bsearch-sorted-slice` | complexity | ✅ 16-2709x ParetoDominant | — | O(n) `.contains()` on sorted slice → O(log n) `binary_search()` |
| `dedupe-hashset-vs-vec` | complexity | ~ ParetoDominant (regime-fragile) | ⚠️ `context_notes` | O(n²) `Vec::contains` dedup loop → O(n) `HashSet` collect (wins at N ≥ 1000; HashSet overhead dominates at small N — see Bench 015/020) |
| `entry-vs-contains-insert` | complexity | ✅ 1.5x ParetoDominant | — | `contains_key()` + `insert()` → `entry().or_insert()` (merge HashMap lookups: 2 hashes → 1 hash) |
| `loop-fusion` | complexity | ✅ 1.5x ParetoDominant | — | Two iterator passes (`.sum()` + `.max()`) over same data → one fused `for` loop (cache locality; compiler can't fuse separate iterator consumers) |
| `prefix-sum-2d` | complexity | ✅ 34-30000x ParetoDominant | — | O(n) 2D region-sum scan per query → O(1) prefix sum (boundary-vs-volume: prefix encodes boundary cumulative sum, each query reads 4 corners instead of scanning interior) |
| `swap-remove` | complexity | ✅ 2-18485x ParetoDominant | — | `Vec::remove(i)` O(n) shift → `Vec::swap_remove(i)` O(1) swap (order doesn't matter; win grows with N) |
| `fxhashmap` | complexity | ✅ 1.6-4.2x ParetoDominant | — | std `HashMap` (SipHash, DoS-resistant) → `FxHashMap` (FxHash, 1 mul + xor) for non-adversarial keys (compiler can't swap hashers; used pervasively in rustc) |
| `sort-unstable` | complexity | ✅ 1.08-1.25x ParetoDominant | — | Stable `sort()` (allocates) → `sort_unstable()` in-place (Issue 012 / Bench 016) |
| `vec-remove-front-to-vecdeque` | complexity | ✅ 3.65-1270x ParetoDominant | — | `Vec::remove(0)` O(n²) drain → `VecDeque::pop_front` O(n) (Issue 013 / Bench 017) |
| `btreemap-sorted-iteration` | complexity | ✅ 3.73-6.91x ParetoDominant | — | `HashMap` + sort per query → `BTreeMap` natural order (Issue 013 / Bench 017) |
| `static-str` | allocation | ✅ 42x ParetoDominant | — | `String` return → `&'static str` for constant labels |
| `vec-with-capacity` | allocation | ✅ 1.1-4.3x ParetoDominant | — | `Vec::new()` + push loop → `Vec::with_capacity(n)` (realloc cost; Bench 020 alloc axis: 16→1 allocs, 1,048,560→400,000 B) |
| `format-concat-loop` | allocation | ✅ 42-95x ParetoDominant | — | `format!("{s}{x}")` concat in loop → `String::push_str` (O(n²) → O(n); Bench 020 alloc axis: 1999→1 allocs, 1498× bytes) |
| `ok-or-else` | allocation | ✅ 97x ParetoDominant | — | `ok_or(expensive())` eager error construction → `ok_or_else(|| expensive())` lazy closure (avoids unused error allocation; Bench 020 alloc axis: 3.7M→37K allocs, 100×) |
| `mem-take` | allocation | ✅ 1.9-2.6x ParetoDominant | — | `buf.clone()` + `buf.clear()` → `mem::take(&mut buf)` (moves Vec allocation O(1), no deep clone; Bench 020 alloc axis: exactly 2× both axes) |
| `avoid-clone` | allocation | ✅ 1.3-3.0x ParetoDominant | — | `fn(data: Vec<T>)` clone-on-call → `fn(data: &[T])` borrow (clone does heap alloc + memcpy; compiler can't elide) |
| `box-str-immutable` | allocation | ✅ 1.1-1.3x ParetoDominant | — | `Vec<String>` (3-word handle) → `Vec<Box<str>>` (2-word, drops capacity field) for immutable string collections (Bench 020: no steady-state alloc delta — the win is width/cache density) |
| `reuse-allocation-clear` | allocation | ~ ParetoDominant (regime-fragile) | ⚠️ `context_notes` | `fn() { Vec::new() }` alloc-per-call → `fn(buf: &mut Vec<T>)` clear-reuse (wins ≤10K; loses at 100K due to L2 cache pressure — see Bench 015/020) |
| `box-avoid` | allocation | ✅ 9.6-42x ParetoDominant | — | `Vec<Box<i64>>` (heap per elem) → `Vec<i64>` (stack values) — heap alloc per element dwarfs the data work |
| `smallvec-stack` | allocation | ✅ 27-75x ParetoDominant | — | `Vec<T>::new()` (always heap) → stack array for ≤8 typical (SmallVec pattern); heap alloc dominates at small N |
| `log-write-direct` | allocation | ✅ 1.9-2.0x ParetoDominant | — | `buf.push_str(&format!(...))` (temp String per iter) → `write!(buf, ...)` direct (zero temp alloc; Bench 020 alloc axis: 10,015→15 allocs) |
| `cow-conditional-alloc` | allocation | ✅ 1.32-1.49x ParetoDominant | — | `String` return (always alloc) → `Cow<str>` borrow when no transform (Issue 013 / Bench 017; Bench 020 alloc axis: 75,000→30,000) |
| `arc-str-payload` | allocation | ✅ 5.38-11.38x ParetoDominant | — | Clone-per-message immutable payload → `Arc<str>` (clone the handle, not the bytes; Issue 014 / Bench 020: allocs 1001→2, bytes 42×) |
| `buffered-io` | iterators | ✅ 99-132x ParetoDominant | — | `File::read(&mut [u8; 1])` syscall per byte → `BufReader` 8KB batched reads (syscall overhead dominates) |
| `layout-reorder` | layout | ❌ NotBenchable | — | Field reorder to eliminate padding (deterministic `size_of`, not a runtime microbench) |
| `repr-u8-enum` | layout | ❌ NotBenchable | — | Field-less enum → `#[repr(u8)]` (deterministic `size_of`, same class as layout-reorder) |
| `mutex-to-atomic` | concurrency | ✅ 2.5x ParetoDominant | — | `Mutex<u32>` counter inc → `AtomicU32::fetch_add` (Mutex lock/unlock is opaque; ~2.5× single-threaded; Bench 020 alloc axis: 1→0 allocs) |
| `papaya-lockfree` | concurrency | ❌ 0.55-0.79x ParetoDominated | ⚠️ `context_notes` | `Arc<RwLock<HashMap>>` → `papaya` lock-free map (benefit needs multi-threaded contention) |
| `tokio-guard-across-await` | concurrency | ❌ NotBenchable (**metric mismatch**) | ⚠️ `context_notes` | Guard held across `.await` → fetch-before-guard (cost is OTHER tasks' wait latency, not wall-clock — Issue 014 T2.3; clippy `await_holding_lock` covers std guards only) |
| `bounded-channel-backpressure` | concurrency | ❌ NotBenchable (**metric mismatch**) | ⚠️ `context_notes` | Unbounded channel + hot producer → bounded `channel(cap)` (win is bounded queue depth; semantics-changing backpressure — Issue 014 T2.4) |
| `simd-chunk` | simd | ✅ 3.9x ParetoDominant | — | Scalar loop → 4-wide chunked loop for LLVM auto-vectorization |
| `bool-to-bitpack` | simd | ✅ 7-11x ParetoDominant | — | `Vec<bool>` set-bit count → packed `Vec<u64>` + `count_ones()` (data density + hardware POPCNT) |
| `branchless-conditional` | simd | ✅ 10x ParetoDominant | — | Data-dependent `if` in hot loop → `val * cond as u64` branchless multiply (LLVM vectorizes the multiply but cannot vectorize the branchy loop) |
| `chunks-exact` | simd | ✅ 13-16x ParetoDominant | — | `slice::chunks` (variable last chunk) → `slice::chunks_exact` (uniform chunks, enables vectorization; source: nnethercote/perf-book) |
| `copy-from-slice` | simd | ~ Mixed (regime-fragile) | ⚠️ `context_notes` | Manual index-loop copy → `slice::copy_from_slice` (LLVM auto-vectorizes the manual loop for u8 — small throughput delta; the idiom still guarantees SIMD + no bounds check — Issue 012 / Bench 016) |
| `f32-counter-to-int` | arithmetic | ✅ 7.5-11x ParetoDominant | — | f32 accumulator for integer tally → u64 accumulator (f32 add dep chain can't be broken without fast-math) |
| `collect-avoidance` | iterators | ✅ 2.3-3.0x ParetoDominant | — | Unnecessary `.collect()` intermediate → chained iterator (allocation + double pass → single pass, zero alloc; Bench 020 alloc axis: 8,000,000→0 B) |
| `retain-inplace` | iterators | ✅ 1.5-2.1x ParetoDominant | — | `iter().filter().collect()` new Vec → `Vec::retain()` in-place filter (no allocation, single pass; ~1.5-2× across all sizes) |
| `sort-by-key` | complexity | ~ Mixed (regime-fragile) | ⚠️ `context_notes` | `sort_by` repeated closure → `sort_by_key` single extraction (Issue 012 / Bench 016) |
| `dyn-to-enum` | dispatch | ✅ 2.5-2.9x ParetoDominant | — | `Box<dyn Trait>` dispatch → enum dispatch (devirtualize: vtable indirection → direct match, compiler can inline) |
| `precompute-table` | caching | ❌ 0.20-0.40x ParetoDominated | ⚠️ `context_notes` | Recompute-per-call → `const` lookup table (compiler optimizes popcount to POPCNT in release) |
| `regex-hoist-lazylock` | caching | ✅ 5443-5675x ParetoDominant | — | `Regex::new` per call → `static LazyLock<Regex>` (compile once; Issue 014 / Bench 020: allocs 1.3M→0, bytes 563.6M→0 per 1000 calls — the largest deterministic win in the corpus) |
| `bevy-reactive-loop` | bevy | ❌ NotBenchable | — | `WinitSettings::default()` (Continuous, 100% CPU) → `reactive_low_power` (needs Bevy runtime) |
| `wasm-batch-ffi` | wasm | ❌ NotBenchable | — | Per-call FFI (250ns floor × N×M) → single batch invocation (needs WASM runtime) |
| `collect-append-to-extend` | allocation | ✅ 9.8-13.1x ParetoDominant | — | `collect()` + `append()` staging → `extend()` (no intermediate Vec; Batch 48 alloc axis: 6251→1 allocs, bytes halved @100K) |
| `hashset-insert-bool-return` | complexity | ✅ 1.19-1.31x ParetoDominant | — | `!set.contains(&x)` + `set.insert(x)` double lookup → `insert`'s bool return (one hash-table walk; Batch 48) |
| `unwrap-or-else-lazy` | allocation | ✅ 1.9-2.3x ParetoDominant | — | `unwrap_or(expensive())` eager default → `unwrap_or_else(closure)` lazy (99%-Some regime; Batch 48 — sibling of `ok-or-else`) |
| `filter-count-to-any` | iterators | ❌ 0.82-0.90x ParetoDominated | ⚠️ `context_notes` | `filter(p).count() > 0` → `any(p)` — **refuted at mid-position hits** (Batch 48): LLVM vectorizes the count loop while `any`'s early-exit forces scalar iteration; wins only for ≲10%-position hits or non-vectorizable predicates |
| `vec-macro-zero-fill` | allocation | ✅ 32-2505x ParetoDominant | — | `Vec::new()` + `resize(n, 0)` explicit memset → `vec![0; n]` `alloc_zeroed`/calloc lazy zero pages (Batch 48) |
| `iter-copied-deref-map` | iterators | ⚠️ Mixed (0.80x @1M) | ⚠️ `context_notes` | `.map(|x| *x)` → `.copied()` (Batch 48): parity ≤100K, SLOWER at 1M on sum folds — the idiom's TrustedLen matters for `collect`, not folds |
| `filter-map-fuse` | iterators | ✅ ParetoDominant | — | `filter(p).map(f)` two adapters → `filter_map` fused (Batch 48: timing-only win, allocs parity — the perf-book's hedge resolved by measurement) |
| `vec-insert-front-deque` | complexity | ✅ 7.8-115.3x ParetoDominant | — | `Vec::insert(0, x)` O(n²) prepend loop → `VecDeque::push_front` O(1) (Batch 48 — the write-side sibling of `vec-remove-front-to-vecdeque`) |
| `distance-sq-no-sqrt` | arithmetic | ✅ 1.21-1.24x ParetoDominant | — | `(dx²+dy²).sqrt() < r` → `dx²+dy² < r²` compare-squared (Batch 53 — monotonic sqrt; keep the sqrt when the VALUE is consumed) |
| `hashmap-with-capacity` | allocation | ✅ 2.4-3.1x ParetoDominant | — | `HashMap::new()` + known-count insert loop → `HashMap::with_capacity(n)` (Batch 53 — rehash ladder re-SipHashes all entries per resize; allocs 12→1) |
| `powi-vs-powf` | arithmetic | ~ Mixed (all-Tie) | ⚠️ `context_notes` | `x.powf(2.0)` → `x.powi(2)` (Batch 53 refutation: LLVM lowers both identically in release; idiom value = debug profiles + exponents ≥ 3) |
| `soa-hot-field` | layout | ~ Mixed (latency-bound) | ⚠️ `context_notes` | AoS wide-struct single-field scan → SoA split (Batch 53: serial f32 add-chains hide the win; pays on throughput-bound field consumers — count/min-max/transform) |
| `shrink-to-fit-after-build` | allocation | ❌ NotBenchable | — | Vec with over-allocated capacity → `shrink_to_fit()` after build (memory win, not speed; the realloc may even make build slower) |
| `box-large-enum-variant` | layout | ❌ NotBenchable | — | Enum with one `[u8; 1024]` variant → `Box<[u8; 1024]>` (shrinks common case from 1032→16 bytes; size_of win improves cache density) |
| `async-yield-point` | iterators | ❌ NotBenchable | — | Long async fn (no `.await`) → cooperative `yield_now()` every 100 iters (fairness/p99-latency win under contention, not single-task throughput) |
| `async-future-size` | layout | ❌ NotBenchable | — | Large locals across `.await` → scoped blocks (drops before next await; shrinks future state machine via `size_of_val`) |
| `bufwriter-batched` | io | ✅ 2.48-21.57x ParetoDominant | — | `File::write` per record → `BufWriter` batched (syscall amortization; Issue 012 / Bench 016) |

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

## Bench validator (Pareto verdict + deterministic alloc axis — behind `rust_perf_bench`)

`PerfBenchValidator` (the **promotion gate** for the domain):
1. Generate a temp **bin** crate embedding the rule's `BenchSpec.harness` +
   a timing loop (dynamic calibration → warmup → median-of-samples) + a
   counting `#[global_allocator]` (Issue 014 T1 — fully-qualified paths, zero
   `use` statements, so injected code can never collide with harness imports).
2. Run `cargo run --release` on the temp crate.
3. Parse JSON lines:
   `{"size":N,"buggy_ns":X,"fixed_ns":Y,"buggy_allocs":A,"buggy_bytes":B,"fixed_allocs":C,"fixed_bytes":D}`.
4. Compute per-size outcome (Faster/Slower/Tie at ±5% threshold).
5. Aggregate into a `ParetoVerdict` (ParetoDominant if faster on majority).
6. **Alloc axis** (Issue 014 T1): after the timing loops, one steady-state
   call of each side with the counters reset — exact integers (warmup has
   settled one-time init like LazyLock compilation).
   `run_bench_with_determinism(n)` re-executes + asserts the axis is
   bit-identical across runs — the determinism proof (variance = harness
   bug, not noise).

Rules without a `BenchSpec` are honestly marked `NotBenchable` — they need
external runtimes (Bevy, WASM) a temp crate cannot provide, or (the Issue 014
class) their cost metric is not wall-clock.

See [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) for
the original promotion-gate results, [Bench 015](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md)
for the Phase 4 finalization, [Bench 016](../../.benchmarks/016_rust_perf_v2_sorting_io_copy_goat.md)
+ [Bench 017](../../.benchmarks/017_rust_perf_v3_data_structure_choice_goat.md)
for the Issue 012/013 expansions, and [Bench 020](../../.benchmarks/020_rust_perf_v4_alloc_axis_goat.md)
for the alloc-axis sweep on the 50-rule corpus (G-det: 40/40 rules stable
across 3 repeat runs).

## Caveat (Proposal 001 caveat 3 — closed by Bench 013, re-confirmed by Benches 015/016/017/020)

The original caveat called perf "the weakest link" because perf has no
single ground truth. **Bench 013 closes this**: the Pareto validator now
provides the real perf ground truth. **Bench 015 re-confirms on the expanded
corpus**: 29/31 benchable rules are Pareto-dominant (clear wins); 2/31 are
honestly Pareto-dominated (compiler optimizations, regime-dependent benefits);
2 of the 29 ParetoDominant are regime-fragile (1 slower size each, kept
annotated) — exactly the kind of honest signal the caveat predicted.
**Bench 020 adds the deterministic complement**: the alloc axis (exact
integers, stable across repeat runs on 40/40 rules) removes the noise floor
from the second half of every perf claim — timing still decides the verdict,
but allocation cost is now a provable integer, not a probabilistic estimate.

## Drafter + pipeline integration (`context_notes` propagation, 2026-08-13)

The 4 annotated rules (2 `ParetoDominated` + 2 regime-fragile) carry a
`context_notes: Some(...)` annotation on
[`PerfRule`](../../src/domains/rust_perf/mod.rs). The annotation is **rule-level
metadata** — it describes the RULE, not a specific fix. The pipeline looks it
up from the corpus at heal time + surfaces it on ALL paths (drafter / memory /
Stage2):

```text
PerfRule.context_notes
      (struct field — the source of truth for rule metadata)
         │
         ▼
HealEntry::context_notes()          (trait method, default None)
         │
         ├──→ DraftedFix.context_notes  (L1 drafter output — for standalone drafter use)
         │
         ▼
HealPipeline::heal()                (corpus lookup at the top of heal())
         │
         ▼
HealResult.context_notes            (pipeline output — same annotation on all paths)
         │
         ▼
consumer                            (CLI/IDE/CI displays ⚠️ when Some)
```

**The drafter + pipeline do NOT skip or downgrade annotated rules.** The rule
is a genuine pattern — just not an unconditional win in the default
configuration. The consumer (CLI caller, IDE plugin, CI gate) reads
`DraftedFix::context_notes` (direct drafter API) or `HealResult::context_notes`
(self-evolve pipeline API) and decides whether their context matches
(multi-threaded? POPCNT target? large N?).

This is the honest design: the pipeline surfaces the annotation; it does not
make policy decisions about whether a rule applies. The policy belongs to the
consumer, who knows their context.

### Pipeline path coverage

| Path | `context_notes` | How |
|---|---|---|
| **Drafter** (`HealSource::Drafter`) | ✅ from corpus lookup | `heal()` looks up the corpus entry by `rule_id` + calls `HealEntry::context_notes()`. |
| **Memory** (`HealSource::Memory`) | ✅ from corpus lookup | Same corpus lookup — `FixTrajectory` doesn't carry the annotation (it's fix-level data), but the corpus is the source of truth for rule-level metadata. |
| **Stage2** (`HealSource::Stage2Adaptive`) | ✅ from corpus lookup | Same corpus lookup — the adaptive drafter serves a cached verified fix; the annotation is still the rule-level metadata from the corpus. |

### Consumer display

Examples that print fix output now surface the annotation:
- `heal_demo.rs` — prints `⚠️ Context-dependent: <notes>` when `DraftedFix::context_notes` is `Some`
- `self_evolve_real_world_demo.rs` — appends `⚠️` marker to the per-lint line when `HealResult::context_notes` is `Some`

Non-breaking: `HealEntry::context_notes()` has a default `None` impl, so
`clippy_lints` + `kernel_opt` entries need no changes — they are unconditional
wins (or honestly `NotBenchable`) and return `None`. The new
`HealResult::context_notes` + `Candidate.context_notes` fields are additive
(existing test assertions only read `.source`/`.evolution_score`/`.fixed_code`/`.parses`).

## Cargo-in-cargo test constraint

Tests that spawn `cargo test`/`cargo run` are `#[ignore]`'d ("slow: runs
real cargo subprocess (seconds); also needs `--test-threads=1` under
cargo-in-cargo"). This mirrors the `clippy_verify` test pattern — concurrent
cargo invocations against temp crates race on cargo's build cache lock.

## Build-config recommendations (not healable)

The code healer edits `.rs` via `syn`. PGO / LTO / codegen-units / BOLT are
`Cargo.toml` + CI-workflow changes — **out of scope for the healer.** This
section documents them as recommendations, not corpus rules. Source:
nnethercote/perf-book `build-configuration.md` + Kobzol (cargo-pgo).

| Config | What it does | Expected gain | Trade-off |
|---|---|---|---|
| `codegen-units = 1` | Lets LLVM see the entire crate for better optimization | 1-5% CPU | Slower compile (LLVM can't parallelize per-crate) |
| `lto = "thin"` | Cross-crate inlining (LTO) — sees through `pub fn` boundaries | 2-10% | Slower compile (link-time analysis across all deps) |
| `panic = "abort"` | Removes unwinding machinery (~6KB binary size + some codegen paths) | 1-3% binary | Can't catch panics; no unwinding for Drop |
| `strip = true` | Strips debug symbols from the release binary | 30-70% binary size | No stack traces in production |
| **PGO** (`cargo-pgo`) | Profile-Guided Optimization: instrument → run representative workload → rebuild with profile | 10-20% CPU | Requires a representative workload run; CI integration complexity |
| **BOLT** (post-link) | Binary Optimization Layout Tool — reorders binary layout post-link for better i-cache | 2-5% on top of PGO | Requires LLVM BOLT; Linux x86-64 only |

**Recommendation:** for CPU-bound production binaries, the ROI hierarchy is:
1. `codegen-units = 1` + `lto = "thin"` (easy, always worth it)
2. PGO if the workload is stable + representable (biggest single gain)
3. BOLT if on Linux x86-64 + already doing PGO (incremental on top)

The healer does NOT suggest these — they're Cargo.toml/CI changes, not code
patterns. A separate "build-config audit" tool (out of scope) could check for
them.

## Proposers (Plan 005 Phase 4 + Phase 6 T6.1 — the `--fix` upgrade)

**DONE 2026-08-14.** [`src/domains/rust_perf/proposers/`](../../src/domains/rust_perf/proposers/)
ships the top-5 Pareto-dominant rules as `syn` AST detectors +
**span-preserving source edits**: each proposer walks the parsed file
immutably and emits `SourceEdit` byte ranges (via `proc-macro2`
`span-locations`); `apply_edits` splices them back-to-front. Everything
outside a matched span stays byte-identical — no whole-file reformat
(Bench 019). The latent matcher (`--suggest`) picks/reports rules; the
proposer's visitor is the application gate — retrieval scores never
auto-apply.

| Rule | Rewrite | Notes |
|---|---|---|
| `vec-with-capacity` | `Vec::new()`/`vec![]` + push loop → `Vec::with_capacity(<cap>)` | capacity only from unambiguous sources (`0..n`, `0..=n`, `&path`, `path.iter()`); bare-path iterables skipped (could be `.len()`-less iterators) |
| `entry-vs-contains-insert` | `if !m.contains_key(&k) { m.insert(k, v); }` → `m.entry(k).or_insert(v);` | statement-level rewrite (preserves `()` type); walks all stmts file-wide |
| `static-str` | `-> String { "lit".to_string() }` → `-> &'static str { "lit" }` | signature-visible (return type); `.to_owned()` + `String::from` forms too |
| `bool-to-bitpack` | `flags: &[bool]` count → `flags: &[u64]` + `count_ones().sum()` | signature-visible (param type); param name preserved (no rename hazards) |
| `fxhashmap` | std `HashMap` → `rustc_hash::FxHashMap` (use swap + group slimming + path rewrite) | healed crate needs the `rustc-hash` dep; glob/`pub use`/fn-local forms skipped |
| `hashset-insert-bool-return` | `if !s.contains(k) { s.insert(k); .. }` → `if s.insert(k) { .. }` | insert stmt deleted via the shared `delete_stmt_edit`; key equivalence modulo leading `&`/`*` layers (2026-08-17) |
| `collect-append-to-extend` | `let mut staged = it.collect(); recv.append(&mut staged);` → `recv.extend(it);` | single-use staging gate (exactly 2 ident occurrences file-wide); first-mention adjacency in one block (2026-08-17) |
| `vec-macro-zero-fill` | `let mut v = Vec::new(); v.resize(n, 0);` → `let mut v = vec![0; n];` | zero int/float literals only; `v` stays alive for later uses; a read-only-after binding may surface a rustc `unused_mut` warning — dropping `mut` unconditionally would risk E0596 on unknown mutators, which is strictly worse (2026-08-17) |

The 2026-08-17 additions are the Batch 48 follow-up: the three parse-clean
ParetoDominant rules whose proposers were deferred pending bench verdicts
(measured ParetoDominant in Bench 020 — now `--fix` eligible). Two syn facts
learned landing them: **a type annotation wraps the pattern** (`let mut v:
Vec<u8> = ..` parses as `Pat::Type` around the ident — unwrap before
matching), and **`LitFloat::base10_digits` keeps `.`/`e`/sign separators**
(`"0.0"` → `"0.0"` — filter to digit chars before the all-zero check).
`filter-count-to-any` stays proposer-less BY DESIGN (ParetoDominated
0.82-0.90x — a losing rewrite never gets a `--fix` path).

The `for_each_fn` shared helper (now in
[`edit_emitter.rs`](../../src/edit_emitter.rs)) scans free fns + impl
methods + nested inline `mod` blocks. `--fix` CLI wiring shipped with
Phase 6 T6.1 (Bench 019); the clippy proposers joined the same emitter in
T6.4; `delete_stmt_edit` (the line-scaffolding-aware statement deletion)
joined in the 2026-08-17 batch. G5 real-repo validation precedent: Bench 019
(katgpt-pruners, 20 edits landed) + Bench 031 (`--fix --verify`, 14/12/2).

## Source

- [`src/domains/rust_perf/mod.rs`](../../src/domains/rust_perf/mod.rs) — `PerfRule` + `RustPerfDomain`
- [`src/domains/rust_perf/corpus.rs`](../../src/domains/rust_perf/corpus.rs) — `build_perf_seed_corpus()` + 31 `BenchSpec`s (8 rules are honestly NotBenchable)
- [`src/domains/rust_perf/proposers/`](../../src/domains/rust_perf/proposers/) — the 8 `syn` AST proposers (Phase 4 top-5 + the 2026-08-17 Batch 48 follow-up) + `propose_fix` dispatch
- [`src/domains/rust_perf/verify.rs`](../../src/domains/rust_perf/verify.rs) — `PerfVerifier` (cargo test)
- [`src/domains/rust_perf/bench.rs`](../../src/domains/rust_perf/bench.rs) — `PerfBenchValidator` (Pareto verdict)
- [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs) — scaffold end-to-end demo
- [`examples/perf_bench_demo.rs`](../../examples/perf_bench_demo.rs) — the GOAT gate (Bench 013 promotion + Bench 015 Phase 4 finalization)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Proposal 001 caveat 3](../../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness (now closed)
- [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the original promotion-gate GOAT results (historical; pre-expansion corpus snapshots)
- [Bench 015](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md) — Phase 4 finalization on the expanded 39-rule corpus (29/31 ParetoDominant)
