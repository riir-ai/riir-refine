//! The code-egress consent scope — ONE definition shared by every
//! transport that can send user code off-machine (Plan 202 §2 law 5).
//!
//! Sending user code to a hosted tier is a THIRD consent scope, more
//! sensitive than stats or mining consent: its `Unset` means REFUSE, the
//! accepted spelling is deliberate (never a generic truthy value), and
//! the refusal blocks the wire BEFORE any socket. Two consumers today:
//! R2's `HostedExpertL4` (generate) and R3a's `SeatRerankClient`
//! (rerank) — the same env, the same spelling, the same taxonomy. A
//! second spelling here would be the drift the workspace hunts: consent
//! that means two things means neither.
//!
//! Feature-gated as the ANY of its consumers (the `llm_spend` shape):
//! the types are pure std, and each consumer's feature lights them.

use std::time::Instant;

/// The code-egress consent env — its OWN scope, shared with no other
/// consent surface. The accepted value is [`EGRESS_ALLOW`]: a value that
/// names the payload shape, so consenting to send code off-machine is a
/// deliberate act, never a copied `=1`.
pub const EGRESS_ENV: &str = "RIIR_REFINE_CODE_EGRESS";

/// The only value that consents. Anything else — unset, blank, `0`,
/// `true`, `1`, junk — refuses (junk refuses LOUD, naming the accepted
/// spelling; a typo must never look like consent).
pub const EGRESS_ALLOW: &str = "snippet";

/// Why [`EgressConsent::parse`] refused to send code off-machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EgressRefusal {
    /// The env is unset/blank — the default posture is NO egress.
    Unset,
    /// The env is set to something that is not [`EGRESS_ALLOW`] — a
    /// typo or a copied generic value must never look like consent.
    NotConsent(String),
}

impl std::fmt::Display for EgressRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unset => write!(
                f,
                "{EGRESS_ENV} is unset — code egress is a scope of its own and \
                 no other consent implies it; set {EGRESS_ENV}={EGRESS_ALLOW} to \
                 allow sending snippets to the hosted expert"
            ),
            Self::NotConsent(got) => write!(
                f,
                "{EGRESS_ENV}={got:?} is not consent — the only accepted value is \
                 {EGRESS_ALLOW:?} (a generic truthy value is refused on purpose: \
                 consenting to send user code off-machine must be deliberate)"
            ),
        }
    }
}

/// The code-egress consent verdict — pure, testable without env races
/// (the `is_armed_with` / `operator_pins_from` seam pattern).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EgressConsent {
    Allowed,
    Refused(EgressRefusal),
}

impl EgressConsent {
    /// Parse the env value into the verdict.
    #[must_use]
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            None => Self::Refused(EgressRefusal::Unset),
            Some(v) if v.trim().is_empty() => Self::Refused(EgressRefusal::Unset),
            Some(v) if v.trim() == EGRESS_ALLOW => Self::Allowed,
            Some(v) => Self::Refused(EgressRefusal::NotConsent(v.trim().to_string())),
        }
    }

    /// The process-env form.
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse(std::env::var(EGRESS_ENV).ok().as_deref())
    }

    /// `true` only for [`EgressConsent::Allowed`].
    #[must_use]
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// The shared pre-socket deadline check: an already-expired budget means
/// nothing was (or will be) sent. Every hosted transport declines on it
/// BEFORE the socket — the conservative contract, spelled once.
///
/// `true` = the deadline has passed; the caller declines.
#[must_use]
pub fn deadline_expired(deadline: Instant) -> bool {
    deadline <= Instant::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_refuses_everything_but_the_deliberate_spelling() {
        assert_eq!(EgressConsent::parse(None), EgressConsent::Refused(EgressRefusal::Unset));
        assert_eq!(
            EgressConsent::parse(Some("")),
            EgressConsent::Refused(EgressRefusal::Unset)
        );
        assert_eq!(
            EgressConsent::parse(Some("   ")),
            EgressConsent::Refused(EgressRefusal::Unset)
        );
        assert_eq!(EgressConsent::parse(Some("snippet")), EgressConsent::Allowed);
        // Whitespace around the spelling is trimmed (the original R2
        // semantics — kept byte-identical by the move).
        assert_eq!(EgressConsent::parse(Some(" snippet ")), EgressConsent::Allowed);
        for junk in ["1", "true", "0", "yes", "file", "Snippet"] {
            let got = EgressConsent::parse(Some(junk));
            assert!(
                matches!(&got, EgressConsent::Refused(EgressRefusal::NotConsent(_))),
                "{junk:?} must not consent"
            );
        }
    }

    #[test]
    fn deadline_expired_is_the_passed_instant_only() {
        let future = Instant::now() + std::time::Duration::from_secs(60);
        assert!(!deadline_expired(future));
        let past = Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .unwrap();
        assert!(deadline_expired(past));
    }
}
