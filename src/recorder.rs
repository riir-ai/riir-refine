//! Standalone self-evolve recorder — memory + learning with graceful oracle
//! degradation (Issue 018).
//!
//! The [`HealPipeline`](crate::self_evolve::HealPipeline) composes the
//! memory + evolution layers with the L1 drafter for snippet-level healing.
//! This module is the OTHER consumer shape: tools that apply fixes through
//! a different path (the `cargo refine --fix` proposers + edit emitter) still
//! want their applied/reverted fixes to feed the same memory + learning —
//! without dragging the whole pipeline along.
//!
//! ## The oracle split (the reason this module exists)
//!
//! - **Memory** needs NO oracle: every applied fix is a trajectory worth
//!   storing, and reverted fixes carry a definitive fail verdict. The
//!   memory half therefore runs in EVERY build (Issue 018 promoted the
//!   layer from `clippy_verify`-gated to default; the modules are
//!   unconditional).
//! - **Learning** (the EMA direction update in
//!   [`LatentSkillEvolution`](crate::evolve::LatentSkillEvolution)) needs a
//!   trustworthy verdict oracle. Without one, verdict-supplying callers
//!   would pollute the evolved directions with unverified guesses — so the
//!   recorder no-ops the EMA half with ONE clear log line when the
//!   `clippy_verify` oracle is unavailable, exactly the degradation shape
//!   of Issue 015's verify-absent path.
//!
//! The split is a policy of THIS recorder, not of the primitive: callers
//! with their own sound verdict path can still drive
//! [`LatentSkillEvolution::record_outcome`] directly.
//!
//! ## Persistence (Issue 018)
//!
//! A CLI process is short-lived — an in-memory store would be permanently
//! starved (the exact problem the promotion fixes). The recorder persists
//! via [`LatentFixMemory::save`]/[`load`](LatentFixMemory::load): the
//! `cargo refine` bin resolves the platform cache path (`RIIR_REFINE_MEMORY`
//! overrides) + merges on load, so trajectories accumulate across runs.
//!
//! ## Opt-out
//!
//! `RIIR_REFINE_SELF_EVOLVE=0` (or `off`/`false`/`no`) makes the recorder
//! fully inert — no recording, no persistence, no log lines — for consumers
//! who want the pure heal loop.

use crate::embedder::{EMBED_DIM, embed_into};
use crate::evolve::LatentSkillEvolution;
use crate::memory::{FixTrajectory, LatentFixMemory};

/// Env var disabling the self-evolve layer entirely (pure-loop opt-out).
pub const ENV_SELF_EVOLVE: &str = "RIIR_REFINE_SELF_EVOLVE";

/// Reason recorded when learning is unavailable in a build without the
/// `clippy_verify` oracle.
const NO_ORACLE: &str = "built without the clippy_verify oracle";

/// Whether an env value selects the pure-loop opt-out.
fn is_opt_out(val: Option<&str>) -> bool {
    match val {
        Some(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "0" | "off" | "false" | "no"
        ),
        None => false,
    }
}

/// Memory + learning recorder with graceful oracle degradation (Issue 018).
///
/// Construct via [`from_env`](Self::from_env) (honors
/// [`ENV_SELF_EVOLVE`](self::ENV_SELF_EVOLVE)) or
/// [`with_opt_out`](Self::with_opt_out) (explicit). See the module docs for
/// the oracle split + persistence contract.
pub struct EvolveRecorder {
    memory: LatentFixMemory,
    evolution: LatentSkillEvolution,
    /// Whether EMA learning is active (oracle present + not opted out).
    learning: bool,
    /// Why learning is off, when it is (surfaced in the one degradation line).
    disabled_reason: Option<&'static str>,
    /// One-line guarantee: the degradation log fires at most once.
    degradation_logged: bool,
    /// Pure-loop opt-out: fully inert (no recording, no persistence, no log).
    opted_out: bool,
    /// Record calls made (applied + outcome) — drives the persist decision.
    recorded: usize,
}

impl Default for EvolveRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl EvolveRecorder {
    /// Fresh recorder with the build's default oracle state: learning is on
    /// iff `clippy_verify` is compiled in.
    #[must_use]
    pub fn new() -> Self {
        let learning = cfg!(feature = "clippy_verify");
        Self {
            memory: LatentFixMemory::default(),
            evolution: LatentSkillEvolution::default(),
            learning,
            disabled_reason: if learning { None } else { Some(NO_ORACLE) },
            degradation_logged: false,
            opted_out: false,
            recorded: 0,
        }
    }

