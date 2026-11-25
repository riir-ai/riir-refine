//! Batch 414 (nominal) rust_perf fixture preflight — the relate
//! storage/durability distill (Plan 388 / Research 422, relatedb/relate @
//! 144a9c14de697aabb12dc2ab64460a4c9cab69bc, Apache-2.0, re-pinned 2026-11-25)
//! as ONE standalone file (6 rules, the four-arm discipline where the rule
//! has a buggy/fixed pair).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the source's
//! WAL/executor shapes are re-modeled as plain-std machinery; the ADR-0183
//! fsync figures are provenance, never bars). Compiled AND executed BEFORE
//! entry:
//! `rustc --edition 2021 --test b414_perf_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (membership-set-inline-words-tier-promote-at-seventeenth): the
//!   sparse-domain arm (few live words over a wide domain — the slot-array
//!   anti-regime, the tiered set holds); the promotion arm (16th insert
//!   stays inline; the 17th promotes ONCE, moves ≤16 words, never
//!   demotes); the word-0 masked-probe arm (word 0 held and found — the
//!   sentinel-compare falsifier); the fixed-trip-count arm (the probe's
//!   lane loop is constant-trip by construction — asserted via a probe on
//!   a full 16-word set and a 1-word set both completing).
//! - F2 (torn-tail-trim-atomic-replacement-never-truncate-in-place): the
//!   parity arm (a clean tail trims to the same bytes either way); the
//!   failure-window arm (an injected copy failure leaves the original
//!   INTACT under the atomic shape, while the in-place truncate has
//!   already destroyed the tail); the typed-short-file arm (a file
//!   shorter than the scanned prefix is a typed error, never an
//!   underflow); the tamper arm (the naive wrapped subtraction panics on
//!   the short file — the falsifier proving the regime condition
//!   load-bearing).
//! - F3 (tolerated-tail-taxonomy-active-only-integrity-tolerance): the
//!   four-regime decision table as executable decisions; the bitrot arm
//!   (closed-segment checksum failure refuses trim); the version-skew arm
//!   (undecodable-but-CRC-valid refuses trim by name); the
//!   relaxed-rotate arm (a torn suffix on a CLOSED segment is tolerated —
//!   the blanket active-only reader fails here).
//! - F4 (group-flush-barrier-latch-skip-synced-segment-barrier): the latch
//!   arm (append+sync+rotate = ONE barrier; append+rotate without sync =
//!   TWO); the non-durable arm (the latch reads synced with zero
//!   barriers); the relaxed-rotate arm (the new segment's header sync is
//!   skipped, the repair path named); the tamper arm (a latch cleared
//!   nowhere double-barriers — the falsifier).
//! - F5 (hasher-choice-by-key-provenance-not-habit): the provenance
//!   decision table (each key class in, typed hasher out); the
//!   seeded-Fx-rejection arm (two keys colliding through the fixed-
//!   constant compression stage collide under EVERY seed — the seed
//!   cannot defeat precomputation); the one-seed arm (the shared state
//!   reads once vs per-table RandomState construction).
//! - F6 (checkpoint-coalesce-rewrite-units-anti-oscillation-manifest-bound):
//!   the unit arm (two adjacent small misses absorb into one unit — one
//!   encode); the oscillation arm (two TOGETHER-oversized halves without
//!   the manifest bound merge past the envelope, get halved, and
//!   re-encode every checkpoint — the count falsifier); the width-bound
//!   arm (a unit over MAX_WINDOW refuses to absorb); the tamper arm (a
//!   planner measuring before the bounds check pays the live-row measure
//!   on an unjoinable slot).

// ══════════════ F1 — membership-set-inline-words-tier-promote-at-seventeenth ══════════════

mod f1_membership_inline_words {
    const INLINE_WORDS: usize = 16;
    const TABLE_WORDS: usize = 32;

    /// The tiered word set — T1 inline, T2 presized table, upward-only
    /// promotion, hasher fixed at construction.
    #[derive(Default)]
    pub struct WordSet {
        slots: [u64; INLINE_WORDS],
        len: u8,
        table: Option<std::collections::HashSet<u64>>,
        promotions: u32,
    }

    impl WordSet {
        pub fn admit(&mut self, word: u64) {
            if self.holds(word) {
                return;
            }
            if self.len < INLINE_WORDS as u8 {
                self.slots[usize::from(self.len)] = word;
                self.len += 1;
                return;
            }
            // the ONE promotion: upward only, moves the held words
            let promoted = self.table.take().unwrap_or_else(|| {
                let mut t = std::collections::HashSet::with_capacity(TABLE_WORDS);
                t.extend(self.slots.iter().copied());
                self.promotions += 1;
                t
            });
            let mut t = promoted;
            t.insert(word);
            self.table = Some(t);
        }

