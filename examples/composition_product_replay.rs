//! Issue 251 T2 — paired adjudication of the multiplicative (soft-AND)
//! composition challenger against the additive incumbent, over the durable
//! trajectory store on THIS box.
//!
//! ## What is replayed
//!
//! A selection pool is one rule's (`lint_key`'s) set of stored trajectories —
//! the candidates `select_best_candidate` chooses among for that rule. Groups
//! with at least two rows carrying `N = S+F ≥ 2` outcomes are kept (the
//! issue's "n≥2 groups").
//!
//! Both laws score the SAME two legs production computes:
//! - **evolution** — `sigmoid(dot(learn_direction, row_direction))` (the
//!   durable EMA twin, `LatentSkillEvolution::score_candidate` at
//!   temperature 1), or the neutral `0.5` when the rule has no `learn:` row;
//! - **reliability** — `best_belief_score(S_e, F_e, 0.05) · journey_factor`.
//!
//! Additive: `selection_product::additive_score` (the incumbent expression,
//! bit-identical). Product: `selection_product::product_score` (`ln + ln`);
//! a pool holding a refused leg falls back to additive — counted, never
//! hidden.
//!
//! ## Cross-fit (why the outcome is not the input)
//!
//! The store keeps aggregated counts, so a naive replay would score each row
//! on the very outcomes it is judged by. Each row's `N` outcomes are split
//! without replacement (seeded hypergeometric draw): `⌊N/2⌋` are EVIDENCE
//! (feed the reliability leg), the rest are HELD OUT (the pick's quality is
//! its held-out success rate). The split is repeated `--splits` times and
//! averaged per group; the paired delta's SE is taken ACROSS GROUPS.
//!
//! Disclosed leakage: the `learn:` direction is the final EMA over ALL
//! outcomes, so the evolution leg has seen the held-out half. Both arms
//! share that leg, so the paired delta is fair between the laws, but the
//! absolute quality is optimistic.
//!
//! ## Bars (issue 251)
//!
//! - Non-inferiority δ = max(1.0pp, 2.5·SE); the challenger PROMOTE-ELIGIBLE
//!   only when it wins (mean − 1.96·SE > 0). A tie or loss DEMOTES.
//! - Never-slower: both laws timed interleaved over the same pools.
//!
//! Report-only, read-only (`TrajStore::open`, nothing written); exit 0 on any
//! population, including an UNMEASURABLE one.
//!
//! ```bash
//! cargo run --release --features latent_retrieval,composition_product \
//!     --example composition_product_replay -- [--splits 200] [--root PATH]
//! ```

use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::time::Instant;

use riir_refine::selection_product::{additive_score, product_score};
use riir_refine::traj_store::TrajStore;

const BELIEF_EPSILON: f32 = 0.05;

/// One candidate row, reduced to what the replay needs.
struct Row {
    evo: f32,
    s: u32,
    f: u32,
    journey: f32,
}

/// SplitMix64 — seeded, deterministic, dependency-free.
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Successes among `m` draws without replacement from `s` successes in `n`.
fn hypergeometric(rng: &mut Rng, s: u32, n: u32, m: u32) -> u32 {
    let (mut s_rem, mut n_rem, mut hit) = (s, n, 0u32);
    for _ in 0..m {
        if rng.unit() < f64::from(s_rem) / f64::from(n_rem) {
            hit += 1;
            s_rem -= 1;
        }
        n_rem -= 1;
    }
    hit
}

/// First-index argmax (strict `>`), the production tie rule absent a GP.
fn argmax(scores: &[f32]) -> usize {
    let mut best = 0;
    for (i, &v) in scores.iter().enumerate().skip(1) {
        if v > scores[best] {
            best = i;
        }
    }
    best
}

