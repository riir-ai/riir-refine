//! Issue 151 — the secret-blind surface, AT THE PROCESS BOUNDARY (the
//! `repo_posture_cli.rs` pattern).
//!
//! | arm | asserted |
//! |---|---|
//! | the report | `--secret-scan` over a fixture dir prints the per-finding `[sec]` lines (kind + path:line:col), exit 0 |
//! | the blind rule | the report NEVER contains the planted secret bytes NOR the unkeyed fingerprint (T2's shareable-output rule, at the process boundary) |
//! | the blind transform | `--secret-scan --blind <file>` prints the whole text with `[REDACTED:<kind>]` placeholders, byte-exact outside findings, exit 0 |
//! | the dir misuse | `--blind` on a DIRECTORY exits **2** (a file is the unit) |
//! | the misuse | `--blind` without `--secret-scan`… is a free-standing flag — usage error not asserted here (the flag is accepted where harmless); the ONE-file arm is what matters |
//! | the empty walk | `--secret-scan` over an empty dir exits **1** — a confident green zero is the failure mode (the frontier-report law) |
//! | the in-run warn | `--suggest` over the fixture prints the `secrets: N finding(s)` summary + `[sec]` lines, quiet-by-default otherwise |
//!
//! Hermetic: the same env kills as `repo_posture_cli` (lease/meter/gate/
//! self-evolve/auto-oracle off, unreachable service URL). File-level cfg =
//! the cargo-refine bin's own `required-features`.

#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))]

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Planted secrets. The AWS value is gitleaks' canonical doc example
/// (a documented non-real key); the GitHub token is shape-real but not a
/// live credential. These ARE the values the tests assert are ABSENT from
/// shareable output.
const AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
const GITHUB_TOKEN: &str = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
const FIXTURE: &str = "const KEY: &str = \"AKIAIOSFODNN7EXAMPLE\";\n// token ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890\nlet ok = \"hello world\";\n";

fn refine(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let mut envs: HashMap<&str, String> = HashMap::new();
    envs.insert("RIIR_REFINE_KAT_LEASE", "0".to_string());
    envs.insert("RIIR_REFINE_KAT_METER", "0".to_string());
    envs.insert("RIIR_REFINE_KAT_GATE", "0".to_string());
    envs.insert("RIIR_REFINE_SELF_EVOLVE", "0".to_string());
    envs.insert("RIIR_REFINE_AUTO_ORACLE", "0".to_string());
    envs.insert(
        "RIIR_REFINE_KAT_SERVICE_URL",
        "http://127.0.0.1:9/".to_string(),
    );
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .args(args)
        .envs(envs)
        .current_dir(dir)
        .output()
        .expect("spawn cargo-refine");
    (
        out.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn fixture_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "secret_scan_cli_{}_{}_{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    dir
}

#[test]
fn report_lists_findings_and_leaks_nothing() {
    let dir = fixture_dir("report");
    fs::write(dir.join("config.rs"), FIXTURE).expect("write");
    let (code, out) = refine(&dir, &["--secret-scan", "."]);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("[sec] secret-shaped literal (aws-access-key)"),
        "{out}"
    );
    assert!(
        out.contains("[sec] secret-shaped literal (github-token)"),
        "{out}"
    );
    assert!(out.contains("config.rs:1:"), "{out}");
    // The shareable-output rule, at the process boundary: no value, no
    // unkeyed fingerprint.
    assert!(!out.contains(AWS_KEY), "AWS key leaked into report: {out}");
    assert!(
        !out.contains(GITHUB_TOKEN),
        "GitHub token leaked into report"
    );
    let fp = blake3::hash(AWS_KEY.as_bytes()).to_hex()[..12].to_string();
    assert!(!out.contains(&fp), "unkeyed fingerprint leaked into report");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn blind_transform_prints_redacted_text_and_writes_nothing() {
    let dir = fixture_dir("blind");
    fs::write(dir.join("config.rs"), FIXTURE).expect("write");
    let (code, out) = refine(&dir, &["--secret-scan", "--blind", "config.rs"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("[REDACTED:aws-access-key]"), "{out}");
    assert!(
        out.contains("let ok = \"hello world\";"),
        "byte-exact outside findings: {out}"
    );
    assert!(
        !out.contains(AWS_KEY) && !out.contains(GITHUB_TOKEN),
        "{out}"
    );
    // The shareable-output rule covers the fingerprint too (verdict R2):
    // blind output is the paste-me text — no unkeyed fingerprint there.
    let fp = blake3::hash(AWS_KEY.as_bytes()).to_hex()[..12].to_string();
    assert!(!out.contains(&fp), "fingerprint leaked into blind output");
    // Read-only: the file on disk is untouched.
    assert_eq!(
        fs::read_to_string(dir.join("config.rs")).expect("read"),
        FIXTURE
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn blind_on_a_directory_refuses() {
    let dir = fixture_dir("blinddir");
    let (code, out) = refine(&dir, &["--secret-scan", "--blind", "."]);
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("needs a FILE"), "{out}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn empty_walk_refuses_instead_of_passing() {
    let dir = fixture_dir("empty");
    let (code, out) = refine(&dir, &["--secret-scan", "."]);
    assert_eq!(code, Some(1), "empty walk must not read as a pass: {out}");
    assert!(out.contains("EMPTY WALK"), "{out}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn suggest_run_discloses_findings_quiet_when_clean() {
    let dir = fixture_dir("warn");
    fs::write(dir.join("config.rs"), FIXTURE).expect("write");
    let (code, out) = refine(&dir, &["--suggest", "."]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("secrets: "), "{out}");
    assert!(out.contains("[sec] secret-shaped literal"), "{out}");
    assert!(!out.contains(AWS_KEY), "warn leaked the value: {out}");
    // The warn lines are shareable too — no unkeyed fingerprint (verdict R2).
    let fp = blake3::hash(AWS_KEY.as_bytes()).to_hex()[..12].to_string();
    assert!(!out.contains(&fp), "fingerprint leaked into warn lines");
    fs::remove_dir_all(&dir).ok();

    // Quiet-by-default: a clean file prints NO secrets line at all.
    let dir = fixture_dir("clean");
    fs::write(dir.join("clean.rs"), "fn main() { println!(\"hello\"); }\n").expect("write");
    let (code, out) = refine(&dir, &["--suggest", "."]);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        !out.contains("secrets:"),
        "clean run printed a secrets line: {out}"
    );
    fs::remove_dir_all(&dir).ok();
}
