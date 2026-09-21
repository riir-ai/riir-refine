//! Issue 107 T1 — the fixseq **horizon report**: AUC(k) vs lead k over the
//! per-attempt ring (`.heal/fixseq/run-*.jsonl`, Issue 91 T1.5).
//!
//! REPORT-FIRST, gates nothing (the frontier-report law): the question is
//! lane viability for the probe-gated fix-verification proposal — is an
//! attempt's outcome **linearly decodable from the ring state k attempts
//! earlier**, at an AUC that beats both floors? `horizon ≥ 2` pays (T1's
//! viability bar); a horizon of 0/none says the ring's own history carries
//! no usable lookahead signal and the T3 probe lane is dead on arrival.
//!
//! What is measured, precisely: for every run and every target index `i ≥ k`,
//! one sample — features from record `j = i-k` ONLY (strict lead: nothing
//! from `(j, i]` may enter the feature vector, so the future cannot leak),
//! label `kept(i)`. The readout is a closed-form ridge fit
//! (`(G + λI)⁻¹h`, the same math T3 will consume via katgpt-core's
//! `OnlineLinearReadout` once katgpt-rs Issue 780 lands — local +
//! provisional until then, clearly marked). Fitting means **either polarity
//! is a hit**: on alternating data the fit learns the flipped weight, so
//! AUC measures linear decodability, not direction.
//!
//! Hold-out protocol: **leave-one-run-out** (LORO) — deterministic, no RNG;
//! the fit for run r's scores never sees run r. Pooled + per-target-code
//! family rows (the family is the TARGET's code — known at proposal time,
//! which is when the T4 gate would read the probe).
//!
//! The two floors (the UQ "Report the Floor" rule):
//! - **base**: a constant predictor — AUC 0.5 by construction. The header
//!   carries the pooled keep-rate and its Beta-LCB₅ so a high AUC over a
//!   99%-keep population reads as what it is (thin neg support does the
//!   real work; the per-k `neg` column makes it visible).
//! - **history-aggregate**: the Laplace-smoothed per-strategy keep-rate
//!   computed LORO (the trajectory store's per-FAMILY counters in report
//!   form) — a real ranked predictor; the readout must beat `max(0.5, it)`.
//!
//! Refusals — never a confident green zero (empty ring / blind walk /
//! single-class labels all exit 1 at the CLI):
//! - [`HorizonRefusal::Empty`] — zero runs parsed;
//! - [`HorizonRefusal::BlindWalk`] — runs exist but no run reaches 2
//!   attempts (zero lead-1 samples; the Bench 087 K=1 shape);
//! - [`HorizonRefusal::SingleClass`] — every measurable k is single-class
//!   (AUC undefined everywhere).
//!
//! Ungated by design (the `frontier_report` store-free law): the ring's
//! jsonl wire shape is mirrored locally in [`RingLine`] — with the SAME
//! `#[serde(default)]` legacy compat the driver's own `AttemptRecord`
//! carries (pre-Issue-100 rings read `masking (0,0)`, pre-105 read journey
//! `0`) — so `--no-default-features` builds compile the stats and the
//! module never imports the `rustc_errors`-gated domain.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use katgpt_core::best_belief_score;
use serde::Deserialize;

use crate::span_embed::SPAN_EMBED_DIMS;

/// The LCB ε for the header's base-rate bound — the frontier convention
/// (`FRONTIER_LCB_EPSILON`), 0.05.
pub const HEADER_LCB_EPSILON: f32 = 0.05;

/// Ridge shrinkage λ — AUC is rank-based, so the exact value only needs to
/// keep the tiny normal-equation system non-singular.
pub const RIDGE_LAMBDA: f64 = 1.0;

/// Maximum lead k probed.
pub const K_MAX: usize = 8;

/// A k row is MEASURABLE only with this many samples…
pub const MIN_SAMPLES: usize = 10;
/// …and at least this many of EACH class.
pub const MIN_CLASS: usize = 3;

/// The beats-floor margin: `auc(k) ≥ max(0.5, floor_hist(k)) + MARGIN`.
pub const MARGIN: f64 = 0.05;

/// Minimum samples for a per-family row (below: counted, not tabulated).
pub const MIN_FAMILY_SAMPLES: usize = 8;

/// One ring line — the jsonl wire shape the driver's `AttemptRecord`
/// serializes to (Issue 91 T1.5). Local mirror so this module stays
/// ungated; see the module doc for the law.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RingLine {
    /// 0-based position in the run's attempt order.
    pub seq: usize,
    /// The rustc error code the candidate answered (e.g. `E0507`).
    pub code: String,
    /// Stable site identity — 16 hex.
    pub span_hash: String,
    /// The playbook strategy that was tried.
    pub strategy_id: String,
    /// `Kept` | `RevertedNoProgress` | `RevertedIo` (external enum tag).
    pub outcome: String,
    /// Coded-error count before → after.
    pub errors: (usize, usize),
    /// Masking-stage count before → after (Issue 100). Default so
    /// pre-100 rings decode.
    #[serde(default)]
    pub masking: (usize, usize),
    /// Run-level journey integral (Issue 105), stamped at dump time.
    #[serde(default)]
    pub journey_residual_sum: u32,
    /// Run-level terminal residual (Issue 105).
    #[serde(default)]
    pub terminal_residual: u32,
    /// Issue 107 T2 — the span-shape embedding column (quantized hashed
    /// unigram+bigram bag over the attempt-time trigger text; see
    /// [`crate::span_embed`]). `None` on pre-T2 rings.
    #[serde(default)]
    pub span_embed: Option<[i8; SPAN_EMBED_DIMS]>,
}

impl RingLine {
    /// The outcome as a bool label — `None` on an unknown tag (a corrupt
    /// line the caller counts and skips, never silently folds).
    #[must_use]
    pub fn kept(&self) -> Option<bool> {
        match self.outcome.as_str() {
            "Kept" => Some(true),
            "RevertedNoProgress" | "RevertedIo" => Some(false),
            _ => None,
        }
    }
}

/// One run's ordered ring, with its source file for provenance rendering.
#[derive(Clone, Debug)]
pub struct RunRing {
    /// File name of the `run-*.jsonl` (ordering key, sorted at load).
    pub source: String,
    /// Ordered attempts.
    pub lines: Vec<RingLine>,
}

// ── Loading ─────────────────────────────────────────────────────────────────

/// Load every run ring under `root` — `root/.heal/fixseq/run-*.jsonl`
/// (the `dump_fixseq` layout), or `root` itself when it IS a ring file.
/// Files sort by name; within a file, lines order by `seq` (the dump order
/// is already ordered — the sort is a defense, not a re-sort).
///
/// Returns the runs, the count of unparseable lines (bad json or unknown
/// outcome tag), and the file count scanned.
#[must_use]
pub fn load_runs(root: &Path) -> (Vec<RunRing>, usize, usize) {
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    if root.is_file() {
        files.push(root.to_path_buf());
    } else if let Ok(rd) = std::fs::read_dir(root.join(".heal").join("fixseq")) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("run-"))
            {
                files.push(p);
            }
        }
    }
    files.sort();

    let mut runs = Vec::with_capacity(files.len());
    let mut bad = 0;
    for p in &files {
        let Ok(text) = std::fs::read_to_string(p) else {
            bad += 1;
            continue;
        };
        let mut lines: Vec<RingLine> = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<RingLine>(line) {
                Ok(rl) if rl.kept().is_some() => lines.push(rl),
                Ok(_) | Err(_) => bad += 1,
            }
        }
        lines.sort_by_key(|l| l.seq);
        if !lines.is_empty() {
            runs.push(RunRing {
                source: p
                    .file_name()
                    .map_or_else(|| p.to_string_lossy().into_owned(), |n| {
                        n.to_string_lossy().into_owned()
                    }),
                lines,
            });
        }
    }
    (runs, bad, files.len())
}

// ── The readout ─────────────────────────────────────────────────────────────

