//! The offline gate (Plan 325 T2; issue 192 — "a fully-offline mode
//! (`RIIR_REFINE_OFFLINE=1`) documented for air-gapped users"). Plan 342
//! adds the MARKER: `cargo refine offline` — a HARD firewall.
//!
//! Two sources, one predicate: the env (exact literals `1` / `true` /
//! `on`, the promote-only env law) and the persistent marker file in the
//! account dir (`cargo refine offline` writes it; ONLY `cargo refine
//! online` clears it). THE FIREWALL LAW: the marker wins — a config
//! choice or an env value can never re-enable a network lane while the
//! marker stands (`RIIR_REFINE_OFFLINE=0` against the marker stays
//! offline). The env can only ADD offline, never remove it.
//!
//! One predicate, consulted by EVERY network lane before it dials: the
//! update surface hooks, the ONE kat URL seam (`command_service_url` —
//! every kat dial resolves through it), the points client, the burn-sync
//! / fix-stats push tails, and the hosted expert / escalation lanes. A
//! lane that dials without consulting this module is a bug, and the
//! charitable gate's positive assertions pin the consults that exist
//! today.
//!
//! Ungated by design: every build (public and internal) honors both
//! sources — an air-gapped user must never depend on which features they
//! compiled.
//!
//! Precedence vs the first-run consent: offline (either source) wins. A
//! stored "never send" answer and the env are the same posture; the env
//! is per-invocation, the marker is persistent until `online`.

use std::path::PathBuf;

/// The exact env name. A differently-cased or prefixed spelling does
/// nothing (the promote-only literal law — never a fuzzy match).
pub const OFFLINE_ENV: &str = "RIIR_REFINE_OFFLINE";

/// The marker file's name inside the account dir (beside settings.toml
/// and the consent records — one home for the machine-local state).
pub const MARKER_FILE: &str = "offline";

/// The marker's full path (the account-dir resolution the settings
/// module shares).
#[must_use]
pub fn marker_path() -> PathBuf {
    crate::kat_mining_consent::default_dir().join(MARKER_FILE)
}

/// `true` iff the marker file exists — the persistent firewall half.
#[must_use]
pub fn marker_set() -> bool {
    marker_path().exists()
}

/// `true` iff EITHER source is active: the marker (the firewall) or the
/// env (the per-invocation posture). The marker cannot be cleared by the
/// env — that is the whole point of it.
#[must_use]
pub fn offline() -> bool {
    marker_set()
        || matches!(
            std::env::var(OFFLINE_ENV).as_deref(),
            Ok("1" | "true" | "on")
        )
}

/// The one-line refusal a lane prints instead of dialing. Stable prefix
/// (`offline: <lane> skipped — `) so scripts can grep it; the SUFFIX
/// names the remedy: the marker names `cargo refine online` (the only
/// way out), the env names itself. Never an error-level exit by itself —
/// lanes decide their own exit posture (reads degrade, sends skip).
#[must_use]
pub fn refusal_line(lane: &str) -> String {
    if marker_set() {
        format!(
            "offline: {lane} skipped — the offline marker is set ({}) — `cargo refine online` re-enables the network",
            marker_path().display()
        )
    } else {
        format!("offline: {lane} skipped — {OFFLINE_ENV} is set (local healing is unaffected)")
    }
}

/// The explicit-command refusal (exit 2 posture — the `--net` pin's
/// discipline): a lane that MUST dial (update, sync, login, topup) has no
/// degrade half; it refuses loudly before any socket.
pub fn refuse_command(command: &str) -> ! {
    if marker_set() {
        eprintln!(
            "offline: {command} refused — the offline marker is set ({}) — `cargo refine online` re-enables the network",
            marker_path().display()
        );
    } else {
        eprintln!(
            "offline: {command} refused — {OFFLINE_ENV} is set (local healing is unaffected)"
        );
    }
    std::process::exit(2)
}

/// Write the marker (the `cargo refine offline` verb). Records the
/// timestamp so the user can see when the firewall went up; an existing
/// marker is left untouched (the first-set time is the honest one).
/// Returns the marker path.
///
/// # Errors
/// The dir could not be created or the write failed.
pub fn write_marker() -> Result<PathBuf, String> {
    let path = marker_path();
    if path.exists() {
        return Ok(path);
    }
    let dir = path
        .parent()
        .ok_or_else(|| "offline: the account dir has no parent".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| format!("offline: {}: {e}", dir.display()))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let body = format!(
        "offline since unix {now}\n\ncargo refine online clears this; env vars and settings.toml cannot.\n"
    );
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body).map_err(|e| format!("offline: {}: {e}", path.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("offline: {}: {e}", path.display()))?;
    Ok(path)
}

