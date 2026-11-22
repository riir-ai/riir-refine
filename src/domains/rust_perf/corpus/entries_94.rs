// Batch 399 — the mlx-serve ANE seed's `rust_perf` half (Plan 399 /
// Research 432 §Queue slice 2, ddalcu/mlx-serve @ 96311b26, MIT; the
// controlling ANE half lands in src/domains/ane/corpus.rs in the same
// commit — the Batch-398 structure mirrored).
//
// rust_perf 387 → 389: two measurement-honesty transcription rules from the
// share-table section and its own dated correction:
// - `calibration-probe-swept-band-single-point-outlier` — a calibration
//   INPUT is a swept band, never one probe point (their own 2026-09-11
//   correction: the M4 Max ACE row's G = 7.8 TFLOPS was an outlier probe;
//   swept at 256-4096 rows the same box reads 11.2-13.3 TFLOPS, which
//   re-lands the solve at ~0.47 for all three models and does NOT
//   reproduce the 0.60 optimum the outlier implied). Carries the dtype
//   face (the complement timed in the dtype the stream really sees:
//   bf16 probes read ACE's f32 GPU 27% fast) and the
//   judge-after-cold-compile face.
// - `metric-after-normalizer-measures-the-normalizer` — a metric computed
//   downstream of a global normalizer measures the normalizer's gain, not
//   the signal (the ACE-Step ±1 dB sign-flipping "loudness lift").
//
// Fixtures land in src/domains/rust_perf/corpus/tests.rs (the shape arms).
// NOTHING INVENTED — every quote read at the clone this session.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 399 — the mlx-serve ANE seed's `rust_perf` half.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b399_rules() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "calibration-probe-swept-band-single-point-outlier".to_string(),
            name: "A calibration input is a SWEPT BAND, never one probe point: a single-point \
                   throughput probe can sit an outlier away from the truth (one probe read 7.8 \
                   TFLOPS where a 256-4096-row sweep on the same box reads 11.2-13.3), and the \
                   swept probe is quoted with its row range AND the dtype the workload's stream \
                   really uses — a calibrator that names its dtype"
                .to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"/// NAIVE: the calibration times the GPU side ONCE at one fixed shape
/// and feeds the number straight into the split solve. One probe read
/// 7.8 TFLOPS — an outlier — so the solve landed at 0.60 where the real
/// optimum is ~0.47; and the probe ran in bf16 while the workload's
/// stream is f32, which alone read 27% fast.
fn probe_gpu_tflops() -> f64 {
    // one point, one dtype, no range — the whole calibration rides this
    time_matmul_tops(/* rows */ 4096, /* dtype */ DType::Bf16)
}

fn solve_split(a_tflops: f64, g_tflops: f64) -> f64 {
    a_tflops / (a_tflops + g_tflops)
}

enum DType {
    Bf16,
    F32,
}

fn time_matmul_tops(_rows: usize, _dtype: DType) -> f64 {
    7.8 // the outlier, consumed as truth
}"#
            .to_string(),
            fixed_code: r#"/// FIXED: the probe is a SWEEP over the workload's row range at the
/// dtype the stream really uses; the calibration consumes the swept
/// band (a plausible spread, not a point), and the solve's verdict is
/// re-judged in a clean boot (the requests right after a cold compile
/// run 3-5% slow for a while — a calibration read there is tainted).
const PROBE_ROWS: std::ops::Range<usize> = 256..4096;

struct SweptBand {
    min_tflops: f64,
    max_tflops: f64,
    dtype: &'static str, // the stream's real dtype, named by the calibrator
}

fn probe_gpu_band(dtype: &'static str) -> SweptBand {
    let mut readings = PROBE_ROWS
        .clone()
        .step_by(256)
        .map(|rows| time_matmul_tops(rows, dtype))
        .collect::<Vec<_>>();
    readings.sort_by(|a, b| a.total_cmp(b));
    SweptBand {
        // the band, not the point: one row's outlier cannot steer the solve
        min_tflops: *readings.first().expect("sweep non-empty"),
        max_tflops: *readings.last().expect("sweep non-empty"),
        dtype,
    }
}

