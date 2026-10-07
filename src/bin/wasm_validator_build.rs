//! Plan 199 T3.2/T3.3 — the validator authoring lane's build step.
//!
//! Builds every validator source crate under `validator_src/` for
//! wasm32-unknown-unknown, then installs each through the committed-
//! artifact store (`assets/validators/manifest.json`: version bump, atomic
//! manifest rename, byte-identical reinstall refused) — admission-verified
//! and smoke-run before anything is written, so a module the lane would
//! refuse never reaches the corpus.
//!
//! ```text
//! cargo run --features wasm_validator --bin wasm-validator-build            # build + install + verify
//! cargo run --features wasm_validator --bin wasm-validator-build -- --check # verify the committed artifacts only
//! ```
//!
//! Run from the repo root. `--check` is the CI-able posture (no cargo build
//! of the wasm crates — it loads + certifies what is committed through the
//! store).

use std::path::Path;
use std::process::Command;

use riir_refine::validators::{ValidatorStore, DEFAULT_FUEL};

/// One validator in the authoring lane. Adding a validator = one row here +
/// a crate under `validator_src/` compiling against the fixed ABI.
struct ValidatorSpec {
    /// Source crate dir (repo-root relative), a standalone (workspace-
    /// excluded) cdylib crate.
    src_dir: &'static str,
    /// The cdylib artifact cargo emits (crate name, `_`-joined).
    artifact: &'static str,
    /// Store name (the manifest key + `<name>.wasm`).
    name: &'static str,
}

const VALIDATORS: &[ValidatorSpec] = &[ValidatorSpec {
    src_dir: "validator_src/useless_format_brace",
    artifact: "validator_useless_format_brace",
    name: "useless_format_brace",
}];

/// The committed store root (repo-root relative).
const STORE_ROOT: &str = "assets/validators";

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");
    if check_only {
        if let Err(e) = check_all() {
            eprintln!("✗ check failed: {e}");
            std::process::exit(1);
        }
        println!("wasm-validator-build: check-only pass over the committed store");
        return;
    }
    let mut failed = false;
    for spec in VALIDATORS {
        if let Err(e) = build_and_install(spec) {
            eprintln!("✗ {}: {e}", spec.name);
            failed = true;
        }
    }
    if let Err(e) = check_all() {
        eprintln!("✗ post-install verify: {e}");
        failed = true;
    }
    if failed {
        std::process::exit(1);
    }
}

fn build_and_install(spec: &ValidatorSpec) -> Result<(), String> {
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

    let src = Path::new(spec.src_dir)
        .join("target/wasm32-unknown-unknown/release")
        .join(format!("{}.wasm", spec.artifact));
    let bytes = std::fs::read(&src).map_err(|e| format!("read {}: {e}", src.display()))?;
    let size = bytes.len();
    let digest = blake3::hash(&bytes).to_string();

    // Smoke the fixed ABI end to end BEFORE the store sees the bytes: a
    // brace-free span passes, a format-args span is refused with the brace
    // count as its score.
    probe_root_probe(&bytes)?;

    let store = ValidatorStore::new(STORE_ROOT);
    match store.install(spec.name, &bytes) {
        Ok(record) => println!(
            "✓ {} — {} bytes, blake3 {}, installed as v{}",
            spec.name, size, digest, record.version
        ),
        Err(riir_refine::validators::StoreError::AlreadyCurrent) => println!(
            "✓ {} — {} bytes, blake3 {}, already current (no version bump)",
            spec.name, size, digest
        ),
        Err(e) => return Err(format!("install refused: {e}")),
    }
    Ok(())
}

/// Offline smoke over raw bytes through the same admission + ABI fixtures
/// the store's install path enforces — kept out of the store itself so the
/// store stays a pure manifest/artifact surface.
fn probe_root_probe(bytes: &[u8]) -> Result<(), String> {
    let module = riir_refine::validators::ValidatorModule::admit(bytes)
        .map_err(|e| format!("admission refused: {e}"))?;
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
    Ok(())
}

fn check_all() -> Result<(), String> {
    let store = ValidatorStore::new(STORE_ROOT);
    let admitted = store.verify().map_err(|e| e.to_string())?;
    for a in admitted {
        println!(
            "✓ {} v{} — blake3 {}, state {:?}",
            a.record.name, a.record.version, a.record.blake3, a.record.state
        );
    }
    Ok(())
}
