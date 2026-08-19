//! Seed corpus — rust performance rules distilled from the `rust-optimize`
//! skill checklist.
//!
//! Each entry is a `(rule_id, name, category, buggy_code, fixed_code, context_notes)`
//! tuple representing a real perf pattern from the skill. The corpus is
//! BLAKE3-committed via [`Corpus::new`](crate::corpus::Corpus::new). The
//! `context_notes` field is `Option<String>` — `Some(...)` for rules the bench
//! validator found ParetoDominated/Mixed/Tie in the default config (Bench 013 /
//! 038) or that are NotBenchable-with-a-metric-mismatch (Issue 014), `None` for
//! unconditional wins + honestly NotBenchable rules.
//!
//! 73 entries across 12 categories (complexity 15, allocation 18, simd 7,
//! layout 6, iterators 9, concurrency 4, io 3, arithmetic 6, dispatch 2,
//! caching 2, bevy 1, wasm 1). 64 are benchable; 10 are honestly
//! NotBenchable (8 external-runtime/size-class + 2 metric-mismatch, Issue 014).
//! Batch 48 (idle-queue item 11, 2026-08-16) added the 8 online-search
//! candidates (perf-book std-types + iterators chapters): collect-append-to-
//! extend, hashset-insert-bool-return, unwrap-or-else-lazy, filter-count-to-
//! any, vec-macro-zero-fill, iter-copied-deref-map, filter-map-fuse,
//! vec-insert-front-deque. Batch 53 (idle-queue item 11, 2026-08-17) added
//! the 4 online-search candidates: powi-vs-powf, distance-sq-no-sqrt,
//! hashmap-with-capacity, soa-hot-field. Batch 59 (2026-08-18, the first
//! EXTERNAL-source distill via the `distill` skill) added
//! division-by-invariant-multiplication (libdivide via the
//! raygon-renderer/thermite Rust port). Batch 61 (idle-queue item 11,
//! 2026-08-18) added the 2 oxc-article rules: keyword-map-to-match +
//! usize-index-to-u32. Batch 62 (2026-08-19, Research 070 microGPT-C
//! distill) added the 2 CPU-SIMD rules: transposed-weight-register-gemv +
//! newton-refinement-vs-hardware-transcendental. Batch 64 (2026-08-19,
//! Research 072 — the ARM Neon / bounds-check online-search pool) added
//! loop-order-innermost-contiguous + debug-assert-length-hoist. Batch 66
//! (2026-08-19, Research 073 — the polars kernel-idiom distill) added
//! sparse-set-bit-iteration-tz-blsr + sortedness-check-branchless-accumulate.
//! Batch 67 (2026-08-19, Research 074 — the regex-automata distill) added
//! byte-find-position-to-memchr. Batch 68 (2026-08-19, Research 075 — the
//! tantivy utility-layer distill) added varint-compose-word-single-store
//! (+ the tz-blsr xor-clear anti-shape rider note). Batch 69 (2026-08-19,
//! Research 075 §Secondary — the T6 refutation) added
//! unaligned-word-load-from-bytes.
//!
//! ## Bench specs (behind `rust_perf_bench`)
//!
//! Benchable rules carry an optional [`BenchSpec`] — a self-contained harness
//! embedding both buggy + fixed code inline + defining `make_input` /
//! `run_buggy` / `run_fixed` entry points. The bench validator generates a
//! temp bin crate from the spec + runs a timing loop at multiple sizes.
//!
//! Currently benchable (64/74):
//! - `o1-lookup` (Vec scan vs HashMap lookup — clear O(n) vs O(1))
//! - `bsearch-sorted-slice` (linear scan vs binary search — O(n) vs O(log n) on sorted data)
//! - `dedupe-hashset-vs-vec` (Vec::contains loop vs HashSet collect — O(n²) vs O(n))
//! - `entry-vs-contains-insert` (contains_key+insert vs entry API — merge HashMap lookups)
//! - `loop-fusion` (two iterator passes vs one fused loop — cache locality, merge loops)
//! - `prefix-sum-2d` (O(n) region scan vs O(1) prefix sum — boundary-vs-volume, grows with N)
//! - `swap-remove` (Vec::remove O(n) shift vs Vec::swap_remove O(1) swap — grows with N)
//! - `sort-unstable` (stable sort allocates vs sort_unstable in-place — no allocation + faster algo)
//! - `sort-by-key` (sort_by repeated closure vs sort_by_key single extraction — N vs N·log N)
//! - `vec-remove-front-to-vecdeque` (Vec::remove(0) O(n²) drain vs VecDeque::pop_front O(n) — grows with N²)
//! - `btreemap-sorted-iteration` (HashMap + sort per query vs BTreeMap natural order — O(n log n) vs O(n) per query)
//! - `static-str` (String alloc vs &'static str — allocation cost)
//! - `simd-chunk` (scalar vs chunked loop — vectorization at large N)
//! - `branchless-conditional` (if vs bool-as-usize multiply — vectorization + branch prediction)
//! - `bool-to-bitpack` (Vec<bool> count vs packed Vec<u64> POPCNT — data density)
//! - `chunks-exact` (chunks variable-last vs chunks_exact uniform — enables vectorization)
//! - `copy-from-slice` (index-loop copy vs copy_from_slice — SIMD + no bounds check)
//! - `mutex-to-atomic` (Mutex<u32> vs AtomicU32 — lock/unlock overhead)
//! - `f32-counter-to-int` (f32 accumulator vs u64 — serial dep chain)
//! - `collect-avoidance` (.collect() intermediate vs chained iterator — allocation + double pass)
//! - `dyn-to-enum` (Box<dyn Trait> vs enum — vtable indirection + no inlining)
//! - `ok-or-else` (ok_or(expensive()) eager vs ok_or_else(closure) lazy — avoids unused error alloc)
//! - `precompute-table` (recompute vs const lookup table — **context-dependent**,
//!   see `context_notes`; not a release-mode win per Bench 013)
//! - `papaya-lockfree` (RwLock vs papaya lock-free — **context-dependent**,
//!   see `context_notes`; only a win under multi-threaded contention)
//! - `vec-with-capacity` (Vec::new() growth vs Vec::with_capacity — realloc cost)
//! - `format-concat-loop` (format! concat in loop vs push_str — O(n²) vs O(n))
//! - `fxhashmap` (std HashMap SipHash vs FxHashMap FxHash — hasher speed, 2-4×)
//! - `retain-inplace` (filter().collect() new Vec vs retain() in-place — ~1.5-2×)
//! - `mem-take` (clone+clear vs mem::take move — ~3×, avoids deep clone)
//! - `avoid-clone` (fn(data: Vec<T>) clone → fn(data: &[T]) borrow — ~1.3-3×)
//! - `box-str-immutable` (Vec<String> 3-word → Vec<Box<str>> 2-word — ~1.1-1.3×)
//! - `reuse-allocation-clear` (alloc-per-call → clear-reuse scratch buffer — **Mixed**, see `context_notes`)
//! - `box-avoid` (Vec<Box<i64>> heap per elem → Vec<i64> stack values — ~10-35×)
//! - `smallvec-stack` (Vec<T> always heap → stack array for ≤8 typical — ~7-12×)
//! - `buffered-io` (File::read byte-by-byte → BufReader 8KB batched — ~97×)
//! - `bufwriter-batched` (File::write per-record → BufWriter batched — syscall amortization)
//! - `log-write-direct` (push_str(&format!(...)) temp alloc → write! direct — ~1.7×)
//! - `cow-conditional-alloc` (String always-alloc → Cow<str> borrow when no transform — N vs 0.1N allocations)
//! - `regex-hoist-lazylock` (Regex::new per call → static LazyLock — compile once, Issue 014 T2.1)
//! - `arc-str-payload` (per-recipient payload clone → Arc<str> handle clone — N×bytes → 1 alloc, Issue 014 T2.2)
//! - `collect-append-to-extend` (collect+append staging → extend — no intermediate Vec, Batch 48)
//! - `hashset-insert-bool-return` (contains+insert double lookup → insert's bool return — Batch 48)
//! - `unwrap-or-else-lazy` (unwrap_or(eager) → unwrap_or_else(closure) — Batch 48)
//! - `filter-count-to-any` (filter(p).count() > 0 full scan → any(p) short-circuit — Batch 48)
//! - `vec-macro-zero-fill` (Vec::new+resize memset → vec![0; n] calloc — Batch 48)
//! - `iter-copied-deref-map` (.map(|x| *x) → .copied() TrustedLen — Batch 48)
//! - `filter-map-fuse` (filter(p).map(f) → filter_map fused — Batch 48)
//! - `vec-insert-front-deque` (Vec::insert(0,_) O(n²) → VecDeque::push_front — Batch 48)
//! - `powi-vs-powf` (x.powf(2.0) libm call → x.powi(2) multiply — Batch 53)
//! - `distance-sq-no-sqrt` (sqrt-then-compare → squared compare — Batch 53)
//! - `hashmap-with-capacity` (HashMap::new() rehash ladder → with_capacity(n) — Batch 53)
//! - `soa-hot-field` (AoS strided single-field scan → SoA contiguous column — Batch 53)
//! - `division-by-invariant-multiplication` (hardware integer div by loop-invariant RUNTIME d → libdivide-style magic mulhi+shift — Batch 59, thermite distill)
//! - `keyword-map-to-match` (static-keyword HashMap dispatch → plain match on literals — LLVM length-switch, Batch 61)
//! - `usize-index-to-u32` (16B usize spans → 8B u32 spans — cache density under bandwidth-bound scans, Batch 61)
//! - `transposed-weight-register-gemv` (row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA into output accumulators — 1.18-6.08x ParetoDominant, Batch 62, microGPT-C distill; the win GROWS at large nin — the scalar accumulator's serial FP chain is the real cost, not the horizontal reduce)
//! - `newton-refinement-vs-hardware-transcendental` (Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction `fdiv`/`fsqrt` on CPU — Mixed 1.01-1.09x M3 / ParetoDominant 1.64-1.66x x86 (re-gated), hardware never loses; Batch 62, annotated)
//! - `loop-order-innermost-contiguous` (i-j-k nested accumulation with a column-striding inner access → i-k-j reorder so the innermost varies the LAST index — 7.1-13.8x ParetoDominant, grows with n; Batch 64)
//! - `debug-assert-length-hoist` (`debug_assert!` mentioning `.len()` is erased in release, dropping the constraint bounds-check elision needed → `assert!` — Mixed regime-split 1.13x@1K / 0.67-0.71x@100K stable on aarch64 (x86 re-gate: 1.00x/0.92x, holds cross-arch); Batch 64, annotated)
//! - `sparse-set-bit-iteration-tz-blsr` (64-iteration set-bit scan → trailing_zeros+BLSR, popcount iterations — sparse-mask regime, Batch 66)
//! - `sortedness-check-branchless-accumulate` (early-exit sortedness validation → branchless `sorted &=` full scan, vectorizes — usually-sorted regime, Batch 66)
//! - `byte-find-position-to-memchr` (u8 slice iter().position byte scan → memchr::memchr SIMD block scan — position is NOT specialized in std, contains is; dep-gated, Batch 67)
//! - `varint-compose-word-single-store` (per-byte write_all varint loop → 5-threshold ladder + single write_all — MEASURED ParetoDominated on aarch64 AND x86 across writer regimes (the 4090-box re-gate: 0.40-0.50x there, HARDER): the honest-loser annotation stops the tantivy folk port; Batch 68)
//! - `unaligned-word-load-from-bytes` (shift/or byte-assembly word read → one unaligned from_le_bytes load — ParetoDominant cross-arch: u64 1.4-2.2x, u32 1.2-1.4x, both arches; the T6 defer refuted; Batch 69)
//!
//! Not benchable (10/74) — honest:
//! - `layout-reorder` (struct size — deterministic `size_of`, not a runtime
//!   microbench; the cache-utilization claim needs a specific access pattern)
//! - `repr-u8-enum` (enum size — deterministic `size_of`, same class as
//!   `layout-reorder`)
//! - `box-large-enum-variant` (enum size — deterministic `size_of`, shrinks
//!   the common case by boxing the rare large variant)
//! - `shrink-to-fit-after-build` (memory win, not speed — releases excess
//!   Vec capacity after incremental construction)
//! - `async-yield-point` (fairness/latency-tail win, not throughput)
//! - `async-future-size` (future state-machine size — deterministic `size_of_val`)
//! - `bevy-reactive-loop` (needs the Bevy runtime — can't run in a temp crate)
//! - `wasm-batch-ffi` (needs a WASM runtime — can't run in a temp crate)
//! - `tokio-guard-across-await` (**metric mismatch**, Issue 014 T2.3 — cost is
//!   OTHER tasks' wait latency under contention, not this task's wall-clock)
//! - `bounded-channel-backpressure` (**metric mismatch + semantics-changing**,
//!   Issue 014 T2.4 — win is bounded queue depth; the buggy path is often
//!   FASTER on wall-clock precisely because it buffers unboundedly)
//!
//! ## Context-dependent rules (`context_notes`)
//!
//! Twelve rules carry a `context_notes: Some(...)` annotation. Eight record that
//! the Pareto validator found them **ParetoDominated** or **Mixed** in the
//! default configuration: the original 4 from Bench 013 (`precompute-table`,
//! `papaya-lockfree`, `dedupe-hashset-vs-vec`, `reuse-allocation-clear`) + 2
//! from Bench 016 / Issue 012 (`sort-by-key`, `copy-from-slice`) + 2 from
//! Bench 48 (`filter-count-to-any` — dominated at mid-position hits by the
//! VECTORIZED count loop, `iter-copied-deref-map` — Mixed, 0.80× at 1M on
//! sum folds) + 2 from Batch 53 / Bench 038 (`powi-vs-powf` all-Tie,
//! `soa-hot-field` latency-bound Mixed). Two more (Issue 014 T2.3/T2.4)
//! annotate the NotBenchable metric-mismatch class: `tokio-guard-across-await` +
//! `bounded-channel-backpressure`. The patterns are genuine wins in their
//! respective regimes and are kept in the corpus as documented patterns, but
//! consumers should treat them as context-dependent — not unconditional
//! suggestions.

use crate::corpus::Corpus;
use crate::domains::rust_perf::{PerfRule, RustPerfDomain};

