//! Modelability oracle — scaffold marginal value as a latent-space quality
//! measurement (Issue 64; Research 437 — `arXiv:2608.29335` `GenFirst`
//! Table 5).
//!
//! The paper's measurement: on frozen reconstruction-VAE latents, adding a
//! normalizing-flow scaffold to the AR prior is worth **74 FID**; on
//! `GenFirst` end-to-end latents it is worth **0.15**. The marginal value of
//! a density-modeling scaffold is itself a measurement of how well-shaped
//! the latent space is. Transferred to the healer's latent populations
//! (fix trajectories, retrieval span/corpus embeddings) as a CHEAP,
//! gradient-free oracle:
//!
//! - **Baseline** (the "plain prior"): one mode — held-out mean squared
//!   distance to the population's global mean.
//! - **Scaffold** (the "flow"): a k-means codebook (Lloyd, deterministic
//!   seeded init — EM, no gradient descent) — held-out mean squared
//!   distance to the nearest centroid.
//! - **Oracle value** `v = ln(baseline_mse / scaffold_mse)` — how much
//!   structure a trivial density model captures beyond a single mode.
//! - **Null calibration** (the `mi_est` permutation-test pattern): a raw `v`
//!   conflates cluster structure with the k-means QUANTIZATION gain that
//!   any population yields from k > 1 centroids (measured: a single 2-D
//!   2-D gaussian scores `v_raw` ≈ 1.9 at k=8 — pure Voronoi effect). The
//!   reported value subtracts the gain measured on a diagonal-matched NULL
//!   population (same n, d, mean, per-dim σ; no cross-dimensional or
//!   cluster structure, deterministic seeded draw):
//!   `value = v_raw − v_null` — EXCESS structure beyond the matched
//!   shapeless baseline, in nats.
//!
//! # Pre-registered prediction (falsifiable — Issue 64 protocol)
//!
//! `v` correlates POSITIVELY with downstream retrieval top-1 across
//! corpus/embedder variants (gate: Spearman ≥ 0.8; kill: < 0.5). The
//! mechanism: a well-clustered space is both trivial-modelable (centroids
//! capture it) and retrievable (nearest-neighbour works); a scrambled or
//! diffuse space defeats the scaffold and the retriever alike. Phase 2
//! (the variant sweep over the retrieval surface) validates the gate; this
//! module is the Phase-1 primitive.
//!
//! # Phase 2 verdict (measured 2026-09-02 — `.benchmarks/064`)
//!
//! **NEGATIVE (kill branch): ρ = 0.2256 < 0.5** over 12 population-distinct
//! corpus/chunking variants (`tests/modelability_sweep.rs`). The real
//! retrieval populations are near-structureless by this measurement
//! (−0.20…+0.23 excess nats vs > 1.5 on synthetic clusters) and downstream
//! top-1 on that surface is BM25-discrimination driven, not latent-shape
//! driven (consistent with T3.5's latent-weight refutation + Issue 020's
//! lexical ceiling). The diagnostic delivers its intended message — do not
//! invest embedder/density-scaffold effort on the 8-D modelless space. The
//! primitive stays valid on synthetic populations and standing for
//! re-measurement when a latent-driven surface exists (e.g. a trained
//! embedder passing the Plan 524 Phase 7 gate).
//
//! # Contract
//!
//! - **Modelless only**: bucketed counts + k-means; no trained density
//!   models (Issue 64 constraint). Lloyd is EM — no gradient descent.
//! - **Deterministic**: fixed input ⇒ bit-identical report (the hold-out
//!   split is a fixed stride, the k-means++ seeding is a fixed LCG; no
//!   entropy, no time).
//! - **Held-out honesty**: both fit terms are measured on the held-out
//!   split only — a degenerate overfit (k → `n_train`) cannot drive `v` up,
//!   pinned by test.
//! - **Diagnostic tier only**: never gates a heal decision (the L4
//!   relevance-gate lesson — a cheap signal must not masquerade as a
//!   relevance judge). Suggestion/annotation surface.
//! - **Offline cadence**: k-means is O(iters · n · k · d) with per-call
//!   scratch allocation — an audit-time diagnostic, deliberately NOT a
//!   hot-path primitive (no G4 gate; the allocations are the design).
//!
//! Count → float conversions are module-allowed: populations are
//! audit-sized (≪ 2⁵² points), so the f64 mantissa cannot lose a count's
//! precision.

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

