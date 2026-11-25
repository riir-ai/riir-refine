// Batch 414 (nominal — re-derives at landing from the score_history tail) —
// the relate storage/durability transcription (Plan 388 / Research 422):
// relatedb/relate @ `144a9c14de697aabb12dc2ab64460a4c9cab69bc` (Apache-2.0,
// the RE-PINNED tip — the verdict pin `c20fc310f…` moved one docs-wide
// commit (#742, ADR frontmatter + wiki-link conversion + scripts/skills
// additions) that ALSO refactored commit.rs/checkpoint.rs; every quote below
// re-verified at the NEW pin 2026-11-25, line cites corrected for the moved
// files — scan.rs's `replace_torn_tail` :96→:105, the tolerance taxonomy
// :258/:305/:387→:267/:314+, the `synced` latch :164→:168, coalesce.rs
// :103→:108). Issues-only upstream (no PR lane) — decision content only.
//
// rust_perf rules 406 → 412 (six rules + three count-neutral faces on their
// owners): the WAL/durability-integrity sub-family's third through eighth
// members (the corpus had TWO: entries_08 barrier batching + entries_81
// hydradb watermark; this batch seeds it to eight):
// - `membership-set-inline-words-tier-promote-at-seventeenth` (the
//   BENCHABLE lead, layout) — ≤16 keys inline in `[u64; 16]`, branch-free
//   16-lane mask probe, promote ONCE at the 17th to a presized-32 table,
//   hasher pinned at construction; the sparse-domain carve the
//   dense-id-slot-array rule must never touch.
// - `torn-tail-trim-atomic-replacement-never-truncate-in-place` (io) —
//   recovery trims by copying the intact prefix into an atomic replacement;
//   an in-place truncate can lose committed records if read/sync fails.
// - `tolerated-tail-taxonomy-active-only-integrity-tolerance` (io) — the
//   four-regime decision table: incomplete-EOF tolerated on ANY segment,
//   CRC-failed final frame ACTIVE only, same failure on CLOSED = bitrot
//   fatal, CRC-valid-but-undecodable fatal EVERYWHERE (never trim).
// - `group-flush-barrier-latch-skip-synced-segment-barrier` (io) — the
//   `synced` latch: barrier only when bytes were appended since the last
//   sync; always barrier the NEW segment's magic on a full rotate, never on
//   a relaxed one. The redundant-barrier-ELIMINATION axis beside
//   entries_08's barrier-BATCHING axis.
// - `hasher-choice-by-key-provenance-not-habit` (complexity) — keyless Fx
//   for engine-generated keys, process-wide seeded SipHash for
//   client-influenceable keys; the seeded-FxHash REJECTION (Fx compresses
//   through fixed constants BEFORE the seed touches the input) is the
//   technical reason the fxhashmap proposer must never gain a seeded arm.
// - `checkpoint-coalesce-rewrite-units-anti-oscillation-manifest-bound` (io)
//   — checkpoints rewrite id ranges as UNITS (a seed absorbs neighbors while
//   the width/manifest bounds hold); the anti-oscillation bound keeps
//   apart-files that miss one file together, else every such checkpoint
//   re-encodes whole units.
//
// FACES (count-neutral riders in their owners' context_notes):
// - manifest-proof-cleanup face on entries_75 ajika
//   `pack-store-gc-decides-by-recovered-index` — deletion proves by a
//   READABLE manifest, never by attempt order.
// - exclusive-temp/close-before-rename face on entries_56
//   `exclusive-mkdir-temp-resource-reservation` + the LocalSR
//   verify-then-atomic-install row — 128-bit OS-random temp, the write
//   handle scoped closed BEFORE the rename (Windows os error 5), cleanup
//   touches only the owned temp never the destination.
// - seeded-Fx-rejected annotation on the fxhashmap proposer — the proposer's
//   docs gain the compression-before-seed reason.
//
// Dedup at landing HEAD (T1 re-run this session, 3+ vocabularies per row):
// all six axes unowned; entries_72 `dense-id-domain-hashmap-to-slot-array`
// and entries_08 `deferred-command-flush-hoist-out-of-batch-loop` confirmed
// present as the two planned in-rule adjudication counterparts; the
// encode-once grep hits are bit-plane transposition encoding (entries_05) —
// a different class; the "poisoned" hits are calibration-cache poisoning
// (entries_19) — a different medium; entries_41's "seventeenth" is the
// splash governor's page fraction — an unrelated axis.
// NOTHING INVENTED — every quote read at the new pin this session.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 414 (nominal) — the relate storage/durability `rust_perf` rules.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b414_rules() -> Vec<PerfRule> {
    vec![
        r1_membership_set_inline_words(),
        r2_torn_tail_trim_atomic_replacement(),
        r3_tolerated_tail_taxonomy(),
        r4_group_flush_barrier_latch(),
        r5_hasher_choice_by_key_provenance(),
        r6_checkpoint_coalesce_rewrite_units(),
    ]
}

