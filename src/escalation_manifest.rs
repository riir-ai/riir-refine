//! Plan 202 R1 (refine half) — the escalation manifest: the operator-owned
//! ARMING surface for per-domain escalation, consuming the shared
//! katgpt-core `escalation_guard` primitive (Issue 923).
//!
//! The ESC law this manifest inherits (Plan 202 §2):
//!
//! - **Manifest-only arming**: no code flip ever arms escalation — a row
//!   in `.refine/escalation_manifest.json` is the ONLY arming surface, so
//!   arming is a file edit, never a release. The manifest SHIPS ALL-EMPTY
//!   (absent file = empty set = zero behavior — the `active_set` law).
//! - **Demote-only switches**: the global env kill switch
//!   `RIIR_REFINE_ESC=0` (the EXACT literal — unset/`1`/junk all leave
//!   rows armed; decoded by `katgpt_core::escalation_guard::
//!   demote_only_decode`) and the per-row `"demoted": true` field. Both
//!   can only DEMOTE; re-arming is a manifest edit.
//! - **`max_rate` is the ONLY rate knob** (the cost ceiling): refine's
//!   escalation rate is its L0–L3 MISS rate, coverage-decided — LOW is
//!   good, never a demotion trigger. `rate_below_min` does not exist
//!   here; the latch is the cost-ceiling shape. The low-volume bound
//!   (the 200-decision window rarely filling at v1 traffic) is covered
//!   from the FIRST request by the `llm_spend` daily cap on the R2
//!   transport — never by this manifest.
//! - **The transports are registered here, never in code** (R2's
//!   `HostedExpertL4` beside the `DaemonL4` precedent; R3a's seat
//!   rerank): a row names its `target`, and only a target kind with a
//!   shipped transport can ever be armed.
//!
//! v1 grammar (UNFROZEN — Plan 202 R7 freezes the contract version only
//! after the R2+R3 dogfood spike; unknown unknowns are expected to move
//! fields, which is why the ledger break is spent late):
//!
//! ```json
//! {
//!   "domains": {
//!     "clippy": {
//!       "modelless_cap": 0.0,
//!       "max_rate": 0.05,
//!       "target": { "kind": "hosted_expert", "endpoint": "…" },
//!       "demoted": false
//!     }
//!   }
//! }
//! ```
//!
//! `modelless_cap` is the worthiness gate's floor: a modelless decision
//! scoring at/above the cap STANDS (never escalates); below it is a MISS
//! (escalation-eligible) and the MISS feeds the latch's rolling window —
//! the domain's temperature (Plan 202 §3). `0.0` = every modelless fix
//! stands (no escalation pressure) — the ALL-EMPTY posture's value.
//!
//! Domain keys are the exact [`crate::draft::DomainId::label`] spellings
//! (`"clippy"`, `"perf"`, `"kernel"`, `"sec"`, `"docker"`, `"dist"`,
//! `"shader"`).

use std::collections::BTreeMap;
use std::path::Path;

use katgpt_core::escalation_guard::{demote_only_decode, RollingRateLatch, DEFAULT_WINDOW};

/// The domain keys this manifest accepts — the exact
/// `crate::draft::DomainId::label` spellings. Mirrored locally so the
/// manifest stays a LIGHT feature (the draft router lives under
/// `latent_retrieval`, a heavy tree this module must not pull); the
/// mirror is cross-pinned by a test whenever both features are on, the
/// same defense as the tpr kill-switch truth-table pin.
pub const DOMAIN_LABELS: &[&str] = &[
    "clippy", "perf", "kernel", "sec", "docker", "dist", "shader",
];

/// The manifest file (beside `active_set.json` — the same operator-owned
/// `.refine/` discipline).
pub const ESCALATION_MANIFEST_FILE: &str = ".refine/escalation_manifest.json";

/// The global demote-only kill switch (the `RIIR_RETHINK_ESC=0` law,
/// refine spelling): `RIIR_REFINE_ESC=0` — the EXACT literal demotes;
/// unset / `1` / `true` / junk all leave armed rows armed.
pub const DEMOTE_ENV: &str = "RIIR_REFINE_ESC";