/// Remove the marker (the `cargo refine online` verb). `false` when no
/// marker stood (already online).
///
/// # Errors
/// The marker existed but could not be removed (carries the path).
pub fn clear_marker() -> Result<bool, String> {
    let path = marker_path();
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path).map_err(|e| format!("offline: {}: {e}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    /// The account-dir env var and the offline env are PROCESS-GLOBAL and
    /// the lib tests run in parallel — every test in this module takes one
    /// shared mutex for its whole body so a sibling test cannot flip an
    /// env var mid-assertion (the pre-existing env tests had the same
    /// exposure; the marker tests made it observable).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// A per-test guard: the env lock held for the whole body + a unique
    /// account dir the env var points at. Drop restores nothing global
    /// (the next test's guard sets what it needs) but removes the dir.
    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        dir: PathBuf,
    }

    impl EnvGuard {
        fn fresh(tag: &str) -> Self {
            let lock = ENV_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let dir = std::env::temp_dir().join(format!(
                "refine_offline_test_{}_{}",
                tag,
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::env::set_var("RIIR_REFINE_KAT_ACCOUNT_DIR", &dir);
            Self { _lock: lock, dir }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
            std::env::remove_var("RIIR_REFINE_KAT_ACCOUNT_DIR");
        }
    }

    #[test]
    fn exact_literals_arm_and_everything_else_is_online() {
        let _g = EnvGuard::fresh("env_literals");
        assert!(!marker_set(), "a fresh dir has no marker");
        for lit in ["1", "true", "on"] {
            std::env::set_var(OFFLINE_ENV, lit);
            assert!(offline(), "{lit} arms");
        }
        for lit in ["0", "false", "off", "", "yes", "TRUE", "On", " 1"] {
            std::env::set_var(OFFLINE_ENV, lit);
            assert!(!offline(), "{lit:?} is not an active literal");
        }
        std::env::remove_var(OFFLINE_ENV);
        assert!(!offline(), "unset is online");
    }

    #[test]
    fn refusal_line_names_the_env_and_lane() {
        let _g = EnvGuard::fresh("env_refusal");
        std::env::remove_var(OFFLINE_ENV);
        let line = refusal_line("rank");
        assert!(line.starts_with("offline: rank skipped — "));
        assert!(line.contains(OFFLINE_ENV));
    }

    #[test]
    fn marker_is_the_firewall_the_env_cannot_clear() {
        let _g = EnvGuard::fresh("marker_firewall");
        std::env::remove_var(OFFLINE_ENV);
        assert!(!offline(), "no marker + unset env is online");
        let path = write_marker().expect("the marker writes");
        assert!(path.ends_with(MARKER_FILE));
        assert!(marker_set());
        assert!(offline(), "the marker alone arms");
        // THE LAW: an env `0` (and any config) cannot clear the marker.
        std::env::set_var(OFFLINE_ENV, "0");
        assert!(offline(), "env 0 does not override the marker");
        std::env::set_var(OFFLINE_ENV, "false");
        assert!(offline(), "env false does not override the marker");
        // The marker refusal names the way out, not the env.
        let line = refusal_line("update");
        assert!(line.starts_with("offline: update skipped — "));
        assert!(line.contains("cargo refine online"), "names the remedy");
        assert!(line.contains(MARKER_FILE), "names the marker path");
        assert!(!line.contains(OFFLINE_ENV));
        // write_marker is idempotent (the first timestamp wins).
        let again = write_marker().unwrap();
        assert_eq!(again, path);
        // Only `online` clears it.
        assert!(
            clear_marker().unwrap(),
            "clearing an existing marker reports true"
        );
        assert!(!marker_set());
        std::env::set_var(OFFLINE_ENV, "1");
        assert!(offline(), "the env still arms on its own");
        std::env::set_var(OFFLINE_ENV, "0");
        assert!(!offline());
        std::env::remove_var(OFFLINE_ENV);
        assert!(
            !clear_marker().unwrap(),
            "clearing an absent marker reports false"
        );
    }

    #[test]
    fn marker_body_records_the_timestamp_and_the_law() {
        let _g = EnvGuard::fresh("marker_body");
        let path = write_marker().unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("offline since unix"),
            "the timestamp is the content"
        );
        assert!(body.contains("cargo refine online"));
        let _ = clear_marker();
    }
}
