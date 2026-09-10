//! The pay-request CLI client (riir-clippy Proposal 011 T1.3, Plan 062) —
//! `cargo heal pay --request [--amount N] [--label L] [--wait SECS] [--qr]`.
//!
//! The payee half of the §1 loop: create a signed pay request on the
//! kat-service, print the **shareable link first** (ask 13 — the link was
//! always the primitive; the QR is a rendering, `--qr` renders it as ASCII),
//! then **poll on credit** (the CLI blocks until the request settles or the
//! wait budget runs out, then prints the invoice line).
//!
//! # Wire mirror, no dep
//!
//! riir-dapps is NOT a dependency (the boundary allowlist; the lease-client
//! wire-mirror precedent). The bytes this module signs —
//! [`create_message`] — are a MIRROR of `riir_dapps::kat::payreq`'s
//! composer, pinned to it by a golden digest test on BOTH sides
//! (independently computed, so a mirror drift fails both repos). The
//! service derives the account from the signed pubkey (`blake3(pk)[..16]`,
//! the [`account_for_pubkey`] mirror) — the wire carries no account id.
//!
//! # Feature shape
//!
//! `kat_service` alone gets the mirrors + parsing + renders (std + blake3 +
//! ed25519 only — already in the tree). The network half ([`PayreqTransport`],
//! the flow, the poll) is `kat_transport`, the stake/vote-client convention.

// The payee create needs NO queue fallback: without the service there is
// no request id, and an unsharable link-first request is not a request.
// (The payer-side CLI — `pay --settle <link>`, T1.2's web wallet — is where
// the vote/stake queued-intent pattern returns.)

#[cfg(feature = "kat_transport")]
use crate::kat_lease_client::HttpResponse;

// ── hex + the account derivation (mirrors) ────────────────────────── ─────

/// Lowercase hex (the wire encoding for ids/pubkeys/signatures).
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

/// MIRROR of `riir_dapps::kat::payreq::account_for_pubkey` (and the sync
/// plane's `submitter_for`): `blake3(pubkey)[..16]` — the account the
/// service attributes the signature to. **Do not drift**: a different
/// derivation here would create requests for an account no route can
/// settle from.
#[must_use]
pub fn account_for_pubkey(pubkey: &[u8; 32]) -> [u8; 16] {
    let d = blake3::hash(pubkey);
    let mut out = [0u8; 16];
    out.copy_from_slice(&d.as_bytes()[..16]);
    out
}

/// MIRROR of dapps `create_message` — the exact signed bytes:
/// `kat:payreq:create:v1:` ‖ `payee_pubkey[32]` ‖ `amount` i64 le ‖ `label`.
/// **Do not drift**: the service verifies over its own composer, and the
/// golden digest test holds the two sides together.
#[must_use]
pub fn create_message(payee_pubkey: &[u8; 32], amount: i64, label: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(21 + 32 + 8 + label.len());
    msg.extend_from_slice(b"kat:payreq:create:v1:");
    msg.extend_from_slice(payee_pubkey);
    msg.extend_from_slice(&amount.to_le_bytes());
    msg.extend_from_slice(label);
    msg
}

/// The label every faucet-preset request carries — MIRROR of
/// `riir_dapps::kat::payreq::FAUCET_LABEL` (§11: "a payreq whose label is
/// `faucet`"). It rides the canonical request id, so a faucet request and
/// a pay request opened by the same account in the same tick get
/// different ids by construction.
pub const FAUCET_LABEL: &str = "faucet";

/// MIRROR of dapps `faucet_message` (T1.8, ask 11) — the exact signed
/// bytes: `kat:payreq:faucet:v1:` ‖ `requester_pubkey[32]`. Signed by the
/// REQUESTER: only the key holder opens a faucet request on their own
/// account.
///
/// **The amount is deliberately NOT in the message.** A faucet request is
/// pinned server-side to the deployment's `faucet_daily_cap_kat`, so there
/// is nothing for a requester to choose and nothing for an attacker to
/// inflate by replaying a captured signature at a different value — the
/// opposite discipline from `settle_message`, where the amount is INSIDE
/// the signature precisely because the payer chooses it.
///
/// **Do not drift**: the service verifies over its own composer, and the
/// golden digest test ([`tests::faucet_message_golden_digest`]) holds the
/// two sides together.
#[must_use]
pub fn faucet_message(requester_pubkey: &[u8; 32]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(21 + 32);
    msg.extend_from_slice(b"kat:payreq:faucet:v1:");
    msg.extend_from_slice(requester_pubkey);
    msg
}
/// MIRROR of dapps `claim_message` (T079) — the exact signed bytes:
/// `kat:payreq:claim:v1:` ‖ `pubkey[32]`. Signed by the ACCOUNT KEY: the
/// onboarding step that mints the once-per-key free-tier grant
/// (`FREE_GRANT_KAT`, the `granted` flag the welcome gate reads).
///
/// **Nothing request-shaped rides the message** — the amount is the
/// ledger's genesis constant and the once-per-key bound is the ledger's
/// `AlreadySettled` refusal, so a captured signature is worth exactly one
/// replay of a claim the key holder was already entitled to.
///
/// **Do not drift**: the service verifies over its own composer, and the
/// golden digest test ([`tests::claim_message_golden_digest`]) holds the
/// two sides together.
#[must_use]
pub fn claim_message(pubkey: &[u8; 32]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(20 + 32);
    msg.extend_from_slice(b"kat:payreq:claim:v1:");
    msg.extend_from_slice(pubkey);
    msg
}

/// The account-claim signed body — `POST /account/claim`. The wire
/// carries the pubkey and the signature and **nothing else** (same
/// discipline as [`faucet_request_body`]: nothing on the wire for a
/// requester to choose or an attacker to inflate).
#[must_use]
pub fn claim_request_body(key: &ed25519_dalek::SigningKey) -> String {
    use ed25519_dalek::Signer as _;
    let pubkey = key.verifying_key().to_bytes();
    let signature = key.sign(&claim_message(&pubkey));
    serde_json::json!({
        "pubkey_hex": hex_encode(&pubkey),
        "signature_hex": hex_encode(&signature.to_bytes()),
    })
    .to_string()
}

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

/// Parse the `POST /account/claim` 200 body.
///
/// # Errors
///
/// A human-readable string when the body is not JSON, carries the
/// service's `error`, or is missing a field.
pub fn parse_claim_response(body: &[u8]) -> Result<ClaimResponse, String> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "the claim response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the service refused the claim — {err}"));
    }
    Ok(ClaimResponse {
        account_id_hex: v
            .get("account_id")
            .and_then(serde_json::Value::as_str)
            .ok_or("the claim response carried no `account_id`")?
            .to_string(),
        granted: v
            .get("granted")
            .and_then(serde_json::Value::as_bool)
            .ok_or("the claim response carried no `granted`")?,
        already: v
            .get("already")
            .and_then(serde_json::Value::as_bool)
            .ok_or("the claim response carried no `already`")?,
        balance_micro: v
            .get("balance_micro")
            .and_then(serde_json::Value::as_i64)
            .ok_or("the claim response carried no `balance_micro`")?,
        claimed_micro: v
            .get("claimed_micro")
            .and_then(serde_json::Value::as_i64)
            .ok_or("the claim response carried no `claimed_micro`")?,
    })
}

/// The ensure-grant flow (T079): claim the once-per-key free-tier grant
/// if the account does not have it yet. Read-first — a granted account
/// costs ZERO requests, so `sync`/`account` can call this unconditionally.
///
/// Returns the claim response, or `None` when the balance read showed the
/// grant already in place (nothing was sent).
///
/// # Errors
///
/// A human-readable string on an unreachable service, a non-200, or an
/// unparseable body.
#[cfg(feature = "kat_transport")]
pub fn ensure_grant_flow(
    transport: &dyn PayreqTransport,
    key: &ed25519_dalek::SigningKey,
    account_hex: &str,
) -> Result<Option<ClaimResponse>, String> {
    let balance_body = match transport.get_balance(account_hex) {
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(c) => {
            return Err(format!(
                "the balance read answered HTTP {c} — cannot decide the claim;                  run `cargo heal account` once the service is reachable"
            ))
        }
        HttpResponse::NotModified => return Err("the balance read answered 304".to_string()),
        HttpResponse::Error(e) => {
            return Err(format!("the service is unreachable ({e}) — cannot decide the claim"))
        }
    };
    let balance = crate::kat_account_client::parse_balance(&balance_body)?;
    if balance.granted {
        return Ok(None);
    }
    let body = match transport.post_claim(&claim_request_body(key)) {
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(c) => return Err(format!("the claim was refused (HTTP {c})")),
        HttpResponse::NotModified => return Err("the claim answered 304".to_string()),
        HttpResponse::Error(e) => return Err(format!("the claim failed: {e}")),
    };
    parse_claim_response(&body).map(Some)
}

