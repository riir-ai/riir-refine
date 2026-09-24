//! Issue 134 **T3** — the ρ-prediction depth sweep (katgpt-rs Research 585 /
//! SAT arXiv:2609.22682): does per-domain **gain-from-depth** rank-agree
//! with the T1 demonstrability constant?
//!
//! The paper's law: multi-candidate depth beats the best member exactly
//! where demonstrability (selection discriminability) is high. Our instance:
//! deepening the retrieval POOL (28 → 56 → 112 candidates) only improves
//! top-1 correctness in domains whose rerank reliably promotes the correct
//! rule when it is present. Measure both axes on the SAME labeled fixture
//! set, Spearman-correlate the domain ranks, and apply the **power floor**:
//!
//! - **ρ printed with n reported beside it** — n is the qualifying-domain
//!   count (T1's labeled-set floor, either formula's spelling);
//! - **REFUSED below FIVE qualifying domains** — at n=3 Spearman cannot
//!   reject and at n=4 it cannot reach the paper's p (the tail-support
//!   rule applied to a rank statistic). The refusal is a verdict, not an
//!   error: the sweep table still prints (the per-domain data accumulates
//!   as thin domains gain labeled sets — the instrument is the deliverable).
//!
//! The available domain set today: clippy + rust_perf always, kernel with
//! `kernel_heal` (the labeled-fixture population, `labeled_eval_spans`) —
//! n=2–3 < 5, so the honest landing verdict is REFUSED. That is the answer
//! the issue anticipated ("the thin ones may not qualify and the refusal is
//! the honest answer then").
//!
//! ```bash
//! cargo run --release --features latent_retrieval,clippy_lints,rust_perf \
//!   --example demonstrability_t3_sweep
//! # with the kernel domain (n=3):
//! cargo run --release --features latent_retrieval,clippy_lints,rust_perf,kernel_heal \
//!   --example demonstrability_t3_sweep
//! ```

// Hit-rate arithmetic: span counts (≤130) → f32 — the pedantic cast lints
// are display-metric noise, allowed with cause (the retrieval_eval
// precedent, one module over).
#![allow(clippy::cast_precision_loss)]

use riir_clippy::demonstrability::{eval_row, Row, T3_DOMAINS};
use riir_clippy::draft::instantiation_rerank::probeable;
use riir_clippy::draft::retrieval_eval::{
    canonical_rank, classify_choice_span, labeled_eval_spans, ChoiceTaxonomy,
};
use riir_clippy::draft::{DomainId, LatentMatcher};

/// Pool depths: baseline = `RERANK_POOL` (28 — the production posture),
/// then 2× and 4×. Depth means POOL depth (`top_k >= 28 ⇒ pool = top_k`),
/// not truncation — a deeper pool changes the rerank's candidate set (the
/// instantiation-first sort can promote a rule from deeper in the pool);
/// truncation alone cannot change top-1.
const DEPTHS: [usize; 3] = [28, 56, 112];

/// The T3 power floor — below this many qualifying domains the Spearman is
/// neither passed nor failed, it is REFUSED (printed with the measured n).
const POWER_FLOOR: usize = 5;

