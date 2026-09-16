//! Plan 005 T7.1/T7.2 — do EMA-evolved domain directions beat corpus centroids?
//!
//! T7.1 generalizes `LatentSkillEvolution`'s key from per-lint to per-domain;
//! T7.2 feeds fix outcomes into it so "the dMoE router gets smarter with use".
//! Measured before wiring, because two of the plan's premises are stale and
//! the third has an analytical answer worth testing:
//!
//! 1. **The router cannot move G1.** Retrieval is fan-out and the router
//!    verdict is display-only (`heal.rs:241`) — established in T3.5.1. So the
//!    only metric this can move is *router annotation accuracy*, currently a
//!    decorative number (Issue 020 killed gating at an 85% bar).
//! 2. **The shipped router does not read these directions.** Issue 020
//!    replaced the 8-D `pick_domain` baseline with `TfCosine` 128-D centroids
//!    (68.8% vs the baseline's 53.1%). The 8-D `directions` T7.2 would evolve
//!    are not on the shipped path at all.
//! 3. **A centroid is already the optimum.** For cosine routing, the
//!    L2-normalized class mean IS the least-squares point estimate. An online
//!    EMA over outcomes drawn from the same distribution converges to the same
//!    place with *more* variance — so on static data it should tie at best.
//!
//! Point 3 is a prediction, so this test tries to falsify it. Both arms see
//! exactly the same training signal (the corpus patterns) and are scored on
//! spans neither has seen, so the comparison is fair rather than rigged.
//!
//! ```bash
//! cargo test --release --features latent_retrieval --test t71_evolved_directions -- --nocapture
//! ```

#![cfg(feature = "latent_retrieval")]

use riir_clippy::domain::{HealDomain, HealEntry};
use riir_clippy::draft::DomainId;
use riir_clippy::evolve::{EvolveConfig, LatentSkillEvolution};

const DIM: usize = 8;

fn corpus_for(domain: DomainId) -> Vec<String> {
    match domain {
        DomainId::Clippy => riir_clippy::domains::clippy_lints::ClippyLintDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect(),
        DomainId::Perf => riir_clippy::domains::rust_perf::RustPerfDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect(),
        DomainId::Kernel => Vec::new(),
        DomainId::Sec => Vec::new(),
        DomainId::Docker => Vec::new(),
        DomainId::Dist => Vec::new(),
        DomainId::ShaderPerf => Vec::new(),
    }
}

fn embed(text: &str) -> [f32; DIM] {
    use riir_rag::{Embedder, ModellessEmbedder};
    ModellessEmbedder.embed(text)
}

fn normalize(v: &mut [f32; DIM]) {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

/// L2-normalized class mean — what `DomainRouter` ships.
fn centroid(patterns: &[String]) -> [f32; DIM] {
    let mut c = [0.0f32; DIM];
    for p in patterns {
        let e = embed(p);
        for (a, b) in c.iter_mut().zip(e) {
            *a += b;
        }
    }
    normalize(&mut c);
    c
}

fn argmax_domain(dirs: &[(DomainId, [f32; DIM])], activity: &[f32; DIM]) -> DomainId {
    let mut best = (f32::NEG_INFINITY, dirs[0].0);
    for (id, d) in dirs {
        let dot: f32 = d.iter().zip(activity).map(|(a, b)| a * b).sum();
        if dot > best.0 {
            best = (dot, *id);
        }
    }
    best.1
}

#[test]
fn t71_evolved_vs_centroid_routing() {
    println!("\n=== T7.1/T7.2 — EMA-evolved domain directions vs corpus centroids ===");

    let domains = [DomainId::Clippy, DomainId::Perf];
    let corpora: Vec<(DomainId, Vec<String>)> =
        domains.iter().map(|d| (*d, corpus_for(*d))).collect();
    for (d, c) in &corpora {
        println!("  {:?}: {} corpus patterns", d, c.len());
    }

    // ── Arm A: centroid (the shipped estimator) ──
    let centroids: Vec<(DomainId, [f32; DIM])> =
        corpora.iter().map(|(d, c)| (*d, centroid(c))).collect();

    // ── Arm B: EMA-evolved, per-DOMAIN key (this is T7.1's generalization —
    //    the store is already `HashMap<String, Direction>`, so a domain key
    //    works today; what was missing was a consumer). Same training signal
    //    as arm A: one recorded success per corpus pattern.
    let mut evolution = LatentSkillEvolution::new(EvolveConfig::default());
    let mut cycles = 0usize;
    for (d, patterns) in &corpora {
        let key = format!("{d:?}");
        for p in patterns {
            evolution.record_outcome(&key, &embed(p), true);
            cycles += 1;
        }
    }
    println!("  evolved over {cycles} recorded fix outcomes (domain-keyed)");

    let evolved: Vec<(DomainId, [f32; DIM])> = corpora
        .iter()
        .map(|(d, _)| {
            let mut dir = *evolution.get(&format!("{d:?}")).expect("direction present");
            normalize(&mut dir);
            (*d, dir)
        })
        .collect();

    // ── Score both on spans NEITHER arm trained on ──
    let mut eval: Vec<(DomainId, &str)> = Vec::new();
    for (_, code) in riir_clippy::draft::eval_fixtures_clippy::CLIPPY_SPANS {
        eval.push((DomainId::Clippy, code));
    }
    for (_, code) in riir_clippy::draft::eval_fixtures_perf::PERF_SPANS {
        eval.push((DomainId::Perf, code));
    }
    println!("  scored on {} held-out spans", eval.len());

    let score = |dirs: &[(DomainId, [f32; DIM])]| -> usize {
        eval.iter()
            .filter(|(want, code)| argmax_domain(dirs, &embed(code)) == *want)
            .count()
    };
    let (c_hits, e_hits) = (score(&centroids), score(&evolved));
    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize| k as f32 / eval.len() as f32 * 100.0;

    println!("\n{:<22} {:>10}", "estimator", "routing acc");
    println!("{:<22} {:>9.1}%", "corpus centroid", pct(c_hits));
    println!("{:<22} {:>9.1}%", "EMA-evolved", pct(e_hits));
    println!(
        "  delta: {:+.1}pp    (Issue 020 gating bar: 85%; shipped TfCosine annotator: 68.8%)",
        pct(e_hits) - pct(c_hits)
    );

    let verdict = if pct(e_hits) >= 85.0 {
        "EVOLUTION CLEARS THE GATING BAR — router gating is revivable, wire it"
    } else if e_hits > c_hits {
        "evolution beats the centroid but stays under the 85% gating bar — \
         still a decorative metric"
    } else {
        "PREDICTION HELD — evolution does not beat the centroid. On static data \
         the L2-normalized class mean is already the cosine optimum; an online \
         EMA is a noisier estimate of the same quantity, not a better one."
    };
    println!("\n  VERDICT: {verdict}");
}
