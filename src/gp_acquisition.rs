//! Issue 048 — GP posterior-variance acquisition (`ActFlow` Eq 10) on the
//! healer's shipped seams.
//!
//! Linear-kernel GP over the pipeline's 8-D embedding space
//! ([`crate::embedder::Direction`]): `k(x, y) = x·y`. Posterior variance
//!
//! ```text
//! σ²(x) = k(x,x) − k_xᵀ (K + λI)⁻¹ k_x ,  k_x = X·x
//! ```
//!
//! computed via the **incremental lower-triangular Cholesky** of `(K + λI)`
//! — one rank-1 append per observation (forward solve + one square root),
//! no re-factorization. Unseen inputs get σ² ≈ k(x,x) by construction (the
//! exploration signal — no counts needed); observed inputs collapse toward
//! λ-attenuated residuals. Pure math, zero deps, modelless.
//!
//! ## Why the Cholesky is f64
//!
//! The factor is stored in f64 so the incremental form agrees with a dense
//! f64 reference solve to ~1e-9 relative (the T1 pin; the issue's 1e-6 bar
//! holds with an order of margin). f32 storage drifts ~1e-5 at N=64 —
//! fine for scoring, not fine for a pin.
//!
//! ## Honest scope (T5, pre-registered)
//!
//! σ² over the LOCAL embedder ([`crate::embedder`], pure BLAKE3 hash —
//! near-orthogonal by avalanche) is dominated by `‖x‖²` norm concentration:
//! the surface is expected NEARLY FLAT over distinct inputs. The A/B gate
//! (`tests/t48_gp_acquisition_ab.rs`) measures the spread + the ordering
//! outcome; a flat call is a recorded NEGATIVE, not a failure to hide.
//! The named escalation path is richer features (BM25 TF-IDF dims) before
//! any model change.
//!
//! ## Consumers
//!
//! - **T2** — [`crate::domains::rustc_errors`] candidate ordering (the
//!   `CompileDriver::with_ordering` seam): order repair candidates by σ² of
//!   their trigger-shape embeddings, high-uncertainty first.
//! - **T3** — [`crate::self_evolve::HealPipeline`] selection tie-break
//!   (lexicographic AFTER the `BetaPosterior` combined score — never
//!   overriding payoff), opt-in via `with_gp_tiebreak`.

/// The embedding dimension this module operates on (the pipeline's
/// [`crate::embedder::EMBED_DIM`]).
pub const DIM: usize = crate::embedder::EMBED_DIM;

/// Default nugget: `(K + λI)` jitter. Large enough to keep the incremental
/// factor positive under duplicate observations, small enough that a fully
/// observed input's σ² still collapses well below an orthogonal one's.
pub const DEFAULT_LAMBDA: f32 = 1e-3;

/// Floor for a degenerate Cholesky diagonal (duplicate / in-span input):
/// clamped to `lambda · THIS` so the factorization stays usable.
const DIAG_FLOOR_RATIO: f64 = 1e-4;

/// Incremental linear-kernel GP posterior-variance model.
///
/// Deterministic by construction (no RNG, no clock): same observation
/// sequence → bit-identical σ² everywhere.
#[derive(Debug, Clone, Default)]
pub struct GpAcquisition {
    /// Observed inputs, in observation order.
    xs: Vec<[f32; DIM]>,
    /// Lower-triangular Cholesky factor of `(K + λI)`, row-major with row
    /// `i` holding `i + 1` entries (row start = `i·(i+1)/2`).
    chol: Vec<f64>,
    lambda: f32,
}

