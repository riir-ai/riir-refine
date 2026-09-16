//! Issue 64 Phase 2 — the pre-registered modelability ↔ downstream top-1
//! Spearman sweep (`riir-clippy/.issues/64_modelability_oracle_scaffold_value.md`).
//!
//! **The protocol (pre-registered in the issue — executed here, not restated):**
//! sweep ≥ 4 corpus/embedder variants on the retrieval surface; metric =
//! Spearman rank correlation between the oracle's `scaffold_value` and the
//! measured downstream top-1 across variants; gate ≥ 0.8 ⇒ the oracle
//! predicts where embedder investment pays; < 0.5 ⇒ kill (record negative,
//! keep the harness as the artifact).
//!
//! **Sweep-point construction (the wiring map's map-correction, recorded):**
//! the map proposed the `IndexTokenizer` ladder × domain surfaces. That axis
//! moves top-1 but CANNOT move the population — the BM25 tokenizer only
//! touches postings/queries, never the `ModellessEmbedder` embeddings the
//! oracle measures. Three tokenizer variants over one domain would be three
//! points with an identical x — tie-slotted by construction, incapable of
//! testing the oracle's actual claim (population shape ⇒ retrieval quality).
//! The corpus/embedder-variant axis the PROTOCOL names is implemented
//! instead, from exactly the levers the issue names ("corpus coverage,
//! chunking"):
//!
//! * **domain surfaces** — clippy / perf / kernel corpora (different rules,
//!   different latent populations, different top-1);
//! * **corpus slices** — deterministic even/odd halves of each domain's
//!   rule-id-sorted rule set (half the distractors → a different corpus AND
//!   a different measured top-1 over the slice's own labeled spans);
//! * **chunking** — multi-scale index (whole patterns + significant lines,
//!   the shipped shape) vs single-scale index (whole patterns only, the
//!   pre-multi-scale shape) — the issue's named "chunking" space lever.
//!
//! That is 12 population-distinct points with `kernel_heal` (8 without).
//!
//! **Population for the oracle:** the index chunk embeddings, replicated
//! exactly (`RuleIndex::build_with`'s chunk set: whole text + each
//! significant trimmed > 8-char line of multi-line texts), content-deduped
//! in first-occurrence order (the substrate upserts identical zone hashes —
//! duplicates never exist in the searched space), embedded by the same
//! `ModellessEmbedder`. The oracle measures the space the retriever actually
//! searches.
//!
//! **Verdict semantics:** the 0.8 / 0.5 gate is a ONE-TIME protocol verdict
//! over the corpus state at measurement time — recorded in
//! `.benchmarks/064_modelability_spearman_sweep.md` + the issue, not a
//! frozen CI pin (the corpus grows every batch; a pinned rho would rot
//! exactly like the stale floors Issue 024 re-pinned). This harness is the
//! standing re-measurement instrument. What IS asserted: non-vacuity
//! (point count, per-point span floor, value spread on both axes) and
//! oracle determinism (bit-identical re-run).
//!
//! **Tier:** diagnostic only — never gates a heal decision (the issue's
//! constraint; the L4 relevance-gate lesson).
//!
//! Run: `cargo test --features latent_retrieval,kernel_heal --test
//! modelability_sweep -- --nocapture` (drop `kernel_heal` for the 8-point
//! surface).

#![cfg(feature = "latent_retrieval")]

use std::collections::HashSet;

use riir_clippy::domain::{HealDomain, HealEntry};
use riir_clippy::domains::clippy_lints::ClippyLintDomain;
#[cfg(feature = "kernel_heal")]
use riir_clippy::domains::kernel_opt::corpus::build_kernel_seed_corpus;
#[cfg(feature = "kernel_heal")]
use riir_clippy::domains::kernel_opt::KernelOptDomain;
use riir_clippy::domains::rust_perf::RustPerfDomain;
use riir_clippy::draft::eval_fixtures_clippy::CLIPPY_SPANS;
use riir_clippy::draft::eval_fixtures_perf::PERF_SPANS;
#[cfg(feature = "kernel_heal")]
use riir_clippy::draft::retrieval_eval::mutate_shape;
use riir_clippy::draft::retrieval_eval::{canonical_rank, run_eval_with};
use riir_clippy::draft::{DomainId, IndexTokenizer, RetrievalConfig, RuleIndex};
use riir_clippy::modelability::{
    rank_correlation, scaffold_value, ModelabilityConfig, ModelabilityReport,
};
use riir_rag::{Embedder, ModellessEmbedder};

