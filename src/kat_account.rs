//! The KAT account — a thin facade over riir-auth's `account_key` substrate
//! (Issue 081, the Plan 327 consolidation home).
//!
//! **What moved.** The login/adoption/record machinery — OpenSSH Ed25519
//! parse (encrypted keys refused), machine-SSH adoption, 0600 key write, pub
//! sidecar, versioned record projection, key-is-the-authority recovery,
//! tamper-evident `verify` — is [`riir_auth::account_key::AccountKey`] now.
//! This module used to ship that machinery itself (Plans 025/026); the
//! workspace carried four copies of the pattern and the consolidation end
//! state is one. What stays here is the kat-facing presentation
//! ([`history_lines`], [`welcome_line`], the grant line in
//! [`KatAccount::summary_lines`]) and the ONE-TIME legacy adoption chain
//! that moves a pre-081 kat account onto the canonical identity.
//!
//! **The canonical identity.** The account dir is
//! [`AccountKey::default_dir()`] — `~/.config/riir-auth` (override:
//! `RIIR_AUTH_ACCOUNT_DIR`), the key file is the stock OpenSSH name
//! (`id_ed25519`), the record is `account.json` — the SAME identity the
//! `ndb` CLI (neuron-db-cli) derives. One machine, one account: `cargo
//! heal`'s payload/vote/burn signing and `ndb`'s private-row sealing share
//! the key file and the BLAKE3(pubkey) account id.
//!
//! **The legacy window.** Accounts created before Issue 081 live in the
//! mining-consent dir (`~/.config/riir-heal`, override:
//! `RIIR_HEAL_KAT_ACCOUNT_DIR`) under the kat file names
//! ([`KatAccount::LEGACY_KEY_FILE`] et al). [`KatAccount::login_default`]
//! adopts them LOUDLY into the canonical dir — the identity (and therefore
//! the KAT ledger balance keyed to it) survives; the legacy files are left
//! in place (copied, not moved) and `--unmine`'s purge sweeps both layouts.
//!
//! **Deprecated env vars.** `RIIR_HEAL_KAT_KEY` (the old key-path override)
//! is still honored by [`KatAccount::login_default`] — with a loud
//! deprecation line — so scripted setups keep working through the window;
//! the canonical override is [`AccountKey::KEY_OVERRIDE_ENV`]
//! (`RIIR_AUTH_ACCOUNT_KEY`). `RIIR_HEAL_KAT_ACCOUNT_DIR` no longer moves
//! the ACCOUNT (it moves the consent plane, which
//! [`crate::kat_mining_consent`] still owns and honors it for) — the
//! account is fleet-canonical now.
//!
//! **Accounting-only (G9d):** the grant line is the LOCAL projection of the
//! §KAT free tier — now literally [`crate::kat_units::FREE_TIER_GRANT_KAT`].
//! The record no longer carries a `grant_kat` field: it was never mutated
//! after first write, so the constant IS the projection (the canonical
//! record shape is riir-auth's — `github_user` login provenance instead).
//!
//! **Testing note.** The chain is exercised through the env-free
//! [`KatAccount::login_default_in`] seam (the substrate's `login_default_with`
//! pattern); the CLI e2e tests own their envs via
//! `RIIR_AUTH_ACCOUNT_DIR` / `RIIR_HEAL_KAT_ACCOUNT_DIR` isolation.

use std::path::{Path, PathBuf};

use riir_auth::account_key::AccountKey;
pub use riir_auth::account_key::{
    account_id_for, short_id, AccountState, LoginOutcome, ACCOUNT_CHANGELOG, ACCOUNT_VERSION,
};

use crate::kat_units::FREE_TIER_GRANT_KAT;

/// A loaded (or just-created) KAT account — a kat-facing newtype over the
/// [`AccountKey`] substrate (a type alias cannot carry the kat-specific
/// `impl`: [`KatAccount::signing_key`], [`KatAccount::summary_lines`], the
/// legacy chain).
#[derive(Debug, Clone)]
pub struct KatAccount(AccountKey);

