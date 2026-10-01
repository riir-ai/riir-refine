//! KAT staking CLI client — the T2.1 fixtures (Proposal 007, Plan 041),
//! driven against a **local std-TCP mock HTTP server** (the lease-client g10
//! precedent — no network, no deps).
//!
//! What this binary pins:
//!
//! - **The three wire shapes** — `POST /stake/request` (the account + amount
//!   request the service answers with a composed transfer URL), `GET
//!   /stake/info?acct=` (the position fetch the renderer consumes), `POST
//!   /stake/unstake` (the drain request + the refusal/error envelope).
//! - **The service-first flows end-to-end** — the client uses the
//!   service-composed URL when the plane answers, and falls back to the
//!   LOCAL composition + intent queue when it does not (the honest
//!   not-live paths, `StakeOutcome::LocalQueued` / the unstake queue).
//! - **The caveat-#15 copy gate on every render the flows produce** (the
//!   `kat_pending_estimator::no_dollar_sign_in_any_render` precedent,
//!   extended: no `$` beside a KAT accrual; accruals labeled "service
//!   credit"; APY only inside the no-promise phrase; the four unstake
//!   disclosures present). Driven through the module's ONE shared
//!   predicate (`caveat15_violation`) + the D3 required-disclosure
//!   checklist (`caveat15_missing_disclosure`) — the canaries live in the
//!   module tests.
//! - **The G2-S render budget smoke** — the position render completes well
//!   inside the 50 ms CLI gate.

#![cfg(feature = "kat_transport")]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use riir_refine::kat_lease_client::HttpResponse;
use riir_refine::kat_stake_client::{self, StakeInfo, StakeOutcome, StakeTransport, TrancheView};

// ── the mock server (std TCP; the g10 shape) ───────────────────────────────

/// What the mock serves for `/stake/info` (default: a fixed two-tranche
/// position; tests may override by pushing a custom body).
struct MockServer {
    addr: SocketAddr,
    captured: Arc<Mutex<Vec<CapturedRequest>>>,
    _handle: std::thread::JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct CapturedRequest {
    method: String,
    path: String,
    body_prefix: Option<String>,
}

impl MockServer {
    fn captured(&self) -> Vec<CapturedRequest> {
        self.captured.lock().unwrap().clone()
    }
    fn base(&self) -> String {
        format!("http://{}", self.addr)
    }
}

#[derive(Clone)]
enum ServeMode {
    /// Answer every endpoint (info body from the fixture).
    Live,
    /// Answer nothing correctly (404s) — every flow falls back.
    Down,
}

fn spawn_stake_mock(mode: ServeMode, info_body: String) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let captured: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let cap = Arc::clone(&captured);
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let req = match read_request(&mut stream) {
                Some(r) => r,
                None => continue,
            };
            cap.lock().unwrap().push(req.clone());
            let resp = route(&mode, &info_body, &req);
            let _ = stream.write_all(&resp);
            let _ = stream.flush();
        }
    });
    MockServer {
        addr,
        captured,
        _handle: handle,
    }
}

fn route(mode: &ServeMode, info_body: &str, req: &CapturedRequest) -> Vec<u8> {
    let path_only = req.path.split('?').next().unwrap_or("");
    match (req.method.as_str(), path_only, mode) {
        ("POST", "/stake/request", ServeMode::Live) => {
            let payee = kat_stake_client::b58_encode(&[0x11; 32]);
            let mint = kat_stake_client::b58_encode(&[0x22; 32]);
            let reference = kat_stake_client::b58_encode(&[0x33; 32]);
            let body = format!(
                r#"{{"url":"solana:{payee}?amount=100.000000&spl-token={mint}&reference={reference}","reference":"{reference}"}}"#
            );
            http(200, body.as_bytes())
        }
        ("GET", "/stake/info", _) if !matches!(mode, ServeMode::Down) => {
            http(200, info_body.as_bytes())
        }
        ("POST", "/stake/unstake", ServeMode::Live) => {
            http(200, br#"{"queued":true,"amount_base":100000000}"#)
        }
        _ => http(404, b"not found"),
    }
}

fn http(status: u16, body: &[u8]) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        _ => "",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut out = head.into_bytes();
    out.extend_from_slice(body);
    out
}

