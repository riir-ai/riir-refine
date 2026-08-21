//! Issue 033 T4 — TETHER-adaptive selection blend, A/B against the pinned
//! `W_EVO/W_RATE` weights.
//!
//! `select_best_candidate` scores `0.6·evo + 0.4·reliability`. That is
//! arXiv:2608.16739's baseline shape `b(ρ) = (1−ρ)·p1 + ρ·p2` with ρ
//! **hand-pinned at 0.4 and never swept**. This harness sweeps it three ways:
//!
//! 1. **Fixed-ρ sweep** — is 0.4 even the best constant? (`ρ = 0.4` must
//!    reproduce `BetaPosterior` bit-identically; that is the harness's own
//!    correctness check.)
//! 2. **Adaptive ρ** — [`TetherFit`] fit online from realized outcomes,
//!    EMA-smoothed, applied one window late (the lag law).
//! 3. **In-sample guarantee** — on the streams the simulation actually
//!    recorded, `SSE(ρ*) ≤ min(SSE(p1-only), SSE(p2-only))`, exactly.
//!
//! Metrics are the two the earlier selection gates established, so the numbers
//! are comparable to their recorded baselines:
//!
//! - **Max metric** — `P(≥1 success within BUDGET attempts)`
//!   (`t26_entropic_max_metric`; BetaPosterior 0.9240 / 0.9379).
//! - **Selection regret** — `p(best available) − p(chosen)`
//!   (`t93_selection_regret`).
//!
//! ## Why a simulation (the same honesty argument as t26/t93)
//!
//! Both metrics need each candidate's TRUE success probability, which is
//! unknown on real fixtures — it is what the selector estimates. `p` is known
//! by construction here; pool shape, counts, and evo noise are randomized
//! across trials, and determinism is pinned.
//!
//! ```bash
//! cargo test --release --test t33_tether_selection_ab -- --nocapture
//! ```

#![allow(clippy::cast_precision_loss)] // loop-index / count fixtures

use riir_clippy::selection_tether::{fit_rho, sse, TetherFit, DEFAULT_RHO};

/// Fixed per-span attempt budget — t26's, so the max metric is comparable.
const BUDGET: u32 = 3;
/// Trials per cell — t26/t93's, same reason.
const TRIALS: usize = 20_000;
const EPSILON: f32 = 0.05;
/// The pinned production weights.
const W_EVO: f32 = 0.6;
const W_RATE: f32 = 0.4;

/// The arms under test.
#[derive(Clone, Copy, PartialEq)]
enum Arm {
    /// `W_EVO·evo + W_RATE·rate` — the raw-rate baseline.
    LinearBlend,
    /// `W_EVO·evo + W_RATE·belief` — the current default.
    BetaPosterior,
    /// `(1−ρ)·evo + ρ·belief` at a FIXED ρ — the sweep the constant never got.
    FixedRho(f32),
    /// `(1−ρ)·evo + ρ·belief` with ρ fit from realized outcomes (Issue 033).
    Tether,
}

fn arm_name(arm: Arm) -> String {
    match arm {
        Arm::LinearBlend => "LinearBlend".to_string(),
        Arm::BetaPosterior => "BetaPosterior".to_string(),
        Arm::FixedRho(r) => format!("fixed rho={r:.2}"),
        Arm::Tether => "TetherBlend".to_string(),
    }
}

/// Deterministic LCG — identical to t26/t93's.
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

/// One candidate: a true quality `p`, an observed record, an evolution score.
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

/// Mirror of `HealPipeline::heal` Step 4 — returns the pick plus the winner's
/// `(p1, p2)` endpoint pair, which is what the TETHER fit observes.
fn pick(arm: Arm, cands: &[Cand], rho: f32) -> (usize, f32, f32) {
    let mut best = (0usize, f32::NEG_INFINITY, 0.0f32, 0.0f32);
    for (i, c) in cands.iter().enumerate() {
        let (p1, p2) = (c.evo, if arm == Arm::LinearBlend { rate(c) } else { belief(c) });
        let score = match arm {
            Arm::LinearBlend | Arm::BetaPosterior => W_EVO * p1 + W_RATE * p2,
            Arm::FixedRho(r) => (1.0 - r) * p1 + r * p2,
            Arm::Tether => (1.0 - rho) * p1 + rho * p2,
        };
        if score > best.1 {
            best = (i, score, p1, p2);
        }
    }
    (best.0, best.2, best.3)
}

