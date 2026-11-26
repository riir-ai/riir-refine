// Batch 425 — the ThunderKittens 2.0 distill's `rust_perf` member (Plan 419 /
// Research 453). Sources: the post https://hazyresearch.stanford.edu/blog/
// 2026-02-19-tk-2 (Sul/Ré, 2026-02-19 — the URL the pin, live-fetch posture) +
// HazyResearch/ThunderKittens @ `67845f5fa48d05343dbbc1ba1403f11061f08d2d`
// (tip 2026-09-12, MIT). ONE rule:
// gpu-bench-input-values-power-throttling (measurement-honesty family — that
// family's home per the Batch-413 domain-routing precedent).
//
// Dedup re-verified at the landing HEAD (both vocabularies, every hit read):
// `throttl|power|bit.flip|integer.like|input.distribution|input group|warmup`
// over the rust_perf measurement-honesty family (entries_70/79/84/94/98/100/
// 102/103/104/105) — `benchmark-input-distribution-realism` (B408) owns the
// MEMORY-behavior axis (locality vs dispersion: zeros and random are both
// lies about CACHE behavior), `isolated-bench-cache-state-vs-live` (B408)
// owns the cache-STATE posture axis, `benchmark-spread-above-threshold`
// (entries_70) owns the warmup-CV axis, `bench-ratio-omitted-when-baseline-
// not-live` (B417) owns the baseline-provenance axis — ZERO owner of the
// POWER axis (input VALUES' bit-flip density → power draw → clock throttle).
// Adjacent composed in-rule both directions.

use crate::domains::rust_perf::PerfRule;

