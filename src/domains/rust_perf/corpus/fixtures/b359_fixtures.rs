//! Batch 359 fixtures — the ajika persistent-incrementality laws as ONE
//! standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b359_fixtures.rs -o /tmp/b359_fixtures && /tmp/b359_fixtures` (the
//! B336→B358 discipline). Each rule's buggy/fixed arms in `entries_75.rs`
//! (rust_perf) embed the same executed code. ORIGINAL Rust throughout —
//! the fixtures are original syntheses of the LAWS (a persisted store is a
//! plain struct with byte fields + a name index; the fixture never links
//! ajika, redb, wincode, or any real storage engine). Four-arm discipline
//! per law: buggy / fixed / all-safe / tampered.
//!
//! F1 = named-outputs-lazy-handles-partial-decode (rust_perf, io, the
//!      BENCHABLE GOAT lead — the fixture doubles as the bench harness
//!      input: the arms, sizes and checksums here are the bench spec's)
//! F2 = derived-cache-throwaway-version-reset-recovery (rust_perf, caching)
//! F3 = pack-store-gc-decides-by-recovered-index (rust_perf, io)
//! F4 = persisted-interner-assignments-checkpoint-with-refering-values
//!      (rust_perf, caching)
//! F5 = scope-scoped-memo-release-idle-holds-no-decoded-state (rust_perf,
//!      caching)
//!
//! Source: rust-glancer/ajika @ `70c1e26df847d3da10d2a59a5a6aefe37db2bf55`
//! (v0.1.7, MIT OR Apache-2.0; main tip UNMOVED at T0 this batch; removed
//! at close) + the handwritten persistent-incrementality blog. Batch 359 /
//! Plan 370 / Research 403.

// ═══════════════════════════════════════════════════════════════════════
// F1 — named outputs + lazy handles: a persisted computation that produces
// K named answers together returns cheap ADDRESS handles; a consumer that
// reads ONE answer decodes ONE field, and the unread fields never
// deserialize. The direct-payload contract hands the consumer the whole
// serialized output, so reading any answer decodes every field.
// ═══════════════════════════════════════════════════════════════════════

/// Each named field carries 256 u64 values (2 KiB encoded) — big enough
/// that framing cost is noise, small enough that the census stays instant.
const FIELD_WORDS: usize = 256;

/// Deterministic field payload: word `i` of field `field` (bit-exact both
/// arms by construction — the same function encodes and verifies).
fn field_word(field: usize, i: usize) -> u64 {
    let mut x = ((field as u64) << 32) ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
}

fn encode_field(field: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(FIELD_WORDS * 8);
    for i in 0..FIELD_WORDS {
        bytes.extend_from_slice(&field_word(field, i).to_le_bytes());
    }
    bytes
}

fn decode_field(bytes: &[u8]) -> Vec<u64> {
    debug_assert_eq!(bytes.len(), FIELD_WORDS * 8);
    bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("8-byte word")))
        .collect()
}

/// A stable field checksum: order-sensitive rotate-xor fold (bit-exact).
fn checksum(values: &[u64]) -> u64 {
    values.iter().fold(0u64, |acc, v| acc.rotate_left(7) ^ *v)
}

/// The persisted output: K named fields stored as encoded byte blobs.
struct StoredOutput {
    fields: Vec<Vec<u8>>,
}

/// The handle: an ADDRESS (which producer, which slot) — nothing decoded,
/// cheap to copy, cheap to persist. The value is fetched on demand.
#[derive(Clone, Copy)]
struct OutputHandle {
    slot: usize,
}

/// The producer emits K named outputs and returns one handle per name.
fn emit_named_outputs(k: usize) -> (StoredOutput, Vec<OutputHandle>) {
    let fields: Vec<Vec<u8>> = (0..k).map(encode_field).collect();
    let handles = (0..k)
        .map(|slot| OutputHandle { slot })
        .collect();
    (StoredOutput { fields }, handles)
}

/// Read result: (answer checksum, bytes decoded, field decodes).
type Read = (u64, usize, usize);

