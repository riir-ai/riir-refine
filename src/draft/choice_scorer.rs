//! Issue 125 — the CUA-S1 tinyx option-scorer clone (Research 573 recipe).
//!
//! A ~719k-param byte-level specialist that scores N runtime-supplied text
//! options against a text context in ONE forward pass — the Jev/CUA-S1
//! contract transplanted onto healer rule selection: context = code span
//! bytes (≤224), options = candidate-rule bytes (≤96 each, rule id +
//! buggy-code pattern) + `skip`, output = one logit per option.
//!
//! Architecture (Research 573 §2, `AttentionHead` adapted from jevlike):
//!
//! ```text
//! byte embed (257, UTF-8+1) + learned abs positions
//!   → 2-layer pre-LN encoder (width 128, 4 heads) over the context
//!   → 1-layer pre-LN encoder over each option, final LN, mean-pool → p_n
//! q_n = W_q·p_n;  K = W_k·ctx;  V = W_v·ctx
//! attended_n = softmax_j(q_n·K_j/√r)·V      # per-option query-conditioned pooling
//! logit_n    = q_n·attended_n/√r            # the self-referential dot
//! ```
//!
//! Supervised CE only (the recipe's finding: no RL needed), `AdamW`,
//! warmup+cosine. Hand-rolled forward/backward — no new deps, no autograd
//! crate; every block is pinned by finite-difference gradient checks in
//! the tests below. Determinism: fixed op order, seeded `SplitMix64` init,
//! no cross-thread float reduction (the training loop reduces worker
//! grads in worker-index order).
//!
//! Grads reuse the [`ChoiceScorer`] shape itself ([`ScorerGrads`] is a
//! newtype) — params, grads, Adam moments and the finite-difference
//! checks all walk the SAME `paths()` enumeration, so there is no second
//! tensor layout to drift out of pairing.
//!
//! PoC-only (`choice_scorer_poc`): the shipped suggest lane stays
//! modelless (`RerankMode::Structural`) until the GOAT gates pass.
//! Softmax-over-options is the documented categorical-choice exception to
//! the house sigmoid rule (mutually exclusive rule choice per span).
//!
//! `PoC` simplification (measured cost, accepted): per-episode scratch is
//! allocated fresh rather than pooled — ~60 small Vecs per episode against
//! a ~26k-episode training run is allocator noise, and the serving path's
//! cost is reported honestly by the G2 gate either way.

// Scoped lint posture for a hand-rolled linear-algebra module (the
// margin_weight/domain_router precedent, module-scope because the whole
// file is one math kernel):
// - cast precision/truncation: every cast here is a dimension/count/seed
//   ≤ 2²⁴ — exactly representable in f32; token counts and vocab indices
//   never approach f32's mantissa limit.
// - single-char + similar names: i/j/k/t/s are matrix indices (row, col,
//   token, sequence) — the notation the math in the doc comments uses.
// - needless_range_loop: the indexed loops ARE the kernel (slice-math with
//   two index spaces each).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::needless_range_loop
)]

use std::f32::consts::PI;

pub const BYTE_VOCAB: usize = 257; // 256 byte values (index b+1) + pad 0

/// CUA-S1-shaped defaults (width 128, 4 heads, FFN ×4, ctx 224, opt 96,
/// 2+1 layers ≈ 719k params). Dims are runtime fields so the
/// gradient-check tests can instantiate a tiny twin at one compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScorerConfig {
    pub width: usize,
    pub heads: usize,
    pub d_ff: usize,
    pub ctx_len: usize,
    pub opt_len: usize,
    pub ctx_layers: usize,
    pub opt_layers: usize,
}

impl Default for ScorerConfig {
    fn default() -> Self {
        Self {
            width: 128,
            heads: 4,
            d_ff: 512,
            ctx_len: 224,
            opt_len: 96,
            ctx_layers: 2,
            opt_layers: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64 + Box-Muller)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Rng(pub u64);

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    #[inline]
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform f32 in [0, 1) — 24 mantissa bits, exact in f32.
    #[must_use]
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1u64 << 24) as f32)
    }

    /// Standard normal via Box-Muller (one draw per call — the stream
    /// order never depends on consumer branching).
    #[must_use]
    pub fn normal(&mut self) -> f32 {
        let mut u1 = self.next_f32();
        if u1 <= f32::EPSILON {
            u1 = f32::EPSILON;
        }
        let u2 = self.next_f32();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }

    #[must_use]
    pub fn uniform_range(&mut self, lo: usize, hi_exclusive: usize) -> usize {
        debug_assert!(hi_exclusive > lo);
        lo + (self.next_u64() % ((hi_exclusive - lo) as u64)) as usize
    }

    /// Deterministic Fisher-Yates byte shuffle (the control arm's context
    /// corruption — same byte multiset, destroyed order).
    #[must_use]
    pub fn shuffled_bytes(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut v = bytes.to_vec();
        for i in (1..v.len()).rev() {
            let j = (self.next_u64() % ((i + 1) as u64)) as usize;
            v.swap(i, j);
        }
        v
    }
}

// ---------------------------------------------------------------------------
// Param tensors
// ---------------------------------------------------------------------------

/// One pre-LN transformer encoder layer (biases on every linear, LN γ/β).
#[derive(Debug, Clone)]
pub struct Layer {
    pub ln1_g: Vec<f32>,
    pub ln1_b: Vec<f32>,
    pub wq: Vec<f32>,
    pub bq: Vec<f32>,
    pub wk: Vec<f32>,
    pub bk: Vec<f32>,
    pub wv: Vec<f32>,
    pub bv: Vec<f32>,
    pub wo: Vec<f32>,
    pub bo: Vec<f32>,
    pub ln2_g: Vec<f32>,
    pub ln2_b: Vec<f32>,
    pub w1: Vec<f32>,
    pub b1: Vec<f32>,
    pub w2: Vec<f32>,
    pub b2: Vec<f32>,
}

impl Layer {
    fn new(cfg: &ScorerConfig, rng: &mut Rng) -> Self {
        let w = cfg.width;
        let f = cfg.d_ff;
        let std = 0.02_f32;
        let resid_std = std / (2.0_f32).sqrt(); // GPT-2 residual-out scaling
        let normal = |n: usize, s: f32, rng: &mut Rng| -> Vec<f32> {
            (0..n).map(|_| rng.normal() * s).collect()
        };
        Self {
            ln1_g: vec![1.0; w],
            ln1_b: vec![0.0; w],
            wq: normal(w * w, std, rng),
            bq: vec![0.0; w],
            wk: normal(w * w, std, rng),
            bk: vec![0.0; w],
            wv: normal(w * w, std, rng),
            bv: vec![0.0; w],
            wo: normal(w * w, resid_std, rng),
            bo: vec![0.0; w],
            ln2_g: vec![1.0; w],
            ln2_b: vec![0.0; w],
            w1: normal(w * f, std, rng),
            b1: vec![0.0; f],
            w2: normal(f * w, resid_std, rng),
            b2: vec![0.0; w],
        }
    }
}

/// Addressable path to one tensor — the single enumeration params, grads
/// and Adam moments all walk (same layout by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TensorPath {
    ByteEmb,
    PosCtx,
    PosOpt,
    CtxLayer(usize, LayerField),
    OptLayer(usize, LayerField),
    CtxLnG,
    CtxLnB,
    OptLnG,
    OptLnB,
    Wq,
    Bq,
    Wk,
    Bk,
    Wv,
    Bv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerField {
    Ln1G,
    Ln1B,
    Wq,
    Bq,
    Wk,
    Bk,
    Wv,
    Bv,
    Wo,
    Bo,
    Ln2G,
    Ln2B,
    W1,
    B1,
    W2,
    B2,
}

impl LayerField {
    pub const ALL: [Self; 16] = [
        Self::Ln1G,
        Self::Ln1B,
        Self::Wq,
        Self::Bq,
        Self::Wk,
        Self::Bk,
        Self::Wv,
        Self::Bv,
        Self::Wo,
        Self::Bo,
        Self::Ln2G,
        Self::Ln2B,
        Self::W1,
        Self::B1,
        Self::W2,
        Self::B2,
    ];
}

fn layer_tensor_mut(layer: &mut Layer, f: LayerField) -> &mut Vec<f32> {
    match f {
        LayerField::Ln1G => &mut layer.ln1_g,
        LayerField::Ln1B => &mut layer.ln1_b,
        LayerField::Wq => &mut layer.wq,
        LayerField::Bq => &mut layer.bq,
        LayerField::Wk => &mut layer.wk,
        LayerField::Bk => &mut layer.bk,
        LayerField::Wv => &mut layer.wv,
        LayerField::Bv => &mut layer.bv,
        LayerField::Wo => &mut layer.wo,
        LayerField::Bo => &mut layer.bo,
        LayerField::Ln2G => &mut layer.ln2_g,
        LayerField::Ln2B => &mut layer.ln2_b,
        LayerField::W1 => &mut layer.w1,
        LayerField::B1 => &mut layer.b1,
        LayerField::W2 => &mut layer.w2,
        LayerField::B2 => &mut layer.b2,
    }
}