/// The escalate-target kinds with a shipped (or planned, Plan 202 R2/R3)
/// transport. Registration is data — adding a kind here is a manifest
/// grammar change, never a code path in the heal pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    /// The `DaemonL4` precedent — a long-lived hosted process under the
    /// existing `L4Fallback` seam (the generate half, R2/R3b).
    L4Daemon,
    /// R2's `HostedExpertL4` — one more transport under the same seam
    /// (the generate half for no-candidate misses).
    HostedExpert,
    /// R3a's rerank on the instinct `Ext` seat — the L0–L3 candidates
    /// become the options; the hosted expert picks or abstains.
    SeatRerank,
}

impl TargetKind {
    /// The wire spelling.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::L4Daemon => "l4_daemon",
            Self::HostedExpert => "hosted_expert",
            Self::SeatRerank => "seat_rerank",
        }
    }

    /// Parse the wire spelling (unknown kinds are REFUSED at load, not
    /// silently ignored — an armed row pointing at a typo'd transport
    /// must never look armed).
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "l4_daemon" => Some(Self::L4Daemon),
            "hosted_expert" => Some(Self::HostedExpert),
            "seat_rerank" => Some(Self::SeatRerank),
            _ => None,
        }
    }
}

/// Where a domain's escalation goes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EscalateTarget {
    /// The transport kind (see [`TargetKind`]).
    pub kind: String,
    /// The transport's address (daemon socket / seat URL / OpenAI-compatible
    /// base) — opaque to this manifest, owned by the transport.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// The rerank arm's lower pool bound (Bench 107's law: a rerank with
    /// nothing to choose between is a paid socket for a non-choice — the
    /// corpus is 1-fix-per-rule everywhere, so fresh installs never have
    /// a rerank-worthy pool). Absent → the frozen default 2; a rerank
    /// row may raise it, never lower it, and the seat's presentation cap
    /// (instinct S8) bounds it above. Meaningless on generate rows —
    /// refused there (a dead knob on an armed row is the conflation this
    /// manifest keeps refusing).
    #[serde(default)]
    pub min_pool: Option<usize>,
    /// The served expert's model id — REQUIRED on `hosted_expert` rows
    /// (R2: the cost guard refuses an unpriced model, so the row must name
    /// it; no default, no env fallback). A dead knob on `l4_daemon` rows
    /// (the daemon carries its own checkpoint) — refused there.
    #[serde(default)]
    pub model: Option<String>,
}

impl EscalateTarget {
    /// The pool bound the dispatch must honor: the row's value, else the
    /// frozen default (`2` — the R7 freeze; the wire never carries a
    /// 1-option rerank from our first customer).
    #[must_use]
    pub fn min_pool_or_default(&self) -> usize {
        self.min_pool.unwrap_or(DEFAULT_MIN_POOL)
    }
}

/// The frozen rerank min-pool default (Plan 202 R7 / Bench 107).
pub const DEFAULT_MIN_POOL: usize = 2;

/// The seat's presentation cap (instinct S8): the pool is presented
/// truncated to this many options. The manifest bounds `min_pool` above 1,
/// but the POOL itself has no manifest bound — the truncation happens at
/// the presentation boundary, and the pool length the measurement records
/// stays the caller's.
///
/// Lives on the MANIFEST module (not the dispatch) so the manifest-alone
/// posture — the Issue-195 placebo arm, which must pick from the pool the
/// seat actually SAW — can read it without a transport feature. The
/// dispatch re-exports it; every existing path is unchanged.
pub const PRESENTATION_CAP: usize = 8;

/// One domain's escalation row.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EscalationRow {
    /// The worthiness gate's floor: a modelless decision scoring
    /// at/above the cap STANDS; below it is a MISS (escalation-eligible)
    /// and feeds the latch's window. `0.0` = every fix stands.
    pub modelless_cap: f64,
    /// The cost ceiling — the ONLY rate knob (the miss rate is
    /// coverage-decided, low is good; `rate_below_min` does not exist).
    /// The latch demotes (sticky) above it.
    pub max_rate: f64,
    /// Where escalation goes (registered transport, never code).
    pub target: EscalateTarget,
    /// Per-row demote-only switch — a manifest edit, the only re-arm.
    #[serde(default)]
    pub demoted: bool,
}