impl GpAcquisition {
    /// New empty model with [`DEFAULT_LAMBDA`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_lambda(DEFAULT_LAMBDA)
    }

    /// New empty model with an explicit nugget.
    #[must_use]
    pub fn with_lambda(lambda: f32) -> Self {
        Self {
            xs: Vec::new(),
            chol: Vec::new(),
            lambda: lambda.max(1e-6),
        }
    }

    /// Pre-allocate for `n` observations (each append adds one row; the
    /// Cholesky grows quadratically — `n·(n+1)/2` f64 entries).
    #[must_use]
    pub fn with_capacity(lambda: f32, n: usize) -> Self {
        Self {
            xs: Vec::with_capacity(n),
            chol: Vec::with_capacity(n * (n + 1) / 2),
            lambda: lambda.max(1e-6),
        }
    }

    /// The nugget in use.
    #[must_use]
    pub fn lambda(&self) -> f32 {
        self.lambda
    }

    /// Observations so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.xs.len()
    }

    /// Whether nothing has been observed yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.xs.is_empty()
    }

    /// Observe one input — rank-1 Cholesky append.
    ///
    /// `O(n²)` dots + forward solve against the existing factor; no
    /// re-factorization of the leading block. A duplicate (or exact
    /// linear combination) of prior inputs yields a non-positive diagonal
    /// and is clamped to `λ·1e-4` (the point carries no new information;
    /// the factorization must stay positive-definite).
    pub fn observe(&mut self, x: &[f32; DIM]) {
        let n = self.xs.len();
        let row_start = n * (n + 1) / 2;
        // k_vec[i] = x_i · x for all prior rows (f64 accumulate).
        let mut k_vec = vec![0.0f64; n];
        for (i, xi) in self.xs.iter().enumerate() {
            k_vec[i] = dot_f64(xi, x);
        }
        // Forward-solve L·l = k_vec (l has n entries), accumulating ||l||².
        let mut l = vec![0.0f64; n];
        let mut l_sq_norm = 0.0f64;
        for i in 0..n {
            let row_i = i * (i + 1) / 2;
            let mut s = k_vec[i];
            for (j, lj) in l.iter().enumerate().take(i) {
                s -= self.chol[row_i + j] * *lj;
            }
            let diag = self.chol[row_i + i];
            l[i] = if diag > 0.0 { s / diag } else { 0.0 };
            l_sq_norm += l[i] * l[i];
        }
        let k_xx = dot_f64(x, x);
        let mut diag = k_xx + f64::from(self.lambda) - l_sq_norm;
        let floor = f64::from(self.lambda) * DIAG_FLOOR_RATIO;
        if diag < floor {
            diag = floor;
        }
        self.chol.extend_from_slice(&l);
        self.chol.push(diag.sqrt());
        self.xs.push(*x);
        debug_assert_eq!(self.chol.len(), row_start + n + 1);
    }

    /// Posterior variance at `x` — `k(x,x) − ‖L⁻¹·(X·x)‖²`, clamped ≥ 0.
    ///
    /// Empty model → the prior `k(x,x)`. `O(n²)` against the stored factor.
    #[must_use]
    pub fn sigma_sq(&self, x: &[f32; DIM]) -> f32 {
        let n = self.xs.len();
        let k_xx = dot_f64(x, x);
        if n == 0 {
            #[allow(clippy::cast_possible_truncation)]
            {
                return k_xx as f32;
            }
        }
        // v = L⁻¹·k_vec, σ² = k(x,x) − ‖v‖².
        let mut v_sq_norm = 0.0f64;
        let mut v = vec![0.0f64; n];
        for i in 0..n {
            let row_i = i * (i + 1) / 2;
            let mut s = dot_f64(&self.xs[i], x);
            for (j, vj) in v.iter().enumerate().take(i) {
                s -= self.chol[row_i + j] * *vj;
            }
            let diag = self.chol[row_i + i];
            v[i] = if diag > 0.0 { s / diag } else { 0.0 };
            v_sq_norm += v[i] * v[i];
        }
        let sigma = (k_xx - v_sq_norm).max(0.0);
        #[allow(clippy::cast_possible_truncation)]
        {
            sigma as f32
        }
    }
}

/// Exact-order f64 dot over two 8-D f32 vectors.
#[inline]
fn dot_f64(a: &[f32; DIM], b: &[f32; DIM]) -> f64 {
    let mut s = 0.0f64;
    for i in 0..DIM {
        s += f64::from(a[i]) * f64::from(b[i]);
    }
    s
}

