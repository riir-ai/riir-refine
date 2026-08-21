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

// ── Issue 034 T1 — selection headroom on the uncontaminated surface ─────────
//
// The lib-side probe (`draft::headroom_probe`) measures headroom against OUR
// labels on single-lint fixtures — exactly the surface
// `rerank_circularity_diagnostic` showed is decided by retrieval, so a number
// read only there could be a fixture artifact. These tests re-measure with
// real `cargo clippy` as ground truth over spans where several corpus lints
// genuinely apply, i.e. where "which candidate wins" is an ordering decision.
//
// Two acceptance criteria are measured, because the lenient one saturates:
//
// - **any-true** (`t1_dflash2_composite_headroom`): top-1 counts if real
//   clippy reports it at all. This mirrors `t355_composite_multi_lint_g1`.
// - **fixable-true** (`t1_dflash2_composite_headroom_fixable`): top-1 counts
//   only if it is a true lint the `--fix` path can actually apply
//   (`FIXABLE_LINTS`). A true-but-unfixable top-1 makes `--fix` decline even
//   when a fixable true lint sits deeper in the pool — that is a real,
//   recoverable selection miss, and it is invisible to the lenient criterion.
//
// Numbers are printed, not pinned: clippy's lint set moves between toolchains
// (see the module note). Only structural invariants are asserted.

/// One surface's headroom measurement.
struct CompositeHeadroom {
    scored: usize,
    /// Spans where the production top-1 was accepted.
    chosen: usize,
    /// Per `HEADROOM_DEPTHS` entry: spans with an accepted rule in the pool.
    pool: Vec<usize>,
    /// `(wanted set, chosen rule, best accepted rank in the deepest pool)`.
    misses: Vec<(String, String, Option<usize>)>,
}

impl CompositeHeadroom {
    #[allow(clippy::cast_precision_loss)] // ≤28 spans; display arithmetic
    fn pct(&self, k: usize) -> f32 {
        if self.scored == 0 { 0.0 } else { k as f32 / self.scored as f32 * 100.0 }
    }
    fn ceiling(&self) -> usize {
        *self.pool.last().expect("depths non-empty")
    }
    fn headroom_pp(&self) -> f32 {
        self.pct(self.ceiling()) - self.pct(self.chosen)
    }
    fn selection_misses(&self) -> usize {
        self.misses.iter().filter(|(_, _, r)| r.is_some()).count()
    }
}

/// The clippy-domain ranked rule ids at pool depth `k`, in production order.
///
/// Owned ids: the matcher is borrowed mutably per query (shared G4 scratch),
/// so a borrowed view cannot outlive the next depth's call.
fn ranked_clippy_ids(matcher: &mut LatentMatcher, code: &str, k: usize) -> Vec<String> {
    let (_, by_domain) = matcher.suggest_span(code, k);
    by_domain
        .iter()
        .find(|(d, _)| *d == DomainId::Clippy)
        .map_or_else(Vec::new, |(_, rules)| {
            rules.iter().take(k).map(|r| r.rule_id.clone()).collect()
        })
}

/// Measure headroom over `(span, accepted-rule set)` pairs.
///
/// Each depth gets its OWN query: the matcher re-ranks *the pool*
/// (`max(top_k, RERANK_POOL)`), so depth is a configuration and a deeper pool
/// can promote a top-1 the shallower one never saw — it cannot be read off by
/// slicing a single deep list.
fn measure_composite_headroom(pairs: &[(&Truth, BTreeSet<String>)]) -> CompositeHeadroom {
    measure_composite_headroom_with(pairs, RerankMode::default())
}

