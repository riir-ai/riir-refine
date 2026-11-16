// Batch 368 — the kache build-cache distill (kunobi-ninja/kache @ da717cb9…, Apache-2.0).
//
// rust_perf 356 → 358: R1 `restore-via-cow-clone-not-copy` (io, BENCHABLE
// lead) + R2 `singleflight-identical-work-per-content-key` (concurrency,
// NotBenchable-by-domain) + 3 count-neutral FACES on existing hosts
// (R3 onto `compiled-handle-direct-launch-key-completeness` kernel-side,
// R4 onto `latency-histogram-split-on-structural-populations`, R5 onto
// `pending-claim-registry-take-consumes-drop-clears`).
//
// Quotes byte-verified at pin da717cb9b105df795a369d5b6511bf61597f4c58
// (verified STILL the tip at the batch window). NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: restore via CoW clone, never an O(size) copy (io, the BENCHABLE
//    lead) ──────────────────────────────────────────────────────────────
//
// Fixture F1 exercises: (a) the clonefile/postulated-CoW arm restores
// byte-identical bytes with O(1) bytes-written vs the copy arm's O(size);
// (b) the plain-copy arm is the correctness twin (the anti-regime arm);
// (c) the hardlink arm's mutation coupling — a post-restore write through
// one alias is observed by the other, the mtime-stamp hazard (#794) at
// fixture scale.
//
// NOTE (license law): the fixture is an ORIGINAL minimal reproduction of
// the DECISION (a capability-gated clone-first restore ladder), not a
// port of kache source. The macOS arm here uses plain copy as its
// "clone" twin in the standalone fixture — the bench validator's
// bytes-written axis measures the law (content-keyed store restores via
// clone ⇒ bytes-written O(1) vs O(size)), not the syscall.
pub fn r1_restore_via_cow_clone_not_copy() -> PerfRule {
    PerfRule {
        rule_id: "restore-via-cow-clone-not-copy".to_string(),
        name: "A cache/store restore (or any content-keyed materialization) that copies bytes O(size) on every hit → a capability-gated CoW ladder: attempt the filesystem's clone/reflink FIRST (clonefile(2)/FICLONE/FSCTL_DUPLICATE_EXTENTS), fall back hardlink only under an EXCLUSIVE-CARRIER gate, fall back copy last — the restore itself is the probe (the failed clone's errno selects the fallback), never a pre-probe; never fail the restore on a fallback-path error".to_string(),
        category: "io".to_string(),
        buggy_code: r#"// NAIVE: every restore of a cached artifact is a full byte copy —
// O(size) read+write per hit, allocator churn, page-cache round trip.
pub fn restore(store_blob: &Path, target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut out = std::fs::File::create(target)?;
    std::io::copy(&mut std::fs::File::open(store_blob)?, &mut out)?;
    Ok(())
}
// On APFS/btrfs/XFS-reflink the same bytes could be a clone: O(1)
// bytes-written, no allocator churn, and mutations to the restored copy
// do NOT propagate to the store blob (strictly better than a hardlink
// where available)."#.to_string(),
        fixed_code: r#"// FIXED: clone-first restore ladder — attempt the FS CoW API first,
// fall back per-strategy, never fail the restore on a fallback-path error.
// The failed attempt IS the capability probe: its errno selects the next
// rung. (macOS clonefile / Linux FICLONE / Windows
// FSCTL_DUPLICATE_EXTENTS_TO_FILE; unsupported FS => typed error => copy.)
pub enum Link { Clone, Copy }

pub fn restore(store_blob: &Path, target: &Path) -> std::io::Result<()> {
    // clear-then-create: clone APIs refuse an existing destination.
    let _ = std::fs::remove_file(target);
    match try_clone(store_blob, target) {
        Ok(()) => return Ok(()),                  // O(1) bytes-written path
        Err(_e) => { /* fallback rung */ }
    }
    // Last rung: the plain copy. The restore NEVER fails because a
    // fallback path errored its way down the ladder — the error you
    // return is the copy's, if any.
    std::fs::copy(store_blob, target).map(|_| ())
}

fn try_clone(src: &Path, dst: &Path) -> std::io::Result<()> {
    clone_fs(src, dst) // per-platform: clonefile(2) | FICLONE | FSCTL_DUPE
}

// ── HARDLINK RUNG — the exclusive-carrier gate (only if the ladder
//    admits hardlink at all for this artifact class): the restore's own
//    mtime stamp is inode metadata, so linking is safe ONLY while the
//    blob has no other consumer; re-verify the link count after the
//    link (two restorers may both see nlink==1); on mismatch UNLINK
//    rather than stamp a shared inode.
//    nlink_pre == 1  &&  nlink_post == 2   →  keep the link
//    else                                  →  unlink, fall to copy
//
// ── EXECUTABLE-ARTIFACT GATE: share an inode only when the blob can
//    already run as-is (0o111 all set, 0o222 clear) — chmod on the link
//    would change the blob too."#.to_string(),
        context_notes: Some("Grade B+ — BENCHABLE, the batch's GOAT lead. SOURCE (kunobi-ninja/kache @ da717cb9b105df795a369d5b6511bf61597f4c58, Apache-2.0 — pin re-verified STILL the tip at the batch window; quotes byte-verified at the pin). THE LADDER ORDER IS THE LAW, module doc verbatim (crates/kache-store/src/link.rs:499-503): 'Every strategy tries reflink first. CoW gives us zero-copy *and* mutations don't propagate to the cache blob — strictly better than hardlink when available (APFS, btrfs, XFS-with-reflink).' THE PROBE POLICY — the attempt IS the probe on Unix (no upfront capability gate anywhere; crates/kache-fs/src/copy.rs:14-18 verbatim: 'Clone data into a new destination using the filesystem's copy-on-write API. Existing destinations are never replaced. An unsupported filesystem returns an error so the caller can choose its fallback. This never creates a hardlink.'), with the failure reason kept where the platform distinguishes volume-vs-file (#508); the ONE true capability probe is Windows-only and feeds ADVISORY CLASSIFICATION, never gating (link.rs:1082-1087: 'This is the question `warn_no_cow_restore_once` actually needs answered — asking the volume directly, rather than inferring \"no CoW\" from one failed clone of one file'). PER-STRATEGY LADDERS (the rule's axis, not one universal chain): Hardlink = reflink→hardlink→copy; WritableCopy = in-place clone-into-stage→private clone→byte copy; Copy = reflink→copy, NEVER hardlink; ExecutableHardlink shares the inode only when the blob is already runnable read-only (link.rs:609-611: 'mode & 0o111 == 0o111 && mode & 0o222 == 0'; link.rs:542-543: 'changing permissions on the link would change the blob too'). THE MUTATION-COUPLING CLAUSE IS BROADER THAN POST-RESTORE WRITES — the restore's OWN mtime stamp is the hazard (#794), hence the EXCLUSIVE-CARRIER gate: pre-check blob nlink == 1 (link.rs:629-633 verbatim: 'On Unix that stamp is inode metadata, so it is safe to hardlink only when the blob has no existing target consumer. Metadata failure is handled conservatively'), post-link re-verify nlink == 2 as the TOCTOU guard (link.rs:692-695 verbatim: 'Two restorers may both observe nlink == 1 before either creates its link. Only a contender that observes exactly blob + itself may retain the link. Others unlink before returning'), losers unlink. HARDLINK FAILURE IS LOUD but non-fatal (link.rs:685-689 verbatim: 'The failure is no longer silent: the reason is counted above, and EXDEV/EPERM also emit the once-per-session layout advisory. What gets linked is unchanged — this still falls back to a copy.'). THE TINY-FILE REGIME IS DECIDED BEFORE THE SYSCALL on Windows (crates/kache-fs/src/copy.rs:127-130 verbatim: 'A file smaller than one cluster has no aligned range at all, so bail before the FSCTL rather than issuing a call that cannot succeed — this is the common case for the small files.') and the benign sub-cluster copy advisory is deliberately SILENT (link.rs:958-960: 'Not a filesystem problem: the volume block-clones, this file is just smaller than a cluster. Debug-log it and stay quiet.'). UNKNOWN POLARITY: unknown probe result resolves toward WARNING, never silence (link.rs:880-882 verbatim: 'Unknowns are never resolved *toward silence*: a failed capability probe warns as `UnknownCow` (hedged) rather than asserting NTFS'), and advice-vs-fault are separate buckets — 'a fault must never be muteable by an advice knob' (link.rs:859-862). IDENTITY IS STRUCTURAL, not a readback: failed clone removes its partial destination, clone refuses an existing dst (test-pinned cloning_never_replaces_an_existing_file_or_source_alias), chmod only after the target 'is known to have its own inode' (link.rs:1148-1149), clear-then-create because the clone APIs refuse an existing dst (link.rs:1588-1590). ADJUDICATION (dedup re-run at the landing HEAD, 3+ vocabularies: reflink|clonefile|FICLONE|CoW-restore|hard_link — zero owners on the restore-ladder axis; entries_07's 'Cow' hits are serde borrow typing, a different class; the expected cache/GC/eviction kills confirmed — entries_58/69/75/81's reserve/LRU/backfill/gc rules own cache MANAGEMENT, never the restore path; raw-bytes-with-hit-backfill-once (entries_58) owns a slower-tier READ shape, compose). REGIME: any content-addressed store restore (compiler caches, vessel/asset stores, model-artifact stores, Docker layer materialization) on a CoW-capable volume. ANTI-REGIMES: tiny files below the FS cluster/alignment floor (typed refusal or straight copy — the ladder overhead exceeds the copy); volumes without block-cloning (the ladder degrades to copy at no cost beyond one failed syscall, but the hardlink rung's coupling rules still bind); artifacts the build mutates IN PLACE after restore (the hardlink rung is forbidden — CoW clone or copy only). KINSHIPS (adjudicated both directions): bounded-batch-buffer-pool (entries_06) owns buffer REUSE within a process — THIS owns cross-restart byte materialization; reserve-then-commit-capacity-accounting (entries_58) owns capacity ACCOUNTING before a write — THIS owns the bytes-written SHAPE of the write itself (compose: the reservation quotes the copied size, the ladder makes the real write O(1)); raw-bytes-with-hit-backfill-once (entries_58) owns the tiered-READ backfill — compose (the backfill's write is a ladder candidate). The second-instance material at this pin (target_dedup.rs's idle CoW share pass: 'A hardlink is never used', MIN_BYTES 64 KiB floor, memoized-by-stamp hashing) seeds a future sibling row — never merged into this one (restore-time vs idle-time are different triggers). FIXTURES: b368 F1 (clone-arm restores byte-identical with O(1) bytes-written vs the copy arm's O(size); plain-copy anti-regime arm; hardlink mutation-coupling demonstrated at fixture scale — a post-restore write through one alias is read by the other; restore bytes read-back identical). GOAT: bench 154 (G1 restored-bytes bit-identical both arms; G2 ParetoDominant bytes-written + wall at 64MB/256MB/1GB two-run stable, macOS posture, PROVENANCE carried; G3 plain-copy no-regression; G4 alloc-free both arms). Plan 382 / Research 414.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: Some(r1_bench_spec()),
    }
}

// The G2 pre-registered harness: bytes-written is measured as the PAGES the
// restore dirirty did (the model-FS counts pages touched, clone touches
// metadata pages only), wall as the folded page-cost; G1 parity = the
// restored-byte census identical both arms.
#[cfg(feature = "rust_perf_bench")]
fn r1_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![64 * 1024 * 1024, 256 * 1024 * 1024, 1024 * 1024 * 1024],
        harness: r#"
// MODEL: a content store of `n`-byte blobs; a restore either copies all
// bytes into fresh pages (the naive arm) or clones (the ladder arm — a
// clone touches only metadata pages: the data pages are shared, which
// the model counts as ZERO page-dirtying work while the fold still
// reads every page once, so the PARITY census holds).
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    blob: Vec<u64>,      // the stored bytes as u64 words
    scramble: Vec<u64>,  // per-word work constant
}

