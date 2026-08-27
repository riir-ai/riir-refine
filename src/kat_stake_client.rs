//! KAT staking CLI client — Proposal 007 T2.1 (Plan 041): the render-only +
//! QR surface for `cargo heal stake --usd <N>` / `stake --info` / `unstake`.
//!
//! **The T1.6 posture**: the Worker/DO HTTP shell is deferred, so this client
//! composes requests + renders, validated against a local std-TCP mock; the
//! live-Worker flip is a one-line change (the service-URL resolution is
//! identical to the lease client's). Service-first, honest fallback:
//!
//! - `stake --usd N` → ask the service for a transfer request (`POST
//!   /stake/request` — the service derives the per-stake reference it must
//!   track for check (d)); when the service is not live, compose the
//!   `solana:` URL LOCALLY from the env-configured **stake-custody payee** (a
//!   SEPARATE wallet from the revenue payee — refundable principal never
//!   commingles) + a fresh on-curve reference, queue the intent under
//!   `.heal/stake/`, and print the honest not-live state. The claim lands via
//!   sync/reference auto-detect later.
//! - `stake --info` → fetch the account's tranche/head/pool view (`GET
//!   /stake/info?acct=`); when not live, render the local-queue fallback view.
//! - `unstake` → compose the drain-all-unlocked request (`POST
//!   /stake/unstake`); when not live, queue + render the escrow/venting
//!   disclosure (the mechanics copy is true regardless of transport).
//!
//! # The service contract mirrored (riir-dapps `src/kat/{staking,solana}.rs`)
//!
//! - URL: `solana:<payee>?amount=<X>&spl-token=<USDC>&reference=<ref>` —
//!   exactly the three load-bearing params (`payment_request_url`).
//! - base58: the Bitcoin alphabet Solana uses, hand-rolled here as the
//!   **mirrored** client copy (riir-dapps is NOT a dep — the boundary
//!   allowlist), pinned by the same math-matrix goldens.
//! - Tranche/account shapes: `StakeAccountHead` / `TrancheRow` /
//!   `StakingParams` — rendered as the proposal §CLI mock (with the
//!   amendment-(b) correction: each deposit owns its own 7-day clock; a
//!   top-up never re-locks earlier tranches).
//!
//! # The caveat-#15 copy rule (inherited verbatim, enforced by test)
//!
//! **Never a `$` figure beside a KAT accrual.** Accruals are labeled "service
//! credit"; the rate honestly floats with the burn pool (can be zero); **no
//! APY is promised anywhere** — the string "APY" appears only inside the
//! no-promise phrase. USDC principal/refund amounts may carry `$` (the
//! proposal's own unstake mock does) — the gate is the KAT-accrual side.
//! Pinned by the in-module copy-gate tests + the integration binary.

use std::path::{Path, PathBuf};

// ── base58 (the mirrored Bitcoin-alphabet codec; dapps solana.rs shape) ────

/// The base58 alphabet: 58 chars, excluding `0` `O` `I` `l` (the visual-
/// ambiguity characters). Pinned by tests (length, first/last, every char
/// as a single-byte round trip).
const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Base58 digit value of one ASCII byte, or `None` for non-alphabet bytes
/// (includes the four excluded chars — `0` `O` `I` `l` reject).
#[must_use]
fn b58_digit(c: u8) -> Option<u8> {
    match c {
        b'1'..=b'9' => Some(c - b'1'),
        b'A'..=b'H' => Some(c - b'A' + 9),
        b'J'..=b'N' => Some(c - b'J' + 17),
        b'P'..=b'Z' => Some(c - b'P' + 22),
        b'a'..=b'k' => Some(c - b'a' + 33),
        b'm'..=b'z' => Some(c - b'm' + 44),
        _ => None,
    }
}

/// Base58-encode (leading zero bytes → leading `1`s, the canonical rule).
///
/// The two truncating casts below are bounded by the divmod math
/// (`cur < 58·256` so `cur/58 < 256`; `rem < 58`) — the dapps mirror's
/// proven shape, allowed rather than restructured.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn b58_encode(bytes: &[u8]) -> String {
    let zeros = bytes.iter().take_while(|&&b| b == 0).count();
    let mut body = bytes[zeros..].to_vec();
    let mut digits: Vec<u8> = Vec::new();
    if !body.is_empty() {
        loop {
            // body = body / 58; rem = body % 58 (big-endian divmod).
            let mut rem: u32 = 0;
            let mut all_zero = true;
            for b in &mut body {
                let cur = (rem << 8) | u32::from(*b);
                *b = (cur / 58) as u8;
                rem = cur % 58;
                if *b != 0 {
                    all_zero = false;
                }
            }
            digits.push(rem as u8);
            if all_zero {
                break;
            }
        }
    }
    let mut out = String::with_capacity(zeros + digits.len());
    for _ in 0..zeros {
        out.push('1');
    }
    for &d in digits.iter().rev() {
        out.push(ALPHABET[d as usize] as char);
    }
    out
}

