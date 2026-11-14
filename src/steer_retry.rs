//! Issue 234 T2 — the steer-retry loop for the escalation chain (steer,
//! don't take over).
//!
//! `SteerDistill` (arXiv:2611.13410, via riir-rethink Research 025) Q2/Q3:
//! guidance that lets the student REVISE ITS OWN RESPONSE beats teacher
//! takeover, and structured guidance (discrepancy / applicable conditions /
//! corrective rule / expected outcome) beats free-form. This module is the
//! loop half of that shape for the fixer's escalation chain: the student
//! generates a fix, the verifier rejects it, the failure category becomes
//! structured guidance ([`SteerGuidance`], T1's builder), and the student
//! revises conditioned on the guidance — at most [`STEER_RETRY_ROUNDS`]
//! steered rounds (a budget below the paper's R=4 to bound spend per
//! failure).
//!
//! Scope laws (the issue's own fence):
//!
//! - **The modelless drafter is never touched.** The loop runs only where a
//!   caller hands it a [`SteerStudent`]; the production student is the
//!   opt-in ternary daemon arm ([`DaemonL4`], behind `l4_lora`). This
//!   module is measurement-only and is never in the release set.
//! - **Decline is always safe.** A student decline (`None`) ends the loop
//!   and the caller keeps whatever it already had — the same contract as
//!   the L4 seam. Nothing here panics on malformed output.
//! - **No new hosted spend lane.** The loop itself opens no socket and
//!   touches no ledger; spend is the student transport's own business (the
//!   daemon is local pipes).
//! - **Distribution honesty.** A steered prompt is a DIFFERENT
//!   distribution than the trained one (the daemon's own doc: a reworded
//!   prompt is off-distribution) — that is inherent to steering and is
//!   exactly what T3's measurement prices. The base prompt composes
//!   unchanged; only the steer appendix is added.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::hosted_expert::SteerGuidance;

/// The default steered-round budget: a revision attempt is conditioned on
/// guidance at most this many times after the plain attempt fails (so at
/// most 1 + [`STEER_RETRY_ROUNDS`] generations per case). One below the
/// paper's R=4 by the issue's own budget choice.
pub const STEER_RETRY_ROUNDS: usize = 3;

/// A promptable student arm: anything that turns a prompt into a
/// candidate fix. `None` is a decline and is always safe — the loop stops,
/// the caller keeps whatever it already had.
pub trait SteerStudent {
    /// Human-readable arm name for traces (e.g. `"daemon-l4(bonsai)"`).
    fn name(&self) -> &str;
    /// One generation. Must not block past the caller's deadline concerns
    /// beyond the transport's own contract, must not panic on malformed
    /// output, and must return `None` rather than an error when the arm
    /// cannot serve.
    fn generate(&mut self, prompt: &str) -> Option<String>;
}

/// Why a candidate fix failed verification. The category is the steering
/// signal: it becomes the guidance's `discrepancy` + `corrective_rule`
/// ([`guidance_from_failure`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteerFailureCategory {
    /// The text did not parse as valid Rust (`syn` rejected it, wrapped
    /// and unwrapped).
    Parse,
    /// The response was empty or byte-identical to the trigger — it
    /// changed nothing.
    NoEdit,
    /// The verifier rejected the candidate (the compile-gate class; the
    /// parse verifier never produces this).
    Rejected,
}

/// One verification failure: the category + the verifier's own detail
/// (the parse error, or the compile-gate's refusal line). The detail is
/// verifier output, never file bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerFailure {
    pub category: SteerFailureCategory,
    pub detail: String,
}

/// The verification result for one attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteerVerdict {
    Verified,
    Failed(SteerFailure),
}

/// The verifier seam. T2 ships the in-process parse verifier
/// ([`ParseVerifier`]); T3's measurement arm plugs the compile-gate
/// verifier (`fix_verify`) into the same trait.
pub trait SteerVerifier {
    fn verify(&self, rule_id: &str, buggy: &str, fix: &str) -> SteerVerdict;
}

/// One generation + its verification outcome. `fix: None` = the student
/// declined (the loop stopped there). `failure: None` with a fix = the
/// attempt verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerAttempt {
    /// 0 = the plain attempt; 1.. = steered rounds.
    pub round: usize,
    /// Whether the prompt carried guidance (rounds ≥ 1).
    pub steered: bool,
    /// The student's answer; `None` = declined.
    pub fix: Option<String>,
    /// Why verification failed; `None` = verified (or declined).
    pub failure: Option<SteerFailure>,
}

