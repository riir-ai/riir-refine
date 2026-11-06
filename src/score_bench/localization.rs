//! Issue 137 T2/T4/T6 — the known-position localization metrics, the DLM
//! remask arming gate, and the panel rows, over the generation-v9
//! corruption corpus (Research 215 / arXiv:2512.15596's CRB instrument).
//!
//! ## Polarity contract (paper-faithful)
//!
//! A [`SpanScorer`] returns **confidence** per leaf token: HIGH = likely
//! correct. Errors localize in the LOW-confidence tail. This matches the
//! consumer this gate exists for — the d2f remask confidence / Plan 414's
//! trained gap are confidences natively — and keeps the paper's gap
//! definition verbatim: `gap = mean(clean-token confidence) − mean(error-
//! token confidence)`, positive for an informative scorer, ~0 for an
//! uninformative one, negative for an anti-scorer.
//!
//! ## The loud-zero laws (T2, asserted live by the panel + by tests)
//!
//! - a constant-confidence scorer reads gap = 0.0 and the gate REFUSES
//!   ([`Refusal::ConstantConfidence`]) — never "perfectly calibrated";
//! - a shuffled-confidence control falls to chance hit@K
//!   ([`chance_hit_at_k`]) — the metric must discriminate;
//! - the floors break separately: the fixture-walk floor (a fixture whose
//!   error spans overlap no leaf token is an Err, never a skip) and the
//!   full-ranking emission floor (a scorer covering N−1 of N tokens is an
//!   Err, never a truncated pass).
//!
//! ## Arming gate (T4)
//!
//! [`arming_verdict`] is the pre-registered consumer contract: a confidence
//! source arms the remask loop only above [`DEFAULT_ARMING_FLOORS`], only
//! with a real signal over its own shuffled control, and never when
//! constant or regressive. The negative control (permuted confidence) is
//! refused by construction — pinned by test and re-proven live every panel
//! run.
//!
//! ## Panel rows (T6)
//!
//! Per-class rows ([`per_class_rows`]) are NEVER pooled — a mixed fixture
//! appears in every class bucket it carries, and the floor binding is
//! min-class ([`class_floor_verdict`]), so "pooled passes" cannot stand in
//! for "every class passes" (the pooled-equals-min arm fails by test).
//! Bernoulli re-draws disclose their CI width ([`bootstrap_hit_ci`],
//! seeded). [`loop_harm_verdict`] / [`paired_non_inferior`] are the paired
//! non-inferiority instruments for the paper's measured failure mode
//! (the loop fixes planted errors while breaking previously-correct
//! positions) — consumed by the DLM lane when it exists.

use std::collections::BTreeSet;
use std::ops::Range;

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;

use super::corruption::{self, CorruptionClass, CorruptionSpec, SplitMix64};

/// The hit@K ladder (flagged-token budget; the gen2 convention's K axis).
pub const EVAL_KS: [usize; 3] = [1, 3, 5];

/// The flagged budget the FPR row reports (the widest K on the ladder).
const FPR_K: usize = 5;

/// `usize` count → `f64`. Every caller passes a token/fixture/flag count
/// far below f64's 2⁵² exact-integer mantissa bound — the invariant lives
/// here, once, instead of an allow at each metric line.
#[must_use]
fn count_to_f64(count: usize) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    {
        count as f64
    }
}

// ── tokenization ────────────────────────────────────────────────────────

/// Every leaf token's byte range (groups flattened; empty ranges skipped).
///
/// # Errors
///
/// When the fixture text fails to tokenize (cannot happen for a
/// compile-clean corpus fixture — the floor is the point).
pub fn leaf_tokens(code: &str) -> Result<Vec<Range<usize>>, String> {
    let stream: TokenStream = code.parse().map_err(|e| format!("tokenize: {e}"))?;
    let mut out = Vec::new();
    walk_tokens(stream, &mut out);
    Ok(out)
}

fn walk_tokens(stream: TokenStream, out: &mut Vec<Range<usize>>) {
    for tree in stream {
        match tree {
            TokenTree::Group(group) => walk_tokens(group.stream(), out),
            other => {
                let range = other.span().byte_range();
                if range.start < range.end {
                    out.push(range);
                }
            }
        }
    }
}

// ── the scorer contract ─────────────────────────────────────────────────

/// One token's confidence (HIGH = likely correct — see the module doc).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanScore {
    pub start: usize,
    pub end: usize,
    pub score: f32,
}

/// What a scorer may read: the fixture's code, its leaf-token ranges, its
/// known error spans, and a stable per-program key (so stochastic scorers —
/// the shuffled control — vary per program yet stay deterministic).
pub struct ScoringContext<'a> {
    pub code: &'a str,
    pub tokens: &'a [Range<usize>],
    pub error_spans: &'a [(usize, usize)],
    pub program_key: u64,
}

/// The consumer seam (T4): the d2f remask confidence / Plan 414's trained
/// gap implement this trait; the arming gate consumes ANY implementation.
pub trait SpanScorer {
    fn name(&self) -> &'static str;
    /// One confidence per token, aligned with `ctx.tokens` (the emission
    /// floor rejects misaligned or truncated output).
    fn score(&self, ctx: &ScoringContext) -> Vec<SpanScore>;
}

fn overlaps(range: &Range<usize>, spans: &[(usize, usize)]) -> bool {
    spans
        .iter()
        .any(|(s, e)| range.start < *e && *s < range.end)
}

/// Perfect confidence: 1.0 on clean tokens, 0.0 on error tokens. The
/// control-sanity scorer — if THIS does not arm, the floors are wrong.
pub struct OracleScorer;

impl SpanScorer for OracleScorer {
    fn name(&self) -> &'static str {
        "oracle"
    }

    fn score(&self, ctx: &ScoringContext) -> Vec<SpanScore> {
        ctx.tokens
            .iter()
            .map(|range| SpanScore {
                start: range.start,
                end: range.end,
                score: if overlaps(range, ctx.error_spans) {
                    0.0
                } else {
                    1.0
                },
            })
            .collect()
    }
}

/// The loud-zero control: every token reads the same confidence — the gate
/// must refuse, never read "perfectly calibrated".
pub struct ConstantScorer(pub f32);

impl SpanScorer for ConstantScorer {
    fn name(&self) -> &'static str {
        "constant"
    }

    fn score(&self, ctx: &ScoringContext) -> Vec<SpanScore> {
        ctx.tokens
            .iter()
            .map(|range| SpanScore {
                start: range.start,
                end: range.end,
                score: self.0,
            })
            .collect()
    }
}

/// Permuted-confidence negative control: the inner scorer's VALUES are
/// Fisher-Yates'd across positions (seeded per program via
/// `seed ^ program_key`), so every per-token value survives but their
/// alignment is destroyed. Must fall to chance hit@K — the proof the
/// metric discriminates.
pub struct ShuffledScorer {
    label: &'static str,
    inner: Box<dyn SpanScorer>,
    seed: u64,
}

