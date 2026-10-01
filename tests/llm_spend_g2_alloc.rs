//! Issue 046 G2 — the spend-gate check path is alloc-free.
//!
//! Own test binary on purpose (the `kat_meter_g2_alloc` convention): the
//! global `CountingAllocator` counts every thread's allocations, so parallel
//! sibling tests would pollute the before/after bracket. A single-test binary
//! has no siblings.
//!
//! The pre-call gate runs before EVERY paid generation — it must not allocate
//! (integer epoch-days, table-scan pricing, f64 math; the only Strings live
//! on the cold decline/error paths, which this fixture also avoids by
//! construction… except where pinned below).
//!
//! ```bash
//! cargo test --features cf_workers_ai --test llm_spend_g2_alloc
//! ```

#![cfg(feature = "cf_workers_ai")]

use riir_refine::llm_spend::{est_input_tokens, SpendLedger};

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
fn g2_reserve_reconcile_release_path_is_alloc_free() {
    // Build everything allocating OUTSIDE the brackets (the prompt, the
    // ledger at a temp path — open does file I/O which allocates).
    let dir = std::env::temp_dir().join(format!("llm_g2_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut prompt = String::with_capacity(48 * 1024);
    while prompt.len() < 48 * 1024 {
        prompt.push_str("fn token_line(idx: usize) -> u32 { (idx as u32) * 3 + 1 } // pad\n");
    }
    const QWEN: &str = "@cf/qwen/qwen3.8-27b";
    let mut l = SpendLedger::open_at_with_cap(dir.join("spend.json"), 1.0);

    // Warm any one-time lazy state, then bracket the three gate operations.
    let warm = l.reserve(QWEN, &prompt, 512).unwrap();
    l.reconcile(warm, QWEN, 8_000, 400, true);
    let warm2 = l.reserve(QWEN, &prompt, 512).unwrap();
    l.release(warm2);

    let before = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    let reserved = l.reserve(QWEN, &prompt, 512).unwrap();
    let mid = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    l.reconcile(reserved, QWEN, 8_000, 400, true);
    let after_reconcile = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    let r3 = l.reserve(QWEN, &prompt, 256).unwrap();
    let after_reserve3 = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    l.release(r3);
    let after_release = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);

    assert_eq!(mid, before, "reserve must not allocate");
    assert_eq!(after_reconcile, mid, "reconcile must not allocate");
    assert_eq!(
        after_reserve3, after_reconcile,
        "a second reserve stays alloc-free"
    );
    assert_eq!(after_release, after_reserve3, "release must not allocate");

    // The estimate helper is on the same hot path.
    let b = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    let est = est_input_tokens(&prompt);
    let a = ALLOCS.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(a, b, "est_input_tokens must not allocate");
    assert!(
        est > 10_000,
        "sanity: a 48 KiB prompt estimates a real token count (got {est})"
    );

    let _ = std::fs::remove_dir_all(dir);
    println!("\n=== llm-spend G2 — reserve/reconcile/release, 0 allocs, est {est} in-tokens ===");
}
