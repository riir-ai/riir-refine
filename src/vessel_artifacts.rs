//! Plan 202 R4: the vessel verify path — ONE artifact protocol across the
//! decision stack (instinct Proposal 001 A1/A8), consumed through the
//! public format repo's crate ([`reflexer_vessel`]).
//!
//! **Verify-only by law.** refine NEVER mints (A8 — minting is
//! riir-train's offline lane; this module has no signing surface) and
//! NEVER embeds vessel bytes (A10 — the compiled-in corpora stay
//! code-sealed; the artifact-dir leak test below pins the assets/ half,
//! and the escalation lane's binary leak scan is R2's test-spec row).
//!
//! **The pin posture** mirrors the reflex wildcard pattern: the compiled
//! pin table is EMPTY until the first release artifact ships (the
//! reflexer `DEFAULT_PINS` law — an emptied table must never silently
//! become the posture), and the operator wildcard
//! `RIIR_REFINE_VESSEL_PUBKEY` joins a 32-byte hex verifying key that
//! accepts any key-id the signature validates under. An unset wildcard is
//! a config gap: a loud refusal naming the env, never a silent pass.
//!
//! **The monotonic apply gate** is the vessel protocol's: strict
//! downgrade refusal, same-version-different-content fork refusal,
//! idempotent re-apply. State is in-memory here — persistence (the
//! artifact store half) is the R2 consumer's concern, not this seam's.

use reflexer_vessel::{
    ApplyRefusal, ApplyState, PinTable, VerifiedVessel, VesselError, MIN_ARTIFACT_VERSION,
};
use std::path::Path;

/// The operator wildcard env var (the reflex pattern: `RIIR_REFLEX_HEADS_PUBKEY`
/// is reflex's; this is refine's own name in the same shape).
pub const VESSEL_PUBKEY_ENV: &str = "RIIR_REFINE_VESSEL_PUBKEY";

/// Why [`operator_pins_from`] refused to build the operator posture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinError {
    /// The wildcard env is unset/empty — a config gap, loud by law.
    Unset,
    /// The wildcard env is set but not 64 hex chars (32 bytes).
    BadHex(String),
}

impl std::fmt::Display for PinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PinError::Unset => write!(
                f,
                "{VESSEL_PUBKEY_ENV} is unset — a vessel load would be unpinnable; \
                 set it to the minting key's 32-byte verifying key (64 hex chars)"
            ),
            PinError::BadHex(got) => write!(
                f,
                "{VESSEL_PUBKEY_ENV} is not 64 hex chars (got {got} chars) — \
                 it must be the minting key's 32-byte verifying key"
            ),
        }
    }
}

/// The pin-posture core, testable without env mutation (the R1
/// `is_armed_with` seam pattern — env races never enter these tests).
/// `wildcard_hex` is the env's value: `None`/empty refuses
/// ([`PinError::Unset`]); otherwise it must decode as 32 bytes of hex.
///
/// # Errors
///
/// [`PinError::Unset`] for a missing/blank wildcard (the config gap);
/// [`PinError::BadHex`] for anything that is not 64 hex chars decoding
/// to a curve point. Never panics: the length, alphabet, and curve
/// checks all route to [`PinError::BadHex`].
pub fn operator_pins_from(wildcard_hex: Option<&str>) -> Result<PinTable, PinError> {
    let hex = match wildcard_hex {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => return Err(PinError::Unset),
    };
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(PinError::BadHex(hex.to_string()));
    }
    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk).map_err(|_| PinError::BadHex(hex.to_string()))?;
        bytes[i] = u8::from_str_radix(pair, 16).map_err(|_| PinError::BadHex(hex.to_string()))?;
    }
    let key = reflexer_vessel::ed25519_dalek::VerifyingKey::from_bytes(&bytes)
        .map_err(|_| PinError::BadHex(hex.to_string()))?;
    Ok(PinTable::empty().with_wildcard(key))
}

/// The operator posture from the process env ([`VESSEL_PUBKEY_ENV`]).
///
/// # Errors
///
/// Same taxonomy as [`operator_pins_from`], reading the env as the
/// wildcard value.
pub fn operator_pins() -> Result<PinTable, PinError> {
    operator_pins_from(std::env::var(VESSEL_PUBKEY_ENV).ok().as_deref())
}

/// Verify a whole vessel buffer against the pins — the single-read
/// decode walk, unchanged ([`reflexer_vessel::decode`]): structure and
/// caps, pin resolution, STRICT signature, class refusal.
///
/// # Errors
///
/// [`VesselError`] as-is — the format crate's refusal taxonomy, never
/// wrapped or degraded.
pub fn verify(buf: &[u8], pins: &PinTable) -> Result<VerifiedVessel, VesselError> {
    reflexer_vessel::decode(buf, pins)
}

