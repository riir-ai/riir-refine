//! Plan 202 R2: `HostedExpertL4` โ€” one MORE transport under the existing
//! [`L4Fallback`](crate::l4_fallback::L4Fallback) seam (beside
//! `DaemonL4`'s long-lived process and `CfClient`'s CF-Workers-AI leg),
//! for the no-candidate miss: the hosted expert GENERATES the fix.
//!
//! **Existing machinery, not new** (the plan's four points):
//! 1. **Cost guard** = [`crate::llm_spend`] โ€” reserve-then-reconcile on
//!    every call; `CapReached` hard-declines for the rest of the UTC day
//!    and is NEVER retried here (the Issue-040 169-request burn rule).
//! 2. **Code-egress consent is its own scope** (Plan 202 ยง2 law 5):
//!    [`EGRESS_ENV`] unset refuses BEFORE any socket โ€” and refuses
//!    independently of every OTHER consent scope (stats, mining โ€” the
//!    generic-`Unset` trap). The accepted spelling is deliberate, never
//!    a bare truthy value.
//! 3. **Hard deadline**: `generate_fix` is synchronous `&mut self` โ€” an
//!    already-expired deadline declines BEFORE the socket; an in-flight
//!    overrun is bounded by the leaf transport's `CALL_TIMEOUT` ceiling.
//!    Deadline expiry is a DECLINE (the conservative contract), never a
//!    partial answer.
//! 4. **Registered in the R1 manifest, never in code**: construction
//!    takes the endpoint from the manifest row
//!    (`target { kind: "hosted_expert", endpoint }`) โ€” there is no
//!    default endpoint, no env fallback, no compile-time host.
//!
//! ## The wire
//!
//! OpenAI-compatible chat-completions via the [`riir_llm`] leaf
//! (`transport::post_chat_on` โ€” URL shape, both timeout ceilings,
//! failure classification, bearer scrub; the same wire `groq_first`
//! rides). The BODY is built here, not by the leaf's `chat_body_for`,
//! because two R2 obligations are wire-shaped:
//!
//! - **Snippet-only payload**: the builder takes ONLY the snippet โ€”
//!   the API makes "accidentally send the file" unrepresentable.
//! - **The no-retention rule rides the wire**: a system message states
//!   it and `metadata.retention = "none"` carries it structurally, so
//!   the receiving tier's refusal is auditable in the request itself
//!   (else A10 runs in reverse: user code becomes moat training data
//!   without opt-in).
//!
//! ## Authentication
//!
//! The endpoint's bearer is the operator's [`TOKEN_ENV`] โ€” read at
//! construction, held in ONE heap location (the `CfClient` rule),
//! scrubbed from every error by the leaf's classification.

use crate::l4_fallback::L4Fallback;
use std::time::Instant;

// The code-egress consent scope moved to [`crate::code_egress`] (Plan 202
// R3a: the seat-rerank client joins as the second consumer — one scope,
// one definition, the `llm_spend` ANY-of-consumers gate shape). The
// re-export preserves every historical path (`hosted_expert::EgressConsent`...
// — the R2 tests and any external caller compile unchanged).
pub use crate::code_egress::{EGRESS_ALLOW, EGRESS_ENV, EgressConsent, EgressRefusal};

/// The hosted endpoint's bearer token env (optional — a local/loopback
/// expert may skip auth; a real hosted tier will not).
pub const TOKEN_ENV: &str = "RIIR_HOSTED_EXPERT_TOKEN";

/// Why [`HostedExpertL4::generate_fix`] declined. `Display` names the
/// refusal class for the run log; a decline is ALWAYS safe for the
/// caller (the modelless answer stands).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decline {
    /// [`EGRESS_ENV`] did not consent โ€” no socket was opened.
    Egress(EgressRefusal),
    /// The spend cap was reached โ€” terminal for the rest of the UTC
    /// day; the reserve path counted the decline.
    CapReached(String),
    /// The model is not in the pricing table โ€” the cost guard refuses
    /// to send without a cap price (never price a guess at $0). Its own
    /// class: unlike the cap it is a CONFIG fix, not a wait-for-midnight.
    Unpriced(String),
    /// The deadline had already expired โ€” nothing was sent.
    DeadlineExpired,
    /// The wire failed (transport/HTTP/shape) or the expert returned
    /// nothing usable. Carries the classified message, bearer-scrubbed.
    Wire(String),
}

