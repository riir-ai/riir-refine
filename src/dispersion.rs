//! Issue 072 T4 — dispersion / frontier selection (J-Zero M3) + T7's
//! standing non-vacuity telemetry.
//!
//! T39/Research 135 measured the healer's selection bottleneck as EVIDENCE
//! VOLUME, not selection math: pools are starved, so any two selectors
//! differ by ≤0.04pp. The paper's (arXiv:2608.26582) contribution this
//! module ports is the frontier axis that decides WHERE to spend the next
//! unit of evidence: per candidate pool, the ENTROPY of observed outcome
//! classes is a closed-form dispersion proxy —
//!
//! - high entropy (mixed outcomes) → informative: front of the authoring /
//!   verification queue (resolving it converts many uncertain candidates);
//! - zero entropy, all-success → saturated: retire / escalate;
//! - zero entropy, all-failure → the candidate family is wrong: regenerate
//!   (simpler fixture, different strategy).
//!
//! Zero entropy for a whole window also flags the LOOP vacuous (T7): a
//! self-evolution loop emitting no dispersion is tuning selection math over
//! noise — the exact T39 finding, now a standing tripwire instead of a
//! one-off measurement.
//!
//! Pure math, feature-free: the driver (`--fix-compile`), the store
//! (`stats_report`), and any future consumer each map their own outcome
//! records onto [`OutcomeClass`] and call [`outcome_entropy`].

use std::collections::BTreeMap;

/// The three observable outcomes for one candidate attempt at one span
/// (the `--fix-compile` driver's own record classes: `applied` = kept,
/// `reverted` = tried-and-failed, `misses` = no candidate proposed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutcomeClass {
    /// Fix applied and kept (compiles clean).
    Kept,
    /// Fix tried and reverted (did not resolve).
    Reverted,
    /// No candidate proposed at all (decline).
    Missed,
}

impl OutcomeClass {
    /// All classes in stable order — the fixed-size histogram index.
    pub const ALL: [Self; 3] = [Self::Kept, Self::Reverted, Self::Missed];

    /// Histogram index (the class table is fixed-size, G4: no alloc).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Kept => 0,
            Self::Reverted => 1,
            Self::Missed => 2,
        }
    }

    /// Stable label (telemetry / logs).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Kept => "kept",
            Self::Reverted => "reverted",
            Self::Missed => "missed",
        }
    }
}

/// Shannon entropy (bits) over the observed outcome classes. Empty input
/// answers 0.0 (no evidence = no dispersion — the vacuous flag, not
/// "informative").
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // counts ≤ usize::MAX over a 3-class table; entropy precision is irrelevant at these magnitudes
pub fn outcome_entropy(outcomes: &[OutcomeClass]) -> f64 {
    if outcomes.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 3];
    for o in outcomes {
        counts[o.index()] += 1;
    }
    let n = outcomes.len() as f64;
    let mut h = 0.0;
    for c in counts {
        if c == 0 {
            continue;
        }
        let p = f64::from(c as u32) / n;
        h -= p * p.log2();
    }
    h
}

/// The three dispersion regimes (the control law).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Mixed outcomes — resolving this pool converts uncertain candidates.
    Informative,
    /// All outcomes identical AND all successful — nothing left to learn.
    Saturated,
    /// All outcomes identical AND all failures — the family is wrong;
    /// regenerate simpler.
    Regenerate,
    /// No observations — not frontable either way.
    Empty,
}

/// Classify a pool's dispersion regime (the control law from the issue).
#[must_use]
pub fn regime_of(outcomes: &[OutcomeClass]) -> Regime {
    if outcomes.is_empty() {
        return Regime::Empty;
    }
    let h = outcome_entropy(outcomes);
    if h > f64::EPSILON {
        return Regime::Informative;
    }
    if outcomes.iter().all(|o| *o == OutcomeClass::Kept) {
        Regime::Saturated
    } else {
        Regime::Regenerate
    }
}

/// One candidate pool entry for frontier ordering: an id + its observed
/// outcomes so far.
#[derive(Debug, Clone)]
pub struct PoolEntry {
    pub id: String,
    pub outcomes: Vec<OutcomeClass>,
}

impl PoolEntry {
    /// A new entry with the given id and (possibly empty) history.
    #[must_use]
    pub fn new(id: impl Into<String>, outcomes: Vec<OutcomeClass>) -> Self {
        Self { id: id.into(), outcomes }
    }
}