impl ShuffledScorer {
    #[must_use]
    pub fn new(inner: Box<dyn SpanScorer>, seed: u64) -> Self {
        Self {
            label: Box::leak(format!("shuffled({})", inner.name()).into_boxed_str()),
            inner,
            seed,
        }
    }
}

impl SpanScorer for ShuffledScorer {
    fn name(&self) -> &'static str {
        self.label
    }

    fn score(&self, ctx: &ScoringContext) -> Vec<SpanScore> {
        let mut scores = self.inner.score(ctx);
        // permute the VALUES only — the span fields stay in position so the
        // emission floor keeps reading aligned (the score↔position binding
        // is exactly the identity a shuffle must destroy).
        let mut values: Vec<f32> = scores.iter().map(|score| score.score).collect();
        let mut rng = SplitMix64::new(self.seed ^ ctx.program_key);
        for i in (1..values.len()).rev() {
            let j = rng.below(i + 1);
            values.swap(i, j);
        }
        for (score, value) in scores.iter_mut().zip(values) {
            score.score = value;
        }
        scores
    }
}

/// The anti-scorer: negates the inner confidence (confident ON errors).
/// The gate must read a regressive gap and refuse.
pub struct InverseScorer {
    label: &'static str,
    inner: Box<dyn SpanScorer>,
}

impl InverseScorer {
    #[must_use]
    pub fn new(inner: Box<dyn SpanScorer>) -> Self {
        Self {
            label: Box::leak(format!("inverse({})", inner.name()).into_boxed_str()),
            inner,
        }
    }
}

impl SpanScorer for InverseScorer {
    fn name(&self) -> &'static str {
        self.label
    }

    fn score(&self, ctx: &ScoringContext) -> Vec<SpanScore> {
        self.inner
            .score(ctx)
            .into_iter()
            .map(|mut s| {
                s.score = -s.score;
                s
            })
            .collect()
    }
}

// ── scoring + evaluation ────────────────────────────────────────────────

/// One scored fixture (the fixture-walk floor fires here: a fixture whose
/// error spans overlap no leaf token, or a scorer that truncates or
/// misaligns, is an Err — never a skipped row).
#[derive(Clone, Debug)]
pub struct ScoredProgram<'a> {
    pub fixture: &'a CorruptionSpec,
    pub n_tokens: usize,
    pub n_error_tokens: usize,
    pub scores: Vec<SpanScore>,
}

/// Score one fixture. `program_key` is blake3(fixture id) — stable across
/// runs, distinct per fixture.
///
/// # Errors
///
/// Tokenization failure, error spans overlapping no leaf token, or a
/// scorer emission that is truncated / misaligned (the full-ranking floor).
///
/// # Panics
///
/// Only through `try_into` on a slice whose length is asserted to be at
/// least 8 bytes (blake3 always yields 32) — unreachable by construction.
pub fn score_program<'a>(
    scorer: &dyn SpanScorer,
    fixture: &'a CorruptionSpec,
) -> Result<ScoredProgram<'a>, String> {
    let tokens = leaf_tokens(fixture.code)?;
    let key_bytes = blake3::hash(fixture.id.as_bytes());
    let program_key = u64::from_le_bytes(
        key_bytes.as_bytes()[..8]
            .try_into()
            .expect("blake3 yields ≥ 8 bytes"),
    );
    let ctx = ScoringContext {
        code: fixture.code,
        tokens: &tokens,
        error_spans: fixture.spans,
        program_key,
    };
    let token_scores = scorer.score(&ctx);
    if token_scores.len() != tokens.len() {
        return Err(format!(
            "{}: scorer {} covered {} of {} tokens (full-ranking emission floor)",
            fixture.id,
            scorer.name(),
            token_scores.len(),
            tokens.len()
        ));
    }
    for (score, token) in token_scores.iter().zip(tokens.iter()) {
        if score.start != token.start || score.end != token.end {
            return Err(format!(
                "{}: scorer {} emitted misaligned spans",
                fixture.id,
                scorer.name()
            ));
        }
    }
    let n_error_tokens = tokens
        .iter()
        .filter(|token| overlaps(token, fixture.spans))
        .count();
    if n_error_tokens == 0 {
        return Err(format!(
            "{}: error spans overlap no leaf token (fixture-walk floor)",
            fixture.id
        ));
    }
    Ok(ScoredProgram {
        fixture,
        n_tokens: tokens.len(),
        n_error_tokens,
        scores: token_scores,
    })
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0f64, 0usize), |(s, n), v| (s + v, n + 1));
    if count == 0 {
        0.0
    } else {
        sum / count_to_f64(count)
    }
}

/// One fixture's localization read.
#[derive(Debug, Clone)]
pub struct ProgramRead {
    pub id: &'static str,
    pub k: usize,
    pub n_tokens: usize,
    pub n_error_tokens: usize,
    pub clean_mean: f64,
    pub error_mean: f64,
    /// hit@K per [`EVAL_KS`] slot.
    pub hit: [bool; 3],
    /// clean tokens among the flagged K (slot 2 = K=5) / K.
    pub fpr5: f64,
    /// 1 / rank of the first error token (0 if never flagged).
    pub rr: f64,
}

/// The aggregate report for one scorer over the fixture set.
#[derive(Debug, Clone)]
pub struct EvalReport {
    pub scorer: String,
    pub reads: Vec<ProgramRead>,
    /// mean(clean confidence) − mean(error confidence); the paper's gap.
    pub gap: f64,
    /// mean hit@K per [`EVAL_KS`] slot.
    pub hit: [f64; 3],
    pub fpr5: f64,
    pub mrr: f64,
    /// true when every score in the set is identical (the loud-zero state).
    pub constant: bool,
}

/// One fixture's localization read, from its scored tokens: error flags,
/// confidence means, ascending-confidence ranking, hit@K / FPR@5 / MRR.
fn read_of(program: &ScoredProgram) -> ProgramRead {
    let is_error: Vec<bool> = program
        .scores
        .iter()
        .map(|score| overlaps(&(score.start..score.end), program.fixture.spans))
        .collect();
    let error_mean = mean(
        program
            .scores
            .iter()
            .zip(is_error.iter())
            .filter(|(_, err)| **err)
            .map(|(score, _)| f64::from(score.score)),
    );
    let clean_mean = mean(
        program
            .scores
            .iter()
            .zip(is_error.iter())
            .filter(|(_, err)| !**err)
            .map(|(score, _)| f64::from(score.score)),
    );
    // ascending confidence = the flagged tail; ties by position
    let mut order: Vec<usize> = (0..program.scores.len()).collect();
    order.sort_by(|&a, &b| {
        program.scores[a]
            .score
            .partial_cmp(&program.scores[b].score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| program.scores[a].start.cmp(&program.scores[b].start))
    });
    let mut hit = [false; 3];
    let mut fpr_slots = 0usize;
    let mut first_error_rank = 0usize;
    for (rank_zero, &index) in order.iter().enumerate() {
        let rank = rank_zero + 1;
        if is_error[index] {
            if first_error_rank == 0 {
                first_error_rank = rank;
            }
            for (slot, &k) in EVAL_KS.iter().enumerate() {
                if rank <= k {
                    hit[slot] = true;
                }
            }
        } else if rank <= FPR_K {
            fpr_slots += 1;
        }
    }
    let fpr5 = count_to_f64(fpr_slots) / count_to_f64(FPR_K);
    let rr = if first_error_rank > 0 {
        1.0 / count_to_f64(first_error_rank)
    } else {
        0.0
    };
    ProgramRead {
        id: program.fixture.id,
        k: program.fixture.k,
        n_tokens: program.n_tokens,
        n_error_tokens: program.n_error_tokens,
        clean_mean,
        error_mean,
        hit,
        fpr5,
        rr,
    }
}