/// A fix that passed verification, with the round that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedFix {
    pub round: usize,
    pub fix: String,
}

/// The loop's outcome: the full attempt trace + the verified fix if any
/// round verified. `deadline_expired` is set when the loop stopped
/// between attempts on an expired deadline (no student call was made for
/// that round); `student_declined` when the student returned `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerOutcome {
    pub attempts: Vec<SteerAttempt>,
    pub verified: Option<VerifiedFix>,
    pub deadline_expired: bool,
    pub student_declined: bool,
}

impl SteerOutcome {
    /// The verified fix, if any (the caller's drop-in upgrade over the
    /// plain attempt).
    #[must_use]
    pub fn verified_fix(&self) -> Option<&str> {
        self.verified.as_ref().map(|v| v.fix.as_str())
    }
}

/// The structured guidance for one failure, derived deterministically
/// from the failure category (the paper's steer: the MEASURED failure
/// feeds back, not a paraphrase). Every field rides
/// [`SteerGuidance::new`]'s own bounding; the detail is verifier output,
/// never file bytes (the snippet-only law holds by construction).
#[must_use]
pub fn guidance_from_failure(rule_id: &str, failure: &SteerFailure) -> SteerGuidance {
    let discrepancy = match failure.category {
        SteerFailureCategory::Parse => format!(
            "the previous response did not parse as valid Rust: {}",
            failure.detail
        ),
        SteerFailureCategory::NoEdit => {
            "the previous response was empty or identical to the trigger — it changed nothing"
                .to_string()
        }
        SteerFailureCategory::Rejected => format!(
            "the previous response failed verification: {}",
            failure.detail
        ),
    };
    let corrective_rule = match failure.category {
        SteerFailureCategory::Parse => {
            "Output complete, valid Rust. Changed lines must still form parseable Rust \
             statements or items — no prose, no markdown fences."
        }
        SteerFailureCategory::NoEdit => {
            "The revision must actually change the code: output the corrected lines that \
             differ from the input."
        }
        SteerFailureCategory::Rejected => {
            "Address the reported verification failure while keeping every other part of \
             the previous response that was already correct."
        }
    };
    SteerGuidance::new(
        &discrepancy,
        &format!(
            "Rule {rule_id}: the revision must be valid Rust, must differ from the \
             trigger snippet, and must satisfy the rule's constraint."
        ),
        corrective_rule,
        "the revised lines verify cleanly under the rule's constraint",
    )
}

/// The steered prompt for a raw-prompt student: the base prompt unchanged
/// (the trained instruction contract), the student's OWN prior response
/// echoed for revision (the paper's revise-your-own shape — model output,
/// never file bytes), then the guidance render.
#[must_use]
pub fn steer_prompt(base_prompt: &str, prior_fix: &str, guidance: &SteerGuidance) -> String {
    format!(
        "{base_prompt}\n\nYOUR PREVIOUS RESPONSE:\n{prior_fix}\n\n{}",
        guidance.render()
    )
}

