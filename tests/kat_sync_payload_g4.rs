//! KAT sync payload — G4 adversarial fixtures (Proposal 005 Phase 0 T0.3,
//! Plan 027).
//!
//! **G4 — zero secrets in any synced payload, by test.** The payload builder
//! is the serialization seam T0.2's gate exists to guard: planted,
//! format-valid (but fake) credentials in every gated field of a
//! [`TrajRowDoc`] must either exclude the row entirely (default policy) or
//! redact byte-exactly with fingerprints-only metadata (`allow_redacted`).
//! Clean rows in a mixed batch must survive byte-identical. The BLAKE3 batch
//! commitment must bind the row set (tamper detection). Builds must be
//! deterministic.
//!
//! The meta-gate scans the ROW LINES (not the header): the header's own
//! 64-hex `batch_commitment` is our cryptographic metadata — exactly the
//! documented fail-closed committed-hash class (Plan 026 §Honest scope); G4
//! is about USER CONTENT.
//!
//! ```bash
//! cargo test --features kat_service,latent_retrieval --test kat_sync_payload_g4 -- --nocapture
//! ```
//!
//! NOTE: the string literals below are DELIBERATELY secret-shaped (that is
//! the point). They are fake values constructed to match published token
//! formats — the gitleaks-fixture convention.

#![cfg(all(feature = "kat_service", feature = "latent_retrieval"))]

use riir_refine::kat_redact::{
    scan_secrets, BlockReason, RedactionGate, SpanOrigin, SyncPolicy, REDACT_VERSION,
};
use riir_refine::kat_sync_payload::{parse_rows, verify_payload, PayloadHeader, RowOutcome, SyncPayloadBuilder};
use riir_refine::traj_store::TrajRowDoc;

/// The shipped-policy shape (mirrors `riir-refine/.refine-sync-policy`).
fn shipped_policy() -> SyncPolicy {
    SyncPolicy {
        version: REDACT_VERSION,
        sync_enabled: true,
        allow_origins: vec![
            "own-repo".into(),
            "authored-fixture".into(),
            "mined-corpus".into(),
        ],
        allow_paths: Vec::new(),
        max_span_bytes: 65_536,
        allow_redacted: false,
    }
}

fn redacting_policy() -> SyncPolicy {
    SyncPolicy {
        allow_redacted: true,
        ..shipped_policy()
    }
}

/// The golden signing seed (`[42; 32]`) — the mirror of the lib's
/// `GOLDEN_SIGNING_SEED` (`src/kat_sync_payload.rs`). Integration tests
/// cannot see the lib's `#[cfg(test)]` helper, so the literal is mirrored
/// here — the documented cross-repo mirror convention (riir-dapps'
/// `tests/kat_mining.rs` mirrors the same seed).
fn golden_signer() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[42u8; 32])
}

fn row(lint_key: &str, shape: &str, fix: &str) -> TrajRowDoc {
    TrajRowDoc {
        lint_key: lint_key.into(),
        shape: shape.into(),
        fix: fix.into(),
        direction: vec![0.25, -0.25, 0.5, 0.0, -0.5, 0.75, -0.75, 1.0],
        success_count: 2,
        fail_count: 0,
        consecutive_fails: 0,
        tier: 0,
        last_run_id: "run-deadbeefcafe-7".into(),
        last_seen_at: 1_700_000_000,
        elo: 1200.0,
        span_elo: 1200.0,
        component: None,
        journey_runs: 0,
        journey_taint_sum: 0,
        src_path: "src/lib.rs".into(),
        rejected: Vec::new(),
    }
}

// ── Planted secrets in every gated field ───────────────────────────────────