/// Oracle configuration. Defaults: 8 centroids, every 5th point held out,
/// 32 Lloyd iterations — the calibration the synthetic gates pin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelabilityConfig {
    /// Scaffold codebook size (k-means centroids). Default 8.
    pub k: usize,
    /// Every `holdout_mod`-th point (by index) joins the held-out split.
    /// Default 5 (80/20 train/held-out).
    pub holdout_mod: usize,
    /// Lloyd iterations. Default 32.
    pub lloyd_iters: usize,
}

impl Default for ModelabilityConfig {
    fn default() -> Self {
        Self {
            k: 8,
            holdout_mod: 5,
            lloyd_iters: 32,
        }
    }
}

/// One oracle evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelabilityReport {
    /// Held-out mean squared distance to the nearest scaffold centroid
    /// (the scaffold's residual).
    pub scaffold_mse: f64,
    /// Held-out mean squared distance to the global mean (the single-mode
    /// baseline's residual).
    pub baseline_mse: f64,
    /// `ln(baseline_mse / scaffold_mse)` — the RAW gain (quantization
    /// effect included). Epsilon-floored so a degenerate population reports
    /// 0 honestly rather than NaN.
    pub value_raw: f64,
    /// The same raw gain measured on the diagonal-matched null population
    /// (the quantization-gain floor any k-means run produces).
    pub null_value: f64,
    /// `value_raw − null_value` — the ORACLE: excess cluster structure
    /// beyond the matched shapeless baseline, in nats. ~0 on structureless
    /// populations (single gaussians score ~0 by construction); large on
    /// genuinely clustered spaces. Epsilon-floored (see [`scaffold_value`]).
    pub value: f64,
    /// Train-split size.
    pub n_train: usize,
    /// Held-out-split size.
    pub n_holdout: usize,
}

/// Evaluate the modelability oracle over a point population (one point per
/// row, uniform dimension).
///
/// # Panics
///
/// Panics if the population has fewer than `2 * config.holdout_mod` points
/// (both splits must be non-trivial), any row is empty, rows have ragged
/// dimensions, or `config.k == 0`. `k` above the train size is clamped to
/// `n_train` (the scaffold then interpolates; the held-out honesty gate
/// still bounds `v`).
#[must_use]
pub fn scaffold_value(population: &[Vec<f32>], config: &ModelabilityConfig) -> ModelabilityReport {
    assert!(config.k >= 1, "k must be >= 1");
    assert!(config.holdout_mod >= 2, "holdout_mod must be >= 2");
    let n = population.len();
    assert!(
        n >= 2 * config.holdout_mod,
        "population ({n}) must have >= 2 × holdout_mod ({}) points",
        config.holdout_mod
    );
    let d = population[0].len();
    assert!(d > 0, "points must be non-empty");
    for (i, p) in population.iter().enumerate() {
        assert_eq!(p.len(), d, "ragged row {i}");
    }

    // Fixed-stride split: every holdout_mod-th index is held out. No RNG —
    // the split is a deterministic function of the input order.
    let mut train: Vec<&[f32]> = Vec::new();
    let mut holdout: Vec<&[f32]> = Vec::new();
    for (i, p) in population.iter().enumerate() {
        if i % config.holdout_mod == 0 {
            holdout.push(p);
        } else {
            train.push(p);
        }
    }
    let n_train = train.len();
    let n_holdout = holdout.len();
    debug_assert!(n_train >= 1 && n_holdout >= 1);

    let (baseline_mse, scaffold_mse, value_raw) = raw_gain(&train, &holdout, config);

    // Null calibration: draw a diagonal-matched population (same n, d,
    // mean, per-dim σ — no cross-dimensional or cluster structure) and
    // measure the same raw gain on it. The difference is the EXCESS
    // structure, in nats. The null is a pure deterministic function of the
    // population's moments (fixed LCG seed) — bit-identical across runs.
    let null_pop = diagonal_matched_null(population);
    let mut null_train: Vec<&[f32]> = Vec::new();
    let mut null_holdout: Vec<&[f32]> = Vec::new();
    for (i, p) in null_pop.iter().enumerate() {
        if i % config.holdout_mod == 0 {
            null_holdout.push(p);
        } else {
            null_train.push(p);
        }
    }
    let (_, _, null_value) = raw_gain(&null_train, &null_holdout, config);

    ModelabilityReport {
        scaffold_mse,
        baseline_mse,
        value_raw,
        null_value,
        value: value_raw - null_value,
        n_train,
        n_holdout,
    }
}