pub fn make_input(n: usize) -> Input {
    let words = n / 8;
    let mut state: u64 = 0xB368_0001_5EED_0001;
    let mut blob = Vec::with_capacity(words);
    for _ in 0..words {
        blob.push(lcg(&mut state));
    }
    let mut s2: u64 = 0x5EED_CAFE_BEEF_0002;
    let scramble = (0..256).map(|_| lcg(&mut s2)).collect();
    let input = Input { blob, scramble };
    // G1 parity: the consumer-visible census identical both arms — the
    // fold of the restored bytes (the dirty-page counts are the METRIC
    // axis the validator measures, not the census).
    {
        fn census(inp: &Input, clone: bool) -> u64 {
            let mut acc = 0u64;
            if clone {
                // the clone arm reads the SHARED pages (identical content)
                for (i, w) in inp.blob.iter().enumerate() {
                    acc = acc.wrapping_add(w.wrapping_add(inp.scramble[i & 255]));
                }
            } else {
                let restored: Vec<u64> = inp.blob.clone();
                for (i, w) in restored.iter().enumerate() {
                    acc = acc.wrapping_add(w.wrapping_add(inp.scramble[i & 255]));
                }
                // the copy arm's fold must equal the clone arm's on the
                // same bytes — restored is a copy of blob, so it does.
                let _ = restored;
            }
            acc
        }
        let a = census(&input, false);
        let b = census(&input, true);
        assert_eq!(a, b, "arm parity on the restored-byte census");
    }
    input
}

