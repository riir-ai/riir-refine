//! The modelless retrieval substrate — VENDORED from `riir-neuron-db`
//! (issue 221 P2b, the publish posture's matcher path).
//!
//! One home for the five vendored files (see each file's provenance header):
//! [`bm25`] — the production lexical index; [`embedder`] — the modelless
//! 8-D embedder + the [`Embedder`]/[`embedder::DenseEmbedder`] traits;
//! [`query`] — the latent-first query/chunk vocabulary; [`chunker`] +
//! [`ast_chunker`] — the chunking halves.
//!
//! The FRESH seam (the `NeuronDbRetriever` replacement — brute-force exact
//! cosine + the vendored BM25 fused query) lives beside these in the
//! draft matcher's consumer, not here: this module is the substrate, the
//! seam is the consumer-side composition (kept next to the corpus index it
//! serves, so the two land and re-pin together).

#[path = "ast_chunker.rs"]
pub mod ast_chunker;
#[path = "bm25.rs"]
pub mod bm25;
#[path = "chunker.rs"]
pub mod chunker;
#[path = "embedder.rs"]
pub mod embedder;
#[path = "math.rs"]
pub mod math;
#[path = "query.rs"]
pub mod query;

pub use bm25::Tokenizer;
pub use embedder::{Embedder, ModellessEmbedder};
pub use math::{desc, sigmoid};
pub use query::{ChunkSource, LatentQuery, RetrievedChunk, LATENT_DIM};