#[test]
fn g4_planted_aws_key_in_shape_blocks_the_row() {
    let gate = RedactionGate::from_policy(shipped_policy());
    let secret = "AKIAIOSFODNN7EXAMPLE";
    let rows = vec![row(
        "identity_op",
        &format!("// setup\nlet k = \"{secret}\";\nfn rest() {{}}"),
        "fn rest() {}",
    )];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&rows, 1);
    assert_eq!(p.rows_included, 0, "secret-bearing row must not serialize");
    assert_eq!(p.rows_blocked, 1);
    match &p.outcomes[0] {
        RowOutcome::Blocked { reason, .. } => {
            assert!(matches!(reason, BlockReason::SecretsFound), "{reason}");
        }
        RowOutcome::Included { .. } => panic!("must block"),
    }
    assert!(
        !p.ndjson.contains(secret),
        "payload leaked the key:\n{}",
        p.ndjson
    );
    assert!(verify_payload(&p.ndjson).is_ok());
}

#[test]
fn g4_planted_github_token_in_fix_blocks_the_row() {
    let gate = RedactionGate::from_policy(shipped_policy());
    let secret = "ghp_16C7eBAF1q9XZKL3m0yv8u2oF4R6tYhUw";
    let rows = vec![row(
        "static_str",
        "fn token() -> String { \"x\".to_string() }",
        &format!("fn token() -> String {{ \"{secret}\".to_string() }}"),
    )];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&rows, 2);
    assert_eq!(p.rows_included, 0);
    assert!(
        !p.ndjson.contains(secret),
        "payload leaked the token:\n{}",
        p.ndjson
    );
}

#[test]
fn g4_secret_in_lint_key_blocks_the_row() {
    // Paranoid uniformity: EVERY text field is gated, including the rule key.
    let gate = RedactionGate::from_policy(shipped_policy());
    let secret = "xoxb-514685712-quiet";
    let rows = vec![row(secret, "let a = 1;", "let a = 1;")];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::MinedCorpus, &signer).build_at(&rows, 3);
    assert_eq!(p.rows_included, 0, "lint_key is a gated field");
    assert!(!p.ndjson.contains(secret));
}

#[test]
fn g4_secret_in_last_run_id_blocks_the_row() {
    let gate = RedactionGate::from_policy(shipped_policy());
    let secret = "glpat-AbCdEf0123456789AbCdEf0123456789";
    let mut r = row(
        "map_unwrap_or",
        "let v = x.map(f).unwrap_or(0);",
        "let v = x.map_or(0, f);",
    );
    r.last_run_id = format!("run-{secret}-1");
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&[r], 4);
    assert_eq!(p.rows_included, 0);
    assert!(!p.ndjson.contains(secret));
}

// ── Mixed batches ──────────────────────────────────────────────────────────

#[test]
fn g4_mixed_batch_clean_survive_blocked_absent() {
    let gate = RedactionGate::from_policy(shipped_policy());
    // 16-char AKIA body (the rule's minimum) — format-valid fake.
    let secret = "AKIAEXAMPLEKEY012345";
    let rows = vec![
        row(
            "collapse_if",
            "if a { if b { f(); } }",
            "if a && b { f(); }",
        ),
        row(
            "identity_op",
            &format!("let n = count * 1; // {secret}"),
            "let n = count;",
        ),
        row("needless_return", "fn g() { return 1; }", "fn g() { 1 }"),
    ];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&rows, 10);
    assert_eq!(p.rows_included, 2);
    assert_eq!(p.rows_blocked, 1);

    // The dirty row's bytes are nowhere; the clean rows survive byte-exact
    // (JSON-escaped form of the shape text is what ships — the clean shapes
    // contain no escapable chars beyond the braces serde leaves alone).
    assert!(!p.ndjson.contains(secret));
    assert!(
        p.ndjson.contains("if a && b { f(); }"),
        "clean fix must survive"
    );
    assert!(
        p.ndjson.contains("fn g() { return 1; }"),
        "clean shape must survive"
    );
    let parsed = parse_rows(&p.ndjson).unwrap();
    assert_eq!(parsed.len(), 2);
    assert!(parsed.iter().any(
        |r| r.shape.contains("if a { if b { f(); } }") && r.fix.contains("if a && b { f(); }")
    ));
    // None of the surviving rows carries the planted shape text.
    assert!(parsed.iter().all(|r| !r.shape.contains("count * 1")));

    // Header counts + commitment all consistent.
    let h = verify_payload(&p.ndjson).unwrap();
    assert_eq!(h.rows_included, 2);
    assert_eq!(h.rows_blocked, 1);
    // The blocked outcome identifies the row by its (clean-derivable) shape id.
    match &p.outcomes[1] {
        RowOutcome::Blocked { reason, .. } => {
            assert!(matches!(reason, BlockReason::SecretsFound));
        }
        RowOutcome::Included { .. } => panic!("row 1 must block"),
    }
}

