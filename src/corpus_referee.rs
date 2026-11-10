//! The c′ referee (issue 209) — telemetry before/after + structural-identity
//! certificate for single-rule corpus-note edits.
//!
//! Verdict provenance: issue 206's delegated-owner verdict (Claude round 1,
//! REVISE → c′, session `411cdf2b`, 2026-11-05). The fixture-set score-bench
//! A/B is structurally UNDECIDED for single-rule arms (the affected stratum
//! cannot reach n ≥ 30 on a finite fixture set), and a zero-flip check cannot
//! fail when the edit provably misses the bench axis — so c′ replaces the
//! referee's EVIDENCE SOURCE, never its bars: n ≥ 30 and LCB95 survive, only
//! where the evidence comes from changes. Pre-registered in `.issues/209`
//! BEFORE any data was read (the no-silent-bar-bend law).
//!
//! # The two halves
//!
//! ## 1. The structural-identity certificate (the bench-axis half)
//!
//! Where a corpus-note edit provably does not touch the bench axis (the
//! 11-04 case: atom tables are a pure function of `buggy_code`, healed
//! output rides compile-time dispatch), the fixture A/B is replaced by the
//! certificate: BLAKE3 digests of the atom tables + healed output, baseline
//! vs edited, recorded on the binding (see
//! `CorpusBinding::verify_bench_identity`). Equal digests = the axis did
//! not move = no bench needed. Unequal digests refuse loud — the edit DID
//! touch the axis and the fixture bench runs as usual.
//!
//! ## 2. The telemetry referee (the advisory-channel half)
//!
//! The note's real effect surface is the rule's own live failure rate —
//! fixture sets are finite, live attempts are not. For an enrolled case
//! ([`Enrollment`]: rule, edit timestamp, pre-edit tally snapshot):
//!
//! - **Split:** the rule's attempts partition at `edit_ts`. The pre side is
//!   the seeded snapshot; the post side is `current − snapshot` (saturating
//!   per bucket — tallies are cumulative, so subtraction is exact where the
//!   snapshot population matches the sweep and an over-count where it does
//!   not; see the bias note below).
//! - **Bar:** one-sided Beta-LCB95 on the failure-rate drop
//!   `p_pre − p_post > 0`, computed as the Bonferroni split
//!   `LCB₀.₀₂₅(p_pre) − UCB₀.₉₇₅(p_post)` via `katgpt_core::best_belief_score`
//!   (the ε-quantile of Beta(1+f, 1+s)); the union bound puts the joint
//!   coverage of the difference at ≥ 95 %. Requires **n ≥ 30 post-edit
//!   attempts** before any read — below it the verdict is
//!   [`Verdict::Pending`], never folded into a pass.
//! - **Confound control (the `DiD` arm):** each corpus batch also changes the
//!   drafter and the corpus, so a raw drop is not attributable. The control
//!   set is the other rules whose evidence splits on BOTH sides of
//!   `edit_ts` (per-row `last_seen_at`: a row untouched since before the
//!   edit is pure pre-edit evidence; a row updated after it is post-window
//!   evidence). `ΔC` = the median of the controls' own rate changes
//!   (deterministic: sorted, even n takes the mean of the two middles).
//!   **PASS requires the drop to exceed the drift**: `lcb_drop > 0 ∧
//!   (p̂_pre − p̂_post) > ΔC`. A drop that does not exceed the era drift
//!   fails the bar — a bar that passed era-wide drafter improvement would be
//!   a false attribution, which is exactly what the verdict's confound pin
//!   exists to prevent.
//! - **Starvation kill window:** if the post window has not reached n ≥ 30
//!   within [`STARVATION_BATCHES`] observed corpus epochs (tracked by the
//!   caller across runs — the verb compares the live corpus epoch against
//!   the enrollment's `last_seen_epoch` and bumps `batches_observed` on
//!   change), the verdict is [`Verdict::Starved`] — UNDECIDED-by-starvation,
//!   recorded as such, never silently dropped.
//! - **Vintage:** the read requires the rule to have post-edit evidence in
//!   the recent window; a stale rule reads [`Verdict::Pending`] with the
//!   vintage reason (the writer lane's existing discipline — the vintage
//!   predicate gates the READ, not the enrollment).
//!
//! # The bias disclosure (pinned, not hidden)
//!
//! The post-window delta rides the CURRENT sweep population; a snapshot
//! taken over a smaller store population (case #1: the 11-04 harvest swept
//! 3 stores, the standing verb sweeps 15) lets pre-edit attempts from
//! later-swept stores land in the post window. The bias direction is
//! toward the pre-edit rate — conservative against a PASS claim, ambiguous
//! for a FAIL. The enrollment carries the snapshot population note and the
//! report prints it verbatim.
//!
//! # Modelless
//!
//! Attempt counts, Beta bounds, BLAKE3 digests. No training, no gradient,
//! no network. The whole module is ungated store-free arithmetic over the
//! [`AttemptRow`] view — the durable-store mapping (`TrajRowDoc →
//! AttemptRow`) is the only latent_retrieval-gated seam, so the state
//! machine, the LCB math and the verdict table run at default features.

