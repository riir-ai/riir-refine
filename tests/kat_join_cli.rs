//! riir-dapps Plan 045 Phase A1/A2 — the first-join environment choice, AT
//! THE PROCESS BOUNDARY (the `repo_posture_cli.rs` hermetic pattern).
//!
//! The class this guards: the A1 prompt must fire ONLY on the no-config
//! first-join path. An existing machine (config record OR env override)
//! must see ZERO new output and ZERO new refusals — the consent /
//! soft-gate flows stay byte-identical — while a fresh non-TTY join
//! refuses loud with the env remedy instead of silently riding the old
//! mainnet default or hanging on a closed pipe.
//!
//! | arm | asserted |
//! |---|---|
//! | env force devnet | `RIIR_REFINE_JOIN_ENV=devnet` records the devnet URL through the config-set record; the recorded line prints |
//! | env force mainnet | records mainnet + the A3 posture copy names every unarmed plane (pool 0, TUNA, crank, settles nothing) |
//! | `--yes` | no env, no TTY → devnet recorded (the bracketed default) |
//! | no env, no `--yes`, non-TTY | exits **2**, names `RIIR_REFINE_JOIN_ENV`, records NOTHING |
//! | invalid env value | exits **2**, names the value AND the `devnet\|mainnet` set, records NOTHING |
//! | existing config | exits 0 (login proceeds), NEVER prompts, the record is untouched |
//! | `config get` after a join | reports the recorded URL with `source: config` (the same record `config set` writes) |
//!
//! Hermetic: `RIIR_REFINE_KAT_ACCOUNT_DIR` isolates the whole account dir
//! (the join record, the keys, the grant projection), the service-url env
//! overrides are REMOVED (an inherited value would read as configured and
//! silently skip the prompt arm), and stdin is a pipe — the non-TTY arm
//! needs it and the TTY-less refusal must be the default shape a CI
//! process sees.

#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))]

#[path = "support/hermetic_home.rs"]
mod hermetic_home;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The config-set record's file name (the ONE record format — the join
/// writes through `config set`'s writer).
const RECORD_FILE: &str = "service_url.json";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_join_cli_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One `cargo refine <args>` with the hermetic join env. `extra_env` wins
/// over the removals (applied after). stdin is ALWAYS a pipe — the
/// non-TTY refusal is the shape every spawned process sees, and the arms
/// that should never prompt must prove it by refusing, not hanging.
fn run_heal(
    dir: &Path,
    extra_env: &[(&str, &str)],
    args: &[&str],
) -> (Option<i32>, String, String) {
    let out = {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-refine"));
        cmd.args(args)
            .env("RIIR_REFINE_KAT_ACCOUNT_DIR", dir.display().to_string())
            .envs(hermetic_home::vars(dir.display().to_string()))
            .env("RIIR_AUTH_ACCOUNT_DIR", dir.display().to_string())
            // The join decision must not see an inherited service-url
            // value: any-source-is-configured would skip the prompt arm
            // silently.
            .env_remove("RIIR_REFINE_KAT_SERVICE_URL")
            .env_remove("RIIR_REFINE_JOIN_ENV")
            .env_remove("RIIR_REFINE_YES")
            .stdin(Stdio::null());
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        cmd.output().expect("spawn cargo-refine")
    };
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn record_url(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join(RECORD_FILE))
        .ok()
        .map(|text| {
            text.lines()
                .find_map(|l| l.trim().strip_prefix("\"url\": \"")?.strip_suffix("\","))
                .unwrap_or_default()
                .to_string()
        })
}

