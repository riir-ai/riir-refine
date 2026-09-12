//! Issue 098 — rule-corpus Cross-OOO axis: the absorption/splitting detector
//! for the rule bank (Research 552 §F2 fusion, the PoC-passed Issue 759
//! primitive now aimed at THIS repo's own corpus).
//!
//! ## The question
//!
//! Do two rules in the corpus trigger on indistinguishable code shapes? That
//! is the SAE-latent absorption/splitting pathology (Chanin et al. 2024)
//! transplanted to rule space: a redundant rule pair silently splits credit,
//! double-counts coverage, and corrupts the per-rule frontier feed the
//! admission gate will consume. `cargo heal --score-bench` measures each rule
//! IN isolation; nothing measures rules AGAINST each other.
//!
//! ## The mapping (all substrate ships)
//!
//! | paper | here |
//! |---|---|
//! | unit (SAE latent) | corpus rule (lint name) |
//! | maximally-activating exemplar | an oracle-labeled triggering span |
//! | activation | binary oracle trigger (`expect_lints` / verified canon) |
//! | exemplar similarity | TF-cosine over [`RustOpBigramTokenizer`] tokens |
//!
//! Rules are UNITS and each rule's triggering spans are its MEI set. With a
//! binary oracle there is no graded activation, so the MEI set is ALL of the
//! rule's triggering spans (no top-K cut — capping would silently drop
//! evidence; the paper's top-K exists to bound cost over huge exemplar
//! pools, which this corpus is not). This makes the bank RAGGED (per-rule
//! span counts differ), so the audit composes the free functions
//! [`ooo_score`] / [`cross_ooo`] directly and hand-builds the
//! [`BankAudit`] that [`greedy_curate`] consumes — `audit_bank_into`'s
//! uniform top-K selection does not apply.
//!
//! ## The similarity metric (the Bench 758 metric-granularity law)
//!
//! Cross-OOO detects redundancy at the similarity metric's concept
//! granularity — the metric DEFINES the concept. The BLAKE3 `ModellessEmbedder`
//! is ruled out by construction: its avalanche property makes distinct inputs
//! near-orthogonal, an S matrix with no structure, and the audit would
//! measure tokenizer noise. [`RustOpBigramTokenizer`] is the metric this repo
//! already ships FOR this corpus's discriminative signal: identifier tokens,
//! punctuation runs, and operator-adjacency bigrams (the tiny-token-crowding
//! fix, Plan 005 T3.5 — the clippy corpus's signal is punctuation), scored
//! **TF-IDF cosine** — IDF is that tokenizer's own stated design law
//! ("ubiquitous runs earn near-zero IDF and contribute nothing; rare ones
//! earn high IDF and do the separating"). A plain-TF control (measured
//! first, see `.benchmarks/085`) lets the generic scaffold (`fn f x op:{`)
//! swamp the rare op signature on micro-span rules and ties every comparison
//! at 0.5 — the metric-granularity law observed live. The concept is
//! therefore **code shape at op-bigram granularity, IDF-weighted**.
//!
//! ## The exemplar pool (population honesty)
//!
//! Only REAL oracle-labeled spans enter the pool:
//!
//! | family | source | labels |
//! |---|---|---|
//! | `v1_frozen` | [`super::fixtures::fixtures`] (36, incl. clean) | frozen real-clippy labels |
//! | `gen6_real_repo` | [`super::generations::generation_v6`] (17) | freeze-e2e real-clippy labels |
//! | `retrieval_fixture` | [`crate::draft::eval_fixtures_clippy::CLIPPY_SPANS`] (50) | one-span-per-lint verified canon |
//!
//! The v2–v5/v7/v8 generation ladders are EXCLUDED (corpus-derived
//! composites — in-distribution by construction; construction bias would
//! trivially dominate the similarity structure). Clean v1 fixtures carry no
//! rule and serve as intruder-only spans. Rules with < 2 spans cannot be
//! audited (Cross-OOO own-similarity is undefined at K = 1) and are reported
//! as unauditable, never silently dropped — the frontier-report law.
//!
//! The non-clippy domains have no measurable population at all: their
//! corpora are exactly one entry per rule (`rust_perf` 126/126,
//! `kernel_opt` 607/607, `sec` 15/15, `docker` 31/31 — measured by the
//! example's population scan), so the axis there is UNMEASURABLE at K ≥ 2,
//! reported as such.
//!
//! ## Verdict law (Issue 098 T3)
//!
//! `cross_ooo(a, b) ≈ 0.5` ⇒ redundant pair (merge candidate); `< 0.6` is
//! the absorption-candidate threshold. If NO pair reads < 0.6 the axis is
//! hygiene-only — a recorded negative, not a failure (the Issue 759
//! defend-wrong precedent).
//!
//! Bit-determinism: the pool is built in sorted-(family, id) order, the
//! tokenizer is pure string scanning, and every reduction is the shared
//! katgpt-core kernel — no ambient state.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use katgpt_core::ooo_audit::{
    cross_ooo, greedy_curate, ooo_score, AuditScratch, BankAudit, CurationResult, OooAuditConfig,
};
use riir_neuron_db::bm25::Tokenizer;
use serde::{Deserialize, Serialize};

