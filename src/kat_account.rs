//! KAT account key + local account view — Proposal 005 Phase 0 T0.1 (Plan 025).
//!
//! **Identity: an Ed25519 keypair in OpenSSH format; account id = BLAKE3(pubkey).**
//! The exact pattern the stack ships three times (`riir-chain/src/ssh_key.rs`,
//! `GmKeySetup`, the mmorpg `avatar_crypto` registry) — the SHAPE and the crate
//! (`ssh-key`), not a code dep: those repos are outside this crate's boundary.
//!
//! **Machine-global, not repo-local.** An account is a user identity shared by
//! every repo this host heals — unlike the meter/charge ledgers, which are
//! deliberately repo-local (per-repo burn). Resolution (the `default_memory_path`
//! precedent at CONFIG tier — a key is identity, not disposable cache):
//! `RIIR_HEAL_KAT_ACCOUNT_DIR` → `$XDG_CONFIG_HOME/riir-heal` →
//! `~/.config/riir-heal` → `%APPDATA%\riir-heal` → temp fallback.
//!
//! Files in the account dir:
//! - [`KatAccount::KEY_FILE`] — OpenSSH private key (unencrypted, `0600` on unix)
//! - [`KatAccount::PUB_FILE`] — OpenSSH public key sidecar
//! - [`KatAccount::RECORD_FILE`] — the versioned local record ([`AccountState`])
//!
//! **The record is a projection of the key** (the charge-recovers-from-the-meter
//! precedent, Plan 024): missing / corrupt / unknown-version → re-derive from the
//! key file + warn. The key is the authority — `account_id` is a pure function
//! of the pubkey, so a lost record costs nothing but its `created_at_unix`.
//!
//! **Accounting-only (G9d):** the grant line ([`FREE_TIER_GRANT_KAT`]) is the
//! LOCAL projection of the §KAT free tier — "100M KAT granted once per account
//! key at login". The server mints it at Phase 1 (T1.2) and enforces
//! once-per-key there; nothing here mints or spends.

use std::path::{Path, PathBuf};

use ssh_key::{Algorithm, LineEnding, PrivateKey};

use crate::kat_meter::FREE_TIER_GRANT_KAT;

/// The account record shape version. Bump ONLY when the record's field set or
/// semantics change, and record the delta in [`ACCOUNT_CHANGELOG`] in the same
/// change (the meter/charge discipline: no silent shape change).
pub const ACCOUNT_VERSION: u32 = 1;

/// Versioned deltas of the account record shape.
pub const ACCOUNT_CHANGELOG: &[(u32, &str)] = &[
    (
        1,
        "initial shape: account_version, account_id (64-hex BLAKE3 of the Ed25519 \
         pubkey), created_at_unix, grant_kat (local projection of the free tier).",
    ),
];

/// The account id derivation: full 32-byte BLAKE3 of the 32-byte Ed25519
/// public key, hex-encoded (64 chars). Pure function of the pubkey — same key,
/// same id, any host, any run (G9a).
#[must_use]
pub fn account_id_for(pubkey: &[u8; 32]) -> String {
    blake3::hash(pubkey).to_hex().to_string()
}

/// Short display form (`9f3a…` — the proposal's payment-URL shape).
#[must_use]
pub fn short_id(account_id: &str) -> &str {
    &account_id[..account_id.len().min(4)]
}

/// The persisted local account record — a PROJECTION of the key + the grant
/// entitlement (accounting-only, G9d).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AccountState {
    /// The record shape version ([`ACCOUNT_VERSION`]).
    pub account_version: u32,
    /// BLAKE3(pubkey), 64-char hex.
    pub account_id: String,
    /// Unix seconds at first record write (informational; lost on record
    /// recovery, preserved on idempotent re-login).
    pub created_at_unix: u64,
    /// The local grant projection — [`FREE_TIER_GRANT_KAT`] at first write,
    /// never mutated afterwards by any Phase 0 code path.
    pub grant_kat: u64,
}

/// What `login` did (the CLI prints one line per variant).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    /// No key existed; a fresh Ed25519 keypair was generated + written.
    Generated { account_id: String },
    /// `--key <path>` imported an existing OpenSSH private key.
    Imported { account_id: String },
    /// A key already existed in the account dir; it was loaded as-is.
    Existing { account_id: String },
}

impl LoginOutcome {
    /// The account id every variant carries.
    #[must_use]
    pub fn account_id(&self) -> &str {
        match self {
            Self::Generated { account_id }
            | Self::Imported { account_id }
            | Self::Existing { account_id } => account_id,
        }
    }
}

