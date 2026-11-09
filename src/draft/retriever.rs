//! The in-repo fused retriever — the matcher path's ONE retrieval primitive
//! (issue 221 P2b).
//!
//! Replaces the substrate `NeuronDbRetriever` on the rule-matcher path: the
//! crates.io publish posture carries zero private deps, and the verdict
//! (issue 221 round-2 D1) made this the ONLY retrieval path — the vendored
//! [`crate::ragplane`] substrate below, this seam above, never both. The
//! scoring semantics mirror the substrate's `retrieve_for_scoring_into`:
//!
//! ```text
//! stage 1  latent  — cosine(query.direction, row.dir), top-`LATENT_K`
//! stage 1b BM25    — the vendored [`Bm25Index`] over `query.symbols` + text,
//!                    augmenting exact_score on latent hits (new rows for
//!                    BM25-only hits)
//! dense   (opt-in) — 768-D cosine rerank, sort by dense_score falling back
//!                    to latent_score (the substrate `rerank_dense` order)
//! ```
//!
//! # Deliberate divergence (disclosed)
//!
//! The substrate's stage 1 selected candidates by hull-diverse greedy
//! (`ShardIndex::retrieve_diverse` — a Clifford-wedge convex-hull primitive).
//! Re-implementing that here would duplicate substrate machinery behind a
//! second vocabulary, so this seam takes **exact cosine top-k** instead. The
//! T3.5 sweep measured the 8-D latent term never changing a ranking at
//! corpus scale (BM25 dominates; latent-only hits never win the fused
//! score), and the acceptance gate for the swap is the retrieval floors
//! (`retrieval_precision_regression_gate`) re-run under this seam — floors
//! hold ⇒ the swap stands; floors decay ⇒ rework, never a silent rank
//! regression. The substrate's `two_stage_oversample` knob has no analogue
//! (brute force is already O(N) with no approximation stage) and
//! `RetrievalConfig` no longer carries it.
//!
//! # Keying
//!
//! The BM25 documents are keyed by the raw zone hash bytes (no `rag:chunk:`
//! prefix — the substrate's `LocalKvStore` content keying is not needed;
//! this seam stores no content, mirroring the score-only packed path). The
//! zone hash itself is the SAME BLAKE3(content) the substrate computed, so
//! the corpus index's `zone_hash → rule_id` sidecar is untouched.
//!
//! # Determinism
//!
//! Stage 1 sorts with `total_cmp` and a row-index tie-break; BM25 inherits
//! the vendored index's own order; the dense sort uses the vendored
//! [`crate::ragplane::math::desc`] total order — the same comparator the
//! substrate sorted with. No `HashMap` iteration ever reaches the output.

use std::collections::HashMap;

use crate::ragplane::bm25::{Bm25Index, Tokenizer};
use crate::ragplane::chunker::RawChunk;
use crate::ragplane::embedder::Embedder;
use crate::ragplane::math::desc;
use crate::ragplane::query::{ChunkSource, LatentQuery, RetrievedChunk, LATENT_DIM};

/// Top-K for the latent cosine stage. The substrate's `DEFAULT_LATENT_K`.
const LATENT_K: usize = 16;

/// Top-K for the BM25 exact stage. The substrate's `DEFAULT_BM25_K`.
const BM25_K: usize = 16;

/// One indexed chunk: the zone hash (BLAKE3 of the content), the 8-D latent
/// direction, and (when the dense path indexed it) the dense direction.
struct IndexedChunk {
    zone_hash: [u8; 32],
    dir: [f32; LATENT_DIM],
    #[cfg(feature = "dense_embed")]
    dense: Option<Vec<f32>>,
}

/// The fused retriever over an indexed chunk set. Cheap to build, owned by
/// the per-domain [`crate::draft::corpus_index::RuleIndex`].
pub(crate) struct RuleRetriever {
    rows: Vec<IndexedChunk>,
    /// `zone_hash → row index` (first insertion wins; identical content
    /// hashes to the identical zone hash, so this is the substrate's upsert).
    by_hash: HashMap<[u8; 32], usize>,
    bm25: Bm25Index,
}

