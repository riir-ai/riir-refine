//! KAT lease client — the G10 client-half fixtures (Proposal 005 Phase 1
//! T1.6, Plan 040), driven against a **local std-TCP mock HTTP server** (the
//! dapps kat_stripe / fixer_regate mock precedent — no network, no deps).
//!
//! The G10 gates (proposal §GOAT, line 337) this binary pins:
//!
//! - **(a) 304 fast path** — no body transfer, no lease charge, and a
//!   stalled-server fixture must start healing within the timeout budget on
//!   the local corpus.
//! - **(b) atomicity** — kill the process at any point during fetch/verify
//!   (fault injection at the store's two install steps) → the next run sees
//!   either the old epoch or a fully-verified new one, never partial.
//! - **(c) tamper** — a 200 body that does not hash to its own
//!   ETag/commitment is rejected with fallback-to-local.
//! - **(d) reheat delta-billing** — a straddle with no changed candidate
//!   sets bills ≈0 KAT; one with N changed spans re-bills exactly those
//!   spans' tokens, deterministically (the G9 discipline).
//! - **(e) itemization** — output separates pass-1/pass-2 token counts and
//!   the lease charge line; no silent aggregates.
//!
//! Plus the sync/claim wire fixtures (POST shape assertions against the
//! captured requests) and the offline-silence gate (G3).

#![cfg(feature = "kat_transport")]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use riir_refine::kat_lease_client::{
    self, run_update_phase, HttpLeaseTransport, HttpResponse, LeaseCharge, LeaseManifest,
    LeaseSegmentDesc, LeaseStage, LeaseStore, LeaseTransport, UpdateStatus,
};

// ── the mock server (std TCP; the kat_stripe shape) ────────────────────────

#[derive(Debug, Clone)]
enum LeaseMode {
    /// Serve the manifest body for /lease (+ segments); honor If-None-Match.
    Serve { manifest: LeaseManifest },
    /// Serve a TAMPERED manifest body (a flipped body byte — the commitment
    /// no longer covers it).
    Tamper { manifest: LeaseManifest },
    /// Accept connections and never respond (the stall fixture).
    Stall,
}

