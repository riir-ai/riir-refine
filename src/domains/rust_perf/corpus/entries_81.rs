//! Entries slice — Batch 365 (2026-11-16): the SECOND TRANCH of the
//! hydra-db/hydradb object-store-graph-db distill (Plan 379 T3 / Research
//! 411 — candidates 9/10/11/3/5 + #6 of the ranked table; the first
//! tranche landed as Batch 364 in `entries_80`, the follow-up deferral
//! recorded there).
//!
//! Source: hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6
//! (tag v0.4.0, 2026-11-06), `ELv2` — transcription-only throughout: every
//! fixture is an ORIGINAL plain-Rust reproduction, no source pasted; the
//! license note rides every entry's provenance row. T0: the clone
//! re-verified at the pin THIS session and every quote below re-read
//! byte-exact (the tranche's quotes carried Batch 364's T0 verification;
//! the re-read stands per the mine-time law).
//!
//! cite-correction (recorded in Research 411): the note's table row 5
//! (`catalog-memo-scoped-to-the-snapshot-no-invalidation`, cited
//! `graph-slate/write.rs:80–87` with a bloom-negative clause) resolves at
//! the pin to the PENDING WRITE-BACK SWEEP REGISTRY — `PendingWriteBackSweep`
//! take/Drop. The lifetime-scoped no-invalidation law IS verified there;
//! the bloom clause is NOT verifiable at the cite and is dropped from the
//! entry; the rule lands under the mechanism's own name
//! (`pending-claim-registry-take-consumes-drop-clears`).
//!
//! Landing postures: `existential-witness-early-exit-scan` is the
//! tranche's one BENCHABLE rule (the GOAT cell: probe-count integer-exact,
//! Pareto validator, two-run stable — record
//! `.benchmarks/155_batch365_goat_cells.md`, numbers landed AT MEASUREMENT,
//! never pre-typed); `wal-watermark-pinned-before-snapshot`,
//! `tail-walk-cost-cap-declines-to-rebuild` and
//! `frozen-artifact-plus-delta-tail-overlay` land NotBenchable-by-domain
//! (correctness-ordering / economics-decline / composition laws — no
//! validator host); `pending-claim-registry-take-consumes-drop-clears`
//! lands NotBenchable-by-model (a synthetic sweep constant would measure
//! the model, never an engine's invalidation path — the
//! `derived-index-write-touches-only-changed-keys` class);
//! `one-shared-disk-cache-not-even-split` lands NotBenchable-by-domain
//! (the win is a cache-QUALITY axis — served fetches / hit rate under a
//! skewed working set — the validator's ns/alloc axes cannot express it;
//! the trace-sim QUALITY ordering is fixture-pinned).
//!
//! Fixtures are ORIGINAL plain-Rust stand-ins, standalone rustc-compiled
//! AND executed before landing (`b365_fixtures`; the
//! B353/B359/B360/B361/B362/B363/B364 pre-entry discipline).
//!
//! Batch 365 / Plan 379 / Research 411.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── io: the consuming predicate's ARITY dispatches the scan shape ──
        PerfRule {
            rule_id: "existential-witness-early-exit-scan".to_string(),
            name: "A scan over a candidate's stored prefixes answers two different consuming predicates with ONE shape → predicate ARITY dispatches the scan shape: an EXISTENTIAL check (does at least one edge survive?) runs the prefixes in order and STOPS at the first witness — one sequential search per candidate, overlapped across candidates — while a UNIVERSAL enumeration (the consumer needs every survivor) keeps the full per-prefix reads; never an arity-blind scan that reads every prefix to answer may-one-exist, and never a speculative early exit on a predicate that needs the last prefix".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: one scan shape for every consumer. The detach path needs
// EVERY surviving edge (it enumerates them); the isolation path only
// needs to know whether ONE survives — but it pays the full
// enumeration too: at K prefixes the existential mass reads K times
// the work its predicate can use.

pub struct Shard {
    /// per-vertex prefix lists (already materialized store reads)
    pub prefixes: Vec<Vec<u32>>,
}

impl Shard {
    /// The isolation consumer only asks: does at least one edge survive?
    /// The shape below answers correctly — after reading EVERY prefix.
    pub fn has_surviving(&self, vertex: usize, alive: &[bool]) -> bool {
        let mut any = false;
        for &p in &self.prefixes[vertex] {
            if alive[p as usize] {
                any = true; // keeps reading the remaining prefixes anyway
            }
        }
        any
    }

    /// The detach consumer needs the full survivor list.
    pub fn survivors(&self, vertex: usize, alive: &[bool], out: &mut Vec<u32>) {
        out.clear();
        for &p in &self.prefixes[vertex] {
            if alive[p as usize] {
                out.push(p);
            }
        }
    }
}

#[test]
fn arity_blind_scan_answers() {
    let shard = Shard { prefixes: vec![vec![0, 1, 2]] };
    assert!(shard.has_surviving(0, &[false, true, false]));
    let mut out = Vec::new();
    shard.survivors(0, &[false, true, false], &mut out);
    assert_eq!(out, vec![1]);
}"#.to_string(),
            fixed_code: r#"// OPT: the predicate's ARITY dispatches the scan shape. The
// existential form runs the prefixes IN ORDER and stops at the first
// witness; the universal form keeps the full enumeration. The source
// carries the law as a comment on the job construction: "Do not
// speculate past a candidate's first surviving edge. Detach needs
// every prefix; isolation needs one sequential search per candidate,
// overlapped with other candidates."

pub struct Shard {
    pub prefixes: Vec<Vec<u32>>,
}

impl Shard {
    /// EXISTENTIAL arity: sequential search, first witness wins.
    pub fn has_surviving(&self, vertex: usize, alive: &[bool]) -> bool {
        self.prefixes[vertex]
            .iter()
            .any(|&p| alive[p as usize]) // stops at the first witness
    }

    /// UNIVERSAL arity: the consumer needs every survivor — full reads.
    pub fn survivors(&self, vertex: usize, alive: &[bool], out: &mut Vec<u32>) {
        out.clear();
        for &p in &self.prefixes[vertex] {
            if alive[p as usize] {
                out.push(p);
            }
        }
    }
}

#[test]
fn arity_dispatch_answers_identically() {
    let shard = Shard { prefixes: vec![vec![0, 1, 2]] };
    assert!(shard.has_surviving(0, &[false, true, false]));
    assert!(!shard.has_surviving(0, &[false, false, false]));
    let mut out = Vec::new();
    shard.survivors(0, &[false, true, false], &mut out);
    assert_eq!(out, vec![1]);
}"#.to_string(),
            context_notes: Some("Grade B — BENCHABLE, the tranche's GOAT lead — MEASURED ParetoDominant 3/0/0 TWO-RUN STABLE (the pre-registered bar: probe-count integer-exact through a touch table, Pareto validator, wins >= 2/3 sizes, never dominated, alloc axis bit-identical across runs; the cell: 5.98x/5.70x/3.78x at [1 024, 16 384, 262 144] candidates, allocs 0->0 at every size, run 2: 6.17x/5.85x/3.76x — verdict class stable, `.benchmarks/155_batch365_goat_cells.md`). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — TRANSCRIPTION-ONLY per the license law: original fixtures, never pasted source; T0 byte-verified this session). THE LAW IN-SOURCE (src/shard/write.rs:3157-3166, verbatim): 'let witness_only = isolation_detach_set.is_some_and(|detach| !detach.contains(vertex));' — the jobs iterator builds ONE sequential-scan job for a witness_only candidate ('Do not speculate past a candidate's first surviving edge. Detach needs every prefix; isolation needs one sequential search per candidate, overlapped with other candidates.') and per-prefix enumeration jobs otherwise. The two populations are REAL in-source: detach enumeration and isolation witness checks over the same stored prefixes. REGIME: any scan whose consuming predicate's arity is known at the call site — existence checks (has-key, has-survivor, contains), first-match lookups, uniqueness probes, admission gates over stored lists; batch fan-outs where the existential mass dominates. ANTI-REGIMES: the consumer genuinely needs the full set (UNIVERSAL arity — early exit would be WRONG; the detach population here); witnesses that sit at the tail with high probability (the sequential form degenerates toward the full read — tie); an index/point probe already exists (it beats both shapes). ADJUDICATION (dedup re-verified at authoring HEAD): filter-count-to-any (entries_03) owns the ITERATOR-SHAPE fix (count()>0 -> any() short-circuit on one slice) — this rule owns the PREDICATE-ARITY DISPATCH over stored prefix lists (the consumer chooses the scan shape, not the iterator combinator) — kin, compose; finder-hoist-reuse (entries_31) owns finder-object reuse across calls — different axis; entries_58/39 'witness' hits are the Drop-witness and second-witness-teardown senses — different; grep 'witness_only|predicate arity|first witness' over rust_perf = zero prior owners for this axis. Bench harness: per-candidate prefix streams (K in 4..64), 85% existential / 15% universal, witnesses ~60% in the first third of K, reads counted through a 1M-entry touch table; G1 census parity asserted in make_input; alloc-free both arms. Plan 379 / Research 411 (the hydradb object-store-graph-db distill, second tranche).".to_string()),
            // The tranche's GOAT lead: Pareto validator over the prefix
            // streams (the cell record: .benchmarks/155).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(existential_witness_bench_spec()),
        },
        // ── io: the durable WAL id is captured BEFORE the snapshot pins ────
        PerfRule {
            rule_id: "wal-watermark-pinned-before-snapshot".to_string(),
            name: "An incremental reader composes its base from a snapshot of a log-tailed store → capture the durable WAL id BEFORE pinning the snapshot, never after: a base published with an unset/zero WAL position (a writer snapshot carries none) forces EVERY later tail walk to read the entire WAL history — worst case the whole history, which GC may have partially collected — while a base pinned at the pre-snapshot durable id keeps every tail bounded to the recent files the sequence guard then filters".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: pin the snapshot first, read the durable WAL id after — and
// publish the base from the snapshot alone. A writer snapshot carries
// no WAL position, so the base's last_wal_id lands at 0: every later
// tail walk starts from the beginning of the WAL history, reading
// files the GC may have already collected.

pub struct Base {
    pub base_sequence: u64,
    pub last_wal_id: u64,
}

pub struct Store {
    /// file id -> entry count, oldest first; GC drops the head
    pub wal_files: Vec<(u64, usize)>,
    pub durable_wal_id: u64,
}

impl Store {
    pub fn snapshot_seq(&self) -> u64 {
        42
    }

    pub fn build_base(&self) -> Base {
        let base_sequence = self.snapshot_seq(); // pinned FIRST
        let _ = self.durable_wal_id; // read AFTER the pin — never used
        Base { base_sequence, last_wal_id: 0 } // the snapshot carries none
    }

    /// The tail walk every later reader pays. From file 0 when the
    /// base carries no WAL position — including files GC already
    /// dropped, which is the correctness hazard, not just the cost.
    pub fn tail_walk(&self, base: &Base) -> Result<usize, String> {
        if base.last_wal_id == 0 {
            // unbounded: the whole history, whatever GC left of it
            self.wal_files
                .iter()
                .map(|(id, _)| {
                    if self.gc_dropped(*id) {
                        Err(format!("file {id} collected — the tail cannot walk it"))
                    } else {
                        Ok(1)
                    }
                })
                .sum::<Result<usize, String>>()
        } else {
            Ok(self
                .wal_files
                .iter()
                .filter(|&&(id, _)| id > base.last_wal_id && !self.gc_dropped(id))
                .count())
        }
    }

    fn gc_dropped(&self, id: u64) -> bool {
        id < self.wal_files.first().map(|&(id, _)| id).unwrap_or(0)
    }
}

#[test]
fn zero_watermark_walks_everything() {
    let store = Store {
        wal_files: (0..8).map(|i| (i, 10)).collect(),
        durable_wal_id: 6,
    };
    let base = store.build_base();
    assert_eq!(base.last_wal_id, 0);
    // GC has collected files 0..2 — the unbounded walk HITS THEM
    assert!(store.tail_walk(&base).is_err());
}"#.to_string(),
            fixed_code: r#"// OPT: capture the durable WAL id BEFORE pinning the snapshot, and
// let the snapshot's own position (when it carries one) win. The
// source's invariant: "every entry durable in files at or below it
// commits before the snapshot pins its sequence, so the tail
// invariant (no entry with seq > base_sequence in files <= last_wal_id)
// holds; at worst the tail re-reads a few recent files whose entries
// the sequence guard skips."

pub struct Base {
    pub base_sequence: u64,
    pub last_wal_id: u64,
}

pub struct Store {
    pub wal_files: Vec<(u64, usize)>,
    pub durable_wal_id: u64,
}

impl Store {
    pub fn snapshot_seq(&self) -> u64 {
        42
    }

    pub fn build_base(&self) -> Base {
        // THE ORDER IS THE LAW: the watermark is captured BEFORE the
        // snapshot pins its sequence.
        let durable_wal_id = self.durable_wal_id;
        let base_sequence = self.snapshot_seq();
        let last_wal_id = durable_wal_id; // (a snapshot-carried position wins)
        Base { base_sequence, last_wal_id }
    }

    pub fn tail_walk(&self, base: &Base) -> Result<usize, String> {
        // bounded: only files above the watermark, only what GC kept
        Ok(self
            .wal_files
            .iter()
            .filter(|&&(id, _)| id > base.last_wal_id && !self.gc_dropped(id))
            .count())
    }

    fn gc_dropped(&self, id: u64) -> bool {
        id < self.wal_files.first().map(|&(id, _)| id).unwrap_or(0)
    }
}

#[test]
fn pinned_watermark_walk_is_bounded() {
    let store = Store {
        wal_files: (0..8).map(|i| (i, 10)).collect(),
        durable_wal_id: 6,
    };
    let base = store.build_base();
    assert_eq!(base.last_wal_id, 6);
    // files 0..2 are GC-collected — the bounded walk never touches them
    assert_eq!(store.tail_walk(&base).unwrap(), 1); // only file 7
}"#.to_string(),
            context_notes: Some("Grade B+ (the note's ndb lead) — io, NotBenchable-by-domain (a correctness ORDERING law: the failure is unbounded or GC-diverged reads, not this process's wall-clock — the dial9/B352 annotation class). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified this session). THE LAW IN-SOURCE (src/engine/index_store.rs:114-125, verbatim): the comment 'A writer snapshot carries no WAL position (`state.rs`), and a generation published with `last_wal_id = 0` forces every later tail to walk the entire WAL history — which GC may have partially collected. Capture the durable WAL id *before* pinning the snapshot: every entry durable in files at or below it commits before the snapshot pins its sequence, so the tail invariant (no entry with seq > base_sequence in files <= last_wal_id) holds; at worst the tail re-reads a few recent files whose entries the sequence guard skips.' — and the code order: 'let durable_wal_id = self.storage.last_durable_wal_id().await?;' BEFORE 'let snapshot = self.storage.snapshot().await?;', composed as 'let last_wal_id = snapshot.last_wal_id().unwrap_or(durable_wal_id);' (a snapshot-carried position wins; the durable id is the fallback). REGIME: any store whose incremental readers compose a frozen base with a log tail — WAL-tailed indexes, changelog-fed materialized views, freeze/thaw over a write-ahead log, catch-up cursors, compaction bookkeeping. ANTI-REGIME: stores whose snapshot carries its own log position (the unwrap-or fallback IS the repair shape — no bug); tail-less fully-rebuilt readers; append-only logs with no GC (unbounded-but-safe — the cost half still argues for the watermark). ADJUDICATION (dedup re-verified at authoring HEAD): grep 'watermark|tail walk|durable wal' over rust_perf = zero owners; mmap-settled-size-gated-read (entries_61 family) owns a SIZE-gated mmap read — different resource, different axis; the lane-intel mapping (ndb WalBackend sealed-segment id before the freeze snapshot + freeze/thaw lineage) is recorded in Research 411 §Lane-intel as the reference design — no open issue forces it. Plan 379 / Research 411.".to_string()),
            // NotBenchable-by-domain: correctness ordering, no validator host.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── io: the tail-walk cost cap whose refusal is DECLINE-to-rebuild ──
        PerfRule {
            rule_id: "tail-walk-cost-cap-declines-to-rebuild".to_string(),
            name: "An incremental path whose input cost grows on a DIFFERENT axis than its output — a tail walk's cost grows with write ACTIVITY (one file per flush) while its output is a delta of a few records — gets a hard input cap whose refusal action is DECLINE to the full rebuild (rebuild + the served base both cheaper than the walk the cap refused), never a partial answer; and the regime is two-sided BY CONSTRUCTION: a delta feed that is already O(its output) carries NO cap — gating it would refuse work that is cheap by construction".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: the incremental index build walks the log tail with no
// cap. Every file is one round trip, and the span between an old
// index generation and the durable head grows with write activity
// (one file per flush), not with data size — an uncapped walk can
// cost minutes to derive a delta of a few edges.

pub struct IndexBuilder {
    pub files_since_generation: usize,
}

pub struct Delta {
    pub edges: usize,
}

#[derive(Debug, PartialEq)]
pub enum BuildError {
    Io,
}

impl IndexBuilder {
    fn read_file(&self, _f: usize) -> Result<Delta, BuildError> {
        Ok(Delta { edges: 3 })
    }

    pub fn incremental_delta(&self) -> Result<Delta, BuildError> {
        // walks EVERY file between the generation and the head —
        // however much write activity happened in between
        let mut delta = Delta { edges: 0 };
        for f in 0..self.files_since_generation {
            let d = self.read_file(f)?;
            delta.edges += d.edges;
        }
        Ok(delta)
    }
}

#[test]
fn uncapped_walk_reads_the_whole_span() {
    let builder = IndexBuilder { files_since_generation: 5_000 };
    let delta = builder.incremental_delta().unwrap();
    // 5 000 round trips to learn that 3 edges per file changed
    assert_eq!(delta.edges, 15_000);
}"#.to_string(),
            fixed_code: r#"// OPT: the cost cap whose refusal is a DECLINE to the full rebuild —
// both cheaper than the walk the cap refused. The regime is
// two-sided, and the source says so: the xlog delta path (the other
// incremental feed) deliberately has no gate — "There is no
// cost-based decline: the xlog scan is O(delta) by construction."
// The axis decides: cap a walk whose input grows with ACTIVITY;
// never cap a feed that is already O(its output).

pub const MAX_TAIL_FILES: usize = 64;

pub struct IndexBuilder {
    pub files_since_generation: usize,
}

pub struct Delta {
    pub edges: usize,
}

#[derive(PartialEq)]
pub enum TailOutcome {
    Delta(Delta),
    /// the caller declines: the full rebuild + the served base are
    /// both cheaper than the walk the cap refused
    Unavailable,
}

impl IndexBuilder {
    fn read_file(&self, _f: usize) -> Delta {
        Delta { edges: 3 }
    }

    pub fn tail_delta(&self, files_since: usize) -> TailOutcome {
        if files_since > MAX_TAIL_FILES {
            return TailOutcome::Unavailable;
        }
        let mut delta = Delta { edges: 0 };
        for f in 0..files_since {
            let d = self.read_file(f);
            delta.edges += d.edges;
        }
        TailOutcome::Delta(delta)
    }

    /// The delta feed: O(delta) by construction — NO cap, by design.
    pub fn xlog_delta(&self, delta_len: usize) -> Delta {
        let mut delta = Delta { edges: 0 };
        for _ in 0..delta_len {
            delta.edges += 3;
        }
        delta
    }
}

#[test]
fn capped_walk_declines_rebuild_serves() {
    let builder = IndexBuilder { files_since_generation: 5_000 };
    // the cap refuses the walk; the caller rebuilds
    assert_eq!(builder.tail_delta(5_000), TailOutcome::Unavailable);
    // inside the cap the delta serves
    assert_eq!(builder.tail_delta(10), TailOutcome::Delta(Delta { edges: 30 }));
    // the O(delta) feed is never gated
    assert_eq!(builder.xlog_delta(1_000_000).edges, 3_000_000);
}"#.to_string(),
            context_notes: Some("Grade B — io, NotBenchable-by-domain (the decision is an ECONOMICS refusal — decline-vs-walk; the cost lives in the infrastructure's round trips, not this process's wall-clock — the annotation class). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified this session). THE LAWS IN-SOURCE, BOTH SIDES, VERBATIM. Side one — the cap (crates/graph-slate/src/core/config.rs:113-121, the max_wal_tail_files field doc): 'Cost cap on the WAL tail walk: the most WAL files a single `topology_tail_since` may read. Every file is one object-store round trip, and the span between an old index generation and the durable head grows with write *activity* (one file per WAL flush), not with graph size — so an uncapped walk can cost minutes to derive a delta of a few edges. Past the cap the tail declines (`Unavailable`), which sends the incremental index build to the full rebuild and the read path to snapshot adjacency: both cheaper than the walk the cap refused. A type whose change stamp has not moved past its generation is never walked, so the cap binds only while the indexer is behind on that type's own edge changes.' Side two — the deliberate no-gate (src/engine/index_store.rs:219-226, the xlog incremental doc): 'Returns `Ok(None)` only for one-time bootstrap conditions (no previous generation, generation payload missing, xlog coverage not yet established); the caller runs the full rebuild, which itself establishes coverage for every cycle after. There is no cost-based decline: the xlog scan is O(delta) by construction.' THE AXIS: input-cost axis (activity) differs from output axis (delta) => cap + decline; input-cost axis == output axis (O(delta)) => no gate. REGIME: incremental feeds whose input cost grows on a different axis than their output — log-tail walks, catch-up scans, backlog drains, queue-replay windows, sync spans between a materialized view and its feed. ANTI-REGIMES: O(delta)-by-construction feeds (gating them refuses cheap work — the counter-example, IN-SOURCE); caps whose refusal PARTIALLY applies (a partial answer is worse than a decline — the law lands the typed Unavailable, never a truncated delta); caps with no decline action (a cap that retries smaller is a different law — this one refuses to the cheaper full path). ADJUDICATION (dedup re-verified at authoring HEAD): generation-counter-stale-timer-invalidation (entries_56) owns staleness BY TIMER — different axis; batch-retry-ladder-classifies-failure-before-permanent-state (entries_64) owns TRANSPORT failure classification — this owns input-cost economics; grep 'decline|cost cap|max.*tail|walk.*cap' over rust_perf = zero owners. Plan 379 / Research 411.".to_string()),
            // NotBenchable-by-domain: decline economics, no validator host.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: frozen artifact at its base epoch + the tail overlay ──
        PerfRule {
            rule_id: "frozen-artifact-plus-delta-tail-overlay".to_string(),
            name: "Freshness over an immutable compiled artifact handled by dropping and rebuilding the whole body, or by caching the producer's movable pointer alongside it → cache the artifact AT ITS BASE EPOCH and compose freshness as a bounded tail overlay ON TOP (base + overlay served together), and re-read the MOVABLE pointer on EVERY call — a cached older pointer keeps answering after the producer moved on, while an older base still serves when the newest generation is past the read epoch".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: the compiled body and the producer's pointer are cached
// TOGETHER, and freshness is handled by dropping and recompiling the
// whole immutable body whenever the pointer moves. Two defects, one
// shape: interleaved readers at different generations pay a WHOLE-BODY
// rebuild per flip, and a reader whose read epoch is BEHIND the newest
// generation is answered with that newest generation anyway — a
// generation past its read epoch.

#[derive(Clone, PartialEq)]
pub struct Matrix(pub [u64; 4]);

pub struct Cached {
    pub compiled: Matrix,
    pub pointer: u64, // cached WITH the artifact — the pointer is never re-read
}

pub struct QueryCache {
    cached: Option<Cached>,
    rebuilds: usize,
}

impl QueryCache {
    fn build(&self, pointer: u64) -> Cached {
        Cached { compiled: Matrix([pointer, 1, 2, 3]), pointer }
    }

    pub fn get(&mut self, published: u64, _read_epoch: u64) -> Matrix {
        match &self.cached {
            None => {
                self.rebuilds += 1;
                self.cached = Some(self.build(published));
            }
            Some(c) if c.pointer != published => {
                // freshness = drop and recompile the WHOLE body
                self.rebuilds += 1;
                self.cached = Some(self.build(published));
            }
            Some(_) => {}
        }
        self.cached.as_ref().unwrap().compiled.clone()
    }
}

#[test]
fn wholesale_rebuilds_per_flip_answers_future_generations() {
    let mut cache = QueryCache { cached: None, rebuilds: 0 };
    let _ = cache.get(10, 10);
    let _ = cache.get(20, 25);
    // interleaved old/new readers: EVERY flip rebuilds the whole body
    for _ in 0..4 {
        let _ = cache.get(10, 25);
        let _ = cache.get(20, 25);
    }
    assert_eq!(cache.rebuilds, 10); // 2 + 8 flips
    // and a reader whose read epoch is 12 is answered with generation
    // 20 — a generation past its read epoch (the freshness violation:
    // "a cached older generation would keep answering after the indexer
    // moved on" is the mirror defect — this shape answers the FUTURE one)
    let m = cache.get(20, 12);
    assert_eq!(m.0[0], 20);
}"#.to_string(),
            fixed_code: r#"// OPT: the artifact is immutable — cache it AT ITS BASE EPOCH (one
// compile per generation, cached across readers) and compose freshness
// as a bounded tail overlay ON TOP (generation..read_epoch). The
// published pointer is READ ON EVERY CALL — never cached — and an
// older generation still serves when the newest is past the read
// epoch: serve the largest compiled generation at or before
// min(published, read_epoch), overlay the gap to the read epoch.

#[derive(Clone, PartialEq)]
pub struct Matrix(pub [u64; 4]);

pub struct QueryCache {
    /// per-generation compile cache — the immutable bodies, keyed at
    /// their base epoch
    generations: std::collections::HashMap<u64, Matrix>,
    compiles: usize,
}

impl QueryCache {
    fn compile(&mut self, epoch: u64) {
        self.compiles += 1;
        self.generations.insert(epoch, Matrix([epoch, 1, 2, 3]));
    }

    /// `published` arrives from a FRESH read of the producer's pointer;
    /// base + overlay served together.
    pub fn get(&mut self, published: u64, read_epoch: u64) -> (Matrix, u64, usize) {
        let serve = if published <= read_epoch {
            // the newest usable generation IS the fresh pointer's: it
            // serves, compiled once and cached across readers
            if !self.generations.contains_key(&published) {
                self.compile(published);
            }
            published
        } else {
            // the newest published is PAST the read epoch: the largest
            // compiled generation at or before the read epoch serves —
            // "an older generation is still used when the newest one is
            // past the read epoch"
            *self
                .generations
                .keys()
                .filter(|&&g| g <= read_epoch)
                .max()
                .expect("a compiled base at or below the read epoch exists")
        };
        // the bounded tail overlay composes the gap the base cannot see
        let overlay = (read_epoch - serve) as usize;
        (self.generations[&serve].clone(), serve, overlay)
    }
}

#[test]
fn base_serves_overlay_composes() {
    let mut cache = QueryCache { generations: Default::default(), compiles: 0 };
    let (_, b, o) = cache.get(10, 10);
    assert_eq!((b, o), (10, 0));
    // the producer publishes 20 once; readers at read epochs 21..25:
    // the base at 20 serves, the overlay composes the gap per reader
    for re in 21..=25u64 {
        let (_, b, o) = cache.get(20, re);
        assert_eq!((b, o), (20, (re - 20) as usize));
    }
    assert_eq!(cache.compiles, 2); // ONE compile per generation, cached across readers
    // an older generation still serves when the newest is past the read epoch
    let (_, b, o) = cache.get(30, 12);
    assert_eq!((b, o), (10, 2)); // base at 10 answers; the tail composes 10..12
    assert_eq!(cache.compiles, 2);
    // interleaved old/new readers: no rebuild per flip
    for _ in 0..4 {
        let _ = cache.get(10, 25);
        let _ = cache.get(20, 25);
    }
    assert_eq!(cache.compiles, 2);
}"#.to_string(),
            context_notes: Some("Grade B — caching, NotBenchable-by-domain (a composition/ordering law: the claim is WHICH generation answers a read and WHAT freshness costs, no validator host). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified this session). THE LAWS IN-SOURCE (src/shard/query.rs, T0-verified): the tail composition — the span names 'The gap between the compiled generation and the read epoch, replayed from the WAL' and the success arm returns them TOGETHER: 'Ok(GraphTopologyTail::Complete(overlay)) => { return Ok(Some((compiled, Some(overlay), false))); }'; and the pointer law (published_generation_matrix doc, verbatim): 'The published pointer is read on every call, not the cached artifact: a cached older generation would keep answering after the indexer moved on. An older generation is still used when the newest one is past the read epoch.' REGIME: immutable compiled artifacts behind a moving production pointer — compiled query plans, frozen embeddings + a delta log, codegen caches behind re-exports, frozen weight snapshots with a deterministic overlay (the freeze/thaw + reader-LoRA shape). ANTI-REGIMES: content-addressed pointers (the key IS the content — re-reading is a no-op); mutable artifacts (the invalidation laws own them, not this); UNBOUNDED tails (the overlay must be bounded — tail-walk-cost-cap-declines-to-rebuild, this batch, is the companion law). ADJUDICATION (dedup re-verified at authoring HEAD): generation-counter-stale-timer-invalidation (entries_56) owns TIMER staleness — different consumer; driver-pipeline-cache-sealed-adapter-keyed (entries_78) owns sealed-artifact INTEGRITY at load — compose; grep 'base epoch|tail overlay|frozen artifact' over rust_perf = zero owners. Lane intel (Research 411 §Lane-intel): the ndb freeze/thaw + R2Wal overlay is the mapped analog — reference design, no open issue. Plan 379 / Research 411.".to_string()),
            // NotBenchable-by-domain: freshness composition, no validator host.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: the pending-claim registry take-consumes/drop-clears
        PerfRule {
            rule_id: "pending-claim-registry-take-consumes-drop-clears".to_string(),
            name: "A registry of pending work claims (producer spawns, consumer takes) grows invalidation machinery — TTLs, sweep intervals, GC passes that cannot tell a live claim from a dead one — while dead claims linger and producers raise dead targets → bind each entry's lifetime to ITS operation: the consumer TAKEs the entry (consume-on-read) and a claim dropped before its consumer ran REMOVES ITSELF in Drop — the next producer spawns fresh instead of raising a dead claim's target; no TTL, no sweep, no stale-claim GC".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// NAIVE: claims are inserted at spawn and only cleared by a
// time-based sweep. A producer that dies before its consumer runs
// leaves the entry behind: the next producer finds the stale claim
// and RAISES ITS TARGET — work aimed at a task that will never run —
// and the sweep itself cannot tell liveness from age.

pub struct Registry {
    pending: std::collections::HashMap<(String, String), u64>,
    ttl: std::time::Duration,
    born: std::time::Instant,
}

impl Registry {
    fn new() -> Self {
        Self {
            pending: Default::default(),
            ttl: std::time::Duration::from_secs(30),
            born: std::time::Instant::now(),
        }
    }

    pub fn claim(&mut self, key: (String, String), target: u64) {
        self.maybe_gc();
        self.pending.insert(key, target);
    }

    /// producers RAISE the target of whatever claim is present —
    /// including one whose owner already died
    pub fn raise(&mut self, key: &(String, String), target: u64) {
        self.pending
            .entry(key.clone())
            .and_modify(|t| *t = (*t).max(target))
            .or_insert(target);
    }

    fn maybe_gc(&mut self) {
        if self.born.elapsed() >= self.ttl {
            // ages everything out by TIME, not by liveness — a live
            // long claim is dropped too, a dead young claim survives
            self.pending.clear();
            self.born = std::time::Instant::now();
        }
    }
}

#[test]
fn stale_claim_survives_and_raises_a_dead_target() {
    let mut reg = Registry::new();
    reg.claim(("cell".into(), "edge".into()), 10);
    // the claim's owner died without consuming — no RAII binding, so
    // the entry lingers; the next producer raises the DEAD claim
    reg.raise(&("cell".into(), "edge".into()), 99);
    let raised = reg.pending.get(&("cell".to_string(), "edge".to_string()));
    assert_eq!(raised, Some(&99)); // aimed at a task that will never run
}"#.to_string(),
            fixed_code: r#"// OPT: the entry's lifetime IS the operation's lifetime. The
// consumer TAKEs the claim (consume-on-read); a claim dropped before
// its consumer ran REMOVES ITSELF — the source: "Dropped untaken, by
// a sweep that failed or was aborted before it took its target, it
// clears the entry, so the property's next write-back spawns a sweep
// rather than raising the target of one that will never run." No
// TTL, no sweep, no stale-claim GC.

pub struct Registry {
    pending: std::collections::HashMap<(String, String), u64>,
}

/// RAII claim: `take` consumes it; dropping it untaken removes it.
pub struct Claim<'a> {
    registry: &'a mut Registry,
    key: Option<(String, String)>,
}

impl Registry {
    pub fn claim(&mut self, key: (String, String), target: u64) -> Claim<'_> {
        self.pending.insert(key.clone(), target);
        Claim { registry: self, key: Some(key) }
    }

    /// producers raise ONLY a live claim's target — the map holds
    /// live claims by construction
    pub fn raise_live(&mut self, key: &(String, String), target: u64) -> bool {
        self.pending
            .entry(key.clone())
            .and_modify(|t| *t = (*t).max(target))
            .is_some()
    }
}

impl Claim<'_> {
    /// the consumer's take: consume-on-read, the entry is gone
    pub fn take(mut self) -> Option<u64> {
        let key = self.key.take()?;
        self.registry.pending.remove(&key)
    }
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            self.registry.pending.remove(&key);
        }
    }
}