/// The steer-retry loop. Attempt 0 runs the plain prompt; every failure
/// after that derives guidance from the failure and re-prompts with the
/// student's own prior response echoed, at most `max_rounds` steered
/// rounds (the issue's default: [`STEER_RETRY_ROUNDS`]). The deadline is
/// checked BEFORE each generation (an expired budget stops the loop
/// pre-call, the same posture as the hosted call's own guard).
pub fn steer_retry<S: SteerStudent, V: SteerVerifier>(
    student: &mut S,
    verifier: &V,
    rule_id: &str,
    buggy_code: &str,
    base_prompt: &str,
    deadline: Option<Instant>,
    max_rounds: usize,
) -> SteerOutcome {
    let mut outcome = SteerOutcome {
        attempts: Vec::with_capacity(max_rounds.saturating_add(1)),
        verified: None,
        deadline_expired: false,
        student_declined: false,
    };
    let mut prior_failure: Option<SteerFailure> = None;
    let mut round = 0usize;
    loop {
        // Deadline BEFORE the call — a spent budget never reaches the
        // student (the hosted call's own pre-socket posture, lifted to the
        // loop).
        if deadline.is_some_and(|d| d <= Instant::now()) {
            outcome.deadline_expired = true;
            return outcome;
        }
        let prompt = match &prior_failure {
            Some(failure) => {
                let guidance = guidance_from_failure(rule_id, failure);
                let prior_fix = outcome
                    .attempts
                    .last()
                    .and_then(|a| a.fix.clone())
                    .unwrap_or_default();
                steer_prompt(base_prompt, &prior_fix, &guidance)
            }
            // Round 0 (prior_failure is None by construction — a steered
            // round only runs after a failure).
            None => base_prompt.to_string(),
        };
        let Some(fix) = student.generate(&prompt) else {
            outcome.student_declined = true;
            outcome.attempts.push(SteerAttempt {
                round,
                steered: round > 0,
                fix: None,
                failure: None,
            });
            return outcome;
        };
        let trimmed = fix.trim();
        let failure = if trimmed.is_empty() || trimmed == buggy_code.trim() {
            Some(SteerFailure {
                category: SteerFailureCategory::NoEdit,
                detail: "empty or identical to the trigger".to_string(),
            })
        } else {
            match verifier.verify(rule_id, buggy_code, &fix) {
                SteerVerdict::Verified => None,
                SteerVerdict::Failed(f) => Some(f),
            }
        };
        let steered = round > 0;
        match failure {
            None => {
                outcome.attempts.push(SteerAttempt {
                    round,
                    steered,
                    fix: Some(fix.clone()),
                    failure: None,
                });
                outcome.verified = Some(VerifiedFix { round, fix });
                return outcome;
            }
            Some(f) => {
                outcome.attempts.push(SteerAttempt {
                    round,
                    steered,
                    fix: Some(fix),
                    failure: Some(f.clone()),
                });
                prior_failure = Some(f);
            }
        }
        if round >= max_rounds {
            return outcome;
        }
        round += 1;
    }
}

/// The in-process parse verifier: the L2 pruner's `syn` check (the same
/// full-file / wrapped-snippet / single-item ladder the drafter gate
/// uses). Microseconds per call; the compile-gate verifier is T3's plug-in
/// on the same trait.
#[derive(Debug, Clone, Default)]
pub struct ParseVerifier {
    pruner: crate::pruner::RustAnalyzerPruner,
}

impl ParseVerifier {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SteerVerifier for ParseVerifier {
    fn verify(&self, rule_id: &str, buggy: &str, fix: &str) -> SteerVerdict {
        let _ = (rule_id, buggy);
        let result = self.pruner.validate(fix, rule_id);
        if result.parses {
            SteerVerdict::Verified
        } else {
            SteerVerdict::Failed(SteerFailure {
                category: SteerFailureCategory::Parse,
                detail: result
                    .diagnostics
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "parse failed without a diagnostic".to_string()),
            })
        }
    }
}

/// Route one verified (trigger, steered fix) pair into the authoring
/// intake — the internalization half ("internalization = corpus rows, our
/// medium"). The rows are content-addressed and idempotent per pair:
/// `blake3(buggy ‖ fix ‖ rule)` names the file, so a re-routed identical
/// pair writes nothing new and a changed pair starts a fresh row.
///
/// Placement note (recorded on the issue): the corpus-gap lane itself is
/// census-shaped (rows derive from a `CoverageManifest` partition per
/// `(lint, epoch)`) and cannot carry arbitrary exemplar pairs, so the
/// steer rows land under the SAME `.refine/misses/` consumption tree the
/// gap rows ride — `.refine/misses/steer/` — where the authoring queue
/// already reads.
///
/// # Errors
///
/// [`std::io::Error`] on any filesystem failure.
pub fn route_verified_pair(
    root: &Path,
    rule_id: &str,
    buggy: &str,
    fix: &str,
    round: usize,
) -> std::io::Result<PathBuf> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"i234-steer-pair-v1");
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(buggy.as_bytes());
    hasher.update(b"\0");
    hasher.update(fix.as_bytes());
    let digest_hex = hasher.finalize().to_hex().to_string();
    let dir = root.join(".refine").join("misses").join("steer");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{rule_id}-{}.md", &digest_hex[..8]));
    if path.exists() {
        return Ok(path);
    }
    let body = format!(
        "# steer-retry intake — {rule_id}\n\n\
         Verified steered revision (round {round}) from the Issue 234 T2 loop.\n\
         Pair digest: {} (idempotent per pair — re-routing writes nothing new).\n\
         Consumption: the authoring queue reads `.refine/misses/*`; distill\n\
         or fold into the rule's corpus at the next mining pass.\n\n\
         ## trigger\n\n```rust\n{buggy}\n```\n\n\
         ## steered fix\n\n```rust\n{fix}\n```\n",
        &digest_hex[..8],
    );
    // Atomic publish: tmp + rename so a crash never leaves a partial row
    // (the house law for persisted artifacts).
    let tmp = dir.join(format!(
        ".{rule_id}-{}.tmp-{}",
        &digest_hex[..8],
        std::process::id()
    ));
    std::fs::write(&tmp, &body)?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

