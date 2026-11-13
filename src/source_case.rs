//! Issue 232 T4 — the source-case-first gate (Research 495, L3's law;
//! source: dang2026t3 via `riir-train/.research/495`).
//!
//! **The law:** T³'s L3 found that a candidate step which does not improve
//! on the data that GENERATED it carries no held-out value — zero-length,
//! random, and uphill trial steps all landed in noise. The extraction at
//! the fixer surface: a drafted fix must improve on its source case — the
//! (trigger, rule-exemplar-pair) data that generated it — BEFORE the
//! compile gate spends the "Q half" (the workspace recompile). The compile
//! gate is the expensive half (compile-minutes per verify round); this
//! check is the cheap half (embed arithmetic, zero compile), so a candidate
//! the source case already refuses never costs a round.
//!
//! **Two mechanical decline classes — nothing else (the preregistered
//! shape):**
//!
//! 1. [`DeclineClass::NoOp`] — the fix byte-equals the trigger text it
//!    replaces, or is empty/whitespace (the zero-length step class). A
//!    no-op cannot resolve a finding.
//! 2. [`DeclineClass::Uphill`] — the fix resembles the rule's BUGGY
//!    exemplar more than its FIXED exemplar (cos(fix, buggy) >
//!    cos(fix, fixed), signed, EXACT integer — see below). The candidate
//!    looks like the bug, not the resolution: it does not improve on the
//!    data that generated it.
//!
//! There is deliberately NO distance floor / random-step decline class:
//! T³'s random class exists, but a floor here would be a threshold — and
//! this module ships report-only with the issue's preregistered G2 bar
//! (see below). The random axis is REPORTED as a continuous metric (mean
//! max-cosine over rows); whether a floor earns its place is the wiring
//! cell's question, decided on numbers, never smuggled in as a constant.
//!
//! **Exactness (the Issue-940 family):** the uphill comparison is exact
//! integer arithmetic over the [`span_embed`] i8 vectors — 32 dims × 127²
//! fits i32 per dot/norm, and the cross-multiplied squares fit i64 with
//! ~67× headroom (`516_128²` × `516_128` ≈ 1.4e17 < 9.2e18). Cosines are
//! SIGNED (the feature hashing is signed), so the comparison runs in two
//! steps: the sign quadrant decides first (a positive cosine beats a
//! negative one outright), and the squared cross-multiplication decides
//! within a quadrant — valid because the squaring is monotone on
//! non-negatives, with the direction flipped in the both-negative
//! quadrant. The verdict needs STRICT uphill (ties pass through): the
//! near-identical-exemplar cousin (T2's flip-axis lesson — a one-token
//! fix's exemplars embed almost identically) resolves epsilon-free by
//! exactness, never by a tolerance band. Same inputs → same verdict, every
//! machine, bit for bit — the check itself crosses no sync boundary and
//! has no f64 anywhere.
//!
//! **Report-only (the issue's proof-path ledger):** nothing here gates a
//! fix path. The confusion readout (verdict × lived outcome, weighted by
//! the rows' outcome counts) measures what the gate WOULD have declined
//! against what actually happened; the preregistered G2 bar decides
//! whether a wiring cell opens:
//!
//! > **Preregistered G2 bar** (written into the issue at the T4 claim,
//! > before any measurement ran): on the lived store, the gate declines
//! > ≥ 50% of failed-fix outcomes (decline recall) while declining ≤ 10%
//! > of succeeded-fix outcomes (false-decline rate). A recall miss closes
//! > the gate here with the numbers (the ungoat discipline); a pass opens
//! > the wiring cell — declined candidates skip their verify round
//! > (compile-minutes saved at the fix_verify posture), the gate consults
//! > T3's multiplicity predicate at arming, and T1's ledger carries the
//! > window hygiene.
//!
//! **Outcome semantics caveat (disclosed, not hidden):** the store's
//! `success_count` folds BOTH success classes — oracle-verified lint
//! resolution AND compile-only passes (no oracle feed) — and the row does
//! not carry the split durably. The false-decline rate therefore reads
//! against the store's own outcome law (the wire the fixer uses), not
//! against pure lint-resolution truth; the render says so.
//!
//! ## Feature posture
//!
//! `source_case_gate` (opt-in, default-OFF, no implies — std only; the
//! check and the confusion readout are embed arithmetic over
//! [`crate::span_embed`], ungated like the embedder). The CLI lane
//! `--source-case-report` composes `all(source_case_gate,
//! latent_retrieval)` to read the lived store and the compiled corpora;
//! a build without `source_case_gate` refuses exit 2 (the corpus-gap
//! shape). Feature-off is byte-identical by construction (new gated code
//! only, no shared-path edit).

