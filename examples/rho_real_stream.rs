//! Issue 229 (plan 368 T5, verdict round 1 REVISE fold) — the real-stream
//! prior-quality measurement: the correlation ρ̂ between the production
//! prior axis (pool-normalized evolution score) and realized reliability,
//! measured over the durable outcome store on THIS box.
//!
//! What the cell needed: the replay's ρ sweep passed only at synthesized
//! ρ ∈ {0.7, 1.0} and regressed at ρ ≤ 0.3 — so whether `SelectionMode::Puct`
//! can ever pay on a real stream is decided by the ρ the production prior
//! actually carries. The reviewer's fold: measure it NOW from the store
//! rather than defer to a future consumer.
//!
//! ## The two axes, exactly as production computes them
//!
//! - **Reliability** (the Q axis): `best_belief_score(S, F, ε)` per `traj:`
//!   row, restricted to rows with `N = S+F ≥ --min-n` (default 3 — below
//!   that the Beta quantile is dominated by the prior and the correlation
//!   is definitionally inflated).
//! - **Prior** (the P axis): `LatentSkillEvolution::score_candidate` — the
//!   sigmoid of the dot between the rule's DURABLE EMA direction (the
//!   `learn:` row, the persisted twin the store itself evolves under the
//!   same α/β rule) and the row's own `direction`. A rule with no `learn:`
//!   row scores 0.5 (the neutral prior — exactly what a fresh process sees).
//!
//! ## The disclosed selection bias (why ρ̂ is an UPPER bound)
//!
//! Only candidates the selector PICKED get outcomes (the store is the
//! picked-candidates' ledger), so the reliability axis is observed on a
//! selected sample: the correlation is inflated by the selection itself.
//! ρ̂ is therefore read as an upper bound on the deployable prior quality,
//! and the pre-registered decision rule (plan 368 §T5) uses the bound
//! accordingly — ties go to the incumbent.
//!
//! Both Pearson and Spearman are reported (the replay normalized P into a
//! pool-relative position; Spearman on the raw scores is the
//! scale-free read, Pearson the calibrated one). Per-rule breakdown
//! beside the pooled number — a pooled ρ̂ driven by one dense rule is a
//! different claim than a uniform one.
//!
//! Report-only, exit 0 on any population (including zero — the honest empty
//! store is a valid reading, not an error). Read-only: the store is opened
//! through the same `TrajStore::open` the heal lane uses; nothing is
//! written.
//!
//! ```bash
//! cargo run --release --example rho_real_stream -- [--min-n 3] [--root PATH]
//! ```

use riir_refine::traj_store::TrajStore;

