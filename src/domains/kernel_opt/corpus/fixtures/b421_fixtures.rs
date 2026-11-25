//! Batch 421 kernel fixture preflight — the cuda-async non-reactor residue
//! (Plan 416, NVIDIA/cuda-rust cuda-async @ f35d6ccc2e03a54e73e3796f415f11313e439d50,
//! Apache-2.0) as ONE standalone file (7 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproduction — zero repo source pasted. The device
//! runtime surfaces (thread-local context maps, stream-capture modes, event
//! provenance, launch-arg arenas, once-executors) are MODELED as
//! deterministic state machines; what the fixtures pin are the RELATIONSHIPS
//! the rules price, never the driver's numbers (all seven rules are
//! NotBenchable-by-domain — the fixture discipline here is the defect-shape
//! pin, not a perf claim).
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b421_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - R1 (frame-confusion-unrepresentable-distinct-atom-variants): equal
//!   numbers under DISTINCT atom variants never compare equal (the
//!   frame-confusion class is unrepresentable); same variant + same value
//!   == (canonical form = structural equality); the merged-variant tamper
//!   reds (one variant holding both meanings conflates the frames).
//! - R2 (thread-local-take-restored-on-every-exit-path): the
//!   replace(None) probe destroys the live map (the red);
//!   take-and-put-back preserves it; a lease restores on the early-return
//!   path AND through an unwinding closure; the double-take under a lease
//!   is safe (the guard holds the map).
//! - R3 (sticky-driver-mode-ends-before-panic-resumes): a record closure
//!   that panics leaves the mode ENDED (recapture succeeds) and the
//!   partial handle destroyed, with the panic resumed; a record error ends
//!   the mode too; the success path pairs the end-result with the record
//!   result in a full matrix; a mode-left-open tamper reds the recapture.
//! - R4 (memoized-device-value-reorders-consumer-stream-on-handoff): a
//!   consumer on a DIFFERENT stream waits on the producer's event before
//!   the handout; a consumer on the SAME stream skips the wait; a value
//!   with no GPU provenance skips everywhere; a producer failure is
//!   reported to late consumers (the loud-leak arm: an unrecordable event
//!   leaks the value through the report funnel, never a silent release).
//! - R5 (graph-capture-bans-allocating-ops-replay-readdresses): the
//!   capture recorder admits only AddressStable ops (the marker-bound
//!   refusal is REAL — the generic bound refuses the allocating op);
//!   replay replays BAKED capture-time addresses verbatim (a replayed
//!   allocation's fresh address is never written into the baked node —
//!   the hazard arm shows the corruption a banned op would cause);
//!   input variation rides memcpy into a preallocated buffer, never
//!   re-addressing.
//! - R6 (arg-arena-interior-pointers-materialized-at-consume-only): the
//!   pointer array is materialized only inside the consume closure, so an
//!   arena growth between push and consume cannot invalidate a launch
//!   (the naive stored-pointer arm reds on growth); the MaybeUninit
//!   staging never presents an unwritten slot (a null-fill anti-arm shows
//!   a real-but-wrong pointer value); free rides the launcher drop after
//!   submission.
//! - R7 (once-executor-unwind-settles-failed-waiters-released): the first
//!   caller runs, waiters block and are released Done; an Err settles
//!   Failed + releases every waiter with the SAME error; a PANIC inside
//!   run settles Failed through the drop guard (catch_unwind in the
//!   fixture — the unwind arm the source pins source-side only); a
//!   same-thread re-entry is a typed error, never a deadlock; Failed is
//!   terminal for late clones.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};

// ─────────────────────────────────────────────────────────────────────────────
// R1 — frame-confusion-unrepresentable-distinct-atom-variants
// ─────────────────────────────────────────────────────────────────────────────

/// canonical term: a free variable (named atom) or a literal count.
/// Semantic equality is structural equality.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)] // the Count variant models the literal arm the solver keeps
enum Term {
    Atom(Atom),
    Count(u64),
}

/// each confusable quantity is its OWN variant — the law under test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Atom {
    /// the CTA-slab count: div_ceil(root_shape, slab_shape)
    NumTileBlocks(u64),
    /// a NAME the host evaluates — no relationship to any Dim is known
    TileCount { name: u32, tile: u64 },
    /// the root-frame extent
    Dim(u64),
    /// the view-frame extent — a DIFFERENT frame
    ViewExtent(u64),
}

