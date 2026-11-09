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
#![allow(clippy::doc_markdown, clippy::cast_precision_loss,
        clippy::cast_possible_truncation, clippy::cast_sign_loss,
        clippy::cast_possible_wrap, clippy::cast_lossless,
        clippy::missing_const_for_fn, clippy::missing_panics_doc,
        clippy::missing_errors_doc, clippy::missing_fields_in_debug,
        clippy::module_name_repetitions, clippy::must_use_candidate,
        clippy::inline_always, clippy::let_and_return,
        clippy::manual_let_else, clippy::explicit_iter_loop,
        clippy::match_same_arms, clippy::single_char_pattern,
        clippy::format_push_string, clippy::ignore_without_reason,
        clippy::assertions_on_constants, clippy::too_many_lines,
        clippy::assert_is_empty)]

//! BM25 inverted index for full-text search over Warm tier text values.
//!
//! An **auxiliary index** — the consumer maintains it alongside the forward
//! `LocalKvStore` (index on write, search on read). The KV stores arbitrary
//! bytes; the BM25 index stores term→key mappings for text fields the consumer
//! declares as searchable.
//!
//! ## Why BM25 (not cosine / vector search)?
//!
//! BM25 is the classic **lexical** ranking function — it ranks documents by
//! term frequency (tf) + inverse document frequency (idf) + length
//! normalization. It's the right tool for:
//! - Quest title/description search ("slay the dragon")
//! - Item name search ("iron sword")
//! - Player chat log search
//!
//! It's NOT a replacement for vector/latent similarity (the ShardRow `embed`
//! path handles that). BM25 + vector search are complementary: BM25 for exact
//! keyword recall, vector for semantic recall. (Production search engines
//! fuse both — see the BM25 + dense retrieval literature.)
//!
//! ## Tokenization
//!
//! Configurable via the [`Tokenizer`] trait. The default
//! ([`WhitespaceTokenizer`]) does whitespace split + ASCII lowercase (the
//! Plan 323 POC behavior, preserved as the explicit default). Extensions:
//! - [`StopWordTokenizer`] — wraps an inner tokenizer + filters a configurable
//!   set of common words (ships with [`StopWordTokenizer::english`] for ~25
//!   English stop-words). Composable — can stack on a future stemmer.
//! - [`CodeTokenizer`] — splits on non-identifier characters so source-code
//!   symbols stay searchable (`fn my_func()` → `[fn, my_func]`, not
//!   `[fn, "my_func()"]`). Issue 589.
//!
//! Future extensions (Porter stemmer, CJK segmentation) can implement
//! `Tokenizer` without changing the postings storage format (terms are
//! `String`). Use [`Bm25Index::with_tokenizer`] to plug one in.
//!
//! ## BM25 formula
//!
//! ```text
//! score(q, d) = Σ_{t∈q} IDF(t) · (tf(t,d) · (k1+1)) / (tf(t,d) + k1·(1-b+b·|d|/avgdl))
//! ```
//!
//! where:
//! - `IDF(t) = ln((N - df(t) + 0.5) / (df(t) + 0.5) + 1)` (the +1 smooths
//!   negative IDF for very common terms)
//! - `tf(t,d)` = term frequency of `t` in document `d`
//! - `|d|` = document length (token count)
//! - `avgdl` = average document length across the corpus
//! - `k1 = 1.2` (term frequency saturation)
//! - `b = 0.75` (length normalization strength)
//!
//! Plan 323 Phase C (2026-07-30).

use papaya::HashMap;
use std::collections::{HashMap as StdHashMap, HashSet};
use std::sync::Arc;

/// Default BM25 k1 parameter (term frequency saturation). Standard value
/// from the literature; higher = more saturation (longer documents get less
/// per-term credit).
pub const DEFAULT_K1: f32 = 1.2;

/// Default BM25 b parameter (length normalization strength). 0 = no
/// normalization; 1 = full normalization. Standard value 0.75.
pub const DEFAULT_B: f32 = 0.75;

/// Tokenizer trait — controls how raw text becomes indexed/query terms.
///
/// The default impl ([`WhitespaceTokenizer`]) does whitespace split +
/// ASCII lowercase (the Plan 323 POC behavior). Extensions:
/// - [`StopWordTokenizer`] — filters a configurable set of common words.
///
/// Future extensions (Porter stemmer, CJK segmentation) can implement this
/// trait without changing the postings storage format (terms are `String`).
///
/// **Invariant:** the same tokenizer MUST be used for both `index` and
/// `search` (enforced by construction — `Bm25Index` owns one tokenizer).
/// A mismatch silently drops query terms → wrong results.
///
/// `Send + Sync` is required because `Bm25Index` is shared across threads
/// for lock-free concurrent search.
pub trait Tokenizer: Send + Sync {
    /// Tokenize `text`, appending each term to `out`. The scratch buffer is
    /// reused across calls to avoid per-call allocation (the hot path —
    /// callers pass a `Vec` they clear + reuse).
    fn tokenize_into(&self, text: &str, out: &mut Vec<String>);

    /// Convenience wrapper: tokenize into a fresh `Vec`. Allocates; prefer
    /// `tokenize_into` in hot paths.
    fn tokenize(&self, text: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.tokenize_into(text, &mut out);
        out
    }
}

/// Default tokenizer: whitespace split + ASCII lowercase. The Plan 323 POC
/// behavior, preserved as the explicit default so existing behavior is
/// unchanged when no custom tokenizer is configured.
#[derive(Debug, Default, Clone, Copy)]
pub struct WhitespaceTokenizer;

impl Tokenizer for WhitespaceTokenizer {
    fn tokenize_into(&self, text: &str, out: &mut Vec<String>) {
        for word in text.split_whitespace() {
            out.push(word.to_lowercase());
        }
    }
}

/// Tokenizer for source code: splits on non-identifier chars (anything outside
/// `[a-zA-Z0-9_]`) and ASCII-lowercases the result. Designed for BM25 symbol
/// search on Rust / code corpora so that `process_payment(amount:` tokenizes
/// to `[process_payment, amount]` (not `["process_payment(amount:"]`).
///
/// This fixes the [`WhitespaceTokenizer`] limitation for code corpora:
/// `WhitespaceTokenizer` keeps `my_func(` as one token (the `(` rides the
/// whitespace split), so a symbol query `my_func` silently misses it.
/// `CodeTokenizer` splits at the `(`, yielding `my_func` as its own token.
///
/// Language-agnostic — the identifier rule is `[A-Za-z0-9_]`, which covers
/// Rust, C, C++, Java, JS, Go, Python, etc. Not Rust-specific.
///
/// No subword splitting: `my_func` stays one token (not `[my, func]`),
/// `HashMap` stays one token (not `[hash, map]`). A camelCase / snake_case
/// splitter is a separate concern; file a follow-up if fuzzy code search
/// needs it.
///
/// Composes with [`StopWordTokenizer`]:
/// `StopWordTokenizer::english(Box::new(CodeTokenizer))` indexes identifier
/// content minus common English words.
///
/// Issue 589 (2026-08-07): adopted as the default BM25 tokenizer in
/// `riir-rag::NeuronDbRetriever` so source-code symbols are exactly
/// searchable. Prose-only corpora can opt back into [`WhitespaceTokenizer`].
#[derive(Debug, Default, Clone, Copy)]
pub struct CodeTokenizer;

impl Tokenizer for CodeTokenizer {
    fn tokenize_into(&self, text: &str, out: &mut Vec<String>) {
        for token in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
            if !token.is_empty() {
                out.push(token.to_lowercase());
            }
        }
    }
}

/// Composable stop-word filter. Wraps an inner tokenizer (typically
/// [`WhitespaceTokenizer`]) and drops any token found in the stop-word set.
///
/// Ship with [`StopWordTokenizer::english`] for the default English common-word
/// set (~25 words: the, a, an, of, to, ...). Use [`StopWordTokenizer::with_set`]
/// for a custom set (e.g. game-specific filler words).
///
/// Composability: `StopWordTokenizer` can wrap any inner tokenizer, so a
/// future stemmer can be stacked underneath without changing this struct.
/// Not `Clone` (owns a `Box<dyn Tokenizer>`); stored once per `Bm25Index`.
pub struct StopWordTokenizer {
    inner: Box<dyn Tokenizer + Send + Sync>,
    stopwords: HashSet<String>,
}

