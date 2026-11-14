//! Batch 361 fixtures (Plan 372 / Research 405 residue) — ORIGINAL std-only
//! reproductions, never polars source. Four-arm discipline per law:
//! buggy / fixed / all-safe / tampered. Standalone-compiled AND executed
//! BEFORE entry: `rustc --edition 2021 --test b361_fixtures.rs && ./b361_fixtures`
//! (the B353/B359/B360 pre-entry discipline).
//! ⛔ EXECUTION POSTURE (Plan-337, the B352/B353 box-state discipline): the
//! sibling correctness suite holds amo at authoring time — this file is
//! WRITTEN zero-contention; EXECUTION is the quiet-window gate BEFORE any
//! corpus edit (plan 372 T3). Nothing here is "verified" until that run.
//!
//! Arms:
//! - R3 `map-inplace-same-size-transmute`: map+collect vs in-place rewrite +
//!   buffer reclaim; G1 = outputs identical across the value ladder;
//!   all-safe = the size-mismatch negative arm (the gate refuses, collect
//!   stays); tampered = the buffer reclaimed WITHOUT the conversion loop —
//!   the bit-garbage divergence, caught.
//! - R6 `thread-local-codec-context-reuse`: per-call handle construction vs
//!   thread_local reuse with a SIZE ADMISSION; G1 = outputs identical across
//!   a level ladder incl. the reuse path; all-safe = the oversize posture
//!   (temp handle, never stored); tampered = the admission removed — an
//!   oversize config persists, caught.
//! - R7 `batch-hash-probe-prefetch-two-pass`: probe-per-key vs the two-pass
//!   block probe with the cache-residency gate; G1 = identical sums with the
//!   gate armed and disarmed; all-safe = the small-table plain path (zero
//!   prefetches); tampered = the gate INVERTED (small table prefetched, big
//!   one not) — the regime violation observable in the prefetch counter.
//! - R8 `hot-key-skew-split-table`: single table vs the bounded hot tier with
//!   promote-on-miss; G1 = the logical count map identical on skew and
//!   uniform streams; all-safe = the tier bound holds; tampered = promotion
//!   LOSES the moved count — the union diverges from the single-table truth.

// ── R3 shared ─────────────────────────────────────────────────────────
const R3_SAME_SIZE: () = {
    assert!(core::mem::size_of::<f32>() == core::mem::size_of::<i32>());
    assert!(core::mem::align_of::<f32>() == core::mem::align_of::<i32>());
};

fn to_i32_map_collect(v: &[f32]) -> Vec<i32> {
    v.iter().map(|&x| x as i32).collect()
}

