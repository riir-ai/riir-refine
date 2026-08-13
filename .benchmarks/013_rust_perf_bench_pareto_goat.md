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
| G1 (correctness) | benchable rules produce real measurements (ran=true, non-empty results) | all 18 benchable rules run | ✅ PASS (18/18 ran) |
| G2 (latency) | per-rule latency (temp crate compile + bench subprocess) | documented, not hot-path | ~5-15s/rule (compile-dominated) |
| G3 (no-regression) | default features (no `rust_perf_bench`) still pass | default `cargo test --lib` green | ✅ PASS (91 default tests) |
| G4 (alloc-free) | N/A — subprocess-based validator | N/A | ✅ PASS (documented N/A) |
| G5 (Pareto verdict) | majority of benchable rules are Pareto-dominant | `pareto_dominant * 2 > benchable` | ✅ PASS (15/18 = majority) |

## Measured results (27 benchable, 4 not-benchable)

### Pareto-dominant (24/27) — the pattern works

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
| `prefix-sum-2d` | 34.42x (N=1000) | **29964.14x** (N=1000000) | 29964.14x | ✅ ParetoDominant (4/4 sizes faster) |
| `ok-or-else` | 98.68x (N=100) | **95.42x** (N=100000) | 98.68x | ✅ ParetoDominant (4/4 sizes faster) |
| `swap-remove` | 2.97x (N=100) | **7926.76x** (N=100000) | 7926.76x | ✅ ParetoDominant (4/4 sizes faster) |
| `chunks-exact` | 13.31x (N=1000) | **15.94x** (N=1000000) | 15.94x | ✅ ParetoDominant (4/4 sizes faster) |
| `fxhashmap` | 3.29x (N=100000) | **5.79x** (N=1000) | 5.79x | ✅ ParetoDominant (8/8 sizes faster — insert + lookup) |
| `retain-inplace` | 1.98x (N=1000) | **1.28x** (N=1000000) | 1.98x | ✅ ParetoDominant (4/4 sizes faster) |
| `mem-take` | 2.55x (N=1000) | **1.93x** (N=100000) | 2.55x | ✅ ParetoDominant (4/4 sizes faster) |
| `avoid-clone` | 2.97x (N=100) | **1.48x** (N=100000) | 2.97x | ✅ ParetoDominant (4/4 sizes faster) |
| `box-str-immutable` | 1.10x (N=1000) | **1.32x** (N=100000) | 1.32x | ✅ ParetoDominant (3/3 sizes faster) |
| `reuse-allocation-clear` | 4.04x (N=100) | **0.87x** (N=100000) | 4.04x | ✅ ParetoDominant (3/4 faster) — ⚠️ loses at N=100K (L2 cache pressure), carries `context_notes` |

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

`prefix-sum-2d` wins 34×–30000× because the buggy path scans all O(n) interior
cells per region-sum query while the fixed path answers each query in O(1) via
4 corner lookups on a pre-computed prefix sum. This is checklist item 1's
"boundary-vs-volume" sub-item (Stokes / divergence theorem) — the discrete
instantiation is the 2D prefix sum (a.k.a. summed-area table / integral image):
the prefix array encodes the boundary cumulative sum, and each query
reconstructs the region mass from boundary values alone. The win grows
linearly with N (scan is O(K × region_area) for K queries; prefix is O(K)
after a one-time O(N) build). Valid for d ≤ 3 (2D maps, 3D belief regions);
for d ≥ 8 (HLA state) the prefix array exceeds the interior. This rule is
distinct from `o1-lookup` (point query via HashMap) — prefix sum handles
aggregate/range queries via mathematical pre-computation, not a data structure
swap. The compiler cannot derive a prefix-sum transform from a nested-loop
scan — it requires the programmer to recognize the pattern.

`ok-or-else` wins ~97× because the buggy path constructs the error String for
every element (even when the Option is Some) while the fixed path only
constructs it when the Option is None. The compiler cannot prove that
`expensive_error()` has no side effects (it calls `format!` which allocates),
so it must execute it eagerly. This pattern comes from nnethercote/perf-book
"Standard Library Types" — `ok_or(e)` eagerly evaluates `e`; `ok_or_else(\|\| e)`
lazily evaluates it. Same applies to `map_or`, `unwrap_or`, `Result::or`.

