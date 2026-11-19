//! Issue 248 (katgpt-rs Plan 639 T3.4) — the novelty-gated `ema_step` rate.
//! Gates pre-registered in `.issues/248_ema_step_novelty_gated_rate.md`
//! (pushed `b2e88a09` before any measurement).
//!
//! ```bash
//! cargo test --features evolve_novelty --test evolve_novelty_gates             # G1
//! cargo test --release --features evolve_novelty --test evolve_novelty_gates \
//!     -- --ignored --nocapture g2_                                           # G2 fixture
//! cargo test --release --features evolve_novelty --test evolve_novelty_gates \
//!     -- --ignored --nocapture g3_                                           # G3 cost cell
//! ```

#![cfg(feature = "evolve_novelty")]

use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

use riir_refine::embedder::{Direction, EMBED_DIM, embed};
use riir_refine::evolve::{
    EvolveConfig, LatentSkillEvolution, NoveltyChannel, NoveltyRetentionConfig, ema_step,
    ema_step_gated,
};

/// splitmix64 — the fixture's only randomness (seeded, reproducible).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[allow(clippy::cast_precision_loss)]
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn bits(d: &Direction) -> [u32; EMBED_DIM] {
    let mut out = [0u32; EMBED_DIM];
    for (o, v) in out.iter_mut().zip(d) {
        *o = v.to_bits();
    }
    out
}

// ─────────────────────────────── G1 ───────────────────────────────────

/// One seeded outcome stream over three lints and six fixes.
fn stream(seed: u64, n: usize) -> Vec<(usize, Direction, bool)> {
    let fixes: Vec<Direction> = (0..6)
        .map(|i| embed("g1_rule", &format!("fix_{i}")))
        .collect();
    let mut rng = Rng(seed);
    (0..n)
        .map(|_| {
            let lint = (rng.next() % 3) as usize;
            let fix = fixes[(rng.next() % 6) as usize];
            (lint, fix, rng.unit() < 0.6)
        })
        .collect()
}

const LINTS: [&str; 3] = ["lint_a", "lint_b", "lint_c"];

#[test]
fn g1_unarmed_layer_is_bit_identical_to_the_incumbent() {
    let cfg = EvolveConfig::default();
    let mut layer = LatentSkillEvolution::default();
    assert!(!layer.novelty_armed());
    // An invalid config refuses to arm — the instance stays incumbent.
    let bad = NoveltyRetentionConfig {
        g_max: 1.0,
        ..NoveltyRetentionConfig::default()
    };
    assert!(!layer.arm_novelty(bad));
    assert!(!layer.novelty_armed());

    let mut reference: HashMap<&str, Direction> = HashMap::new();
    for (lint, fix, success) in stream(7, 2_000) {
        let name = LINTS[lint];
        layer.record_outcome(name, &fix, success);
        match reference.get_mut(name) {
            Some(d) => {
                ema_step(d, &fix, success, &cfg);
            }
            None if success => {
                reference.insert(name, fix);
            }
            None => {}
        }
        for l in LINTS {
            assert_eq!(
                layer.get(l).map(bits),
                reference.get(l).map(bits),
                "unarmed layer diverged from the incumbent on {l}"
            );
        }
    }
}

fn run_armed(seed: u64) -> Vec<[u32; EMBED_DIM]> {
    let mut layer = LatentSkillEvolution::default();
    assert!(layer.arm_novelty(NoveltyRetentionConfig::default()));
    for (lint, fix, success) in stream(seed, 2_000) {
        layer.record_outcome(LINTS[lint], &fix, success);
    }
    LINTS
        .iter()
        .filter_map(|l| layer.get(l).map(bits))
        .collect()
}

#[test]
fn g1_armed_path_is_deterministic_and_not_vacuous() {
    let a = run_armed(11);
    assert_eq!(a, run_armed(11), "armed path must be bit-reproducible");
    let mut unarmed = LatentSkillEvolution::default();
    for (lint, fix, success) in stream(11, 2_000) {
        unarmed.record_outcome(LINTS[lint], &fix, success);
    }
    let u: Vec<_> = LINTS
        .iter()
        .filter_map(|l| unarmed.get(l).map(bits))
        .collect();
    assert_ne!(a, u, "the armed gate never changed a step — vacuous");
}