use crate::span_embed::{SPAN_EMBED_DIMS, span_embed};

/// The readout's schema version — carried in the rendered header, so a law
/// change moves every consumer loud.
pub const READOUT_VERSION: u64 = 1;

/// Why the source case refuses a candidate. Mechanical classes only —
/// no thresholds, no floors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclineClass {
    /// The fix byte-equals the trigger it replaces, or is empty — the
    /// zero-length step class. A no-op cannot resolve a finding.
    NoOp,
    /// cos(fix, buggy) > cos(fix, fixed) — signed, exact integer. The
    /// candidate resembles the bug more than the resolution.
    Uphill,
}

/// The source-case verdict for one (trigger, fix, exemplar-pair) check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCaseVerdict {
    /// The source case does not refuse the candidate — the compile gate
    /// may spend its round on it (at the wiring posture; this module
    /// never wires anything).
    Improves,
    /// The source case refuses the candidate — the compile gate would be
    /// a wasted round.
    Declined(DeclineClass),
}

/// One dot product over the quantized embeddings, exact in i32 (32 dims
/// × 127² = `516_128` max).
fn dot(a: &[i8; SPAN_EMBED_DIMS], b: &[i8; SPAN_EMBED_DIMS]) -> i64 {
    let mut acc: i64 = 0;
    for i in 0..SPAN_EMBED_DIMS {
        acc += i64::from(i32::from(a[i])) * i64::from(i32::from(b[i]));
    }
    acc
}

/// One squared norm, exact (≤ `516_128`).
fn norm2(a: &[i8; SPAN_EMBED_DIMS]) -> i64 {
    dot(a, a)
}

/// The signed uphill comparison, exact integer: does
/// `cos(f, b) > cos(f, c)` hold? Cosines are `dot/√norm2` with signed
/// numerators and non-negative denominators, so:
///
/// - opposite signs: the positive side wins outright (a positive cosine
///   beats a negative one; zero beats a negative);
/// - same (non-negative) sign: square the cross-multiplication —
///   `fb²·nc > fc²·nb` — monotone on non-negatives;
/// - same (non-positive) sign: the flip — `fb²·nc < fc²·nb`.
///
/// i64 headroom: |fb| ≤ `516_128`, squared ≈ 2.7e11, × nc ≤ `516_128` ≈
/// 1.4e17 — fits with ~67× margin. Exact: no epsilon, no tolerance band;
/// the strict-`>` tie rule passes near-identical exemplar pairs through
/// (T2's flip-axis cousin).
#[must_use]
fn uphill(f: &[i8; SPAN_EMBED_DIMS], b: &[i8; SPAN_EMBED_DIMS], c: &[i8; SPAN_EMBED_DIMS]) -> bool {
    use std::cmp::Ordering::{Equal, Greater, Less};
    let fb = dot(f, b);
    let fc = dot(f, c);
    match (fb.cmp(&0), fc.cmp(&0)) {
        // cos(f,b) > 0 ≥ cos(f,c): the bug side is strictly more similar.
        (Greater, Less | Equal) => true,
        // cos(f,b) ≤ 0 < cos(f,c) — or both zero (the tie): not uphill.
        (Less | Equal, Greater | Equal) | (Equal, Less) => false,
        // Both positive: squaring is monotone on non-negatives.
        (Greater, Greater) => fb * fb * norm2(c) > fc * fc * norm2(b),
        // Both negative: the comparison flips (|fb| smaller = closer to 0
        // = larger cosine).
        (Less, Less) => fb * fb * norm2(c) < fc * fc * norm2(b),
    }
}

