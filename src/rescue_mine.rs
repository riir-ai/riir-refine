//! Issue 156 — the state-level rescue-trajectory mining filter (`FlyBy`
//! Algorithm 1, arXiv:2609.34327, distilled in katgpt-rs Research 609 §2.6).
//!
//! `FlyBy` mines STATES, not rules: retain a decision state `s` iff `0` of its
//! no-tool continuations succeed (`b0 = 0`) AND at least 3 of 4 tool-assisted
//! continuations succeed (`bd ≥ 3`) — a state-level measurement of "this is
//! a knowledge bottleneck with a known rescue". This module is the local
//! modelless translation, consumed by the arm-H lane
//! (`src/bin/fixer_regate.rs --arm H`, which already runs the PAIRED
//! continuations — the tallies are bookkeeping over them, never new
//! sampling) and surfaced as `--misses` ordering evidence.
//!
//! Vocabulary mapping (paper → this crate):
//! - *no-tool continuation* → the modelless arm's answer, scored by the
//!   same `keep_strict` instrument as every other lane (b0).
//! - *tool-assisted continuation* → the escalation chain's answer at its
//!   manifest depth (`seat_rerank` / `hosted_expert`), counted ONLY when the
//!   transport actually served — a stood-by-modelless leg is a no-tool
//!   continuation in disguise, and counting it as tool-assisted would
//!   dilute the rescue evidence.
//! - *shallowest sufficient depth* → the cheapest depth at which a rescue
//!   was ever demonstrated for the state (min over successful sightings).
//!
//! ## The augment-only law (Issue 156, verdict round 2)
//!
//! The network-oracle agreement VETO (cross-model, e.g. openthai) remains
//! the acceptance AUTHORITY. This b0/bd predicate is same-model
//! self-consistency — strictly weaker evidence — so it can only NARROW the
//! candidate set entering the veto lane, never widen acceptance. Everything
//! here is report/queue-ordering evidence; nothing admits anything.
//!
//! UNGATED like [`crate::frontier_report`]: pure arithmetic over counters,
//! store-free, so lib tests cover it at default features. The durable
//! tallies live on the [`crate::traj_store`] row (the Issue-105 journey
//! precedent: an orthogonal evidence axis that never touches the outcome
//! counters or the tier FSM — a modelless failure is evidence about the
//! STATE, never against the rule).

use crate::memory::EvidenceTier;

/// The knowledge-bottleneck certification bar for the probe-gated lane
/// (Plan 621 T5.2): the state's b0 side is KNOWLEDGE-LIKE only when the
/// Wilson upper bound on its accumulated no-tool pass-rate clears BELOW
/// this ε — a state at (2 attempts, 0 successes) is `Undetermined` (keep
/// probing, the escalation would be premature), while (16, 0) certifies at
/// `wilson_hi` ≈ 0.194 < 0.2. Declared, never asserted: the raw keep
/// predicate is unaffected; this bar only arms the probe-gated ADVICE.
pub const KB_EPSILON: f64 = 0.2;

/// The probe's two-sided 95% critical value (the substrate `probe()` docs'
/// spelling — the CI COMPUTATION is consumed from katgpt-core, never
/// copied; this is the named critical-value literal the consumers pass).
pub const PROBE_Z95: f64 = 1.959_963_984_540_054;

/// The probe-gated half of the rescue lane (Plan 621 Phase 5 T5.2): the
/// Phase-2 bottleneck classifier over the state's ACCUMULATED b0 tallies.
/// The raw keep predicate stays the truth table exactly as Issue 156 pinned
/// it (`b0 = 0 ∧ bd ≥ 3` — byte-identical, the G3 anchor); this lane adds
/// the attempt-count honesty the raw predicate lacks: a state whose no-tool
/// control failed twice is NOT the same evidence as one that failed
/// sixteen times, and spending a tool-assisted escalation on the former is
/// the waste Plan 621's gate exists to prevent. Composition, not
/// replacement: the class is ADVICE for the arm-H escalation ordering —
/// the augment-only law is unchanged (the agreement VETO stays the
/// acceptance authority; the probe can only narrow, never admit).
#[cfg(feature = "rescue_probe")]
pub mod probe_gate {
    use super::{KB_EPSILON, PROBE_Z95, StateRescueTally};
    use katgpt_core::state_probe::{classify, probe, ProbeClass, ProbeInput};

