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

//! Embedding — converts text to a latent direction `[f32; 8]`.
//!
//! Modelless default: [`ModellessEmbedder`] (BLAKE3 + DFT + sigmoid projections,
//! deterministic, no weights). Model-based (opt-in, post-C13):
//! `KimiK3Embedder` (mean-pooled hidden states).
//!
//! ## Modelless embedding signals
//!
//! The `ModellessEmbedder` must produce a meaningful `[f32; 8]` direction
//! without trained weights. Inspired by `riir-auth::SessionFingerprint`
//! (5-signal modelless fingerprint in `[f32; 8]`):
//!
//! | Signal | What it captures | Projection to f32 |
//! |---|---|---|
//! | BLAKE3 hash buckets | Deterministic identity (exact-match signal) | Split `BLAKE3(text)` into 8 × 4-byte chunks → normalize to `[-1, 1]` |
//! | Token frequency DFT | Repetitive structural patterns (loops, imports) | DFT of top-N token frequency vector → first 8 coefficients |
//! | Symbol density | Identifier-heavy vs literal-heavy code | `n_identifiers / n_tokens` → sigmoid |
//! | Nesting depth | Control-flow complexity (deeply nested vs flat) | `max_depth / expected_max` → sigmoid |
//! | Line length entropy | Formatting style (compact vs verbose) | Shannon entropy of line lengths → sigmoid |
//!
//! Each signal produces a scalar; the 8-D direction is the concatenation +
//! sigmoid gating. The result is a deterministic, BLAKE3-committable
//! fingerprint that captures *structural* similarity. It will NOT capture deep
//! semantic similarity (that's the KimiK3 embedder's job) — but it's the
//! honest modelless baseline.

use crate::ragplane::query::LATENT_DIM;
use blake3::Hasher;

/// Embedder — converts text to a latent direction `[f32; 8]`.
///
/// Modelless default: [`ModellessEmbedder`] (BLAKE3 + DFT + sigmoid, deterministic,
/// no weights). Model-based (opt-in; the concrete impl composes from `riir-rag-bonsai`, riir-ai): `BonsaiEmbedder`
/// (mean-pooled hidden states, Plan 524 Phase 7 / Issue 596).
///
/// The embedding feeds `NeuronShard.hla_moments` (the KNN-indexed field) at
/// index time, and constructs the `LatentQuery.direction` at query time.
///
/// ## Model identity (Issue 983)
///
/// Every embedder MUST expose a stable identity string. There is deliberately
/// NO default impl: a default would let a new embedder inherit somebody
/// else's identity, which is the exact defect this exists to prevent — a model
/// swap silently mixing two vector spaces in one index. The identity is
/// BLAKE3-hashed into `EmbedProvenance.model_id` at index time and compared
/// against the provenance sidecar's existing rows, refusing a batch whose
/// embedder differs from the one that built the index.
///
/// Identity is whatever the embedder deems behavior-changing: an algorithm
/// version constant (`ModellessEmbedder`), or a checkpoint-derived digest
/// (`BonsaiEmbedder` — file name + length + header geometry, NOT the type
/// name: two different GGUFs behind one type are two models).
///
/// `DenseEmbedder` deliberately carries no `model_id` of its own: every
/// dense-capable type implements `Embedder` too (the `DualEmbedder`
/// super-trait requires it), and a same-named method on BOTH traits would make
/// `dyn DualEmbedder` call sites ambiguous (forced UFCS). One identity per
/// embedder, exposed once, here.
pub trait Embedder: Send + Sync {
    /// Embed `text` into a latent direction `[f32; 8]`.
    fn embed(&self, text: &str) -> [f32; LATENT_DIM];

    /// The behavior-changing identity of this embedder (see trait docs).
    ///
    /// Must be stable for the lifetime of the embedder and distinct across
    /// embedders whose vectors must never be mixed in one index. Bump the
    /// string whenever the embedding algorithm or checkpoint changes what a
    /// stored vector MEANS.
    fn model_id(&self) -> &str;
}