fn solve_split_from_band(a_tflops: f64, band: &SweptBand) -> f64 {
    // consume the band's midpoint as G; the answer is only a SEED for a
    // once-per-model calibration anyway (the measured-table law)
    let g = (band.min_tflops + band.max_tflops) / 2.0;
    a_tflops / (a_tflops + g)
}

enum DType {
    Bf16,
    F32,
}

fn time_matmul_tops(_rows: usize, _dtype: &'static str) -> f64 {
    11.2 // the swept floor, honest
}"#
            .to_string(),
            context_notes: Some(
                "THE MEASURED CLASS, the source's OWN correction (2026-09-11), verbatim: \
                 \"the M4 Max ACE row's G (7.8 TFLOPS) was an outlier probe. Swept at 256-4096 \
                 rows the same box reads 11.2-13.3 TFLOPS for ACE, 12.6-13.5 for Krea and \
                 12.9-13.5 for H3, so the solve lands at ~0.47 for all three and does NOT \
                 reproduce ACE's 0.60 optimum\" — a single-point probe steered a split \
                 calibration to a wrong optimum and the correction re-landed THREE models' \
                 solves at once. THE LAW: a number that feeds a calibration (a split ratio, a \
                 budget, a schedule) is a SWEPT BAND over the workload's real range, never one \
                 probe point; the sweep names its row/size range AND its dtype — the probes \
                 that read fast in the wrong dtype are a measured face: \"the GPU fed the dtype \
                 the MLP really sees (bf16 ones read ACE's f32 GPU 27% fast)\" — a 27% skew in \
                 the calibration input moves the optimum materially. POSTURE FACE: \"Judge a \
                 calibrated share in a boot WITHOUT a cold compile: the requests after one run \
                 3-5% slow for a while\" — a calibration read immediately after a cold compile \
                 measures the aftermath, not the steady state. The probe point is also FIXED at \
                 a declared row count for reuse across builds (\"MEDIA_PROBE_ROWS (4096)\") — the \
                 sweep is over the workload's range, at a pinned protocol. DISTINCT-FROM: \
                 benchmark-spread-above-threshold-reports-unstable (B345) owns the STABILITY \
                 VERDICT on measured spread (warm until CV settles, report UNSTABLE past a \
                 spread bar) — THIS moves the sweep UPSTREAM into the calibration INPUT (sweep \
                 before the number is consumed, dtype named, single-point refused as an \
                 outlier class); aa-control-prices-the-fidelity-instrument (B225) prices the \
                 A/A instrument to expose a false-positive-generating fidelity budget — THIS \
                 is the input-sweep half of the same honesty family; \
                 measured-round-cost-spec-controller (kernel, B398) guards a measured TABLE \
                 against contamination (implausible-step/self-spike guards ON the samples) — \
                 THIS refuses the single-point SHAPE of the input itself; they compose (the \
                 table's samples swept + contamination-guarded). NotBenchable-by-model (the \
                 deciding metric is calibration correctness on the real workload — no \
                 validator hosts it; the source's own correction IS the evidence). Regime: any \
                 calibration whose input is a timed probe (split ratios, budgets, schedules, \
                 kernel-choice tables). ANTI-REGIME: probes whose workload range is genuinely \
                 one shape (a fixed-tile engine — sweep anyway to prove the flatness). Source: \
                 ddalcu/mlx-serve @ 96311b26 (docs/reference.md, the \"Media offload share\" \
                 section + its Correction paragraph; every quote byte-verified at the clone \
                 this session)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "metric-after-normalizer-measures-the-normalizer".to_string(),
            name: "A quality metric computed DOWNSTREAM of a global normalizer measures the \
                   normalizer's gain, not the signal — place the metric before the normalizer \
                   or on per-frame normalized statistics; a whole-track scalar after a \
                   track-level normalizer reads a sign-flipping noise artifact as a real lift"
                .to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"/// NAIVE: the comparison pipeline normalizes each track, then reads
/// ONE whole-track RMS as the loudness metric. peakNormalize sets the
/// whole track's gain from ONE sample (the peak), so the metric reads
/// the normalizer's per-seed gain — a +/-1 dB "loudness lift" that
/// flips sign across seeds and measures nothing about the signal.
fn compare_tracks(a: &[f32], b: &[f32]) -> Metric {
    let a_norm = peak_normalize(a);
    let b_norm = peak_normalize(b);
    Metric::WholeTrackRms(rms(a_norm) - rms(b_norm)) // reads the GAIN, not the audio
}

fn peak_normalize(x: &[f32]) -> Vec<f32> {
    let peak = x.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    let g = 1.0 / peak.max(1e-9); // ONE sample steers the whole gain
    x.iter().map(|v| v * g).collect()
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>() / x.len() as f64).sqrt() as f32
}

