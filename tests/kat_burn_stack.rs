//! Issue 076 (owner mandate) — the burn auto-sync STACKING test: the git
//! analogy driven through the REAL `cargo-heal` binary against a local
//! std-TCP mock HTTP server (the `tests/kat_lease_client_g10.rs` harness
//! shape — no network, no new deps).
//!
//! | phase | service | expected |
//! |---|---|---|
//! | 1. a run with burn N, service down | closed port | exit 0, ONE `burn sync: … stacked` line, NO ack file — the delta is a local commit |
//! | 2. the next run, burn grows to Y | live mock | ONE `POST /burn` carrying `burn_kat_total == Y` (the FULL stacked total, not the delta), the signature verifies over the documented message, the ack advances to Y, the success line carries the server balance from the response |
//! | 3. a run with NO new burn | live mock | ZERO requests (skip path), no `burn sync:` line, exit 0, ack untouched |
//!
//! Every run gets its own scratch root (`RIIR_HEAL_TRAJ_ROOT`) + account
//! dir (`RIIR_HEAL_KAT_ACCOUNT_DIR`) + service URL; the lease/reheat lanes
//! are kill-switched so the burn push is the only dial under test. The real
//! `~/.config/riir-heal` is never read or written.
//!
//! Gated on the cargo-heal bin's own `required-features` plus
//! `kat_transport` (the burn client's transport half). Without them the
//! file compiles to an empty binary that prints a green `0 passed`, which
//! is why the `required-features` row in Cargo.toml is not optional (the
//! Issue-713 green-zero trap).

#![cfg(all(
    feature = "kat_transport",
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints"
))]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use riir_clippy::kat_burn_client::{self, KatBurnAck};

/// The closed loopback port — connection REFUSED, instantly, no network.
const CLOSED_PORT: &str = "http://127.0.0.1:1";

/// One tiny source file — enough to bill a nonzero cumulative burn on the
/// first run (1 KAT per code word token).
const A_RS: &str = "pub fn add(a: u32, b: u32) -> u32 { a.wrapping_add(b) }\n";

// ── the mock server (std TCP; the kat_lease_client_g10 shape) ─────────────

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