/// The default embedder — deterministic BLAKE3 + DFT + sigmoid projections.
///
/// No trained weights. Always available (no feature gate). The permanent
/// modelless floor — model-based embedders (`KimiK3Embedder`, post-C13) must
/// BEAT this on retrieval quality (GOAT G5 gate, Plan 524 Phase 7) to justify
/// their latency + dep cost.
///
/// Deterministic: same text → same direction, BLAKE3-committable. Captures
/// structural similarity (token distribution, nesting, formatting) but NOT deep
/// semantic similarity. See the module docs for the signal inventory.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModellessEmbedder;

/// The modelless embedder's identity string (Issue 983).
///
/// The algorithm IS the model: BLAKE3 buckets + DFT + sigmoid projections,
/// 8-D. Bump the version tag whenever `embed_modelless` changes what a stored
/// vector means — a silently-changed projection is a model swap wearing the
/// same type name.
pub const MODELLESS_MODEL_ID: &str = "modelless-blake3-dft-sigmoid@8d-v1";

impl Embedder for ModellessEmbedder {
    #[inline]
    fn embed(&self, text: &str) -> [f32; LATENT_DIM] {
        embed_modelless(text)
    }

    #[inline]
    fn model_id(&self) -> &str {
        MODELLESS_MODEL_ID
    }
}

/// Dense embedder — produces a high-D semantic vector for `DenseEmbedIndex`.
///
/// Sibling to [`Embedder`] (8-D latent). Any type implementing both
/// [`Embedder`] + [`DenseEmbedder`] can drive the two-stage retrieval path
/// (8-D pre-filter → 768-D rerank). Currently only
/// `BonsaiEmbedder` (crate `riir-rag-bonsai`, riir-ai) implements
/// this; a future dedicated sentence-encoder impl would also fit.
///
/// Feature-gated (`dense_embed`) — the modelless path produces only 8-D
/// (the 768-D index is empty unless a `DenseEmbedder` populates it).
pub trait DenseEmbedder: Send + Sync {
    /// Embed `text` into a dense semantic vector.
    ///
    /// Returns a `Vec<f32>` of length [`Self::dense_dim`]. The vector SHOULD
    /// be L2-normalized so cosine similarity = dot product (what
    /// `DenseEmbedIndex` expects for brute-force KNN + rerank).
    fn embed_dense(&self, text: &str) -> Vec<f32>;

    /// The dimensionality of [`Self::embed_dense`] output.
    fn dense_dim(&self) -> usize;

    /// Late-chunking shape (ndb Issue 641 T2, opt-in
    /// `embed_contextual`): ONE logical pass over `document`, one vector
    /// per span, spans NON-OVERLAPPING and byte-offset char-boundary
    /// aligned ([`SpanRange`]).
    ///
    /// The intended model-based contract: a single forward pass produces
    /// position-level states and each span vector pools that shared state —
    /// the span embedding is CONDITIONED on the document (Jina late
    /// chunking, arXiv:2409.04701). The default impl is the correct-but-
    /// slow fallback: per-span [`Self::embed_dense`] calls (N passes).
    /// Implementations sharing hidden state across spans SHOULD override.
    #[cfg(feature = "embed_contextual")]
    fn embed_contextual(&self, document: &str, spans: &[SpanRange]) -> Vec<Vec<f32>> {
        spans
            .iter()
            .map(|s| self.embed_dense(&document[s.start..s.end]))
            .collect()
    }

    /// Whether [`Self::embed_contextual`] is a true shared-pass impl (one
    /// walk over the document) rather than the per-span fallback. Diagnostics
    /// only — callers never branch on it.
    #[cfg(feature = "embed_contextual")]
    fn contextual_shared_pass(&self) -> bool {
        false
    }
}