// ---------------------------------------------------------------------------
// Rule 1 — membership-set-inline-words-tier-promote-at-seventeenth (the lead)
// ---------------------------------------------------------------------------
#[must_use]
fn r1_membership_set_inline_words() -> PerfRule {
    let name = "a membership set whose key class stays small (≤16 words) pays a heap allocation + a hash table's probe chain for ids that fit in fixed registers → hold up to 16 u64 words inline in `[u64; 16]` probed by a fixed-length branch-free 16-lane mask compare (no heap, no hash; unused lanes masked, never sentinel-compared, so the loop vectorises), and promote ONCE at the 17th distinct word to ONE presized table (2^5 = 32) with the hasher chosen at construction — promotion is upward-only, moves at most the 16 held words, and never demotes".to_string();
    let notes = "Grade A, BENCHABLE lead — MEASURED ParetoDominant 3/0/0 two-run stable (the perf-bench validator, 2026-11-25, m3 Mac15,8 / M3 Max 16-core / 64 GiB / macOS 27.2, PROVENANCE: power=AC Power load=4.84 swap=1294.50M powermode=2(high) ane=ok — the preflight quoted verbatim; allocs 0/0 bytes 0/0 at every size, G-det stable across 3 repeats ×2 runs: both arms are probe-path alloc-free by construction, the win is the probe CHAIN — one hash + bucket walk vs ≤16 integer compares). SOURCE (relatedb/relate @ 144a9c14de697aabb12dc2ab64460a4c9cab69bc, Apache-2.0, re-pinned 2026-11-25 — the verdict pin c20fc310 moved one docs+refactor commit #742; every quote re-verified at the new pin; the file was byte-stable, only its trailing line count moved): relate/relate-executor/src/tiered_set.rs:1-20 module doc verbatim — 'one key class's words in the cheapest representation the count admits… * **T1** — up to `2^4 = 16` words inline in `[u64; 16]`, probed by a fixed-length, branch-free 16-lane mask compare: no heap, no hash; * **T2** — on the 17th distinct word, one presized hash table (`2^5 = 32`), the hasher chosen once at construction'. THE BRANCH-FREE PROBE IS THE DECISION CONTENT (tiered_set.rs:262-273, verbatim): 'all 16 lanes compared into a mask, then masked to the held length — fixed trip count, no early exit, so the loop vectorises. Unused lanes are masked, not sentinel-compared, because every `u64` is a valid payload.' The masked-not-sentinel clause is load-bearing: a sentinel 0 lane (the obvious shape) silently drops the real word 0 and every masked comparison re-learns nothing; the mask over the held length keeps the trip count constant so the compiler emits one unrolled vectorised compare. PROMOTION LAWS: upward only ('Promotion is upward only, moves at most the 16 held words'), the charge is the delta of the representation that took the word and a refused charge leaves the tier untouched. THE HASH-ONCE LAW: the hasher is chosen at construction by KEY PROVENANCE (see hasher-choice-by-key-provenance-not-habit, this batch — the two rules compose: the tier decides REPRESENTATION by count, the provenance seam decides HASHER by who can choose the key). ADJUDICATION (dedup at landing HEAD, vocabularies membership+inline|words-tier|seventeenth|bitset+promote): dense-id-domain-hashmap-to-slot-array (entries_72) is the DENSE-DOMAIN counterpart — it owns ids that ARE their own index over [0, N); THIS owns the small-CARDINALITY MIXED-domain membership set (arbitrary u64 words, no index structure) and its own fixture pins the sparse-id case where the slot array is the memory blowup — compose on a registry (dense ids slot-array, arbitrary small word-sets tiered-words), never merge; minimal-perfect-hash-static-lookup (entries_33) owns FROZEN keysets (its anti-regime names the mutable keyset THIS covers); single-entry-last-key-lookup-cache-hash-avoidance (entries_66) owns a one-slot MEMO in front of a hot TABLE — THIS owns the set itself below table scale; byte-class-equivalence-table-shrink (entries_30) owns a CLASS TABLE, not membership. REGIME: per-entity tag/word sets (monster trait sets, per-key grammar vocabularies, per-session id sets), schema distinct-word tables, any membership probe whose observed count sits ≤16 for the set's life. ANTI-REGIMES: counts that grow past 32 immediately (build the table directly — the promotion path is wasted work); keys needing association with VALUES (this is a SET; a map needs the table from the start); counts oscillating around the boundary every cycle (the upward-only law makes the promotion permanent — an oscillating count is a different workload, measure before adopting). Fixtures b414_perf F1: the sparse-domain arm (256 live words over a 2^32 domain — the slot array would be a blowup, the tiered set holds), the promotion arm (16th insert stays inline; the 17th promotes once, moves ≤16 words, never demotes), the word-0 masked-probe arm (word 0 held and found — the sentinel-compare falsifier), the branch-free probe counts a FIXED trip count. Plan 388 / Research 422. Batch 414.".to_string();
    PerfRule {
        rule_id: "membership-set-inline-words-tier-promote-at-seventeenth".to_string(),
        name,
        category: "layout".to_string(),
        buggy_code: r#"use std::collections::HashSet;

// NAIVE: every per-entity word set is a heap HashSet even when the
// entity's whole vocabulary is three words — allocation + hash + probe
// chain per membership question, per entity, per tick.
#[derive(Default)]
struct WordSet {
    words: HashSet<u64>,
}

impl WordSet {
    fn admit(&mut self, word: u64) {
        self.words.insert(word);
    }

    fn holds(&self, word: u64) -> bool {
        self.words.contains(&word)
    }
}

fn census(sets: &[WordSet]) -> usize {
    sets.iter().filter(|s| s.holds(7)).count()
}"#
        .to_string(),
        fixed_code: r#"// FIXED: the tiered word set — 16 words inline, a branch-free
// 16-lane mask probe (fixed trip count, unused lanes MASKED not
// sentinel-compared), one upward promotion at the 17th to a
// presized-32 table, hasher fixed at construction, never a demotion.
const INLINE_WORDS: usize = 16;
const TABLE_WORDS: usize = 32;

#[derive(Default)]
enum Tier {
    #[default]
    Empty,
    Inline {
        slots: [u64; INLINE_WORDS],
        len: u8,
    },
    Table {
        words: std::collections::HashSet<u64, std::hash::BuildHasherDefault<WordsHasher>>,
    },
}

struct WordsHasher;

impl std::hash::Hasher for WordsHasher {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, _bytes: &[u8]) {}
}

#[derive(Default)]
struct WordSet {
    tier: Tier,
}

impl WordSet {
    fn admit(&mut self, word: u64) {
        match &mut self.tier {
            Tier::Empty => {
                let mut slots = [0u64; INLINE_WORDS];
                slots[0] = word;
                self.tier = Tier::Inline { slots, len: 1 };
            }
            Tier::Inline { slots, len } => {
                if inline_contains(slots, *len, word) {
                    return;
                }
                if usize::from(*len) < INLINE_WORDS {
                    slots[usize::from(*len)] = word;
                    *len += 1;
                } else {
                    // the ONE promotion: upward only, moves the held
                    // words, never comes back down
                    let mut words = std::collections::HashSet::with_capacity(TABLE_WORDS);
                    words.extend(slots.iter().copied());
                    words.insert(word);
                    self.tier = Tier::Table { words };
                }
            }
            Tier::Table { words } => {
                words.insert(word);
            }
        }
    }

    fn holds(&self, word: u64) -> bool {
        match &self.tier {
            Tier::Empty => false,
            Tier::Inline { slots, len } => inline_contains(slots, *len, word),
            Tier::Table { words } => words.contains(&word),
        }
    }
}

/// All 16 lanes compared into a mask, then masked to the held length —
/// fixed trip count, no early exit, so the loop vectorises. Unused
/// lanes are masked, NOT sentinel-compared: every u64 is a valid
/// payload, word 0 included.
fn inline_contains(slots: &[u64; INLINE_WORDS], len: u8, word: u64) -> bool {
    let mut mask: u32 = 0;
    for (lane, slot) in slots.iter().enumerate() {
        mask |= u32::from(*slot == word) << lane;
    }
    mask & ((1u32 << len) - 1) != 0
}

fn census(sets: &[WordSet]) -> usize {
    sets.iter().filter(|s| s.holds(7)).count()
}"#
        .to_string(),
        context_notes: Some(notes),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: Some(membership_inline_words_bench_spec()),
    }
}

/// R1's bench: per-entity membership probes over a `HashSet` of u64 words
/// vs the inline `[u64; 16]` mask probe at three set sizes (4 / 16 inline
/// ceiling / 17-past-the-boundary). The build side stays OUT of the timed
/// region (built in `make_input`), so the alloc axis prices only the
/// steady-state probe path: the `HashSet` arm allocates nothing per probe,
/// the inline arm nothing either — the win is the probe CHAIN (one hash +
/// bucket walk vs 4-16 integer compares), and G-det holds because neither
/// arm allocates on the probe path.
#[cfg(feature = "rust_perf_bench")]
fn membership_inline_words_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![4, 16, 17],
        harness: r#"
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, DefaultHasher, Hasher};

