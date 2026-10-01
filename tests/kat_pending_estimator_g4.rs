//! KAT pending-record estimator — G4-class adversarial fixtures (Proposal 005
//! Phase 0 T0.4, Plan 028).
//!
//! **The gates here are the caveat-#15 framing gate + the recovery gate.**
//! The estimator is the surface that renders an ACCRUAL (money that is not
//! money — KAT is a non-transferable service credit): no rendered string may
//! carry `$` beside an accrual, blocked rows may never earn a KAT figure, and
//! the accrual line must travel with "service credit" + "if proven" +
//! "provisional". The watermark must recover-to-empty (over-sync is the
//! fail-safe direction) on corrupt + version-mismatch, never abort. The
//! estimator must be deterministic (identical inputs → identical estimate +
//! lines) — the `build_at` precedent applied to accounting.
//!
//! ```bash
//! cargo test --all-features --test kat_pending_estimator_g4 -- --nocapture
//! ```
//!
//! NOTE: the planted string literals are DELIBERATELY secret-shaped (the
//! gitleaks-fixture convention — fake values in published token formats).

#![cfg(all(feature = "kat_service", feature = "latent_retrieval"))]

use riir_clippy::kat_pending_estimator::{
    estimate_pending, PendingEstimate, SyncWatermark, ESTIMATE_VERSION,
    PROVISIONAL_KAT_PER_PENDING_RECORD, SYNC_STATE_VERSION,
};
use riir_clippy::kat_redact::{RedactionGate, SpanOrigin, SyncPolicy, REDACT_VERSION};
use riir_clippy::kat_sync_payload::SyncPayloadBuilder;
use riir_clippy::traj_store::TrajRowDoc;

fn allow_policy() -> SyncPolicy {
    SyncPolicy {
        version: REDACT_VERSION,
        sync_enabled: true,
        allow_origins: vec!["own-repo".into()],
        allow_paths: Vec::new(),
        max_span_bytes: 65_536,
        allow_redacted: false,
    }
}

fn redact_policy() -> SyncPolicy {
    SyncPolicy {
        allow_redacted: true,
        ..allow_policy()
    }
}

/// The golden signing seed (`[42; 32]`) — the mirror of the lib's
/// `GOLDEN_SIGNING_SEED`; integration tests cannot see the lib's
/// `#[cfg(test)]` helper, so the literal is mirrored here (the documented
/// cross-repo mirror convention).
fn golden_signer() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[42u8; 32])
}

fn row(lint_key: &str, shape: &str, fix: &str) -> TrajRowDoc {
    TrajRowDoc {
        lint_key: lint_key.into(),
        shape: shape.into(),
        fix: fix.into(),
        direction: vec![0.1, -0.2, 0.3, 0.0, 0.5, -0.5, 0.7, 0.9],
        success_count: 3,
        fail_count: 1,
        consecutive_fails: 0,
        tier: 0,
        last_run_id: "run-abc123-42".into(),
        last_seen_at: 1_700_000_000,
        elo: 1215.5,
        span_elo: 1188.25,
        component: None,
        journey_runs: 0,
        journey_taint_sum: 0,
    }
}

fn two_rows() -> Vec<TrajRowDoc> {
    vec![
        row(
            "uninlined_format_args",
            "fn f() { println!(\"{}\", x); }",
            "fn f() { println!(\"{x}\"); }",
        ),
        row("needless_return", "fn g() { return 1; }", "fn g() { 1 }"),
    ]
}

/// The watermark a successful sync of `rows` under `policy` leaves.
fn watermark_after_sync(rows: &[TrajRowDoc], policy: &SyncPolicy) -> SyncWatermark {
    let gate = RedactionGate::from_policy(policy.clone());
    let signer = golden_signer();
    let payload = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(rows, 0);
    SyncWatermark::from_payload(&payload)
}

fn est(rows: &[TrajRowDoc], policy: &SyncPolicy, wm: &SyncWatermark) -> PendingEstimate {
    let gate = RedactionGate::from_policy(policy.clone());
    estimate_pending(rows, &gate, SpanOrigin::OwnRepo, wm)
}

