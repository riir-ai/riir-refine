//! Issue 128 (T1-T4) — the fixseq cascade-law + derailment-index report.
//!
//! The Traverse/Scout statistics (arXiv:2609.17930, Research 200) adapted to
//! the repair-driver attempt ring: counting over labeled steps, deterministic,
//! no judge, no training. Three measurements land on the SAME ring format
//! [`crate::horizon_report`] already loads (`load_runs` is the ONE loader —
//! this module never re-reads the store):
//!
//! - **Cascade law (T3):** `P(attempt_{i+1} bad | attempt_i bad)` vs
//!   `P(attempt_{i+1} bad | attempt_i good)`, pooled with Wilson 95% CIs. The
//!   driver REVERTS bad attempts — a mechanical cascade-breaker — so the
//!   honest hypothesis is the law is WEAKER here than the paper's 13.7x/10.9x
//!   barrel-ahead jumps; a null is a real, publishable-shaped result about the
//!   keep-rule's design, never a failure of the instrument.
//! - **Derailment index (T4):** first-wrong-attempt position per indicator +
//!   cross-indicator +-k agreement curves + the positional-uniformity guard
//!   (G4: KS vs point-mass — blocks "just check attempt 3" shortcuts from
//!   claiming localization gains).
//! - **Silent failure (T4):** % of failed runs whose final attempt looks good
//!   (kept) — the driver-side twin of the katgpt-rs green-zero law.
//!
//! Report-first law: reads rings, gates NOTHING, wires NOTHING into the
//! driver/fixpoint paths (G3 — the analyzer is a pure consumer). Refuses
//! (rendered loud, CLI exit 1) on EMPTY (zero runs) and NO-TRANSITIONS (every
//! run single-attempt) — never a confident green zero over no data.
//!
//! G2: the statistics are single-pass O(attempts) counters — no per-attempt
//! allocations beyond the small position/code maps; nothing here runs in any
//! hot path. Ungated (the `horizon_report` law): the only rustc_errors-domain
//! constant it needs ([`MASKING_STAGE_CODES`]) is a local mirror pinned in
//! sync by a cfg'd test, exactly the `RingLine` precedent.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::horizon_report::RunRing;

/// Local mirror of `rustc_errors::diag::MASKING_STAGE_CODES` (currently
/// exactly `E0614`) — ungated module, gated source. `masking_codes_match_diag`
/// pins the sync on every build that compiles the domain.
pub const MASKING_STAGE_CODES: &[&str] = &["E0614"];

/// Membership test for [`MASKING_STAGE_CODES`].
#[must_use]
pub fn is_masking_code(code: &str) -> bool {
    MASKING_STAGE_CODES.contains(&code)
}

/// T2 — the three per-attempt badness indicators, ALL reported (no
/// single-indicator cherry-pick).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Badness {
    /// (a) `outcome` is a revert — the driver's own verdict.
    OutcomeReverted,
    /// (b) the attempt RAISED the coded-error count (kept or not).
    ErrorsGrew,
    /// (c) the attempt RAISED the masking-stage count (Issue 100's lex axis).
    MaskingGrew,
}

impl Badness {
    /// All three, definition order.
    pub const ALL: [Badness; 3] = [
        Badness::OutcomeReverted,
        Badness::ErrorsGrew,
        Badness::MaskingGrew,
    ];

    /// The stable render label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Badness::OutcomeReverted => "reverted",
            Badness::ErrorsGrew => "errors-grew",
            Badness::MaskingGrew => "masking-grew",
        }
    }

    /// `bad(line)` under this indicator. Line-level, order-free — the
    /// cascade pass pairs it across adjacent attempts.
    #[must_use]
    pub fn is_bad(&self, l: &crate::horizon_report::RingLine) -> bool {
        match self {
            Badness::OutcomeReverted => l.kept() == Some(false),
            Badness::ErrorsGrew => l.errors.1 > l.errors.0,
            Badness::MaskingGrew => l.masking.1 > l.masking.0,
        }
    }
}

/// Wilson score interval (95%, z = 1.959964) for k successes in n trials.
/// `(0.0, 0.0)` on n = 0 — the caller's `Option` accessors decide honesty.
#[allow(clippy::cast_precision_loss)] // counters -> f64 — the report's quantity
#[must_use]
pub fn wilson(k: usize, n: usize) -> (f64, f64) {
    const Z: f64 = 1.959_964;
    if n == 0 {
        return (0.0, 0.0);
    }
    let p = k as f64 / n as f64;
    let denom = 1.0 + Z * Z / n as f64;
    let center = (p + Z * Z / (2.0 * n as f64)) / denom;
    let half = Z * (p * (1.0 - p) / n as f64 + Z * Z / (4.0 * n as f64 * n as f64)).sqrt() / denom;
    (center - half, center + half)
}

/// T3 — transition counters for one indicator over one corpus. The four
/// cells of the 2x2 (prev-attempt badness x next-attempt badness).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CascadeStats {
    /// next bad after a bad prev.
    pub bad_after_bad: usize,
    /// next good after a bad prev.
    pub good_after_bad: usize,
    /// next bad after a good prev.
    pub bad_after_good: usize,
    /// next good after a good prev.
    pub good_after_good: usize,
}

impl CascadeStats {
    /// Total adjacent-pair transitions counted.
    #[must_use]
    pub const fn transitions(&self) -> usize {
        self.bad_after_bad + self.good_after_bad + self.bad_after_good + self.good_after_good
    }

    /// P(next bad | prev bad); `None` when no bad-prev transitions exist.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counters -> f64 — the report's quantity
    pub fn p_bad_given_bad(&self) -> Option<f64> {
        let n = self.bad_after_bad + self.good_after_bad;
        if n == 0 {
            None
        } else {
            Some(self.bad_after_bad as f64 / n as f64)
        }
    }

