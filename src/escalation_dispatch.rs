//! Plan 202 R6 — the escalation DISPATCH: the runtime consumer of the R1
//! manifest that joins armed rows to their shipped transports (R3a's
//! `SeatRerankClient`, R2's `HostedExpertL4`). This is the piece the
//! fixer-regate arm H drives and the production heal path will consume —
//! one definition of the chain's per-case arithmetic:
//!
//! ```text
//! worthiness (modelless_cap)  →  cost ceiling (the latch, sticky)
//!   → by row kind:
//!       seat_rerank   → pool ≥ min_pool ⇒ rerank (abstain ⇒ modelless stands)
//!       hosted_expert ⇒ generate the fix (decline ⇒ modelless stands)
//!   → every decline keeps the modelless answer
//! ```
//!
//! ## The laws this module inherits
//!
//! - **Manifest-only arming**: [`EscalationDispatch::arm`] returns `None`
//!   unless a manifest row exists, is not demoted, and the global
//!   demote-only env has not fired. No code flip ever arms escalation.
//! - **`min_pool` is the dispatch's gate, not the wire's** (the R7
//!   freeze): the client refuses an EMPTY list; the BOUND (default
//!   [`crate::escalation_manifest::DEFAULT_MIN_POOL`], row-raisable) is
//!   enforced HERE so a below-bound pool never becomes seat traffic at
//!   all — [`StandReason::BelowMinPool`] is a first-class outcome the
//!   measurement reads.
//! - **The readiness retry is deadline-capped** (Bench 107 finding 2): the
//!   instinct edge answers `503 {"code":"loading"}` while a lazy seat
//!   loads; the dispatch retries THAT ONE code under the REMAINING
//!   deadline with a small backoff, never past the budget — a late
//!   complete reply is dropped by the client anyway (compute-once).
//!   `lane_failed` and every other decline fall through immediately.
//! - **The latch is the cost ceiling**: each case's miss feeds the
//!   cost-ceiling latch; above `max_rate` the demotion is STICKY for the
//!   process lifetime (R1's owner rule — re-arm is an operator act).
//! - **A decline is always safe**: every non-answer path returns
//!   [`EscalatedFix::StoodByModelless`] and the caller serves the
//!   modelless fix it already has. Nothing here panics on remote output.

use std::time::{Duration, Instant};

use crate::code_egress::EgressConsent;
use crate::escalation_manifest::{DEMOTE_ENV, EscalationManifest, EscalationRow, TargetKind};

/// The suite every healing rerank rides (the R7 freeze: ONE suite for v1 —
/// one seat lane hosts the healing rerank expert; per-domain lanes wait
/// for measured need).
pub const HEAL_SUITE: &str = "heal";

/// The seat's presentation cap — moved to the manifest module (the
/// Issue-195 placebo reads it at the manifest-alone posture); re-exported
/// here so every existing dispatch-path spelling stays valid.
pub use crate::escalation_manifest::PRESENTATION_CAP;

/// The readiness-retry policy (the lazy 503 window). `max_attempts` is the
/// belt against a pathologically fast-failing seat; the DEADLINE is the
/// real bound — every attempt re-checks the remaining budget first.
#[derive(Clone, Copy, Debug)]
pub struct ReadinessPolicy {
    pub max_attempts: u8,
    pub backoff: Duration,
}

impl Default for ReadinessPolicy {
    fn default() -> Self {
        // Bench 107: the observed lazy window was ~100 ms under budgeted
        // retry; 12 × 25 ms ≈ 275 ms of sleep covers it many times over,
        // and the caller's deadline (arm-H default 2 s) is the ceiling.
        Self {
            max_attempts: 12,
            backoff: Duration::from_millis(25),
        }
    }
}

/// Why the modelless answer stands. EVERY variant is safe: the caller
/// already has the modelless fix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StandReason {
    /// The modelless score is at/above the row's `modelless_cap` — the
    /// worthiness gate says it STANDS (never escalates). `cap == 0.0`
    /// (the ALL-EMPTY posture) routes every case here.
    Worthy,
    /// The cost ceiling latched (sticky demotion above `max_rate`).
    RateLatched,
    /// The pool is below the row's `min_pool` bound — a rerank with
    /// nothing to choose between is a paid socket for a non-choice
    /// (the R7 freeze; Bench 107 finding 1).
    BelowMinPool,
    /// The seat abstained — the expert saw the candidates and declined to
    /// override. The modelless answer stands BY DESIGN (abstention is
    /// first-class).
    SeatAbstained,
    /// The hosted lane's fair-use drain pre-check declined (Proposal
    /// 027): the day's free allowance is spent and the cached balance
    /// cannot cover a billable call — the LOCAL mirror of the seat's
    /// enforcement, and a POLICY verdict like
    /// [`StandReason::SeatAbstained`], never a wire error. The modelless
    /// answer stands; the refuel path is contribution (the run-log
    /// disclosure names it — payment vocabulary is deliberately absent).
    DrainExhausted,
    /// The transport declined (consent / cap / deadline / wire / contract).
    /// Carries the short class text for the run log.
    Declined(String),
}

/// One dispatch decision. `Answer` means the HOSTED lane produced the fix
/// the caller should serve; `StoodByModelless` means serve the modelless
/// answer and record why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EscalatedFix {
    Answer {
        fix: String,
        /// `seat_pick` (rerank) or `hosted_generate` — the audit axis.
        source: &'static str,
        /// The seat's decision receipt when the lane published one
        /// (the R7 receipt tier; `None` on generate answers and on seats
        /// older than the receipt tier).
        #[cfg(feature = "seat_rerank")]
        receipt: Option<crate::seat_rerank::SeatReceipt>,
    },
    StoodByModelless {
        reason: StandReason,
    },
}

impl EscalatedFix {
    /// The seat receipt when this answer carries one (the R7 tier — the
    /// audit trail reads it without matching the enum; the receipt field
    /// is behind `seat_rerank`, so this accessor is too).
    #[cfg(feature = "seat_rerank")]
    #[must_use]
    pub fn receipt(&self) -> Option<&crate::seat_rerank::SeatReceipt> {
        match self {
            Self::Answer { receipt, .. } => receipt.as_ref(),
            Self::StoodByModelless { .. } => None,
        }
    }
}

/// Why [`EscalationDispatch::arm`] refused to build a dispatch. An armed
/// row whose transport is not COMPILED IN is a configuration error worth a
/// loud refusal — the row must never LOOK armed while doing nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmError(pub String);

impl std::fmt::Display for ArmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "escalation dispatch: {}", self.0)
    }
}

