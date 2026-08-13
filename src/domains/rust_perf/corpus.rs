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
//! ## Bench specs (behind `rust_perf_bench`)
//!
//! Benchable rules carry an optional [`BenchSpec`] — a self-contained harness
//! embedding both buggy + fixed code inline + defining `make_input` /
//! `run_buggy` / `run_fixed` entry points. The bench validator generates a
//! temp bin crate from the spec + runs a timing loop at multiple sizes.
//!
//! Currently benchable (7/11):
//! - `o1-lookup` (Vec scan vs HashMap lookup — clear O(n) vs O(1))
//! - `static-str` (String alloc vs &'static str — allocation cost)
//! - `simd-chunk` (scalar vs chunked loop — vectorization at large N)
//! - `precompute-table` (recompute vs const lookup table — **context-dependent**,
//!   see `context_notes`; not a release-mode win per Bench 013)
//! - `papaya-lockfree` (RwLock vs papaya lock-free — **context-dependent**,
//!   see `context_notes`; only a win under multi-threaded contention)
//! - `vec-with-capacity` (Vec::new() growth vs Vec::with_capacity — realloc cost)
//! - `format-concat-loop` (format! concat in loop vs push_str — O(n²) vs O(n))
//!
//! Not benchable (4/11) — honest:
//! - `layout-reorder` (struct size — deterministic `size_of`, not a runtime
//!   microbench; the cache-utilization claim needs a specific access pattern)
//! - `repr-u8-enum` (enum size — deterministic `size_of`, same class as
//!   `layout-reorder`)
//! - `bevy-reactive-loop` (needs the Bevy runtime — can't run in a temp crate)
//! - `wasm-batch-ffi` (needs a WASM runtime — can't run in a temp crate)
//!
//! ## Context-dependent rules (`context_notes`)
//!
//! Two rules carry a `context_notes: Some(...)` annotation recording that the
//! Bench 013 Pareto validator found them **ParetoDominated** in the default
//! configuration (`precompute-table`, `papaya-lockfree`). The patterns are
//! genuine wins in their respective regimes (non-POPCNT targets / multi-
//! threaded contention) and are kept in the corpus as documented patterns,
//! but consumers should treat them as context-dependent — not unconditional
//! suggestions.

use crate::corpus::Corpus;
use crate::domains::rust_perf::{PerfRule, RustPerfDomain};

/// Build a seed corpus from common rust performance rules.
///
/// Returns a BLAKE3-committed [`Corpus<RustPerfDomain>`] with 11 entries
/// covering the main `rust-optimize` skill categories (complexity, allocation,
/// layout, concurrency, simd, caching, bevy, wasm). Allocation is the deepest
/// category (3 rules: `static-str`, `vec-with-capacity`, `format-concat-loop`)
/// — it is the most common real-world Rust perf anti-pattern.
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
    fn seed_corpus_has_eleven_entries_across_eight_categories() {
        let corpus = build_perf_seed_corpus();
        assert_eq!(corpus.entries.len(), 11);
        let mut categories: Vec<&str> =
            corpus.entries.iter().map(|e| e.category.as_str()).collect();
        categories.sort_unstable();
        categories.dedup();
        // 8 distinct categories. Allocation is the deepest (3 rules:
        // static-str, vec-with-capacity, format-concat-loop); layout has 2
        // (layout-reorder, repr-u8-enum); the rest have 1 each.
        assert_eq!(categories.len(), 8);
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
    fn seven_rules_have_bench_specs() {
        let corpus = build_perf_seed_corpus();
        let with_spec = corpus.entries.iter().filter(|e| e.bench_spec.is_some()).count();
        assert_eq!(with_spec, 7, "expected 7 benchable rules, got {with_spec}");
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

    /// Bench 013 found 2 rules ParetoDominated in the default config. Their
    /// `context_notes` annotation records the finding so consumers don't
    /// unconditionally suggest them. The other 9 have `context_notes: None`
    /// (unconditional wins or honestly NotBenchable).
    #[test]
    fn two_dominated_rules_carry_context_notes() {
        let corpus = build_perf_seed_corpus();
        let annotated: Vec<&str> = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_some())
            .map(|e| e.rule_id.as_str())
            .collect();
        assert_eq!(annotated.len(), 2, "expected 2 context-annotated rules");
        assert!(annotated.contains(&"precompute-table"));
        assert!(annotated.contains(&"papaya-lockfree"));
    }

    /// The 9 non-annotated rules (5 ParetoDominant + 4 NotBenchable) must
    /// have `context_notes: None` — they are either unconditional wins or
    /// honestly unmeasurable.
    #[test]
    fn nine_rules_have_no_context_notes() {
        let corpus = build_perf_seed_corpus();
        let unannotated = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_none())
            .count();
        assert_eq!(unannotated, 9, "expected 9 unannotated rules");
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
        assert_eq!(annotated, 2);

        // Simulate a pre-field snapshot: strip `context_notes` from JSON.
        // (A real old snapshot simply omits the key; serde(default) → None.)
        let old_json = json.replace("\"context_notes\":", "\"__removed_context_notes\":");
        let old_back: Vec<PerfRule> = serde_json::from_str(&old_json).unwrap();
        let old_annotated = old_back.iter().filter(|e| e.context_notes.is_some()).count();
        assert_eq!(old_annotated, 0, "missing field should default to None");
    }

    /// Integration: the drafter propagates `context_notes` from `PerfRule`
    /// through `DraftedFix` for the 2 ParetoDominated rules, and `None` for
    /// the 6 unconditional wins. This is the Plan 001 post-promotion wiring
    /// (2026-08-13) — the field existed on `PerfRule` but was invisible to
    /// the domain-agnostic drafter until `HealEntry::context_notes()` landed.
    #[test]
    fn drafter_propagates_context_notes_from_perf_corpus() {
        use crate::domain::HealEntry;

        let corpus = build_perf_seed_corpus();

        // Filter to the 2 annotated rules.
        let annotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_some()).collect();
        assert_eq!(annotated.len(), 2, "expected 2 annotated rules");

        // The HealEntry trait override must agree with the struct field.
        for rule in &annotated {
            assert_eq!(
                HealEntry::context_notes(*rule),
                rule.context_notes.as_deref(),
                "HealEntry::context_notes() must match the PerfRule field for {}",
                rule.rule_id
            );
            assert!(
                rule.context_notes.as_ref().unwrap().contains("ParetoDominated"),
                "annotation should mention ParetoDominated for {}",
                rule.rule_id
            );
        }

        // The 9 unannotated rules must return None from the trait too.
        let unannotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_none()).collect();
        for rule in &unannotated {
            assert!(HealEntry::context_notes(*rule).is_none(), "{} should have no context_notes", rule.rule_id);
        }
    }
}
