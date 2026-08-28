//! Issue 046 — the USD LLM daily-cap ledger (`DEFAULT_LLM_DAILY_CAP_USD = 1.0`).
//!
//! A **circuit breaker, not a budget target** (Proposal 005 §Window gates):
//! every LLM call through [`crate::cf_workers_ai::CfClient`] is priced at the
//! model's published in/out per-M rates against a per-UTC-day accumulator,
//! checked **before the call** (reserve-then-reconcile); at/over cap the call
//! **hard-declines for the rest of the day** — the never-retry-into-burn rule
//! generalized from neurons to dollars. The incident class it kills is
//! measured, not hypothetical: the Issue 040 169-request burn (a schema
//! mismatch billed every request while extraction returned nothing) and the
//! qwen null-at-`finish_reason:"length"` retry loop are both "keep calling
//! past what you meant to spend".
//!
//! ## The reserve-then-reconcile contract
//!
//! - **Reserve before the call**: worst case = `est_in_tokens × in_rate +
//!   max_tokens × out_rate` (thinking tokens bill on the out side — never
//!   visible-only sizing). If `spent + reserved + wanted > cap` →
//!   [`SpendError::CapReached`] — terminal for the rest of the UTC day; the
//!   CALLER must honor it (no built-in retry anywhere).
//! - **Reconcile after the call**: actual usage from the response's `usage`
//!   field releases the reserve and records the actual. If the response
//!   carried no `usage` object, the full reserve is SPENT (never released —
//!   the fail-safe direction is over-counting, never under-counting: the
//!   qwen incident billed 169 requests whose parse produced nothing).
//! - **Unknown model id → refuse the call** ([`SpendError::UnknownModel`]) —
//!   never price a guess at $0.
//!
//! ## Persistence + versions
//!
//! `<root>/.heal/kat_llm_spend.json` (`KAT_LLM_SPEND_FILE` overrides) — a
//! per-UTC-day accumulator, version-stamped (`SPEND_VERSION` +
//! `PRICING_VERSION`, the `kat_meter.json` discipline: a reprice is an
//! explicit versioned delta, never silent). Missing/corrupt/unknown-version →
//! recover to zero + warn (never inflate). The state is saved after reserve
//! AND after reconcile so a crash mid-call keeps the reserve counted.
//!
//! The UTC day is **epoch-days** (`secs / 86_400`) — integer math, zero
//! allocation, and exactly CF's billing-day boundary (resets 00:00 UTC).
//!
//! Free tier note: neurons bind first on the free allocation (10K n/day ≈
//! $0.11 sticker) — the cap is the paid-mode / re-designation backstop. It
//! still runs on the free tier (defense in depth: a runaway loop burns the
//! day's neurons AND trips the cap).
//!
//! G2: the reserve/reconcile path is alloc-free (pinned by
//! `tests/llm_spend_g2_alloc.rs` — the `kat_meter_g2_alloc` convention).

use std::path::{Path, PathBuf};

/// The daily-cap default (Proposal 005 §Window gates, 2026-08-26).
pub const DEFAULT_LLM_DAILY_CAP_USD: f64 = 1.0;

/// Env override for the cap. Parsed once at ledger open; invalid values fall
/// back to the default with a warning (never abort the run). `0` is honored
/// as decline-all (the safe test mode).
pub const ENV_CAP_OVERRIDE: &str = "KAT_LLM_DAILY_CAP_USD";

/// Env override for the ledger file location (tests + multi-root hosts).
pub const ENV_SPEND_FILE: &str = "KAT_LLM_SPEND_FILE";

/// The spend-ledger schema version. Bump when the persisted shape changes.
pub const SPEND_VERSION: u32 = 1;

/// Versioned deltas of the spend-ledger schema (the `kat_meter` changelog
/// discipline — no silent format change).
pub const SPEND_CHANGELOG: &[(u32, &str)] = &[(1, "initial: per-UTC-day USD accumulator (Issue 046)")];