use crate::draft::op_tokenizer::RustOpBigramTokenizer;

/// Absorption-candidate threshold: a pair whose Cross-OOO reads below this
/// is a merge candidate (paper: ~0.5 = redundant, 1.0 = distinguishable;
/// the 0.6 cut leaves headroom above the redundancy floor).
pub const REDUNDANT_PAIR_CROSS: f32 = 0.6;

/// One real oracle-labeled span in the audit pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabeledSpan {
    /// Pool family (`v1_frozen` / `gen6_real_repo` / `retrieval_fixture`).
    pub family: &'static str,
    /// Stable fixture id (pool-unique).
    pub id: String,
    /// The rule this span triggers (empty string = clean/intruder-only).
    pub lint: String,
    /// The span text.
    pub code: String,
}

/// The pooled real oracle-labeled clippy spans, sorted by `(family, id)` —
/// the deterministic pool order every downstream reduction inherits.
#[must_use]
pub fn clippy_span_pool() -> Vec<LabeledSpan> {
    fn push(pool: &mut Vec<LabeledSpan>, family: &'static str, id: String, lint: &str, code: &str) {
        // A multi-lint span (k≥2 gen6 spans) joins the pool under its FIRST
        // lint for the audit activation matrix; the audit question is
        // per-rule span-set distinguishability and membership under the
        // first label is the deterministic choice. Clean spans carry the
        // empty lint (intruder-only).
        pool.push(LabeledSpan {
            family,
            id,
            lint: lint.to_string(),
            code: code.to_string(),
        });
    }
    let mut pool: Vec<LabeledSpan> = Vec::with_capacity(103);
    for f in super::fixtures::fixtures() {
        let lint = f.expect_lints.first().copied().unwrap_or("");
        push(&mut pool, "v1_frozen", f.id.to_string(), lint, f.code);
    }
    for f in super::generations::generation_v6() {
        let lint = f.expect_lints.first().copied().unwrap_or("");
        push(&mut pool, "gen6_real_repo", f.id.to_string(), lint, f.code);
    }
    for (lint, code) in crate::draft::eval_fixtures_clippy::CLIPPY_SPANS {
        push(
            &mut pool,
            "retrieval_fixture",
            format!("retrieval-clippy-{lint}"),
            lint,
            code,
        );
    }
    pool.sort_by(|a, b| (&a.family, &a.id).cmp(&(&b.family, &b.id)));
    pool
}