/// The fixed shape: rewrite in place through raw pointers (no `&f32` alive
/// across the `*mut i32` writes), then reclaim the SAME buffer as the output.
/// `bytemuck::cast_vec` in std-only form; the consume is the contract — the
/// source is dead after the call.
fn to_i32_inplace(v: Vec<f32>) -> Vec<i32> {
    let _: () = R3_SAME_SIZE;
    let mut v = v;
    let len = v.len();
    let cap = v.capacity();
    let ptr = v.as_mut_ptr() as *mut i32;
    for i in 0..len {
        unsafe {
            let x = *ptr.cast::<f32>().add(i); // read the source bits
            *ptr.add(i) = x as i32; // write the converted value
        }
    }
    core::mem::forget(v);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

/// All-safe negative arm: the size-mismatch pair — the gate refuses, collect
/// stays (truncation semantics, a fresh buffer, no reclaim shape applies).
fn to_i32_from_i64_collect(v: &[i64]) -> Vec<i32> {
    const R3_SIZE_MISMATCH: () =
        assert!(core::mem::size_of::<i64>() != core::mem::size_of::<i32>());
    let _: () = R3_SIZE_MISMATCH;
    v.iter().map(|&x| x as i32).collect()
}

/// TAMPERED: the buffer reclaimed WITHOUT the conversion loop — the reuse
/// claim without the rewrite. The "output" is the source's BITS reinterpreted;
/// the divergence from the map+collect truth is the catch.
fn tampered_reclaim_without_rewrite(v: Vec<f32>) -> Vec<i32> {
    let _: () = R3_SAME_SIZE;
    let mut v = v;
    let len = v.len();
    let cap = v.capacity();
    let ptr = v.as_mut_ptr() as *mut i32;
    core::mem::forget(v); // no rewrite loop — the load-bearing half dropped
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

fn r3_ladder() -> Vec<f32> {
    vec![
        -3.0,
        -1.5,
        -0.0,
        0.0,
        1.5,
        3.0,
        7.0,
        100.0,
        1e6,
        2.5e9,
        f32::MAX,
        -f32::MAX,
    ]
}

#[test]
fn r3_g1_ladder_identical_and_negative_arm_gated() {
    let src = r3_ladder();
    let want = to_i32_map_collect(&src);
    let got = to_i32_inplace(src.clone());
    assert_eq!(
        want, got,
        "G1: in-place reuse must be bit-identical to map+collect"
    );

    // the negative arm: a size-mismatch pair never takes the reclaim shape
    let i64s: Vec<i64> = vec![-5, 0, 5, 1 << 40, i64::MAX];
    let trunc = to_i32_from_i64_collect(&i64s);
    assert_eq!(trunc.len(), i64s.len());
    assert_eq!(
        trunc[3],
        (1i64 << 40) as i32,
        "collect keeps its own semantics"
    );
}

#[test]
fn r3_tampered_reclaim_diverges_caught() {
    let src = r3_ladder();
    let truth = to_i32_map_collect(&src);
    let garbage = tampered_reclaim_without_rewrite(src.clone());
    let diverges: Vec<_> = truth
        .iter()
        .zip(garbage.iter())
        .filter(|(t, g)| t != g)
        .collect();
    assert!(
        !diverges.is_empty(),
        "tampered: reclaim-without-rewrite must diverge on the ladder"
    );
}

// ── R6 shared ─────────────────────────────────────────────────────────
struct CodecCtx {
    level: i32,
    scratch: Vec<u8>,
}

impl CodecCtx {
    fn new(level: i32) -> Self {
        // "allocates and initializes its internal state" — the per-call cost
        CodecCtx {
            level,
            scratch: vec![0u8; ctx_bytes(level)],
        }
    }
    fn window_bytes(&self) -> usize {
        self.scratch.capacity()
    }
    fn compress(&mut self, data: &[u8]) -> Vec<u8> {
        self.scratch.clear();
        for (i, &b) in data.iter().enumerate() {
            self.scratch
                .push(b.wrapping_add(i as u8).wrapping_add(self.level as u8));
        }
        self.scratch.clone()
    }
}

fn ctx_bytes(level: i32) -> usize {
    128 + 16 * level.max(0) as usize
}

/// "Upper bound on the size of each zstd context kept per thread between
/// calls." — the admission IS the two-regime: oversize handles are cheap
/// relative to their work, so they are never kept.
const MAX_CACHED_CONTEXT_BYTES: usize = 2 * 1000 * 1000;

fn admitted(level: i32) -> bool {
    ctx_bytes(level) <= MAX_CACHED_CONTEXT_BYTES
}

fn compress_buggy(data: &[u8], level: i32) -> Vec<u8> {
    CodecCtx::new(level).compress(data) // constructed per call
}

thread_local! {
    static R6_CCTX: core::cell::RefCell<Option<CodecCtx>> =
        const { core::cell::RefCell::new(None) };
}

fn compress_fixed(data: &[u8], level: i32) -> Vec<u8> {
    R6_CCTX.with(|slot| {
        let mut slot = slot.borrow_mut();
        if !admitted(level) {
            return CodecCtx::new(level).compress(data); // oversize: never stored
        }
        let mut ctx = slot.take().unwrap_or_else(|| CodecCtx::new(level));
        ctx.level = level; // retune the cached handle
        let out = ctx.compress(data);
        *slot = Some(ctx);
        out
    })
}

fn fixed_cached_bytes() -> Option<usize> {
    R6_CCTX.with(|slot| slot.borrow().as_ref().map(CodecCtx::window_bytes))
}

thread_local! {
    static R6_TAM: core::cell::RefCell<Option<CodecCtx>> =
        const { core::cell::RefCell::new(None) };
}

/// TAMPERED: the admission removed — EVERY handle is cached, oversize
/// included. The violation the gate exists to prevent, demonstrated.
fn compress_tampered(data: &[u8], level: i32) -> Vec<u8> {
    R6_TAM.with(|slot| {
        let mut slot = slot.borrow_mut();
        let mut ctx = slot.take().unwrap_or_else(|| CodecCtx::new(level));
        ctx.level = level;
        let out = ctx.compress(data);
        *slot = Some(ctx); // no admission check
        out
    })
}

fn tampered_cached_bytes() -> Option<usize> {
    R6_TAM.with(|slot| slot.borrow().as_ref().map(CodecCtx::window_bytes))
}

#[test]
fn r6_g1_ladder_identical_incl_reuse_path() {
    let levels = [0, 1, 3, 7, 12];
    let data: Vec<u8> = (0..96u8).collect();
    for round in 0..3 {
        for &level in &levels {
            let b = compress_buggy(&data, level);
            let f = compress_fixed(&data, level);
            assert_eq!(
                b, f,
                "G1 round {round} level {level}: reuse must not change output"
            );
        }
    }
}

#[test]
fn r6_admission_holds_and_tampered_persists_oversize() {
    let data = vec![7u8; 64];
    let oversize_level = 200_000; // 128 + 16*200000 = 3.2 MB > 2 MB bound

    // all-safe: the oversize call answers correctly and stores NOTHING
    let f = compress_fixed(&data, oversize_level);
    let b = compress_buggy(&data, oversize_level);
    assert_eq!(b, f);
    match fixed_cached_bytes() {
        None => {} // nothing cached at all
        Some(n) => assert!(
            n <= MAX_CACHED_CONTEXT_BYTES,
            "cached handle over the bound"
        ),
    }

    // tampered: the same call under no-admission leaves an oversize config
    let _ = compress_tampered(&data, oversize_level);
    let t = tampered_cached_bytes();
    assert!(
        matches!(t, Some(n) if n > MAX_CACHED_CONTEXT_BYTES),
        "tampered posture must demonstrate the persisted-oversize violation \
         (got {t:?})"
    );
    // the deterministic proof: re-run the fixed path and assert the bound
    let _ = compress_fixed(&data, 0);
    if let Some(n) = fixed_cached_bytes() {
        assert!(n <= MAX_CACHED_CONTEXT_BYTES);
    }
}

// ── R7 shared ─────────────────────────────────────────────────────────
const BLOCK_SIZE: usize = 256;
const MIN_PREFETCH_BUCKETS: usize = 1 << 15;

fn hash_key(k: u64) -> u64 {
    k.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

struct ProbeTable {
    buckets: Vec<u64>, // key+1 stored, 0 = empty
}

impl ProbeTable {
    fn new(num_buckets: usize) -> Self {
        assert!(num_buckets.is_power_of_two());
        ProbeTable {
            buckets: vec![0; num_buckets],
        }
    }
    fn bucket_addr(&self, hash: u64) -> usize {
        (hash as usize) & (self.buckets.len() - 1)
    }
    /// The prefetch hook: on x86 this lowers to `_mm_prefetch`; on this
    /// aarch64 box it is the decision-content stub (the entries_17 posture) —
    /// the STRUCTURE and the REGIME GATE are what the fixture proves.
    #[inline]
    fn prefetch(&self, _addr: usize) {}
    fn insert(&mut self, key: u64) {
        let a = self.bucket_addr(hash_key(key));
        self.buckets[a] = key + 1;
    }
    fn resolve(&self, key: u64) -> u64 {
        let slot = self.buckets[self.bucket_addr(hash_key(key))];
        if slot == key + 1 {
            slot - 1
        } else {
            u64::MAX // the miss sentinel (deterministic, both arms)
        }
    }
    fn probe_all_per_key(&self, keys: &[u64]) -> u64 {
        keys.iter().map(|&k| self.resolve(k)).sum()
    }
    /// The fixed shape: hash the block + prefetch the addresses, THEN resolve.
    /// "Smaller tables fit in the cache and aren't prefetched" — the gate.
    fn probe_all_blocked(&self, keys: &[u64], prefetches: &mut u64) -> u64 {
        if self.buckets.len() < MIN_PREFETCH_BUCKETS {
            return self.probe_all_per_key(keys); // cache-resident: plain path
        }
        let mut acc = 0u64;
        for block in keys.chunks(BLOCK_SIZE) {
            let addrs: Vec<usize> = block
                .iter()
                .map(|&k| self.bucket_addr(hash_key(k)))
                .collect();
            for &a in &addrs {
                self.prefetch(a);
                *prefetches += 1;
            }
            for &k in block {
                acc += self.resolve(k);
            }
        }
        acc
    }
    /// TAMPERED: the gate inverted — the small (cache-resident) table pays
    /// the prefetch arm, the big one never does. The regime law, broken.
    fn probe_all_blocked_inverted(&self, keys: &[u64], prefetches: &mut u64) -> u64 {
        if self.buckets.len() > MIN_PREFETCH_BUCKETS {
            return self.probe_all_per_key(keys);
        }
        let mut acc = 0u64;
        for block in keys.chunks(BLOCK_SIZE) {
            let addrs: Vec<usize> = block
                .iter()
                .map(|&k| self.bucket_addr(hash_key(k)))
                .collect();
            for &a in &addrs {
                self.prefetch(a);
                *prefetches += 1;
            }
            for &k in block {
                acc += self.resolve(k);
            }
        }
        acc
    }
}

fn key_stream(n: usize, seed: u64) -> Vec<u64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 33) % 4096
        })
        .collect()
}

