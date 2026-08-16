//! Issue 025 T3 — `#[cube]` item extraction from Rust sources.
//!
//! `CubeCL` kernels authored as `#[cube]` / `#[cube(launch_unchecked)]` Rust
//! functions are the second **non-syn surface** (142 items / ~9.8k body lines
//! in riir-gpu, T1 inventory): `syn` refuses their DSL bodies
//! (Plan 005 Phase 5's recorded deferral), but a **token-tree walk** does not
//! — `proc_macro2` locates the attribute + item span without ever parsing the
//! body as Rust ([`proc_macro2::TokenStream::from_str`] only lexes).
//!
//! The extractor finds every cube-attributed `fn` and returns its source
//! slice + absolute line range, so [`super::wgsl_scan::WgslScanner`] can run
//! the same text patterns over the span with findings mapped back to real
//! file lines.
//!
//! ## Edge cases pinned (the issue's list + the real-tree shapes)
//!
//! | case | handling |
//! |---|---|
//! | `#[cube]` vs `#[cube(launch_unchecked)]` | both extracted; [`CubeItem::launch`] records the launch-family arg group |
//! | attrs BETWEEN cube and fn (`#[cube] #[cfg(test)] fn …` — real: `cubecl_runtime.rs:361`) | outer-attr groups after the cube attr are skipped when locating `fn` |
//! | attrs BEFORE cube (`#[cfg]`/`#[allow]` — real: `wall_cubecl.rs`, `weaver_gpu/mod.rs`) | item text starts at the **cube** attr (the item's identity), not at the preceding cfg |
//! | nested macros in the body (`comptime!(…)`, `terminate!()` — real: every kernel) | macro bodies are token groups; brace matching is unaffected |
//! | raw strings containing unbalanced braces or fake `#[cube] fn` text | a raw string is ONE `Literal` token — its contents are never walked |
//! | `#[cube]` mentions in `//` / `///` / `//!` comments (35 in the real tree) | comments are dropped by the lexer — never extracted |
//! | cube fn nested inside a `mod { }` / `impl { }` block | the walk recurses into brace groups (absolute line numbers preserved) |
//! | non-cube attrs (`#[inline] fn`), inner attrs (`#![…]`) | not matched — the attr group must start with the ident `cube` |
//!
//! ## Known limitations (accepted, documented)
//!
//! - A `macro_rules!` body containing `#[cube] fn` tokens would extract (the
//!   walk cannot distinguish macro-definition text from items). No such file
//!   exists in riir-gpu (grep-verified 2026-08-16); if one lands, gate on the
//!   enclosing ident.
//! - Unlexable source (unbalanced delimiters) yields no items — callers scan
//!   files that compile, so this is unreachable in practice.
//! - Columns are **0-based chars** (the proc-macro2 fallback lexer's
//!   semantics — a documented divergence from `proc_macro`'s 1-based); byte
//!   offsets are derived by walking chars, so multi-byte identifiers slice
//!   correctly. Both semantics are empirically pinned by the exact-text
//!   tests.

use proc_macro2::{Delimiter, LineColumn, TokenStream, TokenTree};
use std::str::FromStr;

/// One extracted `#[cube]`-attributed function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubeItem {
    /// The `fn` name (empty if the token after `fn` was not an ident — never
    /// happens in the real tree).
    pub name: String,
    /// `true` when the cube attr carries a launch-family argument group
    /// (`#[cube(launch_unchecked)]`, `#[cube(launch)]`), `false` for plain
    /// `#[cube]`.
    pub launch: bool,
    /// 1-based line of the leading `#` of the **cube** attribute (preceding
    /// `#[cfg]`/`#[allow]` attrs are not part of the item).
    pub start_line: usize,
    /// 1-based line of the item's last token (the body's closing brace).
    pub end_line: usize,
    /// The source slice of the whole item, cube attribute through closing
    /// brace. Line `n` of this text is file line `start_line + n - 1`.
    pub text: String,
}