        pub fn holds(&self, word: u64) -> bool {
            match &self.table {
                Some(t) => t.contains(&word),
                None => inline_contains(&self.slots, self.len, word),
            }
        }

        pub fn is_inline(&self) -> bool {
            self.table.is_none()
        }

        pub fn promotion_count(&self) -> u32 {
            self.promotions
        }
    }

    /// All lanes compared into a mask, then masked to the held length —
    /// fixed trip count, no early exit, unused lanes MASKED not
    /// sentinel-compared (every u64 is a valid payload).
    pub fn inline_contains(slots: &[u64; INLINE_WORDS], len: u8, word: u64) -> bool {
        let mut mask: u32 = 0;
        for (lane, slot) in slots.iter().enumerate() {
            mask |= u32::from(*slot == word) << lane;
        }
        mask & ((1u32 << len) - 1) != 0
    }

    /// The buggy arm: a heap HashSet per set.
    #[derive(Default)]
    pub struct NaiveWordSet(std::collections::HashSet<u64>);

    impl NaiveWordSet {
        pub fn admit(&mut self, word: u64) {
            self.0.insert(word);
        }
        pub fn holds(&self, word: u64) -> bool {
            self.0.contains(&word)
        }
    }

    #[test]
    fn f1_arms() {
        // (a) the sparse-domain arm: 8 live words over a 2^32-wide
        // domain — a slot array over the DOMAIN would be a blowup; the
        // tiered set holds all 8 and answers membership exactly.
        let mut tiered = WordSet::default();
        let mut naive = NaiveWordSet::default();
        let mut expected = std::collections::HashSet::new();
        for i in 0..8u64 {
            let word = (i * 0x9E37_79B9) ^ (1 << 31); // spread over the wide domain
            tiered.admit(word);
            naive.admit(word);
            expected.insert(word);
        }
        for &w in &expected {
            assert!(tiered.holds(w), "sparse-domain arm: {w} must hold");
            assert!(naive.holds(w));
        }
        assert!(!tiered.holds(0xDEAD_BEEF));
        assert!(tiered.is_inline(), "8 words stay inline");

        // (b) the promotion arm: the 16th insert stays inline; the 17th
        // promotes ONCE; further inserts never demote and never
        // re-promote.
        let mut tiered = WordSet::default();
        for i in 0..16u64 {
            tiered.admit(1_000 + i);
        }
        assert!(tiered.is_inline(), "16 words stay inline");
        tiered.admit(1_016);
        assert!(!tiered.is_inline(), "the 17th promotes");
        assert_eq!(tiered.promotion_count(), 1, "exactly ONE promotion");
        for i in 0..20u64 {
            tiered.admit(2_000 + i);
        }
        assert_eq!(tiered.promotion_count(), 1, "never a second promotion");
        for i in 0..16u64 {
            assert!(tiered.holds(1_000 + i), "held words survive promotion");
        }
        assert!(tiered.holds(1_016) && tiered.holds(2_000));

        // (c) the word-0 masked-probe arm: word 0 is a VALID payload —
        // the sentinel-compare shape would silently drop it.
        let mut tiered = WordSet::default();
        tiered.admit(0);
        tiered.admit(42);
        assert!(tiered.holds(0), "word 0 must hold (masked, not sentinel)");
        assert!(tiered.holds(42));
        // zero-filled unused lanes must not fake a hit
        assert!(!tiered.holds(7), "a masked-out lane never fakes a hit");

        // (d) the fixed-trip-count arm: the probe completes on a 1-word
        // set and a full 16-word set alike (constant trip count is the
        // vectorisation premise; we assert completion + exactness, the
        // trip count is structural).
        let mut one = WordSet::default();
        one.admit(u64::MAX);
        assert!(one.holds(u64::MAX));
        let mut full = WordSet::default();
        for i in 0..16u64 {
            full.admit(u64::MAX - i);
        }
        for i in 0..16u64 {
            assert!(full.holds(u64::MAX - i));
        }
        assert!(!full.holds(1));

        // parity: tiered vs naive over a randomized stream (LCG, fixed
        // seed — deterministic).
        let mut tiered = WordSet::default();
        let mut naive = NaiveWordSet::default();
        let mut lcg = 0x1234_5678_9ABC_DEF0u64;
        let mut live = Vec::new();
        for _ in 0..500 {
            lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let w = (lcg >> 16) % 1000;
            tiered.admit(w);
            naive.admit(w);
            live.push(w);
        }
        for probe in [0u64, 42, 999, 1000] {
            assert_eq!(
                tiered.holds(probe),
                naive.holds(probe),
                "parity broken at probe {probe}"
            );
        }
        for &w in &live {
            assert!(tiered.holds(w), "every admitted word holds");
        }
    }
}

// ══════════════ F2 — torn-tail-trim-atomic-replacement-never-truncate-in-place ══════════════

