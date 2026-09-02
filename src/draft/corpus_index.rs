//! Per-domain latent rule index — the `--suggest` knowledge base (Plan 005 T1.2).
//!
//! Composes the riir-rag substrate as-is (DRY — zero retrieval code
//! reimplemented here): each corpus rule's *pattern text* (clippy/perf:
//! `buggy_code`; kernel: `ref_shape`) is embedded by [`ModellessEmbedder`]
//! (BLAKE3+DFT+sigmoid, deterministic, no weights) and indexed into a
//! [`NeuronDbRetriever`] (`ShardIndex` Clifford-wedge KNN + `Bm25Index`).
//!
//! Retrieval maps a real code span onto ranked rules:
//!
//! ```text
//! code span → ModellessEmbedder → LatentQuery → retrieve() → rule ids
//! ```
//!
//! The `zone_hash → rule_id` sidecar is keyed by `BLAKE3(pattern_text)` —
//! the exact key `NeuronDbRetriever::index_chunks` computes internally
//! (stable contract, documented in riir-rag retriever.rs).

use std::collections::HashMap;

use riir_rag::{
    Embedder, LatentQuery, ModellessEmbedder, NeuronDbRetriever, NeuronDbRetrieverConfig,
    RawChunk, RetrievalProvider, TokenBudget,
};

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
/// allocations are bounded: the substrate's per-`retrieve()` candidate `Vec`
/// (≤ `latent_k` + `bm25_k`) + ≤ one rule-id `String` per distinct retrieved
/// rule + the output clone.
#[derive(Default)]
pub struct SuggestScratch {
    /// Accumulated retrieved candidates across the multi-scale queries of
    /// one suggest call (cleared per call; capacity persists).
    retrieved: Vec<riir_rag::RetrievedChunk>,
    /// Per-rule best score accumulator (cleared via `drain` per call — the
    /// owned keys move straight into `ranked`).
    best: HashMap<String, f32>,
    /// Ranked output (cleared per call; capacity persists).
    ranked: Vec<ScoredRule>,
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
    retriever: NeuronDbRetriever,
    /// The dense embedder, when the Bonsai rerank path is active. Held so
    /// query-side `embed_dense` uses the SAME model the index was built with.
    #[cfg(feature = "bonsai_retrieval")]
    dense: Option<std::sync::Arc<riir_rag::BonsaiEmbedder>>,
    /// Scoring/tokenizing knobs this index was built with.
    config: RetrievalConfig,
    /// `BLAKE3(pattern_text_or_line)` → `rule_id` (the same key the retriever
    /// stores). Multi-scale: one rule contributes its whole pattern + each
    /// significant pattern line.
    by_zone_hash: HashMap<[u8; 32], String>,
    /// Distinct rules (NOT chunk count — multi-scale inflates chunks).
    n_rules: usize,
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
    /// Returns the substrate's indexing error (as a `String`) if
    /// `NeuronDbRetriever::index_chunks` fails.
    pub fn build(patterns: Vec<(String, String)>) -> Result<Self, String> {
        Self::build_with(patterns, RetrievalConfig::default())
    }