fn layer_tensor_ref(l: &Layer, f: LayerField) -> &Vec<f32> {
    match f {
        LayerField::Ln1G => &l.ln1_g,
        LayerField::Ln1B => &l.ln1_b,
        LayerField::Wq => &l.wq,
        LayerField::Bq => &l.bq,
        LayerField::Wk => &l.wk,
        LayerField::Bk => &l.bk,
        LayerField::Wv => &l.wv,
        LayerField::Bv => &l.bv,
        LayerField::Wo => &l.wo,
        LayerField::Bo => &l.bo,
        LayerField::Ln2G => &l.ln2_g,
        LayerField::Ln2B => &l.ln2_b,
        LayerField::W1 => &l.w1,
        LayerField::B1 => &l.b1,
        LayerField::W2 => &l.w2,
        LayerField::B2 => &l.b2,
    }
}

/// The scorer: byte/pos embeddings, ctx + opt encoder stacks, final LNs,
/// and the options-as-queries head. ALSO the grads/moment layout (values
/// interpreted by the consumer — the shape is the point).
#[derive(Debug, Clone)]
pub struct ChoiceScorer {
    pub cfg: ScorerConfig,
    pub byte_emb: Vec<f32>, // 257 × W
    pub pos_ctx: Vec<f32>,  // ctx_len × W
    pub pos_opt: Vec<f32>,  // opt_len × W
    pub ctx_layers: Vec<Layer>,
    pub opt_layers: Vec<Layer>,
    pub ctx_ln_g: Vec<f32>,
    pub ctx_ln_b: Vec<f32>,
    pub opt_ln_g: Vec<f32>,
    pub opt_ln_b: Vec<f32>,
    pub wq: Vec<f32>,
    pub bq: Vec<f32>, // head: W×W, W
    pub wk: Vec<f32>,
    pub bk: Vec<f32>,
    pub wv: Vec<f32>,
    pub bv: Vec<f32>,
}

impl ChoiceScorer {
    #[must_use]
    pub fn new(cfg: ScorerConfig, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let w = cfg.width;
        let std = 0.02_f32;
        let normal =
            |n: usize, rng: &mut Rng| -> Vec<f32> { (0..n).map(|_| rng.normal() * std).collect() };
        Self {
            cfg,
            byte_emb: normal(BYTE_VOCAB * w, &mut rng),
            pos_ctx: normal(cfg.ctx_len * w, &mut rng),
            pos_opt: normal(cfg.opt_len * w, &mut rng),
            ctx_layers: (0..cfg.ctx_layers)
                .map(|_| Layer::new(&cfg, &mut rng))
                .collect(),
            opt_layers: (0..cfg.opt_layers)
                .map(|_| Layer::new(&cfg, &mut rng))
                .collect(),
            ctx_ln_g: vec![1.0; w],
            ctx_ln_b: vec![0.0; w],
            opt_ln_g: vec![1.0; w],
            opt_ln_b: vec![0.0; w],
            wq: normal(w * w, &mut rng),
            bq: vec![0.0; w],
            wk: normal(w * w, &mut rng),
            bk: vec![0.0; w],
            wv: normal(w * w, &mut rng),
            bv: vec![0.0; w],
        }
    }

    #[must_use]
    pub fn param_count(&self) -> usize {
        let w = self.cfg.width;
        let f = self.cfg.d_ff;
        let layer_n = 2 * w + 4 * (w * w + w) + 2 * w + (w * f + f) + (f * w + w);
        BYTE_VOCAB * w
            + self.cfg.ctx_len * w
            + self.cfg.opt_len * w
            + (self.cfg.ctx_layers + self.cfg.opt_layers) * layer_n
            + 4 * w
            + 3 * (w * w + w)
    }

    pub fn tensor_mut(&mut self, p: TensorPath) -> &mut Vec<f32> {
        match p {
            TensorPath::ByteEmb => &mut self.byte_emb,
            TensorPath::PosCtx => &mut self.pos_ctx,
            TensorPath::PosOpt => &mut self.pos_opt,
            TensorPath::CtxLayer(i, f) => layer_tensor_mut(&mut self.ctx_layers[i], f),
            TensorPath::OptLayer(i, f) => layer_tensor_mut(&mut self.opt_layers[i], f),
            TensorPath::CtxLnG => &mut self.ctx_ln_g,
            TensorPath::CtxLnB => &mut self.ctx_ln_b,
            TensorPath::OptLnG => &mut self.opt_ln_g,
            TensorPath::OptLnB => &mut self.opt_ln_b,
            TensorPath::Wq => &mut self.wq,
            TensorPath::Bq => &mut self.bq,
            TensorPath::Wk => &mut self.wk,
            TensorPath::Bk => &mut self.bk,
            TensorPath::Wv => &mut self.wv,
            TensorPath::Bv => &mut self.bv,
        }
    }

    fn tensor_ref(&self, p: TensorPath) -> &Vec<f32> {
        match p {
            TensorPath::ByteEmb => &self.byte_emb,
            TensorPath::PosCtx => &self.pos_ctx,
            TensorPath::PosOpt => &self.pos_opt,
            TensorPath::CtxLayer(i, f) => layer_tensor_ref(&self.ctx_layers[i], f),
            TensorPath::OptLayer(i, f) => layer_tensor_ref(&self.opt_layers[i], f),
            TensorPath::CtxLnG => &self.ctx_ln_g,
            TensorPath::CtxLnB => &self.ctx_ln_b,
            TensorPath::OptLnG => &self.opt_ln_g,
            TensorPath::OptLnB => &self.opt_ln_b,
            TensorPath::Wq => &self.wq,
            TensorPath::Bq => &self.bq,
            TensorPath::Wk => &self.wk,
            TensorPath::Bk => &self.bk,
            TensorPath::Wv => &self.wv,
            TensorPath::Bv => &self.bv,
        }
    }

    /// Every tensor path in FIXED serialization order.
    #[must_use]
    pub fn paths(&self) -> Vec<TensorPath> {
        let mut v = Vec::with_capacity(16 + 16 * (self.cfg.ctx_layers + self.cfg.opt_layers));
        v.push(TensorPath::ByteEmb);
        v.push(TensorPath::PosCtx);
        v.push(TensorPath::PosOpt);
        for i in 0..self.ctx_layers.len() {
            for f in LayerField::ALL {
                v.push(TensorPath::CtxLayer(i, f));
            }
        }
        for i in 0..self.opt_layers.len() {
            for f in LayerField::ALL {
                v.push(TensorPath::OptLayer(i, f));
            }
        }
        v.push(TensorPath::CtxLnG);
        v.push(TensorPath::CtxLnB);
        v.push(TensorPath::OptLnG);
        v.push(TensorPath::OptLnB);
        v.push(TensorPath::Wq);
        v.push(TensorPath::Bq);
        v.push(TensorPath::Wk);
        v.push(TensorPath::Bk);
        v.push(TensorPath::Wv);
        v.push(TensorPath::Bv);
        v.shrink_to_fit();
        v
    }

    /// Flat serialization (checkpoint save/load), `paths()` order.
    #[must_use]
    pub fn to_flat(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.param_count());
        for p in self.paths() {
            out.extend_from_slice(self.tensor_ref(p));
        }
        out
    }

    /// Restore params from a flat vector (BLAKE3-verified upstream); the
    /// seed only drives shapes via cfg, values come from `flat`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when `flat`'s length differs from the config's
    /// `param_count` (a truncated or foreign checkpoint).
    pub fn from_flat(cfg: ScorerConfig, flat: &[f32]) -> Result<Self, String> {
        let mut m = Self::new(cfg, 0);
        if flat.len() != m.param_count() {
            return Err(format!(
                "checkpoint length {} != param_count {}",
                flat.len(),
                m.param_count()
            ));
        }
        let mut off = 0usize;
        let paths = m.paths();
        for p in paths {
            let t = m.tensor_mut(p);
            let n = t.len();
            t.copy_from_slice(&flat[off..off + n]);
            off += n;
        }
        debug_assert_eq!(off, flat.len());
        Ok(m)
    }
}

/// Grads as a newtype over the same layout — zero-initialized via
/// [`ChoiceScorer::zeroed`].
#[derive(Debug, Clone)]
pub struct ScorerGrads(pub ChoiceScorer);

impl ScorerGrads {
    #[must_use]
    pub fn zero_like(m: &ChoiceScorer) -> Self {
        Self(ChoiceScorer::zeroed(&m.cfg))
    }

