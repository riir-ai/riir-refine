//! Issue 115 e2e: the auto-oracle's feature-knob re-aim, proven on a real
//! scratch crate + the real binary — the P15 specimen shape one lane over.
//!
//! The specimen's ONLY diagnostic lives behind a non-default feature:
//! 1. WITHOUT the knob, the auto-oracle lints at default features → the
//!    gated fn compiles to nothing → the span falls back to `[heuristic]`
//!    (the exact blindness class the issue files).
//! 2. WITH `RIIR_HEAL_AUTO_ORACLE_FEATURES=gated`, the spawn carries
//!    `--features gated` → `needless_return` fires → the span is
//!    `[oracle]`-anchored ahead of ranking.
//!
//! Run A additionally asserts the loud clean-tree line names the DEFAULT
//! posture (the run genuinely was clean there); run B asserts the anchor
//! arrived, so the knob is the only difference between the two runs.
#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))] // the bin's required-features — CARGO_BIN_EXE_cargo-heal exists only under them

use std::path::{Path, PathBuf};
use std::process::Command;

const LIB: &str = r#"//! Issue 115 specimen: the only diagnostic is feature-gated.
#[cfg(feature = "gated")]
pub fn answer() -> u32 {
    return 42;
}
"#;

fn scratch_root() -> PathBuf {
    let d = std::env::temp_dir().join(format!("riir_i115_oracle_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    // Store-root marker (resolve_heal_root walks to `.git`) — keeps the
    // spawned binary's self-evolve/store writes inside the scratch dir.
    std::fs::create_dir_all(d.join(".git")).unwrap();
    std::fs::write(
        d.join("Cargo.toml"),
        "[package]\nname = \"riir_i115_scratch\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ngated = []\n",
    )
    .unwrap();
    std::fs::write(d.join("src").join("lib.rs"), LIB).unwrap();
    d
}

/// One suggest run; `features` toggles the Issue-115 knob. The billing plane
/// is hard-off for both runs — a test must never touch a real fleet account,
/// and a network tail must never decide a marker assertion.
fn run_suggest(bin: &str, dir: &Path, features: Option<&str>) -> String {
    let mut cmd = Command::new(bin);
    cmd.args(["--suggest", "--no-fmt"])
        .arg(dir)
        .env_remove("RIIR_HEAL_AUTO_ORACLE") // default posture, explicitly
        .env("RIIR_HEAL_KAT_METER", "0")
        .env("RIIR_HEAL_KAT_GATE", "0")
        // The oracle's clippy spawn builds THIS scratch crate; keep its
        // target dir inside the scratch regardless of how the test harness
        // was invoked (a harness-level CARGO_TARGET_DIR is inherited).
        .env("CARGO_TARGET_DIR", dir.join("target"));
    match features {
        Some(csv) => {
            cmd.env("RIIR_HEAL_AUTO_ORACLE_FEATURES", csv);
        }
        None => {
            cmd.env_remove("RIIR_HEAL_AUTO_ORACLE_FEATURES");
        }
    }
    let out = cmd.output().expect("spawn cargo-heal");
    format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn auto_oracle_features_knob_anchors_gated_diagnostics() {
    let bin = env!("CARGO_BIN_EXE_cargo-heal");

    // Run A — default posture: the gated diagnostic is INVISIBLE.
    let dir_a = scratch_root();
    let out_a = run_suggest(bin, &dir_a, None);
    assert!(
        out_a.contains("clean tree at default features"),
        "run A must report the honest default-features clean tree; output:\n{out_a}"
    );
    assert!(
        out_a.contains("[heuristic]"),
        "run A: the gated span must fall back to heuristic ranking — the Issue-115 blindness;\noutput:\n{out_a}"
    );
    assert!(
        !out_a.contains("oracle finding"),
        "run A must anchor NOTHING (the only diagnostic is feature-gated away);\noutput:\n{out_a}"
    );
    let _ = std::fs::remove_dir_all(&dir_a);

    // Run B — the knob re-aims the spawn: the same span is oracle-anchored.
    let dir_b = scratch_root();
    let out_b = run_suggest(bin, &dir_b, Some("gated"));
    assert!(
        out_b.contains("linting") && out_b.contains("with features [gated]"),
        "run B must print the loud knob-active status line; output:\n{out_b}"
    );
    assert!(
        out_b.contains("[oracle]"),
        "run B: the gated span must be oracle-anchored; output:\n{out_b}"
    );
    assert!(
        out_b.contains("needless_return") && out_b.contains("oracle finding"),
        "run B: needless_return must surface as an oracle finding; output:\n{out_b}"
    );
    assert!(
        !out_b.contains("[heuristic]"),
        "run B: nothing should remain heuristic — the only span is anchored;\noutput:\n{out_b}"
    );
    let _ = std::fs::remove_dir_all(&dir_b);
}