/// Verify a vessel file — [`reflexer_vessel::open`]'s bounded single-read
/// walk (regular-files only, prefix-first class check, no stat-then-read
/// window).
///
/// # Errors
///
/// [`VesselError`] as-is, including the I/O half.
pub fn verify_path(path: &Path, pins: &PinTable) -> Result<VerifiedVessel, VesselError> {
    reflexer_vessel::open(path, pins)
}

/// The monotonic apply gate: floor check (the release-lag hook — a no-op
/// while `MIN_ARTIFACT_VERSION` is 0), strict downgrade/fork refusal,
/// then the state advance. On `Ok`, `current` names the applied artifact
/// (version + commitment); on `Err`, `current` is untouched.
///
/// # Errors
///
/// [`ApplyRefusal::OlderThanCurrent`] (downgrade) and
/// [`ApplyRefusal::VersionFork`] (same version, different bytes); the
/// floor check reuses the same refusal shape.
pub fn apply_monotonic(current: &mut ApplyState, v: &VerifiedVessel) -> Result<(), ApplyRefusal> {
    v.check_floor(MIN_ARTIFACT_VERSION)?;
    v.check_monotonic(current)?;
    *current = ApplyState {
        artifact_version: v.header().artifact_version,
        commitment: v.commitment(),
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflexer_vessel::{encode_public, ApplyRefusal, Class, FORMAT_VERSION, MAGIC};

    fn key(seed: u8) -> reflexer_vessel::ed25519_dalek::SigningKey {
        reflexer_vessel::ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
    }

    fn mint(seed: u8, key_id: u32, version: u64, payload: &[u8]) -> Vec<u8> {
        encode_public(&key(seed), key_id, version, [0; 32], payload)
    }

    fn vk_hex(seed: u8) -> String {
        use std::fmt::Write as _;
        let bytes = key(seed).verifying_key().to_bytes();
        let mut out = String::with_capacity(64);
        for b in bytes {
            let _ = write!(out, "{b:02x}");
        }
        out
    }

    // ── pin posture ───────────────────────────────────────────────────

    #[test]
    fn unset_wildcard_is_a_loud_config_gap() {
        let err = operator_pins_from(None).map(|_| ()).unwrap_err();
        assert_eq!(err, PinError::Unset);
        assert!(err.to_string().contains(VESSEL_PUBKEY_ENV));
        assert!(err.to_string().contains("64 hex"), "names the shape: {err}");
        assert_eq!(
            operator_pins_from(Some("")).map(|_| ()).unwrap_err(),
            PinError::Unset
        );
        assert_eq!(
            operator_pins_from(Some("   ")).map(|_| ()).unwrap_err(),
            PinError::Unset,
            "whitespace-only is unset, not garbage"
        );
    }

    #[test]
    fn bad_hex_refuses_naming_the_env() {
        for garbage in ["nothex", "abcd", &"a".repeat(63), &"g".repeat(64)] {
            let err = operator_pins_from(Some(garbage)).map(|_| ()).unwrap_err();
            assert!(matches!(err, PinError::BadHex(_)), "{garbage}: {err:?}");
            assert!(err.to_string().contains(VESSEL_PUBKEY_ENV));
        }
    }

    #[test]
    fn wildcard_posture_accepts_any_pinned_signature() {
        let pins = operator_pins_from(Some(&vk_hex(9))).expect("valid hex builds the posture");
        // Any key-id verifies under the wildcard when the signature is
        // the wildcard key's; a foreign key's signature does not.
        let v = verify(&mint(9, 7, 1, b"payload"), &pins).expect("wildcard accepts key-id 7");
        assert_eq!(v.header().key_id, 7);
        let foreign = verify(&mint(8, 7, 1, b"payload"), &pins).unwrap_err();
        assert!(matches!(foreign, VesselError::BadSignature));
        // The compiled pin table is NOT the posture: without the wildcard
        // the same vessel is UnknownKey (fail-closed, never silent).
        let err = verify(&mint(9, 7, 1, b"payload"), &PinTable::empty()).unwrap_err();
        assert!(matches!(err, VesselError::UnknownKey(7)));
    }

    // ── verify + tamper ───────────────────────────────────────────────

    #[test]
    fn tampered_payload_refuses_strict() {
        let pins = operator_pins_from(Some(&vk_hex(3))).expect("posture");
        let mut bytes = mint(3, 1, 1, b"corpus bytes");
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        let err = verify(&bytes, &pins).unwrap_err();
        assert!(matches!(err, VesselError::BadSignature), "got {err:?}");
    }

    #[test]
    fn verify_path_roundtrips_a_tmp_file() {
        let pins = operator_pins_from(Some(&vk_hex(4))).expect("posture");
        let bytes = mint(4, 2, 1, b"file payload");
        let tmp = std::env::temp_dir().join(format!(
            "refine_vessel_r4_{}.rflexvsl",
            std::process::id()
        ));
        std::fs::write(&tmp, &bytes).expect("write fixture");
        let v = verify_path(&tmp, &pins).expect("path verify");
        assert_eq!(v.payload(), b"file payload");
        let _ = std::fs::remove_file(&tmp);
    }

    // ── monotonic apply ───────────────────────────────────────────────

    #[test]
    fn apply_is_strictly_monotonic() {
        let pins = operator_pins_from(Some(&vk_hex(5))).expect("posture");
        let v1 = verify(&mint(5, 1, 1, b"one"), &pins).expect("v1");
        let v2 = verify(&mint(5, 1, 2, b"two"), &pins).expect("v2");

        let mut state = ApplyState {
            artifact_version: 0,
            commitment: [0; 32],
        };
        apply_monotonic(&mut state, &v1).expect("genesis accepts v1");
        assert_eq!(state.artifact_version, 1);

        // Idempotent re-apply of the SAME artifact is Ok.
        apply_monotonic(&mut state, &v1).expect("idempotent");
        assert_eq!(state.artifact_version, 1);

        apply_monotonic(&mut state, &v2).expect("advance to v2");
        // Downgrade refuses, state untouched.
        let downgrade = apply_monotonic(&mut state, &v1).unwrap_err();
        assert_eq!(
            downgrade,
            ApplyRefusal::OlderThanCurrent {
                current: 2,
                offered: 1
            }
        );

        // Same version, different content — the lineage fork.
        let fork = verify(&mint(5, 1, 2, b"two-prime"), &pins).expect("fork mints fine");
        let err = apply_monotonic(&mut state, &fork).unwrap_err();
        assert_eq!(err, ApplyRefusal::VersionFork { version: 2 });
        assert_eq!(state.artifact_version, 2, "state untouched by refusals");
    }

    // ── the A10 posture, as tests ─────────────────────────────────────

    #[test]
    fn hosted_reader_never_compiles_into_this_build() {
        // The Cargo.toml posture is load-bearing: default-features = false
        // + vessel_public_read only. reflexer-vessel's own peek gate
        // refuses a hosted-class header structurally when the hosted
        // reader is not compiled; no hosted-class encoder exists to mint
        // a fixture, so the POSTURE is pinned at the manifest instead.
        let manifest = include_str!("../Cargo.toml");
        let row = manifest
            .lines()
            .find(|l| l.starts_with("reflexer-vessel = {"))
            .expect("the reflexer-vessel dep row exists");
        assert!(row.contains("optional = true"), "opt-in only: {row}");
        assert!(row.contains("default-features = false"), "{row}");
        assert!(row.contains("\"vessel_public_read\""), "{row}");
        assert!(!row.contains("vessel_hosted_read"), "the moat: {row}");
        // And the decode-level law holds regardless: this crate never
        // asks for the hosted capability anywhere in its API surface.
        assert_eq!(Class::HostedOnly.as_str(), "hosted-only");
    }

    #[test]
    fn no_vessel_bytes_in_the_artifact_dir() {
        // The A10 leak-scan pattern over assets/: a tracked artifact must
        // never BE a vessel (vessel bytes enter shipped binaries only by
        // include_bytes!, and the corpora stay code-sealed). A file that
        // starts with the vessel MAGIC is a landing that must go through
        // this module's review — the test reds until then.
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let mut scanned = 0usize;
        if let Ok(entries) = std::fs::read_dir(&assets) {
            for e in entries.flatten() {
                let p = e.path();
                if !p.is_file() {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(&p) {
                    scanned += 1;
                    assert!(
                        !bytes.starts_with(&MAGIC),
                        "{} is a vessel — A10 forbids vessel bytes in shipped binaries",
                        p.display()
                    );
                }
            }
        }
        assert!(scanned > 0, "assets/ must exist to scan");
    }

    // ── the format sanity the seam relies on ──────────────────────────

    #[test]
    fn format_version_is_v1() {
        assert_eq!(FORMAT_VERSION, 1);
    }
}