    /// Deterministic in-place accumulate (worker reduce; caller-fixed
    /// path order — bit-stable for a fixed worker count).
    pub fn add_from(&mut self, other: &Self) {
        for p in self.0.paths() {
            let b = other.0.tensor_ref(p);
            let dst = self.0.tensor_mut(p);
            for (d, s) in dst.iter_mut().zip(b.iter()) {
                *d += *s;
            }
        }
    }

    /// Deterministic scale (batch-mean normalization before the step).
    pub fn scale(&mut self, k: f32) {
        for p in self.0.paths() {
            let t = self.0.tensor_mut(p);
            for v in t.iter_mut() {
                *v *= k;
            }
        }
    }
}

impl ChoiceScorer {
    /// All-zero tensors of the configured shape (grads / moments seed).
    #[must_use]
    pub fn zeroed(cfg: &ScorerConfig) -> Self {
        let mut m = Self::new(*cfg, 0);
        for p in m.paths() {
            let t = m.tensor_mut(p);
            for v in t.iter_mut() {
                *v = 0.0;
            }
        }
        m
    }
}

// ---------------------------------------------------------------------------
// Math kernels (row-major, exact op order everywhere)
// ---------------------------------------------------------------------------

const LN_EPS: f32 = 1e-5;

/// `LayerNorm` forward over `t` rows of width `w`. Caches x̂ and 1/σ.
#[allow(clippy::too_many_arguments)] // kernel signature: buffers + dims, one role each
fn ln_fwd(
    x: &[f32],
    t: usize,
    w: usize,
    g: &[f32],
    b: &[f32],
    out: &mut Vec<f32>,
    xhat: &mut Vec<f32>,
    inv_std: &mut Vec<f32>,
) {
    out.clear();
    out.resize(t * w, 0.0);
    xhat.clear();
    xhat.resize(t * w, 0.0);
    inv_std.clear();
    inv_std.resize(t, 0.0);
    let wf = w as f32;
    for i in 0..t {
        let row = &x[i * w..(i + 1) * w];
        let mean = row.iter().sum::<f32>() / wf;
        let var = row
            .iter()
            .map(|v| {
                let d = v - mean;
                d * d
            })
            .sum::<f32>()
            / wf;
        let inv = 1.0 / (var + LN_EPS).sqrt();
        inv_std[i] = inv;
        for j in 0..w {
            let xh = (row[j] - mean) * inv;
            xhat[i * w + j] = xh;
            out[i * w + j] = g[j] * xh + b[j];
        }
    }
}

/// `LayerNorm` backward: dx = (1/σ)·(dx̂ − mean(dx̂) − x̂·mean(dx̂⊙x̂)),
/// dx̂ = dy⊙γ; dγ/dβ accumulated over rows.
#[allow(clippy::too_many_arguments)]
fn ln_bwd(
    dy: &[f32],
    xhat: &[f32],
    inv_std: &[f32],
    t: usize,
    w: usize,
    g: &[f32],
    dg: &mut [f32],
    db: &mut [f32],
    dx: &mut Vec<f32>,
) {
    dx.clear();
    dx.resize(t * w, 0.0);
    let wf = w as f32;
    for i in 0..t {
        let dy_row = &dy[i * w..(i + 1) * w];
        let xh_row = &xhat[i * w..(i + 1) * w];
        let inv = inv_std[i];
        let mut mean_dxhat = 0.0_f32;
        let mut mean_dxhat_xhat = 0.0_f32;
        let mut dxh = vec![0.0_f32; w];
        for j in 0..w {
            dxh[j] = dy_row[j] * g[j];
            mean_dxhat += dxh[j];
            mean_dxhat_xhat += dxh[j] * xh_row[j];
            dg[j] += dy_row[j] * xh_row[j];
            db[j] += dy_row[j];
        }
        mean_dxhat /= wf;
        mean_dxhat_xhat /= wf;
        let dx_row = &mut dx[i * w..(i + 1) * w];
        for j in 0..w {
            dx_row[j] = inv * (dxh[j] - mean_dxhat - xh_row[j] * mean_dxhat_xhat);
        }
    }
}

/// out[t,o] = `Σ_i` in[t,i]·w[i,o] + b[o] — w row-major [`i_dim`, `o_dim`].
fn linear_fwd(
    input: &[f32],
    t: usize,
    i_dim: usize,
    w: &[f32],
    b: &[f32],
    o_dim: usize,
    out: &mut Vec<f32>,
) {
    out.clear();
    out.resize(t * o_dim, 0.0);
    for tt in 0..t {
        let in_row = &input[tt * i_dim..(tt + 1) * i_dim];
        let out_row = &mut out[tt * o_dim..(tt + 1) * o_dim];
        out_row.copy_from_slice(b);
        for (ii, &iv) in in_row.iter().enumerate() {
            let w_row = &w[ii * o_dim..(ii + 1) * o_dim];
            for (oo, ov) in out_row.iter_mut().enumerate() {
                *ov += iv * w_row[oo];
            }
        }
    }
}

/// Linear backward: dx[t,i] = `Σ_o` dy[t,o]·w[i,o];
/// dW[i,o] += `Σ_t` in[t,i]·dy[t,o]; db[o] += `Σ_t` dy[t,o].
#[allow(clippy::too_many_arguments)]
fn linear_bwd(
    input: &[f32],
    dy: &[f32],
    w: &[f32],
    t: usize,
    i_dim: usize,
    o_dim: usize,
    dw: &mut [f32],
    db: &mut [f32],
    dx: &mut Vec<f32>,
) {
    dx.clear();
    dx.resize(t * i_dim, 0.0);
    for tt in 0..t {
        let dy_row = &dy[tt * o_dim..(tt + 1) * o_dim];
        let in_row = &input[tt * i_dim..(tt + 1) * i_dim];
        for (oo, &d) in dy_row.iter().enumerate() {
            db[oo] += d;
        }
        for (ii, &iv) in in_row.iter().enumerate() {
            let dw_row = &mut dw[ii * o_dim..(ii + 1) * o_dim];
            for (oo, &d) in dy_row.iter().enumerate() {
                dw_row[oo] += iv * d;
            }
        }
        let dx_row = &mut dx[tt * i_dim..(tt + 1) * i_dim];
        for (ii, dxv) in dx_row.iter_mut().enumerate() {
            let w_row = &w[ii * o_dim..(ii + 1) * o_dim];
            let mut s = 0.0_f32;
            for (oo, &d) in dy_row.iter().enumerate() {
                s += d * w_row[oo];
            }
            *dxv = s;
        }
    }
}

#[inline]
fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + ((2.0 / PI).sqrt() * (x + 0.044_715 * x * x * x)).tanh())
}

#[inline]
fn gelu_grad(x: f32) -> f32 {
    let inner = (2.0 / PI).sqrt() * (x + 0.044_715 * x * x * x);
    let th = inner.tanh();
    let sech2 = 1.0 - th * th;
    f32::midpoint(1.0, th) + 0.5 * x * sech2 * (2.0 / PI).sqrt() * (1.0 + 3.0 * 0.044_715 * x * x)
}

/// Stable in-place softmax.
fn softmax_inplace(v: &mut [f32]) {
    let max = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0_f32;
    for x in v.iter_mut() {
        *x = (*x - max).exp();
        sum += *x;
    }
    let inv = 1.0 / sum;
    for x in v.iter_mut() {
        *x *= inv;
    }
}

#[must_use]
pub fn softmax(v: &[f32]) -> Vec<f32> {
    let mut out = v.to_vec();
    softmax_inplace(&mut out);
    out
}

/// Cross-entropy loss + gradient `p − onehot(label)`, collapse-free
/// (arXiv:2610.31642; the riir-train `math::ce` form, riir-train Issue 651).
/// The naive `p_label − 1` rounds to exactly 0.0 past the f32 margin cliff
/// (≈16.6) while every other `p_i` stays positive, and `−ln p_label` reads 0.
/// Here the LSE EXCLUDES the label: `s = σ(m − z_label) = 1 − p_label`,
/// `grad[i≠label] = s·exp(z_i − max)/Σ_{o≠label}`, `grad[label] = −s`,
/// `loss = softplus(m − z_label)`. Returns `(loss, grad)`.
#[must_use]
pub fn ce_collapse_free(logits: &[f32], label: usize) -> (f32, Vec<f32>) {
    let mut grad = vec![0.0_f32; logits.len()];
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum_excl = 0.0_f32;
    for (i, (g, &z)) in grad.iter_mut().zip(logits).enumerate() {
        if i != label {
            *g = (z - max).exp();
            sum_excl += *g;
        }
    }
    if sum_excl <= 0.0 {
        grad.fill(0.0);
        return (0.0, grad);
    }
    let gap = sum_excl.ln() + max - logits[label]; // m − z_label
    let s = 1.0 / (1.0 + (-gap).exp());
    let scale = s / sum_excl;
    for g in &mut grad {
        *g *= scale;
    }
    grad[label] = -s;
    (gap.max(0.0) + (-gap.abs()).exp().ln_1p(), grad)
}

