//! Issue 169 T0 — the FLINT multi-vector kill-test harness (measurement-only).
//!
//! Arms over the oracle `retrieval_eval` fixture battery, all in ONE binary at
//! the SAME feature state (fair population):
//!
//! - **(a)** the shipped `LatentMatcher` (8-D latent + BM25 + instantiation
//!   rerank) — run twice: once via `retrieval_eval::run_eval` (the official
//!   report) and once through this file's paired loop; the two MUST agree
//!   exactly (a harness-validity assert — divergence invalidates the record).
//! - **(e)** full-token MaxSim over hashed op-token embeddings —
//!   `D ∈ {8, 32, 64}` × `Integration ∈ {Raw, Full}` × `QueryMode ∈ {All,
//!   Unique}` (see `riir_refine::draft::multivector`).
//!
//! Kill rule (Issue 169 T0): if NO (e) arm beats (a)'s blended top-1 at the
//! widest D, the negative is the answer — record it, STOP, T1 is never built.
//! The paired wins/losses table at D=64 is the decisiveness read; the formal
//! paired LB95 machinery is T3's.
//!
//! Run: `cargo run --release --features multivector_poc --example multivector_poc`

use std::time::Instant;

use riir_refine::draft::latent_matcher::LatentMatcher;
use riir_refine::draft::multivector::{run_arm, Integration, MultivectorIndex, QueryMode};
use riir_refine::draft::retrieval_eval::{
    canonical_rank, labeled_eval_spans, run_eval, EvalReport,
};
use riir_refine::draft::DomainId;

/// Probe depth — mirrors `retrieval_eval::PROBE_TOP_K` (private there;
/// duplicated deliberately, one line, the linking_confusion precedent).
const PROBE_TOP_K: usize = 8;

/// Mirrors `retrieval_eval::domain_index` (pub(crate) there — invisible to an
/// example; duplicated deliberately, one match, the PROBE_TOP_K precedent).
fn domain_index(d: DomainId) -> usize {
    match d {
        DomainId::Clippy => 0,
        DomainId::Perf => 1,
        DomainId::Kernel => 2,
        DomainId::Sec => 3,
        DomainId::Docker => 4,
        DomainId::Dist => 5,
        DomainId::ShaderPerf => 6,
    }
}

fn domain_label(d: DomainId) -> &'static str {
    match d {
        DomainId::Clippy => "clippy",
        DomainId::Perf => "perf",
        DomainId::Kernel => "kernel",
        DomainId::Sec => "sec",
        DomainId::Docker => "docker",
        DomainId::Dist => "dist",
        DomainId::ShaderPerf => "shader",
    }
}

struct PairedRow {
    label: String,
    domain: DomainId,
    rank: Option<usize>,
}

/// Arm (a) paired loop — mirrors `run_eval_on`'s measurement core (suggest →
/// own-domain canonical rank), records per-span rows for the pairing.
fn arm_a_paired(matcher: &mut LatentMatcher) -> Vec<PairedRow> {
    let mut rows = Vec::new();
    for span in labeled_eval_spans() {
        if !matcher.has_domain(span.domain) {
            continue;
        }
        let (_, by_domain) = matcher.suggest_span(&span.code, PROBE_TOP_K);
        let ranked = by_domain
            .iter()
            .find(|(d, _)| *d == span.domain)
            .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));
        let rank = ranked.iter().position(|r| r.rule_id == span.label);
        rows.push(PairedRow {
            label: span.label.clone(),
            domain: span.domain,
            rank,
        });
    }
    rows
}

fn rate(rows: &[PairedRow], domain: Option<DomainId>, k: usize) -> (usize, usize) {
    let mut hit = 0usize;
    let mut n = 0usize;
    for r in rows {
        if let Some(d) = domain {
            if r.domain != d {
                continue;
            }
        }
        n += 1;
        if r.rank.is_some_and(|rank| rank < k) {
            hit += 1;
        }
    }
    (hit, n)
}

fn pct(hit: usize, n: usize) -> String {
    if n == 0 {
        "n/a".to_string()
    } else {
        format!("{hit}/{n} ({:.1}%)", 100.0 * hit as f64 / n as f64)
    }
}