/// TF-IDF cosine similarity over [`RustOpBigramTokenizer`] tokens, written
/// row-major into `out` (`N × N`, symmetric, diagonal exactly 1.0).
///
/// `idf(t) = ln(N / df(t))` over the pool (a term in every span reads 0 —
/// the generic scaffold contributes nothing; the tokenizer's stated design
/// law). Each span is an L2-normalized `tf · idf` vector; cosine = dot over
/// intersecting terms (merge of the sorted term maps). A span whose every
/// token is pool-ubiquitous normalizes to the zero vector and reads 0-similar
/// to everything (diagonal stays 1.0) — reported by the audit as a
/// granularity-floor row, never silently repaired.
pub fn token_cosine_sim_into(pool: &[LabeledSpan], out: &mut Vec<f32>) {
    let n = pool.len();
    // Owned per-span token vectors: the histogram keys borrow the strings, so
    // a shared reusable buffer would fight the borrow checker for no gain at
    // this pool size (100 spans, bench-time only).
    let mut raw_hists: Vec<BTreeMap<String, u32>> = Vec::with_capacity(n);
    for span in pool {
        let mut tok_buf: Vec<String> = Vec::new();
        RustOpBigramTokenizer.tokenize_into(&span.code, &mut tok_buf);
        let mut hist: BTreeMap<String, u32> = BTreeMap::new();
        for t in tok_buf {
            *hist.entry(t).or_insert(0) += 1;
        }
        raw_hists.push(hist);
    }

    // Document frequency over the pool, then idf(t) = ln(N / df(t));
    // pool-ubiquitous terms read exactly 0.
    let mut df: BTreeMap<&str, usize> = BTreeMap::new();
    for hist in &raw_hists {
        for t in hist.keys() {
            *df.entry(t.as_str()).or_insert(0) += 1;
        }
    }
    // idf(t) = ln(N / df(t)); pool-ubiquitous terms read exactly 0. The
    // u32/f32 casts are exact for any realistic corpus size (the report's
    // quantity is a ratio of counts, not an exact integer).
    #[allow(clippy::cast_precision_loss)]
    let idf: BTreeMap<&str, f32> = df
        .iter()
        .map(|(t, d)| (*t, (n as f32 / *d as f32).ln()))
        .collect();

    // L2-normalized tf·idf vectors.
    let mut hists: Vec<BTreeMap<&str, f32>> = Vec::with_capacity(n);
    // tf counts are small — the u32→f32 cast is exact in the range that
    // matters for a term-frequency ratio.
    #[allow(clippy::cast_precision_loss)]
    for hist in &raw_hists {
        let mut weighted: BTreeMap<&str, f32> = hist
            .iter()
            .map(|(t, c)| (t.as_str(), *c as f32 * idf[t.as_str()]))
            .collect();
        let norm: f32 = weighted.values().map(|w| w * w).sum::<f32>().sqrt();
        let scaled = if norm > 0.0 { 1.0 / norm } else { 0.0 };
        for w in weighted.values_mut() {
            *w *= scaled;
        }
        hists.push(weighted);
    }
    out.clear();
    out.resize(n * n, 0.0);
    for i in 0..n {
        out[i * n + i] = 1.0;
        for j in (i + 1)..n {
            // Merge-intersect the sorted maps.
            let mut ai = hists[i].iter().peekable();
            let mut bj = hists[j].iter().peekable();
            let mut dot = 0.0f32;
            while let (Some((ka, va)), Some((kb, vb))) = (ai.peek(), bj.peek()) {
                match ka.cmp(kb) {
                    std::cmp::Ordering::Equal => {
                        dot += **va * **vb;
                        ai.next();
                        bj.next();
                    }
                    std::cmp::Ordering::Less => {
                        ai.next();
                    }
                    std::cmp::Ordering::Greater => {
                        bj.next();
                    }
                }
            }
            out[i * n + j] = dot;
            out[j * n + i] = dot;
        }
    }
}

/// Per-rule audit row.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleAudit {
    /// The audited rule (lint name).
    pub lint: String,
    /// Triggering-span count (the rule's MEI-set size; ≥ 2 for audited rows).
    pub spans: usize,
    /// Asymmetric OOO interpretability (are the rule's spans more similar to
    /// each other than to intruders?).
    pub ooo: f32,
    /// Minimum Cross-OOO against any other audited rule.
    pub min_cross: f32,
    /// The partner rule behind `min_cross`.
    pub min_cross_partner: Option<String>,
}