/// The faucet-request signed body — `POST /payreq/faucet`.
///
/// The wire carries the pubkey and the signature and **nothing else**: no
/// amount (the ledger pins it), no ttl (`link_ttl_secs` does), no account
/// (derived from the signed pubkey). Composed as a named function rather
/// than inline so the "no amount on the wire" property is pinned by a test
/// ([`tests::the_faucet_body_carries_no_amount_and_no_ttl`]) instead of by
/// a code reading.
#[must_use]
pub fn faucet_request_body(key: &ed25519_dalek::SigningKey) -> String {
    use ed25519_dalek::Signer as _;
    let pubkey = key.verifying_key().to_bytes();
    let signature = key.sign(&faucet_message(&pubkey));
    serde_json::json!({
        "pubkey_hex": hex_encode(&pubkey),
        "signature_hex": hex_encode(&signature.to_bytes()),
    })
    .to_string()
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

/// Parse the `POST /payreq/faucet` 200 body.
///
/// # Errors
///
/// A human-readable string when the body is not JSON, carries the
/// service's `error`, or is missing a field. A refusal body is surfaced
/// verbatim — the ledger's four honest refusals (faucet disabled, service
/// account, daily cap used, an open request already exists) are the
/// operator's actual answer and must not be reworded into a timeout.
pub fn parse_faucet_request(body: &[u8]) -> Result<FaucetRequest, String> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "the faucet response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the service refused the faucet request — {err}"));
    }
    Ok(FaucetRequest {
        id_hex: v
            .get("request_id")
            .and_then(serde_json::Value::as_str)
            .ok_or("the faucet response carried no `request_id`")?
            .to_string(),
        amount_micro: v
            .get("amount_micro")
            .and_then(serde_json::Value::as_i64)
            .ok_or("the faucet response carried no `amount_micro`")?,
        expiry: v
            .get("expiry")
            .and_then(serde_json::Value::as_u64)
            .ok_or("the faucet response carried no `expiry`")?,
    })
}

/// The shareable link (ask 13 — link-first): `{base}/payreq/{32 hex}` with
/// any trailing `/` on the configured base trimmed (the
/// `kat_lease_client::payment_url` base rule, mirrored).
#[must_use]
pub fn request_url(service_base: &str, id: &[u8; 16]) -> String {
    request_url_from_id_hex(service_base, &hex_encode(id))
}

/// Compose the link from a hex id (the caller-supplied base — the CLI's
/// configured service URL, trailing `/` trimmed like [`request_url`]).
#[must_use]
pub fn request_url_from_id_hex(service_base: &str, id_hex: &str) -> String {
    format!("{}/payreq/{}", service_base.trim_end_matches('/'), id_hex)
}

/// KAT → micro-units (1 KAT = 10⁶ micro). At most 6 fractional digits;
/// `None` on junk, negatives, or more precision than micro-units can
/// carry. `""` → `None` is the CALLER's any-amount signal, never parsed
/// here (a blank `--amount` is ask 3, not zero).
#[must_use]
pub fn parse_kat_amount(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
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
    if frac.len() > 6 {
        return None;
    }
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut micro = whole.checked_mul(1_000_000)?;
    let mut scale = 100_000_i64;
    for c in frac.bytes() {
        micro = micro.checked_add(i64::from(c - b'0') * scale)?;
        scale /= 10;
    }
    (micro > 0).then_some(micro)
}

/// Render micro-units as a KAT decimal (the invoice/credit lines; exactly
/// the precision the value carries — trailing zeros kept so a pinned
/// `1.500000` reads back the way it was written).
#[must_use]
pub fn render_kat(micro: i64) -> String {
    let sign = if micro < 0 { "-" } else { "" };
    let abs = micro.unsigned_abs();
    format!("{}{}.{:06} KAT", sign, abs / 1_000_000, abs % 1_000_000)
}

// ── the transport + the flow (kat_transport) ────────────────────────────

/// The payreq-channel transport (one impl per protocol — the
/// `VoteTransport` pattern; the ureq HTTP impl + the in-memory scripted
/// mock in tests drive the same trait).
#[cfg(feature = "kat_transport")]
pub trait PayreqTransport {
    /// `POST /payreq/create` (signed JSON body) → 200 with
    /// `{"request_id": "<32 hex>", ...}`.
    fn post_create(&self, json: &str) -> HttpResponse;
    /// `GET /payreq/{id}` → 200 with the request view JSON.
    fn get_request(&self, id_hex: &str) -> HttpResponse;
    /// `GET /payreq/{id}/invoice` → 200 with the invoice JSON (404 until
    /// settled).
    fn get_invoice(&self, id_hex: &str) -> HttpResponse;
    /// `POST /payreq/faucet` (signed JSON body) → 200 with
    /// `{"request_id","created","kind":"faucet","amount_micro","expiry"}`.
    ///
    /// Deliberately NOT given a default implementation: a defaulted method
    /// is an absence that compiles, and this one moves money. Every
    /// transport states its faucet posture explicitly.
    fn post_faucet(&self, json: &str) -> HttpResponse;
    /// `GET /balance?acct=` — the read the claim decision consumes
    /// (grant already in place?).
    fn get_balance(&self, acct_hex: &str) -> HttpResponse;
    /// `POST /account/claim` (signed JSON body) → 200 with
    /// `{"account_id","granted","already","balance_micro","claimed_micro"}`.
    ///
    /// Deliberately NOT given a default implementation, same posture as
    /// [`Self::post_faucet`]: this mints the free-tier grant. Every
    /// transport states its claim posture explicitly.
    fn post_claim(&self, json: &str) -> HttpResponse;
}

/// The ureq HTTP transport over the shared service base URL (the
/// `HttpVoteTransport` pattern: one bounded agent, JSON bodies).
#[cfg(feature = "kat_transport")]
pub struct HttpPayreqTransport {
    agent: ureq::Agent,
    base: String,
}

/// The pure half of [`HttpPayreqTransport::refusal_or_status`] — split out
/// so the decision is unit-testable without a live `ureq::Response`
/// (constructing one needs a socket, and an untestable branch on a money
/// path is an assumption, not a rule).
#[cfg(feature = "kat_transport")]
fn refusal_body_or_status(status: u16, body: Vec<u8>) -> HttpResponse {
    match serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        }) {
        Some(_) => HttpResponse::Ok(body),
        None => HttpResponse::Status(status),
    }
}

#[cfg(feature = "kat_transport")]
impl HttpPayreqTransport {
    /// Build with a per-request timeout in seconds (connect, headers, AND
    /// body read — the explicit-command budget).
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

    fn body(resp: ureq::Response) -> HttpResponse {
        use std::io::Read as _;
        match resp.status() {
            304 => HttpResponse::NotModified,
            200 => {
                let mut buf = Vec::new();
                match resp.into_reader().read_to_end(&mut buf) {
                    Ok(_) => HttpResponse::Ok(buf),
                    Err(e) => HttpResponse::Error(e.to_string()),
                }
            }
            c => HttpResponse::Status(c),
        }
    }

    /// A non-200 whose body carries the service's own `{"error": ...}`
    /// becomes [`HttpResponse::Ok`] of that body, so the parse layer — which
    /// already reads `error` FIRST — is the single arbiter.
    ///
    /// **Why this exists (a real hole, not a style preference).** Every
    /// payreq refusal is `RouteResp::error(status, reason)` on the service
    /// side, i.e. a non-200 WITH `{"error":"<the operator's words>"}`.
    /// `ureq` reports that as `Err(Error::Status(c, resp))`, and dropping
    /// `resp` reduced all four documented faucet refusals (faucet disabled,
    /// service account, daily cap used, one already open) to a bare
    /// `HTTP 400`. The doc contract on [`parse_faucet_request`] promises
    /// those words survive **verbatim**; without this the promise was
    /// unreachable on the deployed path while every mock-driven test passed
    /// — the "unit-gate paths production cannot reach" class.
    ///
    /// A non-200 with no readable JSON `error` stays
    /// [`HttpResponse::Status`]: an opaque failure must not be dressed up
    /// as an answer.
    ///
    /// Deliberately applied to the POST arms only. The GET arms answer 404
    /// for the ordinary *not settled yet* case, and their callers already
    /// carry per-state copy for it — turning that into a parse error would
    /// reword the common case, which is the same defect in the other
    /// direction.
    fn refusal_or_status(status: u16, resp: ureq::Response) -> HttpResponse {
        use std::io::Read as _;
        let mut buf = Vec::new();
        match resp.into_reader().read_to_end(&mut buf) {
            Err(_) => HttpResponse::Status(status),
            Ok(_) => refusal_body_or_status(status, buf),
        }
    }