impl std::fmt::Display for Decline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Egress(r) => write!(f, "code-egress refused: {r}"),
            Self::CapReached(m) => write!(f, "spend cap: {m}"),
            Self::Unpriced(m) => write!(f, "cost guard: {m}"),
            Self::DeadlineExpired => {
                write!(f, "deadline expired before the call โ€” nothing sent")
            }
            Self::Wire(m) => write!(f, "hosted expert wire: {m}"),
        }
    }
}

/// The operator's endpoint authority, resolved once at construction:
/// where the R1 manifest row points and what it authenticates with.
#[derive(Clone)]
pub struct HostedExpertL4 {
    /// The shared leaf agent (connection reuse โ€” ONE per client).
    agent: riir_llm::transport::Agent,
    /// The manifest row's endpoint (the `{base}` the leaf appends
    /// `/chat/completions` to). Never a default.
    endpoint: String,
    /// The model id the served expert answers as (also manifest row
    /// data at the R6 wiring; until then an explicit construction arg โ€”
    /// never hardcoded).
    model: String,
    /// Pre-formatted `Bearer <token>`; empty = no auth header expected
    /// by the endpoint (loopback experts). Held once, never logged.
    bearer: String,
    /// The USD daily-cap ledger (shared file with the CF leg โ€” one
    /// accumulator per UTC day across every LLM consumer).
    spend: crate::llm_spend::SpendLedger,
}

impl std::fmt::Debug for HostedExpertL4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No endpoint (embeds the host), no bearer โ€” the CfClient rule.
        f.debug_struct("HostedExpertL4").finish_non_exhaustive()
    }
}

/// Issue 234 T1 — the STRUCTURED GUIDANCE payload (`SteerDistill` Q2/Q3):
/// guidance that lets the student revise its own response, shaped as
/// discrepancy / applicable conditions / corrective rule / expected
/// outcome — never a teacher takeover (the takeover shape is the
/// existing `build_body`). Opt-in: callers construct
/// [`SteerGuidance`] explicitly; nothing in the default posture
/// references this builder (T3 measures before any default flips).
///
/// The wire obligations ride unchanged: snippet-only + the no-retention
/// rule (system message + metadata), temperature 0. The guidance is
/// bounded per field at construction so a runaway caller cannot
/// balloon the payload past the snippet itself (the R2 budget law's
/// payload half).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerGuidance {
    /// What the compile gate actually rejected in the student's
    /// response (the measured failure, not a paraphrase).
    pub discrepancy: String,
    /// The applicable conditions — the rule ids / code facts the
    /// correction must respect.
    pub conditions: String,
    /// The corrective rule — what to change, stated as guidance (the
    /// student writes the revision itself: steer, don't take over).
    pub corrective_rule: String,
    /// What a correct revision satisfies, checkably.
    pub expected_outcome: String,
}

impl SteerGuidance {
    /// The payload cap per field (bytes), char-boundary safe.
    pub const FIELD_MAX_BYTES: usize = 2048;

    /// Bounds every field and substitutes a marker for empties so the
    /// prompt never shows an unlabeled blank slot.
    #[must_use]
    pub fn new(
        discrepancy: &str,
        conditions: &str,
        corrective_rule: &str,
        expected_outcome: &str,
    ) -> Self {
        let bound = |field: &str| {
            let cut = field.floor_char_boundary(Self::FIELD_MAX_BYTES);
            let trimmed = field[..cut].trim();
            if trimmed.is_empty() {
                "(not recorded)".to_string()
            } else {
                trimmed.to_string()
            }
        };
        Self {
            discrepancy: bound(discrepancy),
            conditions: bound(conditions),
            corrective_rule: bound(corrective_rule),
            expected_outcome: bound(expected_outcome),
        }
    }

