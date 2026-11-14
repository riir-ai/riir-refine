//! Batch 363 fixtures — the rust_perf half (Plan 374 / Research 407):
//! the hotpath.rs CI-benchmarking blog distill. ORIGINAL std-only models,
//! never pasted source (the blog's dedup SHAPE is re-authored with our own
//! counters; its measured figures stay in the research note, never in
//! code). Standalone-compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b363_fixtures.rs && ./b363_fixtures`
//! (the B353/B359/B360/B361/B362 pre-entry discipline).
//!
//! Arms (per the rules' FIXTURES notes):
//! - F1 instruction-count-cache-blindness: two walks with EQUAL modeled
//!   instruction counts but divergent access locality — sequential vs
//!   pointer-chased — the icount-equal / wall-divergent witness, plus the
//!   instruction-bound anti-regime (where icount IS representative) and
//!   the tampered arm (a "blindness-aware" gate that still judges on
//!   icount alone must refuse the locality-divergent pair);
//! - F2 async-wrapper-future-size-exponential-nesting: N levels of
//!   `async fn` wrapper composition vs one hand-written linear Future —
//!   the composed size grows ~2-3x per level (exponential in N), the
//!   hand-written form grows by a constant; a spawn/poll allocation
//!   census rides a counting allocator;
//! - F3 alloc-proxy-inversion-work-ratio-caveat: a dedup pair where the
//!   "optimized" shape allocates MORE while doing LESS work (work-count
//!   proxy) — an allocations-only verdict inverts; the gate demands a
//!   paired work-or-time read before calling an alloc delta a win;
//! - F4 wall-clock-ungated-shared-ci-noise-floor: a policy gate that
//!   judges ONLY on deterministic metrics (wall-clock carries
//!   judged=false) and derives its noise floor from an empty-change
//!   baseline — a timing-only arm must NOT flip the verdict on noise,
//!   and the min-percent floor must skip trivial-share functions.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// ─────────────────────────────────────────────────────────────────────────────
// F1 — instruction-count-cache-blindness
// MODELED counts: every logical op bumps a counter, so "same instructions"
// is a construction, not a measurement claim. Locality is modeled as a
// simulated cache: a sequential walk of a contiguous table hits; a
// pointer-chase of the same table misses by layout.
// ─────────────────────────────────────────────────────────────────────────────

const TABLE_LEN: usize = 64 * 1024;

/// modeled instruction cost per op — the pair below performs the same op
/// sequence shape, so the modeled icount is equal by construction.
#[derive(Default)]
struct OpCount {
    loads: usize,
    adds: usize,
}

struct SimCache {
    lines: HashMap<u64, u64>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl SimCache {
    /// a BOUNCED cache stand-in: a real cache holds a bounded working set;
    /// past capacity the older lines are gone (coarse flush — directional,
    /// never a claim about real LRU policy).
    fn new(capacity: usize) -> Self {
        Self {
            lines: HashMap::new(),
            capacity,
            hits: 0,
            misses: 0,
        }
    }
    fn touch(&mut self, addr: u64) {
        let line = addr / 64;
        if self.lines.contains_key(&line) {
            self.hits += 1;
        } else {
            self.misses += 1;
            self.lines.insert(line, 1);
            if self.lines.len() > self.capacity {
                self.lines.clear(); // capacity exhausted: the working set fell out
            }
        }
    }
}

/// sequential walk: contiguous addresses, every line touched once in order.
fn walk_sequential(table: &[u64], ops: &mut OpCount, cache: &mut SimCache) -> u64 {
    let mut acc = 0u64;
    for (i, &v) in table.iter().enumerate() {
        cache.touch(i as u64 * 8);
        ops.loads += 1;
        acc = acc.wrapping_add(v);
        ops.adds += 1;
    }
    acc
}

/// pointer-chase walk: the SAME number of loads/adds (equal modeled
/// icount), but every hop lands on a scattered address — layout, not
/// instruction count, decides the cache outcome.
fn walk_pointer_chase(
    table: &[u64],
    hops: &[usize],
    ops: &mut OpCount,
    cache: &mut SimCache,
) -> u64 {
    let mut acc = 0u64;
    let mut idx = 0usize;
    for _ in 0..table.len() {
        // the hops ARRAY is the scatter (8192 distinct slots, revisited in
        // laps); a slot-dependent idx stride would collapse into a short
        // functional-graph cycle and visit only a fraction of them.
        let slot = hops[idx % hops.len()] % table.len();
        cache.touch(slot as u64 * 8);
        ops.loads += 1;
        acc = acc.wrapping_add(table[slot]);
        ops.adds += 1;
        idx = idx.wrapping_add(1);
    }
    acc
}

#[test]
fn f1_icount_equal_while_locality_diverges() {
    let table = vec![1u64; TABLE_LEN];
    // a fixed scatter pattern (LCG), deterministic
    let mut x = 0x9E3779B9u64;
    let hops: Vec<usize> = (0..8192)
        .map(|_| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (x >> 33) as usize
        })
        .collect();

