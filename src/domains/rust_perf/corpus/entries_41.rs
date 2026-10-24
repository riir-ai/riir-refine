//! Entries slice C — Batch 270 (2026-10-21, Plan 274 / Research 306): the
//! splash-wave serving pool, slice C (allocation + concurrency + dispatch) —
//! incoai/splash @ 9ca9768a323048f3ec4d586bb2e2e99947b4c1d4 +
//! f6957e847ad63b4d87aa1c1f5d9f3f8378e09f25 + 32cf878d434c44933b5ede50ccdaf7d23fe271d7,
//! fdc8f01d75c4bc116dcb967a58dcb4ab5078a400 + c27aaf7527a33a1085f8d68caa8d19951ce7164b +
//! e90b43c3f20555b576ca86b9ae2fb9ce802dbd29 + a7406bbc685e8216e0a6a220a1302034bc76c1ae +
//! 41e8c82cb8afdfd0cb0f78bd537c8591d9819b9b, 8df9e95a17b3f156cc75d88e5dc0c429670ff235,
//! Apache-2.0; every quote re-verified at the .raw/b270/splash clone this batch.
//! The repo's SECOND distillation (Batch 237 took the governor's growth axis,
//! `entries_26` @ ffc2d4be). All three fixtures are ORIGINAL machinery re-authored
//! from the source shapes (never splash code), extracted to
//! .`raw/b270/fix_<ruleslug>_{buggy,fixed}.rs` and validated standalone
//! (rustc --edition 2021 --test, both arms, all tests green) before embedding.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── allocation (retry pacing): a refused allocation's retry keys on the
        // resource class the refuser refused for ACTUALLY returning — an extent's
        // bytes or a state's buffers — never on per-entry eviction progress:
        // thousands of leaves sample the host before the same refusal comes back
        // ──────────────────────────────────────────────────────────────────────
        // Source: incoai/splash 9ca9768 + f6957e8 + 32cf878, the commit messages,
        // runtime/engine/Engine.cpp:2024-2035 + Cache.cpp:616-644 + Cache.hpp:607-616.
        PerfRule {
            rule_id: "reclaim-progress-keyed-on-refused-resource-returned".to_string(),
            name: "An allocation retry loop that asks the refuser again after every reclaim step which EVICTED something is keyed on the wrong progress class: one refused request then drives thousands of evictions, each paying a host sample, before the same refusal comes back — 'One refused hot block cache extent or staging could so evict thousands of leaves, sampling the host's memory before each retry, and then yield all the same.' The law: the retry keys on the REFUSED RESOURCE CLASS actually returning — 'The allocation is tried again only after memory came back' (an extent's bytes, or a state's buffers); eviction that returns no memory is not retry-worthy ('KV gone from an extent that stays gives the governor nothing'), the reclaim evicts only while the class could still empty an extent (extentWithinReach), pages the grant point never needs stay put, and a refuser that will refuse regardless is never handed rebuildable caches first ('Freeing a cache does not get the allocation past the host before macOS acknowledges the bytes').".to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Provenance: ORIGINAL machinery re-authored from incoai/splash commits
// 9ca9768 + f6957e8 + 32cf878 (Apache-2.0): a memory-governor allocation
// retry loop over an extent-keyed page cache. The host is the OS-governor
// stand-in: it refuses an allocation while its own pressure holds and
// credits back only what the cache RELEASES to it. Never splash code.

/// Bytes one page holds; an extent holds `EXTENT_PAGES` of them and
/// releases its bytes to the host only when it EMPTIES.
pub const PAGE_BYTES: u64 = 100;
pub const EXTENT_PAGES: u32 = 4;
pub const EXTENT_BYTES: u64 = PAGE_BYTES * EXTENT_PAGES as u64;
/// An Index page holds a fraction of a Full page's bytes; the fixture's
/// fraction, credited only while its extent stays partial.
pub const INDEX_BYTES: u64 = PAGE_BYTES / 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Leaf,
    Index,
}

#[derive(Clone, Copy)]
pub struct Page {
    pub id: u32,
    pub form: Form,
    pub extent: u32,
    /// eviction order: the lowest seq goes first, across forms
    pub seq: u64,
    /// in the cache (a victim's flag clears; the flag never re-sets)
    pub cached: bool,
}

/// The host: it REFUSES while its pressure holds, and every ask samples
/// it - the source's "sampling the host's memory before each retry".
pub struct Host {
    pub requirement: u64,
    released: u64,
    pub probes: u32,
}

impl Host {
    pub fn new(requirement: u64) -> Self {
        Self {
            requirement,
            released: 0,
            probes: 0,
        }
    }

    pub fn try_grant(&mut self) -> bool {
        self.probes += 1;
        self.released >= self.requirement
    }

    pub fn credit(&mut self, bytes: u64) {
        self.released += bytes;
    }
}

pub struct Step {
    pub progressed: bool,
    pub released_bytes: u64,
}

pub struct Cache {
    pages: Vec<Page>,
    resident_per_extent: Vec<u32>,
}

impl Cache {
    pub fn new(pages: Vec<Page>, extents: u32) -> Self {
        let mut resident_per_extent = vec![0u32; extents as usize];
        for p in &pages {
            if p.cached {
                resident_per_extent[p.extent as usize] += 1;
            }
        }
        Self {
            pages,
            resident_per_extent,
        }
    }

    /// Evicting a Leaf can still empty an extent; an Index page holds a
    /// fraction of a page and counts toward no Full extent.
    pub fn extent_within_reach(&self) -> bool {
        self.pages.iter().any(|p| p.cached && p.form == Form::Leaf)
    }

    /// One eviction, oldest first across forms. A leaf inside an extent
    /// that stays partial credits the host NOTHING (the extent's bytes
    /// are not returned until it empties); an Index page credits its
    /// fraction; an emptied extent releases its bytes.
    pub fn evict_one(&mut self, host: &mut Host) -> Step {
        let victim = self
            .pages
            .iter()
            .filter(|p| p.cached)
            .min_by_key(|p| p.seq)
            .map(|p| (p.id, p.form, p.extent));
        let Some((id, form, extent)) = victim else {
            return Step {
                progressed: false,
                released_bytes: 0,
            };
        };
        let page = self
            .pages
            .iter_mut()
            .find(|p| p.id == id)
            .expect("victim present");
        page.cached = false;
        self.resident_per_extent[extent as usize] -= 1;
        if self.resident_per_extent[extent as usize] == 0 {
            host.credit(EXTENT_BYTES);
            Step {
                progressed: true,
                released_bytes: EXTENT_BYTES,
            }
        } else if form == Form::Index {
            host.credit(INDEX_BYTES);
            Step {
                progressed: true,
                released_bytes: INDEX_BYTES,
            }
        } else {
            Step {
                progressed: true,
                released_bytes: 0,
            }
        }
    }

    pub fn resident(&self, form: Form) -> u32 {
        self.pages
            .iter()
            .filter(|p| p.cached && p.form == form)
            .count() as u32
    }
}

/// THE DEFECT: the allocation is asked for again after every reclaim
/// step that made progress - one entry per step - so the host is
/// sampled once per eviction while it refuses all the same, and the
/// round-robin falls onto pages the grant point never needed.
pub fn allocate(cache: &mut Cache, host: &mut Host) -> bool {
    loop {
        let step = cache.evict_one(host);
        if !step.progressed {
            return false; // nothing left to reclaim: the lane yields
        }
        if host.try_grant() {
            return true;
        }
    }
}

/// One extent of leaves + one extent of Index pages, oldest first.
pub fn fixture() -> (Cache, Host) {
    let pages = vec![
        Page { id: 0, form: Form::Leaf, extent: 0, seq: 0, cached: true },
        Page { id: 1, form: Form::Leaf, extent: 0, seq: 1, cached: true },
        Page { id: 2, form: Form::Leaf, extent: 0, seq: 2, cached: true },
        Page { id: 3, form: Form::Leaf, extent: 0, seq: 3, cached: true },
        Page { id: 4, form: Form::Index, extent: 1, seq: 4, cached: true },
        Page { id: 5, form: Form::Index, extent: 1, seq: 5, cached: true },
        Page { id: 6, form: Form::Index, extent: 1, seq: 6, cached: true },
        Page { id: 7, form: Form::Index, extent: 1, seq: 7, cached: true },
    ];
    (Cache::new(pages, 2), Host::new(EXTENT_BYTES))
}

