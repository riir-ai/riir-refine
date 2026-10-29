//! Entries slice 137 — Batch 304 (Plan 309 / Research 348): the upstream
//! llamacpp + mlx window. Sources: ggml-org/llama.cpp @ 9b8446b91 + 4d71b42fd
//! + c3df4b7e2 + cf24317be (MIT) — the Metal `mul_mm` store-offset widening
//!   window, the concat one-row rewrite, the `IQ4_NL` midpoint rank, and the
//!   control-vector / M-RoPE bounds repairs — and ml-explore/mlx @ b996a43fa7
//!   (MIT) — the grid-slice dispatch helper. Six rules across two slices;
//!   every quote re-verified byte-for-byte at the pin (plan 309 T0,
//!   2026-10-29). The fixtures are ORIGINAL minimal Rust reproductions —
//!   nothing transcribed from the sources, no string literals in any fixture;
//!   all twelve compile standalone (rustc --edition 2024) and their tests ran
//!   green before landing. All six rules are `NotBenchable`
//!   (work-distribution, cache-line, branch-elimination, bounds, and
//!   dispatch-ceiling axes — no Pareto bench in this corpus hosts them; the
//!   Batch-136 domain-reason class). Dedup dispositions recorded in the batch
//!   report: the #30795 `moe_reduce` dst-aliasing candidate is ALREADY the
//!   founding commit of fused-read-write-dst-input-overlap-refusal (`gpu_109`,
//!   B254) — verify-and-record, not re-authored; the three pre-named A7
//!   rules/faces verified landed (`gpu_108` x2, B253; the #30804
//!   SIZE-PLACED-CUT rider on identity-keyed-graph-retention-stable-arena,
//!   `gpu_08`, B274); the 9b8446b widening faces land as a count-neutral rider
//!   text on widen-index-multiply-to-64bit (`gpu_02`) — staged for the
//!   coordinator (this session may touch only this file).

use super::KernelRule;