/// Base58-decode (inverse of [`b58_encode`]; `None` on any non-alphabet
/// byte or u8-overflow class the caller must guard).
#[must_use]
pub fn b58_decode(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    let mut bytes: Vec<u8> = Vec::new();
    for ch in s.bytes() {
        let d = b58_digit(ch)?;
        // bytes = bytes * 58 + d (big-endian mul-add).
        let mut carry = u32::from(d);
        for b in &mut bytes {
            let cur = (u32::from(*b) * 58) + carry;
            *b = (cur & 0xFF) as u8;
            carry = cur >> 8;
        }
        while carry > 0 {
            bytes.push((carry & 0xFF) as u8);
            carry >>= 8;
        }
    }
    // Leading `1`s are leading zero bytes.
    let zeros = s.bytes().take_while(|&c| c == b'1').count();
    let mut out = vec![0u8; zeros];
    out.extend(bytes.iter().rev().copied());
    Some(out)
}

/// Decode exactly `N` bytes (the pubkey/signature width guard); `None` when
/// the decode is not exactly `N` bytes.
#[must_use]
pub fn b58_decode_fixed<const N: usize>(s: &str) -> Option<[u8; N]> {
    let v = b58_decode(s)?;
    if v.len() != N {
        return None;
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&v);
    Some(out)
}

// ── amounts (integer money only — u64 base units end to end, G4-S) ─────────

/// USDC decimals on Solana (the dapps `USDC_DECIMALS` scale pin: 1 cent =
/// 10⁴ base units; 1 USDC = 10⁶).
pub const USDC_DECIMALS: u32 = 6;

/// Render base units as the exact 6-decimal string the URL carries —
/// `100_000_000` → `"100.000000"` (the dapps `format_amount_base` mirror;
/// a lossless integer formatting, never parsed back by us).
#[must_use]
pub fn format_amount_base(base: u64) -> String {
    let scale = 10_u64.pow(USDC_DECIMALS);
    format!("{}.{:06}", base / scale, base % scale)
}

/// Render base units for display as `100.00 USDC` (2 decimals — the proposal
/// mock's principal shape; integer-exact display, truncating beyond cents —
/// a DISPLAY form only, never a wire form).
#[must_use]
pub fn display_usd(base: u64) -> String {
    format!("{}.{:02}", base / 1_000_000, (base % 1_000_000) / 10_000)
}

/// Parse a `--usd` CLI value ("100", "100.5", "0.01") into base units.
/// Integer-exact: at most 6 fractional digits, no rounding — a value that
/// needs more precision is refused (money is never silently rounded).
#[must_use]
pub fn parse_usd_to_base(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() || s.starts_with('-') {
        return None;
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s, ""),
    };
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if frac.len() > USDC_DECIMALS as usize {
        return None;
    }
    // Bounded by the check above: frac.len() ≤ 6 = USDC_DECIMALS.
    #[allow(clippy::cast_possible_truncation)]
    let frac_len = frac.len() as u32;
    let whole_val: u64 = whole.parse().ok()?;
    let mut base = whole_val.checked_mul(10_u64.pow(USDC_DECIMALS))?;
    if !frac.is_empty() {
        let frac_digits: u64 = frac.parse().ok()?;
        let scale = 10_u64.pow(USDC_DECIMALS - frac_len);
        base = base.checked_add(frac_digits.checked_mul(scale)?)?;
    }
    Some(base)
}

// ── the transfer-request URL (the dapps payment_request_url mirror) ────────

/// `solana:<payee>?amount=<X>&spl-token=<USDC>&reference=<ref>` — exactly
/// the three load-bearing params (label/message are display-only; the host
/// may append them per the spec).
#[must_use]
pub fn stake_transfer_url(payee_b58: &str, amount_base: u64, mint_b58: &str, reference_b58: &str) -> String {
    format!(
        "solana:{payee_b58}?amount={}&spl-token={mint_b58}&reference={reference_b58}",
        format_amount_base(amount_base)
    )
}

// ── the fresh per-stake reference (the entropy pattern, ssh-key flavor) ────

/// Derive a fresh on-curve reference pubkey: a fresh Ed25519 keypair's
/// 32-byte verifying key (an Ed25519 public key IS a valid Solana pubkey —
/// the dapps `new_payment_request` pattern, the `ssh_key` crate's
/// `OsRng` instead of `ed25519-dalek` — already in the `kat_service` tree). The
/// reference must appear in the claiming transaction's account keys (check
/// (d)); fresh-per-request by construction (a new keypair each call).
///
/// # Panics
///
/// Panics only when the OS CSPRNG fails or the generated key is not
/// Ed25519 (both structurally impossible on a healthy host — the
/// [`crate::kat_account`] login generation's own posture).
#[must_use]
pub fn fresh_reference() -> [u8; 32] {
    let key = ssh_key::PrivateKey::random(&mut ssh_key::rand_core::OsRng, ssh_key::Algorithm::Ed25519)
        .expect("OsRng Ed25519 generation");
    *key.public_key()
        .key_data()
        .ed25519()
        .expect("ed25519 by construction")
        .as_ref()
}