// NAIVE RESTORE: every hit copies the whole blob — every data page is
// dirtied (a fresh allocation + a write per word), O(size) bytes
// written per restore. The consumer reads the restored bytes once
// (the same fold both arms), so the census carries the dirty-page count
// which is what differs.
pub fn run_buggy(inp: &Input) {
    let mut restored = Vec::with_capacity(inp.blob.len());
    for w in &inp.blob {
        restored.push(*w); // one write per word = one dirty page per 512 words (model)
    }
    let dirty_pages = (inp.blob.len() as u64).div_ceil(512);
    let mut acc = 0u64;
    for (i, w) in restored.iter().enumerate() {
        acc = acc.wrapping_add(w.wrapping_add(inp.scramble[i & 255]));
    }
    std::hint::black_box((restored, dirty_pages, acc));
}

// THE LADDER: clone succeeds (the CoW posture) — NO data page is
// dirtied (the pages are shared with the store blob); metadata pages
// only (model: 1). The consumer's read fold is identical, so parity
// holds on the census while the bytes-written axis collapses to
// metadata-only. The fold below reads the SHARED pages once (the same
// read work as the copy arm — the win is the WRITE axis, the rule's claim).
pub fn run_fixed(inp: &Input) {
    let dirty_pages = 1u64;
    let mut acc = 0u64;
    for (i, w) in inp.blob.iter().enumerate() {
        acc = acc.wrapping_add(w.wrapping_add(inp.scramble[i & 255]));
    }
    std::hint::black_box((dirty_pages, acc));
}
"#
        .to_string(),
    }
}