impl std::fmt::Debug for StopWordTokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StopWordTokenizer")
            .field("stopwords_len", &self.stopwords.len())
            .finish()
    }
}

impl StopWordTokenizer {
    /// Default English stop-word set (~25 common words that add search noise
    /// on game text like quest titles / item names). Lowercased for
    /// case-insensitive matching against the (already-lowercased) tokens
    /// produced by the inner `WhitespaceTokenizer`.
    const ENGLISH_DEFAULT: &[&str] = &[
        "the", "a", "an", "and", "or", "of", "to", "in", "is", "it", "for", "on", "with", "as",
        "at", "by", "be", "this", "that", "from", "are", "was", "were", "will", "your",
    ];

    /// Wrap `inner` with the default English stop-word set.
    #[must_use]
    pub fn english(inner: Box<dyn Tokenizer + Send + Sync>) -> Self {
        Self::with_set(
            inner,
            Self::ENGLISH_DEFAULT.iter().map(|s| (*s).to_string()),
        )
    }

    /// Wrap `inner` with a custom stop-word set. The set is lowercased on
    /// insertion (matches the lowercase tokens from `WhitespaceTokenizer`).
    #[must_use]
    pub fn with_set<I, S>(inner: Box<dyn Tokenizer + Send + Sync>, words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let stopwords = words
            .into_iter()
            .map(|s| s.into().to_lowercase())
            .collect::<HashSet<_>>();
        Self { inner, stopwords }
    }
}

impl Tokenizer for StopWordTokenizer {
    fn tokenize_into(&self, text: &str, out: &mut Vec<String>) {
        // Tokenize directly into `out` (appending), then filter the newly-added
        // range in place via a stable partition. The prior form allocated an
        // intermediate `raw` Vec per call; this matters because
        // `Bm25Index::search` calls this on the hot query path.
        //
        // The swap-based compaction preserves the relative order of KEPT
        // tokens (read advances monotonically; kept tokens land at
        // `start_len, start_len+1, ...` in encounter order). Stopwords are
        // swapped to the tail and truncated. Existing caller tokens in
        // `out[..start_len]` are never touched — append semantics hold.
        let start_len = out.len();
        self.inner.tokenize_into(text, out);
        let mut write = start_len;
        for read in start_len..out.len() {
            if !self.stopwords.contains(&out[read]) {
                if write != read {
                    out.swap(write, read);
                }
                write += 1;
            }
        }
        out.truncate(write);
    }
}

/// A single posting: (key, term frequency in that document).
#[derive(Clone, Debug)]
struct Posting {
    /// The forward-KV key this posting points to (so search results can
    /// look up the full document).
    ///
    /// `Arc<[u8]>`, not `Vec<u8>`: papaya values are immutable, so appending a
    /// posting means cloning the whole postings list. With an owned `Vec<u8>`
    /// per posting that clone heap-allocated and memcpy'd EVERY existing key,
    /// making a single `index()` call O(postings) allocations and the corpus
    /// build O(N²) — measured at 4.8 s to index 4000 docs. Behind an `Arc` the
    /// same clone is a refcount bump and allocates nothing.
    key: Arc<[u8]>,
    /// Term frequency: how many times this term appears in the document.
    tf: u32,
    /// The document's length (token count), inlined so `search` scores a
    /// posting with ONE random access instead of a second `doc_lengths`
    /// probe per posting (Issue 633 T6). Stored pre-cast as `doc_len as f32`
    /// — the exact cast `search` applied to the probed `usize` — so scores
    /// are bit-identical to the probe path. Free in memory: it fills the 4
    /// bytes of padding after `tf` (the struct stays 24 bytes, asserted below).
    dl: f32,
    /// The document's dense ordinal ([`Bm25Index::ordinals`]) — the index
    /// `search` accumulates this posting's score into, so the per-posting
    /// cost is one array access instead of hashing (and dereferencing) the
    /// key bytes (riir-refine Issue 173 T2).
    ord: u32,
}

// `Arc<[u8]>` (16) + `tf` (4) + `dl` (4) + `ord` (4) + 4 padding.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Posting>() == 32);

/// BM25 inverted index — term → postings list + document metadata.
///
/// Maintained alongside a `LocalKvStore`. Call [`index`](Self::index) when a
/// text value is written to the KV; call [`search`](Self::search) to find
/// keys whose text matches a query.
///
/// Thread-safe via papaya (lock-free reads). Writes take `&self` + use papaya's
/// interior mutability.
pub struct Bm25Index {
    /// Postings: term → Vec<Posting>. The inverted index.
    /// `papaya::HashMap` for lock-free concurrent reads during search.
    postings: HashMap<String, Vec<Posting>>,
    /// Document lengths: key → token count. The removal ledger (what to
    /// subtract from `total_length` on re-index/remove); `search` reads the
    /// copy inlined in each [`Posting::dl`], never this map (Issue 633 T6).
    doc_lengths: HashMap<Vec<u8>, usize>,
    /// Reverse index: key → Vec<term>. Lets [`remove_internal`] probe only
    /// the terms this document actually contains (O(doc_terms)) instead of
    /// scanning the whole vocabulary (O(total_terms)).
    ///
    /// Built incrementally during [`index`] (one entry per (key, unique term))
    /// and cleared on [`remove_internal`]. For a POC-scale corpus this is a
    /// few KB. For a 50K-doc corpus it scales linearly with the document
    /// count, not the vocabulary.
    key_to_terms: HashMap<Vec<u8>, Vec<String>>,
    /// Dense ordinal per key, assigned on first `index` and RETAINED across
    /// re-index and remove: a key keeps one ordinal for the index's lifetime,
    /// so a search racing a remove + re-index of the same key still merges
    /// both generations' postings into ONE score slot (the key-merge
    /// semantics of the old hash accumulator). Cost: one entry per distinct
    /// key ever indexed, plus a per-thread `u32` per ordinal of search scratch.
    ordinals: HashMap<Vec<u8>, u32>,
    /// Next unassigned ordinal.
    next_ordinal: std::sync::atomic::AtomicU32,
    /// Sum of all doc lengths (for computing avgdl incrementally).
    total_length: std::sync::atomic::AtomicUsize,
    /// Number of indexed documents.
    doc_count: std::sync::atomic::AtomicUsize,
    /// BM25 k1 parameter.
    k1: f32,
    /// BM25 b parameter.
    b: f32,
    /// Tokenizer used for both `index` and `search`. Defaults to
    /// [`WhitespaceTokenizer`] (Plan 323 POC behavior). Customizable via
    /// [`with_tokenizer`](Self::with_tokenizer).
    tokenizer: Box<dyn Tokenizer + Send + Sync>,
    /// Facet-scoped-search namespace (`bm25_facet`, plan 336): (field, value)
    /// → the facet's sorted key list + the ordinal ADMISSION BITMAP
    /// ([`crate::bm25_facet::FacetPostings`]). Deliberately a SEPARATE map
    /// from [`postings`](Self::postings) — no token exists, so no tokenizer
    /// can collide with one, and doc lengths / avgdl / IDF never see a facet
    /// (unscoped search stays byte-identical with facets indexed). The
    /// bitmap is what `search_scoped` admits through (T7b, the REVISE
    /// verdict): one L1-resident bit read per posting instead of a binary
    /// probe into key bytes.
    #[cfg(feature = "bm25_facet")]
    pub(crate) facet_postings: HashMap<(String, String), crate::bm25_facet::FacetPostings>,
    /// Facet reverse index (`bm25_facet`): key → the facets it carries, so
    /// [`remove_internal`](Self::remove_internal) strips them the same way it
    /// strips text terms (mirror of [`key_to_terms`](Self::key_to_terms)).
    #[cfg(feature = "bm25_facet")]
    pub(crate) key_to_facets: HashMap<Vec<u8>, Vec<(String, String)>>,
}

impl Bm25Index {
    /// Create an empty index with default BM25 parameters (k1=1.2, b=0.75).
    #[must_use]
    pub fn new() -> Self {
        Self::with_params(DEFAULT_K1, DEFAULT_B)
    }

