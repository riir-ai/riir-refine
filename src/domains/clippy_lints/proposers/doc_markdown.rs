//! `clippy::doc_markdown` — backtick-wrap identifier-shaped words in doc
//! comments (Issue 120: the Plan 134 landing's ~35-finding manual backtick
//! pass, the fifth recorded instance of the family in HISTORY — rows #107
//! and #112, Batches 35 and 36 all paid it by hand before the issue
//! existed).
//!
//! This crate compiles with `#![warn(clippy::pedantic)]` at the root, so
//! EVERY new module landing generates the family; the healer being silent
//! on it meant a manual pass per landing — exactly the mechanical work it
//! exists to own.
//!
//! ## Detection: a faithful port of clippy's own predicates
//!
//! The word-level logic is ported VERBATIM from
//! `clippy_lints/src/doc/markdown.rs` (clippy 1.98-era master, fetched
//! 2026-09-16): split on whitespace + `'`, end-trim keeping `:`, the `()`
//! claw-back, start-trim, single-`:` strips, `doc-valid-idents` membership
//! (the 84-entry `DEFAULT_DOC_VALID_IDENTS` default list, embedded), then
//! fire on `has_underscore || contains("::") || is_camel_case ||
//! ends_with("()")` — including the `es`/`ified`/`s` plural stripping in
//! `is_camel_case` (`IDs`, `Vecs`, `Clippy` all stay silent) and the
//! escaped-underscore (`\_`) and underscore+hyphen refusals.
//!
//! ## Why syn attributes, not a line scan
//!
//! Doc comments are located through `syn::visit::Visit` over REAL
//! `#[doc]` attributes (comment-style only — the source at the attribute's
//! byte range must start `///` or `//!`). A raw line scan cannot
//! distinguish a doc comment from a STRING LITERAL containing `///`
//! text — and this repo's own fixtures embed Rust source in strings;
//! backticking those would change behavior. Macro token trees are opaque
//! to syn, so doc-shaped text inside `macro_rules!` bodies is invisible
//! here (divergence: silent, the safe direction).
//!
//! ## Fix shape
//!
//! One `SourceEdit` per word: the word's byte extent replaced by
//! `` `word` `` — byte-identical to real clippy's `MachineApplicable`
//! suggestion (verified by probe, clippy 0.1.98). Comment-local edits can
//! never change compilation, which is why this sits in `FIXABLE_LINTS`.
//! The P21 oracle lane consumes clippy's own spans verbatim for the same
//! lint once it is in the lane set — the proposer and the oracle lane are
//! complements (the oracle is exact on context, the proposer runs without
//! a feed).
//!
//! ## Context approximations (documented divergences, all skip-direction)
//!
//! Clippy parses the CONCATENATED doc text with pulldown-cmark; this
//! proposer approximates per doc line:
//! - **Fenced code** — a triple-backtick or `~~~` fence toggles tracked
//!   across a doc block (state resets when doc attrs are not on adjacent
//!   lines); fenced lines are skipped. Clippy consumes them as
//!   `CodeBlock` events.
//! - **Indented code** — 4+ spaces after the marker skipped (`CommonMark`
//!   indented code block).
//! - **Inline code spans** — segments between backtick pairs skipped.
//! - **Unbalanced ticks** — a line with an ODD raw backtick count is
//!   skipped whole (clippy suppresses word checks for the entire
//!   paragraph and fires the help-only "backticks are unbalanced" shape,
//!   which has no suggestion to consume).
//! - **Blockquotes** — lines starting `>` skipped (clippy skips words at
//!   `blockquote_level > 0`).
//! - **Link-shaped tokens** — a word containing `](`, `][`, or `://` is
//!   skipped whole; clippy checks link TEXT and autolinks with different
//!   machinery (bare URLs get an `<word>` suggestion instead — a
//!   different fix shape we deliberately do not produce).
//! - **`#[doc = "..."]` attribute form** — skipped (the `+3` marker
//!   slicing only holds for comment-style docs).
//! - **`#[doc(hidden)]` items** — clippy skips them entirely; we do not
//!   (the wrap is cosmetic there, never wrong syntax).

use super::helpers::byte_range;
use super::SourceEdit;
use syn::visit::Visit;
use syn::{Attribute, File};

