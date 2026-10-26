//! Issue 076 (owner mandate) — the burn auto-sync STACKING test: the git
//! analogy driven through the REAL `cargo-refine` binary against a local
//! std-TCP mock HTTP server (the `tests/kat_lease_client_g10.rs` harness
//! shape — no network, no new deps).
//!
//! | phase | service | expected |
//! |---|---|---|
//! | 1. a run with burn ≥ 1 whole KAT, service down | closed port | exit 0, ONE `burn sync: … stacked` line, NO ack file — the delta is a local commit |
//! | 2. the next run, burn grows past the next whole-KAT crossing | live mock | ONE `POST /burn` carrying `burn_kat_total ==` the FULL stacked total floored to WHOLE KAT (not the delta), the signature verifies over the documented message, the ack advances to it, the success line carries the server balance from the response |
//! | 3. a run with NO new burn | live mock | ZERO requests (skip path), no `burn sync:` line, exit 0, ack untouched |
//!
//! Units (issue 081): the charge book is MICRO-KAT, the wire is WHOLE KAT —
//! `wire_kat_total` floors at the sync boundary and sub-KAT remainders ride
//! the next crossing. Every run bills exactly [`ONE_RUN_MICRO_KAT`] so each
//! wire total is exact.
//!
//! Every run gets its own scratch root (`RIIR_REFINE_TRAJ_ROOT`) + account
//! dir (`RIIR_REFINE_KAT_ACCOUNT_DIR`) + service URL; the lease/reheat lanes
//! are kill-switched so the burn push is the only dial under test. The real
//! `~/.config/riir-heal` is never read or written.
//!
//! Gated on the cargo-refine bin's own `required-features` plus
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

#[path = "support/hermetic_home.rs"]
mod hermetic_home;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use riir_refine::kat_burn_client::{self, KatBurnAck};

/// The closed loopback port — connection REFUSED, instantly, no network.
const CLOSED_PORT: &str = "http://127.0.0.1:1";

/// One run must bill at least ONE WHOLE KAT (1_000_000 micro-KAT) or the
/// burn auto-report legitimately skips as sub-granularity: the wire (and
/// the ack) is WHOLE KAT — `wire_kat_total` floors the micro-KAT charge
/// book at the sync boundary and sub-KAT remainders ride the next crossing
/// (issue 081: the 10⁶× over-report incident). Exactly 1M per run keeps
/// every wire total an exact whole number (1 KAT, then 2 KAT, …).
const ONE_RUN_MICRO_KAT: u64 = 1_000_000;

/// The billed fixture: ONE source file worth exactly [`ONE_RUN_MICRO_KAT`]
/// word tokens. Billing is per source READ, raw bytes, parse-independent
/// (the T0.5 contract), so the words ride in a comment: the file is valid
/// Rust that parses to zero items, which is AstChunker's documented
/// whole-file single-chunk fallback — one span, no per-item work, and no
/// signature matching over megabytes.
fn write_a_rs(dir: &Path) {
    let render = |words: u64| {
        let filler: Vec<String> = (0..words).map(|i| format!("w{i}")).collect();
        format!("// {}\n", filler.join(" "))
    };
    // Every filler word is exactly one `count_word_tokens` token (the same
    // counter the binary bills with — `//` is its own token), so one linear
    // adjustment lands exactly on the target; the assert keeps the
    // arithmetic honest instead of trusting it.
    let mut words = ONE_RUN_MICRO_KAT.saturating_sub(8);
    let mut file = render(words);
    let actual = riir_refine::kat_meter::count_word_tokens(file.as_bytes());
    if actual != ONE_RUN_MICRO_KAT {
        words += ONE_RUN_MICRO_KAT - actual;
        file = render(words);
    }
    assert_eq!(
        riir_refine::kat_meter::count_word_tokens(file.as_bytes()),
        ONE_RUN_MICRO_KAT,
        "the fixture must bill exactly one whole KAT per run"
    );
    std::fs::write(dir.join("a.rs"), file).expect("write a.rs");
}

// ── riir-kat Issue 003 T8: the adopt/quarantine-via-flow arm ──────────────

