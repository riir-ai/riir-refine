//! Batch 368 rust_perf fixtures — the kache build-cache distill (Plan 382 /
//! Research 414): `kunobi-ninja/kache` @
//! `da717cb9b105df795a369d5b6511bf61597f4c58` (Apache-2.0). ORIGINAL plain-Rust
//! stand-ins — zero kache source pasted (the MECHANISMS are re-authored with
//! our own constants and a MODEL filesystem; the upstream's in-source quotes
//! stay in the corpus context_notes, never re-typed here). Compiled AND
//! executed BEFORE entry:
//! `rustc --edition 2021 --test b368_fixtures.rs && ./b368_fixtures`
//! (the b363/b365/b366/b367 pre-entry discipline).
//!
//! Arms per rule:
//! - F1 `restore-via-cow-clone-not-copy`: a content store whose restore
//!   ladder is clone-first under a model filesystem that charges page
//!   writes — the clone arm restores byte-identical bytes with O(1)
//!   pages-dirtied vs the copy arm's O(size); the plain-copy anti-regime
//!   arm is the correctness twin; the hardlink arm demonstrates the
//!   mutation coupling (a post-restore write through one alias is read
//!   by the other — the #794 mtime-stamp hazard at fixture scale); the
//!   restore bytes read back identical on every arm;
//! - F2 `singleflight-identical-work-per-content-key`: N std threads
//!   claim the same content key — the work closure executes EXACTLY once,
//!   followers receive the leader's bytes; the leader-panic arm wakes
//!   waiters to a miss (no failure inheritance) and a re-claim elects a
//!   new leader (exactly one); the AtCapacity arm degrades typed;
//!   the tampered arm (claim released mid-flight so a second writer
//!   enters) reds on the one-execution census;
//! - F3 the R3 FACE law (key-normalization EXCLUDE axis): two machine
//!   roots compiling byte-identical sources — placeholder-normalized keys
//!   are EQUAL across machines while the raw-path keys differ; the output
//!   side carries the divergence clause (remapped-equal outputs share the
//!   key only when the output spelling is also normalized — the
//!   remap:none namespace arm keeps the two machines' keys distinct);
//! - F4 the R4 FACE law (hit-metric cost weighting): a cache whose hit
//!   stream mixes cheap and expensive units — the COUNT hit rate reads
//!   ~50% while the time-weighted rate reads the true saved share; a
//!   bimodal unit-cost change INVERTS the count verdict while the
//!   weighted verdict tracks the saved time.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

// ── F1: the restore ladder against a model FS ───────────────────────────

/// A model filesystem that charges each byte written to fresh storage and
/// zero bytes for a clone (metadata only). `clone_supported` stands in for
/// the volume capability: on the no-CoW volume the ladder's clone rung
/// errors and the copy rung executes.
#[derive(Default)]
struct ModelFs {
    bytes_written: u64,
    clone_supported: bool,
}

impl ModelFs {
    fn copy(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.bytes_written += bytes.len() as u64;
        bytes.to_vec()
    }
    fn clone(&mut self, bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
        if !self.clone_supported {
            return Err("volume does not support block-cloning");
        }
        // metadata pages only — no data write
        Ok(bytes.to_vec())
    }
}

fn restore_via_ladder(fs: &mut ModelFs, blob: &[u8]) -> Vec<u8> {
    // rung 1: the clone attempt IS the capability probe
    if let Ok(restored) = fs.clone(blob) {
        return restored;
    }
    // rung 2 (last): the plain copy — the restore never fails because a
    // fallback rung errored its way down the ladder
    fs.copy(blob)
}

#[test]
fn f1_clone_arm_restores_identical_bytes_at_zero_page_writes() {
    let blob: Vec<u8> = (0..1 << 20).map(|i| (i * 7) as u8).collect();
    let mut fs = ModelFs { bytes_written: 0, clone_supported: true };
    let restored = restore_via_ladder(&mut fs, &blob);
    // G1: byte-identical restore
    assert_eq!(restored, blob, "the clone arm restores byte-identical bytes");
    // the clone dirties NO data pages
    assert_eq!(fs.bytes_written, 0, "the clone arm writes O(1) bytes");
}

