//! KAT redaction gate — G4 adversarial fixtures (Proposal 005 Phase 0 T0.2,
//! Plan 026).
//!
//! **G4 — zero secrets in any synced payload, by test.** Planted, format-
//! valid (but fake) credentials in the shapes the proposal names — AWS keys,
//! private keys, connection strings — plus the `.env` leakage class,
//! credential-shaped Rust literals, high-entropy blobs, and mixed
//! user-code spans. Every fixture asserts one of the two sanctioned
//! outcomes:
//!
//! - **Blocked** (deny-by-default origin/path/size or `allow_redacted:
//!   false`): the span never reaches a sync batch.
//! - **Redacted byte-exactly**: everything outside the finding ranges is
//!   byte-identical to the input; the finding bytes are gone; the
//!   placeholder is present.
//!
//! The log gate: a recorded block carries the finding *fingerprint*, never
//! the finding bytes — the log cannot leak what [`Finding`] never stored.
//!
//! ```bash
//! cargo test --features kat_service --test kat_redact_g4 -- --nocapture
//! ```
//!
//! NOTE: the string literals below are DELIBERATELY secret-shaped (that is
//! the point). They are fake values constructed to match published token
//! formats — the gitleaks-fixture convention.

#![cfg(feature = "kat_service")]

use riir_clippy::kat_redact::{
    glob_match, redact, scan_secrets, BlockReason, FindingKind, GateDecision, RedactionGate,
    RedactionLog, SpanOrigin, SyncPolicy, POLICY_FILE_NAME, REDACT_CHANGELOG, REDACT_VERSION,
};

/// The shipped-repo policy shape (mirrors `riir-clippy/.refine-sync-policy`
/// without reading the file — the test pins the SHAPE, the repo pins the
/// instance).
fn shipped_policy() -> SyncPolicy {
    SyncPolicy {
        version: REDACT_VERSION,
        sync_enabled: true,
        allow_origins: vec![
            "own-repo".into(),
            "authored-fixture".into(),
            "mined-corpus".into(),
        ],
        allow_paths: vec![
            "src/**/*.rs".into(),
            "tests/**/*.rs".into(),
            "benches/**/*.rs".into(),
            "examples/**/*.rs".into(),
            "**/*.wgsl".into(),
        ],
        max_span_bytes: 65_536,
        allow_redacted: false,
    }
}

fn gate() -> RedactionGate {
    RedactionGate::from_policy(shipped_policy())
}

/// Assert the G4 core property over a redaction result: every finding's
/// original byte window is absent from the payload, and the bytes outside
/// the (de-overlapped) finding ranges are byte-identical to the input.
fn assert_g4_byte_exact(
    original: &str,
    findings: &[riir_clippy::kat_redact::Finding],
    payload: &str,
) {
    // 1. No finding's original bytes survive.
    for f in findings {
        let window = &original[f.start..f.start + f.len];
        assert!(
            !payload.contains(window),
            "G4 VIOLATION: finding bytes {window:?} survived redaction"
        );
    }
    // 2. Byte-exact outside ranges (de-overlapped, same rule as `redact`).
    let mut kept: Vec<&riir_clippy::kat_redact::Finding> = findings.iter().collect();
    kept.sort_by_key(|f| (f.start, std::cmp::Reverse(f.len)));
    let mut cursor = 0;
    let mut chosen: Vec<(usize, usize, String)> = Vec::new();
    for f in kept {
        if f.start >= cursor {
            chosen.push((f.start, f.start + f.len, f.placeholder()));
            cursor = f.start + f.len;
        }
    }
    let mut expected = String::with_capacity(original.len());
    let mut at = 0;
    for (s, e, ph) in &chosen {
        expected.push_str(&original[at..*s]);
        expected.push_str(ph);
        at = *e;
    }
    expected.push_str(&original[at..]);
    assert_eq!(
        payload, &expected,
        "redaction must be byte-exact outside ranges"
    );
}

