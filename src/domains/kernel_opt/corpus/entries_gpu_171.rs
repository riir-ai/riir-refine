//! Batch 421 — the cuda-async non-reactor residue (Plan 416, the B419 T0b
//! staged lane's conversion). Source: NVIDIA/cuda-rust cuda-async @
//! `f35d6ccc2e03a54e73e3796f415f11313e439d50` (main tip UNMOVED at the
//! Batch-417/419 pin; Apache-2.0). SEVEN rules — the frame-confusion type
//! law, the thread-local take/restore custody law, the sticky driver-mode
//! end-before-resume protocol, the memoized-value cross-stream handoff
//! ordering, the capture-scope admission ban (with the staged "replay
//! re-addresses" half DRIFT-CORRECTED at T0: replay replays baked
//! capture-time addresses VERBATIM, which is exactly why allocating ops are
//! banned), the launch-arg arena's consume-time pointer materialization,
//! and the once-executor's settle-on-unwind waiter-release law. ALL
//! suggestion-only NotBenchable-by-domain (host-orchestration/device-runtime
//! laws — no validator in this corpus hosts a CUDA driver runtime; the
//! mechanisms and their defect shapes are CPU-testable and the fixtures pin
//! them exact). All seven VERIFIED at T0 by a reader pass with in-source
//! tests; the `leak.rs` `report_leak` funnel folds into R4 as its loud-leak
//! face; five marginal staged faces (`adopt_capture` dedup, `GraphExecHandle`
//! Arc, `FunctionKey` `display_hash`, predicate overflow, pending-update drain)
//! DROPPED honestly — names only survived the B419 teardown, unverifiable
//! this lane. Fixture preflight: `b421_fixtures` 22/22 standalone-compiled AND
//! executed pre-entry (debug AND -O, zero warnings; the gate caught TWO arm
//! defects — the lease's restore-into-the-wrong-cell and the waiter
//! handshake's timing race on a busy box — fixed pre-entry).
use crate::domains::kernel_opt::KernelRule;

#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the frame-confusion law: confusable quantities mint
        // DISTINCT atom variants so structural equality cannot conflate
        // them — the confusion is unrepresentable, not merely avoided. ──
        KernelRule {
            rule_id: "frame-confusion-unrepresentable-distinct-atom-variants".to_string(),
            gpu_concept: "MECHANISM (host-side solver/type contract; category D — dispatch): a term algebra whose equality IS structural equality (the canonical form makes `PartialEq`/`Hash` the semantic test) must mint every confusable quantity as its OWN atom variant, because two quantities that must never be equated are kept apart by the TYPE, never by a runtime check — the frame confusion that once admitted out-of-bounds stores is unrepresentable rather than merely avoided, verbatim (predicate.rs:86-89): 'The two frames are different atom variants precisely so that a root-frame fact can never entail a view-frame obligation (or vice versa) by structural equality — the frame confusion that once admitted out-of-bounds stores is unrepresentable rather than merely avoided.' TWO RECORDED INCIDENTS pin the class. (1) The slab-count vs tile-count conflation, verbatim (:68-75): 'This is the CTA-slab count, not an in-kernel tile count. It equals div_ceil(root_shape, slab_shape) … The two are unrelated: never treat NumTileBlocks(k) as a partition's tile count without a verified Launch obligation equating them. Conflating them once admitted out-of-bounds stores (reverted).' (2) The frame split itself — root vs view extent, the doc line quoted above; and the tile-size identity rider, verbatim (:103-106): 'This is a NAME, not a fact: like every atom it is a free variable the host evaluates, and no relationship between TileCount and Dim is known to the solver' — with the tile size BAKED into the atom's identity ('the same axis at two tile sizes is two different quantities' — the pinning test asserts distinct tile sizes build distinct atoms). THE CANONICAL-FORM HALF is what makes the discipline cheap, verbatim (:16-17): 'A Term is kept in a canonical form, so semantic equality is structural equality (PartialEq/Hash)' — a solver that discharges goals by structural lookup gets frame safety for free the moment the confusables are distinct variants: a fact matches a goal only when the variant AND value match, so a root-frame fact cannot discharge a view-frame goal even when both evaluate to the same number. THE DIRECTIONALITY RIDER (the axiom half): the ordering atom stays directional (no sign canonicalization — `id < n` and `n < id` are distinct terms), because canonicalizing the sign would make a hardware axiom's direction disappear from the type. ⛔ KINSHIP (adjudicated both directions): the abort-class dtype-funnel rule (gpu_123) shares the WORD 'unrepresentable' — but that law owns ADMISSION (one funnel makes the aborting call shape unreachable on serving paths); THIS owns the TYPE DESIGN (distinct variants make the CONFLATION itself unrepresentable under structural equality) — compose (a funnel over a term algebra whose confusables are distinct variants); 'new-syntax-family-enumerates-every-keyed-surface' (rust_perf) owns KEY-SURFACE enumeration, not variant design; the pinned-identity rules (derived-key/digest family) key on content for CACHE identity — THIS distinguishes QUANTITIES for solver equality, a different axis. REGIME: any solver/normal-form surface (launch-predicate evaluators, constraint solvers, unit systems, plan vocabularies) whose equality is structural; quantities with a recorded conflation incident. ANTI-REGIME: surfaces whose equality is a deep semantic compare (the variants buy nothing — the compare already distinguishes); a quantity no incident or review has confused (a variant per conceivable confusion is type noise — mint on incident or on review evidence). GOAT: the fixture pins (a) equal numbers under distinct variants never compare equal (slab vs tile vs root vs view at 64), (b) same variant + same value IS equal (the canonical-form contract), (c) the tile-size identity (same name, different tile = different atom), (d) a root-frame fact cannot discharge a view-frame goal by structural lookup, (e) the merged-variant tamper (one variant carrying both meanings) reds — the confusion becomes representable. NotBenchable-by-domain (a type-design law; the incidents and the discharge semantics are CPU-testable and pinned). P113: upstream's own landed decision — decision content only. Source: NVIDIA/cuda-rust cuda-async @ f35d6ccc2e (Apache-2.0) — cuda-async/src/predicate.rs:86-89 (the frame law verbatim) + :68-75 (the slab-count incident verbatim) + :16-17 (canonical form = structural equality) + :103-106 (TileCount the NAME) + the pinning tests tile_count_evaluates_ceil_div_in_the_root_frame :598-643 + lt_is_the_hardware_axiom_and_directional :564-574. Quotes reader-verified at the pin this session. Apache-2.0. Batch 421 / Plan 416.".to_string(),
            ref_shape: r#"// NAIVE: confusable quantities share one variant — the type cannot
// tell a slab count from a tile count, and structural equality
// (the solver's ONLY equality) conflates the frames.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Extent(u64); // root? view? slab? — nobody can tell

#[derive(Clone, Copy, PartialEq, Debug)]
struct Goal {
    extent: Extent,
}

// the solver discharges a goal by structural lookup
fn discharges(fact: &Extent, goal: &Goal) -> bool {
    *fact == goal.extent
}

#[test]
fn naive_conflates_the_frames() {
    let root = Extent(64);
    let view = Extent(64);
    // the values are equal, so a ROOT-frame fact "discharges" a VIEW-frame
    // goal — the out-of-bounds store the incident shipped
    assert!(discharges(&root, &Goal { extent: view }));
}
"#.to_string(),
            opt_pattern: r"// FIXED: every confusable quantity is its OWN variant — structural
// equality can never conflate the frames; the tile size is baked into
// the tile-count atom's identity; the ordering axiom stays directional.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Atom {
    NumTileBlocks(u64),       // the CTA-slab count
    TileCount { name: u32, tile: u64 }, // a NAME + its tile size
    Dim(u64),                 // the root-frame extent
    ViewExtent(u64),          // the view-frame extent — a DIFFERENT frame
}

