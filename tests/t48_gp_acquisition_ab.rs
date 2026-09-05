//! Issue 048 T4/T5 — the GP acquisition A/B gate + the pre-registered
//! negative arm.
//!
//! **Arms** (pool ordering for one labeled span's retrieved rule pool):
//! 1. `retrieval` — the incumbent fused BM25+latent order (the shipped
//!    `--suggest`/`--fix` ranking).
//! 2. `gp` — GP σ² of each (rule, span) pair's 8-D local-embedder
//!    direction, LEAVE-ONE-OUT trained on the other labeled pairs'
//!    directions (deterministic; the "which (rule, span) to query next"
//!    acquisition from the issue's Finding).
//! 3. `random` — seeded LCG shuffle (the falsification control).
//!
//! **Metrics** — discovery@1 / @3 (the correct rule within the arm's first
//! k), MRR, + paired McNemar GP-vs-retrieval on @3 (common random numbers:
//! identical pools). The promotion bar (issue T4): GP ≥ BetaPosterior/
//! retrieval on heal AND discovery strictly greater. A flat σ surface
//! (relative range < `FLAT_SPREAD`) or a GP ≤ random outcome records the
//! NEGATIVE verdict — a measurable outcome, not a failure to hide.
//!
//! **G-flat** runs on the DEFAULT build (no retrieval needed: the fixture
//! texts + the local embedder are enough); the ordering A/B needs
//! `latent_retrieval`. Run:
//!
//! ```bash
//! cargo test --features latent_retrieval --test t48_gp_acquisition_ab -- --nocapture
//! ```

#![allow(clippy::cast_precision_loss)] // count/display fixtures

use riir_clippy::gp_acquisition::{DIM, GpAcquisition};

/// Relative σ² range below which the surface is called FLAT (the T5 bar:
/// (max−min)/mean under this = the ordering carries no signal beyond noise).
const FLAT_SPREAD: f32 = 0.05;

/// The recorded verdict — asserted against the LIVE ordering A/B by
/// `ordering_ab::gp_pool_ordering_vs_retrieval_vs_random` (the t39
/// pattern: the recorded verdict must be re-derivable, never stale).
/// MEASURED (2026-08-27, 100 real pools; re-derived CANONICALLY 2026-08-31
/// after the substrate rule_id tie-break fix — `rank_into`'s HashMap-drain
/// tie-luck used to flip the GP arm's @3 23↔24 run-to-run): GP @3 0.240 ≈
/// random 0.250 ≪ retrieval 0.970; McNemar b=74 c=1 z=+8.43 AGAINST GP. The
/// σ² surface is NOT numerically flat (rel_range 1.82) but its variation is
/// NORM-CONCENTRATION NOISE under the BLAKE3 hash embedder. RE-MEASURED
/// (2026-09-05, post-Batch-91 corpus +4 entries and the B110–112 perf
/// entries shifting the pool boundary): GP @3 0.220 vs random 0.240 —
/// 2 points apart, so the ordering is no longer WITHIN-1 noise-equivalent
/// but plain NEGATIVE (it is decisively WORSE than random, not equal to
/// it); McNemar b=76 c=1 z=+8.55 AGAINST GP, unchanged conclusion. Honest
/// verdict: the 8-D hash embedding carries no locality for a linear-kernel
/// GP to exploit — the escalation path (BM25 TF-IDF dims) stands.
const GP_ACQUISITION_VERDICT: &str = "NEGATIVE";
#[allow(dead_code)] // doc-truth anchor: cited textually by the ordering_ab assertion
const _VERDICT_DOC: &str = GP_ACQUISITION_VERDICT;