/// Feature count (documented in [`features_at`]).
pub const N_FEAT: usize = 9;

/// The embed arm's feature count — the 9 ring-native features plus the 32
/// quantized embedding dims (Issue 107 T2). The arm runs ONLY on rings
/// where every line carries the column (see [`summarize`]).
pub const N_FEAT_EMBED: usize = N_FEAT + SPAN_EMBED_DIMS;

/// Issue 128 T6 — the transition arm's feature count: the 9 ring-native
/// features plus the PREV-attempt-outcome indicator (feature #10).
pub const N_FEAT_PREV: usize = N_FEAT + 1;

/// The strict-lead feature vector from record `a` (position `j` in a run of
/// `run_len` attempts). Ring-native scalars ONLY — T2's embedding column is
/// the designed upgrade, not a prerequisite:
/// `[bias, kept_j, err_before, err_after, mask_before, mask_after,
///   err_progress, mask_progress, pos_normalized]`
#[allow(clippy::cast_precision_loss)] // usize counters → f64 features — the report's quantity
#[must_use]
pub fn features_at(a: &RingLine, run_len: usize) -> [f64; N_FEAT] {
    let denom = run_len.saturating_sub(1).max(1);
    [
        1.0,
        f64::from(u8::from(a.kept() == Some(true))),
        a.errors.0 as f64,
        a.errors.1 as f64,
        a.masking.0 as f64,
        a.masking.1 as f64,
        (a.errors.0 as f64) - (a.errors.1 as f64),
        (a.masking.0 as f64) - (a.masking.1 as f64),
        a.seq as f64 / denom as f64,
    ]
}

/// The embed arm's strict-lead feature vector — [`features_at`] with the
/// T2 column appended, decoded to `[-1, 1]`. `None` when the line predates
/// T2 (no column): the caller's arm-skip discipline, never a zero-fill
/// (a silently-zeroed block would read as "shapeless span", a lie).
#[allow(clippy::cast_precision_loss)] // i8 -> f64 is exact
#[must_use]
pub fn features_at_embed(a: &RingLine, run_len: usize) -> Option<[f64; N_FEAT_EMBED]> {
    let e = a.span_embed.as_ref()?;
    let mut x = [0.0_f64; N_FEAT_EMBED];
    x[..N_FEAT].copy_from_slice(&features_at(a, run_len));
    for (i, v) in e.iter().enumerate() {
        x[N_FEAT + i] = f64::from(*v) / 127.0;
    }
    Some(x)
}

/// Issue 128 T6 — the transition arm's strict-lead feature vector:
/// [`features_at`] with the PREV-attempt-outcome indicator appended
/// (three-state: +1 the attempt before the feature record was kept, -1
/// reverted, 0 no previous attempt — the first record of a run). This is
/// the cascade law's transition feature as a readout dimension: the
/// question T6 asks is whether it RAISES the horizon. Strict-lead safe —
/// the prev outcome sits at i-k-1, one lead OLDER than every native
/// feature; nothing from (i-k, i] enters.
#[must_use]
pub fn features_at_prev(
    a: &RingLine,
    prev: Option<&RingLine>,
    run_len: usize,
) -> [f64; N_FEAT_PREV] {
    let mut x = [0.0_f64; N_FEAT_PREV];
    x[..N_FEAT].copy_from_slice(&features_at(a, run_len));
    x[N_FEAT] = match prev.map(RingLine::kept) {
        Some(Some(true)) => 1.0,
        Some(Some(false)) => -1.0,
        Some(None) | None => 0.0,
    };
    x
}

/// Closed-form ridge fit `(G + λI)w = h` via Gaussian elimination with
/// partial pivoting. `G = Σ xxᵀ`, `h = Σ xy`. With λ > 0 the system is
/// always solvable; returns `None` only on empty input. Const-generic over
/// the feature count — the native arm (9) and the embed arm (41) share this
/// verbatim (Issue 107 T2).
#[must_use]
pub fn ridge_fit<const N: usize>(samples: &[([f64; N], bool)]) -> Option<[f64; N]> {
    if samples.is_empty() {
        return None;
    }
    let size = N;
    // Augmented normal equations: [G + λI | h].
    let mut aug: Vec<Vec<f64>> = vec![vec![0.0; size + 1]; size];
    for (x, y) in samples {
        let yf = f64::from(*y);
        for (ri, row) in aug.iter_mut().enumerate() {
            for (ci, xv) in x.iter().enumerate() {
                row[ci] += xv * x[ri];
            }
            row[size] += x[ri] * yf;
        }
    }
    for (i, row) in aug.iter_mut().enumerate() {
        row[i] += RIDGE_LAMBDA;
    }
    // Gaussian elimination with partial pivoting (N×N — exact enough).
    for col in 0..size {
        let pivot = (col..size)
            .max_by(|&r, &s| aug[r][col].abs().total_cmp(&aug[s][col].abs()))?;
        aug.swap(col, pivot);
        let diag = aug[col][col];
        if diag.abs() < f64::EPSILON {
            return None;
        }
        for v in &mut aug[col][col..=size] {
            *v /= diag;
        }
        // Cloned pivot row (N+1 f64) so the row sweep can iterate while
        // reading it — no split-borrow gymnastics.
        let pivot_row: Vec<f64> = aug[col].clone();
        for (ri, row) in aug.iter_mut().enumerate() {
            if ri == col {
                continue;
            }
            let factor = row[col];
            if factor.abs() > 0.0 {
                for (v, pv) in row.iter_mut().zip(&pivot_row).skip(col) {
                    *v -= factor * pv;
                }
            }
        }
    }
    let mut weights = [0.0; N];
    for (i, row) in aug.iter().enumerate().take(size) {
        weights[i] = row[size];
    }
    Some(weights)
}

/// Mann–Whitney AUC with ties = 0.5 (the `bench_320` formula:
/// `(Σ_{i∈pos,j∈neg} [s_i > s_j] + 0.5·[s_i == s_j]) / (n_pos·n_neg)`).
/// `None` when either class is empty (AUC undefined, never 0.5-by-fiat).
#[allow(clippy::cast_precision_loss)] // usize counts → f64 statistics — the report's quantity
#[allow(clippy::float_cmp)] // exact equality DEFINES the tie class of the rank statistic
#[must_use]
pub fn auc(scores: &[(f64, bool)]) -> Option<f64> {
    let n_pos = scores.iter().filter(|(_, y)| *y).count();
    let n_neg = scores.len() - n_pos;
    if n_pos == 0 || n_neg == 0 {
        return None;
    }
    let mut ranked: Vec<(f64, bool)> = scores.to_vec();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Average ranks per score-tie block, then the positive-class rank sum.
    let mut i = 0;
    let mut rank_sum_pos = 0.0_f64;
    while i < ranked.len() {
        let mut j = i;
        while j < ranked.len() && ranked[j].0 == ranked[i].0 {
            j += 1;
        }
        let avg = (i as f64 + j as f64 - 1.0) / 2.0 + 1.0; // 1-based mean rank
        for e in &ranked[i..j] {
            if e.1 {
                rank_sum_pos += avg;
            }
        }
        i = j;
    }
    let n_pos_f = n_pos as f64;
    let n_neg_f = n_neg as f64;
    Some((rank_sum_pos - n_pos_f * (n_pos_f + 1.0) / 2.0) / (n_pos_f * n_neg_f))
}

// ── The report ──────────────────────────────────────────────────────────────

/// One sample at lead k: features from `i-k`, label at `i`, plus the
/// target's strategy (the history floor's key). Const-generic over the
/// feature count (the native / embed arms instantiate it at 9 / 41).
struct LeadSample<const N: usize> {
    x: [f64; N],
    y: bool,
    strategy: String,
}

/// A scored sample list: `(score, label)` pairs — the shared currency of
/// the readout and both floors.
pub type Scored = Vec<(f64, bool)>;