/// Evaluate a scored fixture set. Ranking is ASCENDING by confidence (the
/// low tail is flagged), ties broken by byte position for determinism.
#[must_use]
pub fn evaluate(scorer: &dyn SpanScorer, programs: &[ScoredProgram]) -> EvalReport {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut any = false;
    for program in programs {
        for score in &program.scores {
            any = true;
            min = min.min(score.score);
            max = max.max(score.score);
        }
    }
    // EXACT comparison is the loud-zero law's premise: a constant scorer
    // must read as constant, not "constant within epsilon".
    #[allow(clippy::float_cmp)]
    let constant = any && min == max;

    let mut reads = Vec::with_capacity(programs.len());
    let mut gap_sum = 0.0f64;
    let mut gap_n = 0usize;
    let mut hit_sum = [0.0f64; 3];
    let mut fpr_sum = 0.0f64;
    let mut rr_sum = 0.0f64;
    let mut counted = 0usize;
    for program in programs {
        let read = read_of(program);
        gap_sum += read.clean_mean - read.error_mean;
        gap_n += 1;
        for (slot, flag) in read.hit.iter().enumerate() {
            if *flag {
                hit_sum[slot] += 1.0;
            }
        }
        fpr_sum += read.fpr5;
        rr_sum += read.rr;
        counted += 1;
        reads.push(read);
    }
    let n = count_to_f64(counted.max(1));
    EvalReport {
        scorer: scorer.name().to_string(),
        reads,
        gap: if gap_n > 0 {
            gap_sum / count_to_f64(gap_n)
        } else {
            0.0
        },
        hit: [hit_sum[0] / n, hit_sum[1] / n, hit_sum[2] / n],
        fpr5: fpr_sum / n,
        mrr: rr_sum / n,
        constant,
    }
}

/// P(at least one error token among K uniformly-random flagged tokens) —
/// `1 − C(N−e, K) / C(N, K)` expanded as a running product. The chance
/// band the shuffled control must fall into.
#[must_use]
pub fn chance_hit_at_k(n_tokens: usize, n_error: usize, k: usize) -> f64 {
    if n_tokens == 0 || n_error == 0 {
        return 0.0;
    }
    if k >= n_tokens {
        return 1.0;
    }
    let mut p_miss = 1.0f64;
    for i in 0..k {
        let miss_numerator = count_to_f64(n_tokens - n_error) - count_to_f64(i);
        if miss_numerator <= 0.0 {
            return 1.0;
        }
        p_miss *= miss_numerator / (count_to_f64(n_tokens) - count_to_f64(i));
    }
    1.0 - p_miss
}

// ── the arming gate (T4) ────────────────────────────────────────────────

/// Pre-registered arming floors. A confidence source arms the remask loop
/// only when every check passes; anything below refuses with a named
/// reason. The FPR cap sits ABOVE the per-fixture floor any scorer — the
/// oracle included — cannot go below: flagging K tokens against |E| true
/// errors yields ≥ (K−|E|)/K clean flags per fixture, so at |E|=1 the
/// perfect FPR@5 reads 0.8. The cap (0.85) therefore catches pure noise
/// (FPR → 1.0), not the structural floor.
#[derive(Debug, Clone, Copy)]
pub struct ArmingFloors {
    pub min_gap: f64,
    pub min_hit1: f64,
    pub max_fpr5: f64,
    /// real hit@1 must EXCEED the shuffled control's by at least this.
    pub shuffle_margin: f64,
}

/// The landing floors (pre-registered in Issue 137 before the first
/// production scorer exists — they bind the seam, not a measurement).
pub const DEFAULT_ARMING_FLOORS: ArmingFloors = ArmingFloors {
    min_gap: 0.05,
    min_hit1: 0.30,
    max_fpr5: 0.85,
    shuffle_margin: 0.10,
};

/// The arming verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arming {
    Armed,
    Refused(Refusal),
}

/// Named refusal reasons (the panel prints them; the DLM lane reads them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    ConstantConfidence,
    RegressiveGap,
    NoSignalOverShuffle,
    GapBelowFloor,
    Hit1BelowFloor,
    Fpr5AboveCap,
}

impl Refusal {
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::ConstantConfidence => "constant_confidence",
            Self::RegressiveGap => "regressive_gap",
            Self::NoSignalOverShuffle => "no_signal_over_shuffle",
            Self::GapBelowFloor => "gap_below_floor",
            Self::Hit1BelowFloor => "hit1_below_floor",
            Self::Fpr5AboveCap => "fpr5_above_cap",
        }
    }
}

/// The gate ladder, checked in a fixed order (first failure names the
/// refusal): constant → regressive → no signal over the shuffled control →
/// gap floor → hit@1 floor → FPR cap.
#[must_use]
pub fn arming_verdict(real: &EvalReport, shuffled: &EvalReport, floors: &ArmingFloors) -> Arming {
    if real.constant {
        return Arming::Refused(Refusal::ConstantConfidence);
    }
    if real.gap < 0.0 {
        return Arming::Refused(Refusal::RegressiveGap);
    }
    if real.hit[0] <= shuffled.hit[0] + floors.shuffle_margin {
        return Arming::Refused(Refusal::NoSignalOverShuffle);
    }
    if real.gap < floors.min_gap {
        return Arming::Refused(Refusal::GapBelowFloor);
    }
    if real.hit[0] < floors.min_hit1 {
        return Arming::Refused(Refusal::Hit1BelowFloor);
    }
    if real.fpr5 > floors.max_fpr5 {
        return Arming::Refused(Refusal::Fpr5AboveCap);
    }
    Arming::Armed
}

// ── panel rows (T6) ─────────────────────────────────────────────────────

/// One per-class row. NEVER pooled into the others: a mixed fixture
/// appears in every bucket it carries (disclosed by the n column).
#[derive(Debug, Clone)]
pub struct ClassRow {
    pub class: CorruptionClass,
    pub n: usize,
    pub gap: f64,
    pub hit1: f64,
    pub hit5: f64,
    pub fpr5: f64,
    pub mrr: f64,
}