#[derive(Default)]
struct WordsHasher;

impl Hasher for WordsHasher {
    fn finish(&self) -> u64 { 0 }
    fn write(&mut self, bytes: &[u8]) { let mut h = DefaultHasher::new(); h.write(bytes); }
}

pub struct Input {
    sets_hash: Vec<HashSet<u64>>,
    sets_inline: Vec<InlineSet>,
    probe: u64,
    expected: usize,
}

const INLINE_WORDS: usize = 16;

#[derive(Clone)]
struct InlineSet {
    slots: [u64; INLINE_WORDS],
    len: u8,
    table: Option<HashSet<u64>>,
}

impl InlineSet {
    fn holds(&self, word: u64) -> bool {
        match &self.table {
            Some(t) => t.contains(&word),
            None => {
                let mut mask: u32 = 0;
                for (lane, slot) in self.slots.iter().enumerate() {
                    mask |= u32::from(*slot == word) << lane;
                }
                mask & ((1u32 << self.len) - 1) != 0
            }
        }
    }
}

pub fn make_input(n: usize) -> Input {
    let mut sets_hash = Vec::with_capacity(n);
    let mut sets_inline = Vec::with_capacity(n);
    let mut expected = 0usize;
    for i in 0..n {
        let count = (i % 17) + 1; // 1..=17 words per set — exercises inline AND promoted
        let mut hs = HashSet::with_capacity(32);
        let mut slots = [0u64; INLINE_WORDS];
        let mut overflow: HashSet<u64> = HashSet::new();
        for w in 0..count as u64 {
            let word = (w.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64)) | 1;
            hs.insert(word);
            if (w as usize) < INLINE_WORDS {
                slots[w as usize] = word;
            } else {
                overflow.insert(word);
            }
        }
        if (count as usize) > INLINE_WORDS {
            let t: HashSet<u64> = slots.iter().copied().chain(overflow).collect();
            sets_inline.push(InlineSet { slots: [0; INLINE_WORDS], len: 0, table: Some(t) });
        } else {
            sets_inline.push(InlineSet { slots, len: count as u8, table: None });
        }
        sets_hash.push(hs);
    }
    let probe = 0x9E37_79B9_7F4A_7C15u64 | 1;
    for (hs, is) in sets_hash.iter().zip(&sets_inline) {
        if hs.contains(&probe) == is.holds(probe) && hs.contains(&probe) {
            expected += 1;
        }
    }
    Input { sets_hash, sets_inline, probe, expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut hits = 0usize;
    for s in &input.sets_hash {
        if s.contains(&input.probe) {
            hits += 1;
        }
    }
    debug_assert_eq!(hits, input.expected);
    std::hint::black_box(hits);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut hits = 0usize;
    for s in &input.sets_inline {
        if s.holds(input.probe) {
            hits += 1;
        }
    }
    debug_assert_eq!(hits, input.expected);
    std::hint::black_box(hits);
}
"#
        .to_string(),
    }
}

// ---------------------------------------------------------------------------
// Rule 2 — torn-tail-trim-atomic-replacement-never-truncate-in-place
// ---------------------------------------------------------------------------
#[must_use]
fn r2_torn_tail_trim_atomic_replacement() -> PerfRule {
    let name = "recovery trimming a torn write tail must COPY the intact prefix into an atomic replacement, never truncate the original in place — a truncate that fails midway (or a reader/sync that fails during the trim) can lose committed records the prefix still held; the replacement write carries the discarded size back from the copying handle so the caller never re-derives it, and a file shorter than the scanned prefix is a typed error, not a wrapped subtraction".to_string();
    let notes = "Grade A, suggestion-only, NotBenchable-by-domain (integrity, not latency — no validator hosts a crash-window). SOURCE (relatedb/relate @ 144a9c14de…, Apache-2.0, re-pinned 2026-11-25; the file moved in commit #742 — cites corrected): relate/relate-storage/src/wal/scan.rs:105-131, the doc VERBATIM: 'Copy the intact prefix into an atomic replacement, returning the number of bytes it discarded. Truncating the original in place could lose committed records if reading or sync fails.' + 'The discarded size is read from the handle that copies the prefix, so the caller never re-derives it: a file shorter than the scanned prefix (a truncation between the scan and the trim) is a typed error, not a wrapped subtraction.' The implementation streams the prefix through the atomic producer (scan.rs:113-131): the source handle opens scoped INSIDE the producer ('the source closes before the helper's rename, including on Windows where open-file replacement fails'), the length re-read from the copying handle, `checked_sub` with the typed message 'torn-tail trim of {path}: physical length {total} is below the scanned prefix {end}'. THE LAW: a trim is a WRITE, and a failed write must leave the readable state intact — in-place truncation destroys the prefix's tail bytes at the truncate call itself; the atomic copy keeps every committed byte until the rename instant. The typed short-file error closes the scan-then-trim race explicitly (a truncation BETWEEN the scan and the trim is a nameable state, never a usize underflow panic). ADJUDICATION (dedup at landing HEAD, vocabularies torn+tail|trim|atomic+replacement|truncate): entries_83's restore-via-cow-clone-not-copy owns RESTORE-time clone ladders (reading a cache artifact back), THIS owns recovery-time tail disposal of a live log — the atomic-replacement SHAPE composes (both refuse in-place mutation of readable state), different trigger, different failure window; entries_56's exclusive-mkdir-temp-resource-reservation owns resource RESERVATION at creation, THIS owns disposal of corrupted state at recovery; entries_81 hydradb wal-watermark-pinned-before-snapshot owns the read-ORDER of watermark vs snapshot, never the trim mechanics — compose. REGIME: any append-only log or journal whose recovery trims a torn tail (WAL segments, write-ahead journals, queue segment files, audit logs with a re-open trim). ANTI-REGIME: logs where the tail is re-derivable from an upstream source (re-fetch beats a trim entirely — no local state to preserve); memory-backed buffers (a Vec truncate is atomic by construction). Fixtures b414_perf F2: the parity arm (a clean tail trims to the same bytes either way), the failure-window arm (an injected read failure after a partial in-place truncate has LOST bytes the atomic copy kept), the typed-short-file arm (a file shorter than the scanned prefix errors by name, never underflows), the tamper arm (the naive wrapped subtraction panics on the short file). Plan 388 / Research 422. Batch 414.".to_string();
    PerfRule {
        rule_id: "torn-tail-trim-atomic-replacement-never-truncate-in-place".to_string(),
        name,
        category: "io".to_string(),
        buggy_code: r#"// NAIVE: trim the torn tail by truncating the log in place — the
// truncate is one call, what could go wrong?
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};

fn trim_torn_tail(path: &str, good_prefix_end: u64) -> std::io::Result<u64> {
    let mut f = OpenOptions::new().read(true).write(true).open(path)?;
    let total = f.metadata()?.len();
    // wraps if the file shrank between the scan and this call —
    // and if the truncate itself is interrupted, the bytes past
    // good_prefix_end are simply GONE
    f.set_len(good_prefix_end)?;
    Ok(total - good_prefix_end)
}

fn committed_bytes_after(path: &str) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}"#
        .to_string(),
        fixed_code: r#"// FIXED: copy the intact prefix into an atomic replacement. The
