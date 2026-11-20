//! Plan 392 Phase 1 (Research 426) — the REPORT-ONLY epiplexity mining-gain
//! census. Loads the committed `.benchmarks/mining_gain_panel.jsonl` fixture,
//! fits the sibling `GainEstimator` (katgpt-core `epiplexity_gain`, Plan 641 —
//! consumed, never forked), and produces the full census report AS DATA so the
//! `mining_gain_census` example stays a thin printer and the gate battery
//! (`tests/epiplexity_census_gates.rs`) tests the REAL pipeline, not a copy.
//!
//! # Plan 392 cross-reference
//!
//! Phase 0 (Research 426) measured the logged exposure matrix at effective
//! rank 1 (eff10; CORRECTED 2026-11-20 — the first pass's divergent Jacobi
//! pairing read 2; the converged spectrum reads 1, trace invariant rel err
//! 2.3e-16) against the cross-domain bar 7 → verdict SPARSE. This census makes
//! that prediction observable on the same fixture: the saturation arm + rank
//! census, the per-domain gain table, the gain-ordered queue, and the two
//! queue gates — the Pile-saturation demotion ([`SATURATION_DEMOTE_HEADROOM`]:
//! predicted remaining gain under the bar → `DEMOTED`) and the dominance
//! refusal ([`DOMINANCE_SKIP_WEIGHT`]: one domain owning the allocation
//! integral → `SKIP-SELECTION`).
//!
//! # Report-only law
//!
//! Nothing here writes anything: the mining queue file is never touched, and
//! the example exits 0 on every successful read. Refusals are typed
//! ([`PanelError`]) and refuse LOUD at the caller (the example exits 2).
//!
//! # Determinism (Issue-940 law)
//!
//! Sorted domain names everywhere, ascending folds, no `HashMap` iteration
//! order in any output (`Vec` + explicit sorts only). The estimator's own
//! reductions are pinned ascending in the sibling module; a surface committing
//! gains across the sync boundary would re-pin order + tie rule at its own
//! site — this lane is report-only and commits nothing.

use std::fmt;
use std::path::Path;

use katgpt_core::epiplexity_gain::{
    GainCell, GainEstimator, MAX_DOMAINS, MixPanel, RankCensus, Saturation,
};
use serde::Deserialize;

/// The fixture's `format` tag — anything else refuses
/// [`PanelError::UnknownFormat`].
pub const PANEL_FORMAT: &str = "mining_gain_panel_v1";

/// The sigmoid-weight temperature the report's weights are computed at
/// (Plan 392 Phase 1's documented call).
pub const SIGMOID_TAU: f32 = 0.5;

/// The Pile-saturation queue gate (Plan 392): a domain whose predicted
/// remaining gain (`gap − ε` headroom) sits strictly under this bar prints
/// `DEMOTED (<name>)` — its remaining learnable gap cannot pay for an
/// allocation. A `None` headroom (a `NoCurve` domain) is already a typed
/// abstain, not a demotion.
pub const SATURATION_DEMOTE_HEADROOM: f64 = 0.005;

/// The dominance refusal bar (Plan 392): when the top normalized weight
/// reaches this share of the allocation integral, one domain owns it and the
/// machinery refuses to run (`SKIP-SELECTION`, still exit 0 — report-only).
pub const DOMINANCE_SKIP_WEIGHT: f64 = 0.85;

/// The `--history` temporal-stability stride: weights are recomputed at every
/// 16th panel row (0-based index modulo).
pub const HISTORY_STRIDE: usize = 16;

/// One decoded fixture panel: the header's domain order plus the aligned row
/// series.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelData {
    /// Domain names in the header's (fixture's) column order.
    pub domains: Vec<String>,
    /// Per-row timestamps (`ts`), aligned with `counts_rows`.
    pub ts: Vec<String>,
    /// Per-row counts, one `Vec<f32>` per row, each exactly
    /// `domains.len()` wide.
    pub counts_rows: Vec<Vec<f32>>,
    /// Per-row accuracy in `[0, 1]` (bounds are the gate battery's own
    /// assertion; the sibling estimator refuses a saturated series itself).
    pub accuracy: Vec<f32>,
}