/// Per-class rows over the scored set (the class axis of the paper's
/// deepest claim: supervision confined to one position class breeds an
/// uninformative signal on the other).
#[must_use]
pub fn per_class_rows(scorer: &dyn SpanScorer, programs: &[ScoredProgram]) -> Vec<ClassRow> {
    let cap_iter = [
        CorruptionClass::Operator,
        CorruptionClass::Literal,
        CorruptionClass::Ident,
    ];
    let mut rows = Vec::with_capacity(cap_iter.len());
    for class in cap_iter {
        let subset: Vec<ScoredProgram> = programs
            .iter()
            .filter(|program| {
                program
                    .fixture
                    .classes
                    .iter()
                    .any(|tag| CorruptionClass::from_tag(tag) == Some(class))
            })
            .cloned()
            .collect();
        if subset.is_empty() {
            continue;
        }
        let report = evaluate(scorer, &subset);
        rows.push(ClassRow {
            class,
            n: subset.len(),
            gap: report.gap,
            hit1: report.hit[0],
            hit5: report.hit[2],
            fpr5: report.fpr5,
            mrr: report.mrr,
        });
    }
    rows
}

/// The class-axis arming binding: EVERY class row must clear the hit@1 and
/// gap floors — the min-class rule. A pooled mean that clears while one
/// class fails is exactly the arm this function refuses.
#[must_use]
pub fn class_floor_verdict(rows: &[ClassRow], floors: &ArmingFloors) -> Arming {
    for row in rows {
        if row.gap < floors.min_gap {
            return Arming::Refused(Refusal::GapBelowFloor);
        }
        if row.hit1 < floors.min_hit1 {
            return Arming::Refused(Refusal::Hit1BelowFloor);
        }
    }
    Arming::Armed
}

/// The first class failing a floor, for the panel's naming print.
#[must_use]
pub fn first_failing_class(
    rows: &[ClassRow],
    floors: &ArmingFloors,
) -> Option<(CorruptionClass, Refusal)> {
    for row in rows {
        if row.gap < floors.min_gap {
            return Some((row.class, Refusal::GapBelowFloor));
        }
        if row.hit1 < floors.min_hit1 {
            return Some((row.class, Refusal::Hit1BelowFloor));
        }
    }
    None
}

/// Wilson 95% interval over a Bernoulli sample — `(estimate, lo, hi)`.
#[must_use]
pub fn wilson95(hits: usize, n: usize) -> (f64, f64, f64) {
    if n == 0 {
        return (0.0, 0.0, 0.0);
    }
    let p = count_to_f64(hits) / count_to_f64(n);
    let z = 1.959_963_984_540_054_f64;
    let n = count_to_f64(n);
    let denom = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denom;
    let half = (z / denom) * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt());
    (p, (center - half).max(0.0), (center + half).min(1.0))
}

/// Seeded bootstrap over per-program hit flags — `(estimate, lo, hi)` at
/// the 2.5 / 97.5 percentiles (B = `draws`; at B = 500 the low index is 12,
/// tail support 488 — disclosed here per the percentile law). Determinism:
/// same flags + seed, byte-identical interval.
#[must_use]
pub fn bootstrap_hit_ci(hit_flags: &[bool], draws: usize, seed: u64) -> (f64, f64, f64) {
    if hit_flags.is_empty() || draws == 0 {
        return (0.0, 0.0, 0.0);
    }
    let mut rng = SplitMix64::new(seed);
    let mut means = Vec::with_capacity(draws);
    for _ in 0..draws {
        let mut sum = 0.0f64;
        for _ in 0..hit_flags.len() {
            if hit_flags[rng.below(hit_flags.len())] {
                sum += 1.0;
            }
        }
        means.push(sum / count_to_f64(hit_flags.len()));
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo_idx = draws * 25 / 1000;
    let hi_idx = (draws * 975 / 1000).min(draws - 1);
    let estimate = means.iter().sum::<f64>() / count_to_f64(draws);
    (estimate, means[lo_idx], means[hi_idx])
}

// ── the loop-harm instruments (T6) ──────────────────────────────────────

/// The paper's measured failure mode, decomposed: a repair loop run over
/// planted errors must FIX them (`fixed == planted`) without BREAKING
/// previously-correct positions (`broke` above the tolerated rate refuses).
///
/// # Errors
///
/// Unfixed planted errors, a broken-position rate above `max_broke_rate`,
/// or a zero-planted run (nothing to fix — never a pass).
pub fn loop_harm_verdict(
    planted: usize,
    fixed: usize,
    broke: usize,
    max_broke_rate: f64,
) -> Result<(), String> {
    if planted == 0 {
        return Err("loop-harm: zero planted errors — nothing to fix, never a pass".to_string());
    }
    if fixed < planted {
        return Err(format!(
            "loop-harm: {fixed} of {planted} planted errors fixed (unfixed remain)"
        ));
    }
    let broke_rate = count_to_f64(broke) / count_to_f64(planted);
    if broke_rate > max_broke_rate {
        return Err(format!(
            "loop-harm: broke {broke} previously-correct position(s) \
             (rate {broke_rate:.3} > cap {max_broke_rate:.3})"
        ));
    }
    Ok(())
}

/// Paired non-inferiority over per-position correctness (before/after):
/// among positions correct BEFORE the loop, the kept rate must be at least
/// `1 − margin`. The paired form of [`loop_harm_verdict`]'s broke axis.
///
/// # Errors
///
/// Length mismatch, an empty vector, or a kept rate below the margin.
pub fn paired_non_inferior(before: &[bool], after: &[bool], margin: f64) -> Result<(), String> {
    if before.len() != after.len() {
        return Err(format!(
            "paired: length mismatch ({} before, {} after)",
            before.len(),
            after.len()
        ));
    }
    let previously_correct: Vec<bool> = before
        .iter()
        .zip(after.iter())
        .filter(|(was, _)| **was)
        .map(|(_, now)| *now)
        .collect();
    if previously_correct.is_empty() {
        return Err("paired: no previously-correct positions to protect".to_string());
    }
    let kept = previously_correct.iter().filter(|kept| **kept).count();
    let keep_rate = count_to_f64(kept) / count_to_f64(previously_correct.len());
    if keep_rate < 1.0 - margin {
        return Err(format!(
            "paired: kept {kept}/{} previously-correct positions (rate {keep_rate:.3} \
             < floor {:.3})",
            previously_correct.len(),
            1.0 - margin
        ));
    }
    Ok(())
}

// ── the generation-v9 panel (the bench runner body) ─────────────────────

fn score_all<'a>(
    scorer: &dyn SpanScorer,
    fixtures: &'a [CorruptionSpec],
) -> Vec<ScoredProgram<'a>> {
    let mut out = Vec::with_capacity(fixtures.len());
    for fixture in fixtures {
        match score_program(scorer, fixture) {
            Ok(scored_program) => out.push(scored_program),
            Err(why) => {
                eprintln!("generation v9: {why}");
                std::process::exit(1);
            }
        }
    }
    out
}

fn print_report_line(report: &EvalReport) {
    println!(
        "{:<22} gap {:+.3}  hit@1 {:.3}  hit@3 {:.3}  hit@5 {:.3}  fpr@5 {:.3}  mrr {:.3}",
        report.scorer,
        report.gap,
        report.hit[0],
        report.hit[1],
        report.hit[2],
        report.fpr5,
        report.mrr
    );
}