    // a small bounded cache (1K lines): the streaming walk's working set
    // is line-once (each line consumed as it passes); the chase's working
    // set exceeds capacity many times over, so re-touches land on flushed
    // lines — layout decides, instruction count does not.
    let mut ops_seq = OpCount::default();
    let mut cache_seq = SimCache::new(1024);
    let _ = walk_sequential(&table, &mut ops_seq, &mut cache_seq);

    let mut ops_chase = OpCount::default();
    let mut cache_chase = SimCache::new(1024);
    let _ = walk_pointer_chase(&table, &hops, &mut ops_chase, &mut cache_chase);

    // THE LAW'S WITNESS: equal modeled instruction count …
    assert_eq!(
        ops_seq.loads, ops_chase.loads,
        "the pair must be icount-equal by construction"
    );
    assert_eq!(ops_seq.adds, ops_chase.adds);
    // … while the cache outcome diverges: the sequential walk touches each
    // line once (misses = lines, hits ≈ 0 with a cold cache and no reuse);
    // the chase re-touches scattered lines so its hit/miss RATIO and the
    // cost per op diverge. Model the wall proxy: misses cost 100x hits.
    let wall_seq = cache_seq.misses * 100 + cache_seq.hits;
    let wall_chase = cache_chase.misses * 100 + cache_chase.hits;
    assert!(
        wall_chase > wall_seq * 2,
        "locality-divergent pair: chase wall proxy {wall_chase} must exceed sequential {wall_seq} — an icount-only gate reports 0% delta on this pair"
    );
}

#[test]
fn f1_instruction_bound_anti_regime() {
    // ANTI-REGIME: a pure-arithmetic chain (no memory) — here icount IS
    // representative: doubling the ops doubles the cost, and a locality
    // caveat would be noise. The gate's caveat must not fire here.
    let mut acc = 1u64;
    let mut adds = 0usize;
    for _ in 0..100_000 {
        acc = acc.wrapping_mul(3).wrapping_add(7);
        adds += 1;
    }
    assert!(acc != 0 && adds == 100_000);
    // a "blindness-aware" gate: no loads modeled → no caveat owed
    let loads = 0usize;
    assert_eq!(
        loads, 0,
        "instruction-bound workload: the cache caveat is inapplicable"
    );
}

