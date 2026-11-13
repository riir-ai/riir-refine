//! Issue 233 T3 — the calibrated-dispatch census (report-only, exit 0).
//!
//! Per (domain × span-shape) group over the oracle-labeled eval fixtures
//! ([`riir_refine::draft::retrieval_eval::labeled_eval_spans`]): the labeled
//! count, the narrowed-posture success count, the Wilson 95% LCB, and the
//! derived verdict against the τ = 0.6 floor + the 20-span small-group rule.
//! Then the GOAT cell: recall@8 of the narrowed posture vs the full fan-out
//! on exactly the spans the derived table would narrow, plus the fan-out cut
//! proxy (rules the retain drops from the pool).
//!
//! Both postures replicate the dispatch path's OWN steps on
//! [`LatentMatcher::pool_and_ranked`]'s pool — pool → the annotated-rule
//! retain → `mark_instantiation_with_file` → `rerank` → truncate — the same
//! functions in the same order `suggest_all_domains` runs them, so the
//! narrowed arm is the true narrowed posture, not a top-8 approximation.
//! (The matcher itself runs with the pinned table EMPTY — the landing
//! posture — so its own suggest path is the full fan-out; the census derives
//! and applies the table locally.)
//!
//! The G2 timing leg (added p50/p95 per file, bench_preflight PROVENANCE)
//! is NOT measurable on a busy box — this census prints the cost PROXY only
//! and names the quiet-box window the real leg needs.
//!
//! Re-run lane: re-run this before revisiting any [`EVIDENCE_GROUPS`] pin
//! (the issue's fixture note). Widening a group = the Issue 216 registration
//! discipline: re-verify the claim + re-run the G2 leg.
//!
//! Run:
//! ```text
//! cargo run --release --features calibrated_dispatch,latent_retrieval,clippy_lints,rust_perf \
//!   --example calibrated_dispatch_census
//! ```

use riir_refine::domain::{SpanShape, SpanShapeSet};
use riir_refine::draft::DomainId;
use riir_refine::draft::LatentMatcher;
use riir_refine::draft::calibrated_dispatch::{
    GroupStats, GroupVerdict, MIN_LABELED, TAU, narrowing_applies_with, verdict_of, wilson_lcb,
};
use riir_refine::draft::instantiation_rerank::{RerankMode, mark_instantiation_with_file, rerank};
use riir_refine::draft::latent_matcher::span_shape::{classify, rule_admits};
use riir_refine::draft::retrieval_eval::labeled_eval_spans;

/// Mirrors `retrieval_eval::PROBE_TOP_K` (private there; duplicated
/// deliberately, one line, the `linking_confusion` precedent).
const PROBE_TOP_K: usize = 8;

/// Every categorical shape, declaration order — the group walk.
const ALL_SHAPES: [SpanShape; 10] = [
    SpanShape::FnDecl,
    SpanShape::TestFn,
    SpanShape::AsyncFn,
    SpanShape::ImplBlock,
    SpanShape::TraitDecl,
    SpanShape::TypeDecl,
    SpanShape::ModBlock,
    SpanShape::MacroItem,
    SpanShape::HotLoop,
    SpanShape::MatchBlock,
];

fn carries(verdict: SpanShapeSet, shape: SpanShape) -> bool {
    verdict.intersects(SpanShapeSet::empty().with(shape))
}

struct SpanOutcome {
    domain: DomainId,
    verdict: SpanShapeSet,
    full_hit: bool,
    narrowed_hit: bool,
    pool_len: usize,
    dropped: usize,
}

