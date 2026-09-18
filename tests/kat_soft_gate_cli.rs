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
//! | all four held | account + ack(balance ≤ 0), no server truth | **1** + `REFUSED` + both refuel paths named + no `scanned` line (the refusal precedes the scan) |
//! | contributed | ack `contributed: Some(true)` | 0 (the sticky OptedIn grace) |
//! | balance > 0 | ack(balance 5_000_000), no truth | 0 |
//! | kill switch | depleted ack + `RIIR_HEAL_KAT_GATE=0` | 0 |
//! | billing switch | depleted ack + `RIIR_HEAL_KAT_METER=0` | 0 |
//! | no account | depleted ack, NO login | 0 (anonymous runs are never gated) |
//! | never synced | account, NO ack | 0 (no server truth → warn-only) |
//! | 079 offer (auto-yes) | depleted ack + `RIIR_HEAL_KAT_PROMPT_YES=1` | 1 + `contributing now` + the sync output, then still `REFUSED` (closed port / no store → nothing accepted) |
//! | 079 offer (TTY ask) | depleted ack, no env, piped stdin | 1 + plain refusal, NO `contributing now` (no TTY → the 075 shape) |
//!
//! Issue 080 rows — the sticky grace is SERVER TRUTH, and since the DATED
//! REMOVAL (2026-09-10, fleet v3 live) it is server truth ONLY: the
//! burn-ack v3 `contributed` field (written only from the authenticated
//! burn-push response) is the sole input, the `kat_contribution` marker
//! module is deleted, and any `contributed_once.json` left on disk by a
//! pre-removal install has ZERO effect (the forge surface is gone):
//!
//! | server truth | marker file | exit |
//! |---|---|---|
//! | `Some(false)` | planted | **1** (the forge-closure regression — the file is ignored entirely) |
//! | `Some(true)` | none | 0 (sticky grace, unchanged) |
//! | `None` (pre-v3), balance ≤ 0 | planted | **1** (the removal: no fallback — the marker lifts nothing) |
//! | `None`, no balance | leftover marker | 0 (never-synced warn-only, the unchanged 075 shape) |
//! | a v2 ack FILE (no new fields), balance 0 | none | 1 (read-back compat: old files still gate) |
//! | a v2 ack FILE (no new fields), balance 0 | planted | **1** (the removal: the transition arm is gone) |
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
        // Issue 081: the account plane moved to the canonical dir — isolate
        // it too (the consent plane still rides the old var above).
        .env("RIIR_AUTH_ACCOUNT_DIR", dir)
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
    assert_eq!(
        code,
        Some(0),
        "hermetic login must succeed. output:\n{text}"
    );
}

/// Seed an ack with the given server balance — the gate's input.
/// `contributed` stays `None`: no server truth stated (a pre-v3 answer or
/// a v2 file) — since the dated removal that is simply NOT contributed.
fn seed_ack(dir: &Path, balance_micro: i64) {
    let ack = KatBurnAck {
        version: riir_clippy::kat_burn_client::BURN_ACK_VERSION,
        burn_kat_total: 1_000,
        meter_watermark_tokens: 500,
        acked_unix: 1_700_000_000,
        last_balance_micro: Some(balance_micro),
        contributed: None,
        first_commitment_hex: None,
        tuna_balance_micro: None,
        tuna_expires_unix: None,
    };
    let heal_dir = dir.join(".heal");
    std::fs::create_dir_all(&heal_dir).expect(".heal dir");
    ack.save(&heal_dir.join(KatBurnAck::FILE_NAME))
        .expect("seed ack");
}

