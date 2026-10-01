//! katgpt-rs Issue 882 P0 rider — differential-anchor query correction on
//! the healer's 8-D latent retrieval leg, A/B'd on the labeled eval spans.
//!
//! `q̂ = q − λ·ā`, ā = the per-domain mean-CORPUS ModellessEmbedder direction
//! (fit at index build over every indexed chunk). The hypothesis the sweep
//! tests: the 8-D embedder is sigmoid-bounded (positive orthant), so raw
//! cosine carries a large common-mode floor, which would explain Bench 027's
//! measured refutation of the latent weight ("the 8-D latent term never
//! changes a ranking"). If the floor is the reason, subtracting it should
//! make the latent term informative — visible as top-1 moving with λ at
//! some latent weight. If top-1 is flat in λ at every weight, the latent
//! leg carries no ranking signal that common-mode rejection can recover and
//! the rider is a measured NEGATIVE for this consumer.
//!
//! Gates:
//! - G3: λ = 0 is bit-identical to the uncorrected matcher (scores via
//!   `to_bits`, every span × every domain) — the kill switch.
//! - G1: λ* picked by direct evaluation (`pick_lambda_by_eval`, never GD) on
//!   blended top-1 per latent weight; per-domain top-1 reported beside it
//!   (trap 1: hubness ≠ illegitimacy — a domain-level regression is a
//!   finding even when the blend improves).
//! - Hubness axis: the per-rule top-1 WIN-COUNT skewness (Bench 886
//!   Finding 1 — the mean-sim skewness cannot move under a mean anchor).

use katgpt_core::differential_anchor::{hubness_skewness, pick_lambda_by_eval};
use riir_refine::draft::retrieval_eval::{labeled_eval_spans, run_eval_on, EvalReport};
use riir_refine::draft::{DomainId, LatentMatcher, RetrievalConfig};
use std::collections::HashMap;

const LAMBDAS: [f32; 7] = [0.0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5];
const WEIGHTS: [f32; 3] = [RetrievalConfig::DEFAULT_LATENT_WEIGHT, 4.0, 16.0];

fn matcher(latent_weight: f32, lambda: f32) -> LatentMatcher {
    LatentMatcher::from_enabled_domains_with(RetrievalConfig {
        latent_weight,
        ..RetrievalConfig::default()
    })
    .expect("build matcher")
    .with_differential_anchor(lambda)
}

fn blended_top1(r: &EvalReport) -> f32 {
    r.blended.recall(1).expect("no labeled spans — feature set wrong")
}

/// Per-rule own-domain top-1 win counts over the labeled spans → skewness.
fn win_count_skewness(m: &mut LatentMatcher) -> f32 {
    let mut wins: HashMap<String, f32> = HashMap::new();
    for span in labeled_eval_spans() {
        let (_, by_domain) = m.suggest_span(&span.code, 1);
        if let Some((_, rules)) = by_domain.iter().find(|(d, _)| *d == span.domain) {
            if let Some(top) = rules.first() {
                *wins.entry(top.rule_id.clone()).or_default() += 1.0;
            }
        }
    }
    let xs: Vec<f32> = wins.into_values().collect();
    hubness_skewness(&xs)
}

#[test]
fn g3_lambda_zero_is_bit_identical() {
    // The base arm must be the UNCORRECTED matcher — the env knob would
    // seed it otherwise and the comparison would be corrected-vs-λ=0.
    assert!(
        std::env::var(riir_refine::draft::corpus_index::DIFF_ANCHOR_LAMBDA_ENV).is_err(),
        "unset RIIR_CLIPPY_DIFF_ANCHOR_LAMBDA for the G3 arm"
    );
    let mut base = LatentMatcher::from_enabled_domains().expect("build matcher");
    let mut zero = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_differential_anchor(0.0);
    let mut n = 0usize;
    for span in labeled_eval_spans() {
        let (_, a) = base.suggest_span(&span.code, 12);
        let (_, b) = zero.suggest_span(&span.code, 12);
        assert_eq!(a.len(), b.len());
        for ((da, ra), (db, rb)) in a.iter().zip(&b) {
            assert_eq!(da, db);
            assert_eq!(ra.len(), rb.len(), "{}", span.label);
            for (x, y) in ra.iter().zip(rb) {
                assert_eq!(x.rule_id, y.rule_id, "{}", span.label);
                assert_eq!(x.score.to_bits(), y.score.to_bits(), "{}", span.label);
                n += 1;
            }
        }
    }
    assert!(n > 100, "G3 compared only {n} scored rules — population floor");
}