/// The armed dispatch for one domain: the manifest row + its rate latch +
/// the transport the row's kind names (built at arm time — construction is
/// inert for both transports; only the per-case calls egress).
pub struct EscalationDispatch {
    label: String,
    row: EscalationRow,
    latch: katgpt_core::escalation_guard::RollingRateLatch,
    /// Present iff the row's kind is `seat_rerank` (and the feature is
    /// compiled — `arm` refuses the row otherwise).
    #[cfg(feature = "seat_rerank")]
    seat: Option<crate::seat_rerank::SeatRerankClient>,
    /// Present iff the row's kind is `hosted_expert` (and the feature is
    /// compiled — `arm` refuses the row otherwise).
    #[cfg(feature = "hosted_expert")]
    expert: Option<crate::hosted_expert::HostedExpertL4>,
    /// The points-drain gate (Proposal 027 — the hosted seat's fair-use
    /// pre-check + per-call attestation). Construction is I/O-free; the
    /// process posture resolves the account dir + key lazily at the
    /// FIRST seat call. It exists exactly where it can act: the seat
    /// call path (a gate without the seat transport is dead weight).
    #[cfg(all(feature = "points_drain", feature = "seat_rerank"))]
    drain: crate::points_drain::DrainGate,
}

impl std::fmt::Debug for EscalationDispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No endpoint (embeds the host), no bearer — the CfClient rule.
        f.debug_struct("EscalationDispatch")
            .field("domain", &self.label)
            .field("kind", &self.row.target.kind)
            .finish_non_exhaustive()
    }
}

/// The shared worthiness + cost-ceiling gate verdict.
enum Gate {
    /// The modelless answer stands for this reason (already latched, or
    /// the score met the cap).
    Stand(StandReason),
    /// A miss under the ceiling — escalate per the row's kind.
    Proceed,
}

impl EscalationDispatch {
    /// Arm from the manifest for `label`. `Ok(None)` = not armed (absent
    /// row / row demoted / global kill switch) — the ALL-EMPTY posture,
    /// zero behavior, not an error. `Err` = the row is armed but cannot be
    /// served (transport missing from this build, endpoint/model absent).
    ///
    /// # Errors
    ///
    /// [`ArmError`] when an ARMED row names a transport this build does not
    /// compile, or the transport's construction data is absent (endpoint /
    /// model — no defaults, no env fallbacks).
    pub fn arm(manifest: &EscalationManifest, label: &str) -> Result<Option<Self>, ArmError> {
        Self::arm_with(manifest, label, std::env::var(DEMOTE_ENV).ok().as_deref())
    }

    /// The env-injectable composition of [`Self::arm`] (the
    /// `is_armed_with` seam pattern — no test races a process env).
    ///
    /// # Errors
    ///
    /// Same taxonomy as [`Self::arm`].
    ///
    /// # Panics
    ///
    /// Only through the manifest's own invariants failing — the row's kind
    /// was validated at load, so `from_label` and the row lookup cannot
    /// miss. A hand-built (never-validated) manifest can panic here by
    /// design: arming an invalid row must be loud, not silent.
    pub fn arm_with(
        manifest: &EscalationManifest,
        label: &str,
        env: Option<&str>,
    ) -> Result<Option<Self>, ArmError> {
        if !manifest.is_armed_with(label, env) {
            return Ok(None);
        }
        let row = manifest
            .row(label)
            .expect("is_armed_with checked the row")
            .clone();
        let kind = TargetKind::from_label(&row.target.kind)
            .expect("manifest validation refused unknown kinds at load");
        let endpoint = || {
            row.target.endpoint.clone().ok_or_else(|| {
                ArmError(format!(
                    "escalation.{label}.target.endpoint: the row is armed but names no \
                     endpoint (the transports have no default, no env fallback)"
                ))
            })
        };
        // Feature-off refusals come FIRST (an armed row must never look
        // armed while doing nothing) — outside the matches, so the matches'
        // Ok type stays nameable in every build.
        #[cfg(not(feature = "seat_rerank"))]
        if matches!(kind, TargetKind::SeatRerank) {
            return Err(ArmError(format!(
                "escalation.{label}: armed row targets seat_rerank but this build was \
                 compiled without the seat_rerank feature — rebuild with it"
            )));
        }
        #[cfg(not(feature = "hosted_expert"))]
        if matches!(kind, TargetKind::HostedExpert) {
            return Err(ArmError(format!(
                "escalation.{label}: armed row targets hosted_expert but this build \
                 was compiled without the hosted_expert feature — rebuild with it"
            )));
        }
        if matches!(kind, TargetKind::L4Daemon) {
            return Err(ArmError(format!(
                "escalation.{label}: l4_daemon has no dispatch wiring yet (the daemon \
                 transport predates the manifest and spawns from explicit \
                 bin/gguf/lora args; its manifest arming lane is R6 follow-up)"
            )));
        }
        #[cfg(feature = "seat_rerank")]
        let seat = match kind {
            TargetKind::SeatRerank => Some(
                crate::seat_rerank::SeatRerankClient::new(endpoint()?.as_str(), HEAL_SUITE)
                    .map_err(|e| ArmError(format!("escalation.{label}: {e}")))?,
            ),
            _ => None,
        };
        #[cfg(feature = "hosted_expert")]
        let expert = match kind {
            TargetKind::HostedExpert => {
                let model = row.target.model.clone().ok_or_else(|| {
                    ArmError(format!(
                        "escalation.{label}.target.model: hosted_expert rows name the \
                         served expert (manifest validation enforces this — a hand-built \
                         row bypassing validate() cannot be served)"
                    ))
                })?;
                Some(crate::hosted_expert::HostedExpertL4::new(
                    endpoint()?,
                    model,
                    crate::llm_spend::SpendLedger::open(std::path::Path::new(".")),
                ))
            }
            _ => None,
        };
        #[cfg(all(feature = "points_drain", feature = "seat_rerank"))]
        let drain = crate::points_drain::DrainGate::for_process();
        Ok(Some(Self {
            label: label.to_string(),
            latch: katgpt_core::escalation_guard::RollingRateLatch::with_window(
                katgpt_core::escalation_guard::DEFAULT_WINDOW,
                None,
                row.max_rate,
            ),
            row,
            #[cfg(feature = "seat_rerank")]
            seat,
            #[cfg(feature = "hosted_expert")]
            expert,
            #[cfg(all(feature = "points_drain", feature = "seat_rerank"))]
            drain,
        }))
    }

    /// The drain-injectable test seam (plan 326 T6): replace the
    /// process-resolved gate with an injected one BEFORE any call — the
    /// `is_armed_with` seam pattern, so no test ever touches the real
    /// account dir or races a process env.
    #[cfg(all(test, feature = "points_drain", feature = "seat_rerank"))]
    #[must_use]
    pub fn with_drain_gate(mut self, gate: crate::points_drain::DrainGate) -> Self {
        self.drain = gate;
        self
    }

    /// The domain label the row is armed under.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The armed row (cap, ceiling, target — the reporting surface).
    #[must_use]
    pub fn row(&self) -> &EscalationRow {
        &self.row
    }

    /// The spend ledger's one-line status (hosted rows only; `None` on a
    /// rerank dispatch — the seat is a decision-plane edge, no tokens).
    #[must_use]
    #[cfg(feature = "hosted_expert")]
    pub fn spend_summary_line(&self) -> Option<String> {
        self.expert.as_ref().map(|e| e.spend().summary_line())
    }