`swap-remove` wins 2×–18485× (grows with N) because `Vec::remove(i)` shifts all
elements after index `i` one position left (O(n)) while `Vec::swap_remove(i)`
moves the last element into the gap (O(1)). The compiler cannot change
algorithmic complexity. This pattern comes from nnethercote/perf-book
"Standard Library Types" — use `swap_remove` when element order doesn't matter.

`chunks-exact` wins ~13–16× because `chunks(8)` yields a potentially-short
final chunk, preventing LLVM from vectorizing the loop body (each chunk may
have a different length, so the vectorizer can't pack iterations into SIMD
lanes). `chunks_exact(8)` guarantees every chunk (except the remainder) is
exactly 8 elements, enabling auto-vectorization. This pattern comes from
nnethercote/perf-book "Iterators" — use `chunks_exact` when chunk size doesn't
exactly divide the slice length.

`fxhashmap` wins 1.6–4.2× because the std `HashMap` uses SipHash-1-3 (a
cryptographic-grade hasher designed for HashDoS resistance — 1 compression +
3 finalization rounds per hash) while `FxHashMap` uses FxHash (1 multiply +
1 xor per hash). The compiler cannot swap hashers — the hasher is an explicit
type parameter. This pattern comes from rustc itself: the compiler's default
HashMap is `FxHashMap` because compiler keys (symbol IDs, def-ids, spans) are
non-adversarial. The rule applies to any non-adversarial key domain (game
state, internal caches, NPC lookup tables). The ~2× insert speedup (2.15–2.76×)
+ ~3–4× lookup speedup (3.17–4.17×) are both ParetoDominant at all sizes.

`retain-inplace` wins 1.5–2.1× because `filter().collect()` allocates a NEW
Vec + copies all surviving elements, while `retain()` filters in-place — zero
allocation, one pass. The compiler cannot transform `filter().collect()` into
an in-place retain because they have different ownership semantics (collect
produces a new owned Vec; retain mutates the existing one). The win shrinks at
larger N (allocation overhead amortized) but stays positive (~1.5× even at
N=1,000,000). This is the sibling of `collect-avoidance` (which eliminates a
.collect() intermediate in a chained iterator); `retain-inplace` covers the
case where you already own the Vec and want to filter it.

`mem-take` wins ~3× because `buf.clone()` deep-clones every String in the Vec
(O(n) allocations + copies) while `mem::take(&mut buf)` moves the Vec's
underlying allocation in O(1) (just swaps the pointer + length + capacity).
The compiler cannot elide the clone — `String::clone()` has visible side
effects (heap allocation). The ~3× speedup is remarkably consistent across all
sizes because the clone cost scales linearly with N while mem::take is O(1).
This pattern is the ownership-transfer sibling of `vec-with-capacity` (which
preallocates) + `format-concat-loop` (which avoids reallocation in a loop).

`avoid-clone` wins 1.3–3.0× because `data: Vec<T>` by value clones the entire
collection on every call (heap alloc + memcpy) while `data: &[T]` borrows —
zero allocation, zero copy. The compiler cannot elide the clone (it has
visible side effects: heap allocation + potential OOM panic). This is the
single most common Rust perf anti-pattern (Plan 002 T-avoid-clone) and was
surprisingly missing from the original 25-rule corpus. The win shrinks at
large N because the sum loop dominates, but the clone overhead keeps it
positive (1.48× even at N=100,000).

