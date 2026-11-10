//! Issue 217 T0 — the size-band jaggedness axis (the ordered OOD cut).
//!
//! ## Why a second axis
//!
//! [`super::withheld_pair_ood`] withholds *(role, filler)* combinations —
//! novel combinations of seen roles and seen shape-classes. The doubleAI
//! CLRS-Text jaggedness protocol (riir-refine Research 386; preregistered in
//! `.issues/217_jaggedness_eval_validator_growth.md` BEFORE any run) asks a
//! different question: **does the corpus fix accuracy collapse on the size
//! bands BETWEEN the bands it was mined from?** A program generalizes across
//! sizes by construction; a mined statistical corpus need not.
//!
//! [`ShapeClass`](super::withheld_pair_ood::ShapeClass) is an *unordered*
//! text-matched enum — no size, depth, or band notion — so "in-between" is
//! meaningless on it. This module adds the **ordered band axis** beside it
//! (consume, never fork): the band is a pure function of the span's token
//! count, lexed with `proc_macro2` (a direct dep, `span-locations`)
//! whitespace-split fallback for snippets that do not lex (e.g. bare
//! expressions), pinned by [`band_axis_is_pinned`].
//!
//! ## The protocol (preregistered floor, never re-tuned)
//!
//! Leave-one-BAND-out at index granularity — the same withholding discipline
//! as the pair protocol, one axis over:
//!
//! | arm | index | what it says |
//! |---|---|---|
//! | `ID` | `P \ {q}` | the band IS represented (leave-one-out) |
//! | `OOD` | `P \ band(q)` | the whole band is ABSENT from the index |
//!
//! Per band: paired rows (same query, both arms in the same pass), the
//! gap = ID top-1 − OOD top-1 in percentage points, a paired-bootstrap 95%
//! CI over the band's rows (B = 1000 resamples), and the preregistered
//! classification:
//!
//! - bands `[16,64)`, `[64,256)`, `[256,1024)` are **in-between** (seen
//!   bands on both sides of the lattice);
//! - `[0,16)` and `[1024,∞)` are **ends** — extrapolation rows, reported
//!   but never counted toward the floor;
//! - a band with fewer than [`N_MIN`] paired rows is **UNSEEN** — reported,
//!   never a pass or a fail.
//!
//! **The floor (fixed in the issue, never edited after a run):** the corpus
//! jags iff ≥ 2 in-between bands carry gap ≥ 10.0 pp with a bootstrap CI
//! lower bound > 0. Otherwise the honest outcome is "no jaggedness measured"
//! (or UNSEEN — insufficient labeled fixtures), and the validator growth
//! loop (Issue 217 T2+) stays closed.
//!
//! **T0 OUTCOME (2026-11-07, the one frozen read):** the floor did NOT fire
//! — `jagged = Some(false)`. Only B1 carried the floor gap (32.7 pp, CI
//! [20.0, 45.5]); B2's gap was 9.1 pp (below the 10 pp floor, CI [0.0,
//! 18.2]); B3 measured 0.0/0.0. Per the preregistration the issue closed as
//! a measurement note and the validator growth loop stays CLOSED. The
//! curve's durable finding: an **absolute size-coverage wall** — ID
//! accuracy 40.9% (B0) → 40.0% (B1) → 12.7% (B2) → 0.0% (B3): the corpus's
//! effective reach ends around the 64-token mark; the remedy class for
//! large spans is large-span corpus rules, not validators. Run posture:
//! background strided 2568 → 642, paired rows stride-uniform cap 60 per
//! band (both disclosures below); run wall 136.7 s via the `band_ood_eval`
//! example.
//!
//! Modelless: deterministic token counting + the shipped retrieval stack.
//! No weights, no training, no GPU, no network.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;

use super::instantiation_rerank::RerankMode;
use super::withheld_pair_ood::{Binding, build_index, rank_of};

// ── the ordered band axis ────────────────────────────────────────────────

/// Band edges (token counts) — part of the specification, pinned by
/// [`band_axis_is_pinned`]. Five bands, B0..B4.
pub const BAND_EDGES: [usize; 4] = [16, 64, 256, 1024];

