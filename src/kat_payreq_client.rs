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
    let whole: i64 = if whole.is_empty() { 0 } else { whole.parse().ok()? };
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
}

/// The ureq HTTP transport over the shared service base URL (the
/// `HttpVoteTransport` pattern: one bounded agent, JSON bodies).
#[cfg(feature = "kat_transport")]
pub struct HttpPayreqTransport {
    agent: ureq::Agent,
    base: String,
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

    fn post_json(&self, path: &str, body: &str) -> HttpResponse {
        let url = format!("{}{path}", self.base);
        match self
            .agent
            .post(&url)
            .set("Content-Type", "application/json")
            .send_string(body)
        {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => HttpResponse::Status(c),
            Err(e) => HttpResponse::Error(e.to_string()),
        }
    }

    fn get(&self, path: &str) -> HttpResponse {
        let url = format!("{}{path}", self.base);
        match self.agent.get(&url).call() {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => HttpResponse::Status(c),
            Err(e) => HttpResponse::Error(e.to_string()),
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
    /// The invoice render line when the poll saw the settle before the
    /// wait budget ended; `None` = not settled yet (or not polled).
    pub settled: Option<String>,
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
        HttpResponse::Status(c) => return Err(format!("pay: service refused the create (HTTP {c})")),
        HttpResponse::Error(e) => return Err(format!("pay: service unreachable: {e}")),
        HttpResponse::NotModified => return Err("pay: service answered 304 for a create".to_string()),
    };
    let parsed: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| "pay: service answered, but the create response was not understood".to_string())?;
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
                            settled = Some(invoice_line(transport, &id_hex));
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

/// The invoice render line (the poll-on-credit payoff; caveat-#15 copy —
/// KAT amounts, never a `$`, never a redemption promise).
#[cfg(feature = "kat_transport")]
fn invoice_line(transport: &dyn PayreqTransport, id_hex: &str) -> String {
    if let HttpResponse::Ok(b) = transport.get_invoice(id_hex) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
            let paid = v
                .get("amount_paid")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let payer = v
                .get("payer")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let payer_short: String = payer.chars().take(8).collect();
            return format!(
                "paid: {} from {payer_short}… — invoice {id_hex}",
                render_kat(paid)
            );
        }
    }
    format!("settled — invoice {id_hex} (details unreadable)")
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

    /// The account derivation mirrors the sync plane (`submitter_for`):
    /// `blake3(pk)[..16]`, hex on the wire.
    #[test]
    fn account_derivation_is_the_house_truncation() {
        let acct = account_for_pubkey(&[7u8; 32]);
        let expect_hex: String = {
            let d = blake3::hash(&[7u8; 32]);
            d.as_bytes()[..16].iter().map(|b| format!("{b:02x}")).collect()
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

    /// Caveat-#15 copy rule: no render path puts a `$` beside a KAT amount
    /// (service credit, not dollars — enforced by test like every client).
    #[cfg(feature = "kat_transport")]
    #[test]
    fn invoice_line_never_renders_a_dollar() {
        struct Mock;
        impl PayreqTransport for Mock {
            fn post_create(&self, _json: &str) -> HttpResponse {
                unimplemented!()
            }
            fn get_request(&self, _id: &str) -> HttpResponse {
                unimplemented!()
            }
            fn get_invoice(&self, _id: &str) -> HttpResponse {
                HttpResponse::Ok(
                    serde_json::json!({
                        "request_id": "ab".repeat(16),
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
        }
        let line = invoice_line(&Mock, &"ab".repeat(16));
        assert!(!line.contains('$'), "no dollars beside KAT: {line}");
        assert!(line.contains("1.500000 KAT"), "the amount renders: {line}");
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
        assert_eq!(
            outcome.settled.as_deref(),
            Some("paid: 1.500000 KAT from cdcdcdcd… — invoice 71b5cbf13626d2554aba7d9b06e63474")
        );
        assert!(outcome.url.ends_with("/payreq/71b5cbf13626d2554aba7d9b06e63474"));
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
        assert!(outcome.settled.is_none(), "the budget ended before any settle");
    }
}