#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut s = 0.0_f32;
    for (x, y) in a.iter().zip(b.iter()) {
        s += x * y;
    }
    s
}

// ---------------------------------------------------------------------------
// Encoder layer forward/backward
// ---------------------------------------------------------------------------

/// Everything the backward needs from one layer's forward.
struct LayerCache {
    t: usize,
    ln1_out: Vec<f32>,
    ln1_xhat: Vec<f32>,
    ln1_inv: Vec<f32>,
    q: Vec<f32>,
    k: Vec<f32>,
    v: Vec<f32>,
    attn: Vec<f32>, // [heads, t, t] softmax rows
    att_concat: Vec<f32>,
    y: Vec<f32>, // x + proj (LN2's input)
    ln2_out: Vec<f32>,
    ln2_xhat: Vec<f32>,
    ln2_inv: Vec<f32>,
    f1: Vec<f32>,
    g_act: Vec<f32>,
}

/// Forward through one pre-LN layer; returns the output [t,w] + caches.
#[allow(clippy::too_many_lines)]
fn layer_forward(layer: &Layer, x: &[f32], t: usize, cfg: &ScorerConfig) -> (LayerCache, Vec<f32>) {
    let w = cfg.width;
    let h = cfg.heads;
    let hd = w / h;
    let scale = 1.0 / (hd as f32).sqrt();
    let mut c = LayerCache {
        t,
        ln1_out: Vec::new(),
        ln1_xhat: Vec::new(),
        ln1_inv: Vec::new(),
        q: Vec::new(),
        k: Vec::new(),
        v: Vec::new(),
        attn: vec![0.0; h * t * t],
        att_concat: vec![0.0; t * w],
        y: vec![0.0; t * w],
        ln2_out: Vec::new(),
        ln2_xhat: Vec::new(),
        ln2_inv: Vec::new(),
        f1: Vec::new(),
        g_act: Vec::new(),
    };
    ln_fwd(
        x,
        t,
        w,
        &layer.ln1_g,
        &layer.ln1_b,
        &mut c.ln1_out,
        &mut c.ln1_xhat,
        &mut c.ln1_inv,
    );
    linear_fwd(&c.ln1_out, t, w, &layer.wq, &layer.bq, w, &mut c.q);
    linear_fwd(&c.ln1_out, t, w, &layer.wk, &layer.bk, w, &mut c.k);
    linear_fwd(&c.ln1_out, t, w, &layer.wv, &layer.bv, w, &mut c.v);
    for hh in 0..h {
        for i in 0..t {
            let q_row = &c.q[i * w + hh * hd..i * w + (hh + 1) * hd];
            let scores = &mut c.attn[(hh * t + i) * t..(hh * t + i + 1) * t];
            for (j, s) in scores.iter_mut().enumerate() {
                let k_row = &c.k[j * w + hh * hd..j * w + (hh + 1) * hd];
                *s = dot(q_row, k_row) * scale;
            }
            softmax_inplace(scores);
            let out_row = &mut c.att_concat[i * w + hh * hd..i * w + (hh + 1) * hd];
            for (o, ov) in out_row.iter_mut().enumerate() {
                let v_row = &c.v[o * w + hh * hd..o * w + (hh + 1) * hd];
                let mut s = 0.0_f32;
                for (a, vv) in scores.iter().zip(v_row.iter()) {
                    s += a * vv;
                }
                *ov = s;
            }
        }
    }
    let mut proj = Vec::new();
    linear_fwd(&c.att_concat, t, w, &layer.wo, &layer.bo, w, &mut proj);
    for i in 0..t * w {
        c.y[i] = x[i] + proj[i];
    }
    let mut ln2_out = Vec::new();
    ln_fwd(
        &c.y,
        t,
        w,
        &layer.ln2_g,
        &layer.ln2_b,
        &mut ln2_out,
        &mut c.ln2_xhat,
        &mut c.ln2_inv,
    );
    c.ln2_out = ln2_out;
    let mut f1 = Vec::new();
    linear_fwd(&c.ln2_out, t, w, &layer.w1, &layer.b1, cfg.d_ff, &mut f1);
    let g_act: Vec<f32> = f1.iter().copied().map(gelu).collect();
    let mut f2 = Vec::new();
    linear_fwd(&g_act, t, cfg.d_ff, &layer.w2, &layer.b2, w, &mut f2);
    let mut out = f2.clone();
    for i in 0..t * w {
        out[i] += c.y[i];
    }
    c.f1 = f1;
    c.g_act = g_act;
    (c, out)
}

/// Backward through one layer: `dout` [t,w] → `dx` [t,w]; grads accumulated
/// into `g` (the same Layer shape, interpreted as grads).
#[allow(clippy::too_many_lines)]
fn layer_backward(
    layer: &Layer,
    c: &LayerCache,
    dout: &[f32],
    cfg: &ScorerConfig,
    g: &mut Layer,
) -> Vec<f32> {
    let t = c.t;
    let w = cfg.width;
    let h = cfg.heads;
    let hd = w / h;
    let scale = 1.0 / (hd as f32).sqrt();

    // ---- FFN backward: out = y + f2; f2 = gelu(ln2_out@W1+B1)@W2+B2 ----
    let mut dg_act = Vec::new();
    linear_bwd(
        &c.g_act,
        dout,
        &layer.w2,
        t,
        cfg.d_ff,
        w,
        &mut g.w2,
        &mut g.b2,
        &mut dg_act,
    );
    for i in 0..t * cfg.d_ff {
        dg_act[i] *= gelu_grad(c.f1[i]);
    }
    let mut dh2 = Vec::new();
    linear_bwd(
        &c.ln2_out, &dg_act, &layer.w1, t, w, cfg.d_ff, &mut g.w1, &mut g.b1, &mut dh2,
    );
    let mut dx_ln2 = Vec::new();
    ln_bwd(
        &dh2,
        &c.ln2_xhat,
        &c.ln2_inv,
        t,
        w,
        &layer.ln2_g,
        &mut g.ln2_g,
        &mut g.ln2_b,
        &mut dx_ln2,
    );
    // dy = dout (residual) + LN2 backward
    let mut dy = dout.to_vec();
    for i in 0..t * w {
        dy[i] += dx_ln2[i];
    }

    // ---- Attention backward: y = x + proj; proj = att_concat@Wo+Bo ----
    let mut datt_concat = Vec::new();
    linear_bwd(
        &c.att_concat,
        &dy,
        &layer.wo,
        t,
        w,
        w,
        &mut g.wo,
        &mut g.bo,
        &mut datt_concat,
    );

    let mut dq = vec![0.0_f32; t * w];
    let mut dk = vec![0.0_f32; t * w];
    let mut dv = vec![0.0_f32; t * w];
    let mut da = vec![0.0_f32; t];
    let mut ds = vec![0.0_f32; t];
    for hh in 0..h {
        for i in 0..t {
            let att_row = &c.attn[(hh * t + i) * t..(hh * t + i + 1) * t];
            let dao = &datt_concat[i * w + hh * hd..i * w + (hh + 1) * hd];
            // da[j] = dao · v[j]
            for (j, da_j) in da.iter_mut().enumerate() {
                let v_row = &c.v[j * w + hh * hd..j * w + (hh + 1) * hd];
                *da_j = dot(dao, v_row);
            }
            // Σ_m a[m]·da[m]
            let dot_a_da: f32 = att_row.iter().zip(da.iter()).map(|(a, d)| a * d).sum();
            for j in 0..t {
                ds[j] = att_row[j] * (da[j] - dot_a_da);
            }
            // dv[j] += a[j]·dao
            for j in 0..t {
                let a_j = att_row[j];
                let dv_row = &mut dv[j * w + hh * hd..j * w + (hh + 1) * hd];
                for (dvv, &dao_v) in dv_row.iter_mut().zip(dao.iter()) {
                    *dvv += a_j * dao_v;
                }
            }
            // dq[i] += Σ_j ds[j]·k[j]·scale
            {
                let dq_row = &mut dq[i * w + hh * hd..i * w + (hh + 1) * hd];
                for j in 0..t {
                    let ds_j = ds[j] * scale;
                    let k_row = &c.k[j * w + hh * hd..j * w + (hh + 1) * hd];
                    for (dqv, &kv) in dq_row.iter_mut().zip(k_row.iter()) {
                        *dqv += ds_j * kv;
                    }
                }
            }
            // dk[j] += ds[j]·q[i]·scale
            {
                let q_row = &c.q[i * w + hh * hd..i * w + (hh + 1) * hd];
                for j in 0..t {
                    let ds_j = ds[j] * scale;
                    let dk_row = &mut dk[j * w + hh * hd..j * w + (hh + 1) * hd];
                    for (dkv, &qv) in dk_row.iter_mut().zip(q_row.iter()) {
                        *dkv += ds_j * qv;
                    }
                }
            }
        }
    }
    // dh1 = dq@Wq^T + dk@Wk^T + dv@Wv^T
    let mut dx_q = Vec::new();
    let mut dx_k = Vec::new();
    let mut dx_v = Vec::new();
    linear_bwd(
        &c.ln1_out, &dq, &layer.wq, t, w, w, &mut g.wq, &mut g.bq, &mut dx_q,
    );
    linear_bwd(
        &c.ln1_out, &dk, &layer.wk, t, w, w, &mut g.wk, &mut g.bk, &mut dx_k,
    );
    linear_bwd(
        &c.ln1_out, &dv, &layer.wv, t, w, w, &mut g.wv, &mut g.bv, &mut dx_v,
    );
    let mut dh1 = vec![0.0_f32; t * w];
    for i in 0..t * w {
        dh1[i] = dx_q[i] + dx_k[i] + dx_v[i];
    }
    let mut dx_ln1 = Vec::new();
    ln_bwd(
        &dh1,
        &c.ln1_xhat,
        &c.ln1_inv,
        t,
        w,
        &layer.ln1_g,
        &mut g.ln1_g,
        &mut g.ln1_b,
        &mut dx_ln1,
    );
    // dx = dy (residual) + LN1 backward
    let mut dx = dy;
    for i in 0..t * w {
        dx[i] += dx_ln1[i];
    }
    dx
}

