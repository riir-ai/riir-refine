//! KAT governance voting client — Proposal 008 T2.4 (`cargo heal vote`).
//!
//! The wire MIRROR of riir-dapps' `kat::governance` voting surface (the
//! [`crate::kat_stake_client`] precedent): the vote-signing domain + message
//! layout are duplicated here BYTE-FOR-BYTE because riir-dapps is outside
//! this crate's boundary (the copy-gate discipline) — and the duplication
//! is pinned by the SAME golden hex asserted in BOTH repos' tests
//! (`vote_message` over the fixture `proposal 1, [0x42;32], Yes` — a silent
//! drift on either side fails a gate).
//!
//! # What the command does
//!
//! `cargo heal vote --proposal <id> --yes|--no`:
//!
//! 1. **service-first**: fetch the proposal (`GET /gov/proposal?id=<id>&acct=<account>`)
//!    and render it — **the weight + the evidence hash lead** (the proposal's
//!    own ask), with the tally + window + state;
//! 2. sign the vote with the LOGIN key (the OpenSSH Ed25519 account key —
//!    the ledger binds it write-once per account, `kat:govvkey`) and submit
//!    (`POST /gov/vote`);
//! 3. **not-live fallback**: queue the signed intent at `.heal/vote/pending.jsonl`
//!    (the stake-queue pattern) and render the honest state — the weight
//!    shows when the service answers, never invented client-side.
//!
//! # Copy discipline (the caveat-#15 family, extended to governance)
//!
//! A vote NEVER moves funds — the copy says so, says "membership", and never
//! puts a `$` beside anything. Enforced by test (`copy_gate_*`).

use std::path::{Path, PathBuf};

use ed25519_dalek::Signer as _;

// ── the mirrored wire contract (riir-dapps kat::governance — no dep) ─────

/// Lowercase hex of a byte slice (the wire encoding for pubkeys +
/// signatures — full-width, unambiguous).
#[must_use]
pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[usize::from(b >> 4)] as char);
        out.push(HEX[usize::from(b & 0x0F)] as char);
    }
    out
}

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

/// MIRROR of dapps `vote_message` — the exact signed bytes:
/// `GOVOTE_SIGNING_DOMAIN ‖ proposal_id le64 ‖ voter_pubkey[32] ‖ choice u8`.
/// **Do not drift**: the dapps-side verifier reconstructs these bytes.
#[must_use]
pub fn vote_message(proposal_id: u64, voter_pubkey: &[u8; 32], choice: VoteChoice) -> Vec<u8> {
    let choice_byte = match choice {
        VoteChoice::Yes => 1u8,
        VoteChoice::No => 2u8,
    };
    let mut m = Vec::with_capacity(GOVOTE_SIGNING_DOMAIN.len() + 8 + 32 + 1);
    m.extend_from_slice(GOVOTE_SIGNING_DOMAIN);
    m.extend_from_slice(&proposal_id.to_le_bytes());
    m.extend_from_slice(voter_pubkey);
    m.push(choice_byte);
    m
}

/// Sign a vote with the account's Ed25519 key (the login key). Returns
/// `(signature, pubkey)` — both ride the request; the ledger binds the
/// pubkey to the account write-once at first vote.
#[must_use]
pub fn sign_vote(
    key: &ed25519_dalek::SigningKey,
    proposal_id: u64,
    choice: VoteChoice,
) -> ([u8; 64], [u8; 32]) {
    let pubkey = key.verifying_key().to_bytes();
    let sig = key
        .sign(&vote_message(proposal_id, &pubkey, choice))
        .to_bytes();
    (sig, pubkey)
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

// ── the renders (pure; the copy gates pin them) ──────────────────────────

/// `1d 02h`-style countdown (the stake family's shared shape).
#[must_use]
pub fn countdown(ticks: u64) -> String {
    let days = ticks / 86_400;
    let hours = (ticks % 86_400) / 3_600;
    format!("{days}d {hours:02}h")
}

/// Render a proposal for the vote screen — **weight + evidence lead** (the
/// proposal §CLI mock's own ask), then the tally + window + state + the
/// constitutional line.
#[must_use]
pub fn render_proposal(p: &ProposalView, now_tick: u64) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let kind = if p.kind == "ban" { "BAN" } else { "UNBAN" };
    let state = p.state.to_uppercase();
    let acct = p.account_hex.as_str();
    let head = &acct[..acct.len().min(4)];
    let tail = &acct[acct.len().saturating_sub(4)..];
    let _ = writeln!(
        out,
        "Proposal #{id} — {kind} account {head}…{tail}   [{state}]",
        id = p.id,
    );
    let evidence = if p.evidence_hash_hex.is_empty() {
        "—".to_string()
    } else {
        let n = p.evidence_hash_hex.len().min(16);
        format!("0x{}…", &p.evidence_hash_hex[..n])
    };
    let _ = writeln!(
        out,
        "Evidence:   {evidence}   (pinned at open — debate cannot be rewritten)"
    );
    if !p.advisory_hash_hex.is_empty() {
        let n = p.advisory_hash_hex.len().min(16);
        let _ = writeln!(
            out,
            "Advisory:   0x{a}…   (the optional GLM transcript slot)",
            a = &p.advisory_hash_hex[..n]
        );
    }
    let cast = p.yes_weight.saturating_add(p.no_weight);
    let _ = writeln!(
        out,
        "Tally:      {yes} yes / {no} no of {tot} eligible ({cast} cast)",
        yes = p.yes_weight,
        no = p.no_weight,
        tot = p.eligible_total,
        cast = cast,
    );
    let window = if now_tick < p.close_tick {
        format!(
            "closes in {}",
            countdown(p.close_tick.saturating_sub(now_tick))
        )
    } else {
        "window closed".to_string()
    };
    let _ = writeln!(out, "Window:     {window}");
    let _ = writeln!(
        out,
        "Your vote:  weight {w}   (staked USDC aged past the open tick; 0 = too young — recorded, zero influence)",
        w = p.your_weight,
    );
    out.push_str(
        "A vote decides MEMBERSHIP only — it never moves funds (no confiscation,\n\
         no claw; a banned account's principal unstakes normally).",
    );
    out
}

