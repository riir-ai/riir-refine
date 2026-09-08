//! The burn auto-sync client (Issue 076 client half, owner mandate
//! 2026-09-08) — after every heal run that settles a charge, the cumulative
//! burn is reported to the service's ledger plane so the account's 100M
//! free grant is debited server-side. Account BILLING, not the mining
//! consent plane: it rides the login, and `--unmine` does not stop it.
//!
//! # Git-style stacking (the owner's explicit mandate)
//!
//! A push that fails is a LOCAL COMMIT. The ack watermark ([`KatBurnAck`]
//! at `<root>/.heal/kat_burn_ack.json`) advances ONLY on a `200` response
//! whose `burn_kat_total` is ≥ what we sent, so the un-pushed delta stays
//! on disk and the NEXT run (or `cargo heal sync`) reports the FULL
//! cumulative total in one signed report. The server keeps
//! `max(stored, reported)` — a replayed report is a no-op — so
//! over-reporting after a lost ack is the safe direction (the
//! `SyncWatermark` fail-safe reasoning, applied to money).
//!
//! # Wire contract (byte-exact; the service side is built to the same spec)
//!
//! `POST {base}/burn`, `Content-Type: application/json`, a single-line body
//! in exactly this field order:
//! `{"account_pubkey_hex":…,"signature_hex":…,"burn_kat_total":…,"meter_watermark_tokens":…}`
//! where the signature covers [`BURN_SIGNING_DOMAIN`] ‖ pubkey (32 bytes) ‖
//! `burn_kat_total` as u64 LE — 52 bytes total ([`burn_report_message`]).
//! `burn_kat_total` is the cumulative
//! [`crate::kat_meter::ChargeState::charged_kat`] (whole KAT);
//! `meter_watermark_tokens` is informational. The service derives the
//! account itself (`blake3(pubkey)[..16]` — the house truncation
//! [`crate::kat_payreq_client::account_for_pubkey`] mirrors), verifies,
//! keeps the monotone watermark, debits the balance warn-and-pay, and
//! answers
//! `200 {"accepted":true,"burn_kat_total":<stored>,"increment_kat":<applied>,"balance_micro":<i64>}`.
//! Replays are 200 no-ops; 401 forged; 400 malformed. Success = HTTP 200
//! with `"accepted":true`; EVERY other answer is a failure — the local
//! stack is retained, never half-applied.
//!
//! # Feature shape (the loud constraint)
//!
//! The pure half — message, body, parser, ack sidecar — lives behind
//! `kat_service` alone (std + serde + the account key), so the module's
//! unit tests run in a `--features kat_service` build exactly like
//! [`crate::kat_account_client`]'s parses. The TRANSPORT rides
//! `kat_transport` — like every other network surface in this repo
//! (`HttpLeaseTransport` / `HttpPayreqTransport` / `HttpAccountTransport`
//! are all ureq behind `kat_transport`, and ureq is that feature's dep).
//! **A `kat_service`-only build — the shipped release set
//! `default + latent_retrieval + kat_service` — compiles the push (and the
//! two call sites' burn lines) to nothing, exactly as it already does for
//! `sync` and the consent heartbeat.** Escaping that gate would take either
//! a new dependency or a hand-rolled HTTP client — both refused by
//! BOUNDARY.md; the gate moves when the release feature set does, not in
//! this module.

use std::path::Path;

use crate::kat_payreq_client::hex_encode;

/// The signing domain — the first 12 bytes of every burn signature. The
/// `v1` suffix is part of the pinned bytes: changing it is a wire break
/// against the service's own verifier, not a tweak.
pub const BURN_SIGNING_DOMAIN: &[u8] = b"kat:burn:v1:";

/// The exact signed bytes (always 52):
/// `BURN_SIGNING_DOMAIN (12) ‖ pubkey (32) ‖ burn_kat_total_le_u64 (8)`.
///
/// **Do not drift**: the service verifies over its own composer built to
/// the same spec, and the golden-bytes test pins the layout byte by byte
/// (seed `[42u8; 32]`, total 12345) so a reviewer — and the server side —
/// can verify it by hand without running anything.
#[must_use]
pub fn burn_report_message(pubkey: &[u8; 32], burn_kat_total: u64) -> [u8; 52] {
    let mut msg = [0u8; 52];
    msg[..12].copy_from_slice(BURN_SIGNING_DOMAIN);
    msg[12..44].copy_from_slice(pubkey);
    msg[44..].copy_from_slice(&burn_kat_total.to_le_bytes());
    msg
}