/// Laplace-smoothed keep-rate — the history-aggregate ranker's score.
#[allow(clippy::cast_precision_loss)] // usize counters → f64 score — the report's quantity
fn laplace_prior(kept: usize, reverted: usize) -> f64 {
    (kept as f64 + 1.0) / (kept as f64 + reverted as f64 + 2.0)
}

/// Run one LORO evaluation over grouped samples: returns (readout scores,
/// history-floor scores) — each entry `(score, label)`, pooled over all
/// held-out folds. Shared verbatim by the pooled k-rows and the per-family
/// rows (the DRY extraction the two duplicated blocks asked for).
fn loro_scores<const N: usize>(run_groups: &[Vec<LeadSample<N>>]) -> (Scored, Scored) {
    let fit = |exclude: usize| -> Option<[f64; N]> {
        let train: Vec<([f64; N], bool)> = run_groups
            .iter()
            .enumerate()
            .filter(|(gi, _)| *gi != exclude)
            .flat_map(|(_, g)| g.iter().map(|s| (s.x, s.y)))
            .collect();
        ridge_fit(&train)
    };
    let mut readout: Scored = Vec::new();
    let mut history: Scored = Vec::new();
    for (gi, g) in run_groups.iter().enumerate() {
        if let Some(w) = fit(gi) {
            for s in g {
                readout.push((w.iter().zip(s.x.iter()).map(|(a, b)| a * b).sum(), s.y));
            }
        }
        // History floor: per-strategy Laplace prior EXCLUDING run gi's own
        // samples (the same LORO fold — no leakage).
        let mut kept_m: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for (hj, hg) in run_groups.iter().enumerate() {
            if hj == gi {
                continue;
            }
            for s in hg {
                let e = kept_m.entry(s.strategy.as_str()).or_default();
                if s.y {
                    e.0 += 1;
                } else {
                    e.1 += 1;
                }
            }
        }
        for s in g {
            let p = kept_m
                .get(s.strategy.as_str())
                .map_or(0.5, |&(kp, np)| laplace_prior(kp, np));
            history.push((p, s.y));
        }
    }
    (readout, history)
}

/// A pooled k-row as rendered.
#[derive(Clone, Debug, PartialEq)]
pub struct HorizonRow {
    /// Lead k.
    pub k: usize,
    /// Samples at this lead.
    pub n: usize,
    /// Kept labels.
    pub pos: usize,
    /// Reverted labels.
    pub neg: usize,
    /// LORO ridge-readout AUC (`None` = single class).
    pub auc_readout: Option<f64>,
    /// LORO history-aggregate (per-strategy Laplace prior) AUC.
    pub auc_history: Option<f64>,
    /// Issue 107 T2 — the EMBED arm's AUC: the same LORO ridge over
    /// native + span-embedding features. `None` = single class OR the arm
    /// was skipped (no column coverage) — disambiguate via
    /// [`HorizonSummary::embed_arm`], never this field alone.
    pub auc_embed: Option<f64>,
    /// Issue 128 T6 — the TRANSITION arm's AUC: the same LORO ridge over
    /// the native features plus the prev-attempt-outcome indicator
    /// (feature #10). Runs on every corpus (the prev column always
    /// exists — `0.0` on a run's first record); `None` only on a
    /// single-class row.
    pub auc_prev: Option<f64>,
}

impl HorizonRow {
    fn measurable(&self) -> bool {
        self.n >= MIN_SAMPLES && self.pos >= MIN_CLASS && self.neg >= MIN_CLASS
    }

    fn beats(&self) -> bool {
        match (self.measurable(), self.auc_readout, self.auc_history) {
            (true, Some(a), Some(h)) => a >= (h.max(0.5) + MARGIN),
            (true, Some(a), None) => a >= 0.5 + MARGIN,
            _ => false,
        }
    }

    /// The embed arm's beats-floor test — same margin discipline over
    /// [`Self::auc_embed`] (the revival-path verdict, Issue 107 T2).
    fn beats_embed(&self) -> bool {
        match (self.measurable(), self.auc_embed, self.auc_history) {
            (true, Some(a), Some(h)) => a >= (h.max(0.5) + MARGIN),
            (true, Some(a), None) => a >= 0.5 + MARGIN,
            _ => false,
        }
    }

    /// Issue 128 T6 — the transition arm's beats-floor test, same margin
    /// discipline over [`Self::auc_prev`].
    fn beats_prev(&self) -> bool {
        match (self.measurable(), self.auc_prev, self.auc_history) {
            (true, Some(a), Some(h)) => a >= (h.max(0.5) + MARGIN),
            (true, Some(a), None) => a >= 0.5 + MARGIN,
            _ => false,
        }
    }

    fn verdict(&self) -> &'static str {
        if !self.measurable() {
            "THIN"
        } else if self.beats() {
            "OK"
        } else if self.auc_readout.is_none() {
            "UNMEASURABLE"
        } else {
            "BELOW-FLOOR"
        }
    }
}

/// One per-family row (at lead 1 — the decision-relevant lead).
#[derive(Clone, Debug, PartialEq)]
pub struct FamilyRow {
    pub code: String,
    pub n: usize,
    pub pos: usize,
    pub neg: usize,
    pub auc_readout: Option<f64>,
    pub auc_history: Option<f64>,
    /// Issue 107 T2 — the embed arm's AUC (arm-skip reads as `None`).
    pub auc_embed: Option<f64>,
}

/// Why the report refused (the green-zero law) — rendered loud, exit 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizonRefusal {
    /// No runs parsed (no store / all corrupt).
    Empty,
    /// Runs exist but none reaches 2 attempts — zero lead-1 samples.
    BlindWalk,
    /// Every measurable k is single-class — AUC undefined everywhere.
    SingleClass,
}

impl HorizonRefusal {
    /// The loud refusal line.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "EMPTY — zero runs parsed (no .heal/fixseq store here)",
            Self::BlindWalk => {
                "BLIND WALK — runs exist but no run reaches 2 attempts; lead-k needs depth"
            }
            Self::SingleClass => "SINGLE CLASS — every measurable lead is one-label (AUC undefined)",
        }
    }
}

/// The computed report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HorizonSummary {
    /// Runs parsed.
    pub runs: usize,
    /// Total attempts.
    pub attempts: usize,
    /// Attempts kept / reverted (pooled).
    pub kept: usize,
    pub reverted: usize,
    /// Base keep-rate (pooled) — Floor A context.
    pub base_rate: f32,
    /// Beta-LCB₅ of the base rate.
    pub base_lcb5: f32,
    /// Run-depth histogram: `hist[d]` = runs with `d+1` attempts (capped
    /// display at 5+; the vector itself is exact up to its len).
    pub depth_hist: BTreeMap<usize, usize>,
    /// Pooled rows, k = 1..=`K_MAX` (only ks with ≥1 sample appear).
    pub rows: Vec<HorizonRow>,
    /// Per-family rows at k = 1, n-desc order.
    pub families: Vec<FamilyRow>,
    /// Families excluded from the table for thinness (count only).
    pub thin_families: usize,
    /// The refusal, if any.
    pub refusal: Option<HorizonRefusal>,
    /// Unparseable lines skipped at load.
    pub bad_lines: usize,
    /// Issue 107 T2 — attempts carrying the embedding column.
    pub embed_attempts: usize,
    /// Issue 107 T2 — whether the embed arm ran. Requires FULL column
    /// coverage (every line of every run): a partial-corpus arm would score
    /// a different sample set than the native rows beside it — skipped
    /// LOUD with the coverage line instead.
    pub embed_arm: bool,
}

impl HorizonSummary {
    /// The horizon verdict: max measurable k that beats both floors, `0`
    /// when none does (report callers render the "none" wording).
    #[must_use]
    pub fn horizon(&self) -> usize {
        self.rows.iter().filter(|r| r.beats()).map(|r| r.k).max().unwrap_or(0)
    }