impl std::ops::Deref for KatAccount {
    type Target = AccountKey;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl KatAccount {
    /// The private key file name inside the (canonical) account dir — the
    /// stock OpenSSH name since Issue 081.
    pub const KEY_FILE: &'static str = AccountKey::KEY_FILE;
    /// The public key sidecar file name (canonical).
    pub const PUB_FILE: &'static str = AccountKey::PUB_FILE;
    /// The versioned local record file name (canonical).
    pub const RECORD_FILE: &'static str = AccountKey::RECORD_FILE;

    /// The private key file name in the LEGACY (pre-081) account dir — what
    /// [`Self::login_default`]'s adoption arm and `--unmine`'s purge sweep
    /// look for. Never written again.
    pub const LEGACY_KEY_FILE: &'static str = "kat_id_ed25519";
    /// The legacy public sidecar name.
    pub const LEGACY_PUB_FILE: &'static str = "kat_id_ed25519.pub";
    /// The legacy record name (the grant-carrying shape; abandoned on
    /// adoption — the canonical record re-projects from the key).
    pub const LEGACY_RECORD_FILE: &'static str = "kat_account.json";

    /// The DEPRECATED key-path override, still honored (warned) by
    /// [`Self::login_default`] through the deprecation window. Superseded by
    /// [`AccountKey::KEY_OVERRIDE_ENV`].
    pub const LEGACY_KEY_OVERRIDE_ENV: &'static str = "RIIR_HEAL_KAT_KEY";

    /// The machine-global canonical account dir (delegates to the substrate).
    #[must_use]
    pub fn default_dir() -> PathBuf {
        AccountKey::default_dir()
    }

    /// The LEGACY account dir — the mining-consent dir the account lived in
    /// before Issue 081 (the consent module owns the resolution and still
    /// honors `RIIR_HEAL_KAT_ACCOUNT_DIR` for it). The adoption source and
    /// the purge sweep's second target.
    #[must_use]
    pub fn legacy_dir() -> PathBuf {
        crate::kat_mining_consent::default_dir()
    }

    /// `cargo heal login` — generate (or import via [`Self::login_import`])
    /// the key, then load-or-recover the record. Idempotent: an existing key
    /// is loaded, never regenerated. Delegates to [`AccountKey::login`].
    ///
    /// # Errors
    ///
    /// String errors for: unreadable/unparseable key files, an encrypted key,
    /// a missing `--key` import source, or unwritable account dir.
    pub fn login(dir: &Path) -> Result<(Self, LoginOutcome), String> {
        AccountKey::login(dir).map(|(a, o)| (Self(a), o))
    }

    /// `cargo heal login --key <path>` — import an existing OpenSSH Ed25519
    /// private key (unencrypted only). Refuses when the account dir already
    /// holds a key (import never overwrites identity). Delegates to
    /// [`AccountKey::login_import`].
    ///
    /// # Errors
    ///
    /// String errors for: an existing key in the account dir, an unreadable/
    /// unparseable/encrypted import source, or write failures.
    pub fn login_import(dir: &Path, source: &Path) -> Result<(Self, LoginOutcome), String> {
        AccountKey::login_import(dir, source).map(|(a, o)| (Self(a), o))
    }

    /// The associated-form loader — `heal.rs` calls `KatAccount::load(&dir)`
    /// by path (Deref does not forward associated functions), so the shim
    /// re-exports the substrate loader under the kat name.
    ///
    /// # Errors
    ///
    /// String errors for: no key file, an unparseable/encrypted key, or an
    /// unparseable-record write-back failure.
    pub fn load(dir: &Path) -> Result<Self, String> {
        AccountKey::load(dir).map(Self)
    }

    /// `cargo heal login` with the default chain — Issue 081's legacy-aware
    /// order. In order:
    ///
    /// 1. a canonical account key exists → [`LoginOutcome::Existing`] (the
    ///    idempotence contract is unchanged; adoption never re-binds an
    ///    account);
    /// 2. `RIIR_HEAL_KAT_KEY` (deprecated) — honored with a loud deprecation
    ///    line, via the [`Self::login_import`] semantics: explicit
    ///    configuration must not silently change meaning;
    /// 3. a LEGACY kat account exists in [`Self::legacy_dir`] — adopted into
    ///    the canonical dir (copied; the legacy files stay) with a loud
    ///    disclosure. This arm comes BEFORE the machine-SSH arm because a
    ///    pre-081 account may be a GENERATED key that no SSH key derives:
    ///    SSH adoption here would silently re-bind the KAT-ledger identity;
    /// 4. otherwise → [`AccountKey::login_default`] (the canonical override
    ///    → the machine's default SSH key → generate fresh with a disclosed
    ///    fallback note — the substrate's tested chain).
    ///
    /// SSH-key rotation afterwards leaves the account on the OLD identity —
    /// the ops-assisted re-binding posture Proposal 011 caveat #7 documents.
    ///
    /// # Errors
    ///
    /// String errors: the overrides unreadable/unparseable, or the
    /// account-dir/record write failures shared with [`Self::login`].
    pub fn login_default(dir: &Path) -> Result<(Self, LoginOutcome), String> {
        Self::login_default_in(dir, Some(&Self::legacy_dir()))
    }

    /// The env-free core of [`Self::login_default`] (the substrate's
    /// `login_default_with` pattern): the legacy dir is explicit, so the
    /// adoption arm is testable without mutating process env. Only the
    /// DEPRECATED override read stays env-based — it IS the deprecated
    /// contract being honored.
    ///
    /// # Errors
    ///
    /// The same errors [`Self::login_default`] documents.
    pub fn login_default_in(
        dir: &Path,
        legacy_dir: Option<&Path>,
    ) -> Result<(Self, LoginOutcome), String> {
        if dir.join(Self::KEY_FILE).exists() {
            let (account, outcome) = AccountKey::login(dir)?;
            return Ok((Self(account), outcome));
        }
        if let Ok(source) = std::env::var(Self::LEGACY_KEY_OVERRIDE_ENV) {
            eprintln!(
                "kat-account: {} is deprecated — use {} (honored this run)",
                Self::LEGACY_KEY_OVERRIDE_ENV,
                AccountKey::KEY_OVERRIDE_ENV
            );
            let (account, outcome) = AccountKey::adopt(dir, Path::new(&source), None)?;
            return Ok((Self(account), outcome));
        }
        if let Some(legacy_dir) = legacy_dir {
            let legacy = legacy_dir.join(Self::LEGACY_KEY_FILE);
            if legacy.is_file() {
                eprintln!(
                    "kat-account: legacy account found at {} — adopting it into the canonical \
                     account dir {} (shared with the `ndb` CLI); the legacy files stay and \
                     `cargo heal --unmine` sweeps both",
                    legacy.display(),
                    dir.display()
                );
                let (account, outcome) = AccountKey::adopt(dir, &legacy, None)?;
                return Ok((Self(account), outcome));
            }
        }
        AccountKey::login_default(dir).map(|(a, o)| (Self(a), o))
    }

    /// The account's Ed25519 SIGNING key, re-read from the key file (the
    /// file is the authority — the record is a projection; re-reading keeps
    /// that invariant for signing too). The seed route: the substrate
    /// exposes the raw 32-byte seed ([`AccountKey::seed`]), the dalek type
    /// is built locally — `ed25519-dalek` was already this feature's dep for
    /// vote signing, and `SigningKey::from_bytes(seed)` is the same
    /// construction the old `ssh-key → keypair → dalek` path performed.
    ///
    /// # Errors
    ///
    /// String errors for a missing key file, an unparseable/encrypted key,
    /// or a non-Ed25519 key.
    pub fn signing_key(&self) -> Result<ed25519_dalek::SigningKey, String> {
        let seed = self.0.seed()?;
        Ok(ed25519_dalek::SigningKey::from_bytes(&seed))
    }

    /// The `account` view lines (identity + grant + the Phase-0 honesty
    /// note). No `$` figures (caveats #12/#15) — tokens only. The grant is
    /// the meter constant (G9d — see the module docs); the `github:` line is
    /// only printed when the substrate record carries provenance.
    #[must_use]
    pub fn summary_lines(&self) -> Vec<String> {
        let kat = |micro: u64| {
            crate::kat_units::render_kat(i64::try_from(micro).unwrap_or(i64::MAX))
        };
        let mut lines = vec![
            format!(
                "account:            {}… ({})",
                short_id(&self.0.state().account_id),
                self.0.state().account_id
            ),
            format!("key:                {}", self.0.key_path().display()),
            format!(
                "grant projection:   {} (free tier, once per account key)",
                kat(FREE_TIER_GRANT_KAT)
            ),
        ];
        if let Some(user) = &self.0.state().github_user {
            lines.push(format!("github:             {user}"));
        }
        lines.push(
            "phase 0: local view — the server mints the grant + holds the authoritative \
             ledger at Phase 1 (T1.2); sync arrives with T0.3+"
                .to_string(),
        );
        lines
    }
}

/// Proposal 011 T0.2 (ask 8.5): the LOCAL earned/spent history lines for
/// `cargo heal account --history` — now the **offline fallback**, since
/// T0.2b landed the service-ledger view
/// ([`crate::kat_account_client::service_history_lines`], Plan 064) and the
/// CLI prefers it whenever `/balance` reads. This render stays because a
/// `kat_service`-only build has no transport, and because an unreachable
/// service must still print something honest.
///
/// Earned is the login-grant projection; spent is this repo's metered
/// burn. Every line obeys the caveat-#15 copy rule (no `$` beside a KAT
/// figure — none at all), pinned by
/// [`tests::history_lines_obey_the_copy_rule`].
#[must_use]
pub fn history_lines(grant_kat: u64, spent_kat: u64, metered_tokens: u64) -> Vec<String> {
    let kat =
        |micro: u64| crate::kat_units::render_kat(i64::try_from(micro).unwrap_or(i64::MAX));
    vec![
        "history (local view — server events land with the service half):".to_string(),
        format!("  earned:  {} (login grant projection)", kat(grant_kat)),
        format!(
            "  spent:   {} ({} tokens metered, this repo)",
            kat(spent_kat),
            metered_tokens
        ),
        format!(
            "  balance: {} (per-repo projection)",
            kat(grant_kat.saturating_sub(spent_kat))
        ),
    ]
}

/// Proposal 011 T0.2 (ask 8.3): the welcome line.
///
/// **The local `no burn yet` heuristic that used to decide when to print
/// this is gone.** T0.2b (Plan 064) replaced it with
/// [`crate::kat_account_client::evaluate_welcome`], which reads the real
/// `all-synced + nodes-healthy` condition off the service plane. This
/// string survives as the copy, and the `kat_service`-only build (no
/// transport compiled) still gates it on freshness — the honest best a
/// build with no network can do.
#[must_use]
pub fn welcome_line() -> &'static str {
    "welcome to the KAT network — this account is live; heal a repo to start metering"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes the `login_default` chain tests: the chain READS the
    /// deprecated-override env var, and libtest runs tests in parallel —
    /// without mutual exclusion one test's window can arm another test's
    /// override arm (measured: exactly that race). Lock scopes wrap only
    /// the call, never the asserts, so a failed assert cannot poison it.
    /// (the `ENV_LOCK` name the CLI e2e files also use.)
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kat_account_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// Restore the env vars the deprecated-override tests touch.
    fn clear_override_env() {
        std::env::remove_var(KatAccount::LEGACY_KEY_OVERRIDE_ENV);
    }

