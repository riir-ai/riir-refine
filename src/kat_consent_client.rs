//! The consent-heartbeat client (owner mandate 2026-09-08) — the one-shot,
//! SIGNED tier note behind `cargo heal --mine` / `cargo heal --unmine`, so
//! the network's web front can show each machine's engagement tier
//! (opted-in / opted-out).
//!
//! # What leaves the machine — and what never does
//!
//! Exactly one POST per `--mine`/`--unmine` run: the tier (`"in"`/`"out"`),
//! a machine label, a timestamp, and the account-key Ed25519 signature over
//! those three. **No heal data** — no rules, no trajectories, no spans, no
//! source, no counts. Best-effort by contract: a skipped or failed
//! heartbeat is one `consent heartbeat: skipped — …` line and never
//! changes an exit code; healing and `sync` semantics are untouched.
//!
//! # Wire contract (byte-exact; the service side is built to the same spec)
//!
//! `POST {base}/mining/consent`, `Content-Type: application/json`, a
//! single-line body in exactly this field order:
//! `{"account_pubkey_hex":…,"consent":…,"machine":…,"ts":…,"signature_hex":…}`.
//! The signature covers [`CONSENT_SIGNING_DOMAIN`] ‖ `consent_u8` ‖
//! `machine_len_le32` ‖ `machine_utf8` ‖ `ts_le64` — `1 = "in"`, `2 =
//! "out"`, the label pre-truncated to ≤ 64 UTF-8 bytes by the caller
//! ([`machine_label`]). The service derives the submitter itself
//! (`blake3(account_pubkey)[..16]` — the house truncation
//! [`crate::kat_payreq_client::account_for_pubkey`] mirrors) and answers
//! `200 {"ok":true,…,"result":"recorded"|"unchanged"}`; any non-200 or
//! unparseable body is a heartbeat failure. The base URL rides
//! [`crate::kat_lease_client::service_url_for_command`] — no new env var.
//!
//! # Feature shape
//!
//! Behind BOTH `kat_service` (the account key, the consent record, the
//! base URL) AND `kat_transport` (ureq) — the sync-payload dual gate minus
//! the payload machinery: this module IS the proof the heartbeat carries
//! none. A build with either half missing compiles the module (and the
//! call sites' heartbeat lines) to nothing.

use crate::kat_account::KatAccount;
use crate::kat_payreq_client::hex_encode;

/// The signing domain — the first bytes of every consent signature. The
/// version suffix (`-v1`) is part of the pinned bytes: changing it is a
/// wire break against the service's own composer, not a tweak.
pub const CONSENT_SIGNING_DOMAIN: &[u8] = b"riir-kat-consent-v1";

/// The exact signed bytes:
/// `CONSENT_SIGNING_DOMAIN ‖ [consent_u8] ‖ machine_len_le32 ‖ machine_utf8
/// ‖ ts_le64` — `1 = opted in`, `2 = opted out`; the length prefix is the
/// machine label's UTF-8 byte length as `u32` little-endian.
///
/// **Do not drift**: the service verifies over its own composer built to
/// the same spec, and the golden-bytes test
/// ([`tests::consent_message_golden_bytes`]) pins the layout byte by byte
/// so a reviewer can verify it by hand without running anything.
#[must_use]
pub fn consent_signing_message(consent_in: bool, machine: &str, ts: u64) -> Vec<u8> {
    let machine = machine.as_bytes();
    let mut msg = Vec::with_capacity(CONSENT_SIGNING_DOMAIN.len() + 1 + 4 + machine.len() + 8);
    msg.extend_from_slice(CONSENT_SIGNING_DOMAIN);
    msg.push(if consent_in { 1 } else { 2 });
    let len = u32::try_from(machine.len()).unwrap_or(u32::MAX);
    msg.extend_from_slice(&len.to_le_bytes());
    msg.extend_from_slice(machine);
    msg.extend_from_slice(&ts.to_le_bytes());
    msg
}

/// The machine label for the heartbeat: `$HOST`, else `$HOSTNAME`, else
/// `$COMPUTERNAME`, else empty (an empty label is valid on the wire —
/// 0..=64 bytes). Truncated to 64 UTF-8 BYTES on a char boundary — the
/// wire contract caps bytes, and a hostname is a label, not a sentence.
/// Env cascade only: no libc, no new deps.
#[must_use]
pub fn machine_label() -> String {
    let raw = std::env::var("HOST")
        .or_else(|_| std::env::var("HOSTNAME"))
        .or_else(|_| std::env::var("COMPUTERNAME"))
        // macOS shells set HOSTNAME as a shell VARIABLE without exporting
        // it (found live 2026-09-08: /bin/sh shows m3, the child process
        // env does not) — the last resort is the platform's own hostname
        // binary. One-shot path (--mine/--unmine), local, sub-second;
        // a missing binary degrades to the empty label, never an error.
        .or_else(|_| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or(std::env::VarError::NotPresent)
        })
        .unwrap_or_default();
    clamp_machine_label(&raw)
}