/// Build a seed corpus from common rust performance rules.
///
/// Returns a BLAKE3-committed [`Corpus<RustPerfDomain>`] with 69 entries
/// covering the main perf pattern categories (complexity, allocation, simd,
/// layout, iterators, concurrency, io, arithmetic, dispatch, caching, bevy,
/// wasm). Allocation is the deepest (18 rules); complexity has 15; iterators
/// has 7; simd has 7 (Batch 62 added 1 + Batch 64 added 1); layout has 6
/// (Batch 61 added 1); concurrency has 4;
/// dispatch has 2 (Batch 61 added 1); caching has 2; the rest have 1.
#[must_use]
#[allow(clippy::too_many_lines)] // seed corpus is a data table; length scales with rule count
pub fn build_perf_seed_corpus() -> Corpus<RustPerfDomain> {
    Corpus::new(vec![
        // ── complexity: O(n) scan → O(1) lookup ────────────────────────────
        PerfRule {
            rule_id: "o1-lookup".to_string(),
            name: "O(n) scan → O(1) lookup".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn find_user(users: &Vec<(u32, String)>, id: u32) -> Option<&String> {\n    users.iter().find(|(uid, _)| *uid == id).map(|(_, n)| n)\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn find_user(users: &HashMap<u32, String>, id: u32) -> Option<&String> {\n    users.get(&id)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(o1_lookup_bench_spec()),
        },
        // ── complexity: O(n) scan on sorted slice → O(log n) binary search ─
        // The compiler cannot transform a linear scan into a binary search;
        // this is a genuine algorithmic improvement on pre-sorted data.
        PerfRule {
            rule_id: "bsearch-sorted-slice".to_string(),
            name: "O(n) .contains() on sorted slice → O(log n) binary_search()".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn contains(sorted: &[u32], target: u32) -> bool {\n    sorted.iter().any(|&x| x == target)\n}".to_string(),
            fixed_code: "fn contains(sorted: &[u32], target: u32) -> bool {\n    sorted.binary_search(&target).is_ok()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bsearch_sorted_slice_bench_spec()),
        },
        // ── complexity: O(n²) dedup via Vec::contains → O(n) HashSet ───────
        // Building a unique Vec by repeated linear membership checks is O(n²).
        // Collecting into a HashSet then draining is O(n) average. The
        // compiler cannot optimize a growing-Vec membership loop into a hash
        // table — this is a pure algorithmic improvement.
        PerfRule {
            rule_id: "dedupe-hashset-vs-vec".to_string(),
            name: "O(n²) Vec::contains dedup loop → O(n) HashSet collect".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn unique(items: &[u32]) -> Vec<u32> {\n    let mut seen = Vec::new();\n    for &x in items {\n        if !seen.contains(&x) { seen.push(x); }\n    }\n    seen\n}".to_string(),
            fixed_code: "use std::collections::HashSet;\nfn unique(items: &[u32]) -> Vec<u32> {\n    let seen: HashSet<u32> = items.iter().copied().collect();\n    seen.into_iter().collect()\n}".to_string(),
            // Bench 013 Mixed (2/4 Faster, 2/4 Slower). HashSet allocation +
            // hashing overhead exceeds the O(n²) cost at small N (≤500).
            // Wins clearly at N ≥ 1000 (up to 7.5× at N=5000). The rule is a
            // genuine algorithmic win for collections known to exceed ~500
            // elements; for small dedup the Vec::contains approach is fine.
            context_notes: Some(
                "Bench 013: Mixed (2/4 Faster, 2/4 Slower). O(n²) cost is \
                 trivial at small N (≤500), so HashSet allocation + hashing \
                 overhead dominates. Wins at N ≥ 1000 (1.5× to 7.5×). Suggest \
                 for dedup on collections known to exceed ~500 elements; for \
                 small dedup, the Vec::contains approach is fine."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(dedupe_hashset_vs_vec_bench_spec()),
        },
        // ── allocation: String → &'static str ─────────────────────────────
        PerfRule {
            rule_id: "static-str".to_string(),
            name: "String → &'static str for constant labels".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn label() -> String { \"ready\".to_string() }".to_string(),
            fixed_code: "fn label() -> &'static str { \"ready\" }".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(static_str_bench_spec()),
        },
        // ── layout: field reorder to eliminate padding ────────────────────
        PerfRule {
            rule_id: "layout-reorder".to_string(),
            name: "Reorder fields to eliminate padding (u8 → u64 → u32 ordering fix)".to_string(),
            category: "layout".to_string(),
            // The bad layout has a u8 between two u64s, forcing 7 bytes of
            // padding after the u8. Reordering u64 → u32 → u8 packs to 16 bytes.
            buggy_code: "#[repr(C)]\nstruct Bad { a: u8, b: u64, c: u32 }".to_string(),
            fixed_code: "#[repr(C)]\nstruct Good { b: u64, c: u32, a: u8 }".to_string(),
            context_notes: None,
            // Not benchable: size_of is deterministic (compile-time), and the
            // cache-utilization claim needs a specific access pattern to show
            // up in a microbench. Honest "NotBenchable".
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: Arc<RwLock<HashMap>> → papaya ────────────────────
        // Per the user AGENTS.md rule: "Use papaya as possible for lock-free
        // Arc<RwLock<HashMap>>". Context-dependent: only a win under
        // multi-threaded contention (Bench 013 measured single-threaded reads
        // as 0.55–0.79× — papaya's epoch GC overhead dominates without
        // concurrent access). The AGENTS.md rule is correct for hot
        // concurrent paths; the annotation is for the single-threaded case.
        PerfRule {
            rule_id: "papaya-lockfree".to_string(),
            name: "Arc<RwLock<HashMap>> → papaya lock-free map".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::{Arc, RwLock};\nuse std::collections::HashMap;\nstruct Store(Arc<RwLock<HashMap<u32, String>>>);".to_string(),
            fixed_code: "use papaya::HashMap;\nstruct Store(HashMap<u32, String>);".to_string(),
            // Bench 013 ParetoDominated (0.55–0.79× single-threaded). The
            // win is real under contention; the annotation marks the
            // default-config (single-threaded bench) as NOT a win.
            context_notes: Some(
                "Bench 013: ParetoDominated 0.55-0.79x single-threaded. \
                 Win only under multi-threaded contention (papaya epoch GC \
                 overhead dominates without concurrent readers). The AGENTS.md \
                 rule (papaya for Arc<RwLock<HashMap>>) is correct for hot \
                 concurrent paths; do not suggest for single-threaded reads."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(papaya_lockfree_bench_spec()),
        },
        // ── concurrency: Mutex<u32> counter → AtomicU32 ─────────────────
        // Per the rust-optimize skill item 6: "Mutex<u32> → AtomicU32".
        // The Mutex lock/unlock path involves an atomic CAS to acquire + an
        // atomic store to release (plus the MutexGuard RAII lifecycle),
        // even on the uncontended path. AtomicU32::fetch_add is a single
        // `lock xadd` instruction. The compiler cannot eliminate Mutex::lock
        // (it's an opaque call). Bench 013: ParetoDominant ~2.5× across all
        // sizes (1K → 1M) single-threaded — no contention needed.
        PerfRule {
            rule_id: "mutex-to-atomic".to_string(),
            name: "Mutex<u32> counter → AtomicU32 fetch_add".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::Mutex;\nstruct Counter(Mutex<u32>);\nimpl Counter {\n    fn new() -> Self { Self(Mutex::new(0)) }\n    fn inc(&self) { *self.0.lock().unwrap() += 1; }\n}".to_string(),
            fixed_code: "use std::sync::atomic::{AtomicU32, Ordering};\nstruct Counter(AtomicU32);\nimpl Counter {\n    fn new() -> Self { Self(AtomicU32::new(0)) }\n    fn inc(&self) { self.0.fetch_add(1, Ordering::Relaxed); }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(mutex_to_atomic_bench_spec()),
        },
        // ── simd: chunked loop for auto-vectorization ─────────────────────
        PerfRule {
            rule_id: "simd-chunk".to_string(),
            name: "Chunked loop (4-wide) to help LLVM auto-vectorize".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn sum(v: &[f32]) -> f32 {\n    let mut s = 0.0;\n    for i in 0..v.len() { s += v[i]; }\n    s\n}".to_string(),
            fixed_code: "fn sum(v: &[f32]) -> f32 {\n    let chunks = v.chunks_exact(4);\n    let rem = v.chunks_exact(4).remainder();\n    let mut acc = [0.0f32; 4];\n    for c in chunks {\n        acc[0] += c[0]; acc[1] += c[1]; acc[2] += c[2]; acc[3] += c[3];\n    }\n    acc.iter().chain(rem).copied().sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(simd_chunk_bench_spec()),
        },
        // ── simd: bool array → packed bitvec for POPCNT acceleration ────────
        // Data density: Vec<bool> uses 1 byte per bool; packed Vec<u64> uses
        // 1 bit. For counting set bits, the fixed path runs POPCNT on each
        // 64-bit word (1 instruction per 64 bools) vs the buggy path's
        // per-byte comparison. The compiler cannot auto-pack bools into bits.
        PerfRule {
            rule_id: "bool-to-bitpack".to_string(),
            name: "Vec<bool> set-bit count → packed Vec<u64> + count_ones()".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn count_true(flags: &[bool]) -> usize {\n    flags.iter().filter(|&&b| b).count()\n}".to_string(),
            fixed_code: "fn count_true(bits: &[u64]) -> usize {\n    bits.iter().map(|w| w.count_ones() as usize).sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bool_to_bitpack_bench_spec()),
        },
        // ── arithmetic: f32 accumulator → u64 accumulator ───────────────────
        // Per the rust-optimize skill item 4: "f32 counters → u32/u64".
        // f32 addition is NOT associative (IEEE 754 rounding), so the
        // compiler CANNOT break the sequential dependency chain or use
        // multiple SIMD accumulators without -C fast-math (which Rust does
        // not enable by default). Each f32 add has 3–4 cycle latency, fully
        // serial. u64 addition IS associative, so the compiler vectorizes
        // (multiple independent accumulators) + the add latency is 1 cycle.
        // Bench 013: ParetoDominant 7.5× → 20.5× (grows with N — the f32
        // dependency chain bottleneck worsens relative to vectorized u64).
        PerfRule {
            rule_id: "f32-counter-to-int".to_string(),
            name: "f32 accumulator for integer tally → u64 accumulator".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn tally(scores: &[u32]) -> f32 {\n    let mut total = 0.0f32;\n    for &s in scores { total += s as f32; }\n    total\n}".to_string(),
            fixed_code: "fn tally(scores: &[u32]) -> u64 {\n    let mut total = 0u64;\n    for &s in scores { total += s as u64; }\n    total\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(f32_counter_to_int_bench_spec()),
        },
        // ── caching: pre-compute lookup table ─────────────────────────────
        // Context-dependent: the compiler optimizes the runtime popcount loop
        // to a single POPCNT instruction in release mode, making the lookup
        // table slower (L1 cache latency > POPCNT latency). Bench 013
        // measured 0.20–0.40× — the rule is NOT a release-mode win on modern
        // CPUs. Kept in the corpus as a documented pattern (valid for targets
        // without POPCNT or in debug mode); the context_notes annotation marks
        // it as NOT a default suggestion.
        PerfRule {
            rule_id: "precompute-table".to_string(),
            name: "Pre-compute lookup table once, reuse across calls".to_string(),
            category: "caching".to_string(),
            buggy_code: "fn parity_byte(b: u8) -> u8 {\n    (0..8).filter(|i| b & (1 << i) != 0).count() as u8 & 1\n}".to_string(),
            fixed_code: "const PARITY: [u8; 256] = {\n    let mut t = [0u8; 256];\n    let mut i = 0;\n    while i < 256 { t[i] = (i.count_ones() & 1) as u8; i += 1; }\n    t\n};\nfn parity_byte(b: u8) -> u8 { PARITY[b as usize] }".to_string(),
            // Bench 013 ParetoDominated (0.20–0.40× release). The compiler
            // already lowers the buggy form to POPCNT; the table is slower.
            context_notes: Some(
                "Bench 013: ParetoDominated 0.20-0.40x release mode. The \
                 compiler lowers (0..8).filter().count() to a single POPCNT \
                 instruction; the lookup table is slower (L1 cache latency \
                 > POPCNT latency). NOT a default suggestion — only valid for \
                 targets without POPCNT or in debug mode."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(precompute_table_bench_spec()),
        },
        // ── bevy: reactive_low_power event loop ───────────────────────────
        // Per the rust-optimize skill: default Continuous pegs CPU at 100%
        // even idle; switch to reactive_low_power matching tick rate.
        PerfRule {
            rule_id: "bevy-reactive-loop".to_string(),
            name: "Bevy Continuous → reactive_low_power event loop".to_string(),
            category: "bevy".to_string(),
            buggy_code: "use bevy::winit::WinitSettings;\nfn setup() -> WinitSettings { WinitSettings::default() }".to_string(),
            fixed_code: "use bevy::winit::{UpdateMode, WinitSettings};\nuse std::time::Duration;\nfn setup() -> WinitSettings {\n    WinitSettings {\n        focused_mode: UpdateMode::reactive_low_power(Duration::from_millis(50)),\n        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(1)),\n    }\n}".to_string(),
            context_notes: None,
            // Not benchable: needs the Bevy runtime (event loop + winit).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── wasm: batch API to amortize FFI floor ─────────────────────────
        // Per the rust-optimize skill: ~250ns FFI floor amortized across N
        // calls via a single batch invocation.
        PerfRule {
            rule_id: "wasm-batch-ffi".to_string(),
            name: "Batch N×M FFI calls into one (amortize ~250ns floor)".to_string(),
            category: "wasm".to_string(),
            buggy_code: "// Per-call FFI: 250ns floor × N×M invocations\nfn validate_all(items: &[u32], rules: &[u32]) -> Vec<bool> {\n    items.iter().map(|i| rules.iter().map(|r| validate_one(*i, *r)).all(|b| b)).collect()\n}\nfn validate_one(_i: u32, _r: u32) -> bool { true }".to_string(),
            fixed_code: "// Batch: one FFI call carries shared state + per-item arrays\nfn validate_all(items: &[u32], rules: &[u32]) -> Vec<bool> {\n    let mut out = vec![false; items.len()];\n    validate_batch(items, rules, &mut out);\n    out\n}\nfn validate_batch(_items: &[u32], _rules: &[u32], _out: &mut [bool]) {\n    // single FFI call: shared rules + per-item arrays → out\n}".to_string(),
            context_notes: None,
            // Not benchable: needs a WASM runtime (js-sys / wasm-bindgen FFI).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── allocation: Vec::new() + push → Vec::with_capacity ───────────
        // Per the rust-optimize skill: "Pre-allocate output arrays upfront,
        // write in-place instead of collecting per-iteration". The Vec
        // growth strategy causes O(log n) reallocations + O(n) total copies;
        // with_capacity does 1 allocation + 0 extra copies.
        PerfRule {
            rule_id: "vec-with-capacity".to_string(),
            name: "Vec::new() + push loop → Vec::with_capacity(n)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v\n}".to_string(),
            fixed_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::with_capacity(n);\n    for i in 0..n { v.push(i as u32); }\n    v\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_with_capacity_bench_spec()),
        },
        // ── allocation: format!("{s}{x}") in loop → push_str ─────────────
        // Classic O(n²) anti-pattern: format! creates a new String each
        // iteration, copying the entire accumulated string. push_str
        // amortizes to O(n). At 1000 chunks of 10 bytes, the buggy path
        // copies ~5MB total vs ~10KB for the fixed path.
        PerfRule {
            rule_id: "format-concat-loop".to_string(),
            name: "format!(\"{s}{x}\") concat in loop → String::push_str".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(chunks: &[&str]) -> String {\n    let mut s = String::new();\n    for chunk in chunks { s = format!(\"{s}{chunk}\"); }\n    s\n}".to_string(),
            fixed_code: "fn build(chunks: &[&str]) -> String {\n    let total: usize = chunks.iter().map(|c| c.len()).sum();\n    let mut s = String::with_capacity(total);\n    for chunk in chunks { s.push_str(chunk); }\n    s\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(format_concat_loop_bench_spec()),
        },
        // ── layout: field-less enum without repr → #[repr(u8)] ───────────
        // Per the rust-optimize skill: "Use #[repr(u8)] on field-less enums
        // to guarantee 1-byte size". Without repr, the compiler picks the
        // default (often 4+ bytes depending on variant count + niche
        // optimization). Not benchable: size_of is compile-time; the runtime
        // impact (memory bandwidth, cache utilization) needs a specific
        // access pattern to show up in a microbench.
        PerfRule {
            rule_id: "repr-u8-enum".to_string(),
            name: "Field-less enum → #[repr(u8)] for guaranteed 1-byte size".to_string(),
            category: "layout".to_string(),
            buggy_code: "enum Color { Red, Green, Blue }".to_string(),
            fixed_code: "#[repr(u8)]\nenum Color { Red, Green, Blue }".to_string(),
            context_notes: None,
            // Not benchable: size_of is deterministic (compile-time), and the
            // cache/bandwidth claim needs a specific access pattern (array of
            // enums). Honest "NotBenchable", same class as layout-reorder.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── iterators: unnecessary .collect() intermediate → chained iterator ──
        // Per the rust-optimize skill item 5: "fix double chunks_exact()".
        // The general pattern: collecting an intermediate Vec then iterating
        // it again. The collect allocates (heap + copy) + forces a second
        // pass. The fixed path chains the iterators — zero allocation, single
        // pass. The compiler cannot eliminate a .collect() (it must produce a
        // concrete Vec). Bench 013: ParetoDominant 2.4–3.0× across all sizes
        // (1K → 1M).
        PerfRule {
            rule_id: "collect-avoidance".to_string(),
            name: "Unnecessary .collect() intermediate → chained iterator".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn transform_sum(vals: &[u64]) -> u64 {\n    let mapped: Vec<u64> = vals.iter().map(|x| x * 2).collect();\n    mapped.iter().sum()\n}".to_string(),
            fixed_code: "fn transform_sum(vals: &[u64]) -> u64 {\n    vals.iter().map(|x| x * 2).sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(collect_avoidance_bench_spec()),
        },
        // ── dispatch: Box<dyn Trait> → enum dispatch (devirtualize) ──────
        // NOT in the rust-optimize skill checklist — a genuine gap. The buggy
        // path uses dynamic dispatch via Box<dyn Trait>: each call is a vtable
        // load + indirect call (branch misprediction risk, no inlining). The
        // fixed path uses an enum: the match arms are direct calls the
        // compiler can inline + branch-predict. The compiler cannot devirtualize
        // through an opaque trait object. Bench 013: ParetoDominant ~2.0×
        // across all sizes (1K → 100K).
        PerfRule {
            rule_id: "dyn-to-enum".to_string(),
            name: "Box<dyn Trait> dispatch → enum dispatch (devirtualize)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "trait Op { fn apply(&self, x: f64) -> f64; }\nfn run(ops: &[Box<dyn Op>], vals: &[f64]) -> f64 {\n    let mut acc = 0.0;\n    for &x in vals { let mut v = x; for op in ops { v = op.apply(v); } acc += v; }\n    acc\n}".to_string(),
            fixed_code: "enum Op { Scale(f64), Offset(f64), Square }\nimpl Op {\n    fn apply(&self, x: f64) -> f64 {\n        match self { Op::Scale(s) => x * s, Op::Offset(o) => x + o, Op::Square => x * x }\n    }\n}\nfn run(ops: &[Op], vals: &[f64]) -> f64 {\n    let mut acc = 0.0;\n    for &x in vals { let mut v = x; for op in ops { v = op.apply(v); } acc += v; }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(dyn_to_enum_bench_spec()),
        },
        // ── complexity: contains_key() + insert() → entry().or_insert() ──
        // Per the rust-optimize skill item 1: "merge HashMap lookups".
        // The buggy path does TWO hash lookups: contains_key hashes + probes,
        // then insert hashes + probes again. The compiler cannot merge them
        // because insert may need to grow the table (opaque call). The fixed
        // path uses the entry() API: ONE hash + probe, in-place insert if
        // absent. Bench 013: ParetoDominant 2.8–3.4× across all sizes (100 →
        // 100K). The win is the eliminated redundant hash + probe — a
        // genuine constant-factor improvement, not algorithmic.
        PerfRule {
            rule_id: "entry-vs-contains-insert".to_string(),
            name: "contains_key() + insert() → entry().or_insert() (merge HashMap lookups)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashMap;\nfn insert_if_absent(map: &mut HashMap<u32, u32>, k: u32, v: u32) {\n    if !map.contains_key(&k) {\n        map.insert(k, v);\n    }\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn insert_if_absent(map: &mut HashMap<u32, u32>, k: u32, v: u32) {\n    map.entry(k).or_insert(v);\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(entry_vs_contains_insert_bench_spec()),
        },
        // ── simd: data-dependent if → branchless arithmetic for vectorization ──
        // Per the rust-optimize skill item 7: "branch-free inner loops (use
        // bool as usize instead of if)". The buggy path uses `if cond[i] { ... }`
        // inside a hot loop — the data-dependent branch prevents LLVM from
        // auto-vectorizing (each iteration may or may not execute the body).
        // The fixed path uses `val * cond[i] as u64` — a branchless multiply
        // that LLVM vectorizes into SIMD lanes (e.g., 4-wide or 8-wide).
        // Bench 013: ParetoDominant 6.8–13× across all conditions tested
        // (unpredictable 50% random → predictable 99%/1%/100%). The win is
        // primarily auto-vectorization, NOT branch prediction — even with
        // 100% predictable conditions, the branchless version is ~10× faster
        // because LLVM vectorizes the multiply loop but cannot vectorize the
        // branchy loop.
        PerfRule {
            rule_id: "branchless-conditional".to_string(),
            name: "Data-dependent if in hot loop → branchless arithmetic (bool as usize)".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {\n    let mut sum: u64 = 0;\n    for i in 0..data.len() {\n        if cond[i] {\n            sum += data[i] as u64;\n        }\n    }\n    sum\n}".to_string(),
            fixed_code: "fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {\n    let mut sum: u64 = 0;\n    for i in 0..data.len() {\n        sum += data[i] as u64 * cond[i] as u64;\n    }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(branchless_conditional_bench_spec()),
        },
        // ── complexity: two passes over same data → one fused pass ──────────
        // Per the rust-optimize skill item 1: "merge loops". The buggy path
        // iterates the data twice: once for `.sum()`, once for `.max()`. Each
        // iterator consumer is a separate trait method call that independently
        // consumes the iterator — the compiler CANNOT fuse them into a single
        // pass (loop fusion is not a standard LLVM optimization pass; it would
        // need to prove aliasing + ordering constraints across two calls).
        // The fixed path does both reductions in one explicit loop — one
        // cache pass, one set of loads. Bench 013: ParetoDominant 1.46–1.52×
        // across all sizes (1K → 1M). The win is cache locality: the data is
        // hot in L1 for the second reduction in the fused loop but cold for
        // the second pass in the buggy version (at large N exceeding L1/L2).
        // This is the complement of `entry-vs-contains-insert` (which merges
        // HashMap lookups in the same iteration) — loop fusion merges
        // iterations over the same collection.
        PerfRule {
            rule_id: "loop-fusion".to_string(),
            name: "Two iterator passes over same data → one fused for loop (merge loops)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn sum_max(data: &[u64]) -> (u64, u64) {\n    let sum: u64 = data.iter().copied().sum();\n    let max: u64 = data.iter().copied().max().unwrap_or(0);\n    (sum, max)\n}".to_string(),
            fixed_code: "fn sum_max(data: &[u64]) -> (u64, u64) {\n    let mut sum = 0u64;\n    let mut max = 0u64;\n    for &x in data {\n        sum += x;\n        if x > max { max = x; }\n    }\n    (sum, max)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(loop_fusion_bench_spec()),
        },
        // ── complexity: O(n) region scan → O(1) prefix sum (boundary-vs-volume) ──
        // Per the rust-optimize skill item 1: "boundary-vs-volume" (Stokes /
        // divergence theorem). The discrete instantiation is the 2D prefix sum
        // (a.k.a. summed-area table / integral image): instead of scanning all
        // O(n) interior cells per region-sum query, pre-compute the cumulative
        // sum once (O(n)), then answer each query in O(1) via 4 corner lookups.
        // The prefix array encodes the "boundary" (cumulative sum at the
        // corners); the region mass is reconstructed from boundary values
        // alone — the discrete form of the Stokes theorem.
        //
        // This is distinct from `o1-lookup` (point query via HashMap): prefix
        // sum handles aggregate/range queries via mathematical pre-computation.
        // The compiler cannot derive a prefix-sum transform from a nested-loop
        // scan — it requires the programmer to recognize the pattern.
        //
        // Bench 013: ParetoDominant 30× → 26,000× (grows linearly with N — the
        // scan is O(K * region_area) per K queries while the prefix is O(K)
        // after a one-time O(n) build). Valid for d ≤ 3 (2D maps, 3D belief
        // regions); for d ≥ 8 (HLA state) the prefix array exceeds the interior.
        PerfRule {
            rule_id: "prefix-sum-2d".to_string(),
            name: "O(n) 2D region-sum scan per query → O(1) prefix sum (boundary-vs-volume)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn region_sum(grid: &Vec<Vec<i64>>, x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {\n    let mut sum = 0;\n    for x in x0..x1 {\n        for y in y0..y1 {\n            sum += grid[x][y];\n        }\n    }\n    sum\n}".to_string(),
            fixed_code: "// Pre-compute once: prefix[x][y] = sum of grid[0..x][0..y]\nfn build_prefix(grid: &[Vec<i64>]) -> Vec<Vec<i64>> {\n    let w = grid.len();\n    let h = grid.first().map(|r| r.len()).unwrap_or(0);\n    let mut p = vec![vec![0i64; h + 1]; w + 1];\n    for x in 0..w {\n        for y in 0..h {\n            p[x + 1][y + 1] = grid[x][y] + p[x][y + 1] + p[x + 1][y] - p[x][y];\n        }\n    }\n    p\n}\nfn region_sum(prefix: &[Vec<i64>], x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {\n    prefix[x1][y1] - prefix[x0][y1] - prefix[x1][y0] + prefix[x0][y0]\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(prefix_sum_2d_bench_spec()),
        },
        // ── allocation: ok_or(expensive()) → ok_or_else(closure) ─────────
        // Source: nnethercote/perf-book "Standard Library Types" chapter.
        // The eager form always constructs the error value; the lazy form
        // only constructs it when the Option is None. ~97× when the error
        // construction involves a String allocation.
        PerfRule {
            rule_id: "ok-or-else".to_string(),
            name: "ok_or(expensive()) → ok_or_else(|| expensive()) (lazy error eval)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn get_value(o: Option<u32>) -> Result<u32, String> {\n    o.ok_or(format!(\"error: missing value at {}\", std::time::SystemTime::now().elapsed().unwrap().as_nanos()))\n}".to_string(),
            fixed_code: "fn get_value(o: Option<u32>) -> Result<u32, String> {\n    o.ok_or_else(|| format!(\"error: missing value at {}\", std::time::SystemTime::now().elapsed().unwrap().as_nanos()))\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(ok_or_else_bench_spec()),
        },
        // ── complexity: Vec::remove (O(n) shift) → Vec::swap_remove (O(1)) ─
        // Source: nnethercote/perf-book "Standard Library Types" chapter.
        // Vec::remove shifts all elements after the index; swap_remove moves
        // the last element into the gap. O(n)→O(1) when order doesn't matter.
        PerfRule {
            rule_id: "swap-remove".to_string(),
            name: "Vec::remove(i) O(n) shift → Vec::swap_remove(i) O(1) swap".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn remove_first_n(data: &mut Vec<u32>, n: usize) {\n    for _ in 0..n.min(data.len()) {\n        data.remove(0);\n    }\n}".to_string(),
            fixed_code: "fn remove_first_n(data: &mut Vec<u32>, n: usize) {\n    for _ in 0..n.min(data.len()) {\n        data.swap_remove(0);\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(swap_remove_bench_spec()),
        },
        // ── simd: chunks (variable last) → chunks_exact (uniform, vectorizes) ─
        // Source: nnethercote/perf-book "Iterators" chapter.
        // chunks() yields a potentially-short final chunk, preventing LLVM
        // from vectorizing the loop body. chunks_exact() guarantees uniform
        // chunk size, enabling auto-vectorization. ~13-16× at all sizes.
        PerfRule {
            rule_id: "chunks-exact".to_string(),
            name: "slice::chunks (variable last) → slice::chunks_exact (uniform, vectorizes)".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn sum_chunks(data: &[u32]) -> u64 {\n    let mut sum = 0u64;\n    for chunk in data.chunks(8) {\n        for &v in chunk { sum += v as u64; }\n    }\n    sum\n}".to_string(),
            fixed_code: "fn sum_chunks(data: &[u32]) -> u64 {\n    let mut sum = 0u64;\n    for chunk in data.chunks_exact(8) {\n        for &v in chunk { sum += v as u64; }\n    }\n    for &v in data.chunks_exact(8).remainder() { sum += v as u64; }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(chunks_exact_bench_spec()),
        },
        // ── complexity: std HashMap (SipHash) → FxHashMap (FxHash) ─────────
        // SipHash is a cryptographic-grade hasher (DoS-resistant). For
        // non-adversarial keys (game state, internal caches), FxHash is
        // 2-4× faster with no collision risk in practice. The compiler
        // cannot swap hashers — this is an explicit choice. Used pervasively
        // in rustc itself (the compiler's default HashMap is FxHashMap).
        PerfRule {
            rule_id: "fxhashmap".to_string(),
            name: "std HashMap (SipHash) → FxHashMap (FxHash) for non-adversarial keys".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use rustc_hash::FxHashSet;\nuse std::collections::HashMap;\nfn lookup(map: &HashMap<u32, u32>, k: u32) -> Option<u32> {\n    map.get(&k).copied()\n}".to_string(),
            fixed_code: "use rustc_hash::FxHashMap;\nuse rustc_hash::FxHashSet;\nfn lookup(map: &FxHashMap<u32, u32>, k: u32) -> Option<u32> {\n    map.get(&k).copied()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(fxhashmap_bench_spec()),
        },
        // ── iterators: filter().collect() → Vec::retain (in-place) ──────────
        // The filter+collect path allocates a NEW Vec and copies surviving
        // elements. retain() filters IN-PLACE — no allocation, one pass.
        // ~1.5-2× faster. The compiler cannot transform filter().collect()
        // into an in-place retain (different ownership semantics).
        PerfRule {
            rule_id: "retain-inplace".to_string(),
            name: "iter().filter().collect() new Vec → Vec::retain() in-place filter".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn remove_multiples_of_3(data: &[u32]) -> Vec<u32> {\n    data.iter().copied().filter(|&x| x % 3 != 0).collect()\n}".to_string(),
            fixed_code: "fn remove_multiples_of_3(data: &mut Vec<u32>) {\n    data.retain(|&x| x % 3 != 0)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(retain_inplace_bench_spec()),
        },
        // ── allocation: clone + clear → mem::take (move, no clone) ──────────
        // When you want to "take" a buffer's contents and leave it empty
        // for reuse, clone+clear deep-copies every element. mem::take moves
        // the Vec's allocation — O(1), zero cloning. ~3× faster. The
        // compiler cannot elide the clone (it has visible side effects).
        PerfRule {
            rule_id: "mem-take".to_string(),
            name: "buf.clone() + buf.clear() → mem::take(&mut buf) (move, no clone)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {\n    let taken = buf.clone();\n    buf.clear();\n    taken\n}".to_string(),
            fixed_code: "fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {\n    std::mem::take(buf)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(mem_take_bench_spec()),
        },
        // ── allocation: unnecessary clone → borrow (&) ───────────────────────
        // The single most common Rust perf anti-pattern (Plan 002 T-avoid-clone).
        // Passing `data: Vec<T>` by value clones the entire collection on every
        // call. Passing `data: &[T]` borrows — zero allocation, zero copy.
        // The compiler can't elide clone (visible side effects: heap alloc).
        // ~1.3-3× faster.
        PerfRule {
            rule_id: "avoid-clone".to_string(),
            name: "fn(data: Vec<T>) clone-on-call → fn(data: &[T]) borrow".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn process(data: Vec<u8>) -> u64 {\n    data.iter().map(|&b| b as u64).sum()\n}".to_string(),
            fixed_code: "fn process(data: &[u8]) -> u64 {\n    data.iter().map(|&b| b as u64).sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(avoid_clone_bench_spec()),
        },
        // ── allocation: Vec<String> → Vec<Box<str>> (drop capacity word) ────
        // String is 3 words (ptr, len, capacity). Box<str> is 2 words (ptr, len).
        // For immutable string collections, Box<str> saves 8 bytes/element on
        // 64-bit → better cache density. ~1.1-1.3× at scale. The compiler can't
        // change the representation type.
        PerfRule {
            rule_id: "box-str-immutable".to_string(),
            name: "Vec<String> (3-word handle) → Vec<Box<str>> (2-word, no capacity)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "struct Config { labels: Vec<String> }".to_string(),
            fixed_code: "struct Config { labels: Vec<Box<str>> }".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(box_str_immutable_bench_spec()),
        },
        // ── allocation: alloc-per-call → clear-reuse (scratch buffer) ────────
        // Allocating a fresh Vec inside a hot function is wasteful when the
        // caller can pass a reusable buffer. The compiler can't hoist the
        // allocation (it may panic on OOM). **Context-dependent** — wins at
        // small-to-medium N but can lose at very large N where the reused
        // buffer exceeds L2 cache (see context_notes).
        PerfRule {
            rule_id: "reuse-allocation-clear".to_string(),
            name: "fn() { let mut v = Vec::new() } alloc-per-call → fn(buf: &mut Vec<T>) clear-reuse".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn hot(n: usize) -> u64 {\n    let mut v: Vec<u32> = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v.iter().map(|&x| x as u64).sum()\n}".to_string(),
            fixed_code: "fn hot(n: usize, buf: &mut Vec<u32>) -> u64 {\n    buf.clear();\n    for i in 0..n { buf.push(i as u32); }\n    buf.iter().map(|&x| x as u64).sum()\n}".to_string(),
            // Bench 013 Mixed (3/4 Faster, 1/4 Slower). Wins clearly at
            // N ≤ 10,000 (up to 5.97× at N=100). At N=100,000 the reused
            // buffer exceeds L2 cache → reuse path is slower than fresh alloc.
            // The rule is a genuine win for hot functions called in tight loops
            // with small-to-medium buffers; for very large buffers, alloc-per-call
            // may be fine.
            context_notes: Some(
                "Bench 013: Mixed 3/4 Faster (5.97× at N=100, 1.60× at N=1000, 1.20× at N=10000) \
                 but 0.87× at N=100000 (reused buffer exceeds L2 cache). Suggest for hot \
                 functions called in tight loops with small-to-medium buffers (≤10K elements).".to_string()
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(reuse_allocation_clear_bench_spec()),
        },
        // ── allocation: Box<small T> heap alloc → T on stack ─────────────────
        // Source: perf-book `types.md` (avoid Box for small types) + MS
        // M-BOX + reintech Box-avoidance. The heap alloc per element dwarfs
        // the data manipulation cost at all sizes.
        PerfRule {
            rule_id: "box-avoid".to_string(),
            name: "Vec<Box<i64>> (heap per elem) → Vec<i64> (stack values)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(n: usize) -> Vec<Box<i64>> {\n    let mut v = Vec::with_capacity(n);\n    for i in 0..n { v.push(Box::new(i as i64)); }\n    v\n}".to_string(),
            fixed_code: "fn build(n: usize) -> Vec<i64> {\n    let mut v = Vec::with_capacity(n);\n    for i in 0..n { v.push(i as i64); }\n    v\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(box_avoid_bench_spec()),
        },
        // ── allocation: Vec (always heap) → SmallVec/array (stack for small N) ─
        // Source: perf-book `heap-allocations.md` + reintech SmallVec. When
        // the typical size is small (≤8) but occasionally larger, the heap
        // alloc on every call dominates even at tiny element counts.
        PerfRule {
            rule_id: "smallvec-stack".to_string(),
            name: "Vec<T>::new() (always heap) → stack array for ≤8 typical (SmallVec)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn sum_small(n: usize) -> u64 {\n    let mut v: Vec<u64> = Vec::new();\n    for i in 0..n { v.push(i as u64); }\n    v.iter().copied().sum()\n}".to_string(),
            fixed_code: "fn sum_small(n: usize) -> u64 {\n    if n <= 8 {\n        let mut arr = [0u64; 8];\n        for i in 0..n { arr[i] = i as u64; }\n        arr.iter().take(n).copied().sum()\n    } else {\n        let mut v: Vec<u64> = Vec::new();\n        for i in 0..n { v.push(i as u64); }\n        v.iter().copied().sum()\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(smallvec_stack_bench_spec()),
        },
        // ── iterators/IO: File::read byte-by-byte → BufReader (batched I/O) ──
        // Source: perf-book `io.md` + MS M-BUFFERED-IO + reintech BufReader.
        // The syscall overhead per byte dominates; BufReader batches reads
        // into an 8KB internal buffer. ~97× at 100KB file size.
        PerfRule {
            rule_id: "buffered-io".to_string(),
            name: "File::read(&mut [u8; 1]) (syscall per byte) → BufReader (8KB batched)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "use std::io::Read;\nfn sum_bytes(path: &str) -> std::io::Result<u64> {\n    let mut f = std::fs::File::open(path)?;\n    let mut total = 0u64;\n    let mut byte = [0u8; 1];\n    while f.read(&mut byte)? == 1 { total += byte[0] as u64; }\n    Ok(total)\n}".to_string(),
            fixed_code: "use std::io::Read;\nfn sum_bytes(path: &str) -> std::io::Result<u64> {\n    let f = std::fs::File::open(path)?;\n    let mut br = std::io::BufReader::new(f);\n    let mut total = 0u64;\n    let mut byte = [0u8; 1];\n    while br.read(&mut byte)? == 1 { total += byte[0] as u64; }\n    Ok(total)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(buffered_io_bench_spec()),
        },
        // ── allocation: push_str(&format!(...)) temp alloc → write! direct ──
        // Source: perf-book `string-formatting.md` + MS M-LOG + reintech.
        // The buggy path creates a temporary String per iteration via format!,
        // pushes it, then drops it. The fixed path uses write! to format directly
        // into the existing buffer — zero temporary allocation. ~1.7× at all sizes.
        // Distinct from `format-concat-loop` (which is about quadratic accumulation
        // via reassignment); this is about avoiding the per-iteration temp.
        PerfRule {
            rule_id: "log-write-direct".to_string(),
            name: "buf.push_str(&format!(...)) (temp String per iter) → write!(buf, ...) direct".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build_log(items: &[u64]) -> String {\n    let mut log = String::new();\n    for &i in items { log.push_str(&format!(\"item={i}\\n\")); }\n    log\n}".to_string(),
            fixed_code: "use std::fmt::Write;\nfn build_log(items: &[u64]) -> String {\n    let mut log = String::new();\n    for &i in items { write!(&mut log, \"item={i}\\n\").unwrap(); }\n    log\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(log_write_direct_bench_spec()),
        },
        // ── allocation (NotBenchable): Vec::shrink_to_fit after build ────────
        // Source: perf-book `heap-allocations.md`. After building a Vec
        // incrementally (push loop, dedup, parse), the capacity may exceed
        // the len significantly (over-allocation during growth). shrink_to_fit
        // releases the excess. This is a **memory win**, not a speed win —
        // NotBenchable (deterministic size_of, not a runtime microbench).
        PerfRule {
            rule_id: "shrink-to-fit-after-build".to_string(),
            name: "Vec with over-allocated capacity → shrink_to_fit() after build".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v // capacity may be ~2× len due to growth\n}".to_string(),
            fixed_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v.shrink_to_fit(); // release excess capacity\n    v\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is peak memory (size_of), not speed.
            // shrink_to_fit is a no-op if capacity == len; the alloc overhead
            // of the realloc may even make it slower for the build path. The
            // value is long-lived Vecs that over-allocated during construction.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── layout (NotBenchable): Box large enum variant ─────────────────────
        // Source: perf-book `types.md`. When an enum has one large variant
        // (e.g., `[u8; 1024]`) and many small ones, every instance carries
        // the large variant's size. Boxing the large variant shrinks the
        // common case (enum becomes 8 bytes + discriminant vs 1032 bytes).
        PerfRule {
            rule_id: "box-large-enum-variant".to_string(),
            name: "Enum with one large variant → Box<LargeVariant> (shrink common case)".to_string(),
            category: "layout".to_string(),
            buggy_code: "enum Event {\n    Click { x: i32, y: i32 },\n    KeyPress(char),\n    BigData([u8; 1024]), // inflates ALL events to 1032+ bytes\n}".to_string(),
            fixed_code: "enum Event {\n    Click { x: i32, y: i32 },\n    KeyPress(char),\n    BigData(Box<[u8; 1024]>), // common case: 16 bytes (discriminant + ptr)\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is size_of (every enum instance shrinks),
            // which improves cache density. Not a speed microbench in isolation.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── iterators (NotBenchable): async yield point for fairness ────────
        // Source: perf-book `async.md`. A long-running async fn without a
        // yield point (no .await) blocks the executor for the entire duration.
        // Adding a cooperative yield point (tokio::task::yield_now) lets the
        // executor schedule other tasks. The win is **latency tail + fairness**,
        // not throughput — NotBenchable (throughput-neutral, the win is in
        // p99 latency under multi-task contention).
        PerfRule {
            rule_id: "async-yield-point".to_string(),
            name: "Long async fn (no yield) → cooperative yield_now() for fairness".to_string(),
            category: "iterators".to_string(),
            buggy_code: "async fn process_batch(items: &[Item]) {\n    for item in items {\n        process(item); // no .await → blocks executor for entire batch\n    }\n}".to_string(),
            fixed_code: "async fn process_batch(items: &[Item]) {\n    for (i, item) in items.iter().enumerate() {\n        process(item);\n        if i % 100 == 0 { tokio::task::yield_now().await; }\n    }\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is p99 latency under contention (fairness),
            // not single-task throughput. Throughput-neutral or slightly negative.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── layout (NotBenchable): async fn future size reduction ───────────
        // Source: perf-book `async.md`. Each async fn compiles to a state
        // machine struct whose size is the max of all locals across all await
        // points. Large locals (big Vecs, arrays) inflates the future size,
        // wasting memory per spawned task. Scoping locals (block expressions)
            // drops them before the next await, shrinking the state machine.
        PerfRule {
            rule_id: "async-future-size".to_string(),
            name: "Large locals across .await → scoped blocks (shrink future state machine)".to_string(),
            category: "layout".to_string(),
            buggy_code: "async fn handle() {\n    let big_buf = vec![0u8; 8192]; // lives across .await → in future\n    let result = do_work(&big_buf).await;\n    // big_buf still in the future state machine here\n    process(result);\n}".to_string(),
            fixed_code: "async fn handle() {\n    let result = {\n        let big_buf = vec![0u8; 8192]; // dropped before .await returns\n        do_work(&big_buf).await\n    }; // big_buf no longer in future state machine\n    process(result);\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is size_of_val of the future (memory per
            // spawned task), not a runtime microbench. Measurable via
            // std::mem::size_of_val but that's deterministic, not a timing.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── complexity: stable sort → unstable sort (no allocation + faster algo) ─
        // Source: std docs + perf-book. `sort()` uses a stable mergesort that
        // allocates temporary storage; `sort_unstable()` uses pattern-defeating
        // quicksort (pdqsort) which is in-place + typically 2-3× faster for
        // random data. The compiler cannot transform a stable sort into an
        // unstable one — this is a library-level algorithm choice.
        PerfRule {
            rule_id: "sort-unstable".to_string(),
            name: "Vec::sort() stable (allocates) → Vec::sort_unstable() (in-place, faster)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn sort_scores(scores: &mut Vec<u32>) {\n    scores.sort();\n}".to_string(),
            fixed_code: "fn sort_scores(scores: &mut Vec<u32>) {\n    scores.sort_unstable();\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sort_unstable_bench_spec()),
        },
        // ── complexity: sort_by(closure) → sort_by_key(closure) (N vs N·log N key extractions) ─
        // Source: std docs. `sort_by(|a, b| f(a).cmp(&f(b)))` calls `f` O(N log N)
        // times; `sort_by_key(|x| f(x))` extracts each key exactly once (N calls)
        // then sorts the cached keys. The win scales with key-extraction cost.
        PerfRule {
            rule_id: "sort-by-key".to_string(),
            name: "sort_by(|a,b| a.x.cmp(&b.x)) → sort_by_key(|x| x.x) (extract key once per element)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn sort_by_score(items: &mut Vec<(String, u32)>) {\n    items.sort_by(|a, b| a.1.cmp(&b.1));\n}".to_string(),
            fixed_code: "fn sort_by_score(items: &mut Vec<(String, u32)>) {\n    items.sort_by_key(|item| item.1);\n}".to_string(),
            context_notes: Some("ParetoDominated in Bench 016 default config: 4/4 Tie at n=1K-100K with integer keys on (String, u32) tuples. The compiler optimizes sort_by's comparator to avoid redundant key reads for simple field access. sort_by_key is a correctness/idiom win (avoids writing the comparator twice), not a throughput win for trivial keys. The win materializes only when the key function is non-trivial (hash computation, nested dereference, or expensive computation that the compiler cannot CSE across comparator calls).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sort_by_key_bench_spec()),
        },
        // ── io: per-record File::write → BufWriter batched (syscall reduction) ──
        // Source: perf-book `io.md` + std docs. Each `File::write` call is a
        // syscall (~1-2µs on Linux, ~2-4µs on macOS). `BufWriter` coalesces
        // writes into 8KB buffers, amortizing the syscall cost. This is the
        // write-side analog of `buffered-io` (BufReader for reads).
        PerfRule {
            rule_id: "bufwriter-batched".to_string(),
            name: "File::write per-record (syscall each) → BufWriter batched (8KB coalesced)".to_string(),
            category: "io".to_string(),
            buggy_code: "use std::io::Write;\nfn write_records(file: &mut std::fs::File, records: &[Vec<u8>]) -> std::io::Result<()> {\n    for rec in records {\n        file.write_all(rec)?; // syscall per record\n    }\n    Ok(())\n}".to_string(),
            fixed_code: "use std::io::{BufWriter, Write};\nfn write_records(file: std::fs::File, records: &[Vec<u8>]) -> std::io::Result<()> {\n    let mut buf = BufWriter::new(file);\n    for rec in records {\n        buf.write_all(rec)?; // buffered → one syscall per 8KB\n    }\n    buf.flush()?; // flush remaining buffered data\n    Ok(())\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bufwriter_batched_bench_spec()),
        },
        // ── simd: manual index-loop copy → slice::copy_from_slice (SIMD + no bounds check) ─
        // Source: std docs + perf-book `simd.md`. A manual `for i in 0..n { dst[i] =
        // src[i]; }` loop does per-element bounds checking + scalar copies.
        // `copy_from_slice` is a single call that LLVM lowers to `memcpy` /
        // SIMD-wide stores when the length is known or aligned. The compiler
        // *can* sometimes auto-vectorize the manual loop, but `copy_from_slice`
        // guarantees it + eliminates the per-index bounds check.
        PerfRule {
            rule_id: "copy-from-slice".to_string(),
            name: "Manual index-loop element copy → slice::copy_from_slice (memcpy / SIMD)".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn copy_data(dst: &mut [u8], src: &[u8]) {\n    for i in 0..src.len() {\n        dst[i] = src[i];\n    }\n}".to_string(),
            fixed_code: "fn copy_data(dst: &mut [u8], src: &[u8]) {\n    dst.copy_from_slice(src);\n}".to_string(),
            context_notes: Some("Mixed in Bench 016 default config: 2/4 Faster (n=64, n=1024), 1/4 Slower (n=4096), 1/4 Tie (n=256). LLVM auto-vectorizes the manual index loop effectively for u8 slices — at medium sizes (256-1024) the performance is indistinguishable, and at n=4096 the manual loop is marginally faster (likely due to memcpy's setup overhead being amortized differently). copy_from_slice is still the correct idiom (eliminates bounds checks, guarantees SIMD lowering, is less code) but consumers should know the throughput difference is small for u8 in release mode. The win is clearer for larger element types (u64, structs) where the manual loop's per-element bounds check is more expensive relative to the copy.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(copy_from_slice_bench_spec()),
        },
        // ── complexity: Vec::remove(0) O(n) shift → VecDeque::pop_front O(1) ──
        // Fills the corpus's data-structure-choice gap: zero prior rules about
        // choosing the right collection. Vec::remove(0) does a memmove of all
        // remaining elements left by one position — O(n) per call, O(n²) to
        // drain N elements. VecDeque is a ring buffer: pop_front just advances
        // the head index — O(1) per call, O(n) to drain. The compiler cannot
        // turn a contiguous-array memmove into a ring-buffer head advance.
        PerfRule {
            rule_id: "vec-remove-front-to-vecdeque".to_string(),
            name: "Vec::remove(0) O(n) shift → VecDeque::pop_front O(1)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn drain_queue(queue: &mut Vec<u32>) -> u64 {\n    let mut sum = 0u64;\n    while !queue.is_empty() {\n        sum += queue.remove(0) as u64;\n    }\n    sum\n}".to_string(),
            fixed_code: "use std::collections::VecDeque;\nfn drain_queue(queue: &mut VecDeque<u32>) -> u64 {\n    let mut sum = 0u64;\n    while let Some(val) = queue.pop_front() {\n        sum += val as u64;\n    }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_remove_front_bench_spec()),
        },
        // ── complexity: HashMap + sort-on-each-query → BTreeMap natural order ──
        // Data-structure choice: when sorted iteration is needed repeatedly,
        // HashMap forces O(n log n) sort per query. BTreeMap maintains sorted
        // order by construction (self-balancing BST) → O(n) iteration. The
        // compiler cannot replace a hash table + sort with a balanced tree.
        PerfRule {
            rule_id: "btreemap-sorted-iteration".to_string(),
            name: "HashMap + sort-on-each-query → BTreeMap natural sorted iteration".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashMap;\nfn sorted_keys(map: &HashMap<u32, u32>) -> Vec<u32> {\n    let mut keys: Vec<u32> = map.keys().copied().collect();\n    keys.sort_unstable();\n    keys\n}".to_string(),
            fixed_code: "use std::collections::BTreeMap;\nfn sorted_keys(map: &BTreeMap<u32, u32>) -> Vec<u32> {\n    map.keys().copied().collect()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(btreemap_sorted_iteration_bench_spec()),
        },
        // ── allocation: String return (always alloc) → Cow<str> borrow ──────
        // Data-structure choice: when a function conditionally transforms a
        // string, returning String forces an allocation even when no
        // transformation occurs. Cow<str> borrows the input in the common
        // (no-work) case — zero allocation. The compiler cannot avoid a
        // String heap allocation; it's a concrete type, not an optimization
        // target. The classic example: HTML escaping where 90%+ of strings
        // have no special characters.
        PerfRule {
            rule_id: "cow-conditional-alloc".to_string(),
            name: "String return (always alloc) → Cow<str> (borrow when no transform)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn escape_html(s: &str) -> String {\n    if !s.contains(['<', '>', '&']) {\n        return s.to_string();\n    }\n    s.replace('<', \"&lt;\")\n        .replace('>', \"&gt;\")\n        .replace('&', \"&amp;\")\n}".to_string(),
            fixed_code: "use std::borrow::Cow;\nfn escape_html(s: &str) -> Cow<str> {\n    if !s.contains(['<', '>', '&']) {\n        return Cow::Borrowed(s);\n    }\n    Cow::Owned(s.replace('<', \"&lt;\").replace('>', \"&gt;\").replace('&', \"&amp;\"))\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(cow_conditional_alloc_bench_spec()),
        },
        // ── caching: Regex::new per call → static LazyLock<Regex> ─────────────
        // Source: Research 032 (hotpath-rs profiling guide). Regex compilation
        // is runtime NFA construction — ~µs of real work the compiler cannot
        // elide (unlike `precompute-table`'s POPCNT shape). Hoisting to a
        // `static LazyLock` compiles ONCE; every call after is a match-only.
        // The guide's prior: ~555× per call. Bench 020 verifies on our harness.
        PerfRule {
            rule_id: "regex-hoist-lazylock".to_string(),
            name: "Regex::new per call → static LazyLock<Regex> (compile once)".to_string(),
            category: "caching".to_string(),
            buggy_code: "use regex::Regex;\nfn is_valid(code: &str) -> bool {\n    let re = Regex::new(r\"^[A-Z]{2}-\\d{4}$\").unwrap(); // recompiles every call\n    re.is_match(code)\n}".to_string(),
            fixed_code: "use regex::Regex;\nuse std::sync::LazyLock;\nstatic RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r\"^[A-Z]{2}-\\d{4}$\").unwrap());\nfn is_valid(code: &str) -> bool {\n    RE.is_match(code)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(regex_hoist_lazylock_bench_spec()),
        },
        // ── allocation: clone-per-message payload → Arc<str> shared ──────────
        // Source: Research 032 (hotpath-rs profiling guide). Fanning out an
        // immutable payload to N recipients via `to_string()` clones copies N
        // × payload-bytes. `Arc<str>` allocates the payload ONCE; recipients
        // hold refcount-bumped handles (pointer copies). Timing prior ~1.3×
        // (memcpy-bound) may Tie — the deterministic alloc axis carries the
        // gate: N×payload bytes → ~1 allocation. Distinct from `avoid-clone`
        // (borrow vs clone at fn boundary) + `box-str-immutable` (struct width).
        PerfRule {
            rule_id: "arc-str-payload".to_string(),
            name: "Clone-per-message immutable payload → Arc<str> (clone the handle, not the bytes)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, String)> {\n    ids.iter().map(|&id| (id, payload.to_string())).collect() // N payload clones\n}".to_string(),
            fixed_code: "use std::sync::Arc;\nfn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, Arc<str>)> {\n    let shared: Arc<str> = Arc::from(payload); // ONE allocation\n    ids.iter().map(|&id| (id, Arc::clone(&shared))).collect() // refcount bumps\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(arc_str_payload_bench_spec()),
        },
        // ── concurrency: guard held across .await → fetch-before-guard ───────
        // Source: Research 032 (hotpath-rs profiling guide). Holding a
        // parking_lot/tokio MutexGuard across a slow .await serializes every
        // OTHER task that touches the lock — the fix fetches what's needed,
        // drops the guard, THEN awaits. clippy's `await_holding_lock` covers
        // std guards only; this rule targets the ecosystem lock crates.
        PerfRule {
            rule_id: "tokio-guard-across-await".to_string(),
            name: "MutexGuard held across .await → fetch data, drop guard, then await".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {\n    let mut cfg = cache.lock().await;\n    let upstream = fetch(&key).await; // guard held across await — blocks all other lockers\n    cfg.apply(upstream);\n}".to_string(),
            fixed_code: "async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {\n    let upstream = fetch(&key).await; // await FIRST — no guard held\n    let mut cfg = cache.lock().await;\n    cfg.apply(upstream);\n    // guard drops at scope end, after the slow path\n}".to_string(),
            context_notes: Some("NotBenchable (metric mismatch): the cost is OTHER tasks' wait latency while the guard is held across a slow await, not this task's wall-clock. A single-threaded harness measures no regression on the buggy path; the win only appears under concurrent lock contention (same class as the park-vs-spin rejection + bevy-reactive-loop NotBenchable precedent). Also note clippy's await_holding_lock lint covers std sync guards only — this rule targets tokio/parking_lot/async-lock guards.".to_string()),
            // Not benchable: contention-latency metric, not wall-clock.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: unbounded channel → bounded (backpressure) ──────────
        // Source: Research 032 (hotpath-rs profiling guide). An unbounded
        // channel + a send loop that outpaces the consumer grows the queue
        // without bound (memory blowup + consumer sees stale data). A bounded
        // channel applies backpressure: send().await suspends the producer.
        PerfRule {
            rule_id: "bounded-channel-backpressure".to_string(),
            name: "Unbounded channel + hot producer → bounded channel (backpressure)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();\nfor event in events {\n    tx.send(event).unwrap(); // never blocks — queue grows unboundedly if consumer is slow\n}".to_string(),
            fixed_code: "let (tx, mut rx) = tokio::sync::mpsc::channel(1024);\nfor event in events {\n    tx.send(event).await.unwrap(); // backpressure: suspends producer when full\n}".to_string(),
            context_notes: Some("NotBenchable (metric mismatch): the win is bounded queue depth + fresh-data latency under producer>consumer overload, not throughput — the buggy path is often FASTER on wall-clock precisely because it buffers unboundedly (that is the bug). Semantics-changing: bounded send applies backpressure (producer suspends) vs unbounded buffering; adopt deliberately, size the bound to your latency budget.".to_string()),
            // Not benchable: queue-depth/backpressure metric + semantics-changing.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── allocation: collect() + append() → extend() ───────────────────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). Staging an iterator into an intermediate `Vec` only to
        // `append` it into the target pays one extra allocation + an extra
        // memcpy per chunk. `extend` writes the iterator's items straight
        // into the target's growth path — zero intermediates. The compiler
        // cannot elide a `collect` (it materializes a concrete `Vec`).
        PerfRule {
            rule_id: "collect-append-to-extend".to_string(),
            name: "collect() + append() staging → extend() (no intermediate Vec)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in chunks {\n        let mut staged: Vec<u32> = c.iter().copied().collect(); // intermediate Vec alloc per chunk\n        target.append(&mut staged);\n    }\n}".to_string(),
            fixed_code: "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in chunks {\n        target.extend(c.iter().copied()); // writes straight into the target\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(collect_append_to_extend_bench_spec()),
        },
        // ── complexity: HashSet contains+insert → insert's bool return ────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). `HashSet` has no `entry` API (unlike `HashMap` — see the
        // `entry-vs-contains-insert` sibling); the membership check IS the
        // insert's `bool` return. `!set.contains(&x)` + `set.insert(x)` walks
        // the hash table twice; `set.insert(x)` alone walks it once. The two
        // calls are opaque to LLVM (hashbrown internals) — no fusion.
        PerfRule {
            rule_id: "hashset-insert-bool-return".to_string(),
            name: "HashSet contains+insert double lookup → insert's bool return (one lookup)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashSet;\nfn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    for &x in items {\n        if !seen.contains(&x) {\n            seen.insert(x);\n            inserted += 1;\n        }\n    }\n    inserted\n}".to_string(),
            fixed_code: "use std::collections::HashSet;\nfn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    for &x in items {\n        if seen.insert(x) {\n            inserted += 1;\n        }\n    }\n    inserted\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hashset_insert_bool_return_bench_spec()),
        },
        // ── allocation: unwrap_or(eager) → unwrap_or_else(closure) ────────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). The `Option` sibling of `ok-or-else`: `unwrap_or(expr)`
        // evaluates `expr` BEFORE checking the `Some` variant — the default
        // is computed (and allocated, if it allocates) even when discarded.
        // `unwrap_or_else(|| expr)` defers construction to the `None` arm.
        // Wins when the default is non-trivial AND `Some` is the common case.
        PerfRule {
            rule_id: "unwrap-or-else-lazy".to_string(),
            name: "unwrap_or(expensive()) eager default → unwrap_or_else(closure) lazy".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn resolve(opt: Option<u32>) -> u32 {\n    opt.unwrap_or(expensive_default(64)) // eagerly evaluated even when Some\n}\nfn expensive_default(n: u32) -> u32 {\n    (0..n).fold(0u32, |acc, i| acc.wrapping_add(i * 31))\n}".to_string(),
            fixed_code: "fn resolve(opt: Option<u32>) -> u32 {\n    opt.unwrap_or_else(|| expensive_default(64)) // built only on None\n}\nfn expensive_default(n: u32) -> u32 {\n    (0..n).fold(0u32, |acc, i| acc.wrapping_add(i * 31))\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(unwrap_or_else_bench_spec()),
        },
        // ── iterators: filter(p).count() > 0 → any(p) short-circuit ───────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). `filter(p).count() > 0` scans the WHOLE iterator, builds a
        // count, then throws it away. `any(p)` stops at the first hit. The
        // compiler cannot derive a short-circuit from a full `count` fold.
        // The win scales with how early the first match lands.
        PerfRule {
            rule_id: "filter-count-to-any".to_string(),
            name: "filter(p).count() > 0 full scan → any(p) short-circuit".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn has_even(vals: &[u32]) -> bool {\n    vals.iter().filter(|&&v| v % 2 == 0).count() > 0 // full scan, no short-circuit\n}".to_string(),
            fixed_code: "fn has_even(vals: &[u32]) -> bool {\n    vals.iter().any(|&v| v % 2 == 0) // stops at the first hit\n}".to_string(),
            // Bench 48: ParetoDominated at mid-position hits (0.82-0.90×) —
            // a genuine folk-wisdom refutation. LLVM VECTORIZES the count
            // loop (batched element scanning) while any's early-exit forces
            // scalar iteration; at a 50% hit position the vectorized FULL
            // count beats the scalar half-scan. any wins only for very early
            // hits (empirically ≤ ~10% position) or non-vectorizable
            // predicates. Keep as an idiom rule with the regime caveat.
            context_notes: Some(
                "Bench 48: ParetoDominated 0.82-0.90x at mid-position hits. \
                 LLVM vectorizes filter().count() into batched scanning while \
                 any()'s early-exit forces scalar iteration — at a 50% hit \
                 position the vectorized full count WINS. any() only wins for \
                 very early hits (~<10% position) or non-vectorizable \
                 predicates. Regime-dependent: suggest only when hits are \
                 known-early or the predicate is expensive/branchy."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_count_to_any_bench_spec()),
        },
        // ── allocation: Vec::new + resize(n, 0) → vec![0; n] calloc path ──────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). `Vec::new()` + `resize(n, 0)` allocates uninitialized
        // memory then runs an explicit memset pass over every byte. `vec![0; n]`
        // routes through `alloc_zeroed` (calloc) — for large buffers the
        // allocator hands back lazily-zeroed OS pages, skipping the touch.
        PerfRule {
            rule_id: "vec-macro-zero-fill".to_string(),
            name: "Vec::new() + resize(n, 0) memset → vec![0; n] alloc_zeroed (calloc)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn zero_buf(n: usize) -> Vec<u8> {\n    let mut v = Vec::new(); // uninit alloc\n    v.resize(n, 0); // explicit memset pass\n    v\n}".to_string(),
            fixed_code: "fn zero_buf(n: usize) -> Vec<u8> {\n    vec![0u8; n] // alloc_zeroed — calloc zero pages\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_macro_zero_fill_bench_spec()),
        },
        // ── iterators: .map(|x| *x) → .copied() specialized adapter ───────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). `.map(|x| *x)` wraps the iterator in the generic `Map`
        // adapter (opaque closure, no size hints). `.copied()` is a
        // specialized adapter implementing `TrustedLen` + `InPlaceIterable`
        // markers — downstream consumers (collect, sum) get exact-size
        // reservations the `Map` form cannot express. In tight release loops
        // LLVM often lowers both identically — the win regime is adapter
        // fusion + allocation sizing, which the bench measures honestly.
        PerfRule {
            rule_id: "iter-copied-deref-map".to_string(),
            name: ".map(|x| *x) generic adapter → .copied() specialized (TrustedLen)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn sum_kills(kills: &[u64]) -> u64 {\n    kills.iter().map(|k| *k).sum() // generic Map adapter, no size hint\n}".to_string(),
            fixed_code: "fn sum_kills(kills: &[u64]) -> u64 {\n    kills.iter().copied().sum() // specialized Copy adapter\n}".to_string(),
            // Bench 48: Mixed — parity at ≤100K, 0.80× at 1M (copied SLOWER).
            // The specialized adapter's length-tracking overhead loses to the
            // fully-unrolled map+sum loop at large N on a sum fold. The rule
            // stands as an idiom (TrustedLen matters for collect's exact-size
            // reservations, not for folds); do not suggest for large-N folds.
            context_notes: Some(
                "Bench 48: Mixed (tie/tie/0.80x at 1K/100K/1M). The specialized \
                 Copied adapter carries length-tracking overhead that loses to \
                 the fully-unrolled Map+sum loop at large N on a sum fold. The \
                 idiom remains correct for collect() (exact-size reservation via \
                 TrustedLen) and for readability; do not suggest expecting a \
                 throughput win on large-N folds."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(iter_copied_deref_map_bench_spec()),
        },
        // ── iterators: filter(p).map(f) → filter_map fused adapter ────────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). The perf-book HEDGES on this one ("filter_map is not
        // always faster") — the fused form removes one adapter state-machine
        // layer but blocks some internal specializations of the separated
        // adapters. Genuinely regime-dependent; the Pareto bench decides the
        // corpus annotation, not the book's hedged prose.
        PerfRule {
            rule_id: "filter-map-fuse".to_string(),
            name: "filter(p).map(f) two adapters → filter_map fused (regime-dependent)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn even_squares(vals: &[u32]) -> Vec<u32> {\n    vals.iter().filter(|&&v| v % 2 == 0).map(|&v| v * v).collect()\n}".to_string(),
            fixed_code: "fn even_squares(vals: &[u32]) -> Vec<u32> {\n    vals.iter().filter_map(|&v| (v % 2 == 0).then_some(v * v)).collect()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_map_fuse_bench_spec()),
        },
        // ── complexity: Vec::insert(0, x) loop → VecDeque::push_front ─────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). The write-side sibling of `vec-remove-front-to-vecdeque`:
        // every `insert(0, x)` memmoves the whole buffer down (O(n) per
        // insert, O(n²) per loop); `VecDeque::push_front` advances a ring
        // head in O(1). The compiler cannot turn a front-insert loop into a
        // ring buffer.
        PerfRule {
            rule_id: "vec-insert-front-deque".to_string(),
            name: "Vec::insert(0, x) O(n²) prepend loop → VecDeque::push_front O(1)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn prepend_history(history: &mut Vec<u32>, events: &[u32]) {\n    for &e in events {\n        history.insert(0, e); // O(n) memmove per insert → O(n²) total\n    }\n}".to_string(),
            fixed_code: "use std::collections::VecDeque;\nfn prepend_history(history: &mut VecDeque<u32>, events: &[u32]) {\n    for &e in events {\n        history.push_front(e); // O(1) ring-buffer head advance\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_insert_front_deque_bench_spec()),
        },
        // ── arithmetic: x.powf(2.0) constant exponent → x.powi(2) ──────────────
        // Source: users.rust-lang.org/t/56451 + baptiste-wicht.com std::pow C++
        // study (Batch 53, idle-queue item 11). `powf` lowers to a libm `pow`
        // call — an opaque scheduling barrier in a hot loop; `powi(2)` compiles
        // to a single multiply. The candidate risk (recorded honestly by the
        // bench, not assumed): LLVM's instcombine rewrites `pow(x, 2.0)` to
        // `x * x` in release — the verdict axis decides whether this is a
        // runtime win or an idiom-only rule.
        PerfRule {
            rule_id: "powi-vs-powf".to_string(),
            name: "x.powf(2.0) libm call → x.powi(2) single multiply (constant exponent)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn sum_squares(vals: &[f32]) -> f32 {\n    vals.iter().map(|v| v.powf(2.0)).sum() // libm pow call per element\n}".to_string(),
            fixed_code: "fn sum_squares(vals: &[f32]) -> f32 {\n    vals.iter().map(|v| v.powi(2)).sum() // single multiply\n}".to_string(),
            // Bench 53 (Bench 038): all-Tie at 1.00x across 1K/100K/1M —
            // LLVM instcombine lowers pow(x, 2.0) to the same x*x multiply
                       // as powi(2) in release, so the classic "powf is slow" advice
            // does NOT hold for the constant-exponent-2 release case. The
            // idiom still matters for: debug-profile hot loops (no instcombine
            // — powf stays a libm call), exponents >= 3 (LLVM does NOT lower
            // pow(x, 3.0) — correctly-rounded pow differs from x*x*x, so it
            // stays a libm call while powi(3) becomes two multiplies), and
            // non-constant exponents where powi is not applicable.
            context_notes: Some(
                "Bench 53: Mixed — all-Tie 1.00x (1K/100K/1M) — LLVM lowers powf(2.0) \
                 to the same x*x as powi(2) in release; the folk wisdom \
                 ('powf is extremely slow') does not hold for \
                 constant-exponent-2 release code. Suggest as an idiom for \
                 debug-profile hot loops (powf stays a libm call without \
                 instcombine) and exponents >= 3 (pow(x,3.0) is NOT lowered — \
                 it stays a libm call while powi(3) is two multiplies); do \
                 not suggest expecting a release-throughput win at exponent 2."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(powi_vs_powf_bench_spec()),
        },
        // ── arithmetic: (dx²+dy²).sqrt() < r → dx²+dy² < r² ────────────────────
        // Source: the Box2D squared-distance convention (box2d.org SIMD
        // matters). sqrt is monotonic on non-negative inputs, so comparing
        // squared distances preserves ordering — the sqrt is pure waste when
        // only the COMPARISON is consumed. Guard: if the distance VALUE is
        // consumed downstream, keep the sqrt; NaN inputs order differently
        // under the two forms (both false in practice for <).
        PerfRule {
            rule_id: "distance-sq-no-sqrt".to_string(),
            name: "(dx*dx+dy*dy).sqrt() < r → dx*dx+dy*dy < r*r (compare squared)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn in_range(dx: f32, dy: f32, r: f32) -> bool {\n    (dx * dx + dy * dy).sqrt() < r // sqrt only to compare\n}".to_string(),
            fixed_code: "fn in_range(dx: f32, dy: f32, r: f32) -> bool {\n    dx * dx + dy * dy < r * r // monotonic on non-negatives — ordering preserved\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(distance_sq_no_sqrt_bench_spec()),
        },
        // ── allocation: HashMap::new() + known-count loop → with_capacity ────
        // Source: users.rust-lang.org/t/44322 ("preallocate your Vecs and
        // HashMaps"). The HashMap sibling of `vec-with-capacity` but a
        // DIFFERENT cost class: growth rehashes — every resize re-runs the
        // hasher (SipHash) over ALL stored elements to place them in the new
        // bucket set, not just a realloc+memcpy. `with_capacity(n)` with a
        // known count pays one table alloc + zero rehash passes.
        PerfRule {
            rule_id: "hashmap-with-capacity".to_string(),
            name: "HashMap::new() + known-count insert loop → HashMap::with_capacity(n)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "use std::collections::HashMap;\nfn index_items(items: &[u32]) -> HashMap<u32, u32> {\n    let mut map = HashMap::new(); // starts at capacity ~3\n    for &it in items {\n        map.insert(it, it.wrapping_mul(31)); // rehash ladder: ~log2(n) resizes, each re-SipHashing all entries\n    }\n    map\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn index_items(items: &[u32]) -> HashMap<u32, u32> {\n    let mut map = HashMap::with_capacity(items.len()); // one table alloc, zero resizes\n    for &it in items {\n        map.insert(it, it.wrapping_mul(31));\n    }\n    map\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hashmap_with_capacity_bench_spec()),
        },
        // ── layout: AoS iteration over one hot field → SoA split ─────────────
        // Source: nnethercote/perf-book data-structure layout + the Box2D SIMD
        // post (Batch 53, idle-queue item 11). Iterating ONE field of a wide
        // AoS struct touches every cache line but consumes 4 of 32 bytes
        // (12.5% utilization) and defeats auto-vectorization (strided
        // gather). The SoA split makes the hot field contiguous — 16 floats
        // per cache line + a clean SIMD sum. The cold fields pay one pointer
        // indirection on THEIR (rare) access.
        PerfRule {
            rule_id: "soa-hot-field".to_string(),
            name: "AoS Vec<WideStruct> single-field scan → SoA split (hot field contiguous)".to_string(),
            category: "layout".to_string(),
            buggy_code: "struct Agent { x: f32, y: f32, vx: f32, vy: f32, hp: u32, level: u32, flags: u64, id: u64 } // 40 bytes\nfn total_x(agents: &[Agent]) -> f32 {\n    agents.iter().map(|a| a.x).sum() // 4 useful bytes per 40-byte struct — strided, scalar\n}".to_string(),
            fixed_code: "struct Agents { xs: Vec<f32>, ys: Vec<f32>, vxs: Vec<f32>, vys: Vec<f32>, hps: Vec<u32>, levels: Vec<u32>, flags: Vec<u64>, ids: Vec<u64> }\nfn total_x(agents: &Agents) -> f32 {\n    agents.xs.iter().sum() // contiguous — 16 floats/line, SIMD-able\n}".to_string(),
            // Bench 53 (Bench 038): Mixed 1.07x/tie/0.96x — a serial f32
            // add-reduction (~3-4 cycle dependency chain per element) dominates
            // BOTH sides; the 40-byte-stride load latency is fully hidden by
            // out-of-order execution, so the bandwidth+vectorization win never
            // materializes on a latency-bound scalar sum. SoA wins materialize
            // on THROUGHPUT-bound field consumers (count/filter over the field,
            // min/max, batch transform — vectorizable, no serial chain) or when
            // multiple hot fields are scanned together.
            context_notes: Some(
                "Bench 53: Mixed (1.07x/tie/0.96x at 1K/100K/1M) — a serial f32 \
                 add-reduction dominates both layouts; the strided-load penalty \
                 is hidden by OoO execution, so SoA buys nothing on \
                 latency-bound scalar sums. The layout win materializes on \
                 throughput-bound field consumers (count/filter, min/max, batch \
                 transform — vectorizable, no serial chain) or multi-field scans; \
                 suggest for those shapes, not for scalar sum folds."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(soa_hot_field_bench_spec()),
        },
        // ── arithmetic: loop-invariant RUNTIME divisor → magic multiplication ──
        // Source: libdivide (ridiculous_fish) via thermite's Rust port
        // (raygon-renderer/thermite `divider.rs` — Batch 59, the first
        // external-source distill via the `distill` skill). Hardware
        // integer division is 20-90+ cycles and NOT vectorizable; a
        // loop-INVARIANT runtime divisor (a function parameter — opaque to
        // LLVM, so no strength reduction) converts once into a magic
        // multiplier + shift (Granlund–Montgomery), then each division is
        // mulhi + shifts (~4 cycles, pipelined). Two-regime rule (thermite's
        // own split): CONSTANT divisors need nothing — LLVM already
        // strength-reduces them to multiplication at compile time; the win
        // is exclusively the RUNTIME-invariant case applied ≥~3 times
        // (setup pays one hardware div). Guard: d must be nonzero + the
        // same value for the whole batch (bucket counts, shard stripes,
        // hash-then-div layouts).
        PerfRule {
            rule_id: "division-by-invariant-multiplication".to_string(),
            name: "Loop of x / d with runtime-invariant d → precomputed magic multiplier + shift (libdivide)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn bucket_sum(values: &[u32], d: u32) -> u32 {\n    let mut acc = 0;\n    for &x in values {\n        acc = acc.wrapping_add(x / d); // hardware udiv per element — d is a runtime invariant\n    }\n    acc\n}".to_string(),
            fixed_code: "struct FastDivU32 { magic: u32, shift: u32, add: bool }\nimpl FastDivU32 {\n    fn new(d: u32) -> Self { // one hardware div, once — amortized after ~3 divisions\n        let l = 31 - d.leading_zeros();\n        if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }\n        let k = 1u64 << l;\n        let base = (k << 32) / u64::from(d);\n        let rem = (k << 32) % u64::from(d);\n        if u64::from(d) - rem < k {\n            Self { magic: base as u32 + 1, shift: l, add: false }\n        } else {\n            let mut m = base * 2;\n            let rem2 = rem * 2;\n            if rem2 >= u64::from(d) { m += 1; }\n            Self { magic: (m + 1) as u32, shift: l, add: true } // (m+1) wraps mod 2^32 — intended (m spans (2^32, 2^33))\n        }\n    }\n    #[inline(always)]\n    fn div(&self, x: u32) -> u32 {\n        if self.magic == 0 { return x >> self.shift; } // power-of-two divisor\n        let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;\n        if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift } // widening add: u32 wrap drops the carry (d=7 x=MAX)\n    }\n}\nfn bucket_sum(values: &[u32], d: u32) -> u32 {\n    let fd = FastDivU32::new(d); // hoisted out of the loop\n    let mut acc = 0;\n    for &x in values {\n        acc = acc.wrapping_add(fd.div(x)); // mulhi + shift per element\n    }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(division_invariant_bench_spec()),
        },
        // ── dispatch: static-keyword HashMap lookup → plain match (LLVM length-switch) ──
        // Source: oxc "Pursuit of Performance on Building a JavaScript
        // Compiler" (oxc.rs/docs/learn/performance, rustmagazine issue-3;
        // Batch 61 online-search distill). oxc tried to beat a plain match on
        // string literals with a gperf-style perfect hash (V8's approach) and
        // COULD NOT — `--emit=llvm-ir` shows LLVM already lowers the match to
        // a switch on string LENGTH + one inline compare per length group.
        // The folk wisdom "string match is O(n) comparisons, use a hash map"
        // is wrong on modern LLVM: the HashMap pays SipHash per lookup while
        // the match pays a length switch + 1-2 byte compares. Bounded
        // fix-space: STATIC key sets only (a literal-initialized map);
        // dynamic keys keep the map.
        PerfRule {
            rule_id: "keyword-map-to-match".to_string(),
            name: "Static-keyword LazyLock<HashMap<&str, V>> dispatch → plain match (LLVM lowers to a length switch)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "use std::collections::HashMap;\nuse std::sync::LazyLock;\n#[derive(Clone, Copy)]\nenum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }\nstatic KEYWORDS: LazyLock<HashMap<&'static str, Kind>> = LazyLock::new(|| {\n    let mut m = HashMap::new();\n    m.insert(\"as\", Kind::As);\n    m.insert(\"do\", Kind::Do);\n    m.insert(\"if\", Kind::If);\n    m.insert(\"let\", Kind::Let);\n    m.insert(\"fn\", Kind::Fn);\n    m.insert(\"while\", Kind::While);\n    m.insert(\"constructor\", Kind::Constructor);\n    m\n});\nfn match_keyword(s: &str) -> Kind {\n    *KEYWORDS.get(s).unwrap_or(&Kind::Ident) // SipHash per lookup\n}".to_string(),
            fixed_code: "#[derive(Clone, Copy)]\nenum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }\nfn match_keyword(s: &str) -> Kind {\n    // LLVM lowers string-literal matches to a switch on length +\n    // inline compares per length group (oxc verified via --emit=llvm-ir)\n    match s {\n        \"as\" => Kind::As,\n        \"do\" => Kind::Do,\n        \"if\" => Kind::If,\n        \"let\" => Kind::Let,\n        \"fn\" => Kind::Fn,\n        \"while\" => Kind::While,\n        \"constructor\" => Kind::Constructor,\n        _ => Kind::Ident,\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(keyword_map_to_match_bench_spec()),
        },
        // ── layout: usize index fields in high-volume structs → u32 ──────────
        // Source: oxc Span optimization (same article, Batch 61). The AST
        // leaf carried `struct Node { start: usize, end: usize }` — 16 bytes
        // — for byte offsets that cannot exceed u32 (< 4GB source). Narrowing
        // to u32 halved the node + measured up to +5% on large files (cache
        // density on the hottest leaf type). The mechanism is bandwidth:
        // half the bytes per element means twice the elements per cache
        // line. Guard: the domain must genuinely bound below 2^32 (byte
        // offsets, entity indexes, tick counters in long sessions need
        // audit). Type ripple is caller-wide — suggestion-quality rewrite,
        // no proposer (the Batch 58 signature-change lesson).
        PerfRule {
            rule_id: "usize-index-to-u32".to_string(),
            name: "usize index/offset fields in high-volume structs → u32 (halve cache footprint)".to_string(),
            category: "layout".to_string(),
            buggy_code: "struct Span { start: usize, end: usize } // 16 bytes — pointer-sized for a < 4GB domain\nfn total_len(spans: &[Span]) -> u64 {\n    let mut acc = 0;\n    for s in spans { acc += (s.end - s.start) as u64; }\n    acc\n}".to_string(),
            fixed_code: "struct Span { start: u32, end: u32 } // 8 bytes — same domain, half the cache line traffic\nfn total_len(spans: &[Span]) -> u64 {\n    let mut acc = 0;\n    for s in spans { acc += u64::from(s.end - s.start); }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(usize_index_to_u32_bench_spec()),
        },
        // ── simd: transposed-weight register GEMV ─────────────────────────────────────
        // Source: microGPT-C (MIT, Research 070, Batch 62). The row-major
        // matvec dots every row into ONE scalar accumulator and pays a
        // horizontal reduction per row (the in-tree `binary.rs`
        // `vaddvq_f32(acc0)` shape); packing weights transposed once at
        // weight-freeze lets each input element broadcast and FMA into
        // output-vector accumulators — zero horizontal reduction, sequential
        // weight streaming, output registers held across the column loop
        // (the ryg 2015 transposed-lane Mv analysis). Distinct from the
        // kernel_opt GPU plane-per-row family (`workgroup-equals-plane-
        // native-reduction`, `single-plane-sufficient-reduction`): those are
        // GPU workgroup/shuffle concerns — this is the CPU SIMD register
        // twin. Architectural (weight storage packs at model-load; per-call
        // `&[f32]` params cannot pack in place) — suggestion-quality, no
        // proposer (the `soa-hot-field` class). Tail tiles 16/8/4 rows keep
        // lanes full at non-multiple nout.
        PerfRule {
            rule_id: "transposed-weight-register-gemv".to_string(),
            name: "Row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA (register GEMV)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// Row-major GEMV: each row is a dot product into ONE scalar\nfn matvec(w: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n    for r in 0..nout {\n        let row = &w[r * nin..(r + 1) * nin];\n        let mut acc = 0.0f32;\n        for c in 0..nin {\n            acc += row[c] * x[c]; // horizontal reduce at the end of EVERY row\n        }\n        y[r] = acc;\n    }\n}".to_string(),
            fixed_code: "// Transposed (column-major) weights: each input element x[c] broadcasts\n// once and FMAs into the whole output vector — lane-parallel, ZERO\n// horizontal reduction anywhere. Pack weights transposed ONCE at\n// weight-freeze; the pack cost amortizes over every call.\nfn matvec(w_t: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n    for r in 0..nout { y[r] = 0.0; }\n    for c in 0..nin {\n        let xc = x[c]; // one load, nout lane FMAs\n        let col = &w_t[c * nout..(c + 1) * nout];\n        for r in 0..nout {\n            y[r] += xc * col[r]; // auto-vectorizes: no per-row reduce\n        }\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(transposed_weight_register_gemv_bench_spec()),
        },
        // ── arithmetic: Newton refinement loses to hardware on CPU ───────────
        // Source: microGPT-C PERFORMANCE.md dead-ends (Research 070,
        // Batch 62) — the honest-annotation class (`filter-count-to-any`
        // precedent): the rule exists to STOP a plausible-looking GPU→CPU
        // port. On GPU, div/sqrt are multi-cycle → approximate + refine
        // wins; on CPU they are single-instruction + correctly rounded →
        // every Newton step is pure extra work (source measured 2-4%
        // slower on Ryzen 5 5600H). Distinct from
        // `regularizer-above-noise-floor-coupled-to-damping-cap` (that is
        // Newton-Schulz ITERATION robustness — different concern). The
        // relaxed-precision-estimate axis (`rsqrtss` 12-bit / `frecpe`) is
        // a SEPARATE valid optimization where accuracy can relax — never
        // suggest replacing precision-required divs.
        PerfRule {
            rule_id: "newton-refinement-vs-hardware-transcendental".to_string(),
            name: "Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction div/sqrt on CPU".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "// GPU-ported fast-math recip + rsqrt: seed + Newton refinement steps.\n// On GPU this wins (div/sqrt are multi-cycle → approx + refine);\n// on CPU the hardware op is single-instruction + correctly rounded,\n// so every refinement step is pure extra work.\nfn fast_inv(x: f32) -> f32 {\n    let r = 1.0 / x; // seed (estimate-tier on GPU)\n    let r = r * (2.0 - x * r); // Newton step 1\n    r * (2.0 - x * r) // Newton step 2\n}\nfn fast_rsqrt(x: f32) -> f32 {\n    let r = 1.0 / x.sqrt();\n    r * (0.5 * (3.0 - x * r * r)) // one Newton refinement\n}".to_string(),
            fixed_code: "// On CPU, `fdiv`/`fsqrt` are single-instruction and correctly rounded\n// — the refinement chain is pure extra work (measured 2-4% slower\n// than plain division on modern x86/ARM). Use the hardware op directly.\nfn fast_inv(x: f32) -> f32 { 1.0 / x }\nfn fast_rsqrt(x: f32) -> f32 { 1.0 / x.sqrt() }".to_string(),
            context_notes: Some(
                "Mixed (Batch 62 bench, M3 Max): hardware `fdiv`/`fsqrt` never \
                 lose to the refinement chain — 1.09x at 1K, 1.01x at 100K \
                 (source measured 2-4% on Ryzen 5 5600H). On CPU div/sqrt are \
                 single-instruction + correctly rounded, so every Newton step is \
                 pure extra work; the verdict INVERTS on GPU (multi-cycle \
                 div/sqrt → approximate+refine wins). Suggest the hardware op on \
                 CPU hot paths; never port GPU fast-math refinement chains. The \
                 relaxed-precision-estimate axis (12-bit estimate intrinsics) is a \
                 separate optimization valid ONLY where accuracy can relax — not \
                 for precision-required divs. x86 re-gate (2026-08-19, 4090 box, quiet): never-loses is CROSS-ARCH and the win is LARGER there — ParetoDominant 1.64x/1.66x at 1K/100K; the near-tie is M3-aarch64-specific (fdiv latency, pipeline-hidden at 100K)."
                .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(newton_refinement_vs_hardware_bench_spec()),
        },
        // ── simd: loop-order-innermost-contiguous ──
        // Source: Arun Nagaraj, 'Matrix Multiplication in Rust with
        // SIMD: From Naive to 126x Faster' (medium @aruntamiln
        // 2026-02-12; repo ArunTamil21/matmat-simd typo ArunTamil21/matmul-simd;
        // x86 AVX measured, technique ISA-independent — it is
        // memory-hierarchy, not ISA). The i-j-k nest's INNERMOST loop
        // (k) strides `b`'s OUTER axis (`b[k][j]` at stride-n) — every
        // access a new cache line, L1 misses 45%->12% after the reorder.
        // Distinct from `transposed-weight-register-gemv` (B62: repacks
        // the WEIGHTS column-major for GEMV — that changes the data
        // layout; this reorders the LOOPS over an unchanged layout, the
        // no-repack general nested-accumulation case), `simd-chunk`
        // (chunking for vectorization width), `layout-reorder` (struct
        // field padding), `soa-hot-field` (AoS->SoA). Legality: no
        // loop-carried deps across the swapped axes (the `c[i][j] +=`
        // accumulation shape is the canonical legal case). Float
        // sum-order changes under the reorder — rel-error tolerance
        // gate, NOT bit-identity (the `distance-sq-no-sqrt` precedent).
        PerfRule {
            rule_id: "loop-order-innermost-contiguous".to_string(),
            name: "i-j-k nested accumulation with a column-striding inner access → i-k-j reorder (innermost varies the LAST index)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// i-j-k nest: the INNERMOST loop (k) walks b[k][j] COLUMN-wise at stride-n\nfn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n    for i in 0..n {\n        for j in 0..n {\n            for k in 0..n {\n                c[i * n + j] += a[i * n + k] * b[k * n + j]; // stride-n inner\n            }\n        }\n    }\n}".to_string(),
            fixed_code: "// i-k-j reorder: the innermost loop (j) varies the LAST index — a\n// CONTIGUOUS row walk through b, c, and the hoisted a row anchor.\nfn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n    for i in 0..n {\n        let a_row = &a[i * n..(i + 1) * n]; // hoist the row anchor\n        for k in 0..n {\n            let a_ik = a_row[k];\n            for j in 0..n {\n                c[i * n + j] += a_ik * b[k * n + j]; // j strides by 1\n            }\n        }\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(loop_order_innermost_contiguous_bench_spec()),
        },
        // ── complexity: debug-assert-length-hoist ──
        // Source: Shnatsel, 'How to avoid bounds checks in Rust
        // (without unsafe!)' (2023-01-17, the canonical article;
        // repo Shnatsel/bounds-check-cookbook). A `debug_assert!`
        // mentioning `.len()` is COMPILED OUT in release — dropping
        // exactly the length constraint the optimizer needed to
        // eliminate the subsequent bounds checks, so release retains
        // ALL of them ('Ironically, this makes the code slower'). The
        // fix is the macro swap: `assert!` survives release, and
        // redundant asserts are optimizer-removed ('you cannot have
        // too many'). The counter-intuition is the value: `debug_assert!`
        // FEELS free (it vanishes!) but its vanishing is the pessimization.
        // Sibling fact encoded: an `assert!` in front of the hot loop is
        // the cheapest bounds-check hoist (check once, not per-iteration).
        // Honest expectation (annotation): aarch64 gains are the 1.03x
        // class (the 15% headline was an x86 unrolling secondary
        // effect) — the rule exists to stop the release-mode
        // pessimization, not to promise throughput.
        PerfRule {
            rule_id: "debug-assert-length-hoist".to_string(),
            name: "debug_assert! mentioning .len() is erased in release (dropping the constraint bounds-check elision needed) → assert!".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// debug_assert! is ERASED in release — the optimizer loses exactly the\n// range constraint it needed to eliminate the loop's bounds checks\nfn checksum(rows: &[(u64, u64)], first: usize, count: usize) -> u64 {\n    debug_assert!(first + count <= rows.len());\n    let mut acc = 0;\n    for i in first..first + count {\n        acc ^= rows[i].0.wrapping_add(rows[i].1); // release keeps the bounds check\n    }\n    acc\n}".to_string(),
            fixed_code: "// assert! SURVIVES release: the range constraint stays visible and the\n// bounds checks are elided (\"you cannot have too many asserts\" — the\n// redundant ones are optimizer-removed, never re-checked)\nfn checksum(rows: &[(u64, u64)], first: usize, count: usize) -> u64 {\n    assert!(first + count <= rows.len());\n    let mut acc = 0;\n    for i in first..first + count {\n        acc ^= rows[i].0.wrapping_add(rows[i].1); // checks elided in release\n    }\n    acc\n}".to_string(),
            context_notes: Some(
                "Mixed, regime-split (Batch 64 bench, M3 Max aarch64): the \
                 canonical range-pinned shape measures 1.13x at n=1K \
                 (elision win) but a STABLE 0.67-0.71x LOSS at n=100K \
                 (3-run verified) — on aarch64 the swap is shape- and \
                 scale-dependent, NOT a free win; the article's own aarch64 \
                 runs measured 1.03x (the 15% headline was an x86 unrolling \
                 secondary effect). Scope: the assert's constraint must be \
                 PROPAGATABLE to the indexed accesses (parameter/range-pinned \
                 indices) — a `debug_assert!` over a DATA-DEPENDENT index \
                 array can never enable elision (the compiler cannot \
                 propagate through runtime index values), so the swap there \
                 is pure cost. The rule's VALUE is stopping the \
                 counter-intuitive release-mode pessimization \
                 (`debug_assert!` feels free — its vanishing drops the \
                 length constraint bounds-check elision needed) while warning \
                 that the fix itself needs measuring per shape. Suggest \
                 `assert!` (redundant asserts are optimizer-removed) on \
                 range-pinned shapes; MEASURE before expecting throughput. x86 re-gate (2026-08-19, 4090 box, quiet): regime CONFIRMED cross-arch — 1.00x tie at 1K, 0.92x at 100K (milder than aarch64's 0.67-0.71x but still no free win; the article's x86 headline does not reproduce on this harness shape either).".to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(debug_assert_length_hoist_bench_spec()),
        },
        // ── arithmetic: sparse set-bit iteration — trailing_zeros + BLSR ──
        // Source: polars kernel idioms (Batch 66, Research 073 —
        // polars-compute/src/filter/boolean.rs + polars-arrow bitmap
        // TrueIdxIter). The buggy shape tests ALL 64 bit positions with a
        // shift+mask each; the fixed loop consumes the least significant
        // set bit per iteration (trailing_zeros + `m &= m.wrapping_sub(1)`
        // — the BLSR instruction where available) so it runs popcount(m)
        // iterations, not 64 — a ~16x iteration cut at 4/64 density. The
        // compiler cannot derive the tz-loop from the shift-test loop (the
        // trip count is data-dependent). Both forms emit identical indices.
        //
        // ANTI-SHAPE (Batch 68 rider, Research 075 — tantivy
        // common/src/bitset.rs `TinySet::pop_lowest`): code that ALREADY
        // iterates by trailing_zeros but clears with `x ^= 1 << lowest` is
        // ALSO this rule's territory — the xor-clear SERIALIZES the count
        // and the clear (the shift needs the tz result: ARM64
        // rbit→clz→lsl→eor, x86 tzcnt→btc), while `x &= x - 1` (Kernighan)
        // makes the clear INDEPENDENT of the count (ARM64 sub→and, x86
        // blsr — tzcnt runs in parallel; tantivy's in-source godbolt:
        // godbolt.org/z/fnfrP1T5f). Same indices, same popcount trips —
        // only the dependency chain changes.
        PerfRule {
            rule_id: "sparse-set-bit-iteration-tz-blsr".to_string(),
            name: "64-iteration set-bit scan → trailing_zeros + BLSR loop".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn sum_set_positions(mask: u64) -> u32 {\n    let mut sum = 0;\n    for i in 0..64 {\n        if (mask >> i) & 1 != 0 {\n            sum += i as u32;\n        }\n    }\n    sum\n}".to_string(),
            fixed_code: "fn sum_set_positions(mask: u64) -> u32 {\n    let mut sum = 0;\n    let mut m = mask;\n    while m != 0 {\n        let i = m.trailing_zeros(); // TZCNT\n        sum += i;\n        m &= m.wrapping_sub(1); // BLSR: clear least significant set bit\n    }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sparse_set_bit_iteration_bench_spec()),
        },
        // ── iterators: sortedness early-exit → branchless accumulate ────
        // Source: polars SortedSlice::is_sorted_ascending (Batch 66,
        // Research 073 — polars-utils/src/slice.rs: "don't early stop or
        // branch so it autovectorizes"). The buggy early-exit is the
        // natural idiom but the per-element branch defeats vectorization;
        // the accumulate form scans all N as compare+AND and
        // auto-vectorizes. The mirrored regime of `filter-count-to-any`
        // (same vectorization mechanism, opposite direction: REMOVING an
        // early exit buys vectorization when the predicate rarely fires).
        PerfRule {
            rule_id: "sortedness-check-branchless-accumulate".to_string(),
            name: "sortedness early-exit validation → branchless accumulate full scan".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn validate_sorted(vals: &[u64]) -> bool {\n    let mut prev = u64::MIN;\n    for &v in vals {\n        if v < prev {\n            return false; // early exit blocks vectorization\n        }\n        prev = v;\n    }\n    true\n}".to_string(),
            fixed_code: "fn validate_sorted(vals: &[u64]) -> bool {\n    let mut sorted = true;\n    let mut prev = u64::MIN;\n    for &v in vals {\n        sorted &= prev <= v; // no branch: compare + AND, auto-vectorizes\n        prev = v;\n    }\n    sorted\n}".to_string(),
            context_notes: Some(
                "Regime-split (Batch 66 bench, M3 Max aarch64, load-recorded ~4-6): ParetoDominant 2.98x@100K / 2.72x@1M on USUALLY-SORTED data — the early-exit branch never fires but still forces scalar compare+branch per element, while the accumulate form vectorizes (batched compare+AND). On data with an EARLY violation the branchless form is ParetoDominated (it scans all N; the early exit stops at the first violation) — the mirrored regime of `filter-count-to-any`. Suggest only for usually-sorted validation (post-sort invariant checks, monotonic timestamps); prefer the early-exit form when violations are expected."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sortedness_check_branchless_bench_spec()),
        },
        // ── iterators: byte-find position scan → memchr primitive ──────
        // Source: regex-automata (Batch 67, Research 074 — rust-lang/regex,
        // regex-automata/src/util/memchr.rs + dfa/accel.rs; the engine's own
        // naive fallback IS this rule's buggy shape). `xs.iter().position(|&b| b == lit)`
        // on a `&[u8]` cannot dispatch to a byte-search primitive — the predicate
        // is opaque to the iterator machinery + early-exit blocks vectorization —
        // so std runs a scalar compare-per-byte scan. The memchr crate's
        // memchr/memrchr (and 2/3-needle variants for `||` chains, the engine's
        // exact arity ladder) scan 16-32B blocks with SIMD + movemask. Same
        // `Option<usize>` — bit-identical. EXCLUDES `xs.contains(&b)`: std
        // ALREADY specializes contains for u8 via `SliceContains` -> core's SWAR
        // memchr (core/src/slice/cmp.rs) — contains is already fast; the
        // position/rposition shape is where the win lives (the asymmetry IS the
        // rule). Dep-gated (the fxhashmap precedent): fire only when the target
        // manifest declares memchr, else suggestion-only. Regime: wash below
        // ~16-32B haystacks (call overhead eats the win; the engine's own
        // fallback concedes naive is fine small).
        PerfRule {
            rule_id: "byte-find-position-to-memchr".to_string(),
            name: "u8 slice iter().position(|&b| b == lit) → memchr::memchr".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn find_delim(buf: &[u8]) -> Option<usize> {\n    buf.iter().position(|&b| b == b';')\n}".to_string(),
            fixed_code: "fn find_delim(buf: &[u8]) -> Option<usize> {\n    memchr::memchr(b';', buf) // SIMD block scan + movemask early exit\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(byte_find_position_to_memchr_bench_spec()),
        },
        // ── io: per-byte Write::write_all varint loop → ladder compose + one call ──
        // Source: tantivy (Batch 68, Research 075 — common/src/vint.rs
        // `write_u32_vint`/`serialize_vint_u32`; coordinator quote-verified
        // 2026-08-19). **MEASURED HONEST LOSER on this box (M3 Max aarch64,
        // load ~25-33, 3 regimes: Vec-push / inline-Cursor writer /
        // Box<dyn Write> vtable writer — ParetoDominated 0/3 sizes in EACH,
        // alloc axis equal + stable, outputs byte-identical).** The folk
        // mechanism story dies twice: (1) 128 is a compile-time CONSTANT —
        // LLVM strength-reduces `x % 128` / `x / 128` to `& 127` / `>> 7`
        // (the Batch 59 two-regime note applied in reverse — there is no
        // hardware-div chain to eliminate), and (2) even N vtable calls
        // into a dyn writer are hidden by out-of-order execution behind the
        // ladder's 5-deep branch chain. The entry stays as an ANNOTATED
        // honest finding (the `precompute-table` class): it exists to STOP
        // the plausible-looking "compose into a word, store once" port
        // from tantivy's codebase — measured, it REGRESSES on modern
        // aarch64 AND on x86 (re-gated 2026-08-19 on the 4090 box: dominated
        // 0/3 there too, 0.40-0.50x — HARDER than aarch64). Regime facts
        // that remain true: the ladder
        // is convention-faithful + bit-identical (exhaustive gate below),
        // and the u64 width boundary (10 bytes > word) keeps the loop.
        PerfRule {
            rule_id: "varint-compose-word-single-store".to_string(),
            name: "Per-byte write_all varint loop (N writer calls) → threshold-ladder compose + single write_all".to_string(),
            category: "io".to_string(),
            buggy_code: "use std::io::{Write, Result};\nfn write_vint<W: Write>(w: &mut W, mut val: u32) -> Result<()> {\n    loop {\n        let byte = (val % 128) as u8;\n        val /= 128;\n        if val == 0 {\n            w.write_all(&[byte | 128])?; // a writer call PER BYTE\n            return Ok(());\n        }\n        w.write_all(&[byte])?;\n    }\n}".to_string(),
            fixed_code: "use std::io::{Write, Result};\nfn write_vint<W: Write>(w: &mut W, val: u32) -> Result<()> {\n    let v = u64::from(val);\n    // compose ALL bytes into one u64 (masked shifts), ONE writer call\n    let (res, len) = if v < (1 << 7) {\n        (v | 128, 1)\n    } else if v < (1 << 14) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)\n    } else if v < (1 << 21) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)\n    } else if v < (1 << 28) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)\n    } else {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)\n    };\n    w.write_all(&res.to_le_bytes()[..len]) // ONE store, ONE call\n}".to_string(),
            context_notes: Some(
                "ParetoDominated (Batch 68 bench, M3 Max aarch64, load-recorded ~25-33, THREE regimes tested): the ladder LOSES 0/3 sizes on the Vec-push shape AND the inline-Cursor writer shape AND the Box<dyn Write> vtable shape — alloc axis equal + stable, outputs byte-identical. The folk mechanism story dies twice: (1) 128 is a compile-time constant, so LLVM strength-reduces x % 128 / x / 128 to & 127 / >> 7 — there is NO hardware-div chain to eliminate (the division-by-invariant-multiplication two-regime note applied in reverse); (2) even N dyn-dispatch writer calls are hidden by out-of-order execution behind the ladder's 5-deep branch chain. The rule exists to STOP the plausible-looking 'compose into a word, store once' port from tantivy's codebase — measured, it REGRESSES on modern aarch64 AND x86 (4090-box re-gate 2026-08-19, quiet, load 10: dominated 0/3 there TOO, HARDER — 0.40x/0.43x/0.50x at 1K/100K/1M, the ladder 2.0-2.5x slower than the per-byte loop; alloc axis equal, G-det stable). Bit-identity remains proven (exhaustive gate: full u16 + 128^i boundaries + LCG u32, both conventions)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(varint_compose_word_single_store_bench_spec()),
        },
        // ── io: shift/or byte-assembly word read → one unaligned from_le_bytes load ──
        // Source: tantivy bitpacker (Batch 69, Research 075 §Secondary — the
        // Plan 013 T6 defer MEASURED-REFUTED: the varint honest-loser was
        // CALL-COUNT (OoO hides N dyn-dispatch writer calls); this is
        // LOAD-COUNT (8 separately-loaded bytes vs 1 fused unaligned load)
        // — a different mechanism class, and OoO cannot hide 8 L1 loads
        // behind 1. LLVM does NOT fuse the shift/or assembly chain into a
        // byte copy (the dependency chain + per-byte bounds checks block
        // it). Cross-arch probe 2026-08-19 (M3 aarch64 load ~118 interleaved
        // + 4090-box x86 load 7-31, median-of-5): u64 2.17-2.23x / 1.39-1.47x,
        // u32 1.27-1.38x / 1.20-1.29x — ParetoDominant BOTH arches BOTH
        // widths, G1 bit-identical at EVERY byte alignment. u16 excluded
        // (2 bytes, unmeasured, diminishing). The bit-packed variant
        // (tantivy `BitUnpacker::get`: + shift+mask after the load) is the
        // same rule applied further along.
        PerfRule {
            rule_id: "unaligned-word-load-from-bytes".to_string(),
            name: "Shift/or byte-assembly word read (8 loads + 7 shifts + 7 ors) → one unaligned slice from_le_bytes load".to_string(),
            category: "io".to_string(),
            buggy_code: "fn read_u64_le(data: &[u8], addr: usize) -> u64 {\n    // 8 indexed loads + 7 shifts + 7 ors — a serial assembly chain\n    (data[addr] as u64)\n        | ((data[addr + 1] as u64) << 8)\n        | ((data[addr + 2] as u64) << 16)\n        | ((data[addr + 3] as u64) << 24)\n        | ((data[addr + 4] as u64) << 32)\n        | ((data[addr + 5] as u64) << 40)\n        | ((data[addr + 6] as u64) << 48)\n        | ((data[addr + 7] as u64) << 56)\n}".to_string(),
            fixed_code: "fn read_u64_le(data: &[u8], addr: usize) -> u64 {\n    // ONE unaligned 8-byte load — LLVM lowers from_le_bytes over a\n    // slice copy to a single unaligned u64 load\n    let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();\n    u64::from_le_bytes(bytes)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(unaligned_word_load_from_bytes_bench_spec()),
        },
    ])
}

// ── Bench specs ────────────────────────────────────────────────────────────
//
// Each spec is a self-contained Rust source fragment embedding buggy + fixed
// code inline + defining `make_input` / `run_buggy` / `run_fixed`. The
// validator wraps this with a timing loop.
//
// Convention for external deps: prefix with `// [dep: crate_name]` — the
// validator strips these markers + adds them to the generated Cargo.toml.

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Bench spec for `o1-lookup`: worst-case Vec scan (last element) vs HashMap O(1).
#[cfg(feature = "rust_perf_bench")]
fn o1_lookup_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10, 100, 1_000, 10_000],
        harness: r#"
use std::collections::HashMap;

mod buggy {
    pub fn find_user(users: &Vec<(u32, String)>, id: u32) -> Option<&String> {
        users.iter().find(|(uid, _)| *uid == id).map(|(_, n)| n)
    }
}

mod fixed {
    use std::collections::HashMap;
    pub fn find_user(users: &HashMap<u32, String>, id: u32) -> Option<&String> {
        users.get(&id)
    }
}

pub struct Input {
    vec: Vec<(u32, String)>,
    map: HashMap<u32, String>,
}

pub fn make_input(n: usize) -> Input {
    let vec: Vec<(u32, String)> = (0..n).map(|i| (i as u32, format!("user{i}"))).collect();
    let map: HashMap<u32, String> = vec.iter().cloned().collect();
    Input { vec, map }
}

pub fn run_buggy(input: &Input) {
    // Worst case: search for the last element (full scan).
    // black_box the key to prevent the compiler from hoisting the comparison.
    let id = std::hint::black_box((input.vec.len() - 1) as u32);
    std::hint::black_box(buggy::find_user(&input.vec, id));
}

pub fn run_fixed(input: &Input) {
    let id = std::hint::black_box((input.vec.len() - 1) as u32);
    std::hint::black_box(fixed::find_user(&input.map, id));
}
"#
        .to_string(),
    }
}

/// Bench spec for `bsearch-sorted-slice`: O(n) linear scan vs O(log n) binary search.
///
/// Both functions search a sorted `&[u32]` for a target that is NOT present
/// (worst case for the linear scan — full pass). Binary search examines only
/// log₂(n) elements. The compiler cannot transform the linear `any()` into a
/// binary search, so the win is real at all sizes ≥ 100.
#[cfg(feature = "rust_perf_bench")]
fn bsearch_sorted_slice_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn contains(sorted: &[u32], target: u32) -> bool {
        sorted.iter().any(|&x| x == target)
    }
}

mod fixed {
    pub fn contains(sorted: &[u32], target: u32) -> bool {
        sorted.binary_search(&target).is_ok()
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Sorted ascending 0,1,2,...,n-1
    (0..n as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    // Worst case: target not present (full O(n) scan).
    let target = std::hint::black_box(input.len() as u32);
    std::hint::black_box(buggy::contains(input, target));
}

pub fn run_fixed(input: &[u32]) {
    let target = std::hint::black_box(input.len() as u32);
    std::hint::black_box(fixed::contains(input, target));
}
"#
        .to_string(),
    }
}

/// Bench spec for `dedupe-hashset-vs-vec`: O(n²) growing-Vec dedup vs O(n) HashSet.
///
/// The buggy path builds a unique list by repeatedly calling `Vec::contains`
/// on a growing vector — O(n²) total comparisons. The fixed path collects
/// into a HashSet in O(n) average. Input has ~n/2 unique values repeated
/// twice, ensuring the growing-Vec scan has meaningful work at every step.
/// Sizes capped at 5_000 because the O(n²) buggy path dominates wall-clock
/// at larger n (~6 ms at 5_000; ~25 ms at 10_000).
#[cfg(feature = "rust_perf_bench")]
fn dedupe_hashset_vs_vec_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 500, 1_000, 5_000],
        harness: r#"
mod buggy {
    pub fn unique(items: &[u32]) -> Vec<u32> {
        let mut seen = Vec::new();
        for &x in items {
            if !seen.contains(&x) {
                seen.push(x);
            }
        }
        seen
    }
}

mod fixed {
    use std::collections::HashSet;
    pub fn unique(items: &[u32]) -> Vec<u32> {
        let seen: HashSet<u32> = items.iter().copied().collect();
        seen.into_iter().collect()
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Half duplicates: values cycle through 0..(n/2), repeated twice.
    // This produces O(n²) behavior in the buggy path (every element
    // triggers a Vec scan of growing length).
    (0..n).map(|i| (i % (n / 2)) as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let v = buggy::unique(input);
    std::hint::black_box(v);
}

pub fn run_fixed(input: &[u32]) {
    let v = fixed::unique(input);
    std::hint::black_box(v);
}
"#
        .to_string(),
    }
}

/// Bench spec for `static-str`: String allocation vs &'static str return.
///
/// Each call returns a label; the bench runs the function in a loop. The
/// `String` version allocates each call; the `&'static str` version is a
/// pointer return. The perf delta is the allocation cost.
#[cfg(feature = "rust_perf_bench")]
fn static_str_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 10, 100], // size doesn't matter much; function is constant
        harness: r#"
mod buggy {
    pub fn label() -> String { "ready".to_string() }
}

mod fixed {
    pub fn label() -> &'static str { "ready" }
}

pub struct Input {
    pub iters: usize,
}

pub fn make_input(n: usize) -> Input {
    Input { iters: n * 100 } // amplify the per-call cost
}

pub fn run_buggy(input: &Input) {
    // Each call allocates a String + drops it.
    for _ in 0..input.iters {
        let s = buggy::label();
        std::hint::black_box(s);
    }
}

pub fn run_fixed(input: &Input) {
    // Each call returns a &'static str (no allocation).
    for _ in 0..input.iters {
        let s = fixed::label();
        std::hint::black_box(s);
    }
}
"#
        .to_string(),
    }
}

/// Bench spec for `simd-chunk`: scalar sum vs 4-wide chunked sum.
///
/// Batch 66 enhancement note (Research 073 — polars `float_sum.rs`
/// provenance): for FLOAT sums the mechanism is subtler than plain
/// auto-vectorization — strict FP forbids reassociation, so the serial
/// form is a single 3-4-cycle-latency dependency chain LLVM can never
/// vectorize. The chunked form wins because N independent accumulator
/// chains (STRIPE=16 in polars) are each legal sequential FP; the
/// vectorizer interleaves them, and the tree-reduce finisher
/// (`(v[0]+v[2])+(v[1]+v[3])`, shuffle-friendly) collapses the tail
/// without re-introducing a long serial chain. The win GROWS with N —
/// the same mechanism documented for `transposed-weight-register-gemv`.
#[cfg(feature = "rust_perf_bench")]
fn simd_chunk_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn sum(v: &[f32]) -> f32 {
        let mut s = 0.0;
        for i in 0..v.len() { s += v[i]; }
        s
    }
}

mod fixed {
    pub fn sum(v: &[f32]) -> f32 {
        let chunks = v.chunks_exact(4);
        let rem = v.chunks_exact(4).remainder();
        let mut acc = [0.0f32; 4];
        for c in chunks {
            acc[0] += c[0]; acc[1] += c[1]; acc[2] += c[2]; acc[3] += c[3];
        }
        acc.iter().chain(rem).copied().sum()
    }
}

pub fn make_input(n: usize) -> Vec<f32> {
    // Pseudo-random but deterministic data (avoid all-zeros which the
    // compiler might specialize away).
    (0..n).map(|i| ((i as u32).wrapping_mul(2654435761) as f32) / 1000.0).collect()
}

pub fn run_buggy(input: &[f32]) {
    let s = buggy::sum(input);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &[f32]) {
    let s = fixed::sum(input);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `bool-to-bitpack`: Vec<bool> count vs packed Vec<u64> POPCNT.
///
/// The buggy path iterates a `Vec<bool>` (1 byte per element) and counts set
/// bits via `filter().count()`. The fixed path packs bools into `Vec<u64>`
/// (1 bit per element) and counts via `count_ones()` (POPCNT instruction).
/// The compiler cannot auto-pack bools into bits — this is a data-density
/// transformation the programmer must make explicitly. The win comes from
/// 8× less memory + hardware POPCNT (1 instruction per 64 bools vs per-byte
/// branch).
#[cfg(feature = "rust_perf_bench")]
fn bool_to_bitpack_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn count_true(flags: &[bool]) -> usize {
        flags.iter().filter(|&&b| b).count()
    }
}

mod fixed {
    pub fn count_true(bits: &[u64]) -> usize {
        bits.iter().map(|w| w.count_ones() as usize).sum()
    }
}

pub struct Input {
    bools: Vec<bool>,
    bits: Vec<u64>,
}

pub fn make_input(n: usize) -> Input {
    // ~67% set bits, pseudo-random but deterministic.
    let bools: Vec<bool> = (0..n)
        .map(|i| (i as u32).wrapping_mul(2654435761) % 3 != 0)
        .collect();
    // Pack bools into u64s.
    let mut bits = vec![0u64; (n + 63) / 64];
    for (i, &b) in bools.iter().enumerate() {
        if b {
            bits[i / 64] |= 1u64 << (i % 64);
        }
    }
    Input { bools, bits }
}

pub fn run_buggy(input: &Input) {
    let c = buggy::count_true(&input.bools);
    std::hint::black_box(c);
}

pub fn run_fixed(input: &Input) {
    let c = fixed::count_true(&input.bits);
    std::hint::black_box(c);
}
"#
        .to_string(),
    }
}

/// Bench spec for `mutex-to-atomic`: Mutex<u32> inc loop vs AtomicU32 fetch_add.
///
/// The buggy path locks + unlocks a `Mutex<u32>` per increment. Even on the
/// uncontended path, this involves an atomic CAS to acquire + an atomic store
/// to release (plus the `MutexGuard` RAII lifecycle). The fixed path uses a
/// single `AtomicU32::fetch_add(1, Relaxed)` — one `lock xadd` instruction.
/// The compiler cannot eliminate `Mutex::lock` (it's an opaque call).
#[cfg(feature = "rust_perf_bench")]
fn mutex_to_atomic_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

mod buggy {
use std::sync::Mutex;
pub struct Counter(Mutex<u32>);
impl Counter {
    pub fn new() -> Self { Self(Mutex::new(0)) }
    #[inline(never)]
    pub fn inc(&self) { *self.0.lock().unwrap() += 1; }
    pub fn get(&self) -> u32 { *self.0.lock().unwrap() }
}
}

mod fixed {
use std::sync::atomic::{AtomicU32, Ordering};
pub struct Counter(AtomicU32);
impl Counter {
    pub fn new() -> Self { Self(AtomicU32::new(0)) }
    #[inline(never)]
    pub fn inc(&self) { self.0.fetch_add(1, Ordering::Relaxed); }
    pub fn get(&self) -> u32 { self.0.load(Ordering::Relaxed) }
}
}

pub struct Input {
    n: usize,
}

pub fn make_input(n: usize) -> Input {
    Input { n }
}

pub fn run_buggy(input: &Input) {
    let c = buggy::Counter::new();
    for _ in 0..input.n { c.inc(); }
    std::hint::black_box(c.get());
}

pub fn run_fixed(input: &Input) {
    let c = fixed::Counter::new();
    for _ in 0..input.n { c.inc(); }
    std::hint::black_box(c.get());
}
"#
        .to_string(),
    }
}

/// Bench spec for `f32-counter-to-int`: f32 accumulator vs u64 accumulator.
///
/// The buggy path accumulates integer values into an `f32` total. f32
/// addition is NOT associative (IEEE 754 rounding), so the compiler cannot
/// break the sequential dependency chain or use multiple SIMD accumulators
/// without `-C fast-math` (which Rust does not enable by default). Each f32
/// add has 3–4 cycle latency, fully serial. The fixed path uses a `u64`
/// accumulator — integer addition IS associative, so the compiler vectorizes
/// with multiple independent accumulators + 1-cycle add latency.
#[cfg(feature = "rust_perf_bench")]
fn f32_counter_to_int_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn tally(scores: &[u32]) -> f32 {
        let mut total = 0.0f32;
        for &s in scores { total += s as f32; }
        total
    }
}

mod fixed {
    pub fn tally(scores: &[u32]) -> u64 {
        let mut total = 0u64;
        for &s in scores { total += s as u64; }
        total
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Pseudo-random but deterministic values in a realistic range.
    (0..n).map(|i| (i as u32).wrapping_mul(2654435761) % 1000).collect()
}

pub fn run_buggy(input: &[u32]) {
    let t = buggy::tally(input);
    std::hint::black_box(t);
}

pub fn run_fixed(input: &[u32]) {
    let t = fixed::tally(input);
    std::hint::black_box(t);
}
"#
        .to_string(),
    }
}

/// Bench spec for `collect-avoidance`: .collect() intermediate vs chained iterator.
///
/// The buggy path collects an intermediate `Vec<u64>` from the map, then
/// iterates it again for the sum. The collect allocates (heap alloc + copy
/// of all elements) + forces a second pass. The fixed path chains map+sum
/// — zero allocation, single pass. The compiler cannot eliminate a
/// `.collect()` (it must produce a concrete `Vec`).
#[cfg(feature = "rust_perf_bench")]
fn collect_avoidance_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn transform_sum(vals: &[u64]) -> u64 {
        let mapped: Vec<u64> = vals.iter().map(|x| x.wrapping_mul(2)).collect();
        mapped.iter().sum()
    }
}

mod fixed {
    pub fn transform_sum(vals: &[u64]) -> u64 {
        vals.iter().map(|x| x.wrapping_mul(2)).sum()
    }
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n as u64).collect()
}

pub fn run_buggy(input: &[u64]) {
    let s = buggy::transform_sum(input);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &[u64]) {
    let s = fixed::transform_sum(input);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `dyn-to-enum`: Box<dyn Trait> vs enum dispatch.
///
/// The buggy path uses `Box<dyn Op>` — each `.apply()` is a vtable load +
/// indirect call. The compiler cannot inline through an opaque trait object
/// or devirtualize when the concrete type is erased. The fixed path uses an
/// enum: the match arms are direct calls the compiler can inline + the
/// branch predictor learns the match arm pattern.
#[cfg(feature = "rust_perf_bench")]
fn dyn_to_enum_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    trait Op {
        fn apply(&self, x: f64) -> f64;
    }
    struct Scale(f64);
    impl Op for Scale { fn apply(&self, x: f64) -> f64 { x * self.0 } }
    struct Offset(f64);
    impl Op for Offset { fn apply(&self, x: f64) -> f64 { x + self.0 } }
    struct Square;
    impl Op for Square { fn apply(&self, x: f64) -> f64 { x * x } }

    fn make_ops() -> Vec<Box<dyn Op>> {
        vec![Box::new(Scale(1.1)), Box::new(Offset(0.5)), Box::new(Square)]
    }
    pub fn run(vals: &[f64]) -> f64 {
        let ops = make_ops();
        let mut acc = 0.0f64;
        for &x in vals {
            let mut v = x;
            for op in &ops { v = op.apply(v); }
            acc += v;
        }
        acc
    }
}

mod fixed {
    enum OpEnum { Scale(f64), Offset(f64), Square }
    impl OpEnum {
        fn apply(&self, x: f64) -> f64 {
            match self {
                OpEnum::Scale(s) => x * s,
                OpEnum::Offset(o) => x + o,
                OpEnum::Square => x * x,
            }
        }
    }
    fn make_ops() -> Vec<OpEnum> {
        vec![OpEnum::Scale(1.1), OpEnum::Offset(0.5), OpEnum::Square]
    }
    pub fn run(vals: &[f64]) -> f64 {
        let ops = make_ops();
        let mut acc = 0.0f64;
        for &x in vals {
            let mut v = x;
            for op in &ops { v = op.apply(v); }
            acc += v;
        }
        acc
    }
}

pub fn make_input(n: usize) -> Vec<f64> {
    (0..n).map(|i| ((i as f64) * 0.1).sin()).collect()
}

pub fn run_buggy(input: &[f64]) {
    let r = buggy::run(input);
    std::hint::black_box(r);
}

pub fn run_fixed(input: &[f64]) {
    let r = fixed::run(input);
    std::hint::black_box(r);
}
"#
        .to_string(),
    }
}

/// Bench spec for `entry-vs-contains-insert`: contains_key() + insert() vs entry().
///
/// The buggy path calls `contains_key(k)` then `insert(k, v)` — two hash
/// computations + two probes. The fixed path uses `entry(k).or_insert(v)` —
/// one hash + one probe, in-place insert. The compiler cannot merge the two
/// lookups because `insert` may need to grow the table (opaque call). Input
/// keys have ~50% existing + ~50% new to exercise both branches.
#[cfg(feature = "rust_perf_bench")]
fn entry_vs_contains_insert_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
use std::collections::HashMap;

mod buggy {
use std::collections::HashMap;
pub fn insert_if_absent(map: &mut HashMap<u32, u32>, keys: &[u32]) {
    for &k in keys {
        if !map.contains_key(&k) {
            map.insert(k, k.wrapping_mul(31));
        }
    }
}
}

mod fixed {
use std::collections::HashMap;
pub fn insert_if_absent(map: &mut HashMap<u32, u32>, keys: &[u32]) {
    for &k in keys {
        map.entry(k).or_insert(k.wrapping_mul(31));
    }
}
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Mix of existing + new keys: keys cycle through 0..(n*2), so ~50% hit.
    (0..n).map(|i| (i as u32).wrapping_mul(7) % (n as u32 * 2)).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut map = HashMap::with_capacity(input.len());
    buggy::insert_if_absent(&mut map, input);
    std::hint::black_box(map.len());
}

pub fn run_fixed(input: &[u32]) {
    let mut map = HashMap::with_capacity(input.len());
    fixed::insert_if_absent(&mut map, input);
    std::hint::black_box(map.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `branchless-conditional`: data-dependent if vs branchless multiply.
///
/// The buggy path uses `if cond[i] { sum += data[i] }` — a data-dependent
/// branch inside a hot loop that prevents LLVM auto-vectorization. The fixed
/// path uses `sum += data[i] * cond[i] as u64` — a branchless multiply that
/// LLVM vectorizes into SIMD lanes. The win is primarily vectorization, not
/// branch prediction: even with 100% predictable conditions the branchless
/// version is ~10× faster because the multiply loop vectorizes but the
/// branchy loop cannot.
#[cfg(feature = "rust_perf_bench")]
fn branchless_conditional_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
pub fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {
    let mut sum: u64 = 0;
    for i in 0..data.len() {
        if cond[i] {
            sum += data[i] as u64;
        }
    }
    sum
}
}

mod fixed {
pub fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {
    let mut sum: u64 = 0;
    for i in 0..data.len() {
        sum += data[i] as u64 * cond[i] as u64;
    }
    sum
}
}

pub struct Input {
    data: Vec<u32>,
    cond: Vec<bool>,
}

pub fn make_input(n: usize) -> Input {
    // ~50% true conditions: pseudo-random but deterministic.
    // This is the worst case for branch prediction (unpredictable).
    let data: Vec<u32> = (0..n).map(|i| (i as u32).wrapping_mul(2654435761)).collect();
    let cond: Vec<bool> = (0..n).map(|i| (i.wrapping_mul(12345) % 2) == 0).collect();
    Input { data, cond }
}

pub fn run_buggy(input: &Input) {
    let s = buggy::conditional_sum(&input.data, &input.cond);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &Input) {
    let s = fixed::conditional_sum(&input.data, &input.cond);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `loop-fusion`: two iterator passes vs one fused loop.
///
/// The buggy path uses `.iter().copied().sum()` + `.iter().copied().max()`
/// — two separate iterator consumers, each independently walking the slice.
/// The compiler cannot fuse them into a single pass. The fixed path does
/// both reductions in one explicit `for` loop — one cache pass. Bench 013:
/// ParetoDominant ~1.5× across all sizes (1K → 1M). The win is cache
/// locality at large N (second reduction in the fused loop reads from L1;
/// second pass in the buggy version re-reads from L2/L3 or RAM).
#[cfg(feature = "rust_perf_bench")]
fn loop_fusion_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
pub fn sum_max(data: &[u64]) -> (u64, u64) {
    let sum: u64 = data.iter().copied().sum();
    let max: u64 = data.iter().copied().max().unwrap_or(0);
    (sum, max)
}
}

mod fixed {
pub fn sum_max(data: &[u64]) -> (u64, u64) {
    let mut sum = 0u64;
    let mut max = 0u64;
    for &x in data {
        sum += x;
        if x > max { max = x; }
    }
    (sum, max)
}
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n).map(|i| (i as u64).wrapping_mul(31) % 1000).collect()
}

pub fn run_buggy(input: &[u64]) {
    let r = buggy::sum_max(input);
    std::hint::black_box(r);
}

pub fn run_fixed(input: &[u64]) {
    let r = fixed::sum_max(input);
    std::hint::black_box(r);
}
"#
        .to_string(),
    }
}

/// Bench spec for `prefix-sum-2d`: O(n) 2D region-sum scan vs O(1) prefix sum.
///
/// Tests K=50 random region queries on a sqrt(N)×sqrt(N) grid. The scan reads
/// K×region_area cells; the prefix reads K×4 cells (after a one-time O(N) build
/// in `make_input`). The win grows linearly with N — at N=1K it's ~30×, at
/// N=1M it's ~26,000×.
#[cfg(feature = "rust_perf_bench")]
fn prefix_sum_2d_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
pub fn region_sum(grid: &[Vec<i64>], x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
    let mut sum = 0i64;
    for x in x0..x1 {
        for y in y0..y1 {
            sum += grid[x][y];
        }
    }
    sum
}
}

mod fixed {
pub fn build_prefix(grid: &[Vec<i64>]) -> Vec<Vec<i64>> {
    let w = grid.len();
    let h = grid.first().map(|r| r.len()).unwrap_or(0);
    let mut p = vec![vec![0i64; h + 1]; w + 1];
    for x in 0..w {
        for y in 0..h {
            p[x + 1][y + 1] = grid[x][y] + p[x][y + 1] + p[x + 1][y] - p[x][y];
        }
    }
    p
}
pub fn region_sum(prefix: &[Vec<i64>], x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
    prefix[x1][y1] - prefix[x0][y1] - prefix[x1][y0] + prefix[x0][y0]
}
}

pub struct Input {
    grid: Vec<Vec<i64>>,
    prefix: Vec<Vec<i64>>,
    queries: Vec<(usize, usize, usize, usize)>,
}

pub fn make_input(n: usize) -> Input {
    let dim = ((n as f64).sqrt().round() as usize).max(2);
    let grid: Vec<Vec<i64>> = (0..dim)
        .map(|x| (0..dim).map(|y| ((x * 31 + y * 17) % 100) as i64).collect())
        .collect();
    let prefix = fixed::build_prefix(&grid);
    // K=50 region queries covering ~half the grid each.
    let half = (dim / 2).max(1);
    let queries: Vec<(usize, usize, usize, usize)> = (0..50)
        .map(|i| {
            let x0 = (i * 7) % half;
            let y0 = (i * 13) % half;
            let x1 = (x0 + half).min(dim);
            let y1 = (y0 + half).min(dim);
            (x0, y0, x1, y1)
        })
        .collect();
    Input { grid, prefix, queries }
}

pub fn run_buggy(input: &Input) {
    let mut total = 0i64;
    for &(x0, y0, x1, y1) in &input.queries {
        total = total.wrapping_add(buggy::region_sum(
            std::hint::black_box(&input.grid),
            std::hint::black_box(x0),
            std::hint::black_box(y0),
            std::hint::black_box(x1),
            std::hint::black_box(y1),
        ));
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let mut total = 0i64;
    for &(x0, y0, x1, y1) in &input.queries {
        total = total.wrapping_add(fixed::region_sum(
            std::hint::black_box(&input.prefix),
            std::hint::black_box(x0),
            std::hint::black_box(y0),
            std::hint::black_box(x1),
            std::hint::black_box(y1),
        ));
    }
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `precompute-table`: runtime popcount loop vs const table.
#[cfg(feature = "rust_perf_bench")]
fn precompute_table_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
mod buggy {
    pub fn parity_byte(b: u8) -> u8 {
        (0..8).filter(|i| b & (1 << i) != 0).count() as u8 & 1
    }
}

mod fixed {
    const PARITY: [u8; 256] = {
        let mut t = [0u8; 256];
        let mut i = 0;
        while i < 256 { t[i] = (i.count_ones() & 1) as u8; i += 1; }
        t
    };
    pub fn parity_byte(b: u8) -> u8 { PARITY[b as usize] }
}

pub fn make_input(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i as u8).wrapping_mul(131)).collect()
}

pub fn run_buggy(input: &[u8]) {
    let mut acc: u8 = 0;
    for &b in input {
        acc ^= buggy::parity_byte(b);
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &[u8]) {
    let mut acc: u8 = 0;
    for &b in input {
        acc ^= fixed::parity_byte(b);
    }
    std::hint::black_box(acc);
}
"#
        .to_string(),
    }
}

/// Bench spec for `papaya-lockfree`: RwLock-contended reads vs papaya lock-free.
///
/// Single-threaded read burst — measures the RwLock read-lock overhead vs
/// papaya's epoch-based read. (True multi-threaded contention would need
/// threads, which complicates the harness; the single-threaded path still
/// shows the lock-acquisition cost.)
#[cfg(feature = "rust_perf_bench")]
fn papaya_lockfree_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
// [dep: papaya = "0.2"]
use std::collections::HashMap as StdHashMap;

mod buggy_store {
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
pub struct Store(Arc<RwLock<HashMap<u32, String>>>);
impl Store {
    pub fn new(entries: &[(u32, String)]) -> Self {
        let mut map = HashMap::new();
        for (k, v) in entries {
            map.insert(*k, v.clone());
        }
        Self(Arc::new(RwLock::new(map)))
    }
    pub fn get(&self, id: u32) -> Option<String> {
        self.0.read().unwrap().get(&id).cloned()
    }
}
}

mod fixed_store {
use papaya::HashMap;
pub struct Store(HashMap<u32, String>);
impl Store {
    pub fn new(entries: &[(u32, String)]) -> Self {
        let map = HashMap::new();
        {
            let mut guard = map.pin();
            for (k, v) in entries {
                guard.insert(*k, v.clone());
            }
        }
        Self(map)
    }
    pub fn get(&self, id: u32) -> Option<String> {
        self.0.pin().get(&id).cloned()
    }
}
}

pub struct Input {
keys: Vec<u32>,
buggy: buggy_store::Store,
fixed: fixed_store::Store,
}

pub fn make_input(n: usize) -> Input {
let entries: Vec<(u32, String)> = (0..n)
    .map(|i| (i as u32, format!("user{i}")))
    .collect();
let keys: Vec<u32> = (0..n as u32).collect();
let buggy = buggy_store::Store::new(&entries);
let fixed = fixed_store::Store::new(&entries);
Input { keys, buggy, fixed }
}

pub fn run_buggy(input: &Input) {
let mut acc = 0u64;
for &k in &input.keys {
    if input.buggy.get(k).is_some() {
        acc += 1;
    }
}
std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
let mut acc = 0u64;
for &k in &input.keys {
    if input.fixed.get(k).is_some() {
        acc += 1;
    }
}
std::hint::black_box(acc);
}
"#
    .to_string(),
}
}

/// Bench spec for `vec-with-capacity`: Vec::new() growth vs Vec::with_capacity.
///
/// The buggy path uses `Vec::new()` + push, triggering the growth strategy
/// (1 → 2 → 4 → 8 → … reallocs + copies). The fixed path pre-allocates with
/// `Vec::with_capacity(n)`, doing 1 allocation + 0 extra copies. The win is
/// the realloc + copy cost, which grows with n.
#[cfg(feature = "rust_perf_bench")]
fn vec_with_capacity_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn build(n: usize) -> Vec<u32> {
        let mut v = Vec::new();
        for i in 0..n {
            v.push(i as u32);
        }
        v
    }
}

mod fixed {
    pub fn build(n: usize) -> Vec<u32> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            v.push(i as u32);
        }
        v
    }
}

pub struct Input {
    pub n: usize,
}

pub fn make_input(n: usize) -> Input {
    Input { n }
}

pub fn run_buggy(input: &Input) {
    let v = buggy::build(input.n);
    std::hint::black_box(v);
}

pub fn run_fixed(input: &Input) {
    let v = fixed::build(input.n);
    std::hint::black_box(v);
}
"#
        .to_string(),
    }
}

/// Bench spec for `format-concat-loop`: O(n²) format! concat vs O(n) push_str.
///
/// The buggy path uses `s = format!("{s}{chunk}")` which allocates a new String
/// each iteration + copies the entire accumulated string. The fixed path uses
/// `s.push_str(chunk)` which amortizes to O(n). At n=1000 chunks of ~10 bytes,
/// the buggy path copies ~5MB total vs ~10KB for the fixed path.
#[cfg(feature = "rust_perf_bench")]
fn format_concat_loop_bench_spec() -> BenchSpec {
    BenchSpec {
        // Cap at 1000: the O(n²) buggy path copies ~5MB at n=1000; larger
        // sizes would dominate the bench wall-clock + risk allocator jitter.
        sizes: vec![100, 500, 1_000],
        harness: r#"
mod buggy {
    pub fn build(chunks: &[String]) -> String {
        let mut s = String::new();
        for chunk in chunks {
            s = format!("{s}{chunk}");
        }
        s
    }
}

mod fixed {
    pub fn build(chunks: &[String]) -> String {
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        let mut s = String::with_capacity(total);
        for chunk in chunks {
            s.push_str(chunk);
        }
        s
    }
}

pub fn make_input(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("chunk-{i:04}")).collect()
}

pub fn run_buggy(input: &[String]) {
    let s = buggy::build(input);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &[String]) {
    let s = fixed::build(input);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `ok-or-else`: eager `ok_or(expensive())` vs lazy
/// `ok_or_else(|| expensive())`. The eager form constructs a String for every
/// element even when the Option is Some; the lazy form only on None.
#[cfg(feature = "rust_perf_bench")]
fn ok_or_else_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn expensive_error(n: u32) -> String {
        let mut s = String::with_capacity(64);
        for i in 0..32 {
            s.push_str(&format!("error field {n}-{i}; "));
        }
        s
    }
    pub fn sum_opts(opts: &[Option<u32>]) -> u32 {
        let mut sum = 0u32;
        for o in opts {
            let r = o.ok_or(expensive_error(0));
            sum += r.unwrap_or(0);
        }
        sum
    }
}

mod fixed {
    pub fn expensive_error(n: u32) -> String {
        let mut s = String::with_capacity(64);
        for i in 0..32 {
            s.push_str(&format!("error field {n}-{i}; "));
        }
        s
    }
    pub fn sum_opts(opts: &[Option<u32>]) -> u32 {
        let mut sum = 0u32;
        for o in opts {
            let r = o.ok_or_else(|| expensive_error(0));
            sum += r.unwrap_or(0);
        }
        sum
    }
}

pub fn make_input(n: usize) -> Vec<Option<u32>> {
    // 99% Some, 1% None — realistic distribution.
    (0..n).map(|i| if i % 100 == 0 { None } else { Some(i as u32) }).collect()
}

pub fn run_buggy(input: &[Option<u32>]) {
    std::hint::black_box(buggy::sum_opts(input));
}

pub fn run_fixed(input: &[Option<u32>]) {
    std::hint::black_box(fixed::sum_opts(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `swap-remove`: Vec::remove (O(n) shift) vs Vec::swap_remove (O(1)).
#[cfg(feature = "rust_perf_bench")]
fn swap_remove_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn remove_front(data: &mut Vec<u32>, count: usize) {
        for _ in 0..count.min(data.len()) {
            data.remove(0);
        }
    }
}

mod fixed {
    pub fn remove_front(data: &mut Vec<u32>, count: usize) {
        for _ in 0..count.min(data.len()) {
            data.swap_remove(0);
        }
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

pub fn run_buggy(input: &Vec<u32>) {
    let mut data = input.clone();
    let count = data.len() / 10;
    buggy::remove_front(&mut data, count);
    std::hint::black_box(data);
}

pub fn run_fixed(input: &Vec<u32>) {
    let mut data = input.clone();
    let count = data.len() / 10;
    fixed::remove_front(&mut data, count);
    std::hint::black_box(data);
}
"#
        .to_string(),
    }
}

/// Bench spec for `chunks-exact`: `chunks(8)` vs `chunks_exact(8)` + remainder.
#[cfg(feature = "rust_perf_bench")]
fn chunks_exact_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_chunks(data: &[u32]) -> u64 {
        let mut sum = 0u64;
        for chunk in data.chunks(8) {
            for &v in chunk { sum += v as u64; }
        }
        sum
    }
}

mod fixed {
    pub fn sum_chunks(data: &[u32]) -> u64 {
        let mut sum = 0u64;
        for chunk in data.chunks_exact(8) {
            for &v in chunk { sum += v as u64; }
        }
        for &v in data.chunks_exact(8).remainder() { sum += v as u64; }
        sum
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    std::hint::black_box(buggy::sum_chunks(input));
}

pub fn run_fixed(input: &[u32]) {
    std::hint::black_box(fixed::sum_chunks(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `fxhashmap`: std HashMap (SipHash) lookup vs FxHashMap (FxHash).
///
/// Both maps pre-populated with n entries. The bench sweeps all keys for
/// membership. SipHash does 2 rounds of AES-like mixing; FxHash does 1
/// multiply + xor. ~2-4× faster lookups, ~2× faster inserts.
#[cfg(feature = "rust_perf_bench")]
fn fxhashmap_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
// [dep: rustc-hash = "2"]
use rustc_hash::FxHashMap;
use std::collections::HashMap as StdHashMap;

pub struct Input {
    keys: Vec<u32>,
    std_map: StdHashMap<u32, u32>,
    fx_map: FxHashMap<u32, u32>,
}

pub fn make_input(n: usize) -> Input {
    // Spread keys via a multiplicative hash to avoid sequential hot-spots.
    let keys: Vec<u32> = (0..n).map(|i| (i as u32).wrapping_mul(2654435761)).collect();
    let mut std_map: StdHashMap<u32, u32> = StdHashMap::with_capacity(n);
    let mut fx_map: FxHashMap<u32, u32> = FxHashMap::default();
    fx_map.reserve(n);
    for &k in &keys {
        std_map.insert(k, k);
        fx_map.insert(k, k);
    }
    Input { keys, std_map, fx_map }
}

pub fn run_buggy(input: &Input) {
    let mut sum = 0u64;
    for &k in &input.keys {
        sum += input.std_map.get(&k).copied().unwrap_or(0) as u64;
    }
    std::hint::black_box(sum);
}

pub fn run_fixed(input: &Input) {
    let mut sum = 0u64;
    for &k in &input.keys {
        sum += input.fx_map.get(&k).copied().unwrap_or(0) as u64;
    }
    std::hint::black_box(sum);
}
"#
        .to_string(),
    }
}

/// Bench spec for `retain-inplace`: filter+collect (new Vec) vs retain (in-place).
///
/// The buggy path allocates a new Vec + copies survivors. The fixed path
/// filters in-place — zero allocation. ~1.5-2× at all sizes.
#[cfg(feature = "rust_perf_bench")]
fn retain_inplace_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn filter_out_multiples(data: &[u32]) -> Vec<u32> {
        data.iter().copied().filter(|&x| x % 3 != 0).collect()
    }
}

mod fixed {
    pub fn filter_out_multiples(data: &mut Vec<u32>) {
        data.retain(|&x| x % 3 != 0);
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n).map(|i| i as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    std::hint::black_box(buggy::filter_out_multiples(input));
}

pub fn run_fixed(input: &[u32]) {
    // Clone to simulate the in-place mutation (the caller already owns the Vec).
    let mut data = input.to_vec();
    fixed::filter_out_multiples(&mut data);
    std::hint::black_box(data);
}
"#
        .to_string(),
    }
}

/// Bench spec for `mem-take`: clone+clear vs mem::take for Vec<String> buffer.
///
/// The buggy path deep-clones every String then clears. The fixed path
/// moves the Vec's allocation in O(1). ~3× at all sizes.
#[cfg(feature = "rust_perf_bench")]
fn mem_take_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
pub fn make_input(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("item_{i}")).collect()
}

mod buggy {
    pub fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {
        let taken = buf.clone();
        buf.clear();
        taken
    }
}

mod fixed {
    pub fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {
        std::mem::take(buf)
    }
}

pub fn run_buggy(input: &[String]) {
    let mut buf = input.to_vec();
    std::hint::black_box(buggy::take_buffer(&mut buf));
}

pub fn run_fixed(input: &[String]) {
    let mut buf = input.to_vec();
    std::hint::black_box(fixed::take_buffer(&mut buf));
}
"#
        .to_string(),
    }
}

/// Bench spec for `avoid-clone`: clone(Vec<u8>) vs borrow(&[u8]).
///
/// The buggy path clones the whole Vec (heap alloc + memcpy) every call.
/// The fixed path borrows the slice — zero allocation. ~1.3-3× faster.
#[cfg(feature = "rust_perf_bench")]
fn avoid_clone_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn process(data: Vec<u8>) -> u64 {
        data.iter().map(|&b| b as u64).sum()
    }
}

mod fixed {
    pub fn process(data: &[u8]) -> u64 {
        data.iter().map(|&b| b as u64).sum()
    }
}

pub fn make_input(n: usize) -> Vec<u8> {
    (0..n as u8).cycle().take(n).collect()
}

pub fn run_buggy(input: &[u8]) {
    std::hint::black_box(buggy::process(input.to_vec()));
}

pub fn run_fixed(input: &[u8]) {
    std::hint::black_box(fixed::process(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `box-str-immutable`: Vec<String> vs Vec<Box<str>>.
///
/// String is 3 words (ptr, len, capacity); Box<str> is 2 (ptr, len).
/// The capacity word savings improve cache density at scale. ~1.1-1.3×.
#[cfg(feature = "rust_perf_bench")]
fn box_str_immutable_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
pub struct Input {
    strings: Vec<String>,
    box_strs: Vec<Box<str>>,
}

pub fn make_input(n: usize) -> Input {
    let strings: Vec<String> = (0..n).map(|i| format!("item_{i}")).collect();
    let box_strs: Vec<Box<str>> = strings.iter().map(|s| s.as_str().into()).collect();
    Input { strings, box_strs }
}

pub fn run_buggy(input: &Input) {
    let mut total = 0usize;
    for s in &input.strings {
        total += s.len();
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let mut total = 0usize;
    for s in &input.box_strs {
        total += s.len();
    }
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `reuse-allocation-clear`: alloc-per-call vs clear-reuse.
///
/// The buggy path allocates a fresh Vec::new() inside the function body
/// every call. The fixed path reuses a caller-passed buffer via .clear().
/// **Mixed** — wins at N ≤ 10,000 (up to 5.97× at N=100), loses at N=100,000
/// where the reused buffer exceeds L2 cache.
#[cfg(feature = "rust_perf_bench")]
fn reuse_allocation_clear_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn build_temp(n: usize) -> u64 {
        let mut v: Vec<u32> = Vec::new();
        for i in 0..n {
            v.push(i as u32);
        }
        v.iter().map(|&x| x as u64).sum()
    }
}

mod fixed {
    pub fn build_temp(n: usize, buf: &mut Vec<u32>) -> u64 {
        buf.clear();
        for i in 0..n {
            buf.push(i as u32);
        }
        buf.iter().map(|&x| x as u64).sum()
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    std::hint::black_box(buggy::build_temp(*input));
}

pub fn run_fixed(input: &usize) {
    let mut buf: Vec<u32> = Vec::with_capacity(*input);
    std::hint::black_box(fixed::build_temp(*input, &mut buf));
}
"#
        .to_string(),
    }
}

/// Bench spec for `box-avoid`: Vec<Box<i64>> (heap per elem) vs Vec<i64> (stack).
///
/// The buggy path allocates a Box on the heap for every element. The fixed
/// path stores values inline. ~10-35× at all sizes — the heap alloc dominates.
#[cfg(feature = "rust_perf_bench")]
fn box_avoid_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn build(n: usize) -> Vec<Box<i64>> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n { v.push(Box::new(i as i64)); }
        v
    }
}

mod fixed {
    pub fn build(n: usize) -> Vec<i64> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n { v.push(i as i64); }
        v
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    let v = buggy::build(*input);
    let total: i64 = v.iter().map(|b| **b).sum();
    std::hint::black_box(total);
}

pub fn run_fixed(input: &usize) {
    let v = fixed::build(*input);
    let total: i64 = v.iter().copied().sum();
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `smallvec-stack`: Vec (always heap) vs stack array for ≤8.
///
/// At small N (1, 4, 8), the heap alloc on every call dominates the actual
/// data work. The fixed path uses a stack array for the small case. ~7-12×.
#[cfg(feature = "rust_perf_bench")]
fn smallvec_stack_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 4, 8],
        harness: r#"
mod buggy {
    pub fn sum_small(n: usize) -> u64 {
        let mut v: Vec<u64> = Vec::new();
        for i in 0..n { v.push(i as u64); }
        v.iter().copied().sum()
    }
}

mod fixed {
    pub fn sum_small(n: usize) -> u64 {
        if n <= 8 {
            let mut arr = [0u64; 8];
            for i in 0..n { arr[i] = i as u64; }
            arr.iter().take(n).copied().sum()
        } else {
            let mut v: Vec<u64> = Vec::new();
            for i in 0..n { v.push(i as u64); }
            v.iter().copied().sum()
        }
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    std::hint::black_box(buggy::sum_small(*input));
}

pub fn run_fixed(input: &usize) {
    std::hint::black_box(fixed::sum_small(*input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `buffered-io`: File::read byte-by-byte vs BufReader.
///
/// Both paths read the same temp file. The buggy path issues a syscall per
/// byte; the fixed path batches reads through BufReader's 8KB buffer.
/// ~97× at 100KB file. The bench writes a temp file per run + cleans up.
#[cfg(feature = "rust_perf_bench")]
fn buffered_io_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000],
        harness: r#"
use std::io::Read;
use std::path::PathBuf;

mod buggy {
    use std::io::Read;
    pub fn sum_bytes(path: &std::path::Path) -> u64 {
        let mut f = std::fs::File::open(path).unwrap();
        let mut total = 0u64;
        let mut byte = [0u8; 1];
        while f.read(&mut byte).unwrap() == 1 {
            total += byte[0] as u64;
        }
        total
    }
}

mod fixed {
    use std::io::Read;
    pub fn sum_bytes(path: &std::path::Path) -> u64 {
        let f = std::fs::File::open(path).unwrap();
        let mut br = std::io::BufReader::new(f);
        let mut total = 0u64;
        let mut byte = [0u8; 1];
        while br.read(&mut byte).unwrap() == 1 {
            total += byte[0] as u64;
        }
        total
    }
}

pub struct Input {
    path: PathBuf,
}

pub fn make_input(n: usize) -> Input {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("perf_bench_buffered_io_{}.dat", n));
    let content: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
    std::fs::write(&path, &content).unwrap();
    Input { path }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::sum_bytes(&input.path));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::sum_bytes(&input.path));
}
"#
        .to_string(),
    }
}

/// Bench spec for `log-write-direct`: push_str(&format!(...)) vs write! direct.
///
/// The buggy path creates a temporary String via format! every iteration,
/// pushes it into the log buffer, then drops the temp. The fixed path uses
/// write! to format directly into the existing buffer — zero temp alloc.
/// ~1.7× at all sizes. Distinct from `format-concat-loop` (quadratic
/// accumulation); this is about avoiding the per-iteration temporary.
#[cfg(feature = "rust_perf_bench")]
fn log_write_direct_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
mod buggy {
    pub fn build_log(items: &[u64]) -> String {
        let mut log = String::new();
        for &i in items {
            log.push_str(&format!("item={i}\n"));
        }
        log
    }
}

mod fixed {
    use std::fmt::Write;
    pub fn build_log(items: &[u64]) -> String {
        let mut log = String::new();
        for &i in items {
            write!(&mut log, "item={i}\n").unwrap();
        }
        log
    }
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n as u64).collect()
}

pub fn run_buggy(input: &[u64]) {
    std::hint::black_box(buggy::build_log(input));
}

pub fn run_fixed(input: &[u64]) {
    std::hint::black_box(fixed::build_log(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `sort-unstable`: stable sort (allocates temp buffer) vs
/// unstable sort (in-place pdqsort).
///
/// Unstable sort avoids the O(n) temporary allocation + uses a faster
/// algorithm (pattern-defeating quicksort vs mergesort). The win is largest
/// on random data where pdqsort's pivot selection is optimal.
#[cfg(feature = "rust_perf_bench")]
fn sort_unstable_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn sort(scores: &mut Vec<u32>) { scores.sort(); }
}

mod fixed {
    pub fn sort(scores: &mut Vec<u32>) { scores.sort_unstable(); }
}

pub struct Input {
    data: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    // Pseudo-random data (deterministic seed for reproducibility).
    let mut data = Vec::with_capacity(n);
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    for i in 0..n {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        data.push((seed.wrapping_add(i as u64)) as u32);
    }
    Input { data }
}

pub fn run_buggy(input: &Input) {
    let mut data = input.data.clone();
    buggy::sort(&mut data);
    std::hint::black_box(&data);
}

pub fn run_fixed(input: &Input) {
    let mut data = input.data.clone();
    fixed::sort(&mut data);
    std::hint::black_box(&data);
}
"#
        .to_string(),
    }
}

/// Bench spec for `sort-by-key`: sort_by with repeated closure calls vs
/// sort_by_key with single key extraction.
///
/// Uses `Vec<(String, u32)>` sorted by the u32 score field. `sort_by` calls
/// the comparator O(N log N) times — each call reads `.1` from two elements.
/// `sort_by_key` extracts the key once per element (N calls), then sorts the
/// cached `u32` keys. The win is cache locality: with large `(String, u32)`
/// tuples spanning multiple cache lines, accessing `.1` fewer times reduces
/// cache pressure.
#[cfg(feature = "rust_perf_bench")]
fn sort_by_key_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 50_000, 100_000],
        harness: r#"
mod buggy {
    pub fn sort_by_score(items: &mut Vec<(String, u32)>) {
        items.sort_by(|a, b| a.1.cmp(&b.1));
    }
}

mod fixed {
    pub fn sort_by_score(items: &mut Vec<(String, u32)>) {
        items.sort_by_key(|item| item.1);
    }
}

pub struct Input {
    items: Vec<(String, u32)>,
}

pub fn make_input(n: usize) -> Input {
    let mut items = Vec::with_capacity(n);
    let mut seed: u64 = 0x12345678ABCDEF00;
    for i in 0..n {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        // Generate strings of varying length (10-30 chars) so each tuple
        // is large enough to span cache lines — makes the key-extraction
        // count difference measurable.
        let len = 10 + (seed % 21) as usize;
        let s: String = (0..len).map(|j| ((seed.wrapping_add(j as u64) % 26) as u8 + b'a') as char).collect();
        items.push((s, (seed % 1_000_000) as u32));
    }
    Input { items }
}

pub fn run_buggy(input: &Input) {
    let mut items = input.items.clone();
    buggy::sort_by_score(&mut items);
    std::hint::black_box(&items);
}

pub fn run_fixed(input: &Input) {
    let mut items = input.items.clone();
    fixed::sort_by_score(&mut items);
    std::hint::black_box(&items);
}
"#
        .to_string(),
    }
}

/// Bench spec for `bufwriter-batched`: per-record File::write (syscall each)
/// vs BufWriter batched.
///
/// Uses `tempfile` to get a real filesystem path. Each record is 64 bytes;
/// with 8KB BufWriter buffering, ~128 records per flush. The buggy path does
/// one syscall per record; the fixed path does one syscall per ~128 records.
#[cfg(feature = "rust_perf_bench")]
fn bufwriter_batched_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
// [dep: tempfile = "3"]
use std::io::{BufWriter, Write};

mod buggy {
    use std::io::Write;
    pub fn write_records(file: &mut std::fs::File, records: &[Vec<u8>]) {
        for rec in records {
            let _ = file.write_all(rec);
        }
    }
}

mod fixed {
    use std::io::{BufWriter, Write};
    pub fn write_records(file: std::fs::File, records: &[Vec<u8>]) {
        let mut buf = BufWriter::new(file);
        for rec in records {
            let _ = buf.write_all(rec);
        }
        let _ = buf.flush();
    }
}

pub struct Input {
    records: Vec<Vec<u8>>,
    dir: tempfile::TempDir,
}

pub fn make_input(n: usize) -> Input {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut records = Vec::with_capacity(n);
    for i in 0..n {
        // 64-byte records (realistic line length)
        let rec = format!("{i:060}\n").into_bytes();
        records.push(rec);
    }
    Input { records, dir }
}

pub fn run_buggy(input: &Input) {
    let path = input.dir.path().join("buggy.txt");
    let mut file = std::fs::File::create(&path).expect("create");
    buggy::write_records(&mut file, &input.records);
    std::hint::black_box(&file);
    drop(file);
    let _ = std::fs::remove_file(&path);
}

pub fn run_fixed(input: &Input) {
    let path = input.dir.path().join("fixed.txt");
    let file = std::fs::File::create(&path).expect("create");
    fixed::write_records(file, &input.records);
    let _ = std::fs::remove_file(&path);
}
"#
        .to_string(),
    }
}