#[test]
fn g1_zero_norm_evidence_falls_back_to_the_base_rate() {
    let cfg = EvolveConfig::default();
    let ncfg = NoveltyRetentionConfig::default().validated().unwrap();
    let mut ch = NoveltyChannel::new();
    let mut dir = [0.0f32; EMBED_DIM]; // zero norm → non-finite agreement
    let mut fix = [0.0f32; EMBED_DIM];
    fix[0] = 1.0;
    let d = ema_step_gated(&mut dir, &fix, true, &cfg, &mut ch, &ncfg);
    assert!(
        (d - cfg.alpha).abs() < 1e-6,
        "fallback rate must be α, got {d}"
    );
}

// ─────────────────────────────── G2 ───────────────────────────────────

#[derive(Clone, Copy)]
enum Arm {
    Fixed(EvolveConfig),
    Gated(EvolveConfig, NoveltyRetentionConfig),
}

fn cfg_at(alpha: f32) -> EvolveConfig {
    EvolveConfig {
        alpha,
        beta: alpha / 2.0,
        temperature: 1.0,
    }
}

fn step(arm: &Arm, dir: &mut Direction, fix: &Direction, success: bool, ch: &mut NoveltyChannel) {
    match arm {
        Arm::Fixed(c) => {
            ema_step(dir, fix, success, c);
        }
        Arm::Gated(c, n) => {
            ema_step_gated(dir, fix, success, c, ch, n);
        }
    }
}

fn margin(dir: &Direction, a: &Direction, b: &Direction) -> f64 {
    let mut m = 0.0f64;
    for i in 0..EMBED_DIM {
        m += f64::from(dir[i]) * (f64::from(a[i]) - f64::from(b[i]));
    }
    m
}

const T: usize = 3_000;
const SEG: usize = 300;
const HOLD: usize = 10;

/// Mean tracking lag over the 9 flips (censored at `SEG`).
#[allow(clippy::cast_precision_loss)]
fn adaptation(arm: &Arm, seed: u64, a: &Direction, b: &Direction) -> f64 {
    let mut rng = Rng(seed);
    let mut dir = *a;
    let mut ch = NoveltyChannel::new();
    let mut ms = Vec::with_capacity(T);
    let mut good_is_a = true;
    for t in 0..T {
        if t > 0 && t % SEG == 0 {
            good_is_a = !good_is_a;
        }
        let pick_a = rng.unit() < 0.5;
        let p = if pick_a == good_is_a { 0.9 } else { 0.1 };
        let success = rng.unit() < p;
        let fix = if pick_a { a } else { b };
        step(arm, &mut dir, fix, success, &mut ch);
        ms.push(margin(&dir, a, b));
    }
    let mut total = 0.0;
    for k in 1..T / SEG {
        let want_pos = k % 2 == 0; // even segments: A good
        let seg = &ms[k * SEG..(k + 1) * SEG];
        let ok = |m: f64| if want_pos { m > 0.0 } else { m < 0.0 };
        let lag = (0..=SEG - HOLD)
            .find(|&j| seg[j..j + HOLD].iter().all(|&m| ok(m)))
            .unwrap_or(SEG);
        total += lag as f64;
    }
    total / (T / SEG - 1) as f64
}

/// RMSE of the margin about its own mean on a noisy-steady stream.
#[allow(clippy::cast_precision_loss)]
fn noise(arm: &Arm, seed: u64, a: &Direction, b: &Direction) -> f64 {
    let mut rng = Rng(seed ^ 0xA5A5_5A5A);
    let mut dir = *a;
    let mut ch = NoveltyChannel::new();
    let mut ms = Vec::with_capacity(T);
    for _ in 0..T {
        let pick_a = rng.unit() < 0.5;
        let p = if pick_a { 0.7 } else { 0.3 };
        let success = rng.unit() < p;
        let fix = if pick_a { a } else { b };
        step(arm, &mut dir, fix, success, &mut ch);
        ms.push(margin(&dir, a, b));
    }
    let w = &ms[500..];
    let mean = w.iter().sum::<f64>() / w.len() as f64;
    (w.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / w.len() as f64).sqrt()
}

