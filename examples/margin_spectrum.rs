//! Issue 077 T2 — the margin spectrum over the frozen v1 fixture set
//! (offline analysis FIRST; no gating, no promotion — measurement only).
//!
//! The paper (arXiv:2604.26841) claims cross-entropy training implicitly
//! maximizes classification margins and that the review/effort law
//! `w(M) ∝ e^{−2M}` concentrates on the smallest-margin items. T2 asks the
//! one measurable question on THIS corpus: does the real rule-ranking
//! margin spectrum show that concentration — i.e. do the smallest-margin
//! spans carry a disproportionate share of the `e^{−2M}` mass?
//!
//! Method (the shared substrate — [`riir_refine::draft::margin_weight`]):
//! for every frozen fixture, the most-contested pre-rerank pool margin
//! `M = top1 − top2` across enabled domains (the `rank_into` canon:
//! score-desc, `total_cmp` + `rule_id` — margins ≥ 0 by construction).
//! Weights `w = e^{−2M}`; the concentration statistic is the mass fraction
//! of the lowest-margin half / decile.
//!
//! Run (writes nothing; prints the spectrum + concentration numbers):
//!
//! ```text
//! cargo run --release --example margin_spectrum \
//!     --features score_bench,latent_retrieval
//! ```
//!
//! Numbers land in `.issues/077_margin_weighted_mining_entropy_gap_ood.md`
//! (T2). Sigmoid/score-delta shape throughout — no softmax (house rule).

// Count→ratio casts over the fixture population (35); allowed with cause.
#![allow(clippy::cast_precision_loss)]

use riir_refine::draft::{concentration_fraction, margin_weight, span_margin, LatentMatcher};
use riir_refine::score_bench::fixtures::fixtures;

/// Spectrum buckets, ascending-margin order; the upper bound is INCLUSIVE
/// (`M ≤ hi`). The last bucket catches everything above 0.5, including the
/// uncontested (`INFINITY` margin → weight 0).
const BUCKETS: [(&str, f32); 6] = [
    ("M = 0             (exact ties)", 0.0),
    ("0 < M ≤ 0.01", 0.01),
    ("0.01 < M ≤ 0.05", 0.05),
    ("0.05 < M ≤ 0.1", 0.1),
    ("0.1 < M ≤ 0.5", 0.5),
    ("M > 0.5 / uncontested", f32::INFINITY),
];

fn main() {
    let fixture_set = fixtures();
    println!(
        "# margin spectrum over the frozen v1 set ({} fixture(s)) — issue 077 T2",
        fixture_set.len()
    );
    let mut matcher = LatentMatcher::from_enabled_domains().unwrap_or_else(|e| {
        eprintln!("matcher build failed: {e}");
        std::process::exit(1);
    });
    println!(
        "# enabled domains: {:?} (min pre-rerank pool margin across domains)",
        matcher.domains()
    );

    // One retrieval pass per fixture: (id, margin, weight) rows, pre-allocated
    // to the known population — the collection pass allocates nothing further.
    let mut rows: Vec<(&str, f32, f32)> = Vec::with_capacity(fixture_set.len());
    let mut contested_domain_total = 0usize;
    for fx in &fixture_set {
        let sm = span_margin(&mut matcher, fx.code);
        contested_domain_total += sm.contested_domains;
        rows.push((fx.id, sm.min, margin_weight(sm.min)));
    }

    // Buckets.
    println!("\n── spectrum (buckets, ascending margin) ──");
    let mut bucket_weights = vec![0.0f32; BUCKETS.len()];
    let mut bucket_counts = vec![0usize; BUCKETS.len()];
    for &(_id, margin, weight) in &rows {
        let idx = BUCKETS
            .iter()
            .position(|(_, hi)| margin <= *hi)
            .unwrap_or(BUCKETS.len() - 1);
        bucket_weights[idx] += weight;
        bucket_counts[idx] += 1;
    }
    let total_weight: f32 = rows.iter().map(|(_, _, w)| w).sum();
    for (i, (label, _)) in BUCKETS.iter().enumerate() {
        println!(
            "{:<28} n={:<3} weight={:<9.4} mass={:>5.1}%",
            label,
            bucket_counts[i],
            bucket_weights[i],
            if total_weight > 0.0 {
                100.0 * bucket_weights[i] / total_weight
            } else {
                0.0
            }
        );
    }

    // The concentration claim: lowest-margin half / decile mass share.
    // Weights sorted DESCENDING = margins ASCENDING (the law's priority
    // order); the top-k of that order IS the smallest-margin k.
    let mut weights_desc: Vec<f32> = rows.iter().map(|&(_, _, w)| w).collect();
    weights_desc.sort_by(|a, b| b.total_cmp(a));
    let n = weights_desc.len();
    let half = n.div_ceil(2);
    let decile = (n / 10).max(1);
    let half_frac = concentration_fraction(&weights_desc, half);
    let decile_frac = concentration_fraction(&weights_desc, decile);
    let contested = rows.iter().filter(|&&(_, m, _)| m.is_finite()).count();

    println!("\n── concentration (the paper's claim, on this corpus) ──");
    println!("contested spans: {contested}/{n} (margin < INFINITY — ≥2 candidates in some domain)");
    println!(
        "lowest-margin HALF   ({half:>2} spans): {:>5.1}% of total e^{{−2M}} mass",
        100.0 * half_frac
    );
    println!(
        "lowest-margin DECILE ({decile:>2} spans): {:>5.1}% of total e^{{−2M}} mass",
        100.0 * decile_frac
    );
    println!(
        "(the ≥50%-in-half inequality is structural for any non-negative \
         descending weights — the measured question is how FAR above 50% \
         the real corpus sits, and whether the decile spike justifies \
         e^{{−2M}}-weighted mining order — issue 077 T3)"
    );
    println!(
        "\ndomain-contest census: {contested_domain_total} contested (fixture, domain) view(s) \
         across {} domain(s)",
        matcher.domains().len()
    );

    // The full table, margin-ascending — the review-priority order the law
    // prescribes (what T3's mining arm consumes).
    rows.sort_by(|a, b| a.1.total_cmp(&b.1));
    println!("\n── per-fixture margins, law-priority order (smallest first) ──");
    for (id, margin, weight) in &rows {
        println!("{id:<52} M={:<10} w={:.6}", fmt_margin(*margin), weight);
    }
}

/// Human form for the margin value (`∞` for uncontested).
fn fmt_margin(m: f32) -> String {
    if m.is_infinite() {
        "∞".to_string()
    } else {
        format!("{m:.4}")
    }
}
