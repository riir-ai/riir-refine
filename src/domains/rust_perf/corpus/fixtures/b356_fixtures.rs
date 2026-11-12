//! Batch 356 fixture laws — the r/rust AI borrow-checker-appeasement
//! rust_perf distill (Research 400 / Plan 366): four laws + one drop
//! adjudication, as ONE standalone file.
//!
//! ORIGINAL plain-Rust reproductions — the source is a POST-ONLY case
//! study with NO repo (the forge-blog precedent), so there is nothing to
//! paste and every fixture is an original synthesis of the pattern
//! shapes. Compiled AND executed BEFORE entry:
//!
//!   rustc --edition 2021 --test b356_fixtures.rs -o fx && ./fx --test-threads=1
//!
//! (the B336/B338/B339/B340/B341/B342/B343/B345/B349 discipline). Each
//! rule's arms in `entries_72.rs` embed the same executed code. The
//! four-arm discipline per law: buggy form, fixed form, all-safe stub
//! (the anti-regime shape the law must NOT condemn), tampered-verdict
//! (a seeded violation the law's own assertion catches — the teeth).
//!
//! Laws under test:
//! F1 `mem-take-field-checkout-empty-window` — (a) the WINDOW HAZARD: a
//!     nested `&mut self` call between take and write-back observes the
//!     field as its empty default (the audit reader sees len 0 while the
//!     data is checked out); (b) the disjoint-split remedy keeps BOTH
//!     halves live — the same nested reader sees the full length, and
//!     the final state is identical; (c) ALL-SAFE: `mem::take` as a
//!     deliberate move-out (the owned `mem-take` rule's shape) has no
//!     window reader and is fine; (d) TAMPERED: a missing check-in (the
//!     taken field dropped) leaves the inventory empty — the final-state
//!     assertion catches it.
//! F2 `clone-before-mutating-loop-borrow-appeasement` — (a) the
//!     appeasement clone (taken only to end a borrow before a mutating
//!     loop over the same owner) allocates per call while a single
//!     read-then-mutate pass returns the SAME pre-round sum with zero
//!     allocs and an identical final state; (b) ALL-SAFE: a clone whose
//!     consumer outlives the call (an undo log) is load-bearing — the
//!     law must not condemn it; (c) TAMPERED: a "clean" that folds AFTER
//!     mutation returns the post-round sum — the equality assertion
//!     catches the ordering bug.
//! F3 `thread-spawn-per-call-pool-reuse` — (a) equality: spawn/join
//!     workers per call and a built-once pool produce the same sum
//!     across sizes and worker counts; (b) the alloc axis: the pooled
//!     call at steady state allocates LESS than the per-call spawn
//!     (packets + handle collect vs one sink + a warm queue); (c)
//!     ALL-SAFE: a one-shot cold call pays the pool build either way —
//!     equality only, no perf claim; (d) TAMPERED: a "pool" rebuilt per
//!     call reads as the buggy arm on the alloc census — the teeth.
//!     TIMING IS NOT ASSERTED HERE (box-load flaky): the bench
//!     validator's median-of-21 cells carry it.
//! F4 `dense-id-domain-hashmap-to-slot-array` — (a) the hash census: a
//!     tick over a dense id domain hashes EVERY id on the map arm and
//!     ZERO on the slot array; (b) the build alloc axis: HashMap growth
//!     allocates repeatedly, one `Vec::with_capacity` allocates once;
//!     (c) ALL-SAFE (the regime boundary, numerically): for a SPARSE id
//!     domain the slot array's byte cost dwarfs the map's — the law must
//!     not fire there; (d) TAMPERED: a "fixed" that still consults a map
//!     shows hashes — caught.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

