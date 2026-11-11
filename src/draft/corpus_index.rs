//! Per-domain latent rule index — the `--suggest` knowledge base (Plan 005 T1.2).
//!
//! Composes the vendored [`crate::ragplane`] substrate as-is (DRY — zero
//! retrieval code reimplemented here): each corpus rule's *pattern text*
//! (clippy/perf: `buggy_code`; kernel: `ref_shape`) is embedded by
//! [`ModellessEmbedder`] (BLAKE3+DFT+sigmoid, deterministic, no weights) and
//! indexed into the in-repo fused retriever [`super::retriever::RuleRetriever`]
//! (exact cosine latent stage + the vendored `Bm25Index` fused query — issue
//! 221 P2b replaced the substrate `NeuronDbRetriever` with this seam as the
//! ONE retrieval path).
//!
//! Retrieval maps a real code span onto ranked rules:
//!
//! ```text
//! code span → ModellessEmbedder → LatentQuery → retrieve() → rule ids
//! ```
//!
//! The `zone_hash → rule_id` sidecar is keyed by `BLAKE3(pattern_text)` —
//! the exact key the retriever computes internally (BLAKE3 of the chunk
//! content, the substrate's stable contract, carried by the seam).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ragplane::chunker::RawChunk;
use crate::ragplane::embedder::{Embedder, ModellessEmbedder};
#[cfg(feature = "differential_anchor")]
use crate::ragplane::query::LATENT_DIM;
use crate::ragplane::query::{ChunkSource, LatentQuery, RetrievedChunk};

#[cfg(feature = "index_cache")]
use super::domain_router::DomainId;
use super::retriever::{RetrievalScratch, RuleRetriever};

/// Issue 643 — the process-global latent-leg-skip toggle (`true` = the
/// incumbent). The production path only READS it (always true); the census
/// A/B flips it around the interleaved walk passes via
/// [`CorpusIndex::set_latent_scan_enabled`].
static LATENT_SCAN: AtomicBool = AtomicBool::new(true);

fn latent_scan_set(enabled: bool) {
    LATENT_SCAN.store(enabled, Ordering::Relaxed);
}

fn latent_scan_get() -> bool {
    LATENT_SCAN.load(Ordering::Relaxed)
}

/// Issue 643 — the process-global latent-leg-skip toggle (`true` = the
/// incumbent). The production path only READS it (always true); the census
/// A/B flips it around the interleaved walk passes. Pub so the census
/// example can drive both arms in ONE binary (the same-binary law).
pub fn set_latent_scan_enabled(enabled: bool) {
    latent_scan_set(enabled);
}

/// A rule retrieved for a code span, with its relevance score.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredRule {
    /// The corpus rule id (clippy lint name / perf kebab id / kernel kebab id).
    pub rule_id: String,
    /// Relevance — the retriever's fused latent/BM25 score for the best
    /// matching chunk of this rule.
    ///
    /// **Text similarity, NOT rule instantiation** (measured — Bench 018
    /// addendum, 2026-08-14): adversarial clean snippets (the correct version
    /// of each corpus anti-pattern) score as high as or higher than true
    /// positives, in both whole-span and multi-scale scoring modes. There is
    /// NO minimum-score threshold that separates them; do not consume this
    /// as a confidence/certainty signal. The proposers' AST visitors are the
    /// only rule-instantiation gate (`--fix`); `--suggest` output is ranked
    /// candidates for a human.
    pub score: f32,
    /// Whether this rule's proposer visitor ACTUALLY MATCHED the span —
    /// rule *instantiation*, as opposed to the text similarity in `score`
    /// (Plan 005 T3.5.4).
    ///
    /// Always `false` until
    /// [`mark_instantiation`](super::instantiation_rerank::mark_instantiation)
    /// runs, and always `false` for rules with no proposer (kernel, and the
    /// perf rules outside the shipped five) — "no proposer" is deliberately
    /// indistinguishable from "no match", because absence of evidence is not
    /// evidence.
    pub instantiates: bool,
}

/// Reusable buffers for the suggest loop (Plan 005 T3.4 / G4).
///
/// Owned by the caller (the [`LatentMatcher`](super::LatentMatcher) holds
/// one) and threaded through `*_into` calls — capacities stabilize across
/// queries, so steady-state scanning grows nothing. Remaining per-query
/// allocations are bounded: ≤ one rule-id `String` per distinct retrieved
/// rule + the output clone (the substrate scratch — the candidate `Vec` +
/// `by_zone` map + the two-stage scan buffers — moved into this struct by
/// Issue 191 T7, so the per-line loop pays its fixed allocations once per
/// SCRATCH, not once per call).
#[derive(Default)]
pub struct SuggestScratch {
    /// Accumulated retrieved candidates across the multi-scale queries of
    /// one suggest call (cleared per call; capacity persists). `pub(crate)`
    /// so the Issue-196 narrowed kernel index reuses the SAME scratch
    /// (one caller-owned buffer, the G4 discipline).
    pub(crate) retrieved: Vec<RetrievedChunk>,
    /// The substrate retrieval scratch (the candidate `Vec` + `by_zone` map +
    /// the cosine/BM25 query buffers). Cleared at entry by every
    /// `retrieve_for_scoring_into` call (no cross-call state — the seam's
    /// determinism contract), so one instance per `SuggestScratch`
    /// is correct for the whole multi-scale loop. Identical results to
    /// the fresh-allocation path; the seam's scratch-reuse twin keeps the
    /// per-line loop allocation-free.
    pub(crate) retrieval: super::retriever::RetrievalScratch,
    /// Per-rule best score accumulator (cleared via `drain` per call — the
    /// owned keys move straight into `ranked`).
    pub(crate) best: HashMap<String, f32>,
    /// Ranked output (cleared per call; capacity persists).
    pub(crate) ranked: Vec<ScoredRule>,
}

impl SuggestScratch {
    /// Current `retrieved` capacity (G4 test observable).
    #[must_use]
    pub fn retrieved_capacity(&self) -> usize {
        self.retrieved.capacity()
    }

    /// Current `ranked` capacity (G4 test observable).
    #[must_use]
    pub fn ranked_capacity(&self) -> usize {
        self.ranked.capacity()
    }
}