#[cfg(all(feature = "steer_retry", feature = "l4_lora"))]
impl SteerStudent for crate::l4_daemon::DaemonL4 {
    // Signature is fixed by the trait: the return is tied to &self, the
    // literal is 'static (the hosted_expert impl's own documented allow).
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        // The daemon's own display name already carries the lora identity.
        "steer-daemon-l4"
    }

    fn generate(&mut self, prompt: &str) -> Option<String> {
        // The daemon's protocol + failure contract IS the student seam
        // (decline = None, never panic); request_prompt exists since
        // Issue 040 for exactly this caller-built-prompt shape.
        self.request_prompt(prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A planted student: each prompt maps to a canned answer by round —
    /// the BT8DRY posture (instrument evidence, no model, no network).
    /// Every received prompt is recorded so the steer shape is assertable.
    struct ScriptedStudent {
        answers: Vec<Option<String>>,
        calls: usize,
        prompts: Vec<String>,
    }

    impl ScriptedStudent {
        fn new(answers: Vec<Option<String>>) -> Self {
            Self {
                answers,
                calls: 0,
                prompts: Vec::new(),
            }
        }
    }

    impl SteerStudent for ScriptedStudent {
        // Signature is fixed by the trait (the hosted_expert precedent).
        #[allow(clippy::unnecessary_literal_bound)]
        fn name(&self) -> &str {
            "scripted"
        }
        fn generate(&mut self, prompt: &str) -> Option<String> {
            self.prompts.push(prompt.to_string());
            let answer = self.answers.get(self.calls).cloned().flatten();
            self.calls += 1;
            answer
        }
    }

    fn verified_on_rule() -> impl SteerVerifier {
        ParseVerifier::new()
    }

    const RULE: &str = "needless_return";
    const BUGGY: &str = "pub fn f() -> u32 { return 42; }";

    #[test]
    fn attempt_zero_success_spends_one_call_and_builds_no_guidance() {
        let good = "pub fn f() -> u32 { 42 }";
        let mut student = ScriptedStudent::new(vec![Some(good.to_string())]);
        let outcome = steer_retry(
            &mut student,
            &verified_on_rule(),
            RULE,
            BUGGY,
            "fix the lint:",
            None,
            STEER_RETRY_ROUNDS,
        );
        assert_eq!(student.calls, 1, "a verified attempt 0 never steers");
        assert_eq!(outcome.verified.as_ref().map(|v| v.round), Some(0));
        assert_eq!(outcome.verified_fix(), Some(good));
        assert!(!outcome.deadline_expired);
        assert!(!outcome.student_declined);
        assert_eq!(outcome.attempts.len(), 1);
        assert!(!outcome.attempts[0].steered);
        assert!(outcome.attempts[0].failure.is_none());
    }

    #[test]
    fn steered_round_carries_the_prior_response_and_the_guidance_labels() {
        // Attempt 0 does not parse; attempt 1 fixes it. The attempt-1
        // prompt must carry the student's OWN prior response + all four
        // guidance labels + the rule id (the steer shape).
        let mut student = ScriptedStudent::new(vec![
            Some("return 42 }{ not rust".to_string()),
            Some("pub fn f() -> u32 { 42 }".to_string()),
        ]);
        let outcome = steer_retry(
            &mut student,
            &verified_on_rule(),
            RULE,
            BUGGY,
            "fix the lint:",
            None,
            STEER_RETRY_ROUNDS,
        );
        assert_eq!(student.calls, 2, "fail -> steer -> verify");
        assert_eq!(outcome.verified.as_ref().map(|v| v.round), Some(1));
        assert!(outcome.attempts[0].failure.is_some());
        assert_eq!(
            outcome.attempts[0].failure.as_ref().map(|f| f.category),
            Some(SteerFailureCategory::Parse)
        );
        assert!(outcome.attempts[1].steered);
        // The steer shape, pinned from the RECORDED prompt: base prompt
        // unchanged, the prior response echoed, the four labels + the
        // rule id + the parse corrective rule present.
        let steer = &student.prompts[1];
        assert!(steer.starts_with("fix the lint:"), "base prompt unchanged");
        assert!(steer.contains("YOUR PREVIOUS RESPONSE:\nreturn 42 }{ not rust"));
        assert!(steer.contains("Discrepancy:"));
        assert!(steer.contains("Rule needless_return"));
        assert!(steer.contains("did not parse as valid Rust"));
        assert!(!steer.contains("not rust at all"), "no unrelated content");
        assert_eq!(student.prompts[0], "fix the lint:", "attempt 0 is plain");
    }

    #[test]
    fn the_steered_prompt_shape_is_derivable_and_labeled() {
        // Pin the prompt composition directly (the spy-free half): the
        // guidance render + the prior-response echo.
        let failure = SteerFailure {
            category: SteerFailureCategory::Parse,
            detail: "parse error: expected `;`".to_string(),
        };
        let guidance = guidance_from_failure(RULE, &failure);
        let prompt = steer_prompt("fix the lint:", "return 42", &guidance);
        assert!(prompt.starts_with("fix the lint:"));
        assert!(prompt.contains("YOUR PREVIOUS RESPONSE:\nreturn 42"));
        assert!(prompt.contains("Discrepancy:"));
        assert!(prompt.contains("Applicable conditions:"));
        assert!(prompt.contains("Corrective rule:"));
        assert!(prompt.contains("Expected outcome:"));
        assert!(prompt.contains("Rule needless_return"));
        assert!(prompt.contains("did not parse as valid Rust"));
        assert!(prompt.contains("Revise your previous response"));
    }

    #[test]
    fn student_decline_stops_the_loop_and_reports() {
        let mut student = ScriptedStudent::new(vec![None]);
        let outcome = steer_retry(
            &mut student,
            &verified_on_rule(),
            RULE,
            BUGGY,
            "fix the lint:",
            None,
            STEER_RETRY_ROUNDS,
        );
        assert_eq!(student.calls, 1);
        assert!(outcome.student_declined);
        assert!(outcome.verified.is_none());
        assert!(outcome.attempts[0].fix.is_none());
    }

    #[test]
    fn mid_loop_decline_keeps_the_trace() {
        let mut student = ScriptedStudent::new(vec![
            Some("not rust {{{".to_string()), // parse-fails
            None,                             // declines under guidance
        ]);
        let outcome = steer_retry(
            &mut student,
            &verified_on_rule(),
            RULE,
            BUGGY,
            "fix the lint:",
            None,
            STEER_RETRY_ROUNDS,
        );
        assert_eq!(student.calls, 2);
        assert!(outcome.student_declined);
        assert_eq!(outcome.attempts.len(), 2);
        assert!(outcome.attempts[1].steered);
        assert!(outcome.verified.is_none());
    }

    #[test]
    fn budget_is_bounded_at_max_rounds_plus_the_plain_attempt() {
        // Every answer parse-fails: 1 plain + STEER_RETRY_ROUNDS steered.
        let answers = (0..=STEER_RETRY_ROUNDS + 2)
            .map(|_| Some("not rust at all".to_string()))
            .collect::<Vec<_>>();
        let mut student = ScriptedStudent::new(answers);
        let outcome = steer_retry(
            &mut student,
            &verified_on_rule(),
            RULE,
            BUGGY,
            "fix the lint:",
            None,
            STEER_RETRY_ROUNDS,
        );
        assert_eq!(
            student.calls,
            STEER_RETRY_ROUNDS + 1,
            "1 plain + R steered, never more (the budget law)"
        );
        assert!(outcome.verified.is_none());
        assert_eq!(outcome.attempts.len(), STEER_RETRY_ROUNDS + 1);
    }

    #[test]
    fn expired_deadline_stops_before_any_call() {
        let mut student = ScriptedStudent::new(vec![Some("pub fn f() -> u32 { 42 }".to_string())]);
        let deadline = Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .expect("clock before 1970");
        let outcome = steer_retry(
            &mut student,
            &verified_on_rule(),
            RULE,
            BUGGY,
            "fix the lint:",
            Some(deadline),
            STEER_RETRY_ROUNDS,
        );
        assert_eq!(
            student.calls, 0,
            "an expired budget never reaches the student"
        );
        assert!(outcome.deadline_expired);
        assert_eq!(outcome.attempts, [] as [SteerAttempt; 0]);
    }

    #[test]
    fn noedit_is_detected_without_the_verifier_and_steers() {
        // Attempt 0 echoes the trigger; attempt 1 fixes it.
        let mut student = ScriptedStudent::new(vec![
            Some(BUGGY.to_string()), // identical to the trigger
            Some("pub fn f() -> u32 { 42 }".to_string()),
        ]);
        let outcome = steer_retry(
            &mut student,
            &verified_on_rule(),
            RULE,
            BUGGY,
            "fix the lint:",
            None,
            STEER_RETRY_ROUNDS,
        );
        assert_eq!(student.calls, 2);
        assert_eq!(
            outcome.attempts[0].failure.as_ref().map(|f| f.category),
            Some(SteerFailureCategory::NoEdit)
        );
        assert_eq!(outcome.verified.as_ref().map(|v| v.round), Some(1));
    }

    #[test]
    fn guidance_fields_pin_per_category_and_stay_bounded() {
        let huge = "x".repeat(SteerGuidance::FIELD_MAX_BYTES * 4);
        let parse = guidance_from_failure(
            RULE,
            &SteerFailure {
                category: SteerFailureCategory::Parse,
                detail: huge.clone(),
            },
        );
        assert!(parse.discrepancy.contains("did not parse as valid Rust"));
        assert!(
            parse.discrepancy.len() <= SteerGuidance::FIELD_MAX_BYTES + 64,
            "the bounded builder caps the runaway detail"
        );
        assert!(parse.corrective_rule.contains("parseable Rust"));

        let noedit = guidance_from_failure(
            RULE,
            &SteerFailure {
                category: SteerFailureCategory::NoEdit,
                detail: String::new(),
            },
        );
        assert!(noedit.discrepancy.contains("changed nothing"));

        let rejected = guidance_from_failure(
            RULE,
            &SteerFailure {
                category: SteerFailureCategory::Rejected,
                detail: "E0499 borrowed twice".to_string(),
            },
        );
        assert!(rejected.discrepancy.contains("E0499"));
        assert!(rejected.corrective_rule.contains("verification failure"));
    }

    #[test]
    fn parse_verifier_accepts_items_and_rejects_prose() {
        let verifier = ParseVerifier::new();
        assert_eq!(
            verifier.verify(RULE, BUGGY, "pub fn f() -> u32 { 42 }"),
            SteerVerdict::Verified
        );
        // Expression snippets ride the wrapped ladder.
        assert_eq!(
            verifier.verify(RULE, BUGGY, "let y = 1 + 1;"),
            SteerVerdict::Verified
        );
        let failed = verifier.verify(RULE, BUGGY, "totally not rust {{{");
        let SteerVerdict::Failed(f) = failed else {
            panic!("prose must fail the parse verifier");
        };
        assert_eq!(f.category, SteerFailureCategory::Parse);
        assert!(f.detail.contains("parse error"));
    }

    #[test]
    fn intake_rows_are_content_addressed_and_idempotent() {
        let root = std::env::temp_dir().join(format!(
            "i234_steer_intake_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos())
        ));
        let first = route_verified_pair(&root, RULE, BUGGY, "pub fn f() -> u32 { 42 }", 1)
            .expect("route first");
        let again = route_verified_pair(&root, RULE, BUGGY, "pub fn f() -> u32 { 42 }", 2)
            .expect("route again");
        assert_eq!(first, again, "identical pairs address the same row");
        let body = std::fs::read_to_string(&first).expect("read row");
        assert!(body.contains("needless_return"));
        assert!(body.contains("pub fn f() -> u32 { 42 }"));
        assert!(body.contains("## trigger"));
        assert!(body.contains("## steered fix"));
        let other = route_verified_pair(&root, RULE, BUGGY, "pub fn f() -> u32 { 7 }", 1)
            .expect("route other");
        assert_ne!(first, other, "a changed pair starts a fresh row");
        let dir_entries = std::fs::read_dir(root.join(".refine").join("misses").join("steer"))
            .expect("read dir")
            .filter_map(std::result::Result::ok)
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .count();
        assert_eq!(dir_entries, 2, "two pairs, two rows, no tmp residue");
        let _ = std::fs::remove_dir_all(&root);
    }
}