/// Why a panel read or a census run refused (typed — the caller refuses LOUD;
/// nothing here falls back silently).
#[derive(Debug)]
pub enum PanelError {
    /// The panel file could not be read.
    Io(std::io::Error),
    /// The file held a valid header but no data rows (a short file).
    NoRows,
    /// The header line was not valid fixture JSON (or the file was empty).
    MalformedHeader(String),
    /// A data line was not valid fixture JSON.
    MalformedRow {
        /// 1-based file line number (the header is line 1).
        line: usize,
        /// The serde parse error's text.
        reason: String,
    },
    /// The header's `format` field is not [`PANEL_FORMAT`].
    UnknownFormat {
        /// The format string the file declared.
        got: String,
    },
    /// A row's `counts` width disagreed with the header's domain count.
    WidthMismatch {
        /// 1-based file line number.
        line: usize,
        /// The width the row carried.
        got: usize,
        /// The domain count the header declared.
        want: usize,
    },
    /// A count or an accuracy value was not finite.
    NonFinite {
        /// 1-based file line number.
        line: usize,
        /// Which field carried it (`counts` or `accuracy`).
        field: &'static str,
    },
    /// The sibling estimator refused the fitted panel (shape, accuracy
    /// domain, observation floors) — see `FitRefusal`'s own `Display`.
    Estimator(katgpt_core::epiplexity_gain::FitRefusal),
}

impl fmt::Display for PanelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "panel read failed: {e}"),
            Self::NoRows => write!(f, "panel holds a header but no data rows"),
            Self::MalformedHeader(reason) => write!(f, "panel header malformed: {reason}"),
            Self::MalformedRow { line, reason } => {
                write!(f, "panel row at line {line} malformed: {reason}")
            }
            Self::UnknownFormat { got } => {
                write!(f, "panel format {got:?} unknown, expected {PANEL_FORMAT:?}")
            }
            Self::WidthMismatch { line, got, want } => write!(
                f,
                "panel row at line {line} carries {got} counts, header declares {want}"
            ),
            Self::NonFinite { line, field } => {
                write!(
                    f,
                    "panel row at line {line} carries a non-finite {field} value"
                )
            }
            Self::Estimator(e) => write!(f, "epiplexity estimator refused the panel: {e}"),
        }
    }
}

