//! Vendored from `riir-neuron-db` (`a020ace2a6ee`, vendored 2026-11-09, issue
//! 221 P2b — the publish posture's modelless retrieval substrate).
//! std-only up to the crate's own deps (blake3 / papaya / quote — all
//! already in refine's tree). The canonical copy stays in the private
//! sibling; drift is caught by `scripts/vendor_ragplane.sh --check`
//! and the pins in `tests/ragplane_parity.rs` — never hand-edit one
//! side. Upstream-internal paths rewritten: crate::query ->
//! crate::ragplane::query, crate::chunker -> crate::ragplane::chunker.
//!
//! LINT POSTURE (vendored-code disclosure): upstream never carried
//! refine's clippy -D bar (lib OR --all-targets, where the vendored test
//! modules also get graded), so the divergent lint classes are allowed at
//! file scope rather than hand-patched — the cast-precision findings are
//! INHERENT to the upstream algorithms (fixing them would change the
//! pinned vectors/floors), the doc-backtick and style classes are churn
//! with zero behavior content. The parity pins + the upstream tests that
//! ship inside each file are the correctness authority, not lint shape.
#![allow(
    clippy::doc_markdown,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::missing_const_for_fn,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::missing_fields_in_debug,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::inline_always,
    clippy::let_and_return,
    clippy::manual_let_else,
    clippy::explicit_iter_loop,
    clippy::match_same_arms,
    clippy::single_char_pattern,
    clippy::format_push_string,
    clippy::ignore_without_reason,
    clippy::assertions_on_constants,
    clippy::too_many_lines,
    clippy::assert_is_empty
)]

//! The latent-first trait surface — what `riir-agents` consumes.
//!
//! The primary retrieval key is the latent direction (`[f32; 8]`). Text +
//! symbols are SECONDARY (BM25 fallback + embedder input). This is "latent RAG
//! inside latent space" — the retrieval itself operates on latent vectors, not
//! raw text.

/// The latent dimension. 8 — matches `NeuronShard.hla_moments` and
/// `riir-agents::LATENT_DIM`. Small enough for L1-cache-resident dot products;
/// expressive enough for diverse chunk retrieval.
pub const LATENT_DIM: usize = 8;

/// The latent query — the latent-first entry point.
///
/// The PRIMARY key is the latent direction (matches `NeuronShard.hla_moments`).
/// Text + symbols are SECONDARY (BM25 fallback + embedder input).
#[derive(Debug, Clone, PartialEq)]
pub struct LatentQuery {
    /// The latent direction — the primary retrieval key.
    pub direction: [f32; LATENT_DIM],
    /// Optional text — for BM25 fallback + display.
    pub text: Option<String>,
    /// Optional exact symbols — for BM25 exact match (e.g. `my_crate::FooBar`).
    pub symbols: Vec<String>,
    /// Optional dense (768-D) direction for `DenseEmbedIndex` rerank (Plan 524
    /// Phase 7 / Issue 596). When present AND the retriever has a
    /// `DenseEmbedIndex` attached, the 8-D latent candidates are reranked by
    /// 768-D cosine similarity. `None` for the modelless path (the default).
    #[cfg(feature = "dense_embed")]
    pub dense_direction: Option<Vec<f32>>,
    /// Issue 643 — the latent-leg-skip knob. `true` (the default, the
    /// incumbent) runs the stage-1 diverse latent scan;
    /// `false` skips it and answers from stage 1b (BM25) alone. Additive:
    /// every other field/behavior is untouched, and `true` is byte-identical
    /// to the pre-643 query.
    pub latent_scan: bool,
}

impl LatentQuery {
    /// Construct a latent-only query (no text/symbol fallback).
    #[must_use]
    pub fn from_direction(direction: [f32; LATENT_DIM]) -> Self {
        Self {
            direction,
            text: None,
            symbols: Vec::new(),
            #[cfg(feature = "dense_embed")]
            dense_direction: None,
            latent_scan: true,
        }
    }

    /// Issue 643 — set the latent-leg-skip knob (`false` = answer from the
    /// BM25 leg alone; see [`Self::latent_scan`]).
    #[must_use]
    pub fn with_latent_scan(mut self, latent_scan: bool) -> Self {
        self.latent_scan = latent_scan;
        self
    }

    /// Construct a latent + text query. The text is used for BM25 fallback +
    /// as embedder input (if the consumer re-embeds it for a model-based path).
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Add an exact symbol for BM25 fallback (e.g. `my_crate::FooBar`).
    #[must_use]
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbols.push(symbol.into());
        self
    }

    /// Add a dense (768-D) direction for `DenseEmbedIndex` rerank (Plan 524
    /// Phase 7 / Issue 596). When set AND the retriever has a `DenseEmbedIndex`,
    /// candidates are reranked by 768-D cosine. No-op when `dense_embed`
    /// feature is off.
    #[cfg(feature = "dense_embed")]
    #[must_use]
    pub fn with_dense_direction(mut self, dense: Vec<f32>) -> Self {
        self.dense_direction = Some(dense);
        self
    }
}