/// The single-line JSON body — the four wire fields in the pinned order.
/// Built by hand rather than `serde_json::json!` because the default
/// `serde_json` map SORTS keys and the wire contract pins the field order
/// (the [`crate::kat_consent_client::consent_body`] precedent). The
/// signature is over [`burn_report_message`] — the body is the TRANSPORT
/// for the signed facts, never an independent encoding of them.
#[must_use]
pub fn burn_report_body(
    key: &ed25519_dalek::SigningKey,
    burn_kat_total: u64,
    meter_watermark_tokens: u64,
) -> String {
    use ed25519_dalek::Signer as _;
    let pubkey = key.verifying_key().to_bytes();
    let signature = key.sign(&burn_report_message(&pubkey, burn_kat_total));
    format!(
        "{{\"account_pubkey_hex\":\"{}\",\"signature_hex\":\"{}\",\"burn_kat_total\":{burn_kat_total},\"meter_watermark_tokens\":{meter_watermark_tokens}}}",
        hex_encode(&pubkey),
        hex_encode(&signature.to_bytes()),
    )
}

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
}

/// Parse the `200` body. The `error` field is read FIRST so a refusal
/// forwarded by the transport (the
/// [`crate::kat_payreq_client`]'s `refusal_or_status` convention) surfaces
/// the service's own words verbatim.
///
/// # Errors
///
/// A human-readable string when the body is not JSON, carries an `error`,
/// does not say `accepted: true`, or is missing any of the three fields —
/// per the contract every one of those is a failure and the local stack is
/// retained.
pub fn parse_burn_response(body: &[u8]) -> Result<BurnReportAck, String> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "the burn response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the service refused the burn report — {err}"));
    }
    if v.get("accepted").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("the burn response did not say `accepted: true`".to_string());
    }
    Ok(BurnReportAck {
        burn_kat_total: v
            .get("burn_kat_total")
            .and_then(serde_json::Value::as_u64)
            .ok_or("the burn response carried no `burn_kat_total`")?,
        increment_kat: v
            .get("increment_kat")
            .and_then(serde_json::Value::as_u64)
            .ok_or("the burn response carried no `increment_kat`")?,
        balance_micro: v
            .get("balance_micro")
            .and_then(serde_json::Value::as_i64)
            .ok_or("the burn response carried no `balance_micro`")?,
    })
}

/// The ack-watermark shape version (bump = a new shape; a mismatch
/// re-watermarks at zero — over-reporting is the safe direction, the
/// server's monotone watermark absorbs it).
pub const BURN_ACK_VERSION: u8 = 1;

/// What the service last ACKNOWLEDGED — the local-commit book of the burn
/// sync. Persisted per-repo at `<root>/.heal/kat_burn_ack.json` (beside the
/// meter/charge ledgers); advanced ONLY by a 200-accepted response
/// ([`report_burn`] enforces the ≥-sent rule), so whatever it does not
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
        }
    }

    /// Exact-path loader (tests + the env-override pattern). Never aborts:
    /// missing → empty (bootstrap, no warning); corrupt / version mismatch
    /// → empty + warning. Over-reporting after a reset is a server-side
    /// no-op (monotone watermark), so zero is the fail-safe direction —
    /// never under-reporting.
    #[must_use]
    pub fn load_at(path: &Path) -> (Self, Option<String>) {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Self>(&text) {
                Ok(a) if a.version == BURN_ACK_VERSION => (a, None),
                Ok(a) => {
                    let warn = format!(
                        "burn-sync: {} carries ack version {} (this build defines \
                         {BURN_ACK_VERSION}); re-watermarking at zero — the next \
                         report carries the full cumulative total (a server-side \
                         no-op if the service already holds it)",
                        path.display(),
                        a.version
                    );
                    (Self::empty(), Some(warn))
                }
                Err(e) => {
                    let warn = format!(
                        "burn-sync: ignoring unreadable ack watermark ({}): {e}; \
                         re-watermarking at zero (over-reporting is a no-op)",
                        path.display()
                    );
                    (Self::empty(), Some(warn))
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::empty(), None),
            Err(e) => {
                let warn = format!(
                    "burn-sync: ignoring unreadable ack watermark ({}): {e}; \
                     re-watermarking at zero (over-reporting is a no-op)",
                    path.display()
                );
                (Self::empty(), Some(warn))
            }
        }
    }

    /// Persist the watermark (the post-acceptance write). Creates the
    /// parent directory if needed — the meter/charge `save` convention (the
    /// module-level caller may hand us a root where `.heal` does not exist
    /// yet; bookkeeping must not depend on another component having run
    /// first).
    ///
    /// # Errors
    ///
    /// Filesystem failure (the caller warns — a lost ack re-reports the
    /// total, which the server absorbs as a replay no-op).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("ack dir: {e}"))?;
        }
        let text =
            serde_json::to_string_pretty(self).map_err(|e| format!("ack serialize: {e}"))?;
        std::fs::write(path, text).map_err(|e| format!("ack write: {e}"))
    }
}

