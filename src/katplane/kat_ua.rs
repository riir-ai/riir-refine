//! The transports' `User-Agent` — VENDORED from `riir-kat` `kat_ua.rs`
//! (issue 221 P2: the one UNGATED `riir_kat` use left after the P2a
//! vendoring — `main()` installs the consumer tag at every posture).
//!
//! Vendored slice: the consumer-tag install + the header value shape over
//! the already-vendored [`crate::kat_net_pin::NET_PIN`]. The ureq `apply`
//! seam, the TOTP factor tail, the T2-route tier test and the signer hint
//! stay riir-kat-side — their consumers (`kat_migrate_client`,
//! `kat_corpus_pr`) are `kat_transport`/`kat_service`-gated and keep
//! resolving `riir_kat::kat_ua` there.
//!
//! Provenance: `riir-kat` `863c590798b5` (the P2a vendoring pin), with the
//! same drift discipline as the rest of `katplane/` —
//! `scripts/vendor_katplane.sh --check` diffs this file against the sibling
//! module, and `tests/katplane_parity.rs` pins the header-value shape.
//!
//! The UA is diagnostic, never protection: it is self-reported.

use std::sync::OnceLock;

/// The consumer tag (e.g. `cargo-refine/0.4.8 (build 8cbdf4db0)`),
/// installed once per process by the dialing binary. First install wins —
/// later calls are no-ops (one process speaks with one voice).
static CONSUMER_TAG: OnceLock<String> = OnceLock::new();

/// Install the dialing binary's identity. Idempotent (first wins).
pub fn install_consumer_tag(tag: impl Into<String>) {
    let _ = CONSUMER_TAG.set(tag.into());
}

/// The UA's net slot: the first word of [`NET_PIN`] — `mainnet`,
/// `devnet`, `multi`, or `conflict`. A header value must be visible
/// ASCII; the full conflict sentence (em-dash included) would make ureq
/// refuse the dial before the pin's own runtime refusal could run.
fn net_slot() -> &'static str {
    let pin = crate::kat_net_pin::NET_PIN;
    let end = pin.find([':', ' ', '(']).unwrap_or(pin.len());
    &pin[..end]
}

/// The `User-Agent` header value every kat transport sends:
/// `<consumer>; riir-kat/<version>; net/<NET_PIN>` (the consumer part
/// reads `unidentified` when nothing was installed).
#[must_use]
pub fn header_value() -> String {
    format!(
        "{}; riir-kat/{}; net/{}",
        CONSUMER_TAG.get().map_or("unidentified", String::as_str),
        env!("CARGO_PKG_VERSION"),
        net_slot()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value names the crate, the net posture, and a consumer slot —
    /// the shape the server-side straggler count parses. Also proves the
    /// one-voice law: the second install in the SAME test is a no-op (the
    /// slot is process-global, so tests must never race it with distinct
    /// expectations — one test owns the slot's whole lifecycle).
    #[test]
    fn header_value_shape_and_first_wins() {
        install_consumer_tag("cargo-refine/0.4.8 (build test)");
        let v = header_value();
        assert!(
            v.starts_with("cargo-refine/0.4.8 (build test); riir-kat/"),
            "{v}"
        );
        let net = v.rsplit("net/").next().unwrap_or("");
        assert!(
            matches!(net, "mainnet" | "devnet" | "multi" | "conflict"),
            "net slot must be a bare word, got {net:?}"
        );
        install_consumer_tag("challenger-should-lose");
        assert!(header_value().starts_with("cargo-refine/0.4.8 (build test);"));
    }
}