impl std::error::Error for PanelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Estimator(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
struct PanelHeader {
    format: String,
    domains: Vec<String>,
    // `note` is fixture provenance; the census ignores it (serde's default
    // unknown-field tolerance covers it).
}

#[derive(Deserialize)]
struct PanelRowJson {
    ts: String,
    counts: Vec<f32>,
    accuracy: f32,
    // `n_runs` is part of the fixture's row shape; the census ignores it.
}

/// Decode the fixture panel at `path` (typed refusals — see [`PanelError`]).
/// Blank lines after the header are tolerated; the row count, the per-row
/// counts width, and finiteness are asserted here. Accuracy bounds are NOT
/// asserted at load — the gate battery pins them for the committed fixture,
/// and the sibling estimator refuses a saturated series itself.
///
/// # Errors
///
/// Typed [`PanelError`] for a missing/unreadable file, an empty or
/// header-only file, a malformed header/row, an unknown `format` string, a
/// counts-width mismatch, or a non-finite value.
pub fn load_panel(path: &Path) -> Result<PanelData, PanelError> {
    let raw = std::fs::read_to_string(path).map_err(PanelError::Io)?;
    let mut lines = raw.lines().enumerate();
    let (_, header_line) = lines
        .next()
        .ok_or_else(|| PanelError::MalformedHeader("file is empty".to_string()))?;
    let header: PanelHeader = serde_json::from_str(header_line)
        .map_err(|e| PanelError::MalformedHeader(e.to_string()))?;
    if header.format != PANEL_FORMAT {
        return Err(PanelError::UnknownFormat { got: header.format });
    }
    if header.domains.is_empty() || header.domains.len() > MAX_DOMAINS {
        return Err(PanelError::MalformedHeader(format!(
            "header declares {} domains, ceiling 1..={MAX_DOMAINS}",
            header.domains.len()
        )));
    }
    let k = header.domains.len();
    let mut ts = Vec::new();
    let mut counts_rows = Vec::new();
    let mut accuracy = Vec::new();
    for (i, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        let line_no = i + 1;
        let row: PanelRowJson =
            serde_json::from_str(line).map_err(|e| PanelError::MalformedRow {
                line: line_no,
                reason: e.to_string(),
            })?;
        if row.counts.len() != k {
            return Err(PanelError::WidthMismatch {
                line: line_no,
                got: row.counts.len(),
                want: k,
            });
        }
        for &c in &row.counts {
            if !c.is_finite() {
                // Defense-in-depth: serde_json's number grammar cannot
                // express a non-finite value (it refuses out-of-range floats
                // itself), so this arm only fires on a future parser swap.
                return Err(PanelError::NonFinite {
                    line: line_no,
                    field: "counts",
                });
            }
        }
        if !row.accuracy.is_finite() {
            return Err(PanelError::NonFinite {
                line: line_no,
                field: "accuracy",
            });
        }
        ts.push(row.ts);
        counts_rows.push(row.counts);
        accuracy.push(row.accuracy);
    }
    if counts_rows.is_empty() {
        return Err(PanelError::NoRows);
    }
    Ok(PanelData {
        domains: header.domains,
        ts,
        counts_rows,
        accuracy,
    })
}

/// The Pile-saturation queue gate (pure): a domain whose predicted remaining
/// gain sits strictly under [`SATURATION_DEMOTE_HEADROOM`] is demoted. `names`
/// pairs positionally with `headrooms`; the answer is in ascending name order.
/// A `None` headroom (a `NoCurve` domain) is already a typed abstain — it is
/// never a demotion.
#[must_use]
pub fn demoted_domains(headrooms: &[Option<f64>], names: &[&str]) -> Vec<String> {
    let mut demoted: Vec<String> = names
        .iter()
        .zip(headrooms.iter())
        .filter_map(|(name, headroom)| match headroom {
            Some(h) if *h < SATURATION_DEMOTE_HEADROOM => Some((*name).to_string()),
            _ => None,
        })
        .collect();
    demoted.sort_unstable();
    demoted
}

/// The dominance refusal (pure): when the top normalized weight reaches
/// [`DOMINANCE_SKIP_WEIGHT`], one domain owns the allocation integral and the
/// machinery refuses to run. `names` pairs positionally with `weights`; a tie
/// at the top resolves to the lexicographically smallest name.
///
/// Ties are detected bit-exactly (`f32::to_bits`), never a tolerance band —
/// the Issue-940 determinism law's fixed strict tie rule. The weights are
/// sigmoid-normalized non-negative, so bit order is value order.
pub fn dominance_skip(weights: &[f32], names: &[&str]) -> Option<String> {
    let mut top_bits = 0_u32;
    for &w in weights {
        top_bits = top_bits.max(w.to_bits());
    }
    if f64::from(f32::from_bits(top_bits)) < DOMINANCE_SKIP_WEIGHT {
        return None;
    }
    let mut best: Option<&str> = None;
    for (w, name) in weights.iter().zip(names.iter()) {
        if w.to_bits() == top_bits {
            match best {
                Some(b) if b <= *name => {}
                _ => best = Some(name),
            }
        }
    }
    best.map(str::to_string)
}

/// Whether a domain's curve was fitted or the domain declined typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveState {
    /// A univariate (or shared cross) curve was fitted.
    Fit,
    /// The domain's count column never moved: `GainCell::NoCurve`, never a
    /// silent gamma.
    NoCurve,
}

/// The rank-census numbers the verdict line prints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CensusSummary {
    /// Domains the census covered.
    pub domains: usize,
    /// The cross-domain bar (`domains − 1`).
    pub bar: usize,
    /// Singular values at or above 10% of the largest (the Phase 0 "effective
    /// rank" proxy).
    pub eff10: usize,
    /// Singular values at or above 1% of the largest.
    pub eff1: usize,
    /// `(Σσ)²/Σσ²` — the effective-rank participation ratio.
    pub participation_ratio: f32,
}

/// One per-domain row of the census table (sorted by domain name).
#[derive(Clone, Debug, PartialEq)]
pub struct DomainRow {
    /// The domain's name.
    pub name: String,
    /// The domain's current raw counts (the panel's final row).
    pub counts_now: f64,
    /// Fitted curve or the typed decline.
    pub curve: CurveState,
    /// The fitted error floor `ε` (`None` for a `NoCurve` domain).
    pub epsilon_floor: Option<f64>,
    /// The remaining learnable gap `gap − ε` (`None` for `NoCurve`).
    pub headroom: Option<f64>,
    /// The marginal gain (`None` for `NoCurve`).
    pub marginal_gain: Option<f32>,
    /// The normalized sigmoid weight at [`SIGMOID_TAU`] (`NoCurve` → 0).
    pub weight: f32,
}

