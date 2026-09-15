//! Machine-global work-stats sharing consent — the persisted state behind
//! `cargo heal --stats on` (opt IN), `cargo heal --stats off` (opt OUT), and
//! bare `cargo heal --stats` (the status readout). riir-dapps Proposal 004
//! Half 1.
//!
//! A THIRD consent state beside mining — and the deliberate INVERSE of
//! mining's default posture:
//!
//! | state | how it is reached | what a future `--fix` push does |
//! |---|---|---|
//! | [`StatsConsent::Unset`] | the default — no file on disk | **nothing** |
//! | [`StatsConsent::OptedIn`] | `cargo heal --stats on` | one tiny signed `{lane, rule_id, count}` row per run |
//! | [`StatsConsent::OptedOut`] | `cargo heal --stats off` | nothing, and onboarding never re-asks |
//!
//! Mining's [`crate::kat_mining_consent`] reads `Unset` as "running `sync`
//! IS the consent" because `sync` is an opt-in act. THIS lane's trigger
//! surface is `--fix`, which every coder runs — so `Unset` here MUST be a
//! refusal ([`StatsConsent::push_allowed`] is `true` for [`StatsConsent::
//! OptedIn`] only). The two consent planes are independent by design: one
//! never implies the other, and neither writes the other's record.
//!
//! ## Deliberate non-features (the `kat_mining_consent` conventions)
//!
//! - **Nothing is ever deleted here.** `--stats off` is forward-only:
//!   already-shared epoch aggregates stay public; the record is rewritten,
//!   never removed.
//! - **No new dep.** `serde_json` (already non-optional) over a small
//!   struct; Unix epoch seconds, not a formatted date.
//! - **Never a panic, never gated.** An unreadable/malformed/future record
//!   reads as [`StatsConsent::Unset`] with a warning the CALL SITE prints —
//!   a consent file we cannot parse is not consent, and not a crash.
//! - **In-crate, not in the riir-kat leaf — yet.** Consent is client-LOCAL
//!   state (no wire parity with the dapps worker), and the leaf is consumed
//!   by three repos; the shared record-machinery unification is a filed
//!   follow-up. The [`default_dir`] resolution DELEGATES to the mining
//!   module's, so the account dir cannot drift between the two planes.

use std::path::{Path, PathBuf};

/// The consent record's schema version. Bump when the meaning of a field
/// changes; an unknown (future) version reads as [`StatsConsent::Unset`]
/// with a warning — the conservative direction for a NEWER writer.
pub const STATS_CONSENT_VERSION: u32 = 1;

/// The consent record's file name inside the account dir (beside
/// `mining_consent.json`, same account dir).
pub const STATS_CONSENT_FILE: &str = "stats_consent.json";

/// The one work-stat lane this consent version covers. The record is
/// lane-tagged (see [`StatsDoc::lanes`]) so a future lane joins by
/// appending — the schema does not move.
pub const FIX_LANE: &str = "fix";

/// Machine-global work-stats sharing consent state.
///
/// `Unset` is a REFUSAL here — see the module docs for why this is the
/// deliberate inverse of mining's `Unset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsConsent {
    /// No record on disk (or one we could not honour) — the deny-by-default
    /// state: NO work-stat row leaves this machine.
    Unset,
    /// `cargo heal --stats on` was run — the push half is explicitly
    /// authorised for the lanes in the record.
    OptedIn,
    /// `cargo heal --stats off` was run — a recorded NO: the push half
    /// refuses AND onboarding never re-asks (that is what separates this
    /// from [`StatsConsent::Unset`]).
    OptedOut,
}

impl StatsConsent {
    /// Does the work-stats push half run in this state?
    ///
    /// ONLY [`StatsConsent::OptedIn`] — the deny-by-default inversion of
    /// mining's `push_allowed` (`!OptedOut`). Keep the two doc comments
    /// side by side: someone grepping one will find the other.
    #[must_use]
    pub fn push_allowed(self) -> bool {
        matches!(self, Self::OptedIn)
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
    /// proposal documents, kept next to the state it describes so the two
    /// cannot drift.
    #[must_use]
    pub fn status_line(self) -> &'static str {
        match self {
            Self::Unset => {
                "stats consent: unset — default OFF; nothing is shared until you opt in"
            }
            Self::OptedIn => {
                "stats consent: opted IN — future `--fix` runs share {lane, rule, count} rows (never code)"
            }
            Self::OptedOut => {
                "stats consent: opted OUT — a recorded NO; nothing is pushed and onboarding will not re-ask"
            }
        }
    }
}

