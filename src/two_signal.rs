//! Issue 232 T5 — the two-signal composition (Research 495, the T³
//! holdout-epistemology extraction's L5 law; source: dang2026t3 via
//! `riir-train/.research/495`).
//!
//! **The law:** a lone held-out window must never single-handedly reverse
//! an armed lane (L5: the holdout signal ALONE measured +0.021 WORSE than
//! nothing). A reversal commits only when BOTH landed signals agree —
//!
//! 1. the **window half** is a FRESH read (never consumed — T1's ledger;
//!    a consumed window's signal is UNDECIDABLE and corroborates
//!    nothing), AND
//! 2. the **arming half** corroborates: the consult says the arming
//!    premise has FAILED (below the knee — T3's own boolean, which IS
//!    `gate < 0.5 ⟺ m < knee`, no new threshold), or the surface measure
//!    refused on the `OnePass` fence (the premise never held — the
//!    strongest corroboration shape).
//!
//! Everything else lands in a named, reported state — never folded into
//! a pass. The truth table is 2×2×2 (consult × freshness × proposal) and
//! exactly ONE row commits; the fixture pins the whole table.
//!
//! **One decision artifact:** the committed decision carries every fact
//! that produced it (the arming facts, the window facts, the corroboration
//! verdict) — a consumer that later asks "why did this lane flip?" reads
//! the artifact, never a diff of two reports.
//!
//! **Side state (T³'s φ is throwaway):** candidate probes evaluate on a
//! FORKED copy ([`SideState::fork`]) — the serving artifact is mutated
//! ONLY through [`commit`], and [`commit`] applies its closure ONLY for a
//! committed ([`Composition::Commit`]) decision. There is no other write
//! path and no bypass constructor: the discipline is structural, not
//! conventional (the fixture proves a non-commit verdict never invokes
//! the closure).
//!
//! **Report-only first:** nothing here gates, refuses, or demotes any fix
//! path (the issue's proof-path ledger: G1 fixture → report-only readout
//! → correlation/utility cell → only then a plan + flag). The CLI lane
//! (`--composition-report`, composed posture) reads the REAL store's
//! arming evidence (T3's measured gate) + window evidence (T1's ledger
//! state over the store's canonical digest, READ-ONLY — the report never
//! records a consumption) and composes them; `--propose-reversal`
//! evaluates the HYPOTHETICAL reversal proposal against that real state
//! (disclosed as hypothetical) — the L5 law read on lived data.
//!
//! **Tier discipline:** L5's +0.021 is a TRAINING-surface number — never
//! imported. The composition adds no constants and no thresholds: it only
//! ANDs two predicates that already landed (T1's freshness, T3's
//! armed/unarmed consult). The verdict vocabulary follows T1's law: a
//! REPORT may not phrase a refusal — `UPHOLD`/`COMMIT`/`UNDECIDABLE`
//! are verdicts, never blocks.
//!
//! ## Feature posture
//!
//! `two_signal_composition` (opt-in, default-OFF, no implies — std only;
//! the core composes its OWN evidence types, decoupled from the producer
//! modules — conversions live behind their features). The CLI lane
//! composes `all(two_signal_composition, multiplicity_gate,
//! consumed_window_ledger, latent_retrieval)`. Feature-off is
//! byte-identical by construction: new gated code only, no shared-path
//! edit.

/// The decision artifact's schema version — carried in the render, so a
/// law change moves every consumer loud.
pub const DECISION_ARTIFACT_VERSION: u64 = 1;

/// The arming-side consult verdict, carried as data so the composition
/// core stays decoupled from the gate module (the fixture constructs it
/// directly; the CLI derives it from a real `MultiplicityGate`).
#[derive(Debug, Clone, PartialEq)]
pub enum ConsultVerdict {
    /// The arming premise HOLDS — the evidence is reuse-dominant (the
    /// consult is armed: `m ≥ knee`, which is `gate ≥ 0.5`, T3's own
    /// equivalence). A reversal has no arming-side support.
    Holds {
        gate: f64,
        multiplicity: u64,
        knee: u64,
    },
    /// The premise FAILED — the consult is unarmed (`m < knee`).
    Failed(FailureFacts),
}

