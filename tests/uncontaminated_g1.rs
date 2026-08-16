//! Plan 005 T3.5.5 — the uncontaminated G1 measurement.
//!
//! T3.5.4's re-ranker looked like a +25pp win and was not one: the clippy
//! fixtures are label-verified by the same proposers the re-ranker consults
//! (`clippy_fixture_labels_verify_via_proposers` asserts own-fires +
//! zero-cross-fires), and `rerank_circularity_diagnostic` showed
//! `RerankMode::All` top-1 == pool recall exactly. The fixture set cannot
//! measure a re-ranker.
//!
//! This harness re-measures against real `cargo clippy`, which knows nothing
//! about our corpus, proposers, or scores.
//!
//! ```bash
//! cargo test --release --features latent_retrieval,clippy_verify \
//!   --test uncontaminated_g1 -- --nocapture --test-threads=1
//! ```
//!
//! Slow: one `cargo clippy` per distinct span (BLAKE3-keyed temp crates, so
//! re-runs hit cargo's build cache).
//!
//! Deliberately NOT a committed regression floor. Clippy's lint set moves
//! between toolchains, so pinning a number here would red the suite on every
//! `rustup update`. It is a measurement you run when you change ranking.

#![cfg(all(feature = "latent_retrieval", feature = "clippy_verify"))]

use std::collections::{BTreeMap, BTreeSet};

use riir_clippy::domain::{HealDomain, HealEntry};
use riir_clippy::draft::{ClippyOracle, DomainId, LatentMatcher, RerankMode, RERANK_POOL};

/// Corpus lint vocabulary — real clippy reports far more than we have rules
/// for, and scoring against unknown lints would measure corpus coverage
/// rather than ranking quality.
fn vocabulary() -> BTreeSet<String> {
    riir_clippy::domains::clippy_lints::ClippyLintDomain::build_corpus()
        .into_iter()
        .map(|e| e.rule_id().to_string())
        .collect()
}

/// One span's ground truth, as real clippy sees it.
struct Truth {
    our_label: &'static str,
    code: &'static str,
    compiles: bool,
    /// Real clippy lints intersected with the corpus vocabulary.
    real: BTreeSet<String>,
    /// First hard error when the span does not compile.
    error: Option<String>,
}

impl Truth {
    fn error_hint(&self) -> &str {
        self.error.as_deref().unwrap_or("(no error captured)")
    }
}

fn gather_truth() -> Vec<Truth> {
    gather_truth_over(riir_clippy::draft::eval_fixtures_clippy::CLIPPY_SPANS)
}

fn gather_truth_over(spans: &'static [(&'static str, &'static str)]) -> Vec<Truth> {
    let oracle = ClippyOracle::new();
    let vocab = vocabulary();
    let mut out = Vec::with_capacity(spans.len());
    for (i, (label, code)) in spans.iter().enumerate() {
        let v = oracle.lints_for(code);
        assert!(v.ran, "cargo clippy did not run: {:?}", v.error);
        if (i + 1) % 12 == 0 {
            println!("  ... labeled {}/{}", i + 1, spans.len());
        }
        out.push(Truth {
            our_label: label,
            code,
            compiles: v.compiles,
            real: v.within_vocabulary(&vocab),
            error: v.error.clone(),
        });
    }
    out
}

#[test]
fn t355_oracle_agreement_report() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    println!("\n=== T3.5.5 — real-clippy oracle vs the proposer oracle ===");
    let truth = gather_truth();

    let n = truth.len();
    let compiling: Vec<&Truth> = truth.iter().filter(|t| t.compiles).collect();
    let agree = compiling.iter().filter(|t| t.real.contains(t.our_label)).count();
    let multi = compiling.iter().filter(|t| t.real.len() > 1).count();
    let empty = compiling.iter().filter(|t| t.real.is_empty()).count();
    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize, d: usize| if d == 0 { 0.0 } else { k as f32 / d as f32 * 100.0 };

    println!("  fixtures                     : {n}");
    println!(
        "  compile under rustc          : {} ({:.1}%)",
        compiling.len(),
        pct(compiling.len(), n)
    );
    println!(
        "  our label confirmed by clippy: {agree}/{} ({:.1}%)",
        compiling.len(),
        pct(agree, compiling.len())
    );
    println!(
        "  real clippy reports NO corpus lint: {empty}/{} ({:.1}%)",
        compiling.len(),
        pct(empty, compiling.len())
    );
    println!(
        "  MULTI-lint spans (>1 corpus lint) : {multi}/{} ({:.1}%)  <- the property the proposer oracle asserts is ZERO",
        compiling.len(),
        pct(multi, compiling.len())
    );

    println!("\n  -- disagreements (our label not in real clippy's set) --");
    let mut shown = 0;
    for t in &truth {
        if !t.compiles {
            println!("  UNCOMPILABLE  {}", t.our_label);
            shown += 1;
        } else if !t.real.contains(t.our_label) {
            let got: Vec<&str> = t.real.iter().map(String::as_str).collect();
            println!("  MISLABELED    {} -> real: {got:?}", t.our_label);
            shown += 1;
        }
        if shown >= 25 {
            println!("  ... (truncated)");
            break;
        }
    }
    if shown == 0 {
        println!("  (none)");
    }
}