/// The multi-scale index rows for one pattern text: the whole text plus
/// each significant line (`>8` trimmed chars) of a multi-line pattern. The
/// ONE definition of "significant line" — the flat build and the Issue-196
/// narrowed kernel build's merged zone lookup share it, so the two zone
/// maps can never drift apart on which rows exist.
pub(crate) fn significant_lines(text: &str) -> impl Iterator<Item = &str> {
    let multi_line = text.lines().count() > 1;
    std::iter::once(text).chain(
        text.lines()
            .filter(move |line| multi_line && line.trim().len() > 8)
            .map(str::trim),
    )
}

/// Head-truncate `text` to at most `budget` BYTES (Bench 136, issue 208
/// T7): a below-budget text (or `budget = None`) returns `text` unchanged
/// (byte-identical, the unarmed posture's contract). Above budget: take the
/// first `budget` bytes, walk back to a char boundary, then cut at the last
/// `\n` within that prefix so the query stays LINE-shaped (BM25 never sees
/// a torn token at the cut); if that only newline is at position 0 — or no
/// newline exists in the prefix — the char-safe byte cut stands. Zero
/// allocation (a slice of the input); deterministic for a given
/// (text, budget) pair.
#[must_use]
pub(crate) fn query_text_within(text: &str, budget: Option<usize>) -> &str {
    let Some(budget) = budget else { return text };
    if text.len() <= budget {
        return text;
    }
    let mut end = budget;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    if let Some(nl) = text[..end].rfind('\n') {
        if nl > 0 {
            return &text[..nl];
        }
    }
    &text[..end]
}

/// Merge retrieved chunks → per-rule best score, reading `zone_lookup`
/// (`zone_hash` → `rule_id`) and `config` (the fused-score weights). The shared
/// merge core: `RuleIndex::rank_into` delegates with its own fields, and
/// the Issue-196 narrowed kernel index calls it directly with its merged
/// zone lookup — one scoring semantics, never a re-implementation.
pub(crate) fn merge_retrieved_best(
    zone_lookup: &HashMap<[u8; 32], String>,
    config: &RetrievalConfig,
    retrieved: &[RetrievedChunk],
    best: &mut HashMap<String, f32>,
) {
    best.clear();
    for chunk in retrieved {
        let Some(rule_id) = zone_lookup.get(&chunk.zone_hash) else {
            continue;
        };
        let latent = config.latent_weight * chunk.latent_score;
        #[allow(unused_mut)] // `mut` is only exercised under `bonsai_retrieval`
        let mut score = chunk.exact_score.map_or(latent, |exact| latent + exact);
        // `dense_score` exists on the vendored RetrievedChunk under the
        // `dense_embed` feature (implied by `bonsai_retrieval`) — gate the
        // read on the feature, the weight on the config.
        #[cfg(feature = "dense_embed")]
        if config.dense_weight > 0.0 {
            if let Some(dense) = chunk.dense_score {
                score += config.dense_weight * dense;
            }
        }
        let entry = best.entry(rule_id.clone()).or_insert(f32::NEG_INFINITY);
        if score > *entry {
            *entry = score;
        }
    }
}

/// Drain the per-rule best map into `ranked`, sort (score desc, then the
/// canonical `rule_id` order), and truncate to `top_k`. The `rule_id` tie-break
/// is LOAD-BEARING, not cosmetic: `best` is a `HashMap`, so `.drain()` order
/// varies per process (`RandomState`), and without a total order two
/// equal-scored rules land in drain-luck order — which rule survives the
/// `top_k` truncation below then differs run to run (the
/// `t48_gp_acquisition_ab` ordering A/B flaked exactly this way: the set
/// flipped at the pool boundary while every aggregate jittered).
/// `total_cmp` + `rule_id` is the same canonical key order
/// `instantiation_rerank::rerank` and the eval's `canonical_rank` already
/// apply — this aligns the substrate with them so the whole retrieval stack
/// is order-deterministic.
pub(crate) fn finalize_ranked<'s>(
    best: &mut HashMap<String, f32>,
    ranked: &'s mut Vec<ScoredRule>,
    top_k: usize,
) -> &'s [ScoredRule] {
    ranked.clear();
    // drain() moves the owned rule-id Strings straight into the output —
    // no per-rule re-allocation for keys.
    for (rule_id, score) in best.drain() {
        ranked.push(ScoredRule {
            rule_id,
            score,
            instantiates: false,
        });
    }
    ranked.sort_unstable_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    let len = ranked.len().min(top_k);
    &ranked[..len]
}

/// Which BM25 tokenizer an index is built with (Plan 005 T3.5).
///
/// A/B-able because the choice is a measured one, not an assumed one — see
/// [`super::retrieval_eval::run_tokenizer_sweep`] and Bench 027.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum IndexTokenizer {
    /// The substrate default `CodeTokenizer` — identifiers only, all
    /// punctuation discarded. The pre-T3.5 shipped behaviour.
    CodeDefault,
    /// [`RustOpTokenizer`](super::op_tokenizer::RustOpTokenizer) — identifiers
    /// plus one token per maximal punctuation run.
    OpAware,
    /// [`RustOpBigramTokenizer`](super::op_tokenizer::RustOpBigramTokenizer) —
    /// `OpAware` plus operator-adjacency bigrams. **Default since T3.5**
    /// (Bench 027).
    #[default]
    OpAwareBigram,
}

/// How a [`RuleIndex`] scores and tokenizes (Plan 005 T3.5).
///
/// Both knobs are A/B-able because both defaults are *measured* choices, not
/// assumed ones — see [`super::retrieval_eval::run_retrieval_sweep`] and
/// Bench 027.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetrievalConfig {
    /// BM25 tokenizer. See [`IndexTokenizer`].
    pub tokenizer: IndexTokenizer,
    /// Weight on the latent term in the fused score
    /// (`latent_weight * latent + bm25`).
    ///
    /// The 8-D `ModellessEmbedder` is BLAKE3+DFT — on the short, structurally
    /// near-identical texts of the clippy corpus its similarity is close to a
    /// hash, so at weight 1.0 it injects noise on top of the (discriminative)
    /// BM25 term. Swept in T3.5; see [`Self::DEFAULT_LATENT_WEIGHT`].
    pub latent_weight: f32,
    /// Weight on the 768-D Bonsai dense-rerank term
    /// (`... + dense_weight * dense_cosine`). `0.0` disables the dense path
    /// entirely — no GGUF is loaded. Requires the `bonsai_retrieval` feature
    /// to have any effect; ignored (with the index built modellessly) without it.
    pub dense_weight: f32,
}