/// The vote-submitted confirmation line.
#[must_use]
pub fn vote_confirm_line(ack: &VoteAck) -> String {
    format!(
        "vote: {c} on proposal #{id} recorded — snapshot weight {w}",
        c = ack.choice,
        id = ack.proposal_id,
        w = ack.snapshot_weight,
    )
}

// ── the local intent queue (.heal/vote/ — the stake-queue pattern) ───────

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

/// Append a vote intent to `.heal/vote/pending.jsonl`.
///
/// # Errors
///
/// The underlying `io::Error` from directory creation or the append.
pub fn queue_vote_intent(root: &Path, intent: &VoteIntent) -> std::io::Result<PathBuf> {
    use std::io::Write as _;
    let path = root.join(".heal").join("vote").join("pending.jsonl");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(f, "{}", serde_json::to_string(intent).unwrap_or_default())?;
    Ok(path)
}

// ── the transport + the service-first flow ───────────────────────────────

/// The governance-channel transport (the `StakeTransport` pattern).
#[cfg(feature = "kat_transport")]
pub trait VoteTransport {
    /// `GET /gov/proposal?id=<id>&acct=<account>` → 200 with the
    /// [`ProposalView`] JSON (the `your_weight` field is server-computed).
    fn get_proposal(
        &self,
        proposal_id: u64,
        account_id: &str,
    ) -> crate::kat_lease_client::HttpResponse;
    /// `POST /gov/vote` (JSON body) → 200 with the [`VoteAck`] JSON.
    fn post_vote(&self, json: &str) -> crate::kat_lease_client::HttpResponse;
}

/// The ureq HTTP transport over the shared service base URL.
#[cfg(feature = "kat_transport")]
pub struct HttpVoteTransport {
    agent: ureq::Agent,
    base: String,
}

#[cfg(feature = "kat_transport")]
impl HttpVoteTransport {
    /// Build with a per-request timeout in seconds (the explicit-command
    /// budget, the `HttpStakeTransport` shape).
    #[must_use]
    pub fn new(base: &str, timeout_secs: u64) -> Self {
        let t = std::time::Duration::from_secs(timeout_secs.max(1));
        let agent = ureq::AgentBuilder::new()
            .timeout(t)
            .timeout_connect(t)
            .timeout_read(t)
            .build();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
        }
    }
}

#[cfg(feature = "kat_transport")]
impl VoteTransport for HttpVoteTransport {
    fn get_proposal(
        &self,
        proposal_id: u64,
        account_id: &str,
    ) -> crate::kat_lease_client::HttpResponse {
        let url = format!(
            "{}/gov/proposal?id={proposal_id}&acct={account_id}",
            self.base
        );
        match self.agent.get(&url).call() {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => crate::kat_lease_client::HttpResponse::Status(c),
            Err(e) => crate::kat_lease_client::HttpResponse::Error(e.to_string()),
        }
    }

