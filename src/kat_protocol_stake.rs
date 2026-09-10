//! Wire types for stake — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).
//!
//! The DTOs moved verbatim out of `crate::kat_stake_client`, which re-exports
//! every name here so all existing `crate::kat_stake_client::X` paths resolve
//! unchanged. No imports: std-only shapes plus serde derive paths; the codecs
//! (base58, amount formatting), env config, queue IO, renders and transports
//! stay with the client.

// ── the wire-schema version ────────────────────────────────────────────────

/// The stake-transfer request schema version.
pub const STAKE_VERSION: u32 = 1;

// ── the local intent queue shapes (.heal/stake/ JSONL rows) ────────────────

/// A queued stake intent — composed locally when the service plane is not
/// live; the claim lands via sync/reference auto-detect later (the service
/// sees the reference in a verified transfer and credits the tranche).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StakeIntent {
    pub stake_version: u32,
    pub account_id: String,
    pub payee_b58: String,
    pub usdc_mint_b58: String,
    pub reference_b58: String,
    pub amount_base: u64,
    pub queued_unix: u64,
}

/// A queued unstake intent (drain-all-unlocked; the service applies it when
/// the plane lands — the escrow/vent mechanics are service-side).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnstakeIntent {
    pub stake_version: u32,
    pub account_id: String,
    pub queued_unix: u64,
}

// ── the service info view (the JSON the /stake/info endpoint returns) ──────

/// One tranche row (the `TrancheRow` mirror: kind/amount/unlock — the FIFO
/// display needs kind + amount + unlock only).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrancheView {
    pub seq: u32,
    /// `"usdc"` | `"kat"` (the `TrancheKind` names).
    pub kind: String,
    pub amount_base: u64,
    pub unlock_at_tick: u64,
}

/// The epoch pool split (the `StakingParams` economics the mock renders).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PoolView {
    pub mining_bp: u32,
    pub staking_bp: u32,
    pub usdc_share_bp: u32,
    pub kat_share_bp: u32,
    pub authority_share_bp: u32,
    pub authority_floor_bp: u32,
    pub authority_decay_bp: u32,
    /// The epoch over which the current authority share is computed.
    pub epoch: u64,
    /// Ticks per epoch (the "over N epochs" decay denominator).
    pub ticks_per_epoch: u64,
}

impl PoolView {
    /// The authority share at [`Self::epoch`] (the linear-decay-to-floor
    /// schedule, computed for display; the ledger's own fn is authoritative).
    #[must_use]
    pub fn authority_share_now_bp(&self) -> u32 {
        let start = self.authority_share_bp;
        let floor = self.authority_floor_bp;
        let decayed = self
            .authority_decay_bp
            .saturating_mul(u32::try_from(self.epoch).unwrap_or(u32::MAX));
        start.saturating_sub(decayed).max(floor)
    }

    /// The epoch index at which the share reaches the floor (display form:
    /// "over N epochs").
    #[must_use]
    pub fn epochs_to_floor(&self) -> u64 {
        if self.authority_decay_bp == 0 {
            return 0;
        }
        let span = self
            .authority_share_bp
            .saturating_sub(self.authority_floor_bp);
        u64::from(span.div_ceil(self.authority_decay_bp))
    }
}

/// The account's staking view (what `GET /stake/info` returns + what the
/// renderer consumes).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StakeInfo {
    pub account_id: String,
    /// Sum of USDC-principal tranches, base units.
    pub staked_base: u64,
    /// Accrued staking incentives, KAT micro-units (i64 — the ledger's
    /// `MicroUnits`; rendered as whole KAT, never a $ figure).
    pub accrued_kat_micro: i64,
    /// The reward-restake flag (renders ON/OFF; flipping is service-side).
    pub restake: bool,
    /// FIFO tranche rows (unlock countdowns render per-tranche).
    pub tranches: Vec<TrancheView>,
    /// The epoch pool split (the mock's "Epoch pool:" rows).
    pub pool: PoolView,
}