fn main() {
    let mut splits: u32 = 200;
    let mut root = std::env::current_dir().unwrap_or_default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--splits" => splits = args.next().and_then(|v| v.parse().ok()).unwrap_or(200),
            "--root" => {
                if let Some(v) = args.next() {
                    root = v.into();
                }
            }
            other => {
                eprintln!("unknown arg {other} — usage: [--splits N] [--root PATH]");
                std::process::exit(2);
            }
        }
    }

    let (store, bad) = match TrajStore::open(&root) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("store open failed: {e}");
            std::process::exit(1);
        }
    };
    let dim = riir_refine::EMBED_DIM;
    let learn: HashMap<String, Vec<f32>> = store
        .learn_rows()
        .into_iter()
        .filter(|r| r.direction.len() == dim)
        .map(|r| (r.lint_key, r.direction))
        .collect();

    let mut groups: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    let (mut thin, mut neutral) = (0u64, 0u64);
    let total = store.len();
    for row in store.dump() {
        let n = row.success_count.saturating_add(row.fail_count);
        if n < 2 || row.direction.len() != dim {
            thin += 1;
            continue;
        }
        let evo = match learn.get(&row.lint_key) {
            Some(ev) => {
                katgpt_core::sigmoid(katgpt_core::simd::simd_dot_f32(ev, &row.direction, dim))
            }
            None => {
                neutral += 1;
                0.5
            }
        };
        groups.entry(row.lint_key.clone()).or_default().push(Row {
            evo,
            s: row.success_count,
            f: row.fail_count,
            journey: riir_refine::journey::journey_factor(row.journey_runs, row.journey_taint_sum),
        });
    }
    groups.retain(|_, v| v.len() >= 2);

    println!(
        "store: {total} traj row(s) (corrupt {bad}), {} learn row(s); rows with N<2 skipped {thin}, \
         neutral-evo rows {neutral}",
        learn.len()
    );
    let g = groups.len();
    let pool_rows: usize = groups.values().map(Vec::len).sum();
    println!(
        "groups (rules with >= 2 rows at N >= 2): {g}  (pool rows {pool_rows}), splits {splits}"
    );
    if g < 2 {
        println!(
            "\nVERDICT: UNMEASURABLE — {g} group(s); the store does not carry enough multi-candidate \
             pools for a paired read. The additive incumbent stands; the challenger stays DEMOTED."
        );
        return;
    }

    // Per-group, split-averaged paired delta + agreement.
    let mut deltas: Vec<f64> = Vec::with_capacity(g);
    let mut agree_sum = 0.0f64;
    let (mut q_add_sum, mut q_prod_sum) = (0.0f64, 0.0f64);
    let mut refused_pools = 0u64;
    let mut disagree_groups = 0usize;
    // Pools retained for the timing cell: (evo, rel) per candidate.
    let mut timing_pools: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut add_scores = Vec::new();
    let mut prod_scores = Vec::new();
    for (gi, rows) in groups.values().enumerate() {
        let mut rng = Rng(0x2510_0000 ^ gi as u64);
        let (mut d_sum, mut agree, mut qa, mut qp) = (0.0f64, 0u32, 0.0f64, 0.0f64);
        for _ in 0..splits {
            let mut legs = Vec::with_capacity(rows.len());
            let mut held = Vec::with_capacity(rows.len());
            for r in rows {
                let n = r.s + r.f;
                let m = n / 2;
                let se = hypergeometric(&mut rng, r.s, n, m);
                let fe = m - se;
                let rel = katgpt_core::best_belief_score(se, fe, BELIEF_EPSILON) * r.journey;
                legs.push((r.evo, rel));
                held.push(f64::from(r.s - se) / f64::from(n - m));
            }
            add_scores.clear();
            add_scores.extend(legs.iter().map(|&(e, r)| additive_score(e, r)));
            prod_scores.clear();
            let mut refused = false;
            for &(e, r) in &legs {
                match product_score(e, r) {
                    Some(v) => prod_scores.push(v),
                    None => refused = true,
                }
            }
            let a = argmax(&add_scores);
            let p = if refused {
                refused_pools += 1;
                a
            } else {
                argmax(&prod_scores)
            };
            agree += u32::from(a == p);
            qa += held[a];
            qp += held[p];
            d_sum += held[p] - held[a];
            if timing_pools.len() < 4096 {
                timing_pools.push(legs);
            }
        }
        let k = f64::from(splits);
        deltas.push(d_sum / k);
        agree_sum += f64::from(agree) / k;
        q_add_sum += qa / k;
        q_prod_sum += qp / k;
        if agree < splits {
            disagree_groups += 1;
        }
    }

    let gf = g as f64;
    let mean = deltas.iter().sum::<f64>() / gf;
    let var = deltas.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (gf - 1.0);
    let se = (var / gf).sqrt();
    let delta_bar = (0.01f64).max(2.5 * se);
    println!("\nPAIRED (product − additive), held-out success rate of the pick:");
    println!(
        "  additive {:.4}  product {:.4}  delta {:+.4} (SE {:.4}, groups {g})",
        q_add_sum / gf,
        q_prod_sum / gf,
        mean,
        se
    );
    println!(
        "  pick agreement {:.4}  groups with any disagreement {disagree_groups}/{g}  \
         refused-leg pools {refused_pools} (scored additive)",
        agree_sum / gf
    );
    println!("  non-inferiority delta = max(1.0pp, 2.5*SE) = {delta_bar:.4}");

    // Never-slower: interleaved timing over the same pools, 5 rounds, min.
    let mut best_add = f64::INFINITY;
    let mut best_prod = f64::INFINITY;
    let picks: usize = timing_pools.len();
    for _ in 0..5 {
        let t = Instant::now();
        let mut acc = 0usize;
        for pool in &timing_pools {
            let mut b = 0usize;
            let mut bv = f32::NEG_INFINITY;
            for (i, &(e, r)) in pool.iter().enumerate() {
                let v = additive_score(black_box(e), black_box(r));
                if v > bv {
                    bv = v;
                    b = i;
                }
            }
            acc ^= b;
        }
        black_box(acc);
        best_add = best_add.min(t.elapsed().as_secs_f64());
        let t = Instant::now();
        let mut acc = 0usize;
        for pool in &timing_pools {
            let mut b = 0usize;
            let mut bv = f32::NEG_INFINITY;
            for (i, &(e, r)) in pool.iter().enumerate() {
                let v = product_score(black_box(e), black_box(r)).unwrap_or(f32::NEG_INFINITY);
                if v > bv {
                    bv = v;
                    b = i;
                }
            }
            acc ^= b;
        }
        black_box(acc);
        best_prod = best_prod.min(t.elapsed().as_secs_f64());
    }
    let pf = picks.max(1) as f64;
    println!(
        "\nTIMING (min of 5, {picks} pools): additive {:.1} ns/pick  product {:.1} ns/pick  ({:.2}x); \
         alloc: both laws are stack-only arithmetic (alloc-neutral by construction)",
        best_add / pf * 1e9,
        best_prod / pf * 1e9,
        best_prod / best_add.max(f64::MIN_POSITIVE)
    );

    let verdict = if mean - 1.96 * se > 0.0 {
        "WIN — product beats additive on the paired bar (PROMOTE-ELIGIBLE; T3 sweep owed)"
    } else if mean > -delta_bar {
        "TIE — no separation at two legs; challenger DEMOTES (additive incumbent stands)"
    } else {
        "LOSS — product is inferior; challenger DEMOTES (additive incumbent stands)"
    };
    println!("\nVERDICT: {verdict}");
}