    /// The state's probe estimate over its ACCUMULATED no-tool leg:
    /// `pass_count = b0_successes`, `n = b0_attempts`, answers-not-tracked
    /// (the rescue tallies carry no answer histogram — the verdict reads
    /// only the Wilson bounds; `h_mm` is 0.0 by the kernel's own contract).
    #[must_use]
    pub fn estimate(t: &StateRescueTally) -> katgpt_core::state_probe::ProbeEstimate {
        probe(
            &ProbeInput {
                pass_count: t.b0_successes,
                n: t.b0_attempts,
                histogram: &[],
            },
            PROBE_Z95,
        )
    }

    /// The state's bottleneck class over the accumulated b0 leg — the
    /// arm-H escalation ADVICE: `KnowledgeLike` ⇒ escalate (the rescue is
    /// the only path); `Productive` ⇒ never (execution-reachable, V > 0);
    /// `Undetermined` ⇒ keep probing (the escalation would be premature
    /// spend).
    #[must_use]
    pub fn class(t: &StateRescueTally) -> ProbeClass {
        classify(&estimate(t), KB_EPSILON)
    }

    /// The probe-gated keep reading: the state is a rescue exemplar iff it
    /// certifies knowledge-like AND the raw predicate's demonstrated-rescue
    /// bar holds. The raw [`super::StateRescueTally::keep`] is UNCHANGED —
    /// this is the escalation-gate composition, never a replacement (the
    /// truth-table test pins the raw one byte-identical).
    #[must_use]
    pub fn keep_probe_gated(t: &StateRescueTally) -> bool {
        matches!(class(t), ProbeClass::KnowledgeLike) && t.keep()
    }

    /// The probe column's render cell for the `--misses` table: the class
    /// label with the b0 interval — `(advice | lo, hi | n)`. Empty for a
    /// state with no b0 attempts (the kernel's interval is undefined
    /// there — the honest cell is `-`).
    #[must_use]
    pub fn cell(t: &StateRescueTally) -> String {
        if t.b0_attempts == 0 {
            return "-".to_string();
        }
        let e = estimate(t);
        let cls = classify(&e, KB_EPSILON);
        let word = match cls {
            ProbeClass::KnowledgeLike => "KB",
            ProbeClass::Productive => "exec",
            ProbeClass::Undetermined => "undet",
        };
        format!("{word} [{:.2},{:.2}] n={}", e.wilson_lo, e.wilson_hi, t.b0_attempts)
    }
}

/// The tool-assisted success bar — `FlyBy`'s `bd ≥ 3` of 4 (a 3-of-4 rescue
/// majority; `(0, 2)` is REJECT: the rescue is undemonstrated).
pub const BD_KEEP_MIN: u32 = 3;

/// The `FlyBy` probe's per-arm continuation count (the paper's 4/4 design —
/// documentation + the honest denominator for yield math; the local tallies
/// accumulate across arm-H runs rather than sampling 4-at-once).
pub const FLYBY_CONTINUATIONS_PER_ARM: u32 = 4;

/// The escalation depth a rescue was demonstrated at. Rank order IS the
/// cost order (the ESC chain's two live depths); `None` (0) means no rescue
/// was ever demonstrated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RescueDepth {
    None,
    SeatRerank,
    HostedExpert,
}

impl RescueDepth {
    /// Stable `u8` for the durable row (0 = none — falsy, the row default).
    #[must_use]
    pub fn as_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::SeatRerank => 1,
            Self::HostedExpert => 2,
        }
    }

    /// Inverse of [`Self::as_u8`] (unknown values → `None` — an undepth'd
    /// tally never masquerades as a demonstrated rescue).
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::SeatRerank,
            2 => Self::HostedExpert,
            _ => Self::None,
        }
    }

    /// Parse an escalation manifest row's `target.kind` (the arm-H lane's
    /// dispatch axis). `None` for anything else — an unknown kind must not
    /// record as a tool-assisted attempt.
    #[must_use]
    pub fn from_kind(kind: &str) -> Option<Self> {
        match kind {
            "seat_rerank" => Some(Self::SeatRerank),
            "hosted_expert" => Some(Self::HostedExpert),
            _ => None,
        }
    }

    /// The render label (tables + summaries).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SeatRerank => "seat_rerank",
            Self::HostedExpert => "hosted_expert",
        }
    }
}