/// One queue entry: a ranked allocable domain, or a trailing typed abstain.
#[derive(Clone, Debug, PartialEq)]
pub struct QueueEntry {
    /// The domain's name.
    pub name: String,
    /// The normalized sigmoid weight (0 for an abstain).
    pub weight: f32,
    /// `false` for the trailing `NoCurve` abstains — never allocable.
    pub allocable: bool,
}

/// One `--history` line: the queue's top-two at a 16th-row snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryLine {
    /// The 0-based panel row the snapshot sits at.
    pub idx: usize,
    /// The row's timestamp.
    pub ts: String,
    /// The top-ranked domain.
    pub top: String,
    /// The top domain's normalized weight.
    pub top_weight: f32,
    /// The second-ranked domain.
    pub second: String,
    /// The second domain's normalized weight.
    pub second_weight: f32,
}

/// The full census report as data — the example prints it, the gate battery
/// pins it.
#[derive(Clone, Debug, PartialEq)]
pub struct CensusReport {
    /// Domain names in the fixture's own column order.
    pub domains: Vec<String>,
    /// Panel rows the census fit over.
    pub rows: usize,
    /// The saturation arm: `"CrossDomain"` or `"Sparse"`.
    pub arm: &'static str,
    /// The rank census behind the verdict.
    pub census: CensusSummary,
    /// Per-domain rows, sorted by domain name.
    pub table: Vec<DomainRow>,
    /// The gain-ordered queue: fitted domains descending by weight (ties →
    /// name order), then the `NoCurve` abstains by name.
    pub queue: Vec<QueueEntry>,
    /// The saturation gate's demotions, sorted by name.
    pub demoted: Vec<String>,
    /// The dominance refusal's domain, when one owns the integral.
    pub dominance_skip: Option<String>,
    /// The τ the sigmoid weights were computed at ([`SIGMOID_TAU`]).
    pub tau: f32,
    /// The `--history` temporal-stability lines; empty when not requested.
    pub history: Vec<HistoryLine>,
}

fn census_summary(c: &RankCensus) -> CensusSummary {
    CensusSummary {
        domains: c.domains,
        bar: c.bar,
        eff10: c.eff10,
        eff1: c.eff1,
        participation_ratio: c.participation_ratio,
    }
}

/// Rank `(name, weight)` pairs for the queue and the history top-two:
/// descending weight, ties in ascending name order (deterministic — names are
/// unique per panel).
fn rank_desc_weight_ties_name(pairs: &[(String, f32)]) -> Vec<(String, f32)> {
    let mut ranked: Vec<(String, f32)> = pairs.to_vec();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    ranked
}

/// Build the gain-ordered queue from the (name-sorted) table: fitted domains
/// descending by weight (ties → name order), then the `NoCurve` abstains by
/// name — typed abstains trail, never interleave.
fn build_queue(table: &[DomainRow]) -> Vec<QueueEntry> {
    let fitted_pairs: Vec<(String, f32)> = table
        .iter()
        .filter(|r| r.curve == CurveState::Fit)
        .map(|r| (r.name.clone(), r.weight))
        .collect();
    let mut queue: Vec<QueueEntry> = rank_desc_weight_ties_name(&fitted_pairs)
        .into_iter()
        .map(|(name, weight)| QueueEntry {
            name,
            weight,
            allocable: true,
        })
        .collect();
    let mut abstains: Vec<&DomainRow> = table
        .iter()
        .filter(|r| r.curve == CurveState::NoCurve)
        .collect();
    abstains.sort_by(|a, b| a.name.cmp(&b.name));
    queue.extend(abstains.into_iter().map(|r| QueueEntry {
        name: r.name.clone(),
        weight: r.weight,
        allocable: false,
    }));
    queue
}

