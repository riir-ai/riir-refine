//! Issue 248 G4 — the armed novelty-gated `ema_step` path is alloc-free.
//! Own test binary on purpose: the counting global allocator counts every
//! thread, so one test per binary (the `points_drain_g4_alloc` convention).
//!
//! ```bash
//! cargo test --features evolve_novelty --test evolve_novelty_g4_alloc
//! ```

#![cfg(feature = "evolve_novelty")]

use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use riir_refine::embedder::embed;
use riir_refine::evolve::{
    EvolveConfig, LatentSkillEvolution, NoveltyChannel, NoveltyRetentionConfig, ema_step_gated,
};

struct CountingAllocator;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { std::alloc::System.realloc(ptr, layout, new_size) }
    }
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { std::alloc::System.alloc_zeroed(layout) }
    }
}

#[global_allocator]
static A: CountingAllocator = CountingAllocator;

#[test]
fn armed_paths_are_alloc_free() {
    let a = embed("g4_rule", "fix_a");
    let b = embed("g4_rule", "fix_b");
    let cfg = EvolveConfig::default();
    let ncfg = NoveltyRetentionConfig::default().validated().unwrap();

    // ── bracket 1: the free fn ─────────────────────────────────────────
    let mut dir = a;
    let mut ch = NoveltyChannel::new();
    let _ = ema_step_gated(&mut dir, &b, true, &cfg, &mut ch, &ncfg);
    let before = ALLOCS.load(Ordering::Relaxed);
    for i in 0..1_000u32 {
        let fix = if i % 3 == 0 { &a } else { &b };
        black_box(ema_step_gated(
            &mut dir,
            black_box(fix),
            i % 2 == 0,
            &cfg,
            &mut ch,
            &ncfg,
        ));
    }
    let free_fn = ALLOCS.load(Ordering::Relaxed) - before;

    // ── bracket 2: the armed layer, after the first-success warm-up ───
    let mut ev = LatentSkillEvolution::with_capacity(4);
    assert!(ev.arm_novelty(ncfg));
    ev.record_outcome("g4_rule", &a, true); // warm-up: entry + channel insert
    let before = ALLOCS.load(Ordering::Relaxed);
    for i in 0..1_000u32 {
        let fix = if i % 3 == 0 { &a } else { &b };
        ev.record_outcome("g4_rule", black_box(fix), i % 2 == 0);
    }
    let layer = ALLOCS.load(Ordering::Relaxed) - before;

    assert_eq!(free_fn, 0, "ema_step_gated allocated {free_fn}×");
    assert_eq!(layer, 0, "armed record_outcome allocated {layer}×");
    black_box(dir);
}
