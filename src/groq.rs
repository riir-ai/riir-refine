//! Issue 051 — groq-first free-tier LLM routing. Issue 106 — the infron
//! free leg (groq → infron → CF).
//!
//! The owner provisioned a Groq API key (`.env`, gitignored): an
//! OpenAI-compatible chat-completions REST API with a free tier of its own.
//! Owner direction: **call Groq first for reasoning; fall back to Cloudflare
//! when the free limit hits** — the free tier absorbs the cost, CF remains
//! the metered fallback its USD cap ([`crate::llm_spend`], Issue 046)
//! already guards. Usage lands on the ops dashboard via the cost-ledger
//! `providers.groq` block (Issue 044's ledger-as-archive convention).
//! Issue 106 adds a SECOND free leg between them: `llm.onerouter.pro`
//! (`INFRON_API_KEY`) — same OpenAI-compatible shape, same qwen3.8-27b
//! model family, measured ~33 tok/s. Free but slow, so it sits BELOW
//! groq (fast free) and ABOVE CF (paid): a rate-limited groq degrades to
//! a slow free leg before it ever burns paid neurons.
//!
//! Both free legs are one generic client ([`OaiClient`]) over a static
//! [`EndpointConf`] — the ladder is provider-list-shaped, exactly as
//! Proposal 005 anticipated ("provider list + per-provider quota state,
//! not a hardcoded pair").
//!
//! # The ladder (per attempt)
//!
//! ```text
//! attempt → Groq (free, fast) ── ok ──→ done (usd = 0)
//!              │ 429 → RateLimited: groq DISABLED for the rest of the
//!              │        window (limits are windowed; hammering a 429 is
//!              │        waste), this attempt falls to the next leg
//!              │ transport/HTTP error → fall to the next leg this
//!              │        attempt, groq stays armed (transient)
//!              │ finish_reason=length + null content → OVERFLOW: surfaced,
//!              │        NOT fallen back (a budget problem overflows on
//!              │        every provider — falling back burns the next leg
//!              │        for nothing); the drain's budget-escalation retry
//!              │        re-enters here
//!              ▼
//!          Infron (free, ~33 tok/s) ── ok ──→ done (usd = 0)
//!              │ same three-way split
//!              ▼
//!          Cloudflare CfClient (unchanged: cap + smoke + ledger)
//! ```
//!
//! **Any subset of legs may be absent; at least ONE must be armed.**
//!
//! - `GROQ_API_KEY` absent → the ladder starts at infron (or pure-CF when
//!   `INFRON_API_KEY` is absent too — byte-identical to pre-051).
//! - CF creds absent → a FREE-ONLY window. The ladder has no paid fallback,
//!   so exhausted free legs are terminal for the window (the
//!   [`FREE_TIER_EXHAUSTED`] verdict — a clean stop that still writes the
//!   ledger row, mirroring CF's cap-reached verdict) instead of a crash.
//! - All three absent → refused at construction, naming every var.
//!
//! CF creds were MANDATORY until 2026-08-28. That gated the FREE leg behind
//! the PAID leg's credentials, so the owner's groq-only `.env` could not run
//! a window at all — `kat_drain` exited 3 before spending a single free
//! token, which is the exact opposite of this module's purpose. Note that a
//! misconfigured CF leg (creds PRESENT, build refused — e.g. an unpriced
//! model) still fails hard: that is a config bug, not an absent leg, and
//! must never degrade silently to a free-only window.
//!
//! # Accounting honesty (the load-bearing rule)
//!
//! Free-leg tokens (groq AND infron) are FREE and must never inflate the
//! CF cost estimate: `ArmSummary` (the ledger `client` block) stays
//! CF-only; free-leg stats ride the ledger row's `providers.groq` /
//! `providers.infron` blocks. `usd_avoided` is an ESTIMATE (free-leg
//! tokens × the CF fallback model's per-M price) and is labeled as one
//! everywhere it renders. A leg's `fallbacks_to_cf` counts attempts that
//! ACTUALLY reached CF — a groq→infron hop is measured by infron's own
//! `calls` counter, never implied to be paid traffic.
//!
//! The qwen3 `/no_think` soft switch is carried over from `CfClient`
//! (same serving family; fires for BOTH legs' qwen3 ids, including the
//! `:free` suffixed infron default). If a serving stack ignores it, the
//! overflow classification + the drain's budget escalation still bound
//! the cost (which is zero on the free tiers).
//!
//! # Trust posture (Issue 106)
//!
//! `llm.onerouter.pro` is a third-party aggregator with no vetted
//! data-retention policy. The egress posture is UNCHANGED from the groq
//! leg: every span passes the redaction gate (`crate::kat_redact`, v3)
//! and the own-repo-only sync policy — no new egress class is opened.

use std::time::Duration;

/// Mirrors `cf_workers_ai::REASONING_OVERFLOW` (same string, defined locally
/// so this module compiles under `groq_first` alone). Drift is pinned by
/// `the_overflow_prefix_matches_cf_clients` (both-features only).
pub const REASONING_OVERFLOW: &str = "reasoning-overflow";

/// Default Groq model — the owner's `.env` probe id.
pub const DEFAULT_GROQ_MODEL: &str = "qwen/qwen3.8-27b";

/// Default infron model — the owner's onerouter probe id. Same qwen3.8
/// family as the groq leg (quality parity with the free leg; the `/no_think`
/// soft switch fires on it via the same `contains("qwen3")` rule).
pub const DEFAULT_INFRON_MODEL: &str = "qwen/qwen3.8-27b:free";

/// One endpoint of the free-tier ladder: a static description of an
/// OpenAI-compatible chat-completions service. Two consts exist —
/// [`GROQ_CONF`] and [`INFRON_CONF`] — and [`OaiClient`] is
/// provider-agnostic over them (Issue 106: the ladder is provider-list-
/// shaped, not a hardcoded pair).
#[derive(Clone, Copy)]
struct EndpointConf {
    /// Ledger/log token ("groq" / "infron") — error attribution.
    label: &'static str,
    url: &'static str,
    /// The env var holding the API key.
    key_env: &'static str,
    /// The env var holding the optional model override.
    model_env: &'static str,
    default_model: &'static str,
}

const GROQ_CONF: EndpointConf = EndpointConf {
    label: "groq",
    url: "https://api.groq.com/openai/v1/chat/completions",
    key_env: "GROQ_API_KEY",
    model_env: "GROQ_MODEL",
    default_model: DEFAULT_GROQ_MODEL,
};