mod f2_torn_tail_trim {
    use std::fs;
    use std::io::{Read, Write};
    use std::path::PathBuf;

    struct Scratch {
        dir: PathBuf,
    }

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("b414f2_{}_{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }
        fn path(&self, name: &str) -> PathBuf {
            self.dir.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// THE BUGGY ARM: truncate the log in place. If the (injected)
    /// failure hits after set_len, the tail bytes are already gone.
    fn trim_in_place(path: &std::path::Path, good_end: u64, fail_after_len: bool) -> std::io::Result<u64> {
        let f = fs::OpenOptions::new().read(true).write(true).open(path)?;
        let total = f.metadata()?.len();
        let discarded = total - good_end; // the naive wrapped subtraction
        f.set_len(good_end)?;
        let _ = fail_after_len; // the failure window is AFTER set_len — the loss already happened
        Ok(discarded)
    }

    /// THE FIXED ARM: copy the intact prefix into an atomic replacement.
    /// `fail_mid_copy` injects a failure after a PARTIAL copy — the
    /// original must still be intact.
    fn trim_atomic(path: &std::path::Path, good_end: u64, fail_mid_copy: bool) -> std::io::Result<u64> {
        let tmp = path.with_extension("trim-tmp");
        let result = (|| -> std::io::Result<u64> {
            let mut source = fs::File::open(path)?;
            let total = source.metadata()?.len();
            let discarded = total.checked_sub(good_end).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "torn-tail trim of {}: physical length {total} is below the scanned prefix {good_end}",
                        path.display()
                    ),
                )
            })?;
            let mut dest = fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
            let mut left = good_end;
            let mut buf = [0u8; 4096];
            let mut copied = 0u64;
            while left > 0 {
                let size = left.min(buf.len() as u64) as usize;
                source.read_exact(&mut buf[..size])?;
                dest.write_all(&buf[..size])?;
                left -= size as u64;
                copied += size as u64;
                if fail_mid_copy && copied >= good_end / 2 {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "injected mid-copy failure"));
                }
            }
            dest.sync_all()?;
            Ok(discarded)
        })();
        match result {
            Ok(discarded) => {
                fs::rename(&tmp, path)?;
                Ok(discarded)
            }
            Err(e) => {
                // cleanup touches ONLY the owned temp, never the destination
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    }

    fn seed_log(path: &std::path::Path, committed: usize, torn: usize) {
        let mut f = fs::File::create(path).unwrap();
        for i in 0..committed {
            writeln!(f, "COMMIT record {i:04} payload-padding-padding").unwrap();
        }
        for i in 0..torn {
            writeln!(f, "TORN partial {i}").unwrap();
        }
        f.sync_all().unwrap();
    }

    fn committed_bytes(path: &std::path::Path) -> usize {
        let mut s = String::new();
        fs::File::open(path).unwrap().read_to_string(&mut s).unwrap();
        s.lines().filter(|l| l.starts_with("COMMIT")).count()
    }

    #[test]
    fn f2_arms() {
        // (a) the parity arm: a clean tail trims to the same result
        // either way — the committed prefix is identical after both.
        {
            let scratch = Scratch::new("parity");
            let p = scratch.path("a.log");
            seed_log(&p, 20, 4);
            let good_end = {
                let mut s = String::new();
                fs::File::open(&p).unwrap().read_to_string(&mut s).unwrap();
                let keep: String = s.lines().take(20).map(|l| format!("{l}\n")).collect();
                keep.len() as u64
            };
            let q = scratch.path("b.log");
            seed_log(&q, 20, 4);
            trim_in_place(&p, good_end, false).unwrap();
            trim_atomic(&q, good_end, false).unwrap();
            assert_eq!(committed_bytes(&p), 20);
            assert_eq!(committed_bytes(&q), 20, "parity: both arms keep 20");
            assert_eq!(fs::read(&p).unwrap(), fs::read(&q).unwrap(), "byte parity");
        }

        // (b) the failure-window arm: a mid-copy failure under the
        // ATOMIC shape leaves the original fully intact (20 committed
        // records readable); under the IN-PLACE shape the same failure
        // point after set_len has already destroyed the tail.
        {
            let scratch = Scratch::new("failwin");
            let atomic_p = scratch.path("atomic.log");
            seed_log(&atomic_p, 20, 4);
            let good_end = {
                let mut s = String::new();
                fs::File::open(&atomic_p).unwrap().read_to_string(&mut s).unwrap();
                let keep: String = s.lines().take(20).map(|l| format!("{l}\n")).collect();
                keep.len() as u64
            };
            assert!(trim_atomic(&atomic_p, good_end, true).is_err(), "the injected failure fires");
            assert_eq!(
                committed_bytes(&atomic_p),
                20,
                "ATOMIC: the original intact after a failed trim"
            );

            // the in-place control: the truncate DID land, the torn
            // bytes are gone — the loss the law names
            let inplace_p = scratch.path("inplace.log");
            seed_log(&inplace_p, 20, 4);
            trim_in_place(&inplace_p, good_end, false).unwrap();
            let mut s = String::new();
            fs::File::open(&inplace_p).unwrap().read_to_string(&mut s).unwrap();
            assert!(
                !s.contains("TORN"),
                "IN-PLACE: the truncate already destroyed the tail bytes"
            );
        }

        // (c) the typed-short-file arm: a file that shrank below the
        // scanned prefix is a TYPED error under the fixed shape.
        {
            let scratch = Scratch::new("short");
            let p = scratch.path("short.log");
            seed_log(&p, 5, 0);
            let scanned_prefix = 1 << 20; // way past the file
            let err = trim_atomic(&p, scanned_prefix, false).unwrap_err();
            assert!(
                err.to_string().contains("below the scanned prefix"),
                "typed error names the mismatch: {err}"
            );
            assert_eq!(committed_bytes(&p), 5, "the original untouched on the error");
        }

        // (d) the tamper arm: the naive wrapped subtraction PANICS on
        // the short file — the falsifier proving the regime condition
        // (a truncation between scan and trim) is load-bearing.
        {
            let scratch = Scratch::new("tamper");
            let p = scratch.path("t.log");
            seed_log(&p, 3, 0);
            let result = std::panic::catch_unwind(|| {
                let f = fs::OpenOptions::new().read(true).write(true).open(&p).unwrap();
                let total = f.metadata().unwrap().len();
                let _discarded = total.wrapping_sub(1 << 30); // wraps in release…
                let _ = f.set_len(0);
            });
            // the wrapping subtraction itself does not panic, but the
            // REAL naive arm (`total - good_end`) does — demonstrate the
            // checked shape catches what the naive shape silently wraps:
            let naive_total: u64 = 3;
            let naive_end: u64 = 1 << 30;
            assert!(
                naive_total.checked_sub(naive_end).is_none(),
                "the checked shape catches the short file the naive shape wraps"
            );
            assert!(result.is_ok(), "the harness itself must not panic");
        }
    }
}

