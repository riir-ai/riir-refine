//! Seed corpus — rust performance rules distilled from the `rust-optimize`
//! skill checklist.
//!
//! Each entry is a `(rule_id, name, category, buggy_code, fixed_code, context_notes)`
//! tuple representing a real perf pattern from the skill. The corpus is
//! BLAKE3-committed via [`Corpus::new`](crate::corpus::Corpus::new). The
//! `context_notes` field is `Option<String>` — `Some(...)` for rules the bench
//! validator found ParetoDominated in the default config (Bench 013), `None`
//! for unconditional wins + honestly NotBenchable rules.
//!
//! 22 entries across 11 categories (complexity 6, allocation 3, simd 3,
//! layout 2, concurrency 2, arithmetic 1, iterators 1, dispatch 1, caching 1,
//! bevy 1, wasm 1). 18 are benchable (15 ParetoDominant + 2 ParetoDominated +
//! 1 Mixed); 4 are honestly NotBenchable.
//!
//! ## Bench specs (behind `rust_perf_bench`)
//!
//! Benchable rules carry an optional [`BenchSpec`] — a self-contained harness
//! embedding both buggy + fixed code inline + defining `make_input` /
//! `run_buggy` / `run_fixed` entry points. The bench validator generates a
//! temp bin crate from the spec + runs a timing loop at multiple sizes.
//!
//! Currently benchable (18/22):
//! - `o1-lookup` (Vec scan vs HashMap lookup — clear O(n) vs O(1))
//! - `bsearch-sorted-slice` (linear scan vs binary search — O(n) vs O(log n) on sorted data)
//! - `dedupe-hashset-vs-vec` (Vec::contains loop vs HashSet collect — O(n²) vs O(n))
//! - `entry-vs-contains-insert` (contains_key+insert vs entry API — merge HashMap lookups)
//! - `loop-fusion` (two iterator passes vs one fused loop — cache locality, merge loops)
//! - `prefix-sum-2d` (O(n) region scan vs O(1) prefix sum — boundary-vs-volume, grows with N)
//! - `static-str` (String alloc vs &'static str — allocation cost)
//! - `simd-chunk` (scalar vs chunked loop — vectorization at large N)
//! - `branchless-conditional` (if vs bool-as-usize multiply — vectorization + branch prediction)
//! - `bool-to-bitpack` (Vec<bool> count vs packed Vec<u64> POPCNT — data density)
//! - `mutex-to-atomic` (Mutex<u32> vs AtomicU32 — lock/unlock overhead)
//! - `f32-counter-to-int` (f32 accumulator vs u64 — serial dep chain)
//! - `collect-avoidance` (.collect() intermediate vs chained iterator — allocation + double pass)
//! - `dyn-to-enum` (Box<dyn Trait> vs enum — vtable indirection + no inlining)
//! - `precompute-table` (recompute vs const lookup table — **context-dependent**,
//!   see `context_notes`; not a release-mode win per Bench 013)
//! - `papaya-lockfree` (RwLock vs papaya lock-free — **context-dependent**,
//!   see `context_notes`; only a win under multi-threaded contention)
//! - `vec-with-capacity` (Vec::new() growth vs Vec::with_capacity — realloc cost)
//! - `format-concat-loop` (format! concat in loop vs push_str — O(n²) vs O(n))
//!
//! Not benchable (4/22) — honest:
//! - `layout-reorder` (struct size — deterministic `size_of`, not a runtime
//!   microbench; the cache-utilization claim needs a specific access pattern)
//! - `repr-u8-enum` (enum size — deterministic `size_of`, same class as
//!   `layout-reorder`)
//! - `bevy-reactive-loop` (needs the Bevy runtime — can't run in a temp crate)
//! - `wasm-batch-ffi` (needs a WASM runtime — can't run in a temp crate)
//!
//! ## Context-dependent rules (`context_notes`)
//!
//! Three rules carry a `context_notes: Some(...)` annotation recording that the
//! Bench 013 Pareto validator found them **ParetoDominated** or **Mixed** in the
//! default configuration (`precompute-table`, `papaya-lockfree`,
//! `dedupe-hashset-vs-vec`). The patterns are genuine wins in their respective
//! regimes (non-POPCNT targets / multi-threaded contention / large N) and are
//! kept in the corpus as documented patterns, but consumers should treat them
//! as context-dependent — not unconditional suggestions.

use crate::corpus::Corpus;
use crate::domains::rust_perf::{PerfRule, RustPerfDomain};