// ── The redacted path ──────────────────────────────────────────────────────

#[test]
fn g4_allow_redacted_placeholders_and_fingerprints() {
    let gate = RedactionGate::from_policy(redacting_policy());
    let secret = "AKIAIOSFODNN7EXAMPLE";
    let dirty_shape = format!("// cfg\nlet key = \"{secret}\";\nfn tail() {{}}");
    let rows = vec![row("identity_op", &dirty_shape, "fn tail() {}")];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&rows, 20);
    assert_eq!(p.rows_included, 1);
    assert_eq!(p.rows_blocked, 0);

    let parsed = parse_rows(&p.ndjson).unwrap();
    assert_eq!(parsed.len(), 1);
    let r = &parsed[0];
    assert_eq!(r.gate.decision, "redacted");
    // Finding bytes gone; placeholder present; the clean halves byte-exact.
    assert!(!r.shape.contains(secret), "redaction failed:\n{}", r.shape);
    assert!(r.shape.contains("[REDACTED:aws-access-key]"));
    assert!(r.shape.starts_with("// cfg\n"));
    assert!(r.shape.ends_with("fn tail() {}"));
    assert_eq!(r.fix, "fn tail() {}");
    // Fingerprints recorded: kind + field + 12-hex, never bytes.
    assert_eq!(r.gate.findings.len(), 1);
    let f = &r.gate.findings[0];
    assert_eq!(f.field, "shape");
    assert_eq!(f.kind, "aws-access-key");
    assert_eq!(f.fingerprint.len(), 12);
    assert!(f.fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    // The outcome flags the redaction.
    assert!(matches!(
        &p.outcomes[0],
        RowOutcome::Included { redacted: true, .. }
    ));
    assert!(!p.ndjson.contains(secret));
    assert!(verify_payload(&p.ndjson).is_ok());
}

// ── Deny-by-default classes ────────────────────────────────────────────────

#[test]
fn g4_no_policy_denies_every_row() {
    let rows = vec![
        row("a", "let x = 1;", "let x = 1;"),
        row("b", "let y = 2;", "let y = 2;"),
    ];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&RedactionGate::deny_all(), SpanOrigin::OwnRepo, &signer)
        .build_at(&rows, 30);
    assert_eq!(p.rows_included, 0);
    assert_eq!(p.rows_blocked, 2);
    // Header-only payload; commitment over zero rows still verifies.
    let h = verify_payload(&p.ndjson).unwrap();
    assert_eq!(h.rows_included, 0);
    assert!(parse_rows(&p.ndjson).unwrap().is_empty());
}

#[test]
fn g4_external_origin_denies() {
    let gate = RedactionGate::from_policy(shipped_policy());
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::External, &signer)
        .build_at(&[row("a", "let x = 1;", "let x = 1;")], 31);
    assert_eq!(p.rows_blocked, 1);
    match &p.outcomes[0] {
        RowOutcome::Blocked { reason, .. } => {
            assert!(matches!(reason, BlockReason::OriginNotAllowed(_)));
        }
        RowOutcome::Included { .. } => panic!("external must deny"),
    }
}