    fn post_json(&self, path: &str, body: &str) -> HttpResponse {
        let mut base = crate::kat_lease_client::request_base(&self.base);
        loop {
            let url = format!("{base}{path}");
            match self
                .agent
                .post(&url)
                .set("Content-Type", "application/json")
                .send_string(body)
            {
                Ok(r) => return Self::body(r),
                Err(ureq::Error::Status(c, r)) => return Self::refusal_or_status(c, r),
                Err(e) => match crate::kat_lease_client::failover_base(&base, &e) {
                    Some(next) => base = next,
                    None => return HttpResponse::Error(e.to_string()),
                },
            }
        }
    }

    fn get(&self, path: &str) -> HttpResponse {
        let mut base = crate::kat_lease_client::request_base(&self.base);
        loop {
            let url = format!("{base}{path}");
            match self.agent.get(&url).call() {
                Ok(r) => return Self::body(r),
                Err(ureq::Error::Status(c, _)) => return HttpResponse::Status(c),
                Err(e) => match crate::kat_lease_client::failover_base(&base, &e) {
                    Some(next) => base = next,
                    None => return HttpResponse::Error(e.to_string()),
                },
            }
        }
    }
}

#[cfg(feature = "kat_transport")]
impl PayreqTransport for HttpPayreqTransport {
    fn post_create(&self, json: &str) -> HttpResponse {
        self.post_json("/payreq/create", json)
    }

    fn get_request(&self, id_hex: &str) -> HttpResponse {
        self.get(&format!("/payreq/{id_hex}"))
    }

    fn get_invoice(&self, id_hex: &str) -> HttpResponse {
        self.get(&format!("/payreq/{id_hex}/invoice"))
    }

    fn post_faucet(&self, json: &str) -> HttpResponse {
        self.post_json("/payreq/faucet", json)
    }

    fn get_balance(&self, acct_hex: &str) -> HttpResponse {
        self.get(&format!("/balance?acct={acct_hex}"))
    }

    fn post_claim(&self, json: &str) -> HttpResponse {
        self.post_json("/account/claim", json)
    }
}

// ── the invoice view + the bit-exact recheck (T1.4) ─────────────────────

/// The parsed invoice row — `GET /payreq/{id}/invoice` (riir-dapps
/// `payreq_invoice_route`).
///
/// Wire-field note: the service's QR slot is spelled `qr_png_blake3` and
/// is **all-zero until the T1.4 render step fills it** (dapps
/// `invoice_set_qr_hash`, fills exactly once). This repo commits the QR
/// **module matrix** (`kat_lease_client::qr_commitment`) rather than a PNG,
/// because `qrcode` ships here with `default-features = false` — there is
/// no `image` in the tree and so no PNG to hash. The matrix is the
/// stronger artifact (any rendering is a function of it), and it is what
/// the recheck compares against.
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

/// Parse the invoice JSON the service serves.
///
/// # Errors
///
/// A human-readable string when the body is not JSON, carries an `error`,
/// or is missing a field the display needs (a partially-read invoice is
/// never rendered as a whole one).
pub fn parse_invoice(body: &[u8]) -> Result<InvoiceView, String> {
    let v: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| "the invoice response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the service refused the invoice read — {err}"));
    }
    let s = |k: &str| -> Result<String, String> {
        v.get(k)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("the invoice carried no `{k}`"))
    };
    Ok(InvoiceView {
        id_hex: s("request_id")?,
        payee_hex: s("payee")?,
        payer_hex: s("payer")?,
        token: s("token")?,
        amount_paid_micro: v
            .get("amount_paid")
            .and_then(serde_json::Value::as_i64)
            .ok_or("the invoice carried no `amount_paid`")?,
        settled_tick: v
            .get("settled")
            .and_then(serde_json::Value::as_u64)
            .ok_or("the invoice carried no `settled`")?,
        qr_commitment_hex: v
            .get("qr_png_blake3")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

/// The verdict of the bit-exact recheck (Proposal 011 §1, "Bit-exact
/// recheck before display"). **Three states, not two** — caveat #3's
/// re-encode-on-read fallback exists precisely because "cannot verify" is
/// not "does not match", and collapsing them would either refuse every
/// pre-bump asset or silently pass every unverified one.
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrRecheck {
    /// A fresh re-encode from the canonical request URL is byte-identical
    /// to the stored commitment — display is authorized.
    Match {
        /// The agreed digest, hex.
        digest_hex: String,
    },
    /// The stored commitment and a fresh re-encode DISAGREE. Display is
    /// REFUSED. Two causes, both refusals: the asset was altered, or the
    /// service committed a different artifact / different render params
    /// than this build's pinned QR contract.
    Mismatch {
        /// What the service stored.
        stored_hex: String,
        /// What this build re-encodes to (`"<unencodable>"` when the URL
        /// does not fit any QR version).
        recomputed_hex: String,
    },
    /// No stored commitment to compare against (the service has not filled
    /// the slot, or the asset predates the pin). Caveat #3's fallback:
    /// display is allowed but **labelled** — never silently.
    Unavailable {
        /// Why the bit-exact leg could not run.
        reason: String,
    },
}

/// The bit-exact recheck: re-encode the QR from the canonical request URL
/// and compare against the invoice's stored commitment.
///
/// The `request_url` MUST be the canonical capability link
/// ([`request_url_from_id_hex`]) — the same bytes the QR asset was
/// encoded from. Passing anything else turns a healthy invoice into a
/// [`QrRecheck::Mismatch`], which is the correct direction to fail in.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn recheck_invoice_qr(inv: &InvoiceView, request_url: &str) -> QrRecheck {
    let recomputed = crate::kat_lease_client::qr_commitment(request_url);
    match (inv.qr_commitment_is_unset(), recomputed) {
        // The service slot is still the all-zero sentinel: the bit-exact
        // leg has nothing to compare against. NOT a tamper — an all-zero
        // stored hash read as a mismatch would flag every invoice the
        // service has ever written.
        (true, _) => QrRecheck::Unavailable {
            reason: "the service has not committed a QR asset digest for this invoice \
                     (the slot is still the unset sentinel)"
                .to_string(),
        },
        (false, None) => QrRecheck::Mismatch {
            stored_hex: inv.qr_commitment_hex.clone(),
            recomputed_hex: "<unencodable>".to_string(),
        },
        (false, Some(d)) => {
            let recomputed_hex = hex_encode(&d);
            if recomputed_hex == inv.qr_commitment_hex {
                QrRecheck::Match {
                    digest_hex: recomputed_hex,
                }
            } else {
                QrRecheck::Mismatch {
                    stored_hex: inv.qr_commitment_hex.clone(),
                    recomputed_hex,
                }
            }
        }
    }
}

/// The compact paid line (the `pay` command's one-line payoff).
/// Caveat-#15 copy: KAT amounts, never a `$`, never a redemption promise.
#[must_use]
pub fn invoice_line(inv: &InvoiceView) -> String {
    let payer_short: String = inv.payer_hex.chars().take(8).collect();
    format!(
        "paid: {} from {payer_short}… — invoice {}",
        render_kat(inv.amount_paid_micro),
        inv.id_hex
    )
}

/// The full invoice display (T1.4) — gated on the recheck.
///
/// On [`QrRecheck::Mismatch`] the invoice body is **not rendered at all**:
/// the proposal's rule is "any mismatch ⇒ refuse to render + tamper
/// indicator", and printing the amount beside a tamper warning is exactly
/// the screenshot that rule exists to prevent.
///
/// Every line obeys the caveat-#15 copy rule: no `$`, and the KAT figure is
/// labelled **service credit** — pinned by
/// [`tests::invoice_display_obeys_the_copy_rule_in_every_recheck_state`].
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn invoice_display_lines(inv: &InvoiceView, recheck: &QrRecheck) -> Vec<String> {
    match recheck {
        QrRecheck::Mismatch {
            stored_hex,
            recomputed_hex,
        } => vec![
            format!("invoice {} REFUSED — tamper indicator", inv.id_hex),
            "  the stored QR commitment and a fresh re-encode disagree; either the asset was \
             altered or it was committed under different render params than this build pins"
                .to_string(),
            format!("  stored:     {stored_hex}"),
            format!("  re-encoded: {recomputed_hex}"),
            "  nothing from this invoice is displayed — report it before acting on it".to_string(),
        ],
        QrRecheck::Match { digest_hex } => {
            let mut lines = invoice_body_lines(inv);
            lines.push(format!(
                "  qr asset:  re-encode matches the stored commitment {} (bit-exact)",
                &digest_hex[..digest_hex.len().min(16)]
            ));
            lines
        }
        QrRecheck::Unavailable { reason } => {
            let mut lines = invoice_body_lines(inv);
            lines.push(format!("  qr asset:  NOT bit-exact-verified — {reason}"));
            lines.push(
                "  (caveat #3 fallback: the re-encode leg needs a committed digest; the row \
                 below is the ledger's, which stands on its own)"
                    .to_string(),
            );
            lines
        }
    }
}

/// The invoice body — shared by the two display states that render it, so
/// the copy exists once.
#[cfg(feature = "kat_transport")]
fn invoice_body_lines(inv: &InvoiceView) -> Vec<String> {
    vec![
        invoice_line(inv),
        format!("  invoice:   {}", inv.id_hex),
        format!("  payee:     {}", inv.payee_hex),
        format!("  payer:     {}", inv.payer_hex),
        format!("  token:     {} (service credit)", inv.token),
        format!(
            "  amount:    {} (what actually arrived)",
            render_kat(inv.amount_paid_micro)
        ),
        format!("  settled:   service tick {}", inv.settled_tick),
    ]
}

/// What the poll saw once the request reached a terminal settle — three
/// states, because "settled but the invoice did not read back" is neither
/// "not settled" nor "here is your invoice".
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettleOutcome {
    /// The settle landed and the invoice row read back.
    Invoice(InvoiceView),
    /// The settle landed but the invoice row did not read back.
    Unreadable(String),
}

/// What a pay-request flow concluded (a plain struct — one shape, no
/// match; the CLI renders the fields directly).
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayRequestOutcome {
    /// The canonical request id, hex (the link's tail, the invoice id).
    pub id_hex: String,
    /// The shareable link (ask 13 — print this FIRST).
    pub url: String,
    /// The amount as created (0 = any amount).
    pub amount_micro: i64,
    /// The settle the poll saw before the wait budget ended; `None` = not
    /// settled yet (or not polled).
    pub settled: Option<SettleOutcome>,
}

/// The create + poll-on-credit flow (T1.3). Signs the create with the
/// local account key (the payee half), prints nothing itself — returns the
/// pieces the CLI renders (the link-first copy rule stays with the caller,
/// testable without a transport).
///
/// `service_base` is the configured service URL the transport was built
/// against (the link composes from it — the trait deliberately carries no
/// base). `amount_micro`: `Some(>0)` pins the amount; `None` = any amount
/// (ask 3 — the payer composes at settle). `wait_secs` bounds the credit
/// poll — a MONOTONE budget (`Instant`), not a wall-clock deadline: a
/// system clock step cannot extend or truncate the wait, and `0` skips the
/// poll entirely (the credit surfaces via `account`). `poll_interval`
/// keeps the test honest (the CLI passes 5 s).
///
/// # Errors
///
/// A human-readable string on a non-200 create, an unreadable create
/// response, or a transport error mid-poll.
#[cfg(feature = "kat_transport")]
pub fn pay_request_flow(
    transport: &dyn PayreqTransport,
    service_base: &str,
    key: &ed25519_dalek::SigningKey,
    amount_micro: Option<i64>,
    label: &str,
    wait_secs: u64,
    poll_interval: std::time::Duration,
) -> Result<PayRequestOutcome, String> {
    use ed25519_dalek::Signer as _;

    let amount = amount_micro.unwrap_or(0);
    let pubkey = key.verifying_key().to_bytes();
    let signature = key.sign(&create_message(&pubkey, amount, label.as_bytes()));
    let req = serde_json::json!({
        "pubkey_hex": hex_encode(&pubkey),
        "signature_hex": hex_encode(&signature.to_bytes()),
        "amount_micro": amount,
        "label": label,
    });
    let body = match transport.post_create(&req.to_string()) {
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(c) => {
            return Err(format!("pay: service refused the create (HTTP {c})"))
        }
        HttpResponse::Error(e) => return Err(format!("pay: service unreachable: {e}")),
        HttpResponse::NotModified => {
            return Err("pay: service answered 304 for a create".to_string())
        }
    };
    let parsed: serde_json::Value = serde_json::from_slice(&body).map_err(|_| {
        "pay: service answered, but the create response was not understood".to_string()
    })?;
    if let Some(err) = parsed.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("pay: refused — {err}"));
    }
    let id_hex = parsed
        .get("request_id")
        .and_then(serde_json::Value::as_str)
        .ok_or("pay: create response carried no request_id")?
        .to_string();
    let url = request_url_from_id_hex(service_base, &id_hex);

    // Poll on credit: the view flips to "settled" when the payer's signed
    // settle lands; the invoice row is then readable. The budget is a
    // monotone elapsed-time check (a fixed wall-clock snapshot would never
    // advance — the mock test caught exactly that spin).
    let mut settled = None;
    if wait_secs > 0 {
        let started = std::time::Instant::now();
        let budget = std::time::Duration::from_secs(wait_secs);
        loop {
            match transport.get_request(&id_hex) {
                HttpResponse::Ok(b) => {
                    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
                        if v.get("status").and_then(serde_json::Value::as_str) == Some("settled") {
                            settled = Some(fetch_invoice(transport, &id_hex));
                            break;
                        }
                        if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
                            return Err(format!("pay: request unreadable — {err}"));
                        }
                    }
                }
                HttpResponse::Error(e) => return Err(format!("pay: poll failed: {e}")),
                _ => {}
            }
            if started.elapsed() >= budget {
                break;
            }
            std::thread::sleep(poll_interval);
        }
    }
    Ok(PayRequestOutcome {
        id_hex,
        url,
        amount_micro: amount,
        settled,
    })
}

