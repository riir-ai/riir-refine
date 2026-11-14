//! Seed corpus — the six ANE host-orchestration rules (Research 409 R1–R6,
//! two-source convergences kept; Plan 377 / the owner's 2026-11-14 call).
//!
//! Each rule pairs a **reference shape** (the stateful / mis-placed /
//! overflow-prone host pattern) with a **fixed shape** (the stateless /
//! placed / guarded pattern). The patterns are Rust-shaped pseudocode of the
//! HOST side (the CoreML/ANE graph and its placement config), so the shared
//! `RustAnalyzerPruner` can validate them in the fast path — the ANE
//! *concept* is encoded in the structure and the `notes` decision content,
//! not in `CoreML` tooling syntax. Every rule carries its source provenance
//! and regime condition in `notes` (the `context_notes` channel) and is
//! disclosed `NotBenchable-by-domain` — the win metric lives in the
//! CoreML/ANE runtime.
//!
//! Sources (quotes re-verified at the pins in Research 409):
//! - `shershah1024/liquidcodec-ane` @ `177f90246605f60d10df0372396acc3f5222d597`
//!   (LFM Open License v1.0; NOTICE transcribes Liquid AI attribution +
//!   CC-BY-4.0 samples).
//! - `beshkenadze/nemotron-3.5-asr-streaming-0.6b-coreml-ane-stream` README
//!   @ `3dabd79` (the second source — caches-in/caches-out convergence).

// Uniform `r#"..."#` raw-string style is deliberate for the data table —
// several shapes embed double quotes in their comments (the eval_fixtures
// convention; the allow is the shape, not a lint dodge).
#![allow(clippy::needless_raw_string_hashes)]

use crate::corpus::Corpus;
use crate::domains::ane::{AneDomain, AneFamily, AneRule};

/// The liquidcodec ANE provenance every rule cites (one constant, one line
/// per rule's notes — never a per-rule copy that can drift).
const LC_ANE: &str = "liquidcodec-ane @ 177f902 (lc_ane.py)";
const BESH: &str = "beshkenadze/nemotron-3.5-asr-streaming-0.6b-coreml-ane-stream README @ 3dabd79";
const ANNOTATION: &str = "suggestion-only, NotBenchable-by-domain (the win metric lives in the CoreML/ANE runtime; no validator on this box hosts it) — measure on the target host before adopting";