/// Bench spec for `copy-from-slice`: manual index-loop copy vs `copy_from_slice`.
///
/// The manual loop does per-element bounds checking + scalar loads/stores.
/// `copy_from_slice` is a single intrinsic that LLVM lowers to `memcpy` or
/// SIMD-wide operations. The win is largest on medium-sized slices where
/// the manual loop's overhead (bounds check + scalar ops) is measurable but
/// the data fits in cache.
#[cfg(feature = "rust_perf_bench")]
fn copy_from_slice_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![64, 256, 1_024, 4_096],
        harness: r#"
mod buggy {
    pub fn copy_data(dst: &mut [u8], src: &[u8]) {
        for i in 0..src.len() {
            dst[i] = src[i];
        }
    }
}

mod fixed {
    pub fn copy_data(dst: &mut [u8], src: &[u8]) {
        dst.copy_from_slice(src);
    }
}

pub struct Input {
    src: Vec<u8>,
    dst: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed: u64 = 0xDEADBEEFCAFEBABE;
    let src: Vec<u8> = (0..n).map(|i| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as u8
    }).collect();
    let dst = vec![0u8; n];
    Input { src, dst }
}

pub fn run_buggy(input: &Input) {
    let mut dst = input.dst.clone();
    buggy::copy_data(&mut dst, &input.src);
    std::hint::black_box(&dst);
}

