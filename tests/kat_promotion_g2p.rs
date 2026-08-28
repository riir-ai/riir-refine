//! KAT T2.4 (Plan 042) — the promotion gate's G2P fixtures: the real-oracle
//! e2e verdicts + the history-isolation pin. The pure halves (schema,
//! registry absorbing, verdict comparison, overlay merge, determinism of
//! the candidate lane) live in the module's unit tests; this binary pins
//! what only the REAL gate can:
//!
//! - **promote-pass** — a benign candidate clears the bar and lands in the
//!   registry as Certified.
//! - **reject-on-heal-regression** — a candidate whose `fixed` parses but
//!   does not COMPILE (type error) scores as nothing-resolved → heal bar.
//! - **reject-on-created-regression** — a candidate whose `fixed` compiles
//!   but introduces a NEW pedantic lint (a truncating cast) → created bar.
//! - **reject-on-parse-fail** — schema-level (never trialed).
//! - **withdrawn-absorbing** — a sweep-detected regressor is withdrawn and
//!   can NEVER re-promote (same id OR same content under a new id).
//! - **determinism ×3** — the same candidates + same base → identical
//!   verdicts across three full gate runs.
//! - **history isolation** — `.benchmarks/score_history.jsonl` is
//!   byte-identical after every trial run above.
//!
//! These run in CI Layer 5c (all-features). The oracle shells out to real
//! cargo clippy on a one-fixture subset (the frozen-fixture codes are
//! content-cached by the oracle's work root, so repeat verdicts are fast).
//! The oracle's shared work root is NOT concurrency-safe — every oracle
//! test takes the binary-wide [`ORACLE_LOCK`] (the GPU_TEST_LOCK
//! precedent).

#![cfg(all(feature = "kat_service", feature = "score_bench"))]

use std::sync::Mutex;

use riir_clippy::kat_promotion::{
    parse_candidates, run_posthoc_sweep, run_promotion_gate, CandidateRow, PromotionRegistry,
    RejectReason, Verdict, CANDIDATE_SCHEMA_VERSION, DOMAIN_CLIPPY,
};
use riir_clippy::score_bench::fixtures::fixtures;
use riir_clippy::score_bench::{draft_oracle, report::HISTORY_JSONL};

/// Serialize every oracle-using test in this binary (the oracle's shared
/// work root + cargo target dir race under parallel invocation).
static ORACLE_LOCK: Mutex<()> = Mutex::new(());

/// The one frozen fixture all e2e candidates target:
/// `composite-return_plus_identity_op` —
/// `pub fn scale_ticks(ticks: u32) -> u32 { return ticks * 1; }`
/// (vocabulary lints at freeze: identity_op + needless_return).
fn subset() -> Vec<riir_clippy::score_bench::BenchFixture> {
    fixtures()
        .into_iter()
        .filter(|f| f.id == "composite-return_plus_identity_op")
        .collect()
}

fn row(rule_id: &str, buggy: &str, fixed: &str) -> CandidateRow {
    CandidateRow {
        schema_version: CANDIDATE_SCHEMA_VERSION,
        rule_id: rule_id.to_string(),
        domain: DOMAIN_CLIPPY.to_string(),
        buggy_code: buggy.to_string(),
        fixed_code: fixed.to_string(),
        provenance: "g2p-test".to_string(),
    }
}

const FIXTURE_SRC: &str = "pub fn scale_ticks(ticks: u32) -> u32 {\n    return ticks * 1;\n}";

const BENIGN_FIXED: &str = "pub fn scale_ticks(ticks: u32) -> u32 {\n    ticks\n}";

/// `fixed` parses but does not compile (type error) — applied-breakage,
/// scored as nothing-resolved ⇒ the heal bar.
const HEAL_POISON_FIXED: &str = "pub fn scale_ticks(ticks: u32) -> u32 {\n    \"not a u32\"\n}";