/// The pricing-table version. Bump ONLY when a rate changes (a reprice) and
/// record the delta in [`PRICING_CHANGELOG`] in the same change. Independent
/// of [`SPEND_VERSION`] on purpose: a rate change does not move the file
/// shape, and a shape change does not reprice anything.
pub const PRICING_VERSION: u32 = 3;

/// Versioned deltas of the pricing table (the reprice audit trail).
pub const PRICING_CHANGELOG: &[(u32, &str)] = &[
    (
        1,
        "initial: @cf/qwen/qwen3.8-27b 0.45/3.20 (Proposal 005 pinned); \
         @cf/meta/llama-3.3-70b-instruct-fp8-fast 0.29/2.25 (Bench 049's recorded per-M basis)",
    ),
    (
        2,
        "owner pivot 2026-08-28: @cf/zai-org/glm-4.7-flash 0.06/0.40 added \
         (CF list rates; exact fit of the committed glm arm rows' recorded \
         per-M basis in llm_cost_ledger.jsonl — the Issue 043 glm session); \
         DEFAULT_MODEL pivoted to it; qwen + llama rates unchanged",
    ),
    (
        3,
        "owner pivot 2026-08-28 (second call): DEFAULT_MODEL moved glm-4.7 → \
         glm-5.3-flash (4.7 FAILED the Issue 043 fixer-quality gate 3/9 vs \
         the 5/9 bar; owner: no llama); @cf/zai-org/glm-5.3-flash 0.15/0.50 \
         added at CF's PUBLISHED rates (catalog-verified by the dao advisory \
         lane's Plan 003 catalog work, 2026-08-28); 4.7 stays priced + \
         selectable via RIIR_CF_MODEL",
    ),
];

/// One model's published per-million-token USD rates (input, output).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPrice {
    pub id: &'static str,
    /// USD per 1M input tokens.
    pub input_per_m: f64,
    /// USD per 1M output tokens (thinking tokens bill here — the out side).
    pub output_per_m: f64,
}

/// The pinned pricing table (PRICING_VERSION-stamped). **Unknown model ids
/// are refused, never priced as $0** — an unpriced model is a hard error.
pub const PRICING_TABLE: &[ModelPrice] = &[
    // The DEFAULT_MODEL (owner pivot 2026-08-28, second call: 4.7 failed the
    // Issue 043 fixer-quality gate, llama rejected as too old). Rates = CF's
    // PUBLISHED rates for glm-5.3-flash, catalog-verified by the dao advisory
    // lane (Plan 003, 2026-08-28). Assumption-grade until our own usage rows
    // exact-fit them (the v2 precedent) — the one-call smoke / first gated
    // run stamps the check.
    ModelPrice {
        id: "@cf/zai-org/glm-5.3-flash",
        input_per_m: 0.15,
        output_per_m: 0.5,
    },
    // The Issue 043 gate model: FAILED the fixer-quality bar (3/9 vs 5/9) but
    // stays priced + selectable (RIIR_CF_MODEL). Rates independently
    // exact-fit by the committed glm arm rows' usd_est in
    // llm_cost_ledger.jsonl (551/277, 2859/275, 319/4066 token pairs all
    // land on 0.06 in / 0.40 out per M to the digit).
    ModelPrice {
        id: "@cf/zai-org/glm-4.7-flash",
        input_per_m: 0.06,
        output_per_m: 0.4,
    },
    // Proposal 005 TL;DR (2026-08-26 pivot): $0.450/$3.20 per M.
    ModelPrice {
        id: "@cf/qwen/qwen3.8-27b",
        input_per_m: 0.45,
        output_per_m: 3.2,
    },
    // Bench 049's recorded per-M basis for the measured arm (assumption-grade,
    // version-stamped — correct in one place when CF publishes a firm table).
    ModelPrice {
        id: "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
        input_per_m: 0.29,
        output_per_m: 2.25,
    },
];