/// (1) an UNSTAMPED ack sitting in the per-net dir (the shape a naive
/// hand-migration leaves) makes the burn flow REFUSE the money decision —
/// the push never dials POST /burn, the refusal names the adopt-state
/// remedy, and the exit code stays 0 (a local refusal, never an error);
/// (2) quarantining that file through the REAL `config adopt-state
/// --discard` CLI unblocks the very next run — the full stacked total
/// pushes, and the ack is born STAMPED (T2's write-side law).
#[test]
fn an_unstamped_ack_refuses_the_push_until_quarantined() {
    let dir = scratch("quarantine");
    write_a_rs(&dir);
    login(&dir);

    // Phase 1: stack against a dead service (the a_failed_push shape) —
    // the per-net charge book is born here, stamped host-only (T2).
    let (code, text) = heal(&dir, CLOSED_PORT, &[]);
    assert_eq!(code, Some(0), "stacking run. output:\n{text}");
    let stacked = charged_kat(&dir);
    assert!(stacked > 0, "the run must bill something to stack");
    assert!(!ack_path(&dir).exists(), "no ack from a dead service");

    // Phase 2: the naive hand-migration — an UNSTAMPED ack lands in the
    // per-net dir (copied from a pre-split machine, say).
    std::fs::write(
        ack_path(&dir),
        "{\"burn_kat_total\": 999, \"contributed\": false, \"first_commitment_hex\": null}",
    )
    .expect("plant unstamped ack");
    let mock = spawn_burn_mock();
    let (code, text) = heal(&dir, &mock.base(), &[]);
    assert_eq!(
        code,
        Some(0),
        "a stamp refusal is a local decision, never an error. output:\n{text}"
    );
    assert!(
        text.contains("carries no network stamp"),
        "the refusal must name the stamp law. output:\n{text}"
    );
    assert!(
        text.contains("adopt-state"),
        "the refusal must name the remedy. output:\n{text}"
    );
    assert!(
        !mock.captured().iter().any(|c| c.path == "/burn"),
        "the refused push must never reach the wire"
    );

    // Phase 3: quarantine THROUGH THE CLI (the remedy the prose names).
    // The discard arm never dials; the URL is the operator's posture.
    let (code, text) = heal(
        &dir,
        &mock.base(),
        &[
            "config",
            "adopt-state",
            "devnet",
            "--discard",
            "--url",
            &mock.base(),
        ],
    );
    assert_eq!(code, Some(0), "discard must succeed. output:\n{text}");
    assert!(
        text.contains("quarantined 1 file(s)"),
        "exactly the planted ack quarantined. output:\n{text}"
    );
    assert!(!ack_path(&dir).exists(), "the unstamped ack is gone");

    // Phase 4: the very next run pushes the FULL stacked total and the
    // ack is born stamped (host + ledger id from the attestation read).
    // The total is the charge book AT PUSH TIME — the refused phase-2 run
    // still billed its own scan (only the PUSH was refused), and phase 4
    // bills its own too; the pushed cumulative must equal the book's
    // final state exactly (the the_next_run precedent's x/y law).
    let (code, text) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code, Some(0), "the post-quarantine run. output:\n{text}");
    let captured = mock.captured();
    let pushes: Vec<_> = captured.iter().filter(|c| c.path == "/burn").collect();
    assert_eq!(pushes.len(), 1, "exactly one push, the full total. {text}");
    let book_now = charged_kat(&dir);
    let wire_now = riir_refine::kat_burn_client::wire_kat_total(book_now);
    assert!(
        pushes[0]
            .body
            .contains(&format!("\"burn_kat_total\":{wire_now}")),
        "the pushed total must be the full book {wire_now} whole KAT (micro {book_now}; stacked \
         micro {stacked} + the refused run's own billing + this run's): {}",
        pushes[0].body
    );
    assert!(book_now > stacked, "the refused run must still have billed");
    let raw = std::fs::read_to_string(ack_path(&dir)).expect("the born ack");
    assert!(
        raw.contains("\"service_host\""),
        "the born ack must carry the stamp: {raw}"
    );
    assert!(
        raw.contains("\"ledger_id\""),
        "the born ack must carry the attested ledger id: {raw}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── the per-net rebase deadlock (riir-kat: the 2026-10-12 zero-burn stall) ──

/// The measured stall, replayed: the per-net book opens FRESH
/// (`charged_kat` near zero — the Issue 003 open_net baseline) while the
/// ACCOUNT carries a bigger server watermark (acked from this net at the
/// book's open watermark). The sync must report `base + net delta`, not
/// the net total alone — pre-fix this run SKIPPED forever (0 pushes) and
/// the server's burn total froze for a month, starving the mining pool.
#[test]
fn the_acked_account_base_plus_the_net_delta_pushes() {
    let dir = scratch("rebase");
    write_a_rs(&dir);
    login(&dir);

    // Phase 1: one billed run against the dead service — the net charge
    // book is born (1M micro, watermark 1M), no ack.
    let (code, _) = heal(&dir, CLOSED_PORT, &[]);
    assert_eq!(code, Some(0));
    assert!(!ack_path(&dir).exists(), "bootstrap: no ack yet");
    let book: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            dir.join(".refine")
                .join("net")
                .join("127.0.0.1")
                .join("kat_charge.json"),
        )
        .expect("the net charge book"),
    )
    .expect("charge json");
    let open_watermark = book
        .get("meter_watermark_tokens")
        .and_then(serde_json::Value::as_u64)
        .expect("watermark");
    assert!(open_watermark > 0);

    // Plant the ACCOUNT truth: a fully-STAMPED ack (the mock's attestation
    // id — "ab" ×32 — and its loopback host) saying the server already
    // holds 21 KAT, acked at THIS net's open watermark. Every field is
    // present: a partial JSON would fail to parse and read as the empty
    // ack (the bootstrap), which is NOT the shape under test.
    let planted = format!(
        "{{\"version\":{},\"burn_kat_total\":21,\"meter_watermark_tokens\":{open_watermark},\
         \"acked_unix\":1700000000,\"last_balance_micro\":174000000,\
         \"contributed\":true,\"first_commitment_hex\":null,\
         \"tuna_balance_micro\":null,\"tuna_expires_unix\":null,\
         \"service_host\":\"127.0.0.1\",\"ledger_id\":\"{}\",\
         \"charged_at_ack_micro\":1000000}}",
        riir_refine::kat_burn_client::BURN_ACK_VERSION,
        "ab".repeat(32)
    );
    std::fs::write(ack_path(&dir), planted).expect("plant the acked account base");

    // Phase 2: one more billed run against the live mock. Pre-fix: the
    // net's own wire total (2) sat below the account base (21) → Skipped,
    // ZERO pushes, forever. Post-fix: the delta since the ack's watermark
    // is one whole KAT → the report is 22.
    let mock = spawn_burn_mock();
    let (code, text) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    let captured = mock.captured();
    assert_eq!(
        captured.len(),
        2,
        "exactly the pre-flight GET / + the POST /burn. output:\n{text}"
    );
    assert_eq!(captured[1].path, "/burn");
    let v: serde_json::Value =
        serde_json::from_str(&captured[1].body).expect("the push body must be JSON");
    assert_eq!(
        v.get("burn_kat_total").and_then(serde_json::Value::as_u64),
        Some(22),
        "the report is the acked base 21 + this net's 1-KAT delta, never the net total. body: {}",
        captured[1].body
    );
    let ack = ack_of(&dir).expect("the ack must advance");
    assert_eq!(ack.burn_kat_total, 22, "the ack adopts the server answer");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The verdict-round arm: a 0.3.0-era per-net ack — account base ABOVE
