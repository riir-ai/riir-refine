//! Entries slice 02 of 06 — rules 27–52 of 126 — moved VERBATIM from the
//! former corpus.rs monolith (2026-09-10 split, scripted by
//! `scripts/split_rust_perf_corpus.py`). Append new batches to the
//! LAST slice or open a new one past ~1800 lines.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
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
        // B86 RIDER (Cloudflare Big Pineapple, Research 100): the rule
        // GENERALIZES to Box<[T]> for ANY frozen (write-once) T — every
        // Vec<T> field on a stored-at-scale record drops the capacity word
        // AND the growth slack. Their cache replaced 8 frozen fields per
        // entry (String/Vec → Box<str>/Box<[T]>) = 64 B/entry, >15 TB freed
        // fleet-wide. The str half is this rule; the slice generalization
        // is the rider — same mechanism (drop the capacity word), any T.
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
        // B86 RIDER (Cloudflare Big Pineapple, Research 100 — their change
        // 4): the HONEST COSTS of boxing the large variant, recorded so the
        // rewrite is not sold as free: (1) allocator size-class rounding —
        // the boxed payload rounds UP to the allocator's bin (their TXT 32 B
        // fit its bin; MX 40 B rounded to 48 — some boxed variants pay more
        // than their inline size); (2) locality scatter — one contiguous
        // allocation becomes many, and the common-case records now chase a
        // pointer on access; (3) the RARE variant pays MORE (their NAPTR,
        // the 136 B variant, now costs the Box header on top). The win is
        // the SKEWED-FREQUENCY regime: ~80% small variants (their A/AAAA
        // traffic) vs a large tail — boxing shrinks the common case exactly
        // because the large case is rare.
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
            context_notes: Some("REGIME BOUNDARY (Batch 153 annotation, async-perf pool): Ryhl's rule of thumb — 'no more than 10 to 100 microseconds between each .await' ('Async: What is blocking?', 2020-12-21) — is the canonical quantitative boundary for when an await-less stretch needs a yield; the fixed 100-iteration interval here is its coarse structural form. RUNTIME PRECEDENT (the same pool, source 4): tokio's own resources self-yield on a per-task budget of 128 ops per tick (since 0.2.14 — tokio.rs/blog/2020-04-preemption; source-reported 'almost 3x' tail-latency reduction, 'in some cases'); for fairness tighter than the runtime's 128 or for NON-tokio resources (no built-in budget) prefer the adaptive counter — see adaptive-yield-consecutive-ready. Fairness/p99 metric class — NotBenchable (throughput-neutral). SCHEDULER-TWEAK REVERT DATUM (Batch 154 annotation, CHANGELOG refresh): tokio shipped 'steal tasks from the LIFO slot' in 1.51.0 (#7431) and REVERTED it in 1.52.2 (#8100) 'due to its performance impact' — even runtime-authored scheduler tweaks ship-then-revert when measured in the wild; the manual-yield decision this rule encodes deserves the same measure-don't-assume posture. COOP API (1.47.0 #7405): the 128-op budget system is now PUBLIC — coop::cooperative(fut) runs a leaf op with a FRESH budget (wrapping non-cooperative leaf work the runtime's resources cannot budget for you) and poll_proceed is its poll-shaped twin. CHANGELOG 1.47/1.51/1.52 read live 2026-09-18.".to_string()),
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
            context_notes: Some("MEASURED ParetoDominant 3.73-6.91x (Bench 013, the rule own gate). REGIME (sorted-read frequency — the two-row decision table completed by Batch 75's `group-keys-borrow-allocate-on-first-sight`, the mirror rule): 0 sorted reads of the grouped output → plain Vec/HashMap, no ordering machinery; ONE final sort after a single grouping pass → build unordered + sort once (the mirror rule — R·L owned-key clones become S·L, and R ordered O(log S) inserts become one S·log(S) sort); MANY repeated sorted reads over a long-lived map → THIS rule (maintain the BTreeMap, iterate for free). The common failure is paying BTreeMap ordered-insert cost during a build whose output is read sorted once or never.".to_string()),
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
        // ── concurrency: adjacent per-thread counter slots → align to cache line ──
        // Source: hotpath.rs profiler blog (Batch 65, Research 081 — the
        // profiler's own GlobalAlloc hook: `static STATS: [ThreadAllocStats;
        // MAX_THREADS]`, 24 B each, indexed by thread ordinal). Two threads
        // bumping atomics in slots that share a coherence line pay a line
        // ping-pong per RMW; padding each slot to the line granularity makes
        // every slot private. Source (x86, align(64)): 2t 70% / 4t 72% /
        // 8t 86% / 12t 85%; single-thread 0% (honest — no contention, no cost).
        // **Pad to the ARCH's line size, not 64 by reflex (measured, M3 Max,
        // 2026-08-22): Apple Silicon lines are 128 B (`hw.cachelinesize`), so
        // `#[repr(align(64))]` leaves even-index slot pairs sharing [0,128)
        // lines — it recovered only 1.16×/0.98×/1.60×/2.44× at 2/4/8/12t
        // where align(128) delivered 3.27×/3.88×/10.75×/5.93×.**
        // `crossbeam_utils::CachePadded` picks per-target. Memory cost: 24 B
        // → 64/128 B per slot (irrelevant at MAX_THREADS scale, load-bearing
        // at millions-of-slots scale).
        PerfRule {
            rule_id: "align-64-false-sharing".to_string(),
            name: "adjacent per-thread atomic counter slots → #[repr(align)] to cache-line granularity".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "static STATS: [ThreadAllocStats; MAX_THREADS]; // 24 B each — several per cache line\n\nfn record_alloc(thread_ord: usize, bytes: u64) {\n    let s = &STATS[thread_ord]; // adjacent ordinals share a coherence line\n    s.allocs.fetch_add(1, Ordering::Relaxed); // line ping-pong under contention\n    s.bytes.fetch_add(bytes, Ordering::Relaxed);\n}".to_string(),
            fixed_code: "#[repr(align(128))] // pad to the ARCH line size (64 on x86, 128 on Apple Silicon) — or crossbeam_utils::CachePadded\nstruct ThreadAllocStats {\n    allocs: AtomicU64,\n    bytes: AtomicU64,\n    frees: AtomicU64,\n}\n\nstatic STATS: [ThreadAllocStats; MAX_THREADS]; // every slot on its own line\n\nfn record_alloc(thread_ord: usize, bytes: u64) {\n    let s = &STATS[thread_ord]; // private line — no ping-pong\n    s.allocs.fetch_add(1, Ordering::Relaxed);\n    s.bytes.fetch_add(bytes, Ordering::Relaxed);\n}".to_string(),
            context_notes: Some(
                "Measured ParetoDominant 4.7-9.4x at 2-12 threads (M3 Max, align(128) fixed arm, load ~3.2, 2026-08-22; single-thread = tie — no contention, no cost, honest). ARCH GRANULARITY CAVEAT (the rule id carries the source's x86 form): Apple Silicon cache lines are 128 B (`hw.cachelinesize`), so `#[repr(align(64))]` leaves even-index slot pairs sharing [0,128) lines — measured it recovers only 1.16-2.44x of the 3.3-10.8x full-separation win. Pad to the ARCH's coherence granularity (align(64) x86 / align(128) Apple Silicon) or use `crossbeam_utils::CachePadded` which picks per-target. Memory trade: 24 B -> 64/128 B per slot (irrelevant at MAX_THREADS scale, load-bearing at millions-of-slots scale). Detection shape: a static/owned array of small structs with Atomic* fields indexed by thread ordinal."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(align_64_false_sharing_bench_spec()),
        },
        // ── concurrency: per-item channel sends → per-producer batch flush ────
        // Source: hotpath.rs profiler blog (Batch 65, Research 081 — the
        // measurement channel: per-measurement `send` at µs intervals to a
        // receiver parked in crossbeam `select!`). `select!` bypasses
        // crossbeam's direct-subscribe spin-wait path, so the receiver PARKS
        // between messages and every send pays a Thread::park/unpark pair
        // (~22% of samples in `Thread::unpark`). Batching per producer
        // thread (local Vec drained on flush) amortizes the wakeup: source
        // 500ns → 80ns per measurement (6.25×). Adjacent-but-distinct from
        // `bounded-channel-backpressure` (queue-fullness POLICY vs
        // per-message WAKEUPS). The direct-subscribe receiver already spins
        // — the storm is select!-specific (upstream PR submitted by the
        // source author).
        PerfRule {
            rule_id: "channel-send-batching-unpark".to_string(),
            name: "per-item MPSC sends to a parked select! receiver → per-producer batch flush. RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- commit 1b89a43 #27830): the quantizer feeds the worker pool in ROW SLABS so the quantize workers never idle -- the producer-side batching half of this rule at model-load scale (chunked enqueue so consumers stay parked-unparked in batches, not per-item).".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "fn record_measurement(tx: &crossbeam_channel::Sender<Sample>, s: Sample) {\n    tx.send(s).unwrap(); // 1 park/unpark pair per message when the\n                         // receiver sits in crossbeam select! (bypasses\n                         // the spin path) — ~22% of samples in Thread::unpark\n}".to_string(),
            fixed_code: "fn record_measurement(buf: &mut Vec<Sample>, tx: &crossbeam_channel::Sender<Vec<Sample>>, s: Sample) {\n    buf.push(s); // batch per producer thread…\n    if buf.len() >= FLUSH_EVERY {\n        // replace keeps capacity for the next batch (take-then-regrow pays\n        // a ~7-alloc growth chain per batch — measured worse on the alloc\n        // axis than the buggy arm)\n        let batch = std::mem::replace(buf, Vec::with_capacity(FLUSH_EVERY));\n        tx.send(batch).unwrap(); // …ONE wakeup per FLUSH_EVERY messages\n    }\n}".to_string(),
            context_notes: Some(
                "Measured ParetoDominant 1.28-1.29x@10K / 1.16x@100K (M3 Max, load ~3.2-3.8, 2026-08-22; allocs 3232->1624 @100K, alloc-det stable). REGIME CAVEAT — the modest margin is honest: a 1P/1C pipeline overlaps the receiver's park/unpark latency with the producer's next-message work, hiding it off the critical path; the source's 6.25x (500ns->80ns per measurement) needs the receiver-side park pair ON the critical path (heavier consumer work per message, or the single-measurement µs-cadence API shape the profiler used). The mechanism is crossbeam-select!-specific — a direct `recv()` receiver spins and never storms (upstream PR filed by the source author). Batching also wins the alloc axis (fewer channel blocks + one Vec per 64 vs one block per ~32). Use `mem::replace` for the flush, NOT `mem::take` (take-then-regrow pays a ~7-alloc Vec growth chain per batch)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(channel_send_batching_unpark_bench_spec()),
        },
    ]
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

/// Bench spec for `mem-take`: clone+clear vs `mem::take` for Vec<String> buffer.
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
/// The buggy path allocates a fresh `Vec::new()` inside the function body
/// every call. The fixed path reuses a caller-passed buffer via .`clear()`.
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

/// Bench spec for `buffered-io`: `File::read` byte-by-byte vs `BufReader`.
///
/// Both paths read the same temp file. The buggy path issues a syscall per
/// byte; the fixed path batches reads through `BufReader`'s 8KB buffer.
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

/// Bench spec for `log-write-direct`: `push_str(&format`!(...)) vs write! direct.
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

/// Bench spec for `sort-by-key`: `sort_by` with repeated closure calls vs
/// `sort_by_key` with single key extraction.
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

/// Bench spec for `bufwriter-batched`: per-record `File::write` (syscall each)
/// vs `BufWriter` batched.
///
/// Uses `tempfile` to get a real filesystem path. Each record is 64 bytes;
/// with 8KB `BufWriter` buffering, ~128 records per flush. The buggy path does
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
/// `Vec::remove(0`) does a memmove of all remaining elements (O(n) per call,
/// O(n²) total). `VecDeque::pop_front` advances the head index (O(1) per call,
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
/// `HashMap` + `sort_unstable` per query is O(n log n) each. `BTreeMap`'s `keys()`
/// iterator is already sorted — O(n) per query. Insert in pseudo-random order
/// so `HashMap`'s iteration order is NOT accidentally sorted.
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

/// Bench spec for `regex-hoist-lazylock`: `Regex::new` per call vs static `LazyLock`.
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

/// Bench spec for `align-64-false-sharing` (Batch 65, Research 081 —
/// the hotpath.rs profiler distill): adjacent 24 B per-thread atomic-counter
/// slots vs `#[repr(align(128))]` slots, N scoped threads bumping their own
/// slot.
///
/// sizes = THREAD COUNTS (the `n` threaded through `make_input` is the
/// worker count, not an array length — the contention axis IS the size
/// axis; 1t deliberately excluded: no contention, honest Tie, and the
/// source measured 0% there). `ITERS=10_000` iterations × 3 RMW per thread
/// keeps the contended arm ≈1 ms/call (the calibration target) at 8t.
/// The fixed arm pads to 128 B — the ARCH line size on this M3 Max
/// (`hw.cachelinesize`; align(64) leaves even-index slot pairs sharing
/// [0,128) lines on Apple Silicon — measured 1.16-2.44× vs the full
/// 3.27-10.75× separation, the annotation's load-bearing caveat).
/// G1: per-call checksum is deterministic (`fetch_add` totals cannot drift
/// with layout) — asserted in-run against the closed form.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn align_64_false_sharing_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![2, 4, 8, 12],
        harness: r#"
mod buggy {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub struct Stats {
        pub allocs: AtomicU64,
        pub bytes: AtomicU64,
        pub frees: AtomicU64,
    }

    pub const SLOTS: usize = 12; // several 24 B slots per coherence line

    pub fn make_slots() -> Vec<Stats> {
        (0..SLOTS)
            .map(|_| Stats {
                allocs: AtomicU64::new(0),
                bytes: AtomicU64::new(0),
                frees: AtomicU64::new(0),
            })
            .collect()
    }

    pub fn run(slots: &[Stats], n_threads: usize, iters: u64) -> u64 {
        std::thread::scope(|s| {
            for t in 0..n_threads {
                s.spawn(move || {
                    let slot = &slots[t];
                    for i in 0..iters {
                        slot.allocs.fetch_add(1, Ordering::Relaxed);
                        slot.bytes.fetch_add(i, Ordering::Relaxed);
                        slot.frees.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        slots
            .iter()
            .map(|x| {
                x.allocs.load(Ordering::Relaxed)
                    + x.bytes.load(Ordering::Relaxed)
                    + x.frees.load(Ordering::Relaxed)
            })
            .sum()
    }
}

mod fixed {
    use std::sync::atomic::{AtomicU64, Ordering};

    #[repr(align(128))] // the ARCH line size (Apple Silicon; 64 on x86)
    pub struct Stats {
        pub allocs: AtomicU64,
        pub bytes: AtomicU64,
        pub frees: AtomicU64,
    }

    pub const SLOTS: usize = 12;

    pub fn make_slots() -> Vec<Stats> {
        (0..SLOTS)
            .map(|_| Stats {
                allocs: AtomicU64::new(0),
                bytes: AtomicU64::new(0),
                frees: AtomicU64::new(0),
            })
            .collect()
    }

    pub fn run(slots: &[Stats], n_threads: usize, iters: u64) -> u64 {
        std::thread::scope(|s| {
            for t in 0..n_threads {
                s.spawn(move || {
                    let slot = &slots[t]; // private line — no ping-pong
                    for i in 0..iters {
                        slot.allocs.fetch_add(1, Ordering::Relaxed);
                        slot.bytes.fetch_add(i, Ordering::Relaxed);
                        slot.frees.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        slots
            .iter()
            .map(|x| {
                x.allocs.load(Ordering::Relaxed)
                    + x.bytes.load(Ordering::Relaxed)
                    + x.frees.load(Ordering::Relaxed)
            })
            .sum()
    }
}

const ITERS: u64 = 10_000; // 3 RMW per iteration per thread

pub struct Input {
    n_threads: usize,
}

pub fn make_input(n: usize) -> Input {
    // n = THREAD COUNT (the contention axis is the size axis).
    Input { n_threads: n }
}

fn expected(n_threads: usize) -> u64 {
    // Deterministic closed form: active slots each get iters allocs
    // + iters frees + sum(0..iters) bytes. Layout cannot change totals
    // (fetch_add is atomic) — the G1 identity.
    n_threads as u64 * (2 * ITERS + ITERS * (ITERS - 1) / 2)
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let slots = buggy::make_slots();
    let cs = buggy::run(&slots, input.n_threads, ITERS);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, expected(input.n_threads));
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let slots = fixed::make_slots();
    let cs = fixed::run(&slots, input.n_threads, ITERS);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, expected(input.n_threads));
}"#
        .to_string(),
    }
}

/// Bench spec for `channel-send-batching-unpark` (Batch 65, Research 081):
/// per-item unbounded-channel sends to a receiver parked in crossbeam
/// `select!` vs per-64-item batch sends (one wakeup per batch).
///
/// The storm regime is load-bearing: the receiver must find the channel
/// EMPTY when it re-polls, so the producer pays ~32 hash rounds per item
/// (µs-scale intervals — the source's measurement cadence) while the
/// receiver only counts. `select!` bypasses crossbeam's direct-subscribe
/// spin path, so the empty poll parks the receiver — one park/unpark pair
/// per message in the buggy arm, one per `FLUSH_EVERY` in the fixed arm.
/// The ctrl arm exists to force the two-channel select! shape (held
/// connected-but-silent until the receiver joins; the data channel's
/// disconnect is the shutdown signal). G1: receiver item count == total
/// in both arms (asserted in-run).
#[cfg(feature = "rust_perf_bench")]
fn channel_send_batching_unpark_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000],
        harness: r#"// [dep: crossbeam-channel = "0.5"]
const FLUSH_EVERY: usize = 64;

#[inline]
fn measurement_work(i: u64) -> u64 {
    // µs-scale per-measurement cost (the profiler's sampling work) — keeps
    // the producer interval above the receiver's processing time so the
    // channel is EMPTY when the receiver re-polls (the park regime).
    let mut h = i.wrapping_mul(0x9E37_79B9_97F4_A7C1);
    for _ in 0..32 {
        h = h.rotate_left(7) ^ h.wrapping_mul(0x2545_F491_4F6C_DD1D);
    }
    h
}

mod buggy {
    pub fn produce(tx: &crossbeam_channel::Sender<u64>, total: u64) {
        for i in 0..total {
            std::hint::black_box(super::measurement_work(i));
            tx.send(i).unwrap(); // ONE park/unpark pair per message
        }
    }
}

mod fixed {
    pub fn produce(tx: &crossbeam_channel::Sender<Vec<u64>>, total: u64) {
        let mut buf: Vec<u64> = Vec::with_capacity(super::FLUSH_EVERY);
        for i in 0..total {
            std::hint::black_box(super::measurement_work(i));
            buf.push(i);
            if buf.len() >= super::FLUSH_EVERY {
                // replace keeps `buf`'s capacity for the next batch — the
                // take-then-regrow variant pays a ~7-alloc growth chain per
                // batch (measured 795 vs 329 allocs @10K — worse than the
                // buggy arm on the alloc axis).
                let batch =
                    std::mem::replace(&mut buf, Vec::with_capacity(super::FLUSH_EVERY));
                tx.send(batch).unwrap(); // one wakeup per 64
            }
        }
        if !buf.is_empty() {
            tx.send(buf).unwrap();
        }
    }
}

pub struct Input {
    total: u64,
}

pub fn make_input(n: usize) -> Input {
    Input { total: n as u64 }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let (tx, rx) = crossbeam_channel::unbounded::<u64>();
    let (_ctx, crx) = crossbeam_channel::unbounded::<()>(); // silent 2nd select arm
    let receiver = std::thread::spawn(move || {
        let mut count: u64 = 0;
        loop {
            crossbeam_channel::select! {
                recv(rx) -> msg => match msg {
                    Ok(_) => count += 1,
                    Err(_) => break, // producer dropped — shutdown
                },
                recv(crx) -> _ => break,
            }
        }
        count
    });
    buggy::produce(&tx, input.total);
    drop(tx);
    let count = receiver.join().unwrap();
    std::hint::black_box(count);
    debug_assert_eq!(count, input.total);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let (tx, rx) = crossbeam_channel::unbounded::<Vec<u64>>();
    let (_ctx, crx) = crossbeam_channel::unbounded::<()>(); // silent 2nd select arm
    let receiver = std::thread::spawn(move || {
        let mut count: u64 = 0;
        loop {
            crossbeam_channel::select! {
                recv(rx) -> msg => match msg {
                    Ok(batch) => count += batch.len() as u64,
                    Err(_) => break,
                },
                recv(crx) -> _ => break,
            }
        }
        count
    });
    fixed::produce(&tx, input.total);
    drop(tx);
    let count = receiver.join().unwrap();
    std::hint::black_box(count);
    debug_assert_eq!(count, input.total);
}"#
        .to_string(),
    }
}