/// The full cross-OOO axis report for the clippy rule bank.
#[derive(Debug, Clone)]
pub struct CrossOooReport {
    /// Pool size (all spans, intruder-only included).
    pub pooled_spans: usize,
    /// Distinct rules with ≥ 1 span.
    pub rules_total: usize,
    /// Audited rules (≥ 2 spans).
    pub audited_units: usize,
    /// Rules with exactly 1 span — unauditable at K ≥ 2, reported not hidden.
    pub unauditable: Vec<(String, usize)>,
    /// Per-rule rows, sorted by `min_cross` ascending (most-entangled first).
    pub rows: Vec<RuleAudit>,
    /// EVERY audited pair, ascending by Cross-OOO (the full ordering; the
    /// absorption list is the prefix under [`REDUNDANT_PAIR_CROSS`]).
    pub all_pairs: Vec<(String, String, f32)>,
    /// Pairs under [`REDUNDANT_PAIR_CROSS`], ascending (the absorption list).
    pub redundant_pairs: Vec<(String, String, f32)>,
    /// `greedy_curate` kept count — the bank's unique-feature count.
    pub curated_kept: usize,
    /// Rules pruned redundant by curation.
    pub pruned_redundant: Vec<String>,
    /// Rules below `ooo_cutoff` (uninterpretable span sets).
    pub dropped_uninterpretable: Vec<String>,
}

impl CrossOooReport {
    /// True when at least one absorption candidate exists (the T3 wiring
    /// condition — see [`REDUNDANT_PAIR_CROSS`]).
    #[must_use]
    pub fn has_redundant_pairs(&self) -> bool {
        !self.redundant_pairs.is_empty()
    }

    /// The additive score-history summary (the `ood`-style instrumentation
    /// row; instrumentation, never a scored axis).
    #[must_use]
    pub fn summary_row(&self) -> CrossOooRow {
        CrossOooRow {
            pooled_spans: self.pooled_spans,
            rules_total: self.rules_total,
            audited_units: self.audited_units,
            unique_feature_count: self.curated_kept,
            redundant_pairs: self.redundant_pairs.len(),
            min_cross: self.rows.iter().map(|r| r.min_cross).fold(f32::INFINITY, f32::min),
        }
    }
}

/// The additive `score_history.jsonl` field (Issue 098 T4, the
/// `HistoryRecord::ood` precedent: `serde(default)` + skip-when-absent, the
/// SVG renderer and saturation detector ignore it, rows pre-dating the axis
/// deserialize as `None`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossOooRow {
    /// Pool size.
    pub pooled_spans: usize,
    /// Distinct rules in the pool.
    pub rules_total: usize,
    /// Rules audited (≥ 2 spans).
    pub audited_units: usize,
    /// `greedy_curate` kept count (paper Fig. 4 unique features).
    pub unique_feature_count: usize,
    /// Pairs below [`REDUNDANT_PAIR_CROSS`].
    pub redundant_pairs: usize,
    /// Minimum pair Cross-OOO over the audited bank (`INFINITY` serialized
    /// as null when no pair exists — serde's f32 default).
    pub min_cross: f32,
}

/// The process-cached axis measurement (the `ood_axis` precedent: the
/// appended history row and every console consumer share ONE measurement,
/// computed once per `--score-bench` process at default cutoffs).
#[must_use]
pub fn cross_ooo_axis() -> &'static CrossOooReport {
    static AXIS: OnceLock<CrossOooReport> = OnceLock::new();
    AXIS.get_or_init(|| audit_clippy_corpus(&OooAuditConfig::default()))
}

/// Render the cross-OOO section for the FRONTIER report's per-rule rows
/// (Issue 098 T3's additive column, bin-side — `frontier_report::render` is
/// ungated and cannot name this module's types, so the caller appends this
/// section under the feature gate). One line per frontier rule the audit
/// covers: `min_cross` + partner, the absorption annotation. Rules outside
/// the audit (non-clippy domains, 1-span rules) print nothing — the audit
/// population honesty lives in the header line.
#[must_use]
pub fn render_frontier_cross_section(report: &CrossOooReport, rules: &[String]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64 * rules.len().min(32));
    let covered: usize = rules
        .iter()
        .filter(|r| report.rows.iter().any(|row| &row.lint == *r))
        .count();
    let _ = writeln!(
        out,
        "cross-OOO axis (Issue 098 — rule-redundancy, additive instrumentation): {covered}/{} frontier rules audited (clippy pool: {} spans, {} auditable; unique_feature_count = {}; absorption candidates < {REDUNDANT_PAIR_CROSS}: {})",
        rules.len(),
        report.pooled_spans,
        report.audited_units,
        report.curated_kept,
        report.redundant_pairs.len(),
    );
    for r in rules {
        if let Some(row) = report.rows.iter().find(|row| &row.lint == r) {
            let flag = if row.min_cross < REDUNDANT_PAIR_CROSS {
                "ABSORPTION-CANDIDATE"
            } else {
                "ok"
            };
            let _ = writeln!(
                out,
                "  {:<34} min_cross {:>6.3}  vs {}  [{flag}]",
                row.lint,
                row.min_cross,
                row.min_cross_partner.as_deref().unwrap_or("—"),
            );
        }
    }
    out
}

