//! Issue 157 P0.2 — the retrieval-negative channel's MISS-SET + NEAR-TIE
//! census (print-only, measurement-only; never in default runs).
//!
//! The triage verdict chain (Claude verdict session `2b8a1316`, rounds 1–3,
//! recorded in `.issues/157`) gated this probe behind pre-registrations that
//! are stated HERE, in the file that produces the numbers, before the run:
//!
//! - **Margin X (the near-tie threshold) is a scale-free FRACTION of the
//!   span's own raw fused top-1 pool score**, fixed at {5%, 10%, 25%} with
//!   **10% the primary** — chosen before the census ran. Rationale: the pool
//!   score is an unnormalized fused latent+BM25 similarity (Bench 018's raw
//!   [3.19..34.71] class), so an ABSOLUTE threshold would not transfer across
//!   spans/corpus; a fraction of the span's own top-1 score does. The census
//!   measures on the PRE-rerank pool (the fused-score surface a latent-dot
//!   demotion actually modifies), while hit/top-1 identity comes from the
//!   shipped `canonical_rank` (instantiation-first) — the order demotion
//!   must actually flip.
//! - **Reachability classes** per top-1 miss span (a score demotion of the
//!   wrong top-1 can only promote gold if the shipped canon lets it):
//!   - `R1 gold-instantiates`: gold carries `instantiates` — demoting the
//!     wrong top-1 promotes gold iff the margin ≤ X (score decides within
//!     the instantiating group). THE flip-reachable class.
//!   - `R2 gold-does-not-instantiate`: gold lacks the flag — a score
//!     demotion can lift gold only past non-instantiating rivals; every
//!     other instantiating rule still outranks it. Counted separately.
//!   - `R3 gold-absent`: gold is in neither the ranked window nor the pool —
//!     retrieval never surfaced it; no demotion helps.
//! - **UNPOWERED vocabulary**: if the miss set is too small to inform the
//!   channel decision, the recorded verdict is UNPOWERED, never a
//!   measured negative (round-2 amendment 3).
//!
//! Reads nothing but the in-tree corpus + fixtures; writes nothing; prints
//! the per-miss anatomy (top-1 rule, gold rank, margin, instantiates flags)
//! so the negative-rule overlap can be joined offline against a trajectory
//! store's `rejected` labels (Issue 148 T-b rows).
//!
//! ```text
//! cargo run --release --features latent_retrieval,clippy_lints,rust_perf,kernel_opt \
//!   --example p157_near_tie_census
//! ```

use riir_refine::draft::RetrievalConfig;
use riir_refine::draft::domain_router::DomainId;
use riir_refine::draft::latent_matcher::LatentMatcher;
use riir_refine::draft::retrieval_eval::{canonical_rank, labeled_eval_spans};

/// Pre-registered near-tie fractions; index 1 (10%) is the PRIMARY readout.
const MARGIN_FRACTIONS: [(&str, f32); 3] = [("5%", 0.05), ("10%", 0.10), ("25%", 0.25)];

/// Index of the primary fraction in [`MARGIN_FRACTIONS`].
const PRIMARY: usize = 1;

/// Probe window — mirrors `retrieval_eval::PROBE_TOP_K` (private there;
/// duplicated deliberately, one line, the linking_confusion precedent).
const TOP_K: usize = 8;

struct MissRow {
    domain: DomainId,
    label: String,
    top1: String,
    top1_inst: bool,
    gold_rank: Option<usize>,
    gold_inst: Option<bool>,
    /// PRE-rerank pool margin: pool[top1].score − pool[gold].score.
    /// `None` when gold (or the top-1) is absent from the pool.
    margin: Option<f32>,
    top1_score: Option<f32>,
}

