//! Issue 191 T2 — the G1 harness's standing smoke gate.
//!
//! Asserts, in one fast run:
//! 1. **Census reconciliation** — the extraction (corpus API, per-file via the
//!    `pub(super)` builders) sees exactly the `#[test]` population the raw
//!    source text carries, modulo the pinned in-tree deltas (the T1
//!    anti-circularity law: an extraction bug, an unwired new batch file, or
//!    an unpinned in-tree corpus change FAILS here, never shrinks the
//!    population).
//! 2. **Floors** — fixture-bearing rules ≥ 248, files ≥ 72 (downward-only,
//!    pinned to the exact first census; the T1 estimate was 290/70).
//! 3. **One end-to-end execution** — the first fixture-bearing rule in
//!    name-ascending module order (deterministic, pre-registered pick)
//!    compiles standalone and its embedded tests pass. This exercises the
//!    rustc-spawn machinery the full runner uses, on real corpus bytes.
//!
//! The FULL population run is the campaign lane
//! (`cargo run --release --example kernel_goat_g1 --features kernel_opt -- --full`),
//! not a test target — minutes of rustc invocations do not belong in
//! `cargo test`.
#![cfg(feature = "kernel_opt")]

use riir_refine::domains::kernel_opt::corpus::goat_g1;

#[test]
fn census_extraction_reconciles_with_the_independent_structural_census() {
    let raw = goat_g1::structural_census().expect("census reads its own crate sources");
    let extracted = goat_g1::extracted_census();
    goat_g1::compare(&raw, &extracted)
        .expect("extraction must reconcile with the independent structural census");
}

#[test]
fn floors_hold() {
    let extracted = goat_g1::extracted_census();
    assert!(
        extracted.fixture_rules >= goat_g1::MIN_FIXTURE_RULES,
        "fixture-bearing rules {} below the downward-only floor {}",
        extracted.fixture_rules,
        goat_g1::MIN_FIXTURE_RULES
    );
    assert!(
        extracted.fixture_files >= goat_g1::MIN_FIXTURE_FILES,
        "fixture-bearing files {} below the downward-only floor {}",
        extracted.fixture_files,
        goat_g1::MIN_FIXTURE_FILES
    );
}

#[test]
fn one_fixture_rule_executes_end_to_end() {
    // Deterministic pick: the first fixture-bearing rule in the extraction
    // order (which mirrors the corpus build order) — pre-registered, no
    // sampling knob. The pair compile exercises the rustc-spawn machinery the
    // full runner uses, on real corpus bytes.
    let report = goat_g1::run_g1(None, Some(1));
    report
        .census_check
        .as_ref()
        .expect("census equality binds even a sampled run");
    let executed: Vec<_> = report
        .rules
        .iter()
        .filter(|r| r.fixture_bearing)
        .collect();
    assert_eq!(
        executed.len(),
        1,
        "the sampled run must execute exactly one fixture-bearing rule"
    );
    let rule = executed[0];
    assert_eq!(
        rule.outcome(),
        goat_g1::G1Outcome::Green,
        "the sampled fixture rule {:?} must compile AND pass its embedded tests: {:?}",
        rule.rule_id,
        rule.arms
            .iter()
            .filter_map(|a| a.error.clone())
            .collect::<Vec<_>>()
    );
}
