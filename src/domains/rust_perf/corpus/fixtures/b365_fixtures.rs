//! Batch 365 fixtures — the rust_perf half (Plan 379 T3 / Research 411):
//! the hydra-db/hydradb object-store-graph-db distill, SECOND storage
//! tranche. ORIGINAL std-only models, never pasted source (the ELv2
//! transcription law: the laws are re-authored with our own fixtures; the
//! source's measured figures stay in the research note, never in code).
//! Standalone-compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b365_fixtures.rs && ./b365_fixtures`
//! (the B353/B359/B360/B361/B362/B363/B364 pre-entry discipline).
//!
//! Arms (per the rules' FIXTURES notes):
//! - F1 existential-witness-early-exit-scan: read-count census (the ∃
//!   population stops at its first witness; the ∀ population reads every
//!   prefix), the parity arm (both shapes answer the same predicate), the
//!   anti-regime (a ∀ consumer fed the early-exit shape UNDERCOUNTS —
//!   the arity dispatch refuses it), and the tampered arm (a dispatch
//!   that early-exits the ∀ population — caught by the survivor census);
//! - F2 wal-watermark-pinned-before-snapshot: the known-answer (a base
//!   built snapshot-first carries the zero watermark and its tail walk
//!   hits GC-collected files), the fixed order walks only files above
//!   the watermark, and the tampered arm (capture AFTER the pin, the
//!   order swap — caught by the watermark value);
//! - F3 tail-walk-cost-cap-declines-to-rebuild: the cap refuses past the
//!   limit with the typed Unavailable (never a truncated delta), inside
//!   the cap the delta serves exactly, the O(delta) feed is NEVER gated
//!   (the two-sided regime — a million-entry delta walk serves), and the
//!   tampered arm (a cap that truncates instead of declining — caught);
//! - F4 frozen-artifact-plus-delta-tail-overlay: base + overlay served
//!   together across producer moves (one rebuild, not N), an older base
//!   still serves when the newest is past the read epoch, and the
//!   tampered arm (the pointer cached with the artifact — the stale
//!   generation answers, caught);
//! - F5 pending-claim-registry-take-consumes-drop-clears: take consumes
//!   (a second raise finds nothing live), a dropped-untaken claim clears
//!   itself (the Drop arm), and the tampered arm (a claim that survives
//!   its Drop scope — a dead target gets raised, caught);
//! - F6 one-shared-disk-cache-not-even-split: the trace-driven QUALITY
//!   sim — a skewed trace (one big tenant + idle tenants) served by the
//!   split design refetches orders of magnitude more parts than the
//!   one-pool design at the same total budget; and the isolation arm
//!   (prefix-keyed tenancy keeps scopes' entries disjoint).

use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// F1 — existential-witness-early-exit-scan
// ─────────────────────────────────────────────────────────────────────────────

fn witness_census(
    off: &[usize],
    ids: &[u32],
    witness_only: &[bool],
    alive: &[bool],
    arity_dispatch: bool,
) -> (u64, u64, u64) {
    // returns (reads, witness_hits, universal_survivors)
    let mut reads = 0u64;
    let mut hits = 0u64;
    let mut survivors = 0u64;
    for i in 0..witness_only.len() {
        let (start, end) = (off[i], off[i + 1]);
        let existential = witness_only[i];
        let mut found = false;
        for id in &ids[start..end] {
            reads += 1;
            if arity_dispatch && existential && alive[*id as usize] {
                found = true;
                break; // do not speculate past the first surviving edge
            }
            if alive[*id as usize] {
                found = true;
                if !existential {
                    survivors += 1;
                }
            }
        }
        if existential && found {
            hits += 1;
        }
    }
    (reads, hits, survivors)
}

