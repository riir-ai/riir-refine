//! Plan 225 T5.2 — the production SERVE boundary for an armed escalation
//! seat. The 2026-10-30 owner condition (plan 225 T5.2): the seat may be
//! wired into the REAL fix path only if the seat's answer still passes the
//! compile check (`--verify`) before it is served, with a test showing a
//! wrong seat pick is rejected and the modelless answer stands in.
//!
//! This module is that boundary in ONE place. It composes the existing
//! [`EscalationDispatch`] decision (worthiness → sticky ceiling → min-pool →
//! readiness → the R7 echo contract) with the caller's compile gate — no
//! parallel decision logic: the dispatch stays the only decider, the
//! compile gate stays the caller's (`fix_verify` in the fix lane). Every
//! non-answer is [`EscalatedFix::StoodByModelless`] — a decline keeps the
//! modelless fix, nothing panics (the R6 law).
//!
//! Worthiness mapping (the one production input the fix lane owns): a
//! SERVED modelless fix maps to score 1.0 — the gate's Worthy stand, the
//! seat is never consulted, the modelless answer IS the answer; a
//! modelless MISS maps to score 0.0 — a miss under the row's cap, the
//! dispatch's Proceed path, the seat's one case. A row that sets
//! `modelless_cap: 0.0` therefore never escalates at all (0.0 is not a
//! miss under 0.0) — the operator's off dial, by the gate's own grammar.

use crate::code_egress::EgressConsent;
use crate::escalation_dispatch::{EscalatedFix, EscalationDispatch, ReadinessPolicy, StandReason};
use std::time::Instant;

/// The per-seat-pick compile check: `true` = the fix may be served.
/// Production passes the `fix_verify`-backed gate; tests inject a
/// deterministic closure (the `_checked_with` seam pattern — no test
/// races a process env or compiles crates).
pub type ServeCompileCheck<'a> = &'a mut dyn FnMut(&str) -> bool;

/// The per-call transport + boundary parameters (the
/// [`EscalationDispatch::escalate_rerank_checked_with`] tail, plus the
/// owner condition's compile check).
pub struct ServeParams<'a> {
    /// The whole readiness-retry + wire budget for this case.
    pub deadline: Instant,
    /// The readiness policy (production: [`ReadinessPolicy::default`]).
    pub policy: ReadinessPolicy,
    /// The shared code-egress consent (the `_checked_with` seam — no test
    /// races a process env).
    pub consent: &'a EgressConsent,
    /// The serve-boundary compile gate — an `Answer` is served only when
    /// this returns `true` for the picked fix.
    pub compile_check: ServeCompileCheck<'a>,
}

/// The rerank arm's serve-boundary decision — the consent-injectable core.
///
/// An `Answer` that fails `compile_check` is demoted to
/// [`StandReason::Declined`] naming the boundary — the wrong pick never
/// reaches a served fix and the modelless answer stands.
pub fn serve_seat_rerank_checked_with(
    dispatch: &mut EscalationDispatch,
    modelless_served: bool,
    snippet: &str,
    pool: &[String],
    params: ServeParams<'_>,
) -> EscalatedFix {
    let score = if modelless_served { 1.0 } else { 0.0 };
    let ServeParams {
        deadline,
        policy,
        consent,
        compile_check,
    } = params;
    match dispatch.escalate_rerank_checked_with(score, snippet, pool, deadline, policy, consent) {
        #[cfg(feature = "seat_rerank")]
        EscalatedFix::Answer {
            fix,
            source,
            receipt,
        } => {
            if compile_check(&fix) {
                EscalatedFix::Answer {
                    fix,
                    source,
                    receipt,
                }
            } else {
                EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(
                        "seat pick failed the serve-boundary compile check — the modelless \
                         answer stands"
                            .to_string(),
                    ),
                }
            }
        }
        #[cfg(not(feature = "seat_rerank"))]
        // Unreachable in a hosted-only build (the dispatch only answers on
        // its own kind and rerank rows refuse to arm without the feature),
        // but the match must typecheck in every legal posture — and the
        // boundary contract still holds if it ever could.
        EscalatedFix::Answer { fix, source } => {
            let _ = source;
            if compile_check(&fix) {
                EscalatedFix::Answer { fix, source }
            } else {
                EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(
                        "seat pick failed the serve-boundary compile check — the modelless \
                         answer stands"
                            .to_string(),
                    ),
                }
            }
        }
        stand => stand,
    }
}

/// The production wrapper: consent read from the process env (the shared
/// code-egress scope — unset/junk refuses pre-socket), everything else
/// identical to [`serve_seat_rerank_checked_with`].
pub fn serve_seat_rerank(
    dispatch: &mut EscalationDispatch,
    modelless_served: bool,
    snippet: &str,
    pool: &[String],
    deadline: Instant,
    policy: ReadinessPolicy,
    compile_check: ServeCompileCheck<'_>,
) -> EscalatedFix {
    let consent = EgressConsent::from_env();
    serve_seat_rerank_checked_with(
        dispatch,
        modelless_served,
        snippet,
        pool,
        ServeParams {
            deadline,
            policy,
            consent: &consent,
            compile_check,
        },
    )
}