// ── the transport + the push (kat_transport) ──────────────────────

#[cfg(feature = "kat_transport")]
use crate::kat_account::KatAccount;

#[cfg(feature = "kat_transport")]
use crate::kat_lease_client::HttpResponse;

/// The burn channel's hard budget — the consent-heartbeat contract: a
/// best-effort billing push must never make a run feel stuck.
pub const BURN_TIMEOUT_SECS: u64 = 5;

/// The burn-channel transport (one trait per channel — the
/// [`crate::kat_payreq_client::PayreqTransport`] pattern; the ureq HTTP
/// impl here, a scripted mock in tests driving the same trait).
#[cfg(feature = "kat_transport")]
pub trait BurnTransport {
    /// `POST /burn` (signed JSON body) → the raw HTTP outcome.
    fn post_burn(&self, json: &str) -> HttpResponse;
}

/// The pure half of [`HttpBurnTransport`]'s non-200 handling — a non-200
/// whose body carries the service's `{"error": …}` becomes
/// [`HttpResponse::Ok`] of that body so the parse layer (which reads
/// `error` FIRST) is the single arbiter; a non-200 with no readable error
/// stays a bare status (an opaque failure must not be dressed up as an
/// answer). Split from the ureq-shaped caller so the branch is unit-testable
/// without a socket (the payreq `refusal_body_or_status` precedent).
#[cfg(feature = "kat_transport")]
fn burn_refusal_or_status(status: u16, body: Vec<u8>) -> HttpResponse {
    match serde_json::from_slice::<serde_json::Value>(&body).ok().and_then(
        |v| v.get("error").and_then(serde_json::Value::as_str).map(str::to_string),
    ) {
        Some(_) => HttpResponse::Ok(body),
        None => HttpResponse::Status(status),
    }
}

/// The ureq HTTP transport over the shared service base URL (the
/// [`crate::kat_payreq_client::HttpPayreqTransport`] shape: one bounded
/// agent, POST-only here).
#[cfg(feature = "kat_transport")]
pub struct HttpBurnTransport {
    agent: ureq::Agent,
    base: String,
}