fn main() {
    let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
    let spans = labeled_eval_spans();
    println!(
        "== Issue 134 T3: gain-from-depth vs demonstrability ({} labeled spans) ==",
        spans.len()
    );
    if spans.is_empty() {
        eprintln!("REFUSED: no labeled spans (no domain features enabled?) — nothing to measure");
        std::process::exit(1);
    }

    // Per domain: top-1 hit counts at each depth + the taxonomy at baseline.
    // Keyed on the domain's LABEL string (DomainId is not Ord — no BTreeMap
    // key), in first-seen order; the render sorts by the canonical table
    // order via T3_DOMAINS anyway.
    let mut hits: std::collections::HashMap<&'static str, [usize; DEPTHS.len()]> =
        std::collections::HashMap::new();
    let mut labeled: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();
    let mut taxonomy: std::collections::HashMap<&'static str, ChoiceTaxonomy> =
        std::collections::HashMap::new();

    for span in &spans {
        if !matcher.has_domain(span.domain) {
            continue;
        }
        let di = span.domain;
        let key = canonical_label(di);
        *labeled.entry(key).or_default() += 1;
        let entry = hits.entry(key).or_default();
        // ONE suggest per depth; the matcher is deterministic in its inputs,
        // so the depth ladder is a controlled variable (same span, same
        // index, only the pool depth moves).
        for (ix, depth) in DEPTHS.iter().enumerate() {
            let (_, by_domain) = matcher.suggest_span(&span.code, *depth);
            let ranked = by_domain
                .iter()
                .find(|(d, _)| *d == di)
                .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));
            if ranked.first().is_some_and(|r| r.rule_id == span.label) {
                entry[ix] += 1;
            }
            // The taxonomy (the T1 constant's input) rides the BASELINE
            // depth and the eval's PROBE window (top-8): run_eval truncates
            // at PROBE_TOP_K=8 before classifying, so a 28-deep window would
            // silently widen `wrong_rule` (surfaced-but-outranked) at
            // `wrong_span`'s expense and shift the constant off the standing
            // column's. `take(8)` reproduces the eval window exactly (same
            // pool, same rerank — the head-8 is identical).
            if ix == 0 {
                let choice = classify_choice_span(
                    &span.label,
                    &span.acceptable,
                    ranked.iter().copied().take(8),
                    |rule_id| probeable(di, rule_id),
                );
                taxonomy.entry(key).or_default().absorb(choice);
            }
        }
    }

    // ── The sweep table + the demonstrability axis ─────────────────────
    // Deterministic row order: the T3_DOMAINS table's, then any measured
    // domain not in the table (shouldn't happen — canonical_label maps into
    // it) at the tail.
    let mut ordered: Vec<&'static str> = T3_DOMAINS
        .iter()
        .copied()
        .filter(|d| labeled.contains_key(d))
        .collect();
    ordered.extend(
        labeled
            .keys()
            .copied()
            .filter(|k| !T3_DOMAINS.contains(k)),
    );
    println!(
        "\n{:<12} {:>8} {:>10} {:>10} {:>10} {:>12}",
        "domain", "labeled", "top1@28", "top1@56", "top1@112", "demonstr."
    );
    let mut rows: Vec<(String, f32, f32)> = Vec::new(); // (domain, constant, gain)
    for key in ordered {
        let n = labeled[key];
        let h = hits.get(key).copied().unwrap_or([0; DEPTHS.len()]);
        // The T1 constant (formula 1): selected/(selected+rejected), the
        // SAME Row arithmetic the standing column prints.
        let row: Row = taxonomy.get(key).map_or_else(
            || eval_row(key, None, &ChoiceTaxonomy::default()),
            |t| eval_row(key, None, t),
        );
        let constant = row.constant();
        let base = h[0] as f32 / n as f32;
        let gain = h[DEPTHS.len() - 1] as f32 / n as f32 - base;
        println!(
            "{:<12} {:>8} {:>10} {:>10} {:>10} {:>12}",
            key,
            n,
            format_pct(base),
            format_pct(h[1] as f32 / n as f32),
            format_pct(h[DEPTHS.len() - 1] as f32 / n as f32),
            constant.map_or("-".to_string(), |c| format!("{c:.3}")),
        );
        if let Some(c) = constant {
            rows.push((key.to_string(), c, gain));
        }
    }

    // ── The Spearman + the power floor ─────────────────────────────────
    let n = rows.len();
    println!("\nqualifying domains (non-empty labeled set): {n}");
    let dropped: Vec<&str> = T3_DOMAINS
        .iter()
        .copied()
        .filter(|d| !rows.iter().any(|(r, _, _)| r == d))
        .collect();
    if !dropped.is_empty() {
        println!("dropped (no measured row): {}", dropped.join(", "));
    }
    if n < POWER_FLOOR {
        println!(
            "\nVERDICT: REFUSED — power floor: n={n} < {POWER_FLOOR} qualifying domains.\n\
             At n=3 Spearman cannot reject; at n=4 it cannot reach the paper's p. Neither\n\
             passed nor failed: the promotion question stays open until more domains carry\n\
             labeled sets (kernel_heal adds one; sec/docker/dist/shader need fixture sets).\n\
             The per-domain sweep above is the accumulating measurement, not the verdict."
        );
        return;
    }
    let rho = spearman(&rows.iter().map(|(_, c, _)| *c).collect::<Vec<_>>(),
        &rows.iter().map(|(_, _, g)| *g).collect::<Vec<_>>());
    println!("\nSpearman ρ(demonstrability, gain-from-depth) = {rho:.3}  (n={n})");
    // The paper's association is positive (ρ=0.90 across their 8 benchmarks);
    // our gate bar is the standard 0.5-tier for a promotion GOAT — below it,
    // keep measurement-only.
    if rho >= 0.5 {
        println!("VERDICT: PASS — the rank-agreement clears the 0.5 bar; promote per the standard rule.");
    } else {
        println!("VERDICT: FAIL — keep measurement-only (T1 survives as instrumentation); record the negative.");
    }
}

fn canonical_label(d: DomainId) -> &'static str {
    riir_clippy::demonstrability::canonical_domain(d.label())
}

fn format_pct(v: f32) -> String {
    format!("{:.1}%", v * 100.0)
}

/// Spearman's ρ with average ranks for ties (hand-rolled: no new deps; the
/// population is the tiny domain set, n ≤ 8).
fn rank_with_ties(values: &[f32]) -> Vec<f32> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut ranks = vec![0.0_f32; values.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && values[order[j + 1]] == values[order[i]] {
            j += 1;
        }
        let avg = (i + j) as f32 / 2.0 + 1.0;
        for k in &order[i..=j] {
            ranks[*k] = avg;
        }
        i = j + 1;
    }
    ranks
}

fn spearman(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let ra = rank_with_ties(a);
    let rb = rank_with_ties(b);
    let n = a.len() as f32;
    let d2: f32 = ra
        .iter()
        .zip(&rb)
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum();
    1.0 - 6.0 * d2 / (n * (n * n - 1.0))
}