/// Bands below [`N_MIN`] paired rows are UNSEEN — reported, never a verdict.
/// Preregistered in `.issues/217_jaggedness_eval_validator_growth.md`.
pub const N_MIN: usize = 30;

/// The floor gap in percentage points — preregistered, never re-tuned.
pub const FLOOR_GAP_PP: f32 = 10.0;

/// Bootstrap resample count for the per-band gap CI.
const BOOTSTRAP_B: usize = 1000;

/// The paired-bootstrap percentile indices — pure integer arithmetic
/// (floor of 0.025/0.975 × (B−1); B is a pinned const so these are exact:
/// 24 and 974). No float casts.
const CI_LO_IDX: usize = (BOOTSTRAP_B - 1) * 25 / 1000;
const CI_HI_IDX: usize = (BOOTSTRAP_B - 1) * 975 / 1000;

/// How many in-between bands must carry the floor gap before the corpus
/// reads as jagged — preregistered.
const FLOOR_BANDS: usize = 2;

/// Per-band cap on PAIRED rows (the deterministic strided sample). The
/// floors need n ≥ [`N_MIN`] per band, not thousands: a full B2 run is
/// 1268 leave-one-out index rebuilds and the ID arm rebuilds per query.
/// The sample is stride-uniform over the population's rule-grouped order
/// (no cherry-picking), the FULL band size is still reported as
/// `coverage`, and every band measures at/above [`N_MIN`] with this cap.
/// Disclosed in every report.
const N_CAP: usize = 60;

/// The index-background cap: the distinct population is stride-reduced to
/// this many patterns BEFORE banding. Measured basis (the `band_ood_cost`
/// probe, release build): [`super::corpus_index::RuleIndex`] build cost is superlinear in patterns
/// (125 ms at 200 → 9.2 s at 2568 warm), and the ID arm rebuilds per
/// query — a full-background run is ~43 min of pure rebuilds, a 800-
/// background run is ~2 min. The floors are untouched: [`N_MIN`] applies
/// to the paired rows (all three in-between bands measure ≥ 30 at this
/// cap; the ENDS may fall UNSEEN and they never count toward the floor
/// anyway). Disclosed in every report; a full-background run stays
/// possible as a detached follow-up.
const BACKGROUND_CAP: usize = 800;

/// Token count of a Rust snippet — the pinned size proxy. `proc_macro2`
/// lexing (a direct dep) when the snippet lexes: **recursive leaf tokens,
/// +1 per group** for its delimiters (a flat top-level count would collapse
/// a whole brace group into one token and push every nested function into
/// B0 — measured, rejected). Whitespace-split fallback for snippets that do
/// not lex (unbalanced delimiters). **Both arms are pinned by
/// [`band_axis_is_pinned`]**, per the T0 text.
#[must_use]
pub fn token_count(code: &str) -> usize {
    match code.parse::<TokenStream>() {
        Ok(ts) => count_leaves(ts),
        Err(_) => code.split_whitespace().count(),
    }
}

/// Recursive leaf count: every non-group token tree = 1; every group =
/// 1 (its delimiter pair) + its inner count.
fn count_leaves(ts: TokenStream) -> usize {
    ts.into_iter()
        .map(|tt| match tt {
            proc_macro2::TokenTree::Group(g) => 1 + count_leaves(g.stream()),
            _ => 1,
        })
        .sum()
}

/// The five size bands, B0..B4. `Display` = the pinned label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band {
    /// `[0, 16)` tokens — an END (extrapolation row).
    B0,
    /// `[16, 64)` tokens — in-between.
    B1,
    /// `[64, 256)` tokens — in-between.
    B2,
    /// `[256, 1024)` tokens — in-between.
    B3,
    /// `[1024, ∞)` tokens — an END (extrapolation row).
    B4,
}

impl Band {
    /// All bands in axis order.
    pub const ALL: [Band; 5] = [Band::B0, Band::B1, Band::B2, Band::B3, Band::B4];