fn main() {
    let mut matcher = LatentMatcher::from_enabled_domains_with(RetrievalConfig::default())
        .expect("build matcher");
    let spans = labeled_eval_spans();
    println!("p157 near-tie census — spans={} TOP_K={TOP_K}", spans.len());
    println!(
        "pre-registered margins: {:?} (primary = 10% of the span's own fused top-1 pool score)",
        MARGIN_FRACTIONS.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );

    let mut hits = 0usize;
    let mut skipped = 0usize;
    let mut misses: Vec<MissRow> = Vec::new();

    for span in &spans {
        if !matcher.has_domain(span.domain) {
            skipped += 1;
            continue;
        }
        let Some((pool, ranked)) = matcher.pool_and_ranked(span.domain, &span.code) else {
            skipped += 1;
            continue;
        };
        let canon = canonical_rank(&ranked);
        let Some(top1) = canon.first() else {
            skipped += 1;
            continue;
        };
        if top1.rule_id == span.label {
            hits += 1;
            continue;
        }
        let gold_rank = canon.iter().position(|r| r.rule_id == span.label);
        let pool_top1_score = pool
            .iter()
            .find(|r| r.rule_id == top1.rule_id)
            .map(|r| r.score);
        let pool_gold_score = pool
            .iter()
            .find(|r| r.rule_id == span.label)
            .map(|r| r.score);
        let margin = match (pool_top1_score, pool_gold_score) {
            (Some(t), Some(g)) => Some(t - g),
            _ => None,
        };
        misses.push(MissRow {
            domain: span.domain,
            label: span.label.clone(),
            top1: top1.rule_id.clone(),
            top1_inst: top1.instantiates,
            gold_rank,
            gold_inst: canon
                .iter()
                .find(|r| r.rule_id == span.label)
                .map(|r| r.instantiates),
            margin,
            top1_score: pool_top1_score,
        });
    }

    let measured = hits + misses.len();
    println!(
        "\nmeasured={measured} (hits={hits} misses={} skipped={skipped})",
        misses.len()
    );
    println!(
        "hits_at_1 = {:.1}%",
        100.0 * hits as f32 / measured.max(1) as f32
    );

    println!("\n── per-miss anatomy ──");
    for m in &misses {
        let margin_s = m
            .margin
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "gold-absent".to_string());
        let frac_s = match (m.margin, m.top1_score) {
            (Some(mg), Some(t)) if t > 0.0 => format!("{:.1}%", 100.0 * mg / t),
            _ => "—".to_string(),
        };
        println!(
            "  {:?} label={:32} top1={:32} (inst={}) gold_rank={:?} gold_inst={:?} margin={margin_s} ({frac_s} of top1)",
            m.domain, m.label, m.top1, m.top1_inst, m.gold_rank, m.gold_inst,
        );
    }

    println!("\n── near-tie census (miss spans, by pre-registered fraction) ──");
    for (name, x) in MARGIN_FRACTIONS {
        let near = misses
            .iter()
            .filter(|m| match (m.margin, m.top1_score) {
                (Some(mg), Some(t)) if t > 0.0 => mg <= x * t,
                _ => false,
            })
            .count();
        println!("  X={name:>3}: {near} of {} miss spans", misses.len());
    }

    println!("\n── reachability classes ──");
    let r1 = misses
        .iter()
        .filter(|m| m.gold_inst == Some(true) && m.gold_rank.is_some())
        .count();
    let r2 = misses
        .iter()
        .filter(|m| m.gold_inst == Some(false) && m.gold_rank.is_some())
        .count();
    let r3 = misses.iter().filter(|m| m.gold_rank.is_none()).count();
    println!("  R1 gold-instantiates (flip-reachable): {r1}");
    println!("  R2 gold-not-instantiating:             {r2}");
    println!("  R3 gold-absent (retrieval miss):       {r3}");

    // The PRIMARY combined readout: R1 spans that are ALSO near-tie at 10% —
    // the population a compliant negative channel could actually flip.
    let primary_x = MARGIN_FRACTIONS[PRIMARY].1;
    let flip_reachable = misses
        .iter()
        .filter(|m| {
            m.gold_inst == Some(true)
                && match (m.margin, m.top1_score) {
                    (Some(mg), Some(t)) if t > 0.0 => mg <= primary_x * t,
                    _ => false,
                }
        })
        .collect::<Vec<_>>();
    println!(
        "\nPRIMARY (X=10% ∧ R1): {} flip-reachable miss span(s)",
        flip_reachable.len()
    );
    for m in &flip_reachable {
        println!("  {:?} {} (top1={})", m.domain, m.label, m.top1);
    }
    println!("\nwrong-top-1 rule ids (for the offline rejected-label overlap join):");
    let mut tops: Vec<&String> = misses.iter().map(|m| &m.top1).collect();
    tops.sort();
    tops.dedup();
    for t in &tops {
        println!("  {t}");
    }
}