/// The outcome contract: granted iff the oldest-first eviction set
/// releases at least `requirement` bytes. Shared with the paced arm.
pub fn outcome_contract(cache: &Cache, requirement: u64) -> bool {
    let mut releasable = 0u64;
    // an extent releases its bytes iff every page cached in it is a Leaf
    // (Index residents keep their extent partial forever, in this shape)
    for extent in 0..cache.resident_per_extent.len() {
        let residents: Vec<_> = cache
            .pages
            .iter()
            .filter(|p| p.cached && p.extent as usize == extent)
            .collect();
        if residents.is_empty() {
            releasable += EXTENT_BYTES; // already released
        } else if residents.iter().all(|p| p.form == Form::Leaf)
            && residents.len() == EXTENT_PAGES as usize
        {
            // releasable by draining these leaves
            releasable += EXTENT_BYTES;
        }
    }
    releasable >= requirement
}

#[test]
fn grants_at_the_same_extent_and_keeps_the_same_pages() {
    // one extent of leaves covers the requirement: both pacing laws
    // grant at the same eviction (the fourth leaf empties extent 0),
    // same evicted set, same surviving pages - only the ask count moves
    let (mut cache, mut host) = fixture();
    assert!(allocate(&mut cache, &mut host));
    assert_eq!(host.probes, 4); // one ask PER EVICTION
    assert_eq!(cache.resident(Form::Leaf), 0);
    assert_eq!(cache.resident(Form::Index), 4); // the grant came first
    assert!(outcome_contract(&cache, EXTENT_BYTES));
}

#[test]
fn a_refusal_that_never_lifts_samples_per_eviction_then_yields() {
    // requirement beyond every releasable byte: the loop asks after
    // EVERY step - 8 evictions, 8 samples - and the Index pages fall
    // to the round-robin before the yield
    let (mut cache, mut host) = fixture();
    host.requirement = 12_000;
    assert!(!allocate(&mut cache, &mut host));
    assert_eq!(host.probes, 8);
    assert_eq!(cache.resident(Form::Leaf), 0);
    assert_eq!(cache.resident(Form::Index), 0); // the collateral
    assert!(!outcome_contract(&cache, 12_000));
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Provenance: ORIGINAL machinery re-authored from incoai/splash commits
// 9ca9768 + f6957e8 + 32cf878 (Apache-2.0): the PACED arm - the
// allocation is asked for again only after memory came back (an
// extent's bytes or a state's buffers), never after a bare eviction
// that returned nothing. Never splash code.

/// Bytes one page holds; an extent holds `EXTENT_PAGES` of them and
/// releases its bytes to the host only when it EMPTIES.
pub const PAGE_BYTES: u64 = 100;
pub const EXTENT_PAGES: u32 = 4;
pub const EXTENT_BYTES: u64 = PAGE_BYTES * EXTENT_PAGES as u64;
/// An Index page holds a fraction of a Full page's bytes; the fixture's
/// fraction, credited only while its extent stays partial.
pub const INDEX_BYTES: u64 = PAGE_BYTES / 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Leaf,
    Index,
}

#[derive(Clone, Copy)]
pub struct Page {
    pub id: u32,
    pub form: Form,
    pub extent: u32,
    /// eviction order: the lowest seq goes first, across forms
    pub seq: u64,
    /// in the cache (a victim's flag clears; the flag never re-sets)
    pub cached: bool,
}

/// The host: it REFUSES while its pressure holds, and every ask samples
/// it - the source's "sampling the host's memory before each retry".
pub struct Host {
    pub requirement: u64,
    released: u64,
    pub probes: u32,
}

impl Host {
    pub fn new(requirement: u64) -> Self {
        Self {
            requirement,
            released: 0,
            probes: 0,
        }
    }

    pub fn try_grant(&mut self) -> bool {
        self.probes += 1;
        self.released >= self.requirement
    }

    pub fn credit(&mut self, bytes: u64) {
        self.released += bytes;
    }
}

pub struct Step {
    pub progressed: bool,
    pub released_bytes: u64,
}

pub struct Cache {
    pages: Vec<Page>,
    resident_per_extent: Vec<u32>,
}

impl Cache {
    pub fn new(pages: Vec<Page>, extents: u32) -> Self {
        let mut resident_per_extent = vec![0u32; extents as usize];
        for p in &pages {
            if p.cached {
                resident_per_extent[p.extent as usize] += 1;
            }
        }
        Self {
            pages,
            resident_per_extent,
        }
    }

    /// Evicting a Leaf can still empty an extent; an Index page holds a
    /// fraction of a page and counts toward no Full extent.
    pub fn extent_within_reach(&self) -> bool {
        self.pages.iter().any(|p| p.cached && p.form == Form::Leaf)
    }

    /// One eviction, oldest first across forms. A leaf inside an extent
    /// that stays partial credits the host NOTHING (the extent's bytes
    /// are not returned until it empties); an Index page credits its
    /// fraction; an emptied extent releases its bytes.
    pub fn evict_one(&mut self, host: &mut Host) -> Step {
        let victim = self
            .pages
            .iter()
            .filter(|p| p.cached)
            .min_by_key(|p| p.seq)
            .map(|p| (p.id, p.form, p.extent));
        let Some((id, form, extent)) = victim else {
            return Step {
                progressed: false,
                released_bytes: 0,
            };
        };
        let page = self
            .pages
            .iter_mut()
            .find(|p| p.id == id)
            .expect("victim present");
        page.cached = false;
        self.resident_per_extent[extent as usize] -= 1;
        if self.resident_per_extent[extent as usize] == 0 {
            host.credit(EXTENT_BYTES);
            Step {
                progressed: true,
                released_bytes: EXTENT_BYTES,
            }
        } else if form == Form::Index {
            host.credit(INDEX_BYTES);
            Step {
                progressed: true,
                released_bytes: INDEX_BYTES,
            }
        } else {
            Step {
                progressed: true,
                released_bytes: 0,
            }
        }
    }

    pub fn resident(&self, form: Form) -> u32 {
        self.pages
            .iter()
            .filter(|p| p.cached && p.form == form)
            .count() as u32
    }
}

/// The reclaim step for memory the governor funds: evicts while the
/// allocation's class could still empty an extent, until memory has
/// come back - an extent's bytes (here; a state's buffers in the full
/// shape). KV gone from an extent that stays still gives the governor
/// nothing, so it is not "memory back".
fn reclaim_for_memory(cache: &mut Cache, host: &mut Host) -> bool {
    while cache.extent_within_reach() {
        let step = cache.evict_one(host);
        if !step.progressed {
            return false;
        }
        if step.released_bytes >= EXTENT_BYTES {
            return true; // memory came back: an extent's bytes
        }
    }
    false // only a state would go next; none in this fixture
}

/// THE LAW: the allocation is asked for again ONLY after memory came
/// back - the ask rides the extent release, never the eviction count.
pub fn allocate(cache: &mut Cache, host: &mut Host) -> bool {
    loop {
        if reclaim_for_memory(cache, host) {
            if host.try_grant() {
                return true;
            }
            continue; // memory came back, not enough: reclaim again
        }
        return false;
    }
}

/// One extent of leaves + one extent of Index pages, oldest first.
pub fn fixture() -> (Cache, Host) {
    let pages = vec![
        Page { id: 0, form: Form::Leaf, extent: 0, seq: 0, cached: true },
        Page { id: 1, form: Form::Leaf, extent: 0, seq: 1, cached: true },
        Page { id: 2, form: Form::Leaf, extent: 0, seq: 2, cached: true },
        Page { id: 3, form: Form::Leaf, extent: 0, seq: 3, cached: true },
        Page { id: 4, form: Form::Index, extent: 1, seq: 4, cached: true },
        Page { id: 5, form: Form::Index, extent: 1, seq: 5, cached: true },
        Page { id: 6, form: Form::Index, extent: 1, seq: 6, cached: true },
        Page { id: 7, form: Form::Index, extent: 1, seq: 7, cached: true },
    ];
    (Cache::new(pages, 2), Host::new(EXTENT_BYTES))
}

