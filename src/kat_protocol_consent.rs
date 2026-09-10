//! The consent client area: the consent-heartbeat signing domain that
//! stamps every signed tier note.
//! Wire types for consent — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).

/// The signing domain — the first bytes of every consent signature. The
/// version suffix (`-v1`) is part of the pinned bytes: changing it is a
/// wire break against the service's own composer, not a tweak.
pub const CONSENT_SIGNING_DOMAIN: &[u8] = b"riir-kat-consent-v1";