/// One discovery trial. Feeds realized outcomes into `fit` (Tether arm only)
/// and appends the `(R, p1, p2)` samples to `stream`.
fn run_trial(
    arm: Arm,
    rng: &mut Lcg,
    evo_noise: f32,
    fit: &mut TetherFit,
    stream: &mut Vec<(f32, f32, f32)>,
) -> bool {
    let n_cands = 2 + rng.below(4); // 2..=5 candidates

    // t26's pool shapes: half rare-good (a buried gem), half uniform.
    let rare_good = rng.f32() < 0.5;
    let mut cands: Vec<Cand> = (0..n_cands)
        .map(|idx| {
            let p = if rare_good && idx == 0 {
                0.4 + rng.f32() * 0.5
            } else if rare_good {
                0.02 + rng.f32() * 0.18
            } else {
                rng.f32()
            };
            let n = rng.below(4) as u32; // thin evidence: 0..=3 pre-observations
            let s = (0..n).filter(|_| rng.f32() < p).count() as u32;
            let evo = (p + (rng.f32() - 0.5) * evo_noise).clamp(0.0, 1.0);
            Cand { p, s, f: n - s, evo }
        })
        .collect();

    for _ in 1..=BUDGET {
        let (choice, p1, p2) = pick(arm, &cands, fit.rho());
        let c = &mut cands[choice];
        let hit = rng.f32() < c.p;
        // The realized outcome, fed back exactly as `report_outcome` does:
        // endpoints as they were at SELECTION time, never post-update.
        stream.push((f32::from(u8::from(hit)), p1, p2));
        if arm == Arm::Tether {
            fit.observe(f32::from(u8::from(hit)), p1, p2);
        }
        if hit {
            c.s += 1;
            return true;
        }
        c.f += 1;
    }
    false
}

/// One (arm, noise) cell → (discovery rate, final ρ, recorded stream).
fn run_cell(arm: Arm, seed: u64, evo_noise: f32) -> (f64, f32, Vec<(f32, f32, f32)>) {
    let mut rng = Lcg::new(seed);
    let mut fit = TetherFit::new();
    let mut stream = Vec::new();
    let mut found = 0u64;
    for _ in 0..TRIALS {
        if run_trial(arm, &mut rng, evo_noise, &mut fit, &mut stream) {
            found += 1;
        }
    }
    (found as f64 / TRIALS as f64, fit.rho(), stream)
}

/// The max metric: discovery within budget, across the fixed-ρ sweep and the
/// adaptive arm.
#[test]
fn t33_max_metric_ab() {
    println!("\n=== Issue 033 T4 — max metric (discovery within {BUDGET} attempts) ===");
    for &evo_noise in &[0.6f32, 0.2] {
        println!("  evo_noise = {evo_noise}:");
        let mut beta_cell = 0.0;
        let mut tether_cell = 0.0;
        let mut best_fixed = (f32::NAN, 0.0f64);
        for arm in [
            Arm::LinearBlend,
            Arm::BetaPosterior,
            Arm::FixedRho(0.0),
            Arm::FixedRho(0.2),
            Arm::FixedRho(0.4),
            Arm::FixedRho(0.6),
            Arm::FixedRho(0.8),
            Arm::FixedRho(1.0),
            Arm::Tether,
        ] {
            let (discovery, rho, _) = run_cell(arm, 0x5EED_0001, evo_noise);
            match arm {
                Arm::BetaPosterior => beta_cell = discovery,
                Arm::Tether => tether_cell = discovery,
                Arm::FixedRho(r) if discovery > best_fixed.1 => best_fixed = (r, discovery),
                _ => {}
            }
            let rho_note = match arm {
                Arm::Tether => format!("  final rho {rho:.4}"),
                _ => String::new(),
            };
            println!(
                "    {:<15} discovery {discovery:.4}{rho_note}",
                arm_name(arm)
            );
        }
        println!(
            "    -> best FIXED rho {:.2} at {:.4}; adaptive {:.4}; default (0.4) {:.4}",
            best_fixed.0, best_fixed.1, tether_cell, beta_cell
        );
        println!(
            "    -> adaptive vs default: {:+.4} ({:+.2}%)",
            tether_cell - beta_cell,
            (tether_cell - beta_cell) / beta_cell * 100.0
        );
    }
}

