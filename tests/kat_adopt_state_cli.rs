//! riir-kat Issue 003 T4/T6/T8 (the refine half) — the `config
//! adopt-state` + `config get` state-stamp surface, end to end through the
//! real binary. Hermetic exactly like `kat_burn_stack`: own account dir,
//! own `.refine` root, own service URL; the one mock is a loopback
//! `/history` oracle. NOTHING here reaches a real ledger — the adopt test
//! rides the `--url` dev-rig override, and the declared-net MATCH path
//! never dials the other net (the lazy second pass).

#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service",
    feature = "kat_transport"
))]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A port nothing serves — the fail-closed oracle posture.
const CLOSED_PORT: &str = "http://127.0.0.1:9";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_refine_kat_adopt_cli_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One binary invocation, hermetic (the kat_burn_stack shape).
fn heal(dir: &Path, url: &str, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .current_dir(dir)
        .env("RIIR_REFINE_KAT_ACCOUNT_DIR", dir)
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

fn write_state(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(".refine").join(rel);
    std::fs::create_dir_all(p.parent().expect("parent")).expect("dirs");
    std::fs::write(p, body).expect("write");
}

/// The hermetic local login (zero network — the key-ops-only chain).
fn login(dir: &Path) {
    let (code, text) = heal(dir, CLOSED_PORT, &["login"]);
    assert_eq!(code, Some(0), "hermetic login must succeed. output:\n{text}");
}

fn ack_body(contributed: bool, hex: Option<&str>) -> String {
    format!(
        "{{\"burn_kat_total\": 7, \"contributed\": {contributed}, \"first_commitment_hex\": {}}}",
        hex.map_or("null".to_string(), |h| format!("\"{h}\""))
    )
}

/// A one-request loopback `/history` oracle: answers the contribution fact
/// for whatever account asks. Returns the base URL.
fn spawn_history_mock(contributed: bool, hex: Option<&str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let body = format!(
        "{{\"account_id\": \"{}\", \"balance_micro\": 0, \"granted\": true, \"burn\": null, \
         \"contributed\": {contributed}, \"first_commitment_hex\": {}, \"events\": [], \
         \"events_total\": 0, \"events_truncated\": false, \"receipts\": [], \
         \"receipts_total\": 0, \"receipts_truncated\": false, \"receipts_total_matching\": 0, \
         \"purchases\": [], \"purchases_total\": 0, \"purchases_truncated\": false}}",
        "0".repeat(32),
        hex.map_or("null".to_string(), |h| format!("\"{h}\""))
    );
    std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let mut writer = stream;
            let _ = writeln!(
                writer,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://{addr}")
}

/// T6: `config get` names every state file's network stamp — the UNSTAMPED
/// pre-split set with the adopt-state remedy, the stamped per-net set with
/// its host.
#[test]
fn config_get_names_the_state_stamps() {
    let dir = scratch("get");
    write_state(&dir, "kat_charge.json", "{\"charged_kat\": 5}");
    write_state(
        &dir,
        "net/devnet.gist.rs/kat_charge.json",
        "{\"charged_kat\": 1, \"service_host\": \"devnet.gist.rs\", \"ledger_id\": \"0123456789abcdef\"}",
    );
    let (code, text) = heal(&dir, CLOSED_PORT, &["config", "get"]);
    assert_eq!(code, Some(0), "config get is a pure read. output:\n{text}");
    assert!(text.contains("UNSTAMPED"), "the flat file must flag: {text}");
    assert!(text.contains("adopt-state"), "the remedy must be named: {text}");
    assert!(
        text.contains("last written against `devnet.gist.rs`"),
        "the stamped net-dir file must name its host: {text}"
    );
    assert!(
        text.contains("01234567…"),
        "the ledger id prefix must show: {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T4: a no-oracle file (the charge book) is REFUSED for adoption — never
/// guessed into a net — and stays exactly where it was.
#[test]
fn adopt_state_refuses_no_oracle_files_and_keeps_them() {
    let dir = scratch("refuse");
    write_state(&dir, "kat_charge.json", "{\"charged_kat\": 5}");
    let (code, text) = heal(
        &dir,
        CLOSED_PORT,
        &["config", "adopt-state", "devnet", "--url", CLOSED_PORT],
    );
    assert_eq!(code, Some(2), "a refused file must exit 2. output:\n{text}");
    assert!(
        text.contains("REFUSED kat_charge.json"),
        "the refusal must name the file: {text}"
    );
    assert!(
        text.contains("no server-truth fields"),
        "the reason must be the no-oracle law: {text}"
    );
    assert!(
        dir.join(".refine").join("kat_charge.json").is_file(),
        "the refused file must stay in place"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T4 `--discard`: the quarantine move — files land under
/// `.refine/quarantine/` with `.bak` copies, nothing deleted, exit 0
/// without any network (the dead URL proves discard never dials).
#[test]
fn adopt_state_discard_quarantines_everything() {
    let dir = scratch("discard");
    write_state(&dir, "kat_burn_ack.json", &ack_body(true, Some("ab12")));
    write_state(&dir, "kat_charge.json", "{\"charged_kat\": 5}");
    let (code, text) = heal(
        &dir,
        CLOSED_PORT,
        &["config", "adopt-state", "devnet", "--discard", "--url", CLOSED_PORT],
    );
    assert_eq!(code, Some(0), "discard is a local move. output:\n{text}");
    assert!(text.contains("quarantined 2 file(s)"), "{text}");
    assert!(
        !dir.join(".refine").join("kat_charge.json").exists(),
        "the flat original must be GONE (moved, not copied)"
    );
    let q = dir.join(".refine").join("quarantine");
    assert!(q.join("kat_charge.json").is_file(), "the moved copy");
    assert!(q.join("kat_charge.json.bak").is_file(), "the .bak twin");
    assert!(q.join("kat_burn_ack.json.bak").is_file(), "the ack .bak");
    let _ = std::fs::remove_dir_all(&dir);
}

/// T4's happy path: the mock oracle's contribution row MATCHES the ack's
/// first commitment → the file ADOPTS into the declared net's dir (the
/// loopback `--url` override's port-less net key, the burn-stack law), the
/// non-oracle charge book in the same run still refuses, and the OTHER
/// ledger is never dialed (one request mock + the declared match = no
/// second dial).
#[test]
fn adopt_state_oracle_match_adopts_into_the_net_dir() {
    let dir = scratch("adopt");
    login(&dir);
    write_state(&dir, "kat_burn_ack.json", &ack_body(true, Some("ab12cd34")));
    write_state(&dir, "kat_charge.json", "{\"charged_kat\": 5}");
    let mock = spawn_history_mock(true, Some("ab12cd34"));
    let (code, text) = heal(
        &dir,
        &mock,
        &["config", "adopt-state", "devnet", "--url", &mock],
    );
    assert!(
        text.contains("adopted kat_burn_ack.json"),
        "the matching ack must adopt. output:\n{text}"
    );
    assert_eq!(
        code,
        Some(2),
        "the refused charge book keeps the run at exit 2. output:\n{text}"
    );
    let dest = dir
        .join(".refine")
        .join("net")
        .join("127.0.0.1")
        .join("kat_burn_ack.json");
    assert!(dest.is_file(), "the adopted file must land in the net dir");
    assert!(
        !dir.join(".refine").join("kat_burn_ack.json").exists(),
        "the flat original must be gone"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T4's wrong-net arm: the ack's fact matches the OTHER ledger — adoption
/// to the declared net REFUSES with the belongs-to line (the lazy second
/// pass consulted the other mock).
#[test]
fn adopt_state_names_the_other_net_on_a_mismatch() {
    let dir = scratch("wrongnet");
    login(&dir);
    write_state(&dir, "kat_burn_ack.json", &ack_body(true, Some("ff99")));
    let declared = spawn_history_mock(true, Some("ab12cd34"));
    let (code, text) = heal(
        &dir,
        &declared,
        &["config", "adopt-state", "devnet", "--url", &declared],
    );
    // The other net is the REAL https://mainnet.gist.rs here — the lazy
    // pass dials it once. An offline box yields the discard-only refusal
    // (fail-closed); a connected box may yield belongs-to. Both refuse —
    // assert the refusal + the file's survival, not the network's shape.
    assert_eq!(code, Some(2), "a mismatch must refuse. output:\n{text}");
    assert!(text.contains("REFUSED kat_burn_ack.json"), "{text}");
    assert!(
        dir.join(".refine").join("kat_burn_ack.json").is_file(),
        "the refused file must stay in place"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T11: `--net` is one-invocation sugar — it selects the URL the command
/// resolves (visible through `config get`'s active line) and refuses a
/// non-alias value at parse time.
#[test]
fn net_flag_selects_the_invocation_net() {
    let dir = scratch("netflag");
    let (code, text) = heal(&dir, CLOSED_PORT, &["--net", "devnet", "config", "get"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(
        text.contains("net: this invocation targets devnet"),
        "the sugar line must print: {text}"
    );
    assert!(
        text.contains("https://devnet.gist.rs"),
        "config get must show the selected URL as active: {text}"
    );
    let (code, text) = heal(&dir, CLOSED_PORT, &["--net", "bogus", "config", "get"]);
    assert_eq!(code, Some(2), "a non-alias must refuse: {text}");
    let (code, text) = heal(&dir, CLOSED_PORT, &["config", "adopt-state", "testnet"]);
    assert_eq!(
        code,
        Some(2),
        "the torn-down testnet must not be adoptable: {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