/// The source-case check for one drafted fix. Pure, exact, allocation-free
/// apart from the three embeds (stack arrays). Same inputs → the same
/// verdict on every machine.
///
/// - `fix.trim()` empty or `fix == shape` (byte-exact) → [`DeclineClass::NoOp`];
/// - else the signed exact uphill comparison against the rule's exemplars →
///   [`DeclineClass::Uphill`] on strict uphill, [`SourceCaseVerdict::Improves`]
///   otherwise (ties pass — decline needs evidence).
#[must_use]
pub fn source_case_check(shape: &str, fix: &str, buggy: &str, fixed: &str) -> SourceCaseVerdict {
    if fix.trim().is_empty() || fix == shape {
        return SourceCaseVerdict::Declined(DeclineClass::NoOp);
    }
    let f = span_embed(fix);
    let b = span_embed(buggy);
    let c = span_embed(fixed);
    if uphill(&f, &b, &c) {
        SourceCaseVerdict::Declined(DeclineClass::Uphill)
    } else {
        SourceCaseVerdict::Improves
    }
}

/// One lived row for the confusion readout: the trigger/fix pair + the
/// rule it names + its aggregated outcome counts (the store's row grain —
/// one row per (rule, shape) pair; the verdict is constant per row, the
/// outcomes are counts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivedRow {
    pub rule_id: String,
    pub shape: String,
    pub fix: String,
    pub success_count: u32,
    pub fail_count: u32,
}

/// The exemplar pair for one rule id (the corpus's own source case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExemplarPair {
    pub rule_id: String,
    pub buggy: String,
    pub fixed: String,
}

/// The confusion readout: what the gate WOULD have declined on the lived
/// store, against what actually happened. Verdicts are per row; outcome
/// cells are weighted by the row's own outcome counts (a declined row with
/// 2 wins + 8 fails contributes 2 false-decline + 8 true-decline
/// evidence). Rows are consumed in the CALLER's order (the store's
/// canonical key order) — the mean max-cosine is a sequential fold in
/// exactly that order (the Issue-940 determinism law); the confusion
/// COUNTS are order-independent integers.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfusionReadout {
    pub readout_version: u64,
    /// Rows consumed (non-zero-outcome, non-empty shape/fix).
    pub rows: usize,
    /// Rows skipped: no exemplar pair for the rule (the corpus lacks it —
    /// disclosed, never folded into a verdict).
    pub skipped_no_rule: usize,
    /// Rows skipped: zero recorded outcomes (seed/pending — T3's
    /// disclosure law).
    pub skipped_zero_outcome: usize,
    /// Wins on Improves / fails on Improves.
    pub improves_win: u64,
    pub improves_fail: u64,
    /// Wins / fails on `NoOp` declines.
    pub noop_win: u64,
    pub noop_fail: u64,
    /// Wins / fails on Uphill declines.
    pub uphill_win: u64,
    pub uphill_fail: u64,
    /// Mean over consumed rows of max(cos(fix,buggy), cos(fix,fixed)) —
    /// the RANDOM axis, continuous, NO threshold. Signed cosines; the
    /// max is the signed max (the more-similar side). Sequential f32 fold
    /// in the caller's row order; the display cast carries the deliberate
    /// allow (issue-940), never a silence.
    pub mean_max_cos: f32,
    /// Total wins / fails across consumed rows (the rate denominators).
    pub total_win: u64,
    pub total_fail: u64,
}

impl ConfusionReadout {
    /// Declined fails / all fails — the decline recall axis.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // a report figure
    pub fn decline_recall(&self) -> f32 {
        let declined_fail = self.noop_fail + self.uphill_fail;
        if self.total_fail == 0 {
            return 0.0;
        }
        declined_fail as f32 / self.total_fail as f32
    }

    /// Declined wins / all wins — the false-decline axis.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // a report figure
    pub fn false_decline_rate(&self) -> f32 {
        let declined_win = self.noop_win + self.uphill_win;
        if self.total_win == 0 {
            return 0.0;
        }
        declined_win as f32 / self.total_win as f32
    }

    /// The preregistered G2 bar's verdict on this readout. `None` = the
    /// denominators are too thin to read (no fails, or no wins — a rate
    /// over zero evidence is UNDECIDABLE, never a pass).
    #[must_use]
    pub fn bar_verdict(&self) -> Option<&'static str> {
        if self.total_fail == 0 || self.total_win == 0 {
            return None;
        }
        let recall = self.decline_recall();
        let false_decline = self.false_decline_rate();
        Some(if recall >= 0.50 && false_decline <= 0.10 {
            "PASS"
        } else {
            "MISS"
        })
    }
}