/// One state's accumulated `FlyBy` tallies. Pure counters — the durable row
/// ([`crate::traj_store::TrajRowDoc`]) carries the same five numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RescueTally {
    /// No-tool (modelless) continuations scored.
    pub b0_attempts: u32,
    /// No-tool continuations that verified (`keep_strict`) — ANY nonzero
    /// value rejects the state (execution-reachable, `FlyBy`'s V>0 law).
    pub b0_successes: u32,
    /// Tool-assisted (escalated, transport-served) continuations scored.
    pub bd_attempts: u32,
    /// Tool-assisted continuations that verified — the rescue evidence.
    pub bd_successes: u32,
    /// Shallowest sufficient depth (`RescueDepth::as_u8`); 0 = none yet.
    pub depth: u8,
}

impl RescueTally {
    /// Record one no-tool continuation's outcome.
    pub fn observe_modelless(&mut self, success: bool) {
        self.b0_attempts = self.b0_attempts.saturating_add(1);
        self.b0_successes = self.b0_successes.saturating_add(u32::from(success));
    }

    /// Record one tool-assisted continuation's outcome. A [`RescueDepth::None`]
    /// depth is ignored entirely (the transport never served — there was no
    /// tool-assisted leg to count). The depth records as the SHALLOWEST
    /// sufficient depth: only successful sightings update it, and only
    /// downward.
    pub fn observe_escalated(&mut self, success: bool, depth: RescueDepth) {
        if matches!(depth, RescueDepth::None) {
            return;
        }
        self.bd_attempts = self.bd_attempts.saturating_add(1);
        self.bd_successes = self.bd_successes.saturating_add(u32::from(success));
        if success && (self.depth == 0 || depth.as_u8() < self.depth) {
            self.depth = depth.as_u8();
        }
    }

    /// The `FlyBy` keep predicate: `b0 = 0 ∧ bd ≥ 3`.
    #[must_use]
    pub fn keep(&self) -> bool {
        rescue_keep(self.b0_successes, self.bd_successes)
    }
}

/// The `FlyBy` rescue predicate on success COUNTS: zero no-tool successes AND
/// at least [`BD_KEEP_MIN`] tool-assisted successes. The G1 truth table
/// (`rescue_truth_table_pins_flyby_predicate`) pins the exact shape:
/// `(0,4)/(0,3)` keep · `(1,4)` reject (the control succeeded —
/// execution-reachable, V>0) · `(0,2)` reject (rescue undemonstrated).
#[must_use]
#[allow(clippy::similar_names)] // b0/bd ARE the FlyBy vocabulary (Research 609 §2.6) — the near-identical names are the domain's own, not a slip
pub fn rescue_keep(b0_successes: u32, bd_successes: u32) -> bool {
    b0_successes == 0 && bd_successes >= BD_KEEP_MIN
}

/// One state's rescue evidence as the reports consume it — the projection of
/// a durable [`crate::traj_store::TrajRowDoc`] (or a run-local tally) onto
/// this module's ungated vocabulary. `state_id` is the store's row identity
/// (`lint_key:shape_hash12`).
#[derive(Clone, Debug, PartialEq)]
pub struct StateRescueTally {
    pub state_id: String,
    pub b0_attempts: u32,
    pub b0_successes: u32,
    pub bd_attempts: u32,
    pub bd_successes: u32,
    /// `RescueDepth::as_u8` — 0 when no rescue was demonstrated.
    pub depth: u8,
    /// The row's evidence tier (the exemplar's certification state).
    pub tier: EvidenceTier,
}

impl StateRescueTally {
    /// The `FlyBy` keep predicate over this state's tallies.
    #[must_use]
    pub fn keep(&self) -> bool {
        rescue_keep(self.b0_successes, self.bd_successes)
    }
}

/// The state's band class — the `--misses` ordering vocabulary (Issue 156
/// T3). Rescue-rich states are the learnable-hard band's head; `bd = 0`
/// states are intractable-eviction candidates (`RegretMemory`'s absorbing
/// class); a state whose no-tool control EVER succeeded is execution-
/// reachable and never a rescue candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RescueClass {
    /// The `FlyBy` keep-set (`b0=0 ∧ bd≥3`) — rescue exemplars.
    Rescue,
    /// `b0=0` with the rescue demonstrated below the ≥3 bar.
    LearnableHard,
    /// `bd=0` — escalation never rescued; eviction CANDIDATES only.
    Intractable,
    /// No escalated leg has ever served for this state.
    Unprobed,
    /// The no-tool control succeeded at least once (V>0) — never a
    /// knowledge bottleneck by `FlyBy`'s own law.
    ExecReachable,
}

