//! Batch-intake diffusion-diversity readout (Issue 222 T1 — REPORT-ONLY).
//!
//! The mined-batch intake's shape spread, measured with katgpt-rs Plan 634
//! T1.4's `Div_P^τ` primitive (arXiv:2606.01861 Chen & Li): per batch, the
//! kNN graph over the batch entries' rule embeddings (`SimGraph::build`),
//! `div_tau` with a uniform simplex μ. High Div = the batch spread across
//! shape-space; low Div = the paper's cluster-local curriculum failure —
//! many rules, one fundamental idea. Per-item dedup cannot see that case
//! (pairwise-distinct rules can all diffuse into one cluster); this scalar
//! can.
//!
//! Surfaces:
//! - [`report_all`] — per domain: the per-batch rows, the untagged count,
//!   and a corpus-level Div (μ uniform over the domain's whole corpus —
//!   the corpus spread scalar, the T2 correlation cell's context axis).
//! - [`print_console`] — the score-bench tail line (wired in the bin's
//!   `finish_score_bench`, beside the provenance-strata cluster); one
//!   summary line per domain, the full per-batch table only under
//!   `RIIR_REFINE_BATCH_DIV_FULL=1` (every score-bench run re-derives the
//!   same corpus-static scalars, so only the summaries print by default —
//!   persisting them per run would duplicate the landing-time evidence).
//!
//! Conventions (pinned):
//! - **Batch tag**: the LAST `Batch <digits>` occurrence of the entry's
//!   own text fields (the citation-tail convention lands the entry's own
//!   batch at the end: `Batch 345 / Plan 357 / Research 389.`). An entry
//!   with no tag counts in `untagged`, never folded into a batch.
//! - **Embedding text**: exactly what the retrieval index embeds — the
//!   `HealEntry::buggy_code()` pattern text (the `resolve_corpus` convention
//!   in the draft matcher), so the readout shares the shipped embedding
//!   space. `embed_modelless` returns L2-normalized vectors, so cosine =
//!   the plain ascending-index dot.
//! - **Edges**: kNN with `k = min(DEFAULT_KNN_K, n-1)`, weight = cosine
//!   clipped to `(0, 1]` (non-negative similarities only — the `SimGraph`
//!   contract), deduped to one undirected edge keeping the max weight.
//!   Ties break on ascending neighbor index (Issue-940 discipline); the
//!   graph fold order is pinned inside `SimGraph`/`div_tau`.
//! - **Div semantics**: μ = uniform simplex (every entry contributes
//!   equally); τ = [`DEFAULT_TAU`]. `div_tau` returns the honest 0.0 for
//!   a single-entry batch (no graph to diffuse on — "no spread
//!   measurable", not a perfect score). The scale anchor is the μ=π
//!   identity: a batch spread across the graph reads ≈ 1.0 (the diffused
//!   mass approaches the stationary measure); concentrated intake reads
//!   well below (the katgpt-core fold fix `24aa43051` — found BY this
//!   readout reading ~2.0 on spread batches — restored the documented
//!   normalization; upstream spec pins: katgpt-core
//!   `plan634_t14_div_normalization_spec`). Measured disclosure at the
//!   T1 landing: real mined batches all read 0.98–1.0 — the 8-D hashed
//!   embedding separates within-batch spread weakly at batch sizes
//!   n ≤ 12; the T2 correlation cell owns the discrimination question.
//!
//! Feature-off byte-identity: the whole module is `cfg(feature =
//! "batch_diversity")` and the console call site carries the same cfg — a
//! build without the feature compiles none of this and prints nothing.

use std::collections::BTreeMap;

use katgpt_core::diversity::diffusion::SimGraph;

use crate::domain::{HealDomain, HealEntry};
use crate::ragplane::embedder::{Embedder, ModellessEmbedder};
use crate::ragplane::query::LATENT_DIM;