enum Metric {
    WholeTrackRms(f32),
}"#
            .to_string(),
            fixed_code: r#"/// FIXED: the metric moves INSIDE the normalized domain as per-frame
/// statistics over trimmed frames — frame-level RMS and a log-STFT
/// cosine compare the SIGNAL; the whole-track scalar after the
/// normalizer is never a comparison input.
fn compare_tracks(a: &[f32], b: &[f32], sr: u32) -> Metric {
    let a_frames = trimmed_frames(a, sr);
    let b_frames = trimmed_frames(b, sr);
    let rms_pair = frame_rms_delta(&a_frames, &b_frames);
    let spectral = log_stft_cosine(&a_frames, &b_frames);
    Metric::PerFrame { rms_pair, spectral }
}

struct Frame {
    samples: Vec<f32>,
}

fn trimmed_frames(x: &[f32], _sr: u32) -> Vec<Frame> {
    // trim the silent lead/tail so the peak-normalizer's gain is set by
    // signal frames, not an edge artifact
    vec![Frame { samples: x.to_vec() }]
}

fn frame_rms_delta(_a: &[Frame], _b: &[Frame]) -> f64 {
    0.0
}

fn log_stft_cosine(_a: &[Frame], _b: &[Frame]) -> f64 {
    0.0
}

enum Metric {
    PerFrame { rms_pair: f64, spectral: f64 },
}"#
            .to_string(),
            context_notes: Some(
                "THE MEASURED CLASS, verbatim (the incident): \"ANE vs GPU audio showed a +/-1 dB \
                 RMS 'loudness lift' that changed sign across seeds. Cause: peakNormalize sets \
                 the whole track's gain from one sample. Fix: measure trimmed per-frame RMS and \
                 log-STFT cosine, never a whole-track scalar after a normalizer.\" THE LAW: a \
                 metric computed downstream of a global (track-level) normalizer measures the \
                 NORMALIZER — its gain is set by extreme samples, so the metric inherits the \
                 gain's variance and reads a comparison artifact as a real effect (the \
                 sign-flipping across seeds is the tell: a real property is seed-stable). The \
                 repair moves the metric into the normalized domain as PER-FRAME statistics \
                 over TRIMMED frames (the trim keeps edge artifacts out of the gain), or places \
                 it before the normalizer entirely. DISTINCT-FROM: \
                 benchmark-spread-above-threshold-reports-unstable (B345) verdicts on spread \
                 AFTER measuring — THIS relocates the metric away from a specific upstream \
                 transform; unique-prompts-defeat-prefix-cache-and-cold-warm-reported-separately \
                 (B367) separates populations a shared path would pool — kin in the \
                 measure-what-the-path-actually-did family, different axis (population split vs \
                 metric placement). NotBenchable-by-model (the deciding metric is comparison \
                 correctness against the real signal — no validator hosts it). Regime: any \
                 comparison pipeline downstream of a global normalizer (audio loudness, image \
                 exposure/gain, loss-normalized scores, per-batch whitening). ANTI-REGIME: \
                 metrics that are DESIGNED to read the normalizer (loudness-compliance checks \
                 — but then name the normalizer in the claim). Source: ddalcu/mlx-serve \
                 @ 96311b26 (docs/gotchas/models-media.md, \"A metric taken after a global \
                 normalization measures the normalizer (ACE-Step)\"); every quote byte-verified \
                 at the clone this session."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
