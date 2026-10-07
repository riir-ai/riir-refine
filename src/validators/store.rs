//! The committed-artifact store (Plan 199 T3.3): versioned, BLAKE3-committed
//! validator artifacts with a monotone manifest, hot-swap on version bump,
//! and the withdrawal path.
//!
//! Shape: `assets/validators/` holds one `.wasm` per validator plus
//! `manifest.json` — the ONE record of (name, version, blake3, state). The
//! `wasm-validator-build` bin is the only sanctioned writer: it admits the
//! bytes first, then installs (version +1, atomic manifest rename). Every
//! load re-verifies the blake3 against the manifest and re-runs admission —
//! a swapped or corrupted file is refused, never executed. A version bump
//! hot-swaps by construction: the next `load` sees new bytes under a bumped
//! manifest; a manifest that lags its file is a mismatch refusal, not a
//! silent pass.
//!
//! Withdrawal (Research 233 §6.3): a validator's measured advantage is
//! judged by the `tier_guard` Lemma-4.2 likelihood-ratio exclusion — the same
//! evidence law that certifies durable rule tiers, consumed (never forked).
//! An INVERTED validator (a single fixture-floored false pass, T3.4's
//! G-safety floor) revokes immediately and irreversibly in the manifest;
//! reinstatement is only a NEW version through `install`.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{ValidatorModule, AdmissionError};

/// The manifest's on-disk name inside the store root.
pub const MANIFEST_FILE: &str = "manifest.json";

/// A validator's withdrawal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorState {
    /// Certified, loadable.
    Active,
    /// Withdrawn — loads refuse. Reason recorded, never silently dropped.
    Revoked,
}

/// One manifest row: the committed record of a validator artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorRecord {
    pub name: String,
    /// Monotone; `install` bumps, downgrades do not exist.
    pub version: u32,
    /// blake3 of the `.wasm` bytes — the artifact is what this row says.
    pub blake3: String,
    /// Artifact file name inside the store root.
    pub file: String,
    pub state: ValidatorState,
    /// Set on revocation; the reason is part of the durable record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_reason: Option<String>,
}

/// The committed manifest. Field order is the wire order (deterministic
/// JSON, stable diffs).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorManifest {
    pub validators: Vec<ValidatorRecord>,
}

impl ValidatorManifest {
    #[must_use]
    pub fn row(&self, name: &str) -> Option<&ValidatorRecord> {
        self.validators.iter().find(|r| r.name == name)
    }

    fn row_mut(&mut self, name: &str) -> Option<&mut ValidatorRecord> {
        self.validators.iter_mut().find(|r| r.name == name)
    }
}

/// Every way the store can refuse — an uncertified artifact or a manifest
/// that disagrees with its bytes is a named refusal, never a silent skip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// The artifact failed the admission gate (the reason rides along).
    Admission(String),
    /// The manifest row is missing.
    UnknownValidator(String),
    /// A revoked validator was loaded.
    Revoked { name: String, reason: Option<String> },
    /// The on-disk blake3 does not match the manifest row.
    Blake3Mismatch { expected: String, actual: String },
    /// Manifest or artifact I/O.
    Io(String),
    /// A validator is already installed at exactly these bytes (no-op).
    AlreadyCurrent,
    /// An inverted validator cannot be reinstated in place — install a new
    /// version (deliberate friction; the inversion record is durable).
    InversionIsDurable,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Admission(e) => write!(f, "admission refused: {e}"),
            StoreError::UnknownValidator(n) => write!(f, "unknown validator `{n}`"),
            StoreError::Revoked { name, reason } => write!(
                f,
                "validator `{name}` is revoked{}",
                reason.as_deref().map_or_else(String::new, |r| format!(": {r}"))
            ),
            StoreError::Blake3Mismatch { expected, actual } => {
                write!(f, "artifact bytes do not match the manifest (expected {expected}, got {actual})")
            }
            StoreError::Io(e) => write!(f, "store io: {e}"),
            StoreError::AlreadyCurrent => write!(f, "artifact already installed at this content"),
            StoreError::InversionIsDurable => {
                write!(f, "an inverted validator is never reinstated in place — install a new version")
            }
        }
    }
}