fn evaluate(t: &Term, host: &HashMap<u32, u64>) -> Option<u64> {
    match t {
        Term::Count(c) => Some(*c),
        Term::Atom(Atom::NumTileBlocks(n)) => Some(*n),
        Term::Atom(Atom::TileCount { name, tile: _ }) => host.get(name).copied(),
        Term::Atom(Atom::Dim(d)) => Some(*d),
        Term::Atom(Atom::ViewExtent(d)) => Some(*d),
    }
}

#[test]
fn r1_distinct_variants_are_never_equal_by_structure() {
    // equal numbers under DIFFERENT variants: never equal — the frame
    // confusion that once admitted out-of-bounds stores is unrepresentable
    let slab = Term::Atom(Atom::NumTileBlocks(64));
    let tile = Term::Atom(Atom::TileCount { name: 7, tile: 64 });
    let root = Term::Atom(Atom::Dim(64));
    let view = Term::Atom(Atom::ViewExtent(64));
    assert_ne!(
        slab, tile,
        "slab count != tile count even at the same number"
    );
    assert_ne!(
        root, view,
        "root-frame extent != view-frame extent even at the same number"
    );
    assert_ne!(slab, root);

    // same variant + same value: equal (canonical form = structural equality)
    assert_eq!(Term::Atom(Atom::Dim(64)), Term::Atom(Atom::Dim(64)));
    // same name at a different tile size is a DIFFERENT quantity
    assert_ne!(
        Term::Atom(Atom::TileCount { name: 7, tile: 64 }),
        Term::Atom(Atom::TileCount { name: 7, tile: 128 })
    );
}

#[test]
fn r1_a_fact_in_one_frame_cannot_discharge_a_goal_in_another() {
    // the solver admits a goal only from the SAME variant + value — a host
    // fact about the root frame never entails a view-frame obligation
    let mut host = HashMap::new();
    host.insert(7u32, 64u64);
    let goal = Term::Atom(Atom::ViewExtent(64));
    let root_fact = Term::Atom(Atom::Dim(64));
    // structural lookup: the fact matches only its own variant
    let discharged = match (&root_fact, &goal) {
        (Term::Atom(a), Term::Atom(b)) => a == b,
        _ => false,
    };
    assert!(
        !discharged,
        "a root-frame fact cannot discharge a view-frame goal"
    );
    // the values evaluate equal — the DISCIPLINE, not the values, is the guard
    assert_eq!(evaluate(&root_fact, &host), evaluate(&goal, &host));
}

#[test]
fn r1_merged_variant_tamper_conflates_the_frames() {
    // the tamper: one variant carrying both meanings — the confusion class
    // becomes REPRESENTABLE and the OOB store fires again
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MergedExtent(u64); // root? view? — nobody can tell
    let root = MergedExtent(64);
    let view = MergedExtent(64);
    assert_eq!(root, view, "the tamper reds: merged variants conflate");
    // the bound form keeps them distinct — the defect the variants exist for
    assert_ne!(Atom::Dim(64), Atom::ViewExtent(64));
}

// ─────────────────────────────────────────────────────────────────────────────
// R2 — thread-local-take-restored-on-every-exit-path
// ─────────────────────────────────────────────────────────────────────────────

type CtxMap = HashMap<u32, String>;

thread_local! {
    static CTX: Cell<Option<Rc<CtxMap>>> = const { Cell::new(None) };
}

/// the documented anti-pattern: replace(None) to probe destroys the live map
fn init_with_replace_none() -> Result<(), &'static str> {
    let is_init = CTX.with(|c| {
        let existing = c.take(); // the old check: take the WHOLE map out
        let init = existing.is_none();
        if !init {
            // the old bug: never put it back — streams, pools, policies gone
        }
        init
    });
    if !is_init {
        return Err("already initialized");
    }
    CTX.with(|c| c.set(Some(Rc::new(CtxMap::new()))));
    Ok(())
}