/// Run the confusion readout over lived rows + the exemplar-pair table.
/// Rows without a pair are disclosed (`skipped_no_rule`), never folded.
#[must_use]
pub fn confusion_readout(
    rows: &[LivedRow],
    pairs: &[(String, String, String)],
) -> ConfusionReadout {
    let mut out = ConfusionReadout {
        readout_version: READOUT_VERSION,
        rows: 0,
        skipped_no_rule: 0,
        skipped_zero_outcome: 0,
        improves_win: 0,
        improves_fail: 0,
        noop_win: 0,
        noop_fail: 0,
        uphill_win: 0,
        uphill_fail: 0,
        mean_max_cos: 0.0,
        total_win: 0,
        total_fail: 0,
    };
    for row in rows {
        if row.success_count + row.fail_count == 0 {
            out.skipped_zero_outcome += 1;
            continue;
        }
        if row.shape.is_empty() || row.fix.is_empty() {
            // A row whose halves were never captured cannot be checked —
            // disclosed, never folded into a verdict.
            out.skipped_no_rule += 1;
            continue;
        }
        let Some((_, buggy, fixed)) = pairs.iter().find(|(id, _, _)| *id == row.rule_id) else {
            out.skipped_no_rule += 1;
            continue;
        };
        let verdict = source_case_check(&row.shape, &row.fix, buggy, fixed);
        // The random axis: the signed max COSINE — normalized by BOTH
        // norms (the fix's own norm is the same factor for both sides, so
        // it never affects the max's ordering, but the VALUE must stay in
        // [-1, 1] to be the cosine it claims to be). Sequential f32 fold
        // in the caller's order (the store's canonical key order at the
        // CLI).
        let f = span_embed(&row.fix);
        let b = span_embed(buggy);
        let c = span_embed(fixed);
        let (fb, fc) = (dot(&f, &b), dot(&f, &c));
        let (nf, nb, nc) = (norm2(&f), norm2(&b), norm2(&c));
        let cos = |d: i64, n: i64| -> f32 {
            if n == 0 || nf == 0 {
                0.0
            } else {
                #[allow(clippy::cast_precision_loss)]
                let q = (d as f32) / ((nf as f32).sqrt() * (n as f32).sqrt());
                q
            }
        };
        let max_cos = cos(fb, nb).max(cos(fc, nc));
        out.mean_max_cos += max_cos;
        out.rows += 1;
        out.total_win += u64::from(row.success_count);
        out.total_fail += u64::from(row.fail_count);
        match verdict {
            SourceCaseVerdict::Improves => {
                out.improves_win += u64::from(row.success_count);
                out.improves_fail += u64::from(row.fail_count);
            }
            SourceCaseVerdict::Declined(DeclineClass::NoOp) => {
                out.noop_win += u64::from(row.success_count);
                out.noop_fail += u64::from(row.fail_count);
            }
            SourceCaseVerdict::Declined(DeclineClass::Uphill) => {
                out.uphill_win += u64::from(row.success_count);
                out.uphill_fail += u64::from(row.fail_count);
            }
        }
    }
    if out.rows > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            out.mean_max_cos /= out.rows as f32;
        }
    }
    out
}