/// Combined 8-D + dense embedder — one forward pass → both embeddings.
///
/// Super-trait of [`Embedder`] + [`DenseEmbedder`]. The key method is
/// [`embed_both`](Self::embed_both), which produces BOTH the 8-D latent
/// direction AND the dense semantic vector from a single forward pass.
/// For models where the forward pass is the dominant cost (e.g.
/// `BonsaiEmbedder` (crate `riir-rag-bonsai`) at ~50 s/chunk
/// on M3 Max CPU), this is a **2× wall-clock win** over calling `embed()` +
/// `embed_dense()` separately (which would each re-run the forward pass).
///
/// The default impl calls `embed()` + `embed_dense()` separately — correct
/// but slow. Impl-ers that share the hidden state between both projections
/// SHOULD override `embed_both` to do one forward pass.
///
/// [`NeuronDbRetriever::index_chunks_dense`] consumes this trait (not the
/// two separate trait objects) precisely so the single-forward-pass win is
/// guaranteed at the API surface — a caller cannot accidentally pass two
/// unrelated embedders + pay double.
pub trait DualEmbedder: Embedder + DenseEmbedder {
    /// One forward pass → both 8-D latent + dense semantic embeddings.
    ///
    /// Default impl: two separate calls (`embed` + `embed_dense`). Override
    /// when the underlying model can share the hidden state between both
    /// projections (see [`BonsaiEmbedder::embed_both`]).
    fn embed_both(&self, text: &str) -> ([f32; LATENT_DIM], Vec<f32>) {
        (self.embed(text), self.embed_dense(text))
    }
}

/// Late-chunking span extension ([`DenseEmbedder::embed_contextual`]) —
/// ndb Issue 641 T2. Opt-in via the `embed_contextual` feature; with the
/// feature off the `dense_embed` surface is byte-identical to the pre-T2
/// trait.
#[cfg(feature = "embed_contextual")]
mod contextual {
    /// A byte-offset span within a document, for [`super::DenseEmbedder::embed_contextual`].
    ///
    /// Preconditions (validated by `debug_assert!` + the reference impl —
    /// slicing panics are the release-mode failure mode): `start <= end`,
    /// both are `char` boundaries of the document, and spans are sorted and
    /// NON-OVERLAPPING (late chunking's contract: one vector per disjoint
    /// piece of the document).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpanRange {
        /// Byte offset of the first byte of the span.
        pub start: usize,
        /// Byte offset one past the last byte of the span.
        pub end: usize,
    }

    impl SpanRange {
        /// Construct a span range.
        #[must_use]
        pub const fn new(start: usize, end: usize) -> Self {
            Self { start, end }
        }

        /// The span's length in bytes.
        #[must_use]
        pub const fn len(&self) -> usize {
            self.end - self.start
        }

        /// Whether the span covers no bytes.
        #[must_use]
        pub const fn is_empty(&self) -> bool {
            self.start == self.end
        }
    }
}
#[cfg(feature = "embed_contextual")]
pub use contextual::SpanRange;