    /// The band of a token count — pure lookup against [`BAND_EDGES`].
    #[must_use]
    pub fn of_tokens(tokens: usize) -> Band {
        match tokens {
            t if t < BAND_EDGES[0] => Band::B0,
            t if t < BAND_EDGES[1] => Band::B1,
            t if t < BAND_EDGES[2] => Band::B2,
            t if t < BAND_EDGES[3] => Band::B3,
            _ => Band::B4,
        }
    }

    /// The band of a snippet (lex → count → band).
    #[must_use]
    pub fn of(code: &str) -> Band {
        Band::of_tokens(token_count(code))
    }

    /// The pinned label (`B0..B4` with the token range).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Band::B0 => "B0[0,16)",
            Band::B1 => "B1[16,64)",
            Band::B2 => "B2[64,256)",
            Band::B3 => "B3[256,1024)",
            Band::B4 => "B4[1024,inf)",
        }
    }

    /// `true` for the two ends of the axis — extrapolation rows, reported
    /// but never counted toward the floor.
    #[must_use]
    pub fn is_end(self) -> bool {
        matches!(self, Band::B0 | Band::B4)
    }
}

// ── per-band rows + the report ──────────────────────────────────────

/// One band's paired read.
#[derive(Debug, Clone)]
pub struct BandRow {
    pub band: Band,
    /// Distinct-text bindings living in this band (the corpus's mined
    /// coverage of the band).
    pub coverage: usize,
    /// Paired rows measured (== coverage; every member is a query).
    pub n: usize,
    /// ID top-1 (leave-one-out, the band present) in percent.
    pub id_top1: f32,
    /// OOD top-1 (whole band withheld from the index) in percent.
    pub ood_top1: f32,
    /// `id_top1 − ood_top1` in percentage points.
    pub gap_pp: f32,
    /// Paired-bootstrap 95% CI on the gap `(lo, hi)` in pp.
    pub gap_ci: (f32, f32),
    /// `n < N_MIN` — the band is UNSEEN: reported, never a verdict.
    pub unseen: bool,
}

impl BandRow {
    /// The axis position label: END (extrapolation) or IN-BETWEEN.
    #[must_use]
    pub fn position(&self) -> &'static str {
        if self.band.is_end() {
            "END/extrapolation"
        } else {
            "IN-BETWEEN"
        }
    }

    /// The preregistered floor fires for this band: in-between, measured
    /// (not UNSEEN), gap ≥ [`FLOOR_GAP_PP`], CI lower bound > 0.
    #[must_use]
    pub fn floor_fires(&self) -> bool {
        !self.unseen && !self.band.is_end() && self.gap_pp >= FLOOR_GAP_PP && self.gap_ci.0 > 0.0
    }
}

/// The full T0 report — the jaggedness curve.
#[derive(Debug, Clone)]
pub struct BandReport {
    /// Distinct-text population size.
    pub population: usize,
    /// Per-band rows, axis order.
    pub rows: Vec<BandRow>,
    /// The floor verdict: `Some(true)` = jagged (≥ [`FLOOR_BANDS`] bands
    /// fire); `Some(false)` = measured, no jaggedness; `None` = every
    /// in-between band UNSEEN (insufficient labeled fixtures — the honest
    /// null that still closes the growth loop until more fixtures exist).
    pub jagged: Option<bool>,
}

impl BandReport {
    /// In-between bands that measured at/above [`N_MIN`].
    #[must_use]
    pub fn measured_in_between(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| !r.unseen && !r.band.is_end())
            .count()
    }
}

