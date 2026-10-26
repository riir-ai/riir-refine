//! riir-dapps Plan 045 B7 — `cargo refine topup`, the USDC→KAT checkout,
//! driven through the REAL `cargo-refine` binary against a local std-TCP
//! mock HTTP server (the `kat_refill_cli` harness shape — no network, no
//! new deps).
//!
//! | phase | service | expected |
//! |---|---|---|
//! | 1. topup, not logged in | (no account) | exit 1 + `no account` + `cargo refine login` named — nothing dials |
//! | 2. catalog + intent, live mock | `/pay/catalog` 200 + `/pay/intent` 200 | exit 0, the catalog lines render the SERVER's numbers, the intent lines carry the `solana:` URL + the reference, and the POST body was the SIGNED wire (`pubkey_hex`/`signature_hex`/`pack_id`) |
//! | 3. not armed here | `/pay/catalog` 503 `unconfigured` | exit 1, the honest posture names the env + `{url}/pay` — never a retry |
//! | 4. no pack, non-TTY | catalog 200 | exit 2 + `--pack N` named (agents never guess a price) — ONE catalog dial |
//! | 5. account --history purchases | `/history` 200 with the additive page | the purchases section renders (open → `—` credit, settled → the KAT figure) — the B7 read half |
//!
//! The signature is verified MOCK-SIDE against the canonical composer
//! (`kat_protocol_pay::pay_intent_message`) — the two-sided pin at the
//! process boundary.
//!
//! Every run gets its own scratch root + account dir + service URL; the
//! lease/reheat lanes are kill-switched. Gated on the bin's own
//! `required-features` so `CARGO_BIN_EXE_cargo-refine` exists (the
//! Issue-713 green-zero trap).

#![cfg(all(
    feature = "kat_transport",
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints"
))]

#[path = "support/hermetic_home.rs"]
mod hermetic_home;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use riir_refine::kat_protocol_account::ServiceHistory;

/// The closed loopback port — connection REFUSED, instantly, no network.
const CLOSED_PORT: &str = "http://127.0.0.1:1";

const CATALOG_BODY: &str = r#"{"packs":[{"id":1,"usdc_base":1000000,"kat_micro":10000000},{"id":2,"usdc_base":5000000,"kat_micro":50000000},{"id":3,"usdc_base":20000000,"kat_micro":200000000}],"rate_kat_per_usdc":10,"mint_b58":"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU","payee_b58":"gistXmdXerQ8oWMpdxXhva65pgkAdteQJwyGLGRe5zN","max_open":9,"max_open_per_acct":2,"ttl_secs":86400}"#;
const UNCONFIGURED_BODY: &str =
    r#"{"error":"pay: not armed on this environment","code":"unconfigured"}"#;

#[derive(Debug, Clone)]
struct Captured {
    method: String,
    path: String,
    body: String,
}

struct TopupMock {
    addr: SocketAddr,
    captured: Arc<Mutex<Vec<Captured>>>,
}

impl TopupMock {
    fn captured(&self) -> Vec<Captured> {
        self.captured.lock().unwrap().clone()
    }
    fn base(&self) -> String {
        format!("http://{}", self.addr)
    }
}