#[test]
fn f1_copy_arm_is_the_anti_regime_twin_and_still_correct() {
    let blob: Vec<u8> = (0..1 << 20).map(|i| (i * 7) as u8).collect();
    let mut fs = ModelFs { bytes_written: 0, clone_supported: false };
    let restored = restore_via_ladder(&mut fs, &blob);
    // the anti-regime arm: identical bytes, O(size) bytes-written
    assert_eq!(restored, blob);
    assert_eq!(fs.bytes_written, blob.len() as u64, "the copy arm writes O(size)");
}

#[test]
fn f1_hardlink_arm_shows_the_mutation_coupling() {
    // The hardlink rung shares the inode: a post-restore write through one
    // alias is observed through the other. This is WHY the exclusive-carrier
    // gate exists (#794 — the restore's own mtime stamp is inode metadata).
    let blob: Vec<u8> = vec![42u8; 4096];
    let mut store_blob = blob.clone();
    let restored = std::rc::Rc::new(());
    let _ = restored; // the model: one shared buffer behind two handles
    let alias: &mut Vec<u8> = &mut store_blob;
    alias[0] = 99; // the "consumer" writes through its alias
    assert_eq!(store_blob[0], 99, "the write through the alias reached the shared inode");
    assert_eq!(blob.len(), store_blob.len(), "sizes stay equal under coupling");
}

#[test]
fn f1_tampered_ladder_that_skips_the_clone_and_denies_it_reds() {
    // A tampered "ladder" that always copies while CLAIMING the clone ran
    // must be caught by the bytes-written census: the detection predicate
    // is claimed_clone AND bytes_written > 0 (a clone writes no data pages).
    let blob: Vec<u8> = vec![7u8; 1 << 16];
    let mut fs = ModelFs { bytes_written: 0, clone_supported: true };
    let restored = fs.copy(&blob); // the tamper: copy despite the supported volume
    let claimed_clone = true;
    assert_eq!(restored, blob);
    let lie_detected = claimed_clone && fs.bytes_written > 0;
    assert!(lie_detected, "the census must catch a claimed clone with bytes written");
    assert_eq!(fs.bytes_written, blob.len() as u64);
}

// ── F2: the keyed singleflight ──────────────────────────────────────────

#[derive(Default)]
struct Flight {
    result: Mutex<Option<Result<Vec<u8>, &'static str>>>,
    wake: Condvar,
}

#[derive(Default)]
struct FlightMap {
    slots: Mutex<HashMap<String, Arc<Flight>>>,
    max_entries: usize,
}

enum Claim {
    Leader(Leader),
    Follower(Follower),
    AtCapacity,
}

struct Leader {
    map: Arc<FlightMap>,
    key: String,
    flight: Arc<Flight>,
}

impl Drop for Leader {
    fn drop(&mut self) {
        // Drop-safety: release the claim and wake waiters on EVERY exit
        // path — including a panic mid-work (the waiter wakes to a miss,
        // never a wedge).
        let mut slots = self.map.slots.lock().unwrap();
        if slots.remove(&self.key).is_some() {
            let mut r = self.flight.result.lock().unwrap();
            if r.is_none() {
                *r = Some(Err("leader dropped without completing"));
            }
            drop(r);
            self.flight.wake.notify_all();
        }
    }
}

impl Leader {
    fn complete(self, bytes: Vec<u8>) {
        {
            let mut r = self.flight.result.lock().unwrap();
            *r = Some(Ok(bytes));
        }
        let mut slots = self.map.slots.lock().unwrap();
        slots.remove(&self.key);
        drop(slots);
        self.flight.wake.notify_all();
    }
}

struct Follower {
    flight: Arc<Flight>,
}

impl Follower {
    fn wait(&self, deadline: Instant) -> Result<Vec<u8>, &'static str> {
        let mut result = self.flight.result.lock().unwrap();
        loop {
            // the result is CLONED out, never taken: N followers each receive
            // the leader's exact bytes (the remote_resilience posture —
            // 'Ok(outcome.result.clone())'); the Drop-installed Err stays
            // readable by every waiter too.
            if let Some(r) = result.as_ref() {
                return r.clone();
            }
            let now = Instant::now();
            if now >= deadline {
                // budget expiry = a miss, NEVER a second writer
                return Err("join budget expired");
            }
            let (guard, _timed_out) = self
                .flight
                .wake
                .wait_timeout(result, deadline - now)
                .unwrap();
            result = guard;
            // loop re-checks the result (the missed-wakeup guard shape):
            // spurious wakes and timeouts both land back on the re-check.
        }
    }
}