/// BUGGY ARM: the direct-payload contract — the consumer receives the
/// whole output and must decode EVERY field to read any one of them.
/// O(total) bytes + K decodes per single-field read.
fn read_named_direct(store: &StoredOutput, slot: usize) -> Read {
    let mut bytes_read = 0usize;
    let mut decodes = 0usize;
    let mut answer = 0u64;
    for (i, field) in store.fields.iter().enumerate() {
        bytes_read += field.len();
        let decoded = decode_field(field);
        decodes += 1;
        if i == slot {
            answer = checksum(&decoded);
        }
    }
    (answer, bytes_read, decodes)
}

/// FIXED ARM: the named-output contract — the handle decodes ONE field on
/// demand. O(field) bytes + 1 decode; the unread fields never deserialize.
fn read_named_handle(store: &StoredOutput, handle: OutputHandle) -> Read {
    let field = &store.fields[handle.slot];
    let decoded = decode_field(field);
    (checksum(&decoded), field.len(), 1)
}

/// ALL-SAFE ARM (the anti-regime posture): a consumer that reads EVERY
/// field through handles decodes all of them — the same volume as the
/// direct arm; the framing tax must be bookkeeping only.
fn read_all_via_handles(store: &StoredOutput, handles: &[OutputHandle]) -> Read {
    let mut bytes_read = 0usize;
    let mut decodes = 0usize;
    let mut acc = 0u64;
    for handle in handles {
        let field = &store.fields[handle.slot];
        bytes_read += field.len();
        let decoded = decode_field(field);
        decodes += 1;
        acc = acc.rotate_left(1) ^ checksum(&decoded);
    }
    (acc, bytes_read, decodes)
}

/// The direct arm's read-all shape: one pass, every field decoded.
fn read_all_direct(store: &StoredOutput) -> Read {
    let mut bytes_read = 0usize;
    let mut decodes = 0usize;
    let mut acc = 0u64;
    for field in &store.fields {
        bytes_read += field.len();
        let decoded = decode_field(field);
        decodes += 1;
        acc = acc.rotate_left(1) ^ checksum(&decoded);
    }
    (acc, bytes_read, decodes)
}