/// The behavior table: each route answers per `mode`.
fn spawn_topup_mock(mode: &'static str) -> TopupMock {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let cap = Arc::clone(&captured);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let Some((path, method, body)) = read_request(&mut stream) else {
                continue;
            };
            cap.lock().unwrap().push(Captured {
                path: path.clone(),
                method,
                body,
            });
            let (status, reply) = match (mode, path.split('?').next().unwrap_or(&path)) {
                ("ok", "/pay/catalog") | ("capped2", "/pay/catalog") => {
                    (200, CATALOG_BODY.to_string())
                }
                ("unarmed", "/pay/catalog") => (503, UNCONFIGURED_BODY.to_string()),
                ("ok", "/pay/intent") => (
                    200,
                    r#"{"seq":7,"pack_id":1,"usdc_base":1000000,"kat_micro":10000000,"reference":"3RefX7","url":"solana:pay?example","expires_unix":1800062400,"status":"open"}"#
                        .to_string(),
                ),
                // The capped posture (issue 126): the creation refuses —
                // the account already holds open checkout(s).
                ("capped2", "/pay/intent") => (
                    429,
                    r#"{"error":"pay: 2 intents already open for this account — settle or wait","code":"rate_limited"}"#
                        .to_string(),
                ),
                // Issue 126 — the actions answer the canned intent body
                // (the SAME PayIntentCreated shape; show/void are the
                // server's own re-compose/terminal marks — the mock needs
                // no crypto because the REAL signature checks live in the
                // worker contract tier).
                ("ok", "/pay/intent/void") | ("capped2", "/pay/intent/void") => (
                    200,
                    r#"{"seq":7,"status":"voided"}"#.to_string(),
                ),
                ("ok", "/pay/intent/show") | ("capped2", "/pay/intent/show") => (
                    200,
                    r#"{"seq":7,"pack_id":1,"usdc_base":1000000,"kat_micro":10000000,"reference":"3RefX7","url":"solana:pay?example","expires_unix":1800062400,"status":"open"}"#
                        .to_string(),
                ),
                ("ok", "/history") => (
                    200,
                    format!(
                        r#"{{"account_id":"{}","balance_micro":8000000,"granted":false,"contributed":false,"burn":null,"first_commitment_hex":null,"cegel_balance_micro":0,"seal_balance_micro":0,"receipts":[],"receipts_total_matching":0,"receipts_truncated":false,"events":[],"events_total":0,"events_truncated":false,"tuna_balance_micro":47200000,"tuna_expires_unix":2000000000,"purchases":[{{"seq":7,"pack_id":1,"status":"open","usdc_base":1000000,"paid_base":0,"kat_micro":0,"created_unix":10,"settled_unix":0,"reference":"3RefX7"}},{{"seq":2,"pack_id":1,"status":"settled","usdc_base":1000000,"paid_base":1000000,"kat_micro":10000000,"created_unix":5,"settled_unix":9,"reference":""}}],"purchases_total":2,"purchases_truncated":false}}"#,
                        "a".repeat(32)
                    ),
                ),
                // Two OPEN checkouts, newest FIRST (the server's order) —
                // the >1 arm of the 429 surface (the picker, --pick latest).
                ("capped2", "/history") => (
                    200,
                    format!(
                        r#"{{"account_id":"{}","balance_micro":0,"granted":false,"contributed":false,"burn":null,"first_commitment_hex":null,"cegel_balance_micro":0,"seal_balance_micro":0,"receipts":[],"receipts_total_matching":0,"receipts_truncated":false,"events":[],"events_total":0,"events_truncated":false,"purchases":[{{"seq":8,"pack_id":1,"status":"open","usdc_base":1000000,"paid_base":0,"kat_micro":0,"created_unix":20,"settled_unix":0,"reference":"RefNew"}},{{"seq":7,"pack_id":1,"status":"open","usdc_base":1000000,"paid_base":0,"kat_micro":0,"created_unix":10,"settled_unix":0,"reference":"3RefX7"}}],"purchases_total":2,"purchases_truncated":false}}"#,
                        "a".repeat(32)
                    ),
                ),
                _ => (404, r#"{"error":"no such route","code":"not_found"}"#.to_string()),
            };
            let head = format!("HTTP/1.1 {status} x\r\nContent-Type: application/json\r\n");
            let resp = format!(
                "{head}Content-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
    });
    TopupMock { addr, captured }
}

/// Read one HTTP request (request line + headers + Content-Length body).
fn read_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
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
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut content_length = 0usize;
    for l in lines {
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
    Some((path, method, body))
}

// ── the hermetic CLI runner (the kat_refill_cli pattern) ──────────────────

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_refine_topup_cli_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn heal(dir: &Path, url: &str, extra_env: &[(&str, &str)], args: &[&str]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-refine"));
    cmd.current_dir(dir)
        .env("RIIR_REFINE_KAT_ACCOUNT_DIR", dir)
        .envs(hermetic_home::vars(dir))
        .env("RIIR_AUTH_ACCOUNT_DIR", dir)
        .env("RIIR_REFINE_TRAJ_ROOT", dir)
        .env("RIIR_REFINE_KAT_SERVICE_URL", url)
        .env("RIIR_REFINE_KAT_LEASE", "0")
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
    hermetic_home::assert_minted(&text);
}