// ── the allocation probe: a forwarding global allocator counting every
// alloc + every requested byte. Installed file-level (legal in a
// standalone test binary) so the F2/F3/F4 alloc-axis assertions count
// real allocations. Deltas are read around single calls.
struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(l.size(), Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(n, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(l.size(), Ordering::Relaxed);
        unsafe { System.alloc_zeroed(l) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn snapshot_allocs() -> (usize, usize) {
    (
        ALLOCS.load(Ordering::Relaxed),
        BYTES.load(Ordering::Relaxed),
    )
}

fn alloc_delta(before: (usize, usize)) -> (usize, usize) {
    let now = snapshot_allocs();
    (now.0 - before.0, now.1 - before.1)
}

/// the deterministic per-entity weight (the same constant the entries
/// arms embed)
fn w(i: usize) -> u64 {
    (i as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(0xBF58_476D_1CE4_E5B9)
        >> 33
}

// ═══════════════════════════════════════════════════════════════════
// F1 — mem-take-field-checkout-empty-window
// ═══════════════════════════════════════════════════════════════════

struct SubInv {
    items: Vec<u64>,
}
struct SubSys {
    score: u64,
}
struct GameState {
    inv: SubInv,
    sys: SubSys,
}

impl GameState {
    fn seed(n: usize) -> Self {
        GameState {
            inv: SubInv {
                items: (0..n as u64).collect(),
            },
            sys: SubSys { score: 7 },
        }
    }

    /// THE APPEASEMENT: the field is checked out so the nested `&mut
    /// self` call compiles — inventory is EMPTY until write-back, and
    /// every reader inside the window (flush's audit here, a debug dump
    /// or a re-entrant tick in the wild) observes the empty default.
    fn appeased_flush(&mut self, seen: &mut Vec<usize>) {
        let mut taken = std::mem::take(&mut self.inv.items);
        self.flush(seen); // the nested &mut self call the take appeased
        taken.push(self.sys.score);
        self.inv.items = taken; // check-in
    }

    fn flush(&mut self, seen: &mut Vec<usize>) {
        seen.push(self.inv.items.len()); // the nested reader, IN the window
        self.sys.score = self.sys.score.wrapping_add(1);
    }

    /// THE REMEDY (architectural): split the owner into disjoint halves
    /// so the same "nested" work runs on the OTHER half while the
    /// inventory stays live — no window ever opens, no take, no clone.
    fn split_flush(&mut self, seen: &mut Vec<usize>) {
        let GameState { inv, sys } = self;
        sys.score = sys.score.wrapping_add(1); // flush's write, on the disjoint half
        seen.push(inv.items.len()); // the same reader — sees the FULL data
        inv.items.push(sys.score);
    }

    /// ALL-SAFE: a deliberate move-out (the owned `mem-take` rule's
    /// shape). No nested reader between take and return — no window.
    fn drain(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.inv.items)
    }

    /// TAMPERED: the check-in forgotten — the taken field drops.
    fn broken_flush(&mut self) {
        let mut taken = std::mem::take(&mut self.inv.items);
        taken.push(self.sys.score);
        // `self.inv.items = taken;` FORGOTTEN — the drop the checker must catch
    }
}

#[test]
fn f1_window_hazard_the_nested_reader_sees_the_empty_default() {
    const N: usize = 64;
    let mut appeased = GameState::seed(N);
    let mut seen_appeased = Vec::new();
    appeased.appeased_flush(&mut seen_appeased);
    assert_eq!(
        seen_appeased,
        vec![0],
        "THE LAW: the nested reader inside the take-window observed the \
         EMPTY default, not the live data"
    );

    let mut split = GameState::seed(N);
    let mut seen_split = Vec::new();
    split.split_flush(&mut seen_split);
    assert_eq!(
        seen_split,
        vec![N],
        "THE REMEDY: the disjoint split keeps the inventory live — the \
         same reader sees the full length"
    );
}

#[test]
fn f1_final_state_equality_across_forms() {
    const N: usize = 64;
    let mut appeased = GameState::seed(N);
    let mut sink = Vec::new();
    appeased.appeased_flush(&mut sink);

    let mut split = GameState::seed(N);
    let mut sink2 = Vec::new();
    split.split_flush(&mut sink2);

    assert_eq!(appeased.inv.items, split.inv.items, "final inventory equal");
    assert_eq!(appeased.sys.score, split.sys.score, "final score equal");
    assert_eq!(appeased.inv.items.len(), N + 1, "one item pushed by both");
}

#[test]
fn f1_all_safe_move_out_is_not_the_hazard() {
    const N: usize = 32;
    let mut state = GameState::seed(N);
    let drained = state.drain();
    assert_eq!(drained.len(), N, "the move-out hands over the full data");
    assert!(
        state.inv.items.is_empty(),
        "the move-out INTENDS the empty field — no reader was misled in a window"
    );
}

#[test]
fn f1_tampered_missing_checkin_is_caught() {
    const N: usize = 16;
    let mut state = GameState::seed(N);
    state.broken_flush();
    assert!(
        state.inv.items.is_empty() && N > 0,
        "TEETH: the final-state assertion catches the forgotten check-in — \
         the correct forms end at len {N}+1, this one ends empty"
    );
}

// ═══════════════════════════════════════════════════════════════════
// F2 — clone-before-mutating-loop-borrow-appeasement
// ═══════════════════════════════════════════════════════════════════

struct World {
    hp: Vec<u64>,
}
struct UndoLog {
    snaps: Vec<Vec<u64>>,
}

impl World {
    fn seed(n: usize) -> Self {
        World {
            hp: (0..n as u64).map(|i| w(i as usize) % 1000 + 1).collect(),
        }
    }

    /// THE APPEASEMENT: the clone exists ONLY to end the borrow before
    /// the mutating loop over the same owner; the snapshot's sole
    /// consumer reads it after the loop.
    fn appeased_round(&mut self) -> u64 {
        let snapshot: Vec<u64> = self.hp.clone(); // the appeasement clone
        for v in &mut self.hp {
            *v = v.wrapping_add(1);
        }
        snapshot.iter().fold(0u64, |a, &b| a.wrapping_add(b)) // pre-round sum
    }

    /// THE REMEDY: one read-then-mutate pass — the pre-round sum falls
    /// out of the same loop, zero clones.
    fn closed_round(&mut self) -> u64 {
        let mut pre = 0u64;
        for v in &mut self.hp {
            pre = pre.wrapping_add(*v);
            *v = v.wrapping_add(1);
        }
        pre
    }

    /// ALL-SAFE: the clone whose consumer OUTLIVES the call (an undo
    /// log holding the pre-state for a later rollback) is load-bearing —
    /// it cannot be collapsed into the loop because its reader is a
    /// later call.
    fn legit_round_with_undo(&mut self, undo: &mut UndoLog) {
        undo.snaps.push(self.hp.clone());
        for v in &mut self.hp {
            *v = v.wrapping_add(1);
        }
    }

    /// TAMPERED: the "clean" that folds AFTER mutation — the seeded
    /// ordering bug the equality assertion catches.
    fn broken_round(&mut self) -> u64 {
        for v in &mut self.hp {
            *v = v.wrapping_add(1);
        }
        self.hp.iter().fold(0u64, |a, &b| a.wrapping_add(b)) // post-round sum
    }
}

#[test]
fn f2_same_pre_round_sum_and_final_state() {
    const N: usize = 512;
    let mut a = World::seed(N);
    let mut b = World::seed(N);
    let pre_appeased = a.appeased_round();
    let pre_closed = b.closed_round();
    assert_eq!(
        pre_appeased, pre_closed,
        "both forms return the SAME pre-round sum"
    );
    assert_eq!(a.hp, b.hp, "final hp state identical");
    assert_eq!(
        a.hp[0],
        World::seed(N).hp[0] + 1,
        "every value advanced by one"
    );
}

#[test]
fn f2_the_appeasement_clone_is_the_alloc_axis() {
    const N: usize = 512;
    let mut a = World::seed(N);
    let mut b = World::seed(N);
    let before_a = snapshot_allocs();
    let _ = a.appeased_round();
    let clone_allocs = alloc_delta(before_a).0;
    let before_b = snapshot_allocs();
    let _ = b.closed_round();
    let closed_allocs = alloc_delta(before_b).0;
    assert!(
        clone_allocs > closed_allocs,
        "THE LAW: the appeasement clone allocates per call ({clone_allocs} \
         allocs) while the single-pass form allocates none extra \
         ({closed_allocs} allocs)"
    );
}

#[test]
fn f2_all_safe_undo_log_clone_is_load_bearing() {
    const N: usize = 64;
    let mut state = World::seed(N);
    let mut undo = UndoLog { snaps: Vec::new() };
    let before = state.hp.clone();
    state.legit_round_with_undo(&mut undo);
    assert_eq!(undo.snaps.len(), 1, "the undo log holds the pre-state");
    assert_eq!(
        undo.snaps[0], before,
        "the clone's reader OUTLIVES the call — the clone is load-bearing, \
         not appeasement"
    );
    assert_ne!(state.hp, before, "the live state advanced");
}

#[test]
fn f2_tampered_fold_after_mutation_is_caught() {
    const N: usize = 256;
    let mut a = World::seed(N);
    let mut b = World::seed(N);
    let pre_closed = b.closed_round();
    let broken = a.broken_round();
    assert_ne!(
        broken, pre_closed,
        "TEETH: the equality assertion catches the seeded ordering bug — \
         folding after mutation returns the post-round sum"
    );
}

// ═══════════════════════════════════════════════════════════════════
// F3 — thread-spawn-per-call-pool-reuse
// ═══════════════════════════════════════════════════════════════════

struct Job {
    data: Arc<Vec<u64>>,
    start: usize,
    end: usize,
    sink: Arc<Mutex<u64>>,
}

struct Queue {
    jobs: Mutex<VecDeque<Job>>,
    cv: Condvar,
    pending: Mutex<usize>,
    pending_cv: Condvar,
}

struct Pool {
    queue: Arc<Queue>,
}

fn build_pool(workers: usize) -> Pool {
    let queue = Arc::new(Queue {
        jobs: Mutex::new(VecDeque::new()),
        cv: Condvar::new(),
        pending: Mutex::new(0),
        pending_cv: Condvar::new(),
    });
    for _ in 0..workers {
        let q = Arc::clone(&queue);
        // the BUILT-ONCE workers: detached, they serve every later call
        std::thread::spawn(move || {
            loop {
                let job = {
                    let mut j = q.jobs.lock().unwrap();
                    loop {
                        match j.pop_front() {
                            Some(x) => break x,
                            None => j = q.cv.wait(j).unwrap(),
                        }
                    }
                };
                let sum = job.data[job.start..job.end]
                    .iter()
                    .fold(0u64, |a, &b| a.wrapping_add(b));
                {
                    let mut s = job.sink.lock().unwrap();
                    *s = s.wrapping_add(sum);
                }
                let mut p = q.pending.lock().unwrap();
                *p -= 1;
                q.pending_cv.notify_all();
            }
        });
    }
    Pool { queue }
}

fn submit_and_wait(queue: &Queue, data: &Arc<Vec<u64>>, workers: usize) -> u64 {
    let sink = Arc::new(Mutex::new(0u64));
    let per = data.len().div_ceil(workers).max(1);
    {
        let mut q = queue.jobs.lock().unwrap();
        let mut p = queue.pending.lock().unwrap();
        *p = 0; // no job is in flight between calls (the caller waited)
        let mut i = 0;
        while i < data.len() {
            let end = (i + per).min(data.len());
            q.push_back(Job {
                data: Arc::clone(data),
                start: i,
                end,
                sink: Arc::clone(&sink),
            });
            *p += 1;
            i = end;
        }
    }
    queue.cv.notify_all();
    let mut p = queue.pending.lock().unwrap();
    while *p > 0 {
        p = queue.pending_cv.wait(p).unwrap();
    }
    let total = *sink.lock().unwrap();
    total
}

static POOL: OnceLock<Pool> = OnceLock::new();

/// THE BUGGY ARM: spawn + join the worker set EVERY call — the thread
/// stack, packet and handle cost is paid per frame, forever.
fn frame_naive(data: &[u64], workers: usize) -> u64 {
    let per = data.len().div_ceil(workers).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = data
            .chunks(per)
            .map(|chunk| s.spawn(move || chunk.iter().fold(0u64, |a, &b| a.wrapping_add(b))))
            .collect();
        handles
            .into_iter()
            .fold(0u64, |a, h| a.wrapping_add(h.join().unwrap()))
    })
}

/// THE FIXED ARM: a built-once pool — the same work, the spawn cost
/// paid once and amortized over every later call.
fn frame_pooled(data: &Arc<Vec<u64>>, workers: usize) -> u64 {
    let pool = POOL.get_or_init(|| build_pool(workers));
    submit_and_wait(&pool.queue, data, workers)
}

/// TAMPERED: a "pool" rebuilt per call — the mislabel the alloc census
/// must catch.
fn frame_pooled_rebuilt_per_call(data: &Arc<Vec<u64>>, workers: usize) -> u64 {
    let pool = build_pool(workers);
    submit_and_wait(&pool.queue, data, workers)
}

#[test]
fn f3_equality_across_sizes_and_worker_counts() {
    for len in [1usize, 255, 4096] {
        let data: Arc<Vec<u64>> = Arc::new((0..len).map(|i| w(i)).collect());
        let closed = data.iter().fold(0u64, |a, &b| a.wrapping_add(b));
        for workers in [1usize, 2, 4] {
            let naive = frame_naive(&data, workers);
            let pooled = frame_pooled(&data, workers);
            assert_eq!(naive, closed, "naive sum, len={len} workers={workers}");
            assert_eq!(pooled, closed, "pooled sum, len={len} workers={workers}");
        }
    }
}

#[test]
fn f3_alloc_axis_the_pool_amortizes() {
    const WORKERS: usize = 4;
    let data: Arc<Vec<u64>> = Arc::new((0..4096usize).map(w).collect());

    let before = snapshot_allocs();
    let _ = frame_naive(&data, WORKERS);
    let naive_allocs = alloc_delta(before).0;

    // warm the built-once pool (its construction lands here, once)
    let _ = frame_pooled(&data, WORKERS);
    let before = snapshot_allocs();
    let _ = frame_pooled(&data, WORKERS);
    let pooled_allocs = alloc_delta(before).0;

    assert!(
        pooled_allocs < naive_allocs,
        "THE LAW: the pooled call at steady state allocates {pooled_allocs} \
         times vs the per-call spawn's {naive_allocs}"
    );
}

#[test]
fn f3_all_safe_one_shot_parity() {
    // ONE cold call: the pool build and the spawn set cost the same
    // order — the law makes no claim here, only equality.
    let data: Arc<Vec<u64>> = Arc::new((0..1024usize).map(w).collect());
    let closed = data.iter().fold(0u64, |a, &b| a.wrapping_add(b));
    let naive = frame_naive(&data, 2);
    let pool = build_pool(2);
    let pooled = submit_and_wait(&pool.queue, &data, 2);
    assert_eq!(naive, closed);
    assert_eq!(pooled, closed);
}

#[test]
fn f3_tampered_pool_rebuilt_per_call_is_caught() {
    const WORKERS: usize = 4;
    let data: Arc<Vec<u64>> = Arc::new((0..4096usize).map(w).collect());

    let before = snapshot_allocs();
    let _ = frame_naive(&data, WORKERS);
    let naive_allocs = alloc_delta(before).0;

    let _ = frame_pooled(&data, WORKERS); // warm
    let before = snapshot_allocs();
    let _ = frame_pooled(&data, WORKERS);
    let pooled_allocs = alloc_delta(before).0;

    let before = snapshot_allocs();
    let _ = frame_pooled_rebuilt_per_call(&data, WORKERS);
    let rebuilt_allocs = alloc_delta(before).0;

    assert!(
        rebuilt_allocs >= naive_allocs,
        "TEETH: the rebuilt-per-call 'pool' reads as the buggy arm \
         ({rebuilt_allocs} >= {naive_allocs} allocs) — the census catches \
         the mislabel against the true steady state ({pooled_allocs})"
    );
}

// ═══════════════════════════════════════════════════════════════════
// F4 — dense-id-domain-hashmap-to-slot-array
// ═══════════════════════════════════════════════════════════════════

static HASHES: AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
struct CountingHasher;
impl Hasher for CountingHasher {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, _bytes: &[u8]) {
        HASHES.fetch_add(1, Ordering::Relaxed);
    }
    fn write_u32(&mut self, _n: u32) {
        HASHES.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone, Default)]
