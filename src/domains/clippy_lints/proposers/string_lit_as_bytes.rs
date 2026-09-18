//! `clippy::string_lit_as_bytes` — `"abc".as_bytes()` → `b"abc"` (Batch
//! 161 / Plan 152 — the frontier-lead mining pack VII; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery, 2026-09-18
//! (`scripts/plan152_frontier_probe.py`); the default-groups arm is SILENT —
//! the lint is NURSERY/allow-by-default, pinned by the same probe; the
//! Batch 160 miner comment's "default-level perf" was wrong on both counts).
//!
//! ## Why the rewrite is identity where it can be (the FIXABLE bar)
//!
//! `b"abc"` is `&'static [u8; 3]`; `"abc".as_bytes()` is `&'static [u8]`.
//! At every use where a `&[u8]` (or wider) is expected, the array-ref
//! coerces to the slice-ref with the same bytes and the same lifetime —
//! construction-identical. This is exactly clippy's own `MachineApplicable`
//! suggestion (measured by the plan152 probe: `'b"abc"'`, `'br"a\\nc"'`,
//! `'b"a\\nb"'`), and it carries clippy's own documented known problem:
//! inference-sensitive contexts (`fn f<T: ?Sized>(t: &T)` picking
//! `T = [u8; 3]`, `into_iter()` yielding the array by value) can change
//! behavior — the `--fix --verify` compile gate reverts exactly those
//! edits. We ship the same posture as upstream, with the divergence
//! documented here.
//!
//! ## Guards (the bounded fix-space)
//!
//! - The receiver must be the string literal ITSELF (a `LitStr`): paren
//!   (`("abc").as_bytes()`), reference, and named receivers all refuse —
//!   the lint only fires on literals anyway.
//! - ASCII-only literal content: byte-string literals reject non-ASCII
//!   source characters, and clippy's own lint is ASCII-scoped (measured:
//!   `"héllo".as_bytes()` is SILENT).
//! - NO `\u{` escape in the literal's source text: byte strings cannot
//!   spell unicode escapes. Measured: clippy FIRES on `"\u{41}".as_bytes()`
//!   and suggests `b"\u{41}"` — which DOES NOT COMPILE (an upstream
//!   suggestion bug for escape-spelled ASCII); we are deliberately
//!   stricter and refuse the shape.
//! - Every other escape (`\n`, `\t`, `\\`, `\x41`, `\0`) is valid
//!   byte-string syntax and rides the snippet VERBATIM (measured:
//!   clippy's suggestions re-emit the same spelling).
//! - Raw strings prefix `b` before the `r`: `r"a\n"` → `br"a\n"`
//!   (measured: clippy emits exactly this).
//! - Exactly 0 args; attrs on the call refuse; comments inside the
//!   replaced span refuse (the whole span is re-spelled).

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Lit, LitStr};

use super::helpers::byte_range;
use super::{edit, SourceEdit};
use crate::edit_emitter::drops_interior_comment;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = self.try_rewrite(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, m: &ExprMethodCall) -> Option<String> {
        if m.method != "as_bytes" || !m.args.is_empty() || !m.attrs.is_empty() {
            return None;
        }
        // The receiver must be the literal itself.
        let Expr::Lit(lit) = &*m.receiver else { return None };
        let Lit::Str(s) = &lit.lit else { return None };
        let lit_text = self.lit_snippet(s)?;
        if !literal_is_ascii_bytes(&lit_text) {
            return None;
        }
        let call_span = byte_range(m);
        if drops_interior_comment(self.source, call_span, &[]) {
            return None;
        }
        Some(format!("b{lit_text}"))
    }

    /// The literal's source snippet, quotes and raw prefix included
    /// (`"abc"`, `r"k9"`, `r#"x"#`) — re-spelled verbatim under the `b`
    /// prefix so every escape keeps the author's exact spelling.
    fn lit_snippet(&self, s: &LitStr) -> Option<String> {
        self.source.get(byte_range(s)).map(str::to_string)
    }
}

/// The byte-literal admissibility audit on the snippet's QUOTED content:
/// every char ASCII, and no `\u{` escape (byte strings cannot spell it).
fn literal_is_ascii_bytes(lit_text: &str) -> bool {
    let Some(start) = lit_text.find('"') else { return false };
    let Some(end) = lit_text.rfind('"') else { return false };
    if end <= start {
        return false;
    }
    let content = &lit_text[start + 1..end];
    content.is_ascii() && !content.contains("\\u{")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); b\"abc\" }",
        )
        .is_none());
    }

    #[test]
    fn plain_fires() {
        let out = propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"abc\".as_bytes() }",
        )
        .expect("matched");
        assert!(out.contains("b\"abc\""), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn second_family_case_fires() {
        let out = propose_fix(
            "string_lit_as_bytes",
            "fn dat(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"k9\".as_bytes() }",
        )
        .expect("matched");
        assert!(out.contains("b\"k9\""), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn raw_string_fires() {
        let out = propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); r\"a\\nc\".as_bytes() }",
        )
        .expect("matched");
        assert!(out.contains("br\"a\\nc\""), "raw not b-prefixed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn escape_spelling_rides() {
        let out = propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"a\\nb\".as_bytes() }",
        )
        .expect("matched");
        assert!(out.contains("b\"a\\nb\""), "escape not verbatim: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unicode_escape_refuses() {
        // clippy fires here and suggests `b"\u{41}"` — which does not
        // compile (byte strings cannot spell unicode escapes; the upstream
        // suggestion bug, measured by the plan152 probe). We refuse.
        assert!(propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"\\u{41}\".as_bytes() }",
        )
        .is_none());
    }

    #[test]
    fn non_ascii_refuses() {
        // Byte literals reject non-ASCII source chars; clippy's own lint
        // is ASCII-scoped (measured silent).
        assert!(propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"h\u{e9}llo\".as_bytes() }",
        )
        .is_none());
    }

    #[test]
    fn paren_receiver_refuses() {
        assert!(propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); (\"abc\").as_bytes() }",
        )
        .is_none());
    }

    #[test]
    fn named_receiver_refuses() {
        assert!(propose_fix(
            "string_lit_as_bytes",
            "fn n(s: &str, hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); s.as_bytes() }",
        )
        .is_none());
    }

    #[test]
    fn nested_in_larger_expr_fires() {
        let out = propose_fix(
            "string_lit_as_bytes",
            "fn n(hits: &mut Vec<u8>) -> usize { hits.push(0); \"abc\".as_bytes().len() }",
        )
        .expect("matched");
        assert!(out.contains("b\"abc\".len()"), "not in place: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "string_lit_as_bytes",
            "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"abc\"./* bytes */as_bytes() }",
        )
        .is_none());
    }
}