// ── The named adversarial classes ──────────────────────────────────────────

#[test]
fn g4_aws_access_key_blocks_and_redacts_byte_exactly() {
    let key = "AKIAIOSFODNN7EXAMPLE";
    let span = format!("// config\nlet key = \"{key}\";\nfn rest() {{}}\n");
    let d = gate().check(SpanOrigin::AuthoredFixture, None, &span);
    assert!(matches!(
        d,
        GateDecision::Blocked {
            reason: BlockReason::SecretsFound,
            ..
        }
    ));
    let findings = scan_secrets(&span);
    assert!(findings.iter().any(|f| f.kind == FindingKind::AwsAccessKey));
    let payload = redact(&span, &findings);
    assert_g4_byte_exact(&span, &findings, &payload);
}

#[test]
fn g4_aws_secret_via_env_shape_blocks() {
    let span = "AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMIK7MDENGbPxRfiCYEXAMPLEKEY0\n";
    let d = gate().check(SpanOrigin::MinedCorpus, None, span);
    assert!(!d.is_syncable(), "{d:?}");
    let findings = scan_secrets(span);
    assert!(findings
        .iter()
        .any(|f| f.kind == FindingKind::EnvSecretAssign));
    let payload = redact(span, &findings);
    assert_g4_byte_exact(span, &findings, &payload);
}

#[test]
fn g4_github_token_blocks() {
    let span = "let hook = \"ghp_16C7eBAF1q9XZKL3m0yv8u2oF4R6tYhUwz\";";
    assert!(!gate()
        .check(SpanOrigin::OwnRepo, Some("src/a.rs"), span)
        .is_syncable());
}

#[test]
fn g4_private_key_block_blocks_and_covers_whole_block() {
    let span = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAABFwAAAAdzc2gtcn\n-----END OPENSSH PRIVATE KEY-----\n";
    let d = gate().check(SpanOrigin::OwnRepo, Some("src/key.rs"), span);
    assert!(!d.is_syncable());
    let findings = scan_secrets(span);
    let k = findings
        .iter()
        .find(|f| f.kind == FindingKind::PrivateKeyBlock)
        .unwrap();
    let covered = &span[k.start..k.start + k.len];
    assert!(covered.ends_with("-----END OPENSSH PRIVATE KEY-----"));
    let payload = redact(span, &findings);
    assert_g4_byte_exact(span, &findings, &payload);
    assert!(
        !payload.contains("b3BlbnNzaC1rZXktdjE"),
        "key BODY must not survive"
    );
}

#[test]
fn g4_connection_string_password_blocks() {
    let span = "let db = \"postgres://svc_app:S3cretPass@db.prod.internal:5432/orders\";";
    assert!(!gate()
        .check(SpanOrigin::MinedCorpus, None, span)
        .is_syncable());
    let findings = scan_secrets(span);
    assert!(findings
        .iter()
        .any(|f| f.kind == FindingKind::ConnStringCreds));
    let payload = redact(span, &findings);
    assert_g4_byte_exact(span, &findings, &payload);
}

#[test]
fn g4_env_file_leakage_blocks() {
    let span = "# .env\nDATABASE_PASSWORD=hunter2-sturdy-pass\nREDIS_URL=redis://admin:fallback9@cache:6379\n";
    let d = gate().check(SpanOrigin::External, None, span);
    assert!(!d.is_syncable());
    // Even from an allowed origin the content blocks.
    let d2 = gate().check(SpanOrigin::OwnRepo, Some("src/config.rs"), span);
    assert!(matches!(
        d2,
        GateDecision::Blocked {
            reason: BlockReason::SecretsFound,
            ..
        }
    ));
    let findings = scan_secrets(span);
    let payload = redact(span, &findings);
    assert_g4_byte_exact(span, &findings, &payload);
    assert!(!payload.contains("hunter2-sturdy-pass"));
}