/// zero, NO charge baseline — must not deadlock. The first encounter
/// adopts the current charge as the baseline AND PERSISTS it (a skip
/// never wrote an ack before, so the baseline could never be recorded);
/// the next whole-KAT delta pushes normally. The first encounter must NOT
/// push base + wire(charged) — the pre-ack charges are already covered by
/// the base (the server answered them), and re-reporting them debits
/// phantom KAT.
#[test]
fn a_v5_net_ack_adopts_its_charge_baseline_and_unsticks() {
    let dir = scratch("v5ack");
    write_a_rs(&dir);
    login(&dir);

    // Phase 1: one billed run against the dead service — the net book is
    // born (1M micro charged), no ack.
    let (code, _) = heal(&dir, CLOSED_PORT, &[]);
    assert_eq!(code, Some(0));
    let book: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            dir.join(".refine")
                .join("net")
                .join("127.0.0.1")
                .join("kat_charge.json"),
        )
        .expect("the net charge book"),
    )
    .expect("charge json");
    let charged_now = book
        .get("charged_kat")
        .and_then(serde_json::Value::as_u64)
        .expect("charged");

    // Plant the 0.3.0 shape: base 21, NO charged_at_ack_micro, fully
    // stamped (the mock's attestation).
    let planted = format!(
        "{{\"version\":{},\"burn_kat_total\":21,\"meter_watermark_tokens\":{},\
         \"acked_unix\":1700000000,\"last_balance_micro\":174000000,\
         \"contributed\":true,\"first_commitment_hex\":null,\
         \"tuna_balance_micro\":null,\"tuna_expires_unix\":null,\
         \"service_host\":\"127.0.0.1\",\"ledger_id\":\"{}\"}}",
        riir_refine::kat_burn_client::BURN_ACK_VERSION,
        charged_now,
        "ab".repeat(32)
    );
    std::fs::write(ack_path(&dir), planted).expect("plant the v5 ack");

    // Phase 2: the first encounter — SKIPS (the base alone; no phantom
    // debit) but PERSISTS the adopted baseline. The encounter run bills
    // its own scan before the sync, so the adopted baseline is the POST-
    // run charge — read the book, not the pre-run number.
    let mock = spawn_burn_mock();
    let (code, text) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert_eq!(
        mock.captured().len(),
        0,
        "the first encounter must not dial — the base covers the pre-ack charges"
    );
    let book: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            dir.join(".refine")
                .join("net")
                .join("127.0.0.1")
                .join("kat_charge.json"),
        )
        .expect("the net charge book"),
    )
    .expect("charge json");
    let charged_at_encounter = book
        .get("charged_kat")
        .and_then(serde_json::Value::as_u64)
        .expect("charged");
    let ack = ack_of(&dir).expect("the upgrade must persist the ack");
    assert_eq!(ack.burn_kat_total, 21, "the base is untouched");
    assert_eq!(
        ack.charged_at_ack_micro,
        Some(charged_at_encounter),
        "the adopted baseline: the charge AT the first encounter"
    );

    // Phase 3: one more billed run (+1 whole KAT) — the delta computes
    // from the adopted baseline and PUSHES 22.
    let (code, text) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    let captured = mock.captured();
    assert_eq!(
        captured.len(),
        2,
        "pre-flight GET / + the POST /burn. output:\n{text}"
    );
    let v: serde_json::Value =
        serde_json::from_str(&captured[1].body).expect("the push body must be JSON");
    assert_eq!(
        v.get("burn_kat_total").and_then(serde_json::Value::as_u64),
        Some(22),
        "base 21 + the net's 1-KAT delta. body: {}",
        captured[1].body
    );
    let ack = ack_of(&dir).expect("the ack must advance");
    assert_eq!(ack.burn_kat_total, 22);
    let _ = std::fs::remove_dir_all(&dir);
}

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