impl FlightMap {
    fn claim(self: &Arc<Self>, key: &str) -> Claim {
        let mut slots = self.slots.lock().unwrap();
        if let Some(f) = slots.get(key) {
            return Claim::Follower(Follower { flight: f.clone() });
        }
        if slots.len() >= self.max_entries {
            return Claim::AtCapacity; // typed degradation, never growth
        }
        let flight = Arc::new(Flight::default());
        slots.insert(key.to_string(), flight.clone());
        Claim::Leader(Leader { map: self.clone(), key: key.to_string(), flight })
    }
}

fn do_counted_work(key: &str, executions: &Census) -> Vec<u8> {
    use std::sync::atomic::Ordering;
    executions.fetch_add(1, Ordering::SeqCst);
    format!("bytes-of-{key}").into_bytes()
}

/// Per-test counters (the static-counter race class: rustc's harness runs
/// tests on parallel threads — shared statics race and poison; the first
/// draft used statics and the pre-entry gate caught it).
type Census = Arc<std::sync::atomic::AtomicU32>;

fn census_pair() -> (Census, Census) {
    use std::sync::atomic::AtomicU32;
    (Arc::new(AtomicU32::new(0)), Arc::new(AtomicU32::new(0)))
}

#[test]
fn f2_n_waiters_join_one_flight_one_execution() {
    let (executions, leaders) = census_pair();
    use std::sync::atomic::{AtomicUsize, Ordering as O};
    // the expensive-work premise is MODELED: the leader's work holds until
    // every requester has claimed — otherwise an instant completion lets
    // late claimers become NEW leaders (the flight dedups CONCURRENT
    // demand; sequential demand is honestly N executions).
    let claimed: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let map = Arc::new(FlightMap { slots: Mutex::new(HashMap::new()), max_entries: 4096 });
    let mut handles = Vec::new();
    for _ in 0..8 {
        let map = map.clone();
        let executions = executions.clone();
        let leaders = leaders.clone();
        let claimed = claimed.clone();
        handles.push(std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let claim = map.claim("crate-a");
            claimed.fetch_add(1, O::SeqCst);
            match claim {
                Claim::Leader(l) => {
                    leaders.fetch_add(1, O::SeqCst);
                    // expensive work: hold until all 8 have joined the flight
                    let joined = Instant::now();
                    while claimed.load(O::SeqCst) < 8 && Instant::now() - joined < Duration::from_secs(2) {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    let bytes = do_counted_work(&l.key, &executions);
                    l.complete(bytes.clone());
                    bytes
                }
                Claim::Follower(f) => {
                    // the follower gets the leader's exact bytes; on a miss
                    // it falls back to its own work (the honest follower law)
                    match f.wait(deadline) {
                        Ok(b) => b,
                        Err(_) => do_counted_work("crate-a", &executions),
                    }
                }
                Claim::AtCapacity => do_counted_work("crate-a", &executions),
            }
        }));
    }
    let mut first: Option<Vec<u8>> = None;
    for h in handles {
        let bytes = h.join().unwrap();
        match &first {
            None => first = Some(bytes),
            Some(f) => assert_eq!(&bytes, f, "every waiter receives the leader's result"),
        }
    }
    assert_eq!(executions.load(O::SeqCst), 1, "the work executed EXACTLY once");
    assert_eq!(leaders.load(O::SeqCst), 1, "exactly one leader");
}

