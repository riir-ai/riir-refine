//! Issue 051 — groq-first free-tier LLM routing.
//!
//! The owner provisioned a Groq API key (`.env`, gitignored): an
//! OpenAI-compatible chat-completions REST API with a free tier of its own.
//! Owner direction: **call Groq first for reasoning; fall back to Cloudflare
//! when the free limit hits** — the free tier absorbs the cost, CF remains
//! the metered fallback its USD cap ([`crate::llm_spend`], Issue 046)
//! already guards. Usage lands on the ops dashboard via the cost-ledger
//! `providers.groq` block (Issue 044's ledger-as-archive convention).
//!
//! # The ladder (per attempt)
//!
//! ```text
//! attempt → Groq (free) ── ok ──→ done (usd = 0)
//!              │ 429 → RateLimited: groq DISABLED for the rest of the
//!              │        window (limits are windowed; hammering a 429 is
//!              │        waste), this attempt falls back to CF
//!              │ transport/HTTP error → fall back this attempt, groq stays
//!              │        armed (transient)
//!              │ finish_reason=length + null content → OVERFLOW: surfaced,
//!              │        NOT fallen back (a budget problem overflows on CF
//!              │        too — falling back burns paid neurons on a
//!              │        deterministic failure); the drain's existing
//!              │        budget-escalation retry re-enters here
//!              ▼
//!          Cloudflare CfClient (unchanged: cap + smoke + ledger)
//! ```
//!
//! `GROQ_API_KEY` absent → pure-CF behavior, byte-identical to pre-051.
//! CF creds stay MANDATORY (`CfClient::from_env` fails hard) — the free
//! tier alone is not the contract.
//!
//! # Accounting honesty (the load-bearing rule)
//!
//! Groq tokens are FREE and must never inflate the CF cost estimate:
//! `ArmSummary` (the ledger `client` block) stays CF-only; groq stats ride
//! the ledger row's `providers.groq` block. `usd_avoided` is an ESTIMATE
//! (groq tokens × the CF fallback model's per-M price) and is labeled as
//! one everywhere it renders.
//!
//! The qwen3 `/no_think` soft switch is carried over from `CfClient`
//! (same serving family). If Groq's stack ignores it, the overflow
//! classification + the drain's budget escalation still bound the cost
//! (which is zero on the free tier).

use std::time::Duration;

/// Mirrors `cf_workers_ai::REASONING_OVERFLOW` (same string, defined locally
/// so this module compiles under `groq_first` alone). Drift is pinned by
/// `the_overflow_prefix_matches_cf_clients` (both-features only).
pub const REASONING_OVERFLOW: &str = "reasoning-overflow";

/// Default Groq model — the owner's `.env` probe id.
pub const DEFAULT_GROQ_MODEL: &str = "qwen/qwen3.8-27b";

const GROQ_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

/// One Groq generation.
#[derive(Debug, Clone)]
pub struct GroqGen {
    pub text: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    /// Thinking-token size diagnostic (the `message.reasoning` field; 0 when
    /// the model doesn't reason or thinking is suppressed).
    pub reasoning_chars: usize,
    /// Whether the response carried a `usage` object.
    pub usage_reported: bool,
    /// Remaining free-tier quota from the response headers (diagnostic —
    /// the next call 429s when either hits 0; no pre-flight needed).
    pub remaining_requests: Option<u64>,
    pub remaining_tokens: Option<u64>,
    pub latency_ms: u128,
}

/// Why a Groq call failed. The router's fallback decision keys on this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroqError {
    /// Free-tier limit hit (HTTP 429, or the error body names rate limits) —
    /// disable groq for the window, fall back this attempt.
    RateLimited(String),
    /// `finish_reason=length` with null/empty content — the thinking-model
    /// starvation class. Surfaced, never fallen back (a budget problem).
    Overflow(String),
    /// Transport / non-429 HTTP / shape failure — transient-class; fall back
    /// this attempt, groq stays armed.
    Transport(String),
}

impl GroqError {
    /// The message body (for logs and fallback notices). Never contains the
    /// token (the `CfClient` rule — server error bodies don't echo auth).
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            GroqError::RateLimited(m) | GroqError::Overflow(m) | GroqError::Transport(m) => m,
        }
    }
}

/// Minimal blocking Groq chat-completions client (OpenAI-compatible REST).
///
/// The token is held privately and NEVER appears in errors or Debug output.
pub struct GroqClient {
    agent: ureq::Agent,
    url: String,
    token: String,
    model: String,
}