    /// P(next bad | prev good); `None` when no good-prev transitions exist.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counters -> f64 — the report's quantity
    pub fn p_bad_given_good(&self) -> Option<f64> {
        let n = self.bad_after_good + self.good_after_good;
        if n == 0 {
            None
        } else {
            Some(self.bad_after_good as f64 / n as f64)
        }
    }

    /// Wilson 95% CI for [`Self::p_bad_given_bad`] (`None` on no support).
    #[must_use]
    pub fn ci_bad_given_bad(&self) -> Option<(f64, f64)> {
        let n = self.bad_after_bad + self.good_after_bad;
        (n > 0).then(|| wilson(self.bad_after_bad, n))
    }

    /// Wilson 95% CI for [`Self::p_bad_given_good`] (`None` on no support).
    #[must_use]
    pub fn ci_bad_given_good(&self) -> Option<(f64, f64)> {
        let n = self.bad_after_good + self.good_after_good;
        (n > 0).then(|| wilson(self.bad_after_good, n))
    }

    /// The jump ratio P(bad|bad) / P(bad|good); `None` when either side has
    /// no support or the denominator is 0.
    #[must_use]
    pub fn ratio(&self) -> Option<f64> {
        match (self.p_bad_given_bad(), self.p_bad_given_good()) {
            (Some(a), Some(b)) if b > 0.0 => Some(a / b),
            _ => None,
        }
    }

    /// The cascade verdict for THIS corpus, three levels:
    /// `Separated` (Wilson LCB of P(bad|bad) >= UCB of P(bad|good) — the
    /// CIs do not even overlap, the strongest statement the counters make),
    /// `Weak` (point ratio > 1, intervals overlap), `Null` (no positive
    /// jump).
    #[must_use]
    pub fn verdict(&self) -> CascadeVerdict {
        match (self.p_bad_given_bad(), self.p_bad_given_good()) {
            (Some(a), Some(b)) if a > b => {
                match (self.ci_bad_given_bad(), self.ci_bad_given_good()) {
                    (Some((al, _)), Some((_, bu))) if al >= bu => {
                        CascadeVerdict::Separated
                    }
                    _ => CascadeVerdict::Weak,
                }
            }
            _ => CascadeVerdict::Null,
        }
    }
}

/// One corpus's cascade verdict (see [`CascadeStats::verdict`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CascadeVerdict {
    /// Wilson LCB(P(bad|bad)) >= UCB(P(bad|good)) — separated intervals.
    Separated,
    /// Point ratio > 1 but intervals overlap.
    Weak,
    /// No positive jump (ratio <= 1 or one-sided absence).
    Null,
}

impl CascadeVerdict {
    /// The stable render label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            CascadeVerdict::Separated => "SEPARATED",
            CascadeVerdict::Weak => "WEAK",
            CascadeVerdict::Null => "NULL",
        }
    }
}

/// T4 — the first-wrong-attempt distribution under one indicator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FirstWrong {
    /// Runs where the indicator fires at least once.
    pub runs_with: usize,
    /// Runs where it never fires (the run is right-censored — reported,
    /// never folded into positions).
    pub censored: usize,
    /// position (0-based seq) -> run count.
    pub hist: BTreeMap<usize, usize>,
    /// Mean first-wrong position over `runs_with` (`None` when none).
    pub mean_pos: Option<f64>,
    /// Share of first-wrongs at the modal position (0.0 when none).
    pub max_share: f64,
    /// G4 — KS distance of the empirical position CDF from the point mass
    /// at the mode: `max(F(mode-) , 1 - F(mode))`. 0.0 exactly when every
    /// first-wrong sits at one position (the shortcut's paradise); near the
    /// off-mode share when spread.
    pub ks_vs_point_mass: f64,
}

/// G4 floor — at least a quarter of first-wrongs off the modal position
/// before a localization claim may cite positional statistics.
pub const KS_FLOOR: f64 = 0.25;

/// G4 minimum runs with a first-wrong before the guard is decidable.
pub const MIN_SPREAD_RUNS: usize = 8;

impl FirstWrong {
    /// G4 — the positional-spread guard: the distribution is NOT a point
    /// mass (enough runs with a first-wrong AND `ks >= KS_FLOOR`).
    #[must_use]
    pub fn spread(&self) -> bool {
        self.runs_with >= MIN_SPREAD_RUNS && self.ks_vs_point_mass >= KS_FLOOR
    }
}

/// T4 — root-vs-cascade attribution over the PRIMARY indicator
/// (`OutcomeReverted`), using `masking` as the root signal: an E0614-family
/// fault is the root (typeck blocks borrowck); a non-masking bad attempt
/// that INHERITED a masked state (masking-before > 0) is a cascade victim.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attribution {
    /// Bad attempts whose own code is masking-stage (the E0614 root family).
    pub root: usize,
    /// Bad attempts with inherited masking > 0 but a non-masking code.
    pub cascade: usize,
    /// Bad attempts with no masking anywhere in sight (isolated reverts).
    pub unattributed: usize,
    /// Per-code counts of FIRST-wrong attempts (the `--misses` queue shape —
    /// consumed only if the Issue-093 reopen triggers ever fire).
    pub first_wrong_by_code: BTreeMap<String, usize>,
}

/// T4 — silent-failure census: failed runs (terminal residual > 0) whose
/// final attempt was KEPT (the journey ended mid-progress looking healthy).
///
/// Pre-Issue-105 rings stamp `terminal_residual` 0 (serde default) and read
/// as not-failed — the rate is honest only on 105+ rings; render says so.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SilentFailure {
    /// Runs with `terminal_residual` > 0.
    pub failed: usize,
    /// Of those, runs whose last attempt was kept.
    pub silent: usize,
}

impl SilentFailure {
    /// silent / failed; `None` when no failed runs.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counters -> f64 — the report's quantity
    pub fn rate(&self) -> Option<f64> {
        (self.failed > 0).then(|| self.silent as f64 / self.failed as f64)
    }
}