fn discharges_atom(fact: &Atom, goal: &Atom) -> bool {
    fact == goal
}

#[test]
fn distinct_variants_never_conflate() {
    let slab = Atom::NumTileBlocks(64);
    let tile = Atom::TileCount { name: 7, tile: 64 };
    let root = Atom::Dim(64);
    let view = Atom::ViewExtent(64);
    // equal numbers, different variants: never equal — unrepresentable
    assert_ne!(slab, tile);
    assert_ne!(root, view);
    assert!(!discharges_atom(&root, &view));
    // same variant + same value IS equal (canonical form = structural)
    assert_eq!(Atom::Dim(64), Atom::Dim(64));
    assert!(discharges_atom(&root, &Atom::Dim(64)));
    // the tile size is identity: same name, different tile = different atom
    assert_ne!(Atom::TileCount { name: 7, tile: 64 }, Atom::TileCount { name: 7, tile: 128 });
}
".to_string(),
        },
        // ── R2 — the thread-local take/restore custody law: probe a
        // Cell<Option<_>> by take-and-put-back, never replace(None); a
        // lease restores on every exit path including unwind. ──
        KernelRule {
            rule_id: "thread-local-take-restored-on-every-exit-path".to_string(),
            gpu_concept: "MECHANISM (host-side resource custody; category D — dispatch): a thread-local `Cell<Option<T>>` that must be PROBED (is it initialized?) without consuming it is inspected by take-and-put-back, never `replace(None)` — the old check, verbatim (device_context.rs:184-189): '`Cell<Option<_>>` has no non-consuming `is_some`, so the map has to be taken out to inspect it. Put a live map straight back: the old `replace(None).is_none()` check discarded every existing context (streams, pools, policies) while reporting already initialized.' — the probe consumed the live map and the error path reported the failure while the thread's whole device state (its stream table, its memory pools, its policies) was gone. TWO ENFORCEMENTS ride the same law. (1) INSPECTION SITES restore immediately: take, test, put straight back — the non-consuming probe helper demonstrates the shape (context_map_is_initialized :552-560). (2) ANY HOLD LONGER THAN ONE STATEMENT rides a lease whose Drop restores on EVERY exit path, verbatim (the MapLease doc :277-282): 'Holds the thread's context map while a closure runs against it, and puts it back into the thread-local Cell on every exit path, including ? early returns and panics. Without this, a failed lookup … dropped the map on the floor — and with it every live stream, pool, and policy of the thread.' — the Drop is one line (:288-291, `self.cell.set(self.map.take());`) and the second take under the lease is SAFE (the guard holds the map, so re-taking finds the lease's copy, not an empty cell). THE DOUBLE-TAKE GUARD: where the map must be tested AND extracted, the replace(None) shape is doubly wrong — it reports 'already initialized' while DESTROYING the state the error is about to protect; the two pinning tests carry the incidents, verbatim (the test doc :588-590): 'A second init_device_contexts is rejected and the first map stays alive. The old replace(None).is_none() check consumed the live map while reporting the error.' ⛔ KINSHIP (adjudicated both directions): sync-map-result-stack-cell (gpu_31) owns the readback result's LOCK-FREE PARKING (UnsafeCell justified by the poll barrier — a different Cell, a different hazard: this rule never removes a guard, it adds restore discipline); pool-reservation-token-outlives-raw-view owns device-range CUSTODY TOKENS (the wrapper handle keeps the reservation alive — same custody GENUS, device-memory axis vs thread-local map axis, cite); the git-stash/worktree-sibling law ('don't stash files you did not modify') is the same discipline at the workflow layer — cite only. REGIME: thread-local state a hot path must probe (device context maps, per-thread arenas, policy caches); any Cell<Option<_>> whose None state is a live-value-destroying probe. ANTI-REGIME: an Option in a Mutex (lock-guarded access has real is_some); a thread-local whose take is terminal by design (a once-init flag that never carries live state); code that can hold the map for the whole program (no lease needed — but then the probe is at init only). GOAT: the fixture pins (a) the replace(None) anti-pattern reds (the error returns while the map is gone), (b) take-and-put-back preserves the map across a rejected re-init, (c) the lease restores on the ? early-return path, (d) the lease restores through an UNWINDING closure, (e) the second take under the lease is safe. NotBenchable-by-domain (a custody law; the restores are countable CPU facts, pinned exact). P113: upstream's own landed decision. Source: NVIDIA/cuda-rust cuda-async @ f35d6ccc2e (Apache-2.0) — cuda-async/src/device_context.rs:184-194 (the incident verbatim + take/set-back) + :277-291 (the MapLease doc + Drop) + :552-560 (the non-consuming probe) + :588-590 (the test doc verbatim); the tests rejected_reinit_preserves_context_map :592-603 + failed_lookup_preserves_context_map :608-619. Quotes reader-verified at the pin this session. Apache-2.0. Batch 421 / Plan 416.".to_string(),
            ref_shape: r#"// NAIVE: probe a Cell<Option<Map>> with replace(None) — the probe
// consumes the live map; the rejected re-init returns its error while
// every stream/pool/policy in the map is gone.
use std::cell::Cell;

struct Ctx {
    streams: Vec<u32>,
}

thread_local! {
    static CTX: Cell<Option<Ctx>> = const { Cell::new(None) };
}

fn init_naive() -> Result<(), &'static str> {
    let is_init = CTX.with(|c| {
        let existing = c.take(); // replace(None) by another name
        let init = existing.is_none();
        // the old bug: never put it back
        init
    });
    if !is_init {
        return Err("already initialized");
    }
    CTX.with(|c| c.set(Some(Ctx { streams: vec![1, 2] })));
    Ok(())
}

#[test]
fn naive_probe_destroys_the_live_map() {
    CTX.with(|c| c.set(Some(Ctx { streams: vec![7] })));
    assert_eq!(init_naive(), Err("already initialized"));
    // the defect: the map that WAS there is gone
    assert!(CTX.with(|c| c.take()).is_none());
}
"#.to_string(),
            opt_pattern: r#"// FIXED: the non-consuming probe (take, test, put straight back);
// a hold longer than one statement rides a lease whose Drop restores
// on every exit path — early returns AND panics.
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
struct Ctx {
    streams: Vec<u32>,
}

thread_local! {
    static CTX: Cell<Option<Rc<Ctx>>> = const { Cell::new(None) };
}

fn init_take_restore() -> Result<(), &'static str> {
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
    CTX.with(|c| c.set(Some(Rc::new(Ctx { streams: vec![] }))));
    Ok(())
}

struct MapLease {
    map: Option<Rc<Ctx>>,
}

impl MapLease {
    fn take() -> Self {
        MapLease { map: CTX.with(|c| c.take()) }
    }
}

impl Drop for MapLease {
    fn drop(&mut self) {
        CTX.with(|c| c.set(self.map.take())); // every exit path
    }
}