impl RetrievalConfig {
    /// The T3.5-measured latent weight (Bench 027).
    ///
    /// **The sweep REFUTED this lever**: with the operator-aware tokenizer,
    /// clippy top-1 is 66.7% at every weight in {1.0, 0.5, 0.25, 0.0} — the
    /// 8-D latent term never changes a ranking, because BM25 dominates and
    /// latent-only hits (`exact_score: None`) never win. Kept at 1.0 (the
    /// pre-T3.5 shipped value) so the promotion carries no gratuitous
    /// behaviour change; the knob survives only to keep the sweep runnable
    /// and the refutation reproducible.
    pub const DEFAULT_LATENT_WEIGHT: f32 = 1.0;
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            tokenizer: IndexTokenizer::default(),
            latent_weight: Self::DEFAULT_LATENT_WEIGHT,
            dense_weight: 0.0,
        }
    }
}

/// One domain's corpus indexed for latent retrieval.
///
/// Build cost is proportional to corpus size (clippy ~10, perf ~46, kernel
/// ~155 rules) — each rule is one chunk, one modelless embedding. Built once
/// per process at suggest time, not per query.
pub struct RuleIndex {
    retriever: RuleRetriever,
    /// The dense embedder, when the Bonsai rerank path is active. Held so
    /// query-side `embed_dense` uses the SAME model the index was built with.
    #[cfg(feature = "bonsai_retrieval")]
    dense: Option<std::sync::Arc<riir_rag_bonsai::BonsaiEmbedder>>,
    /// Scoring/tokenizing knobs this index was built with.
    config: RetrievalConfig,
    /// `BLAKE3(pattern_text_or_line)` → `rule_id` (the same key the retriever
    /// stores). Multi-scale: one rule contributes its whole pattern + each
    /// significant pattern line.
    by_zone_hash: HashMap<[u8; 32], String>,
    /// Distinct rules (NOT chunk count — multi-scale inflates chunks).
    n_rules: usize,
    /// Distinct rule ids (build order, deduped) — the full-vocab scoring side
    /// of the trained two-tower posture (Issue 126): the trained term scores
    /// EVERY indexed rule, so retrieval misses can still be rescued.
    /// Read ONLY under `rule_embed_poc` (`rank_into`'s trained arm) — carried
    /// ungated so the struct shape is feature-stable; dead-code-silent
    /// elsewhere by this annotation (the platform-dead-code class).
    #[cfg_attr(not(feature = "rule_embed_poc"), allow(dead_code))]
    rule_ids: Vec<String>,
    /// The frozen two-tower tables + the trained-term weight, when the
    /// Issue-126 posture is attached (opt-in feature; absent by default).
    #[cfg(feature = "rule_embed_poc")]
    trained: Option<(std::sync::Arc<super::rule_embed::TrainedTables>, f32)>,
    /// katgpt-rs Issue 882 P0 rider: the per-domain mean-CORPUS anchor ā
    /// (mean `ModellessEmbedder` direction over every indexed chunk) + the
    /// correction strength λ. λ = 0 (the build default) skips the
    /// correction entirely — byte-identical to the uncorrected path.
    #[cfg(feature = "differential_anchor")]
    diff_anchor: DiffAnchor,
}

/// The differential-anchor state carried by a [`RuleIndex`] (katgpt-rs
/// Issue 882 P0 rider): `q̂ = q − λ·ā` on the 8-D latent query direction.
#[cfg(feature = "differential_anchor")]
#[derive(Debug, Clone, Copy)]
struct DiffAnchor {
    /// Mean-corpus embedding over the indexed chunks (fit at build).
    mean: [f32; LATENT_DIM],
    /// Correction strength; `0.0` ⇒ no correction (the kill switch).
    lambda: f32,
}

#[cfg(feature = "differential_anchor")]
impl DiffAnchor {
    /// Fit the mean-corpus anchor over every chunk the index embeds — the
    /// SAME texts `index_chunks` embeds, so ā lives in the index's space.
    fn fit(chunks: &[RawChunk]) -> Self {
        let mut b = katgpt_core::differential_anchor::AnchorBuilder::new(LATENT_DIM);
        for c in chunks {
            b.observe(&ModellessEmbedder.embed(&c.content));
        }
        let mut mean = [0.0_f32; LATENT_DIM];
        b.finish_into(&mut mean);
        Self {
            mean,
            lambda: Self::env_lambda(),
        }
    }

    /// The same-binary A/B knob (the `RIIR_CLIPPY_NO_TWO_STAGE` precedent):
    /// `RIIR_CLIPPY_DIFF_ANCHOR_LAMBDA=<f32>` seeds λ at build so gates that
    /// construct their own matcher (uncontaminated G1, the score bench) can
    /// be A/B'd without edits. Unset / unparsable / non-finite ⇒ 0.0 (off).
    fn env_lambda() -> f32 {
        std::env::var(DIFF_ANCHOR_LAMBDA_ENV)
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|l| l.is_finite())
            .unwrap_or(0.0)
    }
}

/// Env knob seeding the differential-anchor λ at index build (see
/// `DiffAnchor::env_lambda`). Explicit `set_differential_anchor` overrides.
#[cfg(feature = "differential_anchor")]
pub const DIFF_ANCHOR_LAMBDA_ENV: &str = "RIIR_CLIPPY_DIFF_ANCHOR_LAMBDA";

/// The ONE mapping from a [`RetrievalConfig`]'s tokenizer discriminant to
/// the BM25 tokenizer instance — shared by the build path and the Issue-224
/// cache restore path, so a restored index tokenizes identically to a built
/// one (index/search share one tokenizer by construction).
pub(crate) fn bm25_tokenizer_for(
    config: &RetrievalConfig,
) -> Box<dyn crate::ragplane::bm25::Tokenizer + Send + Sync> {
    match config.tokenizer {
        IndexTokenizer::CodeDefault => Box::new(crate::ragplane::bm25::CodeTokenizer),
        IndexTokenizer::OpAware => Box::new(super::op_tokenizer::RustOpTokenizer),
        IndexTokenizer::OpAwareBigram => Box::new(super::op_tokenizer::RustOpBigramTokenizer),
    }
}