/// Selection regret (t93's metric) — the mean-quality view, in case the
/// adaptive mix trades discovery for average pick quality.
#[test]
fn t33_selection_regret_ab() {
    println!("\n=== Issue 033 T4 — selection regret ===");
    for &max_attempts in &[1u32, 3, 5] {
        print!("  max_attempts={max_attempts:<3}");
        for arm in [Arm::BetaPosterior, Arm::FixedRho(0.0), Arm::FixedRho(1.0), Arm::Tether] {
            let mut rng = Lcg::new(0xC0FFEE ^ u64::from(max_attempts));
            let mut fit = TetherFit::new();
            let mut regret = 0.0f64;
            for _ in 0..TRIALS {
                let n_cands = 2 + rng.below(4);
                let cands: Vec<Cand> = (0..n_cands)
                    .map(|_| {
                        let p = rng.f32();
                        let n = 1 + rng.below(max_attempts as usize) as u32;
                        let s = (0..n).filter(|_| rng.f32() < p).count() as u32;
                        let evo = (p + (rng.f32() - 0.5) * 0.6).clamp(0.0, 1.0);
                        Cand { p, s, f: n - s, evo }
                    })
                    .collect();
                let best_p = cands.iter().map(|c| c.p).fold(f32::MIN, f32::max);
                let (choice, p1, p2) = pick(arm, &cands, fit.rho());
                // One realized draw per trial — the outcome stream production
                // would see.
                let hit = rng.f32() < cands[choice].p;
                if arm == Arm::Tether {
                    fit.observe(f32::from(u8::from(hit)), p1, p2);
                }
                regret += f64::from(best_p - cands[choice].p);
            }
            print!("  {}: {:.4}", arm_name(arm), regret / TRIALS as f64);
            if arm == Arm::Tether {
                print!(" (rho {:.4})", fit.rho());
            }
        }
        println!();
    }
}

/// The paper's in-sample guarantee, on the streams THIS simulation recorded
/// (not a synthetic fixture): the fitted ρ never loses to either endpoint.
#[test]
fn t33_in_sample_never_worse_on_recorded_streams() {
    println!("\n=== Issue 033 T4 — in-sample never-worse (recorded streams) ===");
    for &evo_noise in &[0.6f32, 0.2] {
        for arm in [Arm::BetaPosterior, Arm::Tether] {
            let (_, _, stream) = run_cell(arm, 0x5EED_0001, evo_noise);
            let rho = fit_rho(&stream, DEFAULT_RHO);
            let (fit_sse, p1_sse, p2_sse) = (sse(rho, &stream), sse(0.0, &stream), sse(1.0, &stream));
            let pinned = sse(W_RATE, &stream);
            println!(
                "  noise {evo_noise} arm {:<14} n={} rho*={rho:.4}  SSE fit {fit_sse:.1} \
                 | evo-only {p1_sse:.1} | rel-only {p2_sse:.1} | pinned-0.4 {pinned:.1}",
                arm_name(arm),
                stream.len()
            );
            assert!(
                fit_sse <= p1_sse.min(p2_sse) + 1e-6,
                "SSE(rho*) must not exceed either endpoint"
            );
            assert!(
                fit_sse <= pinned + 1e-6,
                "SSE(rho*) must not exceed the pinned-0.4 blend either \
                 (0.4 is a feasible point of the same convex problem)"
            );
        }
    }
}