// ── config resolution (the lease-URL pattern: env, no hidden defaults) ─────

/// The stake-transfer request schema version.
pub const STAKE_VERSION: u32 = 1;

/// The env var carrying the **stake-custody payee** (base58, 32B) — a
/// SEPARATE wallet from the revenue payee. No default: a custody wallet is
/// deployment identity, not a guess this client should make.
pub const ENV_STAKE_PAYEE: &str = "RIIR_HEAL_KAT_STAKE_PAYEE";

/// The env var carrying the accepted SPL-USDC mint (base58, 32B). Defaults
/// to mainnet USDC when unset (the mint a payment wallet would send anyway;
/// the service is authoritative at claim time regardless).
pub const ENV_USDC_MINT: &str = "RIIR_HEAL_KAT_USDC_MINT";

/// The mainnet SPL-USDC mint (Circle) — the display-default for
/// [`StakeConfig::resolve`]; `RIIR_HEAL_KAT_USDC_MINT` overrides for
/// testnets/own deployments.
pub const MAINNET_USDC_MINT_B58: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// The resolved staking config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeConfig {
    /// The stake-custody payee, base58.
    pub payee_b58: String,
    /// The accepted USDC mint, base58.
    pub usdc_mint_b58: String,
}

impl StakeConfig {
    /// Resolve from the environment. `Err` carries the honest one-line
    /// remedy (which env var, what shape) — never a silent default for the
    /// CUSTODY payee.
    ///
    /// # Errors
    ///
    /// A human-readable string when [`ENV_STAKE_PAYEE`] is unset or not a
    /// 32-byte base58 key, or the mint override is malformed.
    pub fn resolve() -> Result<Self, String> {
        let payee_b58 = std::env::var(ENV_STAKE_PAYEE).unwrap_or_default();
        if payee_b58.is_empty() {
            return Err(format!(
                "{ENV_STAKE_PAYEE} is not set — the stake-custody payee (base58, 32 bytes) is \
                 required to compose a stake transfer locally; set it or wait for the service plane"
            ));
        }
        if b58_decode_fixed::<32>(&payee_b58).is_none() {
            return Err(format!(
                "{ENV_STAKE_PAYEE} is not a valid base58 32-byte pubkey: {payee_b58}"
            ));
        }
        let usdc_mint_b58 = std::env::var(ENV_USDC_MINT).unwrap_or_default();
        let usdc_mint_b58 = if usdc_mint_b58.is_empty() {
            MAINNET_USDC_MINT_B58.to_string()
        } else {
            if b58_decode_fixed::<32>(&usdc_mint_b58).is_none() {
                return Err(format!(
                    "{ENV_USDC_MINT} is not a valid base58 32-byte pubkey: {usdc_mint_b58}"
                ));
            }
            usdc_mint_b58
        };
        Ok(Self {
            payee_b58,
            usdc_mint_b58,
        })
    }
}

// ── the local intent queue (.heal/stake/ — the claims-queue pattern) ───────

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

/// Append a stake intent to `.heal/stake/pending.jsonl`.
///
/// # Errors
///
/// The underlying `io::Error` from directory creation or the append.
pub fn queue_stake_intent(root: &Path, intent: &StakeIntent) -> std::io::Result<PathBuf> {
    use std::io::Write as _;
    let path = root.join(".heal").join("stake").join("pending.jsonl");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(f, "{}", serde_json::to_string(intent).unwrap_or_default())?;
    Ok(path)
}

/// Append an unstake intent to `.heal/stake/unstake_pending.jsonl`.
///
/// # Errors
///
/// The underlying `io::Error`.
pub fn queue_unstake_intent(root: &Path, intent: &UnstakeIntent) -> std::io::Result<PathBuf> {
    use std::io::Write as _;
    let path = root.join(".heal").join("stake").join("unstake_pending.jsonl");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(f, "{}", serde_json::to_string(intent).unwrap_or_default())?;
    Ok(path)
}

/// Count the locally-queued stake intents (the `--info` not-live fallback
/// view). `None` when no queue file exists.
#[must_use]
pub fn queued_stake_count(root: &Path) -> Option<usize> {
    let path = root.join(".heal").join("stake").join("pending.jsonl");
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.lines().filter(|l| !l.trim().is_empty()).count())
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
        let span = self.authority_share_bp.saturating_sub(self.authority_floor_bp);
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

