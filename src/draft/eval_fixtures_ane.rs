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
    // ── Batch 376 wave (the anemll branch-delta, Research 421) ──────────────
    (
        "ane-int8-matmul-shape-silent-wrong-gate",
        r#"fn serve_ffn(x: &[f32], form: Form) -> Vec<f32> {
    // the new INT8 form benchs faster; adopted on speed alone
    int8_matmul_pair(x, form) // rel 0.98 wrong at this shape, no signal
}"#,
    ),
    (
        "ane-stage-parity-gate-cpu-vs-ane-same-model",
        r#"fn accept(build: &Build) -> bool {
    // whole-model cosine passes; per-stage numerics unchecked
    cosine(out_probs(build), ref_probs(build)) > 0.99
}"#,
    ),
    (
        "ane-hybrid-pipeline-block-granularity-not-layer-interleave",
        r#"fn run_layer(x: &Tensor) -> Tensor {
    // within-layer channel split: the second unit idles through the first's half
    let ffn_ane = ane_ffn_channels(x, 0.76);
    gpu_mixers(add(ffn_ane, gpu_ffn_channels(x, 0.24)))
}"#,
    ),
    (
        "ane-prediction-gap-fewer-bigger-calls",
        r#"fn run_blocks(x: &Tensor, n_blocks: usize) -> Vec<Tensor> {
    // one 64-row call per block: N gaps at 0.23 ms each
    (0..n_blocks).map(|b| ane_ffn_64rows(x, b)).collect()
}"#,
    ),
    (
        "ane-concurrent-same-function-silent-corruption",
        r#"fn serve_workers(function: Function, workers: usize) {
    // each worker clones the binding; the SAME function underneath
    (0..workers).map(|_| {
        let f = function.clone_binding();
        spawn(move || publish(f.evaluate(next_input())))
    });
}"#,
    ),
    (
        "ane-host-output-pool-unbounded-call-bound",
        r#"fn decode_loop(chunk: &Chunk, tokens: usize) -> Vec<Token> {
    // unbounded: every call allocates fresh outputs the runtime never frees
    (0..tokens).map(|_| read_token(&chunk.call(next_state()))).collect()
}"#,
    ),
    (
        "ane-package-size-build-gate-before-kernel-panic",
        r#"fn load(path: &str) -> Model {
    // no size gate, no version check — trust the runtime (panics the kernel past 1.1 GB)
    load_unchecked(path)
}"#,
    ),
];