    #[test]
    fn legacy_file_names_are_pinned() {
        // The adoption arm + `--unmine`'s purge sweep look these up by name;
        // a silent rename would orphan every pre-081 account + leave purge
        // incomplete. Pin the wire.
        assert_eq!(KatAccount::LEGACY_KEY_FILE, "kat_id_ed25519");
        assert_eq!(KatAccount::LEGACY_PUB_FILE, "kat_id_ed25519.pub");
        assert_eq!(KatAccount::LEGACY_RECORD_FILE, "kat_account.json");
        // And the canonical names come from the substrate (one source).
        assert_eq!(KatAccount::KEY_FILE, AccountKey::KEY_FILE);
        assert_eq!(KatAccount::PUB_FILE, AccountKey::PUB_FILE);
        assert_eq!(KatAccount::RECORD_FILE, AccountKey::RECORD_FILE);
    }

    #[test]
    fn login_generates_key_and_record() {
        let dir = temp_dir("gen");
        let (account, outcome) = KatAccount::login(&dir).unwrap();
        assert!(matches!(
            outcome,
            LoginOutcome::Generated {
                fallback_note: None,
                ..
            }
        ));
        // The canonical file names.
        assert!(dir.join(KatAccount::KEY_FILE).is_file());
        assert!(dir.join(KatAccount::PUB_FILE).is_file());
        // The record parses back as the SUBSTRATE shape (github_user
        // provenance, no grant field) + carries the version stamp.
        let record = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
        assert!(record.contains(&format!("\"account_version\": {ACCOUNT_VERSION}")));
        let parsed: AccountState = serde_json::from_str(&record).unwrap();
        assert_eq!(parsed, *account.state());
        assert!(parsed.github_user.is_none());
        assert_eq!(parsed.account_id.len(), 64);
        account.verify().unwrap();
    }