/// Seed a v3 ack carrying the server's sticky-grace truth (issue 080) —
/// exactly what an accepted burn push writes when the server answers the
/// v3 shape.
fn seed_ack_with_contributed(dir: &Path, balance_micro: i64, contributed: Option<bool>) {
    let ack = KatBurnAck {
        version: riir_clippy::kat_burn_client::BURN_ACK_VERSION,
        burn_kat_total: 1_000,
        meter_watermark_tokens: 500,
        acked_unix: 1_700_000_000,
        last_balance_micro: Some(balance_micro),
        contributed,
        first_commitment_hex: (contributed == Some(true)).then(|| "ab".repeat(32)),
        tuna_balance_micro: None,
        tuna_expires_unix: None,
    };
    let heal_dir = dir.join(".heal");
    std::fs::create_dir_all(&heal_dir).expect(".heal dir");
    ack.save(&heal_dir.join(KatBurnAck::FILE_NAME))
        .expect("seed ack");
}

/// Seed a literal pre-v3 (v2) ack FILE — no new fields — the shape every
/// existing machine has on disk before the upgrade (issue 080 read-back
/// compat).
fn seed_v2_ack_file(dir: &Path, balance_micro: i64) {
    let heal_dir = dir.join(".heal");
    std::fs::create_dir_all(&heal_dir).expect(".heal dir");
    std::fs::write(
        heal_dir.join(KatBurnAck::FILE_NAME),
        format!(
            r#"{{"version":2,"burn_kat_total":1000,"meter_watermark_tokens":500,"acked_unix":1700000000,"last_balance_micro":{balance_micro}}}"#
        ),
    )
    .expect("seed v2 ack file");
}