/// The outcome contract: granted iff the oldest-first eviction set
/// releases at least `requirement` bytes. Shared with the paced arm.
pub fn outcome_contract(cache: &Cache, requirement: u64) -> bool {
    let mut releasable = 0u64;
    for extent in 0..cache.resident_per_extent.len() {
        let residents: Vec<_> = cache
            .pages
            .iter()
            .filter(|p| p.cached && p.extent as usize == extent)
            .collect();
        if residents.is_empty() {
            releasable += EXTENT_BYTES; // already released
        } else if residents.iter().all(|p| p.form == Form::Leaf)
            && residents.len() == EXTENT_PAGES as usize
        {
            // releasable by draining these leaves
            releasable += EXTENT_BYTES;
        }
    }
    releasable >= requirement
}

#[test]
fn the_ask_rides_the_extent_release_not_the_evictions() {
    // one extent of leaves covers the requirement: granted at the same
    // eviction as the per-step loop (the fourth leaf empties extent 0),
    // same evicted set, same surviving pages - ONE ask, not four
    let (mut cache, mut host) = fixture();
    assert!(allocate(&mut cache, &mut host));
    assert_eq!(host.probes, 1);
    assert_eq!(cache.resident(Form::Leaf), 0);
    assert_eq!(cache.resident(Form::Index), 4); // the grant came first
    assert!(outcome_contract(&cache, EXTENT_BYTES));
}