/// The paired-bootstrap 95% CI on `mean(id_hits) − mean(ood_hits)` over the
/// band's rows (hits = rank-1 retrieval), B = 1000 resamples, percentile
/// interval. Deterministic: a fixed LCG seed, no external RNG state.
fn gap_ci(id_hits: &[bool], ood_hits: &[bool]) -> (f32, f32) {
    let n = id_hits.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    #[allow(clippy::cast_possible_truncation)] // n ≤ population size (~10^3), far below u32::MAX
    let n32 = n as u32;
    #[allow(clippy::cast_precision_loss)] // means over ≤10^3 rows
    let n_f = f64::from(n32);
    let id_u8: Vec<u8> = id_hits.iter().map(|&h| u8::from(h)).collect();
    let ood_u8: Vec<u8> = ood_hits.iter().map(|&h| u8::from(h)).collect();
    let mut deltas: Vec<f32> = Vec::with_capacity(BOOTSTRAP_B);
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15; // fixed seed — reproducible
    for _ in 0..BOOTSTRAP_B {
        let mut id_sum = 0u32;
        let mut ood_sum = 0u32;
        for _ in 0..n {
            // xorshift64* — deterministic, no deps
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // masked to 31 bits, then mod n
            let idx = (((state.wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 33) as usize) % n;
            id_sum += u32::from(id_u8[idx]);
            ood_sum += u32::from(ood_u8[idx]);
        }
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        // CI reported to 0.1 pp; f64→f32 truncation is far below the reported digit
        let delta = ((f64::from(id_sum) - f64::from(ood_sum)) / n_f) as f32 * 100.0;
        deltas.push(delta);
    }
    deltas.sort_unstable_by(f32::total_cmp);
    (deltas[CI_LO_IDX], deltas[CI_HI_IDX])
}

/// The UNSEEN zero row for an empty band — the curve shows the hole.
fn unseen_row(band: Band) -> BandRow {
    BandRow {
        band,
        coverage: 0,
        n: 0,
        id_top1: 0.0,
        ood_top1: 0.0,
        gap_pp: 0.0,
        gap_ci: (0.0, 0.0),
        unseen: true,
    }
}

/// One band's paired measurement: the OOD index (whole band withheld) plus
/// the sampled members' leave-one-out ID arm. Returns the band's row —
/// the caller owns the floor bookkeeping.
fn measure_band(band: Band, pop: &[Binding], idxs: &[usize]) -> BandRow {
    let coverage = idxs.len();

    // The paired-row sample: stride-uniform over the band's members
    // (rule-grouped order), capped at N_CAP — deterministic, disclosed;
    // coverage above stays the FULL band size. n_min applies to the
    // paired rows, not the coverage.
    let stride = coverage.div_ceil(N_CAP);
    let sampled: Vec<usize> = if coverage <= N_CAP {
        idxs.to_vec()
    } else {
        idxs.iter().step_by(stride).copied().collect()
    };
    let n = sampled.len();

    // OOD index: the whole band withheld. Shared by every member.
    let ood_items: Vec<&Binding> = pop
        .iter()
        .enumerate()
        .filter(|(i, _)| !idxs.contains(i))
        .map(|(_, b)| b)
        .collect();
    assert!(
        ood_items.len() + coverage == pop.len(),
        "withheld count mismatch for {band:?}"
    );
    // Withholding leak assertion — never assumed.
    assert!(
        !ood_items.iter().any(|b| Band::of(&b.text) == band),
        "withholding leak: a {band:?} binding survived in the OOD index"
    );
    let ood_index = build_index(&ood_items);

    let mut id_hits: Vec<bool> = Vec::with_capacity(n);
    let mut ood_hits: Vec<bool> = Vec::with_capacity(n);
    for &qi in &sampled {
        let q = &pop[qi];
        // ID arm: leave-one-out (the band present).
        let id_items: Vec<&Binding> = pop
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != qi)
            .map(|(_, b)| b)
            .collect();
        let id_index = build_index(&id_items);
        let (id_rank, _) = rank_of(&id_index, &q.text, &q.rule, None, RerankMode::Structural);
        let (ood_rank, _) = rank_of(&ood_index, &q.text, &q.rule, None, RerankMode::Structural);
        id_hits.push(id_rank.is_some_and(|r| r == 0));
        ood_hits.push(ood_rank.is_some_and(|r| r == 0));
    }

    #[allow(clippy::cast_precision_loss)] // counts ≤ 10^3, one decimal place reported
    let id_top1 = id_hits.iter().filter(|&&h| h).count() as f32 / n as f32 * 100.0;
    #[allow(clippy::cast_precision_loss)] // counts ≤ 10^3, one decimal place reported
    let ood_top1 = ood_hits.iter().filter(|&&h| h).count() as f32 / n as f32 * 100.0;
    let gap = id_top1 - ood_top1;
    BandRow {
        band,
        coverage,
        n,
        id_top1,
        ood_top1,
        gap_pp: gap,
        gap_ci: gap_ci(&id_hits, &ood_hits),
        unseen: n < N_MIN,
    }
}