#[allow(clippy::too_many_lines)]
fn slice_llamacpp_b304() -> Vec<KernelRule> {
    vec![
        // ── Rule 1 (category E — parallelism/work distribution): the concat
        //    rewrite splits copy work over the FLAT ROW space; the old form
        //    split over i2, which is often a singleton, serializing every
        //    row onto one thread. ──
        KernelRule {
            rule_id: "concat-row-space-thread-split-not-singleton-outer-dim".to_string(),
            gpu_concept: "MECHANISM (concat work split over the flat row space, never a singleton outer dim; category E — parallelism/work distribution): the concat op split its copy over the OUTER dim i2 ('for (int i2 = ith; i2 < ne02; i2 += nth)' — the old per-type twins carried 'TODO: smarter multi-theading' on exactly these loops), so when that dim is a singleton — ne02 == 1, the common concat shape — only thread 0 ever enters the loop and one thread memcpys every row. The rewrite, verbatim (commit message): \"Rows are split across threads instead of i2 alone, which is often 1\" — 'const int64_t nr = ne1*ne2*ne3;' + 'const int64_t dr = (nr + params->nth - 1)/params->nth;' + 'const int64_t ir0 = dr*params->ith;' + 'const int64_t ir1 = MIN(ir0 + dr, nr);' with the flat index decomposed back per row ('int64_t i[4] = { 0, ir % ne1, (ir/ne1) % ne2, ir/(ne1*ne2) };'). And the row body collapsed to one copy per row for every type, verbatim: \"Every dst row comes from src0 or src1, or for dim 0 from the row of src0 followed by the row of src1, so a single kernel covers every type: a dense source row is one memcpy, quantized types included\" — 'if (src->nb[0] == es) { ... memcpy(y + r*nby, x + r*nbx, row_size); }' (ggml_concat_copy_rows), the four per-element type twins (concat_any/i8/f16/f32, 176 lines) deleted for one type-blind kernel. THE LAW: split a copy kernel's work over the FLAT ROW space (rows = the product of the trailing dims), never over an outer dim that can be a singleton — the parallel unit is the row, and a dense row is one memcpy whatever the dtype. REGIME: CPU-side multi-threaded row-copy ops (concat, pad, any binary tensor op with per-row sources); outer dims of size 1. ANTI-REGIME: a genuinely multi-valued outer dim with tiny rows (the i2 split was fine there — the flat split degenerates to it, never worse); rows too small for a thread (the row floor is one memcpy). KINSHIP (do not double-count): row-parallelism-ladder-split-vs-pack (entries_gpu_22) owns the DEVICE warps-per-row geometry ladder — THIS is the host thread-pool row split; wide-row-copy-tile-rows-across-workgroups (entries_gpu_11) owns one too-wide row tiled ACROSS workgroups for DRAM saturation — a device-concurrency axis, the opposite defect shape (its rows are huge; here the thread count is capped at 1); pad-row-classification-interior-memcpy (B240) owns pad ops classifying rows into memset/memcpy classes — the dense-row-memcpy face cites it, the work-DISTRIBUTION axis is THIS. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'singleton|flat row|thread split|rows are split' -> zero owners; 'concat' -> same-input-gemv-weight-concat-at-upload (gpu_07, weight concat at upload), multi-head-out-proj-consumes-head-concat (gpu_06, the projection over the head concat), incremental-multi-part-hash-no-concat (hashing) — no copy-work-distribution owner; 'i2 split|outer dim' -> zero. NotBenchable here (work-distribution axis — the gain is thread utilization on the host pool, no in-source number ships; the fixture pins utilization 1/nth -> nth and the exact contiguous row partition). Source: ggml-org/llama.cpp @ 4d71b42fd87db24b8b7c808c151b713aa4459cfd (MIT, #31179, Pascal) — ggml/src/ggml-cpu/ops.cpp:1916-1949 (the helpers; the old i2-split twins deleted) + :1971-1975 (the flat row split) + :1977-1981 (the tile bound), quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348. RIDER (Batch 307 / Plan 312 — ggml-org/llama.cpp @ 6d19453 #31075, ggml/src/ggml-cpu/ ops + ggml-cpu-impl.h, 2026-10-29 — the SAME-REPO SECOND INSTANCE, METAL side, with the dispatch-loop half): the contiguous unary path and the row-broadcast binary path dispatched ONE THREAD PER THREADGROUP (\"ggml_metal_encoder_dispatch_threadgroups(enc, n, 1, 1, 1, 1, 1)\" — threadgroups=(n,1,1), threads=(1,1,1)); the fix widens to up to 256 threads per threadgroup with a bounds-checked tail: \"const int nth = MIN(256, n);\" / \"ggml_metal_encoder_dispatch_threadgroups(enc, (n + nth - 1)/nth, 1, 1, nth, 1, 1);\" — and reads the REAL ceiling from the pipeline (\"const int nth_max = MIN(256, ggml_metal_pipeline_max_theads_per_threadgroup(pipeline));\") instead of assuming the 256. The GLU rows split ACROSS threadgroups (\"a single-row decode was ONE threadgroup for the whole op\") — the row axis becomes the thread axis. The instance confirms the law on a third backend and adds the dispatch-arithmetic clause: the grid re-derives as ceil(n/nth) and the ceiling is QUERIED from the compiled pipeline, never assumed (the dispatch loop pays (n + nth - 1)/nth threadgroups; the tail bounds-check is in-kernel). Quotes byte-verified at the fetched .patch pin (plan 312 T0, 2026-10-29).".to_string(),
            ref_shape: r"#![allow(dead_code)]
// ref_shape (rule: concat-row-space-thread-split-not-singleton-outer-dim):
// the copy splits work over the OUTER dim i2 (i2 = ith; i2 < ne02; i2 += nth).
// When that dim is a singleton -- ne02 == 1, the common concat shape where
// rows or the innermost axis are concatenated -- only thread 0 ever enters
// the loop: the other nth-1 threads idle and one thread memcpys every row.
// Utilization is 1/nth and the wall is the full row count on one thread.

const ROWS: usize = 70; // the rows: ne1 x ne3; i2 has NE02 values
const NE02: usize = 1; // the singleton outer dim
const NTH: usize = 4; // threads

/// the i2-split walk: (threads that copied, rows the busiest thread copied)
fn concat_i2_split() -> (usize, usize) {
    let mut threads_used = 0usize;
    let mut busiest = 0usize;
    for ith in 0..NTH {
        let mut mine = 0usize;
        let mut worked = false;
        let mut i2 = ith;
        while i2 < NE02 {
            worked = true;
            for _i1 in 0..ROWS {
                mine += 1; // one memcpy per row of this i2
            }
            i2 += NTH;
        }
        if worked {
            threads_used += 1;
        }
        busiest = busiest.max(mine);
    }
    (threads_used, busiest)
}

#[test]
fn singleton_outer_dim_serializes_every_row_onto_one_thread() {
    let (threads_used, busiest) = concat_i2_split();
    assert_eq!(threads_used, 1);
    assert_eq!(busiest, ROWS);
    assert_eq!(NTH - threads_used, 3);
}

fn main() {}"
                .to_string(),
            opt_pattern: r"#![allow(dead_code)]
// opt_pattern (rule: concat-row-space-thread-split-not-singleton-outer-dim):
// the work splits over the FLAT ROW space: nr = ne1*ne2*ne3, each thread
// takes dr = ceil(nr/nth) rows (ir0 = dr*ith, ir1 = min(ir0+dr, nr)) and the
// flat index decomposes back per row. A dense source row is ONE memcpy --
// quantized rows included, the packed blocks are contiguous -- so the
// parallel unit is the row, never a possibly-singleton outer dim.

const ROWS: usize = 70;
const NTH: usize = 4;

fn flat_row_split() -> (usize, usize, [usize; NTH]) {
    let dr = (ROWS + NTH - 1) / NTH;
    let mut per_thread = [0usize; NTH];
    let mut threads_used = 0usize;
    for ith in 0..NTH {
        let ir0 = dr * ith;
        let ir1 = usize::min(ir0 + dr, ROWS);
        if ir0 < ir1 {
            threads_used += 1;
        }
        for _ir in ir0..ir1 {
            per_thread[ith] += 1; // one memcpy per row
        }
    }
    (threads_used, dr, per_thread)
}

#[test]
fn every_thread_works_and_rows_partition_exactly() {
    let (threads_used, dr, per_thread) = flat_row_split();
    assert_eq!(threads_used, NTH);
    assert_eq!(dr, ROWS.div_ceil(NTH));
    assert_eq!(per_thread.iter().sum::<usize>(), ROWS);
    assert_eq!(per_thread.iter().copied().max().unwrap(), dr);
    for (ith, count) in per_thread.iter().enumerate() {
        let ir0 = dr * ith;
        let expect = usize::min(dr, ROWS.saturating_sub(ir0));
        assert_eq!(*count, expect);
    }
}

fn main() {}"
                .to_string(),
        },
        // ── Rule 2 (category A — memory access/coalescing): the same rewrite
        //    copies a strided (transposed) source in 32-row tiles with the
        //    ELEMENT index in the outer loop, so a tile's rows read
        //    neighboring addresses. ──
        KernelRule {
            rule_id: "strided-source-row-tile-element-outer-walk".to_string(),
            gpu_concept: "MECHANISM (strided-source copy in row tiles, element-outer walk; category A — memory access/coalescing): the same concat rewrite walks a STRIDED source — a transposed view whose logical rows sit adjacent in memory and whose logical elements sit a full source-row pitch apart — row by row, so every element read jumps nb0 and touches a fresh cache line the walk re-touches only rows later. The fix, verbatim (commit message): \"...and are copied in tiles of 32: for a transposed source the elements are walked in the outer loop, so the rows of a tile read neighboring addresses instead of jumping a full row stride per element.\" In code: '// rows of dst copied together, so that a strided source is read along neighboring rows' + 'static constexpr int64_t GGML_CONCAT_TILE = 32;' + the per-tile bound 'int64_t nt = MIN(MIN(ir1 - ir, ne1 - i[1]), GGML_CONCAT_TILE);' and the element-outer walk 'template <size_t es> static void ggml_concat_copy_strided(...) { for (int64_t i0 = 0; i0 < n; i0++) { for (int64_t r = 0; r < n_rows; r++) { memcpy(y + r*nby + i0*es, x + r*nbx + i0*nb0, es); } } }' — es a compile-time template parameter ('a strided source is copied element by element with the element size known at compile time'), switched at the row helper ('case 1: ... case 2: ... case 4: ... case 8: ... default: GGML_ABORT'). THE LAW: when a copy source is strided along the ELEMENT axis (transposed views: nb0 >> nbx), walk the element index in the OUTER loop over a tile of consecutive destination rows — the tile's row reads then sweep neighboring addresses (16 adjacent rows share a 64-byte line at 4-byte elements) instead of paying one line per element; the tile is also what keeps a dense source on the one-memcpy-per-row path while bounding the strided walk's live row set. REGIME: strided/transposed copy sources (the delta-net conv input concat — the commit adds exactly that perf case: '// delta-net conv input: conv state followed by the transposed qkv of the ubatch'); host caches of tens of lines. ANTI-REGIME: dense sources (nb0 == element size — the row-memcpy path wins outright); an element stride at or under the line size (no line reuse to win); a destination side strided in a conflicting order (the write sweep would pay what the read sweep saves). KINSHIP (do not double-count): wide-row-copy-tile-rows-across-workgroups (entries_gpu_11) tiles a row across DEVICE workgroups for concurrency — THIS walks tiles for host read LOCALITY at fixed parallelism; coalesced-row-read (the seed stub) owns the device coalescing pattern — THIS is its host-cache twin; dequant-once-macrotile-stage-reuse-across-microtiles (entries_gpu_53) amortizes a DECODE across tile uses — THIS amortizes a cache LINE across row reads. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'element outer|neighboring address|row tile|GGML_CONCAT_TILE' -> zero owners; 'transposed' -> the B180 planar-transposed activation plane (cited in gpu_53's kinship prose — an operand LAYOUT axis, not a walk order); 'stride|nb0' -> stride-align-vectorization-key (alignment for vector width) + mirror-ring-store-sliding-window-contiguity (ring layout) — no walk-order owner. NotBenchable here (cache-line locality at host width — the fixture prices an LRU miss model, 16x fewer line fetches on the transposed shape; no in-source cycle number ships, the delta-net perf case was ADDED by the commit without figures). Source: ggml-org/llama.cpp @ 4d71b42fd87db24b8b7c808c151b713aa4459cfd (MIT, #31179) — ggml/src/ggml-cpu/ops.cpp:1918-1949 (GGML_CONCAT_TILE + ggml_concat_copy_strided<es> + ggml_concat_copy_rows) + :1980-1981 (the per-tile bound), quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"#![allow(dead_code)]
// ref_shape (rule: strided-source-row-tile-element-outer-walk): a strided
// (transposed) source is copied ROW-OUTER: for each row, every element is
// read at the element stride nb0 -- a jump of a full source-row pitch per
// element. On a transposed view the logical rows sit adjacent in memory, so
// the walk touches a fresh cache line per element and re-touches the same
// line only many rows later, long after a small cache evicted it.

const R: usize = 64; // logical rows of the source view
const N: usize = 32; // elements per row
const NB0: usize = 2048; // element stride: the transposed view's row pitch
const NBX: usize = 4; // row stride: logically adjacent rows sit adjacent
const LINE: usize = 64; // cache-line bytes
const CACHE_LINES: usize = 4;

/// a deterministic LRU cache over line indices
struct Cache {
    lines: [Option<usize>; CACHE_LINES],
    age: [usize; CACHE_LINES],
    tick: usize,
    misses: usize,
}

impl Cache {
    fn new() -> Self {
        Cache {
            lines: [None; CACHE_LINES],
            age: [0; CACHE_LINES],
            tick: 0,
            misses: 0,
        }
    }
    fn read(&mut self, addr: usize) {
        self.tick += 1;
        let line = addr / LINE;
        if let Some(slot) = self.lines.iter().position(|&l| l == Some(line)) {
            self.age[slot] = self.tick; // hit
        } else {
            self.misses += 1;
            let mut victim = 0usize;
            for i in 1..CACHE_LINES {
                if self.age[i] < self.age[victim] {
                    victim = i;
                }
            }
            self.lines[victim] = Some(line);
            self.age[victim] = self.tick;
        }
    }
}

/// row-outer walk: row r fully, then row r+1 -- an nb0 jump per element
fn copy_row_outer(cache_misses: &mut usize) -> usize {
    let mut c = Cache::new();
    let mut reads = 0usize;
    for r in 0..R {
        for i0 in 0..N {
            c.read(r * NBX + i0 * NB0);
            reads += 1;
        }
    }
    *cache_misses = c.misses;
    reads
}

#[test]
fn row_outer_walk_misses_on_every_read() {
    let mut misses = 0usize;
    let reads = copy_row_outer(&mut misses);
    assert_eq!(reads, R * N);
    // the nb0 jump per element defeats the cache: the same line comes back
    // into play N reads later, far past 4 cached lines
    assert_eq!(misses, reads);
}

fn main() {}"
                .to_string(),
            opt_pattern: r"#![allow(dead_code)]
// opt_pattern (rule: strided-source-row-tile-element-outer-walk): the rows of
// dst are copied together in tiles of GGML_CONCAT_TILE (32), and within a
// tile the ELEMENT index walks in the outer loop: the tile's rows read
// neighboring addresses (a transposed view's adjacent logical rows sit
// adjacent in memory), so each cache line is consumed by 16 consecutive
// reads instead of being re-fetched rows later.

const R: usize = 64;
const N: usize = 32;
const NB0: usize = 2048;
const NBX: usize = 4;
const LINE: usize = 64;
const CACHE_LINES: usize = 4;
const TILE: usize = 32; // GGML_CONCAT_TILE

/// Cache is the ref_shape item, unchanged.
struct Cache {
    lines: [Option<usize>; CACHE_LINES],
    age: [usize; CACHE_LINES],
    tick: usize,
    misses: usize,
}

impl Cache {
    fn new() -> Self {
        Cache {
            lines: [None; CACHE_LINES],
            age: [0; CACHE_LINES],
            tick: 0,
            misses: 0,
        }
    }
    fn read(&mut self, addr: usize) {
        self.tick += 1;
        let line = addr / LINE;
        if let Some(slot) = self.lines.iter().position(|&l| l == Some(line)) {
            self.age[slot] = self.tick;
        } else {
            self.misses += 1;
            let mut victim = 0usize;
            for i in 1..CACHE_LINES {
                if self.age[i] < self.age[victim] {
                    victim = i;
                }
            }
            self.lines[victim] = Some(line);
            self.age[victim] = self.tick;
        }
    }
}

/// copy_row_outer is the ref_shape item, unchanged.
fn copy_row_outer(cache_misses: &mut usize) -> usize {
    let mut c = Cache::new();
    let mut reads = 0usize;
    for r in 0..R {
        for i0 in 0..N {
            c.read(r * NBX + i0 * NB0);
            reads += 1;
        }
    }
    *cache_misses = c.misses;
    reads
}

/// element-outer walk over tiles of TILE consecutive rows
fn copy_tile_element_outer(cache_misses: &mut usize) -> usize {
    let mut c = Cache::new();
    let mut reads = 0usize;
    for i0 in 0..N {
        let mut t0 = 0usize;
        while t0 < R {
            let t1 = usize::min(t0 + TILE, R);
            for r in t0..t1 {
                c.read(r * NBX + i0 * NB0);
                reads += 1;
            }
            t0 += TILE;
        }
    }
    *cache_misses = c.misses;
    reads
}

#[test]
fn element_outer_tiles_read_neighboring_addresses() {
    let mut misses = 0usize;
    let reads = copy_tile_element_outer(&mut misses);
    assert_eq!(reads, R * N);
    // per element index, the tile walk sweeps R adjacent rows: 16 rows share
    // one 64-byte line, so R/16 line fetches per element index
    assert_eq!(misses, N * (R / 16));
    // and 16x fewer misses than the row-outer walk over the same reads
    let mut naive = 0usize;
    assert_eq!(copy_row_outer(&mut naive), reads);
    assert_eq!(naive, misses * 16);
}

fn main() {}"
                .to_string(),
        },
        // ── Rule 3 (category B — compute): IQ4_NL quant creation replaces
        //    the per-lookup binary search with a hoisted midpoint table and
        //    a branch-free rank, reused by every block, lane and encode
        //    pass. ──
        KernelRule {
            rule_id: "codebook-nearest-midpoint-rank-branch-free".to_string(),
            gpu_concept: "MECHANISM (codebook-nearest as a branch-free midpoint rank; category B — compute): the quantization-side nearest-codebook lookup replaced its binary search with a precomputed midpoint table and a rank count, verbatim (ggml-quants.c:28-32): '// index of the nearest kvalues_iq4nl value, with mid[k] = (values[k] + values[k+1])/2' + '// for this table it matches a binary search with the tie rule x - values[k] < values[k+1] - x for every float x' + 'static inline int best_index_iq4nl(const float * GGML_RESTRICT mid, float x) { int l = 15; for (int k = 0; k < 15; ++k) l -= x < mid[k]; return l; }' — the midpoints hoisted ONCE per row quantization ('float mid[15];' + 'for (int k = 0; k < 15; ++k) mid[k] = 0.5f*(values[k] + values[k+1]);' beside the fval copy, ggml-quants.c:4949-4952) and the rank reused at every call site: the initial guess ('Lb[j] = best_index_iq4nl(mid, idt[0]*xb[j]);'), every lane of the multi-try scale search ('float q = fval[best_index_iq4nl(mid, idt[t]*xb[j])];' under the lane precompute 'const int nt = ntry >= 0 ? 2*ntry + 2 : 1;' with the j-outer restructure loading weight[j] once across all lanes), and the final encode passes (:5021, :5035). THE LAW: nearest-neighbor classification over a fixed ordered codebook is a RANK — n-1 comparisons against the precomputed between-entry midpoints, zero data-dependent branches, fixed trip count — replacing a log-n bisection whose trip count and clamps branch per lookup; the swap is exact ONLY where the midpoint comparisons reproduce the search's tie rule on this table (midpoints distinct and monotone; an exact-midpoint x takes the UPPER value on both — 'x - values[k] < values[k+1] - x' is false at equality), which the source asserts per float and the fixture verifies over a census plus every midpoint and clamp. The hoist is the second half: one table build per quantize row, consumed by every block, every lane, and every encode pass. REGIME: fixed small codebooks (16 entries) that quantization loops classify against millions of times per row; consumers wanting a branch-free fixed-latency lookup. ANTI-REGIME: large or unsorted tables (the rank is O(n) per lookup — n over ~32 loses the bisection's O(log n)); tables whose midpoints collide (the tie-equivalence argument dies); a one-shot lookup (no reuse to amortize the build). KINSHIP (do not double-count): branchless-select (the seed stub — a generic select, no classification law); lut-free-bit-spread-vadd4-unpack (entries_gpu_33 — LUT-vs-ALU for BIT expansion, constant-cache economics; THIS is a classification RANK over a tiny hoisted table, the table is the point); packed-total-order-multi-field-argmax (entries_gpu_20 — packed keys for argmax reduction); sign-branch-stable-sigmoid (entries_gpu_03 — branch placement for overflow stability). GREP EVIDENCE (over the corpus, 3+ vocabularies): 'midpoint|best_index' -> zero owners; 'branch-free|branchless' -> the stub + bit-spread + sigmoid — none classifies a codebook; 'binary search' -> op-chain-collapse-membership-binary-search-mask (mlx-serve — a membership MASK via binary search, a different decision, no midpoint rank). NotBenchable here (branch-elimination + hoist axis — the gain is mispredict-free fixed-latency classification plus one table build per row; no in-source cycle number ships; the fixture pins the per-float equivalence census, the zero-branch property, and the weight-load count across lanes). Source: ggml-org/llama.cpp @ c3df4b7e22a1ac38da663552d19e649dd97ca936 (MIT, #31201, Bartowski) — ggml/src/ggml-quants.c:28-34 + :4949-4952 + :4973-4996 + :5021 + :5035, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"#![allow(dead_code)]
// ref_shape (rule: codebook-nearest-midpoint-rank-branch-free): every nearest
// codebook lookup runs a data-dependent binary search over the value table --
// two clamp compares, then a bisection loop whose trip count depends on x --
// and the multi-try scale search re-runs that search per candidate scale per
// element, re-reading the weight for every lane.

const B: usize = 16; // codebook entries

/// the fixture's monotone 16-entry codebook (all distinct, ascending)
fn codebook() -> [f32; B] {
    let mut v = [0.0f32; B];
    for (k, slot) in v.iter_mut().enumerate() {
        *slot = (k as f32 - 8.0) * 1.5;
    }
    v
}

/// the branchy search: clamp compares + a bisection loop + the tie rule
fn best_index_search(val: &[f32; B], x: f32, branches: &mut u32) -> usize {
    *branches += 1;
    if x <= val[0] {
        return 0;
    }
    *branches += 1;
    if x >= val[B - 1] {
        return B - 1;
    }
    let (mut ml, mut mu) = (0usize, B - 1);
    while mu - ml > 1 {
        *branches += 1;
        let mav = (ml + mu) / 2;
        if x < val[mav] {
            mu = mav;
        } else {
            ml = mav;
        }
    }
    *branches += 1;
    if x - val[mu - 1] < val[mu] - x {
        mu - 1
    } else {
        mu
    }
}

/// deterministic census draw spanning past both clamps
struct Lcg(u64);
impl Lcg {
    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let m = ((self.0 >> 40) & 0xFFFF) as f32;
        (m / 65535.0 - 0.5) * 48.0
    }
}

/// the multi-try scale search, lane-outer: the weight re-read per lane
fn scale_search_naive(
    xb: &[f32],
    weight: &[f32],
    idt: &[f32],
    val: &[f32; B],
    weight_loads: &mut u32,
    branches: &mut u32,
) {
    for t in 0..idt.len() {
        for j in 0..xb.len() {
            let _w = weight[j];
            *weight_loads += 1;
            let al = idt[t] * xb[j];
            let _ = best_index_search(val, al, branches);
        }
    }
}

#[test]
fn the_search_branches_per_lookup_and_rereads_weights_per_lane() {
    let val = codebook();
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
    let mut branches = 0u32;
    let mut lookups = 0u32;
    for _ in 0..256 {
        let x = rng.next_f32();
        let _ = best_index_search(&val, x, &mut branches);
        lookups += 1;
    }
    assert!(branches >= 3 * lookups);
    let (lanes, elements) = (5usize, 32usize);
    let xb: Vec<f32> = (0..elements).map(|j| (j as f32) * 0.25 - 4.0).collect();
    let weight: Vec<f32> = (0..elements).map(|j| 1.0 + (j as f32) * 0.125).collect();
    let idt: Vec<f32> = (0..lanes).map(|t| 0.125 * (t as f32 + 1.0)).collect();
    let mut loads = 0u32;
    let mut b2 = 0u32;
    scale_search_naive(&xb, &weight, &idt, &val, &mut loads, &mut b2);
    assert_eq!(loads, (lanes * elements) as u32);
}

fn main() {}"
                .to_string(),
            opt_pattern: r"#![allow(dead_code)]
// opt_pattern (rule: codebook-nearest-midpoint-rank-branch-free): the
// midpoints between adjacent codebook entries are computed ONCE, and the
// nearest index is the RANK of x among them: l = n-1 minus the count of
// midpoints above x -- zero data-dependent branches, a fixed n-1 compare
// chain. For a monotone table of distinct values the rank reproduces the
// search's nearest-tie rule for every float (an exact midpoint takes the
// upper value on both). The scale-search lanes restructure j-outer so the
// weight loads once per element across all lanes, and the table builds once
// per quantize row, consumed by every block and every lane.

const B: usize = 16;

fn codebook() -> [f32; B] {
    let mut v = [0.0f32; B];
    for (k, slot) in v.iter_mut().enumerate() {
        *slot = (k as f32 - 8.0) * 1.5;
    }
    v
}

/// the midpoints, hoisted: mid[k] = (v[k] + v[k+1]) / 2
fn midpoints(val: &[f32; B]) -> [f32; B - 1] {
    let mut mid = [0.0f32; B - 1];
    for k in 0..B - 1 {
        mid[k] = 0.5 * (val[k] + val[k + 1]);
    }
    mid
}

/// the rank: zero branches, exactly n-1 compares
fn best_index_rank(mid: &[f32; B - 1], x: f32, branches: &mut u32) -> usize {
    let mut l = B - 1usize;
    for k in 0..B - 1 {
        l -= usize::from(x < mid[k]); // a compare feeding a subtract
    }
    let _ = branches; // never incremented: the chain is branch-free
    l
}

/// best_index_search is the ref_shape item, unchanged.
fn best_index_search(val: &[f32; B], x: f32, branches: &mut u32) -> usize {
    *branches += 1;
    if x <= val[0] {
        return 0;
    }
    *branches += 1;
    if x >= val[B - 1] {
        return B - 1;
    }
    let (mut ml, mut mu) = (0usize, B - 1);
    while mu - ml > 1 {
        *branches += 1;
        let mav = (ml + mu) / 2;
        if x < val[mav] {
            mu = mav;
        } else {
            ml = mav;
        }
    }
    *branches += 1;
    if x - val[mu - 1] < val[mu] - x {
        mu - 1
    } else {
        mu
    }
}

/// deterministic census draw spanning past both clamps
struct Lcg(u64);
impl Lcg {
    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let m = ((self.0 >> 40) & 0xFFFF) as f32;
        (m / 65535.0 - 0.5) * 48.0
    }
}

/// the scale search, j-outer: one weight load shared by every lane
fn scale_search_rank(
    xb: &[f32],
    weight: &[f32],
    idt: &[f32],
    mid: &[f32; B - 1],
    weight_loads: &mut u32,
    branches: &mut u32,
) {
    for j in 0..xb.len() {
        let _w = weight[j];
        *weight_loads += 1;
        for t in 0..idt.len() {
            let al = idt[t] * xb[j];
            let _ = best_index_rank(mid, al, branches);
        }
    }
}

#[test]
fn the_rank_matches_the_search_over_the_census_and_every_midpoint() {
    let val = codebook();
    let mid = midpoints(&val);
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
    for _ in 0..4096 {
        let x = rng.next_f32();
        let mut b = 0u32;
        let s = best_index_search(&val, x, &mut b);
        let mut b2 = 0u32;
        let r = best_index_rank(&mid, x, &mut b2);
        assert_eq!(s, r);
    }
    for k in 0..B {
        let mut b = 0u32;
        let s = best_index_search(&val, val[k], &mut b);
        let mut b2 = 0u32;
        let r = best_index_rank(&mid, val[k], &mut b2);
        assert_eq!(s, r);
        if k < B - 1 {
            // an exact midpoint takes the UPPER value on both
            let m = mid[k];
            let mut b3 = 0u32;
            let s_mid = best_index_search(&val, m, &mut b3);
            let mut b4 = 0u32;
            let r_mid = best_index_rank(&mid, m, &mut b4);
            assert_eq!(s_mid, k + 1);
            assert_eq!(r_mid, k + 1);
        }
    }
    // the clamps
    for x in [-100.0f32, 100.0] {
        let mut b = 0u32;
        let s = best_index_search(&val, x, &mut b);
        let mut b2 = 0u32;
        let r = best_index_rank(&mid, x, &mut b2);
        assert_eq!(s, r);
    }
}

#[test]
fn the_rank_chain_is_branch_free_and_the_table_builds_once() {
    let val = codebook();
    let mid = midpoints(&val);
    let mut rng = Lcg(0x2545_F491_4F6C_DD1D);
    let mut branches = 0u32;
    for _ in 0..512 {
        let x = rng.next_f32();
        let _ = best_index_rank(&mid, x, &mut branches);
    }
    assert_eq!(branches, 0);
    let (lanes, elements) = (5usize, 32usize);
    let xb: Vec<f32> = (0..elements).map(|j| (j as f32) * 0.25 - 4.0).collect();
    let weight: Vec<f32> = (0..elements).map(|j| 1.0 + (j as f32) * 0.125).collect();
    let idt: Vec<f32> = (0..lanes).map(|t| 0.125 * (t as f32 + 1.0)).collect();
    let mut loads = 0u32;
    let mut b2 = 0u32;
    scale_search_rank(&xb, &weight, &idt, &mid, &mut loads, &mut b2);
    // one load per element across all lanes: 32, not lanes * elements
    assert_eq!(loads, elements as u32);
    assert_eq!(b2, 0);
}

fn main() {}"
                .to_string(),
        },
        // ── Rule 4 (category D — admission correctness): the control-vector
        //    loader range-checks its PARSED layer index at the load boundary
        //    and computes buffer size and write offset in size_t. ──
        KernelRule {
            rule_id: "parsed-artifact-index-range-check-wide-offsets-load-boundary".to_string(),
            gpu_concept: "MECHANISM (parsed artifact index range-checked at the load boundary, offsets in the address width; category D — dispatch/admission correctness): a control-vector loader parsed each direction tensor's layer index from the artifact's tensor names with no upper bound beside the zero check, verbatim (commit message): \"A direction tensor layer index is parsed from the tensor name with no upper bound, and the buffer size and write offset are computed in int. A large index overflows both, so the accumulation writes before the start of the buffer.\" The repair is TWO halves at one boundary, verbatim: \"Pass the model layer count to the loader, reject indices above it like the existing zero index check, and compute the sizes in size_t.\" In code: the check sits beside the existing refusals ('else if (layer_idx > n_layer) { COM_ERR(\"direction tensor layer index %d exceeds the model layer count %d in %s\\n\", layer_idx, n_layer, load_info.fname.c_str()); ... }' — common.cpp:2039-2042, right after the layer_idx == 0 arm) and both computations widen ('result.data.resize(std::max(result.data.size(), (size_t) result.n_embd * layer_idx), 0.0f);' :2066 + 'float * dst = result.data.data() + (size_t) result.n_embd * (layer_idx - 1);  // layer 1 at [0]' :2069 — the cast BEFORE the multiply), with the contract on the signature ('// Direction layer indices must be in [1, n_layer].' — common.h:1240). The founding arithmetic: at n_embd 4096 and layer index 1048576 the i32 resize length wraps to exactly 0 while '4096 * (layer_idx - 1)' wraps to -4096 — the buffer never grows and the accumulation writes 4096 floats BEFORE its start. THE LAW: an index parsed from an artifact (tensor names, file headers, config keys) is untrusted input — range-check it at the load boundary against the capacity of the thing it indexes (beside the existing refusals, never downstream) AND compute its offsets in the address width; either half alone fails (the check without the width still overflows on in-range-but-huge artifacts; the width without the check still writes garbage layers into a correctly-sized buffer). REGIME: loaders over foreign artifacts whose indices select write slots (control vectors, lora adapters, merged shards, tensor-name-keyed maps). ANTI-REGIME: indices the loader itself allocated (internally consistent by construction); a boundary that has already validated the whole file against a schema. KINSHIP (do not double-count): widen-index-multiply-to-64bit (entries_gpu_02) owns the WIDTH axis for flat address arithmetic on device buffers — THIS owns the VALIDATION half (the parsed index against the model's own capacity) at a CPU load boundary and cites that rule for the width discipline; fail-closed-multi-resource-commit-transaction (entries_gpu_08) validates its OWN row indices before a mutation — THIS validates a FOREIGN artifact's parsed indices at ingestion; moe-duplicate-id-compact-prepass (B224) is the same untrusted-input class on the DEVICE side (duplicate expert ids producing wrong/overflowing writes) — cite as the device twin. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'range check|layer index|n_layer' -> zero owners (the popcorn layer_idx hit parameterizes depth-decay dispatch — a legitimate use, not a bound); 'negative offset|before the buffer|unbounded index' -> zero; 'parsed' -> B224 only, cited. NotBenchable here (correctness hazard — a 2^31-class index is adversarial input, not a workload; the fixture pins the wrap arithmetic exactly (the resize wraps to 0 while the offset wraps to -4096), the loud refusal, and naive/fixed agreement on non-overflowing indices). Source: ggml-org/llama.cpp @ cf24317bea431b807dc93319d74c61bed6e20aff (MIT, #31336, Pascal) — common/common.cpp:2036-2043 + :2066 + :2069 + common/common.h:1239-1242, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"#![allow(dead_code)]
// ref_shape (rule: parsed-artifact-index-range-check-wide-offsets-load-boundary):
// a direction-tensor layer index is parsed from the artifact's tensor names
// with NO upper bound beside the zero check, and both the buffer size and the
// write offset are computed in 32-bit int. At n_embd 4096 and layer index
// 1048576 the resize length wraps to exactly zero while the offset wraps
// NEGATIVE: the buffer never grows and the accumulation writes before its
// start.

const N_EMBD: i32 = 4096;

/// what the unbounded loader computes for one parsed index
fn compute_naive(layer_idx: i32) -> (usize, isize) {
    // the resize length, in int: static_cast<size_t>(n_embd * layer_idx)
    let want = N_EMBD.wrapping_mul(layer_idx);
    let resize_len = (want as i64) as usize; // a negative wrap sign-extends huge
    // the write offset, in int: n_embd * (layer_idx - 1)
    let off = N_EMBD.wrapping_mul(layer_idx.wrapping_sub(1));
    let dst = off as isize;
    (resize_len, dst)
}

#[test]
fn the_wrapped_offset_points_before_the_buffer() {
    // 4096 * 1048576 == 2^32 exactly: the i32 resize length wraps to zero,
    // and 4096 * 1048575 == 2^32 - 4096 wraps to -4096 as int
    let (resize_len, dst) = compute_naive(1048576);
    assert_eq!(resize_len, 0);
    assert_eq!(dst, -4096);
    assert!(dst < 0);
    // a smaller overflow also poisons the length: 4096 * 524290 wraps
    // negative, sign-extends to a huge size_t, and the resize dies first
    let (huge_len, _) = compute_naive(524290);
    assert!(huge_len > (1usize << 62));
}

fn main() {}"
                .to_string(),
            opt_pattern: r"#![allow(dead_code)]
// opt_pattern (rule: parsed-artifact-index-range-check-wide-offsets-load-boundary):
// the parsed index is range-checked AT THE LOAD BOUNDARY beside the existing
// zero check -- reject indices above the model's layer count loud -- and the
// buffer size and write offset are computed in size_t (usize), so an in-range
// index always writes inside a buffer that grew to cover it.

const N_EMBD: i32 = 4096;

#[derive(Debug, PartialEq)]
enum Reject {
    ZeroIndex,
    IndexAboveLayers,
}

/// compute_naive is the ref_shape item, unchanged.
fn compute_naive(layer_idx: i32) -> (usize, isize) {
    let want = N_EMBD.wrapping_mul(layer_idx);
    let resize_len = (want as i64) as usize;
    let off = N_EMBD.wrapping_mul(layer_idx.wrapping_sub(1));
    let dst = off as isize;
    (resize_len, dst)
}

fn compute_fixed(layer_idx: i32, n_layer: i32) -> Result<(usize, usize, usize), Reject> {
    if layer_idx == 0 {
        return Err(Reject::ZeroIndex);
    }
    if layer_idx > n_layer {
        return Err(Reject::IndexAboveLayers); // beside the zero check
    }
    let len = (N_EMBD as usize) * (layer_idx as usize); // size_t arithmetic
    let dst = (N_EMBD as usize) * ((layer_idx - 1) as usize); // layer 1 at 0
    Ok((len, dst, N_EMBD as usize))
}

#[test]
fn the_poisoned_index_refuses_at_the_boundary() {
    let n_layer = 128;
    assert_eq!(compute_fixed(0, n_layer), Err(Reject::ZeroIndex));
    assert_eq!(
        compute_fixed(n_layer + 1, n_layer),
        Err(Reject::IndexAboveLayers)
    );
    assert_eq!(compute_fixed(1048576, n_layer), Err(Reject::IndexAboveLayers));
    assert_eq!(compute_fixed(524290, n_layer), Err(Reject::IndexAboveLayers));
}

#[test]
fn in_range_indices_compute_wide_and_covered_offsets() {
    let n_layer = 128;
    for l in 1..=n_layer {
        let (len, dst, row) = compute_fixed(l, n_layer).ok().unwrap();
        assert!(dst + row <= len);
        assert_eq!(dst, (N_EMBD as usize) * ((l - 1) as usize));
        assert_eq!(len, (N_EMBD as usize) * (l as usize));
    }
}

#[test]
fn naive_and_fixed_agree_where_naive_does_not_overflow() {
    for l in [1i32, 2, 3, 127, 128] {
        let (len_n, dst_n) = compute_naive(l);
        let (len_f, dst_f, _row) = compute_fixed(l, 128).ok().unwrap();
        assert_eq!(len_n, len_f);
        assert_eq!(dst_n as usize, dst_f);
    }
}

fn main() {}"
                .to_string(),
        },
        // ── Rule 5 (category A — memory access/bounds): the M-RoPE position
        //    loops hardcode a 2x2 merge INSIDE a stride of n_merge; whole
        //    merge tiles bound the write domain by the patch count. ──
        KernelRule {
            rule_id: "merge-tile-whole-iteration-bounded-write-domain".to_string(),
            gpu_concept: "MECHANISM (whole-merge-tile iteration bounds the write domain; category A — memory access/bounds): four multimodal position encoders filled the M-RoPE positions buffer with an inner unrolled 2x2 merge hardcoded INSIDE an outer stride of n_merge, verbatim (commit message): \"The position loops of the Qwen-VL, GLM, PaddleOCR and Qwen2.5-VL encoders hardcode a 2x2 merge inside a stride of n_merge, so a patch grid that is not a multiple of the merge writes past the positions buffer. Iterate over whole merge tiles with n_merge inner steps, which bounds the writes by the patch count and gives the same positions on aligned grids.\" The shape: 'for (int y = 0; y < ph; y += merge_ratio) { ... for (int dy = 0; dy < 2; dy++) { ... positions[ptr] = y + dy; ... ptr++; } }' — the strided outer loop visits ceil(ph/merge) * ceil(pw/merge) tiles while each entry writes 4 quadrant slots, so an odd grid pushes ptr past n_pos (and y + dy past the patch range); with a merge other than 2 the hardcoded factor fills only the top-left 2x2 of every tile. The repair divides the grid INTO tile counts and iterates whole tiles ('const int pw = image_size_width  / patch_size / merge_ratio;' + 'for (int ty = 0; ty < ph; ty++) { for (int tx = 0; tx < pw; tx++) { for (int dy = 0; dy < merge_ratio; dy++) { for (int dx = 0; dx < merge_ratio; dx++) { const int y = ty * merge_ratio + dy; ...' — clip.cpp:5688-5702, the same shape at :5725-5739 and :5857-5870), and where the grid cannot be trusted to divide, the alignment became a build-boundary assert: \"PaddleOCR also requires a merge aligned image at graph build, as the Qwen-VL family already does\" ('GGML_ASSERT(img.nx() % (patch_size * model.hparams.n_merge) == 0);' — paddleocr.cpp:4-5). THE LAW: when an inner UNROLLED factor is hardcoded independently of the outer STRIDE it nests inside, the write domain is the tile count TIMES the hardcoded factor — not the grid — and overruns on non-multiple grids; iterate whole tiles (outer loops bounded by the tile counts, inner loops by the actual stride, position = tile*stride + offset) so the domain is bounded by the data on ANY grid, and refuse (assert) at the build boundary the grids the tiling cannot express — never truncate silently. REGIME: position/index buffers filled by merge-space tiling (M-RoPE, any 2-D patch-grid reindex); any strided outer loop with an unrolled inner block. ANTI-REGIME: grids guaranteed aligned upstream (the assert still pins the contract — keep it); inner factor == outer stride already (the bug needs them to diverge); allocations sized past the worst case by design. KINSHIP (do not double-count): oob-free-loop-inclusive-of-last-full-tile (entries_gpu_107, B252) owns the loop BOUND of a checked/unchecked tile split (which tiles the fast arm may take) — THIS owns the write domain of a hardcoded-factor loop (how many tiles the walk visits at all); moe-duplicate-id-compact-prepass (B224) is the same overflow-into-a-neighbor's-slice class on device routing ('could overflow into the slice of the next expert') — cite as the device twin; block-quant-weight-pad-never-extends-activations (entries_gpu_35) pads a FORMAT stride — THIS tiles a MERGE stride. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'merge tile|merge ratio|whole merge|n_merge' -> zero owners; 'positions buffer' -> zero; 'write past|overrun' -> fused-read-write-dst prose (the fusion race) + B224's overflow prose — no merge-tiling owner. NotBenchable here (correctness hazard — the overrun is a heap write past the positions vector on odd patch grids; the fixture counts the overrun writes exactly (28 on the 3x3/merge-2 grid), asserts aligned-grid position equality with the fixed walk, and the merge!=2 under-fill). Source: ggml-org/llama.cpp @ cf24317bea431b807dc93319d74c61bed6e20aff (MIT, #31336) — tools/mtmd/clip.cpp:5684-5706 + :5721-5743 + :5854-5873 + tools/mtmd/models/paddleocr.cpp:3-5, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"#![allow(dead_code)]
// ref_shape (rule: merge-tile-whole-iteration-bounded-write-domain): the
// position loops hardcode the inner unrolled factor (dy, dx < 2) INSIDE an
// outer stride of n_merge. On a patch grid that is not a multiple of the
// merge the pointer walks more tiles than the grid holds and writes past the
// positions buffer (each tile entry writes all four quadrant slots); with a
// merge other than 2 the hardcoded factor also fills only the top-left 2x2
// of every tile. Overrun writes are COUNTED, never performed -- the trace of
// the defect.

/// the strided loop with the hardcoded 2x2 merge inside it:
/// (positions, ptr_end, overruns)
fn fill_naive(ph: usize, pw: usize, merge: usize, n_pos: usize) -> (Vec<i32>, usize, usize) {
    let mut positions = vec![i32::MIN; 4 * n_pos];
    let mut ptr = 0usize;
    let mut overruns = 0usize;
    let mut y = 0usize;
    while y < ph {
        let mut x = 0usize;
        while x < pw {
            for dy in 0..2usize {
                for dx in 0..2usize {
                    let (yy, xx) = (y + dy, x + dx);
                    for q in 0..4usize {
                        let slot = q * n_pos + ptr;
                        if ptr < n_pos {
                            positions[slot] = if q % 2 == 0 { yy as i32 } else { xx as i32 };
                        } else {
                            overruns += 1; // the write past the buffer
                        }
                    }
                    ptr += 1;
                }
            }
            x += merge;
        }
        y += merge;
    }
    (positions, ptr, overruns)
}

#[test]
fn an_odd_grid_walks_more_tiles_than_the_grid_holds() {
    // 3x3 patches, merge 2: the strided loop visits ceil(3/2)^2 = 4 tile
    // steps, each advancing ptr once, so ptr reaches 16 over a 9-patch grid
    let (_positions, ptr_end, overruns) = fill_naive(3, 3, 2, 9);
    assert_eq!(ptr_end, 16);
    // 7 tile entries land past the grid; each writes all four quadrant slots
    assert_eq!(overruns, 28);
}

#[test]
fn a_merge_other_than_two_fills_only_the_top_left_corner() {
    // 4x4 patches, merge 4: one tile step is visited, writing 4 entries --
    // the hardcoded 2x2 fills only the tile's top-left 2x2 of the 4x4 grid
    let (positions, ptr_end, _overruns) = fill_naive(4, 4, 4, 16);
    assert_eq!(ptr_end, 4);
    let filled = positions.iter().filter(|&&v| v != i32::MIN).count();
    assert_eq!(filled, 4 * 4);
    // and every written y value is below 2: rows 2..4 of every tile are
    // never touched
    let max_y = positions
        .iter()
        .copied()
        .filter(|&v| v != i32::MIN)
        .max()
        .unwrap();
    assert_eq!(max_y, 1);
}

fn main() {}"
                .to_string(),
            opt_pattern: r"#![allow(dead_code)]
// opt_pattern (rule: merge-tile-whole-iteration-bounded-write-domain): the
// grid divides by the merge INTO TILE COUNTS and the loops iterate whole
// merge tiles -- ty/tx bounded by the tile counts, dy/dx by the actual merge
// ratio, position = tile * merge + offset -- so the write domain is bounded
// by the patch count on ANY grid, and a grid the merge cannot divide is
// refused at the build boundary, never truncated silently.

#[derive(Debug, PartialEq)]
enum Grid {
    Unaligned,
}

/// fill_naive is the ref_shape item, unchanged.
fn fill_naive(ph: usize, pw: usize, merge: usize, n_pos: usize) -> (Vec<i32>, usize, usize) {
    let mut positions = vec![i32::MIN; 4 * n_pos];
    let mut ptr = 0usize;
    let mut overruns = 0usize;
    let mut y = 0usize;
    while y < ph {
        let mut x = 0usize;
        while x < pw {
            for dy in 0..2usize {
                for dx in 0..2usize {
                    let (yy, xx) = (y + dy, x + dx);
                    for q in 0..4usize {
                        let slot = q * n_pos + ptr;
                        if ptr < n_pos {
                            positions[slot] = if q % 2 == 0 { yy as i32 } else { xx as i32 };
                        } else {
                            overruns += 1;
                        }
                    }
                    ptr += 1;
                }
            }
            x += merge;
        }
        y += merge;
    }
    (positions, ptr, overruns)
}

fn fill_tiles(ph: usize, pw: usize, merge: usize) -> Result<(Vec<i32>, usize), Grid> {
    if ph % merge != 0 || pw % merge != 0 {
        return Err(Grid::Unaligned); // the build-boundary refusal
    }
    let (ph_t, pw_t) = (ph / merge, pw / merge);
    let n_pos = ph * pw;
    let mut positions = vec![i32::MIN; 4 * n_pos];
    let mut ptr = 0usize;
    for ty in 0..ph_t {
        for tx in 0..pw_t {
            for dy in 0..merge {
                for dx in 0..merge {
                    let (y, x) = (ty * merge + dy, tx * merge + dx);
                    positions[ptr] = y as i32;
                    positions[n_pos + ptr] = x as i32;
                    positions[2 * n_pos + ptr] = y as i32;
                    positions[3 * n_pos + ptr] = x as i32;
                    ptr += 1;
                }
            }
        }
    }
    Ok((positions, ptr))
}

#[test]
fn aligned_grids_reproduce_the_naive_positions_exactly() {
    // 4x6 patches, merge 2: both walks are fully in range and identical
    let (ref_positions, ref_ptr, ref_over) = fill_naive(4, 6, 2, 24);
    let (opt_positions, opt_ptr) = fill_tiles(4, 6, 2).ok().unwrap();
    assert_eq!(ref_ptr, 24);
    assert_eq!(ref_over, 0);
    assert_eq!(opt_ptr, 24);
    assert_eq!(ref_positions, opt_positions);
}

#[test]
fn an_unaligned_grid_is_refused_not_overrun() {
    // the grid fill_naive overruns (28 writes past the buffer)
    let (_, _, over) = fill_naive(3, 3, 2, 9);
    assert_eq!(over, 28);
    // the tiled walk refuses the build instead
    assert_eq!(fill_tiles(3, 3, 2), Err(Grid::Unaligned));
    assert_eq!(fill_tiles(6, 6, 4), Err(Grid::Unaligned));
}

#[test]
fn a_merge_other_than_two_covers_every_tile_position() {
    // 4x4 patches, merge 4: the naive form fills 4 entries; the tiled walk
    // covers all 16 tile entries with every position inside the grid
    let (naive, naive_ptr, _) = fill_naive(4, 4, 4, 16);
    assert_eq!(naive_ptr, 4);
    let (positions, ptr) = fill_tiles(4, 4, 4).ok().unwrap();
    assert_eq!(ptr, 16);
    let n_pos = 16usize;
    for ptr_i in 0..n_pos {
        let y = positions[ptr_i];
        let x = positions[n_pos + ptr_i];
        assert!((0..4).contains(&(y as usize)));
        assert!((0..4).contains(&(x as usize)));
        assert_eq!(positions[2 * n_pos + ptr_i], y);
        assert_eq!(positions[3 * n_pos + ptr_i], x);
    }
    let _ = naive;
}

fn main() {}"
                .to_string(),
        },
    ]
}

