//! Vendored from `riir-kat` (`863c590798b5`, 2026-11-09, issue 221
//! private-dep minimization): the feature-free protocol tier refine's
//! UNGATED surfaces consume (secret scan, units, consent dir, config
//! store, hosts, net pin, state stamp, soft gate). std-only by the
//! leaf's protocol-tier purity rule. The canonical copy stays in the
//! private sibling (`riir-kat` `src/kat_mining_consent.rs`); drift is caught by the
//! golden pins in `tests/katplane_parity.rs` and repaired by
//! re-vendoring via `scripts/vendor_katplane.sh` — never hand-edit one
//! side.

//! Machine-global contribution consent — the persisted state behind the
//! contribution surface. (The historical CLI writers `cargo refine
//! mine` / `cargo refine unmine` were REMOVED 2026-11-06 per the owner's
//! vocabulary law — issue 192 GAP 10: mining vocabulary reads as
//! crypto-extraction; join/leave is the contribution surface. The RECORD
//! and the push gate stay: a record planted by any older binary keeps
//! governing `sync`'s push half, and `join`/`leave` are the product's
//! consent verbs for the points plane.)
//!
//! Owner mandate 2026-09-08 (the ai.gist.rs three-tier engagement story):
//! the front page documents three honest tiers, and a tier the CLI cannot
//! record is not a tier. This module is that record — nothing else.
//!
//! ## The three tiers, and why `Unset` is not a refusal
//!
//! | state | how it is reached | what `cargo refine sync` does |
//! |---|---|---|
//! | [`MiningConsent::Unset`] | the default — no file on disk | pushes, exactly as before |
//! | [`MiningConsent::OptedIn`] | a persisted opted-in record | pushes |
//! | [`MiningConsent::OptedOut`] | a persisted opted-out record | pull only — the push half REFUSES |
//!
//! `Unset` MUST behave exactly as the pre-consent build did: **running
//! `sync` is itself the consent** (that is the policy line `--mine` has
//! printed since Proposal 008 — "nothing leaves this machine unless you run
//! `cargo refine sync`"). Making the absent state a refusal would silently
//! break every existing user's sync, so only `OptedOut` changes behaviour.
//!
//! ## Deliberate non-features
//!
//! - **Nothing is ever deleted here.** `--unmine` names the queued outbox
//!   path and tells the user the files are theirs to remove; the healer does
//!   not touch them. A tool that deletes evidence of what it queued is not
//!   auditable.
//! - **No dep.** The record is `serde_json` (already a non-optional dep) over
//!   a 3-field struct, and the timestamp is Unix epoch seconds (the
//!   `kat_account` / `kat_redact` convention) rather than a formatted string,
//!   because a formatted UTC string would cost a new dependency and
//!   BOUNDARY.md governs deps.
//! - **Never a panic, never gated.** An unreadable or malformed file reads as
//!   [`MiningConsent::Unset`] with a warning the CALL SITE prints (see
//!   [`read_with_warning`]) — a consent file we cannot parse is not consent,
//!   and it is also not a crash. The module is compiled in EVERY feature
//!   build so `--unmine` works even where `kat_service` did not compile.

use std::path::{Path, PathBuf};

/// The consent record's schema version. Bump when the meaning of a field
/// changes; an unknown (future) version reads as [`MiningConsent::Unset`],
/// which is the conservative direction for a NEWER writer only in the
/// pushing sense — see [`read_with_warning`] for the honest warning.
pub const CONSENT_VERSION: u32 = 1;

/// The consent record's file name inside the account dir.
pub const CONSENT_FILE: &str = "mining_consent.json";

/// Machine-global mining consent state.
///
/// `Unset` is NOT a refusal — see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningConsent {
    /// No record on disk (or one we could not read) — behaves exactly as the
    /// pre-consent build: `sync` pushes, and running `sync` is the consent.
    Unset,
    /// A persisted opted-in record — the push half is explicitly authorised.
    OptedIn,
    /// A persisted opted-out record — the push half REFUSES; the lease
    /// pull half (a read, not a contribution) still runs.
    OptedOut,
}

impl MiningConsent {
    /// Does the outbound push half of `cargo refine sync` run in this state?
    ///
    /// `Unset` and `OptedIn` both push; only `OptedOut` does not.
    #[must_use]
    pub fn push_allowed(self) -> bool {
        !matches!(self, Self::OptedOut)
    }