/// T3's drift signature: ρ must MOVE with the informativeness of the axes. A
/// fixed 0.4 cannot express this; if ρ never moved, the mechanism would be
/// inert and the A/B meaningless.
#[test]
fn t33_rho_drifts_with_axis_informativeness() {
    println!("\n=== Issue 033 T3 — rho drift signature ===");

    // Regime A: the evolution axis is an oracle (zero noise) — the outcome is
    // explained by p1, so the fit should pull ρ DOWN, off the reliability axis.
    // Regime B: the evolution axis is pure noise — reliability is the only
    // signal left, so ρ should be pulled UP relative to A.
    let mut rhos = Vec::new();
    for &evo_noise in &[0.0f32, 2.0] {
        let mut rng = Lcg::new(0xD31F7);
        let mut fit = TetherFit::new();
        let mut stream = Vec::new();
        for _ in 0..4_000 {
            run_trial(Arm::Tether, &mut rng, evo_noise, &mut fit, &mut stream);
        }
        let st = fit.stats();
        println!(
            "  evo_noise {evo_noise:<4} -> rho {:.4}  (last rho_hat {:?}, windows {}, degenerate {})",
            st.rho, st.last_rho_hat, st.windows_closed, st.degenerate_windows
        );
        assert!(st.windows_closed > 0, "the fit must close windows");
        rhos.push(st.rho);
    }
    assert!(
        rhos[1] > rhos[0],
        "rho must rise when the evolution axis stops being informative \
         (oracle-evo {:.4} vs noise-evo {:.4})",
        rhos[0],
        rhos[1]
    );
    assert!(
        (rhos[0] - DEFAULT_RHO).abs() > 1e-4 || (rhos[1] - DEFAULT_RHO).abs() > 1e-4,
        "a rho pinned at the default in BOTH regimes means the mode is inert"
    );
}

/// The harness's own correctness check: `FixedRho(0.4)` IS the production
/// blend, so it must reproduce `BetaPosterior` bit-identically. If this fails,
/// every comparison above is measuring an unintended difference.
#[test]
fn t33_fixed_rho_04_is_the_production_blend() {
    for &evo_noise in &[0.6f32, 0.2] {
        let beta = run_cell(Arm::BetaPosterior, 0x5EED_0001, evo_noise);
        let fixed = run_cell(Arm::FixedRho(W_RATE), 0x5EED_0001, evo_noise);
        assert_eq!(
            beta.0.to_bits(),
            fixed.0.to_bits(),
            "rho=0.4 must reproduce the pinned W_EVO/W_RATE blend exactly"
        );
    }
}

/// T6 determinism: identical outcome streams give a bit-identical cell AND a
/// bit-identical ρ.
#[test]
fn t33_cells_are_deterministic() {
    for &evo_noise in &[0.6f32, 0.2] {
        let a = run_cell(Arm::Tether, 0x5EED_0001, evo_noise);
        let b = run_cell(Arm::Tether, 0x5EED_0001, evo_noise);
        assert_eq!(a.0.to_bits(), b.0.to_bits(), "discovery must be reproducible");
        assert_eq!(a.1.to_bits(), b.1.to_bits(), "final rho must be reproducible");
    }
}

/// Cold start is a no-regression: before the first window closes, the adaptive
/// arm's picks are the production blend's picks, candidate for candidate.
#[test]
fn t33_cold_start_matches_production_picks() {
    let mut rng = Lcg::new(0x0C0D_5747);
    let fit = TetherFit::new();
    for _ in 0..500 {
        let n_cands = 2 + rng.below(4);
        let cands: Vec<Cand> = (0..n_cands)
            .map(|_| {
                let p = rng.f32();
                let n = rng.below(4) as u32;
                let s = (0..n).filter(|_| rng.f32() < p).count() as u32;
                let evo = (p + (rng.f32() - 0.5) * 0.6).clamp(0.0, 1.0);
                Cand { p, s, f: n - s, evo }
            })
            .collect();
        assert_eq!(
            pick(Arm::Tether, &cands, fit.rho()).0,
            pick(Arm::BetaPosterior, &cands, 0.0).0,
            "a cold TetherFit must pick exactly what the pinned blend picks"
        );
    }
}