/// The pure half of [`machine_label`] — byte-cap a label at 64 UTF-8
/// bytes, walking back to the nearest char boundary when byte 64 lands
/// mid-character. Split out so the truncation is testable without
/// mutating the shared process env (tests share it).
#[must_use]
pub fn clamp_machine_label(raw: &str) -> String {
    if raw.len() <= 64 {
        return raw.to_string();
    }
    let mut end = 64;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    raw[..end].to_string()
}

/// The single-line JSON body — the five wire fields in the pinned order
/// (`account_pubkey_hex`, `consent`, `machine`, `ts`, `signature_hex`).
/// Built by hand rather than `serde_json::json!` because the default
/// `serde_json` map SORTS keys and the wire contract pins the field
/// order; `serde_json::to_string` still does the string escaping for the
/// label.
///
/// The signature is over [`consent_signing_message`] — the body is the
/// TRANSPORT for the signed facts, never an independent encoding of them.
#[must_use]
pub fn consent_body(
    key: &ed25519_dalek::SigningKey,
    consent_in: bool,
    machine: &str,
    ts: u64,
) -> String {
    use ed25519_dalek::Signer as _;
    let pubkey = key.verifying_key().to_bytes();
    let signature = key.sign(&consent_signing_message(consent_in, machine, ts));
    let machine_json = serde_json::to_string(machine).unwrap_or_default();
    format!(
        "{{\"account_pubkey_hex\":\"{}\",\"consent\":\"{}\",\"machine\":{},\"ts\":{},\"signature_hex\":\"{}\"}}",
        hex_encode(&pubkey),
        if consent_in { "in" } else { "out" },
        machine_json,
        ts,
        hex_encode(&signature.to_bytes()),
    )
}

/// Parse the `200` body:
/// `{"ok":true,"submitter":…,"consent":…,"result":"recorded"|"unchanged"}`
/// → `"recorded"` / `"unchanged"`.
///
/// # Errors
///
/// A human-readable string when the body is not JSON, carries the
/// service's `error`, does not say `ok: true`, or carries a `result`
/// outside the two documented words — per the contract every one of those
/// is a heartbeat failure, never a half-success.
pub fn parse_consent_response(body: &[u8]) -> Result<&'static str, String> {
    let v: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| "the consent response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the service refused the heartbeat — {err}"));
    }
    if v.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("the consent response did not say `ok`".to_string());
    }
    match v.get("result").and_then(serde_json::Value::as_str) {
        Some("recorded") => Ok("recorded"),
        Some("unchanged") => Ok("unchanged"),
        Some(other) => Err(format!("the consent response carried an unknown result — {other}")),
        None => Err("the consent response carried no `result`".to_string()),
    }
}

/// POST the signed body to `{base}/mining/consent` and return the 200
/// body. Mirrors
/// [`crate::kat_payreq_client::HttpPayreqTransport`]'s agent/timeout
/// construction (connect, headers AND body read under the same budget)
/// and its trailing-slash discipline (`trim_end_matches('/')`). The
/// 5-second budget is the contract: a best-effort heartbeat must never
/// make an interactive command feel stuck.
fn post_consent(url_base: &str, body: &str) -> Result<Vec<u8>, String> {
    let t = std::time::Duration::from_secs(5);
    let agent = ureq::AgentBuilder::new()
        .timeout(t)
        .timeout_connect(t)
        .timeout_read(t)
        .build();
    let url = format!("{}/mining/consent", url_base.trim_end_matches('/'));
    match agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(body)
    {
        Ok(resp) if resp.status() == 200 => {
            use std::io::Read as _;
            let mut buf = Vec::new();
            resp.into_reader()
                .read_to_end(&mut buf)
                .map(|_| buf)
                .map_err(|e| format!("reading the consent response: {e}"))
        }
        Ok(resp) => Err(format!("the consent endpoint answered HTTP {}", resp.status())),
        Err(ureq::Error::Status(c, _)) => Err(format!("the consent endpoint answered HTTP {c}")),
        Err(e) => Err(format!("the consent endpoint is unreachable ({e})")),
    }
}