#[test]
fn g4_credential_shaped_rust_literal_blocks() {
    let span = "const API_KEY: &str = \"live_sk_9f8e7d6c5b4a3210fedcba9876543210\";\n";
    let d = gate().check(SpanOrigin::AuthoredFixture, None, span);
    assert!(!d.is_syncable());
    assert!(scan_secrets(span)
        .iter()
        .any(|f| f.kind == FindingKind::RustSecretLiteral));
}

#[test]
fn g4_high_entropy_blobs_block() {
    let hex = "9f2c7a4e1b8d3f6a0c5e2b9d7f4a1c8e3b6d9f2c7a4e1b8d3f6";
    let b64 = "qP7mZ2xQvN4kR9tLwY3jH6sD1fG5aX0bE8cU2iO4pT7nM9rK";
    for blob in [hex, b64] {
        let span = format!("let digest = \"{blob}\";");
        assert!(
            !gate()
                .check(SpanOrigin::MinedCorpus, None, &span)
                .is_syncable(),
            "entropy blob must block: {blob}"
        );
    }
}

#[test]
fn g4_bearer_and_jwt_shaped_tokens_block() {
    let span = "req.header(\"Authorization\", \"Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig\");";
    assert!(!gate()
        .check(SpanOrigin::OwnRepo, Some("src/api.rs"), span)
        .is_syncable());
    assert!(scan_secrets(span)
        .iter()
        .any(|f| f.kind == FindingKind::BearerToken));
}

// ── Mixed user-code spans ──────────────────────────────────────────────────

#[test]
fn g4_mixed_span_clean_code_survives_secret_stripped() {
    let secret = "AKIAIOSFODNN7EXAMPLE";
    let span = format!(
        "// generated by setup.sh\npub fn clean_one(x: u32) -> u32 {{ x + 1 }}\n\
         const ACCESS_KEY_ID: &str = \"{secret}\";\n\
         pub fn clean_two(x: u32) -> u32 {{ x * 2 }}\n"
    );
    let findings = scan_secrets(&span);
    assert!(!findings.is_empty());
    let payload = redact(&span, &findings);
    assert_g4_byte_exact(&span, &findings, &payload);
    assert!(payload.contains("pub fn clean_one(x: u32) -> u32 { x + 1 }"));
    assert!(payload.contains("pub fn clean_two(x: u32) -> u32 { x * 2 }"));
    assert!(!payload.contains(secret));
    assert!(payload.contains("[REDACTED:"));
}

#[test]
fn g4_user_code_span_blocked_by_origin_regardless_of_content() {
    // Deny-by-default: `external` is not on the shipped allowlist, so even a
    // pristine span from user-submitted code never syncs.
    let pristine = "pub fn add(a: u32, b: u32) -> u32 { a + b }\n";
    let d = gate().check(SpanOrigin::External, None, pristine);
    assert!(matches!(
        d,
        GateDecision::Blocked {
            reason: BlockReason::OriginNotAllowed(SpanOrigin::External),
            ..
        }
    ));
}

// ── Deny-by-default mechanics ──────────────────────────────────────────────