impl std::fmt::Debug for GroqClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately no token.
        f.debug_struct("GroqClient").finish_non_exhaustive()
    }
}

impl GroqClient {
    /// The model id this client targets (for ledger rows).
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model
    }

    /// True when `GROQ_API_KEY` is set (the router's arming check).
    #[must_use]
    pub fn available() -> bool {
        std::env::var("GROQ_API_KEY").is_ok()
    }

    /// Build from `GROQ_API_KEY` (+ optional model override; `GROQ_MODEL`
    /// is the env override, the arg wins when given).
    ///
    /// # Errors
    ///
    /// A human-readable message naming the missing env var — never the token.
    pub fn from_env(model: Option<&str>) -> Result<Self, String> {
        let token = std::env::var("GROQ_API_KEY").map_err(|_| {
            "GROQ_API_KEY is not set — groq-first routing needs it (export GROQ_API_KEY=...)"
                .to_string()
        })?;
        let model = model
            .map(str::to_string)
            .or_else(|| std::env::var("GROQ_MODEL").ok())
            .unwrap_or_else(|| DEFAULT_GROQ_MODEL.to_string());
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_mins(3))
                .build(),
            url: GROQ_URL.to_string(),
            token,
            model,
        })
    }

    /// One generation. Temperature 0 for reproducibility (the `CfClient` rule).
    ///
    /// # Errors
    ///
    /// [`GroqError`] — classified for the router's fallback decision.
    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<GroqGen, GroqError> {
        let t0 = std::time::Instant::now();
        let body = build_groq_body(&self.model, prompt, max_tokens);
        let resp = self
            .agent
            .post(&self.url)
            .set("Authorization", &format!("Bearer {}", self.token))
            .send_json(body)
            .map_err(|e| match e {
                ureq::Error::Status(code, r) => {
                    // Best-effort body extract for the message; the token is
                    // never echoed by the API in error bodies.
                    let detail = r
                        .into_json::<serde_json::Value>()
                        .ok()
                        .and_then(|v| {
                            v.pointer("/error/message")
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_string)
                        })
                        .unwrap_or_else(|| format!("HTTP {code}"));
                    if code == 429 {
                        GroqError::RateLimited(format!("HTTP 429: {detail}"))
                    } else {
                        GroqError::Transport(format!("HTTP {code}: {detail}"))
                    }
                }
                ureq::Error::Transport(other) => {
                    GroqError::Transport(format!("transport: {other}"))
                }
            })?;
        let remaining_requests =
            parse_quota_header(resp.header("x-ratelimit-remaining-requests"));
        let remaining_tokens = parse_quota_header(resp.header("x-ratelimit-remaining-tokens"));
        let v: serde_json::Value = resp
            .into_json()
            .map_err(|e| GroqError::Transport(format!("response was not JSON: {e}")))?;
        let (text, reasoning_chars, prompt_tokens, completion_tokens, total_tokens, usage_reported) =
            parse_groq_result(&v).map_err(|e| match e {
                ParseFail::Overflow(m) => GroqError::Overflow(m),
                ParseFail::Shape(m) => GroqError::Transport(m),
            })?;
        Ok(GroqGen {
            text,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            reasoning_chars,
            usage_reported,
            remaining_requests,
            remaining_tokens,
            latency_ms: t0.elapsed().as_millis(),
        })
    }
}

/// Build the OpenAI-compatible request body, carrying the qwen3 `/no_think`
/// mitigation (the same soft switch `CfClient` applies — same serving
/// family). Pure so the mitigation is unit-testable without a network.
#[must_use]
pub fn build_groq_body(model: &str, prompt: &str, max_tokens: usize) -> serde_json::Value {
    let content = if model.contains("qwen3") {
        format!("{prompt} /no_think")
    } else {
        prompt.to_string()
    };
    serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": content }],
        "max_tokens": max_tokens,
        "temperature": 0.0,
    })
}

/// Parse an `x-ratelimit-remaining-*` header value (`None` = absent/unparseable).
#[must_use]
pub fn parse_quota_header(v: Option<&str>) -> Option<u64> {
    v.and_then(|s| s.trim().parse::<u64>().ok())
}

/// Why [`parse_groq_result`] failed — overflow is a BUDGET verdict (surfaced
/// to the drain's escalation path), shape is transient (fallback-class).
#[derive(Debug)]
enum ParseFail {
    Overflow(String),
    Shape(String),
}

