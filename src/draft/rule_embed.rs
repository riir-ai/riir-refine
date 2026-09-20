//! Issue 126 lane (a) — the sigmoid-NCE two-tower span↔rule embeddings.
//!
//! Rule selection IS goal-conditioned value (the CRL critic with rule-as-goal,
//! arXiv:2206.07568): `φ(span)ᵀψ(rule) ≈ P(rule fixes span)`. The supervision
//! is machine-checked — `(span, rule_that_actually_fixed_it)` from the
//! labeled eval fixtures + each rule's own corpus snippets (self-pairs), with
//! a RULE-LEVEL deterministic split (blake3 of the rule id) withholding ~25%
//! of rules and their fixture spans from training. Rule-level (not
//! fixture-level) is load-bearing: perf/kernel fixtures are authored variants
//! of the corpus `buggy_code` shapes, so a withheld fixture's rule self-pair
//! would leak the association the OOD gate is supposed to measure.
//!
//! Loss: **sigmoid-NCE** — `log σ(φᵀψ⁺/τ) + Σ log(1−σ(φᵀψ⁻/τ))`, in-batch
//! negatives (pairs sharing a rule are positives, skipped as negatives). NOT
//! softmax-InfoNCE: the house rule (sigmoid never softmax) and Bench 048
//! (sigmoid reaches positive margin at `d ≈ log n`; `InfoNCE` needs
//! `Θ(n^(1/3))`). τ = [`SIGMOID_TAU`] is shared by training and runtime
//! scoring — the two must agree or the shipped score distribution is not the
//! trained one.
//!
//! Ship: **frozen i8 tables** (per-matrix absmax → ×127) in a versioned,
//! BLAKE3-verified artifact ([`decode_verified`] refuses a bad hash, wrong
//! version, wrong dims, or truncation). The runtime stays modelless: one
//! hashed bag + one matvec for `φ`, one dot + σ per rule — the trained
//! weights live in the frozen snapshot, never mutated (the freeze/thaw
//! pattern; the trainer is offline, the loader never writes).
//!
//! Posture: `PoC` (`rule_embed_poc`, implies `choice_scorer_poc` — its `Rng`
//! and warmup/cosine schedule are reused). The shipped suggest lane stays
//! modelless until the `GOAT` gates pass; `run_split_eval` (`retrieval_eval`) is
//! the measurement. Determinism: fixed op order, seeded `SplitMix64` init —
//! the ARTIFACT is committed, so the runtime never depends on retraining.

// Scoped lint posture for a hand-rolled linear-algebra module (the
// choice_scorer precedent, module-scope because the whole file is one
// kernel): the casts are dimension/count/index arithmetic (≤ 2²⁴, exact in
// f32); i/j/b/d are matrix indices; the indexed loops ARE the kernel.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::needless_range_loop
)]

use std::collections::HashMap;

use super::choice_scorer::{AdamW, Rng};
use crate::span_embed::fill_hashbag;

/// Input feature space: the hashed unigram+bigram bag width (Weinberger
/// feature-hashing; the span side and the rule side share the tokenization
/// law via [`crate::span_embed::fill_hashbag`]).
pub const FEATURE_BUCKETS: usize = 4096;

/// Embedding dimensionality — the paper's Table 2 pin (d = 64), matching the
/// `span_embed` width class.
pub const EMBED_DIM: usize = 64;

/// The sigmoid-NCE temperature. SHARED by training and runtime scoring: the
/// shipped `score_rule` distribution must be the trained one.
pub const SIGMOID_TAU: f32 = 0.1;

/// Artifact format version — bumped on ANY wire change.
pub const FORMAT_VERSION: u32 = 1;

const MAGIC: &[u8; 4] = b"RLEM";

/// Numerically stable σ: two branches so `e^z` never overflows.
fn sigmoid(z: f32) -> f32 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

/// The deterministic RULE-LEVEL fixture split: every rule whose blake3 id
/// hash reads `blake3(id)[0].is_multiple_of(4)` is WITHHELD from training
/// and measured as the withheld-pair OOD gate (~25% by construction — 64 of
/// 256 byte values). Keyed by rule id, so the split is stable as the corpus
/// grows.
#[must_use]
pub fn fixture_is_withheld(rule_id: &str) -> bool {
    blake3::hash(rule_id.as_bytes()).as_bytes()[0].is_multiple_of(4)
}

// ---------------------------------------------------------------------------
// The runtime view — dequantized f32 over the frozen i8 artifact
// ---------------------------------------------------------------------------

/// One frozen rule row: the id and its ψ direction (dequantized at load).
#[derive(Debug, Clone)]
pub struct RuleRow {
    pub id: String,
    psi: Vec<f32>,
}

impl RuleRow {
    /// The rule's ψ direction (dequantized f32).
    #[must_use]
    pub fn psi(&self) -> &[f32] {
        &self.psi
    }
}