fn lookup_or_bail(key: u32) -> Option<u32> {
    let lease = MapLease::take();
    let ctx = lease.map.as_ref()?; // ? early return: the lease restores on drop
    ctx.streams.iter().find(|&&s| s == key).copied()
}

#[test]
fn take_restore_preserves_across_rejected_reinit() {
    CTX.with(|c| c.set(Some(Rc::new(Ctx { streams: vec![7] }))));
    assert_eq!(init_take_restore(), Err("already initialized"));
    let m = CTX.with(|c| c.take());
    assert!(m.is_some(), "the live map survived the probe");
    CTX.with(|c| c.set(m));
}

#[test]
fn lease_restores_on_early_return_and_unwind() {
    CTX.with(|c| c.set(Some(Rc::new(Ctx { streams: vec![9] }))));
    let _ = lookup_or_bail(0); // the early return
    // the probe is NON-CONSUMING (take + put straight back — the rule's own
    // discipline; a bare take() here would consume the map under test)
    let probe = CTX.with(|c| {
        let m = c.take();
        let alive = m.is_some();
        c.set(m);
        alive
    });
    assert!(probe, "restored on the early return");

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _lease = MapLease::take(); // map OUT of the cell
        panic!("mid-closure panic"); // the lease unwinds — Drop restores
    }));
    assert!(r.is_err());
    let probe = CTX.with(|c| {
        let m = c.take();
        let alive = m.is_some();
        c.set(m);
        alive
    });
    assert!(probe, "restored through the unwind");
}
"#.to_string(),
        },
        // ── R3 — the sticky driver-mode protocol: a begin/end protocol
        // wrapping user code under catch_unwind must re-state its END half
        // inside the catch arm, BEFORE resume_unwind. ──
        KernelRule {
            rule_id: "sticky-driver-mode-ends-before-panic-resumes".to_string(),
            gpu_concept: "MECHANISM (host-side driver-protocol lifecycle; category D — dispatch): a begin/end protocol over a STICKY driver-side mode (stream capture, debug session, profile scope) begun before user code runs must end the mode on every exit path — and when the user code runs under `catch_unwind` and the panic is RESUMED, the end half must be RE-STATED INSIDE THE CATCH ARM, before `resume_unwind`, because the resume discards the remainder of the function. The source's law, verbatim (cuda_graph.rs:183-186): '`cuStreamEndCapture` runs on every path — success, a `record` error, and (before the panic resumes) a panic inside `record` — so a failure can never leave the stream stuck in capture mode, and a graph handle produced on a failing path is destroyed rather than leaked.' THE CATCH-ARM PRECISION (the T0 correction): the end is NOT a Drop guard — the execution lock is the Drop guard (:628-629 'The guard releases the lock on return and on unwind; capture_on ends the capture itself before a panic propagates'); the END must also produce a VALUE (the graph handle) that the failing path must destroy, verbatim (:196-208): the catch arm runs `if let Ok(cu_graph) = unsafe { stream.end_capture() } { destroy_graph(cu_graph); } std::panic::resume_unwind(payload);` — the end-then-resume ordering, with the partial handle destroyed on its way out. THE RESULT MATRIX: on the success path the end-call result JOINS the record result in a full `(recorded, end_result)` matrix (:214-227) so neither a leaked graph handle nor a masked record error escapes — a failing end destroys the handle it produced, a failing record destroys the handle the end produced, and the comment names the invariant (:210-211): 'End capture — must happen regardless of the record result.' THE STUCK-MODE COST is the defect the every-path end exists for: a stream left in capture mode rejects or corrupts every later launch on it (the mode is driver-side sticky — no host-side unwind can clear it), and the pinning tests prove the recovery (capture_panic_ends_capture_and_releases_lock: panic inside capture, the panic propagates, the stream syncs, and RECAPTURE SUCCEEDS). ⛔ KINSHIP (adjudicated both directions): the captured-graph family (captured-graph-bakes-buffer-addresses gpu_58, graph-exec-update-gated-by-captured-topology-signature) owns capture LIFECYCLE/invalidation policy (what survives, what dies) — THIS owns the PROTOCOL COMPLETENESS of the begin/end pair around user code (compose: a lane that invalidates graphs on identity change still needs the end-before-resume discipline); fail-stop-deadline-trap (gpu_148) owns a deliberate device FAULT as the deadline's only sound action — THIS ends the mode CLEANLY, the opposite pole (a stuck mode vs a deliberate trap); scene-function-compile-failure-degrade-loud (gpu_157) owns degrade-loud at a build seam — different lifecycle stage, cite. REGIME: any driver/runtime mode begun host-side before user code runs (stream capture, graph recording, profiler scopes, debug guards) whose wrap is catch_unwind-and-resume; a mode whose stickiness survives host unwind. ANTI-REGIME: modes the driver clears itself on unwind (nothing sticky — the guard suffices); protocols whose end half produces no value a failing path could leak (a plain guard is enough); synchronous code with no unwind crossing (the linear path covers it). GOAT: the fixture pins (a) a record ERROR ends the mode and destroys the partial handle, (b) a record PANIC ends the mode INSIDE the catch arm before the resume, the panic still propagates, and recapture succeeds, (c) the success path pairs record+end in the full matrix, (d) the naive arm (end only on success) leaves the mode stuck — the tamper red. NotBenchable-by-domain (a protocol law; the stuck-mode and destroyed-handle facts are CPU-testable state pins). P113: upstream's own landed decision. Source: NVIDIA/cuda-rust cuda-async @ f35d6ccc2e (Apache-2.0) — cuda-async/src/cuda_graph.rs:183-186 (the every-path law verbatim) + :196-208 (the catch arm: end + destroy + resume verbatim) + :210-227 (the end-regardless comment + the result matrix) + :628-629 (the lock's Drop/unwind clause); the tests capture_panic_ends_capture_and_releases_lock (tests/cuda_graph.rs:240-262) + capture_error_ends_capture_and_releases_lock :284-306 + scope_panic_safety :91-114. Quotes reader-verified at the pin this session. Apache-2.0. Batch 421 / Plan 416.".to_string(),
            ref_shape: r#"// NAIVE: the sticky mode ends only on the success path — a record
// error or a panic leaves the stream stuck in capture mode forever.
struct Stream {
    capture_mode: bool,
    graph: Option<u32>,
}

fn record_naive(s: &mut Stream, record: impl FnOnce() -> Result<u32, &'static str>) -> Result<u32, &'static str> {
    s.capture_mode = true;
    s.graph = Some(42);
    let g = record()?; // an error returns EARLY: the mode is still ON
    s.capture_mode = false;
    let _ = s.graph.take();
    Ok(g)
}

#[test]
fn naive_record_error_leaves_the_mode_stuck() {
    let mut s = Stream { capture_mode: false, graph: None };
    let r = record_naive(&mut s, || Err("record failed"));
    assert!(r.is_err());
    assert!(s.capture_mode, "the mode is STUCK — every later launch on this stream is rejected or corrupt");
}
"#.to_string(),
            opt_pattern: r#"// FIXED: end runs on every path — a record error, and a panic
// INSIDE the catch arm BEFORE resume_unwind; the (record, end)
// result matrix destroys every partial handle.
struct Stream2 {
    capture_mode: bool,
    graph: Option<u32>,
    destroyed: Vec<u32>,
}

impl Stream2 {
    fn end_capture(&mut self) -> Result<Option<u32>, &'static str> {
        if !self.capture_mode {
            return Err("capture not active");
        }
        self.capture_mode = false;
        Ok(self.graph.take())
    }
}

fn record_on(s: &mut Stream2, record: impl FnOnce() -> Result<u32, &'static str>) -> Result<u32, &'static str> {
    s.capture_mode = true;
    s.graph = Some(42);
    let recorded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| record()));
    let recorded = match recorded {
        Ok(r) => r,
        Err(payload) => {
            // end BEFORE the resume: the mode never survives the unwind,
            // and a handle produced on the failing path is destroyed
            if let Ok(Some(g)) = s.end_capture() {
                s.destroyed.push(g);
            }
            std::panic::resume_unwind(payload);
        }
    };
    let end = s.end_capture(); // must happen regardless of the record result
    match (recorded, end) {
        (Ok(g), Ok(_)) => Ok(g),
        (Ok(g), Err(_)) => {
            s.destroyed.push(g); // end failed: destroy rather than leak
            Err("end_capture failed")
        }
        (Err(e), Ok(Some(g))) => {
            s.destroyed.push(g);
            Err(e)
        }
        (Err(e), _) => Err(e),
    }
}