fn hex_bytes(h: &str) -> Vec<u8> {
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).expect("hex"))
        .collect()
}

/// Whether `text` carries a rendered QR — the dense render's glyphs are
/// the Unicode block elements (▀ U+2580 / ▄ U+2584 / █ U+2588, the
/// Dense1x2 packing: two modules per character).
fn has_qr(text: &str) -> bool {
    text.chars()
        .any(|c| matches!(c, '\u{2580}' | '\u{2584}' | '\u{2588}'))
}

/// Phase 1: no account, no dial — the honest local refusal.
#[test]
fn topup_without_an_account_refuses_before_dialing() {
    let dir = scratch("noacct");
    let (code, text) = heal(&dir, CLOSED_PORT, &[], &["topup", "--pack", "1"]);
    assert_eq!(code, Some(1), "the topup must refuse. output:\n{text}");
    assert!(text.contains("no account"), "the reason: {text}");
    assert!(text.contains("cargo refine login"), "the remedy: {text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 2: catalog renders the SERVER's numbers, the intent carries the
/// URL + reference, and the POST body is the SIGNED wire whose signature
/// verifies against the canonical composer (the two-sided pin).
#[test]
fn topup_happy_path_signs_the_canonical_message_and_renders_the_server_numbers() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("happy");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["topup", "--pack", "1"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    // The catalog lines render server numbers (never a local price table).
    assert!(text.contains("--pack 1"), "{text}");
    assert!(text.contains("10.000000 KAT"), "{text}");
    assert!(text.contains("200.000000 KAT"), "{text}");
    assert!(text.contains("2 open per account"), "{text}");
    // The trust line (riir-kat Issue 006): the payer sees WHO is paid and
    // WHICH USDC mint the invoice names before scanning the QR.
    assert!(
        text.contains("pays gistXmdXerQ8oWMpdxXhva65pgkAdteQJwyGLGRe5zN"),
        "{text}"
    );
    assert!(
        text.contains("USDC mint 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"),
        "{text}"
    );
    // The intent lines.
    assert!(text.contains("solana:pay?example"), "{text}");
    assert!(text.contains("3RefX7"), "{text}");
    assert!(text.contains("account --history"), "{text}");
    // The QR default is AUTO: a piped (non-TTY) run renders none — agents
    // never get a block of unicode in their logs.
    assert!(!has_qr(&text), "piped run must not render the QR: {text}");
    // The dials: catalog GET, then the T10 attested-intent PRE-FLIGHT (a
    // `GET /` service-info read — riir-kat 23d551b landed it 2026-10-08,
    // two days after this suite; the mock answers 404 and the
    // local-intent dev-rig allowance proceeds), then the intent POST.
    let caps = mock.captured();
    assert_eq!(caps.len(), 3, "{caps:?}");
    assert_eq!(caps[0].method, "GET");
    assert!(caps[0].path.starts_with("/pay/catalog"));
    assert_eq!(caps[1].method, "GET", "the T10 pre-flight: {caps:?}");
    assert_eq!(
        caps[1].path, "/",
        "the service-info attestation read: {caps:?}"
    );
    assert_eq!(caps[2].method, "POST");
    assert!(caps[2].path.starts_with("/pay/intent"));
    // THE WIRE PIN: the body parses, and the signature verifies against
    // the CANONICAL composer for the pack the CLI was asked for.
    let v: serde_json::Value = serde_json::from_str(&caps[2].body).expect("the signed body");
    assert_eq!(v["pack_id"], 1);
    let pk: [u8; 32] = hex_bytes(v["pubkey_hex"].as_str().unwrap())
        .try_into()
        .ok()
        .unwrap();
    let sig: [u8; 64] = hex_bytes(v["signature_hex"].as_str().unwrap())
        .try_into()
        .ok()
        .unwrap();
    let msg = riir_refine::kat_protocol_pay::pay_intent_message(&pk, 1);
    use ed25519_dalek::Verifier as _;
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk).expect("the key");
    assert!(
        vk.verify(&msg, &ed25519_dalek::Signature::from_bytes(&sig))
            .is_ok(),
        "the POST signature must verify against the canonical composer"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 2b: `--qr` forces the checkout QR even piped — the QR is the URL
/// (the same string), rendered by the shared lease-client renderer, with
/// the wallet pointer line under it.
#[test]
fn topup_qr_flag_renders_the_checkout_qr_even_piped() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("qr");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["topup", "--pack", "1", "--qr"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("solana:pay?example"), "the URL line: {text}");
    assert!(has_qr(&text), "the QR must render (dense): {text}");
    assert!(
        text.contains("Solana Pay wallet"),
        "the pointer line: {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 2c: `--qr --no-qr` is no defensible resolution — exit 2 before
/// ANY dial (the closed port proves nothing reached the wire).
#[test]
fn topup_qr_no_qr_conflict_refuses_before_any_dial() {
    let dir = scratch("qrconflict");
    login(&dir);
    let (code, text) = heal(
        &dir,
        CLOSED_PORT,
        &[],
        &["topup", "--pack", "1", "--qr", "--no-qr"],
    );
    assert_eq!(code, Some(2), "output:\n{text}");
    assert!(text.contains("conflict"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 3: not armed on this environment — the honest posture, never a
/// retry, and it names the page.
#[test]
fn topup_unarmed_env_is_an_honest_refusal() {
    let mock = spawn_topup_mock("unarmed");
    let dir = scratch("unarmed");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["topup", "--pack", "1"]);
    assert_eq!(code, Some(1), "output:\n{text}");
    assert!(text.contains("not armed on"), "{text}");
    assert!(text.contains("/pay"), "the page must be named: {text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 4: no --pack on a non-TTY run — the loud refusal, ONE catalog
/// dial (agents never guess a price).
#[test]
fn topup_without_a_pack_refuses_loud_in_a_non_tty_run() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("nopack");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["topup"]);
    assert_eq!(code, Some(2), "output:\n{text}");
    assert!(text.contains("--pack"), "{text}");
    let caps = mock.captured();
    assert_eq!(caps.len(), 1, "the catalog only — no intent POST: {caps:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 5: the read half — `account --history` renders the purchases
/// section from the additive page (open → `—`; settled → the credit).
#[test]
fn account_history_renders_the_purchases_section() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("history");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["account", "--history"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("purchases (2"), "{text}");
    assert!(text.contains("open"), "{text}");
    assert!(text.contains("settled"), "{text}");
    assert!(text.contains("10.000000 KAT"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The parse never invents: a pre-B7 history (no purchases fields)
/// renders NO purchases section (the additive degradation).
#[test]
fn account_history_without_purchases_renders_no_section() {
    // Unit-level: the route shape with the fields stripped parses as the
    // empty page (the same pin the worker contract carries server-side).
    let stripped = r#"{"account_id":"ab","balance_micro":1,"granted":false,"contributed":false,"burn":null,"first_commitment_hex":null,"cegel_balance_micro":0,"seal_balance_micro":0,"receipts":[],"receipts_total_matching":0,"receipts_truncated":false,"events":[],"events_total":0,"events_truncated":false}"#;
    let h: ServiceHistory = {
        let v: serde_json::Value = serde_json::from_str(stripped).unwrap();
        let (_rows, total, truncated) =
            riir_refine::kat_protocol_pay::parse_purchases_page(&v).expect("absent is empty");
        assert_eq!((total, truncated), (0, false));
        riir_refine::kat_protocol_account::parse_history(stripped.as_bytes()).expect("parses")
    };
    assert!(h.purchases.is_empty());
    let lines = riir_refine::kat_account_client::account_history_lines(&h, 0, 0);
    assert!(
        !lines.iter().any(|l| l.contains("purchases")),
        "no section on a pre-B7 body: {lines:?}"
    );
}

// ── issue 126: the void / show actions + the 429 auto-show ──────────────

/// `--show <seq>` re-prints the checkout: the SAME intent lines + QR the
/// create printed (the server recomposes; the client renders). The mock
/// answers the canned intent body, so the URL line must match exactly.
#[test]
fn topup_show_reprints_the_checkout() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("show");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["topup", "--show", "7", "--qr"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("solana:pay?example"), "{text}");
    assert!(text.contains("3RefX7"), "{text}");
    // The server's own deadline (riir-kat Issue 006) — the mock's fixed
    // future unix, rendered relative with the clock known.
    assert!(text.contains("(unix 1800062400)"), "{text}");
    assert!(has_qr(&text), "the --qr forced QR: {text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--void <seq>` prints the release line (the mock 200s the action).
#[test]
fn topup_void_prints_the_release() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("void");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["topup", "--void", "7"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("voided"), "{text}");
    assert!(text.contains("slot is free"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The 429 auto-show: the intent route refuses `rate_limited` (the mock's
/// `capped` mode), the CLI reads `/history`, finds exactly ONE open
/// checkout, and SHOWS it without asking — the owner's exact ask. The
/// open row in the canned history is seq 7.
#[test]
fn topup_capped_with_one_open_intent_shows_it_without_asking() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let cap = r#"{"error":"pay: 2 intents already open for this account — settle or wait","code":"rate_limited"}"#.to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let Some((path, _, _)) = read_request(&mut stream) else {
                continue;
            };
            let route = path.split('?').next().unwrap_or(&path);
            let (status, reply) = match route {
                "/pay/catalog" => (200, CATALOG_BODY.to_string()),
                "/pay/intent" => (429, cap.clone()),
                "/history" => (
                    200,
                    format!(
                        r#"{{"account_id":"{}","balance_micro":0,"granted":false,"contributed":false,"burn":null,"first_commitment_hex":null,"cegel_balance_micro":0,"seal_balance_micro":0,"receipts":[],"receipts_total_matching":0,"receipts_truncated":false,"events":[],"events_total":0,"events_truncated":false,"purchases":[{{"seq":7,"pack_id":1,"status":"open","usdc_base":1000000,"paid_base":0,"kat_micro":0,"created_unix":10,"settled_unix":0,"reference":"3RefX7"}}],"purchases_total":1,"purchases_truncated":false}}"#,
                        "a".repeat(32)
                    ),
                ),
                "/pay/intent/show" => (
                    200,
                    r#"{"seq":7,"pack_id":1,"usdc_base":1000000,"kat_micro":10000000,"reference":"3RefX7","url":"solana:pay?example","expires_unix":1800062400,"status":"open"}"#
                        .to_string(),
                ),
                _ => (404, r#"{"error":"no such route","code":"not_found"}"#.to_string()),
            };
            let resp = format!(
                "HTTP/1.1 {status} x\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
    });
    let mock_base = format!("http://{addr}");
    let dir = scratch("autoshow");
    login(&dir);
    let (code, text) = heal(&dir, &mock_base, &[], &["topup", "--pack", "1", "--qr"]);
    assert_eq!(
        code,
        Some(0),
        "the auto-show SUCCEEDS (exit 0, a shown checkout): output:\n{text}"
    );
    assert!(
        text.contains("already have an open checkout (seq 7)"),
        "{text}"
    );
    assert!(text.contains("solana:pay?example"), "{text}");
    assert!(has_qr(&text), "the shown checkout's QR: {text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Issue 126 follow-up: `--pick latest` is the agent-safe non-interactive
/// path — on the cap with SEVERAL open checkouts it shows the NEWEST one
/// (the server's first row) and never enters the interactive picker.
#[test]
fn pick_latest_shows_the_newest_without_asking() {
    let mock = spawn_topup_mock("capped2");
    let dir = scratch("picklatest");
    login(&dir);
    let (code, text) = heal(
        &dir,
        &mock.base(),
        &[],
        &["topup", "--pack", "1", "--no-qr", "--pick", "latest"],
    );
    assert_eq!(code, Some(0), "exit 0 (a shown checkout):\n{text}");
    assert!(
        text.contains("--pick latest — showing the newest checkout (seq 8)"),
        "{text}"
    );
    // The show action fired (the seq rides the SIGNED body — binary — so
    // the pick proof is the printed line above plus the route hit; the
    // mock's canned reply is seq 7, never echoed as the pick).
    assert!(
        mock.captured().iter().any(|c| c.path == "/pay/intent/show"),
        "the show route was asked: {:?}",
        mock.captured()
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>()
    );
    assert!(text.contains("solana:pay?example"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The buy friction-cut: the amount resolves the catalog pack
/// (`buy 10 kat` = pack 1) and the whole topup flow rides along.
#[test]
fn buy_resolves_the_pack_from_the_amount() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("buy");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["buy", "10", "kat"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("buying pack 1"), "{text}");
    assert!(text.contains("solana:pay?example"), "{text}");
    // The preset service-url env is an explicit operator intent and wins
    // over the mainnet default (the hermetic-test law).
    assert!(text.contains("buy honors the preset"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The glued `10kat` form parses like the two-word one.
#[test]
fn buy_accepts_the_glued_unit_form() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("buyglued");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["buy", "1usdc"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("buying pack 1"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Nobody guesses a price: an amount no pack sells exactly refuses with
/// the rendered catalog.
#[test]
fn buy_refuses_an_amount_no_pack_matches() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("buymiss");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["buy", "100", "kat"]);
    assert_eq!(code, Some(2), "output:\n{text}");
    assert!(
        text.contains("no pack sells exactly 100.000000 KAT"),
        "{text}"
    );
    // The signed intent was never attempted.
    assert!(
        !mock.captured().iter().any(|c| c.path == "/pay/intent"),
        "no intent POST on a pack miss"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `buy` rides the topup 429 surface too — --pick latest composes with it
/// (the agent story end to end: one command, no prompting, a QR/URL).
#[test]
fn buy_pick_latest_composes_on_the_cap() {
    let mock = spawn_topup_mock("capped2");
    let dir = scratch("buycapped");
    login(&dir);
    let (code, text) = heal(
        &dir,
        &mock.base(),
        &[],
        &["buy", "10", "kat", "--pick", "latest"],
    );
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(
        text.contains("--pick latest — showing the newest checkout (seq 8)"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The value verbs (issue 126 follow-up): `kat` renders the one-line
/// host-labelled balance, `tuna` renders the balance + expiry window.
#[test]
fn kat_and_tuna_render_the_one_line_values() {
    let mock = spawn_topup_mock("ok");
    let dir = scratch("values");
    login(&dir);
    let (code, text) = heal(&dir, &mock.base(), &[], &["kat"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("8.000000 KAT — "), "{text}");
    assert!(text.contains("127.0.0.1"), "host-labelled: {text}");
    let (code, text) = heal(&dir, &mock.base(), &[], &["tuna"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("47.200000 TUNA"), "{text}");
    assert!(text.contains("expires in"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The dialing law (issue 126 follow-up 3, owner 2026-10-14: "all should
/// target mainnet without any flag, flag is for devnet"): a STORED config
/// row never steers the dial. Two mocks: the row names B, the env names
/// A — the command answers from A and B captures nothing. (A stale row is
/// exactly how bare commands dialed devnet before the law.)
#[test]
fn a_stored_config_row_never_steers_the_dial() {
    let dial = spawn_topup_mock("ok");
    let decoy = spawn_topup_mock("ok");
    let dir = scratch("configlaw");
    login(&dir);
    // The stale row: written through the SAME writer `config set` uses,
    // into the scratch account dir the child reads via the env override.
    riir_refine::kat_service_config::store(&dir, &decoy.base())
        .expect("the bookkeeping row stores");
    // The env names the dial; the row names the decoy. The dial must win.
    let (code, text) = heal(&dir, &dial.base(), &[], &["kat"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("8.000000 KAT"), "{text}");
    assert!(
        decoy.captured().is_empty(),
        "the decoy captured nothing: {:?}",
        decoy
            .captured()
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        dial.captured()
            .iter()
            .any(|c| c.path.starts_with("/history")),
        "the dial mock served the read"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The action flags' combination law: --show + --void together is a
/// refusal BEFORE any dial (the closed port proves nothing reached the
/// wire), and both require the topup command.
#[test]
fn topup_show_void_flags_validate_before_any_dial() {
    let dir = scratch("flaglaw");
    login(&dir);
    let (code, text) = heal(
        &dir,
        CLOSED_PORT,
        &[],
        &["topup", "--show", "7", "--void", "7"],
    );
    assert_eq!(code, Some(2), "{text}");
    assert!(text.contains("separate actions"), "{text}");
    // Outside topup: refused at parse.
    let (code, text) = heal(
        &dir,
        CLOSED_PORT,
        &[],
        &["account", "--history", "--show", "7"],
    );
    assert_eq!(code, Some(2), "{text}");
    assert!(text.contains("--show/--void require"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}
