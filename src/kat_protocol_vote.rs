//! Wire types for vote — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).
//!
//! The DTOs + wire consts moved verbatim out of `crate::kat_vote_client`,
//! which re-exports every name here so all existing
//! `crate::kat_vote_client::X` paths resolve unchanged. No imports: the
//! signing helpers (`vote_message` / `sign_vote` / `hex_encode`), the
//! renders, the queue IO and the transports stay with the client.

// ── the mirrored wire contract (riir-dapps kat::governance — no dep) ─────

/// MIRROR of dapps `GOVOTE_SIGNING_DOMAIN` (pinned by the golden test).
pub const GOVOTE_SIGNING_DOMAIN: &[u8] = b"riir-dapps.kat.govote.v1";

/// The vote-request schema version.
pub const VOTE_VERSION: u32 = 1;

/// A vote choice, wire form ("yes"/"no" — the service's JSON vocabulary).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoteChoice {
    Yes,
    No,
}

impl VoteChoice {
    /// The wire string (JSON `choice`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
        }
    }

    /// Parse the wire string (the service response's echo).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "yes" => Some(Self::Yes),
            "no" => Some(Self::No),
            _ => None,
        }
    }
}

// ── the views (the service's JSON responses, deserialized) ───────────────

/// One governance proposal as the service serves it (the render source).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProposalView {
    pub id: u64,
    /// "ban" | "unban".
    pub kind: String,
    /// The account named by the proposal (the ledger's 16-B id, hex).
    pub account_hex: String,
    pub reason_code: u16,
    /// BLAKE3 evidence hash, hex — **rendered on every vote** (the debate's
    /// anchor; pinned at open, immutable mid-vote).
    pub evidence_hash_hex: String,
    /// Optional advisory-transcript hash, hex ("" when none).
    pub advisory_hash_hex: String,
    /// "open" | "passed" | "rejected" | "executed" | "expired".
    pub state: String,
    pub open_tick: u64,
    pub close_tick: u64,
    pub exec_eligible_tick: u64,
    /// The frozen quorum denominator (USDC base units).
    pub eligible_total: u64,
    pub yes_weight: u64,
    pub no_weight: u64,
    /// THE CALLER's snapshot weight on this proposal (USDC base units —
    /// `0` means the tranches are too young / staked after open: recorded
    /// intent, zero influence).
    pub your_weight: u64,
}

/// The vote-submit response (the service's ack).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VoteAck {
    pub proposal_id: u64,
    pub choice: String,
    pub snapshot_weight: u64,
}

// ── the queued intent (.heal/vote/pending.jsonl row) ─────────────────────

/// A queued vote intent — signed locally when the service plane is not
/// live; submitted at sync time later (the signature covers the exact
/// message, so the queued intent is as good as the account's word).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VoteIntent {
    pub vote_version: u32,
    pub account_id: String,
    pub proposal_id: u64,
    pub choice: String,
    pub voter_pubkey_hex: String,
    pub signature_hex: String,
    pub queued_unix: u64,
}