#[cfg(feature = "kat_transport")]
impl HttpBurnTransport {
    /// Build with a per-request timeout in seconds (connect, headers AND
    /// body read under the same budget). Keep it SHORT — this rides every
    /// heal run.
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
impl BurnTransport for HttpBurnTransport {
    fn post_burn(&self, json: &str) -> HttpResponse {
        use std::io::Read as _;
        let url = format!("{}/burn", self.base);
        match self
            .agent
            .post(&url)
            .set("Content-Type", "application/json")
            .send_string(json)
        {
            Ok(resp) => {
                let status = resp.status();
                let mut buf = Vec::new();
                match resp.into_reader().read_to_end(&mut buf) {
                    Ok(_) if status == 200 => HttpResponse::Ok(buf),
                    Ok(_) => burn_refusal_or_status(status, buf),
                    Err(e) => HttpResponse::Error(e.to_string()),
                }
            }
            Err(ureq::Error::Status(c, resp)) => {
                let mut buf = Vec::new();
                match resp.into_reader().read_to_end(&mut buf) {
                    Ok(_) => burn_refusal_or_status(c, buf),
                    Err(_) => HttpResponse::Status(c),
                }
            }
            Err(e) => HttpResponse::Error(e.to_string()),
        }
    }
}

/// Sign + POST the cumulative burn report and parse the ack. The ONLY place
/// the ≥-sent acceptance rule lives: a 200 whose `burn_kat_total` is BELOW
/// what we sent would REWIND the account's burn history, so it is a failure
/// (the stack retained, never half-applied) even though the wire said 200.
///
/// # Errors
///
/// Transport, HTTP status, parse, or the below-sent rule — each already
/// worded for the one `burn sync: offline — …` status line.
#[cfg(feature = "kat_transport")]
pub fn report_burn(
    transport: &dyn BurnTransport,
    key: &ed25519_dalek::SigningKey,
    burn_kat_total: u64,
    meter_watermark_tokens: u64,
) -> Result<BurnReportAck, String> {
    let body = burn_report_body(key, burn_kat_total, meter_watermark_tokens);
    let bytes = match transport.post_burn(&body) {
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(c) => return Err(format!("the burn endpoint answered HTTP {c}")),
        HttpResponse::NotModified => return Err("the burn endpoint answered 304".to_string()),
        HttpResponse::Error(e) => return Err(format!("the burn endpoint is unreachable ({e})")),
    };
    let ack = parse_burn_response(&bytes)?;
    if ack.burn_kat_total < burn_kat_total {
        return Err(format!(
            "the service confirmed {} below the reported {burn_kat_total} — refusing to rewind \
             the account's burn history",
            ack.burn_kat_total
        ));
    }
    Ok(ack)
}

/// The post-run burn-sync verdict — the CLI prints exactly one line for the
/// last two variants and stays silent on the first.
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BurnSync {
    /// Not attempted: the machine is not logged in (anonymous runs never
    /// dial), or the cumulative burn is already covered by the ack.
    Skipped,
    /// The service accepted the cumulative report; the ack advanced.
    Reported(BurnReportAck),
    /// The push was attempted and failed — the delta stays stacked locally
    /// (the git analogy's local commit) and reports with the next run or
    /// `cargo heal sync`.
    Stacked {
        /// The full cumulative total that is now stacked.
        total: u64,
        /// Why the push failed (worded for the status line).
        reason: String,
    },
}

/// The post-run auto-sync entry point: the logged-in account dir + the
/// resolved service URL
/// ([`crate::kat_lease_client::service_url_for_command`]) around
/// [`run_burn_sync_at`].
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn run_burn_sync(root: &Path, charged_kat: u64, meter_watermark_tokens: u64) -> BurnSync {
    run_burn_sync_at(
        &KatAccount::default_dir(),
        root,
        &crate::kat_lease_client::service_url_for_command(),
        charged_kat,
        meter_watermark_tokens,
    )
}

/// The testable core of the burn auto-sync: load the account (anonymous
/// runs return [`BurnSync::Skipped`] WITHOUT dialing), load the ack
/// watermark, skip when there is nothing new, push the FULL cumulative
/// total, and advance the ack ONLY on acceptance. `base_url` is a
/// parameter (not env) so tests can point it at a mock or a closed port
/// hermetically.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn run_burn_sync_at(
    account_dir: &Path,
    root: &Path,
    base_url: &str,
    charged_kat: u64,
    meter_watermark_tokens: u64,
) -> BurnSync {
    let Ok(account) = KatAccount::load(account_dir) else {
        return BurnSync::Skipped;
    };
    let Ok(key) = account.signing_key() else {
        return BurnSync::Skipped;
    };
    let ack_path = root.join(".heal").join(KatBurnAck::FILE_NAME);
    let (ack, warning) = KatBurnAck::load_at(&ack_path);
    if let Some(w) = warning {
        eprintln!("{w}");
    }
    if charged_kat <= ack.burn_kat_total {
        return BurnSync::Skipped;
    }
    let transport = HttpBurnTransport::new(base_url, BURN_TIMEOUT_SECS);
    match report_burn(&transport, &key, charged_kat, meter_watermark_tokens) {
        Ok(server) => {
            let record = KatBurnAck {
                version: BURN_ACK_VERSION,
                burn_kat_total: server.burn_kat_total,
                meter_watermark_tokens,
                acked_unix: now_unix(),
            };
            if let Err(e) = record.save(&ack_path) {
                eprintln!(
                    "burn sync: could not persist the ack watermark ({}): {e} — the next run \
                     re-reports the same total (a server-side replay no-op)",
                    ack_path.display()
                );
            }
            BurnSync::Reported(server)
        }
        Err(reason) => BurnSync::Stacked {
            total: charged_kat,
            reason,
        },
    }
}

