//! The `structural_admission`-gated persistent binding for the Issue 136
//! chain — the ONLY filesystem surface of the structural self-mutation
//! lane, and opt-in by feature so no default build can persist structural
//! state. The state machine it persists lives UNGATED in
//! [`crate::structural_admission`]; this module is bytes-on-disk only:
//!
//! - artifacts: `<root>/.heal/structural/artifacts/<blake3-64hex>` —
//!   content-addressed, write-once, re-verified on every touch (an
//!   existing file whose bytes no longer hash to its address is CORRUPT,
//!   never silently reused);
//! - heads: `<root>/.heal/structural/heads.json` — the [`ChainState`],
//!   saved under compare-and-save: the caller names the state it observed,
//!   and a save whose on-disk state moved since is refused (CAS at the
//!   store layer, mirroring the head-level CAS).
//!
//! No production caller exists (Issue 136 lands the vocabulary; activation
//! of any auto-serve kind is a separate owner call). The first caller will
//! be the review/commit CLI surface, gated on this same feature.

use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::structural_admission::{
    validate_artifact_hash, ChainState, ARTIFACTS_DIR, CHAIN_STATE_FILE,
};

/// The structural chain store rooted at a repo root (the same root the
/// `.heal/` trajectory store hangs off).
#[derive(Debug, Clone)]
pub struct ChainStore {
    root: PathBuf,
}