impl RescueClass {
    /// The queue rank (rescue exemplars first — they are the learnable-hard
    /// band's head; exec-reachable last — nothing to mine there).
    #[must_use]
    pub fn rank(self) -> u8 {
        match self {
            Self::Rescue => 0,
            Self::LearnableHard => 1,
            Self::Intractable => 2,
            Self::Unprobed => 3,
            Self::ExecReachable => 4,
        }
    }

    /// The render label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Rescue => "rescue",
            Self::LearnableHard => "learnable-hard",
            Self::Intractable => "intractable-evict",
            Self::Unprobed => "unprobed",
            Self::ExecReachable => "exec-reachable",
        }
    }
}

/// Classify one state. Order matters and is the predicate itself: keep-set
/// first, then never-escalated, then escalation-never-succeeded, then the
/// b0=0 rescue-below-bar band, then execution-reachable.
#[must_use]
pub fn classify(t: &StateRescueTally) -> RescueClass {
    match (
        t.keep(),
        t.bd_attempts,
        t.bd_successes,
        t.b0_successes,
    ) {
        (true, _, _, _) => RescueClass::Rescue,
        (_, 0, _, _) => RescueClass::Unprobed,
        (_, _, 0, _) => RescueClass::Intractable,
        (_, _, _, 0) => RescueClass::LearnableHard,
        _ => RescueClass::ExecReachable,
    }
}

/// The section summary: class counts + the keep-set hash.
#[derive(Clone, Debug, PartialEq)]
pub struct StateRescueSummary {
    pub population: usize,
    pub keep: usize,
    pub learnable_hard: usize,
    pub intractable: usize,
    pub unprobed: usize,
    pub exec_reachable: usize,
    /// BLAKE3 over the sorted kept state ids (`id` + `\n` each) — same
    /// transcripts → identical hash, in any observation order (T5).
    pub keep_hash: String,
}

/// Summarize a state population (deterministic; the keep-set hash is order-
/// independent by construction).
#[must_use]
pub fn summarize_states(states: &[StateRescueTally]) -> StateRescueSummary {
    let mut summary = StateRescueSummary {
        population: states.len(),
        keep: 0,
        learnable_hard: 0,
        intractable: 0,
        unprobed: 0,
        exec_reachable: 0,
        keep_hash: String::new(),
    };
    let mut kept: Vec<&str> = Vec::new();
    for t in states {
        match classify(t) {
            RescueClass::Rescue => {
                summary.keep += 1;
                kept.push(t.state_id.as_str());
            }
            RescueClass::LearnableHard => summary.learnable_hard += 1,
            RescueClass::Intractable => summary.intractable += 1,
            RescueClass::Unprobed => summary.unprobed += 1,
            RescueClass::ExecReachable => summary.exec_reachable += 1,
        }
    }
    kept.sort_unstable();
    let mut h = blake3::Hasher::new();
    for id in kept {
        h.update(id.as_bytes());
        h.update(b"\n");
    }
    summary.keep_hash = h.finalize().to_hex().to_string();
    summary
}

/// BLAKE3 keep-set hash over kept state ids — the T5 determinism pin, public
/// so the arm-H lane and the queue renderer share ONE derivation.
#[must_use]
pub fn keep_set_hash(state_ids: &[&str]) -> String {
    let mut ids: Vec<&str> = state_ids.to_vec();
    ids.sort_unstable();
    let mut h = blake3::Hasher::new();
    for id in ids {
        h.update(id.as_bytes());
        h.update(b"\n");
    }
    h.finalize().to_hex().to_string()
}

/// The evidence tier's render label (the `TrajectoryExemplar` mapping).
#[must_use]
pub fn tier_label(t: EvidenceTier) -> &'static str {
    match t {
        EvidenceTier::Certified => "Certified",
        EvidenceTier::Heuristic => "Heuristic",
        EvidenceTier::Withdrawn => "Withdrawn",
    }
}

