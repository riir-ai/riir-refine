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

/// The error-prefix classifier for the thinking-model starvation class —
/// ONE home now (the leaf; issue 124). Both this alias and
/// `cf_workers_ai::REASONING_OVERFLOW` re-export `riir_llm::REASONING_OVERFLOW`,
/// so the old cross-module drift-pin test retired with the duplication.
pub const REASONING_OVERFLOW: &str = riir_llm::REASONING_OVERFLOW;

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
/// shaped, not a hardcoded pair). The `base` is everything BEFORE the
/// route — the leaf ([`riir_llm::Wire`]) owns the
/// `/chat/completions` suffix (issue 124; the URL shape had already
/// diverged once between the dao and clippy copies).
#[derive(Clone, Copy, Debug)]
struct EndpointConf {
    /// Ledger/log token ("groq" / "infron") — error attribution.
    label: &'static str,
    base: &'static str,
    /// The env var holding the API key.
    key_env: &'static str,
    /// The env var holding the optional model override.
    model_env: &'static str,
    default_model: &'static str,
}

const GROQ_CONF: EndpointConf = EndpointConf {
    label: "groq",
    base: "https://api.groq.com/openai/v1",
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
    base: "https://llm.onerouter.pro/v1",
    key_env: "INFRON_API_KEY",
    model_env: "INFRON_MODEL",
    default_model: DEFAULT_INFRON_MODEL,
};

// Overall per-call ceiling and the connect-phase ceiling — the LEAF's
// consts now (issue 124; identical values, one home):
// `riir_llm::transport::CALL_TIMEOUT` (180 s) and
// `riir_llm::transport::CONNECT_TIMEOUT` (10 s, load-bearing
// separately — a blackholed connect must not consume the per-call
// ceiling x every span in the window).

/// One OpenAI-compatible generation (either free leg).
#[derive(Debug, Clone)]
pub struct OaiGen {
    pub text: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// Thinking-token size diagnostic (the leaf's `Usage.reasoning_chars`
    /// — the `message.reasoning` field; 0 when the model doesn't reason
    /// or thinking is suppressed).
    pub reasoning_chars: usize,
    /// Whether the response carried a `usage` object.
    pub usage_reported: bool,
    pub latency_ms: u128,
}

/// Why a free-leg call failed — the leaf's [`riir_llm::LlmFailure`]
/// verbatim (issue 124: one classification, one vocabulary). The router's
/// fallback decision keys on the same four variants; `Auth` is the
/// stop-class the leaf added and this router now honors (a 401 from a
/// free leg is a config bug, surfaced — never silently shifted onto the
/// paid CF leg the way a Transport-classification would).
pub use riir_llm::LlmFailure as OaiError;

/// Minimal blocking OpenAI-compatible chat-completions client — the
/// substrate of BOTH free legs (groq + infron), parameterized by an
/// [`EndpointConf`]. Issue 124: the wire mechanics live in the
/// [`riir_llm`] leaf — the leg (label/base/bearer/model/wire), the POST
/// ([`riir_llm::transport::post_chat_on`]: auth header, both timeouts,
/// bearer scrub, failure classification) and the response parse
/// ([`riir_llm::Leg::parse`]). What remains HERE is env resolution and
/// the [`OaiGen`] projection the router's stats consume. The bearer
/// never appears in errors or Debug output (the leaf's `Leg` Debug
/// hides it; `post_chat_on` scrubs it from server-controlled bodies).
pub struct OaiClient {
    leg: riir_llm::Leg,
    /// One agent per client — connection reuse (keep-alive) across the
    /// window's calls, with the leaf's two timeout ceilings applied.
    agent: riir_llm::transport::Agent,
}

impl std::fmt::Debug for OaiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately no leg internals (the bearer never renders).
        f.debug_struct("OaiClient").finish_non_exhaustive()
    }
}

impl OaiClient {
    /// The model id this client targets (for ledger rows).
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.leg.model
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
            leg: riir_llm::Leg {
                label: conf.label.to_string(),
                base: conf.base.to_string(),
                bearer: token,
                model,
                wire: riir_llm::Wire::OpenAi,
            },
            agent: riir_llm::transport::agent(),
        })
    }

    /// One generation. Temperature 0 for reproducibility (the leaf's body
    /// builder). The qwen3 `/no_think` mitigation rides the leaf's
    /// [`riir_llm::chat_body_for`] (it fires for both legs' qwen3 ids,
    /// `:free` suffixes included).
    ///
    /// # Errors
    ///
    /// [`OaiError`] — the leaf's classified failure, ready for the
    /// router's fallback decision.
    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<OaiGen, OaiError> {
        let t0 = std::time::Instant::now();
        let body = riir_llm::chat_body_for(
            self.leg.wire,
            &self.leg.model,
            prompt,
            u64::try_from(max_tokens).unwrap_or(u64::MAX),
            false,
        );
        let raw = riir_llm::transport::post_chat_on(&self.agent, &self.leg, &body)?;
        let (text, usage) = self.leg.parse(&raw)?;
        Ok(OaiGen {
            text,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            reasoning_chars: usage.reasoning_chars,
            usage_reported: usage.reported,
            latency_ms: t0.elapsed().as_millis(),
        })
    }
}