    /// The shared worthiness + cost-ceiling gate: ONE miss observation per
    /// case (a miss = score below the row's cap), then the sticky-latch
    /// check, then the worthiness verdict. The case that TIPS the window
    /// over the ceiling still escalates (its miss was not known at entry);
    /// every case after stands.
    fn gate(&mut self, modelless_score: f32) -> Gate {
        let cap = self.row.modelless_cap;
        let miss = f64::from(modelless_score) < cap;
        self.latch.observe(miss);
        if self.latch.is_latched() {
            return Gate::Stand(StandReason::RateLatched);
        }
        if !miss {
            return Gate::Stand(StandReason::Worthy);
        }
        Gate::Proceed
    }

    /// The rerank arm's full per-case decision — the seat-lane entry
    /// point. Worthiness → ceiling → `min_pool` (pre-socket) → readiness
    /// retry → contract. Every non-answer is
    /// [`EscalatedFix::StoodByModelless`].
    // The T4 per-repo consult adds the run's repo root + the --yes arm to
    // the env-reading entry (the production wiring's full signature).
    #[allow(clippy::too_many_arguments, reason = "the rerank decision's full input set")]
    pub fn escalate_rerank(
        &mut self,
        modelless_score: f32,
        snippet: &str,
        pool: &[String],
        deadline: Instant,
        policy: ReadinessPolicy,
        repo_root: &std::path::Path,
        yes_armed: bool,
    ) -> EscalatedFix {
        let consent = EgressConsent::from_env();
        // The T4 consult (the ask-once-per-repo flow) — the account dir is
        // the caller-resolution seam's own; Disabled while the lane is dark.
        let dir = riir_kat::kat_mining_consent::default_dir();
        let repo_consent =
            crate::repo_consent::consult(&dir, repo_root, snippet.lines().count(), yes_armed);
        self.escalate_rerank_checked_with(
            modelless_score,
            snippet,
            pool,
            deadline,
            policy,
            &consent,
            &repo_consent,
        )
    }

    /// The consent-injectable core of [`Self::escalate_rerank`] (the
    /// `rerank_checked_with` seam pattern — no test races a process env).
    /// `repo_consent` is the T4 per-repo decision (Plan 333) — consult
    /// after the gate, before any transport; `Disabled` is the shipped
    /// dark posture (no gate, byte-identical pre-T4 behavior).
    #[allow(clippy::too_many_arguments, reason = "the rerank decision's full input set")]
    pub fn escalate_rerank_checked_with(
        &mut self,
        modelless_score: f32,
        snippet: &str,
        pool: &[String],
        deadline: Instant,
        policy: ReadinessPolicy,
        consent: &EgressConsent,
        repo_consent: &crate::repo_consent::RepoConsentDecision,
    ) -> EscalatedFix {
        #[cfg(not(feature = "seat_rerank"))]
        let _ = (snippet, deadline, policy, consent, repo_consent);
        if !matches!(self.row.target.kind.as_str(), "seat_rerank") {
            return EscalatedFix::StoodByModelless {
                reason: StandReason::Declined(format!(
                    "dispatch targets {} — not a rerank lane",
                    self.row.target.kind
                )),
            };
        }
        match self.gate(modelless_score) {
            Gate::Stand(reason) => return EscalatedFix::StoodByModelless { reason },
            Gate::Proceed => {}
        }
        // Plan 333 T4: the per-repo consent — AFTER the gate (a gated-out
        // case never spends the consult) and BEFORE any transport work
        // (a refusal never becomes a metered call, the drain pre-check's
        // law). `Disabled` falls through silently — the dark posture.
        if let crate::repo_consent::RepoConsentDecision::Refused(vocab) = repo_consent {
            return EscalatedFix::StoodByModelless {
                reason: StandReason::Declined((*vocab).to_string()),
            };
        }
        // The R7 freeze's enforcement consumer: BELOW the row's bound the
        // pool never becomes seat traffic (the client's empty-list refusal
        // is the structural backstop, not the gate).
        let min_pool = self.row.target.min_pool_or_default();
        if pool.len() < min_pool {
            return EscalatedFix::StoodByModelless {
                reason: StandReason::BelowMinPool,
            };
        }
        #[cfg(feature = "seat_rerank")]
        let presented: Vec<String> = pool.iter().take(PRESENTATION_CAP).cloned().collect();
        #[cfg(not(feature = "seat_rerank"))]
        let _ = pool;
        #[cfg(feature = "seat_rerank")]
        if let Some(client) = self.seat.as_ref() {
            // ONE body builder (the R7 rule), pre-built here so the
            // drain's attestation signs the EXACT bytes on the wire.
            let body = match client.build_body(snippet, &presented) {
                Ok(body) => body,
                // min_pool ≥ the frozen default (2) makes this
                // unreachable; the conservative contract declines rather
                // than proves it.
                Err(e) => {
                    return EscalatedFix::StoodByModelless {
                        reason: StandReason::Declined(e.to_string()),
                    };
                }
            };
            // Proposal 027's UX pre-check (plan 326 T3): AFTER `min_pool`
            // (BelowMinPool never consumes) and IMMEDIATELY before the
            // transport. A refused consent skips the gate — the client's
            // own refusal below is the verdict, and a refusal never
            // becomes a metered call. Inert unless an account key AND
            // served constants exist; fails open on any local error.
            #[cfg(feature = "points_drain")]
            let attested = if matches!(consent, EgressConsent::Allowed) {
                match self.drain.begin_hosted_call(body.as_bytes()) {
                    Ok(attested) => attested,
                    Err(crate::points_drain::DrainDeclined) => {
                        return EscalatedFix::StoodByModelless {
                            reason: StandReason::DrainExhausted,
                        };
                    }
                }
            } else {
                crate::points_drain::AttestedCall::default()
            };
            #[cfg(feature = "points_drain")]
            let headers = attested.header_refs();
            #[cfg(not(feature = "points_drain"))]
            let headers: Vec<(&str, &str)> = Vec::new();
            return match rerank_with_readiness_checked_with_headers(
                client, &body, &presented, &headers, deadline, policy, consent,
            ) {
                Ok(crate::seat_rerank::SeatAnswer::Pick {
                    candidate, receipt, ..
                }) => EscalatedFix::Answer {
                    fix: candidate,
                    source: "seat_pick",
                    receipt,
                },
                Ok(crate::seat_rerank::SeatAnswer::Abstained { .. }) => {
                    EscalatedFix::StoodByModelless {
                        reason: StandReason::SeatAbstained,
                    }
                }
                Err(e) => EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(e.to_string()),
                },
            };
        }
        // Kind seat_rerank with no client: `arm` refuses the row when the
        // feature is off, so this is unreachable in any build that can hold
        // a seat dispatch — but the conservative contract declines rather
        // than proves it.
        EscalatedFix::StoodByModelless {
            reason: StandReason::Declined("seat transport not built".to_string()),
        }
    }

    /// The generate arm's full per-case decision — the hosted-expert-lane
    /// entry point. Worthiness → ceiling → R2's four-point call. Every
    /// non-answer is [`EscalatedFix::StoodByModelless`].
    pub fn escalate_generate(
        &mut self,
        modelless_score: f32,
        snippet: &str,
        deadline: Instant,
        max_tokens: u64,
        repo_root: &std::path::Path,
        yes_armed: bool,
    ) -> EscalatedFix {
        let consent = EgressConsent::from_env();
        let dir = riir_kat::kat_mining_consent::default_dir();
        let repo_consent =
            crate::repo_consent::consult(&dir, repo_root, snippet.lines().count(), yes_armed);
        self.escalate_generate_checked_with(
            modelless_score,
            snippet,
            deadline,
            max_tokens,
            &consent,
            &repo_consent,
        )
    }

    /// The consent-injectable core of [`Self::escalate_generate`] — the
    /// same T4 per-repo consult as the rerank core (after the gate,
    /// before the transport; `Disabled` = the dark shipped posture).
    pub fn escalate_generate_checked_with(
        &mut self,
        modelless_score: f32,
        snippet: &str,
        deadline: Instant,
        max_tokens: u64,
        consent: &EgressConsent,
        repo_consent: &crate::repo_consent::RepoConsentDecision,
    ) -> EscalatedFix {
        #[cfg(not(feature = "hosted_expert"))]
        let _ = (snippet, deadline, max_tokens, consent, repo_consent);
        if !matches!(self.row.target.kind.as_str(), "hosted_expert") {
            return EscalatedFix::StoodByModelless {
                reason: StandReason::Declined(format!(
                    "dispatch targets {} — not a generate lane",
                    self.row.target.kind
                )),
            };
        }
        match self.gate(modelless_score) {
            Gate::Stand(reason) => return EscalatedFix::StoodByModelless { reason },
            Gate::Proceed => {}
        }
        // Plan 333 T4: the per-repo consent — same placement as the rerank
        // core (after the gate, before the transport).
        if let crate::repo_consent::RepoConsentDecision::Refused(vocab) = repo_consent {
            return EscalatedFix::StoodByModelless {
                reason: StandReason::Declined((*vocab).to_string()),
            };
        }
        #[cfg(feature = "hosted_expert")]
        if let Some(expert) = self.expert.as_mut() {
            return match expert.generate_fix_checked_with(
                snippet,
                Some(deadline),
                max_tokens,
                consent.clone(),
            ) {
                Ok(fix) => EscalatedFix::Answer {
                    fix,
                    source: "hosted_generate",
                    #[cfg(feature = "seat_rerank")]
                    receipt: None,
                },
                Err(e) => EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(e.to_string()),
                },
            };
        }
        EscalatedFix::StoodByModelless {
            reason: StandReason::Declined("hosted transport not built".to_string()),
        }
    }
}

