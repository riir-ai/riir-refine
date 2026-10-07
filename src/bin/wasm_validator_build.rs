//! Plan 199 T3.2 — the validator authoring lane's build step.
//!
//! Builds every validator source crate under `validator_src/` for
//! wasm32-unknown-unknown, installs the artifacts into `assets/validators/`
//! (BLAKE3-stamped), then admission-verifies + smoke-runs each through the
//! in-process pipeline — build and verify in one command, so a module the
//! admission gate would refuse never lands in the corpus.
//!
//! ```text
//! cargo run --features wasm_validator --bin wasm-validator-build           # build + verify + install
//! cargo run --features wasm_validator --bin wasm-validator-build -- --check # verify the committed artifacts only
//! ```
//!
//! Run from the repo root. `--check` is the CI-able posture (no cargo build
//! of the wasm crates — it reads + certifies what is committed).

use std::path::Path;
use std::process::Command;

use riir_refine::validators::{ValidatorModule, DEFAULT_FUEL};

/// One validator in the authoring lane. Adding a validator = one row here +
/// a crate under `validator_src/` compiling against the fixed ABI.
struct ValidatorSpec {
    /// Source crate dir (repo-root relative), a standalone (workspace-
    /// excluded) cdylib crate.
    src_dir: &'static str,
    /// The cdylib artifact cargo emits (crate name, `_`-joined).
    artifact: &'static str,
    /// Installed artifact name under `assets/validators/`.
    name: &'static str,
}

const VALIDATORS: &[ValidatorSpec] = &[ValidatorSpec {
    src_dir: "validator_src/useless_format_brace",
    artifact: "validator_useless_format_brace",
    name: "useless_format_brace",
}];

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");
    let mut failed = false;
    for spec in VALIDATORS {
        if let Err(e) = process(spec, check_only) {
            eprintln!("✗ {}: {e}", spec.name);
            failed = true;
        }
    }
    if failed {
        std::process::exit(1);
    }
    println!(
        "wasm-validator-build: {} validator(s) verified",
        VALIDATORS.len()
    );
}

fn process(spec: &ValidatorSpec, check_only: bool) -> Result<(), String> {
    if !check_only {
        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--target",
                "wasm32-unknown-unknown",
                "--manifest-path",
                &format!("{}/Cargo.toml", spec.src_dir),
            ])
            .status()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;
        if !status.success() {
            return Err("wasm build failed".into());
        }
    }

    let src = Path::new(spec.src_dir)
        .join("target/wasm32-unknown-unknown/release")
        .join(format!("{}.wasm", spec.artifact));
    let bytes = std::fs::read(&src).map_err(|e| format!("read {}: {e}", src.display()))?;
    let digest = blake3::hash(&bytes);
    let size = bytes.len();

    // The admission gate runs BEFORE anything is installed: a module the
    // lane would refuse never reaches the corpus.
    let module = ValidatorModule::admit(&bytes)
        .map_err(|e| format!("admission refused: {e}"))?;

    // Smoke the fixed ABI end to end: a brace-free span passes, a
    // format-args span is refused with the brace count as its score.
    let clean = module
        .run_once(b"format!(\"hello\")", DEFAULT_FUEL)
        .map_err(|e| format!("clean-span smoke trapped: {e:?}"))?;
    if clean.verdict != 0 {
        return Err(format!("clean-span smoke returned verdict {}", clean.verdict));
    }
    let braced = module
        .run_once(b"format!(\"{}\", x)", DEFAULT_FUEL)
        .map_err(|e| format!("braced-span smoke trapped: {e:?}"))?;
    if braced.verdict != 1 || braced.score != 2 {
        return Err(format!(
            "braced-span smoke returned verdict {} score {} (want 1 / 2)",
            braced.verdict, braced.score
        ));
    }

    if !check_only {
        let dest_dir = Path::new("assets/validators");
        std::fs::create_dir_all(dest_dir).map_err(|e| format!("mkdir: {e}"))?;
        let dest = dest_dir.join(format!("{}.wasm", spec.name));
        std::fs::write(&dest, &bytes).map_err(|e| format!("write {}: {e}", dest.display()))?;
    }

    println!(
        "✓ {} — {} bytes, blake3 {}{}",
        spec.name,
        size,
        digest,
        if check_only { " (check-only)" } else { " installed" }
    );
    Ok(())
}