    #[test]
    fn login_is_idempotent_and_preserves_the_record() {
        let dir = temp_dir("idem");
        let (first, _) = KatAccount::login(&dir).unwrap();
        let before = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
        let (second, outcome) = KatAccount::login(&dir).unwrap();
        assert!(matches!(outcome, LoginOutcome::Existing { .. }));
        assert_eq!(second.state(), first.state());
        let after = std::fs::read_to_string(dir.join(KatAccount::RECORD_FILE)).unwrap();
        assert_eq!(
            before, after,
            "an idempotent re-login must not rewrite the record"
        );
    }

    #[test]
    fn login_import_adopts_an_existing_key_and_refuses_overwrite() {
        let dir = temp_dir("imp");
        let (first, _) = KatAccount::login(&dir).unwrap();
        let dir2 = temp_dir("imp2");
        let (imported, outcome) =
            KatAccount::login_import(&dir2, &dir.join(KatAccount::KEY_FILE)).unwrap();
        assert!(matches!(outcome, LoginOutcome::Imported { .. }));
        assert_eq!(imported.state().account_id, first.state().account_id);
        // Import never overwrites identity.
        let err = KatAccount::login_import(&dir2, &dir.join(KatAccount::KEY_FILE)).unwrap_err();
        assert!(err.contains("never overwrites"), "{err}");
    }