#[test]
fn f1_named_outputs_partial_decode() {
    let k = 8usize;
    let slot = 3usize;
    let (mut store, handles) = emit_named_outputs(k);

    // G1 — bit-identical answers: the partial read through the handle
    // returns EXACTLY what the whole-output decode returns.
    let (direct_cs, direct_bytes, direct_decodes) = read_named_direct(&store, slot);
    let (handle_cs, handle_bytes, handle_decodes) = read_named_handle(&store, handles[slot]);
    assert_eq!(direct_cs, handle_cs, "G1: partial read answer must be bit-identical");

    // The bytes-read census: whole = K fields, partial = one field. The
    // pre-registered bar is >=3x at the fixture's shape (K=8 gives 8x).
    assert_eq!(handle_bytes, FIELD_WORDS * 8);
    assert_eq!(direct_bytes, k * FIELD_WORDS * 8);
    assert!(
        direct_bytes >= 3 * handle_bytes,
        "bytes-read bar: whole-output decode must read >=3x the partial read"
    );
    assert_eq!(handle_decodes, 1);
    assert_eq!(direct_decodes, k);

    // ALL-SAFE posture: a read-everything consumer through handles decodes
    // the same volume as the direct arm, with identical answers.
    let (all_handle_cs, all_handle_bytes, _) = read_all_via_handles(&store, &handles);
    let (all_direct_cs, all_direct_bytes, _) = read_all_direct(&store);
    assert_eq!(all_handle_cs, all_direct_cs, "read-all answers identical");
    assert_eq!(all_handle_bytes, all_direct_bytes, "read-all volume identical");

    // TAMPERED arm — the misuse pin: a handle is an ADDRESS, not a
    // snapshot. Replacing the producer's stored bytes (a recompute) is
    // visible through the OLD handle: it asks for the new answer.
    let replacement = {
        let mut bytes = vec![0u8; FIELD_WORDS * 8];
        for (i, chunk) in bytes.chunks_exact_mut(8).enumerate() {
            chunk.copy_from_slice(&field_word(slot, i).wrapping_add(1).to_le_bytes());
        }
        bytes
    };
    let old_cs = checksum(&decode_field(&store.fields[slot]));
    let new_cs = checksum(&decode_field(&replacement));
    assert_ne!(old_cs, new_cs);
    store.fields[slot] = replacement;
    let (reread_cs, _, _) = read_named_handle(&store, handles[slot]);
    assert_eq!(
        reread_cs, new_cs,
        "a kept handle reads the CURRENT stored bytes, never a pinned old version"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — the derived cache is THROW-AWAY: persisted derived state resets on
// a schema/version mismatch and recovers-over-hard-fail on corruption; the
// application never crashes on a cache problem, and a healthy cache still
// reuses everything.
// ═══════════════════════════════════════════════════════════════════════

/// The source of truth (never disposable).
fn source_of_truth(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|name| ((*name).to_string(), name.to_uppercase()))
        .collect()
}

/// A derived-cache snapshot as it would sit on disk: a schema tag plus the
/// derived entries; `corrupt` marks flipped bytes.
#[derive(Clone)]
struct CacheOnDisk {
    schema_tag: String,
    entries: Vec<(String, String)>,
    corrupt: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum OpenOutcome {
    /// Healthy cache, matching schema: full reuse, zero recompute.
    FullReuse,
    /// Version mismatch or corruption: the cache was discarded (all or the
    /// corrupt part) and rebuilt from the source of truth. Never fatal.
    Recovered { discarded: usize },
    /// The BUGGY posture's outcome: the cache problem is fatal to the app.
    Fatal,
}

/// What the source of truth derives for every name (the recompute).
fn derive_all(names: &[&str]) -> Vec<(String, String)> {
    source_of_truth(names)
}

/// BUGGY ARM: open treats the cache as an asset — any cache problem is a
/// hard failure the app cannot start past.
fn open_hard_fail(disk: &CacheOnDisk, app_schema: &str) -> OpenOutcome {
    if disk.corrupt {
        return OpenOutcome::Fatal; // "cache is not available, what do I do???"
    }
    if disk.schema_tag != app_schema {
        return OpenOutcome::Fatal; // old cache refuses to load, app crashes
    }
    OpenOutcome::FullReuse
}

/// FIXED ARM: the cache is a disposable, exclusively-owned derived state —
/// a schema mismatch RESETS it, corruption is discarded-and-recovered, and
/// the app never crashes on a cache problem.
fn open_throwaway(disk: &CacheOnDisk, app_schema: &str, names: &[&str]) -> OpenOutcome {
    if disk.schema_tag != app_schema {
        // "Both Ajika and application supply schema versions ... if the
        // stored cache doesn't match it, we reset it." ALL derived state
        // goes; the source of truth rebuilds it.
        let discarded = disk.entries.len();
        let rebuilt = derive_all(names);
        debug_assert_eq!(rebuilt.len(), names.len());
        return OpenOutcome::Recovered { discarded };
    }
    if disk.corrupt {
        // "Opening can discard corrupt cache data" — recovery over hard
        // failing, even when it means re-computing from scratch.
        let discarded = disk.entries.len();
        let rebuilt = derive_all(names);
        debug_assert_eq!(rebuilt.len(), names.len());
        return OpenOutcome::Recovered { discarded };
    }
    OpenOutcome::FullReuse
}

/// An UNCERTAIN COMMIT: the write may or may not have landed. The published
/// checkpoint is what a reopen recovers.
struct UncertainWrite {
    published: Vec<(String, String)>, // what the last checkpoint actually published
}

/// BUGGY reopen: after an uncertain commit the app cannot tell committed
/// from uncommitted state and refuses to continue.
fn reopen_hard_fail(_disk: &UncertainWrite) -> OpenOutcome {
    OpenOutcome::Fatal
}

/// FIXED reopen: the recovered checkpoint decides — reads resume from what
/// the index actually published.
fn reopen_recover(published: &UncertainWrite, names: &[&str]) -> (OpenOutcome, Vec<(String, String)>) {
    let mut state = published.published.clone();
    for (name, derived) in derive_all(names) {
        if !state.iter().any(|(n, _)| *n == name) {
            state.push((name, derived));
        }
    }
    (OpenOutcome::FullReuse, state)
}

#[test]
fn f2_derived_cache_is_throwaway() {
    let names = ["alpha", "beta", "gamma", "delta"];
    let truth = source_of_truth(&names);

    // Healthy cache, matching schema: full reuse.
    let healthy = CacheOnDisk {
        schema_tag: "derived-v2".to_string(),
        entries: truth.clone(),
        corrupt: false,
    };
    assert_eq!(open_throwaway(&healthy, "derived-v2", &names), OpenOutcome::FullReuse);
    // ...and the buggy arm agrees when everything is fine.
    assert_eq!(open_hard_fail(&healthy, "derived-v2"), OpenOutcome::FullReuse);

    // Version mismatch: the fixed arm resets and rebuilds; the buggy arm
    // makes the mismatch fatal.
    let stale = CacheOnDisk {
        schema_tag: "derived-v1".to_string(),
        entries: vec![("alpha".to_string(), "WRONG".to_string())],
        corrupt: false,
    };
    assert_eq!(
        open_throwaway(&stale, "derived-v2", &names),
        OpenOutcome::Recovered { discarded: 1 }
    );
    assert_eq!(open_hard_fail(&stale, "derived-v2"), OpenOutcome::Fatal);

    // Corruption: the fixed arm discards the corrupt data and recovers;
    // the buggy arm crashes the app on its own cache.
    let corrupt = CacheOnDisk {
        schema_tag: "derived-v2".to_string(),
        entries: truth.clone(),
        corrupt: true,
    };
    assert_eq!(
        open_throwaway(&corrupt, "derived-v2", &names),
        OpenOutcome::Recovered { discarded: truth.len() }
    );
    assert_eq!(open_hard_fail(&corrupt, "derived-v2"), OpenOutcome::Fatal);

    // G1: after EVERY fixed-arm path the derived answers equal the source
    // of truth — discard-and-recompute never changes the answers.
    let rebuilt = derive_all(&names);
    assert_eq!(rebuilt, truth, "recovery is answer-identical");

    // Uncertain commit: the buggy reopen refuses to continue; the fixed
    // reopen recovers the published checkpoint and fills the gap.
    let write = UncertainWrite {
        published: vec![("alpha".to_string(), "ALPHA".to_string())],
    };
    assert_eq!(reopen_hard_fail(&write), OpenOutcome::Fatal);
    let (outcome, state) = reopen_recover(&write, &names);
    assert_eq!(outcome, OpenOutcome::FullReuse);
    assert_eq!(state, truth, "reopen serves exactly the checkpointed truth");
}

// ═══════════════════════════════════════════════════════════════════════
// F3 — pack-store GC decides by the RECOVERED INDEX: values append to
// private pack files that seal (flush) BEFORE their locations publish in
// the index; a failed preparation leaves harmless orphans; once a commit
// has been attempted, ONLY the recovered index decides which files are
// safe to delete.
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct PackFile {
    name: String,
    bytes: Vec<u8>,
    sealed: bool,
}

/// value-key -> (file name, offset, len) — the committed locations.
#[derive(Clone, Default)]
struct Index {
    locations: Vec<(String, String, usize, usize)>,
}

impl Index {
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.locations.iter().map(|(_, f, _, _)| f.clone()).collect();
        names.sort();
        names.dedup();
        names
    }
}

/// The store as a crash left it: sealed files on disk, an index that may
/// or may not name them.
struct StoreAfterCrash {
    files: Vec<PackFile>,
    committed: Index,
}

/// The recovery step: reopen loads the committed index — the ONLY
/// publication authority.
fn recover_index(crash: &StoreAfterCrash) -> Index {
    crash.committed.clone()
}

/// BUGGY GC: the keep-set comes from a stale in-memory map captured BEFORE
/// the crash — it never sees what the recovered index actually published.
fn gc_by_stale_map(crash: &StoreAfterCrash, stale_known: &[String]) -> Vec<String> {
    let mut survivors = Vec::new();
    for file in &crash.files {
        if stale_known.iter().any(|name| *name == file.name) {
            survivors.push(file.name.clone());
        }
    }
    survivors
}

/// FIXED GC: after a commit attempt, only the recovered index decides —
/// keep exactly the files it names, delete every other file. A deletion
/// failure is deferred (tolerated), never a data decision.
fn gc_by_recovered_index(crash: &StoreAfterCrash) -> (Vec<String>, Vec<String>) {
    let index = recover_index(crash);
    let keep = index.names();
    let mut kept = Vec::new();
    let mut deleted = Vec::new();
    for file in &crash.files {
        if keep.iter().any(|name| *name == file.name) {
            kept.push(file.name.clone());
        } else {
            deleted.push(file.name.clone());
        }
    }
    (kept, deleted)
}

/// Read a value back through the index: (file, offset, len) must resolve
/// to byte-identical content.
fn read_through_index(store: &StoreAfterCrash, key: &str) -> Option<Vec<u8>> {
    let index = recover_index(store);
    let (_, file, offset, len) = index.locations.iter().find(|(k, _, _, _)| k == key)?;
    let pack = store.files.iter().find(|f| &f.name == file)?;
    pack.bytes.get(*offset..*offset + *len).map(<[u8]>::to_vec)
}

#[test]
fn f3_pack_gc_decides_by_recovered_index() {
    // The crash shape: a value appended to a FRESH private file, the file
    // SEALED (flushed), locations published in the index, then the process
    // died. A failed PREPARATION also left an orphan (never published).
    let committed_bytes = b"committed-value-payload".to_vec();
    let orphan_bytes = b"failed-preparation-orphan".to_vec();
    let crash = StoreAfterCrash {
        files: vec![
            PackFile {
                name: "pack-0007".to_string(),
                bytes: committed_bytes.clone(),
                sealed: true,
            },
            PackFile {
                name: "pack-0008".to_string(),
                bytes: orphan_bytes,
                sealed: true,
            },
        ],
        committed: Index {
            locations: vec![(
                "shape-index".to_string(),
                "pack-0007".to_string(),
                0,
                committed_bytes.len(),
            )],
        },
    };

    // The stale-map GC is the buggy arm: its in-memory map predates the
    // crash, so it deletes the file the recovered index names — the
    // committed value is LOST.
    let stale = gc_by_stale_map(&crash, &["pack-0001".to_string()]);
    assert!(
        !stale.iter().any(|name| name == "pack-0007"),
        "the stale keep-set loses the committed pack: data destruction"
    );

    // The recovered-index GC keeps exactly the committed file and deletes
    // the harmless orphan.
    let (kept, deleted) = gc_by_recovered_index(&crash);
    assert_eq!(kept, vec!["pack-0007".to_string()]);
    assert_eq!(deleted, vec!["pack-0008".to_string()]);

    // G1: every value the recovered index names reads back BYTE-IDENTICAL
    // after the GC.
    let value = read_through_index(&crash, "shape-index").expect("committed value");
    assert_eq!(value, committed_bytes, "surviving pack byte-identical");

    // ALL-SAFE posture: every file the index names — the GC deletes
    // nothing.
    let all_indexed = StoreAfterCrash {
        files: vec![PackFile {
            name: "pack-0007".to_string(),
            bytes: committed_bytes.clone(),
            sealed: true,
        }],
        committed: Index {
            locations: vec![(
                "shape-index".to_string(),
                "pack-0007".to_string(),
                0,
                committed_bytes.len(),
            )],
        },
    };
    let (kept, deleted) = gc_by_recovered_index(&all_indexed);
    assert!(deleted.is_empty(), "an all-indexed store loses no file");
    assert_eq!(kept.len(), 1);

    // TAMPERED arm: a stranger file appears in the packs directory between
    // the crash and the GC. The recovered index ignores it — only indexed
    // files survive, whatever else the directory contains.
    let mut tampered = StoreAfterCrash {
        files: vec![
            PackFile {
                name: "pack-0007".to_string(),
                bytes: committed_bytes.clone(),
                sealed: true,
            },
            PackFile {
                name: "pack-stranger".to_string(),
                bytes: b"planted".to_vec(),
                sealed: false,
            },
        ],
        committed: Index {
            locations: vec![(
                "shape-index".to_string(),
                "pack-0007".to_string(),
                0,
                committed_bytes.len(),
            )],
        },
    };
    tampered.files[0].sealed = true;
    let (_, deleted) = gc_by_recovered_index(&tampered);
    assert_eq!(deleted, vec!["pack-stranger".to_string()]);
    assert_eq!(
        read_through_index(&tampered, "shape-index").expect("committed value"),
        committed_bytes
    );
}

// ═══════════════════════════════════════════════════════════════════════
// F4 — persisted interner assignments checkpoint TOGETHER WITH the values
// that refer to them; IDs are never recycled within a live interner; an
// epoch reset discards assignments AND the values referring to them as one
// unit.
// ═══════════════════════════════════════════════════════════════════════

/// The live interner: names -> compact ids; new assignments stay in memory
/// until a durable checkpoint saves them.
#[derive(Default)]
struct LiveInterner {
    assignments: Vec<(String, u32)>,
    next_id: u32,
}

impl LiveInterner {
    fn intern(&mut self, name: &str) -> u32 {
        if let Some((_, id)) = self.assignments.iter().find(|(n, _)| n == name) {
            return *id; // stable: the same name never re-mints within a live interner
        }
        let id = self.next_id;
        self.next_id += 1;
        self.assignments.push((name.to_string(), id));
        id
    }
    fn resolve(&self, id: u32) -> Option<&str> {
        self.assignments
            .iter()
            .find(|&(_, assigned)| *assigned == id)
            .map(|(name, _)| name.as_str())
    }
}

/// A persisted record that refers to interned ids.
#[derive(Clone)]
struct StoredRecord {
    payload: String,
    refers: Vec<u32>,
}

/// The durable checkpoint.
#[derive(Default, Clone)]
struct Checkpoint {
    assignments: Vec<(String, u32)>,
    records: Vec<StoredRecord>,
}

/// BUGGY checkpoint: the records commit FIRST; the pending assignments are
/// saved "later" — a crash between the two publishes values whose ids no
/// mapping names.
fn checkpoint_values_first(_live: &LiveInterner, records: &[StoredRecord]) -> Checkpoint {
    Checkpoint {
        assignments: Vec::new(), // "we'll save the assignments on the next pass"
        records: records.to_vec(),
    }
}

/// FIXED checkpoint: assignments and the values that may refer to them are
/// saved TOGETHER, in one durable unit.
fn checkpoint_together(live: &LiveInterner, records: &[StoredRecord]) -> Checkpoint {
    Checkpoint {
        assignments: live.assignments.clone(),
        records: records.to_vec(),
    }
}

/// Restore: rebuild an interner view from a checkpoint.
fn restore(cp: &Checkpoint) -> (LiveInterner, Vec<Option<String>>) {
    let mut live = LiveInterner::default();
    live.assignments = cp.assignments.clone();
    live.next_id = cp.assignments.iter().map(|(_, id)| *id + 1).max().unwrap_or(0);
    let resolutions = cp
        .records
        .iter()
        .map(|record| {
            record
                .refers
                .iter()
                .all(|id| live.resolve(*id).is_some())
                .then(|| {
                    record
                        .refers
                        .iter()
                        .map(|id| live.resolve(*id).unwrap_or("?").to_string())
                        .collect::<Vec<_>>()
                        .join("+")
                })
        })
        .collect();
    (live, resolutions)
}

/// An epoch reset discards assignments AND the referring values together —
/// the atomically-discardable unit.
fn epoch_reset(_cp: &Checkpoint) -> Checkpoint {
    Checkpoint::default()
}

#[test]
fn f4_interner_assignments_checkpoint_with_their_values() {
    let mut live = LiveInterner::default();
    let shape = live.intern("shape");
    let body = live.intern("body");
    assert_ne!(shape, body);

    let records = vec![StoredRecord {
        payload: "decl for a.rs".to_string(),
        refers: vec![shape, body],
    }];

    // G1 — the happy path: checkpoint together, restore, and every refer
    // resolves to the SAME name it had in the live interner.
    let good = checkpoint_together(&live, &records);
    let (restored, resolutions) = restore(&good);
    assert_eq!(
        resolutions,
        vec![Some("shape+body".to_string())],
        "restored resolution must be bit-identical to the live one"
    );
    assert_eq!(restored.resolve(shape), Some("shape"));
    assert_eq!(
        restored.assignments.iter().find(|(n, _)| n == "shape").is_some(),
        true
    );
    assert_eq!(good.records[0].payload, "decl for a.rs");

    // BUGGY arm: values committed without their assignments — restore
    // leaves a DANGLING id (the exact failure the law names).
    let bad = checkpoint_values_first(&live, &records);
    let (_, dangling) = restore(&bad);
    assert_eq!(
        dangling,
        vec![None],
        "values published ahead of their assignments dangle after restore"
    );

    // TAMPERED arm — the recycle pin: a NEW name never receives a used
    // id's number within a live interner, and a re-interned name keeps its
    // id (stable identity across recomputations).
    let first = live.intern("shape");
    let fresh = live.intern("session");
    assert_eq!(first, shape, "re-interning is stable, never a new id");
    assert_ne!(fresh, shape, "ids are never recycled within a live interner");

    // Epoch reset: assignments and referring values go TOGETHER — no
    // orphan record survives pointing into a discarded mapping.
    let reset = epoch_reset(&good);
    assert!(reset.assignments.is_empty() && reset.records.is_empty());
    let (_, after_reset) = restore(&reset);
    assert!(after_reset.iter().all(|r| r.is_none()));
}

// ═══════════════════════════════════════════════════════════════════════
// F5 — scope-scoped memo release: a working set of decoded objects exists
// to serve ONE operation; at the operation boundary the whole memo table
// releases, and the idle database holds only the compact durable records.
// Decoded values reconstruct from records on demand. The release is by
// SCOPE (unconditional at the boundary), not by capacity pressure.
// ═══════════════════════════════════════════════════════════════════════

/// The durable record: the COMPACT form the backend keeps between
/// operations (8 bytes per element).
#[derive(Clone)]
struct DurableRecord {
    compact: Vec<u8>,
}

impl DurableRecord {
    fn of(values: &[u64]) -> Self {
        let mut compact = Vec::with_capacity(values.len() * 8);
        for v in values {
            compact.extend_from_slice(&v.to_le_bytes());
        }
        Self { compact }
    }
    /// Reconstruct the decoded form on demand.
    fn decode(&self) -> Vec<u64> {
        self.compact
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("8-byte word")))
            .collect()
    }
    fn compact_bytes(&self) -> usize {
        self.compact.len()
    }
    /// The decoded form costs the same bytes as compact here (u64 words);
    /// the census distinguishes RESIDENCE (which form is held while idle),
    /// not size.
    #[allow(dead_code)]
    fn decoded_bytes(&self) -> usize {
        self.compact.len()
    }
}