/// Caller-owned scratch for the scoring loop (the substrate
/// `retriever::RetrievalScratch` replacement — same reuse law: pass the SAME
/// scratch across calls; every field is cleared at entry, capacity retained,
/// steady-state allocation zero after warmup).
#[derive(Default)]
pub(crate) struct RetrievalScratch {
    /// The candidate list (the seam's output view).
    candidates: Vec<RetrievedChunk>,
    /// `zone_hash → candidates` position (dedup + BM25 augment).
    by_zone: HashMap<[u8; 32], usize>,
    /// Stage-1 cosine buffer (score, row) — cleared + refilled per call.
    cos: Vec<(f32, usize)>,
    /// BM25 query texts (symbols + the optional text), reused per call.
    bm25_queries: Vec<String>,
}

impl RuleRetriever {
    /// An empty retriever with the substrate-default BM25 tokenizer
    /// ([`CodeTokenizer`] — `Bm25Index::new`'s own default).
    pub(crate) fn new() -> Self {
        Self {
            rows: Vec::new(),
            by_hash: HashMap::new(),
            bm25: Bm25Index::new(),
        }
    }

    /// Swap the BM25 tokenizer. MUST precede `index_chunks` — the tokenizer
    /// is used at both index and search time, and swapping it after indexing
    /// silently desyncs the postings from the queries (the substrate
    /// contract, restated: `with_bm25_tokenizer` there carried the same
    /// requirement).
    pub(crate) fn with_bm25_tokenizer(mut self, tokenizer: Box<dyn Tokenizer + Send + Sync>) -> Self {
        self.bm25 = Bm25Index::with_tokenizer(tokenizer);
        self
    }

    /// Index `chunks` with the 8-D embedder (the modelless path). Returns the
    /// number of rows in the index after the batch (duplicates of
    /// already-indexed content are upserted, mirroring the substrate's
    /// idempotent re-index). Infallible today (the substrate's `Result` came
    /// from its `LocalKvStore` writes, which this seam does not make); the
    /// call sites' `?` shape is kept so growing an error path later does not
    /// churn the corpus index.
    pub(crate) fn index_chunks(&mut self, chunks: Vec<RawChunk>, embedder: &dyn Embedder) -> usize {
        for chunk in chunks {
            let zone_hash = *blake3::hash(chunk.content.as_bytes()).as_bytes();
            let dir = embedder.embed(&chunk.content);
            self.insert_row(IndexedChunk {
                zone_hash,
                dir,
                #[cfg(feature = "dense_embed")]
                dense: None,
            });
            // Key = the raw zone-hash bytes (see the module's keying note).
            self.bm25.index(&zone_hash, &chunk.content);
        }
        self.rows.len()
    }

    /// Index `chunks` with BOTH the 8-D latent and the dense embeddings —
    /// ONE `embed_both` pass per chunk (the substrate's dual-indexing path,
    /// kept for the Bonsai rerank arm's single-forward-pass law). The
    /// embedding SOURCE is the dep-side `DualEmbedder` (what
    /// `riir_rag_bonsai::BonsaiEmbedder` implements — the vendored trait set
    /// has no foreign impls); the vectors are STORED in this seam's own
    /// rows, so the fused retrieval stays in-repo.
    ///
    /// A chunk whose dense vector length differs from the embedder's own
    /// `dense_dim` keeps its latent row and no dense vector (the substrate
    /// skipped the dense insert on the same mismatch). Infallible today (see
    /// [`index_chunks`](Self::index_chunks)).
    #[cfg(feature = "bonsai_retrieval")]
    pub(crate) fn index_chunks_dense(
        &mut self,
        chunks: Vec<RawChunk>,
        embedder: &dyn riir_rag::embedder::DualEmbedder,
    ) -> usize {
        for chunk in chunks {
            let zone_hash = *blake3::hash(chunk.content.as_bytes()).as_bytes();
            let (dir, dense_vec) = embedder.embed_both(&chunk.content);
            let dense = if dense_vec.len() == embedder.dense_dim() {
                Some(dense_vec)
            } else {
                None
            };
            self.insert_row(IndexedChunk {
                zone_hash,
                dir,
                dense,
            });
            self.bm25.index(&zone_hash, &chunk.content);
        }
        self.rows.len()
    }

