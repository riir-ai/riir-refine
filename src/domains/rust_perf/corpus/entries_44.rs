//! Entries slice B - Batch 271 (2026-10-21, Plan 275 / Research 307): the
//! splash-wave second-tier serving pool, slice B (3 dispatch rules) -
//! incoai/splash @ 4d179956, 68b9e706, dab88419, Apache-2.0; every quote
//! re-verified at the .raw/b271/splash clone this batch. All three rules are
//! corpus-only `NotBenchable` (decision-content serving laws: a bandwidth-
//! derived reader-cap partition, a fixed-read lane isolation with a per-lane
//! estimate split, and a reject-before-side-effectful-setup ordering) - no
//! proposer files. Fixtures are ORIGINAL machinery (never splash code
//! verbatim), each compiled and executed standalone (rustc --edition 2021
//! --test) before this table was assembled; the buggy/fixed arm pairs share
//! contract tests and differ only on the law.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "size-class-keyed-reader-cap-bimodal-reads".to_string(),
            name: "A shared SSD reader pool serves a restore queue of two size classes at ONE width -- index restores (a few KiB a read) and whole restores (a slot a read) both take every reader, so a wave of whole restores pins a slot of read memory per reader at once, outside the memory plan (13 MiB at Flash-Next's slot size, 32 to 64 MiB on the dense 27B; an index restore of 24 KiB pinned as far under the thread-local buffer it replaced) → partition the queue's submits by task SIZE CLASS and cap the large class's concurrency at a constant derived from measured bandwidth (kWholeRestoreReaders = 8, past 8 GB/s on an M5 Pro -- an M5 Pro reads 460 KiB pieces at 8.1 GB/s from four threads): index restores go out on every reader, whole restores on at most the cap, so worst-case in-flight pinned read memory is cap x per-task footprint (whole = a slot, index = its segment) and whole restores hold at most eight slots and only while they run".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `size-class-keyed-reader-cap-bimodal-reads` (riir-refine
// rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN arm.
//
// Re-authored shape from incoai/splash runtime/engine/KvPageTier.{hpp,cpp}
// @ 4d179956 (Apache-2.0): a shared SSD reader pool serving a restore queue
// of TWO size classes -- index restores (a few KiB each) and whole restores
// (a full slot each). The anti-pattern submits EVERY batch at the pool's
// full width, so a wave of whole restores pins a slot of read memory per
// reader at once -- outside the memory plan -- instead of capping the
// whole class at kWholeRestoreReaders (the bandwidth-derived constant) and
// letting the small class take every reader.
//
// ORIGINAL machinery -- never splash code verbatim. Deterministic rounds,
// never threads: a round advances every active task by one piece; the
// peak in-flight accounting is the decision content.

pub const READERS: usize = 32;
pub const SLOT_BYTES: usize = 13 * 1024 * 1024; // Flash-Next's slot size
pub const INDEX_BYTES: usize = 24 * 1024; // an index restore's read
pub const PIECE_BYTES: usize = 460 * 1024; // the piece a reader reads a round

/// A queued restore: which slot, and whether it reads only the index
/// segment (a few KiB) or the whole slot.
pub struct Restore {
    pub id: u64,
    pub index: bool,
    pub seg_bytes: usize, // the read's own size (slot start to segment end)
    pub done: bool,
}

#[derive(Default)]
pub struct Ledger {
    // the decision content: the worst-case read memory pinned at once,
    // and how wide each class ever ran
    pub peak_in_flight_bytes: usize,
    pub peak_whole_width: usize,
    pub peak_index_width: usize,
    pub completions: usize,
    pub rounds: usize,
}

pub struct Pool {
    pub queued: Vec<Restore>,
    pub ledger: Ledger,
}

impl Pool {
    pub fn new() -> Self {
        Pool {
            queued: Vec::new(),
            ledger: Ledger::default(),
        }
    }

    pub fn submit(&mut self, restores: Vec<Restore>) {
        self.queued.extend(restores);
    }

    // THE ANTI-PATTERN: the batch width is the pool's width for BOTH
    // classes -- a whole restore takes every reader too, so a full wave
    // pins READERS slots of read memory at once.
    fn whole_batch_width(&self) -> usize {
        READERS
    }

    // The two classes go out as two batches, the index restores first
    // (stable partition, submitRestores' order), each round advancing
    // every active task one piece; a task reads `seg_bytes` in total.
    pub fn drain(&mut self) {
        // index batch first, then whole batch (the partition order)
        let order: Vec<usize> = (0..self.queued.len())
            .filter(|&i| self.queued[i].index)
            .chain((0..self.queued.len()).filter(|&i| !self.queued[i].index))
            .collect();
        let mut cursor = 0;
        while cursor < order.len() {
            // the batch: every remaining task of ONE class
            let class_index = self.queued[order[cursor]].index;
            let batch: Vec<usize> = order[cursor..]
                .iter()
                .copied()
                .take_while(|&i| self.queued[i].index == class_index)
                .collect();
            cursor += batch.len();
            let width = if class_index {
                READERS
            } else {
                self.whole_batch_width()
            };
            // rounds until the batch drains; the in-flight accounting is
            // per round at full width for as long as tasks remain
            let mut remaining: Vec<usize> = batch;
            while !remaining.is_empty() {
                self.ledger.rounds += 1;
                let active = remaining.len().min(width);
                let in_flight: usize = remaining[..active]
                    .iter()
                    .map(|&i| self.queued[i].seg_bytes)
                    .sum();
                if in_flight > self.ledger.peak_in_flight_bytes {
                    self.ledger.peak_in_flight_bytes = in_flight;
                }
                if class_index {
                    if active > self.ledger.peak_index_width {
                        self.ledger.peak_index_width = active;
                    }
                } else if active > self.ledger.peak_whole_width {
                    self.ledger.peak_whole_width = active;
                }
                // one piece per active task per round
                for &i in remaining[..active].iter() {
                    let moved = PIECE_BYTES.min(self.queued[i].seg_bytes);
                    self.queued[i].seg_bytes -= moved;
                    if self.queued[i].seg_bytes == 0 {
                        self.queued[i].done = true;
                        self.ledger.completions += 1;
                    }
                }
                remaining.retain(|&i| !self.queued[i].done);
            }
        }
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_every_restore_completes_exactly_once() {
    let mut p = Pool::new();
    p.submit(vec![
        Restore { id: 1, index: true, seg_bytes: INDEX_BYTES, done: false },
        Restore { id: 2, index: false, seg_bytes: SLOT_BYTES, done: false },
        Restore { id: 3, index: false, seg_bytes: SLOT_BYTES, done: false },
    ]);
    p.drain();
    assert_eq!(p.ledger.completions, 3);
    assert!(p.queued.iter().all(|r| r.done));
}

#[test]
fn eq_index_restores_take_every_reader() {
    // more index restores than readers: the whole pool runs at once
    let mut p = Pool::new();
    p.submit((1..=48u64).map(|i| Restore {
        id: i,
        index: true,
        seg_bytes: INDEX_BYTES,
        done: false,
    }).collect());
    p.drain();
    assert_eq!(p.ledger.peak_index_width, READERS);
}

#[test]
fn eq_a_whole_restores_read_is_its_own_size() {
    // the whole class reads a slot, the index class its segment: the
    // per-task footprints the in-flight accounting is keyed on
    let mut p = Pool::new();
    p.submit(vec![Restore {
        id: 9,
        index: false,
        seg_bytes: SLOT_BYTES,
        done: false,
    }]);
    p.drain();
    assert_eq!(p.ledger.peak_in_flight_bytes, SLOT_BYTES);
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_a_whole_wave_pins_a_slot_per_reader() {
    // 12 whole restores over 32 readers: every one runs at once -- 12
    // slots of read memory pinned outside the memory plan (156 MiB at
    // Flash-Next's slot size), where the cap holds at most eight.
    let mut p = Pool::new();
    p.submit((1..=12u64).map(|i| Restore {
        id: i,
        index: false,
        seg_bytes: SLOT_BYTES,
        done: false,
    }).collect());
    p.drain();
    assert_eq!(p.ledger.peak_whole_width, 12);
    assert_eq!(p.ledger.peak_in_flight_bytes, 12 * SLOT_BYTES);
}

#[test]
fn bug_a_full_wave_pins_the_whole_pools_width() {
    // a wave at least as deep as the pool: the width IS the pool -- 32
    // slots in flight, 32 x 13 MiB = 416 MiB of read memory at once
    let mut p = Pool::new();
    p.submit((1..=32u64).map(|i| Restore {
        id: i,
        index: false,
        seg_bytes: SLOT_BYTES,
        done: false,
    }).collect());
    p.drain();
    assert_eq!(p.ledger.peak_whole_width, READERS);
    assert_eq!(p.ledger.peak_in_flight_bytes, READERS * SLOT_BYTES);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `size-class-keyed-reader-cap-bimodal-reads` (riir-refine
// rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/KvPageTier.{hpp,cpp}
// @ 4d179956 (Apache-2.0): a restore queue of TWO size classes goes out as
// two batches. The index class (a few KiB a read) takes EVERY reader; the
// whole class (a slot a read) takes at most kWholeRestoreReaders -- a
// constant derived from measured bandwidth (an M5 Pro reads 460 KiB pieces
// at 8.1 GB/s from four threads) -- so worst-case in-flight read memory is
// cap x per-task footprint, and whole restores hold at most eight slots
// and only while they run.
//
// ORIGINAL machinery -- never splash code verbatim. Deterministic rounds,
// never threads: a round advances every active task by one piece; the
// peak in-flight accounting is the decision content.

pub const READERS: usize = 32;
pub const SLOT_BYTES: usize = 13 * 1024 * 1024; // Flash-Next's slot size
pub const INDEX_BYTES: usize = 24 * 1024; // an index restore's read
pub const PIECE_BYTES: usize = 460 * 1024; // the piece a reader reads a round

// the bandwidth-derived cap: kWholeRestoreReaders
pub const K_WHOLE_READERS: usize = 8;

/// A queued restore: which slot, and whether it reads only the index
/// segment (a few KiB) or the whole slot.
pub struct Restore {
    pub id: u64,
    pub index: bool,
    pub seg_bytes: usize, // the read's own size (slot start to segment end)
    pub done: bool,
}

#[derive(Default)]
pub struct Ledger {
    // the decision content: the worst-case read memory pinned at once,
    // and how wide each class ever ran
    pub peak_in_flight_bytes: usize,
    pub peak_whole_width: usize,
    pub peak_index_width: usize,
    pub completions: usize,
    pub rounds: usize,
}

pub struct Pool {
    pub queued: Vec<Restore>,
    pub ledger: Ledger,
}

impl Pool {
    pub fn new() -> Self {
        Pool {
            queued: Vec::new(),
            ledger: Ledger::default(),
        }
    }

    pub fn submit(&mut self, restores: Vec<Restore>) {
        self.queued.extend(restores);
    }

    // THE LAW: the whole class's width is the cap, never the pool's
    // width -- their memory stays within kWholeRestoreReaders slots.
    fn whole_batch_width(&self) -> usize {
        K_WHOLE_READERS.min(READERS)
    }

    // The two classes go out as two batches, the index restores first
    // (stable partition, submitRestores' order), each round advancing
    // every active task one piece; a task reads `seg_bytes` in total.
    pub fn drain(&mut self) {
        let order: Vec<usize> = (0..self.queued.len())
            .filter(|&i| self.queued[i].index)
            .chain((0..self.queued.len()).filter(|&i| !self.queued[i].index))
            .collect();
        let mut cursor = 0;
        while cursor < order.len() {
            let class_index = self.queued[order[cursor]].index;
            let batch: Vec<usize> = order[cursor..]
                .iter()
                .copied()
                .take_while(|&i| self.queued[i].index == class_index)
                .collect();
            cursor += batch.len();
            let width = if class_index {
                READERS
            } else {
                self.whole_batch_width()
            };
            // rounds until the batch drains; at most `width` tasks are in
            // flight per round, so the pinned read memory is bounded by
            // width x the per-task footprint
            let mut remaining: Vec<usize> = batch;
            while !remaining.is_empty() {
                self.ledger.rounds += 1;
                let active = remaining.len().min(width);
                let in_flight: usize = remaining[..active]
                    .iter()
                    .map(|&i| self.queued[i].seg_bytes)
                    .sum();
                if in_flight > self.ledger.peak_in_flight_bytes {
                    self.ledger.peak_in_flight_bytes = in_flight;
                }
                if class_index {
                    if active > self.ledger.peak_index_width {
                        self.ledger.peak_index_width = active;
                    }
                } else if active > self.ledger.peak_whole_width {
                    self.ledger.peak_whole_width = active;
                }
                // one piece per active task per round
                for &i in remaining[..active].iter() {
                    let moved = PIECE_BYTES.min(self.queued[i].seg_bytes);
                    self.queued[i].seg_bytes -= moved;
                    if self.queued[i].seg_bytes == 0 {
                        self.queued[i].done = true;
                        self.ledger.completions += 1;
                    }
                }
                remaining.retain(|&i| !self.queued[i].done);
            }
        }
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_every_restore_completes_exactly_once() {
    let mut p = Pool::new();
    p.submit(vec![
        Restore { id: 1, index: true, seg_bytes: INDEX_BYTES, done: false },
        Restore { id: 2, index: false, seg_bytes: SLOT_BYTES, done: false },
        Restore { id: 3, index: false, seg_bytes: SLOT_BYTES, done: false },
    ]);
    p.drain();
    assert_eq!(p.ledger.completions, 3);
    assert!(p.queued.iter().all(|r| r.done));
}

#[test]
fn eq_index_restores_take_every_reader() {
    // more index restores than readers: the whole pool runs at once
    let mut p = Pool::new();
    p.submit((1..=48u64).map(|i| Restore {
        id: i,
        index: true,
        seg_bytes: INDEX_BYTES,
        done: false,
    }).collect());
    p.drain();
    assert_eq!(p.ledger.peak_index_width, READERS);
}

#[test]
fn eq_a_whole_restores_read_is_its_own_size() {
    // the whole class reads a slot, the index class its segment: the
    // per-task footprints the in-flight accounting is keyed on
    let mut p = Pool::new();
    p.submit(vec![Restore {
        id: 9,
        index: false,
        seg_bytes: SLOT_BYTES,
        done: false,
    }]);
    p.drain();
    assert_eq!(p.ledger.peak_in_flight_bytes, SLOT_BYTES);
}

// -- the law's own shape ------------------------------------------------------

#[test]
fn law_whole_restores_hold_at_most_k_whole_restore_readers_slots() {
    // 12 whole restores: at most 8 run at once -- their memory stays
    // within kWholeRestoreReaders slots (8 x 13 MiB = 104 MiB), and
    // only while they run
    let mut p = Pool::new();
    p.submit((1..=12u64).map(|i| Restore {
        id: i,
        index: false,
        seg_bytes: SLOT_BYTES,
        done: false,
    }).collect());
    p.drain();
    assert_eq!(p.ledger.completions, 12); // all complete regardless
    assert_eq!(p.ledger.peak_whole_width, K_WHOLE_READERS);
    assert_eq!(p.ledger.peak_in_flight_bytes, K_WHOLE_READERS * SLOT_BYTES);
}

#[test]
fn law_a_full_wave_never_exceeds_the_cap_either() {
    let mut p = Pool::new();
    p.submit((1..=32u64).map(|i| Restore {
        id: i,
        index: false,
        seg_bytes: SLOT_BYTES,
        done: false,
    }).collect());
    p.drain();
    assert_eq!(p.ledger.peak_whole_width, K_WHOLE_READERS.min(READERS));
    assert_eq!(p.ledger.peak_in_flight_bytes, K_WHOLE_READERS * SLOT_BYTES);
}

#[test]
fn law_the_cap_never_touches_the_small_class() {
    // a deep index wave still runs pool-wide: the cap is the WHOLE
    // class's, the small class takes every reader
    let mut p = Pool::new();
    p.submit((1..=64u64).map(|i| Restore {
        id: i,
        index: true,
        seg_bytes: INDEX_BYTES,
        done: false,
    }).collect());
    p.drain();
    assert_eq!(p.ledger.peak_index_width, READERS);
    assert_eq!(p.ledger.completions, 64);
}
"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is the in-flight pinned-memory ACCOUNTING of a shared reader pool under a bimodal restore queue -- which width each size class submits at, keyed on a bandwidth-derived constant -- and a Rust microbench of the stand-in cannot measure it; source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ 4d1799564d2ff9092a7963b914b9fc86f20fd640 'Read restores into memory of their own size, whole ones on fewer readers', Apache-2.0, Jian Chen, 2026-10-17; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'Restores moved to the 32 SSD readers for every target, each reading into a thread-local buffer grown to a whole slot and kept until the readers exit: every reader that ever ran a KV restore pinned a slot, outside the memory plan, 13 MiB at Flash-Next's slot size but 32 to 64 MiB on the dense 27B; an index restore of 24 KiB grew it as far.' THE LAW (runtime/engine/KvPageTier.cpp:151-153 @ 4d179956, verbatim): 'Index restores read a few KiB each, on every reader; a whole restore reads a slot, so fewer of them run at once and their memory stays within kWholeRestoreReaders slots.' -- realized as the batch-width dispatch (KvPageTier.cpp:171, verbatim code): 'index ? readers_->readers() : std::min(kWholeRestoreReaders, readers_->readers()),' over the stable-partitioned two-batch submit (KvPageTier.cpp:154: 'for (const bool index : {true, false})'). THE CALIBRATION (runtime/engine/KvPageTier.hpp:51-56 @ 4d179956, verbatim): 'Writes in flight at once, by default, and restores at most; the readers whole restores take at once (an M5 Pro reads 460 KiB pieces at 8.1 GB/s from four threads, model::SsdReaders).' over 'static constexpr uint32_t kWrites = 64;' + 'static constexpr uint32_t kRestores = 4096;' + 'static constexpr unsigned kWholeRestoreReaders = 8;' -- the cap is a MEASURED-BANDWIDTH constant, not a fraction of the pool. THE REPAIR (message, verbatim): 'The restores submitted together go out as two batches, the index restores on every reader and the whole restores, a slot each, on at most kWholeRestoreReaders (8, past 8 GB/s on an M5 Pro), so whole restores hold at most eight slots and only while they run.' -- the per-task footprints the worst case is keyed on are the read's own size ('A restore now reads into memory of its read's size, from the slot's start to a segment's end ... an index restore takes 24 KiB'), and the commit's BUFFER half (the thread-local reader buffer replaced by a task-scoped read, freed when its task ends) is a RIDER FACE landing separately on reuse-allocation-clear (entries_02) in the same batch -- THIS rule owns the size-class dispatch cap, the rider the buffer lifetime; they compose. The commit's completion-lifetime half ('poll() now retires a batch only once its end has returned (SsdReaders::stop), so no completion of a tier's restore runs after the tier has gone') is disclosed, not authored here. REGIME: shared reader pools serving a bimodal work queue (KiB-class index/segment reads beside slot-sized whole reads) where the large class's per-task footprint x the pool width is pinned memory the memory plan does not fund; the cap constant derived from measured bandwidth (the k tasks that saturate it). ANTI-REGIMES: uniform task sizes (no class to partition -- the pool width serves everyone); a small class whose footprint is itself slot-scale (the cap is the pool's); pools already freeing per-task buffers whose width x footprint is funded (the reservation story, reserve-stream-caches-at-admission's axis). KINSHIP/DISTINCT-FROM (read + disclosed): page-miss-blocks-one-ranged-read-scatter (B270, entries_42) owns read BATCHING and failure isolation (one ranged read per miss, a failed read is an error not an empty result); THIS owns the WIDTH each size class takes on a shared pool -- distinct axis, compose. reserve-stream-caches-at-admission-warm-request-paths-at-load (B257, entries_37) owns load-time RESERVATION of per-stream caches (allocate + warm at admission); THIS is per-submit concurrency capping of a shared pool at serving time -- reserve vs ration, distinct. constrained-unconstrained-lanes-one-decode-command (B270, entries_42) merges lanes by COST CLASS into one command; THIS partitions one queue's SUBMITS by READ-SIZE class across a fixed pool -- both class-keyed dispatch, the axis differs (who shares a command vs who shares the pool's width). DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'size class|size-class|reader pool|whole restore|restore reader|kWholeRestoreReaders|in-flight readers|460 KiB|8.1 GB/s' -- zero owners; 'bimodal' hits are katgpt-rs statistics fixtures (gaussianity benches), no dispatch rule; entries_42's 'reader pool' hits are the critical-busy write/read ordering faces, a different axis. FIXTURE PROVENANCE: original machinery (never splash code) -- a deterministic reader-pool simulator (rounds, never threads) over a two-class restore queue (index = 24 KiB segments, whole = 13 MiB slots, 32 readers, 460 KiB pieces a round, index-batch-first partition order), with peak-in-flight-bytes and peak-class-width ledgers; the shared-contract tests (every restore completes exactly once, index restores take every reader, a whole restore's read is its own size) pass in BOTH arms; the discriminating tests pin the whole wave (buggy: 12-wide and 32-wide waves pin 12x and 32x the slot size -- 416 MiB at the pool's width; fixed: 8-wide, 8 x slot bytes = 104 MiB) and the untouched small class (64 index restores still run 32-wide in the fixed arm). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ 4d1799564d2ff9092a7963b914b9fc86f20fd640 -- runtime/engine/KvPageTier.cpp:150-156 + :169-171, runtime/engine/KvPageTier.hpp:51-56, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "fixed-read-command-runs-alone-sized-by-its-own-reads".to_string(),
            name: "A prefill command whose cost is dominated by a row-independent fixed read (a lane whose history holds KV only the SSD cache has reads ALL of that history in each prefill command, layer by layer, whatever the command's rows) is planned like any other -- peers join the command it leads and wait the whole fixed read without amortizing any of it, its contended rows keep the group floor, its read wait trains the SHARED per-row estimate every other command's slices come from, and a command that has read nothing yet seeds the lane's estimate at zero → a lane that streams its history runs ALONE: nothing joins a command it leads, and it joins none it does not lead (overtaken often enough, it leads one); contended, it takes the fewest rows whose per-row time still covers its estimated wait on the reads (halving from its group floor while rows/2 stays at or above the contended floor and rows/2 x per-row still covers the wait -- fewer rows would not end the command sooner); the wait trains a PER-LANE estimate, never the shared per-row estimate (the shared estimate observes the command as if it had not waited); and a command that read nothing measures NOTHING, never zero".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `fixed-read-command-runs-alone-sized-by-its-own-reads`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/Scheduler.cpp
// @ 68b9e706 (Apache-2.0): a lane whose history holds KV only the SSD cache
// has reads ALL of that history in each prefill command, whatever the
// command's rows -- its cost is a FIXED read plus rows x per-row. The
// anti-pattern plans such a lane like any other: peers join its command
// (they wait for the whole fixed read and amortize none of it), its
// contended rows never shrink toward the fewest whose per-row time covers
// the fixed read, its read wait trains the SHARED per-row estimate every
// other command's slices come from, and a command that read nothing yet
// seeds the lane's estimate at zero.
//
// ORIGINAL machinery -- never splash code verbatim. One scheduler, four
// lanes, deterministic f64 arithmetic; the estimates are the decision
// content.

pub const GROUP_ROWS: usize = 256;
pub const PER_ROW_MS: f64 = 0.4;
pub const FIXED_READ_MS: f64 = 40.0;
pub const EMA_WEIGHT: f64 = 0.25;

/// A ready prefill lane: how many rows remain, whether it streams.
pub struct Ready {
    pub lane: usize,
    pub streams: bool,
    pub remaining: usize,
}

#[derive(Default)]
pub struct Plan {
    pub items: Vec<(usize, usize)>, // (lane, rows)
}

pub struct Scheduler {
    // the SHARED per-row estimate every command's slices come from
    pub per_row_est_ms: f64,
    // the PER-LANE fixed-read wait estimate (Option: no sample yet)
    pub lane_stream_ms: Vec<Option<f64>>,
}

impl Scheduler {
    pub fn new(lanes: usize) -> Self {
        Scheduler {
            per_row_est_ms: PER_ROW_MS,
            lane_stream_ms: vec![None; lanes],
        }
    }

    fn observe_per_row(&mut self, rows: usize, compute_ms: f64) {
        let sample = compute_ms / rows as f64;
        self.per_row_est_ms = (1.0 - EMA_WEIGHT) * self.per_row_est_ms + EMA_WEIGHT * sample;
    }

    // THE ANTI-PATTERN: a streaming leader is planned like any contended
    // leader -- its group floor, no wait-cover halving.
    fn streaming_rows(&self, leader_streams: bool, contended_rows: usize) -> usize {
        if !leader_streams {
            return GROUP_ROWS;
        }
        GROUP_ROWS.max(contended_rows)
    }

    pub fn plan(&self, leader: &Ready, peers: &[Ready], contended_rows: usize) -> Plan {
        let mut plan = Plan::default();
        let leader_rows = self.streaming_rows(leader.streams, contended_rows);
        plan.items.push((leader.lane, leader_rows));
        // peers join while budget allows -- streaming peers included
        for peer in peers {
            plan.items.push((peer.lane, peer.remaining.min(GROUP_ROWS)));
        }
        plan
    }

    // `stream_wait`: the lane's reported wait on its history's reads.
    // THE ANTI-PATTERN: the full wall trains the shared estimate, and a
    // read-nothing command (None) seeds the lane's estimate at zero.
    pub fn complete(
        &mut self,
        plan: &Plan,
        wall_ms: f64,
        host_ms: f64,
        stream_wait: Option<f64>,
    ) {
        let _ = host_ms;
        let rows: usize = plan.items.iter().map(|(_, r)| r).sum();
        self.observe_per_row(rows, wall_ms);
        let lane = plan.items[0].0;
        let waited = stream_wait.unwrap_or(0.0);
        self.lane_stream_ms[lane] = Some(match self.lane_stream_ms[lane] {
            Some(old) => (1.0 - EMA_WEIGHT) * old + EMA_WEIGHT * waited,
            None => waited,
        });
    }

    // a streaming command's wall: the fixed read plus the rows' compute
    pub fn stream_wall_ms(rows: usize) -> f64 {
        FIXED_READ_MS + rows as f64 * PER_ROW_MS
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_uncontended_streaming_takes_its_group() {
    // uncontended, a streaming lane takes its prefill group as before
    let s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 0);
    assert_eq!(plan.items, vec![(0, GROUP_ROWS)]);
}

#[test]
fn eq_plain_contended_command_plans_alike() {
    // a non-streaming leader joins its (non-streaming) peers in both arms
    let s = Scheduler::new(3);
    let leader = Ready { lane: 0, streams: false, remaining: 512 };
    let peers = vec![
        Ready { lane: 1, streams: false, remaining: 64 },
        Ready { lane: 2, streams: false, remaining: 32 },
    ];
    let plan = s.plan(&leader, &peers, 32);
    assert_eq!(plan.items.len(), 3);
    assert_eq!(plan.items[0], (0, GROUP_ROWS));
    assert_eq!(plan.items[1], (1, 64));
    assert_eq!(plan.items[2], (2, 32));
}

#[test]
fn eq_one_observed_wait_becomes_the_first_sample() {
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 120.0, Some(FIXED_READ_MS));
    assert_eq!(s.lane_stream_ms[0], Some(FIXED_READ_MS));
}

#[test]
fn eq_stream_estimate_is_an_ema() {
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 120.0, Some(40.0));
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 160.0, Some(80.0));
    let want = (1.0 - EMA_WEIGHT) * 40.0 + EMA_WEIGHT * 80.0;
    assert!((s.lane_stream_ms[0].unwrap() - want).abs() < 1e-9);
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_peers_join_a_streaming_command_and_wait_for_its_fixed_read() {
    // decoders join a streaming lane's command: each waits the WHOLE
    // fixed read, and nothing of it is amortized across them
    let s = Scheduler::new(4);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let peers = vec![
        Ready { lane: 1, streams: false, remaining: 8 },
        Ready { lane: 2, streams: false, remaining: 8 },
        Ready { lane: 3, streams: false, remaining: 8 },
    ];
    let plan = s.plan(&leader, &peers, 24);
    assert_eq!(plan.items.len(), 4, "nothing keeps the peers out");
    let wall = Scheduler::stream_wall_ms(GROUP_ROWS);
    // the wall is the fixed read plus the rows: every peer's wait
    // carries the whole fixed read, nothing of it amortized
    assert!((wall - FIXED_READ_MS - GROUP_ROWS as f64 * PER_ROW_MS).abs() < 1e-9);
    assert_eq!(plan.items[0], (0, GROUP_ROWS));
}

#[test]
fn bug_the_stream_wait_trains_the_shared_per_row_estimate() {
    // the command's wall is fixed-read + rows x per-row; folding the WHOLE
    // wall into the shared per-row estimate inflates it for every other
    // command's slices
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 120.0, Some(FIXED_READ_MS));
    assert!(s.per_row_est_ms > PER_ROW_MS, "per_row={}", s.per_row_est_ms);
}

#[test]
fn bug_contended_rows_never_shrink_toward_the_fixed_read() {
    // the fewest rows whose per-row time still covers the fixed read is
    // 128 (128 x 0.4 = 51.2 >= 40; 64 x 0.4 = 25.6 < 40) -- the
    // anti-pattern takes the group floor anyway
    let s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    assert_eq!(plan.items[0].1, GROUP_ROWS);
    assert_ne!(plan.items[0].1, 128);
}

#[test]
fn bug_a_command_that_read_nothing_seeds_the_estimate_at_zero() {
    // before its first pages spill, a streaming lane runs commands that
    // read no history: the anti-pattern seeds its estimate at 0.0, and
    // the next contended plan (in the law arm, 128 rows) collapses to
    // the contended floor
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, GROUP_ROWS as f64 * PER_ROW_MS, 120.0, None);
    assert_eq!(s.lane_stream_ms[0], Some(0.0));
    // the zeroed estimate: the halving check would pass at any width, so
    // the contended floor is all that remains
    let contended = 32usize;
    let est = s.lane_stream_ms[0].unwrap();
    let mut rows = GROUP_ROWS.max(contended);
    while rows / 2 >= contended && (rows / 2) as f64 * s.per_row_est_ms >= est {
        rows /= 2;
    }
    assert_eq!(rows, contended, "a zero estimate collapses the width");
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `fixed-read-command-runs-alone-sized-by-its-own-reads`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/Scheduler.cpp
// @ 68b9e706 (Apache-2.0): a command whose cost is dominated by a
// row-independent fixed read (a lane that streams its whole history
// whatever its rows) runs ALONE -- nothing joins a command it leads, and
// it joins none it does not lead; contended, it takes the fewest rows
// whose per-row time still covers its estimated wait on the reads (halving
// from its group floor); the wait trains ITS OWN lane estimate, never the
// shared per-row estimate; and a command that read nothing yet measures
// NOTHING (Option), never zero.
//
// ORIGINAL machinery -- never splash code verbatim. One scheduler, four
// lanes, deterministic f64 arithmetic; the estimates are the decision
// content.

pub const GROUP_ROWS: usize = 256;
pub const PER_ROW_MS: f64 = 0.4;
pub const FIXED_READ_MS: f64 = 40.0;
pub const EMA_WEIGHT: f64 = 0.25;

/// A ready prefill lane: how many rows remain, whether it streams.
pub struct Ready {
    pub lane: usize,
    pub streams: bool,
    pub remaining: usize,
}

#[derive(Default)]
pub struct Plan {
    pub items: Vec<(usize, usize)>, // (lane, rows)
}

pub struct Scheduler {
    // the SHARED per-row estimate every command's slices come from
    pub per_row_est_ms: f64,
    // the PER-LANE fixed-read wait estimate (Option: no sample yet)
    pub lane_stream_ms: Vec<Option<f64>>,
}

impl Scheduler {
    pub fn new(lanes: usize) -> Self {
        Scheduler {
            per_row_est_ms: PER_ROW_MS,
            lane_stream_ms: vec![None; lanes],
        }
    }

    fn observe_per_row(&mut self, rows: usize, compute_ms: f64) {
        let sample = compute_ms / rows as f64;
        self.per_row_est_ms = (1.0 - EMA_WEIGHT) * self.per_row_est_ms + EMA_WEIGHT * sample;
    }

    // THE LAW: a streaming leader's contended width is the fewest rows
    // whose per-row time still covers its estimated wait on the reads --
    // fewer rows would not end the command sooner. No sample yet: the
    // group floor stands.
    fn streaming_rows(&self, leader_streams: bool, lane: usize, contended_rows: usize) -> usize {
        if !leader_streams {
            return GROUP_ROWS;
        }
        let mut rows = GROUP_ROWS.max(contended_rows);
        if let Some(stream_ms) = self.lane_stream_ms[lane] {
            while rows / 2 >= contended_rows
                && (rows / 2) as f64 * self.per_row_est_ms >= stream_ms
            {
                rows /= 2;
            }
        }
        rows
    }

    pub fn plan(&self, leader: &Ready, peers: &[Ready], contended_rows: usize) -> Plan {
        let mut plan = Plan::default();
        let leader_rows = self.streaming_rows(leader.streams, leader.lane, contended_rows);
        plan.items.push((leader.lane, leader_rows));
        if leader.streams {
            // nothing joins a command a streaming lane leads
            return plan;
        }
        // and a streaming lane joins none it does not lead
        for peer in peers {
            if peer.streams {
                continue;
            }
            plan.items.push((peer.lane, peer.remaining.min(GROUP_ROWS)));
        }
        plan
    }

    // `stream_wait`: the lane's reported wait on its history's reads; a
    // command that read nothing reports none and measures nothing.
    // THE LAW: the shared estimate trains as if the command had not
    // waited on its history's reads, and the lane's own estimate with
    // that wait.
    pub fn complete(
        &mut self,
        plan: &Plan,
        wall_ms: f64,
        host_ms: f64,
        stream_wait: Option<f64>,
    ) {
        // only a prefill command of one lane streams its history
        assert!(
            stream_wait.is_none() || plan.items.len() == 1,
            "only a prefill command of one lane streams its history"
        );
        let waited = stream_wait
            .map(|w| w.clamp(0.0, host_ms.max(0.0)))
            .unwrap_or(0.0);
        let rows: usize = plan.items.iter().map(|(_, r)| r).sum();
        self.observe_per_row(rows, wall_ms - waited);
        if let Some(wait) = stream_wait {
            let lane = plan.items[0].0;
            let wait = wait.clamp(0.0, host_ms.max(0.0));
            self.lane_stream_ms[lane] = Some(match self.lane_stream_ms[lane] {
                Some(old) => (1.0 - EMA_WEIGHT) * old + EMA_WEIGHT * wait,
                None => wait,
            });
        }
    }

    // a streaming command's wall: the fixed read plus the rows' compute
    pub fn stream_wall_ms(rows: usize) -> f64 {
        FIXED_READ_MS + rows as f64 * PER_ROW_MS
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_uncontended_streaming_takes_its_group() {
    // uncontended, a streaming lane takes its prefill group as before
    let s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 0);
    assert_eq!(plan.items, vec![(0, GROUP_ROWS)]);
}

#[test]
fn eq_plain_contended_command_plans_alike() {
    // a non-streaming leader joins its (non-streaming) peers in both arms
    let s = Scheduler::new(3);
    let leader = Ready { lane: 0, streams: false, remaining: 512 };
    let peers = vec![
        Ready { lane: 1, streams: false, remaining: 64 },
        Ready { lane: 2, streams: false, remaining: 32 },
    ];
    let plan = s.plan(&leader, &peers, 32);
    assert_eq!(plan.items.len(), 3);
    assert_eq!(plan.items[0], (0, GROUP_ROWS));
    assert_eq!(plan.items[1], (1, 64));
    assert_eq!(plan.items[2], (2, 32));
}

#[test]
fn eq_one_observed_wait_becomes_the_first_sample() {
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 120.0, Some(FIXED_READ_MS));
    assert_eq!(s.lane_stream_ms[0], Some(FIXED_READ_MS));
}

#[test]
fn eq_stream_estimate_is_an_ema() {
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 120.0, Some(40.0));
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 160.0, Some(80.0));
    let want = (1.0 - EMA_WEIGHT) * 40.0 + EMA_WEIGHT * 80.0;
    assert!((s.lane_stream_ms[0].unwrap() - want).abs() < 1e-9);
}

// -- the law's own shape ------------------------------------------------------

#[test]
fn law_a_streaming_leader_runs_alone() {
    let s = Scheduler::new(4);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let peers = vec![
        Ready { lane: 1, streams: false, remaining: 8 },
        Ready { lane: 2, streams: false, remaining: 8 },
        Ready { lane: 3, streams: false, remaining: 8 },
    ];
    let plan = s.plan(&leader, &peers, 24);
    assert_eq!(plan.items, vec![(0, GROUP_ROWS)], "nothing joins it");
}

#[test]
fn law_a_streaming_lane_joins_none_it_does_not_lead() {
    let s = Scheduler::new(3);
    let leader = Ready { lane: 0, streams: false, remaining: 512 };
    let peers = vec![
        Ready { lane: 1, streams: true, remaining: 1024 },
        Ready { lane: 2, streams: false, remaining: 32 },
    ];
    let plan = s.plan(&leader, &peers, 32);
    assert_eq!(plan.items.len(), 2);
    assert_eq!(plan.items[0], (0, GROUP_ROWS));
    assert_eq!(plan.items[1], (2, 32), "the streaming peer is passed over");
}

#[test]
fn law_contended_rows_are_the_fewest_covering_the_fixed_read() {
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    // the first sample trains the lane estimate at 40 ms
    let plan = s.plan(&leader, &[], 32);
    assert_eq!(plan.items[0].1, GROUP_ROWS, "no sample yet: the group stands");
    s.complete(&plan, Scheduler::stream_wall_ms(GROUP_ROWS), 120.0, Some(FIXED_READ_MS));
    let plan = s.plan(&leader, &[], 32);
    assert_eq!(plan.items[0].1, 128);
    // the law's own cover check: 128 rows' compute still covers the wait,
    // the next halving would not
    assert!((128 / 2) as f64 * s.per_row_est_ms < s.lane_stream_ms[0].unwrap());
    assert!(128 as f64 * s.per_row_est_ms >= s.lane_stream_ms[0].unwrap());
}

#[test]
fn law_the_stream_wait_stays_out_of_the_shared_estimate() {
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    let plan = s.plan(&leader, &[], 32);
    let wall = Scheduler::stream_wall_ms(GROUP_ROWS);
    s.complete(&plan, wall, 120.0, Some(FIXED_READ_MS));
    // the shared estimate trained on compute alone: wall - wait == rows x
    // per-row, so it holds its value exactly
    assert!((s.per_row_est_ms - PER_ROW_MS).abs() < 1e-9);
    // the lane's own estimate took the wait
    assert_eq!(s.lane_stream_ms[0], Some(FIXED_READ_MS));
}

#[test]
fn law_a_command_that_read_nothing_measures_nothing() {
    let mut s = Scheduler::new(2);
    let leader = Ready { lane: 0, streams: true, remaining: 1024 };
    // the command before its first pages spill: no reads, no wait
    let plan = s.plan(&leader, &[], 32);
    s.complete(&plan, GROUP_ROWS as f64 * PER_ROW_MS, 120.0, None);
    assert_eq!(s.lane_stream_ms[0], None, "no zero seed");
    // the next contended plan still has no sample: the group stands
    let plan = s.plan(&leader, &[], 32);
    assert_eq!(plan.items[0].1, GROUP_ROWS);
}
"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is serving-PLAN policy -- which lanes join a command, how many rows a fixed-read leader takes when contended, and WHICH estimate its wait trains -- and a Rust microbench of the stand-in cannot measure it; the fixture's f64 arithmetic proves the shape, never a bar). SOURCE (incoai/splash @ 68b9e7069df0a64c2ad95cc322f4d293f4d0ce53 'Run a lane that streams its history alone, sized by its reads', Apache-2.0, Jian Chen, 2026-10-17; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE PREMISE (commit message, verbatim): 'A lane whose history holds KV only the SSD cache has reads all of that history in each prefill command, layer by layer, whatever the command's rows (KvHistoryStream).' -- the cost is a FIXED read plus rows x per-row, so the anti-pattern is planning that lane by the shared rules: peers join a command they only extend, the contended width keeps the group floor, the wait lands in the shared estimates, and a read-nothing command seeds a zero. THE LAW, four clauses, all verbatim @ 68b9e706: (1) THE ISOLATION (runtime/engine/Scheduler.cpp:343-347): 'A lane that streams its history runs alone: nothing joins a command it leads, and it joins none it does not lead; overtaken often enough, it leads one.' over 'if (view.request->streamsHistory && &view != &leader) continue;'. (2) THE CONTENDED WIDTH (Scheduler.cpp:414-422, the streamingRows halving loop, verbatim code): 'while (rows / 2 >= contendedRows && rows / 2 * prefillMillisecondsPerToken_ >= *leader.streamMilliseconds) rows /= 2;' from 'uint32_t rows = std::max(leader.spec.prefillGroupRows, contendedRows);' -- the fewest rows whose per-row time still covers the estimated wait; fewer rows would not end the command sooner, and it never takes fewer rows than another contended command; no sample yet (a None estimate) and the group stands. (3) THE ESTIMATE SPLIT (Scheduler.cpp:573-580): 'A streaming command trains the command estimate as if it had not waited on its history's reads, and its lane's own estimate with that wait.' over 'observePrefill(rows, wallMilliseconds - waited, hostMilliseconds - waited);' -- the message's statement of it: 'The cost estimate is per kind: the wait a command that read its history reports (ModelBatchTicket::streamMilliseconds) feeds that lane's own running estimate, which grows with its history, and stays out of the fixed and per-row estimate the other commands' slices come from.' (4) THE NO-ZERO-SEED (Scheduler.cpp:521-531): 'A lane that streams may run commands that read nothing yet, before its first pages spill: those say nothing of its reads.' over the std::optional nullopt form -- 'const std::optional<double> stream = streaming && std::isfinite(*streamMilliseconds) && std::isfinite(hostMilliseconds) ? std::optional(std::clamp(*streamMilliseconds, 0.0, std::max(hostMilliseconds, 0.0))) : std::nullopt;' (the message: 'the engine passes no wait for those (Scheduler::complete takes it as an optional), so they leave the estimate alone rather than seed it at zero'). THE REPAIR (message, verbatim, the planning bullets): 'One that does runs alone: it takes no peers into a command it leads and joins none it does not lead. Passed over, it counts as overtaken, so it leads a command after kMaximumOvertakes.' + 'Contended (decoders, or any ready peer of at least its priority, all of which wait for its whole command), it takes the fewest rows, halving from its group's, whose time still covers its estimated wait on the reads'. REGIME: batch schedulers with a lane whose per-command cost has a large row-INDEPENDENT term (a history stream, a fixed scan, a static load beside the rows) among row-linear peers; contended planning where peers only extend the wait and never amortize the fixed part. ANTI-REGIMES: cost that grows with rows (the merge pole -- see constrained-unconstrained-lanes-one-decode-command); a fixed read that DOES amortize across peers (a shared cache read -- isolation over-refuses there); uncontended-only schedulers (the group stands either way). KINSHIP/DISTINCT-FROM (read + disclosed, BOTH named): constrained-unconstrained-lanes-one-decode-command (B270, entries_42) is the MERGE pole -- its cost is PER-LANE, so one command can carry both modes and merging is the win; THIS is the ISOLATE pole -- the leader's cost is FIXED per command, so every peer is a pure waiter and the win is refusing them; its anti-regime 'expert-routed cycles whose cost grows with rows' does NOT cover fixed-read isolation -- they are the two poles of the same join decision, keyed on where the cost lives. lone-stream-graph-replay-mode-switch (B200, entries_14) keys the EXECUTION MODE on LIVE POPULATION (lone takes the graphs; joined goes eager); THIS keys the PLANNING on the cost SHAPE (a fixed read the rows cannot amortize) regardless of population -- a streaming lane runs alone beside ten decoders or zero; population there, cost-shape here. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'streams its history|streamsHistory|streaming lane|runs alone|fixed read|streamMilliseconds|observePrefill|prefill group|per-lane estimate|no-zero-seed|zero seed|streamingRows' -- zero owners; the 'runs alone' hits are entries_15's spec-copy-window ramp (a per-stream WIDTH ramp gated on being alone, not a join refusal) and entries_14's population mode switch; the 'zero seed' hits are RNG seeding prose. FIXTURE PROVENANCE: original machinery (never splash code) -- a deterministic scheduler (four lanes, f64 arithmetic) with a shared per-row estimate, per-lane Option stream estimates, a leader/peer planner and a completion observer; the shared-contract tests (uncontended streaming takes its group, plain contended planning identical in both arms, one observed wait becomes the first sample, the estimate is an EMA) pass in BOTH arms; the discriminating tests pin the join (buggy: 4 items in a streaming leader's command; fixed: 1), the estimate pollution (buggy: the shared per-row estimate inflates 0.4 to ~0.439 from one command; fixed: holds 0.4 exactly), the width (buggy: the group floor 256; fixed: 128, the fewest rows covering a 40 ms wait at 0.4 ms/row, with the cover check asserted), and the zero seed (buggy: Some(0.0) collapses the next contended width to the floor; fixed: None and the group stands). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ 68b9e7069df0a64c2ad95cc322f4d293f4d0ce53 -- runtime/engine/Scheduler.cpp:343-347 + :414-422 + :521-531 + :573-580, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "reject-plan-runs-before-setup-with-outliving-side-effects".to_string(),
            name: "A start's engine memory plan runs AFTER the setup whose side effects outlive a rejection -- a start whose plan could not hold the model opened or locked the SSD cache's files, logged the cache and removed the root's stale caches before it stopped, and the post-setup re-derivation ran for every target from disk state alone, where dropping the staging's reservation can RAISE a floor and refuse a start the first plan held → the plan that can reject runs BEFORE the side-effectful setup (the engine's plan with the staging the tier keeps; one that cannot hold the model stops the start with its breakdown, before the tier's files open); after the setup, re-derive the plan ONLY where the setup can move the outcome (the plan spills) and only MONOTONE -- a spilling plan takes the room the tier's volume leaves active KV, spills only where its KV file opened, and keeps the staging, so its floors can only shrink; a dense target never re-derives".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `reject-plan-runs-before-setup-with-outliving-side-effects`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the ANTI-PATTERN
// arm.
//
// Re-authored shape from incoai/splash runtime/engine/RuntimeResources.mm
// @ dab88419 (Apache-2.0): a start whose plan cannot hold the model opened
// or locked the SSD cache's files, logged the cache and removed the root's
// stale caches BEFORE it stopped -- the setup's side effects outlive the
// rejection -- and re-derived the plan unconditionally afterwards, from
// disk state alone, where the re-derivation can RAISE a floor and fail a
// start the first plan held.
//
// ORIGINAL machinery -- never splash code verbatim. The plan arithmetic is
// a floor model: floor = model KV - staging - disk room, rejected past the
// budget; the setup's side effects are a ledger.

pub const GIB: usize = 1 << 30;

/// The engine's plan: the KV floor it holds the model within, and whether
/// the floor needed the disk (the spill enabled).
pub struct EngineMemoryPlan {
    pub kv_floor_bytes: usize,
    pub spills: bool,
}

#[derive(Debug)]
pub struct RuntimeResourcesError {
    pub stage: &'static str,
    pub message: String,
}

/// The setup whose side effects outlive any rejection: open + lock the
/// tier's files, log, clear the root's stale caches (destructive).
#[derive(Default)]
pub struct Setup {
    pub files_opened: usize,
    pub files_locked: usize,
    pub log_lines: usize,
    pub stale_caches_cleared: usize,
    pub kv_file_open: bool,
}

/// Boot state: the side-effect ledger + the plan bookkeeping.
#[derive(Default)]
pub struct Boot {
    pub setup: Setup,
    pub plan_attempts: usize,
    pub rederives: usize,
    pub floors_seen: Vec<usize>,
    pub stale_caches: Vec<&'static str>,
    pub rejected: Option<RuntimeResourcesError>,
    pub plan: Option<EngineMemoryPlan>,
}

/// The plan: `staging` the tier keeps, `disk_room` what the volume leaves
/// active KV. The floor is what the KV must fit; over the budget, reject.
fn plan_memory(
    boot: &mut Boot,
    model_kv: usize,
    staging: usize,
    disk_room: usize,
    budget: usize,
) -> Result<EngineMemoryPlan, RuntimeResourcesError> {
    boot.plan_attempts += 1;
    let floor = model_kv.saturating_sub(staging).saturating_sub(disk_room);
    boot.floors_seen.push(floor);
    if floor > budget {
        return Err(RuntimeResourcesError {
            stage: "MemoryPlanning",
            message: format!(
                "the plan cannot hold the model: KV floor {} over the {} budget",
                floor, budget
            ),
        });
    }
    Ok(EngineMemoryPlan {
        kv_floor_bytes: floor,
        // the plan spills where the KV alone cannot sit in active memory:
        // it needs the disk to hold the model
        spills: model_kv > budget,
    })
}

// THE ANTI-PATTERN, part one: the setup runs BEFORE the plan that can
// reject, so a rejected start's side effects outlive it.
fn setup(boot: &mut Boot, open_kv_file: bool) {
    boot.setup.files_opened += 2; // the KV file + the state file
    boot.setup.files_locked += 2;
    boot.setup.log_lines += 1;
    boot.setup.stale_caches_cleared += boot.stale_caches.len();
    boot.stale_caches.clear();
    boot.setup.kv_file_open = open_kv_file;
}

// THE ANTI-PATTERN, part two: the re-derivation runs for EVERY target
// after the setup, from disk state alone -- the staging's reservation is
// lost, so the floor can RISE where the room is smaller than the staging.
fn boot_start(boot: &mut Boot, model_kv: usize, staging: usize, budget: usize) -> bool {
    setup(boot, true);
    match plan_memory(boot, model_kv, staging, 0, budget) {
        Err(e) => {
            boot.rejected = Some(e);
            false
        }
        Ok(plan) => {
            let spills = plan.spills;
            let floor = plan.kv_floor_bytes;
            boot.plan = Some(plan);
            // unconditional re-derive, disk state alone
            let disk_room = 6 * GIB;
            match plan_memory(boot, model_kv, 0, disk_room, budget) {
                Err(e) => {
                    boot.rejected = Some(e);
                    let _ = spills;
                    false
                }
                Ok(replanned) => {
                    boot.rederives += 1;
                    let _ = floor;
                    boot.plan = Some(replanned);
                    true
                }
            }
        }
    }
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_the_plan_arithmetic_rejects_past_the_budget() {
    let mut b = Boot::default();
    // 40 GiB model KV, no staging, no room: floor 40 over a 32 GiB budget
    assert!(plan_memory(&mut b, 40 * GIB, 0, 0, 32 * GIB).is_err());
    // with staging 8 + room 2 the floor is 30: it holds
    let plan = plan_memory(&mut b, 40 * GIB, 8 * GIB, 2 * GIB, 32 * GIB).unwrap();
    assert_eq!(plan.kv_floor_bytes, 30 * GIB);
}

#[test]
fn eq_setup_records_its_side_effects() {
    let mut b = Boot::default();
    b.stale_caches = vec!["root/stale-a", "root/stale-b"];
    setup(&mut b, true);
    assert_eq!(b.setup.files_opened, 2);
    assert_eq!(b.setup.files_locked, 2);
    assert_eq!(b.setup.log_lines, 1);
    assert_eq!(b.setup.stale_caches_cleared, 2);
    assert!(b.setup.kv_file_open);
}

#[test]
fn eq_a_held_start_rederives_once_after_the_files_open() {
    // a spilling plan that holds re-derives once after the setup, in
    // both arms (the arms differ on WHICH plans re-derive)
    let mut b = Boot::default();
    // floor 32 under a 48 GiB budget: holds; the anti-pattern re-derives
    // every plan that holds, this one included
    assert!(boot_start(&mut b, 40 * GIB, 8 * GIB, 48 * GIB));
    assert_eq!(b.rederives, 1);
}

// -- the anti-pattern's own shape ---------------------------------------------

#[test]
fn bug_a_rejected_start_side_effects_outlive_it() {
    // a plan that cannot hold the model AT ALL (44 GiB of KV, no
    // staging, 40 GiB budget): the setup opened and locked the cache's
    // files, logged the cache and destroyed the root's stale caches
    // before the start was stopped
    let mut b = Boot::default();
    b.stale_caches = vec!["root/stale-a", "root/stale-b", "root/stale-c"];
    assert!(!boot_start(&mut b, 44 * GIB, 0, 40 * GIB));
    assert!(b.rejected.is_some(), "the start was stopped");
    // ...after the setup's side effects outlived it
    assert_eq!(b.setup.files_opened, 2);
    assert_eq!(b.setup.files_locked, 2);
    assert_eq!(b.setup.log_lines, 1);
    assert_eq!(b.setup.stale_caches_cleared, 3);
    assert!(b.stale_caches.is_empty(), "the stale caches are gone");
    assert_eq!(b.plan_attempts, 1, "one plan, made after the setup");
}

#[test]
fn bug_the_setup_runs_before_the_first_plan_even_when_it_rejects() {
    // a plan that cannot hold the model AT ALL: the anti-pattern opens
    // and locks the tier's files before it finds out -- the setup ran
    // before the one plan that rejected
    let mut b = Boot::default();
    let ok = boot_start(&mut b, 40 * GIB, 0, 32 * GIB);
    assert!(!ok);
    assert!(b.rejected.is_some());
    assert_eq!(b.plan_attempts, 1);
    assert_eq!(b.setup.files_opened, 2, "the files opened before the reject");
}

#[test]
fn bug_the_unconditional_rederive_can_raise_a_floor() {
    // staging 8 GiB, room learned 6 GiB, budget 33: the first plan's
    // floor is 32 and HELD; the re-derive from disk state alone is 34 --
    // HIGHER than the floor it replaced -- and it REFUSES the start the
    // first plan held, after the files opened
    let mut b = Boot::default();
    assert!(!boot_start(&mut b, 40 * GIB, 8 * GIB, 33 * GIB));
    assert!(b.rejected.is_some(), "the start the first plan held is refused");
    assert_eq!(b.floors_seen, vec![32 * GIB, 34 * GIB], "the floor ROSE");
    assert!(b.setup.files_opened > 0, "after the files opened");
    assert_eq!(b.rederives, 0, "a failed re-derivation is no re-derivation");
}

#[test]
fn bug_a_dense_target_rederives_for_nothing() {
    // a plan that spills nothing (the KV fits active memory: 32 GiB of
    // KV under a 48 GiB budget): the anti-pattern still re-derives it
    // after the setup -- the setup cannot move that outcome
    let mut b = Boot::default();
    assert!(boot_start(&mut b, 32 * GIB, 8 * GIB, 48 * GIB));
    assert_eq!(b.rederives, 1, "re-derived though nothing could move");
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `reject-plan-runs-before-setup-with-outliving-side-effects`
// (riir-refine rust_perf corpus, Batch 271 / Plan 275) -- the LAW arm.
//
// Re-authored shape from incoai/splash runtime/engine/RuntimeResources.mm
// @ dab88419 (Apache-2.0): the plan that can REJECT a start runs BEFORE
// the setup whose side effects outlive the rejection (open + lock the SSD
// cache's files, log, clear stale caches); after the setup, the plan is
// re-derived ONLY where the setup can move the outcome (the plan spills)
// and only MONOTONE -- a spilling plan takes the room the volume leaves
// active KV and keeps the staging the tier holds, so its floors can only
// shrink; a dense target never re-derives.
//
// ORIGINAL machinery -- never splash code verbatim. The plan arithmetic is
// a floor model: floor = model KV - staging - disk room, rejected past the
// budget; the setup's side effects are a ledger.

pub const GIB: usize = 1 << 30;

/// The engine's plan: the KV floor it holds the model within, and whether
/// the floor needed the disk (the spill enabled).
pub struct EngineMemoryPlan {
    pub kv_floor_bytes: usize,
    pub spills: bool,
}

#[derive(Debug)]
pub struct RuntimeResourcesError {
    pub stage: &'static str,
    pub message: String,
}

/// The setup whose side effects outlive any rejection: open + lock the
/// tier's files, log, clear the root's stale caches (destructive).
#[derive(Default)]
pub struct Setup {
    pub files_opened: usize,
    pub files_locked: usize,
    pub log_lines: usize,
    pub stale_caches_cleared: usize,
    pub kv_file_open: bool,
}

/// Boot state: the side-effect ledger + the plan bookkeeping.
#[derive(Default)]
pub struct Boot {
    pub setup: Setup,
    pub kv_file_opens: bool,
    pub plan_attempts: usize,
    pub rederives: usize,
    pub floors_seen: Vec<usize>,
    pub stale_caches: Vec<&'static str>,
    pub rejected: Option<RuntimeResourcesError>,
    pub plan: Option<EngineMemoryPlan>,
}

/// The plan: `staging` the tier keeps, `disk_room` what the volume leaves
/// active KV. The floor is what the KV must fit; over the budget, reject.
fn plan_memory(
    boot: &mut Boot,
    model_kv: usize,
    staging: usize,
    disk_room: usize,
    budget: usize,
) -> Result<EngineMemoryPlan, RuntimeResourcesError> {
    boot.plan_attempts += 1;
    let floor = model_kv.saturating_sub(staging).saturating_sub(disk_room);
    boot.floors_seen.push(floor);
    if floor > budget {
        return Err(RuntimeResourcesError {
            stage: "MemoryPlanning",
            message: format!(
                "the plan cannot hold the model: KV floor {} over the {} budget",
                floor, budget
            ),
        });
    }
    Ok(EngineMemoryPlan {
        kv_floor_bytes: floor,
        // the plan spills where the KV alone cannot sit in active memory:
        // it needs the disk to hold the model
        spills: model_kv > budget,
    })
}

// The setup runs only after a plan that holds the model: its side effects
// can never outlive a rejection.
fn setup(boot: &mut Boot, open_kv_file: bool) {
    boot.setup.files_opened += 2; // the KV file + the state file
    boot.setup.files_locked += 2;
    boot.setup.log_lines += 1;
    boot.setup.stale_caches_cleared += boot.stale_caches.len();
    boot.stale_caches.clear();
    boot.setup.kv_file_open = open_kv_file;
}

// THE LAW: plan first; the setup only after a plan that holds; the
// re-derivation only where the plan spills, and monotone -- it keeps the
// staging and ADDS the room the volume leaves, so the floor can only
// shrink; where the KV file did not open, the room contributes nothing.
fn boot_start(boot: &mut Boot, model_kv: usize, staging: usize, budget: usize) -> bool {
    // the engine's plan with the staging the tier keeps; one that cannot
    // hold the model stops the start, before the tier's files open
    let memory_plan = match plan_memory(boot, model_kv, staging, 0, budget) {
        Err(e) => {
            boot.rejected = Some(e);
            return false;
        }
        Ok(p) => p,
    };
    // the tier's files open now, where the disk takes them
    let spills = memory_plan.spills;
    let floor = memory_plan.kv_floor_bytes;
    boot.plan = Some(memory_plan);
    setup(boot, boot.kv_file_opens);
    if spills {
        // a plan that spills takes the room the tier's volume leaves
        // active KV, and spills only where its KV file opened: its floors
        // can only shrink
        let disk_room = if boot.setup.kv_file_open { 6 * GIB } else { 0 };
        match plan_memory(boot, model_kv, staging, disk_room, budget) {
            Err(e) => {
                boot.rejected = Some(e);
                return false;
            }
            Ok(replanned) => {
                assert!(
                    replanned.kv_floor_bytes <= floor,
                    "the re-derivation is monotone: {} -> {}",
                    floor,
                    replanned.kv_floor_bytes
                );
                boot.rederives += 1;
                boot.plan = Some(replanned);
            }
        }
    }
    true
}

// -- tests -------------------------------------------------------------------

#[test]
fn eq_the_plan_arithmetic_rejects_past_the_budget() {
    let mut b = Boot::default();
    // 40 GiB model KV, no staging, no room: floor 40 over a 32 GiB budget
    assert!(plan_memory(&mut b, 40 * GIB, 0, 0, 32 * GIB).is_err());
    // with staging 8 + room 2 the floor is 30: it holds
    let plan = plan_memory(&mut b, 40 * GIB, 8 * GIB, 2 * GIB, 32 * GIB).unwrap();
    assert_eq!(plan.kv_floor_bytes, 30 * GIB);
}

#[test]
fn eq_setup_records_its_side_effects() {
    let mut b = Boot::default();
    b.stale_caches = vec!["root/stale-a", "root/stale-b"];
    setup(&mut b, true);
    assert_eq!(b.setup.files_opened, 2);
    assert_eq!(b.setup.files_locked, 2);
    assert_eq!(b.setup.log_lines, 1);
    assert_eq!(b.setup.stale_caches_cleared, 2);
    assert!(b.setup.kv_file_open);
}

#[test]
fn eq_a_held_start_rederives_once_after_the_files_open() {
    // a spilling plan that holds re-derives once after the setup, in
    // both arms (the arms differ on WHICH plans re-derive)
    let mut b = Boot::default();
    b.kv_file_opens = true;
    // 40 GiB of KV over a 39 GiB budget: the plan spills, its floor 32
    // holds, and the re-derived floor holds in both arms
    assert!(boot_start(&mut b, 40 * GIB, 8 * GIB, 39 * GIB));
    assert_eq!(b.rederives, 1);
}

// -- the law's own shape ------------------------------------------------------

#[test]
fn law_a_rejected_start_opens_nothing_and_clears_nothing() {
    // 40 GiB model KV, no staging, 32 GiB budget: the plan cannot hold
    // the model -- the start stops with its breakdown BEFORE the tier's
    // files open, and the root's stale caches are intact
    let mut b = Boot::default();
    b.stale_caches = vec!["root/stale-a", "root/stale-b", "root/stale-c"];
    assert!(!boot_start(&mut b, 40 * GIB, 0, 32 * GIB));
    let rejected = b.rejected.expect("the start was stopped");
    assert_eq!(rejected.stage, "MemoryPlanning");
    assert_eq!(b.plan_attempts, 1);
    assert_eq!(b.setup.files_opened, 0, "no file opened");
    assert_eq!(b.setup.files_locked, 0);
    assert_eq!(b.setup.log_lines, 0);
    assert_eq!(b.setup.stale_caches_cleared, 0);
    assert_eq!(b.stale_caches.len(), 3, "the stale caches survive the reject");
}

#[test]
fn law_the_rederivation_is_monotone_floors_can_only_shrink() {
    // the staging the tier keeps stays counted; the volume's room lands
    // on top: 40 - 8 - 6 = 26, never above the 32 it replaced
    let mut b = Boot::default();
    b.kv_file_opens = true;
    assert!(boot_start(&mut b, 40 * GIB, 8 * GIB, 36 * GIB));
    let plan = b.plan.unwrap();
    assert_eq!(plan.kv_floor_bytes, 26 * GIB);
    assert_eq!(b.floors_seen, vec![32 * GIB, 26 * GIB]);
    assert_eq!(b.rederives, 1);
}

#[test]
fn law_a_dense_target_never_rederives() {
    // a plan that spills nothing: the setup cannot move its outcome, so
    // the plan is never re-derived (the setup's own ordering holds)
    let mut b = Boot::default();
    assert!(boot_start(&mut b, 32 * GIB, 8 * GIB, 48 * GIB));
    assert_eq!(b.rederives, 0);
    assert_eq!(b.setup.files_opened, 2, "the files opened AFTER the plan held");
    assert_eq!(b.plan_attempts, 1);
}

#[test]
fn law_where_the_kv_file_did_not_open_the_floor_only_stays() {
    // the volume takes nothing (the KV file did not open): the room
    // contributes nothing, and the floor stays what the first plan made
    // -- the staging the tier keeps still counts
    let mut b = Boot::default();
    b.kv_file_opens = false;
    assert!(boot_start(&mut b, 40 * GIB, 8 * GIB, 36 * GIB));
    let plan = b.plan.unwrap();
    assert_eq!(plan.kv_floor_bytes, 32 * GIB);
    assert_eq!(b.floors_seen, vec![32 * GIB, 32 * GIB], "only shrink or stay");
    assert_eq!(b.rederives, 1);
    assert!(!b.setup.kv_file_open);
}
"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is boot-flow ORDERING policy -- a rejecting plan against a setup whose side effects outlive the rejection, and the monotonicity of the post-setup re-derivation -- and a Rust microbench of the stand-in cannot measure it; the fixture's floor arithmetic proves the shape, never a bar). SOURCE (incoai/splash @ dab88419a70128eaa9d61c1c05d866cef7fec1f6 'Stop a start whose plan cannot hold the model before the SSD cache's files open', Apache-2.0, Jian Chen, 2026-10-17; every quote re-verified verbatim at the .raw/b271/splash clone this batch at that pin; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'Since a block-sparse target's plan takes the room the SSD cache's volume leaves its KV, the start opened the cache's files before it made the engine's plan, for every target: a start whose plan could not hold the model opened or locked the cache's files, logged the cache and removed the root's stale caches before it stopped.' THE LAW, two clauses, both verbatim @ dab88419: (1) REJECT BEFORE SETUP (runtime/engine/RuntimeResources.mm:871-883): 'The engine's plan with the staging the tier keeps; one that cannot hold the model stops the start with its breakdown, before the tier's files open.' over 'const auto requirePlan = [&] {...}' + 'EngineMemoryPlan memoryPlan = requirePlan();'. (2) THE CONDITIONAL MONOTONE RE-DERIVE (RuntimeResources.mm:898-917): 'The tier's files open now, where the disk takes them (openDiskTier): the plan set the staging aside without asking it, and where they do not open the staging stays unused. A plan that spills takes the room the tier's volume leaves active KV, and spills only where its KV file opened: its floors can only shrink.' over 'if (memoryPlan.breakdown().kvSpill.enabled) { ... kvFileOpen = disk.kv != nullptr; ... memoryPlan = requirePlan(); }' -- the re-derivation runs ONLY inside the spills branch (a dense target's plan cannot move), and it can only LOWER floors (it adds the volume's room; it never takes the staging away), which is what makes it safe to run after the files already opened. THE REPAIR (message, verbatim): 'It plans first again, as it did, and plans once more after the files open only where the plan spills, whose floors the volume's room and a KV file that did not open can only lower.' + the effect line: 'Effect: a dense target's start opens the cache's files after its plan, as before its KV could spill.' THE ANTI-PATTERN'S SECOND FACE (the fixture's, the law's contrapositive): a re-derivation that re-plans from disk state ALONE drops the staging's reservation, so the floor can RISE (the fixture: floors 32 then 34 GiB at a 33 GiB budget) and refuse a start the first plan held -- with the files already opened; the monotone clause is the guard that makes the post-setup re-derive admissible at all. REGIME: boot/start sequences where a validating step (a memory plan, a budget check, an admission) can reject and later steps carry OUTLIVING side effects (file create/open/lock, log lines, destructive stale-cache clears); post-setup re-validation only where the setup can move the outcome, and then only monotone. ANTI-REGIMES: side-effect-free setup (any order works); a validation that DEPENDS on the setup's output (then the validation consumes the dependency and the ordering question is moot -- here the first plan deliberately plans WITHOUT the tier, 'the plan set the staging aside without asking it'); re-derivations that can raise a bar (refuse them or clamp them monotone). KINSHIP/DISTINCT-FROM (read + disclosed): reserve-stream-caches-at-admission-warm-request-paths-at-load (B257, entries_37) owns the setup side itself at LOAD (reserve + warm per-stream caches once the count is known) -- THIS orders a REJECTION against that class of setup and prices the re-derive's admission -- opposite concern, same surface, compose. degrade-not-fail-load-qualification-probe (B219, entries_21) owns load ADMISSION policy (a real-weight probe admitting or degrading a footprint); THIS owns WHEN the rejecting plan runs relative to side-effectful setup and the monotonicity of the post-setup re-plan -- admission-vs-ordering, distinct axes. reclaim-progress-keyed-on-refused-resource-returned (B270, entries_41) keys an allocation RETRY on the refused resource class actually returning after a refusal; THIS keys the ORDER of the refusal before destructive setup -- refusal semantics vs refusal ordering, distinct. DEDUP (greps at authoring over rust_perf + kernel_opt corpora): 'requirePlan|memoryPlan|re-derive|rederive|stale cache|kvSpill|openDiskTier|plan before|before the files open|side effect' -- zero owners in either corpus; the 're-derive'/'side effect' hits are clippy-proposer purity prose and the lint-table toolchain re-derive, unrelated axes. FIXTURE PROVENANCE: original machinery (never splash code) -- a boot sequencer with a floor-model plan (floor = model KV - staging - disk room, rejected past the budget; spills keyed on the KV alone exceeding the budget), a side-effect ledger (files opened/locked, log lines, stale caches cleared, a kv-file-open flag) and the spilling re-derivation; the shared-contract tests (the plan arithmetic rejects past the budget, the setup records its side effects, a held start re-derives once after the files open) pass in BOTH arms; the discriminating tests pin the rejection (buggy: files opened and locked, logged, 3 stale caches destroyed, then stopped; fixed: stopped at stage MemoryPlanning with zero files opened and the stale caches intact), the raise (buggy: floors 32 then 34 GiB at a 33 GiB budget, refused after the files opened; fixed: floors 32 then 26 GiB, the monotone assert in the re-derive), the dense target (buggy re-derives though nothing could move; fixed never re-derives), and the closed KV file (fixed: the room contributes nothing, floors 32 then 32 -- only shrink or stay). Wave: Batch 271 / Plan 275 / Research 307 (the splash-wave SECOND-TIER serving pool staged by Batch 270 / Research 306). Cite: @ dab88419a70128eaa9d61c1c05d866cef7fec1f6 -- runtime/engine/RuntimeResources.mm:871-883 + :898-917, commit message (all quotes); Batch 271 / Plan 275.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