// The `now_unix`-anchor in the poll: monotone-enough for a CLI wait
// budget (a system clock step backwards ends the poll early — an honest
// exit, the credit surfaces via `account`).

/// Read the invoice row (the poll-on-credit payoff). An unreadable row is
/// reported as such — never rendered as a zero-amount invoice, which is
/// what a `.unwrap_or(0)` on `amount_paid` used to do.
#[cfg(feature = "kat_transport")]
pub fn fetch_invoice(transport: &dyn PayreqTransport, id_hex: &str) -> SettleOutcome {
    match transport.get_invoice(id_hex) {
        HttpResponse::Ok(b) => match parse_invoice(&b) {
            Ok(inv) => SettleOutcome::Invoice(inv),
            Err(e) => SettleOutcome::Unreadable(e),
        },
        HttpResponse::Status(c) => {
            SettleOutcome::Unreadable(format!("the invoice read answered HTTP {c}"))
        }
        HttpResponse::Error(e) => SettleOutcome::Unreadable(format!("invoice read failed: {e}")),
        HttpResponse::NotModified => {
            SettleOutcome::Unreadable("the invoice read answered 304".to_string())
        }
    }
}

// ── T1.8: the faucet preset flow (`cargo heal faucet --request`) ────────

/// What one `faucet --request` produced (the caller's receipt).
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaucetOutcome {
    /// The opened row, exactly as the ledger reported it.
    pub request: FaucetRequest,
    /// The capability URL for that row — link-first, ask 13.
    pub url: String,
}

/// Open one faucet request: sign [`faucet_message`], `POST
/// /payreq/faucet`, and report the row the ledger pinned.
///
/// **One POST, no poll.** The pay-request flow polls because a payer is
/// expected to settle within a CLI wait budget; a faucet row is settled by
/// the DAILY crank, so a poll here would either be a lie (a few seconds
/// against a once-a-day pass) or a day-long hang. The honest surface is:
/// open the row, print what the ledger pinned, and say where the credit
/// shows up.
///
/// Nothing about the amount is proposed — [`FaucetRequest::amount_micro`]
/// is read back from the deployment's `faucet_daily_cap_kat`. A zero
/// amount is therefore NOT the pay-request "any amount" convention; it
/// means the row read back without one, which is an error rather than a
/// free pass.
///
/// # Errors
///
/// A human-readable string on an unreachable service, a non-200, an
/// unparseable body, a refusal (surfaced verbatim), or a zero pinned
/// amount.
#[cfg(feature = "kat_transport")]
pub fn faucet_request_flow(
    transport: &dyn PayreqTransport,
    service_base: &str,
    key: &ed25519_dalek::SigningKey,
) -> Result<FaucetOutcome, String> {
    let body = match transport.post_faucet(&faucet_request_body(key)) {
        HttpResponse::Ok(b) => b,
        // A refusal rides a non-200 WITH a body, and the body is the
        // ledger's own words (faucet disabled / service account / cap
        // used / one already open) — reading only the status code would
        // turn every one of those into an opaque "HTTP 400".
        HttpResponse::Status(c) => {
            return Err(format!(
                "faucet: the service refused the request (HTTP {c})"
            ))
        }
        HttpResponse::Error(e) => return Err(format!("faucet: service unreachable: {e}")),
        HttpResponse::NotModified => {
            return Err("faucet: the service answered 304 for a faucet request".to_string())
        }
    };
    let request = parse_faucet_request(&body).map_err(|e| format!("faucet: {e}"))?;
    match request.amount_micro {
        0 => Err(
            "faucet: the ledger pinned an amount of 0 — this deployment's \
             `faucet_daily_cap_kat` is unset or the row did not read back; there is nothing \
             to credit"
                .to_string(),
        ),
        _ => Ok(FaucetOutcome {
            url: request_url_from_id_hex(service_base, &request.id_hex),
            request,
        }),
    }
}

