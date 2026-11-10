//! `corpus_binding` — the Research-649 fusion record artifact (issue 206): the
//! per-rule drive × gate-stage × failure-class binding, BLAKE3-digested and
//! corpus-epoch-keyed, stale-refusing.
//!
//! Posture (honest): this is the test-side `PoC` of issue 206's tasks 1+2 — the
//! diagnose predicate, the record, and the stale refusal over synthetic
//! outcome fixtures. The real-telemetry wiring, the arm-A `context_notes`
//! writer, and the score-bench A/B referee are the corpus half (deferred —
//! the sibling batch owns the corpus entries files). Nothing here mutates the
//! corpus and no production caller exists yet; the module is inert until the
//! corpus half wires it.
//!
//! Determinism law: same telemetry in → byte-identical records out (sequential
//! folds, first-index stage ties, sorted output; `recorded_at` rides as
//! un-hashed bookkeeping so a re-run at a later time reproduces the digest).

use blake3::Hasher;

/// The L2 pipeline stages that can fail systematically, in first-index tie
/// order: when several stages clear the bar, the EARLIEST in this order is
/// the recorded gate stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GateStage {
    /// Drafts that failed the L2 `syn` parse.
    SynParse = 0,
    /// Drafts the compile gate rejected (includes benign reverts — the loop
    /// working; the diagnose bar separates systematic from incidental).
    CompileGate = 1,
    /// Drafts that passed compile but the target lint persisted at verify.
    ClippyVerify = 2,
}

/// Per-rule outcome counts over one corpus epoch (the diagnose input).
/// `attempts` = the sum of the four fields; proposer-side declines are NOT
/// attempts (they never reach a gate).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuleOutcomes {
    pub parse_failures: u32,
    pub compile_failures: u32,
    /// Drafts that passed compile but the target lint persisted at verify.
    pub verify_failures: u32,
    /// Drafts applied and verified (the healthy outcome).
    pub applied: u32,
}

impl RuleOutcomes {
    /// Total gate-reaching attempts for the rule.
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.parse_failures
            .wrapping_add(self.compile_failures)
            .wrapping_add(self.verify_failures)
            .wrapping_add(self.applied)
    }

    /// The stage's failure share over attempts, in `[0.0, 1.0]`. Exact for
    /// every `u32` count (computed in `f64`).
    #[must_use]
    pub fn stage_rate(&self, stage: GateStage) -> f64 {
        let attempts = self.attempts();
        if attempts == 0 {
            return 0.0;
        }
        let failures = match stage {
            GateStage::SynParse => self.parse_failures,
            GateStage::CompileGate => self.compile_failures,
            GateStage::ClippyVerify => self.verify_failures,
        };
        f64::from(failures) / f64::from(attempts)
    }
}

/// A diagnosed degenerate rule: the grammar record's drive leg.
#[derive(Clone, Debug, PartialEq)]
pub struct DegenerateRule {
    pub rule_id: String,
    pub stage: GateStage,
    pub attempts: u32,
    pub failures: u32,
    /// `failures / attempts` at the recorded stage.
    pub rate: f64,
}

/// Population floor: a rule with fewer attempts is UNDECIDED, never flagged
/// (the issue-206 discipline — a floor, never a folded verdict).
pub const MIN_ATTEMPTS: u32 = 8;
/// The degeneracy bar (inclusive): a stage at/over this failure share flags.
pub const DEGENERACY_BAR: f64 = 0.5;

/// Diagnose with the pinned default floor and bar.
#[must_use]
pub fn diagnose(rules: &[(&str, RuleOutcomes)]) -> Vec<DegenerateRule> {
    diagnose_with(rules, MIN_ATTEMPTS, DEGENERACY_BAR)
}

/// The failure class a gate stage belongs to — the issue-206 arm-A split
/// (the `component::ComponentClass` Drafter/Gate tags, shared vocabulary):
/// parse and compile failures are DRAFTER-context failures (the drafted fix
/// was defective — the class a `context_notes` annotation addresses); a
/// verify-persist failure is a GATE failure (the fix passed compile but did
/// not resolve the lint — the demote-execution class).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FailureClass {
    /// `SynParse` + `CompileGate` (the drafter-context class).
    Drafter = 0,
    /// `ClippyVerify` (the gate class).
    Gate = 1,
}