use katgpt_core::best_belief_score;

use crate::component::ComponentClass;

// ── The structural-identity certificate (issue 209 T1) ──────────────────────

/// Why a bench-identity certificate refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchIdentityRefusal {
    /// The edited corpus's bench-axis bytes differ from the baseline's —
    /// the edit moved the axis the fixture bench measures and the A/B must
    /// run. Carries which half moved (atoms, healed) so the report names it.
    AxisMoved {
        atoms_equal: bool,
        healed_equal: bool,
    },
}

/// BLAKE3 digests of the bench-axis bytes, baseline corpus vs edited
/// corpus: the atom tables (pure functions of `buggy_code`) and the healed
/// output (compile-time dispatch). Computed by the caller over the two
/// corpus generations the edit touches; the referee only compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BenchIdentity {
    pub baseline_atoms: [u8; 32],
    pub baseline_healed: [u8; 32],
    pub edited_atoms: [u8; 32],
    pub edited_healed: [u8; 32],
}

impl BenchIdentity {
    /// Digest the two generations' bench-axis bytes (atoms, healed) — the
    /// ONE place the byte→digest mapping lives.
    #[must_use]
    pub fn new(
        baseline_atoms: &[u8],
        baseline_healed: &[u8],
        edited_atoms: &[u8],
        edited_healed: &[u8],
    ) -> Self {
        Self {
            baseline_atoms: blake3_digest(baseline_atoms),
            baseline_healed: blake3_digest(baseline_healed),
            edited_atoms: blake3_digest(edited_atoms),
            edited_healed: blake3_digest(edited_healed),
        }
    }

    /// The certificate read: both pairs equal = the edit provably did not
    /// touch the bench axis (the fixture A/B may be replaced by this
    /// record). Either pair unequal = the axis moved = the fixture bench
    /// runs (refuse loud, never a silent pass-through).
    ///
    /// # Errors
    ///
    /// [`BenchIdentityRefusal::AxisMoved`] when either digest pair differs.
    #[must_use = "a certificate that is never read proves nothing"]
    pub fn verify(&self) -> Result<(), BenchIdentityRefusal> {
        let atoms_equal = self.baseline_atoms == self.edited_atoms;
        let healed_equal = self.baseline_healed == self.edited_healed;
        if atoms_equal && healed_equal {
            Ok(())
        } else {
            Err(BenchIdentityRefusal::AxisMoved {
                atoms_equal,
                healed_equal,
            })
        }
    }
}

/// Digest bytes into the certificate's 32-byte commitment (BLAKE3, domain-
/// separated so a digest never equals a bare hash of unrelated bytes).
#[must_use]
pub fn blake3_digest(bytes: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"riir-refine/bench_identity/v1");
    h.update(bytes);
    *h.finalize().as_bytes()
}

// ── The referee view (decoupled from the durable store) ─────────────────────

/// The referee's decoupled view of one durable attempt row (cumulative
/// tallies per (rule, shape) + the row's decision-time component class and
/// freshness stamp). The durable-store type maps into this behind the
/// `latent_retrieval` gate; everything downstream is store-free.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptRow {
    /// The rule id the corpus keys on (the code half of the store's
    /// `"{code}:{strategy}"` key — the mapping half owns the split).
    pub rule_id: String,
    pub success_count: u32,
    pub fail_count: u32,
    /// The LAST decision-time class the row recorded — the stage evidence
    /// (the same attribution the diagnose reads).
    pub component: Option<ComponentClass>,
    /// Freshest activity stamp (unix seconds; 0 = never stamped).
    pub last_seen_at: u64,
}

impl AttemptRow {
    /// The row's total attempts (cumulative).
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.success_count + self.fail_count
    }
}

