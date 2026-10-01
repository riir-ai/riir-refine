//! Issue 026 T3 — the MAX metric: per-span best outcome found within a fixed
//! attempt budget.
//!
//! Where `t93_selection_regret` measured MEAN selection regret
//! (BetaPosterior's home turf — the thin-evidence regime, −8.4% regret),
//! this harness measures the DISCOVERY metric the entropic objective
//! targets: given a pool of candidates whose true rates are unknown to the
//! selector, and a fixed budget of [`BUDGET`] attempts, did the loop find at
//! least one successful fix?
//!
//! `P(≥1 success within budget)` is the "best fix found for this span"
//! quantity — the max metric, NOT the average heal rate. The selector sees
//! only sampled counts + a noisy evolution score (mirroring
//! `HealPipeline::heal` Step 4's inputs); outcomes are Bernoulli(p) draws.
//! Two evo-noise levels are measured: the t93-harsh 0.6 spread and a milder
//! 0.2 spread (production directions evolve slowly from the 0.5 prior).
//!
//! ## Why a simulation (same honesty argument as t93)
//!
//! "Best outcome found" needs each candidate's TRUE success probability to
//! draw outcomes; on real fixtures `p` is unknown. `p` is known by
//! construction here; pool shape, counts, and evo noise are randomized
//! across trials. Determinism is pinned (LCG + 3 identical repeat runs).
//!
//! ```bash
//! cargo test --release --test t26_entropic_max_metric -- --nocapture
//! ```

use riir_refine::selection_entropic::{
    solve_beta, tilt_advantages_into, tilted_weights, KL_BUDGET_LN2,
};

/// Harness-local arms: the three production modes plus the two principled
/// entropic→[0,1] mappings (the paper has no blend mapping — it has no evo
/// axis — so the mapping is our design choice and BOTH candidates are
/// measured before any verdict).
#[derive(Clone, Copy)]
enum Arm {
    LinearBlend,
    BetaPosterior,
    /// sigmoid(A_β) — bounded advantage, center-neutral (house sigmoid).
    EntSigmoid,
    /// sigmoid(A_β) over RAW rates (unseen → 0.5) — the "observed outcome"
    /// reward reading, completing the reward-source × mapping space.
    EntSigmoidRate,
    /// w_β/w_max — pool-relative rank score; the argmax keeps full weight.
    EntMaxNorm,
}

const W_EVO: f32 = 0.6;
const W_RATE: f32 = 0.4;
const EPSILON: f32 = 0.05;
/// Fixed per-span attempt budget (the discovery window). Three attempts
/// against 2-5 candidates: selection quality is binding — a mode that
/// wastes attempts on weak candidates misses the gem. (B=10 saturated at
/// ~0.99 for every mode — no discriminating power; recorded in Bench 035.)
const BUDGET: u32 = 3;
/// Trials per (mode, noise) cell.
const TRIALS: usize = 20_000;

/// Deterministic LCG — identical to t93's.
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

fn belief(c: &Cand) -> f32 {
    katgpt_core::best_belief_score(c.s, c.f, EPSILON)
}

fn rate(c: &Cand) -> f32 {
    let total = c.s + c.f;
    if total == 0 {
        0.5
    } else {
        c.s as f32 / total as f32
    }
}

/// Mirror of `HealPipeline::heal` Step 4 — the selection under test.
fn pick(mode: Arm, cands: &[Cand], scratch: &mut Vec<f32>) -> usize {
    // Entropic pool paths (production: pools of size ≤ 1 degrade loudly to
    // the per-candidate belief score — mirrored here).
    if matches!(
        mode,
        Arm::EntSigmoid | Arm::EntSigmoidRate | Arm::EntMaxNorm
    ) && cands.len() > 1
    {
        scratch.clear();
        for c in cands {
            scratch.push(if matches!(mode, Arm::EntSigmoidRate) {
                rate(c)
            } else {
                belief(c)
            });
        }
        if matches!(mode, Arm::EntSigmoid | Arm::EntSigmoidRate) {
            let mut adv = Vec::new();
            tilt_advantages_into(scratch, KL_BUDGET_LN2, &mut adv);
            let mut best = (0usize, f32::NEG_INFINITY);
            for (i, c) in cands.iter().enumerate() {
                let score = W_EVO * c.evo + W_RATE * katgpt_core::sigmoid(adv[i]);
                if score > best.1 {
                    best = (i, score);
                }
            }
            return best.0;
        }
        let beta = solve_beta(scratch, KL_BUDGET_LN2);
        let mut w = Vec::new();
        tilted_weights(scratch, beta, &mut w);
        let w_max = w.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut best = (0usize, f32::NEG_INFINITY);
        for (i, c) in cands.iter().enumerate() {
            let score = W_EVO * c.evo + W_RATE * (w[i] / w_max);
            if score > best.1 {
                best = (i, score);
            }
        }
        return best.0;
    }
    let mut best = (0usize, f32::NEG_INFINITY);
    for (i, c) in cands.iter().enumerate() {
        let reliability = match mode {
            Arm::LinearBlend => rate(c),
            Arm::BetaPosterior | Arm::EntSigmoid | Arm::EntSigmoidRate | Arm::EntMaxNorm => {
                belief(c)
            }
        };
        let score = W_EVO * c.evo + W_RATE * reliability;
        if score > best.1 {
            best = (i, score);
        }
    }
    best.0
}