    /// Distinct indexed chunks (post-dedup).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    fn insert_row(&mut self, row: IndexedChunk) {
        if let Some(existing) = self.by_hash.get(&row.zone_hash) {
            // Same content re-indexed: refresh the row in place (the
            // substrate's upsert) — identical content ⇒ identical vectors,
            // but a re-embed keeps the stored state fresh regardless.
            self.rows[*existing] = row;
        } else {
            self.by_hash.insert(row.zone_hash, self.rows.len());
            self.rows.push(row);
        }
    }

    /// Retrieve scored candidates for `query` into `scratch` (cleared at
    /// entry) and return the candidate list. Score-only packed path: no
    /// content or source is populated (the corpus index reads
    /// `zone_hash` + scores only).
    pub(crate) fn retrieve_for_scoring_into<'a>(
        &self,
        query: &LatentQuery,
        scratch: &'a mut RetrievalScratch,
    ) -> &'a mut Vec<RetrievedChunk> {
        scratch.candidates.clear();
        scratch.by_zone.clear();

        // ── Stage 1: latent cosine top-k ────────────────────────────────────
        // `query.latent_scan == false` skips this stage ENTIRELY (the
        // Issue-643 exact latent-skip shape; stage 1b below is unchanged).
        if query.latent_scan && !self.rows.is_empty() {
            scratch.cos.clear();
            for (i, row) in self.rows.iter().enumerate() {
                let score = cosine(&query.direction, &row.dir);
                scratch.cos.push((score, i));
            }
            // Score desc, row asc on ties — total order, no drift.
            scratch.cos.sort_unstable_by(|a, b| desc(b.0, a.0).then(a.1.cmp(&b.1)));
            for &(score, i) in scratch.cos.iter().take(LATENT_K) {
                let row = &self.rows[i];
                let pos = scratch.candidates.len();
                scratch.by_zone.insert(row.zone_hash, pos);
                scratch.candidates.push(RetrievedChunk {
                    zone_hash: row.zone_hash,
                    content: None,
                    source: ChunkSource::new("", 0, 0),
                    latent_score: score,
                    exact_score: None,
                    graph_score: None,
                    stale: false,
                    #[cfg(feature = "dense_embed")]
                    dense_score: None,
                });
            }
        }

        // ── Stage 1b: BM25 exact fallback ───────────────────────────────────
        // Per query symbol + the optional text; augment `exact_score` on a
        // chunk already found via the latent path, new rows for BM25-only
        // hits (the substrate's stage shape, byte-for-byte).
        scratch.bm25_queries.clear();
        scratch.bm25_queries.extend(query.symbols.iter().cloned());
        if let Some(text) = &query.text {
            scratch.bm25_queries.push(text.clone());
        }
        for q in &scratch.bm25_queries {
            for (key, score) in self.bm25.search(q, BM25_K) {
                let Ok(bytes) = <[u8; 32]>::try_from(key.as_slice()) else {
                    continue;
                };
                let Some(&row_idx) = self.by_hash.get(&bytes) else {
                    continue;
                };
                if let Some(&pos) = scratch.by_zone.get(&bytes) {
                    if scratch.candidates[pos].exact_score.is_none() {
                        scratch.candidates[pos].exact_score = Some(score);
                    }
                } else {
                    let row = &self.rows[row_idx];
                    let pos = scratch.candidates.len();
                    scratch.by_zone.insert(bytes, pos);
                    scratch.candidates.push(RetrievedChunk {
                        zone_hash: row.zone_hash,
                        content: None,
                        source: ChunkSource::new("", 0, 0),
                        latent_score: 0.0,
                        exact_score: Some(score),
                        graph_score: None,
                        stale: false,
                        #[cfg(feature = "dense_embed")]
                        dense_score: None,
                    });
                }
            }
        }

        // ── Dense rerank (opt-in) ───────────────────────────────────────────
        // The substrate `rerank_dense` shape: 768-D cosine per candidate that
        // carries a dense vector, then sort by dense_score falling back to
        // latent_score (dense-reranked hits at the top, latent-only fallback
        // below). The fused-score merge downstream reads `dense_score` off
        // the rows (gated), so the sort AND the score contribution both ride.
        #[cfg(feature = "dense_embed")]
        if let Some(query_dense) = &query.dense_direction {
            for cand in &mut scratch.candidates {
                if let Some(&row_idx) = self.by_hash.get(&cand.zone_hash) {
                    if let Some(dense) = &self.rows[row_idx].dense {
                        if query_dense.len() == dense.len() {
                            cand.dense_score = Some(cosine_dense(query_dense, dense));
                        }
                    }
                }
            }
            scratch.candidates.sort_by(|a, b| {
                desc(
                    a.dense_score.unwrap_or(a.latent_score),
                    b.dense_score.unwrap_or(b.latent_score),
                )
            });
        }

        &mut scratch.candidates
    }
}