#[cfg(feature = "latent_retrieval")]
impl From<&crate::traj_store::TrajRowDoc> for AttemptRow {
    fn from(row: &crate::traj_store::TrajRowDoc) -> Self {
        Self {
            rule_id: row
                .lint_key
                .rsplit_once(':')
                .map_or_else(|| row.lint_key.clone(), |(code, _)| code.to_string()),
            success_count: row.success_count,
            fail_count: row.fail_count,
            component: row.component,
            last_seen_at: row.last_seen_at,
        }
    }
}

// ── The pinned constants (the pre-registration's numbers) ───────────────────

/// Minimum post-edit attempts before any read (the verdict stays PENDING
/// below it — never folded into a pass).
pub const MIN_POST_ATTEMPTS: u32 = 30;

/// The starvation kill window: observed corpus epochs (batches) after the
/// edit. The pre-registration: 8 batches ≈ a month of mining at the
/// observed cadence.
pub const STARVATION_BATCHES: u32 = 8;

/// The Bonferroni split of the 5 % mass across the two sides of the
/// difference bound: `P(p_pre` ≥ LCB) ≥ 0.975 and `P(p_post` ≤ UCB) ≥ 0.975
/// gives the difference a ≥ 95 % joint lower bound (union bound).
pub const LCB_SPLIT_EPSILON: f32 = 0.025;

// ── The fold (the referee's windows) ────────────────────────────────────────

/// One window's outcome tallies (the same bucket split the diagnose reads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowTally {
    pub parse_failures: u32,
    pub compile_failures: u32,
    pub verify_failures: u32,
    pub applied: u32,
}

impl WindowTally {
    /// The bridge from the diagnose fold's tallies (the same four buckets;
    /// one home so the referee and the diagnose NEVER drift apart).
    #[must_use]
    pub fn from_rule_outcomes(o: &crate::corpus_binding::RuleOutcomes) -> Self {
        Self {
            parse_failures: o.parse_failures,
            compile_failures: o.compile_failures,
            verify_failures: o.verify_failures,
            applied: o.applied,
        }
    }

    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.parse_failures + self.compile_failures + self.verify_failures + self.applied
    }

    #[must_use]
    pub fn failures(&self) -> u32 {
        self.parse_failures + self.compile_failures + self.verify_failures
    }

    /// The overall failure rate (all-cause fails / attempts) — the referee's
    /// metric (the stage SHARE stays the diagnose metric; this is the
    /// rule's failure rate full stop).
    #[must_use]
    pub fn failure_rate(&self) -> Option<f64> {
        let n = self.attempts();
        (n > 0).then(|| f64::from(self.failures()) / f64::from(n))
    }

    /// Bucket-wise saturating subtraction (current − snapshot). A bucket
    /// that would go negative means the snapshot population disagrees with
    /// the sweep — clamped, never a panic; the caller's disclosure note is
    /// the honest carrier (see the module bias note).
    #[must_use]
    pub fn saturating_sub(&self, pre: &Self) -> Self {
        Self {
            parse_failures: self.parse_failures.saturating_sub(pre.parse_failures),
            compile_failures: self.compile_failures.saturating_sub(pre.compile_failures),
            verify_failures: self.verify_failures.saturating_sub(pre.verify_failures),
            applied: self.applied.saturating_sub(pre.applied),
        }
    }

    /// The diagnose fold's attribution rule (one home — the bin's fold and
    /// the referee's windows share it): Gate-class fails land in the verify
    /// bucket, everything else (Drafter, Retrieval, unattributed) in the
    /// compile bucket; E-class corpus gaps never inflate a gate bucket.
    pub fn fold_row(&mut self, row: &AttemptRow) {
        let bucket = match row.component {
            Some(ComponentClass::Gate) => &mut self.verify_failures,
            _ => &mut self.compile_failures,
        };
        *bucket += row.fail_count;
        self.applied += row.success_count;
    }
}

/// Why a referee read produced no verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingReason {
    /// The post window has not reached [`MIN_POST_ATTEMPTS`] yet.
    BelowSupport {
        post_attempts: u32,
        batches_observed: u32,
    },
    /// The rule's post-edit evidence is outside the recent window (the
    /// vintage predicate gates the READ, not the enrollment).
    Vintage { last_seen_at: u64 },
    /// The pre side has no attempts — the "before" rate is undefined.
    NoPreEvidence,
}