/// Issue 106 — the second free leg. `llm.onerouter.pro`, measured ~33
/// tok/s: FREE but slow, so it sits below groq and above the metered CF
/// fallback. The 3-min [`CALL_TIMEOUT`] comfortably covers a 2k-token
/// completion at that rate (~62 s) plus queueing headroom.
const INFRON_CONF: EndpointConf = EndpointConf {
    label: "infron",
    url: "https://llm.onerouter.pro/v1/chat/completions",
    key_env: "INFRON_API_KEY",
    model_env: "INFRON_MODEL",
    default_model: DEFAULT_INFRON_MODEL,
};

/// Overall per-call ceiling — a reasoning model at a 2k budget can legitimately
/// take minutes (and the infron leg at ~33 tok/s takes ~62 s for a full 2k).
const CALL_TIMEOUT: Duration = Duration::from_mins(3);
/// Connect-phase ceiling. Load-bearing SEPARATELY from [`CALL_TIMEOUT`]: a
/// blackholed TCP connect (dropped packets, DNS-to-nowhere) is indistinguishable
/// from a slow model under a single overall timeout, so it would stall for the
/// full 3 min — × every span in the window. A `--limit 25` run could hang 75
/// minutes on a dead network. Connecting is fast or broken; nothing in between.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// One OpenAI-compatible generation (either free leg).
#[derive(Debug, Clone)]
pub struct OaiGen {
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

/// Why a free-leg call failed. The router's fallback decision keys on this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OaiError {
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

impl OaiError {
    /// The message body (for logs and fallback notices). Never contains the
    /// token (the `CfClient` rule — server error bodies don't echo auth).
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::RateLimited(m) | Self::Overflow(m) | Self::Transport(m) => m,
        }
    }
}

/// Minimal blocking OpenAI-compatible chat-completions client — the
/// substrate of BOTH free legs (groq + infron), parameterized by an
/// [`EndpointConf`].
///
/// The token is held privately and NEVER appears in errors or Debug output.
pub struct OaiClient {
    agent: ureq::Agent,
    url: String,
    /// The pre-formatted `Bearer <token>` header value, built ONCE.
    ///
    /// Two reasons, both deliberate. Perf: `generate` no longer allocates a
    /// fresh header String per call. Security: the secret exists in exactly
    /// one heap location for the client's lifetime instead of being copied
    /// into a new allocation per request and dropped un-zeroed, so it cannot
    /// be smeared across freed heap pages.
    auth: String,
    /// The bare token, kept ONLY to scrub it out of outbound error strings
    /// (defense in depth — see [`OaiClient::scrub`]).
    token: String,
    /// The per-leg redaction placeholder (`"<GROQ_API_KEY redacted>"` /
    /// `"<INFRON_API_KEY redacted>"`), built once.
    redacted: String,
    /// The leg's ledger/log token ("groq" / "infron") — error attribution.
    label: &'static str,
    model: String,
}

impl std::fmt::Debug for OaiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately no token.
        f.debug_struct("OaiClient").finish_non_exhaustive()
    }
}