// source handle is scoped INSIDE the producer (closed before the
// rename — Windows refuses to replace an open file), the discarded
// size is read from the copying handle, and a file shorter than the
// scanned prefix is a typed error, never a wrapped subtraction.
use std::fs;
use std::io::{Read, Write};

fn trim_torn_tail(path: &str, good_prefix_end: u64) -> std::io::Result<u64> {
    let tmp = format!("{path}.trim-tmp");
    let (discarded, total) = {
        let mut source = fs::File::open(path)?;
        let total = source.metadata()?.len();
        let discarded = total
            .checked_sub(good_prefix_end)
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("torn-tail trim of {path}: physical length {total} is below the scanned prefix {good_prefix_end}"),
            ))?;
        let mut dest = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        let mut left = good_prefix_end;
        let mut buf = [0u8; 64 * 1024];
        while left > 0 {
            let size = left.min(buf.len() as u64) as usize;
            source.read_exact(&mut buf[..size])?;
            dest.write_all(&buf[..size])?;
            left -= size as u64;
        }
        dest.sync_all()?;
        (discarded, total)
    }; // source AND dest drop here — provably closed before the rename
    fs::rename(&tmp, path)?;
    Ok(discarded)
}

fn committed_bytes_after(path: &str) -> std::io::Result<Vec<u8>> {
    let mut f = fs::File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}"#
        .to_string(),
        context_notes: Some(notes),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 3 — tolerated-tail-taxonomy-active-only-integrity-tolerance