/// `fixed` compiles but introduces a NEW pedantic lint (truncating cast —
/// proposer-irreducible) ⇒ the created bar.
const CREATED_POISON_FIXED: &str =
    "pub fn scale_ticks(ticks: u32) -> u32 {\n    let _trunc = ticks as u8;\n    ticks\n}";

fn snapshot_history() -> Option<Vec<u8>> {
    std::fs::read(HISTORY_JSONL).ok()
}

/// One gate run over one candidate — the per-scenario shape (candidates in
/// one file may shadow each other's match keys; each rejection path gets
/// its OWN trial so the bar it trips is unambiguous).
fn gate_one(
    oracle: &riir_clippy::draft::clippy_oracle::ClippyOracle,
    candidate: CandidateRow,
) -> (riir_clippy::kat_promotion::GateReport, PromotionRegistry) {
    let mut registry = PromotionRegistry::default();
    let report = run_promotion_gate(
        oracle,
        &subset(),
        std::slice::from_ref(&candidate),
        &mut registry,
    );
    (report, registry)
}

#[test]
fn promote_pass_then_both_regression_bars_with_history_isolation() {
    let _guard = ORACLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let oracle = draft_oracle();
    assert!(oracle.is_available(), "cargo not on PATH — the gate cannot derive ground truth");
    let history_before = snapshot_history();

    // ── promote-pass: the benign candidate clears the bar ──────────────
    let (report, registry) = gate_one(&oracle, row("net-benign", FIXTURE_SRC, BENIGN_FIXED));
    let v = &report.verdicts[0];
    assert_eq!(v.rule_id, "net-benign");
    assert_eq!(v.verdict, Verdict::Promoted, "benign clears the bar: {v:?}");
    assert!(v.fired, "the benign candidate's buggy_code occurs in the fixture");
    assert_eq!(registry.promoted.len(), 1);
    assert_eq!(
        registry.promoted["net-benign"].tier,
        riir_clippy::memory::EvidenceTier::Certified
    );
    assert!(registry.overlay_triples().iter().any(|t| t.0 == "net-benign"));

    // ── reject-on-heal-regression: parses, does not compile ────────────
    let (report, registry) = gate_one(&oracle, row("net-heal-poison", FIXTURE_SRC, HEAL_POISON_FIXED));
    let v = &report.verdicts[0];
    assert!(
        matches!(v.verdict, Verdict::Rejected(RejectReason::HealRateRegression { base: 2, trial: 0 })),
        "type-error candidate rejected on the heal bar (2 resolved baseline → 0): {v:?}"
    );
    assert!(registry.promoted.is_empty(), "a rejected candidate never enters the overlay");

    // ── reject-on-created-regression: compiles, introduces a new lint ──
    let (report, registry) =
        gate_one(&oracle, row("net-created-poison", FIXTURE_SRC, CREATED_POISON_FIXED));
    let v = &report.verdicts[0];
    assert!(
        matches!(
            v.verdict,
            Verdict::Rejected(RejectReason::CreatedRateRegression { base: 0, .. })
        ),
        "cast-poison candidate rejected on the created bar: {v:?}"
    );
    assert!(registry.promoted.is_empty());

    // HISTORY ISOLATION: none of the trials above touched the real history.
    assert_eq!(
        snapshot_history(),
        history_before,
        "trial runs must never touch score_history.jsonl"
    );
}

#[test]
fn gate_verdicts_are_bit_identical_across_three_runs() {
    let _guard = ORACLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let oracle = draft_oracle();
    assert!(oracle.is_available(), "cargo not on PATH");
    let candidates = vec![
        row("net-benign", FIXTURE_SRC, BENIGN_FIXED),
        row("net-heal-poison", FIXTURE_SRC, HEAL_POISON_FIXED),
    ];
    let mut runs: Vec<Vec<(String, Verdict, bool)>> = Vec::new();
    for _ in 0..3 {
        let mut registry = PromotionRegistry::default();
        let report = run_promotion_gate(&oracle, &subset(), &candidates, &mut registry);
        runs.push(
            report
                .verdicts
                .iter()
                .map(|v| (v.rule_id.clone(), v.verdict.clone(), v.fired))
                .collect(),
        );
    }
    assert_eq!(runs[0], runs[1], "determinism: gate run 1 vs 2");
    assert_eq!(runs[1], runs[2], "determinism: gate run 2 vs 3");
}