/// The on-disk record. Deliberately tiny and forward-readable; lane-tagged
/// so a second work-stat lane joins by appending to [`StatsDoc::lanes`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StatsDoc {
    /// [`STATS_CONSENT_VERSION`] at write time.
    version: u32,
    /// `true` = opted in, `false` = opted out. Absence of the FILE is the
    /// third state; this field is never tri-valued.
    opted_in: bool,
    /// The lanes this consent covers. Written as `["fix"]` for an opt-in
    /// (see [`FIX_LANE`]) and `[]` for an opt-out — a future lane adds its
    /// name here without a schema change. Informational at read time: the
    /// push half checks the STATE plus its own lane membership.
    lanes: Vec<String>,
    /// Unix epoch seconds (UTC) at write time. `0` when the clock is
    /// unreadable — a bogus timestamp is worse than a zero one.
    recorded_unix_secs: u64,
}

/// The account dir this record lives in — DELEGATES to the mining consent
/// module's resolution (`RIIR_HEAL_KAT_ACCOUNT_DIR`, else
/// `XDG_CONFIG_HOME/riir-heal`, else `$HOME/.config/riir-heal`, else
/// `%APPDATA%/riir-heal`, else a temp dir), so both consent planes share
/// one home and the resolution cannot drift between them.
#[must_use]
pub fn default_dir() -> PathBuf {
    riir_kat::kat_mining_consent::default_dir()
}

/// The consent record's full path inside `dir`.
#[must_use]
pub fn consent_path(dir: &Path) -> PathBuf {
    dir.join(STATS_CONSENT_FILE)
}

/// Read the consent state, discarding the diagnostic.
///
/// Never panics; a missing, unreadable, malformed or future-versioned record
/// reads as [`StatsConsent::Unset`] — which here means "do not share", the
/// safe direction by construction. Use [`read_with_warning`] at a call site
/// that can print.
#[must_use]
pub fn read(dir: &Path) -> StatsConsent {
    read_with_warning(dir).0
}

/// Read the consent state plus one honest warning line when the record
/// exists and could not be honoured.
///
/// `None` warning means either "no record" (the ordinary
/// [`StatsConsent::Unset`]) or a clean read.
#[must_use]
pub fn read_with_warning(dir: &Path) -> (StatsConsent, Option<String>) {
    let path = consent_path(dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (StatsConsent::Unset, None),
        Err(e) => {
            return (
                StatsConsent::Unset,
                Some(format!(
                    "stats consent: {} is unreadable ({e}) — treating consent as unset; \
                     `cargo heal --stats off` rewrites it",
                    path.display()
                )),
            );
        }
    };
    match serde_json::from_str::<StatsDoc>(&text) {
        Ok(doc) if doc.version == STATS_CONSENT_VERSION => (
            if doc.opted_in {
                StatsConsent::OptedIn
            } else {
                StatsConsent::OptedOut
            },
            None,
        ),
        Ok(doc) => (
            StatsConsent::Unset,
            Some(format!(
                "stats consent: {} is version {} (this build understands \
                 {STATS_CONSENT_VERSION}) — treating consent as unset; re-run \
                 `cargo heal --stats on` or `--stats off`",
                path.display(),
                doc.version
            )),
        ),
        Err(e) => (
            StatsConsent::Unset,
            Some(format!(
                "stats consent: {} is malformed ({e}) — treating consent as unset; \
                 `cargo heal --stats off` rewrites it",
                path.display()
            )),
        ),
    }
}