#[allow(clippy::too_many_lines)]
fn slice_mlx_grid_slices() -> Vec<KernelRule> {
    vec![
        // ── Rule 6 (category E — parallelism/dispatch): a flat grid past the
        //    platform total-thread ceiling is SLICED into per-ceiling
        //    dispatches, each slice re-binding its buffer offsets; the
        //    incumbent silently truncated the tail. ──
        KernelRule {
            rule_id: "grid-ceiling-sliced-dispatch-per-slice-buffer-rebind".to_string(),
            gpu_concept: "MECHANISM (grid-ceiling sliced dispatch with per-slice buffer rebinding; category E — parallelism/dispatch): a row-parallel Metal launch whose flat grid is n_rows * group_size silently TRUNCATES past the platform ceiling — the commit that landed the shared helper states it verbatim (mlx/backend/metal/utils.h:23-28): \"Metal silently truncates a grid dimension past UINT32_MAX threads, so a launch of n_groups threadgroups along x is split into several dispatches that each fit. bind(first_group) is called before every dispatch to set whatever that slice needs, such as buffer offsets or a starting index. The slice size is kept a multiple of 16 groups so buffer offsets stay aligned. grid_y and grid_z are passed through unchanged to every dispatch.\" The helper: 'const int64_t max_groups = static_cast<int64_t>(UINT32_MAX / group_size) & ~int64_t(15);' then 'for (int64_t start = 0; start < n_groups; start += max_groups) { const int64_t groups = std::min(max_groups, n_groups - start); bind(start); compute_encoder.dispatch_threads(MTL::Size(groups * group_size, grid_y, grid_z), group_dims); }' (:37-45) — at the simd-size threadgroup (32) that ceiling is 134217712 = 2^27 - 16 groups, the commit title's \"past 2^27 rows\" (softmax/logsumexp/rms_norm/layer_norm all moved: logsumexp.cpp:84, softmax.cpp:76, normalization.cpp:84/:197/:298/:420, reduce.cpp:759/:823/:902/:940), each binding its slice's byte offsets through the callback ('compute_encoder.set_input_array(in, 0, row * in_row_bytes); compute_encoder.set_output_array(out, 1, row * out.itemsize());' — logsumexp.cpp:85-87) beside the row-count widening ('int64_t n_rows = in.data_size() / axis_size;' :59 — the width half cites widen-index-multiply-to-64bit, never re-owned here). THE LAW: when a flat grid can exceed the platform's total-thread dispatch ceiling, SLICE the dispatch along the indexed axis into ceiling-sized chunks and re-bind every buffer's byte offset at each slice's first row through a bind callback — the kernel stays flat per slice (no grid-stride loop to write, no per-thread bounds check), every unit of work runs exactly once, and clamping the grid instead silently drops the tail (the founding defect: rows past the ceiling never normalized). The 16-group alignment keeps every slice's base offset aligned. REGIME: row-parallel norm/reduction/softmax launches whose row count scales with batch x sequence on big-memory boxes; any 1-D flat grid over an indexed buffer on Metal (the total-thread ceiling) or CUDA per-dimension limits. ANTI-REGIME: grids that cannot exceed the ceiling (one dispatch — the helper's first slice is the incumbent byte-identically); kernels needing cross-slice reductions (slicing must not cut a cooperative reduction — per-row ops only); SYCL-class launchers that instead SHRINK the work-group and cover with an in-kernel grid-stride loop (the moe SoA rule's rider records that spelling — compose, never merge: shrink+grid-stride is the kernel-cooperative alternative, slice+rebind the kernel-blind one). KINSHIP (do not double-count): grid-stride-loop (the seed stub, category A) owns KERNEL-side iteration coverage — THIS is HOST-side launch slicing with argument rebinding, the kernel untouched; block-count-clamp (the seed stub) clamps a launch at 65535 with no coverage story — THIS replaces the clamp (which IS the truncation defect) with exact cover; occupancy-grid-cap-refuted-uncapped-default (entries_gpu_07) is the occupancy-side cap decision on non-cooperative kernels — different axis (a cap to fit RESIDENCY, not the dispatch ceiling); row-parallelism-ladder-split-vs-pack (entries_gpu_22) packs rows per warp + caps the grid into a grid-stride loop — its cap is the kernel-cooperative cousin; length-keyed-dispatch-cut-invariance (entries_gpu_101) owns the NUMERICS axis of a dispatch boundary — a slice here processes whole rows in one launch, so per-row arithmetic is bit-identical and no — CROSS-KIN (Batch 304, same batch): silent-zero-launch-row-chunked-past-grid-z-ceiling (entries_gpu_138) is the CUDA twin — gridDim.z's 65535 ceiling fails SILENT (no raise, output unwritten), remedied by row-chunked launches + the boundary sweep; the platforms differ in the failure MODE (Metal TRUNCATES the grid and drops the tail; CUDA ACCEPTS and runs nothing) and the remedy shape (per-slice buffer byte-offset rebind vs chunked launches + workspace sized at min(work, ceiling)) — same law, both halves: a launch whose row count can cross a platform dispatch ceiling must be split AT the ceiling, never clamped and never trusted. cut contract is owed. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'in slices|per-slice|sliced dispatch|max_groups' -> zero owners (the n_groups hits are MoE expert-group counts); 'UINT32_MAX|grid ceiling|total-thread' -> zero (the 65535 hits are block-count-clamp's CUDA grid-dim stub + prose asserts); 'grid-stride' -> the stub + five distinct-from cites (gpu_02/07/18/22 + the SoA SYCL rider) — every cite marks it distinct, none slices a launch. NotBenchable here (dispatch-capacity axis — the gain is correct coverage past a platform ceiling, not cycles; the fixture pins the ceiling formula (134217712 = 2^27 - 16 at group 32), the exact-launch partition, per-slice rebinding, and the truncated-tail discriminator). Source: ml-explore/mlx @ b996a43fa7c0d14880ce0e2685ca19c4b37360b4 (MIT, #4717, Arup Kanti Dash; the law's earlier reduction instance is 9cb9c06fb #4437) — mlx/backend/metal/utils.h:23-46 (the helper + its law) + logsumexp.cpp:59,:84-87 + softmax.cpp:47,:76 + normalization.cpp:84,:197,:298,:420 + reduce.cpp:759,:823,:902,:940, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"#![allow(dead_code)]
// ref_shape (rule: grid-ceiling-sliced-dispatch-per-slice-buffer-rebind): one
// flat dispatch of n_rows * group_size threads. The platform SILENTLY
// TRUNCATES a grid dimension past its total-thread ceiling -- Metal: the
// width past UINT32_MAX threads -- so rows behind the ceiling never run and
// their outputs keep whatever the buffer held.

const GROUP: usize = 32;

fn row_result(r: usize) -> f32 {
    (r as f32) * 0.5 + 1.0
}

/// the flat launch: the device runs only the ceiling-bounded head threads
fn dispatch_flat_truncated(n_rows: usize, max_groups: usize, out: &mut [Option<f32>]) -> usize {
    let n_threads = n_rows * GROUP;
    let ran = n_threads.min(max_groups * GROUP); // the silent truncation
    let mut t = 0usize;
    while t < ran {
        out[t / GROUP] = Some(row_result(t / GROUP));
        t += 1;
    }
    ran / GROUP
}

#[test]
fn the_tail_rows_behind_the_ceiling_never_run() {
    let mut out: Vec<Option<f32>> = vec![None; 100];
    let groups_ran = dispatch_flat_truncated(100, 64, &mut out);
    assert_eq!(groups_ran, 64);
    assert!(out[..64].iter().all(|v| v.is_some()));
    assert!(out[64..].iter().all(|v| v.is_none()));
}

fn main() {}"
                .to_string(),
            opt_pattern: r"#![allow(dead_code)]
// opt_pattern (rule: grid-ceiling-sliced-dispatch-per-slice-buffer-rebind):
// the launch SLICES along the row axis: max_groups threadgroups per dispatch,
// the ceiling kept a multiple of 16 groups so buffer offsets stay aligned,
// and every slice re-binds its buffers at the slice's first row through a
// bind(first_group) callback before dispatching. The kernel stays flat per
// slice; every row runs exactly once; grid_y and grid_z pass through
// unchanged.

const GROUP: usize = 32;

fn row_result(r: usize) -> f32 {
    (r as f32) * 0.5 + 1.0
}

/// the real ceiling: Metal's width past UINT32_MAX total threads, floored to
/// a 16-group boundary -- at group 32 that is 2^27 - 16 groups
fn metal_max_groups(group_size: usize) -> usize {
    ((u32::MAX as u64 / group_size as u64) as i64 & !15i64) as usize
}

#[derive(Debug, PartialEq)]
struct Launch {
    first_row: usize, // bind(first_group): the slice's buffer offset
    groups: usize,
    grid_y: usize,
    grid_z: usize,
}

/// dispatch_flat_truncated is the ref_shape item, unchanged.
fn dispatch_flat_truncated(n_rows: usize, max_groups: usize, out: &mut [Option<f32>]) -> usize {
    let n_threads = n_rows * GROUP;
    let ran = n_threads.min(max_groups * GROUP);
    let mut t = 0usize;
    while t < ran {
        out[t / GROUP] = Some(row_result(t / GROUP));
        t += 1;
    }
    ran / GROUP
}

fn dispatch_sliced(
    n_rows: usize,
    max_groups: usize,
    grid_y: usize,
    grid_z: usize,
    out: &mut [Option<f32>],
) -> Vec<Launch> {
    let mut launches = Vec::new();
    let mut start = 0usize;
    while start < n_rows {
        let groups = usize::min(max_groups, n_rows - start);
        // bind(start): the slice's buffers re-point at row * row_bytes
        for r in start..start + groups {
            out[r] = Some(row_result(r));
        }
        launches.push(Launch {
            first_row: start,
            groups,
            grid_y,
            grid_z,
        });
        start += groups;
    }
    launches
}

#[test]
fn the_real_ceiling_is_where_the_two_pow27_figure_comes_from() {
    // 4294967295 / 32 = 134217727, floored to a 16-group boundary
    assert_eq!(metal_max_groups(32), 134_217_712);
    assert_eq!(metal_max_groups(32) % 16, 0);
    assert_eq!(metal_max_groups(32) + 16, 1usize << 27);
}

#[test]
fn below_the_ceiling_one_dispatch_identical_to_the_flat_form() {
    let mut a: Vec<Option<f32>> = vec![None; 100];
    let mut b: Vec<Option<f32>> = vec![None; 100];
    let groups_ran = dispatch_flat_truncated(100, 128, &mut a);
    let launches = dispatch_sliced(100, 128, 1, 1, &mut b);
    assert_eq!(groups_ran, 100);
    assert_eq!(launches.len(), 1);
    assert_eq!(
        launches[0],
        Launch {
            first_row: 0,
            groups: 100,
            grid_y: 1,
            grid_z: 1
        }
    );
    assert_eq!(a, b);
}

#[test]
fn past_the_ceiling_every_slice_rebinds_and_every_row_runs_once() {
    let mut out: Vec<Option<f32>> = vec![None; 100];
    let launches = dispatch_sliced(100, 64, 3, 5, &mut out);
    assert_eq!(launches.len(), 2);
    assert_eq!(
        launches[0],
        Launch {
            first_row: 0,
            groups: 64,
            grid_y: 3,
            grid_z: 5
        }
    );
    assert_eq!(
        launches[1],
        Launch {
            first_row: 64,
            groups: 36,
            grid_y: 3,
            grid_z: 5
        }
    );
    for (r, v) in out.iter().enumerate() {
        assert_eq!(*v, Some(row_result(r)));
    }
    // the naive flat form truncates where the sliced form covers exactly
    let mut stale: Vec<Option<f32>> = vec![None; 100];
    let truncated = dispatch_flat_truncated(100, 64, &mut stale);
    assert_eq!(truncated, 64);
    assert!(stale[64..].iter().all(|v| v.is_none()));
}

fn main() {}"
                .to_string(),
        },
    ]
}

pub fn entries() -> Vec<KernelRule> {
    let mut v = slice_llamacpp_b304();
    v.extend(slice_mlx_grid_slices());
    v
}