/// Why the arming side agrees with a reversal.
#[derive(Debug, Clone, PartialEq)]
pub enum FailureFacts {
    /// Below the knee — the measured gate stands, the unit's counter
    /// just sits under it.
    BelowKnee {
        gate: f64,
        multiplicity: u64,
        knee: u64,
    },
    /// The surface measure refused on the `OnePass` fence — the premise
    /// never held (1-pass surfaces never arm at any multiplicity). The
    /// kind tag (`"one-pass"`) names the fence verdict; blindness and
    /// malformed records are NOT here — they yield no composition at all
    /// (a blind readout composes nothing; the CLI refuses before
    /// composing).
    SurfaceRefused { kind: String },
}

impl FailureFacts {
    /// The short tag the decision line carries.
    #[must_use]
    pub fn tag(&self) -> String {
        match self {
            Self::BelowKnee { .. } => "below-knee".to_string(),
            Self::SurfaceRefused { kind } => format!("surface:{kind}"),
        }
    }
}

/// The arming-side evidence at decision time.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmingEvidence {
    pub consult: ConsultVerdict,
}

/// The window-side evidence (T1's read, carried as data).
#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvidence {
    /// First consumption — the window's signal is valid evidence.
    Fresh { proposes_reversal: bool },
    /// Re-consumption — the window is poisoned; its verdict is
    /// UNDECIDABLE (T1's law) and can corroborate nothing.
    Consumed { reuse: u32, proposes_reversal: bool },
}

/// The composed decision — ONE artifact carrying every fact that
/// produced it.
#[derive(Debug, Clone, PartialEq)]
pub enum Composition {
    /// Both signals agree — the proposal commits through the one write
    /// path ([`commit`]). This variant is UNREACHABLE from a consumed
    /// window or a lone one: the constructors below are the only path,
    /// and neither state maps here.
    Commit(CommitFacts),
    /// The lane stands.
    Uphold(UpholdReason),
    /// The window half is poisoned — the composition can decide nothing
    /// either way (T1's UNDECIDABLE, carried forward).
    WindowUndecidable { reuse: u32 },
}

/// Why the lane stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpholdReason {
    /// Nothing was proposed — there is nothing to compose.
    NoProposal,
    /// A lone fresh window proposed against a HOLDING arming consult —
    /// the L5 law: one signal never reverses the lane alone.
    LoneProposal,
}

/// The committed decision's facts.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitFacts {
    /// Why the arming side agreed.
    pub failure: FailureFacts,
    /// The window half was fresh (always true in a commit — carried for
    /// the artifact's completeness, so a reader never infers it).
    pub window_fresh: bool,
}

/// Compose the two signals into one decision. Pure: the same inputs
/// always yield the same verdict (the fixture pins byte-identical
/// repeats), and no input's absence is ever folded into a commit.
#[must_use]
pub fn compose(arming: &ArmingEvidence, window: &WindowEvidence) -> Composition {
    let (fresh, reuse, proposes) = match window {
        WindowEvidence::Fresh { proposes_reversal } => (true, 0, *proposes_reversal),
        WindowEvidence::Consumed {
            reuse,
            proposes_reversal,
        } => (false, *reuse, *proposes_reversal),
    };
    // Nothing proposed — there is nothing to compose, whatever the other
    // half says (the poison state is reported only when a proposal
    // exists to adjudicate).
    if !proposes {
        return Composition::Uphold(UpholdReason::NoProposal);
    }
    // The poisoned-window law is INDEPENDENT of the arming side: a
    // consumed window's signal decides nothing (T1: UNDECIDABLE).
    if !fresh {
        return Composition::WindowUndecidable { reuse };
    }
    match &arming.consult {
        // The L5 row: a lone held-out signal never reverses a lane whose
        // arming premise still holds.
        ConsultVerdict::Holds { .. } => Composition::Uphold(UpholdReason::LoneProposal),
        ConsultVerdict::Failed(failure) => Composition::Commit(CommitFacts {
            failure: failure.clone(),
            window_fresh: true,
        }),
    }
}

