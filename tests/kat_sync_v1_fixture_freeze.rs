//! T13 v1 fixture freeze — site 18 of the `riir-kat/.issues/003` derived
//! signature-site matrix: the mining sync payload's canonical signing
//! message (`PayloadHeader::signing_message`).
//!
//! riir-kat's freeze (`src/kat_protocol_freeze.rs`) pins the other 17 sites;
//! THIS composer is dual-pinned refine ↔ riir-dapps (Issue 082 — no cargo
//! dep between the repos; the golden payload is the byte-level drift
//! tripwire), so its frozen bytes live on the canonical client side, HERE.
//!
//! The frozen header inputs are FIXED literals — deliberately NOT the live
//! `PAYLOAD_VERSION`/`REDACT_VERSION` constants — so this pin decouples the
//! COMPOSER from unrelated version bumps: the frozen bytes stay frozen until
//! T12's v2 domain composer lands beside them. (A bump that changes the
//! composer's OUTPUT for these inputs would red this test — the same
//! deliberate re-pin discipline as `golden_payload_bytes_are_pinned`.)
//!
//! The signature verify arm is T13's three-way matrix in riir-dapps (which
//! carries the ed25519 dep); this freeze pins the MESSAGE bytes.

use riir_refine::kat_sync_payload::{PayloadHeader, SYNC_SIGNING_DOMAIN};

/// The frozen pre-T12 signing message for the fixed header below
/// (payload v4 / redact v5 / created 1_700_000_000 / 3+1 rows /
/// origin "own-repo" / commitment 0xab×32) — 92 bytes.
const FROZEN: &[u8] = &[
    0x72, 0x69, 0x69, 0x72, 0x2d, 0x6b, 0x61, 0x74, 0x2d, 0x73, 0x79, 0x6e, 0x63, 0x2d, 0x70, 0x61,
    0x79, 0x6c, 0x6f, 0x61, 0x64, 0x2d, 0x76, 0x32, 0x04, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
    0x00, 0xf1, 0x53, 0x65, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x00, 0x00, 0x6f, 0x77, 0x6e, 0x2d, 0x72, 0x65, 0x70, 0x6f, 0xab, 0xab, 0xab, 0xab,
    0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab,
    0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab,
];

fn frozen_header() -> PayloadHeader {
    PayloadHeader {
        payload_version: 4,
        redact_version: 5,
        origin: "own-repo".to_string(),
        created_unix: 1_700_000_000,
        rows_included: 3,
        rows_blocked: 1,
        batch_commitment: "ab".repeat(32),
        account_pubkey_hex: String::new(),
        signature_hex: String::new(),
    }
}

#[test]
fn sync_signing_message_v1_frozen() {
    let msg = frozen_header()
        .signing_message()
        .expect("commitment decodes");
    assert_eq!(msg.len(), 92);
    assert!(
        msg.starts_with(SYNC_SIGNING_DOMAIN),
        "the -v2 domain spelling is load-bearing (Issue 103) — a rename is a wire break"
    );
    assert_eq!(msg.as_slice(), FROZEN, "the sync signing-message composer drifted from the frozen pre-T12 bytes — if this is the T12 v2 cutover, freeze the v2 bytes in a NEW test beside this one");
}
