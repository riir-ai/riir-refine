//! Batch 343 fixture laws — the sccache cache-layer RE-distill (the
//! tiered-hit backfill family) as ONE standalone file.
//!
//! ORIGINAL plain-Rust hierarchical-cache reproductions — zero sccache code
//! pasted (the source is Apache-2.0; the fixtures are original syntheses of
//! the laws). Compiled AND executed BEFORE entry: `rustc --edition 2021
//! --test b343_fixtures.rs` (the B336/B338/B339/B340/B341/B342 discipline).
//! Each rule's ref/opt arms in `entries_69.rs` embed the same executed code.
//!
//! Laws under test:
//! F1 `raw-bytes-with-hit-backfill-once` — (a) parse identity across arms;
//! (b) the read census: one physical read serves the parse AND the fill
//! (two reads in the naive arm); (c) answer-before-fill (the detached fill
//! lands AFTER the hit answered); (d) fastest-tier skip (a level-0 hit
//! enqueues no fill); (e) the capability-gated skip degrades to a logged
//! skip, never a second read; (f) the stub arm (a synchronous blocking
//! fill) FAILS the answer-before-fill pin.
//! F2 `vanished-entry-tolerant-eviction-panic-else` — (a) the kind split:
//! NotFound tolerates (PeerEvicted), any other error panics loud; (b) the
//! all-fatal naive arm panics on the benign peer event; (c) the all-swallowed
//! naive arm hides corruption (its ledger over-counts freed bytes);
//! (d) the tampered predicate (NotFound as corruption) fails the peer-race
//! input.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