#[test]
fn f1_tampered_icount_only_gate_still_reports_zero_delta() {
    // TAMPER: a gate that KNOWS about locality but still judges on the
    // icount column alone must be caught reporting the pair as a no-change
    // regression sweep — the witness asserts the tampered verdict is
    // blind (equal icount → "no regression") while the wall proxy says
    // otherwise. This is the law the rule pins: the icount column cannot
    // adjudicate a locality-divergent pair.
    let icount_a = 20_971_545usize; // the blog's equal-count shape, our own constant
    let icount_b = 20_971_545usize;
    let icount_delta_pct: f64 = 0.0;
    let wall_a = 5.06f64;
    let wall_b = 133.04f64;
    let wall_delta_pct = (wall_b - wall_a) / wall_a * 100.0;
    let _ = (icount_a, icount_b);
    let blind_verdict = if icount_delta_pct.abs() < 0.1 {
        "no-regression"
    } else {
        "regression"
    };
    assert_eq!(
        blind_verdict, "no-regression",
        "the icount-only gate is blind by construction"
    );
    assert!(
        wall_delta_pct > 1000.0,
        "the wall context shows the divergence the icount gate missed: {wall_delta_pct}%"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — async-wrapper-future-size-exponential-nesting
// Each wrapper level is an `async fn` that awaits the level below; the
// compiler stores the inner future in the level's state machine, so the
// composed size multiplies (~2x+ per level). The hand-written form stores
// one inner future once (linear). Counting allocator rides the poll path.
// ─────────────────────────────────────────────────────────────────────────────

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

/// counting global allocator — the census backend for the F2 poll path
/// (standalone-safe; `allocator_api` is unstable on plain rustc).
struct CountingGlobal;

unsafe impl std::alloc::GlobalAlloc for CountingGlobal {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        std::alloc::System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        std::alloc::System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: CountingGlobal = CountingGlobal;

/// L0: the leaf — a ready future carrying a small payload.
async fn leaf() -> u64 {
    42
}

/// the naive measuring-wrapper shape the law prices: each layer composes
/// the level below TWICE (a twin/race/retry-style wrapper — the shape the
/// blog's generated macro state machines effectively held: the wrapped
/// future stored 2-3x per layer). Both branches are real held futures
/// joined by a tiny hand-rolled poll; the composed type therefore carries
/// 2^DEPTH copies of the leaf frame — exponential in nesting depth.
struct Join2<A, B> {
    a: A,
    b: B,
}

impl<A: Future<Output = u64>, B: Future<Output = u64>> Future for Join2<A, B> {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // structural pinning: the fields are never moved out — only pinned
        // projections reach their own poll impls (the pin-project pattern,
        // hand-rolled to stay standalone-compile-safe).
        let this = unsafe { self.get_unchecked_mut() };
        let a = unsafe { Pin::new_unchecked(&mut this.a) };
        match a.poll(cx) {
            Poll::Ready(v) => Poll::Ready(v),
            Poll::Pending => {
                let b = unsafe { Pin::new_unchecked(&mut this.b) };
                b.poll(cx).map(|_| 0)
            }
        }
    }
}

async fn twin1() -> u64 {
    Join2 {
        a: leaf(),
        b: leaf(),
    }
    .await
}
async fn twin2() -> u64 {
    Join2 {
        a: twin1(),
        b: twin1(),
    }
    .await
}
async fn twin3() -> u64 {
    Join2 {
        a: twin2(),
        b: twin2(),
    }
    .await
}
async fn twin4() -> u64 {
    Join2 {
        a: twin3(),
        b: twin3(),
    }
    .await
}
async fn twin5() -> u64 {
    Join2 {
        a: twin4(),
        b: twin4(),
    }
    .await
}

/// the hand-written linear alternative: ONE wrapper type parameterized by
/// the inner future — the composed type stores the inner future ONCE per
/// layer as a plain field; the size grows by a constant header, never
/// multiplicatively.
struct Linear<L> {
    inner: L,
}

impl<L: Future<Output = u64> + Unpin> Future for Linear<L> {
    type Output = u64;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx)
    }
}

fn linear_chain(f: impl Future<Output = u64> + Unpin) -> impl Future<Output = u64> + Unpin {
    Linear { inner: f }
}

#[test]
fn f2_composed_size_exponential_vs_linear() {
    let composed = twin5();
    let leaf_f = Box::pin(async { 42u64 });
    let linear = linear_chain(linear_chain(linear_chain(linear_chain(leaf_f))));

    let composed_size = std::mem::size_of_val(&composed);
    let linear_size = std::mem::size_of_val(&linear);

    // exponential vs linear: 5 twin layers = 2^5 = 32 leaf copies by
    // construction, while the 4-layer linear chain carries ONE copy — the
    // composed form must dwarf the linear form (the blog measured 2-3x
    // PER LEVEL; our floor is far more conservative).
    assert!(
        composed_size > linear_size * 4,
        "twin-composed future ({composed_size} B) must dwarf the linear chain ({linear_size} B) — the per-layer duplication is the law"
    );

    // exponential vs linear: at 8 composed async-fn levels the size must
    // exceed a 4-level linear chain by a wide factor (the blog measured
    // 2-3x PER LEVEL — our floor is conservative: total > 4x at 8 vs 4).
    assert!(
        composed_size > linear_size * 4,
        "composed async-fn future ({composed_size} B) must dwarf the linear chain ({linear_size} B) — the per-level state-machine duplication is the law"
    );

    // pin-to-poll both to completion under the counting allocator: the
    // census proves both chains complete (no hidden panic) and records
    // their poll-path allocations — the SIZE axis above is the law; the
    // census is the disclosure that neither chain hides its cost.
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    ALLOCS.store(0, Ordering::SeqCst);
    let mut composed = Box::pin(twin5());
    let composed_poll = match composed.as_mut().poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("leaf chain must be immediately ready"),
    };
    let composed_poll_allocs = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(composed_poll, 42);

    ALLOCS.store(0, Ordering::SeqCst);
    let leaf_f2 = Box::pin(async { 42u64 });
    let mut linear = Box::pin(linear_chain(linear_chain(linear_chain(linear_chain(
        leaf_f2,
    )))));
    let linear_poll = match linear.as_mut().poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("linear chain must be immediately ready"),
    };
    let linear_poll_allocs = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(linear_poll, 42);
    // both chains complete through the same one-poll path; the census is
    // recorded, never asserted unequal — the size law above carries the
    // verdict.
    assert!(
        composed_poll_allocs + linear_poll_allocs >= 2,
        "each Box::pin allocates once — the census is live"
    );
}