// ---------------------------------------------------------------------------
// Model forward
// ---------------------------------------------------------------------------

/// Embed bytes (`byte_emb[b+1`] + pos[i]) into `x`; returns token count.
fn embed_bytes(
    x: &mut Vec<f32>,
    emb: &[f32],
    pos: &[f32],
    bytes: &[u8],
    max_len: usize,
    w: usize,
) -> usize {
    let t = bytes.len().min(max_len);
    x.clear();
    x.resize(t * w, 0.0);
    for (i, &b) in bytes.iter().take(t).enumerate() {
        let emb_row = &emb[(b as usize + 1) * w..(b as usize + 2) * w];
        let pos_row = &pos[i * w..(i + 1) * w];
        let xr = &mut x[i * w..(i + 1) * w];
        for j in 0..w {
            xr[j] = emb_row[j] + pos_row[j];
        }
    }
    t
}

struct StackFwd {
    t: usize,
    out: Vec<f32>,
    caches: Vec<LayerCache>,
}

fn stack_forward(
    layers: &[Layer],
    bytes: &[u8],
    emb: &[f32],
    pos: &[f32],
    max_len: usize,
    cfg: &ScorerConfig,
) -> StackFwd {
    let w = cfg.width;
    let mut x = Vec::new();
    let t = embed_bytes(&mut x, emb, pos, bytes, max_len, w);
    let mut cur = x;
    let mut caches = Vec::with_capacity(layers.len());
    for layer in layers {
        let (cache, out) = layer_forward(layer, &cur, t, cfg);
        caches.push(cache);
        cur = out;
    }
    StackFwd {
        t,
        out: cur,
        caches,
    }
}

/// Context encoding → final-LN'd hidden states [T, W] (+ caches).
fn ctx_forward(m: &ChoiceScorer, ctx_bytes: &[u8]) -> (StackFwd, Vec<f32>, Vec<f32>, Vec<f32>) {
    let w = m.cfg.width;
    let stack = stack_forward(
        &m.ctx_layers,
        ctx_bytes,
        &m.byte_emb,
        &m.pos_ctx,
        m.cfg.ctx_len,
        &m.cfg,
    );
    let mut out = Vec::new();
    let mut xhat = Vec::new();
    let mut inv = Vec::new();
    ln_fwd(
        &stack.out,
        stack.t,
        w,
        &m.ctx_ln_g,
        &m.ctx_ln_b,
        &mut out,
        &mut xhat,
        &mut inv,
    );
    (stack, out, xhat, inv)
}

/// Option encoding → mean-pooled final-LN vector [W] (+ caches).
fn opt_forward(
    m: &ChoiceScorer,
    opt_bytes: &[u8],
) -> (StackFwd, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let w = m.cfg.width;
    let stack = stack_forward(
        &m.opt_layers,
        opt_bytes,
        &m.byte_emb,
        &m.pos_opt,
        m.cfg.opt_len,
        &m.cfg,
    );
    let mut out = Vec::new();
    let mut xhat = Vec::new();
    let mut inv = Vec::new();
    ln_fwd(
        &stack.out,
        stack.t,
        w,
        &m.opt_ln_g,
        &m.opt_ln_b,
        &mut out,
        &mut xhat,
        &mut inv,
    );
    // mean-pool
    let mut p = vec![0.0_f32; w];
    for i in 0..stack.t {
        let row = &out[i * w..(i + 1) * w];
        for j in 0..w {
            p[j] += row[j];
        }
    }
    let inv_s = 1.0 / (stack.t as f32);
    for v in &mut p {
        *v *= inv_s;
    }
    (stack, out, xhat, inv, p)
}

/// The head: K/V over context tokens + per-option q/attention/logit.
struct HeadIO {
    k_mat: Vec<f32>,      // [T,W]
    v_mat: Vec<f32>,      // [T,W]
    logits: Vec<f32>,     // [N]
    p_n: Vec<Vec<f32>>,   // [N][W] option pools
    q_n: Vec<Vec<f32>>,   // [N][W]
    a_n: Vec<Vec<f32>>,   // [N][T] attention rows
    att_n: Vec<Vec<f32>>, // [N][W]
}

fn head_forward(m: &ChoiceScorer, t_ctx: usize, ctx_h: &[f32], opt_pools: &[&[f32]]) -> HeadIO {
    let w = m.cfg.width;
    let scale = 1.0 / (w as f32).sqrt();
    let n = opt_pools.len();
    let mut k_mat = Vec::new();
    let mut v_mat = Vec::new();
    linear_fwd(ctx_h, t_ctx, w, &m.wk, &m.bk, w, &mut k_mat);
    linear_fwd(ctx_h, t_ctx, w, &m.wv, &m.bv, w, &mut v_mat);
    let mut logits = Vec::with_capacity(n);
    let mut q_n = Vec::with_capacity(n);
    let mut a_n = Vec::with_capacity(n);
    let mut att_n = Vec::with_capacity(n);
    for pool in opt_pools {
        let mut q = Vec::new();
        linear_fwd(pool, 1, w, &m.wq, &m.bq, w, &mut q);
        let mut scores = vec![0.0_f32; t_ctx];
        for (j, s) in scores.iter_mut().enumerate() {
            *s = dot(&q, &k_mat[j * w..(j + 1) * w]) * scale;
        }
        softmax_inplace(&mut scores);
        let mut att = vec![0.0_f32; w];
        for j in 0..t_ctx {
            let a_j = scores[j];
            let v_row = &v_mat[j * w..(j + 1) * w];
            for (av, vv) in att.iter_mut().zip(v_row.iter()) {
                *av += a_j * vv;
            }
        }
        let logit = dot(&q, &att) * scale;
        q_n.push(q);
        a_n.push(scores);
        att_n.push(att);
        logits.push(logit);
    }
    HeadIO {
        k_mat,
        v_mat,
        logits,
        p_n: opt_pools.iter().map(|p| p.to_vec()).collect(),
        q_n,
        a_n,
        att_n,
    }
}

/// Everything the episode backward needs.
struct EpisodeCache {
    t_ctx: usize,
    ctx_stack: StackFwd,
    ctx_h: Vec<f32>,
    ctx_xhat: Vec<f32>,
    ctx_inv: Vec<f32>,
    head: HeadIO,
    opt_stacks: Vec<StackFwd>,
    opt_ln_xhat: Vec<Vec<f32>>,
    opt_ln_inv: Vec<Vec<f32>>,
}