#[test]
fn a_refusal_that_never_lifts_asks_once_and_keeps_the_rebuildable_pages() {
    // requirement beyond every releasable byte: the drain stops at the
    // extent it released (Index pages can empty no Full extent), asks
    // once, and yields - the Index pages of the other chains STAY,
    // exactly the source's own disclosure ("the chain's Index pages
    // stay" vs "the Index pages fell from 31 to 6")
    let (mut cache, mut host) = fixture();
    host.requirement = 12_000;
    assert!(!allocate(&mut cache, &mut host));
    assert_eq!(host.probes, 1);
    assert_eq!(cache.resident(Form::Leaf), 0);
    assert_eq!(cache.resident(Form::Index), 4); // the class could empty no further extent
    assert!(!outcome_contract(&cache, 12_000));
}
"#.to_string(),
            context_notes: Some("Grade B+ — NotBenchable ON THIS BOX (a serving-economy pacing law over an OS memory governor; the evidence is soak telemetry on foreign silicon — M5 Max-class macOS, source-reported figures are PROVENANCE, never bars; the fixture's probe counter is the shape axis, not a latency bar). SOURCE (incoai/splash @ 9ca9768a323048f3ec4d586bb2e2e99947b4c1d4 + f6957e847ad63b4d87aa1c1f5d9f3f8378e09f25 + 32cf878d434c44933b5ede50ccdaf7d23fe271d7, Apache-2.0, read at the .raw/b270/splash clone; every quote verbatim this batch — the repo's SECOND distillation: Batch 237 took its growth axis, entries_26 @ ffc2d4be). THE LAW (9ca9768 message): 'Engine::allocate asked for such an allocation again after every reclaim step that made progress, and a step (Cache::reclaimOne) evicts one cache entry: a KV leaf gives the governor nothing until its extent empties, and an Index page holds a seventeenth of a Full page's bytes at Flash-Next's.' + 'One refused hot block cache extent or staging could so evict thousands of leaves, sampling the host's memory before each retry, and then yield all the same.' + the fix: 'The allocation is tried again only after memory came back; KV gone from an extent that stays still wakes the requests that wait for pages.' THE IN-CODE KEY (runtime/engine/Engine.cpp:2031-2033 at 9ca9768, verbatim): '// KV gone from an extent that stays gives the governor nothing: the' / '// allocation is asked for again only once memory has come back.' / 'reclaimed.madeProgress = reclaimed.reclaimedBytes || reclaimed.evictedState;' — the progress flag the retry loop reads is REWRITTEN to memory-backed classes only (the reclaim's own madeProgress still wakes page waiters). THE RECLAIM SHAPE (runtime/engine/Cache.cpp:616-644 at 9ca9768, reclaimForMemory): free pages first as the extent they fill, then 'the victims in reclaimOne's order while their KV can empty an extent' (the extentWithinReach(upTo) gate, now of the allocation's class), 'otherwise only a state' (reclaimStateForLane); its contract (Cache.hpp:607-616): 'The reclaim step for memory the governor funds that an allocation was refused.' MEASURED (source-reported, 9ca9768): 'In a 90-minute soak on a 36 GB M5 Max, one engine tick took 4.75 s over 3,242 refused reservations while the Index pages in memory fell by 3,160, and a lane was suspended at its end; every rise of the longest tick fell in a window of 700 to 3,250 refusals.' The engine test's own before/after: 'the floor is asked for at most four times and the chain's Index pages stay. Before, it was asked for 44 times, and the Index pages fell from 31 to 6.' THE HOST-REFUSAL HALF (f6957e8): 'Freeing a cache does not get the allocation past the host before macOS acknowledges the bytes, and while macOS warns not at all: that is why the reuse path evicts no cached KV or state it cannot reuse, and why the paced pass takes the model's caches, toward what the host lacks.' — the reuse path narrows to idle buffers ('The reuse path now frees idle buffers alone (IdleMemory::Buffers), for a lane's state, its KV pages and the readers of spilled KV alike') and reclaimIdleState loses its default scope, 'so that each caller names what it may free.' THE FALLBACK HALF (32cf878, runtime/engine/Engine.cpp:1635-1642): 'The fallback is now offered whatever refused the memory, and the allocation is attempted again as soon as it changes something' — 'so a state the start no longer pins is not evicted for nothing'; the lease drop 'acts only when judge() says Fail, which a host refusal never does', so a refused start KEEPS its lease and waits for the pressure to lift (the new testHostRefusedStartKeepsItsLease). REGIME: allocation retry loops over a governor/host that refuses while its own pressure holds, against a cache whose unit of RETURNABLE memory (an extent, a state's buffers) is larger than its unit of eviction (a leaf, a page): key the retry on the returnable unit. ANTI-REGIME: caches where every eviction returns memory to the refuser immediately (page-granular — per-step retry is exact there); refusers that charge nothing per sample (the probe cost is one axis, the cache collateral the other — a free in-process check does not fix the collateral); the PACED passes deliberately 'keep waiting for any transfer' (c27aaf7: 'A pass still waits for any transfer') — this law is for the REFUSED-allocation reclaim only. DISTINCT-FROM (all read this batch): discretionary-cache-growth-keys-on-spare-not-available (entries_26, B237 — the SAME repo, the GROWTH axis: which estimate a discretionary cache may grow on, spare vs available, plus the OS-warning stop; THIS is the REFUSAL-RETRY axis: what counts as progress after a refusal comes back — they compose: the governor refuses, the retry keys on release); cache-budget-first-sessions-take-remainder-never-reshrink (entries_37, B257) is STARTUP budget ordering under one cap — no retry loop, no refuser sampling; degrade-not-fail-load-qualification-probe (entries_21, B219) degrades a fast path at LOAD by measured exactness — no eviction economy. Closer-owner greps over rust_perf rule_ids: 'reclaim|governor' → entries_26 only (the growth axis above); 'watchdog|deadline|timeout' → ZERO; 'probe' rules (entries_06/08/20/21/30/31) are all instrumentation probes, none a refusal-retry pacing law. NOVEL: the retry-keys-on-returned-class law is new to the corpus. Cite: @ 9ca9768a323048f3ec4d586bb2e2e99947b4c1d4 — the commit message; runtime/engine/Engine.cpp:2024-2035 (reclaimForState's keying rewrite) + :1635-1642 at 32cf878 (the fallback+tryOnce move); runtime/engine/Cache.cpp:616-644 (reclaimForMemory); runtime/engine/Cache.hpp:607-616 (the contract comment); @ f6957e847ad63b4d87aa1c1f5d9f3f8378e09f25 — the commit message (the host-refusal half); @ 32cf878d434c44933b5ede50ccdaf7d23fe271d7 — the commit message; RIDER (Batch 270 / Plan 274, incoai/splash @ 945635d440ac0041e4dfe30262497881947d22e0 'Refill the Index runway after each spill', Apache-2.0 — the RUNWAY face of this rule's own keepRunway machinery): a consuming op that draws down a fixed reserve must refill it AT ITS OWN CONSUMPTION POINT, keyed on its own consumption — never leave the refill to a later phase nothing schedules. The defect, commit message verbatim: 'spillWritten took the free pages and allocated extents for the rest, but never refilled the runway, and nothing else did outside tests: the first spills used it up, and from then on, as in a process that never allocated one, a Resident lane converting at the wall found no Index page for its writes (pendingPages(Full) is capped by the free Index pages), was denied instead of waiting, and yielded or failed; a spill at the wall was refused.' The repair keys the refill ON THE CONSUMER'S MOVES (runtime/engine/Cache.cpp:1067-1075 at the pin, verbatim): '// Then the runway, an extent's free Index pages, is refilled: the next' / '// spill frees Full pages without an allocation, at the memory wall too,' / '// where a converted request's pages spill through it (pendingPages).' — spillWritten keeps an extent's worth of free Index pages after its moves, allocating an Index extent when fewer are free, and the refill's failure is a TYPED denial the caller prices ('return {TokenAdmissionFailure::Denied, pool_.extentPages() - free, free, runway.failure, kv::PageForm::Index};' — 'a refusal is Denied, of the index form, with the pages the runway lacks, for the engine's optional allocation to make room or do without'). The bootstrap moves to the first consuming call ('Its first call, every prepare's on a spill-capable target, makes the runway, so the pool's builder allocates none'), which also makes a reclaim release transient ('A runway that a reclaim or a pressure pass released comes back at the next prepare once growth is admitted'). KINSHIP: this rule's own body keys the ALLOCATION RETRY on returned memory; the rider keys the RESERVE LEVEL on consumed reserve — both are consumption-economy laws over the same governor-funded spill machinery. Distinct from reserve-stream-caches-at-admission-warm-request-paths-at-load (rust_perf entries_37, B257) — that warms caches AT LOAD, this refills a working reserve AT CONSUMPTION; from cache-budget-first-sessions-take-remainder-never-reshrink (entries_37, B257) — STARTUP ordering, not a steady-state level law; from compaction-reserve-before-prompt-build (entries_06, B131) — a reserve before ONE build, not maintenance ACROSS calls. REGIME: any fixed reserve a steady-state op consumes (page-form runways, staging rings, free-slot pools). ANTI-REGIME: reserves refilled by a dedicated phase that provably always runs — and verify it actually runs, because the defect here is precisely 'nothing else did outside tests'. Batch 270 rider. Batch 270 / Plan 274 / Research 306 (slice C).".to_string()),
            // Not benchable on this box: a pacing law over an OS memory governor — the fixture's probe counter is the shape axis.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency (wait keying): a waiter parked on a resource refusal keys
        // its wait on WHICH shortfall the in-flight transfers return — a restore
        // or a covering write justifies the wait, a steady writer's self-consumed
        // write-through never does (the soak's 20-50 s first-token stalls)
        // ──────────────────────────────────────────────────────────────────────
        // Source: incoai/splash fdc8f01 + c27aaf7 + e90b43c + a7406bb + 41e8c82,
        // the commit messages, runtime/engine/Cache.cpp:889-897 (returnsInFlight)
        // + Cache.hpp:754-764 (Shortfall) + Engine.cpp:1621-1640.
        PerfRule {
            rule_id: "refusal-wait-keyed-on-shortfall-returning-transfer".to_string(),
            name: "A waiter parked on a resource refusal must key its WAIT on WHICH shortfall the in-flight transfers return — keying on ANY transfer in flight parks the lane behind work that never returns what it needs ('its pages written through after each command keep a transfer in flight: the decoding lane got neither its floor nor the prompt's memory' — a 90-minute soak held seven lanes at their first token for 20 to 50 s); the law: 'A refused allocation's reclaim ... holds back what is in use for, and reports pending, only a transfer that returns what it lacks', unified into one query — 'Cache::returnsInFlight(Shortfall) is now the rule, with the shortfall as the free pages that cover it' — where a page written through counts 'only by the page it frees (pendingPages), where those pages and the free ones cover the shortfall'; a pass (not a refusal) still waits for any transfer. RIDER (Batch 274 / Plan 278 — the TRANSIENT-SHORTFALL face, incoai/splash @ e88a0d0 'Wait for memory macOS is still returning before the oracle refuses', Apache-2.0): an admission oracle measuring a TRANSIENT resource state waits a bounded, ANNOUNCED recovery window before its refusal stands — macOS returns a just-exited process's memory in ~0.25-0.5 s, so an admission right after a sibling exit reads a false shortfall; the oracle polls (1 s interval, 30 s ceiling) while the shortfall persists and refuses only past the window; no change when memory suffices, the deficit is structural, or the model is already loaded; the refusal names the window so the caller distinguishes 'no memory' from 'memory not yet returned'. Count-neutral rider, Batch 274.".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Provenance: ORIGINAL machinery re-authored from incoai/splash commits
// fdc8f01 + c27aaf7 + e90b43c + a7406bb + 41e8c82 (Apache-2.0): a
// scheduler's wait-or-act decision after a resource refusal, over a
// broker whose in-flight transfers it reads. Never splash code.

/// The refused lane's shortfall, in units of the form it needs.
pub const SHORTFALL: u32 = 4;

#[derive(Clone, Copy, Debug)]
pub enum Transfer {
    /// Lands with its units INTO THE FREE POOL for anyone: the
    /// restore / persistence-write / state-write class - each "holds
    /// back what a reclaim takes once it lands".
    Restore { units: u32 },
    /// A writer's own page write-through: in flight for as long as its
    /// owner keeps running; its units cycle back to the owner, so the
    /// free pool gains nothing while it writes.
    SpillWrite { units: u32 },
}

pub struct Broker {
    pub free: u32,
    pub inflight: Vec<Transfer>,
    pub spills_landed: u32,
}

impl Broker {
    pub fn new() -> Self {
        Self {
            free: 0,
            inflight: Vec::new(),
            spills_landed: 0,
        }
    }

    /// THE DEFECT: any transfer in flight makes the refusal pending -
    /// whatever it returns, whatever form it frees, however short of
    /// the shortfall it falls.
    pub fn any_in_flight(&self) -> bool {
        !self.inflight.is_empty()
    }

    /// Land every in-flight transfer. A restore frees its units to the
    /// pool. A spill write frees its units to the pool at landing, and
    /// its owner re-takes them at once while it keeps writing (the
    /// units cycle, the pool never gains while it runs).
    pub fn tick(&mut self, writer_still_running: bool) {
        let landed = std::mem::take(&mut self.inflight);
        for t in landed {
            match t {
                Transfer::Restore { units } => self.free += units,
                Transfer::SpillWrite { units } => {
                    self.spills_landed += 1;
                    if writer_still_running {
                        self.inflight.push(Transfer::SpillWrite { units: 2 });
                    } else {
                        self.free += units;
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LaneState {
    Waiting,
    Proceeded,
    Admitted,
}

/// One scheduler tick over the refused lane, the DEFECTIVE rule:
/// admitted when the pool covers the shortfall; else wait while ANY
/// transfer is in flight; proceed (the fallback / yield path) only
/// when the broker is fully idle. Decide first, then land.
pub fn tick_lane(
    broker: &mut Broker,
    state: LaneState,
    writer_still_running: bool,
) -> LaneState {
    let next = if state == LaneState::Admitted {
        LaneState::Admitted
    } else if broker.free >= SHORTFALL {
        LaneState::Admitted
    } else if broker.any_in_flight() {
        LaneState::Waiting
    } else {
        LaneState::Proceeded
    };
    broker.tick(writer_still_running);
    next
}

/// The stall scenario: a writer whose write-through is in flight every
/// tick; nothing else. The refused lane needs 4; the write frees 2
/// that go straight back to the writer.
pub fn stall_broker() -> Broker {
    let mut b = Broker::new();
    b.inflight.push(Transfer::SpillWrite { units: 2 });
    b
}

#[test]
fn a_writer_always_in_flight_waits_the_lane_out_forever() {
    // the soak's shape: every decode gap has a transfer in flight, so
    // the "any transfer" rule waits - the lane never acts while the
    // writer runs, however long that is
    let mut broker = stall_broker();
    let mut state = LaneState::Waiting;
    for _ in 0..8 {
        state = tick_lane(&mut broker, state, true);
    }
    assert_eq!(state, LaneState::Waiting); // stalled through all 8 ticks
    assert!(broker.spills_landed >= 7); // the writer never stopped
}

#[test]
fn a_relevant_transfer_justifies_the_wait() {
    // a restore lands 4 units: the "any transfer" rule waits here too -
    // this is the class the defect shares with the paced rule
    let mut broker = Broker::new();
    broker.inflight.push(Transfer::Restore { units: 4 });
    let mut state = LaneState::Waiting;
    state = tick_lane(&mut broker, state, false); // wait (justified)
    assert_eq!(state, LaneState::Waiting);
    state = tick_lane(&mut broker, state, false); // landed: free = 4
    assert_eq!(state, LaneState::Admitted);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Provenance: ORIGINAL machinery re-authored from incoai/splash commits
// fdc8f01 + c27aaf7 + e90b43c + a7406bb + 41e8c82 (Apache-2.0): the
// PACED arm - the wait-or-act decision after a resource refusal keys on
// WHICH shortfall the in-flight transfers return. Never splash code.

/// The refused lane's shortfall, in units of the form it needs.
pub const SHORTFALL: u32 = 4;

#[derive(Clone, Copy, Debug)]
pub enum Transfer {
    /// Lands with its units INTO THE FREE POOL for anyone: the
    /// restore / persistence-write / state-write class - each "holds
    /// back what a reclaim takes once it lands".
    Restore { units: u32 },
    /// A writer's own page write-through: in flight for as long as its
    /// owner keeps running; its units cycle back to the owner, so the
    /// free pool gains nothing while it writes.
    SpillWrite { units: u32 },
}

pub struct Broker {
    pub free: u32,
    pub inflight: Vec<Transfer>,
    pub spills_landed: u32,
}

impl Broker {
    pub fn new() -> Self {
        Self {
            free: 0,
            inflight: Vec::new(),
            spills_landed: 0,
        }
    }

    /// THE LAW: a transfer counts only where it returns the shortfall.
    /// A restore (the restore / persistence-write / state-write class)
    /// always holds back what a reclaim takes once it lands; a spill
    /// write counts only by the units it frees at landing, where those
    /// and the free ones cover the shortfall.
    pub fn returns_in_flight(&self, shortfall: u32) -> bool {
        let mut freed_at_landing = 0u32;
        for t in &self.inflight {
            match *t {
                Transfer::Restore { .. } => return true,
                Transfer::SpillWrite { units } => freed_at_landing += units,
            }
        }
        self.free + freed_at_landing >= shortfall
    }

    /// Land every in-flight transfer. A restore frees its units to the
    /// pool. A spill write frees its units to the pool at landing, and
    /// its owner re-takes them at once while it keeps writing (the
    /// units cycle, the pool never gains while it runs).
    pub fn tick(&mut self, writer_still_running: bool) {
        let landed = std::mem::take(&mut self.inflight);
        for t in landed {
            match t {
                Transfer::Restore { units } => self.free += units,
                Transfer::SpillWrite { units } => {
                    self.spills_landed += 1;
                    if writer_still_running {
                        self.inflight.push(Transfer::SpillWrite { units: 2 });
                    } else {
                        self.free += units;
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LaneState {
    Waiting,
    Proceeded,
    Admitted,
}

/// One scheduler tick over the refused lane, the PACED rule: admitted
/// when the pool covers the shortfall; else wait only while a transfer
/// RETURNS the shortfall; proceed (the fallback / yield path) the
/// moment nothing in flight will. Decide first, then land.
pub fn tick_lane(
    broker: &mut Broker,
    state: LaneState,
    writer_still_running: bool,
) -> LaneState {
    let next = if state == LaneState::Admitted {
        LaneState::Admitted
    } else if broker.free >= SHORTFALL {
        LaneState::Admitted
    } else if broker.returns_in_flight(SHORTFALL) {
        LaneState::Waiting
    } else {
        LaneState::Proceeded
    };
    broker.tick(writer_still_running);
    next
}

/// The stall scenario: a writer whose write-through is in flight every
/// tick; nothing else. The refused lane needs 4; the write frees 2
/// that go straight back to the writer.
pub fn stall_broker() -> Broker {
    let mut b = Broker::new();
    b.inflight.push(Transfer::SpillWrite { units: 2 });
    b
}

#[test]
fn the_lane_acts_when_no_in_flight_transfer_returns_the_shortfall() {
    // the soak's shape, repaired: the writer's write-through is in
    // flight every tick, but it frees 2 while the lane lacks 4 and the
    // free pool gains nothing - the rule proceeds instead of waiting
    let mut broker = stall_broker();
    let mut state = LaneState::Waiting;
    state = tick_lane(&mut broker, state, true);
    assert_eq!(state, LaneState::Proceeded); // acted at the first tick
}

#[test]
fn the_lane_still_waits_for_a_transfer_that_covers_the_shortfall() {
    // a spill write whose freed units DO cover the shortfall: the rule
    // waits, and is admitted the tick after it lands - the wait is
    // justified, never abandoned
    let mut broker = Broker::new();
    broker.inflight.push(Transfer::SpillWrite { units: 4 });
    let mut state = LaneState::Waiting;
    state = tick_lane(&mut broker, state, false); // 0 + 4 >= 4: wait
    assert_eq!(state, LaneState::Waiting);
    state = tick_lane(&mut broker, state, false); // landed: free = 4
    assert_eq!(state, LaneState::Admitted);
    // and a restore justifies the wait outright
    let mut r = Broker::new();
    r.inflight.push(Transfer::Restore { units: 4 });
    assert!(r.returns_in_flight(SHORTFALL));
}

#[test]
fn the_wait_ends_once_the_covering_transfer_is_gone() {
    // a restore of 1 (short of the shortfall) beside the writer: the
    // restore justifies one tick of wait; once it has landed, only the
    // non-covering writer remains and the lane acts
    let mut broker = Broker::new();
    broker.inflight.push(Transfer::Restore { units: 1 });
    broker.inflight.push(Transfer::SpillWrite { units: 2 });
    let mut state = LaneState::Waiting;
    state = tick_lane(&mut broker, state, true); // restore present: wait
    assert_eq!(state, LaneState::Waiting);
    state = tick_lane(&mut broker, state, true); // free 1 + freed 2 < 4: act
    assert_eq!(state, LaneState::Proceeded);
    for _ in 0..6 {
        state = tick_lane(&mut broker, state, true);
    }
    assert_eq!(state, LaneState::Proceeded); // never re-latched
}
"#.to_string(),
            context_notes: Some("Grade B+ — NotBenchable ON THIS BOX (a coordination law; the evidence is soak telemetry on foreign silicon — source-reported figures are PROVENANCE, never bars; the fixture's wait/act ledger is the shape axis). SOURCE (incoai/splash @ fdc8f01d75c4bc116dcb967a58dcb4ab5078a400 + c27aaf7527a33a1085f8d68caa8d19951ce7164b + e90b43c3f20555b576ca86b9ae2fb9ce802dbd29 + a7406bbc685e8216e0a6a220a1302034bc76c1ae + 41e8c82cb8afdfd0cb0f78bd537c8591d9819b9b, Apache-2.0, read at the .raw/b270/splash clone; every quote verbatim this batch). THE DEFECT (fdc8f01 message): 'when no lane fit, any transfer in flight made the refusal pending (Cache::transfersInFlight), so the lane waited. The lane that yields first ... paid only when nothing in the command got memory and nothing was in flight. That held while a refused lane's peers grew too and were refused in turn within a page ... A Spill lane's floor of the hot block cache is a refusal its peers do not share ... and its pages written through after each command keep a transfer in flight: the decoding lane got neither its floor nor the prompt's memory. A 90-minute soak on a 36 GB M5 Max (Flash-Next Q2_0 with the DFlash2 draft, two to four agent sessions at 100K-185K tokens) held seven Spill lanes at their first token for 20 to 50 s, every decode gap over 10 s (36.6 s at most); each ended only when a lane holding the hot block cache finished or the prompt's lane was suspended by an unrelated refusal.' THE FIRST NARROWING (fdc8f01): a refused allocation's reclaim 'holds back what is in use for, and reports pending, only a transfer that returns what it lacks (Cache::refusalPending): a leaf or a state it holds (a restore, a persistence write, the state write), a leaf the scan found waiting for the tier's room (VictimScan::waited), and, for memory the governor funds, the extent that the Full pages demotions and spills free fill with the free ones.' THE UNDERSHOOT POLE (e90b43c): narrowing too far turns the wait into a premature failure — 'A chain whose newest blocks have no copy ... gave the reclaim one block, and the step after found nothing to take and nothing pending: alone, the refused lane failed with capacity_exhausted while the chain still held its memory. In a 45-minute soak ... eight requests failed so, 0-3 s after a Resident lane of 88K-115K tokens finished or was cancelled ... The run before the narrower rule met the same moment four times, waiting for any transfer, and decoded.' — the wait set must cover exactly what the shortfall NEEDS (the demotion holding back the next leaf counts), never more, never less. THE UNIFICATION (c27aaf7): 'Cache::returnsInFlight(Shortfall) is now the rule, with the shortfall as the free pages that cover it: the KV pages it lacks, of their form; for memory the governor funds an extent's worth of Full pages; none for a state's buffers while the host refuses growth.' + 'A page written through counts only by the page it frees (pendingPages), where those pages and the free ones cover the shortfall: a refused page allocation already waits for them when they do (admitPages), and otherwise takes what is in use beside them.' + 'Without spilling this is transfersInFlight(), main's rule' + 'A pass still waits for any transfer.' THE IN-CODE QUERY (runtime/engine/Cache.cpp:889-897 at c27aaf7, verbatim): 'bool Cache::returnsInFlight(const Shortfall &shortfall) const noexcept { if (!restores_.empty() || writes(WriteClass::Reclaim) || writes(WriteClass::Persist) || states_.writing()) return true; // With no demotion in flight, the pages of the form that transfers free are those written through that will spill. const uint32_t freed = pendingPages(shortfall.form); return freed && shortfall.pages && pool_.freePageCount(shortfall.form) + freed >= shortfall.pages; }' — the restore/demotion/persist/state-write class ALWAYS counts ('each holds back what a reclaim takes once it lands'); the spill write-through counts only where it covers. THE SHORTFALL (Cache.hpp:754-764 at c27aaf7): 'What an allocation that was refused lacks, as the free pages that cover it: the KV pages it lacks, of their form; for memory the governor funds, an extent's worth of Full pages, whose extent then leaves memory; none for a state's buffers while the host refuses growth, which only a state gives.' THE REACH HALF (a7406bb, runtime/engine/Cache.cpp:838): the extent-reach check subtracts ONLY in-flight demotions — 'The check now subtracts only the demotions in flight, as its comment says: their pages are cached leaves', among the reusable ones.' — a written-through page is 'pages their Spill requests hold, never among the reusable ones': the WAIT counts them, the REACH check must not (the old pendingPages subtraction understated what eviction could free beside a writer, passing over an idle chain whose leaves would empty an extent). REGIME: wait-or-act decisions after a resource refusal, over a broker with transfer classes whose returns differ (restores/demotions that return the shortfall vs a steady writer's self-consumed write-throughs that never do). ANTI-REGIME: the paced/background pass, which deliberately waits for any transfer; single-transfer-class brokers (the distinction is vacuous there); peer-LIVENESS waits (bounded-peer-handoff-wait's axis — a failed peer never returns anything, so its wait is bounded and fails loud instead). DISTINCT-FROM (all read this batch): bounded-peer-handoff-wait (entries_22, B219) bounds a wait on a PEER's liveness (failed/quitting flags, a fixed bound, a named error); THIS keys a WAIT on the resource's own return schedule — the failure mode there is an unbounded spin on a dead peer, here an unbounded wait on an irrelevant transfer; cache-write-admission-requires-restoring-consumer (entries_27, B239) gates cache WRITES on a restoring consumer existing — an admission predicate, not a wait predicate (both are require-the-consumer shapes, different surfaces); empty-slot-first-victim-order-save-before-overwrite (entries_28, B241) is slot/victim ORDER, unrelated to wait keying; spin-lock-one-message-per-exchange (entries_22, B219) is spin economics. Closer-owner greps over rust_perf rule_ids: 'backoff|pending|in-flight|stall|spin' → spin-lock-one-message-per-exchange + seq-fork-carries-pending-rollback-record (a state-fork record, unrelated). NOVEL: the wait-keyed-on-shortfall-return law is new. Cite: @ fdc8f01d75c4bc116dcb967a58dcb4ab5078a400 — the commit message; runtime/engine/Cache.hpp:741 (the refusalPending contract comment) + runtime/engine/Cache.cpp:862 (refusalPending) + runtime/engine/Engine.cpp:1621-1640 (the yields-first suspension); @ c27aaf7527a33a1085f8d68caa8d19951ce7164b — the commit message; runtime/engine/Cache.cpp:889-897 (returnsInFlight) + Cache.hpp:754-764 (Shortfall); @ e90b43c3f20555b576ca86b9ae2fb9ce802dbd29 — the commit message (demotionHoldsLeaf, folded into returnsInFlight by c27aaf7); @ a7406bbc685e8216e0a6a220a1302034bc76c1ae — the commit message + Cache.cpp:838; RIDER (Batch 270 / Plan 274, incoai/splash @ ea1683d5d05f562eaebf991b3e2b64aa145e937b 'Tell a request the write budget refused when to retry', Apache-2.0 — the CLIENT-LEVEL face of this rule's wait-keying law, one layer out across the wire): a retryable refusal should carry WHEN a retry may succeed, computed from the refuser's own refill ledger, instead of a default backoff the caller burns out. The defect, commit message verbatim: 'The server answered it as every retryable failure, 503 with Retry-After: 1, while the budget refills over hours: a client that honours the header retried a whole admission every second until then.' The repair: 'The engine now gives a failure the seconds after which a retry may succeed (Engine::LaneEnd, EngineEventSink::failed), for kv_write_budget when the budget covers the commitments again (Cache::spillWritesAdmittedAt). The Error event carries them (retryAfterSeconds, only on a retryable one; native protocol 11), and the server's 503 sends them as Retry-After; a failure without them still sends 1.' — the refuser already knows when its shortfall refills (runtime/engine/Engine.cpp:981-983 at the pin, verbatim: '// It refills over hours: a retry may succeed once it covers again' / '// what Spill requests committed, or sooner as they end.' / 'const double admitted = cache_.spillWritesAdmittedAt().value_or(now);'), so deriving the retry time there is one query, while deriving it at the client means retry-storming the whole admission path once per second for hours. KINSHIP: this rule's own body keys a WAIT on which in-flight transfer returns the shortfall; the rider keys a RETRY on when the budget's shortfall refills — the same returns-the-shortfall keying, carried across the wire as a typed hint instead of discovered by re-asking. Distinct from bounded-peer-handoff-wait (rust_perf entries_22, B219) — that bounds a peer-liveness wait, this prices a retry on the resource's own refill clock. REGIME: quota/budget/lease/rate-limit refusals whose refiller knows the refill time (daily budgets, token buckets with known refill rates, lease expiries). ANTI-REGIME: refusals whose retry-readiness is genuinely unknowable (keep the default backoff — 'a failure without them still sends 1'); non-retryable failures (the hint rides retryable ones only). Batch 270 rider. RIDER (Batch 270 / Plan 274, incoai/splash @ 41e8c82cb8afdfd0cb0f78bd537c8591d9819b9b 'Count reusable pages by form', Apache-2.0 — the COUNT-KEYING face of this rule's machinery: these counts feed the very query that decides wait-vs-act, so a wrong count mis-keys the wait in BOTH directions): a reusable/available counter must be keyed on the FORM (class) it counts, never aggregated across forms before the subtraction — the defect is an unsigned WRAPAROUND that makes a guard pass vacuously. Commit message verbatim: 'KvCache::idlePagesOnChains counted those pages of both forms. A state in use on a spilled chain, such as a suspended request's resume point, subtracted the chain's idle Index pages from the Full count, which wrapped around once they outnumbered it: extentWithinReach then let a state's reclaim evict ordinary KV for an extent it could not empty, and the paused path's guard (reuseCachedPagesWhilePaused) passed with nothing to reuse.' The repair keys every layer: 'Both now take the form they count, Full unless said otherwise; the engine asks for the form its admission was refused (TokenAdmission::form).' — reusablePages takes the form (runtime/engine/Cache.cpp:606-612 at the pin: 'uint32_t Cache::reusablePages(ReclaimClass upTo, kv::PageForm form) const { const uint32_t unheld = pool_.reusablePages(form); ... return unheld - kv_.idlePagesOnChains(states_.usedStates(), form); }') and the walk filters per entry (runtime/engine/KvCache.cpp:371: 'if (entry.transfer == Transfer::None && entry.form == form)'). Distinct from narrow-counter-overflow-promotion (rust_perf entries_03, B78) — that is the LAYOUT axis (u64 to u32 with a checked_add promotion sidecar for density; its overflow is DETECTED and promoted, never a silently-wrong guard); THIS is the KEYING axis: a subtraction mixing two classes wraps SILENTLY and passes a guard with nothing behind it — the green-zero shape. Cross-ref: the defeated extentWithinReach is reclaim-progress-keyed-on-refused-resource-returned's own gate (slice C rule 7) — the rules share the machinery, the faces do not overlap (rule 7 owns the RETRY keying, this rule the WAIT keying, this face the COUNTS both read). REGIME: resource ledgers with per-class counts (forms, tiers, size classes) whose guards subtract one class's reserved set from another's pool. ANTI-REGIME: single-class ledgers (nothing to mix); counters where the subtraction is checked/saturating AND the guard re-derives from per-class reads. Batch 270 rider. Batch 270 / Plan 274 / Research 306 (slice C).".to_string()),
            // Not benchable on this box: a coordination law — the fixture's wait/act ledger is the shape axis.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── dispatch (watchdog): the deadline moves a timeout on at the command's
        // observed progress — its commit, each host step as the GPU work before it
        // ends and as it returns — while no single stretch may run past the timeout
        // ──────────────────────────────────────────────────────────────────────
        // Source: incoai/splash 8df9e95, the commit message,
        // runtime/metal/CommandWatchdog.hpp:8-30 + the two backend/loop tests.
        PerfRule {
            rule_id: "watchdog-deadline-moves-on-command-progress".to_string(),
            name: "A command watchdog that bounds the command's WHOLE length gives up on commands that are making progress — 'the watchdog would restart the engine, and drop every session's RAM cache, while the command made progress'; the law: the deadline is 'a timeout after its last progress: its commit, and then each of its host steps as the GPU work before it ends and as it returns', so a command split by host steps 'may run as long as each part ends within the timeout', while 'no stretch of one, its GPU work between two steps or a step, may run past the timeout' — and progress from any OTHER command moves nothing (the sequence guard).".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Provenance: ORIGINAL machinery re-authored from incoai/splash commit
// 8df9e95 (Apache-2.0): a command watchdog on a virtual clock. Never
// splash code.

/// THE DEFECT: the deadline is set once at the command's commit and
/// nothing moves it - the watchdog bounds the command's WHOLE length,
/// so a command split by host steps (each part well inside the
/// timeout, the whole far past it) is given up on while it makes
/// progress, and the restart drops every session's RAM cache.
pub struct Watchdog {
    timeout: f64,
    sequence: u64,
    deadline: f64,
}

impl Watchdog {
    pub fn new(timeout: f64) -> Self {
        Self {
            timeout,
            sequence: 0,
            deadline: 0.0,
        }
    }

    pub fn start(&mut self, seq: u64, now: f64) {
        self.sequence = seq;
        self.deadline = now + self.timeout;
    }

    pub fn complete(&mut self, seq: u64) {
        if self.sequence == seq {
            self.sequence = 0;
        }
    }

    pub fn expired(&self, now: f64) -> bool {
        self.sequence != 0 && now >= self.deadline
    }
}

#[derive(Default)]
pub struct Health {
    pub completed: u32,
    pub dropped: u32,
    /// a restart drops every session's RAM cache
    pub cache_drops: u32,
}

/// Run one command on a virtual clock. `parts` alternates GPU stretch,
/// host step, GPU stretch, host step, ... (here every part is a plain
/// duration). The health check runs every `check_every`; an expired
/// watchdog aborts the command - the restart - and drops the cache.
pub fn run_command(
    wd: &mut Watchdog,
    seq: u64,
    parts: &[f64],
    check_every: f64,
    now: &mut f64,
    h: &mut Health,
) {
    wd.start(seq, *now);
    let mut next_check = *now + check_every;
    for &span in parts {
        *now += span;
        while next_check <= *now {
            if wd.expired(next_check) {
                h.dropped += 1;
                h.cache_drops += 1;
                return; // the command is given up on; it never completes
            }
            next_check += check_every;
        }
    }
    wd.complete(seq);
    h.completed += 1;
}

/// A streamed prefill: eight host steps of 50 with a 50 GPU stretch
/// before each - every part ends well within the 100 timeout, the
/// whole command runs 800.
pub const PROGRESSING_PARTS: [f64; 16] = [50.0; 16];

#[test]
fn a_progressing_command_is_given_up_on_at_the_whole_length() {
    // the incident: every 50-unit part is healthy, the 800-unit whole
    // is not - the watchdog restarts the engine at the 100 mark and
    // drops every session's RAM cache while the command made progress
    let mut wd = Watchdog::new(100.0);
    let mut h = Health::default();
    let mut now = 0.0;
    run_command(&mut wd, 3, &PROGRESSING_PARTS, 2.0, &mut now, &mut h);
    assert_eq!(h.dropped, 1);
    assert_eq!(h.cache_drops, 1); // the collateral: every session's cache
    assert_eq!(h.completed, 0);
}

#[test]
fn a_stalled_stretch_is_still_given_up_on() {
    // one host step of 300 against the 100 timeout: no boundary inside
    // it to move the deadline - the give-up is correct here
    let mut wd = Watchdog::new(100.0);
    let mut h = Health::default();
    let mut now = 0.0;
    run_command(&mut wd, 1, &[300.0], 2.0, &mut now, &mut h);
    assert_eq!(h.dropped, 1);
    assert_eq!(h.completed, 0);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Provenance: ORIGINAL machinery re-authored from incoai/splash commit
// 8df9e95 (Apache-2.0): the PACED arm - the watchdog's deadline moves
// a timeout on at the command's observed progress, and no single
// stretch between two progresses may run past the timeout. Never
// splash code.

/// THE LAW: the deadline is a timeout after the command's LAST
/// progress - its commit, and then each host step as the GPU work
/// before it ends and as the step returns. Progress from any other
/// command moves nothing.
pub struct Watchdog {
    timeout: f64,
    sequence: u64,
    deadline: f64,
}

impl Watchdog {
    pub fn new(timeout: f64) -> Self {
        Self {
            timeout,
            sequence: 0,
            deadline: 0.0,
        }
    }

    pub fn start(&mut self, seq: u64, now: f64) {
        self.sequence = seq;
        self.deadline = now + self.timeout;
    }

    pub fn progress(&mut self, seq: u64, now: f64) {
        if self.sequence == seq {
            self.deadline = now + self.timeout;
        }
    }

    pub fn complete(&mut self, seq: u64) {
        if self.sequence == seq {
            self.sequence = 0;
        }
    }

    pub fn expired(&self, now: f64) -> bool {
        self.sequence != 0 && now >= self.deadline
    }
}

#[derive(Default)]
pub struct Health {
    pub completed: u32,
    pub dropped: u32,
    /// a restart drops every session's RAM cache
    pub cache_drops: u32,
}

/// Run one command on a virtual clock. `parts` alternates GPU stretch,
/// host step, GPU stretch, host step, ... (here every part is a plain
/// duration). The health check runs every `check_every`; an expired
/// watchdog aborts the command - the restart - and drops the cache.
/// The runner moves the deadline at each part boundary: the GPU work
/// before a step ending (the runner takes the step up) and the step
/// returning.
pub fn run_command(
    wd: &mut Watchdog,
    seq: u64,
    parts: &[f64],
    check_every: f64,
    now: &mut f64,
    h: &mut Health,
) {
    wd.start(seq, *now);
    let mut next_check = *now + check_every;
    for &span in parts {
        *now += span;
        while next_check <= *now {
            if wd.expired(next_check) {
                h.dropped += 1;
                h.cache_drops += 1;
                return; // the command is given up on; it never completes
            }
            next_check += check_every;
        }
        wd.progress(seq, *now); // this command's progress moves ITS deadline
    }
    wd.complete(seq);
    h.completed += 1;
}

/// A streamed prefill: eight host steps of 50 with a 50 GPU stretch
/// before each - every part ends well within the 100 timeout, the
/// whole command runs 800.
pub const PROGRESSING_PARTS: [f64; 16] = [50.0; 16];

#[test]
fn a_progressing_command_completes_healthy() {
    // every part ends within the timeout, so the deadline keeps moving
    // and the command runs to its end - the cache is never dropped
    let mut wd = Watchdog::new(100.0);
    let mut h = Health::default();
    let mut now = 0.0;
    run_command(&mut wd, 3, &PROGRESSING_PARTS, 2.0, &mut now, &mut h);
    assert_eq!(h.completed, 1);
    assert_eq!(h.dropped, 0);
    assert_eq!(h.cache_drops, 0);
}

#[test]
fn a_stalled_stretch_is_still_given_up_on() {
    // one host step of 300 against the 100 timeout: no boundary inside
    // it, no progress - the give-up is preserved
    let mut wd = Watchdog::new(100.0);
    let mut h = Health::default();
    let mut now = 0.0;
    run_command(&mut wd, 1, &[300.0], 2.0, &mut now, &mut h);
    assert_eq!(h.dropped, 1);
    assert_eq!(h.completed, 0);
    // a command without host steps is bounded as before
    let mut wd2 = Watchdog::new(100.0);
    let mut h2 = Health::default();
    let mut now2 = 0.0;
    run_command(&mut wd2, 2, &[150.0], 2.0, &mut now2, &mut h2);
    assert_eq!(h2.dropped, 1);
}

#[test]
fn another_commands_progress_moves_nothing() {
    // the source's own arithmetic (its 120 s watchdog): command 3
    // starts at 0 (deadline 120), its progress at 100 moves the
    // deadline to 220, and command 2's progress at 200 moves NOTHING -
    // so the deadline expires between 219.999 and 220.0
    let mut wd = Watchdog::new(120.0);
    wd.start(3, 0.0);
    wd.progress(3, 100.0);
    wd.progress(2, 200.0);
    assert!(!wd.expired(219.999));
    assert!(wd.expired(220.0));
}
"#.to_string(),
            context_notes: Some("Grade B — decision-content, NotBenchable ON THIS BOX (a liveness/pacing law; the evidence is the source's own Metal-backend test on Apple silicon — source-reported figures are PROVENANCE, never bars; a Rust microbench of deadline arithmetic measures nothing the rule asserts — the Batch-182/186 corpus-only class). SOURCE (incoai/splash @ 8df9e95a17b3f156cc75d88e5dc0c429670ff235 'Give up on a command only once it stops making progress', Apache-2.0, read at the .raw/b270/splash clone; every quote verbatim this batch). THE LAW (runtime/metal/CommandWatchdog.hpp:8-13 at 8df9e95, verbatim): 'A command's deadline is a timeout after its last progress: its commit, and then each of its host steps as the GPU work before it ends and as it returns, so that a command split by host steps (a streamed prefill reading its history layer by layer) may run as long as each part ends within the timeout.' THE MECHANISM (CommandWatchdog.hpp:27-30, verbatim): 'void progress(uint64_t sequence, double nowSeconds) noexcept { if (sequence_ == sequence) deadlineSeconds_ = nowSeconds + timeoutSeconds_; }' — the SEQUENCE GUARD is load-bearing: the source's own unit test (dev/tests/engine/native_engine_loop_test.cpp:565-571) asserts 'A command's progress (its host steps) moves its deadline, and another command's moves nothing' — start(3, 0.0), progress(3, 100.0), progress(2, 200.0) leaves expired(219.999) false and expired(220.0) true at the 120 s timeout. THE BOUND (commit message): 'A command without host steps is bounded as before, and no stretch of one, its GPU work between two steps or a step, may run past the timeout.' THE WHY (commit message): 'The 120 s command watchdog bounded a command's whole length. A command split by host steps does CPU work between its GPU parts: a streamed prefill group of a block-sparse target will read each attention layer's spilled history in host steps, and on a slow external SSD a long group's reads take minutes while each part ends in seconds; the watchdog would restart the engine, and drop every session's RAM cache, while the command made progress.' THE RUNNER DISCIPLINE (commit message): 'The runner moves the deadline with the times it reads for the step's timing, and moves it for the step's return once the GPU went on, so that the submission lock the watchdog shares with commits never holds the GPU back.' THE SOURCE'S OWN VERIFICATION (dev/tests/engine/metal_backend_test.mm:1977-1982, under MTL_SHADER_VALIDATION=1 with a 100 ms timeout and health checks every 2 ms): 'The watchdog bounds each stretch of a command between its progress, its host steps as the work before each ends and as each returns, not the whole command: checked as often as the engine's health checks may, a command of eight host steps of 50 ms each outlasts the 100 ms timeout and completes with the backend healthy, and one host step of 300 ms is given up on.' REGIME: watchdogs bounding multi-part commands whose parts each finish well within the timeout (GPU+host-step pipelines, streamed prefills, chunked IO with per-chunk progress). ANTI-REGIME: watchdogs over genuinely atomic commands (a single submit has no internal progress points — the whole-length bound is correct there); progress sources that fire without forward motion (a heartbeat ticking while stuck would deaden the watchdog — the progress must be OBSERVED WORK COMPLETING, the source's step boundaries); peer-liveness waits (bounded-peer-handoff-wait's axis). DISTINCT-FROM (all read this batch): bounded-peer-handoff-wait (entries_22, B219) — a BOUNDED wait that FAILS a dead peer with a named error; THIS is a MOVING deadline that survives a healthy multi-part command — the two are opposite poles of the same when-to-give-up problem (never-extend-but-bounded vs extend-on-progress); thermal-hysteresis-band-pacing-chunk-placement (entries_21, B219) paces WORK ADMISSION by box state (a hysteresis band per chunk); THIS paces WATCHDOG EXPIRY by command progress — no admission gating, no box telemetry; adaptive-yield-consecutive-ready (entries_06) is a scheduler-fairness yield counter; deadline-bounded-poll-of-submitted-completion-event (kernel_opt entries_gpu_60) bounds a POLL with a fall-through deadline fixed at poll start — never extended, the opposite motion. Closer-owner greps over rust_perf rule_ids: 'watchdog|deadline|timeout' → ZERO (the corpus had no watchdog law at all; kernel_opt's one deadline rule is the poll above). NOVEL: the first watchdog/deadline-extension law in rust_perf. Cite: @ 8df9e95a17b3f156cc75d88e5dc0c429670ff235 — runtime/metal/CommandWatchdog.hpp:8-13+22-30 (the class comment + progress()); dev/tests/engine/native_engine_loop_test.cpp:565-571 (the foreign-sequence arithmetic); dev/tests/engine/metal_backend_test.mm:1977-1982 (watchdogFollowsHostSteps); the commit message; Batch 270 / Plan 274 / Research 306 (slice C).".to_string()),
            // Not benchable on this box: a liveness law — deadline arithmetic measured by nothing but its own assertions.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
