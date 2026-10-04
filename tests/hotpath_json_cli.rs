//! Issue 146 T0.5 — the `--hotpath-json` feed lane, AT THE PROCESS BOUNDARY
//! (the `repo_posture_cli.rs` pattern). The profile-guided-healing oracle
//! lane's T0 contract: a hotpath report is validated + keyed + disclosed
//! before any anchoring exists (T0.2); the STALE-PROFILE HAZARD is the
//! load-bearing arm — a report taken at commit X must never anchor spans at
//! commit Y, so a keyed MISMATCH refuses with exit 2 (the `--clippy-json`
//! hard-error precedent), never a silent ranking-only fallback.
//!
//! | arm | asserted |
//! |---|---|
//! | keyed match | exit 0; `keyed MATCHED at repo HEAD` + the ranked `[hot]` span line (calls × avg, alloc twin joined) |
//! | stale mismatch | exit **2**; `STALE PROFILE REFUSED` naming both SHAs |
//! | unreadable path | exit **2**; `cannot read` |
//! | unkeyed report | exit 0; the loud `UNKEYED` disclosure (no `meta.git` — cannot mismatch, proceeds best-effort) |
//! | absent flag | no hotpath output at all (default-off is byte-compat) |
//!
//! Hermetic: the `repo_posture_cli.rs` env set (lease/meter/gate/self-evolve
//! off, unreachable loopback service) + `RIIR_REFINE_AUTO_ORACLE=0` (no
//! cargo spawn — the fixture is a cargo project ONLY because the keying
//! needs a project root; the oracle is not under test here).
//!
//! File-level cfg = the cargo-refine bin's own `required-features` (the
//! `build_stamp_cli.rs` law): with less there is no binary to invoke and
//! `CARGO_BIN_EXE_cargo-refine` does not resolve.

#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))]

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