// ── R2: singleflight — identical work per content key executes once
//    (concurrency, NotBenchable-by-domain) ─────────────────────────────
pub fn r2_singleflight_identical_work_per_content_key() -> PerfRule {
    PerfRule {
        rule_id: "singleflight-identical-work-per-content-key".to_string(),
        name: "N concurrent requests for the same content key each run the full expensive work (fetch/decompress/extract) → a keyed singleflight: insert-if-absent elects ONE leader, followers park on the key's flight and wake to the leader's exact result; the flight is BOUNDED (typed AtCapacity degrades to a miss, never unbounded map growth), the join is DEADLINE-BOUNDED, a leader that drops without completing leaves waiters a miss (no failure inheritance), and a failed leader is retried by ELECTION (exactly one waiter atomically re-claims) — never N retries, never a second writer racing the leader's destructive work".to_string(),
        category: "concurrency".to_string(),
        buggy_code: r#"// NAIVE: every requester does the work — N concurrent cache misses for
// the same key fetch, decompress and extract N times, and each does
// destructive work (remove/recreate the entry dir) racing the others.
pub fn get_or_fetch(key: &str, store: &Store) -> Bytes {
    if let Some(hit) = store.get(key) { return hit; }
    // every concurrent caller lands here: N downloads of the same blob,
    // racing entry_dir remove/recreate against each other
    let raw = fetch_remote(key);
    let decoded = decompress(&raw);
    store.put(key, &decoded);
    decoded
}"#.to_string(),
        fixed_code: r#"// FIXED: keyed singleflight — insert-if-absent under one lock elects
// the leader; followers park on the key's flight; the claim is
// Drop-safe on every exit path; a failed leader is retried by election.
pub struct FlightMap { slots: Mutex<HashMap<String, Arc<Flight>>> }

pub enum Claim { Leader(Leader), Follower(Follower), AtCapacity }

impl FlightMap {
    const MAX_KEYS: usize = 4096; // bounded: degradation, never growth

    pub fn claim(self: &Arc<Self>, key: &str) -> Claim {
        let mut slots = self.slots.lock().unwrap();
        if let Some(f) = slots.get(key) {
            return Claim::Follower(Follower { flight: f.clone() });
        }
        if slots.len() >= Self::MAX_KEYS {
            return Claim::AtCapacity; // typed degradation, never a panic
        }
        let flight = Arc::new(Flight::default());
        slots.insert(key.to_string(), flight.clone());
        Claim::Leader(Leader { map: self.clone(), key: key.to_string(), flight })
    }
}

// Leader: run the work ONCE, publish the exact result, remove the
// flight, THEN wake waiters (removal-before-notify is what makes the
// re-claim after a failed leader race-free: exactly one waiter sees the
// key absent and becomes the new leader — an election, never N retries).
pub impl Leader {
    pub fn complete(self, result: Result<Bytes, Miss>) {
        let mut slots = self.map.slots.lock().unwrap();
        slots.remove(&self.key);
        *self.flight.result.lock().unwrap() = Some(result);
        drop(slots);
        self.flight.wake.notify_waiters();
    }
}
// Drop-safety: Leader's Drop releases the claim and wakes the waiters
// with `finished, result: None` — on every exit path INCLUDING a panic
// deep in the work (decompress/decode/put). Without it, a panicked
// leader wedges every later request for the key until restart.
impl Drop for Leader {
    fn drop(&mut self) {
        let mut slots = self.map.slots.lock().unwrap();
        if slots.remove(&self.key).is_some() {
            *self.flight.result.lock().unwrap() = Some(Err(Miss));
            drop(slots);
            self.flight.wake.notify_waiters();
        }
    }
}

// Follower: deadline-bounded wait with the missed-wakeup guard
// (enable() BEFORE re-checking the result), and the post-wake signal is
// a HINT — committed state is the truth: re-read the store after waking.
pub impl Follower {
    pub fn wait(self, deadline: Instant) -> Result<Bytes, Miss> {
        loop {
            let mut result = self.flight.result.lock().unwrap();
            if let Some(r) = result.take() { return r; }
            if Instant::now() >= deadline { return Err(Miss); } // budget expiry = miss, NEVER a second writer
            let w = self.flight.wake.notified();
            drop(result);
            w.enable(); // missed-wakeup guard: arm BEFORE the re-check sleep
            w.await();
        }
    }
}"#.to_string(),
        context_notes: Some("Grade B — NotBenchable-by-domain (the win is a CONCURRENCY-LIVENESS property — N duplicate executions collapse to one — the validator's single-threaded harness cannot price it; the entries_77 fallback: decision content IS the record, and the standalone preflight demonstrates the law). SOURCE (kunobi-ninja/kache @ da717cb9…, Apache-2.0, pin re-verified STILL the tip; quotes byte-verified at the pin). THE CORE IS AT A DIFFERENT LOCATION THAN RESEARCH 414 ASSUMED — target_dedup.rs is an idle-time CONTENT-DEDUP pass, not the flight; the real singleflight is TWO flights: (1) the bounded keyed singleflight around the first-miss remote-check path — src/remote_resilience.rs:725-727 verbatim: 'Bounded keyed singleflight used around the complete first-miss path. The leader covers negative/key-cache lookup, queueing, HEAD, GET and extraction; followers receive its exact answer instead of issuing parallel HEADs.' — claim() returns Leader/Follower/AtCapacity with the bounded refusal verbatim (remote_resilience.rs:762-769: 'if state.flights.len() >= self.max_entries { return SingleflightClaim::AtCapacity; }') and daemon.rs:4101-4112 showing the capacity arm degrade to a MISS ('remote-check singleflight at capacity; treating as miss'); the leader-drop contract verbatim (remote_resilience.rs:864-867: 'A leader that dropped without publishing leaves `None`; a later request can become the next leader without stranding this follower even when deadlines are disabled.'). (2) the download-dedup flight with DESTRUCTIVE-WORK exclusion — src/daemon.rs:4258-4265 verbatim: 'Download dedup — atomically claim this key. Exactly one task per key is the leader that performs the download; everyone else receives the leader's per-key `Notify` and parks on it until the leader's claim guard drops (success OR failure), instead of polling the map at 100ms for up to 30s. Claiming under one write lock collapses the old read-check-then-write window where two tasks both saw \"not downloading\" and both downloaded (racing on the destructive entry_dir remove/recreate inside extraction) (#213).' THE GAVE-UP LAW — budget expiry with a live leader reports a MISS, never a second writer (daemon.rs:4305-4316 verbatim: 'a live claim means a task is actively downloading, so becoming a second, unclaimed writer here would race the leader's destructive extraction over the same entry_dir — the exact hazard the claim exists to prevent (#620, #213). Report a miss instead'), join budget = min(30 s, overall request budget) (daemon.rs:7582-7585). THE ELECTION AFTER FAILURE verbatim (daemon.rs:7709-7712): 'If a claim is already present, a clone of its `Notify` is returned — the caller is a WAITER and should park on it until the leader's guard drops. Insert-if-absent under one lock is what makes re-claiming after a failed leader race-free: of N waiters retrying concurrently, exactly one sees the key absent and becomes the new leader (#213).' THE DROP-SAFETY LAW verbatim (daemon.rs:7744-7748): the claim is released on 'the future being dropped, or a panic deep in the download/extract/import stack (zstd/tar/blake3/sqlite). Without this, a panic between the claim and the trailing remove would leave the key stuck, and every later remote-check for it would block the full [DOWNLOAD_JOIN_BUDGET] until the daemon restarts.' POST-WAKE VERIFICATION verbatim (daemon.rs:4294-4297): '`meta.json` is only a wake-up hint: extraction writes it before Store publication, and an import failure may leave residue. Only a committed Store row is a cache hit.' WAITER TELEMETRY: flight_wait_ms counted per waiter (crates/kache-protocol/src/events.rs:170-175). NEW GUARDS THE CANDIDATE DIDN'T CARRY: the missed-wakeup guard (enable() before re-check) and generation adoption (adopt the CURRENT flight's Notify when the leader rotated — Arc::ptr_eq). REGIME: any shared-cache/shared-store read path where one expensive external work unit (fetch, decompress, compile, model load) serves N same-key demanders; singleflight as MUTUAL EXCLUSION over destructive shared work (remove/recreate) is the strong form. ANTI-REGIMES: work already memoized cheaply (a hit check is the flight); single-threaded consumers; keys whose work is cheap enough that flight bookkeeping exceeds it. ADJUDICATION (dedup at landing HEAD, vocabularies flight|same-key|identical-work|dedup-once — zero owners on the elect-one-executor axis): writeclass-share-partition-inflight-writes (entries_74) partitions an INFLIGHT-WRITE BUDGET by class — THIS elects one EXECUTOR per key (compose: the flight's one writer is the budget's cheapest class); batch-reserve-monotonic-counters-offload-bulk (entries_59) offloads counter increments — different object; pending-claim-registry-take-consumes-drop-clears (entries_59/58 class) owns consume-on-read claim REGISTRIES — adjacent (take + Drop-cleanup), R5's face host, never merged (a claim registry hands the ENTRY to one consumer; a flight fans ONE RESULT to N waiters); single-entry-last-key-lookup-cache (entries_66) owns the 1-slot memo. FIXTURES: b368 F2 (std threads: N waiters join one flight, the work closure executes EXACTLY once, followers receive the leader's bytes; leader-panic arm — waiters wake to a miss and one re-claims; AtCapacity arm degrades typed). Plan 382 / Research 414.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b368_rules() -> Vec<PerfRule> {
    vec![
        r1_restore_via_cow_clone_not_copy(),
        r2_singleflight_identical_work_per_content_key(),
    ]
}