// ── the renders (the proposal §CLI mock, amendment (b) corrected) ──────────

/// `1d 02h`-style tick countdown (ticks are seconds at the deployment
/// default — the display is a duration, honest when the host clock differs).
#[must_use]
pub fn countdown(ticks: u64) -> String {
    let days = ticks / 86_400;
    let hours = (ticks % 86_400) / 3_600;
    format!("{days}d {hours:02}h")
}

/// The stake copy (the mock's two-line form, verbatim semantics):
/// `stake 100.00 USDC → 7-day lockup → earn KAT from the burn pool
///  (custody wallet, refundable after unlock; no APY promise)`.
#[must_use]
pub fn stake_line(amount_base: u64) -> String {
    format!(
        "stake {} USDC → 7-day lockup → earn KAT from the burn pool\n       \
         (custody wallet, refundable after unlock; no APY promise)",
        display_usd(amount_base)
    )
}

/// The `--info` render (the mock's shape; per-tranche FIFO unlock rows +
/// the amendment-(b) clock wording — a top-up never re-locks earlier
/// tranches, which the pre-amendment mock line got wrong).
#[allow(clippy::too_many_lines)] // one render block per mock row — splitting would hide the shape
#[must_use]
pub fn render_info(info: &StakeInfo, now_tick: u64) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Staked:          {} USDC (refundable)",
        display_usd(info.staked_base)
    );
    // Per-tranche FIFO display: one Unlock row per not-yet-unlocked tranche
    // (oldest first); unlocked tranches fold into an "unlocked now" row.
    let mut any_locked = false;
    let mut unlocked_count = 0usize;
    let mut unlocked_base = 0u64;
    for t in &info.tranches {
        if t.unlock_at_tick <= now_tick {
            unlocked_count += 1;
            if t.kind == "usdc" {
                unlocked_base = unlocked_base.saturating_add(t.amount_base);
            }
        } else if t.kind == "usdc" {
            any_locked = true;
            let _ = writeln!(
                out,
                "Unlock:          in {}   (tranche #{} — each deposit owns its own 7-day clock)",
                countdown(t.unlock_at_tick.saturating_sub(now_tick)),
                t.seq
            );
        }
    }
    if !any_locked && unlocked_count > 0 {
        let _ = writeln!(
            out,
            "Unlock:          unlocked now — {} tranche(s), {} USDC drainable by `unstake`",
            unlocked_count,
            display_usd(unlocked_base)
        );
    } else if !any_locked && info.tranches.is_empty() {
        out.push_str("Unlock:          — (no tranches)\n");
    }
    // The KAT accrual — service credit, never a $ figure (caveat #15).
    let kat_whole = (info.accrued_kat_micro / 1_000_000).max(0);
    let kat_frac = (info.accrued_kat_micro % 1_000_000).unsigned_abs();
    let _ = writeln!(
        out,
        "Incentives:      +{kat_whole}.{kat_frac:06} KAT accrued — service credit, rate floats with burn"
    );
    let _ = writeln!(
        out,
        "Epoch pool:      mining {}% · staking {}% (usdc {}% / kat-reward {}% of the slice)",
        bp_pct(info.pool.mining_bp),
        bp_pct(info.pool.staking_bp),
        bp_pct(info.pool.usdc_share_bp),
        bp_pct(info.pool.kat_share_bp),
    );
    let _ = writeln!(
        out,
        "                 · authority share {}%→{}% over {} epochs",
        bp_pct(info.pool.authority_share_bp),
        bp_pct(info.pool.authority_floor_bp),
        info.pool.epochs_to_floor()
    );
    out.push_str(if info.restake {
        "Reward restake:  ON — mining credits auto-lock as KAT tranches (7-day clock each;\n                 redeem instantly after unlock; opt back out anytime)"
    } else {
        "Reward restake:  OFF — mining credits stay liquid (the never-brick default)"
    });
    out
}

/// bp → whole-percent display (1000 → "10"; 250 → "2.5").
#[must_use]
fn bp_pct(bp: u32) -> String {
    let whole = bp / 100;
    let frac = bp % 100;
    if frac == 0 {
        format!("{whole}")
    } else {
        format!("{whole}.{frac}")
    }
}

/// The unstake disclosure (the mock's form): the escrow window, the
/// venting rule, the refund-to-source binding, and the accrual stop — all
/// four disclosures present on every unstake render (the copy gate pins
/// them).
#[must_use]
pub fn unstake_disclosure() -> String {
    "Queued: refund enters a 48h escrow, then drains in the next daily batch\n\
     (exact-once; refunds over $10,000 vent in equal daily slices — 5 days).\n\
     Refund returns to the funding wallet (bound at first stake — never changeable\n\
     by the service key). Incentive accrual stopped at request."
        .to_string()
}

// ── the transport + the service-first flows (kat_transport) ────────────────