/// Ranked-list depth (mirrors the eval's `PROBE_TOP_K`; top-1 only needs 1,
/// the extra depth keeps the measurement shaped like the eval's).
const PROBE_K: usize = 8;

/// Minimum in-slice labeled spans for a point to count (a slice with < 5
/// measurable spans is a vacuous point — assert, don't score).
const MIN_SPANS_PER_POINT: usize = 5;

// ────────────────────────────────────────────────────────────────────────────
// Domain data
// ────────────────────────────────────────────────────────────────────────────

struct DomainData {
    name: &'static str,
    domain: DomainId,
    /// `(rule_id, buggy_code)` — exactly what `RuleIndex::build_with` gets.
    patterns: Vec<(String, String)>,
    /// `(label, code)` eval spans.
    spans: Vec<(String, String)>,
}

fn clippy_data() -> DomainData {
    DomainData {
        name: "clippy",
        domain: DomainId::Clippy,
        patterns: ClippyLintDomain::build_corpus()
            .into_iter()
            .map(|e| (e.rule_id().to_string(), e.buggy_code().to_string()))
            .collect(),
        spans: CLIPPY_SPANS
            .iter()
            .map(|(l, c)| ((*l).to_string(), (*c).to_string()))
            .collect(),
    }
}

fn perf_data() -> DomainData {
    DomainData {
        name: "perf",
        domain: DomainId::Perf,
        patterns: RustPerfDomain::build_corpus()
            .into_iter()
            .map(|e| (e.rule_id().to_string(), e.buggy_code().to_string()))
            .collect(),
        spans: PERF_SPANS
            .iter()
            .map(|(l, c)| ((*l).to_string(), (*c).to_string()))
            .collect(),
    }
}

#[cfg(feature = "kernel_heal")]
fn kernel_data() -> DomainData {
    DomainData {
        name: "kernel",
        domain: DomainId::Kernel,
        patterns: KernelOptDomain::build_corpus()
            .into_iter()
            .map(|e| (e.rule_id().to_string(), e.buggy_code().to_string()))
            .collect(),
        spans: {
            // Mirrors retrieval_eval::kernel_fixture_spans(30), with the
            // stride forced ODD: an even stride samples only even sorted
            // positions, so the even/odd half-corpus slices (the slice-a /
            // slice-b points below) degenerate — slice-b held zero labeled
            // spans once the corpus passed 600 entries (607/30 = 20) and the
            // vacuity assert correctly refused the point.
            let entries = build_kernel_seed_corpus().entries;
            let step = (entries.len() / 30).max(1) | 1;
            entries
                .iter()
                .step_by(step)
                .take(30)
                .map(|e| (e.rule_id.clone(), mutate_shape(&e.ref_shape)))
                .collect()
        },
    }
}

fn all_domains() -> Vec<DomainData> {
    #[cfg_attr(not(feature = "kernel_heal"), allow(unused_mut))]
    let mut v = vec![clippy_data(), perf_data()];
    #[cfg(feature = "kernel_heal")]
    v.push(kernel_data());
    v
}

// ────────────────────────────────────────────────────────────────────────────
// Population + measurement
// ────────────────────────────────────────────────────────────────────────────

/// The exact chunk set `RuleIndex::build_with` indexes: whole text + each
/// significant (trimmed > 8 char) line of multi-line texts — content-deduped
/// in first-occurrence order.
fn chunk_texts(patterns: &[(String, String)], multi_scale: bool) -> Vec<String> {
    let mut texts: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut push = |t: &str| {
        if seen.insert(t.to_string()) {
            texts.push(t.to_string());
        }
    };
    for (_, text) in patterns {
        push(text);
        if multi_scale && text.lines().count() > 1 {
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.len() > 8 {
                    push(trimmed);
                }
            }
        }
    }
    texts
}

/// The oracle population: one `ModellessEmbedder` embedding per chunk.
fn population(texts: &[String]) -> Vec<Vec<f32>> {
    let embedder = ModellessEmbedder;
    texts.iter().map(|t| embedder.embed(t).to_vec()).collect()
}

/// Deterministic even/odd half of the rule-id-sorted rule set.
fn slice(patterns: &[(String, String)], even: bool) -> Vec<(String, String)> {
    let mut sorted: Vec<(String, String)> = patterns.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    sorted
        .into_iter()
        .enumerate()
        .filter(|(i, _)| (i % 2 == 0) == even)
        .map(|(_, p)| p)
        .collect()
}