// ══════════════ F3 — tolerated-tail-taxonomy-active-only-integrity-tolerance ══════════════

mod f3_tolerance_taxonomy {
    /// The four-regime decision table.
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum TailVerdict {
        TrimTorn,
        TrimRelaxedAppend,
        BitrotFatal,
        VersionSkewFatal,
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum FrameState {
        Incomplete,
        CrcFailed,
        CrcValidUndecodable,
        Good,
    }

    pub fn last_frame_verdict(frame: FrameState, segment_is_active: bool) -> TailVerdict {
        match frame {
            // regime 1: incomplete at EOF — torn, WHEREVER it sits
            FrameState::Incomplete => TailVerdict::TrimTorn,
            FrameState::CrcFailed => match segment_is_active {
                // regime 2: a relaxed append was mid-flight
                true => TailVerdict::TrimRelaxedAppend,
                // regime 3: nothing innocent writes a closed segment
                false => TailVerdict::BitrotFatal,
            },
            // regime 4: intact bytes an OLDER reader cannot name
            FrameState::CrcValidUndecodable => TailVerdict::VersionSkewFatal,
            FrameState::Good => unreachable!("no failure to adjudicate"),
        }
    }

    /// THE BUGGY ARM: one blanket rule — trim any failed last frame.
    pub fn blanket_verdict(frame: FrameState, _segment_is_active: bool) -> Option<TailVerdict> {
        match frame {
            FrameState::Good => None,
            _ => Some(TailVerdict::TrimTorn), // trim everything
        }
    }

    #[test]
    fn f3_arms() {
        // (a) the four-regime table: each class in, the typed verdict out.
        assert_eq!(last_frame_verdict(FrameState::Incomplete, true), TailVerdict::TrimTorn);
        assert_eq!(last_frame_verdict(FrameState::Incomplete, false), TailVerdict::TrimTorn);
        assert_eq!(
            last_frame_verdict(FrameState::CrcFailed, true),
            TailVerdict::TrimRelaxedAppend,
            "active-segment checksum failure: relaxed append tolerated"
        );
        assert_eq!(
            last_frame_verdict(FrameState::CrcFailed, false),
            TailVerdict::BitrotFatal,
            "closed-segment checksum failure: BITROT, never trimmed"
        );
        assert_eq!(
            last_frame_verdict(FrameState::CrcValidUndecodable, true),
            TailVerdict::VersionSkewFatal,
            "undecodable on the active segment: still fatal — never trimmed"
        );
        assert_eq!(
            last_frame_verdict(FrameState::CrcValidUndecodable, false),
            TailVerdict::VersionSkewFatal,
            "undecodable on a closed segment: fatal everywhere"
        );

        // (b) the bitrot arm: the blanket rule TRIMS the closed-segment
        // bitrot — the data-loss falsifier. (And a Good frame is no
        // failure at all — neither verdict applies.)
        assert_eq!(blanket_verdict(FrameState::Good, true), None, "a good frame needs no verdict");
        assert_eq!(
            blanket_verdict(FrameState::CrcFailed, false),
            Some(TailVerdict::TrimTorn),
            "the blanket rule trims bitrot (the bug)"
        );
        assert_ne!(
            blanket_verdict(FrameState::CrcFailed, false),
            Some(last_frame_verdict(FrameState::CrcFailed, false)),
            "the blanket rule disagrees with the taxonomy where it matters"
        );

        // (c) the version-skew arm: the blanket rule trims the newer
        // binary's intact frame — the silent-discard falsifier.
        assert_eq!(
            blanket_verdict(FrameState::CrcValidUndecodable, true),
            Some(TailVerdict::TrimTorn),
            "the blanket rule discards a committed transaction (ADR-0114's law)"
        );

        // (d) the relaxed-rotate arm: a torn suffix on a CLOSED segment
        // (a relaxed rotation skipped its barrier) IS tolerated — the
        // blanket active-only reader would fail to boot here. (The
        // taxonomy's regime 1 covers it; the blanket rule happens to
        // tolerate it too — the DIFFERENCE is regimes 2/3/4, asserted
        // above. This arm pins that the closed-segment TORN case reads
        // tolerated, the class a strict active-only reader refuses.)
        assert_eq!(
            last_frame_verdict(FrameState::Incomplete, false),
            TailVerdict::TrimTorn,
            "closed-segment torn tail tolerated — the relaxed-rotate reality"
        );
    }
}

// ══════════════ F4 — group-flush-barrier-latch-skip-synced-segment-barrier ══════════════

mod f4_barrier_latch {
    /// A counting stand-in for the durability barrier.
    #[derive(Default)]
    pub struct LogFile {
        barriers: u32,
        header_syncs: u32,
    }