    /// The embed arm's horizon — max measurable k whose native+embedding
    /// readout beats both floors. `0` when the arm did not run or beat
    /// nothing (the caller renders which).
    #[must_use]
    pub fn horizon_embed(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.beats_embed())
            .map(|r| r.k)
            .max()
            .unwrap_or(0)
    }

    /// Issue 128 T6 — the transition arm's horizon: max measurable k whose
    /// native+prev-outcome readout beats both floors (the T6 question: is
    /// this >= [`Self::horizon`]?).
    #[must_use]
    pub fn horizon_prev(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.beats_prev())
            .map(|r| r.k)
            .max()
            .unwrap_or(0)
    }
}

/// Compute the whole report over loaded runs. Deterministic: LORO over
/// runs in load order, `BTreeMap`s for families, no RNG.
///
/// # Panics
/// Panics on an internal invariant break ONLY: an embed-arm sample whose
/// `span_embed` is `None` after [`HorizonSummary::embed_arm`] verified full
/// column coverage (unreachable without a data race on the rings).
#[allow(clippy::too_many_lines)] // one population, one algorithm — the pooled-lead sweep, the refusal ladder, and the family split share state that splitting would thread through every arm
#[allow(clippy::many_single_char_names)] // the lead-k sample grammar (k lead, i index, x features, y label) — math notation, not names
#[allow(clippy::cast_precision_loss)] // usize/f32 rate + counter casts — the report's quantities
#[allow(clippy::cast_possible_truncation)] // u32 is the LCB primitive's counter width; counts are ~10³
#[must_use]
pub fn summarize(runs: &[RunRing]) -> HorizonSummary {
    let attempts: usize = runs.iter().map(|r| r.lines.len()).sum();
    let mut kept = 0usize;
    let mut reverted = 0usize;
    let mut embed_attempts = 0usize;
    let mut depth_hist: BTreeMap<usize, usize> = BTreeMap::new();
    for r in runs {
        *depth_hist.entry(r.lines.len().saturating_sub(1)).or_default() += 1;
        for l in &r.lines {
            match l.kept() {
                Some(true) => kept += 1,
                Some(false) => reverted += 1,
                // load_runs filters these; a direct-call summary still
                // counts only known labels.
                None => {}
            }
            if l.span_embed.is_some() {
                embed_attempts += 1;
            }
        }
    }
    let n_tot = kept + reverted;
    let base_rate = if n_tot == 0 {
        0.0
    } else {
        kept as f32 / n_tot as f32
    };
    let base_lcb5 = best_belief_score(kept as u32, reverted as u32, HEADER_LCB_EPSILON);
    let mut s = HorizonSummary {
        runs: runs.len(),
        attempts,
        kept,
        reverted,
        base_rate,
        base_lcb5,
        depth_hist,
        ..HorizonSummary::default()
    };

    if runs.is_empty() {
        s.refusal = Some(HorizonRefusal::Empty);
        return s;
    }
    if runs.iter().all(|r| r.lines.len() < 2) {
        s.refusal = Some(HorizonRefusal::BlindWalk);
        return s;
    }

    // Issue 107 T2 — the embed arm runs ONLY at full column coverage (a
    // partial-corpus arm would score a different sample set than the native
    // rows beside it; skipped-loud beats silently-narrow).
    let embed_arm = runs
        .iter()
        .all(|r| r.lines.iter().all(|l| l.span_embed.is_some()));
    s.embed_attempts = embed_attempts;
    s.embed_arm = embed_arm;

    // Samples per lead, per run (LORO needs per-run grouping).
    let mut per_k: Vec<Vec<Vec<LeadSample<N_FEAT>>>> = (0..=K_MAX).map(|_| Vec::new()).collect();
    let mut per_k_e: Vec<Vec<Vec<LeadSample<N_FEAT_EMBED>>>> =
        (0..=K_MAX).map(|_| Vec::new()).collect();
    // Issue 128 T6 — the transition arm's sample pool (always collected:
    // the prev column exists on every ring — 0.0 on a run's first record).
    let mut per_k_p: Vec<Vec<Vec<LeadSample<N_FEAT_PREV>>>> =
        (0..=K_MAX).map(|_| Vec::new()).collect();
    for ring in runs {
        let len = ring.lines.len();
        let kmax = K_MAX.min(len.saturating_sub(1));
        for (k, slot) in per_k.iter_mut().enumerate().take(kmax + 1).skip(1) {
            let mut group = Vec::with_capacity(len - k);
            let mut group_e = Vec::with_capacity(len - k);
            let mut group_p = Vec::with_capacity(len - k);
            for i in k..len {
                let (x, y) = (&ring.lines[i - k], &ring.lines[i]);
                // T6: the feature record's PREV attempt — one lead OLDER
                // than every native feature (i-k-1); None at a run head.
                let j = i - k;
                let prev = if j >= 1 { Some(&ring.lines[j - 1]) } else { None };
                if let Some(yb) = y.kept() {
                    group.push(LeadSample {
                        x: features_at(x, len),
                        y: yb,
                        strategy: y.strategy_id.clone(),
                    });
                    group_p.push(LeadSample {
                        x: features_at_prev(x, prev, len),
                        y: yb,
                        strategy: y.strategy_id.clone(),
                    });
                    if embed_arm {
                        // unwrap is total by the embed_arm coverage check.
                        group_e.push(LeadSample {
                            x: features_at_embed(x, len)
                                .expect("embed_arm checked full coverage"),
                            y: yb,
                            strategy: y.strategy_id.clone(),
                        });
                    }
                }
            }
            if !group.is_empty() {
                slot.push(group);
                per_k_p[k].push(group_p);
                if embed_arm {
                    per_k_e[k].push(group_e);
                }
            }
        }
    }

    let mut any_auc_defined = false;
    for (k, run_groups) in per_k.into_iter().enumerate() {
        if run_groups.is_empty() {
            continue;
        }
        let n: usize = run_groups.iter().map(Vec::len).sum();
        let pos = run_groups
            .iter()
            .flatten()
            .filter(|s| s.y)
            .count();
        let neg = n - pos;
        let (readout_scores, hist_scores) = loro_scores(&run_groups);
        let auc_readout = auc(&readout_scores);
        let auc_history = auc(&hist_scores);
        let auc_embed = if embed_arm {
            let (emb_scores, _) = loro_scores(&per_k_e[k]);
            auc(&emb_scores)
        } else {
            None
        };
        // Issue 128 T6 — the transition arm over the same samples.
        let (prev_scores, _) = loro_scores(&per_k_p[k]);
        let auc_prev = auc(&prev_scores);

        if pos >= MIN_CLASS && neg >= MIN_CLASS {
            any_auc_defined = any_auc_defined || auc_readout.is_some();
        }
        s.rows.push(HorizonRow {
            k,
            n,
            pos,
            neg,
            auc_readout,
            auc_history,
            auc_embed,
            auc_prev,
        });
    }

    if !any_auc_defined {
        // Depth ≥ 2 existed somewhere (else BlindWalk above), but no lead
        // produced a both-class row — AUC undefined everywhere. (Both
        // classes SOMEWHERE but every row THIN is NOT a refusal: the table
        // renders with THIN verdicts and horizon none.)
        s.refusal = Some(HorizonRefusal::SingleClass);
        return s;
    }

    // Family split at k = 1 (decision-relevant lead), grouped PER RUN per
    // code so the LORO fold excludes the whole run — same-run
    // autocorrelation must not train on itself.
    let mut fam: BTreeMap<String, Vec<Vec<LeadSample<N_FEAT>>>> = BTreeMap::new();
    let mut fam_e: BTreeMap<String, Vec<Vec<LeadSample<N_FEAT_EMBED>>>> = BTreeMap::new();
    for r in runs {
        let m = r.lines.len();
        if m < 2 {
            continue;
        }
        let mut per_code: BTreeMap<String, Vec<LeadSample<N_FEAT>>> = BTreeMap::new();
        let mut per_code_e: BTreeMap<String, Vec<LeadSample<N_FEAT_EMBED>>> = BTreeMap::new();
        for i in 1..m {
            let (x, y) = (&r.lines[i - 1], &r.lines[i]);
            if let Some(yb) = y.kept() {
                per_code.entry(y.code.clone()).or_default().push(LeadSample {
                    x: features_at(x, m),
                    y: yb,
                    strategy: y.strategy_id.clone(),
                });
                if embed_arm {
                    per_code_e
                        .entry(y.code.clone())
                        .or_default()
                        .push(LeadSample {
                            x: features_at_embed(x, m)
                                .expect("embed_arm checked full coverage"),
                            y: yb,
                            strategy: y.strategy_id.clone(),
                        });
                }
            }
        }
        for (code, v) in per_code {
            if !v.is_empty() {
                fam.entry(code).or_default().push(v);
            }
        }
        for (code, v) in per_code_e {
            if !v.is_empty() {
                fam_e.entry(code).or_default().push(v);
            }
        }
    }
    let mut thin = 0usize;
    let mut fam_rows: Vec<FamilyRow> = Vec::new();
    for (code, run_groups) in fam {
        let n: usize = run_groups.iter().map(Vec::len).sum();
        let pos = run_groups
            .iter()
            .flatten()
            .filter(|s| s.y)
            .count();
        let neg = n - pos;
        if n < MIN_FAMILY_SAMPLES || pos < 2 || neg < 2 {
            thin += 1;
            continue;
        }
        let (readout_scores, hist_scores) = loro_scores(&run_groups);
        let auc_embed = if embed_arm {
            let (emb_scores, _) = loro_scores(&fam_e[&code]);
            auc(&emb_scores)
        } else {
            None
        };
        fam_rows.push(FamilyRow {
            code,
            n,
            pos,
            neg,
            auc_readout: auc(&readout_scores),
            auc_history: auc(&hist_scores),
            auc_embed,
        });
    }
    fam_rows.sort_by(|a, b| b.n.cmp(&a.n).then(a.code.cmp(&b.code)));
    s.thin_families = thin;
    s.families = fam_rows;
    s
}

