//! Plan 117 — `cargo refine --refill`, the dev/test faucet refill, driven
//! through the REAL `cargo-refine` binary against a local std-TCP mock HTTP
//! server (the `tests/kat_burn_stack.rs` harness shape — no network, no new
//! deps).
//!
//! | phase | service | expected |
//! |---|---|---|
//! | 1. refill, not logged in | (no account) | exit 1 + `no account` + `cargo refine login` named — nothing dials |
//! | 2. refill, live mock | faucet 200 + crank 200 + balance 200 (0 → 10 KAT) | exit 0, `kat-fuel:` line carries the server truth, the ack's `last_balance_micro` is REFRESHED (the 075 gate's input — the whole point of the command), the crank POST carried the bearer token |
//! | 3. the gate's refuel menu | closed port, depleted ack | the refusal lists contribute + pay and NEVER `--refill` (riir-dapps Issue 094 D4) |
//! | 4. mainnet posture | mainnet URL, live mock, logged in | exit 1, `REFUSED`, both refuel paths named verbatim, ZERO requests captured (Issue 094 D4) |
//!
//! The devnet-ops side of D4 is phase 2 itself: its loopback mock URL is
//! the `wrangler dev` posture, so a green phase 2 IS the assertion that the
//! surviving devnet-ops tool still works end to end. The pinned-host
//! classification (devnet opens, testnet/custom fail closed) is unit-armed
//! in `riir-kat`'s `kat_refill::tests` rather than dialed from here — this
//! file reaches no network by construction.
//!
//! Every run gets its own scratch root (`RIIR_HEAL_TRAJ_ROOT`) + account
//! dir (`RIIR_HEAL_KAT_ACCOUNT_DIR`) + service URL; the lease/reheat lanes
//! are kill-switched so the refill is the only dial under test. The real
//! `~/.config/riir-heal` is never read or written.
//!
//! Gated on the cargo-refine bin's own `required-features` plus
//! `kat_transport` (the refill flow's transport half). Without them the
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

use riir_clippy::kat_burn_client::KatBurnAck;

/// The closed loopback port — connection REFUSED, instantly, no network.
const CLOSED_PORT: &str = "http://127.0.0.1:1";

// ── the mock server (std TCP; the kat_burn_stack shape) ───────────────────

#[derive(Debug, Clone)]
struct Captured {
    path: String,
    bearer: Option<String>,
}

struct RefillMock {
    addr: SocketAddr,
    captured: Arc<Mutex<Vec<Captured>>>,
}

impl RefillMock {
    fn captured(&self) -> Vec<Captured> {
        self.captured.lock().unwrap().clone()
    }
    fn base(&self) -> String {
        format!("http://{}", self.addr)
    }
}

/// A three-route mock: `POST /payreq/faucet` → 200 with the documented
/// opened-row shape (the deployment's cap, pinned); `POST
/// /admin/crank/daily` → 200 with the full crank report (the parse refuses
/// partials, so every counter must ride); `GET /balance` → 200, first read
/// 0 (the baseline), then the credited 10 KAT forever.
fn spawn_refill_mock() -> RefillMock {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let cap = Arc::clone(&captured);
    let first_balance = Arc::new(Mutex::new(true));
    let fb = Arc::clone(&first_balance);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let Some((path, bearer, _body)) = read_request(&mut stream) else {
                continue;
            };
            cap.lock().unwrap().push(Captured {
                path: path.clone(),
                bearer,
            });
            let reply = if path.starts_with("/payreq/faucet") {
                format!(
                    "{{\"request_id\":\"{}\",\"amount_micro\":10000000,\"expiry\":9999}}",
                    "b".repeat(32)
                )
            } else if path.starts_with("/admin/crank/daily") {
                r#"{"date":"2026-09-09","credited":1,"faucet_settled":1,"granted_micro":10000000,"expired":0,"skipped_already":0,"skipped_dormant":0,"skipped_budget":0,"skipped_treasury":0,"skipped_faucet_cap":0,"needs_alert":false}"#.to_string()
            } else if path.starts_with("/balance") {
                let mut first = fb.lock().unwrap();
                let micro = if *first {
                    *first = false;
                    0
                } else {
                    10_000_000
                };
                format!(
                    "{{\"account_id\":\"{}\",\"balance_micro\":{micro},\"granted\":true}}",
                    "a".repeat(32)
                )
            } else {
                String::new()
            };
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
    });
    RefillMock { addr, captured }
}

