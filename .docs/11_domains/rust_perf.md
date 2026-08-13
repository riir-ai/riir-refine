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

39 seed rules across 11 categories (rust-optimize skill checklist +
nnethercote/perf-book + rustc-hash/bevy real-world patterns). Allocation is the
deepest category (12 rules). Complexity has 10. Layout, simd, iterators have 4
each; concurrency has 2; the rest have 1 each. 31 are benchable (carry a
`BenchSpec`); 8 are honestly `NotBenchable` (need external runtimes or are
structural — size_of, memory, fairness). 4 of the 31 benchable rules carry
`context_notes` (Bench 015 finalization: 2 `ParetoDominated` —
`precompute-table` + `papaya-lockfree` — are honestly slower in the default
config; 2 regime-fragile — `dedupe-hashset-vs-vec` + `reuse-allocation-clear`
— pass ParetoDominant but have 1 slower size each, flagging them as
scale-dependent. Consumers should treat annotated rules as context-dependent,
not unconditional suggestions).

| Rule ID | Category | Bench verdict | Context? | Pattern |
|---|---|---|---|---|
| `o1-lookup` | complexity | ✅ 570x ParetoDominant | — | O(n) `Vec` scan → O(1) `HashMap` lookup |
| `bsearch-sorted-slice` | complexity | ✅ 16-2709x ParetoDominant | — | O(n) `.contains()` on sorted slice → O(log n) `binary_search()` |
| `dedupe-hashset-vs-vec` | complexity | ~ ParetoDominant (3/4 Faster; 1 slower @ n=500) | ⚠️ `context_notes` | O(n²) `Vec::contains` dedup loop → O(n) `HashSet` collect (wins at N ≥ 1000; HashSet overhead dominates at small N; regime-fragile — see Bench 015) |
| `entry-vs-contains-insert` | complexity | ✅ 1.5x ParetoDominant | — | `contains_key()` + `insert()` → `entry().or_insert()` (merge HashMap lookups: 2 hashes → 1 hash) |
| `loop-fusion` | complexity | ✅ 1.5x ParetoDominant | — | Two iterator passes (`.sum()` + `.max()`) over same data → one fused `for` loop (cache locality; compiler can't fuse separate iterator consumers) |
| `prefix-sum-2d` | complexity | ✅ 34-30000x ParetoDominant | — | O(n) 2D region-sum scan per query → O(1) prefix sum (boundary-vs-volume: prefix encodes boundary cumulative sum, each query reads 4 corners instead of scanning interior) |
| `swap-remove` | complexity | ✅ 2-18485x ParetoDominant | — | `Vec::remove(i)` O(n) shift → `Vec::swap_remove(i)` O(1) swap (order doesn't matter; win grows with N) |
| `fxhashmap` | complexity | ✅ 1.6-4.2x ParetoDominant | — | std `HashMap` (SipHash, DoS-resistant) → `FxHashMap` (FxHash, 1 mul + xor) for non-adversarial keys (compiler can't swap hashers; used pervasively in rustc) |
| `static-str` | allocation | ✅ 42x ParetoDominant | — | `String` return → `&'static str` for constant labels |
| `vec-with-capacity` | allocation | ✅ 1.1-4.3x ParetoDominant | — | `Vec::new()` + push loop → `Vec::with_capacity(n)` (realloc cost) |
| `format-concat-loop` | allocation | ✅ 42-95x ParetoDominant | — | `format!("{s}{x}")` concat in loop → `String::push_str` (O(n²) → O(n)) |
| `ok-or-else` | allocation | ✅ 97x ParetoDominant | — | `ok_or(expensive())` eager error construction → `ok_or_else(\|\| expensive())` lazy closure (avoids unused error allocation; source: nnethercote/perf-book) |
| `mem-take` | allocation | ✅ 1.9-2.6x ParetoDominant | — | `buf.clone()` + `buf.clear()` → `mem::take(&mut buf)` (moves Vec allocation O(1), no deep clone) |
| `avoid-clone` | allocation | ✅ 1.3-3.0x ParetoDominant | — | `fn(data: Vec<T>)` clone-on-call → `fn(data: &[T])` borrow (clone does heap alloc + memcpy; compiler can't elide) |
| `box-str-immutable` | allocation | ✅ 1.1-1.3x ParetoDominant | — | `Vec<String>` (3-word handle) → `Vec<Box<str>>` (2-word, drops capacity field) for immutable string collections |
| `reuse-allocation-clear` | allocation | ~ ParetoDominant (3/4 Faster; 1 slower @ n=100K) | ⚠️ `context_notes` | `fn() { Vec::new() }` alloc-per-call → `fn(buf: &mut Vec<T>)` clear-reuse (wins ≤10K; loses at 100K due to L2 cache pressure; regime-fragile — see Bench 015) |
| `box-avoid` | allocation | ✅ 9.6-42x ParetoDominant | — | `Vec<Box<i64>>` (heap per elem) → `Vec<i64>` (stack values) — heap alloc per element dwarfs the data work |
| `smallvec-stack` | allocation | ✅ 27-75x ParetoDominant | — | `Vec<T>::new()` (always heap) → stack array for ≤8 typical (SmallVec pattern); heap alloc dominates at small N |
| `buffered-io` | iterators | ✅ 99-132x ParetoDominant | — | `File::read(&mut [u8; 1])` syscall per byte → `BufReader` 8KB batched reads (syscall overhead dominates) |
| `log-write-direct` | allocation | ✅ 1.9-2.0x ParetoDominant | — | `buf.push_str(&format!(...))` (temp String per iter) → `write!(buf, ...)` direct (zero temp alloc; distinct from `format-concat-loop` which is about quadratic accumulation) |
| `layout-reorder` | layout | ❌ NotBenchable | — | Field reorder to eliminate padding (deterministic `size_of`, not a runtime microbench) |
| `repr-u8-enum` | layout | ❌ NotBenchable | — | Field-less enum → `#[repr(u8)]` (deterministic `size_of`, same class as layout-reorder) |
| `mutex-to-atomic` | concurrency | ✅ 2.5x ParetoDominant | — | `Mutex<u32>` counter inc → `AtomicU32::fetch_add` (Mutex lock/unlock is opaque; ~2.5× single-threaded) |
| `papaya-lockfree` | concurrency | ❌ 0.55-0.79x ParetoDominated | ⚠️ `context_notes` | `Arc<RwLock<HashMap>>` → `papaya` lock-free map (benefit needs multi-threaded contention) |
| `simd-chunk` | simd | ✅ 3.9x ParetoDominant | — | Scalar loop → 4-wide chunked loop for LLVM auto-vectorization |
| `bool-to-bitpack` | simd | ✅ 7-11x ParetoDominant | — | `Vec<bool>` set-bit count → packed `Vec<u64>` + `count_ones()` (data density + hardware POPCNT) |
| `branchless-conditional` | simd | ✅ 10x ParetoDominant | — | Data-dependent `if` in hot loop → `val * cond as u64` branchless multiply (LLVM vectorizes the multiply but cannot vectorize the branchy loop) |
| `chunks-exact` | simd | ✅ 13-16x ParetoDominant | — | `slice::chunks` (variable last chunk) → `slice::chunks_exact` (uniform chunks, enables vectorization; source: nnethercote/perf-book) |
| `f32-counter-to-int` | arithmetic | ✅ 7.5-11x ParetoDominant | — | f32 accumulator for integer tally → u64 accumulator (f32 add dep chain can't be broken without fast-math) |
| `collect-avoidance` | iterators | ✅ 2.3-3.0x ParetoDominant | — | Unnecessary `.collect()` intermediate → chained iterator (allocation + double pass → single pass, zero alloc) |
| `retain-inplace` | iterators | ✅ 1.5-2.1x ParetoDominant | — | `iter().filter().collect()` new Vec → `Vec::retain()` in-place filter (no allocation, single pass; ~1.5-2× across all sizes) |
| `dyn-to-enum` | dispatch | ✅ 2.5-2.9x ParetoDominant | — | `Box<dyn Trait>` dispatch → enum dispatch (devirtualize: vtable indirection → direct match, compiler can inline) |
| `precompute-table` | caching | ❌ 0.20-0.40x ParetoDominated | ⚠️ `context_notes` | Recompute-per-call → `const` lookup table (compiler optimizes popcount to POPCNT in release) |
| `bevy-reactive-loop` | bevy | ❌ NotBenchable | — | `WinitSettings::default()` (Continuous, 100% CPU) → `reactive_low_power` (needs Bevy runtime) |
| `wasm-batch-ffi` | wasm | ❌ NotBenchable | — | Per-call FFI (250ns floor × N×M) → single batch invocation (needs WASM runtime) |
| `shrink-to-fit-after-build` | allocation | ❌ NotBenchable | — | Vec with over-allocated capacity → `shrink_to_fit()` after build (memory win, not speed; the realloc may even make build slower) |
| `box-large-enum-variant` | layout | ❌ NotBenchable | — | Enum with one `[u8; 1024]` variant → `Box<[u8; 1024]>` (shrinks common case from 1032→16 bytes; size_of win improves cache density) |
| `async-yield-point` | iterators | ❌ NotBenchable | — | Long async fn (no `.await`) → cooperative `yield_now()` every 100 iters (fairness/p99-latency win under contention, not single-task throughput) |
| `async-future-size` | layout | ❌ NotBenchable | — | Large locals across `.await` → scoped blocks (drops before next await; shrinks future state machine via `size_of_val`) |

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
the original promotion-gate results, and [Bench 015](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md)
for the Phase 4 finalization on the expanded 39-rule corpus.

## Caveat (Proposal 001 caveat 3 — closed by Bench 013, re-confirmed by Bench 015)

The original caveat called perf "the weakest link" because perf has no
single ground truth. **Bench 013 closes this**: the Pareto validator now
provides the real perf ground truth. **Bench 015 re-confirms on the expanded
corpus**: 29/31 benchable rules are Pareto-dominant (clear wins); 2/31 are
honestly Pareto-dominated (compiler optimizations, regime-dependent benefits);
2 of the 29 ParetoDominant are regime-fragile (1 slower size each, kept
annotated) — exactly the kind of honest signal the caveat predicted.

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

## Source

- [`src/domains/rust_perf/mod.rs`](../../src/domains/rust_perf/mod.rs) — `PerfRule` + `RustPerfDomain`
- [`src/domains/rust_perf/corpus.rs`](../../src/domains/rust_perf/corpus.rs) — `build_perf_seed_corpus()` + 31 `BenchSpec`s (8 rules are honestly NotBenchable)
- [`src/domains/rust_perf/verify.rs`](../../src/domains/rust_perf/verify.rs) — `PerfVerifier` (cargo test)
- [`src/domains/rust_perf/bench.rs`](../../src/domains/rust_perf/bench.rs) — `PerfBenchValidator` (Pareto verdict)
- [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs) — scaffold end-to-end demo
- [`examples/perf_bench_demo.rs`](../../examples/perf_bench_demo.rs) — the GOAT gate (Bench 013 promotion + Bench 015 Phase 4 finalization)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Proposal 001 caveat 3](../../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness (now closed)
- [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the original promotion-gate GOAT results (historical; pre-expansion corpus snapshots)
- [Bench 015](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md) — Phase 4 finalization on the expanded 39-rule corpus (29/31 ParetoDominant)