/// Construct the modelless 8-D embedding of `text`.
///
/// The 8 dimensions carry 5 structural signals (see module docs):
/// - Dims 0-3: BLAKE3 hash buckets (4 × 4-byte chunks → [-1, 1]).
/// - Dim 4: token frequency DFT first-coefficient magnitude (structural rhythm).
/// - Dim 5: symbol density (identifier-heavy vs literal-heavy).
/// - Dim 6: nesting depth (control-flow complexity).
/// - Dim 7: line length entropy (formatting style).
///
/// The 8 scalars are concatenated + L2-normalized for stable dot products.
/// The result is deterministic for the same input (G1 gate).
pub fn embed_modelless(text: &str) -> [f32; LATENT_DIM] {
    if text.is_empty() {
        return [0.0; LATENT_DIM];
    }

    // ── Signal 1: BLAKE3 hash buckets → dims 0-3 ────────────────────────
    // Hash the full text once, split into 4 × 4-byte chunks → 4 scalars in [-1, 1].
    let mut hasher = Hasher::new();
    hasher.update(text.as_bytes());
    let hash = hasher.finalize(); // OutputReader → 32 bytes
    let hb = hash.as_bytes(); // 32 bytes
    let mut blake3_scalars = [0.0f32; 4];
    for i in 0..4 {
        let chunk = u32::from_le_bytes([hb[i * 4], hb[i * 4 + 1], hb[i * 4 + 2], hb[i * 4 + 3]]);
        // Map u32 → [-1, 1]: (val / u32::MAX) * 2 - 1.
        blake3_scalars[i] = ((chunk as f64 / u32::MAX as f64) * 2.0 - 1.0) as f32;
    }

    // ── Signal 2: token frequency DFT first-coefficient → dim 4 ─────────
    // A cheap "structural rhythm" signal: take the byte-value histogram
    // (256 bins), compute the magnitude of the first DFT coefficient.
    // Repetitive text (e.g. `use a; use b; use c;`) has a strong DC + low
    // harmonics; diverse text spreads energy across frequencies. We use
    // |X[1]| / (sum of histogram) for a normalized "rhythm" score.
    let mut hist = [0u32; 256];
    for &b in text.as_bytes() {
        hist[b as usize] = hist[b as usize].saturating_add(1);
    }
    let total_bytes = text.len() as f32;
    // First-coefficient magnitude (Goertzel-style for k=1): Re + Im of
    // Σ hist[b] * e^(i·2π·b/256). Using cos/sin tables would be heavier than
    // the signal warrants — approximate via a single accumulator each.
    let mut re = 0.0f32;
    let mut im = 0.0f32;
    for (b, &count) in hist.iter().enumerate() {
        let theta = 2.0 * std::f32::consts::PI * (b as f32) / 256.0;
        re += (count as f32) * theta.cos();
        im += (count as f32) * theta.sin();
    }
    let dft_mag = (re * re + im * im).sqrt() / total_bytes.max(1.0);
    // Sigmoid-gate to (0, 1) — most texts produce small magnitudes.
    let dft_scalar = crate::ragplane::math::sigmoid(dft_mag * 8.0);

    // ── Signal 3: symbol density → dim 5 ────────────────────────────────
    // Identifier-heavy code (lots of `[a-zA-Z_]`) vs literal-heavy.
    let n_ident = text
        .bytes()
        .filter(|c| c.is_ascii_alphabetic() || *c == b'_')
        .count();
    let density = n_ident as f32 / total_bytes.max(1.0);
    let density_scalar = crate::ragplane::math::sigmoid((density - 0.5) * 6.0);

    // ── Signal 4: nesting depth → dim 6 ────────────────────────────────
    // Max brace/paren depth (control-flow complexity). Capped at a sane max.
    let mut max_depth = 0i32;
    let mut cur_depth = 0i32;
    for b in text.as_bytes() {
        match *b {
            b'{' | b'(' | b'[' => {
                cur_depth += 1;
                if cur_depth > max_depth {
                    max_depth = cur_depth;
                }
            }
            b'}' | b')' | b']' if cur_depth > 0 => {
                cur_depth -= 1;
            }
            _ => {}
        }
    }
    let depth_scalar = crate::ragplane::math::sigmoid(max_depth as f32 / 6.0);

    // ── Signal 5: line length entropy → dim 7 ──────────────────────────
    // Shannon entropy of the distribution of line lengths. Compact code
    // (uniform line lengths) → low entropy; verbose code → high entropy.
    let mut line_lens: Vec<u32> = Vec::new();
    let mut cur_len = 0u32;
    for &b in text.as_bytes() {
        if b == b'\n' {
            line_lens.push(cur_len);
            cur_len = 0;
        } else {
            cur_len = cur_len.saturating_add(1);
        }
    }
    // Last line (no trailing newline).
    if cur_len > 0 || line_lens.is_empty() {
        line_lens.push(cur_len);
    }
    let entropy_scalar = if line_lens.is_empty() {
        0.0
    } else {
        // Build histogram of lengths (bucketed by 16 chars), compute H.
        let mut buckets = [0u32; 16];
        let total_lines = line_lens.len() as f32;
        for &l in &line_lens {
            let bucket = ((l as usize) / 16).min(15);
            buckets[bucket] += 1;
        }
        let mut h = 0.0f32;
        for &c in &buckets {
            if c > 0 {
                let p = c as f32 / total_lines;
                h -= p * p.ln();
            }
        }
        // H ∈ [0, ln(16) ≈ 2.77]. Sigmoid-map ln(16) → ~0.95.
        crate::ragplane::math::sigmoid((h - 1.0) * 2.0)
    };

    // ── Assemble + L2-normalize ────────────────────────────────────────
    let v = [
        blake3_scalars[0],
        blake3_scalars[1],
        blake3_scalars[2],
        blake3_scalars[3],
        dft_scalar,
        density_scalar,
        depth_scalar,
        entropy_scalar,
    ];
    normalize_l2(v)
}

