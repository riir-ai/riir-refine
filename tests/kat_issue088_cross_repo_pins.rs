//! Issue 088 spin-out cross-repo pins — the assertions that became
//! cross-crate when the kat service plane split into `riir-kat` (protocol +
//! client plane) and riir-clippy (the healer-coupled remainder). Both sides
//! of each pin are only visible HERE, so this file is where they live.
//!
//! The sync-payload redact floor: `§7 step 4`'s `MIN_REDACT_VERSION` (now
//! `riir_kat::kat_protocol_wetrun::MIN_REDACT_VERSION`) is the v2 sync
//! envelope's `PAYLOAD_VERSION` — if the payload version moves, this floor
//! is the thing that decides whether an OLD payload is still pushable, so
//! the drift must be a deliberate edit. Originally a cfg'd unit test inside
//! `kat_wetrun` (gated `latent_retrieval`, where `kat_sync_payload` lived);
//! a cfg'd-out test is a green that measured nothing, so post-split the pin
//! runs UNCONDITIONALLY here — that is the upgrade, not a relocation. The
//! `crate::kat_sync_payload` path IS valid in an integration test — the
//! error above was a stale build artifact: integration tests see the lib
//! crate root, where the module is `#[cfg(feature = "kat_service")]-gated
//! + latent_retrieval`, matching this file's own whole-file gate.
#![cfg(all(feature = "kat_service", feature = "latent_retrieval"))]

/// Split the floor pin onto its own target so the gate is count-visible:
/// this file reports exactly 1 test under its feature combo (the Layer 5d
/// zero-test detector would name it if the gate ever went stale).
#[test]
fn the_redact_floor_matches_the_service_envelope_version() {
    assert_eq!(
        riir_kat::kat_protocol_wetrun::MIN_REDACT_VERSION,
        riir_clippy::kat_sync_payload::PAYLOAD_VERSION,
        "§7 step 4's floor is the v2 sync envelope's version — if the payload version \
         moves, this floor is the thing that decides whether an OLD payload is still \
         pushable, so the drift must be a deliberate edit here"
    );
}