/// The live control laws — a violation means the METRIC is broken.
fn enforce_control_laws(
    constant_report: &EvalReport,
    shuffled_report: &EvalReport,
    inverse_report: &EvalReport,
    chance1: f64,
) {
    if !constant_report.constant || constant_report.gap != 0.0 {
        eprintln!(
            "generation v9: CONTROL LAW VIOLATION — constant scorer read gap {} (must be exactly 0.0)",
            constant_report.gap
        );
        std::process::exit(1);
    }
    let shuffle_band = chance1 + 0.15;
    if shuffled_report.hit[0] > shuffle_band {
        eprintln!(
            "generation v9: CONTROL LAW VIOLATION — shuffled hit@1 {} exceeds the chance band {}",
            shuffled_report.hit[0], shuffle_band
        );
        std::process::exit(1);
    }
    if inverse_report.gap >= 0.0 {
        eprintln!(
            "generation v9: CONTROL LAW VIOLATION — inverse scorer read gap {} (must be negative)",
            inverse_report.gap
        );
        std::process::exit(1);
    }
    println!("  controls: constant refused ✓  shuffled ≤ chance+0.15 ✓  inverse regressive ✓");
}

/// The arming-verdict block (each control + the oracle's own sanity arm).
fn print_arming_verdicts(
    oracle_report: &EvalReport,
    constant_report: &EvalReport,
    shuffled_report: &EvalReport,
    inverse_report: &EvalReport,
    floors: &ArmingFloors,
) {
    for (report, shuffled) in [
        (oracle_report, shuffled_report),
        (constant_report, shuffled_report),
        (shuffled_report, shuffled_report),
        (inverse_report, shuffled_report),
    ] {
        match arming_verdict(report, shuffled, floors) {
            Arming::Armed => println!("  arming {}: ARMED", report.scorer),
            Arming::Refused(refusal) => {
                println!("  arming {}: REFUSED {}", report.scorer, refusal.tag());
            }
        }
    }
    match arming_verdict(oracle_report, shuffled_report, floors) {
        Arming::Armed => {}
        Arming::Refused(refusal) => {
            eprintln!(
                "generation v9: CONTROL LAW VIOLATION — the oracle scorer failed to arm ({}) — floors are wrong",
                refusal.tag()
            );
            std::process::exit(1);
        }
    }
}

/// The per-class rows + the min-class binding print.
fn print_class_rows(oracle_scored: &[ScoredProgram], floors: &ArmingFloors) {
    let rows = per_class_rows(&OracleScorer, oracle_scored);
    println!("  per-class (oracle — buckets never pooled):");
    for row in &rows {
        println!(
            "    {:<9} n={:<2} gap {:+.3}  hit@1 {:.3}  hit@5 {:.3}  fpr@5 {:.3}  mrr {:.3}",
            row.class.tag(),
            row.n,
            row.gap,
            row.hit1,
            row.hit5,
            row.fpr5,
            row.mrr
        );
    }
    match class_floor_verdict(&rows, floors) {
        Arming::Armed => println!("  class-floor binding (min-class): ARMED"),
        Arming::Refused(refusal) => match first_failing_class(&rows, floors) {
            Some((class, _)) => println!(
                "  class-floor binding (min-class): REFUSED {} at class {}",
                refusal.tag(),
                class.tag()
            ),
            None => println!(
                "  class-floor binding (min-class): REFUSED {}",
                refusal.tag()
            ),
        },
    }
}

/// The generation-v9 panel: the headline table, the live control laws, the
/// arming verdicts, the per-class rows, and the CI disclosure. Any control
/// violation is a loud exit 1 (the instrument found the METRIC broken, not
/// a scorer).
pub fn run_generation9_panel() {
    match corruption::v9_freeze_status() {
        Ok(admitted) => println!(
            "generation v9: {admitted} admitted + {} control fixture(s) — corpus pins verified",
            corruption::control_corpus().len()
        ),
        Err(why) => {
            eprintln!("generation v9: {why}");
            std::process::exit(1);
        }
    }
    let fixtures = corruption::generation_v9();

    println!("─ corruption-bench (generation v9, confidence polarity) ────────");
    let oracle_scored = score_all(&OracleScorer, &fixtures);
    let oracle_report = evaluate(&OracleScorer, &oracle_scored);
    print_report_line(&oracle_report);
    for &k in &corruption::LADDER_KS {
        let subset: Vec<&ProgramRead> = oracle_report
            .reads
            .iter()
            .filter(|read| read.k == k)
            .collect();
        let n = subset.len();
        let hit1 = subset.iter().filter(|read| read.hit[0]).count();
        let mrr = mean(subset.iter().map(|read| read.rr));
        println!(
            "  k={k:<1} n={n:<2} hit@1 {:.3}  mrr {:.3}",
            count_to_f64(hit1) / count_to_f64(n.max(1)),
            mrr
        );
    }

    // the chance band + the controls
    let chance1 = mean(
        oracle_report
            .reads
            .iter()
            .map(|read| chance_hit_at_k(read.n_tokens, read.n_error_tokens, 1)),
    );
    let constant_report = evaluate(
        &ConstantScorer(0.5),
        &score_all(&ConstantScorer(0.5), &fixtures),
    );
    let shuffled_scorer = ShuffledScorer::new(Box::new(OracleScorer), 1);
    let shuffled_report = evaluate(&shuffled_scorer, &score_all(&shuffled_scorer, &fixtures));
    let inverse_scorer = InverseScorer::new(Box::new(OracleScorer));
    let inverse_report = evaluate(&inverse_scorer, &score_all(&inverse_scorer, &fixtures));

    println!("  chance hit@1 (rank-random): {chance1:.3}");
    print_report_line(&constant_report);
    print_report_line(&shuffled_report);
    print_report_line(&inverse_report);
    enforce_control_laws(&constant_report, &shuffled_report, &inverse_report, chance1);

    let floors = DEFAULT_ARMING_FLOORS;
    print_arming_verdicts(
        &oracle_report,
        &constant_report,
        &shuffled_report,
        &inverse_report,
        &floors,
    );
    print_class_rows(&oracle_scored, &floors);

    // the re-draw CI disclosure
    let flags: Vec<bool> = oracle_report.reads.iter().map(|read| read.hit[0]).collect();
    let (est, lo, hi) = bootstrap_hit_ci(&flags, 500, 0x4352_4239);
    println!(
        "  re-draw CI (hit@1, seeded bootstrap B=500): est {est:.3} [{lo:.3}, {hi:.3}] width {:.3}",
        hi - lo
    );

    println!(
        "  seam: no production confidence source wired — SpanScorer is the consumer contract;\n        the d2f remask / Plan 414 trained gap implement it and face these floors"
    );
    println!("────────────────────────────────────────────────────────────────");

    // T5 — the healer span-ranking POC (pre-registered: all three outcomes
    // are legitimate closers; only silence is a failure).
    run_healer_ranking_poc();
}