    /// Fresh recorder with an explicit pure-loop opt-out (`true` = inert).
    #[must_use]
    pub fn with_opt_out(opt_out: bool) -> Self {
        let mut r = Self::new();
        if opt_out {
            r.opted_out = true;
            r.learning = false;
            r.disabled_reason = Some("disabled via RIIR_REFINE_SELF_EVOLVE");
        }
        r
    }

    /// Fresh recorder honoring [`ENV_SELF_EVOLVE`](self::ENV_SELF_EVOLVE).
    #[must_use]
    pub fn from_env() -> Self {
        let opt = std::env::var(ENV_SELF_EVOLVE).ok();
        Self::with_opt_out(is_opt_out(opt.as_deref()))
    }

    /// Whether the recorder does anything at all (false = pure-loop opt-out).
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        !self.opted_out
    }

    /// Whether EMA learning is active.
    #[must_use]
    pub fn learning_enabled(&self) -> bool {
        self.learning
    }

    /// Why learning is off, when it is (the one degradation line's reason).
    #[must_use]
    pub fn degradation_reason(&self) -> Option<&'static str> {
        self.disabled_reason
    }

    /// Whether the degradation line has already fired (test hook for the
    /// one-line guarantee).
    #[must_use]
    pub fn degradation_logged(&self) -> bool {
        self.degradation_logged
    }

    /// Record calls made so far (applied + outcome).
    #[must_use]
    pub fn recorded_count(&self) -> usize {
        self.recorded
    }

    /// Disable learning (idempotent). Used by oracle-present builds when the
    /// oracle fails AT RUNTIME (e.g. cargo/clippy unrunnable) — the memory
    /// half keeps working.
    pub fn disable_learning(&mut self, reason: &'static str) {
        if self.learning {
            self.learning = false;
            self.disabled_reason = Some(reason);
        }
    }

    /// Record an APPLIED fix — memory only, no verdict. This is the
    /// oracle-free half: trajectories accumulate in every build.
    pub fn record_applied(&mut self, rule_id: &str, buggy: &str, fixed: &str) {
        if self.opted_out {
            return;
        }
        self.memory.insert(FixTrajectory::new(
            rule_id.to_string(),
            buggy.to_string(),
            fixed.to_string(),
        ));
        self.recorded += 1;
        self.maybe_log_degradation();
    }

    /// Record the applied edits of one reviewed file — the `--fix`
    /// non-verify path. `edits` are `(rule_id, _line, replacement, restore)`
    /// in the review shape; only edits that were actually WRITTEN should be
    /// passed (review-only runs record nothing).
    pub fn record_reviewed(&mut self, edits: &[(String, usize, String, String)]) {
        for (rule_id, _line, replacement, restore) in edits {
            self.record_applied(rule_id, restore, replacement);
        }
    }

    /// Record an outcome: memory stats ALWAYS; the EMA learning only when
    /// enabled (graceful no-op + ONE log line otherwise).
    ///
    /// Callers must only pass `success` from a sound verdict source
    /// (a compile gate's revert, a real clippy re-run, manual review).
    ///
    /// This path carries NO verification context (Issue 021): the outcome
    /// counts toward stats + the Withdrawn streak, but never certifies. The
    /// current CLI call sites are compile-gate reverts (failures) where the
    /// flag is behaviorally irrelevant — a failure withdraws regardless. A
    /// future caller whose success verdict comes from a full validator
    /// (clippy re-run proving resolution) should use
    /// [`record_outcome_verified`](Self::record_outcome_verified) so the
    /// trajectory's evidential tier promotes to Certified.
    pub fn record_outcome(&mut self, rule_id: &str, buggy: &str, fixed: &str, success: bool) {
        self.record_outcome_verified(rule_id, buggy, fixed, success, false);
    }

    /// Record an outcome WITH its verification context (Issue 021).
    ///
    /// `verified: true` only when the verdict came from a full validator —
    /// a verified success certifies the trajectory's
    /// [`EvidenceTier`](crate::memory::EvidenceTier) (the only promotion
    /// path, and the only exit from Withdrawn).
    pub fn record_outcome_verified(
        &mut self,
        rule_id: &str,
        buggy: &str,
        fixed: &str,
        success: bool,
        verified: bool,
    ) {
        self.record_outcome_signal(rule_id, buggy, fixed, success, verified, None);
    }

    /// Issue 105 — record one run's journey credit on the memory bucket
    /// (the memory.json twin of the traj-store `record_journey`). Called
    /// AFTER `record_outcome_verified` for every kept attempt; the outcome
    /// call owns row creation, this only stamps the journey evidence.
    /// `Some(composite_256)` only for TAINTED runs (the run ended with a
    /// masking-stage error present); clean runs bump `journey_runs` alone.
    /// Respects `opted_out` like every other recording path. Never touches
    /// outcome counters or tiers — the credit is an orthogonal evidence
    /// axis consumed at selection time via
    /// [`crate::journey::journey_factor`].
    pub fn memory_record_journey(
        &mut self,
        rule_id: &str,
        fixed: &str,
        taint_composite_256: Option<u32>,
    ) {
        if self.opted_out {
            return;
        }
        self.memory
            .record_journey(rule_id, fixed, taint_composite_256);
    }

    /// Record a FAILURE with its decision-time signal (Issue 047): the
    /// component class is derived where the failure is observed — never
    /// reconstructed post-hoc — and tags the trajectory bucket for the
    /// audit's per-class counts. Returns the derived class so callers can
    /// mirror it into their other durable surfaces (the traj store rows).
    ///
    /// Equivalent to `record_outcome_verified(.., false, false)` + the
    /// `tag_component` call (`verified` is outcome-irrelevant on failure —
    /// only a verified SUCCESS certifies).
    pub fn record_failure_signal(
        &mut self,
        rule_id: &str,
        buggy: &str,
        fixed: &str,
        signal: &crate::component::FailureSignal,
    ) -> crate::component::ComponentClass {
        let class = crate::component::classify(signal);
        self.record_outcome_signal(rule_id, buggy, fixed, false, false, Some(class));
        class
    }

    /// The shared recording core: outcome + optional component attribution.
    fn record_outcome_signal(
        &mut self,
        rule_id: &str,
        buggy: &str,
        fixed: &str,
        success: bool,
        verified: bool,
        component: Option<crate::component::ComponentClass>,
    ) {
        if self.opted_out {
            return;
        }
        // 1. Memory first — the always-on half.
        self.memory.insert(FixTrajectory::new(
            rule_id.to_string(),
            buggy.to_string(),
            fixed.to_string(),
        ));
        self.memory
            .record_outcome(rule_id, fixed, success, verified);
        if let Some(class) = component {
            self.memory.tag_component(rule_id, fixed, class);
        }
        self.recorded += 1;

        // 2. Learning — oracle-gated.
        if self.learning {
            let mut fix_dir = [0.0f32; EMBED_DIM];
            embed_into(rule_id, fixed, &mut fix_dir);
            self.evolution.record_outcome(rule_id, &fix_dir, success);
        } else {
            self.maybe_log_degradation();
        }
    }

    /// Borrow the memory (for retrieval / inspection / seeding).
    #[must_use]
    pub fn memory(&self) -> &LatentFixMemory {
        &self.memory
    }

    /// Mutably borrow the memory (for seeding / loading).
    pub fn memory_mut(&mut self) -> &mut LatentFixMemory {
        &mut self.memory
    }

    /// Borrow the evolution layer (for inspection / testing).
    #[must_use]
    pub fn evolution(&self) -> &LatentSkillEvolution {
        &self.evolution
    }

    /// Best-effort load of persisted trajectories into the memory (missing
    /// file = fresh; corrupt = `Err`, caller warns + continues).
    ///
    /// # Errors
    ///
    /// See [`LatentFixMemory::load`] — `InvalidData` for malformed files,
    /// raw I/O otherwise (`NotFound` is `Ok`).
    pub fn load(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.load_verified(path).map(|_| ())
    }

    /// [`load`](Self::load), reporting the BLAKE3 commitment check (T7.4).
    ///
    /// # Errors
    ///
    /// See [`LatentFixMemory::load_verified`].
    pub fn load_verified(
        &mut self,
        path: &std::path::Path,
    ) -> std::io::Result<crate::memory::CommitmentStatus> {
        self.memory.load_verified(path)
    }

    /// Persist the memory to `path` (v1 JSON envelope).
    ///
    /// # Errors
    ///
    /// See [`LatentFixMemory::save`] — I/O errors + serialization failure.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        self.memory.save(path)
    }

    /// Emit the degradation line at most once per recorder lifetime.
    fn maybe_log_degradation(&mut self) {
        if self.learning || self.degradation_logged || self.opted_out {
            return;
        }
        self.degradation_logged = true;
        if let Some(reason) = self.disabled_reason {
            eprintln!("self-evolve: memory-only (learning disabled: {reason})");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_recorder_learning_matches_build() {
        let r = EvolveRecorder::new();
        assert_eq!(r.learning_enabled(), cfg!(feature = "clippy_verify"));
        assert!(r.is_enabled());
        assert_eq!(r.recorded_count(), 0);
    }

    #[test]
    fn record_applied_accumulates_and_dedups() {
        let mut r = EvolveRecorder::new();
        r.record_applied("needless_return", "return 42;", "42");
        r.record_applied("needless_return", "return 42;", "42"); // identical triple dedups
        r.record_applied(
            "collapsible_if",
            "if a { if b { x(); } }",
            "if a && b { x(); }",
        );
        assert_eq!(r.memory().len(), 2);
        assert_eq!(r.recorded_count(), 3);
    }

    #[test]
    fn record_failure_signal_tags_the_bucket() {
        // Issue 047: the class is derived at record time + tags the bucket
        // for the audit; the returned class is what callers mirror into the
        // traj store rows.
        let mut r = EvolveRecorder::new();
        let class = r.record_failure_signal(
            "needless_return",
            "return x;",
            "x",
            &crate::component::FailureSignal::reverted(crate::component::RevertKind::CompileGate),
        );
        assert_eq!(class, crate::component::ComponentClass::Drafter);
        let t = r
            .memory()
            .iter()
            .find(|t| t.lint_name == "needless_return")
            .expect("bucket");
        assert_eq!(t.component, Some(crate::component::ComponentClass::Drafter));
        assert_eq!(t.fail_count, 1);
        assert_eq!(r.recorded_count(), 1);
    }

    #[test]
    fn record_failure_signal_classifies_even_when_opted_out() {
        // The class is caller-visible regardless of the opt-out — callers
        // mirror it into their own durable surfaces.
        let mut r = EvolveRecorder::with_opt_out(true);
        let class = r.record_failure_signal(
            "collapsible_else_if",
            "else { if c { } }",
            "else if c { }",
            &crate::component::FailureSignal::false_positive(true),
        );
        assert_eq!(class, crate::component::ComponentClass::Gate);
        assert_eq!(r.memory().len(), 0, "opted-out recorder records nothing");
    }

    #[test]
    fn opt_out_parser_accepts_the_documented_values() {
        assert!(is_opt_out(Some("0")));
        assert!(is_opt_out(Some("off")));
        assert!(is_opt_out(Some("FALSE")));
        assert!(is_opt_out(Some(" no ")));
        assert!(!is_opt_out(Some("1")));
        assert!(!is_opt_out(Some("")));
        assert!(!is_opt_out(None));
    }

    #[test]
    fn opt_out_recorder_is_fully_inert() {
        let mut r = EvolveRecorder::with_opt_out(true);
        assert!(!r.is_enabled());
        assert!(!r.learning_enabled());
        r.record_applied("needless_return", "return 42;", "42");
        r.record_outcome("needless_return", "return 42;", "42", true);
        assert_eq!(r.recorded_count(), 0, "opted-out recorder records nothing");
        assert!(r.memory().is_empty());
        assert!(!r.degradation_logged(), "opted-out recorder stays silent");
        // Explicit construction == from_env with the var set (env parsing is
        // tested separately; no global env mutation in parallel tests).
        assert_eq!(
            EvolveRecorder::from_env().is_enabled(),
            !is_opt_out(std::env::var(ENV_SELF_EVOLVE).ok().as_deref())
        );
    }

    #[test]
    fn runtime_oracle_failure_disables_learning_but_not_memory() {
        let mut r = EvolveRecorder::new();
        r.disable_learning("oracle unavailable (cargo failed to run)");
        assert!(!r.learning_enabled());
        if cfg!(feature = "clippy_verify") {
            assert_eq!(
                r.degradation_reason(),
                Some("oracle unavailable (cargo failed to run)")
            );
        } else {
            // Already disabled at construction — the build-time reason stands
            // (more accurate than the runtime one).
            assert_eq!(
                r.degradation_reason(),
                Some("built without the clippy_verify oracle")
            );
        }
        r.record_applied("needless_return", "return 42;", "42");
        assert_eq!(
            r.memory().len(),
            1,
            "memory must keep working after a runtime disable"
        );
    }

    #[test]
    fn persistence_accumulates_across_recorder_lifetimes() {
        let dir = std::env::temp_dir().join(format!(
            "riir-refine-recorder-{}-persist",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("memory.json");
        let _ = std::fs::remove_file(&path);

        // Run 1: record + persist.
        let mut r1 = EvolveRecorder::new();
        r1.load(&path).expect("missing file is Ok");
        r1.record_applied("needless_return", "return 42;", "42");
        r1.save(&path).expect("save run 1");

        // Run 2: load + record more + persist.
        let mut r2 = EvolveRecorder::new();
        r2.load(&path).expect("load run 2");
        assert_eq!(r2.memory().len(), 1, "run 1 trajectories must survive");
        r2.record_applied(
            "collapsible_if",
            "if a { if b { x(); } }",
            "if a && b { x(); }",
        );
        r2.save(&path).expect("save run 2");

        // Run 3: both accumulated.
        let mut r3 = EvolveRecorder::new();
        r3.load(&path).expect("load run 3");
        assert_eq!(r3.memory().len(), 2, "entries must accumulate across runs");

        let _ = std::fs::remove_file(&path);
    }

    // ── Oracle-absent contract (the Issue 018 acceptance path) ──────────
    // Learning no-ops, memory still works, one log line, zero panics.
    #[cfg(not(feature = "clippy_verify"))]
    #[test]
    fn oracle_absent_learning_noops_memory_works_one_line() {
        let mut r = EvolveRecorder::new();
        assert!(!r.learning_enabled());
        assert_eq!(
            r.degradation_reason(),
            Some("built without the clippy_verify oracle")
        );

        r.record_outcome("needless_return", "return 42;", "42", true);
        // Memory half works.
        assert_eq!(r.memory().len(), 1);
        assert_eq!(r.memory().get(0).unwrap().success_count, 1);
        // Learning half no-ops.
        assert!(
            r.evolution().is_empty(),
            "no EMA direction may evolve without an oracle"
        );
        // The one log line fired exactly once.
        assert!(r.degradation_logged());
        r.record_outcome(
            "collapsible_if",
            "if a { if b { x(); } }",
            "if a && b { x(); }",
            true,
        );
        assert!(r.degradation_logged(), "still one line (flag is sticky)");
    }

    // ── Oracle-present contract (mechanism unchanged, G7 shape) ─────────
    // Mirrors HealPipeline::report_outcome: verdict → memory stats + EMA.
    #[cfg(feature = "clippy_verify")]
    #[test]
    fn oracle_present_learning_records_and_no_degradation_line() {
        let mut r = EvolveRecorder::new();
        assert!(r.learning_enabled());
        r.record_outcome("needless_return", "return 42;", "42", true);
        assert_eq!(r.memory().len(), 1);
        assert_eq!(r.memory().get(0).unwrap().success_count, 1);
        assert!(
            !r.evolution().is_empty(),
            "EMA must evolve under the oracle"
        );
        assert_eq!(r.degradation_reason(), None);
        assert!(!r.degradation_logged());
    }

    // ── The "--fix run accumulates" acceptance (production path) ────────
    // Exercises the exact API the cargo-refine bin calls after a --fix --write
    // pass: proposers → edit emitter → record_reviewed. No feature flags
    // beyond the domains the fix path itself needs.
    #[cfg(feature = "clippy_lints")]
    #[test]
    fn memory_accumulates_after_fix_run() {
        let src = "fn f() -> usize {\n    return 42;\n}\n";
        let parsed = syn::parse_file(src).expect("fixture parses");
        let edits =
            crate::domains::clippy_lints::proposers::collect_edits("needless_return", src, &parsed);
        assert!(!edits.is_empty(), "fixture must trigger needless_return");
        let (fixed, splices) = crate::edit_emitter::apply_edits_traced(
            src,
            edits.iter().map(|re| re.edit.clone()).collect(),
        );
        assert!(fixed.contains("42"), "fix applied");
        assert!(!fixed.contains("return 42"), "return stripped");

        // The bin's recording loop: only edits that SURVIVED selection.
        let applied: std::collections::HashSet<(usize, usize)> =
            splices.iter().map(|s| (s.start, s.end)).collect();
        let mut reviewed: Vec<(String, usize, String, String)> = Vec::new();
        for re in &edits {
            if applied.contains(&(re.edit.start, re.edit.end)) {
                let restore = src
                    .get(re.edit.start..re.edit.end)
                    .map(str::to_string)
                    .unwrap_or_default();
                reviewed.push((re.rule_id.clone(), 1, re.edit.replacement.clone(), restore));
            }
        }
        assert_ne!(reviewed, []);

        let mut rec = EvolveRecorder::new();
        rec.record_reviewed(&reviewed);
        assert!(
            !rec.memory().is_empty(),
            "entries must accumulate after a --fix run"
        );
        let traj = rec.memory().get(0).expect("recorded");
        assert_eq!(traj.lint_name, "needless_return");
        assert_eq!(traj.fixed_code, "42");
    }
}