/// [`measure_composite_headroom`] under an explicit re-rank mode — the
/// instrument-power control. Headroom is a property of the SELECTOR under
/// test, so the selector must be nameable.
fn measure_composite_headroom_with(
    pairs: &[(&Truth, BTreeSet<String>)],
    rerank: RerankMode,
) -> CompositeHeadroom {
    let depths = riir_clippy::draft::HEADROOM_DEPTHS;
    let mut matcher = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(rerank);

    let mut out = CompositeHeadroom {
        scored: pairs.len(),
        chosen: 0,
        pool: vec![0; depths.len()],
        misses: Vec::new(),
    };

    for (t, accepted) in pairs {
        let lists: Vec<Vec<String>> = depths
            .iter()
            .map(|&k| ranked_clippy_ids(&mut matcher, t.code, k))
            .collect();
        // Production selection is the RERANK_POOL-deep query, by definition.
        let chosen = ranked_clippy_ids(&mut matcher, t.code, RERANK_POOL)
            .first()
            .cloned();

        let correct = chosen.as_ref().is_some_and(|r| accepted.contains(r));
        if correct {
            out.chosen += 1;
        }
        for (i, list) in lists.iter().enumerate() {
            if list.iter().any(|r| accepted.contains(r)) {
                out.pool[i] += 1;
            }
        }
        if !correct {
            let deepest = lists.last().expect("depths non-empty");
            let rank = deepest.iter().position(|r| accepted.contains(r));
            let want: Vec<&str> = accepted.iter().map(String::as_str).collect();
            out.misses.push((
                format!("{want:?}"),
                chosen.unwrap_or_else(|| "(none)".to_string()),
                rank,
            ));
        }
    }
    out
}

fn print_composite_headroom(tag: &str, h: &CompositeHeadroom) {
    let depths = riir_clippy::draft::HEADROOM_DEPTHS;
    print!("\n  [{tag}] scored {} | recall@1 {:.1}%", h.scored, h.pct(h.chosen));
    for (i, &k) in depths.iter().enumerate() {
        print!("   pool@{k} {:.1}%", h.pct(h.pool[i]));
    }
    println!(
        "\n  [{tag}] headroom (pool@{} − recall@1): {:+.1}pp   ({} selection / {} retrieval misses)",
        depths[depths.len() - 1],
        h.headroom_pp(),
        h.selection_misses(),
        h.misses.len() - h.selection_misses(),
    );
    for (want, chosen, rank) in &h.misses {
        match rank {
            Some(r) => println!("    SELECTION  want={want} chosen={chosen} best_accepted_rank={r}"),
            None => println!("    RETRIEVAL  want={want} chosen={chosen} (nothing accepted in pool)"),
        }
    }
    let bar = riir_clippy::draft::HEADROOM_STOP_BAR_PP;
    println!(
        "  [{tag}] T1.2 stop bar {bar:.1}pp -> {}",
        if h.headroom_pp() < bar { "STOP on this criterion" } else { "PROCEED on this criterion" }
    );
    if h.chosen == h.scored {
        println!(
            "  [{tag}] NOTE: recall@1 is SATURATED at 100% — this criterion has zero \
             measurement power here; a 0.0pp reading is 'cannot see', not 'is not there'."
        );
    }
}

/// The multi-lint spans real clippy confirms, with their true corpus lints.
fn composite_scorable(truth: &[Truth]) -> Vec<&Truth> {
    truth.iter().filter(|t| t.compiles && t.real.len() > 1).collect()
}

fn structural_invariants(h: &CompositeHeadroom) {
    assert!(
        h.ceiling() >= h.chosen,
        "pool ceiling {} below recall@1 {} — the probe is comparing two different \
         rankings and the headroom number is meaningless",
        h.ceiling(),
        h.chosen
    );
    assert_eq!(
        h.misses.len(),
        h.scored - h.chosen,
        "miss taxonomy must cover exactly the non-chosen spans"
    );
}

/// T1, lenient criterion: any lint real clippy reports counts.
#[test]
fn t1_dflash2_composite_headroom() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    println!("\n=== Issue 034 T1 — headroom, real-clippy composite, ANY-TRUE ===");
    let truth = gather_truth_over(riir_clippy::draft::eval_fixtures_composite::COMPOSITE_SPANS);
    let scorable = composite_scorable(&truth);
    println!("  authored {} | scorable multi-lint {}", truth.len(), scorable.len());
    if scorable.len() < 5 {
        println!("  INSUFFICIENT: fewer than 5 multi-lint spans — no verdict.");
        return;
    }

    let pairs: Vec<(&Truth, BTreeSet<String>)> =
        scorable.iter().map(|t| (*t, t.real.clone())).collect();
    let h = measure_composite_headroom(&pairs);
    print_composite_headroom("any-true", &h);
    structural_invariants(&h);
}