/// Render the `--misses` rescue-band section (Issue 156 T3, report-only —
/// house style: `String` + `fmt::Write`, deterministic, an empty population
/// renders its loud marker). Ordering: rescue exemplars (the learnable-hard
/// band's head) → learnable-hard → intractable-eviction candidates →
/// unprobed → exec-reachable; within a class, rescue evidence depth first
/// (`bd` desc), then state id. The band column is the ESCALATED axis read
/// through the existing frontier band machinery (0 < `p̂_d` ≤ 0.5 = the
/// hard-but-learnable band on the tool axis too).
#[must_use]
pub fn render_misses_section(states: &[StateRescueTally]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(384 + states.len() * 112);

    if states.is_empty() {
        let _ = writeln!(
            out,
            "# rescue — state-level rescue band (Issue 156, FlyBy b0=0 ∧ bd≥3; report-only)\n\
             (empty: no state carries rescue tallies — fixer-regate --arm H mines them)\n"
        );
        return out;
    }

    let summary = summarize_states(states);
    let _ = writeln!(
        out,
        "# rescue — state-level rescue band (Issue 156, FlyBy b0=0 ∧ bd≥3; report-only)\n"
    );
    let _ = writeln!(
        out,
        "population: {pop} state(s) with rescue tallies | keep-set (rescue exemplars): {keep} \
         | learnable-hard: {lh} | intractable-eviction candidates: {itr} | unprobed: {up} \
         | exec-reachable: {er}",
        pop = summary.population,
        keep = summary.keep,
        lh = summary.learnable_hard,
        itr = summary.intractable,
        up = summary.unprobed,
        er = summary.exec_reachable,
    );
    let _ = writeln!(
        out,
        "keep-set hash: {} (same transcripts → identical)",
        &summary.keep_hash[..12],
    );

    // The escalated-axis band through the EXISTING frontier machinery — one
    // band classifier in the crate (kept = bd successes, reverted = bd
    // non-successes).
    let entries: Vec<(String, u32, u32)> = states
        .iter()
        .map(|s| {
            (
                s.state_id.clone(),
                s.bd_successes,
                s.bd_attempts.saturating_sub(s.bd_successes),
            )
        })
        .collect();
    let frontier = crate::frontier_report::summarize(&entries);
    let band_of: std::collections::BTreeMap<&str, &str> = frontier
        .rows
        .iter()
        .map(|r| (r.rule.as_str(), r.class.label()))
        .collect();

    let mut ordered: Vec<&StateRescueTally> = states.iter().collect();
    ordered.sort_by(|a, b| {
        classify(a)
            .rank()
            .cmp(&classify(b).rank())
            .then(b.bd_successes.cmp(&a.bd_successes))
            .then(a.state_id.cmp(&b.state_id))
    });

    render_misses_table(&mut out, &ordered, &band_of);

    let _ = writeln!(
        out,
        "\n(class: rescue = the FlyBy keep-set — the learnable-hard band's head; \
         learnable-hard = b0=0 with the rescue demonstrated below the ≥{BD_KEEP_MIN} bar; \
         intractable-evict = bd=0, RegretMemory's absorbing class — eviction CANDIDATES only; \
         unprobed = no escalated leg has served yet; exec-reachable = the no-tool control \
         succeeded (V>0) — never a rescue candidate. AUGMENT-ONLY: this band narrows the \
         candidate set entering the agreement VETO — the veto stays the acceptance authority.)\n"
    );
    #[cfg(feature = "rescue_probe")]
    let _ = writeln!(
        out,
        "(probe b0 = the Plan-621 T5.2 escalation ADVICE over the accumulated no-tool leg: \
         KB = knowledge-like at ε = {KB_EPSILON} — the rescue is the only path; exec = \
         productive — the control succeeded, never escalate; undet = more b0 evidence \
         needed before the escalation is worth its spend. Advice only: the raw keep \
         predicate and the augment-only law are unchanged.)\n"
    );
    out
}

