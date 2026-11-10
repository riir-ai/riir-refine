//! POC 023 — empirical answers to the two Plan 005 design questions that a
//! review raised against the unstarted Phase 9 (calibrated UQ) and Phase 11
//! (Poincaré fix-imagination).
//!
//! Both questions were raised as *arithmetic doubts*, not as opinions, so both
//! are decidable by measurement. This file is the measurement. It is a POC —
//! it prints numbers and asserts only the load-bearing conclusions, so it
//! stays green as a regression witness once the plan is refined.
//!
//! ## Q1 (Phase 9) — how many residuals before a per-rule 95% interval is real?
//!
//! T9.2 proposes `ConformalIntervalCalibrator<FixForecaster>` with a **per-rule**
//! residual ring of capacity 64, and T9.5 gates on "empirical coverage of the
//! 95% interval ≥ 90%". The shipped calibrator reads a *weighted empirical
//! quantile* over the pool (`weighted_quantile_pair`), and on an empty pool it
//! returns `(0.0, 0.0)` — a **zero-width interval centred on the point
//! forecast**. That is the dangerous small-sample failure mode: not a wide
//! useless interval, but a silent claim of perfect confidence. This POC sweeps
//! the pool size and reports the warm-up point where coverage crosses the gate.
//!
//! ## Q2 (Phase 11) — is the L2 validator actually the bottleneck?
//!
//! T11.5 asserts "the imagination gate is µs-scale; the L2 pruner is ms-scale;
//! net throughput rises if imagination rejects ≥20% of bad fixes" (T11.6 then
//! gates at ≥30% rejection and ≥1.5× latency). The premise is that L2 dominates.
//! But riir-refine's default L2 is [`RustAnalyzerPruner`], whose fast path is an
//! **in-process `syn` parse**, not a `cargo` subprocess. This POC measures the
//! real per-fix L2 cost, the real imagination-gate cost, and then computes the
//! Amdahl ceiling on the achievable speedup.

use std::time::Instant;

use katgpt_core::conformal::{
    ConformalIntervalCalibrator, DecayUnit, PointForecaster, PredictiveInterval, ResidualMode,
};
use katgpt_core::poincare::{FitConfig, fit_poincare_adapter, poincare_navigate_into};

