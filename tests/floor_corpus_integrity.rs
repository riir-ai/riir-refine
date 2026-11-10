//! Issue 035 T2 — the G-floor harness's own gates.
//!
//! A floor measurement is only as trustworthy as its corpus, and a broken
//! corpus fails SILENTLY in the flattering direction: a "mutant" that still
//! compiles scores as a free win for every arm, inflating fix rates. Both
//! failure modes below were REAL — found by these checks on the first run:
//!
//! - `e0597_ref_escapes_block` narrowed a scope around a `usize` length.
//!   `usize` is `Copy`, so nothing was borrowed and the mutant compiled.
//! - `e0502_borrow_across_mutation` borrowed but kept an `as usize` cast, so
//!   it failed with E0606 (bad cast) rather than E0502 (borrow conflict).
//!
//! Slow: scaffolds a temp crate and runs `cargo check` twice per mutant.
//!
//! ```bash
//! cargo test --features rustc_errors --test floor_corpus_integrity -- --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use riir_refine::domains::rustc_errors::floor::{
    FixVerdict, FloorHarness, MUTANT_CASES, NullFixer,
};

/// Every mutant must fail to compile, with the code it declares.
///
/// This runs the no-op arm, so what it measures is the CORPUS, not a fixer.
#[test]
fn every_mutant_fails_with_its_declared_code() {
    let harness = FloorHarness::new();
    if !harness.is_available() {
        eprintln!("SKIP: cargo not on PATH");
        return;
    }
    let report = harness.measure(&NullFixer).expect("harness ran");
    harness.cleanup();

    assert!(
        report.invalid_mutants.is_empty(),
        "corpus integrity failures — these mutants do not provoke the code \
         they claim, so every arm's score on them is meaningless:\n  {}",
        report.invalid_mutants.join("\n  ")
    );
}

/// The falsification check: an arm that repairs nothing must score exactly 0%.
///
/// Above zero means some mutant compiles without help, i.e. the harness is
/// measuring the mutation rather than the fixer.
#[test]
fn the_no_op_arm_repairs_nothing() {
    let harness = FloorHarness::new();
    if !harness.is_available() {
        eprintln!("SKIP: cargo not on PATH");
        return;
    }
    let report = harness.measure(&NullFixer).expect("harness ran");
    let free_wins: Vec<String> = report
        .outcomes
        .iter()
        .filter(|o| o.verdict != FixVerdict::Unfixed)
        .map(|o| o.name.clone())
        .collect();
    harness.cleanup();

    assert!(
        free_wins.is_empty(),
        "the no-op arm scored {:.1}% — these mutants compile with no fixer at \
         all, so they are free wins that inflate every arm: {:?}",
        report.total_fix_rate() * 100.0,
        free_wins
    );
}

/// Corpus shape: both territories represented, and rows partition cleanly.
#[test]
fn corpus_is_stratified_and_rows_partition() {
    assert!(
        MUTANT_CASES.len() >= 12,
        "corpus shrank to {} cases",
        MUTANT_CASES.len()
    );
    for code in ["E0597", "E0499", "E0502"] {
        assert!(
            MUTANT_CASES.iter().any(|c| c.expect_code == code),
            "uncontested code {code} missing — coverage would be measured \
             only over codes cargo fix already owns"
        );
    }
    let harness = FloorHarness::new();
    if !harness.is_available() {
        eprintln!("SKIP: cargo not on PATH");
        return;
    }
    let report = harness.measure(&NullFixer).expect("harness ran");
    harness.cleanup();
    let mut counted = 0;
    for row in &report.rows {
        assert_eq!(
            row.matched + row.diverged + row.unfixed,
            row.mutants,
            "{} row does not partition",
            row.code
        );
        counted += row.mutants;
    }
    assert_eq!(counted, MUTANT_CASES.len(), "rows dropped mutants");
}
