//! The mining-consent PUSH gate, driven through the REAL binary — plus the
//! pin that the removed `--mine`/`--unmine` CLI surface stays removed.
//!
//! History: the 2026-09-08 consent tiers were driven by `--mine`/`--unmine`.
//! The owner's vocabulary law extension (2026-11-06, issue 192 GAP 10:
//! mining vocabulary reads as crypto-extraction to non-blockchain users;
//! join/leave is the contribution surface) REMOVED that CLI. What stays is
//! the `kat_mining_consent` STATE machinery (internal, GAP-10 exempt) and
//! the sync push gate that honours it — so this suite now drives the gate
//! through PLANTED consent records, the same JSON the historical writer
//! produced:
//!
//! | invocation | exit | assertion |
//! |---|---|---|
//! | `sync` with a planted opted-out record | 0 | push SKIPPED (nothing built, signed or sent), pull still attempted |
//! | `sync` with no record (Unset) | 0/1 | the gate lets the push half through |
//! | `--mine` / `--unmine` | **2** | unknown flag — the surface is gone; the usage page does not advertise it |
//!
//! Every run gets its own `RIIR_REFINE_KAT_ACCOUNT_DIR` under the temp dir —
//! the real `~/.config/riir-heal` is never read or written, and no test
//! mutates the shared process env (each child gets its own).
//!
//! Gated on the binary's own `required-features` plus `kat_transport` (the
//! `sync` command's gate). Without them the file compiles to an empty
//! binary that prints a green `0 passed`, which is why the `required-features`
//! row in Cargo.toml is not optional.

#![cfg(all(
    feature = "kat_transport",
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints"
))]

#[path = "support/hermetic_home.rs"]
mod hermetic_home;

use std::path::{Path, PathBuf};
use std::process::Command;

/// A scratch account dir + repo root, unique per test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_heal_consent_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One binary invocation, hermetic: its own account dir, its own `.refine`
/// root, a dead service URL (no network), and the meter/lease kill switches
/// the CI convention uses.
fn heal(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .env("RIIR_REFINE_KAT_ACCOUNT_DIR", dir)
        .envs(hermetic_home::vars(dir))
        .env("RIIR_AUTH_ACCOUNT_DIR", dir)
        .env("RIIR_REFINE_TRAJ_ROOT", dir)
        // 127.0.0.1:1 is closed: the lease pull fails fast instead of
        // reaching the real service from a test.
        .env("RIIR_REFINE_KAT_SERVICE_URL", DEAD_URL)
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

/// The dead service URL every child is pointed at — also the key of the
/// per-net state partition (riir-kat Issue 003 T1) the binary resolves.
const DEAD_URL: &str = "http://127.0.0.1:1";

fn consent_file(dir: &Path) -> PathBuf {
    dir.join(riir_refine::kat_mining_consent::CONSENT_FILE)
}

fn state(dir: &Path) -> riir_refine::kat_mining_consent::MiningConsent {
    riir_refine::kat_mining_consent::read(dir)
}

use riir_refine::kat_mining_consent::MiningConsent;

/// Plant a consent record in the exact JSON shape the historical writer
/// produced (and the reader still accepts) — the CLI writer is gone; the
/// record is what the gate reads.
fn plant_consent(dir: &Path, opted_in: bool) {
    let doc = format!(
        r#"{{"version": {}, "opted_in": {}, "recorded_unix_secs": 0}}"#,
        riir_refine::kat_mining_consent::CONSENT_VERSION,
        opted_in
    );
    std::fs::write(consent_file(dir), doc).expect("plant consent record");
}

/// The gate: `sync` refuses its PUSH half while a planted opted-out record
/// exists — nothing is built, signed or sent; the lease PULL still runs
/// (a read is not a contribution).
#[test]
fn sync_refuses_the_push_half_while_opted_out() {
    let dir = scratch("refuse");
    plant_consent(&dir, false);
    assert_eq!(
        state(&dir),
        MiningConsent::OptedOut,
        "the plant must read back"
    );

    let (code, text) = heal(&dir, &["sync"]);
    assert_eq!(code, Some(0), "an opt-out is not an error. output:\n{text}");
    assert!(
        text.contains("push SKIPPED"),
        "sync must say the push was skipped. output:\n{text}"
    );
    assert!(
        text.contains("a contribution opt-out is on record"),
        "the skip must name the vocabulary-law wording. output:\n{text}"
    );
    assert!(
        !text.contains("sync payload:"),
        "no payload may be BUILT or signed while opted out. output:\n{text}"
    );
    assert!(
        text.contains("corpus lease"),
        "the lease PULL half must still run — a read is not a contribution. output:\n{text}"
    );
    assert!(
        !text.contains("cargo refine mine") && !text.contains("cargo refine unmine"),
        "no refusal may teach the removed commands. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The control arm: with consent UNSET, `sync` behaves exactly as it did
/// before the consent record existed — it reaches the payload half.
/// Without this the refusal test above cannot distinguish "the gate works"
/// from "sync never pushes anyway".
#[test]
fn sync_with_unset_consent_still_reaches_the_push_half() {
    use riir_refine::kat_mining_consent::MiningConsent;
    let dir = scratch("syncunset");
    let heal_dir = dir.join(".refine");
    std::fs::create_dir_all(&heal_dir).expect(".refine");
    std::fs::write(heal_dir.join("trajectories.kv"), "").expect("plant a store");

    assert_eq!(state(&dir), MiningConsent::Unset, "no record, no gate");
    let (code, text) = heal(&dir, &["sync"]);
    assert!(
        !text.contains("push SKIPPED"),
        "Unset must NOT refuse — running sync IS the consent. output:\n{text}"
    );
    // It gets as far as the store/account half (whichever it stops on is
    // environment-dependent; what matters is that the CONSENT gate let it
    // through). Exit 1 is legitimate here — an empty planted store has no
    // account behind it.
    assert!(
        matches!(code, Some(0) | Some(1)),
        "unexpected exit {code:?}. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The removal pin: the `--mine`/`--unmine` surface is GONE (2026-11-06
/// vocabulary law, issue 192 GAP 10). Both flags exit 2 as unknown flags,
/// neither writes a consent record, and the usage page does not advertise
/// them. A future edit that re-adds the flags or the teach lines reds here.
#[test]
fn the_removed_mine_surface_stays_gone() {
    use riir_refine::kat_mining_consent::MiningConsent;
    let dir = scratch("removed");
    for invocation in ["--mine", "--unmine", "--purge"] {
        let (code, text) = heal(&dir, &[invocation]);
        assert_eq!(
            code,
            Some(2),
            "{invocation} must be an unknown flag now. output:\n{text}"
        );
        assert_eq!(
            state(&dir),
            MiningConsent::Unset,
            "{invocation} must not write consent"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// And the help page does not teach the removed commands (the GAP-10 sweep
/// half: no mine/unmine teach lines anywhere in `--help-all` output).
#[test]
fn usage_does_not_advertise_the_removed_surface() {
    let dir = scratch("usage");
    // No mode, no paths, an unknown flag → the usage page, exit 2.
    let (code, text) = heal(&dir, &["--definitely-not-a-flag"]);
    assert_eq!(code, Some(2));
    assert!(
        !text.contains("cargo refine mine") && !text.contains("cargo refine unmine"),
        "the USAGE block must not teach the removed commands. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