impl RuleIndex {
    /// Index `patterns` — one chunk per `(rule_id, pattern_text)`, plus one
    /// chunk per significant LINE of multi-line patterns (multi-scale: corpus
    /// patterns are mostly 1-5 line excerpts while real code spans are whole
    /// items — matching at both scales closes the granularity gap).
    ///
    /// Duplicate texts/lines are deduplicated by the substrate (identical
    /// content → identical `zone_hash` → upsert); the sidecar maps that hash to
    /// the last rule with that text. Corpus patterns are distinct in practice.
    ///
    /// # Errors
    ///
    /// Infallible today (the in-repo seam's modelless indexing cannot fail;
    /// the `Result` shape predates the issue-221 P2b seam swap and stays so
    /// callers — and any future fallible path — do not churn).
    pub fn build(patterns: Vec<(String, String)>) -> Result<Self, String> {
        Self::build_with(patterns, RetrievalConfig::default())
    }

    /// [`build`](Self::build) with an explicit BM25 tokenizer — the A/B
    /// constructor the T3.5 sweep drives. `build` delegates with the
    /// promoted default.
    ///
    /// # Errors
    ///
    /// Infallible today (see [`build`](Self::build)).
    pub fn build_with(
        patterns: Vec<(String, String)>,
        config: RetrievalConfig,
    ) -> Result<Self, String> {
        let embedder = ModellessEmbedder;
        let n_rules = patterns.len();
        let matcher_timing = std::env::var("RIIR_REFINE_MATCHER_TIMING").as_deref() == Ok("1");
        let t_chunks = std::time::Instant::now();
        let mut by_zone_hash = HashMap::with_capacity(n_rules);
        let mut rule_ids: Vec<String> = Vec::with_capacity(n_rules);
        let mut seen_ids: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(n_rules);
        let mut chunks: Vec<RawChunk> = Vec::with_capacity(n_rules);
        for (rule_id, text) in patterns {
            if seen_ids.insert(rule_id.clone()) {
                rule_ids.push(rule_id.clone());
            }
            let push = |content: &str, chunks: &mut Vec<RawChunk>| {
                chunks.push(RawChunk {
                    source: ChunkSource::new(format!("rule:{rule_id}"), 0, 0),
                    content: content.to_string(),
                });
                *blake3::hash(content.as_bytes()).as_bytes()
            };
            let whole_hash = push(&text, &mut chunks);
            by_zone_hash.insert(whole_hash, rule_id.clone());
            // Multi-scale: index each significant line of a multi-line pattern
            // under the SAME rule_id (>8 chars skips braces/blank noise) —
            // the shared `significant_lines` definition (the narrowed kernel
            // build's merged zone lookup reuses it, Issue 196).
            for content in significant_lines(&text).skip(1) {
                let line_hash = push(content, &mut chunks);
                by_zone_hash.insert(line_hash, rule_id.clone());
            }
        }
        #[cfg(feature = "differential_anchor")]
        let diff_anchor = DiffAnchor::fit(&chunks);
        let chunks_ms = t_chunks.elapsed().as_secs_f64() * 1e3;
        // The seam's BM25 tokenizer choice MUST precede `index_chunks` — the
        // tokenizer is used at both index and search time, and swapping it
        // after indexing silently desyncs the postings from the queries
        // (substrate contract, carried by the seam).
        //
        // Issue 063 note: the substrate's two-stage oversample knob has no
        // analogue on the in-repo seam — the latent stage is already exact
        // O(N) brute force (issue 221 P2b); the `RIIR_CLIPPY_NO_TWO_STAGE`
        // kill-switch had no posture to toggle and is retired with the knob.
        let mut retriever = RuleRetriever::new().with_bm25_tokenizer(bm25_tokenizer_for(&config));
        #[cfg(feature = "bonsai_retrieval")]
        {
            if config.dense_weight > 0.0 {
                use riir_rag_bonsai::{BonsaiEmbedStrategy, BonsaiEmbedder};
                let path = riir_rag_bonsai::bonsai_embedder::default_gguf_path();
                let bonsai = BonsaiEmbedder::load(&path, BonsaiEmbedStrategy::MeanPool)
                    .map_err(|e| format!("bonsai load ({}): {e}", path.display()))?;
                let bonsai = std::sync::Arc::new(bonsai);
                retriever.index_chunks_dense(chunks, bonsai.as_ref());
                return Ok(Self {
                    retriever,
                    config,
                    dense: Some(bonsai),
                    by_zone_hash,
                    n_rules,
                    rule_ids,
                    #[cfg(feature = "rule_embed_poc")]
                    trained: None,
                    #[cfg(feature = "differential_anchor")]
                    diff_anchor,
                });
            }
        }
        retriever.index_chunks(chunks, &embedder);
        if matcher_timing {
            eprintln!(
                "[matcher-timing] build_with: chunks {:.1}ms index_chunks {:.1}ms",
                chunks_ms,
                t_chunks.elapsed().as_secs_f64() * 1e3 - chunks_ms
            );
        }
        Ok(Self {
            retriever,
            config,
            #[cfg(feature = "bonsai_retrieval")]
            dense: None,
            by_zone_hash,
            n_rules,
            rule_ids,
            #[cfg(feature = "rule_embed_poc")]
            trained: None,
            #[cfg(feature = "differential_anchor")]
            diff_anchor,
        })
    }

    /// Number of indexed rules (distinct rule ids, not chunks).
    #[must_use]
    pub fn len(&self) -> usize {
        self.n_rules
    }