/// A two-route HTTP mock: `GET /` → 200 with a readable devnet
/// attestation (T10/T2: the money transports pre-flight the service-info
/// read, and the T2 flow-level stamp judge shares the same cached read —
/// one GET per process per host), `POST /burn` → 200 with the documented
/// accepted shape, echoing the reported total (what a monotone server
/// does for a fresh report).
fn spawn_burn_mock() -> BurnMock {
    spawn_burn_mock_with("")
}

/// The same mock with an EXTRA raw-JSON fragment spliced into the 200
/// body before the closing brace (leading comma supplied by the caller):
/// the accepted shape grows additively (the B2 split, a TUNA position)
/// and the mock must be able to say so.
fn spawn_burn_mock_with(extra_fields: &str) -> BurnMock {
    let extra_fields = extra_fields.to_string(); // owned — the thread outlives the caller's borrow
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let cap = Arc::clone(&captured);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let Some((path, body)) = read_request(&mut stream) else {
                continue;
            };
            cap.lock().unwrap().push(Captured {
                path: path.clone(),
                body: body.clone(),
            });
            // The attestation route: a fixed 64-hex id — readable, so the
            // T2 judge and the T10 transport gate share one cached read.
            let reply = if path == "/" {
                format!(
                    "{{\"service\":\"kat\",\"env\":\"devnet\",\"ledger_id\":\"{}\"}}",
                    "ab".repeat(32)
                )
            } else {
                let total = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v.get("burn_kat_total").and_then(serde_json::Value::as_u64))
                    .unwrap_or(0);
                format!(
                    "{{\"accepted\":true,\"burn_kat_total\":{total},\"increment_kat\":{total},\"balance_micro\":99000000000000{extra_fields}}}"
                )
            };
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
        .find_map(|l| {
            l.strip_prefix("Content-Length:")
                .and_then(|v| v.trim().parse().ok())
        })
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
        "riir_refine_burn_stack_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One binary invocation, hermetic: its own account dir, its own `.refine`