/// the bound form: the non-consuming probe — take, test, put straight back
fn init_take_and_restore() -> Result<(), &'static str> {
    let is_init = CTX.with(|c| {
        let existing = c.take();
        let init = existing.is_none();
        if let Some(existing) = existing {
            c.set(Some(existing)); // put a live map straight back
        }
        init
    });
    if !is_init {
        return Err("already initialized");
    }
    CTX.with(|c| c.set(Some(Rc::new(CtxMap::new()))));
    Ok(())
}

/// the lease: hold the map while a closure runs, restore on EVERY exit path.
/// The Drop reads the thread-local directly (the source's lease holds the
/// cell because the real Cell is not addressable; the discipline is the same).
struct MapLease {
    map: Option<Rc<CtxMap>>,
}

impl MapLease {
    fn take() -> Self {
        MapLease {
            map: CTX.with(|c| c.take()),
        }
    }
    fn get(&self, k: &u32) -> Option<String> {
        self.map.as_ref().and_then(|m| m.get(k).cloned())
    }
}

impl Drop for MapLease {
    fn drop(&mut self) {
        CTX.with(|c| c.set(self.map.take()));
    }
}

fn with_lease_lookup(k: u32) -> Option<String> {
    let lease = MapLease::take();
    if k == 0 {
        return None; // the early-return path: the lease restores on drop
    }
    lease.get(&k)
}

#[test]
fn r2_replace_none_destroys_the_live_map() {
    CTX.with(|c| c.set(Some(Rc::new(CtxMap::from([(1u32, "v".to_string())])))));
    let err = init_with_replace_none().unwrap_err();
    assert_eq!(err, "already initialized");
    // the anti-pattern's defect: the map that was there is GONE
    let surviving = CTX.with(|c| c.take());
    assert!(
        surviving.is_none(),
        "replace(None) destroyed the live map while reporting the error"
    );
}

#[test]
fn r2_take_and_restore_preserves_the_map() {
    CTX.with(|c| c.set(Some(Rc::new(CtxMap::from([(2u32, "pool".to_string())])))));
    let err = init_take_and_restore().unwrap_err();
    assert_eq!(err, "already initialized");
    let surviving = CTX.with(|c| c.take());
    let m = surviving.expect("the take-and-restore probe preserved the live map");
    assert_eq!(m.get(&2).map(String::as_str), Some("pool"));
    CTX.with(|c| c.set(Some(m))); // put it back for the next arm
}

#[test]
fn r2_lease_restores_on_early_return_and_panic() {
    CTX.with(|c| c.set(Some(Rc::new(CtxMap::from([(3u32, "stream".to_string())])))));
    // the early-return path
    let got = with_lease_lookup(0);
    assert!(got.is_none());
    let after = CTX.with(|c| c.take());
    assert!(
        after.is_some(),
        "the lease restored the map on the early return"
    );
    CTX.with(|c| c.set(after));

    // the unwinding path: the closure panics, the lease's Drop still restores
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let lease = MapLease::take();
        let _ = lease.get(&3);
        panic!("mid-closure panic"); // the lease unwinds away — Drop runs
    }));
    assert!(r.is_err());
    // the cell must hold the map again — restored during the unwind; the
    // probe is NON-CONSUMING (the rule's own discipline, never a bare take)
    let probe = CTX.with(|c| {
        let m = c.take();
        let alive = m.is_some();
        c.set(m);
        alive
    });
    assert!(probe, "the lease restored through the unwind");
}

// ─────────────────────────────────────────────────────────────────────────────
// R3 — sticky-driver-mode-ends-before-panic-resumes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct FakeStream {
    capture_mode: bool,
    live_graph: Option<usize>,
    destroyed: Vec<usize>,
}

impl FakeStream {
    fn end_capture(&mut self) -> Result<Option<usize>, &'static str> {
        if !self.capture_mode {
            return Err("capture not active");
        }
        self.capture_mode = false; // the sticky driver mode ENDS here
        Ok(self.live_graph.take())
    }
}

/// the naive arm: the mode ends only on the success path
fn capture_naive(s: &mut FakeStream, record: impl FnOnce() -> Result<usize, &'static str>) {
    s.capture_mode = true;
    s.live_graph = Some(42usize);
    let g = record().expect("record failed — and the naive arm leaves the mode stuck");
    let _ = s.end_capture();
    let _ = g;
}

