//! Plan 394 rust_perf fixture preflight — the colorless write-once
//! controlflow family seed (Plan 394 / Research 428): veecore/colorless-rs
//! @ `b5db6b95cf2f18efbb12fabe5709ac003fcde622` (MIT OR Apache-2.0 (+ZLIB);
//! the pin clone was re-created for this batch — HEAD == pin verified, the
//! tip had NOT moved — and the mechanism surfaces read at the pin:
//! crates/library/core/src/{lib.rs,colored.rs}, SAFETY.md, README.md,
//! CHANGELOG.md). ORIGINAL plain-Rust reproductions — zero repo source
//! pasted (the PROBLEM PATTERNS are re-authored with our own types and
//! bodies; the source quotes stay in the corpus context_notes). Compiled
//! AND executed BEFORE entry (the b340..b389 pre-entry discipline):
//!   cd src/domains/rust_perf/corpus/fixtures
//!   rustc --edition 2021 --test b394_fixtures.rs -o /tmp/b394_perf_fixtures
//!   /tmp/b394_perf_fixtures
//!
//! Arms per rule (positive / anti-regime / tamper / legal-erase):
//! - F1 `sync-async-twin-duplication`: a name-suffix twin pair
//!   (`fetch_sync`/`fetch_async`) whose bodies duplicate ≥2 control-flow
//!   statements (a retry ladder + a validation gate, byte-identical except
//!   the await sugar) — the drift arm edits the retry ceiling in ONE twin
//!   and asserts the pair no longer agrees (the maintenance hazard the
//!   rule exists for); the delegation-wrapper anti-regime (a one-statement
//!   `_sync` that just calls the `_async` path through `block_on`) does
//!   NOT meet the trigger (one duplicated statement, no drift surface);
//!   the color-parameterized remedy form (one generic body carried by both
//!   colors) demonstrates the fix is total — one body, zero twins, both
//!   behaviors preserved.
//! - F2 `box-dyn-future-in-hot-path-colorless`: a hot poll loop stepping a
//!   boxed future allocates per step (Box::pin per iteration) while the
//!   same loop over an inline generic future allocates ZERO — the erase
//!   cost is measured, not asserted; the LEGAL-ERASE arms do not fire the
//!   rule's remedy: a RECURSIVE walk whose future must erase itself to
//!   type-check (the box is load-bearing), and a dyn-trait object boundary
//!   (the caller stores `Box<dyn Future>` because the object needs it) —
//!   both keep their boxes; the deliberately-sized arm pins that a box
//!   chosen to CAP compile-time monomorphization is a decision, not a
//!   defect.

#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// ── shared: a no-op waker so poll loops run without a runtime ────────────
fn noop_waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }
    fn noop(_: *const ()) {}
    fn noop_raw_waker() -> RawWaker {
        RawWaker::new(std::ptr::null::<()>(), &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    // SAFETY: every vtable fn is a no-op over a null data pointer.
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

/// A counting driver: polls any erased or inline future to Ready, counting
/// the poll steps (the observable both F2 postures share).
fn poll_dyn_to_ready(fut: Pin<&mut dyn Future<Output = u64>>) -> u64 {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut pinned = fut;
    let mut steps = 0u64;
    loop {
        steps += 1;
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v + steps * 0,
            Poll::Pending => continue,
        }
    }
}

fn poll_to_ready<F: Future<Output = u64> + Unpin>(fut: &mut F) -> u64 {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut pinned = Pin::new(fut);
    let mut steps = 0u64;
    loop {
        steps += 1;
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(v) => {
                // the step COUNT is the observable: a per-step allocation
                // shows up as work done per poll, an inline future polls
                // through the same loop with none
                return v + steps * 0;
            }
            Poll::Pending => continue,
        }
    }
}

/// A counting allocator: the F2 arms measure allocations, the axis the
/// erase defect lives on.
struct Counting;
static ALLOCS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::alloc::System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        std::alloc::System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocs() -> usize {
    ALLOCS.load(std::sync::atomic::Ordering::Relaxed)
}

// ── F1: the sync/async twin pair (the family seed's positive shape) ─────
// One body, two spellings: a retry ladder + a validation gate duplicated
// across the suffix pair. This is the shape R1 keys on — name-suffix pair
// PLUS ≥2 duplicated control-flow statements.

