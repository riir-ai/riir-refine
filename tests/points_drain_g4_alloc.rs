//! Plan 326 T6(f) — the G4 alloc canary on the drain pre-check (Proposal
//! 027: "two integer compares; no allocation"). Own test binary on
//! purpose: the counting global allocator counts every thread, so
//! parallel sibling tests would pollute the before/after bracket (the
//! `kat_meter_g2_alloc` convention — one test, one binary).
//!
//! ```bash
//! cargo test --features points_drain --test points_drain_g4_alloc
//! ```

#![cfg(feature = "points_drain")]

use riir_refine::points_drain::{DayState, DrainGate, DrainVerdict, allow_hosted};

struct CountingAllocator;

static ALLOCS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        unsafe { std::alloc::System.realloc(ptr, layout, new_size) }
    }
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        unsafe { std::alloc::System.alloc_zeroed(layout) }
    }
}

#[global_allocator]
static A: CountingAllocator = CountingAllocator;

const TODAY: &str = "2026-11-02";

#[test]
fn the_precheck_paths_are_alloc_free() {
    // ── bracket 1: the pure decision, steady state ─────────────────────
    // The warmup call absorbs the once-per-day roll (its String write is
    // the ROLL, not the check) and any lazy libtest machinery.
    let drain = riir_ledger::DrainState {
        allowance_remaining_today: 1_000_000,
        billable_ceiling_today: 1_000_000,
        rate_served: 2,
    };
    let mut state = DayState::default();
    assert_eq!(
        allow_hosted(&mut state, &drain, TODAY),
        DrainVerdict::AllowedFree
    );
    let before = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    for _ in 0..1_000 {
        assert_eq!(
            allow_hosted(&mut state, &drain, TODAY),
            DrainVerdict::AllowedFree
        );
    }
    let after = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        after, before,
        "the steady-state pre-check must not allocate"
    );

    // ── bracket 2: the inert gate pass (the byte-identical posture) ───
    let mut gate = DrainGate::inert();
    let _ = gate.begin_hosted_call(b"body"); // warmup
    let before = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    for _ in 0..1_000 {
        let _ = gate.begin_hosted_call(b"body").expect("inert allows");
    }
    let after = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(after, before, "the inert pre-check pass must not allocate");
}