// ═══════════════════════════════════════════════════════════════════════
// F1 — raw-bytes-with-hit-backfill-once
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct Entry {
    payload: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
struct Parsed(Vec<(String, u64)>);

fn parse(data: &[u8]) -> Parsed {
    let mut records = Vec::new();
    for c in data.chunks_exact(16) {
        let name = String::from_utf8_lossy(&c[..8]).into_owned();
        let value = u64::from_le_bytes(c[8..16].try_into().unwrap());
        records.push((name, value));
    }
    Parsed(records)
}

/// physical-read census: counts byte-buffer CLONES off the remote store
#[derive(Default)]
struct ReadCensus(Cell<usize>);

impl ReadCensus {
    fn read(&self, data: &[u8]) -> Vec<u8> {
        self.0.set(self.0.get() + 1);
        data.to_vec()
    }
}

struct NaiveCache {
    remote: Vec<(String, Entry)>,
    fast: Mutex<HashMap<String, Entry>>,
    census: ReadCensus,
}

impl NaiveCache {
    fn get(&self, key: &str) -> Option<Parsed> {
        if let Some(entry) = self.fast.lock().unwrap().get(key) {
            return Some(parse(&entry.payload));
        }
        let pos = self.remote.iter().position(|(k, _)| k == key)?;
        let payload = self.census.read(&self.remote[pos].1.payload); // read #1
        let parsed = parse(&payload);
        let fill = self.census.read(&self.remote[pos].1.payload); // read #2
        self.fast.lock().unwrap().insert(key.to_string(), Entry { payload: fill });
        Some(parsed)
    }
}

struct TieredCache {
    remote: Vec<(String, Entry)>,
    fast: Mutex<HashMap<String, Entry>>,
    fills: Mutex<Vec<(String, Rc<Vec<u8>>)>>,
    census: ReadCensus,
    /// the slower tier advertises raw capability; false models the
    /// capability-gated skip arm
    remote_raw_capable: bool,
}

impl TieredCache {
    fn get_raw(&self, key: &str) -> Option<Rc<Vec<u8>>> {
        let pos = self.remote.iter().position(|(k, _)| k == key)?;
        let bytes = self.census.read(&self.remote[pos].1.payload);
        Some(Rc::new(bytes))
    }

    /// the source's get_with_raw shape: (hit, shared bytes), ONE read
    fn get_with_raw(&self, key: &str) -> Option<(Parsed, Rc<Vec<u8>>)> {
        if let Some(entry) = self.fast.lock().unwrap().get(key) {
            // level-0 hit: NOTHING faster to fill - the skip arm
            return Some((parse(&entry.payload), Rc::new(entry.payload.clone())));
        }
        let data = self.get_raw(key)?;
        let parsed = parse(&data);
        if self.remote_raw_capable {
            self.fills.lock().unwrap().push((key.to_string(), Rc::clone(&data)));
        }
        Some((parsed, data))
    }

    fn get(&self, key: &str) -> Option<Parsed> {
        self.get_with_raw(key).map(|(p, _)| p)
    }

    fn drain_fills(&self) -> usize {
        let mut fills = self.fills.lock().unwrap();
        let n = fills.len();
        for (key, bytes) in fills.drain(..) {
            self.fast.lock().unwrap().insert(key, Entry { payload: (*bytes).clone() });
        }
        n
    }
}

fn seeded_cache() -> (NaiveCache, TieredCache, Parsed) {
    let mut payload = Vec::new();
    for i in 0..4u64 {
        let mut name = format!("rec{i:05}").into_bytes();
        name.resize(8, b'_');
        payload.extend_from_slice(&name);
        payload.extend_from_slice(&(i * 100).to_le_bytes());
    }
    let expected = parse(&payload);
    let naive = NaiveCache {
        remote: vec![("k".to_string(), Entry { payload: payload.clone() })],
        fast: Mutex::new(HashMap::new()),
        census: ReadCensus::default(),
    };
    let tiered = TieredCache {
        remote: vec![("k".to_string(), Entry { payload })],
        fast: Mutex::new(HashMap::new()),
        fills: Mutex::new(Vec::new()),
        census: ReadCensus::default(),
        remote_raw_capable: true,
    };
    (naive, tiered, expected)
}

#[test]
fn f1_parse_identity_across_arms() {
    let (naive, tiered, expected) = seeded_cache();
    let a = naive.get("k").unwrap();
    let b = tiered.get("k").unwrap();
    assert_eq!(a, b);
    assert_eq!(b, expected, "both arms parse the same records");
}

#[test]
fn f1_one_physical_read_serves_parse_and_fill() {
    let (_, tiered, _) = seeded_cache();
    tiered.get("k").unwrap();
    assert_eq!(tiered.census.0.get(), 1, "the fixed arm reads ONCE (the parse views the shared bytes)");
    tiered.drain_fills();
    assert_eq!(tiered.census.0.get(), 1, "the fill consumes the SAME bytes - no second read");

    let (naive, _, _) = seeded_cache();
    naive.get("k").unwrap();
    assert_eq!(naive.census.0.get(), 2, "the naive arm pays read #1 (parse) + read #2 (fill)");
}

#[test]
fn f1_answer_before_fill() {
    let (_, tiered, expected) = seeded_cache();
    let parsed = tiered.get("k").unwrap();
    // the fill is still DETACHED: the outbox holds it, the hit already
    // answered
    assert_eq!(tiered.fills.lock().unwrap().len(), 1, "the fill is queued, not yet applied");
    assert_eq!(parsed, expected, "the caller has the parsed value BEFORE the fill drains");
    tiered.drain_fills();
    assert!(tiered.fast.lock().unwrap().contains_key("k"), "the drain applies the promotion");
}

#[test]
fn f1_fastest_tier_hit_skips_backfill() {
    let (_, tiered, _) = seeded_cache();
    tiered.get("k").unwrap(); // slow-tier hit: the fill is queued
    tiered.drain_fills(); // the background worker applies the promotion
    tiered.census.0.set(0);
    let _ = tiered.get("k").unwrap(); // NOW a level-0 hit
    assert_eq!(tiered.census.0.get(), 0, "the fastest tier hit reads nothing from the slower tier");
    assert!(tiered.fills.lock().unwrap().is_empty(), "nothing faster to fill - no backfill entry");
}

#[test]
fn f1_capability_gated_skip_is_never_a_second_read() {
    let (_, mut tiered, expected) = seeded_cache();
    tiered.remote_raw_capable = false;
    let parsed = tiered.get("k").unwrap();
    assert_eq!(parsed, expected, "the hit still answers");
    assert_eq!(tiered.census.0.get(), 1, "one read");
    assert!(tiered.fills.lock().unwrap().is_empty(), "no raw capability -> logged skip, no fill");
}

// The stub arm: a fill that BLOCKS the read path (answers only after the
// promotion applied) must FAIL the answer-before-fill law.
fn blocking_fill_get(cache: &TieredCache, key: &str) -> Option<Parsed> {
    if let Some(entry) = cache.fast.lock().unwrap().get(key) {
        return Some(parse(&entry.payload));
    }
    let pos = cache.remote.iter().position(|(k, _)| k == key)?;
    let bytes = cache.census.read(&cache.remote[pos].1.payload);
    let parsed = parse(&bytes);
    // THE STUB: apply the fill SYNCHRONOUSLY before answering
    cache.fast.lock().unwrap().insert(key.to_string(), Entry { payload: bytes });
    Some(parsed)
}

#[test]
fn f1_stub_blocking_fill_fails_the_law() {
    let (_, tiered, expected) = seeded_cache();
    let parsed = blocking_fill_get(&tiered, "k").unwrap();
    // the stub's applied fill leaves the outbox empty at answer time - the
    // honest arm has the fill QUEUED at answer time. The divergence IS the
    // stub's failure.
    let honest_queued = {
        let (_, honest, _) = seeded_cache();
        honest.get("k").unwrap();
        let queued = honest.fills.lock().unwrap().len();
        queued
    };
    assert_eq!(honest_queued, 1, "the honest arm has the fill queued at answer time");
    assert_ne!(
        tiered.fills.lock().unwrap().len(),
        honest_queued,
        "the blocking stub must disagree with the honest arm's answer-time state"
    );
    assert_eq!(parsed, expected);
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — vanished-entry-tolerant-eviction-panic-else
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug)]
enum EvictOutcome {
    Removed,
    PeerEvicted,
}