struct MockServer {
    addr: SocketAddr,
    captured: Arc<Mutex<Vec<CapturedRequest>>>,
    _handle: std::thread::JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct CapturedRequest {
    method: String,
    path: String,
    if_none_match: Option<String>,
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

/// One captured body per test (the content-addressed segment bytes live in
/// the closure's environment via the manifest).
fn spawn_lease_mock(mode: LeaseMode, segments: HashMap<[u8; 32], Vec<u8>>) -> MockServer {
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
            if matches!(mode, LeaseMode::Stall) {
                // Accept + never respond: the client's read timeout must
                // fire within its budget.
                std::thread::sleep(Duration::from_secs(30));
                continue;
            }
            let resp = route(&mode, &segments, &req);
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

fn route(
    mode: &LeaseMode,
    segments: &HashMap<[u8; 32], Vec<u8>>,
    req: &CapturedRequest,
) -> Vec<u8> {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/lease") => {
            let manifest = match mode {
                LeaseMode::Serve { manifest } | LeaseMode::Tamper { manifest } => manifest,
                LeaseMode::Stall => unreachable!("stall never routes"),
            };
            let etag = kat_lease_client::hex32(&manifest.commitment);
            if req.if_none_match.as_deref() == Some(etag.as_str()) {
                http(304, &[], Some(&etag))
            } else {
                let mut body = match mode {
                    LeaseMode::Tamper { .. } => {
                        let mut b = manifest.to_bytes();
                        b[4] ^= 0xFF; // flip a body byte (inside the epoch field)
                        b
                    }
                    _ => manifest.to_bytes(),
                };
                // Re-derive nothing: the served bytes are exactly what the
                // client must verify.
                let etag_hdr = kat_lease_client::hex32(&manifest.commitment);
                let _ = &mut body;
                http(200, &body, Some(&etag_hdr))
            }
        }
        ("GET", p) if p.starts_with("/segment/") => {
            let hex = p.trim_start_matches("/segment/");
            match kat_lease_client::unhex32(hex).and_then(|id| segments.get(&id).cloned()) {
                Some(bytes) => http(200, &bytes, None),
                None => http(404, b"not found", None),
            }
        }
        ("POST", "/sync") => http(200, br#"{"accepted":true}"#, None),
        ("POST", "/claim") => http(200, br#"{"queued":true}"#, None),
        _ => http(404, b"not found", None),
    }
}

fn http(status: u16, body: &[u8], etag: Option<&str>) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        304 => "Not Modified",
        _ => "",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(e) = etag {
        head.push_str(&format!("ETag: {e}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    // BYTES, not String — the manifest body is binary (LE fields + hashes);
    // a lossy UTF-8 round-trip would corrupt it (caught live by the first
    // happy-path run: every 200 arrived "tampered").
    let mut out = head.into_bytes();
    out.extend_from_slice(body);
    out
}

/// Read one HTTP request (headers + Content-Length body). `None` on EOF.
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
    let mut if_none_match = None;
    let mut content_length = 0usize;
    for line in lines {
        if let Some(v) = line.strip_prefix("If-None-Match:") {
            if_none_match = Some(v.trim().to_string());
        }
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
        body_prefix = Some(String::from_utf8_lossy(&body[..body.len().min(120)]).into_owned());
    }
    Some(CapturedRequest {
        method,
        path,
        if_none_match,
        body_prefix,
    })
}

// ── fixtures ───────────────────────────────────────────────────────────────

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir-refine-g10-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Build a corpus-pack segment (one domain's evolved corpus).
fn pack_segment(domain: &str, rule_ids: &[&str]) -> Vec<u8> {
    let pack = riir_refine::kat_lease_client::CorpusPack {
        pack_version: riir_refine::kat_lease_client::CORPUS_PACK_VERSION,
        domain: domain.to_string(),
        entries: rule_ids
            .iter()
            .map(|r| riir_refine::kat_lease_client::PatternEntry {
                rule_id: r.to_string(),
                buggy_code: format!("fn {r}_buggy() {{}}"),
                fixed_code: format!("fn {r}_fixed() {{}}"),
            })
            .collect(),
    };
    riir_refine::kat_lease_client::render_pack(&pack)
}

fn manifest_for(epoch: u64, segments: &[Vec<u8>]) -> (LeaseManifest, HashMap<[u8; 32], Vec<u8>>) {
    let descs: Vec<LeaseSegmentDesc> = segments
        .iter()
        .map(|s| LeaseSegmentDesc::of(s.as_slice()))
        .collect();
    let manifest = LeaseManifest::build(epoch, 1, descs);
    let mut map = HashMap::with_capacity(segments.len());
    for s in segments {
        let d = LeaseSegmentDesc::of(s);
        map.insert(d.id, s.clone());
    }
    (manifest, map)
}

// ── G10a: the 304 fast path ────────────────────────────────────────────────

#[test]
fn g10a_304_fast_path_no_body_no_charge() {
    let (manifest, segs) = manifest_for(9, &[pack_segment("clippy", &["a", "b"])]);
    let server = spawn_lease_mock(LeaseMode::Serve { manifest }, segs);
    let dir = temp_dir("304");
    let store = LeaseStore::at_dir(dir.join("store"));
    let mut charge = LeaseCharge::open(&store);

    // First fetch: 200 → applied + charged.
    let t = HttpLeaseTransport::new(&server.base(), 5);
    let out = run_update_phase(&t, &store, &mut charge, false, None);
    assert!(
        matches!(
            out.status,
            UpdateStatus::Applied {
                epoch: 9,
                charge_kat: riir_refine::kat_lease_client::LEASE_FEE_KAT
            }
        ),
        "first fetch applies + charges: {:?}",
        out.status
    );

    // The 304 case: the second lease check carries If-None-Match, gets 304,
    // charges NOTHING, transfers no body.
    let before = charge.charged_kat();
    let start = Instant::now();
    let out = run_update_phase(&t, &store, &mut charge, false, None);
    assert!(matches!(out.status, UpdateStatus::NotModified));
    assert_eq!(charge.charged_kat(), before, "a 304 NEVER charges");
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "the 304 fast path is one RTT"
    );
    // The wire evidence: the second /lease request carried the ETag.
    let reqs = server.captured();
    let leases: Vec<&CapturedRequest> = reqs.iter().filter(|r| r.path == "/lease").collect();
    assert_eq!(leases.len(), 2);
    assert!(leases[1].if_none_match.is_some(), "If-None-Match sent");
}

#[test]
fn g10a_stalled_server_returns_within_the_budget() {
    // The stall fixture: the server accepts + never responds. The client's
    // bounded window must fire → Failed → healing proceeds on local (the
    // outcome IS the "proceeds" — run_update_phase returning is the heal
    // continuing).
    let server = spawn_lease_mock(LeaseMode::Stall, HashMap::new());
    let dir = temp_dir("stall");
    let store = LeaseStore::at_dir(dir.join("store"));
    let mut charge = LeaseCharge::open(&store);
    let t = HttpLeaseTransport::new(&server.base(), 1);
    let start = Instant::now();
    let out = run_update_phase(&t, &store, &mut charge, false, None);
    let elapsed = start.elapsed();
    assert!(
        matches!(out.status, UpdateStatus::Failed(_)),
        "{:?}",
        out.status
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "healing starts within the timeout budget (took {elapsed:?})"
    );
    // The store is untouched (fresh — no epoch).
    assert!(store.current().is_none());
}

// ── G10b: atomicity (the happy path + fault injection) ─────────────────────

#[test]
fn g10b_happy_path_fetches_verifies_and_swaps_atomically() {
    let (manifest, segs) = manifest_for(
        12,
        &[
            pack_segment("clippy", &["r1"]),
            pack_segment("perf", &["p1", "p2"]),
        ],
    );
    let server = spawn_lease_mock(LeaseMode::Serve { manifest }, segs);
    let dir = temp_dir("happy");
    let store = LeaseStore::at_dir(dir.join("store"));
    let mut charge = LeaseCharge::open(&store);
    let t = HttpLeaseTransport::new(&server.base(), 5);

    let out = run_update_phase(&t, &store, &mut charge, false, None);
    assert!(matches!(
        out.status,
        UpdateStatus::Applied { epoch: 12, .. }
    ));
    assert_eq!(store.current_epoch(), Some(12));
    // The pointer is complete + every segment on disk.
    let m = store.current().unwrap();
    assert_eq!(m.segments.len(), 2);
    for s in &m.segments {
        assert!(store.segment_bytes(&s.id).is_some());
    }
    // Both segments were FETCHED (fresh store — the diff was everything).
    let fetched = server
        .captured()
        .iter()
        .filter(|r| r.path.starts_with("/segment/"))
        .count();
    assert_eq!(fetched, 2, "content-addressed diff fetched both segments");
    // The overlay resolves (the install feeds the matcher).
    let overlay = riir_refine::kat_lease_client::overlay_from_epoch(&store, None);
    assert!(overlay.is_some());
    let (epoch, domains) = overlay.unwrap();
    assert_eq!(epoch, 12);
    assert_eq!(domains.len(), 2, "clippy + perf packs");
}

#[test]
fn g10b_interrupt_before_swap_leaves_the_old_epoch_intact() {
    // Fault injection at the store's two-step install: E+1's segments are
    // written (the interrupt point — the process dies before the pointer
    // rename). The next run sees E1 fully, never a partial E2.
    let dir = temp_dir("fault");
    let store = LeaseStore::at_dir(dir.join("store"));
    let e1_pack = pack_segment("clippy", &["e1rule"]);
    store
        .install_atomic(
            &LeaseStage::begin(LeaseManifest::build(
                1,
                1,
                vec![LeaseSegmentDesc::of(&e1_pack)],
            ))
            .and_then(|mut s| {
                s.stage_segment(&LeaseSegmentDesc::of(&e1_pack).id, &e1_pack)?;
                s.commit()
            })
            .unwrap(),
        )
        .unwrap();
    assert_eq!(store.current_epoch(), Some(1));

    // E2 arrives + its segments land on disk — then the "crash".
    let e2_segs = [pack_segment("clippy", &["new"])];
    let (m2, _) = manifest_for(2, &e2_segs);
    let mut stage = LeaseStage::begin(m2.clone()).unwrap();
    stage
        .stage_segment(&LeaseSegmentDesc::of(&e2_segs[0]).id, &e2_segs[0])
        .unwrap();
    let leased = stage.commit().unwrap();
    store.install_segments(&leased).unwrap(); // ← the interrupt point
                                              // The next run sees E1 — the segments are inert without the pointer.
    assert_eq!(store.current_epoch(), Some(1));
    let overlay_epoch =
        riir_refine::kat_lease_client::overlay_from_epoch(&store, None).map(|(e, _)| e);
    assert_eq!(overlay_epoch, Some(1));
    // Completing the swap (the retry) lands E2 atomically.
    store.publish_pointer(&m2).unwrap();
    assert_eq!(store.current_epoch(), Some(2));
}

// ── G10c: tamper ───────────────────────────────────────────────────────────

#[test]
fn g10c_tampered_manifest_is_rejected_with_fallback_to_local() {
    let (manifest, segs) = manifest_for(3, &[pack_segment("clippy", &["x"])]);
    let server = spawn_lease_mock(LeaseMode::Tamper { manifest }, segs);
    let dir = temp_dir("tamper");
    let store = LeaseStore::at_dir(dir.join("store"));
    // A prior good epoch is installed — tamper must FALL BACK to it.
    let prior = pack_segment("clippy", &["prior-rule"]);
    store
        .install_atomic(
            &LeaseStage::begin(LeaseManifest::build(
                1,
                1,
                vec![LeaseSegmentDesc::of(&prior)],
            ))
            .and_then(|mut s| {
                s.stage_segment(&LeaseSegmentDesc::of(&prior).id, &prior)?;
                s.commit()
            })
            .unwrap(),
        )
        .unwrap();
    let mut charge = LeaseCharge::open(&store);
    let t = HttpLeaseTransport::new(&server.base(), 5);
    let out = run_update_phase(&t, &store, &mut charge, false, None);
    match &out.status {
        UpdateStatus::Failed(e) => assert!(
            e.to_lowercase().contains("tamper"),
            "the tamper verdict names itself: {e}"
        ),
        other => panic!("tamper must Failed, got {other:?}"),
    }
    assert_eq!(store.current_epoch(), Some(1), "fallback to local E1");
    assert_eq!(charge.charged_kat(), 0, "a rejected manifest never charges");
}

// ── G10d: reheat delta-billing (deterministic) ─────────────────────────────

fn spans(items: &[(&str, u64, &[&str])]) -> Vec<riir_refine::kat_lease_client::PassSpans> {
    items
        .iter()
        .map(
            |(text, tokens, rules)| riir_refine::kat_lease_client::PassSpans {
                key: riir_refine::kat_lease_client::span_key(text),
                tokens: *tokens,
                rules: {
                    let mut v: Vec<String> = rules.iter().map(|r| r.to_string()).collect();
                    v.sort();
                    v
                },
            },
        )
        .collect()
}

#[test]
fn g10d_no_changed_candidate_sets_bills_approximately_zero() {
    // The straddle fixture: E+1 landed but changed NO span's top-k set.
    let mut pass1: HashMap<[u8; 32], Vec<String>> = HashMap::new();
    for (text, _t, rules) in &[
        ("fn a() {}", 50u64, &["needless_return"][..]),
        ("fn b() {}", 60, &["vec-with-capacity"]),
        ("fn c() {}", 70, &["manual_map"]),
    ] {
        let mut v: Vec<String> = rules.iter().map(|r| r.to_string()).collect();
        v.sort();
        pass1.insert(riir_refine::kat_lease_client::span_key(text), v);
    }
    let pass2 = spans(&[
        ("fn a() {}", 50, &["needless_return"]),
        ("fn b() {}", 60, &["vec-with-capacity"]),
        ("fn c() {}", 70, &["manual_map"]),
    ]);
    let plan = riir_refine::kat_lease_client::plan_reheat(&pass1, &pass2);
    assert_eq!(plan.reheat_tokens, 0, "≈0 KAT re-bill");
    assert_eq!(plan.changed.len(), 0);
    assert_eq!(plan.unchanged, 3);
}

#[test]
fn g10d_n_changed_spans_rebill_exactly_their_tokens_deterministically() {
    let mut pass1: HashMap<[u8; 32], Vec<String>> = HashMap::new();
    for (text, rules) in [
        ("fn same() {}", &["a"][..]),
        ("fn changed() {}", &["old"][..]),
        ("fn edited_away() {}", &["c"]),
    ] {
        let mut v: Vec<String> = rules.iter().map(|r| r.to_string()).collect();
        v.sort();
        pass1.insert(riir_refine::kat_lease_client::span_key(text), v);
    }
    // Pass 2 (post-fix tree): `same` identical; `changed` has a NEW top-k
    // set under E+1; `edited_away` was rewritten (new chunk text); + one
    // entirely new chunk.
    let pass2 = spans(&[
        ("fn same() {}", 10, &["a"]),
        ("fn changed() {}", 20, &["new-rule"]),
        ("fn edited_v2() {}", 30, &["c"]),
        ("fn fresh() {}", 40, &["z"]),
    ]);
    let plan = riir_refine::kat_lease_client::plan_reheat(&pass1, &pass2);
    assert_eq!(plan.changed.len(), 3, "changed + edited + fresh");
    assert_eq!(
        plan.reheat_tokens,
        20 + 30 + 40,
        "exactly those spans' tokens"
    );
    assert_eq!(plan.unchanged, 1);
    // G9 determinism: identical replays → identical plan.
    assert_eq!(
        plan,
        riir_refine::kat_lease_client::plan_reheat(&pass1, &pass2)
    );
}

// ── G10e: itemization ──────────────────────────────────────────────────────

#[test]
fn g10e_itemized_output_separates_passes_and_charge() {
    let report = riir_refine::kat_lease_client::ReheatReport {
        pass1_epoch: 4,
        pass2_epoch: 5,
        pass1_tokens: 12_500,
        pass2_tokens: 400,
        changed_spans: 14,
        new_fixes: 14,
    };
    let line = report.itemized_line();
    assert_eq!(
        line,
        "pass 1 (epoch 4): 12500 tokens · pass 2 (epoch 5): 400 tokens, +14 fixes"
    );
    // The lease charge line is SEPARATE (never aggregated into the pass
    // line) + carries no $ figure.
    let charge_line = LeaseCharge::line(5, 1000);
    assert!(charge_line.starts_with("corpus lease epoch 5"));
    assert!(charge_line.contains("KAT"));
    assert!(!charge_line.contains('$'));
    assert!(!line.contains("KAT"), "the pass line carries tokens only");
}

// ── the sync/claim wire fixtures + G3 silence ──────────────────────────────

#[test]
fn sync_posts_the_ndjson_payload_and_the_wire_shape_is_captured() {
    let (manifest, segs) = manifest_for(1, &[pack_segment("clippy", &["s"])]);
    let server = spawn_lease_mock(LeaseMode::Serve { manifest }, segs);
    let t = HttpLeaseTransport::new(&server.base(), 5);
    let ndjson = "{\"payload_version\":1}\n{\"lint_key\":\"x\"}\n";
    match t.post_sync(ndjson) {
        HttpResponse::Ok(_) => {}
        other => panic!("mock must accept the sync POST: {other:?}"),
    }
    let reqs = server.captured();
    let post = reqs
        .iter()
        .find(|r| r.method == "POST" && r.path == "/sync")
        .expect("the POST /sync arrived");
    assert!(post
        .body_prefix
        .as_deref()
        .is_some_and(|b| b.contains("payload_version")));
}

#[test]
fn claim_posts_the_composed_request() {
    let (manifest, segs) = manifest_for(1, &[pack_segment("clippy", &["c"])]);
    let server = spawn_lease_mock(LeaseMode::Serve { manifest }, segs);
    let t = HttpLeaseTransport::new(&server.base(), 5);
    let json = r#"{"claim_version":1,"rail":"solana"}"#;
    match t.post_claim(json) {
        HttpResponse::Ok(_) => {}
        other => panic!("mock must accept the claim POST: {other:?}"),
    }
    let reqs = server.captured();
    let post = reqs
        .iter()
        .find(|r| r.path == "/claim")
        .expect("the POST /claim arrived");
    assert!(post
        .body_prefix
        .as_deref()
        .is_some_and(|b| b.contains("solana")));
}

#[test]
fn g3_offline_failure_is_silent_and_local_stands() {
    // G3 (the offline-bit-identity construction): a dead endpoint fails
    // fast, prints nothing (silent() is true), and the store is untouched.
    // With kat features ON + offline + no installed epoch, behavior is
    // byte-identical to features-off: no epoch → compiled-in corpus.
    let dir = temp_dir("offline");
    let store = LeaseStore::at_dir(dir.join("store"));
    let mut charge = LeaseCharge::open(&store);
    // 127.0.0.1:9 (discard port) — unreachable by construction.
    let t = HttpLeaseTransport::new("http://127.0.0.1:9", 1);
    let out = run_update_phase(&t, &store, &mut charge, false, None);
    assert!(matches!(out.status, UpdateStatus::Failed(_)));
    assert!(
        out.silent(),
        "the run-start phase prints NOTHING on failure"
    );
    assert!(
        store.current().is_none(),
        "no epoch installed → compiled-in"
    );
    assert_eq!(charge.charged_kat(), 0);
}

#[test]
fn out_of_kat_pauses_hot_push_without_failing() {
    // §KAT warn-and-pay: at 0 remaining the update phase SKIPS (hot-push
    // paused at epoch end) — mining/settlement continue elsewhere; the
    // phase itself never errors.
    let (manifest, segs) = manifest_for(5, &[pack_segment("clippy", &["z"])]);
    let server = spawn_lease_mock(LeaseMode::Serve { manifest }, segs);
    let dir = temp_dir("ookat");
    let store = LeaseStore::at_dir(dir.join("store"));
    let mut charge = LeaseCharge::open(&store);
    let t = HttpLeaseTransport::new(&server.base(), 5);
    let out = run_update_phase(&t, &store, &mut charge, true, None);
    match out.status {
        UpdateStatus::Skipped(reason) => {
            assert!(reason.contains("out-of-KAT"), "{reason}");
        }
        other => panic!("out-of-KAT must Skip, got {other:?}"),
    }
    assert!(store.current().is_none(), "no fetch happened");
}

#[test]
fn compat_gap_stays_local() {
    // A manifest needing a newer client → Skipped, local stands, no charge.
    let manifest = LeaseManifest::build(99, u32::MAX, vec![]);
    let server = spawn_lease_mock(LeaseMode::Serve { manifest }, HashMap::new());
    let dir = temp_dir("compat");
    let store = LeaseStore::at_dir(dir.join("store"));
    let mut charge = LeaseCharge::open(&store);
    let t = HttpLeaseTransport::new(&server.base(), 5);
    let out = run_update_phase(&t, &store, &mut charge, false, None);
    assert!(
        matches!(out.status, UpdateStatus::Skipped(_)),
        "{:?}",
        out.status
    );
    assert!(store.current().is_none());
    assert_eq!(charge.charged_kat(), 0);
}