    /// One short human word for a status line.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::OptedIn => "opted in",
            Self::OptedOut => "opted out",
        }
    }

    /// The one-line status the CLI prints for this state — the wording the
    /// front page documents, kept next to the state it describes so the two
    /// cannot drift.
    #[must_use]
    pub fn status_line(self) -> &'static str {
        match self {
            Self::Unset => {
                "mining consent: unset — `cargo refine sync` pushes (running sync IS the consent)"
            }
            Self::OptedIn => {
                "mining consent: opted IN — `cargo refine sync` pushes the redacted batch"
            }
            Self::OptedOut => {
                "mining consent: opted OUT — `cargo refine sync` pulls only; nothing is pushed"
            }
        }
    }
}

/// The on-disk record. Deliberately tiny and forward-readable.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ConsentDoc {
    /// [`CONSENT_VERSION`] at write time.
    version: u32,
    /// `true` = opted in, `false` = opted out. Absence of the FILE is the
    /// third state; this field is never tri-valued.
    opted_in: bool,
    /// Unix epoch seconds (UTC) at write time. `0` when the clock is
    /// unreadable — a bogus timestamp is worse than a zero one.
    recorded_unix_secs: u64,
}

/// The account dir this record lives in — the same resolution
/// `kat_account::KatAccount::default_dir()` uses (it delegates here, so the
/// two cannot drift): `RIIR_REFINE_KAT_ACCOUNT_DIR`, else
/// `XDG_CONFIG_HOME/riir-heal`, else `$HOME/.config/riir-heal`, else
/// `%APPDATA%/riir-heal`, else a temp dir.
///
/// Lives in THIS module rather than `kat_account` because consent must be
/// readable and writable in every feature build, and `kat_account` is behind
/// `kat_service`.
#[must_use]
pub fn default_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("RIIR_REFINE_KAT_ACCOUNT_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("riir-heal")
}

/// The consent record's full path inside `dir`.
#[must_use]
pub fn consent_path(dir: &Path) -> PathBuf {
    dir.join(CONSENT_FILE)
}

/// Read the consent state, discarding the diagnostic.
///
/// Never panics; a missing, unreadable, malformed or future-versioned record
/// reads as [`MiningConsent::Unset`]. Use [`read_with_warning`] at a call
/// site that can print — silently swallowing "your consent file is corrupt"
/// is how a user ends up believing they opted out when they did not.
#[must_use]
pub fn read(dir: &Path) -> MiningConsent {
    read_with_warning(dir).0
}

/// Read the consent state plus one honest warning line when the record
/// exists and could not be honoured.
///
/// `None` warning means either "no record" (the ordinary [`MiningConsent::Unset`])
/// or a clean read.
#[must_use]
pub fn read_with_warning(dir: &Path) -> (MiningConsent, Option<String>) {
    let path = consent_path(dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (MiningConsent::Unset, None),
        Err(e) => {
            return (
                MiningConsent::Unset,
                Some(format!(
                    "mining consent: {} is unreadable ({e}) — treating consent as unset; \
                     removing the record resets it",
                    path.display()
                )),
            );
        }
    };
    match serde_json::from_str::<ConsentDoc>(&text) {
        Ok(doc) if doc.version == CONSENT_VERSION => (
            if doc.opted_in {
                MiningConsent::OptedIn
            } else {
                MiningConsent::OptedOut
            },
            None,
        ),
        Ok(doc) => (
            MiningConsent::Unset,
            Some(format!(
                "mining consent: {} is version {} (this build understands {CONSENT_VERSION}) — \
                 treating consent as unset; removing the record resets it",
                path.display(),
                doc.version
            )),
        ),
        Err(e) => (
            MiningConsent::Unset,
            Some(format!(
                "mining consent: {} is malformed ({e}) — treating consent as unset; \
                 removing the record resets it",
                path.display()
            )),
        ),
    }
}

/// Persist the consent state. `opted_in = true` is `--mine`, `false` is
/// `--unmine`. Idempotent — a repeat write is a byte-level rewrite with a
/// fresh timestamp, never an error.
///
/// # Errors
///
/// String errors when the account dir cannot be created or the record cannot
/// be written (a read-only `$HOME`, a full disk). The caller decides whether
/// that is fatal; for `--unmine` it MUST be, because a consent opt-out we
/// failed to persist is not an opt-out.
pub fn write(dir: &Path, opted_in: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not create the account dir {}: {e}", dir.display()))?;
    let doc = ConsentDoc {
        version: CONSENT_VERSION,
        opted_in,
        recorded_unix_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    };
    let json = serde_json::to_string_pretty(&doc)
        .map_err(|e| format!("could not serialize the consent record: {e}"))?;
    let path = consent_path(dir);
    std::fs::write(&path, format!("{json}\n"))
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(())
}

/// `--mine` and `--unmine` are opposite writes to one field, so passing both
/// is a user error with no defensible resolution (neither "last wins" nor
/// "in wins" is guessable from the command line). A pure predicate so the
/// rejection is testable without driving the binary.
#[must_use]
pub fn flags_conflict(mine: bool, unmine: bool) -> bool {
    mine && unmine
}