/// A forked evaluation state — T³'s φ is throwaway: candidate probes
/// mutate the fork, never the serving artifact. Promotion to the
/// serving artifact happens only through [`commit`].
#[derive(Debug, Clone)]
pub struct SideState<A: Clone> {
    side: A,
}

impl<A: Clone> SideState<A> {
    /// Fork the serving artifact into side state (the probe's scratch
    /// world).
    #[must_use]
    pub fn fork(serving: &A) -> Self {
        Self {
            side: serving.clone(),
        }
    }

    /// Evaluate a candidate against the side state. The closure sees the
    /// FORK — the serving artifact is unreachable here by construction.
    pub fn probe(&mut self, f: impl FnOnce(&mut A)) {
        f(&mut self.side);
    }

    /// Read the side state (a probe's verdict reads out of the fork).
    #[must_use]
    pub fn state(&self) -> &A {
        &self.side
    }

    /// Consume the fork into its inner state (the caller's read-out).
    #[must_use]
    pub fn into_inner(self) -> A {
        self.side
    }
}

/// The ONE write path to a serving artifact: a COMMITTED decision
/// applies `apply`; every other verdict leaves the artifact untouched
/// and the closure never runs (returns `false`). There is no bypass —
/// the fixture proves the seven non-commit rows never invoke it.
pub fn commit<A>(serving: &mut A, decision: &Composition, apply: impl FnOnce(&mut A)) -> bool {
    if matches!(decision, Composition::Commit { .. }) {
        apply(serving);
        true
    } else {
        false
    }
}

impl Composition {
    /// The one-line verdict render. Report vocabulary only — the
    /// refusal/blocked word family never appears (T1's verdict law: a
    /// REPORT may not phrase a refusal).
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            Self::Commit(facts) => format!(
                "[composition] verdict=COMMIT signals=both premise={} window=fresh — \
                 corroborated: the fresh window and the arming side agree \
                 (I232 T5, report-only)",
                facts.failure.tag()
            ),
            Self::Uphold(UpholdReason::NoProposal) => {
                "[composition] verdict=UPHOLD reason=no-proposal — nothing proposed; \
                 the lane stands (I232 T5, report-only)"
                    .to_string()
            }
            Self::Uphold(UpholdReason::LoneProposal) => {
                "[composition] verdict=UPHOLD reason=lone-window — a lone held-out \
                 signal does not reverse a holding lane (I232 T5, report-only)"
                    .to_string()
            }
            Self::WindowUndecidable { reuse } => format!(
                "[composition] verdict=UNDECIDABLE reuse={reuse} — a consumed \
                 window's signal decides nothing (I232 T5, report-only)"
            ),
        }
    }

    /// The full decision artifact: version + both signals' facts + the
    /// verdict. Deterministic formatting (gate at 5 decimals; the
    /// bit-exact value is the producer fixture's business).
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "two-signal composition (i232 T5 — report-only; L5's law: a lone \
             held-out signal never reverses an armed lane alone) \
             v{DECISION_ARTIFACT_VERSION}"
        );
        match self {
            Self::Commit(facts) => {
                let _ = writeln!(out, "verdict: COMMIT (both signals agree)");
                match &facts.failure {
                    FailureFacts::BelowKnee {
                        gate,
                        multiplicity,
                        knee,
                    } => {
                        let _ = writeln!(
                            out,
                            "arming: premise=failed below-knee gate={gate:.5} \
                             multiplicity={multiplicity} knee={knee}"
                        );
                    }
                    FailureFacts::SurfaceRefused { kind } => {
                        let _ = writeln!(
                            out,
                            "arming: premise=failed surface:{kind} (the fence verdict — \
                             the premise never held)"
                        );
                    }
                }
                let _ = writeln!(
                    out,
                    "window: fresh={} (first consumption; the signal is valid evidence)",
                    facts.window_fresh
                );
                let _ = write!(
                    out,
                    "write path: commit — the serving artifact mutates ONLY here \
                     (probes evaluated on side state)"
                );
            }
            Self::Uphold(reason) => {
                let _ = writeln!(out, "verdict: UPHOLD");
                match reason {
                    UpholdReason::NoProposal => {
                        let _ = write!(
                            out,
                            "reason: no-proposal — nothing proposed; the lane stands"
                        );
                    }
                    UpholdReason::LoneProposal => {
                        let _ = write!(
                            out,
                            "reason: lone-window — a lone held-out signal does not \
                             reverse a holding lane (L5)"
                        );
                    }
                }
            }
            Self::WindowUndecidable { reuse } => {
                let _ = writeln!(out, "verdict: UNDECIDABLE (window reuse={reuse})");
                let _ = write!(
                    out,
                    "window: consumed — a re-consumed window's signal decides nothing \
                     (I232 T1's law carried forward)"
                );
            }
        }
        out
    }
}