/// Parse the OpenAI-compatible response into
/// `(text, reasoning_chars, prompt_tokens, completion_tokens, total_tokens, usage_reported)`.
///
/// Null/absent/empty `content` with `finish_reason == "length"` is the
/// thinking-model starvation class (same signature `cf_workers_ai::
/// parse_chat_result` classifies) — the overflow error text starts with
/// [`REASONING_OVERFLOW`] so `starts_with` classifies it without parsing.
///
/// # Errors
///
/// [`ParseFail::Overflow`] for the budget class; [`ParseFail::Shape`] for
/// anything else (fallback-class).
fn parse_groq_result(
    v: &serde_json::Value,
) -> Result<(String, usize, u64, u64, u64, bool), ParseFail> {
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str);
    let finish = v
        .pointer("/choices/0/finish_reason")
        .and_then(serde_json::Value::as_str);
    if finish == Some("length")
        && content.is_none_or(|c| c.trim().is_empty())
    {
        return Err(ParseFail::Overflow(format!(
            "{REASONING_OVERFLOW}: content null/empty with finish_reason=length — \
             raise max_tokens or disable thinking"
        )));
    }
    let Some(text) = content else {
        return Err(ParseFail::Shape(format!(
            "groq response had no message content: {v}"
        )));
    };
    let reasoning_chars = v
        .pointer("/choices/0/message/reasoning")
        .and_then(serde_json::Value::as_str)
        .map_or(0, str::len);
    let usage = v.get("usage");
    let usage_reported = usage.is_some();
    let get = |k: &str| {
        usage
            .and_then(|u| u.get(k))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    };
    Ok((
        text.to_string(),
        reasoning_chars,
        get("prompt_tokens"),
        get("completion_tokens"),
        get("total_tokens"),
        usage_reported,
    ))
}

// ── Provider ladder (requires the CF fallback leg) ─────────────────────────

#[cfg(feature = "cf_workers_ai")]
pub mod router {
    //! The groq-first provider ladder over [`crate::cf_workers_ai::CfClient`].

    use super::{GroqClient, GroqError};
    use crate::cf_workers_ai::CfClient;