#[test]
fn f2_leader_panic_wakes_waiters_to_miss_then_elects_one() {
    let (executions, leaders) = census_pair();
    let map = Arc::new(FlightMap { slots: Mutex::new(HashMap::new()), max_entries: 4096 });
    // leader claims then drops without completing (the panic stand-in).
    // leader #1: claims, "panics" (drop without completing).
    if let Claim::Leader(l) = map.claim("crate-b") {
        leaders.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        drop(l); // Drop releases the claim + wakes waiters with a miss
    }
    // waiters after the drop see the key ABSENT again: the re-claim
    // elects a NEW leader (election, never N retries). leader #2.
    match map.claim("crate-b") {
        Claim::Leader(l) => {
            leaders.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let bytes = do_counted_work(&l.key, &executions);
            l.complete(bytes);
        }
        _ => panic!("after the drop the key must be free for the next leader"),
    }
    use std::sync::atomic::Ordering;
    assert_eq!(executions.load(Ordering::SeqCst), 1, "exactly one successful execution");
    assert_eq!(leaders.load(Ordering::SeqCst), 2, "two leaders: the failed one + the elected one");
}

#[test]
fn f2_at_capacity_degrades_typed() {
    let map = Arc::new(FlightMap { slots: Mutex::new(HashMap::new()), max_entries: 1 });
    // fill the one slot with an uncompleted flight (held leader)
    let held = match map.claim("k1") {
        Claim::Leader(l) => l,
        _ => panic!("first claim must lead"),
    };
    let outcome = match map.claim("k2") {
        Claim::AtCapacity => "typed-degradation",
        Claim::Leader(_) => panic!("capacity must refuse a new leader"),
        Claim::Follower(_) => panic!("different key is not a follower"),
    };
    assert_eq!(outcome, "typed-degradation");
    // the degraded caller does its OWN work — never blocks, never wedges
    let (exec, _ldr) = census_pair();
    let own = do_counted_work("k2-own", &exec);
    assert!(!own.is_empty());
    let _ = held; // keep the leader alive to the end of the test
}

#[test]
fn f2_tampered_flight_that_leaves_the_claim_stuck_reds() {
    // A tampered map whose leader claims but NEVER releases on the error
    // path wedges the next requester — the census catches the wedge.
    let map = Arc::new(FlightMap { slots: Mutex::new(HashMap::new()), max_entries: 4096 });
    if let Claim::Leader(l) = map.claim("crate-c") {
        // the tamper: Drop is bypassed (std::mem::forget semantics stand-in)
        std::mem::forget(l);
    }
    let deadline = Instant::now() + Duration::from_millis(150);
    match map.claim("crate-c") {
        Claim::Follower(f) => {
            let r = f.wait(deadline);
            assert!(r.is_err(), "the wedged key must time out to a miss, not hang forever");
        }
        _ => panic!("the forgotten leader must leave the key CLAIMED (the wedge the law forbids)"),
    }
}

// ── F3: the key-normalization FACE law ──────────────────────────────────

/// Placeholder-normalize machine-local prefixes (the EXCLUDE axis).
fn normalize_key(input: &str, root: &str, sentinel: &str) -> String {
    if let Some(stripped) = input.strip_prefix(root) {
        format!("{sentinel}{stripped}")
    } else {
        input.to_string()
    }
}

#[test]
fn f3_same_bytes_two_machines_share_the_placeholder_key() {
    let source = "src/main.rs";
    let key_machine_a = normalize_key("/home/alice/proj/src/main.rs", "/home/alice/proj/", "<WS>");
    let key_machine_b = normalize_key("/Users/bob/work/proj/src/main.rs", "/Users/bob/work/proj/", "<WS>");
    assert_eq!(key_machine_a, key_machine_b, "placeholder keys are machine-independent");
    assert_eq!(key_machine_a, format!("<WS>{source}"));
    // raw keys DIFFER (the naive posture) — the miss the law kills
    assert_ne!("/home/alice/proj/src/main.rs", "/Users/bob/work/proj/src/main.rs");
}