/// Look up a model's price. `None` = not in the table → the call is refused.
#[must_use]
pub fn price_for(model_id: &str) -> Option<&'static ModelPrice> {
    PRICING_TABLE.iter().find(|p| p.id == model_id)
}

/// Parse a cap override value (pure half of [`cap_from_env`] — testable
/// without touching the process env). `None` → default. Invalid → default
/// (the caller warns). `0` is honored (decline-all).
#[must_use]
pub fn parse_cap(raw: Option<&str>) -> f64 {
    match raw {
        None => DEFAULT_LLM_DAILY_CAP_USD,
        Some(s) => match s.trim().parse::<f64>() {
            Ok(v) if v.is_finite() && v >= 0.0 => v,
            _ => DEFAULT_LLM_DAILY_CAP_USD,
        },
    }
}

/// The cap in effect: `KAT_LLM_DAILY_CAP_USD` parsed once, invalid → default
/// + warn (never abort the run).
#[must_use]
pub fn cap_from_env() -> f64 {
    match std::env::var(ENV_CAP_OVERRIDE) {
        Ok(v) => {
            let cap = parse_cap(Some(&v));
            if v.trim().parse::<f64>() != Ok(cap) {
                eprintln!(
                    "llm-spend: {ENV_CAP_OVERRIDE}={v:?} is not a non-negative finite number — using ${cap:.2}"
                );
            }
            cap
        }
        Err(_) => DEFAULT_LLM_DAILY_CAP_USD,
    }
}

/// Conservative input-token estimate for a prompt (pre-call — exact token
/// counts need the response's `usage`). Code averages ~4 chars/token; /3
/// systematically OVER-estimates tokens, which is the safe direction for a
/// cap (over-reserve → over-decline, never over-spend). Alloc-free.
#[must_use]
pub fn est_input_tokens(prompt: &str) -> u64 {
    prompt.len() as u64 / 3 + 1
}

/// The worst-case USD cost of one call (the reserve). Pure; `Err` on an
/// unknown model id (never price a guess).
///
/// # Errors
///
/// [`SpendError::UnknownModel`] — the model is not in [`PRICING_TABLE`].
pub fn worst_case_usd(model_id: &str, est_in_tokens: u64, max_tokens: u64) -> Result<f64, SpendError> {
    let p = price_for(model_id).ok_or_else(|| SpendError::UnknownModel(model_id.to_string()))?;
    // Pricing math is a report/bound figure, not a sync value — the u64 → f64
    // precision loss is irrelevant at token counts (< 2^32; the ArmSummary
    // cost-column precedent).
    #[allow(clippy::cast_precision_loss)]
    Ok((est_in_tokens as f64 * p.input_per_m + max_tokens as f64 * p.output_per_m) / 1e6)
}

/// The cap was reached — terminal for the rest of the UTC day. The caller
/// MUST honor this (no retry); it is not an error to surface to a user, it
/// is the circuit breaker doing its job.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapReached {
    /// Reconciled actuals already spent today.
    pub spent: f64,
    /// Outstanding un-reconciled reserves.
    pub reserved: f64,
    /// The worst-case this call wanted.
    pub wanted: f64,
    /// The cap in effect.
    pub cap: f64,
}

impl std::fmt::Display for CapReached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LLM daily cap reached: ${:.4} spent + ${:.4} reserved + ${:.4} wanted > ${:.2} cap — \
             hard-declining for the rest of the UTC day (resets 00:00 UTC; this is a circuit \
             breaker, not an error to retry)",
            self.spent, self.reserved, self.wanted, self.cap
        )
    }
}

/// The spend-gate verdict for a call.
#[derive(Debug, Clone, PartialEq)]
pub enum SpendError {
    /// The daily cap is exhausted — terminal for the rest of the UTC day.
    CapReached(CapReached),
    /// The model id is not in [`PRICING_TABLE`] — refusing to price a guess.
    UnknownModel(String),
}