/// T1 — one corpus's census + statistics (a corpus = one labeled root).
#[derive(Clone, Debug, PartialEq)]
pub struct CorpusStats {
    /// The corpus label (the CLI uses the path; the mint manifest uses the
    /// generation config).
    pub label: String,
    /// Runs parsed.
    pub runs: usize,
    /// Runs with >= 2 attempts (the only runs that can produce transitions).
    pub runs_multi_attempt: usize,
    /// Runs with >= 2 reverted attempts (the Issue-093 organic class).
    pub runs_multi_revert: usize,
    /// Total attempts.
    pub attempts: usize,
    /// Kept attempts.
    pub kept: usize,
    /// Reverted attempts.
    pub reverted: usize,
    /// Adjacent-pair transitions (attempts - runs, per-run).
    pub transitions: usize,
    /// Cascade counters per indicator.
    pub cascade: BTreeMap<Badness, CascadeStats>,
    /// First-wrong distribution per indicator.
    pub first_wrong: BTreeMap<Badness, FirstWrong>,
    /// Root-vs-cascade attribution (primary indicator).
    pub attribution: Attribution,
    /// Silent-failure census.
    pub silent: SilentFailure,
}

impl CorpusStats {
    /// The zeroed census with the indicator maps pre-keyed (the `Default`
    /// shape `analyze_corpus` and test fixtures build on).
    #[must_use]
    pub fn zeroed(label: &str) -> Self {
        Self {
            label: label.to_string(),
            runs: 0,
            runs_multi_attempt: 0,
            runs_multi_revert: 0,
            attempts: 0,
            kept: 0,
            reverted: 0,
            transitions: 0,
            cascade: Badness::ALL.iter().map(|i| (*i, CascadeStats::default())).collect(),
            first_wrong: Badness::ALL.iter().map(|i| (*i, FirstWrong::default())).collect(),
            attribution: Attribution::default(),
            silent: SilentFailure::default(),
        }
    }
}

impl Default for CorpusStats {
    fn default() -> Self {
        Self::zeroed("")
    }
}

/// T4 — cross-indicator +-k agreement: over runs where BOTH indicators flag
/// a first-wrong, the count with |posA - posB| <= k for k = 0..=3 (the
/// paper's tolerance curve — here between OUR indicators, since no human
/// ground truth exists; agreement is the honest substitute).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgreementRow {
    /// Indicator A (definition order — always the smaller ordinal).
    pub a: Badness,
    /// Indicator B.
    pub b: Badness,
    /// Runs where both flagged a first-wrong.
    pub runs_both: usize,
    /// counts[k] = runs with |posA - posB| <= k, k = 0..=3 (monotone).
    pub within: [usize; 4],
}

/// T7 G1 — does the law reproduce across corpora? The disjointness rule
/// lives in the MINTING (two different generation configs, Issue 128 T5/T7);
/// this verdict only asks whether the per-corpus verdicts AGREE. A law
/// reproduced only within one generator is GENERATOR-CONDITIONAL — recorded
/// as such by the caller, never promoted to a driver law.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferVerdict {
    /// Every corpus Separated.
    AllSeparated,
    /// Every corpus positive (all Separated or Weak, at least one Weak).
    AllPositive,
    /// Every corpus Null.
    AllNull,
    /// Corpora disagree — the law does NOT transfer as measured.
    Disagrees,
    /// Fewer than two corpora carry transitions for this indicator.
    Unmeasured,
}

impl TransferVerdict {
    /// The stable render label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            TransferVerdict::AllSeparated => "REPRODUCED-SEPARATED",
            TransferVerdict::AllPositive => "REPRODUCED-WEAK",
            TransferVerdict::AllNull => "NULL-ALL",
            TransferVerdict::Disagrees => "DISAGREES",
            TransferVerdict::Unmeasured => "UNMEASURED",
        }
    }
}

/// Why the report refused (the green-zero law) — rendered loud, exit 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixseqRefusal {
    /// No runs parsed anywhere.
    Empty,
    /// Runs exist but no run reaches 2 attempts — zero transitions.
    NoTransitions,
}

impl FixseqRefusal {
    /// The loud refusal line.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            FixseqRefusal::Empty => "EMPTY — zero runs parsed (no .heal/fixseq store here)",
            FixseqRefusal::NoTransitions => {
                "NO TRANSITIONS — runs exist but no run reaches 2 attempts; the cascade law needs depth"
            }
        }
    }
}

/// The computed report over one or more labeled corpora.
#[derive(Clone, Debug, PartialEq)]
pub struct FixseqReport {
    /// Per-corpus stats, input order.
    pub corpora: Vec<CorpusStats>,
    /// The pooled corpus (label "pooled") — all runs together.
    pub pooled: CorpusStats,
    /// Cross-indicator agreement rows (pooled).
    pub agreement: Vec<AgreementRow>,
    /// T7 G1 transfer verdict per indicator.
    pub transfer: BTreeMap<Badness, TransferVerdict>,
    /// The refusal, if any.
    pub refusal: Option<FixseqRefusal>,
}

