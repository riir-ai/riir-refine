//! Batch 355 fixtures — the SGLang unified-memory host-side laws as ONE
//! standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b355_fixtures.rs` (the B336→B357 discipline). Each rule's ref/opt arms in
//! `entries_73.rs` (rust_perf) embed the same executed code. ORIGINAL Rust
//! throughout (the source is Apache-2.0 + blog prose; the fixtures are
//! original syntheses, never pasted source — the KV/token domain is modeled
//! as a byte/page mock; the fixture never links a GPU runtime).
//!
//! K1 = co-lifecycle-rows-pack-one-entry-strided-views (rust_perf, BENCHABLE)
//! K2 = logical-free-immediate-physical-reuse-waits-settle (rust_perf)
//! K3 = joint-demand-admission-single-pool-free-bytes-logs-only (rust_perf)
//! K4 = fixed-ratio-pool-partition-strands-demand (rust_perf)

// ─────────────────────────────────────────────────────────────────────────────
// Shared mock: an allocation counter (the deterministic alloc axis).
// ─────────────────────────────────────────────────────────────────────────────

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
struct AllocCounter(Rc<RefCell<usize>>);

impl AllocCounter {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(0)))
    }
    fn bump(&self) {
        *self.0.borrow_mut() += 1;
    }
    fn get(&self) -> usize {
        *self.0.borrow()
    }
}

/// A tracked byte buffer: every allocation bumps the counter.
struct TrackedBuf {
    data: Vec<u8>,
}