/// The referee's verdict — the four pre-registered states, never pooled.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// LCB95 on the drop > 0 AND the drop exceeds the co-epoch control
    /// drift (the `DiD` gate): the note's effect is verified and attributable.
    Pass {
        lcb_drop: f64,
        drop: f64,
        control_drift: f64,
    },
    /// No verified drop: the LCB on the difference is ≤ 0, or the drop does
    /// not exceed the drift — both numbers printed, the report is the
    /// record.
    Fail {
        lcb_drop: f64,
        drop: f64,
        control_drift: f64,
    },
    /// No read yet — the reason names the missing evidence class.
    Pending { reason: PendingReason },
    /// UNDECIDED-by-starvation: [`STARVATION_BATCHES`] corpus epochs
    /// observed since the edit without reaching [`MIN_POST_ATTEMPTS`].
    Starved {
        post_attempts: u32,
        batches_observed: u32,
    },
}

impl Verdict {
    /// The verdict's one-word state (the report vocabulary).
    #[must_use]
    pub fn state(&self) -> &'static str {
        match self {
            Verdict::Pass { .. } => "PASS",
            Verdict::Fail { .. } => "FAIL",
            Verdict::Pending { .. } => "PENDING",
            Verdict::Starved { .. } => "UNDECIDED-BY-STARVATION",
        }
    }
}

/// One enrolled corpus-note edit (the referee's case record).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Enrollment {
    /// The rule the note landed on.
    pub rule: String,
    /// The corpus epoch the note landed under — the SHORT label the note
    /// itself carries (8 hex; the full digest is the compose gate's
    /// business, this is the human-facing identity).
    pub edit_epoch_short: String,
    /// Unix seconds at the edit — the split point.
    pub edit_ts: u64,
    /// The pre-edit tally snapshot (the diagnose-time values the note was
    /// raised from — case #1's live in the landed note text itself).
    pub pre: WindowTally,
    /// Corpus epochs observed since the edit (the caller's batch counter —
    /// the verb bumps it when the live corpus epoch changes between runs).
    pub batches_observed: u32,
    /// The freshest full corpus epoch seen at the last run (hex; None =
    /// never run — the first run seeds it without counting a batch).
    pub last_seen_epoch: Option<String>,
    /// The snapshot population disclosure (printed verbatim by the report —
    /// the bias note's honest carrier).
    pub snapshot_population: String,
}

/// The read inputs for one enrolled case (everything except the rule's own
/// windows: the caller folds the rows once per run).
#[derive(Clone, Copy, Debug)]
pub struct RefereeInputs<'a> {
    /// The rule's CURRENT total tallies (the merged sweep).
    pub rule_current: &'a WindowTally,
    /// The rule's freshest activity stamp (0 = none).
    pub rule_last_seen: u64,
    /// Control candidates: rules ≠ the enrolled rule with evidence on both
    /// sides of the edit (the fold's split — see [`split_windows`]).
    pub controls: &'a [(String, WindowTally, WindowTally)],
    /// "Now" for the vintage gate (unix seconds).
    pub now: u64,
    /// The vintage window floor (None = no vintage gate).
    pub recent_floor: Option<u64>,
}

/// Split a rule's rows into pre-edit (untouched since `edit_ts` — pure
/// pre-edit evidence) and post-window (updated after `edit_ts`) tallies.
///
/// Rows updated AFTER the edit carry mixed pre+post attempts; their post
/// side is contaminated by exactly the class the module bias note
/// discloses — the SAME class the enrolled rule's snapshot-subtraction
/// post side carries (pre-edit rows from later-swept stores), so the two
/// sides of the `DiD` read the same contamination class and the difference
/// stays meaningful as a drift estimate.
#[must_use]
pub fn split_windows(rows: &[AttemptRow], edit_ts: u64) -> (WindowTally, WindowTally) {
    let mut pre = WindowTally::default();
    let mut post = WindowTally::default();
    for row in rows {
        if row.last_seen_at != 0 && row.last_seen_at <= edit_ts {
            pre.fold_row(row);
        } else {
            // Unstamped rows (0) fold post-side: no evidence of pre-edit
            // activity is not evidence of pre-edit activity (the
            // conservative side for a DROP claim — they inflate the post
            // rate, never the pre rate).
            post.fold_row(row);
        }
    }
    (pre, post)
}

/// The deterministic median (sorted; even n takes the mean of the two
/// middles — pinned so a re-run reproduces bit-identically).
#[must_use]
fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(match values.len() {
        1 => values[0],
        even_len if even_len % 2 == 0 => {
            f64::midpoint(values[even_len / 2 - 1], values[even_len / 2])
        }
        _ => values[values.len() / 2],
    })
}