/// Store-layer failures. Io is transparent; everything else is a named
/// corruption/race with the offending detail.
#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    /// An on-disk artifact/heads file failed verification (wrong hash,
    /// unparseable JSON, wrong version).
    Corrupt(String),
    /// The on-disk state moved since the caller observed it — re-observe
    /// and retry (the store-layer CAS).
    CasRace,
    MalformedArtifactHash(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "structural store io: {e}"),
            StoreError::Corrupt(why) => write!(f, "structural store corrupt: {why}"),
            StoreError::CasRace => write!(
                f,
                "structural store CAS refused: heads.json moved since the caller \
                 observed it — re-observe and retry"
            ),
            StoreError::MalformedArtifactHash(hash) => {
                write!(f, "structural store: malformed artifact hash {hash:?}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl ChainStore {
    /// Open (not create) the store over `repo_root`.
    #[must_use]
    pub fn open(repo_root: impl Into<PathBuf>) -> Self {
        ChainStore { root: repo_root.into() }
    }

    /// The heads file path (diagnostics + tests).
    #[must_use]
    pub fn state_path(&self) -> PathBuf {
        self.root.join(CHAIN_STATE_FILE)
    }

    /// The artifacts directory (diagnostics + tests).
    #[must_use]
    pub fn artifacts_dir(&self) -> PathBuf {
        self.root.join(ARTIFACTS_DIR)
    }

    /// Raw heads read: `None` = absent file (genesis), `Some` = parsed +
    /// version-checked state.
    fn load_raw(&self) -> Result<Option<ChainState>, StoreError> {
        let bytes = match fs::read(self.state_path()) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StoreError::Io(e)),
        };
        let state: ChainState = serde_json::from_slice(&bytes)
            .map_err(|e| StoreError::Corrupt(format!("heads.json: {e}")))?;
        if state.version != crate::structural_admission::CHAIN_STATE_VERSION {
            return Err(StoreError::Corrupt(format!(
                "heads.json version {} != supported {} — upgrade path is a human act",
                state.version,
                crate::structural_admission::CHAIN_STATE_VERSION,
            )));
        }
        Ok(Some(state))
    }

    /// Load the chain state; an absent heads file is genesis (the empty
    /// chain — the default posture).
    ///
    /// # Errors
    ///
    /// [`StoreError::Corrupt`] for unparseable JSON or a wrong version;
    /// [`StoreError::Io`] for unreadable files.
    pub fn load_state(&self) -> Result<ChainState, StoreError> {
        Ok(self.load_raw()?.unwrap_or_else(ChainState::genesis))
    }

    /// Compare-and-save: the on-disk state must still equal the
    /// `observed` state the caller built `next` from, else
    /// [`StoreError::CasRace`]. Written via temp-file + rename so a reader
    /// never sees a torn heads file.
    ///
    /// # Errors
    ///
    /// [`StoreError::CasRace`] when the on-disk state no longer equals
    /// `observed`; [`StoreError::Corrupt`] / [`StoreError::Io`] as
    /// [`ChainStore::load_state`].
    pub fn cas_save(&self, observed: &ChainState, next: &ChainState) -> Result<(), StoreError> {
        let on_disk = self.load_raw()?;
        let still_observed = match &on_disk {
            Some(state) => state == observed,
            None => *observed == ChainState::genesis(),
        };
        if !still_observed {
            return Err(StoreError::CasRace);
        }
        let path = self.state_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(next).map_err(|e| {
            StoreError::Corrupt(format!("heads.json serialize: {e}"))
        })?)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Write a proposal artifact under its content address (full 64-hex
    /// BLAKE3), immutably. Re-writing identical bytes is a no-op returning
    /// the same address; an existing file whose bytes no longer hash to
    /// its address is corrupt and REFUSED, never silently reused.
    ///
    /// # Errors
    ///
    /// [`StoreError::Corrupt`] when an existing file at the address carries
    /// different bytes; [`StoreError::Io`] for filesystem failures.
    pub fn write_artifact(&self, bytes: &[u8]) -> Result<String, StoreError> {
        let hash = blake3::hash(bytes).to_hex().to_string();
        let dir = self.artifacts_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(&hash);
        if path.exists() {
            let existing = fs::read(&path)?;
            if existing != bytes {
                return Err(StoreError::Corrupt(format!(
                    "artifact {hash} exists with different bytes — refusing to reuse the address"
                )));
            }
            return Ok(hash);
        }
        fs::write(&path, bytes)?;
        Ok(hash)
    }

    /// Read + re-verify an artifact by address: the bytes must still hash
    /// to the address they are stored under.
    ///
    /// # Errors
    ///
    /// [`StoreError::MalformedArtifactHash`] for a bad address spelling;
    /// [`StoreError::Corrupt`] when the stored bytes no longer hash to it;
    /// [`StoreError::Io`] when absent/unreadable.
    pub fn read_artifact(&self, hash: &str) -> Result<Vec<u8>, StoreError> {
        if validate_artifact_hash(hash).is_err() {
            return Err(StoreError::MalformedArtifactHash(hash.to_owned()));
        }
        let bytes = fs::read(self.artifacts_dir().join(hash))?;
        if blake3::hash(&bytes).to_hex().as_str() != hash {
            return Err(StoreError::Corrupt(format!(
                "artifact {hash} bytes no longer hash to their address"
            )));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structural_admission::{
        ChainState, StructuralProposal, ProposalPayload, BenchShape, artifact_hash,
        canonical_artifact_bytes,
    };
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEMP_SEQ: AtomicU32 = AtomicU32::new(0);

    /// pid + counter-suffixed temp root — never a fixed shared path (the
    /// concurrent-process truncation class).
    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "riir_clippy_struct_adm_{}_{}_{}",
            tag,
            std::process::id(),
            TEMP_SEQ.fetch_add(1, Ordering::Relaxed),
        ))
    }

    fn sample_proposal() -> StructuralProposal {
        StructuralProposal {
            title: "add manual_range_contains fixture".to_owned(),
            payload: ProposalPayload::CorpusRule {
                domain: "clippy".to_owned(),
                rule_id: "manual_range_contains".to_owned(),
                bench: BenchShape::NotBenchable,
                body: "{\"rule_id\":\"manual_range_contains\"}".to_owned(),
            },
        }
    }

    #[test]
    fn artifact_write_read_roundtrip_is_idempotent() {
        let root = temp_root("roundtrip");
        let _ = fs::remove_dir_all(&root);
        let store = ChainStore::open(&root);
        let proposal = sample_proposal();
        let bytes = canonical_artifact_bytes(&proposal).unwrap();
        let addr = store.write_artifact(&bytes).unwrap();
        assert_eq!(addr, artifact_hash(&proposal).unwrap());
        assert!(store.artifacts_dir().join(&addr).is_file());
        // idempotent re-write: same address, no error
        assert_eq!(store.write_artifact(&bytes).unwrap(), addr);
        // read-back re-verifies
        assert_eq!(store.read_artifact(&addr).unwrap(), bytes);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn artifact_corruption_is_refused_never_reused() {
        let root = temp_root("corrupt");
        let _ = fs::remove_dir_all(&root);
        let store = ChainStore::open(&root);
        let bytes = canonical_artifact_bytes(&sample_proposal()).unwrap();
        let addr = store.write_artifact(&bytes).unwrap();
        // corrupt the file behind the address
        fs::write(store.artifacts_dir().join(&addr), b"tampered").unwrap();
        // write refuses to reuse the corrupt address
        assert!(matches!(store.write_artifact(&bytes), Err(StoreError::Corrupt(_))));
        // read refuses the tampered bytes
        assert!(matches!(store.read_artifact(&addr), Err(StoreError::Corrupt(_))));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn malformed_addresses_refuse_before_any_io() {
        let root = temp_root("badaddr");
        let _ = fs::remove_dir_all(&root);
        let store = ChainStore::open(&root);
        assert!(matches!(
            store.read_artifact("../../etc/passwd"),
            Err(StoreError::MalformedArtifactHash(_))
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn absent_heads_file_is_genesis() {
        let root = temp_root("genesis");
        let _ = fs::remove_dir_all(&root);
        let store = ChainStore::open(&root);
        assert_eq!(store.load_state().unwrap(), ChainState::genesis());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cas_save_roundtrips_and_refuses_races() {
        let root = temp_root("cas");
        let _ = fs::remove_dir_all(&root);
        let store = ChainStore::open(&root);
        let observed = store.load_state().unwrap();
        let next = observed.clone().advance(None, &"1".repeat(64), 100, "first").unwrap();
        store.cas_save(&observed, &next).unwrap();
        assert_eq!(store.load_state().unwrap(), next);

        // a second writer that still observes genesis — its save is a race
        let race_next = ChainState::genesis().advance(None, &"2".repeat(64), 101, "racer").unwrap();
        assert!(matches!(
            store.cas_save(&ChainState::genesis(), &race_next),
            Err(StoreError::CasRace)
        ));

        // a correct re-observe succeeds
        let fresh = store.load_state().unwrap();
        let next2 = fresh.clone().advance(Some(1), &"3".repeat(64), 102, "second").unwrap();
        store.cas_save(&fresh, &next2).unwrap();
        assert_eq!(store.load_state().unwrap(), next2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn version_mismatch_is_loud_corruption() {
        let root = temp_root("version");
        let _ = fs::remove_dir_all(&root);
        let store = ChainStore::open(&root);
        let mut future = ChainState::genesis();
        future.version = crate::structural_admission::CHAIN_STATE_VERSION + 1;
        fs::create_dir_all(store.state_path().parent().unwrap()).unwrap();
        fs::write(store.state_path(), serde_json::to_vec(&future).unwrap()).unwrap();
        assert!(matches!(store.load_state(), Err(StoreError::Corrupt(_))));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn torn_or_unparseable_heads_is_corrupt() {
        let root = temp_root("torn");
        let _ = fs::remove_dir_all(&root);
        let store = ChainStore::open(&root);
        fs::create_dir_all(store.state_path().parent().unwrap()).unwrap();
        fs::write(store.state_path(), b"{not json").unwrap();
        assert!(matches!(store.load_state(), Err(StoreError::Corrupt(_))));
        let _ = fs::remove_dir_all(&root);
    }
}