/// T1-T4 — analyze one corpus (single pass over runs; counters only).
#[allow(clippy::cast_precision_loss)] // counters -> f64 rates — the report's quantity
#[allow(clippy::too_many_lines)] // one census pass feeding five tables — the fold shares the per-run walk state
#[must_use]
pub fn analyze_corpus(label: &str, runs: &[RunRing]) -> CorpusStats {
    let mut s = CorpusStats::zeroed(label);
    // Per-indicator first-wrong fold state: position histogram + sum.
    let mut hist: BTreeMap<Badness, BTreeMap<usize, usize>> = BTreeMap::new();
    let mut pos_sum: BTreeMap<Badness, usize> = BTreeMap::new();

    for ring in runs {
        let lines = &ring.lines;
        s.runs += 1;
        s.attempts += lines.len();
        let mut reverts_in_run = 0usize;
        for l in lines {
            match l.kept() {
                Some(true) => s.kept += 1,
                Some(false) => {
                    s.reverted += 1;
                    reverts_in_run += 1;
                }
                None => {}
            }
        }
        if lines.len() >= 2 {
            s.runs_multi_attempt += 1;
        }
        if reverts_in_run >= 2 {
            s.runs_multi_revert += 1;
        }

        // T3 — the 2x2 transition cells, all three indicators in one walk.
        for w in lines.windows(2) {
            let (prev, next) = (&w[0], &w[1]);
            s.transitions += 1;
            for ind in Badness::ALL {
                let cell = s.cascade.entry(ind).or_default();
                match (ind.is_bad(prev), ind.is_bad(next)) {
                    (true, true) => cell.bad_after_bad += 1,
                    (true, false) => cell.good_after_bad += 1,
                    (false, true) => cell.bad_after_good += 1,
                    (false, false) => cell.good_after_good += 1,
                }
            }
        }

        // T4 — first-wrong positions + attribution, one walk.
        let mut first_seen: BTreeMap<Badness, Option<usize>> = BTreeMap::new();
        for (i, l) in lines.iter().enumerate() {
            for ind in Badness::ALL {
                let seen = first_seen.entry(ind).or_insert(None);
                if seen.is_none() && ind.is_bad(l) {
                    *seen = Some(i);
                }
            }
            if Badness::OutcomeReverted.is_bad(l) {
                if is_masking_code(&l.code) {
                    s.attribution.root += 1;
                } else if l.masking.0 > 0 {
                    s.attribution.cascade += 1;
                } else {
                    s.attribution.unattributed += 1;
                }
            }
        }
        for (ind, first) in first_seen {
            match first {
                Some(p) => {
                    *hist.entry(ind).or_default().entry(p).or_default() += 1;
                    *pos_sum.entry(ind).or_default() += p;
                    if ind == Badness::OutcomeReverted {
                        let code = &lines[p].code;
                        *s
                            .attribution
                            .first_wrong_by_code
                            .entry(code.clone())
                            .or_default() += 1;
                    }
                }
                None => {
                    s.first_wrong.entry(ind).or_default().censored += 1;
                }
            }
        }

        // T4 — silent failure (run-level; pre-105 rings stamp 0 = not failed).
        if let Some(last) = lines.last() {
            if last.terminal_residual > 0 {
                s.silent.failed += 1;
                if last.kept() == Some(true) {
                    s.silent.silent += 1;
                }
            }
        }
    }

    // Fold the first-wrong histograms into the per-indicator tables.
    for ind in Badness::ALL {
        let h = hist.get(&ind);
        let runs_with = h.map_or(0, |m| m.values().sum());
        let mean_pos = (runs_with > 0).then(|| {
            pos_sum.get(&ind).copied().unwrap_or(0) as f64 / runs_with as f64
        });
        let (max_share, ks) = h.map_or((0.0, 0.0), |m| {
            let total: usize = m.values().sum();
            // Mode = highest count, ties to the SMALLEST position (the
            // shortcut's best case — the guard must not be flattered by a
            // tie-break).
            let (&mode, &mode_n) = m
                .iter()
                .max_by_key(|(p, c)| (*c, std::cmp::Reverse(**p)))
                .map_or((&0, &0), |(p, c)| (p, c));
            let share = mode_n as f64 / total as f64;
            let f_mode = m
                .iter()
                .filter(|(p, _)| **p <= mode)
                .map(|(_, c)| c)
                .sum::<usize>() as f64
                / total as f64;
            // KS distance to the point mass at the mode:
            // max(F(mode) - share, 1 - F(mode)).
            (share, (f_mode - share).max(1.0 - f_mode))
        });
        let censored = s.first_wrong.get(&ind).map_or(0, |f| f.censored);
        s.first_wrong.insert(
            ind,
            FirstWrong {
                runs_with,
                censored,
                hist: h.cloned().unwrap_or_default(),
                mean_pos,
                max_share,
                ks_vs_point_mass: ks,
            },
        );
    }
    s
}

/// The +-k cross-indicator agreement rows (pooled runs).
#[must_use]
pub fn agreement(runs: &[RunRing]) -> Vec<AgreementRow> {
    let mut rows = Vec::new();
    for (ai, a) in Badness::ALL.iter().enumerate() {
        for b in Badness::ALL.iter().skip(ai + 1) {
            let mut row = AgreementRow {
                a: *a,
                b: *b,
                runs_both: 0,
                within: [0; 4],
            };
            for ring in runs {
                let pa = ring.lines.iter().position(|l| a.is_bad(l));
                let pb = ring.lines.iter().position(|l| b.is_bad(l));
                if let (Some(pa), Some(pb)) = (pa, pb) {
                    row.runs_both += 1;
                    let d = pa.abs_diff(pb);
                    for (k, slot) in row.within.iter_mut().enumerate() {
                        if d <= k {
                            *slot += 1;
                        }
                    }
                }
            }
            rows.push(row);
        }
    }
    rows
}

/// T7 G1 — the transfer verdict for one indicator across corpora.
#[must_use]
pub fn transfer_verdict(corpora: &[CorpusStats], ind: Badness) -> TransferVerdict {
    let with_support: Vec<CascadeVerdict> = corpora
        .iter()
        .filter(|c| c.cascade.get(&ind).is_some_and(|cs| cs.transitions() > 0))
        .map(|c| c.cascade[&ind].verdict())
        .collect();
    match with_support.len() {
        0 | 1 => TransferVerdict::Unmeasured,
        _ => {
            let all_sep = with_support.iter().all(|v| *v == CascadeVerdict::Separated);
            let all_pos = with_support.iter().all(|v| {
                matches!(v, CascadeVerdict::Separated | CascadeVerdict::Weak)
            });
            let all_null = with_support.iter().all(|v| *v == CascadeVerdict::Null);
            if all_sep {
                TransferVerdict::AllSeparated
            } else if all_pos {
                TransferVerdict::AllPositive
            } else if all_null {
                TransferVerdict::AllNull
            } else {
                TransferVerdict::Disagrees
            }
        }
    }
}

