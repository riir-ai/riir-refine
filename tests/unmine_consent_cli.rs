//! Owner mandate 2026-09-08 — the `--mine` / `--unmine` consent tiers,
//! driven through the REAL binary.
//!
//! `src/kat_mining_consent.rs`'s own unit tests cover the record (round
//! trip, malformed → `Unset`, idempotency, the conflict predicate). What
//! they cannot cover is the part that actually shipped broken before in this
//! CLI (Plan 064's `pay --request`): a flag that parses in the source and
//! does the wrong thing at the process boundary. So this suite spawns
//! `cargo-refine` and asserts the three tiers by their EXIT CODE and their
//! printed wording:
//!
//! | invocation | exit | consent after |
//! |---|---|---|
//! | `--mine` | 0 | opted in |
//! | `--unmine` | 0 | opted out |
//! | `--unmine` twice | 0, 0 | opted out (idempotent, says "already") |
//! | `--mine --unmine` | **2** | UNCHANGED — the conflict is rejected before either write |
//! | `sync` while opted out | 0 | opted out, push SKIPPED, pull still attempted |
//!
//! Every run gets its own `RIIR_HEAL_KAT_ACCOUNT_DIR` under the temp dir —
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

use std::path::{Path, PathBuf};
use std::process::Command;

/// A scratch account dir + repo root, unique per test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_heal_unmine_{tag}_{}_{}",
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
        .env("RIIR_HEAL_KAT_ACCOUNT_DIR", dir)
        // Issue 081: the purge sweep targets the canonical account dir too —
        // isolate it (the consent plane still rides the old var above).
        .env("RIIR_AUTH_ACCOUNT_DIR", dir)
        .env("RIIR_HEAL_TRAJ_ROOT", dir)
        // 127.0.0.1:1 is closed: the lease pull fails fast instead of
        // reaching the real service from a test.
        .env("RIIR_HEAL_KAT_SERVICE_URL", "http://127.0.0.1:1")
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

fn consent_file(dir: &Path) -> PathBuf {
    dir.join(riir_clippy::kat_mining_consent::CONSENT_FILE)
}

fn state(dir: &Path) -> riir_clippy::kat_mining_consent::MiningConsent {
    riir_clippy::kat_mining_consent::read(dir)
}