/// The staking-channel transport (one impl per protocol — the
/// `LeaseTransport` pattern; the ureq HTTP impl + the std-TCP mock in
/// tests drive the same trait).
#[cfg(feature = "kat_transport")]
pub trait StakeTransport {
    /// `POST /stake/request` (JSON body) → 200 with the service-composed
    /// transfer request `{"url": "solana:…"}`.
    fn post_stake_request(&self, json: &str) -> crate::kat_lease_client::HttpResponse;
    /// `GET /stake/info?acct=<id>` → 200 with the [`StakeInfo`] JSON.
    fn get_stake_info(&self, account_id: &str) -> crate::kat_lease_client::HttpResponse;
    /// `POST /stake/unstake` (JSON body) → 200 queued / error JSON.
    fn post_unstake(&self, json: &str) -> crate::kat_lease_client::HttpResponse;
}

/// The ureq HTTP transport over the shared service base URL (the
/// `HttpLeaseTransport` pattern: one bounded agent, JSON bodies).
#[cfg(feature = "kat_transport")]
pub struct HttpStakeTransport {
    agent: ureq::Agent,
    base: String,
}

#[cfg(feature = "kat_transport")]
impl HttpStakeTransport {
    /// Build with a per-request timeout in seconds (the explicit-command
    /// budget; every request — connect, headers, AND body read — covered).
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

    fn post_json(&self, path: &str, body: &str) -> crate::kat_lease_client::HttpResponse {
        let url = format!("{}{path}", self.base);
        match self
            .agent
            .post(&url)
            .set("Content-Type", "application/json")
            .send_string(body)
        {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => crate::kat_lease_client::HttpResponse::Status(c),
            Err(e) => crate::kat_lease_client::HttpResponse::Error(e.to_string()),
        }
    }
}

#[cfg(feature = "kat_transport")]
impl StakeTransport for HttpStakeTransport {
    fn post_stake_request(&self, json: &str) -> crate::kat_lease_client::HttpResponse {
        self.post_json("/stake/request", json)
    }

    fn get_stake_info(&self, account_id: &str) -> crate::kat_lease_client::HttpResponse {
        let url = format!("{}/stake/info?acct={}", self.base, account_id);
        match self.agent.get(&url).call() {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => crate::kat_lease_client::HttpResponse::Status(c),
            Err(e) => crate::kat_lease_client::HttpResponse::Error(e.to_string()),
        }
    }

    fn post_unstake(&self, json: &str) -> crate::kat_lease_client::HttpResponse {
        self.post_json("/stake/unstake", json)
    }
}

/// What a stake-request flow concluded (the CLI prints one line per arm;
/// the QR + URL print above them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StakeOutcome {
    /// The service answered with a composed transfer URL.
    ServiceUrl(String),
    /// Composed locally + queued (the not-live path). Carries the URL + the
    /// queue path.
    LocalQueued { url: String, queued_at: PathBuf },
}

/// The stake-request composition flow (service-first; the local fallback
/// needs [`StakeConfig::resolve`]). Exposed for the integration fixtures;
/// the CLI wraps it with the QR render + copy.
///
/// # Errors
///
/// A human-readable string when neither the service nor the local config
/// can compose the request (the honest no-custody-payee case), or the queue
/// write fails.
#[cfg(feature = "kat_transport")]
pub fn stake_request_flow(
    transport: &dyn StakeTransport,
    account_id: &str,
    amount_base: u64,
    root: &Path,
    now_unix: u64,
) -> Result<StakeOutcome, String> {
    let req = serde_json::json!({
        "stake_version": STAKE_VERSION,
        "account_id": account_id,
        "amount_base": amount_base,
    });
    if let crate::kat_lease_client::HttpResponse::Ok(body) =
        transport.post_stake_request(&req.to_string())
    {
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(url) = parsed.get("url").and_then(serde_json::Value::as_str) {
                return Ok(StakeOutcome::ServiceUrl(url.to_string()));
            }
        }
        return Err("service answered but the response carried no transfer URL".to_string());
    }
    // Not live: compose locally (fresh reference, env payee) + queue.
    let cfg = StakeConfig::resolve()?;
    let reference = fresh_reference();
    let reference_b58 = b58_encode(&reference);
    let url = stake_transfer_url(&cfg.payee_b58, amount_base, &cfg.usdc_mint_b58, &reference_b58);
    let intent = StakeIntent {
        stake_version: STAKE_VERSION,
        account_id: account_id.to_string(),
        payee_b58: cfg.payee_b58,
        usdc_mint_b58: cfg.usdc_mint_b58,
        reference_b58,
        amount_base,
        queued_unix: now_unix,
    };
    let queued_at = queue_stake_intent(root, &intent).map_err(|e| e.to_string())?;
    Ok(StakeOutcome::LocalQueued { url, queued_at })
}