/// Summarize one or more labeled corpora into the report (pools runs for
/// the pooled corpus + agreement rows; computes the per-indicator transfer
/// verdicts).
#[must_use]
pub fn summarize(labeled: &[(String, Vec<RunRing>)]) -> FixseqReport {
    let corpora: Vec<CorpusStats> = labeled
        .iter()
        .map(|(label, runs)| analyze_corpus(label, runs))
        .collect();
    let total: usize = labeled.iter().map(|(_, r)| r.len()).sum();
    let mut all: Vec<RunRing> = Vec::with_capacity(total);
    for (_, r) in labeled {
        all.extend(r.iter().cloned());
    }
    let pooled = analyze_corpus("pooled", &all);
    let agreement = agreement(&all);
    let transfer = Badness::ALL
        .iter()
        .map(|i| (*i, transfer_verdict(&corpora, *i)))
        .collect();
    let refusal = match pooled.runs {
        0 => Some(FixseqRefusal::Empty),
        _ if pooled.transitions == 0 => Some(FixseqRefusal::NoTransitions),
        _ => None,
    };
    FixseqReport {
        corpora,
        pooled,
        agreement,
        transfer,
        refusal,
    }
}

// ── Render ──────────────────────────────────────────────────────────────────

/// Render the report (ASCII-only — the cp874 console law).
#[allow(clippy::too_many_lines)] // one report, one renderer — sections share the header state
#[must_use]
pub fn render(r: &FixseqReport) -> String {
    let mut out = String::with_capacity(4096);
    writeln!(
        out,
        "fixseq cascade + derailment report (Issue 128; Research 200 / arXiv:2609.17930)"
    )
    .unwrap();
    if let Some(f) = r.refusal {
        writeln!(out, "REFUSAL: {}", f.label()).unwrap();
        return out;
    }
    let p = &r.pooled;
    writeln!(
        out,
        "corpora {} | runs {} (multi-attempt {}, multi-revert {}) | attempts {} | kept {} | reverted {} | transitions {}",
        r.corpora.len(),
        p.runs,
        p.runs_multi_attempt,
        p.runs_multi_revert,
        p.attempts,
        p.kept,
        p.reverted,
        p.transitions
    )
    .unwrap();
    writeln!(
        out,
        "silent failure: {}/{} failed runs ended on a kept attempt ({}) | pre-105 rings stamp terminal_residual 0 and read as not-failed",
        p.silent.silent,
        p.silent.failed,
        p.silent
            .rate()
            .map_or("n/a".to_string(), |v| format!("{:.1}%", 100.0 * v))
    )
    .unwrap();
    writeln!(out).unwrap();

    // T3 — the cascade law, per indicator, per corpus + pooled + transfer.
    writeln!(
        out,
        "cascade law (Wilson 95% CIs; the driver reverts bad attempts - a NULL is a real result)"
    )
    .unwrap();
    for ind in Badness::ALL {
        writeln!(out, "  indicator: {}", ind.label()).unwrap();
        writeln!(
            out,
            "    {:<24} {:>5} {:>7} {:>12} {:>7} {:>12} {:>6} {:<12}",
            "corpus", "trans", "P(b|b)", "CI", "P(b|g)", "CI", "ratio", "verdict"
        )
        .unwrap();
        for c in r.corpora.iter().chain(std::iter::once(&r.pooled)) {
            let Some(cs) = c.cascade.get(&ind) else { continue };
            if cs.transitions() == 0 {
                writeln!(
                    out,
                    "    {:<24} {:>5} {:>43} {:>6} {:<12}",
                    c.label, 0, "-", "-", "UNMEASURED"
                )
                .unwrap();
                continue;
            }
            let ci = |x: Option<(f64, f64)>| {
                x.map_or("-".to_string(), |(l, u)| format!("[{l:.3},{u:.3}]"))
            };
            let pbb = cs
                .p_bad_given_bad()
                .map_or("-".to_string(), |v| format!("{v:.3}"));
            let pbg = cs
                .p_bad_given_good()
                .map_or("-".to_string(), |v| format!("{v:.3}"));
            let ratio = cs
                .ratio()
                .map_or("-".to_string(), |v| format!("{v:.2}"));
            writeln!(
                out,
                "    {:<24} {:>5} {:>7} {:>12} {:>7} {:>12} {:>6} {:<12}",
                c.label,
                cs.transitions(),
                pbb,
                ci(cs.ci_bad_given_bad()),
                pbg,
                ci(cs.ci_bad_given_good()),
                ratio,
                cs.verdict().label()
            )
            .unwrap();
        }
        if let Some(tv) = r.transfer.get(&ind) {
            writeln!(out, "    T7 transfer: {}", tv.label()).unwrap();
        }
        writeln!(out).unwrap();
    }

    // T4 — derailment: first-wrong positions (pooled).
    writeln!(
        out,
        "derailment index (pooled) - first-wrong position per indicator"
    )
    .unwrap();
    writeln!(
        out,
        "  {:<14} {:>6} {:>9} {:>6} {:>9} {:>10} {:>7}",
        "indicator", "runs", "censored", "mean", "maxshare", "KS(ptmass)", "G4"
    )
    .unwrap();
    for ind in Badness::ALL {
        if let Some(fw) = r.pooled.first_wrong.get(&ind) {
            writeln!(
                out,
                "  {:<14} {:>6} {:>9} {:>6} {:>9} {:>10.3} {:>7}",
                ind.label(),
                fw.runs_with,
                fw.censored,
                fw.mean_pos.map_or("-".to_string(), |m| format!("{m:.1}")),
                format!("{:.2}", fw.max_share),
                fw.ks_vs_point_mass,
                if fw.spread() { "SPREAD" } else { "narrow" }
            )
            .unwrap();
        }
    }
    if let Some(fw) = r.pooled.first_wrong.get(&Badness::OutcomeReverted) {
        if !fw.hist.is_empty() {
            let mut cells = String::new();
            for (pos, cnt) in &fw.hist {
                use std::fmt::Write as _;
                let _ = write!(cells, "{pos}x{cnt} ");
            }
            writeln!(out, "  reverted first-wrong hist: {cells}").unwrap();
        }
    }
    writeln!(out).unwrap();

    // T4 — cross-indicator agreement (pooled).
    if r.agreement.iter().any(|a| a.runs_both > 0) {
        writeln!(
            out,
            "cross-indicator agreement (pooled) - |posA - posB| <= k rates"
        )
        .unwrap();
        writeln!(
            out,
            "  {:<34} {:>5} {:>6} {:>6} {:>6} {:>6}",
            "pair", "runs", "k=0", "k=1", "k=2", "k=3"
        )
        .unwrap();
        for a in &r.agreement {
            #[allow(clippy::cast_precision_loss)] // counters -> f64 — the report's quantity
            let pct = |k: usize| {
                if a.runs_both == 0 {
                    "-".to_string()
                } else {
                    format!("{:.0}%", 100.0 * a.within[k] as f64 / a.runs_both as f64)
                }
            };
            writeln!(
                out,
                "  {:<34} {:>5} {:>6} {:>6} {:>6} {:>6}",
                format!("{} vs {}", a.a.label(), a.b.label()),
                a.runs_both,
                pct(0),
                pct(1),
                pct(2),
                pct(3)
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    // T4 — attribution.
    let at = &r.pooled.attribution;
    writeln!(
        out,
        "root-vs-cascade attribution (reverted attempts, pooled): root(E0614-family) {} | cascade(inherited mask) {} | unattributed {}",
        at.root, at.cascade, at.unattributed
    )
    .unwrap();
    if !at.first_wrong_by_code.is_empty() {
        let mut top: Vec<(&String, &usize)> = at.first_wrong_by_code.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let cells: Vec<String> = top
            .iter()
            .take(8)
            .map(|(c, n)| format!("{c}x{n}"))
            .collect();
        writeln!(
            out,
            "  first-wrong by code (misses-queue shape): {}",
            cells.join(" ")
        )
        .unwrap();
    }
    writeln!(
        out,
        "\nreport-only (Issue 128 scope guard): nothing wired into driver/fixpoint paths; G2 single-pass counters."
    )
    .unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon_report::RingLine;

    fn rl(
        seq: usize,
        code: &str,
        outcome: &str,
        errs: (usize, usize),
        mask: (usize, usize),
        term: u32,
    ) -> RingLine {
        RingLine {
            seq,
            code: code.to_string(),
            span_hash: format!("{seq:016x}"),
            strategy_id: "s".to_string(),
            outcome: outcome.to_string(),
            errors: errs,
            masking: mask,
            journey_residual_sum: 0,
            terminal_residual: term,
            span_embed: None,
        }
    }

    fn ring(lines: Vec<RingLine>) -> RunRing {
        RunRing {
            source: "run-test.jsonl".to_string(),
            lines,
        }
    }

    #[test]
    fn wilson_bounds_point_and_shrink() {
        let (l, u) = wilson(5, 10);
        assert!((l - 0.2367).abs() < 1e-3 && (u - 0.7633).abs() < 1e-3, "({l},{u})");
        assert_eq!(wilson(3, 0), (0.0, 0.0));
        let l0 = wilson(0, 40).0;
        assert_eq!(l0, 0.0);
        // Width shrinks with n at the same rate.
        let w = |n: usize| {
            let (a, b) = wilson(n / 2, n);
            b - a
        };
        assert!(w(400) < w(40), "{} {}", w(400), w(40));
        // lcb <= k/n <= ucb on a grid.
        for n in [1usize, 3, 7, 12, 50] {
            for k in 0..=n {
                let (a, b) = wilson(k, n);
                let p = k as f64 / n as f64;
                assert!(a <= p + 1e-9 && p <= b + 1e-9, "k={k} n={n} ({a},{b})");
            }
        }
    }

    #[test]
    fn cascade_cells_and_verdicts() {
        // The K R R K walk: (K->R) good->bad, (R->R) bad->bad, (R->K)
        // bad->good under indicator (a).
        let cs = CascadeStats {
            bad_after_bad: 1,
            good_after_bad: 1,
            bad_after_good: 1,
            good_after_good: 0,
        };
        assert_eq!(cs.transitions(), 3);
        assert!((cs.p_bad_given_bad().unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(cs.p_bad_given_good(), Some(1.0));
        assert_eq!(cs.verdict(), CascadeVerdict::Null); // 0.5 < 1.0

        // Separated: 18/20 bad after bad vs 2/20 bad after good.
        let sep = CascadeStats {
            bad_after_bad: 18,
            good_after_bad: 2,
            bad_after_good: 2,
            good_after_good: 18,
        };
        assert_eq!(sep.verdict(), CascadeVerdict::Separated);
        // Weak: ratio > 1, intervals overlap (small n).
        let weak = CascadeStats {
            bad_after_bad: 3,
            good_after_bad: 1,
            bad_after_good: 1,
            good_after_good: 7,
        };
        assert_eq!(weak.verdict(), CascadeVerdict::Weak);
        // Equal rates -> Null.
        let eq = CascadeStats {
            bad_after_bad: 10,
            good_after_bad: 10,
            bad_after_good: 10,
            good_after_good: 10,
        };
        assert_eq!(eq.p_bad_given_bad(), eq.p_bad_given_good());
        assert_eq!(eq.verdict(), CascadeVerdict::Null);
    }

    #[test]
    fn first_wrong_spread_guard() {
        // All first-wrongs at one position: KS = 0, not spread (and below
        // MIN_SPREAD_RUNS here anyway).
        let mut hist = BTreeMap::new();
        hist.insert(3usize, 10usize);
        let fw = FirstWrong {
            runs_with: 10,
            censored: 0,
            hist,
            mean_pos: Some(3.0),
            max_share: 1.0,
            ks_vs_point_mass: 0.0,
        };
        assert!(!fw.spread());
        // Half at 1, half at 3 (10 runs): KS = 0.5 >= floor -> spread.
        let mut h2 = BTreeMap::new();
        h2.insert(1usize, 5usize);
        h2.insert(3usize, 5usize);
        let f2 = FirstWrong {
            runs_with: 10,
            censored: 2,
            hist: h2,
            mean_pos: Some(2.0),
            max_share: 0.5,
            ks_vs_point_mass: 0.5,
        };
        assert!(f2.spread());
        // 8 runs (the floor), ks at the floor exactly -> spread.
        let mut h3 = BTreeMap::new();
        h3.insert(0usize, 6usize);
        h3.insert(2usize, 2usize);
        // F(0)=0.75, share 0.75: KS = max(0, 0.25) = 0.25.
        let f3 = FirstWrong {
            runs_with: 8,
            censored: 0,
            hist: h3,
            mean_pos: Some(0.5),
            max_share: 0.75,
            ks_vs_point_mass: 0.25,
        };
        assert!(f3.spread());
    }

    #[test]
    fn analyze_corpus_census_attribution_silent() {
        // Run 1: K, R E0614 (root), K, R E0499 with inherited mask
        // (cascade), K — terminal 2 (failed, last kept -> silent).
        let run1 = ring(vec![
            rl(0, "E0597", "Kept", (5, 4), (0, 0), 2),
            rl(1, "E0614", "RevertedNoProgress", (4, 4), (1, 0), 2),
            rl(2, "E0499", "Kept", (4, 3), (0, 0), 2),
            rl(3, "E0499", "RevertedNoProgress", (3, 3), (1, 1), 2),
            rl(4, "E0502", "Kept", (3, 2), (0, 0), 2),
        ]);
        // Run 2: single attempt, kept, clean terminal -> not failed.
        let run2 = ring(vec![rl(0, "E0597", "Kept", (1, 0), (0, 0), 0)]);
        let s = analyze_corpus("t", &[run1, run2]);
        assert_eq!(s.runs, 2);
        assert_eq!(s.attempts, 6);
        assert_eq!(s.kept, 4);
        assert_eq!(s.reverted, 2);
        assert_eq!(s.runs_multi_attempt, 1);
        assert_eq!(s.runs_multi_revert, 1); // run1 carries 2 reverts
        assert_eq!(s.transitions, 4);
        // Attribution: E0614 revert -> root; E0499 revert with masking.0>0
        // -> cascade.
        assert_eq!(s.attribution.root, 1);
        assert_eq!(s.attribution.cascade, 1);
        assert_eq!(s.attribution.unattributed, 0);
        assert_eq!(s.attribution.first_wrong_by_code.get("E0614"), Some(&1));
        // Silent: run1 failed (terminal 2) and last kept -> 1/1.
        assert_eq!(s.silent.failed, 1);
        assert_eq!(s.silent.silent, 1);
        // Indicator (b) errors-grew: errors never rise — all four
        // transitions are good->good.
        assert_eq!(s.cascade[&Badness::ErrorsGrew].transitions(), 4);
        assert_eq!(s.cascade[&Badness::ErrorsGrew].good_after_good, 4);
        // Indicator (c) masking-grew: line 3 has mask 1->1 (not >) — never
        // bad.
        assert_eq!(s.cascade[&Badness::MaskingGrew].bad_after_bad, 0);
    }

    #[test]
    fn analyze_computes_ks_from_hist() {
        // Two runs, first-wrong at 0 and 0 -> point mass -> KS 0.
        let mk = |p: usize| {
            let mut v = Vec::new();
            for i in 0..4usize {
                let outcome = if i < p { "Kept" } else { "RevertedNoProgress" };
                v.push(rl(i, "E0499", outcome, (4, 4), (0, 0), 0));
            }
            ring(v)
        };
        let s = analyze_corpus(
            "t",
            &[mk(0), mk(0)],
        );
        let fw = &s.first_wrong[&Badness::OutcomeReverted];
        assert_eq!(fw.runs_with, 2);
        assert!((fw.ks_vs_point_mass - 0.0).abs() < 1e-9, "{}", fw.ks_vs_point_mass);
        assert!((fw.max_share - 1.0).abs() < 1e-9);
        assert!((fw.mean_pos.unwrap() - 0.0).abs() < 1e-9);
        // First-wrong at 0 and 2: F(0)=0.5, share 0.5 -> KS = 0.5.
        let s2 = analyze_corpus("t", &[mk(0), mk(2)]);
        let fw2 = &s2.first_wrong[&Badness::OutcomeReverted];
        assert!((fw2.ks_vs_point_mass - 0.5).abs() < 1e-9, "{}", fw2.ks_vs_point_mass);
        // 0 and 1: F(0)=0.5 share 0.5, mode=0 -> KS = max(0.5-0.5, 0.5)=0.5.
        let s3 = analyze_corpus("t", &[mk(0), mk(1)]);
        let fw3 = &s3.first_wrong[&Badness::OutcomeReverted];
        assert!((fw3.ks_vs_point_mass - 0.5).abs() < 1e-9, "{}", fw3.ks_vs_point_mass);
    }

    #[test]
    fn agreement_counts_within_k() {
        // Run A: reverted at 1, errors-grew never -> excluded (censored b).
        let a = ring(vec![
            rl(0, "E0597", "Kept", (5, 5), (0, 0), 0),
            rl(1, "E0597", "RevertedNoProgress", (5, 5), (0, 0), 0),
        ]);
        // Run B: reverted at 1, errors-grew at 3 (d = 2).
        let b = ring(vec![
            rl(0, "E0597", "Kept", (5, 5), (0, 0), 0),
            rl(1, "E0597", "RevertedNoProgress", (5, 5), (0, 0), 0),
            rl(2, "E0597", "Kept", (5, 5), (0, 0), 0),
            rl(3, "E0597", "Kept", (5, 6), (0, 0), 0),
        ]);
        let rows = agreement(&[a, b]);
        let row = rows
            .iter()
            .find(|r| r.a == Badness::OutcomeReverted && r.b == Badness::ErrorsGrew)
            .unwrap();
        assert_eq!(row.runs_both, 1);
        assert_eq!(row.within, [0, 0, 1, 1]);
    }

    #[test]
    fn transfer_verdicts_require_agreement() {
        let stats_with = |bab: usize, gab: usize, bag: usize, gag: usize| CorpusStats {
            cascade: Badness::ALL
                .iter()
                .map(|i| {
                    (*i, if *i == Badness::OutcomeReverted {
                        CascadeStats {
                            bad_after_bad: bab,
                            good_after_bad: gab,
                            bad_after_good: bag,
                            good_after_good: gag,
                        }
                    } else {
                        CascadeStats::default()
                    })
                })
                .collect(),
            ..CorpusStats::zeroed("t")
        };
        let sep = stats_with(18, 2, 2, 18);
        let nul = stats_with(2, 18, 2, 18);
        let weak = stats_with(3, 1, 1, 7);
        assert_eq!(
            transfer_verdict(&[sep.clone(), sep.clone()], Badness::OutcomeReverted),
            TransferVerdict::AllSeparated
        );
        assert_eq!(
            transfer_verdict(&[sep.clone(), nul.clone()], Badness::OutcomeReverted),
            TransferVerdict::Disagrees
        );
        assert_eq!(
            transfer_verdict(&[sep.clone()], Badness::OutcomeReverted),
            TransferVerdict::Unmeasured
        );
        assert_eq!(
            transfer_verdict(&[nul.clone(), nul], Badness::OutcomeReverted),
            TransferVerdict::AllNull
        );
        assert_eq!(
            transfer_verdict(&[sep.clone(), weak], Badness::OutcomeReverted),
            TransferVerdict::AllPositive
        );
        // ErrorsGrew has zero transitions everywhere -> Unmeasured.
        assert_eq!(
            transfer_verdict(&[sep.clone(), sep], Badness::ErrorsGrew),
            TransferVerdict::Unmeasured
        );
    }

    #[test]
    fn summarize_refuses_on_empty_and_no_transitions() {
        let r = summarize(&[]);
        assert_eq!(r.refusal, Some(FixseqRefusal::Empty));
        let solo = vec![(
            "solo".to_string(),
            vec![ring(vec![rl(0, "E0597", "Kept", (1, 0), (0, 0), 0)])],
        )];
        let r2 = summarize(&solo);
        assert_eq!(r2.refusal, Some(FixseqRefusal::NoTransitions));
    }

    #[test]
    fn summarize_pools_transitions_and_labels() {
        let mk = |phase: usize| {
            let mut v = Vec::new();
            for i in 0..6usize {
                let outcome = if (i + phase) % 4 < 2 {
                    "Kept"
                } else {
                    "RevertedNoProgress"
                };
                v.push(rl(i, "E0499", outcome, (6, 6), (0, 0), 1));
            }
            ring(v)
        };
        let corpora = vec![
            ("cfgA".to_string(), vec![mk(0), mk(1)]),
            ("cfgB".to_string(), vec![mk(2), mk(3)]),
        ];
        let r = summarize(&corpora);
        assert!(r.refusal.is_none());
        assert_eq!(r.corpora.len(), 2);
        assert_eq!(r.pooled.runs, 4);
        assert_eq!(r.pooled.transitions, 4 * 5);
        // Pooled transitions = sum of per-corpus transitions.
        let per_sum: usize = r.corpora.iter().map(|c| c.transitions).sum();
        assert_eq!(per_sum, r.pooled.transitions);
    }

    #[test]
    fn render_is_ascii_and_covers_sections() {
        let mk = |phase: usize| {
            let mut v = Vec::new();
            for i in 0..6usize {
                let outcome = if (i + phase) % 4 < 2 {
                    "Kept"
                } else {
                    "RevertedNoProgress"
                };
                v.push(rl(i, "E0499", outcome, (6, 6), (0, 0), 1));
            }
            ring(v)
        };
        let corpora = vec![
            ("cfgA".to_string(), vec![mk(0), mk(1)]),
            ("cfgB".to_string(), vec![mk(2), mk(3)]),
        ];
        let r = summarize(&corpora);
        let text = render(&r);
        assert!(text.chars().all(|c| c.is_ascii()), "{text}");
        assert!(text.contains("cascade law"));
        assert!(text.contains("derailment index"));
        assert!(text.contains("root-vs-cascade attribution"));
        assert!(text.contains("T7 transfer"));
        // The refusal renders loud + short.
        let empty = summarize(&[]);
        assert!(render(&empty).starts_with("fixseq cascade"));
        assert!(render(&empty).contains("REFUSAL"));
    }

    /// The ungated local mirror must equal the gated source of truth on
    /// every build that compiles the domain (the `RingLine` precedent).
    #[cfg(feature = "rustc_errors")]
    #[test]
    fn masking_codes_match_diag() {
        assert_eq!(
            MASKING_STAGE_CODES,
            crate::domains::rustc_errors::diag::MASKING_STAGE_CODES
        );
    }
}