/// The readout render (the `--source-case-report` body). States the
/// preregistered bar beside the numbers, the outcome-semantics caveat,
/// and the wiring plan — and carries NO refusal vocabulary (the T2
/// discipline: a REPORT never refuses; the blindness refusals are the
/// CLI's stderr lines).
#[must_use]
#[allow(clippy::cast_precision_loss)] // report figures
pub fn render(rep: &ConfusionReadout) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "source-case readout (i232 T4 — report-only; L3's law: a candidate that does not \
         improve on the data that generated it carries no held-out value) v{READOUT_VERSION}\n\
         rows: {} consumed (win {} / fail {}) — skipped: no-rule {}, zero-outcome {}\n\
         verdict x outcome (weighted by the rows' outcome counts):\n\
         \x20 Improves: win {} / fail {}\n\
         \x20 Declined(NoOp): win {} / fail {}\n\
         \x20 Declined(Uphill): win {} / fail {}",
        rep.rows,
        rep.total_win,
        rep.total_fail,
        rep.skipped_no_rule,
        rep.skipped_zero_outcome,
        rep.improves_win,
        rep.improves_fail,
        rep.noop_win,
        rep.noop_fail,
        rep.uphill_win,
        rep.uphill_fail
    );
    let bar = rep
        .bar_verdict()
        .unwrap_or("UNDECIDABLE (thin denominators)");
    let _ = writeln!(
        out,
        "decline recall on failed fixes: {:.1}% (preregistered G2 bar: >= 50%)\n\
         false-decline rate on succeeded fixes: {:.1}% (preregistered G2 bar: <= 10%)\n\
         bar verdict: {bar}\n\
         mean max-cos (the random axis — continuous, no threshold): {:.4}\n\
         caveat: the store's win class folds oracle-verified + compile-only passes (no durable \
         split) — the rates read against the fixer's own outcome law\n\
         wiring plan: declined candidates skip their verify round (compile-minutes saved); \
         the gate consults T3's multiplicity predicate at arming; no wiring exists — \
         this lane reports only",
        rep.decline_recall() * 100.0,
        rep.false_decline_rate() * 100.0,
        rep.mean_max_cos
    );
    out
}

#[cfg(feature = "latent_retrieval")]
mod store_surface {
    //! The lived-surface feeding half: the store's trajectory rows +
    //! the compiled corpora's exemplar pairs. Composes
    //! `all(source_case_gate, latent_retrieval)` — the gate module itself
    //! stays std-only (the crate builds at every posture).

    use super::{ExemplarPair, LivedRow, confusion_readout, render};
    use crate::domain::HealEntry;
    use std::path::Path;

    /// The surface name the CLI lane renders.
    pub const SURFACE_NAME: &str = "traj-store";

    /// Gather the compiled corpora's exemplar pairs (T2's per-domain
    /// pattern — every cfg block's binding lives at fn scope so the
    /// `&dyn HealEntry` refs outlive it; the coercion to `&dyn` happens
    /// at push time, mirroring T2's `cli_report`).
    #[must_use]
    pub fn compiled_pairs() -> Vec<ExemplarPair> {
        let mut all: Vec<&dyn HealEntry> = Vec::new();
        // Each corpus binding lives at fn scope so the `&dyn HealEntry`
        // refs in `all` outlive every block (borrowck: a cfg block would
        // drop its binding while `all` still borrows it — T2's shape).
        #[cfg(feature = "clippy_lints")]
        let c_clippy = crate::build_seed_corpus();
        #[cfg(feature = "clippy_lints")]
        for e in &c_clippy.entries {
            all.push(e);
        }
        #[cfg(feature = "rust_perf")]
        let c_perf = crate::build_perf_seed_corpus();
        #[cfg(feature = "rust_perf")]
        for e in &c_perf.entries {
            all.push(e);
        }
        #[cfg(feature = "kernel_opt")]
        let c_kernel = crate::build_kernel_seed_corpus();
        #[cfg(feature = "kernel_opt")]
        for e in &c_kernel.entries {
            all.push(e);
        }
        #[cfg(feature = "sec")]
        let c_sec = crate::build_sec_seed_corpus();
        #[cfg(feature = "sec")]
        for e in &c_sec.entries {
            all.push(e);
        }
        #[cfg(feature = "docker")]
        let c_docker = crate::build_docker_seed_corpus();
        #[cfg(feature = "docker")]
        for e in &c_docker.entries {
            all.push(e);
        }
        #[cfg(feature = "dist")]
        let c_dist = crate::build_dist_seed_corpus();
        #[cfg(feature = "dist")]
        for e in &c_dist.entries {
            all.push(e);
        }
        all.into_iter()
            .map(|e| ExemplarPair {
                rule_id: e.rule_id().to_string(),
                buggy: e.buggy_code().to_string(),
                fixed: e.fixed_code().to_string(),
            })
            .collect()
    }