/// The frontier verdict for one entry — the ordering key AND the queue
/// action, so a consumer can print why each item is where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct FrontierVerdict {
    pub id: String,
    pub entropy: f64,
    pub regime: Regime,
}

/// Frontier-order a candidate pool: informative (high entropy) first, then
/// empty (no evidence — still worth one probe), then regenerate, saturated
/// last. Ties break by id (deterministic; the corpus house rule).
///
/// Zero-allocation on the hot path is NOT claimed here — the sort owns a
/// Vec of verdicts by design (a cold-path authoring queue, not the fix
/// loop); the G4 that matters is that [`outcome_entropy`] and
/// [`regime_of`] are O(n) over a fixed-size class table with no alloc.
#[must_use]
pub fn frontier_order(pool: &[PoolEntry]) -> Vec<FrontierVerdict> {
    fn rank(r: Regime) -> u8 {
        match r {
            Regime::Informative => 0,
            Regime::Empty => 1,
            Regime::Regenerate => 2,
            Regime::Saturated => 3,
        }
    }
    let mut v: Vec<FrontierVerdict> = pool
        .iter()
        .map(|e| FrontierVerdict {
            id: e.id.clone(),
            entropy: outcome_entropy(&e.outcomes),
            regime: regime_of(&e.outcomes),
        })
        .collect();
    v.sort_by(|a, b| {
        rank(a.regime)
            .cmp(&rank(b.regime))
            .then(b.entropy.total_cmp(&a.entropy))
            .then(a.id.cmp(&b.id))
    });
    v
}

/// The T7 telemetry line: one pool's dispersion column. `None` entropy
/// across a window = the vacuous flag the caller escalates.
#[must_use]
pub fn dispersion_column(id: &str, outcomes: &[OutcomeClass]) -> String {
    let h = outcome_entropy(outcomes);
    let r = regime_of(outcomes);
    let rlabel = match r {
        Regime::Informative => "informative",
        Regime::Saturated => "saturated",
        Regime::Regenerate => "regenerate",
        Regime::Empty => "empty",
    };
    format!("{id}: h={h:.3} regime={rlabel} n={}", outcomes.len())
}

/// Entropy over a whole window of per-pool entropies — the standing
/// non-vacuity instrument (T7): if every pool in the window reads 0.0, the
/// loop has emitted no dispersion and selection tuning is meaningless.
#[must_use]
pub fn window_is_vacuous(per_pool_entropies: &[f64]) -> bool {
    per_pool_entropies.iter().all(|h| *h <= f64::EPSILON)
}

/// Per-pool outcome counts → the histogram form the store rows carry
/// (kept/reverted/missed counts), expanded deterministically into the
/// class sequence [`outcome_entropy`] consumes. Order: all Kept, all
/// Reverted, all Missed (entropy is order-invariant; the expansion exists
/// so callers with count-shaped records need no alloc of their own).
#[must_use]
pub fn from_counts(kept: usize, reverted: usize, missed: usize) -> Vec<OutcomeClass> {
    let mut out = Vec::with_capacity(kept + reverted + missed);
    for _ in 0..kept {
        out.push(OutcomeClass::Kept);
    }
    for _ in 0..reverted {
        out.push(OutcomeClass::Reverted);
    }
    for _ in 0..missed {
        out.push(OutcomeClass::Missed);
    }
    out.shrink_to_fit();
    out
}