    /// The guidance rendered as the user-message appendix (the four
    /// labeled sections; the student REVISES its own prior response —
    /// the `SteerDistill` steer shape, not a takeover).
    #[must_use]
    pub fn render(&self) -> String {
        format!(
            "YOUR PREVIOUS RESPONSE FAILED.\nDiscrepancy: {}\nApplicable conditions: {}\nCorrective rule: {}\nExpected outcome: {}\nRevise your previous response accordingly. Output ONLY the corrected lines that differ, no explanation.",
            self.discrepancy, self.conditions, self.corrective_rule, self.expected_outcome
        )
    }
}

impl HostedExpertL4 {
    /// Build from the R1 manifest row's pieces. `endpoint` is the
    /// OpenAI-compatible base (the leaf appends the route); `model` is
    /// the served expert's id; `spend` is the process-wide ledger.
    ///
    /// Construction does NOT read the consent env and opens NO socket โ€”
    /// building a transport is inert; only `generate_fix` egresses.
    #[must_use]
    pub fn new(endpoint: String, model: String, spend: crate::llm_spend::SpendLedger) -> Self {
        let bearer = match std::env::var(TOKEN_ENV) {
            Ok(t) if !t.trim().is_empty() => format!("Bearer {}", t.trim()),
            _ => String::new(),
        };
        Self {
            agent: riir_llm::transport::agent(),
            endpoint,
            model,
            bearer,
            spend,
        }
    }

    /// The spend ledger (the R6 arm-H cost axis reads it for the run
    /// summary; the guard itself stays inside `generate_fix_checked_with`).
    #[must_use]
    pub fn spend(&self) -> &crate::llm_spend::SpendLedger {
        &self.spend
    }