/// T1, production criterion: the top-1 must be a true lint `--fix` can APPLY.
///
/// `FIXABLE_LINTS` is the `--fix` eligibility table. A top-1 that real clippy
/// confirms but no proposer can rewrite yields a decline, so on that span the
/// pipeline heals nothing even when a fixable true lint sat at rank 2. The
/// lenient criterion scores those spans as wins; this one does not.
#[test]
fn t1_dflash2_composite_headroom_fixable() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    println!("\n=== Issue 034 T1 — headroom, real-clippy composite, FIXABLE-TRUE ===");
    let truth = gather_truth_over(riir_clippy::draft::eval_fixtures_composite::COMPOSITE_SPANS);
    let scorable = composite_scorable(&truth);

    let fixable: BTreeSet<&str> =
        riir_clippy::domains::clippy_lints::proposers::FIXABLE_LINTS
            .iter()
            .copied()
            .collect();

    // Only spans with at least one FIXABLE true lint are scorable: a span
    // whose every true lint is unfixable has no correct answer to select.
    let pairs: Vec<(&Truth, BTreeSet<String>)> = scorable
        .iter()
        .filter_map(|t| {
            let accepted: BTreeSet<String> = t
                .real
                .iter()
                .filter(|l| fixable.contains(l.as_str()))
                .cloned()
                .collect();
            match accepted.is_empty() {
                true => None,
                false => Some((*t, accepted)),
            }
        })
        .collect();
    println!(
        "  multi-lint {} | with a FIXABLE true lint {} ({} fixable lints in the table)",
        scorable.len(),
        pairs.len(),
        fixable.len()
    );
    if pairs.len() < 5 {
        println!("  INSUFFICIENT: fewer than 5 scorable spans — no verdict.");
        return;
    }

    let h = measure_composite_headroom(&pairs);
    print_composite_headroom("fixable-true", &h);
    structural_invariants(&h);
}

/// T1 instrument-power control: does this surface's headroom metric respond
/// at all?
///
/// Both criteria above read 0.0pp with `recall@1` pinned at 100%, and a
/// saturated metric cannot distinguish "no headroom" from "cannot see". The
/// discriminating question is whether the SAME metric on the SAME spans shows
/// headroom for a WEAKER selector. `RerankMode::Off` is that weaker selector:
/// pure text similarity, no instantiation evidence.
///
/// - control shows headroom, shipped mode shows none → the metric works and
///   the T3.5.5 instantiation re-ranker has already consumed the band. A
///   DFlash2-style selector would be re-harvesting a harvested field.
/// - control also reads 0.0pp → the metric is blind on this surface and it
///   contributes no evidence to the T1.2 stop rule either way.
#[test]
fn t1_dflash2_composite_headroom_control() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    println!("\n=== Issue 034 T1 — headroom instrument-power control (rerank OFF vs shipped) ===");
    let truth = gather_truth_over(riir_clippy::draft::eval_fixtures_composite::COMPOSITE_SPANS);
    let scorable = composite_scorable(&truth);
    let pairs: Vec<(&Truth, BTreeSet<String>)> =
        scorable.iter().map(|t| (*t, t.real.clone())).collect();
    if pairs.len() < 5 {
        println!("  INSUFFICIENT: fewer than 5 multi-lint spans — no verdict.");
        return;
    }

    let off = measure_composite_headroom_with(&pairs, RerankMode::Off);
    let shipped = measure_composite_headroom_with(&pairs, RerankMode::default());
    print_composite_headroom("rerank=off", &off);
    print_composite_headroom("rerank=shipped", &shipped);
    structural_invariants(&off);
    structural_invariants(&shipped);

    println!(
        "\n  instrument verdict: off {:+.1}pp vs shipped {:+.1}pp -> {}",
        off.headroom_pp(),
        shipped.headroom_pp(),
        if off.headroom_pp() >= riir_clippy::draft::HEADROOM_STOP_BAR_PP {
            "METRIC HAS POWER — a weaker selector leaves a band this metric sees, so the \
             shipped 0.0pp is a real negative, not blindness"
        } else {
            "METRIC IS BLIND on this surface — it cannot see a band even for the weakest \
             selector, so it contributes NO evidence to the T1.2 stop rule"
        }
    );
}