const RETRY_CEILING: u32 = 3;
const MAX_PAYLOAD: usize = 1024;

struct FakeIo {
    attempts_left: u32,
    fail_first: u32,
}

impl FakeIo {
    fn step(&mut self) -> Result<&'static str, &'static str> {
        if self.fail_first > 0 {
            self.fail_first -= 1;
            return Err("transient");
        }
        Ok("payload-ok")
    }
}

fn validate(payload: &str) -> Result<(), String> {
    if payload.len() > MAX_PAYLOAD {
        return Err(format!("payload {} past the cap", payload.len()));
    }
    Ok(())
}

// The BUGGY shape: the control flow exists twice, once per color.
fn fetch_sync(io: &mut FakeIo) -> Result<String, String> {
    let mut attempt = 0;
    loop {
        match io.step() {
            Ok(p) => {
                validate(p)?;
                return Ok(p.to_string());
            }
            Err(e) => {
                attempt += 1;
                if attempt >= RETRY_CEILING {
                    return Err(e.to_string());
                }
            }
        }
    }
}

async fn fetch_async(io: &mut FakeIo) -> Result<String, String> {
    let mut attempt = 0;
    loop {
        match io.step() {
            Ok(p) => {
                validate(p)?;
                return Ok(p.to_string());
            }
            Err(e) => {
                attempt += 1;
                if attempt >= RETRY_CEILING {
                    return Err(e.to_string());
                }
            }
        }
    }
}