/// The frozen two-tower tables — decoded once, then read-only (the runtime
/// never mutates weights; the artifact is the freeze/thaw snapshot).
#[derive(Debug, Clone)]
pub struct TrainedTables {
    /// Feature-bucket count the tables were trained at (≤ [`FEATURE_BUCKETS`]).
    pub buckets: usize,
    /// Embedding width (== [`EMBED_DIM`]; decode refuses anything else).
    pub dim: usize,
    /// The shared σ temperature (== [`SIGMOID_TAU`] for this build).
    pub tau: f32,
    span_proj: Vec<f32>,
    rules: Vec<RuleRow>,
    index: HashMap<Box<str>, u32>,
}

impl TrainedTables {
    /// Distinct rules carrying a ψ row.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// The rules, in artifact (id-sorted) order.
    #[must_use]
    pub fn rules(&self) -> &[RuleRow] {
        &self.rules
    }

    /// Project a span's text through the frozen span tower: hashed bag →
    /// matvec. Empty/whitespace-only text → the zero vector (σ(0) = 0.5 for
    /// every rule — a constant term, order-preserving).
    #[must_use]
    pub fn project_span(&self, text: &str) -> [f32; EMBED_DIM] {
        let mut acc = [0.0_f32; FEATURE_BUCKETS];
        fill_hashbag(text, self.buckets, &mut acc);
        let mut phi = [0.0_f32; EMBED_DIM];
        for (b, &w) in acc.iter().enumerate() {
            if w == 0.0 {
                continue;
            }
            let row = &self.span_proj[b * self.dim..(b + 1) * self.dim];
            for (d, &wr) in row.iter().enumerate() {
                phi[d] += w * wr;
            }
        }
        phi
    }

    /// The trained score for one rule against a projected span:
    /// `σ(φᵀψ/τ)`. `None` when the rule has no ψ row (graceful degradation —
    /// the caller falls back to the untrained similarity for that rule) or
    /// when `phi` was projected at a different width.
    #[must_use]
    pub fn score_rule(&self, phi: &[f32], rule_id: &str) -> Option<f32> {
        debug_assert_eq!(phi.len(), self.dim);
        if phi.len() != self.dim {
            return None;
        }
        let &row = self.index.get(rule_id)?;
        let psi = &self.rules[row as usize].psi;
        Some(sigmoid(dot(phi, psi) / self.tau))
    }
}

/// Dequantize: `b * scale` — the exact inverse of [`quantize`], whose
/// `scale = absmax/127` maps absmax to 127. (The `/127.0` form here was the
/// landed-and-caught bug: mixed conventions shrank every decoded weight
/// 127×, dots collapsed to ≈0 and every score read the σ(0) = 0.5 null —
/// measured via the self-pair probe before this fix.)
fn dequant(b: i8, scale: f32) -> f32 {
    f32::from(b) * scale
}

/// The SHIPPED frozen tables, compiled into the binary via `include_bytes!`
/// (hermetic: no path resolution, no IO at load). Decode-verified on first
/// use — a tampered or foreign artifact fails LOUDLY, never scores.
///
/// # Panics
///
/// On the first call, if the embedded artifact fails its own verification
/// (hash, version, dims) — a build/publish integrity fault, never a runtime
/// input, so the failure is a panic rather than a degraded score.
///
/// Provenance: trained by `examples/rule_embed_poc.rs --train` at the
/// committed trainer + corpus state; sha256 of the artifact is pinned in
/// `.benchmarks/099_rule_embed_sigmoid_nce.md`. Retraining moves the bytes —
/// the standing gate test re-pins beside it.
#[cfg(feature = "rule_embed_poc")]
pub fn embedded_tables() -> &'static TrainedTables {
    static TABLES: std::sync::OnceLock<TrainedTables> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        const RAW: &[u8] = include_bytes!("../../assets/rule_embed_frozen_v1.bin");
        decode_verified(RAW)
            .unwrap_or_else(|e| panic!("embedded rule-embed artifact refused: {e}"))
    })
}

// ---------------------------------------------------------------------------
// Artifact wire format — magic | header | span tower | ψ rows | blake3
// ---------------------------------------------------------------------------