impl std::fmt::Display for SpendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpendError::CapReached(c) => write!(f, "{c}"),
            SpendError::UnknownModel(m) => write!(
                f,
                "model {m:?} is not in the LLM pricing table (PRICING_VERSION {PRICING_VERSION}) — \
                 refusing to price a guess at $0; add it to PRICING_TABLE + bump PRICING_VERSION"
            ),
        }
    }
}

impl std::error::Error for SpendError {}

/// The persisted ledger state (the per-UTC-day USD accumulator).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpendState {
    /// The ledger schema that produced this file.
    pub version: u32,
    /// The pricing table that priced these amounts (reprice audit stamp).
    pub pricing_version: u32,
    /// The UTC day this accumulator belongs to, as **epoch-days**
    /// (`secs / 86_400`) — integer, alloc-free, exactly CF's billing day.
    pub day: u64,
    /// The cap in effect when this day's accumulator was opened (recorded
    /// for transparency; enforcement reads the in-memory value).
    pub cap_usd: f64,
    /// Reconciled actuals spent today.
    pub spent_usd: f64,
    /// Outstanding un-reconciled reserves (a crash keeps these counted).
    pub reserved_usd: f64,
    /// Calls that passed the gate today.
    pub calls: u64,
    /// Calls the gate declined today (cap + unknown model).
    pub declines: u64,
    /// Cumulative reconciled spend across all days (audit convenience).
    pub all_time_spent_usd: f64,
}

impl Default for SpendState {
    fn default() -> Self {
        Self {
            version: SPEND_VERSION,
            pricing_version: PRICING_VERSION,
            day: today_epoch_days(),
            cap_usd: DEFAULT_LLM_DAILY_CAP_USD,
            spent_usd: 0.0,
            reserved_usd: 0.0,
            calls: 0,
            declines: 0,
            all_time_spent_usd: 0.0,
        }
    }
}

/// Today's UTC day as epoch-days (`secs / 86_400`) — the exact 00:00 UTC
/// billing boundary. Alloc-free.
#[must_use]
pub fn today_epoch_days() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400)
}

/// The reserve-then-reconcile USD daily-cap ledger.
#[derive(Debug, Clone)]
pub struct SpendLedger {
    path: PathBuf,
    state: SpendState,
}

impl SpendLedger {
    /// The ledger file name under the `.heal` root.
    pub const FILE_NAME: &'static str = "kat_llm_spend.json";

    /// Open (or create) at `<root>/.heal/kat_llm_spend.json`, cap from env.
    #[must_use]
    pub fn open(root: &Path) -> SpendLedger {
        Self::open_at(root.join(".heal").join(Self::FILE_NAME))
    }

    /// Open at an explicit path, cap from env (the `KAT_LLM_SPEND_FILE`
    /// override shape + tests).
    #[must_use]
    pub fn open_at(path: PathBuf) -> SpendLedger {
        Self::at_path(path, cap_from_env())
    }

    /// Open at an explicit path with an explicit cap (tests + programmatic
    /// consumers).
    #[must_use]
    pub fn open_at_with_cap(path: PathBuf, cap_usd: f64) -> SpendLedger {
        Self::at_path(path, cap_usd)
    }