// ─────────────────────────────────────────────────────────────────────────
// Deterministic PRNG — no `rand` dep, reproducible across runs/machines.
// ─────────────────────────────────────────────────────────────────────────

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    /// Uniform in `[0, 1)`.
    fn f32(&mut self) -> f32 {
        // Numerical Recipes LCG constants.
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / (16_777_216.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Q1 — conformal warm-up curve for a per-rule keyed calibrator.
// ─────────────────────────────────────────────────────────────────────────

/// Stand-in for Plan 005 T9.1's `FixForecaster`: maps a drafted fix to a
/// predicted-correctness score. The POC supplies the point forecast directly
/// via `interval_from_point_into`, so this impl is only here to satisfy the
/// type bound — exactly the shape T9.1 describes ("wrap `TernaryDrafter`'s
/// existing `confidence` output as the point forecast").
struct FixForecaster;

impl PointForecaster for FixForecaster {
    fn forecast_into(&mut self, _delay_state: &[f32], _h: usize, out: &mut f32) {
        *out = 0.0;
    }
}

/// One synthetic `(drafter_confidence, verified_outcome)` observation.
///
/// The generative model is deliberately *favourable* to the calibrator: the
/// drafter is roughly calibrated (a fix with confidence `c` really does verify
/// with probability `c`), so any coverage shortfall is a small-sample artifact,
/// not model misspecification. A miscalibrated drafter would need MORE
/// residuals, not fewer — this sweep is a lower bound on the warm-up cost.
fn draw_observation(rng: &mut Lcg) -> (f32, f32) {
    let confidence = 0.55 + 0.4 * rng.f32();
    let outcome = if rng.f32() < confidence { 1.0 } else { 0.0 };
    (confidence, outcome)
}

fn new_calibrator() -> ConformalIntervalCalibrator<FixForecaster> {
    ConformalIntervalCalibrator::new(
        FixForecaster,
        /* n_channels */ 1, // one channel = one rule key (T9.2's per-rule pool)
        /* max_h */ 1, // fix correctness is a horizon-1 question
        /* m */ 1, // no seasonality in a fix stream
        /* capacity */ 64, // T9.2's stated ring capacity
        /* exp_lambda */ 0.0, // no recency decay: equal weight, the friendliest case
        DecayUnit::Step,
        ResidualMode::Paper,
        /* orientation */ false,
    )
}

/// Measure empirical 95%-interval coverage after warming the pool with `n`
/// residuals. Returns `(coverage, mean_width)`.
fn coverage_after_warmup(n_warmup: usize, seed: u64) -> (f32, f32) {
    const ALPHA: f32 = 0.05; // 95% interval
    const N_EVAL: usize = 4000;

    let mut cal = new_calibrator();
    let mut rng = Lcg::new(seed);

    for _ in 0..n_warmup {
        let (confidence, outcome) = draw_observation(&mut rng);
        cal.update_residual(outcome, confidence, 0, 1);
        cal.step();
    }

    let mut covered = 0usize;
    let mut width_sum = 0.0_f32;
    let mut interval = PredictiveInterval {
        lower: 0.0,
        point: 0.0,
        upper: 0.0,
        alpha: ALPHA,
    };

    for _ in 0..N_EVAL {
        let (confidence, outcome) = draw_observation(&mut rng);
        cal.interval_from_point_into(confidence, 0, 1, ALPHA, &mut interval);
        if outcome >= interval.lower && outcome <= interval.upper {
            covered += 1;
        }
        width_sum += interval.upper - interval.lower;
    }

    (covered as f32 / N_EVAL as f32, width_sum / N_EVAL as f32)
}

#[test]
fn poc023_q1_conformal_warmup_curve() {
    println!("\n=== POC 023 Q1 — per-rule conformal warm-up (95% interval) ===");
    println!("  gate: T9.5 requires empirical coverage >= 90%");
    println!(
        "{:>8}  {:>10}  {:>10}  {:>8}",
        "n_resid", "coverage", "mean_width", "gate"
    );

    // Averaged over seeds so a single unlucky warm-up draw can't decide the
    // answer (`rerun-numeric-gates-three-times` discipline).
    const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];
    let sweep = [0usize, 1, 2, 4, 8, 16, 19, 24, 32, 48, 64];

    let mut first_pass_n: Option<usize> = None;
    let mut coverage_at_zero = 0.0_f32;

    for n in sweep {
        let (mut cov_sum, mut wid_sum) = (0.0_f32, 0.0_f32);
        for seed in SEEDS {
            let (c, w) = coverage_after_warmup(n, seed);
            cov_sum += c;
            wid_sum += w;
        }
        let coverage = cov_sum / SEEDS.len() as f32;
        let width = wid_sum / SEEDS.len() as f32;
        let pass = coverage >= 0.90;
        if n == 0 {
            coverage_at_zero = coverage;
        }
        if pass && first_pass_n.is_none() {
            first_pass_n = Some(n);
        }
        println!(
            "{n:>8}  {:>9.1}%  {width:>10.4}  {:>8}",
            coverage * 100.0,
            if pass { "PASS" } else { "FAIL" }
        );
    }

    match first_pass_n {
        Some(n) => {
            println!("\n  → warm-up threshold: {n} residuals before the T9.5 gate is reachable")
        }
        None => println!("\n  → gate NEVER reached within a 64-slot pool"),
    }

    // The load-bearing finding: a cold pool does not produce a wide,
    // conservative interval — it produces a ZERO-WIDTH one that silently
    // claims certainty. This is the failure mode the plan must guard against.
    let (cold_cov, cold_width) = coverage_after_warmup(0, 7);
    println!(
        "  → cold pool (n=0): width={cold_width:.6}, coverage={:.1}%",
        cold_cov * 100.0
    );
    assert!(
        cold_width < 1e-6,
        "expected a degenerate zero-width interval on an empty pool, got {cold_width}"
    );
    assert!(
        coverage_at_zero < 0.90,
        "a cold pool must NOT pass the T9.5 coverage gate (got {coverage_at_zero})"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Q2 — is L2 really the bottleneck the imagination gate would relieve?
// ─────────────────────────────────────────────────────────────────────────

/// Real spans to validate — the same shape `--fix` feeds the L2 pruner:
/// a rewritten function body, not a whole file.
const SPANS: &[&str] = &[
    r#"fn sum_evens(v: &[u32]) -> u32 { v.iter().filter(|x| **x % 2 == 0).sum() }"#,
    r#"fn build(n: usize) -> Vec<String> {
        let mut out = Vec::with_capacity(n);
        for i in 0..n { out.push(format!("item-{i}")); }
        out
    }"#,
    r#"fn lookup(map: &mut std::collections::HashMap<String, u32>, k: &str, v: u32) {
        map.entry(k.to_string()).or_insert(v);
    }"#,
    r#"fn classify(x: Option<u32>) -> u32 {
        match x { Some(v) => v * 2, None => 0 }
    }"#,
    r#"fn normalize(rows: &[Vec<f32>]) -> Vec<f32> {
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let total: f32 = r.iter().sum();
            out.push(if total > 0.0 { total } else { 1.0 });
        }
        out
    }"#,
];