/// The verdict cell: a **paired** discovery comparison under common random
/// numbers, because the unpaired deltas above (±0.003 at 20 000 trials, SE
/// ≈ 0.0018) cannot separate a real difference from noise.
///
/// Pairing: each trial pre-draws the pool AND a per-candidate outcome table,
/// so both arms face bit-identical latent randomness — candidate `c`'s `k`-th
/// attempt succeeds or fails the same way whichever arm attempted it. What
/// remains is purely the selection difference. Discordance is then a McNemar
/// count: `b` = adaptive found it and the default did not, `c` = the reverse,
/// `z = (b − c)/sqrt(b + c)`.
#[test]
fn t33_paired_discovery_mcnemar() {
    println!("\n=== Issue 033 T5 — paired discovery (common random numbers) ===");
    const PAIRED_TRIALS: usize = 200_000;

    for &evo_noise in &[0.6f32, 0.2] {
        let mut rng = Lcg::new(0x9A17_ED00);
        let mut fit = TetherFit::new();
        // b = adaptive only, c = default only.
        let (mut b, mut c, mut both, mut neither) = (0u64, 0u64, 0u64, 0u64);

        for _ in 0..PAIRED_TRIALS {
            // ── pool + outcome table, drawn ONCE and shared by both arms ──
            let n_cands = 2 + rng.below(4);
            let rare_good = rng.f32() < 0.5;
            let mut pool: Vec<Cand> = Vec::with_capacity(n_cands);
            let mut outcomes: Vec<[bool; BUDGET as usize]> = Vec::with_capacity(n_cands);
            for idx in 0..n_cands {
                let p = if rare_good && idx == 0 {
                    0.4 + rng.f32() * 0.5
                } else if rare_good {
                    0.02 + rng.f32() * 0.18
                } else {
                    rng.f32()
                };
                let n = rng.below(4) as u32;
                let s = (0..n).filter(|_| rng.f32() < p).count() as u32;
                let evo = (p + (rng.f32() - 0.5) * evo_noise).clamp(0.0, 1.0);
                let mut draws = [false; BUDGET as usize];
                for d in &mut draws {
                    *d = rng.f32() < p;
                }
                pool.push(Cand { p, s, f: n - s, evo });
                outcomes.push(draws);
            }

            let replay = |arm: Arm, rho: f32, fit: Option<&mut TetherFit>| -> bool {
                let mut cands: Vec<Cand> = pool
                    .iter()
                    .map(|c| Cand { p: c.p, s: c.s, f: c.f, evo: c.evo })
                    .collect();
                let mut used = vec![0usize; cands.len()];
                let mut fit = fit;
                for _ in 0..BUDGET {
                    let live_rho = fit.as_ref().map_or(rho, |f| f.rho());
                    let (choice, p1, p2) = pick(arm, &cands, live_rho);
                    let k = used[choice].min(BUDGET as usize - 1);
                    used[choice] += 1;
                    let hit = outcomes[choice][k];
                    if let Some(f) = fit.as_mut() {
                        f.observe(f32::from(u8::from(hit)), p1, p2);
                    }
                    if hit {
                        return true;
                    }
                    cands[choice].f += 1;
                }
                false
            };

            // The default arm never mutates the fit; the adaptive arm owns it
            // (so its ρ trajectory is driven by its OWN realized stream).
            let default_hit = replay(Arm::BetaPosterior, W_RATE, None);
            let adaptive_hit = replay(Arm::Tether, 0.0, Some(&mut fit));

            match (adaptive_hit, default_hit) {
                (true, true) => both += 1,
                (true, false) => b += 1,
                (false, true) => c += 1,
                (false, false) => neither += 1,
            }
        }

        let n = PAIRED_TRIALS as f64;
        let disc = (b + c) as f64;
        let z = if disc > 0.0 {
            (b as f64 - c as f64) / disc.sqrt()
        } else {
            0.0
        };
        println!(
            "  evo_noise {evo_noise}: adaptive {:.4} vs default {:.4}  \
             (both {both}, adaptive-only {b}, default-only {c}, neither {neither})",
            (both + b) as f64 / n,
            (both + c) as f64 / n
        );
        println!(
            "    discordant {disc:.0}  z = {z:+.2}  -> {}",
            if z.abs() < 1.96 {
                "NO significant difference (|z| < 1.96)"
            } else if z > 0.0 {
                "adaptive SIGNIFICANTLY better"
            } else {
                "adaptive SIGNIFICANTLY worse"
            }
        );
        println!("    final rho {:.4}  ({:?})", fit.rho(), fit.stats());
    }
}
