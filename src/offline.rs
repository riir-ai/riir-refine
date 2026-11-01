//! The offline gate (Plan 325 T2; issue 192 — "a fully-offline mode
//! (`RIIR_REFINE_OFFLINE=1`) documented for air-gapped users").
//!
//! One predicate, consulted by EVERY network lane before it dials: the
//! update surface hooks, the points client, the burn-sync / fix-stats push
//! tails, and the hosted expert / escalation lanes. The env is the exact
//! literals `1` / `true` / `on` (the promote-only env law) — anything else
//! (unset included) is online. Offline is a PROMISE the binary must keep:
//! a lane that dials without consulting this module is a bug, and the
//! charitable gate's positive assertions pin the consults that exist today.
//!
//! Ungated by design: every build (public and internal) honors the env —
//! an air-gapped user must never depend on which features they compiled.
//!
//! Precedence vs the first-run consent: the env wins. A stored
//! "never send" answer and the env are the same posture; the env is
//! per-invocation, the stored answer is persistent (see
//! [`crate::first_run_consent`]).

/// The exact env name. A differently-cased or prefixed spelling does
/// nothing (the promote-only literal law — never a fuzzy match).
pub const OFFLINE_ENV: &str = "RIIR_REFINE_OFFLINE";

/// `true` iff the offline env is set to an exact active literal.
#[must_use]
pub fn offline() -> bool {
    matches!(
        std::env::var(OFFLINE_ENV).as_deref(),
        Ok("1" | "true" | "on")
    )
}

/// The one-line refusal a lane prints instead of dialing. Stable prefix
/// (`offline:`) so scripts can grep it; names the env so the user can turn
/// it off. Never an error-level exit by itself — lanes decide their own
/// exit posture (reads degrade, sends skip).
#[must_use]
pub fn refusal_line(lane: &str) -> String {
    format!("offline: {lane} skipped — {OFFLINE_ENV} is set (local healing is unaffected)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_literals_arm_and_everything_else_is_online() {
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
        let line = refusal_line("rank");
        assert!(line.starts_with("offline: rank skipped — "));
        assert!(line.contains(OFFLINE_ENV));
    }
}