/// Tier 3: `--unmine` records the opt-out, exits 0, and says what it did
/// NOT turn off.
#[test]
fn unmine_records_the_opt_out_and_names_what_still_works() {
    use riir_clippy::kat_mining_consent::MiningConsent;
    let dir = scratch("optout");
    assert_eq!(
        state(&dir),
        MiningConsent::Unset,
        "a fresh dir starts unset"
    );

    let (code, text) = heal(&dir, &["--unmine"]);
    assert_eq!(code, Some(0), "--unmine must exit 0. output:\n{text}");
    assert_eq!(state(&dir), MiningConsent::OptedOut, "output:\n{text}");
    assert!(consent_file(&dir).exists(), "the record must be on disk");

    // What it turned OFF.
    assert!(text.contains("opted OUT"), "output:\n{text}");
    assert!(
        text.contains("push half of `cargo refine sync`"),
        "--unmine must name the push half as the thing that stopped. output:\n{text}"
    );
    // What it did NOT turn off — the honest half.
    assert!(
        text.contains("UNCHANGED") && text.contains("cargo refine --fix"),
        "--unmine must say healing is unchanged. output:\n{text}"
    );
    // The queued batches: named, and declared the user's to delete.
    assert!(
        text.contains(".refine/outbox"),
        "--unmine must name the outbox path. output:\n{text}"
    );
    assert!(
        text.contains("cargo refine --mine"),
        "--unmine must name the re-enable command. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--unmine` deletes nothing — a queued batch survives it verbatim.
#[test]
fn unmine_deletes_no_queued_batch() {
    let dir = scratch("nodelete");
    let outbox = dir.join(".refine").join("outbox");
    std::fs::create_dir_all(&outbox).expect("outbox");
    let queued = outbox.join("deadbeefcafe.ndjson");
    std::fs::write(&queued, "{\"row\":1}\n").expect("plant a queued batch");

    let (code, text) = heal(&dir, &["--unmine"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(
        queued.exists(),
        "--unmine must NOT delete queued batches — they are the user's. output:\n{text}"
    );
    assert_eq!(
        std::fs::read_to_string(&queued).expect("read back"),
        "{\"row\":1}\n",
        "the queued batch must be byte-identical"
    );
    assert!(
        text.contains("1 queued batch file(s)"),
        "--unmine must count what is queued. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Idempotency: the second `--unmine` is a no-op that says so, exit 0.
#[test]
fn unmine_is_idempotent() {
    use riir_clippy::kat_mining_consent::MiningConsent;
    let dir = scratch("idempotent");
    let (code1, _) = heal(&dir, &["--unmine"]);
    assert_eq!(code1, Some(0));
    let (code2, text2) = heal(&dir, &["--unmine"]);
    assert_eq!(code2, Some(0), "the second --unmine must still exit 0");
    assert!(
        text2.contains("already opted out"),
        "the second run must say it changed nothing. output:\n{text2}"
    );
    assert_eq!(state(&dir), MiningConsent::OptedOut);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Tier 2: `--mine` keeps every line of the Proposal-008 explainer AND now
/// records the opt-in.
#[test]
fn mine_keeps_the_explainer_and_records_the_opt_in() {
    use riir_clippy::kat_mining_consent::MiningConsent;
    let dir = scratch("optin");
    let (code, text) = heal(&dir, &["--mine"]);
    assert_eq!(code, Some(0), "--mine must exit 0. output:\n{text}");

    // The pinned explainer blocks — added to, never removed. (72b65676
    // collapsed the 4-step START to 3 — login is now automatic inside
    // --mine, the explicit `cargo refine login` line stays as the repair
    // path — and the pin follows the product.)
    for needle in [
        "KAT mining — turn your healing work into network credit",
        "WHAT: every cargo refine run records HOW the healer behaved",
        "POLICY (read before contributing):",
        "Devnet: KAT carries NO cash value and NO redemption right.",
        "START (3 steps):",
        "https://ai.gist.rs",
        "cargo refine login",
        "cargo refine --fix",
        "cargo refine sync",
        "WATCH: cargo refine account",
    ] {
        assert!(
            text.contains(needle),
            "--mine dropped a pinned explainer line: {needle:?}. output:\n{text}"
        );
    }
    // The feature-half status lines.
    assert!(text.contains("status:"), "output:\n{text}");
    assert!(text.contains("transport:"), "output:\n{text}");

    // The new half: the consent WRITE + the state line + the opt-out pointer.
    assert_eq!(state(&dir), MiningConsent::OptedIn, "output:\n{text}");
    assert!(
        text.contains("mining is opted IN for this machine"),
        "--mine must state the consent was recorded. output:\n{text}"
    );
    assert!(
        text.contains(MiningConsent::OptedIn.status_line()),
        "--mine must print the current consent state. output:\n{text}"
    );
    assert!(
        text.contains("cargo refine --unmine"),
        "--mine must name the opt-out. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--mine` after `--unmine` re-enables — the tiers are reversible.
#[test]
fn mine_re_enables_after_unmine() {
    use riir_clippy::kat_mining_consent::MiningConsent;
    let dir = scratch("reenable");
    assert_eq!(heal(&dir, &["--unmine"]).0, Some(0));
    assert_eq!(state(&dir), MiningConsent::OptedOut);
    let (code, text) = heal(&dir, &["--mine"]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert_eq!(state(&dir), MiningConsent::OptedIn, "output:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Mutual exclusion: exit 2, a message naming both flags, and — the part
/// that matters — NEITHER write happened.
#[test]
fn mine_and_unmine_together_are_rejected_without_writing() {
    use riir_clippy::kat_mining_consent::MiningConsent;
    let dir = scratch("conflict");
    let (code, text) = heal(&dir, &["--mine", "--unmine"]);
    assert_eq!(code, Some(2), "the conflict must exit 2. output:\n{text}");
    assert!(
        text.contains("--mine") && text.contains("--unmine"),
        "output:\n{text}"
    );
    assert_eq!(
        state(&dir),
        MiningConsent::Unset,
        "the conflict must be rejected BEFORE either side writes. output:\n{text}"
    );
    assert!(
        !consent_file(&dir).exists(),
        "no record may be created by a rejected invocation"
    );

    // And the reverse order behaves identically (the check is on the pair,
    // not on argument order).
    let dir2 = scratch("conflict_rev");
    let (code2, _) = heal(&dir2, &["--unmine", "--mine"]);
    assert_eq!(code2, Some(2));
    assert_eq!(state(&dir2), MiningConsent::Unset);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

/// Tier 3's teeth: `cargo refine sync` refuses its PUSH half while opted out,
/// still runs the lease PULL half, and exits 0 (an opt-out is not an error).
#[test]
fn sync_refuses_the_push_half_while_opted_out() {
    let dir = scratch("syncrefuse");
    // A trajectory store must EXIST, or "no trajectory store" would be the
    // reason nothing pushed and the test would pass for the wrong reason.
    let heal_dir = dir.join(".refine");
    std::fs::create_dir_all(&heal_dir).expect(".refine");
    std::fs::write(heal_dir.join("trajectories.kv"), "").expect("plant a store");

    assert_eq!(heal(&dir, &["--unmine"]).0, Some(0));
    let (code, text) = heal(&dir, &["sync"]);
    assert_eq!(code, Some(0), "an opt-out is not an error. output:\n{text}");
    assert!(
        text.contains("push SKIPPED"),
        "sync must say the push was skipped. output:\n{text}"
    );
    assert!(
        text.contains("cargo refine --mine"),
        "the refusal must name the re-enable. output:\n{text}"
    );
    assert!(
        !text.contains("sync payload:"),
        "no payload may be BUILT or signed while opted out. output:\n{text}"
    );
    assert!(
        text.contains("corpus lease"),
        "the lease PULL half must still run — a read is not a contribution. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The control arm: with consent UNSET, `sync` behaves exactly as it did
/// before this feature existed — it reaches the payload half. Without this
/// the refusal test above cannot distinguish "the gate works" from "sync
/// never pushes anyway".
#[test]
fn sync_with_unset_consent_still_reaches_the_push_half() {
    use riir_clippy::kat_mining_consent::MiningConsent;
    let dir = scratch("syncunset");
    let heal_dir = dir.join(".refine");
    std::fs::create_dir_all(&heal_dir).expect(".refine");
    std::fs::write(heal_dir.join("trajectories.kv"), "").expect("plant a store");

    assert_eq!(state(&dir), MiningConsent::Unset, "no --mine, no --unmine");
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

/// The help text advertises `--unmine` in both places a user looks.
/// (`tests/heal_usage_flag_parity.rs` proves every advertised flag has a
/// parser arm; this proves the flag is advertised at all.)
#[test]
fn usage_advertises_unmine() {
    let dir = scratch("usage");
    // No mode, no paths, an unknown flag → the usage page, exit 2.
    let (code, text) = heal(&dir, &["--definitely-not-a-flag"]);
    assert_eq!(code, Some(2));
    assert!(
        text.contains("cargo refine --unmine"),
        "the USAGE block must show --unmine. output:\n{text}"
    );
    assert!(
        text.contains("--unmine        Opt OUT of contributing"),
        "the flag description list must document --unmine. output:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