/// Diffusion steps for the readout (`Div_P^τ`). One hop: a near-duplicate
/// batch keeps its diffused mass local (low Div), a spread batch pushes
/// mass into distinct neighborhoods (high Div). The T2 correlation cell
/// may sweep τ; the console pin stays 1 for comparability across runs.
pub const DEFAULT_TAU: usize = 1;

/// kNN neighborhood size, adapted down to `n - 1` per batch.
pub const DEFAULT_KNN_K: usize = 5;

/// One mined batch's intake row.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchDivRow {
    /// Domain label (`kernel_opt`, `rust_perf`).
    pub domain: &'static str,
    /// The parsed `Batch <digits>` tag.
    pub batch: u32,
    /// Intake entries in the batch.
    pub n: usize,
    /// Undirected kNN edges after dedup (0 when `n < 2`).
    pub edges: usize,
    /// `Div_P^τ` of the uniform intake distribution over the batch's kNN
    /// graph (0.0 for `n < 2` — no graph, no spread measurable).
    pub div: f32,
}

/// One domain's full readout.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainReport {
    /// Domain label.
    pub domain: &'static str,
    /// Per-batch rows, ascending batch number.
    pub rows: Vec<BatchDivRow>,
    /// Entries whose text carried no `Batch <digits>` tag — counted loud,
    /// never folded into a batch (the honest-unparsed convention).
    pub untagged: usize,
    /// Corpus-level Div: μ uniform over EVERY entry of the domain (tagged
    /// or not) on one whole-domain kNN graph — the corpus spread scalar.
    pub corpus_div: f32,
    /// Total entries in the domain corpus.
    pub total: usize,
}

/// One intake entry: rule id, the tag haystack (the entry's own text
/// fields), and the embedding text (the pattern the retrieval index
/// embeds).
pub struct IntakeEntry {
    pub rule_id: String,
    pub tag_haystack: String,
    pub embed_text: String,
}

// Semantics of `DomainReport::untagged`: entries whose own text carries
// no `Batch <digits>` tag. These are the seed-era corpus rules (the
// hand-seeded domain tables that predate the mining batches) — they are
// NOT batch intake and never fold into a batch row; the count keeps
// them loud instead of pretending they belong to batch 0.