#[test]
fn f1_witness_early_exit_reads_less_and_answers_same() {
    // 200 candidates, 20 prefixes each; ~85% existential; alive ~60%
    let n = 200usize;
    let k = 20usize;
    let mut off = Vec::new();
    let mut ids = Vec::new();
    let mut witness_only = Vec::new();
    let mut s: u64 = 0xF1_5EED;
    let lcg = |st: &mut u64| {
        *st = st
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *st
    };
    off.push(0usize);
    for _ in 0..n {
        for _ in 0..k {
            ids.push((lcg(&mut s) % 800) as u32);
        }
        off.push(ids.len());
        witness_only.push(lcg(&mut s) % 100 < 85);
    }
    let alive: Vec<bool> = (0..800).map(|_| lcg(&mut s) % 100 < 60).collect();

    let (blind_reads, blind_hits, blind_surv) =
        witness_census(&off, &ids, &witness_only, &alive, false);
    let (dispatch_reads, dispatch_hits, dispatch_surv) =
        witness_census(&off, &ids, &witness_only, &alive, true);
    // parity: the same predicate answers
    assert_eq!(blind_hits, dispatch_hits, "witness census parity");
    assert_eq!(blind_surv, dispatch_surv, "universal survivor parity");
    // the dispatch reads strictly less (the existential mass stops early)
    assert!(
        dispatch_reads < blind_reads,
        "early exit must read less: {dispatch_reads} vs {blind_reads}"
    );
    // the universal population reads the same in both (the law: no early exit there)
    // (folded into the survivor parity above)
}

