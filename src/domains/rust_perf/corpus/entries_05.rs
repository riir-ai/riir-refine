//! Entries slice 05 of 06 — rules 88–115 of 126 — moved VERBATIM from the
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
        // ── complexity: eagerly-computed diagnostic statistic scans the whole ──
// collection per operation → remove from the hot path / maintain ─────
// incrementally ──────────────────────────────────────────────────────
// Source: Meilisearch "Patching LMDB" (2026-03-18), STOP COMPUTING
// USELESS STATS: the mean-degree sanity statistic — "added to Hannoy
// very early on as a sanity check" — was computed during item-ID
// retrieval, full-scanning the vector store on every insert batch:
// "Removing this reduces the time from 559s to about 2–3s and brings
// the complexity down to O(n), where n is the number of newly
// inserted or removed embeddings." The decision content: a diagnostic
// eagerly computed in a hot path is a hidden O(collection) tax that
// GROWS with total size — the hot path pays for the whole past, not
// the new work. Fix shapes: remove it (tests/CI now guard the
// invariant), gate behind debug/opt-in, or maintain the aggregate
// incrementally. Distinct from `sort-key-cache-precompute-out-of-lock`
// (comparator recompute under a lock, per-comparison; this is
// per-operation whole-collection statistics) and from
// `filter-count-to-any` (predicate full scan vs short-circuit; this is
// an aggregate whose RESULT isn't even consumed in production).
// Suggestion-only (type-blind: a parse-only proposer cannot prove the
// statistic is unconsumed or that the loop is diagnostic — the
// soa-hot-field class).
PerfRule {
    rule_id: "debug-stat-full-scan-out-of-hot-path".to_string(),
    name: "per-insert aggregate statistic recomputed over the ENTIRE collection (mean/count full scan) → out of the hot path (maintained incrementally / debug-gated)".to_string(),
    category: "complexity".to_string(),
    buggy_code: "use std::collections::HashMap;\nstruct Item {\n    links: Vec<u32>,\n}\nstruct Store {\n    items: Vec<Item>,\n}\nimpl Store {\n    // a sanity statistic \"added very early on\" — full-scans EVERY item\n    // on EVERY insert batch. O(total) per op: the hot path pays for the\n    // whole past, not the new work; grows without bound as the store does.\n    fn mean_degree(&self) -> f64 {\n        let total: usize = self.items.iter().map(|i| i.links.len()).sum();\n        self.items.len() as f64 / total.max(1) as f64\n    }\n    fn insert_batch(&mut self, new_items: Vec<Item>) {\n        for it in new_items {\n            self.items.push(it);\n        }\n        // diagnostic eagerly computed in the hot path — the only consumer\n        // was the developer's peace of mind (CI now guards the invariant)\n        let _mean = self.mean_degree();\n    }\n}".to_string(),
    fixed_code: "struct Item {\n    links: Vec<u32>,\n}\nstruct Store {\n    items: Vec<Item>,\n    // maintained aggregate: link_count updates by the DELTA each insert\n    // — the statistic stays available at O(1) for whoever needs it, and\n    // the hot path never pays for the whole past\n    link_count: usize,\n}\nimpl Store {\n    fn mean_degree(&self) -> f64 {\n        self.items.len() as f64 / self.link_count.max(1) as f64\n    }\n    fn insert_batch(&mut self, new_items: Vec<Item>) {\n        for it in new_items {\n            self.link_count += it.links.len();\n            self.items.push(it);\n        }\n        // no full scan — the invariant moved to CI/tests (the source's\n        // fix: remove it; the maintained form when the stat IS wanted)\n    }\n}".to_string(),
    context_notes: Some("MEASURED ParetoDominant 3/0/0 — 168.91x/1929.27x/5851.00x at N=10K/100K/300K store size (B=100 inserted per op; M3 Max release, rustc 1.97, 2026-08-27, load ~2.4 of 16; verdict-stable ×2: 179.55x/1883.75x/6293.55x on the re-run; allocs 0/0 equal by design, alloc-det stable). REGIME (store-size to batch-size ratio N/B): the buggy arm pays O(N) per insert batch — the win GROWS linearly with total store size (the source: 559s → 2–3s on a 19M-item store, ~200x on the step). At N ≈ B (tiny store, huge batches) the scan is one pass over what the insert touches anyway — expected Tie; the rule lives where the collection ACCUMULATES (insert-heavy stores where per-op diagnostics re-walk the past). The decision content: diagnostics eagerly computed in hot paths are a hidden O(collection) tax — the source's fix was REMOVAL (tests/CI now guard the invariant); the maintained-aggregate form is the fix when the statistic is actually consumed downstream. Distinct from `sort-key-cache-precompute-out-of-lock` (per-comparison recompute under lock) — this is per-operation whole-collection statistics. Suggestion-only (a parse-only proposer cannot prove the statistic unconsumed). Source: Meilisearch blog 2026-03-18 §Stop computing useless stats.".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: Some(debug_stat_full_scan_out_of_hot_path_bench_spec()),
},
// ── complexity: measurement honesty — a timed harness whose work result ──
// is dead measures ZERO under fat LTO (rustc 1.98.1 measured class) ───────
// Source: post-mining intake P10 (2026-09-04), found by the katgpt-rs Issue
// 723 Class A2 fixes (`b4cef008`): a direct-call probe in ONE binary showed
// the used result at 16.6 µs vs `let _ = f()` ≈ 0 — the fold eliminates
// inlined-callee work when the caller discards the result, EVEN THROUGH
// `black_box` INSIDE the callee (the barrier protects the callee's own
// return value from intra-callee folding; it cannot keep the CALLER's dead
// result alive — under fat LTO the whole chain is one CFG and the work's
// only observable use is gone). Three harness patterns proven in-tree:
// (1) the timed work must CHAIN its state (each iteration consumes the
// previous accumulator — the xorshift-PRNG shape, unprunable by
// construction): parallel sinks cancel in batches three ways — XOR dies
// on GF(2) linearity over 0..2^k, wrapping_add collapses to the
// seed-independent 2^64 − n/2 constant over balanced power-of-two
// batches, and constant work folds through anything; (2) the
// black_box lives at the CONSUMER (in the timed loop, not only inside
// the work fn — a caller-dead result dies under fat LTO); (3) for
// bar-adjacent ratios, interleave (baseline, candidate) chunks and take a
// median-of-ratios — two sequential 2M-iteration arms read +5.2%/+21.7%
// drift thirty seconds apart while the paired median read −0.0%/+0.6%
// twice. The LOUD-BASELINE corollary is the cheap detector every timing
// gate should carry: assert the baseline elapsed is NONZERO before forming
// a ratio. The failure directions differ by comparison form — a `>=`
// PASS bar with a NaN ratio (0 ns baseline → 0/0) is always-false = a loud
// fail; a `<=` budget bar with NaN is always-false TOO, so a budget gate
// reads as PASSING forever while measuring nothing — the silent direction.
// Both arms must do equal non-pruner work (one shared timed fn, the
// candidate's delta passed as data) so the ratio isolates the delta rather
// than arming the fold on one side only. Distinct from
// `debug-stat-full-scan-out-of-hot-path` (an UNCONSUMED PRODUCTION
// statistic dragged through a hot path; this is the HARNESS side — the
// production code is fine, the measurement of it is empty) and from
// `encoded-layout-borrowed-view`'s bench note ("an unread &str costs zero
// and the ratio inflates" — the same physics from the bench-consumption
// side; this rule is the general harness contract). Suggestion-only: the
// detector is a dataflow question (is the timed loop's result observable
// anywhere?) that a parse-only proposer cannot answer; the loud
// zero-baseline assert is the machine-checkable surrogate. No Pareto bench
// is possible — the rule's entire content is that the baseline is zero, so
// there is nothing to ratio (NotBenchable, the measurement-honesty axis).
PerfRule {
    rule_id: "dead-result-timing-loop-folded-under-lto".to_string(),
    name: "timed loop discards its work result (`let _ = f()`) → fat-LTO fold eliminates the inlined callee and the gate times ZERO — xorshift data-dependent work + `black_box(sink)` at the consumer + a loud nonzero-baseline assert".to_string(),
    category: "complexity".to_string(),
    buggy_code: "use std::hint::black_box;\nuse std::time::Instant;\n\nfn work(seed: u64) -> u64 {\n    // some non-trivial kernel the gate pretends to measure\n    let mut x = seed;\n    for _ in 0..64 {\n        x ^= x << 13;\n        x ^= x >> 7;\n        x ^= x << 17;\n    }\n    x\n}\n\nfn overhead_ratio(iters: u32) -> f64 {\n    let start = Instant::now();\n    for i in 0..iters {\n        // the result is DISCARDED: under rustc 1.98.1 + fat LTO the\n        // inlined body folds away (the callee-internal black_box cannot\n        // keep a caller-dead result alive) and this times ~0 ns\n        let _ = work(black_box(i as u64));\n    }\n    let baseline = start.elapsed();\n    // NaN when baseline is 0: with a `<=` budget bar this is an\n    // always-false comparison = a SILENT vacuous pass, forever\n    baseline.as_nanos() as f64 / 1_000.0\n}".to_string(),
    fixed_code: "use std::hint::black_box;\nuse std::time::Instant;\n\nfn work(seed: u64) -> u64 {\n    let mut x = seed;\n    for _ in 0..64 {\n        x ^= x << 13;\n        x ^= x >> 7;\n        x ^= x << 17;\n    }\n    x\n}\n\nfn overhead_ratio(iters: u32) -> f64 {\n    let start = Instant::now();\n    // CHAIN the state: each iteration consumes the previous accumulator,
    // so every output depends on the whole sequential history — the
    // xorshift-PRNG shape, unprunable by construction. PARALLEL sinks
    // cancel in batches and are the trap this rule exists for: an XOR
    // sink over a GF(2)-linear work fn (xorshift, CRC, most bit math)
    // is exactly 0 over 0..2^k (linearity), and even a wrapping_add
    // sink collapses to the seed-independent constant 2^64 − n/2
    // over a power-of-two batch when the work’s output bits are
    // balanced — both measured, not theorized. The per-iteration
    // black_box on the index keeps the loop input live; the final
    // black_box consumes the chain.
    let mut state = black_box(0x9E37_79B9_7F4A_7C15u64);
    for i in 0..iters {
        state = work(state ^ black_box(i as u64));
    }
    black_box(state);\n    let measured = start.elapsed();\n    // the loud-baseline assert: a gate that measured 0 ns is a gate that\n    // measured NOTHING — fail before any ratio can go NaN\n    assert!(measured.as_nanos() > 0, \"timing harness folded: dead-result work measured zero ns — chain the timed state — parallel sinks cancel in batches (XOR: GF(2) linearity; balanced-add: the 2^64 − n/2 constant over 2^k)\");\n    measured.as_nanos() as f64 / 1_000.0\n}".to_string(),
    context_notes: Some("NotBenchable — measurement-honesty axis (Batch 109): the rule's content is that the baseline folds to zero, so there is no nonzero quantity to ratio; the evidence is the katgpt-rs direct-call probe (used result 16.6 µs vs `let _ = f()` ≈ 0, one binary, rustc 1.98.1 + fat LTO) and the rv gate that measured 0 ns → NaN% for its entire release life (found by Issue 723 Class A2, `b4cef008`). The load-bearing subtleties: (1) `black_box` INSIDE the callee does NOT save the work when the CALLER discards the result — under fat LTO the barrier only pins the callee's own return value, and the whole chain is one CFG where the value has no observer; (2) PARALLEL sinks cancel in batches — THREE measured modes: XOR over a GF(2)-linear work fn (xorshift, CRC, most bit math) is exactly 0 over 0..2^k (linearity; the gate caught the rule’s OWN first draft dying on this), wrapping_add over a power-of-two batch collapses to the seed-independent constant 2^64 − n/2 when the work’s output bits are balanced (each bit is 1 in exactly half the terms regardless of seed — measured: both arms read 18446744073709549568), and a constant-work fn folds through any sink at compile time — the robust fix is CHAINED STATE (each iteration consumes the previous accumulator, the xorshift-PRNG shape, unprunable by construction); (3) the NaN direction contract: NaN compares false against EVERYTHING, so a `>=` PASS bar fails loudly on a zero baseline while a `<=` budget bar passes silently forever — the assert must fire BEFORE the ratio; (4) for ratio gates, equal non-pruner work in both arms (one shared timed fn) + interleaved median-of-ratios (sequential arms drifted +5.2%/+21.7% thirty seconds apart; the paired median read −0.0%/+0.6% twice). Distinct from `debug-stat-full-scan-out-of-hot-path` (unconsumed PRODUCTION statistic; this is the HARNESS side) and from `encoded-layout-borrowed-view`'s bench-consumption note (the same physics from the bench side). Suggestion-only (a parse-only proposer cannot prove the timed result is unobservable; the zero-baseline assert is the machine-checkable surrogate). Source: post-mining intake P10, katgpt-rs Issue 723 Class A2 evidence (`tests/bench_gdsd_modelless.rs` + `tests/rv_gated_routing.rs` sink docs).".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: None,
},
// ── caching: materialize a lookup index over ALL entries (full scan) ───
// to serve a SUBSET of random accesses → lazy/direct access of only ───
// the needed entries ───────────────────────────────────────────────────
// Source: same article, FULL SCANS ARE HARMFUL + WHAT'S THE NEW TRICK:
// the ImmutableItems/ImmutableLinks structures did "a full scan of all
// the entries we will need to access", collecting pointers into a
// HashMap — then "we were polluting the cache with useless,
// unnecessary disk pages just by iterating over all of them to fetch
// pointers." Replaced by FrozenReader: "read uncommitted graph nodes
// and connections lazily, on demand" so "only the necessary pages are
// loaded into the cache — not all available pages in LMDB. We avoid
// evicting important pages." The decision content: building an
// everything-index to serve a subset of accesses pays the scan TWICE —
// once in time (O(all) build) and once in cache (the scan's own
// traffic evicts the hot pages the subset will need). When the
// underlying store offers a direct access path, index only on demand.
// Distinct from `collect-avoidance` (intermediate-Vec staging, alloc
// axis; this is working-set residency) and from
// `encoded-layout-borrowed-view` (representation re-expansion inside
// an accessor; this is eager INDEX MATERIALIZATION before a subset
// read). Suggestion-only (architectural — a parse-only proposer
// cannot see the direct-access path exists).
PerfRule {
    rule_id: "eager-pointer-collect-lazy-fetch".to_string(),
    name: "full-scan materialize a HashMap of ALL entry pointers to serve a SUBSET of random reads → direct/lazy access of only the needed entries. RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- --lazy-mode, commits fac889f #27794 + bebc935 #27969 + 2578138 #27837 + b19cbe9 #27869; APP-LAYER at this pin: common/arg.cpp:2737, include/llama.h:217-220, src/llama-model-loader.cpp:1082-1088): an entire user-facing feature built as this rule's pattern at model-load scale -- lazy mode defers tensor materialization to first use (llama_lazy_mode ON/AUTO/OFF) instead of collecting every weight upfront.".to_string(),
    category: "caching".to_string(),
    buggy_code: "use std::collections::HashMap;\nstruct Node {\n    payload: [u64; 4],\n}\nstruct Store {\n    nodes: Vec<Node>, // direct-indexable: ids are dense 0..len\n}\nimpl Store {\n    // full scan of EVERY entry, collecting pointers into a HashMap —\n    // \"polluting the cache with useless, unnecessary disk pages just by\n    // iterating over all of them to fetch pointers\"\n    fn freeze_all(&self) -> HashMap<usize, &Node> {\n        let mut m = HashMap::with_capacity(self.nodes.len());\n        for (i, n) in self.nodes.iter().enumerate() {\n            m.insert(i, n);\n        }\n        m\n    }\n    fn touch(&self, ids: &[usize]) -> u64 {\n        let index = self.freeze_all(); // O(all) build to serve a subset\n        let mut acc = 0u64;\n        for &id in ids {\n            if let Some(n) = index.get(&id) {\n                acc = acc.wrapping_add(n.payload[0]);\n            }\n        }\n        acc\n    }\n}".to_string(),
    fixed_code: "struct Node {\n    payload: [u64; 4],\n}\nstruct Store {\n    nodes: Vec<Node>,\n}\nimpl Store {\n    // lazy/on-demand: no index build, no scan — \"only the necessary pages\n    // are loaded into the cache\"; the store IS the direct-access path\n    fn touch(&self, ids: &[usize]) -> u64 {\n        let mut acc = 0u64;\n        for &id in ids {\n            if let Some(n) = self.nodes.get(id) {\n                acc = acc.wrapping_add(n.payload[0]);\n            }\n        }\n        acc\n    }\n}".to_string(),
    context_notes: Some("MEASURED ParetoDominant 3/0/0 — 1903.70x/5414.62x/21353.90x at N=100K/300K/1M store size (K=1,000 touched ids; M3 Max release, rustc 1.97, 2026-08-27, load ~2.4 of 16; verdict-stable ×2: 2217.57x/5962.48x/25736.41x on the re-run; allocs 1->0, bytes 35.7 MB -> 0 at N=1M — the HashMap build eliminated; alloc-det stable). REGIME (subset ratio K/N + working-set fit): the win is two-part — the O(N) scan+hash build eliminated AND the scan's own cache traffic no longer evicts the K pages the subset reads (the source's disk-page story, reproduced here at cache scale: N=1M × 32 B nodes ≈ 32 MB > L2, so the scan evicts what the subset needs). At K ≈ N (read-most-everything) the index build amortizes to nothing — expected Tie-to-slower for the lazy form only when the SAME subset is read REPEATEDLY (then a persistent index is the sort-key-cache rule instead); the rule lives at K/N ≪ 1 with one-shot or sparse access. The eager arm's HashMap<usize, &Node> is the faithful source shape (pointer maps over mmap'd entries). Distinct from `collect-avoidance` (alloc staging) — the load-bearing half here is working-set residency; distinct from `encoded-layout-borrowed-view` (accessor re-expansion) — this is index materialization BEFORE a subset read. Suggestion-only (architectural: a parse-only proposer cannot see the direct-access path). Source: Meilisearch blog 2026-03-18 §Full scans are harmful + §But what's the new trick?.".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: Some(eager_pointer_collect_lazy_fetch_bench_spec()),
},
// ── simd: repeatedly filtered integer column stored row-wise → bit-plane ──
// transposition + zero-decompression bitwise predicate evaluation ─────────
// Source: arXiv:2608.26368 (Research 097, the O'Neil & Quass bit-sliced
// index rebuilt as a PRIMARY representation). Values are transposed into b
// bit-planes (b = bits of the column max — adaptive per block in the
// source); a > c filter becomes the two-mask MSB→LSB recurrence
// (gt |= eq & plane when c's bit is 0; eq &= plane when it is 1) — pure
// AND/OR/NOT word ops, NO decompression, 64 values per word op, and b/32
// the bytes to stream (b=6 → 5.3× less). The win needs the AMORTIZATION
// regime: encoding is O(N·b) once (source break-even 7–25 scans; single-
// scan workloads are OUT — the source's own 1M-row single-pass cells
// measured sub-1×) and the win GROWS as b shrinks (b≈32 → 1× compression,
// no win; source guideline b ≤ 16). Point lookups are out (the layout is
// scan-shaped); wide-type EQUALITY loses (scalar == is one instruction vs
// b plane ANDs — the source measured 0.5–0.7× for timestamp eq) — range /
// comparison predicates are the win; large IN-lists scale k·b passes (hash
// on materialized values past ~k=8). Distinct from `soa-hot-field`
// (field-level columnar layout, no compression, no predicate algebra —
// bit-plane is the per-BIT extreme with a different consumer shape); from
// `simd-chunk` (chunked word loops are the VEHICLE here, not the
// mechanism); from `bool-to-bitpack` (flag DENSITY for counting, not value
// predicates); from `sparse-set-bit-iteration-tz-blsr` (iteration WITHIN a
// word, not layout across values). Suggestion-only (a parse-only proposer
// cannot prove the column is re-scanned).
PerfRule {
    rule_id: "bitplane-transpose-repeated-predicate-scan".to_string(),
    name: "repeatedly filtered integer column stored row-wise → encode-once bit-plane transposition + zero-decompression bitwise predicate evaluation (b/32 the bytes, 64 values per word op)".to_string(),
    category: "simd".to_string(),
    buggy_code: "// Row-oriented event column; EVERY dashboard query re-scans it.\nstruct Events {\n    user_age: Vec<u32>, // 4 B/value; 10M events = 40 MB streamed per scan\n}\nimpl Events {\n    fn count_older_than(&self, age: u32) -> u64 {\n        let mut hits = 0u64;\n        for &a in &self.user_age {\n            if a > age {\n                hits += 1; // one value per iteration, full-width bytes\n            }\n        }\n        hits\n    }\n}".to_string(),
    fixed_code: "// Encode ONCE at ingest; every filter is bitwise plane ops — the\n// values are NEVER materialized (zero decompression).\nstruct BitSlicedColumn {\n    planes: Vec<Vec<u64>>, // planes[bit][word]; index 0 = LSB\n    words: usize,\n}\nimpl BitSlicedColumn {\n    fn encode(values: &[u32]) -> Self {\n        let max_val = values.iter().copied().max().unwrap_or(0);\n        let b = (32 - max_val.leading_zeros()).max(1) as usize; // adaptive width\n        let words = values.len().div_ceil(64);\n        let mut planes = vec![vec![0u64; words]; b];\n        for (i, &v) in values.iter().enumerate() {\n            for bit in 0..b {\n                planes[bit][i / 64] |= (((v >> bit) & 1) as u64) << (i % 64);\n            }\n        }\n        Self { planes, words }\n    }\n    // x > c: two-mask MSB→LSB recurrence — pure AND/OR per plane word.\n    // DOMAIN CONTRACT (the paper's §3.5 bias transform): c must be\n    // representable in the column's bit-width — one branch per QUERY, not\n    // per value (an out-of-range c means nothing can exceed it).\n    fn count_gt(&self, c: u32) -> u64 {\n        let b = self.planes.len() as u32;\n        if b < 32 && c >= (1u32 << b) {\n            return 0;\n        }\n        let mut gt = vec![0u64; self.words];\n        let mut eq = vec![u64::MAX; self.words];\n        for (bit, plane) in self.planes.iter().enumerate().rev() {\n            if (c >> bit) & 1 == 0 {\n                // data 1 where c 0 → still-equal values are confirmed greater\n                for w in 0..self.words {\n                    gt[w] |= eq[w] & plane[w];\n                }\n            } else {\n                // data 0 where c 1 → drop from the still-equal set\n                for w in 0..self.words {\n                    eq[w] &= plane[w];\n                }\n            }\n        }\n        gt.iter().map(|w| u64::from(w.count_ones())).sum()\n    }\n}".to_string(),
    context_notes: Some("MEASURED ParetoDominant 3/0/0 — 1.96x/2.16x/1.93x at N=10K/100K/1M (64 filter scans per timed call over the SAME column, b=7 planes, ~50% selectivity; M3 Max release, rustc 1.97, 2026-08-28, load ~4.7-5.6 of 16; verdict-stable x2: 2.08x/2.20x/1.96x on the re-run; allocs 0/128 — the fixed arm allocates its two scratch masks per scan, hoistable by the caller; alloc-det stable). HONEST REGIME NOTE — the measured ~2x is NOT the source's 10.7x headline: in Rust the SCALAR arm auto-vectorizes (LLVM lowers the threshold-count loop to SIMD, ~0.126 ns/elem vs the paper's non-vectorized Java scalar at ~0.65), which compresses the win to the LAYOUT half — b/32 fewer bytes streamed + 64 values per word op. Never cite the paper's 10.7x as this mechanism's isolated number. AMORTIZATION is the rule's regime gate: encoding is O(N*b) ONCE (outside the timed region here by design — the ingest-amortization shape); the source measured break-even at 7-25 scans and its own 1M-row single-pass cells ran sub-1x, so single-scan/write-once workloads are OUT of regime. b<=16 guideline (b~32 = 1x compression, no win); point lookups OUT (scan-shaped layout); wide-type EQUALITY loses (the source: 0.5-0.7x for timestamp eq — scalar == is one instruction vs b plane ANDs); IN-lists scale k*b passes (hash on materialized values past ~k=8). DOMAIN CONTRACT: the predicate constant must be representable in the column width — out-of-range c returns 0 through one branch per query (the paper's §3.5 bias transform; the G1 gate caught the unguarded form marking ALL rows greater on an out-of-range constant). Data-oblivious side property: plane-reduction throughput is selectivity-constant (the source: <6% spread across 0.1%-90% vs the scalar branchy loop's ~4.5x swing). Differentiation in-rule vs soa-hot-field / simd-chunk / bool-to-bitpack / sparse-set-bit-iteration-tz-blsr. Suggestion-only (a parse-only proposer cannot prove the column is re-scanned). Source: arXiv:2608.26368 (Research 097).".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: Some(bitplane_transpose_repeated_predicate_scan_bench_spec()),
},
        // ── layout: row-major single-token matvec (x re-walked per output
        // row) → weights repacked ONCE into consumption-ordered tiles, the
        // activation slice L1-resident across each tile's rows ──────────
        // Source: cflow "Pipeline-Native Transformers" (arXiv:2608.23841,
        // Batch 85, Research 098 — the CC-BY-4.0 paper distill; x86-64
        // source numbers, M3-re-gated here). Single-token decode is
        // bandwidth-bound (arithmetic intensity ~4 FLOP/byte vs machine
        // balance ~20); in the naive row-major GEMV every output row
        // re-walks the full activation x — once W-row + x exceed L1-d the
        // activation is evicted before reuse and re-fetched once per row
        // (n_out × n_in × 4 B of x traffic). The tile fix repacks W at
        // load/conversion time into TILE_ROWS × TILE_COLS tiles stored in
        // compute-consumption order; the matvec walks tiles sequentially
        // and the x column-slice (TILE_COLS × 4 B) stays L1-resident
        // across all TILE_ROWS rows — x traffic drops by TILE_ROWS×.
        // REGIME (the honest M3 note): the crossover sits where W-row + x
        // exceeds L1-d — M3 Max P-core L1-d is 128 KB (4× the source's
        // Sandy Bridge 32 KB), so the win appears only at working sets
        // the source would call comfortably-resident (Tie there — both
        // arms stream W sequentially with x resident). The paper's Q4
        // sibling-projection stripe interleave (QKV/gate-up packed by
        // output stripe so one x pass serves sibling matvecs) is the same
        // repack applied to sibling projections — noted as the variant.
        // Distinct from `transposed-weight-register-gemv` (B62 —
        // register/accumulator granularity for the dot itself;
        // COMPOSABLE: tile first for cache, transpose within a tile for
        // registers), from `loop-order-innermost-contiguous` (B64 —
        // reorders loops over an UNCHANGED layout; this repacks the
        // data), from `soa-hot-field` (struct fields, not matvec
        // weights), and from kernel_opt B14/B15 (GPU global-memory
        // access-count rules — cousins, different substrate). Tile
        // accumulation order changes per-output float summation →
        // rel-error gate, NOT bit-identity (the B64/powi gate shape).
        PerfRule {
            rule_id: "weight-tile-consumption-order".to_string(),
            name: "row-major single-token matvec (x re-walked per output row) → weights repacked once into consumption-ordered tiles, the x slice L1-resident across each tile's rows RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- cache_kernels.cu:265-320; LAYOUT TWIN recorded with that caveat, not the argsort instance): the x-interleaved cache layout is materialized AT INSERT TIME (head_size split into x-sized chunks at key_dst) so the consumer kernel's reads are contiguous aligned x-element runs -- the write side arranges bytes for the reader.".to_string(),
            category: "layout".to_string(),
            buggy_code: "fn matvec_rowmajor(w: &[f32], x: &[f32], y: &mut [f32], n_in: usize) {\n    // W is [n_out x n_in] row-major. Every output row re-walks ALL of x;\n    // once a W row + x exceed L1, x is evicted before its next use and\n    // re-fetched from L2/DRAM once PER ROW (n_out × n_in × 4 B of x\n    // traffic — pure waste: x is the SAME vector every row).\n    for (r, yo) in y.iter_mut().enumerate() {\n        let row = &w[r * n_in..(r + 1) * n_in];\n        let mut acc = 0.0f32;\n        for (wv, &xv) in row.iter().zip(x) {\n            acc += wv * xv;\n        }\n        *yo = acc;\n    }\n}".to_string(),
            fixed_code: "const TILE_ROWS: usize = 64;\nconst TILE_COLS: usize = 512;\n\n/// Repack W ONCE at load/conversion time into consumption-ordered tiles:\n/// tile-major, row-within-tile, column-within-row — the matvec walks this\n/// linearly (setup cost amortized over every decode step).\nfn repack_tiles(w: &[f32], n_out: usize, n_in: usize) -> Vec<f32> {\n    let mut tiles = Vec::with_capacity(w.len());\n    for tr in (0..n_out).step_by(TILE_ROWS) {\n        for tc in (0..n_in).step_by(TILE_COLS) {\n            for r in tr..(tr + TILE_ROWS).min(n_out) {\n                let row = &w[r * n_in..(r + 1) * n_in];\n                tiles.extend_from_slice(&row[tc..(tc + TILE_COLS).min(n_in)]);\n            }\n        }\n    }\n    tiles\n}\n\nfn matvec_tiled(tiles: &[f32], x: &[f32], y: &mut [f32], n_out: usize, n_in: usize) {\n    // y is pre-zeroed; each column tile's partial dot ACCUMULATES into\n    // y[tr + r]. The x slice (TILE_COLS × 4 B = 2 KB) stays L1-resident\n    // across the tile's TILE_ROWS rows — x is read once per row-group\n    // instead of once per row (TILE_ROWS× less x traffic).\n    let mut off = 0usize;\n    for tr in (0..n_out).step_by(TILE_ROWS) {\n        for tc in (0..n_in).step_by(TILE_COLS) {\n            let rows = TILE_ROWS.min(n_out - tr);\n            let cols = TILE_COLS.min(n_in - tc);\n            let xs = &x[tc..tc + cols];\n            for r in 0..rows {\n                let tile_row = &tiles[off + r * cols..off + (r + 1) * cols];\n                let mut partial = 0.0f32;\n                for (wv, &xv) in tile_row.iter().zip(xs) {\n                    partial += wv * xv;\n                }\n                y[tr + r] += partial;\n            }\n            off += rows * cols;\n        }\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 4/0/0 — 1.32x/1.35x/1.35x/1.56x at n_in=4K/16K/32K/64K (n_out=64, TILE 64x512; M3 Max release, rustc 1.97, 2026-08-28, load ~8.9 of 16; verdict-stable x2 — run 2: 1.32/1.35/1.35/1.56; allocs 1/1 + bytes 256/256 equal at every size, alloc-det stable). The win GROWS monotonically with working set — the ratio story the source predicted, SHIFTED for M3's 128 KB P-core L1-d (4x the source's Sandy Bridge 32 KB): W-row + x per row runs 32 KB / 128 KB / 256 KB / 512 KB across the sweep. HONEST SMALL-ARM NOTE: even the L1-resident arm (n=4096, row+x = 32 KB ~ 1/4 L1) measured 1.32x FASTER — the paper's own 1x-L1 cells also read ~1.12x; below the eviction boundary the tile walk still wins on x-slice register/L1 reuse vs the full-x re-walk per row (the mechanism is weaker, not absent). MAGNITUDE HONESTY: the source's 1.12-1.36x AVX2 wall-clock (plus 7.29x fewer L1-d misses, PMU) is the same band we measure — cite that, never the Q4-tile end-to-end numbers. REGIME: single-token GEMV / memory-bound matvec where W is streamed once + x is re-read per output row; the repack is setup-amortized (load/conversion time — never in the hot path). At x + row fully L1-resident with tiny N the indirection can cost (unmeasured below 4K); batched-multi-token matmuls amortize x across the batch differently (the B62 transposed-register rule composes there). Suggestion-only (a parse-only proposer cannot prove the repack site is load-time — a mid-loop repack would regress; the soa-hot-field precedent). Source: arXiv:2608.23841 cflow (Research 098).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(weight_tile_consumption_order_bench_spec()),
        },
        // ── io: software prefetch sprinkled on a bandwidth/storage-bound
        // stream → remove it (the miss pipeline is already full) ─────────
        // Source: cflow §decode optimizations (arXiv:2608.23841, Batch 85,
        // Research 098). THE ANNOTATION CLASS (the varint/precompute-table
        // family — the rule exists to STOP a plausible-looking port): SW
        // prefetch buys ≈nothing when the workload is bandwidth-bound and
        // is ACTIVELY HARMFUL at decode scale — the paper's prefetch
        // thread walked expert regions at stride 256 and burned ~48% of
        // decode time; disabling it alone was their single largest decode
        // win at 1.92×. Direct-I/O A/B: PF ±0.1% at 64 MB working set;
        // PF=1 4% WORSE at 4.7 GB. Two-regime: compute-bound loops CAN
        // hide latency with prefetch (the x86 folk wisdom's home turf);
        // storage/RAM-bound streams add only issue slots + cache
        // pressure. x86-benched source numbers; the aarch64 analog (PRFM)
        // unmeasured here — no stable Rust prefetch intrinsic on aarch64
        // (the powi-vs-powf cross-arch annotation precedent). Proposer-less
        // by design (a losing rewrite never gets a --fix path — the
        // filter-count-to-any rationale applied to prefetch).
        PerfRule {
            rule_id: "software-prefetch-storage-bound".to_string(),
            name: "software prefetch on a bandwidth/storage-bound stream → remove it (the miss pipeline is already saturated; prefetch only adds issue slots + cache pressure)".to_string(),
            category: "io".to_string(),
            buggy_code: "#[cfg(target_arch = \"x86_64\")]\nfn checksum_stream(data: &[u32]) -> u64 {\n    let mut acc = 0u64;\n    for i in 0..data.len() {\n        // SW prefetch of a future block: when the stream is storage/RAM\n        // bound the miss pipeline is already saturated — this only adds\n        // issue slots + L1 pressure (the source: PF=1 was 4% WORSE at\n        // 4.7 GB direct-I/O; the prefetch THREAD burned 48% of decode\n        // time; disabling it alone = 1.92×).\n        unsafe {\n            let ahead = (i + 64).min(data.len() - 1);\n            core::arch::x86_64::_mm_prefetch(\n                data.as_ptr().add(ahead) as *const i8,\n                core::arch::x86_64::_MM_HINT_T0,\n            );\n        }\n        acc = acc.wrapping_add(u64::from(data[i]));\n    }\n    acc\n}".to_string(),
            fixed_code: "fn checksum_stream(data: &[u32]) -> u64 {\n    // The bare stream: the hardware prefetcher already lines up purely\n    // sequential access; on a bandwidth/storage-bound loop the explicit\n    // prefetch bought nothing and cost issue slots + L1 pressure.\n    let mut acc = 0u64;\n    for &v in data {\n        acc = acc.wrapping_add(u64::from(v));\n    }\n    acc\n}".to_string(),
            context_notes: Some("NotBenchable on this box — x86-only source evidence (no stable aarch64 prefetch intrinsic to re-gate with; the powi-vs-powf cross-arch precedent) — and the annotation exists to STOP a plausible-looking port: in the source's own A/B the prefetch arm reads ParetoDominated-adjacent on bandwidth/storage-bound streams (PF ±0.1% at 64 MB working set; PF=1 4% WORSE at 4.7 GB direct-I/O; at decode scale the dedicated prefetch thread burned ~48% of decode time walking expert regions at stride 256 — disabling it alone was their single largest decode win at 1.92×). REGIME: the two-sided boundary — compute-bound loops CAN hide latency with prefetch (the x86 folk wisdom's home regime); storage/RAM-bound streams add only issue slots + cache pressure. Proposer-less BY DESIGN (the filter-count-to-any rationale: a losing rewrite never gets a --fix path). Source: arXiv:2608.23841 (Research 098).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── allocation: structured per-record store → serialize-once byte buffer ─
        // Source: Cloudflare 1.1.1.1 DNS-cache memory optimization (Batch 86,
        // Research 100 — the Big Pineapple distill, 2026-08-27). A frozen-record
        // store that pays one heap allocation PER FIELD PER RECORD (name
        // handle + payload Vec) carries allocator round-up + malloc-header
        // overhead on every record, and the insert path pays that malloc
        // storm per entry. Serializing each record ONCE into one
        // length-prefixed byte buffer + a u32 offset table costs TWO
        // allocations total regardless of record count; the rebuild consumer
        // copies record bytes straight out of the buffer (no re-serialize) —
        // the source's near-isolated attributions were +13% insert / −5%
        // lookup (their END-TO-END numbers are +43%/−19% — never cite those
        // isolated). REGIME: sequential-rebuild consumers win (cache →
        // response rebuild); consumers that randomly index + mutate records
        // in place lose (every patch becomes a buffer rewrite — the source's
        // own round-robin-rotation caveat). ANTI-SHAPE (the source's own
        // rejected alternative): do NOT store the FULL wire message — variant
        // flags (their DNSSEC DO bit) then force caching two copies or
        // post-build filtering; store record bytes + keep the structured
        // metadata you patch on.
        PerfRule {
            rule_id: "serialize-once-byte-buffer-store".to_string(),
            name: "frozen records as per-field heap allocations → serialize once into ONE length-prefixed byte buffer + offset table (2 allocations total, copy-out rebuild)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "// Frozen record cache: every record owns its heap fields - one\n// Box<str> for the name + one Vec<u8> for the payload PER RECORD.\n// At store scale the malloc storm dominates the insert path\n// (2n+ allocations, each paying allocator round-up + header).\nstruct Entry {\n    name: Box<str>,\n    data: Vec<u8>,\n}\n\nfn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> Vec<Entry> {\n    names\n        .iter()\n        .zip(payloads)\n        .map(|(n, p)| Entry { name: n.clone(), data: p.clone() })\n        .collect()\n}".to_string(),
            fixed_code: "// Serialize each record ONCE into one length-prefixed buffer + a\n// u32 offset table: TWO allocations total regardless of record\n// count. Rebuild consumers copy record bytes straight out (no\n// re-serialize); only name-carrying records ever need parsing.\nfn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> (Box<[u8]>, Vec<u32>) {\n    let mut total = 4; // record-count prefix\n    for (n, p) in names.iter().zip(payloads) {\n        total += 4 + n.len() + p.len();\n    }\n    let mut buf = Vec::with_capacity(total);\n    let mut offsets = Vec::with_capacity(names.len());\n    buf.extend_from_slice(&(names.len() as u32).to_le_bytes());\n    for (n, p) in names.iter().zip(payloads) {\n        offsets.push(buf.len() as u32);\n        buf.extend_from_slice(&(n.len() as u16).to_le_bytes());\n        buf.extend_from_slice(n.as_bytes());\n        buf.extend_from_slice(&(p.len() as u16).to_le_bytes());\n        buf.extend_from_slice(p);\n    }\n    (buf.into_boxed_slice(), offsets)\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 4.26x/3.52x/2.90x at n=1K/10K/100K (M3 Max release, rustc 1.97, 2026-08-29, load ~5 of 16; verdict-stable x2 — run 2: 4.26/3.47/2.87), alloc axis EXACT 200,001 → 2 allocations at n=100K (2n+1 → 2: the malloc storm IS the mechanism), bytes 7.90 MB → 4.70 MB (allocator round-up + headers gone), alloc-det stable. The win SHRINKS with n (4.26x → 2.90x) as the copy dominates the per-alloc overhead — still dominant at every tested size. REGIME: sequential-rebuild consumers (cache → response rebuild) — the source's near-isolated +13% insert is the same surface; consumers that randomly index + mutate records in place lose (every patch becomes a buffer rewrite). ANTI-SHAPE: full-wire-message storage (the source's own rejected alternative — variant flags force two copies or post-build filtering; store record bytes + patchable metadata). Distinct from `encoded-layout-borrowed-view` (a READ-side encoding erased by a generic accessor; this is the STORE representation) and from `reuse-allocation-clear` (a scratch buffer ACROSS calls; this is the persisted store shape). Suggestion-only (architectural — a parse-only proposer cannot prove records are frozen or the consumer rebuilds). Source: Cloudflare 1.1.1.1 DNS-cache memory optimization (2026-08-27), Batch 86 / Research 100.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(serialize_once_byte_buffer_store_bench_spec()),
        },
        // ── allocation: unconditional per-record field → context-equal elision ─
        // Source: Cloudflare Big Pineapple (Batch 86, Research 100 — the
        // owner-elision change). When a stored field equals a value the
        // ENCLOSING CONTEXT already holds (their owner == the queried domain
        // = the cache key, in hand at every lookup), storing it per record is
        // a heap allocation for information the context carries for free:
        // Option<Box<T>> with None = "same as context", resolved at read via
        // as_deref().unwrap_or(context). ~80% of their records elided. REGIME
        // (two-sided): high %-equal wins (allocation saved per record, the
        // insert-time equality compare is a cheap early-exit); low %-equal is
        // pure overhead — the Option tag widens the struct + every read pays
        // the branch for nothing. COMPOSABLE with serialize-once (the elided
        // marker is one bit in the record header). Distinct from interning/
        // dedup: nothing is shared or canonicalized — the value is simply
        // ABSENT because the context re-derives it.
        PerfRule {
            rule_id: "context-equal-field-elide".to_string(),
            name: "per-record field that equals the enclosing context (the lookup key) → Option<Box<T>> elided to None, resolved against the context at read".to_string(),
            category: "allocation".to_string(),
            buggy_code: "// Every record stores its owner name unconditionally - even though\n// ~80% of owners EQUAL the queried name (the cache key) that the\n// lookup site already holds. One heap allocation per record for\n// information the context carries for free.\nstruct Answer {\n    ttl: u32,\n    owner: Box<str>,\n}\n\nfn build(owners: &[Box<str>], ttls: &[u32]) -> Vec<Answer> {\n    owners\n        .iter()\n        .zip(ttls)\n        .map(|(o, &t)| Answer { ttl: t, owner: o.clone() })\n        .collect()\n}".to_string(),
            fixed_code: "// None == \"same as the context name\" (elided - the caller resolves\n// against the key it already holds); Some = a chain target that\n// genuinely needs its own allocation. ~80% of records now store NO\n// name at all (the elide comparison runs once per INSERT).\nstruct Answer {\n    ttl: u32,\n    owner: Option<Box<str>>,\n}\n\nfn build(context: &str, owners: &[Box<str>], ttls: &[u32]) -> Vec<Answer> {\n    owners\n        .iter()\n        .zip(ttls)\n        .map(|(o, &t)| Answer {\n            ttl: t,\n            owner: if o.as_ref() == context { None } else { Some(o.clone()) },\n        })\n        .collect()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 3.12x/3.01x/2.00x at n=1K/10K/100K at the ~80% context-equal mix (M3 Max release, rustc 1.97, 2026-08-29, load ~5 of 16; verdict-stable x2 — run 2: 3.13/2.98/2.00), alloc axis EXACT 100,001 → 20,056 at n=100K (the ~20% non-equal owners keep their allocation — the elision regime made visible), bytes 4.04 MB → 2.76 MB, alloc-det stable. The fixed arm pays the insert-time equality compare + the read branch and STILL dominates at every size — the malloc removal outweighs both. REGIME (two-sided): high %-equal wins (the source's DNS distribution, ~80% owners = qname); LOW %-equal is pure overhead — the Option tag + the always-on compare buy nothing (unmeasured below ~50%; the boundary is where allocations-saved < compare-cost, which for small T sits far below 50%). Composes with `serialize-once-byte-buffer-store` (the elided marker is one bit in the record header). Distinct from interning/dedup: nothing is shared or canonicalized — the value is ABSENT because the context re-derives it. Suggestion-only (architectural — a parse-only proposer cannot prove the field equals the context). Source: Cloudflare Big Pineapple (2026-08-27), Batch 86 / Research 100.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(context_equal_field_elide_bench_spec()),
        },
        // ── layout (NotBenchable-speed): bool cluster → one bitflag byte ──────
        // Source: Cloudflare Big Pineapple (Batch 86, Research 100): several
        // per-record bools packed into ONE bitflag byte — their own
        // observation is that the PADDING COLLAPSE shrank the struct by more
        // than the raw bool bytes. Stored-at-scale regime only: a record
        // held millions of times multiplies every byte (their per-entry
        // 953 → 420 B fleet table). CROSS-CITE the kernel_opt INVERSE regime
        // (Batch 32a `bool-fields-over-bitset-hot-reads`): per-tick READ
        // flags stay bools — pack only when FOOTPRINT dominates (bytes
        // stored/streamed), never when decode ALU dominates. DEFAULT-REPR
        // CAVEAT: field reordering makes the collapse layout-dependent —
        // MEASURE size_of before/after (the corpus gate pins 12 → 8 for the
        // fixture layout below; that is evidence for THAT layout, not a
        // promise for yours). NotBenchable-speed: the win is a deterministic
        // size_of delta (the `layout-reorder`/`box-large-enum-variant`
        // class), not a runtime microbench.
        PerfRule {
            rule_id: "bitflag-pack-when-footprint-dominates".to_string(),
            name: "stored-at-scale struct with a cluster of bools → one bitflag byte (footprint regime — the padding collapse can do part of the work; measure size_of before/after)".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Stored-at-scale record: eight independent bools. repr(Rust)\n// clusters them, but the flag bytes + alignment padding still\n// bloat every record in a table held millions of times.\nstruct NodeRecord {\n    weight: u32,\n    reachable: bool,\n    verified: bool,\n    stale: bool,\n    pinned: bool,\n    external: bool,\n    compressed: bool,\n    anchored: bool,\n    sealed: bool,\n}".to_string(),
            fixed_code: "// The eight flags as ONE bitfield byte: 5 bytes of fields round to\n// 8 vs 12 - the padding collapse does part of the work. MEASURE\n// size_of before/after: default-repr reordering makes the collapse\n// layout-dependent (pinned 12 -> 8 in the corpus gate).\nstruct NodeRecord {\n    weight: u32,\n    flags: u8, // bit0 reachable .. bit7 sealed\n}\n\nconst REACHABLE: u8 = 1 << 0;\nconst VERIFIED: u8 = 1 << 1;\nconst STALE: u8 = 1 << 2;\nconst PINNED: u8 = 1 << 3;\nconst EXTERNAL: u8 = 1 << 4;\nconst COMPRESSED: u8 = 1 << 5;\nconst ANCHORED: u8 = 1 << 6;\nconst SEALED: u8 = 1 << 7;".to_string(),
            context_notes: Some("NotBenchable-speed — the win is a deterministic size_of delta (12 → 8 for the pinned fixture layout), the `layout-reorder`/`box-large-enum-variant` class, not a runtime microbench. REGIME: stored-at-scale records (footprint dominates — the source's per-entry 953 → 420 B table) ONLY; the kernel_opt `bool-fields-over-bitset-hot-reads` rule is the documented inverse regime (per-tick READ flags stay bools — decode ALU dominates there). DEFAULT-REPR CAVEAT: field reordering makes the padding collapse layout-dependent — measure size_of before/after on YOUR layout; never promise the collapse (the source's struct gained more from padding than from the raw bool bytes, which need not reproduce). Composes with `serialize-once-byte-buffer-store` (a flag byte packs into the record header). Source: Cloudflare Big Pineapple DNS-cache memory optimization (2026-08-27), Batch 86 / Research 100.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── iterators: map(expensive).filter(relevant) → filter-first on the RAW input ─
        // Source: noahlev.org "Making Rustdoc 33% faster" (2026-08-27,
        // Batch 91 / Research 106 — rust-lang PR 159623, the same mechanism
        // again in PR 159779). Rustdoc's build_extern_trait_impls built the
        // expensive display representation (build_impl) for EVERY trait
        // impl in the entire dependency graph, THEN filtered; the fix
        // re-expressed the relevance filter on the RAW ty data and placed
        // it as a guard before each build_impl call (-20% wall, -12% RSS,
        // perf.rust-lang.org-measured; PR 159779 re-ran the mechanism on
        // synthetic impls, -6%). REGIME (two-sided): win ∝ discard-fraction
        // × transform cost; a wash when the transform is cheap (LLVM
        // inlines + CSEs it into the filter) or the filter is
        // non-selective. PRECONDITION (load-bearing): the predicate must be
        // RE-EXPRESSIBLE on the input type — impossible when the mapped
        // value carries information the raw item cannot derive (the
        // source's own work: they adapted the filter to raw ty data
        // first). The re-expressed filter may be OVER-permissive (the
        // source's own choice — extra kept impls only cost time when
        // consumers tolerate them: precision may drop, recall must not).
        // COMPOSES with `filter-map-fuse` (reorder, THEN optionally fuse);
        // ADJACENT to `eager-pointer-collect-lazy-fetch` (both pay O(all)
        // to use O(subset), but that rule fixes the ACCESS path, this one
        // the PIPELINE order — the corpus's same-family distinct-fix-space
        // precedent). Suggestion-only (a parse-only proposer cannot prove
        // the predicate re-expresses on the input — the rewrite is
        // semantic).
        PerfRule {
            rule_id: "filter-before-expensive-transform".to_string(),
            name: "map(expensive).filter(relevant) → re-express the predicate on the RAW input and filter first (win ∝ discard-fraction × transform cost)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "// Renders EVERY doc into its display page first, then keeps only the\n// published ones. render_page is the expensive step (a fresh\n// allocation + a full pass over the body) and ~94% of its output is\n// discarded by the filter behind it.\nstruct Doc {\n    published: bool,\n    rank: u32,\n    body: [u64; 8],\n}\nstruct Page {\n    published: bool,\n    score: u64,\n}\n\nfn render_page(doc: &Doc) -> Page { /* expensive: allocate + fold */ }\n\nfn published_pages(docs: &[Doc]) -> Vec<Page> {\n    docs.iter().map(render_page).filter(|p| p.published).collect()\n}".to_string(),
            fixed_code: "// The published check needs only the RAW doc - filter FIRST, render\n// only the survivors. Same Vec, same order (filter-then-map preserves\n// order for a pure render and a pure predicate).\nstruct Doc {\n    published: bool,\n    rank: u32,\n    body: [u64; 8],\n}\nstruct Page {\n    published: bool,\n    score: u64,\n}\n\nfn render_page(doc: &Doc) -> Page { /* expensive: allocate + fold */ }\n\nfn published_pages(docs: &[Doc]) -> Vec<Page> {\n    docs.iter().filter(|d| d.published).map(render_page).collect()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 11.85x/11.30x/6.61x at n=1K/30K/300K (x86_64-pc-windows-msvc, rustc 1.97, 2026-08-31, box quiet; verdict-stable x2 — run 1: ~12x/10.67x/8.28x; the ratio jitters with box state, the verdict does not), alloc axis EXACT 300,014 → 18,764 at n=300K (n+14 → 6.25%·n+14: the ~94% discarded renders ARE the mechanism — one escaping digest allocation per render), bytes 2.77 MB → 1.65 MB, alloc-det stable. REGIME (two-sided): win ∝ discard-fraction × transform cost; a WASH when the transform is cheap (LLVM inlines + CSEs it into the filter) or the filter is non-selective — do not apply for cheap/inlinable transforms. PRECONDITION: the predicate must be re-expressible on the input type (the mapped value must carry no information the raw item cannot derive) — the source's own precondition (they adapted the filter to raw ty data first); when not re-expressible the rewrite is IMPOSSIBLE, not merely slower. The re-expressed filter may be OVER-permissive (the source's own choice: keeping extras is correctness-safe when consumers tolerate them — precision may drop, recall must not). COMPOSES with `filter-map-fuse` (reorder, THEN optionally fuse). ADJACENT to `eager-pointer-collect-lazy-fetch` (that rule fixes the ACCESS path for subset reads; this one fixes the PIPELINE order — distinct fix spaces, the corpus's same-family precedent). The 6.61x at 300K vs 11.85x at 1K is the kept-item render work becoming a constant fraction of the total plus collect locality, not a regime boundary. Suggestion-only (semantic rewrite — a parse-only proposer cannot prove re-expressibility). Source: noahlev.org 'Making Rustdoc 33% faster' (2026-08-27) + rust-lang PRs 159623/159779, Batch 91 / Research 106.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_before_expensive_transform_bench_spec()),
        },
        // ── complexity: transform-to-decide → decide on the invariant raw projection ─
        // Source: noahlev.org "Making Rustdoc 33% faster" (2026-08-27,
        // Batch 91 / Research 106 — rust-lang PR 159854). The impl-inlining
        // decision computed the FULL cleaned self type (clean_middle_ty +
        // the param_env normalization of where clauses — 50% of the pass)
        // when the decision consumes only the HEAD of the type (the `Vec`
        // in `Vec<i32>`, the `String` in `&mut String`); deciding from the
        // raw head skipped the normalization entirely (-18% wall). Generic
        // form: the decision needs a cheap PROJECTION that is INVARIANT
        // under an expensive canonicalization — compute the projection on
        // the raw input and pay the canonicalization only behind the
        // decision (or for the survivors — composes with
        // `filter-before-expensive-transform`). PRECONDITION
        // (load-bearing): the projection must be invariant under the
        // transform — if the canonical form can change the projected
        // field, the raw shortcut answers a DIFFERENT question.
        // Suggestion-only (a parse-only proposer cannot prove projection
        // invariance). Caveat: PR 159854's diff was NOT read — the mapping
        // is article-level (Research 106).
        PerfRule {
            rule_id: "decision-projection-before-construction".to_string(),
            name: "transform-then-project to decide → compute the invariant projection on the RAW input; pay the expensive canonicalization only behind the decision".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// The predicate canonicalizes the WHOLE item (sorts + dedupes the\n// tag set into a fresh Vec) only to read one field the RAW item\n// already determines - the transform is pure overhead for every\n// non-matching item.\nstruct Item {\n    tags: [u32; 8],\n}\nstruct Canonical {\n    primary: u32,\n    tags: Vec<u32>,\n}\n\nfn canonicalize(item: &Item) -> Canonical { /* sort + dedupe; primary = tags[0] */ }\nfn raw_primary(item: &Item) -> u32 { /* min of the tags - no allocation */ }\n\nfn count_hot(items: &[Item], hot: u32) -> usize {\n    items.iter().filter(|i| canonicalize(i).primary == hot).count()\n}".to_string(),
            fixed_code: "// primary is INVARIANT under canonicalization (both are the minimum\n// of the tag set) - test the raw projection; canonicalize only the\n// callers that need the full canonical form.\nstruct Item {\n    tags: [u32; 8],\n}\nstruct Canonical {\n    primary: u32,\n    tags: Vec<u32>,\n}\n\nfn canonicalize(item: &Item) -> Canonical { /* sort + dedupe; primary = tags[0] */ }\nfn raw_primary(item: &Item) -> u32 { /* min of the tags - no allocation */ }\n\nfn count_hot(items: &[Item], hot: u32) -> usize {\n    items.iter().filter(|i| raw_primary(i) == hot).count()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 24.58x/24.38x/24.37x at n=1K/30K/300K (x86_64-pc-windows-msvc, rustc 1.97, 2026-08-31, box quiet; verdict-stable x2 — the alloc axis byte-identical across runs and the Pareto verdict identical), alloc axis EXACT 300,000 → 0 at n=300K (the fixed predicate NEVER needs the canonical form — the decision answers from the raw projection), bytes 9.60 MB → 0, alloc-det stable. PRECONDITION (load-bearing — this IS the rule's content): the projection must be INVARIANT under the canonicalization (`canonicalize(i).primary == raw_primary(i)` is pinned over the fixture as a test); if the canonical form can change the projected field, the raw shortcut answers a DIFFERENT question. REGIME: win ∝ pass-rate × canonicalization cost; near-100% pass-rate buys nothing (every item needs the form anyway). The consuming decision here DISCARDS the canonical form (count/decide); when survivors need the form, construct behind the decision — composes with `filter-before-expensive-transform`. Distinct from `ok-or-else`/`unwrap-or-else-lazy` (single-value laziness behind Option — this is decide-from-a-projection) and from `filter-map-fuse` (adapter fusion — no decision content). Suggestion-only (a parse-only proposer cannot prove projection invariance). Caveat: PR 159854's diff was NOT read — the mapping is article-level (Research 106). Source: noahlev.org 'Making Rustdoc 33% faster' (2026-08-27) + rust-lang PR 159854, Batch 91 / Research 106.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(decision_projection_before_construction_bench_spec()),
        },
        // -- io: coalesce-nearby-reads-bounded-gap-cacheability-gated ----
        // Batch 94 (2026-09-01, Research 110 -- the oMLX PR #3260 SSD
        // expert streaming distill). Random-ish addressed reads
        // (per-expert rows) issued as one pread PER ROW merge into one
        // pread per CLUSTER when the byte gap between consecutive rows
        // fits a bounded budget (source: 64 KiB) -- the over-read gap
        // bytes are free ONLY when the OS page cache absorbs them, so
        // the budget is COUPLED TO CACHEABILITY: a direct / F_NOCACHE
        // descriptor coalesces with gap 0 (exactly-adjacent rows only).
        // Merged spans re-select the requested rows afterwards,
        // preserving the caller's order incl. duplicates.
        // Suggestion-only (I/O orchestration -- not a parse-only
        // rewrite); serving-level source numbers only, no isolated
        // corpus bench.
        PerfRule {
            rule_id: "coalesce-nearby-reads-bounded-gap-cacheability-gated".to_string(),
            name: "one pread per requested row -> merge rows within a bounded byte gap into one pread; gap budget collapses to 0 on non-cacheable descriptors".to_string(),
            category: "io".to_string(),
            buggy_code: "// One pread PER requested row: 9 hot experts scattered across two\n// shards cost 9 syscalls, each paying its own overhead; rows 40 KiB\n// apart pay the gap three times over.\nfn read_rows(fd: i32, rows: &[(u64, u32)]) -> Vec<Vec<u8>> {\n    rows.iter().map(|(off, len)| pread(fd, *off, *len)).collect()\n}\n\n".to_string(),
            fixed_code: "// Merge rows whose byte gap fits the budget into ONE pread per\n// cluster, then re-select the requested rows from the merged spans\n// (caller's order and duplicates preserved). Cacheability-gated:\n// gap 0 when the fd is opened no-cache -- over-read is only free\n// when the page cache absorbs it.\nconst MAX_GAP: u64 = 64 * 1024;\nfn read_rows(fd: i32, rows: &[(u64, u32)], cacheable: bool) -> Vec<Vec<u8>> {\n    let budget = if cacheable { MAX_GAP } else { 0 };\n    let mut sorted: Vec<(u64, u64)> =\n        rows.iter().map(|(o, l)| (*o, *o + *l as u64)).collect();\n    sorted.sort_by_key(|r| r.0);\n    let mut spans: Vec<(u64, u64)> = Vec::new();\n    for (off, end) in sorted {\n        match spans.last_mut() {\n            Some((_, last_end)) if off - *last_end <= budget => {\n                *last_end = end;\n            }\n            _ => spans.push((off, end)),\n        }\n    }\n    spans.iter().map(|(s, e)| pread(fd, *s, (e - s) as u32)).collect()\n}\n\n".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source ships serving-level numbers only -- no isolated A/B for the coalescing itself; honestly NotBenchable here). The DECISION CONTENT is the cacheability coupling, not the merge: gap budget = cacheable ? 64 KiB : 0 -- in direct/F_NOCACHE mode over-read bytes are NOT free (they bypass the page cache and cannot repay the extra transfer), so only exactly-adjacent rows merge; on a cacheable descriptor the page cache repays the gap and 64 KiB of slack is cheap. Merge is gap-BOUNDED, not whole-range (an unbounded merge re-reads the shard -- the eager-pointer-collect-lazy-fetch failure shape on the transport axis). Re-selection after the merge preserves the caller's row order incl. duplicates; sort + merge at O(n log n) over the REQUESTED set only. RIDERS from the same source: (1) Darwin hints (fcntl F_RDAHEAD on cacheable fds, F_NOCACHE on direct fds) are applied ONCE per persistent descriptor and advisory-swallowed on unsupported platforms -- a descriptor-lifetime hint, not a per-read posix_fadvise; (2) fd cache keyed (path, cache-mode) so direct and cached descriptors coexist for the same file (note: that dict's get/set is unsynchronized in the source -- double os.open with the loser's fd leaked -- a concurrency hazard recorded in Research 110 section 7, not folded into this rule). Distinct from buffered-io (sequential stream buffering with a fixed window; this is random-addressed access merged by measured gap), from eager-pointer-collect-lazy-fetch (avoids materializing an O(all) index to read a subset; this transports a known subset in fewer syscalls), and from software-prefetch-storage-bound (both are storage-side; SW prefetch hides latency on compute-bound streams, coalescing removes syscall count on random-addressed streams -- different mechanisms, compose freely). Suggestion-only (I/O orchestration; a parse-only proposer cannot see descriptor cacheability). Source: jundot/omlx PR #3260 safetensors.py:448+452 (cold_max_gap_bytes = 64 KiB, clamped), :560-562 (gap-bounded range split), :597-598 (gap 0 when not use_file_cache), :675-678 (re-selection from merged spans), :536-549 (advisory fcntl hints), :526-531 (fd cache keyed (path, use_file_cache)). Batch 94 / Research 110 / Plan 058.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: N known DISJOINT byte-range reads from one file run as one
        // serial pread per range -------------------------------------------
        // Batch 118 (2026-09-06, Research 142 / Plan 097 -- the antirez/ds4
        // PR #991 PLE-gather distill). A cold gather of N fixed-size scattered
        // rows from one large file runs the device at queue depth ONE when
        // issued as one serial pread per row: every row's page-in latency is
        // paid serially (~60 us/row, 6.9-7.7 s per fresh 8K chunk;
        // warm = 204 ms). Splitting the row list into contiguous index ranges
        // across threads -- each doing positional preads on the SHARED fd --
        // overlaps the page-ins (QD1 -> QD32): 6857 -> 338 ms at 32 threads,
        // ~20x cold; warm NEUTRAL; 64 threads no further gain. Byte-identity
        // is by CONSTRUCTION, not by ordering: each row decodes independently
        // from per-thread scratch and writes its own indexed output slice, so
        // completion order is irrelevant and no concatenation step exists.
        // Suggestion-only (I/O orchestration -- not a parse-only rewrite).
        PerfRule {
            rule_id: "parallel-disjoint-pread-hides-cold-page-latency".to_string(),
            name: "N known disjoint byte-range reads from one file issued as one serial pread per range -> thread-parallel positional reads on the shared fd + indexed (not concatenated) writes; byte-identical, engages past a range-count floor".to_string(),
            category: "io".to_string(),
            buggy_code: "// One serial pread PER row: a cold region costs ~60 us/row at queue\n// depth one -- 6.9-7.7 s per fresh 8K chunk; warm cache hides the cost.\nfn gather_rows(fd: i32, offsets: &[u64], row_bytes: usize, out: &mut [u8]) {\n    for (r, off) in offsets.iter().enumerate() {\n        let row = pread_exact(fd, *off, row_bytes); // serial page-ins, QD1\n        out[r * row_bytes..(r + 1) * row_bytes].copy_from_slice(&row);\n    }\n}\n\n".to_string(),
            fixed_code: "// Contiguous range partition across threads; positional reads on the\n// SHARED fd (read_at / pread -- no shared cursor); indexed writes make\n// completion order irrelevant -> byte-identical by construction. Floor +\n// cap + degrade-to-inline keep the transform a pure optimization.\nfn gather_rows(fd: i32, offsets: &[u64], row_bytes: usize, out: &mut [u8]) -> bool {\n    if offsets.len() < 512 {\n        return gather_rows_serial(fd, offsets, row_bytes, out); // engagement floor\n    }\n    let threads = offsets.len().min(32); // source default 32, cap 64\n    let per = div_ceil(offsets.len(), threads);\n    let mut ok = true;\n    std::thread::scope(|s| {\n        let handles: Vec<_> = (0..threads)\n            .map(|t| {\n                let begin = t * per;\n                let end = (begin + per).min(offsets.len());\n                s.spawn(move || {\n                    let mut slice_ok = true;\n                    for r in begin..end {\n                        match read_at_bytes(fd, offsets[r], row_bytes) {\n                            Ok(row) => out[r * row_bytes..(r + 1) * row_bytes]\n                                .copy_from_slice(&row), // indexed write: no ordering step\n                            Err(_) => slice_ok = false,\n                        }\n                    }\n                    slice_ok\n                })\n            })\n            .collect();\n        for h in handles {\n            ok &= h.join().unwrap_or(false); // all-or-nothing: AND of ranges\n        }\n    });\n    ok\n}\n\n".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (source numbers are campaign-level: serial 557 -> 933/971/997 tok/s at 8/16/32 threads, gather 6857 -> 922/533/338 ms; full-model 16K/128 greedy dumps SHA-identical between threads=1 and 32; honestly NotBenchable here -- page-cache state dominates any microbench, see the cold-vs-warm note). MECHANISM: one serial pread per row runs the storage device at queue depth one, so each row's cold page-in latency is paid serially (~60 us/row on the source's SSD); N threads of positional preads overlap the page-ins (QD1 -> QD32). BYTE-IDENTITY BY CONSTRUCTION: fixed-size rows -> disjoint byte ranges -> per-row indexed output slices, so completion order is irrelevant and no concatenation/reordering step exists (their gate: SHA-identical dumps, threads=1 vs 32). REGIME: first-touch/cold page cache dominates (measured ~20x); ranges computable upfront (fixed row size -> offset = row * row_bytes, checked mul); range count >= floor (512 rows ~= 50 KB min gather; source default 32 threads, cap 64, config clamped to row count); file large enough that open/stat amortizes (30 GB sidecar). ANTI-REGIME: sequential dependence between ranges; WARM cache (measured neutral -- gather 200 ms hidden under an 8.3 s GPU wall); tiny inputs where spawn overhead dominates; NEAR-ADJACENT ranges are the OPPOSITE regime -- coalesce-nearby-reads-bounded-gap-cacheability-gated (B94) merges those into fewer syscalls; the two compose only when the set splits into far-apart clusters. RUST MAPPING: std::os::unix::fs::FileExt::read_at on a shared &File (positional, no cursor state) -- never Seek+read on a shared handle (the seek position is shared mutable state); a failed worker degrade means the CALLING thread runs that slice inline (never wrong output); error contract = all-or-nothing (AND of per-range results). Their escape hatch: an mmap serial path behind an env knob. Record-only: a direct-GPU gather variant was on the source's rejected list -- the validated design is CPU-parallel pread. Distinct from coalesce-nearby-reads (fewer syscalls for NEAR rows; this overlaps LATENCY for DISJOINT rows), from buffered-io (sequential streams), and from software-prefetch-storage-bound (compute-bound streams). Suggestion-only (I/O orchestration; a parse-only proposer cannot see fd sharing or page-cache state). Source: antirez/ds4 PR #991 @ 236cb2a5 (MIT) -- ds4_qwen4.c:2340-2345 (the QD1 comment + ~60 us/row), :2327-2338 (EINTR + short-read loop), :2385-2388 (pread_exact + indexed write), :2420-2428 (contiguous range partition), :2346-2357 (floor 512 + cap 64 + clamp), :2430-2438 (degrade-to-inline), :2443-2444 (AND), :2038-2051 (checked-mul offset fn), :2493-2500 (mmap escape); QWEN38_PERF_HANDOFF.md:872-881 (23c: the cold sweep + warm neutral + SHA-identical), :1850 (GPU gather rejected). Batch 118 / Research 142 / Plan 097.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- dispatch: conditional dispatch with both arms loading a KNOWN fn
        // and converging into ONE call site (the DestProp-merged shape) ----
        // Batch 99 (2026-09-02, Research 119 / Plan 067 -- the wasmi v2.0
        // blog distill). rustc's MIR DestinationPropagation (>=1.92, on by
        // default) folds `let f = if c { h_a } else { h_b }; f(x)` into a
        // conditional move of the two code addresses + a SINGLE indirect
        // branch: every dispatch (whichever handler) shares one
        // branch-target-predictor entry and the call can never devirtualize.
        // Verified on this toolchain (x86_64-pc-windows-msvc, rustc 1.97,
        // asm-inspected): the merged form emits `cmovne` + one indirect
        // `jmp r9` tail-jump; the split form emits TWO DIRECT `jmp h_*`
        // tail-jumps -- distinct sites, monomorphic history, no re-merge.
        // Suggest-only (a parse-only proposer cannot see codegen).
        PerfRule {
            rule_id: "mir-destprop-branch-site-merge".to_string(),
            name: "conditional dispatch phi'd into ONE call site (cmov + single indirect branch) -> keep two distinct call sites (direct jumps, per-site predictor history)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// A hot interpreter step loads a KNOWN handler in each arm, then the\n// two fn pointers converge into one call. rustc's MIR\n// DestinationPropagation (>=1.92) turns this into a conditional move of\n// the two code addresses + a SINGLE indirect branch: every dispatch --\n// whichever handler -- shares one branch-target-predictor entry, and\n// the call can never devirtualize.\ntype Handler = fn(&mut State, u32);\n\n#[inline(never)]\nfn handler_add(st: &mut State, x: u32) { st.acc = st.acc.wrapping_add(x); }\n#[inline(never)]\nfn handler_mul(st: &mut State, x: u32) { st.acc = st.acc.wrapping_mul(31).wrapping_add(x); }\n\nfn step(pick_add: bool, st: &mut State, x: u32) {\n    let f: Handler = if pick_add { handler_add } else { handler_mul };\n    f(st, x); // ONE call site shared by both arms\n}".to_string(),
            fixed_code: "// Two DISTINCT tail-call sites, one per arm: each site keeps its own\n// (monomorphic) branch-target history and the compiler emits DIRECT\n// jumps -- no phi, no indirect dispatch, nothing to mispredict. In a\n// hot dispatch loop this was wasmi 2.0's single ~50% fix.\ntype Handler = fn(&mut State, u32);\n\n#[inline(never)]\nfn handler_add(st: &mut State, x: u32) { st.acc = st.acc.wrapping_add(x); }\n#[inline(never)]\nfn handler_mul(st: &mut State, x: u32) { st.acc = st.acc.wrapping_mul(31).wrapping_add(x); }\n\nfn step(pick_add: bool, st: &mut State, x: u32) {\n    if pick_add {\n        handler_add(st, x); // site 1 -- direct, own history\n    } else {\n        handler_mul(st, x); // site 2 -- direct, own history\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 2/0/1 -- 1.04x/1.24x/1.25x at n=1K/100K/1M dispatches, 50/50 mixed stream, the corpus Pareto validator the recorded instrument (x86_64-pc-windows-msvc, rustc 1.97, 2026-09-02, box quiet; alloc axis EXACT 0/0/0/0 at every size, alloc-det stable across 3 runs). Verdict-stable vs the mine-time probe (median-of-15 methodology): 1.11x/1.21x/1.19x + a second run 1.05x/1.22x/1.18x -- the two large sizes are rock-stable at 1.18-1.25x, the 1K cell straddles the Faster threshold (1.04-1.11) and lands Tie under the validator. MECHANISM (asm-inspected on this toolchain, not inferred): the merged form emits `lea rax,[rip+h_a]; lea r9,[rip+h_b]; test cl,cl; cmovne r9,rax; jmp r9` -- a conditional move of the two code addresses + ONE indirect tail-jump; the split form emits TWO DIRECT `jmp h_a` / `jmp h_b` tail-jumps. Both mechanisms are load-bearing and BOTH are the rule's honest content: (1) devirtualization enablement -- the phi of two KNOWN fn items can never fold to a direct call, so the merged form forces an indirect branch forever; (2) per-site predictor history -- the single site is 2-target polymorphic, mixing BTB history across arms. REGIME (measured, three skew arms): 98/02 skew 1.18/1.30/1.28 (most stable -- rare transitions pay full mispredicts at the shared site); 50/50 1.11/1.21/1.19; 75/25 1.31/1.05/1.03 (borderline at the two large sizes). NEGATIVE CONTROL (the boundary, measured): when the two fn pointers are RUNTIME-OPAQUE (fn args, table entries behind opaque loads -- genuinely dynamic dispatch), BOTH forms keep indirect calls and the split buys NOTHING (0.95-1.00x across all nine cells, two runs) -- the rule applies ONLY when the arms load compile-time-known code pointers (fn items / consts / statics), where splitting re-enables direct calls; for genuinely dynamic dispatch see dyn-to-enum (change the representation) instead. TOOLCHAIN REGIME: the deopt class is rustc >=1.92 MIR DestinationPropagation (default-on; the source measured the 1.91->1.92 boundary, ~2800 -> >4200 CoreMark ~+50% from the single fix on the wasmi tail-call dispatch, independently reproduced on Stitch); re-gate on toolchain bumps (the debug-assert-length-hoist cross-arch precedent). The source's own scope note: only the tail-call-based dispatch configurations see improvements (our fixture is the tail-call shape -- the step fn's last op is the handler call). Suggestion-only (a parse-only proposer cannot see codegen shapes). Source: wasmi v2.0 blog 'Accidental Rust Deoptimization' (wasmi-labs.github.io, 2026-09-01, Robin Freyler), Batch 99 / Research 119 / Plan 067.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(mir_destprop_branch_site_merge_bench_spec()),
        },
        // -- dispatch: a hot loop reads its handler fn pointer OUT of a
        // runtime table on EVERY dispatch (the classic `HANDLERS[op](..)`
        // idiom) ----------------------------------------------------------
        // Batch 101 (2026-09-02, Research 119 §Parallel-session addendum --
        // the aarch64 complement of Batch 99, probe-only candidate
        // promoted). On aarch64 the merged/indirect dispatch keeps an
        // IN-LOOP indexed load of the handler (`csel x8, xzr, x22, ne;
        // ldr x8, [x19, x8]; blr x8`) -- and DestProp's SPLIT fix does not
        // help here (the split re-merges structurally on this target). The
        // hoist does: loading both table entries ONCE before the loop and
        // selecting between registers eliminates the per-dispatch load.
        // The win is regime-bound: unpredictable streams only.
        // Suggest-only (a parse-only proposer cannot see codegen).
        PerfRule {
            rule_id: "dispatch-handler-pair-hoist".to_string(),
            name: "hot loop re-reads its handler fn pointer from a runtime table every dispatch -> hoist the table entries once before the loop (register-resident select)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// A hot dispatch loop reads the handler OUT of a runtime table on\n// EVERY dispatch. On aarch64 the indirect call keeps that indexed\n// load in the loop (csel offset + ldr + blr) -- a memory access on\n// the dispatch critical path that the predictor cannot hide on\n// unpredictable streams. Note: splitting into two call sites does\n// NOT fix this shape (DestProp re-merges the split on aarch64 --\n// see mir-destprop-branch-site-merge's cross-arch note).\ntype Handler = fn(u64) -> u64;\n\nstatic HANDLERS: [Handler; 2] = [op_add, op_mul];\n\n#[inline(never)]\nfn op_add(x: u64) -> u64 { x.wrapping_add(0x9E37) }\n#[inline(never)]\nfn op_mul(x: u64) -> u64 { x.wrapping_mul(31) }\n\nfn run(ops: &[u32], xs: &[u64]) -> u64 {\n    let mut acc = 0u64;\n    for (i, &op) in ops.iter().enumerate() {\n        let f = HANDLERS[(op & 1) as usize]; // in-loop indexed load\n        acc = f(acc ^ xs[i]);\n    }\n    acc\n}".to_string(),
            fixed_code: "// Hoist BOTH table entries once before the loop: the handlers sit\n// in registers for the whole loop and the per-dispatch body is a\n// register select + direct-ish call -- the indexed load is GONE from\n// the critical path. Wins on unpredictable streams (LCG-random\n// 1.19-1.23x at 10K/1M dispatches); structured/periodic streams are a\n// Tie (a learnable stream lets the single indirect site's predictor\n// hide the load).\ntype Handler = fn(u64) -> u64;\n\nstatic HANDLERS: [Handler; 2] = [op_add, op_mul];\n\n#[inline(never)]\nfn op_add(x: u64) -> u64 { x.wrapping_add(0x9E37) }\n#[inline(never)]\nfn op_mul(x: u64) -> u64 { x.wrapping_mul(31) }\n\nfn run(ops: &[u32], xs: &[u64]) -> u64 {\n    let f0 = HANDLERS[0]; // hoisted once\n    let f1 = HANDLERS[1];\n    let mut acc = 0u64;\n    for (i, &op) in ops.iter().enumerate() {\n        let x = acc ^ xs[i];\n        acc = if op & 1 == 0 { f0(x) } else { f1(x) };\n    }\n    acc\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant on BOTH arches. aarch64 (M3 Max, rustc 1.97.0, macOS 26.6.2): 2/0/0 -- 1.23x/1.22x at 10K/1M dispatches on the LCG-random (unpredictable) stream, band 1.19-1.23x across 3 runs, alloc axis EXACT 0/0, alloc-det stable x3; raw probe LC 0.837-0.840 = fixed wins ~16-19%, verdict-stable x3. x86_64 (4090 box, i7-13700K, windows-msvc, quiet-window re-gate 2026-09-04, verdict-stable x2): 3/0/0 -- 2.69-2.73x at 1K / 1.09-1.10x at 100K / 1.07-1.08x at 1M, alloc 0/0, alloc-det stable -- the Plan-069 expected-Tie REFUTED: the lea-rip no-in-loop-load analysis belonged to Batch 99's KNOWN-ITEMS fixture (compile-time code pointers); this rule's fixture is a RUNTIME TABLE, so x86 must also load the handler pointer in-loop and the hoist eliminates it on both arches. REGIME (measured, three stream shapes, aarch64): the win exists ONLY on unpredictable streams -- period-16 0.988-1.004 and alternation 0.990-1.003 read Tie (a learnable stream lets the single indirect site's predictor hide the load), so consumers on patterned dispatch streams gain nothing; the x86 cells ran the LCG-random stream only (the regime boundary is predictor-mechanical and expected arch-independent, but the structured-stream cells were not re-measured on x86 -- noted honestly). MECHANISM (asm-pinned on aarch64, not inferred): the merged form keeps an IN-LOOP indexed load (`csel x8, xzr, x22, ne; ldr x8, [x19, x8]; blr x8` -- DestProp's merged-load shape; the classic HANDLERS[op](..) idiom is the buggy form) and the hoist eliminates it. DEDUP vs mir-destprop-branch-site-merge (Batch 99): distinct mechanism AND fixture shape -- that rule = compile-time-KNOWN fn items + tail-call shape + the SPLIT transform (two direct jmp sites, per-site predictor history; x86-verified no-re-merge); this rule = runtime TABLE + value-returning handlers + the HOIST transform (register-resident select; on aarch64 the split re-merges structurally, so Batch 99's fix does not transfer to this shape -- and Batch 99's runtime-opaque NEGATIVE CONTROL covers splitting on unpredictable streams, not hoisting). The two compose in the dispatch family: known items -> split (B99); runtime table -> hoist (this). Suggestion-only (a parse-only proposer cannot see codegen shapes). Source: the wasmi v2.0 distill's parallel-session lane, Research 119 §Parallel-session addendum (probe-only evidence, queued unnumbered, promoted Batch 101 / Plan 069); x86 re-gate 2026-09-04 (Batch 101 FU).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(dispatch_handler_pair_hoist_bench_spec()),
        },
        // -- io: a RESUMABLE multi-part artifact build trusts its on-disk
        // cache -------------------------------------------------------------
        // Batch 121 (2026-09-07, Research 144 / Plan 101 -- the ds4 PR #991
        // IQ2 delta distill, commit c5ae2eb5). A long quantize/build job that
        // caches per-part outputs + a manifest for resume must REVALIDATE
        // every identity input (source payload, calibration data, the TOOL
        // binary itself) against the manifest BEFORE reusing cached parts,
        // and re-verify each cached part's stored digest before skipping its
        // work. A stale-tool or swapped-source resume otherwise "succeeds"
        // and silently mixes outputs across tool versions.
        // Suggestion-only (build orchestration -- not a parse-only rewrite).
        PerfRule {
            rule_id: "resume-revalidates-provenance-before-reuse".to_string(),
            name: "resumable multi-part build reuses cached parts on manifest presence -> revalidate every identity input (source + calibration + tool hashes) against the manifest, and re-digest each cached part, before any reuse; mismatch refuses".to_string(),
            category: "io".to_string(),
            buggy_code: "// A long build caches per-part outputs + a manifest and SKIPS any\n// part already present -- but never checks that the SOURCE, the\n// calibration data, or the TOOL binary still match what produced the\n// cache. After a tool upgrade or a source swap the resume\n// \"succeeds\" and the artifact silently mixes parts from two tool\n// versions.\nfn build_part(dir: &Path, name: &str, src: &Input) -> PathBuf {\n    let out = dir.join(name);\n    if out.exists() {\n        return out; // presence = trust: nothing revalidated\n    }\n    let bytes = quantize(src);\n    std::fs::write(&out, bytes).unwrap();\n    out\n}".to_string(),
            fixed_code: "// The manifest records the build IDENTITY (source + calibration +\n// tool hashes). Resume compares EVERY identity field first -- any\n// mismatch refuses the whole run -- then re-digests each cached part\n// before skipping it. Partial cache from a DIFFERENT identity is a\n// liability, not a speedup.\n#[derive(serde::Serialize, serde::Deserialize)]\nstruct Manifest { source: String, calib: String, tool: String, parts: Vec<Part> }\n\nfn resume_or_build(dir: &Path, id: &Identity) -> Vec<PathBuf> {\n    let path = dir.join(\"manifest.json\");\n    if let Ok(m) = std::fs::read_to_string(&path) {\n        let m: Manifest = serde_json::from_str(&m).unwrap();\n        assert_eq!(m.source, id.source, \"resume provenance mismatch: source\");\n        assert_eq!(m.calib, id.calib, \"resume provenance mismatch: calib\");\n        assert_eq!(m.tool, id.tool, \"resume provenance mismatch: tool\");\n    }\n    // per-part: re-digest before reuse\n    for part in &parts {\n        let p = dir.join(&part.name);\n        if digest(&p) == part.sha256 { continue; } // verified reuse\n        write_part_atomic(&p, part);\n    }\n    parts\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source runs the quantize stage in ~minutes-per-tensor on a 48-layer trunk; resume-cost avoided is a full re-quantize; honestly NotBenchable here -- correctness-of-resume, not speed). THE FAILURE CLASS: a resumable build whose cache key is PRESENCE instead of IDENTITY mixes outputs across tool/source versions after any upgrade or swap -- the artifact builds \"successfully\" and the mixed provenance is invisible until a quality or hash comparison fails downstream. The identity tuple is THREE-WAY and all three are load-bearing: source payload (a swapped checkpoint), calibration data (a different imatrix changes every quantized byte), and the TOOL ITSELF (a new quantizer library produces different encodings from identical inputs -- its hash belongs in the identity). Per-part reuse re-digests the cached file (guards against truncated/tampered cache files, not just manifest drift); identity mismatch refuses the WHOLE run rather than silently rebuilding (a partial rebuild under a new identity would still mix). Composes with atomic-part-writes (.tmp + rename, the next rule): a crash mid-write must leave NO parseable part for the next resume to trust. Distinct from cache-invalidation-by-content-hash patterns (key a memo by input hash -- this is the BUILD-level identity of a multi-artifact pipeline incl. the tool), from commitment-hashing shapes (what goes INTO a digest -- this is what a resume must CHECK), and from checkpoint/resume of long COMPUTES (same instinct, different artifact: intermediate state vs published parts). Suggestion-only (build orchestration; a parse-only proposer cannot see the cache-identity coupling). Source: ivanfioravanti/ds4-metal PR #991 commit c5ae2eb5 (MIT) -- gguf-tools/qwen4_iq2.py:29-46 (identity dict {source, imatrix provenance, library_sha256} + 'Resume provenance mismatch' refuse), :49-52 (per-tensor digest re-verify before 'verified existing tensors' skip), :58-61 (per-part .tmp write + replace + manifest save after EVERY tensor). Batch 121 / Research 144 / Plan 101.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: a multi-part artifact becomes visible the moment the last
        // part is written, BEFORE anything is verified ----------------------
        // Batch 121 (same distill). Write to a hidden path, RE-READ every
        // payload through an independent reader, hash-compare against the
        // plan, THEN atomic-rename publish + sidecar manifest; refuse
        // existing outputs; template/guard checks run BEFORE any byte.
        PerfRule {
            rule_id: "write-verify-readback-before-publish".to_string(),
            name: "multi-part artifact assembly publishes on last write -> write hidden (.incomplete), re-read every payload via an independent reader + hash-compare, then atomic-rename publish with a sidecar manifest; refuse existing outputs".to_string(),
            category: "io".to_string(),
            buggy_code: "// Assemble N parts into one artifact, publishing as you go: a crash\n// or a bad disk leaves a HALF-WRITTEN artifact at the real path,\n// indistinguishable from a complete one -- and nothing re-reads what\n// was written, so a silent corruption ships.\nfn assemble(parts: &[Part], out: &Path) -> std::io::Result<()> {\n    let mut f = std::fs::File::create(out)?; // REAL path from byte one\n    for p in parts {\n        f.write_all(&p.payload)?; // no read-back, no per-part digest\n    }\n    Ok(()) // published before verified\n}".to_string(),
            fixed_code: "// Three gates before the artifact exists: (1) pre-flight template\n// checks + refuse-existing (before ANY byte); (2) write to a hidden\n// .incomplete path; (3) re-read every payload through an INDEPENDENT\n// reader and hash-compare against the plan, then atomic-rename\n// publish + a sidecar manifest recording per-part digests.\nfn assemble(parts: &[Part], out: &Path) -> Result<(), AssembleError> {\n    if out.exists() || incomplete_path(out).exists() {\n        return Err(AssembleError::Exists); // never clobber\n    }\n    check_template(parts)?; // guards BEFORE any byte is written\n    let tmp = incomplete_path(out);\n    write_all_parts(&tmp, parts)?;\n    let mut reader = IndependentReader::open(&tmp)?; // NOT the write buffer\n    for p in parts {\n        if reader.digest(p.name)? != p.expected_sha {\n            return Err(AssembleError::Verify(p.name)); // refuse, do not fix\n        }\n    }\n    std::fs::rename(&tmp, out)?; // atomic publish\n    write_sidecar_manifest(out, parts)?;\n    Ok(())\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's validation read back 1,255 written tensors; assembly refuses existing outputs and 'verifies every written tensor before publishing'; honestly NotBenchable here -- artifact integrity, not speed). WHY AN INDEPENDENT READER: verifying through the same buffered writer/reader that produced the payload can confirm what the CACHE holds, not what the DISK holds; the source opens a fresh Reader on the .incomplete file and seeks per-tensor (an O(1) extra pass over data already destined for disk -- the verify cost is one sequential read of the artifact). WHY ATOMIC-RENAME: consumers key on the artifact PATH; a visible half-artifact is worse than no artifact (crash-consistency: rename is the only publish step that cannot be observed half-done on POSIX). Refuse-existing beats clobber: a re-run into a live path must not destroy the previous artifact before the new one is proven. Template guards run BEFORE byte one (architecture/metadata keys, part count, per-part kind+dims, external-contract keys) -- a mis-planned assembly should cost zero I/O. The sidecar manifest (per-part digest + sizes + total SHA) makes the artifact auditable AFTER publish and gives the NEXT resume its identity inputs (composes with resume-revalidates-provenance-before-reuse: the manifest this rule publishes is the manifest that rule consumes). Distinct from write-tmp-then-rename crash-safety idioms (rename alone without the read-back verifies the RENAME, not the CONTENT), from backup-verify-restore cycles (post-hoc; this gates the publish itself), and from incremental-checkpoint-resume (mid-compute state -- this is the final publish gate). Suggestion-only (I/O orchestration; a parse-only proposer cannot see reader independence or rename atomicity). Source: ivanfioravanti/ds4-metal PR #991 commit c5ae2eb5 (MIT) -- gguf-tools/qwen4_iq2.py:78-80 (refuse existing .out + .incomplete), :82-90 (template guards before any write: arch kv, block count, external-PLE key absent, per-part kind/dims), :91-127 (plan + .incomplete write, per-part .tmp+replace on the quantize side), :129-152 (fresh Reader read-back + 'Written tensor mismatch' refuse), :153-160 (rename publish + sidecar JSON manifest with total sha256). Batch 121 / Research 144 / Plan 101.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement honesty): an A/B on ONE changed component that\n// rebuilds everything else differently is confounded -----------------
        // Batch 121 (same distill). To measure one component's precision\n// change, byte-copy every OTHER component from the baseline artifact\n// and replace ONLY the under-test set; keep confounded recipes out\n// of the headline; report PAIRED differences; memory counters that\n// cannot see mmap-backed device pages are partial budgets, named\n// as such.
        PerfRule {
            rule_id: "ab-isolate-one-component-by-baseline-byte-copy".to_string(),
            name: "A/B on one changed component rebuilt alongside other deltas -> byte-copy every other component from the baseline artifact, replace ONLY the under-test set, report paired diffs; confounded variants stay out of the headline; un-instrumented memory counters are partial budgets, labelled".to_string(),
            category: "io".to_string(),
            buggy_code: "// Measuring the gate/up precision change by building an IQ2 model\n// \"from scratch\": every tensor re-derives, so the diff vs the Q4_K\n// baseline mixes the gate/up change with fresh MXFP4/MTP/control\n// re-encodes -- the comparison cannot attribute a quality delta to\n// the one variable it names.\nfn build_iq2(src: &Checkpoint, calib: &Imatrix) -> Model {\n    let mut m = Model::new();\n    for t in src.tensors() {\n        m.set(t.name, quantize(t, pick_dtype(t), calib)); // EVERYTHING re-quantized\n    }\n    m\n}".to_string(),
            fixed_code: "// Copy every tensor BYTE-IDENTICALLY from the baseline artifact and\n// replace ONLY the under-test set (the 96 trunk gate/up tensors):\n// the artifact diff IS the variable. Paired per-case diffs (test-only\n// passes vs baseline-only passes) replace raw pass counts; confounded\n// recipes stay out of the headline; un-instrumented memory counters\n// are reported as PARTIAL budgets.\nfn build_iq2(src: &Checkpoint, calib: &Imatrix, baseline: &Model) -> Model {\n    let mut m = baseline.clone(); // byte-identical: diff = the variable\n    for layer in 0..48 {\n        for part in [\"gate\", \"up\"] {\n            let name = format!(\"blk.{layer}.ffn_{part}_exps.weight\");\n            let t = quantize_exact(src.tensor(&name), DType::Iq2Xxs, calib);\n            m.replace_verified(&name, t); // digest-checked replacement\n        }\n    }\n    m\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's own numbers demonstrate the trap it avoids: headline hard-suite passes are FLAT across recipes -- IQ2 36/50 vs Q4_K 35/50 -- while the paired BF16 drift is 3-6x worse: NLL 0.27828 vs 0.21287, top-1 agreement 91.32% vs 96.57%, logprob MAE 0.13992 vs 0.04307; honestly NotBenchable here -- experimental design, not speed). FOUR load-bearing halves: (1) ISOLATION BY BASELINE BYTE-COPY -- every non-under-test tensor is copied verbatim from the baseline artifact so the artifact diff IS the variable (the source copies all non-expert tensors from the combined Q4_K GGUF incl. MXFP4 down projections, MTP, dense/control, and swaps only the 96 trunk gate/up tensors, digest-verified); (2) PAIRED differences over raw counts -- report exact per-case disagreements (test-only-passed vs baseline-only-passed: 2/1, 2/1, 3/1), which survive small samples where pass totals do not; (3) CONFOUNDED variants stay out of the headline -- Q8 (also changes down weights) and Q4_0 (an earlier complete recipe) are named as such; 'the most controlled quantization comparison is IQ2 versus Q4_K-imatrix'; (4) UN-INSTRUMENTED counters are PARTIAL budgets, labelled -- peak RSS/footprint 'do not reliably account for all mmap-backed GPU weights and must not be interpreted as the total model memory requirement'; file size + tracked runtime allocations + resident external sidecar pages = a partial budget, and the test box's capacity 'does not validate' another machine's headroom. RIDER (calibration provenance): reuse the SAME pinned calibration/imatrix as the baseline recipe (six zero-count entries take a deterministic weight-energy fallback) so the precision change is not confounded with a calibration change; per-tensor source hashes retained. Distinct from measurement-honesty timing rules (dead-result-timing-loop-eliminated-under-lto -- instrument the TIMING loop; this isolates the VARIABLE under test), from paired-median-of-ratios interleaving (same-box timing A/B methodology -- this is artifact-level A/B design where the baseline itself is the control), and from the lossy-surface promotion rule (deployed-path retention gates -- this is how to BUILD the comparison that informs such a gate). Suggestion-only (experimental design; a parse-only proposer cannot see artifact provenance). Source: ivanfioravanti/ds4-metal PR #991 commit c5ae2eb5 (MIT) -- gguf-tools/README.md IQ2 section ('It then copies every other tensor from an existing combined Q4_K qwen4exp GGUF ... This isolates the gate/up precision change for a quality comparison'), speed-bench/qwen38-iq2-quality.md:37 (confounded recipes + the controlled-comparison line), :25-32 (paired disagreement table), :52 (the mmap/RSS partial-budget caveat), :58 (pinned imatrix + zero-count fallback). Batch 121 / Research 144 / Plan 101.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: a structured transcript turn is assembled by hand-pushing
// individual tokens -------------------------------------------------
        // Batch 122 (2026-09-07, Plan 102 -- the ds4 PR vision delta,
        // commit 69c38d2). The CLI built an image turn by pushing a user
        // token, a vision span, a newline, then the text -- and image
        // turns broke: the hand assembly drifted from the model's own
        // chat template. The fix routes the turn through ONE model-aware
        // multimodal formatter.
        PerfRule {
            rule_id: "model-aware-format-over-hand-assembled-tokens".to_string(),
            name: "structured transcript turn assembled by hand-pushing tokens (role token + special span + separator + text) -> emit through ONE model-aware formatter that owns the template; hand assembly drifts silently".to_string(),
            category: "io".to_string(),
            buggy_code: "// An image turn is assembled piecemeal: user token, vision span,\n// separator, then the text tokens. Each piece is right in isolation --\n// but the sequence drifts from the model's chat template (which role\n// marker wraps the image? where does the text go?), and the model\n// just answers WRONG. No error, no warning: the transcript looks\n// plausible.\nfn add_image_turn(t: &mut Transcript, img: &Image, text: &str) {
    t.push(token_user());              // role marker, hand-picked
    t.push_vision_span(img);           // special tokens, hand-placed
    if !text.is_empty() {
        t.push_str(\"\\n\");             // separator, hand-picked
        t.push_text(text);             // text AFTER the image... or before?
    }
}".to_string(),
            fixed_code: "// ONE model-aware formatter owns the turn: role, image span, and\n// text placement are the TEMPLATE's decisions, not the caller's. The\n// same entry point serves text-only turns; a format failure rolls the\n// whole turn back atomically (nothing half-enters the transcript).\nfn add_image_turn(t: &mut Transcript, img: &Image, text: &str) -> Result<(), TurnError> {
    let rollback = t.len();
    let span = t.add_image(img)?;
    t.append_multimodal_message(\"user\", &[\"\", text], img, span) // template owns the layout
        .map_err(|e| { t.truncate(rollback); e })?;  // atomic: nothing half-enters
    Ok(())
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's fix history IS the failure: image turns worked under the template-aware API and broke under the hand assembly -- the CLI pushed token_user + vision span + newline + text separately, then 69c38d2 replaced all of it with one model-aware multimodal append; honestly NotBenchable here -- correctness of emission, not speed). THE FAILURE CLASS: template drift is SILENT -- the transcript still tokenizes, the model still answers, and nothing downstream can tell the role/span/separator layout stopped matching what the model was trained on; the only witness is answer quality. The formatter must be MODEL-AWARE (the same turn serializes differently per model family/template version) and ATOMIC (a failed span reservation rolls back the whole turn -- a half-turn is a poisoned transcript). Composes with write-verify-readback-before-publish (the transcript IS an artifact being assembled) and resume-revalidates-provenance (the template version is part of the build identity). Distinct from string-formatting-vs-structured-apis (display concerns; this is a SEMANTIC wire format the consumer is a trained model), from escape/encoding rules (character-level; this is turn-structure-level), and from prompt-template-bypass audits (they find the drift; this prevents writing it). Suggestion-only (a parse-only proposer cannot see that pushed tokens form a template-governed sequence). Source: ivanfioravanti/ds4-metal PR #991 commit 69c38d2 (MIT) -- ds4_cli.c:1536-1553 (the hand assembly removed: token_user + repl_chat_add_image + newline + text pushes -> one ds4_chat_append_multimodal_message with rollback_len/rollback_images guard arms). Batch 122 / Plan 102.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: a feature needs an external artifact and ships without
// the provisioning step ---------------------------------------------
        // Batch 122 (same distill, commit 9f4313d). The Qwen vision tower,
// its Metal kernel, and the make target all LANDED -- but the
// downloader never fetched the encoder, so --vision was dead out of
// the box. The fix adds the download target following the existing
// GLM pattern.
        PerfRule {
            rule_id: "implemented-surface-requires-provisioning-target".to_string(),
            name: "feature requires an external artifact it does not vend -> ship the fetch/provision target in the SAME change; an implemented-but-unprovisioned feature is dead out of the box".to_string(),
            category: "io".to_string(),
            buggy_code: "// The feature is COMPLETE: parser accepts --vision, the kernel is\n// wired, tests exist behind it. But the weights live in a registry\n// the setup script never fetches -- every user hits 'file not found'\n// on first run.\nfn main() {
    let args = parse_args();
    if let Some(vision) = &args.vision {
        let encoder = load_encoder(vision)?; // shipped; nothing downloads it
        run(args.model, encoder)?;
    }
    Ok(())
}".to_string(),
            fixed_code: "// The same change that lands the feature lands its provisioning:\n// a named setup target (following the existing per-artifact pattern),\n// a clear on-disk location, a load-time error that NAMES the missing\n// target, and a --check that reports provisioned vs missing.\nfn main() -> Result<(), SetupError> {
    let args = parse_args();
    if let Some(vision) = &args.vision {
        let encoder = match load_encoder(vision) {
            Ok(e) => e,
            Err(_) if !provisioned(VISION_TARGET)? => {
                return Err(SetupError::MissingArtifact { target: VISION_TARGET, hint: format!(\"run: ./setup.sh {VISION_TARGET}\") });
            }
            Err(e) => return Err(e.into()),
        };
        run(args.model, encoder)?;
    }
    Ok(())
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's own commit message: 'Vision for Qwen3.8-Flash-Next is implemented ... but the downloader never fetched the encoder, so --vision could not be used out of the box. Follow the GLM 5.3 pattern: a standalone qwen38-vision target'; honestly NotBenchable here -- provisioning completeness, not speed). THE FAILURE CLASS: the gap is invisible in review (every diff hunk is a working feature) and invisible in CI (tests skip or fake the artifact) -- it surfaces only on a fresh machine, which is exactly the onboarding path. The rule has THREE parts: the provision target EXISTS and is named per-artifact (not a monolithic download-everything); the load-time failure NAMES the target that would have provided it (the error is the setup doc); a --check-style probe reports provisioned-vs-missing without side effects. Composes with resume-revalidates-provenance-before-reuse (the provisioned artifact's hash joins the identity) and model-aware-format-over-hand-assembled-tokens (both are feature-completeness axes that review cannot see). Distinct from vendoring-vs-downloading policy (this is the GAP the policy must close, whatever the policy), from error-message-quality rules (this is provisioning toplogy: the error is one part of three), and from docker-layer-provisioning rules (same instinct, container-scoped). Suggestion-only (a parse-only proposer cannot see which CLI flags require which external files). Source: ivanfioravanti/ds4-metal PR #991 commit 9f4313d (MIT) -- download_model.sh:37 (QWEN38_MMPROJ_REPO=ggml-org/Qwen3.8-Flash-Next-GGUF), :167 (QWEN38_VISION_FILE=mmproj-Qwen3.8-Flash-Next-Q8_0.gguf), :184-168 (the qwen38-vision target: 'about 0.6 GB on disk. Load it with --vision; this target does not update ./ds4flash.gguf'). Batch 122 / Plan 102.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement honesty): a multi-turn REPL regression that
// trusts the process exit code ---------------------------------------
        // Batch 122 (same distill, commit 69c38d2's regression). A REPL
// session can FAIL one turn and still exit zero if a later turn
// succeeds -- the exit code aggregates the SESSION, not the turns.
// The regression must scan the captured diagnostics for per-turn
// failure patterns alongside the exit code, and save full transcripts
// for inspection instead of grading content.
        PerfRule {
            rule_id: "diagnostics-not-just-exit-code".to_string(),
            name: "multi-turn session regression gated on process exit code -> also scan captured stderr for per-turn failure patterns; exit code aggregates the session, not the turns; save full transcripts for inspection".to_string(),
            category: "io".to_string(),
            buggy_code: "// Drive a REPL through several turns in one subprocess; assert on\n// the exit code. Turn 2 fails (logged to stderr), turns 3-4 succeed,\n// the process exits 0 -- the regression passes with a broken turn\n// inside it.\nfn run_session(repl: &str, session: &str) -> anyhow::Result<()> {\n    let out = Command::new(repl).stdin(session).output()?;\n    assert!(out.status.success(), \"session failed\");\n    Ok(())\n}".to_string(),
            fixed_code: "// The exit code is the LAST gate, not the only one: scan the\n// captured stderr for per-turn failure patterns (turn-scoped),\n// verify the success markers the tool itself prints, and save the\n// full transcript for human inspection -- do not try to grade\n// content in the harness.\nfn run_session(repl: &str, session: &str, transcript_path: &str) -> anyhow::Result<()> {\n    let out = Command::new(repl).stdin(session).output()?;\n    let diag = String::from_utf8_lossy(&out.stderr);\n    let turn_failures = scan_failures(&diag, &[\"failed\", \"outside the vocabulary\"]);\n    assert!(turn_failures.is_empty(), \"turn failures: {turn_failures:?}\");\n    assert!(markers_present(&out.stdout), \"turn success markers missing\");\n    std::fs::write(transcript_path, &out.stdout)?; // save, don't grade\n    assert!(out.status.success(), \"session failed\");\n    Ok(())\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's regression comment: 'The REPL can return zero after an image turn fails if a later text turn succeeds. Check diagnostics as well as the process exit code'; the scan is a turn-scoped regex over captured stderr: 'failed|outside the vocabulary', plus the success marker 'ds4: image WxH, N image tokens' proving the encoder actually ran; honestly NotBenchable here -- test honesty, not speed). WHY THE EXIT CODE LIES: a REPL is a session aggregate -- any per-turn failure that the tool recovers from (or that only kills one turn) is invisible in the final status; the failure pattern must be scoped tightly enough to stay a FAILURE signal (the source's pattern also catches a benign 'outside the vocabulary' diag only because that diag IS a failure there). SAVE-DON'T-GRADE: the harness persists full stdout/stderr per mode for human inspection instead of asserting on model output content -- grading content makes the test flaky; grading DIAGNOSTICS keeps it deterministic. Both decode modes run (ordinary + speculative) so the failure class cannot hide behind one path. Distinct from assert-on-stderr-string rules (fragile matching; this ASSERTS on scanned failure patterns and saves everything), from golden-output regression (content-pinned; this is deliberately content-free), and from exit-code-only CI gates (the anti-shape itself). Suggestion-only (a parse-only proposer cannot see that the subprocess is a multi-turn session). Source: ivanfioravanti/ds4-metal PR #991 commit 69c38d2 (MIT) -- tests/test_qwen4_cli_vision.py:46-48 ('The REPL can return zero after an image turn fails if a later text turn succeeds' + the failure scan), :49-51 (the success marker + transcripts saved, 'it does not grade image content'). Batch 122 / Plan 102.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement integrity): a numeric-regression gate that trusts
// argmax/top-k ID equality or tolerant parsing -----------------------
        // Batch 123 (2026-09-07, Plan 103 -- the ds4 upstream surge,
// commits 85a4f0e/5ad935d). The source's own tooling detected FULL-VECTOR
// drift at 4/4 frontiers where argmax IDs matched; the old comparator
// silently accepted missing rows, duplicate rows and NaN.
        PerfRule {
            rule_id: "full-vector-bit-identity-over-argmax".to_string(),
            name: "numeric-regression gate compares argmax/top-k IDs or parses measurement artifacts tolerantly -> compare the FULL float vector bit-exactly (canonical spelling, signed zero preserved), pre-validate artifacts (manifest coverage, no duplicate/missing rows, no NaN, internal ratio consistency), and report drift / invalid / identical as THREE statuses".to_string(),
            category: "io".to_string(),
            buggy_code: "// The comparator trusts partial identity and tolerant parsing: argmax\n// IDs match (a lossy projection -- thousands of logits can drift while\n// the sampled token stays identical), missing rows fold to 0.0, duplicate\n// IDs silently overwrite, NaN parses fine. The drift ships as PASS.\nfn compare(base: &str, cand: &str) -> bool {
    let rows = load_lenient(base);   // missing key .get(key, 0.0); dup overwrites
    for case in cases {
        if argmax(base_logits(case)) != argmax(cand_logits(case)) {
            return false;            // ID equality = the whole gate
        }
    }
    true
}".to_string(),
            fixed_code: "// Three gates BEFORE comparing: (1) artifact validity -- manifest\n// coverage, no duplicate/missing rows, canonical finite spelling (no\n// NaN/Inf/scientific), internal consistency (ratio == num/denom within\n// the producer's half-print unit); (2) full-vector identity -- every\n// float32 bit, signed zero preserved, argmax checked but NOT sufficient;\n// (3) three statuses -- identical / drift (valid but changed) / invalid\n// (untrustworthy), never folded into one boolean.\nfn compare(base: &str, cand: &str) -> Verdict {
    let (b, c) = (validate(base)?, validate(cand)?); // refuse garbage upfront
    for case in &b.cases {
        let (x, y) = (b.full_vector(case), c.full_vector(case));
        if x.to_bits() != y.to_bits() {        // FULL vector, bit-exact
            return Verdict::Drift;             // argmax matched -- still drift
        }
    }
    Verdict::Identical
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's own commit message: 'The existing comparator silently accepts missing rows, duplicate rows and NaN in reproduced controls; the validator rejects them... The frontier tool detects full-vector drift despite matching argmax IDs at all four measured frontiers' -- a MEASURED failure class, not hypothetical; honestly NotBenchable here -- measurement integrity, not speed). WHY ARGMAX IS A LOSY PROJECTION: a kernel/quantization change can drift thousands of logits while the sampled token stays identical -- ID equality is the compression the gate must NOT trust; the source checks argmax AND the full float32 word vector, with the canonical-spelling round trip preserving signed zero ('argmax_logit: float32 bits do not match logits[argmax_id]' + 'argmax_id: not the first maximum of the complete logit vector'). WHY PRE-VALIDATION: a comparison over garbage is a confident wrong answer -- the validator refuses incomplete manifest coverage ('missing manifest cases'), duplicate case IDs, non-canonical numbers (NaN/Inf/scientific notation/blank refused by a strict regex), and internal inconsistency (ratio must equal numerator/denominator within HALF_PRINT_UNIT -- the producer's own rounding). THREE STATUSES: drift (valid but changed bytes) and invalid (untrustworthy artifact) are different failure classes with different actions -- folding them into one boolean destroys the triage. THE WORKSPACE HOOK: this is the comparison contract our own bit-identity gates (B118/B119's byte-identity logit dumps, the quant-family retention gates) already implement -- the rule generalizes it for corpus completeness; a lossy-ID gate on quant/kernels is exactly the class our lossy-surface promotion rule (deployed-path retention) exists to catch. Distinct from diagnostics-not-just-exit-code (captures failure evidence; this validates SUCCESSFUL artifacts before comparing), write-verify-readback-before-publish (the producer's publish gate; this is the consumer's comparison contract over two already-published artifacts), resume-revalidates-provenance-before-reuse (provenance on resume; this is matched-artifact validity + identity semantics), and ab-isolate-one-component-by-baseline-byte-copy (isolation method; this decides what may COUNT AS IDENTICAL). Suggestion-only (a parse-only proposer cannot see which IDs are lossy projections of which vectors). Source: antirez/ds4 @ c0a6119f (MIT) -- gguf-tools/quality-testing/validate_scores.py:146,152 (duplicate-ID refuse + missing-manifest refuse), :93-95 (the canonical finite-number regex '-?(?:0|[1-9][0-9]{0,308})\\.[0-9]{9}'), :120-123 (Decimal ratio == num/denom within half-print-unit); compare_frontier_logits.py:64-68 (canonical float32 spelling round-trip, signed zero preserved), :129-134 (argmax cross-check + full-vector identity), :194 ('identical' / 'drift' / 'invalid' three-status). Batch 123 / Plan 103 / Research 146. RIDER (Batch 129 external confirmation, ds4-metal @ b85a6174 -- the cached hyper-connection activation rejection, speed-bench/qwen38-q2-decode-fusions.md:96-101): TWO WEAK GATES PASSED while the full comparison caught the drift -- the cache passed local FP64-reference error checks AND preserved greedy output on screening prompts, then failed full-logit comparison at 2.63 max-abs / 0.532 RMS with top-1 identical ('Local rounding changes can be amplified in this routed model ... matching generated text alone was not sufficient evidence'). Mechanism note worth keeping: MoE ROUTING amplifies local rounding (one perturbed logit flips an expert choice and the divergence compounds), so routed models need the FULL-vector gate at strictly lower tolerance than dense ones.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (robustness classification): a bool validator that folds\n// error into overflow routes every failure to the destructive path ---
        // Batch 123 (same distill, commit e1711c2). A rendering/embedding\n// failure classified as "doesn't fit" compacted the live conversation --\n// the most destructive, lossy handler -- on a non-capacity error.
        PerfRule {
            rule_id: "renderer-error-is-not-capacity-overflow".to_string(),
            name: "bool try-fit validator hides build/render ERRORS inside the overflow answer -> tri-state (error / fits / overflow); only a TRUE capacity overflow may fire the destructive fallback (compact/truncate/reset), and the error cause propagates out of the callee".to_string(),
            category: "io".to_string(),
            buggy_code: "// bool \"fits\" conflates build failure with doesn't-fit: a rendering\n// error (bad image, tokenizer failure) returns false -> the caller\n// compacts the live transcript -- lossy, irreversible, and caused by\n// nothing the compaction can fix. The error string dies in a stack local.\nfn tool_observation_fits(obs: &Observation, reserve: usize) -> bool {
    let built = build_observation(obs); // Err swallowed: renderer failure...
    match built {
        Ok(text) => projected(text) + reserve < ctx_limit(),
        Err(_) => false, // ...classified as \"doesn't fit\"
    }
}
// caller:
fn ingest(obs: &Observation, reserve: usize) {
    if !tool_observation_fits(&obs, reserve) {
        compact_conversation(\"tool result would exceed context\"); // WRONG path
    }
}".to_string(),
            fixed_code: "// Tri-state: Error / Fits / Overflow. A build failure propagates its\n// cause and never touches the transcript; only a genuine projected-limit\n// overflow may compact.\nenum Fits { Error(String), Yes, No }
fn tool_observation_fits(obs: &Observation, reserve: usize) -> Fits {
    match build_observation(obs) {
        Err(e) => Fits::Error(e.to_string()), // the cause leaves the callee
        Ok(text) => {
            if projected(text) + reserve < ctx_limit() { Fits::Yes } else { Fits::No }
        }
    }
}
// caller:
fn ingest(obs: &Observation, reserve: usize) {
    match tool_observation_fits(&obs, reserve) {
        Fits::Error(e) => report_observation_error(e),  // report, never compact
        Fits::No => compact_conversation(\"tool result would exceed context\"),
        Fits::Yes => append_observation(),
    }
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source changed the validator's return from bool to a tri-state int with the comment '-1 is a rendering/embedding error, not a request to compact the context'; every call site branches 'if (fits < 0) goto observation_error; if (!fits) compact' -- honestly NotBenchable here -- control-flow classification, not speed). THE FAILURE CLASS: a validator whose real domain is {error, fits, overflow} typed as bool collapses error into the ONE answer whose handler is destructive and irreversible (compaction rewrites/summarizes live context); every unrelated failure then surfaces as context loss, and the swallowed error string makes it undiagnosable. THE CONTRACT has three parts: the tri-state return, the destructive handler gated by the ONLY condition that names it (projected tokens exceeding context), and the cause propagated out of the callee (err buffer / error variant) instead of dying in a stack local. Applies to any ingestion loop classifying tool results, attachments, embeddings, or message batches before appending to live state. Distinct from diagnostics-not-just-exit-code (captures failure evidence for later grading; this decides the CONTROL-FLOW classification so a non-capacity error cannot fire a destructive path), from the Result-vs-Option idiom (type-level error propagation generally; this is the specific tri-state where a SUCCESS answer also has a destructive binary split), and from bounded-retry rules (budget conservation; orthogonal axis). Suggestion-only (a parse-only proposer cannot see which false answer routes to which handler). Source: antirez/ds4 @ c0a6119f (MIT) -- ds4_agent.c:9000-9017 (the tri-state signature + the '-1 is a rendering/embedding error' comment), 9911/9929/9944 (call sites: fits < 0 -> observation_error; !fits -> agent_worker_compact), 9972-9976 (the error label frees the observation + returns the real message; the transcript is never touched on -1). Batch 123 / Plan 103 / Research 146.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
// -- io (evaluation design): a learned predictor benchmarked against
// an always-issues baseline conflates the MODEL with the ADMISSION POLICY.
        // Batch 124 (2026-09-07, Plan 104 -- arXiv:2609.04040 distill, walk
// #8). Matched controls -- the SAME confidence gate on a 257-parameter
// online MLP and on a classical stride predictor -- make the neural
// advantage vanish; the gate alone removes 35% of prefetches and lifts
// accuracy 11% -> 15% while DRAM reads move only +-0.07%; gate-closed
// execution reproduces the no-prefetch baseline EXACTLY.
        PerfRule {
            rule_id: "matched-control-disentangles-policy-from-model".to_string(),
            name: "learned predictor benchmarked against an always-issues baseline -> run MATCHED CONTROLS (the same admission gate on the learned model AND the classical baseline) so the model's contribution separates from the gate's; report proxy AND endpoint metrics; make the gate-closed arm bit-identical to the no-action baseline".to_string(),
            category: "io".to_string(),
            buggy_code: "// The learned prefetcher is compared against a classical one that\n// ALWAYS issues: any accuracy win may come from the confidence gate\n// the learned version bundles, not from learning. And the gate is\n// graded on ACCURACY alone -- a proxy that can improve while endpoint\n// cost (DRAM traffic) does not move.\nfn evaluate(learned: &dyn Predictor, classical: &dyn Predictor) -> Report {\n    let a = run(learned);   // gate bundled inside: model vs policy confounded\n    let b = run(classical); // always-issues: no gate at all\n    Report { win: a.accuracy > b.accuracy, accuracy: a.accuracy }\n}".to_string(),
            fixed_code: "// Matched controls: the SAME admission gate wraps BOTH predictors,\n// so the comparison isolates the MODEL. Three report axes: proxy\n// (accuracy), endpoint (DRAM traffic), and the null-arm identity --\n// gate-closed must reproduce the no-action baseline bit-exactly (a\n// provable no-op, not a mostly-no-op).\nfn evaluate(learned: &dyn Predictor, classical: &dyn Predictor) -> Report {\n    let gate = ConfidenceGate::default();\n    let a = run_gated(learned, &gate);   // model varies, gate fixed\n    let b = run_gated(classical, &gate); // same gate: the control\n    let null_arm = run_gated(&NullPredictor, &gate);\n    debug_assert_eq!(null_arm.endpoint, no_action_endpoint()); // identity\n    Report {\n        model_win: a.endpoint < b.endpoint,  // endpoint, not proxy\n        proxy: a.accuracy,\n        endpoint: a.endpoint,\n    }\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, paper-measured (arXiv:2609.04040, SPEC CPU2017 in native ChampSim: the matched-control design made a 257-parameter online MLP INDISTINGUISHABLE from gated stride on random traffic and SLOWER on most regular streams -- the neural advantage was the gate all along; the gate alone removed 35% of prefetches and lifted accuracy 11% -> 15% while DRAM reads changed only 0.07%; honestly NotBenchable here -- evaluation design, not speed). THREE decision contents: (1) MATCHED CONTROLS -- the admission policy is its own variable; wrap the learned model AND the classical baseline in the SAME gate, else the benchmark answers 'is the bundle better?' when it claims to answer 'is learning better?'; (2) PROXY-VS-ENDPOINT -- accuracy (a proxy) improved substantially while the endpoint (DRAM reads) did not move: better proxies do not imply better endpoints, so the report must carry BOTH and the win condition must be the endpoint; (3) NULL-ARM IDENTITY -- gate-closed execution reproduced the no-prefetch baseline exactly, which makes the gate a provable no-op when closed (the admission-gate twin of feature-flag bit-identity: an arm you cannot prove inert when off is an arm that can corrupt the experiment). THE WORKSPACE LAW this paper independently re-derived: gate first, predictor later -- Issue 880's kill-switch (no consumer, no prefetch), evpi_gate, convergence-cadence exits; and its honesty clause is the same one behind the lossy-surface promotion rule (aggregate metrics can be flat while the thing you care about moved). Distinct from ab-isolate-one-component-by-baseline-byte-copy (B121 -- isolates the ARTIFACT under test by byte-copying the baseline; this isolates POLICY from MODEL at evaluation-design level, no artifact copying), from full-vector-bit-identity-over-argmax (Batch 123 -- what may COUNT AS IDENTICAL in a comparison; this decides which COMPARISONS are meaningful at all), from dead-result-timing-loop-folded-under-lto (B109 -- the timing loop's work must not vanish; this is confound removal, not instrument integrity), and from diagnostics-not-just-exit-code (failure evidence; orthogonal stage). Suggestion-only (a parse-only proposer cannot see which benchmark conflates model with policy). Source: arXiv:2609.04040 'Confidence-Gated Admission for Hardware Prefetching: When the Gate Matters More Than the Predictor' (Majdane, Casartelli, Lopedoto, cs.AR/cs.PF, 2026-09-03) -- abstract: matched controls, 257-param MLP vs gated stride, 35% prefetch removal, accuracy 11->15%, DRAM +-0.07%, 'We prove gate-closed execution reproduces the no-prefetch baseline exactly', 'The gate matters more than the predictor, and better proxies do not imply better endpoints'. Batch 124 / Plan 104.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
// -- io (oracle-contract / measurement honesty): when a parity or regression
// CONTROL fails, reproduce it on the UNTOUCHED baseline FIRST -- a failure
// that reproduces there indicts the oracle, not the change under test.
        // Batch 125 (2026-09-07, Plan 106 -- ds4 PR rebase-surge, commit
// ffd85d4). Two shipped instances: (1) compute-shape overclaim -- comparing
// a live session's logprobs (last token via the single-token decode step)
// against a fresh session that prefilled the FULL prefix compares two
// different FP accumulation orders; the fresh side must replay the same
// prefill/decode split before the vectors are comparable; (2) unowned-scratch
// overclaim -- a fused gate/up kernel writes `mid` directly and never
// populates the gate/up scratch the unfused path leaves as intermediate, so
// the oracle must compare the outputs the consumer actually reads.
        PerfRule {
            rule_id: "control-reproduces-on-baseline-trims-overclaim".to_string(),
            name: "a failing parity/regression control is first REPRODUCED on the untouched baseline -- a reproduction indicts the ORACLE's contract, and the fix trims the oracle to consumed outputs under matching compute shape; never widen numeric tolerance as the fix".to_string(),
            category: "io".to_string(),
            buggy_code: "// The parity control fails after a fused kernel lands. The reflex: the\n// fusion regressed -- widen the tolerance or skip the test. But nobody ran\n// the SAME control against the UNTOUCHED baseline.\nfn parity_gate(live: &Session, fresh_prefill: &Session) -> bool {\n    // live: prefill-to-len-1 + single-token decode step for the last token\n    // fresh: FULL-prefix prefill -- a DIFFERENT FP accumulation order\n    max_abs_diff(live.logprobs(), fresh_prefill.logprobs()) < 1e-5 // flaky\n}\n\n// twin: the oracle compares outputs the fused path leaves unowned --\nfn gate_up_parity(unfused_scratch: &Scratch, fused: &Fused) -> bool {\n    unfused_scratch.gate == fused.gate && unfused_scratch.up == fused.up\n    // fused writes `mid` directly; gate/up scratch is never populated\n}".to_string(),
            fixed_code: "// Triage: reproduce the failure on the pre-change baseline. If it\n// reproduces, the oracle overclaims -- trim it to the contract consumers\n// actually depend on, under the SAME compute shape; never touch tolerance.\nfn parity_gate(live: &Session, reference: &Session) -> bool {\n    // both arms: prefill to len-1, then evaluate the last token via the\n    // SAME single-token decode step -- identical accumulation order\n    debug_assert!(parity_gate_baseline_reproduces()); // triage, one-time\n    max_abs_diff(live.logprobs(), reference.logprobs()) == 0.0 // exact now\n}\n\nfn gate_up_parity(unfused: &Scratch, fused: &Fused) -> bool {\n    // compare the CONSUMED outputs only; unowned scratch is skipped in\n    // BOTH arms (named in a comment so the skip is a decision, not drift)\n    fused.mid == unfused.mid\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-shipped (both instances landed in the same QA-hardening commit; honestly NotBenchable here -- oracle-contract correctness, not speed). THE PROTOCOL: (1) reproduce the failing control on the UNTOUCHED baseline -- reproduction splits 'the change regressed' from 'the oracle overclaims' without touching any tolerance; (2) if it reproduces, the oracle asserted a contract the implementation never promised -- trim it to the outputs consumers actually read under the SAME compute shape (chunk boundaries, prefill/decode split, scratch ownership); (3) the fused path may leave optional/unused outputs UNOWNED -- skip them in both arms or compare through the consumer, with the skip named. THE FAILURE CLASS: an oracle that compares across accumulation orders (prefill-computed vs decode-computed) or across ownership boundaries (scratch a fused contract no longer populates) fails FOREVER on correct code -- the suite then either blocks good changes or trains developers to widen tolerances, which destroys the gate for real regressions. Distinct from full-vector-bit-identity-over-argmax (Batch 123 -- comparison STRICTNESS over artifacts: full vector, three statuses; this constructs the comparison so both sides share compute shape + consumed outputs -- the two compose), from ab-isolate-one-component-by-baseline-byte-copy (B121 -- the baseline as ISOLATION mechanism in A/B design; this uses baseline reproduction as the failing-control TRIAGE step), and from diagnostics-not-just-exit-code (B122 -- captures failure evidence; this fixes which CONTRACT the oracle asserts). Suggestion-only (a parse-only proposer cannot see which outputs a consumer reads). Source: ivanfioravanti/ds4-metal @ ffd85d42 (MIT) -- commit ffd85d4 'Preserve Qwen tool handling and validate the main rebase' (tests/ds4_test.c:203-204 'Match the replay's prefill/decode split. A full-prefix prefill uses different floating-point accumulation than the final decode step.' + tests/test_metal_moe_prefill.c:85-87 'Fused gate/up kernels write mid directly. Compare all consumed outputs against the unfused control, not unused scratch.' / 'if (run && i < 2) continue;'). Batch 125 / Plan 106 / Research 148.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement honesty): instrumented per-kernel attribution is
// RANKING evidence, never production timing.
        // Batch 129 (2026-09-08, Plan 110 -- ds4 Qwen Q2 distill). The
// encoder-timeline profiler slowed the pipeline ~38% (54.4 -> ~34 tok/s);
// the ranked-#1 candidate on that profile measured -6.95% and ranked-#2
// -19.70%, while the ranking's LAST item produced the retained win.
        PerfRule {
            rule_id: "instrumented-attribution-not-production-times".to_string(),
            name: "kernel-timeline attribution collected under INSTRUMENTATION is ranking-only evidence -- instrumentation itself slows the pipeline (the source: 54.4 -> ~34 tok/s, ~38%), so never read instrumented absolute times as production costs, never set budget targets from them, and state the wall-time methodology next to the numbers".to_string(),
            category: "io".to_string(),
            buggy_code: "// The profile says gate/up dominates, so the team sets a budget
// target from the instrumented absolute time and greenlights the
// biggest family first -- nobody asked what the profiler cost.
fn attribute_regions(app: &mut App) -> Plan {
    let times = profile_with_instrumentation(app); // ~38% slower run
    let top = times.largest_family();
    Plan { target: times[top], first: top } // absolute vs production
}".to_string(),
            fixed_code: "// Instrumented times rank CANDIDATES; go/no-go comes from an
// un-instrumented A/B screen. The report labels the methodology
// (synchronized wall-time, not hardware counters).
fn attribute_regions(app: &mut App) -> Plan {
    let times = profile_with_instrumentation(app);
    let ranking = times.rank_families(); // ORDER is the evidence
    let candidate = ranking.top();
    let verdict = ab_screen_uninstrumented(app, candidate); // the real gate
    Plan { candidate, verdict, note: \"wall-time attribution, profiler adds overhead\" }
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-shipped (the profiler overhead is stated in the source's own decision doc; honestly NotBenchable here -- measurement honesty, not speed). THREE decision contents: (1) INSTRUMENTED TIMES ARE NOT PRODUCTION TIMES -- the encoder-timeline instrumentation reduced throughput 54.4 -> ~34 tok/s, so its per-kernel-family absolute times cannot set budget targets (speed-bench/qwen38-q2-decode-fusions.md:103-105); (2) RANKING SURVIVES, ABSOLUTES DO NOT -- the profile is still valid evidence for ORDERING candidates, and the ordering can be wrong in aggregate: the ranked-#1 family (Q8 dense projections; IQ2 gate/up unpack measured -6.95%, ranked-#2 routed-input packing -19.70%) contained only rejections while the ranking's LAST item (GDN serialization) produced the retained +2.1-2.9% win -- rank candidates, screen before committing; (3) WALL-TIME IS NOT HW COUNTERS -- synchronized wall-time attribution must be labeled as such next to the numbers (prefill-structural.md:84-88: 'These are synchronized wall-time attributions, not GPU hardware counters; the profiler adds overhead'). Distinct from dead-result-timing-loop-folded-under-lto (B109 -- the harness's own work must not vanish; this is the OBSERVER distorting the observed), from dispatch-cost-sequential-measurement-not-single-op (single-op measurement protocol for ONE dispatch decision; this rules on the validity of instrumented end-to-end attribution as a whole), and from work-scaling-probe-launch-bound (uses profiler RANGES to measure a work function; never rules on absolute-time validity). Suggestion-only (a parse-only proposer cannot see which profiler distorted which run). Source: ivanfioravanti/ds4-metal @ b85a6174 (MIT) -- qwen38-q2-decode-fusions.md:103-105 + qwen38-prefill-structural.md:84-88. Batch 129 / Plan 110 / Research 151.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
// -- io (A/B design): a control arm built by UNSETTING the candidate env
// silently runs the enabled DEFAULT -- both arms identical.
        // Batch 129 (2026-09-08, Plan 110 -- ds4 Qwen Q2 distill). The
// harness originally unset the candidate variable for control; when the
// candidate is the DEFAULT-ON path, unsetting is a no-op and the A/B
// reads ~0 while looking like a clean null result. Fix shipped in-source:
// an explicit --control-value, validated as 0 vs 1.
        PerfRule {
            rule_id: "unset-control-arm-silent-default-parity".to_string(),
            name: "when the candidate behavior is the DEFAULT-ON path, a control arm implemented by UNSETTING the env switch silently runs the same enabled default -- the harness must accept an EXPLICIT control value, validation must pass control!=candidate (0 vs 1), and the report must state which arm is control; an unset-env control on a default-on candidate proves nothing".to_string(),
            category: "io".to_string(),
            buggy_code: "// The candidate optimization is DEFAULT-ON (env unset = enabled).
// The \"control\" arm unsets the same env -- which the enabled default
// treats as ON. Both arms run the candidate; the A/B is vacuous.
fn ab_bench(cfg: &Config) -> f64 {
    let a = run_with(cfg, |e| e.remove(\"CANDIDATE\")); // = default = ON
    let b = run_with(cfg, |e| e.insert(\"CANDIDATE\", \"1\"));
    (b - a) / a // ~0.0 -- reads as 'no effect', proves nothing
}".to_string(),
            fixed_code: "// The control value is EXPLICIT and validated to differ from the
// candidate's; both arm values are echoed in the header so a mislabeled
// arm cannot masquerade as a null result.
fn ab_bench(cfg: &Config, control: &str, candidate: &str) -> f64 {
    assert_ne!(control, candidate, \"control must differ from candidate\");
    let a = run_with(cfg, |e| e.insert(\"CANDIDATE\", control)); // explicit
    let b = run_with(cfg, |e| e.insert(\"CANDIDATE\", candidate));
    eprintln!(\"arms: control={control} candidate={candidate}\");
    (b - a) / a
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-shipped (the harness fix + validation landed in the same commit; honestly NotBenchable here -- experimental design, not speed). THE DEFECT: when the candidate is the DEFAULT-ON path, 'unset the env for control' is a NO-OP -- both arms run the candidate, the measured delta reads ~0, and the vacuous screen looks like a clean null result (or, worse, the sign convention silently flips with arm labeling: the source's 1K ABBA log intentionally labels the enabled default as CONTROL and rollback as candidate, so its negative candidate delta is not a regression -- prefill-structural.md:153-155). THE FIX CONTRACT: (1) the harness accepts an explicit control value (metal_prefill_variant_bench.c:214: the control arm sets cfg->control_value when provided, unsets only when not -- backward compatible, which is how the defect shipped); (2) validation passes --control-value 0 --candidate-value 1 and FAILS if the two arms measure identical (prefill-structural.md:58-61: 'enabled defaults cannot silently make both variants identical'); (3) the report names which arm is control. THE FAMILY: this is the control-arm CONSTRUCTION twin of matched-control-disentangles-policy-from-model (B124 -- matched controls isolate policy from model; this ensures the control arm actually executes the control) and the enable-default twin of the null-arm identity face (B124 half 3: the OFF-arm must be provably inert -- here the OFF-arm was never OFF). Distinct from ab-isolate-one-component-by-baseline-byte-copy (B121 -- artifact-level isolation of the component under test; this is the env-arm wiring of the A/B itself) and from diagnostics-not-just-exit-code (B122 -- failure evidence capture; orthogonal stage). Suggestion-only (a parse-only proposer cannot see which env var defaults to ON). Source: ivanfioravanti/ds4-metal @ b85a6174 (MIT) -- speed-bench/qwen38-prefill-structural.md:58-61, :153-155 + metal_prefill_variant_bench.c:52, :214 (the --control-value harness fix). Batch 129 / Plan 110 / Research 151.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (admission policy): a WILLNEED prefault of a huge mmap sidecar
        // runs unconditionally -------------------------------------------
        // Batch 139 (2026-09-09, Plan 119 / Research 154 -- the ds4-metal
        // PR #5 PLE-prefault distill, commits bd7061b + 881c678). A ~30 GiB
        // memory-mapped n-gram sidecar inherited prefetch_cpu=true, so
        // model_open's whole-file POSIX_MADV_WILLNEED dragged the table
        // toward the page cache at startup: on 64 GB machines pressure
        // stops it partway (10.5 GiB resident measured on an M5 Pro) and
        // the prefaulted pages feed SUSTAINED pressure at larger contexts,
        // while demand paging keeps the true working set ~700 MiB. The
        // fix ADMITS the prefault only with real RAM headroom, fails
        // CLOSED when RAM cannot be queried, logs the decision, and
        // keeps a force override; the companion DONTNEED sweep is
        // default-off with the source's own unbated-cost caveat.
        PerfRule {
            rule_id: "prefault-admission-keyed-to-ram-headroom".to_string(),
            name: "a WILLNEED/madvise prefault of a large mmap sidecar gated by nothing -> admit it only when hw.memsize covers resident model + sidecar + a fixed OS margin, FAIL CLOSED on an unqueryable RAM total, log the decision, and keep an explicit force override; the release sweep (DONTNEED) is a separate default-off knob".to_string(),
            category: "io".to_string(),
            buggy_code: "// The sidecar opens with prefetch_cpu=true: a whole-file WILLNEED
// runs at startup regardless of machine size. On a 64 GB box hosting
// a 41.7 GiB model the prefault fights the model for page cache,
// stops partway (10.5 GiB resident measured), and the pages it DID
// pull feed sustained pressure for the rest of the session.
fn open_sidecar(path: &str) -> Sidecar {
    let m = mmap_readonly(path)?;
    madvise(m.as_ptr(), m.len(), Advice::WillNeed)?; // unconditional
    Ok(Sidecar { map: m })
}

".to_string(),
            fixed_code: "// Admission: prefault ONLY with measured headroom (model + sidecar +
// margin); an unqueryable RAM total FAILS CLOSED to demand paging;
// the decision is logged and force-overridable. The release sweep is
// a separate default-off knob.
fn open_sidecar(path: &str, model_bytes: u64) -> Result<Sidecar, Open> {
    let m = mmap_readonly(path)?;
    let prefault = match prefault_admission(model_bytes, m.len() as u64) {
        Admission::Yes => {
            eprintln!(\"sidecar: full prefault (headroom ok)\");
            true
        }
        reason => {
            eprintln!(\"sidecar: demand-paged ({reason:?})\");
            false
        }
    };
    if prefault {
        madvise(m.as_ptr(), m.len(), Advice::WillNeed)?;
    }
    Ok(Sidecar { map: m })
}

enum Admission { Yes, NoRamQuery, InsufficientHeadroom }

fn prefault_admission(model: u64, sidecar: u64) -> Admission {
    const MARGIN: u64 = 16 << 30;
    let Some(total) = host_ram_bytes() else { return Admission::NoRamQuery };
    if total >= model + sidecar + MARGIN {
        Admission::Yes
    } else {
        Admission::InsufficientHeadroom
    }
}

fn host_ram_bytes() -> Option<u64> {
    None // sysctl/proc equivalent; None must mean NO prefault
}

".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source ships campaign numbers, no isolated A/B of the admission itself; honestly NotBenchable here -- page-cache state dominates any microbench). THE TWO-SIDED TRADE, stated in-source: cold idle prefill is ~1/3 slower demand-paged (~650 vs ~950 t/s on an M5 Max) -- the admission BUYS warm-start speed only where it cannot hurt; 128 GiB machines pass the same predicate (128 >= 41.7 + 29.8 + 16) and prefault again, so the constant is portable even though the verdict is per-box. FOUR load-bearing halves: (1) the margin is an OS reserve, not slack -- 16 GiB fixed; (2) FAIL-CLOSED direction: unreadable hw.memsize disables the prefault (slow-but-safe), never prefaults on a guess -- resource-admission heuristics fail toward the demand-paged default; (3) the decision is a logged line naming which arm ran (the source: 'PLE sidecar resident prefetch enabled (full table)' / 'PLE sidecar demand-paged (insufficient RAM headroom or DS4_QWEN4_PLE_PREFETCH_FULL=0)'), and the env override forces either arm; (4) the companion POSIX_MADV_DONTNEED sweep (evict gathered tokens every N calls; measured 717 -> 482 MiB @1024 -> 129 MiB @256) is DEFAULT-OFF with the source's own caveat 'the full-power throughput cost of the sweep is not yet benchmarked' -- an unmeasured cost ships behind an explicit opt-in, never as the default. INSTRUMENT CAVEAT worth keeping with the rule: on macOS, mincore()/resident-page counters report CACHE-WIDE residency of the backing FILE, not pages mapped into THIS task -- the sweep's code REFUSES to log a residency figure and names vmmap against the live process as the reliable instrument; a metric that measures the wrong scope is refused, not reported. Distinct from parallel-disjoint-pread-hides-cold-page-latency (B118 -- overlaps cold page-ins ONCE at gather time; this decides whether pages are faulted EAGERLY at all and what backs the eviction policy), from coalesce-nearby-reads-bounded-gap-cacheability-gated (B94 -- merges syscalls for random reads; different axis: fault-in policy vs read shape), from batch-pool-read-makes-serial-readahead-redundant (B123 -- deletes a read-ahead HINT that duplicates a queued parallel pool; this keeps a prefault but ADMITS it by headroom -- the hint there was redundant work, the prefault here is a capacity decision), and from sparse-demand-paged-kv-cache (kernel_opt B107 -- VRAM placement sparsity; this is host page-cache policy). Suggestion-only (a parse-only proposer cannot see mapping sizes or host RAM). Source: ivanfioravanti/ds4-metal @ 18ca8ec (MIT) -- ds4.c:66896-66903 (the demand-by-design comment), :66928-66953 (the admission + margin + decision log + DS4_QWEN4_PLE_PREFETCH_FULL override), :42376-42385 (sysctl hw.memsize), :42407-42438 (the DONTNEED sweep + DS4_QWEN4_PLE_EVICT_TOKENS + the mincore refusal note 'the sweep does not log a residency figure; vmmap against the live process is the reliable measurement'); commit 881c678 msg ('Restore the full-table WILLNEED prefault only when hw.memsize covers the resident model, the sidecar, and a 16 GiB OS margin: 128 GiB machines prefault again ... while 64 GiB machines stay demand-paged (64 < 41.7 + 29.8 + 16)'; 'Greedy output is byte-identical cold vs warm and with the override in either direction'). Batch 139 / Plan 119 / Research 154.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// Bench spec for `debug-stat-full-scan-out-of-hot-path` (Batch 76,
/// Research 090 — the Meilisearch LMDB/vector-store distill): per-insert-batch
/// mean-degree statistic recomputed over the ENTIRE store vs maintained
/// incremental aggregate.
///
/// sizes = STORE SIZE N (the accumulation axis): the buggy arm pays O(N) per
/// op (B=100 inserted per call) — the win GROWS with store size (the source:
/// 559s → 2–3s at 19M items). G1: both arms return the same final
/// (items.len, `link_count`, `mean_bits`) triple, `debug_assert_eq`'d against a
/// make_input-computed ground truth. Alloc axis: equal by design (2 Vecs for
/// the store + batch in both arms) — the TIMING axis is the mechanism.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn debug_stat_full_scan_out_of_hot_path_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 300_000],
        harness: r#"
pub struct Item {
    pub links: Vec<u32>,
}
pub struct Input {
    pub store: Vec<Item>,
    pub batch: Vec<Item>,
    pub link_count: usize, // the maintained aggregate (fixed arm's starting state)
    pub expected_len: usize,
    pub expected_links: usize,
}
fn make_items(rng: &mut u64, count: usize, max_links: usize) -> Vec<Item> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        *rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let n = (*rng as usize) % max_links;
        let mut links = Vec::with_capacity(n);
        for _ in 0..n {
            *rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            links.push((*rng >> 16) as u32);
        }
        out.push(Item { links });
    }
    out
}
pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    // the store starts at n - B items so post-insert size is exactly n
    let b = 100usize;
    let store = make_items(&mut rng, n.saturating_sub(b), 8);
    let batch = make_items(&mut rng, b, 8);
    let link_count = store.iter().map(|i| i.links.len()).sum();
    let expected_len = store.len() + batch.len();
    let expected_links = link_count + batch.iter().map(|i| i.links.len()).sum::<usize>();
    Input { store, batch, link_count, expected_len, expected_links }
}
mod buggy {
    use super::Item;
    // the statistic's scan IS the cost: one pass over EVERY item's links
    // per insert batch — the source's mean-degree sanity statistic,
    // "added very early on", eagerly computed in the hot path. Non-mutating
    // (statistic over store+batch) so timed calls are idempotent.
    pub fn stat_with_batch(items: &[Item], batch: &[Item]) -> (usize, usize, u64) {
        // O(total) per op — re-derives the WHOLE store's link count
        let total: usize = items.iter().chain(batch).map(|i| i.links.len()).sum();
        let len = items.len() + batch.len();
        let mean = len as f64 / total.max(1) as f64;
        (len, total, mean.to_bits())
    }
}
mod fixed {
    use super::Item;
    // maintained aggregate: link_count was updated by DELTA at each insert —
    // the hot path pays only the NEW batch's links, never the whole past
    pub fn stat_with_batch(items: &[Item], link_count: usize, batch: &[Item]) -> (usize, usize, u64) {
        let batch_links: usize = batch.iter().map(|i| i.links.len()).sum(); // O(B)
        let total = link_count + batch_links;
        let len = items.len() + batch.len();
        let mean = len as f64 / total.max(1) as f64;
        (len, total, mean.to_bits())
    }
}
pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let r = buggy::stat_with_batch(&input.store, &input.batch);
    let chk = r.0.wrapping_mul(31).wrapping_add(r.1).wrapping_mul(31).wrapping_add(r.2 as usize);
    std::hint::black_box(chk);
    debug_assert_eq!(r.0, input.expected_len);
    debug_assert_eq!(r.1, input.expected_links);
}
pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let r = fixed::stat_with_batch(&input.store, input.link_count, &input.batch);
    let chk = r.0.wrapping_mul(31).wrapping_add(r.1).wrapping_mul(31).wrapping_add(r.2 as usize);
    std::hint::black_box(chk);
    debug_assert_eq!(r.0, input.expected_len);
    debug_assert_eq!(r.1, input.expected_links);
}
"#
        .to_string(),
    }
}

/// Bench spec for `eager-pointer-collect-lazy-fetch` (Batch 76, Research 090 —
/// the Meilisearch LMDB/vector-store distill): full-scan HashMap-of-pointers
/// build to serve a K-id subset vs direct lazy access.
///
/// sizes = STORE SIZE N at fixed K=1,000 touched ids (the subset-ratio axis:
/// N=1M × 32 B nodes ≈ 32 MB > L2 — the scan's cache traffic evicts what the
/// subset reads; the source's disk-page story at cache scale). G1: both arms
/// fold the same wrapping checksum over the touched payloads, `debug_assert_eq`'d
/// against the `make_input` ground truth. Alloc axis: buggy builds a
/// `HashMap`<usize, &Node> (3 allocs, ~2.4 MB at N=1M); fixed allocates nothing.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn eager_pointer_collect_lazy_fetch_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 300_000, 1_000_000],
        harness: r#"
pub const K_TOUCHED: usize = 1_000;
pub struct Node {
    pub payload: [u64; 4],
}
pub struct Input {
    pub nodes: Vec<Node>,
    pub ids: Vec<usize>,
    pub expected_chk: u64,
}
pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x0DDB_1A5E_5BAD_5EED;
    let mut nodes = Vec::with_capacity(n);
    for _ in 0..n {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        nodes.push(Node { payload: [rng, rng >> 13, rng >> 29, rng >> 47] });
    }
    // K ids scattered over the whole range — the subset pattern
    let mut ids = Vec::with_capacity(K_TOUCHED);
    let mut expected_chk: u64 = 0;
    for _ in 0..K_TOUCHED {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let id = (rng as usize) % n;
        ids.push(id);
        expected_chk = expected_chk
            .wrapping_mul(31)
            .wrapping_add(nodes[id].payload[0]);
    }
    Input { nodes, ids, expected_chk }
}
mod buggy {
    use super::Node;
    // "a full scan of all the entries we will need to access", pointer map
    // over everything — "polluting the cache with useless, unnecessary disk
    // pages just by iterating over all of them to fetch pointers"
    pub fn touch(nodes: &[Node], ids: &[usize]) -> u64 {
        let mut index = std::collections::HashMap::with_capacity(nodes.len());
        for (i, n) in nodes.iter().enumerate() {
            index.insert(i, n); // O(all) build to serve a subset
        }
        let mut acc = 0u64;
        for &id in ids {
            if let Some(n) = index.get(&id) {
                acc = acc.wrapping_add(n.payload[0]);
            }
        }
        acc
    }
}
mod fixed {
    use super::Node;
    // lazy/on-demand — "only the necessary pages are loaded into the cache";
    // the store IS the direct-access path
    pub fn touch(nodes: &[Node], ids: &[usize]) -> u64 {
        let mut acc = 0u64;
        for &id in ids {
            if let Some(n) = nodes.get(id) {
                acc = acc.wrapping_add(n.payload[0]);
            }
        }
        acc
    }
}
pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let acc = buggy::touch(&input.nodes, &input.ids);
    std::hint::black_box(acc);
    debug_assert_eq!(acc, input.expected_chk);
}
pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let acc = fixed::touch(&input.nodes, &input.ids);
    std::hint::black_box(acc);
    debug_assert_eq!(acc, input.expected_chk);
}
"#
        .to_string(),
    }
}

/// Bench spec for `weight-tile-consumption-order` (Batch 85, Research 098
/// — the cflow "Pipeline-Native Transformers" distill): row-major matvec
/// (x re-walked per output row) vs weights repacked ONCE into
/// consumption-ordered 64×512 tiles (the x slice L1-resident across each
/// tile's 64 rows).
///
/// n = `n_in`; `n_out` fixed at 64 — the memory-bounded shape that crosses M3
/// Max's 128 KB P-core L1-d (4× the source's Sandy Bridge 32 KB target —
/// the size axis is SHIFTED, the ratio story preserved). W-row + x per row:
///   n=4096  →  32 KB (L1-resident — the honest Tie arm; both arms stream
///                     W sequentially with x resident)
///   n=16384 → 128 KB (the L1 boundary — transition)
///   n=32768 → 256 KB (2× L1 — eviction regime; row-major re-fetches x
///                     from L2 once per row, tiled once per row-GROUP)
///   n=65536 → 512 KB (4× L1 — the paper's headline regime)
/// `TILE_ROWS=64` = `n_out` (one row-group); `TILE_COLS=512` (x slice = 2 KB).
/// The repack lives in `make_input` (setup — NEVER measured). Float
/// sum-order differs between the arms (per-tile partials) — the corpus
/// rel-error gate covers it (NOT bit-identity; the B64 gate shape).
#[cfg(feature = "rust_perf_bench")]
fn weight_tile_consumption_order_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![4_096, 16_384, 32_768, 65_536],
        harness: "\nmod buggy {\n    pub fn matvec_rowmajor(w: &[f32], x: &[f32], y: &mut [f32], n_in: usize) {\n        for (r, yo) in y.iter_mut().enumerate() {\n            let row = &w[r * n_in..(r + 1) * n_in];\n            let mut acc = 0.0f32;\n            for (wv, &xv) in row.iter().zip(x) {\n                acc += wv * xv;\n            }\n            *yo = acc;\n        }\n    }\n}\n\nmod fixed {\n    pub const TILE_ROWS: usize = 64;\n    pub const TILE_COLS: usize = 512;\n    pub fn repack_tiles(w: &[f32], n_out: usize, n_in: usize) -> Vec<f32> {\n        let mut tiles = Vec::with_capacity(w.len());\n        for tr in (0..n_out).step_by(TILE_ROWS) {\n            for tc in (0..n_in).step_by(TILE_COLS) {\n                for r in tr..(tr + TILE_ROWS).min(n_out) {\n                    let row = &w[r * n_in..(r + 1) * n_in];\n                    tiles.extend_from_slice(&row[tc..(tc + TILE_COLS).min(n_in)]);\n                }\n            }\n        }\n        tiles\n    }\n    pub fn matvec_tiled(tiles: &[f32], x: &[f32], y: &mut [f32], n_out: usize, n_in: usize) {\n        let mut off = 0usize;\n        for tr in (0..n_out).step_by(TILE_ROWS) {\n            for tc in (0..n_in).step_by(TILE_COLS) {\n                let rows = TILE_ROWS.min(n_out - tr);\n                let cols = TILE_COLS.min(n_in - tc);\n                let xs = &x[tc..tc + cols];\n                for r in 0..rows {\n                    let tile_row = &tiles[off + r * cols..off + (r + 1) * cols];\n                    let mut partial = 0.0f32;\n                    for (wv, &xv) in tile_row.iter().zip(xs) {\n                        partial += wv * xv;\n                    }\n                    y[tr + r] += partial;\n                }\n                off += rows * cols;\n            }\n        }\n    }\n}\n\nconst N_OUT: usize = 64;\n\npub struct Input { w: Vec<f32>, tiles: Vec<f32>, x: Vec<f32>, zero: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let hash = |i: usize, s: u32| {\n        let h = (i as u32).wrapping_mul(0x9E37_79B9).wrapping_add(s);\n        ((h >> 8) as f32) / 16_777_216.0 - 0.5\n    };\n    let w: Vec<f32> = (0..N_OUT * n).map(|i| hash(i, 1)).collect();\n    let x: Vec<f32> = (0..n).map(|i| hash(i, 2)).collect();\n    let tiles = fixed::repack_tiles(&w, N_OUT, n); // setup — never measured\n    Input { w, tiles, x, zero: vec![0.0; N_OUT] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut y = input.zero.clone();\n    buggy::matvec_rowmajor(&input.w, &input.x, &mut y, input.x.len());\n    std::hint::black_box(&y);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut y = input.zero.clone();\n    fixed::matvec_tiled(&input.tiles, &input.x, &mut y, N_OUT, input.x.len());\n    std::hint::black_box(&y);\n}\n".to_string(),
    }
}

/// Bench spec for `bitplane-transpose-repeated-predicate-scan` (Batch 83,
/// Research 097 — arXiv:2608.26368 distill): 64 filter scans per timed call
/// over the SAME column (the ingest-amortization regime — encoding happens
/// once in `make_input`, OUTSIDE the timed region by design; the source's
/// break-even is 7–25 scans). Values in [0, 100) → b = 7 planes; threshold
/// 50 ≈ 50% selectivity (the scalar branch predictor's worst case). G1 is
/// integer-exact: the bit-plane popcount MUST equal the scalar count.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn bitplane_transpose_repeated_predicate_scan_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 1_000_000],
        harness: r#"
const SCANS: usize = 64;      // scans per timed call (amortization regime)
const BIT_CAP: u64 = 100;     // values in [0, 100) -> b = 7 planes
const THRESHOLD: u32 = 50;    // ~50% selectivity

mod buggy {
    pub fn scan_count(values: &[u32], threshold: u32) -> u64 {
        let mut hits = 0u64;
        for &v in values {
            if v > threshold {
                hits += 1;
            }
        }
        hits
    }
}

mod fixed {
    pub struct BitSliced {
        pub planes: Vec<Vec<u64>>, // [bit][word]
        pub words: usize,
    }

    pub fn gt_popcount(col: &BitSliced, threshold: u32) -> u64 {
        let b = col.planes.len() as u32;
        if b < 32 && threshold >= (1u32 << b) {
            return 0;
        }
        let mut gt = vec![0u64; col.words];
        let mut eq = vec![u64::MAX; col.words];
        for (bit, plane) in col.planes.iter().enumerate().rev() {
            if (threshold >> bit) & 1 == 0 {
                for w in 0..col.words {
                    gt[w] |= eq[w] & plane[w];
                }
            } else {
                for w in 0..col.words {
                    eq[w] &= plane[w];
                }
            }
        }
        gt.iter().map(|w| u64::from(w.count_ones())).sum()
    }
}

pub struct Input {
    values: Vec<u32>,
    fixed_col: fixed::BitSliced,
    threshold: u32,
    expected_per_scan: u64,
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        values.push(((rng >> 33) % BIT_CAP) as u32);
    }
    let max_val = values.iter().copied().max().unwrap_or(0);
    let b = (32 - max_val.leading_zeros()).max(1) as usize;
    let words = n.div_ceil(64);
    let mut planes = vec![vec![0u64; words]; b];
    for (i, &v) in values.iter().enumerate() {
        for (bit, plane) in planes.iter_mut().enumerate() {
            plane[i / 64] |= u64::from((v >> bit) & 1) << (i % 64);
        }
    }
    let expected_per_scan = values.iter().filter(|&&v| v > THRESHOLD).count() as u64;
    Input {
        values,
        fixed_col: fixed::BitSliced { planes, words },
        threshold: THRESHOLD,
        expected_per_scan,
    }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut total = 0u64;
    for _ in 0..SCANS {
        total += buggy::scan_count(&input.values, input.threshold);
    }
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected_per_scan * SCANS as u64);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut total = 0u64;
    for _ in 0..SCANS {
        total += fixed::gt_popcount(&input.fixed_col, input.threshold);
    }
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected_per_scan * SCANS as u64);
}
"#
        .to_string(),
    }
}

/// Bench spec for `serialize-once-byte-buffer-store` (Batch 86, Research
/// 100 — the Cloudflare Big Pineapple distill): structured per-record
/// store (2 heap allocations per record) vs ONE length-prefixed byte
/// buffer + u32 offset table (2 allocations TOTAL). The insert path is
/// the measured surface (the source's near-isolated +13% insert); the
/// read guards touch every record so neither arm can be dead-coded.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn serialize_once_byte_buffer_store_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub struct Entry {
        pub name: Box<str>,
        pub data: Vec<u8>,
    }
    pub fn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> Vec<Entry> {
        names
            .iter()
            .zip(payloads)
            .map(|(n, p)| Entry { name: n.clone(), data: p.clone() })
            .collect()
    }
}

mod fixed {
    pub fn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> (Box<[u8]>, Vec<u32>) {
        let mut total = 4usize;
        for (n, p) in names.iter().zip(payloads) {
            total += 4 + n.len() + p.len();
        }
        let mut buf = Vec::with_capacity(total);
        let mut offsets = Vec::with_capacity(names.len());
        buf.extend_from_slice(&(names.len() as u32).to_le_bytes());
        for (n, p) in names.iter().zip(payloads) {
            offsets.push(buf.len() as u32);
            buf.extend_from_slice(&(n.len() as u16).to_le_bytes());
            buf.extend_from_slice(n.as_bytes());
            buf.extend_from_slice(&(p.len() as u16).to_le_bytes());
            buf.extend_from_slice(p);
        }
        (buf.into_boxed_slice(), offsets)
    }
}

pub struct Input {
    names: Vec<Box<str>>,
    payloads: Vec<Vec<u8>>,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut names = Vec::with_capacity(n);
    let mut payloads = Vec::with_capacity(n);
    for _ in 0..n {
        let name_len = 4 + lcg(&mut rng) as usize % 28; // 4..32 B names
        let mut name = String::with_capacity(name_len);
        for _ in 0..name_len {
            name.push((b'a' + lcg(&mut rng) as u8 % 26) as char);
        }
        let payload_len = 4 + lcg(&mut rng) as usize % 36; // 4..40 B payloads
        let payload: Vec<u8> = (0..payload_len).map(|_| lcg(&mut rng) as u8).collect();
        names.push(name.into_boxed_str());
        payloads.push(payload);
    }
    Input { names, payloads }
}

pub fn run_buggy(input: &Input) {
    let store = buggy::build(&input.names, &input.payloads);
    let mut total = 0usize;
    for e in &store {
        total += e.name.len() + e.data.len();
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let (buf, offsets) = fixed::build(&input.names, &input.payloads);
    let mut total = 0usize;
    for &off in &offsets {
        let o = off as usize;
        let nl = u16::from_le_bytes([buf[o], buf[o + 1]]) as usize;
        total += nl;
    }
    std::hint::black_box(total + buf.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `context-equal-field-elide` (Batch 86, Research 100):
/// owner stored on EVERY record vs Option<Box<str>> elided to None when the
/// owner equals the record's lookup context (the cache key) — resolved at
/// read via `as_deref().unwrap_or(context`). ~80% context-equal mix (the
/// source's DNS-owner distribution); the alloc axis is the headline
/// (n → ~0.2n owner allocations); the fixed arm pays the insert-time
/// equality compare (the mechanism's honest cost).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn context_equal_field_elide_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub struct Answer {
        pub ttl: u32,
        pub owner: Box<str>,
    }
    pub fn build(owners: &[Box<str>], ttls: &[u32]) -> Vec<Answer> {
        owners
            .iter()
            .zip(ttls)
            .map(|(o, &t)| Answer { ttl: t, owner: o.clone() })
            .collect()
    }
}

mod fixed {
    pub struct Answer {
        pub ttl: u32,
        pub owner: Option<Box<str>>,
    }
    pub fn build_elided(
        contexts: &[Box<str>],
        owners: &[Box<str>],
        ttls: &[u32],
    ) -> Vec<Answer> {
        contexts
            .iter()
            .zip(owners)
            .zip(ttls)
            .map(|((c, o), &t)| Answer {
                ttl: t,
                owner: if o.as_ref() == c.as_ref() { None } else { Some(o.clone()) },
            })
            .collect()
    }
}

pub struct Input {
    contexts: Vec<Box<str>>, // the lookup key each record belongs to
    owners: Vec<Box<str>>,   // ~80% identical to the context
    ttls: Vec<u32>,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut contexts = Vec::with_capacity(n);
    let mut owners = Vec::with_capacity(n);
    let mut ttls = Vec::with_capacity(n);
    for _ in 0..n {
        let ctx_len = 8 + lcg(&mut rng) as usize % 17; // 8..25 B names
        let mut ctx = String::with_capacity(ctx_len);
        for _ in 0..ctx_len {
            ctx.push((b'a' + lcg(&mut rng) as u8 % 26) as char);
        }
        let ctx: Box<str> = ctx.into_boxed_str();
        let owner: Box<str> = if lcg(&mut rng) % 5 != 0 {
            ctx.clone() // ~80% context-equal (the elision regime)
        } else {
            let mut o = String::with_capacity(ctx_len + 4);
            o.push_str(&ctx);
            o.push_str(".x"); // a genuinely different chain target
            o.into_boxed_str()
        };
        contexts.push(ctx);
        owners.push(owner);
        ttls.push(lcg(&mut rng) as u32 | 1);
    }
    Input { contexts, owners, ttls }
}

pub fn run_buggy(input: &Input) {
    let store = buggy::build(&input.owners, &input.ttls);
    let mut total = 0usize;
    for a in &store {
        total += a.owner.len();
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let store = fixed::build_elided(&input.contexts, &input.owners, &input.ttls);
    let mut total = 0usize;
    for (a, c) in store.iter().zip(&input.contexts) {
        total += a.owner.as_deref().unwrap_or(c).len();
    }
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-before-expensive-transform` (Batch 91,
/// Research 106 — the Rustdoc filter-first distill): ~6% keep-rate, an
/// opaque `#[inline(never)]` render whose digest allocation ESCAPES into
/// the collected Vec (LLVM cannot elide it) + a full body fold — the
/// discarded work is real on both axes. G1: both orders collect the
/// IDENTICAL page Vec (order included).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn filter_before_expensive_transform_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 30_000, 300_000],
        harness: r#"
pub struct Doc {
    pub published: bool,
    pub body: [u64; 8],
}

pub struct Page {
    pub published: bool,
    pub digest: Box<[u8]>,
}

#[inline(never)]
pub fn render_page(doc: &Doc) -> Page {
    let mut acc: u64 = 0;
    for &v in &doc.body {
        acc = acc.wrapping_mul(0x1_0000_0001_b3).wrapping_add(v);
    }
    // the digest allocation ESCAPES into the collected Vec - LLVM
    // cannot elide it, the discarded work is real
    let digest = vec![
        (acc & 0xFF) as u8,
        ((acc >> 8) & 0xFF) as u8,
        ((acc >> 16) & 0xFF) as u8,
        ((acc >> 24) & 0xFF) as u8,
    ]
    .into_boxed_slice();
    Page { published: doc.published, digest }
}

mod buggy {
    use super::{render_page, Doc, Page};
    pub fn published_pages(docs: &[Doc]) -> Vec<Page> {
        docs.iter().map(render_page).filter(|p| p.published).collect()
    }
}

mod fixed {
    use super::{render_page, Doc, Page};
    pub fn published_pages(docs: &[Doc]) -> Vec<Page> {
        docs.iter().filter(|d| d.published).map(render_page).collect()
    }
}

pub struct Input {
    docs: Vec<Doc>,
}

pub fn make_input(n: usize) -> Input {
    let docs = (0..n)
        .map(|i| {
            let seed = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            Doc {
                published: i % 16 == 0, // ~6.25% keep-rate
                body: [seed; 8],
            }
        })
        .collect();
    Input { docs }
}

pub fn run_buggy(input: &Input) {
    let pages = buggy::published_pages(&input.docs);
    let mut touch = pages.len() as u64;
    if let Some(first) = pages.first() {
        touch += u64::from(first.digest[0]);
    }
    std::hint::black_box(touch);
}

pub fn run_fixed(input: &Input) {
    let pages = fixed::published_pages(&input.docs);
    let mut touch = pages.len() as u64;
    if let Some(first) = pages.first() {
        touch += u64::from(first.digest[0]);
    }
    std::hint::black_box(touch);
}
"#
        .to_string(),
    }
}

/// Bench spec for `decision-projection-before-construction` (Batch 91,
/// Research 106 — the Rustdoc head-before-param_env distill): the buggy
/// predicate canonicalizes EVERY item (fresh Vec + sort + dedup behind an
/// `#[inline(never)]` boundary) to read a field the raw item already
/// determines; the fixed predicate computes the invariant min directly.
/// ~1.6% pass-rate. The invariance precondition + the canonicalize-call
/// axis are pinned in the corpus gate (every item vs zero calls).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn decision_projection_before_construction_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 30_000, 300_000],
        harness: r#"
pub struct Item {
    pub tags: [u32; 8],
}

pub struct Canonical {
    pub primary: u32,
    pub tags: Vec<u32>,
}

#[inline(never)]
pub fn canonicalize(item: &Item) -> Canonical {
    let mut tags: Vec<u32> = item.tags.to_vec(); // the construction alloc
    tags.sort_unstable();
    tags.dedup();
    Canonical { primary: tags[0], tags }
}

pub fn raw_primary(item: &Item) -> u32 {
    let mut m = u32::MAX;
    for &t in &item.tags {
        m = m.min(t);
    }
    m
}

mod buggy {
    use super::{canonicalize, Item};
    pub fn count_hot(items: &[Item], hot: u32) -> usize {
        items.iter().filter(|i| canonicalize(i).primary == hot).count()
    }
}

mod fixed {
    use super::{raw_primary, Item};
    pub fn count_hot(items: &[Item], hot: u32) -> usize {
        items.iter().filter(|i| raw_primary(i) == hot).count()
    }
}

pub struct Input {
    items: Vec<Item>,
    hot: u32,
}

pub fn make_input(n: usize) -> Input {
    let items = (0..n)
        .map(|i| {
            let base = (i % 64) as u32;
            let mut tags = [0u32; 8];
            for (k, t) in tags.iter_mut().enumerate() {
                *t = base * 8 + (k as u32) * 31; // min == base * 8, no wrap
            }
            Item { tags }
        })
        .collect();
    let hot = 128; // base == 16 -> ~1.56% pass-rate
    Input { items, hot }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_hot(&input.items, input.hot));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_hot(&input.items, input.hot));
}
"#
        .to_string(),
    }
}

/// Bench spec for `mir-destprop-branch-site-merge` (Batch 99, Research 119
/// — the wasmi v2.0 blog distill): the `DestProp` shape — both arms load a
/// KNOWN fn item, converge into one call (cmov + single indirect tail-jump,
/// asm-verified) vs two distinct direct tail-jump sites. 50/50 mixed
/// dispatch stream (LCG), sizes on the dispatch-count axis. G1 output
/// identity is pinned in `batch99_split_sites_preserve_output_and_dispatch_sequence`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn mir_destprop_branch_site_merge_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
pub type Handler = fn(&mut u64, u64);

#[inline(never)]
pub fn handle_a(s: &mut u64, x: u64) {
    *s = s.wrapping_add(x).wrapping_add(0x9E37);
}

#[inline(never)]
pub fn handle_b(s: &mut u64, x: u64) {
    *s = s.wrapping_mul(31).wrapping_add(x);
}

// BUGGY: both arms load a KNOWN fn item, the pointers converge (phi) --
// cmov of the two code addresses + ONE indirect tail-jump site.
#[inline(never)]
pub fn step_merged(cond: bool, s: &mut u64, x: u64) {
    let f: Handler = if cond { handle_a } else { handle_b };
    f(s, x);
}

// FIXED: two DISTINCT tail-call sites -- direct jumps, per-site history.
#[inline(never)]
pub fn step_split(cond: bool, s: &mut u64, x: u64) {
    if cond {
        handle_a(s, x);
    } else {
        handle_b(s, x);
    }
}

pub struct Input {
    conds: Vec<u32>,
    xs: Vec<u64>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0x243F_6A88_85A3_08D3u64;
    let conds: Vec<u32> = (0..n)
        .map(|_| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            u32::from(((seed >> 33) as u32) % 256 < 128) // 50/50 mixed stream
        })
        .collect();
    let xs: Vec<u64> = (0..n)
        .map(|i| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) ^ (i as u64)
        })
        .collect();
    Input { conds, xs }
}

pub fn run_buggy(input: &Input) {
    let mut s = 0u64;
    for i in 0..input.conds.len() {
        step_merged(input.conds[i] != 0, &mut s, input.xs[i]);
    }
    std::hint::black_box(s);
}

pub fn run_fixed(input: &Input) {
    let mut s = 0u64;
    for i in 0..input.conds.len() {
        step_split(input.conds[i] != 0, &mut s, input.xs[i]);
    }
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `dispatch-handler-pair-hoist` (Batch 101, Research 119
/// §Parallel-session addendum): the runtime-table dispatch shape — the
/// buggy loop reads `HANDLERS[op & 1]` (indexed fn-pointer load) every
/// dispatch; the fixed loop hoists both entries once and selects between
/// registers. LCG-random 2-op stream (the UNPREDICTABLE regime — the
/// measured win band 1.19–1.23×; structured streams are the Tie band, a
/// different harness, not gated here). G1 output identity is pinned in
/// `batch101_hoist_preserves_output_and_both_handlers_run`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn dispatch_handler_pair_hoist_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
pub type Handler = fn(u64) -> u64;

#[inline(never)]
pub fn op_add(x: u64) -> u64 {
    x.wrapping_add(0x9E37)
}

#[inline(never)]
pub fn op_mul(x: u64) -> u64 {
    x.wrapping_mul(31)
}

pub static HANDLERS: [Handler; 2] = [op_add, op_mul];

// BUGGY: indexed fn-pointer load on EVERY dispatch (aarch64: the
// indirect call keeps an in-loop ldr on its critical path).
#[inline(never)]
pub fn run_table(ops: &[u32], xs: &[u64]) -> u64 {
    let mut acc = 0u64;
    for (i, &op) in ops.iter().enumerate() {
        let f = HANDLERS[(op & 1) as usize];
        acc = f(acc ^ xs[i]);
    }
    acc
}

// FIXED: both entries hoisted once — register-resident select in-loop.
#[inline(never)]
pub fn run_hoisted(ops: &[u32], xs: &[u64]) -> u64 {
    let f0 = HANDLERS[0];
    let f1 = HANDLERS[1];
    let mut acc = 0u64;
    for (i, &op) in ops.iter().enumerate() {
        let x = acc ^ xs[i];
        acc = if op & 1 == 0 { f0(x) } else { f1(x) };
    }
    acc
}

pub struct Input {
    ops: Vec<u32>,
    xs: Vec<u64>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0x243F_6A88_85A3_08D3u64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed >> 33
    };
    let ops: Vec<u32> = (0..n).map(|_| (next() % 2) as u32).collect();
    let xs: Vec<u64> = (0..n).map(|i| next() ^ (i as u64)).collect();
    Input { ops, xs }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(run_table(&input.ops, &input.xs));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(run_hoisted(&input.ops, &input.xs));
}
"#
        .to_string(),
    }
}
