//! Issue 033 T6 — G4 (allocation) + determinism for the TETHER selection
//! blend, measured with a global counting allocator.
//!
//! The claim under test: `SelectionMode::TetherBlend` adds **zero steady-state
//! allocations** over the default mode. The fit itself is three `f64`
//! accumulators (no heap, ever); the only new heap state is the pending-pick
//! attribution buffer, which is a reused `String` pair (`clear` + `push_str`),
//! so it allocates during warm-up and never again.
//!
//! A global allocator is process-global, so this lives in its own test binary
//! (the `scope_gate_adoption::g4_alloc` precedent).
//!
//! ```bash
//! cargo test --release --test t33_tether_g4_alloc -- --nocapture
//! ```

#![cfg(feature = "clippy_lints")]

use riir_refine::selection_tether::TetherFit;
use riir_refine::{
    build_seed_corpus, HealPipeline, RustAnalyzerPruner, SelectionMode, TernaryDrafter,
};

struct CountingAllocator;

// Counted PER THREAD, not process-globally.
//
// A `static AtomicUsize` here was unusable: `cargo test` runs these tests on
// parallel threads inside ONE process, so a sibling test's allocations landed
// inside another test's `before..after` delta. Measured under the default
// harness: `t33_g4_fit_never_allocates` saw **332** allocations against an
// expected 0, from the ~70 000 its neighbour was making. Deterministic, not
// flaky — this file could never pass Layer 5c (`--all-features --tests`),
// which runs the harness at its default width.
//
// Serialising the tests with a mutex is NOT sufficient, and the difference is
// only visible on repeat runs: the harness still spawns the other test threads
// while one test is measuring, and those spawns allocate. That attempt left a
// residual `fit allocs: 6` and still failed 1 run in 3.
//
// A thread-local is safe inside a `GlobalAlloc`: `const`-initialised and
// `Cell<usize>` has no `Drop`, so access neither allocates nor registers a
// destructor, and cannot re-enter the allocator. `try_with` additionally
// tolerates access during thread teardown.
//
// Semantically equivalent for what is measured here — every path under test
// (`TetherFit::observe`/`blend`, `HealPipeline::heal`) is single-threaded, so
// there are no worker-thread allocations to miss.
thread_local! {
    static ALLOCS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get().wrapping_add(1)));
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: CountingAllocator = CountingAllocator;

fn allocs() -> usize {
    ALLOCS.with(std::cell::Cell::get)
}

/// The fit alone: 10 000 observations across ~600 windows must allocate
/// nothing at all — not "nothing after warm-up", nothing.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "allocation COUNTS are a debug artifact for a \
    zero-alloc claim: debug builds do not inline, so String/iterator machinery \
    allocates where release elides it entirely. Measured 2026-08-29 on the same \
    commit -- fit allocs debug 332 vs release 0; steady-state delta debug +13827 \
    vs release -1. This binary's own docs say --release; honour that."
)]
fn t33_g4_fit_never_allocates() {
    let mut fit = TetherFit::new();
    // Warm up the allocator's own bookkeeping outside the measured region.
    let mut sink = 0.0f32;
    let before = allocs();
    for i in 0..10_000u32 {
        let p1 = f32::from((i % 11) as u8) / 11.0;
        let p2 = f32::from((i % 7) as u8) / 7.0;
        fit.observe(f32::from(u8::from(i % 3 == 0)), p1, p2);
        sink += fit.blend(p1, p2);
    }
    let delta = allocs() - before;
    println!(
        "\n=== Issue 033 T6 — fit allocs: {delta} (rho {:.4}, windows {}, sink {sink:.1}) ===",
        fit.rho(),
        fit.stats().windows_closed
    );
    assert_eq!(delta, 0, "TetherFit must be heap-free");
}

/// The pipeline seam: TetherBlend's steady-state heal must not allocate more
/// than the default mode's.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "allocation COUNTS are a debug artifact for a \
    zero-alloc claim: debug builds do not inline, so String/iterator machinery \
    allocates where release elides it entirely. Measured 2026-08-29 on the same \
    commit -- fit allocs debug 332 vs release 0; steady-state delta debug +13827 \
    vs release -1. This binary's own docs say --release; honour that."
)]
fn t33_g4_tether_mode_adds_zero_steady_state_allocs() {
    let corpus = build_seed_corpus();

    let count_steady_heals = |mode: SelectionMode| -> usize {
        let mut p = HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new())
            .with_selection_mode(mode);
        p.seed_from_corpus(&corpus.entries);
        // Warm-up: sizes the candidate/scratch buffers AND the pending-pick
        // strings (the only new heap state TetherBlend introduces).
        for e in &corpus.entries {
            let _ = p.heal(&e.lint_name, &e.buggy_code, &corpus.entries);
        }
        let before = allocs();
        for _ in 0..3 {
            for e in &corpus.entries {
                let _ = p.heal(&e.lint_name, &e.buggy_code, &corpus.entries);
            }
        }
        allocs() - before
    };

    let default_mode = count_steady_heals(SelectionMode::BetaPosterior);
    let tether = count_steady_heals(SelectionMode::TetherBlend);
    println!(
        "\n=== Issue 033 T6 — steady-state heal allocs: default {default_mode}, \
         tether {tether} (delta {}) ===",
        tether as isize - default_mode as isize
    );
    assert!(
        tether <= default_mode,
        "TetherBlend must not add steady-state allocations \
         (default {default_mode}, tether {tether})"
    );
}

/// Determinism at the pipeline seam: the same heal + outcome stream twice
/// gives a bit-identical ρ (T6).
#[test]
fn t33_pipeline_rho_is_deterministic() {
    let corpus = build_seed_corpus();

    let run = || -> riir_refine::TetherStats {
        let mut p = HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new())
            .with_selection_mode(SelectionMode::TetherBlend);
        p.seed_from_corpus(&corpus.entries);
        for pass in 0..4 {
            for (i, e) in corpus.entries.iter().enumerate() {
                if let Some(h) = p.heal(&e.lint_name, &e.buggy_code, &corpus.entries) {
                    // A deterministic pseudo-verdict — this test measures
                    // reproducibility, not heal quality. `verified: true` is
                    // required: the fit ignores unverified outcomes (the
                    // Issue-018 oracle split at this seam).
                    let success = (i + pass) % 3 != 0;
                    p.report_outcome(&e.lint_name, &e.buggy_code, &h.fixed_code, success, true);
                }
            }
        }
        p.flush_tether_window();
        p.tether_stats()
    };

    let (a, b) = (run(), run());
    println!("\n=== Issue 033 T6 — pipeline rho determinism: {a:?} ===");
    assert_eq!(a.rho.to_bits(), b.rho.to_bits(), "rho must be reproducible");
    assert_eq!(a.windows_closed, b.windows_closed);
    assert!(
        a.windows_closed > 0,
        "the pipeline must actually feed the fit — a never-closing window \
         would make this test vacuous"
    );
}
