//! Issue 140 — the cargo-convention invocation path, at the process boundary.
//!
//! `cargo refine <args>` hands the binary its OWN NAME as argv[1] (cargo's
//! subcommand convention; measured: `cargo echotest foo bar` → the binary
//! receives `echotest foo bar`). Plan 192 T1.2 renamed the binary
//! `cargo-heal` → `cargo-refine` but the argv strip still removed only the
//! old `heal` spelling, so every cargo-convention invocation leaked `refine`
//! into the scan paths:
//!
//! | invocation | broken behavior (at HEAD `46a03883`) |
//! |---|---|
//! | `cargo refine login` | KAT subcommand DEMOTED — `dry run — reviewing 2 path(s)` over `["refine", "login"]` |
//! | `cargo refine` (bare) | `reviewing 1 path(s)` over `["refine"]`, never the `.` default |
//! | `cargo refine --suggest` | `no .rs/… files under the given paths`, exit 1 (scans nonexistent `refine/`) |
//! | `cargo refine --suggest src/` | worked (dead root contributes nothing) — why T1.2d's smoke stayed green |
//!
//! Every other CLI test spawns the binary DIRECTLY
//! (`CARGO_BIN_EXE_cargo-refine` + mode flags), which carries no subcommand
//! word — this file is the only coverage of the convention path. The leading
//! `"refine"` here is the exact first argument cargo passes; a regression in
//! the strip fails these arms with the symptoms above, not with a fabricated
//! harness shape.
//!
//! The `heal` spelling stays covered by the same strip (the dist-era binaries
//! keep that name until the owner's T1.2f flip), so it gets an arm too.
//!
//! Hermetic (the kat_soft_gate_cli precedent): own account dir, own `.refine`
//! root, closed-port service URL, lease kill switch — nothing reaches the
//! network, the real account is never read.

#![cfg(all(
    feature = "kat_service",
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints"
))]

#[path = "support/hermetic_home.rs"]
mod hermetic_home;

use std::path::{Path, PathBuf};
use std::process::Command;

/// The closed loopback port — connection REFUSED, instantly, no network.
const CLOSED_PORT: &str = "http://127.0.0.1:1";

/// One tiny source file — enough for the bare dry run to have something to
/// scan (the walk is a plain filesystem walk; no git repo needed).
const A_RS: &str = "pub fn add(a: u32, b: u32) -> u32 { a.wrapping_add(b) }\n";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_refine_convention_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One binary invocation, hermetic (the kat_soft_gate_cli `heal` helper
/// shape). `args` MUST start with the own-name word cargo would pass.
fn refine(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-refine"));
    cmd.current_dir(dir)
        .env("RIIR_REFINE_KAT_ACCOUNT_DIR", dir)
        .envs(hermetic_home::vars(dir))
        .env("RIIR_AUTH_ACCOUNT_DIR", dir)
        .env("RIIR_REFINE_TRAJ_ROOT", dir)
        .env("RIIR_REFINE_KAT_SERVICE_URL", CLOSED_PORT)
        .env("RIIR_REFINE_KAT_LEASE", "0")
        .env("RIIR_REFINE_AUTO_ORACLE", "0")
        .args(args);
    let out = cmd.output().expect("spawn cargo-refine");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

/// `cargo refine` (bare) scans `.` — the leaked-own-name regression turned
/// this into a scan of the nonexistent `refine/` dir ("no .rs/… files",
/// exit 1). With a.rs present the bare dry run must find work.
#[test]
fn bare_invocation_with_leading_own_name_scans_cwd() {
    let dir = scratch("bare");
    std::fs::write(dir.join("a.rs"), A_RS).expect("write a.rs");
    let (code, text) = refine(&dir, &["refine"]);
    assert!(
        !text.contains("no .rs/.wgsl/Dockerfile/Cargo.toml files"),
        "the bare cargo-convention form scanned a leaked path instead of `.`:\n{text}"
    );
    assert_ne!(code, Some(1), "bare form must not exit 1:\n{text}");
    // And it reviewed the CURRENT DIRECTORY — the bare-posture `.` default
    // has its own message; the leaked-own-name regression said
    // "reviewing 1 path(s)" (the leaked `refine`) instead.
    assert!(
        text.contains("reviewing the current directory"),
        "unexpected bare-run output:\n{text}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// `cargo refine login` is a KAT subcommand, never a scan — the leak demoted
/// it to `dry run — reviewing 2 path(s)` over `["refine", "login"]`.
#[test]
fn kat_subcommand_with_leading_own_name_is_not_demoted() {
    let dir = scratch("login");
    let (code, text) = refine(&dir, &["refine", "login"]);
    assert!(
        !text.contains("reviewing"),
        "the KAT subcommand was demoted into a scan:\n{text}"
    );
    assert_eq!(code, Some(0), "hermetic login must succeed:\n{text}");
    std::fs::remove_dir_all(&dir).ok();
}

/// The dist-era spelling strips identically (the tap still installs
/// `cargo-heal` binaries until the owner's T1.2f flip — those binaries run
/// THIS parser when built from source at HEAD).
#[test]
fn legacy_heal_spelling_still_strips() {
    let dir = scratch("heal");
    std::fs::write(dir.join("a.rs"), A_RS).expect("write a.rs");
    let (code, text) = refine(&dir, &["heal"]);
    assert!(
        !text.contains("no .rs/.wgsl/Dockerfile/Cargo.toml files"),
        "the legacy spelling leaked into the scan paths:\n{text}"
    );
    assert_ne!(code, Some(1), "legacy bare form must not exit 1:\n{text}");
    std::fs::remove_dir_all(&dir).ok();
}

/// `--suggest` with a leading own-name and an explicit path still scans the
/// path (the surviving shape at HEAD — pins that the strip did not START
/// eating real scan paths: a dir literally named `refine` must still be
/// scannable when explicitly given, because the strip only ever removes the
/// FIRST argument).
#[test]
fn explicit_path_scan_unaffected_by_the_strip() {
    let dir = scratch("explicit");
    let sub = dir.join("refine");
    std::fs::create_dir_all(&sub).expect("subdir");
    std::fs::write(sub.join("a.rs"), A_RS).expect("write a.rs");
    // `--suggest refine` — argv would be [own-name?] no: direct spawn with
    // the path AFTER the mode flag, no own-name word; the strip never runs.
    let (code, text) = refine(&dir, &["--suggest", "refine"]);
    assert_ne!(code, Some(1), "--suggest over the explicit dir:\n{text}");
    assert!(
        !text.contains("no .rs/.wgsl/Dockerfile/Cargo.toml files"),
        "the explicitly named dir was not scanned:\n{text}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
