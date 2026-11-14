//! Plan 377 — `ane` domain eval fixtures (hand-authored variants of the six
//! seed rules; the `eval_fixtures_dist` convention — variant identifiers,
//! the same anti-pattern shape, one span per rule). Pure data; consumed by
//! `super::retrieval_eval` under the `ane` feature.
//!
//! Honest confusability note: R3/R4 both convert `CoreML` graphs and share
//! `MLComputeUnits`-class vocabulary — their fixtures are legitimately close
//! in the retrieval space; the domain's demonstrability constant measures
//! exactly that selection difficulty.

// Uniform `r#"..."#` raw-string style is deliberate for fixture data.
#![allow(clippy::needless_raw_string_hashes)]

/// (`rule_id`, `span_code`) — one span per rule; each genuinely instantiates
/// the rule's anti-pattern (semantic label, authored against the corpus
/// entry, variant names so the fixture cannot be a corpus substring match).
pub const ANE_SPANS: &[(&str, &str)] = &[
    (
        "ane-fixed-window-mask-stateless-streaming",
        r#"struct ChunkResumer {
    ring: Vec<f32>,
}

impl ChunkResumer {
    fn push(&mut self, frame: &[f32]) -> Vec<f32> {
        self.ring.extend_from_slice(frame);
        self.ring.iter().map(|v| v * 0.5).collect()
    }
}"#,
    ),
    (
        "ane-fp16-norm-pow2-prescale-overflow",
        r#"fn half_precision_rms(x: &[f32]) -> f32 {
    let acc: f32 = x.iter().map(|v| v * v).sum();
    acc.sqrt()
}"#,
    ),
    (
        "ane-fp16-norm-pow2-prescale-overflow",
        r#"// second variant: the squared sum over a raw hot residual, no prescale
fn energy_of(stream: &[f32]) -> f32 {
    stream.iter().map(|v| v * v).sum::<f32>().sqrt()
}"#,
    ),
    (
        "ane-compute-plan-pin-cpu-and-neural-engine",
        r#"let opts = LoadOptions::default();
// units left at the catch-all default — the runtime picks the GPU
let handle = ml_load("enc.mlmodelc", opts.with_units(Units::Everything));"#,
    ),
    (
        "ane-compute-plan-pin-cpu-and-neural-engine",
        r#"fn build_plan(model: &CompiledPlan) -> Schedule {
    Schedule::any_device(model) // placement unchecked; ANE residency ~2%
}"#,
    ),
    (
        "ane-graph-restructure-for-ane-lowering",
        r#"fn attention_projections(x: &Tensor, wq: &Tensor, wk: &Tensor, wv: &Tensor) -> Vec<Tensor> {
    vec![dense(x, wq), dense(x, wk), dense(x, wv)]
}

fn dense(x: &Tensor, w: &Tensor) -> Tensor {
    x.mul(w)
}"#,
    ),
    (
        "ane-precision-dsp-stays-cpu-fp32",
        r#"fn encode_voice(samples: &[f32]) -> Vec<f32> {
    let frames = half_fft(samples); // complex exp at half width
    decoder_on_accelerator(&frames)
}"#,
    ),
    (
        "ane-weight-palette-depth-retention-gate",
        r#"// shipped 4-bit k-means palette because the blog post said 6-bit was fine
fn quantize_encoder(w: &[f32]) -> Vec<u8> {
    kmeans_palette(w, 4)
}"#,
    ),
];
