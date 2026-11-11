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

//! AST chunker — Rust-code-specific chunking via `syn`.
//!
//! Splits a Rust source file into chunks at semantic boundaries (`impl`
//! blocks, `fn` items, `mod` declarations, top-level `struct`/`enum`/`trait`).
//! This produces code chunks that preserve syntactic validity — a query about
//! a method retrieves the WHOLE `impl` block it belongs to, not a window that
//! slices through the middle of a signature.
//!
//! Behind feature gate `ast_chunker` (adds the `syn` dep, ~2MB binary). OFF by
//! default — only worth it for code corpora. The default
//! [`WindowChunker`](crate::WindowChunker) is fine for prose + mixed text.
//!
//! ## Line-number strategy (span-free)
//!
//! `proc_macro2::Span`'s byte-offset methods (`start()`/`end()`/`byte_range()`)
//! are either nightly-gated or feature-flagged across versions. To stay stable
//! across toolchains, this chunker does NOT rely on span internals. Instead it
//! reconstructs each item's source text via `to_token_stream().to_string()`
//! and locates that text in the original source to derive line ranges. This is
//! O(n×m) per file but chunking is a one-time index cost, not a hot-path call.

use crate::ragplane::chunker::{Chunker, RawChunk};
use crate::ragplane::query::ChunkSource;
use quote::ToTokens;

/// AST chunker — splits Rust source at `impl`/`fn`/`mod`/`struct`/`enum`/`trait`
/// boundaries.
///
/// Each chunk covers exactly one top-level item. The chunk's
/// `start_line`/`end_line` are the item's location in the original source
/// (1-based, inclusive).
///
/// For non-Rust input or input that fails to parse, falls back to a single
/// chunk containing the whole file (degrades gracefully — never panics).
#[derive(Debug, Clone, Copy, Default)]
pub struct AstChunker;

impl Chunker for AstChunker {
    fn chunk(&self, content: &str, source: &str) -> Vec<RawChunk> {
        if content.is_empty() {
            return Vec::new();
        }

        // Parse as a `syn::File` (the crate root).
        let file = if let Ok(f) = syn::parse_str::<syn::File>(content) {
            f
        } else {
            // Parse failure → single chunk covering the whole file.
            let end_line = content.matches('\n').count() + 1;
            return vec![RawChunk {
                content: content.to_string(),
                source: ChunkSource::new(source, 1, end_line),
            }];
        };

        // Precompute the byte offset of the start of each line — used to map
        // a found-substring offset back to a 1-based line number in O(log n).
        let mut line_starts: Vec<usize> = vec![0]; // line 1 starts at byte 0
        for (offset, ch) in content.char_indices() {
            if ch == '\n' {
                line_starts.push(offset + 1);
            }
        }
        let byte_to_line = |byte_offset: usize| -> usize {
            // Binary search: line number = count of line_starts ≤ byte_offset.
            match line_starts.binary_search(&byte_offset) {
                Ok(line) => line + 1, // 1-based
                Err(ins) => ins, // ins = number of starts ≤ offset → line = ins (1-based via index+1-1)
            }
        };

        let mut out = Vec::with_capacity(file.items.len());
        let mut search_from = 0usize; // cursor: don't re-find earlier items

        for item in &file.items {
            // Reconstruct the item's source text. `to_token_stream()` gives the
            // normalized token form; `.to_string()` joins with single spaces.
            // This is NOT byte-identical to the original (comments + whitespace
            // are lost), but the FIRST IDENTIFIER + signature tokens are stable
            // enough to locate the item in the source.
            let item_str = quote_item_signature(item);
            if item_str.is_empty() {
                continue;
            }

            // Find the item's signature in the source using whitespace-insensitive
            // matching. `to_token_stream().to_string()` normalizes whitespace
            // (adds spaces between tokens), so a plain `str::find` misses when
            // the source has no space (e.g. `alpha()` vs token-stream `alpha ()`).
            // The match returns BOTH ends in the HAYSTACK: the matched span's
            // end is the signature's end IN THE SOURCE — never compute it as
            // `sig_start + needle.len()` (the normalized needle is longer than
            // the source text: `&Vec<User>` → `& Vec < User >`; scanning from
            // past the source `{` left fn tails as chunk gaps — Issue 942).
            let found = find_whitespace_insensitive(content, search_from, &item_str);
            let (sig_start, sig_end) = if let Some((start, end)) = found {
                (start, end)
            } else {
                // Signature not found (e.g. macro-generated, comments
                // intervened). Skip — better to lose this item than to
                // produce a wrong-location chunk.
                continue;
            };
            // The chunk spans from the signature start to the END of the item
            // body. Find the matching closing delimiter (`}` for items with
            // bodies, `;` for items without).
            let end_byte = find_item_end(content, sig_end).unwrap_or(content.len());
            let chunk_str = &content[sig_start..end_byte];
            if chunk_str.is_empty() {
                continue;
            }

            let start_line = byte_to_line(sig_start);
            let end_line = byte_to_line(end_line_byte(content, end_byte, sig_start));

            out.push(RawChunk {
                content: chunk_str.to_string(),
                source: ChunkSource::new(source, start_line, end_line),
            });

            // Advance cursor PAST the item body so the next item search
            // doesn't re-find this item.
            search_from = end_byte;
        }

        // If the file has zero top-level items (e.g. only comments), emit the
        // whole file as one chunk so we don't lose content.
        if out.is_empty() {
            let end_line = content.matches('\n').count() + 1;
            out.push(RawChunk {
                content: content.to_string(),
                source: ChunkSource::new(source, 1, end_line),
            });
        }

        out.shrink_to_fit();
        out
    }
}