    #[test]
    fn login_default_adopts_the_legacy_kat_account() {
        // Seed a legacy account: a canonical-style key, filed under the
        // legacy kat name in a scratch legacy dir — exactly what a pre-081
        // `cargo heal login` left behind.
        let seed_dir = temp_dir("legacy_seed");
        let (seed, _) = KatAccount::login(&seed_dir).unwrap();
        let legacy_dir = temp_dir("legacy_dir");
        std::fs::copy(
            seed_dir.join(KatAccount::KEY_FILE),
            legacy_dir.join(KatAccount::LEGACY_KEY_FILE),
        )
        .unwrap();

        let canonical = temp_dir("canonical");
        let (account, outcome, seed_id) = {
            let _guard = ENV_LOCK.lock().unwrap();
            KatAccount::login_default_in(&canonical, Some(&legacy_dir))
                .map(|(a, o)| (a, o, seed.state().account_id.clone()))
        }
        .unwrap();
        // Adopted, from the legacy path, same identity the legacy key
        // derives — the KAT ledger balance keyed to that id survives.
        assert!(matches!(outcome, LoginOutcome::Adopted { .. }));
        assert_eq!(outcome.account_id(), seed_id);
        assert_eq!(
            account.state().account_id,
            account_id_for(&account.public_key_bytes().unwrap())
        );
        // Copied, not moved: the legacy file stays (purge sweeps it later).
        assert!(legacy_dir.join(KatAccount::LEGACY_KEY_FILE).is_file());
        assert!(canonical.join(KatAccount::KEY_FILE).is_file());
        // And the canonical record was re-projected from the key.
        assert!(canonical.join(KatAccount::RECORD_FILE).is_file());
    }