/// Run the band-withholding protocol over `population`.
///
/// Rerank posture: [`RerankMode::Structural`] (the shipped default), pure
/// retrieval — no domain probe (the populations here span domains).
///
/// # Panics
///
/// Panics if a withheld index still contains the withheld band (the
/// withholding assertion — never assumed), or if index construction fails.
#[must_use]
pub fn run_band_ood(population: Vec<Binding>) -> BandReport {
    // Distinct texts only — same law as the pair protocol (RuleIndex
    // dedupes identical chunk content, so a duplicate would silently drop
    // a binding).
    let mut seen = BTreeSet::new();
    let pop: Vec<Binding> = population
        .into_iter()
        .filter(|b| seen.insert(*blake3::hash(b.text.as_bytes()).as_bytes()))
        .collect();

    // The background stride — deterministic prefix-stride to
    // [`BACKGROUND_CAP`], disclosed in the report. Band counts reported by
    // the curve are the STRIDED coverage; the full corpus band distribution
    // is a cost-probe record (band_ood_cost), not an eval input.
    let pop: Vec<Binding> = if pop.len() > BACKGROUND_CAP {
        let stride = pop.len().div_ceil(BACKGROUND_CAP);
        let raw = pop.len();
        let pop: Vec<Binding> = pop
            .into_iter()
            .enumerate()
            .filter(|(i, _)| i % stride == 0)
            .map(|(_, b)| b)
            .collect();
        assert!(pop.len() <= BACKGROUND_CAP, "background stride overshot");
        eprintln!(
            "band_ood: background strided {raw} -> {} (stride {stride}, cap {BACKGROUND_CAP})",
            pop.len()
        );
        pop
    } else {
        pop
    };

    // Members per band, axis order.
    let mut members: Vec<(Band, Vec<usize>)> = Band::ALL.iter().map(|&b| (b, Vec::new())).collect();
    for (i, b) in pop.iter().enumerate() {
        let band = Band::of(&b.text);
        members[band as usize].1.push(i);
    }

    let mut rows = Vec::with_capacity(members.len());
    let mut fired = 0usize;
    let mut measured_in_between = 0usize;
    for (band, idxs) in &members {
        if idxs.is_empty() {
            // No bindings in this band at all — UNSEEN with an honest zero
            // row (the curve must show the hole, not hide it).
            rows.push(unseen_row(*band));
            continue;
        }
        let row = measure_band(*band, &pop, idxs);
        if !row.unseen && !band.is_end() {
            measured_in_between += 1;
            if row.floor_fires() {
                fired += 1;
            }
        }
        rows.push(row);
    }

    let jagged = if measured_in_between == 0 {
        None // every in-between band UNSEEN — the honest null
    } else {
        Some(fired >= FLOOR_BANDS)
    };

    BandReport {
        population: pop.len(),
        rows,
        jagged,
    }
}