#[cfg(feature = "multiplicity_gate")]
impl ArmingEvidence {
    /// Derive the arming evidence from a real measured gate and the
    /// unit's (or the surface's) multiplicity. The consult IS T3's own
    /// boolean — the composition adds no threshold.
    #[must_use]
    pub fn from_consult(
        gate: &crate::multiplicity_gate::MultiplicityGate,
        multiplicity: u64,
    ) -> Self {
        let knee = gate.knee();
        let value = gate.gate(multiplicity);
        let consult = match gate.consult(multiplicity) {
            crate::multiplicity_gate::Consult::Armed => ConsultVerdict::Holds {
                gate: value,
                multiplicity,
                knee,
            },
            crate::multiplicity_gate::Consult::Unarmed => {
                ConsultVerdict::Failed(FailureFacts::BelowKnee {
                    gate: value,
                    multiplicity,
                    knee,
                })
            }
        };
        Self { consult }
    }

    /// Derive the arming evidence from a refused surface measure. Only
    /// the `OnePass` fence corroborates (the premise never held) — a blind
    /// or malformed record yields NO composition at all: a blind readout
    /// composes nothing, and the caller refuses before composing.
    ///
    /// # Errors
    ///
    /// [`crate::multiplicity_gate::KneeRefusal::Blind`] and
    /// [`crate::multiplicity_gate::KneeRefusal::MalformedZero`] — the
    /// named reason; never folded into a verdict.
    pub fn from_refusal(reason: &crate::multiplicity_gate::KneeRefusal) -> Result<Self, String> {
        match reason {
            crate::multiplicity_gate::KneeRefusal::OnePassSurface { .. } => Ok(Self {
                consult: ConsultVerdict::Failed(FailureFacts::SurfaceRefused {
                    kind: "one-pass".to_string(),
                }),
            }),
            crate::multiplicity_gate::KneeRefusal::Blind => Err(
                "arming evidence blind — zero units measured; a blind surface \
                 composes nothing"
                    .to_string(),
            ),
            crate::multiplicity_gate::KneeRefusal::MalformedZero { units } => Err(format!(
                "arming evidence malformed — {units} unit(s) with zero encounters; \
                 a corrupt surface record composes nothing"
            )),
        }
    }
}

#[cfg(feature = "consumed_window_ledger")]
impl WindowEvidence {
    /// Derive the window evidence from a real ledger read (the mutating
    /// consult path).
    #[must_use]
    pub fn from_read(read: &crate::consumed_window::WindowRead, proposes_reversal: bool) -> Self {
        match read {
            crate::consumed_window::WindowRead::Fresh => Self::Fresh { proposes_reversal },
            crate::consumed_window::WindowRead::Consumed { reuse } => Self::Consumed {
                reuse: *reuse,
                proposes_reversal,
            },
        }
    }

    /// The READ-ONLY form: a report lane reads the reuse count WITHOUT
    /// recording a consumption (T4's hygiene law — the report never
    /// saves). `reuse == 0` is fresh; anything above is consumed.
    #[must_use]
    pub fn from_reuse(reuse: u32, proposes_reversal: bool) -> Self {
        if reuse == 0 {
            Self::Fresh { proposes_reversal }
        } else {
            Self::Consumed {
                reuse,
                proposes_reversal,
            }
        }
    }
}

#[cfg(all(test, feature = "two_signal_composition"))]
mod tests {
    use super::*;

    fn holding() -> ArmingEvidence {
        ArmingEvidence {
            consult: ConsultVerdict::Holds {
                gate: 0.731,
                multiplicity: 60,
                knee: 51,
            },
        }
    }