/// The loaded manifest (ALL-EMPTY at ship: absent file, zero rows).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EscalationManifest {
    /// Domain label → row. [`BTreeMap`] for deterministic serialization
    /// (round-trip stability is a test).
    #[serde(default)]
    pub domains: BTreeMap<String, EscalationRow>,
}

impl EscalationManifest {
    /// Load from `path`. An ABSENT file is the ALL-EMPTY manifest, not an
    /// error (the `active_set` law: the default posture owns nothing). Any
    /// OTHER read error, or a present-but-invalid file, is a loud refusal —
    /// an armed-looking manifest that silently loads empty is the failure
    /// this prevents.
    ///
    /// # Errors
    ///
    /// Any read error other than not-found, a JSON parse failure, or the
    /// first [`Self::validate`] refusal — each a loud refusal naming the
    /// path and the cause.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = match std::fs::read_to_string(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => {
                return Err(format!("escalation manifest {}: {e}", path.display()));
            }
        };
        let manifest: Self = serde_json::from_str(&bytes)
            .map_err(|e| format!("escalation manifest {}: {e}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Field-scoped validation — everything decodable without the
    /// transports.
    ///
    /// # Errors
    ///
    /// The first offending row, named `escalation.<domain>.<field>` with
    /// the grammar in the message.
    pub fn validate(&self) -> Result<(), String> {
        for (label, row) in &self.domains {
            if !DOMAIN_LABELS.contains(&label.as_str()) {
                return Err(format!(
                    "escalation domain {label:?}: not a DomainId label \
                     (clippy/perf/kernel/sec/docker/dist/shader)"
                ));
            }
            if !row.modelless_cap.is_finite() || !(0.0..=1.0).contains(&row.modelless_cap) {
                return Err(format!(
                    "escalation.{label}.modelless_cap: {} must be finite in [0, 1]",
                    row.modelless_cap
                ));
            }
            if row.max_rate.is_finite() && 0.0 < row.max_rate && row.max_rate < 1.0 {
                // the cost-ceiling grammar (the shared primitive's shape)
            } else {
                return Err(format!(
                    "escalation.{label}.max_rate: {} must be finite with \
                     0 < max_rate < 1 (the ONLY rate knob — the cost ceiling)",
                    row.max_rate
                ));
            }
            match TargetKind::from_label(&row.target.kind) {
                Some(kind) => match (kind, row.target.min_pool) {
                    // The rerank arm raises the frozen default, never
                    // lowers it; the seat's presentation cap bounds above.
                    (TargetKind::SeatRerank, Some(n)) if !(2..=8).contains(&n) => {
                        return Err(format!(
                            "escalation.{label}.target.min_pool: {n} must be in 2..=8 \
                             (the frozen default is {DEFAULT_MIN_POOL}; the seat's \
                             presentation cap is 8)",
                        ));
                    }
                    // A dead knob on a generate row: refused — an armed
                    // row carrying meaningless fields is the conflation
                    // class this manifest keeps refusing.
                    (TargetKind::L4Daemon | TargetKind::HostedExpert, Some(_)) => {
                        return Err(format!(
                            "escalation.{label}.target.min_pool: only seat_rerank rows \
                             carry a pool bound (the generate arm owns no-candidate \
                             traffic by construction)",
                        ));
                    }
                    _ => {}
                },
                None => {
                    return Err(format!(
                        "escalation.{label}.target.kind: {:?} is not a registered \
                         transport (l4_daemon/hosted_expert/seat_rerank)",
                        row.target.kind
                    ));
                }
            }
            // The model id: required where the transport cannot construct
            // without it, a dead knob everywhere else (the same conflation
            // law as min_pool).
            match TargetKind::from_label(&row.target.kind) {
                Some(TargetKind::HostedExpert) if row.target.model.is_none() => {
                    return Err(format!(
                        "escalation.{label}.target.model: hosted_expert rows must name \
                         the served expert's model id (the cost guard refuses an \
                         unpriced model — R2: no default, no env fallback)"
                    ));
                }
                Some(TargetKind::L4Daemon | TargetKind::SeatRerank)
                    if row.target.model.is_some() =>
                {
                    return Err(format!(
                        "escalation.{label}.target.model: only hosted_expert rows \
                         carry a model id (the daemon carries its own checkpoint; \
                         the seat picks among OUR candidates — nothing to name)"
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The row for a domain label, if one exists.
    #[must_use]
    pub fn row(&self, domain_label: &str) -> Option<&EscalationRow> {
        self.domains.get(domain_label)
    }

    /// Is the domain's escalation ARMED? A row exists, is not demoted,
    /// and the global env kill switch (exact literal) has not fired.
    /// Demote-only in every branch: nothing here can arm what a manifest
    /// edit did not arm.
    #[must_use]
    pub fn is_armed(&self, domain_label: &str) -> bool {
        self.is_armed_with(domain_label, std::env::var(DEMOTE_ENV).ok().as_deref())
    }

    /// The testable composition: `env` is the kill-switch value the
    /// caller read (or None) — decoded by the SHARED primitive, so the
    /// exact-literal table is pinned once at katgpt-core and consumed
    /// here without re-derivation.
    #[must_use]
    pub fn is_armed_with(&self, domain_label: &str, env: Option<&str>) -> bool {
        demote_only_decode(env) && self.row(domain_label).is_some_and(|row| !row.demoted)
    }

    /// Construct the domain's rate guard (the cost-ceiling shape — no
    /// lower bound, `rate_below_min` cannot exist). `None` when not
    /// armed. A FRESH latch per call: the caller (the R2/R3 runtime)
    /// owns the instance and feeds it misses.
    #[must_use]
    pub fn guard(&self, domain_label: &str) -> Option<RollingRateLatch> {
        self.guard_with(domain_label, std::env::var(DEMOTE_ENV).ok().as_deref())
    }

    /// The testable composition of [`Self::guard`] — `env` is the
    /// kill-switch value the caller read (or None).
    #[must_use]
    pub fn guard_with(&self, domain_label: &str, env: Option<&str>) -> Option<RollingRateLatch> {
        if !self.is_armed_with(domain_label, env) {
            return None;
        }
        let row = self
            .row(domain_label)
            .unwrap_or_else(|| unreachable!("is_armed_with checked the row"));
        Some(RollingRateLatch::with_window(
            DEFAULT_WINDOW,
            None,
            row.max_rate,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_json(max_rate: f64) -> String {
        format!(
            "{{\"domains\":{{\"clippy\":{{
                \"modelless_cap\": 0.0,
                \"max_rate\": {max_rate},
                \"target\": {{ \"kind\": \"hosted_expert\", \"endpoint\": \"http://127.0.0.1:9\", \"model\": \"test-model\" }}
            }}}}}}"
        )
    }

    fn parse(s: &str) -> Result<EscalationManifest, String> {
        let m: EscalationManifest = serde_json::from_str(s).map_err(|e| e.to_string())?;
        m.validate()?;
        Ok(m)
    }

    #[test]
    fn absent_file_is_the_all_empty_manifest() {
        let m = EscalationManifest::load(Path::new("/nonexistent/escalation.json"))
            .expect("absent file loads empty");
        assert_eq!(m, EscalationManifest::default());
        assert!(!m.is_armed_with("clippy", None));
        assert!(m.guard("clippy").is_none() || std::env::var(DEMOTE_ENV).is_err());
    }

    #[test]
    fn empty_object_is_empty() {
        let m = parse("{}").expect("empty object");
        assert!(m.domains.is_empty());
    }

    #[test]
    fn valid_row_loads_and_arms() {
        let m = parse(&row_json(0.05)).expect("valid row");
        let row = m.row("clippy").expect("row present");
        assert!((row.max_rate - 0.05).abs() < 1e-12);
        assert!(!row.demoted);
        assert!(m.is_armed_with("clippy", None), "armed absent env");
        assert!(m.is_armed_with("clippy", Some("1")), "junk env stays armed");
        assert!(m.is_armed_with("clippy", Some("true")), "true stays armed");
    }

    #[test]
    fn exact_literal_env_demotes_only() {
        let m = parse(&row_json(0.05)).expect("valid row");
        assert!(
            !m.is_armed_with("clippy", Some("0")),
            "the exact literal demotes"
        );
        assert!(m.is_armed_with("clippy", Some(" 0")), "not the literal");
        assert!(m.is_armed_with("clippy", Some("0.0")), "not the literal");
    }

    #[test]
    fn demoted_row_is_not_armed_but_parses() {
        let m = parse(
            "{\"domains\":{\"perf\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"seat_rerank\"},\"demoted\":true}}}",
        )
        .expect("demoted row parses");
        assert!(!m.is_armed_with("perf", None), "row-level demotion");
        // The kill switch can only demote further, never arm:
        assert!(!m.is_armed_with("perf", Some("1")));
    }

    #[test]
    fn unknown_domain_label_refused() {
        let e =
            parse(&row_json(0.05).replace("clippy", "not_a_domain")).expect_err("unknown domain");
        assert!(e.contains("not a DomainId label"), "{e}");
    }

    #[test]
    fn max_rate_grammar_refused() {
        // JSON-representable out-of-grammar values hit the field validation.
        for bad in [0.0, 1.0, -0.1, 1.5] {
            let e = parse(&row_json(bad)).expect_err("bad max_rate");
            assert!(e.contains("max_rate"), "{e}");
        }
        // NaN cannot ride JSON (serde_json refuses it) — hit it directly.
        let m = EscalationManifest {
            domains: BTreeMap::from([(
                "clippy".to_string(),
                EscalationRow {
                    modelless_cap: 0.0,
                    max_rate: f64::NAN,
                    target: EscalateTarget {
                        kind: "hosted_expert".to_string(),
                        endpoint: None,
                        min_pool: None,
                        model: Some("test-model".to_string()),
                    },
                    demoted: false,
                },
            )]),
        };
        let e = m.validate().expect_err("NaN max_rate");
        assert!(e.contains("max_rate"), "{e}");
    }

    #[test]
    fn modelless_cap_grammar_refused() {
        let e = parse(&row_json(0.05).replace("\"modelless_cap\": 0.0", "\"modelless_cap\": 1.5"))
            .expect_err("bad cap");
        assert!(e.contains("modelless_cap"), "{e}");
    }

    #[test]
    fn unknown_target_kind_refused() {
        let e = parse(&row_json(0.05).replace("hosted_expert", "carrier_pigeon"))
            .expect_err("bad kind");
        assert!(e.contains("registered transport"), "{e}");
    }

    // ── the rerank min-pool gate (Plan 202 R7 / Bench 107) ────────────

    #[test]
    fn min_pool_defaults_to_the_frozen_two_and_overrides_only_upward() {
        // Absent → the frozen default 2 (the wire never carries a
        // 1-option rerank from our first customer).
        let m = parse(
            "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"seat_rerank\",\"endpoint\":\"http://127.0.0.1:8092\"}}}}",
        )
        .expect("seat_rerank row without min_pool");
        assert_eq!(
            m.row("clippy").unwrap().target.min_pool_or_default(),
            DEFAULT_MIN_POOL
        );
        // An explicit value within the grammar resolves as written.
        let m = parse(
            "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"seat_rerank\",\"min_pool\":4}}}}",
        )
        .expect("seat_rerank row with min_pool 4");
        assert_eq!(m.row("clippy").unwrap().target.min_pool_or_default(), 4);
    }

    #[test]
    fn min_pool_out_of_grammar_refused() {
        // Below the law (1 = a paid socket for a non-choice).
        let e = parse(
            "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"seat_rerank\",\"min_pool\":1}}}}",
        )
        .expect_err("min_pool 1");
        assert!(e.contains("min_pool") && e.contains("2..=8"), "{e}");
        // Above the seat's presentation cap (S8 = 8).
        let e = parse(
            "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"seat_rerank\",\"min_pool\":9}}}}",
        )
        .expect_err("min_pool 9");
        assert!(e.contains("min_pool"), "{e}");
    }

    #[test]
    fn min_pool_on_a_generate_row_is_a_dead_knob_and_refused() {
        let e = parse(
            "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"hosted_expert\",\"min_pool\":2}}}}",
        )
        .expect_err("min_pool on hosted_expert");
        assert!(e.contains("only seat_rerank rows"), "{e}");
    }

    // ── the hosted model id (Plan 202 R6 wiring — R2's construction law) ──

    #[test]
    fn hosted_expert_row_requires_a_model_id() {
        let e = parse(
            "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"hosted_expert\",\"endpoint\":\"http://127.0.0.1:9\"}}}}",
        )
        .expect_err("hosted_expert without model");
        assert!(
            e.contains("target.model") && e.contains("hosted_expert"),
            "{e}"
        );
        // With the model the row loads.
        let m = parse(
            "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"hosted_expert\",\"endpoint\":\"http://127.0.0.1:9\",\
             \"model\":\"qwen3.8-27b\"}}}}",
        )
        .expect("hosted_expert with model");
        assert_eq!(
            m.row("clippy").unwrap().target.model.as_deref(),
            Some("qwen3.8-27b")
        );
    }

    #[test]
    fn model_on_a_non_hosted_row_is_a_dead_knob_and_refused() {
        for kind in ["l4_daemon", "seat_rerank"] {
            let e = parse(&format!(
                "{{\"domains\":{{\"clippy\":{{\"modelless_cap\":0.0,\"max_rate\":0.1,\
                     \"target\":{{\"kind\":\"{kind}\",\"model\":\"x\"}}}}}}}}"
            ))
            .expect_err("model on a non-hosted row");
            assert!(e.contains("only hosted_expert rows"), "{kind}: {e}");
        }
    }

    #[test]
    fn guard_is_cost_ceiling_shaped() {
        let m = parse(&row_json(0.05)).expect("valid row");
        let mut g = m
            .guard_with("clippy", None)
            .expect("armed row constructs a guard");
        assert_eq!(
            g.bounds(),
            (None, 0.05),
            "no min — rate_below_min cannot exist"
        );
        assert_eq!(g.window(), DEFAULT_WINDOW);
        // The cost-ceiling law: zero misses (every modelless fix stands)
        // fill the window and NEVER latch — a low rate is good, not a
        // demotion trigger.
        for _ in 0..DEFAULT_WINDOW {
            g.observe(false);
        }
        assert_eq!(g.rate(), Some(0.0));
        assert!(!g.is_latched());
        // And the ceiling works: sustained misses above max_rate latch.
        for _ in 0..DEFAULT_WINDOW {
            g.observe(true);
        }
        assert!(g.is_latched());
    }

    #[test]
    fn guard_refused_when_demoted() {
        let m = parse(
            "{\"domains\":{\"kernel\":{\"modelless_cap\":0.0,\"max_rate\":0.1,\
             \"target\":{\"kind\":\"l4_daemon\"}}}}",
        )
        .expect("valid row");
        assert!(m.guard_with("kernel", None).is_some());
        assert!(m.guard_with("kernel", Some("0")).is_none(), "env demotion");
        assert!(m.guard_with("nothing", None).is_none(), "no row, no guard");
    }

    #[test]
    fn round_trip_is_stable() {
        let m = parse(&row_json(0.05)).expect("valid row");
        let bytes = serde_json::to_string_pretty(&m).expect("serialize");
        let back: EscalationManifest = serde_json::from_str(&bytes).expect("deserialize");
        assert_eq!(m, back);
    }

    /// The mirror-pin: `DOMAIN_LABELS` must equal the draft router's
    /// `DomainId::ALL` labels whenever both features are on (the tpr
    /// truth-table-pin defense — a drift here means the manifest accepts
    /// a domain the router never routes, or rejects one it does).
    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn domain_labels_pin_the_draft_router() {
        let router: Vec<&str> = crate::draft::DomainId::ALL
            .iter()
            .map(|d| d.label())
            .collect();
        assert_eq!(DOMAIN_LABELS, router.as_slice());
    }
}