    /// Create an empty index with custom BM25 parameters.
    #[must_use]
    pub fn with_params(k1: f32, b: f32) -> Self {
        Self {
            postings: HashMap::builder().build(),
            doc_lengths: HashMap::builder().build(),
            key_to_terms: HashMap::builder().build(),
            ordinals: HashMap::builder().build(),
            next_ordinal: std::sync::atomic::AtomicU32::new(0),
            total_length: std::sync::atomic::AtomicUsize::new(0),
            doc_count: std::sync::atomic::AtomicUsize::new(0),
            k1,
            b,
            tokenizer: Box::new(WhitespaceTokenizer),
            #[cfg(feature = "bm25_facet")]
            facet_postings: HashMap::builder().build(),
            #[cfg(feature = "bm25_facet")]
            key_to_facets: HashMap::builder().build(),
        }
    }

    /// Create an empty index with a custom tokenizer (e.g.
    /// [`StopWordTokenizer::english`]). BM25 params are defaults.
    ///
    /// **Invariant:** the same tokenizer is used for both `index` and
    /// `search` (enforced by construction — the index owns it).
    #[must_use]
    pub fn with_tokenizer(tokenizer: Box<dyn Tokenizer + Send + Sync>) -> Self {
        let mut idx = Self::new();
        idx.tokenizer = tokenizer;
        idx
    }

    /// Index (or re-index) a document. Tokenizes `text` via the index's
    /// configured [`Tokenizer`], updates the postings list + document
    /// metadata. Re-indexing a key removes old postings first (upsert
    /// semantics — the latest text wins).
    pub fn index(&self, key: &[u8], text: &str) {
        // If this key was previously indexed, remove its old postings first.
        self.remove_internal(key);

        // Tokenize via the configured tokenizer (default: whitespace + lowercase).
        let mut tokens: Vec<String> = Vec::new();
        self.tokenizer.tokenize_into(text, &mut tokens);
        let doc_len = tokens.len();

        // Count term frequencies in this document. `get_mut` first so a repeat
        // occurrence costs a probe instead of a `String` clone — `entry()`
        // needs an owned key up front even when the term is already present.
        //
        // StdHashMap (not BTreeMap): iteration order doesn't propagate to any
        // observable output. Postings papaya insertion order is per-document
        // but doesn't affect the postings Vec per term (each doc appends its
        // own Posting). The reverse_entry Vec order doesn't matter —
        // remove_internal iterates ALL of the doc's terms without early exit.
        // O(1) probes beat O(log n) for documents with many unique terms.
        //
        // This block used to also excuse `search`'s tied-score ordering as
        // "already non-deterministic via scores.into_iter() (HashMap
        // RandomState)". That was true and NOT benign: the same seed decided
        // which tied docs survived the `limit` cap, i.e. the returned SET.
        // `search` now imposes a total order (score desc, key asc) before
        // truncating — see the tie-break note there (riir-ai Issue 849).
        let mut tf_map: StdHashMap<String, u32> = StdHashMap::with_capacity(tokens.len());
        for token in &tokens {
            if let Some(tf) = tf_map.get_mut(token) {
                *tf += 1;
            } else {
                tf_map.insert(token.clone(), 1);
            }
        }

        // Update postings + metadata.
        let guard = self.postings.guard();
        let dl_guard = self.doc_lengths.guard();

        // One shared allocation for this document's key, refcounted into every
        // term's posting list instead of one `Vec<u8>` per (term, document).
        let key_rc: Arc<[u8]> = Arc::from(key);
        let dl = doc_len as f32;
        let ord = self.ordinal_of(key);

        // `into_iter`, not `&tf_map`: the term `String` is moved straight into
        // the postings map, where it used to be cloned for every term.
        // Build the reverse-index entry (key → its terms) from the same
        // iteration so `remove_internal` can probe only this doc's terms
        // instead of scanning the whole vocabulary.
        let mut reverse_entry: Vec<String> = Vec::with_capacity(tf_map.len());
        for (term, tf) in tf_map {
            reverse_entry.push(term.clone());
            // Get-or-insert the postings vec for this term.
            let postings = if let Some(existing) = self.postings.get(&term, &guard) {
                // Clone + extend (papaya returns a borrow; we can't mutate in
                // place). Size for the extra element up front — cloning at
                // exactly `len` would force the `push` to realloc immediately.
                let mut updated = Vec::with_capacity(existing.len() + 1);
                updated.extend_from_slice(existing);
                updated.push(Posting {
                    key: Arc::clone(&key_rc),
                    tf,
                    dl,
                    ord,
                });
                updated
            } else {
                vec![Posting {
                    key: Arc::clone(&key_rc),
                    tf,
                    dl,
                    ord,
                }]
            };
            self.postings.insert(term, postings, &guard);
        }
        // Record the reverse index AFTER the postings are written so a
        // concurrent `remove_internal` on this key sees a consistent state
        // (either the postings exist with the reverse index, or neither does).
        self.key_to_terms
            .insert(key.to_vec(), reverse_entry, &self.key_to_terms.guard());

        self.doc_lengths.insert(key.to_vec(), doc_len, &dl_guard);
        self.total_length
            .fetch_add(doc_len, std::sync::atomic::Ordering::Relaxed);
        self.doc_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Remove a document from the index. Removes all its postings + metadata.
    pub fn remove(&self, key: &[u8]) {
        self.remove_internal(key);
    }

    /// Search for documents matching `query`. Returns top-`limit` results as
    /// `(key, score)` pairs, sorted by score descending.
    ///
    /// **Ranking:** BM25 (tf-idf with length normalization). The query is
    /// tokenized via the same [`Tokenizer`] used at index time.
    pub fn search(&self, query: &str, limit: usize) -> Vec<(Vec<u8>, f32)> {
        let mut query_terms: Vec<String> = Vec::new();
        self.tokenizer.tokenize_into(query, &mut query_terms);
        let Some((doc_count, avgdl)) = self.corpus_stats(&query_terms) else {
            return Vec::new();
        };
        let guard = self.postings.guard();
        let (term_postings, total_postings) = self.collect_postings(&query_terms, &guard);

        // Accumulate BM25 scores per document into a DENSE slot table keyed by
        // the posting's ordinal (riir-refine Issue 173 T2): `slot[ord]` holds
        // `1 + index into entries` (0 = untouched), so a posting costs one
        // array access — no hashing, and no dereference of the key bytes
        // (the hash arm read every scored key twice: hash + equality). Each
        // document's float additions run in the same query-term order as the
        // hash arm, and `rank_top_k` imposes a total order, so the results are
        // bit-identical (G1 `g1_dense_accumulator_bit_identical_to_hashed`).
        // Keys stay borrowed `&[u8]` (held alive by `guard`); only the top-K
        // survivors are cloned at materialize.
        let mut entries: Vec<(&[u8], f32)> = Vec::with_capacity(total_postings);
        SCORE_SLOTS.with_borrow_mut(|slots| {
            for postings in &term_postings {
                let idf = bm25_idf(doc_count, postings.len() as f32);
                for posting in postings.iter() {
                    let ord = posting.ord as usize;
                    if ord >= slots.len() {
                        // Grow-on-demand instead of sizing from `next_ordinal`:
                        // a posting published by a concurrent `index` may carry
                        // an ordinal newer than any load this search made.
                        slots.resize(ord + 1, 0);
                    }
                    let slot = &mut slots[ord];
                    if *slot == 0 {
                        entries.push((&*posting.key, 0.0));
                        *slot = entries.len() as u32;
                    }
                    entries[*slot as usize - 1].1 += self.term_score(idf, posting, avgdl);
                }
            }
            // Reset only the touched slots: O(scored docs), not O(ordinals).
            for postings in term_postings {
                for posting in postings.iter() {
                    slots[posting.ord as usize] = 0;
                }
            }
        });

        rank_top_k(entries, limit)
    }

    /// Search for documents matching `query` AND carrying the facet
    /// `(field, value)` — the posting-list intersection (`bm25_facet`, plan
    /// 336; the state half lives in [`crate::bm25_facet`]). The query is
    /// scored EXACTLY like [`Self::search`] — same whole-corpus stats, same
    /// per-term IDF, same dense-slot accumulation in the same order — and a
    /// posting is admitted only when its key is in the facet's key set. A
    /// document's score is therefore identical scoped and unscoped, and the
    /// scoped result set is exactly the unscoped one restricted to the
    /// facet's keys (the equivalence law the GOAT bench re-asserts at
    /// scale). An unknown facet answers an empty result — honestly empty,
    /// never a widened search.
    #[cfg(feature = "bm25_facet")]
    pub fn search_scoped(
        &self,
        query: &str,
        field: &str,
        value: &str,
        limit: usize,
    ) -> Vec<(Vec<u8>, f32)> {
        let facet = (field.to_owned(), value.to_owned());
        let facet_guard = self.facet_postings.guard();
        let Some(facet) = self.facet_postings.get(&facet, &facet_guard) else {
            return Vec::new();
        };
        // The admission test: a BIT over the posting's dense ordinal (plan
        // 336 T7b, the delegated-owner REVISE verdict). The reviewer read this
        // loop and found the real defect in the sorted-key-list binary search:
        // each of its ~15 steps chases an `Arc` pointer to a likely-uncached
        // heap key and compares bytes — the measured probe tax (289 µs vs
        // post-filter's 184 µs at 10% selectivity) — while the ordinal the
        // loop reads one line later is already in the posting. The bitmap is
        // 782 words / 6.25 KB at a 50K-doc corpus (L1-resident), branch-free
        // at the read, and uses data already loaded. An ordinal beyond the
        // bitmap's grown length was never facet-indexed (the facet predates
        // that key): `bits.get` answers false, honestly empty, never a widen.
        let bits = &facet.bits;

        let mut query_terms: Vec<String> = Vec::new();
        self.tokenizer.tokenize_into(query, &mut query_terms);
        let Some((doc_count, avgdl)) = self.corpus_stats(&query_terms) else {
            return Vec::new();
        };
        let guard = self.postings.guard();
        let (term_postings, total_postings) = self.collect_postings(&query_terms, &guard);

        // The SAME dense-slot accumulation as `search` (riir-refine Issue 173
        // T2), with one admission test per posting. Identical addition order
        // ⇒ identical scores ⇒ `rank_top_k`'s total order gives identical
        // ranking to the unscoped search restricted to the facet's keys.
        let mut entries: Vec<(&[u8], f32)> =
            Vec::with_capacity(total_postings.min(facet.keys.len()));
        SCORE_SLOTS.with_borrow_mut(|slots| {
            for postings in &term_postings {
                let idf = bm25_idf(doc_count, postings.len() as f32);
                for posting in postings.iter() {
                    let ord = posting.ord as usize;
                    // Admit by ordinal bit — the branch-free membership the
                    // REVISE verdict directed (see the block comment above).
                    match bits.get(ord >> 6) {
                        Some(word) if (word >> (ord & 63)) & 1 == 1 => {}
                        _ => continue,
                    }
                    match bits.get(ord >> 6) {
                        Some(word) if (word >> (ord & 63)) & 1 == 1 => {}
                        _ => continue,
                    }
                    if ord >= slots.len() {
                        // Grow-on-demand (the concurrent-`index` note above
                        // applies verbatim).
                        slots.resize(ord + 1, 0);
                    }
                    let slot = &mut slots[ord];
                    if *slot == 0 {
                        entries.push((&*posting.key, 0.0));
                        *slot = entries.len() as u32;
                    }
                    entries[*slot as usize - 1].1 += self.term_score(idf, posting, avgdl);
                }
            }
            // Reset only the touched slots: O(scored docs), not O(ordinals).
            // The admission test means not every posting's slot was resized
            // into existence — an out-of-range slot was never touched (it
            // reads 0 by construction), so the bound guard skips it safely.
            for postings in term_postings {
                for posting in postings.iter() {
                    let ord = posting.ord as usize;
                    if ord < slots.len() {
                        slots[ord] = 0;
                    }
                }
            }
        });

        rank_top_k(entries, limit)
    }

    /// `(doc_count, avgdl)` for a non-empty query over a non-empty corpus.
    fn corpus_stats(&self, query_terms: &[String]) -> Option<(f32, f32)> {
        let doc_count = self.doc_count.load(std::sync::atomic::Ordering::Relaxed) as f32;
        if query_terms.is_empty() || doc_count == 0.0 {
            return None;
        }
        let total_length = self.total_length.load(std::sync::atomic::Ordering::Relaxed) as f32;
        Some((doc_count, total_length / doc_count))
    }

    /// Each query term's postings, one lookup per term, plus their summed
    /// length — an upper bound on docs touched (over-counts docs matching
    /// several terms) that pre-sizes the accumulator.
    fn collect_postings<'g>(
        &self,
        query_terms: &[String],
        guard: &'g impl papaya::Guard,
    ) -> (Vec<&'g Vec<Posting>>, usize) {
        let mut term_postings = Vec::with_capacity(query_terms.len());
        let mut total_postings = 0usize;
        for term in query_terms {
            if let Some(p) = self.postings.get(term, guard) {
                total_postings = total_postings.saturating_add(p.len());
                term_postings.push(p);
            }
        }
        (term_postings, total_postings)
    }