/// `DomainId::ALL` order (clippy, perf, kernel, sec, docker, dist,
/// shader) — mirrors `retrieval_eval::domain_index` for the eval-report
/// rows used here (shader sits at index 6, matching the eval's arm).
fn eval_domain_index(d: DomainId) -> usize {
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

struct Point {
    name: String,
    oracle: ModelabilityReport,
    top1: f64,
    n_chunks: usize,
    n_spans: usize,
}

/// One sweep point: oracle over the point's chunk population + measured
/// top-1 over the point's in-corpus labeled spans (a span whose label is
/// absent from the point's rule set cannot hit and is excluded from the
/// denominator — the slice protocol).
fn sweep_point(
    name: &str,
    patterns: &[(String, String)],
    spans: &[(String, String)],
    multi_scale: bool,
) -> Point {
    let texts = chunk_texts(patterns, multi_scale);
    let pop = population(&texts);
    let oracle = scaffold_value(&pop, &ModelabilityConfig::default());

    let labels: HashSet<&str> = patterns.iter().map(|(id, _)| id.as_str()).collect();
    let index = RuleIndex::build_with(patterns.to_vec(), RetrievalConfig::default())
        .unwrap_or_else(|e| panic!("{name}: index build failed: {e}"));

    let mut hits = 0usize;
    let mut n = 0usize;
    for (label, code) in spans {
        if !labels.contains(label.as_str()) {
            continue;
        }
        n += 1;
        let suggested = index.suggest(code, PROBE_K);
        let ranked = canonical_rank(&suggested);
        if ranked.first().is_some_and(|r| r.rule_id == *label) {
            hits += 1;
        }
    }
    assert!(
        n >= MIN_SPANS_PER_POINT,
        "{name}: only {n} in-corpus labeled spans — vacuous point"
    );

    Point {
        name: name.to_string(),
        oracle,
        top1: hits as f64 / n as f64,
        n_chunks: texts.len(),
        n_spans: n,
    }
}

#[allow(clippy::cast_precision_loss)] // display fractions over tiny counts
fn print_table(points: &[Point]) {
    println!(
        "{:<16} {:>7} {:>6} {:>9} {:>9} {:>9} {:>8}",
        "point", "chunks", "spans", "v_raw", "null", "value", "top-1"
    );
    for p in points {
        println!(
            "{:<16} {:>7} {:>6} {:>9.3} {:>9.3} {:>9.3} {:>7.1}%",
            p.name,
            p.n_chunks,
            p.n_spans,
            p.oracle.value_raw,
            p.oracle.null_value,
            p.oracle.value,
            p.top1 * 100.0
        );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The pre-registered sweep
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn modelability_tracks_downstream_top1() {
    let domains = all_domains();
    let mut points: Vec<Point> = Vec::new();

    for d in &domains {
        points.push(sweep_point(
            &format!("{}/full-ms", d.name),
            &d.patterns,
            &d.spans,
            true,
        ));
        points.push(sweep_point(
            &format!("{}/slice-a", d.name),
            &slice(&d.patterns, true),
            &d.spans,
            true,
        ));
        points.push(sweep_point(
            &format!("{}/slice-b", d.name),
            &slice(&d.patterns, false),
            &d.spans,
            true,
        ));
        points.push(sweep_point(
            &format!("{}/full-ss", d.name),
            &d.patterns,
            &d.spans,
            false,
        ));
    }

    let expected_points = if cfg!(feature = "kernel_heal") { 12 } else { 8 };
    assert_eq!(points.len(), expected_points, "sweep point count");

    // Non-vacuity: both axes must actually vary — a constant x or y makes
    // any correlation unmeasurable, not zero.
    let xs: Vec<f64> = points.iter().map(|p| p.oracle.value).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.top1).collect();
    assert!(
        xs.iter().any(|x| *x != xs[0]),
        "oracle values are all identical — no ordering to correlate"
    );
    assert!(
        ys.iter().any(|y| *y != ys[0]),
        "top-1 values are all identical — no ordering to correlate"
    );

    // Oracle determinism: bit-identical re-run — rebuild the first domain's
    // multi-scale population from scratch (the population fn is a pure
    // function of the corpus texts, so the rebuild IS the same bits).
    let first = &domains[0];
    let rep2 = scaffold_value(
        &population(&chunk_texts(&first.patterns, true)),
        &ModelabilityConfig::default(),
    );
    let rep1 = points[0].oracle;
    assert_eq!(rep1, rep2, "oracle must be deterministic");

    println!("== Issue 64 Phase 2 — modelability vs downstream top-1 ==");
    println!("protocol: pre-registered Spearman gate >= 0.8 PASS / < 0.5 kill (verdict recorded, not CI-pinned)");
    print_table(&points);

    let rho = rank_correlation(&xs, &ys).expect("non-constant by the asserts above");
    assert!(rho.is_finite() && (-1.0..=1.0).contains(&rho));

    let verdict = if rho >= 0.8 {
        "PASS — the oracle predicts where embedder investment pays"
    } else if rho < 0.5 {
        "NEGATIVE (kill branch) — oracle is noise on these populations; harness kept as the artifact"
    } else {
        "GRAY ZONE (0.5–0.8) — neither pre-registered branch fired"
    };
    println!(
        "Spearman rho(oracle value, top-1) = {rho:.4} over {} points — {verdict}",
        points.len()
    );

    // Supplementary: the domain axis alone (full-ms points) — the coarsest
    // population contrast. n = 3–5, so this is indicative, not decisive.
    let dom: Vec<&Point> = points
        .iter()
        .filter(|p| p.name.ends_with("/full-ms"))
        .collect();
    let dxs: Vec<f64> = dom.iter().map(|p| p.oracle.value).collect();
    let dys: Vec<f64> = dom.iter().map(|p| p.top1).collect();
    if let Some(drho) = rank_correlation(&dxs, &dys) {
        println!(
            "domain-axis-only rho = {drho:.4} over {} full-corpus points (indicative, tiny n)",
            dom.len()
        );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The wiring map's literal axis, reported as the map-correction evidence
// ────────────────────────────────────────────────────────────────────────────

/// The map proposed (domain × tokenizer) points. Executed here for the
/// record: top-1 moves across tokenizers (the measured T3.5 spread), the
/// oracle value CANNOT (the population never changes) — the tie structure
/// that makes the axis incapable of testing the oracle. Cross-checks the
/// recorded T3.5 floors as a side effect.
#[test]
fn tokenizer_axis_cannot_move_the_population() {
    let domains = all_domains();

    // Population-axis oracle value per domain (computed ONCE — identical
    // across every tokenizer row by construction).
    let oracle: Vec<(&str, f64)> = domains
        .iter()
        .map(|d| {
            let texts = chunk_texts(&d.patterns, true);
            let rep = scaffold_value(&population(&texts), &ModelabilityConfig::default());
            (d.name, rep.value)
        })
        .collect();

    let tokenizers = [
        ("code-default", IndexTokenizer::CodeDefault),
        ("op-aware", IndexTokenizer::OpAware),
        ("op-bigram", IndexTokenizer::OpAwareBigram),
    ];

    println!("== wiring-map literal axis: (domain x tokenizer) ==");
    println!(
        "{:<12} {:<12} {:>9} {:>9}",
        "domain", "tokenizer", "oracle", "top-1"
    );

    let mut pts: Vec<(f64, f64)> = Vec::new();
    for (tname, tok) in tokenizers {
        let report = run_eval_with(RetrievalConfig {
            tokenizer: tok,
            ..RetrievalConfig::default()
        });
        for (i, d) in domains.iter().enumerate() {
            let m = &report.per_domain[eval_domain_index(d.domain)];
            let top1 = f64::from(m.recall(1).unwrap_or_else(|| {
                panic!(
                    "{} domain produced no eval metrics — feature set wrong",
                    d.name
                )
            }));
            println!(
                "{:<12} {:<12} {:>9.3} {:>8.1}%",
                d.name,
                tname,
                oracle[i].1,
                top1 * 100.0
            );
            pts.push((oracle[i].1, top1));
        }
    }

    let xs: Vec<f64> = pts.iter().map(|(x, _)| *x).collect();
    let ys: Vec<f64> = pts.iter().map(|(_, y)| *y).collect();
    match rank_correlation(&xs, &ys) {
        Some(rho) => println!(
            "tie-corrected rho over the literal axis = {rho:.4} — x is tie-slotted by \
             construction (3 tokenizers share one population per domain); the axis \
             measures the tokenizer, not the space"
        ),
        None => println!("literal-axis rho undefined (no x spread) — as predicted"),
    }
}