/// The `faucet --request` render — caveat-#15 copy (no `$`, KAT labelled
/// service credit) and the pinned amount presented as the LEDGER's, never
/// as the requester's choice.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn faucet_display_lines(outcome: &FaucetOutcome) -> Vec<String> {
    vec![
        format!("faucet request {}:", &outcome.request.id_hex),
        format!("  share this link: {}", outcome.url),
        format!(
            "  amount:    {} service credit — PINNED by the deployment's daily faucet cap \
             (not a value this request chose)",
            render_kat(outcome.request.amount_micro)
        ),
        format!(
            "  expires:   ledger tick {} (the deployment's link_ttl)",
            outcome.request.expiry
        ),
        "  the credit lands at the next daily crank pass; `cargo heal account` shows it"
            .to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE cross-repo golden (riir-dapps `tests/kat_payreq.rs`
    /// `create_message_golden_digest` asserts the SAME digest — computed
    /// independently of either implementation): the create message the
    /// signed route verifies.
    #[test]
    fn create_message_golden_digest() {
        let mut h = blake3::Hasher::new();
        h.update(&create_message(&[9u8; 32], 1000, b"inv-1"));
        assert_eq!(
            h.finalize().to_hex().to_string(),
            "59901ebfd866ebe44855e5da732902d5c88a2a8d662d079c40f7ce67a5bac1ff"
        );
    }

    /// THE cross-repo faucet golden (riir-dapps `tests/kat_payreq.rs`
    /// `faucet_message_golden_digest` asserts the SAME two digests): the
    /// T1.8 faucet-request message the signed route verifies.
    ///
    /// Both digests were computed **independently of either
    /// implementation** — `b3sum` over the documented byte layout
    /// (`kat:payreq:faucet:v1:` ‖ 32 pubkey bytes), and the method was
    /// validated first by reproducing the already-pinned
    /// `create_message` golden above bit-for-bit. Copying the service's
    /// number would have pinned the copy, not the contract.
    #[test]
    fn faucet_message_golden_digest() {
        let digest = |pk: &[u8; 32]| {
            let mut h = blake3::Hasher::new();
            h.update(&faucet_message(pk));
            h.finalize().to_hex().to_string()
        };
        // 0x5A — the fixture riir-dapps' own faucet test uses.
        assert_eq!(
            digest(&[0x5Au8; 32]),
            "acaeeffeb97116e50054b545c0f1ce77ca45ddcfe39b0a2d26876ffc650af3dd"
        );
        // 0x09 — the SAME key the create golden pins, so cross-domain
        // separation is provable at a shared key rather than inferred
        // from two unrelated fixtures.
        assert_eq!(
            digest(&[9u8; 32]),
            "d3907ac379aa29f437a5439e29efa724934093f8d62ae43d71e4006fa7b8ee02"
        );
    }
    /// THE cross-repo CLAIM golden (T079) — riir-dapps
    /// `tests/kat_payreq.rs::claim_message_golden_digest` pins the SAME
    /// two digests, computed independently of BOTH implementations, so
    /// the client mirror cannot drift from the verifying side.
    #[test]
    fn claim_message_golden_digest() {
        let digest = |pk: &[u8; 32]| {
            let mut h = blake3::Hasher::new();
            h.update(&claim_message(pk));
            h.finalize().to_hex().to_string()
        };
        assert_eq!(
            digest(&[0x5Au8; 32]),
            "d9256f89b00a68b64516c0aa1a1dd9c1955507ab7f3ffcff21a9524a5c4734f8"
        );
        assert_eq!(
            digest(&[9u8; 32]),
            "00f2801ddad51fb906aad37a40936c872191c77591b8c099c9b6b844d6043543"
        );
    }

    /// The claim body carries the pubkey and the signature and NOTHING
    /// else — nothing on the wire for a requester to choose or an
    /// attacker to inflate.
    #[test]
    fn the_claim_body_carries_no_amount() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[3u8; 32]);
        let body = claim_request_body(&key);
        let v: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(
            v.as_object().expect("object").len(),
            2,
            "pubkey + signature only"
        );
        assert!(v.get("pubkey_hex").is_some());
        assert!(v.get("signature_hex").is_some());
        assert!(v.get("amount").is_none());
        assert!(v.get("amount_micro").is_none());
    }

    /// The ensure-grant flow: read-first (granted ⇒ NO post), claim on
    /// missing grant, idempotent replay handled.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn ensure_grant_claims_once_and_is_a_noop_when_granted() {
        use std::cell::RefCell;

        struct ClaimMock {
            granted: RefCell<bool>,
            claims: RefCell<u32>,
        }
        impl PayreqTransport for ClaimMock {
            fn post_create(&self, _json: &str) -> HttpResponse {
                unimplemented!()
            }
            fn get_request(&self, _id: &str) -> HttpResponse {
                unimplemented!()
            }
            fn get_invoice(&self, _id: &str) -> HttpResponse {
                unimplemented!()
            }
            fn post_faucet(&self, _json: &str) -> HttpResponse {
                unimplemented!()
            }
            fn get_balance(&self, acct_hex: &str) -> HttpResponse {
                let granted = *self.granted.borrow();
                HttpResponse::Ok(
                    serde_json::json!({
                        "account_id": acct_hex,
                        "balance_micro": if granted { 100_000_000 } else { 0 },
                        "granted": granted,
                    })
                    .to_string()
                    .into_bytes(),
                )
            }
            fn post_claim(&self, _json: &str) -> HttpResponse {
                *self.claims.borrow_mut() += 1;
                self.granted.replace(true);
                HttpResponse::Ok(
                    serde_json::json!({
                        "account_id": "ab".repeat(16),
                        "granted": true,
                        "already": false,
                        "balance_micro": 100_000_000,
                        "claimed_micro": 100_000_000,
                    })
                    .to_string()
                    .into_bytes(),
                )
            }
        }
        let key = ed25519_dalek::SigningKey::from_bytes(&[4u8; 32]);

        // Not granted: exactly one claim POST, granted flips.
        let mock = ClaimMock {
            granted: RefCell::new(false),
            claims: RefCell::new(0),
        };
        let out = ensure_grant_flow(&mock, &key, &"ab".repeat(16)).expect("claimed");
        let claim = out.expect("a claim happened");
        assert!(!claim.already);
        assert_eq!(claim.claimed_micro, 100_000_000);
        assert_eq!(*mock.claims.borrow(), 1);

        // Granted: read-first, ZERO posts.
        let out = ensure_grant_flow(&mock, &key, &"ab".repeat(16)).expect("read ok");
        assert!(out.is_none(), "no second claim");
        assert_eq!(
            *mock.claims.borrow(),
            1,
            "read-first: the replay never hit the wire"
        );
    }

    /// The layout the golden is a digest OF — asserted separately so a
    /// re-pin cannot quietly change the shape, and so the four payreq
    /// domains are pinned mutually distinct at one key.
    #[test]
    fn faucet_message_is_domain_separated_and_carries_no_amount() {
        let pk = [0x5Au8; 32];
        let msg = faucet_message(&pk);
        assert!(msg.starts_with(b"kat:payreq:faucet:v1:"));
        assert_eq!(msg.len(), 21 + 32, "domain + pubkey, nothing else");
        assert_eq!(&msg[21..], &pk, "the requester's key is the whole payload");
        assert_ne!(
            faucet_message(&[0x5Bu8; 32]),
            msg,
            "a different requester is a different message"
        );
        // A faucet signature must not be replayable as a create (and the
        // reverse), including at the amount-0 "any amount" create.
        assert_ne!(create_message(&pk, 0, FAUCET_LABEL.as_bytes()), msg);
        assert!(
            !create_message(&pk, 0, FAUCET_LABEL.as_bytes()).starts_with(b"kat:payreq:faucet:v1:")
        );
    }

    /// The wire body carries the pubkey + signature and NOTHING else.
    ///
    /// Pinned as a KEY-SET assertion rather than a spot check: an
    /// `amount_micro` slot appearing here later would let a requester
    /// propose their own faucet amount, which is exactly the replay the
    /// amount-free message exists to prevent — and a per-field check
    /// would not have noticed a *new* field at all.
    #[test]
    fn the_faucet_body_carries_no_amount_and_no_ttl() {
        use ed25519_dalek::Verifier as _;

        let key = ed25519_dalek::SigningKey::from_bytes(&[0x11u8; 32]);
        let body = faucet_request_body(&key);
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        let mut keys: Vec<&str> = v
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["pubkey_hex", "signature_hex"],
            "the faucet wire is pubkey + signature ONLY — no amount, no ttl, no account"
        );
        assert_eq!(v["signature_hex"].as_str().map(str::len), Some(128));
        assert_eq!(v["pubkey_hex"].as_str().map(str::len), Some(64));
        // And the signature verifies over the mirrored message.
        let sig_bytes = {
            let hex = v["signature_hex"].as_str().unwrap();
            let mut out = [0u8; 64];
            for (i, b) in out.iter_mut().enumerate() {
                *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex");
            }
            out
        };
        let pubkey = key.verifying_key();
        pubkey
            .verify(
                &faucet_message(&pubkey.to_bytes()),
                &ed25519_dalek::Signature::from_bytes(&sig_bytes),
            )
            .expect("the body's signature verifies over faucet_message");
    }

    /// The faucet 200 parse + the four honest refusals surfacing verbatim.
    #[test]
    fn parse_faucet_request_reads_the_pinned_amount_and_surfaces_refusals() {
        let ok = serde_json::json!({
            "request_id": "ab".repeat(16),
            "created": true,
            "kind": "faucet",
            "amount_micro": 5_000_000,
            "expiry": 4242,
        })
        .to_string();
        assert_eq!(
            parse_faucet_request(ok.as_bytes()),
            Ok(FaucetRequest {
                id_hex: "ab".repeat(16),
                amount_micro: 5_000_000,
                expiry: 4242,
            })
        );
        for reason in [
            "payreq: the faucet is disabled at this deployment (faucet_daily_cap_kat = 0)",
            "payreq: service accounts cannot request from the faucet",
            "payreq: faucet daily cap already used for this account today",
            "payreq: an open faucet request already exists for this account",
        ] {
            let body = serde_json::json!({"error": reason}).to_string();
            let err = parse_faucet_request(body.as_bytes()).expect_err("a refusal is an error");
            assert!(
                err.contains(reason),
                "the ledger's own refusal must survive verbatim, got {err:?}"
            );
        }
        // A missing amount is an error, never a zero-amount faucet.
        let no_amt = serde_json::json!({"request_id": "ab".repeat(16), "expiry": 1}).to_string();
        assert!(parse_faucet_request(no_amt.as_bytes()).is_err());
    }

    /// The account derivation mirrors the sync plane (`submitter_for`):
    /// `blake3(pk)[..16]`, hex on the wire.
    #[test]
    fn account_derivation_is_the_house_truncation() {
        let acct = account_for_pubkey(&[7u8; 32]);
        let expect_hex: String = {
            let d = blake3::hash(&[7u8; 32]);
            d.as_bytes()[..16].iter().fold(String::new(), |mut acc, b| {
                use std::fmt::Write as _;
                let _ = write!(acc, "{b:02x}");
                acc
            })
        };
        assert_eq!(hex_encode(&acct), expect_hex);
    }

    /// Link compose: trailing-slash trim, exact path, round-trip readable
    /// (the id segment is the last 32 chars either way).
    #[test]
    fn request_url_composes_link_first() {
        let id = [0xABu8; 16];
        assert_eq!(
            request_url("https://kat.example.com", &id),
            format!("https://kat.example.com/payreq/{}", hex_encode(&id))
        );
        // A trailing slash on the configured base must not double.
        assert_eq!(
            request_url("https://kat.example.com/", &id),
            request_url("https://kat.example.com", &id)
        );
    }

    /// KAT amount parsing: whole, fractional, junk, precision, and the
    /// positive-only bound (0 is not an amount — blank means any).
    #[test]
    fn parse_kat_amount_matrix() {
        assert_eq!(parse_kat_amount("2"), Some(2_000_000));
        assert_eq!(parse_kat_amount("1.5"), Some(1_500_000));
        assert_eq!(parse_kat_amount("0.000001"), Some(1));
        assert_eq!(parse_kat_amount(" 12.25 "), Some(12_250_000));
        assert_eq!(parse_kat_amount("0"), None, "zero is not an amount");
        assert_eq!(parse_kat_amount("-1"), None);
        assert_eq!(parse_kat_amount("1.0000005"), None, "sub-micro precision");
        assert_eq!(parse_kat_amount("abc"), None);
        assert_eq!(parse_kat_amount(""), None);
        assert_eq!(parse_kat_amount("1..2"), None);
        assert_eq!(parse_kat_amount("1."), Some(1_000_000));
        assert_eq!(parse_kat_amount(".5"), Some(500_000));
    }

    /// The render is the parse's mirror (round-trip at the micro edge).
    #[test]
    fn render_kat_round_trips_the_parse() {
        for s in ["0.000001", "1.5", "1234.56789"] {
            let micro = parse_kat_amount(s).expect("parses");
            let rendered = render_kat(micro);
            let back = parse_kat_amount(rendered.trim_end_matches(" KAT").trim());
            assert_eq!(back, Some(micro));
        }
        assert_eq!(render_kat(-5), "-0.000005 KAT");
    }

    // ── T1.4: the invoice view + the bit-exact recheck ──────────────────

    /// A settled invoice body with a caller-chosen QR commitment slot.
    fn invoice_json(qr_hex: &str) -> Vec<u8> {
        serde_json::json!({
            "request_id": "71b5cbf13626d2554aba7d9b06e63474",
            "payee": "ab".repeat(16),
            "payer": "cd".repeat(16),
            "token": "kat",
            "amount_paid": 1_500_000,
            "settled": 42,
            "qr_png_blake3": qr_hex,
        })
        .to_string()
        .into_bytes()
    }

    /// The parse is total over the served shape, and a partial row is an
    /// error rather than a zero-filled invoice (the old
    /// `amount_paid.unwrap_or(0)` would have rendered "0.000000 KAT paid"
    /// for a malformed row — a wrong figure is worse than no figure).
    #[test]
    fn parse_invoice_is_total_and_refuses_partial_rows() {
        let inv = parse_invoice(&invoice_json(&"0".repeat(64))).expect("parses");
        assert_eq!(inv.id_hex, "71b5cbf13626d2554aba7d9b06e63474");
        assert_eq!(inv.amount_paid_micro, 1_500_000);
        assert_eq!(inv.settled_tick, 42);
        assert_eq!(inv.token, "kat");
        assert!(
            inv.qr_commitment_is_unset(),
            "all-zero is the unset sentinel"
        );

        // Absent slot is equally unset.
        let no_slot = serde_json::json!({
            "request_id": "ab".repeat(16), "payee": "ab".repeat(16),
            "payer": "cd".repeat(16), "token": "kat",
            "amount_paid": 1, "settled": 2,
        })
        .to_string();
        assert!(parse_invoice(no_slot.as_bytes())
            .unwrap()
            .qr_commitment_is_unset());

        // Missing money field → error, not a zero.
        let partial = serde_json::json!({
            "request_id": "ab".repeat(16), "payee": "ab".repeat(16),
            "payer": "cd".repeat(16), "token": "kat", "settled": 2,
        })
        .to_string();
        let err = parse_invoice(partial.as_bytes()).expect_err("refused");
        assert!(err.contains("amount_paid"), "{err}");

        // A service error body surfaces as an error.
        let refused = serde_json::json!({"error": "not settled yet"}).to_string();
        assert!(parse_invoice(refused.as_bytes()).is_err());
        assert!(parse_invoice(b"not json").is_err());
    }

    /// The recheck's three states, each reached for its own reason.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn recheck_reaches_all_three_states_for_the_right_reasons() {
        let url = request_url_from_id_hex(
            crate::kat_lease_client::DEFAULT_SERVICE_URL,
            "71b5cbf13626d2554aba7d9b06e63474",
        );
        let good =
            hex_encode(&crate::kat_lease_client::qr_commitment(&url).expect("the link encodes"));

        // (a) Unset sentinel → Unavailable. This is the state EVERY invoice
        //     the service has written is in today (dapps leaves
        //     `qr_png_blake3` zeroed until its own T1.4 render step), so
        //     reading it as a tamper would refuse every real invoice.
        let unset = parse_invoice(&invoice_json(&"0".repeat(64))).unwrap();
        assert!(matches!(
            recheck_invoice_qr(&unset, &url),
            QrRecheck::Unavailable { .. }
        ));

        // (b) Agreeing digest → Match.
        let ok = parse_invoice(&invoice_json(&good)).unwrap();
        assert_eq!(
            recheck_invoice_qr(&ok, &url),
            QrRecheck::Match {
                digest_hex: good.clone()
            }
        );

        // (c) Any other value → Mismatch. Including a well-formed digest of
        //     the WRONG link — the asset is bound to its request.
        let other_url = request_url_from_id_hex(
            crate::kat_lease_client::DEFAULT_SERVICE_URL,
            &"ab".repeat(16),
        );
        let wrong = hex_encode(&crate::kat_lease_client::qr_commitment(&other_url).unwrap());
        let tampered = parse_invoice(&invoice_json(&wrong)).unwrap();
        assert!(matches!(
            recheck_invoice_qr(&tampered, &url),
            QrRecheck::Mismatch { .. }
        ));
        // And a garbage-but-nonzero slot is a refusal, never a pass.
        let junk = parse_invoice(&invoice_json("deadbeef")).unwrap();
        assert!(matches!(
            recheck_invoice_qr(&junk, &url),
            QrRecheck::Mismatch { .. }
        ));
    }

    /// A mismatch REFUSES to render the invoice body (Proposal 011 §1).
    /// Printing the amount beside a tamper warning is the screenshot that
    /// rule exists to prevent, so assert the figures are ABSENT.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn a_mismatch_refuses_to_display_the_invoice_body() {
        let inv = parse_invoice(&invoice_json(&"ab".repeat(32))).unwrap();
        let lines = invoice_display_lines(
            &inv,
            &QrRecheck::Mismatch {
                stored_hex: "ab".repeat(32),
                recomputed_hex: "cd".repeat(32),
            },
        );
        let joined = lines.join("\n");
        assert!(joined.contains("REFUSED"), "{joined}");
        assert!(joined.contains("tamper indicator"), "{joined}");
        assert!(
            !joined.contains("1.500000"),
            "a refused invoice must not print its amount: {joined}"
        );
        // Assert the BODY LABELS are absent rather than the field values:
        // a value-substring check false-fired on the fixture, because a
        // 32-byte digest rendered as repeated "cd" contains the payer's
        // repeated "cd". Labels are the stable discriminator.
        for label in ["payee:", "payer:", "token:", "amount:", "settled:"] {
            assert!(
                !joined.contains(label),
                "a refused invoice must not print the `{label}` row: {joined}"
            );
        }

        // The two rendering states DO carry the body, and label the QR leg
        // differently — an unverified display must never read as verified.
        let matched = invoice_display_lines(
            &inv,
            &QrRecheck::Match {
                digest_hex: "ef".repeat(32),
            },
        )
        .join("\n");
        assert!(matched.contains("1.500000 KAT"), "{matched}");
        assert!(matched.contains("bit-exact"), "{matched}");
        let unavailable = invoice_display_lines(
            &inv,
            &QrRecheck::Unavailable {
                reason: "no digest".to_string(),
            },
        )
        .join("\n");
        assert!(unavailable.contains("1.500000 KAT"), "{unavailable}");
        assert!(
            unavailable.contains("NOT bit-exact-verified"),
            "the fallback must SAY it is a fallback: {unavailable}"
        );
    }

    /// Caveat-#15 copy rule over EVERY new display surface and EVERY
    /// recheck state: no `$` anywhere, and the KAT figure is labelled
    /// service credit.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn invoice_display_obeys_the_copy_rule_in_every_recheck_state() {
        let inv = parse_invoice(&invoice_json(&"0".repeat(64))).unwrap();
        let states = [
            QrRecheck::Match {
                digest_hex: "ab".repeat(32),
            },
            QrRecheck::Unavailable {
                reason: "unset".to_string(),
            },
            QrRecheck::Mismatch {
                stored_hex: "ab".repeat(32),
                recomputed_hex: "cd".repeat(32),
            },
        ];
        for state in &states {
            for line in invoice_display_lines(&inv, state) {
                assert!(!line.contains('$'), "no dollars beside KAT: {line}");
            }
        }
        // The rendering states must LABEL the credit (caveat #15's other
        // half: an accrual figure without the service-credit label is the
        // exposure, not just the `$`).
        for state in &states[..2] {
            let joined = invoice_display_lines(&inv, state).join("\n");
            assert!(joined.contains("service credit"), "{joined}");
        }
        assert!(!invoice_line(&inv).contains('$'));
        assert!(invoice_line(&inv).contains("1.500000 KAT"));
    }

    /// The full flow against a scripted mock: create → parse the id →
    /// poll the view (one "created" tick, one "settled") → the invoice
    /// line rides the outcome. The mock drives the SAME trait the ureq
    /// transport implements.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn pay_request_flow_creates_polls_and_sees_the_credit() {
        use std::cell::RefCell;
        use std::time::Duration;

        struct ScriptedMock {
            polls: RefCell<u32>,
        }
        impl PayreqTransport for ScriptedMock {
            fn post_create(&self, json: &str) -> HttpResponse {
                // The signed body must carry the four wire fields.
                let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
                assert!(v["pubkey_hex"].as_str().is_some());
                assert_eq!(v["signature_hex"].as_str().map(str::len), Some(128));
                assert_eq!(v["amount_micro"].as_i64(), Some(1_500_000));
                assert_eq!(v["label"].as_str(), Some("invoice #1042"));
                HttpResponse::Ok(
                    serde_json::json!({
                        "request_id": "71b5cbf13626d2554aba7d9b06e63474",
                        "created": true,
                        "amount_micro": 1_500_000
                    })
                    .to_string()
                    .into_bytes(),
                )
            }
            fn get_request(&self, id_hex: &str) -> HttpResponse {
                assert_eq!(id_hex, "71b5cbf13626d2554aba7d9b06e63474");
                let mut n = self.polls.borrow_mut();
                *n += 1;
                let status = if *n == 1 { "created" } else { "settled" };
                HttpResponse::Ok(
                    serde_json::json!({
                        "request_id": id_hex,
                        "status": status,
                        "payee": "ab".repeat(16),
                        "amount_micro": 1_500_000,
                        "any_amount": false,
                        "amount_paid": 0
                    })
                    .to_string()
                    .into_bytes(),
                )
            }
            fn get_invoice(&self, id_hex: &str) -> HttpResponse {
                HttpResponse::Ok(
                    serde_json::json!({
                        "request_id": id_hex,
                        "payee": "ab".repeat(16),
                        "payer": "cd".repeat(16),
                        "token": "kat",
                        "amount_paid": 1_500_000,
                        "settled": 42,
                        "qr_png_blake3": "0".repeat(64),
                    })
                    .to_string()
                    .into_bytes(),
                )
            }
            fn post_faucet(&self, _json: &str) -> HttpResponse {
                // This mock scripts the PAY-request flow only. The faucet
                // channel (T1.8) has its own mocks in `kat_wetrun`.
                unimplemented!()
            }
            fn get_balance(&self, _acct_hex: &str) -> HttpResponse {
                unimplemented!()
            }
            fn post_claim(&self, _json: &str) -> HttpResponse {
                unimplemented!()
            }
        }

        let key = ed25519_dalek::SigningKey::from_bytes(&[0xC5; 32]);
        let mock = ScriptedMock {
            polls: RefCell::new(0),
        };
        let outcome = pay_request_flow(
            &mock,
            "https://kat.example.com",
            &key,
            Some(1_500_000),
            "invoice #1042",
            10,
            Duration::from_millis(1),
        )
        .expect("flow completes");
        assert_eq!(outcome.id_hex, "71b5cbf13626d2554aba7d9b06e63474");
        assert_eq!(outcome.amount_micro, 1_500_000);
        let Some(SettleOutcome::Invoice(inv)) = outcome.settled.as_ref() else {
            panic!(
                "the poll must have read the invoice row: {:?}",
                outcome.settled
            );
        };
        // The exact compact line stays pinned (T1.3's golden copy).
        assert_eq!(
            invoice_line(inv),
            "paid: 1.500000 KAT from cdcdcdcd… — invoice 71b5cbf13626d2554aba7d9b06e63474"
        );
        assert_eq!(inv.amount_paid_micro, 1_500_000);
        assert!(
            inv.qr_commitment_is_unset(),
            "the mock serves the unset sentinel"
        );
        assert!(outcome
            .url
            .ends_with("/payreq/71b5cbf13626d2554aba7d9b06e63474"));
    }

    /// A faucet-only mock: every pay-request method is `unimplemented!()`,
    /// so a flow that reached one would panic rather than pass quietly.
    /// The count is a `RefCell` because "exactly one POST, no poll" is the
    /// property under test — a faucet row is settled by the DAILY crank,
    /// so a poll here would be either a lie or a day-long hang.
    #[cfg(feature = "kat_transport")]
    struct FaucetMock {
        posts: std::cell::RefCell<u32>,
        reply: HttpResponse,
    }

    #[cfg(feature = "kat_transport")]
    impl PayreqTransport for FaucetMock {
        fn post_create(&self, _json: &str) -> HttpResponse {
            unimplemented!("the faucet flow must never open a PAY request")
        }
        fn get_request(&self, _id: &str) -> HttpResponse {
            unimplemented!("the faucet flow must never poll — the crank settles it")
        }
        fn get_invoice(&self, _id: &str) -> HttpResponse {
            unimplemented!("the faucet flow must never read an invoice row")
        }
        fn post_faucet(&self, json: &str) -> HttpResponse {
            *self.posts.borrow_mut() += 1;
            // The body the service actually verifies over: pubkey +
            // signature, and NOTHING the requester could inflate.
            let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
            assert_eq!(v.as_object().map(serde_json::Map::len), Some(2));
            assert_eq!(v["pubkey_hex"].as_str().map(str::len), Some(64));
            assert_eq!(v["signature_hex"].as_str().map(str::len), Some(128));
            self.reply.clone()
        }
        fn get_balance(&self, _acct_hex: &str) -> HttpResponse {
            unimplemented!("the faucet flow must never read a balance")
        }
        fn post_claim(&self, _json: &str) -> HttpResponse {
            unimplemented!("the faucet flow must never claim the grant")
        }
    }

    #[cfg(feature = "kat_transport")]
    fn faucet_mock(reply: HttpResponse) -> FaucetMock {
        FaucetMock {
            posts: std::cell::RefCell::new(0),
            reply,
        }
    }

    /// The happy path: ONE signed POST, the ledger's pinned amount read
    /// back (never proposed), the link composed from the returned id, and
    /// no poll at all.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn faucet_request_flow_posts_once_and_never_polls() {
        let mock = faucet_mock(HttpResponse::Ok(
            serde_json::json!({
                "request_id": "71b5cbf13626d2554aba7d9b06e63474",
                "created": true,
                "kind": "faucet",
                "amount_micro": 10_000_000,
                "expiry": 86_400,
            })
            .to_string()
            .into_bytes(),
        ));
        let key = ed25519_dalek::SigningKey::from_bytes(&[0xF0; 32]);
        let out = faucet_request_flow(&mock, "https://kat.example.com/", &key)
            .expect("the flow completes");
        assert_eq!(*mock.posts.borrow(), 1, "exactly one POST — no retry loop");
        assert_eq!(out.request.amount_micro, 10_000_000);
        assert_eq!(out.request.expiry, 86_400);
        assert_eq!(
            out.url,
            "https://kat.example.com/payreq/71b5cbf13626d2554aba7d9b06e63474"
        );

        // The render: caveat-#15 copy, and the amount is presented as the
        // DEPLOYMENT's, not as a value this request chose.
        let joined = faucet_display_lines(&out).join("\n");
        assert!(!joined.contains('$'), "{joined}");
        assert!(joined.contains("10.000000 KAT"), "{joined}");
        assert!(joined.contains("service credit"), "{joined}");
        assert!(joined.contains("PINNED"), "{joined}");
        assert!(joined.contains(&out.url), "link-first: {joined}");
    }

    /// A pinned amount of zero is an ERROR, not an "any amount" faucet.
    ///
    /// The pay-request convention reads `amount_micro == 0` as *"the payer
    /// chooses"*; carrying that convention into the faucet would turn a
    /// deployment with the faucet switched off (`faucet_daily_cap_kat = 0`)
    /// into an unbounded open request.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn a_zero_pinned_faucet_amount_is_refused_not_read_as_any_amount() {
        let mock = faucet_mock(HttpResponse::Ok(
            serde_json::json!({
                "request_id": "ab".repeat(16),
                "amount_micro": 0,
                "expiry": 1,
            })
            .to_string()
            .into_bytes(),
        ));
        let key = ed25519_dalek::SigningKey::from_bytes(&[0xF1; 32]);
        let err = faucet_request_flow(&mock, "https://kat.example.com", &key)
            .expect_err("a zero cap has nothing to credit");
        assert!(err.contains("faucet_daily_cap_kat"), "{err}");
        assert!(err.contains("nothing"), "{err}");
    }

    /// The hole this closes: a payreq refusal is a **non-200 WITH a body**
    /// (`RouteResp::error` ⇒ `{"error": "<the operator's words>"}`), and
    /// dropping that body reduced all four documented faucet refusals to a
    /// bare `HTTP 400`. Pinned on the pure half so the branch is reachable
    /// without a socket.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn a_non_200_carrying_the_services_error_body_is_not_reduced_to_a_status() {
        let refusal = serde_json::json!({
            "error": "payreq: an open faucet request already exists for this account"
        })
        .to_string()
        .into_bytes();
        assert_eq!(
            refusal_body_or_status(400, refusal.clone()),
            HttpResponse::Ok(refusal),
            "the service ANSWERED — its answer is the payload"
        );
        // An opaque failure must NOT be dressed up as an answer.
        for opaque in [b"<html>502</html>".to_vec(), b"{}".to_vec(), Vec::new()] {
            assert_eq!(
                refusal_body_or_status(502, opaque),
                HttpResponse::Status(502)
            );
        }
    }

    /// End-to-end over the flow: the four documented refusals reach the
    /// caller VERBATIM once the transport stops discarding the body.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn the_four_faucet_refusals_reach_the_caller_verbatim() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[0xF3; 32]);
        for reason in [
            "payreq: the faucet is disabled at this deployment (faucet_daily_cap_kat = 0)",
            "payreq: service accounts cannot request from the faucet",
            "payreq: faucet daily cap already used for this account today",
            "payreq: an open faucet request already exists for this account",
        ] {
            // What `refusal_or_status` now hands the flow for a 400.
            let mock = faucet_mock(refusal_body_or_status(
                400,
                serde_json::json!({"error": reason})
                    .to_string()
                    .into_bytes(),
            ));
            let err = faucet_request_flow(&mock, "https://kat.example.com", &key)
                .expect_err("a refusal is an error");
            assert!(
                err.contains(reason),
                "the ledger's own words must survive; got {err:?}"
            );
        }
    }

    /// The transport-level failure modes each surface as their own
    /// sentence, and none of them is retried.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn faucet_transport_failures_surface_once_and_honestly() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[0xF2; 32]);
        for (reply, needle) in [
            (HttpResponse::Status(400), "refused"),
            (HttpResponse::Error("dns".to_string()), "unreachable"),
            (HttpResponse::NotModified, "304"),
            (HttpResponse::Ok(b"not json".to_vec()), "not JSON"),
        ] {
            let mock = faucet_mock(reply);
            let err = faucet_request_flow(&mock, "https://kat.example.com", &key)
                .expect_err("every one of these is an error");
            assert!(err.starts_with("faucet: "), "{err}");
            assert!(err.contains(needle), "{needle:?} not in {err:?}");
            assert_eq!(*mock.posts.borrow(), 1, "one attempt, never a retry storm");
        }
    }

    /// A service refusal surfaces as a human-readable error, not a panic.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn create_refusal_surfaces_honestly() {
        struct Refusing;
        impl PayreqTransport for Refusing {
            fn post_create(&self, _json: &str) -> HttpResponse {
                HttpResponse::Status(401)
            }
            fn get_request(&self, _id: &str) -> HttpResponse {
                unimplemented!()
            }
            fn get_invoice(&self, _id: &str) -> HttpResponse {
                unimplemented!()
            }
            fn post_faucet(&self, _json: &str) -> HttpResponse {
                // This mock scripts the PAY-request flow only. The faucet
                // channel (T1.8) has its own mocks in `kat_wetrun`.
                unimplemented!()
            }
            fn get_balance(&self, _acct_hex: &str) -> HttpResponse {
                unimplemented!()
            }
            fn post_claim(&self, _json: &str) -> HttpResponse {
                unimplemented!()
            }
        }
        let key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        let err = pay_request_flow(
            &Refusing,
            "https://kat.example.com",
            &key,
            Some(1),
            "",
            0,
            std::time::Duration::from_millis(1),
        )
        .expect_err("refused");
        assert!(err.contains("401"), "{err}");
    }

    /// `now_unix` rides in so the poll is testable without a clock; assert
    /// the wait-budget bound ends the poll (a never-settling view).
    #[cfg(feature = "kat_transport")]
    #[test]
    fn poll_ends_on_the_wait_budget() {
        use std::time::Duration;

        struct NeverSettles;
        impl PayreqTransport for NeverSettles {
            fn post_create(&self, _json: &str) -> HttpResponse {
                HttpResponse::Ok(
                    serde_json::json!({"request_id": "ab".repeat(16), "created": true})
                        .to_string()
                        .into_bytes(),
                )
            }
            fn get_request(&self, id_hex: &str) -> HttpResponse {
                HttpResponse::Ok(
                    serde_json::json!({"request_id": id_hex, "status": "created"})
                        .to_string()
                        .into_bytes(),
                )
            }
            fn get_invoice(&self, _id: &str) -> HttpResponse {
                unimplemented!()
            }
            fn post_faucet(&self, _json: &str) -> HttpResponse {
                // This mock scripts the PAY-request flow only. The faucet
                // channel (T1.8) has its own mocks in `kat_wetrun`.
                unimplemented!()
            }
            fn get_balance(&self, _acct_hex: &str) -> HttpResponse {
                unimplemented!()
            }
            fn post_claim(&self, _json: &str) -> HttpResponse {
                unimplemented!()
            }
        }
        let key = ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]);
        let outcome = pay_request_flow(
            &NeverSettles,
            "https://kat.example.com",
            &key,
            None,
            "",
            1,
            Duration::from_millis(1),
        )
        .expect("flow completes");
        assert!(
            outcome.settled.is_none(),
            "the budget ended before any settle"
        );
    }
}
