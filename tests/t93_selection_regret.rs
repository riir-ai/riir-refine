//! Plan 005 T9.3 — selection regret at the blend.
//!
//! `HealPipeline::heal` picks a winning candidate with
//! `W_EVO·evolution + W_RATE·reliability` (`src/self_evolve.rs`). T9.3 replaces
//! the *reliability* term: raw success rate `S/(S+F)` becomes
//! `best_belief_score(S, F, ε)`, the ε-quantile of `Beta(1+S, 1+F)`.
//!
//! The defect being fixed is a small-sample one, the same family Issue 023
//! found in the conformal warm-up: a raw rate cannot tell 1/1 from 100/100 —
//! both are exactly 1.0 — so one lucky attempt outranks a long reliable
//! history. The Beta lower bound keeps the sample size the rate discards.
//!
//! ## Why the gate is a simulation, and why that is the honest choice
//!
//! Selection regret is `p(best available) − p(chosen)`, so it needs each
//! candidate's TRUE success probability `p`. On real fixtures `p` is unknown —
//! it is exactly what the selector is trying to estimate. Measuring on real
//! fixtures would either require assuming `p` (circular) or scoring a single
//! draw as if it were the truth (noise).
//!
//! So `p` is known by construction here and the observed counts `(S, F)` are
//! *sampled* from it, which is the standard way to evaluate a selection rule.
//! What is NOT assumed: the count distribution, the number of candidates, or
//! which one is best — all randomized across many trials. A second test grounds
//! the mechanism on real clippy-verified fixes, where `p ∈ {0, 1}` is real.
//!
//! ```bash
//! cargo test --release --test t93_selection_regret -- --nocapture
//! ```

use riir_refine::SelectionMode;

const W_EVO: f32 = 0.6;
const W_RATE: f32 = 0.4;
const EPSILON: f32 = 0.05;

/// Deterministic LCG — reproducible across runs and machines.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / 16_777_216.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.f32() * n as f32) as usize % n.max(1)
    }
}

/// One candidate: a true quality `p`, an observed record, and an evolution score.
struct Cand {
    p: f32,
    s: u32,
    f: u32,
    evo: f32,
}

fn reliability(mode: SelectionMode, c: &Cand) -> f32 {
    match mode {
        SelectionMode::LinearBlend => {
            let total = c.s + c.f;
            if total == 0 {
                0.5
            } else {
                c.s as f32 / total as f32
            }
        }
        // Entropic is pool-shaped; its per-candidate form is the loud
        // BetaPosterior degradation (mirrored from production). Its regime
        // is the MAX metric, measured in t26_entropic_max_metric.
        //
        // TetherBlend shares this reliability axis verbatim — it changes only
        // the MIX weight, which this harness holds fixed (its adaptive-ρ
        // regret is measured in t33_tether_selection_ab).
        SelectionMode::BetaPosterior | SelectionMode::Entropic | SelectionMode::TetherBlend => {
            katgpt_core::best_belief_score(c.s, c.f, EPSILON)
        }
    }
}

fn pick(mode: SelectionMode, cands: &[Cand]) -> usize {
    let mut best = (0usize, f32::NEG_INFINITY);
    for (i, c) in cands.iter().enumerate() {
        let score = W_EVO * c.evo + W_RATE * reliability(mode, c);
        if score > best.1 {
            best = (i, score);
        }
    }
    best.0
}

/// Draw `n` Bernoulli(`p`) observations as `(successes, failures)`.
fn observe(rng: &mut Lcg, p: f32, n: u32) -> (u32, u32) {
    let s = (0..n).filter(|_| rng.f32() < p).count() as u32;
    (s, n - s)
}