impl From<GateStage> for FailureClass {
    fn from(stage: GateStage) -> Self {
        match stage {
            GateStage::SynParse | GateStage::CompileGate => Self::Drafter,
            GateStage::ClippyVerify => Self::Gate,
        }
    }
}

impl FailureClass {
    /// The stable short tag (mirrors `component::ComponentClass::as_str` so
    /// a binding record and its component attribution read as one family).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Drafter => "W",
            Self::Gate => "C",
        }
    }
}

/// Arm A (issue 206): the templated `context_notes` text for a diagnosed
/// degenerate rule — the ONLY probe-derivable corpus mutation. The template
/// is keyed by failure class; every value in the text is telemetry
/// (counts, share, stage, the corpus epoch the diagnosis was made under).
/// No fix content is derived: template split / span narrowing are excluded
/// by the issue's scope restriction, so the note only tells the consumer
/// what the record measured and to verify context before applying.
///
/// `epoch_short` is the SHORT form of the corpus epoch commitment (the
/// caller's digest prefix, typically 8-12 hex) — carried so a reader knows
/// the counts' vintage (the stale-epoch law: the diagnosis is stale the
/// moment a batch lands; the epoch label says which batch it predates).
///
/// Determinism: same inputs → byte-identical text (a re-run at a later
/// time reproduces; no timestamps — the module law).
#[must_use]
pub fn degenerate_context_note(rule: &DegenerateRule, epoch_short: &str) -> String {
    let class = FailureClass::from(rule.stage);
    let rate_pct = rule.rate * 100.0;
    let stage_word = match rule.stage {
        GateStage::SynParse => "syn-parse",
        GateStage::CompileGate => "compile-gate",
        GateStage::ClippyVerify => "verify-persist",
    };
    let class_clause = match class {
        FailureClass::Drafter => {
            "the drafted fix was defective for this span's context (drafter \
             class) — verify the span's shape against this rule's buggy/fixed \
             pair before applying; decline when the context does not match"
        }
        FailureClass::Gate => {
            "the drafted fix applied but the target lint persisted at verify \
             (gate class) — this rule's fix does not reliably resolve this \
             shape; treat any suggestion as a lead, not a fix"
        }
    };
    format!(
        "telemetry-context (corpus_binding v1, class {}, stage {}, epoch {}): \
         this rule's drafted fixes failed the {} in {}/{} recorded attempts \
         ({:.0}%) at that epoch — {}. Counts are cumulative store telemetry \
         over prior corpus epochs, not a measured gain.",
        class.as_str(),
        stage_word,
        epoch_short,
        stage_word,
        rule.failures,
        rule.attempts,
        rate_pct,
        class_clause,
    )
}

/// Diagnose with explicit floor and bar. Deterministic: output sorted by
/// `rule_id`; each rule's stage is the first in tie order whose rate clears
/// the bar; a rule below the attempt floor is UNDECIDED (absent, never
/// folded into a verdict).
#[must_use]
pub fn diagnose_with(
    rules: &[(&str, RuleOutcomes)],
    min_attempts: u32,
    bar: f64,
) -> Vec<DegenerateRule> {
    let mut out: Vec<DegenerateRule> = rules
        .iter()
        .filter(|(_, stats)| stats.attempts() >= min_attempts)
        .filter_map(|(rule_id, stats)| {
            let stage = GateStage::SYN_PARSE_ORDER
                .iter()
                .copied()
                .find(|stage| stats.stage_rate(*stage) >= bar)?;
            let failures = match stage {
                GateStage::SynParse => stats.parse_failures,
                GateStage::CompileGate => stats.compile_failures,
                GateStage::ClippyVerify => stats.verify_failures,
            };
            Some(DegenerateRule {
                rule_id: (*rule_id).to_string(),
                stage,
                attempts: stats.attempts(),
                failures,
                rate: stats.stage_rate(stage),
            })
        })
        .collect();
    out.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
    out
}

impl GateStage {
    /// First-index tie order (declaration order).
    const SYN_PARSE_ORDER: [GateStage; 3] = [
        GateStage::SynParse,
        GateStage::CompileGate,
        GateStage::ClippyVerify,
    ];
}

/// Why a binding refuses to drive a corpus edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingRefusal {
    /// The corpus moved under the record (a batch landed since the
    /// diagnosis) — the record is stale and must not drive an edit.
    StaleEpoch {
        recorded_epoch: [u8; 32],
        current_epoch: [u8; 32],
    },
    /// The rule is absent from the supplied corpus — it cannot carry a
    /// `context_notes` annotation (the writer refuses loud, never silently
    /// no-ops).
    RuleAbsent { drive_rule: String },
}

