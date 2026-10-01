//! Issue 119 e2e (the yield-session delta): `--coverage-report`'s two
//! exit contracts, pinned at the PROCESS boundary against the real binary
//! and the real toolchain.
//!
//! The landing commit (`300411ed`) verified both contracts LIVE but
//! committed no `tests/` arm — this file is that arm (the repo's own law:
//! a gate nobody runs is documentation). The lib fixtures in
//! `src/corpus_coverage/tests.rs` cover the pure arithmetic; these two
//! tests cover what only the process boundary can — the spawn, the env
//! override, the exit codes.
//!
//! 1. Default driver (`clippy-driver` on PATH via rustup — CI images and
//!    dev boxes carry it): exit 0, the structural report lines, and a
//!    nonzero compiled-fix count (a zero would mean the diff itself went
//!    blind). NO live numbers are pinned — the report is
//!    toolchain-derived by design; only structure is contractual.
//! 2. `RIIR_HEAL_CLIPPY_DRIVER` naming a nonexistent binary: exit 1 + the
//!    BLIND line naming the remedy knob (the frontier-report law — never
//!    a green zero over a blind derivation).
#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))] // the bin's required-features — CARGO_BIN_EXE_cargo-refine exists only under them

use std::process::Command;

fn heal_bin() -> &'static str {
    env!("CARGO_BIN_EXE_cargo-refine")
}

#[test]
fn coverage_report_runs_against_the_installed_toolchain() {
    let out = Command::new(heal_bin())
        .arg("--coverage-report")
        .env_remove("RIIR_HEAL_CLIPPY_DRIVER")
        .env("RIIR_HEAL_KAT_METER", "0")
        .env("RIIR_HEAL_KAT_GATE", "0")
        .output()
        .expect("spawn cargo-refine");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "expected exit 0, got {:?}\nstdout:\n{stdout}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("corpus coverage-debt report (Issue 119)"),
        "the report header"
    );
    assert!(
        stdout.contains("toolchain      clippy"),
        "the version stamp is derived from the installed driver"
    );
    assert!(
        stdout.contains("UNCOVERED"),
        "the debt headline"
    );
    assert!(
        stdout.contains("uncovered by group"),
        "the group-debt table"
    );
    assert!(
        stdout.contains("frequency-ranked head"),
        "the evidence-join section"
    );
    // A blind diff would report zero fix-lane lints; the compiled-in
    // `FIXABLE_LINTS ∪ VERIFY_LINTS` is nonzero by construction — the
    // line's exact count moves with the corpus, zero does not.
    let n = stdout
        .lines()
        .find(|l| l.contains("compiled fix"))
        .and_then(|l| l.split_whitespace().nth(2))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    assert!(n > 0, "the compiled fix lane must classify ≥1 lint");
}

#[test]
fn coverage_report_refuses_loudly_without_a_driver() {
    let out = Command::new(heal_bin())
        .arg("--coverage-report")
        .env("RIIR_HEAL_CLIPPY_DRIVER", "definitely-not-a-clippy-driver-119")
        .env("RIIR_HEAL_KAT_METER", "0")
        .env("RIIR_HEAL_KAT_GATE", "0")
        .output()
        .expect("spawn cargo-refine");
    assert_eq!(
        out.status.code(),
        Some(1),
        "a blind derivation refuses — never a green zero"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("BLIND"),
        "the refusal is the BLIND posture, in-band on the report:\n{stdout}"
    );
    assert!(
        stdout.contains("RIIR_HEAL_CLIPPY_DRIVER"),
        "the refusal names the remedy knob:\n{stdout}"
    );
}