#[test]
fn take_consumes_and_drop_clears() {
    let mut reg = Registry::default();
    let target;
    {
        let claim = reg.claim(("cell".into(), "edge".into()), 10);
        target = claim.take();
    } // already taken: Drop removes nothing
    assert_eq!(target, Some(10));
    // an ABORTED claim (dropped untaken) clears itself
    {
        let _claim = reg.claim(("cell".into(), "edge".into()), 10);
    } // dropped untaken — Drop fires
    // nothing live to raise: the dead claim is gone by construction
    assert!(!reg.raise_live(&("cell".to_string(), "edge".to_string()), 99));
}"#.to_string(),
            context_notes: Some("Grade B — concurrency, NotBenchable-by-model (a synthetic invalidation cost would measure OUR sweep constant, never an engine's — the derived-index-write-touches-only-changed-keys class). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified this session). CITE-CORRECTION (recorded in Research 411): the note's table row 5 staged the name `catalog-memo-scoped-to-the-snapshot-no-invalidation` citing graph-slate/write.rs:80–87 with a bloom-negative clause; the VERIFIED site at that cite (src/shard/write.rs:74-103 at the pin) is the PENDING WRITE-BACK SWEEP REGISTRY — the struct doc (verbatim): 'A detached sweep's entry in the shard's pending sweep targets, from the spawn until the sweep takes its target. Dropped untaken, by a sweep that failed or was aborted before it took its target, it clears the entry, so the property's next write-back spawns a sweep rather than raising the target of one that will never run.' + PendingWriteBackSweep::take (verbatim): 'The newest generation published for the property while this sweep was pending. From here on the property has no pending sweep, and its next write-back spawns one.' + the Drop impl removing the untaken key. The lifetime-scoped no-invalidation law IS verified there; the bloom clause is NOT verifiable at the cite and is DROPPED from this entry; the rule lands under the mechanism's own name. REGIME: pending-work registries — claim tables, in-flight dedupe maps, lease/coalescing registries, spawn-until-consume coordination of any kind. ANTI-REGIMES: consumers that POLL and cannot own an RAII handle (a liveness-scoped TTL keyed to the owner's heartbeat is the nearest honest shape); entries that MUST outlive their producer (a resume-after-crash journal — durability law, not liveness); claims where raising a dead target is harmless (idempotent work). ADJUDICATION (dedup re-verified at authoring HEAD): batch-local-memoization-negative-cache-tombstone (the note's named adjacency) owns the negative-TOMBSTONE MECHANISM for memo caches — this owns the CLAIM-REGISTRY LIFETIME law (consume-or-self-remove) — DISTINCT; derived-cache-throwaway-version-reset-recovery (entries_75) owns reset-on-corruption recovery — different axis; persistent-cache-teardown-needs-second-witness (entries_39) owns destructive teardown evidence — different; grep 'pending claim|consume.on.read|raise.*target' over rust_perf = zero owners. FACE (count-neutral, Batch 368 / Plan 382 / Research 414, kunobi-ninja/kache @ da717cb9 Apache-2.0): the kache daemon's download-claim carries the SAME RAII lifetime law at flight scale — the DownloadingGuard's Drop releases the key on EVERY exit path 'or a panic deep in the download/extract/import stack (zstd/tar/blake3/sqlite). Without this, a panic between the claim and the trailing remove would leave the key stuck, and every later remote-check for it would block the full [DOWNLOAD_JOIN_BUDGET] until the daemon restarts' (src/daemon.rs:7744-7748) — a claim-registry entry that does not clear itself converts one failed unit into a machine-wide stall; compose (the registry owns the lifetime law, the flight adds the leader/follower election). Plan 379 / Research 411.".to_string()),
            // NotBenchable-by-model: a synthetic sweep constant measures the
            // model, not an engine's invalidation path.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: one shared pool, one eviction order — never an even split
        PerfRule {
            rule_id: "one-shared-disk-cache-not-even-split".to_string(),
            name: "A node's cache budget split EVENLY across its max-concurrency slot count starves the skewed tenant while the budget idles (48 GiB over 64 scopes left each 768 MiB, however little the others held — the big scope refetched the same parts from storage on every read and evicted hundreds of MB a second while most of the node's budget sat empty) → ONE cache over the whole budget with ONE eviction order, handed to every scope as their store; tenancy stays prefix-keyed so scopes cannot read each other's entries".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// NAIVE: every scope builds its own cache; the budget is divided
// evenly across the MAX slot count — on prod, 48 GiB over 64 scopes
// left each scope 768 MiB, however little the others held. A scope
// whose working set is larger refetches the same parts from storage
// on every read while the other scopes' shares sit mostly empty.

pub const MAX_OPEN_SCOPES: usize = 64;
pub const BUDGET_BYTES: u64 = 48 * 1024 * 1024 * 1024; // 48 GiB

pub struct ScopeCache {
    share: u64,
    used: u64,
    entries: std::collections::HashMap<u64, u64>, // part id -> len
}

impl ScopeCache {
    fn admit(&mut self, part: u64, len: u64) {
        while self.used + len > self.share {
            let Some(victim) = self.entries.keys().next().copied() else { break };
            if let Some(vlen) = self.entries.remove(&victim) {
                self.used -= vlen;
            }
        }
        self.used += len;
        self.entries.insert(part, len);
    }
}

pub struct Node {
    scopes: Vec<ScopeCache>, // one per scope, budget / MAX_OPEN_SCOPES each
}

impl Node {
    pub fn new() -> Self {
        let share = BUDGET_BYTES / MAX_OPEN_SCOPES as u64; // 768 MiB each
        Self {
            scopes: (0..MAX_OPEN_SCOPES)
                .map(|_| ScopeCache { share, used: 0, entries: Default::default() })
                .collect(),
        }
    }
}

#[test]
fn even_split_starves_the_skewed_scope() {
    let node = Node::new();
    // scope 1's working set is 20 GiB; its share is 768 MiB —
    // every read past the share refetches from storage
    let share = BUDGET_BYTES / MAX_OPEN_SCOPES as u64;
    assert_eq!(share, 768 * 1024 * 1024);
    assert_eq!(node.scopes.len(), MAX_OPEN_SCOPES); // 64 idle shares beside one starved scope
}"#.to_string(),
            fixed_code: r#"// OPT: ONE cache over the node's store — the whole budget and ONE
// eviction order — handed to every scope as their store. Entries are
// keyed by the scope's path prefix, so scopes cannot read each
// other's parts. A scope opens with no cache of its own.

pub const BUDGET_BYTES: u64 = 48 * 1024 * 1024 * 1024; // 48 GiB

pub struct SharedCache {
    budget: u64,
    used: u64,
    entries: std::collections::HashMap<String, u64>, // path -> len
}

impl SharedCache {
    pub fn new() -> Self {
        Self { budget: BUDGET_BYTES, used: 0, entries: Default::default() }
    }

    fn admit(&mut self, path: String, len: u64) {
        while self.used + len > self.budget {
            // ONE eviction order over the WHOLE budget — the idle
            // shares of a split design are here for the busy scope
            let Some(victim) = self.entries.keys().next().map(String::clone) else { break };
            if let Some(vlen) = self.entries.remove(&victim) {
                self.used -= vlen;
            }
        }
        self.used += len;
        self.entries.insert(path, len);
    }

    pub fn scopes_cannot_read_each_others_entries(&self, a: &str, b: &str) -> bool {
        // tenancy is the path prefix: different prefixes, disjoint keys
        a.split('/').next() != b.split('/').next()
    }
}

#[test]
fn one_pool_serves_the_whole_budget() {
    let mut cache = SharedCache::new();
    // the busy scope may use the WHOLE budget — no per-scope share
    let big: u64 = 20 * 1024 * 1024 * 1024;
    cache.admit("scope1/part0".into(), big / 2);
    cache.admit("scope1/part1".into(), big / 2);
    assert_eq!(cache.used, big); // the split design capped this at 768 MiB
    assert!(cache.scopes_cannot_read_each_others_entries("scope1", "scope2"));
}"#.to_string(),
            context_notes: Some("Grade B — caching, NotBenchable-by-domain (the win is a cache-QUALITY axis — served fetches / hit rate under a skewed working set; the validator's ns/alloc axes cannot express it, no per-op wall host; the QUALITY ordering is fixture-pinned in b365_fixtures F6). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified this session). THE LAWS IN-SOURCE (src/engine/shared_disk_cache.rs:1-30, module doc, verbatim fragments): 'A scoped runtime keeps up to `GRAPH_MAX_OPEN_SCOPES` of them, so the node's cache budget used to be split evenly across that many slots: on prod, 48 GiB over 64 scopes left each scope 768 MiB, however little the others held. A scope whose working set was larger refetched the same SST parts from object storage on every read, and evicted hundreds of MB a second, while most of the node's budget sat empty.' + the law: '[`SharedDiskCache`] instead builds one `CachedObjectStore` over the node's object store, with the whole budget and one eviction order, and hands that store to every scope as their object store.' + the isolation half: 'Entries are keyed by object path and every scope's objects live under its own prefix, so scopes cannot read each other's parts.' + the refusal half: 'A scope then opens with no disk cache of its own ([`SharedDiskCache::take`] clears it from the options), which is why the scoped runtime refuses options that still carry one.' REGIME: multi-tenant caches sized by max concurrency — per-connection caches, per-shard buffer pools, per-session object caches, per-worker scratch pools. ANTI-REGIMES: hard tenant ISOLATION requirements (one tenant must never be evicted by another — prefix-keying prevents READS, not evictions; per-tenant floors + a shared remainder is the nearest shape); homogeneous working sets (the split is harmless); budgets small enough that no scope's share is the binding constraint. ADJUDICATION (the note's named adjacency, re-verified both directions at authoring HEAD): cache-share-capped-at-measured-distinct-working-set owns per-tenant CAPS at a measured working-set knee — the OPPOSITE mechanism (cap the big tenant; this un-splits the budget into one pool) — DISTINCT both directions; cross-tier-lru-recency family (entries_54) owns recency propagation across TIERS — different axis; grep 'even split|one pool|shared cache|budget split' over rust_perf = zero owners. Plan 379 / Research 411.".to_string()),
            // NotBenchable-by-domain: the win is cache QUALITY under skew.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// Bench pair for `existential-witness-early-exit-scan` (Batch 365,
/// Plan 379 T3 / Research 411): the arity-blind full-enumeration scan vs
/// the predicate-arity dispatch (sequential witness search for the
/// existential population) over per-candidate prefix streams.
/// Integer-exact reads through a 1M-entry touch table; alloc-free both
/// arms; G1 census parity asserted in `make_input`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)]
fn existential_witness_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_024, 16_384, 262_144],
        harness: r#"
