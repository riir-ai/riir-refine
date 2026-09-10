//! P12 e2e: re-proposal suppression across runs.
//!
//! Proves the full loop on a real scratch crate + the real binary:
//! 1. `cargo-heal --fix --write` heals a `match_bool` shape (the pair
//!    (rule, restore, fix) lands in the crate-local store as an apply).
//! 2. The test seeds two consecutive compile-gate fails for that exact pair
//!    (what a `--verify` run records when the edit breaks the build twice).
//! 3. Restoring the original shape and re-running `--fix --write` must
//!    SUPPRESS the re-proposal: the file stays byte-identical and the
//!    suppression report prints. Without the suppression this run would
//!    deterministically re-apply — the katgpt-rs 20-edit/42-check churn.
//!
//! Seeding through the lib API (not a cargo run) is exact by construction:
//! the store row key is (lint_key, blake3(restore)) and the row's `fix`
//! text must match the proposal byte-for-byte — both halves are the same
//! strings the first run just produced (asserted before seeding).
#![cfg(all(feature = "latent_retrieval", feature = "fix_verify"))]

use std::path::{Path, PathBuf};
use std::process::Command;

const RULE: &str = "match_bool";
const RESTORE: &str = "match c { true => a, _ => b }";
const FIX: &str = "if c { a } else { b }";
const ORIGINAL: &str =
    "pub fn f(c: bool, a: u32, b: u32) -> u32 {\n    match c { true => a, _ => b }\n}\n";

fn scratch_root(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "riir_p12_suppress_{}_{}_{}",
        std::process::id(),
        tag,
        line!()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    // The store root resolves by walking up to `.git` — plant the marker so
    // the spawned binary's store and the test's seeded store are the SAME
    // store (resolve_heal_root contract).
    std::fs::create_dir_all(d.join(".git")).unwrap();
    d
}

fn write_lib(dir: &Path, text: &str) {
    std::fs::write(dir.join("src").join("lib.rs"), text).unwrap();
}

fn run_heal(bin: &str, dir: &PathBuf) -> String {
    let out = Command::new(bin)
        .args(["--fix", "--write", "--no-fmt"])
        .arg(dir)
        .env("RIIR_HEAL_SUPPRESS_REVERTED", "") // default: suppression ON
        .output()
        .expect("spawn cargo-heal");
    format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn p12_suppresses_a_twice_reverted_pair_across_runs() {
    let bin = env!("CARGO_BIN_EXE_cargo-heal");
    let dir = scratch_root("main");
    write_lib(&dir, ORIGINAL);

    // Run 1: the match_bool edit applies normally (streak 0 — the apply is
    // a sighting, not a verdict).
    let out1 = run_heal(bin, &dir);
    let healed = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(
        healed.contains(FIX),
        "run 1 must heal the match shape; output:\n{out1}\nfile:\n{healed}"
    );

    // Seed what two failed verify runs would have recorded for the exact
    // pair: two compile-gate fails (streak 2 = the suppression threshold).
    {
        let (store, bad) = riir_clippy::traj_store::TrajStore::open(&dir).unwrap();
        assert_eq!(bad, 0);
        store
            .record(RULE, RESTORE, FIX, false, false, "run-verify-1")
            .unwrap();
        store
            .record(RULE, RESTORE, FIX, false, false, "run-verify-2")
            .unwrap();
        assert_eq!(store.pair_consecutive_fails(RULE, RESTORE, FIX), 2);
    }

    // Restore the original shape (the operator re-runs on the unhealed
    // tree) and re-run.
    write_lib(&dir, ORIGINAL);
    let out2 = run_heal(bin, &dir);
    let after = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(
        after, ORIGINAL,
        "run 2 must NOT re-apply the twice-reverted pair; output:\n{out2}"
    );
    assert!(
        out2.contains("suppressed 1 previously-reverted edit(s)"),
        "the suppression report must name the skip; output:\n{out2}"
    );

    // The escape hatch: RIIR_HEAL_SUPPRESS_REVERTED=0 forces the retry.
    let out3 = Command::new(bin)
        .args(["--fix", "--write", "--no-fmt"])
        .arg(&dir)
        .env("RIIR_HEAL_SUPPRESS_REVERTED", "0")
        .output()
        .expect("spawn cargo-heal (kill-switch run)");
    let retried = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(
        retried.contains(FIX),
        "kill-switch must force the retry; output:\n{}",
        String::from_utf8_lossy(&out3.stdout)
    );

    let _ = std::fs::remove_dir_all(&dir);
}