/// clippy's `DEFAULT_DOC_VALID_IDENTS` (`clippy_config/src/conf.rs`, fetched
/// 2026-09-16 from master ≈ 1.98). A consumer's `clippy.toml`
/// `doc-valid-idents` may extend this — we embed the default only; extra
/// exemptions a repo configures are a documented over-wrap divergence
/// (cosmetic, and the oracle lane stays exact).
const DOC_VALID_IDENTS: &[&str] = &[
    "KiB", "MiB", "GiB", "TiB", "PiB", "EiB",
    "MHz", "GHz", "THz",
    "AccessKit",
    "CoAP", "CoreFoundation", "CoreGraphics", "CoreText",
    "DevOps",
    "Direct2D", "Direct3D", "DirectWrite", "DirectX",
    "ECMAScript",
    "GPLv2", "GPLv3",
    "GitHub", "GitLab",
    "IPv4", "IPv6",
    "InfiniBand", "RoCE",
    "ClojureScript", "CoffeeScript", "JavaScript", "PostScript", "PureScript", "TypeScript",
    "PowerPC", "PowerShell", "WebAssembly",
    "NaN", "NaNs",
    "OAuth", "GraphQL",
    "SQLite", "MySQL", "PostgreSQL", "MariaDB", "MongoDB",
    "OCaml",
    "OpenAL", "OpenDNS", "OpenGL", "OpenMP", "OpenSSH", "OpenSSL", "OpenStreetMap", "OpenTelemetry",
    "OpenType",
    "WebAuthn", "WebGL", "WebGL2", "WebGPU", "WebRTC", "WebSocket", "WebTransport",
    "WebP", "OpenExr", "YCbCr", "sRGB",
    "TensorFlow",
    "TrueType",
    "iOS", "macOS", "FreeBSD", "NetBSD", "OpenBSD", "NixOS",
    "TeX", "LaTeX", "BibTeX", "BibLaTeX",
    "MinGW",
    "CamelCase",
];

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { docs: Vec::new() };
    v.visit_file(file);
    // Visit order is roughly source order but not guaranteed across item
    // kinds — sort, then process with cross-line fence tracking.
    v.docs.sort_by_key(|r| r.start);
    let mut edits = Vec::new();
    let mut in_fence = false;
    let mut prev_line: Option<usize> = None;
    let mut line_no = 0usize;
    let mut scanned = 0usize;
    for range in v.docs {
        // Line number of this doc attr (for the adjacency fence reset).
        while scanned < range.start {
            if source.as_bytes()[scanned] == b'\n' {
                line_no += 1;
            }
            scanned += 1;
        }
        if prev_line.is_some_and(|p| line_no.saturating_sub(p) > 1) {
            // A gap between doc attrs = a new doc block: unbalanced-fence
            // state from the previous block must not leak (an unbalanced
            // fence is already broken markdown; suppressing later
            // unrelated docs for it would be a false skip).
            in_fence = false;
        }
        prev_line = Some(line_no);
        let line_end = source[range.start..]
            .find('\n')
            .map_or(source.len(), |i| range.start + i);
        let line = &source[range.start..line_end];
        // Comment-style only: the marker is exactly 3 chars, and the doc
        // text (syn's `#[doc]` value) is the rest of the line verbatim.
        let Some(text) = line.strip_prefix("///").or_else(|| line.strip_prefix("//!")) else {
            continue;
        };
        process_doc_line(text, range.start + 3, &mut in_fence, &mut edits);
    }
    edits
}

struct Collector {
    docs: Vec<std::ops::Range<usize>>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if attr.path().is_ident("doc") {
            self.docs.push(byte_range(attr));
        }
        syn::visit::visit_attribute(self, attr);
    }
}

