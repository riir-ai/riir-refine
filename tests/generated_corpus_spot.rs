//! Plan 019 T6.1 — family-level spot integrity for the generated corpus.
//!
//! The full-corpus integrity run (null arm over all N≥212) is the
//! measurement session's job (the example prints + the bench records it);
//! this test keeps the ROUTINE suite fast while still catching a generator
//! family that stops producing its declared code (a template typo, a
//! toolchain change) — one representative case per family, env-gated OFF
//! for default runs via the standard slow-test pattern (cargo IS required).
//!
//! ```bash
//! cargo test --features rustc_errors --test generated_corpus_spot -- --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use riir_clippy::domains::rustc_errors::corpus_gen::generate_corpus;
use riir_clippy::domains::rustc_errors::floor::{FixVerdict, FloorHarness, NullFixer};

/// One case per (family, sub-family) prefix — the first of each.
#[test]
fn every_generated_family_produces_its_declared_code() {
    let corpus = generate_corpus();
    let prefixes = [
        "gen_e0597_str_borrow_",
        "gen_e0597_slice_",
        "gen_e0597_as_str_",
        "gen_e0597_fmt_",
        "gen_e0597_str_idx_",
        "gen_e0499_flat_",
        "gen_e0502_copy_",
        "gen_e0502_len_",
        "gen_e0505_clone_",
        "gen_e0596_mut_",
        "gen_e0596_str_",
        "gen_e0382_clone_",
        "gen_e0507_field_",
        "gen_e0373_str_",
        "gen_e0373_sum_",
    ];
    let mut picked = Vec::new();
    for p in prefixes {
        let Some(c) = corpus.iter().find(|c| c.name.starts_with(p)) else {
            panic!("generated corpus is missing family {p}");
        };
        picked.push(c);
    }

    let harness = FloorHarness::new();
    if !harness.is_available() {
        eprintln!("SKIP: cargo not on PATH");
        return;
    }
    let cases: Vec<_> = picked
        .iter()
        .map(|c| riir_clippy::domains::rustc_errors::floor::HarnessCase::generated(c))
        .collect();
    let report = harness
        .measure_cases(&NullFixer, &cases)
        .expect("harness ran");
    harness.cleanup();

    let bad: Vec<&str> = report
        .outcomes
        .iter()
        .filter(|o| !o.mutant_valid || o.verdict != FixVerdict::Unfixed)
        .map(|o| o.name.as_str())
        .collect();
    assert!(
        bad.is_empty(),
        "family integrity failures (mutant compiles or wrong code): {bad:?}"
    );
}

/// The full-corpus heavy gate (env-gated): the null arm over EVERY case —
/// committed anchors + the whole generated corpus. This is the same check
/// the measurement example runs; the env gate keeps it out of routine
/// suites (it is ~200 cargo invocations) while keeping it one command for
/// CI-when-wanted.
#[test]
fn full_corpus_declared_codes_hold() {
    if std::env::var("RIIR_FULL_CORPUS_INTEGRITY").ok().as_deref() != Some("1") {
        eprintln!("SKIP: set RIIR_FULL_CORPUS_INTEGRITY=1 to run the full-corpus gate");
        return;
    }
    use riir_clippy::domains::rustc_errors::floor::HarnessCase;
    let harness = FloorHarness::new();
    if !harness.is_available() {
        eprintln!("SKIP: cargo not on PATH");
        return;
    }
    let mut cases: Vec<HarnessCase<'_>> = riir_clippy::domains::rustc_errors::floor::MUTANT_CASES
        .iter()
        .map(|c| HarnessCase::committed(c).expect("committed integrity"))
        .collect();
    let corpus = generate_corpus();
    for g in &corpus {
        cases.push(HarnessCase::generated(g));
    }
    let report = harness
        .measure_cases(&NullFixer, &cases)
        .expect("harness ran");
    harness.cleanup();
    assert!(
        report.invalid_mutants.is_empty(),
        "corpus integrity failures:\n  {}",
        report.invalid_mutants.join("\n  ")
    );
    assert_eq!(
        report.total_fix_rate(),
        0.0,
        "null arm repaired something — harness measuring the mutation"
    );
    // Seed-validity half: every seed's ground-truth test must pass — a
    // seed with a wrong expectation poisons the semantics pass (the
    // first corpus shipped 10; only post-fix testing caught them because
    // unrepaired cases never get tested).
    let seed_harness = FloorHarness::new();
    let bad_seeds = seed_harness
        .verify_seed_tests(&cases)
        .expect("seed verification ran");
    seed_harness.cleanup();
    assert!(
        bad_seeds.is_empty(),
        "seeds whose OWN ground-truth test fails (wrong expectations — the \
         corpus ground truth is broken):\n  {}",
        bad_seeds.join("\n  ")
    );
}