/// The unstake flow (service-first; the not-live fallback queues + renders
/// the disclosure).
///
/// # Errors
///
/// A human-readable string on a queue-write failure.
#[cfg(feature = "kat_transport")]
pub fn unstake_flow(
    transport: &dyn StakeTransport,
    account_id: &str,
    root: &Path,
    now_unix: u64,
) -> Result<String, String> {
    let req = serde_json::json!({
        "stake_version": STAKE_VERSION,
        "account_id": account_id,
    });
    let body = match transport.post_unstake(&req.to_string()) {
        crate::kat_lease_client::HttpResponse::Ok(b) => Some(b),
        _ => None,
    };
    if let Some(b) = body {
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&b) {
            if let Some(err) = parsed.get("error").and_then(serde_json::Value::as_str) {
                let remaining = parsed.get("remaining_ticks").and_then(serde_json::Value::as_u64);
                return Ok(match (err, remaining) {
                    ("LockupNotExpired", Some(r)) => format!(
                        "unstake: refused — lockup not expired ({r} ticks ≈ {} remain)",
                        countdown(r)
                    ),
                    (other, _) => format!("unstake: refused — {other}"),
                });
            }
            if parsed.get("queued").and_then(serde_json::Value::as_bool) == Some(true) {
                let amount = parsed
                    .get("amount_base")
                    .and_then(serde_json::Value::as_u64)
                    .map_or_else(String::new, display_usd);
                return Ok(format!(
                    "unstake: queued by the service ({amount} USDC)\n{}",
                    unstake_disclosure()
                ));
            }
        }
        return Ok("unstake: service answered, but the response was not understood".to_string());
    }
    let intent = UnstakeIntent {
        stake_version: STAKE_VERSION,
        account_id: account_id.to_string(),
        queued_unix: now_unix,
    };
    let queued_at = queue_unstake_intent(root, &intent).map_err(|e| e.to_string())?;
    Ok(format!(
        "unstake: service plane not yet live — intent queued locally at {}\n{}",
        queued_at.display(),
        unstake_disclosure()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── base58: the dapps math-matrix goldens (the mirrored-codec pin) ──

    #[test]
    fn b58_all_zeros_are_leading_ones() {
        let system_program = "1".repeat(32);
        assert_eq!(b58_encode(&[0u8; 32]), system_program);
        assert_eq!(b58_decode(&system_program), Some(vec![0u8; 32]));
        assert_eq!(b58_decode_fixed::<32>(&system_program), Some([0u8; 32]));
        assert_eq!(b58_encode(&[]), "");
        assert_eq!(b58_decode(""), Some(vec![]));
        assert_eq!(b58_encode(&[0u8]), "1");
        assert_eq!(b58_decode("1"), Some(vec![0u8]));
    }

    #[test]
    fn b58_every_alphabet_char_round_trips_as_a_single_byte() {
        let mut chars = Vec::with_capacity(58);
        for i in 0u8..58 {
            let s = b58_encode(&[i]);
            assert_eq!(s.chars().count(), 1, "byte {i} must encode to one char");
            assert_eq!(b58_decode(&s), Some(vec![i]));
            chars.push(s);
        }
        let uniq: std::collections::HashSet<_> = chars.iter().collect();
        assert_eq!(uniq.len(), 58);
        assert_eq!(chars.first().unwrap(), "1");
        assert_eq!(chars.last().unwrap(), "z");
    }

    #[test]
    fn b58_leading_zeros_are_preserved_both_directions() {
        assert_eq!(b58_encode(&[0, 0, 1]), format!("11{}", b58_encode(&[1])));
        let round = b58_decode(&b58_encode(&[0u8, 0, 0, 5, 0])).unwrap();
        assert_eq!(round, vec![0u8, 0, 0, 5, 0]);
    }

    #[test]
    fn b58_round_trips_pseudo_random_pubkeys() {
        let mut seed = 0xDEAD_BEEF_CAFE_F00Fu64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            seed
        };
        for _ in 0..64 {
            let mut key = [0u8; 32];
            for chunk in key.chunks_mut(8) {
                chunk.copy_from_slice(&next().to_le_bytes());
            }
            let s = b58_encode(&key);
            assert_eq!(b58_decode_fixed::<32>(&s), Some(key));
        }
    }

    #[test]
    fn b58_rejects_invalid_chars_and_wrong_widths() {
        for bad in ["0", "O", "I", "l", "hello world!", "1O1"] {
            assert!(b58_decode_fixed::<32>(bad).is_none(), "{bad}");
        }
        assert!(b58_decode_fixed::<32>("1").is_none()); // right alphabet, wrong width
    }

    // ── the URL shape (the dapps three-param spec pin) ──────────────────

    #[test]
    fn transfer_url_is_the_three_param_spec_shape() {
        let payee = b58_encode(&[0x11; 32]);
        let mint = b58_encode(&[0x22; 32]);
        let reference = b58_encode(&[0x33; 32]);
        assert_eq!(
            stake_transfer_url(&payee, 100_000_000, &mint, &reference),
            format!("solana:{payee}?amount=100.000000&spl-token={mint}&reference={reference}")
        );
        // Non-integer base never appears: the amount is the exact 6-decimal
        // formatting of an integer.
        assert_eq!(
            stake_transfer_url(&payee, 1_000_050, &mint, &reference),
            format!("solana:{payee}?amount=1.000050&spl-token={mint}&reference={reference}")
        );
    }

    #[test]
    fn fresh_reference_is_a_distinct_full_width_key() {
        let a = fresh_reference();
        let b = fresh_reference();
        // Fresh-per-request: two draws are distinct (a collision would need
        // a 2^-128 event — this is a smoke, not a statistical claim).
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        // Encodes to a valid base58 pubkey width.
        assert_eq!(b58_decode_fixed::<32>(&b58_encode(&a)), Some(a));
    }

    // ── amounts ──────────────────────────────────────────────────────────

    #[test]
    fn usd_parse_is_integer_exact_and_refuses_over_precision() {
        assert_eq!(parse_usd_to_base("100"), Some(100_000_000));
        assert_eq!(parse_usd_to_base("100.5"), Some(100_500_000));
        assert_eq!(parse_usd_to_base("0.01"), Some(10_000));
        assert_eq!(parse_usd_to_base(" 7 "), Some(7_000_000));
        assert_eq!(parse_usd_to_base("100.1234567"), None); // 7 digits — refused, never rounded
        assert_eq!(parse_usd_to_base("-5"), None);
        assert_eq!(parse_usd_to_base("abc"), None);
        assert_eq!(parse_usd_to_base("1.2.3"), None);
        assert_eq!(parse_usd_to_base(""), None);
        assert_eq!(parse_usd_to_base("18446744073709.551616"), None); // u64 overflow — refused
    }

    #[test]
    fn amount_formatting_matches_the_wire_form() {
        assert_eq!(format_amount_base(100_000_000), "100.000000");
        assert_eq!(format_amount_base(1_000_050), "1.000050");
        assert_eq!(format_amount_base(0), "0.000000");
        assert_eq!(display_usd(100_000_000), "100.00");
    }

    // ── the renders ──────────────────────────────────────────────────────

    fn fixture_info() -> StakeInfo {
        StakeInfo {
            account_id: "9f3a".to_string(),
            staked_base: 100_000_000,
            accrued_kat_micro: 2_431_000_000, // 2,431 KAT
            restake: false,
            tranches: vec![
                TrancheView {
                    seq: 0,
                    kind: "usdc".into(),
                    amount_base: 60_000_000,
                    unlock_at_tick: 100_000 + 86_400 * 3 + 3_600 * 5, // ~3d 05h out
                },
                TrancheView {
                    seq: 1,
                    kind: "usdc".into(),
                    amount_base: 40_000_000,
                    unlock_at_tick: 100_000 + 86_400 * 4 + 3_600, // ~4d 01h out
                },
            ],
            pool: PoolView {
                mining_bp: 9_000,
                staking_bp: 1_000,
                usdc_share_bp: 7_000,
                kat_share_bp: 3_000,
                authority_share_bp: 2_000,
                authority_floor_bp: 500,
                authority_decay_bp: 25,
                epoch: 0,
                ticks_per_epoch: 604_800,
            },
        }
    }

    #[test]
    fn info_render_matches_the_mock_shape() {
        let text = render_info(&fixture_info(), 100_000);
        assert!(text.contains("Staked:          100.00 USDC (refundable)"), "{text}");
        // Per-tranche FIFO unlock rows (the mission's requirement).
        assert!(text.contains("Unlock:          in 3d 05h   (tranche #0"), "{text}");
        assert!(text.contains("tranche #1"), "{text}");
        // The amendment-(b) wording (NOT the superseded clock-reset line).
        assert!(text.contains("each deposit owns its own 7-day clock"), "{text}");
        assert!(!text.contains("top-ups restart"), "{text}");
        // The accrual line — service credit, no $ (caveat #15).
        assert!(
            text.contains("Incentives:      +2431.000000 KAT accrued — service credit, rate floats with burn"),
            "{text}"
        );
        // The epoch pool rows.
        assert!(
            text.contains("Epoch pool:      mining 90% · staking 10% (usdc 70% / kat-reward 30% of the slice)"),
            "{text}"
        );
        assert!(text.contains("authority share 20%→5% over 60 epochs"), "{text}");
        // The restake flag row.
        assert!(text.contains("Reward restake:  OFF"), "{text}");
    }

    #[test]
    fn stake_line_matches_the_mock_copy() {
        let line = stake_line(100_000_000);
        assert!(
            line.contains("stake 100.00 USDC → 7-day lockup → earn KAT from the burn pool"),
            "{line}"
        );
        assert!(line.contains("(custody wallet, refundable after unlock; no APY promise)"), "{line}");
    }

    #[test]
    fn unstake_disclosure_carries_all_four_disclosures() {
        let d = unstake_disclosure();
        assert!(d.contains("48h escrow"), "{d}");
        assert!(d.contains("daily batch"), "{d}");
        assert!(d.contains("vent in equal daily slices"), "{d}");
        assert!(d.contains("Refund returns to the funding wallet"), "{d}");
        assert!(d.contains("never changeable"), "{d}");
        assert!(d.contains("Incentive accrual stopped at request"), "{d}");
    }

    // ── the caveat-#15 copy gate (the no_dollar_sign_in_any_render
    // precedent, extended to the staking surface) ────────────────────────

    #[test]
    fn copy_gate_no_dollar_beside_a_kat_accrual() {
        // Every render this module can produce: gate each LINE — no line
        // carries both a `$` and a KAT/accrual figure.
        let renders = [
            stake_line(100_000_000),
            render_info(&fixture_info(), 100_000),
            unstake_disclosure(),
        ];
        for text in &renders {
            for line in text.lines() {
                let mentions_kat =
                    line.contains("KAT") || line.contains("accru") || line.contains("Incentive");
                assert!(
                    !(mentions_kat && line.contains('$')),
                    "caveat #15: a KAT/accrual line must never carry a $ figure: {line}"
                );
            }
        }
    }

    #[test]
    fn copy_gate_accruals_are_service_credit_and_no_apy_promise() {
        let info = render_info(&fixture_info(), 100_000);
        assert!(info.contains("service credit"), "{info}");
        // "APY" appears ONLY inside the no-promise phrase.
        for text in [stake_line(1_000_000), info.clone(), unstake_disclosure()] {
            if text.contains("APY") {
                assert!(text.contains("no APY promise"), "APY must only appear in the no-promise phrase: {text}");
            }
        }
    }

    #[test]
    fn copy_gate_unstake_discloses_queue_delay_and_escrow() {
        let d = unstake_disclosure();
        assert!(d.contains("escrow"), "{d}");
        assert!(d.contains("vent"), "{d}");
    }

    // ── the queue + pool math ────────────────────────────────────────────

    #[test]
    fn pool_authority_schedule_math() {
        let mut pool = fixture_info().pool;
        assert_eq!(pool.authority_share_now_bp(), 2_000);
        assert_eq!(pool.epochs_to_floor(), 60); // (2000-500)/25
        pool.epoch = 30;
        assert_eq!(pool.authority_share_now_bp(), 2_000 - 25 * 30); // 1250
        pool.epoch = 100;
        assert_eq!(pool.authority_share_now_bp(), 500); // clamped at the floor
    }

    #[test]
    fn stake_intent_queues_and_counts() {
        let dir = std::env::temp_dir().join(format!("kat_stake_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let intent = StakeIntent {
            stake_version: STAKE_VERSION,
            account_id: "acct".into(),
            payee_b58: b58_encode(&[0x11; 32]),
            usdc_mint_b58: b58_encode(&[0x22; 32]),
            reference_b58: b58_encode(&[0x33; 32]),
            amount_base: 100_000_000,
            queued_unix: 1,
        };
        queue_stake_intent(&dir, &intent).unwrap();
        queue_stake_intent(&dir, &intent).unwrap();
        assert_eq!(queued_stake_count(&dir), Some(2));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn countdown_formats_days_and_hours() {
        assert_eq!(countdown(86_400 * 4 + 3 * 3_600), "4d 03h");
        assert_eq!(countdown(0), "0d 00h");
    }

    #[test]
    fn config_resolution_validates_base58_shapes() {
        // A valid 32B payee + the mint default.
        std::env::set_var(ENV_STAKE_PAYEE, b58_encode(&[0x44; 32]));
        let cfg = StakeConfig::resolve().unwrap();
        assert_eq!(cfg.usdc_mint_b58, MAINNET_USDC_MINT_B58);
        // The default mint itself must decode to 32 bytes (shape pin).
        assert!(b58_decode_fixed::<32>(MAINNET_USDC_MINT_B58).is_some());
        // A malformed payee is refused with the remedy text.
        std::env::set_var(ENV_STAKE_PAYEE, "not-a-key!!");
        let err = StakeConfig::resolve().unwrap_err();
        assert!(err.contains(ENV_STAKE_PAYEE), "{err}");
        std::env::remove_var(ENV_STAKE_PAYEE);
        // Unset → the honest remedy.
        let err = StakeConfig::resolve().unwrap_err();
        assert!(err.contains(ENV_STAKE_PAYEE), "{err}");
    }
}