/// The read: the four-state verdict over one enrolled case.
#[must_use]
pub fn read(enrollment: &Enrollment, inputs: &RefereeInputs<'_>) -> Verdict {
    let post = inputs.rule_current.saturating_sub(&enrollment.pre);
    let post_attempts = post.attempts();

    // Starvation first: the kill window is terminal on the under-support
    // cases (the pre-registration: "does not reach n ≥ 30 within 8
    // batches"). Support met = the read proceeds.
    if enrollment.batches_observed >= STARVATION_BATCHES && post_attempts < MIN_POST_ATTEMPTS {
        return Verdict::Starved {
            post_attempts,
            batches_observed: enrollment.batches_observed,
        };
    }

    // Support floor: below it the verdict is PENDING, never folded.
    if post_attempts < MIN_POST_ATTEMPTS {
        return Verdict::Pending {
            reason: PendingReason::BelowSupport {
                post_attempts,
                batches_observed: enrollment.batches_observed,
            },
        };
    }

    // Vintage gates the READ (the writer lane's discipline): the rule must
    // have post-edit evidence inside the recent window.
    if let Some(floor) = inputs.recent_floor {
        if inputs.rule_last_seen != 0 && inputs.rule_last_seen < floor {
            return Verdict::Pending {
                reason: PendingReason::Vintage {
                    last_seen_at: inputs.rule_last_seen,
                },
            };
        }
    }

    let Some(pre_rate) = enrollment.pre.failure_rate() else {
        return Verdict::Pending {
            reason: PendingReason::NoPreEvidence,
        };
    };
    // The post side cannot be empty here (post_attempts ≥ 30 above).
    let post_rate = post.failure_rate().unwrap_or(0.0);
    let drop = pre_rate - post_rate;

    // The Beta-LCB95 on the drop (the Bonferroni split): LCB on the pre
    // failure rate (the failure COUNT is the Beta successes — the quantity
    // being lower-bounded) minus the UCB on the post failure rate.
    let lcb_pre = f64::from(best_belief_score(
        enrollment.pre.failures(),
        enrollment.pre.applied,
        LCB_SPLIT_EPSILON,
    ));
    let ucb_post = f64::from(best_belief_score(
        post.failures(),
        post.applied,
        1.0 - LCB_SPLIT_EPSILON,
    ));
    let lcb_drop = lcb_pre - ucb_post;

    // The DiD arm: the control set's median rate change over the same
    // split. Controls carry (pre-side, post-side) tallies from the fold;
    // a control with an empty side contributes nothing (filtered).
    let mut deltas: Vec<f64> = inputs
        .controls
        .iter()
        .filter_map(|(_, cpre, cpost)| Some(cpre.failure_rate()? - cpost.failure_rate()?))
        .collect();
    let control_drift = median(&mut deltas).unwrap_or(0.0);

    // PASS = the verified drop AND it exceeds the era drift.
    if lcb_drop > 0.0 && drop > control_drift {
        Verdict::Pass {
            lcb_drop,
            drop,
            control_drift,
        }
    } else {
        Verdict::Fail {
            lcb_drop,
            drop,
            control_drift,
        }
    }
}

// ── The enrollment store (machine-local, the verb's --enroll/--referee) ─────

/// The on-disk enrolled-case ledger (v1): `<repo>/.refine/
/// corpus_referee_enrollments.json`. Machine-local by design — the durable
/// stores it reads are machine-local too, so a referee verdict is a verdict
/// about THIS box's telemetry; the enrollment RECORD travels through git
/// (the issue's task tick + the commit that lands it), not through this
/// file.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnrollmentFile {
    pub format_version: u32,
    pub cases: Vec<Enrollment>,
}

impl EnrollmentFile {
    /// The current format version (bump on any wire change; loaders refuse
    /// unknown versions loud — never a silent reinterpretation).
    pub const FORMAT_VERSION: u32 = 1;

    /// The ledger's path under a repo root.
    #[must_use]
    pub fn path(repo_root: &std::path::Path) -> std::path::PathBuf {
        repo_root
            .join(".refine")
            .join("corpus_referee_enrollments.json")
    }