// / A retrieved chunk — content + provenance + scores.
///
/// Content is loaded LAZILY (Issue 602 — lazy content loading for two-stage
/// retrieval). During `retrieve_candidates`, only `zone_hash` + scores are
/// populated; `content` stays `None`. The packer loads content on-demand for
/// chunks it actually selects (greedy budget fit), cutting content loads
/// from `latent_k` (typically 100) to ~10 (the final packed set) — a ~10×
/// reduction in String allocations + KV lookups per query.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievedChunk {
    /// The BLAKE3 zone_hash identifying this chunk in the `LocalKvStore` +
    /// `DenseEmbedIndex`. Always set during retrieval. Used by the dense
    /// rerank stage to look up the 768-D embedding without re-hashing the
    /// content (Issue 602 T2 — eliminates the wasteful `blake3::hash(content)`
    /// in `rerank_dense`).
    pub zone_hash: [u8; 32],
    /// The chunk content. `None` during candidate retrieval; loaded lazily
    /// by the packer for the final selected subset (Issue 602).
    pub content: Option<String>,
    /// Source provenance (file path, line range).
    pub source: ChunkSource,
    /// Latent similarity score (dot-product or Clifford wedge L1).
    pub latent_score: f32,
    /// BM25 exact-match score (`None` if retrieved via latent path only).
    pub exact_score: Option<f32>,
    /// GraphRAG proximity score (`None` when `graph_rag` is off OR the chunk
    /// has no graph-neighbor relationship to a query entity). Inverse-
    /// sigmoid of the BFS hop distance to the nearest query entity in the
    /// `KgTripleIndex`: distance 0 → ~1.0, distance k → `sigmoid(-λ*k)`.
    /// See `GraphRagConfig::graph_decay_lambda`. Added by Plan 526.
    pub graph_score: Option<f32>,
    /// **Stale evidence marker** (Issue 827 T4). `true` when the chunk's
    /// `zone_hash` resolves to a *superseded* entry in the attached
    /// `ChunkClaimLedger` — i.e. the document that produced this text has
    /// since been re-indexed and no longer contains it. The content is still
    /// readable from the store, so the chunk is still *citable*; it is just no
    /// longer *current*, and a caller that renders it as present-tense fact is
    /// citing dead text.
    ///
    /// Always `false` when no ledger is attached (feature `chunk_claim` off,
    /// or attached-but-empty) — an un-tracked corpus must not have every
    /// citation flagged. The field is deliberately NOT feature-gated: it is a
    /// plain `bool` with no gated dependency, and a `cfg`-gated public field
    /// on a struct with 20 literal construction sites is precisely the
    /// compile-break class that Issue 830 spent 99 errors on.
    pub stale: bool,
    /// Dense (768-D) cosine similarity score from `DenseEmbedIndex` rerank
    /// (Plan 524 Phase 7 / Issue 596). `None` when `dense_embed` is off,
    /// no `DenseEmbedIndex` is attached, or the query has no dense direction.
    #[cfg(feature = "dense_embed")]
    pub dense_score: Option<f32>,
}

/// Source provenance for a chunk (file path + line range).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSource {
    /// File path (or `"<text>"` for ad-hoc strings).
    pub path: String,
    /// Starting line number (1-based, inclusive).
    pub start_line: usize,
    /// Ending line number (1-based, inclusive).
    pub end_line: usize,
}

impl ChunkSource {
    /// Construct a source from a path + 1-based inclusive line range.
    #[must_use]
    pub fn new(path: impl Into<String>, start_line: usize, end_line: usize) -> Self {
        Self {
            path: path.into(),
            start_line,
            end_line,
        }
    }
}

/// The retrieval provider — what `riir-agents` consumes.
///
/// One method: retrieve relevant chunks given a latent query + budget.
/// Implementations:
/// - [`NeuronDbRetriever`](crate::NeuronDbRetriever) (default, feature `neuron_db`)
///   — composes `ShardIndex::retrieve_diverse` (Clifford wedge) + `Bm25Index`
///   (exact symbol) + `LocalKvStore` (chunk content).
/// - Mock providers in tests (the trait is the seam).
pub trait RetrievalProvider: Send + Sync {
    /// Retrieve relevant chunks given a latent query + token budget.
    ///
    /// Returns chunks ordered by relevance (latent_score + exact_score
    /// weighted — see [`TokenBudgetPacker`](crate::TokenBudgetPacker)). The
    /// caller (riir-agents) fits them into the model context window.
    fn retrieve(
        &self,
        query: &LatentQuery,
        budget: crate::ragplane::query::TokenBudget,
    ) -> Vec<RetrievedChunk>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latent_query_from_direction_defaults() {
        let q = LatentQuery::from_direction([0.1; LATENT_DIM]);
        assert_eq!(q.direction, [0.1; LATENT_DIM]);
        assert!(q.text.is_none());
        assert!(q.symbols.is_empty());
    }

    #[test]
    fn latent_query_builder_chain() {
        let q = LatentQuery::from_direction([0.2; LATENT_DIM])
            .with_text("find error handler")
            .with_symbol("my_crate::handle_error");
        assert_eq!(q.text.as_deref(), Some("find error handler"));
        assert_eq!(q.symbols, vec!["my_crate::handle_error"]);
    }

    #[test]
    fn chunk_source_new() {
        let s = ChunkSource::new("src/main.rs", 10, 25);
        assert_eq!(s.path, "src/main.rs");
        assert_eq!(s.start_line, 10);
        assert_eq!(s.end_line, 25);
    }
}

/// The token budget a retrieval may spend (vendored from upstream packer.rs
/// with the type — the packer itself stays sibling-side; the vendored
/// `RetrievalProvider` trait names it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenBudget(pub usize);
