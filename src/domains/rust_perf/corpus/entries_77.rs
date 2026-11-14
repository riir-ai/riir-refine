//! Batch 361 — the polars staged-leads conversion `rust_perf` batch
//! (Plan 372 / Research 405 residue): three of the five deferred leads
//! survived authoring — the same-size in-place conversion (layout, the
//! lead: GOAT `ParetoDominant` 2/0/1 two-run stable at the standalone
//! preflight — the crate validator cannot host the consume, spec None —
//! the alloc axis 1→0), the thread-local codec-context reuse with a SIZE ADMISSION
//! (caching, suggestion-only — the two-regime IS the constant's doc), and
//! the batch-hash-probe-prefetch two-pass with the cache-residency gate
//! (io, decision-content-only on aarch64 — the positive arm of
//! `software-prefetch-storage-bound`). One count-neutral FACE on B4-era
//! `previous-row-group-check` (the SORTED pole of the run-locality law)
//! and one HONEST DROP (R8 `hot-key-skew-split-table`: the pre-registered
//! bars failed two-run stable — skew 0.465-0.467x, uniform 0.001x — the
//! bounded form's sort-based promote dominates; the drop IS the record).
//!
//! Source: `pola-rs/polars` @
//! `5c36cdd75a30a8ab12a9f70e251c0f267235b2a7` (master tip
//! 2026-11-13T21:53Z; MIT — copyright Ritchie Vink; every quote verified
//! byte-exact at this pin this batch; the tip moved ~6h past B360's pin
//! `437948302` and all five primary sites stood; the two disclosed drifts
//! are in-rule: R5's #29673 site re-worded upstream — the run-start law
//! generalized to hash-locality — and R7 gained a SECOND in-source
//! instance, the insert-side prefetch arm; clone removed at close).
//!
//! ROUTING per row: `rust_perf` — host-side hot-path laws; `kernel_opt`
//! loses (no GPU surface), sec loses (no attacker-facing surface).
//!
//! ORIGINAL Rust fixtures (`corpus/fixtures/b361_fixtures.rs`,
//! standalone-compiled AND executed before entry — rustc --edition 2021
//! --test, four-arm discipline per law; 8/8 green at the quiet window
//! after the held gate caught two fixture bugs: the `u64::MAX` miss-sentinel
//! sum overflow and a vacuous key-value tamper trigger replaced by a
//! positional one), never pasted source.
//! Batch 361 / Plan 372 / Research 405 (the staged residue).

