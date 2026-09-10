//! The pinned KAT service hosts — the one place the network's deployment
//! URLs are named. Both the client transports and the machine-local
//! `kat_service_config` (the Solana-CLI-model `config set --url` surface)
//! consume these, so a host can never drift between the config tier and the
//! transport tier.
//!
//! Feature-free: naming a host is not behavior. The apex IS mainnet (the
//! const never moves — the QR goldens pin it, riir-dapps Plan 023's
//! premise); devnet is its own worker deployment; testnet is the offline
//! demo. The client failover PAIR is default→devnet ONLY — a DEV client
//! must never fail over onto MAINNET.

/// The apex — `DEFAULT_SERVICE_URL` for every transport, and the `mainnet`
/// alias target in the config map. Deliberately the SAME string forever:
/// the QR goldens pin it (riir-dapps Plan 023 S3).
pub const MAINNET_SERVICE_URL: &str = "https://ai.gist.rs";

/// The devnet subdomain — the config map's `devnet` alias and the
/// transports' failover PAIR target (the other known service host,
/// REPOINTED 2026-09-10 from the retired `heal.gist.rs`, riir-dapps Plan
/// 023 S2). A configured or env-set URL has NO pair — the shipped "never
/// custom" rule — so no cross-network failover can fire for an opted-in
/// dev machine.
pub const DEVNET_SERVICE_URL: &str = "https://devnet.ai.gist.rs";

/// The testnet subdomain — the config map's `testnet` alias (the
/// offline-demo deployment). Never a client default; never a failover
/// target.
pub const TESTNET_SERVICE_URL: &str = "https://testnet.ai.gist.rs";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_pinned_and_distinct() {
        assert_eq!(MAINNET_SERVICE_URL, "https://ai.gist.rs");
        assert_eq!(DEVNET_SERVICE_URL, "https://devnet.ai.gist.rs");
        assert_eq!(TESTNET_SERVICE_URL, "https://testnet.ai.gist.rs");
        assert_ne!(MAINNET_SERVICE_URL, DEVNET_SERVICE_URL);
        assert_ne!(DEVNET_SERVICE_URL, TESTNET_SERVICE_URL);
        // the failover direction is default->devnet, never default->mainnet:
        // the pair must not equal the default
        assert_ne!(DEVNET_SERVICE_URL, MAINNET_SERVICE_URL);
    }
}