/// L2-normalize a direction vector. Zero-vector returns zero (no direction).
pub fn normalize_l2(v: [f32; LATENT_DIM]) -> [f32; LATENT_DIM] {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-10 {
        return [0.0; LATENT_DIM];
    }
    let mut out = [0.0f32; LATENT_DIM];
    for i in 0..LATENT_DIM {
        out[i] = v[i] / norm;
    }
    out
}

/// The modelless CONTEXTUAL reference embedder (ndb Issue 641 T2, opt-in
/// `embed_contextual`): the deterministic floor for the late-chunking shape.
///
/// Each span's dense vector is **BLAKE3-XOF keyed on the span's own text** —
/// expanded to [`Self::DENSE_DIM`] f32s in [-1, 1] and L2-normalized. This
/// makes the delta-reindex identity a CONSTRUCTION, not a test hope: an
/// unchanged chunk keeps its key, hence a byte-identical vector, regardless
/// of what else changed in the document.
///
/// The trade this buys, stated honestly: the floor is CONTEXT-BLIND. A true
/// late-chunking model conditions each span vector on the whole document
/// (an edit anywhere shifts every vector); this reference cannot, because
/// the gate it serves — unchanged chunk → unchanged vector under reindex —
/// is exactly the property context-sensitivity would violate. Model-based
/// impls (`riir-rag-bonsai`) get context-sensitivity and are gated by
/// parity fixtures instead. What the floor DOES provide: the shared-pass
/// shape (one walk over the document, spans sliced from the same `&str`,
/// exactly-sized output buffers) and a stable floor the T1/T3 instruments
/// can diff any future contextual embedder against.
#[cfg(feature = "embed_contextual")]
#[derive(Debug, Clone, Copy, Default)]
pub struct ModellessContextualEmbedder;

#[cfg(feature = "embed_contextual")]
impl ModellessContextualEmbedder {
    /// The reference impl's dense width (distinct from Bonsai's 768 — this
    /// is the modelless floor's own dimensionality).
    pub const DENSE_DIM: usize = 256;

    /// The BLAKE3-XOF span vector: deterministic, L2-normalized, keyed on
    /// the span bytes alone.
    #[must_use]
    pub fn span_vector(span_bytes: &[u8]) -> Vec<f32> {
        let mut out = Vec::with_capacity(Self::DENSE_DIM);
        // Empty span → zero vector (the embed_modelless empty-text
        // convention: no content, no direction).
        if span_bytes.is_empty() {
            out.resize(Self::DENSE_DIM, 0.0);
            return out;
        }
        let mut reader = blake3::Hasher::new().update(span_bytes).finalize_xof();
        let mut bytes16 = [0u8; 16];
        for _ in 0..Self::DENSE_DIM / 4 {
            reader.fill(&mut bytes16);
            for w in bytes16.as_chunks::<4>().0 {
                let word = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
                // u32 → [-1, 1], the embed_modelless bucket mapping.
                out.push(((word as f64 / u32::MAX as f64) * 2.0 - 1.0) as f32);
            }
        }
        // L2-normalize (zero span → zero vector, no direction).
        let norm: f32 = out.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm >= 1e-10 {
            for x in &mut out {
                *x /= norm;
            }
        } else {
            out.fill(0.0);
        }
        out.shrink_to_fit();
        out
    }
}