    fn failed_below_knee() -> ArmingEvidence {
        ArmingEvidence {
            consult: ConsultVerdict::Failed(FailureFacts::BelowKnee {
                gate: 0.000_05,
                multiplicity: 3,
                knee: 51,
            }),
        }
    }

    fn fresh_proposal() -> WindowEvidence {
        WindowEvidence::Fresh {
            proposes_reversal: true,
        }
    }

    fn fresh_neutral() -> WindowEvidence {
        WindowEvidence::Fresh {
            proposes_reversal: false,
        }
    }

    fn consumed_proposal(reuse: u32) -> WindowEvidence {
        WindowEvidence::Consumed {
            reuse,
            proposes_reversal: true,
        }
    }

    #[test]
    fn lone_window_reversal_upholds_and_never_commits() {
        // I232 G1: a lone fresh window proposing against a HOLDING
        // consult is upheld — one signal never reverses the lane alone
        // (L5), and the write path stays shut.
        let d = compose(&holding(), &fresh_proposal());
        assert_eq!(d, Composition::Uphold(UpholdReason::LoneProposal));
        let mut serving = vec![7u64, 8, 9];
        let applied = commit(&mut serving, &d, |s| s.push(99));
        assert!(!applied);
        assert_eq!(serving, vec![7, 8, 9]);
    }

    #[test]
    fn corroborated_reversal_commits_exactly_once() {
        // The corroborated row: the premise failed below the knee + a
        // fresh window proposes — the commit path fires ONCE and the
        // artifact carries both signals' facts.
        let d = compose(&failed_below_knee(), &fresh_proposal());
        let facts = match &d {
            Composition::Commit(f) => f.clone(),
            other => panic!("expected COMMIT, got {other:?}"),
        };
        assert!(facts.window_fresh);
        assert_eq!(
            facts.failure,
            FailureFacts::BelowKnee {
                gate: 0.00005,
                multiplicity: 3,
                knee: 51,
            }
        );
        let mut serving = vec![1u64];
        let applied = commit(&mut serving, &d, |s| s.push(2));
        assert!(applied);
        assert_eq!(serving, vec![1, 2]);
        // A second commit on the SAME decision is still one apply per
        // call — the caller owns idempotence; the point here is the
        // closure runs only for Commit verdicts.
        assert!(commit(&mut serving, &d, |s| s.push(2)));
        assert_eq!(serving, vec![1, 2, 2]);
    }

    #[test]
    fn consumed_window_is_undecidable_on_both_arming_sides() {
        // The poisoned-window law is independent of the arming side: a
        // consumed window's proposal decides nothing, holding or failed.
        for arming in [holding(), failed_below_knee()] {
            let d = compose(&arming, &consumed_proposal(2));
            assert_eq!(d, Composition::WindowUndecidable { reuse: 2 });
            let mut serving = vec![5u64];
            assert!(!commit(&mut serving, &d, |s| s.clear()));
            assert_eq!(serving, vec![5]);
        }
        let d = compose(&holding(), &consumed_proposal(9));
        assert!(
            d.line().contains("verdict=UNDECIDABLE") && d.line().contains("reuse=9"),
            "{}",
            d.line()
        );
    }

    #[test]
    fn no_proposal_upholds_on_every_arming_window_shape() {
        // Nothing proposed — nothing to compose, whatever the halves say.
        let shapes = [
            WindowEvidence::Fresh {
                proposes_reversal: false,
            },
            WindowEvidence::Consumed {
                reuse: 4,
                proposes_reversal: false,
            },
        ];
        for arming in [holding(), failed_below_knee()] {
            for w in &shapes {
                assert_eq!(
                    compose(&arming, w),
                    Composition::Uphold(UpholdReason::NoProposal)
                );
            }
        }
    }