/// Median of a sample set, in nanoseconds.
fn median_ns(mut samples: Vec<u128>) -> u128 {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

#[test]
fn poc023_q2_l2_vs_imagination_latency() {
    println!("\n=== POC 023 Q2 — L2 cost vs imagination-gate cost ===");

    // ── Measure L2: the default RustAnalyzerPruner fast path is a `syn` parse.
    const REPS: usize = 200;
    let mut l2_samples = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let t0 = Instant::now();
        for span in SPANS {
            // Same call the pruner's fast path makes. The `to_string` models
            // the pruner's owned-String handoff, so this OVERSTATES L2 cost
            // slightly — which makes the Q2 conclusion conservative (i.e. it
            // is generous to the Phase 11 latency claim we are testing).
            let owned = span.to_string();
            let parsed = syn::parse_file(&owned);
            std::hint::black_box(&parsed);
        }
        l2_samples.push(t0.elapsed().as_nanos());
    }
    let l2_ns_per_fix = median_ns(l2_samples) / SPANS.len() as u128;

    // ── Measure the imagination gate: fit an 8→8 adapter, then time one
    //    `poincare_navigate_into` + the goodness dot product (T11.3).
    let mut rng = Lcg::new(42);
    let n_fit = 64usize;
    let z_rows: Vec<Vec<f32>> = (0..n_fit)
        .map(|_| (0..8).map(|_| rng.f32() * 2.0 - 1.0).collect())
        .collect();
    let t_rows: Vec<Vec<f32>> = (0..n_fit)
        .map(|_| (0..8).map(|_| rng.f32() * 2.0 - 1.0).collect())
        .collect();
    let z_refs: Vec<&[f32]> = z_rows.iter().map(Vec::as_slice).collect();
    let t_refs: Vec<&[f32]> = t_rows.iter().map(Vec::as_slice).collect();

    let adapter = fit_poincare_adapter(&z_refs, &t_refs, 8, 8, 8, 8, &FitConfig::default())
        .expect("8→8 adapter fit should succeed on well-conditioned synthetic samples");

    let z_src = [0.1_f32, -0.2, 0.3, 0.05, -0.4, 0.15, 0.2, -0.1];
    let delta = [0.02_f32, 0.01, -0.03, 0.04, 0.0, -0.01, 0.02, 0.03];
    let goodness_dir = [0.35_f32, 0.35, 0.35, 0.35, 0.35, 0.35, 0.35, 0.35];
    let mut z_out = [0.0_f32; 8];
    let mut phi_scratch = vec![0.0_f32; adapter.phi_out()];
    let mut hidden_scratch = vec![0.0_f32; adapter.phi_hidden()];

    let mut imag_samples = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let t0 = Instant::now();
        for _ in 0..SPANS.len() {
            poincare_navigate_into(
                &z_src,
                &delta,
                &adapter,
                &mut z_out,
                &mut phi_scratch,
                &mut hidden_scratch,
            );
            // T11.3's `score_fix_goodness`: dot + sigmoid.
            let dot: f32 = z_out
                .iter()
                .zip(goodness_dir.iter())
                .map(|(a, b)| a * b)
                .sum();
            let score = 1.0 / (1.0 + (-dot).exp());
            std::hint::black_box(score);
        }
        imag_samples.push(t0.elapsed().as_nanos());
    }
    let imag_ns_per_fix = median_ns(imag_samples) / SPANS.len() as u128;

    // ── Bench 018's measured retrieval cost, the third term in the pipeline.
    const RETRIEVE_NS_PER_CHUNK: u128 = 349_000; // 349 µs, Bench 018 G2

    println!("  L2 (syn parse, per fix)        : {l2_ns_per_fix:>10} ns");
    println!("  imagination gate (per fix)     : {imag_ns_per_fix:>10} ns");
    println!("  retrieval (Bench 018, per chunk): {RETRIEVE_NS_PER_CHUNK:>10} ns");

    // ── Amdahl ceiling. Imagination replaces an L2 call only on the fixes it
    //    REJECTS: saved = base_rate · recall · L2. It costs `imag` on EVERY fix.
    //
    //    speedup = T_base / (T_base − b·r·L2 + imag)
    //
    //    T11.6 gates at ≥1.5×; T11.5 claims ≥20% rejection suffices.
    let t_base = (RETRIEVE_NS_PER_CHUNK + l2_ns_per_fix) as f64;
    println!("\n  Amdahl ceiling on end-to-end speedup (T11.6 gate = 1.50x):");
    println!(
        "{:>12}  {:>10}  {:>10}",
        "bad-fix rate", "recall", "speedup"
    );
    let mut best = 0.0_f64;
    for base_rate in [0.05_f64, 0.15, 0.30, 0.50, 1.00] {
        for recall in [0.30_f64, 1.00] {
            let saved = base_rate * recall * l2_ns_per_fix as f64;
            let speedup = t_base / (t_base - saved + imag_ns_per_fix as f64);
            best = best.max(speedup);
            println!("{base_rate:>12.2}  {recall:>10.2}  {speedup:>9.4}x");
        }
    }
    println!(
        "\n  → best achievable speedup, even at a 100% bad-fix rate and 100% recall: {best:.4}x"
    );

    // The load-bearing finding: L2 is not ms-scale, so it cannot be the
    // bottleneck the T11.6 latency gate assumes. BOX-INDEPENDENT, so it is
    // the only asserted half: the ms-scale premise fails long before 1 ms
    // on every host measured (83 µs M3, 256 µs ubuntu-4core).
    assert!(
        l2_ns_per_fix < 1_000_000,
        "T11.5 premise says L2 is ms-scale; measured {l2_ns_per_fix} ns"
    );
    // The Amdahl ceiling is NOT asserted: `best` depends on the L2/imagination
    // ratio RELATIVE to the fixed 349 µs retrieval constant, so a box whose
    // syn-parse is ~3x slower than the authoring M3 (the first linux run,
    // 2026-09-03: L2 255.6 µs, imag 9.0 µs) arithmetically reaches 1.69x —
    // the saved FRACTION of t_base grows with L2 while retrieval stays fixed.
    // The 1.5x finding is a property of the authoring box's timing mix, not an
    // invariant; the measured table above is the record per run.
    println!("\n  verdict: recorded per-box (see the table); asserted only L2 < 1 ms");
}