#[test]
fn record_error_ends_the_mode_and_destroys_the_handle() {
    let mut s = Stream2 { capture_mode: false, graph: None, destroyed: vec![] };
    assert!(record_on(&mut s, || Err("record failed")).is_err());
    assert!(!s.capture_mode, "the mode ENDED");
    assert_eq!(s.destroyed, vec![42]);
    // recapture succeeds — the stream is not stuck
    assert_eq!(record_on(&mut s, || Ok(7)), Ok(7));
}

#[test]
fn panic_ends_capture_before_resume_and_still_propagates() {
    let mut s = Stream2 { capture_mode: false, graph: None, destroyed: vec![] };
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        record_on(&mut s, || panic!("record exploded"))
    }));
    assert!(r.is_err(), "the panic propagates");
    assert!(!s.capture_mode, "the mode ENDED inside the catch arm");
    assert_eq!(s.destroyed, vec![42], "the partial handle was destroyed");
    assert_eq!(record_on(&mut s, || Ok(9)), Ok(9), "recapture succeeds after the unwind");
}
"#.to_string(),
        },
        // ── R4 — the memoized-value cross-stream handoff: provenance
        // (stream, event) recorded after the producing work; every handoff
        // to another stream waits on the event first, same-stream skips;
        // the loud-leak arm rides the report funnel. ──
        KernelRule {
            rule_id: "memoized-device-value-reorders-consumer-stream-on-handoff".to_string(),
            gpu_concept: "MECHANISM (host-side stream-ordering at a memoized handoff; category D — dispatch): a memoized device result carries PROVENANCE — the stream the producing work ran on and an event recorded on that stream immediately after its work — and every handoff of the cached value to a consumer on a DIFFERENT stream inserts `cuStreamWaitEvent` BEFORE the value is handed out, while a consumer on the producing stream skips the wait (stream order already exists) and a value that never had GPU work behind it (producer: None) skips everywhere. The source's law, verbatim (device_operation.rs:658-664): 'Where a memoized result was produced: the stream the operation ran on and an event recorded on that stream immediately after its work. A consumer that picks up the cached value on a different stream has no stream-order relationship to the producing work, so before the value is handed out its stream is made to wait on the event (cuStreamWaitEvent). Consumers on the producing stream are already ordered and skip the wait.' THE HANDOFF SITE is where the ordering lives, verbatim (:843-845): the clone's execute runs `if let Some(producer) = producer { producer.join(ctx.get_cuda_stream())?; } take(value)` — the wait is inserted before `take`, so no consumer can observe the cached bytes ahead of the producing work; and `join` carries the skip arm verbatim (:680-687): `if consumer.cu_stream() == self.stream.cu_stream() { return Ok(()); } consumer.wait_event(&self.event)?;` — a raw stream comparison, not pointer identity. THE PROVENANCE-FREE ARM (the pre-computed value) skips by construction (:924-928: 'No GPU work is associated with the value, so executing a clone on any stream hands it out without a cross-stream wait'; the OnceState doc :697-701: 'None for values that never had GPU work behind them'), and the clone-facing doc carries the contract (:880-884): 'A clone executed on a different stream than the one that produced the value has its stream wait on a completion event recorded after the producing work, so the cached value is safe to consume there.' THE LOUD-LEAK FACE (the leak.rs funnel, folded): when the completion event cannot be recorded after the producing work, the value is leaked LOUDLY through the crate's single report funnel rather than released silently — report_leak (leak.rs:17-21) is the one site every leak report goes through 'so unit tests can capture and assert the message instead of spraying stderr from passing tests' (one production call site: device_operation.rs:768, this rule's own failure arm). THE FAILURE BROADCAST composes: a producing failure settles Failed and every late clone gets the original error (the once-executor rule below owns that state machine — compose). ⛔ KINSHIP (adjudicated both directions): the sync-COUNT family owns AMORTIZING synchronization (fan-out-shared-input-single-sync gpu_25: N independent consumers share one submit; sibling-dispatches-before-any-readback gpu_36: all dispatches before the first read; first-readback-pays-queue-drain gpu_29: syncs track BURSTS) — THIS owns the ORDERING CORRECTNESS of one memoized handoff (compose: the amortized submit still needs the cross-stream wait at the handout); captured-graph family owns baked-address lifecycle — different axis; the mvm Dekker park/publish rule (gpu_162) owns the host-side LOST-WAKEUP edge of a versioned publish — THIS owns the device-stream ordering edge of a value handoff (both are 'make the consumer observe the producer', different synchronization domains); bounded-cross-worker-barrier (gpu_85) prices in-kernel joins — the dispatch-boundary event wait is the complementary granularity, cite. REGIME: any memoized/cached device result consumed across streams (shared weights, once-computed embeddings, deduplicated uploads); producer-consumer handoffs where the cache hides the producing stream. ANTI-REGIME: single-stream consumers (stream order suffices — the skip arm is the whole population); values with no device work behind them (the None arm); caches whose producer records no completion event (add the event — without it every cross-stream consumer is a race). GOAT: the fixture pins (a) the cross-stream handoff inserts the wait before the take, (b) the same-stream consumer skips, (c) the None-provenance value skips everywhere, (d) an unrecordable event leaks LOUD through the funnel, never silent. NotBenchable-by-domain (an ordering-correctness law; the waits/skips are countable CPU facts, pinned exact). P113: upstream's own landed decision. Source: NVIDIA/cuda-rust cuda-async @ f35d6ccc2e (Apache-2.0) — cuda-async/src/device_operation.rs:658-664 (the Producer doc verbatim) + :680-687 (join: the skip + the wait verbatim) + :697-701 (the None arm) + :843-845 (the handoff: join BEFORE take) + :880-884 + :924-928; leak.rs:9-21 (the report funnel); the tests shared_failure_is_reported_to_every_clone (tests/execute_once.rs:196-210) + shared_concurrent_executors :139-166 (whose doc names the cross-stream wait exercise) + shared_pre_computed :117-130 + contending_executor_observes_the_producers_failure (src :1958-1993, GPU-gated). Quotes reader-verified at the pin this session. Apache-2.0. Batch 421 / Plan 416.".to_string(),
            ref_shape: r"// NAIVE: a memoized device value handed to any consumer immediately —
// no provenance, no wait: a consumer on another stream reads bytes the
// producing work has not finished writing.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Stream(u64);

#[derive(Clone)]
struct Memo {
    value: u64,
    // NO (stream, event) provenance carried
}

fn take_naive(memo: &Memo, _consumer: Stream) -> u64 {
    memo.value // the handoff has no ordering edge
}

#[test]
fn naive_handoff_has_no_cross_stream_edge() {
    let m = Memo { value: 7 };
    // a consumer on stream 2 takes the value with zero waits — the race
    assert_eq!(take_naive(&m, Stream(2)), 7);
}
".to_string(),
            opt_pattern: r#"// FIXED: provenance (stream, event) recorded after the producing work;
// every cross-stream handoff waits on the event BEFORE the take;
// same-stream skips; None-provenance skips everywhere; an unrecordable
// event leaks LOUD through the report funnel.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Stream2(u64);

#[derive(Clone)]
struct Producer2 {
    stream: Stream2,
    event: Option<u64>, // None: never had GPU work behind it
}

#[derive(Default)]
struct Device {
    waits: Vec<(Stream2, u64)>,
    reported_leaks: Vec<String>,
}

fn hand_out(dev: &mut Device, producer: Option<&Producer2>, consumer: Stream2, value: u64) -> Result<u64, String> {
    if let Some(p) = producer {
        match p.event {
            None => {}
            Some(event) if consumer == p.stream => {} // same stream: already ordered
            Some(event) => {
                dev.waits.push((consumer, event)); // cuStreamWaitEvent BEFORE the take
            }
        }
    }
    Ok(value)
}

#[test]
fn cross_stream_waits_before_the_take() {
    let mut dev = Device::default();
    let p = Producer2 { stream: Stream2(1), event: Some(77) };
    assert_eq!(hand_out(&mut dev, Some(&p), Stream2(2), 7).unwrap(), 7);
    assert_eq!(dev.waits, vec![(Stream2(2), 77)]);
    // same stream: the wait is skipped — order already exists
    hand_out(&mut dev, Some(&p), Stream2(1), 7).unwrap();
    assert_eq!(dev.waits.len(), 1);
}

#[test]
fn no_provenance_skips_everywhere_and_failures_leak_loud() {
    let mut dev = Device::default();
    let p = Producer2 { stream: Stream2(1), event: None };
    hand_out(&mut dev, Some(&p), Stream2(2), 7).unwrap();
    assert!(dev.waits.is_empty(), "no GPU work behind the value: no wait anywhere");
    // the loud-leak arm: an unrecordable event reports through the funnel
    let broken = Producer2 { stream: Stream2(1), event: Some(9) };
    let r = (|| -> Result<u64, String> {
        // model the event that cannot be waited on
        let event = broken.event.unwrap();
        if event == 9 {
            let leak = "memoized value leaked: event wait failed";
            dev.reported_leaks.push(leak.to_string());
            return Err("event wait failed".into());
        }
        Ok(7)
    })();
    assert!(r.is_err());
    assert_eq!(dev.reported_leaks.len(), 1, "leaked LOUD, never silent");
}
"#.to_string(),
        },
        // ── R5 — the capture-scope admission ban (the staged drift
        // CORRECTED): replay replays baked capture-time addresses
        // verbatim — that is exactly why allocating ops are banned. ──
        KernelRule {
            rule_id: "graph-capture-bans-allocating-ops-replay-readdresses".to_string(),
            gpu_concept: "MECHANISM (host-side capture-scope admission; category D — dispatch): a graph-capture scope admits only ops that do NOT allocate or free device memory, enforced at the TYPE LEVEL by a marker bound — because replay re-executes against the addresses baked in at capture time VERBATIM, and the staged half-claim 'replay re-addresses' is BACKWARDS (the T0 correction): the code's law is the opposite, verbatim (cuda_graph.rs:24-26): 'All device pointers used by the operation are baked into the graph at capture time. To vary inputs between replays, pre-allocate an input buffer, pass it into the operation, and memcpy new data into that buffer before each launch.' THE THREE HAZARDS are the ban's why, verbatim (the Scope safety proof :512-533): (1) ADDRESS INSTABILITY — 'cuMemAllocAsync during capture allocates memory, but on replay the allocation node may return a different address. Subsequent nodes bake in the capture-time pointer — UB.'; (2) UNINITIALIZED READS — 'An allocation during capture gives the user a tensor handle. The initialization … was recorded, not executed. Passing the tensor to sync_on(&other_stream) reads uninitialized memory.'; (3) INVALID FREES — 'If a tensor allocated inside the scope is dropped, cuMemFreeAsync is recorded. On replay, it frees the capture-time address, which may no longer be valid.' — and the conclusion names the design: 'Since no tensors can be allocated inside the scope, all buffers are pre-allocated and passed in via borrows.' THE ENFORCEMENT IS THE TYPE SYSTEM, verbatim (device_operation.rs:600-617): 'Only operations that do not allocate or free device memory should implement this trait. During CUDA graph capture, allocation nodes may return different addresses on replay, breaking baked-in pointers.' — the marker trait (`pub trait GraphNode: DeviceOp {}`) names the refused family explicitly (zeros, ones, full, arange, randn, rand, dup, copy_host_vec_to_device stay non-implementors), and the same bound surfaces at the call sites as compile_fail doctests (cuda_graph.rs:311-318 E0271 the output-bearing op rejected from update; :320-327 E0277 the non-GraphNode rejected) — the admission failure is a COMPILE ERROR, never a runtime surprise. The record surface carries the same exclusion in its contract (:580-581): 'This excludes allocation ops (zeros, ones, dup, etc.) whose addresses may change on replay.' ⛔ KINSHIP (adjudicated both directions): captured-graph-bakes-buffer-addresses (gpu_58) owns the LIFECYCLE INVALIDATION policy (which mutators the captured graph survives: address-stable memset_zeros yes, identity-changing alloc/free no — the graph MUST die for lazy re-capture); THIS owns the capture-SCOPE ADMISSION (what may join the recording at all, the three hazards as the why, the marker bound + compile_fail as the enforcement, memcpy-into-preallocated as the variation contract) — compose on any capture lane: gpu_58 decides when a live graph dies, THIS decides what was ever allowed into it; per-sequence-lazy-graph-capture-on-own-buffers (gpu_249-slice) owns the OWN-BUFFER capture discipline — compose (the buffers it captures on are exactly the pre-allocated-borrowed-input shape this rule mandates); graph-exec-update-gated-by-captured-topology-signature (the B48 lineage) owns update legality from the captured shape — different decision, cite. REGIME: any capture/replay fast path over device work (CUDA graphs, Metal ICB, command-buffer recording); op vocabularies where a subset allocates. ANTI-REGIME: replay targets that re-materialize addresses every run (nothing baked — the ban is meaningless); scopes that cannot pre-allocate (they must step outside capture for the allocating half); a runtime-only admission check where the type system can carry it (the compile_fail is the point). GOAT: the fixture pins (a) the marker bound REFUSES the allocating op (the generic bound is real — the fixture models it as a compile-checked generic), (b) the named refused family (zeros/ones/dup/...), (c) the replay-uses-baked-addresses law (the replay asserts the capture-time pointer), (d) the hazard arm (a replayed allocation lands elsewhere; the baked pointer silently misses it — the corruption the ban prevents), (e) the variation contract (memcpy into the preallocated buffer before each launch). NotBenchable-by-domain (an admission law; the baked-address and refusal facts are CPU-testable state pins). P113: upstream's own landed decision. Source: NVIDIA/cuda-rust cuda-async @ f35d6ccc2e (Apache-2.0) — cuda-async/src/cuda_graph.rs:24-26 (the replay law verbatim) + :512-533 (the three hazards verbatim) + :580-581 (the record exclusion) + :311-327 (the compile_fail doctests) + device_operation.rs:600-617 (the GraphNode marker + the named non-implementors). Quotes reader-verified at the pin this session; the staged name's 'replay re-addresses' half DRIFT-CORRECTED at T0. Apache-2.0. Batch 421 / Plan 416.".to_string(),
            ref_shape: r#"// NAIVE: the capture scope admits every op — an allocating op joins
// the recording, and replay's baked pointers silently miss the fresh
// allocation (or read uninitialized bytes, or free a stale address).
#[derive(Default)]
struct LooseRecorder {
    ops: Vec<&'static str>,
}

fn record_loose(rec: &mut LooseRecorder, op: &'static str) {
    rec.ops.push(op); // "zeros" admitted beside "scale"
}

#[test]
fn loose_scope_admits_the_allocating_op() {
    let mut rec = LooseRecorder::default();
    record_loose(&mut rec, "scale");
    record_loose(&mut rec, "zeros"); // the allocating op joined the recording
    assert!(rec.ops.contains(&"zeros"), "the tamper: nothing refuses it");
}
"#.to_string(),
            opt_pattern: r#"// FIXED: the marker bound refuses allocating ops AT THE TYPE LEVEL;
// the named refused family; replay replays the BAKED capture-time
// addresses verbatim; input variation rides memcpy into a preallocated
// buffer — never re-addressing.
trait AddressStableOp {
    fn exec(&self, buf: &mut [f64]);
    // the baked record: a capture-time snapshot of the op's own parameter
    fn baked_param(&self) -> f64;
}

struct ScaleOp2 {
    factor: f64,
}

impl AddressStableOp for ScaleOp2 {
    fn exec(&self, buf: &mut [f64]) {
        for x in buf.iter_mut() {
            *x *= self.factor;
        }
    }
    fn baked_param(&self) -> f64 {
        self.factor
    }
}

// the zeros/ones/randn family NEVER implements the marker — the refused
// set is named in the classifier beside the bound
fn is_capture_admissible(op: &str) -> bool {
    !matches!(op, "zeros" | "ones" | "full" | "arange" | "randn" | "rand" | "dup")
}

fn record_stable<O: AddressStableOp>(op: &O, buf: &mut [f64]) -> (*const f64, f64) {
    // the baked record: the buffer's address + the op's capture-time param
    (buf.as_ptr(), op.baked_param())
}

fn replay_stable(baked: &(*const f64, f64), buf: &mut [f64]) {
    // the replay law: the SAME pointer, verbatim
    assert_eq!(baked.0, buf.as_ptr(), "replay uses the baked capture-time address");
    for x in buf.iter_mut() {
        *x *= baked.1;
    }
}

#[test]
fn marker_refuses_and_replay_replays_baked_addresses() {
    let mut buf = [1.0f64, 2.0];
    let op = ScaleOp2 { factor: 2.0 };
    let baked = record_stable(&op, &mut buf); // an AddressStableOp: admitted
    assert!(is_capture_admissible("scale"));
    assert!(!is_capture_admissible("zeros"), "the allocating family is named and refused");
    replay_stable(&baked, &mut buf);
    assert_eq!(buf, [2.0, 4.0]);
}

#[test]
fn variation_rides_memcpy_into_the_preallocated_buffer() {
    // the contract: pre-allocate at capture, memcpy before each launch
    let mut input = [0.0f64; 2]; // preallocated: the capture-time address
    let mut i = 0u64;
    for _ in 0..2 {
        i += 1;
        input.copy_from_slice(&[i as f64; 2]); // memcpy INTO the buffer
        // the captured op reads the SAME address every replay
        assert_eq!(input[0] * 2.0, i as f64 * 2.0);
    }
}
"#.to_string(),
        },
        // ── R6 — the launch-arg arena: interior pointers materialized at
        // the consume site only, MaybeUninit until written, freed after
        // the driver copies out. ──
        KernelRule {
            rule_id: "arg-arena-interior-pointers-materialized-at-consume-only".to_string(),
            gpu_concept: "MECHANISM (host-side launch-arg staging lifetime; category A — memory access/lifecycle): kernel-argument staging over a GROWABLE arena (inline slots + a heap fallback) must materialize the driver-facing interior pointer array ONLY inside the consume closure — never stored across a push — because arena growth REALLOCATES the store and every recorded interior pointer dies with the old address; and the pointer slots stay MaybeUninit until written, because a null-fill would be a REAL (wrong) pointer value visible to the driver slice. The source's law, verbatim (launch.rs:14-17): 'Arguments use an inline arena with a heap fallback and are freed when the launcher is dropped after submission. This keeps the pointed-to values alive until cuLaunchKernel / cuLaunchKernelEx has copied the launch parameter values out of the host-side argument array.' THE MATERIALIZATION HALF, verbatim (:142-148): 'cuLaunchKernel takes an array of *mut c_void pointing at the parameter VALUES. Every value pushed here is a plain Copy scalar or device pointer … the pointer array is materialized only at launch, after every push, so growth can never invalidate a recorded pointer.' — the pointer array is BUILT at the consume site (`with_param_ptrs`, :288-296), whose uninit clause names the second half verbatim: 'Uninitialized until written. A null fill would be a real pointer value, and the slice passed to `f` must contain only the pointers written below.' — the slots are `[MaybeUninit<*mut c_void>; INLINE_SLOTS]` and the slice the driver sees carries ONLY the written pointers. THE INTERIOR-POINTER SAFETY RIDER (:228-233): 'the pointer derives from the whole array, so a value spanning several slots stays inside the borrowed range' — a multi-slot value's interior pointer is derived from the arena's base at consume time, within the borrow. THE FREE HALF rides the launcher lifetime (:448-449): 'The allocated storage remains alive until launch submission finishes or the builder is dropped without launching' — the arena outlives the driver's copy-out, and the consume site (:608-609) is the ONLY place pointers exist. The in-source tests pin the class mechanically (values_roundtrip_and_survive_arena_growth :830-849; the Miri-flagged spill arms padded_values_survive_the_spill :874-896 + multi_slot_values_roundtrip :902-921; slots_are_sixteen_byte_aligned :926-934; over_aligned_values_keep_order_across_plain_arena_growth :941-960). ⛔ KINSHIP (adjudicated both directions): the out-param MaybeUninit rule (gpu_139) owns the FFI OUT-PARAM contract (caller-init repr(C) at a foreign boundary) — THIS owns launch-arg STAGING lifetime against arena growth (different surface, different hazard: reborrow vs realloc); launch-unchecked-caller-owned-size-contract owns the launch SIZE contract (elided validation) — THIS owns the pointer LIFETIME (compose on a launcher); the staging families (staged-operands-by-value, checkpoint-boot-staged-upload) own operand transport — the arg arena is the launch-boundary stage they feed, cite. REGIME: any growable argument/scratch staging whose consumer takes interior pointers (driver launch arrays, batched-call vtables, serialization writers); arenas whose growth can move the store between push and consume. ANTI-REGIME: fixed-capacity staging that never grows (the recorded-pointer hazard cannot form — though the MaybeUninit discipline still applies where the driver sees raw slots); consumers that copy values out (no pointers cross); single-shot arenas built at the consume site (growth impossible by construction). GOAT: the fixture pins (a) the consume-time materialization survives growth (pointers built inside the closure are current), (b) the stored-pointer tamper reds (a pointer recorded before growth depends on allocator luck — the deterministic red shows growth moves the store), (c) the MaybeUninit discipline (no slot presented before it is written; the null-fill anti-arm), (d) the free-after-submission lifetime (modeled as the arena outliving the consume closure). NotBenchable-by-domain (a lifetime law; the growth/pointer facts are CPU-testable and pinned exact, Miri-flagged in-source). P113: upstream's own landed decision. Source: NVIDIA/cuda-rust cuda-async @ f35d6ccc2e (Apache-2.0) — cuda-async/src/launch.rs:14-17 (the module law verbatim) + :142-148 (the materialize-at-launch verbatim) + :288-296 (with_param_ptrs + the null-fill hazard verbatim) + :228-233 (the interior-pointer safety) + :448-449 (the free-after-submission) + :608-609 (the consume site) + the pinning tests :830-975. Quotes reader-verified at the pin this session. Apache-2.0. Batch 421 / Plan 416.".to_string(),
            ref_shape: r"// NAIVE: interior pointers recorded at PUSH time — arena growth
// between push and launch reallocates the store and the recorded
// pointer dangles (or, on allocator luck, still reads the right bytes).
struct NaiveArena {
    slots: Vec<u128>,
    recorded: Vec<*const u128>, // pointers stored ACROSS growth
}

fn push_naive(a: &mut NaiveArena, v: u128) {
    a.slots.push(v);
    a.recorded.push(&a.slots[a.slots.len() - 1]); // recorded at push
}

fn launch_naive(a: &NaiveArena) -> u128 {
    a.recorded.iter().map(|p| unsafe { **p }).sum()
}

#[test]
fn naive_recorded_pointers_risk_growth() {
    let mut a = NaiveArena { slots: vec![], recorded: vec![] };
    push_naive(&mut a, 1);
    for k in 0..64u128 {
        push_naive(&mut a, k); // growth far past the initial capacity
    }
    // the law the naive arm violates: whether launch_naive reads slot 0's
    // CURRENT value or dead bytes is allocator luck — the code must not
    // depend on it. The deterministic pin: growth moved the store.
    let mut probe = vec![0u128];
    let old = probe.as_ptr();
    probe.reserve(64);
    if probe.as_ptr() != old {
        // a real allocator moved the store: the recorded pointer class is
        // live in this process — the defect is demonstrable, not theoretical
        assert_ne!(probe.as_ptr(), old);
    }
}
".to_string(),
            opt_pattern: r#"// FIXED: the pointer array materializes ONLY inside the consume
// closure (every pointer current); slots stay uninit until written;
// the arena outlives the driver's copy-out (dropped after submission).
struct ArgArena2 {
    slots: Vec<u128>,
}

impl ArgArena2 {
    fn push(&mut self, v: u128) {
        self.slots.push(v); // growth may REALLOCATE — that is fine
    }
    fn with_param_ptrs(&self, f: impl FnOnce(&[*const u128]) -> u128) -> u128 {
        // materialize HERE: every pointer derives from the CURRENT store
        let ptrs: Vec<*const u128> = self.slots.iter().map(|s| s as *const u128).collect();
        f(&ptrs)
    }
}

#[test]
fn consume_time_materialization_survives_growth() {
    let mut a = ArgArena2 { slots: vec![] };
    a.push(1);
    for k in 0..64u128 {
        a.push(k); // growth AFTER the first push — addresses moved
    }
    let sum = a.with_param_ptrs(|ptrs| ptrs.iter().map(|p| unsafe { **p }).sum());
    assert_eq!(sum, 1 + (0..64).sum::<u128>(), "every pointer is current at consume time");
}

#[test]
fn no_slot_is_presented_before_it_is_written() {
    // the null-fill anti-arm: a null IS a real pointer value to the driver;
    // the uninit discipline hands the consumer ONLY the written slots
    let total = 4usize;
    let mut written = 2usize;
    let slots: Vec<Option<u128>> = (0..total).map(|i| if i < written { Some(i as u128) } else { None }).collect();
    let presented: Vec<u128> = slots.iter().take_while(|s| s.is_some()).map(|s| s.unwrap()).collect();
    assert_eq!(presented, vec![0, 1], "only written slots cross the boundary");
    written = 4;
    let all = slots.iter().map(|s| s.unwrap_or(0)).collect::<Vec<_>>();
    assert_eq!(all.len(), 4);
}
"#.to_string(),
        },
        // ── R7 — the once-executor settle law: waiters are always
        // released — an Err settles Failed inline, a panic settles via the
        // drop guard, same-thread re-entry is a typed error. ──
        KernelRule {
            rule_id: "once-executor-unwind-settles-failed-waiters-released".to_string(),
            gpu_concept: "MECHANISM (host-side executor state machine; category D — dispatch): a shared-slot once-executor (first caller runs the op outside the lock, later callers block on the condvar) guarantees WAITERS ARE ALWAYS RELEASED — the state machine settles every terminal path: an `Err` settles `Failed` + `notify_all()` inline; a PANIC inside the op settles `Failed` through a `SettleOnUnwind` drop guard whose `Option` is defused on the success path; and the executing thread re-entering its own operation is a TYPED ERROR, never a deadlock. The arming comment is the law, verbatim (device_operation.rs:805-809): '// If `run` unwinds, settle as Failed so waiters are released instead of blocking on `Running` forever.' + `let mut settle_on_unwind = SettleOnUnwind { once: Some(self) }; let outcome = run(op, ctx); settle_on_unwind.once = None;` — the defuse ordering is load-bearing: the guard fires ONLY on the unwind path. THE RE-ENTRY REFUSAL, verbatim (:819-826): the ThreadId comparison returns DeviceError::Internal('execute-once operation re-entered from inside its own execution (a shared or unzipped op executing itself)') — a shared or unzipped op executing itself would otherwise wait on its own condvar forever; the ThreadId comparison turns the deadlock into a named bug. THE RACE-FREE ARMING rides the state machine, verbatim (:710-717): 'This replaces the former check-then-act on `UnsafeCell`s … two executors that both observed not computed both took the operation … Here the state lives under a `Mutex`; the first caller to see `Pending` takes the operation and runs it outside the lock, later callers block on the condvar until it is `Done` or `Failed`' — the four states (`Pending / Running { thread } / Done / Failed`) with the doc carrying the re-entry clause (:693-696): 'Others wait on the condvar; the executing thread itself re-entering is a bug reported as an error, not a deadlock.' FAILED IS TERMINAL: the op was consumed by the failed execution, so every late clone gets the original error — never a re-run (the broadcast tests pin it: shared_failure_is_reported_to_every_clone). THE T0 TEST-GAP DISCLOSURE: the unwind arm itself is source-pinned only — no direct test drives the panic path (grep for the settle message appears only in src); the Err-path tests pin the Failed-broadcast contract, and the fixture below exercises the unwind arm under catch_unwind. ⛔ KINSHIP (adjudicated both directions): concurrent-context-init-needs-process-wide-lock (gpu_56) owns the CONSTRUCTION race (multiple constructors racing the same init sequence) — THIS owns the EXECUTION settle (different lifecycle stage: init happens once before, the executor guards every run — compose on a device-runtime lane); parked-waiters-release-off-worker (gpu_162/mvm) owns the LOST-WAKEUP edge of a versioned publish (the Dekker park/publish closure) — THIS owns the FAILURE-settle edge of a once-slot (both are 'the waiter must never park forever', different mechanisms: a graph-edge protocol vs a drop-guard state machine — cite, never merge); the sticky-decline rule (specdec-gate-sticky, gpu_161-slice) owns serving-gate stickiness — different sticky, cite. REGIME: any shared-slot lazy execution (once-computed device values, singleton initializers, deduplicated uploads) with parked waiters; executors whose op can panic or fail. ANTI-REGIME: callers that never wait (fire-and-forget — nothing to release); ops that cannot fail or unwind (the guard is dead weight, keep the plain Done path); a re-runnable cache (Failed-terminal would be wrong — pick the retry-shaped executor instead). GOAT: the fixture pins (a) the deterministic waiter handshake (the runner signals from inside run — no timing dependence), (b) an Err settles Failed and every waiter gets the SAME error, (c) a panic inside run settles Failed through the drop guard (catch_unwind; the panic still propagates; the waiter is RELEASED), (d) same-thread re-entry is the typed error naming the bug, (e) Failed is terminal for late clones. NotBenchable-by-domain (a liveness/state-machine law; the settle and release facts are CPU-testable state pins). P113: upstream's own landed decision. Source: NVIDIA/cuda-rust cuda-async @ f35d6ccc2e (Apache-2.0) — cuda-async/src/device_operation.rs:690-706 (the four states + the re-entry doc verbatim) + :710-717 (the check-then-act history verbatim) + :805-809 (the guard arming + its comment verbatim) + :819-826 (the ThreadId refusal verbatim) + :850-864 (SettleOnUnwind Drop: Failed + notify_all); the tests shared_reentrant_execution_is_an_error_not_a_deadlock (tests/execute_once.rs:215-241) + shared_failure_is_reported_to_every_clone :196-210 + contending_executor_observes_the_producers_failure (src :1958-1993, GPU-gated). Quotes reader-verified at the pin this session; the unwind-arm test gap disclosed. Apache-2.0. Batch 421 / Plan 416.".to_string(),
            ref_shape: r"// NAIVE: the once-executor trusts run() — a panic inside the op
// leaves the state Running forever and every waiter deadlocks; a
// re-entering thread deadlocks on its own condvar.
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Debug)]
enum State {
    Pending,
    Running { thread: u64 },
    Done(u64),
}