#[cfg(feature = "embed_contextual")]
impl Embedder for ModellessContextualEmbedder {
    fn embed(&self, text: &str) -> [f32; LATENT_DIM] {
        embed_modelless(text)
    }

    fn model_id(&self) -> &str {
        MODELLESS_CONTEXTUAL_MODEL_ID
    }
}

#[cfg(feature = "embed_contextual")]
impl DenseEmbedder for ModellessContextualEmbedder {
    fn embed_dense(&self, text: &str) -> Vec<f32> {
        Self::span_vector(text.as_bytes())
    }

    fn dense_dim(&self) -> usize {
        Self::DENSE_DIM
    }

    fn embed_contextual(&self, document: &str, spans: &[SpanRange]) -> Vec<Vec<f32>> {
        // Preconditions: char boundaries, ordered, non-overlapping. Loud in
        // debug; slicing/overlap violations are caller bugs.
        debug_assert!(
            spans.windows(2).all(|w| w[0].end <= w[1].start),
            "spans must be sorted and non-overlapping"
        );
        debug_assert!(
            spans.iter().all(|s| s.end <= document.len()
                && document.is_char_boundary(s.start)
                && document.is_char_boundary(s.end)),
            "spans must be in-bounds char boundaries"
        );
        // Shared pass: ONE walk — every span slices the same document
        // borrow; each output vector is exactly-sized (with_capacity), no
        // intermediate allocations.
        let mut out = Vec::with_capacity(spans.len());
        for s in spans {
            out.push(Self::span_vector(&document.as_bytes()[s.start..s.end]));
        }
        out
    }

    fn contextual_shared_pass(&self) -> bool {
        true
    }
}