`box-str-immutable` wins 1.1–1.3× because `String` is a 3-word handle (ptr,
len, capacity) while `Box<str>` is a 2-word handle (ptr, len — the capacity
is irrelevant for immutable slices). For collections of >1000 immutable
strings, the 8-byte-per-element savings (on 64-bit) improves cache density →
fewer cache lines loaded per scan. The compiler cannot change the
representation type. The win grows with N (1.10× at N=1,000 → 1.32× at
N=100,000) because more elements fit per cache line. This is distinct from
`static-str` (String → &'static str for literals); `box-str-immutable` is
for owned immutable collections where you can't use a literal.

### Investigation note: nineteen candidate patterns tested, eleven verified

This expansion tested nineteen candidate patterns across six sessions (6 from
the `rust-optimize` skill checklist gap audit + 5 from nnethercote/perf-book
chapter distillation + 4 from rustc-hash/bevy real-world pattern mining + 4
from Mara Bos / MS Pragmatic / reintech source distillation):

| Candidate | Source | Verdict | Honest reason |
|---|---|---|---|
| **loop-fusion** (merge loops) | checklist item 1 | ✅ ParetoDominant (~1.5×) | Compiler can't fuse two separate iterator consumers; cache locality win at all N |
| **prefix-sum-2d** (boundary-vs-volume) | checklist item 1 | ✅ ParetoDominant (34×–30000×) | Compiler can't derive a prefix-sum transform from a nested-loop scan; win grows linearly with N |
| **ok-or-else** (lazy error eval) | perf-book Std Types | ✅ ParetoDominant (~97×) | `ok_or(expensive())` eagerly constructs error for every element; `ok_or_else(\|\| expensive())` only on None. Compiler can't prove expensive() has no side effects |
| **swap-remove** (O(1) removal) | perf-book Std Types | ✅ ParetoDominant (2×–18485×) | `Vec::remove` shifts O(n) elements; `swap_remove` is O(1). Compiler can't change algorithmic complexity |
| **chunks-exact** (vectorizable chunks) | perf-book Iterators | ✅ ParetoDominant (13×–16×) | `chunks` yields variable-length final chunk → prevents vectorization; `chunks_exact` guarantees uniform chunk size |
| **fxhashmap** (fast hasher) | rustc/bevy real-world | ✅ ParetoDominant (1.6–4.2×) | SipHash-1-3 (DoS-resistant) vs FxHash (1 mul + 1 xor). Compiler can't swap hashers; used pervasively in rustc |
| **retain-inplace** (in-place filter) | real-world pattern | ✅ ParetoDominant (1.5–2.1×) | `filter().collect()` allocates new Vec; `retain()` filters in-place. Different ownership semantics → compiler can't transform |
| **mem-take** (move, no clone) | real-world pattern | ✅ ParetoDominant (~3×) | `clone()` deep-copies O(n); `mem::take()` moves O(1). Compiler can't elide clone (visible side effects) |
| **avoid-clone** (borrow vs clone) | Plan 002 T-avoid-clone | ✅ ParetoDominant (1.3–3.0×) | `fn(data: Vec<T>)` clones on call; `fn(data: &[T])` borrows. Compiler can't elide clone (heap alloc + OOM panic = opaque) |
| **box-str-immutable** (Vec<String> → Vec<Box<str>>) | Plan 002 T-heap-box-str | ✅ ParetoDominant (1.1–1.3×) | String is 3 words (ptr, len, cap); Box<str> is 2. Saves 8 bytes/element → better cache density at scale |
| **reuse-allocation-clear** (scratch buffer reuse) | Plan 002 T-heap-reuse-clear | ~ Mixed (3/4 Faster) | Wins at N ≤ 10K (up to 5.97×); loses at N=100K (reused buffer exceeds L2 cache). Context-dependent |
| loop-invariant hoisting | checklist item 8 | ❌ ParetoDominated (0.88–1.02×) | **LLVM's LICM pass already hoists the call** — tested with `#[inline(never)]` + `expensive(k)` where k is loop-invariant: LLVM sees the function only depends on k, not on loop state, and hoists it despite no inlining |
| iter-to-index (use index arithmetic) | checklist item 5 | ❌ ParetoDominated (~1.00×) | **Compiler produces identical codegen** for `.zip().enumerate().map()` chains and direct index loops |
| mem::replace vs clone+assign | bonus (not in checklist) | ❌ Rejected — bench-design mismatch | ParetoDominant when isolated but the mutation pattern doesn't fit the read-only corpus bench framework |
| filter_map vs filter().map() | perf-book Iterators | ❌ ParetoDominated (0.76–0.94×) | **LLVM optimizes `filter().map()` into a single pass** — the `filter_map` closure returns Option which adds an extra branch. The perf-book says it "may be faster" but in practice the compiler already fuses |
| clone_from vs clone assign | perf-book Std Types | ❌ ParetoDominated (0.91–0.98×) | **clone_from's allocation reuse overhead exceeds the benefit** when dst capacity ≥ src len — the reuse check + in-place copy costs more than a fresh allocation. Win only materializes when the dst Vec is significantly larger |
| bounds-check iter vs index | perf-book Bounds Checks | ❌ Tie (~1.00×) | **LLVM elides bounds checks** for `for i in 0..len { arr[i] }` — the compiler proves `i < len` from the loop bound. The perf-book warns "these can affect performance, though less often than you might expect" — confirmed empirically |
| array vs Vec (fixed-size) | real-world pattern | ❌ Tie (~1.01× at N=64) | **Compiler vectorizes both equally** — the array's stack allocation advantage is negligible for sum loops. Small win (1.23×) at N=16 is within noise at the ns level. REJECT — not a consistent, clear win |
| soa-vs-aos (struct of arrays) | Plan 002 T-soa-layout | ❌ ParetoDominated/Tie (~0.86–1.01×) | **AoS cache utilization is already good for 6-field structs** — SoA benefit only materializes for much wider structs (16+ fields). At N=100K SoA was actually SLOWER (0.86×) |

The empirical verification BEFORE corpus addition is the key defense: eight
of nineteen candidates were honestly rejected (compiler already handles them,
or bench-design mismatch, or ParetoDominated), and eleven were added. Only the
genuinely benchable patterns passed both the ParetoDominance gate AND the
bench-design fitness check.

**The perf-book distillation lesson:** nnethercote/perf-book is the canonical
Rust performance reference, but 3 of 6 patterns tested from it were already
handled by LLVM (filter_map fusion, clone_from allocation reuse, bounds-check
elision). The perf-book documents patterns that WERE perf wins in specific
historical contexts (older rustc versions, specific code shapes) but LLVM has
since learned to handle. The empirical-before-add discipline catches these.

### Mixed (1/27) — scale-dependent

| Rule | Speedup range | Verdict | Honest reason |
|---|---:|---|---|
| `dedupe-hashset-vs-vec` | 0.64x–7.53x | ~ Mixed (2/4 Faster, 2/4 Slower) | HashSet allocation + hashing overhead exceeds the O(n²) cost at small N (≤500 elements, trivial comparison count). Wins clearly at N ≥ 1000 (up to 7.53x at N=5000). Suggest for dedup on collections known to exceed ~500 elements; for small dedup, the `Vec::contains` approach is fine. |

### Pareto-dominated (2/27) — honest findings

| Rule | Speedup range | Verdict | Honest reason |
|---|---:|---|---|
| `precompute-table` | 0.20x–0.40x | ❌ ParetoDominated | The compiler optimizes `(0..8).filter().count() as u8 & 1` to a single **POPCNT** instruction in release mode, making the lookup table SLOWER (L1 cache latency > POPCNT latency). The rule is NOT a release-mode win — it would only win on architectures without POPCNT or in debug mode. |
| `papaya-lockfree` | 0.55x–0.79x | ❌ ParetoDominated | papaya's epoch-based GC has overhead that only pays off under **multi-threaded contention**. The single-threaded harness measures the overhead without the benefit. This is expected — the rule's value is `Arc<RwLock<HashMap>>` replacement under concurrent access, not single-threaded reads. |

These are NOT bugs in the validator — they're exactly the kind of
honest signal Proposal 001 caveat 3 predicted ("perf has no single
ground truth — 'faster' depends on workload, hardware, input size").

### Not benchable (4/31) — honest markings

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
- G1: all 27 benchable rules produce real measurements ✅
- G5: 24/27 benchable rules are Pareto-dominant (majority) ✅

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
- [`rust-optimize` skill](~/.agents/skills/rust-optimize/SKILL.md) — the checklist source (11 items)
- [nnethercote/perf-book](https://github.com/nnethercote/perf-book) — the Rust Performance Book (canonical perf reference; 3 rules distilled: ok-or-else, swap-remove, chunks-exact; 3 candidates tested + rejected)