    /// The request body for one snippet — the SNIPPET-ONLY builder. The
    /// signature takes the snippet and nothing else: there is no path
    /// from a file handle to this function. The no-retention rule rides
    /// the wire twice (system message + metadata) so the receiving
    /// tier's obligation is in the request itself.
    #[must_use]
    pub fn build_body(&self, snippet: &str, max_tokens: u64) -> String {
        let system = "You are a Rust lint-fixing expert. Output ONLY the corrected lines that \
             differ from the input, no explanation. RETENTION RULE (binding): the \
             user's code in this request must not be retained, logged, or used for \
             training — respond to it, then discard it.";
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": snippet },
            ],
            "max_tokens": max_tokens,
            "temperature": 0.0,
            "metadata": { "retention": "none" },
        });
        serde_json::to_string(&body).unwrap_or_default()
    }

    /// The steer-mode request body: the snippet + the structured guidance
    /// appendix (the SNIPPET-ONLY law holds — the guidance carries no file
    /// bytes, only the four labeled fields), the no-retention rule riding
    /// the wire exactly as the takeover builder rides it (system message +
    /// metadata).
    #[must_use]
    pub fn build_steer_body(
        &self,
        snippet: &str,
        guidance: &SteerGuidance,
        max_tokens: u64,
    ) -> String {
        let system = "You are a Rust lint-fixing expert. You previously gave a response that failed verification. Revise YOUR OWN response per the structured guidance. Output ONLY the corrected lines that differ from the input, no explanation. RETENTION RULE (binding): the user's code in this request must not be retained, logged, or used for training — respond to it, then discard it.";
        let user = format!("{snippet}\n\n{}", guidance.render());
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "max_tokens": max_tokens,
            "temperature": 0.0,
            "metadata": { "retention": "none" },
        });
        serde_json::to_string(&body).unwrap_or_default()
    }

    /// The steer-mode call: the same consent / deadline / reserve / wire /
    /// reconcile taxonomy as [`Self::generate_fix_checked_with`], with the
    /// body built by [`Self::build_steer_body`] (opt-in; the default
    /// posture never reaches here until T3's measurement says so).
    ///
    /// # Errors
    ///
    /// The same [`Decline`] taxonomy; the guidance rides inside the billed
    /// prompt (the reserve covers the snippet estimate path unchanged).
    pub fn generate_steer_fix_checked_with(
        &mut self,
        snippet: &str,
        guidance: &SteerGuidance,
        deadline: Option<Instant>,
        max_tokens: u64,
        consent: EgressConsent,
    ) -> Result<String, Decline> {
        if let EgressConsent::Refused(r) = consent {
            return Err(Decline::Egress(r));
        }
        if deadline.is_some_and(|d| d <= Instant::now()) {
            return Err(Decline::DeadlineExpired);
        }
        let reserved = match self.spend.reserve(&self.model, snippet, max_tokens) {
            Ok(r) => r,
            Err(e) => {
                let _ = self.spend.save();
                return Err(match e {
                    crate::llm_spend::SpendError::CapReached(c) => {
                        Decline::CapReached(c.to_string())
                    }
                    crate::llm_spend::SpendError::UnknownModel(m) => Decline::Unpriced(format!(
                        "model {m:?} is not in the pricing table — refusing to send without a cap price (add it to PRICING_TABLE + bump PRICING_VERSION)"
                    )),
                });
            }
        };
        let _ = self.spend.save();
        let leg = riir_llm::Leg {
            label: "hosted-expert-steer".to_string(),
            base: self.endpoint.clone(),
            bearer: self.bearer.clone(),
            model: self.model.clone(),
            wire: riir_llm::Wire::OpenAi,
        };
        let body = self.build_steer_body(snippet, guidance, max_tokens);
        let sent = match riir_llm::transport::post_chat_on(&self.agent, &leg, &body) {
            Ok(raw) => raw,
            Err(f) => {
                self.spend.release(reserved);
                let _ = self.spend.save();
                return Err(Decline::Wire(f.message().to_string()));
            }
        };
        let (text, usage) = match riir_llm::parse(riir_llm::Wire::OpenAi, &sent) {
            Ok(pair) => pair,
            Err(f) => {
                self.spend.reconcile(reserved, &self.model, 0, 0, false);
                let _ = self.spend.save();
                return Err(Decline::Wire(f.message().to_string()));
            }
        };
        self.spend.reconcile(
            reserved,
            &self.model,
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.reported,
        );
        let _ = self.spend.save();
        let fix = text.trim();
        if fix.is_empty() {
            return Err(Decline::Wire("expert returned an empty fix".to_string()));
        }
        Ok(fix.to_string())
    }

    /// The full call: consent โ’ deadline โ’ reserve โ’ wire โ’ reconcile.
    /// `Ok(fix)` = the hosted expert's answer; `Err(Decline)` = decline
    /// (always safe โ€” the caller keeps the modelless result).
    ///
    /// # Errors
    ///
    /// Every [`Decline`] class; never panics on malformed remote output
    /// (the trait contract: a fallback that panics takes down a heal
    /// that would otherwise have returned a modelless answer).
    pub fn generate_fix_checked(
        &mut self,
        snippet: &str,
        deadline: Option<Instant>,
        max_tokens: u64,
    ) -> Result<String, Decline> {
        self.generate_fix_checked_with(snippet, deadline, max_tokens, EgressConsent::from_env())
    }

    /// The consent-injectable core of [`Self::generate_fix_checked`] โ€”
    /// the form tests drive directly, so no test ever mutates the
    /// process env (the `is_armed_with` / `operator_pins_from` seam
    /// pattern; parallel tests never race a global here).
    ///
    /// # Errors
    ///
    /// Same taxonomy as the env-reading form.
    pub fn generate_fix_checked_with(
        &mut self,
        snippet: &str,
        deadline: Option<Instant>,
        max_tokens: u64,
        consent: EgressConsent,
    ) -> Result<String, Decline> {
        // (2) consent โ€” BEFORE anything else; refusal opens no socket.
        if let EgressConsent::Refused(r) = consent {
            return Err(Decline::Egress(r));
        }
        // (3) deadline โ€” an expired budget declines pre-socket (the
        // in-flight ceiling is the leaf transport's CALL_TIMEOUT).
        if deadline.is_some_and(|d| d <= Instant::now()) {
            return Err(Decline::DeadlineExpired);
        }
        // (1) the cost guard โ€” reserve BEFORE the wire; CapReached is
        // terminal for the day and is returned as a decline, never retried.
        let reserved = match self.spend.reserve(&self.model, snippet, max_tokens) {
            Ok(r) => r,
            Err(e) => {
                let _ = self.spend.save();
                return Err(match e {
                    crate::llm_spend::SpendError::CapReached(c) => {
                        Decline::CapReached(c.to_string())
                    }
                    crate::llm_spend::SpendError::UnknownModel(m) => Decline::Unpriced(format!(
                        "model {m:?} is not in the pricing table โ€” refusing to send without a \
                         cap price (add it to PRICING_TABLE + bump PRICING_VERSION)"
                    )),
                });
            }
        };
        // Crash-safety: persist the outstanding reserve BEFORE the wire.
        let _ = self.spend.save();

        let leg = riir_llm::Leg {
            label: "hosted-expert".to_string(),
            base: self.endpoint.clone(),
            bearer: self.bearer.clone(),
            model: self.model.clone(),
            wire: riir_llm::Wire::OpenAi,
        };
        let body = self.build_body(snippet, max_tokens);
        let sent = match riir_llm::transport::post_chat_on(&self.agent, &leg, &body) {
            Ok(raw) => raw,
            Err(f) => {
                // The request never completed โ€” nothing billed.
                self.spend.release(reserved);
                let _ = self.spend.save();
                return Err(Decline::Wire(f.message().to_string()));
            }
        };
        // A response arrived โ€” tokens were almost certainly consumed even
        // if the shape walk fails (the qwen-incident rule: spend the
        // reserve, never under-count).
        let (text, usage) = match riir_llm::parse(riir_llm::Wire::OpenAi, &sent) {
            Ok(pair) => pair,
            Err(f) => {
                self.spend.reconcile(reserved, &self.model, 0, 0, false);
                let _ = self.spend.save();
                return Err(Decline::Wire(f.message().to_string()));
            }
        };
        self.spend.reconcile(
            reserved,
            &self.model,
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.reported,
        );
        let _ = self.spend.save();
        let fix = text.trim();
        if fix.is_empty() {
            // An empty answer is the expert declining โ€” not an error.
            return Err(Decline::Wire("expert returned an empty fix".to_string()));
        }
        Ok(fix.to_string())
    }
}