/// One item-level read for the T5 POC: the fixture split into top-level
/// non-test items, each tagged corrupted (contains an error span) or clean.
struct ItemRead<'a> {
    fixture_id: &'a str,
    corrupted: bool,
    /// The matcher's max rule-similarity score for this item's text —
    /// CONFIDENCE polarity (high = text looks corpus-typical = likely
    /// clean; the documented Bench-018 caveat says this is text similarity,
    /// not a certainty signal — exactly what T5 prices).
    score: f32,
}

/// Split one fixture into top-level items (the test module excluded — the
/// grader is trusted and never corrupted), tag each by error-span
/// containment, and score it with the healer's span ranking.
fn item_reads_for_fixture<'a>(
    matcher: &mut crate::draft::latent_matcher::LatentMatcher,
    fixture: &'a CorruptionSpec,
) -> Vec<ItemRead<'a>> {
    let Ok(file) = syn::parse_file(fixture.code) else {
        return Vec::new();
    };
    // the test module's byte range (excluded from the split)
    let test_range = fixture
        .code
        .find("mod tests")
        .map(|position| position..fixture.code.len());
    let mut reads = Vec::new();
    for item in &file.items {
        let span = item.span().byte_range();
        if span.start >= span.end || span.end > fixture.code.len() {
            continue;
        }
        if let Some(tests) = &test_range {
            if span.start >= tests.start {
                continue; // inside the test module
            }
        }
        let item_text = &fixture.code[span.clone()];
        let corrupted = fixture
            .spans
            .iter()
            .any(|(start, end)| span.start <= *start && *end <= span.end);
        let (_, by_domain) = matcher.suggest_span(item_text, 3);
        let score = by_domain
            .iter()
            .flat_map(|(_, rules)| rules.iter().map(|rule| rule.score))
            .fold(0.0f32, f32::max);
        reads.push(ItemRead {
            fixture_id: fixture.id,
            corrupted,
            score,
        });
    }
    reads.shrink_to_fit();
    reads
}