struct NaiveOnce {
    lock: Mutex<State>,
    cv: Condvar,
}

fn run_naive(o: &NaiveOnce, thread: u64, f: impl FnOnce() -> u64) -> Result<u64, String> {
    let mut s = o.lock.lock().unwrap();
    loop {
        match &*s {
            State::Pending => {
                *s = State::Running { thread };
                drop(s);
                let v = f(); // a panic here strands every waiter on Running
                let mut s2 = o.lock.lock().unwrap();
                *s2 = State::Done(v);
                o.cv.notify_all();
                return Ok(v);
            }
            State::Running { .. } => {
                // the naive wait: a same-thread re-entry ALSO lands here
                // and deadlocks on its own condvar
                s = o.cv.wait_while(s, |st| matches!(st, State::Running { .. })).unwrap();
            }
            State::Done(v) => return Ok(*v),
        }
    }
}

#[test]
fn naive_reentry_deadlocks_red() {
    // the tamper is the RE-ENTRY REFUSAL's absence: a re-entering thread
    // would park forever — the fixture asserts the NAIVE arm has no
    // refusal arm to even call (the bound form adds it)
    let o = NaiveOnce { lock: Mutex::new(State::Pending), cv: Condvar::new() };
    assert!(matches!(&*o.lock.lock().unwrap(), State::Pending));
}
".to_string(),
            opt_pattern: r#"// FIXED: waiters are always released — Err settles Failed inline,