/// The LAST `Batch <digits>` occurrence (the citation-tail convention
/// puts the entry's own batch at the end; kinship citations elsewhere in
/// the text name other batches). Case-sensitive — lowercase prose ("the
/// batch's lead row") never matches. Returns `None` when no tag exists.
#[must_use]
pub fn parse_batch_tag(haystack: &str) -> Option<u32> {
    let bytes = haystack.as_bytes();
    let needle = b"Batch ";
    let mut last: Option<u32> = None;
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let start = i + needle.len();
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() && end - start < 9 {
                end += 1;
            }
            if end > start {
                // A digit run followed by a non-digit (or the text's end):
                // the boundary check keeps "Batch 345th" from reading 3450.
                if end == bytes.len() || !bytes[end].is_ascii_digit() {
                    if let Ok(v) = haystack[start..end].parse::<u32>() {
                        last = Some(v);
                    }
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    last
}

/// kNN edges over L2-normalized embeddings: per node the top-`k` others
/// by cosine (dot of unit vectors; ascending-index tiebreak), weight
/// clipped to `(0, 1]`, deduped to one undirected edge keeping the max.
/// Zero vectors carry no similarity (dot 0.0 → no edge). Deterministic:
/// no `HashMap` iteration order feeds the result (`BTreeMap` dedup).
#[must_use]
pub fn knn_edges(vecs: &[[f32; LATENT_DIM]], k: usize) -> Vec<(usize, usize, f32)> {
    let n = vecs.len();
    if n < 2 {
        return Vec::new();
    }
    let k_eff = k.min(n - 1).max(1);
    let mut pairs: BTreeMap<(usize, usize), f32> = BTreeMap::new();
    for (i, vi) in vecs.iter().enumerate() {
        let mut sims: Vec<(f32, usize)> = Vec::with_capacity(n - 1);
        for (j, vj) in vecs.iter().enumerate() {
            if j == i {
                continue;
            }
            let mut d = 0.0_f32;
            for (a, b) in vi.iter().zip(vj.iter()) {
                d += a * b;
            }
            sims.push((d, j));
        }
        sims.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        for &(s, j) in sims.iter().take(k_eff) {
            // NaN-safe positivity gate (a NaN dot can never pass — finite
            // inputs make it unreachable, the gate keeps the contract
            // explicit without a negated partial-ordered comparison).
            if !matches!(s.partial_cmp(&0.0), Some(std::cmp::Ordering::Greater)) {
                continue;
            }
            let w = s.min(1.0);
            let key = if i < j { (i, j) } else { (j, i) };
            let slot = pairs.entry(key).or_insert(0.0_f32);
            if w > *slot {
                *slot = w;
            }
        }
    }
    pairs.into_iter().map(|((a, b), w)| (a, b, w)).collect()
}

/// `Div_P^τ` of the uniform simplex over a batch's kNN graph.
// The uniform simplex over a count — the report's quantity; allowed.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn div_of(n: usize, edges: &[(usize, usize, f32)], tau: usize) -> f32 {
    if n < 2 {
        return 0.0;
    }
    let g = SimGraph::build(n, edges);
    let mu = vec![1.0_f32 / n as f32; n];
    let mut q = vec![0.0_f32; n];
    let mut r = vec![0.0_f32; n];
    g.div_tau(&mu, tau, &mut q, &mut r)
}

fn embed_all<'a, I: IntoIterator<Item = &'a str>>(texts: I) -> Vec<[f32; LATENT_DIM]> {
    texts
        .into_iter()
        .map(|t| ModellessEmbedder.embed(t))
        .collect()
}

/// Assemble one domain's [`DomainReport`] from its intake entries.
#[must_use]
pub fn domain_report(
    domain: &'static str,
    intake: &[IntakeEntry],
    tau: usize,
    k: usize,
) -> DomainReport {
    let total = intake.len();
    // Batch → entry indices, ascending batch (BTreeMap); first-seen entry
    // order preserved within a batch (stable push).
    let mut by_batch: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    let mut untagged = 0_usize;
    for (idx, e) in intake.iter().enumerate() {
        match parse_batch_tag(&e.tag_haystack) {
            Some(b) => {
                by_batch.entry(b).or_default().push(idx);
            }
            None => untagged += 1,
        }
    }
    let vecs = embed_all(intake.iter().map(|e| e.embed_text.as_str()));
    let mut rows = Vec::with_capacity(by_batch.len());
    for (batch, members) in &by_batch {
        let member_vecs: Vec<[f32; LATENT_DIM]> = members.iter().map(|&i| vecs[i]).collect();
        let edges = knn_edges(&member_vecs, k);
        let div = div_of(members.len(), &edges, tau);
        rows.push(BatchDivRow {
            domain,
            batch: *batch,
            n: members.len(),
            edges: edges.len(),
            div,
        });
    }
    let corpus_edges = knn_edges(&vecs, k);
    let corpus_div = div_of(total, &corpus_edges, tau);
    DomainReport {
        domain,
        rows,
        untagged,
        corpus_div,
        total,
    }
}

/// The whole readout over every compiled-in mined domain. Both collectors
/// are cfg-gated on their domain features; a build with neither returns
/// an empty report (the console says so honestly).
#[must_use]
pub fn report_all(tau: usize, k: usize) -> Vec<DomainReport> {
    let kernel = {
        #[cfg(feature = "kernel_opt")]
        {
            Some(domain_report("kernel_opt", &kernel_intake(), tau, k))
        }
        #[cfg(not(feature = "kernel_opt"))]
        {
            None
        }
    };
    let perf = {
        #[cfg(feature = "rust_perf")]
        {
            Some(domain_report("rust_perf", &perf_intake(), tau, k))
        }
        #[cfg(not(feature = "rust_perf"))]
        {
            None
        }
    };
    [kernel, perf].into_iter().flatten().collect()
}

#[cfg(feature = "kernel_opt")]
fn kernel_intake() -> Vec<IntakeEntry> {
    crate::domains::kernel_opt::KernelOptDomain::build_corpus()
        .into_iter()
        .map(|e| IntakeEntry {
            tag_haystack: format!(
                "{}\n{}\n{}\n{}",
                e.gpu_concept,
                e.buggy_code(),
                e.fixed_code(),
                e.context_notes().unwrap_or_default()
            ),
            rule_id: e.rule_id().to_string(),
            embed_text: e.buggy_code().to_string(),
        })
        .collect()
}

#[cfg(feature = "rust_perf")]
fn perf_intake() -> Vec<IntakeEntry> {
    crate::domains::rust_perf::RustPerfDomain::build_corpus()
        .into_iter()
        .map(|e| IntakeEntry {
            tag_haystack: format!(
                "{}\n{}\n{}",
                e.context_notes().unwrap_or_default(),
                e.buggy_code(),
                e.fixed_code()
            ),
            rule_id: e.rule_id().to_string(),
            embed_text: e.buggy_code().to_string(),
        })
        .collect()
}

/// The console line per domain:
/// `batch-div (222 T1, tau=1 k=5): kernel_opt: 163 batch(es) · 1196/1196
/// tagged · corpus_div=0.8123 · newest b349 n=1 div=0.5000`
/// (fractions at 4 dp). Deterministic; no trailing summary when no
/// domain compiled in (an honest empty, never a fabricated line).
#[must_use]
// Fractions over usize counters — the report's quantities; allowed.
#[allow(clippy::cast_precision_loss)]
pub fn render_summaries(reports: &[DomainReport]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    if reports.is_empty() {
        out.push_str(
            "batch-div (issue 222 T1): no mined domain compiled in (enable kernel_opt/rust_perf)\n",
        );
        return out;
    }
    for r in reports {
        let _ = write!(
            out,
            "batch-div (issue 222 T1, tau={} k={}): {}: {} batch(es) · {}/{} tagged · corpus_div={:.4}",
            DEFAULT_TAU,
            DEFAULT_KNN_K,
            r.domain,
            r.rows.len(),
            r.total - r.untagged,
            r.total,
            r.corpus_div
        );
        match r.rows.last() {
            Some(row) => {
                let _ = writeln!(
                    out,
                    " · newest b{} n={} div={:.4}",
                    row.batch, row.n, row.div
                );
            }
            None => {
                out.push_str(" · newest none\n");
            }
        }
    }
    out
}

/// The full per-batch table (the `RIIR_REFINE_BATCH_DIV_FULL=1` detail;
/// also the T2 correlation cell's data read).
#[must_use]
pub fn render_full(reports: &[DomainReport]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for r in reports {
        let _ = writeln!(
            out,
            "batch-div full: {} ({} batch(es), {} untagged, corpus_div={:.4}):",
            r.domain,
            r.rows.len(),
            r.untagged,
            r.corpus_div
        );
        for row in &r.rows {
            let _ = writeln!(
                out,
                "  b{} n={} edges={} div={:.4}",
                row.batch, row.n, row.edges, row.div
            );
        }
    }
    out
}

/// The score-bench tail entry: one summary line per domain to stdout,
/// the full table only when `RIIR_REFINE_BATCH_DIV_FULL=1`. Never fails,
/// never changes an exit code (report-only contract).
pub fn print_console() {
    let reports = report_all(DEFAULT_TAU, DEFAULT_KNN_K);
    print!("{}", render_summaries(&reports));
    if std::env::var("RIIR_REFINE_BATCH_DIV_FULL").as_deref() == Ok("1") {
        print!("{}", render_full(&reports));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_parses_last_occurrence() {
        assert_eq!(
            parse_batch_tag("cites Batch 300 earlier. Batch 345 / Plan 357 / Research 389."),
            Some(345)
        );
        assert_eq!(
            parse_batch_tag("Batch 349 / Plan 360 / Research 393."),
            Some(349)
        );
        assert_eq!(parse_batch_tag("the batch's lead row — no tag"), None);
        assert_eq!(parse_batch_tag("batch 12 lowercase never matches"), None);
        assert_eq!(parse_batch_tag(""), None);
        // The boundary check: "Batch 345th" must not read the '3450' run.
        assert_eq!(parse_batch_tag("Batch 345th of many"), Some(345));
        assert_eq!(parse_batch_tag("Batch 12 then Batch 7 /"), Some(7));
    }

    #[test]
    fn knn_edges_deterministic_and_k_adaptive() {
        let a = ModellessEmbedder.embed("fn f(x: u8) -> u8 { x.wrapping_add(1) }");
        let b = ModellessEmbedder.embed("async fn spawn_task(queue: &mut Queue) { queue.drain() }");
        let c = ModellessEmbedder.embed("struct PacketHeader { magic: u32, crc: u16 }");
        let v = vec![a, b, c];
        let e1 = knn_edges(&v, 5);
        let e2 = knn_edges(&v, 5);
        assert_eq!(e1, e2, "deterministic rebuild");
        // k adapts to n-1 = 2; both edges exist, no self loops, symmetric
        // dedup (each unordered pair at most once).
        assert!(e1.len() <= 3);
        for &(x, y, w) in &e1 {
            assert!(x < y, "undirected dedup: {x}<{y}");
            assert!((0.0..=1.0).contains(&w), "weight clipped to (0,1]: {w}");
        }
    }

    #[test]
    fn identical_pair_reads_lower_div_than_distinct_pair() {
        let long_a = "fn accumulate_rows(rows: &[Row]) -> Vec<f32> { let mut out = vec![0.0; 32]; \
for row in rows { for (c, v) in row.cols.iter().enumerate() { out[c] += *v as f32; } } out }";
        let long_a2 = "fn accumulate_rows_copy(rows: &[Row]) -> Vec<f32> { let mut acc = vec![0.0; 32]; \
for row in rows { for (c, v) in row.cols.iter().enumerate() { acc[c] += *v as f32; } } acc }";
        let long_b = "#[derive(Debug)] enum Wire { Mint { amount: u64, memo: String }, Burn { amount: u64 } } \
fn encode(w: &Wire) -> Vec<u8> { match w { Wire::Mint { amount, memo } => { let mut b = amount.to_le_bytes().to_vec(); b.extend_from_slice(memo.as_bytes()); b } Wire::Burn { amount } => amount.to_le_bytes().to_vec() } }";
        let same = vec![
            ModellessEmbedder.embed(long_a),
            ModellessEmbedder.embed(long_a2),
        ];
        let mixed = vec![
            ModellessEmbedder.embed(long_a),
            ModellessEmbedder.embed(long_b),
        ];
        let div_same = div_of(2, &knn_edges(&same, 5), DEFAULT_TAU);
        let div_mixed = div_of(2, &knn_edges(&mixed, 5), DEFAULT_TAU);
        assert!(
            div_same < div_mixed,
            "a near-duplicate batch must read lower Div than a distinct pair: {div_same} vs {div_mixed}"
        );
    }

    #[test]
    fn single_entry_and_empty_intake_are_honest_zeros() {
        let v = vec![ModellessEmbedder.embed("fn a() {}")];
        let edges = knn_edges(&v, 5);
        assert_eq!(edges, [] as [(usize, usize, f32); 0]);
        assert_eq!(div_of(1, &edges, DEFAULT_TAU), 0.0);
        let r = domain_report(
            "kernel_opt",
            &[IntakeEntry {
                rule_id: "solo".to_string(),
                tag_haystack: "Batch 1 / x".to_string(),
                embed_text: "fn a() {}".to_string(),
            }],
            DEFAULT_TAU,
            DEFAULT_KNN_K,
        );
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0].div, 0.0);
        assert_eq!(r.rows[0].edges, 0);
        assert_eq!(r.untagged, 0);
    }

    #[test]
    fn domain_report_groups_and_counts_untagged() {
        let intake = vec![
            IntakeEntry {
                rule_id: "a".to_string(),
                tag_haystack: "one Batch 10 / two".to_string(),
                embed_text: "fn alpha() { 1 }".to_string(),
            },
            IntakeEntry {
                rule_id: "b".to_string(),
                tag_haystack: "Batch 10 again".to_string(),
                embed_text: "fn beta() { 2 }".to_string(),
            },
            IntakeEntry {
                rule_id: "c".to_string(),
                tag_haystack: "no citation here".to_string(),
                embed_text: "fn gamma() { 3 }".to_string(),
            },
            IntakeEntry {
                rule_id: "d".to_string(),
                tag_haystack: "older Batch 9 / newer Batch 11 /".to_string(),
                embed_text: "fn delta() { 4 }".to_string(),
            },
        ];
        let r = domain_report("rust_perf", &intake, DEFAULT_TAU, DEFAULT_KNN_K);
        assert_eq!(r.total, 4);
        assert_eq!(r.untagged, 1);
        // Ascending batch: [10, 11]; b10 has 2 entries, b11 has 1.
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.rows[0].batch, 10);
        assert_eq!(r.rows[0].n, 2);
        assert_eq!(r.rows[1].batch, 11);
        assert_eq!(r.rows[1].n, 1);
        // Corpus div computed over all 4 entries.
        assert!(r.corpus_div >= 0.0);
    }

    #[test]
    fn renders_are_deterministic() {
        let r = domain_report(
            "kernel_opt",
            &[IntakeEntry {
                rule_id: "a".to_string(),
                tag_haystack: "Batch 42 /".to_string(),
                embed_text: "fn x() {}".to_string(),
            }],
            DEFAULT_TAU,
            DEFAULT_KNN_K,
        );
        let reports = vec![r];
        assert_eq!(render_summaries(&reports), render_summaries(&reports));
        assert_eq!(render_full(&reports), render_full(&reports));
        let s = render_summaries(&reports);
        assert!(s.contains("batch-div (issue 222 T1"));
        assert!(s.contains("kernel_opt"));
        assert!(s.contains("newest b42"));
    }

    #[cfg(feature = "kernel_opt")]
    #[test]
    fn real_kernel_corpus_report_is_sane() {
        let reports = report_all(DEFAULT_TAU, DEFAULT_KNN_K);
        let kernel = reports
            .iter()
            .find(|r| r.domain == "kernel_opt")
            .expect("kernel_opt compiled in");
        assert!(!kernel.rows.is_empty(), "the corpus carries batch tags");
        // Ascending batches; newest is the max.
        assert!(kernel.rows.windows(2).all(|w| w[0].batch < w[1].batch));
        assert_eq!(
            kernel.rows.last().unwrap().batch,
            kernel.rows.iter().map(|r| r.batch).max().unwrap()
        );
        // Tagged + untagged partitions the corpus exactly.
        let tagged: usize = kernel.rows.iter().map(|r| r.n).sum();
        assert_eq!(tagged + kernel.untagged, kernel.total);
        // The seed-era rules (hand-seeded tables predating the mining
        // batches) are the untagged class — the MINED majority carries
        // the tag. This pins that the tag parse actually reaches the
        // corpus (a regression to 0 tagged would red here).
        assert!(
            tagged > kernel.untagged,
            "majority of kernel_opt entries are batch-tagged: {tagged}/{}",
            kernel.total
        );
    }
}