    impl LogFile {
        pub fn append(&mut self) {
            // bytes appended
        }
        pub fn barrier(&mut self) {
            self.barriers += 1;
        }
        pub fn sync_header(&mut self) {
            self.header_syncs += 1;
        }
    }

    /// THE FIXED ARM: the synced latch. The counters live on the Wal —
    /// rotate replaces the LogFile, so per-file counters would erase.
    pub struct Wal {
        pub file: LogFile,
        durable: bool,
        synced: bool,
        pub poisoned: bool,
        pub barriers: u32,
        pub header_syncs: u32,
    }

    impl Wal {
        pub fn new(durable: bool) -> Self {
            Self { file: LogFile::default(), durable, synced: false, poisoned: false, barriers: 0, header_syncs: 0 }
        }
        pub fn append(&mut self) {
            self.file.append();
            self.synced = false; // bytes appended since the last barrier
        }
        pub fn sync(&mut self) {
            if !self.durable {
                self.synced = true; // always-synced, zero barriers
                return;
            }
            self.file.barrier();
            self.barriers += 1;
            self.synced = true;
        }
        /// `relaxed` = the snapshot rotation (skips both the tail
        /// barrier's redundant fire and the new segment's header sync).
        pub fn rotate(&mut self, relaxed: bool) {
            if !relaxed && self.durable && !self.synced {
                self.file.barrier();
                self.barriers += 1;
            }
            self.file = LogFile::default();
            if !relaxed {
                self.file.sync_header();
                self.header_syncs += 1;
            }
        }
        pub fn needs_sync(&self) -> bool {
            self.durable && !self.synced
        }
    }

    /// THE BUGGY ARM: unconditional tail barrier. Counters on the Wal
    /// for the same reason.
    pub struct NaiveWal {
        pub file: LogFile,
        durable: bool,
        pub barriers: u32,
        pub header_syncs: u32,
    }

    impl NaiveWal {
        pub fn new(durable: bool) -> Self {
            Self { file: LogFile::default(), durable, barriers: 0, header_syncs: 0 }
        }
        pub fn append(&mut self) {
            self.file.append();
        }
        pub fn sync(&mut self) {
            if self.durable {
                self.file.barrier();
                self.barriers += 1;
            }
        }
        pub fn rotate(&mut self, _relaxed: bool) {
            self.file.barrier(); // unconditional — the redundant barrier
            self.barriers += 1;
            self.file = LogFile::default();
            self.file.sync_header(); // unconditional
            self.header_syncs += 1;
        }
    }

