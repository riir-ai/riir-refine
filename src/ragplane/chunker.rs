//! Vendored from `riir-neuron-db` (`658b3f5`, vendored 2026-11-09, issue
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

//! Chunking — splits a document into chunks. Pluggable (window / AST /
//! heading / sentence).
//!
//! The default [`WindowChunker`] is a fixed-size window with overlap, line-
//! boundary aware (doesn't split mid-line). The opt-in [`AstChunker`] (feature
//! `ast_chunker`) chunks Rust code by `impl`/`fn`/`mod` boundaries. The opt-in
//! [`HeadingChunker`](crate::heading_chunker::HeadingChunker) (feature
//! `heading_chunker`) chunks prose/markdown at ATX heading boundaries.

use crate::ragplane::query::ChunkSource;

/// A raw chunk — content + source provenance, BEFORE embedding + indexing.
///
/// Produced by [`Chunker::chunk`], consumed by `NeuronDbRetriever::index_*`
/// (which embeds each chunk via an [`Embedder`](crate::Embedder) + stores it).
#[derive(Debug, Clone)]
pub struct RawChunk {
    /// The chunk content (text).
    pub content: String,
    /// Source provenance (file path, line range).
    pub source: ChunkSource,
}

/// Chunker — splits a document into chunks. Pluggable (window / AST /
/// heading / sentence).
///
/// Default impl: [`WindowChunker`] (fixed-size window with overlap, line-aware).
/// Opt-in: [`AstChunker`](crate::ragplane::ast_chunker::AstChunker) (feature `ast_chunker`,
/// Rust AST chunking by `impl`/`fn`/`mod` boundaries);
/// [`HeadingChunker`](crate::heading_chunker::HeadingChunker) (feature
/// `heading_chunker`, prose/markdown ATX-heading sections — ndb Issue 638).
pub trait Chunker: Send + Sync {
    /// Split `content` (a single document/string) into chunks, each tagged
    /// with `source` path + the line range it covers.
    fn chunk(&self, content: &str, source: &str) -> Vec<RawChunk>;
}

/// The default chunker — fixed-size window with overlap, line-boundary aware.
///
/// Default window size: 512 tokens (estimated at 4 chars/token). Default
/// overlap: 64 tokens. Both are configurable via [`with_window`](Self::with_window)
/// + [`with_overlap](Self::with_overlap).
///
/// Line-boundary aware at BOTH ends: never splits mid-line. If the window
/// would end mid-line it extends to the end of the current line, and the next
/// chunk's start is snapped back to the beginning of the line the stride
/// landed in. This keeps code chunks syntactically valid (no partial `fn`
/// signature) and makes `ChunkSource::start_line` a citation that resolves —
/// the cited line IS the chunk's first line.
///
/// End-only snapping is not enough and was the original bug: the stride is a
/// byte count from the previous chunk's START, so once an end was extended,
/// every later chunk began mid-line (`" DEFAULT_WINDOW_TOKENS * CHARS..."`)
/// and cited a line whose text did not match. The cost of snapping both ends
/// is a little extra overlap — measured 2.5% over the overlap setting at the
/// default 2048/256, pinned in `tests/real_corpus_shipped_path.rs`.
#[derive(Debug, Clone)]
pub struct WindowChunker {
    /// Window size in estimated tokens (4 chars/token). Default: 512.
    window_chars: usize,
    /// Overlap in estimated tokens. Default: 64.
    overlap_chars: usize,
}

/// Default chars-per-token estimate for the modelless path. The real Kimi-K3
/// tokenizer (feature-gated, Phase 4 P4.2) is more accurate; this is the
/// modelless floor.
pub const CHARS_PER_TOKEN: usize = 4;

/// Default window size (512 tokens × 4 chars/token = 2048 chars).
pub const DEFAULT_WINDOW_TOKENS: usize = 512;
/// Default overlap (64 tokens × 4 chars/token = 256 chars).
pub const DEFAULT_OVERLAP_TOKENS: usize = 64;

impl Default for WindowChunker {
    fn default() -> Self {
        Self {
            window_chars: DEFAULT_WINDOW_TOKENS * CHARS_PER_TOKEN,
            overlap_chars: DEFAULT_OVERLAP_TOKENS * CHARS_PER_TOKEN,
        }
    }
}

impl WindowChunker {
    /// Create a `WindowChunker` with custom window size (in tokens).
    #[must_use]
    pub fn with_window_tokens(mut self, window_tokens: usize) -> Self {
        self.window_chars = window_tokens.saturating_mul(CHARS_PER_TOKEN).max(1);
        self
    }

    /// Create a `WindowChunker` with custom overlap (in tokens).
    #[must_use]
    pub fn with_overlap_tokens(mut self, overlap_tokens: usize) -> Self {
        self.overlap_chars = overlap_tokens.saturating_mul(CHARS_PER_TOKEN);
        self
    }

    /// Direct char-window config (for tests that want exact control).
    #[must_use]
    pub fn with_window_chars(mut self, window_chars: usize) -> Self {
        self.window_chars = window_chars.max(1);
        self
    }