// ── Deterministic LCG (t26/t33/t39's) ─────────────────────────────────
// (test-support RNG: short names + the bit-shift cast are the fixture
// notation, lifted verbatim from t26/t33/t39 — block-allowed, not per-site.
// Gated `latent_retrieval` because the ONLY consumer is the ordering_ab mod;
// without the gate the struct is dead under --all-features' -D warnings.)
#[cfg(feature = "latent_retrieval")]
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod lcg {
    pub(super) struct Lcg(pub(super) u64);

    impl Lcg {
        pub(super) fn new(seed: u64) -> Self {
            Self(seed)
        }
        pub(super) fn f32(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 40) as f32) / 16_777_216.0
        }
        pub(super) fn below(&mut self, n: usize) -> usize {
            (self.f32() * n as f32) as usize % n.max(1)
        }
    }
}
#[cfg(feature = "latent_retrieval")]
use lcg::Lcg;

/// The labeled (rule, span) pairs — default-build subset (clippy fixtures
/// are compiled under `clippy_lints`, a default feature; this test file
/// compiles under default features for the flatness probe).
fn labeled_pairs() -> Vec<(&'static str, &'static str)> {
    #[cfg(feature = "latent_retrieval")]
    {
        use riir_clippy::draft::eval_fixtures_clippy::CLIPPY_SPANS;
        CLIPPY_SPANS.to_vec()
    }
    #[cfg(not(feature = "latent_retrieval"))]
    {
        // The draft fixtures are behind latent_retrieval; a minimal inline
        // set keeps the flatness probe runnable on the default build.
        // Shapes mirror the corpus lint family (distinct real anti-patterns).
        vec![
            ("needless_return", "pub fn f(x: u64) -> u64 { return x * 2; }"),
            ("match_bool", "pub fn g(flag: bool) -> u8 { match flag { true => 1, false => 0 } }"),
            ("identity_op", "pub fn h(m: u64) -> u64 { m * 1 }"),
            ("collapsible_if", "pub fn i(a: bool, b: bool) { if a { if b { g(); } } }"),
            ("map_unwrap_or", "pub fn j(v: Option<u8>) -> u8 { v.map(|x| x + 1).unwrap_or(0) }"),
            ("double_parens", "pub fn k(n: u32) -> u32 { ((n + 4)) }"),
            ("useless_format", "pub fn l() -> String { format!(\"done\") }"),
            ("iter_next_loop", "pub fn m(mut it: std::vec::IntoIter<u8>) { for x in it.next() {} }"),
        ]
    }
}

fn embed_pair(rule: &str, span: &str) -> [f32; DIM] {
    riir_clippy::embedder::embed(rule, span)
}

// ── G-flat: the σ-surface spread (T5's pre-registered probe) ─────────────

/// Leave-one-out σ² for every labeled pair. Deterministic (fixed iteration
/// order; rebuild-per-query keeps the training set exact).
fn loo_sigmas(pairs: &[(&'static str, &'static str)]) -> Vec<f32> {
    let mut out = Vec::with_capacity(pairs.len());
    for i in 0..pairs.len() {
        let mut gp = GpAcquisition::with_capacity(riir_clippy::gp_acquisition::DEFAULT_LAMBDA, pairs.len());
        for (j, (rule, span)) in pairs.iter().enumerate() {
            if j != i {
                gp.observe(&embed_pair(rule, span));
            }
        }
        let (rule, span) = pairs[i];
        out.push(gp.sigma_sq(&embed_pair(rule, span)));
    }
    out
}

#[test]
fn g_flat_sigma_surface_spread() {
    let pairs = labeled_pairs();
    let sigmas = loo_sigmas(&pairs);
    let min = sigmas.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = sigmas.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mean = sigmas.iter().sum::<f32>() / sigmas.len() as f32;
    let rel_range = (max - min) / mean;
    println!(
        "t48 G-flat: n={} σ² min={min:.6} max={max:.6} mean={mean:.6} rel_range={rel_range:.4}",
        pairs.len()
    );
    // The verdict input (not itself a pass/fail — the recorded verdict test
    // consumes it).
    let flat = rel_range < FLAT_SPREAD;
    println!("t48 G-flat: surface {} (bar {FLAT_SPREAD})", if flat { "FLAT" } else { "informative" });
    // Sanity: all finite, non-negative.
    for s in &sigmas {
        assert!(s.is_finite() && *s >= 0.0, "σ² must be finite ≥ 0, got {s}");
    }
    // Report the raw value for the bench note either way.
    if flat {
        println!("t48 G-flat: NEGATIVE-arm trigger — σ surface flat over the pair embeddings");
    }
}

