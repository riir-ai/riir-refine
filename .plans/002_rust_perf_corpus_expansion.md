# Plan 002 — rust_perf Corpus Expansion (Multi-Source Distillation)

**Proposal:** [001](../.proposals/001_multi_domain_modelless_code_healer.md) (caveat 3
extension — corpus coverage, not a new domain)
**Branch:** `develop` (per global rule — no feature branches)
**Status:** in flight — Phase 1 T-hash DONE (2026-08-13, commit `c5e27a3`); corpus now 28 rules. Remaining Phase 1 candidates + Phases 2-4 pending.
**Domain:** `rust_perf` (default-on since 2026-08-13, [Bench 013](../.benchmarks/013_rust_perf_bench_pareto_goat.md))

## Goal

Expand the `rust_perf` corpus from its current 25 rules by distilling new
perf patterns from five sources the user identified. The driving concern
(two parts):

1. **Recheck nnethercote/perf-book coverage.** Bench 013's "References"
   line says "3 rules distilled" from perf-book. A coverage audit shows the
   perf-book has **18 chapters**; only **3** were touched
   (`standard-library-types`, `iterators`, `bounds-checks`). **15 chapters
   are untouched** — a genuine gap, not an intentional limit. This plan
   closes it.
2. **Distill four new sources** the user flagged:
   - Mara Bos, *Rust Atomics and Locks* (mara.nl/atomics + m-ou-se/rust-atomics-and-locks)
   - Kobzol, *Optimizing Rust programs with PGO and BOLT* (cargo-pgo)
   - Microsoft *Pragmatic Rust Guidelines* — Performance
   - reintech *Rust Performance Optimization: Complete Guide 2026*

**The empirical-before-add discipline (Bench 013 lesson) is mandatory for
every candidate:** each new rule is added to the corpus only after its
`BenchSpec` shows ParetoDominance. Candidates the compiler already handles
(or that are ParetoDominated) are honestly rejected + recorded in the
investigation log. The 5/11 acceptance rate from the last expansion is the
expected baseline — perf claims rot as LLVM improves.

## Substrate-first gate (pre-flight, MANDATORY)

Before implementing, verify no substrate is duplicated:
- [x] Existing `PerfRule` struct + `build_perf_seed_corpus()` (28 rules as of 2026-08-13) — the
      corpus to extend, not replace.
- [x] `BenchSpec` harness pattern + `PerfBenchValidator` (Bench 013) — the
      empirical gate every new benchable rule must pass.
- [x] `context_notes` annotation path (Bench 013 promotion) — for rules that
      are ParetoDominated/Mixed but kept as documented patterns.
- [x] `HealEntry::context_notes()` default trait method — new rules inherit
      annotation propagation automatically; no framework change needed.

No new substrate invented. This is pure corpus data addition + bench specs.

## Recheck finding: nnethercote/perf-book coverage (the user's "weird" concern)

**Verdict: the user is correct — only 3 of 18 chapters were distilled.** This
is a real gap, not an intentional scoping decision. The original Phase 2
corpus came from the `rust-optimize` skill checklist (the 14-item list), with
perf-book as a secondary source that only 3 chapters were sampled from.

| perf-book chapter | Status before this plan | Action |
|---|---|---|
| `standard-library-types.md` | ✅ distilled (`ok-or-else`, `swap-remove`); rejected (`clone_from`) | done — no action |
| `iterators.md` | ✅ distilled (`chunks-exact`); rejected (`filter_map`) | done — no action |
| `bounds-checks.md` | ❌ tested + rejected (LLVM elides) | done — no action |
| `hashing.md` | 🆕 untouched | **T-hash candidate** |
| `heap-allocations.md` | 🆕 untouched | **T-heap-* candidates** |
| `inlining.md` | 🆕 untouched | **T-inline candidate** |
| `io.md` | 🆕 untouched | **T-io candidate** |
| `parallelism.md` | 🆕 untouched | **T-rayon candidate** |
| `type-sizes.md` | 🆕 untouched | **T-box-variant candidate** |
| `wrapper-types.md` | 🆕 untouched | **T-box-avoid candidate** |
| `build-configuration.md` | 🆕 untouched | **T-buildconfig (design decision)** |
| `benchmarking.md` | 🆕 meta (how-to, not a code rule) | not distillable |
| `profiling.md` | 🆕 meta | not distillable |
| `machine-code.md` | 🆕 meta | not distillable |
| `general-tips.md` | 🆕 meta | not distillable |
| `compile-times.md` | 🆕 meta (compile speed, not runtime) | not distillable |
| `linting.md` | 🆕 meta (clippy_domain owns this) | not distillable |
| `logging-and-debugging.md` | 🆕 meta | not distillable |