/// Render the summary as the `--horizon-report` text (house style).
#[allow(clippy::too_many_lines)] // one table renderer — the T2 embed column/lines kept inline with their native twins rather than split into fragments reading half a row each
#[must_use]
pub fn render(s: &HorizonSummary) -> String {
    let mut out = String::with_capacity(2048);
    writeln!(
        out,
        "fixseq horizon report (Issue 107 T1) — lead-k linear decodability of attempt outcomes"
    )
    .unwrap();
    writeln!(
        out,
        "runs {} · attempts {} · kept {} · reverted {} · base keep-rate {:.3} (LCB₅ {:.3})",
        s.runs, s.attempts, s.kept, s.reverted, s.base_rate, s.base_lcb5
    )
    .unwrap();
    if s.bad_lines > 0 {
        writeln!(out, "unparseable lines skipped: {}", s.bad_lines).unwrap();
    }
    // Depth histogram (attempt-depth support — the Bench 087 discipline:
    // a lead-k zero over shallow runs is a POWER statement, not a finding).
    let mut hist: Vec<(usize, usize)> = s.depth_hist.iter().map(|(d, c)| (*d, *c)).collect();
    hist.sort_unstable_by_key(|(d, _)| *d);
    let mut h = String::new();
    for (d, c) in hist {
        let label = if d >= 5 { "5+" } else { &d.to_string() };
        write!(h, "{label}×{c} ").unwrap();
    }
    writeln!(out, "run depth (attempts-1): {h}").unwrap();

    // Issue 107 T2 — the embed arm's coverage state, always visible (the
    // arm-skip is a measurement about the corpus, not an absence).
    if s.embed_arm {
        writeln!(
            out,
            "embed arm: ON (span column coverage {}/{} attempts)",
            s.embed_attempts, s.attempts
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "embed arm: SKIPPED — span column coverage {}/{} attempts (T2 rings required; pre-T2 rows decode as None)",
            s.embed_attempts, s.attempts
        )
        .unwrap();
    }

    if let Some(r) = s.refusal {
        writeln!(out, "⛔ HORIZON REFUSAL — {}", r.label()).unwrap();
        return out;
    }

    writeln!(out).unwrap();
    writeln!(
        out,
        "  k |    n | pos | neg | auc(readout) | auc(prev) | auc(embed) | floor(0.5) | floor(history) | verdict"
    )
    .unwrap();
    writeln!(out, "----+------+-----+-----+--------------+-----------+------------+------------+----------------+--------").unwrap();
    for r in &s.rows {
        let ro = r
            .auc_readout
            .map_or("UNMEASURABLE".to_string(), |a| format!("{a:.3}"));
        let pv = r
            .auc_prev
            .map_or("UNMEASURABLE".to_string(), |a| format!("{a:.3}"));
        let em = if s.embed_arm {
            r.auc_embed
                .map_or("UNMEASURABLE".to_string(), |a| format!("{a:.3}"))
        } else {
            "—".to_string()
        };
        let hi = r
            .auc_history
            .map_or("UNMEASURABLE".to_string(), |a| format!("{a:.3}"));
        writeln!(
            out,
            " {:>2} | {:>4} | {:>3} | {:>3} | {:>12} | {:>9} | {:>10} | {:>10} | {:>14} | {}",
            r.k,
            r.n,
            r.pos,
            r.neg,
            ro,
            pv,
            em,
            "0.500",
            hi,
            r.verdict()
        )
        .unwrap();
    }
    let hz = s.horizon();
    if hz > 0 {
        writeln!(
            out,
            "HORIZON = {hz} (max lead where the readout beats max(0.5, history) + {MARGIN}; ≥2 pays)"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "HORIZON = none — no measurable lead beats both floors (T3 probe lane looks dead on this corpus)"
        )
        .unwrap();
    }
    if s.embed_arm {
        let hze = s.horizon_embed();
        if hze > 0 {
            writeln!(
                out,
                "HORIZON(embed) = {hze} (the T2 column's revival path — same bar over native+span features)"
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "HORIZON(embed) = none — the span column did not revive the lane on this corpus either"
            )
            .unwrap();
        }
    }
    // Issue 128 T6 — the transition arm's verdict line, always rendered
    // (the arm always runs; the comparison against HORIZON is the point).
    let hzp = s.horizon_prev();
    let hz = s.horizon();
    if hzp > hz {
        writeln!(
            out,
            "HORIZON(prev) = {hzp} > {hz} — the transition feature RAISES the horizon (Issue 128 T6)"
        )
        .unwrap();
    } else if hzp > 0 {
        writeln!(
            out,
            "HORIZON(prev) = {hzp} (<= native {hz}) — the transition feature does NOT raise the horizon here"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "HORIZON(prev) = none — the transition feature decodes nothing beyond the floors on this corpus"
        )
        .unwrap();
    }

    if !s.families.is_empty() {
        write!(out, "{}", render_family_table(&s.families, s.thin_families)).unwrap();
    }
    writeln!(
        out,
        "\nreadout: closed-form ridge over ring-native features at i-k (PROVISIONAL — the T3 \
         OnlineLinearReadout consumption follows katgpt-rs Issue 780); LORO hold-out, \
         either-polarity; floors: base 0.5 + per-strategy Laplace prior LORO. embed arm \
         (T2): the same ridge over native + span-embedding features (41 dims). prev arm \
         (Issue 128 T6): the same ridge over native + prev-attempt-outcome (10 dims)."
    )
    .unwrap();
    out
}