pub fn b425_rules() -> Vec<PerfRule> {
    vec![
        // ── R5: input VALUES are a power axis — the comparison must fix ──
        PerfRule {
            rule_id: "gpu-bench-input-values-power-throttling".to_string(),
            name: "a GPU benchmark's input VALUES are a power axis: integer-like matrices flip fewer transistor bits per cycle than real-range values, drawing less power and sustaining higher clocks, so a kernel benched on rounded/integer inputs reads faster for POWER reasons, not code reasons — the CUTLASS Profiler's default input rounding made competitor kernels read 100-150 TFLOPs faster than they were; compare kernels ONLY on bitwise-identical real-distribution inputs (one power posture, identical bytes both arms)".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// A GPU kernel comparison benched through a profiler whose
// input setup ROUNDS the matrices to integers (or seeds
// integer-like patterns): whichever kernel it benched reads
// "faster" — the measured gap is POWER (bit-flip density →
// clocks), not code quality.
#[derive(Clone, Copy)]
enum Fill {
    /// the profiler's rounding: values in a tiny integer set
    Rounded,
    /// the real range: uniform [-1, 1) f32 bits
    RealRange,
}

fn bench_kernel_naive(kernel: &str, fill: Fill) -> f64 {
    let inputs = make_inputs(fill);
    // ... the measured ms over the profiled iterations
    measure_ms(kernel, &inputs)
}

fn report_naive() {
    // BOTH arms run on Rounded bytes: the comparison looks fair,
    // but BOTH numbers are inflated — and neither says so.
    let ours = bench_kernel_naive("ours", Fill::Rounded);
    let theirs = bench_kernel_naive("theirs", Fill::Rounded);
    println!("ours {ours:.3} ms vs theirs {theirs:.3} ms");
}

fn make_inputs(_fill: Fill) -> Vec<f32> {
    Vec::new()
}

fn measure_ms(_kernel: &str, _inputs: &[f32]) -> f64 {
    1.0
}
"#.to_string(),
            fixed_code: r#"// FIXED: bitwise-identical REAL-distribution inputs across ALL
// compared kernels — one power posture, identical bytes — and the
// input sizing/rotation discipline for the cache axis (input groups
// sized to ~3x L2 rotate residency naturally; explicit cache
// flushing measures SLOWER even with event timing, so rotation is
// the cold-side mechanism).
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Clone, Copy)]
enum Fill {
    Rounded, // the profiler's rounding — NEVER the comparison input
    RealRange,
}

/// SplitMix64-style uniform fill, seeded PER GROUP so every compared
/// kernel takes byte-identical inputs (the seed is the pin).
fn make_inputs_seeded(fill: Fill, group_seed: u64, n: usize) -> Vec<f32> {
    match fill {
        Fill::Rounded => Vec::new(), // never the comparison input
        Fill::RealRange => (0..n)
            .map(|i| {
                let mut h = DefaultHasher::new();
                (group_seed.wrapping_add(i as u64)).hash(&mut h);
                let x = h.finish();
                // upper 24 bits → [0, 1) → scale to [-1, 1)
                let u = (x >> 40) as f32 * (1.0 / 16_777_216.0);
                u * 2.0 - 1.0
            })
            .collect(),
    }
}

/// the input-group rotation: inputs sized below ~3x L2 rotate through
/// `groups` byte-identical sets — natural eviction, no explicit flush.
fn bench_kernel(kernel: &str, groups: &[Vec<f32>]) -> f64 {
    let n = groups.len();
    let mut acc = 0.0f64;
    for i in 0..100 {
        let idx = i % n;
        acc += measure_ms(kernel, &groups[idx]);
    }
    acc / 100.0
}

fn report_fixed() {
    // ONE group of bytes, seeded once, given to BOTH arms: the power
    // posture is identical, so the gap that remains is code.
    let groups: Vec<Vec<f32>> = (0..3).map(|g| make_inputs_seeded(Fill::RealRange, g * 100, 4096)).collect();
    let ours = bench_kernel("ours", &groups);
    let theirs = bench_kernel("theirs", &groups);
    println!("ours {ours:.3} ms vs theirs {theirs:.3} ms — identical bytes, one power posture");
}
"#.to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class). FULL PROVENANCE: the post https://hazyresearch.stanford.edu/blog/2026-02-19-tk-2 §\"Benchmarking GPU kernels properly\" (Sul/Ré, 2026-02-19, the URL the pin; quotes verified at the live text) + the in-source convention (HazyResearch/ThunderKittens @ 67845f5fa48d MIT — the SplitMix64 uniform [-1,1) fill at kernels/gemm/common.cuh:28-48 and the input-group sizing at kernels/gemm/bf16_b200/bf16_b200_gemm.cu:266-271). THE CONFOUND, post verbatim: 'the CUTLASS Profiler, by default, rounds input values to nearest integers before passing them to the kernels... Integer-like input matrices produce less bit-flipping in the GPU's transistors, which reduces power consumption and, consequently, the likelihood of clock throttling' — their kernels read 100-150 TFLOPs faster under the profiler than against it ('we were adding the prefix \"cutlass\" to our kernel names and were half-convinced that Nvidia had private assembler optimization passes'). THE CONVENTION (post): bitwise-identical inputs (uniform, real range) across all compared kernels · input groups when the working set is under ~3x L2 ('each group's data naturally evicts the previous group's residency in L2... explicit cache flushing meaningfully slows down the measured time even with proper CUDA event usage' — the cold-side mechanism refinement of the cache-state axis) · 500 warmup to power steady state · back-to-back launches under one event pair · idle cooldown between configs. ⛔ ADJUDICATIONS (both directions): vs benchmark-input-distribution-realism (B408) — B408 owns the MEMORY axis (zeros maximize locality, random maximizes dispersion → cache behavior lies) and mandates the workload's real DISTRIBUTION; THIS owns the POWER axis (the same distribution at different VALUE SMOOTHNESS draws different power → clock behavior lies) — a workload can be distribution-realistic AND power-skewed when one arm's values are smoother; compose: real distribution (B408) AND identical bytes across arms (THIS); the fixture pins the skew direction + the cancel-under-identical-bytes law. vs isolated-bench-cache-state-vs-live (B408) — that rule says measure BOTH cache postures and select in-situ; THIS supplies the cold-side MECHANISM (input-group rotation, never explicit flush, which 'meaningfully slows down the measured time'); compose. vs benchmark-spread-above-threshold-reports-unstable (entries_70) — its rolling-CV warmup settles TIMING noise; the 500-warmup power-steady-state convention is its fixed-count positive instance (no new axis). vs bench-ratio-omitted-when-baseline-not-live (B417) — that rule omits the ratio against a foreign baseline; THIS fixes the bytes the ratio was measured ON (different layers of the same comparison; compose). REGIME: GPU/power-limited benchmark comparisons (tensor-core kernels, memory-heavy kernels); the profiler/tool input-setup defaults are the classic source. ANTI-REGIME: CPU benches on non-power-throttling silicon (the axis is inert — the identical-bytes discipline is still good hygiene); single-kernel absolute claims with no comparison (the skew applies, but there is no cross-arm lie — the entries_84 degenerate-output family owns that surface). Batch 425 / Plan 419 / Research 453.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