impl OaiClient {
    /// The model id this client targets (for ledger rows).
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model
    }

    /// True when `GROQ_API_KEY` is set (the router's groq arming check).
    #[must_use]
    pub fn available() -> bool {
        std::env::var(GROQ_CONF.key_env).is_ok()
    }

    /// True when `INFRON_API_KEY` is set (the router's infron arming check).
    #[must_use]
    pub fn infron_available() -> bool {
        std::env::var(INFRON_CONF.key_env).is_ok()
    }

    /// Build the groq leg from `GROQ_API_KEY` (+ optional model override;
    /// `GROQ_MODEL` is the env override, the arg wins when given).
    ///
    /// # Errors
    ///
    /// A human-readable message naming the missing env var — never the token.
    pub fn from_env(model: Option<&str>) -> Result<Self, String> {
        Self::from_conf(GROQ_CONF, model)
    }

    /// Build the infron leg from `INFRON_API_KEY` (`INFRON_MODEL` is the
    /// model override; no explicit-arg override — the drain's one model arg
    /// targets the CF pricing basis, not this leg).
    ///
    /// # Errors
    ///
    /// A human-readable message naming the missing env var — never the token.
    pub fn infron_from_env() -> Result<Self, String> {
        Self::from_conf(INFRON_CONF, None)
    }

    /// The shared builder over an [`EndpointConf`].
    ///
    /// # Errors
    ///
    /// A human-readable message naming the missing env var — never the token.
    fn from_conf(conf: EndpointConf, model: Option<&str>) -> Result<Self, String> {
        let token = std::env::var(conf.key_env).map_err(|_| {
            format!(
                "{} is not set — the {} leg needs it (export {}=...)",
                conf.key_env, conf.label, conf.key_env
            )
        })?;
        let model = model
            .map(str::to_string)
            .or_else(|| std::env::var(conf.model_env).ok())
            .unwrap_or_else(|| conf.default_model.to_string());
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .timeout(CALL_TIMEOUT)
                .timeout_connect(CONNECT_TIMEOUT)
                .build(),
            url: conf.url.to_string(),
            auth: format!("Bearer {token}"),
            redacted: format!("<{} redacted>", conf.key_env),
            label: conf.label,
            token,
            model,
        })
    }

    /// Remove the token from a string bound for a log, error, or ledger row.
    ///
    /// Defense in depth: neither groq nor the onerouter aggregator echoes the
    /// `Authorization` header in error bodies today, and every error path in
    /// this module is hand-audited. This guard is for the paths that DON'T
    /// exist yet — a future provider that echoes the request, a proxy that
    /// inlines the header into a 502 page, a debug endpoint. A leaked key in
    /// `.benchmarks/llm_cost_ledger.jsonl` would be a COMMITTED secret; the
    /// cost of this check is one `contains` on an error path that is already
    /// doing network I/O.
    fn scrub(&self, s: String) -> String {
        // Early return keeps the common (clean) path allocation-free.
        if s.contains(self.token.as_str()) {
            return s.replace(self.token.as_str(), &self.redacted);
        }
        s
    }

    /// One generation. Temperature 0 for reproducibility (the `CfClient` rule).
    ///
    /// # Errors
    ///
    /// [`OaiError`] — classified for the router's fallback decision.
    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<OaiGen, OaiError> {
        let t0 = std::time::Instant::now();
        let body = build_oai_body(&self.model, prompt, max_tokens);
        let resp = self
            .agent
            .post(&self.url)
            .set("Authorization", &self.auth)
            .send_json(body)
            .map_err(|e| match e {
                ureq::Error::Status(code, r) => {
                    // Best-effort body extract for the message, scrubbed —
                    // the body is SERVER-controlled, so it is the one error
                    // string in this module we do not fully author.
                    let detail = r
                        .into_json::<serde_json::Value>()
                        .ok()
                        .and_then(|v| {
                            v.pointer("/error/message")
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_string)
                        })
                        .map_or_else(|| format!("HTTP {code}"), |d| self.scrub(d));
                    if code == 429 {
                        OaiError::RateLimited(format!("HTTP 429: {detail}"))
                    } else {
                        OaiError::Transport(format!("HTTP {code}: {detail}"))
                    }
                }
                // ureq embeds the URL in transport errors; the token lives in
                // a header, not the URL, but scrub anyway — this is the path
                // a future query-param-auth provider would leak through.
                ureq::Error::Transport(other) => {
                    OaiError::Transport(self.scrub(format!("transport: {other}")))
                }
            })?;
        let remaining_requests = parse_quota_header(resp.header("x-ratelimit-remaining-requests"));
        let remaining_tokens = parse_quota_header(resp.header("x-ratelimit-remaining-tokens"));
        let v: serde_json::Value = resp
            .into_json()
            .map_err(|e| OaiError::Transport(self.scrub(format!("response was not JSON: {e}"))))?;
        let (text, reasoning_chars, prompt_tokens, completion_tokens, total_tokens, usage_reported) =
            parse_oai_result(&v, self.label).map_err(|e| match e {
                ParseFail::Overflow(m) => OaiError::Overflow(m),
                ParseFail::Shape(m) => OaiError::Transport(m),
            })?;
        Ok(OaiGen {
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

/// Build the OpenAI-compatible request body (either free leg), carrying the
/// qwen3 `/no_think` mitigation (the same soft switch `CfClient` applies —
/// same serving family, and it fires for the infron leg's `:free`-suffixed
/// ids too). Pure so the mitigation is unit-testable without a network.
#[must_use]
pub fn build_oai_body(model: &str, prompt: &str, max_tokens: usize) -> serde_json::Value {
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

/// Why [`parse_oai_result`] failed — overflow is a BUDGET verdict (surfaced
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
fn parse_oai_result(
    v: &serde_json::Value,
    label: &str,
) -> Result<(String, usize, u64, u64, u64, bool), ParseFail> {
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str);
    let finish = v
        .pointer("/choices/0/finish_reason")
        .and_then(serde_json::Value::as_str);
    if finish == Some("length") && content.is_none_or(|c| c.trim().is_empty()) {
        return Err(ParseFail::Overflow(format!(
            "{REASONING_OVERFLOW}: content null/empty with finish_reason=length — \
             raise max_tokens or disable thinking"
        )));
    }
    let Some(text) = content else {
        return Err(ParseFail::Shape(format!(
            "{label} response had no message content: {v}"
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
    //! The free-first provider ladder over [`crate::cf_workers_ai::CfClient`]:
    //! groq (fast free) → infron (slow free) → Cloudflare (metered).

    use super::{OaiClient, OaiError};
    use crate::cf_workers_ai::CfClient;
    // In-scope HERE, not at the file top: a top-level `use` never reaches a
    // child module, and the 8fc52d22 µ-typing of `avoided_usd` relied on the
    // file-level import — invisible until --all-features turned this
    // cf_workers_ai-gated module on (E0425; the all-features lane has been
    // red since that commit).
    use crate::kat_footer::TOKENS_PER_MTOK;

    /// Which provider served a generation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Provider {
        Groq,
        Infron,
        Cloudflare,
    }

    impl Provider {
        /// Ledger/dashboard token (`"groq"` / `"infron"` / `"cloudflare"`).
        #[must_use]
        pub fn as_str(self) -> &'static str {
            match self {
                Self::Groq => "groq",
                Self::Infron => "infron",
                Self::Cloudflare => "cloudflare",
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
        /// CF billing unit — always 0.0 for a free leg (groq/infron).
        pub neurons: f64,
        pub usage_reported: bool,
        /// Set when provider == Cloudflare AND a free leg was tried first:
        /// which leg failed and why ("<leg>: <notice>", never a token).
        /// Per-attempt precise: the DIRECT predecessor of the CF call. When
        /// a free leg was disabled EARLIER in the window (this attempt
        /// skipped it), the window-level reason is carried instead so every
        /// paid row still shows why it is paying.
        pub free_fallback: Option<String>,
    }

    /// Per-window stats for ONE free leg (the ledger `providers.<leg>`
    /// block's source). `fallbacks` counts attempts that ACTUALLY reached
    /// CF after this leg failed — a groq→infron hop is measured by
    /// infron's own `calls` counter, never implied to be paid traffic.
    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct FreeTierStats {
        /// Attempts made against this leg.
        pub calls: u64,
        pub ok: u64,
        pub prompt_tokens: u64,
        pub completion_tokens: u64,
        pub rate_limited: u64,
        pub transport_errors: u64,
        /// Attempts that went to CF because this leg failed.
        pub fallbacks: u64,
    }

    /// The verdict prefix for "every armed free leg is exhausted and no paid
    /// leg is armed" — TERMINAL for the window: the caller stops cleanly and
    /// still writes its ledger row. Deliberately NOT the USD-cap prefix; no
    /// paid leg exists to cap, and conflating the two would misreport the
    /// cause.
    pub const FREE_TIER_EXHAUSTED: &str = "free-tier exhausted";

    /// The free legs, fast-first order.
    #[derive(Clone, Copy)]
    enum FreeLeg {
        Groq,
        Infron,
    }

    /// Which leg receives an attempt after the current one fails.
    #[derive(Clone, Copy)]
    enum NextLeg {
        Infron,
        Cloudflare,
        Nowhere,
    }

    /// The outcome of one free-leg attempt inside [`LlmRouter::generate`].
    enum LegOutcome {
        /// The leg served this attempt.
        Served(RouterGen),
        /// The leg did not serve: why it failed (leg-labeled), when it
        /// actually tried and failed. `None` = unarmed/already disabled.
        FellThrough(Option<String>),
    }

    /// The free-first ladder: groq → infron → CF. Any subset of legs may be
    /// absent; at least ONE must be armed:
    ///
    /// - `GROQ_API_KEY` absent → the ladder starts at infron.
    /// - `INFRON_API_KEY` absent → groq → CF (the pre-106 ladder).
    /// - CF creds absent → free-only. An exhausted free tier is terminal
    ///   for the window (the [`FREE_TIER_EXHAUSTED`] verdict) instead of a
    ///   hard failure.
    ///
    /// The all-absent case is refused at construction — a router with no
    /// provider can only fail per span, which is worse than failing loudly
    /// once.
    pub struct LlmRouter {
        cf: Option<CfClient>,
        /// The billing/fallback model id — resolved EVEN WHEN the CF leg is
        /// unarmed, so `usd_avoided` keeps an honest pricing basis and the
        /// ledger row keeps a stable `model` field on a free-only window.
        cf_model: String,
        groq: Option<OaiClient>,
        /// Set on the FIRST rate-limit: free-tier limits are windowed, so
        /// hammering a 429 is waste. Transport errors do NOT disable.
        groq_disabled: bool,
        groq_disable_reason: Option<String>,
        pub groq_stats: FreeTierStats,
        infron: Option<OaiClient>,
        /// Same windowed-disable rule as the groq leg (Issue 106).
        infron_disabled: bool,
        infron_disable_reason: Option<String>,
        pub infron_stats: FreeTierStats,
    }

    impl std::fmt::Debug for LlmRouter {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("LlmRouter")
                .field("groq_armed", &self.groq_enabled())
                .field("infron_armed", &self.infron_enabled())
                .field("cf_armed", &self.cf_armed())
                .field("stats", &self.groq_stats)
                .finish_non_exhaustive()
        }
    }

    impl LlmRouter {
        /// Which legs a window gets, or the refusal when none is armed
        /// (pure — env-free, so it is unit-testable without mutating the
        /// process environment). `Ok(cf_wanted)`: build the CF leg.
        ///
        /// # Errors
        ///
        /// No leg armed — the actionable refusal naming every var.
        pub fn arm_plan(
            groq_armed: bool,
            infron_armed: bool,
            cf_creds_present: bool,
        ) -> Result<bool, String> {
            match (groq_armed, infron_armed, cf_creds_present) {
                (_, _, true) => Ok(true),
                (true, _, false) | (_, true, false) => Ok(false),
                (false, false, false) => Err(
                    "no LLM provider is armed: GROQ_API_KEY and INFRON_API_KEY are unset \
                     (the free legs) and CF_API_TOKEN / CF_ACCOUNT_ID are unset (the metered \
                     fallback) — set at least one"
                        .to_string(),
                ),
            }
        }

        /// True when BOTH CF credential vars are present. Distinguishes an
        /// UNARMED leg (creds absent — legitimate, groq may carry the
        /// window) from a MISCONFIGURED one (creds present, build refused,
        /// e.g. an unpriced model), which must never degrade silently.
        fn cf_creds_present() -> bool {
            std::env::var_os("CF_API_TOKEN").is_some_and(|v| !v.is_empty())
                && std::env::var_os("CF_ACCOUNT_ID").is_some_and(|v| !v.is_empty())
        }

        /// Build from env: `OaiClient::from_env` (groq) +
        /// `OaiClient::infron_from_env` (the second free leg) +
        /// `CfClient::from_env` (the metered fallback). Any subset of legs
        /// may be absent; at least one must be armed.
        ///
        /// # Errors
        ///
        /// - No leg armed (`GROQ_API_KEY` + `INFRON_API_KEY` + CF creds all
        ///   unset) — the actionable refusal, naming every var.
        /// - CF creds PRESENT but the client refused (unpriced model, …) —
        ///   the CF message, propagated. A misconfigured fallback is a
        ///   config bug, not an absent leg, so it never degrades to a
        ///   free-only window silently.
        ///
        /// A missing free-leg key is NOT an error (the ladder just starts
        /// lower), and missing CF creds are NOT an error when a free leg is
        /// armed (free-only window).
        pub fn from_env(cf_model: Option<&str>) -> Result<Self, String> {
            let groq = if OaiClient::available() {
                match OaiClient::from_env(None) {
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
            let infron = if OaiClient::infron_available() {
                match OaiClient::infron_from_env() {
                    Ok(c) => Some(c),
                    Err(e) => {
                        eprintln!("[router] infron key present but client build failed: {e}");
                        None
                    }
                }
            } else {
                None
            };
            // Creds present → build hard (propagate an unpriced-model
            // refusal). Absent → the leg is simply unarmed.
            let cf = if Self::arm_plan(groq.is_some(), infron.is_some(), Self::cf_creds_present())? {
                Some(CfClient::from_env(cf_model)?)
            } else {
                eprintln!(
                    "[router] CF fallback NOT armed (CF_API_TOKEN / CF_ACCOUNT_ID unset) \
                     — free-only window: exhausted free tiers end it"
                );
                None
            };
            // Honest pricing basis on a free-only window: the model that
            // WOULD have served — resolved through the same
            // explicit > env > default precedence as an armed client
            // (CfClient::resolve_model_id), so `RIIR_CF_MODEL` reprices a
            // free-only window too. `price_for` is a pure table lookup — it
            // needs no credentials.
            let cf_model = cf.as_ref().map_or_else(
                || CfClient::resolve_model_id(cf_model),
                |c| c.model_id().to_string(),
            );
            Ok(Self {
                cf,
                cf_model,
                groq,
                groq_disabled: false,
                groq_disable_reason: None,
                groq_stats: FreeTierStats::default(),
                infron,
                infron_disabled: false,
                infron_disable_reason: None,
                infron_stats: FreeTierStats::default(),
            })
        }

        /// True when the groq leg is armed (key present, not yet disabled).
        #[must_use]
        pub fn groq_enabled(&self) -> bool {
            self.groq.is_some() && !self.groq_disabled
        }

        /// True when the infron leg is armed (key present, not yet disabled).
        #[must_use]
        pub fn infron_enabled(&self) -> bool {
            self.infron.is_some() && !self.infron_disabled
        }

        /// The groq model id, when the leg exists.
        #[must_use]
        pub fn groq_model(&self) -> Option<&str> {
            self.groq.as_ref().map(OaiClient::model_id)
        }

        /// The infron model id, when the leg exists.
        #[must_use]
        pub fn infron_model(&self) -> Option<&str> {
            self.infron.as_ref().map(OaiClient::model_id)
        }

        /// Why groq is disabled, when it is.
        #[must_use]
        pub fn groq_disable_reason(&self) -> Option<&str> {
            self.groq_disable_reason.as_deref()
        }

        /// Why infron is disabled, when it is.
        #[must_use]
        pub fn infron_disable_reason(&self) -> Option<&str> {
            self.infron_disable_reason.as_deref()
        }

        /// The CF (billing/fallback) model id — the ledger row's `model`.
        /// Resolved even on a groq-only window (the model that WOULD have
        /// served), so pricing and the ledger keep a stable basis.
        #[must_use]
        pub fn model_id(&self) -> &str {
            &self.cf_model
        }

        /// True when the metered fallback leg exists.
        #[must_use]
        pub fn cf_armed(&self) -> bool {
            self.cf.is_some()
        }

        /// The CF client (spend state + server-meter passthroughs) — `None`
        /// on a groq-only window.
        #[must_use]
        pub fn cf(&self) -> Option<&CfClient> {
            self.cf.as_ref()
        }

        /// Which leg receives an attempt after `leg` fails — computed
        /// BEFORE the per-leg field borrows (`&self`). A disabled-in-window
        /// lower free leg is skipped exactly like an unarmed one.
        fn next_after(&self, leg: FreeLeg) -> NextLeg {
            match leg {
                FreeLeg::Groq => {
                    if self.infron.is_some() && !self.infron_disabled {
                        NextLeg::Infron
                    } else if self.cf.is_some() {
                        NextLeg::Cloudflare
                    } else {
                        NextLeg::Nowhere
                    }
                }
                FreeLeg::Infron => {
                    if self.cf.is_some() {
                        NextLeg::Cloudflare
                    } else {
                        NextLeg::Nowhere
                    }
                }
            }
        }

        /// One free-leg attempt. `Err` is the OVERFLOW class ONLY —
        /// surfaced to the caller verbatim (a budget problem overflows on
        /// every provider; falling back would burn the next leg for
        /// nothing). Rate-limit/transport failures return
        /// [`LegOutcome::FellThrough`] with the leg-labeled reason.
        fn attempt_free(
            &mut self,
            leg: FreeLeg,
            prompt: &str,
            max_tokens: usize,
        ) -> Result<LegOutcome, String> {
            // Before the disjoint field borrows below — takes &self.
            let next = self.next_after(leg);
            let (client, disabled, reason, stats, provider) = match leg {
                FreeLeg::Groq => (
                    self.groq.as_ref(),
                    &mut self.groq_disabled,
                    &mut self.groq_disable_reason,
                    &mut self.groq_stats,
                    Provider::Groq,
                ),
                FreeLeg::Infron => (
                    self.infron.as_ref(),
                    &mut self.infron_disabled,
                    &mut self.infron_disable_reason,
                    &mut self.infron_stats,
                    Provider::Infron,
                ),
            };
            if client.is_none() || *disabled {
                return Ok(LegOutcome::FellThrough(None));
            }
            let client = client.expect("guarded above");
            let label = provider.as_str();
            stats.calls += 1;
            match client.generate(prompt, max_tokens) {
                Ok(g) => {
                    stats.ok += 1;
                    stats.prompt_tokens += g.prompt_tokens;
                    stats.completion_tokens += g.completion_tokens;
                    Ok(LegOutcome::Served(RouterGen {
                        text: g.text,
                        prompt_tokens: g.prompt_tokens,
                        completion_tokens: g.completion_tokens,
                        reasoning_chars: g.reasoning_chars,
                        latency_ms: g.latency_ms,
                        provider,
                        neurons: 0.0,
                        usage_reported: g.usage_reported,
                        free_fallback: None,
                    }))
                }
                // A budget problem overflows everywhere — never fall back.
                // Surface with the CF-compatible prefix so the drain's
                // budget-escalation retry re-enters the ladder.
                Err(OaiError::Overflow(msg)) => Err(msg),
                Err(OaiError::RateLimited(msg)) => {
                    stats.rate_limited += 1;
                    *disabled = true;
                    *reason = Some(msg.clone());
                    match next {
                        NextLeg::Infron => eprintln!(
                            "[router] {label} rate-limited — disabled for the rest of the \
                             window, falling back to infron ({msg})"
                        ),
                        // `fallbacks` counts attempts that ACTUALLY reached
                        // CF — the hop to infron is measured by infron's
                        // own `calls` counter.
                        NextLeg::Cloudflare => {
                            stats.fallbacks += 1;
                            eprintln!(
                                "[router] {label} rate-limited — disabled for the rest of \
                                 the window, falling back to CF ({msg})"
                            );
                        }
                        NextLeg::Nowhere => eprintln!(
                            "[router] {label} rate-limited and no fallback armed — the \
                             window ends here ({msg})"
                        ),
                    }
                    Ok(LegOutcome::FellThrough(Some(format!("{label}: {msg}"))))
                }
                Err(OaiError::Transport(msg)) => {
                    stats.transport_errors += 1;
                    match next {
                        NextLeg::Infron => eprintln!(
                            "[router] {label} transport error — falling back to infron this \
                             attempt ({msg})"
                        ),
                        NextLeg::Cloudflare => {
                            stats.fallbacks += 1;
                            eprintln!(
                                "[router] {label} transport error — falling back to CF this \
                                 attempt ({msg})"
                            );
                        }
                        NextLeg::Nowhere => eprintln!(
                            "[router] {label} transport error, no fallback armed — declining \
                             this attempt ({msg})"
                        ),
                    }
                    Ok(LegOutcome::FellThrough(Some(format!("{label}: {msg}"))))
                }
            }
        }

        /// One generation through the ladder. `Err` carries CF's exact
        /// message prefixes (cap reached / reasoning-overflow / transport)
        /// so existing callers' classification needs zero changes.
        ///
        /// # Errors
        ///
        /// CF's error surface (the terminal leg). Free-leg overflow errors
        /// are re-emitted with the same `reasoning-overflow` prefix. On a
        /// free-only window with every free leg down, the
        /// [`FREE_TIER_EXHAUSTED`] prefix (terminal) or a plain per-span
        /// message (transient transport).
        pub fn generate(&mut self, prompt: &str, max_tokens: usize) -> Result<RouterGen, String> {
            // Free legs, fast-first: groq → infron. `notice` carries the
            // DIRECT predecessor's leg-labeled failure for the CF row's
            // `free_fallback` field.
            let mut notice: Option<String> = None;
            for leg in [FreeLeg::Groq, FreeLeg::Infron] {
                match self.attempt_free(leg, prompt, max_tokens)? {
                    LegOutcome::Served(g) => return Ok(g),
                    LegOutcome::FellThrough(why) => {
                        if why.is_some() {
                            notice = why;
                        }
                    }
                }
            }
            // A free leg disabled EARLIER in the window was skipped without
            // failing this attempt — still surface why the paid leg is
            // paying (the window-level reason) on every CF row.
            if notice.is_none() {
                notice = self
                    .groq_disable_reason
                    .as_ref()
                    .map(|r| format!("groq: {r}"))
                    .or_else(|| {
                        self.infron_disable_reason
                            .as_ref()
                            .map(|r| format!("infron: {r}"))
                    });
            }
            let Some(cf) = self.cf.as_mut() else {
                // Free-only window with every free leg down. A rate-limit
                // is TERMINAL (the quota window won't reopen mid-run) and
                // mirrors CF's cap-reached verdict — stop cleanly, keep the
                // ledger row. A transport blip is transient: decline this
                // span only, and let the next one retry the free legs.
                let why =
                    notice.unwrap_or_else(|| "free legs unavailable".to_string());
                return Err(if self.groq_disabled || self.infron_disabled {
                    let armed: Vec<&str> = [
                        self.groq.is_some().then_some("groq"),
                        self.infron.is_some().then_some("infron"),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                    format!(
                        "{FREE_TIER_EXHAUSTED}: {} exhausted ({why}) — no CF fallback \
                         (CF_API_TOKEN / CF_ACCOUNT_ID unset)",
                        armed.join("+")
                    )
                } else {
                    format!("free-leg attempt failed and no CF fallback is armed ({why})")
                });
            };
            let g = cf.generate(prompt, max_tokens)?;
            Ok(RouterGen {
                text: g.text,
                prompt_tokens: g.prompt_tokens,
                completion_tokens: g.completion_tokens,
                reasoning_chars: g.reasoning_chars,
                latency_ms: g.latency_ms,
                provider: Provider::Cloudflare,
                neurons: g.neurons,
                usage_reported: g.usage_reported,
                free_fallback: notice,
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
        // The TOKEN million (per-Mtok rates) — never MICRO_PER_KAT
        // (riir-dapps Plan 035 T10).
        prompt_tokens as f64 / TOKENS_PER_MTOK as f64 * price_in_per_m
            + completion_tokens as f64 / TOKENS_PER_MTOK as f64 * price_out_per_m
    }

    /// The ledger row's `providers` block: `groq` + `infron` sub-blocks
    /// (the dashboard's absorption axis reads these; legacy rows without
    /// the keys stay cloudflare-only). Each `usd_avoided_est` is the
    /// [`avoided_usd`] estimate for THAT leg's tokens at CF prices.
    #[must_use]
    pub fn providers_json(
        groq_stats: &FreeTierStats,
        groq_model: Option<&str>,
        groq_avoided_est: f64,
        infron_stats: &FreeTierStats,
        infron_model: Option<&str>,
        infron_avoided_est: f64,
    ) -> serde_json::Value {
        serde_json::json!({
            "groq": free_leg_block(groq_stats, groq_model, groq_avoided_est),
            "infron": free_leg_block(infron_stats, infron_model, infron_avoided_est),
        })
    }

    /// One free leg's ledger sub-block (the dashboard contract — same keys
    /// the pre-106 groq-only block carried).
    fn free_leg_block(
        stats: &FreeTierStats,
        model: Option<&str>,
        usd_avoided_est: f64,
    ) -> serde_json::Value {
        let any = stats.calls > 0 || stats.rate_limited > 0 || stats.transport_errors > 0;
        serde_json::json!({
            "enabled": any,
            "model": model,
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── body build ────────────────────────────────────────────────────────

    #[test]
    fn qwen3_models_carry_the_no_think_suffix() {
        let body = build_oai_body(DEFAULT_GROQ_MODEL, "fix this", 512);
        let content = body
            .pointer("/messages/0/content")
            .and_then(serde_json::Value::as_str)
            .expect("content");
        assert_eq!(content, "fix this /no_think");
        assert_eq!(
            body.get("model").and_then(serde_json::Value::as_str),
            Some(DEFAULT_GROQ_MODEL)
        );
        assert_eq!(
            body.get("temperature").and_then(serde_json::Value::as_f64),
            Some(0.0)
        );
        assert_eq!(
            body.get("max_tokens").and_then(serde_json::Value::as_u64),
            Some(512)
        );
    }

    #[test]
    fn non_qwen_models_pass_through_unchanged() {
        let body = build_oai_body("llama-3.3-70b", "fix this", 64);
        let content = body
            .pointer("/messages/0/content")
            .and_then(serde_json::Value::as_str)
            .expect("content");
        assert_eq!(content, "fix this");
    }

    #[test]
    fn qwen_free_ids_carry_the_no_think_suffix_too() {
        // The infron leg's default (Issue 106): the `:free` suffix must NOT
        // dodge the qwen3 soft switch.
        let body = build_oai_body(DEFAULT_INFRON_MODEL, "fix this", 512);
        let content = body
            .pointer("/messages/0/content")
            .and_then(serde_json::Value::as_str)
            .expect("content");
        assert_eq!(content, "fix this /no_think");
        assert_eq!(
            body.get("model").and_then(serde_json::Value::as_str),
            Some(DEFAULT_INFRON_MODEL)
        );
    }

    // ── response parse ────────────────────────────────────────────────────

    // `json!` consumes the arg by value — a reference would just force a
    // clone at every call site.
    #[allow(clippy::needless_pass_by_value)]
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
            parse_oai_result(&chat_response(serde_json::json!("the fixed code")), "groq")
                .expect("parses");
        assert_eq!(text, "the fixed code");
        assert_eq!(rc, "let me think about the lint".len());
        assert_eq!((pt, ct, tt), (120, 48, 168));
        assert!(reported);
    }

    #[test]
    fn overflow_is_classified_not_shape() {
        let mut v = chat_response(serde_json::Value::Null);
        v["choices"][0]["finish_reason"] = serde_json::json!("length");
        let err = parse_oai_result(&v, "groq").expect_err("overflow");
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
            parse_oai_result(&v, "groq"),
            Err(ParseFail::Overflow(_))
        ));
    }

    #[test]
    fn missing_content_is_shape_not_overflow() {
        let v = serde_json::json!({ "choices": [] });
        let err = parse_oai_result(&v, "infron").expect_err("shape");
        // The shape error names the leg that produced it.
        let ParseFail::Shape(m) = err else {
            panic!("expected Shape, got {err:?}");
        };
        assert!(m.starts_with("infron response"), "got: {m}");
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
        use super::super::router::{
            avoided_usd, providers_json, FreeTierStats, LlmRouter, FREE_TIER_EXHAUSTED,
        };

        // ── prod-grade hardening (T10) ────────────────────────────────

        #[test]
        fn connect_timeout_is_bounded_well_under_the_call_timeout() {
            // The whole point: a dead network must not consume the per-call
            // ceiling. If these ever converge, a blackholed connect stalls
            // the window again.
            assert!(
                super::super::CONNECT_TIMEOUT < super::super::CALL_TIMEOUT,
                "connect must fail faster than a slow model completes"
            );
            assert!(
                super::super::CONNECT_TIMEOUT.as_secs() <= 15,
                "connecting is fast or broken; nothing in between"
            );
            // A full window must not be able to hang for hours on connects
            // alone: 25 spans × the connect ceiling stays inside ~5 min.
            assert!(super::super::CONNECT_TIMEOUT.as_secs() * 25 <= 300);
        }

        // ── Issue 051 fix: any subset of legs may be absent ──────────

        #[test]
        fn a_groq_only_window_is_armed_not_refused() {
            // THE regression this fix exists for: the owner's .env has only
            // GROQ_API_KEY, and the free leg used to be gated behind the
            // PAID leg's credentials — so no window could run at all.
            assert_eq!(LlmRouter::arm_plan(true, false, false), Ok(false));
        }

        #[test]
        fn an_infron_only_window_is_armed_not_refused() {
            // Issue 106 symmetry: INFRON_API_KEY alone must carry a window.
            assert_eq!(LlmRouter::arm_plan(false, true, false), Ok(false));
        }

        #[test]
        fn cf_creds_present_always_arms_the_fallback() {
            assert_eq!(LlmRouter::arm_plan(true, false, true), Ok(true));
            assert_eq!(LlmRouter::arm_plan(false, true, true), Ok(true));
            assert_eq!(LlmRouter::arm_plan(false, false, true), Ok(true));
        }

        #[test]
        fn no_leg_armed_is_refused_naming_every_var() {
            let e = LlmRouter::arm_plan(false, false, false).expect_err("must refuse");
            assert!(e.contains("GROQ_API_KEY"), "names the fast free leg: {e}");
            assert!(e.contains("INFRON_API_KEY"), "names the slow free leg: {e}");
            assert!(e.contains("CF_API_TOKEN"), "names the paid leg: {e}");
            assert!(e.contains("CF_ACCOUNT_ID"), "names the paid leg: {e}");
        }

        #[test]
        fn the_free_tier_verdict_is_distinct_from_the_usd_cap() {
            // kat_drain treats BOTH as terminal, but they must not share a
            // prefix — the cause has to stay readable in the ledger.
            assert_eq!(FREE_TIER_EXHAUSTED, "free-tier exhausted");
            assert!(!FREE_TIER_EXHAUSTED.starts_with("LLM daily cap reached"));
            assert!(!"LLM daily cap reached: $1".starts_with(FREE_TIER_EXHAUSTED));
        }

        #[test]
        fn a_free_only_window_reports_zero_cf_fallbacks() {
            // `fallbacks_to_cf` must stay 0 when no CF leg exists — a
            // non-zero count would imply paid traffic that never happened.
            let stats = FreeTierStats {
                calls: 3,
                ok: 2,
                rate_limited: 1,
                fallbacks: 0,
                ..FreeTierStats::default()
            };
            let v = providers_json(
                &stats,
                Some("qwen/qwen3.8-27b"),
                0.0,
                &FreeTierStats::default(),
                None,
                0.0,
            );
            assert_eq!(v["groq"]["fallbacks_to_cf"], serde_json::json!(0));
            assert_eq!(v["groq"]["enabled"], serde_json::json!(true));
        }

        #[test]
        #[allow(clippy::float_cmp)] // exact-zero contract: 0 tok MUST price to exactly 0.0
        fn avoided_usd_prices_both_sides() {
            let usd = avoided_usd(1_000_000, 500_000, 0.5, 0.75);
            assert!((usd - (0.5 + 0.375)).abs() < 1e-9);
            assert_eq!(avoided_usd(0, 0, 0.5, 0.75), 0.0);
        }

        #[test]
        fn providers_block_carries_the_dashboard_contract() {
            let stats = FreeTierStats {
                calls: 3,
                ok: 2,
                prompt_tokens: 100,
                completion_tokens: 50,
                rate_limited: 1,
                transport_errors: 0,
                fallbacks: 1,
            };
            let infron = FreeTierStats {
                calls: 2,
                ok: 2,
                prompt_tokens: 80,
                completion_tokens: 40,
                ..FreeTierStats::default()
            };
            let v = providers_json(
                &stats,
                Some("qwen/qwen3.8-27b"),
                0.001,
                &infron,
                Some("qwen/qwen3.8-27b:free"),
                0.002,
            );
            let g = v.get("groq").expect("groq block");
            assert_eq!(
                g.get("enabled").and_then(serde_json::Value::as_bool),
                Some(true)
            );
            assert_eq!(g.get("calls").and_then(serde_json::Value::as_u64), Some(3));
            assert_eq!(g.get("ok").and_then(serde_json::Value::as_u64), Some(2));
            assert_eq!(
                g.get("prompt_tokens").and_then(serde_json::Value::as_u64),
                Some(100)
            );
            assert_eq!(
                g.get("usd_est").and_then(serde_json::Value::as_f64),
                Some(0.0)
            );
            assert_eq!(
                g.get("usd_avoided_est").and_then(serde_json::Value::as_f64),
                Some(0.001)
            );
            assert_eq!(
                g.get("rate_limited").and_then(serde_json::Value::as_u64),
                Some(1)
            );
            assert_eq!(
                g.get("fallbacks_to_cf").and_then(serde_json::Value::as_u64),
                Some(1)
            );
            assert!(g.get("disabled_reason").is_some());
            // Issue 106: the infron block rides the SAME contract shape.
            let i = v.get("infron").expect("infron block");
            assert_eq!(
                i.get("enabled").and_then(serde_json::Value::as_bool),
                Some(true)
            );
            assert_eq!(i.get("calls").and_then(serde_json::Value::as_u64), Some(2));
            assert_eq!(i.get("ok").and_then(serde_json::Value::as_u64), Some(2));
            assert_eq!(
                i.get("usd_est").and_then(serde_json::Value::as_f64),
                Some(0.0)
            );
            assert_eq!(
                i.get("usd_avoided_est").and_then(serde_json::Value::as_f64),
                Some(0.002)
            );
            assert_eq!(
                i.get("model").and_then(serde_json::Value::as_str),
                Some("qwen/qwen3.8-27b:free")
            );
            assert!(i.get("disabled_reason").is_some());
        }

        #[test]
        fn providers_block_is_enabled_false_when_never_called() {
            let v = providers_json(
                &FreeTierStats::default(),
                None,
                0.0,
                &FreeTierStats::default(),
                None,
                0.0,
            );
            for leg in ["groq", "infron"] {
                let b = v.get(leg).unwrap_or_else(|| panic!("{leg} block"));
                assert_eq!(
                    b.get("enabled").and_then(serde_json::Value::as_bool),
                    Some(false)
                );
                assert_eq!(
                    b.get("calls").and_then(serde_json::Value::as_u64),
                    Some(0)
                );
            }
        }
    }

    // ── live smoke (manual — never in CI; burns free quota) ───────────────

    /// Manual pre-flight for the first groq window (T7):
    /// `GROQ_API_KEY=... cargo test --features "cf_workers_ai,groq_first" \
    ///   --lib groq::tests::live_smoke -- --ignored --nocapture`
    #[test]
    #[ignore = "live API call — burns free quota; run manually with GROQ_API_KEY set"]
    fn live_smoke() {
        let Ok(client) = OaiClient::from_env(None) else {
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

    /// Manual pre-flight for the infron leg (Issue 106):
    /// `INFRON_API_KEY=... cargo test --features "cf_workers_ai,groq_first" \
    ///   --lib groq::tests::live_infron_smoke -- --ignored --nocapture`
    ///
    /// At ~33 tok/s a 32-token reply lands in a few seconds; the probe also
    /// measures the wall time so the rate is visible in the log.
    #[test]
    #[ignore = "live API call — burns free quota; run manually with INFRON_API_KEY set"]
    fn live_infron_smoke() {
        let Ok(client) = OaiClient::infron_from_env() else {
            panic!("INFRON_API_KEY must be set for the live infron smoke");
        };
        let gen = client
            .generate("Reply with exactly: OK", 32)
            .expect("infron live smoke");
        eprintln!(
            "[infron-smoke] model={} text={:?} usage={}/{} tok (reported={}) {}ms",
            client.model_id(),
            gen.text.trim(),
            gen.prompt_tokens,
            gen.completion_tokens,
            gen.usage_reported,
            gen.latency_ms,
        );
        assert!(
            gen.text.to_uppercase().contains("OK"),
            "smoke reply should contain OK, got: {:?}",
            gen.text
        );
    }
        #[test]
    fn the_token_is_scrubbed_from_server_controlled_strings() {
        // Built directly (not from_env) so the test never touches the real
        // key and stays parallel-safe.
        let c = OaiClient {
            agent: ureq::AgentBuilder::new().build(),
            url: "https://api.groq.com/openai/v1/chat/completions".to_string(),
            auth: "Bearer gsk_secret".to_string(),
            token: "gsk_secret".to_string(),
            redacted: "<GROQ_API_KEY redacted>".to_string(),
            label: "groq",
            model: DEFAULT_GROQ_MODEL.to_string(),
        };
        let leaked = c.scrub("upstream rejected Bearer gsk_secret at /v1/chat".to_string());
        assert!(
            !leaked.contains("gsk_secret"),
            "token must not survive: {leaked}"
        );
        assert!(leaked.contains("redacted"));
        // The common case is untouched (and allocation-free in spirit — the
        // no-match arm returns the same String).
        let clean = c.scrub("HTTP 500: internal error".to_string());
        assert_eq!(clean, "HTTP 500: internal error");
    }

    #[test]
    fn the_auth_header_is_built_once_and_matches_the_token() {
        let c = OaiClient {
            agent: ureq::AgentBuilder::new().build(),
            url: "https://api.groq.com/openai/v1/chat/completions".to_string(),
            auth: "Bearer gsk_x".to_string(),
            token: "gsk_x".to_string(),
            redacted: "<GROQ_API_KEY redacted>".to_string(),
            label: "groq",
            model: DEFAULT_GROQ_MODEL.to_string(),
        };
        assert_eq!(c.auth, format!("Bearer {}", c.token));
    }

    #[test]
    fn debug_output_never_carries_the_token() {
        let c = OaiClient {
            agent: ureq::AgentBuilder::new().build(),
            url: "https://llm.onerouter.pro/v1/chat/completions".to_string(),
            auth: "Bearer infron_topsecret".to_string(),
            token: "infron_topsecret".to_string(),
            redacted: "<INFRON_API_KEY redacted>".to_string(),
            label: "infron",
            model: DEFAULT_INFRON_MODEL.to_string(),
        };
        let d = format!("{c:?}");
        assert!(!d.contains("infron_topsecret"), "Debug leaked the token: {d}");
        assert!(!d.contains("Bearer"), "Debug leaked the header: {d}");
    }

    /// The Issue 051 fix, on the real API: a GROQ-ONLY environment (no CF
    /// creds, no infron key) must ARM and SERVE. Before the fix this path
    /// was unreachable — `LlmRouter::from_env` built the paid leg first and
    /// returned its `"CF_API_TOKEN is not set"` refusal, so `kat_drain`
    /// exited 3 without ever touching the free tier.
    ///
    /// Standing pre-flight for T7. Run with `GROQ_API_KEY` exported, CF
    /// creds UNSET, and `INFRON_API_KEY` unset (Issue 106: an armed infron
    /// would make this a groq+infron window, not the strict groq-only one):
    ///
    /// ```sh
    /// cargo test --features cf_workers_ai,groq_first --lib \
    ///   groq::tests::live_router_groq_only -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "live API call — burns free quota; run manually with GROQ_API_KEY set and CF/infron creds unset"]
    // Exact-zero contract: a free-tier generation MUST bill exactly 0.0 CF
    // billing units — an epsilon compare would accept a real charge.
    #[allow(clippy::float_cmp)]
    fn live_router_groq_only() {
        use super::router::{LlmRouter, Provider};
        assert!(
            std::env::var_os("CF_API_TOKEN").is_none(),
            "unset CF_API_TOKEN — this test proves the groq-ONLY path"
        );
        assert!(
            std::env::var_os("INFRON_API_KEY").is_none(),
            "unset INFRON_API_KEY — an armed infron leg would break the strict \
             groq-only claim (Issue 106)"
        );
        let mut router = LlmRouter::from_env(None)
            .expect("a groq-only env must arm the router (the Issue 051 fix)");
        assert!(router.groq_enabled(), "the free leg must be armed");
        assert!(!router.cf_armed(), "the paid leg must be absent");
        // The pricing basis survives an absent CF leg — the ledger row and
        // the usd_avoided estimate both depend on it.
        assert!(
            crate::llm_spend::price_for(router.model_id()).is_some(),
            "the would-have-served model must stay priced: {}",
            router.model_id()
        );
        let gen = router
            .generate("Reply with exactly: OK", 32)
            .expect("the groq-only ladder must serve");
        assert_eq!(gen.provider, Provider::Groq);
        assert_eq!(gen.neurons, 0.0, "groq is free — zero CF billing units");
        assert!(gen.free_fallback.is_none());
        assert_eq!(router.groq_stats.fallbacks, 0, "no CF leg → no fallbacks");
        eprintln!(
            "[groq-only] served by {} · billing-basis model {} · {}+{} tok · {} neurons · {}ms",
            gen.provider.as_str(),
            router.model_id(),
            gen.prompt_tokens,
            gen.completion_tokens,
            gen.neurons,
            gen.latency_ms,
        );
        assert!(
            gen.text.to_uppercase().contains("OK"),
            "reply should contain OK, got: {:?}",
            gen.text
        );
    }
}
