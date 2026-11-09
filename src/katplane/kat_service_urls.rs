//! Vendored from `riir-kat` (`863c590798b5`, 2026-11-09, issue 221
//! private-dep minimization): the feature-free protocol tier refine's
//! UNGATED surfaces consume (secret scan, units, consent dir, config
//! store, hosts, net pin, state stamp, soft gate). std-only by the
//! leaf's protocol-tier purity rule. The canonical copy stays in the
//! private sibling (`riir-kat` `src/kat_service_urls.rs`); drift is caught by the
//! golden pins in `tests/katplane_parity.rs` and repaired by
//! re-vendoring via `scripts/vendor_katplane.sh` — never hand-edit one
//! side.

//! The pinned KAT service hosts — the one place the network's deployment
//! URLs are named. Both the client transports and the machine-local
//! `kat_service_config` (the Solana-CLI-model `config set --url` surface)
//! consume these, so a host can never drift between the config tier and the
//! transport tier.
//!
//! Feature-free: naming a host is not behavior. Two API hosts (riir-dapps
//! Plan 047): `mainnet.gist.rs` (the apex worker's ledger) and
//! `devnet.gist.rs` (the devnet worker); the testnet deployment is retired.
//! The client failover PAIR is SAME-LEDGER only — a host falls back to its
//! own legacy alias ([`LEGACY_SERVICE_URLS`]), never to the other network:
//! a DEV client must never fail over onto MAINNET and vice versa
//! (switching networks is a config change, not a failover;
//! `kat_lease_client::fallback_pair_for` is the one seam — 2026-10-07 fix
//! of the old default↔devnet map, which violated this law in code).

/// THE mainnet API host — the gen-2 ledger's API host `mainnet.riir.ai`
/// (riir-dapps Proposal 010 D1 + D6; gen-2 LIVE 2026-10-29, Plan 058 T6e;
/// the host family mirrors gen-1's: `riir.ai` apex front, `mainnet.riir.ai`
/// API, `refine.riir.ai` refine front — owner 2026-10-29).
/// `DEFAULT_SERVICE_URL` for every transport and the `mainnet` alias target
/// (riir-dapps Plan 059 C1, owner order 2026-10-29: "it must be default to
/// mainnet"). The apex `riir.ai` (same worker, same ledger) and the gen-1
/// API hosts (`mainnet.gist.rs`, `ai.gist.rs`) are LEGACY aliases of it ([`LEGACY_SERVICE_URLS`]): a stored or env-set
/// gen-1 URL canonicalizes here, the per-net state dir moves with it, and
/// the T2b state stamp sees the new ledger id (the D9-reset detection —
/// gen-1 state is quarantined, never mixed in).
pub const MAINNET_SERVICE_URL: &str = "https://mainnet.riir.ai";

/// The bare host of [`MAINNET_SERVICE_URL`] — what the net classifier and
/// the state-stamp key compare against (pinned equal by a test).
pub const MAINNET_HOST: &str = "mainnet.riir.ai";

/// Back-compat name for [`MAINNET_SERVICE_URL`] (the pre-flip spelling).
pub const GEN2_MAINNET_SERVICE_URL: &str = MAINNET_SERVICE_URL;

/// THE devnet API host — the config map's `devnet` alias and the transports'
/// failover PAIR target. A configured or env-set URL has NO pair — the
/// shipped "never custom" rule — so no cross-network failover can fire for
/// an opted-in dev machine.
///
/// ⛔ COMPILE-TO-NOTHING (the net-pin doctrine, `kat_net_pin`'s header): under
/// the OPPOSITE sole pin (`net-mainnet` without `net-devnet` — the shipped
/// cargo-refine release set since the owner's mainnet-only directive,
/// 2026-11-04) these consts and their alias rows DO NOT EXIST — the devnet
/// host strings are not in the pinned binary's rodata at all, not merely
/// refused at runtime. The cfg shape is "not mainnet-only": both sets
/// coexist unpinned and at the both-pins conflict posture (runtime refuses
/// there; compilation stays whole for `--all-features` lanes).
#[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
pub const DEVNET_SERVICE_URL: &str = "https://devnet.gist.rs";

/// The bare host of [`DEVNET_SERVICE_URL`] (pinned equal by a test).
#[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
pub const DEVNET_HOST: &str = "devnet.gist.rs";