/// Seed a LEFTOVER `contributed_once.json` in the account dir — the file
/// the retired `kat_contribution` module once read. Since the dated
/// removal (2026-09-10) nothing reads it; tests seed it to prove it has
/// ZERO effect (the forge surface is gone, not merely outranked).
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
    assert!(
        text.contains("REFUSED"),
        "the refusal must be loud. output:\n{text}"
    );
    assert!(
        text.contains("cargo heal --mine"),
        "must name the contribute path. output:\n{text}"
    );
    assert!(
        text.contains("cargo heal sync"),
        "must name the sync step. output:\n{text}"
    );
    assert!(
        text.contains("top up"),
        "must name the top-up path. output:\n{text}"
    );
    assert!(
        text.contains("RIIR_HEAL_KAT_GATE"),
        "must name the kill switch. output:\n{text}"
    );
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
    assert_eq!(
        code,
        Some(1),
        "offline the offer cannot lift the gate. output:\n{text}"
    );
    assert!(
        text.contains("contributing now"),
        "the offer must run. output:\n{text}"
    );
    assert!(
        text.contains("no trajectory store"),
        "the empty-outbox honest line. output:\n{text}"
    );
    assert!(
        text.contains("REFUSED"),
        "still refused offline. output:\n{text}"
    );
    assert!(
        text.contains("did not lift the gate"),
        "the fallthrough must say why. output:\n{text}"
    );
    assert!(!text.contains("scanned "), "output:\n{text}");
    // The offer path never wrote the (now-retired) marker of its own —
    // only an accepted push records the server truth, and the closed port
    // refused everything. Post-removal NOTHING writes this file.
    assert!(
        !dir.join("contributed_once.json").exists(),
        "a failed push must not write the retired sticky-grace marker"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// THE dated-removal proof (issue 080, 2026-09-10): the local marker alone
/// no longer lifts ANYTHING — the `kat_contribution` reader is deleted, so
/// a contributed_once.json with NO server truth behaves exactly like no
/// marker: the depleted balance refuses.
#[test]
fn the_local_marker_no_longer_lifts_the_gate() {
    let dir = scratch("marker");
    write_a_rs(&dir);
    login(&dir);
    seed_ack(&dir, 0);
    seed_marker(&dir); // the leftover file — zero effect

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(
        code,
        Some(1),
        "the retired marker must have zero effect. output:\n{text}"
    );
    assert!(text.contains("REFUSED"), "output:\n{text}");
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

// ── Issue 080 — the sticky grace became SERVER TRUTH ─────────────────

/// THE forge-closure regression (issue 080): the server said
/// `contributed: false` on the authenticated burn-push ack, so a planted
/// machine-local `contributed_once.json` must NOT lift the gate. Since the
/// dated removal the file is not even read — zero effect, the forge
/// surface is fully removed.
#[test]
fn issue_080_server_said_false_closes_the_planted_marker_forge() {
    let dir = scratch("forge080");
    write_a_rs(&dir);
    login(&dir);
    seed_ack_with_contributed(&dir, 0, Some(false));
    seed_marker(&dir); // the forged/stale legacy marker

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(
        code,
        Some(1),
        "a planted marker must NOT lift the gate once the server said false. output:\n{text}"
    );
    assert!(text.contains("REFUSED"), "output:\n{text}");
    assert!(!text.contains("scanned "), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Server truth `Some(true)` lifts the gate — the sticky grace, unchanged
/// shape, now server-sourced (issue 080).
#[test]
fn issue_080_server_said_true_lifts_the_gate() {
    let dir = scratch("true080");
    write_a_rs(&dir);
    login(&dir);
    seed_ack_with_contributed(&dir, 0, Some(true));

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(!text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The dated-removal inverse of the old transition test (issue 080): a
/// pre-v3 answer (`None`) no longer falls back to anything — with the
/// leftover marker present and a depleted balance the gate STILL refuses.
/// The marker is dead code's dead file.
#[test]
fn issue_080_server_silent_leftover_marker_has_zero_effect() {
    let dir = scratch("trans080");
    write_a_rs(&dir);
    login(&dir);
    seed_ack_with_contributed(&dir, 0, None);
    seed_marker(&dir); // the leftover file — zero effect

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(
        code,
        Some(1),
        "`None` + marker must NOT lift the gate post-removal. output:\n{text}"
    );
    assert!(text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The unchanged 075 warn-only shape over a v3 ack (issue 080): absent
/// fields serde-default to `None`, and a never-synced balance stays
/// warn-only — never gated — EVEN with a leftover marker on disk (the
/// anti-brick direction is preserved at the ack level, permissive).
#[test]
fn issue_080_never_synced_stays_warn_only_even_with_a_leftover_marker() {
    let dir = scratch("empty080");
    write_a_rs(&dir);
    login(&dir);
    // A v3 ack that has synced NOTHING usable: no balance, no contributed
    // — `empty()` is exactly the never-synced shape at the current version.
    let ack = riir_clippy::kat_burn_client::KatBurnAck::empty();
    assert_eq!(ack.last_balance_micro, None);
    assert_eq!(ack.contributed, None);
    let heal_dir = dir.join(".heal");
    std::fs::create_dir_all(&heal_dir).expect(".heal dir");
    ack.save(&heal_dir.join(KatBurnAck::FILE_NAME))
        .expect("seed ack");
    seed_marker(&dir); // the leftover file — still zero effect, still warn-only

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(!text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Read-back compat (issue 080): a pre-existing v2 ack FILE — no new
/// fields — must keep gating (the gate did not go blind to old files; its
/// absent fields read `None` = not contributed). The second arm is the
/// removal's hardest case: the SAME v2 file with the legacy marker that
/// once lifted it now refuses — the transition arm is gone.
#[test]
fn issue_080_v2_ack_files_keep_gating_and_the_marker_died() {
    // v2 file, balance 0, no marker → still REFUSED.
    let dir = scratch("v2refuse");
    write_a_rs(&dir);
    login(&dir);
    seed_v2_ack_file(&dir, 0);

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(code, Some(1), "a v2 file must still gate. output:\n{text}");
    assert!(text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);

    // v2 file + legacy marker → STILL REFUSED (the removal: the marker
    // lifts nothing, the pre-removal transition arm is gone).
    let dir = scratch("v2marker");
    write_a_rs(&dir);
    login(&dir);
    seed_v2_ack_file(&dir, 0);
    seed_marker(&dir);

    let (code, text) = heal(&dir, &[], &[]);
    assert_eq!(
        code,
        Some(1),
        "post-removal the marker must NOT lift a v2-file gate. output:\n{text}"
    );
    assert!(text.contains("REFUSED"), "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}