    #[test]
    fn login_default_honours_the_deprecated_key_override_over_legacy() {
        // Lock-scoped env mutation (edition 2021: set_var is safe; the lock
        // keeps sibling tests out; the set/remove pair wraps ONLY the call,
        // so a failed assert cannot poison the lock).
        let result = {
            let _guard = ENV_LOCK.lock().unwrap();
            let source_dir = temp_dir("override_src");
            let (source, _) = KatAccount::login(&source_dir).unwrap();
            // A legacy dir holding a DIFFERENT key — the override (explicit
            // configuration) must win over the legacy adoption.
            let other_dir = temp_dir("override_other");
            let (other, _) = KatAccount::login(&other_dir).unwrap();
            let legacy_dir = temp_dir("override_legacy");
            std::fs::copy(
                other_dir.join(KatAccount::KEY_FILE),
                legacy_dir.join(KatAccount::LEGACY_KEY_FILE),
            )
            .unwrap();

            let canonical = temp_dir("override_canonical");
            let other_id_expected = other.state().account_id.clone();
            std::env::set_var(
                KatAccount::LEGACY_KEY_OVERRIDE_ENV,
                source_dir.join(KatAccount::KEY_FILE),
            );
            let r = KatAccount::login_default_in(&canonical, Some(&legacy_dir));
            clear_override_env();
            r.map(|(a, o)| (a, o, source.state().account_id.clone(), other_id_expected))
        };
        let (account, outcome, source_id, other_id) = result.unwrap();
        // The override arm is the DISCLOSED adoption (source in the outcome,
        // not the bare import) — the CLI prints exactly which key landed.
        assert!(matches!(outcome, LoginOutcome::Adopted { .. }));
        assert_eq!(
            account.state().account_id,
            source_id,
            "the override key wins"
        );
        assert_ne!(
            account.state().account_id,
            other_id,
            "the legacy arm must not fire"
        );
    }

    #[test]
    fn login_default_prefers_the_legacy_account_over_the_machine_ssh_key() {
        // The ORDER pin: with a legacy key present, the chain adopts it even
        // on a host that has a real machine SSH key (this dev box does; CI
        // runners do not) — the substrate chain is never reached, which the
        // recorded adopt SOURCE proves: the legacy path, never ~/.ssh.
        let legacy_seed_dir = temp_dir("order_legacy");
        let (_legacy_seed, _) = KatAccount::login(&legacy_seed_dir).unwrap();
        let legacy_dir = temp_dir("order_legacy_dir");
        std::fs::copy(
            legacy_seed_dir.join(KatAccount::KEY_FILE),
            legacy_dir.join(KatAccount::LEGACY_KEY_FILE),
        )
        .unwrap();

        let canonical = temp_dir("order_canonical");
        let (_account, outcome) = {
            let _guard = ENV_LOCK.lock().unwrap();
            KatAccount::login_default_in(&canonical, Some(&legacy_dir))
        }
        .unwrap();
        match outcome {
            LoginOutcome::Adopted { source, .. } => {
                assert_eq!(source, legacy_dir.join(KatAccount::LEGACY_KEY_FILE));
            }
            other => panic!("the legacy arm must fire, got {other:?}"),
        }
    }

