//! KAT governance voting CLI client — the T2.4 fixtures (Proposal 008,
//! Plan 012), driven against a **local std-TCP mock HTTP server** (the
//! stake g2s precedent — no network, no deps).
//!
//! What this binary pins (the CLI half of the G9-GOV gate):
//!
//! - **The wire shapes** — `GET /gov/proposal?id=&acct=` (the view the
//!   weight+evidence render consumes) + `POST /gov/vote` (the signed vote
//!   request: account, proposal, choice, pubkey, signature — all full-width
//!   hex).
//! - **The service-first flow end-to-end** — the vote renders the
//!   service-served proposal (weight + evidence lead), signs with the
//!   account key, and submits; the service-echoed ack renders.
//! - **The not-live fallback** — the SIGNED intent queues locally
//!   (`.heal/vote/pending.jsonl`), honestly rendered.
//! - **The cross-repo golden** — `vote_message` hex IDENTICAL to the
//!   dapps-side pin (the mirror's drift defense).
//! - **The copy gates** — no `$`, membership-only framing, the
//!   constitutional never-moves-funds line on every screen.

#![cfg(feature = "kat_transport")]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use riir_clippy::kat_lease_client::HttpResponse;
use riir_clippy::kat_vote_client::{
    render_proposal, sign_vote, vote_confirm_line, vote_flow, vote_message, ProposalView, VoteAck,
    VoteChoice, VoteOutcome, VoteTransport,
};

// ── the mock server (std TCP; the stake g2s shape) ───────────────────────