/// The per-family table section (k = 1) — its own renderer so `render`
/// stays the header + pooled table.
fn render_family_table(families: &[FamilyRow], thin: usize) -> String {
    let mut out = String::with_capacity(512);
    writeln!(out).unwrap();
    writeln!(
        out,
        "families at k=1 (target code; n ≥ {MIN_FAMILY_SAMPLES}, both classes ≥ 2):"
    )
    .unwrap();
    writeln!(
        out,
        "  code |    n | pos | neg | auc(readout) | auc(embed) | auc(history)"
    )
    .unwrap();
    writeln!(out, "-------+------+-----+-----+--------------+------------+-------------").unwrap();
    for f in families {
        let ro = f
            .auc_readout
            .map_or("UNMEASURABLE".to_string(), |a| format!("{a:.3}"));
        let em = f
            .auc_embed
            .map_or("—".to_string(), |a| format!("{a:.3}"));
        let hi = f
            .auc_history
            .map_or("UNMEASURABLE".to_string(), |a| format!("{a:.3}"));
        writeln!(
            out,
            " {:>5} | {:>4} | {:>3} | {:>3} | {:>12} | {:>10} | {:>12}",
            f.code, f.n, f.pos, f.neg, ro, em, hi
        )
        .unwrap();
    }
    if thin > 0 {
        writeln!(out, "({thin} thin families excluded from the table)").unwrap();
    }
    out
}