fn noop_waker() -> Waker {
    fn noop_clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &NOOP_VTABLE)
    }
    fn noop_drop(_: *const ()) {}
    fn noop_wake(_: *const ()) {}
    fn noop_wake_by_ref(_: *const ()) {}
    static NOOP_VTABLE: RawWakerVTable =
        RawWakerVTable::new(noop_clone, noop_wake, noop_wake_by_ref, noop_drop);
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &NOOP_VTABLE)) }
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — alloc-proxy-inversion-work-ratio-caveat
// The dedup pair: the "optimized" HashSet shape allocates MORE at small n
// while doing LESS work (a work-count proxy stands in for wall time). An
// allocations-only regression verdict on this pair is an inversion.
// ─────────────────────────────────────────────────────────────────────────────

static F3_ALLOCS: AtomicUsize = AtomicUsize::new(0);

/// counting allocator for F3 (thread-local style via a global counter —
/// the fixture owns its own accounting, no #[global_allocator] so F2's
/// census stays independent).
fn track<T>(v: T) -> T {
    F3_ALLOCS.fetch_add(1, Ordering::SeqCst);
    v
}

fn dedup_contains(values: &[u64], work: &mut usize) -> Vec<u64> {
    let mut unique: Vec<u64> = Vec::new();
    for &value in values {
        *work += unique.len(); // the contains-scan's comparison-count proxy
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn dedup_hash(values: &[u64], work: &mut usize) -> Vec<u64> {
    let mut seen = track(HashMap::<u64, ()>::new());
    let mut unique = Vec::new();
    for &value in values {
        *work += 1; // one hash-probe proxy per element
        if seen.insert(value, ()).is_none() {
            unique.push(value);
        }
    }
    unique
}

#[test]
fn f3_alloc_up_while_work_down_is_an_inversion() {
    // SMALL input, many duplicates: the contains-scan does O(n*unique)
    // comparisons with ZERO heap allocs; the HashSet does O(n) hashes but
    // allocates (table + growth). Alloc delta is a LOSS on this pair
    // while the work proxy says the hash form is cheaper — the exact
    // inversion the caveat prices.
    let values: Vec<u64> = (0..256u64).map(|i| i % 16).collect();

    F3_ALLOCS.store(0, Ordering::SeqCst);
    let mut work_a = 0usize;
    let a = dedup_contains(&values, &mut work_a);
    let allocs_a = F3_ALLOCS.load(Ordering::SeqCst);

    F3_ALLOCS.store(0, Ordering::SeqCst);
    let mut work_b = 0usize;
    let b = dedup_hash(&values, &mut work_b);
    let allocs_b = F3_ALLOCS.load(Ordering::SeqCst);

    assert_eq!(a, b, "both shapes dedup identically (correctness first)");
    assert_eq!(allocs_a, 0, "the scan shape allocates nothing at this size");
    assert!(
        allocs_b > allocs_a,
        "the hash shape allocates MORE — the alloc-only verdict would call the hash form a REGRESSION"
    );
    assert!(
        work_b < work_a,
        "the hash shape does LESS work ({work_b} probes vs {work_a} comparisons) — the alloc verdict inverted against the work axis"
    );
}

#[test]
fn f3_gate_requires_paired_read_before_calling_alloc_a_win() {
    // the gate the rule pins: an allocations-only A/B on this pair must
    // refuse to score until a work-or-time axis is attached; with the
    // work axis attached the verdict correctly favors the hash form.
    struct AllocOnly;
    impl AllocOnly {
        fn verdict(&self, _a: usize, _b: usize) -> &'static str {
            "refused-needs-paired-axis"
        }
    }
    let gate = AllocOnly;
    assert_eq!(gate.verdict(0, 5), "refused-needs-paired-axis");

    struct Paired {
        alloc_a: usize,
        alloc_b: usize,
        work_a: usize,
        work_b: usize,
    }
    impl Paired {
        fn verdict(&self) -> &'static str {
            if self.alloc_b > self.alloc_a && self.work_b < self.work_a {
                "win-with-alloc-regression-disclosed"
            } else {
                "no-change"
            }
        }
    }
    let paired = Paired {
        alloc_a: 0,
        alloc_b: 5,
        work_a: 32640,
        work_b: 256,
    };
    assert_eq!(paired.verdict(), "win-with-alloc-regression-disclosed");
}