#[test]
fn g3_nonzero_lambda_actually_moves_scores() {
    // Liveness: a λ that changes nothing would make every sweep row a
    // green zero. The corrected direction must change SOME latent score.
    let mut base = LatentMatcher::from_enabled_domains().expect("build matcher");
    let mut corr = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_differential_anchor(1.0);
    let moved = labeled_eval_spans().iter().any(|span| {
        let (_, a) = base.suggest_span(&span.code, 12);
        let (_, b) = corr.suggest_span(&span.code, 12);
        a.iter().zip(&b).any(|((_, ra), (_, rb))| {
            ra.iter()
                .zip(rb)
                .any(|(x, y)| x.rule_id != y.rule_id || x.score.to_bits() != y.score.to_bits())
        })
    });
    assert!(moved, "λ=1 moved no score — the correction is not wired");
}

#[test]
#[allow(clippy::cast_precision_loss)]
fn g1_lambda_sweep_per_weight() {
    let domains = [
        DomainId::Clippy,
        DomainId::Perf,
        DomainId::Docker,
        DomainId::Dist,
    ];
    println!("\n  w      λ    blend@1  blend@3  win-skew   per-domain @1 (clippy perf docker dist)");
    let mut verdicts = Vec::new();
    for &w in &WEIGHTS {
        let mut rows: Vec<(f32, f32)> = Vec::new();
        for &lambda in &LAMBDAS {
            let mut m = matcher(w, lambda);
            let skew = win_count_skewness(&mut m);
            let r = run_eval_on(m);
            let b1 = blended_top1(&r);
            let b3 = r.blended.recall(3).unwrap_or(0.0);
            let per: Vec<String> = domains
                .iter()
                .map(|d| {
                    let i = DomainId::ALL.iter().position(|x| x == d).unwrap();
                    r.per_domain[i]
                        .recall(1)
                        .map_or_else(|| "  -  ".into(), |v| format!("{v:.3}"))
                })
                .collect();
            println!(
                "  {w:<5.1}  {lambda:<4.2}  {b1:.4}   {b3:.4}   {skew:+.3}     {}",
                per.join("  ")
            );
            assert!(b1.is_finite() && skew.is_finite());
            rows.push((lambda, b1));
        }
        let star = pick_lambda_by_eval(&LAMBDAS, |l| {
            rows.iter().find(|(x, _)| *x == l).map_or(0.0, |(_, v)| *v)
        });
        let at_star = rows.iter().find(|(x, _)| *x == star).unwrap().1;
        let at_zero = rows[0].1;
        println!(
            "  w={w}: λ*={star} top-1 {at_star:.4} vs λ=0 {at_zero:.4} (Δ {:+.4})",
            at_star - at_zero
        );
        // λ* by direct evaluation can never be worse than λ=0 (λ=0 is on
        // the grid) — asserting it pins the selector, not the hypothesis.
        assert!(at_star >= at_zero);
        verdicts.push((w, star, at_star - at_zero));
    }
    println!("  verdicts (w, λ*, Δtop-1 vs λ=0): {verdicts:?}");
}

/// Name every span whose own-domain top-1 correctness flips between λ=0
/// and λ=0.5 at the shipped weight — so the aggregate Δ is attributable
/// (trap 1: a flip onto a mechanical high-frequency rule may be a CORRECT
/// hub, not noise). Report-only.
#[test]
fn g1_top1_flips_named() {
    let mut a = matcher(RetrievalConfig::DEFAULT_LATENT_WEIGHT, 0.0);
    let mut b = matcher(RetrievalConfig::DEFAULT_LATENT_WEIGHT, 0.5);
    let top1 = |m: &mut LatentMatcher, code: &str, d: DomainId| -> String {
        let (_, by) = m.suggest_span(code, 12);
        by.iter()
            .find(|(x, _)| *x == d)
            .and_then(|(_, r)| riir_refine::draft::retrieval_eval::canonical_rank(r).first().map(|s| s.rule_id.clone()))
            .unwrap_or_default()
    };
    let (mut gained, mut lost) = (0usize, 0usize);
    for span in labeled_eval_spans() {
        let ta = top1(&mut a, &span.code, span.domain);
        let tb = top1(&mut b, &span.code, span.domain);
        let (ha, hb) = (ta == span.label, tb == span.label);
        if ha != hb {
            println!(
                "  {:?} label={} λ=0 top1={ta} → λ=0.5 top1={tb} ({})",
                span.domain,
                span.label,
                if hb { "GAINED" } else { "LOST" }
            );
            if hb { gained += 1 } else { lost += 1 }
        }
    }
    println!("  flips: gained {gained}, lost {lost}");
}