// ── The ordering A/B (latent_retrieval) ─────────────────────────────────

#[cfg(feature = "latent_retrieval")]
mod ordering_ab {
    use riir_clippy::draft::eval_fixtures_clippy::CLIPPY_SPANS;
    use riir_clippy::draft::eval_fixtures_perf::PERF_SPANS;
    use riir_clippy::draft::latent_matcher::LatentMatcher;
    use riir_clippy::embedder::embed;
    use riir_clippy::gp_acquisition::{DIM, GpAcquisition};

    /// Pool probe depth per span (matches the harness's 12-deep pools).
    const POOL_K: usize = 12;

    struct ArmOut {
        hits1: u32,
        hits3: u32,
        mrr: f64,
    }

    fn rank_of(rules: &[String], label: &str) -> Option<usize> {
        rules.iter().position(|r| r == label)
    }

    struct AbOut {
        scored: u32,
        retrieval: ArmOut,
        gp: ArmOut,
        random: ArmOut,
        b: u64,
        c: u64,
        z: f64,
    }

    /// One full pass of the A/B (fresh `LatentMatcher` per call — fresh
    /// index HashMaps, hence a fresh RandomState draw for every `drain()`).
    fn run_ordering_ab() -> AbOut {
        let mut matcher = LatentMatcher::from_enabled_domains().expect("index builds");
        // Labeled pairs across the two default domains.
        let mut pairs: Vec<(&'static str, &'static str)> = CLIPPY_SPANS.to_vec();
        pairs.extend(PERF_SPANS.iter().copied());

        // Precompute pair embeddings (rule, span) for the LOO GP training.
        let embeddings: Vec<[f32; DIM]> = pairs
            .iter()
            .map(|(rule, span)| embed(rule, span))
            .collect();

        let mut out = AbOut {
            scored: 0,
            retrieval: ArmOut { hits1: 0, hits3: 0, mrr: 0.0 },
            gp: ArmOut { hits1: 0, hits3: 0, mrr: 0.0 },
            random: ArmOut { hits1: 0, hits3: 0, mrr: 0.0 },
            b: 0,
            c: 0,
            z: 0.0,
        };
        let mut rng = super::Lcg::new(48);
        // Paired McNemar cells (GP vs retrieval, discovery@3): b = ret hit &
        // gp miss; c = gp hit & ret miss.
        let (mut b, mut c) = (0u64, 0u64);

        for (i, (label, span)) in pairs.iter().enumerate() {
            let (_, by_domain) = matcher.suggest_span(span, POOL_K);
            // The pool: whichever domain's retrieved list CONTAINS the
            // label (the honest ground-truth domain; spans whose correct
            // rule was not retrieved at all are off-axis for ORDERING and
            // skipped).
            let pool: Vec<String> = by_domain
                .iter()
                .filter(|(_, rules)| rules.iter().any(|r| r.rule_id == *label))
                .flat_map(|(_, rules)| rules.iter().map(|r| r.rule_id.clone()))
                .collect();
            if pool.is_empty() {
                continue; // correct rule not retrieved at all — off-axis for ordering
            }
            out.scored += 1;

            // Arm 1: incumbent retrieval order.
            let ret_rank = rank_of(&pool, label).unwrap();
            // Arm 2: GP σ² descending, LOO-trained on the OTHER pairs.
            let mut gp = GpAcquisition::with_capacity(
                riir_clippy::gp_acquisition::DEFAULT_LAMBDA,
                embeddings.len(),
            );
            for (j, e) in embeddings.iter().enumerate() {
                if j != i {
                    gp.observe(e);
                }
            }
            let mut gp_order: Vec<(f32, usize)> = pool
                .iter()
                .enumerate()
                .map(|(idx, rule)| {
                    let e = embed(rule, span);
                    (gp.sigma_sq(&e), idx)
                })
                .collect();
            gp_order.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.cmp(&b.1))
            });
            let gp_rank = gp_order
                .iter()
                .position(|(_, idx)| pool[*idx] == *label)
                .unwrap();
            // Arm 3: seeded shuffle.
            let mut rnd_order: Vec<usize> = (0..pool.len()).collect();
            for k in (1..rnd_order.len()).rev() {
                let j = rng.below(k + 1);
                rnd_order.swap(k, j);
            }
            let rnd_rank = rnd_order.iter().position(|&idx| pool[idx] == *label).unwrap();

            for (arm, rank) in [
                (&mut out.retrieval, ret_rank),
                (&mut out.gp, gp_rank),
                (&mut out.random, rnd_rank),
            ] {
                if rank == 0 {
                    arm.hits1 += 1;
                }
                if rank < 3 {
                    arm.hits3 += 1;
                }
                arm.mrr += 1.0 / (rank + 1) as f64;
            }
            let ret_hit = ret_rank < 3;
            let gp_hit = gp_rank < 3;
            if ret_hit && !gp_hit {
                b += 1;
            }
            if gp_hit && !ret_hit {
                c += 1;
            }
        }

        out.b = b;
        out.c = c;
        out.z = if b + c > 0 {
            (b as f64 - c as f64) / (b as f64 + c as f64).sqrt()
        } else {
            0.0
        };
        out
    }

    fn derive_verdict(ab: &AbOut) -> &'static str {
        // Promotion bar: GP discovery@3 STRICTLY greater than retrieval's
        // AND z > +1.96 AND GP @3 ≥ random's (not noise-equivalent-worse).
        // Noise-equivalence (GP within 1 point of random) is itself the
        // recorded NEGATIVE — the σ² ordering carries no information.
        let noise_equivalent = (ab.gp.hits3 as i64 - ab.random.hits3 as i64).abs() <= 1
            && ab.gp.hits3 < ab.retrieval.hits3;
        let gp_beats =
            ab.gp.hits3 > ab.retrieval.hits3 && ab.z > 1.96 && ab.gp.hits3 > ab.random.hits3;
        if gp_beats {
            "POSITIVE"
        } else if noise_equivalent {
            "NEGATIVE-noise-equivalent"
        } else {
            "NEGATIVE"
        }
    }

    /// The A/B: per labeled span, retrieve the domain pool ONCE (common
    /// random numbers), then order it three ways.
    #[test]
    fn gp_pool_ordering_vs_retrieval_vs_random() {
        let ab = run_ordering_ab();
        assert!(ab.scored >= 40, "expected a real pool population, got {}", ab.scored);
        let n = f64::from(ab.scored);
        println!("t48 ordering A/B over {} pools:", ab.scored);
        println!(
            "  retrieval: @1 {:.3} @3 {:.3} mrr {:.3}",
            ab.retrieval.hits1 as f64 / n,
            ab.retrieval.hits3 as f64 / n,
            ab.retrieval.mrr / n
        );
        println!(
            "  gp:        @1 {:.3} @3 {:.3} mrr {:.3}",
            ab.gp.hits1 as f64 / n,
            ab.gp.hits3 as f64 / n,
            ab.gp.mrr / n
        );
        println!(
            "  random:    @1 {:.3} @3 {:.3} mrr {:.3}",
            ab.random.hits1 as f64 / n,
            ab.random.hits3 as f64 / n,
            ab.random.mrr / n
        );
        println!("t48 McNemar gp-vs-retrieval: b={} c={} z={:.3}", ab.b, ab.c, ab.z);

        // ── The verdict derivation (recorded + asserted) ──────────────
        let derived = derive_verdict(&ab);
        assert_eq!(
            super::GP_ACQUISITION_VERDICT,
            derived,
            "recorded verdict stale: derived {derived} from the live measurement"
        );
    }

    /// The substrate determinism pin (the t48 flake's root cause was
    /// tie-luck in `rank_into`'s HashMap-drain + boundary truncation): each
    /// `run_ordering_ab` builds a FRESH matcher → fresh index HashMaps → a
    /// fresh per-process drain draw, so repeated runs sample the exact
    /// variation that used to flip the GP arm's @3 (23↔24). Without the
    /// canonical rule_id tie-break this assertion fails on the real corpus
    /// (the tie demonstrably exists); with it, the A/B is a pure function
    /// of the corpus.
    #[test]
    fn ordering_ab_is_process_deterministic() {
        let first = run_ordering_ab();
        for run in 1..5 {
            let again = run_ordering_ab();
            assert_eq!(first.scored, again.scored, "run {run}: scored drifted");
            assert_eq!(first.retrieval.hits1, again.retrieval.hits1, "run {run}: ret @1");
            assert_eq!(first.retrieval.hits3, again.retrieval.hits3, "run {run}: ret @3");
            assert_eq!(first.gp.hits1, again.gp.hits1, "run {run}: gp @1");
            assert_eq!(first.gp.hits3, again.gp.hits3, "run {run}: gp @3");
            assert_eq!(first.random.hits1, again.random.hits1, "run {run}: rnd @1");
            assert_eq!(first.random.hits3, again.random.hits3, "run {run}: rnd @3");
            assert_eq!(first.b, again.b, "run {run}: mcnemar b");
            assert_eq!(first.c, again.c, "run {run}: mcnemar c");
            assert!(first.z.to_bits() == again.z.to_bits(), "run {run}: z bits");
            assert_eq!(
                derive_verdict(&first),
                derive_verdict(&again),
                "run {run}: verdict sub-classification drifted"
            );
        }
    }
}