/// root, the given service URL, and the lease/reheat kill switches so the
/// burn push is the ONLY dial under test.
fn heal(dir: &Path, url: &str, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .current_dir(dir)
        .env("RIIR_REFINE_KAT_ACCOUNT_DIR", dir)
        // Issue 081: the account plane moved to the canonical dir — isolate
        // it too (the consent plane still rides the old var above).
        .envs(hermetic_home::vars(dir))
        .env("RIIR_AUTH_ACCOUNT_DIR", dir)
        .env("RIIR_REFINE_TRAJ_ROOT", dir)
        .env("RIIR_REFINE_KAT_SERVICE_URL", url)
        .env("RIIR_REFINE_KAT_LEASE", "0")
        .args(args)
        .output()
        .expect("spawn cargo-refine");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

fn login(dir: &Path) {
    let (code, text) = heal(dir, CLOSED_PORT, &["login"]);
    assert_eq!(
        code,
        Some(0),
        "hermetic login must succeed. output:\n{text}"
    );
    hermetic_home::assert_minted(&text);
}

/// The settled cumulative burn, read off the charge ledger the run saved
/// (T3/T7, riir-kat Issue 003: PER-NET — every mock in this file is
/// loopback, so the net key is the port-less `127.0.0.1`, the same key
/// [`ack_path`] reads; the pre-T7 flat helper is what this file's 4-test
/// suite rode on until the per-net move orphaned it — found red on develop
/// 2026-10-09, fixed with this landing's T8 arm).
fn charged_kat(dir: &Path) -> u64 {
    let text = std::fs::read_to_string(
        dir.join(".refine")
            .join("net")
            .join("127.0.0.1")
            .join("kat_charge.json"),
    )
    .expect("charge file");
    serde_json::from_str::<serde_json::Value>(&text)
        .expect("charge json")
        .get("charged_kat")
        .and_then(serde_json::Value::as_u64)
        .expect("charged_kat")
}

fn ack_path(dir: &Path) -> PathBuf {
    // T1 (riir-kat Issue 003): the flow places the ack in the per-net dir;
    // every mock in this file is loopback (the port-less net key).
    dir.join(".refine")
        .join("net")
        .join("127.0.0.1")
        .join(KatBurnAck::FILE_NAME)
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
        code,
        Some(0),
        "a failed burn push must never change the exit code. output:\n{text}"
    );
    let total = charged_kat(&dir);
    assert!(total > 0, "the run must bill something to stack");
    assert_eq!(
        burn_sync_lines(&text),
        1,
        "exactly one burn-sync status line. output:\n{text}"
    );
    assert!(
        text.contains("stacked"),
        "the failure line must say stacked. output:\n{text}"
    );
    // The stacked total is the WIRE total (whole KAT — issue 081), not the
    // micro book; assert the exact line, unit-honestly.
    let wire = kat_burn_client::wire_kat_total(total);
    assert!(
        text.contains(&format!(
            "burn sync: offline — {wire} KAT stacked, reports with the next run"
        )),
        "the stacked line must carry the stacked wire total {wire}. output:\n{text}"
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
    assert!(
        ack_of(&dir).is_none(),
        "the failed push must not advance the ack"
    );

    // Phase 2: the live mock.
    let mock = spawn_burn_mock();
    let (code2, text2) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code2, Some(0), "output:\n{text2}");
    let y = charged_kat(&dir);
    assert!(y > x, "the second run bills more (y={y}, x={x})");

    let captured = mock.captured();
    // T10/T2: the pre-flight GET / (readable attestation → cached, so the
    // T2 flow-level judge and the transport gate share ONE) precedes the
    // money POST.
    assert_eq!(captured.len(), 2, "exactly one push. output:\n{text2}");
    assert_eq!(captured[0].path, "/");
    assert_eq!(captured[1].path, "/burn");

    let v: serde_json::Value =
        serde_json::from_str(&captured[1].body).expect("the request body must be JSON");
    let sent = v
        .get("burn_kat_total")
        .and_then(serde_json::Value::as_u64)
        .expect("burn_kat_total");
    assert_eq!(
        sent,
        kat_burn_client::wire_kat_total(y),
        "the report carries the FULL stacked total floored to the wire's whole-KAT unit, not \
         just the delta (y={y} micro)"
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
    assert_eq!(ack.version, riir_refine::kat_burn_client::BURN_ACK_VERSION);
    assert_eq!(ack.burn_kat_total, sent);
    assert_eq!(ack.meter_watermark_tokens, watermark);
    assert!(ack.acked_unix > 0);
    assert_eq!(
        ack.last_balance_micro,
        Some(99_000_000_000_000),
        "the ack carries the last-synced server balance (the 075 gate input)"
    );
    // The mock server answers the pre-TUNA shape — the v4 ack persists the
    // absence as None (never zero), the 123 degradation arm.
    assert_eq!(ack.tuna_balance_micro, None);
    assert_eq!(ack.tuna_expires_unix, None);

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

/// Phase 2b (Issue 123): a B2 server answers with the per-report plane
/// split; the tail names the draw (the TUNA→KAT handoff wording on a
/// both-positive answer), and the TUNA position persists into the v4 ack
/// while the split itself stays transient (a per-report fact, not acked).
#[test]
fn a_split_report_names_the_trial_credit_draw() {
    let dir = scratch("split");
    write_a_rs(&dir);
    login(&dir);

    let extra = ",\"charged_tuna_micro\":70000000,\"charged_kat_micro\":30000000,\
                 \"tuna_balance_micro\":0,\"tuna_expires_unix\":1792281600";
    let mock = spawn_burn_mock_with(extra);
    let (code, text) = heal(&dir, &mock.base(), &[]);
    assert_eq!(code, Some(0), "output:\n{text}");

    // ONE run bills exactly ONE_RUN_MICRO_KAT = one whole wire KAT, so
    // the first run already pushes.
    let captured = mock.captured();
    assert_eq!(
        captured.len(),
        2,
        "the run must push (pre-flight GET / + POST). output:\n{text}"
    );
    assert_eq!(captured[1].path, "/burn");

    // The split line rides the report line (2 `burn sync:` lines total).
    assert_eq!(burn_sync_lines(&text), 2, "output:\n{text}");
    assert!(
        text.contains(
            "burn sync: drew 70.000000 TUNA + 30.000000 KAT — trial credit exhausted mid-report; billing KAT now"
        ),
        "the handoff wording must render. output:\n{text}"
    );

    // The position persists into the ack; the split does not.
    let ack = ack_of(&dir).expect("the ack must exist after an accepted push");
    assert_eq!(ack.tuna_balance_micro, Some(0));
    assert_eq!(ack.tuna_expires_unix, Some(1_792_281_600));

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
    assert_eq!(
        mock.captured().len(),
        2,
        "the drain push (pre-flight GET / + POST)"
    );
    let ack = ack_of(&dir).expect("drained");
    assert_eq!(
        ack.burn_kat_total,
        kat_burn_client::wire_kat_total(charged_kat(&dir)),
        "the drain leaves the cumulative fully reported AT THE WIRE'S whole-KAT granularity \
         (a sub-KAT remainder would ride the next crossing)"
    );

    // No new burn: the source file now bills ZERO tokens (whitespace-only
    // — `count_word_tokens` bills nothing for it, pinned by kat_meter's own
    // tests), so the run settles a zero delta and
    // `wire_kat_total(charged_kat) == ack` → the skip path, zero dials.
    // (Deleting the file entirely would exit 1 at the no-sources guard
    // BEFORE the charge block — the wrong reason.)
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