/// The binding record: `drive rule × gate stage`, BLAKE3-digested over the
/// content fields and keyed to the corpus epoch it was diagnosed under.
///
/// Digest scope (v2, issue 209 T1): domain separator ‖ `drive_rule` ‖ stage ‖
/// `corpus_epoch` ‖ the bench-identity cert tag — deliberately NOT
/// `recorded_at` (a re-run at a later time must reproduce the same record;
/// the timestamp is bookkeeping, not identity). The cert tag folds the
/// attached [`BenchIdentity`] in when present (tag byte 0x01 ‖ the four
/// digests) and a bare 0x00 tag when absent — the v1→v2 bump is safe: the
/// record was never persisted in the wild (the verb renders it, the note
/// landing is a batch commit).
pub struct CorpusBinding {
    pub drive_rule: String,
    pub stage: GateStage,
    /// BLAKE3 of the corpus this diagnosis was made under (caller-supplied —
    /// the corpus commitment is the epoch).
    pub corpus_epoch: [u8; 32],
    /// Unix seconds at record time; un-hashed bookkeeping.
    pub recorded_at: u64,
    /// The structural-identity certificate (issue 209 T1) — `None` is the
    /// pre-209 shape; its absence is not an axis move (see
    /// `corpus_referee::BenchIdentity`).
    pub bench_identity: Option<crate::corpus_referee::BenchIdentity>,
    pub digest: [u8; 32],
}

impl CorpusBinding {
    #[must_use]
    pub fn new(
        drive_rule: &str,
        stage: GateStage,
        corpus_epoch: [u8; 32],
        recorded_at: u64,
    ) -> Self {
        let digest = Self::compute_digest(drive_rule, stage, &corpus_epoch, None);
        Self {
            drive_rule: drive_rule.to_string(),
            stage,
            corpus_epoch,
            recorded_at,
            bench_identity: None,
            digest,
        }
    }

    #[must_use]
    fn compute_digest(
        drive_rule: &str,
        stage: GateStage,
        corpus_epoch: &[u8; 32],
        cert: Option<&crate::corpus_referee::BenchIdentity>,
    ) -> [u8; 32] {
        let mut h = Hasher::new();
        h.update(b"riir-refine/corpus_binding/v2");
        h.update(drive_rule.as_bytes());
        h.update(&[0]);
        h.update(&[stage as u8]);
        h.update(corpus_epoch);
        // The cert rides the record's identity when attached (v2); absence
        // is a bare tag so the no-cert digest stays deterministic.
        if let Some(c) = cert {
            h.update(&[0x01]);
            h.update(&c.baseline_atoms);
            h.update(&c.baseline_healed);
            h.update(&c.edited_atoms);
            h.update(&c.edited_healed);
        } else {
            h.update(&[0x00]);
        }
        *h.finalize().as_bytes()
    }

    /// Attach a bench-identity certificate to a binding (chainable — the
    /// cert rides the record the compose gate carries into a note landing).
    /// The digest is RE-DERIVED: the cert is part of the v2 identity (a
    /// record with a cert and the same record without are different
    /// identities, verifiable apart).
    #[must_use]
    pub fn with_bench_identity(mut self, identity: crate::corpus_referee::BenchIdentity) -> Self {
        self.digest = Self::compute_digest(
            &self.drive_rule,
            self.stage,
            &self.corpus_epoch,
            Some(&identity),
        );
        self.bench_identity = Some(identity);
        self
    }

    /// The certificate read on an attached cert; `None` (no cert) is the
    /// pre-209 binding shape and passes through — the cert is opt-in
    /// evidence, its absence is not an axis move.
    ///
    /// # Errors
    ///
    /// [`crate::corpus_referee::BenchIdentityRefusal::AxisMoved`] per
    /// [`crate::corpus_referee::BenchIdentity::verify`].
    pub fn verify_bench_identity(&self) -> Result<(), crate::corpus_referee::BenchIdentityRefusal> {
        match &self.bench_identity {
            Some(cert) => cert.verify(),
            None => Ok(()),
        }
    }