fn main() {
    let mut min_n: u32 = 3;
    let mut root = std::env::current_dir().unwrap_or_default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--min-n" => {
                min_n = args.next().and_then(|v| v.parse().ok()).unwrap_or(3);
            }
            "--root" => {
                if let Some(v) = args.next() {
                    root = v.into();
                }
            }
            other => {
                eprintln!("unknown arg {other} — usage: [--min-n N] [--root PATH]");
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
    if bad > 0 {
        println!("store rows skipped at open (corrupt): {bad}");
    }

    // The durable EMA twin — the persisted evolution state.
    let learn: std::collections::HashMap<String, Vec<f32>> = store
        .learn_rows()
        .into_iter()
        .filter(|r| r.direction.len() == riir_refine::EMBED_DIM)
        .map(|r| (r.lint_key, r.direction))
        .collect();
    println!(
        "store: {} traj row(s), {} learn row(s) with full directions, min-n {min_n}",
        store.len(),
        learn.len()
    );

    // One (prior, reliability) pair per row with enough evidence.
    let mut pairs: Vec<(f32, f32, String)> = Vec::new();
    let mut skipped_thin = 0u64;
    let mut neutral_prior = 0u64;
    for row in store.dump() {
        let n = row.success_count.saturating_add(row.fail_count);
        if n < min_n {
            skipped_thin += 1;
            continue;
        }
        let reliability = katgpt_core::best_belief_score(row.success_count, row.fail_count, 0.05);
        let mut dir = [0.0f32; riir_refine::EMBED_DIM];
        if row.direction.len() != riir_refine::EMBED_DIM {
            continue;
        }
        dir.copy_from_slice(&row.direction);
        let prior = match learn.get(&row.lint_key) {
            Some(evolved) => {
                let mut evolved_arr = [0.0f32; riir_refine::EMBED_DIM];
                evolved_arr.copy_from_slice(evolved);
                let dot =
                    katgpt_core::simd::simd_dot_f32(&evolved_arr, &dir, riir_refine::EMBED_DIM);
                katgpt_core::sigmoid(dot)
            }
            None => {
                neutral_prior += 1;
                0.5
            }
        };
        pairs.push((prior, reliability, row.lint_key));
    }

    println!(
        "rows: {} measured (skipped thin {skipped_thin}, neutral-prior {neutral_prior})",
        pairs.len()
    );
    if pairs.len() < 3 {
        println!(
            "\nRHO-HAT: UNMEASURABLE — population {}/3. The store does not yet carry \
             enough picked-candidate outcomes for a correlation read; the decision \
             rule's UNMEASURABLE arm (ties go to the incumbent) applies.",
            pairs.len()
        );
        return;
    }

    // Pearson.
    let n = pairs.len() as f64;
    let (mut sp, mut sr) = (0.0f64, 0.0f64);
    let (mut mp, mut mr) = (0.0f64, 0.0f64);
    for (p, r, _) in &pairs {
        mp += f64::from(*p);
        mr += f64::from(*r);
    }
    mp /= n;
    mr /= n;
    let (mut cov, mut vp, mut vr) = (0.0f64, 0.0f64, 0.0f64);
    for (p, r, _) in &pairs {
        let dp = f64::from(*p) - mp;
        let dr = f64::from(*r) - mr;
        cov += dp * dr;
        vp += dp * dp;
        vr += dr * dr;
    }
    let pearson = if vp > 0.0 && vr > 0.0 {
        cov / (vp.sqrt() * vr.sqrt())
    } else {
        f64::NAN
    };

    // Spearman (rank correlation — the pool-relative-position read).
    let rank = |vals: &[f32]| -> Vec<f64> {
        let mut idx: Vec<usize> = (0..vals.len()).collect();
        idx.sort_by(|a, b| vals[*a].total_cmp(&vals[*b]));
        let mut ranks = vec![0.0f64; vals.len()];
        let mut i = 0;
        while i < idx.len() {
            let mut j = i;
            while j + 1 < idx.len() && vals[idx[j + 1]] == vals[idx[i]] {
                j += 1;
            }
            let avg = (i + j) as f64 / 2.0 + 1.0;
            for k in i..=j {
                ranks[idx[k]] = avg;
            }
            i = j + 1;
        }
        ranks
    };
    let priors: Vec<f32> = pairs.iter().map(|p| p.0).collect();
    let rels: Vec<f32> = pairs.iter().map(|p| p.1).collect();
    let (rp, rr) = (rank(&priors), rank(&rels));
    let (mrp, mrr) = (rp.iter().sum::<f64>() / n, rr.iter().sum::<f64>() / n);
    let (mut scov, mut svp, mut svr) = (0.0, 0.0, 0.0);
    for i in 0..pairs.len() {
        let dp = rp[i] - mrp;
        let dr = rr[i] - mrr;
        scov += dp * dr;
        svp += dp * dp;
        svr += dr * dr;
    }
    let spearman = if svp > 0.0 && svr > 0.0 {
        scov / (svp.sqrt() * svr.sqrt())
    } else {
        f64::NAN
    };
    sp += spearman; // silence the unused-var lint spine
    sr += pearson;

    // 95% CI on Spearman via Fisher z (n ≥ 3; small-n honesty disclosed).
    let z = 0.5_f64 * ((1.0 + spearman) / (1.0 - spearman)).ln();
    let se = 1.03_f64 / (n - 3.0).max(1.0).sqrt();
    let lo = ((z - 1.96 * se).tan() * 2.0 + 1.0).clamp(-1.0, 1.0);
    let hi = ((z + 1.96 * se).tan() * 2.0 + 1.0).clamp(-1.0, 1.0);

    println!("\nRHO-HAT (prior vs realized reliability, N≥{min_n}):");
    println!("  pearson  {pearson:+.4}");
    println!(
        "  spearman {spearman:+.4}  (95% CI approx {lo:+.3} … {hi:+.3}, Fisher z; selection-bias INFLATED — read as an upper bound)"
    );
    // NOTE: the CI transform above is approximate at small n; the plan's
    // decision rule consumes the point estimate's BOUND DIRECTION with the
    // bias disclosure, never the third decimal.

    // Per-rule breakdown — a pooled ρ̂ driven by one dense rule is a
    // different claim than a uniform one.
    let mut by_rule: std::collections::BTreeMap<String, Vec<(f32, f32)>> = Default::default();
    for (p, r, k) in &pairs {
        by_rule.entry(k.clone()).or_default().push((*p, *r));
    }
    println!("\n  per-rule (n, spearman):");
    for (k, v) in &by_rule {
        if v.len() >= 3 {
            let pv: Vec<f32> = v.iter().map(|x| x.0).collect();
            let rv: Vec<f32> = v.iter().map(|x| x.1).collect();
            let (rp2, rr2) = (rank(&pv), rank(&rv));
            let m = v.len() as f64;
            let (a, b) = (rp2.iter().sum::<f64>() / m, rr2.iter().sum::<f64>() / m);
            let (mut c2, mut x2, mut y2) = (0.0, 0.0, 0.0);
            for i in 0..v.len() {
                c2 += (rp2[i] - a) * (rr2[i] - b);
                x2 += (rp2[i] - a).powi(2);
                y2 += (rr2[i] - b).powi(2);
            }
            let s = if x2 > 0.0 && y2 > 0.0 {
                c2 / (x2 * y2).sqrt()
            } else {
                f64::NAN
            };
            println!("    {k:<40} n={:<4} ρ={s:+.3}", v.len());
        } else {
            println!(
                "    {k:<40} n={:<4} (too thin for a per-rule read)",
                v.len()
            );
        }
    }
    let _ = (sp, sr);
}