#[test]
fn join_env_forces_devnet_through_the_config_set_record() {
    let dir = scratch("devnet");
    let (code, stdout, stderr) = run_heal(&dir, &[("RIIR_REFINE_JOIN_ENV", "devnet")], &["login"]);
    assert_eq!(code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    assert_eq!(
        record_url(&dir).as_deref(),
        Some("https://devnet.gist.rs"),
        "the choice records through the config-set record"
    );
    assert!(
        stdout.contains(
            "recorded to config: devnet (bookkeeping — commands dial mainnet by default; cargo refine --net devnet targets devnet)"
        ),
        "the recorded line: {stdout}"
    );
    assert!(
        !stdout.contains("had an account key already"),
        "a brand-new machine (no key seeded) keeps the proposal transcript byte-identical: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_existing_account_key_makes_the_switch_visible_in_the_join_output() {
    // The verdict's join-visibility clause: a machine with an account key
    // but NO service config was silently riding the mainnet default — the
    // recorded line must name that switch. The key is a REAL ssh key so
    // the login chain after the join proceeds normally.
    let dir = scratch("had_key");
    let work = scratch("had_key_work");
    let key = work.join("id_ed25519");
    let gen = Command::new("ssh-keygen")
        .args([
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "join-cli-clause",
            "-f",
        ])
        .arg(&key)
        .stdin(Stdio::null())
        .output()
        .expect("spawn ssh-keygen");
    assert!(
        gen.status.success(),
        "ssh-keygen failed: {}",
        String::from_utf8_lossy(&gen.stderr)
    );
    std::fs::copy(&key, dir.join("id_ed25519")).expect("seed the account key");
    let (code, stdout, stderr) = run_heal(&dir, &[("RIIR_REFINE_JOIN_ENV", "devnet")], &["login"]);
    assert_eq!(code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    assert_eq!(
        record_url(&dir).as_deref(),
        Some("https://devnet.gist.rs"),
        "the choice still records devnet"
    );
    let recorded_line = stdout
        .lines()
        .find(|l| l.starts_with("recorded to config"))
        .expect("the recorded line prints");
    assert!(
        recorded_line.starts_with(
            "recorded to config: devnet (bookkeeping — commands dial mainnet by default; cargo refine --net devnet targets devnet)"
        ),
        "the base line is unchanged: {recorded_line}"
    );
    assert!(
        recorded_line.contains(
            "(this machine had an account key already; the pre-join default was mainnet)"
        ),
        "the clause is on the recorded line: {recorded_line}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn join_env_forces_mainnet_and_names_every_unarmed_plane() {
    let dir = scratch("mainnet");
    let (code, stdout, stderr) = run_heal(&dir, &[("RIIR_REFINE_JOIN_ENV", "mainnet")], &["login"]);
    assert_eq!(code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    assert_eq!(
        record_url(&dir).as_deref(),
        Some(riir_refine::kat_service_urls::MAINNET_SERVICE_URL),
        "mainnet records the mainnet API host (riir-dapps Plan 047)"
    );
    for needle in [
        "recorded to config: mainnet (bookkeeping — commands dial mainnet by default; cargo refine --net mainnet targets mainnet)",
        "mainnet posture",
        "reward pool is 0",
        "TUNA trial credit is unconfigured",
        "crank",
        "503 fail-closed",
        "settles nothing",
    ] {
        assert!(
            stdout.contains(needle),
            "the mainnet posture copy must name {needle}: {stdout}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn yes_answers_the_mainnet_default_without_a_tty() {
    // The 2026-10-12 first-join flip (decide(): --yes → Mainnet) — this e2e
    // was missed in that commit and had been red at HEAD since (caught
    // 2026-10-14 by the dialing-law work, which re-ran the suite whole).
    let dir = scratch("yes");
    let (code, stdout, stderr) = run_heal(&dir, &[], &["--yes", "login"]);
    assert_eq!(code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    assert_eq!(
        record_url(&dir).as_deref(),
        Some(riir_refine::kat_service_urls::MAINNET_SERVICE_URL),
        "--yes takes the mainnet default (the 2026-10-12 flip + the dialing law)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn non_tty_without_force_or_yes_refuses_and_records_nothing() {
    let dir = scratch("refuse");
    let (code, stdout, stderr) = run_heal(&dir, &[], &["login"]);
    assert_eq!(
        code,
        Some(2),
        "a fresh non-TTY join refuses; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("RIIR_REFINE_JOIN_ENV"),
        "the remedy names the env var: {stderr}"
    );
    assert!(
        stderr.contains("--yes"),
        "the remedy names the flag: {stderr}"
    );
    assert!(
        !stdout.contains("Join which kat network"),
        "no prompt raced the refusal: {stdout}"
    );
    assert!(
        !dir.join(RECORD_FILE).exists(),
        "a refused join records nothing"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn invalid_env_value_refuses_loud_and_records_nothing() {
    let dir = scratch("badenv");
    let (code, _stdout, stderr) =
        run_heal(&dir, &[("RIIR_REFINE_JOIN_ENV", "staging")], &["login"]);
    assert_eq!(code, Some(2), "stderr: {stderr}");
    assert!(
        stderr.contains("staging") && stderr.contains("mainnet|devnet"),
        "the error names the value AND the valid set: {stderr}"
    );
    assert!(
        !dir.join(RECORD_FILE).exists(),
        "an invalid force records nothing"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_existing_config_is_never_prompted_and_never_rewritten() {
    let dir = scratch("existing");
    std::fs::write(
        dir.join(RECORD_FILE),
        "{\n  \"version\": 1,\n  \"url\": \"https://devnet.gist.rs\",\n  \"recorded_unix_secs\": 0\n}\n",
    )
    .expect("seed the record the config-set shape");
    // NO env force, NO --yes, non-TTY: the pre-A1 posture for a configured
    // machine — the login must proceed (exit 0) with NO prompt and NO
    // refusal, and the record stays exactly as seeded.
    let (code, stdout, stderr) = run_heal(&dir, &[], &["login"]);
    assert_eq!(
        code,
        Some(0),
        "a configured machine is never refused: stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !stdout.contains("Join which kat network"),
        "no prompt for a configured machine: {stdout}"
    );
    assert!(
        !stderr.contains("RIIR_REFINE_JOIN_ENV"),
        "no join refusal for a configured machine: {stderr}"
    );
    assert!(
        !stdout.contains("recorded to config"),
        "no rewrite of an existing record: {stdout}"
    );
    assert_eq!(
        record_url(&dir).as_deref(),
        Some("https://devnet.gist.rs"),
        "the existing record is untouched"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_get_reports_the_joined_record() {
    let dir = scratch("configget");
    let (code, _stdout, stderr) = run_heal(&dir, &[("RIIR_REFINE_JOIN_ENV", "devnet")], &["login"]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    // Issue 126 follow-up 3 (the 2026-10-14 dialing law): the joined row is
    // BOOKKEEPING — `config get` shows the DIAL (mainnet, source: default)
    // and the stored row as BOOKKEEPING ONLY, never as the steering source.
    let (code, stdout, stderr) = run_heal(&dir, &[], &["config", "get"]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stdout.contains("https://devnet.gist.rs"), "url: {stdout}");
    assert!(
        stdout.contains("source: default"),
        "the dial reads as the mainnet default (a stored row never steers): {stdout}"
    );
    assert!(
        stdout.contains("BOOKKEEPING ONLY"),
        "the stored row is disclosed as bookkeeping: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
