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
| G1 (correctness) | benchable rules produce real measurements (ran=true, non-empty results) | all 17 benchable rules run | ✅ PASS (17/17 ran) |
| G2 (latency) | per-rule latency (temp crate compile + bench subprocess) | documented, not hot-path | ~5-15s/rule (compile-dominated) |
| G3 (no-regression) | default features (no `rust_perf_bench`) still pass | default `cargo test --lib` green | ✅ PASS (91 default tests) |
| G4 (alloc-free) | N/A — subprocess-based validator | N/A | ✅ PASS (documented N/A) |
| G5 (Pareto verdict) | majority of benchable rules are Pareto-dominant | `pareto_dominant * 2 > benchable` | ✅ PASS (14/17 = majority) |

## Measured results (17 benchable, 4 not-benchable)

### Pareto-dominant (14/17) — the pattern works

| Rule | N=small | N=large | Peak speedup | Verdict |
|---|---:|---:|---:|---|
| `o1-lookup` | 0.60x (N=10) | **570.60x** (N=10000) | 570.60x | ✅ ParetoDominant (3/4 sizes faster) |
| `bsearch-sorted-slice` | 16.00x (N=100) | **2709.00x** (N=100000) | 2709.00x | ✅ ParetoDominant (4/4 sizes faster) |
| `simd-chunk` | 2.90x (N=100) | **3.90x** (N=100000) | 3.90x | ✅ ParetoDominant (4/4 sizes faster) |
| `bool-to-bitpack` | 1.00x (N=100) | **11.03x** (N=100000) | 11.03x | ✅ ParetoDominant (3/4 faster, 1 tie) |
| `branchless-conditional` | 10.49x (N=1000) | **10.29x** (N=1000000) | 10.60x | ✅ ParetoDominant (4/4 sizes faster) |
| `static-str` | 37.06x (N=1) | **43.18x** (N=100) | 43.18x | ✅ ParetoDominant (3/3 sizes faster) |
| `vec-with-capacity` | 4.43x (N=100) | **1.07x** (N=100000) | 4.43x | ✅ ParetoDominant (4/4 sizes faster) |
| `format-concat-loop` | 45.25x (N=100) | **94.58x** (N=1000) | 94.58x | ✅ ParetoDominant (3/3 sizes faster) |
| `mutex-to-atomic` | 2.57x (N=1000) | **2.52x** (N=1000000) | 2.57x | ✅ ParetoDominant (4/4 sizes faster) |
| `f32-counter-to-int` | 7.50x (N=100) | **11.43x** (N=1000) | 11.43x | ✅ ParetoDominant (4/4 sizes faster) |
| `entry-vs-contains-insert` | 1.48x (N=100) | **1.57x** (N=10000) | 1.57x | ✅ ParetoDominant (4/4 sizes faster) |
| `loop-fusion` | 1.46x (N=1000) | **1.56x** (N=1000000) | 1.56x | ✅ ParetoDominant (4/4 sizes faster) |
| `collect-avoidance` | 2.27x (N=1000) | **2.95x** (N=100000) | 2.95x | ✅ ParetoDominant (4/4 sizes faster) |
| `dyn-to-enum` | 2.88x (N=1000) | **2.50x** (N=100000) | 2.88x | ✅ ParetoDominant (3/3 sizes faster) |

The `o1-lookup` N=10 SLOWER (0.60x) is small-N noise — HashMap has
setup overhead that dominates at 10 elements. The O(n)→O(1) win is
overwhelming at large N (570x at N=10000).

`bsearch-sorted-slice` is the standout: O(log n) vs O(n) produces a
**2709×** speedup at N=100000 — the log scale gap grows without bound.