/// Verify + decode the frozen artifact: magic, version, dims, and the
/// BLAKE3 over the whole payload are ALL checked; any mismatch (corruption,
/// truncation, tamper, foreign build) refuses with a named error — never a
/// best-effort read.
///
/// # Errors
///
/// Named `String` for every refusal class (short, bad magic, hash mismatch,
/// version, dims, bounds, duplicate rule id, trailing bytes, invalid UTF-8
/// rule id).
pub fn decode_verified(bytes: &[u8]) -> Result<TrainedTables, String> {
    // header(28) + hash(32) is the floor; any real artifact is larger.
    if bytes.len() < 60 {
        return Err(format!("artifact too short: {} bytes", bytes.len()));
    }
    let (payload, hash) = bytes.split_at(bytes.len() - 32);
    if payload.len() < 28 || &payload[..4] != MAGIC {
        return Err(format!("bad magic (want {MAGIC:?})"));
    }
    let got: [u8; 32] = hash
        .try_into()
        .map_err(|_| format!("hash length {} != 32", hash.len()))?;
    if blake3::hash(payload).as_bytes() != &got {
        return Err("blake3 mismatch — corrupt, truncated, or tampered artifact".into());
    }
    let mut r = Reader::new(payload);
    r.take(4)?; // magic, already checked
    let version = r.u32()?;
    if version != FORMAT_VERSION {
        return Err(format!("format version {version} != {FORMAT_VERSION} — retrain"));
    }
    let buckets = r.u32()? as usize;
    let dim = r.u32()? as usize;
    let tau = r.f32()?;
    let n_rules = r.u32()? as usize;
    if dim != EMBED_DIM {
        return Err(format!("dim {dim} != this build's EMBED_DIM {EMBED_DIM}"));
    }
    if buckets == 0 || buckets > FEATURE_BUCKETS {
        return Err(format!("buckets {buckets} outside 1..={FEATURE_BUCKETS}"));
    }
    if !tau.is_finite() || tau <= 0.0 {
        return Err(format!("tau {tau} not finite-positive"));
    }
    let span_scale = r.f32()?;
    if !span_scale.is_finite() || span_scale <= 0.0 {
        return Err(format!("span scale {span_scale} not finite-positive"));
    }
    let span_i8 = r.take(buckets * dim)?;
    // u8 → i8 is the bit-preserving inverse of the encoder's `b as u8`; the
    // wrap warning is the point — both sides are the same two's-complement
    // byte, round-trip exact by construction.
    #[allow(clippy::cast_possible_wrap)]
    let span_proj: Vec<f32> = span_i8.iter().map(|&b| dequant(b as i8, span_scale)).collect();
    let mut rules = Vec::with_capacity(n_rules.min(65536));
    for _ in 0..n_rules {
        let id_len = r.u32()? as usize;
        if id_len > 4096 {
            return Err(format!("rule id length {id_len} out of bounds"));
        }
        let id = std::str::from_utf8(r.take(id_len)?)
            .map_err(|e| format!("rule id utf-8: {e}"))?
            .to_string();
        let scale = r.f32()?;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(format!("ψ scale {scale} for {id:?} not finite-positive"));
        }
        let psi_i8 = r.take(dim)?;
        #[allow(clippy::cast_possible_wrap)] // bit-preserving inverse of `as u8`
        let psi: Vec<f32> = psi_i8.iter().map(|&b| dequant(b as i8, scale)).collect();
        rules.push(RuleRow { id, psi });
    }
    if r.pos != payload.len() {
        return Err(format!("trailing bytes: {} unconsumed", payload.len() - r.pos));
    }
    let mut index = HashMap::with_capacity(rules.len());
    for (i, row) in rules.iter().enumerate() {
        if index.insert(row.id.as_str().into(), i as u32).is_some() {
            return Err(format!("duplicate rule id {:?}", row.id));
        }
    }
    Ok(TrainedTables {
        buckets,
        dim,
        tau,
        span_proj,
        rules,
        index,
    })
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| "length overflow".to_string())?;
        if end > self.b.len() {
            return Err(format!(
                "truncated: need {n} bytes at {}, have {}",
                self.pos,
                self.b.len() - self.pos
            ));
        }
        let s = &self.b[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(self.u32()?))
    }
}

fn push_u32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn push_f32(v: &mut Vec<u8>, x: f32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// Assemble the canonical artifact from the QUANTIZED pieces (the trainer is
/// the only producer — the runtime view's f32 never re-encodes, because
/// requantization is not guaranteed byte-identical). `rules` must be
/// pre-sorted by id; the sort lives in [`train`] so the bytes are canonical.
fn encode_artifact(
    buckets: usize,
    dim: usize,
    tau: f32,
    span_i8: &[i8],
    span_scale: f32,
    rules: &[(String, f32, Vec<i8>)],
) -> Vec<u8> {
    let mut p = Vec::with_capacity(28 + span_i8.len() + 32);
    p.extend_from_slice(MAGIC);
    push_u32(&mut p, FORMAT_VERSION);
    push_u32(&mut p, buckets as u32);
    push_u32(&mut p, dim as u32);
    push_f32(&mut p, tau);
    push_u32(&mut p, rules.len() as u32);
    push_f32(&mut p, span_scale);
    for &b in span_i8 {
        p.push(b as u8);
    }
    for (id, scale, q) in rules {
        push_u32(&mut p, id.len() as u32);
        p.extend_from_slice(id.as_bytes());
        push_f32(&mut p, *scale);
        for &b in q {
            p.push(b as u8);
        }
    }
    let h = blake3::hash(&p);
    p.extend_from_slice(h.as_bytes());
    p
}

// ---------------------------------------------------------------------------
// The trainer
// ---------------------------------------------------------------------------

/// Trainer knobs (the `choice_scorer` defaults' schedule shape, scaled to the
/// tiny two-tower problem).
#[derive(Debug, Clone, Copy)]
pub struct TrainConfig {
    /// Full-batch steps (the dataset is ≤ a few hundred pairs — full batch
    /// IS the minibatch, deterministically).
    pub steps: usize,
    pub lr: f32,
    /// Decoupled `AdamW` weight decay.
    pub weight_decay: f32,
    pub warmup: usize,
    /// Cosine floor as a fraction of `lr`.
    pub min_lr_ratio: f32,
    /// `SplitMix64` seed for the init (`Rng::new`).
    pub seed: u64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            steps: 3000,
            lr: 0.02,
            weight_decay: 0.01,
            warmup: 200,
            min_lr_ratio: 0.1,
            seed: 126, // the issue number — any fixed seed; never rerun-dependent
        }
    }
}