pub fn run_fixed(input: &Input) {
    let mut dst = input.dst.clone();
    fixed::copy_data(&mut dst, &input.src);
    std::hint::black_box(&dst);
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-remove-front-to-vecdeque`: drain N elements from front.
///
/// Vec::remove(0) does a memmove of all remaining elements (O(n) per call,
/// O(n²) total). VecDeque::pop_front advances the head index (O(1) per call,
/// O(n) total). Grows quadratically with N.
#[cfg(feature = "rust_perf_bench")]
fn vec_remove_front_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 50_000],
        harness: r#"
mod buggy {
    pub fn drain_queue(queue: &mut Vec<u32>) -> u64 {
        let mut sum = 0u64;
        while !queue.is_empty() {
            sum += queue.remove(0) as u64;
        }
        sum
    }
}

mod fixed {
    use std::collections::VecDeque;
    pub fn drain_queue(queue: &mut VecDeque<u32>) -> u64 {
        let mut sum = 0u64;
        while let Some(val) = queue.pop_front() {
            sum += val as u64;
        }
        sum
    }
}

pub struct Input {
    vec: Vec<u32>,
    deque: std::collections::VecDeque<u32>,
}

pub fn make_input(n: usize) -> Input {
    let vec: Vec<u32> = (0..n).map(|i| i as u32).collect();
    let deque: std::collections::VecDeque<u32> = vec.iter().copied().collect();
    Input { vec, deque }
}

pub fn run_buggy(input: &Input) {
    let mut queue = input.vec.clone();
    std::hint::black_box(buggy::drain_queue(&mut queue));
}

pub fn run_fixed(input: &Input) {
    let mut queue = input.deque.clone();
    std::hint::black_box(fixed::drain_queue(&mut queue));
}
"#
        .to_string(),
    }
}

/// Bench spec for `btreemap-sorted-iteration`: sorted iteration × K queries.
///
/// HashMap + sort_unstable per query is O(n log n) each. BTreeMap's keys()
/// iterator is already sorted — O(n) per query. Insert in pseudo-random order
/// so HashMap's iteration order is NOT accidentally sorted.
#[cfg(feature = "rust_perf_bench")]
fn btreemap_sorted_iteration_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 50_000],
        harness: r#"
use std::collections::{BTreeMap, HashMap};

mod buggy {
    use std::collections::HashMap;
    pub fn sorted_keys(map: &HashMap<u32, u32>) -> Vec<u32> {
        let mut keys: Vec<u32> = map.keys().copied().collect();
        keys.sort_unstable();
        keys
    }
}

mod fixed {
    use std::collections::BTreeMap;
    pub fn sorted_keys(map: &BTreeMap<u32, u32>) -> Vec<u32> {
        map.keys().copied().collect()
    }
}

pub struct Input {
    hashmap: HashMap<u32, u32>,
    btreemap: BTreeMap<u32, u32>,
    queries: usize,
}

pub fn make_input(n: usize) -> Input {
    let mut hashmap = HashMap::with_capacity(n);
    let mut btreemap = BTreeMap::new();
    // Insert with pseudo-random keys so HashMap iteration order is NOT sorted.
    for i in 0..n {
        let key = (i.wrapping_mul(7919).wrapping_add(31)) as u32;
        hashmap.insert(key, i as u32);
        btreemap.insert(key, i as u32);
    }
    // Sorted iteration needed repeatedly (N/10 queries) — the common case
    // where a BTreeMap is the right choice over a HashMap.
    let queries = (n / 10).max(1);
    Input { hashmap, btreemap, queries }
}

pub fn run_buggy(input: &Input) {
    let queries = std::hint::black_box(input.queries);
    for _ in 0..queries {
        std::hint::black_box(buggy::sorted_keys(&input.hashmap));
    }
}

pub fn run_fixed(input: &Input) {
    let queries = std::hint::black_box(input.queries);
    for _ in 0..queries {
        std::hint::black_box(fixed::sorted_keys(&input.btreemap));
    }
}
"#
        .to_string(),
    }
}

/// Bench spec for `cow-conditional-alloc`: HTML-escape N strings (90% clean).
///
/// The buggy path calls `s.to_string()` for every string (N allocations).
/// The fixed path returns `Cow::Borrowed(s)` for the 90% that need no escaping
/// (0 allocations) + `Cow::Owned(...)` for the 10% that do. Net: 0.1N vs N
/// allocations.
#[cfg(feature = "rust_perf_bench")]
fn cow_conditional_alloc_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 50_000],
        harness: r#"
use std::borrow::Cow;

mod buggy {
    pub fn escape_html(s: &str) -> String {
        if !s.contains(['<', '>', '&']) {
            return s.to_string();
        }
        s.replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('&', "&amp;")
    }
}

mod fixed {
    use std::borrow::Cow;
    pub fn escape_html(s: &str) -> Cow<str> {
        if !s.contains(['<', '>', '&']) {
            return Cow::Borrowed(s);
        }
        Cow::Owned(s.replace('<', "&lt;").replace('>', "&gt;").replace('&', "&amp;"))
    }
}

pub struct Input {
    strings: Vec<String>,
}

pub fn make_input(n: usize) -> Input {
    // 90% of strings have no special chars (common case); 10% do.
    let strings: Vec<String> = (0..n).map(|i| {
        if i % 10 == 0 {
            format!("hello <b>world</b> #{} & friends", i)
        } else {
            format!("plain text line number {}", i)
        }
    }).collect();
    Input { strings }
}

pub fn run_buggy(input: &Input) {
    let mut total_len = 0usize;
    for s in &input.strings {
        let escaped = buggy::escape_html(s);
        total_len += escaped.len();
    }
    std::hint::black_box(total_len);
}

pub fn run_fixed(input: &Input) {
    let mut total_len = 0usize;
    for s in &input.strings {
        let escaped = fixed::escape_html(s);
        total_len += escaped.len();
    }
    std::hint::black_box(total_len);
}
"#
        .to_string(),
    }
}

/// Bench spec for `regex-hoist-lazylock`: Regex::new per call vs static LazyLock.
///
/// Regex compilation is runtime NFA construction — real work (μs-scale, plus
/// KB of allocation) the compiler cannot elide or cache. The buggy path
/// recompiles the SAME pattern per call; the fixed path compiles once into a
/// `static LazyLock` (initialized during warmup, so the steady-state alloc
/// axis is 0 for the fixed side) and pays only the match. Guide prior ~555×
/// per call (Research 032); verified on this harness in Bench 020.
#[cfg(feature = "rust_perf_bench")]
fn regex_hoist_lazylock_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10, 100, 1_000],
        harness: r#"
// [dep: regex = "1"]
mod buggy {
    use regex::Regex;
    pub fn is_valid(code: &str) -> bool {
        let re = Regex::new(r"^[A-Z]{2}-\d{4}$").unwrap();
        re.is_match(code)
    }
}

mod fixed {
    use regex::Regex;
    use std::sync::LazyLock;
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Z]{2}-\d{4}$").unwrap());
    pub fn is_valid(code: &str) -> bool {
        RE.is_match(code)
    }
}

pub struct Input {
    codes: Vec<String>,
}

pub fn make_input(n: usize) -> Input {
    // Half valid (match path), half invalid (scan-to-end path) — a realistic
    // mix rather than a best-case all-match workload.
    let codes: Vec<String> = (0..n)
        .map(|i| {
            if i % 2 == 0 {
                format!("AB-{:04}", i % 10_000)
            } else {
                format!("invalid code {}!", i)
            }
        })
        .collect();
    Input { codes }
}

pub fn run_buggy(input: &Input) {
    let mut hits = 0usize;
    for code in &input.codes {
        if buggy::is_valid(code) {
            hits += 1;
        }
    }
    std::hint::black_box(hits);
}

pub fn run_fixed(input: &Input) {
    let mut hits = 0usize;
    for code in &input.codes {
        if fixed::is_valid(code) {
            hits += 1;
        }
    }
    std::hint::black_box(hits);
}
"#
        .to_string(),
    }
}

/// Bench spec for `arc-str-payload`: per-recipient payload clone vs Arc<str>.
///
/// A 1 KiB immutable payload fanned out to N recipients. Buggy: `to_string()`
/// per recipient — N allocations + N × 1024 bytes of copying. Fixed: ONE
/// `Arc::from` allocation + refcount-bump clones. Timing prior ~1.3× (the
/// buggy side is memcpy-bound, ~GB/s) may land in the Tie band; the
/// deterministic alloc axis carries the gate (N×1024 B → ~1 alloc + the Vec).
#[cfg(feature = "rust_perf_bench")]
fn arc_str_payload_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10, 100, 1_000],
        harness: r#"
use std::sync::Arc;

mod buggy {
    pub fn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, String)> {
        ids.iter().map(|&id| (id, payload.to_string())).collect()
    }
}

mod fixed {
    use std::sync::Arc;
    pub fn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, Arc<str>)> {
        let shared: Arc<str> = Arc::from(payload);
        ids.iter().map(|&id| (id, Arc::clone(&shared))).collect()
    }
}

pub struct Input {
    payload: String,
    ids: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    // 1 KiB immutable payload (config snapshot, telemetry batch, ...).
    let payload = "0123456789abcdef".repeat(64);
    let ids: Vec<u32> = (0..n as u32).collect();
    Input { payload, ids }
}

pub fn run_buggy(input: &Input) {
    let out = buggy::fan_out(&input.payload, &input.ids);
    // Touch every message's payload length so the strings are materialized,
    // not just the Vec scaffold.
    let total: usize = out.iter().map(|(id, s)| *id as usize + s.len()).sum();
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let out = fixed::fan_out(&input.payload, &input.ids);
    let total: usize = out.iter().map(|(id, s)| *id as usize + s.len()).sum();
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `collect-append-to-extend`: collect+append staging vs extend.
#[cfg(feature = "rust_perf_bench")]
fn collect_append_to_extend_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    // One intermediate Vec alloc + memcpy per chunk.
    pub fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {
        for c in chunks {
            let mut staged: Vec<u32> = c.iter().copied().collect();
            target.append(&mut staged);
        }
    }
}

mod fixed {
    // Straight into the target's growth path — zero intermediates.
    pub fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {
        for c in chunks {
            target.extend(c.iter().copied());
        }
    }
}

pub fn make_input(n: usize) -> Vec<Vec<u32>> {
    // Chunks of 16 — a realistic batch shape (both sides do the same work,
    // the difference is purely the intermediate staging).
    (0..n / 16).map(|b| (0..16).map(|i| (b * 16 + i) as u32).collect()).collect()
}

pub fn run_buggy(input: &[Vec<u32>]) {
    let mut target: Vec<u32> = Vec::with_capacity(input.len() * 16);
    buggy::merge_chunks(&mut target, input);
    std::hint::black_box(target.len());
}

pub fn run_fixed(input: &[Vec<u32>]) {
    let mut target: Vec<u32> = Vec::with_capacity(input.len() * 16);
    fixed::merge_chunks(&mut target, input);
    std::hint::black_box(target.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `hashset-insert-bool-return`: contains+insert double lookup
/// vs insert's bool return. Half-duplicate input so both bool arms fire.
#[cfg(feature = "rust_perf_bench")]
fn hashset_insert_bool_return_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
use std::collections::HashSet;

mod buggy {
    use std::collections::HashSet;
    // Two hash-table walks per element.
    pub fn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {
        let mut inserted = 0;
        for &x in items {
            if !seen.contains(&x) {
                seen.insert(x);
                inserted += 1;
            }
        }
        inserted
    }
}

mod fixed {
    use std::collections::HashSet;
    // ONE hash-table walk per element.
    pub fn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {
        let mut inserted = 0;
        for &x in items {
            if seen.insert(x) {
                inserted += 1;
            }
        }
        inserted
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // 50% duplicates: every element's membership arm fires both ways.
    (0..n).map(|i| (i % (n / 2).max(1)) as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut seen = HashSet::with_capacity(input.len());
    std::hint::black_box(buggy::register_seen(&mut seen, input));
}

pub fn run_fixed(input: &[u32]) {
    let mut seen = HashSet::with_capacity(input.len());
    std::hint::black_box(fixed::register_seen(&mut seen, input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `unwrap-or-else-lazy`: eager default construction vs lazy
/// closure, 99% Some (the regime where the eager default is pure waste).
/// Mirrors the `ok-or-else` harness shape.
#[cfg(feature = "rust_perf_bench")]
fn unwrap_or_else_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
mod buggy {
    fn expensive_default(n: u32) -> u32 {
        (0..n).fold(0u32, |acc, i| acc.wrapping_add(i.wrapping_mul(31)))
    }
    pub fn resolve(opt: Option<u32>) -> u32 {
        opt.unwrap_or(expensive_default(std::hint::black_box(64))) // eagerly evaluated even when Some
    }
}

mod fixed {
    fn expensive_default(n: u32) -> u32 {
        (0..n).fold(0u32, |acc, i| acc.wrapping_add(i.wrapping_mul(31)))
    }
    pub fn resolve(opt: Option<u32>) -> u32 {
        opt.unwrap_or_else(|| expensive_default(std::hint::black_box(64))) // built only on None
    }
}

pub fn make_input(n: usize) -> Vec<Option<u32>> {
    // 99% Some, 1% None — realistic distribution (mirrors ok-or-else).
    (0..n).map(|i| if i % 100 == 0 { None } else { Some(i as u32) }).collect()
}

pub fn run_buggy(input: &[Option<u32>]) {
    // black_box(input) defeats loop-invariant hoisting of the pure loop body
    // out of the caller's timing loop (Batch 48 lesson).
    let input = std::hint::black_box(input);
    let sum: u32 = input.iter().map(|o| buggy::resolve(*o)).sum();
    std::hint::black_box(sum);
}

pub fn run_fixed(input: &[Option<u32>]) {
    let input = std::hint::black_box(input);
    let sum: u32 = input.iter().map(|o| fixed::resolve(*o)).sum();
    std::hint::black_box(sum);
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-count-to-any`: full-scan filter+count vs any
/// short-circuit. First element matches, rest don't — the maximal short-circuit
/// regime (the rule's selling point).
#[cfg(feature = "rust_perf_bench")]
fn filter_count_to_any_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn has_hit(vals: &[u32]) -> bool {
        vals.iter().filter(|&&v| v == 42).count() > 0 // full scan, always
    }
}