    /// [`build`](Self::build) with an explicit BM25 tokenizer — the A/B
    /// constructor the T3.5 sweep drives. `build` delegates with the
    /// promoted default.
    ///
    /// # Errors
    ///
    /// Returns the substrate's indexing error (as a `String`) if
    /// `NeuronDbRetriever::index_chunks` fails.
    pub fn build_with(
        patterns: Vec<(String, String)>,
        config: RetrievalConfig,
    ) -> Result<Self, String> {
        let embedder = ModellessEmbedder;
        let n_rules = patterns.len();
        let mut by_zone_hash = HashMap::with_capacity(n_rules);
        let mut chunks: Vec<RawChunk> = Vec::with_capacity(n_rules);
        for (rule_id, text) in patterns {
            let push = |content: &str, chunks: &mut Vec<RawChunk>| {
                chunks.push(RawChunk {
                    source: riir_rag::ChunkSource::new(format!("rule:{rule_id}"), 0, 0),
                    content: content.to_string(),
                });
                *blake3::hash(content.as_bytes()).as_bytes()
            };
            let whole_hash = push(&text, &mut chunks);
            by_zone_hash.insert(whole_hash, rule_id.clone());
            // Multi-scale: index each significant line of a multi-line pattern
            // under the SAME rule_id (>8 chars skips braces/blank noise).
            if text.lines().count() > 1 {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.len() > 8 {
                        let line_hash = push(trimmed, &mut chunks);
                        by_zone_hash.insert(line_hash, rule_id.clone());
                    }
                }
            }
        }
        // `with_bm25_tokenizer` MUST precede `index_chunks` — the tokenizer is
        // used at both index and search time, and swapping it after indexing
        // silently desyncs the postings from the queries (substrate contract).
        // Issue 063 — opt into riir-rag's TWO-STAGE latent KNN.
        //
        // `NeuronDbRetrieverConfig::default()` ships `two_stage_oversample:
        // None`, which riir-clippy had silently taken since the retriever was
        // adopted. That path is `retrieve_diverse` at **O(N·k)**; setting the
        // field switches to `retrieve_diverse_two_stage` at **O(N + M·k)**
        // (riir-rag Issue 593 / Bench 583). Nothing new was written here — this
        // is shipped upstream substrate this crate had simply never enabled.
        //
        // Why this is the lever, measured rather than reasoned (Issue 063):
        // `retrieve` is **92.1%** of `suggest_span`, against `rank_into` 2.3%
        // and embedding 0.8%. Two plausible-looking culprits were measured and
        // REJECTED first — hoisting the per-domain `query_for` would save 0.6%,
        // and content loading is only ~11% of `retrieve` (a tiny-budget probe
        // cut retrieved chunks to 0 and moved the total by that much). The cost
        // is `retrieve_candidates`, which runs regardless of budget.
        //
        // Measured effect, 11 interleaved A/B pairs (per-pair ratios, because
        // the box was loaded and absolute wall-clock drifted 2x during the
        // session): median **0.806**, 10 of 11 pairs favouring two-stage.
        //
        // Measured quality cost — this is an APPROXIMATION, so it is not free:
        //   * all 14 retrieval floors: IDENTICAL (clippy 89.6/93.8/95.8, perf
        //     96.2/98.1/100.0, kernel 100/100/100, blended 94.7/96.9/98.5,
        //     router 83/131) in both feature states;
        //   * `latent_suggest_goat` G1: IDENTICAL, 21/24, same miss list;
        //   * `uncontaminated_g1`: 8/8, pool recall @12 100%;
        //   * `t48_gp_acquisition_ab`: @1 0.940 and @3 0.970 and McNemar
        //     (b=74 c=1 z=8.429) identical, but **mrr 0.961 -> 0.959** —
        //     one item moved by one rank BELOW top-3. A/B'd in one worktree at
        //     one HEAD, so that delta is attributed to this flag, not to drift.
        // No gated floor moves; the only observable change sits under rank 3,
        // which is why `CLIPPY_TOP3` (at exactly zero margin) is untouched.
        //
        // `RIIR_CLIPPY_NO_TWO_STAGE=1` is the kill-switch (the repo's
        // feature-flag convention) and is what the A/B above toggled.
        let rcfg = || {
            let mut c = NeuronDbRetrieverConfig::default();
            if std::env::var("RIIR_CLIPPY_NO_TWO_STAGE").is_err() {
                c.two_stage_oversample = Some(4);
            }
            c
        };
        let retriever = match config.tokenizer {
            IndexTokenizer::CodeDefault => NeuronDbRetriever::new(rcfg()),
            IndexTokenizer::OpAware => NeuronDbRetriever::new(rcfg())
                .with_bm25_tokenizer(Box::new(super::op_tokenizer::RustOpTokenizer)),
            IndexTokenizer::OpAwareBigram => NeuronDbRetriever::new(rcfg())
                .with_bm25_tokenizer(Box::new(super::op_tokenizer::RustOpBigramTokenizer)),
        };
        #[cfg(feature = "bonsai_retrieval")]
        {
            if config.dense_weight > 0.0 {
                use riir_rag::{BonsaiEmbedStrategy, BonsaiEmbedder};
                let path = riir_rag::bonsai_embedder::default_gguf_path();
                let bonsai = BonsaiEmbedder::load(&path, BonsaiEmbedStrategy::MeanPool)
                    .map_err(|e| format!("bonsai load ({}): {e}", path.display()))?;
                let bonsai = std::sync::Arc::new(bonsai);
                let dense_idx = std::sync::Arc::new(
                    riir_neuron_db::dense_embed::DenseEmbedIndex::with_capacity(chunks.len()),
                );
                let retriever = retriever.with_dense_index(dense_idx);
                retriever
                    .index_chunks_dense(chunks, bonsai.as_ref())
                    .map_err(|e| e.to_string())?;
                return Ok(Self {
                    retriever,
                    config,
                    dense: Some(bonsai),
                    by_zone_hash,
                    n_rules,
                });
            }
        }
        retriever
            .index_chunks(chunks, &embedder)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            retriever,
            config,
            #[cfg(feature = "bonsai_retrieval")]
            dense: None,
            by_zone_hash,
            n_rules,
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