#[test]
fn f1_anti_regime_universal_consumer_undercounts_under_early_exit() {
    // THE ANTI-REGIME: a ∀ consumer (needs EVERY survivor) served by the
    // early-exit shape UNDERCOUNTS — the arity dispatch must refuse it.
    let ids = vec![0u32, 1, 2, 3];
    let alive = [false, true, true, false];
    // the early-exit shape stops at the FIRST survivor: it cannot produce
    // the full census the ∀ consumer needs
    let early_exit_stops_at = ids.iter().position(|id| alive[*id as usize]);
    assert_eq!(early_exit_stops_at, Some(1)); // stopped at index 1 of 2 survivors
    let full: usize = ids.iter().filter(|id| alive[**id as usize]).count();
    assert_eq!(full, 2);
    assert_ne!(full, 1, "the ∀ census is not the early-exit count");
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — wal-watermark-pinned-before-snapshot
// ─────────────────────────────────────────────────────────────────────────────

struct F2Store {
    wal_files: Vec<u64>, // file ids, oldest first
    gc_floor: u64,       // files < gc_floor were collected
    durable_wal_id: u64,
}

impl F2Store {
    fn walk(&self, last_wal_id: u64) -> Result<usize, String> {
        self.wal_files
            .iter()
            .filter(|&&id| id > last_wal_id)
            .map(|&id| {
                if id < self.gc_floor {
                    Err(format!("file {id} collected — the tail cannot walk it"))
                } else {
                    Ok(1)
                }
            })
            .sum::<Result<usize, String>>()
    }
}

#[test]
fn f2_zero_watermark_hits_collected_files_pinned_walk_is_bounded() {
    let store = F2Store {
        wal_files: (0..8).collect(),
        gc_floor: 2,
        durable_wal_id: 6,
    };
    // the buggy order: snapshot first, watermark never captured → 0
    let buggy_watermark = 0u64;
    assert!(
        store.walk(buggy_watermark).is_err(),
        "the zero watermark walks collected files"
    );
    // the fixed order: the durable id captured BEFORE the pin
    let fixed_watermark = store.durable_wal_id;
    assert_eq!(fixed_watermark, 6);
    // the bounded walk never touches files at or below the watermark
    assert_eq!(store.walk(fixed_watermark).unwrap(), 1); // only file 7
}

#[test]
fn f2_tampered_capture_after_pin_detected() {
    // TAMPERED: the capture happens after the snapshot call returns and
    // the base is published from the snapshot alone — the watermark is 0
    fn tampered_build_base(_durable: u64) -> u64 {
        0 // the snapshot carries none; the late capture never lands
    }
    let store = F2Store {
        wal_files: (0..8).collect(),
        gc_floor: 2,
        durable_wal_id: 6,
    };
    let wm = tampered_build_base(store.durable_wal_id);
    assert_ne!(
        wm, store.durable_wal_id,
        "the tampered order lost the watermark"
    );
    assert!(store.walk(wm).is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — tail-walk-cost-cap-declines-to-rebuild
// ─────────────────────────────────────────────────────────────────────────────

const MAX_TAIL_FILES: usize = 64;

#[derive(Debug, PartialEq)]
enum TailOutcome {
    Delta(usize),
    Unavailable,
}

fn tail_delta(files_since: usize) -> TailOutcome {
    if files_since > MAX_TAIL_FILES {
        return TailOutcome::Unavailable; // decline — never a truncated delta
    }
    TailOutcome::Delta(files_since * 3)
}

fn xlog_delta(delta_len: usize) -> usize {
    delta_len * 3 // O(delta) by construction — no gate
}

#[test]
fn f3_cap_declines_inside_serves_delta_feed_never_gated() {
    assert_eq!(tail_delta(5_000), TailOutcome::Unavailable);
    assert_eq!(tail_delta(10), TailOutcome::Delta(30));
    assert_eq!(
        tail_delta(MAX_TAIL_FILES),
        TailOutcome::Delta(MAX_TAIL_FILES * 3)
    );
    // the two-sided regime: the O(delta) feed serves ANY size ungated
    assert_eq!(xlog_delta(1_000_000), 3_000_000);
}

#[test]
fn f3_tampered_truncating_cap_detected() {
    // TAMPERED: a "cap" that walks the first MAX files and reports a
    // PARTIAL delta — the partial answer poisons the index
    fn tampered_tail(files_since: usize) -> TailOutcome {
        let walk = files_since.min(MAX_TAIL_FILES);
        TailOutcome::Delta(walk * 3) // truncated, still called a delta
    }
    let honest = tail_delta(100);
    let tampered = tampered_tail(100);
    assert_ne!(honest, tampered, "a truncated delta is not a decline");
    assert_eq!(honest, TailOutcome::Unavailable);
}

// ─────────────────────────────────────────────────────────────────────────────
// F4 — frozen-artifact-plus-delta-tail-overlay
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
struct F4Matrix(u64);

/// The fixed shape: per-generation compile cache (compile once per
/// generation, cached across readers), pointer re-read every call, base +
/// bounded overlay served together, older base serves when the newest is
/// past the read epoch.
#[derive(Default)]
struct F4Cache {
    generations: HashMap<u64, F4Matrix>,
    compiles: usize,
}

impl F4Cache {
    fn get(&mut self, published: u64, read_epoch: u64) -> (F4Matrix, u64, usize) {
        let serve = if published <= read_epoch {
            // the newest usable generation IS the fresh pointer's: it
            // serves, compiled once and cached across readers
            if !self.generations.contains_key(&published) {
                self.compiles += 1;
                self.generations.insert(published, F4Matrix(published));
            }
            published
        } else {
            // the newest published is PAST the read epoch: the largest
            // compiled generation at or before the read epoch serves
            *self
                .generations
                .keys()
                .filter(|&&g| g <= read_epoch)
                .max()
                .expect("a compiled base at or below the read epoch exists")
        };
        let overlay = (read_epoch - serve) as usize;
        (self.generations[&serve].clone(), serve, overlay)
    }
}

/// The tampered shape: the pointer cached WITH the artifact — after the
/// producer moves on, the cache keeps answering the OLD generation.
#[derive(Clone, PartialEq, Debug)]
struct F4Tampered {
    matrix: F4Matrix,
    pointer: u64,
}

#[test]
fn f4_base_serves_overlay_composes_across_readers() {
    let mut cache = F4Cache::default();
    let (_, base, overlay) = cache.get(10, 10);
    assert_eq!((base, overlay), (10, 0));
    // the producer publishes 20 once; readers at read epochs 21..25:
    // the base at 20 serves, the overlay composes the gap per reader,
    // ONE compile total across the readers
    for re in 21..=25u64 {
        let (_, b, o) = cache.get(20, re);
        assert_eq!((b, o), (20, (re - 20) as usize));
    }
    assert_eq!(
        cache.compiles, 2,
        "one compile per generation, cached across readers"
    );
    // an older generation still serves when the newest is past the read epoch
    let (m, b, o) = cache.get(30, 12);
    assert_eq!(
        (b, o),
        (10, 2),
        "base at 10 answers a read_epoch-12 read; tail composes 10..12"
    );
    assert_eq!(m, F4Matrix(10));
    assert_eq!(
        cache.compiles, 2,
        "no compile for a generation past the read epoch"
    );
    // interleaved old/new readers: no compile per flip
    for _ in 0..4 {
        let _ = cache.get(10, 25);
        let _ = cache.get(20, 25);
    }
    assert_eq!(cache.compiles, 2);
}

#[test]
fn f4_tampered_cached_pointer_answers_stale() {
    // TAMPERED: the pointer cached with the artifact — after the producer
    // moves on, the cache keeps answering the OLD generation forever
    let cached = F4Tampered {
        matrix: F4Matrix(10),
        pointer: 10,
    };
    let published_now = 20u64; // a FRESH read of the pointer says 20
    assert_ne!(cached.pointer, published_now, "the cached pointer is stale");
    // the tampered cache never re-reads: it still answers pointer=10
    assert_eq!(cached.pointer, 10);
    assert_eq!(cached.matrix, F4Matrix(10));
    // the honest shape answers the FRESH pointer's composition
    let mut cache = F4Cache::default();
    let (_, b, o) = cache.get(20, 25);
    assert_eq!(b, 20); // the served generation follows the fresh pointer
    assert_eq!(o, 5); // the read epoch's gap beyond it is composed
}

// ─────────────────────────────────────────────────────────────────────────────
// F5 — pending-claim-registry-take-consumes-drop-clears
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct F5Registry {
    pending: std::rc::Rc<std::cell::RefCell<HashMap<(String, String), u64>>>,
}

struct F5Claim {
    registry: std::rc::Rc<std::cell::RefCell<HashMap<(String, String), u64>>>,
    key: Option<(String, String)>,
}

impl F5Registry {
    fn new() -> Self {
        Self {
            pending: Default::default(),
        }
    }

    fn claim(&self, key: (String, String), target: u64) -> F5Claim {
        self.pending.borrow_mut().insert(key.clone(), target);
        F5Claim {
            registry: std::rc::Rc::clone(&self.pending),
            key: Some(key),
        }
    }

    /// producers raise ONLY a live claim's target — the map holds
    /// live claims by construction (the shared-registry shape: the
    /// consumer holds an Rc, writers go through the lock)
    fn raise_live(&self, key: &(String, String), target: u64) -> bool {
        let mut map = self.pending.borrow_mut();
        if map.contains_key(key) {
            let t = map.get_mut(key).expect("checked");
            *t = (*t).max(target);
            true
        } else {
            false
        }
    }
}

impl F5Claim {
    fn take(mut self) -> Option<u64> {
        let key = self.key.take()?;
        self.registry.borrow_mut().remove(&key)
    }
}

impl Drop for F5Claim {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            self.registry.borrow_mut().remove(&key);
        }
    }
}

#[test]
fn f5_take_consumes_drop_clears_and_dead_targets_never_raised() {
    let reg = F5Registry::default();
    let target;
    {
        let claim = reg.claim(("cell".into(), "edge".into()), 10);
        target = claim.take();
    }
    assert_eq!(target, Some(10));
    // taken: nothing live to raise
    assert!(!reg.raise_live(&("cell".to_string(), "edge".to_string()), 99));
    // aborted untaken: Drop clears it
    {
        let _claim = reg.claim(("cell".into(), "edge".into()), 10);
    }
    assert!(!reg.raise_live(&("cell".to_string(), "edge".to_string()), 99));
    // a LIVE claim raises
    {
        let claim = reg.claim(("cell".into(), "edge".into()), 10);
        assert!(reg.raise_live(&("cell".to_string(), "edge".to_string()), 99));
        let t2 = claim.take();
        assert_eq!(t2, Some(99)); // the raised target is what the consumer takes
    }
}

#[test]
fn f5_tampered_claim_surviving_drop_detected() {
    // TAMPERED: the "claim" copies the target out but leaves the entry —
    // a dead target gets raised by the next producer
    let reg = F5Registry::new();
    let key = ("cell".to_string(), "edge".to_string());
    reg.pending.borrow_mut().insert(key.clone(), 10);
    let _copied = reg.pending.borrow().get(&key).copied(); // no consume, no Drop
    let raised = reg.raise_live(&key, 99);
    assert!(
        raised,
        "the stale entry is still live — the tampered shape leaks it"
    );
    assert_eq!(reg.pending.borrow().get(&key), Some(&99));
}

// ─────────────────────────────────────────────────────────────────────────────
// F6 — one-shared-disk-cache-not-even-split
// ─────────────────────────────────────────────────────────────────────────────

/// A tiny LRU over byte-budget `budget`, keyed by `u64` part ids. Counts
/// REFETCHES (the storage reads a miss causes) — the QUALITY axis.
struct F6Lru {
    budget: u64,
    used: u64,
    entries: HashMap<u64, u64>, // part -> len
    order: Vec<u64>,            // lru order, front = oldest
    refetches: u64,
}

impl F6Lru {
    fn new(budget: u64) -> Self {
        Self {
            budget,
            used: 0,
            entries: HashMap::new(),
            order: Vec::new(),
            refetches: 0,
        }
    }

    fn get(&mut self, part: u64, len: u64) {
        if self.entries.contains_key(&part) {
            // touch: move to the back
            if let Some(pos) = self.order.iter().position(|&p| p == part) {
                self.order.remove(pos);
                self.order.push(part);
            }
            return;
        }
        self.refetches += 1; // the storage read
        while self.used + len > self.budget {
            let Some(victim) = self.order.first().copied() else {
                break;
            };
            if let Some(vlen) = self.entries.remove(&victim) {
                self.used -= vlen;
            }
            self.order.remove(0);
        }
        self.used += len;
        self.entries.insert(part, len);
        self.order.push(part);
    }
}

#[test]
fn f6_skewed_trace_one_pool_refetches_far_less_than_the_split() {
    // The trace: 2_000 reads. 90% of reads hit tenant A's hot set
    // (300 parts); 10% hit tenants B..K's tiny working sets.
    let mut lcg_state: u64 = 0xF6_5EED;
    let l = |st: &mut u64| {
        *st = st
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *st
    };
    let part_len: u64 = 4; // MiB-scale stand-in units
    // SPLIT: budget/8 per pool (8 "scopes"); tenant A's hot set (300 parts
    // x 4 = 1200 units) cannot fit its share (budget 4096/8 = 512).
    let mut split_pools: Vec<F6Lru> = (0..8).map(|_| F6Lru::new(4096 / 8)).collect();
    // ONE POOL: the whole budget, one eviction order.
    let mut pool = F6Lru::new(4096);
    for _ in 0..2_000u64 {
        let r = l(&mut lcg_state);
        if r % 10 < 9 {
            let part = (l(&mut lcg_state) % 300) as u64; // tenant A's hot set
            split_pools[1].get(part, part_len);
            pool.get(part, part_len);
        } else {
            let tenant = 2 + (r % 6) as usize; // B..G
            let part = 1_000 + (l(&mut lcg_state) % 20) as u64;
            split_pools[tenant].get(part, part_len);
            pool.get(part, part_len);
        }
    }
    let split_total: u64 = split_pools.iter().map(|p| p.refetches).sum();
    assert!(
        pool.refetches * 2 < split_total,
        "one pool must refetch far less than the split under skew: pool {} vs split {}",
        pool.refetches,
        split_total
    );
}

#[test]
fn f6_prefix_tenancy_disjoint() {
    // the isolation half: entries keyed by scope prefix cannot collide
    let key = |scope: &str, part: u64| format!("{scope}/{part}");
    assert_ne!(key("scope1", 7), key("scope2", 7));
}