/// T5 — run the healer's span ranking over the generation-v9 fixtures at
/// ITEM granularity and pre-register the outcome:
/// (a) separation AND a consumer → promote (no consumer exists — the
///     healer has no semantic-bug fixer — so a positive gap lands as (c));
/// (b) no separation → the recorded finding (consistent with the
///     Bench-018 text-similarity caveat on `ScoredRule.score`);
/// (c) separation but no consumer → infrastructure-ahead-of-consumer.
fn run_healer_ranking_poc() {
    let mut matcher = match crate::draft::latent_matcher::LatentMatcher::from_enabled_domains() {
        Ok(matcher) => matcher,
        Err(why) => {
            println!("  T5 healer ranking: matcher unavailable ({why}) — skipped loud");
            return;
        }
    };
    let fixtures = corruption::generation_v9();
    let mut reads = Vec::new();
    for fixture in &fixtures {
        reads.extend(item_reads_for_fixture(&mut matcher, fixture));
    }
    let corrupted: Vec<&ItemRead> = reads.iter().filter(|read| read.corrupted).collect();
    let clean: Vec<&ItemRead> = reads.iter().filter(|read| !read.corrupted).collect();
    println!("─ T5 healer span-ranking POC (item granularity) ────────────────");
    if corrupted.is_empty() || clean.is_empty() {
        println!(
            "  T5: population degenerate ({} corrupted / {} clean items) — loud skip",
            corrupted.len(),
            clean.len()
        );
        return;
    }
    let corrupted_mean = mean(corrupted.iter().map(|read| f64::from(read.score)));
    let clean_mean = mean(clean.iter().map(|read| f64::from(read.score)));
    let gap = clean_mean - corrupted_mean;
    // hit@1 under confidence polarity: within each fixture, the corrupted
    // item is the LOWEST-similarity item (flagged first).
    let mut fixtures_total = 0usize;
    let mut hit1 = 0usize;
    for fixture_id in corrupted
        .iter()
        .map(|read| read.fixture_id)
        .collect::<BTreeSet<_>>()
    {
        let items: Vec<&ItemRead> = reads
            .iter()
            .filter(|read| read.fixture_id == fixture_id)
            .collect();
        if items.len() < 2 {
            continue;
        }
        fixtures_total += 1;
        let worst = items
            .iter()
            .min_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("non-empty");
        if worst.corrupted {
            hit1 += 1;
        }
    }
    let hit1_rate = count_to_f64(hit1) / count_to_f64(fixtures_total.max(1));
    // per-fixture rank-random chance: 1 / items-in-fixture, averaged over
    // the fixtures the hit arm actually scored
    let mut chance_sum = 0.0f64;
    for fixture_id in corrupted
        .iter()
        .map(|read| read.fixture_id)
        .collect::<BTreeSet<_>>()
    {
        let items = reads
            .iter()
            .filter(|read| read.fixture_id == fixture_id)
            .count();
        if items >= 2 {
            chance_sum += 1.0 / count_to_f64(items);
        }
    }
    let chance1 = chance_sum / count_to_f64(fixtures_total.max(1));
    // scale-free separation: the gap relative to the clean mean (similarity
    // scores are fused latent/BM25 in the tens — an absolute threshold would
    // be noise-level)
    let relative_gap = if clean_mean > 0.0 {
        gap / clean_mean
    } else {
        0.0
    };
    println!(
        "  items: {} corrupted / {} clean; means clean {clean_mean:.2} vs corrupted {corrupted_mean:.2}; gap {gap:+.2} (relative {relative_gap:+.3}); hit@1 {:.3} vs chance {:.3}",
        corrupted.len(),
        clean.len(),
        hit1_rate,
        chance1
    );
    let separated = relative_gap > 0.10 && hit1_rate > chance1 + 0.10;
    if separated {
        println!(
            "  T5 OUTCOME (c): the ranking SEPARATES corrupted from clean items — \
             but the healer has no semantic-bug fixer to consume it; \
             recorded infrastructure-ahead-of-consumer"
        );
    } else {
        println!(
            "  T5 OUTCOME (b): NO separation — text-similarity scores do not localize \
             CRB corruption (consistent with the ScoredRule.score Bench-018 caveat); \
             recorded finding"
        );
    }
    println!("────────────────────────────────────────────────────────────────");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic static fixture (leaks in tests only).
    fn static_fixture(
        id: &'static str,
        code: &'static str,
        spans: &'static [(usize, usize)],
    ) -> CorruptionSpec {
        CorruptionSpec {
            id,
            k: 1,
            code,
            spans,
            classes: &["operator"],
            blake3: Box::leak("0".repeat(64).into_boxed_str()),
            seed_program: "synthetic",
        }
    }

    fn synthetic_fixtures(count: usize) -> Vec<CorruptionSpec> {
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            let id: &'static str = Box::leak(format!("synthetic-{index:02}").into_boxed_str());
            let code: &'static str = Box::leak(
                format!("pub fn f{index}(a: i64, b: i64) -> i64 {{ a * b + {index} }}\n")
                    .into_boxed_str(),
            );
            let pos = code.find('*').expect("a `*` in the synthetic body");
            let spans: &'static [(usize, usize)] =
                Box::leak(vec![(pos, pos + 1)].into_boxed_slice());
            out.push(static_fixture(id, code, spans));
        }
        out
    }

    #[test]
    fn leaf_tokens_cover_all_leaves_with_byte_ranges() {
        let tokens = leaf_tokens("let x = a + b;\n").expect("tokenizes");
        let text = "let x = a + b;\n";
        assert_eq!(tokens.len(), 7, "let x = a + b ; = 7 leaves");
        for token in &tokens {
            assert_ne!(text[token.start..token.end].to_string(), "");
            assert!(text.is_char_boundary(token.start));
            assert!(text.is_char_boundary(token.end));
        }
    }

    #[test]
    fn oracle_scores_perfectly_and_gate_arms() {
        let fixtures = synthetic_fixtures(6);
        let scored: Vec<ScoredProgram> = fixtures
            .iter()
            .map(|fixture| score_program(&OracleScorer, fixture).expect("scores"))
            .collect();
        let report = evaluate(&OracleScorer, &scored);
        assert!((report.gap - 1.0).abs() < 1e-6, "oracle gap must read 1.0");
        assert!((report.hit[0] - 1.0).abs() < 1e-6);
        // FPR@5 floor: every synthetic fixture has |E|=1, so even the
        // perfect scorer flags (5−1)/5 = 0.8 clean slots — the structural
        // (K−|E|)/K floor, not a scorer defect.
        assert!(
            (report.fpr5 - 0.8).abs() < 1e-6,
            "fpr5 {} must read the (K−|E|)/K floor",
            report.fpr5
        );
        assert!((report.mrr - 1.0).abs() < 1e-6);
        assert!(!report.constant);
        let shuffled = ShuffledScorer::new(Box::new(OracleScorer), 7);
        let shuffled_scored: Vec<ScoredProgram> = fixtures
            .iter()
            .map(|fixture| score_program(&shuffled, fixture).expect("scores"))
            .collect();
        let shuffled_report = evaluate(&shuffled, &shuffled_scored);
        assert_eq!(
            arming_verdict(&report, &shuffled_report, &DEFAULT_ARMING_FLOORS),
            Arming::Armed
        );
    }

    #[test]
    fn constant_scorer_reads_zero_gap_and_is_refused() {
        let fixtures = synthetic_fixtures(3);
        let scored: Vec<ScoredProgram> = fixtures
            .iter()
            .map(|fixture| score_program(&ConstantScorer(0.5), fixture).expect("scores"))
            .collect();
        let report = evaluate(&ConstantScorer(0.5), &scored);
        assert!(report.constant, "constant scorer must be detected");
        // the loud-zero law: EXACT zero — never "within epsilon"
        #[allow(clippy::float_cmp)]
        let is_exact_zero = report.gap == 0.0;
        assert!(
            is_exact_zero,
            "gap must read exactly 0.0, got {}",
            report.gap
        );
        let shuffled = ShuffledScorer::new(Box::new(ConstantScorer(0.5)), 7);
        let shuffled_scored: Vec<ScoredProgram> = fixtures
            .iter()
            .map(|fixture| score_program(&shuffled, fixture).expect("scores"))
            .collect();
        let shuffled_report = evaluate(&shuffled, &shuffled_scored);
        assert_eq!(
            arming_verdict(&report, &shuffled_report, &DEFAULT_ARMING_FLOORS),
            Arming::Refused(Refusal::ConstantConfidence)
        );
    }

    #[test]
    fn shuffled_control_falls_to_chance_band() {
        let fixtures = synthetic_fixtures(40);
        let shuffler = ShuffledScorer::new(Box::new(OracleScorer), 11);
        let scored: Vec<ScoredProgram> = fixtures
            .iter()
            .map(|fixture| score_program(&shuffler, fixture).expect("scores"))
            .collect();
        let report = evaluate(&shuffler, &scored);
        let chance = mean(
            report
                .reads
                .iter()
                .map(|read| chance_hit_at_k(read.n_tokens, read.n_error_tokens, 1)),
        );
        assert!(
            report.hit[0] <= chance + 0.15,
            "shuffled hit@1 {} must sit in the chance band {}",
            report.hit[0],
            chance + 0.15
        );
        assert!(
            report.gap.abs() < 0.15,
            "shuffled gap {} must collapse toward 0",
            report.gap
        );
    }

    #[test]
    fn inverse_scorer_is_regressively_refused() {
        let fixtures = synthetic_fixtures(4);
        let inverse = InverseScorer::new(Box::new(OracleScorer));
        let scored: Vec<ScoredProgram> = fixtures
            .iter()
            .map(|fixture| score_program(&inverse, fixture).expect("scores"))
            .collect();
        let report = evaluate(&inverse, &scored);
        assert!(report.gap < 0.0, "inverse gap must be negative");
        let shuffled = ShuffledScorer::new(Box::new(OracleScorer), 7);
        let shuffled_scored: Vec<ScoredProgram> = fixtures
            .iter()
            .map(|fixture| score_program(&shuffled, fixture).expect("scores"))
            .collect();
        let shuffled_report = evaluate(&shuffled, &shuffled_scored);
        assert_eq!(
            arming_verdict(&report, &shuffled_report, &DEFAULT_ARMING_FLOORS),
            Arming::Refused(Refusal::RegressiveGap)
        );
    }

    #[test]
    fn emission_floor_rejects_a_truncated_scorer() {
        struct Truncated;
        impl SpanScorer for Truncated {
            fn name(&self) -> &'static str {
                "truncated"
            }
            fn score(&self, ctx: &ScoringContext) -> Vec<SpanScore> {
                ctx.tokens[..ctx.tokens.len() - 1]
                    .iter()
                    .map(|range| SpanScore {
                        start: range.start,
                        end: range.end,
                        score: 1.0,
                    })
                    .collect()
            }
        }
        let fixtures = synthetic_fixtures(1);
        let error = score_program(&Truncated, &fixtures[0]).expect_err("must refuse");
        assert!(error.contains("full-ranking emission floor"), "{error}");
    }

    #[test]
    fn misaligned_scorer_is_rejected() {
        struct Shifted;
        impl SpanScorer for Shifted {
            fn name(&self) -> &'static str {
                "shifted"
            }
            fn score(&self, ctx: &ScoringContext) -> Vec<SpanScore> {
                ctx.tokens
                    .iter()
                    .map(|range| SpanScore {
                        start: range.start + 1,
                        end: range.end + 1,
                        score: 1.0,
                    })
                    .collect()
            }
        }
        let fixtures = synthetic_fixtures(1);
        let error = score_program(&Shifted, &fixtures[0]).expect_err("must refuse");
        assert!(error.contains("misaligned"), "{error}");
    }

    #[test]
    fn arming_ladder_names_each_refusal_in_order() {
        let floors = DEFAULT_ARMING_FLOORS;
        let base = EvalReport {
            scorer: "base".to_string(),
            reads: Vec::new(),
            gap: 1.0,
            hit: [1.0, 1.0, 1.0],
            fpr5: 0.0,
            mrr: 1.0,
            constant: false,
        };
        let shuffled = EvalReport {
            scorer: "shuffled".to_string(),
            reads: Vec::new(),
            gap: 0.0,
            hit: [0.05, 0.1, 0.2],
            fpr5: 0.9,
            mrr: 0.1,
            constant: false,
        };
        assert_eq!(arming_verdict(&base, &shuffled, &floors), Arming::Armed);
        // constant wins first
        let mut constant = base.clone();
        constant.constant = true;
        assert_eq!(
            arming_verdict(&constant, &shuffled, &floors),
            Arming::Refused(Refusal::ConstantConfidence)
        );
        // regressive beats the floors
        let mut regressive = base.clone();
        regressive.gap = -0.5;
        assert_eq!(
            arming_verdict(&regressive, &shuffled, &floors),
            Arming::Refused(Refusal::RegressiveGap)
        );
        // no-signal beats the gap floor
        let mut weak = base.clone();
        weak.gap = 0.5;
        weak.hit = [0.1, 0.2, 0.3];
        assert_eq!(
            arming_verdict(&weak, &shuffled, &floors),
            Arming::Refused(Refusal::NoSignalOverShuffle)
        );
        // gap floor
        let mut thin_gap = base.clone();
        thin_gap.gap = 0.01;
        thin_gap.hit = [0.5, 0.7, 0.9];
        assert_eq!(
            arming_verdict(&thin_gap, &shuffled, &floors),
            Arming::Refused(Refusal::GapBelowFloor)
        );
        // hit floor
        let mut thin_hit = base.clone();
        thin_hit.hit = [0.2, 0.4, 0.6];
        assert_eq!(
            arming_verdict(&thin_hit, &shuffled, &floors),
            Arming::Refused(Refusal::Hit1BelowFloor)
        );
        // fpr cap
        let mut noisy = base.clone();
        noisy.fpr5 = 0.9;
        assert_eq!(
            arming_verdict(&noisy, &shuffled, &floors),
            Arming::Refused(Refusal::Fpr5AboveCap)
        );
    }

    #[test]
    fn class_floor_binds_per_class_not_pooled() {
        let floors = DEFAULT_ARMING_FLOORS;
        let rows = vec![
            ClassRow {
                class: CorruptionClass::Operator,
                n: 10,
                gap: 1.0,
                hit1: 0.9,
                hit5: 1.0,
                fpr5: 0.0,
                mrr: 0.95,
            },
            ClassRow {
                class: CorruptionClass::Ident,
                n: 2,
                gap: 1.0,
                hit1: 0.1,
                hit5: 0.2,
                fpr5: 0.0,
                mrr: 0.15,
            },
        ];
        // the pooled MEAN of hit1 (0.6·… weighted by n would be ~0.77) would
        // clear the 0.30 floor — the min-class rule must refuse anyway.
        let pooled_mean_hit1 = (0.9 * 10.0 + 0.1 * 2.0) / 12.0;
        assert!(
            pooled_mean_hit1 > floors.min_hit1,
            "test premise: pooled passes"
        );
        assert_eq!(
            class_floor_verdict(&rows, &floors),
            Arming::Refused(Refusal::Hit1BelowFloor)
        );
        let (class, why) = first_failing_class(&rows, &floors).expect("a failing class");
        assert_eq!(class, CorruptionClass::Ident);
        assert_eq!(why, Refusal::Hit1BelowFloor);
        // every class clearing arms
        let healthy: Vec<ClassRow> = rows
            .iter()
            .map(|row| ClassRow {
                hit1: 0.9,
                ..row.clone()
            })
            .collect();
        assert_eq!(class_floor_verdict(&healthy, &floors), Arming::Armed);
    }

    #[test]
    fn chance_formula_matches_hand_computation() {
        // N=20, e=2, K=1: P = 2/20 = 0.1
        assert!((chance_hit_at_k(20, 2, 1) - 0.1).abs() < 1e-9);
        // N=20, e=20, K=3 → 1.0
        assert!((chance_hit_at_k(20, 20, 3) - 1.0).abs() < 1e-9);
        // N=10, e=1, K=10 → 1.0 (K ≥ N)
        assert!((chance_hit_at_k(10, 1, 10) - 1.0).abs() < 1e-9);
        // N=10, e=5, K=2: 1 − C(5,2)/C(10,2) = 1 − 10/45
        assert!((chance_hit_at_k(10, 5, 2) - (1.0 - 10.0 / 45.0)).abs() < 1e-9);
    }

    #[test]
    fn wilson_interval_is_sane() {
        let (est, lo, hi) = wilson95(0, 10);
        assert!((est - 0.0).abs() < 1e-9);
        assert!(lo < 0.01, "lower bound at 0 hits ≈ 0, got {lo}");
        assert!(hi > 0.0 && hi < 0.35);
        let (est, lo, hi) = wilson95(10, 10);
        assert!((est - 1.0).abs() < 1e-9);
        assert!(hi > 0.99, "upper bound at 10/10 ≈ 1, got {hi}");
        assert!(lo > 0.65);
        let (_, lo, hi) = wilson95(5, 10);
        assert!(lo < 0.5 && hi > 0.5);
        assert_eq!(wilson95(3, 0), (0.0, 0.0, 0.0));
    }

    #[test]
    fn bootstrap_is_deterministic_and_brackets_the_estimate() {
        let flags = vec![true, false, true, true, false, true, true, true];
        let (est_a, lo_a, hi_a) = bootstrap_hit_ci(&flags, 500, 42);
        let (est_b, lo_b, hi_b) = bootstrap_hit_ci(&flags, 500, 42);
        assert_eq!((est_a, lo_a, hi_a), (est_b, lo_b, hi_b));
        assert!(lo_a <= est_a && est_a <= hi_a);
        let (_, lo_c, _) = bootstrap_hit_ci(&flags, 500, 43);
        // a different seed MAY shift the interval — assert it stays valid
        assert!((0.0..=1.0).contains(&lo_c));
    }

    #[test]
    fn loop_harm_refuses_unfixed_and_broken_positions() {
        assert!(loop_harm_verdict(4, 4, 0, 0.05).is_ok());
        assert!(loop_harm_verdict(4, 3, 0, 0.05)
            .unwrap_err()
            .contains("unfixed"));
        assert!(loop_harm_verdict(4, 4, 1, 0.05)
            .unwrap_err()
            .contains("broke 1"));
        assert!(loop_harm_verdict(0, 0, 0, 0.05).is_err());
    }

    #[test]
    fn paired_non_inferiority_binds_the_protected_positions() {
        let before = vec![true, true, true, true, true, true, true, true, true, true];
        let mut after = before.clone();
        after[0] = false;
        // kept 9/10 = 0.9: margin 0.05 refuses, 0.15 passes
        assert!(paired_non_inferior(&before, &after, 0.05).is_err());
        assert!(paired_non_inferior(&before, &after, 0.15).is_ok());
        // length mismatch + empty protections are errors
        assert!(paired_non_inferior(&before, &[], 0.1).is_err());
        assert!(paired_non_inferior(&[false, false], &[false, false], 0.1).is_err());
    }
}