/// Convenience: a `BTreeMap` of pool id → entropy for telemetry sweeps.
#[must_use]
pub fn entropy_map(pool: &[PoolEntry]) -> BTreeMap<String, f64> {
    pool.iter()
        .map(|e| (e.id.clone(), outcome_entropy(&e.outcomes)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)] // 0.0 entropy is exact: equal class counts → exact log2 terms cancel
    fn empty_pool_has_zero_entropy_and_empty_regime() {
        assert_eq!(outcome_entropy(&[]), 0.0);
        assert_eq!(regime_of(&[]), Regime::Empty);
    }

    #[test]
    #[allow(clippy::float_cmp)] // uniform outcome → only one nonzero term, p·log2(p) never runs → exact 0.0
    fn uniform_kept_is_saturated_zero_entropy() {
        let o = vec![OutcomeClass::Kept; 8];
        assert_eq!(outcome_entropy(&o), 0.0);
        assert_eq!(regime_of(&o), Regime::Saturated);
    }

    #[test]
    fn uniform_failure_is_regenerate() {
        let o = vec![OutcomeClass::Reverted; 5];
        assert_eq!(regime_of(&o), Regime::Regenerate);
        let o = vec![OutcomeClass::Missed; 5];
        assert_eq!(regime_of(&o), Regime::Regenerate);
    }

    #[test]
    fn mixed_pool_is_informative_with_max_entropy_at_uniform() {
        let balanced = vec![
            OutcomeClass::Kept,
            OutcomeClass::Reverted,
            OutcomeClass::Missed,
            OutcomeClass::Kept,
            OutcomeClass::Reverted,
            OutcomeClass::Missed,
        ];
        let h = outcome_entropy(&balanced);
        // log2(3) is the max for 3 classes.
        assert!((h - 3f64.log2()).abs() < 1e-12, "h={h}");
        assert_eq!(regime_of(&balanced), Regime::Informative);
        // 50/50 split is informative but below max.
        let half = vec![OutcomeClass::Kept, OutcomeClass::Reverted];
        let h2 = outcome_entropy(&half);
        assert!((h2 - 1.0).abs() < 1e-12);
        assert!(h2 < h);
    }

    #[test]
    fn frontier_orders_informative_first_saturated_last() {
        let pool = vec![
            PoolEntry::new("sat", vec![OutcomeClass::Kept, OutcomeClass::Kept]),
            PoolEntry::new("inf", vec![OutcomeClass::Kept, OutcomeClass::Reverted]),
            PoolEntry::new("reg", vec![OutcomeClass::Missed]),
            PoolEntry::new("emp", vec![]),
        ];
        let ordered = frontier_order(&pool);
        let ids: Vec<&str> = ordered.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, vec!["inf", "emp", "reg", "sat"]);
    }

    #[test]
    fn frontier_ties_break_by_id_and_higher_entropy_wins_within_regime() {
        let pool = vec![
            PoolEntry::new("b_low", vec![OutcomeClass::Kept, OutcomeClass::Reverted, OutcomeClass::Reverted]),
            PoolEntry::new("a_high", vec![
                OutcomeClass::Kept,
                OutcomeClass::Reverted,
                OutcomeClass::Missed,
            ]),
        ];
        let ordered = frontier_order(&pool);
        assert_eq!(ordered[0].id, "a_high");
        assert!(ordered[0].entropy > ordered[1].entropy);
    }

    #[test]
    fn canary_planted_degenerate_pool_flags_vacuous_window() {
        // The T7 canary: a window where every pool reads zero entropy must
        // trip the vacuous flag (both directions pinned).
        let vacuous = vec![
            PoolEntry::new("a", vec![OutcomeClass::Kept; 3]),
            PoolEntry::new("b", vec![OutcomeClass::Missed; 3]),
        ];
        let ents: Vec<f64> = entropy_map(&vacuous).into_values().collect();
        assert!(window_is_vacuous(&ents));

        let live = vec![
            PoolEntry::new("a", vec![OutcomeClass::Kept; 3]),
            PoolEntry::new("b", vec![OutcomeClass::Kept, OutcomeClass::Missed]),
        ];
        let ents: Vec<f64> = entropy_map(&live).into_values().collect();
        assert!(!window_is_vacuous(&ents));
    }

    #[test]
    fn from_counts_matches_direct_classes() {
        let direct = vec![
            OutcomeClass::Kept,
            OutcomeClass::Kept,
            OutcomeClass::Reverted,
            OutcomeClass::Missed,
        ];
        assert_eq!(from_counts(2, 1, 1), direct);
    }

    #[test]
    fn dispersion_column_renders_regime() {
        let line = dispersion_column("p", &[OutcomeClass::Kept, OutcomeClass::Reverted]);
        assert!(line.contains("informative"));
        assert!(line.starts_with("p: h="));
        let line = dispersion_column("q", &[]);
        assert!(line.contains("empty"));
    }

    #[test]
    fn class_table_indices_are_stable() {
        assert_eq!(OutcomeClass::Kept.index(), 0);
        assert_eq!(OutcomeClass::Reverted.index(), 1);
        assert_eq!(OutcomeClass::Missed.index(), 2);
        assert_eq!(OutcomeClass::ALL.len(), 3);
    }
}
