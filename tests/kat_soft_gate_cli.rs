//! Issue 075 (owner soft gate, 2026-09-08) — the depleted-account refusal,
//! driven through the REAL `cargo-heal` binary (the `kat_burn_stack.rs`
//! harness shape — closed port, scratch roots, no network).
//!
//! The gate's input is the last SYNCED server balance carried on the burn
//! ack (`last_balance_micro`, v2), never the local projection. These tests
//! seed that ack directly and assert the four predicate legs AT THE PROCESS
//! BOUNDARY — the part a unit test cannot cover (a flag that parses in the
//! source and gates the wrong mode at `main()` is the Plan-064 failure
//! class):
//!
//! | leg broken/hold | seeded state | exit |
//! |---|---|---|
//! | all four held | account + ack(balance ≤ 0) + no marker | **1** + `REFUSED` + both refuel paths named + no `scanned` line (the refusal precedes the scan) |
//! | contributed | + `contributed_once.json` | 0 (the sticky OptedIn grace) |
//! | balance > 0 | ack(balance 5_000_000), no marker | 0 |
//! | kill switch | depleted ack + `RIIR_HEAL_KAT_GATE=0` | 0 |
//! | billing switch | depleted ack + `RIIR_HEAL_KAT_METER=0` | 0 |
//! | no account | depleted ack, NO login | 0 (anonymous runs are never gated) |
//! | never synced | account, NO ack | 0 (no server truth → warn-only) |
//! | 079 offer (auto-yes) | depleted ack + `RIIR_HEAL_KAT_PROMPT_YES=1` | 1 + `contributing now` + the sync output, then still `REFUSED` (closed port / no store → no marker) |
//! | 079 offer (TTY ask) | depleted ack, no env, piped stdin | 1 + plain refusal, NO `contributing now` (no TTY → the 075 shape) |
//!
//! Every run gets its own scratch root (`RIIR_HEAL_TRAJ_ROOT`) + account
//! dir (`RIIR_HEAL_KAT_ACCOUNT_DIR`); the service URL points at a closed
//! loopback port so an allowed run's post-run burn push fails fast (stacked)
//! without reaching anything. The real `~/.config/riir-heal` is never read
//! or written.
//!
//! Gated on the cargo-heal bin's own `required-features` plus
//! `kat_transport` (the ack's balance half). Without them the file compiles
//! to an empty binary that prints a green `0 passed` — the Issue-713
//! green-zero trap — which is why the `required-features` row in Cargo.toml
//! is not optional.

#![cfg(all(
    feature = "kat_transport",
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints"
))]

use std::path::{Path, PathBuf};
use std::process::Command;

use riir_clippy::kat_burn_client::KatBurnAck;

/// The closed loopback port — connection REFUSED, instantly, no network.
const CLOSED_PORT: &str = "http://127.0.0.1:1";