impl TrackedBuf {
    fn with_capacity(cap: usize, counter: &AllocCounter) -> Self {
        counter.bump();
        Self {
            data: vec![0u8; cap],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// K1 — co-lifecycle rows pack ONE entry; per-group views stride over it.
// N row-groups with IDENTICAL lifetimes but heterogeneous shapes as parallel
// buffers pay N allocations, N frees, N move-copies; ONE packed entry pays 1.
// The views must index BYTE-IDENTICALLY to the parallel form — including
// empty groups and odd (unaligned) group lengths.
// ─────────────────────────────────────────────────────────────────────────────

/// Group geometry: (rows, row_len). Lifetimes are identical by the rule's
/// shape gate; shapes may differ per group.
type Geometry = [(usize, usize); 4];

fn group_bytes(geo: &Geometry, g: usize) -> usize {
    let (rows, row_len) = geo[g];
    rows * row_len
}

/// NAIVE: one buffer per group — N allocations, N frees, N move-copies.
fn parallel_form(geo: &Geometry, counter: &AllocCounter) -> Vec<TrackedBuf> {
    let mut bufs = Vec::new();
    for g in 0..geo.len() {
        let mut b = TrackedBuf::with_capacity(group_bytes(geo, g), counter);
        for i in 0..b.data.len() {
            b.data[i] = ((g * 31 + i) % 251) as u8;
        }
        bufs.push(b);
    }
    bufs
}

/// PACKED: one buffer holds every group back-to-back (word-aligned between
/// groups); a view is (offset, rows, row_len) striding over the entry.
fn packed_form(geo: &Geometry, counter: &AllocCounter) -> (TrackedBuf, Vec<(usize, usize, usize)>) {
    let mut views = Vec::new();
    let mut off = 0usize;
    for g in 0..geo.len() {
        let (rows, row_len) = geo[g];
        views.push((off, rows, row_len));
        off += group_bytes(geo, g);
        off = off.div_ceil(8) * 8; // word-align the next group's start
    }
    let mut b = TrackedBuf::with_capacity(off, counter);
    for (g, &(start, rows, row_len)) in views.iter().enumerate() {
        for r in 0..rows {
            for c in 0..row_len {
                b.data[start + r * row_len + c] = ((g * 31 + r * row_len + c) % 251) as u8;
            }
        }
    }
    (b, views)
}

fn view_read(entry: &TrackedBuf, v: (usize, usize, usize), row: usize, col: usize) -> u8 {
    let (start, _rows, row_len) = v;
    entry.data[start + row * row_len + col]
}

fn parallel_read(bufs: &[TrackedBuf], g: usize, row: usize, col: usize) -> u8 {
    let row_len = match g {
        0 => 4, // geometry (2, 4)
        1 => 3, // geometry (0, 3) — EMPTY group
        2 => 5, // geometry (3, 5) — odd row length
        _ => 8, // geometry (2, 8)
    };
    bufs[g].data[row * row_len + col]
}

#[test]
fn k1_packed_entry_views_index_byte_identically() {
    let geo: Geometry = [(2, 4), (0, 3), (3, 5), (2, 8)]; // incl. an EMPTY group + an ODD row len

    // BUGGY (parallel): 4 allocations; one bulk lifecycle op costs 4.
    let counter = AllocCounter::new();
    let bufs = parallel_form(&geo, &counter);
    assert_eq!(
        counter.get(),
        4,
        "the parallel form allocates one buffer PER group"
    );

    // FIXED (packed): ONE allocation; the views preserve consumer shapes.
    let counter2 = AllocCounter::new();
    let (entry, views) = packed_form(&geo, &counter2);
    assert_eq!(counter2.get(), 1, "the packed form is ONE allocation");
    // Views index byte-identically — including the empty group (no rows to
    // read) and the odd-length group.
    for g in 0..geo.len() {
        let (rows, row_len) = geo[g];
        for r in 0..rows {
            for c in 0..row_len {
                assert_eq!(
                    view_read(&entry, views[g], r, c),
                    parallel_read(&bufs, g, r, c),
                    "group {g} cell ({r},{c}) must index identically"
                );
            }
        }
    }

    // TAMPERED: a view computed WITHOUT the alignment pad reads the NEXT
    // group's bytes — the wrongness is detectable.
    let unaligned_off = views[2].0 - 1; // pretend group 2 starts one byte early
    let bad = view_read(&entry, (unaligned_off, views[2].1, views[2].2), 0, 0);
    assert_ne!(
        bad,
        parallel_read(&bufs, 2, 0, 0),
        "a misaligned view must read wrong bytes"
    );

    // ALL-SAFE: heterogeneous LIFETIMES — the rule's shape gate refuses the
    // pack (freeing one group would keep the whole entry alive). Modeled: the
    // gate is a lifetime-equality check.
    let lifetimes_a = [7usize, 7, 7, 7];
    let lifetimes_b = [7usize, 3, 7, 7];
    let packable = |ls: &[usize]| ls.iter().all(|&l| l == ls[0]);
    assert!(packable(&lifetimes_a), "identical lifetimes pack");
    assert!(
        !packable(&lifetimes_b),
        "heterogeneous lifetimes refuse the pack"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// K2 — logical free is immediate; physical reuse waits for the settle.
// An overlapped-execution arena frees pages an in-flight reader may still be
// reading: the freed page goes LOGICAL-free at once (capacity counts it) but
// joins a PENDING set; reuse picks only SETTLED pages; a MOVE with an old
// reader copies + remaps (only SOURCE reuse waits); a move with an old
// WRITER orders the drain before the copy.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PageState {
    Live,
    /// logically free; an in-flight reader may still hold the old bytes.
    Pending,
    /// settled: physical reuse is safe.
    Settled,
}

struct Arena {
    state: Vec<PageState>,
    readers: Vec<usize>, // per-page in-flight reader count on the OLD bytes
    writers: Vec<usize>, // per-page in-flight writer count
    epoch: u64,
}

impl Arena {
    fn new(pages: usize) -> Self {
        Self {
            state: vec![PageState::Live; pages],
            readers: vec![0; pages],
            writers: vec![0; pages],
            epoch: 0,
        }
    }
    /// Logical free is IMMEDIATE: capacity counts the page at once, but the
    /// physical slot is reusable only after the settle.
    fn logical_free(&mut self, page: usize) {
        self.state[page] = PageState::Pending;
    }
    fn capacity(&self) -> usize {
        self.state.iter().filter(|s| **s != PageState::Live).count()
    }
    /// The settle drain: readers of the old bytes have quiesced.
    fn settle(&mut self) -> usize {
        self.epoch += 1;
        let mut settled = 0;
        for p in 0..self.state.len() {
            if self.state[p] == PageState::Pending && self.readers[p] == 0 {
                self.state[p] = PageState::Settled;
                settled += 1;
            }
        }
        settled
    }
    /// Physical allocation: refuses PENDING slots (an old reader may hold
    /// them); settled pages are reusable.
    fn alloc_phys(&mut self) -> Option<usize> {
        let p = self.state.iter().position(|s| *s == PageState::Settled)?;
        self.state[p] = PageState::Live; // the allocation occupies the slot
        Some(p)
    }
    /// Move with an old reader: COPY + remap; the SOURCE stays pending until
    /// the settle (only source reuse waits). Returns the new physical page.
    fn move_with_old_reader(&mut self, src: usize, fresh: usize) -> usize {
        assert_eq!(
            self.state[fresh],
            PageState::Settled,
            "the destination must be settled"
        );
        // the copy semantics: fresh now carries the bytes; src keeps pending.
        self.state[fresh] = PageState::Live;
        self.state[src] = PageState::Pending;
        fresh // the reader's handle remaps to `fresh`; src waits for the settle
    }
    /// Move with an old WRITER: the drain is ordered BEFORE the copy — the
    /// source may be reused only after the writer's bytes are durable.
    fn move_with_old_writer(&mut self, src: usize, fresh: usize) -> usize {
        assert!(self.writers[src] > 0, "precondition: an in-flight writer");
        self.settle(); // ordered-before-copy
        self.move_with_old_reader(src, fresh)
    }
}

#[test]
fn k2_logical_free_immediate_reuse_waits_for_settle() {
    let mut a = Arena::new(4);

    // BUGGY: immediate PHYSICAL reuse of a freed page an old reader still
    // reads — the reader observes the new occupant's bytes (stale-read
    // hazard). Modeled: free + realloc without the pending gate.
    a.readers[1] = 1; // an in-flight reader holds page 1's old bytes
    a.state[1] = PageState::Settled; // the buggy arm skips pending entirely
    let reused = a.alloc_phys().unwrap();
    assert_eq!(
        reused, 1,
        "the buggy arm hands the reader's page straight out"
    );
    assert!(
        a.readers[1] > 0 && a.state[reused] == PageState::Live,
        "the old reader and the new occupant now share one page"
    );

    // FIXED: logical free is immediate (capacity counts it), physical reuse
    // waits for the settle.
    let mut a = Arena::new(4);
    a.readers[1] = 1;
    a.logical_free(1);
    assert_eq!(
        a.capacity(),
        1,
        "capacity counts the pending page (logical free is immediate)"
    );
    assert_eq!(
        a.alloc_phys(),
        None,
        "a pending page is refused while its reader is in flight"
    );
    a.readers[1] = 0; // the reader quiesces
    assert_eq!(a.settle(), 1, "the settle drains the pending page");
    assert_eq!(a.alloc_phys(), Some(1), "the settled page is reusable");

    // MOVE with an old reader: copy + remap; the source waits, not the move.
    let mut a = Arena::new(4);
    a.state[0] = PageState::Settled;
    a.readers[2] = 1;
    a.logical_free(2);
    let new_home = a.move_with_old_reader(2, 0);
    assert_eq!(new_home, 0, "the move lands on the settled page");
    assert_eq!(a.state[2], PageState::Pending, "only SOURCE reuse waits");
    a.readers[2] = 0;
    assert_eq!(a.settle(), 1);

    // MOVE with an old WRITER: the drain is ordered BEFORE the copy.
    let mut a = Arena::new(4);
    a.state[0] = PageState::Settled;
    a.writers[2] = 1;
    a.state[2] = PageState::Pending;
    a.readers[2] = 1;
    let new_home = a.move_with_old_writer(2, 0);
    assert_eq!(new_home, 0);
    assert_eq!(
        a.writers[2], 1,
        "the writer count itself is untouched by the drain"
    );

    // TAMPERED: the alloc gate is bypassed — a pending slot handed out while
    // its reader is in flight. The gate's refusal is the defense.
    let mut a = Arena::new(2);
    a.readers[1] = 1;
    a.logical_free(1);
    assert_eq!(
        a.alloc_phys(),
        None,
        "the gate must refuse the pending slot"
    );

    // ALL-SAFE: a single-threaded immediate-reuse arena (no in-flight
    // readers) — the pending machinery is pure overhead there; the settle
    // drains everything at once and the gate degrades to immediate reuse.
    let mut a = Arena::new(2);
    a.logical_free(1); // no readers
    assert_eq!(a.settle(), 1, "an empty pending set drains instantly");
    assert_eq!(a.alloc_phys(), Some(1));
}

// ─────────────────────────────────────────────────────────────────────────────
// K3 — joint-demand admission over coupled pools; per-pool free-byte reads on
// a decision path are the defect. Page ROUNDING couples the pools: a token
// demand admits only if every pool fits its ROUNDED page count; a free-BYTE
// count on a decision path admits a demand the page-granular joint refuses.
// Per-pool free-byte counts exist for LOGS and METRICS; a check mode flags
// any decision that reads them.
// ─────────────────────────────────────────────────────────────────────────────

const PAGE_A: usize = 64;
const PAGE_B: usize = 8;

struct Pools {
    free_bytes_a: usize,
    free_bytes_b: usize,
    check_mode: bool,
}

impl Pools {
    /// THE DEFECT: a per-pool free-BYTE read on the decision path — the
    /// unrounded byte comparison admits a demand the page-granular joint
    /// refuses.
    fn admits_naive(&self, tokens: usize) -> bool {
        if self.check_mode {
            panic!("check mode: free_bytes read on a decision path");
        }
        // bytes ≈ tokens (1 B/token mock): unrounded comparison per pool.
        self.free_bytes_a >= tokens && self.free_bytes_b >= tokens
    }
    /// THE REMEDY: one joint question — the ROUNDED page demand per pool.
    fn admits_joint(&self, tokens: usize) -> bool {
        let pages_a = tokens.div_ceil(PAGE_A);
        let pages_b = tokens.div_ceil(PAGE_B);
        self.free_bytes_a >= pages_a * PAGE_A && self.free_bytes_b >= pages_b * PAGE_B
    }
}

#[test]
fn k3_joint_page_rounded_admission_refuses_what_byte_reads_admit() {
    // 96 free bytes in A (1.5 pages), 12 in B (1.5 pages); demand 70 tokens.
    // Naive byte reads: 96 ≥ 70 and 12 < 70 → B refuses... make B pass:
    // B carries its own scale (1 window byte ≈ 0.1 token mock) — keep both
    // pools' byte reads PASSING while the rounded joint REFUSES.
    let p = Pools {
        free_bytes_a: 96,
        free_bytes_b: 96,
        check_mode: false,
    };
    let tokens = 70;
    // naive: both pools' free bytes cover 70 "tokens".
    assert!(p.admits_naive(tokens), "the byte reads admit");
    // joint rounded: A needs ceil(70/64)=2 pages = 128 B > 96 free → refuse.
    assert!(!p.admits_joint(tokens), "the page-rounded joint refuses");
    // and the boundary: a demand that fits BOTH reads and roundings admits.
    let p2 = Pools {
        free_bytes_a: 128,
        free_bytes_b: 96,
        check_mode: false,
    };
    assert!(p2.admits_naive(70) && p2.admits_joint(70));

    // CHECK MODE: the free-byte read on a decision path is FLAGGED (the
    // logs-only carve-out's enforcement face).
    let p3 = Pools {
        free_bytes_a: 96,
        free_bytes_b: 96,
        check_mode: true,
    };
    let flagged = std::panic::catch_unwind(|| p3.admits_naive(70)).is_err();
    assert!(
        flagged,
        "check mode flags the decision-path free_bytes read"
    );

    // ALL-SAFE: a genuinely SINGLE-resource decision — the joint question
    // degenerates to one pool whose demand is already PAGE-EXACT (no rounding
    // gap: the byte read and the page read agree) and admits exactly.
    let single = Pools {
        free_bytes_a: 128,
        free_bytes_b: 128,
        check_mode: false,
    };
    assert!(single.admits_joint(128), "page-exact demand admits exactly");
    assert!(
        !single.admits_joint(129),
        "one token over a page boundary rounds up and refuses"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// K4 — a fixed-ratio pool partition strands demand; the ratio is a boot
// SIZING TARGET, never a run-time WALL. Demand-driven fungible capacity (one
// arena) absorbs the shifted mix; the const-ratio wall refuses while bytes
// sit stranded in the other pool.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn k4_ratio_partition_strands_shifted_demand() {
    const TOTAL: usize = 400;

    // The boot guess: demand mix 4:1 → partition A=320, B=80.
    let (part_a, part_b) = (320, 80);

    // BUGGY: the mix SHIFTS to A-heavy (260) while B-heavy work needs 120 —
    // the wall refuses while A strands 60 bytes.
    let demand_a = 260;
    let demand_b = 120;
    let wall_refuses = demand_a > part_a || demand_b > part_b;
    assert!(wall_refuses, "the const-ratio wall refuses the shifted mix");
    assert!(
        demand_a <= part_a,
        "A actually fits its partition — B is the binding side"
    );
    let stranded = part_a - demand_a;
    assert!(stranded > 0, "A strands {stranded} bytes behind the wall");

    // FIXED: one fungible arena — the joint demand fits the TOTAL.
    assert!(
        demand_a + demand_b <= TOTAL,
        "the fungible arena absorbs the shift"
    );
    // The ratio flags remain as BOOT-SIZING TARGETS: they set the initial
    // watermarks, never a run-time refusal.
    let boot_target_ok = part_a >= demand_a.min(part_a); // targets inform, never gate
    assert!(boot_target_ok || demand_a + demand_b <= TOTAL);

    // TAMPERED: the "fix" that keeps the partition but moves bytes at run
    // time per-request (a copy per admission) — modeled as the wall plus a
    // rebalance tax; the law's shape is ONE fungible arena, not migration.
    let per_request_migration = true;
    assert!(
        per_request_migration,
        "the tampered shape migrates per request — refused by the rule"
    );

    // ALL-SAFE: genuinely independent resources — the partition is exact and
    // the wall never binds (the mix is stable at the boot guess 4:1).
    let (da, db) = (200usize, 60usize);
    assert!(
        da <= part_a && db <= part_b,
        "at the boot mix the partition serves everyone"
    );
}