#[cfg(feature = "kat_transport")]
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[7u8; 32])
    }

    /// Lowercase-hex decode (test-only; the wire encode is `hex_encode`).
    fn hex_decode(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
            .collect()
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_clippy_burn_client_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn burn_message_golden_bytes() {
        // The spec layout, byte by byte, so a reviewer — and the server
        // side, which pins the SAME bytes — can verify by hand:
        //   "kat:burn:v1:"            -> 6b61743a6275726e3a76313a
        //   pubkey = seed [42u8; 32]  -> 2a x 32
        //   burn_kat_total = 12345 LE -> 3930000000000000  (12345 = 0x3039)
        // 12 + 32 + 8 = 52 bytes = 104 hex chars.
        let seed = [42u8; 32];
        let golden = "6b61743a6275726e3a76313a\
                      2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a\
                      3930000000000000";
        let msg = burn_report_message(&seed, 12_345);
        assert_eq!(msg.len(), 52, "the signed message is always 52 bytes");
        assert_eq!(
            hex_encode(&msg),
            golden,
            "the signed message is the spec layout, byte for byte"
        );
    }

    #[test]
    fn the_message_diverges_on_pubkey_and_total() {
        let seed = [42u8; 32];
        let base = burn_report_message(&seed, 12_345);
        let mut other_key = seed;
        other_key[0] ^= 0xFF;
        assert_ne!(
            burn_report_message(&other_key, 12_345),
            base,
            "a different pubkey must move the bytes"
        );
        assert_ne!(
            burn_report_message(&seed, 12_346),
            base,
            "a different total must move the bytes"
        );
        // The total is the LAST 8 bytes, little-endian — the field a
        // server-side debugger eyeballs first.
        assert_eq!(&base[44..], &12_345u64.to_le_bytes());
        assert_eq!(&base[..12], BURN_SIGNING_DOMAIN);
    }

    #[test]
    fn the_body_is_the_documented_single_line_in_the_documented_order() {
        use ed25519_dalek::{Signature, Verifier as _};

        let key = test_key();
        let body = burn_report_body(&key, 77, 8_800);
        assert!(!body.contains('\n'), "single-line body: {body}");
        // Pinned field order — account_pubkey_hex, signature_hex,
        // burn_kat_total, meter_watermark_tokens.
        let pub_i = body.find("\"account_pubkey_hex\"").expect("pubkey field");
        let sig_i = body.find("\"signature_hex\"").expect("signature field");
        let burn_i = body.find("\"burn_kat_total\"").expect("burn field");
        let meter_i = body.find("\"meter_watermark_tokens\"").expect("meter field");
        assert!(pub_i < sig_i && sig_i < burn_i && burn_i < meter_i, "order: {body}");
        assert!(body.starts_with("{\"account_pubkey_hex\":\""), "no leading space: {body}");

        // The body is the TRANSPORT: its signature verifies over the
        // documented message for the body's own pubkey.
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        let pubkey: [u8; 32] = hex_decode(v["account_pubkey_hex"].as_str().expect("hex"))
            .try_into()
            .expect("32 bytes");
        let sig: [u8; 64] = hex_decode(v["signature_hex"].as_str().expect("hex"))
            .try_into()
            .expect("64 bytes");
        assert_eq!(pubkey, key.verifying_key().to_bytes());
        key.verifying_key()
            .verify(
                &burn_report_message(&pubkey, 77),
                &Signature::from_slice(&sig).expect("signature bytes"),
            )
            .expect("the body signature must verify over the documented message");
    }

    #[test]
    fn parse_reads_the_documented_shapes() {
        // The documented happy shape.
        let ok = parse_burn_response(
            br#"{"accepted":true,"burn_kat_total":12345,"increment_kat":200,"balance_micro":99900000}"#,
        )
        .expect("documented shape must parse");
        assert_eq!(
            ok,
            BurnReportAck { burn_kat_total: 12_345, increment_kat: 200, balance_micro: 99_900_000 }
        );

        // A REPLAY: 200, accepted, increment 0.
        let replay = parse_burn_response(
            br#"{"accepted":true,"burn_kat_total":12345,"increment_kat":0,"balance_micro":99900000}"#,
        )
        .expect("a replay is still a 200-accepted answer");
        assert_eq!(replay.increment_kat, 0);
        assert_eq!(replay.burn_kat_total, 12_345);

        // A CLAMPED-UP answer: the server already held more (another
        // machine reported higher) — stored > reported is still success.
        let clamped = parse_burn_response(
            br#"{"accepted":true,"burn_kat_total":13000,"increment_kat":0,"balance_micro":99900000}"#,
        )
        .expect("a server-ahead answer is still success");
        assert_eq!(clamped.burn_kat_total, 13_000);

        // Warn-and-pay can drive the balance NEGATIVE — an i64 on the wire.
        let negative = parse_burn_response(
            br#"{"accepted":true,"burn_kat_total":12345,"increment_kat":12345,"balance_micro":-5}"#,
        )
        .expect("a negative balance is the documented warn-and-pay shape");
        assert_eq!(negative.balance_micro, -5);
    }

    #[test]
    fn every_non_documented_answer_is_an_explicit_failure() {
        // Not JSON.
        assert!(parse_burn_response(b"not json at all").is_err());
        // The service's refusal words surface verbatim (the transport
        // forwards non-200 error bodies to this parser).
        let refused = parse_burn_response(br#"{"error":"signature mismatch"}"#)
            .expect_err("an error body is a failure");
        assert!(refused.contains("signature mismatch"), "{refused}");
        // A 200 that does not say accepted:true.
        assert!(parse_burn_response(
            br#"{"accepted":false,"burn_kat_total":1,"increment_kat":1,"balance_micro":1}"#
        )
        .is_err());
        // Missing fields — each is a failure, never a silent default.
        for missing in [
            r#"{"accepted":true,"increment_kat":1,"balance_micro":1}"#,
            r#"{"accepted":true,"burn_kat_total":1,"balance_micro":1}"#,
            r#"{"accepted":true,"burn_kat_total":1,"increment_kat":1}"#,
        ] {
            assert!(
                parse_burn_response(missing.as_bytes()).is_err(),
                "a missing field must be a failure: {missing}"
            );
        }
    }

    #[test]
    fn the_ack_sidecar_loads_fail_safe_and_round_trips() {
        let dir = temp_root("ack");
        let path = dir.join(KatBurnAck::FILE_NAME);

        // Missing → fresh at zero, NO warning (bootstrap).
        let (ack, warning) = KatBurnAck::load_at(&path);
        assert_eq!(ack, KatBurnAck::empty());
        assert!(warning.is_none(), "a missing ack is the normal first run");

        // Round trip.
        let record = KatBurnAck {
            version: BURN_ACK_VERSION,
            burn_kat_total: 4_242,
            meter_watermark_tokens: 9_999,
            acked_unix: 1_700_000_000,
        };
        record.save(&path).expect("save");
        let (ack, warning) = KatBurnAck::load_at(&path);
        assert_eq!(ack, record);
        assert!(warning.is_none());

        // Corrupt → fresh at zero + the honest warning.
        std::fs::write(&path, "{not json").expect("write corrupt");
        let (ack, warning) = KatBurnAck::load_at(&path);
        assert_eq!(ack, KatBurnAck::empty());
        let warning = warning.expect("a corrupt ack must warn");
        assert!(warning.contains("re-watermarking at zero"), "{warning}");

        // Version mismatch → fresh at zero + the honest warning.
        let future = KatBurnAck { version: 99, ..record };
        future.save(&path).expect("save future");
        let (ack, warning) = KatBurnAck::load_at(&path);
        assert_eq!(ack, KatBurnAck::empty());
        let warning = warning.expect("a version mismatch must warn");
        assert!(warning.contains("carries ack version 99"), "{warning}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(all(test, feature = "kat_transport"))]
mod transport_tests {
    use super::*;

    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    fn test_key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[9u8; 32])
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_clippy_burn_client_t_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// The closed loopback port — connection REFUSED, instantly, no
    /// network (the hermetic "service down" arm).
    const CLOSED_PORT: &str = "http://127.0.0.1:1";

    #[derive(Debug, Clone)]
    struct Captured {
        path: String,
        body: String,
    }

    struct BurnMock {
        addr: SocketAddr,
        captured: Arc<Mutex<Vec<Captured>>>,
    }

    impl BurnMock {
        fn captured(&self) -> Vec<Captured> {
            self.captured.lock().unwrap().clone()
        }
        fn base(&self) -> String {
            format!("http://{}", self.addr)
        }
    }

    /// A one-route HTTP mock: `POST /burn` → 200 with the documented
    /// accepted shape, echoing the reported total (what a monotone server
    /// does for a fresh report). std-TCP only — no new deps.
    fn spawn_echo_burn_mock() -> BurnMock {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let cap = Arc::clone(&captured);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let Some((path, body)) = read_request(&mut stream) else { continue };
                cap.lock().unwrap().push(Captured { path, body: body.clone() });
                let total = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v.get("burn_kat_total").and_then(serde_json::Value::as_u64))
                    .unwrap_or(0);
                let reply = format!(
                    "{{\"accepted\":true,\"burn_kat_total\":{total},\"increment_kat\":{total},\"balance_micro\":99000000000000}}"
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
            }
        });
        BurnMock { addr, captured }
    }

    /// A one-shot mock answering `status` with an optional JSON body —
    /// the 401-forged arm (the documented non-200 refusal).
    fn spawn_status_mock(status: u16, reason: &str, body: &str) -> BurnMock {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let cap = Arc::clone(&captured);
        let body = body.to_string();
        let reason = reason.to_string();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let Some((path, req_body)) = read_request(&mut stream) else { continue };
                cap.lock().unwrap().push(Captured { path, body: req_body });
                let resp = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
            }
        });
        BurnMock { addr, captured }
    }

    fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        let header_end = loop {
            let n = stream.read(&mut tmp).ok()?;
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos;
            }
        };
        let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or_default();
        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_string();
        let content_length: usize = lines
            .find_map(|l| l.strip_prefix("Content-Length:").and_then(|v| v.trim().parse().ok()))
            .unwrap_or(0);
        while buf.len() < header_end + 4 + content_length {
            let n = stream.read(&mut tmp).ok()?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        let body = String::from_utf8_lossy(&buf[header_end + 4..]).into_owned();
        Some((path, body))
    }

    /// A scripted transport — the `AccountTransport` mock precedent: the
    /// same trait the ureq impl drives, answered from a fixed value.
    struct Scripted(HttpResponse);

    impl BurnTransport for Scripted {
        fn post_burn(&self, _json: &str) -> HttpResponse {
            self.0.clone()
        }
    }

    #[test]
    fn report_burn_accepts_documented_answers_and_refuses_rewinds() {
        let key = test_key();

        // The documented happy shape.
        let ok = HttpResponse::Ok(
            br#"{"accepted":true,"burn_kat_total":500,"increment_kat":500,"balance_micro":99500000}"#
                .to_vec(),
        );
        let ack = report_burn(&Scripted(ok), &key, 500, 1_000).expect("documented answer");
        assert_eq!(
            ack,
            BurnReportAck { burn_kat_total: 500, increment_kat: 500, balance_micro: 99_500_000 }
        );

        // A replay (increment 0) and a server-ahead clamp both advance —
        // the rule is ≥ sent, never == sent.
        for (name, stored) in [("replay", 500u64), ("server ahead", 900u64)] {
            let body = format!(
                r#"{{"accepted":true,"burn_kat_total":{stored},"increment_kat":0,"balance_micro":1}}"#
            );
            let ack = report_burn(&Scripted(HttpResponse::Ok(body.into_bytes())), &key, 500, 1_000)
                .unwrap_or_else(|e| panic!("{name} must be accepted: {e}"));
            assert_eq!(ack.burn_kat_total, stored, "{name}");
        }

        // ⛔ A 200 BELOW what we sent would rewind the account's burn
        // history — refused, stack retained.
        let rewind = HttpResponse::Ok(
            br#"{"accepted":true,"burn_kat_total":499,"increment_kat":0,"balance_micro":1}"#
                .to_vec(),
        );
        let err = report_burn(&Scripted(rewind), &key, 500, 1_000)
            .expect_err("a below-sent 200 must be a failure");
        assert!(err.contains("refusing to rewind"), "{err}");

        // Malformed / refusing / unreachable — all failures.
        assert!(report_burn(&Scripted(HttpResponse::Ok(b"garbage".to_vec())), &key, 500, 1_000)
            .is_err());
        let status = report_burn(&Scripted(HttpResponse::Status(401)), &key, 500, 1_000)
            .expect_err("401 is a failure");
        assert!(status.contains("HTTP 401"), "{status}");
        let dead = report_burn(
            &Scripted(HttpResponse::Error("connection refused".to_string())),
            &key,
            500,
            1_000,
        )
        .expect_err("unreachable is a failure");
        assert!(dead.contains("unreachable"), "{dead}");
    }

    #[test]
    fn a_non_200_with_the_services_error_body_surfaces_the_words() {
        // The documented 401-forged shape over a REAL socket: the refusal
        // words survive the transport boundary.
        let mock = spawn_status_mock(401, "Unauthorized", r#"{"error":"forged signature"}"#);
        let transport = HttpBurnTransport::new(&mock.base(), 5);
        let err = report_burn(&transport, &test_key(), 10, 10).expect_err("401 must fail");
        assert!(err.contains("forged signature"), "the service's words must survive: {err}");
        assert_eq!(mock.captured().len(), 1);
        assert_eq!(mock.captured()[0].path, "/burn");

        // A bare opaque status (no JSON error body) stays a status.
        let bare = spawn_status_mock(503, "Service Unavailable", "not json");
        let transport = HttpBurnTransport::new(&bare.base(), 5);
        let err = report_burn(&transport, &test_key(), 10, 10).expect_err("503 must fail");
        assert!(err.contains("HTTP 503"), "{err}");
    }

    #[test]
    fn run_burn_sync_reports_and_advances_the_ack_over_a_real_socket() {
        let account_dir = temp_root("acct");
        let root = temp_root("root");
        KatAccount::login(&account_dir).expect("hermetic login");

        let mock = spawn_echo_burn_mock();
        let outcome = run_burn_sync_at(&account_dir, &root, &mock.base(), 321, 4_000);
        let BurnSync::Reported(ack) = outcome else {
            panic!("expected Reported, got {outcome:?}");
        };
        assert_eq!(ack.burn_kat_total, 321);
        assert_eq!(ack.balance_micro, 99_000_000_000_000);

        // The captured request is the documented wire shape.
        let captured = mock.captured();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].path, "/burn");
        let v: serde_json::Value = serde_json::from_str(&captured[0].body).expect("body JSON");
        assert_eq!(
            v.get("burn_kat_total").and_then(serde_json::Value::as_u64),
            Some(321),
            "the report carries the FULL cumulative total"
        );
        assert_eq!(
            v.get("meter_watermark_tokens").and_then(serde_json::Value::as_u64),
            Some(4_000)
        );

        // The ack advanced to exactly what the server confirmed.
        let ack_path = root.join(".heal").join(KatBurnAck::FILE_NAME);
        let (on_disk, warning) = KatBurnAck::load_at(&ack_path);
        assert!(warning.is_none());
        assert_eq!(on_disk.burn_kat_total, 321);
        assert_eq!(on_disk.meter_watermark_tokens, 4_000);
        assert!(on_disk.acked_unix > 0);

        // A second call with NO new burn: skip path, zero dials.
        let outcome = run_burn_sync_at(&account_dir, &root, &mock.base(), 321, 4_000);
        assert_eq!(outcome, BurnSync::Skipped);
        assert_eq!(mock.captured().len(), 1, "no new burn must not dial");

        // A LOWER local total (the charge ledger recovered fresh under an
        // advanced ack) also skips — the server is ahead, never behind.
        let outcome = run_burn_sync_at(&account_dir, &root, &mock.base(), 5, 100);
        assert_eq!(outcome, BurnSync::Skipped);
        assert_eq!(mock.captured().len(), 1);

        let _ = std::fs::remove_dir_all(&account_dir);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn anonymous_runs_and_dead_services_never_advance_the_ack() {
        let account_dir = temp_root("anon");
        let root = temp_root("root");

        // Anonymous (no account on disk): skipped WITHOUT dialing.
        let echo = spawn_echo_burn_mock();
        let outcome = run_burn_sync_at(&account_dir, &root, &echo.base(), 50, 50);
        assert_eq!(outcome, BurnSync::Skipped);
        assert_eq!(echo.captured().len(), 0, "anonymous runs never dial");
        assert!(!root.join(".heal").join(KatBurnAck::FILE_NAME).exists());

        // Logged in + new burn + dead service: the stack is retained and
        // the ack stays absent.
        KatAccount::login(&account_dir).expect("hermetic login");
        let outcome = run_burn_sync_at(&account_dir, &root, CLOSED_PORT, 50, 50);
        let BurnSync::Stacked { total, reason } = outcome else {
            panic!("expected Stacked, got {outcome:?}");
        };
        assert_eq!(total, 50, "the FULL cumulative total stacks");
        assert!(!reason.is_empty());
        assert!(
            !root.join(".heal").join(KatBurnAck::FILE_NAME).exists(),
            "a failed push must not create the ack"
        );

        let _ = std::fs::remove_dir_all(&account_dir);
        let _ = std::fs::remove_dir_all(&root);
    }
}