    /// The full CLI readout over the store at `root`: fold rows, join the
    /// compiled pairs, run the confusion readout, render. Blind shapes
    /// (no compiled pairs / no consumable rows) return `Err` — never a
    /// green zero.
    ///
    /// # Errors
    ///
    /// The store open failure; the blind refusals (no pairs, no rows).
    pub fn cli_readout(root: &Path) -> Result<String, String> {
        let pairs = compiled_pairs();
        if pairs.is_empty() {
            return Err(
                "source-case-report: no healing domain compiled into this build — rebuild with at least one domain feature"
                    .to_string(),
            );
        }
        let (store, bad) = crate::traj_store::TrajStore::open(root)?;
        if bad > 0 {
            return Err(format!(
                "source-case-report: {bad} corrupt trajectory row(s) — the surface record is \
                 incomplete; refusing a partial measure"
            ));
        }
        let rows: Vec<LivedRow> = store
            .dump()
            .into_iter()
            .map(|r| LivedRow {
                rule_id: r.lint_key,
                shape: r.shape,
                fix: r.fix,
                success_count: r.success_count,
                fail_count: r.fail_count,
            })
            .collect();
        if rows.iter().all(|r| r.success_count + r.fail_count == 0) {
            return Err(
                "source-case-report: no outcome-bearing rows in the store — nothing to read"
                    .to_string(),
            );
        }
        let pair_tuples: Vec<(String, String, String)> = pairs
            .into_iter()
            .map(|p| (p.rule_id, p.buggy, p.fixed))
            .collect();
        let rep = confusion_readout(&rows, &pair_tuples);
        // The mut is consumed only at the ledger posture (the hygiene
        // line's push below); the cfg_attr is the main() precedent.
        #[cfg_attr(not(feature = "consumed_window_ledger"), allow(unused_mut))]
        let mut text = render(&rep);
        // The T1 window-hygiene line: consult the ledger read-only (the
        // report is a read, never a selection — nothing is saved).
        #[cfg(feature = "consumed_window_ledger")]
        {
            let digest = store.consumed_window_digest();
            let ledger_path = root.join(".refine").join("window_ledger.json");
            let mut ledger = crate::consumed_window::WindowLedger::load(&ledger_path);
            let epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let read = ledger.consult(&digest, "source-case-report", epoch);
            // READ-ONLY consult: render the line, then drop WITHOUT saving
            // — the report never mutates the hygiene state.
            let line = ledger.report_line(&digest, &read);
            drop(ledger);
            text.push_str(&line);
            text.push('\n');
        }
        Ok(text)
    }
}

#[cfg(feature = "latent_retrieval")]
pub use store_surface::{SURFACE_NAME, cli_readout, compiled_pairs};

#[cfg(all(test, feature = "source_case_gate"))]
mod tests {
    use super::*;

    const BUGGY: &str = "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }";
    const FIXED: &str =
        "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() }";

    #[test]
    fn noop_classes_decline() {
        // Byte-equal fix: the zero-length step.
        assert_eq!(
            source_case_check(BUGGY, BUGGY, BUGGY, FIXED),
            SourceCaseVerdict::Declined(DeclineClass::NoOp)
        );
        // Empty / whitespace-only fix: cannot resolve anything.
        assert_eq!(
            source_case_check(BUGGY, "", BUGGY, FIXED),
            SourceCaseVerdict::Declined(DeclineClass::NoOp)
        );
        assert_eq!(
            source_case_check(BUGGY, "   \n\t ", BUGGY, FIXED),
            SourceCaseVerdict::Declined(DeclineClass::NoOp)
        );
    }

    #[test]
    fn the_rules_own_resolution_improves_and_its_own_bug_is_uphill() {
        // Engagement: the fixed exemplar as the fix IMPROVES (it is the
        // resolution, exactly).
        assert_eq!(
            source_case_check(BUGGY, FIXED, BUGGY, FIXED),
            SourceCaseVerdict::Improves
        );
        // The buggy exemplar as the fix is the uphill class (the bug
        // re-inserted).
        assert_eq!(
            source_case_check(BUGGY, BUGGY, BUGGY, FIXED),
            SourceCaseVerdict::Declined(DeclineClass::NoOp) // byte-equal fires NoOp first
        );
        // A NEAR-copy of the buggy exemplar (whitespace shifted — not
        // byte-equal) must still land Uphill via the embed space.
        let near_bug = BUGGY.replace("out.push(page)", "out.push(page) ");
        assert_eq!(
            source_case_check(BUGGY, &near_bug, BUGGY, FIXED),
            SourceCaseVerdict::Declined(DeclineClass::Uphill)
        );
    }