struct MockServer {
    addr: SocketAddr,
    captured: Arc<Mutex<Vec<CapturedRequest>>>,
    _handle: std::thread::JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct CapturedRequest {
    method: String,
    path: String,
    body: Option<String>,
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
    /// Answer both endpoints (the proposal body from the fixture).
    Live,
    /// 404 everything — the flow falls back to the local queue.
    Down,
}

fn spawn_vote_mock(mode: ServeMode, proposal_body: String) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let captured: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let cap = Arc::clone(&captured);
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let Some(req) = read_request(&mut stream) else {
                continue;
            };
            cap.lock().unwrap().push(req.clone());
            let resp = route(&mode, &proposal_body, &req);
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

fn route(mode: &ServeMode, proposal_body: &str, req: &CapturedRequest) -> Vec<u8> {
    let path_only = req.path.split('?').next().unwrap_or("");
    match (req.method.as_str(), path_only, mode) {
        ("GET", "/gov/proposal", ServeMode::Live) => http(200, proposal_body.as_bytes()),
        ("POST", "/gov/vote", ServeMode::Live) => http(
            200,
            br#"{"proposal_id":7,"choice":"yes","snapshot_weight":5000}"#,
        ),
        _ => http(404, b"not found"),
    }
}

fn http(status: u16, body: &[u8]) -> Vec<u8> {
    let reason = if status == 200 { "OK" } else { "" };
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
    let mut body = None;
    if content_length > 0 {
        while buf.len() < header_end + 4 + content_length {
            let n = stream.read(&mut tmp).ok()?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        body = Some(String::from_utf8_lossy(&buf[header_end + 4..]).into_owned());
    }
    Some(CapturedRequest { method, path, body })
}

/// The HTTP transport driving the mock (the vote client's own impl over the
/// mock's base URL — the SAME code the CLI uses).
struct MockVoteTransport {
    inner: riir_clippy::kat_vote_client::HttpVoteTransport,
}

impl VoteTransport for MockVoteTransport {
    fn get_proposal(&self, proposal_id: u64, account_id: &str) -> HttpResponse {
        self.inner.get_proposal(proposal_id, account_id)
    }
    fn post_vote(&self, json: &str) -> HttpResponse {
        self.inner.post_vote(json)
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────

fn proposal_body() -> String {
    let v = ProposalView {
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
    };
    serde_json::to_string(&v).unwrap()
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kat_vote_g9_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Hex → fixed-width bytes (the test-side decoder for the wire fields).
fn hex_fixed<const N: usize>(s: &str) -> [u8; N] {
    let mut out = [0u8; N];
    for (i, b) in s.as_bytes().chunks(2).take(N).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(b).unwrap_or("00"), 16).unwrap_or(0);
    }
    out
}

// ── the wire + the flow ───────────────────────────────────────────────────

/// THE cross-repo golden — byte-identical to the dapps-side pin
/// (`g9_vote_message_golden_hex`): a drift in EITHER mirror fails a gate.
#[test]
fn vote_message_hex_is_the_dapps_pinned_golden() {
    let msg = vote_message(1, &[0x42; 32], VoteChoice::Yes);
    let hex = riir_clippy::kat_vote_client::hex_encode(&msg);
    assert_eq!(
        hex,
        "726969722d64617070732e6b61742e676f766f74652e7631\
         0100000000000000\
         4242424242424242424242424242424242424242424242424242424242424242\
         01"
        .replace(' ', "")
    );
}

/// The service-first flow: the proposal renders (weight + evidence), the
/// signed vote submits, and the REQUEST carries the full-width pubkey +
/// signature hex + the exact fields the ledger verifies.
#[test]
fn service_first_flow_submits_the_signed_vote() {
    let server = spawn_vote_mock(ServeMode::Live, proposal_body());
    let transport = MockVoteTransport {
        inner: riir_clippy::kat_vote_client::HttpVoteTransport::new(&server.base(), 5),
    };
    let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
    let root = temp_root("live");

    let (view, outcome) = vote_flow(
        &transport,
        "a1b2c3d4",
        7,
        VoteChoice::Yes,
        &key,
        &root,
        1_234_567_890,
    )
    .expect("flow");

    // The view came from the service + renders the weight + evidence.
    let v = view.expect("the service answered");
    assert_eq!(v.id, 7);
    let screen = render_proposal(&v, 1_500);
    assert!(screen.contains("0xabab"), "evidence: {screen}");
    assert!(screen.contains("weight 5000"), "{screen}");

    // The outcome is the service ack.
    assert_eq!(
        outcome,
        VoteOutcome::Submitted(VoteAck {
            proposal_id: 7,
            choice: "yes".to_string(),
            snapshot_weight: 5_000,
        })
    );

    // The wire: GET with both params, POST with the full-width fields.
    let captured = server.captured();
    let get = captured.iter().find(|c| c.method == "GET").expect("GET");
    assert!(
        get.path.contains("/gov/proposal?id=7&acct=a1b2c3d4"),
        "GET path: {}",
        get.path
    );
    let post = captured.iter().find(|c| c.method == "POST").expect("POST");
    assert!(post.path.contains("/gov/vote"), "POST path: {}", post.path);
    let body: HashMap<String, serde_json::Value> =
        serde_json::from_str(post.body.as_deref().unwrap_or("")).unwrap();
    assert_eq!(body["vote_version"], 1);
    assert_eq!(body["proposal_id"], 7);
    assert_eq!(body["choice"], "yes");
    assert_eq!(body["account_id"], "a1b2c3d4");
    // Full-width hex: 32-B pubkey = 64 chars; 64-B signature = 128 chars.
    assert_eq!(body["voter_pubkey_hex"].as_str().unwrap().len(), 64);
    assert_eq!(body["signature_hex"].as_str().unwrap().len(), 128);

    // The submitted signature VERIFIES against the mirrored message — the
    // exact bytes the dapps ledger reconstructs.
    use ed25519_dalek::Verifier as _;
    let pubkey: [u8; 32] = hex_fixed(body["voter_pubkey_hex"].as_str().unwrap_or(""));
    let sig: [u8; 64] = hex_fixed(body["signature_hex"].as_str().unwrap_or(""));
    key.verifying_key()
        .verify(&vote_message(7, &pubkey, VoteChoice::Yes), &sig.into())
        .expect("the submitted signature verifies against the ledger's message");

    // No queue file when the service answered.
    assert!(!root.join(".heal/vote/pending.jsonl").exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// The not-live fallback: the SIGNED intent queues locally with the same
/// full-width fields, and the outcome says so.
#[test]
fn not_live_flow_queues_the_signed_intent() {
    let server = spawn_vote_mock(ServeMode::Down, proposal_body());
    let transport = MockVoteTransport {
        inner: riir_clippy::kat_vote_client::HttpVoteTransport::new(&server.base(), 5),
    };
    let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
    let root = temp_root("down");

    let (view, outcome) = vote_flow(
        &transport,
        "a1b2c3d4",
        7,
        VoteChoice::No,
        &key,
        &root,
        1_234_567_890,
    )
    .expect("flow");

    assert!(view.is_none(), "no invented view client-side");
    match outcome {
        VoteOutcome::LocalQueued { queued_at } => {
            let text = std::fs::read_to_string(&queued_at).expect("the queued file");
            let intent: riir_clippy::kat_vote_client::VoteIntent =
                serde_json::from_str(text.trim()).expect("valid JSON intent");
            assert_eq!(intent.proposal_id, 7);
            assert_eq!(intent.choice, "no");
            assert_eq!(intent.voter_pubkey_hex.len(), 64);
            assert_eq!(intent.signature_hex.len(), 128);
            // The queued signature verifies — a queued vote is as binding
            // as a submitted one.
            use ed25519_dalek::Verifier as _;
            let pubkey: [u8; 32] = hex_fixed(&intent.voter_pubkey_hex);
            let sig: [u8; 64] = hex_fixed(&intent.signature_hex);
            key.verifying_key()
                .verify(&vote_message(7, &pubkey, VoteChoice::No), &sig.into())
                .expect("the queued signature verifies");
        }
        other => panic!("expected LocalQueued, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The service can REFUSE (window closed / already voted / weight rules) —
/// the refusal surfaces as an error, never a queue.
#[test]
fn service_refusal_is_an_error_not_a_queue() {
    // A transport whose vote POST returns the error envelope (the
    // proposal GET still answers — the render precedes the refusal).
    struct Refusing;
    impl VoteTransport for Refusing {
        fn get_proposal(&self, _: u64, _: &str) -> HttpResponse {
            HttpResponse::Ok(proposal_body().into_bytes())
        }
        fn post_vote(&self, _: &str) -> HttpResponse {
            HttpResponse::Ok(br#"{"error":"vote window closed"}"#.to_vec())
        }
    }
    let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
    let root = temp_root("refused");
    let err = vote_flow(&Refusing, "a1b2c3d4", 7, VoteChoice::Yes, &key, &root, 1).unwrap_err();
    assert!(err.contains("vote window closed"), "{err}");
    assert!(
        !root.join(".heal/vote/pending.jsonl").exists(),
        "a refusal is never queued"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The ack render + the copy gates on every screen the flow produces.
#[test]
fn renders_and_copy_gates() {
    let v: ProposalView = serde_json::from_str(&proposal_body()).unwrap();
    let screen = render_proposal(&v, 1_500);
    assert!(screen.contains("Evidence:"));
    assert!(screen.contains("Your vote:  weight 5000"));
    assert!(screen.contains("never moves funds"));
    assert!(!screen.contains('$'));
    let ack = vote_confirm_line(&VoteAck {
        proposal_id: 7,
        choice: "yes".to_string(),
        snapshot_weight: 5_000,
    });
    assert!(ack.contains("recorded — snapshot weight 5000"));
    assert!(!ack.contains('$'));
    // sign_vote + the message round-trip (self-consistency; the golden is
    // the cross-repo half). The verify re-derives the SAME message the
    // client signs (vote_message) and checks the signature against the
    // returned pubkey — a real signature check, not just length pins.
    use ed25519_dalek::Verifier as _;
    let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
    let (sig, pubkey) = sign_vote(&key, 9, VoteChoice::No);
    assert_eq!(sig.len(), 64);
    assert_eq!(pubkey.len(), 32);
    let msg = vote_message(9, &pubkey, VoteChoice::No);
    let vkey = ed25519_dalek::VerifyingKey::from_bytes(&pubkey).expect("valid pubkey");
    let sig = ed25519_dalek::Signature::from_bytes(&sig);
    assert!(vkey.verify(&msg, &sig).is_ok());
}