    /// Load from a repo's `.refine/` dir; a missing file is the empty
    /// ledger (first enrollment), an unparsable or unknown-version file
    /// refuses loud.
    ///
    /// # Errors
    /// A present-but-unparsable file or an unknown `format_version`.
    pub fn load(repo_root: &std::path::Path) -> Result<Self, String> {
        let path = Self::path(repo_root);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) if !path.exists() => {
                return Ok(Self {
                    format_version: Self::FORMAT_VERSION,
                    cases: Vec::new(),
                });
            }
            Err(e) => return Err(format!("{} unreadable: {e}", path.display())),
        };
        let file: Self = serde_json::from_str(&text)
            .map_err(|e| format!("{} unparsable: {e}", path.display()))?;
        if file.format_version != Self::FORMAT_VERSION {
            return Err(format!(
                "{}: format_version {} ≠ supported {} — bump or migrate, never reinterpret",
                path.display(),
                file.format_version,
                Self::FORMAT_VERSION
            ));
        }
        Ok(file)
    }

    /// Save atomically (temp file in the same dir, then rename).
    ///
    /// # Errors
    /// Any I/O failure (the caller reports; the verb exits loud).
    pub fn save(&self, repo_root: &std::path::Path) -> Result<(), String> {
        let path = Self::path(repo_root);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("enrollment ledger serialize: {e}"))?;
        let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(())
    }

    /// Upsert by rule id (one enrollment per rule — a re-enroll REPLACES;
    /// the enrollment record is the pre-edit snapshot, and there is exactly
    /// one edit to enroll).
    pub fn upsert(&mut self, case: Enrollment) {
        if let Some(slot) = self.cases.iter_mut().find(|c| c.rule == case.rule) {
            *slot = case;
        } else {
            self.cases.push(case);
        }
    }
}