/// The behavior-changing identity of the modelless contextual floor (the
/// [`ModellessEmbedder::model_id`] convention — Issue 983). Bump when the
/// span-vector derivation changes what a stored vector means.
#[cfg(feature = "embed_contextual")]
pub const MODELLESS_CONTEXTUAL_MODEL_ID: &str = "modelless-contextual-blake3-xof@256d-v1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_is_deterministic() {
        let a = embed_modelless("fn main() { println!(\"hi\"); }");
        let b = embed_modelless("fn main() { println!(\"hi\"); }");
        assert_eq!(a, b, "same input must produce identical embedding");
    }

    #[test]
    fn empty_text_is_zero_vector() {
        let v = embed_modelless("");
        assert_eq!(v, [0.0; LATENT_DIM]);
    }

    #[test]
    fn different_texts_get_different_embeddings() {
        let a = embed_modelless("fn handle_error(e: Error) { ... }");
        let b = embed_modelless("struct UserAccount { id: u64 }");
        assert_ne!(a, b, "different texts should have different embeddings");
    }

    #[test]
    fn embedding_is_unit_normalized() {
        let v = embed_modelless("some code with content\nline two\nline three\n");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm should be 1.0, got {norm}");
    }

    #[test]
    fn deeper_nesting_increases_dim6() {
        // Deep nesting should produce a higher dim-6 (depth) value.
        let flat = embed_modelless("a = 1\nb = 2\nc = 3\n");
        let nested = embed_modelless("{{{{{{{{deeply nested code}}}}}}}}");
        // dim 6 is the depth scalar (pre-normalize). After normalize the
        // relationship may flip if other dims dominate, but for these two
        // extremes depth should win.
        assert!(
            nested[6] >= flat[6] - 1e-6,
            "nested depth dim6={} should be >= flat dim6={}",
            nested[6],
            flat[6]
        );
    }

    #[test]
    fn object_safety() {
        let e: Box<dyn Embedder> = Box::new(ModellessEmbedder);
        let v = e.embed("test");
        assert_eq!(v.len(), LATENT_DIM);
    }

    #[test]
    fn modelless_embedder_matches_free_function() {
        let e = ModellessEmbedder;
        let via_trait = e.embed("some text");
        let direct = embed_modelless("some text");
        assert_eq!(via_trait, direct);
    }

    // ── T2: the contextual floor (feature embed_contextual) ──────────
    #[cfg(feature = "embed_contextual")]
    mod contextual {
        use super::*;

        const DOC: &str = "first span text\nsecond span text\nthird span text\n";

        fn spans() -> [SpanRange; 3] {
            // Byte offsets of the three lines in DOC.
            let l1 = "first span text\n".len();
            let l2 = l1 + "second span text\n".len();
            let l3 = l2 + "third span text\n".len();
            [
                SpanRange::new(0, l1 - 1),
                SpanRange::new(l1, l2 - 1),
                SpanRange::new(l2, l3 - 1),
            ]
        }

        #[test]
        fn deterministic_same_inputs() {
            let e = ModellessContextualEmbedder;
            let a = e.embed_contextual(DOC, &spans());
            let b = e.embed_contextual(DOC, &spans());
            assert_eq!(a, b, "same (doc, spans) must be byte-identical");
        }

        #[test]
        fn delta_reindex_identity_unchanged_chunk_byte_identical() {
            // Two documents sharing span 2's bytes verbatim, differing
            // elsewhere: the shared chunk's vector must be byte-identical.
            let doc_a = "alpha neighbors\nshared chunk bytes\nother tail\n";
            let doc_b = "COMPLETELY different\nshared chunk bytes\ntail two\n";
            let shared = "shared chunk bytes";
            let sa = doc_a.find(shared).unwrap();
            let sb = doc_b.find(shared).unwrap();
            let e = ModellessContextualEmbedder;
            let va = &e.embed_contextual(doc_a, &[SpanRange::new(sa, sa + shared.len())])[0];
            let vb = &e.embed_contextual(doc_b, &[SpanRange::new(sb, sb + shared.len())])[0];
            assert_eq!(
                va, vb,
                "unchanged chunk must keep its vector across a reindex"
            );
        }

        #[test]
        fn span_vector_is_l2_normalized_and_dimmed() {
            let v = ModellessContextualEmbedder::span_vector(b"some span content");
            assert_eq!(v.len(), ModellessContextualEmbedder::DENSE_DIM);
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-4, "norm {norm}");
            let empty = ModellessContextualEmbedder::span_vector(b"");
            assert!(empty.iter().all(|x| *x == 0.0), "empty span → zero vector");
        }

        #[test]
        fn shared_pass_flag_and_trait_object() {
            let e = ModellessContextualEmbedder;
            assert!(e.contextual_shared_pass());
            let obj: &dyn DenseEmbedder = &e;
            assert!(obj.contextual_shared_pass());
            assert_eq!(obj.dense_dim(), ModellessContextualEmbedder::DENSE_DIM);
            assert_eq!(
                obj.embed_dense("x").len(),
                ModellessContextualEmbedder::DENSE_DIM
            );
        }

        #[test]
        fn default_trait_impl_is_per_span_fallback() {
            // A bare DenseEmbedder with NO override gets the per-span
            // fallback; its vectors match the reference's span vectors
            // (same derivation), flagged not-shared-pass.
            struct Bare;
            impl DenseEmbedder for Bare {
                fn embed_dense(&self, text: &str) -> Vec<f32> {
                    ModellessContextualEmbedder::span_vector(text.as_bytes())
                }
                fn dense_dim(&self) -> usize {
                    ModellessContextualEmbedder::DENSE_DIM
                }
            }
            let b = Bare;
            assert!(!b.contextual_shared_pass());
            let e = ModellessContextualEmbedder;
            let via_default = b.embed_contextual(DOC, &spans());
            let via_override = e.embed_contextual(DOC, &spans());
            assert_eq!(via_default, via_override);
        }

        #[test]
        #[cfg(debug_assertions)]
        #[should_panic(expected = "non-overlapping")]
        fn overlapping_spans_refused_in_debug() {
            let e = ModellessContextualEmbedder;
            let _ = e.embed_contextual(DOC, &[SpanRange::new(0, 10), SpanRange::new(5, 20)]);
        }
    }
}