/// One discovery trial: build a pool, spend the budget, report whether a
/// success was found and how many attempts it took (BUDGET+1 if never).
fn run_trial(mode: Arm, rng: &mut Lcg, evo_noise: f32) -> (bool, u32) {
    let n_cands = 2 + rng.below(4); // 2..=5 candidates

    // Pool shape, randomized across trials: half rare-good (one genuinely
    // fixable candidate buried among weak ones — the discovery shape),
    // half uniform (no buried gem — diversity is the only lever).
    let rare_good = rng.f32() < 0.5;
    let mut cands: Vec<Cand> = (0..n_cands)
        .map(|idx| {
            let p = if rare_good && idx == 0 {
                0.4 + rng.f32() * 0.5 // the gem: p ∈ [0.4, 0.9)
            } else if rare_good {
                0.02 + rng.f32() * 0.18 // the crowd: p ∈ [0.02, 0.20) — weak
            } else {
                rng.f32() // uniform [0, 1)
            };
            // Thin-evidence start: 0..=3 pre-observations (0 = unseen —
            // the healer's cold-start regime).
            let n = rng.below(4) as u32;
            let s = (0..n).filter(|_| rng.f32() < p).count() as u32;
            let evo = (p + (rng.f32() - 0.5) * evo_noise).clamp(0.0, 1.0);
            Cand {
                p,
                s,
                f: n - s,
                evo,
            }
        })
        .collect();

    let mut scratch = Vec::new();
    for attempt in 1..=BUDGET {
        let choice = pick(mode, &cands, &mut scratch);
        let c = &mut cands[choice];
        if rng.f32() < c.p {
            c.s += 1;
            return (true, attempt);
        }
        c.f += 1;
    }
    (false, BUDGET + 1)
}

/// Run all trials for one (mode, noise) cell; returns
/// (discovery_rate, mean_attempts_to_success_or_budget).
fn run_cell(mode: Arm, seed: u64, evo_noise: f32) -> (f64, f64) {
    let mut rng = Lcg::new(seed);
    let (mut found, mut attempts) = (0u64, 0u64);
    for _ in 0..TRIALS {
        let (hit, att) = run_trial(mode, &mut rng, evo_noise);
        if hit {
            found += 1;
        }
        attempts += u64::from(att);
    }
    (
        found as f64 / TRIALS as f64,
        attempts as f64 / TRIALS as f64,
    )
}

#[test]
fn t26_max_metric_ab() {
    println!("\n=== Issue 026 T3 — max metric (discovery within {BUDGET} attempts) ===");
    for &evo_noise in &[0.6f32, 0.2] {
        println!("  evo_noise = {evo_noise}:");
        let cap_iter = [
            Arm::LinearBlend,
            Arm::BetaPosterior,
            Arm::EntSigmoid,
            Arm::EntSigmoidRate,
            Arm::EntMaxNorm,
        ];
        let mut cells = Vec::with_capacity(cap_iter.len());
        for mode in cap_iter {
            let cell = run_cell(mode, 0x5EED_0001, evo_noise);
            cells.push((mode, cell));
            println!(
                "    {:<13} discovery {:.4}  mean-attempts {:.3}",
                match mode {
                    Arm::LinearBlend => "LinearBlend",
                    Arm::BetaPosterior => "BetaPosterior",
                    Arm::EntSigmoid => "Ent-sigmoid",
                    Arm::EntSigmoidRate => "Ent-sig-rate",
                    Arm::EntMaxNorm => "Ent-maxnorm",
                },
                cell.0,
                cell.1
            );
        }
        // Determinism pin: re-running the Ent-sigmoid cell reproduces it exactly.
        let rerun = run_cell(Arm::EntSigmoid, 0x5EED_0001, evo_noise);
        assert_eq!(
            cells[2].1 .0.to_bits(),
            rerun.0.to_bits(),
            "Entropic cell must be deterministic across repeat runs"
        );
    }
}

/// The pathology the KL budget must prevent: a lucky 1/1 candidate cannot
/// own selection against a proven 18/20 record, even under the max-seeking
/// tilt (its belief reward ranks it low — the tilt amplifies relative
/// ranking, it does not invent confidence).
#[test]
fn t26_budget_protects_against_lucky_one_of_one() {
    let cands = vec![
        Cand {
            p: 0.0,
            s: 1,
            f: 0,
            evo: 0.5,
        },
        Cand {
            p: 1.0,
            s: 18,
            f: 2,
            evo: 0.5,
        },
    ];
    let mut scratch = Vec::new();
    assert_eq!(
        pick(Arm::EntSigmoid, &cands, &mut scratch),
        1,
        "the tilt must prefer the proven 18/20 candidate over lucky 1/1"
    );
    assert_eq!(
        pick(Arm::EntMaxNorm, &cands, &mut scratch),
        1,
        "the max-norm mapping must also prefer the proven candidate"
    );
}

/// Single-candidate pools degrade (no shape) — the pick is the only
/// candidate and must not panic.
#[test]
fn t26_single_candidate_pool_degrades() {
    let cands = vec![Cand {
        p: 0.4,
        s: 0,
        f: 2,
        evo: 0.55,
    }];
    let mut scratch = Vec::new();
    assert_eq!(pick(Arm::EntSigmoid, &cands, &mut scratch), 0);
    assert_eq!(pick(Arm::EntMaxNorm, &cands, &mut scratch), 0);
}