/// Raw scaffold gain for one (train, hold-out) split — returns
/// `(baseline_mse, scaffold_mse, ln(baseline/scaffold))`. The ln-ratio is
/// epsilon-floored (a degenerate split drives both to ~0 and the ratio to
/// NaN; the honest report is gain 0 — no observable structure difference).
fn raw_gain(train: &[&[f32]], holdout: &[&[f32]], config: &ModelabilityConfig) -> (f64, f64, f64) {
    const EPS: f64 = 1e-12;
    let d = train[0].len();
    let n_train = train.len();
    let n_holdout = holdout.len();

    // Baseline: the train-split global mean, scored on held-out.
    let mut mean = vec![0.0f64; d];
    for p in train {
        for (m, x) in mean.iter_mut().zip(p.iter()) {
            *m += f64::from(*x);
        }
    }
    for m in &mut mean {
        *m /= n_train as f64;
    }
    let baseline_mse = {
        let mut acc = 0.0f64;
        for p in holdout {
            let mut s = 0.0f64;
            for (x, m) in p.iter().zip(mean.iter()) {
                let diff = f64::from(*x) - m;
                s += diff * diff;
            }
            acc += s;
        }
        acc / n_holdout as f64
    };

    // Scaffold: deterministic k-means++ (LCG-seeded) + Lloyd on the train
    // split, scored on held-out.
    let k = config.k.min(n_train);
    let centroids = kmeanspp_init(train, k);
    let centroids = lloyd(train, centroids, config.lloyd_iters);
    let scaffold_mse = {
        let mut acc = 0.0f64;
        for p in holdout {
            acc += min_sq_dist(p, &centroids);
        }
        acc / n_holdout as f64
    };

    let gain = (baseline_mse.max(EPS) / scaffold_mse.max(EPS)).ln();
    (baseline_mse, scaffold_mse, gain)
}

/// Diagonal-matched null population: same n, d, per-point coordinate count;
/// coordinates drawn i.i.d. from `N(mean_j, σ_j)` — the population's own
/// per-dimension moments. Preserves scale (and anisotropy) while destroying
/// cross-dimensional and cluster structure. Fixed LCG seed ⇒ deterministic.
fn diagonal_matched_null(population: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let n = population.len();
    let d = population[0].len();
    let mut mean = vec![0.0f64; d];
    for p in population {
        for (m, x) in mean.iter_mut().zip(p.iter()) {
            *m += f64::from(*x);
        }
    }
    for m in &mut mean {
        *m /= n as f64;
    }
    let mut var = vec![0.0f64; d];
    for (j, v) in var.iter_mut().enumerate() {
        let mut acc = 0.0f64;
        for p in population {
            let diff = f64::from(p[j]) - mean[j];
            acc += diff * diff;
        }
        *v = acc / n as f64;
    }
    let mut rng = Lcg(0xDEAD_BEEF_CAFE_BABE);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut row = Vec::with_capacity(d);
        for j in 0..d {
            let u1 = (rng.unit() + 1e-12).min(1.0);
            let u2 = rng.unit();
            let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
            // The null population stores f32 rows (the population's own
            // element type); the draw is f64 and rounds on store — the
            // null's moments are re-measured from the SAME f32 rows, so
            // no drift enters the calibration.
            #[allow(clippy::cast_possible_truncation)]
            let sample = (mean[j] + var[j].sqrt() * z) as f32;
            row.push(sample);
        }
        out.push(row);
    }
    out
}

/// Squared distance to the NEAREST centroid (f64 accumulation).
fn min_sq_dist(point: &[f32], centroids: &[Vec<f64>]) -> f64 {
    let mut best = f64::INFINITY;
    for c in centroids {
        let mut acc = 0.0f64;
        for (x, m) in point.iter().zip(c.iter()) {
            let diff = f64::from(*x) - m;
            acc += diff * diff;
        }
        if acc < best {
            best = acc;
        }
    }
    best
}

/// Fixed-seed LCG (the entropy.rs test-module constants) — the k-means++
/// probe stream. No entropy, no wall-clock: the init is a pure function of
/// the population.
struct Lcg(u64);
impl Lcg {
    /// Uniform in [0, 1). The u64 → f64 sample takes the LCG's top 31 bits
    /// (already normalized by the division).
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as f64) / f64::from(u32::MAX)
    }
}

