//! The payreq client area: the claim / faucet-request / invoice wire
//! payloads the kat-service's 200 bodies parse into.
//! Wire types for payreq — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).

/// The label every faucet-preset request carries — MIRROR of
/// `riir_dapps::kat::payreq::FAUCET_LABEL` (§11: "a payreq whose label is
/// `faucet`"). It rides the canonical request id, so a faucet request and
/// a pay request opened by the same account in the same tick get
/// different ids by construction.
pub const FAUCET_LABEL: &str = "faucet";

/// The parsed claim response — the 200 body of `POST /account/claim`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimResponse {
    /// The account id the service derived from the signed key (32 hex).
    pub account_id_hex: String,
    /// Whether the once-per-key free-tier grant is now applied — always
    /// `true` on a 200.
    pub granted: bool,
    /// `true` when this call was a REPLAY (the grant already existed) —
    /// an idempotent read, not an error.
    pub already: bool,
    /// The account's balance after the claim, micro-units.
    pub balance_micro: i64,
    /// What THIS claim credited (0 on a replay).
    pub claimed_micro: i64,
}

/// The opened faucet request — the 200 body of `POST /payreq/faucet`.
///
/// `amount_micro` is the ledger's, echoed back: the CLI **displays** it and
/// never proposes it. A zero here is not "any amount" (the pay-request
/// convention) — it means the row read back without one, which the caller
/// treats as unusable rather than as a free pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaucetRequest {
    /// The canonical request id, hex (32 chars).
    pub id_hex: String,
    /// The amount the ledger pinned (micro-units).
    pub amount_micro: i64,
    /// The `link_ttl`-derived deadline, in the ledger's tick domain.
    pub expiry: u64,
}

/// The parsed invoice row — `GET /payreq/{id}/invoice` (riir-dapps
/// `payreq_invoice_route`).
///
/// Wire-field note: the service's QR slot is spelled `qr_png_blake3` and
/// is **all-zero until the T1.4 render step fills it** (dapps
/// `invoice_set_qr_hash`, fills exactly once). This repo commits the QR
/// **module matrix** rather than a PNG, because `qrcode` ships with
/// `default-features = false` — there is no `image` in the tree and so no
/// PNG to hash. The matrix is the stronger artifact (any rendering is a
/// function of it), and it is what the recheck compares against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceView {
    /// The request/invoice id, hex.
    pub id_hex: String,
    /// The payee account (16-byte ledger id), hex.
    pub payee_hex: String,
    /// The payer account (16-byte ledger id), hex.
    pub payer_hex: String,
    /// `"kat"` or `"usdc"` — the rail the settle used.
    pub token: String,
    /// What actually arrived, in micro-units (an any-amount request
    /// records the amount the payer composed).
    pub amount_paid_micro: i64,
    /// The service tick the settle landed at.
    pub settled_tick: u64,
    /// The service's committed QR asset digest, hex — `""` or 64 zeros
    /// while the service has not filled it.
    pub qr_commitment_hex: String,
}

impl InvoiceView {
    /// The unfilled-commitment sentinel: absent, or 32 zero bytes (the
    /// dapps row's `[0u8; 32]` initial state).
    #[must_use]
    pub fn qr_commitment_is_unset(&self) -> bool {
        self.qr_commitment_hex.is_empty() || self.qr_commitment_hex.bytes().all(|b| b == b'0')
    }
}