    fn post_vote(&self, json: &str) -> crate::kat_lease_client::HttpResponse {
        let url = format!("{}/gov/vote", self.base);
        match self
            .agent
            .post(&url)
            .set("Content-Type", "application/json")
            .send_string(json)
        {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => crate::kat_lease_client::HttpResponse::Status(c),
            Err(e) => crate::kat_lease_client::HttpResponse::Error(e.to_string()),
        }
    }
}

#[cfg(feature = "kat_transport")]
impl HttpVoteTransport {
    /// ureq response → the shared envelope (the `HttpStakeTransport`
    /// private-helper shape).
    fn body(resp: ureq::Response) -> crate::kat_lease_client::HttpResponse {
        use std::io::Read as _;
        match resp.status() {
            304 => crate::kat_lease_client::HttpResponse::NotModified,
            200 => {
                let mut buf = Vec::new();
                match resp.into_reader().read_to_end(&mut buf) {
                    Ok(_) => crate::kat_lease_client::HttpResponse::Ok(buf),
                    Err(e) => crate::kat_lease_client::HttpResponse::Error(e.to_string()),
                }
            }
            c => crate::kat_lease_client::HttpResponse::Status(c),
        }
    }
}

/// What a vote flow concluded (the CLI prints one arm).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoteOutcome {
    /// The service accepted the signed vote (the ack carries the snapshot
    /// weight the LEDGER computed — the number that counts).
    Submitted(VoteAck),
    /// Not live: the signed intent queued locally (the path).
    LocalQueued { queued_at: PathBuf },
}