// ── Provider ladder (requires the CF fallback leg) ─────────────────────────────────

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
        /// Present-but-rejected credentials (HTTP 401/403, the leaf's
        /// stop-class) — issue 124. Distinct from `transport_errors` so a
        /// broken key is readable in the ledger instead of wearing a
        /// transient costume.
        pub auth_errors: u64,
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
                     (the free legs) and CLOUDFLARE_OPS_API_TOKEN / CLOUDFLARE_ACCOUNT_ID are unset (the metered \
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
            std::env::var_os("CLOUDFLARE_OPS_API_TOKEN").is_some_and(|v| !v.is_empty())
                && std::env::var_os("CLOUDFLARE_ACCOUNT_ID").is_some_and(|v| !v.is_empty())
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
            let cf = if Self::arm_plan(groq.is_some(), infron.is_some(), Self::cf_creds_present())?
            {
                Some(CfClient::from_env(cf_model)?)
            } else {
                eprintln!(
                    "[router] CF fallback NOT armed (CLOUDFLARE_OPS_API_TOKEN / CLOUDFLARE_ACCOUNT_ID unset) \
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
                // Stop-class (the leaf rule, issue 124): a credential that
                // is PRESENT but rejected is a config bug — falling through
                // would hide it behind the free tier and silently shift the
                // window onto the paid leg. Surfaced verbatim (labeled);
                // the window's spans decline loudly until the env is fixed.
                Err(OaiError::Auth(msg)) => {
                    stats.auth_errors += 1;
                    Err(format!("{label}: {msg}"))
                }
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
        /// are re-emitted with the same `reasoning-overflow` prefix.
        /// Free-leg auth rejections (401/403 — the leaf's stop-class,
        /// issue 124) surface labeled and hard: a present-but-rejected
        /// credential is a config bug, never silently degraded onto the
        /// paid leg. On a free-only window with every free leg down, the
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
                let why = notice.unwrap_or_else(|| "free legs unavailable".to_string());
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
                         (CLOUDFLARE_OPS_API_TOKEN / CLOUDFLARE_ACCOUNT_ID unset)",
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
            "auth_errors": stats.auth_errors,
            "fallbacks_to_cf": stats.fallbacks,
            "disabled_reason": serde_json::Value::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── leaf wiring (issue 124) ─────────────────────────────────────
    //
    // The body builder, response parser, classification, timeouts and
    // bearer hygiene moved to the riir-llm leaf and are tested THERE
    // (26 leaf tests). What stays pinned HERE is the wiring: this
    // module's client must ride the leaf with THIS module's confs —
    // base URLs without the route suffix (the leaf appends
    // /chat/completions) and the OpenAi wire.

    #[test]
    fn free_leg_confs_carry_base_urls_and_the_openai_wire() {
        // The route suffix belongs to the leaf's Wire::url — a full URL
        // in a conf would double it (the dao smoke's doubled-segment
        // class). Assert the INVARIANT, not the exact host strings.
        for (conf, host) in [
            (GROQ_CONF, "api.groq.com"),
            (INFRON_CONF, "llm.onerouter.pro"),
        ] {
            assert!(
                !conf.base.ends_with("/chat/completions"),
                "{conf:?} carries the leaf's route"
            );
            assert!(conf.base.contains(host));
        }
        let groq = OaiClient {
            leg: riir_llm::Leg {
                label: "groq".to_string(),
                base: GROQ_CONF.base.to_string(),
                bearer: "k".to_string(),
                model: DEFAULT_GROQ_MODEL.to_string(),
                wire: riir_llm::Wire::OpenAi,
            },
            agent: riir_llm::transport::agent(),
        };
        assert_eq!(
            groq.leg.url(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
        // The qwen3 mitigation rides the leaf's body builder (asserted
        // there); pin one integration point: the client's model feeds it.
        let body = riir_llm::chat_body(&groq.leg.model, "fix this", 512);
        assert!(body.contains("fix this /no_think"), "{body}");
        let infron_body = riir_llm::chat_body(DEFAULT_INFRON_MODEL, "fix this", 512);
        assert!(infron_body.contains("fix this /no_think"), "{infron_body}");
    }

    #[test]
    fn oai_error_is_the_leaf_failure_vocabulary() {
        // The alias must stay an alias — a hand-rolled enum here would
        // re-open the classification drift the leaf closed.
        assert_eq!(
            std::mem::discriminant(&OaiError::RateLimited(String::new())),
            std::mem::discriminant(&riir_llm::LlmFailure::RateLimited(String::new()))
        );
        assert!(matches!(
            OaiError::Auth(String::new()),
            riir_llm::LlmFailure::Auth(_)
        ));
    }

    // ── router (both-features) ────────────────────────────────────────────

    #[cfg(feature = "cf_workers_ai")]
    mod router_tests {
        use super::super::router::{
            avoided_usd, providers_json, FreeTierStats, LlmRouter, FREE_TIER_EXHAUSTED,
        };

        // ── prod-grade hardening (T10) ────────────────────────────────
        // ── prod-grade hardening (T10) ────────────────

        #[test]
        fn connect_timeout_is_bounded_well_under_the_call_timeout() {
            // The whole point: a dead network must not consume the per-call
            // ceiling. The consts live in the leaf now (issue 124) — the
            // invariant is asserted there too; this keeps it pinned from
            // the consumer side.
            let (call, connect) = (
                riir_llm::transport::CALL_TIMEOUT,
                riir_llm::transport::CONNECT_TIMEOUT,
            );
            assert!(
                connect < call,
                "connect must fail faster than a slow model completes"
            );
            assert!(
                connect.as_secs() <= 15,
                "connecting is fast or broken; nothing in between"
            );
            // A full window must not be able to hang for hours on connects
            // alone: 25 spans × the connect ceiling stays inside ~5 min.
            assert!(connect.as_secs() * 25 <= 300);
        }

        #[test]
        fn auth_errors_are_counted_and_distinct_from_transport() {
            // Issue 124: the leaf's Auth stop-class is counted per leg so a
            // broken key reads as itself in the ledger block (calls > 0,
            // ok == 0, auth_errors == calls, transport_errors == 0 is the
            // broken-key signature; previously it wore the transient
            // costume and silently shifted the window onto the paid leg).
            let stats = FreeTierStats {
                calls: 4,
                auth_errors: 4,
                ..FreeTierStats::default()
            };
            let v = providers_json(&stats, Some("m"), 0.0, &FreeTierStats::default(), None, 0.0);
            assert_eq!(v["groq"]["auth_errors"], serde_json::json!(4));
            assert_eq!(v["groq"]["transport_errors"], serde_json::json!(0));
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
            assert!(
                e.contains("CLOUDFLARE_OPS_API_TOKEN"),
                "names the paid leg: {e}"
            );
            assert!(
                e.contains("CLOUDFLARE_ACCOUNT_ID"),
                "names the paid leg: {e}"
            );
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
                auth_errors: 0,
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
                assert_eq!(b.get("calls").and_then(serde_json::Value::as_u64), Some(0));
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
            "[groq-smoke] model={} text={:?} usage={}/{} tok (reported={}) reasoning={} {}ms",
            client.model_id(),
            gen.text.trim(),
            gen.prompt_tokens,
            gen.completion_tokens,
            gen.usage_reported,
            gen.reasoning_chars,
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
    // Issue 124: the token-scrub / auth-header / Debug-hygiene tests moved
    // with the code — the leaf's transport owns those paths and tests them
    // (`scrub_bearer_redacts_and_keeps_the_clean_path`,
    // `debug_never_carries_the_bearer`).

    /// The Issue 051 fix, on the real API: a GROQ-ONLY environment (no CF
    /// creds, no infron key) must ARM and SERVE. Before the fix this path
    /// was unreachable — `LlmRouter::from_env` built the paid leg first and
    /// returned its `"CLOUDFLARE_OPS_API_TOKEN is not set"` refusal, so `kat_drain`
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
    // Needs the router + llm_spend — both cf_workers_ai-gated. The gate
    // was missing while nothing compiled groq_first ALONE; issue 124 made
    // that posture real (the leaf-transport feature) and rustc caught it.
    #[cfg(feature = "cf_workers_ai")]
    #[test]
    #[ignore = "live API call — burns free quota; run manually with GROQ_API_KEY set and CF/infron creds unset"]
    // Exact-zero contract: a free-tier generation MUST bill exactly 0.0 CF
    // billing units — an epsilon compare would accept a real charge.
    #[allow(clippy::float_cmp)]
    fn live_router_groq_only() {
        use super::router::{LlmRouter, Provider};
        assert!(
            std::env::var_os("CLOUDFLARE_OPS_API_TOKEN").is_none(),
            "unset CLOUDFLARE_OPS_API_TOKEN — this test proves the groq-ONLY path"
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