impl L4Fallback for HostedExpertL4 {
    /// The seam's shape: decline (`None`) is always safe. The deadline
    /// is the whole-call budget from NOW (the pipeline calls this on a
    /// genuine miss; there is no earlier clock to inherit).
    fn generate_fix(&mut self, _rule_id: &str, buggy_code: &str) -> Option<String> {
        const EXPERT_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);
        let deadline = Instant::now() + EXPERT_BUDGET;
        self.generate_fix_checked(buggy_code, Some(deadline), 512)
            .ok()
    }

    #[allow(clippy::unnecessary_literal_bound)] // signature is fixed by the trait
    fn name(&self) -> &str {
        "hosted-expert-l4"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_spend::SpendLedger;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// The priced model id the cap/happy-path arms use when the refusal
    /// under test is NOT the pricing: a real `PRICING_TABLE` entry, so the
    /// reserve passes/fails on the CAP alone. Asserted priced here so a
    /// pricing-table pivot re-pins this constant deliberately.
    const PRICED_MODEL: &str = "@cf/zai-org/glm-5.3-flash";

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// An isolated per-test ledger (cap 0 = decline-all, the `llm_spend`
    /// test-mode spelling) at a PID+seq-unique temp path — never the
    /// process-wide `.refine` file, never shared between tests.
    fn ledger(cap: f64) -> SpendLedger {
        let p = std::env::temp_dir().join(format!(
            "hosted_expert_spend_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&p);
        SpendLedger::open_at_with_cap(p, cap)
    }

    fn expert(endpoint: String, cap: f64) -> HostedExpertL4 {
        HostedExpertL4::new(endpoint, "test-expert".into(), ledger(cap))
    }

    /// A `PRICED_MODEL`-identified expert — the model is priced, so a
    /// refusal is genuinely the arm under test (cap / deadline / wire).
    fn priced_expert(endpoint: String, cap: f64) -> HostedExpertL4 {
        HostedExpertL4::new(endpoint, PRICED_MODEL.into(), ledger(cap))
    }

    #[test]
    fn the_test_model_is_priced() {
        assert!(
            crate::llm_spend::price_for(PRICED_MODEL).is_some(),
            "{PRICED_MODEL} left the pricing table — re-pin PRICED_MODEL deliberately"
        );
    }

    const OK_BODY: &str = r#"{"choices":[{"message":{"content":"fn f() -> i32 { 1 }"},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;

    /// A loopback one-shot OpenAI-compatible server: accepts ONE
    /// connection, records the request bytes, answers `body` over HTTP.
    /// The receiver lets a test assert on exactly what left the box.
    fn fake_expert(body: &str) -> (String, std::sync::mpsc::Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback bind");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = std::sync::mpsc::channel();
        let owned = body.to_string();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                // Read the WHOLE request: one `read` returns at the first
                // packet boundary, which can be headers-only — loop until
                // the headers end and Content-Length bytes have arrived.
                let mut buf = Vec::with_capacity(65536);
                let mut chunk = [0u8; 8192];
                loop {
                    let n = sock.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(hend) = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
                    {
                        let head = String::from_utf8_lossy(&buf[..hend]).to_ascii_lowercase();
                        let clen: usize = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse().ok())
                            .unwrap_or(0);
                        if buf.len() >= hend + clen {
                            break;
                        }
                    }
                }
                let _ = tx.send(buf);
                let http = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    owned.len(),
                    owned
                );
                let _ = sock.write_all(http.as_bytes());
            }
        });
        (format!("http://{addr}"), rx)
    }

    // ── (§6) consent: Unset must never open a socket ───────────────────

    #[test]
    fn consent_parse_refuses_everything_but_the_deliberate_spelling() {
        // The scope is its OWN: no other consent spelling unlocks code
        // egress (the generic-Unset trap the plan names).
        for other in [
            None,
            Some(""),
            Some("   "),
            Some("1"),
            Some("true"),
            Some("yes"),
            Some("stats"),
        ] {
            let verdict = EgressConsent::parse(other);
            assert!(!verdict.is_allowed(), "{other:?} must not consent");
        }
        assert!(matches!(
            EgressConsent::parse(None),
            EgressConsent::Refused(EgressRefusal::Unset)
        ));
        // The refusal NAMES the code-egress scope (the test-spec line).
        let msg = match EgressConsent::parse(None) {
            EgressConsent::Refused(r) => r.to_string(),
            EgressConsent::Allowed => panic!("unset must refuse"),
        };
        assert!(msg.contains(EGRESS_ENV), "names the env: {msg}");
        assert!(msg.contains("scope of its own"), "{msg}");
        // A junk value refuses LOUD (naming the accepted spelling), never
        // silently.
        let EgressConsent::Refused(junk) = EgressConsent::parse(Some("snipnet")) else {
            panic!("junk must refuse")
        };
        assert!(junk.to_string().contains(EGRESS_ALLOW));
        assert!(EgressConsent::parse(Some(EGRESS_ALLOW)).is_allowed());
    }

    #[test]
    fn refusal_blocks_the_wire_before_any_socket() {
        let (endpoint, rx) = fake_expert(OK_BODY);
        let mut e = expert(endpoint, 1.0);
        let err = e
            .generate_fix_checked_with("fn f() { 0 }", None, 64, EgressConsent::parse(None))
            .expect_err("must decline without consent");
        assert_eq!(err, Decline::Egress(EgressRefusal::Unset));
        assert!(
            rx.try_recv().is_err(),
            "NO connection may arrive when egress is Unset"
        );
    }

    // ── (§6) deadline expiry => decline, nothing sent ──────────────────

    #[test]
    fn expired_deadline_declines_before_the_socket() {
        let (endpoint, rx) = fake_expert(OK_BODY);
        let mut e = priced_expert(endpoint, 1.0);
        let past = Instant::now().checked_sub(std::time::Duration::from_secs(1));
        let err = e
            .generate_fix_checked_with("fn f() { 0 }", past, 64, EgressConsent::Allowed)
            .expect_err("expired deadline must decline");
        assert_eq!(err, Decline::DeadlineExpired);
        assert!(
            rx.try_recv().is_err(),
            "an expired deadline must not reach the wire"
        );
    }

    // ── (§6) cap reached => decline with NO retry ────────────────────

    #[test]
    fn cap_reached_declines_and_the_second_call_declines_too() {
        let (endpoint, rx) = fake_expert(OK_BODY);
        // cap 0 = decline-all (the test-mode spelling); the model is
        // priced so the refusal is genuinely the cap.
        let mut e = priced_expert(endpoint, 0.0);
        let err = e
            .generate_fix_checked_with("fn f() { 0 }", None, 64, EgressConsent::Allowed)
            .expect_err("cap 0 must decline the first call");
        assert!(matches!(err, Decline::CapReached(_)), "got {err:?}");
        // NO retry-into-burn: the second call declines from the LEDGER
        // again (the cap is terminal for the day), and still no socket.
        let err2 = e
            .generate_fix_checked_with("fn g() { 1 }", None, 64, EgressConsent::Allowed)
            .expect_err("the cap stays terminal — no retry");
        assert!(matches!(err2, Decline::CapReached(_)));
        assert!(rx.try_recv().is_err(), "a capped call never opens a socket");
        // The ledger counted both declines (the honest-counter rule).
        assert_eq!(e.spend.state().declines, 2);
    }

    #[test]
    fn unpriced_model_is_its_own_decline_class() {
        // A model outside PRICING_TABLE refuses BEFORE the wire (never a
        // $0 guess) — and is NOT the cap class (a config fix, not a wait).
        let (endpoint, rx) = fake_expert(OK_BODY);
        let mut e = expert(endpoint, 1.0);
        let err = e
            .generate_fix_checked_with("fn f() { 0 }", None, 64, EgressConsent::Allowed)
            .expect_err("unpriced model must refuse");
        assert!(matches!(err, Decline::Unpriced(_)), "got {err:?}");
        assert!(rx.try_recv().is_err());
    }

    // ── (§6) snippet-only payload + the retention rule on the wire ──

    #[test]
    fn body_is_snippet_only_and_carries_the_retention_rule() {
        let e = expert("http://127.0.0.1:1".into(), 1.0);
        let snippet = "fn f() -> i32 { return 1; }";
        let body = e.build_body(snippet, 256);
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");

        // The snippet is there…
        let user = v["messages"][1]["content"].as_str().expect("user msg");
        assert_eq!(user, snippet, "the payload is EXACTLY the snippet");
        // …and the retention rule rides the wire twice.
        let system = v["messages"][0]["content"].as_str().expect("system msg");
        assert!(system.contains("not be retained"), "{system}");
        assert_eq!(v["metadata"]["retention"], "none");
        // OpenAI wire: model in the body, temperature pinned.
        assert_eq!(v["model"], "test-expert");
        assert_eq!(v["temperature"], 0.0);
    }

    #[test]
    fn sent_bytes_carry_the_snippet_and_nothing_else_of_the_host() {
        // The §6 byte test: the request that left the box contains the
        // snippet, the retention rule, and NO content from outside the
        // snippet. The marker stands for hypothetical FILE content — it
        // is deliberately NOT part of the snippet, so if any file-scoped
        // path ever leaked into the payload, the marker would ride along
        // and this test would catch it in the recorded bytes.
        let file_marker = "FILE_CONTENT_MARKER_must_never_travel";
        let snippet = "fn real() { 7 }";
        let (endpoint, rx) = fake_expert(OK_BODY);
        let mut e = priced_expert(endpoint, 1.0);
        let fix = e
            .generate_fix_checked_with(snippet, None, 64, EgressConsent::Allowed)
            .expect("the happy path answers");
        assert_eq!(fix, "fn f() -> i32 { 1 }");

        let sent = String::from_utf8(rx.recv().expect("the server saw the request")).expect("utf8");
        assert!(sent.contains(snippet), "snippet travels: {sent}");
        assert!(!sent.contains(file_marker), "file content never travels");
        assert!(sent.contains("retention"), "the rule rides the wire");
    }

    // ── the seam contract ───────────────────────────────────────────

    #[test]
    fn seam_declines_safely_and_names_itself() {
        // Without consent (the env is unset in tests — the default
        // posture), the L4Fallback surface is None — never a panic,
        // never a partial answer (the conservative contract).
        let mut e = expert("http://127.0.0.1:1".into(), 1.0);
        assert!(e.generate_fix("rule", "fn f() { 0 }").is_none());
        assert_eq!(L4Fallback::name(&e), "hosted-expert-l4");
    }

    #[test]
    fn debug_never_carries_the_endpoint_or_the_token() {
        let e = expert("http://secret-host:9".into(), 1.0);
        let rendered = format!("{e:?}");
        assert!(!rendered.contains("secret-host"), "{rendered}");
    }

    // ── Issue 234 T1: the steer-guidance builder ────────────────────────

    #[test]
    fn steer_body_carries_the_guidance_and_the_no_retention_rule() {
        let e = expert("http://127.0.0.1:1".into(), 1.0);
        let g = SteerGuidance::new(
            "borrow of moved value: `v`",
            "clippy::move_and_deref; the value is moved into the closure",
            "borrow before the move, or clone at the capture site",
            "the snippet compiles and the lint stays fixed",
        );
        let body = e.build_steer_body("fn f() { v }", &g, 512);
        // The no-retention rule rides the wire exactly as the takeover
        // builder rides it (system message + metadata).
        assert!(body.contains("must not be retained"), "{body}");
        assert!(
            body.contains("\"retention\":\"none\"") || body.contains("retention\": \"none"),
            "{body}"
        );
        // All four labeled sections are present.
        for section in [
            "Discrepancy: borrow of moved value",
            "Applicable conditions: clippy::move_and_deref",
            "Corrective rule: borrow before the move",
            "Expected outcome: the snippet compiles",
        ] {
            assert!(body.contains(section), "missing {section:?} in {body}");
        }
        // The snippet rides too (snippet-only law: no file bytes, but the
        // snippet itself is the payload).
        assert!(body.contains("fn f() { v }"), "{body}");
    }

    #[test]
    fn steer_guidance_bounds_fields_and_marks_empties() {
        let long = "w".repeat(SteerGuidance::FIELD_MAX_BYTES + 500);
        let g = SteerGuidance::new(&long, "", "  ", "ok");
        assert!(g.discrepancy.len() <= SteerGuidance::FIELD_MAX_BYTES);
        assert_eq!(g.conditions, "(not recorded)");
        assert_eq!(g.corrective_rule, "(not recorded)");
        assert_eq!(g.expected_outcome, "ok");
        // Char-boundary safety: a multibyte tail never splits mid-character.
        let mb = "é".repeat(SteerGuidance::FIELD_MAX_BYTES);
        let g2 = SteerGuidance::new(&mb, "a", "b", "c");
        assert!(g2.discrepancy.chars().all(|c| c == 'é'));
    }

    #[test]
    fn steer_call_declines_before_the_socket_on_egress_and_deadline() {
        // Egress refused (env unset in tests): decline, nothing sent.
        let mut e = priced_expert("http://127.0.0.1:1".into(), 1.0);
        let g = SteerGuidance::new("d", "c", "r", "e");
        let verdict = e.generate_steer_fix_checked_with(
            "fn f() { 0 }",
            &g,
            Some(Instant::now() + std::time::Duration::from_secs(60)),
            512,
            EgressConsent::from_env(),
        );
        assert!(
            matches!(verdict, Err(Decline::Egress(_))),
            "unset env must refuse the steer call like the takeover call"
        );
        // Expired deadline: declined pre-socket.
        let expired = Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .expect("clock before 1970");
        let verdict = e.generate_steer_fix_checked_with(
            "fn f() { 0 }",
            &g,
            Some(expired),
            512,
            EgressConsent::Allowed,
        );
        assert!(matches!(verdict, Err(Decline::DeadlineExpired)));
    }

    #[test]
    fn steer_call_happy_path_sends_the_guided_body() {
        let (addr, rx) = fake_expert(OK_BODY);
        let mut e = priced_expert(addr, 1.0);
        let g = SteerGuidance::new(
            "the borrow is moved",
            "the closure captures by move",
            "clone at the capture site",
            "it compiles",
        );
        let fix = e
            .generate_steer_fix_checked_with(
                "fn f() { v }",
                &g,
                Some(Instant::now() + std::time::Duration::from_secs(60)),
                512,
                EgressConsent::Allowed,
            )
            .expect("the happy path returns the expert's fix");
        assert_eq!(fix, "fn f() -> i32 { 1 }");
        let sent = String::from_utf8(rx.recv().expect("request captured")).expect("utf8");
        assert!(sent.contains("hosted-expert-steer") || sent.contains("Revise YOUR OWN response"));
        assert!(
            sent.contains("Corrective rule: clone at the capture site"),
            "{sent}"
        );
        assert!(sent.contains("must not be retained"), "{sent}");
    }
}
