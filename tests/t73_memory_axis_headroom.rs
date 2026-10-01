//! Plan 005 T7.3 — does trajectory memory have headroom as a retrieval axis?
//!
//! T7.3 proposes consulting [`LatentFixMemory`] before the corpus retrieve:
//! *if a validated past fix exists for a similar span, prefer its rule*. That
//! is a third evidence axis on top of text similarity and (since T3.5.5)
//! proposer instantiation.
//!
//! Before wiring it, this measures whether the opportunity exists — building a
//! third axis that cannot change any ranking would be waste, and the honest
//! way to find out is to look rather than to assume.
//!
//! ## The split, and why it is held out
//!
//! T3.5.4 was a lesson in measuring against a contaminated oracle. Populating
//! memory from the spans it is then evaluated on would be the same mistake in
//! a new coat — the memory would contain the answer verbatim.
//!
//! So the two span sets are kept disjoint, which also happens to mirror the
//! real deployment shape:
//!
//! - **Populate** from the 48 single-lint `CLIPPY_SPANS` (the healer's history
//!   of simple, already-validated fixes).
//! - **Evaluate** on the 25 multi-lint `COMPOSITE_SPANS` (new, harder code).
//!
//! Different source text, authored separately, no overlap.
//!
//! ```bash
//! cargo test --release --features latent_retrieval,clippy_verify \
//!   --test t73_memory_axis_headroom -- --nocapture --test-threads=1
//! ```

#![cfg(all(feature = "latent_retrieval", feature = "clippy_verify"))]

use std::collections::BTreeSet;

use riir_refine::domain::{HealDomain, HealEntry};
use riir_refine::draft::{ClippyOracle, DomainId, LatentMatcher, RERANK_POOL};
use riir_refine::memory::{LatentFixMemory, LatentQuery, RetrievalWeights};

fn vocabulary() -> BTreeSet<String> {
    riir_refine::domains::clippy_lints::ClippyLintDomain::build_corpus()
        .into_iter()
        .map(|e| e.rule_id().to_string())
        .collect()
}

/// Memory populated from the single-lint fixtures — the healer's "history".
///
/// Each is recorded as a CERTIFIED trajectory (full-validator admission):
/// these are the fixtures real `cargo clippy` confirms, so the strongest
/// evidence tier is the honest one. Weaker tiers would only understate the
/// axis, and the question here is whether it has headroom AT BEST.
fn populate_memory() -> LatentFixMemory {
    use riir_refine::domains::clippy_lints::proposers::propose_fix;
    let mut memory = LatentFixMemory::new(RetrievalWeights::default());
    let mut recorded = 0usize;
    for (lint, code) in riir_refine::draft::eval_fixtures_clippy::CLIPPY_SPANS {
        // Only record fixes we can actually produce — a trajectory needs a
        // fixed_code, and inventing one would poison the memory.
        if let Some(fixed) = propose_fix(lint, code) {
            memory.record_certified(lint, code, &fixed);
            recorded += 1;
        }
    }
    println!("  memory populated: {recorded} certified trajectories from the single-lint set");
    memory
}

#[test]
fn t73_memory_axis_headroom() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    println!("\n=== T7.3 — trajectory-memory axis: headroom probe ===");

    let memory = populate_memory();
    let vocab = vocabulary();

    // Real-clippy ground truth for the held-out composite spans.
    let mut truth: Vec<(&str, &str, BTreeSet<String>)> = Vec::new();
    for (tag, code) in riir_refine::draft::eval_fixtures_composite::COMPOSITE_SPANS {
        let v = oracle.lints_for(code);
        if !v.compiles {
            continue;
        }
        let real = v.within_vocabulary(&vocab);
        if real.len() > 1 {
            truth.push((tag, code, real));
        }
    }
    println!("  held-out multi-lint spans: {}", truth.len());

    let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");

    // Three counters that between them decide whether to build the axis:
    //   already_right  — shipped ranking already puts a true rule first;
    //                    memory cannot improve these.
    //   memory_could   — shipped top-1 is wrong AND memory's top pick for the
    //                    span is a true rule → the axis would fix it.
    //   memory_cannot  — shipped top-1 is wrong and memory does not know
    //                    better → the axis is silent.
    let (mut already_right, mut memory_could, mut memory_cannot) = (0usize, 0usize, 0usize);
    let mut memory_would_break = 0usize;

    for (tag, code, real) in &truth {
        let (_, by_domain) = matcher.suggest_span(code, RERANK_POOL);
        let ranked: Vec<&str> = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .map_or(Vec::new(), |(_, rules)| {
                rules.iter().map(|r| r.rule_id.as_str()).collect()
            });
        let top1_ok = ranked.first().is_some_and(|r| real.contains(*r));

        // What does memory say? Query it once per candidate rule in the pool
        // and take the best-scoring hit — the shape T7.3 describes.
        let mut best: Option<(&str, f32)> = None;
        for cand in ranked.iter().take(RERANK_POOL) {
            let q = LatentQuery::new((*cand).to_string(), code);
            if let Some(hit) = memory.retrieve(&q, 1).first() {
                if best.is_none_or(|(_, s)| hit.score > s) {
                    best = Some((cand, hit.score));
                }
            }
        }
        let memory_pick_is_true = best.is_some_and(|(r, _)| real.contains(r));

        match (top1_ok, memory_pick_is_true) {
            (true, false) if best.is_some() => {
                memory_would_break += 1;
                already_right += 1;
            }
            (true, _) => already_right += 1,
            (false, true) => {
                memory_could += 1;
                println!(
                    "    COULD FIX  {tag}: top1={:?} memory={:?}",
                    ranked.first(),
                    best.map(|b| b.0)
                );
            }
            (false, false) => memory_cannot += 1,
        }
    }

    let n = truth.len();
    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize| {
        if n == 0 {
            0.0
        } else {
            k as f32 / n as f32 * 100.0
        }
    };
    println!(
        "\n  shipped ranking already correct : {already_right}/{n} ({:.1}%)",
        pct(already_right)
    );
    println!(
        "  memory COULD fix a wrong top-1  : {memory_could}/{n} ({:.1}%)  <- the headroom",
        pct(memory_could)
    );
    println!(
        "  memory silent on a wrong top-1  : {memory_cannot}/{n} ({:.1}%)",
        pct(memory_cannot)
    );
    println!("  memory would OVERRIDE a correct top-1 with a false rule: {memory_would_break}/{n} ({:.1}%)  <- the risk", pct(memory_would_break));

    println!(
        "\n  VERDICT: {}",
        if memory_could == 0 {
            "NO HEADROOM — the memory axis cannot change a single ranking on \
             held-out spans. Building it would be dead weight."
        } else if memory_could > memory_would_break {
            "HEADROOM EXISTS — wire the axis and gate it on the held-out set."
        } else {
            "NET-NEGATIVE — memory would break at least as many rankings as it fixes."
        }
    );
}