// ── The commitment ─────────────────────────────────────────────────────────

#[test]
fn g4_commitment_binds_the_row_set() {
    let gate = RedactionGate::from_policy(shipped_policy());
    let rows = vec![
        row("a_rule", "let a = 1;", "let a = 1;"),
        row("b_rule", "let b = 2;", "let b = 2;"),
    ];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&rows, 40);
    assert!(verify_payload(&p.ndjson).is_ok());

    // Tamper one row byte (a counter), header untouched → mismatch.
    let mut lines: Vec<String> = p.ndjson.split('\n').map(str::to_string).collect();
    assert!(lines[1].contains("\"success_count\":2"));
    lines[1] = lines[1].replace("\"success_count\":2", "\"success_count\":9");
    let err = verify_payload(&lines.join("\n")).unwrap_err();
    assert!(err.contains("commitment mismatch"), "{err}");

    // Truncate the batch (header + first row only): the commitment covered
    // BOTH rows → must fail.
    let mut truncated: Vec<&str> = p.ndjson.lines().collect();
    truncated.truncate(2);
    let truncated = format!("{}\n", truncated.join("\n"));
    assert!(verify_payload(&truncated).is_err());
}

// ── Determinism ────────────────────────────────────────────────────────────

#[test]
fn g4_builds_are_deterministic() {
    let gate = RedactionGate::from_policy(shipped_policy());
    let signer = golden_signer();
    let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer);
    let rows = vec![
        row("zz_rule", "let z = 9;", "let z = 9;"),
        row("aa_rule", "let a = 1;", "let a = 1;"),
    ];
    let p1 = b.build_at(&rows, 50);
    let p2 = b.build_at(&rows, 50);
    assert_eq!(p1.ndjson, p2.ndjson, "same stamp must give identical bytes");
    // Input order must not move the bytes (sort by (lint_key, shape_id)).
    let mut shuffled = rows.clone();
    shuffled.reverse();
    assert_eq!(b.build_at(&shuffled, 50).ndjson, p1.ndjson);
    // The first row line is the aa_rule (sorted), not zz_rule (input-first).
    let first_row = p1.ndjson.lines().nth(1).unwrap_or_default();
    assert!(first_row.contains("aa_rule"));
}

// ── The aggregate meta-gate ────────────────────────────────────────────────

#[test]
fn g4_zero_secrets_in_row_lines_of_synced_payloads() {
    let gate = RedactionGate::from_policy(shipped_policy());
    let rows = vec![
        row(
            "uninlined_format_args",
            "fn f() { println!(\"{}\", x); }",
            "fn f() { println!(\"{x}\"); }",
        ),
        row("needless_return", "fn g() { return 1; }", "fn g() { 1 }"),
        row(
            "static_str",
            "fn s() -> &'static str { \"ok\" }",
            "fn s() -> &'static str { \"ok\" }",
        ),
    ];
    let signer = golden_signer();
    let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&rows, 60);
    assert_eq!(p.rows_included, 3);
    // G4 over USER CONTENT: scan the row lines only — the header's own
    // 64-hex commitment is our metadata (the documented fail-closed class).
    let row_lines = p.ndjson.split_once('\n').map_or("", |(_, rest)| rest);
    assert!(
        scan_secrets(row_lines).is_empty(),
        "synced row lines must be scan-clean:\n{row_lines}"
    );
}

// ── T12b: the network-bound signing arm (riir-kat Issue 003, site 12) ─────