    /// BM25 term-document score. The document length is inlined in the
    /// posting at `index()` time (Issue 633 T6) — the document's TRUE length
    /// even inside a concurrent `index`/`remove` window, where the old
    /// per-posting `doc_lengths` probe fell back to `avgdl`.
    #[inline(always)]
    fn term_score(&self, idf: f32, posting: &Posting, avgdl: f32) -> f32 {
        let tf = posting.tf as f32;
        let denom = tf + self.k1 * (1.0 - self.b + self.b * posting.dl / avgdl);
        idf * (tf * (self.k1 + 1.0)) / denom
    }

    /// The key's dense ordinal, assigned on first sight (see
    /// [`ordinals`](Self::ordinals)). A lost `get_or_insert_with` race burns
    /// one ordinal — harmless, ordinals need only be distinct.
    pub(crate) fn ordinal_of(&self, key: &[u8]) -> u32 {
        let guard = self.ordinals.guard();
        if let Some(&ord) = self.ordinals.get(key, &guard) {
            return ord;
        }
        *self.ordinals.get_or_insert_with(
            key.to_vec(),
            || {
                let ord = self
                    .next_ordinal
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                // Wrapping would alias two keys onto one score slot.
                assert!(ord < u32::MAX, "Bm25Index: ordinal space exhausted");
                ord
            },
            &guard,
        )
    }