    /// Direct char-overlap config (for tests that want exact control).
    #[must_use]
    pub fn with_overlap_chars(mut self, overlap_chars: usize) -> Self {
        self.overlap_chars = overlap_chars;
        self
    }
}

/// Round `i` UP to the next UTF-8 character boundary in `s` (clamped to
/// `s.len()`).
///
/// `WindowChunker` measures its window in **bytes**, so a raw window offset
/// can land inside a multi-byte character — and `&content[i..]` panics when it
/// does. Rounding UP (never down) also guarantees forward progress: the
/// returned index is strictly greater than any `i` that sits mid-character, so
/// the chunk loop cannot stall.
///
/// (`str::ceil_char_boundary` is still unstable; this is the same operation.)
#[inline]
fn ceil_char_boundary(s: &str, mut i: usize) -> usize {
    let n = s.len();
    if i >= n {
        return n;
    }
    while i < n && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Snap `i` BACK to the start of the line containing it (byte index just past
/// the preceding `'\n'`, or 0).
///
/// The chunker documents "never splits mid-line", but extending only the chunk
/// END to a line boundary does not achieve that: the next chunk's start is a
/// raw byte stride, so it lands mid-line whenever the previous chunk's end was
/// extended past `start + window`. That produced chunks beginning with a
/// fragment (`" DEFAULT_WINDOW_TOKENS * CHARS_PER_TOKEN,"`) and a `start_line`
/// citation whose text did not match the chunk's own first line.
///
/// Snapping BACKWARD (not forward) is what preserves coverage: the returned
/// index is `<= i <= previous_end`, so no byte can fall between two chunks.
/// The caller handles the one case where backward snapping cannot make
/// progress (a line longer than the stride).
///
/// A line start is always a char boundary, so this needs no separate UTF-8
/// guard.
#[inline]
fn line_start_at_or_before(s: &str, i: usize) -> usize {
    let i = i.min(s.len());
    match s.as_bytes()[..i].iter().rposition(|&b| b == b'\n') {
        Some(nl) => nl + 1,
        None => 0,
    }
}

impl Chunker for WindowChunker {
    fn chunk(&self, content: &str, source: &str) -> Vec<RawChunk> {
        if content.is_empty() {
            return Vec::new();
        }

        // Advance by (window - overlap) each step. Stride must be ≥ 1.
        let stride = self.window_chars.saturating_sub(self.overlap_chars).max(1);

        let bytes = content.as_bytes();
        let total = bytes.len();
        let mut out = Vec::new();
        let mut start = 0usize;

        while start < total {
            // Tentative end = start + window_chars, clamped to total and
            // snapped UP to a char boundary — the window is a byte budget, so
            // it can land inside a multi-byte character and every slice below
            // would panic. Rounding up also keeps `end > start`.
            let mut end = ceil_char_boundary(content, (start + self.window_chars).min(total));

            // Line-boundary extension: if we're not at EOF AND the char at `end`
            // is not at a line start, advance to the end of the current line.
            // This keeps chunks line-aligned (no partial lines). Only extend if
            // there IS a newline ahead (don't loop forever on a lineless doc).
            if end < total {
                // If the next char after `end` isn't already a line boundary,
                // advance to just past the next '\n'.
                let needs_extend = end == 0 || bytes[end - 1] != b'\n';
                if needs_extend {
                    if let Some(nl_rel) = content[end..].find('\n') {
                        end += nl_rel + 1; // include the '\n'
                    } else {
                        end = total; // no more newlines → rest of doc
                    }
                }
            }

            // Compute the 1-based line range for this chunk.
            let chunk_str = &content[start..end];
            let start_line = 1 + content[..start].matches('\n').count();
            let end_line = start_line + chunk_str.matches('\n').count();

            out.push(RawChunk {
                content: chunk_str.to_string(),
                source: ChunkSource::new(source, start_line, end_line),
            });

            // Advance by stride. If stride >= (end - start) we'd loop forever
            // on a line that exceeds the window — guard against it.
            let advance = stride.min(end.saturating_sub(start).max(1));
            // Snap the next start BACK to a line boundary so the next chunk
            // begins at column 0. Without this the chunker splits mid-line
            // (contradicting its own contract) and `start_line` cites a line
            // whose text differs from the chunk's first line — the citation
            // does not resolve. Backward snapping keeps `next <= end`, so no
            // byte falls between chunks.
            let next = line_start_at_or_before(content, start + advance);
            start = if next > start {
                next
            } else {
                end.max(ceil_char_boundary(content, start + advance))
            };

            // Last chunk: if we've reached or passed total, stop.
            if end >= total {
                break;
            }
        }

        out.shrink_to_fit();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_content_produces_no_chunks() {
        let c = WindowChunker::default();
        assert!(c.chunk("", "x.rs").is_empty());
    }

    #[test]
    fn short_content_is_one_chunk() {
        let c = WindowChunker::default();
        let chunks = c.chunk("hello\nworld", "greet.rs");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "hello\nworld");
        assert_eq!(chunks[0].source.start_line, 1);
        assert_eq!(chunks[0].source.end_line, 2);
        assert_eq!(chunks[0].source.path, "greet.rs");
    }

    #[test]
    fn chunk_does_not_split_mid_line() {
        // A single very long line (longer than the window). The chunker must
        // NOT split it mid-line — it extends the window to the end of the line.
        let long_line = "a".repeat(1000);
        let content = format!("{long_line}\nsecond line\n");
        let c = WindowChunker::default().with_window_chars(100);
        let chunks = c.chunk(&content, "long.rs");

        // The first chunk must contain the WHOLE first line (1000 chars),
        // even though the window is only 100 chars — line-boundary aware.
        assert!(chunks[0].content.contains(&long_line));
        // Each chunk ends at a line boundary (or EOF).
        for ch in &chunks {
            if !ch.content.is_empty() {
                assert!(
                    ch.content.ends_with('\n') || ch.content == content.trim_end_matches('\n'),
                    "chunk must end at line boundary: {:?}",
                    &ch.content[ch.content.len().saturating_sub(10)..]
                );
            }
        }
    }

    #[test]
    fn multiple_chunks_have_overlap() {
        // Two windows worth of content with clear line boundaries.
        let mut content = String::new();
        for i in 0..50 {
            content.push_str(&format!("line {i}\n"));
        }
        let c = WindowChunker::default()
            .with_window_chars(60) // ~6 lines per window
            .with_overlap_chars(20); // ~2 lines overlap
        let chunks = c.chunk(&content, "multi.rs");

        assert!(chunks.len() > 1, "should produce multiple chunks");
        // All chunks should be from the same source.
        for ch in &chunks {
            assert_eq!(ch.source.path, "multi.rs");
        }
    }

    #[test]
    fn line_numbers_are_1_based_and_monotonic() {
        let content = "a\nb\nc\nd\ne\n";
        let c = WindowChunker::default().with_window_chars(4);
        let chunks = c.chunk(content, "lines.rs");
        assert!(!chunks.is_empty());
        // First chunk starts at line 1.
        assert_eq!(chunks[0].source.start_line, 1);
        // Line numbers never decrease across chunks.
        for w in chunks.windows(2) {
            assert!(
                w[1].source.start_line >= w[0].source.start_line,
                "line numbers must not decrease"
            );
        }
    }

    #[test]
    fn object_safety() {
        // Box<dyn Chunker> must compile (object safety).
        let c: Box<dyn Chunker> = Box::new(WindowChunker::default());
        let chunks = c.chunk("x\n", "o.rs");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn multibyte_content_does_not_panic() {
        // Regression: `window_chars` is a BYTE budget, so a window boundary
        // can land inside a multi-byte character. Before the char-boundary
        // snap this panicked with "byte index N is not a char boundary".
        // The workspace's own docs are full of → ≥ µ σ β ×, so this was
        // reachable from ordinary source files.
        let doc = "aaaa→bbbb\ncccc\n".repeat(40);
        for w in [4usize, 5, 7, 13, 50, 101] {
            let out = WindowChunker::default()
                .with_window_chars(w)
                .chunk(&doc, "t.rs");
            assert!(!out.is_empty(), "window {w} produced no chunks");
            // Reassembly check: every chunk must be a valid slice of the doc.
            for c in &out {
                assert!(
                    doc.contains(&c.content),
                    "window {w}: chunk not a substring"
                );
            }
        }
    }

    #[test]
    fn multibyte_chunks_cover_every_line() {
        // Stronger than "does not panic": no character may be dropped or
        // corrupted by the boundary snapping. Chunks do NOT tile exactly
        // (line-boundary extension makes them overlap even at overlap 0), so
        // the invariant is coverage: every line survives intact in some chunk.
        let doc = "héllo wörld → ✓\nsecond ligne ≥ 3\nтретья строка\n";
        for w in [5usize, 9, 17, 40] {
            let out = WindowChunker::default()
                .with_window_chars(w)
                .chunk(doc, "t.rs");
            for line in doc.lines().filter(|l| !l.is_empty()) {
                assert!(
                    out.iter().any(|c| c.content.contains(line)),
                    "window {w}: line {line:?} missing from every chunk"
                );
            }
        }
    }

    #[test]
    fn multibyte_emoji_and_cjk_survive_chunking() {
        // 4-byte characters (emoji) and 3-byte CJK at every window size.
        let doc = "🎉🎉🎉 party\n日本語のテキスト\n한국어 텍스트\n";
        for w in [1usize, 2, 3, 5, 8, 13] {
            let out = WindowChunker::default()
                .with_window_chars(w)
                .chunk(doc, "t.rs");
            assert!(!out.is_empty(), "window {w} produced no chunks");
            for line in doc.lines().filter(|l| !l.is_empty()) {
                assert!(
                    out.iter().any(|c| c.content.contains(line)),
                    "window {w}: line {line:?} missing"
                );
            }
        }
    }
}
