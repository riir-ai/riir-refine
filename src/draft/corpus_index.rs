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
    pub score: f32,
}

/// One domain's corpus indexed for latent retrieval.
///
/// Build cost is proportional to corpus size (clippy ~10, perf ~46, kernel
/// ~155 rules) — each rule is one chunk, one modelless embedding. Built once
/// per process at suggest time, not per query.
pub struct RuleIndex {
    retriever: NeuronDbRetriever,
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
        let retriever = NeuronDbRetriever::new(NeuronDbRetrieverConfig::default());
        retriever
            .index_chunks(chunks, &embedder)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            retriever,
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
        let activity = ModellessEmbedder.embed(code);
        let query = LatentQuery::from_direction(activity).with_text(code);
        // Generous budget: for suggest mode we want every retrieved candidate
        // (latent_k + bm25_k, ≤32 at default config), not a packed subset.
        let retrieved = self.retriever.retrieve(&query, TokenBudget(usize::MAX));
        self.rank_retrieved(retrieved.iter(), top_k)
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
        let mut retrieved_all: Vec<riir_rag::RetrievedChunk> = Vec::new();
        {
            let activity = ModellessEmbedder.embed(code);
            let query = LatentQuery::from_direction(activity).with_text(code);
            retrieved_all.extend(self.retriever.retrieve(&query, TokenBudget(usize::MAX)));
        }
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.len() <= 8 {
                continue;
            }
            let activity = ModellessEmbedder.embed(trimmed);
            let query = LatentQuery::from_direction(activity).with_text(trimmed);
            retrieved_all.extend(self.retriever.retrieve(&query, TokenBudget(usize::MAX)));
        }
        self.rank_retrieved(retrieved_all.iter(), top_k)
    }

    /// Merge retrieved chunks → per-rule best score → descending rank.
    fn rank_retrieved<'a>(
        &self,
        retrieved: impl Iterator<Item = &'a riir_rag::RetrievedChunk>,
        top_k: usize,
    ) -> Vec<ScoredRule> {
        let mut best: HashMap<&str, f32> = HashMap::new();
        for chunk in retrieved {
            let Some(rule_id) = self.by_zone_hash.get(&chunk.zone_hash) else {
                continue;
            };
            let score = chunk
                .exact_score
                .map_or(chunk.latent_score, |exact| chunk.latent_score + exact);
            let entry = best.entry(rule_id.as_str()).or_insert(f32::NEG_INFINITY);
            if score > *entry {
                *entry = score;
            }
        }
        let mut ranked: Vec<ScoredRule> = best
            .into_iter()
            .map(|(rule_id, score)| ScoredRule {
                rule_id: rule_id.to_string(),
                score,
            })
            .collect();
        ranked.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.truncate(top_k);
        ranked
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
        // deterministic; the substrate adds no randomness).
        let index = RuleIndex::build(mini_corpus()).expect("build");
        let a = index.suggest("fn calc(a: i32) -> i32 { return a + 1; }", 3);
        let b = index.suggest("fn calc(a: i32) -> i32 { return a + 1; }", 3);
        assert_eq!(a, b);
    }
}