/// The machine-checked supervision. `pairs` = TRAIN-split positives ONLY
/// (withheld rules' fixture spans AND self-pairs stay out — [`train`]
/// re-checks and refuses). `rule_texts` covers EVERY rule that needs a ψ row
/// — including withheld ones: ψ comes from a rule's own corpus text (public),
/// never from its label association.
#[derive(Debug, Clone, Default)]
pub struct TrainData {
    /// `(span_text, rule_id)` positives.
    pub pairs: Vec<(String, String)>,
    /// `(rule_id, snippet_text)` — buggy + fixed corpus text concatenated.
    pub rule_texts: Vec<(String, String)>,
}

/// The trainer's output: the frozen bytes (canonical, BLAKE3-sealed), the
/// verified runtime view decoded FROM those bytes (never constructed
/// separately), and the run summary.
#[derive(Debug, Clone)]
pub struct Trained {
    pub tables: TrainedTables,
    /// The canonical frozen artifact — write these bytes to disk/`assets/`.
    pub bytes: Vec<u8>,
    /// Mean sigmoid-NCE loss of the LAST step.
    pub final_loss: f32,
    pub n_pairs: usize,
    pub n_rules: usize,
    pub steps: usize,
}

/// Train the two-tower embeddings with sigmoid-NCE + in-batch negatives.
///
/// # Errors
///
/// Empty inputs, duplicate rule ids, a pair referencing a rule without a ψ
/// row, or a pair referencing a WITHHELD rule (the rule-level split is
/// enforced here — defense in depth behind the caller's own filter). Also
/// fails if the produced artifact fails its own decode self-check.
pub fn train(data: &TrainData, cfg: &TrainConfig) -> Result<Trained, String> {
    if data.pairs.is_empty() {
        return Err("TrainData.pairs is empty — nothing to train on".into());
    }
    if data.rule_texts.is_empty() {
        return Err("TrainData.rule_texts is empty — no ψ rows to ship".into());
    }
    let mut rule_row: HashMap<&str, usize> = HashMap::with_capacity(data.rule_texts.len());
    for (i, (id, _)) in data.rule_texts.iter().enumerate() {
        if rule_row.insert(id.as_str(), i).is_some() {
            return Err(format!("duplicate rule id {id:?} in rule_texts"));
        }
    }
    let mut acc: Vec<f32> = Vec::with_capacity(FEATURE_BUCKETS);
    let ys: Vec<(Vec<u32>, Vec<f32>)> = data
        .rule_texts
        .iter()
        .map(|(_, text)| sparse_features(text, &mut acc))
        .collect();
    let mut xs: Vec<(Vec<u32>, Vec<f32>)> = Vec::with_capacity(data.pairs.len());
    let mut pair_rule: Vec<usize> = Vec::with_capacity(data.pairs.len());
    for (span, rule) in &data.pairs {
        if fixture_is_withheld(rule) {
            return Err(format!(
                "pair references withheld rule {rule:?} — filter the train split first"
            ));
        }
        let &row = rule_row
            .get(rule.as_str())
            .ok_or_else(|| format!("pair rule {rule:?} has no ψ row in rule_texts"))?;
        xs.push(sparse_features(span, &mut acc));
        pair_rule.push(row);
    }

    let mut rng = Rng::new(cfg.seed);
    let mut w_span: Vec<f32> = (0..FEATURE_BUCKETS * EMBED_DIM).map(|_| rng.normal() * 0.05).collect();
    let mut w_rule: Vec<f32> = (0..FEATURE_BUCKETS * EMBED_DIM).map(|_| rng.normal() * 0.05).collect();
    let mut ws = Workspace::new(xs.len());
    let mut opt_span = FlatAdamW::new(w_span.len());
    let mut opt_rule = FlatAdamW::new(w_rule.len());
    let mut final_loss = 0.0_f32;
    for step in 0..cfg.steps {
        let lr = AdamW::lr_at(
            cfg.lr,
            step as u64,
            cfg.steps as u64,
            cfg.warmup as u64,
            cfg.min_lr_ratio,
        );
        final_loss = loss_and_grad(&w_span, &w_rule, &xs, &ys, &pair_rule, SIGMOID_TAU, &mut ws);
        opt_span.step(&mut w_span, &ws.g_span, lr, cfg.weight_decay);
        opt_rule.step(&mut w_rule, &ws.g_rule, lr, cfg.weight_decay);
    }

    // ψ rows for EVERY rule through the trained rule tower.
    let mut row_buf = [0.0_f32; EMBED_DIM];
    let mut psi_rows: Vec<(String, f32, Vec<i8>)> = Vec::with_capacity(ys.len());
    for ((id, _), y) in data.rule_texts.iter().zip(ys.iter()) {
        matvec(&w_rule, &y.0, &y.1, &mut row_buf);
        let (q, scale) = quantize(&row_buf);
        psi_rows.push((id.clone(), scale, q));
    }
    // Canonical artifact order: id-sorted. The sort lives HERE so the bytes
    // (and their blake3) are a pure function of the training inputs.
    psi_rows.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let (span_i8, span_scale) = quantize(&w_span);
    let bytes = encode_artifact(
        FEATURE_BUCKETS,
        EMBED_DIM,
        SIGMOID_TAU,
        &span_i8,
        span_scale,
        &psi_rows,
    );
    // Self-check: the shipped bytes MUST decode — a producer whose artifact
    // its own reader refuses is worse than no artifact.
    let tables = decode_verified(&bytes).map_err(|e| format!("self-check failed: {e}"))?;
    Ok(Trained {
        tables,
        bytes,
        final_loss,
        n_pairs: xs.len(),
        n_rules: ys.len(),
        steps: cfg.steps,
    })
}