const SEEDS: [u64; 3] = [1, 2, 3];
const CURVE: [f32; 11] = [0.01, 0.02, 0.03, 0.05, 0.07, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5];

#[test]
#[ignore = "G2 fixture — run with --release --ignored --nocapture"]
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn g2_refine_fixture_matched_point_sweep() {
    let a = embed("g2_rule", "fix_a");
    let b = embed("g2_rule", "fix_b");
    let cos = {
        let (mut d, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
        for i in 0..EMBED_DIM {
            d += a[i] * b[i];
            na += a[i] * a[i];
            nb += b[i] * b[i];
        }
        d / (na * nb).sqrt()
    };
    println!("fixture cos(A, B) = {cos:.4}");
    assert!(cos < 0.9, "fixture A/B too similar to separate");

    // curve[p][seed] = (adaptation, noise)
    let curve: Vec<[(f64, f64); 3]> = CURVE
        .iter()
        .map(|&al| {
            let arm = Arm::Fixed(cfg_at(al));
            SEEDS.map(|s| (adaptation(&arm, s, &a, &b), noise(&arm, s, &a, &b)))
        })
        .collect();
    let mean = |r: &[(f64, f64); 3]| {
        (
            r.iter().map(|x| x.0).sum::<f64>() / 3.0,
            r.iter().map(|x| x.1).sum::<f64>() / 3.0,
        )
    };
    println!("\nFIXED CURVE (β = α/2) — adaptation (ticks) / noise RMSE, seeds 1 2 3 | mean");
    for (al, r) in CURVE.iter().zip(&curve) {
        let (ma, mn) = mean(r);
        println!(
            "α {al:>5.2} | {:>6.1}/{:.4} {:>6.1}/{:.4} {:>6.1}/{:.4} | {ma:>6.1}/{mn:.4}",
            r[0].0, r[0].1, r[1].0, r[1].1, r[2].0, r[2].1
        );
    }
    let mut line: Vec<(f64, f64)> = curve.iter().map(mean).collect();
    line.sort_by(|x, y| x.0.total_cmp(&y.0));

    println!(
        "\nGATED GRID — base α, κ, μ, β_sat, g_min | per-seed adapt/noise | mean | (i) p | (ii) | (iii) dist | PASS"
    );
    let mut passing = 0usize;
    let mut best: Option<(f64, String)> = None;
    for base in [0.05f32, 0.1] {
        for kappa in [0.4f32, 0.8] {
            for mu in [0.2f32, 0.4] {
                for beta in [4.0f32, 8.0] {
                    for g_min in [0.5f32, 0.7] {
                        let n = NoveltyRetentionConfig {
                            kappa,
                            mu,
                            beta,
                            g_min,
                            g_max: 0.995,
                            alpha_fast: 0.3,
                            alpha_slow: 0.03,
                        }
                        .validated()
                        .expect("grid config valid");
                        let arm = Arm::Gated(cfg_at(base), n);
                        let r =
                            SEEDS.map(|s| (adaptation(&arm, s, &a, &b), noise(&arm, s, &a, &b)));
                        let (ma, mn) = mean(&r);
                        // (i) one p for all three seeds.
                        let p_i = (0..CURVE.len()).find(|&p| {
                            (0..3).all(|s| {
                                r[s].0 <= 0.7 * curve[p][s].0 && r[s].1 <= 1.1 * curve[p][s].1
                            })
                        });
                        // (ii) no fixed point dominates on seed means.
                        let dominated = curve.iter().map(mean).any(|(pa, pn)| pa <= ma && pn <= mn);
                        // (iii) distance below the piecewise-linear curve.
                        let dist = line.windows(2).find_map(|w| {
                            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
                            (x0 <= ma && ma <= x1 && x1 > x0).then(|| {
                                let y = y0 + (y1 - y0) * (ma - x0) / (x1 - x0);
                                y - mn
                            })
                        });
                        let pass = p_i.is_some() && !dominated && dist.is_some_and(|d| d > 0.0);
                        let label = format!("{base:.2} {kappa:.1} {mu:.1} {beta:.0} {g_min:.1}");
                        println!(
                            "{label} | {:>6.1}/{:.4} {:>6.1}/{:.4} {:>6.1}/{:.4} | {ma:>6.1}/{mn:.4} | {} | {} | {} | {}",
                            r[0].0,
                            r[0].1,
                            r[1].0,
                            r[1].1,
                            r[2].0,
                            r[2].1,
                            p_i.map_or("-".to_string(), |p| format!("α{:.2}", CURVE[p])),
                            if dominated { "DOM" } else { "ok" },
                            dist.map_or("n/a".to_string(), |d| format!("{d:+.4}")),
                            if pass { "PASS" } else { "fail" }
                        );
                        if pass {
                            passing += 1;
                            let d = dist.unwrap_or(0.0);
                            if best.as_ref().is_none_or(|(bd, _)| d > *bd) {
                                best = Some((d, label));
                            }
                        }
                    }
                }
            }
        }
    }
    println!(
        "\nG2 VERDICT: {} — {passing}/32 cells pass; best {:?}",
        if passing > 0 { "PASS" } else { "FAIL" },
        best
    );
}

// ─────────────────────────────── G3 ───────────────────────────────────

const PAIRS: usize = 51;
const CHUNK: usize = 20_000;

#[allow(clippy::cast_precision_loss)]
fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

#[test]
#[ignore = "G3 cost cell — run with --release --ignored --nocapture on a quiet box"]
#[allow(clippy::cast_precision_loss)]
fn g3_added_cost_per_update_interleaved() {
    let fixes: Vec<Direction> = (0..8)
        .map(|i| embed("g3_rule", &format!("fix_{i}")))
        .collect();
    let succ: Vec<bool> = {
        let mut r = Rng(42);
        (0..64).map(|_| r.unit() < 0.6).collect()
    };
    let cfg = EvolveConfig::default();
    let ncfg = NoveltyRetentionConfig::default().validated().unwrap();
    let mut dir_a = fixes[0];
    let mut dir_b = fixes[0];
    let mut ch = NoveltyChannel::new();

    let run_a = |dir: &mut Direction| {
        let t0 = Instant::now();
        for i in 0..CHUNK {
            let fix = black_box(&fixes[i & 7]);
            let s = black_box(succ[i & 63]);
            black_box(ema_step(dir, fix, s, black_box(&cfg)));
        }
        t0.elapsed().as_nanos() as f64 / CHUNK as f64
    };
    let run_b = |dir: &mut Direction, ch: &mut NoveltyChannel| {
        let t0 = Instant::now();
        for i in 0..CHUNK {
            let fix = black_box(&fixes[i & 7]);
            let s = black_box(succ[i & 63]);
            black_box(ema_step_gated(
                dir,
                fix,
                s,
                black_box(&cfg),
                ch,
                black_box(&ncfg),
            ));
        }
        t0.elapsed().as_nanos() as f64 / CHUNK as f64
    };
    // warm-up
    run_a(&mut dir_a);
    run_b(&mut dir_b, &mut ch);
    let mut a = Vec::with_capacity(PAIRS);
    let mut b = Vec::with_capacity(PAIRS);
    for p in 0..PAIRS {
        if p % 2 == 0 {
            a.push(run_a(&mut dir_a));
            b.push(run_b(&mut dir_b, &mut ch));
        } else {
            b.push(run_b(&mut dir_b, &mut ch));
            a.push(run_a(&mut dir_a));
        }
    }
    black_box((dir_a, dir_b));
    let (ma, mb) = (median(&mut a), median(&mut b));
    assert!(ma > 0.0 && mb > 0.0, "loud zero: an arm measured no work");
    println!(
        "G3 incumbent a {ma:.3} ns/call | gated b {mb:.3} ns/call | Δ {:.3} ns | ratio {:.3}×",
        mb - ma,
        mb / ma
    );
}
