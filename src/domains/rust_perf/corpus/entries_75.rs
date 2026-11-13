//! Batch 359 — the ajika persistent-incrementality `rust_perf` batch
//! (Plan 370 / Research 403): the corpus's FIRST persistent-incremental-
//! computation family — salsa-shaped memoization made durable (0 owners
//! before this batch; the dedup greps `memo|incremental|salsa|invalidat|
//! recompute` over `rust_perf` + `kernel_opt` + `sec` hit prose and the
//! measurement-honesty/nearest-neighbor families only). Five laws: one
//! BENCHABLE GOAT lead (named outputs + lazy handles, the partial-decode
//! read path, measured on the `PerfBenchValidator`) + four suggestion-only
//! `NotBenchable-by-domain` decision-content rules (the throw-away cache,
//! the recovered-index GC, the persisted interner, the scope-released
//! memos — their failure axes are idle-memory residency, crash-recovery
//! integrity and checkpoint ordering, none of which a CPU Pareto validator
//! hosts).
//!
//! Source: `rust-glancer/ajika` @ `70c1e26df847d3da10d2a59a5a6aefe37db2bf55`
//! (v0.1.7, MIT OR Apache-2.0, main tip UNMOVED at T0 this batch — the
//! 2026-11-12 verdict pin is the batch pin; clone removed at close) + the
//! handwritten blog at `rust-glancer.github.io/blog/persistent-
//! incrementality-in-rust` (2026-11-06). The author discloses LLM-assisted
//! development; the IN-SOURCE module docs are clean and quotable; the
//! Ajika BOOK is LLM-generated and is never quote material. Provenance
//! lineage (Research 403): Ajika's DESIGN is the red-green/memo model
//! salsa popularized (cite salsa-rs/salsa + the rust-analyzer "Durable
//! Incrementality" blog as the upstream idea) — the rules below are
//! engine-agnostic decision content, not salsa-specific.
//!
//! ROUTING per row: `rust_perf` — storage/serialization/hot-path laws over
//! a durable store; `kernel_opt`'s GPU clauses lose (zero GPU surface), sec
//! loses because the throw-away law's threat model is bit-rot/version skew
//! (recomputable-state robustness), not attacker-facing input validation —
//! the sec corpus's `execution-artifact-trust-ladder-before-deserialize`
//! owns the OPPOSITE polarity (execution-capable artifacts verify before
//! deserialize; recomputable caches discard and recover).
//!
//! ORIGINAL Rust fixtures (`corpus/fixtures/b359_fixtures.rs`, 5/5
//! standalone-compiled AND executed before entry — rustc --edition 2021
//! --test, four-arm discipline per law), never pasted source.
//! Batch 359 / Plan 370 / Research 403.