#[test]
fn r7_g1_identical_and_gate_counters() {
    let keys = key_stream(2048, 0xDE);
    let mut big = ProbeTable::new(1 << 15); // gate ARMED (== MIN, not <)
    let mut small = ProbeTable::new(1 << 8); // gate disarmed
    for &k in &keys {
        big.insert(k);
        small.insert(k);
    }
    // G1: the blocked shape answers identically on both postures
    let mut pf_big = 0u64;
    let mut pf_small = 0u64;
    assert_eq!(
        big.probe_all_per_key(&keys),
        big.probe_all_blocked(&keys, &mut pf_big)
    );
    assert_eq!(
        small.probe_all_per_key(&keys),
        small.probe_all_blocked(&keys, &mut pf_small)
    );
    // the regime gate is observable: armed counts, disarmed never prefetches
    assert!(pf_big > 0, "armed table must run the prefetch arm");
    assert_eq!(pf_small, 0, "cache-resident table must take the plain path");
}

#[test]
fn r7_tampered_inverted_gate_caught() {
    let keys = key_stream(2048, 0xDE);
    let mut small = ProbeTable::new(1 << 8);
    for &k in &keys {
        small.insert(k);
    }
    let mut pf = 0u64;
    let _ = small.probe_all_blocked_inverted(&keys, &mut pf);
    assert!(
        pf > 0,
        "tampered: the inverted gate prefetches the cache-resident table — \
         the regime violation the real gate prevents, demonstrated"
    );
}