fn forward_cached(m: &ChoiceScorer, ctx_bytes: &[u8], options: &[&[u8]]) -> EpisodeCache {
    let (ctx_stack, ctx_h, ctx_xhat, ctx_inv) = ctx_forward(m, ctx_bytes);
    let mut pools: Vec<Vec<f32>> = Vec::with_capacity(options.len());
    let mut opt_stacks = Vec::with_capacity(options.len());
    let mut opt_ln_xhat = Vec::with_capacity(options.len());
    let mut opt_ln_inv = Vec::with_capacity(options.len());
    for opt in options {
        let (stack, _out, xhat, inv, p) = opt_forward(m, opt);
        pools.push(p);
        opt_stacks.push(stack);
        opt_ln_xhat.push(xhat);
        opt_ln_inv.push(inv);
    }
    let pool_refs: Vec<&[f32]> = pools.iter().map(Vec::as_slice).collect();
    let head = head_forward(m, ctx_stack.t, &ctx_h, &pool_refs);
    EpisodeCache {
        t_ctx: ctx_stack.t,
        ctx_stack,
        ctx_h,
        ctx_xhat,
        ctx_inv,
        head,
        opt_stacks,
        opt_ln_xhat,
        opt_ln_inv,
    }
}

impl ChoiceScorer {
    /// Inference forward: one logit per option (same arithmetic as the
    /// training path — the finite-difference checks rely on this).
    #[must_use]
    pub fn forward_logits(&self, ctx_bytes: &[u8], options: &[&[u8]]) -> Vec<f32> {
        let cache = forward_cached(self, ctx_bytes, options);
        cache.head.logits
    }

    /// Cached-pool inference (eval fast path): options pre-encoded via
    /// [`Self::encode_option`], only the context + head run per span.
    #[must_use]
    pub fn forward_logits_cached(&self, ctx_bytes: &[u8], opt_pools: &[&[f32]]) -> Vec<f32> {
        let (stack, ctx_h, _xh, _inv) = ctx_forward(self, ctx_bytes);
        head_forward(self, stack.t, &ctx_h, opt_pools).logits
    }

    /// Public option encoder (the eval cache builder).
    #[must_use]
    pub fn encode_option(&self, opt_bytes: &[u8]) -> Vec<f32> {
        opt_forward(self, opt_bytes).4
    }

    /// CE loss of one episode (the numeric-grad oracle).
    #[must_use]
    pub fn loss(&self, ctx_bytes: &[u8], options: &[&[u8]], label: usize) -> f32 {
        let logits = self.forward_logits(ctx_bytes, options);
        ce_collapse_free(&logits, label).0
    }

    /// One training episode: forward + full backward; CE loss returned,
    /// grads accumulated into `grads` (same-shape `ChoiceScorer`).
    #[allow(clippy::too_many_lines)]
    pub fn forward_backward(
        &self,
        grads: &mut ScorerGrads,
        ctx_bytes: &[u8],
        options: &[&[u8]],
        label: usize,
    ) -> f32 {
        let w = self.cfg.width;
        let scale = 1.0 / (w as f32).sqrt();
        let g = &mut grads.0;
        let cache = forward_cached(self, ctx_bytes, options);
        // ---- dlogits = p − onehot (collapse-free) ----
        let (loss, dlogits) = ce_collapse_free(&cache.head.logits, label);

        let t = cache.t_ctx;
        let mut dk_mat = vec![0.0_f32; t * w];
        let mut dv_mat = vec![0.0_f32; t * w];
        let mut dpools: Vec<Vec<f32>> = Vec::with_capacity(options.len());

        for n in 0..options.len() {
            let dl = dlogits[n];
            let q = &cache.head.q_n[n];
            let att = &cache.head.att_n[n];
            let a_row = &cache.head.a_n[n];
            // logit = scale·(q·att)
            let dq_from_logit: Vec<f32> = att.iter().map(|&a| scale * dl * a).collect();
            let datt: Vec<f32> = q.iter().map(|&qv| scale * dl * qv).collect();
            // da[j] = datt · V[j]; ds[j] = a[j]·(da[j] − Σ a·da)
            let mut da = vec![0.0_f32; t];
            for j in 0..t {
                let v_row = &cache.head.v_mat[j * w..(j + 1) * w];
                da[j] = dot(&datt, v_row);
            }
            let dot_a_da: f32 = a_row.iter().zip(da.iter()).map(|(a, d)| a * d).sum();
            let ds: Vec<f32> = (0..t).map(|j| a_row[j] * (da[j] - dot_a_da)).collect();
            // dv[j] += a[j]·datt ; dK[j] += scale·ds[j]·q
            for j in 0..t {
                let a_j = a_row[j];
                let dv_row = &mut dv_mat[j * w..(j + 1) * w];
                for (dvv, &datt_v) in dv_row.iter_mut().zip(datt.iter()) {
                    *dvv += a_j * datt_v;
                }
                let s_ds = scale * ds[j];
                let dk_row = &mut dk_mat[j * w..(j + 1) * w];
                for (dkv, &qv) in dk_row.iter_mut().zip(q.iter()) {
                    *dkv += s_ds * qv;
                }
            }
            // dq = scale·dl·att + scale·Σ_j ds[j]·K[j]
            let mut dq = dq_from_logit;
            for j in 0..t {
                let s_ds = scale * ds[j];
                let k_row = &cache.head.k_mat[j * w..(j + 1) * w];
                for (dqv, &kv) in dq.iter_mut().zip(k_row.iter()) {
                    *dqv += s_ds * kv;
                }
            }
            // head params: dWq/dbq/dp via the SAME linear_bwd the rest of
            // the graph uses (q = Wq·p + bq, t=1: dWq[i,o] += p_i·dq_o,
            // dbq += dq, dp = Wqᵀ·dq — the hand-written outer product was
            // transposed and caught by the finite-difference gate)
            let pool = &cache.head.p_n[n];
            let mut dp = vec![0.0_f32; w];
            linear_bwd(pool, &dq, &self.wq, 1, w, w, &mut g.wq, &mut g.bq, &mut dp);
            dpools.push(dp);
        }

        // ---- ctx side: dctx_h = Wk^T·dK + Wv^T·dV + head LN/proj grads ----
        let mut dx_k = Vec::new();
        let mut dx_v = Vec::new();
        linear_bwd(
            &cache.ctx_h,
            &dk_mat,
            &self.wk,
            t,
            w,
            w,
            &mut g.wk,
            &mut g.bk,
            &mut dx_k,
        );
        linear_bwd(
            &cache.ctx_h,
            &dv_mat,
            &self.wv,
            t,
            w,
            w,
            &mut g.wv,
            &mut g.bv,
            &mut dx_v,
        );
        let mut dctx_h = vec![0.0_f32; t * w];
        for i in 0..t * w {
            dctx_h[i] = dx_k[i] + dx_v[i];
        }
        // final LN backward
        let mut dstack = Vec::new();
        ln_bwd(
            &dctx_h,
            &cache.ctx_xhat,
            &cache.ctx_inv,
            t,
            w,
            &self.ctx_ln_g,
            &mut g.ctx_ln_g,
            &mut g.ctx_ln_b,
            &mut dstack,
        );
        // ctx layers backward (reverse), then embedding scatter
        let mut d = dstack;
        for l in (0..self.ctx_layers.len()).rev() {
            d = layer_backward(
                &self.ctx_layers[l],
                &cache.ctx_stack.caches[l],
                &d,
                &self.cfg,
                &mut g.ctx_layers[l],
            );
        }
        for (i, &b) in ctx_bytes.iter().take(t).enumerate() {
            let row = &d[i * w..(i + 1) * w];
            let emb_row = &mut g.byte_emb[(b as usize + 1) * w..(b as usize + 2) * w];
            let pos_row = &mut g.pos_ctx[i * w..(i + 1) * w];
            for j in 0..w {
                emb_row[j] += row[j];
                pos_row[j] += row[j];
            }
        }

        // ---- option side ----
        for (n, opt) in options.iter().enumerate() {
            let s = cache.opt_stacks[n].t;
            // pool backward: d(o_ln_out)[s,:] += dp / S
            let mut doln = vec![0.0_f32; s * w];
            let inv_s = 1.0 / (s as f32);
            for i in 0..s {
                let row = &mut doln[i * w..(i + 1) * w];
                for (r, &dpv) in row.iter_mut().zip(dpools[n].iter()) {
                    *r += dpv * inv_s;
                }
            }
            // final LN backward
            let mut dopt_stack = Vec::new();
            ln_bwd(
                &doln,
                &cache.opt_ln_xhat[n],
                &cache.opt_ln_inv[n],
                s,
                w,
                &self.opt_ln_g,
                &mut g.opt_ln_g,
                &mut g.opt_ln_b,
                &mut dopt_stack,
            );
            let mut d = dopt_stack;
            for l in (0..self.opt_layers.len()).rev() {
                d = layer_backward(
                    &self.opt_layers[l],
                    &cache.opt_stacks[n].caches[l],
                    &d,
                    &self.cfg,
                    &mut g.opt_layers[l],
                );
            }
            // embedding scatter
            for (i, &b) in opt.iter().take(s).enumerate() {
                let row = &d[i * w..(i + 1) * w];
                let emb_row = &mut g.byte_emb[(b as usize + 1) * w..(b as usize + 2) * w];
                let pos_row = &mut g.pos_opt[i * w..(i + 1) * w];
                for j in 0..w {
                    emb_row[j] += row[j];
                    pos_row[j] += row[j];
                }
            }
        }
        loss
    }
}