impl Default for RuleRetriever {
    fn default() -> Self {
        Self::new()
    }
}

/// Cosine similarity for two 8-D vectors — the substrate's `cosine` shape
/// verbatim (zero vectors return 0).
fn cosine(a: &[f32; LATENT_DIM], b: &[f32; LATENT_DIM]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-10 || nb < 1e-10 {
        return 0.0;
    }
    dot / (na * nb)
}

/// Cosine similarity for dense (variable-length) vectors — the substrate's
/// `cosine_dense` shape (used by the rerank stage).
#[cfg(feature = "dense_embed")]
fn cosine_dense(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-10 || nb < 1e-10 {
        return 0.0;
    }
    dot / (na * nb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ragplane::embedder::ModellessEmbedder;
    use crate::ragplane::query::LatentQuery;

    fn corpus() -> Vec<(&'static str, &'static str)> {
        vec![
            ("needless_return", "fn compute(x: i32) -> i32 { return x * 2; }"),
            ("redundant_closure", "let names: Vec<String> = people.iter().map(|p| fmt(p)).collect();"),
            ("lookup_loop", "fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n    for u in users {\n        if u.id == id { return Some(u); }\n    }\n    None\n}"),
        ]
    }

    fn build() -> RuleRetriever {
        let mut r = RuleRetriever::new();
        let chunks = corpus()
            .into_iter()
            .map(|(id, text)| RawChunk {
                source: ChunkSource::new(format!("rule:{id}"), 0, 0),
                content: text.to_string(),
            })
            .collect::<Vec<_>>();
        r.index_chunks(chunks, &ModellessEmbedder);
        r
    }

    #[test]
    fn indexes_and_dedups_by_content() {
        let mut r = build();
        assert_eq!(r.len(), 3);
        // Identical content re-indexed → upsert, not a second row.
        let dup = vec![RawChunk {
            source: ChunkSource::new("again", 0, 0),
            content: "fn compute(x: i32) -> i32 { return x * 2; }".to_string(),
        }];
        r.index_chunks(dup, &ModellessEmbedder);
        assert_eq!(r.len(), 3, "identical content must upsert");
    }

    #[test]
    fn bm25_leg_finds_the_rule_the_latent_leg_may_miss() {
        let r = build();
        let mut scratch = RetrievalScratch::default();
        let query = LatentQuery::from_direction([0.0; LATENT_DIM])
            .with_text("for u in users { if u.id == id { return Some(u); } }");
        let out = r.retrieve_for_scoring_into(&query, &mut scratch);
        assert!(
            out.iter().any(|c| c.exact_score.is_some()),
            "the BM25 leg must surface an exact hit"
        );
    }

    #[test]
    fn latent_scan_false_answers_from_bm25_alone() {
        let r = build();
        let mut scratch = RetrievalScratch::default();
        let query = LatentQuery::from_direction([1.0; LATENT_DIM])
            .with_latent_scan(false)
            .with_text("fn compute");
        let out = r.retrieve_for_scoring_into(&query, &mut scratch);
        assert!(
            out.iter().all(|c| c.latent_score == 0.0),
            "latent-skip: no row may carry a latent score"
        );
    }

    #[test]
    fn candidates_are_deterministic_across_calls() {
        let r = build();
        let query = LatentQuery::from_direction(ModellessEmbedder.embed("fn area(w: f64, h: f64) -> f64 { return w * h; }"))
            .with_text("return w * h;");
        let mut s1 = RetrievalScratch::default();
        let mut s2 = RetrievalScratch::default();
        let a = r.retrieve_for_scoring_into(&query, &mut s1).clone();
        let b = r.retrieve_for_scoring_into(&query, &mut s2).clone();
        assert_eq!(a, b, "same query, same scratch shape → identical rows");
    }

    #[test]
    fn scratch_reuse_stabilizes_capacities() {
        let r = build();
        let mut scratch = RetrievalScratch::default();
        let query = LatentQuery::from_direction([0.5; LATENT_DIM]).with_text("for u in users");
        let c1 = scratch.candidates.capacity();
        for _ in 0..4 {
            r.retrieve_for_scoring_into(&query, &mut scratch);
        }
        assert!(
            scratch.candidates.capacity() <= c1.max(16) * 2,
            "steady-state growth must stay bounded"
        );
    }
}