// ── Estimator partition ─────────────────────────────────────────────────────

#[test]
fn empty_store_zero_everything() {
    let e = est(&[], &allow_policy(), &SyncWatermark::empty());
    assert_eq!(e, PendingEstimate::default());
    assert_eq!(e.est_kat_service_credit(), 0);
    assert!(e.pending_line().contains("Records pending proof: 0"));
}

#[test]
fn bootstrap_every_gate_passed_row_is_unsynced() {
    // No sync has EVER happened (the pre-T1.6 normal state).
    let e = est(&two_rows(), &allow_policy(), &SyncWatermark::empty());
    assert_eq!(e.pending_unsynced, 2);
    assert_eq!(e.synced_awaiting_proof, 0);
    assert_eq!(e.blocked, 0);
    assert_eq!(
        e.est_kat_service_credit(),
        2 * PROVISIONAL_KAT_PER_PENDING_RECORD
    );
}

#[test]
fn full_coverage_estimate_holds_across_classes() {
    // Everything synced — the accrual must NOT drop (rows only leave the
    // accrual when the server proves or rejects; locally they stay pending).
    let rows = two_rows();
    let policy = allow_policy();
    let wm = watermark_after_sync(&rows, &policy);
    let e = est(&rows, &policy, &wm);
    assert_eq!(e.pending_unsynced, 0);
    assert_eq!(e.synced_awaiting_proof, 2);
    assert_eq!(e.records_pending_proof(), 2);
    assert_eq!(
        e.est_kat_service_credit(),
        2 * PROVISIONAL_KAT_PER_PENDING_RECORD
    );
}

#[test]
fn verdict_change_re_pends_the_row() {
    let rows = two_rows();
    let policy = allow_policy();
    let wm = watermark_after_sync(&rows, &policy);
    let mut changed = rows.clone();
    changed[0].fail_count += 1; // new evidence the server lacks
    let e = est(&changed, &policy, &wm);
    assert_eq!(e.pending_unsynced, 1);
    assert_eq!(e.synced_awaiting_proof, 1);
}

#[test]
fn tier_change_re_pends_the_row() {
    let rows = two_rows();
    let policy = allow_policy();
    let wm = watermark_after_sync(&rows, &policy);
    let mut changed = rows.clone();
    changed[1].tier = 2; // Heuristic → Withdrawn (Issue 021 semantics)
    let e = est(&changed, &policy, &wm);
    assert_eq!(e.pending_unsynced, 1);
    assert_eq!(e.synced_awaiting_proof, 1);
}

#[test]
fn new_row_pends() {
    let rows = two_rows();
    let policy = allow_policy();
    let wm = watermark_after_sync(&rows, &policy);
    let mut grown = rows.clone();
    grown.push(row("identity_op", "let y = x * 1;", "let y = x;"));
    let e = est(&grown, &policy, &wm);
    assert_eq!(e.pending_unsynced, 1);
    assert_eq!(e.synced_awaiting_proof, 2);
}

#[test]
fn deleted_row_leaves_no_residue() {
    // The watermark may cover MORE rows than the store holds (rows pruned by
    // compaction / withdrawn-cleanup). Coverage of absent rows is inert —
    // counts stay store-driven.
    let rows = two_rows();
    let policy = allow_policy();
    let wm = watermark_after_sync(&rows, &policy);
    let pruned = &rows[..1];
    let e = est(pruned, &policy, &wm);
    assert_eq!(e.pending_unsynced, 0);
    assert_eq!(e.synced_awaiting_proof, 1);
    assert_eq!(e.blocked, 0);
}