fn evict_one(path: &std::path::Path) -> EvictOutcome {
    match std::fs::remove_file(path) {
        Ok(()) => EvictOutcome::Removed,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("debug: entry vanished before our eviction: {path:?}");
            EvictOutcome::PeerEvicted
        }
        Err(e) => panic!("eviction failed on our own entry {path:?}: {e} ({:?})", e.kind()),
    }
}

fn evict_all(paths: &[std::path::PathBuf]) -> usize {
    let mut peer = 0;
    for p in paths {
        if let EvictOutcome::PeerEvicted = evict_one(p) {
            peer += 1;
        }
    }
    peer
}

fn naive_fatal(paths: &[std::path::PathBuf]) {
    for p in paths {
        std::fs::remove_file(p).expect("eviction must succeed");
    }
}

fn naive_silent(paths: &[std::path::PathBuf]) -> usize {
    let mut freed = 0;
    for p in paths {
        if std::fs::remove_file(p).is_ok() {
            freed += 1;
        }
    }
    freed
}

fn tmp_paths(n: usize, tag: &str) -> Vec<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("b343_fixtures_{}_{tag}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    (0..n)
        .map(|i| {
            let p = dir.join(format!("entry_{i}.cache"));
            std::fs::write(&p, b"payload").unwrap();
            p
        })
        .collect()
}

#[test]
fn f2_peer_vanished_entry_tolerates() {
    let paths = tmp_paths(2, "peer");
    // a peer daemon evicted entry 0 before us
    assert!(std::fs::remove_file(&paths[0]).is_ok());
    let peer_count = evict_all(&paths);
    assert_eq!(peer_count, 1, "the vanished entry classifies as PeerEvicted - the loop continues");
    assert!(std::fs::remove_file(&paths[1]).is_ok() || !paths[1].exists(), "entry 1 still evicted by us");
    let _ = std::fs::remove_dir_all(paths[0].parent().unwrap());
}

#[test]
#[should_panic(expected = "eviction failed on our own entry")]
fn f2_self_caused_error_panics_loud() {
    // a DIRECTORY at the victim path: remove_file fails with a NON-NotFound
    // kind (on Windows: PermissionDenied / on unix: IsADirectory) - our own
    // ledger corruption class
    let dir = std::env::temp_dir().join(format!("b343_fixtures_{}_corrupt", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let victim = dir.join("entry_0.cache");
    std::fs::create_dir_all(&victim).unwrap(); // a dir where a FILE is expected
    let _ = evict_all(&[victim]);
    panic!("the honest classifier must have panicked BEFORE this line");
}

#[test]
fn f2_naive_fatal_breaks_on_the_peer_event() {
    let paths = tmp_paths(1, "naivefatal");
    std::fs::remove_file(&paths[0]).unwrap(); // peer evicts first
    std::panic::catch_unwind(|| naive_fatal(&paths))
        .expect_err("the all-fatal arm MUST panic on the benign peer event - the divergence IS the defect");
    let _ = std::fs::remove_dir_all(paths[0].parent().unwrap());
}

#[test]
fn f2_naive_silent_hides_corruption() {
    let paths = tmp_paths(2, "naivesilent");
    // a peer evicts entry 0 (benign); entry 1 is replaced by a DIRECTORY
    // (our ledger's real corruption)
    std::fs::remove_file(&paths[0]).unwrap();
    let corrupt = &paths[1];
    std::fs::remove_file(corrupt).unwrap();
    std::fs::create_dir_all(corrupt).unwrap();
    let freed = naive_silent(&paths);
    assert_eq!(freed, 0, "the silent arm hides BOTH events - the peer race AND the corruption");
    // the honest arm tolerates the peer and PANICS on the corruption:
    let outcome = std::panic::catch_unwind(|| evict_all(&paths));
    assert!(outcome.is_err(), "the honest arm fails loud on the self-caused error");
    let _ = std::fs::remove_dir_all(paths[0].parent().unwrap());
}

#[test]
fn f2_tampered_predicate_fails_the_peer_input() {
    // the tamper: NotFound treated as corruption (panic) - must diverge
    // from the honest classifier on the peer-race input
    fn tampered(paths: &[std::path::PathBuf]) {
        for p in paths {
            match std::fs::remove_file(p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => panic!("tampered: NotFound is corruption"),
                Err(e) => panic!("tampered: {e}"),
            }
        }
    }
    let paths = tmp_paths(1, "tamper");
    std::fs::remove_file(&paths[0]).unwrap(); // peer evicts first
    let outcome = std::panic::catch_unwind(|| tampered(&paths));
    assert!(outcome.is_err(), "the tampered predicate panics where the honest arm tolerates");
    let _ = std::fs::remove_dir_all(paths[0].parent().unwrap());
}