#[allow(clippy::too_many_lines)] // the corpus entries ARE the payload
pub fn entries() -> Vec<crate::domains::rust_perf::PerfRule> {
    vec![
        // ── R1 — the GOAT lead: a persisted computation that produces
        // several answers together emits NAMED, independently readable
        // outputs and returns cheap handles; consumers decode one field on
        // demand; the unread fields never deserialize. ───────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "named-outputs-lazy-handles-partial-decode".to_string(),
            name: "a persisted computation that produces several answers together emits NAMED, independently readable outputs and returns cheap address handles — a consumer that reads one answer decodes one field on demand, and the unread fields never deserialize; handing back one whole-payload blob forces every read to decode O(total)".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: one query computes K answers and persists them as ONE
// payload blob. Every consumer gets the whole blob, so reading any
// single answer deserializes all K fields — a small read costs
// O(total) bytes and K decodes.
struct WholePayload {
    fields: Vec<Vec<u64>>, // K persisted fields, decoded together
}

fn read_one_answer(payload: &WholePayload, slot: usize) -> u64 {
    // the whole-payload contract: decode EVERY field to use ANY field
    let mut answer = 0u64;
    for (i, field) in payload.fields.iter().enumerate() {
        let decoded = field.clone(); // O(field) work, paid K times
        if i == slot {
            answer = decoded[0];
        }
    }
    answer
}"#.to_string(),
            fixed_code: r#"// REMEDY: emit named outputs and return ADDRESS handles. The handle
// carries only (producer, slot) — cheap to obtain, cheap to copy,
// cheap to persist; the value decodes on demand. A read of one named
// output touches O(field) bytes and the unread fields never
// deserialize. REGIME: big persisted outputs read partially (any
// durable memoization/salsa-shaped engine, index + payload stores,
// per-file derived caches). ANTI-REGIME: every consumer reads every
// field (the framing/handle layer is pure overhead — the read-all
// posture decodes the same volume both ways); tiny outputs where
// framing ≈ payload; consumers that need a POINT-IN-TIME snapshot — a
// handle is an address, it reads the CURRENT stored bytes (a kept
// handle asks for the new answer, never a pinned old version).
struct NamedOutputs {
    fields: Vec<Vec<u64>>, // the same storage, named
}

#[derive(Clone, Copy)]
struct OutputHandle {
    slot: usize, // the address: which producer, which slot
}

fn emit_named_outputs(k: usize) -> (NamedOutputs, Vec<OutputHandle>) {
    let fields = (0..k).map(|f| vec![f as u64; 256]).collect();
    let handles = (0..k).map(|slot| OutputHandle { slot }).collect();
    (NamedOutputs { fields }, handles)
}

fn read_via_handle(out: &NamedOutputs, handle: OutputHandle) -> u64 {
    // decode ONE named output on demand — the unread fields never
    // deserialize
    out.fields[handle.slot][0]
}"#.to_string(),
            context_notes: Some("MEASURED ParetoDominant (rust_perf_bench validator, amo, 2026-11-13, the partial-read posture at sizes [8, 64, 256] fields of n words, reading the middle field — the full record in this note's tail). UPSTREAM: present at pin (rust-glancer/ajika @ 70c1e26d, v0.1.7, MIT OR Apache-2.0 — their own design; decision content only; the 2026-11-12 verdict pin is the batch pin, tip unmoved at T0). THE LAW IN-SOURCE (runtime/output.rs module doc, byte-verified at the pin): 'Some work naturally produces several answers together, but consumers do not need all of them. A query can emit named outputs and return a set of `Output` handles.' + (the struct doc) 'The handle holds an address, not the table or a particular version of it. Keeping or saving the handle does not keep the table in memory.' + the encode law 'Only the identity is encoded. T names the value to decode later and need not have any particular representation for the selector itself to round-trip.' Blog (handwritten, §Outputs): 'Handle is cheap to obtain and deserialize, and you only query the data you actually need.' The handle's snapshot-misuse law (output.rs value doc): 'A handle kept from an earlier revision asks for the new answer, not the old one' — the tampered fixture arm pins it (replacing the stored bytes is visible through the old handle). The read side counts what happened (Statistics): 'Values decoded from storage' (decoded) and 'Encoded payload bytes loaded for decoding' (bytes_read) — the partial read's decoded==1/bytes==field vs the whole read's decoded==K/bytes==total is the source's own census shape. REGIME + ANTI-REGIME above (the framing tax is honest overhead: at the read-all posture both arms decode the same volume — the fixture's all-safe arm pins value + volume identity). ADJUDICATIONS (do not double-count): kernel_opt load-time-decode-blind-transform-absorption (gpu_149) owns ABSORPTION-AT-LOAD — re-shaping the decode table once so the hot decode never sees format features (equivalence-pinned, no delta); THIS owns DECODE-ON-DEMAND — the read path itself reads less (the unread fields never deserialize; the win is volume, not format-blindness; compose: absorb at load, then read partially). launch-cost-scales-with-allocation-count-pack-one-arena (entries_36) + co-lifecycle-rows-pack-one-entry-strided-views (entries_73) own PACKING many items into ONE allocation to amortize lifecycle ops — THIS is the OPPOSITE pole on the read side: one persisted unit stays WHOLE at rest, and the partial read is served by addressing INTO it without deserializing the rest (pack when writing/lifecycle-bound; name-and-address when reading is partial). eager-pointer-collect-lazy-fetch (entries_05) owns fetch-time pointer-materialization avoidance in memory — THIS owns the durable-storage read volume. FIXTURE (b359_fixtures F1): 5/5 standalone-compiled AND executed before entry; F1 = the bench harness input (the arms, field shape 256 u64 words, and the rotate-xor checksums ARE the spec's). BENCH (partial-read posture, sizes [8,64,256] fields of n words, two-run stability: run 1 = run 2 verdict class identical): ParetoDominant 3/0/0 BOTH runs, speedups 7.48x/49.65x/158.11x (run 2: 7.18x/55.10x/161.96x), allocs 16->2 / 128->2 / 512->2, alloc bytes 1,024->128 / 65,536->1,024 / 1,048,576->4,096 (the bytes axis = K:1 at every size — the >=3x bytes-read bar cleared 8x at n=8 and 256x at n=256); the anti-regime read-all posture (same sizes, the consumer reads every field) measured Tie/Tie/Tie at all three sizes in BOTH runs (1.02x/1.02x/1.00x) — the framing tax does not regress the full consumer (batch359_goat re-runs both postures on demand). G1 bit-identical answers enforced by the validator's parity + F1's exact checksums. Cite: @ 70c1e26d — crates/ajika/src/runtime/output.rs:1-11,56-99 (module + struct + value docs), evaluate.rs:128-141 (db.output emission); the blog §Outputs. Batch 359 / Plan 370 / Research 403.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(named_outputs_lazy_handles_partial_decode_bench_spec()),
        },
        // ── R2 — the derived cache is THROW-AWAY: it resets on a
        // schema/version mismatch and recovers-over-hard-fail on
        // corruption; the app never crashes on a cache problem. ──────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "derived-cache-throwaway-version-reset-recovery".to_string(),
            name: "a derived cache is a disposable optimization, never an asset — writer and app schema versions identify the revision and a mismatch RESETS the whole cache, corruption is discarded-and-recovered (recompute beats crash), an uncertain commit recovers via reopen of the same owner, and a healthy cache still reuses everything; the app never refuses to start over its own cache".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: the cache is treated as an asset. A version mismatch or
// corrupted bytes makes open() a hard failure — the app refuses to
// start past its own optimization, and an uncertain commit is
// unrecoverable because nothing knows what landed.
#[derive(Debug)]
enum CacheError {
    VersionMismatch { stored: String, app: String },
    Corrupt,
}

fn open_as_asset(stored_schema: &str, stored: &[u8], app_schema: &str) -> Result<Cache, CacheError> {
    if stored_schema != app_schema {
        // the old cache "can't be loaded" — fatal
        return Err(CacheError::VersionMismatch {
            stored: stored_schema.to_string(),
            app: app_schema.to_string(),
        });
    }
    if !checksum_ok(stored) {
        return Err(CacheError::Corrupt); // a corrupt cache kills the app
    }
    Ok(Cache::load(stored))
}

struct Cache;

fn checksum_ok(_bytes: &[u8]) -> bool {
    true
}

impl Cache {
    fn load(_bytes: &[u8]) -> Self {
        Self
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: the disposable-cache contract. Writer + app schema versions
// identify the revision: a mismatch RESETS (all derived state goes, the
// source of truth rebuilds). Corruption is discarded-and-recovered —
// recompute beats crash. An uncertain commit recovers by reopening the
// same owner: the recovered checkpoint decides what is visible. A
// healthy, matching cache reuses everything (the reset path costs
// nothing when nothing is wrong). REGIME: any durable derived state —
// memoization caches, analysis indexes, build caches. ANTI-REGIME: the
// cache IS the source of truth (nothing to recompute from — then the
// ladder is verify-and-refuse, not throw-away); multi-tenant shared
// caches where "exclusively owned" does not hold.
#[derive(Debug)]
enum OpenOutcome {
    FullReuse,                     // healthy + matching: zero recompute
    Reset { discarded: usize },    // version mismatch: throw the whole thing
    Recovered { discarded: usize }, // corruption: discard the corrupt part
}

fn open_throwaway(
    stored_schema: &str,
    stored: &[u8],
    app_schema: &str,
    source_len: usize,
) -> OpenOutcome {
    if stored_schema != app_schema {
        // "Both Ajika and application supply schema versions ... if the
        // stored cache doesn't match it, we reset it."
        return OpenOutcome::Reset { discarded: stored.len() };
    }
    if !checksum_ok(stored) {
        // recovery over hard failing, even if it means re-computing
        // everything from scratch
        return OpenOutcome::Recovered { discarded: stored.len() };
    }
    let _ = source_len; // healthy: reuse as-is
    OpenOutcome::FullReuse
}

struct Cache;

fn checksum_ok(bytes: &[u8]) -> bool {
    !bytes.iter().all(|&b| b == 0xFF) // stand-in integrity check
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the failure axes are availability and recovery integrity — refuse-to-start vs recover; no CPU validator hosts a durable-cache lifecycle — the dial9/Batch-136 annotation, not a demotion). UPSTREAM: present at pin (rust-glancer/ajika @ 70c1e26d, MIT OR Apache-2.0 — decision content only, tip unmoved at T0). THE LAW IN-SOURCE (ajika-storage/src/lib.rs module doc, byte-verified at the pin): 'The directory is a disposable cache, exclusively owned until `Database` drops. Opening can discard corrupt cache data. After an uncertain commit, call `Database::reopen` on the same owner to recover the published checkpoint.' Blog (handwritten, §Enter persistency): 'the cache must be _throw-away_. Both Ajika and application supply schema versions to identify the revision, and if the stored cache doesn't match it, we reset it. If cache gets corrupted, Ajika will attempt recovery over hard failing, even if it means re-computing data from scratch.' + the application-posture sentence: 'user would _most likely_ prefer the cache to be discarded and computations to be repeated one more time, rather than the app crashing with \"cache is not available, what do I do???\"'. The epoch shape (runtime/mod.rs open doc): 'A different application epoch discards them when opening the directory. ... Change the epoch when query semantics or stored types change. Ajika detects changes in tracked data, but cannot infer that editing a query's Rust implementation changes its answer' — the writer+app VERSION PAIR is the reset key (format::FORMAT 'ajika-packs-2' + the app epoch compose into one identity). The reset path in-source (database/mod.rs open): InvalidCache -> reset(None) (clear query data, KEEP the identity table) -> still InvalidCache -> reset(Some(&[])) (discard assignments too, 'identities_discarded'), warn 'discarded corrupt cache contents' — a two-rung discard ladder, query data first, identities last. reopen(): 'After an uncertain commit, the recovered index tells us which packs to keep.' REGIME + ANTI-REGIME above. ADJUDICATIONS (do not double-count): sec execution-artifact-trust-ladder-before-deserialize (B345) owns the OPPOSITE polarity — EXECUTION-CAPABLE artifacts (pickle/wasm/dylib) are verified before deserialize and tampered bytes refuse REGARDLESS of consent, because the artifact can run code; THIS owns RECOMPUTABLE derived state where the correct failure response is discard-and-recompute (a cache problem is an availability event, not a security event; the threat model is bit-rot/version skew, not an attacker). rust_perf resume-revalidates-provenance-before-reuse (entries_05, B121) owns refuse-and-stop on a BUILD-pipeline identity mismatch (source+calibration+tool — mixing outputs across versions is the hazard, so mismatch REFUSES the run); THIS owns discard-and-continue on DERIVED state (the source of truth is intact, so the correct response is reset+recompute, never refuse-to-start). entries_19 fastest-of-n-dip-drift-retime-cost-table owns MEASUREMENT-NOISE repair of a captured cost table (a slow STARTUP SAMPLE poisons one entry; the repair re-times the entry/table); THIS owns VERSION/CORRUPTION reset (the whole revision is wrong, not one noisy sample — there is nothing to re-time, the data is discarded). The positive home instance (recorded, no double-land): refine's own index_cache (issue 224/226) — content-addressed key over corpus + retrieval config + format versions, corrupt/stale refuses loud and rebuilds; same discard-and-recompute posture, one home owner already. FIXTURE (b359_fixtures F2): healthy reuse / mismatch reset (discarded count named) / corrupt recover / uncertain-commit reopen, the buggy arm's Fatal posture pinned at every cache problem, G1 = recovery is answer-identical to the source of truth. Cite: @ 70c1e26d — crates/ajika-storage/src/lib.rs:1-11 (module doc), database/mod.rs:120-160 (the reset ladder + reopen), runtime/mod.rs:102-108 (epoch law); the blog §Enter persistency. Batch 359 / Plan 370 / Research 403.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R3 — pack-store GC: files seal BEFORE their locations publish;
        // after a commit attempt ONLY the recovered index decides what is
        // safe to delete. ─────────────────────────────────────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "pack-store-gc-decides-by-recovered-index".to_string(),
            name: "a multi-blob persisted store writes values to append-only pack files that seal BEFORE their locations publish, so a failed preparation leaves harmless orphans — and once a commit has been attempted, ONLY the recovered index decides which files are safe to delete; a GC driven by an in-memory keep-set (or by directory listing) destroys committed data after any crash".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: the GC decides from an in-memory map of "packs I know about".
// After a crash, the map predates the recovered index: it deletes the
// sealed file whose locations DID publish (committed data, lost), and
// keeps whatever it happens to remember.
#[derive(Default)]
struct Store {
    known_packs: Vec<String>,   // the stale in-memory keep-set
    files: Vec<(String, Vec<u8>)>,
    index: Vec<(String, String, usize, usize)>, // key -> (file, offset, len)
}

impl Store {
    /// The buggy GC: the keep-set is "what I remember".
    fn collect_garbage(&self) -> Vec<String> {
        let mut deleted = Vec::new();
        for (name, _) in &self.files {
            if !self.known_packs.iter().any(|known| known == name) {
                // a committed-but-not-yet-known pack is "garbage"
                deleted.push(name.clone());
            }
        }
        deleted
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: the commit protocol makes orphans harmless, and the recovered
// index is the only deletion authority. (1) Values append to PRIVATE
// files with names safe to create before the checkpoint commits. (2) The
// file SEALs (flush + directory sync) BEFORE the index names it. (3) A
// failed preparation leaves harmless orphans — no reader can reach them
// because readers only use names in the index. (4) Once commit has been
// ATTEMPTED, reopen recovers the index and ONLY the files it names
// survive; everything else in the directory is deleted. (5) A deletion
// failure is deferred (tolerated, retried next pass) — it never changes
// what the index says exists. REGIME: any multi-blob store — pack files,
// chunk stores, log-structured caches. ANTI-REGIME: single-artifact
// publishes (one file + atomic rename — write-verify-readback-before-
// publish owns that, with its read-back gate this GC does not replace);
// stores whose index itself is rebuilt from the files (content-
// addressed: the files ARE the index).
struct Store {
    files: Vec<(String, Vec<u8>)>,
    index: Vec<(String, String, usize, usize)>, // the publication authority
}

impl Store {
    /// Reopen: the recovered index decides.
    fn recovered_index(&self) -> Vec<String> {
        let mut names: Vec<String> =
            self.index.iter().map(|(_, file, _, _)| file.clone()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// The fixed GC: keep exactly what the recovered index names; delete
    /// every other file (orphans included). Deletion failures defer.
    fn collect_garbage(&self) -> (Vec<String>, Vec<String>) {
        let keep = self.recovered_index();
        let mut kept = Vec::new();
        let mut deleted = Vec::new();
        for (name, _) in &self.files {
            if keep.iter().any(|known| known == name) {
                kept.push(name.clone());
            } else {
                deleted.push(name.clone());
            }
        }
        (kept, deleted)
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the failure axis is crash-recovery data integrity — committed-but-unindexed bytes surviving GC; no CPU validator hosts a crash/reopen cycle — the dial9/Batch-136 annotation, not a demotion). UPSTREAM: present at pin (rust-glancer/ajika @ 70c1e26d, MIT OR Apache-2.0 — decision content only, tip unmoved at T0). THE LAW IN-SOURCE (ajika-storage/src/pack/mod.rs module doc, byte-verified at the pin): 'Append values to private files, then seal them before publishing their locations. A unique file name is safe to create before the checkpoint commits: readers only use names in the index. Failed preparation leaves harmless orphans. Once commit has been attempted, only the recovered index can decide which files are safe to delete.' The publication authority (transaction/mod.rs module doc): 'The index is only publication authority. New frames may already exist on disk while a write is pending, but no reader can find them through the old index. Pack accounting changes in the same commit as locations, so deleting the last value can retire its entire file.' The seal (pack/mod.rs finish): 'Complete the bytes and file names before the index commit can name them' — flush + sync_all + directory sync. The GC itself (database/mod.rs reconcile_files): the keep-set = every PACKS-table name, then per-directory-entry: keep.contains -> survive, else remove_file (NotFound tolerated; OTHER failures deferred with 'pack cleanup deferred' warn + reconciliation_due stays set — the deferred-cleanup arm), never a data decision. Pack rollover at TARGET_BYTES = 64 * 1024 * 1024 (pack/mod.rs:21). REGIME + ANTI-REGIME above. ADJUDICATIONS (do not double-count): rust_perf write-verify-readback-before-publish (entries_05, B121) owns the SINGLE-ARTIFACT publish gate — write hidden, re-read through an independent reader, hash-compare, THEN atomic-rename (content verification before one file becomes visible); THIS owns a STORE's crash-safe GC over MANY blobs — no content read-back, the question is which FILES the recovered index still names after a crash (seal-before-publish + index-as-authority; compose: the read-back gate publishes one artifact, this GC reclaims around it). entries_05's artifact assembly (the .incomplete + rename idiom) is one artifact's verify+rename — THIS is many blobs + GC-after-recovery. resume-revalidates-provenance-before-reuse (entries_05) owns resume identity checks, not deletion. persistent-cache-teardown-needs-second-witness (entries_39) owns the teardown half (destroying a cache needs a second witness) — the ADJACENT-POLE: teardown refuses to delete cache data without independent confirmation; THIS deletes PRECISELY BECAUSE one authority (the recovered index) answers the question (the store's own recovery is the witness). vanished-entry-tolerant-eviction-panic-else (entries_69) owns the shared-directory eviction ERROR classification (peer NotFound vs own corruption); THIS owns WHICH files survive at all (the keep-set source), compose at the remove_file call site. FIXTURE (b359_fixtures F3): crash-after-seal (committed pack survives, orphan deleted), stale-map GC loses the committed pack (the buggy arm), all-indexed deletes nothing, the stranger-file tampered arm (the index ignores unindexed files), G1 = every indexed value byte-identical after GC. Cite: @ 70c1e26d — crates/ajika-storage/src/pack/mod.rs:1-6,21,163-186; transaction/mod.rs:1-5; database/mod.rs:520-560 (reconcile_files). Batch 359 / Plan 370 / Research 403.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R4 — the DROP-OR-KEEP gate resolved KEEP: the persisted-interner
        // checkpoint-ordering law is a distinct axis (intern* greps over
        // rust_perf + kernel_opt own size-axis interning and a GPU input
        // intern table, never persistence/checkpoint ordering). ───────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "persisted-interner-assignments-checkpoint-with-refering-values".to_string(),
            name: "persisted records that reference interned IDs are valid only while the interner state that minted them persists — assignments checkpoint TOGETHER WITH (never after) the values that may refer to them, IDs are never recycled within a live interner, and an epoch reset discards assignments and the values referring to them as one unit".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: records commit first; the pending interner assignments are
// saved "on the next pass". A crash between the two publishes values
// whose IDs no mapping names — and if a new name later reuses the
// number, the restored record silently points at the WRONG field.
#[derive(Default)]
struct Interner {
    assignments: Vec<(String, u32)>, // pending, in memory
    next_id: u32,
}

struct Checkpoint {
    records: Vec<StoredRecord>,
    assignments: Option<Vec<(String, u32)>>, // "saved later"
}

#[derive(Clone)]
struct StoredRecord {
    payload: String,
    refers: Vec<u32>,
}

fn checkpoint_values_first(
    live: &Interner,
    records: &[StoredRecord],
) -> Checkpoint {
    Checkpoint {
        records: records.to_vec(),
        assignments: None, // the next pass will save them (it never does)
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: one durable unit. (1) Assignments save TOGETHER WITH the
// values that may refer to them — a checkpoint that names a record
// names every ID that record uses. (2) IDs never recycle within a live
// interner: the same name re-interns to the SAME id, and a new name
// never receives a used number (stable identity across recomputations).
// (3) An epoch/full reset discards assignments AND the values referring
// to them TOGETHER — no orphan record survives pointing into a
// discarded mapping. (4) An ID is only meaningful with its interner:
// another database can assign the same number to different fields, so
// never carry IDs across stores. REGIME: any durable engine whose
// records reference interned entities (paths, symbols, chunks). 
// ANTI-REGIME: in-memory-only interners (no checkpoint boundary — a
// plain map suffices); IDs derived from content (self-validating, no
// assignment state to persist).
struct Interner {
    assignments: Vec<(String, u32)>,
    next_id: u32,
}

#[derive(Clone, Default)]
struct Checkpoint {
    records: Vec<StoredRecord>,
    assignments: Vec<(String, u32)>, // saved WITH the records
}

#[derive(Clone)]
struct StoredRecord {
    payload: String,
    refers: Vec<u32>,
}

impl Interner {
    fn intern(&mut self, name: &str) -> u32 {
        if let Some((_, id)) = self.assignments.iter().find(|(n, _)| n == name) {
            return *id; // stable: never re-mints within a live interner
        }
        let id = self.next_id;
        self.next_id += 1;
        self.assignments.push((name.to_string(), id));
        id
    }
}

fn checkpoint_together(live: &Interner, records: &[StoredRecord]) -> Checkpoint {
    Checkpoint {
        records: records.to_vec(),
        assignments: live.assignments.clone(),
    }
}

fn epoch_reset() -> Checkpoint {
    Checkpoint::default() // assignments AND referring values go together
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the failure axis is checkpoint-ORDERING integrity — dangling IDs after restore, silent aliasing after a recycle; no CPU validator hosts a durable interner — the dial9/Batch-136 annotation, not a demotion). UPSTREAM: present at pin (rust-glancer/ajika @ 70c1e26d, MIT OR Apache-2.0 — decision content only, tip unmoved at T0). THE LAW IN-SOURCE (src/interner/mod.rs, byte-verified at the pin): 'New assignments stay in memory until a durable query checkpoint saves them alongside the values that may refer to them. They are never rolled back or reused within a live database.' + 'IDs are never recycled within a live interner. Query collection and `Database::invalidate` preserve them, but a full cache reset, such as an epoch change, can discard assignments together with the query data referring to them.' + the cross-store law: 'An ID is only meaningful with its interner: another database can assign the same number to different fields.' Blog (handwritten, §Interner): 'IDs in Ajika are persisted in addition to being stored in memory, and on restart this interner state is reloaded.' The reset ladder ordering (database/mod.rs open) is the mechanism: reset(None) clears QUERY DATA and KEEPS the identity table (assignments outlive the query data that referenced them — correct, the values went with the reset); reset(Some(&[])) additionally discards assignments — the SECOND rung, only when the index itself is unreadable, and both rungs discard assignments + referring values as one unit. REGIME + ANTI-REGIME above. ADJUDICATIONS (do not double-count): skewed-payload-inline-encode-intern-overflow-tail (entries_66) owns the SIZE axis of interning (overflow tails interned deduped to shrink wire format — interning as layout); THIS owns the PERSISTENCE axis (when the ID mapping commits relative to the records that reference it). kernel_opt host-scalar-kernel-input-intern-table (gpu_123) owns a GPU input-staging intern table (device-side scalar staging); THIS owns durable-checkpoint ordering (no device in the loop). tune-cache-keyed-by-spelled-variant-names (entries_50) owns persisted-decision keys surviving renames (tune winners keyed on spelled strings — identity stability of RECORDED DECISIONS); THIS owns the identity stability of REFERENCED ENTITIES inside restored data (compose: the spelled-key law names what to key on, this law names when the mapping must commit). The drop-or-keep gate resolved KEEP: three vocabulary families (size-axis interning, GPU intern tables, keyed-decision identity) own adjacent axes; zero owners of checkpoint-ordering. FIXTURE (b359_fixtures F4): values-first checkpoint dangles after restore (the buggy arm), together-checkpoint resolves bit-identically, the recycle pin (re-intern stable, new name never gets a used id), the epoch reset discards both as one unit. Cite: @ 70c1e26d — crates/ajika/src/interner/mod.rs:1-10,15-36; database/mod.rs:120-140 (the reset ladder). Batch 359 / Plan 370 / Research 403.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R5 — the DROP-OR-FACE gate resolved LAND: the scope-release axis
        // has zero owners (the eviction families free by CAPACITY under
        // pressure; eager-pointer-collect-lazy-fetch avoids fetch-time
        // materialization, not operation-boundary release) and a face would
        // bury the boundary law under a fetch law. The vocabulary collision
        // (ajika "transient" queries vs the GPU buffer-lifetime family) is
        // adjudicated in-rule below. ──────────────────────────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "scope-scoped-memo-release-idle-holds-no-decoded-state".to_string(),
            name: "a working set of decoded objects exists to serve ONE operation — at the operation boundary the whole memo table releases (values, keys, metadata, locks), the idle database holds only the compact durable records, and the next operation reconstructs decoded values from records on demand; residency is bounded by the scope, never parked while idle".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: decoded values live in a GLOBAL cache that survives every
// operation. Nothing releases at the boundary, so the idle process
// holds every decode it ever served — plus the per-record locks the
// cached entries keep — and residency grows with the workload's whole
// history instead of its working set.
#[derive(Default)]
struct GlobalDecodedCache {
    decoded: Vec<(usize, Vec<u64>)>, // record index -> decoded values
    locks_held: usize,               // each cached entry holds one
}

impl GlobalDecodedCache {
    fn read(&mut self, records: &[DurableRecord], index: usize) -> &[u64] {
        if let Some(pos) = self.decoded.iter().position(|(i, _)| *i == index) {
            self.locks_held += 1;
            return &self.decoded[pos].1;
        }
        let values = records[index].decode();
        self.locks_held += 1;
        self.decoded.push((index, values));
        &self.decoded[self.decoded.len() - 1].1
    }
    /// Idle residency: everything ever decoded, held forever.
    fn idle_bytes(&self) -> usize {
        self.decoded.iter().map(|(_, v)| v.len() * 8).sum()
    }
}

struct DurableRecord {
    compact: Vec<u8>, // the compact durable form
}

impl DurableRecord {
    fn decode(&self) -> Vec<u64> {
        self.compact
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }
}"#.to_string(),
            fixed_code: r#"// REMEDY: the memo table is OPERATION-SCOPED. Inside the scope, one
// decode is shared by every reader (release must not cost intra-operation
// recompute). At the boundary the WHOLE table releases — values, keys,
// metadata, locks — unconditionally (by SCOPE, not by capacity pressure:
// no LRU bookkeeping, no pressure signal needed). The idle state holds
// only the compact durable records; the next operation reconstructs
// decoded values from records on demand, bit-identically. REGIME: long-
// lived analysis/IDE sessions, request-scoped serving, batch jobs — any
// engine whose durable layer can reconstruct decoded forms. ANTI-REGIME:
// the durable layer cannot reconstruct (decode is one-way or
// source-of-truth is gone — then an eviction family with capacity bounds
// is the honest shape); single-operation processes where scope and
// process lifetime coincide (release-at-boundary degenerates to
// drop-at-exit).
struct Operation {
    records: Vec<DurableRecord>,
    memos: Vec<(usize, Vec<u64>, u32)>, // (record, decoded, readers holding it)
}

impl Operation {
    fn read(&mut self, index: usize) -> &[u64] {
        if let Some(pos) = self.memos.iter().position(|(i, _, _)| *i == index) {
            self.memos[pos].2 += 1;
            return &self.memos[pos].1;
        }
        let values = self.records[index].decode(); // reconstruct on demand
        self.memos.push((index, values, 1));
        &self.memos[self.memos.len() - 1].1
    }
    /// THE BOUNDARY: the operation ends; the whole memo table releases.
    fn finish(self) -> Vec<DurableRecord> {
        self.records // memos dropped here, wholesale
    }
}

struct DurableRecord {
    compact: Vec<u8>,
}

impl DurableRecord {
    fn decode(&self) -> Vec<u64> {
        self.compact
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }
    fn compact_bytes(&self) -> usize {
        self.compact.len()
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the win axis is idle-memory residency — bytes held between operations and lock counts; no CPU Pareto validator hosts an idle-residency census — the dial9/Batch-136 annotation, not a demotion). UPSTREAM: present at pin (rust-glancer/ajika @ 70c1e26d, MIT OR Apache-2.0 — decision content only, tip unmoved at T0). THE LAW IN-SOURCE (runtime/cache.rs module doc, byte-verified at the pin): 'The backend owns completed records between operations; these working objects do not keep their keys, output metadata or locks resident while the database is idle.' + (Memo doc) 'Its decoded values remain available to other callers until the operation finishes. The next operation creates a new memo and recovers the record and validity proof from the backend.' + (State doc) 'After an operation, the entire state is released. Completed records and validity live in the backend.' Blog (handwritten, §Inside Ajika): 'Record contains information like values metadata for a query and dependencies for it (note: values are stored separately!). Thus, Memos are released on the end of the run and then later can be reconstructed from Record on demand.' + the key part: 'released memos don't occupy RAM. This is the key part of Ajika! During the query execution, the values can be loaded, but if you don't run any operations, you don't have to pay for it. Outputs will just wait for you inside of the filesystem! Moreover, whenever we run queries, we only load what we need, not everything.' REGIME + ANTI-REGIME above. THE DISTINCT AXIS (the drop-or-face adjudication): eviction families (slru-probation-protected-promote-on-second-touch entries_49, cache-budget-first-sessions-take-remainder-never-reshrink entries_37, fixed-ratio-pool-partition-strands-demand entries_73, hot-slot-recycler flat tables) free by CAPACITY under PRESSURE — bookkeeping decides victims; THIS releases by SCOPE unconditionally at the operation boundary — no pressure signal exists, no victim selection, the whole table goes (compose: scope-release for the operation working set, capacity-eviction for anything that must outlive a single operation). eager-pointer-collect-lazy-fetch (entries_05) owns fetch-time pointer-materialization avoidance (don't pre-materialize an index of everything to serve a subset); THIS owns the operation-BOUNDARY release of already-decoded state with durable reconstruction (a face would bury the boundary law under a fetch law — hence LAND, and the adjudication is mandatory in-rule). kernel_opt transient-spike-pins-persistent-scratch (gpu_05) + snapshot-transient-grad-for-deferred-download (gpu_02) own GPU BUFFER lifetimes under in-flight encoders/scratch reuse — the word 'transient' collides with ajika's TRANSIENT QUERIES (computed once, retained for the operation, dropped instead of persisted — runtime/mod.rs open doc + the blog §Enter persistency), but the axes are disjoint: GPU scratch residency is measured in bytes pinned by a historical max on live device buffers; THIS is host-side decoded-state reconstruction from a DURABLE layer, no device in the loop. In-memory-only memos (mask-memo-hoist-to-owner-lifetime entries_09, range-table-page-memoization entries_29, batch-local-memoization-negative-cache-tombstone entries_06, single-entry-last-key-lookup-cache-hash-avoidance entries_66) memoize INTO an owner that outlives calls — THIS releases at the scope boundary into a durable layer that can reconstruct; the two compose (owner-lifetime memoization INSIDE the operation, scope-release AT the boundary). FIXTURE (b359_fixtures F5): one decode serves every reader in the operation (the all-safe arm), the boundary releases wholesale (idle residency = the compact record floor, constant across 9 replayed operations), the buggy global cache holds every decode + locks while idle, G1 = reconstruction is bit-identical. Cite: @ 70c1e26d — crates/ajika/src/runtime/cache.rs:1-10,26-51,84-96 (module/Memo/State/Table docs); the blog §Inside Ajika. Batch 359 / Plan 370 / Research 403.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// The R1 bench spec — the partial-read posture (the promotion posture):
/// K named fields, the consumer reads ONE middle field. The buggy arm
/// decodes every field (the whole-payload contract); the fixed arm decodes
/// one (the named-outputs contract). The anti-regime read-all posture is
/// re-measured by `batch359_goat_named_outputs` on demand.
#[cfg(feature = "rust_perf_bench")]
fn named_outputs_lazy_handles_partial_decode_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![8, 64, 256],
        harness: r#"
pub struct Input {
    fields: usize,
    words_per_field: usize,
    slot: usize,
}

pub fn make_input(n: usize) -> Input {
    // n named fields of n words each; read ONE middle field.
    Input {
        fields: n,
        words_per_field: n,
        slot: n / 2,
    }
}

fn field_word(field: usize, i: usize) -> u64 {
    let mut x = ((field as u64) << 32) ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
}

fn encode_field(field: usize, words: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words * 8);
    for i in 0..words {
        bytes.extend_from_slice(&field_word(field, i).to_le_bytes());
    }
    bytes
}

fn decode_field(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn checksum(values: &[u64]) -> u64 {
    values.iter().fold(0u64, |acc, v| acc.rotate_left(7) ^ *v)
}

// BUGGY: the whole-payload contract — decode EVERY field to read one.
pub fn run_buggy(input: &Input) {
    let mut answer = 0u64;
    for field in 0..input.fields {
        let bytes = encode_field(field, input.words_per_field);
        let decoded = decode_field(&bytes);
        if field == input.slot {
            answer = checksum(&decoded);
        }
    }
    std::hint::black_box(answer);
}

// FIXED: named outputs + a handle — decode ONE field on demand.
pub fn run_fixed(input: &Input) {
    let bytes = encode_field(input.slot, input.words_per_field);
    let decoded = decode_field(&bytes);
    std::hint::black_box(checksum(&decoded));
}

pub fn expected(input: &Input) -> u64 {
    checksum(&decode_field(&encode_field(input.slot, input.words_per_field)))
}
"#
        .to_string(),
    }
}