#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic
pub fn build_ane_seed_corpus() -> Corpus<AneDomain> {
    let entries = vec![
        // ── R1 (B+ LEAD) — two-source convergence ────────────────────────────
        AneRule {
            rule_id: "ane-fixed-window-mask-stateless-streaming".to_string(),
            ane_concept: "fixed-shape stateless windowing replaces stateful streaming caches"
                .to_string(),
            trigger: "a streaming host that carries decoder/conv state across chunks \
                      (an MLState handle, a cache ring, KV-style state) where the model's \
                      receptive field fits a fixed window"
                .to_string(),
            buggy_code: r#"// Stateful streaming: every chunk re-opens the cached state; the
// conversion must carry MLState in/out and the runtime pays the stateful path.
struct StreamDecoder {
    cache: Vec<f32>,
}

impl StreamDecoder {
    fn next_chunk(&mut self, chunk: &[f32]) -> Vec<f32> {
        self.cache.extend_from_slice(chunk);
        decode(&self.cache)
    }
}

fn decode(window: &[f32]) -> Vec<f32> {
    window.iter().map(|x| x * 2.0).collect()
}"#
            .to_string(),
            fixed_code: r#"// Stateless fixed window: the receptive field is bounded by the window,
// so the graph needs NO state — chunks are identical to the full read
// (the chunked ≡ full property the conversion proves once).
const WINDOW: usize = 120;

const fn receptive_field() -> usize {
    // conv(5, d=6) + dilated stack (3, d=29) — must fit the window, or the
    // stateless conversion is REFUSED (the admission gate).
    5 * 6 + 3 * 29
}

fn window_chunk(frames: &[f32], start: usize) -> Vec<f32> {
    let field = receptive_field();
    let mut window = vec![0.0f32; WINDOW];
    let valid = WINDOW.min(frames.len().saturating_sub(start));
    window[..valid].copy_from_slice(&frames[start..start + valid]);
    // The validity mask zeroes conv inputs and masks attention keys, so the
    // zero-padded tail contributes nothing — chunked == full by construction.
    mask_tail(&mut window, valid);
    decode(&window)
}

const fn admission_gate() -> bool {
    receptive_field() <= WINDOW
}

fn mask_tail(window: &mut [f32], valid: usize) {
    for slot in window.iter_mut().skip(valid) {
        *slot = 0.0;
    }
}

fn decode(window: &[f32]) -> Vec<f32> {
    window.iter().map(|x| x * 2.0).collect()
}"#
            .to_string(),
            family: AneFamily::Streaming,
            notes: Some(format!(
                "ADMISSION GATE is the decision content: receptive field <= window \
                 (liquidcodec: \"receptive field is 5*6 + 3*29 = 117 frames < 120\"); the validity \
                 mask zeroes conv inputs + masks attention keys; chunked == full proven by a check \
                 whose negative control fails by design (chunks without left context differ 3.2). \
                 TWO-SOURCE convergence: {BESH} does the same with caches as explicit in/out \
                 (\"no MLState\"). KINSHIP: pad-tail-to-fixed-tile (gpu_08) owns tail-padding — THIS \
                 owns whole-graph windowing replacing the stateful path; cite both. Regime: CoreML/ANE \
                 streaming hosts. {ANNOTATION}. Sources: {LC_ANE}; {BESH}."
            )),
        },
        // ── R2 (B) — the overflow pole of the pow2 prescale family ──────────
        AneRule {
            rule_id: "ane-fp16-norm-pow2-prescale-overflow".to_string(),
            ane_concept: "power-of-2 prescale keeps fp16 norm statistics from overflowing"
                .to_string(),
            trigger: "an fp16 mean-of-squares / RMS norm whose squared accumulator can \
                      overflow half precision (residual streams reach thousands while fp16 \
                      maxes at 65,504 and x^2 quadruples the exponent headroom)"
                .to_string(),
            buggy_code: r#"// fp16 norm on a hot residual stream: x^2 overflows half precision
// before the mean lands (x ~ 2300 on the residual; 2300^2 over fp16 max).
fn rms_norm_fp16(x: &[f32]) -> Vec<f32> {
    let sum_sq: f32 = x.iter().map(|v| v * v).sum();
    let inv = 1.0 / sum_sq.sqrt();
    x.iter().map(|v| v * inv).collect()
}"#
            .to_string(),
            fixed_code: r#"// Power-of-2 prescale: scale each norm so the scaled max |x| <= 32
// (x^2 mean stays far below the fp16 max); the prescale is a power of two,
// so the un-scale is EXACT (no mantissa change, only the exponent).
const PRESCALE_SHIFT: i32 = 6; // 2^6 = 64: max |x| 2048 -> 32; (32)^2 = 1024 << 65,504

fn rms_norm_fp16_prescaled(x: &[f32]) -> Vec<f32> {
    let scale = f32::from(2u8).powi(PRESCALE_SHIFT);
    let scaled: Vec<f32> = x.iter().map(|v| v / scale).collect();
    let sum_sq: f32 = scaled.iter().map(|v| v * v).sum();
    let inv = 1.0 / sum_sq.sqrt();
    // The norm result is scale-invariant: no un-scale needed on the output.
    scaled.iter().map(|v| v * inv).collect()
}"#
            .to_string(),
            family: AneFamily::Numerics,
            notes: Some(format!(
                "Regime: fp16/ANE-class arithmetic on norm statistics — never needed on f32 hosts. \
                 Source quote: \"prescale each norm so the scaled max |x| is <= 32 (x^2 mean stays far \
                 below fp16 max)\" — the residual stream reaches ~2,300 vs fp16 max 65,504. KINSHIP: \
                 the prescale family (B242 masked-in-place, hi-lo-split-pow2) owns the SUBNORMAL-FLOOR \
                 pole on GEMM operands — THIS owns the OVERFLOW pole on norm statistics. Distinct pole \
                 of the same pow2 law; merge refused. {ANNOTATION}. Source: {LC_ANE} L40+L237."
            )),
        },
        // ── R3 (B−) — host placement ─────────────────────────────────────────
        AneRule {
            rule_id: "ane-compute-plan-pin-cpu-and-neural-engine".to_string(),
            ane_concept: "pin compute units to CPU+ANE, never .all — placement decides residency"
                .to_string(),
            trigger: "a CoreML host whose compute-unit policy is left at `.all` (or unset) \
                      while the model is meant to run on the Neural Engine"
                .to_string(),
            buggy_code: r#"// `.all` lets the runtime place the whole graph wherever it likes —
// in practice the GPU wins and the ANE idles.
fn load_model(path: &str) -> LoadedModel {
    let config = ComputeUnits::All;
    load_with(path, config)
}

struct LoadedModel;
fn load_with(_path: &str, _units: ComputeUnits) -> LoadedModel {
    LoadedModel
}

enum ComputeUnits {
    All,
    CpuAndNeuralEngine,
}"#
            .to_string(),
            fixed_code: r#"// cpuAndNeuralEngine keeps 100% cost-weighted ANE residency; the int32
// mask ops drop to CPU as a negligible island — placement islands are
// acceptable, a whole-graph GPU demotion is not.
fn load_model(path: &str) -> LoadedModel {
    let config = ComputeUnits::CpuAndNeuralEngine;
    load_with(path, config)
}

struct LoadedModel;
fn load_with(_path: &str, _units: ComputeUnits) -> LoadedModel {
    LoadedModel
}

enum ComputeUnits {
    All,
    CpuAndNeuralEngine,
}"#
            .to_string(),
            family: AneFamily::Placement,
            notes: Some(format!(
                "Regime: CoreML/ANE hosts. Source quotes: \"with .all, CoreML places the whole graph on \
                 the GPU (~2% ANE)\"; \".cpuAndNeuralEngine keeps 100% cost-weighted ANE residency\"; \
                 \"int32 mask ops drop to CPU as one negligible island\" — placement islands are \
                 acceptable. Zero prior corpus owners for compute-unit placement. {ANNOTATION}. \
                 Source: {BESH}."
            )),
        },
        // ── R4 (B−) — graph shape decides lowering ───────────────────────────
        AneRule {
            rule_id: "ane-graph-restructure-for-ane-lowering".to_string(),
            ane_concept: "restructure the graph so ops lower onto the ANE (fold projections, 1x1 convs)"
                .to_string(),
            trigger: "a converted model whose op census shows most ops falling back to the CPU \
                      (the negative control: post-training int4 left 386 of 464 ops on the CPU)"
                .to_string(),
            buggy_code: r#"// Separated projections + linear-as-matmul ops: shapes the ANE lowering
// cannot fuse, so the ops fall back to the CPU.
fn projections(x: &[f32], w_q: &[f32], w_k: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let q = matmul(x, w_q);
    let k = matmul(x, w_k);
    (q, k)
}

fn matmul(x: &[f32], w: &[f32]) -> Vec<f32> {
    x.iter().map(|v| v * w[0]).collect()
}"#
            .to_string(),
            fixed_code: r#"// One folded projection + 1x1 convs instead of linears: the graph shape
// the ANE lowers whole. (The measured negative stands as the regime
// evidence: plain post-training int4 "dropped to 9.8 dB and left the ANE
// (386 of 464 ops on the CPU)" — restructure, don't just quantize.)
fn projections_folded(x: &[f32], w_qk: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let half = w_qk.len() / 2;
    let q = conv1x1(x, &w_qk[..half]);
    let k = conv1x1(x, &w_qk[half..]);
    (q, k)
}

fn conv1x1(x: &[f32], w: &[f32]) -> Vec<f32> {
    x.iter().map(|v| v * w[0]).collect()
}"#
            .to_string(),
            family: AneFamily::GraphShape,
            notes: Some(format!(
                "Source quotes: \"projections folded into one 48->512 linear; 1x1 convs instead of \
                 linears\"; the NEGATIVE is the regime evidence — plain post-training int4 \"dropped to \
                 9.8 dB and left the ANE (386 of 464 ops on the CPU)\". Adjacent to \
                 one-program-many-functions-accelerator-service-memory (residency axis) — THIS owns \
                 the lowering axis; distinct. {ANNOTATION}. Source: {LC_ANE} L5+L52."
            )),
        },
        // ── R5 (B−) — precision-critical DSP edge ────────────────────────────
        AneRule {
            rule_id: "ane-precision-dsp-stays-cpu-fp32".to_string(),
            ane_concept: "keep the STFT/iFFT tail on CPU fp32; the learned stack takes the ANE"
                .to_string(),
            trigger: "a DSP head/tail (complex exp, iFFT, overlap-add) converted into the ANE \
                      package where fp16 phase error compounds"
                .to_string(),
            buggy_code: r#"// The whole pipeline — STFT head included — rides the fp16 graph; the
// complex exponentials lose phase precision at half width.
fn pipeline_fp16(samples: &[f32]) -> Vec<f32> {
    let spec = stft_fp16(samples);
    decode_fp16(&spec)
}

fn stft_fp16(_samples: &[f32]) -> Vec<f32> {
    Vec::new()
}

fn decode_fp16(spec: &[f32]) -> Vec<f32> {
    spec.to_vec()
}"#
            .to_string(),
            fixed_code: r#"// The STFT head runs OUTSIDE the package (CPU, fp32); the learned stack
// runs on the ANE. Complex exp / iFFT / overlap-add stay exact where they
// are precision-critical.
fn pipeline_split(samples: &[f32]) -> Vec<f32> {
    let spec = stft_fp32_cpu(samples);
    let frames = decode_ane(&spec);
    overlap_add_fp32_cpu(&frames)
}

fn stft_fp32_cpu(_samples: &[f32]) -> Vec<f32> {
    Vec::new()
}

fn decode_ane(spec: &[f32]) -> Vec<f32> {
    spec.to_vec()
}

fn overlap_add_fp32_cpu(frames: &[f32]) -> Vec<f32> {
    frames.to_vec()
}"#
            .to_string(),
            family: AneFamily::PrecisionDsp,
            notes: Some(format!(
                "Regime: precision-critical DSP at graph edges. Source quote: \"the STFT head runs \
                 outside the package (CPU, fp32)\" — complex exp / iFFT / overlap-add tail stays CPU \
                 fp32 for precision while the learned stack runs ANE. {ANNOTATION}. Source: {LC_ANE} \
                 L6 + the tail."
            )),
        },
        // ── R6 (B−) — palette depth retention gate ───────────────────────────
        AneRule {
            rule_id: "ane-weight-palette-depth-retention-gate".to_string(),
            ane_concept: "palette depth is a measured retention gate, never a published verdict"
                .to_string(),
            trigger: "adopting a palettization depth because a published card claims it neutral \
                      (8-bit palettized won +28% word-identical on one model; 6-bit degraded the \
                      transcript there, while another shipper claims 6-bit WER-neutral)"
                .to_string(),
            buggy_code: r#"// Adopt the depth a README claims: 6-bit palettized "WER-neutral" —
// measured elsewhere, transferred here unverified.
fn pack_weights_6bit(weights: &[f32]) -> Vec<u8> {
    palette_quantize(weights, 6)
}

fn palette_quantize(_w: &[f32], bits: u8) -> Vec<u8> {
    vec![bits]
}"#
            .to_string(),
            fixed_code: r#"// Measure retention per posture; adopt only a depth whose transcript
// retention on THIS model is neutral (int8 weight-only = size-not-speed on
// ANE: 316x vs 320x fp16; 8-bit won +28% word-identical once; 6-bit
// degraded there AND was claimed neutral elsewhere — method-dependent).
fn pack_weights_measured(weights: &[f32], depth: u8, retention: f64) -> Option<Vec<u8>> {
    if retention_gate(retention) {
        Some(palette_quantize(weights, depth))
    } else {
        None
    }
}

const fn retention_gate(retention: f64) -> bool {
    retention >= 1.0
}

fn palette_quantize(_w: &[f32], bits: u8) -> Vec<u8> {
    vec![bits]
}"#
            .to_string(),
            family: AneFamily::PaletteQuant,
            notes: Some(format!(
                "MEASURED TWICE INDEPENDENTLY: int8 weight-only = size-not-speed on ANE (316x vs 320x \
                 fp16, {LC_ANE}); 8-bit palettized \"+28% faster ... word-for-word identical\" while \
                 their 6-bit \"degraded the transcript\" ({BESH}); liquidcodec 4/6-bit k-means gave \
                 4.4/16.6 dB. REGIME-CONDITION DISCREPANCY RECORDED HONESTLY: FluidAudio ships 6-bit \
                 palettized encoder layers claiming WER-neutral — palette tolerance is METHOD-dependent \
                 (group size/quantizer differ); the rule lands as \"measure retention per posture, \
                 never assume either published verdict transfers\". {ANNOTATION}."
            )),
        },
    ]
    .into_iter()
    .collect();

    Corpus::new(entries)
}

#[cfg(test)]
mod tests;