/// The readiness retry over [`crate::seat_rerank::SeatRerankClient::rerank`]
/// (env-consent form). See [`rerank_with_readiness_checked`].
///
/// # Errors
///
/// The client's [`crate::seat_rerank::SeatDecline`] taxonomy; the
/// loading-window exhaustion path reports
/// [`crate::seat_rerank::SeatDecline::DeadlineExpired`] (the budget, not
/// the attempt count, is the cause).
#[cfg(feature = "seat_rerank")]
pub fn rerank_with_readiness(
    client: &crate::seat_rerank::SeatRerankClient,
    snippet: &str,
    pool: &[String],
    deadline: Instant,
    policy: ReadinessPolicy,
) -> Result<crate::seat_rerank::SeatAnswer, crate::seat_rerank::SeatDecline> {
    let consent = EgressConsent::from_env();
    rerank_with_readiness_checked(client, snippet, pool, deadline, policy, &consent)
}

/// The consent-injectable core. Retries ONLY the lazy seat's loading code
/// (`503 {"code":"loading"}` → the client's `Wire("loading")`); every other
/// decline — `lane_failed`, contract violations, consent, deadlines —
/// falls through on the FIRST attempt, and the whole loop is bounded by
/// the caller's deadline (checked before every sleep).
///
/// # Errors
///
/// Same taxonomy as [`rerank_with_readiness`].
#[cfg(feature = "seat_rerank")]
pub fn rerank_with_readiness_checked(
    client: &crate::seat_rerank::SeatRerankClient,
    snippet: &str,
    pool: &[String],
    deadline: Instant,
    policy: ReadinessPolicy,
    consent: &EgressConsent,
) -> Result<crate::seat_rerank::SeatAnswer, crate::seat_rerank::SeatDecline> {
    let body = client.build_body(snippet, pool)?;
    rerank_with_readiness_checked_with_headers(client, &body, pool, &[], deadline, policy, consent)
}