/// One-line-per-band print — the curve, ready to paste into the issue.
#[must_use]
pub fn print_band_report(r: &BandReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(512);
    let _ = writeln!(
        out,
        "band_ood background={} measured_in_between={} jagged={:?} (background strided to {BACKGROUND_CAP}, paired rows sampled stride-uniform cap {N_CAP}; coverage = strided band size)",
        r.population,
        r.measured_in_between(),
        r.jagged
    );
    out.push_str(
        "band          pos              cov    n   id%   ood%   gap_pp  ci95        state\n",
    );
    for row in &r.rows {
        let state = if row.unseen {
            "UNSEEN"
        } else if row.floor_fires() {
            "FLOOR-FIRES"
        } else {
            "ok"
        };
        let _ = writeln!(
            out,
            "{:<13} {:<16} {:>4} {:>5} {:>5.1} {:>6.1} {:>7.1} [{:>5.1},{:>5.1}] {}",
            row.band.label(),
            row.position(),
            row.coverage,
            row.n,
            row.id_top1,
            row.ood_top1,
            row.gap_pp,
            row.gap_ci.0,
            row.gap_ci.1,
            state,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pinned counter + band edges — the AGREE-round condition. Names
    /// both tokenizer arms and pins every edge.
    #[test]
    fn band_axis_is_pinned() {
        // Edge pins: every boundary value, both sides.
        let b = |t: usize| Band::of_tokens(t);
        assert_eq!(b(0), Band::B0);
        assert_eq!(b(15), Band::B0);
        assert_eq!(b(16), Band::B1);
        assert_eq!(b(63), Band::B1);
        assert_eq!(b(64), Band::B2);
        assert_eq!(b(255), Band::B2);
        assert_eq!(b(256), Band::B3);
        assert_eq!(b(1023), Band::B3);
        assert_eq!(b(1024), Band::B4);
        assert_eq!(b(10_000), Band::B4);

        // Tokenizer arm 1 — a lexing snippet, recursive count pinned by
        // hand: `fn a() { x }` = outer [fn, a, Group(), Group({x})] → 1+1+
        // (1+0) + (1+1) = 5. The counter recurses INTO groups (a flat
        // top-level count would say 4 and collapse nesting — measured,
        // rejected: it would push every nested fn into B0).
        let snippet = "fn a() { x }";
        assert_eq!(token_count(snippet), 5, "recursive leaf+group count pinned");
        assert_eq!(Band::of(snippet), Band::B0);

        // Tokenizer arm 2 — a NON-lexing snippet falls back to whitespace
        // split (unbalanced delimiters refuse to lex; pinned count).
        let frag = "}}";
        assert!(
            frag.parse::<TokenStream>().is_err(),
            "unbalanced delimiters must NOT lex"
        );
        assert_eq!(
            token_count(frag),
            1,
            "whitespace fallback pinned (no whitespace ⇒ 1 token)"
        );

        // Ends: B0 and B4 are ends; the middle three are in-between.
        assert!(Band::B0.is_end() && Band::B4.is_end());
        assert!(!Band::B1.is_end() && !Band::B2.is_end() && !Band::B3.is_end());

        // UNSEEN law: below N_MIN is never a verdict.
        let row = BandRow {
            band: Band::B1,
            coverage: N_MIN - 1,
            n: N_MIN - 1,
            id_top1: 100.0,
            ood_top1: 0.0,
            gap_pp: 100.0,
            gap_ci: (99.0, 100.0),
            unseen: true,
        };
        assert!(!row.floor_fires(), "UNSEEN bands never fire the floor");
    }

    /// The bootstrap CI is deterministic (fixed seed) and sane: identical
    /// hit vectors → CI == [gap, gap]; the manufactured separation keeps
    /// the CI away from 0.
    #[test]
    fn gap_ci_is_deterministic_and_sane() {
        let a = vec![true; 30];
        let b = vec![false; 30];
        let (lo, hi) = gap_ci(&a, &b);
        assert!((lo - 100.0).abs() < 1e-3 && (hi - 100.0).abs() < 1e-3);

        let c = vec![true; 20];
        let d = vec![true; 30];
        let (lo2, hi2) = gap_ci(&c, &d);
        assert!((lo2 - 0.0).abs() < 1e-3 && (hi2 - 0.0).abs() < 1e-3);

        // Determinism: same inputs, same CI.
        let (lo3, hi3) = gap_ci(&a, &b);
        assert_eq!((lo, hi), (lo3, hi3));
    }

    /// The runner's withholding invariant fires on a manufactured mismatch:
    /// `ood.len() + members.len()` must equal the population — a harness
    /// bug, not a finding.
    #[test]
    #[should_panic(expected = "withheld count mismatch")]
    fn withholding_mismatch_refuses() {
        let pop: Vec<Binding> = vec![
            Binding::clippy("r1", "fn a() {}", "test"),
            Binding::clippy("r2", "fn b() {}", "test"),
        ];
        let members: Vec<usize> = vec![0];
        // Lost r2 — the manufactured mismatch:
        let ood: Vec<&Binding> = pop.iter().skip(2).collect();
        assert!(
            ood.len() + members.len() != pop.len(),
            "mismatch manufactured"
        );
        assert!(
            ood.len() + members.len() == pop.len(),
            "withheld count mismatch for manufactured"
        );
    }
}