    #[test]
    fn uphill_is_signed_and_exact() {
        // A fix anti-correlated with the resolution but correlated with
        // the bug: the sign quadrant must decide, without any squaring
        // ambiguity. Construct via a totally unrelated fix — both
        // cosines near zero, their SIGNS decide.
        let unrelated = "fn zz(q8: u128) -> bool { q8 == 0 }";
        // Determinism first: same inputs, same verdict.
        let v1 = source_case_check(BUGGY, unrelated, BUGGY, FIXED);
        let v2 = source_case_check(BUGGY, unrelated, BUGGY, FIXED);
        assert_eq!(v1, v2);
        // Near-identical exemplars (the flip-axis cousin): the pairs
        // cannot discriminate, and the strict tie rule passes through —
        // no tolerance band, no epsilon.
        let v = source_case_check(BUGGY, FIXED, BUGGY, BUGGY);
        // f = fixed, b = buggy, c = buggy: fb == fc exactly → not uphill.
        assert_eq!(v, SourceCaseVerdict::Improves);
    }

    #[test]
    fn confusion_readout_weights_by_outcome_counts() {
        let rows = vec![
            // Improves: 3 wins / 1 fail
            LivedRow {
                rule_id: "r_a".into(),
                shape: BUGGY.into(),
                fix: FIXED.into(),
                success_count: 3,
                fail_count: 1,
            },
            // Uphill (near-bug fix): 2 wins / 8 fails
            LivedRow {
                rule_id: "r_b".into(),
                shape: "let mut v = Vec::new(); for x in xs { v.push(x); } v".into(),
                fix: "let mut v = Vec::new(); for x in xs { v.push(x); } v ".into(),
                success_count: 2,
                fail_count: 8,
            },
            // No rule for this one — disclosed, never folded.
            LivedRow {
                rule_id: "r_ghost".into(),
                shape: "a".into(),
                fix: "b".into(),
                success_count: 1,
                fail_count: 0,
            },
            // Zero outcomes — skipped (T3's disclosure law).
            LivedRow {
                rule_id: "r_a".into(),
                shape: "c".into(),
                fix: "d".into(),
                success_count: 0,
                fail_count: 0,
            },
        ];
        let pairs = vec![
            ("r_a".to_string(), BUGGY.to_string(), FIXED.to_string()),
            (
                "r_b".to_string(),
                "let mut v = Vec::new(); for x in xs { v.push(x); } v".to_string(),
                "let v: Vec<_> = xs.into_iter().collect(); v".to_string(),
            ),
        ];
        let rep = confusion_readout(&rows, &pairs);
        assert_eq!(rep.rows, 2);
        assert_eq!(rep.skipped_no_rule, 1);
        assert_eq!(rep.skipped_zero_outcome, 1);
        assert_eq!(rep.improves_win, 3);
        assert_eq!(rep.improves_fail, 1);
        // The near-bug fix embeds closest to the buggy exemplar — the
        // r_b row declines Uphill and its 2/8 counts land there.
        assert_eq!(rep.uphill_win, 2);
        assert_eq!(rep.uphill_fail, 8);
        assert_eq!(rep.total_win, 5);
        assert_eq!(rep.total_fail, 9);
        // Rates: recall 8/9, false-decline 2/5.
        assert!((rep.decline_recall() - 8.0 / 9.0).abs() < 1e-6);
        assert!((rep.false_decline_rate() - 2.0 / 5.0).abs() < 1e-6);
        // The bar: recall 88.9% ≥ 50 ✓ but false-decline 40% > 10 ✗ → MISS.
        assert_eq!(rep.bar_verdict(), Some("MISS"));
    }