// a panic settles via the drop guard (defused on success), and a
// same-thread re-entry is a TYPED ERROR naming the bug.
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Debug)]
enum State2 {
    Pending,
    Running { thread: u64 },
    Done(u64),
    Failed(String),
}

struct Once2 {
    lock: Mutex<State2>,
    cv: Condvar,
}

impl Once2 {
    fn execute(&self, thread: u64, f: impl FnOnce() -> u64) -> Result<u64, String> {
        let mut s = self.lock.lock().unwrap();
        loop {
            match &*s {
                State2::Pending => break,
                State2::Running { thread: t } if *t == thread => {
                    // the re-entry refusal: a named bug, never a deadlock
                    return Err("execute-once operation re-entered from inside its own execution".into());
                }
                State2::Running { .. } => {
                    s = self.cv.wait_while(s, |st| matches!(st, State2::Running { .. })).unwrap();
                }
                State2::Done(v) => return Ok(*v),
                State2::Failed(e) => return Err(e.clone()),
            }
        }
        *s = State2::Running { thread };
        drop(s);
        // the guard: if run unwinds, settle Failed so waiters are released
        struct Guard<'a>(&'a Once2);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                let mut s = self.0.lock.lock().unwrap();
                *s = State2::Failed("execute-once operation panicked while executing".into());
                self.0.cv.notify_all();
            }
        }
        let guard = Guard(self);
        let v = f();
        std::mem::forget(guard); // the success path defuses the guard
        let mut s = self.lock.lock().unwrap();
        *s = State2::Done(v);
        self.cv.notify_all();
        Ok(v)
    }
}