    /// Which provider served a generation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Provider {
        Groq,
        Cloudflare,
    }

    impl Provider {
        /// Ledger/dashboard token (`"groq"` / `"cloudflare"`).
        #[must_use]
        pub fn as_str(self) -> &'static str {
            match self {
                Provider::Groq => "groq",
                Provider::Cloudflare => "cloudflare",
            }
        }
    }

    /// One generation from the ladder (a provider-tagged superset of the
    /// CF client's `CfGen`).
    #[derive(Debug, Clone)]
    pub struct RouterGen {
        pub text: String,
        pub prompt_tokens: u64,
        pub completion_tokens: u64,
        pub reasoning_chars: usize,
        pub latency_ms: u128,
        pub provider: Provider,
        /// CF billing unit — always 0.0 for groq (free tier).
        pub neurons: f64,
        pub usage_reported: bool,
        /// Set when provider == Cloudflare AND groq was tried first: why the
        /// attempt fell back (the notice text, never a token).
        pub groq_fallback: Option<String>,
    }

    /// Per-window groq stats (the ledger `providers.groq` block's source).
    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct GroqStats {
        /// Attempts made against groq.
        pub calls: u64,
        pub ok: u64,
        pub prompt_tokens: u64,
        pub completion_tokens: u64,
        pub rate_limited: u64,
        pub transport_errors: u64,
        /// Attempts that went to CF because groq failed.
        pub fallbacks: u64,
    }

    /// The groq-first ladder. `GROQ_API_KEY` absent → pure CF (pre-051
    /// behavior, byte-identical). CF creds stay mandatory — the fallback
    /// must exist.
    pub struct LlmRouter {
        cf: CfClient,
        groq: Option<GroqClient>,
        /// Set on the FIRST rate-limit: free-tier limits are windowed, so
        /// hammering a 429 is waste. Transport errors do NOT disable.
        groq_disabled: bool,
        disable_reason: Option<String>,
        pub stats: GroqStats,
    }

    impl std::fmt::Debug for LlmRouter {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("LlmRouter")
                .field("groq_armed", &self.groq_enabled())
                .field("stats", &self.stats)
                .finish_non_exhaustive()
        }
    }

    impl LlmRouter {
        /// Build from env: `CfClient::from_env` (mandatory — the fallback)
        /// + `GroqClient::from_env` (optional — the free leg).
        ///
        /// # Errors
        ///
        /// The CF client's `from_env` message (missing `CF_API_TOKEN` /
        /// `CF_ACCOUNT_ID`, unpriced model). A missing `GROQ_API_KEY` is NOT
        /// an error — the router degrades to pure CF.
        pub fn from_env(cf_model: Option<&str>) -> Result<Self, String> {
            let cf = CfClient::from_env(cf_model)?;
            let groq = if GroqClient::available() {
                match GroqClient::from_env(None) {
                    Ok(g) => Some(g),
                    Err(e) => {
                        // Unreachable in practice (available() checked the
                        // same var) — degrade loudly rather than guess.
                        eprintln!("[router] groq key present but client build failed: {e}");
                        None
                    }
                }
            } else {
                None
            };
            Ok(Self {
                cf,
                groq,
                groq_disabled: false,
                disable_reason: None,
                stats: GroqStats::default(),
            })
        }

        /// True when the free leg is armed (key present, not yet disabled).
        #[must_use]
        pub fn groq_enabled(&self) -> bool {
            self.groq.is_some() && !self.groq_disabled
        }

        /// The groq model id, when the leg exists.
        #[must_use]
        pub fn groq_model(&self) -> Option<&str> {
            self.groq.as_ref().map(GroqClient::model_id)
        }

        /// Why groq is disabled, when it is.
        #[must_use]
        pub fn disable_reason(&self) -> Option<&str> {
            self.disable_reason.as_deref()
        }

        /// The CF (billing/fallback) model id — the ledger row's `model`.
        #[must_use]
        pub fn model_id(&self) -> &str {
            self.cf.model_id()
        }

        /// The CF client (spend state + server-meter passthroughs).
        #[must_use]
        pub fn cf(&self) -> &CfClient {
            &self.cf
        }

        /// One generation through the ladder. `Err` carries CF's exact
        /// message prefixes (cap reached / reasoning-overflow / transport)
        /// so existing callers' classification needs zero changes.
        ///
        /// # Panics
        ///
        /// Never in practice: the `groq_enabled` guard implies the client is
        /// `Some`; the `expect` pins that invariant against future edits.
        ///
        /// # Errors
        ///
        /// CF's error surface (the terminal leg). Groq overflow errors are
        /// re-emitted with the same `reasoning-overflow` prefix.
        pub fn generate(&mut self, prompt: &str, max_tokens: usize) -> Result<RouterGen, String> {
            if self.groq_enabled() {
                let groq = self
                    .groq
                    .as_ref()
                    .expect("groq_enabled implies groq is Some");
                self.stats.calls += 1;
                match groq.generate(prompt, max_tokens) {
                    Ok(g) => {
                        self.stats.ok += 1;
                        self.stats.prompt_tokens += g.prompt_tokens;
                        self.stats.completion_tokens += g.completion_tokens;
                        return Ok(RouterGen {
                            text: g.text,
                            prompt_tokens: g.prompt_tokens,
                            completion_tokens: g.completion_tokens,
                            reasoning_chars: g.reasoning_chars,
                            latency_ms: g.latency_ms,
                            provider: Provider::Groq,
                            neurons: 0.0,
                            usage_reported: g.usage_reported,
                            groq_fallback: None,
                        });
                    }
                    // A budget problem overflows on CF too — never fall back
                    // (that would burn paid neurons on a deterministic
                    // failure). Surface with the CF-compatible prefix so the
                    // drain's budget-escalation retry re-enters HERE.
                    Err(GroqError::Overflow(msg)) => return Err(msg),
                    Err(GroqError::RateLimited(msg)) => {
                        self.stats.rate_limited += 1;
                        self.stats.fallbacks += 1;
                        self.groq_disabled = true;
                        self.disable_reason = Some(msg.clone());
                        eprintln!(
                            "[router] groq rate-limited — disabled for the rest of the \
                             window, falling back to CF ({msg})"
                        );
                    }
                    Err(GroqError::Transport(msg)) => {
                        self.stats.transport_errors += 1;
                        self.stats.fallbacks += 1;
                        eprintln!("[router] groq transport error — falling back to CF this attempt ({msg})");
                    }
                }
            }
            let fell_back = self.stats.fallbacks > 0;
            let g = self.cf.generate(prompt, max_tokens)?;
            Ok(RouterGen {
                text: g.text,
                prompt_tokens: g.prompt_tokens,
                completion_tokens: g.completion_tokens,
                reasoning_chars: g.reasoning_chars,
                latency_ms: g.latency_ms,
                provider: Provider::Cloudflare,
                neurons: g.neurons,
                usage_reported: g.usage_reported,
                groq_fallback: fell_back.then(|| {
                    self.disable_reason
                        .clone()
                        .unwrap_or_else(|| "groq transport error".to_string())
                }),
            })
        }
    }

    /// The `usd_avoided` ESTIMATE: groq tokens priced at the CF fallback
    /// model's published per-M rates. Labeled as an estimate everywhere it
    /// renders — groq is free; this is what CF would have charged.
    ///
    /// Token counts are ~10^6-10^7 in any real window — f64's 52-bit
    /// mantissa carries them exactly; the `as` casts are lossless here.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn avoided_usd(
        prompt_tokens: u64,
        completion_tokens: u64,
        price_in_per_m: f64,
        price_out_per_m: f64,
    ) -> f64 {
        prompt_tokens as f64 / 1_000_000.0 * price_in_per_m
            + completion_tokens as f64 / 1_000_000.0 * price_out_per_m
    }

    /// The ledger row's `providers.groq` block (the dashboard's absorption
    /// axis reads this; legacy rows without the key stay cloudflare-only).
    /// `usd_avoided_est` is the [`avoided_usd`] estimate.
    #[must_use]
    pub fn providers_json(
        stats: &GroqStats,
        usd_avoided_est: f64,
        groq_model: Option<&str>,
    ) -> serde_json::Value {
        let any = stats.calls > 0 || stats.rate_limited > 0 || stats.transport_errors > 0;
        serde_json::json!({
            "groq": {
                "enabled": any,
                "model": groq_model,
                "calls": stats.calls,
                "ok": stats.ok,
                "prompt_tokens": stats.prompt_tokens,
                "completion_tokens": stats.completion_tokens,
                "usd_est": 0.0,
                "usd_avoided_est": usd_avoided_est,
                "rate_limited": stats.rate_limited,
                "transport_errors": stats.transport_errors,
                "fallbacks_to_cf": stats.fallbacks,
                "disabled_reason": serde_json::Value::Null,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── body build ────────────────────────────────────────────────────────

    #[test]
    fn qwen3_models_carry_the_no_think_suffix() {
        let body = build_groq_body(DEFAULT_GROQ_MODEL, "fix this", 512);
        let content = body
            .pointer("/messages/0/content")
            .and_then(serde_json::Value::as_str)
            .expect("content");
        assert_eq!(content, "fix this /no_think");
        assert_eq!(
            body.get("model").and_then(serde_json::Value::as_str),
            Some(DEFAULT_GROQ_MODEL)
        );
        assert_eq!(body.get("temperature").and_then(serde_json::Value::as_f64), Some(0.0));
        assert_eq!(body.get("max_tokens").and_then(serde_json::Value::as_u64), Some(512));
    }

    #[test]
    fn non_qwen_models_pass_through_unchanged() {
        let body = build_groq_body("llama-3.3-70b", "fix this", 64);
        let content = body
            .pointer("/messages/0/content")
            .and_then(serde_json::Value::as_str)
            .expect("content");
        assert_eq!(content, "fix this");
    }

    // ── response parse ────────────────────────────────────────────────────

    fn chat_response(content: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-x",
            "model": DEFAULT_GROQ_MODEL,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content,
                    "reasoning": "let me think about the lint"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 48,
                "total_tokens": 168
            }
        })
    }

    #[test]
    fn openai_shape_parses_with_usage_and_reasoning() {
        let (text, rc, pt, ct, tt, reported) =
            parse_groq_result(&chat_response(serde_json::json!("the fixed code"))).expect("parses");
        assert_eq!(text, "the fixed code");
        assert_eq!(rc, "let me think about the lint".len());
        assert_eq!((pt, ct, tt), (120, 48, 168));
        assert!(reported);
    }

    #[test]
    fn overflow_is_classified_not_shape() {
        let mut v = chat_response(serde_json::Value::Null);
        v["choices"][0]["finish_reason"] = serde_json::json!("length");
        let err = parse_groq_result(&v).expect_err("overflow");
        let ParseFail::Overflow(m) = err else {
            panic!("expected Overflow, got {err:?}");
        };
        assert!(m.starts_with(REASONING_OVERFLOW), "got: {m}");
    }

    #[test]
    fn empty_content_with_length_finish_is_overflow_too() {
        let mut v = chat_response(serde_json::json!("   "));
        v["choices"][0]["finish_reason"] = serde_json::json!("length");
        assert!(matches!(
            parse_groq_result(&v),
            Err(ParseFail::Overflow(_))
        ));
    }

    #[test]
    fn missing_content_is_shape_not_overflow() {
        let v = serde_json::json!({ "choices": [] });
        assert!(matches!(
            parse_groq_result(&v),
            Err(ParseFail::Shape(_))
        ));
    }

    // ── quota headers ─────────────────────────────────────────────────────

    #[test]
    fn quota_headers_parse_and_tolerate_absence() {
        assert_eq!(parse_quota_header(Some(" 17 ")), Some(17));
        assert_eq!(parse_quota_header(Some("0")), Some(0));
        assert_eq!(parse_quota_header(None), None);
        assert_eq!(parse_quota_header(Some("n/a")), None);
    }

    // ── prefix drift pin (both-features only) ─────────────────────────────

    #[cfg(feature = "cf_workers_ai")]
    #[test]
    fn the_overflow_prefix_matches_cf_clients() {
        assert_eq!(REASONING_OVERFLOW, crate::cf_workers_ai::REASONING_OVERFLOW);
    }

    // ── router (both-features) ────────────────────────────────────────────

    #[cfg(feature = "cf_workers_ai")]
    mod router_tests {
        use super::super::router::{avoided_usd, providers_json, GroqStats};

        #[test]
        fn avoided_usd_prices_both_sides() {
            let usd = avoided_usd(1_000_000, 500_000, 0.5, 0.75);
            assert!((usd - (0.5 + 0.375)).abs() < 1e-9);
            assert_eq!(avoided_usd(0, 0, 0.5, 0.75), 0.0);
        }

        #[test]
        fn providers_block_carries_the_dashboard_contract() {
            let stats = GroqStats {
                calls: 3,
                ok: 2,
                prompt_tokens: 100,
                completion_tokens: 50,
                rate_limited: 1,
                transport_errors: 0,
                fallbacks: 1,
            };
            let v = providers_json(&stats, 0.001, Some("qwen/qwen3.8-27b"));
            let g = v.get("groq").expect("groq block");
            assert_eq!(g.get("enabled").and_then(serde_json::Value::as_bool), Some(true));
            assert_eq!(g.get("calls").and_then(serde_json::Value::as_u64), Some(3));
            assert_eq!(g.get("ok").and_then(serde_json::Value::as_u64), Some(2));
            assert_eq!(g.get("prompt_tokens").and_then(serde_json::Value::as_u64), Some(100));
            assert_eq!(g.get("usd_est").and_then(serde_json::Value::as_f64), Some(0.0));
            assert_eq!(
                g.get("usd_avoided_est").and_then(serde_json::Value::as_f64),
                Some(0.001)
            );
            assert_eq!(g.get("rate_limited").and_then(serde_json::Value::as_u64), Some(1));
            assert_eq!(g.get("fallbacks_to_cf").and_then(serde_json::Value::as_u64), Some(1));
            assert!(g.get("disabled_reason").is_some());
        }

        #[test]
        fn providers_block_is_enabled_false_when_never_called() {
            let v = providers_json(&GroqStats::default(), 0.0, None);
            let g = v.get("groq").expect("groq block");
            assert_eq!(g.get("enabled").and_then(serde_json::Value::as_bool), Some(false));
            assert_eq!(g.get("calls").and_then(serde_json::Value::as_u64), Some(0));
        }
    }

    // ── live smoke (manual — never in CI; burns free quota) ───────────────

    /// Manual pre-flight for the first groq window (T7):
    /// `GROQ_API_KEY=... cargo test --features "cf_workers_ai,groq_first" \
    ///   --lib groq::tests::live_smoke -- --ignored --nocapture`
    #[test]
    #[ignore = "live API call — burns free quota; run manually with GROQ_API_KEY set"]
    fn live_smoke() {
        let Ok(client) = GroqClient::from_env(None) else {
            panic!("GROQ_API_KEY must be set for the live smoke");
        };
        let gen = client
            .generate("Reply with exactly: OK", 32)
            .expect("groq live smoke");
        eprintln!(
            "[groq-smoke] model={} text={:?} usage={}/{} tok (reported={}) quota-req={:?} quota-tok={:?} {}ms",
            client.model_id(),
            gen.text.trim(),
            gen.prompt_tokens,
            gen.completion_tokens,
            gen.usage_reported,
            gen.remaining_requests,
            gen.remaining_tokens,
            gen.latency_ms,
        );
        assert!(
            gen.text.to_uppercase().contains("OK"),
            "smoke reply should contain OK, got: {:?}",
            gen.text
        );
    }
}