    fn at_path(path: PathBuf, cap_usd: f64) -> SpendLedger {
        let mut state = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<SpendState>(&text) {
                Ok(s) if s.version == SPEND_VERSION => s,
                Ok(s) => {
                    eprintln!(
                        "llm-spend: {} carries version {} (this build defines {}); \
                         recovering to zero — see SPEND_CHANGELOG",
                        path.display(),
                        s.version,
                        SPEND_VERSION
                    );
                    SpendState::default()
                }
                Err(e) => {
                    eprintln!(
                        "llm-spend: {} is corrupt ({e}); recovering to zero (never inflate)",
                        path.display()
                    );
                    SpendState::default()
                }
            },
            Err(_) => SpendState::default(),
        };
        // A pricing-table reprice does NOT re-derive past days (the frozen
        // watermark discipline); it only stamps what new calls will use.
        state.pricing_version = PRICING_VERSION;
        state.cap_usd = cap_usd;
        let mut ledger = SpendLedger { path, state };
        ledger.roll_day(today_epoch_days());
        ledger
    }

    /// Roll to `today`: a new UTC day resets the accumulator (spent,
    /// reserves, calls, declines) while all-time + the cap carry. Idempotent.
    fn roll_day(&mut self, today: u64) {
        if self.state.day != today {
            self.state.day = today;
            self.state.spent_usd = 0.0;
            self.state.reserved_usd = 0.0;
            self.state.calls = 0;
            self.state.declines = 0;
        }
    }

    /// The persisted state (day accumulator + version stamps).
    #[must_use]
    pub fn state(&self) -> &SpendState {
        &self.state
    }

    /// Reserve the worst case for one call. `Ok(wanted)` = the call may
    /// proceed (the reserve is outstanding until reconcile/release).
    ///
    /// # Errors
    ///
    /// [`SpendError::CapReached`] — terminal for the rest of the UTC day;
    /// [`SpendError::UnknownModel`] — the model is not priced.
    pub fn reserve(
        &mut self,
        model_id: &str,
        prompt: &str,
        max_tokens: u64,
    ) -> Result<f64, SpendError> {
        self.roll_day(today_epoch_days());
        let wanted = worst_case_usd(model_id, est_input_tokens(prompt), max_tokens)?;
        if self.state.spent_usd + self.state.reserved_usd + wanted > self.state.cap_usd {
            let reached = CapReached {
                spent: self.state.spent_usd,
                reserved: self.state.reserved_usd,
                wanted,
                cap: self.state.cap_usd,
            };
            self.state.declines += 1;
            return Err(SpendError::CapReached(reached));
        }
        self.state.reserved_usd += wanted;
        self.state.calls += 1;
        Ok(wanted)
    }

    /// Reconcile after a completed call: release `reserved`, record the
    /// actual. `usage_reported == false` (or a billed failure) spends the
    /// FULL reserve — the fail-safe direction is over-counting (the qwen
    /// incident billed 169 requests whose parse produced nothing).
    pub fn reconcile(
        &mut self,
        reserved: f64,
        model_id: &str,
        actual_in: u64,
        actual_out: u64,
        usage_reported: bool,
    ) {
        let actual = if usage_reported {
            worst_case_usd(model_id, actual_in, actual_out).unwrap_or(reserved)
        } else {
            reserved
        };
        self.state.reserved_usd = (self.state.reserved_usd - reserved).max(0.0);
        self.state.spent_usd += actual;
        self.state.all_time_spent_usd += actual;
    }

    /// Release a reserve without spending (the request never completed —
    /// transport error before any generation; nothing was billed).
    pub fn release(&mut self, reserved: f64) {
        self.state.reserved_usd = (self.state.reserved_usd - reserved).max(0.0);
    }

    /// Count an externally-declined call (e.g. the neuron bound declined
    /// first) — keeps the declines counter honest when callers gate on more
    /// than USD.
    pub fn note_decline(&mut self) {
        self.state.declines += 1;
    }

    /// Persist the state (small JSON; called after reserve and after
    /// reconcile so a crash mid-call keeps the reserve counted).
    ///
    /// # Errors
    ///
    /// io errors from creating the parent dir / writing the file.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(&self.state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&self.path, text)
    }

    /// The one-line status for run summaries (cold path).
    #[must_use]
    pub fn summary_line(&self) -> String {
        format!(
            "llm-spend: ${:.4} / ${:.2} today ({} call(s) passed, {} declined; ledger v{}, \
             pricing v{}; resets 00:00 UTC)",
            self.state.spent_usd,
            self.state.cap_usd,
            self.state.calls,
            self.state.declines,
            self.state.version,
            self.state.pricing_version
        )
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("llm_spend_{name}_{}", std::process::id()))
    }

    const PROMPT: &str = "fn main() { println!(\"hello, world\"); } // fixed";

    #[test]
    fn pricing_table_covers_the_designated_and_measured_models() {
        let q = price_for("@cf/qwen/qwen3.8-27b").expect("qwen priced");
        assert!((q.input_per_m - 0.45).abs() < 1e-9);
        assert!((q.output_per_m - 3.2).abs() < 1e-9);
        let l = price_for("@cf/meta/llama-3.3-70b-instruct-fp8-fast").expect("llama priced");
        assert!((l.input_per_m - 0.29).abs() < 1e-9);
        assert!((l.output_per_m - 2.25).abs() < 1e-9);
        let g = price_for("@cf/zai-org/glm-4.7-flash").expect("glm-4.7 priced (Issue 043 gate model)");
        assert!((g.input_per_m - 0.06).abs() < 1e-9);
        assert!((g.output_per_m - 0.4).abs() < 1e-9);
        let g5 = price_for("@cf/zai-org/glm-5.3-flash").expect("glm-5.3 priced (the DEFAULT_MODEL)");
        assert!((g5.input_per_m - 0.15).abs() < 1e-9);
        assert!((g5.output_per_m - 0.5).abs() < 1e-9);
        assert!(price_for("@cf/nope/mystery").is_none(), "unknown model must be unpriced");
        // Version stamps are changelog-covered (the kat_meter discipline).
        assert!(SPEND_CHANGELOG.iter().any(|(v, _)| *v == SPEND_VERSION));
        assert!(PRICING_CHANGELOG.iter().any(|(v, _)| *v == PRICING_VERSION));
    }

    #[test]
    fn parse_cap_none_default_zero_honored_invalid_falls_back() {
        assert!((parse_cap(None) - DEFAULT_LLM_DAILY_CAP_USD).abs() < 1e-12);
        assert!((parse_cap(Some("0.5")) - 0.5).abs() < 1e-12);
        assert!(parse_cap(Some("0")).abs() < 1e-12, "0 = decline-all (test mode)");
        assert!((parse_cap(Some("2.75")) - 2.75).abs() < 1e-12);
        assert!(
            (parse_cap(Some("banana")) - DEFAULT_LLM_DAILY_CAP_USD).abs() < 1e-12,
            "invalid falls back to the default, never aborts"
        );
        assert!(
            (parse_cap(Some("-1")) - DEFAULT_LLM_DAILY_CAP_USD).abs() < 1e-12,
            "negative is invalid (a negative cap would allow unbounded spend)"
        );
    }

    #[test]
    fn worst_case_prices_both_sides_and_refuses_unknown_models() {
        // 1k in + 128 out at qwen rates = 1000*0.45/1e6 + 128*3.2/1e6.
        let got = worst_case_usd("@cf/qwen/qwen3.8-27b", 1_000, 128).unwrap();
        let want = (1_000.0 * 0.45 + 128.0 * 3.2) / 1e6;
        assert!((got - want).abs() < 1e-12);
        assert!(
            matches!(
                worst_case_usd("@cf/nope/mystery", 1, 1),
                Err(SpendError::UnknownModel(_))
            ),
            "never price a guess"
        );
        // Thinking tokens bill on the out side: max_tokens IS the out bound.
        let a = worst_case_usd("@cf/qwen/qwen3.8-27b", 0, 512).unwrap();
        let b = worst_case_usd("@cf/qwen/qwen3.8-27b", 0, 64).unwrap();
        assert!(a > b * 7.0, "max_tokens dominates the reserve (out rate is steep)");
    }

    #[test]
    fn est_input_tokens_is_conservative_for_code() {
        // ~4 chars/token typical for code → /3 over-estimates (safe side).
        let est = est_input_tokens("fn main() { let x = 1; }");
        assert!(est >= 8, "estimate must not under-count (got {est})");
        assert_eq!(est_input_tokens(""), 1);
    }

    #[test]
    fn reserve_then_reconcile_partial_release() {
        let file = tmp_file("reconcile");
        let mut l = SpendLedger::open_at_with_cap(file.clone(), 1.0);
        let reserved = l.reserve("@cf/qwen/qwen3.8-27b", PROMPT, 128).unwrap();
        assert!(reserved > 0.0);
        assert!((l.state().reserved_usd - reserved).abs() < 1e-12);
        assert_eq!(l.state().calls, 1);
        // Actuals come in UNDER the reserve → partial release.
        l.reconcile(reserved, "@cf/qwen/qwen3.8-27b", 60, 40, true);
        let expected = (60.0 * 0.45 + 40.0 * 3.2) / 1e6;
        assert!((l.state().spent_usd - expected).abs() < 1e-12, "actual recorded");
        assert!(l.state().reserved_usd.abs() < 1e-12, "reserve fully released");
        assert!((l.state().all_time_spent_usd - expected).abs() < 1e-12);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn reconcile_without_usage_spends_the_full_reserve() {
        let file = tmp_file("no_usage");
        let mut l = SpendLedger::open_at_with_cap(file, 1.0);
        let reserved = l.reserve("@cf/meta/llama-3.3-70b-instruct-fp8-fast", PROMPT, 256).unwrap();
        l.reconcile(reserved, "@cf/meta/llama-3.3-70b-instruct-fp8-fast", 0, 0, false);
        assert!((l.state().spent_usd - reserved).abs() < 1e-12, "no usage → worst case spent");
        let _ = std::fs::remove_file(tmp_file("no_usage"));
    }

    #[test]
    fn release_clears_the_reserve_without_spending() {
        let file = tmp_file("release");
        let mut l = SpendLedger::open_at_with_cap(file, 1.0);
        let reserved = l.reserve("@cf/qwen/qwen3.8-27b", PROMPT, 128).unwrap();
        l.release(reserved);
        assert!(l.state().reserved_usd.abs() < 1e-12);
        assert!(l.state().spent_usd.abs() < 1e-12, "transport failure billed nothing");
        let _ = std::fs::remove_file(tmp_file("release"));
    }

    #[test]
    fn cap_reached_declines_and_counts() {
        let file = tmp_file("cap");
        let mut l = SpendLedger::open_at_with_cap(file, 0.0005);
        // A prompt big enough that the worst case exceeds the tiny cap.
        let big = "x".repeat(3_000);
        let err = l.reserve("@cf/qwen/qwen3.8-27b", &big, 512).unwrap_err();
        assert!(matches!(err, SpendError::CapReached(_)));
        assert_eq!(l.state().declines, 1);
        assert_eq!(l.state().calls, 0);
        // The error is self-describing (the caller must honor, not retry).
        let msg = err.to_string();
        assert!(msg.contains("cap"), "message names the cap: {msg}");
        let _ = std::fs::remove_file(tmp_file("cap"));
    }

    #[test]
    fn zero_cap_declines_everything() {
        let file = tmp_file("zero_cap");
        let mut l = SpendLedger::open_at_with_cap(file, 0.0);
        assert!(l.reserve("@cf/qwen/qwen3.8-27b", "hi", 1).is_err());
        let _ = std::fs::remove_file(tmp_file("zero_cap"));
    }

    #[test]
    fn unknown_model_refuses_even_under_an_empty_cap() {
        let file = tmp_file("unknown_model");
        let mut l = SpendLedger::open_at_with_cap(file, 100.0);
        assert!(matches!(
            l.reserve("@cf/nope/mystery", "hi", 1),
            Err(SpendError::UnknownModel(_))
        ));
        let _ = std::fs::remove_file(tmp_file("unknown_model"));
    }

    #[test]
    fn day_rollover_resets_the_accumulator() {
        let file = tmp_file("rollover");
        let mut l = SpendLedger::open_at_with_cap(file.clone(), 1.0);
        let reserved = l.reserve("@cf/qwen/qwen3.8-27b", PROMPT, 128).unwrap();
        l.reconcile(reserved, "@cf/qwen/qwen3.8-27b", 60, 40, true);
        assert!(l.state().spent_usd > 0.0);
        let all_time = l.state().all_time_spent_usd;
        // A stale day rolls: reserve() checks the day first.
        let mut stale = l.state().clone();
        stale.day = 0; // 1970-01-01 — provably not today
        std::fs::write(&file, serde_json::to_string(&stale).unwrap()).unwrap();
        let fresh = SpendLedger::open_at_with_cap(file.clone(), 1.0);
        assert!(fresh.state().spent_usd.abs() < 1e-12, "fresh day resets spent");
        assert_eq!(fresh.state().calls, 0);
        assert!(
            (fresh.state().all_time_spent_usd - all_time).abs() < 1e-12,
            "all-time carries across days"
        );
        // And a call passes on the fresh day.
        assert!(fresh.state().spent_usd + fresh.state().reserved_usd < 1.0);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn persistence_round_trip_is_lossless_and_stamped() {
        let file = tmp_file("roundtrip");
        let mut l = SpendLedger::open_at_with_cap(file.clone(), 0.05);
        let reserved = l.reserve("@cf/qwen/qwen3.8-27b", PROMPT, 128).unwrap();
        l.reconcile(reserved, "@cf/qwen/qwen3.8-27b", 60, 40, true);
        l.save().unwrap();
        let reopened = SpendLedger::open_at_with_cap(file.clone(), 0.05);
        assert_eq!(reopened.state(), l.state());
        assert_eq!(reopened.state().version, SPEND_VERSION);
        assert_eq!(reopened.state().pricing_version, PRICING_VERSION);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn corrupt_ledger_recovers_to_zero() {
        let file = tmp_file("corrupt");
        std::fs::write(&file, "definitely not json").unwrap();
        let l = SpendLedger::open_at_with_cap(file.clone(), 1.0);
        assert!(l.state().spent_usd.abs() < 1e-12);
        assert_eq!(l.state().version, SPEND_VERSION, "recovered with the current stamp");
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn determinism_same_inputs_same_file_bytes() {
        let mut states = Vec::new();
        for i in 0..3 {
            let file = tmp_file(&format!("det{i}"));
            let mut l = SpendLedger::open_at_with_cap(file, 1.0);
            let r1 = l.reserve("@cf/qwen/qwen3.8-27b", PROMPT, 128).unwrap();
            l.reconcile(r1, "@cf/qwen/qwen3.8-27b", 60, 40, true);
            let r2 = l.reserve("@cf/meta/llama-3.3-70b-instruct-fp8-fast", PROMPT, 64).unwrap();
            l.reconcile(r2, "@cf/meta/llama-3.3-70b-instruct-fp8-fast", 60, 10, true);
            states.push(serde_json::to_string(&l.state()).unwrap());
        }
        assert_eq!(states[0], states[1]);
        assert_eq!(states[1], states[2]);
    }

    #[test]
    fn summary_line_names_the_cap_and_the_reset_boundary() {
        let file = tmp_file("summary");
        let l = SpendLedger::open_at_with_cap(file.clone(), 1.0);
        let line = l.summary_line();
        assert!(line.contains("$1.00"), "carries the cap: {line}");
        assert!(line.contains("00:00 UTC"));
        let _ = std::fs::remove_file(file);
    }
}