#[test]
fn g4_no_policy_file_denies_all() {
    let dir = std::env::temp_dir().join(format!("kat_g4_nopolicy_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // Point at a policy path that does not exist → deny-all + warning.
    let loaded = RedactionGate::load(&dir.join(POLICY_FILE_NAME));
    assert!(loaded.warning.is_some());
    assert!(!loaded
        .gate
        .check(SpanOrigin::OwnRepo, Some("src/a.rs"), "fn f() {}")
        .is_syncable());
    std::fs::remove_dir_all(&dir).ok();
}

/// **The shipped policy FILE must load at the current gate version.**
///
/// `REDACT_VERSION` mismatch is fail-closed by design: a policy written for
/// another gate version yields `deny_all()` + a warning. That is the correct
/// safety behavior and also a silent-breakage trap — bumping
/// `REDACT_VERSION` without migrating `.refine-sync-policy` in the same commit
/// turns every sync into a deny-all, and nothing else in the suite notices,
/// because every other test constructs its policy from `REDACT_VERSION`
/// (so they follow the bump automatically) or writes a temp fixture.
///
/// This test reads the REPO'S ACTUAL FILE — the one instance the "test pins
/// the SHAPE, the repo pins the instance" split otherwise leaves unpinned.
/// Added 2026-08-28 after the `REDACT_VERSION` 1 → 2 bump (Bench 052) had to
/// migrate the file by hand with no gate to catch a miss.
#[test]
fn g4_shipped_policy_file_loads_at_the_current_gate_version() {
    // CARGO_MANIFEST_DIR so the test is cwd-independent.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(POLICY_FILE_NAME);
    assert!(
        path.is_file(),
        "the shipped policy must exist at {}",
        path.display()
    );

    let loaded = RedactionGate::load(&path);
    assert!(
        loaded.warning.is_none(),
        "the shipped .refine-sync-policy failed to load cleanly — a          REDACT_VERSION bump almost certainly landed without migrating the          file, which silently deny-alls every sync: {:?}",
        loaded.warning
    );

    // And it is genuinely usable: an in-policy span syncs, proving the load
    // produced the real gate rather than `deny_all()`.
    assert!(
        loaded
            .gate
            .check(SpanOrigin::OwnRepo, Some("src/kat_redact.rs"), "fn f() {}")
            .is_syncable(),
        "a clean own-repo span on an allowed path must sync under the          shipped policy — if it does not, the file loaded as deny-all"
    );
    // The shipped posture itself, pinned against accidental widening: user
    // code stays denied on ORIGIN (Bench 052 — this policy line is the
    // control that protects consumer code, not the pattern set).
    assert!(
        !loaded
            .gate
            .check(SpanOrigin::External, None, "fn f() {}")
            .is_syncable(),
        "`external` must remain absent from the shipped allow_origins"
    );
}

#[test]
fn g4_path_not_on_allowlist_blocks() {
    let d = gate().check(
        SpanOrigin::OwnRepo,
        Some("private/keys/prod.rs"),
        "fn f() {}",
    );
    assert!(matches!(
        d,
        GateDecision::Blocked {
            reason: BlockReason::PathNotAllowed(_),
            ..
        }
    ));
}

#[test]
fn g4_oversize_span_blocks() {
    let policy = SyncPolicy {
        max_span_bytes: 32,
        ..shipped_policy()
    };
    let gate = RedactionGate::from_policy(policy);
    let span = "fn main() { println!(\"way more than thirty-two bytes total\"); }";
    assert!(matches!(
        gate.check(SpanOrigin::OwnRepo, None, span),
        GateDecision::Blocked {
            reason: BlockReason::SpanTooLarge { .. },
            ..
        }
    ));
}

#[test]
fn g4_clean_authored_fixture_allows() {
    let span = "pub fn f(v: &[u8]) -> usize { v.len() }";
    assert_eq!(
        gate().check(SpanOrigin::AuthoredFixture, None, span),
        GateDecision::Allow
    );
}

#[test]
fn g4_clean_rust_is_not_secret_shaped() {
    // Realistic clean code — identifiers, generics, paths, ordinary strings.
    // Zero findings is the false-positive contract (fail-closed must not
    // mean fail-useless).
    let span = r#"use std::collections::HashMap;
pub struct Config { pub name: String, pub retries: u8 }
impl Config {
    pub fn lookup(map: &HashMap<String, Config>, name: &str) -> Option<&Config> {
        map.get(name).filter(|c| c.retries > 0)
    }
}
pub const GREETING: &str = "hello, world — have a nice day";
"#;
    assert_eq!(
        scan_secrets(span),
        Vec::new(),
        "clean Rust must not trip the scanner"
    );
    assert!(gate()
        .check(SpanOrigin::OwnRepo, Some("src/config.rs"), span)
        .is_syncable());
}

// ── The log ────────────────────────────────────────────────────────────────

#[test]
fn g4_log_never_carries_finding_bytes() {
    let dir = std::env::temp_dir().join(format!("kat_g4_log_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let log = RedactionLog::at(dir.join("log.jsonl"));
    let secret = "AKIAIOSFODNN7EXAMPLE";
    let span = format!("const KEY: &str = \"{secret}\";");
    let d = gate().check(SpanOrigin::AuthoredFixture, None, &span);
    assert!(!d.is_syncable());
    log.record(
        &d,
        SpanOrigin::AuthoredFixture,
        "reachable:test",
        "abcdef012345",
    )
    .unwrap();
    // Also record an allow + a redacted row to pin the event vocabulary.
    log.record(
        &GateDecision::Allow,
        SpanOrigin::AuthoredFixture,
        "reachable:ok",
        "123456abcdef",
    )
    .unwrap();
    let text = std::fs::read_to_string(log.path()).unwrap();
    assert!(!text.contains(secret), "log leaked finding bytes:\n{text}");
    assert!(!text.contains("hunter"), "sanity");
    assert!(text.contains("\"event\":\"blocked\""));
    assert!(text.contains("\"event\":\"allow\""));
    assert!(text.contains("aws-access-key"));
    assert!(text.contains("abcdef012345"));
    std::fs::remove_dir_all(&dir).ok();
}

// ── Determinism + versioning ───────────────────────────────────────────────

#[test]
fn g4_scan_is_bit_stable_across_runs() {
    let span = "PASSWORD=hunter2sturdy\nlet t = \"ghp_16C7eBAF1q9XZKL3m0yv8u2oF4R6tYhUw\";\n";
    let first = scan_secrets(span);
    assert!(first.len() >= 2);
    for _ in 0..100 {
        assert_eq!(scan_secrets(span), first);
    }
}

#[test]
fn g4_version_changelog_is_complete() {
    for v in 1..=REDACT_VERSION {
        assert!(
            REDACT_CHANGELOG.iter().any(|(cv, _)| *cv == v),
            "version {v} has no changelog row"
        );
    }
}

#[test]
fn g4_policy_globs_behave() {
    // The shipped allow_paths semantics, pinned.
    assert!(glob_match("src/**/*.rs", "src/kat_redact.rs"));
    assert!(glob_match("tests/**/*.rs", "tests/kat_redact_g4.rs"));
    assert!(!glob_match("src/**/*.rs", "target/debug/x.rs"));
    assert!(glob_match("**/*.wgsl", "kernels/gemv.wgsl"));
}

// ── The aggregate meta-gate ────────────────────────────────────────────────

#[test]
fn g4_zero_secrets_in_any_synced_payload() {
    // Every fixture payload this gate marks syncable (Allow) or redacted,
    // re-checked: none may contain any finding bytes from its original.
    let cases: Vec<(&str, SpanOrigin, Option<&str>)> = vec![
        (
            "pub fn f(v: &[u8]) -> usize { v.len() }",
            SpanOrigin::AuthoredFixture,
            None,
        ),
        (
            "impl Display for Foo { fn fmt(&self, f: &mut Formatter) -> fmt::Result { Ok(()) } }",
            SpanOrigin::OwnRepo,
            Some("src/foo.rs"),
        ),
        (
            "let mut total: u64 = 0; for row in rows { total += row.len() as u64; }",
            SpanOrigin::MinedCorpus,
            None,
        ),
    ];
    for (span, origin, path) in cases {
        let d = gate().check(origin, path, span);
        assert!(d.is_syncable(), "clean fixture must pass: {span:?}");
        let payload = match d {
            GateDecision::Allow => span.to_string(),
            GateDecision::Redacted { payload, .. } => payload,
            GateDecision::Blocked { .. } => unreachable!("checked above"),
        };
        assert_eq!(
            scan_secrets(&payload),
            Vec::new(),
            "synced payload must be scan-clean"
        );
    }
}