    /// Embed `code`, retrieve, and rank rules. `top_k` caps the result.
    ///
    /// The query carries the code text alongside the latent direction so the
    /// substrate's BM25 hybrid stage can boost on identifier overlap — the
    /// designed retrieval path, not a bolt-on.
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
        let query = self.query_for(code);
        let mut retrieved = self.retriever.retrieve(&query, TokenBudget(usize::MAX));
        scratch.retrieved.append(&mut retrieved);
        self.rank_into(&scratch.retrieved, top_k, &mut scratch.best, &mut scratch.ranked)
    }

    /// Multi-scale suggest: query the whole text AND each significant line,
    /// merging per-rule best scores across all queries.
    ///
    /// Matches the multi-scale INDEX side (whole patterns + pattern lines):
    /// a whole-function span whose interior line instantiates a rule still
    /// surfaces that rule, even when the item-scale embedding is distant.
    /// Query count = 1 + significant lines — fine for suggest mode (not a
    /// hot path; each query ~µs at corpus scale, Bench 575).
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
        scratch.retrieved.clear();
        {
            let query = self.query_for(code);
            let mut retrieved_vec = self.retriever.retrieve(&query, TokenBudget(usize::MAX));
            scratch.retrieved.append(&mut retrieved_vec);
        }
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.len() <= 8 {
                continue;
            }
            let query = self.query_for(trimmed);
            let mut line_vec = self.retriever.retrieve(&query, TokenBudget(usize::MAX));
            scratch.retrieved.append(&mut line_vec);
        }
        self.rank_into(&scratch.retrieved, top_k, &mut scratch.best, &mut scratch.ranked)
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
        let q = LatentQuery::from_direction(ModellessEmbedder.embed(text)).with_text(text);
        #[cfg(feature = "bonsai_retrieval")]
        if let Some(bonsai) = &self.dense {
            use riir_rag::DenseEmbedder;
            return q.with_dense_direction(bonsai.embed_dense(text));
        }
        q
    }

    fn rank_into<'s>(
        &self,
        retrieved: &[riir_rag::RetrievedChunk],
        top_k: usize,
        best: &'s mut HashMap<String, f32>,
        ranked: &'s mut Vec<ScoredRule>,
    ) -> &'s [ScoredRule] {
        best.clear();
        for chunk in retrieved {
            let Some(rule_id) = self.by_zone_hash.get(&chunk.zone_hash) else {
                continue;
            };
            let latent = self.config.latent_weight * chunk.latent_score;
            #[allow(unused_mut)] // `mut` is only exercised under `bonsai_retrieval`
            let mut score = chunk.exact_score.map_or(latent, |exact| latent + exact);
            // `dense_score` only exists on RetrievedChunk under riir-rag's
            // `bonsai_embedder` cfg — gate the read, not just the weight.
            #[cfg(feature = "bonsai_retrieval")]
            if self.config.dense_weight > 0.0 {
                if let Some(dense) = chunk.dense_score {
                    score += self.config.dense_weight * dense;
                }
            }
            let entry = best.entry(rule_id.clone()).or_insert(f32::NEG_INFINITY);
            if score > *entry {
                *entry = score;
            }
        }
        ranked.clear();
        // drain() moves the owned rule-id Strings straight into the output —
        // no per-rule re-allocation for keys.
        for (rule_id, score) in best.drain() {
            ranked.push(ScoredRule { rule_id, score, instantiates: false });
        }
        // The rule_id tie-break is LOAD-BEARING, not cosmetic: `best` is a
        // HashMap, so `.drain()` order varies per process (RandomState), and
        // without a total order two equal-scored rules land in drain-luck
        // order — which rule survives the `top_k` truncation below then
        // differs run to run (the t48_gp_acquisition_ab ordering A/B flaked
        // exactly this way: the set flipped at the pool boundary while every
        // aggregate jittered). `total_cmp` + rule_id is the same canonical
        // key order `instantiation_rerank::rerank` and the eval's
        // `canonical_rank` already apply — this aligns the substrate with
        // them so the whole retrieval stack is order-deterministic.
        ranked.sort_unstable_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });
        let len = ranked.len().min(top_k);
        &ranked[..len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn linear_scan_pattern_retrieves_o1_lookup() {
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let hits = index.suggest(
            "fn get_item(items: &[Item], key: u64) -> Option<&Item> {\n    for it in items {\n        if it.key == key { return Some(it); }\n    }\n    None\n}",
            3,
        );
        assert!(!hits.is_empty());
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