/// The headers-carrying core of the readiness retry (plan 326 T5): the
/// SAME law — retries ONLY the lazy seat's loading code, every other
/// decline falls through on the first attempt, the whole loop bounded by
/// the caller's deadline — over a PRE-BUILT body + the per-call
/// attestation headers. The drain composes the headers once per
/// dispatch call; a readiness retry re-sends the identical request (the
/// same logical call, not a second metered one).
///
/// # Errors
///
/// Same taxonomy as [`rerank_with_readiness`].
#[cfg(feature = "seat_rerank")]
pub fn rerank_with_readiness_checked_with_headers(
    client: &crate::seat_rerank::SeatRerankClient,
    body: &str,
    pool: &[String],
    headers: &[(&str, &str)],
    deadline: Instant,
    policy: ReadinessPolicy,
    consent: &EgressConsent,
) -> Result<crate::seat_rerank::SeatAnswer, crate::seat_rerank::SeatDecline> {
    for _ in 0..policy.max_attempts {
        match client.rerank_body_checked_with(body, pool, headers, deadline, consent.clone()) {
            Ok(answer) => return Ok(answer),
            // The lazy seat's loading window: the ONE retryable decline.
            Err(crate::seat_rerank::SeatDecline::Wire(code)) if code == "loading" => {
                if Instant::now() + policy.backoff >= deadline {
                    return Err(crate::seat_rerank::SeatDecline::DeadlineExpired);
                }
                std::thread::sleep(policy.backoff);
            }
            Err(e) => return Err(e),
        }
    }
    Err(crate::seat_rerank::SeatDecline::Wire(format!(
        "readiness: {} attempts and the seat is still loading — standing down \
         (eager-load the row or raise the deadline)",
        policy.max_attempts
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest parser + the arming semantics — every compiled posture
    /// of this module runs these.
    fn manifest_from(json: &str) -> EscalationManifest {
        let m: EscalationManifest = serde_json::from_str(json).expect("manifest json");
        m.validate().expect("manifest valid");
        m
    }

    #[test]
    fn unarmed_manifest_is_none_zero_behavior() {
        let m = manifest_from("{}");
        assert!(
            EscalationDispatch::arm_with(&m, "clippy", None)
                .expect("arm ok")
                .is_none(),
            "the ALL-EMPTY posture arms nothing"
        );
        // A present-but-demoted row and the exact-literal env both demote;
        // junk env leaves an armed row armed (the demote-only law).
        let json = "{\"domains\":{\"clippy\":{\"modelless_cap\":0.5,\"max_rate\":0.5,\
                    \"demoted\":true,\"target\":{\"kind\":\"seat_rerank\",\
                    \"endpoint\":\"http://127.0.0.1:9\"}}}}";
        let m = manifest_from(json);
        let armed = EscalationDispatch::arm_with(&m, "clippy", None);
        assert!(
            matches!(armed, Ok(None)),
            "a demoted row arms nothing: {armed:?}"
        );
        assert!(
            matches!(
                EscalationDispatch::arm_with(&m, "clippy", Some("0")),
                Ok(None)
            ),
            "the exact-literal kill switch demotes"
        );
        let json = "{\"domains\":{\"clippy\":{\"modelless_cap\":0.5,\"max_rate\":0.5,\
                    \"target\":{\"kind\":\"seat_rerank\",\"endpoint\":\"http://127.0.0.1:9\"}}}}";
        let m = manifest_from(json);
        let armed = EscalationDispatch::arm_with(&m, "clippy", Some("1"));
        assert!(
            matches!(armed, Ok(Some(_))),
            "junk env stays armed: {armed:?}"
        );
    }

    /// The seat-lane tests — need the seat transport compiled (`arm` would
    /// refuse the row otherwise, and the wire types would not exist).
    #[cfg(feature = "seat_rerank")]
    mod seat_tests {
        use super::*;
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::mpsc::{Receiver, channel};

        const SNIPPET: &str = "fn foo() -> i32 { return 42; }";
        const BASE: &str = "http://127.0.0.1:9";

        fn seat_row_json(cap: f64, extra_target: &str) -> String {
            let extra = if extra_target.is_empty() {
                String::new()
            } else {
                format!(",{extra_target}")
            };
            format!(
                "{{\"domains\":{{\"clippy\":{{\"modelless_cap\":{cap},\"max_rate\":0.5,\
                 \"target\":{{\"kind\":\"seat_rerank\",\"endpoint\":\"{BASE}\"{extra}}}}}}}}}"
            )
        }

        /// A seat server that answers each connection with the NEXT response
        /// in the list (the loading-window rehearsal) and records every
        /// REQUEST body for the wire-boundary assertions.
        fn fake_seat_multi(responses: &[(&str, &str)]) -> (String, Receiver<Vec<u8>>) {
            let listener = TcpListener::bind("127.0.0.1:0").expect("loopback bind");
            let addr = listener.local_addr().expect("addr");
            let owned: Vec<(String, String)> = responses
                .iter()
                .map(|(s, b)| ((*s).to_string(), (*b).to_string()))
                .collect();
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                for (status, body) in &owned {
                    let Ok((mut sock, _)) = listener.accept() else {
                        break;
                    };
                    // Read the whole request (the R2 rule), record, answer.
                    let mut buf = Vec::with_capacity(16_384);
                    let mut chunk = [0u8; 4096];
                    loop {
                        let n = sock.read(&mut chunk).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                        if let Some(pos) =
                            buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
                        {
                            let head = String::from_utf8_lossy(&buf[..pos]).to_ascii_lowercase();
                            let clen: usize = head
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:"))
                                .and_then(|v| v.trim().parse().ok())
                                .unwrap_or(0);
                            if buf.len() >= pos + clen {
                                break;
                            }
                        }
                    }
                    let _ = tx.send(buf);
                    let http = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = sock.write_all(http.as_bytes());
                }
            });
            (format!("http://{addr}"), rx)
        }

        /// Count the `options` array length in a recorded request body.
        fn sent_options_len(req: &[u8]) -> usize {
            let raw = String::from_utf8_lossy(req);
            let body = raw.rsplit("\r\n\r\n").next().unwrap_or("");
            let v: serde_json::Value = serde_json::from_str(body).expect("request body json");
            v["options"].as_array().map_or(0, Vec::len)
        }

        /// Drain the recorded requests; the count is the request count.
        fn drain(rx: &Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
            let mut out = Vec::new();
            while let Ok(req) = rx.try_recv() {
                out.push(req);
            }
            out.shrink_to_fit();
            out
        }

        fn ok_pick_doc(pick: usize) -> String {
            format!(
                "{{\"suite\":\"heal\",\"options\":[\"a\",\"b\"],\"pick\":\"{}\",\
                 \"pick_index\":{pick},\"abstained\":false,\"confidence\":0.9}}",
                ["a", "b"][pick]
            )
        }

        fn pool2() -> Vec<String> {
            vec!["a".to_string(), "b".to_string()]
        }

        fn armed_seat(json: &str) -> EscalationDispatch {
            EscalationDispatch::arm_with(&manifest_from(json), "clippy", None)
                .expect("arm ok")
                .expect("armed")
        }

        /// A listener that must accept NOTHING — bound, then switched to
        /// nonblocking (the `seat_rerank` tests' pattern, local copy).
        struct SilentListener(TcpListener);

        impl SilentListener {
            fn bind() -> Self {
                let l = TcpListener::bind("127.0.0.1:0").expect("silent bind");
                l.set_nonblocking(true).expect("nonblocking");
                Self(l)
            }
            fn assert_no_connections(&self) {
                std::thread::sleep(Duration::from_millis(120));
                assert!(
                    self.0.accept().is_err(),
                    "a socket was opened despite the pre-socket refusal"
                );
            }
        }

        #[test]
        fn armed_seat_row_builds_and_kind_checked_methods_decline() {
            let mut d = armed_seat(&seat_row_json(0.5, ""));
            assert_eq!(d.label(), "clippy");
            // The generate entry point on a rerank dispatch is a caller bug
            // — declined, never a panic, never a socket.
            let deadline = Instant::now() + Duration::from_secs(1);
            let fix = d.escalate_generate_checked_with(
                0.0,
                SNIPPET,
                deadline,
                128,
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            match fix {
                EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(msg),
                } => assert!(msg.contains("not a generate lane"), "{msg}"),
                other => panic!("expected a decline, got {other:?}"),
            }
        }

        #[test]
        fn worthy_score_stands_without_a_socket() {
            let listener = SilentListener::bind();
            let mut d = armed_seat(&seat_row_json(0.5, ""));
            let deadline = Instant::now() + Duration::from_secs(1);
            let fix = d.escalate_rerank_checked_with(
                0.6, // >= cap 0.5: the worthiness gate says STANDS
                SNIPPET,
                &pool2(),
                deadline,
                ReadinessPolicy::default(),
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            assert_eq!(
                fix,
                EscalatedFix::StoodByModelless {
                    reason: StandReason::Worthy
                }
            );
            listener.assert_no_connections();
        }

        #[test]
        fn min_pool_gates_pre_socket() {
            let listener = SilentListener::bind();
            let mut d = armed_seat(&seat_row_json(0.5, ""));
            let deadline = Instant::now() + Duration::from_secs(1);
            let fix = d.escalate_rerank_checked_with(
                0.1,
                SNIPPET,
                &["the-only-candidate".to_string()], // 1 < the frozen default 2
                deadline,
                ReadinessPolicy::default(),
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            assert_eq!(
                fix,
                EscalatedFix::StoodByModelless {
                    reason: StandReason::BelowMinPool
                }
            );
            // A raised row bound gates the same way at its own value.
            let mut d = armed_seat(&seat_row_json(0.5, "\"min_pool\":3"));
            let fix = d.escalate_rerank_checked_with(
                0.1,
                SNIPPET,
                &pool2(), // 2 < the row's 3
                deadline,
                ReadinessPolicy::default(),
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            assert_eq!(
                fix,
                EscalatedFix::StoodByModelless {
                    reason: StandReason::BelowMinPool
                }
            );
            listener.assert_no_connections();
        }

        #[test]
        fn readiness_retry_rides_the_loading_window_then_picks() {
            // Two 503-loading windows, then the pick — the Bench-107
            // lazy-seat rehearsal. Three requests must arrive; a Pick.
            let (addr, rx) = fake_seat_multi(&[
                (
                    "503 Service Unavailable",
                    "{\"code\":\"loading\",\"error\":\"lazy\"}",
                ),
                (
                    "503 Service Unavailable",
                    "{\"code\":\"loading\",\"error\":\"still\"}",
                ),
                ("200 OK", &ok_pick_doc(1)),
            ]);
            let json = seat_row_json(0.5, "").replace(BASE, &addr);
            let mut d = armed_seat(&json);
            let deadline = Instant::now() + Duration::from_secs(5);
            let fix = d.escalate_rerank_checked_with(
                0.1,
                SNIPPET,
                &pool2(),
                deadline,
                ReadinessPolicy {
                    max_attempts: 6,
                    backoff: Duration::from_millis(1),
                },
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            match fix {
                EscalatedFix::Answer { fix, source, .. } => {
                    assert_eq!(fix, "b");
                    assert_eq!(source, "seat_pick");
                }
                other @ EscalatedFix::StoodByModelless { .. } => {
                    panic!("expected the seat's pick, got {other:?}");
                }
            }
            assert_eq!(drain(&rx).len(), 3, "two loading retries + the pick");
        }

        #[test]
        fn lane_failed_declines_on_the_first_attempt() {
            let (addr, rx) = fake_seat_multi(&[
                (
                    "503 Service Unavailable",
                    "{\"code\":\"lane_failed\",\"error\":\"boom\"}",
                ),
                ("200 OK", &ok_pick_doc(0)), // must NEVER be reached
            ]);
            let json = seat_row_json(0.5, "").replace(BASE, &addr);
            let mut d = armed_seat(&json);
            let deadline = Instant::now() + Duration::from_secs(5);
            let fix = d.escalate_rerank_checked_with(
                0.1,
                SNIPPET,
                &pool2(),
                deadline,
                ReadinessPolicy::default(),
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            match fix {
                EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(msg),
                } => assert!(msg.contains("lane_failed"), "{msg}"),
                other => panic!("expected the lane_failed decline, got {other:?}"),
            }
            assert_eq!(drain(&rx).len(), 1, "no retry on lane_failed");
        }

        #[test]
        fn exhausted_loading_window_reports_the_budget() {
            // Four loading answers vs a 4-attempt belt and a 120 ms
            // deadline: the sleep-budget check trips BEFORE the belt runs
            // out, so the decline is the DEADLINE class (the budget is the
            // cause, not the attempt count).
            let (addr, rx) = fake_seat_multi(&[
                ("503 Service Unavailable", "{\"code\":\"loading\"}"),
                ("503 Service Unavailable", "{\"code\":\"loading\"}"),
                ("503 Service Unavailable", "{\"code\":\"loading\"}"),
                ("503 Service Unavailable", "{\"code\":\"loading\"}"),
            ]);
            let json = seat_row_json(0.5, "").replace(BASE, &addr);
            let mut d = armed_seat(&json);
            let deadline = Instant::now() + Duration::from_millis(120);
            let fix = d.escalate_rerank_checked_with(
                0.1,
                SNIPPET,
                &pool2(),
                deadline,
                ReadinessPolicy {
                    max_attempts: 4,
                    backoff: Duration::from_millis(50),
                },
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            match fix {
                EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(msg),
                } => assert!(
                    msg.contains("deadline expired") || msg.contains("still loading"),
                    "{msg}"
                ),
                other => panic!("expected the stand-down decline, got {other:?}"),
            }
            let n = drain(&rx).len();
            assert!(
                (1..=4).contains(&n),
                "requests bounded by the belt, got {n}"
            );
        }

        #[test]
        fn abstain_stands() {
            let (addr, rx) = fake_seat_multi(&[(
                "200 OK",
                "{\"suite\":\"heal\",\"options\":[\"a\",\"b\"],\"pick\":null,\
                 \"pick_index\":null,\"abstained\":true}",
            )]);
            let json = seat_row_json(0.5, "").replace(BASE, &addr);
            let mut d = armed_seat(&json);
            let deadline = Instant::now() + Duration::from_secs(5);
            let fix = d.escalate_rerank_checked_with(
                0.1,
                SNIPPET,
                &pool2(),
                deadline,
                ReadinessPolicy::default(),
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            assert_eq!(
                fix,
                EscalatedFix::StoodByModelless {
                    reason: StandReason::SeatAbstained
                }
            );
            assert_eq!(drain(&rx).len(), 1);
        }

        #[test]
        fn presentation_cap_truncates_the_pool_to_eight() {
            // The seat S8 cap: a 10-candidate pool presents 8, read back
            // from the recorded request body at the wire boundary. The
            // server's echo ([a,b]) mismatches the 8 sent, so the contract
            // decline is expected; the assertion is what LEFT the box.
            let (addr, rx) = fake_seat_multi(&[("200 OK", &ok_pick_doc(0))]);
            let json = seat_row_json(0.5, "").replace(BASE, &addr);
            let mut d = armed_seat(&json);
            let pool: Vec<String> = (0..10).map(|i| format!("c{i}")).collect();
            let deadline = Instant::now() + Duration::from_secs(5);
            let fix = d.escalate_rerank_checked_with(
                0.1,
                SNIPPET,
                &pool,
                deadline,
                ReadinessPolicy::default(),
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            match fix {
                EscalatedFix::StoodByModelless {
                    reason: StandReason::Declined(msg),
                } => assert!(msg.contains("echo mismatch"), "{msg}"),
                other => panic!("expected the echo contract decline, got {other:?}"),
            }
            let reqs = drain(&rx);
            assert_eq!(reqs.len(), 1);
            assert_eq!(
                sent_options_len(&reqs[0]),
                PRESENTATION_CAP,
                "the pool is presented truncated to the seat's cap"
            );
        }

        #[test]
        fn latch_latches_sticky_above_the_ceiling_without_sockets() {
            let listener = SilentListener::bind();
            // max_rate 0.5: every below-min-pool case is a MISS (score
            // below cap) feeding the window — the gate fires pre-socket,
            // so the latch is testable without a seat.
            let mut d = armed_seat(&seat_row_json(0.5, ""));
            let deadline = Instant::now() + Duration::from_secs(1);
            let policy = ReadinessPolicy::default();
            let single = vec!["only".to_string()];
            let mut saw_below_min = false;
            for _ in 0..katgpt_core::escalation_guard::DEFAULT_WINDOW {
                let fix = d.escalate_rerank_checked_with(
                    0.1,
                    SNIPPET,
                    &single,
                    deadline,
                    policy,
                    &EgressConsent::Allowed,
                    &crate::repo_consent::RepoConsentDecision::disabled(),
                );
                if let EscalatedFix::StoodByModelless {
                    reason: StandReason::BelowMinPool,
                } = &fix
                {
                    saw_below_min = true;
                }
            }
            assert!(saw_below_min, "the misses fed the window");
            // Window full at 100% misses > 0.5 -> latched. The NEXT case
            // stands by the ceiling — even a WORTHY score never un-latches
            // (sticky).
            let fix = d.escalate_rerank_checked_with(
                0.9,
                SNIPPET,
                &pool2(),
                deadline,
                policy,
                &EgressConsent::Allowed,
                &crate::repo_consent::RepoConsentDecision::disabled(),
            );
            assert_eq!(
                fix,
                EscalatedFix::StoodByModelless {
                    reason: StandReason::RateLatched
                }
            );
            listener.assert_no_connections();
        }

        /// The points-drain order pins (plan 326 T6): the gate sits AFTER
        /// `min_pool` and IMMEDIATELY before the transport — a
        /// non-seat-call outcome consumes NOTHING, an exhausted gate
        /// declines locally, an inert gate changes nothing on the wire.
        /// Compiled only when the drain rides the seat transport (the
        /// combined posture).
        #[cfg(feature = "points_drain")]
        mod drain_tests {
            use super::*;
            use crate::points_drain::{DayState, DrainGate};
            use riir_ledger::DrainState;

            const SEED: [u8; 32] = [11u8; 32];

            fn served(allowance: u64, ceiling: u64, rate: u64) -> DrainState {
                DrainState {
                    allowance_remaining_today: allowance,
                    billable_ceiling_today: ceiling,
                    rate_served: rate,
                }
            }

            fn gate_with(state: DayState) -> DrainGate {
                DrainGate::for_tests(state, Some(ed25519_dalek::SigningKey::from_bytes(&SEED)))
            }

            fn armed_drain(json: &str, gate: DrainGate) -> EscalationDispatch {
                EscalationDispatch::arm_with(&manifest_from(json), "clippy", None)
                    .expect("arm ok")
                    .expect("armed")
                    .with_drain_gate(gate)
            }

            fn counters(state: &DayState) -> (u64, u64, u64) {
                (
                    state.hosted_calls_today,
                    state.billable_calls_today,
                    state.call_seq,
                )
            }

            #[test]
            fn worthy_never_consumes_the_day_state() {
                let listener = SilentListener::bind();
                let gate = gate_with(DayState {
                    drain_state: Some(served(10, 10, 2)),
                    ..DayState::default()
                });
                let probe = gate.clone(); // clones share the injected state
                let before = probe.test_snapshot().expect("injected state");
                let mut d = armed_drain(&seat_row_json(0.5, ""), gate);
                let fix = d.escalate_rerank_checked_with(
                    0.9, // >= cap 0.5: the worthiness gate says STANDS
                    SNIPPET,
                    &pool2(),
                    Instant::now() + Duration::from_secs(1),
                    ReadinessPolicy::default(),
                    &EgressConsent::Allowed,
                    &crate::repo_consent::RepoConsentDecision::disabled(),
                );
                assert_eq!(
                    fix,
                    EscalatedFix::StoodByModelless {
                        reason: StandReason::Worthy
                    }
                );
                assert_eq!(
                    counters(&probe.test_snapshot().expect("injected state")),
                    counters(&before),
                    "a Worthy outcome consumes NOTHING (the gate sits after it)"
                );
                listener.assert_no_connections();
            }

            #[test]
            fn below_min_pool_never_consumes_the_day_state() {
                let listener = SilentListener::bind();
                let gate = gate_with(DayState {
                    drain_state: Some(served(10, 10, 2)),
                    ..DayState::default()
                });
                let probe = gate.clone();
                let before = probe.test_snapshot().expect("injected state");
                let mut d = armed_drain(&seat_row_json(0.5, ""), gate);
                let fix = d.escalate_rerank_checked_with(
                    0.1,
                    SNIPPET,
                    &["the-only-candidate".to_string()], // 1 < the frozen default 2
                    Instant::now() + Duration::from_secs(1),
                    ReadinessPolicy::default(),
                    &EgressConsent::Allowed,
                    &crate::repo_consent::RepoConsentDecision::disabled(),
                );
                assert_eq!(
                    fix,
                    EscalatedFix::StoodByModelless {
                        reason: StandReason::BelowMinPool
                    }
                );
                assert_eq!(
                    counters(&probe.test_snapshot().expect("injected state")),
                    counters(&before),
                    "BelowMinPool returns BEFORE the drain gate — nothing consumed"
                );
                listener.assert_no_connections();
            }

            #[test]
            fn rate_latched_never_consumes_the_day_state() {
                let listener = SilentListener::bind();
                let gate = gate_with(DayState {
                    drain_state: Some(served(10, 10, 2)),
                    ..DayState::default()
                });
                let probe = gate.clone();
                let before = probe.test_snapshot().expect("injected state");
                let mut d = armed_drain(&seat_row_json(0.5, ""), gate);
                let deadline = Instant::now() + Duration::from_secs(1);
                // Fill the latch window with below-min-pool misses —
                // those return BEFORE the drain gate (the order under
                // test) and consume nothing.
                let single = vec!["only".to_string()];
                for _ in 0..katgpt_core::escalation_guard::DEFAULT_WINDOW {
                    let _ = d.escalate_rerank_checked_with(
                        0.1,
                        SNIPPET,
                        &single,
                        deadline,
                        ReadinessPolicy::default(),
                        &EgressConsent::Allowed,
                        &crate::repo_consent::RepoConsentDecision::disabled(),
                    );
                }
                // The latched verdict, on a WORTHY score with a full pool:
                // the gate is never reached (the latch fires first).
                let fix = d.escalate_rerank_checked_with(
                    0.9,
                    SNIPPET,
                    &pool2(),
                    deadline,
                    ReadinessPolicy::default(),
                    &EgressConsent::Allowed,
                    &crate::repo_consent::RepoConsentDecision::disabled(),
                );
                assert_eq!(
                    fix,
                    EscalatedFix::StoodByModelless {
                        reason: StandReason::RateLatched
                    }
                );
                assert_eq!(
                    counters(&probe.test_snapshot().expect("injected state")),
                    counters(&before),
                    "the latched verdict consumed NOTHING"
                );
                listener.assert_no_connections();
            }

            #[test]
            fn exhausted_declines_before_any_socket() {
                let listener = SilentListener::bind();
                let gate = gate_with(DayState {
                    cached_balance: Some(1), // below the fee
                    drain_state: Some(served(0, 10, 2)),
                    ..DayState::default()
                });
                let probe = gate.clone();
                let mut d = armed_drain(&seat_row_json(0.5, ""), gate);
                let fix = d.escalate_rerank_checked_with(
                    0.1,
                    SNIPPET,
                    &pool2(),
                    Instant::now() + Duration::from_secs(1),
                    ReadinessPolicy::default(),
                    &EgressConsent::Allowed,
                    &crate::repo_consent::RepoConsentDecision::disabled(),
                );
                assert_eq!(
                    fix,
                    EscalatedFix::StoodByModelless {
                        reason: StandReason::DrainExhausted
                    }
                );
                listener.assert_no_connections();
                let snap = probe.test_snapshot().expect("injected state");
                assert_eq!(
                    counters(&snap),
                    (0, 0, 0),
                    "the decline consumed NOTHING (no count, no seq)"
                );
            }

            #[test]
            fn absent_constants_are_inert_and_send_no_headers() {
                let (addr, rx) = fake_seat_multi(&[("200 OK", &ok_pick_doc(0))]);
                let json = seat_row_json(0.5, "").replace(BASE, &addr);
                let gate = gate_with(DayState {
                    cached_balance: Some(99),
                    drain_state: None, // the inert posture: no served constants
                    ..DayState::default()
                });
                let probe = gate.clone();
                let mut d = armed_drain(&json, gate);
                let fix = d.escalate_rerank_checked_with(
                    0.1,
                    SNIPPET,
                    &pool2(),
                    Instant::now() + Duration::from_secs(1),
                    ReadinessPolicy::default(),
                    &EgressConsent::Allowed,
                    &crate::repo_consent::RepoConsentDecision::disabled(),
                );
                assert!(
                    matches!(
                        fix,
                        EscalatedFix::Answer {
                            source: "seat_pick",
                            ..
                        }
                    ),
                    "inert: the seat call proceeds as before"
                );
                let snap = probe.test_snapshot().expect("injected state");
                assert_eq!(counters(&snap), (0, 0, 0), "inert consumes nothing");
                let reqs = drain(&rx);
                assert_eq!(reqs.len(), 1);
                let head = String::from_utf8_lossy(&reqs[0]).to_ascii_lowercase();
                assert!(
                    !head.contains("x-points-"),
                    "an inert gate composes no attestation headers"
                );
            }

            #[test]
            fn attested_calls_count_and_land_the_three_headers_once() {
                let (addr, rx) = fake_seat_multi(&[("200 OK", &ok_pick_doc(0))]);
                let json = seat_row_json(0.5, "").replace(BASE, &addr);
                let gate = gate_with(DayState {
                    cached_balance: Some(99),
                    drain_state: Some(served(10, 10, 2)),
                    ..DayState::default()
                });
                let probe = gate.clone();
                let mut d = armed_drain(&json, gate);
                let fix = d.escalate_rerank_checked_with(
                    0.1,
                    SNIPPET,
                    &pool2(),
                    Instant::now() + Duration::from_secs(1),
                    ReadinessPolicy::default(),
                    &EgressConsent::Allowed,
                    &crate::repo_consent::RepoConsentDecision::disabled(),
                );
                assert!(matches!(fix, EscalatedFix::Answer { .. }));
                let snap = probe.test_snapshot().expect("injected state");
                assert_eq!(
                    counters(&snap),
                    (1, 0, 1),
                    "the allowed call consumed one free count + one seq"
                );
                let reqs = drain(&rx);
                assert_eq!(reqs.len(), 1);
                let raw = String::from_utf8_lossy(&reqs[0]);
                let (head, body) = raw.split_once("\r\n\r\n").expect("request shape");
                let head = head.to_ascii_lowercase();
                for name in ["x-points-account:", "x-points-pubkey:", "x-points-sig:"] {
                    assert_eq!(
                        head.matches(name).count(),
                        1,
                        "{name} appears EXACTLY once (the kat_ua law)"
                    );
                }
                // the sent body IS the one the one builder emits — the
                // attestation signed exactly these bytes (the crypto pin
                // lives in the points_drain module tests)
                let expected = crate::seat_rerank::SeatRerankClient::new(&addr, HEAL_SUITE)
                    .expect("endpoint ok")
                    .build_body(SNIPPET, &pool2())
                    .expect("pool ok");
                assert_eq!(body, expected, "the transport sent exactly the built body");
            }
        }
    }

    /// The hosted-lane test — needs the hosted transport compiled.
    #[cfg(feature = "hosted_expert")]
    #[test]
    fn hosted_row_gates_worthiness_and_declines_refused_consent() {
        use crate::code_egress::EgressRefusal;
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        const SNIPPET: &str = "fn foo() -> i32 { return 42; }";
        let silent = {
            let l = TcpListener::bind("127.0.0.1:0").expect("silent bind");
            l.set_nonblocking(true).expect("nonblocking");
            l
        };
        let json = "{\"domains\":{\"clippy\":{\"modelless_cap\":0.5,\"max_rate\":0.5,\
                    \"target\":{\"kind\":\"hosted_expert\",\"endpoint\":\"http://127.0.0.1:1\",\
                    \"model\":\"test-model\"}}}}";
        let mut d = EscalationDispatch::arm_with(&manifest_from(json), "clippy", None)
            .expect("arm ok")
            .expect("armed");
        // The rerank entry on a generate dispatch is the same caller-bug
        // decline class.
        let deadline = Instant::now() + Duration::from_secs(1);
        let fix = d.escalate_rerank_checked_with(
            0.0,
            SNIPPET,
            &["a".to_string(), "b".to_string()],
            deadline,
            ReadinessPolicy::default(),
            &EgressConsent::Allowed,
            &crate::repo_consent::RepoConsentDecision::disabled(),
        );
        match fix {
            EscalatedFix::StoodByModelless {
                reason: StandReason::Declined(msg),
            } => assert!(msg.contains("not a rerank lane"), "{msg}"),
            other => panic!("expected the kind decline, got {other:?}"),
        }
        // Unworthy + consent refused: a decline, and NO socket.
        let fix = d.escalate_generate_checked_with(
            0.0,
            SNIPPET,
            deadline,
            128,
            &EgressConsent::Refused(EgressRefusal::Unset),
            &crate::repo_consent::RepoConsentDecision::disabled(),
        );
        match fix {
            EscalatedFix::StoodByModelless {
                reason: StandReason::Declined(msg),
            } => assert!(msg.contains("code-egress refused"), "{msg}"),
            other => panic!("expected the consent decline, got {other:?}"),
        }
        // Worthy: stands pre-socket (the consent never mattered).
        let fix = d.escalate_generate_checked_with(
            0.6,
            SNIPPET,
            deadline,
            128,
            &EgressConsent::Refused(EgressRefusal::Unset),
            &crate::repo_consent::RepoConsentDecision::disabled(),
        );
        assert_eq!(
            fix,
            EscalatedFix::StoodByModelless {
                reason: StandReason::Worthy
            }
        );
        std::thread::sleep(Duration::from_millis(120));
        assert!(
            silent.accept().is_err(),
            "a socket was opened despite the pre-socket refusal"
        );
    }
}