/// Run the ragged-bank cross-OOO audit over a span pool (the audit kernel —
/// `audit_clippy_corpus` is the production pool's thin wrapper).
///
/// Paper-default cutoffs (`0.8 / 0.8`) unless the caller overrides. Pure:
/// builds the pool, the similarity matrix, the per-rule MEI sets, then
/// composes the katgpt-core kernels — deterministic in, bit-identical out.
/// Build the ragged per-rule MEI bank over an audited unit list: per-unit
/// MEI set = ALL its spans, per-unit OOO, symmetric Cross-OOO matrix.
/// (`audit_bank_into`'s uniform top-K selection does not apply to a ragged
/// bank — this composes the free kernels directly.)
fn build_ragged_bank(audited: &[(&str, Vec<u32>)], sim: &[f32], n: usize) -> BankAudit {
    let u = audited.len();
    let max_k = audited.iter().map(|(_, s)| s.len()).max().unwrap_or(0);
    let mut audit = BankAudit::new(u, n, max_k);
    for (unit, (_lint, spans)) in audited.iter().enumerate() {
        audit.meis[unit].clone_from(spans);
        audit.ooo[unit] = ooo_score(spans, sim, n);
    }
    for i in 0..u {
        audit.cross[i * u + i] = 1.0;
        for j in (i + 1)..u {
            let c = cross_ooo(&audit.meis[i], &audit.meis[j], sim, n);
            audit.cross[i * u + j] = c;
            audit.cross[j * u + i] = c;
        }
    }
    audit
}