/// The vote flow (service-first; the not-live fallback queues the SIGNED
/// intent — the signature is over the exact message bytes, so a queued vote
/// is as binding as a submitted one).
///
/// # Errors
///
/// A human-readable string when the key cannot be loaded, the proposal
/// fetch/parse fails with the service live, or the queue write fails.
#[cfg(feature = "kat_transport")]
pub fn vote_flow(
    transport: &dyn VoteTransport,
    account_id: &str,
    proposal_id: u64,
    choice: VoteChoice,
    key: &ed25519_dalek::SigningKey,
    root: &Path,
    now_unix: u64,
) -> Result<(Option<ProposalView>, VoteOutcome), String> {
    let (signature, pubkey) = sign_vote(key, proposal_id, choice);
    let pubkey_hex = hex_encode(&pubkey);
    let signature_hex = hex_encode(&signature);

    // Service-first: the proposal view (weight + evidence) when the plane
    // answers; the queue fallback renders without it (honest — no invented
    // weights client-side).
    let mut view = None;
    if let crate::kat_lease_client::HttpResponse::Ok(body) =
        transport.get_proposal(proposal_id, account_id)
    {
        match serde_json::from_slice::<ProposalView>(&body) {
            Ok(v) => view = Some(v),
            Err(e) => return Err(format!("vote: service proposal response unreadable: {e}")),
        }
    }

    let req = serde_json::json!({
        "vote_version": VOTE_VERSION,
        "account_id": account_id,
        "proposal_id": proposal_id,
        "choice": choice.as_str(),
        "voter_pubkey_hex": pubkey_hex,
        "signature_hex": signature_hex,
    });
    if let crate::kat_lease_client::HttpResponse::Ok(body) = transport.post_vote(&req.to_string()) {
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(err) = parsed.get("error").and_then(serde_json::Value::as_str) {
                return Err(format!("vote: refused by the service — {err}"));
            }
            if let Ok(ack) = serde_json::from_slice::<VoteAck>(&body) {
                return Ok((view, VoteOutcome::Submitted(ack)));
            }
        }
        return Err("vote: service answered, but the response was not understood".to_string());
    }

    let intent = VoteIntent {
        vote_version: VOTE_VERSION,
        account_id: account_id.to_string(),
        proposal_id,
        choice: choice.as_str().to_string(),
        voter_pubkey_hex: pubkey_hex,
        signature_hex,
        queued_unix: now_unix,
    };
    let queued_at = queue_vote_intent(root, &intent).map_err(|e| e.to_string())?;
    Ok((view, VoteOutcome::LocalQueued { queued_at }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_proposal() -> ProposalView {
        ProposalView {
            id: 7,
            kind: "ban".to_string(),
            account_hex: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6".to_string(),
            reason_code: 1,
            evidence_hash_hex: "ab".repeat(32),
            advisory_hash_hex: String::new(),
            state: "open".to_string(),
            open_tick: 1_000,
            close_tick: 1_000 + 259_200,
            exec_eligible_tick: 1_000 + 259_200 + 172_800,
            eligible_total: 10_000,
            yes_weight: 5_000,
            no_weight: 3_000,
            your_weight: 5_000,
        }
    }

    /// THE cross-repo golden (dapps `g9_vote_message_golden_hex` asserts the
    /// SAME hex): domain ‖ id le64 ‖ pubkey ‖ choice.
    #[test]
    fn vote_message_golden_hex_matches_the_dapps_side() {
        let msg = vote_message(1, &[0x42; 32], VoteChoice::Yes);
        let hex = hex_encode(&msg);
        assert_eq!(
            hex,
            "726969722d64617070732e6b61742e676f766f74652e7631\
             0100000000000000\
             4242424242424242424242424242424242424242424242424242424242424242\
             01"
            .replace(' ', "")
        );
        assert_eq!(
            GOVOTE_SIGNING_DOMAIN,
            b"riir-dapps.kat.govote.v1".as_slice()
        );
        // The No choice is the 0x02 byte.
        let no = vote_message(1, &[0x42; 32], VoteChoice::No);
        assert_eq!(no[no.len() - 1], 0x02);
    }

    /// A signature made here VERIFIES against the mirrored message — the
    /// self-consistency half of the mirror (the cross-repo hex is the other).
    #[test]
    fn signed_vote_verifies_against_the_mirrored_message() {
        use ed25519_dalek::Verifier as _;
        let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let (sig, pubkey) = sign_vote(&key, 42, VoteChoice::Yes);
        key.verifying_key()
            .verify(&vote_message(42, &pubkey, VoteChoice::Yes), &sig.into())
            .expect("mirror verifies");
        // And the exact bytes the ledger reconstructs: changing ANY of
        // (id, pubkey, choice) breaks it.
        assert!(key
            .verifying_key()
            .verify(&vote_message(43, &pubkey, VoteChoice::Yes), &sig.into())
            .is_err());
    }

    /// The render leads with WEIGHT + EVIDENCE (the proposal §CLI mock's
    /// ask) and carries the membership-only line on every screen.
    #[test]
    fn render_leads_with_weight_and_evidence() {
        let p = fixture_proposal();
        let r = render_proposal(&p, 1_500);
        assert!(r.contains("Evidence:   0xabab"), "{r}");
        assert!(
            r.contains("0xabababababababab…"),
            "the evidence hash renders"
        );
        assert!(r.contains("Your vote:  weight 5000"), "{r}");
        assert!(r.contains("[OPEN]"), "{r}");
        assert!(
            r.contains("Tally:      5000 yes / 3000 no of 10000 eligible"),
            "{r}"
        );
        assert!(r.contains("closes in"), "{r}");
        assert!(
            r.contains("never moves funds"),
            "the constitutional line: {r}"
        );
        assert!(!r.contains('$'), "no $ anywhere on a governance screen");
    }

    /// Zero weight renders the honest too-young explanation, not silence.
    #[test]
    fn zero_weight_renders_the_age_explanation() {
        let mut p = fixture_proposal();
        p.your_weight = 0;
        let r = render_proposal(&p, 1_500);
        assert!(r.contains("weight 0"), "{r}");
        assert!(r.contains("too young"), "{r}");
    }

    /// The copy gates: no `$`, no slashing language, membership-only
    /// framing (the constitutional line names what it FORBIDS — "no
    /// confiscation" — the gate asserts nothing PROMISES movement).
    #[test]
    fn copy_gate_no_dollar_and_no_fund_movement_promise() {
        let p = fixture_proposal();
        let screens = format!(
            "{}\n{}",
            render_proposal(&p, 1_500),
            vote_confirm_line(&VoteAck {
                proposal_id: 7,
                choice: "yes".to_string(),
                snapshot_weight: 5_000,
            })
        );
        assert!(!screens.contains('$'), "no $ on any vote screen");
        assert!(!screens.contains("slash"), "no slashing language");
        assert!(screens.contains("never moves funds"));
        assert!(screens.contains("MEMBERSHIP only"));
    }

    /// The intent queues as valid JSON with the signature at full width.
    #[test]
    fn vote_intent_queues_with_a_full_width_signature() {
        let dir = std::env::temp_dir().join(format!("kat_vote_q_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let intent = VoteIntent {
            vote_version: VOTE_VERSION,
            account_id: "a1b2".to_string(),
            proposal_id: 7,
            choice: "yes".to_string(),
            voter_pubkey_hex: "42".repeat(32),
            signature_hex: "ab".repeat(64),
            queued_unix: 1_000,
        };
        let path = queue_vote_intent(&dir, &intent).expect("queue");
        let text = std::fs::read_to_string(&path).expect("read");
        let parsed: VoteIntent = serde_json::from_str(text.trim()).expect("round-trip");
        assert_eq!(parsed, intent);
        assert_eq!(parsed.signature_hex.len(), 128, "64 B signature as hex");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