// ---------------------------------------------------------------------------
// The math kernel — fixed op order, no cross-thread float reduction
// ---------------------------------------------------------------------------

/// Preallocated per-step scratch (the `choice_scorer` rule: capacities stabilize,
/// the loop allocates nothing).
struct Workspace {
    phi: Vec<f32>,  // n × D — span tower outputs
    psi: Vec<f32>,  // n × D — each pair's rule tower output
    gphi: Vec<f32>, // n × D
    gpsi: Vec<f32>, // n × D
    g_span: Vec<f32>,
    g_rule: Vec<f32>,
}

impl Workspace {
    fn new(n_pairs: usize) -> Self {
        Self {
            phi: vec![0.0; n_pairs * EMBED_DIM],
            psi: vec![0.0; n_pairs * EMBED_DIM],
            gphi: vec![0.0; n_pairs * EMBED_DIM],
            gpsi: vec![0.0; n_pairs * EMBED_DIM],
            g_span: vec![0.0; FEATURE_BUCKETS * EMBED_DIM],
            g_rule: vec![0.0; FEATURE_BUCKETS * EMBED_DIM],
        }
    }
}

/// Sigmoid-NCE forward + backward over the full batch. Returns the mean
/// per-pair loss; leaves analytic grads in `ws.g_span` / `ws.g_rule`.
/// `tau` is a parameter so the finite-difference test can run at 1.0.
fn loss_and_grad(
    w_span: &[f32],
    w_rule: &[f32],
    xs: &[(Vec<u32>, Vec<f32>)],
    ys: &[(Vec<u32>, Vec<f32>)],
    pair_rule: &[usize],
    tau: f32,
    ws: &mut Workspace,
) -> f32 {
    let n = xs.len();
    let d = EMBED_DIM;
    ws.g_span.fill(0.0);
    ws.g_rule.fill(0.0);
    ws.gphi.fill(0.0);
    ws.gpsi.fill(0.0);
    for i in 0..n {
        matvec(w_span, &xs[i].0, &xs[i].1, &mut ws.phi[i * d..(i + 1) * d]);
        let y = &ys[pair_rule[i]];
        matvec(w_rule, &y.0, &y.1, &mut ws.psi[i * d..(i + 1) * d]);
    }
    let mut loss = 0.0_f32;
    for i in 0..n {
        let phi_i = &ws.phi[i * d..(i + 1) * d];
        let psi_i = &ws.psi[i * d..(i + 1) * d];
        // positive: -log σ(z⁺), dL/dz = σ(z⁺) - 1
        let zp = dot(phi_i, psi_i) / tau;
        let sp = sigmoid(zp);
        loss += -(sp.max(1e-7)).ln();
        let gp = (sp - 1.0) / tau;
        for b in 0..d {
            ws.gphi[i * d + b] += gp * psi_i[b];
            ws.gpsi[i * d + b] += gp * phi_i[b];
        }
        // in-batch negatives: -log(1 - σ(z⁻)), dL/dz = σ(z⁻); pairs sharing
        // the rule are positives, skipped.
        for j in 0..n {
            if pair_rule[j] == pair_rule[i] {
                continue;
            }
            let psi_j = &ws.psi[j * d..(j + 1) * d];
            let zn = dot(phi_i, psi_j) / tau;
            let sn = sigmoid(zn);
            loss += -((1.0 - sn).max(1e-7)).ln();
            let gn = sn / tau;
            for b in 0..d {
                ws.gphi[i * d + b] += gn * psi_j[b];
                ws.gpsi[j * d + b] += gn * phi_i[b];
            }
        }
    }
    for i in 0..n {
        scatter_grad(&mut ws.g_span, &ws.gphi[i * d..(i + 1) * d], &xs[i].0, &xs[i].1);
        let y = &ys[pair_rule[i]];
        scatter_grad(&mut ws.g_rule, &ws.gpsi[i * d..(i + 1) * d], &y.0, &y.1);
    }
    loss / n as f32
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Sparse (bucket-indexed) row × matrix: `out = Σ_b x_b · W[b]`.
fn matvec(w: &[f32], idx: &[u32], val: &[f32], out: &mut [f32]) {
    out.fill(0.0);
    let d = out.len();
    for (&b, &xb) in idx.iter().zip(val.iter()) {
        let row = &w[b as usize * d..(b as usize + 1) * d];
        for (i, &wr) in row.iter().enumerate() {
            out[i] += xb * wr;
        }
    }
}

/// The transpose: `g_W[b] += gout · x_b`.
fn scatter_grad(g: &mut [f32], gout: &[f32], idx: &[u32], val: &[f32]) {
    let d = gout.len();
    for (&b, &xb) in idx.iter().zip(val.iter()) {
        let row = &mut g[b as usize * d..(b as usize + 1) * d];
        for (i, &gg) in gout.iter().enumerate() {
            row[i] += gg * xb;
        }
    }
}

/// Shared tokenization → sparse signed L2-normalized unigram+bigram bag at
/// [`FEATURE_BUCKETS`] (the `acc` scratch is caller-owned; training only).
fn sparse_features(text: &str, acc: &mut Vec<f32>) -> (Vec<u32>, Vec<f32>) {
    acc.clear();
    acc.resize(FEATURE_BUCKETS, 0.0);
    fill_hashbag(text, FEATURE_BUCKETS, acc);
    let mut idx = Vec::new();
    let mut val = Vec::new();
    for (b, &v) in acc.iter().enumerate() {
        if v != 0.0 {
            idx.push(b as u32);
            val.push(v);
        }
    }
    (idx, val)
}

/// Per-matrix absmax → ×127 i8 (the `span_embed` quantization law). A zero
/// matrix keeps scale 1.0 so dequant stays defined.
fn quantize(m: &[f32]) -> (Vec<i8>, f32) {
    let absmax = m.iter().fold(0.0_f32, |a, &v| a.max(v.abs()));
    let scale = if absmax > 0.0 { absmax / 127.0 } else { 1.0 };
    let q = m.iter().map(|&v| round_clamp_i8(v / scale)).collect();
    (q, scale)
}

fn round_clamp_i8(v: f32) -> i8 {
    let r = v.round();
    if r > 127.0 {
        127
    } else if r < -127.0 {
        -127
    } else {
        r as i8
    }
}

/// Flat-tensor `AdamW` (decoupled weight decay) — the `choice_scorer::AdamW` is
/// typed to its own param shape, so this kernel carries the flat twin. The
/// warmup+cosine SCHEDULE is shared (`AdamW::lr_at`). Generalize the pair
/// into one constructor when a third consumer exists (rule of three).
struct FlatAdamW {
    m: Vec<f32>,
    v: Vec<f32>,
    t: u64,
}

impl FlatAdamW {
    fn new(len: usize) -> Self {
        Self {
            m: vec![0.0; len],
            v: vec![0.0; len],
            t: 0,
        }
    }

    fn step(&mut self, w: &mut [f32], g: &[f32], lr: f32, wd: f32) {
        self.t += 1;
        let (b1, b2, eps) = (0.9_f32, 0.999_f32, 1e-8_f32);
        let bc1 = 1.0 - b1.powi(self.t as i32);
        let bc2 = 1.0 - b2.powi(self.t as i32);
        for i in 0..w.len() {
            let gi = g[i] + wd * w[i];
            self.m[i] = b1 * self.m[i] + (1.0 - b1) * gi;
            self.v[i] = b2 * self.v[i] + (1.0 - b2) * gi * gi;
            let mh = self.m[i] / bc1;
            let vh = self.v[i] / bc2;
            w[i] -= lr * mh / (vh.sqrt() + eps);
        }
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_pair(span: &str, rule: &str) -> (String, String) {
        (span.to_string(), rule.to_string())
    }

    /// The first non-withheld suffix for a rule-name prefix — the tiny
    /// fixtures must dodge the (deterministic) withheld set, since `train`
    /// refuses any pair referencing a withheld rule.
    fn name_not_withheld(prefix: &str) -> String {
        (0..100)
            .map(|i| format!("{prefix}_{i}x"))
            .find(|name| !fixture_is_withheld(name))
            .expect("non-withheld name exists")
    }

    fn tiny_train_data() -> TrainData {
        let a = name_not_withheld("rule_a");
        let b = name_not_withheld("rule_b");
        let c = name_not_withheld("rule_c");
        TrainData {
            pairs: vec![
                tiny_pair("let mut v = Vec::new(); v.push(1);", &a),
                tiny_pair("for x in iter { sum += x; }", &b),
                tiny_pair("match opt { Some(x) => x, None => 0 }", &c),
            ],
            rule_texts: vec![
                (
                    a.clone(),
                    "let mut v = Vec::new(); v.push(1); fixed: let v = Vec::from([1]);".into(),
                ),
                (
                    b.clone(),
                    "for x in iter { sum += x; } fixed: sum = x + sum;".into(),
                ),
                (
                    c.clone(),
                    "match opt { Some(x) => x, None => 0 } use unwrap_or".into(),
                ),
            ],
        }
    }

    /// The Issue-126 refactor must be byte-identical at the original 32
    /// buckets: `fill_hashbag` + quantize == `span_embed`.
    #[test]
    fn hashbag_matches_span_embed_at_32() {
        let texts = [
            "let mut v = Vec::new(); v.push(1);",
            "pub fn swallow_following_space(source: &str, mut e: SourceEdit) -> SourceEdit {",
            "for x in iter { sum += x; }",
            "",
        ];
        for text in texts {
            let mut acc = [0.0_f32; 32];
            fill_hashbag(text, 32, &mut acc);
            let quant: Vec<i8> = acc
                .iter()
                .map(|&v| {
                    let r = v * crate::span_embed::SPAN_EMBED_SCALE;
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let q = r.round().clamp(-127.0, 127.0) as i8;
                    q
                })
                .collect();
            assert_eq!(quant, crate::span_embed::span_embed(text).to_vec());
        }
    }

    #[test]
    fn split_is_deterministic_and_quarter_rate() {
        for id in ["collapsible_if", "manual_clamp", "needless_return", "rule_a"] {
            assert_eq!(fixture_is_withheld(id), fixture_is_withheld(id), "{id}");
        }
        let ids: Vec<String> = (0..200).map(|i| format!("fixture_split_probe_{i}")).collect();
        let withheld = ids.iter().filter(|id| fixture_is_withheld(id)).count();
        // ~Binomial(200, 0.25), σ ≈ 6.1 — ±3σ.
        assert!(
            (40..=60).contains(&withheld),
            "25% split drifted: {withheld}/200"
        );
    }

    #[test]
    fn artifact_roundtrip_and_tamper_refusal() {
        let data = tiny_train_data();
        let tr = train(&data, &TrainConfig { steps: 20, ..TrainConfig::default() })
            .expect("tiny train");
        let a = decode_verified(&tr.bytes).expect("decode");
        let b = decode_verified(&tr.bytes).expect("decode");
        assert_eq!(a.rule_count(), b.rule_count());
        for (ra, rb) in a.rules().iter().zip(b.rules().iter()) {
            assert_eq!(ra.id, rb.id);
            assert_eq!(ra.psi(), rb.psi());
        }
        // tamper: flip one payload byte → hash mismatch
        let mut bad = tr.bytes.clone();
        let mid = bad.len() / 2;
        bad[mid] ^= 0x01;
        assert!(decode_verified(&bad).is_err(), "tampered payload must refuse");
        // truncation
        assert!(decode_verified(&tr.bytes[..tr.bytes.len() - 1]).is_err());
        // bad magic
        let mut bad2 = tr.bytes.clone();
        bad2[0] = b'X';
        assert!(decode_verified(&bad2).is_err());
        // uncovered rule → None (graceful), never a panic
        let phi = a.project_span("let mut v = Vec::new();");
        assert!(a.score_rule(&phi, "rule_not_in_tables").is_none());
    }

    #[test]
    fn tiny_training_separates_positives() {
        let data = tiny_train_data();
        let tr = train(&data, &TrainConfig { steps: 400, ..TrainConfig::default() })
            .expect("tiny train");
        assert!(tr.final_loss.is_finite());
        assert_eq!(tr.tables.rule_count(), 3);
        assert_eq!(tr.tables.dim, EMBED_DIM);
        let phi = tr.tables.project_span("let mut v = Vec::new(); v.push(1);");
        let hit = tr.tables.score_rule(&phi, &name_not_withheld("rule_a")).expect("row");
        let miss = tr.tables.score_rule(&phi, &name_not_withheld("rule_b")).expect("row");
        // MAGNITUDE, not just ordering: the pre-fix dequant bug passed this
        // test with every score at 0.5 ± 1e-4 (a ±1e-4 margin is noise).
        // A trained positive pair must sit clearly above the σ(0) = 0.5 null.
        assert!(hit > 0.75, "positive σ {hit} must clear the null by a real margin");
        assert!(hit > miss, "trained positive {hit} must outrank negative {miss}");
    }

    #[test]
    #[allow(clippy::float_cmp)] // σ(0) = 0.5 and never-touched grads are construction identities
    fn zero_span_scores_uniform_half() {
        let data = tiny_train_data();
        let tr = train(&data, &TrainConfig { steps: 20, ..TrainConfig::default() })
            .expect("tiny train");
        let phi = tr.tables.project_span("   \n\t ");
        assert!(phi.iter().all(|&v| v == 0.0));
        let a = tr
            .tables
            .score_rule(&phi, &name_not_withheld("rule_a"))
            .expect("row");
        let b = tr
            .tables
            .score_rule(&phi, &name_not_withheld("rule_b"))
            .expect("row");
        // σ(0) = 0.5 EXACTLY by the sigmoid's z == 0.0 branch — a
        // construction identity, not a float-comparison hazard.
        assert_eq!(a, b);
        assert!((a - 0.5).abs() < 1e-6, "σ(0) = 0.5, got {a}");
    }

    #[test]
    fn train_refuses_withheld_rule_pairs() {
        // Find a withheld probe id by scanning the deterministic split.
        let mut withheld = None;
        for i in 0..1000 {
            let id = format!("probe_{i}");
            if fixture_is_withheld(&id) {
                withheld = Some(id);
                break;
            }
        }
        let Some(w) = withheld else { unreachable!("25% of 1000") };
        let data = TrainData {
            pairs: vec![tiny_pair("let x = 1;", &w)],
            rule_texts: vec![(w.clone(), "let x = 1;".into())],
        };
        let err = train(&data, &TrainConfig::default()).unwrap_err();
        assert!(err.contains("withheld"), "got: {err}");
    }

    /// Every analytic gradient coordinate sampled along the data's ACTIVE
    /// buckets (plus an inactive bucket pinned to exactly zero) must match
    /// the central finite difference. `tau = 1.0` so the positive/negative
    /// paths are both wide-open.
    #[test]
    #[allow(clippy::float_cmp)] // untouched coordinates receive exactly 0.0 gradient
    fn gradient_matches_finite_difference() {
        let data = tiny_train_data();
        let cfg = TrainConfig::default();
        let mut acc: Vec<f32> = Vec::with_capacity(FEATURE_BUCKETS);
        let ys: Vec<(Vec<u32>, Vec<f32>)> = data
            .rule_texts
            .iter()
            .map(|(_, t)| sparse_features(t, &mut acc))
            .collect();
        let xs: Vec<(Vec<u32>, Vec<f32>)> = data
            .pairs
            .iter()
            .map(|(s, _)| sparse_features(s, &mut acc))
            .collect();
        // pair_rule follows tiny_train_data's rule_texts ORDER (a, b, c).
        let pair_rule: Vec<usize> = (0..data.pairs.len()).collect();

        let mut rng = Rng::new(cfg.seed);
        let mut w_span: Vec<f32> = (0..FEATURE_BUCKETS * EMBED_DIM).map(|_| rng.normal() * 0.05).collect();
        let mut w_rule: Vec<f32> = (0..FEATURE_BUCKETS * EMBED_DIM).map(|_| rng.normal() * 0.05).collect();
        let mut ws = Workspace::new(xs.len());
        let _base = loss_and_grad(&w_span, &w_rule, &xs, &ys, &pair_rule, 1.0, &mut ws);
        let g_span_ref = ws.g_span.clone();
        let g_rule_ref = ws.g_rule.clone();

        // Buckets the data actually touches — the coords where gradients are
        // nonzero by construction (a random coord's analytic grad is usually
        // 0, which pins nothing).
        let mut buckets: Vec<usize> = xs
            .iter()
            .chain(ys.iter())
            .flat_map(|(i, _)| i.iter().map(|&b| b as usize))
            .collect();
        buckets.sort_unstable();
        buckets.dedup();
        assert!(!buckets.is_empty());
        let eps = 1e-2_f32;
        for k in 0..12_usize {
            let b = buckets[(k * 7) % buckets.len()];
            let dd = (k * 3) % EMBED_DIM;
            let coord = b * EMBED_DIM + dd;
            // span tower
            let orig = w_span[coord];
            w_span[coord] = orig + eps;
            let lp = loss_and_grad(&w_span, &w_rule, &xs, &ys, &pair_rule, 1.0, &mut ws);
            w_span[coord] = orig - eps;
            let lm = loss_and_grad(&w_span, &w_rule, &xs, &ys, &pair_rule, 1.0, &mut ws);
            w_span[coord] = orig;
            let num = (lp - lm) / (2.0 * eps);
            let ana = g_span_ref[coord];
            let err = (ana - num).abs() / 1.0_f32.max(ana.abs()).max(num.abs());
            assert!(err < 5e-2, "span coord {coord}: analytic {ana} vs numeric {num} (err {err})");
            // rule tower
            let orig = w_rule[coord];
            w_rule[coord] = orig + eps;
            let lp = loss_and_grad(&w_span, &w_rule, &xs, &ys, &pair_rule, 1.0, &mut ws);
            w_rule[coord] = orig - eps;
            let lm = loss_and_grad(&w_span, &w_rule, &xs, &ys, &pair_rule, 1.0, &mut ws);
            w_rule[coord] = orig;
            let num = (lp - lm) / (2.0 * eps);
            let ana = g_rule_ref[coord];
            let err = (ana - num).abs() / 1.0_f32.max(ana.abs()).max(num.abs());
            assert!(err < 5e-2, "rule coord {coord}: analytic {ana} vs numeric {num} (err {err})");
        }
        // An inactive bucket's gradient is exactly zero in both towers.
        let empty = (0..FEATURE_BUCKETS)
            .find(|b| !buckets.contains(b))
            .expect("inactive bucket exists");
        let coord = empty * EMBED_DIM;
        // An untouched coordinate receives NO gradient term — exactly 0.0 by
        // construction (the scatter only writes data-touched buckets).
        assert_eq!(g_span_ref[coord], 0.0);
        assert_eq!(g_rule_ref[coord], 0.0);
    }
}