    #[test]
    fn truth_table_has_exactly_one_committing_row() {
        // 2×2×2 (consult × freshness × proposal) — exactly ONE row
        // commits; flipping any single conjunct away from it never does.
        let armings = [holding(), failed_below_knee()];
        let windows = [
            fresh_proposal(),
            fresh_neutral(),
            consumed_proposal(1),
            WindowEvidence::Consumed {
                reuse: 3,
                proposes_reversal: false,
            },
        ];
        let mut commits = 0;
        for a in &armings {
            for w in &windows {
                if matches!(compose(a, w), Composition::Commit(_)) {
                    commits += 1;
                }
            }
        }
        assert_eq!(commits, 1, "exactly one committing row in the truth table");
    }

    #[test]
    fn render_carries_facts_and_report_vocabulary_only() {
        // The decision lines are verdicts, never phrased refusals — the
        // banned word family never appears; the artifact carries both
        // signals' facts.
        let d = compose(&failed_below_knee(), &fresh_proposal());
        let text = d.render();
        assert!(text.contains("verdict: COMMIT"));
        assert!(text.contains("below-knee gate=0.00005"));
        assert!(text.contains("multiplicity=3 knee=51"));
        assert!(text.contains("window: fresh=true"));
        let lone = compose(&holding(), &fresh_proposal());
        assert!(lone.line().contains("verdict=UPHOLD"));
        assert!(lone.line().contains("reason=lone-window"));
        for banned in ["refus", "blocked", "demote", "skip"] {
            for text in [d.line(), d.render(), lone.line(), lone.render()] {
                assert!(
                    !text.to_ascii_lowercase().contains(banned),
                    "banned word {banned} in: {text}"
                );
            }
        }
        assert!(
            d.render()
                .contains(&format!("v{DECISION_ARTIFACT_VERSION}"))
        );
    }

    #[test]
    fn same_inputs_compose_byte_identically() {
        // Pure function: the same evidence composes to the same verdict,
        // byte-identical renders (determinism pin).
        let a = failed_below_knee();
        let w = fresh_proposal();
        let d1 = compose(&a, &w);
        let d2 = compose(&a, &w);
        assert_eq!(d1, d2);
        assert_eq!(d1.line(), d2.line());
        assert_eq!(d1.render(), d2.render());
    }

    #[test]
    fn side_state_probe_never_touches_the_serving_artifact() {
        // T³'s φ is throwaway: probes see the fork; the serving
        // artifact is unreachable from the probe path by construction.
        let serving = vec![10u64, 20];
        let mut side = SideState::fork(&serving);
        side.probe(|s| {
            s.push(30);
            s[0] = 999;
        });
        assert_eq!(side.state(), &vec![999, 20, 30]);
        assert_eq!(serving, vec![10, 20]);
        let inner = side.into_inner();
        assert_eq!(inner, vec![999, 20, 30]);
    }

    #[cfg(feature = "multiplicity_gate")]
    #[test]
    fn one_pass_refusal_corroborates_blind_and_malformed_yield_nothing() {
        use crate::multiplicity_gate::KneeRefusal;
        // The fence corroborates (the premise never held).
        let fence = ArmingEvidence::from_refusal(&KneeRefusal::OnePassSurface {
            units: 10,
            total: 11,
        })
        .expect("one-pass corroborates");
        let d = compose(&fence, &fresh_proposal());
        assert!(matches!(d, Composition::Commit(_)));
        assert!(
            d.line().contains("premise=surface:one-pass"),
            "{}",
            d.line()
        );
        // Blind and malformed are evidence-quality failures: no
        // composition at all, named loudly.
        assert!(ArmingEvidence::from_refusal(&KneeRefusal::Blind).is_err());
        assert!(ArmingEvidence::from_refusal(&KneeRefusal::MalformedZero { units: 2 }).is_err());
        // A real measured gate consults through unchanged semantics:
        // [6,2,1,1] measures knee 6 — consult(6) holds, consult(2) failed.
        let g = crate::multiplicity_gate::MultiplicityGate::measure(&[6, 2, 1, 1])
            .expect("reuse-dominant");
        let holds = ArmingEvidence::from_consult(&g, 6);
        let failed = ArmingEvidence::from_consult(&g, 2);
        assert_eq!(
            compose(&holds, &fresh_proposal()),
            Composition::Uphold(UpholdReason::LoneProposal)
        );
        assert!(matches!(
            compose(&failed, &fresh_proposal()),
            Composition::Commit(_)
        ));
    }
}