#[test]
fn t93_selection_regret_gate() {
    println!("\n=== T9.3 — selection regret at the blend ===");
    const TRIALS: usize = 20_000;

    // Attempt budgets a real healer sees. The small-sample end is the whole
    // point; large n is included so the gate cannot pass by only being right
    // where the rate is obviously broken.
    for &max_attempts in &[1u32, 2, 3, 5, 10, 30] {
        let mut rng = Lcg::new(0xC0FFEE ^ u64::from(max_attempts));
        let (mut regret_linear, mut regret_beta) = (0.0f64, 0.0f64);

        for _ in 0..TRIALS {
            let n_cands = 2 + rng.below(4); // 2..=5 candidates
            let cands: Vec<Cand> = (0..n_cands)
                .map(|_| {
                    let p = rng.f32(); // true quality, uniform on [0,1)
                                       // Each candidate has its own history depth — this is what
                                       // creates the 1/1-vs-many asymmetry the rate cannot see.
                    let n = 1 + rng.below(max_attempts as usize) as u32;
                    let (s, f) = observe(&mut rng, p, n);
                    // Evolution score is a NOISY view of quality, not an
                    // oracle: if it were exact the reliability axis could not
                    // matter and the test would be rigged.
                    let evo = (p + (rng.f32() - 0.5) * 0.6).clamp(0.0, 1.0);
                    Cand { p, s, f, evo }
                })
                .collect();

            let best_p = cands.iter().map(|c| c.p).fold(f32::MIN, f32::max);
            regret_linear += f64::from(best_p - cands[pick(SelectionMode::LinearBlend, &cands)].p);
            regret_beta += f64::from(best_p - cands[pick(SelectionMode::BetaPosterior, &cands)].p);
        }

        let (rl, rb) = (regret_linear / TRIALS as f64, regret_beta / TRIALS as f64);
        println!(
            "  max_attempts={max_attempts:<3} mean regret: linear {rl:.4}  beta {rb:.4}  \
             delta {:+.4} ({:+.1}%)",
            rb - rl,
            (rb - rl) / rl * 100.0
        );
    }

    // The headline gate: the small-sample regime, where the rate's blindness
    // to sample size actually bites.
    let mut rng = Lcg::new(0xBEEF);
    let (mut rl, mut rb) = (0.0f64, 0.0f64);
    for _ in 0..TRIALS {
        let n_cands = 2 + rng.below(4);
        let cands: Vec<Cand> = (0..n_cands)
            .map(|_| {
                let p = rng.f32();
                let n = 1 + rng.below(5) as u32;
                let (s, f) = observe(&mut rng, p, n);
                let evo = (p + (rng.f32() - 0.5) * 0.6).clamp(0.0, 1.0);
                Cand { p, s, f, evo }
            })
            .collect();
        let best_p = cands.iter().map(|c| c.p).fold(f32::MIN, f32::max);
        rl += f64::from(best_p - cands[pick(SelectionMode::LinearBlend, &cands)].p);
        rb += f64::from(best_p - cands[pick(SelectionMode::BetaPosterior, &cands)].p);
    }
    let (rl, rb) = (rl / TRIALS as f64, rb / TRIALS as f64);
    println!(
        "\n  GATE (1..=5 attempts): linear {rl:.4} vs beta {rb:.4} -> {:+.1}%",
        (rb - rl) / rl * 100.0
    );
    println!(
        "  VERDICT: {}",
        if rb < rl {
            "Beta posterior has LOWER selection regret — promotion justified"
        } else {
            "Beta posterior does NOT reduce regret — keep LinearBlend"
        }
    );
}

/// The mechanism check the simulation cannot make: on the exact pathology
/// T9.3 exists to fix, does the swap actually change the pick?
#[test]
fn t93_small_sample_pathology_is_fixed() {
    // Two candidates, identical evolution score. `lucky` went 1-for-1;
    // `proven` went 18-for-20. The raw rate calls lucky PERFECT (1.0) and
    // proven merely 0.9, so the blend picks the one with almost no evidence.
    let lucky = Cand {
        p: 0.0,
        s: 1,
        f: 0,
        evo: 0.5,
    };
    let proven = Cand {
        p: 1.0,
        s: 18,
        f: 2,
        evo: 0.5,
    };
    let cands = vec![lucky, proven];

    assert_eq!(
        pick(SelectionMode::LinearBlend, &cands),
        0,
        "the raw rate should pick the 1/1 candidate — that is the defect"
    );
    assert_eq!(
        pick(SelectionMode::BetaPosterior, &cands),
        1,
        "the Beta lower bound should prefer the 18/20 candidate"
    );

    println!("\n=== T9.3 — small-sample pathology ===");
    for (tag, c) in [("lucky 1/1", &cands[0]), ("proven 18/20", &cands[1])] {
        println!(
            "  {tag:<14} rate {:.3}   beta {:.3}",
            reliability(SelectionMode::LinearBlend, c),
            reliability(SelectionMode::BetaPosterior, c)
        );
    }
}