/// k-means++ initialization over the train split (deterministic seeded
/// probes): first centroid = train[0] (the fixed-stride split already
/// spreads it), each next centroid sampled proportionally to squared
/// distance by a linear scan over the cumulative weights.
fn kmeanspp_init(train: &[&[f32]], k: usize) -> Vec<Vec<f64>> {
    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);
    centroids.push(train[0].iter().map(|x| f64::from(*x)).collect());
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
    while centroids.len() < k {
        // D² weighting: squared distance to the nearest existing centroid.
        let weights: Vec<f64> = train.iter().map(|p| min_sq_dist(p, &centroids)).collect();
        let total: f64 = weights.iter().sum();
        if total <= 0.0 {
            // All train points coincide with existing centroids — fill the
            // remaining slots with copies (Lloyd keeps them; the held-out
            // score is unaffected).
            centroids.push(centroids[0].clone());
            continue;
        }
        let target = rng.unit() * total;
        let mut cum = 0.0f64;
        let mut pick = train.len() - 1;
        for (i, w) in weights.iter().enumerate() {
            cum += w;
            if cum >= target {
                pick = i;
                break;
            }
        }
        centroids.push(train[pick].iter().map(|x| f64::from(*x)).collect());
    }
    centroids.shrink_to_fit();
    centroids
}

/// Lloyd iterations: assign to nearest centroid, move each centroid to its
/// cluster mean. Empty clusters re-seed at the FARTHEST point from their
/// nearest centroid (deterministic rescue; keeps every centroid live).
fn lloyd(train: &[&[f32]], mut centroids: Vec<Vec<f64>>, iters: usize) -> Vec<Vec<f64>> {
    let d = train[0].len();
    let k = centroids.len();
    for _ in 0..iters {
        let mut sums = vec![vec![0.0f64; d]; k];
        let mut counts = vec![0usize; k];
        for p in train {
            // Nearest centroid.
            let mut best = 0usize;
            let mut best_d = f64::INFINITY;
            for (ci, c) in centroids.iter().enumerate() {
                let mut acc = 0.0f64;
                for (x, m) in p.iter().zip(c.iter()) {
                    let diff = f64::from(*x) - m;
                    acc += diff * diff;
                }
                if acc < best_d {
                    best_d = acc;
                    best = ci;
                }
            }
            counts[best] += 1;
            for (s, x) in sums[best].iter_mut().zip(p.iter()) {
                *s += f64::from(*x);
            }
        }
        for ci in 0..k {
            if counts[ci] == 0 {
                // Empty-cluster rescue: re-seed at the train point farthest
                // from its assigned centroid (deterministic).
                let mut far = 0usize;
                let mut far_d = -1.0f64;
                for (i, p) in train.iter().enumerate() {
                    let dist = min_sq_dist(p, &centroids);
                    if dist > far_d {
                        far_d = dist;
                        far = i;
                    }
                }
                centroids[ci] = train[far].iter().map(|x| f64::from(*x)).collect();
            } else {
                for (m, s) in centroids[ci].iter_mut().zip(sums[ci].iter()) {
                    *m = s / counts[ci] as f64;
                }
            }
        }
    }
    centroids
}

/// Tie-corrected Spearman rank correlation between two equal-length samples
/// (Phase 2 of the Issue 64 protocol: oracle value vs measured downstream
/// top-1 across corpus/chunking variants).
///
/// Ties receive the average of their would-be ranks (standard mid-rank
/// convention); the correlation is Pearson's r over the rank vectors.
/// Returns `None` when the samples differ in length, carry fewer than 2
/// pairs, or either side is constant — no ordering to correlate is
/// "undefined", not 0.
///
/// Diagnostic-tier measurement utility: same posture as
/// [`scaffold_value`] — never gates a heal decision.
#[must_use]
pub fn rank_correlation(xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return None;
    }
    let rx = average_ranks(xs)?;
    let ry = average_ranks(ys)?;
    pearson(&rx, &ry)
}

/// Mid-ranks (ties → the average of their would-be ranks), 1-based.
/// `None` on a constant sample (every rank equal → no ordering).
///
/// The float-equality tests ARE the semantics: rank ties are EXACT value
/// equality (these are the caller's own values; merging near-misses with an
/// epsilon would corrupt the ranks the correlation is defined over).
#[allow(clippy::float_cmp)]
fn average_ranks(v: &[f64]) -> Option<Vec<f64>> {
    let n = v.len();
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut ranks = vec![0.0f64; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for &k in &idx[i..=j] {
            ranks[k] = avg;
        }
        i = j + 1;
    }
    let first = ranks[0];
    if ranks.iter().all(|&r| r == first) {
        None
    } else {
        Some(ranks)
    }
}