/// An admitted validator fresh from the store: the module plus the record
/// it certified against.
#[derive(Debug)]
pub struct AdmittedValidator {
    pub record: ValidatorRecord,
    pub module: Arc<ValidatorModule>,
}

/// The artifact store over a root directory (repo: `assets/validators/`).
/// No interior mutability — the manifest on disk is the source of truth and
/// every operation re-reads it; concurrent safety is the atomic rename in
/// [`ValidatorStore::install`].
#[derive(Debug, Clone)]
pub struct ValidatorStore {
    root: PathBuf,
}

impl ValidatorStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    fn artifact_path(&self, file: &str) -> PathBuf {
        self.root.join(file)
    }

    fn read_manifest(&self) -> Result<ValidatorManifest, StoreError> {
        let path = self.manifest_path();
        if !path.exists() {
            return Ok(ValidatorManifest::default());
        }
        let text = std::fs::read_to_string(&path).map_err(|e| StoreError::Io(e.to_string()))?;
        serde_json::from_str(&text).map_err(|e| StoreError::Io(format!("manifest parse: {e}")))
    }

    /// Atomic manifest write: temp file + rename, so a concurrent reader
    /// never sees a torn manifest.
    fn write_manifest(&self, manifest: &ValidatorManifest) -> Result<(), StoreError> {
        std::fs::create_dir_all(&self.root).map_err(|e| StoreError::Io(e.to_string()))?;
        let tmp = self.manifest_path().with_extension("json.tmp");
        let json = serde_json::to_string_pretty(manifest).map_err(|e| StoreError::Io(e.to_string()))?;
        std::fs::write(&tmp, json).map_err(|e| StoreError::Io(e.to_string()))?;
        std::fs::rename(&tmp, self.manifest_path()).map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(())
    }

    /// Install (or upgrade) a validator: admit the bytes, then commit them
    /// under a bumped version. Identical bytes are a [`StoreError::
    /// AlreadyCurrent`] no-op — reinstalling unchanged content must not
    /// manufacture version history.
    ///
    /// # Errors
    /// [`StoreError::Admission`] (bytes failed the gate),
    /// [`StoreError::AlreadyCurrent`] (byte-identical reinstall), I/O on
    /// the artifact or manifest write.
    pub fn install(&self, name: &str, bytes: &[u8]) -> Result<ValidatorRecord, StoreError> {
        // Admission FIRST: an uncertifiable module never reaches the store.
        ValidatorModule::admit(bytes).map_err(|e: AdmissionError| StoreError::Admission(e.to_string()))?;
        let digest = blake3::hash(bytes).to_string();

        let mut manifest = self.read_manifest()?;
        if let Some(row) = manifest.row(name) {
            if row.blake3 == digest {
                return Err(StoreError::AlreadyCurrent);
            }
        }

        let file = format!("{name}.wasm");
        std::fs::write(self.artifact_path(&file), bytes).map_err(|e| StoreError::Io(e.to_string()))?;

        let record = if let Some(row) = manifest.row_mut(name) {
            // Upgrade: bump the version, re-point the digest. A revoked
            // validator becomes Active again ONLY through a new version
            // — the deliberate friction for a withdrawn artifact.
            row.version += 1;
            row.blake3.clone_from(&digest);
            row.file.clone_from(&file);
            row.state = ValidatorState::Active;
            row.revoked_reason = None;
            row.clone()
        } else {
            let row = ValidatorRecord {
                name: name.to_string(),
                version: 1,
                blake3: digest.clone(),
                file: file.clone(),
                state: ValidatorState::Active,
                revoked_reason: None,
            };
            manifest.validators.push(row.clone());
            row
        };
        self.write_manifest(&manifest)?;
        Ok(record)
    }

    /// Load + certify: manifest row → blake3 verify → admission → cached
    /// module. The hot-swap path: after `install` bumps the manifest, the
    /// next load serves the new artifact (the capability-as-data property —
    /// no rebuild of the host).
    ///
    /// # Errors
    /// [`StoreError::UnknownValidator`], [`StoreError::Revoked`],
    /// [`StoreError::Blake3Mismatch`], [`StoreError::Admission`], I/O.
    pub fn load(&self, name: &str) -> Result<AdmittedValidator, StoreError> {
        let manifest = self.read_manifest()?;
        let row = manifest.row(name).cloned().ok_or_else(|| StoreError::UnknownValidator(name.to_string()))?;
        if row.state == ValidatorState::Revoked {
            return Err(StoreError::Revoked {
                name: row.name.clone(),
                reason: row.revoked_reason.clone(),
            });
        }
        let bytes = std::fs::read(self.artifact_path(&row.file)).map_err(|e| StoreError::Io(e.to_string()))?;
        let actual = blake3::hash(&bytes).to_string();
        if actual != row.blake3 {
            return Err(StoreError::Blake3Mismatch {
                expected: row.blake3.clone(),
                actual,
            });
        }
        let module = ValidatorModule::admit(&bytes)
            .map_err(|e| StoreError::Admission(e.to_string()))?;
        Ok(AdmittedValidator {
            record: row,
            module: Arc::new(module),
        })
    }

    /// The `--check` posture: load every recorded validator, return one
    /// receipt line per row. First failure is the error.
    ///
    /// # Errors
    /// The first [`ValidatorStore::load`] failure across the manifest.
    pub fn verify(&self) -> Result<Vec<AdmittedValidator>, StoreError> {
        let manifest = self.read_manifest()?;
        let mut out = Vec::with_capacity(manifest.validators.len());
        for row in &manifest.validators {
            out.push(self.load(&row.name)?);
        }
        Ok(out)
    }

    /// Withdraw: the durable revocation record. Loads refuse from here on.
    ///
    /// # Errors
    /// [`StoreError::UnknownValidator`] or manifest I/O.
    pub fn revoke(&self, name: &str, reason: &str) -> Result<ValidatorRecord, StoreError> {
        self.set_state(name, ValidatorState::Revoked, Some(reason.to_string()))
    }

    /// Auto-revoke for an INVERTED validator (T3.4's G-safety floor: one
    /// fixture-floored false pass). Same durable record as [`Self::revoke`]
    /// — the caller supplies the measured evidence line.
    ///
    /// # Errors
    /// [`StoreError::UnknownValidator`] or manifest I/O.
    pub fn record_inversion(&self, name: &str, evidence: &str) -> Result<ValidatorRecord, StoreError> {
        self.revoke(name, &format!("INVERTED: {evidence}"))
    }

    /// Reinstate a revoked validator WITHOUT new content — refused: the
    /// inversion record is durable; reinstatement is a new version through
    /// [`Self::install`].
    ///
    /// # Errors
    /// Always [`StoreError::InversionIsDurable`] for a revoked row.
    pub fn reinstate(&self, name: &str) -> Result<ValidatorRecord, StoreError> {
        let manifest = self.read_manifest()?;
        let row = manifest.row(name).ok_or_else(|| StoreError::UnknownValidator(name.to_string()))?;
        if row.state == ValidatorState::Revoked {
            return Err(StoreError::InversionIsDurable);
        }
        Ok(row.clone())
    }

    fn set_state(
        &self,
        name: &str,
        state: ValidatorState,
        reason: Option<String>,
    ) -> Result<ValidatorRecord, StoreError> {
        let mut manifest = self.read_manifest()?;
        let row = manifest
            .row_mut(name)
            .ok_or_else(|| StoreError::UnknownValidator(name.to_string()))?;
        row.state = state;
        row.revoked_reason = reason;
        let record = row.clone();
        self.write_manifest(&manifest)?;
        Ok(record)
    }
}

/// The withdrawal verdict for a validator with MEASURED advantage data:
/// consumes the `tier_guard` Lemma-4.2 likelihood-ratio exclusion (the same
/// evidence law as durable rule-tier certification — consumed, never
/// forked). `goal` is the store's prior belief the validator helps; the
/// Laplace-smoothed measured rate is the outcome. When the ratio excludes,
/// the measured evidence does not support keeping the validator.
#[must_use]
pub fn withdrawal_excludes(successes: u32, fails: u32, goal: f32) -> bool {
    let outcome = crate::tier_guard::outcome_probability(successes, fails);
    crate::tier_guard::lemma_4_2_excludes(goal, outcome, crate::tier_guard::TIER_GUARD_EPSILON)
}

/// The positive form: the validator's measured advantage supports keeping it.
#[must_use]
pub fn advantage_certified(successes: u32, fails: u32, goal: f32) -> bool {
    !withdrawal_excludes(successes, fails, goal)
}