/// The measurement T3.5.4 could not make: G1 scored against real clippy.
#[test]
fn t355_uncontaminated_rerank_g1() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    println!("\n=== T3.5.5 — uncontaminated re-rank G1 ===");
    let truth = gather_truth();

    // Only spans that COMPILE and carry at least one corpus lint are scorable.
    // An uncompilable span has no trustworthy labels; a span with no corpus
    // lint has no correct answer to rank first.
    let scorable: Vec<&Truth> = truth
        .iter()
        .filter(|t| t.compiles && !t.real.is_empty())
        .collect();
    println!("  scorable spans: {} of {}", scorable.len(), truth.len());
    if scorable.is_empty() {
        println!("  NOTHING SCORABLE — see the agreement report for why.");
        return;
    }

    let modes = [RerankMode::Off, RerankMode::Structural, RerankMode::All];
    let mut results: BTreeMap<&str, (usize, usize)> = BTreeMap::new();

    for mode in modes {
        let tag = match mode {
            RerankMode::Off => "off",
            RerankMode::Structural => "structural",
            RerankMode::All => "all",
        };
        let mut matcher = LatentMatcher::from_enabled_domains()
            .expect("build matcher")
            .with_rerank_mode(mode);

        let (mut hit1, mut hit3) = (0usize, 0usize);
        for t in &scorable {
            let (_, by_domain) = matcher.suggest_span(t.code, RERANK_POOL);
            let ranked: Vec<&str> = by_domain
                .iter()
                .find(|(d, _)| *d == DomainId::Clippy)
                .map_or(Vec::new(), |(_, rules)| {
                    rules.iter().map(|r| r.rule_id.as_str()).collect()
                });
            // Correct = the top-ranked rule is one real clippy actually
            // reports for this span. With multi-lint spans, ANY true lint
            // counts — the task is "surface something that applies", not
            // "guess which of several we happened to write down".
            if ranked.first().is_some_and(|r| t.real.contains(*r)) {
                hit1 += 1;
            }
            if ranked.iter().take(3).any(|r| t.real.contains(*r)) {
                hit3 += 1;
            }
        }
        results.insert(tag, (hit1, hit3));
    }

    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize| k as f32 / scorable.len() as f32 * 100.0;
    println!("\n{:<12} {:>10} {:>10}", "mode", "top-1", "top-3");
    for (tag, (h1, h3)) in &results {
        println!("{tag:<12} {:>9.1}% {:>9.1}%", pct(*h1), pct(*h3));
    }

    let off = results["off"].0;
    let structural = results["structural"].0;
    let all = results["all"].0;
    println!(
        "\n  structural vs off: {:+.1}pp    all vs off: {:+.1}pp",
        pct(structural) - pct(off),
        pct(all) - pct(off)
    );

    // The contamination check, re-run against the real oracle: if the
    // re-ranker is still just reproducing pool recall, the gain is still not
    // an effect size.
    let mut matcher = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(RerankMode::Off);
    let in_pool = scorable
        .iter()
        .filter(|t| {
            let (_, by_domain) = matcher.suggest_span(t.code, RERANK_POOL);
            by_domain
                .iter()
                .find(|(d, _)| *d == DomainId::Clippy)
                .is_some_and(|(_, rules)| rules.iter().any(|r| t.real.contains(&r.rule_id)))
        })
        .count();
    println!(
        "  pool recall @{RERANK_POOL}: {:.1}% ({in_pool}/{})",
        pct(in_pool),
        scorable.len()
    );
    println!(
        "  contamination check: all-mode top-1 {} pool recall — {}",
        if all == in_pool { "==" } else { "!=" },
        if all == in_pool {
            "STILL DEGENERATE (the multi-lint spans did not break the tie)"
        } else {
            "genuinely re-ranking (not explained by retrieval alone)"
        }
    );
}

