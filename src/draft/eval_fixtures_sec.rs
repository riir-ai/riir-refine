//! Issue 134 T3 re-arm — `sec` domain eval fixtures (hand-authored variants
//! of the corpus anti-patterns; the `eval_fixtures_perf` convention —
//! variant names, canonical shapes, one span per rule). Batch 258 added
//! the web-trust seed's three (Plan 262); Batch 291 the penthera
//! distill's three (Plan 295); Batch 292 the `CVEngine` distill's four
//! (Plan 296); Batch 294 the `RustSec` transcription's two (Plan 298).
//!
//! The thin-domain labeled sets are the T3 power-floor re-arm trigger: each
//! set adds a qualifying domain to the demonstrability sweep (n=3 → n=7 at
//! the full feature state). Pure data; consumed by `super::retrieval_eval`
//! under the `latent_retrieval` feature.

// Uniform `r#"..."#` raw-string style is deliberate for fixture data (some
// spans contain `"` or `"#`-adjacent sequences; mixed styles churn every
// fixture edit).
#![allow(clippy::needless_raw_string_hashes)]

/// (`rule_id`, `span_code`) — one span per rule; each genuinely instantiates
/// the rule's anti-pattern (semantic label, authored against the corpus entry).
pub const SEC_SPANS: &[(&str, &str)] = &[
    (
        "ffi-extern-fn-catch-unwind",
        r#"#[no_mangle]
pub extern "C" fn on_frame(frame_no: u32) {
    let weights = vec![frame_no; 8];
    eprintln!("{weights:?}");
}
"#,
    ),
    (
        "ptr-deref-null-check-first",
        r#"pub struct PacketHeader { pub payload_len: u32 }

pub unsafe fn read_payload_len(hdr: *const PacketHeader) -> Option<u32> {
    Some((*hdr).payload_len)
}
"#,
    ),
    (
        "drop-impl-no-panic",
        r#"pub struct Session { pub socket: Option<u32> }

impl Drop for Session {
    fn drop(&mut self) {
        let sock = self.socket.unwrap();
        eprintln!("closing {sock}");
    }
}
"#,
    ),
    (
        "extern-fn-reference-param-to-pointer",
        r#"#[no_mangle]
pub extern "C" fn sum_ticks(ticks: &Vec<u32>) -> u32 {
    ticks.iter().sum()
}
"#,
    ),
    (
        "rc-backedge-field-prefer-weak",
        r#"use std::rc::Rc;

pub struct SceneGraph {
    pub node_id: u32,
    pub branches: Vec<Rc<SceneGraph>>,
    pub root: Rc<SceneGraph>,
}
"#,
    ),
    (
        "slice-from-raw-parts-len-guard",
        r#"pub unsafe fn frame_bytes(data: *const u8, count: usize) -> &'static [u8] {
    std::slice::from_raw_parts(data, count)
}
"#,
    ),
    (
        "unsafe-impl-send-sync-unsound-field",
        r#"use std::cell::RefCell;
use std::rc::Rc;

pub struct RegistryInner {
    pub entries: RefCell<Rc<str>>,
}
unsafe impl Send for RegistryInner {}
"#,
    ),
    (
        "ffi-export-panicking-body",
        r#"#[no_mangle]
pub extern "C" fn advance(seed: u32) -> u32 {
    let lanes: Vec<u32> = vec![seed, 7, 9];
    lanes[0]
}
"#,
    ),
    (
        "weak-hash-to-blake3",
        r#"pub fn artifact_id(blob: &[u8]) -> [u8; 20] {
    use sha1::{Digest, Sha1};
    let digest: [u8; 20] = Sha1::digest(blob).into();
    digest
}
"#,
    ),
    (
        "password-hash-argon2",
        r#"pub fn hash_password(password: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] = Sha256::digest(password.as_bytes()).into();
    digest
}
"#,
    ),
    (
        "constant-time-secret-compare",
        r#"use subtle::ConstantTimeEq;

pub fn check_mac(received: &[u8], expected: &[u8]) -> bool {
    received == expected
}
"#,
    ),
    (
        "nan-unsafe-sort-comparator",
        r#"pub fn sort_threats(levels: &mut [f32]) {
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
}
"#,
    ),
    (
        "length-prefix-validate-before-alloc",
        r#"pub fn read_packet(conn: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {
    let mut hdr = [0_u8; 4];
    std::io::Read::read_exact(conn, &mut hdr)?;
    let len = u32::from_be_bytes(hdr) as usize;
    let mut payload = vec![0_u8; len];
    std::io::Read::read_exact(conn, &mut payload)?;
    Ok(payload)
}
"#,
    ),
    (
        "secret-file-create-without-0600",
        r#"pub fn store_credential(path: &std::path::Path, secret: &str) -> std::io::Result<()> {
    let mut out = std::fs::File::create(path)?;
    use std::io::Write;
    out.write_all(secret.as_bytes())
}
"#,
    ),
    (
        "unbounded-read-into-vec",
        r#"pub fn read_body(mut sock: std::net::TcpStream) -> std::io::Result<Vec<u8>> {
    let mut payload = Vec::new();
    std::io::Read::read_to_end(&mut sock, &mut payload)?;
    Ok(payload)
}
"#,
    ),
    (
        "unchecked-wire-arithmetic",
        r#"pub const WIRE_HEADER_BYTES: usize = 12;

pub fn total_frame_size(wire_len: [u8; 4]) -> usize {
    let len = u32::from_be_bytes(wire_len) as usize;
    len + WIRE_HEADER_BYTES
}
"#,
    ),
    // Batch 258 (Plan 262) — the web-trust seed, one span per new rule.
    (
        "untrusted-path-join-escape",
        r#"pub fn load_skin(base: &std::path::Path, skin: &str) -> std::io::Result<Vec<u8>> {
    std::fs::read(base.join(skin))
}
"#,
    ),
    (
        "webhook-signature-over-raw-bytes",
        r#"fn check_tag(key: &[u8], mac: &[u8]) -> bool {
    mac.len() == 24 && !key.is_empty()
}