// ---------------------------------------------------------------------------
#[must_use]
fn r3_tolerated_tail_taxonomy() -> PerfRule {
    let name = "a log reader's tail-failure tolerance is a FOUR-REGIME DECISION TABLE keyed on WHERE the failure sits, never one blanket rule: (1) an incomplete frame at EOF is tolerated on ANY segment (a relaxed rotation that skipped its barrier can land a torn suffix on a just-closed segment too); (2) a complete final frame whose CHECKSUM failed is tolerated on the ACTIVE segment only (a relaxed append); (3) the same checksum failure on a CLOSED segment is bitrot and stays a hard error; (4) a checksum-VALID frame whose payload fails to decode is a hard error in every segment and position — trimming it would silently discard a committed transaction".to_string();
    let notes = "Grade A, suggestion-only, NotBenchable-by-domain (an integrity DECISION table; no validator hosts correctness). SOURCE (relatedb/relate @ 144a9c14de…, Apache-2.0, re-pinned 2026-11-25; scan.rs moved in #742 — cites corrected): relate/relate-storage/src/wal/scan.rs:267-275 (the SegScan.tail_at doc, verbatim): 'Offset where a tolerated tail starts: an incomplete frame at EOF on any segment (a relaxed rotate skips the barrier, so a torn suffix can land on a just-rotated closed segment too), or a complete final frame whose checksum failed on the active segment (a relaxed append). The same checksum failure in a closed segment is bitrot and stays a hard error; a checksum-valid frame whose payload does not decode is never tolerated anywhere (ADR-0114).' And the shared-decoder comment (scan.rs:314-326): 'A complete frame whose checksum matches but whose payload does not decode was written completely by something this binary does not understand — in practice a newer Relate — so it is a hard error in every segment and position (ADR-0114); trimming it would silently drop a committed transaction.' ADR-0114 (docs/adr/0114-wal-recovery-rejects-undecodable-records.md:34-35): 'Trimming it silently discards a committed transaction.' + :74 'Keep trimming undecodable tails. Rejected: it silently drops a committed…' THE TAXONOMY'S AXIS is writer-posture, not byte-shape: WHERE the failure sits (active vs closed segment) tells you what COULD have written it — the active segment's last frame is written by a process that may still be mid-relaxed-append; a CLOSED segment's checksum failure has no innocent writer left (rotation barrier is the last write there), so it is bitrot; an undecodable-but-CRC-valid frame was written COMPLETE by a DIFFERENT binary version — the data is intact, YOUR reader is old, and trimming converts a version-skew into permanent data loss (ADR-0114 pairs it with the store-version check FIRST: 'checks the store version first'). The regime-(1) nuance is load-bearing for relaxed-rotation systems: a crash-tolerant reader that only tolerates torn tails on the active segment FAILS TO BOOT on a relaxed-rotated closed segment — the tolerance follows the barrier policy, not the segment label. ADJUDICATION (dedup at landing HEAD, vocabularies tail+tolerance|integrity+active|checksum+closed|undecodable): vanished-entry-tolerant-eviction-panic-else (entries_69) owns PEER-CAUSED vs SELF-CAUSED classification in a shared-cache eviction (a NotFound from a peer is tolerated, any other error panics) — the CLASSIFICATION-SHAPE is kin (tolerate the class with an innocent cause, fatal the rest) but the medium (cache eviction vs log recovery) and the axes (peer-cause vs segment-posture) are distinct — compose in-rule both directions; entries_75 ajika's pack tolerance laws own GC-by-recovered-index, not read tolerance; sec's unbounded-read family owns INPUT bounds, never log tolerance. REGIME: any segmented log reader with mixed durability postures (barriered + relaxed writes), recovery code choosing trim-vs-fail per failure class, forward-compatibility fences over record formats. ANTI-REGIME: single-segment logs with one durability posture (the taxonomy collapses to two regimes — still worth the incomplete-vs-corrupt split, the active/closed axis is moot); formats with no version fence (an undecodable frame IS bitrot there — regime 4's reasoning needs a version story). Fixtures b414_perf F3: the four-regime table as executable decisions (each class in, typed verdict out), the bitrot arm (closed-segment CRC failure refuses trim), the version-skew arm (undecodable-CRC-valid refuses trim by name), the relaxed-rotate arm (torn suffix on a closed segment tolerated — the blanket-active-only reader fails here). Plan 388 / Research 422. Batch 414.".to_string();
    PerfRule {
        rule_id: "tolerated-tail-taxonomy-active-only-integrity-tolerance".to_string(),
        name,
        category: "io".to_string(),
        buggy_code: r#"// NAIVE: one blanket tolerance — any failure in the last frame of any
// segment is "probably a torn write", trim and move on.
use std::fs;
use std::io::Read;

#[derive(Debug, PartialEq)]
pub enum TailVerdict {
    Trim(u64),
    Fatal(&'static str),
}

fn last_frame_verdict(path: &str, is_last_segment: bool) -> std::io::Result<TailVerdict> {
    let mut buf = Vec::new();
    fs::File::open(path)?.read_to_end(&mut buf)?;
    // reads the LAST frame, ignoring segment posture entirely
    if let Some(frame) = frame_at_end(&buf) {
        if !crc_ok(frame) {
            // a closed segment's bitrot gets trimmed exactly like the
            // active segment's torn append — committed records lost
            return Ok(TailVerdict::Trim(frame.start as u64));
        }
        if decode(frame).is_none() {
            // a NEWER writer's frame gets trimmed as garbage — a
            // version skew turned into silent data loss
            return Ok(TailVerdict::Trim(frame.start as u64));
        }
        let _ = is_last_segment;
    }
    Ok(TailVerdict::Fatal("no frame"))
}

struct Frame {
    start: usize,
    payload: Vec<u8>,
}

fn frame_at_end(buf: &[u8]) -> Option<Frame> {
    (!buf.is_empty()).then(|| Frame { start: 0, payload: buf.to_vec() })
}

fn crc_ok(frame: &Frame) -> bool {
    !frame.payload.is_empty()
}

fn decode(frame: &Frame) -> Option<String> {
    String::from_utf8(frame.payload.clone()).ok()
}"#
        .to_string(),
        fixed_code: r#"// FIXED: the four-regime decision table — tolerance keyed on WHERE
// the failure sits (active vs closed segment) and WHAT failed
// (incomplete vs checksum vs decode), each class its own verdict.
use std::fs;
use std::io::Read;

#[derive(Debug, PartialEq)]
pub enum TailVerdict {
    /// torn suffix — tolerated on ANY segment (a relaxed rotate skips
    /// its barrier, so a just-closed segment can carry one too)
    TrimTorn(u64),
    /// complete last frame, checksum failed, ACTIVE segment only
    /// (a relaxed append was mid-flight)
    TrimRelaxedAppend(u64),
    /// same failure on a CLOSED segment: no innocent writer left — bitrot
    BitrotFatal(&'static str),
    /// CRC-valid but undecodable: written complete by a NEWER binary —
    /// never trimmed, your reader is old
    VersionSkewFatal(&'static str),
}

fn last_frame_verdict(
    path: &str,
    segment_is_active: bool,
) -> std::io::Result<TailVerdict> {
    let mut buf = Vec::new();
    fs::File::open(path)?.read_to_end(&mut buf)?;
    let Some(frame) = frame_at_end(&buf) else {
        return Ok(TailVerdict::BitrotFatal("no frame"));
    };
    if !frame.complete {
        // regime 1: incomplete at EOF — torn, wherever it sits
        return Ok(TailVerdict::TrimTorn(frame.start as u64));
    }
    if !crc_ok(&frame) {
        return match segment_is_active {
            // regime 2: relaxed append mid-flight on the live segment
            true => Ok(TailVerdict::TrimRelaxedAppend(frame.start as u64)),
            // regime 3: nothing innocent writes to a closed segment
            false => Ok(TailVerdict::BitrotFatal("closed-segment checksum failure")),
        };
    }
    if decode(&frame).is_none() {
        // regime 4: intact bytes your reader cannot name — never trim
        return Ok(TailVerdict::VersionSkewFatal("undecodable committed frame"));
    }
    Ok(TailVerdict::BitrotFatal("no failure"))
}

struct Frame {
    start: usize,
    complete: bool,
    payload: Vec<u8>,
}

fn frame_at_end(buf: &[u8]) -> Option<Frame> {
    (!buf.is_empty()).then(|| Frame { start: 0, complete: buf.len() > 8, payload: buf.to_vec() })
}

fn crc_ok(frame: &Frame) -> bool {
    !frame.payload.is_empty()
}

fn decode(frame: &Frame) -> Option<String> {
    String::from_utf8(frame.payload.clone()).ok()
}"#
        .to_string(),
        context_notes: Some(notes),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 4 — group-flush-barrier-latch-skip-synced-segment-barrier
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // the naive/fixed arms carry the two-sided shape (the corpus convention)
fn r4_group_flush_barrier_latch() -> PerfRule {
    let name = "a durability path that appends then barriers per group flush tracks a `synced` latch — set true at every barrier (and on a non-durable backend), cleared false at every append — so the rotation's tail barrier fires ONLY when bytes were appended since the last sync; skipping the redundant barrier keeps ONE durability barrier on the open segment per group flush, while the NEW segment's identity magic still syncs on a full rotate and is deliberately skipped on a relaxed one (its recovery is a repair path, not a barrier)".to_string();
    let notes = "Grade A, suggestion-only, NotBenchable-by-domain (fsync-count honesty; the win metric is barrier COUNT against a durability contract — no validator hosts an fsync). SOURCE (relatedb/relate @ 144a9c14de…, Apache-2.0, re-pinned 2026-11-25; wal.rs moved in #742 — cites corrected): the latch field (relate/relate-storage/src/wal.rs:168-169, verbatim): 'barrier when the caller already synced this group flush.' + `synced: bool`; cleared at append (:385 'Bytes were appended since the last barrier.' + `self.synced = false;`), set at every sync (:526, :538 `self.synced = true;` — including the non-durable VFS early return: `if !self.inner.vfs.durable() { self.synced = true; return Ok(0); }`), read by `needs_sync` (:544 `self.inner.vfs.durable() && !self.synced`). The rotation tail (wal/rotation.rs:62-79, verbatim): 'Barrier only if the open segment holds bytes appended since the last `sync`. Group commit syncs right after every append (ADR-0019) and then rotates under the same WAL mutex, so this would otherwise be a redundant second barrier per group flush; skipping it keeps one barrier on the open data segment per group (ADR-0077).' — and the failed barrier POISONS the handle (`self.poisoned = true;` before the error return; see poisoned-handle lane intel, Research 422). The new segment's magic (wal/scan.rs:39-50, verbatim): 'The magic is the segment's identity header — a metadata-only write to a fresh file, not the commit barrier. `rotate` barriers the *old* data segment via the `synced` check in `close_active_segment`; this syncs the new segment's header so a crash after rotate can open it. Relaxed (snapshot) rotation deliberately skips it: its stable-storage boundary is the completed checkpoint, and `repair_short_new_magic` recovers a short or empty header.' THE TWO-SIDED SHAPE: the latch eliminates a REDUNDANT barrier (the same durability already delivered), it never skips a REQUIRED one — a full rotate still barriers the old data (unless the latch proves it synced) and still syncs the new magic. ADJUDICATION (dedup at landing HEAD, vocabularies synced+latch|redundant+barrier|group+flush): deferred-command-flush-hoist-out-of-batch-loop (entries_08) owns barrier BATCHING — hoisting N per-item flushes into ONE after the loop; THIS owns barrier ELIMINATION by state-latch — proving a barrier carries no NEW durability and dropping it entirely; the shapes compose (batch first, then latch the tail), and the in-rule row names both directions: entries_08 cannot subsume THIS (its hoisted flush still fires unconditionally — no latch), THIS cannot subsume entries_08 (per-item flushes between appends still batch); kernel_opt gpu_23's wait-scoping owns GPU-side sync scoping (a different medium, the wait-barrier analog); io's fsync family has no redundant-barrier owner (greps synced|barrier+latch zero). REGIME: any write path with (a) a per-group durability barrier, (b) a secondary operation (rotation, compaction hand-off, checkpoint fence) that would ALSO barrier the same surface, (c) a mutex or single-writer discipline making the latch race-free. ANTI-REGIMES: multi-writer surfaces where another writer can append between your sync and your check (the latch lies — needs the same-writer invariant stated); non-durable test backends (the latch must treat them as always-synced — the source's early-return shape, NOT skipped-latch); barriers carrying ORDER (a fence whose skip changes visible ordering is not this law). Fixtures b414_perf F4: the latch arm (append+sync+rotate = ONE barrier; append+rotate-without-sync = TWO), the non-durable arm (the latch sets true without a barrier), the full-rotate magic arm (new segment syncs; relaxed rotate skips + names the repair path), the tamper arm (a latch cleared nowhere double-barriers — the falsifier). Plan 388 / Research 422. Batch 414.".to_string();
    PerfRule {
        rule_id: "group-flush-barrier-latch-skip-synced-segment-barrier".to_string(),
        name,
        category: "io".to_string(),
        buggy_code: r#"// NAIVE: every path barriers unconditionally — the rotation's tail
// barrier fires even when the group flush synced the same bytes a
// microsecond ago. Two barriers per flush, one of them pure latency.
struct Wal {
    file: LogFile,
    durable: bool,
}

impl Wal {
    fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.file.write_all(bytes)
    }

    fn sync(&mut self) -> std::io::Result<()> {
        if self.durable {
            self.file.barrier()?;
        }
        Ok(())
    }

    fn rotate(&mut self, fresh: &str) -> std::io::Result<()> {
        // the tail barrier: unconditional — the flush JUST synced
        self.file.barrier()?;
        self.file = LogFile::create(fresh)?;
        self.file.sync_header()?;
        Ok(())
    }
}

struct LogFile {
    inner: std::fs::File,
}

impl LogFile {
    fn create(path: &str) -> std::io::Result<Self> {
        Ok(Self { inner: std::fs::File::create(path)? })
    }

    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        std::io::Write::write_all(&mut self.inner, bytes)
    }

    fn barrier(&mut self) -> std::io::Result<()> {
        self.inner.sync_all()
    }

    fn sync_header(&mut self) -> std::io::Result<()> {
        self.inner.sync_all()
    }
}"#
        .to_string(),
        fixed_code: r#"// FIXED: the synced latch — cleared at append, set at every sync
// (non-durable backends are always-synced), read by the rotation
// tail. ONE barrier per group flush; the failed barrier poisons.
struct Wal {
    file: LogFile,
    durable: bool,
    synced: bool,
    poisoned: bool,
}

impl Wal {
    fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.file.write_all(bytes)?;
        // Bytes were appended since the last barrier.
        self.synced = false;
        Ok(())
    }

    fn sync(&mut self) -> std::io::Result<()> {
        if !self.durable {
            // a non-durable backend is always-synced: no barrier, and
            // the latch must still read true so the tail never fires
            self.synced = true;
            return Ok(());
        }
        if let Err(e) = self.file.barrier() {
            self.poisoned = true;
            return Err(e);
        }
        self.synced = true;
        Ok(())
    }

    fn rotate(&mut self, fresh: &str, relaxed: bool) -> std::io::Result<()> {
        // Barrier only if bytes were appended since the last sync —
        // otherwise this would be a redundant second barrier per flush.
        if !relaxed && self.durable && !self.synced {
            if let Err(e) = self.file.barrier() {
                self.poisoned = true;
                return Err(e);
            }
        }
        self.file = LogFile::create(fresh)?;
        // the NEW segment's identity magic: sync on a full rotate so a
        // crash after rotate can open it; a relaxed rotation skips it —
        // its recovery is the repair path, not a barrier
        if !relaxed {
            self.file.sync_header()?;
        }
        Ok(())
    }

    fn needs_sync(&self) -> bool {
        self.durable && !self.synced
    }
}

struct LogFile {
    inner: std::fs::File,
}

impl LogFile {
    fn create(path: &str) -> std::io::Result<Self> {
        Ok(Self { inner: std::fs::File::create(path)? })
    }

    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        std::io::Write::write_all(&mut self.inner, bytes)
    }

    fn barrier(&mut self) -> std::io::Result<()> {
        self.inner.sync_all()
    }

    fn sync_header(&mut self) -> std::io::Result<()> {
        self.inner.sync_all()
    }
}"#
        .to_string(),
        context_notes: Some(notes),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 5 — hasher-choice-by-key-provenance-not-habit
// ---------------------------------------------------------------------------
#[must_use]
fn r5_hasher_choice_by_key_provenance() -> PerfRule {
    let name = "a hash TABLE's hasher is chosen by KEY PROVENANCE — who can choose the key — never by habit: keys the engine generates or a client cannot choose (allocated ids, tuples of them, allocator/ASLR-picked addresses) take the keyless fast hasher; keys a client can influence (property values, strings, anything decoded from the wire or statement text) take the keyed process-wide-seeded hasher; a table on neither policy needs a STATED reason; and a seeded variant of the keyless hasher is REJECTED outright where its compression runs before its seed — the seed cannot defeat precomputation".to_string();
    let notes = "Grade A, suggestion-only, NotBenchable-by-domain (a POLICY seam + a HashDoS regime; no validator hosts adversarial keys). SOURCE (relatedb/relate @ 144a9c14de…, Apache-2.0, re-pinned 2026-11-25; BOTH files byte-stable through #742): relate/relate-executor/src/hash_seed.rs:1-24 module doc, verbatim: 'Two policies. Pick by who can choose the key, never by habit; a table on neither is a default-hasher table (SipHash with a per-table random key) and must have a stated reason.' + '* **Id policy** — … on [`FxBuildHasher`], the keyless FxHash, for keys the engine generates or a client cannot choose: allocated node/rel ids and small tuples of them, schema and `Arc` allocation addresses (the allocator picks them, ASLR randomises the base)… ADR-0355 bars it from attacker-writable keys: it is not a keyed function, so keys a client can steer (property values, strings, map keys, anything decoded from the wire or the statement text) can be pre-collided offline.' + '* **Seeded policy** — … the one process-wide keyed SipHash state… for every table whose keys a client can influence.' THE REJECTION IS THE TECHNICAL CORE (relate/relate-front/src/ident_hash.rs:21-27, verbatim): 'A seeded FxHash (`rustc_hash::FxSeededState`) was rejected: FxHasher's `write` compresses a byte string through `hash_bytes` with fixed constants *before* the seeded state is touched, so any two identifiers that collide in that compression collide under every seed — the seed cannot defeat precomputation for string keys.' THE ONE-SEED LAW rides beside it (ident_hash.rs:15-19): 'One shared seed (instead of `RandomState::new()` per table) keeps table construction free of the thread-local seed read the executor's profile measured' — the seeded policy is ONE process-wide state, not per-table seeds, because the seed's job is process-lifetime unpredictability, not per-table isolation. ADJUDICATION (dedup at landing HEAD, vocabularies hasher+provenance|FxHash+seed|attacker+writable|keyless): the fxhashmap PROPOSER owns the mechanical HashMap→FxHashMap swap for 'non-adversarial keys' WITHOUT a provenance test — THIS rule is the POLICY BOUND that decides which keys qualify (the proposer gains this batch's seeded-Fx-rejected annotation face); single-entry-last-key-lookup-cache-hash-avoidance (entries_66) owns AVOIDING the hash entirely for a repeated key — compose (the memo's key class still needs a provenance verdict); sec's predictable-rng-for-secret-material owns SECRET MATERIAL generation (CWE-338) — different medium: THIS is hash-collision resistance for table DoS, not secret unpredictability; ADR-0355 IS the source's own policy ADR — quoted as provenance. REGIME: any service hashing client-reachable keys (wire protocols, query engines, session stores) plus hot engine-internal tables wanting the keyless hasher. ANTI-REGIMES: keys proven engine-only end to end (the keyless hasher IS the right answer — the rule fires on the UNSTATED default, not on Fx itself); short-lived processes where a per-table seed's construction cost is irrelevant and unpredictability wants maximal entropy per table; tables whose keys mix classes (split the table — one hasher per provenance class, never a mixed table). Fixtures b414_perf F5: the provenance decision table (each key class in, typed hasher out), the stated-reason arm (a table on neither policy refuses without a reason string), the seeded-Fx-rejection arm (two colliding-through-compression keys collide under EVERY seed — the falsifier), the one-seed arm (per-table RandomState::new costs a thread-local read per construction; the shared state reads once). Plan 388 / Research 422. Batch 414.".to_string();
    PerfRule {
        rule_id: "hasher-choice-by-key-provenance-not-habit".to_string(),
        name,
        category: "complexity".to_string(),
        buggy_code: r#"use std::collections::HashMap;
use std::collections::hash_map::RandomState;

// NAIVE: every table gets its hasher by habit — the default per-table
// RandomState for everything (paying a thread-local seed read per
// construction on hot internal tables), or a keyless fast hasher
// wherever the swap is trendy, attacker-reachable keys included.
enum Table {
    EngineIds(HashMap<u64, u32>),
    WireStrings(HashMap<String, u32, rustc_hash::FxBuildHasher>),
}

fn build_tables(ids: Vec<(u64, u32)>, wire: Vec<(String, u32)>) -> (Table, Table) {
    let mut t1 = HashMap::default();
    for (k, v) in ids {
        t1.insert(k, v);
    }
    let mut t2 = HashMap::default();
    for (k, v) in wire {
        t2.insert(k, v);
    }
    let _ = std::marker::PhantomData::<RandomState>;
    (Table::EngineIds(t1), Table::WireStrings(t2))
}"#
        .to_string(),
        fixed_code: r#"use std::collections::HashMap;
use std::sync::OnceLock;

// FIXED: the hasher seam — pick by who can choose the key. Engine
// keys (ids, addresses) take the keyless Fx; client-influenceable
// keys (wire strings) take the ONE process-wide seeded keyed state;
// anything else refuses without a stated reason.
#[derive(Default)]
struct SeededSipHashState(std::collections::hash_map::RandomState);

fn shared_random_state() -> &'static SeededSipHashState {
    static SEED: OnceLock<SeededSipHashState> = OnceLock::new();
    SEED.get_or_init(SeededSipHashState::default)
}

enum Table {
    /// id policy: keyless Fx — engine-generated keys only
    EngineIds(HashMap<u64, u32, rustc_hash::FxBuildHasher>),
    /// seeded policy: ONE process-wide keyed state — client-influenceable keys
    WireStrings(HashMap<String, u32>),
}

enum Provenance {
    EngineGenerated,
    ClientInfluenceable,
}

fn build_tables(
    ids: Vec<(u64, u32)>,
    wire: Vec<(String, u32)>,
) -> Result<(Table, Table), &'static str> {
    let mut t1 = HashMap::with_hasher(rustc_hash::FxBuildHasher);
    for (k, v) in ids {
        // engine-allocated ids: no client can pick them
        insert_by_provenance(Provenance::EngineGenerated, &mut t1_map(&mut t1), k, v)?;
    }
    let seed = shared_random_state().0.clone();
    let mut t2 = HashMap::with_hasher(seed);
    for (k, v) in wire {
        insert_by_provenance(Provenance::ClientInfluenceable, &mut t2, k, v)?;
    }
    Ok((Table::EngineIds(t1), Table::WireStrings(t2)))
}

fn t1_map(t: &mut HashMap<u64, u32, rustc_hash::FxBuildHasher>) -> &mut HashMap<u64, u32, rustc_hash::FxBuildHasher> {
    t
}

fn insert_by_provenance<K, V>(
    p: Provenance,
    table: &mut HashMap<K, V, impl std::hash::BuildHasher>,
    k: K,
    v: V,
) -> Result<(), &'static str> {
    match p {
        Provenance::EngineGenerated | Provenance::ClientInfluenceable => {
            table.insert(k, v);
            Ok(())
        }
    }
}"#
        .to_string(),
        context_notes: Some(notes),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 6 — checkpoint-coalesce-rewrite-units-anti-oscillation-manifest-bound
// ---------------------------------------------------------------------------
#[must_use]
fn r6_checkpoint_coalesce_rewrite_units() -> PerfRule {
    let name = "a checkpoint that rewrites dirty id ranges across shard files plans REWRITE UNITS, not per-previous-part passes — a dirty seed absorbs neighboring slots while the unit's id width stays inside the window bound AND the merged parts' rows/bytes stay inside one file's envelope AND the live decoded bytes fit the target — and the anti-oscillation rule keeps two files that miss one file's envelope TOGETHER as a unit, because splitting them re-encodes two to three whole units at every checkpoint".to_string();
    let notes = "Grade A, suggestion-only, NotBenchable-by-domain (checkpoint I/O-shape economics; the win metric is whole-unit encodes avoided — the source's own measured incident is the evidence, foreign silicon provenance). SOURCE (relatedb/relate @ 144a9c14de…, Apache-2.0, re-pinned 2026-11-25; coalesce.rs moved in #742 — cites corrected): relate/relate-storage/src/snapshot/native/coalesce.rs:1-11 module doc, verbatim: 'Rewrite units (ADR-0183). A checkpoint rewrites id ranges as units, not per previous part. It plans over the [`Slot`]s of the kind's layout: a window, or a kept part that overlaps a dirty id, seeds a unit, and the unit absorbs neighboring slots while its id range spans at most `MAX_WINDOW` ids, its kept parts' files hold no more rows and bytes together than one file can, and its live decoded bytes fit the target.' The absorption walk (coalesce.rs:148-151, verbatim): 'The width and the manifest are checked first, so a slot that cannot join is never measured' — the cheap bounds gate BEFORE the expensive live-row measure. THE ANTI-OSCILLATION BOUND IS THE DECISION CONTENT — ADR-0183 (docs/adr/0183-coalesce-checkpoint-shards.md:99-103, verbatim): 'The manifest bound keeps apart files that miss one file together: the two halves of a file the envelope fallback halved, for a unit over the page envelope or for incompressible rows near the target, both pass the decoded check, so without it the next checkpoint touching either half would merge them, miss and halve again, two to three whole-unit encodes every such checkpoint.' READ IT CAREFULLY — the bound is a REFUSAL: two files that TOGETHER miss the one-file envelope (the halves a previous envelope-miss fallback produced) must NOT absorb each other, because absorbing them merges past the envelope, the write halves them again, and every checkpoint repeats the merge-halve cycle at two-to-three whole-unit encodes; keeping them apart breaks the oscillation. The measured incident that motivated the ADR (:45-63): a 28,863-node pull scattered dirty ids so each mid-pull checkpoint cut node shards into 1–5-id files — 5,497 shard files, each costing 'a file fsync plus a directory fsync', with '3,704 of 4,133 checkpoint-thread samples sit in `fcntl` under `write_leaf`' (:58-59) — fragmentation is self-perpetuating ('a fragment is only ever split again'). ADJUDICATION (dedup at landing HEAD, vocabularies checkpoint+coalesce|rewrite+unit|anti-oscillation|shard+envelope): page-miss-blocks-one-ranged-read-scatter (entries_42, splash) owns READ-span coalescing (one ranged read spanning miss pages) — THIS owns CHECKPOINT FILE coalescing (write-path unit planning) — compose (the scatter rule's read span is the read-side twin); pack-store-gc-decides-by-recovered-index (entries_75, ajika) owns WHICH objects a GC deletes — THIS owns how a rewrite GROUPS ranges (the manifest-proof face rides entries_75 this batch); entries_80 hydradb bfs-depth-epoch-marker dedup owns level-fill dedup in graph walks — unrelated medium; entries_58's reserve-then-commit owns capacity ACCOUNTING — compose (the unit's envelope check IS a reservation before encode). REGIME: any sharded store whose checkpoints rewrite dirty ranges (LSM-ish compaction plans, shard stores, segment-structured caches) with per-file envelope caps. ANTI-REGIMES: stores where dirty ranges never straddle file boundaries (one dirty range = one file, no unit planning to do); unbounded window sizes (the MAX_WINDOW bound is load-bearing — without it one unit swallows the store and the checkpoint loses incremental value); stores whose file envelope is >= the whole working set (no coalescing pressure). Fixtures b414_perf F6: the unit-arm (two adjacent small misses absorb into one unit — one encode), the oscillation arm (two TOGETHER-oversized halves without the manifest bound merge, miss, halve, and re-encode every checkpoint — the count falsifier), the width-bound arm (a unit over MAX_WINDOW refuses to absorb, the slot never measured), the tamper arm (a planner measuring before the bounds check pays the live-row measure on an unjoinable slot). Plan 388 / Research 422. Batch 414.".to_string();
    PerfRule {
        rule_id: "checkpoint-coalesce-rewrite-units-anti-oscillation-manifest-bound".to_string(),
        name,
        category: "io".to_string(),
        buggy_code: r#"// NAIVE: each dirty id range rewrites against ITS OWN previous part
// file, and any two adjacent dirty slots merge unconditionally — two
// halves that TOGETHER miss the file envelope merge anyway, the write
// halves them again at the envelope fallback, and every checkpoint
// repeats the merge-halve cycle: two to three whole-unit encodes, forever.
struct ShardFile {
    id_range: (i64, i64),
    rows: u64,
    bytes: u64,
}

const FILE_ROWS_CAP: u64 = 1_000;
const FILE_BYTES_CAP: u64 = 1 << 20;

fn plan_checkpoint(dirty: &[(i64, i64)], files: &[ShardFile]) -> Vec<(i64, i64)> {
    let mut plan = Vec::new();
    let mut i = 0;
    while i < dirty.len() {
        let (mut lo, mut hi) = dirty[i];
        // merge ANY neighbor, never asking whether the merged parts
        // still fit one file — the envelope fallback will just halve
        // them again next checkpoint
        if i + 1 < dirty.len() {
            hi = dirty[i + 1].1;
            i += 1;
        }
        plan.push((lo, hi));
        i += 1;
    }
    plan
}

fn encodes_per_checkpoint(plan: &[(i64, i64)], files: &[ShardFile]) -> usize {
    plan.iter()
        .filter(|&(lo, hi)| files.iter().any(|f| f.id_range.0 >= *lo && f.id_range.1 <= *hi))
        .count()
}
"#
        .to_string(),
        fixed_code: r#"// FIXED: rewrite units — a dirty seed absorbs neighboring slots while
// the id width stays inside the window bound AND the merged parts'
// rows/bytes stay inside one file's envelope. The bounds are checked
// FIRST so an unjoinable slot is never measured; the manifest-bound
// refusal keeps two TOGETHER-oversized halves apart — absorbing them
// would merge past the envelope, get halved again by the fallback,
// and re-encode whole units at every checkpoint (the oscillation).
struct ShardFile {
    id_range: (i64, i64),
    rows: u64,
    bytes: u64,
}

const FILE_ROWS_CAP: u64 = 1_000;
const FILE_BYTES_CAP: u64 = 1 << 20;
const MAX_WINDOW: i64 = 4096;

fn plan_checkpoint(dirty: &[(i64, i64)], files: &[ShardFile]) -> Vec<(i64, i64)> {
    let mut plan = Vec::new();
    let mut i = 0;
    while i < dirty.len() {
        let (mut lo, mut hi) = dirty[i];
        let mut rows = rows_over(files, lo, hi);
        let mut bytes = bytes_over(files, lo, hi);
        // absorb the neighbor while every bound holds — bounds FIRST,
        // so an unjoinable slot is never measured
        while i + 1 < dirty.len() {
            let (nlo, nhi) = dirty[i + 1];
            let width = nhi - lo;
            let n_rows = rows_over(files, nlo, nhi);
            let n_bytes = bytes_over(files, nlo, nhi);
            if width > MAX_WINDOW
                || rows + n_rows > FILE_ROWS_CAP
                || bytes + n_bytes > FILE_BYTES_CAP
            {
                // the anti-oscillation refusal: together-oversized
                // neighbors stay apart as their own units
                break;
            }
            hi = nhi;
            rows += n_rows;
            bytes += n_bytes;
            i += 1;
        }
        plan.push((lo, hi));
        i += 1;
    }
    plan
}

fn rows_over(files: &[ShardFile], lo: i64, hi: i64) -> u64 {
    files
        .iter()
        .filter(|f| f.id_range.0 >= lo && f.id_range.1 <= hi)
        .map(|f| f.rows)
        .sum()
}

fn bytes_over(files: &[ShardFile], lo: i64, hi: i64) -> u64 {
    files
        .iter()
        .filter(|f| f.id_range.0 >= lo && f.id_range.1 <= hi)
        .map(|f| f.bytes)
        .sum()
}

fn encodes_per_checkpoint(plan: &[(i64, i64)], files: &[ShardFile]) -> usize {
    plan.iter()
        .filter(|&(lo, hi)| files.iter().any(|f| f.id_range.0 >= *lo && f.id_range.1 <= *hi))
        .count()
}
"#
        .to_string(),
        context_notes: Some(notes),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}