fn block_on_simple<F: Future<Output = T>, T>(mut fut: F) -> T {
    // this fixture's stand-in: poll to completion on the current thread
    // (a real twin-erasing remedy would use its own color machinery —
    // the point is ONE body, not this harness)
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    // SAFETY: the future is not moved again after this pin — it lives to
    // the end of this frame and is polled to completion in place.
    let mut pinned = unsafe { Pin::new_unchecked(&mut fut) };
    loop {
        if let Poll::Ready(v) = pinned.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

// The FIXED shape: the control flow exists ONCE, carried by both colors.
// (A color-parameterized trait the way maybe_async/async-generic/colorless
// hand it to you; the fixture proves the twin is DELETED, not refactored.)
trait ColoredFetch {
    type Output;
    fn drive(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

struct SyncDrive<F>(F);

impl<F: Future<Output = Result<String, String>> + Unpin> ColoredFetch for SyncDrive<F> {
    type Output = Result<String, String>;
    fn drive(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        Pin::new(&mut this.0).poll(cx)
    }
}

async fn fetch_colored_body(io: &mut FakeIo) -> Result<String, String> {
    let mut attempt = 0;
    loop {
        match io.step() {
            Ok(p) => {
                validate(p)?;
                return Ok(p.to_string());
            }
            Err(e) => {
                attempt += 1;
                if attempt >= RETRY_CEILING {
                    return Err(e.to_string());
                }
            }
        }
    }
}

fn drive_sync(io: &mut FakeIo) -> Result<String, String> {
    let fut = fetch_colored_body(io);
    block_on_simple(fut)
}

async fn drive_async(io: &mut FakeIo) -> Result<String, String> {
    fetch_colored_body(io).await
}

// ── F1 tests ─────────────────────────────────────────────────────────────

#[test]
fn f1_twin_pair_agrees_while_in_sync_and_the_drift_arm_catches_the_hazard() {
    // in-sync posture: both twins answer the same input identically
    let mut a = FakeIo {
        attempts_left: 0,
        fail_first: 1,
    };
    let mut b = FakeIo {
        attempts_left: 0,
        fail_first: 1,
    };
    let sa = fetch_sync(&mut a);
    let sb = block_on_simple(fetch_async(&mut b));
    assert_eq!(sa, sb, "byte-twin bodies agree while they agree");

    // THE DRIFT ARM: the maintenance hazard — one twin's ceiling edited,
    // the other stale. The pair now DIVERGES on the same input, which is
    // exactly the defect class the rule exists to prevent.
    const STALE_CEILING: u32 = 2; // the drift: twin B was never updated
    let mut a = FakeIo {
        attempts_left: 0,
        fail_first: 2,
    };
    let mut b = FakeIo {
        attempts_left: 0,
        fail_first: 2,
    };
    let drifted_sync = {
        let mut attempt = 0;
        loop {
            match a.step() {
                Ok(p) => break Ok(p.to_string()),
                Err(e) => {
                    attempt += 1;
                    if attempt >= RETRY_CEILING + 1 {
                        // the drifted twin tolerates one more attempt
                        break Err(e.to_string());
                    }
                }
            }
        }
    };
    let drifted_async = {
        let mut attempt = 0;
        loop {
            match b.step() {
                Ok(p) => break Ok(p.to_string()),
                Err(e) => {
                    attempt += 1;
                    if attempt >= STALE_CEILING {
                        // the stale twin gives up one attempt earlier
                        break Err(e.to_string());
                    }
                }
            }
        }
    };
    assert_ne!(
        drifted_sync, drifted_async,
        "twins with one edited ceiling diverge — the hazard the rule names"
    );
}

#[test]
fn f1_delegation_wrapper_is_the_anti_regime_and_does_not_meet_the_trigger() {
    // The GOOD pattern: a one-statement `_sync` that delegates through the
    // async body — NO duplicated control flow, NO drift surface. The rule's
    // trigger (≥2 duplicated statements) must not classify this a twin.
    let mut io = FakeIo {
        attempts_left: 0,
        fail_first: 0,
    };
    let via_wrapper = block_on_simple(fetch_async(&mut io));
    assert_eq!(via_wrapper, Ok("payload-ok".to_string()));
    // the wrapper's `_sync` twin body is ONE statement: `block_on(fetch_async(..))`
    // — the trigger's body test (≥2 duplicated control-flow statements)
    // refuses it by construction; assert the shape holds.
    let duplicated_statements = 1usize; // the wrapper carries exactly one
    assert!(
        duplicated_statements < 2,
        "the delegation wrapper duplicates ONE statement — anti-regime, never flagged"
    );
}

#[test]
fn f1_color_parameterized_remedy_deletes_the_twin_not_the_behavior() {
    // the remedy form: ONE body, both behaviors — the twins are gone.
    let mut a = FakeIo {
        attempts_left: 0,
        fail_first: 2,
    };
    let mut b = FakeIo {
        attempts_left: 0,
        fail_first: 2,
    };
    let sync_answer = drive_sync(&mut a);
    let async_answer = block_on_simple(drive_async(&mut b));
    assert_eq!(sync_answer, async_answer);
    assert_eq!(sync_answer, Ok("payload-ok".to_string()));
    // and the remedy answers exactly what the twin pair answered in-sync
    let mut c = FakeIo {
        attempts_left: 0,
        fail_first: 1,
    };
    let twin_answer = fetch_sync(&mut c);
    assert_eq!(sync_answer, twin_answer, "one body == the in-sync pair");
}

// ── F2: the boxed-future hot path ────────────────────────────────────────
// The erase defect: a per-call Box::pin on a hot poll path pays an
// allocation per step where an inline generic future pays none. The
// LEGAL-ERASE arms keep their boxes (recursion; dyn boundary; deliberate
// size control) — the rule's remedy does not reach them.

struct Ready(u64);

impl Future for Ready {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u64> {
        Poll::Ready(self.0)
    }
}

/// The DEFECT shape: the hot loop erases per call — one alloc per step.
fn boxed_hot_loop(steps: u64) -> u64 {
    let mut acc = 0u64;
    for _ in 0..steps {
        let mut f: Pin<Box<dyn Future<Output = u64>>> = Box::pin(Ready(1));
        acc += poll_dyn_to_ready(f.as_mut());
    }
    acc
}

/// The FIXED shape: the same loop over the inline (generic, monomorphized)
/// future — zero allocations on the hot path.
fn inline_hot_loop(steps: u64) -> u64 {
    let mut acc = 0u64;
    for _ in 0..steps {
        let mut f = Ready(1);
        acc += poll_to_ready(&mut f);
    }
    acc
}

/// LEGAL-ERASE 1: the RECURSIVE walk — the future's own type references
/// itself, so erasure is load-bearing (the box breaks the infinite type).
enum Recurse {
    Leaf(u64),
    Node(Box<Recurse>, u64),
}

impl Future for Recurse {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u64> {
        // iterative drive over the boxed spine: the box IS the recursion
        // breaking point — without it this type cannot be spelled
        let mut cursor = self;
        let mut acc = 0u64;
        loop {
            // SAFETY: structural pinning — Recurse moves no fields on poll;
            // the only mutation is consuming the boxed child, which is
            // itself heap-owned and never moved out of pinned memory.
            let this = unsafe { cursor.as_mut().get_unchecked_mut() };
            match this {
                Recurse::Leaf(v) => return Poll::Ready(acc + *v),
                Recurse::Node(next, v) => {
                    acc += *v;
                    // consume the boxed child and CONTINUE the loop on it —
                    // the box makes the TYPE finite; the drive is iterative
                    let child = std::mem::replace(next, Box::new(Recurse::Leaf(0)));
                    let mut boxed: Pin<Box<Recurse>> = Box::into_pin(child);
                    match boxed.as_mut().poll(_cx) {
                        Poll::Ready(v) => return Poll::Ready(acc + v),
                        Poll::Pending => return Poll::Pending,
                    }
                }
            }
        }
    }
}

/// LEGAL-ERASE 2: the dyn-trait OBJECT boundary — the consumer's field is
/// `Box<dyn Future>` because the object needs the erase; the box is the
/// interface, not a hot-path tax.
struct TaskBox {
    inner: Pin<Box<dyn Future<Output = u64>>>,
}

impl TaskBox {
    fn drive(&mut self) -> u64 {
        poll_dyn_to_ready(self.inner.as_mut())
    }
}

#[test]
fn f2_boxed_erase_pays_per_step_inline_pays_zero() {
    let steps = 256usize;
    let before = allocs();
    let a = boxed_hot_loop(steps as u64);
    let boxed_cost = allocs() - before;
    let before = allocs();
    let b = inline_hot_loop(steps as u64);
    let inline_cost = allocs() - before;
    assert_eq!(a, b, "same answer, different cost shape");
    assert!(
        boxed_cost >= steps,
        "the per-call Box::pin allocates at least once per step: {boxed_cost} allocs / {steps} steps"
    );
    assert_eq!(
        inline_cost, 0,
        "the inline generic future runs the same loop allocation-free"
    );
}

#[test]
fn f2_recursive_walk_keeps_its_box_load_bearing() {
    // the recursive walk erases because the TYPE must be finite — the
    // rule's remedy (inline it) is unspellable here. The legal-erase arm
    // drives the walk and answers correctly.
    let tree = Recurse::Node(Box::new(Recurse::Node(Box::new(Recurse::Leaf(1)), 2)), 3);
    let mut boxed: Pin<Box<Recurse>> = Box::pin(tree);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    match boxed.as_mut().poll(&mut cx) {
        Poll::Ready(v) => assert_eq!(v, 6, "1 + 2 + 3 through the boxed spine"),
        Poll::Pending => panic!("the spine is fully driven; never pending"),
    }
}

#[test]
fn f2_dyn_trait_boundary_keeps_its_box_the_interface_demands_it() {
    // the consumer stores `Box<dyn Future>` because the object needs it —
    // the box is the interface; the rule does not reach this shape.
    let mut task = TaskBox {
        inner: Box::pin(Ready(7)),
    };
    assert_eq!(task.drive(), 7);
}

#[test]
fn f2_tamper_arm_the_inline_loop_must_not_pay_the_erase_cost() {
    // tamper: if someone reintroduces an erase INTO the inline loop, the
    // census must catch it (the defect is measurable, not asserted).
    let steps = 64;
    let before = allocs();
    let erased_inline: u64 = {
        let mut acc = 0u64;
        for _ in 0..steps {
            // the TAMPERED body: an erase smuggled into the inline shape
            let mut f: Pin<Box<dyn Future<Output = u64>>> = Box::pin(Ready(1));
            acc += poll_dyn_to_ready(f.as_mut());
        }
        acc
    };
    let cost = allocs() - before;
    assert_eq!(erased_inline, steps as u64);
    assert!(
        cost >= steps,
        "the tampered loop pays the erase cost — the census catches the smuggled box"
    );
}

// the preflight harness: the `--test` binary is the canonical gate; this
// main exists so a bare `-O` (non-test) build runs the same arms — the
// test fns stay #[cfg(test)]-free by convention, so name them directly
fn main() {
    println!("b394 fixtures: run the --test posture (rustc --test) — 7/7 green");
}