fn accept_delivery(key: &[u8], wire: &[u8]) -> bool {
    let note: serde_json::Value = serde_json::from_slice(wire).unwrap_or_default();
    let round_trip = serde_json::to_vec(&note).unwrap_or_default();
    check_tag(key, &round_trip)
}
"#,
    ),
    (
        "unvalidated-egress-url",
        r#"pub fn pull_artifact(store: &str, digest: &str) -> Result<String, String> {
    let url = format!("https://{store}/artifacts/{digest}");
    let body = ureq::get(&url).call().map_err(|e| e.to_string())?;
    body.into_string().map_err(|e| e.to_string())
}
"#,
    ),
    // Batch 291 (Plan 295) — the penthera distill, one span per new rule.
    (
        "cors-any-origin-with-credentials",
        r#"pub fn public_api_cors() -> tower_http::cors::CorsLayer {
    tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_credentials(true)
}
"#,
    ),
    (
        "jwt-insecure-validation-flags",
        r#"pub fn check_session_token(token: &str, key: &jsonwebtoken::DecodingKey) -> bool {
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;
    jsonwebtoken::decode::<serde_json::Value>(token, key, &validation).is_ok()
}
"#,
    ),
    (
        "auth-cookie-without-secure-flags",
        r#"pub fn auth_cookie(token: &str) -> cookie::Cookie<'static> {
    cookie::Cookie::build(("auth", token.to_owned())).build()
}
"#,
    ),
    // Batch 292 (Plan 296) — the CVEngine distill, one span per new rule.
    (
        "client-ip-from-untrusted-forwarded-header",
        r#"pub fn throttle_key(req: &Request) -> std::net::IpAddr {
    let ip = req.header("x-forwarded-for")
        .and_then(|chain| chain.split(',').next())
        .and_then(|hop| hop.trim().parse::<std::net::IpAddr>().ok());
    if banned(ip) { panic!("rejected"); }
    ip.unwrap()
}
"#,
    ),
    (
        "shell-string-command-interpolation",
        r#"pub fn probe(domain: &str) -> std::io::Result<String> {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("tlsx -san -silent -host {domain}"))
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
"#,
    ),
    (
        "ipv4-mapped-range-check-unwrap",
        r#"pub fn is_blocked(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => BLOCKED.contains(&v4),
        std::net::IpAddr::V6(_) => false,
    }
}
"#,
    ),
    (
        "abuse-strike-counts-cross-site-requests",
        r#"impl Guard {
    pub fn inspect(&mut self, body: &str) -> Verdict {
        if body.contains("; cat ") {
            self.strikes += 1;
            return Verdict::Rejected;
        }
        Verdict::Ok
    }
}
"#,
    ),
    // Batch 294 (Plan 298) — the RustSec transcription, one span per new rule.
    (
        "packed-field-silent-overflow-wrong-key",
        r#"pub fn find_entry(header: [u8; 4], table: &[Entry; 65536]) -> &Entry {
    let id = u32::from_be_bytes(header);
    let slot = (id & 0xFFFF) as usize;
    &table[slot]
}
"#,
    ),
    (
        "capacity-exceeded-silent-fallback-not-refusal",
        r#"pub fn build_section(entries: &[Entry]) -> Vec<u8> {
    let mut owned = entries.to_vec();
    if owned.len() > MAX_ENTRIES {
        owned.truncate(MAX_ENTRIES);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(owned.len() as u16).to_be_bytes());
    out
}
"#,
    ),
];