/// The message the CLI prints when [`flags_conflict`] holds. Exit 2 (the
/// usage-error code every other heal flag conflict uses).
pub const CONFLICT_MESSAGE: &str = "--mine and --unmine are opposites — pass exactly one (--mine opts IN to \
     contributing redacted batches, --unmine opts OUT)";

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch dir that never touches the real `~/.config/riir-heal`.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_heal_consent_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn absent_record_is_unset_and_pushes() {
        let dir = scratch("absent");
        let (state, warning) = read_with_warning(&dir);
        assert_eq!(state, MiningConsent::Unset);
        assert!(
            warning.is_none(),
            "an absent record is ordinary, not a warning"
        );
        assert!(
            state.push_allowed(),
            "Unset MUST behave as the pre-consent build — making the absent state a refusal \
             breaks every existing user"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trip_unset_in_out_and_back_in() {
        let dir = scratch("roundtrip");
        assert_eq!(read(&dir), MiningConsent::Unset);
        write(&dir, true).expect("opt in");
        assert_eq!(read(&dir), MiningConsent::OptedIn);
        assert!(read(&dir).push_allowed());
        write(&dir, false).expect("opt out");
        assert_eq!(read(&dir), MiningConsent::OptedOut);
        assert!(
            !read(&dir).push_allowed(),
            "OptedOut is the ONLY refusing state"
        );
        write(&dir, true).expect("opt back in");
        assert_eq!(read(&dir), MiningConsent::OptedIn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_is_idempotent() {
        let dir = scratch("idempotent");
        write(&dir, false).expect("first opt out");
        let first = std::fs::read_to_string(consent_path(&dir)).expect("record");
        write(&dir, false).expect("second opt out must not error");
        write(&dir, false).expect("third opt out must not error");
        assert_eq!(read(&dir), MiningConsent::OptedOut);
        // The state is identical; only the timestamp may differ.
        assert!(first.contains("\"opted_in\": false"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_record_is_unset_with_a_warning() {
        let dir = scratch("malformed");
        std::fs::write(consent_path(&dir), "{ not json at all").expect("plant");
        let (state, warning) = read_with_warning(&dir);
        assert_eq!(state, MiningConsent::Unset);
        let warning = warning.expect("a malformed record MUST warn, not silently read as unset");
        assert!(warning.contains("malformed"), "warning was: {warning}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn future_version_is_unset_with_a_warning() {
        let dir = scratch("future");
        std::fs::write(
            consent_path(&dir),
            r#"{"version":99,"opted_in":false,"recorded_unix_secs":0}"#,
        )
        .expect("plant");
        let (state, warning) = read_with_warning(&dir);
        assert_eq!(state, MiningConsent::Unset);
        assert!(
            warning.unwrap_or_default().contains("version 99"),
            "a future-versioned record must name its version"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_record_is_unset_not_a_panic() {
        let dir = scratch("empty");
        std::fs::write(consent_path(&dir), "").expect("plant");
        assert_eq!(read(&dir), MiningConsent::Unset);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflicting_flags_are_rejected() {
        assert!(flags_conflict(true, true));
        assert!(!flags_conflict(true, false));
        assert!(!flags_conflict(false, true));
        assert!(!flags_conflict(false, false));
        assert!(CONFLICT_MESSAGE.contains("--mine"));
        assert!(CONFLICT_MESSAGE.contains("--unmine"));
    }

    #[test]
    fn default_dir_honours_the_env_override() {
        // Read-only assertion on the resolution ORDER: the override wins
        // whenever it is set, which is what every test in this repo relies
        // on to stay off the real account dir. Asserted without mutating
        // the process env (tests share it).
        let resolved = default_dir();
        let override_dir = std::env::var("RIIR_REFINE_KAT_ACCOUNT_DIR").ok();
        match override_dir {
            Some(dir) => assert_eq!(resolved, PathBuf::from(dir)),
            None => assert!(
                resolved.ends_with("riir-heal"),
                "the fallback must be a riir-heal dir, got {}",
                resolved.display()
            ),
        }
    }

    #[test]
    fn status_lines_are_distinct_and_name_the_command() {
        let lines = [
            MiningConsent::Unset.status_line(),
            MiningConsent::OptedIn.status_line(),
            MiningConsent::OptedOut.status_line(),
        ];
        assert!(lines[0] != lines[1] && lines[1] != lines[2] && lines[0] != lines[2]);
        for line in lines {
            assert!(line.starts_with("mining consent: "), "line was: {line}");
        }
        assert!(MiningConsent::OptedOut.status_line().contains("pulls only"));
    }
}