    #[test]
    fn bar_verdict_is_undecidable_on_thin_denominators() {
        let mut rep = ConfusionReadout {
            readout_version: READOUT_VERSION,
            rows: 1,
            skipped_no_rule: 0,
            skipped_zero_outcome: 0,
            improves_win: 5,
            improves_fail: 0,
            noop_win: 0,
            noop_fail: 0,
            uphill_win: 0,
            uphill_fail: 0,
            mean_max_cos: 0.5,
            total_win: 5,
            total_fail: 0,
        };
        assert_eq!(rep.bar_verdict(), None);
        // Both legs must hold for PASS: recall (declined fails / all
        // fails) ≥ 50% AND false-decline (declined wins / all wins) ≤
        // 10%. A thin-fails shape with zero declined fails misses on
        // recall first.
        rep.total_fail = 4;
        assert_eq!(rep.bar_verdict(), Some("MISS"), "recall 0/4 = 0% misses");
        rep.noop_fail = 2;
        rep.noop_win = 1;
        rep.total_win = 19;
        rep.improves_win = 18;
        // recall 2/4 = 50% ✓, false-decline 1/19 = 5.3% ✓ → PASS.
        assert_eq!(rep.bar_verdict(), Some("PASS"));
    }

    #[test]
    fn render_carries_no_refusal_vocabulary_and_names_the_bar() {
        let rep = confusion_readout(
            &[LivedRow {
                rule_id: "r_a".into(),
                shape: BUGGY.into(),
                fix: FIXED.into(),
                success_count: 4,
                fail_count: 2,
            }],
            &[("r_a".to_string(), BUGGY.to_string(), FIXED.to_string())],
        );
        let text = render(&rep);
        let low = text.to_ascii_lowercase();
        for w in ["refuse", "reject", "deny", "refusal"] {
            assert!(!low.contains(w), "the report must never say {w}");
        }
        assert!(text.contains("preregistered G2 bar"));
        assert!(text.contains("no wiring exists"));
        assert!(text.contains("verdict x outcome"));
    }

    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn store_surface_reads_the_lived_rows() {
        let root = std::env::temp_dir().join(format!("source_case_g1_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp root");
        // Seed: r_a IMPROVES 18W/2F (the fixed exemplar as the fix), r_b
        // UPHILL 1W/9F (a near-bug fix), r_ghost has no pair (skipped).
        let (store, bad) = crate::traj_store::TrajStore::open(&root).expect("open store");
        assert_eq!(bad, 0);
        let fixed_shaped = "let v: Vec<_> = xs.into_iter().collect(); v";
        for k in 0..20 {
            let win = !(k == 5 || k == 15);
            store
                .record("r_a", BUGGY, FIXED, win, win, "g1-seed")
                .expect("record");
        }
        let near_bug = "let mut v = Vec::new(); for x in xs { v.push(x); } v ";
        for k in 0..10 {
            let win = k < 1;
            store
                .record(
                    "r_b",
                    "let mut v = Vec::new(); for x in xs { v.push(x); } v",
                    near_bug,
                    win,
                    win,
                    "g1-seed",
                )
                .expect("record");
        }
        store
            .record("r_ghost", "a", "b", true, true, "g1-seed")
            .expect("record");
        drop(store);
        // The compiled_pairs() path is posture-dependent; drive the readout
        // directly with explicit pairs (the CLI's corpus join is the
        // composed posture's business — the fixture pins the arithmetic).
        let (store2, _) = crate::traj_store::TrajStore::open(&root).expect("reopen");
        let rows: Vec<LivedRow> = store2
            .dump()
            .into_iter()
            .map(|r| LivedRow {
                rule_id: r.lint_key,
                shape: r.shape,
                fix: r.fix,
                success_count: r.success_count,
                fail_count: r.fail_count,
            })
            .collect();
        let pairs = vec![
            ("r_a".to_string(), BUGGY.to_string(), FIXED.to_string()),
            (
                "r_b".to_string(),
                "let mut v = Vec::new(); for x in xs { v.push(x); } v".to_string(),
                fixed_shaped.to_string(),
            ),
        ];
        let pair_tuples: Vec<(String, String, String)> = pairs;
        let rep = confusion_readout(&rows, &pair_tuples);
        assert_eq!(rep.rows, 2, "r_a + r_b consumed; r_ghost skipped no-rule");
        assert_eq!(rep.skipped_no_rule, 1);
        // r_a: 18 wins / 2 fails on Improves (the deliberate seeding
        // above — a deterministic pattern, never 3 consecutive fails).
        assert_eq!(rep.improves_win, 18);
        assert_eq!(rep.improves_fail, 2);
        // r_b: 1 win / 9 fails on Uphill.
        assert_eq!(rep.uphill_win, 1);
        assert_eq!(rep.uphill_fail, 9);
        let _ = std::fs::remove_dir_all(&root);
    }
}