#[test]
fn coverage_follows_the_projection_not_the_row() {
    // A secret-bearing row the REDACTING policy includes (placeholders in
    // place): synced + covered. Under the ALLOW policy the same row is
    // BLOCKED — coverage evaporates when the projection changes class (the
    // server holds a row we would no longer send). Deterministic: the
    // planted AKIA value is the payload G4 fixture shape.
    let dirty = row(
        "needless_return",
        "let k = \"AKIAIOSFODNN7EXAMPLE\"; return k;",
        "let k = \"AKIAIOSFODNN7EXAMPLE\"; k",
    );
    let wm = watermark_after_sync(std::slice::from_ref(&dirty), &redact_policy());
    let e = est(std::slice::from_ref(&dirty), &redact_policy(), &wm);
    assert_eq!(e.synced_awaiting_proof, 1);
    assert_eq!(e.blocked, 0);

    let e2 = est(std::slice::from_ref(&dirty), &allow_policy(), &wm);
    assert_eq!(e2.blocked, 1);
    assert_eq!(e2.synced_awaiting_proof, 0);
    assert_eq!(e2.records_pending_proof(), 0);

    // The no-spurious-churn side: policies that produce IDENTICAL
    // projections (clean rows, no findings) do not re-pend.
    let clean = two_rows();
    let wm_clean = watermark_after_sync(&clean, &allow_policy());
    let e3 = est(&clean, &redact_policy(), &wm_clean);
    assert_eq!(e3.synced_awaiting_proof, 2);
    assert_eq!(e3.pending_unsynced, 0);
}

#[test]
fn planted_secret_blocks_and_never_earns() {
    // The load-bearing blocked-row case: a credential in `shape` excludes
    // the row; it counts as blocked and contributes ZERO to the accrual.
    let dirty = row(
        "needless_return",
        "let k = \"AKIAIOSFODNN7EXAMPLE\"; return k;",
        "let k = \"AKIAIOSFODNN7EXAMPLE\"; k",
    );
    let rows = vec![two_rows()[0].clone(), dirty];
    let e = est(&rows, &allow_policy(), &SyncWatermark::empty());
    assert_eq!(e.blocked, 1);
    assert_eq!(e.pending_unsynced, 1);
    assert_eq!(e.records_pending_proof(), 1);
    assert_eq!(
        e.est_kat_service_credit(),
        PROVISIONAL_KAT_PER_PENDING_RECORD
    );
}

#[test]
fn all_blocked_accrual_zero() {
    let gate = RedactionGate::deny_all();
    let e = estimate_pending(
        &two_rows(),
        &gate,
        SpanOrigin::OwnRepo,
        &SyncWatermark::empty(),
    );
    assert_eq!(e.blocked, 2);
    assert_eq!(e.records_pending_proof(), 0);
    assert_eq!(e.est_kat_service_credit(), 0);
    assert!(!e.blocked_line().contains("KAT"), "blocked never earns");
}

// ── The caveat-#15 framing gate (adversarial) ───────────────────────────────

#[test]
fn no_dollar_sign_in_any_render() {
    let cases = [
        est(&[], &allow_policy(), &SyncWatermark::empty()),
        est(&two_rows(), &allow_policy(), &SyncWatermark::empty()),
        estimate_pending(
            &two_rows(),
            &RedactionGate::deny_all(),
            SpanOrigin::OwnRepo,
            &SyncWatermark::empty(),
        ),
    ];
    for e in cases {
        for line in [e.pending_line(), e.blocked_line()] {
            assert!(!line.contains('$'), "caveat #15 violated: {line}");
        }
    }
}

#[test]
fn accrual_line_carries_service_credit_if_proven_provisional() {
    let e = est(&two_rows(), &allow_policy(), &SyncWatermark::empty());
    let line = e.pending_line();
    assert!(line.contains("service credit"), "{line}");
    assert!(line.contains("if proven"), "{line}");
    assert!(line.contains("provisional"), "{line}");
    assert!(line.contains("KAT"), "{line}");
    assert!(
        line.contains("2"),
        "the record count travels with the figure"
    );
}

#[test]
fn blocked_line_carries_policy_hint_not_value() {
    let e = estimate_pending(
        &two_rows(),
        &RedactionGate::deny_all(),
        SpanOrigin::OwnRepo,
        &SyncWatermark::empty(),
    );
    let line = e.blocked_line();
    assert!(line.contains("2"), "{line}");
    assert!(line.contains(".refine-sync-policy"), "{line}");
    assert!(!line.contains("KAT"), "{line}");
    assert!(!line.contains('$'), "{line}");
}