/// The retired API names → their canonical host (Plan 047). The legacy
/// names stay served by the SAME workers for shipped clients (≤ 0.2.1) until
/// a release on the new hosts has shipped; a newer client canonicalizes a
/// STORED or env-set legacy URL on read ([`canonical_service_url`]), so an
/// upgraded install moves without user action. `testnet.ai.gist.rs` has no
/// entry: that deployment is gone, so its URL stays a custom (dead) host
/// rather than silently becoming another network.
///
/// ⛔ ORDER IS LOAD-BEARING: the FIRST row naming a canonical host is that
/// host's failover pair (`kat_lease_client::fallback_pair_for`), so it must
/// be the SAME worker + ledger. For mainnet that is the gen-2 apex
/// `riir.ai` — never a gen-1 host, which is another ledger.
pub const LEGACY_SERVICE_URLS: &[(&str, &str)] = &[
    // The gen-2 apex front serves the same worker + ledger (Plan 059 C1).
    ("https://riir.ai", MAINNET_SERVICE_URL),
    // gen-1 mainnet API hosts → the gen-2 API host (Plan 059 C1; the
    // workers answer them with permanent 308s once Plan 059 C4 lands).
    ("https://mainnet.gist.rs", MAINNET_SERVICE_URL),
    ("https://ai.gist.rs", MAINNET_SERVICE_URL),
    #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
    ("https://devnet.ai.gist.rs", DEVNET_SERVICE_URL),
];

/// A legacy API URL → its canonical host; any other URL unchanged
/// (trailing-slash and case tolerant on the match; a path/query is never
/// rewritten — only a bare service base is a service URL).
#[must_use]
pub fn canonical_service_url(url: &str) -> String {
    let base = url.trim().trim_end_matches('/');
    LEGACY_SERVICE_URLS
        .iter()
        .find(|(legacy, _)| base.eq_ignore_ascii_case(legacy))
        .map_or_else(
            || url.to_string(),
            |(_, canonical)| (*canonical).to_string(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_pinned_and_distinct() {
        assert_eq!(MAINNET_SERVICE_URL, "https://mainnet.riir.ai");
        assert_eq!(MAINNET_SERVICE_URL, format!("https://{MAINNET_HOST}"));
    }

    #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
    #[test]
    fn devnet_host_is_pinned() {
        assert_eq!(DEVNET_SERVICE_URL, "https://devnet.gist.rs");
        assert_eq!(DEVNET_SERVICE_URL, format!("https://{DEVNET_HOST}"));
        // the failover direction is default->devnet, never default->mainnet:
        // the pair must not equal the default
        assert_ne!(DEVNET_SERVICE_URL, MAINNET_SERVICE_URL);
    }

    #[test]
    fn legacy_urls_canonicalize_and_nothing_else_moves() {
        assert_eq!(
            canonical_service_url("https://ai.gist.rs"),
            MAINNET_SERVICE_URL
        );
        assert_eq!(
            canonical_service_url("https://AI.gist.rs/"),
            MAINNET_SERVICE_URL
        );
        // the apex and the gen-1 mainnet hosts are the gen-2 API host
        assert_eq!(
            canonical_service_url("https://riir.ai/"),
            MAINNET_SERVICE_URL
        );
        assert_eq!(
            canonical_service_url("https://mainnet.gist.rs"),
            MAINNET_SERVICE_URL
        );
        assert_eq!(
            canonical_service_url("https://Mainnet.gist.rs/"),
            MAINNET_SERVICE_URL
        );
        #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
        {
            assert_eq!(
                canonical_service_url("https://devnet.ai.gist.rs/"),
                DEVNET_SERVICE_URL
            );
        }
        // a retired deployment never becomes another network
        assert_eq!(
            canonical_service_url("https://testnet.ai.gist.rs"),
            "https://testnet.ai.gist.rs"
        );
        // a path is never rewritten, a custom host never moves
        assert_eq!(
            canonical_service_url("https://ai.gist.rs/pay"),
            "https://ai.gist.rs/pay"
        );
        assert_eq!(
            canonical_service_url("http://localhost:8787"),
            "http://localhost:8787"
        );
        assert_eq!(
            canonical_service_url(MAINNET_SERVICE_URL),
            MAINNET_SERVICE_URL
        );
        for (legacy, canonical) in LEGACY_SERVICE_URLS {
            assert_ne!(legacy, canonical);
        }
    }
}