#[test]
fn t12b_ledger_id_arms_the_v3_splice_and_none_keeps_v2_bytes() {
    use riir_refine::kat_sync_payload::{SYNC_SIGNING_DOMAIN, SYNC_SIGNING_DOMAIN_V3};

    let gate = RedactionGate::from_policy(shipped_policy());
    let rows = vec![row("needless_return", "fn g() { return 1; }", "fn g() { 1 }")];
    let signer = golden_signer();
    let id = [0xD8u8; 32];

    // The DEFAULT build is byte-identical to the pre-T12b shape (the
    // v1 fixture freeze pins exactly this — the freeze stays green).
    let legacy = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build_at(&rows, 7);
    let head = legacy.ndjson.lines().next().unwrap();
    assert!(
        !head.contains("ledger_id"),
        "the wire header is UNCHANGED — the ledger id never rides the payload"
    );

    // The armed build: same header SHAPE, signature under the `-v3` splice.
    let bound = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer)
        .with_ledger_id(Some(id))
        .build_at(&rows, 7);
    assert_eq!(
        bound.ndjson.lines().next().unwrap().split("\"signature_hex\"").next().unwrap(),
        head.split("\"signature_hex\"").next().unwrap(),
        "identical bytes up to the signature (same commitment, same header)"
    );
    assert_ne!(legacy.ndjson, bound.ndjson, "the signatures differ (different domains)");

    // THE SPLICE LAW at the sync site: v3-message = v3-domain ‖ id ‖ v2-tail.
    // Verify through the SIGNED payload: parse the header (the wire shape is
    // unchanged) and check the signature over the reconstructed splice —
    // exactly what the dapps mirror does server-side with the entry-injected
    // id. (refine's own `verify_payload` stays a legacy-only self-check; no
    // production path self-verifies a v3 payload — build posts, the SERVER
    // verifies.)
    let head = bound.ndjson.lines().next().unwrap();
    let h: PayloadHeader = serde_json::from_str(head).expect("the wire header parses");
    let v2_msg = {
        // Rebuild the legacy message from the parsed header (the mirror of
        // dapps `signing_message`): domain ‖ scalars ‖ commitment.
        let commitment = hex_to_32(&h.batch_commitment).expect("64-hex");
        let mut m = Vec::new();
        m.extend_from_slice(SYNC_SIGNING_DOMAIN);
        m.extend_from_slice(&h.payload_version.to_le_bytes());
        m.extend_from_slice(&h.redact_version.to_le_bytes());
        m.extend_from_slice(&h.created_unix.to_le_bytes());
        m.extend_from_slice(&h.rows_included.to_le_bytes());
        m.extend_from_slice(&h.rows_blocked.to_le_bytes());
        m.extend_from_slice(&(u32::try_from(h.origin.len()).unwrap_or(u32::MAX)).to_le_bytes());
        m.extend_from_slice(h.origin.as_bytes());
        m.extend_from_slice(&commitment);
        m
    };
    let v3_msg = riir_refine::kat_protocol_v2::ledger_bound(
        SYNC_SIGNING_DOMAIN_V3,
        &id,
        SYNC_SIGNING_DOMAIN,
        &v2_msg,
    );
    use ed25519_dalek::{Signature, Verifier as _};
    let vk = signer.verifying_key();
    let sig_bytes = hex_to_64(&h.signature_hex).expect("128-hex");
    vk.verify(&v3_msg, &Signature::from_bytes(&sig_bytes))
        .expect("the signature is over the -v3 splice of the v2 message");
    // And the legacy build still verifies under the legacy domain only.
    let head2 = legacy.ndjson.lines().next().unwrap();
    let h2: PayloadHeader = serde_json::from_str(head2).expect("the wire header parses");
    let sig2 = hex_to_64(&h2.signature_hex).expect("128-hex");
    vk.verify(&v2_msg, &Signature::from_bytes(&sig2))
        .expect("the legacy signature is over the -v2 message");
}

/// Hex-decode 64 chars into 32 bytes (the test-side mirror of
/// `decode_hex32`; the lib helper is private, and duplicating eight lines
/// in a test beats widening the API for it).
fn hex_to_32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn hex_to_64(s: &str) -> Option<[u8; 64]> {
    if s.len() != 128 {
        return None;
    }
    let mut out = [0u8; 64];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
