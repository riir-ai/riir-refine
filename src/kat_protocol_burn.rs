//! The burn client area: the burn-report signing domain, the service's
//! 200 ack payload, and the ack-watermark sidecar shape.
//! Wire types for burn — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).

/// The signing domain — the first 12 bytes of every burn signature. The
/// `v1` suffix is part of the pinned bytes: changing it is a wire break
/// against the service's own verifier, not a tweak.
pub const BURN_SIGNING_DOMAIN: &[u8] = b"kat:burn:v1:";

/// The parsed `200` body — what the service stored, applied, and now holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BurnReportAck {
    /// The server's stored cumulative burn for this account, whole KAT
    /// (`max(stored, reported)` — may exceed what we sent when another
    /// machine reported higher).
    pub burn_kat_total: u64,
    /// The increment the server applied to the burn liability this report.
    /// `0` on a replay.
    pub increment_kat: u64,
    /// The account balance AFTER the warn-and-pay debit, micro-units
    /// (1 KAT = 10⁶ micro; may go negative — warn-and-pay, never brick).
    pub balance_micro: i64,
    /// Whether the service says this account has EVER had an accepted
    /// mining batch (issue 080) — the sticky-grace SERVER truth, persisted
    /// into the ack verbatim. `None` = the field was absent or not a bool
    /// (a pre-v3 server answer); since the dated removal (2026-09-10) that
    /// is simply NOT contributed — there is no local-marker fallback.
    pub contributed: Option<bool>,
    /// The commitment hex of the first accepted batch (64 hex chars per
    /// the contract; provenance only — the gate never reads it). `None`
    /// when absent or not a string.
    pub first_commitment_hex: Option<String>,
}

/// The ack-watermark shape version (bump = a new shape; a mismatch
/// re-watermarks at zero — over-reporting is the safe direction, the
/// server's monotone watermark absorbs it).
///
/// History: v1 = the 076 shape (total, meter watermark, ack time). v2 adds
/// `last_balance_micro` (Issue 075) — the last-synced server balance the
/// soft gate reads; the one-time v1→v2 reset re-reports the full cumulative
/// total, a server-side no-op. v3 adds `contributed` +
/// `first_commitment_hex` (Issue 080) — the sticky-grace server truth;
/// ADDITIVE (optional fields), so a v2 file upgrades in place — see
/// `KatBurnAck::load_at`.
pub const BURN_ACK_VERSION: u8 = 3;

/// What the service last ACKNOWLEDGED — the local-commit book of the burn
/// sync. Persisted per-repo at `<root>/.heal/kat_burn_ack.json` (beside the
/// meter/charge ledgers); advanced ONLY by a 200-accepted response
/// (`report_burn` enforces the ≥-sent rule), so whatever it does not
/// cover is the stacked, un-pushed delta.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KatBurnAck {
    /// The sidecar shape version ([`BURN_ACK_VERSION`]).
    pub version: u8,
    /// The cumulative burn the server last confirmed for this account.
    pub burn_kat_total: u64,
    /// The meter watermark as of that report (informational provenance).
    pub meter_watermark_tokens: u64,
    /// Unix seconds of the accepted report (informational).
    pub acked_unix: u64,
    /// The account balance AFTER the warn-and-pay debit, as the server
    /// answered (`None` = a pre-v3 ack / never synced). The 075 soft gate
    /// reads THIS — the last-synced server truth — never the local
    /// projection (which models neither the faucet drip nor mining credits).
    #[serde(default)]
    pub last_balance_micro: Option<i64>,
    /// Whether the service has ever ACCEPTED a mining batch from this
    /// account, exactly as the burn-push response answered (`None` = a
    /// pre-v3 ack / pre-v3 server). Issue 080: the sticky-grace SERVER
    /// truth and, since the dated removal (2026-09-10), the ONLY input —
    /// `Some(true)` lifts the gate; `None`/`Some(false)` do not, and the
    /// retired machine-local marker has zero effect in every case.
    #[serde(default)]
    pub contributed: Option<bool>,
    /// The first accepted batch's commitment hex (provenance only — the
    /// gate never reads it). `None` when the server did not send it.
    #[serde(default)]
    pub first_commitment_hex: Option<String>,
}

impl Default for KatBurnAck {
    fn default() -> Self {
        Self::empty()
    }
}

impl KatBurnAck {
    /// The ack file name under the `.heal` root.
    pub const FILE_NAME: &'static str = "kat_burn_ack.json";

    /// The never-synced watermark: the whole cumulative burn is stacked.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            version: BURN_ACK_VERSION,
            burn_kat_total: 0,
            meter_watermark_tokens: 0,
            acked_unix: 0,
            last_balance_micro: None,
            contributed: None,
            first_commitment_hex: None,
        }
    }
}

/// The charge book → wire unit divisor. The local charge ledger counts in
/// MICRO-KAT (`charged_kat` = 1 micro per code word token at the v1 rate —
/// 1 KAT per MILLION tokens; the same unit as the 100 KAT grant's
/// 100,000,000 micro and the ledger's `MicroUnits`), while the PINNED wire
/// contract carries `burn_kat_total` in WHOLE KAT (the service debits
/// `increment × 1_000_000` micro). Sending the book value unconverted
/// over-reports by 10⁶× and the warn-and-pay clamp wipes the whole grant
/// (measured live 2026-09-09: a true 2.32 KAT spend consumed the account's
/// full 100 KAT grant). Floor conversion: sub-KAT remainders accumulate in
/// the book and ride the next report — the server's granularity is the
/// whole KAT it can actually debit.
pub const WIRE_PER_CHARGE_UNITS: u64 = 1_000_000;