    #[test]
    fn signing_key_matches_the_raw_seed_route() {
        use ed25519_dalek::Signer;
        // Bit-compat pin: the seed route (substrate seed → dalek) signs
        // identically to reading the keypair straight out of the OpenSSH
        // file — the exact construction the old module performed.
        let dir = temp_dir("sign");
        let (account, _) = KatAccount::login(&dir).unwrap();
        let msg = b"issue-081 signing route pin";
        let via_shim = account.signing_key().unwrap();
        let via_shim = via_shim.sign(msg);

        let text = std::fs::read_to_string(dir.join(KatAccount::KEY_FILE)).unwrap();
        let key = ssh_key::PrivateKey::from_openssh(&text).unwrap();
        let keypair = key.key_data().ed25519().unwrap();
        let direct = ed25519_dalek::SigningKey::from_bytes(keypair.private.as_ref());
        assert_eq!(
            direct.verifying_key().to_bytes(),
            account.public_key_bytes().unwrap()
        );
        let via_direct = direct.sign(msg);
        assert_eq!(via_shim.to_bytes(), via_direct.to_bytes());
    }

    #[test]
    fn verify_detects_record_tampering() {
        let dir = temp_dir("tamper");
        let (account, _) = KatAccount::login(&dir).unwrap();
        account.verify().unwrap();
        // A record rewritten with a foreign id: `load` recovers (the key is
        // the authority) and re-derives the true id.
        let mut state = account.state().clone();
        state.account_id = "f".repeat(64);
        std::fs::write(
            dir.join(KatAccount::RECORD_FILE),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();
        let recovered = KatAccount::load(&dir).unwrap();
        assert_eq!(recovered.state().account_id, account.state().account_id);
        recovered.verify().unwrap();
    }

    #[test]
    fn history_lines_obey_the_copy_rule() {
        // The caveat-#15 gate (no $ beside a KAT figure) for the T0.2 renders.
        let lines = history_lines(FREE_TIER_GRANT_KAT, 1_234, 56_789);
        assert_eq!(lines.len(), 4);
        for line in &lines {
            assert!(
                !line.contains('$'),
                "copy rule: no $ in a KAT render — got {line:?}"
            );
            if line.contains("KAT") {
                assert!(line.len() < 120, "render discipline: keep lines short");
            }
        }
        // Issue 031: renders go through the shared 6-decimal formatter —
        // the balance is the saturation subtraction, rendered as KAT.
        let expected = crate::kat_units::render_kat(
            i64::try_from(FREE_TIER_GRANT_KAT - 1_234).unwrap_or(i64::MAX),
        );
        assert!(
            lines[3].contains(&expected),
            "balance is the saturation subtraction, rendered as KAT — got {:?}",
            lines[3]
        );
        assert!(!welcome_line().contains('$'));
        assert!(welcome_line().contains("KAT network"));
    }

    #[test]
    fn summary_lines_carry_the_grant_projection_and_no_currency() {
        let dir = temp_dir("summary");
        let (account, _) = KatAccount::login(&dir).unwrap();
        let lines = account.summary_lines();
        // The grant is the meter constant now (G9d) — the render still
        // carries it, as 6-decimal KAT (issue 031).
        let grant_line = lines
            .iter()
            .find(|l| l.contains("grant projection"))
            .expect("the grant line survives the facade move");
        let expected = crate::kat_units::render_kat(
            i64::try_from(FREE_TIER_GRANT_KAT).unwrap_or(i64::MAX),
        );
        assert!(grant_line.contains(&expected), "{grant_line}");
        for line in &lines {
            assert!(!line.contains('$'), "copy rule: no $ — got {line:?}");
        }
    }
}