/// The rank table (header + one row per state, the Issue-156 order already
/// applied by the caller). The probe-gated b0-advice column (Plan 621 T5.2,
/// feature `rescue_probe`) rides the ON posture; the OFF posture's table is
/// byte-identical to its pre-T5.2 shape (G3).
fn render_misses_table(
    out: &mut String,
    ordered: &[&StateRescueTally],
    band_of: &std::collections::BTreeMap<&str, &str>,
) {
    use std::fmt::Write as _;
    #[cfg(feature = "rescue_probe")]
    out.push_str(
        "| rank | state | tier | b0 suc/att | bd suc/att | depth | p̂_d | band | class | probe b0 |\n\
         |---|---|---|---|---|---|---|---|---|---|\n",
    );
    #[cfg(not(feature = "rescue_probe"))]
    out.push_str(
        "| rank | state | tier | b0 suc/att | bd suc/att | depth | p̂_d | band | class |\n\
         |---|---|---|---|---|---|---|---|---|\n",
    );
    for (i, s) in ordered.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)] // rate over u32 counters — the report's quantity
        let p_hat = if s.bd_attempts == 0 {
            0.0
        } else {
            s.bd_successes as f32 / s.bd_attempts as f32
        };
        #[cfg(feature = "rescue_probe")]
        let probe_cell = probe_gate::cell(s);
        #[cfg(feature = "rescue_probe")]
        let _ = writeln!(
            out,
            "| {} | {} | {} | {}/{} | {}/{} | {} | {p_hat:.3} | {} | {} | {} |",
            i + 1,
            s.state_id,
            tier_label(s.tier),
            s.b0_successes,
            s.b0_attempts,
            s.bd_successes,
            s.bd_attempts,
            RescueDepth::from_u8(s.depth).label(),
            band_of.get(s.state_id.as_str()).copied().unwrap_or("-"),
            classify(s).label(),
            probe_cell,
        );
        #[cfg(not(feature = "rescue_probe"))]
        let _ = writeln!(
            out,
            "| {} | {} | {} | {}/{} | {}/{} | {} | {p_hat:.3} | {} | {} |",
            i + 1,
            s.state_id,
            tier_label(s.tier),
            s.b0_successes,
            s.b0_attempts,
            s.bd_successes,
            s.bd_attempts,
            RescueDepth::from_u8(s.depth).label(),
            band_of.get(s.state_id.as_str()).copied().unwrap_or("-"),
            classify(s).label(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::similar_names)] // b0/bd ARE the FlyBy vocabulary — the test helper's parameter names carry the domain
    fn state(id: &str, b0_s: u32, b0_a: u32, bd_s: u32, bd_a: u32) -> StateRescueTally {
        StateRescueTally {
            state_id: id.to_string(),
            b0_attempts: b0_a,
            b0_successes: b0_s,
            bd_attempts: bd_a,
            bd_successes: bd_s,
            depth: u8::from(bd_s > 0),
            tier: EvidenceTier::Heuristic,
        }
    }

    /// T5 — the G1 truth table, pinned exactly as the issue states it.
    #[test]
    fn rescue_truth_table_pins_flyby_predicate() {
        // KEEP — zero no-tool successes AND ≥3 tool-assisted successes.
        assert!(rescue_keep(0, 4));
        assert!(rescue_keep(0, 3));
        // REJECT — the control succeeded once: execution-reachable (V>0).
        assert!(!rescue_keep(1, 4));
        // REJECT — the rescue is undemonstrated (2 < 3).
        assert!(!rescue_keep(0, 2));
        // The rest of the boundary, pinned so the shape stays exact.
        assert!(!rescue_keep(0, 0));
        assert!(!rescue_keep(0, 1));
        assert!(!rescue_keep(2, 4));
        assert!(!rescue_keep(4, 4));
        // The RescueTally method is the same predicate.
        let mut t = RescueTally::default();
        assert!(!t.keep());
        for _ in 0..4 {
            t.observe_modelless(false);
            t.observe_escalated(true, RescueDepth::SeatRerank);
        }
        assert!(t.keep());
    }

    /// T5 — same transcripts → identical keep-set hash, in any order.
    #[test]
    fn keep_set_hash_is_transcript_deterministic() {
        let a = ["s2", "s1", "s3"];
        let b = ["s1", "s3", "s2"];
        assert_eq!(keep_set_hash(&a), keep_set_hash(&b));
        assert_eq!(keep_set_hash(&a), keep_set_hash(&a));
        assert_ne!(keep_set_hash(&a), keep_set_hash(&["s1", "s3"]));
        assert_ne!(keep_set_hash(&a), keep_set_hash(&["s1", "s2", "s4"]));
        // The empty set is deterministic too (never confusable with a set).
        assert_eq!(keep_set_hash(&[]), keep_set_hash(&[]));
        assert_ne!(keep_set_hash(&[]), keep_set_hash(&["x"]));
        // summarize_states derives the SAME hash for the same kept set.
        let states = vec![
            state("k1", 0, 4, 3, 4),
            state("k2", 0, 4, 4, 4),
            state("r", 1, 4, 4, 4), // exec-reachable, not kept
        ];
        let summary = summarize_states(&states);
        assert_eq!(summary.keep, 2);
        assert_eq!(summary.keep_hash, keep_set_hash(&["k1", "k2"]));
    }

    /// The accumulation law: shallowest-sufficient depth, the ≥3 bar, and
    /// the V>0 flip (one modelless success un-keeps the state forever).
    #[test]
    fn tallies_accumulate_toward_the_keep_predicate() {
        let mut t = RescueTally::default();
        for _ in 0..4 {
            t.observe_modelless(false);
        }
        t.observe_escalated(true, RescueDepth::HostedExpert);
        t.observe_escalated(true, RescueDepth::HostedExpert);
        t.observe_escalated(false, RescueDepth::HostedExpert);
        assert_eq!(t.bd_attempts, 3);
        assert_eq!(t.bd_successes, 2);
        assert!(!t.keep(), "2 < 3 — rescue undemonstrated");
        t.observe_escalated(true, RescueDepth::SeatRerank);
        assert_eq!(
            t.depth,
            RescueDepth::SeatRerank.as_u8(),
            "the shallowest depth at which a rescue succeeded wins"
        );
        assert!(t.keep());
        // A deeper success never moves the shallowest-sufficient depth.
        t.observe_escalated(true, RescueDepth::HostedExpert);
        assert_eq!(t.depth, RescueDepth::SeatRerank.as_u8());
        // One modelless success flips the class: execution-reachable (V>0).
        t.observe_modelless(true);
        assert!(!t.keep());
        // A stood-by-modelless leg is NOT a tool-assisted attempt.
        let mut u = RescueTally::default();
        u.observe_escalated(true, RescueDepth::None);
        assert_eq!((u.bd_attempts, u.bd_successes, u.depth), (0, 0, 0));
    }

    /// The class vocabulary and its queue rank (T3's ordering law).
    #[test]
    fn classify_covers_the_state_vocabulary() {
        assert_eq!(classify(&state("a", 0, 4, 4, 4)), RescueClass::Rescue);
        assert_eq!(classify(&state("b", 0, 4, 3, 3)), RescueClass::Rescue);
        assert_eq!(classify(&state("c", 0, 4, 1, 2)), RescueClass::LearnableHard);
        assert_eq!(classify(&state("d", 0, 4, 0, 3)), RescueClass::Intractable);
        assert_eq!(classify(&state("e", 0, 4, 0, 0)), RescueClass::Unprobed);
        assert_eq!(classify(&state("f", 1, 4, 4, 4)), RescueClass::ExecReachable);
        // Rank order: rescue first, exec-reachable last.
        assert!(
            RescueClass::Rescue.rank() < RescueClass::LearnableHard.rank()
                && RescueClass::LearnableHard.rank() < RescueClass::Intractable.rank()
                && RescueClass::Intractable.rank() < RescueClass::Unprobed.rank()
                && RescueClass::Unprobed.rank() < RescueClass::ExecReachable.rank()
        );
    }

    /// The depth vocabulary round-trips and parses the manifest kinds.
    #[test]
    fn rescue_depth_roundtrips_and_refuses_unknown_kinds() {
        for d in [
            RescueDepth::None,
            RescueDepth::SeatRerank,
            RescueDepth::HostedExpert,
        ] {
            assert_eq!(RescueDepth::from_u8(d.as_u8()), d);
        }
        assert_eq!(RescueDepth::from_u8(9), RescueDepth::None);
        assert_eq!(RescueDepth::from_kind("seat_rerank"), Some(RescueDepth::SeatRerank));
        assert_eq!(RescueDepth::from_kind("hosted_expert"), Some(RescueDepth::HostedExpert));
        assert_eq!(RescueDepth::from_kind("modelless"), None);
        assert_eq!(RescueDepth::from_kind(""), None);
    }

    /// The `--misses` section: loud empty marker, deterministic order
    /// (rescue exemplars head the table), the band column present.
    #[test]
    fn misses_section_renders_rescue_first_and_empty_loud() {
        let empty = render_misses_section(&[]);
        assert!(empty.contains("empty"), "the loud empty marker: {empty}");

        let states = vec![
            state("zz_reachable", 1, 4, 4, 4),
            state("aa_rescue", 0, 4, 3, 4),
            state("mm_hard", 0, 3, 1, 2),
            state("bb_intractable", 0, 2, 0, 2),
            state("cc_unprobed", 0, 1, 0, 0),
        ];
        let rendered = render_misses_section(&states);
        // Deterministic across two renders (same transcripts).
        assert_eq!(rendered, render_misses_section(&states));
        // The rescue exemplar ranks first; exec-reachable last.
        let aa = rendered.find("| 1 | aa_rescue").expect("aa_rescue at rank 1");
        let zz = rendered.find("zz_reachable").expect("zz present");
        assert!(aa < zz, "rescue exemplar before exec-reachable");
        assert!(rendered.contains("keep-set hash:"), "{rendered}");
        assert!(rendered.contains("AUGMENT-ONLY"), "{rendered}");
        assert!(rendered.contains("| band |"), "band column: {rendered}");
        // The population line carries every class count.
        for needle in [
            "keep-set (rescue exemplars): 1",
            "learnable-hard: 1",
            "intractable-eviction candidates: 1",
            "unprobed: 1",
            "exec-reachable: 1",
        ] {
            assert!(rendered.contains(needle), "{needle} missing: {rendered}");
        }
    }

    /// Plan 621 T5.2 — the probe-gated escalation advice over the
    /// ACCUMULATED b0 leg (feature `rescue_probe`; the raw predicate's own
    /// truth table above is posture-independent and stays byte-identical).
    #[cfg(feature = "rescue_probe")]
    mod probe_gate {
        use super::{state, super::probe_gate};

        #[test]
        fn enough_zero_success_attempts_certify_knowledge_like() {
            // 20 no-tool attempts, 0 successes: wilson_hi ≈ 0.161 < 0.2.
            let t = state("kb", 0, 20, 3, 4);
            assert!(matches!(
                probe_gate::class(&t),
                katgpt_core::state_probe::ProbeClass::KnowledgeLike
            ));
            // And the probe-gated keep agrees with the raw keep here.
            assert!(t.keep());
            assert!(probe_gate::keep_probe_gated(&t));
        }

        #[test]
        fn thin_evidence_reads_undetermined_where_the_raw_predicate_reads_keep() {
            // THE disclosure the probe adds: (2 attempts, 0 successes) is
            // keep-eligible under the raw count predicate but the Wilson
            // upper bound (≈ 0.78) is nowhere near the 0.2 bar — escalating
            // on this state is premature spend. Both readings stand; the
            // probe-gated one is the ADVICE.
            let t = state("thin", 0, 2, 3, 4);
            assert!(t.keep(), "raw predicate unchanged (the truth table)");
            assert!(matches!(
                probe_gate::class(&t),
                katgpt_core::state_probe::ProbeClass::Undetermined
            ));
            assert!(!probe_gate::keep_probe_gated(&t));
        }

        #[test]
        fn one_success_blocks_the_knowledge_certification() {
            // V > 0 evidence: a single control success in 20 keeps the
            // interval over the bar (hi ≈ 0.21) — undetermined, never KB.
            let t = state("vpos", 1, 20, 3, 4);
            assert!(matches!(
                probe_gate::class(&t),
                katgpt_core::state_probe::ProbeClass::Undetermined
            ));
            assert!(!probe_gate::keep_probe_gated(&t));
        }

        #[test]
        fn saturated_control_is_productive_never_kb() {
            let t = state("prod", 20, 20, 4, 4);
            assert!(matches!(
                probe_gate::class(&t),
                katgpt_core::state_probe::ProbeClass::Productive
            ));
            assert!(!probe_gate::keep_probe_gated(&t));
        }

        #[test]
        fn empty_b0_leg_renders_the_honest_dash_and_reads_undetermined() {
            let t = state("never", 0, 0, 0, 0);
            assert_eq!(probe_gate::cell(&t), "-");
            assert!(matches!(
                probe_gate::class(&t),
                katgpt_core::state_probe::ProbeClass::Undetermined
            ));
        }

        #[test]
        fn the_probe_column_renders_with_the_table() {
            let states = vec![state("kb", 0, 20, 3, 4), state("thin", 0, 2, 3, 4)];
            let rendered = super::super::render_misses_section(&states);
            assert!(rendered.contains("| probe b0 |"), "header: {rendered}");
            assert!(rendered.contains("KB ["), "kb cell: {rendered}");
            assert!(rendered.contains("undet ["), "thin cell: {rendered}");
            assert!(rendered.contains("ADVICE"), "the augment-only legend: {rendered}");
        }
    }
}