/// A loaded (or just-created) KAT account.
#[derive(Debug)]
pub struct KatAccount {
    dir: PathBuf,
    state: AccountState,
}

/// Parse an OpenSSH Ed25519 private key from its text form. Encrypted keys are
/// REFUSED with the remedy text (the `riir-chain` `SshKeyError::Encrypted`
/// precedent — a policy refusal, not a parse failure).
fn parse_private_key(content: &str) -> Result<PrivateKey, String> {
    let key = PrivateKey::from_openssh(content)
        .map_err(|e| format!("parsing OpenSSH private key: {e}"))?;
    if key.is_encrypted() {
        return Err(
            "SSH private key is passphrase-encrypted; this host supports unencrypted keys only — \
             generate one with: ssh-keygen -t ed25519 -N '' (or drop the passphrase first)"
                .to_string(),
        );
    }
    if key.key_data().ed25519().is_none() {
        return Err("key is not Ed25519".to_string());
    }
    Ok(key)
}

/// Extract the raw 32-byte Ed25519 pubkey from a parsed private key.
fn pubkey_of(key: &PrivateKey) -> [u8; 32] {
    *key
        .public_key()
        .key_data()
        .ed25519()
        .expect("ed25519 checked at parse")
        .as_ref()
}

/// The account's Ed25519 SIGNING key (Proposal 008 T2.4 — vote signing).
/// Reads the on-disk OpenSSH key + converts to the dalek type (the key file
/// is the authority — the record is a projection; re-reading keeps that
/// invariant for signing too).
fn signing_key_of(key: &PrivateKey) -> Result<ed25519_dalek::SigningKey, String> {
    let kp = key
        .key_data()
        .ed25519()
        .ok_or_else(|| "key is not Ed25519".to_string())?;
    ed25519_dalek::SigningKey::try_from(kp)
        .map_err(|e| format!("converting the account key to a signing key: {e}"))
}

/// Write a file with `0600` on unix (best-effort elsewhere).
fn write_private(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("writing {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("chmod 600 {}: {e}", path.display()))?;
    }
    Ok(())
}

impl KatAccount {
    /// The private key file name inside the account dir.
    pub const KEY_FILE: &'static str = "kat_id_ed25519";
    /// The public key sidecar file name.
    pub const PUB_FILE: &'static str = "kat_id_ed25519.pub";
    /// The versioned local record file name.
    pub const RECORD_FILE: &'static str = "kat_account.json";