#[test]
fn parse_unsafe_candidate_is_rejected_at_the_schema() {
    // fixed_code that does not parse never reaches trial — rejected by
    // parse_candidates with the ParseUnsafe reason (no oracle needed).
    let broken = format!(
        "{{\"schema_version\":{CANDIDATE_SCHEMA_VERSION},\"rule_id\":\"r-bad\",\"domain\":\"clippy\",\"buggy_code\":\"return 1;\",\"fixed_code\":\"fn (\"}}"
    );
    let err = parse_candidates(broken.as_bytes()).unwrap_err();
    assert!(err.contains("ParseUnsafe"), "{err}");

    // Defense-in-depth: the gate re-rejects such a row even when handed one
    // directly (constructed, bypassing parse).
    let oracle = draft_oracle();
    if !oracle.is_available() {
        eprintln!("cargo not on PATH — skipping the gate half of this test");
        return;
    }
    let _guard = ORACLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut registry = PromotionRegistry::default();
    let r = row("r-bad", "return 1;", "fn (");
    let report = run_promotion_gate(&oracle, &subset(), std::slice::from_ref(&r), &mut registry);
    assert!(
        matches!(report.verdicts[0].verdict, Verdict::Rejected(RejectReason::ParseUnsafe)),
        "{:?}",
        report.verdicts[0]
    );
    assert!(registry.promoted.is_empty());
}

#[test]
fn posthoc_sweep_withdraws_regressor_and_it_can_never_return() {
    let oracle = draft_oracle();
    assert!(oracle.is_available(), "cargo not on PATH");
    let _guard = ORACLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // A rule that PASSED its gate once (or was promoted under an older
    // bench) and now regresses: simulate by promoting the type-error
    // candidate directly through the registry API (the documented
    // post-hoc entry path — the sweep is what catches it).
    let mut registry = PromotionRegistry::default();
    registry.promote(row("net-regressed", FIXTURE_SRC, HEAL_POISON_FIXED)).unwrap();
    assert_eq!(registry.promoted.len(), 1);

    let sweep = run_posthoc_sweep(&oracle, &subset(), &mut registry);
    assert!(sweep.ran, "the sweep runs when the overlay is live");
    assert_eq!(sweep.withdrawn, vec!["net-regressed".to_string()]);
    assert!(registry.promoted.is_empty(), "withdrawal removes the rule from the overlay");
    assert!(registry.is_withdrawn(&row("net-regressed", FIXTURE_SRC, HEAL_POISON_FIXED)));

    // ABSORBING: re-running the gate with the same candidate NEVER
    // re-promotes — no re-certification exit for network rules.
    let report = run_promotion_gate(
        &oracle,
        &subset(),
        &[row("net-regressed", FIXTURE_SRC, HEAL_POISON_FIXED)],
        &mut registry,
    );
    assert!(
        matches!(report.verdicts[0].verdict, Verdict::Rejected(RejectReason::WithdrawnAbsorbing)),
        "{:?}",
        report.verdicts[0]
    );
    // Nor can it return under a NEW id with the same content.
    let report2 = run_promotion_gate(
        &oracle,
        &subset(),
        &[row("net-regressed-v2", FIXTURE_SRC, HEAL_POISON_FIXED)],
        &mut registry,
    );
    assert!(
        matches!(report2.verdicts[0].verdict, Verdict::Rejected(RejectReason::WithdrawnAbsorbing)),
        "{:?}",
        report2.verdicts[0]
    );
    assert!(registry.promoted.is_empty());

    // A clean sweep afterwards reports no live rules.
    let again = run_posthoc_sweep(&oracle, &subset(), &mut registry);
    assert!(!again.ran);
}