/// The per-rule rows (min-cross + partner) and the ascending pair list off a
/// finished bank.
fn bank_rows_and_pairs(
    audited: &[(&str, Vec<u32>)],
    audit: &BankAudit,
) -> (Vec<RuleAudit>, Vec<(String, String, f32)>) {
    let u = audited.len();
    let mut rows: Vec<RuleAudit> = Vec::with_capacity(u);
    for (i, (lint, _)) in audited.iter().enumerate() {
        let mut min_cross = f32::INFINITY;
        let mut partner = None;
        for (j, (other, _)) in audited.iter().enumerate() {
            if j != i && audit.cross[i * u + j] < min_cross {
                min_cross = audit.cross[i * u + j];
                partner = Some(other.to_string());
            }
        }
        rows.push(RuleAudit {
            lint: lint.to_string(),
            spans: audit.meis[i].len(),
            ooo: audit.ooo[i],
            min_cross,
            min_cross_partner: partner,
        });
    }
    rows.sort_by(|a, b| {
        a.min_cross
            .partial_cmp(&b.min_cross)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.lint.cmp(&b.lint))
    });

    let mut all_pairs: Vec<(String, String, f32)> = Vec::new();
    for (i, (lint_a, _)) in audited.iter().enumerate() {
        for (j, (lint_b, _)) in audited.iter().enumerate().skip(i + 1) {
            all_pairs.push((lint_a.to_string(), lint_b.to_string(), audit.cross[i * u + j]));
        }
    }
    all_pairs.sort_by(|a, b| {
        a.2.partial_cmp(&b.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    (rows, all_pairs)
}

#[must_use]
pub fn audit_pool(pool: &[LabeledSpan], cfg: &OooAuditConfig) -> CrossOooReport {
    let n = pool.len();

    // Rule table: sorted lint name → span indices (pool order → deterministic).
    // The index cast is bounded by the pool size (u32 MEI indices are the
    // katgpt-core kernel's currency; the pool is hundreds, not millions).
    #[allow(clippy::cast_possible_truncation)]
    let mut by_rule: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    #[allow(clippy::cast_possible_truncation)]
    for (idx, span) in pool.iter().enumerate() {
        if !span.lint.is_empty() {
            by_rule
                .entry(span.lint.as_str())
                .or_default()
                .push(idx as u32);
        }
    }
    let rules_total = by_rule.len();

    // Similarity matrix.
    let mut sim: Vec<f32> = Vec::with_capacity(n * n);
    token_cosine_sim_into(pool, &mut sim);

    // Audited units: rules with ≥ 2 spans, in sorted-lint order.
    let audited: Vec<(&str, Vec<u32>)> = by_rule
        .iter()
        .filter(|(_, spans)| spans.len() >= 2)
        .map(|(lint, spans)| (*lint, spans.clone()))
        .collect();
    let u = audited.len();
    let unauditable: Vec<(String, usize)> = by_rule
        .iter()
        .filter(|(_, spans)| spans.len() < 2)
        .map(|(lint, spans)| ((*lint).to_string(), spans.len()))
        .collect();

    let audit = build_ragged_bank(&audited, &sim, n);

    // Greedy curation over the ragged bank (paper Fig. 4a).
    let mut scratch = AuditScratch::new();
    let mut curation = CurationResult::new(u);
    greedy_curate(&audit, cfg, &mut scratch, &mut curation);
    let lint_of = |unit: usize| audited[unit].0.to_string();
    let pruned_redundant: Vec<String> =
        curation.pruned_redundant.iter().map(|&u| lint_of(u)).collect();
    let dropped_uninterpretable: Vec<String> = curation
        .dropped_uninterpretable
        .iter()
        .map(|&u| lint_of(u))
        .collect();

    // Per-rule rows (most-entangled first) + the ascending pair list.
    let (rows, all_pairs) = bank_rows_and_pairs(&audited, &audit);
    let redundant_pairs: Vec<(String, String, f32)> = all_pairs
        .iter()
        .filter(|(_, _, c)| *c < REDUNDANT_PAIR_CROSS)
        .cloned()
        .collect();

    CrossOooReport {
        pooled_spans: n,
        rules_total,
        audited_units: u,
        unauditable,
        rows,
        all_pairs,
        redundant_pairs,
        curated_kept: curation.unique_feature_count(),
        pruned_redundant,
        dropped_uninterpretable,
    }
}

/// The production audit: the pooled real oracle-labeled clippy spans.
#[must_use]
pub fn audit_clippy_corpus(cfg: &OooAuditConfig) -> CrossOooReport {
    audit_pool(&clippy_span_pool(), cfg)
}

/// Render the report body (the example driver's console face).
#[must_use]
pub fn render_report(report: &CrossOooReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(4096);
    writeln!(
        out,
        "cross-OOO axis — clippy rule bank (Issue 098, Research 552 §F2)"
    )
    .ok();
    writeln!(
        out,
        "pool: {} real oracle-labeled spans · {} rules · {} auditable (>=2 spans) · {} unauditable (1 span)",
        report.pooled_spans, report.rules_total, report.audited_units, report.unauditable.len()
    )
    .ok();
    writeln!(out, "similarity metric: TF-IDF cosine over RustOpBigramTokenizer tokens (code shape at op-bigram granularity, IDF-weighted)").ok();
    writeln!(
        out,
        "greedy_curate(0.8, 0.8): unique_feature_count = {} of {} (pruned redundant: {}, uninterpretable: {})",
        report.curated_kept,
        report.audited_units,
        report.pruned_redundant.len(),
        report.dropped_uninterpretable.len()
    )
    .ok();

    out.push_str("\nper-rule rows (most-entangled first):\n");
    let _ = writeln!(out, "{:<34}{:>6}{:>8}{:>10}  partner", "rule", "spans", "ooo", "min_x");
    for r in &report.rows {
        let _ = writeln!(
            out,
            "{:<34}{:>6}{:>8.3}{:>10.3}  {}",
            r.lint,
            r.spans,
            r.ooo,
            r.min_cross,
            r.min_cross_partner.as_deref().unwrap_or("—")
        );
    }

    out.push_str("\nabsorption candidates (cross < 0.6, ascending):\n");
    if report.redundant_pairs.is_empty() {
        out.push_str("  NONE — the axis is hygiene-only at this pool (Issue 098 T3 negative condition)\n");
    } else {
        for (a, b, c) in &report.redundant_pairs {
            let _ = writeln!(out, "  {a} <-> {b} : {c:.3}");
        }
    }

    out.push_str("\n20 most-redundant pairs overall (ascending, threshold-agnostic):\n");
    for (a, b, c) in report.all_pairs.iter().take(20) {
        let _ = writeln!(out, "  {a} <-> {b} : {c:.3}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use katgpt_core::ooo_audit::OooAuditConfig;

    fn span(family: &'static str, id: &str, lint: &str, code: &str) -> LabeledSpan {
        LabeledSpan {
            family,
            id: id.to_string(),
            lint: lint.to_string(),
            code: code.to_string(),
        }
    }

    #[test]
    fn t1_pool_is_deterministic_and_fully_labeled() {
        let a = clippy_span_pool();
        let b = clippy_span_pool();
        assert_eq!(a, b, "pool build must be bit-deterministic");
        assert!(a.len() >= 100, "expected the 3-family pool, got {}", a.len());
        // Sorted (family, id) order.
        let sorted = a
            .windows(2)
            .all(|w| (&w[0].family, &w[0].id) <= (&w[1].family, &w[1].id));
        assert!(sorted);
        // Every family present.
        for f in ["v1_frozen", "gen6_real_repo", "retrieval_fixture"] {
            assert!(a.iter().any(|s| s.family == f), "family {f} missing");
        }
    }

    #[test]
    fn t1_sim_matrix_symmetric_with_unit_diagonal() {
        let pool = vec![
            span("t", "a", "lint_a", "pub fn a(x: u32) -> u32 { return x * 1; }"),
            span("t", "b", "lint_b", "pub fn b(s: &str) -> String { format!(\"{}\", s) }"),
            span("t", "c", "", "pub fn clean() {}"),
        ];
        let mut sim = Vec::new();
        token_cosine_sim_into(&pool, &mut sim);
        let n = pool.len();
        assert_eq!(sim.len(), n * n);
        for i in 0..n {
            assert_eq!(sim[i * n + i], 1.0);
            for j in 0..n {
                assert_eq!(sim[i * n + j], sim[j * n + i], "symmetry at ({i},{j})");
                assert!((0.0..=1.0).contains(&sim[i * n + j]));
            }
        }
        // The two distinct-shape spans share the ubiquitous scaffold only —
        // IDF zeroes it, so their similarity stays low.
        assert!(sim[0 * n + 1] < 0.5, "distinct spans read {} ", sim[0 * n + 1]);
    }

    #[test]
    fn t1_shared_pool_ubiquitous_terms_contribute_nothing() {
        // Two spans identical except one identifier each — every SHARED token
        // is pool-ubiquitous (df = N) and earns idf = 0, so the similarity
        // rides ONLY the unique identifiers: exactly 0.0 despite ~90%
        // textual identity. The IDF design law, pinned (the tokenizer module's
        // "ubiquitous runs earn near-zero IDF and contribute nothing").
        let pool = vec![
            span("t", "a", "l1", "fn a(x: u32) -> u32 { x * 1 }"),
            span("t", "b", "l1", "fn b(x: u32) -> u32 { x * 1 }"),
        ];
        let mut sim = Vec::new();
        token_cosine_sim_into(&pool, &mut sim);
        assert_eq!(sim[1], 0.0, "ubiquitous-token contributions must vanish");

        // Identical TEXT reads exactly 1.0 — but only when the pool is large
        // enough that the shared tokens are not pool-ubiquitous (in a 2-span
        // pool of identical texts EVERY token is df=N → idf=0 → zero
        // vectors → 0.0; the IDF law cuts both ways and that is the pin).
        let pool2 = vec![
            span("t", "a", "l1", "fn a(x: u32) -> u32 { x * 1 }"),
            span("t", "b", "l2", "fn a(x: u32) -> u32 { x * 1 }"),
            span("t", "c", "l3", "fn z(s: &str) -> String { format!(\"q\") }"),
        ];
        let mut sim2 = Vec::new();
        token_cosine_sim_into(&pool2, &mut sim2);
        assert!(
            (sim2[0 * 3 + 1] - 1.0).abs() < 1e-5,
            "identical text → 1.0 within f32 epsilon, got {}",
            sim2[0 * 3 + 1]
        );
        assert!(sim2[0 * 3 + 2] < 0.5, "distinct shapes stay apart");
    }

    #[test]
    fn t2_planted_duplicate_rule_is_flagged_and_pruned() {
        // Rule A: identity-op-family spans sharing identifier vocabulary
        // (real corpus shape). Rule DUP: the SAME code under a second label
        // — the planted absorption (Chanin: merged latents fire on
        // indistinguishable inputs; identical MEI sets → cross exactly 0.0).
        // Rule C: a distinct shape family.
        let a_codes = [
            "fn scale(v: u32) -> u32 { v * 1 }",
            "fn bump(v: u32) -> u32 { v + 0 }",
            "fn trim(v: u32) -> u32 { v - 0 }",
        ];
        let c_codes = [
            "fn tag(s: &str) -> String { format!(\"x\") }",
            "fn label(s: &str) -> String { format!(\"y\") }",
            "fn name(s: &str) -> String { format!(\"z\") }",
        ];
        let mut pool = Vec::new();
        for (i, code) in a_codes.iter().enumerate() {
            pool.push(span("t", &format!("a{i}"), "rule_a", code));
            // Same TEXT under the dup label — the absorption shape.
            pool.push(span("t", &format!("dup{i}"), "rule_dup", code));
        }
        for (i, code) in c_codes.iter().enumerate() {
            pool.push(span("t", &format!("c{i}"), "rule_c", code));
        }
        let report = audit_pool(&pool, &OooAuditConfig::default());
        assert_eq!(report.audited_units, 3);
        // The planted pair is an absorption candidate at exactly 0.0
        // (identical MEI sets — every own/other comparison ties).
        let pair = report
            .redundant_pairs
            .iter()
            .find(|(a, b, _)| (a == "rule_a" && b == "rule_dup") || (a == "rule_dup" && b == "rule_a"));
        assert_eq!(pair.map(|p| p.2), Some(0.0), "planted duplicate must read exactly 0.0");
        // Fig-4a greedy semantics for a PURE clone pair: the clones sit AT
        // the OOO threshold (own == other), never strictly below — ooo
        // collapses to 0.5 and the ooo filter drops BOTH members before the
        // redundancy pass. A pure clone contributes ZERO interpretable
        // standalone features; only rule_c survives curation. (The realistic
        // prune-redundant path — interpretable members, one pruned — is
        // pinned by the real-pool anchor tests: manual_map 0.462.)
        assert_eq!(report.curated_kept, 1, "kept: {:?}", report.rows);
        assert!(
            report.dropped_uninterpretable.contains(&"rule_a".to_string())
                && report.dropped_uninterpretable.contains(&"rule_dup".to_string()),
            "dropped: {:?}",
            report.dropped_uninterpretable
        );
        // min_cross partner fields are wired both ways.
        let a_row = report.rows.iter().find(|r| r.lint == "rule_a").unwrap();
        assert_eq!(a_row.min_cross, 0.0);
        assert_eq!(a_row.min_cross_partner.as_deref(), Some("rule_dup"));
    }

    #[test]
    fn t2_clippy_audit_is_deterministic() {
        let cfg = OooAuditConfig::default();
        let a = audit_clippy_corpus(&cfg);
        let b = audit_clippy_corpus(&cfg);
        assert_eq!(a.summary_row(), b.summary_row());
        assert_eq!(a.rows, b.rows);
        assert_eq!(a.redundant_pairs, b.redundant_pairs);
    }

    #[test]
    fn t2_single_match_split_pair_is_the_calibration_anchor() {
        // Defend-wrong pin: clippy's own single_match / single_match_else
        // split is a KNOWN redundant pair — the axis must read it under the
        // threshold. If corpus drift moves this number, the audit moved, and
        // the bench doc (.benchmarks/085) must be re-measured.
        let report = audit_clippy_corpus(&OooAuditConfig::default());
        let pair = report
            .all_pairs
            .iter()
            .find(|(a, b, _)| {
                (a == "single_match" && b == "single_match_else")
                    || (a == "single_match_else" && b == "single_match")
            })
            .expect("both split lints must be in the audited pool");
        assert!(
            pair.2 < REDUNDANT_PAIR_CROSS,
            "single_match <-> single_match_else read {}; the calibration anchor broke",
            pair.2
        );
    }
}