/// σ² over a slice of inputs (convenience for spread probes).
#[must_use]
pub fn sigma_sq_all(gp: &GpAcquisition, xs: &[[f32; DIM]]) -> Vec<f32> {
    xs.iter().map(|x| gp.sigma_sq(x)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic LCG for fixture inputs (no external dep).
    /// (test-support math: the casts + short names ARE the numeric
    /// reference notation — allowed as a block, not suppressed per-site.)
    #[allow(
        clippy::cast_precision_loss,
        clippy::many_single_char_names,
        clippy::needless_range_loop,
        clippy::cast_possible_truncation
    )]
    mod numerics {
        use super::*;
        struct Lcg(u64);
        impl Lcg {
            fn next_f32(&mut self) -> f32 {
                self.0 = self
                    .0
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                ((self.0 >> 40) as f32) / 16_777_216.0
            }
            fn dir(&mut self) -> [f32; DIM] {
                let mut d = [0.0f32; DIM];
                for v in &mut d {
                    *v = self.next_f32() * 2.0 - 1.0;
                }
                d
            }
        }

        /// f64 dense reference: build K+λI, solve, compare σ².
        fn dense_sigma_sq(train: &[[f32; DIM]], x: &[f32; DIM], lambda: f32) -> f64 {
            let n = train.len();
            let mut k = vec![vec![0.0f64; n]; n];
            for i in 0..n {
                for j in 0..n {
                    k[i][j] = dot_f64(&train[i], &train[j]);
                }
                k[i][i] += f64::from(lambda);
            }
            // Gaussian elimination solve (K+λI)·α = k_vec (small n, fine).
            let mut k_vec = vec![0.0f64; n];
            for (i, xi) in train.iter().enumerate() {
                k_vec[i] = dot_f64(xi, x);
            }
            let mut aug = k.clone();
            for i in 0..n {
                aug[i].push(k_vec[i]);
            }
            for col in 0..n {
                // partial pivot
                let mut pivot = col;
                for r in col + 1..n {
                    if aug[r][col].abs() > aug[pivot][col].abs() {
                        pivot = r;
                    }
                }
                aug.swap(col, pivot);
                let d = aug[col][col];
                if d.abs() < 1e-12 {
                    continue;
                }
                for r in col + 1..n {
                    let f = aug[r][col] / d;
                    for c in col..=n {
                        aug[r][c] -= f * aug[col][c];
                    }
                }
            }
            let mut alpha = vec![0.0f64; n];
            for row in (0..n).rev() {
                let mut s = aug[row][n];
                for c in row + 1..n {
                    s -= aug[row][c] * alpha[c];
                }
                let d = aug[row][row];
                alpha[row] = if d.abs() < 1e-12 { 0.0 } else { s / d };
            }
            let k_xx = dot_f64(x, x);
            let mut reduction = 0.0f64;
            for i in 0..n {
                reduction += k_vec[i] * alpha[i];
            }
            (k_xx - reduction).max(0.0)
        }

        #[test]
        fn t1_matches_dense_solve_at_n64() {
            // T1 pin: incremental Cholesky vs f64 dense solve, N=64 training
            // inputs, 16 queries. The issue's bar is 1e-6; f64 storage holds
            // ~1e-9 — asserted at 1e-6 with margin.
            let mut rng = Lcg(42);
            let train: Vec<_> = (0..64).map(|_| rng.dir()).collect();
            let queries: Vec<_> = (0..16).map(|_| rng.dir()).collect();
            let lambda = DEFAULT_LAMBDA;
            let mut gp = GpAcquisition::with_lambda(lambda);
            for x in &train {
                gp.observe(x);
            }
            let mut max_err = 0.0f64;
            for q in &queries {
                let inc = f64::from(gp.sigma_sq(q));
                let dense = dense_sigma_sq(&train, q, lambda);
                let err = (inc - dense).abs();
                assert!(err < 1e-6, "inc={inc} dense={dense} err={err}");
                max_err = max_err.max(err);
            }
            // Honest margin report: well under the bar.
            assert!(max_err < 1e-9, "expected ~1e-9, got {max_err}");
        }

        #[test]
        fn observed_input_collapses_orthogonal_expands() {
            let mut rng = Lcg(7);
            let train: Vec<_> = (0..8).map(|_| rng.dir()).collect();
            let mut gp = GpAcquisition::new();
            for x in &train {
                gp.observe(x);
            }
            // A training point's σ² is small...
            let s_train = gp.sigma_sq(&train[3]);
            // ...while an orthogonal-ish fresh point keeps ≈ ‖x‖².
            // Build a fresh point guaranteed far from the training span: with
            // 8 training points in 8-D the orthogonal complement is thin, so
            // use many more training points than dims and compare the MEANS.
            let fresh: Vec<_> = (0..32).map(|_| rng.dir()).collect();
            let mean_fresh: f64 =
                fresh.iter().map(|x| f64::from(gp.sigma_sq(x))).sum::<f64>() / fresh.len() as f64;
            assert!(
                f64::from(s_train) < mean_fresh * 0.5,
                "train σ²={s_train} should collapse vs fresh mean {mean_fresh}"
            );
            // And σ² never exceeds the prior k(x,x).
            for x in &fresh {
                let prior = dot_f64(x, x);
                assert!(f64::from(gp.sigma_sq(x)) <= prior + 1e-6);
            }
        }

        #[test]
        fn duplicate_observation_is_safe() {
            let x = [0.5, -0.5, 0.25, -0.25, 0.125, -0.125, 0.0625, -0.0625];
            let mut gp = GpAcquisition::new();
            gp.observe(&x);
            gp.observe(&x);
            gp.observe(&x);
            let s = gp.sigma_sq(&x);
            assert!(s.is_finite() && s >= 0.0, "duplicate obs broke σ²: {s}");
            assert!(s < 1e-3, "duplicate should collapse: {s}");
            // Factorization still positive-definite downstream.
            gp.observe(&[1.0; DIM]);
            assert!(gp.sigma_sq(&[1.0; DIM]).is_finite());
        }

        #[test]
        fn empty_model_returns_prior() {
            let gp = GpAcquisition::new();
            let x = [0.3; DIM];
            let expect = dot_f64(&x, &x) as f32;
            assert!((gp.sigma_sq(&x) - expect).abs() < 1e-7);
        }

        #[test]
        fn deterministic_bits() {
            let mut rng = Lcg(99);
            let train: Vec<_> = (0..16).map(|_| rng.dir()).collect();
            let mut a = GpAcquisition::new();
            let mut b = GpAcquisition::new();
            for x in &train {
                a.observe(x);
                b.observe(x);
            }
            let q = rng.dir();
            assert_eq!(a.sigma_sq(&q).to_bits(), b.sigma_sq(&q).to_bits());
            assert_eq!(a.chol, b.chol);
        }
    }
}