/// The measurement the single-lint fixtures structurally cannot make.
///
/// On a span where several rules genuinely apply, "which surfaces first" is a
/// real ordering decision — multiple proposers fire, so top-1 is no longer
/// decided by whether the one true rule was retrieved. If `all`-mode top-1
/// still equals pool recall here, the re-ranker is degenerate for a deeper
/// reason than fixture shape.
#[test]
fn t355_composite_multi_lint_g1() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    println!("\n=== T3.5.5 — composite multi-lint G1 (the discriminating set) ===");
    let truth = gather_truth_over(riir_clippy::draft::eval_fixtures_composite::COMPOSITE_SPANS);

    let compiling: Vec<&Truth> = truth.iter().filter(|t| t.compiles).collect();
    let multi: Vec<&&Truth> = compiling.iter().filter(|t| t.real.len() > 1).collect();
    #[allow(clippy::cast_precision_loss)]
    let pc = |k: usize, d: usize| if d == 0 { 0.0 } else { k as f32 / d as f32 * 100.0 };
    println!(
        "  authored {} | compile {} ({:.0}%) | MULTI-lint {} ({:.0}%)",
        truth.len(),
        compiling.len(),
        pc(compiling.len(), truth.len()),
        multi.len(),
        pc(multi.len(), compiling.len())
    );
    for t in &truth {
        if !t.compiles {
            println!("  UNCOMPILABLE {}: {}", t.our_label, t.error_hint());
        }
    }
    println!("\n  -- real clippy label sets --");
    for t in &compiling {
        let got: Vec<&str> = t.real.iter().map(String::as_str).collect();
        println!("  {:<40} {got:?}", t.our_label);
    }

    // Score ONLY the genuinely multi-lint spans — that is the whole point.
    let scorable: Vec<&&Truth> = multi;
    if scorable.len() < 5 {
        println!(
            "\n  INSUFFICIENT: only {} multi-lint spans — not enough to conclude anything.",
            scorable.len()
        );
        return;
    }

    let mut results: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for mode in [RerankMode::Off, RerankMode::Structural, RerankMode::All] {
        let tag = match mode {
            RerankMode::Off => "off",
            RerankMode::Structural => "structural",
            RerankMode::All => "all",
        };
        let mut matcher = LatentMatcher::from_enabled_domains()
            .expect("build matcher")
            .with_rerank_mode(mode);
        let (mut hit1, mut hit3) = (0usize, 0usize);
        for t in &scorable {
            let (_, by_domain) = matcher.suggest_span(t.code, RERANK_POOL);
            let ranked: Vec<&str> = by_domain
                .iter()
                .find(|(d, _)| *d == DomainId::Clippy)
                .map_or(Vec::new(), |(_, rules)| {
                    rules.iter().map(|r| r.rule_id.as_str()).collect()
                });
            if ranked.first().is_some_and(|r| t.real.contains(*r)) {
                hit1 += 1;
            }
            if ranked.iter().take(3).any(|r| t.real.contains(*r)) {
                hit3 += 1;
            }
        }
        results.insert(tag, (hit1, hit3));
    }

    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize| k as f32 / scorable.len() as f32 * 100.0;
    println!("\n  scored on {} multi-lint spans", scorable.len());
    println!("{:<12} {:>10} {:>10}", "mode", "top-1", "top-3");
    for (tag, (h1, h3)) in &results {
        println!("{tag:<12} {:>9.1}% {:>9.1}%", pct(*h1), pct(*h3));
    }

    let (off, structural, all) = (results["off"].0, results["structural"].0, results["all"].0);
    println!(
        "\n  structural vs off: {:+.1}pp    all vs off: {:+.1}pp",
        pct(structural) - pct(off),
        pct(all) - pct(off)
    );

    let mut matcher = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(RerankMode::Off);
    let in_pool = scorable
        .iter()
        .filter(|t| {
            let (_, by_domain) = matcher.suggest_span(t.code, RERANK_POOL);
            by_domain
                .iter()
                .find(|(d, _)| *d == DomainId::Clippy)
                .is_some_and(|(_, rules)| rules.iter().any(|r| t.real.contains(&r.rule_id)))
        })
        .count();
    println!("  pool recall @{RERANK_POOL}: {:.1}% ({in_pool}/{})", pct(in_pool), scorable.len());
    println!(
        "  degeneracy check: all-mode top-1 {} pool recall -> {}",
        if all == in_pool { "==" } else { "!=" },
        if all == in_pool {
            "STILL DEGENERATE"
        } else {
            "GENUINE RE-RANKING (top-1 is an ordering decision, not a retrieval one)"
        }
    );
}

/// G2 — what does the re-rank cost per span?
///
/// One `syn` parse plus K visitor walks per domain. Issue 023 measured a bare
/// parse at ~19-23 us and the retrieval pipeline at ~349 us/chunk, so the
/// budget question is whether the probe is a rounding error or a second
/// pipeline.
#[test]
fn t355_rerank_latency_g2() {
    use std::time::Instant;
    let spans = riir_clippy::draft::eval_fixtures_composite::COMPOSITE_SPANS;
    let mut timings = Vec::new();
    for mode in [RerankMode::Off, RerankMode::Structural] {
        let mut matcher = LatentMatcher::from_enabled_domains()
            .expect("build matcher")
            .with_rerank_mode(mode);
        // Warm the index/caches so the first span does not skew the median.
        for (_, code) in spans.iter().take(3) {
            let _ = matcher.suggest_span(code, RERANK_POOL);
        }
        let reps = 20;
        let t0 = Instant::now();
        for _ in 0..reps {
            for (_, code) in spans {
                let out = matcher.suggest_span(code, RERANK_POOL);
                std::hint::black_box(&out);
            }
        }
        let per_span = t0.elapsed().as_secs_f64() / f64::from(reps) / spans.len() as f64;
        timings.push((mode, per_span * 1e6));
    }
    println!("\n=== T3.5.5 G2 — re-rank latency ===");
    for (mode, us) in &timings {
        println!("  {:<12} {us:>8.1} us/span", format!("{mode:?}"));
    }
    let overhead = timings[1].1 - timings[0].1;
    println!("  overhead: {overhead:+.1} us/span ({:+.1}%)", overhead / timings[0].1 * 100.0);
}