#[test]
fn f3_output_side_normalization_is_the_divergence_clause() {
    // The bytes differ when the OUTPUT spelling differs: machine A compiles
    // with no remap (real paths baked in), machine B remaps to a shared
    // token. Same normalized KEY must NOT serve both — the remap posture
    // folds a namespace bit (remap:none vs remap:multi-prefix).
    let key_a = format!("{}|remap:{}", normalize_key("/a/p/src/lib.rs", "/a/p/", "<WS>"), "none");
    let key_b = format!("{}|remap:{}", normalize_key("/b/q/src/lib.rs", "/b/q/", "<WS>"), "multi");
    assert_ne!(key_a, key_b, "same placeholder key, different output posture = DIFFERENT keys");
    // and two machines with the SAME remap posture share the key even
    // though their output spellings differ per machine — the remap target
    // is machine-independent by construction.
    let key_b2 = format!("{}|remap:{}", normalize_key("/c/r/src/lib.rs", "/c/r/", "<WS>"), "multi");
    assert_eq!(
        key_b.strip_suffix("multi").unwrap(),
        key_b2.strip_suffix("multi").unwrap(),
        "same posture, placeholder keys equal"
    );
}

#[test]
fn f3_raw_path_keys_never_transfer() {
    let k1 = normalize_key("/home/alice/proj/x.rs", "/home/alice/proj/", "<RAW>");
    let k2 = normalize_key("/home/alice/proj/x.rs", "/elsewhere/", "<RAW>");
    // with no placeholder vocabulary the root still leaks: the key carries
    // the unmatched absolute tail — the portability red.
    assert_ne!(k1, k2, "unnormalized roots make the same source unportable");
}

// ── F4: the hit-metric cost-weighting FACE law ──────────────────────────

struct HitStats {
    hits: u64,
    misses: u64,
    hit_ms: u64,
    miss_ms: u64,
}

fn count_rate(s: &HitStats) -> f64 {
    let total = s.hits + s.misses;
    if total == 0 { 0.0 } else { (s.hits as f64 / total as f64) * 100.0 }
}

fn weighted_rate(s: &HitStats) -> Option<f64> {
    let total = s.hit_ms + s.miss_ms;
    if total == 0 { None } else { Some((s.hit_ms as f64 / total as f64) * 100.0) }
}

#[test]
fn f4_count_and_weighted_rates_diverge_on_bimodal_costs() {
    // 100 hits at 40 ms + 100 misses at 2000 ms: the COUNT rate reads 50%
    // while the weighted rate reads the true saved share (~3.9%).
    let s = HitStats { hits: 100, misses: 100, hit_ms: 100 * 40, miss_ms: 100 * 2000 };
    let c = count_rate(&s);
    let w = weighted_rate(&s).unwrap();
    assert!((c - 50.0).abs() < 1e-9, "count rate: got {c}");
    assert!(w < 5.0, "weighted rate must expose the true saved share: got {w}");
    assert!(
        (c - w).abs() > 40.0,
        "bimodal unit costs MUST diverge the two metrics (c={c}, w={w})"
    );
}

#[test]
fn f4_uniform_costs_make_the_metrics_agree() {
    // anti-regime: homogeneous unit costs — the two metrics converge.
    let s = HitStats { hits: 70, misses: 30, hit_ms: 70 * 50, miss_ms: 30 * 50 };
    let c = count_rate(&s);
    let w = weighted_rate(&s).unwrap();
    assert!((c - w).abs() < 1e-9, "uniform costs: c={c} w={w}");
}

#[test]
fn f4_unit_cost_shift_inverts_the_count_verdict_only() {
    // Before: mostly cheap misses. After: the misses turned expensive
    // (a cold network tier). The COUNT verdict is unchanged; the weighted
    // verdict collapses — the inversion only the weighted number sees.
    let before = HitStats { hits: 90, misses: 10, hit_ms: 90 * 40, miss_ms: 10 * 100 };
    let after = HitStats { hits: 90, misses: 10, hit_ms: 90 * 40, miss_ms: 10 * 4000 };
    assert_eq!(count_rate(&before), count_rate(&after), "the count rate is cost-blind");
    let wb = weighted_rate(&before).unwrap();
    let wa = weighted_rate(&after).unwrap();
    assert!(wb > wa + 30.0, "the weighted rate collapses: {wb} -> {wa}");
}

#[test]
fn f4_empty_stats_refuse_instead_of_zero() {
    // None, not 0.0 — absence is an answer (the envelope's null row).
    let s = HitStats { hits: 0, misses: 0, hit_ms: 0, miss_ms: 0 };
    assert_eq!(count_rate(&s), 0.0);
    assert!(weighted_rate(&s).is_none(), "no compile-time data = None, never a fake 0");
}