#[allow(clippy::too_many_lines)] // the corpus entries ARE the payload
pub fn entries() -> Vec<crate::domains::rust_perf::PerfRule> {
    vec![
        // ── layout: a same-size primitive conversion allocates a fresh
        // buffer it could REUSE — the size/align gate makes the source's
        // buffer the output; the consume is the contract. ──────────────
        // Source: pola-rs/polars @ 5c36cdd75, polars-compute/src/arity.rs
        // prim_unary_values (+ the binary form).
        crate::domains::rust_perf::PerfRule {
            rule_id: "map-inplace-same-size-transmute".to_string(),
            name: "a primitive-to-primitive conversion whose input and output have the same size AND alignment allocates a fresh buffer via map+collect while the source dies right after → rewrite IN PLACE through the same allocation and reclaim it as the output type (bytemuck::cast_vec or the raw-parts form); the consume is the contract — the shape is legal only while the source is dead after the call".to_string(),
            category: "layout".to_string(),
            buggy_code: r#"// NAIVE: convert an owned buffer of f32 values to i32 via
// map+collect — a fresh Vec allocation per call, the source dropped
// right after. The conversion is same-size (4 bytes → 4 bytes), so
// the source's buffer could BE the output.

pub fn f32_vals_to_i32(v: Vec<f32>) -> Vec<i32> {
    v.iter().map(|&x| x as i32).collect()
}

// the caller's shape — the source dies at the call:
// let out = f32_vals_to_i32(readings); // readings: owned
"#.to_string(),
            fixed_code: r#"// OPT: the size/align gate holds, so rewrite through the same
// allocation and reclaim it. polars' kernel shape (arity.rs
// prim_unary_values): 'Reuse memory if possible' — the gate checked
// BEFORE the in-place kernel, the transmute() of the array after.

pub const SAME_SIZE: () = {
    assert!(core::mem::size_of::<f32>() == core::mem::size_of::<i32>());
    assert!(core::mem::align_of::<f32>() == core::mem::align_of::<i32>());
};

pub fn f32_vals_to_i32(v: Vec<f32>) -> Vec<i32> {
    let _: () = SAME_SIZE;
    let mut v = v;
    let len = v.len();
    let cap = v.capacity();
    let ptr = v.as_mut_ptr() as *mut i32;
    for i in 0..len {
        // read + write through raw pointers only — no `&f32` alive
        // across the `*mut i32` writes
        unsafe {
            let x = *ptr.cast::<f32>().add(i);
            *ptr.add(i) = x as i32;
        }
    }
    core::mem::forget(v);
    // the bytemuck::cast_vec shape in raw form (dep-free here; fire the
    // crate form only when the target manifest declares bytemuck)
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

#[test]
fn in_place_reuse_is_bit_identical_to_map_collect() {
    let src: Vec<f32> =
        [-3.0, -1.5, 0.0, 1.5, 3.0, 7.0, 100.0, 1e6].to_vec();
    let want: Vec<i32> = src.iter().map(|&x| x as i32).collect();
    let got = f32_vals_to_i32(src.clone());
    assert_eq!(want, got, "G1: bit-identical to map+collect");
}
"#.to_string(),
            context_notes: Some("MEASURED ParetoDominant 2/0/1 TWO-RUN STABLE (1.267x/1.499x at n=16384, 1.075x/1.066x at n=262144, Tie 1.000x at n=1024 both runs) — amo, M5 Ultra / Mac, rustc -O standalone preflight (.plans/b361_goat_preflight.rs, counting allocator), 2026-11-14 quiet window, load 2.7; alloc axis EXACT 1→0 per call (buggy: 1 alloc = the collect buffer; fixed: 0 allocs — the source's buffer IS the output; the SETUP clone disclosed as setup, never counted in the arm). INSTRUMENT SPLIT (disclosed, the honest-landing law): the crate validator's shared &Input harness CANNOT express the consume asymmetry — the fixed arm pays an input clone the buggy arm never pays, and a clone-taxed harness measured ParetoDominated 0/3 (the clone tax, not the law); bench_spec stays None and the STANDALONE PREFLIGHT is the instrument of record, the entries_17 decision-content precedent's measurement half. SOURCE (pola-rs/polars @ 5c36cdd75a30a8ab12a9f70e251c0f267235b2a7, master tip 2026-11-13T21:53Z, MIT; quotes verified byte-exact at the .raw/polars pin clone this batch). THE LAW (polars-compute/src/arity.rs prim_unary_values, verbatim): '/// Applies a function to all the values (regardless of nullability).' + '/// May reuse the memory of the array if possible.' + '// Reuse memory if possible.' + 'if size_of::<I>() == size_of::<O>() && align_of::<I>() == align_of::<O>() {' + '// SAFETY: checked same size & alignment I/O, NativeType is always Pod.' + 'return arr.transmute::<O>();' — THE GATE (same size AND same alignment) IS the decision content: it makes the source's buffer a legal output buffer. SECOND IN-SOURCE INSTANCE: the binary form '/// May reuse the memory of one of its arguments if possible.' + 'if size_of::<L>() == size_of::<O>() && align_of::<L>() == align_of::<O>() {' (lhs reused, rhs borrowed). THE CONSUME PRECONDITION IS LOAD-BEARING (the detection contract): bytemuck::cast_vec CONSUMES the source — the shape is legal only while the source is dead after the call (the first-mention scope class); a borrowed-source caller cannot take the shape (the all-safe negative arm; the size-MISMATCH pair i64→i32 stays collect). REGIME: owned same-size primitive buffers converted in hot rebuild paths (readings, samples, raw-frame pixel repacks, wire decode scratch); the win grows with call FREQUENCY at small-to-mid n (the alloc share) and holds on memory-traffic locality at large n (measured). ANTI-REGIMES: the source survives the call (borrowed input — the shape is IMPOSSIBLE, not a loss); size mismatch (i64→i32: collect keeps its own semantics); float↔int value casts where the target needs the EXACT source bits (bit-reinterpretation is a DIFFERENT conversion — this rule rewrites VALUES, never bits). G1: outputs bit-identical to map+collect on the value-cast ladder (the embedded test + the fixture's 12-value ladder incl. saturating extremes). DEDUP ADJUDICATION: retain-inplace (entries_02) owns the FILTER axis (predicate drops elements, borrow-based, no consume); the entries_06 bytemuck GPU-upload rule owns SKIP-conversion-when-layout-equals-target (no conversion happens — THIS converts values IN PLACE); bitflag-pack-when-footprint-dominates (entries_05) is the bool→bitfield byte pack (no transmute axis). ZERO prior owners of the same-size-transmute/in-place-conversion axis (T1 grep at HEAD `baa7bc16`: transmute-reuse/bytemuck/cast_slice hits read and ruled). Batch 361 / Plan 372 / Research 405 (the staged residue).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None, // the crate validator's &Input harness cannot
            // express the consume asymmetry (the law IS the ownership
            // transfer): a clone inside the timed arm measured
            // ParetoDominated 0/3 — the clone tax, not the law. The
            // standalone preflight (which owns the source) is the
            // instrument of record; the spec stays None rather than
            // land a harness that misreports the verdict class.
        },
        // ── caching: a stateful codec/handle constructed per call in a hot
        // loop re-pays its init every time → thread_local reuse with a
        // SIZE ADMISSION (oversize handles are cheap relative to their
        // work — never kept). ───────────────────────────────────────────
        // Source: pola-rs/polars @ 5c36cdd75, polars-utils/src/compression/
        // zstd.rs (thread_local CCTX/DCTX + MAX_CACHED_CONTEXT_BYTES).
        crate::domains::rust_perf::PerfRule {
            rule_id: "thread-local-codec-context-reuse".to_string(),
            name: "a stateful codec/compression handle constructed per call in a hot loop re-pays allocation + internal-state init every call → reuse it per thread in a slot, with a SIZE ADMISSION: handles configured beyond a byte bound are drop-on-return — their init is cheap relative to the work they do, so caching them buys nothing and pins memory".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: build the codec context fresh on every call — the
// allocation and internal-state init are paid per chunk, inside a
// hot rebuild loop.

fn compress_chunk(data: &[u8], level: i32) -> Vec<u8> {
    let mut ctx = CodecCtx::new(level); // alloc + init per call
    ctx.compress(data)
}
"#.to_string(),
            fixed_code: r#"// OPT: one context per thread, retuned per call, with the SIZE
// ADMISSION polars pins in the constant's own doc: contexts for high
// compression levels or large windows exceed the bound and are NOT
// kept — for those the cost of creating a context is small relative
// to the work.

const MAX_CACHED_CONTEXT_BYTES: usize = 2 * 1000 * 1000;

fn admitted(level: i32) -> bool {
    ctx_bytes(level) <= MAX_CACHED_CONTEXT_BYTES
}

thread_local! {
    static CCTX: core::cell::RefCell<Option<CodecCtx>> =
        const { core::cell::RefCell::new(None) };
}

fn compress_chunk(data: &[u8], level: i32) -> Vec<u8> {
    CCTX.with(|slot| {
        let mut slot = slot.borrow_mut();
        if !admitted(level) {
            return CodecCtx::new(level).compress(data); // never stored
        }
        let mut ctx = slot.take().unwrap_or_else(|| CodecCtx::new(level));
        ctx.level = level; // retune the cached handle
        let out = ctx.compress(data);
        *slot = Some(ctx);
        out
    })
}
"#.to_string(),
            context_notes: Some("SUGGESTION-ONLY, honestly NotBenchable-by-domain (the metric is codec THROUGHPUT; a stubbed-std fixture proves the reuse arm fires and the admission holds — 8/8 fixture green incl. the persisted-oversize tamper — but cannot measure a codec win; the validator's stub erases it). SOURCE (pola-rs/polars @ 5c36cdd75a30a8ab12a9f70e251c0f267235b2a7, MIT; quotes verified byte-exact at the .raw/polars pin clone this batch). THE LAW (polars-utils/src/compression/zstd.rs, verbatim): '//! zstd compression and decompression using state that is reused per thread.' + '//! Creating a zstd context allocates and initializes its internal state, which' + '//! can cost more than compressing or decompressing a small buffer.' + '/// Upper bound on the size of each zstd context kept per thread between calls.' + '/// Contexts for high compression levels or large windows exceed it and are not' + '/// kept, but for those the cost of creating a context is small relative to the' + '/// work.' + 'const MAX_CACHED_CONTEXT_BYTES: usize = 2 * 1000 * 1000;' + the thread_local CCTX/DCTX block — THE TWO-REGIME IS IN THE CONSTANT'S OWN DOC: small-window contexts cache; high-level/large-window contexts don't (their init is amortized by the work). THE ADMISSION IS THE RULE'S OWN CONTENT (reuse alone is generic; the size bound is the polars decision). REGIME: stateful handles (codec/regex/serialization contexts) constructed per call in hot loops where the handle's config stays in the cheap band. ANTI-REGIME: oversize configs (drop-on-return — the admission's negative arm, the fixture's tampered arm pins the persisted-oversize violation); thread-unsafe handles; handles whose retune costs more than construction. DEDUP ADJUDICATION: the entries_06 thread_local scratch-buffer rule owns DATA-buffer reuse (no stateful handle, no admission axis); logical-free-immediate-physical-reuse-waits-settle (entries_73) owns deferred MEMORY reuse across a settle boundary (a timing axis, not a handle-lifecycle axis); reuse-allocation-clear (entries_02) is an owned scratch buffer ACROSS calls (no admission). ZERO prior owners of the thread_local-context/size-admission axis (T1 grep at HEAD `baa7bc16`: thread_local hits = the scratch-buffer rule + fixture/negative notes; CCTX/zstd/codec-context = 0). Batch 361 / Plan 372 / Research 405 (the staged residue).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── io: a probe loop hashes then immediately probes — the bucket
        // load stalls before the next hash can issue → hash the BLOCK,
        // prefetch the addresses, then resolve; cache-resident tables
        // skip the arm entirely. ────────────────────────────────────────
        // Source: pola-rs/polars @ 5c36cdd75, polars-expr/src/hash_keys.rs
        // (+ the second in-source instance: the insert-side arm in
        // idx_table/single_key.rs).
        crate::domains::rust_perf::PerfRule {
            rule_id: "batch-hash-probe-prefetch-two-pass".to_string(),
            name: "a many-key hash-table probe loop that hashes each key and immediately probes stalls on the bucket load before the next hash can issue → two passes per BLOCK: hash the whole block + prefetch the bucket addresses, THEN resolve — several lookups in flight per block; a table below the cache-residency floor skips the prefetch arm entirely (small tables fit in the cache)".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: hash → probe, one key at a time. The bucket load is a
// cache miss on a big table, and the next hash cannot start until
// this probe resolves — the memory latency is exposed per key.

fn probe_all(keys: &[K], table: &Table) -> u64 {
    keys.iter().map(|k| table.probe(hash(k))).sum()
}
"#.to_string(),
            fixed_code: r#"// OPT: two passes per block — hash the block + prefetch the
// bucket addresses, THEN resolve. polars pins BOTH the block size
// and the regime gate as constants: 'Keys are hashed and prefetched
// in blocks of this many' (256) and 'Smaller tables fit in the cache
// and aren't prefetched' (1 << 15).

const BLOCK_SIZE: usize = 256;
const MIN_PREFETCH_BUCKETS: usize = 1 << 15;

fn probe_all_blocked(keys: &[K], table: &Table) -> u64 {
    if table.num_buckets() < MIN_PREFETCH_BUCKETS {
        return keys.iter().map(|k| table.probe(hash(k))).sum();
    }
    let mut acc = 0u64;
    for block in keys.chunks(BLOCK_SIZE) {
        let addrs: Vec<usize> =
            block.iter().map(|k| table.bucket_addr(hash(k))).collect();
        for &a in &addrs {
            table.prefetch(a); // x86 _mm_prefetch; core::arch arm only
        }
        for (i, k) in block.iter().enumerate() {
            acc += table.resolve(addrs[i], k);
        }
    }
    acc
}
"#.to_string(),
            context_notes: Some("NotBenchable ON THIS BOX — decision-content-only (the prefetch arm lowers to x86 core::arch::_mm_prefetch; this aarch64 box compiles the arm to NOTHING — the entries_17 posture, the M3/4090 GOAT re-gate rides an x86 window if ever owed; the fixture proves STRUCTURE + GATE, never a speed). SOURCE (pola-rs/polars @ 5c36cdd75a30a8ab12a9f70e251c0f267235b2a7, MIT; quotes verified byte-exact at the .raw/polars pin clone this batch). THE LAW (polars-expr/src/hash_keys.rs:29-32, verbatim): '/// Keys are hashed and prefetched in blocks of this many.' + 'pub(crate) const BLOCK_SIZE: usize = 256;' + '/// Smaller tables fit in the cache and aren't prefetched.' + 'pub(crate) const MIN_PREFETCH_BUCKETS: usize = 1 << 15;' — THE REGIME GATE IS THE DECISION CONTENT (cache-resident tables skip the arm; the fixture's counter-observable arms pin it: armed counts, disarmed never prefetches, the INVERTED gate is the tampered violation). THE BLOCK SHAPE (idx_table/single_key.rs:94-95, verbatim): '/// Probes the keys in blocks, first hashing (and prefetching) the whole' — hash-all THEN probe-all, the CPU works on several lookups at once. SECOND IN-SOURCE INSTANCE (verified this batch, the insert-side twin — single_key.rs ~L245-251): '// Reserve up front so the table doesn't move after prefetching.' + 'let prefetch = self.table.num_buckets() >= MIN_PREFETCH_BUCKETS;' + 'self.table.prefetch_insert(*hash);' — the INSERT path runs the same gated prefetch (a rehash after prefetching would invalidate the prefetched addresses — the reserve is load-bearing). ADJUDICATION IN-RULE BOTH DIRECTIONS vs software-prefetch-storage-bound (entries_05): THAT owns the bandwidth/storage-bound NEGATIVE arm (prefetch buys ≈nothing when bandwidth-bound, ACTIVELY HARMFUL at decode scale) and NAMED the positive regime without owning it; THIS owns the latency-bound POSITIVE arm with the cache-residency gate — a table below 2^15 buckets is the anti-regime HERE and the prefetch loop is the anti-regime THERE. REGIME: many-key probe loops over tables past L2 (joins, dedup maps, symbol tables); block-able key streams. ANTI-REGIMES: cache-resident tables (the gate); single-key or tiny lookups; bandwidth-bound streaming (the owner rule's arm). Batch 361 / Plan 372 / Research 405 (the staged residue).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