// ---------------------------------------------------------------------------
// AdamW + warmup/cosine schedule
// ---------------------------------------------------------------------------

/// `AdamW` moments over the `paths()` layout.
pub struct AdamW {
    m: ChoiceScorer,
    v: ChoiceScorer,
    pub step: u64,
    pub lr: f32,
    pub b1: f32,
    pub b2: f32,
    pub eps: f32,
    pub wd: f32,
    pub total_steps: u64,
    pub warmup: u64,
    pub min_ratio: f32,
}

impl AdamW {
    #[must_use]
    pub fn new(shape_like: &ChoiceScorer, lr: f32) -> Self {
        Self {
            m: ChoiceScorer::zeroed(&shape_like.cfg),
            v: ChoiceScorer::zeroed(&shape_like.cfg),
            step: 0,
            lr,
            b1: 0.9,
            b2: 0.999,
            eps: 1e-8,
            wd: 0.01,
            total_steps: 1,
            warmup: 1,
            min_ratio: 0.01,
        }
    }

    /// Warmup + cosine decay to `min_ratio`·lr.
    #[must_use]
    pub fn lr_at(base: f32, step: u64, total_steps: u64, warmup: u64, min_ratio: f32) -> f32 {
        if total_steps == 0 {
            return base;
        }
        if step < warmup {
            return base * ((step + 1) as f32 / warmup as f32);
        }
        let progress = ((step - warmup) as f32 / (total_steps - warmup).max(1) as f32).min(1.0);
        let cos = (PI * progress).cos();
        base * (min_ratio + (1.0 - min_ratio) * (1.0 + cos) * 0.5)
    }