/// One tiny source file — enough for an allowed run to bill something.
const A_RS: &str = "pub fn add(a: u32, b: u32) -> u32 { a.wrapping_add(b) }\n";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_clippy_soft_gate_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One binary invocation, hermetic: its own account dir, its own `.heal`
/// root, the closed-port service URL, and the lease kill switch so nothing
/// but the gate under test decides the run's fate.
fn heal(dir: &Path, extra_env: &[(&str, &str)], args: &[&str]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-heal"));
    cmd.current_dir(dir)
        .env("RIIR_HEAL_KAT_ACCOUNT_DIR", dir)
        .env("RIIR_HEAL_TRAJ_ROOT", dir)
        .env("RIIR_HEAL_KAT_SERVICE_URL", CLOSED_PORT)
        .env("RIIR_HEAL_KAT_LEASE", "0")
        .args(args);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("spawn cargo-heal");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

/// Offline login into the scratch account dir (the burn-stack precedent:
/// login needs no service).
fn login(dir: &Path) {
    let (code, text) = heal(dir, &[], &["login"]);
    assert_eq!(code, Some(0), "hermetic login must succeed. output:\n{text}");
}

/// Seed the v2 ack with the given server balance — the gate's input.
fn seed_ack(dir: &Path, balance_micro: i64) {
    let ack = KatBurnAck {
        version: riir_clippy::kat_burn_client::BURN_ACK_VERSION,
        burn_kat_total: 1_000,
        meter_watermark_tokens: 500,
        acked_unix: 1_700_000_000,
        last_balance_micro: Some(balance_micro),
    };
    let heal_dir = dir.join(".heal");
    std::fs::create_dir_all(&heal_dir).expect(".heal dir");
    ack.save(&heal_dir.join(KatBurnAck::FILE_NAME)).expect("seed ack");
}

/// Seed the contribution marker (the sticky grace) in the account dir.
fn seed_marker(dir: &Path) {
    std::fs::write(
        dir.join("contributed_once.json"),
        "{\"version\":1,\"first_accepted_unix\":1700000000,\"commitment_hex\":\"cafe\"}",
    )
    .expect("seed marker");
}

/// All four legs held → REFUSED with both refuel paths, and the scan never
/// started (no `scanned` line — the refusal precedes the meter, let alone
/// the work).
#[test]
fn depleted_never_contributed_refuses_at_run_start() {
    let dir = scratch("refuse");
    write_a_rs(&dir);
    login(&dir);
    seed_ack(&dir, 0);

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(1), "the soft gate must refuse. output:\n{text}");
    assert!(text.contains("REFUSED"), "the refusal must be loud. output:\n{text}");
    assert!(text.contains("cargo heal --mine"), "must name the contribute path. output:\n{text}");
    assert!(text.contains("cargo heal sync"), "must name the sync step. output:\n{text}");
    assert!(text.contains("top up"), "must name the top-up path. output:\n{text}");
    assert!(text.contains("RIIR_HEAL_KAT_GATE"), "must name the kill switch. output:\n{text}");
    // Issue 079: no env and no TTY (Command::output pipes stdin) → the
    // offer must NOT fire — the plain 075 refusal shape.
    assert!(
        !text.contains("contributing now"),
        "the offer must not fire without the env or a TTY. output:\n{text}"
    );
    assert!(
        !text.contains("scanned "),
        "the refusal must precede the scan. output:\n{text}"
    );
    // The gate reads, never writes: no charge ledger may appear.
    assert!(
        !dir.join(".heal").join("kat_charge.json").exists(),
        "a refused run must not open the billing path"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same depleted state, fired through an explicit `--fix` — the gate is
/// mode-wide (every scanning mode burns fuel; the bare-dry-run billing is
/// measured in `kat_burn_stack`), not a `--write`-only gate.
#[test]
fn the_gate_is_mode_wide_fix_refuses_too() {
    let dir = scratch("fixmode");
    write_a_rs(&dir);
    login(&dir);
    seed_ack(&dir, -1);

    let (code, text) = heal(&dir, &[], &["--fix"]);
    assert_eq!(code, Some(1), "output:\n{text}");
    assert!(text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Issue 079: the automation hook runs the contribute-now offer, then —
/// with the service at a closed port and no trajectory store to build a
/// batch from — the gate STILL refuses, honestly (no marker was written;
/// the sticky grace needs an ACCEPTED push, never a promise).
#[test]
fn prompt_yes_offers_contribute_now_then_still_refuses_offline() {
    let dir = scratch("prompt_yes");
    write_a_rs(&dir);
    login(&dir);
    seed_ack(&dir, 0);

    let (code, text) = heal(&dir, &[("RIIR_HEAL_KAT_PROMPT_YES", "1")], &["--fix"]);
    assert_eq!(code, Some(1), "offline the offer cannot lift the gate. output:\n{text}");
    assert!(text.contains("contributing now"), "the offer must run. output:\n{text}");
    assert!(
        text.contains("no trajectory store"),
        "the empty-outbox honest line. output:\n{text}"
    );
    assert!(text.contains("REFUSED"), "still refused offline. output:\n{text}");
    assert!(
        text.contains("did not lift the gate"),
        "the fallthrough must say why. output:\n{text}"
    );
    assert!(!text.contains("scanned "), "output:\n{text}");
    // The offer path writes no marker of its own — only an accepted push
    // does (the closed port refused everything).
    assert!(
        !dir.join("contributed_once.json").exists(),
        "a failed push must not write the sticky-grace marker"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The sticky grace: a contributed-and-synced machine is never gated, even
/// at a deep negative balance (payouts land at the epoch settle).
#[test]
fn the_contribution_marker_lifts_the_gate() {
    let dir = scratch("marker");
    write_a_rs(&dir);
    login(&dir);
    seed_ack(&dir, 0);
    seed_marker(&dir);

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(!text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A positive synced balance allows the run (the normal case).
#[test]
fn a_positive_synced_balance_allows_the_run() {
    let dir = scratch("positive");
    write_a_rs(&dir);
    login(&dir);
    seed_ack(&dir, 5_000_000);

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(!text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The kill switches disarm the gate: the 075 gate switch and the shared
/// billing switch (one billing plane, the Plan 024 decision).
#[test]
fn the_kill_switches_disarm_the_gate() {
    for (tag, env) in [
        ("gate_switch", ("RIIR_HEAL_KAT_GATE", "0")),
        ("billing_switch", ("RIIR_HEAL_KAT_METER", "0")),
    ] {
        let dir = scratch(tag);
        write_a_rs(&dir);
        login(&dir);
        seed_ack(&dir, 0);

        let (code, text) = heal(&dir, &[env], &[]);
        assert_eq!(code, Some(0), "{tag}: output:\n{text}");
        assert!(!text.contains("REFUSED"), "{tag}: output:\n{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// An anonymous machine is never gated — no account, no honest gate (the
/// funnel stays open).
#[test]
fn no_account_stays_warn_only() {
    let dir = scratch("anon");
    write_a_rs(&dir);
    seed_ack(&dir, 0);

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(!text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A logged-in machine that never completed a burn push has no server
/// truth — warn-only, never gated (the projection is NOT the input).
#[test]
fn never_synced_stays_warn_only() {
    let dir = scratch("fresh");
    write_a_rs(&dir);
    login(&dir);

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(!text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

fn write_a_rs(dir: &Path) {
    std::fs::write(dir.join("a.rs"), A_RS).expect("write a.rs");
}
