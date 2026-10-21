//! Entries slice D — Batch 270 (2026-10-21, Plan 274 / Research 306): the
//! splash-wave serving pool, slice D (io + the f52d1ad dispatch grade) —
//! incoai/splash @ c7a42ccb6d38049f0bdf53cf7fd93f51cd712121 +
//! c14ba7323274d626b176bc0544a92961736fd503 (rule 10) @
//! fc6fa5415c2d7f54e3da90caf8ba8a2b006e52ef + be309949048075c13379f1d3b4a2942cfde9bb49 +
//! 47325f9b0c96e9d60b5e1c588c6b750f8820e250 (rule 11) @
//! f52d1ad5123afb94d97a8b93b70faf8b76b1db17 (the dispatch grade, `LANDed`),
//! Apache-2.0; every quote re-verified at the .raw/b270/splash clone this
//! batch. Fixtures are ORIGINAL machinery — re-authored Rust miniatures, never
//! splash code verbatim — each compiled and executed standalone
//! (`rustc --edition 2021 --test`) before embedding: rule 10 arms 4/4 + 5/5,
//! rule 11 arms 5/5 + 6/6, dispatch arms 6/6 + 7/7.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── io: a spill-store resolve reads each missed block with its own
        // read and a twice-failed read fails the whole command → a page's
        // misses are ONE ranged read, and failure stays per READ ──────────
        // Source: incoai/splash @ c7a42cc + c14ba73 (Apache-2.0),
        // runtime/model/KvBlockCache.cpp (+ SsdReaders::runBatch via 47325f9).
        PerfRule {
            rule_id: "page-miss-blocks-one-ranged-read-scatter".to_string(),
            name: "A resolve over a spill store reads each missed block with its own read and a twice-failed read fails the whole command → a page's missed blocks of the layer are ONE ranged read, 'from the first missed block's record to the last's, and scatters each into its slot; the records between are read and left', and failure stays per READ — 'a read that fails twice fails every block of it, each lane that lists one, as a block's read did' — the scatter's other reads proceed and place, the command survives; run the reads as one now-batch that KEEPS its per-read failures (one wake, ahead of the background, counted as critical), never a background batch drained before finish() looked".to_string(),
            category: "io".to_string(),
            buggy_code: r##"#![allow(dead_code)]

// ORIGINAL fixture machinery for the riir-refine rust_perf corpus entry
// `page-miss-blocks-one-ranged-read-scatter` (Batch 270, slice D). The shape
// is distilled from incoai/splash (Apache-2.0) commits c7a42cc + c14ba73 --
// a hot-block-cache resolve over a spill store; this is a re-authored
// Rust miniature, never splash code verbatim.
//
// THIS FILE IS THE ANTI-PATTERN ARM. The naive resolve reads each missed
// block with its own read, and one read that fails twice fails the whole
// command: every page pays a read per block, and one bad page takes the
// scatter's good blocks down with it.

const RECORD_BYTES: usize = 8;

/// A read that fails is tried once more (a transient I/O error); a second
/// failure is the read's own.
const RETRIES: usize = 2;

/// The spill store: page `i` is `Some(bytes)` when its records are readable,
/// `None` when the page is missing (a read of it fails every time). Pages are
/// contiguous runs of fixed-size records; a record's bytes are a function of
/// its index so placement is checkable.
pub struct PageStore {
    pages: Vec<Option<Vec<u8>>>,
    pub reads: u64,
    pub bytes_brought: u64,
}

impl PageStore {
    pub fn new(pages: Vec<Option<Vec<u8>>>) -> Self {
        PageStore { pages, reads: 0, bytes_brought: 0 }
    }

    pub fn readable(page_count: usize, records_per_page: usize) -> Vec<Option<Vec<u8>>> {
        (0..page_count)
            .map(|p| {
                Some(
                    (0..records_per_page)
                        .flat_map(|r| record_bytes(p, r))
                        .collect(),
                )
            })
            .collect()
    }

    /// One positional read. Every attempt counts as a read; only bytes a
    /// successful read brings count as read bytes.
    pub fn read(&mut self, page: usize, offset: usize, len: usize) -> Option<Vec<u8>> {
        self.reads += 1;
        match &self.pages[page] {
            Some(bytes) => {
                self.bytes_brought += len as u64;
                Some(bytes[offset..offset + len].to_vec())
            }
            None => None,
        }
    }
}

/// A record's checkable bytes: a function of (page, record).
pub fn record_bytes(page: usize, record: usize) -> Vec<u8> {
    let seed = (page * 1_000_003 + record * 97 + 5) as u64;
    (0..RECORD_BYTES as u64).map(|i| (seed.wrapping_mul(i + 7) % 251) as u8).collect()
}

/// One missed block: the page that holds its record, the record's index in
/// the page, and the bitmask of lanes whose command lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Miss {
    pub page: usize,
    pub record: usize,
    pub lanes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub page: usize,
    pub record: usize,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneFailure {
    pub lane: u32,
    pub page: usize,
    pub record: usize,
}

/// What a resolve made observable. A resolve that fails as a command places
/// nothing its caller may use: the command's lanes fail together.
#[derive(Debug, PartialEq)]
pub struct Resolve {
    pub placed: Vec<Placed>,
    pub failures: Vec<LaneFailure>,
    pub command_failed: bool,
}

fn lanes_of(mask: u32) -> Vec<u32> {
    (0..32).filter(|&l| mask >> l & 1 == 1).collect()
}

/// THIS ARM: one read per missed block, retried once; a read that fails twice
/// throws, the batch rethrows its first failure once every task ended, and
/// the resolve fails as a whole -- every lane that listed any block of it
/// fails, and no placement is reported.
pub fn resolve(store: &mut PageStore, misses: &[Miss]) -> Resolve {
    let mut sorted: Vec<Miss> = misses.to_vec();
    sorted.sort_by_key(|m| (m.page, m.record));
    let mut placed = Vec::new();
    let mut any_failure = false;
    for miss in &sorted {
        let offset = miss.record * RECORD_BYTES;
        let mut bytes = None;
        for _ in 0..RETRIES {
            if bytes.is_none() {
                bytes = store.read(miss.page, offset, RECORD_BYTES);
            }
        }
        match bytes {
            Some(b) => placed.push(Placed { page: miss.page, record: miss.record, bytes: b }),
            None => any_failure = true,
        }
    }
    if any_failure {
        let mut failures = Vec::new();
        for m in &sorted {
            for lane in lanes_of(m.lanes) {
                failures.push(LaneFailure { lane, page: m.page, record: m.record });
            }
        }
        return Resolve { placed: Vec::new(), failures, command_failed: true };
    }
    Resolve { placed, failures: Vec::new(), command_failed: false }
}

/// The FIXED shape, embedded as the equivalence oracle (its own file carries
/// the full form and the law tests): a page's missed blocks of the layer are
/// ONE ranged read, from the first missed record to the last -- the records
/// between are read and left -- and a twice-failed read fails only the lanes
/// listing its own blocks. The scatter's other reads proceed.
pub fn resolve_fixed_oracle(store: &mut PageStore, misses: &[Miss]) -> Resolve {
    let mut sorted: Vec<Miss> = misses.to_vec();
    sorted.sort_by_key(|m| (m.page, m.record));
    // A page's blocks, in place order, are one span: consecutive sorted
    // misses of the same page join it.
    let mut spans: Vec<Vec<Miss>> = Vec::new();
    for m in &sorted {
        match spans.last_mut() {
            Some(span) if span[0].page == m.page => span.push(*m),
            _ => spans.push(vec![*m]),
        }
    }
    let mut placed = Vec::new();
    let mut failures = Vec::new();
    for span in &spans {
        let first = span[0].record;
        let last = span[span.len() - 1].record;
        let offset = first * RECORD_BYTES;
        let len = (last - first + 1) * RECORD_BYTES;
        let mut bytes = None;
        for _ in 0..RETRIES {
            if bytes.is_none() {
                bytes = store.read(span[0].page, offset, len);
            }
        }
        match bytes {
            Some(buf) => {
                for m in span {
                    let at = (m.record - first) * RECORD_BYTES;
                    placed.push(Placed {
                        page: m.page,
                        record: m.record,
                        bytes: buf[at..at + RECORD_BYTES].to_vec(),
                    });
                }
            }
            None => {
                // This read failed twice: the lanes listing ITS blocks fail.
                for m in span {
                    for lane in lanes_of(m.lanes) {
                        failures.push(LaneFailure { lane, page: m.page, record: m.record });
                    }
                }
            }
        }
    }
    Resolve { placed, failures, command_failed: false }
}

fn three_page_misses() -> Vec<Miss> {
    // Nine missed blocks over three pages (records 0, 2, 5 of each), lanes
    // 0 and 1 list page 0's, lane 1 lists page 1's, lane 2 lists page 2's.
    let mut misses = Vec::new();
    for (page, lanes) in [(0usize, 0b011u32), (1, 0b010), (2, 0b100)] {
        for record in [0usize, 2, 5] {
            misses.push(Miss { page, record, lanes });
        }
    }
    misses
}

#[test]
fn all_readable_placements_are_byte_identical_across_the_arms() {
    let mut a = PageStore::new(PageStore::readable(3, 8));
    let mut b = PageStore::new(PageStore::readable(3, 8));
    let misses = three_page_misses();
    let naive = resolve(&mut a, &misses);
    let fixed = resolve_fixed_oracle(&mut b, &misses);
    assert!(!naive.command_failed);
    assert!(!fixed.command_failed);
    assert!(naive.failures.is_empty() && fixed.failures.is_empty());
    assert_eq!(naive.placed, fixed.placed);
}

#[test]
fn the_naive_shape_pays_one_read_per_missed_block() {
    let mut store = PageStore::new(PageStore::readable(3, 8));
    let misses = three_page_misses();
    let _ = resolve(&mut store, &misses);
    assert_eq!(store.reads, misses.len() as u64); // 9 reads for 9 blocks
}

#[test]
fn a_page_miss_fails_the_whole_command_not_one_read() {
    let mut pages = PageStore::readable(3, 8);
    pages[1] = None; // page 1 is missing: every read of it fails twice
    let mut store = PageStore::new(pages);
    let misses = three_page_misses();
    let out = resolve(&mut store, &misses);
    assert!(out.command_failed);
    assert!(out.placed.is_empty(), "a failed command places nothing");
    // Every lane that lists ANY block of the resolve fails together: 9
    // misses, page 0's three blocks carry lanes 0+1 each, page 1's carry
    // lane 1, page 2's carry lane 2.
    let failed_lanes: Vec<u32> = out.failures.iter().map(|f| f.lane).collect();
    assert_eq!(failed_lanes, vec![0, 1, 0, 1, 0, 1, 1, 1, 1, 2, 2, 2]);
}

#[test]
fn the_fixed_shape_fails_only_the_lanes_listing_the_missing_page() {
    let mut pages = PageStore::readable(3, 8);
    pages[1] = None;
    let mut store = PageStore::new(pages);
    let misses = three_page_misses();
    let out = resolve_fixed_oracle(&mut store, &misses);
    assert!(!out.command_failed);
    assert_eq!(out.placed.len(), 6, "the other pages' reads proceed and place");
    assert!(out.failures.iter().all(|f| f.page == 1), "only the missing page's lanes fail");
    assert_eq!(out.failures.len(), 3);
}
"##.to_string(),
            fixed_code: r##"#![allow(dead_code)]

// ORIGINAL fixture machinery for the riir-refine rust_perf corpus entry
// `page-miss-blocks-one-ranged-read-scatter` (Batch 270, slice D). The shape
// is distilled from incoai/splash (Apache-2.0) commits c7a42cc + c14ba73 --
// a hot-block-cache resolve over a spill store; this is a re-authored
// Rust miniature, never splash code verbatim.
//
// THIS FILE IS THE LAW ARM. A page's missed blocks of the layer are ONE
// ranged read, from the first missed record to the last -- the records
// between are read and left -- and a read that fails twice fails only the
// lanes listing its own blocks: the scatter's other reads proceed and place.

const RECORD_BYTES: usize = 8;

/// A read that fails is tried once more (a transient I/O error); a second
/// failure is the read's own.
const RETRIES: usize = 2;

/// The spill store: page `i` is `Some(bytes)` when its records are readable,
/// `None` when the page is missing (a read of it fails every time). Pages are
/// contiguous runs of fixed-size records; a record's bytes are a function of
/// its index so placement is checkable.
pub struct PageStore {
    pages: Vec<Option<Vec<u8>>>,
    pub reads: u64,
    pub bytes_brought: u64,
}

impl PageStore {
    pub fn new(pages: Vec<Option<Vec<u8>>>) -> Self {
        PageStore { pages, reads: 0, bytes_brought: 0 }
    }

    pub fn readable(page_count: usize, records_per_page: usize) -> Vec<Option<Vec<u8>>> {
        (0..page_count)
            .map(|p| {
                Some(
                    (0..records_per_page)
                        .flat_map(|r| record_bytes(p, r))
                        .collect(),
                )
            })
            .collect()
    }

    /// One positional read. Every attempt counts as a read; only bytes a
    /// successful read brings count as read bytes.
    pub fn read(&mut self, page: usize, offset: usize, len: usize) -> Option<Vec<u8>> {
        self.reads += 1;
        match &self.pages[page] {
            Some(bytes) => {
                self.bytes_brought += len as u64;
                Some(bytes[offset..offset + len].to_vec())
            }
            None => None,
        }
    }
}

/// A record's checkable bytes: a function of (page, record).
pub fn record_bytes(page: usize, record: usize) -> Vec<u8> {
    let seed = (page * 1_000_003 + record * 97 + 5) as u64;
    (0..RECORD_BYTES as u64).map(|i| (seed.wrapping_mul(i + 7) % 251) as u8).collect()
}

/// One missed block: the page that holds its record, the record's index in
/// the page, and the bitmask of lanes whose command lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Miss {
    pub page: usize,
    pub record: usize,
    pub lanes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub page: usize,
    pub record: usize,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneFailure {
    pub lane: u32,
    pub page: usize,
    pub record: usize,
}

/// What a resolve made observable.
#[derive(Debug, PartialEq)]
pub struct Resolve {
    pub placed: Vec<Placed>,
    pub failures: Vec<LaneFailure>,
    pub command_failed: bool,
}

fn lanes_of(mask: u32) -> Vec<u32> {
    (0..32).filter(|&l| mask >> l & 1 == 1).collect()
}

/// THIS ARM: sort the misses by (page, record); a page's consecutive misses
/// are one span; the span is read in ONE ranged read from the first missed
/// record to the last, and each block scatters its slice into its slot -- the
/// records between are read and left. A span whose read fails twice fails
/// every block of it, each lane that lists one; the scatter's other reads
/// proceed.
pub fn resolve(store: &mut PageStore, misses: &[Miss]) -> Resolve {
    let mut sorted: Vec<Miss> = misses.to_vec();
    sorted.sort_by_key(|m| (m.page, m.record));
    let mut spans: Vec<Vec<Miss>> = Vec::new();
    for m in &sorted {
        match spans.last_mut() {
            Some(span) if span[0].page == m.page => span.push(*m),
            _ => spans.push(vec![*m]),
        }
    }
    let mut placed = Vec::new();
    let mut failures = Vec::new();
    for span in &spans {
        let first = span[0].record;
        let last = span[span.len() - 1].record;
        let offset = first * RECORD_BYTES;
        let len = (last - first + 1) * RECORD_BYTES;
        let mut bytes = None;
        for _ in 0..RETRIES {
            if bytes.is_none() {
                bytes = store.read(span[0].page, offset, len);
            }
        }
        match bytes {
            Some(buf) => {
                for m in span {
                    let at = (m.record - first) * RECORD_BYTES;
                    placed.push(Placed {
                        page: m.page,
                        record: m.record,
                        bytes: buf[at..at + RECORD_BYTES].to_vec(),
                    });
                }
            }
            None => {
                // This read failed twice: the lanes listing ITS blocks fail.
                for m in span {
                    for lane in lanes_of(m.lanes) {
                        failures.push(LaneFailure { lane, page: m.page, record: m.record });
                    }
                }
            }
        }
    }
    Resolve { placed, failures, command_failed: false }
}

/// The NAIVE shape, embedded as the equivalence oracle (its own file carries
/// the full form): one read per missed block; a read that fails twice fails
/// the whole resolve as a command.
pub fn resolve_buggy_oracle(store: &mut PageStore, misses: &[Miss]) -> Resolve {
    let mut sorted: Vec<Miss> = misses.to_vec();
    sorted.sort_by_key(|m| (m.page, m.record));
    let mut placed = Vec::new();
    let mut any_failure = false;
    for miss in &sorted {
        let offset = miss.record * RECORD_BYTES;
        let mut bytes = None;
        for _ in 0..RETRIES {
            if bytes.is_none() {
                bytes = store.read(miss.page, offset, RECORD_BYTES);
            }
        }
        match bytes {
            Some(b) => placed.push(Placed { page: miss.page, record: miss.record, bytes: b }),
            None => any_failure = true,
        }
    }
    if any_failure {
        let mut failures = Vec::new();
        for m in &sorted {
            for lane in lanes_of(m.lanes) {
                failures.push(LaneFailure { lane, page: m.page, record: m.record });
            }
        }
        return Resolve { placed: Vec::new(), failures, command_failed: true };
    }
    Resolve { placed, failures: Vec::new(), command_failed: false }
}

fn three_page_misses() -> Vec<Miss> {
    // Nine missed blocks over three pages (records 0, 2, 5 of each), lanes
    // 0 and 1 list page 0's, lane 1 lists page 1's, lane 2 lists page 2's.
    let mut misses = Vec::new();
    for (page, lanes) in [(0usize, 0b011u32), (1, 0b010), (2, 0b100)] {
        for record in [0usize, 2, 5] {
            misses.push(Miss { page, record, lanes });
        }
    }
    misses
}

#[test]
fn all_readable_placements_are_byte_identical_across_the_arms() {
    let mut a = PageStore::new(PageStore::readable(3, 8));
    let mut b = PageStore::new(PageStore::readable(3, 8));
    let misses = three_page_misses();
    let fixed = resolve(&mut a, &misses);
    let naive = resolve_buggy_oracle(&mut b, &misses);
    assert!(!fixed.command_failed);
    assert!(!naive.command_failed);
    assert!(fixed.failures.is_empty() && naive.failures.is_empty());
    assert_eq!(fixed.placed, naive.placed);
}

#[test]
fn a_pages_misses_are_one_read() {
    let mut store = PageStore::new(PageStore::readable(3, 8));
    let misses = three_page_misses();
    let _ = resolve(&mut store, &misses);
    // Nine blocks over three pages: three ranged reads, not nine.
    assert_eq!(store.reads, 3);
}

#[test]
fn the_span_reads_through_the_gap_and_leaves_the_records_between() {
    // One page, misses at records 0 and 5: the read spans records 0..=5
    // (48 bytes), places the two missed ones, and the four records between
    // are read and left.
    let mut store = PageStore::new(PageStore::readable(1, 8));
    let misses = vec![
        Miss { page: 0, record: 0, lanes: 1 },
        Miss { page: 0, record: 5, lanes: 2 },
    ];
    let out = resolve(&mut store, &misses);
    assert_eq!(store.reads, 1);
    assert_eq!(store.bytes_brought, 48);
    assert_eq!(out.placed.len(), 2);
    assert_eq!(out.placed[0].bytes, record_bytes(0, 0));
    assert_eq!(out.placed[1].bytes, record_bytes(0, 5));
}

#[test]
fn a_page_miss_fails_one_ranged_read_not_the_scatter() {
    let mut pages = PageStore::readable(3, 8);
    pages[1] = None; // page 1 is missing: its span's read fails twice
    let mut store = PageStore::new(pages);
    let misses = three_page_misses();
    let out = resolve(&mut store, &misses);
    assert!(!out.command_failed, "the command survives one page's failure");
    assert_eq!(out.placed.len(), 6, "the scatter's other reads proceed and place");
    assert!(out.placed.iter().all(|p| p.page != 1));
    // Only the lanes listing the failed span's blocks fail.
    assert!(out.failures.iter().all(|f| f.page == 1));
    assert_eq!(out.failures.len(), 3);
    assert_eq!(out.failures.iter().map(|f| f.lane).collect::<Vec<_>>(), vec![1, 1, 1]);
}

#[test]
fn read_bytes_counts_what_the_reads_brought() {
    let mut store = PageStore::new(PageStore::readable(3, 8));
    let misses = three_page_misses();
    let _ = resolve(&mut store, &misses);
    // Three spans of records 0..=5: 48 bytes each.
    assert_eq!(store.bytes_brought, 144);
}
"##.to_string(),
            context_notes: Some("Grade B+ — NotBenchable ON THIS BOX (the win is read-count + failure-isolation shape on a serving storage tier; the figures are the source's replay numbers over committed selection traces on an M5 Pro — source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ c7a42ccb6d38049f0bdf53cf7fd93f51cd712121 + c14ba7323274d626b176bc0544a92961736fd503, Apache-2.0 — runtime/model/KvBlockCache.cpp at the pins; every quote verbatim this batch). THE COALESCING LAW (c7a42cc commit message, verbatim): 'Reads of up to 16 KiB cost about the same as one of 8 KiB, and a layer's records of a page lie in place order, so a resolve now reads each page's missed blocks of the layer in one read, from the first missed block's record to the last's, and scatters each into its slot; the records between are read and left.' — in-code, the resolve doc (KvBlockCache.cpp:205-210): 'reads of up to 16 KiB cost about the same, and the records of a layer's blocks lie in place order, so one read across a page's misses, the records between them too, costs less than a read for each.'; :271 'A page's blocks, in place order, are one span.'; the Span doc :71-73 'One read of a page's blocks a resolve misses in its layer, the reads [first, end) of its blocks, in place order: from the first block's record to the last's, the records between too.'; the mechanics — spanRange :184-188 (the first missed record's offset through the last's end) and readSpan :190-202: ONE readKvCopy of the span, then a per-record subspan scatter ('Blocks that cannot be read leave zeros, which their lanes' attention reads before they fail.', :193-194). THE ISOLATION LAW (the staged core — failure granularity is the READ, never the command): commit 'A read that fails twice fails every block of it, each lane that lists one, as a block's read did.'; in-code :280-281 'blocks the readers read twice in vain fail the lanes that list them, not the command.'; the loop :287-301 walks batch->failed(index) PER SPAN — a failed span frees its blocks' slots and fails the lanes listing them while every other span places (the scatter's other reads proceed); readSpan's throw (:202) is the task's own — the batch keeps it per task (Batch::failed) instead of rethrowing it command-wide (run() 'is it, throwing the first'). THE BATCH HALF (c14ba73, riding 47325f9's runBatch — the tier-write rule's third commit): 'A resolve started its reads as a background batch and finished it at once (SsdReaders::start, finish), to keep each block's failure for the lanes that list it, which woke the readers twice and, when they took every task before finish() looked, ran them without counting as reads a step waits for (criticalBusy), so the drive's writes did not hold back. It now runs them now with SsdReaders::runBatch, which keeps the failures.' — keep-failures needs a run-now batch that RETURNS per-task failures; the start+finish simulation paid two wakes and could silently disarm the deferral trigger (see tier-write-defers-to-urgent-reads-except-a-read-behind-it). MEASURED (source-reported, c7a42cc message): 'Replayed on the committed selection traces at the floor, a step's read time drops by 10-11% (1.2x for a 16 KiB read, linear in the bytes beyond).' — at 256K on the M5 Pro the hot block cache 'misses about 2,400 blocks a step' at 8 KiB each, 'nearly all a spilled decode's cost over resident (4.5 s of the 5.1 s it adds to 2 x 512 tokens)'; also 'read_bytes counts what the reads brought' (:289, stats.readBytes += spanRange().bytes — the spans' bytes, not per-block sums). REGIME: a decode step resolves a scatter of missed blocks; a layer's records of a page are contiguous in place order, so a page's misses span one range; reads to ~16 KiB cost like one 8 KiB read. ANTI-REGIME: misses spread one-per-page across far-apart pages (nothing to coalesce — the latency-overlap axis is parallel-disjoint-pread-hides-cold-page-latency); spans far past 16 KiB ('linear in the bytes beyond' — the over-read becomes real transfer); records NOT in place order per page (the span premise breaks). DISTINCT-FROM parallel-disjoint-pread-hides-cold-page-latency (B118): that overlaps COLD PAGE-IN LATENCY for DISJOINT ranges across threads (QD1 to QD32) and pins an ALL-OR-NOTHING error contract ('error contract = all-or-nothing (AND of per-range results)') — THIS merges NEAR-ADJACENT (same-page) records into fewer reads AND pins the OPPOSITE error contract, per-span isolation; the two compose, and B118's own anti-regime names near-adjacent sets as coalescing's regime. DISTINCT-FROM coalesce-nearby-reads-bounded-gap-cacheability-gated (B94): same fewer-reads-for-near-rows family, but B94's decision content is the GAP BUDGET coupled to cacheability (64 KiB page-cache-gated; gap 0 on direct fds) and it says nothing about failure granularity — THIS merges within a page's own extent (bounded by construction, no budget knob, no cacheability axis) and its content is the isolation granularity + the run-now counting. DISTINCT-FROM checkpoint-reads-skip-page-cache-with-refusal-fallback (B257): the LOAD path's direct-I/O + refusal fallback — different surface, no isolation axis. KINSHIP: batch-verify-optimistic-aggregate-fallback-individual — work granularity aggregate-vs-individual with a fallback on a verify path; this is the storage-read twin (isolate the failing unit, keep the rest working). Suggestion-only (I/O orchestration; a parse-only proposer cannot see page locality, batch structure, or lane masks). Fixtures ORIGINAL machinery, never splash code (.raw/b270/fix_page_miss_*; both arms embed BOTH shapes and cross-assert byte-identical placements on all-readable stores, reads 3 vs 9, and the isolation contrast on a one-page-miss store). Cite: @ c7a42ccb6d38049f0bdf53cf7fd93f51cd712121 — runtime/model/KvBlockCache.cpp:71-73+184-202+205-210+271-301; @ c14ba7323274d626b176bc0544a92961736fd503 — runtime/model/KvBlockCache.cpp (start+finish to runBatch); Batch 270 / Plan 274 / Research 306.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── io: a slow-tier write never yields (or yields to every read) →
        // defers to urgent reads, bounded, EXCEPT while one of its own
        // file's reads waits behind it ─────────────────────────────────────
        // Source: incoai/splash @ fc6fa54 + be30994 + 47325f9 (Apache-2.0),
        // runtime/model/SlotFile.{hpp,cpp} + SsdReaders.{hpp,cpp}.
        PerfRule {
            rule_id: "tier-write-defers-to-urgent-reads-except-a-read-behind-it".to_string(),
            name: "A slow-tier write either never yields or yields to every read → a write holds each chunk of a write back while the reads a step waits for run, at most 2 ms a chunk, EXCEPT while a read of its own file waits behind it in the same worker queue: 'while any waits, a write holds back for no other reads, and it holds back again once the worker took the last' — or the read behind would wait as long as the write it follows, and a write that deferred to every arriving read would starve; a worker that only writes keeps the deferral whole; and the trigger must COUNT the reads a step actually waits for (a now-batch, counted critical while it runs — a background batch drained before finish() looked ran uncounted and silently disarmed the deferral)".to_string(),
            category: "io".to_string(),
            buggy_code: r##"#![allow(dead_code)]

// ORIGINAL fixture machinery for the riir-refine rust_perf corpus entry
// `tier-write-defers-to-urgent-reads-except-a-read-behind-it` (Batch 270,
// slice D). The shape is distilled from incoai/splash (Apache-2.0) commits
// fc6fa54 + be30994 + 47325f9 -- a slow-tier slot-file worker whose write
// chunks yield to urgent reads; this is a re-authored deterministic Rust
// miniature (discrete ticks, one queue, no threads), never splash code
// verbatim.
//
// THIS FILE IS THE ANTI-PATTERN ARM. The write holds each chunk back while
// the reader pool's urgent batch runs -- and that is ALL it checks: a read
// of its own file queued BEHIND it waits as long, because the write ahead
// of it holds back for readers it does not even share the queue with.

/// Ticks a write chunk may hold back (the source's 2 ms per 1 MiB chunk).
pub const K_DEFERRAL: u64 = 8;
/// The deferral poll (the source's 100 us re-check).
pub const POLL: u64 = 1;

/// One tier file's worker op.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// A write of `1` chunks... label and chunk count.
    Write(&'static str, u32),
    Read(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub label: &'static str,
    pub is_write: bool,
    pub at_tick: u64,
}

/// The queue's outcome. Completion ORDER is queue order in every arm; the
/// deferral only moves WHEN a completion lands and what the budget counted.
#[derive(Debug, PartialEq)]
pub struct Schedule {
    pub completions: Vec<Completion>,
    pub deferred_ticks: u64,
}

/// The tier file's worker: one FIFO queue of reads and writes, run front to
/// back. `critical_busy` says a now-batch of urgent reads (reads a step waits
/// for) is running on the engine's reader pool -- the drive is busy with them.
pub struct Worker {
    pub critical_busy: bool,
    queue: std::collections::VecDeque<Op>,
    queued_reads: u32,
    now: u64,
    deferred: u64,
}

impl Worker {
    pub fn new(critical_busy: bool) -> Self {
        Worker { critical_busy, queue: std::collections::VecDeque::new(), queued_reads: 0, now: 0, deferred: 0 }
    }

    pub fn submit(&mut self, op: Op) {
        if matches!(op, Op::Read(_)) {
            self.queued_reads += 1;
        }
        self.queue.push_back(op);
    }

    /// How many reads of this file sit in the queue right now (the count the
    /// submit path and the worker keep under the queue's lock).
    pub fn queued_reads(&self) -> u32 {
        self.queued_reads
    }

    /// THIS ARM: a chunk waits while the pool's urgent batch runs -- whether
    /// or not a read of this file waits behind the write in its own queue.
    fn should_hold_back(&self) -> bool {
        self.critical_busy
    }

    /// Run the queue front to back; each read is one tick of service, each
    /// write chunk first holds back (a bounded while, polled) then serves one
    /// tick.
    pub fn drain(mut self) -> Schedule {
        let mut completions = Vec::new();
        while let Some(op) = self.queue.pop_front() {
            match op {
                Op::Read(label) => {
                    // The worker took the read: it leaves the queue count.
                    self.queued_reads -= 1;
                    self.now += 1;
                    completions.push(Completion { label, is_write: false, at_tick: self.now });
                }
                Op::Write(label, chunks) => {
                    for _ in 0..chunks {
                        let mut waited = 0;
                        while self.should_hold_back() && waited < K_DEFERRAL {
                            self.now += POLL;
                            self.deferred += POLL;
                            waited += POLL;
                        }
                        self.now += 1;
                    }
                    completions.push(Completion { label, is_write: true, at_tick: self.now });
                }
            }
        }
        Schedule { completions, deferred_ticks: self.deferred }
    }
}

/// The FIXED arm, embedded as the ordering oracle (its own file carries the
/// full form and the law tests): a chunk holds back while the pool's urgent
/// batch runs EXCEPT while a read of this file waits behind the write -- then
/// it holds back for no other reads, and defers again once the worker took
/// the last queued read.
pub fn drain_fixed_oracle(critical_busy: bool, ops: &[Op]) -> Schedule {
    let mut queue = std::collections::VecDeque::new();
    let mut queued_reads = 0u32;
    for &op in ops {
        if matches!(op, Op::Read(_)) {
            queued_reads += 1;
        }
        queue.push_back(op);
    }
    let mut now = 0u64;
    let mut deferred = 0u64;
    let mut completions = Vec::new();
    while let Some(op) = queue.pop_front() {
        match op {
            Op::Read(label) => {
                queued_reads -= 1;
                now += 1;
                completions.push(Completion { label, is_write: false, at_tick: now });
            }
            Op::Write(label, chunks) => {
                for _ in 0..chunks {
                    let mut waited = 0;
                    while queued_reads == 0 && critical_busy && waited < K_DEFERRAL {
                        now += POLL;
                        deferred += POLL;
                        waited += POLL;
                    }
                    now += 1;
                }
                completions.push(Completion { label, is_write: true, at_tick: now });
            }
        }
    }
    Schedule { completions, deferred_ticks: deferred }
}

fn w2() -> Op {
    Op::Write("w2mib", 2)
}
fn w1(label: &'static str) -> Op {
    Op::Write(label, 1)
}
fn r(label: &'static str) -> Op {
    Op::Read(label)
}

#[test]
fn same_ops_complete_in_the_same_order_in_both_arms() {
    let ops = vec![w2(), r("hot-block")];
    let mut a = Worker::new(true);
    for &op in &ops {
        a.submit(op);
    }
    let naive = a.drain();
    let fixed = drain_fixed_oracle(true, &ops);
    // The law reorders nothing: labels and order are queue order in both.
    let labels: Vec<&'static str> = naive.completions.iter().map(|c| c.label).collect();
    let labels_fixed: Vec<&'static str> = fixed.completions.iter().map(|c| c.label).collect();
    assert_eq!(labels, labels_fixed);
    assert_eq!(labels, vec!["w2mib", "hot-block"]);
}

#[test]
fn the_write_holds_back_even_with_a_read_of_its_file_waiting_behind_it() {
    let mut a = Worker::new(true);
    a.submit(w2());
    a.submit(r("hot-block"));
    let naive = a.drain();
    // Two chunks, each held back the full bound while the urgent batch runs.
    assert_eq!(naive.deferred_ticks, 2 * K_DEFERRAL);
}

#[test]
fn the_read_behind_waits_as_long_as_the_write_it_follows() {
    let ops = vec![w2(), r("hot-block")];
    let mut a = Worker::new(true);
    for &op in &ops {
        a.submit(op);
    }
    let naive = a.drain();
    let fixed = drain_fixed_oracle(true, &ops);
    let read_naive = naive.completions.iter().find(|c| !c.is_write).unwrap();
    let read_fixed = fixed.completions.iter().find(|c| !c.is_write).unwrap();
    assert_eq!(read_naive.at_tick, 19); // 8 + 1 + 8 + 1 + 1
    assert_eq!(read_fixed.at_tick, 3);
    assert!(read_fixed.at_tick < read_naive.at_tick);
}

#[test]
fn a_write_only_file_keeps_the_deferral_whole() {
    // No reads ever queued: both arms defer identically.
    let ops = vec![w1("state")];
    let mut a = Worker::new(true);
    for &op in &ops {
        a.submit(op);
    }
    let naive = a.drain();
    let fixed = drain_fixed_oracle(true, &ops);
    assert_eq!(naive.deferred_ticks, K_DEFERRAL);
    assert_eq!(fixed.deferred_ticks, K_DEFERRAL);
}

#[test]
fn a_batch_not_counted_critical_disarms_the_deferral() {
    // The deferral is exactly as strong as the counting of the reads a step
    // waits for: nothing counted critical, nothing held back (the silent
    // disarm class the runBatch fix closed).
    let ops = vec![w1("state"), r("hot-block")];
    let mut a = Worker::new(false);
    for &op in &ops {
        a.submit(op);
    }
    let naive = a.drain();
    let fixed = drain_fixed_oracle(false, &ops);
    assert_eq!(naive.deferred_ticks, 0);
    assert_eq!(fixed.deferred_ticks, 0);
}
"##.to_string(),
            fixed_code: r##"#![allow(dead_code)]

// ORIGINAL fixture machinery for the riir-refine rust_perf corpus entry
// `tier-write-defers-to-urgent-reads-except-a-read-behind-it` (Batch 270,
// slice D). The shape is distilled from incoai/splash (Apache-2.0) commits
// fc6fa54 + be30994 + 47325f9 -- a slow-tier slot-file worker whose write
// chunks yield to urgent reads; this is a re-authored deterministic Rust
// miniature (discrete ticks, one queue, no threads), never splash code
// verbatim.
//
// THIS FILE IS THE LAW ARM. A write holds each chunk back while the reads a
// step waits for run -- at most the bound per chunk -- EXCEPT while a read of
// its own file waits behind it in the same worker queue: then it holds back
// for no other reads (or the read behind would wait as long, and a write
// that always yielded would starve), and it defers again once the worker
// took the last queued read.

/// Ticks a write chunk may hold back (the source's 2 ms per 1 MiB chunk).
pub const K_DEFERRAL: u64 = 8;
/// The deferral poll (the source's 100 us re-check).
pub const POLL: u64 = 1;

/// One tier file's worker op.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// A write of the given chunk count under a label.
    Write(&'static str, u32),
    Read(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub label: &'static str,
    pub is_write: bool,
    pub at_tick: u64,
}

/// The queue's outcome. Completion ORDER is queue order in every arm; the
/// deferral only moves WHEN a completion lands and what the budget counted.
#[derive(Debug, PartialEq)]
pub struct Schedule {
    pub completions: Vec<Completion>,
    pub deferred_ticks: u64,
}

/// The tier file's worker: one FIFO queue of reads and writes, run front to
/// back. `critical_busy` says a now-batch of urgent reads (reads a step waits
/// for) is running on the engine's reader pool -- the drive is busy with them.
pub struct Worker {
    pub critical_busy: bool,
    queue: std::collections::VecDeque<Op>,
    queued_reads: u32,
    now: u64,
    deferred: u64,
}

impl Worker {
    pub fn new(critical_busy: bool) -> Self {
        Worker { critical_busy, queue: std::collections::VecDeque::new(), queued_reads: 0, now: 0, deferred: 0 }
    }

    pub fn submit(&mut self, op: Op) {
        if matches!(op, Op::Read(_)) {
            self.queued_reads += 1;
        }
        self.queue.push_back(op);
    }

    /// How many reads of this file sit in the queue right now (the count the
    /// submit path and the worker keep under the queue's lock).
    pub fn queued_reads(&self) -> u32 {
        self.queued_reads
    }

    /// THIS ARM: a chunk holds back only while the pool's urgent batch runs
    /// AND no read of this file waits behind the write in its own queue. A
    /// file whose worker only writes keeps the deferral whole.
    fn should_hold_back(&self) -> bool {
        self.queued_reads == 0 && self.critical_busy
    }

    /// Run the queue front to back; each read is one tick of service, each
    /// write chunk first holds back (a bounded while, polled) then serves one
    /// tick.
    pub fn drain(mut self) -> Schedule {
        let mut completions = Vec::new();
        while let Some(op) = self.queue.pop_front() {
            match op {
                Op::Read(label) => {
                    // The worker took the read: it leaves the queue count,
                    // and the writes behind it defer again.
                    self.queued_reads -= 1;
                    self.now += 1;
                    completions.push(Completion { label, is_write: false, at_tick: self.now });
                }
                Op::Write(label, chunks) => {
                    for _ in 0..chunks {
                        let mut waited = 0;
                        while self.should_hold_back() && waited < K_DEFERRAL {
                            self.now += POLL;
                            self.deferred += POLL;
                            waited += POLL;
                        }
                        self.now += 1;
                    }
                    completions.push(Completion { label, is_write: true, at_tick: self.now });
                }
            }
        }
        Schedule { completions, deferred_ticks: self.deferred }
    }
}

/// The NAIVE arm, embedded as the ordering oracle (its own file carries the
/// full form): a chunk holds back while the pool's urgent batch runs whether
/// or not a read of this file waits behind the write.
pub fn drain_buggy_oracle(critical_busy: bool, ops: &[Op]) -> Schedule {
    let mut queue = std::collections::VecDeque::new();
    for &op in ops {
        queue.push_back(op);
    }
    let mut now = 0u64;
    let mut deferred = 0u64;
    let mut completions = Vec::new();
    while let Some(op) = queue.pop_front() {
        match op {
            Op::Read(label) => {
                now += 1;
                completions.push(Completion { label, is_write: false, at_tick: now });
            }
            Op::Write(label, chunks) => {
                for _ in 0..chunks {
                    let mut waited = 0;
                    while critical_busy && waited < K_DEFERRAL {
                        now += POLL;
                        deferred += POLL;
                        waited += POLL;
                    }
                    now += 1;
                }
                completions.push(Completion { label, is_write: true, at_tick: now });
            }
        }
    }
    Schedule { completions, deferred_ticks: deferred }
}

fn w2() -> Op {
    Op::Write("w2mib", 2)
}
fn w1(label: &'static str) -> Op {
    Op::Write(label, 1)
}
fn r(label: &'static str) -> Op {
    Op::Read(label)
}

#[test]
fn same_ops_complete_in_the_same_order_in_both_arms() {
    let ops = vec![w2(), r("hot-block")];
    let mut a = Worker::new(true);
    for &op in &ops {
        a.submit(op);
    }
    let fixed = a.drain();
    let naive = drain_buggy_oracle(true, &ops);
    // The law reorders nothing: labels and order are queue order in both.
    let labels: Vec<&'static str> = fixed.completions.iter().map(|c| c.label).collect();
    let labels_naive: Vec<&'static str> = naive.completions.iter().map(|c| c.label).collect();
    assert_eq!(labels, labels_naive);
    assert_eq!(labels, vec!["w2mib", "hot-block"]);
}

#[test]
fn a_write_with_a_read_of_its_file_queued_behind_it_does_not_hold_back() {
    let mut a = Worker::new(true);
    a.submit(w2());
    let read = r("hot-block");
    a.submit(read);
    assert_eq!(a.queued_reads(), 1);
    let fixed = a.drain();
    // The read behind would wait as long if the write yielded again: it does
    // not. The bound counts nothing here.
    assert_eq!(fixed.deferred_ticks, 0);
    let read_done = fixed.completions.iter().find(|c| !c.is_write).unwrap();
    assert_eq!(read_done.at_tick, 3);
}

#[test]
fn the_next_write_holds_back_again_once_the_worker_took_the_last_read() {
    let ops = vec![w1("state-a"), r("hot-block"), w1("state-b")];
    let mut a = Worker::new(true);
    for &op in &ops {
        a.submit(op);
    }
    let fixed = a.drain();
    // state-a: a read waits behind -> runs at once (tick 1). hot-block: taken
    // at tick 2, the queue count drops to 0. state-b: no reads behind now ->
    // it holds back the full bound again.
    assert_eq!(fixed.deferred_ticks, K_DEFERRAL);
    let labels: Vec<(&'static str, u64)> =
        fixed.completions.iter().map(|c| (c.label, c.at_tick)).collect();
    assert_eq!(labels, vec![("state-a", 1), ("hot-block", 2), ("state-b", 11)]);
}

#[test]
fn a_write_only_file_keeps_the_deferral_whole() {
    // No reads ever queued: the deferral is unchanged by the except arm.
    let ops = vec![w1("state")];
    let mut a = Worker::new(true);
    for &op in &ops {
        a.submit(op);
    }
    let fixed = a.drain();
    let naive = drain_buggy_oracle(true, &ops);
    assert_eq!(fixed.deferred_ticks, K_DEFERRAL);
    assert_eq!(naive.deferred_ticks, K_DEFERRAL);
    assert_eq!(fixed, naive);
}

#[test]
fn a_chunk_waits_at_most_the_bound_and_the_budget_counts_the_wait() {
    let ops = vec![w2(), r("hot-block"), w1("state-c")];
    let mut a = Worker::new(true);
    for &op in &ops {
        a.submit(op);
    }
    let fixed = a.drain();
    // hot-block runs early (no deferral of w2mib); state-c defers the full
    // bound once the queue is read-free. The budget counts exactly the wait.
    assert_eq!(fixed.deferred_ticks, K_DEFERRAL);
    let w_done = fixed.completions.iter().find(|c| c.label == "w2mib").unwrap();
    assert_eq!(w_done.at_tick, 2, "two chunks, no hold-back");
}

#[test]
fn a_batch_not_counted_critical_disarms_the_deferral() {
    // The deferral is exactly as strong as the counting of the reads a step
    // waits for: nothing counted critical, nothing held back (the silent
    // disarm class the runBatch fix closed).
    let ops = vec![w1("state"), r("hot-block")];
    let mut a = Worker::new(false);
    for &op in &ops {
        a.submit(op);
    }
    let fixed = a.drain();
    assert_eq!(fixed.deferred_ticks, 0);
}
"##.to_string(),
            context_notes: Some("Grade B+ — NotBenchable ON THIS BOX (the win is a queue-ordering/deference law on a serving storage tier; the figures are the source's M5-Pro/M5-Max serving measurements — source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ fc6fa5415c2d7f54e3da90caf8ba8a2b006e52ef + be309949048075c13379f1d3b4a2942cfde9bb49 + 47325f9b0c96e9d60b5e1c588c6b750f8820e250, Apache-2.0 — runtime/model/SlotFile.{hpp,cpp} + SsdReaders.{hpp,cpp} at the pins; every quote verbatim this batch). THE DEFERRAL (fc6fa54, verbatim): 'The SSD cache's files hold each 1 MiB chunk of a write back while a batch runs now on the pool, the reads a step waits for, at most 2 ms a chunk (SlotFile::Options::deferWrites over SsdReaders::criticalBusy), so that a write burst yields to an expert load and, later, to a spilled block's read; the budget counts the time (DiskBudget::writesDeferredMicroseconds).' — the bound SlotFile.hpp:332-333 'The longest a write's chunk waits for reads (Options::deferWrites).' + kWriteDeferral{2}ms; the re-check SlotFile.cpp:427-428 'How often a write's chunk looks again whether the reads it waits for ended.' + kDeferralPoll 100us; the hold-back loop :854-866 (at fc6fa54) with defer() before each chunk's move (:894); the counter SlotFile.hpp:209-213 'Cumulative microseconds the files' writes waited for reads a step waits for'; SsdReaders.hpp:15 (at fc6fa54) 'writes to the drive hold back (criticalBusy)'. THE EXCEPT ARM (be30994 — the rule's core): the commit is titled 'Let a write hold back for no other reads while one of its file's waits behind it' and its body: 'A file's worker runs reads and writes in one queue, so a read queued behind a write that held each chunk back for the readers' urgent reads waited as long: up to 2 ms a MiB, 128 ms behind a 35B's 64 MiB state with streamed experts.' + 'The worker now counts the reads in its queue (Backing::queuedReads, kept by submit and the worker under the queue's lock); while any waits, a write holds back for no other reads, and it holds back again once the worker took the last. A file whose worker only writes, as the KV tier's will, keeps the deferral whole.' — SlotFile.cpp:861-862 'Reads a step waits for go first: while they run, a chunk waits for them, a bounded while, unless a read of this file waits behind it.'; :863-865 the whole law in one predicate: 'return !backing.queuedReads.load(std::memory_order_relaxed) && backing.deferWrites();' — the write DEFERS ONLY WHILE NOTHING OF ITS OWN IS BEHIND IT; the count is kept at submit (:758-760 fetch_add) and released when the worker TAKES the read (:787-789 fetch_sub), so the deferral re-arms exactly when the last queued read leaves the queue; the field doc :54-55 'The reads in it, which every write ahead of them delays: while any waits, a write holds back for no other reads (Options::deferWrites).'; the Work flag SlotFile.hpp:478-479 'A read: the writes ahead of it hold back for no other reads.'; the class doc :278 'most kWriteDeferral, unless a read of the file waits behind the write.'; the Options doc :326-331 '...unless a read of the file waits behind the write and so would wait as long.'. THE TRIGGER-COUNTING HALF (47325f9 + c14ba73): the deferral is exactly as strong as the counting of the reads a step waits for — 'runBatch() runs a batch now as run() does, one wake of the readers, ahead of the background and counted as critical while it runs, and returns it with its failures its own (Batch::failed); run() is it, throwing the first.'; the defect closed: 'its readers were woken twice, first for the background queue, where some took an older background batch first, and when they had taken every task before finish() looked, the batch ran without counting as one a step waits for (criticalBusy), so the drive's writes did not hold back for it.' — SsdReaders.hpp:9-10 (at 47325f9) 'A batch either runs now (run, runBatch, finish), on the calling thread and every reader, ahead of every background batch, because a step waits for it'; criticalBusy() :144; runBatch :120-122 + SsdReaders.cpp:83-93. ANTI-STARVATION READING (why the except arm is law, not a special case): a write that deferred to every read with reads arriving continuously never completes, and a read queued behind it waits behind a write that yields to readers it does not even share the queue with — the except arm bounds BOTH: the write proceeds while its own queue holds a read; the read behind waits one write, not one write plus every future read's deferral. MEASURED (source-reported, be30994 message): the read behind waited 'up to 2 ms a MiB, 128 ms behind a 35B's 64 MiB state with streamed experts' — gone while its own read waits. REGIME: one worker queue mixing a tier's slow writes with its own reads; a cross-tier urgent-read pool (expert loads, spilled-KV resolves) whose busy state is visible to the write path; chunked writes (the bound is PER CHUNK, and the budget counts the waited time). ANTI-REGIME: a write-only worker ('keeps the deferral whole'); files that must never wait (the deferral is opt-in per file: 'A model whose weights do not stream never runs a batch now, so its writes never wait'); a priority QUEUE where reads jump ahead of writes (reordering, a different mechanism, no starvation question); unbounded deferral (never — the 2 ms bound is load-bearing). DISTINCT-FROM deferred-command-flush-hoist-out-of-batch-loop (B174): that amortizes a fixed barrier cost over a batch loop (fewer sync points, one flush after the loop); THIS is priority deference with a starvation guard — same work, different WAITING; neither subsumes the other. DISTINCT-FROM channel-send-batching-unpark and bufwriter-batched (entries_02): wakeup batching and syscall buffering — no priority axis, no except arm. DISTINCT-FROM cache-write-admission-requires-restoring-consumer (B239, caching): write ADMISSION (does a future task restore this state?), not write deference — no waiting axis. NOVEL: grep 'defer|urgent|starv|yield|priorit|hold back' over the io-category rules: zero owner of read-priority write deferral or an except-while-behind arm. Suggestion-only (I/O orchestration; a parse-only proposer cannot see queue composition or cross-tier busy state). Fixtures ORIGINAL machinery, never splash code (.raw/b270/fix_tier_write_*; deterministic discrete-tick worker sims — both arms embed both predicates, cross-assert identical completion ORDER, and pin the law's numbers: the read behind at tick 3 vs 19, the re-arm, the write-only posture, the disarmed trigger). Cite: @ fc6fa5415c2d7f54e3da90caf8ba8a2b006e52ef — SlotFile.hpp:209-213+276-278+326-333, SlotFile.cpp:427-428+854-866+894, SsdReaders.hpp:15; @ be309949048075c13379f1d3b4a2942cfde9bb49 — SlotFile.cpp:54-56+758-760+787-789+861-876+904, SlotFile.hpp:278+326-331+478-479; @ 47325f9b0c96e9d60b5e1c588c6b750f8820e250 — SsdReaders.hpp:9-10+120-122+144, SsdReaders.cpp:83-93; Batch 270 / Plan 274 / Research 306.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── dispatch: decode planned by CONSTRAINT MODE splits mixed traffic
        // → plan by priority and stage alone; one command carries both
        // modes, masks scoped to the constrained lanes alone ───────────────
        // Source: incoai/splash @ f52d1ad (Apache-2.0),
        // runtime/engine/Scheduler.{hpp,cpp} + runtime/model/{Model.hpp,Runtime.cpp}.
        PerfRule {
            rule_id: "constrained-unconstrained-lanes-one-decode-command".to_string(),
            name: "Decode planned by CONSTRAINT MODE — a grammar lane and a free lane never share a command, the batch plan carries a constrained flag and the runtime refuses a plan that mixes them, so on mixed traffic each command serves half the lanes and the forward's fixed cost is paid twice a round — → plan decode by priority and stage alone: a cycle with any constrained lane goes through the constrained ticket, its draft and target forward run EVERY lane, masks are asked for the cycle's constrained lanes alone and only their selections read masks (a mask response for another lane stays stale, ownsMaskWait), the verify budget prices the cycle as a constrained one — one command per round instead of two".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r##"#![allow(dead_code)]

// ORIGINAL fixture machinery for the riir-refine rust_perf corpus entry
// `constrained-unconstrained-lanes-one-decode-command` (Batch 270, slice D).
// The shape is distilled from incoai/splash (Apache-2.0) commit f52d1ad --
// a serving scheduler that planned decode by constraint mode; this is a
// re-authored deterministic Rust miniature (tick-priced commands, a
// deterministic logits oracle), never splash code verbatim.
//
// THIS FILE IS THE ANTI-PATTERN ARM. The scheduler partitions the ready
// lanes by CONSTRAINT MODE and the runtime refuses a plan that mixes them:
// on mixed traffic each command serves half the lanes, and the forward's
// fixed cost is paid once per command instead of once per round.

/// A decode command's fixed cost (paid per command, whatever it carries).
pub const FORWARD_BASE: u64 = 10;
/// The marginal cost of one lane in a command.
pub const PER_LANE: u64 = 1;
/// The constrained cycle's host dependency: the mask round-trip between the
/// commands of a constrained cycle.
pub const MASK_WAIT: u64 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lane {
    pub id: u32,
    pub constrained: bool,
}

/// Deterministic logits for (lane, step) -- a fixture oracle, not a model.
fn logits(lane_id: u32, step: u32) -> [i32; 4] {
    let s = ((lane_id as i64 * 7 + step as i64 * 13) % 97) as i32;
    [s, s + 5, s - 3, s + 1]
}

/// A constrained lane's mask admits every token but the stop (3); a free
/// lane's selection reads no mask.
fn admits(lane: &Lane, tok: u32) -> bool {
    !lane.constrained || tok != 3
}

/// Greedy pick over the admitted tokens; ties keep the first (deterministic).
fn pick(lane: &Lane, step: u32) -> u32 {
    let lg = logits(lane.id, step);
    let mut best = 0u32;
    for t in 1..4u32 {
        if admits(lane, t) && lg[t as usize] > lg[best as usize] {
            best = t;
        }
    }
    best
}

#[derive(Debug, PartialEq, Eq)]
pub struct BurstReport {
    /// (lane id, token) in input-lane order, rounds concatenated.
    pub tokens: Vec<(u32, u32)>,
    pub commands: u32,
    pub ticks: u64,
}

/// The OLD runtime contract: a batch plan carries a constrained flag, and a
/// plan that mixes the modes is refused outright.
pub fn plan_refuses_mixed(lanes: &[Lane]) -> Result<(), String> {
    let has_c = lanes.iter().any(|l| l.constrained);
    let has_f = lanes.iter().any(|l| !l.constrained);
    if has_c && has_f {
        return Err("constrained and unconstrained lanes never share a command".to_string());
    }
    Ok(())
}

/// THIS ARM: one decode command per constraint mode per round. The
/// constrained command carries the host mask dependency; the free command
/// never asks for masks. Mixed traffic therefore pays the forward's fixed
/// cost twice per round, each command serving half the lanes.
pub fn decode_round(lanes: &[Lane], masks_ready: bool, step: u32) -> BurstReport {
    // The scheduler plans BY CONSTRAINT MODE, so the runtime never sees a
    // mixed plan: each command this round carries one mode only.
    let cons: Vec<&Lane> = lanes.iter().filter(|l| l.constrained).collect();
    let free: Vec<&Lane> = lanes.iter().filter(|l| !l.constrained).collect();
    let cons_owned: Vec<Lane> = cons.iter().map(|l| **l).collect();
    let free_owned: Vec<Lane> = free.iter().map(|l| **l).collect();
    debug_assert!(plan_refuses_mixed(&cons_owned).is_ok());
    debug_assert!(plan_refuses_mixed(&free_owned).is_ok());
    let mut commands = 0u32;
    let mut ticks = 0u64;
    let mut tokens = Vec::new();
    if !cons.is_empty() {
        commands += 1;
        ticks += FORWARD_BASE + cons.len() as u64 * PER_LANE;
        if !masks_ready {
            ticks += MASK_WAIT;
        }
    }
    if !free.is_empty() {
        commands += 1;
        ticks += FORWARD_BASE + free.len() as u64 * PER_LANE;
    }
    // Tokens in input-lane order; every lane decodes one token per round.
    for l in lanes {
        tokens.push((l.id, pick(l, step)));
    }
    BurstReport { tokens, commands, ticks }
}

/// A burst of `rounds` decode rounds at one step each.
pub fn decode_burst(lanes: &[Lane], rounds: u32, masks_ready: bool) -> BurstReport {
    let mut out = BurstReport { tokens: Vec::new(), commands: 0, ticks: 0 };
    for step in 0..rounds {
        let round = decode_round(lanes, masks_ready, step);
        out.tokens.extend_from_slice(&round.tokens);
        out.commands += round.commands;
        out.ticks += round.ticks;
    }
    out
}

/// A constrained lane's FIRST token is still selected by a plan of its own:
/// the constrained lanes run one command and wait for their masks; the free
/// lanes never join that wait. (Preserved by the fix; pinned in both arms.)
pub fn select_first_tokens(lanes: &[Lane], masks_ready: bool) -> BurstReport {
    let cons: Vec<&Lane> = lanes.iter().filter(|l| l.constrained).collect();
    let free: Vec<&Lane> = lanes.iter().filter(|l| !l.constrained).collect();
    let mut commands = 0u32;
    let mut ticks = 0u64;
    let mut tokens = Vec::new();
    if !cons.is_empty() {
        commands += 1;
        ticks += FORWARD_BASE + cons.len() as u64 * PER_LANE;
        if !masks_ready {
            ticks += MASK_WAIT;
        }
        for l in &cons {
            tokens.push((l.id, pick(l, 0)));
        }
    }
    if !free.is_empty() {
        commands += 1;
        ticks += FORWARD_BASE + free.len() as u64 * PER_LANE;
        for l in &free {
            tokens.push((l.id, pick(l, 0)));
        }
    }
    BurstReport { tokens, commands, ticks }
}

fn mixed_lanes() -> Vec<Lane> {
    vec![
        Lane { id: 0, constrained: true },
        Lane { id: 1, constrained: true },
        Lane { id: 2, constrained: false },
        Lane { id: 3, constrained: false },
    ]
}

fn token_oracle(lanes: &[Lane], rounds: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for step in 0..rounds {
        for l in lanes {
            out.push((l.id, pick(l, step)));
        }
    }
    out
}

#[test]
fn mixed_traffic_decodes_the_same_tokens_as_the_per_lane_oracle() {
    let lanes = mixed_lanes();
    let burst = decode_burst(&lanes, 8, true);
    assert_eq!(burst.tokens, token_oracle(&lanes, 8));
}

#[test]
fn mixed_traffic_pays_the_forward_base_twice_a_round() {
    let lanes = mixed_lanes();
    let burst = decode_burst(&lanes, 8, true);
    // Two commands a round, each serving half the lanes: 16 commands for the
    // burst; 27 ticks a round = (10 + 2) + (10 + 2).
    assert_eq!(burst.commands, 16);
    assert_eq!(burst.ticks, 8 * (2 * FORWARD_BASE + 4 * PER_LANE));
}

#[test]
fn the_runtime_refuses_a_plan_that_mixes_the_modes() {
    let lanes = mixed_lanes();
    assert!(plan_refuses_mixed(&lanes).is_err());
    assert!(plan_refuses_mixed(&lanes[..2]).is_ok());
    assert!(plan_refuses_mixed(&lanes[2..]).is_ok());
}

#[test]
fn homogeneous_traffic_is_unchanged_one_command_a_round() {
    // Constrained-only: the partition is one group, one command a round --
    // the same shape the fix keeps (its ticks are priced by the oracle below
    // in the fixed arm's file).
    let lanes = vec![
        Lane { id: 0, constrained: true },
        Lane { id: 1, constrained: true },
    ];
    let burst = decode_burst(&lanes, 8, true);
    assert_eq!(burst.commands, 8);
    assert_eq!(burst.ticks, 8 * (FORWARD_BASE + 2 * PER_LANE));
    assert_eq!(burst.tokens, token_oracle(&lanes, 8));

    let free = vec![
        Lane { id: 2, constrained: false },
        Lane { id: 3, constrained: false },
    ];
    let burst = decode_burst(&free, 8, true);
    assert_eq!(burst.commands, 8);
    assert_eq!(burst.ticks, 8 * (FORWARD_BASE + 2 * PER_LANE));
}

#[test]
fn a_single_lane_is_unchanged() {
    let lanes = vec![Lane { id: 0, constrained: false }];
    let burst = decode_burst(&lanes, 8, true);
    assert_eq!(burst.commands, 8);
    assert_eq!(burst.ticks, 8 * (FORWARD_BASE + PER_LANE));
}

#[test]
fn a_constrained_lanes_first_token_is_its_own_plan() {
    let lanes = mixed_lanes();
    let first = select_first_tokens(&lanes, false);
    // The constrained lanes' plan waits for their masks alone; the free
    // lanes' tokens never join that wait.
    assert_eq!(first.commands, 2);
    assert_eq!(first.ticks, FORWARD_BASE + 2 * PER_LANE + MASK_WAIT + FORWARD_BASE + 2 * PER_LANE);
    assert_eq!(first.tokens, vec![(0, pick(&lanes[0], 0)), (1, pick(&lanes[1], 0)), (2, pick(&lanes[2], 0)), (3, pick(&lanes[3], 0))]);
}
"##.to_string(),
            fixed_code: r##"#![allow(dead_code)]

// ORIGINAL fixture machinery for the riir-refine rust_perf corpus entry
// `constrained-unconstrained-lanes-one-decode-command` (Batch 270, slice D).
// The shape is distilled from incoai/splash (Apache-2.0) commit f52d1ad --
// a serving scheduler that stopped planning decode by constraint mode; this
// is a re-authored deterministic Rust miniature (tick-priced commands, a
// deterministic logits oracle), never splash code verbatim.
//
// THIS FILE IS THE LAW ARM. The scheduler plans decode by priority and stage
// alone -- the plan carries no constrained flag: a cycle with any constrained
// lane goes through the constrained ticket, its forward runs EVERY lane, it
// asks for the masks of its constrained lanes alone, and only their
// selections read masks (a mask response for another lane stays stale). One
// command per round instead of two on mixed traffic.

/// A decode command's fixed cost (paid per command, whatever it carries).
pub const FORWARD_BASE: u64 = 10;
/// The marginal cost of one lane in a command.
pub const PER_LANE: u64 = 1;
/// The constrained cycle's host dependency: the mask round-trip, asked for
/// the cycle's constrained lanes alone.
pub const MASK_WAIT: u64 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lane {
    pub id: u32,
    pub constrained: bool,
}

/// Deterministic logits for (lane, step) -- a fixture oracle, not a model.
fn logits(lane_id: u32, step: u32) -> [i32; 4] {
    let s = ((lane_id as i64 * 7 + step as i64 * 13) % 97) as i32;
    [s, s + 5, s - 3, s + 1]
}

/// A constrained lane's mask admits every token but the stop (3); a free
/// lane's selection reads no mask -- not even a stale one left in its slot.
fn admits(lane: &Lane, tok: u32) -> bool {
    !lane.constrained || tok != 3
}

/// Greedy pick over the admitted tokens; ties keep the first (deterministic).
fn pick(lane: &Lane, step: u32) -> u32 {
    let lg = logits(lane.id, step);
    let mut best = 0u32;
    for t in 1..4u32 {
        if admits(lane, t) && lg[t as usize] > lg[best as usize] {
            best = t;
        }
    }
    best
}

#[derive(Debug, PartialEq, Eq)]
pub struct BurstReport {
    /// (lane id, token) in input-lane order, rounds concatenated.
    pub tokens: Vec<(u32, u32)>,
    pub commands: u32,
    pub ticks: u64,
}

/// THIS ARM: one command per round over every ready lane. The cycle waits for
/// the masks of its CONSTRAINED lanes alone (ownsMaskWait: a mask response
/// for a free lane is never waited on, and free selections never read one).
pub fn decode_round(lanes: &[Lane], masks_ready: bool, step: u32) -> BurstReport {
    let any_constrained = lanes.iter().any(|l| l.constrained);
    let mut ticks = FORWARD_BASE + lanes.len() as u64 * PER_LANE;
    if any_constrained && !masks_ready {
        ticks += MASK_WAIT;
    }
    let tokens = lanes.iter().map(|l| (l.id, pick(l, step))).collect();
    let commands = u32::from(!lanes.is_empty());
    BurstReport { tokens, commands, ticks }
}

/// A burst of `rounds` decode rounds at one step each.
pub fn decode_burst(lanes: &[Lane], rounds: u32, masks_ready: bool) -> BurstReport {
    let mut out = BurstReport { tokens: Vec::new(), commands: 0, ticks: 0 };
    for step in 0..rounds {
        let round = decode_round(lanes, masks_ready, step);
        out.tokens.extend_from_slice(&round.tokens);
        out.commands += round.commands;
        out.ticks += round.ticks;
    }
    out
}

/// A constrained lane's FIRST token is still selected by a plan of its own:
/// the constrained lanes run one command and wait for their masks; the free
/// lanes never join that wait. (Preserved by the fix; pinned in both arms.)
pub fn select_first_tokens(lanes: &[Lane], masks_ready: bool) -> BurstReport {
    let cons: Vec<Lane> = lanes.iter().filter(|l| l.constrained).copied().collect();
    let free: Vec<Lane> = lanes.iter().filter(|l| !l.constrained).copied().collect();
    let mut commands = 0u32;
    let mut ticks = 0u64;
    let mut tokens = Vec::new();
    if !cons.is_empty() {
        commands += 1;
        ticks += FORWARD_BASE + cons.len() as u64 * PER_LANE;
        if !masks_ready {
            ticks += MASK_WAIT;
        }
    }
    if !free.is_empty() {
        commands += 1;
        ticks += FORWARD_BASE + free.len() as u64 * PER_LANE;
    }
    for l in lanes {
        tokens.push((l.id, pick(l, 0)));
    }
    BurstReport { tokens, commands, ticks }
}

fn mixed_lanes() -> Vec<Lane> {
    vec![
        Lane { id: 0, constrained: true },
        Lane { id: 1, constrained: true },
        Lane { id: 2, constrained: false },
        Lane { id: 3, constrained: false },
    ]
}

fn token_oracle(lanes: &[Lane], rounds: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for step in 0..rounds {
        for l in lanes {
            out.push((l.id, pick(l, step)));
        }
    }
    out
}

/// The anti-pattern's pricing, embedded as the comparison oracle (its own
/// file carries the full form): one command per constraint mode per round.
fn decode_buggy_oracle(lanes: &[Lane], rounds: u32, masks_ready: bool) -> BurstReport {
    let cons: Vec<Lane> = lanes.iter().filter(|l| l.constrained).copied().collect();
    let free: Vec<Lane> = lanes.iter().filter(|l| !l.constrained).copied().collect();
    let mut out = BurstReport { tokens: Vec::new(), commands: 0, ticks: 0 };
    for _ in 0..rounds {
        let mut commands = 0u32;
        let mut ticks = 0u64;
        if !cons.is_empty() {
            commands += 1;
            ticks += FORWARD_BASE + cons.len() as u64 * PER_LANE;
            if !masks_ready {
                ticks += MASK_WAIT;
            }
        }
        if !free.is_empty() {
            commands += 1;
            ticks += FORWARD_BASE + free.len() as u64 * PER_LANE;
        }
        out.commands += commands;
        out.ticks += ticks;
    }
    out.tokens = token_oracle(lanes, rounds);
    out
}

#[test]
fn mixed_traffic_decodes_the_same_tokens_as_the_per_lane_oracle() {
    let lanes = mixed_lanes();
    let burst = decode_burst(&lanes, 8, true);
    assert_eq!(burst.tokens, token_oracle(&lanes, 8));
    assert_eq!(burst.tokens, decode_buggy_oracle(&lanes, 8, true).tokens);
}

#[test]
fn mixed_traffic_runs_one_command_a_round_where_the_partition_ran_two() {
    let lanes = mixed_lanes();
    let fixed = decode_burst(&lanes, 8, true);
    let buggy = decode_buggy_oracle(&lanes, 8, true);
    assert_eq!(fixed.commands, 8, "one command per round");
    assert_eq!(buggy.commands, 16, "two commands a round, each serving half the lanes");
    assert_eq!(fixed.ticks, 8 * (FORWARD_BASE + 4 * PER_LANE));
    assert!(fixed.ticks < buggy.ticks, "the mixed regime strictly wins");
}

#[test]
fn homogeneous_traffic_is_unchanged() {
    // Constrained-only: the fix's cycle is exactly the old constrained
    // command; free-only: exactly the old free command. No gain without
    // mixing -- the plan shapes and ticks are identical.
    let cons = vec![
        Lane { id: 0, constrained: true },
        Lane { id: 1, constrained: true },
    ];
    let fixed = decode_burst(&cons, 8, true);
    let buggy = decode_buggy_oracle(&cons, 8, true);
    assert_eq!(fixed.commands, 8);
    assert_eq!(fixed.ticks, buggy.ticks);
    assert_eq!(fixed.tokens, token_oracle(&cons, 8));

    let free = vec![
        Lane { id: 2, constrained: false },
        Lane { id: 3, constrained: false },
    ];
    let fixed = decode_burst(&free, 8, true);
    let buggy = decode_buggy_oracle(&free, 8, true);
    assert_eq!(fixed.commands, 8);
    assert_eq!(fixed.ticks, buggy.ticks);
}

#[test]
fn a_single_lane_is_unchanged() {
    let lanes = vec![Lane { id: 0, constrained: false }];
    let fixed = decode_burst(&lanes, 8, true);
    assert_eq!(fixed.commands, 8);
    assert_eq!(fixed.ticks, 8 * (FORWARD_BASE + PER_LANE));
}

#[test]
fn a_cycle_waits_once_for_its_constrained_lanes_masks_alone() {
    let lanes = mixed_lanes();
    let round = decode_round(&lanes, false, 0);
    // One mask wait per cycle, for the constrained lanes alone -- never per
    // lane, never for a free lane.
    assert_eq!(round.ticks, FORWARD_BASE + 4 * PER_LANE + MASK_WAIT);
    assert_eq!(round.commands, 1);

    // A free lane without a mask ready holds nothing up: no constrained lane,
    // no wait.
    let free_only = vec![Lane { id: 2, constrained: false }];
    let round = decode_round(&free_only, false, 0);
    assert_eq!(round.ticks, FORWARD_BASE + PER_LANE);
}

#[test]
fn a_free_lanes_selection_never_reads_a_mask() {
    // A stale mask sits in a free lane's slot while the constrained lanes'
    // masks are still out: the cycle completes, and the free lane's pick is
    // the maskless oracle pick.
    let lanes = mixed_lanes();
    let round = decode_round(&lanes, false, 0);
    let free_pick = round.tokens.iter().find(|(id, _)| *id == 2).unwrap().1;
    assert_eq!(free_pick, pick(&lanes[2], 0));
    // And under a mask that would have changed it (token 3 is the argmax for
    // some steps), a free lane still picks it.
    let free_lane = Lane { id: 2, constrained: false };
    let lg = logits(2, 1);
    let argmax = (0..4u32).max_by_key(|&t| lg[t as usize]).unwrap();
    assert_eq!(pick(&free_lane, 1), argmax);
}

#[test]
fn a_constrained_lanes_first_token_is_its_own_plan() {
    let lanes = mixed_lanes();
    let first = select_first_tokens(&lanes, false);
    // The constrained lanes' plan waits for their masks alone; the free
    // lanes' tokens never join that wait.
    assert_eq!(first.commands, 2);
    assert_eq!(first.ticks, FORWARD_BASE + 2 * PER_LANE + MASK_WAIT + FORWARD_BASE + 2 * PER_LANE);
    assert_eq!(first.tokens, vec![(0, pick(&lanes[0], 0)), (1, pick(&lanes[1], 0)), (2, pick(&lanes[2], 0)), (3, pick(&lanes[3], 0))]);
}
"##.to_string(),
            context_notes: Some("Grade B+ — NotBenchable ON THIS BOX (the win is an end-to-end serving dispatch figure measured over HTTP on MLX/M5 Max, ABBA against the parent build; a fixture microbench would measure the fixture's own tick model — source-reported figures are PROVENANCE, never bars). SOURCE (incoai/splash @ f52d1ad5123afb94d97a8b93b70faf8b76b1db17, Apache-2.0 — runtime/engine/Scheduler.{hpp,cpp} + runtime/model/{Model.hpp,Runtime.cpp} at the pin; every quote verbatim this batch). THE ANTI-PATTERN (the deleted lines, verbatim from the diff): Model.hpp's BatchPlan carried the flag and its law as a comment — 'A decode's lanes exchange token masks with the host; constrained and unconstrained lanes never share a command.' — and Scheduler.cpp:nextDecode planned 'plan.constrained = ready.front()->spec.constrained;' filtering 'request->spec.constrained != plan.constrained'; the commit: 'A lane under a grammar (a JSON schema, a tool call's framing) and a free lane never shared a decode command: the scheduler planned decode by constraint mode, and the runtime refused a plan that mixed them. Two agents, one under a grammar and one not, took turns, each command serving half the lanes, although the sampling kernels apply each lane's mask by its own policy and the constrained cycle's three commands run whatever lanes it holds.' THE LAW (f52d1ad): 'The scheduler now plans decode by priority and stage alone, and the batch plan has no constrained flag: the runtime sends a cycle with any constrained lane through the constrained ticket.' — Scheduler.cpp:496-503 the constraint filter deleted (priority + decodeStage only); Scheduler.hpp:75-78 'Decode takes the ready lanes of one priority and stage, constrained or not, without waiting for more'; Runtime.cpp:2282-2288 the ticket doc 'A DFlash cycle with constrained lanes has one host dependency between three Metal commands: a constrained lane's draft proposals define its grammar simulation, while the target forward is independent of the resulting masks. The batch's unconstrained lanes run the same commands and ask for no mask, which their selections never read.'; the mask scoping guards Runtime.cpp:2312-2316 + :2369-2373 ('if (!constrained(lane)) continue;' — only constrained lanes get mask requests and read masks at selection); the readiness conjunct Runtime.cpp:2356 'masksReady = masksReady && (!constrained(lane) || abandoned_[lane] || !lanes_[lane].request->maskWords.empty());' — a free lane's missing or stale mask holds nothing up (ownsMaskWait, Runtime.cpp:2389); the budget Runtime.cpp:2204-2209 verifyBudget->observe(..., constrained, ...) prices the cycle as a constrained one; and 'A constrained lane's first token is still selected by a plan of its own.'. MEASURED (source-reported, f52d1ad message, two constrained + two free requests at once): 'Qwen3.6-35B-A3B MLX 4-bit: 349 to 457 tok/s (-23.7% time per token, noise 0.3%) ... 320 decode commands a burst to 175, B4 in place of B2.'; 'Qwen3.8-27B MLX 4-bit: 138 to 210 tok/s (-34.0%, noise 2.9%)'. THE ANTI-REGIME (the source prices it honestly): 'Qwen3.8-Flash-Next UD-IQ3_XXS ... 22.6 to 23.0 tok/s (-1.6%, noise 2.8% ...), 130 decode commands a burst in place of 225. A cycle there reads the experts its rows route, which grow with its rows, so a shared command saves little.' — and single requests are flat: '-0.03% .. -0.05% ... within noise' (one lane, no mixing, no gain — the fixture pins homogeneous and single-lane traffic unchanged). REGIME: a decode server batching lanes of mixed constraint modes at one priority+stage; the forward's fixed cost is material per command; mask policy is per-lane ('the sampling kernels apply each lane's mask by its own policy'), so one command can carry both modes and only some lanes need masks. ANTI-REGIME: expert-routed cycles whose cost grows with rows (the Flash-Next cell); homogeneous traffic (nothing to share); a constrained lane's FIRST token (its own plan — pinned in the fixture); runtimes whose mask path cannot scope per-lane staleness (the ownsMaskWait discipline is the precondition: only constrained selections read masks, a mask response for another lane stays stale). DISTINCT-FROM dispatch-pass-fusion-skips-empty-handlers (entries_06, complexity): that fuses a visitor pipeline so EMPTY handlers are never dispatched (per-node-kind registration tables); THIS fuses DECODE COMMANDS across a constraint-mode partition (batch composition, not handler emptiness). DISTINCT-FROM deferred-command-flush-hoist-out-of-batch-loop (B174, concurrency): that hoists a barrier out of a loop (fixed sync cost amortization); THIS removes a PLAN PARTITION (which lanes may share a command) — the shared cost is the forward's, not a barrier's. DISTINCT-FROM draft-cost-timed-at-served-widths-not-extrapolated (B257, dispatch): cost-table measurement discipline — unrelated axis. DISTINCT-FROM mir-destprop-branch-site-merge (B99) + dispatch-handler-pair-hoist (B101): codegen-level dispatch-site merging/hoisting of fn pointers — instruction layer; THIS is scheduler-layer batch composition (lanes into commands), no codegen claim. KINSHIP: grammar-window-draft-prune-path-local-mask (B196) is the corpus's other constrained-decoding rule — it prunes the ROWS of one verify window by grammar admissibility before the forward; THIS decides which LANES may share a command at the scheduler; they compose (its windows ride this rule's mixed cycles). NOVEL: grep 'constrained|constraint mode|one command|batch plan|ownsMaskWait|mask wait' over both corpora: zero owner of constraint-mode planning or mixed-lane command fusion. Suggestion-only (a parse-only proposer cannot see scheduler plan composition or mask-wait scoping). Fixtures ORIGINAL machinery, never splash code (.raw/b270/fix_lane_dispatch_*; a deterministic logits oracle with a per-lane mask policy — both arms cross-assert identical token streams, pin the command counts (8 vs 16 for an 8-round mixed burst) and the tick model (17 vs 27 a round at FORWARD_BASE 10 / PER_LANE 1 / MASK_WAIT 3), and pin the regime edges: homogeneous and single-lane unchanged, the refusal contract, ownsMaskWait, the first-token plan). Cite: @ f52d1ad5123afb94d97a8b93b70faf8b76b1db17 — runtime/engine/Scheduler.cpp:496-503, Scheduler.hpp:75-78, runtime/model/Model.hpp (BatchPlan: the deleted flag + comment), runtime/model/Runtime.cpp:2204-2209+2282-2288+2312-2316+2356+2369-2373+2389; Batch 270 / Plan 274 / Research 306.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