fn main() {
    println!("== i233 T3 calibrated-dispatch census ==");
    println!(
        "floor tau={TAU} · small-group rule n>={MIN_LABELED} · Wilson 95% LCB · probe k={PROBE_TOP_K}"
    );

    let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
    let spans = labeled_eval_spans();

    let mut skipped_domain = 0usize;
    let mut unclassified = 0usize;
    let mut outcomes: Vec<SpanOutcome> = Vec::new();

    for span in &spans {
        if !matcher.has_domain(span.domain) {
            skipped_domain += 1;
            continue;
        }
        let Some((pool, ranked_full)) = matcher.pool_and_ranked(span.domain, &span.code) else {
            skipped_domain += 1;
            continue;
        };
        let file = syn::parse_file(&span.code).ok();
        let verdict = classify(&span.code, file.as_ref());
        if verdict == SpanShapeSet::ALL {
            // No usable parse or no recognized root item — the dispatch
            // never arms (both postures identical); belongs to no group.
            unclassified += 1;
            continue;
        }
        // The full arm reads the production serve order (`rerank`'s key
        // order — what `suggest_span` truncates from).
        let full_hit = ranked_full
            .iter()
            .take(PROBE_TOP_K)
            .any(|r| r.rule_id == span.label);

        // The narrowed posture: the dispatch path's own steps, on the same
        // pre-mark pool (post active-set exclusions — `pool_and_ranked`
        // already honors them), with the FULL hand-registered narrowing
        // armed. The gold-label answer under the widest narrowing is the
        // evidence the floor calibrates on.
        let mut narrowed = pool.clone();
        narrowed.retain(|r| rule_admits(&r.rule_id, verdict));
        let dropped = pool.len() - narrowed.len();
        if let Some(file) = &file {
            mark_instantiation_with_file(
                span.domain,
                &span.code,
                file,
                &mut narrowed,
                RerankMode::default(),
            );
        }
        rerank(&mut narrowed);
        let narrowed_hit = narrowed
            .iter()
            .take(PROBE_TOP_K)
            .any(|r| r.rule_id == span.label);

        outcomes.push(SpanOutcome {
            domain: span.domain,
            verdict,
            full_hit,
            narrowed_hit,
            pool_len: pool.len(),
            dropped,
        });
    }

    println!(
        "spans: {} labeled · {skipped_domain} skipped (domain disabled) · {unclassified} unclassified (verdict ALL — never narrows) · {} grouped",
        spans.len(),
        outcomes.len()
    );

    // Per-(domain × shape) group census. All domains print (the evidence
    // informs any future domain extension), but only Perf rows are
    // consultable — the dispatch retain is Perf-only, so the derived table
    // and the GOAT cell below are strictly Perf.
    println!("\n-- group census (domain × shape; Perf rows are the consultable ones) --");
    println!(
        "{:<8} {:<10} {:>6} {:>6} {:>8} {:>8}  verdict",
        "domain", "shape", "n", "succ", "rate", "lcb"
    );
    let mut derived: Vec<(DomainId, SpanShape, GroupStats)> = Vec::new();
    let mut any_measurable = false;
    for domain in DomainId::ALL {
        for &shape in &ALL_SHAPES {
            let members: Vec<&SpanOutcome> = outcomes
                .iter()
                .filter(|o| o.domain == domain && carries(o.verdict, shape))
                .collect();
            if members.is_empty() {
                continue;
            }
            let n = members.len();
            let succ = members.iter().filter(|o| o.narrowed_hit).count();
            let stats = GroupStats {
                labeled: n as u32,
                successes: succ as u32,
            };
            let verdict = verdict_of(stats);
            if n >= MIN_LABELED {
                any_measurable = true;
            }
            if verdict == GroupVerdict::Calibrated && domain == DomainId::Perf {
                derived.push((domain, shape, stats));
            }
            let lcb = wilson_lcb(stats.successes, stats.labeled);
            let (rate, lcb_s) = match lcb {
                Some(v) => (
                    format!(
                        "{:.3}",
                        f64::from(stats.successes) / f64::from(stats.labeled)
                    ),
                    format!("{v:.3}"),
                ),
                None => ("n/a".to_string(), "n/a".to_string()),
            };
            let name = match verdict {
                GroupVerdict::Calibrated => "CALIBRATED",
                GroupVerdict::Fallback => "FALLBACK",
                GroupVerdict::Sparse => "sparse",
            };
            println!(
                "{:<8} {:<10} {:>6} {:>6} {:>8} {:>8}  {name}",
                domain.label(),
                format!("{shape:?}"),
                n,
                succ,
                rate,
                lcb_s
            );
        }
    }

    // The derived table, printable as a future pin (never paste run-to-pass —
    // the GOAT gate + census provenance own a row).
    println!("\n-- derived table (Calibrated rows only) --");
    if derived.is_empty() {
        println!("(empty — every group Sparse or under the floor)");
    } else {
        for (domain, shape, stats) in &derived {
            println!(
                "    (DomainId::{:?}, SpanShape::{:?}, GroupStats {{ labeled: {}, successes: {} }}),",
                format!("{domain:?}"),
                format!("{shape:?}"),
                stats.labeled,
                stats.successes
            );
        }
    }

    // GOAT cell: recall@k narrowed vs full on the PERF spans the derived
    // table would actually narrow (the consult's domain — other domains
    // never narrow, their spans stay full fan-out by construction).
    println!("\n-- GOAT cell (narrowed vs full, on the Perf spans the derived table narrows) --");
    let narrowed_spans: Vec<&SpanOutcome> = outcomes
        .iter()
        .filter(|o| o.domain == DomainId::Perf && narrowing_applies_with(&derived, o.verdict))
        .collect();
    if narrowed_spans.is_empty() {
        println!("no span narrows under the derived table — recall cell vacuous");
    } else {
        let full_hits = narrowed_spans.iter().filter(|o| o.full_hit).count();
        let narrowed_hits = narrowed_spans.iter().filter(|o| o.narrowed_hit).count();
        let losses = narrowed_spans
            .iter()
            .filter(|o| o.full_hit && !o.narrowed_hit)
            .count();
        let n = narrowed_spans.len();
        println!(
            "spans narrowed: {n} · recall full {:.4} ({full_hits}/{n}) · recall narrowed {:.4} ({narrowed_hits}/{n}) · full→narrowed recall losses: {losses}",
            f64::from(full_hits as u32) / n as f64,
            f64::from(narrowed_hits as u32) / n as f64,
        );
        let dropped_sum: usize = narrowed_spans.iter().map(|o| o.dropped).sum();
        let pool_sum: usize = narrowed_spans.iter().map(|o| o.pool_len).sum();
        println!(
            "cost proxy: mean pool {:.1} · mean retained {:.1} · mean dropped {:.1} ({:.1}% of pool)",
            pool_sum as f64 / n as f64,
            (pool_sum - dropped_sum) as f64 / n as f64,
            dropped_sum as f64 / n as f64,
            100.0 * dropped_sum as f64 / pool_sum.max(1) as f64,
        );
    }

    // Verdict.
    println!("\n-- verdict --");
    if !any_measurable {
        println!(
            "0 groups reach n>={MIN_LABELED} over the static eval fixtures — static evidence \
             cannot calibrate any (domain × shape) group; EVIDENCE_GROUPS stays EMPTY (all-Sparse, \
             fail-open to the full fan-out)."
        );
        println!(
            "the widening path is the oracle-fed battery (auto-oracle --clippy-json labels over \
             real workspace scans; Issue 216 registration discipline), and the G2 timing leg \
             (added p50/p95 per file, bench_preflight PROVENANCE) owns a quiet-box window."
        );
    } else {
        println!(
            "measurable groups exist — read the census rows above: Calibrated rows are pin \
             candidates (GOAT gate + provenance first), Fallback rows are measured refusals."
        );
    }
    println!("report-only: exit 0");
}