/// Persist the consent state. `opt_in = true` is `--stats on`, `false` is
/// `--stats off`. Idempotent — a repeat write is a rewrite with a fresh
/// timestamp, never an error.
///
/// # Errors
///
/// String errors when the account dir cannot be created or the record cannot
/// be written. The caller decides whether that is fatal; for `off` it MUST
/// be — an opt-out we failed to persist is not an opt-out.
pub fn write(dir: &Path, opt_in: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not create the account dir {}: {e}", dir.display()))?;
    let doc = StatsDoc {
        version: STATS_CONSENT_VERSION,
        opted_in: opt_in,
        lanes: if opt_in {
            vec![FIX_LANE.to_owned()]
        } else {
            Vec::new()
        },
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

/// `--stats on` and `--stats off` are opposite writes to one field, so
/// passing both is a user error with no defensible resolution (the
/// `--mine`/`--unmine` rule). A pure predicate so the rejection is testable
/// without driving the binary.
#[must_use]
pub fn flags_conflict(on: bool, off: bool) -> bool {
    on && off
}

/// The message the CLI prints when [`flags_conflict`] holds. Exit 2 (the
/// usage-error code every other heal flag conflict uses).
pub const CONFLICT_MESSAGE: &str = "--stats on and --stats off are opposites — pass exactly one \
 (--stats on opts IN to sharing {lane, rule, count} work stats, --stats off opts OUT)";

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch dir that never touches the real account dir (the
    /// `kat_mining_consent` test convention).
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_heal_stats_consent_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn absent_record_is_unset_and_does_not_push() {
        let dir = scratch("absent");
        let (state, warning) = read_with_warning(&dir);
        assert_eq!(state, StatsConsent::Unset);
        assert!(
            warning.is_none(),
            "an absent record is ordinary, not a warning"
        );
        assert!(
            !state.push_allowed(),
            "deny-by-default: Unset MUST refuse — the deliberate inverse of mining's Unset"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trip_in_out_and_back_in() {
        let dir = scratch("roundtrip");
        assert_eq!(read(&dir), StatsConsent::Unset);
        write(&dir, true).expect("opt in");
        assert_eq!(read(&dir), StatsConsent::OptedIn);
        assert!(read(&dir).push_allowed(), "OptedIn is the ONLY sharing state");
        write(&dir, false).expect("opt out");
        assert_eq!(read(&dir), StatsConsent::OptedOut);
        assert!(!read(&dir).push_allowed());
        write(&dir, true).expect("opt back in");
        assert_eq!(read(&dir), StatsConsent::OptedIn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn opt_in_record_is_lane_tagged_and_opt_out_clears_lanes() {
        let dir = scratch("lanes");
        write(&dir, true).expect("opt in");
        let text = std::fs::read_to_string(consent_path(&dir)).expect("record");
        assert!(
            text.contains("\"lanes\":") && text.contains(FIX_LANE),
            "the opt-in record must carry the lane tag, was: {text}"
        );
        write(&dir, false).expect("opt out");
        let text = std::fs::read_to_string(consent_path(&dir)).expect("record");
        assert!(
            !text.contains(FIX_LANE),
            "an opt-out record must not claim lanes, was: {text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_is_idempotent() {
        let dir = scratch("idempotent");
        write(&dir, false).expect("first opt out");
        write(&dir, false).expect("second opt out must not error");
        write(&dir, false).expect("third opt out must not error");
        assert_eq!(read(&dir), StatsConsent::OptedOut);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_record_is_unset_with_a_warning() {
        let dir = scratch("malformed");
        std::fs::write(consent_path(&dir), "{ not json at all").expect("plant");
        let (state, warning) = read_with_warning(&dir);
        assert_eq!(state, StatsConsent::Unset);
        let warning = warning.expect("a malformed record MUST warn, not silently read as unset");
        assert!(warning.contains("malformed"), "warning was: {warning}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn future_version_is_unset_with_a_warning() {
        let dir = scratch("future");
        std::fs::write(
            consent_path(&dir),
            r#"{"version":99,"opted_in":true,"lanes":["fix"],"recorded_unix_secs":0}"#,
        )
        .expect("plant");
        let (state, warning) = read_with_warning(&dir);
        assert_eq!(
            state,
            StatsConsent::Unset,
            "a future version must NOT read as consent"
        );
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
        assert_eq!(read(&dir), StatsConsent::Unset);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflicting_flags_are_rejected() {
        assert!(flags_conflict(true, true));
        assert!(!flags_conflict(true, false));
        assert!(!flags_conflict(false, true));
        assert!(!flags_conflict(false, false));
        assert!(CONFLICT_MESSAGE.contains("--stats on"));
        assert!(CONFLICT_MESSAGE.contains("--stats off"));
    }

    #[test]
    fn default_dir_delegates_to_the_shared_account_dir() {
        // The two consent planes MUST share one account-dir resolution —
        // asserted by calling both and comparing, without mutating the env.
        assert_eq!(
            default_dir(),
            riir_kat::kat_mining_consent::default_dir(),
            "the stats record must live beside the mining record"
        );
    }

    #[test]
    fn status_lines_are_distinct_and_name_the_lane() {
        let lines = [
            StatsConsent::Unset.status_line(),
            StatsConsent::OptedIn.status_line(),
            StatsConsent::OptedOut.status_line(),
        ];
        assert!(lines[0] != lines[1] && lines[1] != lines[2] && lines[0] != lines[2]);
        for line in lines {
            assert!(line.starts_with("stats consent: "), "line was: {line}");
        }
        assert!(StatsConsent::Unset.status_line().contains("default OFF"));
        assert!(StatsConsent::OptedOut.status_line().contains("re-ask"));
    }

    #[test]
    fn record_lives_beside_the_mining_record_not_over_it() {
        let dir = scratch("distinct");
        assert_ne!(
            consent_path(&dir),
            riir_kat::kat_mining_consent::consent_path(&dir),
            "the two consent planes must never share a file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
