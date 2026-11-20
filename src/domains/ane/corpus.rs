//! Seed corpus — the ANE host-orchestration rules (Research 409 R1–R6 seed
//! + the Research 421 anemll branch-delta wave, Batch 376; the two-source
//!   convergences kept).
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
//! Sources (quotes re-verified at the pins in Research 409 + 421; the 421
//! wave re-verified at the NEW tip `fe8de185d7ff45e5663a21f2ecfd3d9e1f2ffe40`
//! — the batch T0 re-pin, the verdict pin `912925f7` had moved):
//! - `shershah1024/liquidcodec-ane` @ `177f90246605f60d10df0372396acc3f5222d597`
//!   (LFM Open License v1.0; NOTICE transcribes Liquid AI attribution +
//!   CC-BY-4.0 samples).
//! - `beshkenadze/nemotron-3.5-asr-streaming-0.6b-coreml-ane-stream` README
//!   @ `3dabd79` (the second source — caches-in/caches-out convergence).
//! - `Anemll/anemll-forge` @ `fe8de185` branch `v8-gpu-ane-prefill` (MIT;
//!   the V8 hybrid prefill + Core AI port notes + the two-pinned diff
//!   `117ded62..fe8de185` proving the `v8_prefill` docs NEW since the 09-30
//!   verdict pin).

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
const ANEM: &str = "Anemll/anemll-forge @ fe8de185 (v8-gpu-ane-prefill branch; re-pinned at the batch T0 past the 912925f verdict pin)";
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
                 of the same pow2 law; merge refused. FACE (Batch 376, the UNDERFLOW pole of the \
                 prescale itself): a prescale sized for LARGE magnitudes creates subnormals at TINY \
                 ones — anemll's RMSNorm with a /64 pre-scale hit embeddings with rms 0.015, where \
                 (x/64)^2 ~= 5e-8 is fp16-subnormal and Core AI's ANE lowering mis-normalized (qkv \
                 0.70x per-token scalar, cos still 1.0 — invisible to cosine checks; their lesson \
                 verbatim: \"check norms / magnitudes, not only cosines\"). The pole law: a prescale \
                 factor must keep BOTH ends in range — max|x|/s below the format max AND \
                 (min|x|/s)^2 above the subnormal floor; a scale-free spelling (xs = x / max|x|, \
                 squares in [0,1]) sidesteps both poles when a fixed factor cannot. {ANNOTATION}. \
                 Sources: {LC_ANE} L40+L237; the pole: {ANEM} (COREAI_PORT_NOTES.md compiler \
                 differences #2)."
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
                 the lowering axis; distinct. FACE (Batch 376, two second instances — algebraic \
                 rewrite + op substitution both decide placement): (a) the doubling inverse spelled \
                 as chained matmuls of COMPUTED matrices, (I+N)^-1 = (I-N)(I+N^2)(I+N^4), put NO ANE \
                 region in the whole graph (layer 0 ran 131 ms off-ANE) while a single dynamic 8x8 \
                 batched matmul placed fine — the fix is an ALGEBRAICALLY equivalent rewrite (forward \
                 substitution over the 8 rows, elementwise + reduce) that lands fully on the ANE \
                 (1.6 ms standalone): restructure means RE-SPELLING the math, not just reshaping ops. \
                 (b) `index_copy_` KV writes lower to `scatter_nd` and the graph goes to the GPU \
                 (15 ms); the placed design keeps KV read-only inputs with host-committed rows — one \
                 placement-hostile op demotes the whole graph, so the op SUBSTITUTION (what crosses \
                 the boundary, what stays) is part of the same decision. {ANNOTATION}. Sources: \
                 {LC_ANE} L5+L52; the instances: {ANEM} (COREAI_PORT_NOTES.md compiler differences \
                 #1 + #3)."
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
        // ── Batch 376 wave (Research 421, the anemll branch delta) ─────────
        // ── A4 (B, the LEAD) — shape-dependent silent wrongness of INT8 forms ─
        AneRule {
            rule_id: "ane-int8-matmul-shape-silent-wrong-gate".to_string(),
            ane_concept: "an accelerator's quantized matmul is correct only at shapes its lowering \
                          supports — wrong results arrive with no error signal, so every NEW form \
                          (shape, split, fusion) is checked against a host reference before it serves"
                .to_string(),
            trigger: "a new quantized (INT8/FP8-class) accelerator matmul form — a different K \
                      width, a K-split, a fused gate+up pair, a new batch width — about to enter \
                      service on the strength of its measured speed alone"
                .to_string(),
            buggy_code: r#"// The new INT8 form benchs faster, so it is adopted on the speed
// number alone. Two of its shapes compute near-zero (rel err ~1.0)
// and one computes rel err ~1.24 — silently; nothing in the serving
// path compares against the reference.
fn serve_ffn(x: &[f32], rows: usize, form: Form) -> Vec<f32> {
    match form {
        Form::GateUpK5120 => int8_matmul_pair(x, W_GATE, W_UP), // rel 0.98: wrong
        Form::Blocks17x1024NoSplit => int8_matmul(x, W17),      // rel 1.24: wrong
        Form::SplitK => int8_matmul_split_k(x, W17),            // the fast, correct form
    }
}

enum Form { GateUpK5120, Blocks17x1024NoSplit, SplitK }

fn int8_matmul_pair(_x: &[f32], _a: &[i8], _b: &[i8]) -> Vec<f32> {
    Vec::new()
}

fn int8_matmul(_x: &[f32], _w: &[i8]) -> Vec<f32> {
    Vec::new()
}

fn int8_matmul_split_k(_x: &[f32], _w: &[i8]) -> Vec<f32> {
    Vec::new()
}"#
                .to_string(),
            fixed_code: r#"// FIXED: the host-reference gate runs ONCE per new form at the
// shapes it will serve; a form that fails the check is refused
// (or kept behind its proven-good shapes only) — the speed number
// never certifies correctness.
enum Form { GateUpK5120, Blocks17x1024NoSplit, SplitK }

fn host_reference(x: &[f32]) -> Vec<f32> {
    // fp32 ground truth on the same weights
    x.iter().map(|v| v * 2.0).collect()
}

fn rel_err(a: &[f32], b: &[f32]) -> f64 {
    let num: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(p, q)| (p - q).abs() as f64)
        .sum();
    let den: f64 = b.iter().map(|v| v.abs() as f64).sum::<f64>().max(1e-12);
    num / den
}

fn admit_form(form: Form, probe: &[f32]) -> Result<Form, &'static str> {
    let got = serve_form(probe, form);
    if rel_err(&got, &host_reference(probe)) <= 0.01 {
        Ok(form)
    } else {
        // "with nothing to signal it" — the check IS the signal
        Err("quantized form silently wrong at this shape; refuse or K-split")
    }
}

fn serve_form(x: &[f32], form: Form) -> Vec<f32> {
    let _ = (x, form);
    Vec::new()
}"#
                .to_string(),
            family: AneFamily::Validation,
            notes: Some(format!(
                "THE LAW, verbatim at the pin (PREFILL_RESEARCH.md shape probe + lesson 3): \"Gate and \
                 up as separate K=5120 matmuls compute wrong results (rel 0.98) at any width, and so \
                 does 17x1024 without the K split at 1024 rows (rel 1.24), with nothing to signal it. \
                 Always check ANE INT8 output against a host reference.\" / \"Fast ANE INT8 forms can \
                 be silently wrong. Check every new form against a host reference.\" THE MEASURED \
                 ADMISSION: the SAME weights run 13 TOPS at the wrong shape and 48.5 TOPS once the K \
                 dimension is split in two 2560 parts — the wrong form is not slower, it is FASTER \
                 garbage; the speed probe alone cannot certify a form. Distinct from A1 \
                 (ane-stage-parity-gate: whole-model gates blind to per-stage fp16 EXECUTION \
                 wrongness — found by stage taps) — THIS is quantized-FORM shape wrongness found by \
                 a direct host check on every new form; the two regimes compose (a form passes the \
                 host check AND the compiled artifact passes the stage-parity gate). Distinct from \
                 accelerator-backend-first-tile-acceptance-gate (kernel_opt gpu_154): THAT owns \
                 backend ADOPTION (first-real-input, per-machine verdict cache) — THIS owns form/shape \
                 admission INSIDE one adopted backend (every new shape/fusion/split is a new decision; \
                 the check is cheap and per-form, not a cached machine verdict). Regime: any \
                 quantized form admitted to serve on an accelerator whose lowering is shape-sensitive \
                 (INT8/FP8-class matmuls, fused pairs, K-splits, batch-width entries). ANTI-REGIME: \
                 forms already proven at the exact serving shape (re-checking a frozen form per call \
                 is dead weight — the gate runs at FORM ADMISSION, never per call); bit-exact \
                 integer paths with no tolerance question. {ANNOTATION}. Source: {ANEM} \
                 (research/v8_prefill/PREFILL_RESEARCH.md:302-303, :406-407; the probe at \
                 w8a8/a8_ffn_shape_probe.py)."
            )),
        },
        // ── A1 (B− RE-READ, rule form landed) — whole-model gates are blind ──
        AneRule {
            rule_id: "ane-stage-parity-gate-cpu-vs-ane-same-model".to_string(),
            ane_concept: "whole-model gates (cosine, KL, end-to-end NLL) are BLIND to per-stage \
                          accelerator numerics bugs — tap the stages of the ANE program and compare \
                          CPU_ONLY vs CPU_AND_NE of the SAME compiled model, checking magnitudes and \
                          subnormal fractions, not only cosine"
                .to_string(),
            trigger: "an accelerator build whose end-to-end metrics look acceptable (fluent \
                      output, plausible NLL, whole-model cosine above threshold) while a per-stage \
                      numerical defect is suspected, or before trusting a new compiled artifact"
                .to_string(),
            buggy_code: r#"// The whole-model gate passes, so the build ships. Every DeltaNet
// layer is ~50% wrong at the tensor entering out_proj — invisible
// here: the model still generates fluent text and the trace NLL
// even IMPROVED (the copying artifact: memorized context reads well).
fn accept(build: &Build) -> bool {
    let out = build.generate(PROMPT);
    fluent(&out) && cosine(out_probs(build), ref_probs(build)) > 0.99
}

fn fluent(_t: &str) -> bool {
    true
}

fn out_probs(_b: &Build) -> Vec<f32> {
    Vec::new()
}

fn ref_probs(_b: &Build) -> Vec<f32> {
    Vec::new()
}

fn cosine(_a: Vec<f32>, _b: Vec<f32>) -> f32 {
    0.995
}

struct Build;
struct PROMPT;

impl Build {
    fn generate(_p: PROMPT) -> String {
        String::new()
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: the stage-parity gate — same compiled artifact, both
// compute-unit postures, per-stage tensors compared with MAGNITUDE
// checks (subnormal fraction, near-zero absolute error), not only
// cosine. A stage that diverges on one unit refuses the build.
const MAX_SUBNORMAL_FRACTION: f64 = 0.01;
const MAX_STAGE_REL_ERR: f64 = 0.02;

struct StageTap {
    name: &'static str,
    cpu: Vec<f32>,
    ane: Vec<f32>,
}

fn stage_parity(build: &Build) -> Result<(), String> {
    for tap in taps(build) {
        // cosine alone hides a 51% relative error when directions agree
        let rel = rel_l2(&tap.cpu, &tap.ane);
        let subnormals = fraction_subnormal(&tap.ane);
        if rel > MAX_STAGE_REL_ERR || subnormals > MAX_SUBNORMAL_FRACTION {
            return Err(format!(
                "stage {}: rel {rel:.3}, subnormal share {subnormals:.2} — the ANE executes this stage wrong",
                tap.name
            ));
        }
    }
    Ok(())
}

fn taps(_b: &Build) -> Vec<StageTap> {
    Vec::new()
}

fn rel_l2(a: &[f32], b: &[f32]) -> f64 {
    0.0
}

fn fraction_subnormal(_v: &[f32]) -> f64 {
    0.0
}

struct Build;
impl Build {
    fn stages_cpu_only(&self) {}
}"#
                .to_string(),
            family: AneFamily::Validation,
            notes: Some(format!(
                "THE LAW, verbatim at the pin (ANE_DELTANET_NUMERICS.md): \"Neither bug is visible in \
                 cosine-similarity checks of the whole model (the model still generates fluent text), \
                 in the M3U KL runs (PyTorch), or in small test graphs (they run on the CPU). Look at \
                 per-stage tensors of the ANE program, compare CPU_ONLY vs CPU_AND_NE of the SAME \
                 compiled model, and check magnitudes (subnormal fraction) and near-zero activation \
                 accuracy.\" THE MEASURED PREMISE: two fp16 bugs left every DeltaNet layer ~50% wrong \
                 (51% rel-L2 at out_proj input) while end-to-end NLL looked BETTER (2.651 vs the \
                 honest 2.420 — the copying artifact); the per-stage method + the fixes (tanh-form \
                 silu, GDN_SQ=16/GDN_SV=64 prescale) took the stage error 51% -> 1.3% at no \
                 measurable cost (\"the extra elementwise ops disappear in the weight-bandwidth-bound \
                 chunk time\"). RE-READ PROVENANCE (the 09-30 verdict @ 117ded62): the per-stage-tap \
                 method and the fp16 numerics triple were named LANE INTEL for the laya lane there — \
                 what this entry adds is the RULE FORM (the detection-GRANULARITY law as a corpus \
                 gate); the ane corpus did not exist at that verdict. The gate is an \
                 admission/build-time discipline, never per-call. REGIME: any compiled accelerator \
                 artifact whose host reference can run the same graph (CPU_ONLY vs the accelerated \
                 posture of the SAME build); the subnormal-fraction check is the near-zero regime's \
                 detector where cosine cannot see scale loss. ANTI-REGIME: a backend with no \
                 dual-posture execution (nothing to compare — use A4's form-level host check); \
                 bit-exact paths. Distinct from ane-int8-matmul-shape-silent-wrong-gate (A4, this \
                 batch — quantized-FORM shape wrongness, direct host check per new form) and from \
                 accelerator-backend-first-tile-acceptance-gate (kernel_opt gpu_154 — backend \
                 adoption on the first REAL INPUT, per-machine cached verdict): THIS taps \
                 INTERMEDIATE stages of ONE adopted build to find where inside it the execution goes \
                 wrong; compose (adoption gate -> form gate -> stage gate, three granularities of the \
                 same validity law). KINSHIP: the \"check norms/magnitudes, not only cosines\" \
                 discipline is shared with the prescale rule's underflow-pole face (same batch). \
                 {ANNOTATION}. Source: {ANEM} (ANE_DELTANET_NUMERICS.md:46-48, :21, :130-134, :138)."
            )),
        },
        // ── A8 (B−) — pipeline granularity: split across BLOCKS, not layers ──
        AneRule {
            rule_id: "ane-hybrid-pipeline-block-granularity-not-layer-interleave".to_string(),
            ane_concept: "a two-accelerator pipeline must split work at a granularity that keeps \
                          BOTH units busy — split within a layer and the second unit idles through \
                          the other's half; pipeline whole ops per unit across prompt BLOCKS instead"
                .to_string(),
            trigger: "a hybrid host partitioning model work between two accelerators (an ANE and \
                      a GPU) where the split puts part of one layer's work on each unit"
                .to_string(),
            buggy_code: r#"// The FFN's channels are split inside each layer: the ANE takes its
// share, then WAITS while the GPU runs the mixers and the remaining
// quarter — the ANE is busy about 42% of each block.
fn run_layer(x: &Tensor) -> Tensor {
    let ffn_ane = ane_ffn_channels(x, 0.76);
    let ffn_gpu = gpu_ffn_channels(x, 0.24);
    // both units serialize on the mixer
    gpu_mixers(add(ffn_ane, ffn_gpu))
}

fn ane_ffn_channels(_x: &Tensor, _share: f32) -> Tensor {
    Tensor::new()
}

fn gpu_ffn_channels(_x: &Tensor, _share: f32) -> Tensor {
    Tensor::new()
}

fn gpu_mixers(_x: Tensor) -> Tensor {
    Tensor::new()
}

fn add(_a: Tensor, _b: Tensor) -> Tensor {
    Tensor::new()
}

struct Tensor;

impl Tensor {
    fn new() -> Self {
        Tensor
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: whole ops per unit, pipelined across prompt BLOCKS —
// the ANE runs block k's FFN while the GPU mixes block k-1's layer,
// so neither idles. A per-layer comparative-advantage table decides
// WHICH whole op goes where (ANE LUT FFN 23.2us vs GPU 36.3us per
// token-layer; mixers about equal).
fn run_prompt_blocks(blocks: &[Tensor]) -> Vec<Tensor> {
    blocks
        .iter()
        .enumerate()
        .map(|(k, blk)| {
            let ffn = ane_ffn_whole(blk); // 1.56x better on the ANE
            let prev_mixed = if k > 0 {
                gpu_mixers(&blocks[k - 1]) // overlap, not wait
            } else {
                Tensor::new()
            };
            add(ffn, prev_mixed)
        })
        .collect()
}

fn ane_ffn_whole(_x: &Tensor) -> Tensor {
    Tensor::new()
}

fn gpu_mixers(_x: &Tensor) -> Tensor {
    Tensor::new()
}

fn add(_a: Tensor, _b: Tensor) -> Tensor {
    Tensor::new()
}

struct Tensor;

impl Tensor {
    fn new() -> Self {
        Tensor
    }
}"#
                .to_string(),
            family: AneFamily::Placement,
            notes: Some(format!(
                "THE HEADLINE, verbatim at the pin (PREFILL_RESEARCH.md): splash's Neural Engine \
                 \"takes 76% of each FFN's channels inside each layer and waits while the GPU runs \
                 the mixers (busy about 42% of each 2,048-row chunk)\" — hybrid speedup 1.31-1.51x \
                 over the GPU-alone baseline — \"while V8 pipelines prompt blocks so both units stay \
                 busy\" (V8 adds 1.83/2.08/2.02/1.92/1.72/1.60x at 2K..95K vs splash's \
                 1.49/1.51/1.48/1.43/1.35/1.31 over the same baseline). THE ADMISSION RULE'S SHAPE: \
                 the per-layer comparative-advantage table — \"Per token per layer, the ANE LUT FFN \
                 costs 23.2 us against 36.3 on the GPU (1.56x better on the ANE), while the mixers \
                 cost about the same on both\" — WHOLE OP to the unit that wins it, then pipeline \
                 BLOCKS so the dependency chain crosses units once per layer, never mid-op. \
                 Source-own caveat carried: \"their shares are not isolated (no matched phase \
                 profiles or scheduling ablations yet, and the quantizations differ)\" — the \
                 mechanism attribution is their stated hypothesis beside the measured busy fractions, \
                 never a proven decomposition. REGIME: any dual-accelerator host partitioning one \
                 model's ops (ANE+GPU, NPU+GPU-class). ANTI-REGIME: single-accelerator hosts; ops \
                 with no cross-unit dependency chain (independent streams already overlap); units \
                 whose submission overhead exceeds the block compute (pipeline granularity BELOW the \
                 overhead floor re-introduces idle — A9's gap law is the floor). Distinct from \
                 liquidcodec R3/R4 (placement REPORTING + graph restructure for lowering) and from \
                 ane-split-calibration-fit-bandwidth-interference-term (rust_perf entries_74 — a \
                 WITHIN-op channel split calibrated per device; the coarse pipeline decision decides \
                 WHETHER such a within-layer split should exist at all). KINSHIP: the 09-30 record's \
                 hybrid addendum names the binding decision (\"split by LAYER TYPE, not layer range \
                 and not per-tensor\") — lane intel there, the rule lands here. {ANNOTATION}. Source: \
                 {ANEM} (research/v8_prefill/PREFILL_RESEARCH.md:78-84, :164-168)."
            )),
        },
        // ── A9 (B−) — the per-call firmware gap: fewer, bigger predictions ──
        AneRule {
            rule_id: "ane-prediction-gap-fewer-bigger-calls".to_string(),
            ane_concept: "every accelerator evaluation carries a fixed firmware/driver gap no \
                          submission trick hides — the lever is FEWER, BIGGER calls: batch rows \
                          per call up to the measured per-row efficiency cliff, never past it"
                .to_string(),
            trigger: "a serving loop issuing many small accelerator calls whose profiler shows a \
                      fixed idle gap before every evaluation, hidden by neither event chaining nor \
                      any submission trick"
                .to_string(),
            buggy_code: r#"// One 64-row call per block: N blocks, N gaps. The host tries
// shared events and pre-submission to hide the gap; the gap stays
// (driver/firmware-side), and wider 128-row calls measured WORSE per
// token (19%), so the loop sits at the small-call floor.
fn run_blocks(x: &Tensor, n_blocks: usize) -> Vec<Tensor> {
    (0..n_blocks)
        .map(|b| {
            let ev = chain_event(last()); // the trick everyone tries — gap stays
            ane_ffn_64rows(x, b, ev)
        })
        .collect()
}

fn chain_event(_prev: Event) -> Event {
    Event
}

fn last() -> Event {
    Event
}

fn ane_ffn_64rows(_x: &Tensor, _b: usize, _ev: Event) -> Tensor {
    Tensor::new()
}

struct Tensor;
struct Event;

impl Tensor {
    fn new() -> Self {
        Tensor
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: batch rows per call — one 128-row call per PAIR of
// 64-row blocks (two blocks' weights in one function), halving the
// gap count; the batch width stops at the measured per-row cliff
// (128-row calls stay 19% worse per token than 64-row ones).
fn run_blocks_paired(x: &Tensor, n_blocks: usize) -> Vec<Tensor> {
    (0..n_blocks)
        .step_by(2)
        .map(|b| {
            // two blocks in one call: one gap per pair, not per block
            ane_ffn_128rows_two_blocks(x, b)
        })
        .collect()
}

fn ane_ffn_128rows_two_blocks(_x: &Tensor, _b: usize) -> Tensor {
    // serves both blocks' rows; measured ~18% faster per call than two
    Tensor::new()
}

struct Tensor;

impl Tensor {
    fn new() -> Self {
        Tensor
    }
}"#
                .to_string(),
            family: AneFamily::Serving,
            notes: Some(format!(
                "THE LAW, verbatim at the pin: \"**Fewer, bigger ANE predictions.** Every evaluation \
                 has a fixed 0.11-0.23 ms gap that no submission trick hides.\" (PREFILL_RESEARCH.md \
                 lesson 1) — the Instruments evidence: \"a 0.23 ms idle gap before every ANE \
                 evaluation. It comes from the driver or firmware: private _ANEClient calls and Metal \
                 shared-event chaining (splash's method) show it too. Only fewer, bigger predictions \
                 help: `ffn2` entries run two 64-row blocks in one function, saving one gap per \
                 pair.\" THE CLIFF (the admission bound): \"128-row ANE calls stay 19% worse per token \
                 than 64-row ones\" — batch per call UP TO the measured per-row efficiency cliff, \
                 never past it; the chunked-FFN variant measured -18% per call at the same rows. \
                 RIDER (the submission-gap stall, the server findings): submitting the next \
                 accelerator call within ~2 ms of the previous one's return caused SPORADIC 300-700 \
                 ms stalls a few times per 100 cycles; a 3 ms host-side gap after each call \
                 eliminated them (greedy leaves ~1.5 ms of natural host work; sampled ~6 ms — the \
                 stall showed only in greedy runs): pacing beats zero-gap submission even though the \
                 fixed gap itself cannot be hidden. REGIME: any accelerator serving loop whose \
                 profiler shows a fixed per-call idle gap (ANE/CoreML-class firmware gaps; similar \
                 per-launch floors on queued NPU-class runtimes). ANTI-REGIME: gap-free runtimes \
                 (batch anyway for the usual reasons, not for this law); workloads whose batch width \
                 is latency-bound by the caller (batching past the caller's wait is a different \
                 trade). Distinct from entries_74's one-program-many-functions (the PROGRAM-count \
                 memory axis) — THIS owns the CALL-count latency axis; distinct from A8 (pipeline \
                 granularity — the units' overlap; A9 is the per-unit call shape under any pipeline). \
                 {ANNOTATION}. Source: {ANEM} (research/v8_prefill/PREFILL_RESEARCH.md:404-405, \
                 :179-187; the stall: COREAI_PORT_NOTES.md context-ladder section, the \
                 DRAFT_GAP_MS fix)."
            )),
        },
        // ── A10 (B) — concurrent same-function calls corrupt silently ──────
        AneRule {
            rule_id: "ane-concurrent-same-function-silent-corruption".to_string(),
            ane_concept: "one compiled accelerator function serves ONE call stream — two threads \
                          calling the SAME function concurrently, even with separate bindings, \
                          corrupt outputs at random with no error; serialize per function handle"
                .to_string(),
            trigger: "a multi-threaded host sharing one compiled accelerator function across \
                      worker threads, each thread holding its own binding/output arrays"
                .to_string(),
            buggy_code: r#"// Each worker has its OWN binding (separate output arrays, separate
// input staging), but they share the compiled function — the runtime's
// internal state is per-function, not per-binding. Outputs corrupt at
// random: KL spikes 0.6 to 4.0, no error, wrong answers ship.
fn serve_workers(program: Program, workers: usize) {
    let function = compile(program)?;
    let handles: Vec<JoinHandle<()>> = (0..workers)
        .map(|_| {
            let f = function.clone_binding(); // the SAME function underneath
            spawn_worker(move || loop {
                let out = f.evaluate(next_input()); // concurrent = corrupted
                publish(out);
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

fn compile(p: Program) -> Result<Function, Error> {
    let _ = p;
    Ok(Function)
}

fn next_input() -> Input {
    Input
}

fn publish(_o: Vec<f32>) {}

struct Program;
struct Input;
struct Function;
struct Error;
struct JoinHandle<T>(T);

impl JoinHandle<()> {
    fn join(self) -> Result<(), Box<dyn std::any::Any>> {
        Ok(())
    }
}

fn spawn_worker<F>(_f: F) -> JoinHandle<()>
where
    F: FnOnce(),
{
    JoinHandle(())
}

impl Function {
    fn clone_binding(&self) -> Self {
        Function
    }

    fn evaluate(&self, _i: Input) -> Vec<f32> {
        Vec::new()
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: ONE call thread owns every evaluate on this function
// (the pair design: one ANE thread per two units); workers stage
// inputs and ship them over a channel — the runtime's per-function
// state is never entered from two threads at once.
fn serve_workers_serialized(program: Program, workers: usize) {
    let function = compile(program)?;
    let (tx, rx) = mpsc::channel::<Input>();
    let caller = spawn_worker(move || {
        for input in rx {
            publish(function.evaluate(input)); // serialized: no corruption
        }
    });
    for _ in 0..workers {
        let tx = tx.clone();
        spawn_worker(move || loop {
            tx.send(next_input()).unwrap(); // workers stage, never call
        });
    }
    drop(tx);
    caller.join().unwrap();
}

fn compile(p: Program) -> Result<Function, Error> {
    let _ = p;
    Ok(Function)
}

fn next_input() -> Input {
    Input
}

fn publish(_o: Vec<f32>) {}

struct Program;
struct Input;
struct Function;
struct Error;

impl Function {
    fn evaluate(&self, _i: Input) -> Vec<f32> {
        Vec::new()
    }
}"#
                .to_string(),
            family: AneFamily::Concurrency,
            notes: Some(format!(
                "THE LAW, verbatim at the pin: \"**one ANE thread per pair**: two threads calling the \
                 same Core AI function concurrently (different bindings) corrupt outputs at random \
                 (KL spikes 0.6 to 4).\" / lesson 2: \"**Never call the same Core AI function from two \
                 threads at once**, even with different bindings.\" (PREFILL_RESEARCH.md:176-177, \
                 :405-406). THE SHAPE OF THE HAZARD: the corruption is UNDETECTABLE at the call site \
                 — outputs come back well-formed; the damage is statistical (KL spikes 0.6-4.0 over \
                 a healthy ~0.0-0.02), so no check-on-return catches it — the rule is the GUARD, not \
                 a check. The measured fix IS the pair design: one dedicated call thread per \
                 function (their V8 runs one ANE thread per two GPU calls). Distinct from the sec \
                 FFI family (arc-refcount-across-raw-boundary owns custody across the raw boundary; \
                 ffi-callback-invoke-outside-lock owns cross-FFI DEADLOCK) — THIS owns silent DATA \
                 corruption inside ONE framework's runtime across same-process threads, no FFI \
                 boundary needed. Regime: any compiled accelerator function whose runtime documents \
                 (or measures) single-caller semantics — CoreML MLModel's documented thread-safety \
                 exception, Core AI functions (measured here), vendor NPU contexts. ANTI-REGIME: \
                 runtimes that DOCUMENT thread-safe evaluate on independent sessions (verify against \
                 the vendor doc, never assume either way); single-threaded hosts. {ANNOTATION}. \
                 Source: {ANEM} (research/v8_prefill/PREFILL_RESEARCH.md:176-177, :405-406; the pair \
                 mode at :175-179)."
            )),
        },
        // ── A6 (B−) — the runtime-owned output-pool leak: budget the unfixable ─
        AneRule {
            rule_id: "ane-host-output-pool-unbounded-call-bound".to_string(),
            ane_concept: "a runtime that allocates fresh output buffers per call and never recycles \
                          them grows until it aborts — when the leak is RUNTIME-OWNED (no caller \
                          API to supply outputs), the host must bound call counts, watch the \
                          resource, and budget the recycle, never assume the loop can run forever"
                .to_string(),
            trigger: "a long-running accelerator inference loop whose per-call time climbs and \
                      whose wired/resident memory grows linearly with call count on a runtime that \
                      offers no output-buffer reuse API"
                .to_string(),
            buggy_code: r#"// The decode loop calls the chunk function forever; every call
// allocates 12 fresh output arrays the runtime pools and never frees.
// Per-call time 8.7 -> 12.9 ms, wired memory +0.5 GB per 600 calls,
// abort at ~850 calls (NDArray+Pool: Failed to allocate storage).
// gc.collect() every call changes nothing: not Python object lifetime.
fn decode_loop(chunk: &Chunk, tokens: usize) -> Vec<Token> {
    let mut out = Vec::new();
    for _ in 0..tokens {
        let outputs = chunk.call(next_state()); // fresh NDArrays every call
        out.push(read_token(&outputs));
    }
    out
}

fn read_token(_o: &[Buffer]) -> Token {
    Token
}

fn next_state() -> State {
    State
}

struct Chunk;
struct State;
struct Token;
struct Buffer;

impl Chunk {
    fn call(&self, _s: State) -> Vec<Buffer> {
        Vec::new()
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: the leak is inside the runtime — budget it. Bound the
// call count per process epoch, watch wired memory against the
// budget, recycle (reload the program) at the bound, and treat the
// reload cost as part of the serving math (their reload: 60-80 s).
const CALL_BUDGET: usize = 600;
const WIRED_BUDGET_BYTES: usize = 512 * 1024 * 1024;

fn decode_loop_bounded(chunk: &Chunk, tokens: usize, mut wired: usize) -> Vec<Token> {
    let mut out = Vec::new();
    let mut calls = 0;
    for _ in 0..tokens {
        if calls >= CALL_BUDGET || wired >= WIRED_BUDGET_BYTES {
            // the runtime-owned leak: recycle BEFORE the abort, not after
            chunk.reload(); // the budgeted recycle (60-80 s, priced into the lane)
            calls = 0;
            wired = 0;
        }
        let outputs = chunk.call(next_state());
        wired += outputs_wired_delta(&outputs);
        out.push(read_token(&outputs));
        calls += 1;
    }
    out
}

fn outputs_wired_delta(_o: &[Buffer]) -> usize {
    0
}

fn read_token(_o: &[Buffer]) -> Token {
    Token
}

fn next_state() -> State {
    State
}

struct Chunk;
struct State;
struct Token;
struct Buffer;

impl Chunk {
    fn call(&self, _s: State) -> Vec<Buffer> {
        Vec::new()
    }

    fn reload(&self) {}
}"#
                .to_string(),
            family: AneFamily::Lifecycle,
            notes: Some(format!(
                "THE LAW, verbatim at the pin (FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md + \
                 COREAI_PORT_NOTES.md): \"every call allocates new output NDArrays (12 per chunk \
                 call for our model), the runtime's output pool keeps growing (per-chunk call time \
                 8.7 -> 12.9 ms, wired memory +0.5 GB over 600 calls), and after ~850 function calls \
                 the process aborts\" (NDArray+Pool.swift:77 Fatal error) — \"No API to supply output \
                 buffers (Core ML has `outputBackings`)\"; gc every call with a frozen heap changes \
                 nothing; the source's candidate fix names the direction (DeltaNet states as \
                 runtime state buffers updated in place, \"whole-buffer writes, not scatter\", so a \
                 chunk returns only the small outputs — test it stays on the ANE) and otherwise \
                 \"report to Apple\". THE RULE: when the leak is runtime-owned, the HOST carries the \
                 lifecycle budget — bound calls per epoch, watch the resource (wired memory read \
                 cheaply via host_statistics64 wire_count, or the runtime's own per-call-time \
                 climb: 8.7 -> 12.9 ms is itself the gauge), recycle at the budget BEFORE the \
                 abort, price the recycle (reload 60-80 s even warm) into the serving math. A \
                 decoder calling many programs per token (theirs: 17/token) cannot run at all \
                 without the budget. DISTINCT-FROM: `untrusted-keyed-state-unbounded` (sec) owns \
                 peer-keyed table growth (attacker-keyed cardinality) — THIS owns a fixed-call-count \
                 runtime leak with no adversary; `capacity-gated-exact-undo-loud-degrade` (entries_82) \
                 owns undo budget admission — adjacent budget discipline, different axis; \
                 `derived-cache-throwaway-version-reset-recovery` (entries_75) owns \
                 discard-and-recover on a disposable cache — the recycle-at-budget shape is that \
                 law's resource twin, adjudicated compose. Regime: any long-running accelerator \
                 serving loop on a runtime that allocates per call. ANTI-REGIME: runtimes with \
                 output-buffer reuse (supply the buffers — the leak does not exist); workloads whose \
                 total call count is naturally bounded below the budget. {ANNOTATION}. Sources: \
                 {ANEM} (FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md §2; COREAI_PORT_NOTES.md the \
                 output-pool blocker + the state-buffer candidate)."
            )),
        },
        // ── A7 (C+) — gate size/complexity + version at BUILD time: the panic ─
        AneRule {
            rule_id: "ane-package-size-build-gate-before-kernel-panic".to_string(),
            ane_concept: "a failure class the process cannot catch (a kernel panic in the \
                          accelerator driver, a segfault on a package-version mismatch) must be \
                          gated BEFORE it fires — size/complexity gates at build time and an \
                          explicit version check before load, never a runtime try"
                .to_string(),
            trigger: "an accelerator package build whose size/complexity is growing past a \
                      measured-stable envelope, or a package load where the producer's format \
                      version may exceed what the runtime reads"
                .to_string(),
            buggy_code: r#"// The build ships whatever size it lands at; the load path assumes
// a version mismatch would fail with a version error. Past the
// measured-stable envelope the driver panics THE KERNEL (uncatchable
// from user space); a format mismatch segfaults instead of erroring.
fn load(path: &str) -> Model {
    // no size gate, no version check — trust the runtime
    load_unchecked(path)
}

fn build(spec: &Spec) -> Package {
    compile_all_layers(spec) // 12 layers, 1.67 GB — nobody checks
}

fn compile_all_layers(_s: &Spec) -> Package {
    Package
}

fn load_unchecked(_p: &str) -> Model {
    Model
}

struct Spec;
struct Package;
struct Model;

impl Package {
    fn layers(&self) -> usize {
        12
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: the envelope is a measured build gate — stay inside the
// measured-stable size/layer budget (their data: 4-layer/0.52-0.72 GB
// stable for thousands of calls; 8-layer/1.1 GB panics the kernel,
// reproduced on a fresh boot), and the producer's format version is
// checked against the runtime's known-good range BEFORE load, with a
// typed refusal naming both versions.
const MAX_LAYERS: usize = 4;
const MAX_PACKAGE_BYTES: usize = 800 * 1024 * 1024; // inside 0.52-0.72 GB stable
const KNOWN_GOOD_FORMAT: u32 = 7;

fn build(spec: &Spec) -> Result<Package, BuildRefusal> {
    let pkg = compile_all_layers(spec);
    if pkg.layers() > MAX_LAYERS || pkg.size_bytes() > MAX_PACKAGE_BYTES {
        return Err(BuildRefusal::OverStableEnvelope {
            layers: pkg.layers(),
            bytes: pkg.size_bytes(),
            note: "past the measured-stable envelope the driver panics the kernel — split the package",
        });
    }
    Ok(pkg)
}

fn load(pkg: &Package, format_version: u32) -> Result<Model, LoadRefusal> {
    if format_version > KNOWN_GOOD_FORMAT {
        // the runtime segfaults on newer formats — refuse typed instead
        return Err(LoadRefusal::FormatTooNew {
            got: format_version,
            known_good: KNOWN_GOOD_FORMAT,
        });
    }
    Ok(load_checked(pkg))
}

fn load_checked(_p: &Package) -> Model {
    Model
}

struct Spec;
struct Package;
struct Model;

enum BuildRefusal {
    OverStableEnvelope { layers: usize, bytes: usize, note: &'static str },
}

enum LoadRefusal {
    FormatTooNew { got: u32, known_good: u32 },
}

impl Package {
    fn layers(&self) -> usize {
        4
    }

    fn size_bytes(&self) -> usize {
        700 * 1024 * 1024
    }
}"#
                .to_string(),
            family: AneFamily::Lifecycle,
            notes: Some(format!(
                "THE MEASURED CLASS, at the pin (FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md §4): a \
                 12-layer 1.67 GB Core AI chunk failed every call with a typed runtime error \
                 (MPSGraphDelegateError, reproducible); loading an 8-layer 1.1 GB chunk PANICKED THE \
                 KERNEL — \"panic(cpu 7 ...): Break 0x0800 instruction exception from kernel. Panic \
                 (by design)\", panicked task `aned`, backtrace in the ANE driver, reproduced on a \
                 freshly booted machine; 4-layer chunks (0.52-0.72 GB) of the same model ran \
                 thousands of calls stable; a watchdog reset hit during large-context compiles; and \
                 a package-format version mismatch \"the load segfaults the process instead of \
                 failing with a version error\" (writer emits 7.1.2, runtime reads up to 7.0.80). \
                 THE LAW: a failure the process cannot catch (kernel panic, segfault) can only be \
                 PREVENTED upstream — gate package size/complexity at BUILD time against a \
                 measured-stable envelope (split past it), and version-check before load with a \
                 typed refusal; a runtime `try` around the load is worthless against this class. \
                 Grade C+ honesty: the envelope numbers are one machine/OS generation's measurements \
                 (M6, macOS 27) — re-measure the envelope per target; the RULE (gate before the \
                 uncatchable) is the transferable part. DISTINCT-FROM: \
                 batch-load-admission-sized-from-files-refuses-past-the-hard-budget (B274) owns \
                 WORK admission at a budget — THIS owns artifact admission against an UNCACTCHABLE \
                 failure class (the refusal protects the host, not a budget); \
                 ane-host-output-pool-unbounded-call-bound (this batch) owns the runtime's OWN leak \
                 at serve time — THIS the build/load-time gate. Regime: accelerator package \
                 producers and load paths (Core ML / Core AI / vendor NPU containers). ANTI-REGIME: \
                 failures with typed runtime errors (handle them at load, no gate needed); packages \
                 far inside any measured envelope with a pinned format range. {ANNOTATION}. Source: \
                 {ANEM} (FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md §4)."
            )),
        },
    ]
    .into_iter()
    .collect();

    Corpus::new(entries)
}

#[cfg(test)]
mod tests;
