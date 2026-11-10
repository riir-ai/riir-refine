//! The ragplane vendoring parity pins (issue 221 P2b).
//!
//! Same discipline as `katplane_parity.rs`: the vendored substrate is the
//! SOURCE OF TRUTH for the matcher path, so the load-bearing upstream facts
//! are pinned HERE — a sibling re-vendor that changes them breaks THIS build
//! loudly, never silently moves the matcher's behavior.
//!
//! The full upstream test suites also run (they ship inside the vendored
//! files — 54 tests at the vendoring sha); these pins cover the constants
//! and contracts the fresh seam + the floors DEPEND on, so a drift is a
//! named failure, not a mystery rank regression.

use riir_refine::ragplane::{Embedder, LATENT_DIM, ModellessEmbedder, Tokenizer, bm25, query};

/// The latent dimension the whole matcher pipeline assumes —
/// `NeuronShard.hla_moments`-shaped, `riir-agents`-matched. A drift here
/// changes every embedding and invalidates every pinned floor.
#[test]
pub fn latent_dim_is_the_pinned_8() {
    assert_eq!(LATENT_DIM, 8);
}

/// The embedder's committed model id (upstream `modelless-blake3-dft-sigmoid
/// @8d-v1`): deterministic, L2-normalized, empty-input zero vector. The
/// floors were pinned under THESE vectors; any change invalidates them.
#[test]
pub fn modelless_embedder_is_deterministic_normalized_and_zero_on_empty() {
    let e = ModellessEmbedder;

    let empty = e.embed("");
    assert_eq!(
        empty, [0.0; LATENT_DIM],
        "empty input must be the zero vector"
    );

    let a = e.embed("fn fix_capacity_in_loop(v: &mut Vec<u8>) { v.reserve(64); }");
    let b = e.embed("fn fix_capacity_in_loop(v: &mut Vec<u8>) { v.reserve(64); }");
    assert_eq!(a, b, "determinism: same input, same vector");

    let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-4 || norm == 0.0,
        "L2-normalized (or exactly zero): got {norm}"
    );
}

/// Two structurally different snippets embed to DIFFERENT directions —
/// the discriminating power the latent stage needs (weak, hence the BM25
/// fusion, but nonzero).
#[test]
pub fn modelless_embedder_discriminates_structures() {
    let e = ModellessEmbedder;
    let x = e.embed("for i in 0..n { if xs[i] > max { max = xs[i]; } }");
    let y = e.embed("let config: Result<Config, Error> = serde_json::from_str(s);");
    let dot: f32 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    assert!(
        dot < 0.999,
        "distinct structures must not collapse: dot={dot}"
    );
}

/// The BM25 index's contract the fusion relies on: search returns
/// (zone-key, score) pairs, empty index is silent, and a doc indexed under
/// a key surfaces when queried by its term.
#[test]
pub fn bm25_index_search_round_trip() {
    let idx = bm25::Bm25Index::new();
    idx.index(
        b"rule:clippy:redundant_clone",
        "remove the redundant clone of the value",
    );
    idx.index(b"rule:perf:uninit_vec", "replace zeroed vec with uninit");

    let hits = idx.search("redundant clone", 5);
    assert!(!hits.is_empty(), "the indexed doc must surface");
    assert!(
        hits.iter()
            .any(|(k, _)| k.as_slice() == b"rule:clippy:redundant_clone"),
        "the right key surfaces: {hits:?}"
    );
    assert!(
        hits.iter()
            .all(|(k, _)| k.as_slice() != b"rule:perf:uninit_vec"),
        "the non-matching doc must not outrank into the query's terms"
    );

    let miss = idx.search("totally_unrelated_token_zzz", 5);
    assert!(miss.is_empty(), "no term match → no hits");
}

/// The AST chunker's contract: a Rust file chunks at fn boundaries, and
/// each chunk carries source provenance the zone-hash identity is computed
/// from.
#[test]
pub fn ast_chunker_splits_at_item_boundaries() {
    use riir_refine::ragplane::chunker::Chunker as _;
    let src = "fn alpha() { let x = 1; }\nfn beta() { let y = 2; }\n";
    let chunks = riir_refine::ragplane::ast_chunker::AstChunker.chunk(src, "src/lib.rs");
    assert!(
        chunks.len() >= 2,
        "two fns must chunk apart: {}",
        chunks.len()
    );
    assert!(
        chunks.iter().all(|c| !c.content.is_empty()),
        "no empty chunks"
    );
}

/// The Tokenizer trait's surface the RustOpTokenizer impl binds against —
/// present, Send + Sync, and the CodeTokenizer default splits identifiers.
#[test]
pub fn code_tokenizer_splits_identifiers() {
    let toks = bm25::CodeTokenizer.tokenize("redundant_clone fix_capacity");
    assert!(toks.len() >= 2);
}

/// The query vocabulary's builder contract the matcher consumes.
#[test]
pub fn latent_query_carries_text_symbols_and_direction() {
    let q = query::LatentQuery::from_direction([0.25; LATENT_DIM])
        .with_text("remove redundant clone")
        .with_symbol("Vec::clone");
    assert_eq!(q.direction, [0.25; LATENT_DIM]);
    assert_eq!(q.text.as_deref(), Some("remove redundant clone"));
    assert_eq!(q.symbols, vec!["Vec::clone".to_string()]);
}