struct CountBuild;
impl BuildHasher for CountBuild {
    type Hasher = CountingHasher;
    fn build_hasher(&self) -> CountingHasher {
        CountingHasher
    }
}

fn tick_map(map: &mut HashMap<u32, u64, CountBuild>) -> u64 {
    let n = map.len() as u32;
    for id in 0..n {
        if let Some(v) = map.get_mut(&id) {
            *v = v.wrapping_add(w(id as usize));
        }
    }
    map.values().fold(0u64, |a, &b| a.wrapping_add(b))
}

fn tick_vec(vec: &mut Vec<u64>) -> u64 {
    for (id, v) in vec.iter_mut().enumerate() {
        *v = v.wrapping_add(w(id));
    }
    vec.iter().fold(0u64, |a, &b| a.wrapping_add(b))
}

/// TAMPERED: the "fixed" that still consults a map — the census must
/// see hashes where the slot array shows none.
fn tick_vec_broken(map: &mut HashMap<u32, u64, CountBuild>) -> u64 {
    let n = map.len() as u32;
    for id in 0..n {
        if let Some(v) = map.get_mut(&id) {
            *v = v.wrapping_add(w(id as usize));
        }
    }
    map.values().fold(0u64, |a, &b| a.wrapping_add(b))
}

#[test]
fn f4_equality_and_the_hash_census() {
    const N: usize = 4096;
    let mut map: HashMap<u32, u64, CountBuild> = HashMap::default();
    let mut vec: Vec<u64> = Vec::with_capacity(N);
    for i in 0..N as u32 {
        let x = w(i as usize);
        map.insert(i, x);
        vec.push(x);
    }

    HASHES.store(0, Ordering::Relaxed);
    let map_sum = tick_map(&mut map);
    let map_hashes = HASHES.load(Ordering::Relaxed);

    HASHES.store(0, Ordering::Relaxed);
    let vec_sum = tick_vec(&mut vec);
    let vec_hashes = HASHES.load(Ordering::Relaxed);

    assert_eq!(map_sum, vec_sum, "same final state after one tick");
    assert!(
        map_hashes >= N,
        "THE LAW: the map arm hashed every id on the tick pass \
         ({map_hashes} hashes for {N} ids)"
    );
    assert_eq!(
        vec_hashes, 0,
        "THE REMEDY: the slot array hashes NOTHING — direct index"
    );
}