/// Extract every `#[cube]`-attributed `fn` from a Rust source, in source
/// order. See the [module docs](self) for the pinned edge cases.
#[must_use]
pub fn extract_cube_items(source: &str) -> Vec<CubeItem> {
    let Ok(stream) = TokenStream::from_str(source) else {
        return Vec::new();
    };
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let line_starts = build_line_starts(source);
    let mut out = Vec::new();
    walk(&tokens, source, &line_starts, &mut out);
    out
}

/// Byte offset of each line start (line 1 = 0). Lines are `\n`-separated.
fn build_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Byte offset of the character at column `col` on `line`.
/// **0-based columns** — the proc-macro2 fallback lexer reports 0-based
/// char columns (a documented divergence from `proc_macro`'s 1-based;
/// empirically pinned by the nested-item test: an indented `#[cube]`'s span
/// start reported the 0-based column, one less than the 1-based position).
/// A `col` pointing one past the last char returns the line's content end.
fn line_col_to_byte(source: &str, line_starts: &[usize], line: usize, col: usize) -> usize {
    line_col_inner(source, line_starts, line, col, false)
}

/// Exclusive-end variant: byte offset JUST PAST the character at `col`
/// (0-based). The fallback lexer reports a group's `Span::end()` AT the
/// closing delimiter character (empirically pinned by the exact-text tests
/// — the slice was missing exactly the final `}`), so the end conversion
/// must include that char.
fn line_col_to_byte_exclusive(
    source: &str,
    line_starts: &[usize],
    line: usize,
    col: usize,
) -> usize {
    line_col_inner(source, line_starts, line, col, true)
}

fn line_col_inner(
    source: &str,
    line_starts: &[usize],
    line: usize,
    col: usize,
    include_char: bool,
) -> usize {
    let idx = line.saturating_sub(1);
    if idx >= line_starts.len() {
        return source.len();
    }
    let line_start = line_starts[idx];
    let next = line_starts.get(idx + 1).copied().unwrap_or(source.len());
    // Content end: exclusive of the line terminator.
    let mut content_end = next;
    let bytes = source.as_bytes();
    if content_end > line_start && bytes.get(content_end - 1) == Some(&b'\n') {
        content_end -= 1;
        if content_end > line_start && bytes.get(content_end - 1) == Some(&b'\r') {
            content_end -= 1;
        }
    }
    // `idx` = 0-based char index of the current char == the fallback's
    // column semantics. Return when it reaches `col`.
    for (seen, (off, ch)) in source[line_start..content_end].char_indices().enumerate() {
        if seen >= col {
            return if include_char {
                line_start + off + ch.len_utf8()
            } else {
                line_start + off
            };
        }
    }
    content_end
}

fn walk(tokens: &[TokenTree], source: &str, line_starts: &[usize], out: &mut Vec<CubeItem>) {
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                if let Some((item, consumed)) = try_match_cube_item(tokens, i, source, line_starts)
                {
                    out.push(item);
                    i += consumed;
                    continue;
                }
                i += 1;
            }
            // Recurse into brace groups: `mod tests { #[cube] fn … }`,
            // `impl … { … }`. Paren/bracket groups cannot carry outer attrs.
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                walk(&inner, source, line_starts, out);
                i += 1;
            }
            _ => i += 1,
        }
    }
}