    /// Whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_zone_hash.is_empty()
    }

    /// Attach the frozen two-tower tables + the trained-term weight — the
    /// Issue-126 A/B posture knob. Absent tables/rows degrade gracefully
    /// (only rules with a ψ row get the trained term).
    #[cfg(feature = "rule_embed_poc")]
    pub fn set_rule_embed(
        &mut self,
        tables: &std::sync::Arc<super::rule_embed::TrainedTables>,
        weight: f32,
    ) {
        self.trained = Some((std::sync::Arc::clone(tables), weight));
    }

    /// Set the differential-anchor strength λ (katgpt-rs Issue 882 P0
    /// rider). `0.0` restores the uncorrected path exactly (the correction
    /// is skipped, not multiplied by zero).
    #[cfg(feature = "differential_anchor")]
    pub fn set_differential_anchor(&mut self, lambda: f32) {
        self.diff_anchor.lambda = lambda;
    }

    /// The fitted mean-corpus anchor ā (measurement observable).
    #[cfg(feature = "differential_anchor")]
    #[must_use]
    pub fn differential_anchor_mean(&self) -> &[f32; LATENT_DIM] {
        &self.diff_anchor.mean
    }

    // ------------------------------------------------------------------
    // Issue 224 T1 — the index-cache snapshot surface (feature
    // `index_cache`). `to_snapshot` extracts the built state the cache
    // stores; `from_snapshot` restores it. The pair is the ONLY sanctioned
    // way a matcher skips the corpus build: every field that determines a
    // ranking is carried, riders (`set_rule_embed` / active-set /
    // `set_differential_anchor` overrides) stay constructor-external as
    // before, and the cache lane disarms under every posture this pair
    // cannot represent (route-narrowed kernel, Bonsai dense).
    // ------------------------------------------------------------------

    /// Capture this built index for the cache store. Cheap by design
    /// (issue 226): the BM25 index is captured as a shared handle and only
    /// serialized on the writer thread ([`super::index_cache::DomainCapture`]),
    /// so the critical path pays the row copy, not the canonical BM25 encode.
    #[cfg(feature = "index_cache")]
    // The only error arm is the cfg'd Bonsai dense refusal — without
    // `bonsai_retrieval` the `Result` is always `Ok`, by posture not design.
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn capture(
        &self,
        domain: DomainId,
    ) -> Result<super::index_cache::DomainCapture, String> {
        // Defense in depth: the lane refuses to arm under the dense
        // posture (the embedder is not serializable state), but a future
        // caller bypassing `Lane::resolve` must not store a dense index
        // either — restoring one would silently drop query-side rerank.
        #[cfg(feature = "bonsai_retrieval")]
        if self.dense.is_some() {
            return Err(
                "dense-embedded index is not cacheable (the Bonsai embedder is not serialized)"
                    .to_string(),
            );
        }
        Ok(super::index_cache::DomainCapture::new(
            super::index_cache::DomainSnapshot {
                domain,
                rows: self.retriever.to_row_snaps(),
                // Unordered here; `DomainCapture::finish` sorts it into the
                // canonical order on the writer thread.
                by_zone_hash: self
                    .by_zone_hash
                    .iter()
                    .map(|(h, r)| (*h, r.clone()))
                    .collect(),
                rule_ids: self.rule_ids.clone(),
                n_rules: self.n_rules,
                bm25: Vec::new(),
                #[cfg(feature = "differential_anchor")]
                anchor_mean: self.diff_anchor.mean,
            },
            self.retriever.bm25_shared(),
        ))
    }

    /// Rebuild an index from a cache snapshot. The BM25 restore is
    /// self-validating (digest + structural invariants — a corrupt snapshot
    /// refuses as a typed error, surfaced here as a `String`); everything
    /// else is field-exact.
    #[cfg(feature = "index_cache")]
    pub(crate) fn from_snapshot(
        snap: super::index_cache::DomainSnapshot,
        config: RetrievalConfig,
    ) -> Result<Self, String> {
        let bm25 = crate::ragplane::bm25::Bm25Index::restore_snapshot(
            bm25_tokenizer_for(&config),
            &snap.bm25,
        )
        .map_err(|e| e.to_string())?;
        let retriever = RuleRetriever::from_row_snaps(snap.rows, bm25);
        Ok(Self {
            retriever,
            config,
            #[cfg(feature = "bonsai_retrieval")]
            dense: None,
            by_zone_hash: snap.by_zone_hash.into_iter().collect(),
            n_rules: snap.n_rules,
            rule_ids: snap.rule_ids,
            #[cfg(feature = "rule_embed_poc")]
            trained: None,
            #[cfg(feature = "differential_anchor")]
            diff_anchor: DiffAnchor {
                mean: snap.anchor_mean,
                lambda: DiffAnchor::env_lambda(),
            },
        })
    }

    /// Project the query span through the frozen span tower, when the
    /// trained posture is attached.
    #[cfg(feature = "rule_embed_poc")]
    fn phi_for(&self, code: &str) -> Option<[f32; super::rule_embed::EMBED_DIM]> {
        let (tables, _) = self.trained.as_ref()?;
        Some(tables.project_span(code))
    }

    /// Embed `code`, retrieve, and rank rules. `top_k` caps the result.
    ///
    /// The query carries the code text alongside the latent direction so the
    /// substrate's BM25 hybrid stage can boost on identifier overlap — the
    /// designed retrieval path, not a bolt-on.
    #[must_use]
    pub fn suggest(&self, code: &str, top_k: usize) -> Vec<ScoredRule> {
        let mut scratch = SuggestScratch::default();
        self.suggest_into(code, top_k, &mut scratch).to_vec()
    }

    /// [`suggest`](Self::suggest) with caller-owned scratch (the G4 path).
    pub fn suggest_into<'s>(
        &self,
        code: &str,
        top_k: usize,
        scratch: &'s mut SuggestScratch,
    ) -> &'s [ScoredRule] {
        scratch.retrieved.clear();
        // The trained posture's query projection (owned under the feature —
        // a stack array — and borrowed into the rank call; `None` when the
        // feature is off, typed uniformly as Option<&[f32]>).
        #[cfg(feature = "rule_embed_poc")]
        let phi_owned = self.phi_for(code);
        let phi: Option<&[f32]> = {
            #[cfg(feature = "rule_embed_poc")]
            {
                phi_owned.as_ref().map(<[f32; 64]>::as_slice)
            }
            #[cfg(not(feature = "rule_embed_poc"))]
            {
                None
            }
        };
        // Score-only retrieval (Issue 063): `rank_into` reads `zone_hash` +
        // scores only, so the packed `retrieve()` path's content Strings +
        // source lookups were 100% discarded work — measured 53-61% of each
        // retrieve call (the gate's G2 latency overrun).
        self.retrieve_scored_into(code, &mut scratch.retrieved, &mut scratch.retrieval);
        self.rank_into(
            phi,
            &scratch.retrieved,
            top_k,
            &mut scratch.best,
            &mut scratch.ranked,
        )
    }

    /// Multi-scale suggest: query the whole text AND each significant line,
    /// merging per-rule best scores across all queries.
    ///
    /// Matches the multi-scale INDEX side (whole patterns + pattern lines):
    /// a whole-function span whose interior line instantiates a rule still
    /// surfaces that rule, even when the item-scale embedding is distant.
    /// Query count = 1 + significant lines — fine for suggest mode (not a
    /// hot path; each query ~µs at corpus scale, Bench 575).
    #[must_use]
    pub fn suggest_deep(&self, code: &str, top_k: usize) -> Vec<ScoredRule> {
        let mut scratch = SuggestScratch::default();
        self.suggest_deep_into(code, top_k, &mut scratch).to_vec()
    }

    /// [`suggest_deep`](Self::suggest_deep) with caller-owned scratch — the
    /// G4 path: buffer capacities stabilize across queries (zero growth in
    /// steady state; the only remaining allocations are the ≤ `distinct
    /// rules` id strings + the returned output clone).
    ///
    /// The returned slice borrows `scratch` — consume (`.to_vec()` / print)
    /// before the next call.
    pub fn suggest_deep_into<'s>(
        &self,
        code: &str,
        top_k: usize,
        scratch: &'s mut SuggestScratch,
    ) -> &'s [ScoredRule] {
        // `usize::MAX` = every significant line (the exact pre-cap behavior:
        // same skips, same order, same query sequence — the capped variant's
        // below-cap branch IS this loop). No text budget (the T7 cap arms
        // only through the `KernelIndex` dispatch).
        self.suggest_deep_into_capped(code, top_k, scratch, usize::MAX, None)
    }

    /// [`Self::suggest_deep_into`] with a per-span line-query cap (Bench 134,
    /// issue 208 T6): the whole-span query ALWAYS runs; the per-line queries
    /// are stride-selected when they exceed `max_line_queries` (indices
    /// `floor(i * len / cap)`, distinct ascending — uniform coverage; the
    /// per-rule-best merge makes query order irrelevant, so selection affects
    /// only WHICH lines are queried). `usize::MAX` = the exact uncapped path.
    /// The allocation is one bounded scratch Vec per SPAN (never per query).
    ///
    /// `query_text_budget` (Bench 136, issue 208 T7): when `Some(B)`, EVERY
    /// query text this call issues — the whole-span query AND each line
    /// query — is head-truncated to at most `B` bytes via
    /// [`query_text_within`] (a lossy, deterministic cut; Bench 135 proved
    /// the residual cost is query-COUNT-invariant, so the text mass is the
    /// lever). `None` = the exact pre-T7 text (below-budget and unarmed
    /// paths are byte-identical by construction).
    pub(crate) fn suggest_deep_into_capped<'s>(
        &self,
        code: &str,
        top_k: usize,
        scratch: &'s mut SuggestScratch,
        max_line_queries: usize,
        query_text_budget: Option<usize>,
    ) -> &'s [ScoredRule] {
        scratch.retrieved.clear();
        // The whole-span query text (truncated only when a budget arms).
        let whole = query_text_within(code, query_text_budget);
        #[cfg(feature = "rule_embed_poc")]
        let phi_owned = self.phi_for(whole);
        let phi: Option<&[f32]> = {
            #[cfg(feature = "rule_embed_poc")]
            {
                phi_owned.as_ref().map(<[f32; 64]>::as_slice)
            }
            #[cfg(not(feature = "rule_embed_poc"))]
            {
                None
            }
        };
        {
            self.retrieve_scored_into(whole, &mut scratch.retrieved, &mut scratch.retrieval);
        }
        // The significant lines (the SAME two skips as always, Issue 063:
        // insignificant lines contribute nothing, and a line whose trimmed
        // text IS the whole span duplicates the whole-span query).
        let mut significant = Vec::new();
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.len() <= 8 || trimmed == code {
                continue;
            }
            significant.push(trimmed);
        }
        if significant.len() <= max_line_queries {
            for line in &significant {
                self.retrieve_scored_into(
                    query_text_within(line, query_text_budget),
                    &mut scratch.retrieved,
                    &mut scratch.retrieval,
                );
            }
        } else {
            // Stride selection over `significant` (len > cap here, so the
            // indices are strictly ascending and distinct).
            for i in 0..max_line_queries {
                let idx = i * significant.len() / max_line_queries;
                self.retrieve_scored_into(
                    query_text_within(significant[idx], query_text_budget),
                    &mut scratch.retrieved,
                    &mut scratch.retrieval,
                );
            }
        }
        self.rank_into(
            phi,
            &scratch.retrieved,
            top_k,
            &mut scratch.best,
            &mut scratch.ranked,
        )
    }

    /// Merge retrieved chunks → per-rule best score → descending rank,
    /// writing into the caller's `best`/`ranked` buffers (reused capacity).
    /// Split-field params (not `&mut SuggestScratch`) so call sites can
    /// field-split the borrow alongside `&scratch.retrieved`.
    #[allow(clippy::type_complexity)]
    /// Build the query for `text`, attaching the 768-D dense direction when
    /// the Bonsai rerank path is active.
    #[cfg_attr(not(feature = "bonsai_retrieval"), allow(clippy::unused_self))]
    fn query_for(&self, text: &str) -> LatentQuery {
        #[allow(unused_mut)] // `mut` is only exercised under `differential_anchor`
        let mut direction = ModellessEmbedder.embed(text);
        // katgpt-rs Issue 882 P0 rider: fold the common-mode reference out of
        // the query (one axpy on a stack array). λ = 0 skips — byte-identical.
        #[cfg(feature = "differential_anchor")]
        if self.diff_anchor.lambda != 0.0 {
            katgpt_core::differential_anchor::correct_query(
                &mut direction,
                &self.diff_anchor.mean,
                self.diff_anchor.lambda,
            );
        }
        #[cfg_attr(not(feature = "bonsai_retrieval"), allow(unused_mut))]
        // `mut` only under `bonsai_retrieval`
        let mut q = LatentQuery::from_direction(direction)
            .with_text(text)
            .with_latent_scan(latent_scan_get());
        #[cfg(feature = "bonsai_retrieval")]
        if let Some(bonsai) = &self.dense {
            // The dep-side trait — what `BonsaiEmbedder` implements (the
            // vendored trait set has no foreign impls).
            use riir_rag::embedder::DenseEmbedder as _;
            q = q.with_dense_direction(bonsai.embed_dense(text));
            return q;
        }
        q
    }

    /// Merge retrieved chunks → per-rule best score → descending rank,
    /// writing into the caller's `best`/`ranked` buffers (reused capacity).
    /// Split-field params (not `&mut SuggestScratch`) so call sites can
    /// field-split the borrow alongside `&scratch.retrieved`.
    ///
    /// `phi` is the trained posture's query projection (Issue 126) — `None`
    /// when the feature is off or no tables are attached, in which case the
    /// ranking is byte-identical to the pre-126 baseline.
    #[cfg_attr(not(feature = "rule_embed_poc"), allow(unused_variables))]
    fn rank_into<'s>(
        &self,
        phi: Option<&[f32]>,
        retrieved: &[RetrievedChunk],
        top_k: usize,
        best: &'s mut HashMap<String, f32>,
        ranked: &'s mut Vec<ScoredRule>,
    ) -> &'s [ScoredRule] {
        merge_retrieved_best(&self.by_zone_hash, &self.config, retrieved, best);
        // The trained posture (Issue 126): score EVERY indexed rule through
        // the frozen towers and blend — additive on retrieved candidates,
        // stand-alone for rules retrieval missed entirely (the rescue path
        // the fusion cannot provide). Rules without a ψ row are skipped
        // (`score_rule` → None), an honest per-rule fallback to the baseline.
        #[cfg(feature = "rule_embed_poc")]
        if let (Some(phi), Some((tables, weight))) = (phi, &self.trained) {
            for id in &self.rule_ids {
                if let Some(t) = tables.score_rule(phi, id) {
                    let s = weight * t;
                    match best.get_mut(id) {
                        Some(e) => *e += s,
                        None => {
                            best.insert(id.clone(), s);
                        }
                    }
                }
            }
        }
        finalize_ranked(best, ranked, top_k)
    }

    /// Retrieve scored candidates for `text` into `out` (append) — the ONE
    /// retrieval primitive, shared by the flat suggest paths and the
    /// Issue-196 narrowed kernel index (which queries its per-category
    /// sub-indices through this method, never the seam directly).
    /// Score-only packed path: no content/source loads (Issue 063 law).
    ///
    /// Issue 191 T7's scratch-reuse law carries over: `retrieval` carries the
    /// candidate `Vec` + `by_zone` map across the per-line loop instead of
    /// reallocating per call. Identical results; the scratch is cleared at
    /// entry, so no state crosses calls.
    pub(crate) fn retrieve_scored_into(
        &self,
        text: &str,
        out: &mut Vec<RetrievedChunk>,
        retrieval: &mut RetrievalScratch,
    ) {
        let query = self.query_for(text);
        let retrieved = self.retriever.retrieve_for_scoring_into(&query, retrieval);
        out.append(retrieved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn mini_corpus() -> Vec<(String, String)> {
        vec![
            (
                "needless-return".to_string(),
                "fn compute(x: i32) -> i32 { return x * 2; }".to_string(),
            ),
            (
                "redundant-closure".to_string(),
                "let names: Vec<String> = people.iter().map(|p| format_name(p)).collect();"
                    .to_string(),
            ),
            (
                "o1-lookup".to_string(),
                "fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n    for u in users {\n        if u.id == id { return Some(u); }\n    }\n    None\n}"
                    .to_string(),
            ),
        ]
    }

    #[test]
    fn builds_and_indexes_all_rules() {
        let index = RuleIndex::build(mini_corpus()).expect("build");
        assert_eq!(index.len(), 3);
        assert!(!index.is_empty());
    }

    #[test]
    fn top1_matches_the_closest_pattern_variant() {
        // A real-world-shaped variant of the needless-return pattern:
        // different identifiers, different body, same `return expr;` shape.
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let hits = index.suggest("fn area(w: f64, h: f64) -> f64 { return w * h; }", 3);
        assert!(!hits.is_empty(), "must retrieve at least one rule");
        assert_eq!(
            hits[0].rule_id, "needless-return",
            "top-1 must be the return-shape rule, got {hits:?}"
        );
    }

    // ── Bench 134 (issue 208 T6): the per-span line-query cap ───────────

    #[test]
    fn capped_at_max_is_byte_identical_to_uncapped() {
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let code = "fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n    let mut found: Option<&User> = None;\n    for u in users.iter() {\n        if u.id == id { found = Some(u); }\n    }\n    found\n}";
        let mut s1 = SuggestScratch::default();
        let mut s2 = SuggestScratch::default();
        let a = index.suggest_deep_into(code, 3, &mut s1);
        let b = index.suggest_deep_into_capped(code, 3, &mut s2, usize::MAX, None);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.rule_id, y.rule_id);
            assert_eq!(x.score, y.score, "scores must be bit-identical at MAX");
        }
    }

    #[test]
    fn cap_preserves_the_top_rule_on_a_line_heavy_span() {
        let index = RuleIndex::build(mini_corpus()).expect("build");
        // A line-heavy span whose interior instantiates the o1-lookup rule.
        let mut code =
            String::from("fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n");
        for i in 0..60 {
            let _ = writeln!(code, "    let pad_{i} = {i} + some_padding_value_{i};");
        }
        code.push_str(
            "    for u in users {\n        if u.id == id { return Some(u); }\n    }\n    None\n}",
        );
        let mut s1 = SuggestScratch::default();
        let mut s2 = SuggestScratch::default();
        let uncapped = index.suggest_deep_into(&code, 3, &mut s1);
        let capped = index.suggest_deep_into_capped(&code, 3, &mut s2, 2, None);
        assert_ne!(uncapped, []);
        assert_eq!(
            capped[0].rule_id, uncapped[0].rule_id,
            "the cap must preserve the rank-1 rule on a line-heavy span (the whole-span query dominates; stride selection keeps body coverage)"
        );
    }

    #[test]
    fn cap_below_max_queries_fewer_lines_but_same_whole_span() {
        // Behavior guard: the capped path still runs the whole-span query
        // (the span itself is significant), so an empty cap result is a bug.
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let code = "fn compute(x: i32) -> i32 {\n    let a = first_step(x);\n    let b = second_step(a);\n    return b * 2;\n}";
        let mut s = SuggestScratch::default();
        let capped = index.suggest_deep_into_capped(code, 3, &mut s, 1, None);
        assert!(!capped.is_empty(), "the whole-span query always runs");
    }

    // ── Bench 136 (issue 208 T7): the per-query TEXT cap ───────────────

    #[test]
    fn query_text_within_below_budget_and_none_are_identity() {
        let text = "fn a() { b }\nfn c() { d }"; // 24 bytes
        assert_eq!(query_text_within(text, None), text);
        assert_eq!(
            query_text_within(text, Some(1024)),
            text,
            "below budget = unchanged"
        );
        assert_eq!(
            query_text_within(text, Some(text.len())),
            text,
            "exactly at budget = unchanged"
        );
    }

    #[test]
    fn query_text_within_cuts_at_the_last_newline_within_budget() {
        // Three lines of 10 bytes each (with the \n separators).
        let text = "aaaaaaaaaa\nbbbbbbbbbb\ncccccccccc";
        // Budget 22 → the char-safe prefix covers through the 2nd \n (idx 21);
        // the cut lands at the last \n within it → exactly the first two lines
        // minus the trailing newline.
        let cut = query_text_within(text, Some(22));
        assert_eq!(cut, "aaaaaaaaaa\nbbbbbbbbbb");
        assert!(cut.len() <= 22);
    }

    #[test]
    fn query_text_within_char_boundary_fallback() {
        // 'é' is 2 bytes; 'π' is 2 bytes. A budget landing mid-char must walk
        // back to a boundary, never panic, never split a char.
        let text = "hélloé wörldπéπ"; // multibyte throughout
        let cut = query_text_within(text, Some(5));
        assert!(cut.len() <= 5, "the char-safe cut never exceeds the budget");
        assert!(text.starts_with(cut), "the cut is a prefix of the input");
        // Every char boundary of the cut is a char boundary of the text by
        // construction (it IS a &str slice) — the assertion above pins the
        // prefix property; this test exists to prove no panic + no torn char.
    }

    #[test]
    fn query_text_within_newline_at_zero_and_no_newline_fallbacks() {
        // A text STARTING with \n: the only newline in the prefix is at 0 →
        // the byte cut stands (never an empty query).
        let text = "\nbbbbbbbbbb\ncccccccccc";
        let cut = query_text_within(text, Some(6));
        assert!(
            !cut.is_empty(),
            "the nl==0 guard must never produce an empty query"
        );
        assert_eq!(cut, "\nbbbbb");
        // A single-line giant text: no \n in the prefix → the byte cut stands.
        let flat = "x".repeat(100);
        assert_eq!(query_text_within(&flat, Some(40)), &flat[..40]);
    }

    #[test]
    fn text_cap_none_is_byte_identical_to_uncapped_on_a_giant_span() {
        // The `MAX + None` ≡ `suggest_deep_into` delegation equivalence,
        // pinned on a GIANT span (the shape the T7 lever targets).
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let mut code =
            String::from("fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n");
        for i in 0..200 {
            let _ = writeln!(
                code,
                "    let literal_{i} = \"pad text {i} to grow the span past any sane budget\""
            );
        }
        code.push_str(
            "    for u in users {\n        if u.id == id { return Some(u); }\n    }\n    None\n}",
        );
        assert!(code.len() > 4096, "the fixture span must be giant");
        let mut s1 = SuggestScratch::default();
        let mut s2 = SuggestScratch::default();
        let a = index.suggest_deep_into(&code, 3, &mut s1);
        let b = index.suggest_deep_into_capped(&code, 3, &mut s2, usize::MAX, None);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.rule_id, y.rule_id);
            assert_eq!(x.score, y.score, "None budget must be bit-identical");
        }
    }

    #[test]
    fn text_cap_truncated_whole_span_query_still_runs_and_retrieves() {
        // Behavior guard: the whole-span query runs on the TRUNCATED text and
        // still retrieves (an empty armed-budget result is a bug); and the
        // truncation helper bounds the giant span's line queries too.
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let mut code =
            String::from("fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n");
        for i in 0..120 {
            let _ = writeln!(
                code,
                "    let literal_{i} = \"pad text {i} padding the span far past the budget\""
            );
        }
        code.push_str(
            "    for u in users {\n        if u.id == id { return Some(u); }\n    }\n    None\n}",
        );
        // The whole-span text the armed budget would hand the query:
        let whole = query_text_within(&code, Some(1024));
        assert!(
            whole.len() <= 1024,
            "the giant span's query text is bounded"
        );
        assert!(code.starts_with(whole), "head truncation = a prefix");
        // A giant single LINE (the kilobyte-literal shape) is bounded too:
        let giant_line = format!("let s = \"{}\";", "y".repeat(3000));
        let line_cut = query_text_within(&giant_line, Some(1024));
        assert!(line_cut.len() <= 1024);
        // And the capped+budgeted suggest still answers (whole-span query ran).
        let mut s = SuggestScratch::default();
        let capped = index.suggest_deep_into_capped(&code, 3, &mut s, 2, Some(1024));
        assert!(
            !capped.is_empty(),
            "the whole-span query always runs, truncated or not"
        );
    }

    #[test]
    fn linear_scan_pattern_retrieves_o1_lookup() {
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let hits = index.suggest(
            "fn get_item(items: &[Item], key: u64) -> Option<&Item> {\n    for it in items {\n        if it.key == key { return Some(it); }\n    }\n    None\n}",
            3,
        );
        assert_ne!(hits, [] as [ScoredRule; 0]);
        assert_eq!(
            hits[0].rule_id, "o1-lookup",
            "top-1 must be the linear-scan rule, got {hits:?}"
        );
    }

    #[test]
    fn top_k_is_respected() {
        let index = RuleIndex::build(mini_corpus()).expect("build");
        assert!(index.suggest("fn f() {}", 2).len() <= 2);
    }

    #[test]
    fn deterministic_output() {
        // G1 helper: identical query → identical ranking (BLAKE3+DFT is
        // deterministic; the substrate adds no randomness). Each call builds
        // a FRESH `SuggestScratch` → a fresh `HashMap` with its own
        // RandomState, so every iteration samples a different `drain()`
        // order — the rule_id tie-break in `rank_into` must make the output
        // identical across all of them.
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let first = index.suggest("fn calc(a: i32) -> i32 { return a + 1; }", 3);
        for _ in 0..8 {
            let again = index.suggest("fn calc(a: i32) -> i32 { return a + 1; }", 3);
            assert_eq!(first, again, "ranking drifted across fresh-scratch runs");
        }
    }
}
