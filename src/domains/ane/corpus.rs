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
//! - `ddalcu/mlx-serve` @ `96311b26` (MIT; the Batch 399 ANE wave — the
//!   `docs/reference.md` "ANE prefill rules" + media-offload + share-table
//!   sections and the gotchas incidents they cite; Research 432 §Queue
//!   named this surface "the ane domain's next seed", the Batch 398 T0 pin
//!   still the tip at this batch's T0).

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
const MLXS: &str = "ddalcu/mlx-serve @ 96311b26 (docs/reference.md; the Batch-398 T0 pin, re-verified at the Batch-401 T0 — the tip has since moved, the pin is the record)";
const MLXSG: &str = "ddalcu/mlx-serve @ 96311b26 (docs/gotchas/engine-mlx.md)";
const MLXSM: &str = "ddalcu/mlx-serve @ 96311b26 (docs/gotchas/models-media.md)";
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
                 DRAFT_GAP_MS fix). FACE (Batch 399, the free-width face, count-neutral; \
                 ddalcu/mlx-serve @ 96311b26 docs/reference.md, MIT): a runtime whose rows up \
                 to its free width cost what a small batch does gives the batch width a FLOOR \
                 before any cliff tuning — \"a program object runs one call at a time, and up to \
                 64 rows cost what 8 do\": batch to the free width (64) first, THEN tune past \
                 it against this rule's measured cliff. Compose: the free width is the cliff \
                 search's starting bound, never a replacement for the cliff measurement.")),
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

        // ── Batch 399 wave (Research 432 §Queue slice 2 — the mlx-serve ANE seed) ──
        // ── M1 (B+ LEAD) — the fp16 range scale folds into the LINEAR operand ───
        AneRule {
            rule_id: "ane-fp16-range-scale-folds-into-linear-operand".to_string(),
            ane_concept: "a split/seam accelerator graph is fp16 end to end, so a partial-sum \
                          range guard must SCALE — and the fold lands on the LINEAR operand of \
                          the fused product (multiplied back on read), never on the operand \
                          sitting under a nonlinearity and never on the output alone"
                .to_string(),
            trigger: "an accelerator (ANE/CoreML-class) offload seam whose fused product \
                      (silu(gate)*up-class) feeds a reduction whose partial sums can peak far \
                      past the fp16 max — the seam plans its weight copies and its parity probe"
                .to_string(),
            buggy_code: r#"// Split MLP offload: ANE holds gate/up channels + the down conv, GPU
// the complement; partials ADD at the seam. No range scale anywhere:
// the graph computes fp16 throughout, and the down-conv partial sum
// peaks at ~1.7M on real latents while fp16 maxes at 65_504 —
// saturation to INF from block 36, then NaN; the video renders BLACK.
// The parity probe misses it: per-block cos reads 0.99999 on probe
// inputs that never reach the saturating amplitude.
fn split_mlp_offload(gate: &[f16], up: &[f16], down_w: &[f16]) -> Vec<f16> {
    let act: Vec<f16> = gate.iter().zip(up).map(|(g, u)| g * u).collect();
    let mut out = vec![0f16; act.len()];
    for (o, a) in out.iter_mut().zip(&act) {
        *o = dot_f16(down_w, a); // partial peaks ~1.7M > 65_504: INF, NaN
    }
    debug_assert!(cos_ratio(&out, gpu_reference()).abs() > 0.99); // probe-blind
    out
}

fn dot_f16(_w: &[f16], _a: &f16) -> f16 {
    f16::ZERO
}

fn cos_ratio(_a: &[f16], _b: &[f16]) -> f16 {
    f16::ONE
}

fn gpu_reference() -> Vec<f16> {
    Vec::new()
}"#
                .to_string(),
            fixed_code: r#"// FIXED: the range scale rides the LINEAR operand's weight copy
// AT BUILD TIME — `up` is linear into silu(gate)*up, so folding 1/256
// there scales the whole product exactly (a power of two is exact in
// fp16: no mantissa change, only the exponent), and the read-back
// multiplies it out. Scaling `gate` is WRONG (it sits under the
// nonlinearity — the product stops being the scaled one); scaling the
// down-conv output alone is WRONG (the partial already saturated).
const OUT_PLANE_SCALE_LOG2: i32 = -8; // 2^-8 = 1/256, folded into `up`

fn build_up_copy(up_host: &[f16]) -> Vec<f16> {
    let s = f16::from(2u8).powi(OUT_PLANE_SCALE_LOG2);
    up_host.iter().map(|u| *u * s).collect() // build-time, once per weight
}

fn split_mlp_offload_scaled(
    gate: &[f16],
    up_copy: &[f16], // the scaled copy
    down_w: &[f16],
) -> Vec<f16> {
    let act: Vec<f16> = gate.iter().zip(up_copy).map(|(g, u)| g * u).collect();
    let mut out = vec![0f16; act.len()];
    for (o, a) in out.iter_mut().zip(&act) {
        *o = dot_f16(down_w, a); // partial now peaks ~1.7M/256 < 65_504
    }
    let back = f16::from(2u8).powi(-OUT_PLANE_SCALE_LOG2);
    out.iter().map(|v| *v * back).collect() // multiply back ON READ
}

/// The probe discipline: range headroom is invisible to cos on probe
/// inputs — the parity gate runs amplitude probes PAST the real peak.
fn parity_with_amplitude_probes(out: &[f16], reference: &[f16]) -> bool {
    cos_ratio(out, reference) > f16::from_f32(0.99)
        && amplitude_probe(out, reference, 16.0) // 16x the real amplitude
}

fn amplitude_probe(_out: &[f16], _reference: &[f16], _factor: f32) -> bool {
    true
}"#
                .to_string(),
            family: AneFamily::Numerics,
            notes: Some(format!(
                "THE MEASURED CLASS: the ANE MLP offload on H3 gave 1.24x and an all-zero video; \
                 the program computes in fp16 and H3's fc2 partial peaks at ~1.7M (fp16 max \
                 65_504): INF, then NaN; per-block parity read cos 0.99999 on probes that never \
                 reached the saturating amplitude; Krea had been silently losing precision (cos \
                 vs GPU 0.996 → 0.9993 after the fix). \"A correct-looking image is not evidence \
                 of range headroom.\" THE LAW, verbatim: \"The ANE graph is fp16 END TO END, so a \
                 partial-sum seam must SCALE — folded into the `up` copy at build time, \
                 multiplied back on read — exact: per-row int8 puts it in the row scale, and `up` \
                 is linear into silu(gate)*up. Scaling `gate`, under a nonlinearity, or fc2 alone \
                 is wrong.\" OPERAND-PLACEMENT is the decision content: the fold's operand must be \
                 LINEAR INTO THE PRODUCT (the fused output scales exactly); the nonlinearity-side \
                 operand and the output alone both break the scaling identity. KINSHIPS (all \
                 adjudicated, merge refused): \
                 ane-fp16-norm-pow2-prescale-overflow (this corpus) owns the OVERFLOW pole on \
                 NORM STATISTICS (x² of a residual stream) with a scale-invariant output and NO \
                 read-back — THIS owns the partial-sum/product pole with a read-back multiply; \
                 constant-scale-folded-into-upload-copy (kernel gpu_04, B242) folds a LoRA \
                 alpha/rank scale into the upload copy for DISPATCH ELIMINATION (perf motive, \
                 no range constraint) — THIS folds for RANGE (the numerics motive) and adds the \
                 placement law the perf fold never needed; \
                 fp16-subnormal-product-power-of-two-rescale + hi-lo-split-pow2-prescale (rust_perf, \
                 B352) own the UNDERFLOW pole on GEMM operands — THIS the overflow pole at a \
                 partial-sum seam; masked-in-place-field-prescale-other-operand (kernel gpu_98) \
                 cancels a PER-PASS BIT OFFSET through the other operand — a bit-alignment axis, \
                 not a range axis. Regime: fp16 end-to-end accelerator graphs with fused \
                 nonlinearity products feeding reductions (ANE/CoreML-class; never needed on \
                 f32 hosts). ANTI-REGIME: the scale factor not a power of two in fp16 (mantissa \
                 rounds — the exactness claim dies); a graph whose fused operand is NOT linear \
                 into the read product (place the scale on the true linear input or widen the \
                 accumulator instead). {ANNOTATION}. Sources: {MLXS} (ANE prefill rules, the \
                 OUT_PLANE_SCALE bullet); the incident: {MLXSM} (\"The ANE seam is fp16 end to \
                 end, and H3's DiT rendered black\"); the probe discipline (16x amplitude probes): \
                 {MLXSG} (\"What the ANE build taught\")."
            )),
        },

        // ── M2 (B+) — the load bill is the runtime COPY's shape, not the shard ──
        AneRule {
            rule_id: "ane-load-bills-runtime-copy-shape-not-shard-bytes".to_string(),
            ane_concept: "an accelerator load-admission gate bills the engine's RUNTIME footprint \
                          — the transformed weight copy (a sub-byte pack's int8 copy GROWS), each \
                          slot's KV at the top context, and the build-time transient peak — never \
                          the on-disk shard bytes, which under-admit and swap inside a ranked \
                          request with nothing logged"
                .to_string(),
            trigger: "an accelerator model-load gate (or a media-offload build gate) sizing \
                      admission from the pack/shard bytes on disk, while the runtime will hold \
                      a converted copy, per-slot KV at the top context, and a compile transient"
                .to_string(),
            buggy_code: r#"// Admission sized from the SHARD bytes on disk. Three blind spots:
// (1) the runtime copies weights to int8 — a 4-bit pack's int8 copy
//     GROWS (~1.8x its linears), so shard bytes UNDER-bill the copy;
// (2) KV is held per slot AT THE TOP CONTEXT — shard bytes carry none;
// (3) the ANE build's transient peak (f32 scratch during compile).
// The total-RAM gate alone admitted 6.8 GB on a 32 GB box beside a
// 14.7 GB pack and swapped ~380 MB inside the ranked request, with
// nothing logged.
fn admit_model(pack_shard_bytes: u64, free_ram: u64) -> bool {
    pack_shard_bytes + KV_STAGING < free_ram // shard bytes != runtime bytes
}

const KV_STAGING: u64 = 0; // "KV? that's runtime, figure it out later""#
                .to_string(),
            fixed_code: r#"// FIXED: the bill names the runtime shapes. The int8 copy is
// billed PER MODEL from the conversion plan (not the shard); KV is
// billed at the TOP context per slot; the build peak (hidden*ffn
// scratch in f32) is billed against what is available NOW under a
// swap floor. A bill past the headroom refuses BY NAME.
const SWAP_FLOOR_BYTES: u64 = 4 * 1024 * 1024 * 1024;

struct LoadBill {
    int8_copy_bytes: u64,    // from the conversion plan, NOT the shard
    kv_top_context_bytes: u64, // per slot, at the top context rung
    build_peak_bytes: u64,   // 4 * hidden * ffn * sizeof::<f32>() class
}

fn bill(pack: &Pack, plan: &ConvertPlan, slots: usize, top_ctx_rows: usize) -> LoadBill {
    LoadBill {
        // a 4-bit pack's linears GROW ~1.8x when copied to int8 —
        // the plan knows the per-linear shapes, the shard does not
        int8_copy_bytes: plan.converted_bytes(pack),
        kv_top_context_bytes: plan.kv_bytes_per_slot(top_ctx_rows) * slots as u64,
        build_peak_bytes: plan.build_peak_bytes(),
    }
}

fn gate_allows(b: &LoadBill, free_ram_now: u64, wired_floor: u64) -> Result<(), String> {
    let available = free_ram_now.saturating_sub(wired_floor);
    let total = b.int8_copy_bytes + b.kv_top_context_bytes;
    if total > available {
        return Err(format!(
            "load refused: copy+KV {total} > available {available} (shard bytes were smaller)"
        ));
    }
    if b.build_peak_bytes > available.saturating_sub(SWAP_FLOOR_BYTES) {
        return Err("load refused: build transient would cross the swap floor".into());
    }
    Ok(())
}"#
                .to_string(),
            family: AneFamily::Lifecycle,
            notes: Some(format!(
                "THE MEASURED CLASS, verbatim: \"The media gate bills the BUILD PEAK too \
                 (mediaGateRefusal: total-RAM bill, then int8 copy + buildPeakBytes = 4 x hidden \
                 x ffn x f32 against what is available now under a 4 GB SWAP_FLOOR_BYTES). The \
                 total-RAM gate alone admitted 6.8 GB on a 32 GB M1 Pro beside a 14.7 GB pack \
                 and swapped ~380 MB inside the ranked request with nothing logged.\" And the \
                 load gate: \"the gate and the preflight bill the int8 copies and each slot's KV \
                 at the top context (load_bytes), not the pack's shards.\" THE GROWTH LAW: the \
                 runtime copy of a sub-byte pack is LARGER than the shard — \"a 4-bit pack \
                 therefore grows (~1.8x its linears)\" and \"ANE copy stays int8, billed per \
                 model (engineBillBytes + gateAllows, headroom 12 GB)\"; the bill implementation \
                 reads shard headers and RE-BILLS the linears the conversion names \
                 (w8a8.residentBytes + expectBilled, the arm asserting resident bytes against \
                 the bill). Loading peaks at the CONVERTED size, not packed-plus-copy (the \
                 struct loaders convert then drop the shard keys). Distinct-from (all \
                 adjudicated): batch-load-admission-sized-from-files-refuses-past-the-hard-budget \
                 (B274) owns admission SIZED FROM FILES (metadata, nothing read) — THIS is the \
                 pole it must be paired with: when the runtime TRANSFORMS what it loaded, the \
                 file-sized bill is the under-bill, and the transformed shape is the billable; \
                 gpu-residency-summed-beyond-process-and-cgroup-accounting (B367) dedups a \
                 measured FOOTPRINT by owner — THIS derives the bill from the conversion PLAN \
                 before anything is resident; one-program-many-functions-accelerator-service-memory \
                 (B358) bills PER LOADED PROGRAM at the accelerator service — THIS bills the \
                 load-time copy + KV + transient at the host gate; \
                 budget-room-from-constructed-artifact-not-configured-cap (B375) derives budget \
                 room from the CONSTRUCTED artifact's true size — kin in motive, but THAT corrects \
                 a configured cap vs built rows, THIS replaces the shard proxy with the runtime \
                 shape class entirely. ANE-side notes: sub-byte ANE weights are a NO-GO on the \
                 compile path (int4/sub-byte constexpr forms fail on macOS 26.x aned, bisected) \
                 so the copy is int8 by construction and the growth is structural; probe convs \
                 need ROWS>=32. Regime: accelerator load/build admission gates (ANE/CoreML, \
                 NPU containers, quantized-pack loaders). ANTI-REGIME: a runtime that serves \
                 shards in place with no conversion and no per-slot KV (the shard IS the \
                 footprint — then file sizing is exact); budgets with an external hard floor \
                 already carried per-shape. {ANNOTATION}. Sources: {MLXS} (the engine section's \
                 Loading bullet + the ANE prefill rules' build-peak + int8 bullets + the w8a8 \
                 \"The bill\" bullet); the int8 no-go: {MLXSG} (\"What the ANE build taught\")."
            )),
        },

        // ── M3 (B+) — the compile-cache cap is volume-derived, pruned by lineage ──
        AneRule {
            rule_id: "ane-compile-cache-cap-volume-derived-lineage-pruned".to_string(),
            ane_concept: "an accelerator compile cache's byte cap is DERIVED FROM THE LIVE VOLUME \
                          (min(configured, free - reserve)) and pruned by lineage before bytes, \
                          never evicting the in-flight build set — a fixed cap above free space \
                          is no cap and ships SILENT PARTIAL COVERAGE, and evicting the loading \
                          model's entries recompiles into an eviction thrash"
                .to_string(),
            trigger: "a compile-artifact cache (ANE programs, tuned kernels, graphs) whose byte \
                      cap is a configured constant and whose eviction is LRU-only — on a small \
                      disk or a share sweep, coverage decays boot over boot"
                .to_string(),
            buggy_code: r#"// The cap is a fixed constant and eviction is plain LRU. Two measured
// failures: (1) a fixed cap above the disk's free space is no cap —
// a full disk ships SILENT PARTIAL COVERAGE, advertised as ready: 19/64
// (coverage fell 112 -> 29 programs/boot over one night; 49 GB in an
// evening); (2) LRU evicts the model STILL LOADING: with two layouts of
// one model over the cap, every cold store evicted the loading model's
// entries, which recompiled and evicted more — a thrash loop.
const CAP_GB: u64 = 40; // configured; the volume's free space unread

fn prune(cache: &mut Cache) {
    while cache.bytes() > CAP_GB * GB {
        // plain LRU: the oldest entry, whatever it belongs to
        let victim = cache.lru_oldest().expect("non-empty");
        cache.remove(victim); // may be the model currently building
    }
}

struct Cache;

impl Cache {
    fn bytes(&self) -> u64 {
        0
    }

    fn lru_oldest(&self) -> Option<EntryId> {
        None
    }

    fn remove(&mut self, _id: EntryId) {}
}

struct EntryId;
const GB: u64 = 1024 * 1024 * 1024;"#
                .to_string(),
            fixed_code: r#"// FIXED: the cap is min(configured, volume free + cache - reserve);
// the prune drops the group's OTHER VARIANTS first (a share sweep once
// stacked 7.9 GB for five values), then LRU; the set being built is
// never evicted. A free-disk floor refuses the build BY NAME, and
// coverage converges across boots via the cache (in-session retries
// after a full disk are futile — the compiler service holds its budget
// for the connection's lifetime).
const CONFIGURED_CAP_GB: u64 = 40;
const DISK_RESERVE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const BUILD_FLOOR_BYTES: u64 = 1024 * 1024 * 1024; // refused by name below

fn effective_cap(volume_free: u64, cache_bytes: u64) -> u64 {
    // a fixed cap above free space was no cap: a full disk ships
    // silent partial coverage (the ready: N/M under-count is the tell)
    CONFIGURED_CAP_GB * GB
        .min(volume_free + cache_bytes)
        .saturating_sub(DISK_RESERVE_BYTES)
}

fn prune_lineage(cache: &mut Cache, building_group: GroupId) {
    let cap = effective_cap(volume_free("/private/tmp"), cache.bytes());
    // 1) same-group OTHER variants first (stale shares/shapes),
    // 2) then LRU past the cap,
    // 3) NEVER the in-flight build set.
    cache.drop_other_variants_of(building_group);
    while cache.bytes() > cap {
        match cache.lru_oldest_not_in(building_group) {
            Some(id) => cache.remove(id),
            None => break, // nothing evictable: refuse the next build
        }
    }
}

fn volume_free(_path: &str) -> u64 {
    0
}

struct Cache;

struct GroupId;

struct EntryId;

const GB: u64 = 1024 * 1024 * 1024;

impl Cache {
    fn bytes(&self) -> u64 {
        0
    }

    fn drop_other_variants_of(&mut self, _g: GroupId) {}

    fn lru_oldest_not_in(&self, _g: GroupId) -> Option<EntryId> {
        None
    }

    fn remove(&mut self, _id: EntryId) {}
}

fn preflight_build(volume_free: u64) -> Result<(), String> {
    if volume_free < BUILD_FLOOR_BYTES {
        return Err(format!(
            "ANE build refused: free disk {volume_free} < floor {BUILD_FLOOR_BYTES}"
        ));
    }
    Ok(())
}"#
                .to_string(),
            family: AneFamily::Lifecycle,
            notes: Some(format!(
                "THE MEASURED CLASS: coverage fell 112 to 29 programs per boot over one night, \
                 the server advertised ready: 19/64 and ran an A/B at 30% coverage; 49 GB of \
                 cache in an evening; a share sweep stacked 7.9 GB for five values of one group; \
                 with two layouts of one model over the cap, every cold store evicted entries of \
                 the model still loading, which then recompiled and evicted more. THE LAWS, \
                 verbatim: \"A fixed cap above free space was no cap: a full disk ships SILENT \
                 PARTIAL COVERAGE (ready: N/M under-count is the tell)\"; \"The prune drops the \
                 group's other variants first and never evicts the set being built\"; cap = \
                 \"min(MLX_SERVE_ANE_CACHE_CAP_GB 40, volume free + cache - CACHE_DISK_RESERVE \
                 8 GB)\"; \"A compile session's budget is the INTERNAL free disk at boot \
                 (~260 MB/program); in-session retries are futile — a cold build CONVERGES \
                 across boots\"; \"When ANE compiles fail bare, check disk first.\" STAGING NOTES \
                 (the lifecycle ring around the cache): killed-server staging leaks 8-20 GB/boot \
                 and is reaped by pid-marker ownership on first create (the framework owns \
                 $TMPDIR/<identifier>); compile INPUTS are deleted after load but the COMPILED \
                 artifacts must stay — the compiler service demand-reads them for the \
                 connection's lifetime (deleting a live program's staging is unsafe; only \
                 weights blob + MIL text are deletable post-load). KINSHIPS (adjudicated): \
                 reserve-then-commit-capacity-accounting (kernel entries_58, B242-sccache) owns \
                 RESERVE-then-commit capacity accounting at store time — THIS derives the cap \
                 from live volume state and orders the EVICTION (lineage first, in-flight \
                 protected); budget-room-from-constructed-artifact-not-configured-cap (B375) \
                 corrects a configured cap against the CONSTRUCTED artifact's true size — THIS \
                 corrects it against the EXTERNAL volume (the disk is the constraint, not the \
                 artifact); vanished-entry-tolerant-eviction-panic-else (B343-sccache) owns the \
                 eviction ERROR classes (peer NotFound vs panic) — THIS the eviction ORDER and \
                 the cap derivation; derived-cache-throwaway-version-reset-recovery (B359-ajika) \
                 owns discard-and-recompute recovery — the converge-across-boots clause is that \
                 posture applied to compile coverage; persistent-cache-teardown-needs-second-witness \
                 owns teardown witness, not cap derivation. Regime: compile-artifact caches \
                 (ANE programs, Metal/CUDA tuned kernels, ML graph caches) on volumes the build \
                 does not own. ANTI-REGIME: caches on a volume dedicated to them (the cap IS \
                 the volume); LRU correctness proven under the real dual-layout load. \
                 {ANNOTATION}. Sources: {MLXS} (the engine section's Compile-cache bullet + the \
                 ANE prefill rules' disk/lineage/session bullets); the incidents: {MLXSG} \
                 (\"The compile budget is the internal free disk, and a killed server leaks its \
                 staging\")."
            )),
        },

        // ── M4 (B+) — the plane row-pitch grid: compiles fine, fails EVERY eval ──
        AneRule {
            rule_id: "ane-plane-row-pitch-grid-refused-at-plan-time".to_string(),
            ane_concept: "an accelerator plane/tile whose per-channel row pitch misses its byte \
                          grid compiles silently and fails EVERY evaluation — the shape grid is \
                          checked and the row count FLOORED at plan time with a typed refusal, \
                          never trusted to compile-time or eval-time signals"
                .to_string(),
            trigger: "a split/offload planner computing tile or share row counts whose byte pitch \
                      (rows x dtype width) must sit on the accelerator's grid — the share \
                      parameter straddles a rounding line"
                .to_string(),
            buggy_code: r#"// The share parameter picks the row split directly. rows = 16 mod 32
// (e.g. 0.35 of a 6144-row plane) COMPILES silently — then fails EVERY
// eval ("Program Inference error"), and the per-chunk fallback recomputes
// the whole layer on the GPU: the v2 share-0.35 collapse measured BELOW
// the off arm, with no compile-time or load-time error anywhere.
fn split_rows(plane_rows: usize, share: f64) -> usize {
    let rows = (plane_rows as f64 * share) as usize;
    rows // 2150: pitch 2150*2 = 4300 bytes — off the 64-byte grid
}

fn compile_tile(_rows: usize) -> Program {
    Program // compiles fine
}

struct Program;"#
                .to_string(),
            fixed_code: r#"// FIXED: the grid is a PLAN-TIME check. The row count floors to the
// alignment the pitch demands (rows*2 bytes on a 64-byte grid == rows
// % 32 == 0) and BOTH emitters refuse an off-grid row count BY NAME —
// compile-acceptance is never evidence (off-grid compiles fine, then
// fails every eval).
const GRID_BYTES: usize = 64;
const DTYPE_BYTES: usize = 2;
const ROW_MULTIPLE: usize = GRID_BYTES / DTYPE_BYTES; // 32

const fn floor_rows_to_grid(rows: usize) -> usize {
    rows - (rows % ROW_MULTIPLE)
}

fn split_rows_checked(plane_rows: usize, share: f64) -> Result<usize, String> {
    let raw = (plane_rows as f64 * share) as usize;
    let rows = floor_rows_to_grid(raw);
    if rows == 0 {
        return Err("split refused: share floors to zero rows".into());
    }
    if raw != rows {
        // both emitters refuse by name — never let an off-grid shape
        // reach the compiler
        return Err(format!(
            "split refused: {raw} rows off the {ROW_MULTIPLE}-row grid (pitch must sit on {GRID_BYTES}B)"
        ));
    }
    Ok(rows)
}

struct Program;"#
                .to_string(),
            family: AneFamily::GraphShape,
            notes: Some(format!(
                "THE MEASURED CLASS, verbatim: \"fp16 plane row pitch must be equivalent 0 mod 32 \
                 — off-grid tiles compile fine then fail EVERY eval ('Program Inference error'; \
                 the v2 share-0.35 collapse). aneShareRows floors to 32; both C emitters refuse \
                 by name.\" And the incident: \"share 0.35 measured below the off arm. Cause: an \
                 fp16 plane's per-channel pitch (rows x 2 bytes) must sit on a 64-byte grid; \
                 rows == 16 mod 32 compile silently and fail every eval, and the per-chunk \
                 fallback recomputed on the GPU.\" THE GRID IDENTITY: rows x dtype-width on a \
                 64-byte grid equals rows % 32 for fp16 — the byte grid and the row multiple are \
                 ONE constraint stated in two units; keep the dtype width IN the constant so a \
                 dtype change re-derives the grid. THE FAILURE SHAPE is the class: \
                 compile-acceptance and eval-failure are SPLIT — the compiler accepts the shape, \
                 every evaluation fails, and the fallback silently eats the win (the measured \
                 arm read WORSE than off). THE LAW: plan-time floor + typed refusal at BOTH \
                 emitters; never a compile-time or load-time trust. KINSHIPS (adjudicated): \
                 ane-int8-matmul-shape-silent-wrong-gate (this corpus, A4) owns the silent-WRONG \
                 shape class (quantized matmul computes near-zero/wrong with NO error — the \
                 defense is a host-reference gate); THIS owns the silent-FAIL shape class \
                 (compiles, then every eval errors — the defense is the plan-time grid floor); \
                 both are \"compile-success is not serve-correctness\" but the failure signal \
                 differs (wrong data vs loud eval error) and so does the defense (reference gate \
                 vs grid floor); align-64-false-sharing (kernel entries_gpu_02 class) aligns \
                 CACHE LINES on the host — a different grid (L1 line vs eval-plane pitch), a \
                 different cost (shuffling vs every-eval failure); pow2-block-alignment (kernel) \
                 aligns for mask arithmetic — likewise. Regime: accelerator tile/share planners \
                 whose row counts feed compiled programs (ANE/CoreML-class; any accelerator \
                 with an eval-side pitch grid). ANTI-REGIME: shapes already floored upstream \
                 with a refusal at the same grid; accelerators that reject off-grid shapes at \
                 compile time with a typed error (the compiler IS the check — then only the \
                 floor-for-usability half applies). {ANNOTATION}. Sources: {MLXS} (ANE prefill \
                 rules, the pitch bullet); the incident: {MLXSG} (\"The share 0.35 'tiling cliff' \
                 was every eval failing\")."
            )),
        },

        // ── M5 (B) — the split share is calibrated per silicon, never interpolated ──
        AneRule {
            rule_id: "ane-split-share-per-silicon-never-interpolated".to_string(),
            ane_concept: "a split/offload share's optimum is PER SILICON and PER WORKLOAD — the \
                          analytic solve is only the SEED for a once-per-model calibration whose \
                          result STICKS as a lineage tag, an explicit override is NEVER cached as \
                          calibrated, and a share change needs its own A/B (never interpolation)"
                .to_string(),
            trigger: "a host/accelerator split ratio (channel share, layer split, CPU/GPU \
                      partition) whose value is derived from a formula or copied across chips, \
                      and a compiled artifact is keyed by the value"
                .to_string(),
            buggy_code: r#"// The share is solved per request (or copied from another chip), and
// every re-solve RECOMPILES the program set: the share straddled a
// rounding line, so per-request re-solve recompiled + pruned the whole
// set on every request size and on probe jitter. Measured optima:
// M4 0.85/0.60/0.45 per model; M3 Ultra 0.35; 0.50 REGRESSES on M4 —
// interpolation across chips is wrong by construction.
fn share_for(request: &Request) -> f64 {
    // re-solve per request: probe jitter moves the answer, the answer
    // keys compiled artifacts, artifacts recompile + evict per request
    solve_share(request.size).unwrap_or(0.45)
}

fn solve_share(_size: usize) -> Option<f64> {
    None
}

struct Request;

impl Request {
    fn size(&self) -> usize {
        0
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: the analytic solve is the SEED, clamped; calibration runs
// ONCE per model on real tiles (the GPU fed the dtype the workload
// really sees) and the result STICKS as the compiled set's lineage tag;
// an explicit override tags PLAIN and is never reused as calibrated;
// unknown silicon keeps the seed; the plateau is ~6% wide, so landing
// near is enough — never interpolate across chips.
const SEED_CLAMP: (f64, f64) = (0.25, 0.85);

enum ShareTag {
    Calibrated(f64),  // reused by every later build of the set
    Plain(f64),       // an explicit override or an uncalibrated seed: never reused
}

fn solve_seed(ane_tflops: f64, gpu_tflops: f64) -> f64 {
    let s = ane_tflops / (ane_tflops + gpu_tflops); // s* = A / (A + G)
    s.clamp(SEED_CLAMP.0, SEED_CLAMP.1)
}

fn calibrate_once(model: &Model, seed: f64, probe: &Probe) -> ShareTag {
    if probe.explicit_override.is_some() {
        // "--ane-split always wins" — and is never cached as calibrated
        return ShareTag::Plain(probe.explicit_override.unwrap());
    }
    let share = balance_two_timings(model, seed, probe.dtype_of_stream());
    ShareTag::Calibrated(share) // the set's `calibrated share=X` lineage tag
}

struct Model;

struct Probe {
    explicit_override: Option<f64>,
    dtype_of_stream: fn() -> &'static str,
}

fn balance_two_timings(_m: &Model, _seed: f64, _dtype: &str) -> f64 {
    0.45
}"#
                .to_string(),
            family: AneFamily::Placement,
            notes: Some(format!(
                "THE MEASURED CLASS, verbatim: \"The ANE split's optimum is per SILICON \
                 (ane.defaultShare; M4 channel 0.45 → 311/304, 0.50 regresses; M3 Ultra 0.45 = \
                 nothing, 0.35 +8.5%/+13.7%). A share change needs its own A/B, never \
                 interpolation.\" And the calibration law: \"The LM prefill share is a \
                 per-silicon table because the seam races a live decode; a denoise step is a \
                 batch job, and its optimum is per (chip, workload). Balance point s* = A / \
                 (A + G) ... That solve is only the SEED ... Calibration runs ONCE per model: \
                 every later build reuses the share of its most recently used compiled set \
                 (ane.cachedShare, the entry's `calibrated share=X` lineage tag; an explicit \
                 share or an uncalibrated seed tags plain `share=X` and is never reused; a \
                 build-time cache hit re-tags the entry). A re-solve per build recompiled + \
                 pruned the whole set on every request size and on probe jitter.\" THE FOUR \
                 SUB-LAWS: (1) per-silicon-per-workload table, never interpolated (the same \
                 formula lands 0.85/0.60/0.45 across models on ONE chip); (2) solve-once-then-
                 stick (the tagged-share cache — and the artifact-keying parameter is resolved \
                 ONCE per model, never per request: per-request re-solve thrashes the compile \
                 cache through M3's eviction path); (3) the override/cached distinction (an \
                 explicit share is a Plain tag — an operator's number must not masquerade as a \
                 measured one on later builds); (4) the seed's honest fallback (unknown silicon \
                 keeps 0.45; the plateau is ~6% wide, so landing near is enough — precision \
                 beyond the plateau is waste). MEASUREMENT POSTURE: \"Judge a calibrated share in \
                 a boot WITHOUT a cold compile: the requests after one run 3-5% slow for a \
                 while.\" KINSHIPS (adjudicated): ane-split-calibration-fit-bandwidth-interference-term \
                 (this corpus, B376) owns the WITHIN-op calibration FIT T(s) = max(A, G + u x A) \
                 with the argmin search — THIS owns the share's LIFETIME (per-silicon table, \
                 solve-once-stick, override-vs-cached tags, never-interpolate); \
                 measured-round-cost-spec-controller (kernel, B398) owns a measured per-bucket \
                 table with contamination guards — same measured-table law, different axis \
                 (round cost per shape vs split share per silicon); \
                 per-generation-plan-table-unmeasured-generation-fallback (kernel, B376-anemll) \
                 owns the unmeasured-generation fallback — THIS's unknown-silicon 0.45 seed is \
                 that law's instance; launch-heuristic-pinned-to-calibration-arch (kernel) pins \
                 a heuristic to the calibration arch — kin in the per-arch posture. Regime: \
                 host/accelerator split ratios keying compiled artifacts. ANTI-REGIME: shares \
                 with no compiled artifact downstream (re-solving is cheap, no thrash); a single \
                 chip family with the workload mix pinned. {ANNOTATION}. Sources: {MLXS} (the \
                 ANE prefill rules' per-silicon bullet + the \"Media offload share\" section)."
            )),
        },

        // ── M6 (B) — a full-activation adapter makes the partial split ineligible ──
        AneRule {
            rule_id: "ane-full-activation-adapter-declines-split".to_string(),
            ane_concept: "an adapter (LoRA-class) summed at forward from the FULL activation makes \
                          a partial accelerator split INELIGIBLE — the block declines to the GPU \
                          whole until the adapter is folded into the snapshot; a fused-projection \
                          consumer of the same full activation declines the same way"
                .to_string(),
            trigger: "an eligibility check for a partial accelerator offload (channels or rows \
                      split across engines) on a block that carries a forward-time adapter, or \
                      whose projections are fused into one weight the split would half-own"
                .to_string(),
            buggy_code: r#"// The split eligibility check reads only the weights being split.
// The LoRA is applied AT FORWARD by summing over the FULL activation —
// but the split hands the accelerator only half of it, so the adapter's
// sum reads an activation the split never makes whole. Result: wrong
// deltas on every split block.
fn block_eligible(weights: &Weights, split: &Split) -> bool {
    // only checks the split's own byte/shape budget — never asks what
    // else READS the full activation at forward
    split.fits(weights)
}

struct Weights;

struct Split;

impl Split {
    fn fits(&self, _w: &Weights) -> bool {
        true
    }
}"#
                .to_string(),
            fixed_code: r#"// FIXED: eligibility asks EVERY forward-time consumer of the FULL
// activation. An adapter summed at forward declines the block (the GPU
// runs it whole until the adapter is folded into the snapshot); a
// fused multi-projection weight the split would half-own declines too.
fn block_eligible(weights: &Weights, split: &Split, adapter: Option<&Adapter>) -> Eligibility {
    if adapter.is_some() {
        // the adapter's sum reads the FULL activation — half never
        // leaves the accelerator program: decline until folded
        return Eligibility::Declined("adapter summed at forward from the full activation");
    }
    if weights.projections_fused() && split.partitions_projection_inputs() {
        // combined-proj archs stay GPU: the fused weight is one forward
        // consumer the split cannot half-serve
        return Eligibility::Declined("fused projections read the full activation");
    }
    if split.fits(weights) {
        Eligibility::Eligible
    } else {
        Eligibility::Declined("split does not fit the block's budget")
    }
}

enum Eligibility {
    Eligible,
    Declined(&'static str),
}

struct Weights;

impl Weights {
    fn projections_fused(&self) -> bool {
        false
    }
}

struct Split;

impl Split {
    fn fits(&self, _w: &Weights) -> bool {
        true
    }

    fn partitions_projection_inputs(&self) -> bool {
        false
    }
}

struct Adapter;"#
                .to_string(),
            family: AneFamily::GraphShape,
            notes: Some(format!(
                "THE MEASURED CLASS, verbatim: \"A LoRA-attached block DECLINES (aneBlockEligible \
                 in both): the adapter is summed at forward from the FULL activation, half of \
                 which never leaves the ANE program. Turbo binds blocks.N.mlp.fc1/fc2, so H3's \
                 fast path is GPU-only until the LoRA is folded into the int8 snapshot.\" And the \
                 fused-consumer instance: \"combined-proj archs stay GPU\" (a fused qkv/z \
                 projection is one forward consumer of the full input — the split half-owns it \
                 and cannot serve it). The w8a8 layer carries the SAME law on the GPU side: \
                 \"with an adapter attached the plain GEMM is split instead (forwardPair)\" — the \
                 stacked/interleaved fusion that serves the adapter-free path is REPLACED by the \
                 split form when an adapter attaches. THE LAW: eligibility is a property of the \
                 block's FULL-ACTIVATION CONSUMERS, not of the split's own budget — enumerate \
                 every forward-time reader of the whole activation (adapters, fused \
                 projections, residual taps) and decline on any one the split cannot half-serve; \
                 the remedy is not a smaller split, it is FOLDING the consumer into the snapshot \
                 (adapter into the int8 snapshot) and re-eligibility. Clean (no corpus owner): \
                 the nearest kin are the split/fusion families in kernel_opt, none of which \
                 owns the adapter-vs-split eligibility axis. Regime: split-offload planners over \
                 adapter-carrying or fused-projection blocks. ANTI-REGIME: adapters applied to \
                 the SPLIT portion only (the split owns the whole consumer); adapters pre-folded \
                 at load (the snapshot is whole again — re-check eligibility after the fold). \
                 {ANNOTATION}. Sources: {MLXS} (the Media offload section's LoRA bullet + the \
                 GDN fused-conv bullet + the w8a8 Fused-weights bullet)."
            )),
        },

        // ── M7 (B-) — the deep-K reduction chunks in-graph past the cliff ──
        AneRule {
            rule_id: "ane-deep-k-reduction-chunked-in-graph".to_string(),
            ane_concept: "a single accelerator conv/matmul whose reduction dim K runs deep pays \
                          a measured cliff — K is CHUNKED IN-GRAPH into slabs past the cliff \
                          width, ONE compiled tile width carries every chunk, and tails run the \
                          host GPU"
                .to_string(),
            trigger: "an accelerator program whose down-projection/conv K width runs thousands \
                      of inputs deep (a large-FFN model's down conv) compiled as one operator"
                .to_string(),
            buggy_code: r#"// The down conv compiles as ONE operator with its natural K:
// a single K=17408 conv runs at 4.3 TFLOPS on the accelerator —
// a 2.6x cliff under the engine's 11.8 TFLOPS rate.
fn build_down_conv(k: usize) -> Program {
    compile_conv(/* out */ 1, /* K */ k) // one deep operator
}

fn compile_conv(_out: usize, _k: usize) -> Program {
    Program
}

struct Program;"#
                .to_string(),
            fixed_code: r#"// FIXED: K is chunked IN-GRAPH into slabs (partial sums carried
// between chunks), past the measured cliff width (~4.6k inputs) —
// ONE compiled tile width serves every chunk (a second width is a
// second program set: disk + compile time), and the tail chunk that
// does not fill a slab runs the host GPU instead.
const K_CHUNK: usize = 4_600; // past the measured cliff

fn build_down_conv_chunked(k: usize) -> Program {
    // one tile width for every chunk — the graph carries the partial
    // sums between slabs; the accumulator add is exact in fp32
    compile_conv_chunked(/* K */ k, /* chunk */ K_CHUNK)
}

fn compile_conv_chunked(_k: usize, _chunk: usize) -> Program {
    Program
}

struct Program;"#
                .to_string(),
            family: AneFamily::GraphShape,
            notes: Some(format!(
                "THE MEASURED CLASS, verbatim: \"A single deep-K ANE conv is a 2.6x cliff: K-chunk \
                 past ~4.6k inputs in-graph; ONE compiled tile width (denseMLPMaybeAne), tails \
                 run GPU.\" And the numbers: \"A single K=17408 down conv runs at 4.3 TFLOPS; \
                 K-chunking into slabs restores 11.8.\" THE THREE SUB-LAWS: (1) chunk IN-GRAPH \
                 (the graph carries partial sums between slabs — not a host-side loop over \
                 separate programs, which re-pays the per-call gap the ane-prediction-gap rule \
                 owns); (2) ONE tile width (every chunk compiles into the same program set — a \
                 width per chunk class is disk + compile time for no rate); (3) tails run GPU \
                 (the ragged last slab is the host's, never a second accelerator shape). \
                 Regime: accelerator graph builders over large-FFN/down-projection operators \
                 (ANE/CoreML-class; the cliff width is per-engine — re-measure, the ~4.6k is ONE \
                 engine generation's). ANTI-REGIME: K inside the cliff width (one operator is \
                 faster — chunking pays the accumulator adds); hosts where the in-graph partial \
                 carry is unsupported (fall back to operator splits and measure). Clean (no \
                 corpus owner; the deep-K axis is new — the closest kernel kin are the split-K \
                 GEMM families, which split for OCCUPANCY on GPU, not for an engine-rate cliff \
                 on an accelerator graph). {ANNOTATION}. Sources: {MLXS} (ANE prefill rules, \
                 the staging/deep-K bullet); the numbers: {MLXSG} (\"What the ANE build taught\")."
            )),
        },

        // ── M8 (B-) — I/O planes shared per shape class within a unit ──
        AneRule {
            rule_id: "ane-io-planes-shared-per-shape-class".to_string(),
            ane_concept: "an accelerator unit's I/O planes are SHARED across its shape classes — \
                          evals are serial, so one plane per class-role (not per shape) bounds the \
                          footprint (3 surfaces replace 224) and binding memsets nothing"
                .to_string(),
            trigger: "an accelerator unit (program set) whose every shape class binds its own \
                      fresh I/O surfaces, multiplying the resident footprint by the shape count"
                .to_string(),
            buggy_code: r#"// Every shape class binds fresh I/O planes: N shapes x input/output
// planes each, all resident for the unit's lifetime. On a 224-shape
// unit the plane bill alone ran ~32 GB.
struct Unit {
    planes: Vec<Surfaces>, // one Surfaces per shape class
}

struct Surfaces;

impl Surfaces {
    fn bind(&mut self, input: &[f16]) {
        // fresh bind, fresh zero-fill
        zero_fill();
        copy(input);
    }
}

fn zero_fill() {}

fn copy(_input: &[f16]) {}"#
                .to_string(),
            fixed_code: r#"// FIXED: evals are SERIAL (the concurrency contract — see
// ane-concurrent-same-function-silent-corruption for what happens when
// that is violated), so ONE plane per class-role is shared across the
// unit's shape classes: 3 surfaces replace 224, and the bind COPIES
// over the previous contents (memset at bind is paid nowhere).
struct UnitShared {
    // role-keyed, shape-shared: input, output, scratch
    planes: Roles<Surfaces>,
}

struct Roles<T> {
    input: T,
    output: T,
    scratch: T,
}

impl UnitShared {
    fn eval(&mut self, input: &[f16], shape: Shape) {
        // copy OVER the plane (no memset): evals are serial, so the
        // previous eval's contents are dead by construction
        copy_into(&mut self.planes.input, input);
        run_program_for(shape, &self.planes);
    }
}

struct Surfaces;

struct Shape;

fn copy_into(_dst: &mut Surfaces, _src: &[f16]) {}

fn run_program_for(_shape: Shape, _planes: &Roles<Surfaces>) {}"#
                .to_string(),
            family: AneFamily::Lifecycle,
            notes: Some(format!(
                "THE MEASURED CLASS, verbatim: \"ANE I/O planes shared per SHAPE CLASS within a \
                 unit (evals serial; 3 surfaces replace 224; never memset at bind).\" And the \
                 bill: \"evals are serial, so I/O planes are shared per shape class (bill 32 → \
                 20 GB).\" THE PREMISE IS THE CONCURRENCY CONTRACT: sharing is legal BECAUSE \
                 evals are serial — the previous eval's plane contents are dead by construction, \
                 so the bind COPIES over them and memsets nothing (the memset is paid at every \
                 bind under the naive shape). The concurrency rule (\"another side of the same \
                 constraint\" (ane-concurrent-same-function-silent-corruption, A10) owns the \
                 corruption when two evals overlap; THIS owns the footprint win the serial \
                 contract buys. KINSHIPS (adjudicated): dual-bank-scratch-retention-per-round-class \
                 (kernel entries_gpu_138, B379) retains PER-CLASS scratch banks and swaps the \
                 active bank on class flip — THAT separates classes with disjoint row regimes \
                 into banks; THIS shares ONE set of role planes ACROSS classes because the \
                 evals serialize (the axis flips: bank-per-class vs plane-shared-across-classes; \
                 the discriminator is whether the class transitions INTERLEAVE — they do not \
                 here, evals are serial); scratch-bank-isolates-one-shot-operands-from-hot-cache \
                 (kernel) isolates one-shot operands — a residency-class axis, not a shape-class \
                 axis. Regime: accelerator program sets evaluated serially over many shape \
                 classes. ANTI-REGIME: concurrent evaluation of the unit (the planes race — see \
                 A10; give each in-flight eval its own surfaces); shape classes whose lifetimes \
                 overlap (streams/slots — per-slot planes, then share across shapes within the \
                 slot). {ANNOTATION}. Sources: {MLXS} (ANE prefill rules, the I/O planes bullet); \
                 the bill: {MLXSG} (\"The share 0.35 'tiling cliff' was every eval failing\")."
            )),
        },

        // ── Batch 401 — the procedure-bank wrong-API-twin (the swallowed-  ──
        // ── slowdown law; Research 434's staged candidate #1, executed)    ──
        AneRule {
            rule_id: "ane-procedure-bank-wrong-symbol-api-swallowed-slowdown".to_string(),
            ane_concept: "a capability query that answers a DEFAULT (0 / empty) instead of \
                          erroring converts a functional break into a silent perf loss: resolve \
                          per-procedure symbol indices from the OWNER object (the in-memory \
                          wrapper is not a subclass of the model carrying the selectors), \
                          require the returned count, and refuse a multi-procedure bank BY NAME \
                          when the capability is unreachable — never degrade into identity \
                          indices that serve only procedure 0"
                .to_string(),
            trigger: "an accelerated-device program bank (N > 1 procedures sharing one weight \
                      buffer, each its own procedureIndex) whose host resolves per-procedure \
                      I/O symbol indices through a selector on the wrapper object — especially \
                      a twin spelling that answers 0/an empty array for every procedure — or \
                      falls back to the identity mapping when the query cannot answer"
                .to_string(),
            buggy_code: r#"// NAIVE: the symbol query rides the wrapper the host happens to hold.
// The twin selector answers 0 for every procedure instead of erroring,
// so the code falls back to identity indices 0..count-1 — which serve
// procedure 0 and fail EVERY other one. The GPU recompute absorbs each
// failure, so the break reads as 'the bank is slow', not 'the bank is
// broken': the only observable is the failure counter.
struct AneInMemoryModel {
    model: Option<AneModel>, // the wrapper OWNS the model; it is NOT one
}

impl AneInMemoryModel {
    fn symbol_indices(&self, proc: usize, count: usize) -> Vec<usize> {
        // wrong twin on the wrong receiver: answers 0, never errors
        match input_symbol_indices_for_procedure(proc) {
            Some(v) if v.len() == count => v,
            _ => (0..count).collect(), // identity: right for proc 0, wrong for > 0
        }
    }
}

fn input_symbol_indices_for_procedure(_proc: usize) -> Option<Vec<usize>> {
    Some(Vec::new()) // the swallowed-slowdown shape: a default, not an error
}"#
                .to_string(),
            fixed_code: r#"// FIXED: resolve from the capability's OWNER (unwrap the wrapper
// first — the right selector on the wrong receiver is unreachable, not
// wrong); require the returned count to match; and when the capability
// is unreachable for a shape that cannot work without it, refuse the
// create BY NAME so the caller's split ladder walks down to banks of one.
struct AneInMemoryModel {
    model: Option<AneModel>,
}

impl AneInMemoryModel {
    fn inner_model(&self) -> Option<&AneModel> {
        self.model.as_ref() // the selectors live on the model, not the wrapper
    }

    fn has_symbol_api(&self) -> bool {
        self.inner_model().is_some() // query the owner, never the wrapper
    }

    fn symbol_indices(&self, proc: usize, count: usize) -> Option<Vec<usize>> {
        let model = self.inner_model()?;
        let v = model.symbol_indices_for_procedure(proc)?;
        (v.len() == count).then_some(v) // count-validated before use
    }
}

fn create_bank(procs: &[Proc], m: &AneInMemoryModel) -> Result<Handle, String> {
    // a bank of one is fine on identity; a bank of N > 1 without the
    // capability is refused BY NAME — the swallowed-slowdown posture is
    // the defect this gate exists to prevent
    if procs.len() > 1 && !m.has_symbol_api() {
        return Err(format!(
            "exposes no per-procedure symbol indices, so a bank of {} cannot be dispatched",
            procs.len()
        ));
    }
    Ok(Handle::new(procs, m))
}

struct AneModel;

impl AneModel {
    fn symbol_indices_for_procedure(&self, _proc: usize) -> Option<Vec<usize>> {
        None
    }
}

struct Proc;

struct Handle;

impl Handle {
    fn new(_procs: &[Proc], _m: &AneInMemoryModel) -> Self {
        Self
    }
}"#
                .to_string(),
            family: AneFamily::Lifecycle,
            notes: Some(format!(
                "THE MEASURED CLASS, verbatim (docs/reference.md:847): \"symbol indices READ \
                 from `procedureInfoForProcedureIndex:` on the `_ANEModel` behind `-model` \
                 (NOT `inputSymbolIndicesForProcedureIndex:`, which answers 0 and fails every \
                 procedure >0 as a swallowed slowdown; `eval_failures` is the tell).\" And the \
                 bill (lib/ane/ane_bridge.m:302-307): \"a bank's procedures do NOT share one \
                 symbol numbering — the identity mapping serves procedure 0 and fails EVERY \
                 other one with a bare \"Program Inference error\" (measured 2026-08-20: 200 \
                 of 252 evals dead on a 42-procedure bank).\" THE RECEIVER TRAP (:307-311): \
                 \"The selectors live on `_ANEModel`, and `_ANEInMemoryModel` is NOT a subclass \
                 of it (checked: `instancesRespondToSelector:` is NO) — it OWNS one, reachable \
                 through its `-model` accessor once the net is loaded.\" THE FAIL-CLOSED GATE \
                 (:335-341 + :580-585): \"A bank of more than one procedure CANNOT be served \
                 without it — identity indices make every procedure past the first fail at \
                 eval, which reads as a slow offload rather than a broken one — so the create \
                 is refused by name and the caller's split ladder walks down to banks of \
                 one.\" THE SILENCE CONTRACT (tests/test_ane_prefill.sh:120-122): \"A failed \
                 eval never fails the REQUEST — the seam recomputes that chunk on the GPU — \
                 so the only evidence is this line. Silence here is the contract.\" And the \
                 misattribution it produced (docs/gotchas/engine-mlx.md:759-762): \"identity \
                 indices made 200 of 210 evals fail into the GPU fallback, reading as 'banks \
                 cost 23%'.\" DECISION CONTENT: this is a PARTIAL zero — procedure 0 succeeds \
                 under identity indices, so a single-procedure smoke arm reads green; the \
                 capability path must be exercised per-procedure (N > 1) before any bank arm \
                 claims success, and the failure counter must sit beside the success signal \
                 (/props per-unit evals + eval_failures, asserted zero in every test arm) \
                 because the request path is contractually silent. KINSHIPS (adjudicated): \
                 degenerate-output-invalidates-throughput-claim (rust_perf entries_84) owns \
                 the WRONG-OUTPUT axis (degraded text invalidates the claim) — THIS ships \
                 correct outputs (every failed eval is recovered by an exact GPU recompute) \
                 and only the claim of acceleration is false: a capability-boundary failure \
                 wearing a perf-regression costume; the kernel fail-open family (gpu_24, \
                 gpu_50) fails OPEN because the query's subject is a MEASUREMENT — THIS \
                 refuses fail-CLOSED because the subject is a LOAD-BEARING CAPABILITY the \
                 dispatch cannot work without (the polarity flips with the subject); \
                 ane-int8-matmul-shape-silent-wrong-gate owns silent-WRONG (bad data, no \
                 error — host-reference gate) and ane-row-pitch-grid-refused-at-plan-time \
                 owns silent-FAIL (compiles, then every eval errors LOUD) — THIS owns \
                 silent-DEGRADE: right data, wrong device, no error ever reaches the caller; \
                 silent-zero-launch-row-chunked-past-grid-z-ceiling (kernel gpu_137/138) is \
                 a launch doing NO work — THIS is a PARTIAL zero. Regime: multi-procedure \
                 program banks served through an exact-but-slower recovery path. \
                 ANTI-REGIME: a procedure that genuinely takes zero inputs (the empty index \
                 array IS the answer); single-procedure programs where identity indices are \
                 the contract (the refusal is gated on procedure_count > 1 at the source, \
                 ane_bridge.m:582); postures where unavailability is already typed and \
                 surfaced. {ANNOTATION}. Sources: {MLXS} (docs/reference.md:847, the \
                 procedure-bank bullet) + {MLXSG} (the eval-fallback misattribution) — \
                 quotes re-verified at the pin at this batch's T0."
            )),
        },
    ]
    .into_iter()
    .collect();

    Corpus::new(entries)
}

#[cfg(test)]
mod tests;