// ── Tests (default features — the module is store-free) ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_binding::{CorpusBinding, GateStage};

    fn row(
        rule: &str,
        success: u32,
        fail: u32,
        component: Option<ComponentClass>,
        seen: u64,
    ) -> AttemptRow {
        AttemptRow {
            rule_id: rule.to_string(),
            success_count: success,
            fail_count: fail,
            component,
            last_seen_at: seen,
        }
    }

    fn tally(compile_fails: u32, applied: u32) -> WindowTally {
        WindowTally {
            compile_failures: compile_fails,
            applied,
            ..WindowTally::default()
        }
    }

    fn enrollment(pre_compile: u32, pre_applied: u32) -> Enrollment {
        Enrollment {
            rule: "r".to_string(),
            edit_epoch_short: "ac280021".to_string(),
            edit_ts: 1_000,
            pre: tally(pre_compile, pre_applied),
            batches_observed: 0,
            last_seen_epoch: None,
            snapshot_population: "test".to_string(),
        }
    }

    fn inputs<'a>(
        current: &'a WindowTally,
        controls: &'a [(String, WindowTally, WindowTally)],
    ) -> RefereeInputs<'a> {
        RefereeInputs {
            rule_current: current,
            rule_last_seen: 2_000,
            controls,
            now: 3_000,
            recent_floor: None,
        }
    }

    // ── The certificate (T1) ────────────────────────────────────────────

    #[test]
    fn bench_identity_equal_digests_verify_and_unequal_refuse() {
        let bytes = b"atoms-and-healed-output";
        let cert = BenchIdentity::new(bytes, bytes, bytes, bytes);
        assert!(cert.verify().is_ok(), "identical generations must certify");

        let moved = BenchIdentity::new(bytes, bytes, b"edited-atoms", bytes);
        assert_eq!(
            moved.verify(),
            Err(BenchIdentityRefusal::AxisMoved {
                atoms_equal: false,
                healed_equal: true,
            }),
            "an atom-table change must refuse loud naming the moved half"
        );
    }

    #[test]
    fn bench_identity_digest_is_domain_separated_and_deterministic() {
        let a = blake3_digest(b"x");
        assert_eq!(a, blake3_digest(b"x"), "re-run reproduces");
        // Not the bare BLAKE3 of the bytes: the domain separator shows.
        let bare = {
            let mut h = blake3::Hasher::new();
            h.update(b"x");
            *h.finalize().as_bytes()
        };
        assert_ne!(a, bare, "the certificate digest is domain-separated");
    }

    #[test]
    fn binding_carries_the_certificate_and_none_passes_through() {
        let binding = CorpusBinding::new("r", GateStage::CompileGate, [7u8; 32], 0);
        assert!(
            binding.verify_bench_identity().is_ok(),
            "no cert = the pre-209 shape, not an axis move"
        );
        let bytes = b"same";
        let certed = binding.with_bench_identity(BenchIdentity::new(bytes, bytes, bytes, bytes));
        assert!(certed.verify_bench_identity().is_ok());
        // The cert rides the digest: the same content WITHOUT the cert is a
        // different identity (v2), so build the moved record fresh.
        let moved = CorpusBinding::new("r", GateStage::CompileGate, [7u8; 32], 0)
            .with_bench_identity(BenchIdentity::new(bytes, bytes, b"other", bytes));
        assert!(moved.verify_bench_identity().is_err());
    }

    // ── The fold + split (T2) ───────────────────────────────────────────

    #[test]
    fn split_windows_separates_on_the_edit_stamp() {
        let rows = [
            row("r", 5, 5, None, 900),   // pre-edit row (untouched)
            row("r", 7, 3, None, 1_500), // updated after the edit
            row("r", 1, 0, None, 0),     // unstamped → post-side
        ];
        let (pre, post) = split_windows(&rows, 1_000);
        assert_eq!(
            pre.attempts(),
            10,
            "untouched rows are pure pre-edit evidence"
        );
        assert_eq!(
            post.attempts(),
            11,
            "updated + unstamped rows fold post-side"
        );
    }

    #[test]
    fn fold_attributes_gate_class_to_the_verify_bucket_only() {
        let mut t = WindowTally::default();
        t.fold_row(&row("r", 3, 2, Some(ComponentClass::Gate), 1));
        t.fold_row(&row("r", 3, 4, None, 1));
        assert_eq!(
            t.verify_failures, 2,
            "Gate-class fails land in the verify bucket"
        );
        assert_eq!(
            t.compile_failures, 4,
            "unattributed fails land in the compile bucket"
        );
        assert_eq!(t.applied, 6);
    }

    #[test]
    fn saturating_sub_clamps_a_disagreeing_snapshot() {
        let post = tally(9, 9).saturating_sub(&tally(20, 20));
        assert_eq!(post, WindowTally::default(), "clamp, never panic");
    }

    // ── The verdict state machine (T3) ──────────────────────────────────

    #[test]
    fn below_support_is_pending_with_the_reason() {
        let e = enrollment(9, 9); // pre rate 0.5
        // post = (11,13) − (9,9) = (2 fails, 4 applied) = 6 attempts.
        let v = read(&e, &inputs(&tally(11, 13), &[]));
        assert_eq!(
            v,
            Verdict::Pending {
                reason: PendingReason::BelowSupport {
                    post_attempts: 6,
                    batches_observed: 0,
                }
            },
            "n < 30 is PENDING, never folded into a pass"
        );
    }

    #[test]
    fn starvation_is_terminal_only_under_support() {
        let mut e = enrollment(9, 9);
        e.batches_observed = STARVATION_BATCHES;
        // Support met → the read proceeds (the kill window gates the
        // UNDER-support cases only).
        let v = read(&e, &inputs(&tally(11, 100), &[])); // post attempts 91 ≥ 30
        assert!(
            matches!(v, Verdict::Fail { .. } | Verdict::Pass { .. }),
            "support met: the kill window must not preempt the read, got {v:?}"
        );
        // Under support + window exhausted → the terminal starvation state.
        let v_low = read(&e, &inputs(&tally(11, 13), &[]));
        assert_eq!(
            v_low,
            Verdict::Starved {
                post_attempts: 6,
                batches_observed: STARVATION_BATCHES,
            },
            "8 batches without support is UNDECIDED-by-starvation"
        );
        assert_eq!(v_low.state(), "UNDECIDED-BY-STARVATION");
    }

    #[test]
    fn vintage_gate_holds_the_read_at_pending() {
        let e = enrollment(9, 9);
        let i = RefereeInputs {
            rule_current: &tally(11, 100), // support met
            rule_last_seen: 1_500,         // older than the floor
            controls: &[],
            now: 10_000,
            recent_floor: Some(9_000),
        };
        assert_eq!(
            read(&e, &i),
            Verdict::Pending {
                reason: PendingReason::Vintage {
                    last_seen_at: 1_500
                }
            },
            "the vintage predicate gates the READ, not the enrollment"
        );
    }

    #[test]
    fn no_pre_evidence_is_pending() {
        let e = enrollment(0, 0);
        let v = read(&e, &inputs(&tally(5, 100), &[]));
        assert_eq!(
            v,
            Verdict::Pending {
                reason: PendingReason::NoPreEvidence
            },
            "a rule with no pre-edit attempts has no 'before' rate"
        );
    }

    #[test]
    fn verified_drop_over_quiet_drift_passes() {
        let e = enrollment(18, 18); // pre: 18/36 = 0.5
        let controls = [(
            "c1".to_string(),
            tally(10, 10), // pre rate 0.5
            tally(11, 11), // post rate 0.5 → ΔC = 0.0 (quiet era)
        )];
        // current (20,66) → post (2, 50): fails 2 / attempts 50 = 0.04.
        let v = read(&e, &inputs(&tally(20, 66), &controls));
        let Verdict::Pass {
            lcb_drop,
            drop,
            control_drift,
        } = &v
        else {
            panic!("expected PASS, got {v:?}");
        };
        assert!(*lcb_drop > 0.0, "the LCB must verify the drop");
        assert!((*drop - 0.46).abs() < 1e-9);
        assert_eq!(*control_drift, 0.0);
        assert_eq!(v.state(), "PASS");
    }

    #[test]
    fn no_drop_fails_loud() {
        let e = enrollment(18, 18); // pre rate 0.5
        let Verdict::Fail {
            lcb_drop,
            drop,
            control_drift,
        } = read(&e, &inputs(&tally(33, 67), &[]))
        // post: 15/50 = 0.3
        else {
            panic!("expected FAIL");
        };
        assert!(drop > 0.0, "the point drop is positive (0.2)");
        assert!(
            lcb_drop <= 0.0,
            "…but n=50 post attempts at this delta cannot verify at LCB95"
        );
        assert_eq!(
            control_drift, 0.0,
            "no controls → drift defaults to 0 (printed)"
        );
    }

    #[test]
    fn a_drop_that_does_not_exceed_the_era_drift_fails_the_bar() {
        let e = enrollment(18, 18); // pre rate 0.5
        // The WHOLE era improved by ~0.175 — the control moved with the
        // rule, so the drop is not attributable to the note.
        let controls = [(
            "c1".to_string(),
            tally(20, 20), // pre rate 0.5
            tally(13, 27), // post rate 0.325 → ΔC = 0.175
        )];
        // current = pre + (13,27) → post = (13, 27): rate 13/40 = 0.325,
        // so the drop (0.175) EQUALS the control drift — `drop > drift`
        // fails, the DiD gate binds, verdict FAIL.
        let v = read(&e, &inputs(&tally(31, 45), &controls));
        let Verdict::Fail {
            drop,
            control_drift,
            ..
        } = v
        else {
            panic!("expected FAIL, got {v:?}");
        };
        assert!(
            drop <= control_drift,
            "drop {drop} must not exceed the drift {control_drift} for the DiD gate to bind"
        );
    }

    // ── The median (determinism) ────────────────────────────────────────

    #[test]
    fn median_is_deterministic_odd_even_and_empty() {
        assert_eq!(median(&mut []), None);
        assert_eq!(median(&mut [3.0]), Some(3.0));
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(
            median(&mut [4.0, 1.0, 2.0, 3.0]),
            Some(2.5),
            "even n = mean of middles"
        );
        assert_eq!(median(&mut [0.0, -1.0]), Some(-0.5));
    }

    // ── The enrollment store round-trip ─────────────────────────────────

    #[test]
    fn enrollment_file_round_trips_and_refuses_unknown_versions() {
        // pid-scoped scratch (the shared-temp-path law: never a fixed name).
        let dir =
            std::env::temp_dir().join(format!("corpus_referee_roundtrip_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");

        let mut file = EnrollmentFile::load(&dir).expect("missing file = empty ledger");
        assert_eq!(file.cases.len(), 0);
        file.upsert(enrollment(9, 9));
        file.save(&dir).expect("save");

        let reloaded = EnrollmentFile::load(&dir).expect("reload");
        assert_eq!(reloaded.format_version, EnrollmentFile::FORMAT_VERSION);
        assert_eq!(reloaded.cases.len(), 1);
        assert_eq!(reloaded.cases[0].rule, "r");

        // Upsert replaces by rule id, never duplicates.
        let mut again = reloaded;
        again.upsert(enrollment(20, 20));
        assert_eq!(again.cases.len(), 1);
        assert_eq!(again.cases[0].pre.compile_failures, 20);

        // An unknown version refuses loud.
        let bad = EnrollmentFile {
            format_version: 99,
            cases: Vec::new(),
        };
        bad.save(&dir).expect("save bad");
        assert!(
            EnrollmentFile::load(&dir).is_err(),
            "unknown version refuses"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    // ── The metric ──────────────────────────────────────────────────────

    #[test]
    fn failure_rate_is_none_on_an_empty_window() {
        assert_eq!(WindowTally::default().failure_rate(), None);
        assert_eq!(tally(1, 1).failure_rate(), Some(0.5));
    }
}