#[test]
fn accrual_saturates_never_overflows() {
    let e = PendingEstimate {
        blocked: 0,
        pending_unsynced: u64::MAX,
        synced_awaiting_proof: 1,
    };
    assert_eq!(e.records_pending_proof(), u64::MAX);
    assert_eq!(e.est_kat_service_credit(), u64::MAX);
}

// ── Watermark persistence + recovery ────────────────────────────────────────

#[test]
fn watermark_save_load_round_trip() {
    let rows = two_rows();
    let wm = watermark_after_sync(&rows, &allow_policy());
    let dir = std::env::temp_dir().join(format!("kat_g4_wm_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(SyncWatermark::FILE_NAME);
    wm.save(&path).unwrap();
    let (loaded, warn) = SyncWatermark::load_at(&path);
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(loaded.sync_state_version, SYNC_STATE_VERSION);
    assert_eq!(loaded, wm);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_watermark_is_silent_bootstrap() {
    let dir = std::env::temp_dir().join(format!("kat_g4_none_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (wm, warn) = SyncWatermark::load_at(&dir.join("nope.json"));
    assert_eq!(wm, SyncWatermark::empty());
    assert!(
        warn.is_none(),
        "bootstrap is the normal state, not a warning"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn corrupt_watermark_recovers_empty_with_warning() {
    let dir = std::env::temp_dir().join(format!("kat_g4_bad_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(SyncWatermark::FILE_NAME);
    std::fs::write(&path, "{ definitely not json").unwrap();
    let (wm, warn) = SyncWatermark::load_at(&path);
    assert_eq!(wm, SyncWatermark::empty());
    let w = warn.expect("recovery must be surfaced");
    assert!(w.contains("re-pending"), "{w}");
    // And the recovered watermark estimates everything pending again.
    let e = est(&two_rows(), &allow_policy(), &wm);
    assert_eq!(e.pending_unsynced, 2);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn version_mismatch_recovers_empty_with_warning() {
    let dir = std::env::temp_dir().join(format!("kat_g4_ver_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(SyncWatermark::FILE_NAME);
    let future = format!(
        "{{\"sync_state_version\": {}, \"last_batch_commitment\": \"x\", \
         \"payload_version\": 1, \"synced_rows\": {{}}}}",
        SYNC_STATE_VERSION + 1
    );
    std::fs::write(&path, future).unwrap();
    let (wm, warn) = SyncWatermark::load_at(&path);
    assert_eq!(wm, SyncWatermark::empty());
    assert!(warn.is_some());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Determinism ─────────────────────────────────────────────────────────────

#[test]
fn same_inputs_identical_estimate_and_lines() {
    let rows = two_rows();
    let policy = allow_policy();
    let wm = watermark_after_sync(&rows, &policy);
    let a = est(&rows, &policy, &wm);
    let b = est(&rows, &policy, &wm);
    assert_eq!(a, b);
    assert_eq!(a.pending_line(), b.pending_line());
    assert_eq!(a.blocked_line(), b.blocked_line());
    assert_eq!(a.est_kat_service_credit(), b.est_kat_service_credit());
}

#[test]
fn input_order_does_not_change_counts_or_lines() {
    let rows = two_rows();
    let policy = allow_policy();
    let wm = watermark_after_sync(&rows, &policy);
    let mut shuffled = rows.clone();
    shuffled.reverse();
    let a = est(&rows, &policy, &wm);
    let b = est(&shuffled, &policy, &wm);
    assert_eq!(a, b);
    assert_eq!(a.pending_line(), b.pending_line());
}

#[test]
fn estimate_version_stamped_in_line() {
    let e = est(&two_rows(), &allow_policy(), &SyncWatermark::empty());
    assert!(
        e.pending_line().contains(&format!("v{ESTIMATE_VERSION}")),
        "the provisional disclaimer carries the estimate version"
    );
}