fn heal(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let mut envs: HashMap<&str, String> = HashMap::new();
    envs.insert("RIIR_REFINE_KAT_LEASE", "0".to_string());
    envs.insert("RIIR_REFINE_KAT_METER", "0".to_string());
    envs.insert("RIIR_REFINE_KAT_GATE", "0".to_string());
    envs.insert("RIIR_REFINE_SELF_EVOLVE", "0".to_string());
    envs.insert("RIIR_REFINE_AUTO_ORACLE", "0".to_string());
    envs.insert(
        "RIIR_REFINE_KAT_SERVICE_URL",
        "http://127.0.0.1:9".to_string(),
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

/// A minimal cargo+git project. The manifest exists so
/// `detect_project_root` resolves the root; the git repo exists so
/// `rev-parse HEAD` answers the keying question.
fn toy_repo(tag: &str) -> (tempdir::TempDirGuard, String) {
    let guard = tempdir::TempDirGuard::new(tag);
    let dir = guard.path();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"hp-toy\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/main.rs"),
        "fn main() {\n    let v: Vec<u32> = (0..100).collect();\n    let s: u64 = v.iter().map(|x| u64::from(*x)).sum();\n    println!(\"{s}\");\n}\n",
    )
    .unwrap();
    for a in [vec!["init", "-q"], vec!["add", "-A"], vec!["commit", "-qm", "toy"]] {
        let ok = Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(&a)
            .current_dir(&dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        assert!(ok, "git {} failed (git is required for the keying arms)", a[0]);
    }
    let sha = String::from_utf8(
        Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(dir)
            .output()
            .expect("rev-parse")
            .stdout,
    )
    .unwrap();
    (guard, sha.trim().to_string())
}

/// A one-entry report at `sha`: one located timing entry + its alloc twin,
/// exercising the duration/byte string parsing + the twin join end-to-end.
fn report_json(sha: Option<&str>) -> String {
    let git = sha
        .map(|s| format!(r#""git": {{"sha": "{s}"}}"#))
        .map(|g| format!(", {g}"))
        .unwrap_or_default();
    format!(
        r#"{{
          "type": "hotpath-report", "version": "0.7.0",
          "meta": {{"rustc": "1.98.0", "os": "macos-aarch64", "created_at": "2026-10-04T00:00:00Z"{git}}},
          "functions_timing": {{"profiling_mode": "functions", "time_elapsed": "1.0 s",
            "total_elapsed_ns": 1000000000, "description": "x", "caller_name": "", "percentiles": [0.5],
            "total_count": 1, "included_count": 1,
            "data": [{{"id": 1, "name": "hp_toy::main", "calls": 500, "sampled_calls": 500,
              "avg": "200.0 µs", "total": "100.0 ms", "percent_total": "80.0%",
              "location": {{"file": "src/main.rs", "line": 2, "column": 5}}}}]}},
          "functions_alloc": {{"profiling_mode": "functions", "time_elapsed": "1.0 s",
            "total_elapsed_ns": 1000000000, "total_allocated": "1.0 MB", "description": "x",
            "caller_name": "", "percentiles": [0.5], "total_count": 1, "included_count": 1,
            "data": [{{"id": 1, "name": "hp_toy::main", "calls": 300, "sampled_calls": 300,
              "avg": "512 B", "total": "150.0 KB", "percent_total": "50.0%",
              "location": {{"file": "src/main.rs", "line": 2, "column": 5}}}}]}}
        }}"#
    )
}

// The repo_posture_cli.rs pattern uses std::env::temp_dir + a unique suffix;
// a tiny guard keeps that shape without a new dev-dep.
mod tempdir {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    pub struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        pub fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "refine-hotpath-cli-{tag}-{}-{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&path).expect("temp dir");
            Self { path }
        }

        pub fn path(&self) -> PathBuf {
            self.path.clone()
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

#[test]
fn keyed_match_proceeds_and_discloses_the_ranked_span() {
    let (guard, sha) = toy_repo("match");
    let report = guard.path().join("report.json");
    std::fs::write(&report, report_json(Some(&sha))).unwrap();
    let (code, out) = heal(&guard.path(), &["refine", "--hotpath-json", "report.json", "."]);
    assert_eq!(code, Some(0), "matched feed proceeds: {out}");
    assert!(out.contains("keyed MATCHED at repo HEAD"), "{out}");
    assert!(
        out.contains("[hot] hp_toy::main — src/main.rs:2 — 500 calls x 200000 ns avg | alloc 150000 B"),
        "the ranked span line carries the parsed duration + the joined alloc twin: {out}"
    );
}

#[test]
fn stale_mismatch_refuses_exit_2() {
    let (guard, _sha) = toy_repo("stale");
    let report = guard.path().join("report.json");
    std::fs::write(
        &report,
        report_json(Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")),
    )
    .unwrap();
    let (code, out) = heal(&guard.path(), &["refine", "--hotpath-json", "report.json", "."]);
    assert_eq!(code, Some(2), "a stale profile REFUSES: {out}");
    assert!(out.contains("STALE PROFILE REFUSED"), "{out}");
    assert!(
        out.contains("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
        "the refusal names the report's SHA: {out}"
    );
}

#[test]
fn unreadable_feed_is_a_hard_error() {
    let (guard, _sha) = toy_repo("unreadable");
    let (code, out) = heal(&guard.path(), &["refine", "--hotpath-json", "nope.json", "."]);
    assert_eq!(code, Some(2), "an unreadable requested oracle never falls back silently: {out}");
    assert!(out.contains("cannot read"), "{out}");
}

#[test]
fn unkeyed_report_proceeds_with_loud_disclosure() {
    let (guard, _sha) = toy_repo("unkeyed");
    let report = guard.path().join("report.json");
    std::fs::write(&report, report_json(None)).unwrap();
    let (code, out) = heal(&guard.path(), &["refine", "--hotpath-json", "report.json", "."]);
    assert_eq!(code, Some(0), "unkeyed cannot mismatch: {out}");
    assert!(out.contains("UNKEYED"), "{out}");
    assert!(out.contains("[hot] hp_toy::main"), "{out}");
}

#[test]
fn absent_flag_prints_no_hotpath_lines() {
    let (guard, _sha) = toy_repo("absent");
    let (code, out) = heal(&guard.path(), &["refine", "."]);
    assert_eq!(code, Some(0), "{out}");
    assert!(!out.contains("hotpath feed:"), "default-off stays silent: {out}");
}