// ── R8 shared ─────────────────────────────────────────────────────────
const HOT_MAX: usize = 8;
const PROMOTE_CHECK_EVERY: u64 = 16;

use std::collections::HashMap;

fn count_single(stream: &[u64]) -> HashMap<u64, u64> {
    let mut m = HashMap::new();
    for &k in stream {
        *m.entry(k).or_insert(0) += 1;
    }
    m
}

struct TwoTier {
    hot: HashMap<u64, u64>,
    main: HashMap<u64, u64>,
    since_check: u64,
}

impl TwoTier {
    fn new() -> Self {
        TwoTier {
            hot: HashMap::new(),
            main: HashMap::new(),
            since_check: 0,
        }
    }
    fn add(&mut self, k: u64) {
        if let Some(c) = self.hot.get_mut(&k) {
            *c += 1;
            return;
        }
        *self.main.entry(k).or_insert(0) += 1;
        self.since_check += 1;
        if self.since_check >= PROMOTE_CHECK_EVERY {
            self.since_check = 0;
            self.promote_top();
        }
    }
    /// Promote the most-missed main keys into the BOUNDED hot tier; evict the
    /// coldest back when over the bound. hot ∪ main is one logical map at
    /// every instant — the split is placement only.
    fn promote_top(&mut self) {
        let mut rows: Vec<(u64, u64)> = self.main.iter().map(|(&k, &c)| (k, c)).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (k, c) in rows.into_iter().take(HOT_MAX) {
            if self.hot.len() >= HOT_MAX {
                break;
            }
            if !self.hot.contains_key(&k) {
                self.hot.insert(k, c);
                self.main.remove(&k);
            }
        }
        while self.hot.len() > HOT_MAX {
            let cold = self.hot.iter().min_by_key(|(_, &c)| c).map(|(&k, _)| k);
            match cold {
                Some(k) => {
                    let c = self.hot.remove(&k).unwrap();
                    *self.main.entry(k).or_insert(0) = c;
                }
                None => break,
            }
        }
    }
    fn merged(&self) -> HashMap<u64, u64> {
        let mut m = self.main.clone();
        for (k, c) in &self.hot {
            *m.entry(*k).or_insert(0) += *c;
        }
        m
    }
}