/// A one-route HTTP mock: `POST /burn` → 200 with the documented accepted
/// shape, echoing the reported total (what a monotone server does for a
/// fresh report).
fn spawn_burn_mock() -> BurnMock {
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

/// Read one HTTP request (headers + Content-Length body). `None` on EOF.
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

// ── the hermetic CLI runner (the unmine_consent_cli pattern) ──────────────

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_clippy_burn_stack_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One binary invocation, hermetic: its own account dir, its own `.heal`
/// root, the given service URL, and the lease/reheat kill switches so the
/// burn push is the ONLY dial under test.
fn heal(dir: &Path, url: &str, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-heal"))
        .current_dir(dir)
        .env("RIIR_HEAL_KAT_ACCOUNT_DIR", dir)
        .env("RIIR_HEAL_TRAJ_ROOT", dir)
        .env("RIIR_HEAL_KAT_SERVICE_URL", url)
        .env("RIIR_HEAL_KAT_LEASE", "0")
        .args(args)
        .output()
        .expect("spawn cargo-heal");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

fn login(dir: &Path) {
    let (code, text) = heal(dir, CLOSED_PORT, &["login"]);
    assert_eq!(code, Some(0), "hermetic login must succeed. output:\n{text}");
}

fn write_a_rs(dir: &Path) {
    std::fs::write(dir.join("a.rs"), A_RS).expect("write a.rs");
}

/// The settled cumulative burn, read off the charge ledger the run saved.
fn charged_kat(dir: &Path) -> u64 {
    let text =
        std::fs::read_to_string(dir.join(".heal").join("kat_charge.json")).expect("charge file");
    serde_json::from_str::<serde_json::Value>(&text)
        .expect("charge json")
        .get("charged_kat")
        .and_then(serde_json::Value::as_u64)
        .expect("charged_kat")
}

fn ack_path(dir: &Path) -> PathBuf {
    dir.join(".heal").join(KatBurnAck::FILE_NAME)
}

fn ack_of(dir: &Path) -> Option<KatBurnAck> {
    let text = std::fs::read_to_string(ack_path(dir)).ok()?;
    Some(serde_json::from_str(&text).expect("ack json"))
}

fn burn_sync_lines(text: &str) -> usize {
    text.matches("burn sync:").count()
}

/// Lowercase-hex decode (the wire encode is `hex_encode`).
fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

// ── the git analogy ───────────────────────────────────────────────────────

/// Phase 1: a push that does not pass through is a LOCAL COMMIT — one
/// stacked line, no ack file, exit code unaffected.
#[test]
fn a_failed_push_is_a_local_commit() {
    let dir = scratch("stack");
    write_a_rs(&dir);
    login(&dir);

    let (code, text) = heal(&dir, CLOSED_PORT, &[]);
    assert_eq!(
        code, Some(0),
        "a failed burn push must never change the exit code. output:\n{text}"
    );
    let total = charged_kat(&dir);
    assert!(total > 0, "the run must bill something to stack");
    assert_eq!(
        burn_sync_lines(&text),
        1,
        "exactly one burn-sync status line. output:\n{text}"
    );
    assert!(text.contains("stacked"), "the failure line must say stacked. output:\n{text}");
    assert!(
        text.contains(&total.to_string()),
        "the stacked line must carry the stacked total {total}. output:\n{text}"
    );
    assert!(
        !ack_path(&dir).exists(),
        "a failed push must NOT create the ack watermark"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 2: the NEXT run commits + pushes the FULL stacked total in one
/// report; the ack advances; the balance line comes from the response.
#[test]
fn the_next_run_pushes_the_full_stacked_total_and_advances_the_ack() {
    use ed25519_dalek::{Signature, Verifier as _};

    let dir = scratch("push");
    write_a_rs(&dir);
    login(&dir);

    // Phase 1: stack against a dead service.
    let (code1, _) = heal(&dir, CLOSED_PORT, &[]);
    assert_eq!(code1, Some(0));
    let x = charged_kat(&dir);
    assert!(x > 0, "the first run must bill something");
    assert!(ack_of(&dir).is_none(), "the failed push must not advance the ack");

    // Phase 2: the live mock.
    let mock = spawn_burn_mock();
    let (code2, text2) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code2, Some(0), "output:\n{text2}");
    let y = charged_kat(&dir);
    assert!(y > x, "the second run bills more (y={y}, x={x})");

    let captured = mock.captured();
    assert_eq!(captured.len(), 1, "exactly one request. output:\n{text2}");
    assert_eq!(captured[0].path, "/burn");

    let v: serde_json::Value =
        serde_json::from_str(&captured[0].body).expect("the request body must be JSON");
    let sent = v
        .get("burn_kat_total")
        .and_then(serde_json::Value::as_u64)
        .expect("burn_kat_total");
    assert_eq!(
        sent, y,
        "the report carries the FULL stacked total, not just the delta"
    );
    let watermark = v
        .get("meter_watermark_tokens")
        .and_then(serde_json::Value::as_u64)
        .expect("meter_watermark_tokens");
    assert!(watermark > 0, "the meter watermark rides along");

    // The signature verifies over the documented message for the claimed key.
    let pubkey_hex = v
        .get("account_pubkey_hex")
        .and_then(serde_json::Value::as_str)
        .expect("account_pubkey_hex");
    let sig_hex = v
        .get("signature_hex")
        .and_then(serde_json::Value::as_str)
        .expect("signature_hex");
    assert_eq!(pubkey_hex.len(), 64, "64 hex chars of ed25519 pubkey");
    assert_eq!(sig_hex.len(), 128, "128 hex chars of ed25519 signature");
    let pubkey: [u8; 32] = hex_decode(pubkey_hex).try_into().expect("pubkey bytes");
    let sig: [u8; 64] = hex_decode(sig_hex).try_into().expect("signature bytes");
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&pubkey).expect("verifying key");
    verifying
        .verify(
            &kat_burn_client::burn_report_message(&pubkey, sent),
            &Signature::from_slice(&sig).expect("signature bytes"),
        )
        .expect("the burn report signature must verify over the documented message");

    // The ack advanced to exactly what the server confirmed.
    let ack = ack_of(&dir).expect("the ack must exist after an accepted push");
    assert_eq!(ack.version, 3);
    assert_eq!(ack.burn_kat_total, sent);
    assert_eq!(ack.meter_watermark_tokens, watermark);
    assert!(ack.acked_unix > 0);
    assert_eq!(
        ack.last_balance_micro,
        Some(99_000_000_000_000),
        "the v3 ack carries the last-synced server balance (the 075 gate input)"
    );

    // The success line: exactly one, carrying the fresh server balance
    // from the push response (99_000_000_000_000 micro = 99000000.000000 KAT).
    assert_eq!(burn_sync_lines(&text2), 1, "output:\n{text2}");
    assert!(text2.contains("reported"), "output:\n{text2}");
    assert!(
        text2.contains("99000000.000000 KAT"),
        "the server balance must come from the push response. output:\n{text2}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 3: a run with no new burn sends NOTHING (the skip path).
#[test]
fn a_run_with_no_new_burn_sends_nothing() {
    let dir = scratch("skip");
    write_a_rs(&dir);
    login(&dir);

    // Stack, then drain.
    let (code1, _) = heal(&dir, CLOSED_PORT, &[]);
    assert_eq!(code1, Some(0));
    let mock = spawn_burn_mock();
    let (code2, text2) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code2, Some(0), "output:\n{text2}");
    assert_eq!(mock.captured().len(), 1, "the drain push");
    let ack = ack_of(&dir).expect("drained");
    assert_eq!(
        ack.burn_kat_total,
        charged_kat(&dir),
        "the drain leaves the cumulative fully reported"
    );

    // No new burn: the source file now bills ZERO tokens (whitespace-only
    // — `count_word_tokens` bills nothing for it, pinned by kat_meter's own
    // tests), so the run settles a zero delta and `charged_kat == ack` →
    // the skip path, zero dials. (Deleting the file entirely would exit 1
    // at the no-sources guard BEFORE the charge block — the wrong reason.)
    std::fs::write(dir.join("a.rs"), "\n").expect("blank out a.rs");
    let mock3 = spawn_burn_mock();
    let (code3, text3) = heal(&dir, &mock3.base(), &[]);
    assert_eq!(code3, Some(0), "output:\n{text3}");
    assert_eq!(
        mock3.captured().len(),
        0,
        "nothing may be dialed when there is nothing new. output:\n{text3}"
    );
    assert_eq!(
        burn_sync_lines(&text3),
        0,
        "a skip is silent. output:\n{text3}"
    );
    assert_eq!(
        ack_of(&dir).as_ref().map(|a| a.burn_kat_total),
        Some(ack.burn_kat_total),
        "the ack is untouched"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