`mutex-to-atomic` is a consistent ~2.5× across all sizes — the Mutex
lock/unlock path involves an atomic CAS to acquire + an atomic store to
release (plus the `MutexGuard` RAII lifecycle), while `AtomicU32::fetch_add`
is a single `lock xadd` instruction. The compiler cannot eliminate
`Mutex::lock` (it's an opaque call). This is the first concurrency rule
that's ParetoDominant single-threaded (`papaya-lockfree` needs contention).

`f32-counter-to-int` wins because f32 addition is NOT associative (IEEE
754 rounding), so the compiler cannot break the sequential dependency chain
or use multiple SIMD accumulators without `-C fast-math` (which Rust does not
enable by default). Each f32 add has 3–4 cycle latency, fully serial. The u64
accumulator vectorizes freely (integer addition IS associative) + 1-cycle add
latency.

`collect-avoidance` wins because `.collect()` must produce a concrete `Vec` —
the compiler cannot eliminate it (it's a heap allocation + full copy of all
elements). The chained iterator makes a single pass with zero allocation.
The ~2.3–3.0× speedup is consistent across sizes (allocation + copy overhead
is proportional to N; the iterator fusion only saves the second pass, but the
allocation itself is the bottleneck).

`dyn-to-enum` wins because `Box<dyn Trait>` forces a vtable load + indirect
call on every `.apply()` — the compiler cannot devirtualize through an opaque
trait object or inline the callee. The enum's `match` arms are direct calls
the compiler can inline + the branch predictor learns the pattern. The ~2.5×
speedup is consistent — it's the fundamental cost of one level of indirection
per call (3 ops in the inner loop → 3× vtable misses). This rule was NOT in
the `rust-optimize` skill checklist — a genuine gap the corpus expansion
found.

`branchless-conditional` wins ~10× because the data-dependent `if cond[i]`
inside the hot loop prevents LLVM auto-vectorization — each iteration may or
may not execute the body, so the vectorizer cannot pack multiple iterations
into SIMD lanes. The fixed path uses `val * cond[i] as u64` — a branchless
multiply that LLVM vectorizes freely. **The win is primarily vectorization,
not branch prediction**: even with 100% predictable conditions (verified via
a separate standalone bench at 99%/1%/100% true), the branchless version is
still ~10× faster because the multiply loop vectorizes but the branchy loop
cannot. This rule covers checklist item 7's "branch-free inner loops" sub-item
that was previously only partially covered by `bool-to-bitpack` (data density)
— the two are distinct: `bool-to-bitpack` is about 8× memory compression +
hardware POPCNT; `branchless-conditional` is about auto-vectorization of the
loop body itself.

`entry-vs-contains-insert` wins ~1.5× because the buggy path does TWO hash
lookups (`contains_key` hashes + probes, then `insert` hashes + probes again)
while the fixed path does ONE (`entry().or_insert()` hashes + probes once,
inserts in-place if absent). The compiler cannot merge the two lookups
because `insert` may need to grow the table (opaque call). This is checklist
item 1's "merge HashMap lookups" sub-item — a very common pattern that was
previously not covered. The ~1.5× speedup is a genuine constant-factor
improvement (eliminating one redundant hash + probe per key), not algorithmic.

`loop-fusion` wins ~1.5× because the buggy path iterates the data TWICE (once
for `.sum()`, once for `.max()`) while the fixed path does both reductions in
one pass. The compiler CANNOT fuse the two separate iterator consumers — each
is an independent trait method call that consumes the iterator, and LLVM
doesn't have a loop-fusion pass (it would need to prove aliasing + ordering
constraints across two calls). The win is cache locality: at large N, the
second reduction in the fused loop reads from L1 (data still hot from the
first reduction), while the second pass in the buggy version re-reads from
L2/L3 or RAM. This is checklist item 1's "merge loops" sub-item — the
complement of `entry-vs-contains-insert` (which merges HashMap lookups in the
same iteration) + `collect-avoidance` (which eliminates a `.collect()`
intermediate that forces a second pass). Loop fusion is a distinct pattern:
both passes are read-only consumers over the same slice, but the compiler
can't merge them.

### Investigation note: three candidate gaps tested, one verified

This expansion tested three candidate patterns from the checklist:

| Candidate | Checklist item | Verdict | Honest reason |
|---|---|---|---|
| **loop-fusion** (merge loops) | 1. Complexity | ✅ ParetoDominant (~1.5×) | Compiler can't fuse two separate iterator consumers; cache locality win at all N |
| loop-invariant hoisting (compute once not N×M) | 8. Caching | ❌ ParetoDominated (0.88–1.02×) | **LLVM's LICM pass already hoists the call** — tested with `#[inline(never)]` + `expensive(k)` where k is loop-invariant: LLVM sees the function only depends on k, not on loop state, and hoists it despite no inlining. The manual hoist is sometimes SLOWER (register allocation interference). |
| iter-to-index (use index arithmetic) | 5. Iterators | ❌ ParetoDominated (~1.00×) | **Compiler produces identical codegen** for `.zip().enumerate().map()` chains and direct index loops — tested zip+enumerate+map vs `for i in 0..n`: within noise at all N. LLVM lowers iterator chains to the same pointer-arithmetic loops as explicit indexing. |

The empirical verification BEFORE corpus addition is the key defense: two
of three candidates were honestly ParetoDominated (compiler already handles
them) and were correctly excluded. Only the genuinely benchable pattern
(loop fusion) was added. This mirrors the prior session's methodology
(`entry-vs-contains-insert` + `branchless-conditional` were also verified
before addition).

### Mixed (1/17) — scale-dependent

| Rule | Speedup range | Verdict | Honest reason |
|---|---:|---|---|
| `dedupe-hashset-vs-vec` | 0.64x–7.53x | ~ Mixed (2/4 Faster, 2/4 Slower) | HashSet allocation + hashing overhead exceeds the O(n²) cost at small N (≤500 elements, trivial comparison count). Wins clearly at N ≥ 1000 (up to 7.53x at N=5000). Suggest for dedup on collections known to exceed ~500 elements; for small dedup, the `Vec::contains` approach is fine. |

### Pareto-dominated (2/17) — honest findings

| Rule | Speedup range | Verdict | Honest reason |
|---|---:|---|---|
| `precompute-table` | 0.20x–0.40x | ❌ ParetoDominated | The compiler optimizes `(0..8).filter().count() as u8 & 1` to a single **POPCNT** instruction in release mode, making the lookup table SLOWER (L1 cache latency > POPCNT latency). The rule is NOT a release-mode win — it would only win on architectures without POPCNT or in debug mode. |
| `papaya-lockfree` | 0.55x–0.79x | ❌ ParetoDominated | papaya's epoch-based GC has overhead that only pays off under **multi-threaded contention**. The single-threaded harness measures the overhead without the benefit. This is expected — the rule's value is `Arc<RwLock<HashMap>>` replacement under concurrent access, not single-threaded reads. |

These are NOT bugs in the validator — they're exactly the kind of
honest signal Proposal 001 caveat 3 predicted ("perf has no single
ground truth — 'faster' depends on workload, hardware, input size").

### Not benchable (4/21) — honest markings

| Rule | Reason |
|---|---|
| `layout-reorder` | `size_of` is deterministic (compile-time); the cache-utilization claim needs a specific access pattern to show up in a microbench. |
| `repr-u8-enum` | `size_of` is deterministic (compile-time), same class as `layout-reorder`. |
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
- G1: all 17 benchable rules produce real measurements ✅
- G5: 14/17 benchable rules are Pareto-dominant (majority) ✅

The promotion follows the global AGENTS.md rule "promote to default if gain
also demote loser":
- **Promoted**: the `rust_perf` domain moved from opt-in to `default = [...]`
  in Cargo.toml.
- **Demoted (annotated)**: the 2 ParetoDominated rules (`precompute-table`,
  `papaya-lockfree`) + the 1 Mixed rule (`dedupe-hashset-vs-vec`) now carry a
  `context_notes: Some(...)` annotation on [`PerfRule`] documenting that the
  bench found them slower or scale-dependent in the default config. The
  patterns are kept in the corpus (they're genuine wins in their respective
  regimes — non-POPCNT targets / multi-threaded contention / large N) but
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

# Run the GOAT gate (slow — compiles + runs 11 temp crates)
cargo run --example perf_bench_demo --features rust_perf_bench --release
```

## References

- [Proposal 001 caveat 3](../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness this closes
- [Plan 001 Phase 2](../.plans/001_multi_domain_framework_refactor.md) — the rust_perf domain scaffold this validator completes
- [`.docs/11_domains/rust_perf.md`](../.docs/11_domains/rust_perf.md) — domain doc (updated with the validator hierarchy)
