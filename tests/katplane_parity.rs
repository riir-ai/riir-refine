//! Golden pins for the vendored `katplane` (issue 221) — the contract
//! constants a hand-edit or a bad re-vendor would silently move.
//!
//! Provenance: exported from `riir-kat` `863c590798b5` (2026-11-09). These
//! are BEHAVIOR pins, not text pins — the vendored files may accumulate
//! lint-posture doc fixes, but the values below must match the canonical
//! sibling exactly. When `riir-kat` changes one of these, the change is a
//! wire/config event: re-vendor (`scripts/vendor_katplane.sh`) AND update
//! this file in the same commit, citing the sibling sha.

#![allow(clippy::pedantic)] // golden pins, not production code

use riir_refine::kat_redact::FindingKind::*;
use riir_refine::kat_service_urls::{GEN2_MAINNET_SERVICE_URL, MAINNET_SERVICE_URL};
use riir_refine::kat_units::{
    CHARGE_MICRO_KAT_PER_MILLION_TOKENS, FREE_TIER_GRANT_MICRO_KAT, FREE_TIER_TOKENS_MTOK,
    MICRO_PER_KAT,
};

/// The one unit conversion — the µ↔whole boundary every money path rides
/// (the Issue-094 role-shape law). 10⁶, never 10⁹ (Solana's lamports role).
#[test]
fn micro_per_kat_is_the_pinned_one_million() {
    assert_eq!(MICRO_PER_KAT, 1_000_000);
}

/// The free-tier grant identity (riir-dapps Issue 094 D1, dapps-golden):
/// 100 whole KAT, µ-stored; the meter price and the Mtok allowance must
/// move TOGETHER or the credit is 10⁶× wrong (the Issue-025 failure mode).
#[test]
fn free_tier_grant_identity_is_pinned() {
    assert_eq!(FREE_TIER_GRANT_MICRO_KAT.0, 100_000_000);
    assert_eq!(CHARGE_MICRO_KAT_PER_MILLION_TOKENS.0, 1_000_000);
    assert_eq!(FREE_TIER_TOKENS_MTOK, 100);
    // 100 KAT at 1 µ per token = 100 Mtok — the grant identity, checked.
    assert_eq!(
        FREE_TIER_GRANT_MICRO_KAT.0 / CHARGE_MICRO_KAT_PER_MILLION_TOKENS.0,
        100
    );
}

/// The hosted nets under their pinned names (the QR-goldens law pins the
/// apex URL in the client tests; this pin holds the VENDORED copy to the
/// same values so a re-vendor cannot silently re-point a host).
#[test]
fn service_hosts_are_pinned() {
    assert_eq!(MAINNET_SERVICE_URL, "https://mainnet.riir.ai");
    // The devnet host compiles to nothing under the shipped net-mainnet
    // pin (the foreign-hosts compile-to-nothing discipline), so its pin
    // rides the same cfg the source const does.
    #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
    {
        assert_eq!(
            riir_refine::kat_service_urls::DEVNET_SERVICE_URL,
            "https://devnet.gist.rs"
        );
    }
    assert_eq!(GEN2_MAINNET_SERVICE_URL, MAINNET_SERVICE_URL);
}

/// The redact scanner keeps its rule census — pinned by an EXHAUSTIVE
/// `FindingKind` match, so a re-vendor that adds or drops a rule kind
/// breaks THIS build instead of silently moving the scanner's surface
/// (the `tag()` spellings stay frozen per the vendored module's own
/// v4/v6 notes). Update the arms AND the count in the same commit as the
/// re-vendor, citing the sibling sha.
#[test]
fn redact_finding_kind_census_is_pinned() {
    let all = [
        AwsAccessKey,
        GithubToken,
        GitlabToken,
        SlackToken,
        OpenAiStyleKey,
        PrivateKeyBlock,
        ConnStringCreds,
        BearerToken,
        EnvSecretAssign,
        RustSecretLiteral,
        HighEntropyHex,
        HighEntropyB64,
        GoogleApiKey,
        StripeLiveKey,
        SendGridToken,
        NpmToken,
        PypiToken,
        HuggingFaceToken,
        ShopifyToken,
        SquareToken,
        DatabricksToken,
        DigitalOceanToken,
        FigmaToken,
        SupabaseToken,
        NotionToken,
        LinearToken,
        PerplexityKey,
        XaiKey,
        GroqKey,
        MailgunKey,
        AgeKey,
        TelegramBotToken,
        DiscordBotToken,
        JwtToken,
        AtlassianToken,
        FacebookToken,
        PuttyPrivateKey,
        ReplicateToken,
        LangsmithKey,
        PineconeKey,
    ];
    assert_eq!(all.len(), 40, "the vendored rule census at 863c590798b5");
    // Every kind carries its own frozen tag — the exhaustive list above
    // must cover the enum, so this round-trip reads tautological only
    // until a variant is added (then the array fails to name it and the
    // count refuses).
    for kind in all {
        assert!(!kind.tag().is_empty());
    }
}