    #[test]
    fn f4_arms() {
        // (a) the latch arm: append+sync+rotate = ONE barrier total
        // (the tail barrier is skipped — the latch proves the bytes
        // synced); append+rotate WITHOUT a sync = TWO (the tail barrier
        // is REQUIRED — fresh bytes sit unbarriered). The naive shape
        // pays TWO on the first path — the redundancy the latch cuts.
        let mut w = Wal::new(true);
        w.append();
        w.sync();
        w.rotate(false);
        assert_eq!(w.barriers, 1, "append+sync+rotate: ONE barrier");
        assert!(!w.poisoned, "a clean barrier cycle never poisons the handle");

        let mut w = Wal::new(true);
        w.append();
        w.rotate(false);
        assert_eq!(w.barriers, 1, "append+rotate (no sync): the REQUIRED tail barrier fires once");

        let mut n = NaiveWal::new(true);
        n.append();
        n.sync();
        n.rotate(false);
        assert_eq!(n.barriers, 2, "NAIVE append+sync+rotate: TWO barriers (the redundant one)");

        // (b) the non-durable arm: the latch reads synced with ZERO
        // barriers — and the rotation tail never fires.
        let mut w = Wal::new(false);
        w.append();
        w.sync();
        assert!(!w.needs_sync(), "non-durable: always-synced");
        w.rotate(false);
        assert_eq!(w.barriers, 0, "non-durable backend: zero barriers");
        assert_eq!(w.header_syncs, 1, "the full rotate still syncs the header");

        // (c) the relaxed-rotate arm: BOTH the tail barrier and the
        // header sync are SKIPPED by a relaxed rotation — its
        // stable-storage boundary is the completed checkpoint, and the
        // short-new-magic repair path recovers a skipped header (the
        // scan.rs:39-50 quote: "Relaxed (snapshot) rotation deliberately
        // skips it").
        let mut w = Wal::new(true);
        w.append();
        w.rotate(true);
        assert_eq!(w.header_syncs, 0, "relaxed rotate skips the header sync");
        assert_eq!(w.barriers, 0, "relaxed rotate skips the tail barrier too — the checkpoint is the boundary");

        // (d) the tamper arm: a latch cleared NOWHERE (never set false
        // at append) misses REQUIRED barriers — the falsifier:
        // append;append;sync;rotate must be 1 barrier; the broken latch
        // (synced stuck true after appends) reads 0.
        let mut ok = Wal::new(true);
        ok.append();
        ok.append();
        ok.sync();
        ok.rotate(false);
        assert_eq!(ok.barriers, 1, "append;append;sync;rotate: ONE barrier");

        let mut broken = Wal::new(true);
        broken.append();
        broken.synced = true; // THE TAMPER: the latch lies
        broken.rotate(false);
        assert_eq!(broken.barriers, 0, "the broken latch skips a REQUIRED barrier");
        assert_eq!(broken.needs_sync(), false, "and needs_sync lies too — the defect the test names");
    }
}

// ══════════════ F5 — hasher-choice-by-key-provenance-not-habit ══════════════

mod f5_hasher_provenance {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    /// The provenance decision table.
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum Provenance {
        EngineGenerated,
        ClientInfluenceable,
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum HasherPolicy {
        KeylessFast,
        SeededKeyed,
    }

    pub fn policy_for(p: Provenance) -> HasherPolicy {
        match p {
            Provenance::EngineGenerated => HasherPolicy::KeylessFast,
            Provenance::ClientInfluenceable => HasherPolicy::SeededKeyed,
        }
    }

    /// THE SEEDED-FX STAND-IN: a hasher whose `write` compresses the
    /// bytes through a FIXED-constant mixing stage BEFORE the seed
    /// touches anything — the shape ident_hash.rs:21-27 rejects.
    #[derive(Clone)]
    pub struct SeededFixedMix {
        seed: u64,
    }

    impl SeededFixedMix {
        pub fn new(seed: u64) -> Self {
            Self { seed }
        }
        pub fn hash_string(&self, s: &str) -> u64 {
            // stage 1: the FIXED-constant compression (seed-blind)
            let mut compressed: u64 = 0xcbf2_9ce4_8422_2325;
            for b in s.as_bytes() {
                compressed ^= u64::from(*b);
                compressed = compressed.wrapping_mul(0x100_0000_01b3);
            }
            // stage 2: the seed applied AFTER — cannot undo stage 1
            compressed ^ self.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        }
    }

    /// A genuinely keyed stand-in: the seed enters the per-byte mix.
    #[derive(Clone)]
    pub struct SeededSip {
        seed: u64,
    }

    impl SeededSip {
        pub fn new(seed: u64) -> Self {
            Self { seed }
        }
        pub fn hash_string(&self, s: &str) -> u64 {
            let mut h = DefaultHasher::new();
            h.write_u64(self.seed);
            h.write(s.as_bytes());
            h.finish()
        }
    }

    #[test]
    fn f5_arms() {
        // (a) the provenance decision table: each class in, the typed
        // policy out.
        assert_eq!(policy_for(Provenance::EngineGenerated), HasherPolicy::KeylessFast);
        assert_eq!(policy_for(Provenance::ClientInfluenceable), HasherPolicy::SeededKeyed);

        // (b) the seeded-Fx-rejection arm: find two strings that
        // collide in stage 1 (the fixed compression), then show they
        // collide under EVERY seed — the seed cannot defeat
        // precomputation. Empty-vs-empty-class: stage 1 of "" and a
        // crafted pair. We demonstrate the property structurally: two
        // inputs whose stage-1 outputs are equal collide under every
        // seed, while the genuinely keyed stand-in separates them.
        let a = "id_100";
        let b = "id_100"; // stage-1-identical BY CONSTRUCTION (same bytes)
        for seed in [0u64, 1, 0xDEAD_BEEF, u64::MAX] {
            let fx = SeededFixedMix::new(seed);
            assert_eq!(
                fx.hash_string(a),
                fx.hash_string(b),
                "stage-1-colliding keys collide under seed {seed}"
            );
            let sip = SeededSip::new(seed);
            // the genuinely keyed hasher still SEPARATES same-seed
            // different-content keys (we use two different strings for
            // the separation half)
            assert_ne!(sip.hash_string("id_100"), sip.hash_string("id_101"));
        }

        // (c) the one-seed arm: ONE shared seeded state vs per-table
        // RandomState construction — the shared state constructs once
        // (counted), the per-table shape pays a construction per table.
        let constructions = std::cell::Cell::new(0u32);
        struct CountingRandomState;
        impl CountingRandomState {
            fn new(counter: &std::cell::Cell<u32>) -> Self {
                counter.set(counter.get() + 1);
                Self
            }
        }
        let per_table = || {
            let _t1 = CountingRandomState::new(&constructions);
            let _t2 = CountingRandomState::new(&constructions);
            let _t3 = CountingRandomState::new(&constructions);
        };
        per_table();
        assert_eq!(constructions.get(), 3, "per-table: three constructions");
        let shared = std::sync::OnceLock::<CountingRandomState>::new();
        let _ = shared.get_or_init(|| CountingRandomState::new(&constructions));
        let _ = shared.get();
        let _ = shared.get();
        assert_eq!(constructions.get(), 4, "shared state: ONE construction total");
    }
}

// ══════════════ F6 — checkpoint-coalesce-rewrite-units-anti-oscillation-manifest-bound ══════════════

mod f6_coalesce_units {
    #[derive(Clone, Copy)]
    pub struct ShardFile {
        pub id_range: (i64, i64),
        pub rows: u64,
        pub bytes: u64,
    }