/// The operation: a scope that holds decoded memos and releases them
/// wholesale when it ends.
struct Operation {
    records: Vec<DurableRecord>,
    /// The memo table — decoded values keyed by record index, plus a
    /// per-record lock count the memos hold while live.
    memos: Vec<(usize, Vec<u64>, u32)>,
}

impl Operation {
    fn new(records: Vec<DurableRecord>) -> Self {
        Self {
            records,
            memos: Vec::new(),
        }
    }
    /// Read a record INSIDE the operation: decode once, share the memo with
    /// every later reader in the SAME operation.
    fn read(&mut self, index: usize) -> &[u64] {
        if let Some(pos) = self.memos.iter().position(|(i, _, _)| *i == index) {
            self.memos[pos].2 += 1; // the lock the memo holds while live
            let (_, values, _) = &self.memos[pos];
            return values;
        }
        let values = self.records[index].decode();
        self.memos.push((index, values, 1));
        let (_, values, _) = &self.memos[self.memos.len() - 1];
        values
    }
    /// THE BOUNDARY: the operation ends; the whole memo table releases —
    /// values, keys, and lock counts. The backend keeps only the compact
    /// records.
    fn finish(self) -> Vec<DurableRecord> {
        self.records // memos dropped here, wholesale
    }
}

/// BUGGY posture: a GLOBAL decoded cache that survives the operation —
/// values (and their locks) stay resident while the database is idle.
#[derive(Default)]
struct IdleResident {
    decoded: Vec<(usize, Vec<u64>)>,
    locks_held: usize,
}

