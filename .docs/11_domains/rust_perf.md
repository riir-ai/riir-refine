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

20 seed rules across 11 skill-checklist categories. Complexity is the
deepest category (5 rules — the 3 algorithmic anti-patterns + HashMap
lookup merging + branchless-vectorization); allocation + simd have 3 each;
layout + concurrency have 2; the rest have 1 each. 16 are benchable (carry a
`BenchSpec`); 4 are honestly `NotBenchable` (need external runtimes or are
structural). 3 of the 16 benchable rules carry `context_notes` (Bench 013
found 2 `ParetoDominated` + 1 `Mixed` in the default config — consumers
should treat them as context-dependent, not unconditional suggestions).

| Rule ID | Category | Bench verdict | Context? | Pattern |
|---|---|---|---|---|
| `o1-lookup` | complexity | ✅ 570x ParetoDominant | — | O(n) `Vec` scan → O(1) `HashMap` lookup |
| `bsearch-sorted-slice` | complexity | ✅ 16-2709x ParetoDominant | — | O(n) `.contains()` on sorted slice → O(log n) `binary_search()` |
| `dedupe-hashset-vs-vec` | complexity | ~ Mixed (2/4 Faster) | ⚠️ `context_notes` | O(n²) `Vec::contains` dedup loop → O(n) `HashSet` collect (wins at N ≥ 1000; HashSet overhead dominates at small N) |
| `entry-vs-contains-insert` | complexity | ✅ 1.5x ParetoDominant | — | `contains_key()` + `insert()` → `entry().or_insert()` (merge HashMap lookups: 2 hashes → 1 hash) |
| `static-str` | allocation | ✅ 42x ParetoDominant | — | `String` return → `&'static str` for constant labels |
| `vec-with-capacity` | allocation | ✅ 1.1-4.3x ParetoDominant | — | `Vec::new()` + push loop → `Vec::with_capacity(n)` (realloc cost) |
| `format-concat-loop` | allocation | ✅ 42-95x ParetoDominant | — | `format!("{s}{x}")` concat in loop → `String::push_str` (O(n²) → O(n)) |
| `layout-reorder` | layout | ❌ NotBenchable | — | Field reorder to eliminate padding (deterministic `size_of`, not a runtime microbench) |
| `repr-u8-enum` | layout | ❌ NotBenchable | — | Field-less enum → `#[repr(u8)]` (deterministic `size_of`, same class as layout-reorder) |
| `mutex-to-atomic` | concurrency | ✅ 2.5x ParetoDominant | — | `Mutex<u32>` counter inc → `AtomicU32::fetch_add` (Mutex lock/unlock is opaque; ~2.5× single-threaded) |
| `papaya-lockfree` | concurrency | ❌ 0.55-0.79x ParetoDominated | ⚠️ `context_notes` | `Arc<RwLock<HashMap>>` → `papaya` lock-free map (benefit needs multi-threaded contention) |
| `simd-chunk` | simd | ✅ 3.9x ParetoDominant | — | Scalar loop → 4-wide chunked loop for LLVM auto-vectorization |
| `bool-to-bitpack` | simd | ✅ 7-11x ParetoDominant | — | `Vec<bool>` set-bit count → packed `Vec<u64>` + `count_ones()` (data density + hardware POPCNT) |
| `branchless-conditional` | simd | ✅ 10x ParetoDominant | — | Data-dependent `if` in hot loop → `val * cond as u64` branchless multiply (LLVM vectorizes the multiply but cannot vectorize the branchy loop) |
| `f32-counter-to-int` | arithmetic | ✅ 7.5-11x ParetoDominant | — | f32 accumulator for integer tally → u64 accumulator (f32 add dep chain can't be broken without fast-math) |
| `collect-avoidance` | iterators | ✅ 2.3-3.0x ParetoDominant | — | Unnecessary `.collect()` intermediate → chained iterator (allocation + double pass → single pass, zero alloc) |
| `dyn-to-enum` | dispatch | ✅ 2.5-2.9x ParetoDominant | — | `Box<dyn Trait>` dispatch → enum dispatch (devirtualize: vtable indirection → direct match, compiler can inline) |
| `precompute-table` | caching | ❌ 0.20-0.40x ParetoDominated | ⚠️ `context_notes` | Recompute-per-call → `const` lookup table (compiler optimizes popcount to POPCNT in release) |
| `bevy-reactive-loop` | bevy | ❌ NotBenchable | — | `WinitSettings::default()` (Continuous, 100% CPU) → `reactive_low_power` (needs Bevy runtime) |
| `wasm-batch-ffi` | wasm | ❌ NotBenchable | — | Per-call FFI (250ns floor × N×M) → single batch invocation (needs WASM runtime) |

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
provides the real perf ground truth. 13/16 benchable rules are Pareto-dominant
(clear wins); 2/16 are honestly Pareto-dominated (compiler optimizations,
regime-dependent benefits); 1/16 is Mixed (scale-dependent) — exactly the
kind of honest signal the caveat predicted.

## Drafter + pipeline integration (`context_notes` propagation, 2026-08-13)

The 3 annotated rules (2 `ParetoDominated` + 1 `Mixed`) carry a
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

## Source

- [`src/domains/rust_perf/mod.rs`](../../src/domains/rust_perf/mod.rs) — `PerfRule` + `RustPerfDomain`
- [`src/domains/rust_perf/corpus.rs`](../../src/domains/rust_perf/corpus.rs) — `build_perf_seed_corpus()` + 13 `BenchSpec`s
- [`src/domains/rust_perf/verify.rs`](../../src/domains/rust_perf/verify.rs) — `PerfVerifier` (cargo test)
- [`src/domains/rust_perf/bench.rs`](../../src/domains/rust_perf/bench.rs) — `PerfBenchValidator` (Pareto verdict)
- [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs) — scaffold end-to-end demo
- [`examples/perf_bench_demo.rs`](../../examples/perf_bench_demo.rs) — the GOAT gate (Bench 013)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Proposal 001 caveat 3](../../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness (now closed)
- [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the GOAT results
