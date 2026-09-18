//! Issue 088 spin-out cross-repo pins — the assertions that became
//! cross-crate when the kat service plane split into `riir-kat` (protocol +
//! client plane) and riir-clippy (the healer-coupled remainder). Both sides
//! of each pin are only visible HERE, so this file is where they live.
//!
//! The sync-payload redact floor: `§7 step 4`'s `MIN_REDACT_VERSION` (now
//! `riir_kat::kat_protocol_wetrun::MIN_REDACT_VERSION`) gates whether a
//! payload is pushable at all. It used to be pinned == `PAYLOAD_VERSION` —
//! true while both were 2, a coincidence that masked a real drift: the
//! service floor (and the gate version) moved to 3 at Bench 053 and the
//! client mirror stayed at 2, so the wetrun would push redact-2 payloads the
//! service refuses. The floor tracks the GATE version (`REDACT_VERSION`, the
//! value it is a floor ON), not the payload schema version (which moved to 3
//! independently at Issue 103) — so the pin now asserts exactly that.
//! Originally a cfg'd unit test inside `kat_wetrun` (gated
//! `latent_retrieval`, where `kat_sync_payload` lived); a cfg'd-out test is a
//! green that measured nothing, so post-split the pin runs UNCONDITIONALLY
//! here — that is the upgrade, not a relocation.
#![cfg(all(feature = "kat_service", feature = "latent_retrieval"))]

/// Split the floor pin onto its own target so the gate is count-visible:
/// this file reports exactly 1 test under its feature combo (the Layer 5d
/// zero-test detector would name it if the gate ever went stale).
#[test]
fn the_redact_floor_matches_the_service_gate_version() {
    assert_eq!(
        riir_kat::kat_protocol_wetrun::MIN_REDACT_VERSION,
        riir_kat::kat_redact::REDACT_VERSION,
        "§7 step 4's floor tracks the CURRENT GATE version — the wetrun must \
         refuse to push exactly what the service would refuse (the service \
         floor rides the same gate version). A drift here means the wetrun \
         pushes payloads the ingest 400s; move BOTH sides deliberately."
    );
}