/// Build a seed corpus from common rust performance rules.
///
/// Returns a BLAKE3-committed [`Corpus<RustPerfDomain>`] with 22 entries
/// covering the main `rust-optimize` skill categories (complexity, allocation,
/// layout, concurrency, simd, arithmetic, iterators, dispatch, caching, bevy,
/// wasm). Complexity is the deepest category (6 rules: 3 algorithmic + entry
/// merge + loop fusion + prefix-sum-2d); allocation has 3; simd
/// has 3; layout + concurrency have 2; the rest have 1 each.
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
    fn seed_corpus_has_twenty_two_entries_across_eleven_categories() {
        let corpus = build_perf_seed_corpus();
        assert_eq!(corpus.entries.len(), 22);
        let mut categories: Vec<&str> =
            corpus.entries.iter().map(|e| e.category.as_str()).collect();
        categories.sort_unstable();
        categories.dedup();
        // 11 distinct categories. Complexity is the deepest (6 rules);
        // allocation + simd have 3 each; layout + concurrency have 2;
        // arithmetic, iterators, dispatch, caching, bevy, wasm have 1 each.
        assert_eq!(categories.len(), 11);
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
    fn eighteen_rules_have_bench_specs() {
        let corpus = build_perf_seed_corpus();
        let with_spec = corpus.entries.iter().filter(|e| e.bench_spec.is_some()).count();
        assert_eq!(with_spec, 18, "expected 18 benchable rules, got {with_spec}");
    }

    #[cfg(feature = "rust_perf_bench")]
    #[test]
    fn four_rules_are_not_benchable() {
        let corpus = build_perf_seed_corpus();
        let without_spec: Vec<&str> = corpus
            .entries
            .iter()
            .filter(|e| e.bench_spec.is_none())
            .map(|e| e.rule_id.as_str())
            .collect();
        assert_eq!(without_spec.len(), 4);
        assert!(without_spec.contains(&"layout-reorder"));
        assert!(without_spec.contains(&"repr-u8-enum"));
        assert!(without_spec.contains(&"bevy-reactive-loop"));
        assert!(without_spec.contains(&"wasm-batch-ffi"));
    }

    /// Bench 013 found 3 rules with non-dominant verdicts in the default
    /// config (2 ParetoDominated + 1 Mixed). Their `context_notes` annotation
    /// records the finding so consumers don't unconditionally suggest them.
    /// The other 19 have `context_notes: None` (unconditional wins or honestly
    /// NotBenchable).
    #[test]
    fn three_annotated_rules_carry_context_notes() {
        let corpus = build_perf_seed_corpus();
        let annotated: Vec<&str> = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_some())
            .map(|e| e.rule_id.as_str())
            .collect();
        assert_eq!(annotated.len(), 3, "expected 3 context-annotated rules");
        assert!(annotated.contains(&"precompute-table"));
        assert!(annotated.contains(&"papaya-lockfree"));
        assert!(annotated.contains(&"dedupe-hashset-vs-vec"));
    }

    /// The 19 non-annotated rules (15 ParetoDominant + 4 NotBenchable) must
    /// have `context_notes: None` — they are either unconditional wins or
    /// honestly unmeasurable.
    #[test]
    fn nineteen_rules_have_no_context_notes() {
        let corpus = build_perf_seed_corpus();
        let unannotated = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_none())
            .count();
        assert_eq!(unannotated, 19, "expected 19 unannotated rules");
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
        assert_eq!(annotated, 3);

        // Simulate a pre-field snapshot: strip `context_notes` from JSON.
        // (A real old snapshot simply omits the key; serde(default) → None.)
        let old_json = json.replace("\"context_notes\":", "\"__removed_context_notes\":");
        let old_back: Vec<PerfRule> = serde_json::from_str(&old_json).unwrap();
        let old_annotated = old_back.iter().filter(|e| e.context_notes.is_some()).count();
        assert_eq!(old_annotated, 0, "missing field should default to None");
    }

    /// Integration: the drafter propagates `context_notes` from `PerfRule`
    /// through `DraftedFix` for the 3 annotated rules (2 ParetoDominated + 1
    /// Mixed), and `None` for the 12 unconditional wins. This is the Plan 001
    /// post-promotion wiring (2026-08-13) — the field existed on `PerfRule`
    /// but was invisible to the domain-agnostic drafter until
    /// `HealEntry::context_notes()` landed.
    #[test]
    fn drafter_propagates_context_notes_from_perf_corpus() {
        use crate::domain::HealEntry;

        let corpus = build_perf_seed_corpus();

        // Filter to the 3 annotated rules.
        let annotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_some()).collect();
        assert_eq!(annotated.len(), 3, "expected 3 annotated rules");

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
                    || rule.context_notes.as_ref().unwrap().contains("Mixed"),
                "annotation should mention ParetoDominated or Mixed for {}",
                rule.rule_id
            );
        }

        // The 19 unannotated rules must return None from the trait too.
        let unannotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_none()).collect();
        assert_eq!(unannotated.len(), 19, "expected 19 unannotated rules");
        for rule in &unannotated {
            assert!(HealEntry::context_notes(*rule).is_none(), "{} should have no context_notes", rule.rule_id);
        }
    }
}