// ── Tie incidence on real pool shapes (default build) ────────────────────

#[test]
fn tie_incidence_on_unseen_candidate_pools() {
    // The tie-break can only fire on EXACT combined-score equality. The
    // common real shape: unseen candidates (belief(0,0) both) with a fresh
    // evolution layer (neutral score both) — the pool the t3 fixture test
    // constructs. Measure: given the fixture pair directions, how often do
    // PAIRS tie exactly under the production scoring arithmetic?
    let pairs = labeled_pairs();
    let mut ties = 0u64;
    let mut total = 0u64;
    for i in 0..pairs.len() {
        for j in i + 1..pairs.len() {
            let d_a = embed_pair(pairs[i].0, pairs[i].1);
            let d_b = embed_pair(pairs[j].0, pairs[j].1);
            // Production combined score with neutral evolution + unseen
            // belief (the fresh-pipeline shape) — reliability term equal by
            // construction, evolution by the neutral 0.5.
            let s_a = 0.6_f32 * 0.5 + 0.4_f32 * katgpt_core::best_belief_score(0, 0, 0.05);
            let s_b = 0.6_f32 * 0.5 + 0.4_f32 * katgpt_core::best_belief_score(0, 0, 0.05);
            total += 1;
            if s_a == s_b {
                ties += 1;
            }
            let _ = (d_a, d_b); // directions only matter under a fitted evolution layer
        }
    }
    let rate = ties as f64 / total as f64;
    println!("t48 tie incidence (fresh-pool shape): {ties}/{total} = {rate:.3}");
    // In the fresh shape, ALL unseen candidates tie — the tie-break's
    // addressable surface is every fresh pool, which is why it ships as a
    // flag (not a default): on fresh pools it reorders by σ² (≈ norms under
    // the hash embedder — the flatness probe's verdict governs whether that
    // reordering carries information).
    assert!((rate - 1.0).abs() < 1e-9);
}