/// Extract a short, locatable signature string for a top-level item.
///
/// This is NOT the full item text — it's a short prefix (the item kind keyword
/// + the name) that we can reliably find in the source via `str::find`.
///
/// Using just the name + a leading keyword avoids false positives from
/// `to_token_stream()` whitespace normalization.
fn quote_item_signature(item: &syn::Item) -> String {
    use syn::Item;
    // Use `quote!` to get the token stream, then stringify + take the prefix
    // up to the first `{` or `;` (the signature, not the body). This keeps the
    // search string short + stable.
    let tokens = match item {
        Item::Fn(_) => item.to_token_stream(),
        Item::Struct(_) => item.to_token_stream(),
        Item::Enum(_) => item.to_token_stream(),
        Item::Trait(_) => item.to_token_stream(),
        Item::Impl(_) => item.to_token_stream(),
        Item::Mod(_) => item.to_token_stream(),
        Item::Const(_) => item.to_token_stream(),
        Item::Static(_) => item.to_token_stream(),
        Item::Type(_) => item.to_token_stream(),
        Item::Use(_) => item.to_token_stream(),
        _ => return String::new(),
    };
    let full = tokens.to_string();
    // Take up to the first `{` (body start) or `;` (item end) — whichever
    // comes first. Keep the signature for matching.
    let cut = full
        .find('{')
        .or_else(|| full.find(';'))
        .unwrap_or(full.len());
    full[..cut].trim().to_string()
}

/// Find the byte offset of the END of a top-level item, given a search start
/// AFTER the item's signature.
///
/// Scans forward from `start`, tracking brace depth. Returns the offset just
/// past the matching closing `}` (for body items) or just past the next `;`
/// (for bodyless items like `use` / `type` aliases). Returns `None` if no end
/// is found (malformed source) — the caller falls back to end-of-file.
fn find_item_end(content: &str, start: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut i = start;
    let mut depth: i32 = 0;
    let mut saw_brace = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                saw_brace = true;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 && saw_brace {
                    return Some(i + 1); // just past the closing brace
                }
            }
            b';' if depth == 0 => {
                // Bodyless item (use, type alias, etc.).
                return Some(i + 1);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Compute the byte offset to use for the END line number. We want the line of
/// the LAST character of the chunk, which is `end_byte - 1` (clamped to start).
fn end_line_byte(content: &str, end_byte: usize, start_byte: usize) -> usize {
    if end_byte == 0 || end_byte <= start_byte {
        return start_byte;
    }
    // If end_byte is past EOF (shouldn't happen — clamped by caller), clamp.
    let last = (end_byte - 1).min(content.len().saturating_sub(1));
    last.max(start_byte)
}

/// Whitespace-insensitive substring search. Finds the first occurrence of
/// `needle` in `haystack` starting at or after `from`, treating any run of
/// whitespace in `needle` as matching any run of whitespace (including zero)
/// in `haystack.
///
/// This is needed because `syn`'s `to_token_stream().to_string()` normalizes
/// whitespace (e.g. `alpha()` becomes `alpha ()`), so a plain `str::find` of
/// the reconstructed signature fails on source where tokens are adjacent.
///
/// Returns the byte offsets `(start, end)` of the match in `haystack`, where
/// `end` is just past the LAST MATCHED TOKEN IN THE HAYSTACK — the matched
/// source span, whose length differs from the needle's normalized length
/// (the haystack may have less whitespace, or none). Callers deriving a scan
/// cursor from the match MUST use `end`, never `start + needle.len()`.
/// Returns `None` when there is no match.
fn find_whitespace_insensitive(
    haystack: &str,
    from: usize,
    needle: &str,
) -> Option<(usize, usize)> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    // Tokenize the needle on whitespace boundaries.
    let needle_tokens: Vec<&str> = needle.split_whitespace().collect();
    if needle_tokens.is_empty() {
        return None;
    }
    let first_tok = needle_tokens[0];

    // Search candidate start positions: each occurrence of first_tok in the
    // haystack (from `from` onward). For each candidate, verify the rest of
    // the tokens match with whitespace-insensitive gaps.
    let mut search_from = from;
    while let Some(rel) = haystack[search_from..].find(first_tok) {
        let start = search_from + rel;
        if let Some(end) = verify_token_sequence(haystack, start, &needle_tokens) {
            return Some((start, end));
        }
        search_from = start + first_tok.len();
    }
    None
}