/// Pearson's r over two equal-length vectors; `None` on zero variance.
fn pearson(xs: &[f64], ys: &[f64]) -> Option<f64> {
    let n = xs.len() as f64;
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (&x, &y) in xs.iter().zip(ys.iter()) {
        sx += x;
        sy += y;
        sxx += x * x;
        syy += y * y;
        sxy += x * y;
    }
    let cov = sxy - sx * sy / n;
    let vx = sxx - sx * sx / n;
    let vy = syy - sy * sy / n;
    if vx <= 0.0 || vy <= 0.0 {
        return None;
    }
    Some(cov / (vx.sqrt() * vy.sqrt()))
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic gaussian via Box–Muller over a fixed LCG.
    struct Gauss(Lcg);
    impl Gauss {
        fn next(&mut self, mean: f64, sigma: f64) -> f64 {
            let u1 = (self.0.unit() + 1e-12).min(1.0);
            let u2 = self.0.unit();
            mean + sigma * (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
        }
    }

    fn blob_population(
        n_per_blob: usize,
        centers: &[(f64, f64)],
        sigma: f64,
        seed: u64,
    ) -> Vec<Vec<f32>> {
        let mut g = Gauss(Lcg(seed));
        let mut out = Vec::with_capacity(n_per_blob * centers.len());
        for _ in 0..n_per_blob {
            for (cx, cy) in centers {
                out.push(vec![g.next(*cx, sigma) as f32, g.next(*cy, sigma) as f32]);
            }
        }
        out.shrink_to_fit();
        out
    }

    #[test]
    fn clustered_space_scores_high_diffuse_scores_low() {
        // Three tight blobs: the scaffold captures the cluster structure —
        // a large EXCESS value over the diagonal-matched null (the null
        // absorbs the k-means quantization gain any population yields).
        let clustered = blob_population(80, &[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)], 0.5, 42);
        let rep_c = scaffold_value(&clustered, &ModelabilityConfig::default());
        assert!(
            rep_c.value > 1.5,
            "tight blobs must score high (v = {:.3}, raw {:.3}, null {:.3})",
            rep_c.value,
            rep_c.value_raw,
            rep_c.null_value
        );
        // A single diffuse gaussian IS the null shape — the corrected value
        // must sit near zero (the raw gain is pure quantization effect and
        // cancels against the null).
        let diffuse = blob_population(240, &[(0.0, 0.0)], 3.0, 43);
        let rep_d = scaffold_value(&diffuse, &ModelabilityConfig::default());
        assert!(
            rep_d.value.abs() < 0.75,
            "a single gaussian must null out (v = {:.3}, raw {:.3}, null {:.3})",
            rep_d.value,
            rep_d.value_raw,
            rep_d.null_value
        );
        assert!(
            rep_c.value > rep_d.value + 1.0,
            "clustered ({:.3}) must dominate diffuse ({:.3})",
            rep_c.value,
            rep_d.value
        );
    }

    /// The held-out honesty gate: an overfit scaffold (`k` = `n_train`, every
    /// train point its own centroid → train residual 0) must NOT drive the
    /// value up — held-out residual stays the honest estimate.
    #[test]
    fn holdout_resists_overfit() {
        let pop = blob_population(50, &[(0.0, 0.0), (5.0, 5.0)], 1.0, 44);
        let n_train = pop.len() - pop.len() / 5;
        let overfit_cfg = ModelabilityConfig {
            k: n_train,
            ..ModelabilityConfig::default()
        };
        let rep = scaffold_value(&pop, &overfit_cfg);
        let honest_cfg = ModelabilityConfig {
            k: 8,
            ..ModelabilityConfig::default()
        };
        let honest = scaffold_value(&pop, &honest_cfg);
        // The overfit's train residual is exactly 0 by construction; if the
        // report leaked train fit, its value would explode. Held-out, the
        // overfit's centroids sit ON train points — held-out residual is
        // bounded and the two values stay within a modest factor.
        assert!(
            (rep.value - honest.value).abs() < 2.0,
            "k=n_train must not collapse the held-out value (overfit {:.3} vs honest {:.3})",
            rep.value,
            honest.value
        );
    }

    #[test]
    fn determinism_x3_bit_identical() {
        let pop = blob_population(60, &[(0.0, 0.0), (4.0, 7.0), (9.0, 1.0)], 0.8, 45);
        let run = || {
            let r = scaffold_value(&pop, &ModelabilityConfig::default());
            format!(
                "{:e}|{:e}|{:e}|{:e}|{:e}",
                r.scaffold_mse, r.baseline_mse, r.value_raw, r.null_value, r.value
            )
        };
        let r1 = run();
        let r2 = run();
        let r3 = run();
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }

    #[test]
    fn degenerate_population_reports_zero_not_nan() {
        // All points identical: both MSEs → 0, ratio → NaN without the
        // epsilon floor. The honest report is value 0 (no observable
        // structure difference).
        let pop: Vec<Vec<f32>> = vec![vec![1.5, -2.0]; 40];
        let rep = scaffold_value(&pop, &ModelabilityConfig::default());
        assert!(
            rep.value.abs() < 1e-6,
            "degenerate population must report ~0 (v = {:.3e}, scaffold {:.3e}, baseline {:.3e})",
            rep.value,
            rep.scaffold_mse,
            rep.baseline_mse
        );
        assert!(rep.value.is_finite());
    }

    #[test]
    fn validation_panics() {
        let tiny: Vec<Vec<f32>> = vec![vec![0.0, 0.0]; 9]; // < 2 × holdout_mod(5)
        assert!(
            std::panic::catch_unwind(|| scaffold_value(&tiny, &ModelabilityConfig::default()))
                .is_err()
        );
        let ragged = vec![
            vec![0.0, 0.0],
            vec![1.0, 1.0],
            vec![2.0, 2.0, 3.0],
            vec![0.5, 0.5],
            vec![1.5, 1.5],
            vec![2.5, 2.5],
            vec![3.0, 3.0],
            vec![4.0, 4.0],
            vec![5.0, 5.0],
            vec![6.0, 6.0],
        ];
        assert!(std::panic::catch_unwind(|| scaffold_value(
            &ragged,
            &ModelabilityConfig::default()
        ))
        .is_err());
        assert!(std::panic::catch_unwind(|| {
            scaffold_value(
                &vec![vec![0.0, 0.0]; 20],
                &ModelabilityConfig {
                    k: 0,
                    ..ModelabilityConfig::default()
                },
            )
        })
        .is_err());
    }

    #[test]
    fn split_accounting_matches_input() {
        let pop = blob_population(37, &[(0.0, 0.0), (3.0, 3.0)], 0.5, 46);
        let rep = scaffold_value(&pop, &ModelabilityConfig::default());
        assert_eq!(rep.n_train + rep.n_holdout, pop.len());
        assert_eq!(rep.n_holdout, pop.len() / 5 + 1); // indices 0,5,10,...
    }

    #[test]
    fn rank_correlation_monotone_is_perfect() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = [10.0, 20.0, 30.0, 40.0, 50.0];
        let r = rank_correlation(&xs, &ys).expect("non-constant");
        assert!((r - 1.0).abs() < 1e-12, "ascending: {r}");
        let rev = [50.0, 40.0, 30.0, 20.0, 10.0];
        let r = rank_correlation(&xs, &rev).expect("non-constant");
        assert!((r + 1.0).abs() < 1e-12, "descending: {r}");
        // Rank correlation is invariant under any monotone transform of
        // either side (nonlinear spacing).
        let warped = [0.1, 0.5, 7.0, 7.1, 900.0];
        let r = rank_correlation(&xs, &warped).expect("non-constant");
        assert!((r - 1.0).abs() < 1e-12, "warped monotone: {r}");
    }

    #[test]
    fn rank_correlation_ties_use_average_ranks() {
        // x = [1, 1, 2] → ranks [1.5, 1.5, 3]; y = [5, 3, 1] → [3, 2, 1].
        // Pearson over those: cov −1.5, vx 1.5, vy 2 → r = −1.5/√3 ≈ −0.866.
        let r = rank_correlation(&[1.0, 1.0, 2.0], &[5.0, 3.0, 1.0]).expect("non-constant");
        let expected = -1.5 / (1.5f64 * 2.0).sqrt();
        assert!((r - expected).abs() < 1e-12, "tied: {r} vs {expected}");
    }

    #[test]
    fn rank_correlation_undefined_cases_return_none() {
        assert!(rank_correlation(&[1.0], &[2.0]).is_none(), "n < 2");
        assert!(
            rank_correlation(&[1.0, 2.0], &[1.0]).is_none(),
            "length mismatch"
        );
        assert!(
            rank_correlation(&[7.0, 7.0, 7.0], &[1.0, 2.0, 3.0]).is_none(),
            "constant x"
        );
        assert!(
            rank_correlation(&[1.0, 2.0, 3.0], &[4.0, 4.0, 4.0]).is_none(),
            "constant y"
        );
    }
}