**7 chapters are distillable into code rules; 8 are meta (profiling/benchmarking
how-tos) and correctly excluded.** This plan distills the 7.

## Complete candidate task list

Every candidate is a `- [ ]` task below. An agent works top-to-bottom; each
task is self-contained (source + pattern + bench feasibility). Mark `[x]` when
distilled+benched, `[-]` when honestly rejected (with reason recorded in the
investigation log), `[~]` when deferred.

> **Bench feasibility key:**
> - **Benchable** — fits the temp-crate `BenchSpec` harness (pure Rust, no external runtime)
> - **NotBenchable** — needs an external runtime (Bevy/WASM/OS) or is structural (size_of); add to corpus with `bench_spec: None`
> - **Design-decision** — doesn't fit the code-healer shape (edits Cargo.toml, not .rs); needs a scope ruling before adding

### Source A — nnethercote/perf-book (7 untouched distillable chapters)

- [x] **T-hash** · `fast-hasher` · category: complexity · **Benchable** — DONE 2026-08-13
  - Source: perf-book `hashing.md` + MS M-FAST-HASHER + reintech FxHashMap + rustc/bevy real-world
  - Pattern: `HashMap::default()` (SipHash, DoS-safe) for trusted internal keys →
    `FxHashMap`/`foldhash` (faster non-crypto hasher). The default hasher's
    randomization + SipHash rounds cost ~2-10× vs FxHash on integer/pointer keys.
  - **Bench result (Bench 013 validator):** ParetoDominant 8/8 sizes (3.3-5.8×).
    Shipped as rule `fxhashmap` in commit `c5e27a3`.
  - **Category note:** filed under `complexity` (not new `hashing` category) because
    the existing complexity category already holds the HashMap-related rules
    (`o1-lookup`, `entry-vs-contains-insert`). A 1-rule category is too sparse.
  - Overlap: none. Distinct from `o1-lookup` (that's algorithmic;
    this is constant-factor on the hash function itself).

- [ ] **T-heap-box-str** · `box-str-immutable` · category: allocation · **Benchable**
  - Source: perf-book `heap-allocations.md` + MS M-BOX-DST
  - Pattern: `Vec<String>` / owned `String` for immutable sequences →
    `Vec<Box<str>>` / `Box<str>`. Drops the `capacity` word (3-word → 2-word
    handle) + shrink-to-fit. Win when the collection has >1000 instances.
  - Overlap: none. Distinct from `static-str` (that's String→&'static str for
    literals; this is owned-String→Box<str> for stored immutable data).

- [ ] **T-heap-smallvec** · `smallvec-stack` · category: allocation · **Benchable**
  - Source: perf-book `heap-allocations.md` + reintech SmallVec
  - Pattern: `Vec<T>` for a usually-small collection (≤8 elements typical) →
    `SmallVec<[T; 8]>` (stack-allocates the common case, spills to heap on overflow).
  - Overlap: none. Distinct from `vec-with-capacity` (that pre-allocates heap;
    this avoids heap entirely for the common case). Needs `smallvec` dep in bench crate.