fn main() {
    println!("== Issue 169 T0 — full-token MaxSim kill-test ==");
    println!(
        "profile: {} (timing is load-disclosed context, NOT a gate — G2 is T5)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );

    // ── arm (a): the shipped matcher ─────────────────────────────────────
    let t0 = Instant::now();
    let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
    let build_a = t0.elapsed();
    // The rule corpus in the SAME enumeration arm (a) indexed — never a
    // second corpus source that could drift from it.
    let pairs: Vec<(String, DomainId, String)> = matcher
        .rule_book_map()
        .iter()
        .map(|(id, info)| (id.clone(), info.domain, info.buggy_code.clone()))
        .collect();
    let mut per_domain_counts: Vec<(DomainId, usize)> = Vec::new();
    for (_, d, _) in &pairs {
        match per_domain_counts.iter_mut().find(|(sd, _)| sd == d) {
            Some((_, n)) => *n += 1,
            None => per_domain_counts.push((*d, 1)),
        }
    }
    println!(
        "arm (a) corpus: {} rules across {} domains, built in {:?} — {}",
        pairs.len(),
        per_domain_counts.len(),
        build_a,
        per_domain_counts
            .iter()
            .map(|(d, n)| format!("{}={n}", domain_label(*d)))
            .collect::<Vec<_>>()
            .join(" ")
    );

    let official: EvalReport = run_eval();
    let t0 = Instant::now();
    let rows_a = arm_a_paired(&mut matcher);
    let score_a = t0.elapsed();
    // Harness validity: the paired loop MUST reproduce the official report
    // exactly (blended + per-domain) — divergence invalidates this record.
    let (a1, an) = rate(&rows_a, None, 1);
    let (a3, _) = rate(&rows_a, None, 3);
    let (a5, _) = rate(&rows_a, None, 5);
    let ob = &official.blended;
    assert_eq!(
        (a1, a3, a5, an),
        (ob.hits_at_1, ob.hits_at_3, ob.hits_at_5, ob.labeled),
        "harness defect: arm (a) paired loop != run_eval official report"
    );
    for (d, _) in &per_domain_counts {
        let m = &official.per_domain[domain_index(*d)];
        let (h, n) = rate(&rows_a, Some(*d), 1);
        assert_eq!(
            (h, n),
            (m.hits_at_1, m.labeled),
            "harness defect: per-domain drift at {}",
            domain_label(*d)
        );
    }
    println!("arm (a) VALIDATED: paired loop == run_eval (blended + per-domain exact)");
    println!(
        "arm (a) blended: top1 {} top3 {} top5 {} — scored in {:?} ({:.0} µs/span)",
        pct(a1, an),
        pct(a3, an),
        pct(a5, an),
        score_a,
        score_a.as_micros() as f64 / an.max(1) as f64
    );
    for (d, _) in &per_domain_counts {
        let (h1, n) = rate(&rows_a, Some(*d), 1);
        let (h3, _) = rate(&rows_a, Some(*d), 3);
        let (h5, _) = rate(&rows_a, Some(*d), 5);
        println!(
            "arm (a) {:<7} top1 {:<16} top3 {:<16} top5 {}",
            domain_label(*d),
            pct(h1, n),
            pct(h3, n),
            pct(h5, n)
        );
    }

    // ── arm (e): the width × posture × weighting sweep ───────────────────
    let d8 = sweep::<8>(&pairs, &rows_a, &official);
    let d32 = sweep::<32>(&pairs, &rows_a, &official);
    let d64 = sweep::<64>(&pairs, &rows_a, &official);

    // ── the kill rule, at the widest D ───────────────────────────────────
    println!("\n== kill rule (T0): best (e) blended top-1 at D=64 vs arm (a) ==");
    let (best_name, best_hit, best_n) = d64
        .iter()
        .max_by_key(|(_, hit, _)| *hit)
        .map_or(("none", 0, 0), |(name, hit, n)| (name.as_str(), *hit, *n));
    println!("arm (a) blended top-1: {}", pct(a1, an));
    println!(
        "arm (e) best [{best_name}] at D=64: {}",
        pct(best_hit, best_n)
    );
    for (width, sweep) in [("D=8", &d8), ("D=32", &d32), ("D=64", &d64)] {
        let (name, hit, n) = sweep
            .iter()
            .max_by_key(|(_, hit, _)| *hit)
            .map_or(("none", 0, 0), |(name, hit, n)| (name.as_str(), *hit, *n));
        println!("  best at {width}: [{name}] {} (arm a: {a1})", pct(hit, n));
    }
    if best_hit > a1 {
        println!(
            "VERDICT: (e) > (a) at the widest D — the prerequisite HOLDS; T1 pooling may be built (T3's paired LB95 still governs promotion)."
        );
    } else {
        println!(
            "VERDICT: (e) <= (a) at the widest D — KILL. Full-token MaxSim does not beat the shipped matcher on this task; record the negative, T1 is never built, riir-train 647 stays closed."
        );
    }
}

/// One width's full sweep. Returns blended top-1 (arm name, hits, labeled)
/// per posture — the kill-rule's comparison rows.
fn sweep<const D: usize>(
    pairs: &[(String, DomainId, String)],
    rows_a: &[PairedRow],
    official: &EvalReport,
) -> Vec<(String, usize, usize)> {
    let idx = MultivectorIndex::<D>::from_rules(pairs);
    println!(
        "\n== arm (e) D={D}: digest {} model_id {} ==",
        idx.corpus_digest, idx.model_id
    );
    let mut rows: Vec<(String, usize, usize)> = Vec::new();
    let mut paired_rep = None;
    for integration in [Integration::Full, Integration::Raw] {
        for mode in [QueryMode::All, QueryMode::Unique] {
            let iname = match integration {
                Integration::Full => "Full",
                Integration::Raw => "Raw",
            };
            let mname = match mode {
                QueryMode::All => "All",
                QueryMode::Unique => "Unique",
            };
            // The clean-excerpt profile rides the flagship posture only
            // (context for the record; not a gate).
            let with_clean = D == 64 && integration == Integration::Full && mode == QueryMode::All;
            let rep = run_arm(&idx, integration, mode, with_clean);
            let b = rep.blended;
            println!(
                "arm (e) D={D:<2} {iname}/{mname:<6} top1 {:<16} top3 {:<16} top5 {:<16} score {:>8.0} ms ({:>5.2} µs/pair, {} pairs) misses {}",
                pct(b.at_1, b.labeled),
                pct(b.at_3, b.labeled),
                pct(b.at_5, b.labeled),
                rep.score.as_millis(),
                rep.score.as_nanos() as f64 / 1000.0 / rep.pairs.max(1) as f64,
                rep.pairs,
                rep.misses.len(),
            );
            for dd in idx.domain_list() {
                let h = rep.domain_hits(dd);
                let ah = &official.per_domain[domain_index(dd)];
                println!(
                    "         {:<7} top1 {:<16} (arm a: {}/{}) delta {:+}",
                    domain_label(dd),
                    pct(h.at_1, h.labeled),
                    ah.hits_at_1,
                    ah.labeled,
                    h.at_1 as i64 - ah.hits_at_1 as i64,
                );
            }
            if with_clean {
                for (tag, tops) in &rep.clean_top1 {
                    let joined = tops
                        .iter()
                        .map(|(d, r)| format!("{}:{r}", domain_label(*d)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!("         clean [{tag}] {joined}");
                }
            }
            rows.push((format!("{iname}/{mname}"), b.at_1, b.labeled));
            if integration == Integration::Full && mode == QueryMode::All {
                paired_rep = Some(rep);
            }
        }
    }
    // The paired decisiveness read at this width — Full/All vs arm (a).
    if let Some(rep) = paired_rep {
        let mut wins = 0usize;
        let mut losses = 0usize;
        for (span, e_hit) in &rep.span_top1 {
            let Some(a_row) = rows_a.iter().find(|r| &r.label == span) else {
                panic!("harness defect: span {span} missing from arm (a) rows");
            };
            let a_hit = a_row.rank.is_some_and(|r| r < 1);
            match (*e_hit, a_hit) {
                (true, false) => wins += 1,
                (false, true) => losses += 1,
                _ => {}
            }
        }
        println!(
            "arm (e) D={D} Full/All PAIRED vs (a), blended top-1: +{wins} wins / −{losses} losses / {} ties (of {})",
            rep.span_top1.len() - wins - losses,
            rep.span_top1.len()
        );
    }
    rows
}