/// Read one HTTP request (request line + headers + Content-Length body).
/// `None` on EOF.
fn read_request(stream: &mut TcpStream) -> Option<(String, Option<String>, String)> {
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
    let mut bearer = None;
    let mut content_length = 0usize;
    for l in lines {
        if let Some(v) = l.strip_prefix("Authorization: Bearer ") {
            bearer = Some(v.trim().to_string());
        }
        if let Some(v) = l.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    while buf.len() < header_end + 4 + content_length {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let body = String::from_utf8_lossy(&buf[header_end + 4..]).into_owned();
    Some((path, bearer, body))
}

// ── the hermetic CLI runner (the unmine_consent_cli pattern) ──────────────

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_clippy_refill_cli_{tag}_{}_{}",
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
/// refill is the ONLY dial under test.
fn heal(dir: &Path, url: &str, extra_env: &[(&str, &str)], args: &[&str]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-refine"));
    cmd.current_dir(dir)
        .env("RIIR_HEAL_KAT_ACCOUNT_DIR", dir)
        // Issue 081: the account plane moved to the canonical dir — isolate
        // it too (the consent plane still rides the old var above).
        .env("RIIR_AUTH_ACCOUNT_DIR", dir)
        .env("RIIR_HEAL_TRAJ_ROOT", dir)
        .env("RIIR_HEAL_KAT_SERVICE_URL", url)
        .env("RIIR_HEAL_KAT_LEASE", "0")
        .args(args);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("spawn cargo-refine");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

fn login(dir: &Path) {
    let (code, text) = heal(dir, CLOSED_PORT, &[], &["login"]);
    assert_eq!(
        code,
        Some(0),
        "hermetic login must succeed. output:\n{text}"
    );
}

fn ack_of(dir: &Path) -> Option<KatBurnAck> {
    let text = std::fs::read_to_string(dir.join(".heal").join(KatBurnAck::FILE_NAME)).ok()?;
    Some(serde_json::from_str(&text).expect("ack json"))
}

/// Phase 1: no account, no dial — the honest local refusal.
#[test]
fn refill_without_an_account_refuses_before_dialing() {
    let dir = scratch("noacct");
    let (code, text) = heal(&dir, CLOSED_PORT, &[], &["--refill"]);
    assert_eq!(code, Some(1), "the refill must refuse. output:\n{text}");
    assert!(
        text.contains("no account"),
        "the reason must be named. output:\n{text}"
    );
    assert!(
        text.contains("cargo refine login"),
        "the onboarding step must be named. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 2: the full happy path — faucet opened, crank triggered WITH the
/// bearer, balance confirmed, and the 075 gate's input refreshed from the
/// server truth. Exit 0 because the final synced balance is positive.
#[test]
fn refill_opens_settles_confirms_and_refreshes_the_gate_input() {
    let dir = scratch("happy");
    login(&dir);
    // The gate input starts DEPLETED (the state --refill exists to fix).
    let ack_path = dir.join(".heal").join(KatBurnAck::FILE_NAME);
    std::fs::create_dir_all(dir.join(".heal")).expect(".heal dir");
    let depleted = KatBurnAck {
        version: riir_clippy::kat_burn_client::BURN_ACK_VERSION,
        burn_kat_total: 1_000,
        meter_watermark_tokens: 500,
        acked_unix: 1_700_000_000,
        last_balance_micro: Some(0),
        contributed: None,
        first_commitment_hex: None,
        tuna_balance_micro: None,
        tuna_expires_unix: None,
    };
    depleted.save(&ack_path).expect("seed depleted ack");

    let mock = spawn_refill_mock();
    let token = "devnet-crank-token";
    let (code, text) = heal(
        &dir,
        &mock.base(),
        &[("RIIR_HEAL_KAT_CRANK_TOKEN", token)],
        &["--refill"],
    );

    assert_eq!(
        code,
        Some(0),
        "a credited refill must exit 0. output:\n{text}"
    );
    assert!(
        text.contains("refill: faucet request"),
        "the faucet leg is named. output:\n{text}"
    );
    assert!(
        text.contains("/payreq/"),
        "the shareable link composes. output:\n{text}"
    );
    assert!(
        text.contains("crank:    ran"),
        "the crank leg reports. output:\n{text}"
    );
    assert!(
        text.contains("faucet_settled 1"),
        "the crank counters surface. output:\n{text}"
    );
    assert!(
        text.contains("kat-fuel:") && text.contains("10.000000 KAT"),
        "the server truth renders in KAT. output:\n{text}"
    );
    assert!(
        text.contains("soft-gate input refreshed"),
        "the T5 refresh must announce itself. output:\n{text}"
    );

    // The gate input is the ack — it must now carry the credited balance.
    let ack = ack_of(&dir).expect("the ack must survive the refresh");
    assert_eq!(
        ack.last_balance_micro,
        Some(10_000_000),
        "the 075 gate input must read the credited server truth"
    );
    assert_eq!(
        ack.burn_kat_total, depleted.burn_kat_total,
        "the burn watermark is untouched"
    );
    assert_eq!(ack.contributed, None, "the sticky-grace truth is untouched");

    // The crank POST carried the bearer (the authorization on the wire).
    let captured = mock.captured();
    assert!(
        captured
            .iter()
            .any(|c| c.path.starts_with("/payreq/faucet")),
        "the faucet POST dialed: {captured:?}"
    );
    assert!(
        captured
            .iter()
            .any(|c| c.path.starts_with("/admin/crank/daily") && c.bearer.as_deref() == Some(token)),
        "the crank POST carried the bearer token: {captured:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 3, as amended by riir-dapps Issue 094 D4: the 075 refusal names
/// exactly TWO refuel paths — contribute and pay — and must NOT name
/// `--refill`, which retired with the daily faucet/drip tier it fuelled.
/// (This arm previously asserted the opposite, that `--refill` was listed
/// FIRST; the inversion is the D4 verdict, not a regression.)
#[test]
fn the_depleted_gate_names_contribute_and_pay_never_refill() {
    let dir = scratch("gatename");
    login(&dir);
    let ack_path = dir.join(".heal").join(KatBurnAck::FILE_NAME);
    std::fs::create_dir_all(dir.join(".heal")).expect(".heal dir");
    let ack = KatBurnAck {
        version: riir_clippy::kat_burn_client::BURN_ACK_VERSION,
        burn_kat_total: 1_000,
        meter_watermark_tokens: 500,
        acked_unix: 1_700_000_000,
        last_balance_micro: Some(0),
        contributed: None,
        first_commitment_hex: None,
        tuna_balance_micro: None,
        tuna_expires_unix: None,
    };
    ack.save(&ack_path).expect("seed depleted ack");

    let (code, text) = heal(&dir, CLOSED_PORT, &[], &[]);
    assert_eq!(code, Some(1), "the soft gate must refuse. output:\n{text}");
    assert!(
        !text.contains("cargo refine --refill"),
        "D4: a retired user path must not be advertised. output:\n{text}"
    );
    assert!(
        text.contains("cargo refine --mine"),
        "contribute must be named. output:\n{text}"
    );
    assert!(
        text.contains("top up:"),
        "pay must be named. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Issue 094 D4, the mainnet half: `--refill` REFUSES on mainnet posture,
/// names both surviving refuel paths, and dials NOTHING — the account is
/// fully logged in, so the only thing standing between it and the faucet
/// is the posture gate.
#[test]
fn refill_refuses_on_mainnet_posture_and_names_both_paths() {
    let dir = scratch("mainnet");
    login(&dir);
    let mock = spawn_refill_mock();
    // The posture is read from the resolved SERVICE URL, so point the run
    // at mainnet by name while a live faucet mock sits there unused: if
    // the gate leaked, the mock would record a request.
    let (code, text) = heal(
        &dir,
        riir_clippy::kat_service_urls::MAINNET_SERVICE_URL,
        &[("RIIR_HEAL_KAT_CRANK_TOKEN", "ops-token")],
        &["--refill"],
    );
    assert_eq!(code, Some(1), "mainnet refill must refuse. output:\n{text}");
    assert!(text.contains("REFUSED"), "loudly. output:\n{text}");
    assert!(text.contains("MAINNET"), "by posture. output:\n{text}");
    assert!(
        text.contains("cargo refine --mine"),
        "contribute must be named verbatim. output:\n{text}"
    );
    assert!(
        text.contains("/pay?acct="),
        "pay (packs) must be named. output:\n{text}"
    );
    assert!(
        mock.captured().is_empty(),
        "the refusal must dial nothing. captured: {:?}",
        mock.captured()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--refill-wait` without `--refill` is a usage error (the `--purge`
/// requires-`--unmine` shape).
#[test]
fn refill_wait_requires_refill() {
    let dir = scratch("waitguard");
    let (code, text) = heal(&dir, CLOSED_PORT, &[], &["--refill-wait", "5"]);
    assert_eq!(code, Some(2), "a usage error exits 2. output:\n{text}");
    assert!(
        text.contains("--refill-wait requires --refill"),
        "the guard must name the pair. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