/// The one-line call site: load the account, read the tier the command
/// just wrote, sign the note, POST it, and report what the service said.
///
/// The TIER is read from the on-disk consent record — NOT a parameter —
/// so `--mine`/`--unmine` cannot report a tier that disagrees with what
/// they just recorded: both write consent FIRST, then call this. `Unset`
/// is an error (a tier note about a tier that does not exist would be a
/// lie); both call sites write before calling, so it is unreachable from
/// them.
///
/// # Errors
///
/// `"not logged in (cargo heal login first)"` when the account key is
/// missing; otherwise any account-key, consent-record, network, or
/// response failure — each already worded for the one-line
/// `consent heartbeat: skipped — …` print.
pub fn send_heartbeat(url_base: &str) -> Result<&'static str, String> {
    let account = KatAccount::load(&KatAccount::default_dir())
        .map_err(|_| "not logged in (cargo heal login first)".to_string())?;
    let key = account.signing_key()?;
    let consent_in = match crate::kat_mining_consent::read(&crate::kat_mining_consent::default_dir())
    {
        crate::kat_mining_consent::MiningConsent::OptedIn => true,
        crate::kat_mining_consent::MiningConsent::OptedOut => false,
        crate::kat_mining_consent::MiningConsent::Unset => {
            return Err(
                "no consent record on disk (run `cargo heal --mine` or `--unmine` first)"
                    .to_string(),
            );
        }
    };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let body = consent_body(&key, consent_in, &machine_label(), ts);
    let resp = post_consent(url_base, &body)?;
    parse_consent_response(&resp)
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

    #[test]
    fn consent_message_golden_bytes() {
        // The spec layout, byte by byte, so a reviewer can verify by hand:
        //   "riir-kat-consent-v1"    -> 726969722d6b61742d636f6e73656e742d7631
        //   consent_u8 = 1 (in)      -> 01
        //   machine_len_le32 = 8     -> 08000000
        //   "test-mac"               -> 746573742d6d6163
        //   ts_le64 = 1_700_000_000  -> 00f1536500000000  (0x0000_0000_6553_F100)
        // 19 + 1 + 4 + 8 + 8 = 40 bytes.
        let golden = "726969722d6b61742d636f6e73656e742d7631\
                      01\
                      08000000\
                      746573742d6d6163\
                      00f1536500000000";
        assert_eq!(
            hex_encode(&consent_signing_message(true, "test-mac", 1_700_000_000)),
            golden,
            "the signed message is the spec layout, byte for byte"
        );
    }

    #[test]
    fn the_out_variant_differs_only_in_the_consent_byte() {
        let mut in_msg = consent_signing_message(true, "test-mac", 1_700_000_000);
        let out_msg = consent_signing_message(false, "test-mac", 1_700_000_000);
        assert_eq!(in_msg.len(), out_msg.len());
        assert_eq!(in_msg[19], 1, "byte 19 is consent_u8: 1 = in");
        assert_eq!(out_msg[19], 2, "byte 19 is consent_u8: 2 = out");
        in_msg[19] = 2;
        assert_eq!(in_msg, out_msg, "flipping byte 19 alone reconciles in ↔ out");
    }

    #[test]
    fn the_machine_length_prefix_is_explicit_le32() {
        let msg = consent_signing_message(true, "test-mac", 0);
        assert_eq!(&msg[20..24], &[8, 0, 0, 0], "len 8 as u32 LE");
        assert_eq!(msg.len(), 19 + 1 + 4 + 8 + 8);
        let empty = consent_signing_message(true, "", 0);
        assert_eq!(&empty[20..24], &[0, 0, 0, 0], "an empty label is valid");
        // 19 + 1 + 4 + 8 = domain + consent_u8 + len prefix + ts (machine is empty).
        assert_eq!(empty.len(), 19 + 1 + 4 + 8);
    }

    #[test]
    fn clamp_keeps_short_labels_and_caps_at_64_bytes() {
        assert_eq!(clamp_machine_label(""), "");
        assert_eq!(clamp_machine_label("test-mac"), "test-mac");
        let exactly_64 = "a".repeat(64);
        assert_eq!(clamp_machine_label(&exactly_64), exactly_64);
        let sixty_five = "a".repeat(65);
        assert_eq!(
            clamp_machine_label(&sixty_five),
            exactly_64,
            "byte 65 is cut, byte 64 kept"
        );
    }

    #[test]
    fn clamp_cuts_back_to_a_char_boundary() {
        // 32 × "é" = exactly 64 bytes — fits whole.
        let fits = "é".repeat(32);
        assert_eq!(clamp_machine_label(&fits).len(), 64);
        // 33 × "é" = 66 bytes; byte 64 IS a boundary (the start of é #33)
        // — cut exactly there, nothing walked back.
        assert_eq!(clamp_machine_label(&"é".repeat(33)), fits);
        // "a" + 32 × "é" = 65 bytes; byte 64 is the SECOND byte of the
        // last é — walk back to 63, the boundary where that é starts.
        let mixed = format!("a{}", "é".repeat(32));
        let clamped = clamp_machine_label(&mixed);
        assert_eq!(clamped.len(), 63);
        assert!(clamped.starts_with('a'));
        assert_eq!(clamped.chars().count(), 32, "no half-cut character");
    }

    #[test]
    fn the_body_is_the_documented_single_line_with_the_documented_fields() {
        let body = consent_body(&test_key(), true, "test-mac", 1_700_000_000);
        assert!(!body.contains('\n'), "single line, got {body}");
        let v: serde_json::Value =
            serde_json::from_str(&body).expect("the body must parse order-insensitively");
        for field in [
            "account_pubkey_hex",
            "consent",
            "machine",
            "ts",
            "signature_hex",
        ] {
            assert!(v.get(field).is_some(), "missing field {field}");
        }
        assert_eq!(v["consent"], "in");
        assert_eq!(v["machine"], "test-mac");
        assert_eq!(v["ts"], 1_700_000_000u64);
        let pubkey_hex = v["account_pubkey_hex"].as_str().expect("string");
        assert_eq!(pubkey_hex.len(), 64, "32 bytes as hex");
        assert!(
            pubkey_hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "lowercase hex only, got {pubkey_hex}"
        );
        let sig_hex = v["signature_hex"].as_str().expect("string");
        assert_eq!(sig_hex.len(), 128, "64 bytes as hex");
    }

    #[test]
    fn the_body_escapes_the_machine_label_and_carries_the_out_tier() {
        let body = consent_body(&test_key(), false, "we\"rd\\name", 7);
        let v: serde_json::Value = serde_json::from_str(&body).expect("escaped body parses");
        assert_eq!(v["machine"], "we\"rd\\name", "JSON escaping round-trips");
        assert_eq!(v["consent"], "out");
        assert_eq!(v["ts"], 7u64);
    }

    #[test]
    fn the_signed_body_verifies_over_the_documented_message() {
        // The full wire contract in one assertion: the signature in the
        // body verifies — with the account_pubkey_hex in the body — over
        // consent_signing_message of the same facts. The body is the
        // transport for the signed facts, never an independent encoding.
        use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
        let key = test_key();
        let body = consent_body(&key, true, "hb-host", 42);
        let v: serde_json::Value = serde_json::from_str(&body).expect("parses");
        let pubkey: [u8; 32] = hex_decode(v["account_pubkey_hex"].as_str().expect("hex"))
            .try_into()
            .expect("32 bytes");
        let sig: [u8; 64] = hex_decode(v["signature_hex"].as_str().expect("hex"))
            .try_into()
            .expect("64 bytes");
        let vk = VerifyingKey::from_bytes(&pubkey).expect("valid key");
        vk.verify(
            &consent_signing_message(true, "hb-host", 42),
            &Signature::from_bytes(&sig),
        )
        .expect("the body signature must verify over the documented message");
    }

    #[test]
    fn parse_consent_response_reads_both_documented_results() {
        assert_eq!(
            parse_consent_response(
                br#"{"ok":true,"submitter":"ab","consent":"in","result":"recorded"}"#
            ),
            Ok("recorded")
        );
        assert_eq!(
            parse_consent_response(
                br#"{"ok":true,"submitter":"ab","consent":"out","result":"unchanged"}"#
            ),
            Ok("unchanged")
        );
    }

    #[test]
    fn every_non_documented_answer_is_an_explicit_failure() {
        for (body, needle) in [
            (b"not json at all".as_slice(), "not JSON"),
            (
                br#"{"error":"consent endpoint disabled"}"#.as_slice(),
                "consent endpoint disabled",
            ),
            (
                br#"{"ok":false,"result":"recorded"}"#.as_slice(),
                "did not say `ok`",
            ),
            (br#"{"ok":true}"#.as_slice(), "no `result`"),
            (
                br#"{"ok":true,"result":"maybe"}"#.as_slice(),
                "unknown result",
            ),
        ] {
            let err = parse_consent_response(body).expect_err("must fail");
            assert!(err.contains(needle), "body {body:?} → {err}");
        }
    }
}