/// The `--history` walk: re-seat the estimator's counts at every
/// [`HISTORY_STRIDE`]th row, recompute the weights, and record the ranked
/// top-two among the FITTED domains (abstains are unallocable and never
/// top/second). A snapshot with fewer than two fitted domains emits no line
/// (the top/second read is undefined there). Runs after the report's
/// table/queue/gates are captured — it mutates only the estimator.
fn walk_history(
    est: &mut GainEstimator,
    data: &PanelData,
    fitted_mask: &[bool; MAX_DOMAINS],
) -> Result<Vec<HistoryLine>, PanelError> {
    let k = data.domains.len();
    let mut lines = Vec::new();
    for (idx, row) in data.counts_rows.iter().enumerate() {
        if idx % HISTORY_STRIDE != 0 {
            continue;
        }
        est.update_counts(row).map_err(PanelError::Estimator)?;
        let w = est.sigmoid_weights(SIGMOID_TAU);
        let pairs: Vec<(String, f32)> = (0..k)
            .filter(|&d| fitted_mask[d])
            .map(|d| (data.domains[d].clone(), w[d]))
            .collect();
        let ranked = rank_desc_weight_ties_name(&pairs);
        if let [top, second, ..] = ranked.as_slice() {
            lines.push(HistoryLine {
                idx,
                ts: data.ts[idx].clone(),
                top: top.0.clone(),
                top_weight: top.1,
                second: second.0.clone(),
                second_weight: second.1,
            });
        }
    }
    Ok(lines)
}

/// Run the full census over the fixture at `panel_path`: decode, fit the
/// sibling `GainEstimator`, and build the verdict, the per-domain table, the
/// gain-ordered queue, the two queue gates, and (when `history`) the 16th-row
/// temporal-stability walk. The table, queue, and gates reflect the panel's
/// FINAL row (the fit seats it); the history walk runs after and only fills
/// [`CensusReport::history`].
///
/// # Errors
///
/// Typed [`PanelError`] — the loader's refusals, or the sibling estimator's
/// own `FitRefusal` passed through as [`PanelError::Estimator`].
pub fn run_census(panel_path: &Path, history: bool) -> Result<CensusReport, PanelError> {
    let data = load_panel(panel_path)?;
    let k = data.domains.len();
    let n = data.counts_rows.len();
    let mut flat: Vec<f32> = Vec::with_capacity(k * n);
    for row in &data.counts_rows {
        flat.extend_from_slice(row);
    }
    let panel = MixPanel::new(k, n, &flat);
    let mut est = GainEstimator::fit(&panel, &data.accuracy).map_err(PanelError::Estimator)?;

    let weights = est.sigmoid_weights(SIGMOID_TAU);
    let gains = est.marginal_gain();
    let counts_now = est.counts_now();

    let (arm, census) = match est.saturation() {
        Saturation::CrossDomain(c) => ("CrossDomain", census_summary(c)),
        Saturation::Sparse(c) => ("Sparse", census_summary(c)),
    };

    let mut table: Vec<DomainRow> = Vec::with_capacity(k);
    let mut headroom_by_domain: [Option<f64>; MAX_DOMAINS] = [None; MAX_DOMAINS];
    let mut fitted_mask: [bool; MAX_DOMAINS] = [false; MAX_DOMAINS];
    let mut names: Vec<&str> = Vec::with_capacity(k);
    for (d, name) in data.domains.iter().enumerate() {
        let headroom = est.headroom_of(d);
        headroom_by_domain[d] = headroom;
        names.push(name.as_str());
        let (curve, gain) = match gains[d] {
            GainCell::Gain(g) => (CurveState::Fit, Some(g)),
            GainCell::NoCurve => (CurveState::NoCurve, None),
        };
        fitted_mask[d] = curve == CurveState::Fit;
        table.push(DomainRow {
            name: name.clone(),
            counts_now: counts_now[d],
            curve,
            epsilon_floor: est.epsilon_floor_of(d),
            headroom,
            marginal_gain: gain,
            weight: weights[d],
        });
    }
    table.sort_by(|a, b| a.name.cmp(&b.name));

    let queue = build_queue(&table);
    let demoted = demoted_domains(&headroom_by_domain[..k], &names);
    let dominance_skip = dominance_skip(&weights[..k], &names);

    let history_lines = if history {
        walk_history(&mut est, &data, &fitted_mask)?
    } else {
        Vec::new()
    };

    Ok(CensusReport {
        domains: data.domains,
        rows: n,
        arm,
        census,
        table,
        queue,
        demoted,
        dominance_skip,
        tau: SIGMOID_TAU,
        history: history_lines,
    })
}
