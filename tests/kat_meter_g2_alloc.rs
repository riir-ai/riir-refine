//! KAT meter G2 alloc gate — Proposal 005 Phase 0 T0.5 (Plan 023).
//!
//! Lives in its OWN test binary on purpose: the global `CountingAllocator`
//! counts every thread's allocations, so parallel sibling tests would pollute
//! the before/after bracket. A single-test binary has no siblings (the
//! `t33_tether_g4_alloc` convention, hardened).
//!
//! ```bash
//! cargo test --features kat_service --test kat_meter_g2_alloc
//! ```

#![cfg(feature = "kat_service")]

use riir_refine::kat_meter::count_word_tokens;

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
}

#[global_allocator]
static A: CountingAllocator = CountingAllocator;

#[test]
fn g2_count_path_is_alloc_free() {
    // Build the payload FIRST (its allocations are outside the bracket),
    // then bracket ONLY the count call. 1 MiB of code-like bytes.
    let mut payload = Vec::with_capacity(1024 * 1024);
    let line = b"fn token_line(idx: usize) -> u32 { (idx as u32) * 3 + 1 } // padding padding\n";
    while payload.len() < 1024 * 1024 {
        payload.extend_from_slice(line);
    }
    // Warm the ambient allocation sources OUTSIDE the bracket: libtest
    // machinery (thread-local init, panic-formatting buffers) allocates
    // lazily on first use and the sites differ per platform — measured live
    // on the first linux run (2026-09-03): +4 allocs inside the bracket
    // (180 vs 176) that a macOS run never sees. The warm-up call absorbs
    // them; the bracket then measures only count_word_tokens.
    let _ = count_word_tokens(&payload);
    let before = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    let n = count_word_tokens(&payload);
    let after = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(after, before, "count_word_tokens must not allocate");
    assert!(
        n > 100_000,
        "sanity: 1 MiB of code bills a substantial count (got {n})"
    );
    println!("\n=== KAT meter G2 — 1 MiB counted, 0 allocs, {n} word tokens ===");
}