#[test]
fn f4_build_alloc_axis_map_growth_vs_one_vec() {
    const N: usize = 4096;
    let before = snapshot_allocs();
    let mut map: HashMap<u32, u64, CountBuild> = HashMap::default();
    for i in 0..N as u32 {
        map.insert(i, w(i as usize));
    }
    let map_allocs = alloc_delta(before).0;

    let before = snapshot_allocs();
    let vec: Vec<u64> = Vec::with_capacity(N);
    let vec_allocs = alloc_delta(before).0;
    std::mem::forget(vec); // keep the buffer alive to the end of the test

    assert!(
        map_allocs > vec_allocs,
        "THE LAW: building the map grows the table repeatedly \
         ({map_allocs} allocs) while one Vec::with_capacity allocates \
         once ({vec_allocs})"
    );
}

#[test]
fn f4_all_safe_sparse_domain_is_the_anti_regime() {
    const DOMAIN: usize = 1 << 20;
    const LIVE: usize = 16;
    // sparse ids scattered across a huge domain
    let mut map: HashMap<u32, u64, CountBuild> = HashMap::default();
    for k in 0..LIVE {
        let id = (w(k) as usize % DOMAIN) as u32;
        map.insert(id, w(k));
    }

    let before = snapshot_allocs();
    let map_bytes = alloc_delta(before).1;
    let before = snapshot_allocs();
    let vec: Vec<u64> = Vec::with_capacity(DOMAIN);
    let vec_bytes = alloc_delta(before).1;
    std::mem::forget(vec);

    assert!(
        vec_bytes > map_bytes * 100,
        "THE ANTI-REGIME, numerically: at {LIVE} live ids over a {DOMAIN} \
         domain the slot array costs {vec_bytes} bytes vs the map's \
         {map_bytes} — the swap is a memory blowup here, the law must not \
         fire"
    );
    assert_eq!(map.len(), LIVE, "the map serves the sparse domain fine");
}

#[test]
fn f4_tampered_map_in_the_fixed_arm_is_caught() {
    const N: usize = 1024;
    let mut map: HashMap<u32, u64, CountBuild> = HashMap::default();
    for i in 0..N as u32 {
        map.insert(i, w(i as usize));
    }
    HASHES.store(0, Ordering::Relaxed);
    let _ = tick_vec_broken(&mut map);
    let broken_hashes = HASHES.load(Ordering::Relaxed);
    assert!(
        broken_hashes >= N,
        "TEETH: a 'fixed' that still consults a map shows {broken_hashes} \
         hashes where the slot array shows 0 — the census catches it"
    );
}
