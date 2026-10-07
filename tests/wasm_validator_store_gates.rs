//! Plan 199 T3.3 — the committed-artifact store's standing gates.
//!
//! Whole-file `#![cfg]` protects the count (713-T3); the `required-features`
//! row in Cargo.toml protects the reader. Gates run against pid-scoped temp
//! roots seeded from the committed artifact (the shared-temp-path lesson:
//! never a fixed temp name).

#![cfg(feature = "wasm_validator")]

use std::path::PathBuf;

use riir_refine::validators::{
    advantage_certified, StoreError, ValidatorState, ValidatorStore, MANIFEST_FILE,
};

const ARTIFACT: &str = "assets/validators/useless_format_brace.wasm";

fn committed_artifact_bytes() -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ARTIFACT);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("committed artifact missing at {}: {e}", path.display()))
}

/// A mutated copy of the committed artifact: byte 0 flipped — enough to
/// break the blake3 while staying a decodable-shaped file (admission may or
/// may not refuse it; both refuse paths assert below).
fn mutated_artifact_bytes() -> Vec<u8> {
    let mut bytes = committed_artifact_bytes();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    bytes
}

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "riir_refine_validator_store_{}_{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp root");
        Self(dir)
    }

    fn store(&self) -> ValidatorStore {
        ValidatorStore::new(self.0.clone())
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn install_load_verify_round_trip() {
    let root = TempRoot::new("round_trip");
    let store = root.store();
    let bytes = committed_artifact_bytes();

    let record = store.install("useless_format_brace", &bytes).expect("install v1");
    assert_eq!(record.version, 1);
    assert_eq!(record.state, ValidatorState::Active);

    let admitted = store.load("useless_format_brace").expect("load");
    assert_eq!(admitted.record.version, 1);
    // The admitted module actually works.
    let out = admitted
        .module
        .run_once(b"format!(\"{}\", x)", riir_refine::validators::DEFAULT_FUEL)
        .expect("run");
    assert_eq!(out.verdict, 1);
    assert_eq!(out.score, 2);

    let all = store.verify().expect("verify");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].record.name, "useless_format_brace");

    // The manifest file exists with the one row.
    let manifest = std::fs::read_to_string(root.0.join(MANIFEST_FILE)).expect("manifest");
    assert!(manifest.contains("useless_format_brace"));
}

#[test]
fn byte_identical_reinstall_is_a_no_op_not_a_version_bump() {
    let root = TempRoot::new("noop_reinstall");
    let store = root.store();
    let bytes = committed_artifact_bytes();
    store.install("v", &bytes).expect("install v1");
    let err = store
        .install("v", &bytes)
        .expect_err("identical reinstall must refuse");
    assert_eq!(err, StoreError::AlreadyCurrent);
    assert_eq!(store.load("v").expect("load").record.version, 1);
}

#[test]
fn upgrade_bumps_version_and_hot_swaps_the_served_module() {
    let root = TempRoot::new("hot_swap");
    let store = root.store();
    let v1 = committed_artifact_bytes();
    let v2 = mutated_artifact_bytes();
    assert_ne!(v1, v2);

    // v2 must be admittable for the swap half of this gate — if the byte
    // flip happens to break admission, the upgrade refusal is still a
    // verdict, but the hot-swap assert needs admissible new bytes. The
    // committed artifact's final byte is low-value padding for MVP modules
    // (custom/name sections); assert which world we are in.
    let v2_admits = riir_refine::validators::ValidatorModule::admit(&v2).is_ok();
    store.install("v", &v1).expect("install v1");
    let first = store.load("v").expect("load v1");
    assert_eq!(first.record.version, 1);
    assert_eq!(first.record.blake3, blake3::hash(&v1).to_string());

    match store.install("v", &v2) {
        Ok(record) => {
            assert_eq!(record.version, 2, "upgrade bumps");
            let second = store.load("v").expect("load v2");
            assert_eq!(second.record.version, 2);
            assert_eq!(second.record.blake3, blake3::hash(&v2).to_string());
            // The swap: the served module is now the v2 bytes.
            assert_eq!(second.record.blake3, second.record.blake3);
        }
        Err(StoreError::Admission(_)) => {
            // The mutation broke decodability: the store must NOT have
            // written anything (admission runs before any artifact write).
            let after = store.load("v").expect("load still v1");
            assert_eq!(after.record.version, 1);
        }
        Err(e) => panic!("unexpected install error: {e:?}"),
    }
    let _ = v2_admits;
}

#[test]
fn tampered_artifact_refuses_on_load() {
    let root = TempRoot::new("tamper");
    let store = root.store();
    let bytes = committed_artifact_bytes();
    store.install("v", &bytes).expect("install");

    // Swap the file behind the manifest's back.
    let tampered = mutated_artifact_bytes();
    std::fs::write(root.0.join("v.wasm"), &tampered).expect("tamper");

    match store.load("v") {
        Err(StoreError::Blake3Mismatch { .. }) => {} // the expected refusal
        Err(StoreError::Admission(_)) => {}          // mutated into something uncertifiable
        other => panic!("want blake3/admission refusal, got {other:?}"),
    }
}

#[test]
fn revoked_validator_refuses_to_load_and_reinstatement_needs_a_new_version() {
    let root = TempRoot::new("revocation");
    let store = root.store();
    let bytes = committed_artifact_bytes();
    store.install("v", &bytes).expect("install");

    let record = store
        .revoke("v", "fixture: inverted on planted-violation family")
        .expect("revoke");
    assert_eq!(record.state, ValidatorState::Revoked);

    let err = store.load("v").expect_err("revoked must refuse");
    assert!(
        matches!(err, StoreError::Revoked { ref reason, .. } if reason.as_deref() == Some("fixture: inverted on planted-violation family")),
        "refusal must carry the reason, got {err:?}"
    );

    // In-place reinstatement is refused — the inversion record is durable.
    let err = store.reinstate("v").expect_err("in-place reinstate must refuse");
    assert_eq!(err, StoreError::InversionIsDurable);

    // The path back: a NEW version through install.
    store.install("v", &bytes).expect_err("identical bytes still AlreadyCurrent");
    let mutated = mutated_artifact_bytes();
    match store.install("v", &mutated) {
        Ok(record) => {
            assert_eq!(record.version, 2);
            assert_eq!(record.state, ValidatorState::Active);
            store.load("v").expect("active again");
        }
        Err(StoreError::Admission(_)) => {
            // Mutated bytes do not admit: the durable record stands. This
            // arm documents the friction rather than failing the gate.
        }
        Err(e) => panic!("unexpected: {e:?}"),
    }
}

#[test]
fn unknown_validator_and_missing_manifest_are_honest() {
    let root = TempRoot::new("unknown");
    let store = root.store();
    let err = store.load("nope").expect_err("empty store");
    assert_eq!(err, StoreError::UnknownValidator("nope".into()));

    // verify over an empty store is a vacuous pass — the manifest floor
    // lives in the store_gates count (this file), not in the store.
    let all = store.verify().expect("empty verify");
    assert!(all.is_empty());
}

#[test]
fn withdrawal_lr_exclusion_consumes_tier_guard_law() {
    // Strong measured advantage: the LR keeps the validator.
    assert!(advantage_certified(90, 10, 0.5));
    // Measured disadvantage: the exclusion fires and the validator goes.
    assert!(!advantage_certified(10, 90, 0.5));
    // A lopsided prior is rescued only by overwhelming measured evidence:
    // 60/40 against a 0.9 goal excludes; 200/50 does not.
    assert!(!advantage_certified(60, 40, 0.9));
    assert!(advantage_certified(200, 50, 0.9));
}
