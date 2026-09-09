//! The machine-global "contributed at least once" record (Issue 075 — the
//! soft gate's fourth predicate input).
//!
//! One fact, one file: `<account dir>/contributed_once.json`. Written by the
//! telemetry sync the first time the service ACCEPTS a pushed batch (the
//! repo-local `kat_sync_watermark.json` already advances on the same
//! acceptance — but the watermark is per-repo, and the gate must not brick a
//! contributor in their second repo, so the account-level fact lives beside
//! the account, not beside the repo).
//!
//! # Read direction (load-bearing)
//!
//! `has_contributed` resolves ambiguity ANTI-BRICK: a corrupt/unreadable
//! marker reads as CONTRIBUTED. The marker only ever makes the 075 gate more
//! permissive, so a false "contributed" merely lets a depleted free-rider
//! through a soft gate, while a false "not contributed" would refuse a real
//! contributor at run start — the exact brick the owner's decision forbids
//! (075: the gate must never brick contributing users). Absence is still
//! evidence of absence: a missing file reads as NOT contributed, or the gate
//! could never fire for a fresh account.
//!
//! The record is write-once per valid content: the FIRST acceptance is the
//! fact that matters ("has ever contributed"), so a later mark leaves a
//! valid file untouched. `--unmine` does NOT erase it — a past contribution
//! is a historical fact, not a consent state (the consent record is
//! [`crate::kat_mining_consent`]'s concern and stays separate).
//!
//! # TRANSITIONAL since Issue 080 (the server-truth swap)
//!
//! The 075 gate NO LONGER trusts this marker alone: the burn-ack v3
//! `contributed` field (written only from the authenticated burn-push
//! response) is the sticky-grace SERVER truth, and `Some(false)` refuses
//! even when a marker is present — a planted `contributed_once.json` no
//! longer lifts the gate. This marker now fills in ONLY while the server
//! has never said otherwise (`None`, a pre-v3 ack/server), so existing
//! contributed machines survive the fleet transition. DATED REMOVAL: once
//! the fleet is v3, `has_contributed` and this file die together (the
//! follow-up task on issue 080).

use std::path::Path;

/// The record shape version (bump = a new shape; a mismatch reads as
/// corrupt, which resolves anti-brick — see the module docs).
pub const CONTRIBUTION_VERSION: u8 = 1;

/// The marker file name under the account dir.
pub const FILE_NAME: &str = "contributed_once.json";

/// What one accepted sync push recorded (informational provenance — the
/// gate reads only the file's existence/validity).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContributionRecord {
    /// The record shape version ([`CONTRIBUTION_VERSION`]).
    pub version: u8,
    /// Unix seconds of the first accepted push.
    pub first_accepted_unix: u64,
    /// The accepted batch commitment (hex, full) — provenance only.
    pub commitment_hex: String,
}

/// Record an accepted push. Fail-soft: a write failure returns `Err` for the
/// caller to warn about — bookkeeping must never fail a sync that the
/// service already accepted.
///
/// # Errors
///
/// Filesystem failure (create dir / serialize / write).
pub fn mark(dir: &Path, commitment_hex: &str) -> Result<(), String> {
    if has_valid_record(dir) {
        return Ok(()); // write-once: the first acceptance is the fact
    }
    let record = ContributionRecord {
        version: CONTRIBUTION_VERSION,
        first_accepted_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        commitment_hex: commitment_hex.to_string(),
    };
    std::fs::create_dir_all(dir).map_err(|e| format!("contribution dir: {e}"))?;
    let text = serde_json::to_string_pretty(&record)
        .map_err(|e| format!("contribution serialize: {e}"))?;
    std::fs::write(dir.join(FILE_NAME), text).map_err(|e| format!("contribution write: {e}"))
}

/// The gate's input: has this machine's account ever had a telemetry batch
/// ACCEPTED? Missing → false (fresh account); valid → true; corrupt →
/// **true** (anti-brick; the module docs carry the direction argument).
///
/// Since issue 080 the gate calls this ONLY as the fallback while the burn
/// ack's `contributed` is `None` (a pre-v3 server) — never as the primary
/// truth; see the module docs.
#[must_use]
pub fn has_contributed(dir: &Path) -> bool {
    // Presence IS the fact: the file is only ever written by an accepted
    // push, so any present content — valid, corrupt, or a future version —
    // reads CONTRIBUTED (anti-brick); absence reads NOT contributed.
    dir.join(FILE_NAME).exists()
}

fn has_valid_record(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join(FILE_NAME))
        .ok()
        .and_then(|text| serde_json::from_str::<ContributionRecord>(&text).ok())
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_heal_contrib_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn missing_reads_not_contributed_then_mark_round_trips_write_once() {
        let dir = scratch("round");
        assert!(
            !has_contributed(&dir),
            "a fresh account has never contributed"
        );
        mark(&dir, "cafe").expect("first mark");
        assert!(has_contributed(&dir));

        // Write-once: a second mark leaves the FIRST record untouched.
        let first = std::fs::read_to_string(dir.join(FILE_NAME)).expect("read");
        mark(&dir, "beef").expect("second mark is a no-op");
        let second = std::fs::read_to_string(dir.join(FILE_NAME)).expect("read");
        assert_eq!(first, second, "the first acceptance is the recorded fact");
        assert!(second.contains("cafe"), "the first commitment survives");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_reads_contributed_and_mark_self_heals() {
        let dir = scratch("corrupt");
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join(FILE_NAME), "{not json").expect("corrupt");
        assert!(
            has_contributed(&dir),
            "a corrupt marker must resolve anti-brick (contributed)"
        );
        // A future accepted push rewrites the corrupt file (self-heal).
        mark(&dir, "cafe").expect("mark over corrupt");
        assert!(has_contributed(&dir));
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).expect("read");
        assert!(text.contains("cafe"), "the corrupt file was replaced");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn future_version_reads_contributed() {
        let dir = scratch("future");
        std::fs::create_dir_all(&dir).expect("dir");
        let future = format!(
            "{{\"version\":{},\"first_accepted_unix\":1,\"commitment_hex\":\"aa\"}}",
            CONTRIBUTION_VERSION + 1
        );
        std::fs::write(dir.join(FILE_NAME), future).expect("write");
        assert!(
            has_contributed(&dir),
            "a NEWER shape is not evidence of absence — resolve anti-brick"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