/// Process one doc line's text (`base` = byte offset of `text[0]` in the
/// file). Skips code/quote contexts, then applies clippy's word logic to
/// every outside-backtick segment.
fn process_doc_line(text: &str, base: usize, in_fence: &mut bool, edits: &mut Vec<SourceEdit>) {
    let indent = text.len() - text.trim_start_matches(' ').len();
    let t = &text[indent..];
    // Fence toggle / fenced-content skip.
    if t.starts_with("```") || t.starts_with("~~~") {
        *in_fence = !*in_fence;
        return;
    }
    if *in_fence {
        return;
    }
    // CommonMark indented code block (4+ spaces).
    if indent >= 4 {
        return;
    }
    // Blockquote content is exempt (clippy: blockquote_level > 0).
    if t.starts_with('>') {
        return;
    }
    // Per-line unbalanced-tick approximation: an odd raw backtick count
    // suppresses the paragraph in clippy (help-only fire). Balanced pairs
    // are inline code spans — handled by segment splitting below.
    if text.matches('`').count() % 2 == 1 {
        return;
    }
    // Outside-backtick segments only (even indices after splitting on '`').
    let mut seg_base = base;
    for (seg_idx, segment) in text.split('`').enumerate() {
        let seg_end = seg_base + segment.len();
        if seg_idx % 2 == 0 {
            for (token_off, orig_word) in split_words(segment) {
                let token_start = seg_base + token_off;
                if let Some((word_off, word)) = clippy_trimmed_word(orig_word) {
                    if fires_doc_markdown(word) {
                        let word_start = token_start + word_off;
                        edits.push(SourceEdit {
                            start: word_start,
                            end: word_start + word.len(),
                            replacement: format!("`{word}`"),
                        });
                    }
                }
            }
        }
        // +1 for the consumed '`' separator.
        seg_base = seg_end + 1;
    }
}

/// Split a segment on whitespace and `'` (clippy's separator set),
/// yielding each whitespace-token with its byte offset within the segment.
fn split_words(segment: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0usize;
    let mut rest = segment;
    std::iter::from_fn(move || {
        let lead = rest
            .find(|c: char| !(c.is_whitespace() || c == '\''))
            .unwrap_or(rest.len());
        rest = &rest[lead..];
        offset += lead;
        if rest.is_empty() {
            return None;
        }
        let len = rest
            .find(|c: char| c.is_whitespace() || c == '\'')
            .unwrap_or(rest.len());
        let word = &rest[..len];
        rest = &rest[len..];
        let at = offset;
        offset += len;
        Some((at, word))
    })
}

/// Apply clippy's trim dance to one whitespace-token. Returns the byte
/// offset of the resulting word within the token plus the word, or `None`
/// when the word is empty / all-`:` / a known valid ident.
///
/// Ported from `markdown.rs::check`: end-trim keeping `:`, the `()`
/// claw-back, start-trim, single-`:` strips, valid-ident skip, and the
/// trailing-paren balancing (whose start offset mirrors clippy's
/// `original_len - word.len()` arithmetic).
fn clippy_trimmed_word(orig_word: &str) -> Option<(usize, &str)> {
    let trim_pattern = |c: char| !c.is_alphanumeric() && c != ':';
    // Trim punctuation as in `some comment (see foo::bar).`
    let end_trimmed = orig_word.trim_end_matches(trim_pattern);
    let mut word = end_trimmed;
    // If the word is immediately followed by `()`, claw it back.
    if let Some(tmp) = orig_word.get(..word.len() + 2) {
        if tmp.ends_with("()") {
            word = tmp;
        }
    }
    let original_len = word.len();
    let start_trimmed = word.trim_start_matches(trim_pattern);
    let mut word_off = original_len - start_trimmed.len();
    word = start_trimmed;
    // Remove a leading/trailing single `:` which may be part of a sentence.
    if word.starts_with(':') && !word.starts_with("::") {
        let stripped = word.trim_start_matches(':');
        word_off += word.len() - stripped.len();
        word = stripped;
    }
    if word.ends_with(':') && !word.ends_with("::") {
        word = word.trim_end_matches(':');
    }
    if word.chars().all(|c| c == ':') {
        return None;
    }
    if DOC_VALID_IDENTS.contains(&word) {
        return None;
    }
    // Ensure that all reachable matching closing parens are included as
    // well (clippy's `size_diff` arithmetic, faithfully — it can exceed
    // the true leading offset when a trailing `:` was stripped; that only
    // matters when unclosed parens coexist with both strips, where clippy
    // makes the same move).
    let size_diff = original_len - word.len();
    let mut open_parens = 0usize;
    let mut close_parens = 0usize;
    for c in word.chars() {
        if c == '(' {
            open_parens += 1;
        } else if c == ')' {
            close_parens += 1;
        }
    }
    while close_parens < open_parens
        && orig_word
            .get(size_diff..=(word.len() + size_diff))
            .is_some_and(|tmp| tmp.ends_with(')'))
    {
        word = &orig_word[size_diff..(word.len() + size_diff)];
        word_off = size_diff;
        close_parens += 1;
    }
    if word.is_empty() {
        return None;
    }
    Some((word_off, word))
}