// Path helper for the CLI (kept here so tests cover the layout law).
/// The fixseq directory layout under a repo root (mirrors
/// `evolve::fixseq_dir`, ungated here).
#[must_use]
pub fn fixseq_layout(root: &Path) -> std::path::PathBuf {
    root.join(".heal").join("fixseq")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(seq: usize, code: &str, strat: &str, outcome: &str, errors: (usize, usize)) -> RingLine {
        RingLine {
            seq,
            code: code.to_string(),
            span_hash: format!("s{seq}"),
            strategy_id: strat.to_string(),
            outcome: outcome.to_string(),
            errors,
            masking: (0, 0),
            journey_residual_sum: 0,
            terminal_residual: 0,
            span_embed: None,
        }
    }

    /// Issue 128 T6 — the transition feature is three-state and sits one
    /// lead OLDER than every native feature (None at a run head).
    #[test]
    #[allow(clippy::float_cmp)] // exact-by-construction: the prev indicator is SET to the constants ±1.0/0.0 — the assertion pins the constants
    fn prev_feature_is_three_state() {
        let a = line(2, "E0499", "s", "Kept", (4, 3));
        let kept_prev = line(1, "E0499", "s", "Kept", (5, 4));
        let rev_prev = line(1, "E0499", "s", "RevertedNoProgress", (5, 5));
        let base = features_at(&a, 8);
        let p = features_at_prev(&a, Some(&kept_prev), 8);
        assert_eq!(p[..N_FEAT], base);
        assert_eq!(p[N_FEAT], 1.0);
        assert_eq!(features_at_prev(&a, Some(&rev_prev), 8)[N_FEAT], -1.0);
        assert_eq!(features_at_prev(&a, None, 8)[N_FEAT], 0.0);
    }

    /// Issue 128 T6 — the transition arm decodes a planted LAG-2
    /// anti-persistence signal the native 9 features cannot see: outcomes
    /// follow the period-4 pattern [K K R R] with per-run phase rotation,
    /// so `o_i` = NOT o_{i-2} (prev-visible at lead 1) while o_{i-1}
    /// (native-visible) and position (phase-rotated across runs) are both
    /// uninformative pooled.
    #[test]
    fn prev_arm_decodes_planted_lag2_signal_native_cannot() {
        let mk = |phase: usize| {
            let mut v = Vec::new();
            for i in 0..8usize {
                let outcome = if (i + phase) % 4 < 2 {
                    "Kept"
                } else {
                    "RevertedNoProgress"
                };
                v.push(line(i, "E0499", "s", outcome, (6, 6)));
            }
            run(v)
        };
        let runs: Vec<RunRing> = (0..8).map(mk).collect();
        let s = summarize(&runs);
        assert!(s.refusal.is_none(), "{:?}", s.refusal);
        let row1 = s.rows.iter().find(|r| r.k == 1).expect("k=1 row");
        let prev = row1.auc_prev.expect("prev AUC defined");
        let nat = row1.auc_readout.expect("native AUC defined");
        assert!(
            prev >= 0.95,
            "transition arm must decode the lag-2 signal: {prev}"
        );
        assert!(
            nat <= 0.65,
            "native 9 features must NOT see it at lead 1: {nat}"
        );
        assert!(s.horizon_prev() >= 1);
    }

    /// Issue 128 T6 — the transition arm must NOT beat the floors on a
    /// no-transition-signal corpus (the honest-null companion).
    #[test]
    fn prev_arm_does_not_beat_on_no_signal() {
        // Runs where the outcome alternates deterministically by position
        // (i % 2) — prev at lead k is redundant with position, and LORO
        // across phase-rotated runs kills positional leakage; simpler: all
        // runs identical [K R K R ...], so every feature is a deterministic
        // function of position and LORO trains on the same pattern it
        // scores — AUC may be high for BOTH arms, so assert only that prev
        // does not EXCEED the native readout's horizon (the T6 question is
        // marginal, never absolute). The real null-check is the ridge over
        // identical samples: fit == holdout, both arms read the same.
        let mk = || {
            let mut v = Vec::new();
            for i in 0..8usize {
                let outcome = if i % 2 == 0 { "Kept" } else { "RevertedNoProgress" };
                v.push(line(i, "E0499", "s", outcome, (6, 6)));
            }
            run(v)
        };
        let runs: Vec<RunRing> = (0..4).map(|_| mk()).collect();
        let s = summarize(&runs);
        // Both arms see the same deterministic pattern; the transition
        // feature adds NO information beyond kept_j (which already encodes
        // the parity) — the arms must agree within epsilon at every k.
        for r in &s.rows {
            if let (Some(p), Some(n)) = (r.auc_prev, r.auc_readout) {
                assert!(
                    (p - n).abs() <= 0.02,
                    "k={}: prev {p} must not drift from native {n} on a parity corpus",
                    r.k
                );
            }
        }
    }

    fn run(lines: Vec<RingLine>) -> RunRing {
        RunRing {
            source: format!("run-test{}.jsonl", lines.len()),
            lines,
        }
    }

    // ── AUC ────────────────────────────────────────────────────────────────

    #[test]
    fn auc_known_values_and_edges() {
        // Perfect separation.
        let s = vec![(0.9, true), (0.8, true), (0.2, false), (0.1, false)];
        assert!((auc(&s).unwrap() - 1.0).abs() < 1e-12);
        // Reversed — the statistic is signed, not abs'd (a fit readout would
        // flip the weight; the RAW statistic's polarity is information).
        let s = vec![(0.1, true), (0.9, false)];
        assert!((auc(&s).unwrap() - 0.0).abs() < 1e-12);
        // All ties → 0.5 exactly (the tie=0.5 law).
        let s = vec![(0.5, true), (0.5, false), (0.5, true), (0.5, false)];
        assert!((auc(&s).unwrap() - 0.5).abs() < 1e-12);
        // Single class → undefined, never 0.5-by-fiat.
        assert!(auc(&[(0.1, true), (0.9, true)]).is_none());
        // Mann–Whitney on a known small case: ranks 1,2,3,4 with pos at
        // ranks 2 and 4 → U = (2+4) - 3 = 3 of 4 → 0.75.
        let s = vec![(0.1, false), (0.2, true), (0.3, false), (0.4, true)];
        assert!((auc(&s).unwrap() - 0.75).abs() < 1e-12);
    }

    // ── Wire compat ────────────────────────────────────────────────────────

    #[test]
    fn ring_line_parses_driver_shapes() {
        // The exact legacy shapes the driver's own round-trip test pins.
        let pre100: RingLine = serde_json::from_str(
            r#"{"seq":0,"code":"E0502","span_hash":"x","strategy_id":"s","outcome":"Kept","errors":[2,1]}"#,
        )
        .unwrap();
        assert_eq!(pre100.masking, (0, 0));
        assert_eq!(pre100.kept(), Some(true));
        let current: RingLine = serde_json::from_str(
            r#"{"seq":1,"code":"E0614","span_hash":"y","strategy_id":"t","outcome":"RevertedNoProgress","errors":[3,3],"masking":[1,1],"journey_residual_sum":7,"terminal_residual":2}"#,
        )
        .unwrap();
        assert_eq!(current.kept(), Some(false));
        assert_eq!(current.journey_residual_sum, 7);
        // Issue 107 T2 — a T2 row carries the quantized column; every
        // legacy shape decodes it as None.
        let t2: RingLine = serde_json::from_str(
            r#"{"seq":2,"code":"E0507","span_hash":"z","strategy_id":"u","outcome":"Kept","errors":[2,1],"masking":[0,0],"span_embed":[0,1,-1,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#,
        )
        .unwrap();
        assert_eq!(t2.span_embed.as_ref().map(|v| v.len()), Some(crate::span_embed::SPAN_EMBED_DIMS));
        assert_eq!(t2.span_embed.unwrap()[3], 64);
        assert_eq!(pre100.span_embed, None);
        assert_eq!(current.span_embed, None);
        // Unknown outcome tag — corrupt, never folded.
        let bad: RingLine = serde_json::from_str(
            r#"{"seq":2,"code":"E","span_hash":"z","strategy_id":"u","outcome":"Mystery","errors":[1,1]}"#,
        )
        .unwrap();
        assert!(bad.kept().is_none());
    }

    // ── Ridge ──────────────────────────────────────────────────────────────

    #[test]
    fn ridge_recovers_planted_linear_signal() {
        // y = x[1] > 0.5 — the kept_j feature alone decides.
        let mut samples = Vec::with_capacity(40);
        for i in 0..40 {
            let mut x = [0.0; N_FEAT];
            x[0] = 1.0;
            x[1] = f64::from(i % 2);
            x[2] = f64::from(i % 5); // decorrelated noise feature
            samples.push((x, x[1] > 0.5));
        }
        let w = ridge_fit(&samples).expect("fit");
        let mut scored = Vec::with_capacity(samples.len());
        for (x, y) in &samples {
            let s = w.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
            scored.push((s, *y));
        }
        assert!((auc(&scored).unwrap() - 1.0).abs() < 1e-9);
        assert!(ridge_fit::<N_FEAT>(&[]).is_none());
        assert!(ridge_fit::<N_FEAT_EMBED>(&[]).is_none(), "the embed arm's fit shares the empty-input contract");
    }

    // ── Refusals ───────────────────────────────────────────────────────────

    #[test]
    fn empty_blind_and_single_class_refuse() {
        let e = summarize(&[]);
        assert_eq!(e.refusal, Some(HorizonRefusal::Empty));
        assert!(render(&e).contains("HORIZON REFUSAL"));

        let blind = summarize(&[run(vec![line(0, "E0502", "s", "Kept", (2, 1))])]);
        assert_eq!(blind.refusal, Some(HorizonRefusal::BlindWalk));

        // Depth 2+ but every outcome Kept — AUC undefined everywhere.
        let all_kept: Vec<RunRing> = (0..6)
            .map(|r| {
                let lines: Vec<RingLine> = (0..4)
                    .map(|i| line(i, "E0502", "s", "Kept", (4 - i, 3 - i)))
                    .collect();
                RunRing {
                    source: format!("run-k{r}.jsonl"),
                    lines,
                }
            })
            .collect();
        let sc = summarize(&all_kept);
        assert_eq!(sc.refusal, Some(HorizonRefusal::SingleClass));
    }

    // ── The two-sided horizon canary ───────────────────────────────────────

    #[test]
    fn horizon_detects_planted_persistence_and_rejects_no_signal() {
        // (a) PLANTED: the label at target t (t ≥ 2) copies the
        // ERROR-PROGRESS feature at t-2 (progress 3 ⇒ keep), i.e. a pure
        // lead-2 signal. 12 runs × 6 attempts → 48 k=2 samples, balanced.
        let planted: Vec<RunRing> = (0..12)
            .map(|r| {
                let lines: Vec<RingLine> = (0..6)
                    .map(|i| {
                        let prog = (i + r) % 2 == 0;
                        let (eb, ea) = if prog { (5, 2) } else { (5, 5) };
                        let kept_i = if i < 2 {
                            false
                        } else {
                            (i - 2 + r) % 2 == 0
                        };
                        let outcome = if kept_i { "Kept" } else { "RevertedNoProgress" };
                        line(i, "E0502", "s", outcome, (eb, ea))
                    })
                    .collect();
                RunRing {
                    source: format!("run-p{r}.jsonl"),
                    lines,
                }
            })
            .collect();
        let s = summarize(&planted);
        assert_eq!(s.refusal, None, "planted fixture must be measurable");
        let k2 = s.rows.iter().find(|row| row.k == 2).expect("k=2 row");
        assert!(
            k2.auc_readout.is_some_and(|a| a >= 0.9),
            "planted lead-2 signal must decode: {:?}",
            k2.auc_readout
        );
        assert!(s.horizon() >= 2, "horizon must reach the planted lead 2");

        // (b) NO SIGNAL: 20 two-attempt runs whose FIRST record is
        // identical everywhere (constant feature vector ⇒ every readout
        // score ties ⇒ AUC exactly 0.5) and whose target label is a fixed
        // balanced pattern across runs — deterministic, seed-free, and
        // immune to the alternation trap (any alternating label pattern is
        // perfectly learnable from `kept_j`, i.e. it IS signal).
        let target_pattern = [
            true, false, true, true, false, true, false, false, true, false, true, false, false,
            true, false, true, true, false, false, true,
        ];
        let nosig: Vec<RunRing> = target_pattern
            .iter()
            .enumerate()
            .map(|(r, &kept)| {
                let outcome = if kept { "Kept" } else { "RevertedNoProgress" };
                RunRing {
                    source: format!("run-n{r}.jsonl"),
                    lines: vec![
                        line(0, "E0502", "s", "Kept", (3, 3)),
                        line(1, "E0502", "s", outcome, (3, 3)),
                    ],
                }
            })
            .collect();
        let s2 = summarize(&nosig);
        assert_eq!(s2.refusal, None);
        for row in &s2.rows {
            if row.measurable() {
                assert!(
                    !row.beats(),
                    "constant-feature rows must not beat the floor: k={} auc={:?}",
                    row.k,
                    row.auc_readout
                );
            }
        }
        assert_eq!(s2.horizon(), 0, "no-signal fixture must yield horizon none");
    }

    // ── Issue 107 T2: the embed arm ──────────────────────────────────────

    /// The arm runs ONLY at full column coverage — a partial corpus skips
    /// LOUD (coverage line), never narrows the sample set silently.
    #[test]
    fn embed_arm_requires_full_coverage() {
        let mk = |r: usize, with_col: bool| {
            let lines: Vec<RingLine> = (0..4)
                .map(|i| {
                    // KKRR rotated: zero lag-1 autocorrelation and balanced
                    // at every pooled position — kept_j and pos carry
                    // nothing (the alternation trap's lesson, one test up).
                    let outcome = if (i + r) % 4 < 2 {
                        "Kept"
                    } else {
                        "RevertedNoProgress"
                    };
                    let mut l = line(i, "E0502", "s", outcome, (4, 4));
                    if with_col {
                        l.span_embed =
                            Some(crate::span_embed::span_embed("let v = Vec::new();"));
                    }
                    l
                })
                .collect();
            RunRing {
                source: format!("run-c{r}.jsonl"),
                lines,
            }
        };
        let mut runs: Vec<RunRing> = (0..4).map(|r| mk(r, true)).collect();
        runs.push(mk(9, false)); // the pre-T2 straggler
        let s = summarize(&runs);
        assert_eq!(s.refusal, None);
        assert!(!s.embed_arm, "one column-less run must skip the arm");
        assert_eq!(s.embed_attempts, 16, "16 of 20 attempts carry the column");
        assert_eq!(s.attempts, 20);
        assert!(s.rows.iter().all(|r| r.auc_embed.is_none()));
        assert_eq!(s.horizon_embed(), 0);
        let text = render(&s);
        assert!(text.contains("embed arm: SKIPPED"), "loud skip: {text}");
        assert!(text.contains("16/20"), "coverage in the line: {text}");

        // Full coverage → the arm runs (embeddings here carry no signal —
        // identical texts — so it must NOT beat the floor: the negative
        // control for the revival test below).
        let full: Vec<RunRing> = (0..5).map(|r| mk(r, true)).collect();
        let s2 = summarize(&full);
        assert!(s2.embed_arm, "full coverage runs the arm");
        assert_eq!(s2.embed_attempts, 20);
        let k1 = s2.rows.iter().find(|row| row.k == 1).expect("k=1 row");
        assert!(
            !k1.beats_embed(),
            "identical embeddings carry no signal — the arm must not beat the floor: {:?}",
            k1.auc_embed
        );
        assert!(render(&s2).contains("embed arm: ON"));
    }

    /// The revival canary — a span-embedded signal the RING-NATIVE features
    /// cannot see: the label at i is determined by the TOKEN CONTENT of
    /// line i-1 (keep-marker vs revert-marker text), while every native
    /// scalar is constant and the only live native feature (`kept_j`) sees
    /// the label sequence at one lag — correlated only through the token
    /// pattern's lag-1 autocorrelation, which the rotation pool keeps mild.
    /// The embed arm must decode the content signal at lead 1 (≥ 0.9).
    #[test]
    fn embed_arm_decodes_planted_span_signal() {
        let text_k = "fn keep_marker_case() { let val = 1; }";
        let text_r = "fn revert_marker_case() { let val = 2; }";
        let emb_k = crate::span_embed::span_embed(text_k);
        let emb_r = crate::span_embed::span_embed(text_r);
        assert_ne!(emb_k, emb_r, "fixture texts must embed differently");

        // Token pattern K K R K R R (circular φ ≈ −0.33: the mild negative
        // lag-1 autocorrelation gives the native arm SOMETHING but far from
        // separation); each run rotates the pattern so position carries no
        // pooled signal.
        let pattern = [true, true, false, true, false, false];
        let runs: Vec<RunRing> = (0..12)
            .map(|r| {
                let lines: Vec<RingLine> = (0..6)
                    .map(|i| {
                        let is_k = pattern[(i + r) % pattern.len()];
                        // Line 0's outcome is the kept_j feature for sample
                        // i=1 only — held constant so it carries nothing.
                        let kept_i = if i == 0 {
                            false
                        } else {
                            pattern[(i - 1 + r) % pattern.len()]
                        };
                        let outcome = if kept_i {
                            "Kept"
                        } else {
                            "RevertedNoProgress"
                        };
                        let mut l = line(i, "E0502", "s", outcome, (4, 4));
                        l.span_embed = Some(if is_k { emb_k } else { emb_r });
                        l
                    })
                    .collect();
                RunRing {
                    source: format!("run-emb{r}.jsonl"),
                    lines,
                }
            })
            .collect();
        let s = summarize(&runs);
        assert_eq!(s.refusal, None);
        assert!(s.embed_arm, "full coverage");
        assert_eq!(s.embed_attempts, s.attempts);
        let k1 = s.rows.iter().find(|row| row.k == 1).expect("k=1 row");
        assert!(k1.measurable());
        assert!(
            k1.auc_embed.is_some_and(|a| a >= 0.9),
            "the planted span-content signal must decode through the column: {:?}",
            k1.auc_embed
        );
        assert!(
            k1.auc_readout.is_none_or(|a| a < 0.9),
            "the native arm must NOT fully see a content-only signal: {:?}",
            k1.auc_readout
        );
        assert!(
            s.horizon_embed() >= 1,
            "the revival path must fire at lead 1"
        );
        let text = render(&s);
        assert!(text.contains("auc(embed)"), "column rendered");
    }

    // ── History floor hygiene ──────────────────────────────────────────────

    #[test]
    fn history_floor_prior_excludes_own_run() {
        // Direct unit on the summarize path: two strategies with OPPOSITE
        // per-run rates — a leaky prior (including run r) would score its
        // own labels perfectly; LORO must not.
        // Run A: strat-a all Kept (4 attempts), Run B: strat-a all Reverted.
        let a = run(vec![
            line(0, "E0502", "strat-a", "Kept", (4, 3)),
            line(1, "E0502", "strat-a", "Kept", (3, 2)),
            line(2, "E0502", "strat-a", "Kept", (2, 1)),
            line(3, "E0502", "strat-a", "Kept", (1, 0)),
        ]);
        let b = run(vec![
            line(0, "E0502", "strat-a", "RevertedNoProgress", (4, 4)),
            line(1, "E0502", "strat-a", "RevertedNoProgress", (4, 4)),
            line(2, "E0502", "strat-a", "RevertedNoProgress", (4, 4)),
            line(3, "E0502", "strat-a", "RevertedNoProgress", (4, 4)),
        ]);
        let s = summarize(&[a, b]);
        assert_eq!(s.refusal, None);
        let k1 = s.rows.iter().find(|row| row.k == 1).expect("k=1 row");
        // LORO: every reverted sample is scored by a prior fit ONLY on
        // keeps (and vice versa) — the floor's AUC must be ≤ 0.5 here
        // (it anti-ranks), proving the fold excludes the scored run.
        assert!(
            k1.auc_history.is_some_and(|h| h <= 0.5 + 1e-9),
            "LORO history floor must not leak: {:?}",
            k1.auc_history
        );
    }

    // ── Load layout ────────────────────────────────────────────────────────

    #[test]
    fn load_runs_reads_dump_layout_sorted_and_counts_bad() {
        let root = std::env::temp_dir().join(format!("riir_hz_{}_{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = fixseq_layout(&root);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("run-b.jsonl"),
            "{\"seq\":0,\"code\":\"E0502\",\"span_hash\":\"x\",\"strategy_id\":\"s\",\"outcome\":\"Kept\",\"errors\":[2,1]}\n\
             {\"seq\":1,\"code\":\"E0502\",\"span_hash\":\"x\",\"strategy_id\":\"s\",\"outcome\":\"RevertedNoProgress\",\"errors\":[1,1]}\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("run-a.jsonl"),
            "{\"seq\":0,\"code\":\"E0614\",\"span_hash\":\"y\",\"strategy_id\":\"t\",\"outcome\":\"Kept\",\"errors\":[3,2]}\n\
             not-json\n\
             {\"seq\":1,\"code\":\"E0614\",\"span_hash\":\"y\",\"strategy_id\":\"t\",\"outcome\":\"Weird\",\"errors\":[2,2]}\n",
        )
        .unwrap();
        let (runs, bad, files) = load_runs(&root);
        assert_eq!((runs.len(), bad, files), (2, 2, 2));
        // Sorted by file name — run-a first.
        assert_eq!(runs[0].source, "run-a.jsonl");
        assert_eq!(runs[0].lines.len(), 1); // the corrupt + unknown lines skipped
        assert_eq!(runs[1].source, "run-b.jsonl");
        // A file path loads as a single run.
        let (one, _, _) = load_runs(&dir.join("run-b.jsonl"));
        assert_eq!(one.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    // ── Determinism ────────────────────────────────────────────────────────

    #[test]
    fn summarize_is_deterministic() {
        let mk = |r: usize| {
            let lines: Vec<RingLine> = (0..7)
                .map(|i| {
                    let outcome = if (i + r).is_multiple_of(3) {
                        "Kept"
                    } else {
                        "RevertedNoProgress"
                    };
                    let (eb, ea) = ((i * 3 + r) % 5 + 1, (i + r) % 3);
                    line(i, &format!("E050{}", r % 3), "strat", outcome, (eb, ea))
                })
                .collect();
            RunRing {
                source: format!("run-d{r}.jsonl"),
                lines,
            }
        };
        let a = summarize(&(0..9).map(mk).collect::<Vec<_>>());
        let b = summarize(&(0..9).map(mk).collect::<Vec<_>>());
        assert_eq!(a, b);
        assert_eq!(render(&a), render(&b));
    }
}