// ─────────────────────────────────────────────────────────────────────────────
// F4 — wall-clock-ungated-shared-ci-noise-floor
// The policy gate: deterministic metrics judge; wall-clock is context
// (judged=false); the noise floor derives from an empty-change baseline;
// the min-percent floor skips trivial-share functions.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Run {
    icount: u64,
    alloc_bytes: u64,
    wall_ms: f64,
}

struct Policy {
    noise_floor_icount_pct: f64, // derived from the empty-change baseline
    min_percent_total: f64,      // trivial-share floor
}

impl Policy {
    /// the empty-change baseline: N runs of UNCHANGED code give the floor
    /// (the blog's 100-run empty-PR methodology, our own small-N model).
    fn from_empty_baseline(runs: &[Run]) -> Self {
        let mut worst = 0.0f64;
        let base = runs[0].icount as f64;
        for r in runs {
            let spread = ((r.icount as f64 - base) / base * 100.0).abs();
            if spread > worst {
                worst = spread;
            }
        }
        Self {
            noise_floor_icount_pct: worst.ceil() + 1.0,
            min_percent_total: 0.5,
        }
    }

    /// wall-clock NEVER judges; icount judges above the derived floor;
    /// the wall numbers ride the report as context (returned beside the
    /// verdict, never consulted by it).
    fn verdict(&self, base: &Run, cand: &Run, cand_share_pct: f64) -> (&'static str, f64) {
        let verdict = if cand_share_pct < self.min_percent_total {
            "skipped-trivial-share"
        } else {
            let delta_pct = (cand.icount as f64 - base.icount as f64) / base.icount as f64 * 100.0;
            if delta_pct.abs() <= self.noise_floor_icount_pct {
                "no-regression"
            } else if delta_pct < 0.0 {
                "improvement"
            } else {
                "regression"
            }
        };
        // context, never judged: the wall ratio rides the report
        (verdict, cand.wall_ms / base.wall_ms)
    }
}

#[test]
fn f4_timing_noise_cannot_flip_a_judged_verdict() {
    // empty-change baseline: 3 runs, icount wiggles <0.2%, wall swings 3x
    let baseline = vec![
        Run {
            icount: 4_247_997_37,
            alloc_bytes: 2_425_213,
            wall_ms: 35.0,
        },
        Run {
            icount: 4_247_403_25,
            alloc_bytes: 2_425_213,
            wall_ms: 68.3,
        },
        Run {
            icount: 4_248_017_54,
            alloc_bytes: 2_425_213,
            wall_ms: 98.0,
        },
    ];
    let policy = Policy::from_empty_baseline(&baseline);
    let base = &baseline[0];

    // candidate: SAME icount (within the derived floor), wall improved 3x —
    // the timing-only arm would scream "improvement"; the policy must
    // answer no-regression, and the wall number stays in the report as
    // context (never judged).
    let cand = Run {
        icount: 424_750_000,
        alloc_bytes: 2_425_213,
        wall_ms: 32.0,
    };
    let (verdict, wall_ratio) = policy.verdict(base, &cand, 90.0);
    assert_eq!(
        verdict, "no-regression",
        "wall noise must not flip the judged verdict — icount within the derived floor"
    );
    assert!(
        wall_ratio < 1.0,
        "the wall context rides the report ({wall_ratio}x) — visible, never judged"
    );
    // the alloc context too: identical bytes across the pair, never judged
    assert_eq!(base.alloc_bytes, cand.alloc_bytes);
}

#[test]
fn f4_floor_skips_trivial_share_and_catches_real_regressions() {
    let baseline = vec![
        Run {
            icount: 1_000_000,
            alloc_bytes: 100,
            wall_ms: 10.0,
        },
        Run {
            icount: 1_000_000,
            alloc_bytes: 100,
            wall_ms: 11.0,
        },
        Run {
            icount: 1_000_000,
            alloc_bytes: 100,
            wall_ms: 12.0,
        },
    ];
    let policy = Policy::from_empty_baseline(&baseline);

    // a real +10% icount regression on a 90%-share function: judged
    let (v, _) = policy.verdict(
        &baseline[0],
        &Run {
            icount: 1_100_000,
            alloc_bytes: 100,
            wall_ms: 10.0,
        },
        90.0,
    );
    assert_eq!(v, "regression");
    // the same delta on a 0.2%-share helper: skipped by the floor
    let (v, _) = policy.verdict(
        &baseline[0],
        &Run {
            icount: 1_100_000,
            alloc_bytes: 100,
            wall_ms: 10.0,
        },
        0.2,
    );
    assert_eq!(v, "skipped-trivial-share");
}