    /// The machine-global account dir (see the module docs for the tier
    /// rationale). `RIIR_HEAL_KAT_ACCOUNT_DIR` overrides the whole path.
    #[must_use]
    pub fn default_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("RIIR_HEAL_KAT_ACCOUNT_DIR") {
            return PathBuf::from(dir);
        }
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
            })
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .unwrap_or_else(std::env::temp_dir);
        base.join("riir-heal")
    }

    /// `cargo heal login` — generate (or import via [`Self::login_import`])
    /// the key, then load-or-recover the record. Idempotent: an existing key
    /// is loaded, never regenerated.
    ///
    /// # Errors
    ///
    /// String errors for: unreadable/unparseable key files, an encrypted key,
    /// a missing `--key` import source, or unwritable account dir.
    pub fn login(dir: &Path) -> Result<(Self, LoginOutcome), String> {
        let key_path = dir.join(Self::KEY_FILE);
        if key_path.exists() {
            let account = Self::load(dir)?;
            let account_id = account.state.account_id.clone();
            Ok((account, LoginOutcome::Existing { account_id }))
        } else {
            Self::generate_into(dir)
        }
    }

    /// `cargo heal login --key <path>` — import an existing OpenSSH Ed25519
    /// private key (unencrypted only). Refuses when the account dir already
    /// holds a key (import never overwrites identity).
    ///
    /// # Errors
    ///
    /// String errors for: an existing key in the account dir, an unreadable/
    /// unparseable/encrypted import source, or write failures.
    pub fn login_import(dir: &Path, source: &Path) -> Result<(Self, LoginOutcome), String> {
        let key_path = dir.join(Self::KEY_FILE);
        if key_path.exists() {
            return Err(format!(
                "account dir already holds a key ({}); import never overwrites identity — \
                 delete it first if you really mean to switch accounts",
                key_path.display()
            ));
        }
        let text = std::fs::read_to_string(source)
            .map_err(|e| format!("reading import key {}: {e}", source.display()))?;
        let key = parse_private_key(&text)?;

        std::fs::create_dir_all(dir)
            .map_err(|e| format!("creating account dir {}: {e}", dir.display()))?;
        let openssh = key
            .to_openssh(LineEnding::LF)
            .map_err(|e| format!("serializing key: {e}"))?;
        write_private(&key_path, &openssh)?;
        Self::write_pub_sidecar(dir, &key)?;

        let account = Self::with_fresh_record(dir, &key)?;
        let account_id = account.state.account_id.clone();
        Ok((account, LoginOutcome::Imported { account_id }))
    }

    /// Generate a fresh Ed25519 keypair into the account dir.
    fn generate_into(dir: &Path) -> Result<(Self, LoginOutcome), String> {
        let mut key = PrivateKey::random(&mut ssh_key::rand_core::OsRng, Algorithm::Ed25519)
            .map_err(|e| format!("generating Ed25519 keypair: {e}"))?;
        key.set_comment("riir-heal kat account");

        std::fs::create_dir_all(dir)
            .map_err(|e| format!("creating account dir {}: {e}", dir.display()))?;
        let openssh = key
            .to_openssh(LineEnding::LF)
            .map_err(|e| format!("serializing key: {e}"))?;
        write_private(&dir.join(Self::KEY_FILE), &openssh)?;
        Self::write_pub_sidecar(dir, &key)?;

        let account = Self::with_fresh_record(dir, &key)?;
        let account_id = account.state.account_id.clone();
        Ok((account, LoginOutcome::Generated { account_id }))
    }

    /// Build the account over a just-written key with a FRESH record — the
    /// generators know the record is new (no recovery warning, which is for
    /// a key that LOST its record, not one that just got one).
    fn with_fresh_record(dir: &Path, key: &PrivateKey) -> Result<Self, String> {
        let account_id = account_id_for(&pubkey_of(key));
        let state = Self::fresh_state(&account_id);
        Self::write_record(dir, &state)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            state,
        })
    }

    /// Write the `.pub` sidecar for a held private key.
    fn write_pub_sidecar(dir: &Path, key: &PrivateKey) -> Result<(), String> {
        let pub_text = key
            .public_key()
            .to_openssh()
            .map_err(|e| format!("serializing pubkey: {e}"))?;
        let pub_path = dir.join(Self::PUB_FILE);
        std::fs::write(&pub_path, format!("{pub_text}\n"))
            .map_err(|e| format!("writing {}: {e}", pub_path.display()))
    }

    /// Load the account from `dir`: parse the key, derive the account id,
    /// then load-or-recover the record. **The key is the authority** — a
    /// missing/corrupt/unknown-version record is re-derived from the key with
    /// one warning (never fatal to an explicit `account` query, and never
    /// silently regenerating the KEY).
    ///
    /// # Errors
    ///
    /// String errors for: no key file, an unparseable/encrypted key, or an
    /// unparseable-record write-back failure.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let key_path = dir.join(Self::KEY_FILE);
        if !key_path.exists() {
            return Err(format!(
                "no account key at {} — run `cargo heal login` first",
                key_path.display()
            ));
        }
        let text = std::fs::read_to_string(&key_path)
            .map_err(|e| format!("reading {}: {e}", key_path.display()))?;
        let key = parse_private_key(&text)?;
        let account_id = account_id_for(&pubkey_of(&key));

        let record_path = dir.join(Self::RECORD_FILE);
        let mut recovered = false;
        // Read → parse → validate, as one Option chain: `None` at any step is
        // the recovery path (the key is the authority — the record is a
        // projection, exactly as the charge is of the meter).
        let mut state = std::fs::read_to_string(&record_path)
            .ok()
            .and_then(|text| serde_json::from_str::<AccountState>(&text).ok())
            .filter(|s| s.account_version == ACCOUNT_VERSION && s.account_id == account_id)
            .unwrap_or_else(|| {
                recovered = true;
                Self::fresh_state(&account_id)
            });
        // Version restamp on recovery (the charge-salvage shape: the derived
        // fields are rebuilt, the version always rides the file).
        state.account_version = ACCOUNT_VERSION;
        state.account_id = account_id;
        if recovered {
            eprintln!(
                "kat-account: record missing/corrupt/stale — recovered from the key (the \
                 authority); grant re-projected at {FREE_TIER_GRANT_KAT} KAT, created_at reset"
            );
            Self::write_record(dir, &state)?;
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            state,
        })
    }

    /// The account's Ed25519 SIGNING key, re-read from the key file
    /// (Proposal 008 T2.4 — `cargo heal vote` signs with the login key;
    /// the file is the authority, the record is a projection).
    ///
    /// # Errors
    ///
    /// String errors for a missing key file, an unparseable/encrypted key,
    /// or a non-Ed25519 key.
    pub fn signing_key(&self) -> Result<ed25519_dalek::SigningKey, String> {
        let key_path = self.dir.join(Self::KEY_FILE);
        let text = std::fs::read_to_string(&key_path)
            .map_err(|e| format!("reading {}: {e}", key_path.display()))?;
        signing_key_of(&parse_private_key(&text)?)
    }

    /// Persist the record file (the generators' fresh write + load's
    /// recovery write-back — one encoder, one shape).
    fn write_record(dir: &Path, state: &AccountState) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(state).map_err(|e| format!("encoding record: {e}"))?;
        let path = dir.join(Self::RECORD_FILE);
        std::fs::write(&path, format!("{json}\n"))
            .map_err(|e| format!("writing {}: {e}", path.display()))
    }

    /// A fresh record projection for a derived account id.
    fn fresh_state(account_id: &str) -> AccountState {
        AccountState {
            account_version: ACCOUNT_VERSION,
            account_id: account_id.to_string(),
            created_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()),
            grant_kat: FREE_TIER_GRANT_KAT,
        }
    }

    /// Tamper evidence: recompute the account id from the KEY file's pubkey
    /// and compare against the record. Also re-checks the `.pub` sidecar
    /// agrees with the private key (the round-trip pin).
    ///
    /// # Errors
    ///
    /// A human-readable mismatch description on any disagreement.
    pub fn verify(&self) -> Result<(), String> {
        let text = std::fs::read_to_string(self.dir.join(Self::KEY_FILE))
            .map_err(|e| format!("reading key: {e}"))?;
        let key = parse_private_key(&text)?;
        let derived = account_id_for(&pubkey_of(&key));
        if derived != self.state.account_id {
            return Err(format!(
                "account id mismatch: key derives {derived}, record says {} — the record or \
                 key was modified after login",
                self.state.account_id
            ));
        }
        let pub_text = std::fs::read_to_string(self.dir.join(Self::PUB_FILE))
            .map_err(|e| format!("reading pub sidecar: {e}"))?;
        let from_sidecar = ssh_key::PublicKey::from_openssh(pub_text.trim_end())
            .map_err(|e| format!("parsing pub sidecar: {e}"))?;
        if from_sidecar.key_data().ed25519().map(|e| *e.as_ref())
            != Some(pubkey_of(&key))
        {
            return Err(
                "pub sidecar disagrees with the private key — regenerate it before syncing"
                    .to_string(),
            );
        }
        Ok(())
    }

    /// The loaded account state (record projection).
    #[must_use]
    pub fn state(&self) -> &AccountState {
        &self.state
    }

    /// The account dir.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The private key path (for display).
    #[must_use]
    pub fn key_path(&self) -> PathBuf {
        self.dir.join(Self::KEY_FILE)
    }

    /// The `account` view lines (identity + grant + the Phase-0 honesty note).
    /// No `$` figures (caveats #12/#15) — tokens only.
    #[must_use]
    pub fn summary_lines(&self) -> Vec<String> {
        vec![
            format!(
                "account:            {}… ({})",
                short_id(&self.state.account_id),
                self.state.account_id
            ),
            format!("key:                {}", self.key_path().display()),
            format!(
                "grant projection:   {} KAT (free tier, once per account key)",
                self.state.grant_kat
            ),
            "phase 0: local view — the server mints the grant + holds the authoritative \
             ledger at Phase 1 (T1.2); sync arrives with T0.3+"
                .to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kat_acct_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn account_id_is_blake3_of_the_pubkey() {
        // Independent recompute (the "pin every reimplemented derivation"
        // lesson): expected computed inline, not via the code under test.
        let pubkey = [7u8; 32];
        let expected = blake3::hash(&pubkey).to_hex().to_string();
        assert_eq!(account_id_for(&pubkey), expected);
        assert_eq!(account_id_for(&pubkey).len(), 64);
        // Different pubkeys → different ids; short form is the first 4 chars.
        assert_ne!(account_id_for(&[8u8; 32]), expected);
        assert_eq!(short_id(&expected), &expected[..4]);
    }

    #[test]
    fn login_generates_key_and_record() {
        let dir = temp_dir("gen");
        let (account, outcome) = KatAccount::login(&dir).unwrap();
        assert!(matches!(outcome, LoginOutcome::Generated { .. }));

        let key_text = std::fs::read_to_string(dir.join(KatAccount::KEY_FILE)).unwrap();
        let key = PrivateKey::from_openssh(&key_text).unwrap();
        assert!(!key.is_encrypted());
        assert!(key.key_data().ed25519().is_some());
        assert_eq!(
            account.state().account_id,
            account_id_for(&pubkey_of(&key)),
            "record id must be the derivation of the generated key"
        );
        assert_eq!(account.state().grant_kat, FREE_TIER_GRANT_KAT);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(KatAccount::KEY_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "private key must be 0600");
        }

        // The record parses back + carries the version stamp.
        let record = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
        assert!(record.contains(&format!("\"account_version\": {ACCOUNT_VERSION}")));
        let parsed: AccountState = serde_json::from_str(&record).unwrap();
        assert_eq!(parsed, *account.state());
    }

    #[test]
    fn login_is_idempotent_and_preserves_the_record() {
        let dir = temp_dir("idem");
        let (first, _) = KatAccount::login(&dir).unwrap();
        let (second, outcome) = KatAccount::login(&dir).unwrap();
        assert!(matches!(outcome, LoginOutcome::Existing { .. }));
        assert_eq!(first.state(), second.state(), "re-login must not move the record");
    }

    #[test]
    fn login_import_adopts_an_existing_key() {
        let source_dir = temp_dir("src");
        let (_, _) = KatAccount::login(&source_dir).unwrap();
        let source_key = source_dir.join(KatAccount::KEY_FILE);

        let dir = temp_dir("imp");
        let (imported, outcome) = KatAccount::login_import(&dir, &source_key).unwrap();
        assert!(matches!(outcome, LoginOutcome::Imported { .. }));
        assert_eq!(
            imported.state().account_id,
            KatAccount::load(&source_dir).unwrap().state().account_id,
            "import must adopt the source key's identity"
        );

        // A second import into the same dir refuses (never overwrites).
        assert!(KatAccount::login_import(&dir, &source_key).is_err());
    }

    #[test]
    fn generated_key_round_trips_with_the_pub_sidecar() {
        let dir = temp_dir("rt");
        let (account, _) = KatAccount::login(&dir).unwrap();
        account.verify().unwrap();

        let pub_text = std::fs::read_to_string(dir.join(KatAccount::PUB_FILE)).unwrap();
        assert!(pub_text.starts_with("ssh-ed25519 "), "sidecar is OpenSSH form");
    }

    #[test]
    fn verify_detects_record_tampering() {
        let dir = temp_dir("tamper");
        let (account, _) = KatAccount::login(&dir).unwrap();
        account.verify().unwrap();

        // Rewrite the record with a foreign account id.
        let mut state = account.state().clone();
        state.account_id = "0".repeat(64);
        let record = dir.join(KatAccount::RECORD_FILE);
        std::fs::write(&record, serde_json::to_string(&state).unwrap()).unwrap();

        // `load` recovers (key is the authority)…
        let recovered = KatAccount::load(&dir).unwrap();
        assert_ne!(recovered.state().account_id, "0".repeat(64));
        // …and `verify` on a doctored in-memory state catches it.
        let mut doctored = recovered;
        doctored.state.account_id = "1".repeat(64);
        assert!(doctored.verify().is_err());
    }

    #[test]
    fn corrupt_or_stale_record_recovers_from_the_key() {
        let dir = temp_dir("recover");
        let (first, _) = KatAccount::login(&dir).unwrap();
        let record = dir.join(KatAccount::RECORD_FILE);

        // Corrupt.
        std::fs::write(&record, "{ not json").unwrap();
        let (again, outcome) = KatAccount::login(&dir).unwrap();
        assert!(matches!(outcome, LoginOutcome::Existing { .. }));
        assert_eq!(again.state().account_id, first.state().account_id);
        assert_eq!(again.state().grant_kat, FREE_TIER_GRANT_KAT);

        // Unknown version.
        let mut state = first.state().clone();
        state.account_version = 99;
        std::fs::write(&record, serde_json::to_string(&state).unwrap()).unwrap();
        let (again, _) = KatAccount::login(&dir).unwrap();
        assert_eq!(again.state().account_version, ACCOUNT_VERSION);
        assert_eq!(again.state().account_id, first.state().account_id);
    }

    #[test]
    fn load_without_a_key_names_the_remedy() {
        let dir = temp_dir("nokey");
        let err = KatAccount::load(&dir).unwrap_err();
        assert!(err.contains("cargo heal login"), "error must name the remedy: {err}");
    }

    #[test]
    fn summary_carries_no_currency_figure() {
        // Caveats #12/#15: no `$` beside KAT accounting — tokens only.
        let dir = temp_dir("summary");
        let (account, _) = KatAccount::login(&dir).unwrap();
        for line in account.summary_lines() {
            assert!(!line.contains('$'), "summary line carries a currency figure: {line}");
        }
    }
}