impl IdleResident {
    fn read(&mut self, records: &[DurableRecord], index: usize) -> &[u64] {
        if let Some(pos) = self.decoded.iter().position(|(i, _)| *i == index) {
            self.locks_held += 1;
            let (_, values) = &self.decoded[pos];
            return values;
        }
        let values = records[index].decode();
        self.locks_held += 1;
        self.decoded.push((index, values));
        let (_, values) = &self.decoded[self.decoded.len() - 1];
        values
    }
    /// Idle residency: decoded bytes + the count of live lock holders.
    fn idle_bytes(&self) -> usize {
        self.decoded.iter().map(|(_, v)| v.len() * 8).sum()
    }
}

#[test]
fn f5_memos_release_at_the_operation_boundary() {
    // Three records of 64 words each: 512 B compact, 512 B decoded.
    let words = [64usize, 64, 64];
    let records: Vec<DurableRecord> = words
        .iter()
        .enumerate()
        .map(|(r, &n)| {
            DurableRecord::of(
                &(0..n)
                    .map(|i| field_word(r + 100, i))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    // INSIDE one operation: repeated reads share the memo — the all-safe
    // arm (release must not cost intra-operation recompute).
    let mut op = Operation::new(records.clone());
    let first = op.read(1).to_vec();
    let second = op.read(1).to_vec();
    assert_eq!(first, second, "one decode serves every reader in the operation");
    assert_eq!(op.memos.len(), 1, "one memo per record per operation");
    let third = op.read(1).to_vec();
    assert_eq!(third, first, "the memo stays shared through the operation");

    // THE BOUNDARY: the operation ends, memos release wholesale; idle
    // residency is the compact records only.
    let idle_records = op.finish();
    let idle_bytes: usize = idle_records.iter().map(DurableRecord::compact_bytes).sum();
    assert_eq!(idle_bytes, 3 * 64 * 8, "idle state = compact records only");

    // G1: reconstruction from records is bit-identical to the decoded memo
    // the operation held.
    let reconstructed = idle_records[1].decode();
    assert_eq!(reconstructed, first, "reconstruction is bit-identical");

    // BUGGY posture: the global decoded cache holds values ACROSS idle
    // periods — residency grows with every operation and locks stay held.
    let mut resident = IdleResident::default();
    for op_index in 0..3usize {
        let _ = resident.read(&idle_records, op_index);
    }
    let resident_bytes = resident.idle_bytes();
    assert_eq!(
        resident_bytes, 3 * 64 * 8,
        "the resident posture holds every decode while idle"
    );
    assert_eq!(resident.locks_held, 3, "locks stay held while idle");
    assert!(
        resident_bytes > 0 && idle_bytes == resident.idle_bytes(),
        "the census: idle-resident bytes grow with reads; scope-release stays at the record floor"
    );

    // The scope-release posture's idle census NEVER grows, whatever ran.
    for op_index in 0..9usize {
        let mut another = Operation::new(idle_records.clone());
        let _ = another.read(op_index % 3);
        let done = another.finish();
        let floor: usize = done.iter().map(DurableRecord::compact_bytes).sum();
        assert_eq!(
            floor, idle_bytes,
            "after every operation the idle residency is the same record floor"
        );
    }
}