mod fixed {
    pub fn has_hit(vals: &[u32]) -> bool {
        vals.iter().any(|&v| v == 42) // stops at the first hit
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Hit lands at n/2 — an early-but-not-zero short-circuit regime. (A
    // position-0 hit drives `any` to a literal sub-ns call, which the
    // validator's div-by-zero guard classifies as a Tie — the guard exists
    // for malformed 0ns measurements, not for genuinely free calls.)
    let mut v = vec![7u32; n];
    v[n / 2] = 42;
    v
}

pub fn run_buggy(input: &[u32]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::has_hit(input));
}

pub fn run_fixed(input: &[u32]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::has_hit(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-macro-zero-fill`: uninit alloc + memset vs calloc.
/// Both sides black_box-read a byte near the end so the pages are touched
/// identically for the read; the difference is the WRITE path (memset vs
/// lazy zero pages).
#[cfg(feature = "rust_perf_bench")]
fn vec_macro_zero_fill_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn zero_buf(n: usize) -> Vec<u8> {
        let mut v = Vec::new();
        v.resize(n, 0); // uninit alloc + explicit memset
        v
    }
}

mod fixed {
    pub fn zero_buf(n: usize) -> Vec<u8> {
        vec![0u8; n] // alloc_zeroed — calloc zero pages
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    // black_box(n) defeats loop-invariant hoisting; spread reads (first /
    // middle / last) make partial memset elision observable.
    let n = std::hint::black_box(*input);
    let v = buggy::zero_buf(n);
    let mid = v.len() / 2;
    std::hint::black_box((v[0], v[mid], v[v.len() - 1]));
}

pub fn run_fixed(input: &usize) {
    let n = std::hint::black_box(*input);
    let v = fixed::zero_buf(n);
    let mid = v.len() / 2;
    std::hint::black_box((v[0], v[mid], v[v.len() - 1]));
}
"#
        .to_string(),
    }
}

/// Bench spec for `iter-copied-deref-map`: generic Map adapter vs specialized
/// copied(). Sum consumer at plain sizes — honest Tie expected in tight
/// release loops; the alloc axis + adapter markers carry the distinction.
#[cfg(feature = "rust_perf_bench")]
fn iter_copied_deref_map_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_kills(kills: &[u64]) -> u64 {
        kills.iter().map(|k| *k).sum()
    }
}

mod fixed {
    pub fn sum_kills(kills: &[u64]) -> u64 {
        kills.iter().copied().sum()
    }
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n).map(|i| i as u64).collect()
}

pub fn run_buggy(input: &[u64]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::sum_kills(input));
}

pub fn run_fixed(input: &[u64]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::sum_kills(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-map-fuse`: two adapters vs filter_map. The
/// perf-book hedges — the Pareto verdict decides the annotation.
#[cfg(feature = "rust_perf_bench")]
fn filter_map_fuse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn even_squares(vals: &[u32]) -> Vec<u32> {
        vals.iter().filter(|&&v| v % 2 == 0).map(|&v| v * v).collect()
    }
}

mod fixed {
    pub fn even_squares(vals: &[u32]) -> Vec<u32> {
        vals.iter().filter_map(|&v| (v % 2 == 0).then_some(v * v)).collect()
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    std::hint::black_box(buggy::even_squares(input));
}

pub fn run_fixed(input: &[u32]) {
    std::hint::black_box(fixed::even_squares(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-insert-front-deque`: Vec::insert(0, x) O(n²) prepend
/// loop vs VecDeque::push_front O(1). Mirrors the vec-remove-front harness.
#[cfg(feature = "rust_perf_bench")]
fn vec_insert_front_deque_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![250, 1_000, 4_000],
        harness: r#"
use std::collections::VecDeque;

mod buggy {
    pub fn prepend_all(out: &mut Vec<u32>, items: &[u32]) {
        for &x in items {
            out.insert(0, x); // O(n) memmove each → O(n²) total
        }
    }
}

mod fixed {
    use std::collections::VecDeque;
    pub fn prepend_all(out: &mut VecDeque<u32>, items: &[u32]) {
        for &x in items {
            out.push_front(x); // O(1) ring head advance
        }
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n).map(|i| i as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut out = Vec::with_capacity(input.len());
    buggy::prepend_all(&mut out, input);
    std::hint::black_box(out.len());
}

pub fn run_fixed(input: &[u32]) {
    let mut out = VecDeque::with_capacity(input.len());
    fixed::prepend_all(&mut out, input);
    std::hint::black_box(out.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `powi-vs-powf`: libm pow call vs single multiply. The
/// honest risk the bench adjudicates: LLVM instcombine may rewrite
/// `pow(x, 2.0)` to `x * x` in release, collapsing the win (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn powi_vs_powf_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_squares(vals: &[f32]) -> f32 {
        vals.iter().map(|v| v.powf(2.0)).sum()
    }
}

mod fixed {
    pub fn sum_squares(vals: &[f32]) -> f32 {
        vals.iter().map(|v| v.powi(2)).sum()
    }
}

pub fn make_input(n: usize) -> Vec<f32> {
    (0..n).map(|i| i as f32 * 0.5 - 0.25).collect()
}

pub fn run_buggy(input: &[f32]) {
    std::hint::black_box(buggy::sum_squares(input));
}

pub fn run_fixed(input: &[f32]) {
    std::hint::black_box(fixed::sum_squares(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `distance-sq-no-sqrt`: sqrt-then-compare vs squared
/// compare. On aarch64 f32 sqrt is a single NEON instruction — the bench
/// decides whether the scalar-ARM reality matches the Box2D x86-SIMD
/// rationale (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn distance_sq_no_sqrt_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
pub struct Query {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
}

mod buggy {
    pub fn count_in_range(pts: &[(f32, f32)], cx: f32, cy: f32, r: f32) -> usize {
        pts.iter()
            .filter(|&&(x, y)| {
                let dx = x - cx;
                let dy = y - cy;
                (dx * dx + dy * dy).sqrt() < r
            })
            .count()
    }
}

mod fixed {
    pub fn count_in_range(pts: &[(f32, f32)], cx: f32, cy: f32, r: f32) -> usize {
        let r2 = r * r;
        pts.iter()
            .filter(|&&(x, y)| {
                let dx = x - cx;
                let dy = y - cy;
                dx * dx + dy * dy < r2
            })
            .count()
    }
}

pub struct Input {
    pts: Vec<(f32, f32)>,
    q: Query,
}

pub fn make_input(n: usize) -> Input {
    let pts = (0..n).map(|i| (i as f32 * 0.013, i as f32 * 0.029)).collect();
    Input {
        pts,
        q: Query { cx: 0.0, cy: 0.0, r: 5000.0 },
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_in_range(&input.pts, input.q.cx, input.q.cy, input.q.r));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_in_range(&input.pts, input.q.cx, input.q.cy, input.q.r));
}
"#
        .to_string(),
    }
}

/// Bench spec for `hashmap-with-capacity`: rehash ladder vs single exact
/// reservation. The alloc axis gates this even where SipHash timing
/// dominates (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn hashmap_with_capacity_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![8, 64, 512, 4096],
        harness: r#"
mod buggy {
    use std::collections::HashMap;
    pub fn build(n: usize) -> usize {
        let mut m: HashMap<u64, u64> = HashMap::new();
        for i in 0..n as u64 {
            m.insert(i, i.wrapping_mul(2_654_435_761));
        }
        m.len()
    }
}

mod fixed {
    use std::collections::HashMap;
    pub fn build(n: usize) -> usize {
        let mut m: HashMap<u64, u64> = HashMap::with_capacity(n);
        for i in 0..n as u64 {
            m.insert(i, i.wrapping_mul(2_654_435_761));
        }
        m.len()
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    std::hint::black_box(buggy::build(*input));
}

pub fn run_fixed(input: &usize) {
    std::hint::black_box(fixed::build(*input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `soa-hot-field`: 40-byte AoS stride scan vs contiguous
/// SoA column sum (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn soa_hot_field_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub struct Agent {
        pub x: f32,
        pub y: f32,
        pub vx: f32,
        pub vy: f32,
        pub hp: u32,
        pub level: u32,
        pub flags: u64,
        pub id: u64,
    }
    pub fn total_x(agents: &[Agent]) -> f32 {
        agents.iter().map(|a| a.x).sum()
    }
}

mod fixed {
    pub struct Agents {
        pub xs: Vec<f32>,
        pub ys: Vec<f32>,
        pub vxs: Vec<f32>,
        pub vys: Vec<f32>,
        pub hps: Vec<u32>,
        pub levels: Vec<u32>,
        pub flags: Vec<u64>,
        pub ids: Vec<u64>,
    }
    pub fn total_x(agents: &Agents) -> f32 {
        agents.xs.iter().sum()
    }
}

pub struct Input {
    aos: Vec<buggy::Agent>,
    soa: fixed::Agents,
}

pub fn make_input(n: usize) -> Input {
    let mut aos = Vec::with_capacity(n);
    let mut soa = fixed::Agents {
        xs: Vec::with_capacity(n),
        ys: Vec::with_capacity(n),
        vxs: Vec::with_capacity(n),
        vys: Vec::with_capacity(n),
        hps: Vec::with_capacity(n),
        levels: Vec::with_capacity(n),
        flags: Vec::with_capacity(n),
        ids: Vec::with_capacity(n),
    };
    for i in 0..n as u64 {
        let x = i as f32 * 0.5;
        let y = i as f32 * 0.25;
        let vx = i as f32 * 0.125;
        let vy = i as f32 * 0.0625;
        aos.push(buggy::Agent {
            x,
            y,
            vx,
            vy,
            hp: (i % 1000) as u32,
            level: (i % 50) as u32,
            flags: i,
            id: i,
        });
        soa.xs.push(x);
        soa.ys.push(y);
        soa.vxs.push(vx);
        soa.vys.push(vy);
        soa.hps.push((i % 1000) as u32);
        soa.levels.push((i % 50) as u32);
        soa.flags.push(i);
        soa.ids.push(i);
    }
    Input { aos, soa }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::total_x(&input.aos));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::total_x(&input.soa));
}
"#
        .to_string(),
    }
}

/// Bench spec for `division-by-invariant-multiplication`: hardware integer
/// division by a loop-invariant RUNTIME divisor vs libdivide-style magic
/// multiplication (Batch 59, thermite distill). The divisor is a runtime
/// field passed through `black_box` at the call boundary — LLVM already
/// strength-reduces compile-time-constant divisors, so the baseline must
/// stay opaque for the bench to measure the rule's actual regime. Setup
/// (one hardware div inside `FastDiv::new`) is INSIDE the timed call — the
/// honest per-batch shape; it amortizes to noise at n >= 1000.
#[cfg(feature = "rust_perf_bench")]
const DIVISION_INVARIANT_FIXED_SRC: &str = r#"    pub struct FastDivU32 { magic: u32, shift: u32, add: bool }
    impl FastDivU32 {
        pub fn new(d: u32) -> Self {
            let l = 31 - d.leading_zeros();
            if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
            let k = 1u64 << l;
            let base = (k << 32) / u64::from(d);
            let rem = (k << 32) % u64::from(d);
            if u64::from(d) - rem < k {
                Self { magic: base as u32 + 1, shift: l, add: false } // ceil magic — the +1 is load-bearing
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u64::from(d) { m += 1; }
                Self { magic: (m + 1) as u32, shift: l, add: true } // (m+1) wraps mod 2^32 — intended
            }
        }
        #[inline(always)]
        pub fn div(&self, x: u32) -> u32 {
            if self.magic == 0 { return x >> self.shift; } // power-of-two divisor
            let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;
            // widening add: hi + x reaches ~2^33 — a u32 wrap drops the carry bit
            // (d=7 x=u32::MAX fails by 2^29). The theorem holds in wide math.
            if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift }
        }
    }
    pub struct FastDivU64 { magic: u64, shift: u32, add: bool }
    impl FastDivU64 {
        pub fn new(d: u64) -> Self {
            let l = 63 - d.leading_zeros();
            if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
            let k = 1u128 << l;
            let base = (k << 64) / u128::from(d);
            let rem = (k << 64) % u128::from(d);
            if u128::from(d) - rem < k {
                Self { magic: base as u64 + 1, shift: l, add: false } // ceil magic — the +1 is load-bearing
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u128::from(d) { m += 1; }
                Self { magic: (m + 1) as u64, shift: l, add: true } // (m+1) wraps mod 2^64 — intended
            }
        }
        #[inline(always)]
        pub fn div(&self, x: u64) -> u64 {
            if self.magic == 0 { return x >> self.shift; } // power-of-two divisor
            let hi = ((u128::from(self.magic) * u128::from(x)) >> 64) as u64;
            // widening add (see u32 twin): u64 wrap would drop the carry bit
            if self.add { ((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64 } else { hi >> self.shift }
        }
    }
    pub fn sum_quot_u32(data: &[u32], d: u32) -> u32 {
        let fd = FastDivU32::new(d); // one hardware div, hoisted
        let mut acc = 0u32;
        for &x in data {
            acc = acc.wrapping_add(fd.div(x)); // mulhi + shifts per element
        }
        acc
    }
    pub fn sum_quot_u64(data: &[u64], d: u64) -> u64 {
        let fd = FastDivU64::new(d);
        let mut acc = 0u64;
        for &x in data {
            acc = acc.wrapping_add(fd.div(x));
        }
        acc
    }"#;

#[cfg(feature = "rust_perf_bench")]
fn division_invariant_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_quot_u32(data: &[u32], d: u32) -> u32 {
        let mut acc = 0u32;
        for &x in data {
            acc = acc.wrapping_add(x / d); // hardware udiv per element
        }
        acc
    }
    pub fn sum_quot_u64(data: &[u64], d: u64) -> u64 {
        let mut acc = 0u64;
        for &x in data {
            acc = acc.wrapping_add(x / d);
        }
        acc
    }
}

mod fixed {
__FASTDIV_IMPLS__
}

pub struct Input {
    data32: Vec<u32>,
    data64: Vec<u64>,
    d32: u32,
    d64: u64,
}

pub fn make_input(n: usize) -> Input {
    // diverse residues; the divisors are runtime fields (NOT constants —
    // see the spec doc comment on strength reduction)
    let data32 = (0..n).map(|i| (i as u32).wrapping_mul(2654435761)).collect();
    let data64 = (0..n).map(|i| (i as u64).wrapping_mul(6364136223846793005)).collect();
    Input { data32, data64, d32: 97, d64: 1000003 }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::sum_quot_u32(&input.data32, std::hint::black_box(input.d32)));
    std::hint::black_box(buggy::sum_quot_u64(&input.data64, std::hint::black_box(input.d64)));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::sum_quot_u32(&input.data32, std::hint::black_box(input.d32)));
    std::hint::black_box(fixed::sum_quot_u64(&input.data64, std::hint::black_box(input.d64)));
}"#
            .replace("__FASTDIV_IMPLS__", DIVISION_INVARIANT_FIXED_SRC),
    }
}

/// Bench spec for `keyword-map-to-match`: static-keyword HashMap dispatch vs
/// plain match on string literals. The map pays SipHash per lookup; the match
/// compiles to a switch on length + 1-2 inline compares (LLVM; oxc verified
/// via --emit=llvm-ir after failing to beat it with a gperf-style perfect
/// hash). Mixed identifier workload (7 keywords + 3 non-keywords cycling);
/// the LazyLock init amortizes across the timing loop — the measured cost is
/// per-lookup dispatch, not construction.
#[cfg(feature = "rust_perf_bench")]
fn keyword_map_to_match_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: r#"
mod buggy {
    use std::collections::HashMap;
    use std::sync::LazyLock;
    #[derive(Clone, Copy)]
    pub enum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }
    static KEYWORDS: LazyLock<HashMap<&'static str, Kind>> = LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("as", Kind::As);
        m.insert("do", Kind::Do);
        m.insert("if", Kind::If);
        m.insert("let", Kind::Let);
        m.insert("fn", Kind::Fn);
        m.insert("while", Kind::While);
        m.insert("constructor", Kind::Constructor);
        m
    });
    pub fn match_keyword(s: &str) -> u64 {
        *KEYWORDS.get(s).unwrap_or(&Kind::Ident) as u64
    }
}

mod fixed {
    #[derive(Clone, Copy)]
    pub enum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }
    pub fn match_keyword(s: &str) -> u64 {
        let k = match s {
            "as" => Kind::As,
            "do" => Kind::Do,
            "if" => Kind::If,
            "let" => Kind::Let,
            "fn" => Kind::Fn,
            "while" => Kind::While,
            "constructor" => Kind::Constructor,
            _ => Kind::Ident,
        };
        k as u64
    }
}

pub fn make_input(n: usize) -> Vec<&'static str> {
    let words = ["as", "identifier", "do", "if", "some_long_name", "let", "fn", "while", "constructor", "another_name"];
    (0..n).map(|i| words[i % words.len()]).collect()
}

pub fn run_buggy(input: &[&'static str]) {
    let input = std::hint::black_box(input);
    let acc: u64 = input.iter().map(|s| buggy::match_keyword(s)).sum();
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &[&'static str]) {
    let input = std::hint::black_box(input);
    let acc: u64 = input.iter().map(|s| fixed::match_keyword(s)).sum();
    std::hint::black_box(acc);
}"#
        .to_string(),
    }
}

/// Bench spec for `usize-index-to-u32`: 16-byte spans vs 8-byte spans under a
/// bandwidth-bound sum pass. At 1K elements both arrays are L1-resident (16KB
/// vs 8KB) — expected Tie. At 1M the wide array is 16MB (exceeds L2 on most
/// cores) while the narrow is 8MB — the bandwidth win should materialize.
/// Same access pattern both sides; only the field width differs (the oxc Span
/// change, isolated).
#[cfg(feature = "rust_perf_bench")]
fn usize_index_to_u32_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 1_000_000],
        harness: r#"
mod buggy {
    pub struct Span { pub start: usize, pub end: usize }
    pub fn total_len(spans: &[Span]) -> u64 {
        let mut acc = 0u64;
        for s in spans { acc += (s.end - s.start) as u64; }
        acc
    }
}

mod fixed {
    pub struct Span { pub start: u32, pub end: u32 }
    pub fn total_len(spans: &[Span]) -> u64 {
        let mut acc = 0u64;
        for s in spans { acc += u64::from(s.end - s.start); }
        acc
    }
}

pub struct Input { wide: Vec<buggy::Span>, narrow: Vec<fixed::Span> }

pub fn make_input(n: usize) -> Input {
    let wide: Vec<buggy::Span> = (0..n)
        .map(|i| buggy::Span { start: i * 3, end: i * 3 + (i % 7) + 1 })
        .collect();
    let narrow: Vec<fixed::Span> = wide
        .iter()
        .map(|s| fixed::Span { start: s.start as u32, end: s.end as u32 })
        .collect();
    Input { wide, narrow }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::total_len(&input.wide));
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::total_len(&input.narrow));
}"#
        .to_string(),
    }
}

/// Bench spec for `transposed-weight-register-gemv`: row-major per-row dot
/// with its horizontal reduce, vs transposed weights with per-input
/// broadcast FMA
/// into output-vector accumulators (the ryg 2015 transposed-lane Mv
/// analysis; microGPT-C `mv16_blk16v(4)` shape, portable-Rust form).
/// NOUT fixed at 64 (4 NEON / 2 AVX2 full lanes); `n` sweeps NIN
/// 16 / 64 / 4096. **Measured (Batch 62, M3 Max): ParetoDominant 1.18x /
/// 2.09x / 6.08x — the research's "wash at large nin" expectation was
/// WRONG-DIRECTION**: in strict-FP Rust the row-dot's scalar accumulator is
/// a serial dependency chain LLVM cannot vectorize without reassociation,
/// so it degrades FURTHER at large nin (llama.cpp's fine row-major dots use
/// explicit multi-accumulator SIMD — not what portable scalar code gets).
/// NOT bit-identical (float sum order changes under transposition) — the
/// validator's rel-error tolerance covers it (the `distance-sq-no-sqrt`
/// 1-ulp caveat precedent). `y` is cloned per run (identical alloc count
/// both sides — the alloc axis compares shapes, not the steady-state
/// discipline).
#[cfg(feature = "rust_perf_bench")]
fn transposed_weight_register_gemv_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![16, 64, 4096],
        harness: "\nconst NOUT: usize = 64; // 4 NEON vectors / 2 AVX2 vectors of lanes — tile stays full\n\nmod buggy {\n    pub fn matvec(w: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n        for r in 0..nout {\n            let row = &w[r * nin..(r + 1) * nin];\n            let mut acc = 0.0f32;\n            for c in 0..nin {\n                acc += row[c] * x[c];\n            }\n            y[r] = acc;\n        }\n    }\n}\n\nmod fixed {\n    pub fn matvec(w_t: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n        for r in 0..nout { y[r] = 0.0; }\n        for c in 0..nin {\n            let xc = x[c];\n            let col = &w_t[c * nout..(c + 1) * nout];\n            for r in 0..nout {\n                y[r] += xc * col[r];\n            }\n        }\n    }\n}\n\npub struct Input { w_row: Vec<f32>, w_col: Vec<f32>, x: Vec<f32>, y: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let nin = n; // n is the input dimension; NOUT fixed at 64\n    let nout = NOUT;\n    // Same values both layouts — only the layout differs.\n    let vals = |i: usize, j: usize| {\n        let h = (i as u32).wrapping_mul(0x9E37_79B9) ^ (j as u32).wrapping_mul(0x85EB_CA6B);\n        ((h >> 8) as f32) / 16777216.0 - 0.5\n    };\n    let w_row: Vec<f32> = (0..nout).flat_map(|r| (0..nin).map(move |c| vals(r, c))).collect();\n    let w_col: Vec<f32> = (0..nin).flat_map(|c| (0..nout).map(move |r| vals(r, c))).collect();\n    let x: Vec<f32> = (0..nin).map(|c| vals(9999, c)).collect();\n    Input { w_row, w_col, x, y: vec![0.0; nout] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let nin = input.x.len();\n    buggy::matvec(&input.w_row, &input.x, &mut input.y.clone(), NOUT, nin);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let nin = input.x.len();\n    fixed::matvec(&input.w_col, &input.x, &mut input.y.clone(), NOUT, nin);\n}".to_string(),
    }
}

/// Bench spec for `newton-refinement-vs-hardware-transcendental`:
/// seed + Newton-refined `inv`/`rsqrt` vs the plain hardware ops. On CPU
/// the hardware op is single-instruction + correctly rounded, so the
/// refinement steps are pure extra work. **Measured (Batch 62, M3 Max):
/// Mixed — hardware never loses: 1.09x at 1K, 1.01x at 100K** (source
/// measured 2-4% on Ryzen 5 5600H; the M3's fdiv/sqrt latency flattens
/// the gap at 100K where the pipeline hides it). **x86 re-gate
/// (2026-08-19, 4090 box): ParetoDominant 1.64x/1.66x — the win is
/// LARGER on x86; the near-tie is M3-aarch64-specific.** The rule's purpose is
/// stopping the plausible-looking GPU→CPU fast-math port — the annotation
/// class (`filter-count-to-any` precedent). Well-conditioned [0.5, 2.0]
/// inputs — deterministic.
#[cfg(feature = "rust_perf_bench")]
fn newton_refinement_vs_hardware_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: "\nmod buggy {\n    pub fn fast_inv(x: f32) -> f32 {\n        let r = 1.0 / x;\n        let r = r * (2.0 - x * r);\n        r * (2.0 - x * r)\n    }\n    pub fn fast_rsqrt(x: f32) -> f32 {\n        let r = 1.0 / x.sqrt();\n        r * (0.5 * (3.0 - x * r * r))\n    }\n}\n\nmod fixed {\n    pub fn fast_inv(x: f32) -> f32 { 1.0 / x }\n    pub fn fast_rsqrt(x: f32) -> f32 { 1.0 / x.sqrt() }\n}\n\npub fn make_input(n: usize) -> Vec<f32> {\n    // Values in [0.5, 2.0]: no inf/nan/underflow in inv or rsqrt —\n    // deterministic, well-conditioned.\n    (0..n)\n        .map(|i| {\n            let h = (i as u32).wrapping_mul(0x9E37_79B9);\n            0.5 + ((h >> 8) as f32) / 8388608.0\n        })\n        .collect()\n}\n\npub fn run_buggy(input: &[f32]) {\n    let input = std::hint::black_box(input);\n    let acc: f32 = input\n        .iter()\n        .map(|&x| buggy::fast_inv(x) + buggy::fast_rsqrt(x))\n        .sum();\n    std::hint::black_box(acc);\n}\n\npub fn run_fixed(input: &[f32]) {\n    let input = std::hint::black_box(input);\n    let acc: f32 = input\n        .iter()\n        .map(|&x| fixed::fast_inv(x) + fixed::fast_rsqrt(x))\n        .sum();\n    std::hint::black_box(acc);\n}".to_string(),
    }
}

/// Bench spec for `loop-order-innermost-contiguous`: i-j-k vs i-k-j
/// nested-accumulation matmul. The inner loop's access pattern is the
/// whole story: column-stride-n vs contiguous row walk (L1 misses
/// 45%->12% in the source). Square sizes 64/128/256 — L1/L2-resident so
/// the access-pattern effect dominates. Float sum-order differs between
/// the orders — the validator's rel-error tolerance covers it (NOT
/// bit-identity; the `distance-sq-no-sqrt` 1-ulp caveat precedent).
#[cfg(feature = "rust_perf_bench")]
fn loop_order_innermost_contiguous_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![64, 128, 256],
        harness: "\nmod buggy {\n    pub fn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n        for i in 0..n {\n            for j in 0..n {\n                let mut acc = 0.0f32;\n                for k in 0..n {\n                    acc += a[i * n + k] * b[k * n + j]; // k strides b's OUTER axis\n                }\n                c[i * n + j] = acc;\n            }\n        }\n    }\n}\n\nmod fixed {\n    pub fn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n        for i in 0..n {\n            for j in 0..n {\n                c[i * n + j] = 0.0;\n            }\n        }\n        for i in 0..n {\n            let a_row = &a[i * n..(i + 1) * n];\n            for k in 0..n {\n                let a_ik = a_row[k];\n                let b_row = &b[k * n..(k + 1) * n];\n                let c_row = &mut c[i * n..(i + 1) * n];\n                for j in 0..n {\n                    c_row[j] += a_ik * b_row[j]; // j strides by 1 everywhere\n                }\n            }\n        }\n    }\n}\n\npub struct Input { a: Vec<f32>, b: Vec<f32>, c: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let vals = |s: u32| (0..n * n)\n        .map(move |i| {\n            let h = (i as u32).wrapping_mul(0x9E37_79B9).wrapping_add(s);\n            ((h >> 8) as f32) / 16_777_216.0 - 0.5\n        })\n        .collect();\n    Input { a: vals(1), b: vals(2), c: vec![0.0; n * n] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let n = (input.a.len() as f64).sqrt() as usize;\n    buggy::matmul(&input.a, &input.b, &mut input.c.clone(), n);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let n = (input.a.len() as f64).sqrt() as usize;\n    fixed::matmul(&input.a, &input.b, &mut input.c.clone(), n);\n}".to_string(),
    }
}

/// Bench spec for `debug-assert-length-hoist`: debug_assert vs assert
/// before an indexing loop. The whole point is RELEASE behavior
/// (debug_assert is erased there — in debug both assert forms run);
/// the validator builds generated crates in release, so the buggy side
/// retains every bounds check. **Measured (Batch 64, M3 aarch64):
/// Mixed regime-split — 1.13x at n=1K (elision win) but a STABLE
/// 0.67-0.71x LOSS at n=100K** (3-run verified; the article's own
/// aarch64 number was 1.03x; x86 re-gate 2026-08-19 on the 4090 box:
/// 1.00x tie @1K, 0.92x @100K — regime confirmed cross-arch, milder
/// loss; the x86 headline win does not reproduce on this shape). The
/// canonical range-pinned shape
/// (first+count <= len feeding the induction loop); data-dependent
/// index arrays cannot propagate the constraint at all (measured
/// 0.74-0.80x dominated on the first harness shape — elision is
/// impossible there, the assert is pure cost).
#[cfg(feature = "rust_perf_bench")]
fn debug_assert_length_hoist_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: "\nmod buggy {\n    pub fn range_checksum(rows: &[(u64, u64)], first: usize, count: usize,\n                          out: &mut Vec<u64>) {\n        debug_assert!(first + count <= rows.len()); // ERASED in release —\n        // the optimizer loses exactly this range constraint\n        for i in first..first + count {\n            out.push(rows[i].0 ^ rows[i].1); // bounds checks RETAINED\n        }\n    }\n}\n\nmod fixed {\n    pub fn range_checksum(rows: &[(u64, u64)], first: usize, count: usize,\n                          out: &mut Vec<u64>) {\n        assert!(first + count <= rows.len()); // SURVIVES release — the range\n        // constraint stays visible to the optimizer\n        for i in first..first + count {\n            out.push(rows[i].0 ^ rows[i].1); // bounds checks ELIDED\n        }\n    }\n}\n\npub struct Input { rows: Vec<(u64, u64)>, scratch: Vec<u64> }\n\npub fn make_input(n: usize) -> Input {\n    let rows: Vec<(u64, u64)> = (0..n)\n        .map(|i| {\n            let h = (i as u64).wrapping_mul(0x9E37_79B9_97F4_A7C1);\n            (h ^ (h >> 31), h.wrapping_mul(3))\n        })\n        .collect();\n    Input { rows, scratch: Vec::with_capacity(n) }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut out = input.scratch.clone();\n    buggy::range_checksum(&input.rows, 3, input.rows.len() - 3, &mut out);\n    std::hint::black_box(&out);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut out = input.scratch.clone();\n    fixed::range_checksum(&input.rows, 3, input.rows.len() - 3, &mut out);\n    std::hint::black_box(&out);\n}".to_string(),
    }
}

/// Bench spec for `sparse-set-bit-iteration-tz-blsr`: 64-iteration
/// shift-test scan vs trailing_zeros+BLSR loop over sparse masks.
///
/// ~4/64 set bits per word (the rule's applicable sparse regime —
/// `0x0101..` selects bit 0 of each byte, each set with p~1/2). The
/// buggy form always executes 64 shift+test iterations; the fixed form
/// executes popcount iterations (~4). Both emit identical index sums
/// (bit-identical G1 by construction). At dense masks the two converge —
/// the honest regime boundary recorded in the rule's annotation.
#[cfg(feature = "rust_perf_bench")]
fn sparse_set_bit_iteration_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: "\nmod buggy {\n    pub fn sum_set_positions(mask: u64) -> u32 {\n        let mut sum = 0;\n        for i in 0..64 {\n            if (mask >> i) & 1 != 0 { sum += i as u32; }\n        }\n        sum\n    }\n}\n\nmod fixed {\n    pub fn sum_set_positions(mask: u64) -> u32 {\n        let mut sum = 0;\n        let mut m = mask;\n        while m != 0 {\n            let i = m.trailing_zeros();\n            sum += i;\n            m &= m.wrapping_sub(1);\n        }\n        sum\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u64> {\n    // ~4/64 set bits per word (sparse regime — the rule's applicable\n    // window): 0x0101.. selects bit 0 of each byte, each set with p~1/2\n    // under the multiplicative hash.\n    (0..n)\n        .map(|i| (i as u64).wrapping_mul(0x9E37_79B9_97F4_A7C1) & 0x0101_0101_0101_0101)\n        .collect()\n}\n\npub fn run_buggy(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let mut acc = 0u32;\n    for &m in input {\n        acc = acc.wrapping_add(buggy::sum_set_positions(m));\n    }\n    std::hint::black_box(acc);\n}\n\npub fn run_fixed(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let mut acc = 0u32;\n    for &m in input {\n        acc = acc.wrapping_add(fixed::sum_set_positions(m));\n    }\n    std::hint::black_box(acc);\n}\n".to_string(),
    }
}

/// Bench spec for `sortedness-check-branchless-accumulate`: early-exit
/// branch-per-element validation vs branchless compare+AND accumulate
/// on USUALLY-SORTED data.
///
/// Strictly increasing u64s (the applicable regime — the early-exit
/// branch never fires, so the buggy form pays scalar compare+branch per
/// element for nothing; the accumulate form has no exit and vectorizes).
/// On early-violation data the regime flips (early exit wins) — the
/// mirrored regime of `filter-count-to-any`, recorded in the annotation.
#[cfg(feature = "rust_perf_bench")]
fn sortedness_check_branchless_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000],
        harness: "\nmod buggy {\n    pub fn validate_sorted(vals: &[u64]) -> bool {\n        let mut prev = u64::MIN;\n        for &v in vals {\n            if v < prev { return false; }\n            prev = v;\n        }\n        true\n    }\n}\n\nmod fixed {\n    pub fn validate_sorted(vals: &[u64]) -> bool {\n        let mut sorted = true;\n        let mut prev = u64::MIN;\n        for &v in vals {\n            sorted &= prev <= v;\n            prev = v;\n        }\n        sorted\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u64> {\n    // Strictly increasing (usually-sorted regime — the applicable window:\n    // the early-exit branch never fires, so the buggy form pays a scalar\n    // compare+branch per element for nothing).\n    (0..n).map(|i| (i as u64).wrapping_mul(2)).collect()\n}\n\npub fn run_buggy(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let ok = buggy::validate_sorted(input);\n    std::hint::black_box(ok);\n}\n\npub fn run_fixed(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let ok = fixed::validate_sorted(input);\n    std::hint::black_box(ok);\n}\n".to_string(),
    }
}

/// Bench spec for `byte-find-position-to-memchr`: scalar early-exit
/// position scan vs the memchr crate's SIMD block scan.
///
/// Needle at n-16 (~90% position on the larger sizes — both arms do a
/// ~full scan; stray delimiters scrubbed in setup so both arms find the
/// same index). 32B = the honest small-regime arm (the Teddy-bound wash
/// where call overhead eats the win); 4K/64K = the SIMD win regime.
/// Deterministic positions; `Option<usize>` identical (bit-identical).
#[cfg(feature = "rust_perf_bench")]
fn byte_find_position_to_memchr_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![32, 256, 4_096, 65_536],
        harness: "// [dep: memchr = \"2\"]\nmod buggy {\n    pub fn find_delim(buf: &[u8]) -> Option<usize> {\n        buf.iter().position(|&b| b == b';')\n    }\n}\n\nmod fixed {\n    pub fn find_delim(buf: &[u8]) -> Option<usize> {\n        memchr::memchr(b';', buf)\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u8> {\n    // Pseudo-random bytes; stray delimiters scrubbed so the FIRST ';' sits\n    // at n-16 (both arms do ~full scan — the representative regime; the\n    // scrub runs in setup, never measured).\n    let mut v: Vec<u8> = (0..n)\n        .map(|i| ((i as u32).wrapping_mul(2654435761) >> 24) as u8)\n        .collect();\n    for b in &mut v {\n        if *b == b';' { *b = b':'; }\n    }\n    if n >= 16 {\n        v[n - 16] = b';';\n    }\n    v\n}\n\npub fn run_buggy(input: &[u8]) {\n    let input = std::hint::black_box(input);\n    let at = buggy::find_delim(input);\n    std::hint::black_box(at);\n}\n\npub fn run_fixed(input: &[u8]) {\n    let input = std::hint::black_box(input);\n    let at = fixed::find_delim(input);\n    std::hint::black_box(at);\n}\n".to_string(),
    }
}

/// Bench spec for `varint-compose-word-single-store` (Batch 68, Research
/// 075 — the tantivy distill): per-byte `write_all` writer calls vs the
/// 5-threshold ladder composing all bytes into one u64 + ONE `write_all`.
///
/// THE WRITER SHAPE IS THE RULE'S REGIME (first bench run, load-recorded:
/// the Vec-push shape measures ParetoDominated 0/3 — 128 is a compile-time
/// constant so LLVM strength-reduces the div/mod, and Vec::push-per-byte is
/// cheap enough that the ladder's if-chain loses; the honest win is N
/// writer calls → 1). Sink: `std::io::Cursor<&mut Vec<u8>>` — a real
/// generic `Write` impl with per-call branching but no syscall noise; on a
/// virtual `Box<dyn Write>` or a real file/socket writer the per-call cost
/// — and therefore the win — only grows. Input mixes ALL five serialized
/// widths deterministically (i % 5 selects the magnitude band). Stop-bit-
/// on-last convention; outputs bit-identical by the exhaustive corpus gate.
#[cfg(feature = "rust_perf_bench")]
fn varint_compose_word_single_store_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
use std::io::Write;

// Dyn-dispatch sink: the write path is IDENTICAL work (extend_from_slice)
// for both arms — the measured differential is purely N vtable calls vs 1
// (the production shape for trait-object writers; fully-inlined generic
// sinks measure ~parity, see the rule's context note).
struct Sink(Vec<u8>);
impl std::io::Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
fn new_sink(cap: usize) -> Box<dyn std::io::Write> {
    Box::new(Sink(Vec::with_capacity(cap)))
}

mod buggy {
    use std::io::{Result, Write};
    pub fn write_vint<W: Write>(w: &mut W, mut val: u32) -> Result<()> {
        loop {
            let byte = (val % 128) as u8;
            val /= 128;
            if val == 0 {
                w.write_all(&[byte | 128])?; // a writer call PER BYTE
                return Ok(());
            }
            w.write_all(&[byte])?;
        }
    }
}

mod fixed {
    use std::io::{Result, Write};
    pub fn write_vint<W: Write>(w: &mut W, val: u32) -> Result<()> {
        let v = u64::from(val);
        let (res, len) = if v < (1 << 7) {
            (v | 128, 1)
        } else if v < (1 << 14) {
            ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)
        } else if v < (1 << 21) {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)
        } else if v < (1 << 28) {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)
        } else {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)
        };
        w.write_all(&res.to_le_bytes()[..len]) // ONE store, ONE call
    }
}

pub struct Input {
    values: Vec<u32>,
    total_bytes: usize,
}

pub fn make_input(n: usize) -> Input {
    // Deterministic mix across all five magnitude bands (every ladder arm +
    // every loop trip count): band i%5 selects a value in the 1..5-byte range.
    let mut values = Vec::with_capacity(n);
    let mut total_bytes = 0usize;
    for i in 0..n {
        let band = (i % 5) as u32;
        let val: u32 = if band == 4 {
            u32::MAX - ((i as u32) % 64) // >= 2^28: 5-byte band
        } else {
            (1u32 << (7 * band)) + ((i as u32) & ((1u32 << (7 * band)) - 1))
        };
        values.push(val);
        total_bytes += band as usize + 1;
    }
    Input { values, total_bytes }
}

pub fn run_buggy(input: &Input) {
    let mut cur = new_sink(input.total_bytes + 16);
    for &v in std::hint::black_box(&input.values) {
        buggy::write_vint(&mut cur, v).unwrap();
    }
    std::hint::black_box(&cur);
}

pub fn run_fixed(input: &Input) {
    let mut cur = new_sink(input.total_bytes + 16);
    for &v in std::hint::black_box(&input.values) {
        fixed::write_vint(&mut cur, v).unwrap();
    }
    std::hint::black_box(&cur);
}"#
            .to_string(),
    }
}

/// Bench spec for `unaligned-word-load-from-bytes` (Batch 69, Research
/// 075 §Secondary — the Plan 013 T6 refutation): the 8-term
/// shift/or byte-assembly chain vs ONE unaligned `from_le_bytes` load.
///
/// **Cross-arch probe 2026-08-19 (median-of-5, interleaved arms): u64
/// 2.17-2.23x on aarch64 (M3, load ~118 shared) / 1.39-1.47x on x86 (4090
/// box, load 7-31); u32 1.27-1.38x / 1.20-1.29x — ParetoDominant BOTH
/// arches BOTH widths.** The mechanism is LOAD-COUNT (8 L1 loads + the
/// serial shift/or chain vs 1 fused load) — distinct from the varint
/// honest-loser's call-count class (OoO hides N dyn-dispatch calls; it
/// cannot hide 8 loads behind 1). The u32 family shares the buffer; the
/// harness measures the u64 headline (the family's worst-case-for-buggy
/// shape). G1: bit-identical at EVERY byte alignment (the exhaustive corpus
/// test below).
#[cfg(feature = "rust_perf_bench")]
fn unaligned_word_load_from_bytes_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000, 10_000_000],
        harness: r#"
mod buggy {
    pub fn read_u64_le(data: &[u8], addr: usize) -> u64 {
        (data[addr] as u64)
            | ((data[addr + 1] as u64) << 8)
            | ((data[addr + 2] as u64) << 16)
            | ((data[addr + 3] as u64) << 24)
            | ((data[addr + 4] as u64) << 32)
            | ((data[addr + 5] as u64) << 40)
            | ((data[addr + 6] as u64) << 48)
            | ((data[addr + 7] as u64) << 56)
    }
}

mod fixed {
    pub fn read_u64_le(data: &[u8], addr: usize) -> u64 {
        let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();
        u64::from_le_bytes(bytes)
    }
}

pub struct Input {
    data: Vec<u8>,
    idxs: Vec<usize>,
}

pub fn make_input(n: usize) -> Input {
    // 4 KiB buffer (L1-resident — the load-count mechanism isolated from
    // cache-miss noise) + a deterministic address scatter hitting every
    // byte alignment.
    const BUF: usize = 4096;
    let data: Vec<u8> = (0..BUF as u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    let idxs: Vec<usize> = (0..n).map(|i| i.wrapping_mul(48_271) % (BUF - 8)).collect();
    Input { data, idxs }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for &a in &input.idxs {
        acc = acc.wrapping_add(buggy::read_u64_le(&input.data, a));
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for &a in &input.idxs {
        acc = acc.wrapping_add(fixed::read_u64_le(&input.data, a));
    }
    std::hint::black_box(acc);
}"#
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_corpus_round_trips() {
        let corpus = build_perf_seed_corpus();
        assert!(corpus.verify());
        let frozen = corpus.freeze().unwrap();
        let thawed = Corpus::<RustPerfDomain>::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert!(thawed.verify());
    }

    #[test]
    fn seed_corpus_has_seventy_four_entries_across_twelve_categories() {
        let corpus = build_perf_seed_corpus();
        assert_eq!(corpus.entries.len(), 74);
        let mut categories: Vec<&str> =
            corpus.entries.iter().map(|e| e.category.as_str()).collect();
        categories.sort_unstable();
        categories.dedup();
        // 12 distinct categories. Allocation is the deepest (18 rules,
        // including Issue 014's arc-str-payload + Batch 48's three + Batch
        // 53's hashmap-with-capacity); complexity has 15 (Batch 48 added
        // hashset-insert-bool-return + vec-insert-front-deque; Batch 64
        // added debug-assert-length-hoist); iterators
        // has 9 (Batch 48 added three; Batch 66 added
        // sortedness-check-branchless-accumulate; Batch 67 added
        // byte-find-position-to-memchr); simd has 7 (Batch 62 added
        // transposed-weight-register-gemv; Batch 64 added
        // loop-order-innermost-contiguous); layout has 6 (Batch 53 added
        // soa-hot-field; Batch 61 added usize-index-to-u32) + concurrency
        // has 4 (Issue 014 added the two metric-mismatch rules); arithmetic
        // has 6 (Batch 53 added powi-vs-powf + distance-sq-no-sqrt; Batch 59
        // added division-by-invariant-multiplication; Batch 62 added
        // newton-refinement-vs-hardware-transcendental); dispatch has 2
        // (Batch 61 added keyword-map-to-match); caching has 2 (Issue 014
        // added regex-hoist-lazylock); io has 3 (Batch 68 added
        // varint-compose-word-single-store; Batch 69 added
        // unaligned-word-load-from-bytes); bevy, wasm have 1 each.
        assert_eq!(categories.len(), 12);
    }

    /// G1 for `varint-compose-word-single-store` (Batch 68, Research 075 —
    /// the tantivy distill): the 5-threshold ladder in the corpus fixture +
    /// bench harness must be exactly equivalent to the reference per-byte
    /// loop across a FULL u16 sweep + every 128^i band boundary + u32::MAX +
    /// an LCG u32 sample — bit-identical output bytes (the Batch 59
    /// ported-numeric exhaustive-gate rule; the source ships its own
    /// equivalence test at 128^i boundaries — `aux_test_serialize_vint_u32`
    /// — this gate is the stronger local mirror). BOTH conventions are
    /// proven: (a) stop-bit-on-LAST (tantivy VInt — the shipped fixture)
    /// and (b) continuation-bit-on-all-but-LAST (protobuf/LEB128 — the same
    /// ladder mechanism with the convention bit placed where the loop put
    /// it), demonstrating the fix is convention-faithful, not
    /// convention-specific.
    #[test]
    #[allow(clippy::too_many_lines)] // exhaustive gate: both conventions + boundaries belong together
    fn unaligned_word_load_matches_assembly_exhaustive() {
        // Batch 69 G1: the from_le_bytes slice read must be bit-identical to
        // the shift/or assembly at EVERY byte alignment over a wide buffer,
        // u64 AND u32 (the cross-arch probe's exhaustive half, pinned as a
        // standing gate).
        fn assemble_u64(data: &[u8], addr: usize) -> u64 {
            u64::from(data[addr])
                | (u64::from(data[addr + 1]) << 8)
                | (u64::from(data[addr + 2]) << 16)
                | (u64::from(data[addr + 3]) << 24)
                | (u64::from(data[addr + 4]) << 32)
                | (u64::from(data[addr + 5]) << 40)
                | (u64::from(data[addr + 6]) << 48)
                | (u64::from(data[addr + 7]) << 56)
        }
        fn load_u64(data: &[u8], addr: usize) -> u64 {
            let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();
            u64::from_le_bytes(bytes)
        }
        fn assemble_u32(data: &[u8], addr: usize) -> u32 {
            u32::from(data[addr])
                | (u32::from(data[addr + 1]) << 8)
                | (u32::from(data[addr + 2]) << 16)
                | (u32::from(data[addr + 3]) << 24)
        }
        fn load_u32(data: &[u8], addr: usize) -> u32 {
            let bytes: [u8; 4] = (&data[addr..addr + 4]).try_into().unwrap();
            u32::from_le_bytes(bytes)
        }

        let n = 4096u32;
        let data: Vec<u8> = (0..n)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
            .collect();

        for addr in 0..((n - 8) as usize) {
            assert_eq!(assemble_u64(&data, addr), load_u64(&data, addr), "u64 @ {addr}");
        }
        for addr in 0..((n - 4) as usize) {
            assert_eq!(assemble_u32(&data, addr), load_u32(&data, addr), "u32 @ {addr}");
        }
    }

    #[test]
    fn varint_ladder_matches_reference_loop_exhaustive() {
        // (a) The SHIPPED ladder — mirror of the corpus fixture (same math).
        fn ladder_stop_bit_last(val: u32, out: &mut Vec<u8>) {
            let v = u64::from(val);
            let (res, len) = if v < (1 << 7) {
                (v | 128, 1)
            } else if v < (1 << 14) {
                ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)
            } else if v < (1 << 21) {
                ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)
            } else if v < (1 << 28) {
                ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)
            } else {
                ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)
            };
            let buf = res.to_le_bytes();
            out.extend_from_slice(&buf[..len]);
        }
        // Reference per-byte loop (the buggy shape, used as ground truth).
        fn reference_stop_bit_last(mut val: u32, out: &mut Vec<u8>) {
            loop {
                let byte = (val % 128) as u8;
                val /= 128;
                if val == 0 {
                    out.push(byte | 128);
                    return;
                }
                out.push(byte);
            }
        }
        // (b) The protobuf/LEB128 convention ladder — same mechanism, the
        // continuation bit lands on every byte EXCEPT the last (bit 7 set
        // on bytes 0..len-2 of the composed word).
        fn ladder_continuation(val: u32, out: &mut Vec<u8>) {
            let v = u64::from(val);
            let (res, len) = if v < (1 << 7) {
                (v, 1)
            } else if v < (1 << 14) {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1), 2)
            } else if v < (1 << 21) {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1) | 0x80 << 8 | ((v & (127 << 14)) << 2), 3)
            } else if v < (1 << 28) {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1) | 0x80 << 8 | ((v & (127 << 14)) << 2) | 0x80 << 16 | ((v & (127 << 21)) << 3), 4)
            } else {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1) | 0x80 << 8 | ((v & (127 << 14)) << 2) | 0x80 << 16 | ((v & (127 << 21)) << 3) | 0x80 << 24 | ((v & (127 << 28)) << 4), 5)
            };
            let buf = res.to_le_bytes();
            out.extend_from_slice(&buf[..len]);
        }
        fn reference_continuation(mut val: u32, out: &mut Vec<u8>) {
            loop {
                if val < 128 {
                    // final byte: NO continuation bit (u8::try_from is
                    // infallible here — guarded by the val < 128 check)
                    out.push(u8::try_from(val).unwrap());
                    return;
                }
                out.push((val % 128) as u8 | 0x80);
                val /= 128;
            }
        }

        fn check_pair(val: u32) {
            let mut a = Vec::new();
            let mut b = Vec::new();
            ladder_stop_bit_last(val, &mut a);
            reference_stop_bit_last(val, &mut b);
            assert_eq!(a, b, "stop-bit ladder diverged at val={val}");
            let mut c = Vec::new();
            let mut d = Vec::new();
            ladder_continuation(val, &mut c);
            reference_continuation(val, &mut d);
            assert_eq!(c, d, "continuation ladder diverged at val={val}");
        }

        // Full u16 sweep (exhaustive low range).
        for val in 0..=u32::from(u16::MAX) {
            check_pair(val);
        }
        // Every band boundary ±1 (128^i) + the extremes.
        for i in 1..5u32 {
            let p: u32 = 1 << (7 * i);
            for val in [p - 1, p, p + 1] {
                check_pair(val);
            }
        }
        check_pair(u32::MAX);
        check_pair(u32::MAX - 1);
        // LCG u32 sample (16K values across the full range).
        let mut state = 0x853c_49e6_748f_ea9b_u64;
        for _ in 0..16_384 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            check_pair((state >> 32) as u32);
        }
    }

    /// Canaries: the corpus fixture + the bench harness must contain the
    /// SAME ladder construction this exhaustive gate proves — a substring
    /// pin so the three copies (fixture / harness / gate mirror) cannot
    /// drift (the Batch 59 canary pattern).
    #[test]
    fn varint_fixture_and_harness_carry_the_proven_ladder() {
        let corpus = build_perf_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "varint-compose-word-single-store")
            .expect("varint rule present");
        let needle = "((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2)";
        assert!(
            rule.fixed_code.contains(needle),
            "fixture fixed_code must carry the proven 3+-band ladder fragment"
        );
        assert!(rule.buggy_code.contains("val /= 128"),
            "fixture buggy_code must be the per-byte div-chain loop");
        #[cfg(feature = "rust_perf_bench")]
        {
            let spec = rule.bench_spec.as_ref().expect("bench spec");
            assert!(
                spec.harness.contains(needle),
                "bench harness must carry the same ladder construction"
            );
        }
    }

    /// G1 for `division-by-invariant-multiplication`: the magic-division
    /// algorithm in the corpus fixture + bench harness must be exactly
    /// equivalent to hardware `/` across an exhaustive low-divisor sweep +
    /// boundary numerators + selected large divisors (Batch 59). A mirror
    /// of the harness impl lives here (same math); under
    /// `rust_perf_bench` a substring canary additionally pins the harness
    /// string to the same construction lines so the two cannot drift.
    #[test]
    #[allow(clippy::too_many_lines)] // exhaustive gate: the u32/u64 twins + shared edge data belong together
    fn division_magic_matches_hardware_div_exhaustive_low_d() {
        struct FastDivU32 { magic: u32, shift: u32, add: bool }
        #[allow(clippy::cast_possible_truncation)] // libdivide magic: the high-word extraction + magic wrap ARE the algorithm
        impl FastDivU32 {
            fn new(d: u32) -> Self {
                let l = d.ilog2();
                if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
                let k = 1u64 << l;
                let base = (k << 32) / u64::from(d);
                let rem = (k << 32) % u64::from(d);
                if u64::from(d) - rem < k {
                    Self { magic: base as u32 + 1, shift: l, add: false }
                } else {
                    let mut m = base * 2;
                    let rem2 = rem * 2;
                    if rem2 >= u64::from(d) { m += 1; }
                    Self { magic: (m + 1) as u32, shift: l, add: true }
                }
            }
            fn div(&self, x: u32) -> u32 {
                if self.magic == 0 { return x >> self.shift; }
                let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;
                if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift }
            }
        }
        struct FastDivU64 { magic: u64, shift: u32, add: bool }
        #[allow(clippy::cast_possible_truncation)] // libdivide magic: the high-word extraction + magic wrap ARE the algorithm
        impl FastDivU64 {
            fn new(d: u64) -> Self {
                let l = d.ilog2();
                if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
                let k = 1u128 << l;
                let base = (k << 64) / u128::from(d);
                let rem = (k << 64) % u128::from(d);
                if u128::from(d) - rem < k {
                    Self { magic: base as u64 + 1, shift: l, add: false }
                } else {
                    let mut m = base * 2;
                    let rem2 = rem * 2;
                    if rem2 >= u128::from(d) { m += 1; }
                    Self { magic: (m + 1) as u64, shift: l, add: true }
                }
            }
            fn div(&self, x: u64) -> u64 {
                if self.magic == 0 { return x >> self.shift; }
                let hi = ((u128::from(self.magic) * u128::from(x)) >> 64) as u64;
                if self.add { ((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64 } else { hi >> self.shift }
            }
        }

        struct Lcg(u64);
        impl Lcg {
            fn next_u64(&mut self) -> u64 {
                self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                self.0
            }
            fn next_u32(&mut self) -> u32 { (self.next_u64() >> 33) as u32 }
        }
        let mut rng = Lcg(0x2545_F491_4F6C_DD1D);

        // Exhaustive low divisors (all u32 d in 1..=4096, all u64 d in 1..=257)
        for d in 1u32..=4096 {
            let fd = FastDivU32::new(d);
            let probe = |x: u32| {
                assert_eq!(fd.div(x), x / d, "u32 mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d + 1);
            probe(d.saturating_mul(2).saturating_sub(1)); probe(u32::MAX);
            for _ in 0..8 { probe(rng.next_u32()); }
        }
        for d in 1u64..=257 {
            let fd = FastDivU64::new(d);
            let probe = |x: u64| {
                assert_eq!(fd.div(x), x / d, "u64 mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d + 1);
            probe(d.saturating_mul(2).saturating_sub(1)); probe(u64::MAX);
            for _ in 0..8 {
                probe(rng.next_u64());
            }
        }
        // Selected large/edge divisors: powers of two, primes near 2^31,
        // 2^32, 2^63, 2^64, and values that exercise the ADD branch + the
        // sentinel boundary (magic wrapping near 0).
        let edge32: &[u32] = &[65_535, 65_536, 65_537, 1_000_003, 2_147_483_647, 2_147_483_648, 3_000_000_001, 4_294_967_291, 4_294_967_295, 3, 5, 7, 9, 11];
        for &d in edge32 {
            let fd = FastDivU32::new(d);
            let probe = |x: u32| {
                assert_eq!(fd.div(x), x / d, "u32 edge mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d.saturating_add(1));
            probe(u32::MAX - 1); probe(u32::MAX);
            for _ in 0..16 { probe(rng.next_u32()); }
        }
        let edge64: &[u64] = &[3, 5, 7, 9, 11, 65_537, 1_000_003, 4_294_967_291, 4_294_967_296, 4_294_967_311, 9_223_372_036_854_775_783, 9_223_372_036_854_775_808, 9_223_372_036_854_775_809, 18_446_744_073_709_551_557, u64::MAX - 1, u64::MAX];
        for &d in edge64 {
            let fd = FastDivU64::new(d);
            let probe = |x: u64| {
                assert_eq!(fd.div(x), x / d, "u64 edge mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d.saturating_add(1));
            probe(u64::MAX - 1); probe(u64::MAX);
            for _ in 0..16 {
                probe(rng.next_u64());
            }
        }

        // Canary: the harness string must carry the same construction core
        // (drift between this mirror + the spec = silent correctness rot).
        #[cfg(feature = "rust_perf_bench")]
        {
            let spec = division_invariant_bench_spec();
            for needle in [
                "let base = (k << 32) / u64::from(d);",
                "let base = (k << 64) / u128::from(d);",
                "((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32",
                "((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64",
            ] {
                assert!(spec.harness.contains(needle), "harness drifted from the equivalence-test mirror: missing `{needle}`");
            }
        }
    }

    #[test]
    fn seed_corpus_rule_ids_are_unique() {
        let corpus = build_perf_seed_corpus();
        let mut ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate rule_id in seed corpus");
    }

    #[test]
    fn tamper_detected() {
        let corpus = build_perf_seed_corpus();
        let mut tampered = corpus.clone();
        tampered.entries[0].fixed_code = "// tampered".to_string();
        assert!(!tampered.verify());
    }

    #[test]
    fn heal_entry_accessors_return_correct_fields() {
        use crate::domain::HealEntry;
        let corpus = build_perf_seed_corpus();
        // Corpus is sorted by rule_id alphabetically — find by id, not index.
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "o1-lookup")
            .expect("o1-lookup should be in seed corpus");
        assert_eq!(entry.rule_id(), "o1-lookup");
        assert!(!entry.buggy_code().is_empty());
        assert!(!entry.fixed_code().is_empty());
    }

    #[test]
    fn heal_domain_build_corpus_matches_seed() {
        use crate::domain::HealDomain;
        let via_trait = RustPerfDomain::build_corpus();
        let via_seed = build_perf_seed_corpus().entries;
        assert_eq!(via_trait.len(), via_seed.len());
        // Both go through the same sort, so the order is identical.
        assert_eq!(via_trait.first().map(|e| e.rule_id.as_str()), via_seed.first().map(|e| e.rule_id.as_str()));
        assert_eq!(via_trait.last().map(|e| e.rule_id.as_str()), via_seed.last().map(|e| e.rule_id.as_str()));
    }

    #[test]
    #[cfg(not(feature = "rust_perf_verify"))]
    fn validate_parses_valid_code() {
        use crate::domain::HealDomain;
        let verdict = RustPerfDomain::validate(
            "o1-lookup",
            "fn foo() -> i32 { 42 }",
        );
        assert!(verdict.parses);
    }

    #[test]
    #[cfg(not(feature = "rust_perf_verify"))]
    fn validate_rejects_garbage() {
        use crate::domain::HealDomain;
        let verdict = RustPerfDomain::validate("o1-lookup", "not valid rust {");
        assert!(!verdict.parses);
    }

    #[cfg(feature = "rust_perf_bench")]
    #[test]
    fn sixty_four_rules_have_bench_specs() {
        let corpus = build_perf_seed_corpus();
        let with_spec = corpus.entries.iter().filter(|e| e.bench_spec.is_some()).count();
        assert_eq!(with_spec, 64, "expected 64 benchable rules, got {with_spec}");
    }

    #[cfg(feature = "rust_perf_bench")]
    #[test]
    fn ten_rules_are_not_benchable() {
        let corpus = build_perf_seed_corpus();
        let without_spec: Vec<&str> = corpus
            .entries
            .iter()
            .filter(|e| e.bench_spec.is_none())
            .map(|e| e.rule_id.as_str())
            .collect();
        assert_eq!(without_spec.len(), 10);
        assert!(without_spec.contains(&"layout-reorder"));
        assert!(without_spec.contains(&"repr-u8-enum"));
        assert!(without_spec.contains(&"bevy-reactive-loop"));
        assert!(without_spec.contains(&"wasm-batch-ffi"));
        assert!(without_spec.contains(&"shrink-to-fit-after-build"));
        assert!(without_spec.contains(&"box-large-enum-variant"));
        assert!(without_spec.contains(&"async-yield-point"));
        assert!(without_spec.contains(&"async-future-size"));
        // Issue 014 T2.3/T2.4 — NotBenchable metric-mismatch class.
        assert!(without_spec.contains(&"tokio-guard-across-await"));
        assert!(without_spec.contains(&"bounded-channel-backpressure"));
    }

    /// Bench 013 found 4 rules with non-dominant verdicts in the default
    /// config (2 ParetoDominated + 2 Mixed). Their `context_notes` annotation
    /// records the finding so consumers don't unconditionally suggest them.
    /// Bench 016 (Issue 012) added 2 more annotated rules (sort-by-key,
    /// copy-from-slice). Issue 014 T2.3/T2.4 added 2 more in the
    /// NotBenchable metric-mismatch class (tokio-guard-across-await,
    /// bounded-channel-backpressure). Batch 48 added 2 verdict-class rules
    /// (filter-count-to-any ParetoDominated, iter-copied-deref-map Mixed).
    /// Batch 53 (Bench 038) added 2 more (powi-vs-powf all-Tie Mixed,
    /// soa-hot-field latency-bound Mixed). Batch 62 added 1 (the
    /// newton-refinement regime-inversion annotation) + Batch 64 added 1
    /// (debug-assert-length-hoist, the release-erasure class) + Batch 66
    /// added 1 (sortedness-check-branchless-accumulate, the regime-split
    /// class — the mirrored regime of filter-count-to-any). The other 56
    /// have `context_notes: None` (unconditional wins or honestly NotBenchable).
    #[test]
    fn fifteen_annotated_rules_carry_context_notes() {
        let corpus = build_perf_seed_corpus();
        let annotated: Vec<&str> = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_some())
            .map(|e| e.rule_id.as_str())
            .collect();
        assert_eq!(annotated.len(), 16, "expected 16 context-annotated rules");
        assert!(annotated.contains(&"precompute-table"));
        assert!(annotated.contains(&"papaya-lockfree"));
        assert!(annotated.contains(&"dedupe-hashset-vs-vec"));
        assert!(annotated.contains(&"reuse-allocation-clear"));
        assert!(annotated.contains(&"sort-by-key"));
        assert!(annotated.contains(&"copy-from-slice"));
        assert!(annotated.contains(&"tokio-guard-across-await"));
        assert!(annotated.contains(&"bounded-channel-backpressure"));
        assert!(annotated.contains(&"filter-count-to-any"));
        assert!(annotated.contains(&"iter-copied-deref-map"));
        assert!(annotated.contains(&"powi-vs-powf"));
        assert!(annotated.contains(&"soa-hot-field"));
        assert!(annotated.contains(&"newton-refinement-vs-hardware-transcendental"));
        assert!(annotated.contains(&"sortedness-check-branchless-accumulate"));
        assert!(annotated.contains(&"debug-assert-length-hoist"));
    }

    /// The non-annotated rules (ParetoDominant + honestly NotBenchable) must
    /// have `context_notes: None` — they are either unconditional wins or
    /// honestly unmeasurable.
    #[test]
    fn unannotated_rules_have_no_context_notes() {
        let corpus = build_perf_seed_corpus();
        let unannotated = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_none())
            .count();
        assert_eq!(unannotated, 58, "expected 58 unannotated rules (74 - 16 annotated)");
    }

    /// `context_notes` is backward-compatible via `#[serde(default)]`: an older
    /// freeze/thaw snapshot (pre-this-field) deserializes as `None`.
    #[test]
    fn context_notes_backward_compatible_serde() {
        let corpus = build_perf_seed_corpus();
        // Serialize + deserialize round-trip preserves the annotation.
        let json = serde_json::to_string(&corpus.entries).unwrap();
        let back: Vec<PerfRule> = serde_json::from_str(&json).unwrap();
        let annotated = back.iter().filter(|e| e.context_notes.is_some()).count();
        assert_eq!(annotated, 16);

        // Simulate a pre-field snapshot: strip `context_notes` from JSON.
        // (A real old snapshot simply omits the key; serde(default) → None.)
        let old_json = json.replace("\"context_notes\":", "\"__removed_context_notes\":");
        let old_back: Vec<PerfRule> = serde_json::from_str(&old_json).unwrap();
        let old_annotated = old_back.iter().filter(|e| e.context_notes.is_some()).count();
        assert_eq!(old_annotated, 0, "missing field should default to None");
    }

    /// Integration: the drafter propagates `context_notes` from `PerfRule`
    /// through `DraftedFix` for the annotated rules, and `None` for the
    /// unconditional wins. Sixteen annotations: Bench 013 (2 ParetoDominated
    /// and 2 Mixed), Bench 016 (sort-by-key and copy-from-slice), Issue 014
    /// (2 NotBenchable metric-mismatch), Batch 48 (2 verdict-class),
    /// Batch 53 (Bench 038, 2 verdict-class), Batch 62 (1 regime-class),
    /// Batch 64 (1 release-erasure class), Batch 66 (1 regime-split
    /// class), and Batch 68 (1 ParetoDominated honest-loser — the tantivy
    /// varint ladder regresses on aarch64 across 3 tested writer regimes).
    #[test]
    fn drafter_propagates_context_notes_from_perf_corpus() {
        use crate::domain::HealEntry;

        let corpus = build_perf_seed_corpus();

        // Filter to the 16 annotated rules.
        let annotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_some()).collect();
        assert_eq!(annotated.len(), 16, "expected 16 annotated rules");

        // The HealEntry trait override must agree with the struct field.
        for rule in &annotated {
            assert_eq!(
                HealEntry::context_notes(*rule),
                rule.context_notes.as_deref(),
                "HealEntry::context_notes() must match the PerfRule field for {}",
                rule.rule_id
            );
            assert!(
                rule.context_notes.as_ref().unwrap().contains("ParetoDominated")
                    || rule.context_notes.as_ref().unwrap().contains("Mixed")
                    || rule.context_notes.as_ref().unwrap().contains("NotBenchable"),
                "annotation should mention ParetoDominated, Mixed, or the \
                 NotBenchable metric-mismatch class for {}",
                rule.rule_id
            );
        }

        // The unannotated rules must return None from the trait too.
        let unannotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_none()).collect();
        assert_eq!(unannotated.len(), 58, "expected 58 unannotated rules");
        for rule in &unannotated {
            assert!(HealEntry::context_notes(*rule).is_none(), "{} should have no context_notes", rule.rule_id);
        }
    }
}
