//! Issue 024 — real-clippy truth for the cross-domain (co-present) spans.
//!
//! The proposer label test (`switch_cost.rs::cross_domain_fixture_labels_
//! verify_via_proposers`) proves the merged spans fire the right proposers
//! and no cross-domain proposer. This harness adds the oracle half: each
//! merged span COMPILES, and the set of corpus-vocabulary lints real
//! clippy reports on it is EXACTLY `{labeled lint}` — one clippy
//! anti-pattern per span, no combination effects.
//!
//! ```bash
//! cargo test --release --features latent_retrieval,clippy_verify \
//!   --test cross_domain_oracle_truth -- --nocapture --test-threads=1
//! ```
//!
//! Slow (one `cargo clippy` per span, BLAKE3-keyed temp crates so re-runs
//! hit cargo's build cache). Deliberately not a pinned-suite regression
//! floor — same contract as `uncontaminated_g1.rs` (clippy's lint set
//! moves between toolchains; run it when touching the pair table).

#![cfg(all(feature = "latent_retrieval", feature = "clippy_verify"))]

use std::collections::BTreeSet;

use riir_refine::domain::{HealDomain, HealEntry};
use riir_refine::draft::switch_cost::cross_domain_spans;
use riir_refine::draft::{ClippyOracle, DomainId};

fn vocabulary() -> BTreeSet<String> {
    riir_refine::domains::clippy_lints::ClippyLintDomain::build_corpus()
        .into_iter()
        .map(|e| e.rule_id().to_string())
        .collect()
}

#[test]
fn cross_domain_spans_oracle_truth() {
    let oracle = ClippyOracle::new();
    assert!(oracle.is_available(), "cargo not on PATH");
    let vocab = vocabulary();
    let spans = cross_domain_spans();
    for (i, span) in spans.iter().enumerate() {
        let verdict = oracle.lints_for(&span.code);
        assert!(verdict.ran, "cargo clippy did not run: {:?}", verdict.error);
        assert!(
            verdict.compiles,
            "{} does not compile ({}); a non-compiling span yields NO labels",
            span.tag,
            verdict.error.as_deref().unwrap_or("(no error captured)")
        );
        let in_vocab = verdict.within_vocabulary(&vocab);
        assert_eq!(
            in_vocab,
            BTreeSet::from([span.clippy_lint.to_string()]),
            "{}: corpus lints on the merged span must be exactly the labeled one",
            span.tag
        );
        let _ = span.first; // order is asserted implicitly: both orders run
        if (i + 1) % 8 == 0 {
            println!("  ... verified {}/{}", i + 1, spans.len());
        }
    }
    // Both orders present (the directed axis is real data, not a constant).
    let cf = spans.iter().filter(|s| s.first == DomainId::Clippy).count();
    assert_eq!(cf * 2, spans.len());
}