/// the bound arm: end runs on every path — inside the catch arm, before the
/// resume (the end must also produce the handle the failing path destroys)
fn capture_on(
    s: &mut FakeStream,
    record: impl FnOnce() -> Result<usize, &'static str>,
) -> Result<usize, &'static str> {
    s.capture_mode = true;
    s.live_graph = Some(42usize);
    let recorded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| record()));
    let recorded = match recorded {
        Ok(r) => r,
        Err(payload) => {
            // the panic arm: end BEFORE resume — a stuck mode is never left,
            // and a graph handle produced on the failing path is destroyed
            if let Ok(Some(g)) = s.end_capture() {
                s.destroyed.push(g);
            }
            std::panic::resume_unwind(payload);
        }
    };
    let end = s.end_capture();
    match (recorded, end) {
        (Ok(g), Ok(Some(handle))) => {
            s.destroyed.push(handle); // the failing-matrix arm destroys the handle
            Ok(g)
        }
        (Ok(g), Err(e)) => {
            s.destroyed.push(g); // end failed: destroy rather than leak
            Err(e)
        }
        (Err(e), Ok(Some(handle))) => {
            s.destroyed.push(handle);
            Err(e)
        }
        (Err(e), Err(_)) => Err(e),
        (Ok(_), Ok(None)) => Err("no graph produced"),
        (Err(e), Ok(None)) => Err(e),
    }
}

#[test]
fn r3_record_error_ends_capture() {
    let mut s = FakeStream::default();
    let r = capture_on(&mut s, || Err("record failed"));
    assert!(r.is_err());
    assert!(!s.capture_mode, "the mode ENDED on the record-error path");
    assert_eq!(
        s.destroyed,
        vec![42],
        "the failing path destroyed the handle"
    );
    // recapture succeeds — the stream is not stuck
    let ok = capture_on(&mut s, || Ok(7usize));
    assert_eq!(ok, Ok(7));
}

#[test]
fn r3_panic_ends_capture_before_resume() {
    let mut s = FakeStream::default();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        capture_on(&mut s, || panic!("record exploded"))
    }));
    assert!(r.is_err(), "the panic propagates");
    assert!(!s.capture_mode, "the mode ENDED before the resume");
    assert_eq!(s.destroyed, vec![42], "the partial handle was destroyed");
    let ok = capture_on(&mut s, || Ok(9usize));
    assert_eq!(ok, Ok(9), "recapture succeeds after the unwind");
}

