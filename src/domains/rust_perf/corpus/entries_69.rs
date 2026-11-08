//! Entries slice — Batch 343 (2026-11-08): two `rust_perf` rules from the
//! sccache cache-layer RE-distill (Plan 355 / Research 387) — the tiered-hit
//! backfill family, the files Batch 324 did not mine (zero file overlap with
//! `entries_58`; the dedup KILL of candidate 2 is recorded in-rule below and
//! in the research note's landing addendum).
//!
//! Source: `mozilla/sccache` @ `c1d7204a0f14909c83a9cd983646d5c09149cddd`
//! (origin/main tip 2026-11-07, Apache-2.0, license verified in-tree; every
//! quote byte-verified in the `.raw` clone HEAD-equal to this sha this
//! session). **Pin correction carried:** Research 387 recorded the pin as
//! `c1d7204f90a9ed1d…` — that object does not exist server-side ("not our
//! ref"); the real commit matching the cited short prefix `c1d7204` is the
//! sha above. All line references re-verified against it at T0
//! (`multilevel.rs` :491-526 `backfill_raw_hit`, :571-601 `get`, `disk.rs`
//! :126-136 `get_with_raw`, `lru_disk_cache/mod.rs` :257-288 `make_space`,
//! :367-409 `prepare_add`/`commit`, :192-193 `size()`, :75-95 `get_all_files`,
//! :300-331 `insert_by`/`insert_bytes`).
//!
//! Fixtures are ORIGINAL plain-Rust stand-ins (hierarchical in-memory cache
//! reproductions; zero sccache code pasted), standalone rustc-compiled AND
//! executed before landing. Batch 343 / Plan 355 / Research 387.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── the tiered-hit backfill law: one physical read serves the parse
        // AND the faster-tier fill; the fastest tier never backfills ────────
        PerfRule {
            rule_id: "raw-bytes-with-hit-backfill-once".to_string(),
            name: "A hierarchical cache whose hit at a SLOWER tier must also fill the faster tiers reads the entry ONCE in a form both consumers share - the parse views the raw bytes while the same bytes handle rides to the backfill (get_with_raw returns (Hit, Bytes); the parse wraps a Cursor over data.clone()) - the write-back is spawned DETACHED so the read path never waits on it, a hit at the FASTEST tier skips backfill entirely (there is nothing faster to fill), only levels with a faster neighbor request the raw form (idx > 0 gets raw, idx == 0 plain get - the single-tier deployment never pays the raw-copy arm), and a level without raw capability degrades to a logged skip - never a second physical read".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: a hit at a slower tier is parsed and re-fetched for the fill.
// The read path pays TWO physical reads for one logical hit, and the
// caller blocks on the faster-tier write before answering.

#[derive(Clone)]
struct Entry {
    payload: Vec<u8>,
}

fn parse(data: &[u8]) -> Parsed {
    let mut records = Vec::new();
    for c in data.chunks_exact(16) {
        let name = String::from_utf8_lossy(&c[..8]).into_owned();
        let value = u64::from_le_bytes(c[8..16].try_into().unwrap());
        records.push((name, value));
    }
    Parsed(records)
}

struct TwoTierCache {
    remote: std::sync::Mutex<Vec<(String, Entry)>>,
    fast: std::sync::Mutex<std::collections::HashMap<String, Entry>>,
}

impl TwoTierCache {
    /// A hit at `remote` pays: read #1 (the parse fetch), then read #2
    /// (the backfill fetch), and the caller WAITS for the fast-tier
    /// write before the hit is answered.
    fn get(&self, key: &str) -> Option<Parsed> {
        if let Some(entry) = self.fast.lock().unwrap().get(key) {
            return Some(parse(&entry.payload));
        }
        let mut remote = self.remote.lock().unwrap();
        let pos = remote.iter().position(|(k, _)| k == key)?;
        let payload = remote[pos].1.payload.clone(); // physical read #1
        let parsed = parse(&payload);
        // read #2: the entry is fetched AGAIN to fill the faster tier
        let fill = remote[pos].1.payload.clone();
        // and the caller blocks on the write-back
        self.fast.lock().unwrap().insert(key.to_string(), Entry { payload: fill });
        Some(parsed)
    }
}
"#.to_string(),
            fixed_code: r#"// FIXED: one raw read serves BOTH consumers - the parse views the bytes,
// the same bytes handle rides to a DETACHED fast-tier fill; the fastest
// tier skips backfill (nothing faster to fill); only tiers WITH a faster
// neighbor request the raw form.

#[derive(Clone)]
struct Entry {
    payload: Vec<u8>,
}

fn parse_view(data: &[u8]) -> Parsed {
    // the SAME view-parsing as the buggy arm - the law is not the parser,
    // it is that the parse consumes a VIEW of the shared bytes
    let mut records = Vec::new();
    for c in data.chunks_exact(16) {
        let name = String::from_utf8_lossy(&c[..8]).into_owned();
        let value = u64::from_le_bytes(c[8..16].try_into().unwrap());
        records.push((name, value));
    }
    Parsed(records)
}

#[derive(Debug, PartialEq)]
struct Fill {
    key: String,
    bytes: std::rc::Rc<Vec<u8>>,
}

struct TwoTierCache {
    remote: std::sync::Mutex<Vec<(String, Entry)>>,
    fast: std::sync::Mutex<std::collections::HashMap<String, Entry>>,
    /// the detached-fill outbox: the read path hands the bytes here and
    /// returns; a background worker drains it (modeled, not spawned)
    fills: std::sync::Mutex<Vec<Fill>>,
}

impl TwoTierCache {
    fn get_raw(&self, key: &str) -> Option<std::rc::Rc<Vec<u8>>> {
        let remote = self.remote.lock().unwrap();
        remote.iter().find(|(k, _)| k == key).map(|(_, e)| std::rc::Rc::new(e.payload.clone()))
    }

    /// `get_with_raw` shape: the hit AND the shared bytes, ONE read.
    fn get_with_raw(&self, key: &str) -> Option<(Parsed, std::rc::Rc<Vec<u8>>)> {
        // fast tier first: a hit at level 0 returns WITHOUT any backfill -
        // there is nothing faster to fill (the source's hit_level == 0 arm)
        if let Some(entry) = self.fast.lock().unwrap().get(key) {
            return Some((parse_view(&entry.payload), std::rc::Rc::new(entry.payload.clone())));
        }
        // slower tier: read ONCE, parse a view, hand the SAME bytes to the
        // detached fill (the source: get_with_raw -> Cursor view + Bytes
        // handle; tokio::spawn so the read path never waits)
        let data = self.get_raw(key)?;
        let parsed = parse_view(&data);
        self.fills.lock().unwrap().push(Fill { key: key.to_string(), bytes: std::rc::Rc::clone(&data) });
        Some((parsed, data))
    }

    fn get(&self, key: &str) -> Option<Parsed> {
        // only the slower tier's lookup requests the raw form; the level
        // without a faster neighbor would call the plain get (idx == 0)
        self.get_with_raw(key).map(|(parsed, _)| parsed)
    }

    /// the detached worker's drain: the fill lands AFTER the hit answered
    fn drain_fills(&self) -> usize {
        let mut fills = self.fills.lock().unwrap();
        let n = fills.len();
        for fill in fills.drain(..) {
            self.fast.lock().unwrap().insert(
                fill.key,
                Entry { payload: (*fill.bytes).clone() },
            );
        }
        n
    }
}
"#.to_string(),
            context_notes: Some(r##"Grade B — decision-content, suggestion-only, NotBenchable-by-domain (criterion 1 fails DOMAIN-side: the win is I/O read-amplification — one physical read serving two consumers — plus read-path latency never paying the write-back; neither axis is hostable in the CPU Pareto validator; the B324/B136 standard makes decision-content entries first-class). SOURCE (mozilla/sccache @ c1d7204a0f14909c83a9cd983646d5c09149cddd, origin/main tip 2026-11-07, Apache-2.0 — the PIN CORRECTION: Research 387 recorded c1d7204f90a9ed1d, an object that does not exist server-side; every quote below byte-verified at the real sha this session). THE MECHANISM, byte-verified: disk.rs:126-136 get_with_raw returns (Cache, Option<Bytes>) and parses a VIEW over the shared bytes — 'let hit = CacheRead::from(std::io::Cursor::new(data.clone()))?' + 'Ok((Cache::Hit(hit), Some(data)))' — the parse and the backfill consume the SAME read. multilevel.rs:491-526 backfill_raw_hit: the fastest-tier hit skips the fill entirely ('if hit_level == 0 { return; }' at :492-494), the write-back is spawned DETACHED ('tokio::spawn' :508) so the read path never waits, and the fill goes to EVERY faster tier ('for backfill_idx in 0..hit_level'), each failure debug-logged, never surfaced. multilevel.rs:571-601 get: only levels with a faster neighbor request raw — 'let cache_result = if idx > 0 { match level.get_with_raw(key).await } else { level.get(key).await };' — so the single-tier deployment never pays the raw-copy arm; the in-code comment states the law verbatim: 'Raw bytes obtained above are reused for backfilling; no second read is needed for a raw-capable level.' — and the capability-gated arm is explicit: a level without get_raw support logs 'does not support get_raw(), skipping backfill' and degrades to a skip, NEVER a second physical read. REGIME: any multi-tier cache (memory tiers, disk+remote chains, L1/L2 handles) whose slower-tier hits must propagate to faster tiers; any read path whose result feeds both a consumer and a promotion write. ANTI-REGIMES (carried in-rule): single-tier caches — the idx == 0 arm is the law's own carve-out, requesting raw there is pure copy cost; entries too big to duplicate in memory — the same-bytes trick is unavailable and the streaming copy must go disk-to-disk (the bounded fix-space states this); write-through architectures where the faster tier is authoritative (nothing to backfill). KINSHIPS (read + ruled): reserve-then-commit-capacity-accounting (entries_58, B324, same source) owns the WRITE-side capacity ledger of the same cache stack — THIS owns the READ-side propagation; tier-write-defers-to-urgent-reads-except-a-read-behind-it (entries_42, B270) owns tier READ/WRITE scheduling within a worker queue — different axis (which waits for which vs what a hit fills); the memoization family owns compute-once laws — THIS is read-once (the deduplicated work is I/O, not computation); empty-first/save-before-overwrite (entries_28) is victim order, unrelated. NOVEL (T1 greps at landing HEAD): backfill / write_back / writeback / propagate-dot-hit over rust_perf+kernel — the gpu_102 writeback hits are page-cache gauge prose (memory categories), zero owner of the tier-hit-backfill axis. DEDUP KILL (this batch, recorded as the gate working): candidate 2 'advertised-size-staged-write-reconciles-at-commit' (Research 387 Law 2) is a DUPLICATE of reserve-then-commit-capacity-accounting (entries_58) — that rule already carries the full prepare_add/commit law verbatim from the SAME source (reserve on the advertised size, pending_size counts against capacity, commit re-measures real_size and evicts only the saturating_sub delta, exact-size writes are the anti-regime); Research 387's dedup note mis-read it as a 'scan-time container rule'. The mtime-sorted-walk KILL also held (get_all_files :75-95 walk+sort-by-mtime = mtime-encoded-lru-recency-persistence's boot-init half, the B324 positive instance). Candidate 4 per-tier-lockfree-stat-slots dropped as expected (the corpus's debug-stat-full-scan-out-of-hot-path owns the hot-path-stats axis; per-TIER keying is a parameterization). Suggestion-only (the defect needs the tier topology + the shared-bytes knowledge; a parse-only proposer cannot see either). FIXTURE PROVENANCE: original machinery (hierarchical in-memory cache reproduction, zero sccache code) — the buggy arm pays two physical reads + blocks on the fill; the fixed arm reads once, parses a view, hands the same bytes to a detached-fill outbox, skips the fill on a fastest-tier hit, and the drain happens after the hit answered; the test pins one-read (clone count), answer-before-fill (the outbox is non-empty AFTER get returns and the parse is available before drain), fastest-tier-skip (no fill entry from a level-0 hit), and parse-identity (both arms parse identical records). Cite: mozilla/sccache @ c1d7204a — src/cache/disk.rs:126-136 (get_with_raw), src/cache/multilevel.rs:491-526 (backfill_raw_hit: the hit_level==0 skip, the detached spawn, the 0..hit_level fan), :571-601 (get: the idx split + the no-second-read comment + the capability skip). Batch 343 / Plan 355 / Research 387."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None, // NotBenchable-by-domain: the win is I/O read-amplification (one physical read serving two consumers), not a CPU clock claim; the fixtures carry the read-count/answer-before-fill pins
        },
        // ── the shared-directory eviction law: peer-caused NotFound tolerates,
        // self-caused corruption panics loud ────────────────────────────────
        PerfRule {
            rule_id: "vanished-entry-tolerant-eviction-panic-else".to_string(),
            name: "An eviction loop over a SHARED directory (multiple daemons, one cache root) races other owners' evictions: fs::remove_file failing NotFound means a peer evicted the entry first - TOLERATE and continue (debug log); ANY other error is our own state corruption - PANIC loud; a correctness invariant whose violation may be caused by a PEER must classify peer-caused failures (tolerate+continue) from self-caused ones (fail loud) instead of flattening both into one error arm - flattening hides real corruption, failing on peers breaks the shared deployment".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE arm A: every removal failure is fatal - a peer's concurrent
// eviction (the shared-deployment norm) crashes us.
fn evict_all_or_panic(victims: &[std::path::PathBuf]) {
    for path in victims {
        // a peer daemon evicted this entry first -> we panic on a
        // BENIGN concurrent removal: the shared deployment breaks
        std::fs::remove_file(path).expect("eviction must succeed");
    }
}

// NAIVE arm B: every removal failure is swallowed - real corruption hides.
fn evict_all_or_silence(victims: &[std::path::PathBuf]) {
    for path in victims {
        // our own ledger named this file, it is UNREADABLE, permissions
        // are wrong, the FS is failing - logged and forgotten: corruption
        // hides until the budget accounting drifts
        let _ = std::fs::remove_file(path);
    }
}
"#.to_string(),
            fixed_code: r#"// FIXED: the KIND SPLIT - NotFound is a peer's eviction (tolerate +
// debug log), anything else is self-caused corruption (panic loud).

#[derive(Debug)]
enum EvictOutcome {
    Removed,
    /// a peer evicted the entry first - benign in a shared deployment
    PeerEvicted,
}

fn evict_one(path: &std::path::Path) -> EvictOutcome {
    match std::fs::remove_file(path) {
        Ok(()) => EvictOutcome::Removed,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // debug-log + continue: 'Sometimes the file has already been
            // removed ... when the max cache size has been reached'
            eprintln!("debug: entry vanished before our eviction: {path:?}");
            EvictOutcome::PeerEvicted
        }
        Err(e) => {
            // ANY other error is OUR state corruption - fail loud
            panic!("eviction failed on our own entry {path:?}: {e} ({:?})", e.kind())
        }
    }
}

fn evict_all(victims: &[std::path::PathBuf]) -> usize {
    let mut peer_evicted = 0;
    for path in victims {
        if let EvictOutcome::PeerEvicted = evict_one(path) {
            peer_evicted += 1;
        }
    }
    peer_evicted
}
"#.to_string(),
            context_notes: Some(r##"Grade B− — decision-content, suggestion-only, NotBenchable-by-domain (correctness class: the invariant is eviction-ledger consistency under concurrent owners; the failure modes are a crash on a benign peer event or silent corruption, neither a clock claim). SOURCE (mozilla/sccache @ c1d7204a0f14909c83a9cd983646d5c09149cddd, origin/main tip 2026-11-07, Apache-2.0 — pin correction carried: Research 387's c1d7204f90a9ed1d does not exist server-side; quotes byte-verified at the real sha this session). THE MECHANISM, byte-verified (src/lru_disk_cache/mod.rs:257-288 make_space, the eviction loop): 'fs::remove_file(&remove_path).unwrap_or_else(|e| {' with the kind split — NotFound arm, verbatim: 'Sometimes the file has already been removed' + 'this seems to happen when the max cache size has been reached' + the in-tree issue cite 'github.com/mozilla/sccache/issues/2092' + the debug log 'Error removing file from cache as it was not found' — tolerate + log + continue the eviction loop; ANY other error arm: 'panic!' with 'Error removing file from cache' + the path + the error + its kind — fail loud on self-caused corruption. The in-code TODO frames the invariant: 'check that files are removable during init, so that this is only due to outside interference' — outside interference IS the benign class by construction. THE GENERAL LAW: a correctness invariant whose violation may be CAUSED BY A PEER (shared directory, shared registry, shared queue, multi-writer store) must classify peer-caused failures from self-caused ones — flattening both into one error arm picks poison: all-fatal crashes the deployment on the peer's normal operation (the shared-deployment norm), all-swallowed hides our own ledger corruption until budget accounting drifts. The classification predicate is the error KIND, not the message. REGIME: eviction/GC loops over shared storage (multi-daemon cache roots, shared tmp dirs, multi-writer object stores); any cleanup pass whose targets a peer may legitimately have removed already; distributed ledger GC. ANTI-REGIMES: single-owner directories (a NotFound IS our corruption — panic on it, no peer exists); caches with leader-elected eviction (only the leader evicts — the race cannot occur); remote stores whose delete is idempotent-by-API (404-already-gone is the API's own tolerance). KINSHIPS (read + ruled at T1): persistent-cache-teardown-needs-second-witness (entries_39) owns TEARDOWN/recovery — when to wipe a whole warm tier from unattributable start failures; THIS owns per-entry EVICTION racing peers — different lifecycle stage, different granularity, both are peer-vs-self classification laws (adjacent genus, compose, never merge); the sec TOCTOU family (untrusted-path-join-escape + the checkout class) gates ACCEPTANCE of attacker-controlled paths — THIS classifies benign concurrent removal during our own eviction, no attacker path involved; entries_55's peer rules (read_request_deadline, turnstile) are socket-lane laws — the peer there controls BYTES, here the peer controls the FILESYSTEM. NOVEL (T1 greps at landing HEAD): evict-dot-not_found / peer-dot-race / vanished / remove_file-dot-tolerate over rust_perf+sec — zero owner of the eviction-race classification axis. Suggestion-only (the defect needs the shared-deployment knowledge; single-owner code with the split added is harmless but pointless, so review decides). FIXTURE PROVENANCE: original machinery — the fixed arm's evict_one carries the kind split (Removed / PeerEvicted with a debug log / panic on any other kind); the tests pin the three outcomes AND the two naive arms' failures (all-fatal panics on the benign peer event; all-swallowed hides an unreadable-but-present corruption file — the drift assert shows the naive-silent arm's ledger over-counts freed bytes). Cite: mozilla/sccache @ c1d7204a — src/lru_disk_cache/mod.rs:257-288 (make_space: the unwrap_or_else kind split, the in-tree issue-2092 cite, the outside-interference TODO). Batch 343 / Plan 355 / Research 387."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None, // NotBenchable-by-domain: correctness classification under concurrent owners; the fixtures carry the three-outcome pins
        },
    ]
}