    /// The `sst_certified::verify_against` shape: accept the epoch the record
    /// was diagnosed under, refuse any other (loud, typed — never silent).
    ///
    /// # Errors
    ///
    /// Returns [`BindingRefusal::StaleEpoch`] when `current_epoch` differs
    /// from the recorded one (a corpus batch landed under the record).
    pub fn verify_against(&self, current_epoch: &[u8; 32]) -> Result<(), BindingRefusal> {
        if self.corpus_epoch == *current_epoch {
            Ok(())
        } else {
            Err(BindingRefusal::StaleEpoch {
                recorded_epoch: self.corpus_epoch,
                current_epoch: *current_epoch,
            })
        }
    }

    /// Arm A's composition gate (the stale-refusal law applied to the
    /// note writer): the note may be composed ONLY when the binding is
    /// current against `current_epoch` AND its drive rule is present in the
    /// supplied corpus. Returns the rendered template on success — the
    /// caller's edit is exactly `context_notes = Some(return)`.
    ///
    /// The counts are the diagnose-time values the binding was raised from
    /// (`rule`), re-checked here only for identity of rule + stage; the
    /// template rendering itself is [`degenerate_context_note`].
    ///
    /// # Errors
    ///
    /// [`BindingRefusal::StaleEpoch`] per [`Self::verify_against`];
    /// [`BindingRefusal::RuleAbsent`] when no corpus entry names
    /// `drive_rule`.
    pub fn compose_context_note(
        &self,
        corpus_rule_ids: &[String],
        diagnosed: &DegenerateRule,
        current_epoch: &[u8; 32],
        epoch_short: &str,
    ) -> Result<String, BindingRefusal> {
        self.verify_against(current_epoch)?;
        if diagnosed.rule_id != self.drive_rule || diagnosed.stage != self.stage {
            return Err(BindingRefusal::RuleAbsent {
                drive_rule: self.drive_rule.clone(),
            });
        }
        if !corpus_rule_ids.iter().any(|id| id == &self.drive_rule) {
            return Err(BindingRefusal::RuleAbsent {
                drive_rule: self.drive_rule.clone(),
            });
        }
        Ok(degenerate_context_note(diagnosed, epoch_short))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Task 1's synthetic fixture: a seeded degenerate cluster (the rule
    /// fires, the compile gate fails it systematically).
    fn degenerate_cluster() -> (&'static str, RuleOutcomes) {
        (
            "map_clone",
            RuleOutcomes {
                parse_failures: 0,
                compile_failures: 7,
                verify_failures: 0,
                applied: 3,
            },
        )
    }

    fn healthy_rule() -> (&'static str, RuleOutcomes) {
        (
            "items_after_statements",
            RuleOutcomes {
                parse_failures: 0,
                compile_failures: 1,
                verify_failures: 0,
                applied: 9,
            },
        )
    }

    #[test]
    fn seeded_degenerate_cluster_is_diagnosed() {
        let out = diagnose(&[degenerate_cluster(), healthy_rule()]);
        assert_eq!(out.len(), 1, "only the degenerate cluster flags");
        let d = &out[0];
        assert_eq!(d.rule_id, "map_clone");
        assert_eq!(d.stage, GateStage::CompileGate);
        assert_eq!(d.attempts, 10);
        assert_eq!(d.failures, 7);
        assert!((d.rate - 0.7).abs() < 1e-6);
    }

    #[test]
    fn healthy_rule_is_never_flagged() {
        assert_eq!(diagnose(&[healthy_rule()]).len(), 0);
    }

    #[test]
    fn below_the_attempt_floor_is_undecided_not_flagged() {
        let starving = (
            "young_rule",
            RuleOutcomes {
                parse_failures: 4,
                compile_failures: 0,
                verify_failures: 0,
                applied: 0,
            },
        );
        // 4/4 failing but under MIN_ATTEMPTS: UNDECIDED, never a verdict.
        assert_eq!(diagnose(&[starving]).len(), 0);
        // With the floor lowered it IS flagged — the floor is the only guard.
        assert_eq!(diagnose_with(&[starving], 4, DEGENERACY_BAR).len(), 1);
    }

    #[test]
    fn output_is_sorted_and_deterministic() {
        let rules = [
            (
                "zeta_rule",
                RuleOutcomes {
                    compile_failures: 5,
                    applied: 5,
                    ..RuleOutcomes::default()
                },
            ),
            (
                "alpha_rule",
                RuleOutcomes {
                    verify_failures: 6,
                    applied: 4,
                    ..RuleOutcomes::default()
                },
            ),
            ("mid_rule", healthy_rule().1),
        ];
        let first = diagnose(&rules);
        let second = diagnose(&rules);
        assert_eq!(first, second, "re-run reproduces byte-identical verdicts");
        let ids: Vec<&str> = first.iter().map(|d| d.rule_id.as_str()).collect();
        assert_eq!(
            ids,
            ["alpha_rule", "zeta_rule"],
            "sorted by rule_id; healthy rule absent"
        );
    }

    #[test]
    fn stage_tie_uses_first_index_order() {
        // parse 5/10 = 0.5 and compile 5/10 = 0.5 — BOTH clear the inclusive
        // bar; the earliest tie-order stage (SynParse) must win.
        let both_clear = (
            "tied_rule",
            RuleOutcomes {
                parse_failures: 5,
                compile_failures: 5,
                verify_failures: 0,
                applied: 0,
            },
        );
        let out = diagnose(&[both_clear]);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].stage,
            GateStage::SynParse,
            "earliest tie-order stage wins"
        );
        assert_eq!(
            out[0].failures, 5,
            "failures reported for the RECORDED stage"
        );
    }

    #[test]
    fn verify_persist_class_flags_clippy_verify() {
        let persists = (
            "stub_rule",
            RuleOutcomes {
                parse_failures: 0,
                compile_failures: 0,
                verify_failures: 9,
                applied: 1,
            },
        );
        let out = diagnose(&[persists]);
        assert_eq!(out[0].stage, GateStage::ClippyVerify);
    }

    #[test]
    fn bar_is_inclusive_at_the_boundary() {
        let exactly_at_bar = (
            "edge_rule",
            RuleOutcomes {
                compile_failures: 5,
                applied: 5,
                ..RuleOutcomes::default()
            },
        );
        assert_eq!(diagnose(&[exactly_at_bar])[0].rate, DEGENERACY_BAR);
    }

    fn epoch(seed: u8) -> [u8; 32] {
        let mut e = [0u8; 32];
        e[0] = seed;
        e
    }

    #[test]
    fn stale_epoch_refuses_and_current_accepts() {
        let binding =
            CorpusBinding::new("map_clone", GateStage::CompileGate, epoch(1), 1_700_000_000);
        // Same epoch: the record applies.
        assert!(binding.verify_against(&epoch(1)).is_ok());
        // A batch landed (epoch moved): loud typed refusal, exact bytes named.
        assert_eq!(
            binding.verify_against(&epoch(2)),
            Err(BindingRefusal::StaleEpoch {
                recorded_epoch: epoch(1),
                current_epoch: epoch(2)
            })
        );
    }

    #[test]
    fn digest_is_content_deterministic_and_epoch_scoped() {
        let a = CorpusBinding::new("map_clone", GateStage::CompileGate, epoch(1), 1_700_000_000);
        // Same content, later timestamp: SAME digest (recorded_at is not identity).
        let b = CorpusBinding::new("map_clone", GateStage::CompileGate, epoch(1), 1_700_000_500);
        assert_eq!(a.digest, b.digest);
        // A different rule or epoch: different digest (content-scoped identity).
        let other_rule = CorpusBinding::new(
            "map_unwrap_or",
            GateStage::CompileGate,
            epoch(1),
            1_700_000_000,
        );
        let other_epoch =
            CorpusBinding::new("map_clone", GateStage::CompileGate, epoch(2), 1_700_000_000);
        assert_ne!(a.digest, other_rule.digest);
        assert_ne!(a.digest, other_epoch.digest);
    }

    // ── arm A: the templated context_notes writer (issue 206 corpus half) ──

    #[test]
    fn failure_class_splits_the_three_stages() {
        assert_eq!(
            FailureClass::from(GateStage::SynParse),
            FailureClass::Drafter
        );
        assert_eq!(
            FailureClass::from(GateStage::CompileGate),
            FailureClass::Drafter
        );
        assert_eq!(
            FailureClass::from(GateStage::ClippyVerify),
            FailureClass::Gate
        );
        // Shared vocabulary with the component taxonomy.
        assert_eq!(FailureClass::Drafter.as_str(), "W");
        assert_eq!(FailureClass::Gate.as_str(), "C");
    }

    #[test]
    fn note_is_deterministic_and_carries_the_telemetry() {
        let d = diagnose(&[degenerate_cluster()]).remove(0);
        let a = degenerate_context_note(&d, "ac280021");
        let b = degenerate_context_note(&d, "ac280021");
        assert_eq!(a, b, "same telemetry in → byte-identical note out");
        // Every probe-derivable value appears: class, stage word, counts, share.
        assert!(a.contains("class W"), "drafter-class tag present: {a}");
        assert!(a.contains("compile-gate"), "stage word present: {a}");
        assert!(a.contains("7/10"), "counts present: {a}");
        assert!(a.contains("70%"), "share present: {a}");
        assert!(a.contains("epoch ac280021"), "epoch carried: {a}");
        // A different epoch label changes the text (the counts' vintage is
        // part of the note's identity).
        assert_ne!(a, degenerate_context_note(&d, "0182f9d7"));
    }

    #[test]
    fn note_text_is_keyed_by_failure_class() {
        let drafter = diagnose(&[degenerate_cluster()]).remove(0);
        let gate = diagnose(&[(
            "stub_rule",
            RuleOutcomes {
                verify_failures: 9,
                applied: 1,
                ..RuleOutcomes::default()
            },
        )])
        .remove(0);
        let drafter_note = degenerate_context_note(&drafter, "ac280021");
        let gate_note = degenerate_context_note(&gate, "ac280021");
        assert!(
            drafter_note.contains("verify the span's shape"),
            "{drafter_note}"
        );
        assert!(!drafter_note.contains("lead, not a fix"));
        assert!(gate_note.contains("lead, not a fix"), "{gate_note}");
        assert!(!gate_note.contains("verify the span's shape"));
    }

    #[test]
    fn note_composes_from_a_full_diagnose_pass() {
        // The pipeline shape: diagnose → filter → note per rule, deterministic
        // over the whole pass (the writer's real entry contract).
        let rules = [degenerate_cluster(), healthy_rule()];
        let epoch = "ac280021";
        let notes: Vec<(String, String)> = diagnose(&rules)
            .iter()
            .map(|d| (d.rule_id.clone(), degenerate_context_note(d, epoch)))
            .collect();
        assert_eq!(notes.len(), 1, "only the degenerate rule emits a note");
        assert_eq!(notes[0].0, "map_clone");
        assert!(
            notes[0]
                .1
                .starts_with("telemetry-context (corpus_binding v1")
        );
    }

    #[test]
    fn compose_gate_accepts_current_epoch_and_present_rule() {
        let d = diagnose(&[degenerate_cluster()]).remove(0);
        let binding = CorpusBinding::new("map_clone", GateStage::CompileGate, epoch(1), 0);
        let ids = vec!["map_clone".to_string(), "other".to_string()];
        let note = binding
            .compose_context_note(&ids, &d, &epoch(1), "ac280021")
            .expect("current epoch + rule present");
        assert_eq!(note, degenerate_context_note(&d, "ac280021"));
    }

    #[test]
    fn compose_gate_refuses_stale_epoch_loud() {
        let d = diagnose(&[degenerate_cluster()]).remove(0);
        let binding = CorpusBinding::new("map_clone", GateStage::CompileGate, epoch(1), 0);
        let ids = vec!["map_clone".to_string()];
        assert_eq!(
            binding.compose_context_note(&ids, &d, &epoch(2), "x"),
            Err(BindingRefusal::StaleEpoch {
                recorded_epoch: epoch(1),
                current_epoch: epoch(2)
            }),
            "a batch landed under the record: the writer refuses"
        );
    }

    #[test]
    fn compose_gate_refuses_absent_rule_and_mismatched_diagnosis() {
        let d = diagnose(&[degenerate_cluster()]).remove(0);
        let binding = CorpusBinding::new("map_clone", GateStage::CompileGate, epoch(1), 0);
        // Rule not in the corpus: refuse loud.
        assert_eq!(
            binding.compose_context_note(&["other".to_string()], &d, &epoch(1), "x"),
            Err(BindingRefusal::RuleAbsent {
                drive_rule: "map_clone".to_string()
            })
        );
        // Diagnosis names a different rule/stage than the binding: refuse
        // (the composition must be the binding's own diagnose output).
        let other = DegenerateRule {
            rule_id: "other_rule".to_string(),
            stage: GateStage::ClippyVerify,
            attempts: 9,
            failures: 5,
            rate: 0.5,
        };
        assert!(
            binding
                .compose_context_note(&["map_clone".to_string()], &other, &epoch(1), "x")
                .is_err()
        );
    }
}