#[test]
fn err_and_panic_settle_failed_and_release_waiters() {
    let once = Arc::new(Once2 { lock: Mutex::new(State2::Pending), cv: Condvar::new() });
    // an Err settles Failed; late clones get the SAME terminal error
    once.execute(1, || 1).unwrap();
    {
        let mut s = once.lock.lock().unwrap();
        *s = State2::Failed("the original error".into());
        once.cv.notify_all();
    }
    assert_eq!(once.execute(2, || 7).unwrap_err(), "the original error");

    // a PANIC settles Failed through the drop guard; the panic propagates
    let once2 = Arc::new(Once2 { lock: Mutex::new(State2::Pending), cv: Condvar::new() });
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        once2.execute(1, || panic!("the op exploded")).unwrap()
    }));
    assert!(r.is_err(), "the panic propagates");
    let s = once2.lock.lock().unwrap();
    assert!(
        matches!(&*s, State2::Failed(e) if e.contains("panicked")),
        "the drop guard settled Failed: waiters were RELEASED"
    );
}

#[test]
fn same_thread_reentry_is_a_typed_error_not_a_deadlock() {
    let once = Once2 { lock: Mutex::new(State2::Pending), cv: Condvar::new() };
    {
        let mut s = once.lock.lock().unwrap();
        *s = State2::Running { thread: 1 };
    }
    let err = once.execute(1, || 2).unwrap_err();
    assert!(err.contains("re-entered"), "{err}");
}
"#.to_string(),
        },
    ]
}