pub struct Input {
    /// flat prefix ids, per candidate: off[i]..off[i+1]
    off: Vec<u32>,
    ids: Vec<u32>,
    /// arity: true = existential (isolation), false = universal (detach)
    witness_only: Vec<bool>,
    alive: Vec<bool>,
    table: Vec<u64>,
    mask: u64,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// One stored-read = one real touch-table fold at the id's slot.
#[inline]
fn read(inp: &Input, id: u32, acc: &mut u64) {
    *acc = acc.wrapping_add(inp.table[(id as u64 & inp.mask) as usize]);
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0xB365_0002_5EED_0002;
    let mut off = Vec::with_capacity(n + 1);
    let mut ids = Vec::with_capacity(n * 32);
    let mut witness_only = Vec::with_capacity(n);
    off.push(0u32);
    let id_space = (n * 4 + 64) as u32;
    let mut alive = vec![false; id_space as usize];
    for a in alive.iter_mut() {
        *a = lcg(&mut state) % 100 < 60;
    }
    for _ in 0..n {
        let k = 4 + (lcg(&mut state) as usize % 60); // 4..=63 prefixes
        for _ in 0..k {
            ids.push((lcg(&mut state) % id_space as u64) as u32);
        }
        off.push(ids.len() as u32);
        witness_only.push(lcg(&mut state) % 100 < 85);
    }
    let mut table = vec![0u64; 1 << 20];
    let mut s2: u64 = 0x3654_BEEF_CAFE_F00D;
    for t in table.iter_mut() {
        s2 = lcg(&mut s2);
        *t = s2;
    }
    let mask = (1u64 << 20) - 1;
    let input = Input { off, ids, witness_only, alive, table, mask };
    // G1 parity: both arms report the same witness hits and the same
    // universal-arity survivor census
    let a = buggy::run(&input);
    let b = fixed::run(&input);
    assert_eq!(a, b, "arm parity on the witness/survivor census");
    input
}

mod buggy {
    use super::{read, Input};
    // ARITY-BLIND NAIVE: every candidate's scan reads EVERY prefix in
    // full — both the universal consumers (which need them) and the
    // existential consumers (which stop mattering after the first
    // witness).
    pub fn run(inp: &Input) -> u64 {
        let mut hits = 0u64;
        let mut acc = 0u64;
        for i in 0..inp.witness_only.len() {
            let (start, end) = (inp.off[i] as usize, inp.off[i + 1] as usize);
            let mut found = false;
            for id in &inp.ids[start..end] {
                read(inp, *id, &mut acc);
                if inp.alive[*id as usize] {
                    found = true;
                }
            }
            if inp.witness_only[i] && found {
                hits += 1;
            }
        }
        std::hint::black_box(acc);
        hits
    }
}

mod fixed {
    use super::{read, Input};
    // THE ARITY DISPATCH: existential candidates scan SEQUENTIALLY and
    // stop at the first witness; universal candidates keep the full
    // enumeration (the law — early exit there would be WRONG).
    pub fn run(inp: &Input) -> u64 {
        let mut hits = 0u64;
        let mut acc = 0u64;
        for i in 0..inp.witness_only.len() {
            let (start, end) = (inp.off[i] as usize, inp.off[i + 1] as usize);
            if inp.witness_only[i] {
                let mut found = false;
                for id in &inp.ids[start..end] {
                    read(inp, *id, &mut acc);
                    if inp.alive[*id as usize] {
                        // do not speculate past the first surviving edge
                        found = true;
                        break;
                    }
                }
                if found {
                    hits += 1;
                }
            } else {
                for id in &inp.ids[start..end] {
                    read(inp, *id, &mut acc);
                }
            }
        }
        std::hint::black_box(acc);
        hits
    }
}

pub fn run_buggy(input: &Input) {
    let hits = buggy::run(input);
    std::hint::black_box(hits);
}

pub fn run_fixed(input: &Input) {
    let hits = fixed::run(input);
    std::hint::black_box(hits);
}
"#
        .to_string(),
    }
}