/// The firing predicate, ported from `markdown.rs::check_word` minus the
/// URL arm (bare URLs get clippy's `<word>` shape — a different fix we do
/// not produce; link-shaped words are refused here instead).
fn fires_doc_markdown(word: &str) -> bool {
    // Link-shaped / URL-shaped guards (our divergences, skip-direction).
    /// clippy's `is_camel_case`: starts upper (not digit/lower), ≥2
    /// uppercase, ≥1 lowercase, all alphanumeric — with the plural strips
    /// (`IDs`, `Vecs`, `Clippy` stay silent).
    fn is_camel_case(s: &str) -> bool {
        // Nested fns FIRST (items_after_statements) and the plural strips
        // ported without let-chains (this crate is pre-2024 edition):
        // `-es` after an all-upper S|X stem ("Boxes"), `-ified` after an
        // all-upper stem, then a single trailing `s`.
        fn all_upper(p: &str) -> bool {
            p.chars().all(|c| c.is_ascii_uppercase())
        }
        fn ends_s_or_x(p: &str) -> bool {
            matches!(p.chars().last(), Some('S' | 'X'))
        }
        if s.starts_with(|c: char| c.is_ascii_digit() || c.is_ascii_lowercase()) {
            return false;
        }
        let s = if let Some(prefix) = s.strip_suffix("es") {
            if all_upper(prefix) && ends_s_or_x(prefix) {
                prefix
            } else {
                s.strip_suffix('s').unwrap_or(s)
            }
        } else if let Some(prefix) = s.strip_suffix("ified") {
            if all_upper(prefix) {
                prefix
            } else {
                s.strip_suffix('s').unwrap_or(s)
            }
        } else {
            s.strip_suffix('s').unwrap_or(s)
        };
        s.chars().all(char::is_alphanumeric)
            && s.chars().filter(|c| c.is_uppercase()).take(2).count() > 1
            && s.chars().filter(|c| c.is_lowercase()).take(1).count() > 0
    }
    fn has_underscore(s: &str) -> bool {
        s != "_" && !s.contains("\\_") && s.contains('_')
    }
    fn has_hyphen(s: &str) -> bool {
        s != "-" && s.contains('-')
    }

    if word.contains("://") || word.contains("](") || word.contains("][") {
        return false;
    }
    // Mixed-case words with BOTH separators are names, not idents
    // (clippy Issue #2343 class) — refused before any arm fires.
    if has_underscore(word) && has_hyphen(word) {
        return false;
    }
    has_underscore(word) || word.contains("::") || is_camel_case(word) || word.ends_with("()")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn snake_case_and_camel_case_wrap() {
        let src = "/// Runs the shader_perf stage over a HashMap of buckets.\nfn stage() {}";
        let out = propose_fix("doc_markdown", src).expect("fires");
        assert_eq!(
            out,
            "/// Runs the `shader_perf` stage over a `HashMap` of buckets.\nfn stage() {}"
        );
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn path_words_wrap_whole() {
        let src = "/// See std::vec::Vec for details.\nfn go() {}";
        let out = propose_fix("doc_markdown", src).expect("fires");
        assert!(out.contains("`std::vec::Vec`"), "{out}");
    }

    #[test]
    fn call_suffix_wraps_with_parens() {
        let src = "/// Call foo() first.\nfn go() {}";
        let out = propose_fix("doc_markdown", src).expect("fires");
        assert!(out.contains("`foo()`"), "{out}");
    }

    #[test]
    fn wrapped_parens_word_wraps_inner_only() {
        // clippy strips the leading `(` (start-trim) and the balancing
        // claw-back only re-adds CLOSING parens — so `(shader_perf)`
        // wraps the inner word: `(`shader_perf`)`.
        let src = "/// Call (shader_perf) here.\nfn go() {}";
        let out = propose_fix("doc_markdown", src).expect("fires");
        assert!(out.contains("(`shader_perf`)"), "{out}");
    }

    #[test]
    fn backticked_words_untouched() {
        let src = "/// The `shader_perf` lane and the `HashMap` pool.\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn fenced_code_skipped_and_prose_after_fence_resumes() {
        let src = "/// Example:\n///\n/// ```\n/// let shader_perf = 1;\n/// ```\n/// Then shader_perf again.\nfn go() {}";
        let out = propose_fix("doc_markdown", src).expect("fires after the fence");
        assert!(out.contains("Then `shader_perf` again."), "{out}");
        assert!(
            !out.contains("let `shader_perf`"),
            "fenced line must stay untouched: {out}"
        );
    }

    #[test]
    fn indented_code_skipped() {
        let src = "/// Text:\n///\n///     let shader_perf = 1;\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn blockquote_lines_skipped() {
        let src = "/// Quote:\n/// > shader_perf lives here.\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn unbalanced_tick_line_skipped() {
        let src = "/// A stray ` tick then shader_perf.\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn valid_idents_stay_silent() {
        for ident in ["macOS", "GraphQL", "sRGB", "WebAssembly", "MySQL", "CamelCase"] {
            let src = format!("/// The {ident} lane.\nfn go() {{}}");
            assert!(
                propose_fix("doc_markdown", &src).is_none(),
                "{ident} is a valid ident"
            );
        }
    }

    #[test]
    fn all_caps_and_plain_words_silent() {
        for word in ["NASA", "TODO", "IDs", "Vecs", "Clippy", "foo", "Bar"] {
            let src = format!("/// The {word} lane.\nfn go() {{}}");
            assert!(
                propose_fix("doc_markdown", &src).is_none(),
                "{word} must stay silent"
            );
        }
    }

    #[test]
    fn underscore_hyphen_mix_silent() {
        let src = "/// The foo_bar-baz shape.\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn escaped_underscore_silent() {
        let src = "/// The foo\\_bar emphasis.\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn possessive_split_silent() {
        let src = "/// clippy's own lint.\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn link_shaped_tokens_skipped() {
        // Intra-doc and external link shapes: clippy checks the link TEXT
        // through pulldown events; our token-level approximation refuses
        // the whole link-shaped word (documented divergence).
        for src in [
            "/// See [shader_perf](Vec) now.\nfn go() {}",
            "/// See [shader_perf](https://example.com/a_b) now.\nfn go() {}",
            "/// See <https://example.com/foo_bar> now.\nfn go() {}",
        ] {
            assert!(propose_fix("doc_markdown", src).is_none(), "{src}");
        }
    }

    #[test]
    fn inner_docs_process() {
        let src = "//! Crate shader_perf docs.\nfn go() {}";
        let out = propose_fix("doc_markdown", src).expect("fires");
        assert!(out.contains("`shader_perf`"), "{out}");
    }

    #[test]
    fn attr_form_doc_skipped() {
        // The `#[doc = "..."]` attribute form is a documented divergence:
        // the +3 marker slicing only holds for comment-style docs.
        let src = "#[doc = \"the shader_perf lane\"]\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn string_literal_fake_doc_untouched() {
        // THE load-bearing safety property: a string literal containing
        // doc-shaped text is never an attribute — a line scan would
        // backtick (and so rewrite) STRING CONTENT here.
        let src = "fn fixture() -> &'static str {\n    \"/// the shader_perf lane\\n\"\n}\nfn go() {}";
        assert!(propose_fix("doc_markdown", src).is_none());
    }

    #[test]
    fn fixpoint_stable() {
        let src = "/// Runs the shader_perf stage over a HashMap.\nfn stage() {}";
        let once = propose_fix("doc_markdown", src).expect("fires");
        assert!(propose_fix("doc_markdown", &once).is_none(), "idempotent");
    }

    #[test]
    fn multiple_words_one_line() {
        let src = "/// shader_perf meets ArcSwapGuard.\nfn go() {}";
        let out = propose_fix("doc_markdown", src).expect("fires");
        assert!(out.contains("`shader_perf` meets `ArcSwapGuard`"), "{out}");
    }
}