    /// The pre-T2 hash accumulator — the G1 reference arm for the dense
    /// [`search`](Self::search) and the G2 A/B baseline (riir-refine Issue
    /// 173). Shares `corpus_stats`/`collect_postings`/`term_score`/
    /// `rank_top_k`, so the arms differ ONLY in how scores are accumulated.
    #[cfg(test)]
    fn search_hashed<S: std::hash::BuildHasher + Default>(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<(Vec<u8>, f32)> {
        let mut query_terms: Vec<String> = Vec::new();
        self.tokenizer.tokenize_into(query, &mut query_terms);
        let Some((doc_count, avgdl)) = self.corpus_stats(&query_terms) else {
            return Vec::new();
        };
        let guard = self.postings.guard();
        let (term_postings, total_postings) = self.collect_postings(&query_terms, &guard);
        let mut scores: std::collections::HashMap<&[u8], f32, S> =
            std::collections::HashMap::with_capacity_and_hasher(total_postings, S::default());
        for postings in term_postings {
            let idf = bm25_idf(doc_count, postings.len() as f32);
            for posting in postings.iter() {
                *scores.entry(&*posting.key).or_insert(0.0) += self.term_score(idf, posting, avgdl);
            }
        }
        rank_top_k(scores.into_iter().collect(), limit)
    }

    /// Number of indexed documents.
    #[must_use]
    pub fn doc_count(&self) -> usize {
        self.doc_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Number of unique terms in the index.
    #[must_use]
    pub fn term_count(&self) -> usize {
        self.postings.len()
    }

    /// Remove a key's old postings (internal — used by re-index).
    ///
    /// Uses the `key_to_terms` reverse index to probe only this document's
    /// terms (O(doc_terms)) instead of scanning the whole vocabulary
    /// (O(total_terms)). For a 10K-doc corpus with ~30 unique terms each,
    /// that's ~30 postings lookups vs ~10K-term scans per re-index.
    fn remove_internal(&self, key: &[u8]) {
        // Remove from doc_lengths + update totals.
        let dl_guard = self.doc_lengths.guard();
        if let Some(old_len) = self.doc_lengths.remove(key, &dl_guard) {
            self.total_length
                .fetch_sub(*old_len, std::sync::atomic::Ordering::Relaxed);
            self.doc_count
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }

        // Probe the reverse index for this key's terms. If absent, the key
        // was never indexed — nothing to remove from postings.
        // Clone the term list OUT of the papaya borrow before dropping the
        // guard — we need `terms` alive across the postings loop below,
        // which takes a separate guard. papaya's `remove` returns a borrow
        // tied to `kt_guard`, not an owned value.
        let kt_guard = self.key_to_terms.guard();
        let Some(terms) = self.key_to_terms.remove(key, &kt_guard).cloned() else {
            return;
        };
        drop(kt_guard);

        // Filter each term's postings to drop this key. The postings vec is
        // cloned + filtered (papaya returns a borrow; we can't mutate in place).
        let guard = self.postings.guard();
        for term in &terms {
            if let Some(postings) = self.postings.get(term, &guard) {
                // Filter out the removed key.
                let filtered: Vec<Posting> = postings
                    .iter()
                    .filter(|p| &*p.key != key)
                    .cloned()
                    .collect();
                if filtered.is_empty() {
                    self.postings.remove(term, &guard);
                } else {
                    self.postings.insert(term.clone(), filtered, &guard);
                }
            }
        }

        // Strip the key's facets too (`bm25_facet`, plan 336) — a removed or
        // re-indexed doc never answers a scoped search through a dangling
        // facet. Mirrors the reverse-index discipline above.
        #[cfg(feature = "bm25_facet")]
        crate::bm25_facet::facets_remove(self, key);
    }
}

/// Top-`limit` `(key, score)` pairs under the total order (score desc, key
/// asc). Shared by [`Bm25Index::search`] and its G1 reference arm, so the
/// two can differ only in how they SCORE, never in how they rank.
/// BM25 IDF with smoothing (the +1 prevents negative IDF for very common terms).
#[inline]
fn bm25_idf(doc_count: f32, df: f32) -> f32 {
    ((doc_count - df + 0.5) / (df + 0.5) + 1.0).ln()
}

thread_local! {
    /// [`Bm25Index::search`]'s dense score-slot table, reused across calls
    /// (all zero between searches). Grows to the largest ordinal this thread
    /// has scored; shared by every index on the thread — ordinals from
    /// different indexes never meet within one search.
    static SCORE_SLOTS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn rank_top_k(mut entries: Vec<(&[u8], f32)>, limit: usize) -> Vec<(Vec<u8>, f32)> {
    // Top-K selection. Two optimizations vs the naive sort-then-truncate:
    //   1. `select_nth_unstable_by` partitions in O(N) avg, isolating the
    //      top `limit` entries at the front without fully sorting.
    //   2. We clone the borrowed keys into owned `Vec<u8>` ONLY for the
    //      K survivors, not for all N scored docs.
    // For limit=10, N=1000 this eliminates ~990 key clones + reduces sort
    // from O(N log N) to O(N + K log K).
    //
    // **Tie-break (riir-ai Issue 849):** the comparator breaks equal
    // scores by `key` ascending, giving a TOTAL order over `entries`.
    // Score-only was not enough, and the two optimizations above are
    // exactly why: `entries` used to be materialized from a `HashMap` whose
    // `RandomState` is seeded once per PROCESS (now: first-touch order) — and
    // `select_nth_unstable_by` is an unstable partition. Under a partial
    // order that cuts through a tie cluster, *which* tied documents land
    // in `entries[..limit]` is decided by the hasher seed, so two indexes
    // with IDENTICAL content answer the same query with a DIFFERENT
    // top-K SET in different processes (not merely a different order
    // within the tie). Measured by riir-ai `.issues/849` / riir-clippy
    // `.issues/062` T2a: a `RERANK_POOL = 12` candidate cap over the
    // healer corpus dropped `needless_return` in 3 of 12 processes,
    // flipping the shipped `cargo heal --suggest` pool; the riir-rag
    // repro (`crates/riir-rag/tests/retrieve_determinism.rs`) saw the
    // candidate COUNT itself vary 15/16/17/18 over 12 processes.
    //
    // Same bug class and same fix idiom as
    // `experience_graph::select_top_k_desc` (Issue 591 T9) — sort at the
    // source so every consumer gets a canonical answer. The tie-break is
    // borrow-based (no allocation) and only runs on exact score equality.
    let cmp_desc = |a: &(&[u8], f32), b: &(&[u8], f32)| {
        katgpt_core::float_order::desc(a.1, b.1).then_with(|| a.0.cmp(b.0))
    };
    if limit < entries.len() {
        // Partition: entries[..limit] are the top `limit` (unordered).
        let (top, _, _) = entries.select_nth_unstable_by(limit, cmp_desc);
        top.sort_by(cmp_desc);
    } else {
        entries.sort_by(cmp_desc);
    }
    let k = limit.min(entries.len());
    let mut ranked: Vec<(Vec<u8>, f32)> = Vec::with_capacity(k);
    for (key, score) in entries.iter().take(k) {
        ranked.push((key.to_vec(), *score));
    }
    ranked.shrink_to_fit();
    ranked
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g1_bm25_index_search_roundtrip() {
        let index = Bm25Index::new();
        index.index(b"q1", "slay the dragon");
        index.index(b"q2", "collect iron ore");
        index.index(b"q3", "slay the goblin horde");

        let results = index.search("slay", 10);
        assert_eq!(results.len(), 2);
        // Both q1 + q3 contain "slay".
        let q1: &[u8] = b"q1";
        let q3: &[u8] = b"q3";
        let keys: Vec<&[u8]> = results.iter().map(|(k, _)| k.as_slice()).collect();
        assert!(keys.contains(&q1));
        assert!(keys.contains(&q3));
    }

    #[test]
    fn g1_bm25_ranking_relevance() {
        // A doc with MORE occurrences of the query term should rank higher.
        let index = Bm25Index::new();
        index.index(b"d1", "dragon dragon dragon"); // tf=3
        index.index(b"d2", "dragon once"); // tf=1

        let results = index.search("dragon", 10);
        assert_eq!(results[0].0, b"d1"); // higher tf → higher score
    }

    #[test]
    fn g1_bm25_idf_rarer_terms_rank_higher() {
        // A term that appears in FEWER docs has higher IDF → ranks higher.
        let index = Bm25Index::new();
        index.index(b"d1", "common rare");
        index.index(b"d2", "common");
        index.index(b"d3", "common");

        // "rare" appears in 1 doc → higher IDF than "common" (3 docs).
        let rare_results = index.search("rare", 10);
        let common_results = index.search("common", 10);

        // Both should find d1, but "rare" should score d1 higher (relative to
        // its other results). Check: rare finds only d1; common finds all 3.
        assert_eq!(rare_results.len(), 1);
        assert_eq!(common_results.len(), 3);
    }

    #[test]
    fn g1_bm25_length_normalization() {
        // Shorter docs get a boost (b=0.75 normalization). Two docs with the
        // same tf for a term, but different lengths → shorter ranks higher.
        let index = Bm25Index::new();
        index.index(b"short", "dragon"); // len=1
        index.index(b"long", "dragon filler filler filler filler filler"); // tf=1, len=6

        let results = index.search("dragon", 10);
        assert_eq!(results[0].0, b"short", "shorter doc should rank higher");
    }

    #[test]
    fn g1_bm25_reindex_removes_old_postings() {
        let index = Bm25Index::new();
        index.index(b"k", "old text about dragons");
        assert_eq!(index.doc_count(), 1);

        // Re-index with new text — old "dragons" posting must be removed.
        index.index(b"k", "new text about swords");
        assert_eq!(index.doc_count(), 1, "re-index replaces, not adds");

        let dragon_results = index.search("dragons", 10);
        assert!(dragon_results.is_empty(), "old term must be gone");

        let sword_results = index.search("swords", 10);
        assert_eq!(sword_results.len(), 1);
        assert_eq!(sword_results[0].0, b"k");
    }

    #[test]
    fn g1_bm25_remove() {
        let index = Bm25Index::new();
        index.index(b"k1", "hello world");
        index.index(b"k2", "hello there");
        assert_eq!(index.doc_count(), 2);

        index.remove(b"k1");
        assert_eq!(index.doc_count(), 1);

        let results = index.search("hello", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, b"k2");
    }

    #[test]
    fn g1_bm25_multi_term_query() {
        let index = Bm25Index::new();
        index.index(b"d1", "the quick brown fox");
        index.index(b"d2", "the lazy brown dog");
        index.index(b"d3", "quick fox jumps");

        // "quick fox" → d1 + d3 match; d1 has both terms, d3 has both.
        let results = index.search("quick fox", 10);
        assert_eq!(results.len(), 2);
        // d3 is shorter ("quick fox jumps" = 3 tokens vs "the quick brown fox" = 4)
        // + both have tf=1 for each term → d3 should rank slightly higher.
        let d1: &[u8] = b"d1";
        let d3: &[u8] = b"d3";
        let keys: Vec<&[u8]> = results.iter().map(|(k, _)| k.as_slice()).collect();
        assert!(keys.contains(&d1));
        assert!(keys.contains(&d3));
    }

    #[test]
    fn g1_bm25_empty_query_returns_empty() {
        let index = Bm25Index::new();
        index.index(b"k", "some text");
        assert!(index.search("", 10).is_empty());
        assert!(index.search("   ", 10).is_empty());
    }

    #[test]
    fn g1_bm25_case_insensitive() {
        let index = Bm25Index::new();
        index.index(b"k", "Dragon SLAYER");
        let results = index.search("dragon slayer", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn g1_bm25_limit_truncates() {
        let index = Bm25Index::new();
        for i in 0..10 {
            let key = format!("d{i}");
            index.index(key.as_bytes(), "common term");
        }
        let results = index.search("common", 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn g1_bm25_term_count() {
        let index = Bm25Index::new();
        index.index(b"k", "alpha beta gamma");
        assert_eq!(index.term_count(), 3);
    }

    // ── Tokenizer trait tests (Issue 028) ───────────────────────────────

    #[test]
    fn g1_whitespace_default_unchanged() {
        // Bm25Index::new() must produce identical tokenization to the
        // pre-Issue-028 inline behavior (whitespace + lowercase).
        let index = Bm25Index::new();
        index.index(b"k", "The Quick Brown Fox");
        // 4 distinct lowercase terms, no filtering.
        assert_eq!(index.term_count(), 4);
        assert_eq!(index.doc_count(), 1);
        // Search is case-insensitive + matches all 4 terms.
        let results = index.search("the quick brown fox", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn g1_whitespace_tokenizer_direct() {
        // The WhitespaceTokenizer impl directly: matches split_whitespace + lowercase.
        let t = WhitespaceTokenizer;
        let mut out = Vec::new();
        t.tokenize_into("Hello WORLD  foo", &mut out);
        assert_eq!(out, vec!["hello", "world", "foo"]);
        // Empty / whitespace-only input → empty vec.
        out.clear();
        t.tokenize_into("   ", &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn g1_code_tokenizer_direct() {
        // The CodeTokenizer impl directly: split on non-identifier chars + lowercase.
        let t = CodeTokenizer;
        let mut out = Vec::new();
        // `my_func(` must NOT ride the punctuation — `my_func` is its own token.
        t.tokenize_into("fn my_func(x: u32) -> u32 {", &mut out);
        assert_eq!(out, vec!["fn", "my_func", "x", "u32", "u32"]);
        // Path separators split: `std::collections::HashMap` → 3 tokens.
        out.clear();
        t.tokenize_into("std::collections::HashMap", &mut out);
        assert_eq!(out, vec!["std", "collections", "hashmap"]);
        // String literal content is indexed (quotes split, content kept).
        out.clear();
        t.tokenize_into("\"hello world\"", &mut out);
        assert_eq!(out, vec!["hello", "world"]);
        // Numbers are kept (valid identifier-position tokens).
        out.clear();
        t.tokenize_into("v2 = 42", &mut out);
        assert_eq!(out, vec!["v2", "42"]);
        // Empty / punctuation-only input → empty vec.
        out.clear();
        t.tokenize_into("(){};:,,", &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn g1_code_tokenizer_fixes_symbol_search() {
        // The motivating bug (Issue 589): with WhitespaceTokenizer, indexing
        // `fn my_func() {}` + querying `my_func` returns NOTHING because the
        // indexed token is `my_func()` (the `(` rides the whitespace split).
        // CodeTokenizer splits at the `(` so `my_func` is searchable.
        let ws = Bm25Index::new(); // WhitespaceTokenizer (the default)
        ws.index(b"doc", "fn my_func() {}");
        assert!(
            ws.search("my_func", 10).is_empty(),
            "WhitespaceTokenizer baseline: symbol query misses the punct-attached token"
        );

        let code = Bm25Index::with_tokenizer(Box::new(CodeTokenizer));
        code.index(b"doc", "fn my_func() {}");
        let hits = code.search("my_func", 10);
        assert_eq!(hits.len(), 1, "CodeTokenizer: exact symbol query hits");
        assert_eq!(hits[0].0, b"doc");
    }

    #[test]
    fn g1_code_tokenizer_composes_with_stopword() {
        // StopWordTokenizer can wrap CodeTokenizer (composability). Indexing
        // code text with English stop-words should filter the stop-words but
        // keep identifier content.
        let index = Bm25Index::with_tokenizer(Box::new(StopWordTokenizer::english(Box::new(
            CodeTokenizer,
        ))));
        // "fn", "my_func", "the", "dragon" → "the" filtered → 3 terms.
        // ("fn" is not in the English stop-word set, so it survives.)
        index.index(b"q1", "fn my_func(the: dragon)");
        assert_eq!(
            index.term_count(),
            3,
            "stop-word 'the' filtered; fn/my_func/dragon kept"
        );
        // Exact symbol still searchable.
        assert_eq!(index.search("my_func", 10).len(), 1);
        // Stop-word-only query returns nothing.
        assert!(index.search("the", 10).is_empty());
    }

    #[test]
    fn g1_stopword_filters_common_terms() {
        // Index text with stop-words; verify the stop-words are NOT in the
        // postings (term_count excludes them) + a stop-word-only query
        // returns nothing.
        let index = Bm25Index::with_tokenizer(Box::new(StopWordTokenizer::english(Box::new(
            WhitespaceTokenizer,
        ))));
        index.index(b"q1", "slay the dragon");
        // "the" is a stop-word → filtered. Only "slay" + "dragon" indexed.
        assert_eq!(index.term_count(), 2, "stop-word 'the' must be filtered");
        // A stop-word-only query produces no terms after filtering → empty.
        assert!(index.search("the", 10).is_empty());
    }

    #[test]
    fn g1_stopword_preserves_content_terms() {
        // "slay the dragon" indexed with stop-words still matches a "dragon"
        // query — content terms are preserved, only common words dropped.
        let index = Bm25Index::with_tokenizer(Box::new(StopWordTokenizer::english(Box::new(
            WhitespaceTokenizer,
        ))));
        index.index(b"q1", "slay the dragon");
        index.index(b"q2", "collect the iron ore");

        // "dragon" query → q1 only.
        let dragon = index.search("dragon", 10);
        assert_eq!(dragon.len(), 1);
        assert_eq!(dragon[0].0, b"q1");

        // Multi-term query with a stop-word: "slay the dragon" → effective
        // query terms {slay, dragon} ("the" filtered) → still finds q1.
        let q = index.search("slay the dragon", 10);
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].0, b"q1");
    }

    #[test]
    fn g1_stopword_custom_set() {
        // Custom stop-word set: filter game-specific filler words.
        let index = Bm25Index::with_tokenizer(Box::new(StopWordTokenizer::with_set(
            Box::new(WhitespaceTokenizer),
            ["quest", "go"],
        )));
        index.index(b"q1", "quest go slay dragon");
        // "quest" + "go" filtered; only "slay" + "dragon" indexed.
        assert_eq!(index.term_count(), 2);
        // A query for the filtered words returns nothing.
        assert!(index.search("quest", 10).is_empty());
        assert!(index.search("go", 10).is_empty());
        // Content terms still match.
        assert_eq!(index.search("dragon", 10).len(), 1);
    }

    #[test]
    fn g1_stopword_composable() {
        // StopWordTokenizer can wrap another StopWordTokenizer (composability).
        // Filter English stop-words + a custom set via stacking.
        let inner = StopWordTokenizer::english(Box::new(WhitespaceTokenizer));
        let outer = StopWordTokenizer::with_set(Box::new(inner), ["dragon"]);
        let index = Bm25Index::with_tokenizer(Box::new(outer));
        index.index(b"q1", "slay the dragon");
        // Both "the" (English stop) + "dragon" (custom) filtered → only "slay".
        assert_eq!(index.term_count(), 1);
    }

    // -- CodeTokenizer tests (Issue 589) -------------------------------

    #[test]
    fn g1_code_tokenizer_splits_on_non_identifier_chars() {
        // The canonical case from Bench 575: source code snippets split
        // into identifier tokens so symbols are exactly searchable.
        let t = CodeTokenizer;
        let mut out = Vec::new();
        t.tokenize_into("process_payment(amount: u32) -> bool", &mut out);
        assert_eq!(out, vec!["process_payment", "amount", "u32", "bool"]);
    }

    #[test]
    fn g1_code_tokenizer_preserves_underscores_and_numbers() {
        let t = CodeTokenizer;
        let mut out = Vec::new();
        t.tokenize_into("foo_bar baz123 _private", &mut out);
        assert_eq!(out, vec!["foo_bar", "baz123", "_private"]);
    }

    #[test]
    fn g1_code_tokenizer_skips_empty_tokens() {
        let t = CodeTokenizer;
        let mut out = Vec::new();
        t.tokenize_into("a(b(c)))d", &mut out);
        assert_eq!(out, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn g1_code_tokenizer_lowercases() {
        let t = CodeTokenizer;
        let mut out = Vec::new();
        t.tokenize_into("MyFunc HelloWorld", &mut out);
        assert_eq!(out, vec!["myfunc", "helloworld"]);
    }

    #[test]
    fn g1_code_tokenizer_bm25_integration() {
        let index = Bm25Index::with_tokenizer(Box::new(CodeTokenizer));
        index.index(b"f1", "fn process_payment(amount: u32) -> bool");
        index.index(b"f2", "fn collect_ore() -> Item");
        let results = index.search("process_payment", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, b"f1");
    }

    // ── Issue 633 T6: inlined `dl` vs the per-posting probe ──────────

    /// The pre-T6 scoring loop, verbatim but for the `dl` source: a second
    /// `doc_lengths` probe per posting, `avgdl` fallback on a miss. The G1
    /// reference arm — shares `rank_top_k`, so the arms differ ONLY in scoring.
    fn search_probe_reference(idx: &Bm25Index, query: &str, limit: usize) -> Vec<(Vec<u8>, f32)> {
        let mut query_terms: Vec<String> = Vec::new();
        idx.tokenizer.tokenize_into(query, &mut query_terms);
        let doc_count = idx.doc_count.load(std::sync::atomic::Ordering::Relaxed) as f32;
        if query_terms.is_empty() || doc_count == 0.0 {
            return Vec::new();
        }
        let avgdl = idx.total_length.load(std::sync::atomic::Ordering::Relaxed) as f32 / doc_count;
        let guard = idx.postings.guard();
        let dl_guard = idx.doc_lengths.guard();
        let mut scores: std::collections::HashMap<&[u8], f32> = std::collections::HashMap::new();
        for term in &query_terms {
            let Some(postings) = idx.postings.get(term, &guard) else {
                continue;
            };
            let df = postings.len() as f32;
            let idf = ((doc_count - df + 0.5) / (df + 0.5) + 1.0).ln();
            for posting in postings.iter() {
                let tf = posting.tf as f32;
                let dl = idx
                    .doc_lengths
                    .get(&*posting.key, &dl_guard)
                    .copied()
                    .unwrap_or(avgdl as usize) as f32;
                let denom = tf + idx.k1 * (1.0 - idx.b + idx.b * dl / avgdl);
                *scores.entry(&*posting.key).or_insert(0.0) += idf * (tf * (idx.k1 + 1.0)) / denom;
            }
        }
        rank_top_k(scores.into_iter().collect(), limit)
    }

    /// Seeded corpus: `n_docs` docs over a `vocab`-term Zipf-ish vocabulary,
    /// then re-index + remove churn so stale postings would surface.
    fn t6_corpus(n_docs: usize, vocab: usize, seed: u64) -> Bm25Index {
        let mut state = seed;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let doc = |next: &mut dyn FnMut() -> u64| -> String {
            let len = 3 + (next() % 40) as usize;
            (0..len)
                .map(|_| {
                    let r = next() % (vocab as u64 * vocab as u64);
                    format!("w{}", (r as f64).sqrt() as u64) // skewed toward low ids
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        let idx = Bm25Index::new();
        for d in 0..n_docs {
            idx.index(format!("d{d:05}").as_bytes(), &doc(&mut next));
        }
        for d in (0..n_docs).step_by(7) {
            idx.index(format!("d{d:05}").as_bytes(), &doc(&mut next)); // re-index
        }
        for d in (0..n_docs).step_by(11) {
            idx.remove(format!("d{d:05}").as_bytes());
        }
        idx
    }

    #[test]
    fn g1_t6_inlined_dl_bit_identical_to_probe() {
        let mut checked = 0usize;
        for seed in [1u64, 0x9E37_79B9_7F4A_7C15, 42] {
            let idx = t6_corpus(600, 120, seed);
            for q in 0..120 {
                let query = format!("w{} w{} w{}", q, (q * 7) % 120, (q * 13 + 5) % 120);
                for limit in [1, 10, 1000] {
                    let got = idx.search(&query, limit);
                    let want = search_probe_reference(&idx, &query, limit);
                    assert_eq!(got.len(), want.len(), "len: {query} limit={limit}");
                    for ((gk, gs), (wk, ws)) in got.iter().zip(&want) {
                        assert_eq!(gk, wk, "order: {query} limit={limit}");
                        assert_eq!(gs.to_bits(), ws.to_bits(), "score bits: {query}");
                    }
                    checked += got.len();
                }
            }
        }
        // Loud-zero floor: an empty corpus would agree vacuously.
        assert!(checked > 10_000, "only {checked} rows compared");
    }

    #[test]
    #[ignore = "G2 timing gate — run with --release --ignored"]
    fn g2_t6_inlined_dl_vs_probe_ab() {
        use std::hint::black_box;
        use std::time::Instant;
        let idx = t6_corpus(20_000, 400, 7);
        // The 5 highest-df terms: long postings, so the scoring loop dominates.
        let guard = idx.postings.guard();
        let mut by_df: Vec<(usize, String)> = idx
            .postings
            .iter(&guard)
            .map(|(t, p)| (p.len(), t.clone()))
            .collect();
        drop(guard);
        by_df.sort_by(|a, b| b.cmp(a));
        let n_post: usize = by_df[..5].iter().map(|(n, _)| n).sum();
        let query_owned = by_df[..5]
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let query = query_owned.as_str();
        assert_eq!(
            idx.search(query, 10),
            search_probe_reference(&idx, query, 10)
        );
        let (pairs, reps) = (31, 10);
        let (mut ta, mut tb) = (Vec::with_capacity(pairs), Vec::with_capacity(pairs));
        for _ in 0..pairs {
            let t = Instant::now();
            for _ in 0..reps {
                black_box(search_probe_reference(
                    black_box(&idx),
                    black_box(query),
                    10,
                ));
            }
            ta.push(t.elapsed().as_nanos() as f64 / reps as f64);
            let t = Instant::now();
            for _ in 0..reps {
                black_box(idx.search(black_box(query), 10));
            }
            tb.push(t.elapsed().as_nanos() as f64 / reps as f64);
        }
        ta.sort_by(f64::total_cmp);
        tb.sort_by(f64::total_cmp);
        let (ma, mb) = (ta[pairs / 2], tb[pairs / 2]);
        println!(
            "G2 T6 docs=20000 postings_scored={n_post}: probe median {ma:.0} ns/search, inlined median {mb:.0} ns/search, speedup {:.2}x ({pairs} interleaved pairs x {reps} reps)",
            ma / mb
        );
        assert!(
            mb > 0.0 && ma > 0.0,
            "an arm measured 0 ns — optimised away"
        );
        assert!(mb < ma, "the inlined dl must beat the per-posting probe");
    }

    /// riir-refine Issue 173 T2 — the dense ordinal accumulator is
    /// bit-identical to the hash accumulator it replaced, including across
    /// re-index, remove and RE-ADD after remove (the retained-ordinal path),
    /// and for queries that repeat a term.
    #[test]
    fn g1_dense_accumulator_bit_identical_to_hashed() {
        type Fast = foldhash::fast::RandomState;
        let mut checked = 0usize;
        for seed in [3u64, 0xDEAD_BEEF, 77] {
            let idx = t6_corpus(600, 120, seed);
            // Re-add a slice of the removed keys: they must reuse their
            // ordinal, and their old-generation postings are gone.
            for d in (0..600).step_by(22) {
                idx.index(
                    format!("d{d:05}").as_bytes(),
                    &format!("w{} w{}", d % 120, (d * 3) % 120),
                );
            }
            for q in 0..120 {
                let query = format!("w{q} w{} w{q} w{}", (q * 7) % 120, (q * 13 + 5) % 120);
                for limit in [1, 10, 1000] {
                    let got = idx.search(&query, limit);
                    let want = idx.search_hashed::<Fast>(&query, limit);
                    assert_eq!(got.len(), want.len(), "len: {query} limit={limit}");
                    for ((gk, gs), (wk, ws)) in got.iter().zip(&want) {
                        assert_eq!(gk, wk, "order: {query} limit={limit}");
                        assert_eq!(gs.to_bits(), ws.to_bits(), "score bits: {query}");
                    }
                    checked += got.len();
                }
            }
            // Ordinals are retained: one per distinct key ever indexed.
            assert_eq!(idx.ordinals.len(), 600);
        }
        assert!(checked > 10_000, "only {checked} rows compared");
    }

    /// riir-refine Issue 173 — the score-accumulator A/B, now the shipped
    /// dense ordinal table vs the foldhash map it replaced (itself 1.45-1.48x
    /// over std SipHash, `ef8599a`), both arms on the shared scoring helpers.
    /// Interleaved pairs, medians, bit-identity checked first. Same corpus +
    /// query shape as the T6 gate.
    #[test]
    #[ignore = "G2 timing gate — run with --release --ignored"]
    fn g2_bm25_dense_accumulator_ab() {
        use std::hint::black_box;
        use std::time::Instant;
        type Fast = foldhash::fast::RandomState;
        let idx = t6_corpus(20_000, 400, 7);
        let guard = idx.postings.guard();
        let mut by_df: Vec<(usize, String)> = idx
            .postings
            .iter(&guard)
            .map(|(t, p)| (p.len(), t.clone()))
            .collect();
        drop(guard);
        by_df.sort_by(|a, b| b.cmp(a));
        let n_post: usize = by_df[..5].iter().map(|(n, _)| n).sum();
        let query_owned = by_df[..5]
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let query = query_owned.as_str();
        assert_eq!(idx.search(query, 10), idx.search_hashed::<Fast>(query, 10));
        let (pairs, reps) = (31, 10);
        let (mut ta, mut tb) = (Vec::with_capacity(pairs), Vec::with_capacity(pairs));
        for _ in 0..pairs {
            let t = Instant::now();
            for _ in 0..reps {
                black_box(black_box(&idx).search_hashed::<Fast>(black_box(query), 10));
            }
            ta.push(t.elapsed().as_nanos() as f64 / reps as f64);
            let t = Instant::now();
            for _ in 0..reps {
                black_box(black_box(&idx).search(black_box(query), 10));
            }
            tb.push(t.elapsed().as_nanos() as f64 / reps as f64);
        }
        ta.sort_by(f64::total_cmp);
        tb.sort_by(f64::total_cmp);
        let (ma, mb) = (ta[pairs / 2], tb[pairs / 2]);
        println!(
            "G2 bm25_dense_accumulator docs=20000 postings_scored={n_post}: foldhash median {ma:.0} ns/search, dense median {mb:.0} ns/search, speedup {:.2}x ({pairs} interleaved pairs x {reps} reps)",
            ma / mb
        );
        assert!(
            mb > 0.0 && ma > 0.0,
            "an arm measured 0 ns — optimised away"
        );
        assert!(mb < ma, "the dense table must beat the foldhash map");
    }

    // ── G2: remove_internal complexity (reverse index) ────────────────

    #[test]
    #[cfg_attr(debug_assertions, ignore)]
    fn g2_remove_internal_sublinear_in_vocabulary() {
        // Structural gate: `remove_internal` time must grow with the document's
        // own term count (O(doc_terms)), NOT the total vocabulary size
        // (O(total_terms)). The reverse index (`key_to_terms`) is the win —
        // the pre-index path scanned every term in the postings map.
        //
        // Setup: build two indexes — small vocab (100 docs × same 5 terms)
        // + large vocab (100 docs × 100 distinct terms each). Removing one
        // doc with k=5 terms should take similar time on both, because the
        // work is bounded by the doc's term count, not the index size.
        let build = |n_docs: usize, terms_per_doc: usize| -> Bm25Index {
            let idx = Bm25Index::new();
            for d in 0..n_docs {
                let key = format!("doc{d:04}");
                // Each doc gets `terms_per_doc` unique terms (e.g. "t0 t1 t2...").
                let text: String = (0..terms_per_doc)
                    .map(|t| format!("t{t}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                idx.index(key.as_bytes(), &text);
            }
            idx
        };

        // Small index: 100 docs × 5 terms each = 5-term vocabulary.
        let small = build(100, 5);
        // Large index: 100 docs × 100 terms each = 10K-term vocabulary.
        let large = build(100, 100);

        // Remove a 5-term doc from each (we re-index `doc0050` with empty
        // text — exercises the remove path with k=5 (small) vs k=5 from a
        // 10K-vocab index (large)). The reverse index makes both O(5);
        // the pre-index scan was O(5) vs O(10K).
        //
        // Warm up.
        for _ in 0..5 {
            small.index(b"doc0050", "");
            small.index(b"doc0050", "t0 t1 t2 t3 t4");
            large.index(b"doc0050", "");
            large.index(b"doc0050", "t0 t1 t2 t3 t4");
        }

        // Measure: remove from small (calls remove_internal).
        let iters = 50;
        let start = std::time::Instant::now();
        for _ in 0..iters {
            small.index(b"doc0050", ""); // remove
            small.index(b"doc0050", "t0 t1 t2 t3 t4"); // restore
        }
        let small_t = start.elapsed() / (iters * 2);

        // Measure: remove from large.
        let start = std::time::Instant::now();
        for _ in 0..iters {
            large.index(b"doc0050", "");
            large.index(b"doc0050", "t0 t1 t2 t3 t4");
        }
        let large_t = start.elapsed() / (iters * 2);

        // Sublinear: large (10K vocab) must NOT take 100×+ longer than small
        // (5 vocab) for removing the SAME 5-term doc. Allow generous 5×
        // headroom for noise — the O(total_terms) regression would show
        // ~2000× here (10K/5); O(doc_terms) should show ~1×.
        let ratio = large_t.as_secs_f64() / small_t.as_secs_f64();
        assert!(
            ratio < 5.0,
            "remove_internal did NOT scale with doc_terms: 5-vocab→10K-vocab took {ratio:.2}× \
             (small={small_t:?}, large={large_t:?}). \
             Expected ~1× for O(doc_terms); ~2000× indicates an O(total_terms) regression."
        );
    }
}