    /// One decoupled-weight-decay step (in-place param update).
    pub fn step(&mut self, params: &mut ChoiceScorer, grads: &ScorerGrads) {
        self.step += 1;
        let lr = Self::lr_at(
            self.lr,
            self.step - 1,
            self.total_steps,
            self.warmup,
            self.min_ratio,
        );
        let bc1 = 1.0 - self.b1.powi(self.step as i32);
        let bc2 = 1.0 - self.b2.powi(self.step as i32);
        for p in params.paths() {
            let m = self.m.tensor_mut(p);
            let v = self.v.tensor_mut(p);
            let par = params.tensor_mut(p);
            let gr = grads.0.tensor_ref(p);
            for i in 0..par.len() {
                let gi = gr[i];
                m[i] = self.b1 * m[i] + (1.0 - self.b1) * gi;
                v[i] = self.b2 * v[i] + (1.0 - self.b2) * gi * gi;
                let mh = m[i] / bc1;
                let vh = v[i] / bc2;
                par[i] -= lr * (mh / (vh.sqrt() + self.eps)) + lr * self.wd * par[i];
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — every backward block pinned by finite differences
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Softmax Collapse regression (arXiv:2610.31642): at margin 20 the naive
    /// `p_label − 1` is exactly 0.0; the fixed form matches f64, sums to zero.
    #[test]
    fn ce_collapse_free_survives_the_f32_cliff() {
        let logits = [20.0_f32, 0.0, -1.0];
        assert_eq!(softmax(&logits)[0] - 1.0, 0.0, "premise: naive collapses");
        let (loss, g) = ce_collapse_free(&logits, 0);
        let other = 0.0_f64.exp() + (-1.0_f64).exp();
        let want = -other / (20.0_f64.exp() + other);
        assert!(((f64::from(g[0]) - want) / want).abs() < 1e-5);
        assert!(((f64::from(loss) + want) / want).abs() < 1e-4);
        let sum: f64 = g.iter().map(|&v| f64::from(v)).sum();
        assert!(sum.abs() < 1e-12, "row sums to 0: {sum}");
    }

    fn tiny_cfg() -> ScorerConfig {
        ScorerConfig {
            width: 8,
            heads: 2,
            d_ff: 16,
            ctx_len: 8,
            opt_len: 6,
            ctx_layers: 2,
            opt_layers: 1,
        }
    }

    fn episode() -> (Vec<u8>, Vec<Vec<u8>>, usize) {
        let ctx: Vec<u8> = b"fn a(x: i32) -> i32 { x + 1 }".to_vec();
        let options: Vec<Vec<u8>> = vec![
            b"needless_return\nfn f() { return 1; }".to_vec(),
            b"identity_op\nx + 1".to_vec(),
            b"skip\nno rule".to_vec(),
        ];
        (ctx, options, 1)
    }

    #[test]
    fn param_count_is_cua_s1_class() {
        let m = ChoiceScorer::new(ScorerConfig::default(), 7);
        let n = m.param_count();
        assert!(
            (600_000..800_000).contains(&n),
            "param count {n} outside CUA-S1 class"
        );
        assert_eq!(m.to_flat().len(), n);
    }

    #[test]
    fn deterministic_init_and_serialization_roundtrip() {
        let a = ChoiceScorer::new(ScorerConfig::default(), 42);
        let b = ChoiceScorer::new(ScorerConfig::default(), 42);
        assert_eq!(a.to_flat(), b.to_flat());
        let c = ChoiceScorer::new(ScorerConfig::default(), 43);
        assert_ne!(a.to_flat(), c.to_flat());
        let restored = ChoiceScorer::from_flat(a.cfg, &a.to_flat()).expect("roundtrip");
        assert_eq!(restored.to_flat(), a.to_flat());
    }

    /// The hand-rolled backward's correctness gate, two layers:
    ///
    /// 1. DIRECTIONAL checks (8 seeded random unit directions): analytic
    ///    `g·d` vs the central-difference directional derivative. This
    ///    aggregates the WHOLE gradient into one high-SNR scalar — the f32
    ///    cancellation floor (~5e-6 on L≈1) applies once per direction,
    ///    not per-component, so it validates every tensor's contribution
    ///    including the head's Wq/Wk score path whose per-component grads
    ///    are ~1e-5 (uniform-softmax symmetry) and individually
    ///    FD-unmeasurable at f32 (measured: per-index FD below ~2.5e-3 is
    ///    dominated by deterministic rounding artifacts).
    /// 2. PER-INDEX argmax checks: for each tensor, the largest-|grad|
    ///    index (best FD SNR) where measurable — pins individual formulas
    ///    at the component level.
    ///
    /// Runs on a gently sharpened model (60 steps, lr 3e-4): at init the
    /// loss sits at ln-N uniformity and most grads sit under the per-index
    /// FD noise floor; aggressive sharpening (40 steps, lr 3e-3) instead
    /// enters a high-curvature regime where finite differences stop
    /// converging (measured).
    #[test]
    #[allow(clippy::too_many_lines)]
    fn gradient_check_all_tensors() {
        let cfg = tiny_cfg();
        let mut m = ChoiceScorer::new(cfg, 1234);
        let ctx_a: Vec<u8> = b"fn a(x: i32) -> i32 { x + 1 }".to_vec();
        let opts_a: Vec<Vec<u8>> = vec![
            b"needless_return\nfn f() { return 1; }".to_vec(),
            b"identity_op\nx + 1".to_vec(),
            b"skip\nno rule".to_vec(),
        ];
        let ctx_b: Vec<u8> = b"let s = String::from(\"x\");".to_vec();
        let opts_b: Vec<Vec<u8>> = vec![
            b"manual_string_new\nString::from(\"\")".to_vec(),
            b"single_char_add_str\ns.push('a')".to_vec(),
            b"skip\nno rule".to_vec(),
        ];
        let episodes: Vec<(Vec<u8>, Vec<Vec<u8>>, usize)> =
            vec![(ctx_a, opts_a, 1), (ctx_b, opts_b, 0)];
        let loss_of = |m: &ChoiceScorer| -> f32 {
            let mut s = 0.0_f32;
            for (ctx, opts, label) in &episodes {
                let opt_refs: Vec<&[u8]> = opts.iter().map(Vec::as_slice).collect();
                s += m.loss(ctx, &opt_refs, *label);
            }
            s
        };

        // ---- gentle sharpen, ALTERNATING episodes ----
        // (Sharpening on one episode drives the other's label probability
        // to ~e⁻¹³ on this overfit-prone tiny twin — measured — and the
        // razor-cliff loss surface puts per-direction FD outside its
        // convergence domain. Joint training keeps both episodes' losses
        // in a smooth region.)
        let mut adam = AdamW::new(&m, 1e-3);
        adam.total_steps = 150;
        adam.warmup = 15;
        for step in 0..150 {
            let (ctx, opts, label) = &episodes[step % episodes.len()];
            let opt_refs: Vec<&[u8]> = opts.iter().map(Vec::as_slice).collect();
            let mut g = ScorerGrads::zero_like(&m);
            let _ = m.forward_backward(&mut g, ctx, &opt_refs, *label);
            adam.step(&mut m, &g);
        }
        let loss_after = loss_of(&m);
        assert!(
            loss_after < 1.6,
            "sharpening failed to lift SNR (loss {loss_after:.3} — FD gate cannot run)"
        );

        // ---- analytic grads over both episodes ----
        let mut grads = ScorerGrads::zero_like(&m);
        for (ctx, opts, label) in &episodes {
            let opt_refs: Vec<&[u8]> = opts.iter().map(Vec::as_slice).collect();
            let _l = m.forward_backward(&mut grads, ctx, &opt_refs, *label);
        }

        // ---- layer 1: per-episode directional checks ----
        // Per-EPISODE (not summed): the summed check couples both losses'
        // cliff behavior; each episode's own directional derivative against
        // its own loss is the clean unit (measured: every config passes
        // single-episode; the razor-overfit interplay was the test's own
        // artifact, not the backward's).
        let mut rng = Rng::new(999);
        let h_dir = 5e-2_f32;
        let mut worst_dir_rel = 0.0_f32;
        for (ep_idx, (ctx, opts, label)) in episodes.iter().enumerate() {
            let opt_refs: Vec<&[u8]> = opts.iter().map(Vec::as_slice).collect();
            let mut grads_ep = ScorerGrads::zero_like(&m);
            let _ = m.forward_backward(&mut grads_ep, ctx, &opt_refs, *label);
            for k in 0..4 {
                let mut d = m.to_flat();
                for v in &mut d {
                    *v = rng.normal();
                }
                let norm: f32 = d.iter().map(|v| v * v).sum::<f32>().sqrt();
                for v in &mut d {
                    *v /= norm;
                }
                let mut analytic = 0.0_f32;
                {
                    let mut off = 0usize;
                    for p in m.paths() {
                        let gt = grads_ep.0.tensor_ref(p);
                        for i in 0..gt.len() {
                            analytic += gt[i] * d[off + i];
                        }
                        off += gt.len();
                    }
                }
                let mut off = 0usize;
                for p in m.paths() {
                    let t = m.tensor_mut(p);
                    for v in t.iter_mut() {
                        *v += h_dir * d[off];
                        off += 1;
                    }
                }
                let lp = m.loss(ctx, &opt_refs, *label);
                let mut off = 0usize;
                for p in m.paths() {
                    let t = m.tensor_mut(p);
                    for v in t.iter_mut() {
                        *v -= 2.0 * h_dir * d[off];
                        off += 1;
                    }
                }
                let lmn = m.loss(ctx, &opt_refs, *label);
                let mut off = 0usize;
                for p in m.paths() {
                    let t = m.tensor_mut(p);
                    for v in t.iter_mut() {
                        *v += h_dir * d[off];
                        off += 1;
                    }
                }
                let numeric = (lp - lmn) / (2.0 * h_dir);
                let rel = (analytic - numeric).abs() / (analytic.abs() + numeric.abs() + 1e-4);
                if rel > worst_dir_rel {
                    worst_dir_rel = rel;
                }
                assert!(
                    rel < 5e-2,
                    "directional check ep{ep_idx} dir{k} failed: analytic={analytic:.6} numeric={numeric:.6} rel={rel:.4}"
                );
            }
        }

        // ---- layer 2: per-index argmax checks where measurable ----
        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();
        let mut max_rel: f32 = 0.0;
        for p in m.paths() {
            let n = m.tensor_ref(p).len();
            if n == 0 {
                continue;
            }
            let gt = grads.0.tensor_ref(p);
            let Some((amax, _)) = gt
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            else {
                continue;
            };
            for idx in [amax, 0, n / 2] {
                let analytic = grads.0.tensor_ref(p)[idx];
                let orig = m.tensor_ref(p)[idx];
                let h = (1e-2 * orig.abs()).clamp(1e-3, 5e-2);
                *m.tensor_mut(p).get_mut(idx).expect("idx") = orig + h;
                let lp = loss_of(&m);
                *m.tensor_mut(p).get_mut(idx).expect("idx") = orig - h;
                let lmn = loss_of(&m);
                *m.tensor_mut(p).get_mut(idx).expect("idx") = orig;
                let numeric = (lp - lmn) / (2.0 * h);
                // f32 per-index limits (measured): grads below ~1e-3 are
                // h-unstable; when the clamped h gives a loss delta under
                // ~1e-5 the FD is rounding-dominated and cannot adjudicate
                // — such indices are covered by the directional layer.
                if analytic.abs() + numeric.abs() < 1e-3 || 2.0 * h * analytic.abs() < 1e-5 {
                    continue;
                }
                checked += 1;
                let rel = (analytic - numeric).abs() / (analytic.abs() + numeric.abs() + 1e-4);
                let tol = if analytic.abs() + numeric.abs() < 3e-3 {
                    0.25
                } else {
                    0.08
                };
                if rel > max_rel {
                    max_rel = rel;
                }
                if rel > tol {
                    failures.push(format!(
                        "{p:?}[{idx}] analytic={analytic:.6} numeric={numeric:.6} rel={rel:.4}"
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "per-index gradient check failed (checked {checked}, max_rel={max_rel:.4}):\n{}",
            failures.join("\n")
        );
        assert!(
            worst_dir_rel < 5e-2,
            "directional worst rel {worst_dir_rel:.4}"
        );
        assert!(checked >= 10, "per-index coverage collapsed: {checked}");
    }

    /// A tiny dataset must be learnable — loss drops sharply under `AdamW`.
    #[test]
    fn overfit_tiny_dataset() {
        let cfg = tiny_cfg();
        let mut m = ChoiceScorer::new(cfg, 99);
        let (ctx, opts, label) = episode();
        let opt_refs: Vec<&[u8]> = opts.iter().map(Vec::as_slice).collect();
        let first = m.loss(&ctx, &opt_refs, label);
        let mut opt = AdamW::new(&m, 3e-3);
        opt.total_steps = 200;
        opt.warmup = 20;
        for _ in 0..200 {
            let mut grads = ScorerGrads::zero_like(&m);
            let _ = m.forward_backward(&mut grads, &ctx, &opt_refs, label);
            opt.step(&mut m, &grads);
        }
        let last = m.loss(&ctx, &opt_refs, label);
        assert!(
            last < first * 0.4,
            "loss did not drop enough: {first:.4} -> {last:.4}"
        );
    }

    /// Same seed + same episodes => bit-identical grads (worker reduce's
    /// determinism premise).
    #[test]
    fn grads_deterministic() {
        let cfg = tiny_cfg();
        let m = ChoiceScorer::new(cfg, 5);
        let (ctx, opts, label) = episode();
        let opt_refs: Vec<&[u8]> = opts.iter().map(Vec::as_slice).collect();
        let mut g1 = ScorerGrads::zero_like(&m);
        let mut g2 = ScorerGrads::zero_like(&m);
        let l1 = m.forward_backward(&mut g1, &ctx, &opt_refs, label);
        let l2 = m.forward_backward(&mut g2, &ctx, &opt_refs, label);
        assert_eq!(l1.to_bits(), l2.to_bits());
        assert_eq!(g1.0.to_flat(), g2.0.to_flat());
    }

    #[test]
    fn shuffle_preserves_multiset() {
        let mut rng = Rng::new(3);
        let bytes = b"fn a(x: i32) -> i32 { x + 1 }".to_vec();
        let shuffled = rng.shuffled_bytes(&bytes);
        let mut a = bytes.clone();
        let mut b = shuffled.clone();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        assert_ne!(bytes, shuffled);
    }

    /// The eval fast path (cached option pools) must agree with the plain
    /// forward — the cache is only valid if the arithmetic is identical.
    #[test]
    fn cached_forward_matches_plain() {
        let cfg = tiny_cfg();
        let m = ChoiceScorer::new(cfg, 11);
        let (ctx, opts, _label) = episode();
        let opt_refs: Vec<&[u8]> = opts.iter().map(Vec::as_slice).collect();
        let plain = m.forward_logits(&ctx, &opt_refs);
        let pools: Vec<Vec<f32>> = opts.iter().map(|o| m.encode_option(o)).collect();
        let pool_refs: Vec<&[f32]> = pools.iter().map(Vec::as_slice).collect();
        let cached = m.forward_logits_cached(&ctx, &pool_refs);
        assert_eq!(plain.len(), cached.len());
        for (a, b) in plain.iter().zip(cached.iter()) {
            assert_eq!(a.to_bits(), b.to_bits(), "cached forward diverged");
        }
    }
}