- [ ] **T-heap-reuse-clear** · `reuse-allocation-clear` · category: allocation · **Benchable**
  - Source: perf-book `heap-allocations.md` + MS M-MEM-REUSE
  - Pattern: allocate-fresh-per-call (`let mut v = Vec::new()` inside a hot fn) →
    reuse via `.clear()` on a caller-passed buffer (`fn hot(buf: &mut Vec<T>)`).
    Eliminates per-call alloc. The compiler cannot hoist the allocation (it may
    panic on OOM, so it's an opaque call).
  - Overlap: none. This is the "scratch buffer" pattern from AGENTS.md optimization
    guidelines ("Reuse scratch buffers across loop iterations").

- [ ] **T-heap-shrink-fit** · `shrink-to-fit-after-build` · category: allocation · **NotBenchable** (memory, not speed)
  - Source: perf-book `heap-allocations.md` + MS M-SHRINK-TO-FIT
  - Pattern: long-lived `Vec` built by push-loop without capacity → `.shrink_to_fit()`
    before storing. Frees the ~2× over-allocation from doubling growth.
  - Bench: NotBenchable — the win is memory footprint, not speed (shrinking may
    even cost a realloc). Add as documented rule with `bench_spec: None` +
    `context_notes` noting it's a memory win, not a speed win.

- [ ] **T-inline** · `inline-hot-fn` · category: inlining · **Benchable** (likely reject)
  - Source: perf-book `inlining.md` + reintech #[inline]
  - Pattern: small hot fn called in a tight loop → add `#[inline]`.
  - Honest prediction: **likely ParetoDominated.** LLVM inlines aggressively in
    release mode within a crate; `#[inline]` only helps cross-crate. The perf-book
    itself says the effect is "modest". TEST + record the honest finding either way.

- [ ] **T-io-buffered** · `buffered-io` · category: io · **Benchable** (needs file I/O)
  - Source: perf-book `io.md`
  - Pattern: `File::read`/`File::write` per-line → `BufReader`/`BufWriter` wrapper.
    Each unbuffered syscall has ~µs overhead; buffering amortizes to ~ns/byte.
  - Bench: feasible in a temp crate (write temp file, read it back). Slower bench
    (I/O-bound) but valid. The `io` category is new.

- [ ] **T-rayon** · `rayon-par-iter` · category: parallelism · **Benchable** (needs rayon dep)
  - Source: perf-book `parallelism.md` + reintech Rayon
  - Pattern: sequential `.iter().map(expensive).collect()` → `.par_iter().map(expensive).collect()`
    for CPU-bound work where each item > ~5µs (rayon thread-pool overhead floor).
  - Bench: needs `rayon` dep in the temp crate (network fetch on first build, like
    criterion — but rayon is lighter). Feasible. The `parallelism` category is new.
  - **Caveat (global AGENTS.md):** "Only parallelize when per-task work exceeds
    thread-pool overhead (~5μs for rayon)". The bench must use expensive-enough items
    or it'll show rayon SLOWER (the overhead floor). Document the threshold in `context_notes`.

- [ ] **T-box-variant** · `box-large-enum-variant` · category: layout · **NotBenchable** (size_of)
  - Source: perf-book `type-sizes.md`
  - Pattern: enum with one rare large variant → `Box<LargeVariant>` to shrink the
    enum's size to `max(common_variants, Box.word)`. The common case avoids paying
    for the rare variant's size in every enum value.
  - Bench: NotBenchable (deterministic `size_of`), same class as `layout-reorder`.
    Add as documented rule.

- [ ] **T-box-avoid** · `avoid-unnecessary-box` · category: allocation · **Benchable**
  - Source: perf-book `wrapper-types.md` + MS M-AVOID-INDIRECTION
  - Pattern: `Box<T>` where `T` is small (≤2 words) → `T` directly. The Box adds a
    heap indirection (cache miss) for no benefit when T fits in a register pair.
  - Overlap: distinct from `dyn-to-enum` (that's vtable devirt; this is removing a
    needless heap hop for a concrete small type).

- [ ] **T-buildconfig** · build-configuration (PGO/LTO/codegen-units) · **Design-decision**
  - Source: perf-book `build-configuration.md` + Kobzol PGO article + reintech compiler flags
  - Pattern: `codegen-units=16` (default) → `1`; `lto=false` → `thin`; add PGO
    (`-Cprofile-generate` → run → `-Cprofile-use`); BOLT post-link.
  - **Scope ruling needed:** the code healer edits `.rs` files via `syn` AST. These
    are `Cargo.toml` / `[profile.release]` / CI-workflow changes — **out of scope
    for the code healer.** Decision: document these in `.docs/11_domains/rust_perf.md`
    as "build-config recommendations (not healable rules)" rather than adding corpus
    entries. The healer cannot apply them. See Phase 3.

### Source B — Mara Bos, *Rust Atomics and Locks* (concurrency depth)

The book has 10 chapters; the distillable CODE PATTERNS (not the meta/educational
chapters on processor architecture, OS primitives, or "build your own lock"
exercises) are below. Chapters 4-9 are largely "build it yourself to learn"
(exercises), not production recommendations — std/primitive alternatives win.

- [ ] **T-scope-threads** · `scoped-threads` · category: concurrency · **Benchable**
  - Source: Mara Bos Ch1 (scoped threads) + Ch6 (Arc cost)
  - Pattern: `Arc<T>` + `thread::spawn` + `join` (heap-alloc the shared data + refcount
    inc/dec) → `thread::scope` (borrow shared data by reference, no Arc, no clone).
    Eliminates the Arc allocation + atomic refcount overhead.
  - Overlap: none. New `concurrency` sub-pattern. Benchable (Arc alloc + 2 atomic
    ops vs zero).

- [ ] **T-rwlock-read-heavy** · `rwlock-vs-mutex-read-heavy` · category: concurrency · **Benchable**
  - Source: Mara Bos Ch1 (RwLock) + Ch9
  - Pattern: `Mutex<T>` protecting read-heavy state (99% reads) → `RwLock<T>`
    (multiple concurrent readers). Mutex serializes all readers; RwLock allows
    N concurrent read locks.
  - Overlap: none. **Caveat:** RwLock has higher per-op overhead than Mutex
    (reader counting). Wins ONLY when read:write ratio is high AND critical section
    is non-trivial. Bench must use a read-heavy workload or it'll show RwLock slower.
    Document the regime in `context_notes`.

- [ ] **T-cas-to-fetch** · `cas-loop-to-fetch-op` · category: concurrency · **Benchable**
  - Source: Mara Bos Ch2 (compare_exchange vs fetch_add)
  - Pattern: `compare_exchange` loop for a simple increment (`loop { let o = x.load(); if x.cas(o, o+1).is_ok() { break } }`) → `fetch_add(1, Relaxed)`.
    The CAS loop can fail + retry under contention; fetch_add is a single hardware
    RMW instruction that never retries.
  - Overlap: distinct from `mutex-to-atomic` (that's Mutex→Atomic; this is
    misused-Atomic→correct-Atomic). Common bug pattern.

- [ ] **T-ordering-relaxed** · `ordering-relaxed-where-safe` · category: concurrency · **Benchable** (ARM-visible, x86 marginal)
  - Source: Mara Bos Ch3 (Memory Ordering)
  - Pattern: `SeqCst` ordering for a counter that only needs atomicity (no
    cross-thread ordering dependency) → `Relaxed`. SeqCst inserts a fence on ARM/ARM64;
    Relaxed is a plain atomic op. On x86 (strongly-ordered) the difference is marginal;
    on ARM the fence cost is measurable.
  - **Caveat:** correctness-critical — Relaxed is only safe when no happens-before
    edge is needed. The rule's `context_notes` must carry the safety condition.
    Bench on M3 (ARM64) should show the fence cost; on x86 it may be a Tie.
  - Overlap: none.

- [ ] **T-park-vs-spin** · `parking-vs-busy-spin` · category: concurrency · **Benchable**
  - Source: Mara Bos Ch1 (thread parking) + Ch9 (avoid busy-looping)
  - Pattern: `while !ready { /* spin */ }` busy-wait → `while !ready { thread::park(); }`
    or condvar. Busy-spin burns 100% CPU + starves the thread that would set `ready`.
  - Overlap: none. Benchable (CPU-time metric, not just wall-clock).

### Source C — Kobzol, *PGO and BOLT* (cargo-pgo)

- [ ] **T-pgo** · profile-guided optimization · **Design-decision** (see T-buildconfig)
  - Source: Kobzol blog (cargo-pgo)
  - Pattern: PGO (instrumented build → run workload → rebuild with profile) gives
    ~10-20% on CPU-bound code; BOLT post-link adds ~2-5% more.
  - **Same scope ruling as T-buildconfig:** this is a Cargo.toml/CI workflow change,
    not a `.rs` code pattern the healer can apply. Fold into the build-config doc
    section (Phase 3), not the corpus.

### Source D — Microsoft *Pragmatic Rust Guidelines* — Performance

(Items that overlap with perf-book candidates above are cross-referenced, not
re-listed. New items only.)

- [ ] **T-async-yield** · `async-yield-point` · category: async · **Benchable** (fairness, not speed)
  - Source: MS M-YIELD-POINTS
  - Pattern: long-running CPU-bound `async fn` without `.await` → insert
    `yield_now().await` every 10-100µs of work. Without yield points, the task
    starves other tasks on the same executor (cooperative scheduling).
  - Bench: the win is fairness/latency-tail, not throughput. NotBenchable as a
    speed win; add with `context_notes` noting it's a latency-fairness rule.

- [ ] **T-log-no-format** · `log-no-format-in-hot-loop` · category: allocation · **Benchable**
  - Source: MS M-LOG-OVERHEAD
  - Pattern: `log(format!("...{}", x))` inside a hot loop (allocates the String every
    iteration even if logging is disabled) → structured/tuple log args
    `log(("...", x))` or gate behind `if log_enabled!()`. The `format!` allocates
    unconditionally.
  - Overlap: none. Distinct from `format-concat-loop` (that's building an output
    string; this is logging overhead). Very common in instrumented hot paths.

- [ ] **T-lift-hot-field** · `lift-hot-field-from-arc` · category: layout · **Benchable**
  - Source: MS M-AVOID-INDIRECTION
  - Pattern: `struct Item { config: Arc<Config> }` where `config.feature_flag` is
    checked on every hot call (2 pointer hops: Item→Config→Feature) → denormalize
    the hot field: `struct Item { config: Arc<Config>, feature_flag: bool }`.
    One hop instead of two; the flag is cache-collocated with Item.
  - Overlap: distinct from `avoid-unnecessary-box` (that removes a Box; this
    denormalizes a field out of an Arc chain while keeping the Arc).

- [ ] **T-async-future-size** · `async-future-size` · category: async · **NotBenchable** (size_of_val)
  - Source: MS M-ASYNC-STACK-SIZE
  - Pattern: `async fn hot(large: Large)` embeds `Large` into the future's state
    machine (blows up `size_of_val(&future)`) → `fn hot(large: Large) -> impl Future`
    + process `large` before the async block, or box the large local across await.
  - Bench: NotBenchable (deterministic `size_of_val`). Add as documented rule.

### Source E — reintech *Rust Performance Optimization 2026*

(Most items overlap with perf-book/MS above — cross-referenced. New items only.)

- [ ] **T-object-pool** · `object-pool-reuse` · category: allocation · **Benchable**
  - Source: reintech Object Pools
  - Pattern: allocate+drop in a hot loop (`for x in items { let buf = Vec::with_capacity(N); ... }`)
    → object pool (`Mutex<Vec<Vec<u8>>>` acquire/release; reuse allocations).
  - Overlap: related to `reuse-allocation-clear` (T-heap-reuse-clear) but distinct
    — pool is for when the buffer can't be threaded through the call signature
    (third-party API); clear is when you own the signature. Both are "reuse alloc"
    but the mechanism differs.

- [ ] **T-soa-layout** · `soa-vs-aos` · category: simd/layout · **Benchable**
  - Source: reintech SoA vs AoS
  - Pattern: `Vec<Particle { x, y, z, vx, vy, vz }>` (AoS — processing `x` loads
    `y,z,vx,vy,vz` into cache unused) → `StructOfArrays { xs: Vec<f32>, ys: Vec<f32>, ... }`
    (SoA — processing `x` streams only `xs`). Major cache + vectorization win for
    field-wise iteration.
  - Overlap: distinct from `layout-reorder` (that's padding within one struct;
    this is the data layout across a collection). Big structural change but a
    classic HPC/simd pattern.

- [ ] **T-avoid-clone** · `avoid-unnecessary-clone` · category: allocation · **Benchable**
  - Source: reintech + general Rust wisdom (surprisingly NOT in the current corpus!)
  - Pattern: `.clone()` of a large collection/String where a borrow `&` suffices
    (`fn process(data: Vec<u8>)` cloning on every call → `fn process(data: &[u8])`).
    Clone does a full heap allocation + memcpy.
  - **Gap note:** this is the single most common Rust perf anti-pattern and it's
    NOT in the current 25 rules. High-value addition.

## Phase 1 — Benchable candidates (empirical Pareto gate)

For each `- [ ]` task marked **Benchable** above, the workflow is:
1. Write a `BenchSpec` (buggy + fixed harness, `make_input`/`run_buggy`/`run_fixed`).
2. Run `cargo run --example perf_bench_demo --features rust_perf_bench --release`.
3. If **ParetoDominant** → add `PerfRule` to corpus + check the task `[x]`.
4. If **ParetoDominated** → record in investigation log (Bench 013 §"Investigation
   note" style), do NOT add to corpus, check the task `[-]` with reason.
5. If **Mixed** → add to corpus WITH `context_notes` documenting the regime, check `[x]`.

### Tasks (benchable candidates — empirical gate)

- [x] **T-hash** — `fast-hasher` (FxHash vs SipHash on int keys) — DONE 2026-08-13, ParetoDominant 3.3-5.8×
- [x] **T-heap-box-str** — `box-str-immutable` — DONE 2026-08-13, ParetoDominant 1.1-1.3×
- [x] **T-heap-smallvec** — `smallvec-stack` — DONE 2026-08-13, ParetoDominant 27-75× (stack array for ≤8 typical)
- [x] **T-heap-reuse-clear** — `reuse-allocation-clear` — DONE 2026-08-13, Mixed (3/4 Faster, carries context_notes)
- [-] **T-inline** — `inline-hot-fn` — REJECTED 2026-08-13: ParetoDominated (0.73-0.75×, inline SLOWER). `#[inline(always)]` causes code bloat that hurts i-cache. With proper black_box to prevent constant folding, the inlined version is ~25% slower. LLVM's heuristics are better than human annotation.
- [x] **T-io-buffered** — `buffered-io` — DONE 2026-08-13, ParetoDominant 99-132× (BufReader 8KB batches vs syscall per byte)
- [ ] **T-rayon** — `rayon-par-iter` (needs rayon dep + >5µs/item workload)
- [x] **T-box-avoid** — `avoid-unnecessary-box` — DONE 2026-08-13, ParetoDominant 9.6-42× (heap alloc per element)
- [-] **T-scope-threads** — `scoped-threads` — REJECTED 2026-08-13: Tie (0.98-1.01×). Arc clone cost (~10ns) is negligible vs thread spawn (~7µs). The win is architectural (no Arc needed, borrow checker safety), not perf.
- [-] **T-rwlock-read-heavy** — `rwlock-vs-mutex` — REJECTED 2026-08-13: ParetoDominated (0.94-0.96×, RwLock SLOWER). RwLock has more per-op overhead than Mutex. Win only materializes under TRUE parallelism (multiple concurrent readers), which the sequential harness doesn't exercise.
- [-] **T-cas-to-fetch** — `cas-loop-to-fetch-op` — REJECTED 2026-08-13: Tie (1.07-1.09×). Single-threaded CAS rarely retries (no contention). The win needs multi-threaded contention, which the harness doesn't exercise.
- [-] **T-ordering-relaxed** — `ordering-relaxed-where-safe` — REJECTED 2026-08-13: Tie (1.00-1.02×). No difference on ARM64 (M3 Max) — both SeqCst and Relaxed compile to LDAR/STAR. The cost difference is x86-only.
- [ ] **T-park-vs-spin** — `parking-vs-busy-spin` (spin → thread::park; CPU-time metric)
- [ ] **T-log-no-format** — `log-no-format-in-hot-loop` (format! in loop → structured args)
- [ ] **T-lift-hot-field** — `lift-hot-field-from-arc` (denormalize hot bool out of Arc chain)
- [-] **T-object-pool** — `object-pool-reuse` — REJECTED 2026-08-13: Duplicate of `reuse-allocation-clear`. Same underlying pattern (clear vs alloc). Adding would inflate count without adding insight.
- [-] **T-soa-layout** — `soa-vs-aos` — REJECTED 2026-08-13: ParetoDominated/Tie (0.86-1.01×). AoS cache utilization already good for 6-field structs.
- [x] **T-avoid-clone** — `avoid-unnecessary-clone` — DONE 2026-08-13, ParetoDominant 1.3-3.0×
  - **Additional rules shipped outside this plan's candidate list (rustc/bevy pattern mining):**
- [x] **retain-inplace** — `iter().filter().collect()` → `Vec::retain()` in-place (1.3-2.0×) — DONE 2026-08-13
- [x] **mem-take** — `buf.clone() + buf.clear()` → `mem::take()` (1.9-2.6×) — DONE 2026-08-13

**Expected outcome:** based on the Bench 013 5/11 acceptance rate, expect ~9-12
of these 18 to pass ParetoDominance, ~4-6 to be honestly rejected (LLVM already
handles them), ~2-3 Mixed (regime-gated). The empirical gate decides — no claim
ships without measurement.

## Phase 2 — NotBenchable candidates (documented rules, no bench)

These are real perf patterns but the win is structural (memory size / future size)
not runtime speed. Add to corpus with `bench_spec: None` + honest `context_notes`.

- [ ] **T-heap-shrink-fit** — `shrink-to-fit-after-build` (memory win, not speed)
- [ ] **T-box-variant** — `box-large-enum-variant` (size_of; shrink common case)
- [ ] **T-async-yield** — `async-yield-point` (fairness/latency-tail, not throughput)
- [ ] **T-async-future-size** — `async-future-size` (size_of_val; reduce future state machine)

## Phase 3 — Build-config ruling + doc (design decision)

The code healer edits `.rs` via `syn`. PGO / LTO / codegen-units / BOLT are
`Cargo.toml` + CI-workflow changes — **out of scope for the healer.** This phase
documents them as recommendations, not corpus rules.

- [ ] **T-buildconfig** — scope ruling: NOT corpus rules. Add a "Build-config
      recommendations (not healable)" section to `.docs/11_domains/rust_perf.md`
      covering: `codegen-units=1`, `lto="thin"`, `panic="abort"`, `strip=true`,
      PGO (`cargo-pgo`: instrument → run → optimize, ~10-20% on CPU-bound), BOLT
      (post-link, ~2-5% more). Source: perf-book `build-configuration.md` + Kobzol.
- [ ] **T-pgo** — fold into T-buildconfig doc section (same ruling).

## Phase 4 — Re-bench + GOAT gate (Bench 014)

After Phases 1-3 land, re-run the full Pareto validator on the expanded corpus.

- [ ] **T-rebench** — run `cargo run --example perf_bench_demo --features rust_perf_bench --release`
      on the full expanded corpus. Record results in `.benchmarks/014_rust_perf_expansion_goat.md`.
- [ ] **T-goat-g1** — all new benchable rules produce real measurements (G1).
- [ ] **T-goat-g5** — majority of new benchable rules are ParetoDominant (G5). If
      the acceptance rate is much lower than 5/11, investigate whether the corpus
      is adding noise rather than signal.
- [ ] **T-goat-g3** — default features (no `rust_perf_bench`) still pass
      `cargo test --lib` (no-regression).
- [ ] **T-doc** — update `.docs/11_domains/rust_perf.md` corpus table with new rules
      + update the category count. Update Bench 013's "References" line if needed.
- [ ] **T-clippy** — `cargo clippy --all-targets -D warnings` clean with new rules.
- [ ] **T-agents** — update `AGENTS.md` rust_perf feature description if the rule
      count changes materially (25 → ~35-40).

## What this plan does NOT do

- **Does not invent new substrate.** Pure corpus data + bench specs.
- **Does not change the validator.** The `PerfBenchValidator` + `BenchSpec`
  harness pattern (Bench 013) handles new rules unchanged.
- **Does not promote/demote existing rules.** The 25 existing rules + their
  `context_notes` are untouched.
- **Does not add build-config as corpus rules.** PGO/LTO/codegen-units are
  documented (Phase 3) but not healable — the scope ruling is explicit.
- **Does not defer GPU training.** All candidates are modelless (code patterns).

## References

- [Bench 013](../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the existing
  GOAT gate + the "3 rules distilled from perf-book" line this plan rechecks
- [Plan 001 Phase 2](001_multi_domain_framework_refactor.md) — the rust_perf
  domain scaffold
- [`.docs/11_domains/rust_perf.md`](../.docs/11_domains/rust_perf.md) — domain doc
  (corpus table + validator hierarchy)
- [nnethercote/perf-book](https://github.com/nnethercote/perf-book) — 18 chapters,
  3 distilled before this plan, 7 more distillable here
- [Mara Bos, *Rust Atomics and Locks*](https://mara.nl/atomics/) — concurrency depth
- [m-ou-se/rust-atomics-and-locks](https://github.com/m-ou-se/rust-atomics-and-locks) — book code
- [Kobzol, *PGO and BOLT*](https://kobzol.github.io/rust/cargo/2023/07/28/rust-cargo-pgo.html) — cargo-pgo
- [MS Pragmatic Rust Guidelines — Performance](https://microsoft.github.io/rust-guidelines/guidelines/performance/)
- [reintech, *Rust Performance Optimization 2026*](https://reintech.io/blog/rust-performance-optimization-complete-guide-2026)