/// Verify that `tokens` appear in `haystack` starting at `start`, in order,
/// separated by any whitespace (including none). Each token must appear as a
/// contiguous substring (not a prefix of a longer identifier — we don't
/// enforce word boundaries here; the caller's `first_tok` is distinctive
/// enough in practice).
///
/// Returns `Some(end)` — the byte offset just past the last matched token in
/// the HAYSTACK — on success, `None` on any mismatch.
fn verify_token_sequence(haystack: &str, start: usize, tokens: &[&str]) -> Option<usize> {
    let mut pos = start;
    for (i, tok) in tokens.iter().enumerate() {
        if i == 0 {
            // First token already verified by the caller's find.
            pos += tok.len();
            continue;
        }
        // Skip whitespace.
        let bytes = haystack.as_bytes();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        // `get` (not indexing): a byte window landing inside a multi-byte
        // char (e.g. a `─` divider in a comment) is NOT a match — indexing
        // would panic on the non-char-boundary slice.
        if haystack.get(pos..pos + tok.len()) != Some(*tok) {
            return None;
        }
        pos += tok.len();
    }
    Some(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_rust_file_into_item_chunks() {
        let src = "pub fn alpha() -> u32 { 1 }\n\npub fn beta() -> u32 { 2 }\n\npub struct Foo { x: u32 }\n";
        let c = AstChunker;
        let chunks = c.chunk(src, "lib.rs");
        // 3 top-level items: alpha, beta, Foo.
        assert!(
            chunks.len() >= 3,
            "expected ≥3 item chunks, got {}",
            chunks.len()
        );
        let all_content: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert!(all_content.contains("fn alpha"));
        assert!(all_content.contains("fn beta"));
        assert!(all_content.contains("struct Foo"));
        // Line numbers are 1-based + monotonic.
        assert!(chunks[0].source.start_line >= 1);
        for w in chunks.windows(2) {
            assert!(w[1].source.start_line >= w[0].source.start_line);
        }
    }

    #[test]
    fn parses_impl_block_as_one_chunk() {
        let src = "impl Foo {\n    fn a() {}\n    fn b() {}\n}\n";
        let c = AstChunker;
        let chunks = c.chunk(src, "lib.rs");
        // The whole impl block is one item — its methods are NOT split out.
        assert_eq!(
            chunks.len(),
            1,
            "impl block should be one chunk, got: {chunks:?}"
        );
        assert!(chunks[0].content.contains("impl"));
    }

    #[test]
    fn empty_input_produces_no_chunks() {
        let c = AstChunker;
        assert!(c.chunk("", "x.rs").is_empty());
    }

    #[test]
    fn parse_failure_falls_back_to_whole_file_chunk() {
        let src = "this is not valid rust {{{";
        let c = AstChunker;
        let chunks = c.chunk(src, "bad.rs");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, src);
    }

    #[test]
    fn object_safety() {
        let c: Box<dyn Chunker> = Box::new(AstChunker);
        let chunks = c.chunk("fn main() {}", "o.rs");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn fn_tails_are_covered_full_span_not_normalized_len() {
        // Issue 942 pin: the chunk end is scanned from the signature's end
        // IN THE SOURCE, never from `sig_start + needle.len()` (the
        // NORMALIZED signature is longer than the source text: `&Vec<User>`
        // tokenizes to `& Vec < User >`), so a fn's closing statements + `}`
        // stay INSIDE its chunk — fn tails were chunk gaps before the fix
        // (`area` chunked 1-2, not 1-3; a real clippy diagnostic on the tail
        // sat in NO chunk and no retrieval could surface it).
        let src = "pub fn area(w: f64, h: f64) -> f64 {\n    return w * h;\n}\n\npub fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n    for u in users {\n        if u.id == id {\n            return Some(u);\n        }\n    }\n    None\n}\n";
        let c = AstChunker;
        let chunks = c.chunk(src, "lib.rs");
        assert_eq!(chunks.len(), 2, "one chunk per fn, got: {chunks:?}");
        let area = &chunks[0];
        assert_eq!(
            (area.source.start_line, area.source.end_line),
            (1, 3),
            "area spans its full body incl. the closing brace"
        );
        assert!(
            area.content.trim_end().ends_with('}'),
            "the closing brace is inside the chunk, not a gap"
        );
        let find_user = &chunks[1];
        assert_eq!(
            (find_user.source.start_line, find_user.source.end_line),
            (5, 12),
            "find_user spans its full body incl. the tail statement + closing brace"
        );
        assert!(find_user.content.contains("None"));
        assert!(find_user.content.trim_end().ends_with('}'));
        // No fn-tail gaps: the only uncovered line between chunks is the
        // blank separator (line 4).
        assert_eq!(find_user.source.start_line, area.source.end_line + 2);
    }
}