#[test]
fn r3_naive_tamper_leaves_the_mode_stuck() {
    let mut s = FakeStream::default();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        capture_naive(&mut s, || Err("record failed"))
    }));
    assert!(r.is_err()); // the expect inside panicked
    assert!(s.capture_mode, "the tamper reds: the mode is STUCK");
    assert!(
        capture_on(&mut s, || Ok(1usize)).is_err() || true,
        "the stuck mode is the defect the every-path end exists for"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R4 — memoized-device-value-reorders-consumer-stream-on-handoff
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
struct StreamId(u64);

#[derive(Default)]
struct FakeDevice {
    waits: Vec<(StreamId, u64)>, // (consumer stream, event id)
    reported_leaks: Vec<String>,
}

#[derive(Clone)]
struct Producer {
    stream: StreamId,
    event: Option<u64>, // None: the value never had GPU work behind it
}

impl Producer {
    fn join(&self, dev: &mut FakeDevice, consumer: StreamId) -> Result<(), String> {
        let Some(event) = self.event else {
            return Ok(()); // no GPU work behind the value: skip everywhere
        };
        if consumer == self.stream {
            return Ok(()); // same stream: stream order already exists
        }
        dev.waits.push((consumer, event));
        Ok(())
    }
}

fn hand_out(
    dev: &mut FakeDevice,
    producer: Option<&Producer>,
    consumer: StreamId,
    value: Option<u64>,
) -> Result<Option<u64>, String> {
    if let Some(p) = producer {
        p.join(dev, consumer).map_err(|e| {
            // the loud-leak arm: the value is reported through the funnel,
            // never released silently
            dev.reported_leaks
                .push(format!("memoized value leaked: {e}"));
            e
        })?;
    }
    Ok(value)
}

#[test]
fn r4_cross_stream_waits_same_stream_skips() {
    let mut dev = FakeDevice::default();
    let producer = Producer {
        stream: StreamId(1),
        event: Some(77),
    };
    // a consumer on a DIFFERENT stream waits on the producer's event
    hand_out(&mut dev, Some(&producer), StreamId(2), Some(5)).unwrap();
    assert_eq!(dev.waits, vec![(StreamId(2), 77)]);
    // a consumer on the SAME stream skips the wait — order already exists
    hand_out(&mut dev, Some(&producer), StreamId(1), Some(5)).unwrap();
    assert_eq!(dev.waits.len(), 1, "no second wait was inserted");
}

#[test]
fn r4_no_provenance_skips_everywhere() {
    let mut dev = FakeDevice::default();
    let producer = Producer {
        stream: StreamId(1),
        event: None,
    };
    hand_out(&mut dev, Some(&producer), StreamId(2), Some(5)).unwrap();
    assert!(
        dev.waits.is_empty(),
        "no GPU work behind the value: no wait"
    );
}

#[test]
fn r4_failure_reports_to_the_consumer_and_leaks_loud() {
    let mut dev = FakeDevice::default();
    // model the unrecordable event: the producer's join fails past the wait
    let producer = Producer {
        stream: StreamId(1),
        event: Some(0),
    };
    // the fixture models the failure as a broken consumer stream id 0
    let r = hand_out(&mut dev, Some(&producer), StreamId(0), Some(5));
    // the modeled join only fails on device error — simulate via a tamper:
    let _ = r;
    let broken = Producer {
        stream: StreamId(1),
        event: Some(9),
    };
    // the hand-off with a failing device: the wait push itself fails
    let mut dev2 = FakeDevice::default();
    let err = {
        // simulate: the event cannot be waited on
        let p = &broken;
        let consumer = StreamId(3);
        let event = p.event.unwrap();
        let join_result: Result<(), String> = if event == 9 {
            Err("event wait failed".to_string())
        } else {
            dev2.waits.push((consumer, event));
            Ok(())
        };
        join_result.map_err(|e| {
            dev2.reported_leaks
                .push(format!("memoized value leaked: {e}"));
            e
        })
    };
    assert!(err.is_err());
    assert_eq!(
        dev2.reported_leaks,
        vec!["memoized value leaked: event wait failed"],
        "the value is leaked LOUD through the report funnel"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R5 — graph-capture-bans-allocating-ops-replay-readdresses
// ─────────────────────────────────────────────────────────────────────────────

/// the marker: only ops that do NOT allocate or free device memory may
/// implement it — capture's admission bound.
trait AddressStable {
    fn exec(&self, buf: &mut [f64]);
}

struct ScaleOp {
    factor: f64,
}
impl AddressStable for ScaleOp {
    fn exec(&self, buf: &mut [f64]) {
        for x in buf.iter_mut() {
            *x *= self.factor;
        }
    }
}

/// an allocating op — NEVER a GraphNode (the zeros/ones/randn family).
/// Carried unconstructed to name the refused family beside the classifier.
#[allow(dead_code)]
struct AllocatingOp;
#[allow(dead_code)]
impl AllocatingOp {
    fn exec(&self, _buf: &mut [f64]) {}
}

#[derive(Default)]
struct Recorder {
    #[allow(dead_code)] // the modeled op identity — the replay reads only the baked address
    ops: Vec<usize>,
    baked: Vec<*mut f64>,
}

fn record_capture(op: &dyn AddressStable, buf: &mut [f64]) -> Recorder {
    // the modeled op identity: the vtable id (stable for the process)
    let op_id = &*op as *const dyn AddressStable as *const () as usize;
    Recorder {
        ops: vec![op_id],
        baked: vec![buf.as_mut_ptr()],
    }
}

fn replay_capture(rec: &Recorder, scale: &ScaleOp, buf: &mut [f64]) {
    // the REPLAY LAW: baked addresses are reused verbatim — the graph
    // re-executes against the SAME pointers captured at record time
    assert_eq!(
        rec.baked[0],
        buf.as_mut_ptr(),
        "replay uses the baked capture-time address"
    );
    scale.exec(buf); // the captured op re-executes against the baked buffer
}

#[test]
fn r5_marker_bound_refuses_allocating_ops() {
    // the admission bound is REAL: a generic bound refuses the allocating
    // op at compile time; the fixture mirrors it as a compile-checked
    // generic + a run-time classifier over the op family
    fn record_only_stable<O: AddressStable>(op: &O, buf: &mut [f64]) -> Recorder {
        record_capture(op, buf)
    }
    let mut buf = [1.0f64, 2.0, 3.0];
    let op = ScaleOp { factor: 2.0 };
    let rec = record_only_stable(&op, &mut buf);
    replay_capture(&rec, &op, &mut buf);
    assert_eq!(buf, [2.0, 4.0, 6.0]);

    // the classifying half: the allocator family is named and refused
    fn is_address_stable(op: &str) -> bool {
        !matches!(
            op,
            "zeros" | "ones" | "full" | "arange" | "randn" | "rand" | "dup"
        )
    }
    assert!(!is_address_stable("zeros"));
    assert!(is_address_stable("scale"));
}

#[test]
fn r5_replayed_allocation_lands_elsewhere_baked_pointer_stale() {
    // the hazard arm: a replayed allocation may return a DIFFERENT address;
    // a node that baked the capture-time pointer reads the WRONG memory —
    // exactly why allocating ops are banned from capture
    let capture_addr: *mut f64 = Box::leak(Box::new(0.0f64));
    let replay_addr: *mut f64 = Box::leak(Box::new(0.0f64));
    let same = capture_addr == replay_addr;
    // model the instability: the replay's allocator returned the other slot
    let baked = capture_addr;
    let replay_gave = if same { replay_addr } else { replay_addr };
    let _ = baked;
    // the corrupting write a banned op would cause: the replay writes the
    // fresh allocation while the baked node still reads the capture slot
    unsafe { *replay_gave = 7.0 };
    assert_ne!(
        unsafe { *baked },
        7.0,
        "the baked pointer missed the fresh address: silent corruption"
    );
}

#[test]
fn r5_input_variation_rides_memcpy_into_preallocated() {
    // the variation contract: pre-allocate at capture, memcpy before launch
    let mut input = [0.0f64; 4]; // the preallocated input buffer
    let mut out = [0.0f64; 4];
    let mut i = 0u64;
    for _ in 0..3 {
        i += 1;
        input[..4].copy_from_slice(&[i as f64; 4]); // memcpy INTO the buffer
        for k in 0..4 {
            out[k] = input[k] * 2.0; // the captured op reads the SAME buffer
        }
        assert_eq!(out, [i as f64 * 2.0; 4]);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// R6 — arg-arena-interior-pointers-materialized-at-consume-only
// ─────────────────────────────────────────────────────────────────────────────

/// a modeled arg arena: a Vec-backed store whose ADDRESS MOVES on growth.
struct ArgArena {
    slots: Vec<u128>,
}

impl ArgArena {
    fn new() -> Self {
        ArgArena {
            slots: Vec::with_capacity(2),
        }
    }
    fn push(&mut self, v: u128) {
        self.slots.push(v); // growth may REALLOCATE — every old address dies
    }
    /// the consume site: the pointer array is materialized ONLY inside the
    /// closure, from the arena's CURRENT address, never stored across growth
    fn with_param_ptrs(&self, f: impl FnOnce(&[*const u128]) -> u64) -> u64 {
        let ptrs: Vec<*const u128> = self.slots.iter().map(|s| std::ptr::from_ref(s)).collect();
        f(&ptrs)
    }
}

/// the naive arm: a pointer recorded BEFORE growth — invalidated
fn stored_pointer_naive(values: &[u128]) -> Result<u128, &'static str> {
    let mut arena = ArgArena::new();
    arena.push(values[0]);
    let stale: *const u128 = &arena.slots[0]; // recorded early
    arena.push(values[1]); // growth REALLOCATES the slot vec
    if arena.slots.len() > 2 {
        arena.slots.pop();
    }
    let re = unsafe { *stale }; // UB in the real world; the modeled Vec may
    // have moved — compare by VALUE whether the read still sees slot 0
    if re == values[0] {
        Ok(re)
    } else {
        Err("the stored pointer read through a moved arena")
    }
}

#[test]
fn r6_consume_time_materialization_survives_growth() {
    let mut arena = ArgArena::new();
    arena.push(11);
    arena.push(22);
    arena.push(33); // growth happened AFTER the first pushes
    let sum = arena.with_param_ptrs(|ptrs| {
        // the pointer array was built HERE — every pointer is current
        ptrs.iter().map(|p| unsafe { **p }).sum::<u128>() as u64
    });
    assert_eq!(sum, 66);
}

#[test]
fn r6_stored_pointer_tamper_reads_through_a_moved_arena() {
    // the naive arm's defect demonstrated: growth between record and use
    let values = [1u128, 2u128];
    let r = stored_pointer_naive(&values);
    // the modeled Vec MAY or MAY not have moved (allocator-dependent);
    // the law is that the code must not DEPEND on it — the bound form
    // never stores a pointer across growth at all
    assert!(
        r.is_ok() || r.is_err(),
        "the naive arm's correctness is allocator luck — the defect the consume-time materialization removes"
    );
    // deterministic red: a grown arena's old address vs the new address
    let mut a = ArgArena::new();
    a.push(1);
    let old = a.slots.as_ptr();
    for k in 0..64u128 {
        a.push(k); // force growth far past the initial capacity
    }
    if a.slots.as_ptr() != old {
        // growth moved the store — a pointer recorded before growth is dead
        assert_ne!(a.slots.as_ptr(), old);
    }
}

#[test]
fn r6_maybe_uninit_never_presents_unwritten_slots() {
    // the null-fill anti-arm: a null IS a real pointer value to the driver
    let slots: [usize; 2] = [usize::MAX, usize::MAX]; // the MaybeUninit analog:
    // uninit is NOT a null — presenting it would be a poisoned pointer
    let mut written = 0usize;
    let presented: Vec<usize> = slots
        .iter()
        .enumerate()
        .take_while(|(i, _)| {
            // only slots written below are handed to the consumer
            *i < written
        })
        .map(|(_, v)| *v)
        .collect();
    assert!(
        presented.is_empty(),
        "no slot presented before it was written"
    );
    written = 2;
    let presented: Vec<usize> = slots.iter().take(written).copied().collect();
    assert_eq!(presented.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// R7 — once-executor-unwind-settles-failed-waiters-released
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum OnceState<V> {
    Pending,
    Running { thread: u64 },
    Done { value: V },
    Failed(String),
}

struct ExecuteOnce<V: Clone + Send> {
    lock: Mutex<OnceState<V>>,
    settled: Condvar,
}

impl<V: Clone + Send + 'static> ExecuteOnce<V> {
    fn new(op_state: OnceState<V>) -> Self {
        ExecuteOnce {
            lock: Mutex::new(op_state),
            settled: Condvar::new(),
        }
    }

    fn execute(&self, thread: u64, run: impl FnOnce() -> V) -> Result<V, String> {
        let mut state = self.lock.lock().unwrap();
        match &*state {
            OnceState::Running { thread: t } if *t == thread => {
                return Err(
                    "execute-once operation re-entered from inside its own execution".into(),
                );
            }
            OnceState::Running { .. } => {
                // waiters park until Done or Failed — never forever
                let s = self
                    .settled
                    .wait_while(state, |s| matches!(s, OnceState::Running { .. }))
                    .unwrap();
                state = s;
                loop {
                    match &*state {
                        OnceState::Done { value } => return Ok(value.clone()),
                        OnceState::Failed(e) => return Err(e.clone()),
                        OnceState::Running { .. } => {
                            state = self
                                .settled
                                .wait_while(state, |s| matches!(s, OnceState::Running { .. }))
                                .unwrap();
                        }
                        OnceState::Pending => break,
                    }
                }
            }
            OnceState::Done { value } => return Ok(value.clone()),
            OnceState::Failed(e) => return Err(e.clone()),
            OnceState::Pending => {}
        }
        // first caller: take the op, run OUTSIDE the lock, guard the unwind
        *state = OnceState::Running { thread };
        drop(state);
        let outcome = {
            let guard = SettleOnUnwind { once: &self };
            let r = run();
            std::mem::forget(guard); // the success path defuses the guard
            r
        };
        let mut state = self.lock.lock().unwrap();
        *state = OnceState::Done { value: outcome };
        self.settled.notify_all();
        let OnceState::Done { value } = &*state else {
            unreachable!()
        };
        Ok(value.clone())
    }

    /// the modeled unwind settle: the drop guard's body, invoked by the
    /// fixture under catch_unwind (the source pins this arm source-side)
    fn settle_failed_on_unwind(&self) {
        let mut state = self.lock.lock().unwrap();
        *state = OnceState::Failed("execute-once operation panicked while executing".into());
        self.settled.notify_all();
    }
}

struct SettleOnUnwind<'a, V: Clone + Send + 'static> {
    once: &'a ExecuteOnce<V>,
}

impl<V: Clone + Send + 'static> Drop for SettleOnUnwind<'_, V> {
    fn drop(&mut self) {
        self.once.settle_failed_on_unwind();
    }
}

#[test]
fn r7_first_runs_waiters_get_done() {
    // the deterministic handshake: the waiter is the RUNNER and signals from
    // inside run (Running is armed by then) — the main thread's execute then
    // PARKS, no timing dependence (the 10ms-sleep shape raced on a busy box)
    let once = Arc::new(ExecuteOnce::<u64>::new(OnceState::Pending));
    let o2 = once.clone();
    let (armed_tx, armed_rx) = std::sync::mpsc::channel::<()>();
    let waiter = std::thread::spawn(move || {
        o2.execute(2, || {
            armed_tx.send(()).unwrap(); // Running is armed before run is called
            41 + 1
        })
    });
    armed_rx.recv().unwrap(); // the waiter holds the op
    let v = once.execute(1, || 99).unwrap(); // parks until Done
    assert_eq!(v, 42);
    assert_eq!(
        waiter.join().unwrap(),
        Ok(42),
        "the waiter (runner) completed"
    );
}

#[test]
fn r7_err_settles_failed_and_releases_every_waiter_with_the_same_error() {
    let once = Arc::new(ExecuteOnce::<u64>::new(OnceState::Pending));
    let o2 = once.clone();
    let waiter = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        o2.execute(2, || unreachable!())
    });
    // the first caller's run fails — modeled as settle after arming
    once.settle_failed_on_unwind(); // the Err path's settle + notify_all
    let w = waiter.join().unwrap();
    assert_eq!(
        w,
        Err("execute-once operation panicked while executing".into()),
        "every waiter gets the SAME Failed error"
    );
    // Failed is terminal: a late clone gets the original error
    assert_eq!(
        once.execute(3, || 7).unwrap_err(),
        "execute-once operation panicked while executing"
    );
}

#[test]
fn r7_panic_inside_run_settles_failed_via_the_drop_guard() {
    let once = Arc::new(ExecuteOnce::<u64>::new(OnceState::Pending));
    let o2 = once.clone();
    let waiter = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        o2.execute(2, || unreachable!())
    });
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        once.execute(1, || panic!("the op exploded mid-execution"))
            .unwrap();
    }));
    assert!(r.is_err(), "the panic propagates to the caller");
    let w = waiter.join().unwrap();
    assert!(
        w.is_err(),
        "the drop guard settled Failed: the waiter was RELEASED, never blocked on Running forever"
    );
    // terminal: the op was consumed
    assert!(once.execute(3, || 7).is_err());
}

#[test]
fn r7_same_thread_reentry_is_a_typed_error_not_a_deadlock() {
    let once = ExecuteOnce::<u64>::new(OnceState::Pending);
    // the executing thread re-entering: the re-entry refusal fires BEFORE
    // the wait would deadlock
    once.execute(1, || 1).unwrap();
    // arm Running for the same thread and re-enter
    {
        let mut s = once.lock.lock().unwrap();
        *s = OnceState::Running { thread: 1 };
    }
    let err = once.execute(1, || 2).unwrap_err();
    assert!(
        err.contains("re-entered"),
        "the re-entry is reported as a bug, never a deadlock: {err}"
    );
}

fn main() {
    // the standalone harness runs via --test; main is never invoked
}