fn read_request(stream: &mut TcpStream) -> Option<CapturedRequest> {
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
    let request_line = lines.next().unwrap_or_default().to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut content_length = 0usize;
    for line in lines {
        if let Some(v) = line.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body_prefix = None;
    if content_length > 0 {
        while buf.len() < header_end + 4 + content_length {
            let n = stream.read(&mut tmp).ok()?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        let body = &buf[header_end + 4..];
        body_prefix = Some(String::from_utf8_lossy(&body[..body.len().min(512)]).into_owned());
    }
    Some(CapturedRequest {
        method,
        path,
        body_prefix,
    })
}

// ── fixtures ───────────────────────────────────────────────────────────────

/// The env-mutating tests serialize on this lock — `RIIR_HEAL_KAT_STAKE_PAYEE`
/// is process-global and the default test runner is parallel (the
/// `tests/hero_fusion_ab.rs` AB_LOCK precedent for process-global config).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir-clippy-stake-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn info_fixture() -> StakeInfo {
    StakeInfo {
        account_id: "9f3a".to_string(),
        staked_base: 100_000_000,
        accrued_kat_micro: 2_431_500_000,
        restake: true,
        tranches: vec![
            TrancheView {
                seq: 0,
                kind: "usdc".into(),
                amount_base: 60_000_000,
                unlock_at_tick: 100_000 + 86_400 * 3,
            },
            TrancheView {
                seq: 1,
                kind: "usdc".into(),
                amount_base: 40_000_000,
                unlock_at_tick: 100_000 + 86_400 * 4,
            },
        ],
        pool: kat_stake_client::PoolView {
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
        // Proposal 017 additive fields — the fixture renders the pre-017
        // shape (all None); the buffered render has its own arm below.
        restake_posture: None,
        op_seq: None,
        buffer: None,
        nudge: None,
    }
}

/// A mock implementing the transport trait over captured std-TCP requests —
/// drives the REAL `HttpStakeTransport` (ureq) against the listener so the
/// wire shape assertions cover the production client.
struct MockStakeTransport {
    server: MockServer,
}

impl StakeTransport for MockStakeTransport {
    fn post_stake_request(&self, json: &str) -> HttpResponse {
        let t = transport_at(&self.server.base());
        t.post_stake_request(json)
    }
    fn get_stake_info(&self, account_id: &str) -> HttpResponse {
        let t = transport_at(&self.server.base());
        t.get_stake_info(account_id)
    }
    fn post_unstake(&self, json: &str) -> HttpResponse {
        let t = transport_at(&self.server.base());
        t.post_unstake(json)
    }
    fn post_restake(&self, json: &str) -> HttpResponse {
        let t = transport_at(&self.server.base());
        t.post_restake(json)
    }
    fn post_redeem(&self, json: &str) -> HttpResponse {
        let t = transport_at(&self.server.base());
        t.post_redeem(json)
    }
}

fn transport_at(base: &str) -> riir_refine::kat_stake_client::HttpStakeTransport {
    riir_refine::kat_stake_client::HttpStakeTransport::new(base, 5)
}

// ── the wire-shape + flow fixtures ─────────────────────────────────────────

#[test]
fn stake_request_posts_the_composed_shape_and_uses_the_service_url() {
    let server = spawn_stake_mock(
        ServeMode::Live,
        serde_json::to_string(&info_fixture()).unwrap(),
    );
    let transport = MockStakeTransport { server };
    let root = temp_dir("stake-req");
    let out = kat_stake_client::stake_request_flow(&transport, "acct42", 100_000_000, &root, 1)
        .expect("live flow");
    // The service-composed URL is used verbatim.
    match out {
        StakeOutcome::ServiceUrl(url) => {
            assert!(url.starts_with("solana:"), "{url}");
            assert!(url.contains("amount=100.000000"), "{url}");
            assert!(url.contains("spl-token="), "{url}");
            assert!(url.contains("reference="), "{url}");
        }
        other => panic!("expected ServiceUrl, got {other:?}"),
    }
    // The wire shape: POST /stake/request with the versioned account+amount.
    let caps = transport.server.captured();
    let req = caps
        .iter()
        .find(|c| c.method == "POST" && c.path == "/stake/request")
        .expect("stake request posted");
    let body = req.body_prefix.as_deref().unwrap_or_default();
    assert!(body.contains(r#""stake_version":1"#), "{body}");
    assert!(body.contains(r#""account_id":"acct42""#), "{body}");
    assert!(body.contains(r#""amount_base":100000000"#), "{body}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn stake_not_live_composes_locally_and_queues_the_intent() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Down: every endpoint 404s → the local fallback.
    let server = spawn_stake_mock(ServeMode::Down, String::new());
    let transport = MockStakeTransport { server };
    let root = temp_dir("stake-local");
    // The local composition needs the custody payee env.
    let env_payee = kat_stake_client::b58_encode(&[0x55; 32]);
    std::env::set_var("RIIR_HEAL_KAT_STAKE_PAYEE", &env_payee);
    let out = kat_stake_client::stake_request_flow(&transport, "acct42", 250_000_000, &root, 1)
        .expect("local fallback");
    match out {
        StakeOutcome::LocalQueued { url, queued_at } => {
            assert!(url.starts_with("solana:"), "{url}");
            // The env payee + the 6-decimal amount + a fresh reference.
            assert!(url.contains(&env_payee), "{url}");
            assert!(url.contains("amount=250.000000"), "{url}");
            assert!(url.contains("reference="), "{url}");
            assert!(
                queued_at.exists(),
                "intent queued at {}",
                queued_at.display()
            );
            assert_eq!(
                kat_stake_client::queued_stake_count(&root),
                Some(1),
                "exactly one queued intent"
            );
        }
        other => panic!("expected LocalQueued, got {other:?}"),
    }
    // A SECOND not-live stake queues a SECOND intent (fresh reference, no
    // dedupe — the service dedupes at claim, not the queue).
    let _ = kat_stake_client::stake_request_flow(&transport, "acct42", 100_000_000, &root, 2)
        .expect("second local fallback");
    assert_eq!(kat_stake_client::queued_stake_count(&root), Some(2));
    std::env::remove_var("RIIR_HEAL_KAT_STAKE_PAYEE");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn stake_not_live_without_payee_config_is_an_honest_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let server = spawn_stake_mock(ServeMode::Down, String::new());
    let transport = MockStakeTransport { server };
    let root = temp_dir("stake-nopayee");
    std::env::remove_var("RIIR_HEAL_KAT_STAKE_PAYEE");
    let err = kat_stake_client::stake_request_flow(&transport, "a", 100, &root, 1)
        .expect_err("no payee → honest error");
    assert!(err.contains("RIIR_HEAL_KAT_STAKE_PAYEE"), "{err}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn info_fetch_gets_the_account_query_and_the_flow_renders_the_mock_shape() {
    let info = info_fixture();
    let server = spawn_stake_mock(ServeMode::Live, serde_json::to_string(&info).unwrap());
    let transport = MockStakeTransport { server };
    let HttpResponse::Ok(body) = transport.get_stake_info("9f3a") else {
        panic!("info fetch must answer 200");
    };
    let parsed: StakeInfo = serde_json::from_slice(&body).expect("the service JSON decodes");
    let text = kat_stake_client::render_info(&parsed, 100_000);
    // The mock's rows (per-tranche unlock, the pool split, the restake flag).
    assert!(
        text.contains("Staked:          100.00 USDC (refundable)"),
        "{text}"
    );
    assert!(text.contains("in 3d 00h   (tranche #0"), "{text}");
    assert!(text.contains("in 4d 00h   (tranche #1"), "{text}");
    assert!(
        text.contains("+2431.500000 KAT accrued — service credit"),
        "{text}"
    );
    assert!(text.contains("mining 90% · staking 10%"), "{text}");
    assert!(text.contains("Reward restake:  ON"), "{text}");
    // The wire shape: the account rides the query string.
    let caps = transport.server.captured();
    assert!(
        caps.iter()
            .any(|c| c.method == "GET" && c.path == "/stake/info?acct=9f3a"),
        "the account query shape: {caps:?}"
    );
}

#[test]
fn unstake_posts_the_drain_request_and_renders_the_disclosure() {
    let server = spawn_stake_mock(
        ServeMode::Live,
        serde_json::to_string(&info_fixture()).unwrap(),
    );
    let transport = MockStakeTransport { server };
    let root = temp_dir("unstake");
    let out = kat_stake_client::unstake_flow(
        &transport,
        &ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]),
        "acct42",
        &root,
        1,
    )
    .expect("live unstake");
    // The service-queued arm: the amount + the full disclosure.
    assert!(
        out.contains("unstake: queued by the service (100.00 USDC)"),
        "{out}"
    );
    assert!(out.contains("48h escrow"), "{out}");
    assert!(out.contains("vent in equal daily slices"), "{out}");
    // The wire shape.
    let caps = transport.server.captured();
    let req = caps
        .iter()
        .find(|c| c.method == "POST" && c.path == "/stake/unstake")
        .expect("unstake posted");
    let body = req.body_prefix.as_deref().unwrap_or_default();
    assert!(body.contains(r#""account_id":"acct42""#), "{body}");
    assert!(body.contains(r#""stake_version":1"#), "{body}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn unstake_not_live_queues_and_still_discloses() {
    let server = spawn_stake_mock(ServeMode::Down, String::new());
    let transport = MockStakeTransport { server };
    let root = temp_dir("unstake-local");
    let out = kat_stake_client::unstake_flow(
        &transport,
        &ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]),
        "acct42",
        &root,
        1,
    )
    .expect("not-live unstake queues");
    assert!(
        out.contains("service plane not yet live — intent queued locally"),
        "{out}"
    );
    // All four disclosures present on the not-live path too.
    assert!(out.contains("48h escrow"), "{out}");
    assert!(out.contains("vent in equal daily slices"), "{out}");
    assert!(
        out.contains("Refund returns to the funding wallet"),
        "{out}"
    );
    assert!(
        out.contains("Incentive accrual stopped at request"),
        "{out}"
    );
    // The queue file exists (the unstake intent, distinct from stake intents).
    assert!(root
        .join(".refine")
        .join("stake")
        .join("unstake_pending.jsonl")
        .exists());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn unstake_refusal_shape_renders_remaining_time() {
    // InfoOnly answers info but 404s unstake — use a Live variant that
    // returns the refusal envelope. Build a one-off mock by serving the
    // refusal for /stake/unstake: reuse Down for everything except we
    // assert the refusal DECODE path via the flow's error envelope parsing.
    // Simplest honest pin: the Down mock's fallback already covers the
    // not-live arm; here we pin the refusal PARSE with a bespoke body.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let Some(req) = read_request(&mut stream) else {
                continue;
            };
            let resp = if req.method == "POST" && req.path == "/stake/unstake" {
                http(
                    200,
                    br#"{"error":"LockupNotExpired","remaining_ticks":172800}"#,
                )
            } else {
                http(404, b"not found")
            };
            let _ = stream.write_all(&resp);
            let _ = stream.flush();
        }
    });
    let transport = transport_at(&format!("http://{addr}"));
    let root = temp_dir("unstake-refuse");
    let out = kat_stake_client::unstake_flow(
        &transport,
        &ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]),
        "acct42",
        &root,
        1,
    )
    .expect("refusal renders, not errors");
    assert!(out.contains("refused — lockup not expired"), "{out}");
    assert!(out.contains("2d 00h"), "{out}");
    std::fs::remove_dir_all(&root).ok();
    drop(handle);
}

// ── the caveat-#15 copy gate over every render the flows produce ───────────

#[test]
fn copy_gate_no_dollar_beside_a_kat_accrual_in_any_flow_render() {
    let info = info_fixture();
    let renders: HashMap<String, String> = HashMap::from([
        (
            "stake_line".into(),
            kat_stake_client::stake_line(100_000_000),
        ),
        ("info".into(), kat_stake_client::render_info(&info, 100_000)),
        ("unstake".into(), kat_stake_client::unstake_disclosure()),
    ]);
    // ONE shared predicate with the module gate (the two hand-copied loops
    // this replaces could drift apart); the integration half adds the FLOW
    // coverage — every render the live/not-live flows can produce.
    for (name, text) in &renders {
        assert_eq!(
            kat_stake_client::caveat15_violation(text),
            None,
            "caveat #15 ({name}): {text}"
        );
    }
    // The D3 pinned copy set rides the same shared checklist.
    kat_stake_client::caveat15_missing_disclosure(&renders["stake_line"], &renders["info"])
        .expect("the flow renders carry every required disclosure");
}

// ── the G2-S render budget smoke ───────────────────────────────────────────

#[test]
fn g2s_render_completes_well_inside_the_cli_budget() {
    let info = info_fixture();
    let start = std::time::Instant::now();
    let _ = kat_stake_client::render_info(&info, 100_000);
    let _ = kat_stake_client::stake_line(100_000_000);
    let _ = kat_stake_client::unstake_disclosure();
    let elapsed = start.elapsed();
    // G2-S: "CLI render < 50 ms" — a string render is µs-class; the pin is
    // generous (debug-mode, CI noise) while still catching a pathological
    // regression (e.g. an accidental per-line allocation storm).
    assert!(elapsed.as_millis() < 50, "render took {elapsed:?}");
}