/// Try to match a cube item starting at `tokens[i]` (a `#` punct). Returns
/// the item + the number of tokens consumed, or `None` when this `#` does
/// not start a `#[cube] fn` item.
fn try_match_cube_item(
    tokens: &[TokenTree],
    i: usize,
    source: &str,
    line_starts: &[usize],
) -> Option<(CubeItem, usize)> {
    // `#[…]` outer attribute (inner `#![…]` is not a cube item).
    let attr = match tokens.get(i + 1) {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket => g,
        _ => return None,
    };
    let attr_tokens: Vec<TokenTree> = attr.stream().into_iter().collect();
    match attr_tokens.first() {
        Some(TokenTree::Ident(id)) if *id == "cube" => {}
        _ => return None,
    }
    // Launch-family variant: any paren argument group after `cube`
    // (`launch_unchecked` in the real tree; `launch` is the same shape).
    let launch = attr_tokens.iter().skip(1).any(|t| {
        matches!(t, TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis)
    });

    // Skip further outer attrs between the cube attr and the fn
    // (real shape: `#[cube(launch_unchecked)] #[cfg(test)] fn …`).
    let mut k = i + 2;
    while matches!(tokens.get(k), Some(TokenTree::Punct(p)) if p.as_char() == '#')
        && matches!(
            tokens.get(k + 1),
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket
        )
    {
        k += 2;
    }
    // Optional visibility: `pub` / `pub(crate)` (not in the real tree; pinned
    // for forward-compat).
    if matches!(tokens.get(k), Some(TokenTree::Ident(id)) if *id == "pub") {
        k += 1;
        if matches!(
            tokens.get(k),
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis
        ) {
            k += 1;
        }
    }
    // Expect `fn`.
    if !matches!(tokens.get(k), Some(TokenTree::Ident(id)) if *id == "fn") {
        return None;
    }
    let name = match tokens.get(k + 1) {
        Some(TokenTree::Ident(id)) => id.to_string(),
        _ => String::new(),
    };
    // The body is the first brace group after the fn keyword — the signature
    // (name, generics, args, return type) cannot legally contain one.
    let body_idx = (k..tokens.len()).find(|&j| {
        matches!(tokens.get(j), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
    })?;

    // Item span: leading `#` start .. max token end over the whole item
    // (recursive, so nested macro groups cannot shrink it).
    let start = tokens[i].span().start();
    let end = max_end(&tokens[i..=body_idx]);
    let start_byte = line_col_to_byte(source, line_starts, start.line, start.column);
    let end_byte = line_col_to_byte_exclusive(source, line_starts, end.line, end.column);
    let text = source.get(start_byte..end_byte).unwrap_or_default().to_string();

    Some((
        CubeItem {
            name,
            launch,
            start_line: start.line,
            end_line: end.line,
            text,
        },
        body_idx - i + 1,
    ))
}

/// Max `Span::end()` over a token slice, recursing into groups.
fn max_end(tokens: &[TokenTree]) -> LineColumn {
    let mut max = tokens
        .first()
        .map_or(LineColumn { line: 1, column: 1 }, |t| t.span().end());
    for tt in tokens {
        let mut e = tt.span().end();
        if let TokenTree::Group(g) = tt {
            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
            if !inner.is_empty() {
                let inner_max = max_end(&inner);
                if (inner_max.line, inner_max.column) > (e.line, e.column) {
                    e = inner_max;
                }
            }
        }
        if (e.line, e.column) > (max.line, max.column) {
            max = e;
        }
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(items: &[CubeItem]) -> Vec<&str> {
        items.iter().map(|i| i.name.as_str()).collect()
    }

    /// The real tree shape: plain + launch variant, exact span slicing.
    /// This test also pins the proc-macro2 column base (1-based chars): a
    /// wrong base shifts `start_byte` and fails the text equality.
    #[test]
    fn plain_and_launch_variants_extract_with_exact_text() {
        let src = "// header\n#[cube]\nfn scalar(x: f32) -> f32 {\n    x * 2.0\n}\n\n#[cube(launch_unchecked)]\nfn add(a: &[f32], b: &[f32]) {\n    terminate!();\n}\n";
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["scalar", "add"]);
        assert!(!items[0].launch);
        assert!(items[1].launch);
        assert_eq!(items[0].start_line, 2);
        assert_eq!(items[0].end_line, 5);
        assert_eq!(
            items[0].text,
            "#[cube]\nfn scalar(x: f32) -> f32 {\n    x * 2.0\n}"
        );
        assert_eq!(items[1].start_line, 7);
        assert_eq!(items[1].end_line, 10);
        assert_eq!(
            items[1].text,
            "#[cube(launch_unchecked)]\nfn add(a: &[f32], b: &[f32]) {\n    terminate!();\n}"
        );
    }

    /// Real shape (`cubecl_runtime.rs:361`): an attr between cube and fn.
    #[test]
    fn attr_between_cube_and_fn_is_consumed() {
        let src = "#[cube(launch_unchecked)]\n#[cfg(all(feature = \"cubecl_runtime\", test))]\nfn gelu(input: &[f32]) {\n    comptime!(1.0f32);\n}\n";
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["gelu"]);
        assert!(items[0].launch);
        assert!(items[0].text.starts_with("#[cube(launch_unchecked)]"));
        assert!(items[0].text.contains("#[cfg(all(feature = \"cubecl_runtime\", test))]"));
        assert!(items[0].text.ends_with('}'));
    }

    /// Real shape (`wall_cubecl.rs`): cfg/allow attrs BEFORE the cube attr —
    /// the item starts at the cube attr, not at the cfg.
    #[test]
    fn preceding_attrs_are_not_part_of_the_item() {
        let src = "#[cfg(feature = \"cubecl_runtime\")]\n#[allow(clippy::x)]\n#[cube]\nfn k() {\n}\n";
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["k"]);
        assert_eq!(items[0].start_line, 3);
        assert!(items[0].text.starts_with("#[cube]"));
        assert!(!items[0].text.contains("#[cfg"));
    }

    /// Raw string with unbalanced braces + a fake cube item inside: a raw
    /// string is one Literal token — never walked, never mis-brace-matched.
    /// (The `r##"…"##` test literal embeds an `r#"…"#` raw string verbatim.)
    #[test]
    fn raw_strings_are_opaque() {
        let src = r##"const S: &str = r#"#[cube]
fn evil() { { { ("#;
#[cube]
fn good(x: f32) -> f32 {
    x
}
"##;
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["good"]);
        assert_eq!(items[0].start_line, 3);
    }

    /// Comment mentions (35 in the real tree) are dropped by the lexer.
    #[test]
    fn comment_mentions_are_ignored() {
        let src = "//! docs mention `#[cube]` here\n// #[cube] fn fake() {}\n#[cube]\nfn real() {\n}\n";
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["real"]);
    }

    /// A cube fn nested inside a `mod` block (not in the real tree today,
    /// but the walk must handle it) keeps absolute line numbers.
    #[test]
    fn nested_mod_item_keeps_absolute_lines() {
        let src = "mod tests {\n    #[cube]\n    fn inner(x: f32) -> f32 {\n        x\n    }\n}\n";
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["inner"]);
        assert_eq!(items[0].start_line, 2);
        assert_eq!(items[0].end_line, 5);
        assert!(items[0].text.starts_with("#[cube]"));
        assert!(items[0].text.ends_with('}'));
    }

    /// Non-cube attrs and plain fns are not extracted; empty/garbage input
    /// is a no-op, and repeated extraction is deterministic.
    #[test]
    fn non_cube_and_degenerate_inputs() {
        assert!(extract_cube_items("#[inline]\nfn a() {\n}\n").is_empty());
        assert!(extract_cube_items("fn a() {\n}\n").is_empty());
        assert!(extract_cube_items("").is_empty());
        // Unbalanced delimiters: lexer error → no items (documented limit).
        assert!(extract_cube_items("#[cube]\nfn a() {\n").is_empty());
        let src = "#[cube]\nfn a() {\n}\n";
        assert_eq!(extract_cube_items(src), extract_cube_items(src));
    }

    /// Tokenization (not parsing) is enough: a WGSL-shaped body line lexes as
    /// idents/puncts/literals, so such an item still extracts — this is how
    /// scan-line mapping stays testable against synthetic corpus text.
    #[test]
    fn wgsl_shaped_body_line_still_extracts() {
        let src = "#[cube]\nfn k() {\n    var buf: array<f32, 256>;\n}\n";
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["k"]);
        assert!(items[0].text.contains("var buf: array<f32, 256>;"));
    }

    /// Multi-byte chars BEFORE an item (e.g. in an earlier string) must not
    /// skew the byte slicing — columns are chars, offsets are bytes.
    #[test]
    fn multi_byte_chars_do_not_skew_offsets() {
        let src = "const NOTE: &str = \"日本語テキスト\";\n#[cube]\nfn k(x: f32) -> f32 {\n    x\n}\n";
        let items = extract_cube_items(src);
        assert_eq!(names(&items), ["k"]);
        assert_eq!(items[0].start_line, 2);
        assert_eq!(
            items[0].text,
            "#[cube]\nfn k(x: f32) -> f32 {\n    x\n}"
        );
    }
}