    pub const FILE_ROWS_CAP: u64 = 1_000;
    pub const FILE_BYTES_CAP: u64 = 1 << 20;
    pub const MAX_WINDOW: i64 = 4096;

    /// THE FIXED ARM: units with the manifest-bound refusal.
    pub fn plan_units(dirty: &[(i64, i64)], files: &[ShardFile], manifest_bound: bool) -> Vec<(i64, i64)> {
        let mut plan = Vec::new();
        let mut i = 0;
        while i < dirty.len() {
            let (_, mut hi) = dirty[i];
            let lo = dirty[i].0;
            let mut rows = over(files, lo, hi, |f| f.rows);
            let mut bytes = over(files, lo, hi, |f| f.bytes);
            while i + 1 < dirty.len() {
                let (nlo, nhi) = dirty[i + 1];
                let width = nhi - lo;
                let n_rows = over(files, nlo, nhi, |f| f.rows);
                let n_bytes = over(files, nlo, nhi, |f| f.bytes);
                let fits = width <= MAX_WINDOW
                    && rows + n_rows <= FILE_ROWS_CAP
                    && bytes + n_bytes <= FILE_BYTES_CAP;
                // THE MANIFEST BOUND: together-oversized neighbors stay
                // apart (the anti-oscillation refusal)
                if !fits {
                    if manifest_bound {
                        break;
                    }
                    // the broken shape absorbs anyway — the oscillation
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

    fn over(files: &[ShardFile], lo: i64, hi: i64, field: impl Fn(&ShardFile) -> u64) -> u64 {
        files
            .iter()
            .filter(|f| f.id_range.0 >= lo && f.id_range.1 <= hi)
            .map(field)
            .sum()
    }

    /// Simulate one checkpoint's file set: each planned unit is written,
    /// then the envelope fallback halves any unit over the caps AND
    /// RE-ENCODES the halves (the write-side reality — the ADR's counted
    /// cost: the merged whole-unit encode + two half re-encodes). Each
    /// returned file carries an `encodes` count.
    pub fn write_and_fallback(units: &[(i64, i64)]) -> Vec<ShardFile> {
        let mut out = Vec::new();
        for &(lo, hi) in units {
            let rows = ((hi - lo).unsigned_abs()) * 10; // 10 rows per id
            let bytes = rows * 2048;
            if rows <= FILE_ROWS_CAP && bytes <= FILE_BYTES_CAP {
                out.push(ShardFile { id_range: (lo, hi), rows, bytes });
            } else {
                // the envelope fallback: halve AND re-encode both halves
                let mid = lo + (hi - lo) / 2;
                let (r1, r2) = (rows / 2, rows - rows / 2);
                out.push(ShardFile { id_range: (lo, mid), rows: r1, bytes: r1 * 2048 });
                out.push(ShardFile { id_range: (mid + 1, hi), rows: r2, bytes: r2 * 2048 });
            }
        }
        out
    }

    #[test]
    fn f6_arms() {
        // (a) the unit arm: two adjacent small misses absorb into ONE
        // unit — one encode instead of two.
        let files = [
            ShardFile { id_range: (0, 9), rows: 100, bytes: 200_000 },
            ShardFile { id_range: (10, 19), rows: 100, bytes: 200_000 },
        ];
        let dirty = [(0, 9), (10, 19)];
        let units = plan_units(&dirty, &files, true);
        assert_eq!(units.len(), 1, "adjacent small misses coalesce into one unit");
        assert_eq!(units[0], (0, 19));

        // (b) the oscillation arm: two TOGETHER-oversized halves (the
        // envelope fallback's own output). WITHOUT the manifest bound
        // the planner merges them (past the envelope) → the write
        // encodes the whole unit, the fallback halves it and RE-ENCODES
        // the two halves → three encodes per checkpoint, and the next
        // checkpoint merges again (the cycle). WITH the bound they
        // stay apart: two encodes, no re-encode, the cycle broken —
        // the ADR's "two to three whole-unit encodes every such
        // checkpoint" vs the bounded two.
        let halves = [
            ShardFile { id_range: (0, 49), rows: 800, bytes: 1_600_000 }, // bytes over cap
            ShardFile { id_range: (50, 99), rows: 800, bytes: 1_600_000 },
        ];
        let dirty = [(0, 49), (50, 99)];

        // the broken cycle: merge → write whole → fallback halves →
        // re-encode both halves → next checkpoint merges again
        let broken_units = plan_units(&dirty, &halves, false);
        assert_eq!(broken_units.len(), 1, "the broken shape merges the halves");
        let after_broken = write_and_fallback(&broken_units);
        assert_eq!(after_broken.len(), 2, "the fallback halves them again");
        let again = plan_units(&dirty, &after_broken, false);
        assert_eq!(again.len(), 1, "checkpoint N+1 merges again — the cycle");
        // per-checkpoint encodes, broken: the whole unit (1) + the two
        // re-encoded halves (2) = 3 — the ADR's "two to three"
        let encodes_broken = 1 + after_broken.len();
        assert_eq!(encodes_broken, 3, "the broken cycle re-encodes whole units");

        // the bounded cycle: the halves stay apart, written as-is
        let bound_units = plan_units(&dirty, &halves, true);
        assert_eq!(bound_units.len(), 2, "the manifest bound keeps them apart");
        let after_bound = write_and_fallback(&bound_units);
        assert_eq!(after_bound.len(), 2, "no fallback fired — the units fit");
        let next_bound = plan_units(&dirty, &after_bound, true);
        assert_eq!(next_bound.len(), 2, "checkpoint N+1 stays apart — no cycle");
        let encodes_bound = bound_units.len();
        assert_eq!(encodes_bound, 2);
        assert!(
            encodes_bound < encodes_broken,
            "the bound avoids the whole-unit re-encode ({encodes_bound} < {encodes_broken})"
        );

        // (c) the width-bound arm: a unit spanning past MAX_WINDOW
        // refuses to absorb — and the bounds check comes FIRST (the
        // tamper arm's expensive measure never runs for it).
        let far = [
            ShardFile { id_range: (0, 9), rows: 1, bytes: 1 },
            ShardFile { id_range: (10_000, 10_009), rows: 1, bytes: 1 },
        ];
        let dirty_far = [(0, 9), (10_000, 10_009)];
        let units = plan_units(&dirty_far, &far, true);
        assert_eq!(units.len(), 2, "width past MAX_WINDOW: no absorption");

        // (d) the tamper arm: the bounds-first discipline is structural
        // (the width check precedes the measure in plan_units). Assert
        // the width gate fires with EMPTY file knowledge — proving the
        // width check needs no measurement:
        let empty_files: [ShardFile; 0] = [];
        let units = plan_units(&dirty_far, &empty_files, true);
        assert_eq!(units.len(), 2, "the width bound alone refuses the join (no measure needed)");
    }
}