/// TAMPERED: the promotion that LOSES the moved count (placement overwrites
/// it) — the union diverges from the single-table truth. G1's catch.
struct TwoTierLossy(TwoTier);

impl TwoTierLossy {
    fn add(&mut self, k: u64) {
        self.0.add(k);
    }
    fn promote_lossy(&mut self) {
        let pick = self
            .0
            .main
            .iter()
            .map(|(&k, &c)| (k, c))
            .max_by_key(|(_, c)| *c);
        if let Some((k, _)) = pick {
            self.0.main.remove(&k);
            self.0.hot.insert(k, 0); // the count dropped on the move
        }
    }
    fn merged(&self) -> HashMap<u64, u64> {
        self.0.merged()
    }
}

/// Seeded skew stream: a heavy head with a long tail (zipf-shaped, rounded
/// through a modulus so the key space stays bounded and the counts real).
fn skew_stream(n: usize, seed: u64) -> Vec<u64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let r = ((s >> 33) as f64) / ((u64::MAX >> 33) as f64);
            ((1.0 / (1.0 - r * 0.99)) as u64) % 64
        })
        .collect()
}

#[test]
fn r8_g1_streams_identical_and_bound_holds() {
    for (name, stream) in [
        ("skew", skew_stream(4096, 0xC0FFEE)),
        ("uniform", key_stream(4096, 0x42)),
    ] {
        let truth = count_single(&stream);
        let mut tier = TwoTier::new();
        for &k in &stream {
            tier.add(k);
        }
        assert_eq!(
            truth,
            tier.merged(),
            "G1 {name}: hot ∪ main must equal the single-table truth"
        );
        assert!(
            tier.hot.len() <= HOT_MAX,
            "{name}: the hot tier bound must hold (got {})",
            tier.hot.len()
        );
    }
}

#[test]
fn r8_tampered_count_loss_caught() {
    let stream = skew_stream(1024, 0xC0FFEE);
    let truth = count_single(&stream);
    let mut lossy = TwoTierLossy(TwoTier::new());
    for &k in &stream {
        lossy.add(k);
        if k % 97 == 0 {
            lossy.promote_lossy(); // fires during the stream
        }
    }
    let merged = lossy.merged();
    let total_truth: u64 = truth.values().sum();
    let total_merged: u64 = merged.values().sum();
    assert_ne!(
        total_truth, total_merged,
        "tampered: the count-loss promotion must diverge from the truth"
    );
}
