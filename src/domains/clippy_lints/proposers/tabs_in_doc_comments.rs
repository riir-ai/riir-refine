//! `clippy::tabs_in_doc_comments` — every TAB inside a doc comment becomes
//! four spaces (Batch 171 / Plan 163 — the coverage-report pack XIV;
//! envelope verified under the plain oracle 2026-09-21,
//! `scripts/plan163_pack14_probe.py`).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! Comment bytes carry zero semantics; replacing `\t` with the same
//! leading-width spaces is clippy's own suggestion shape, measured per
//! occurrence (one `\t` → `'    '`, two → eight spaces — probe). The probe
//! pinned the territory WIDER than intuition: tabs inside FENCED CODE
//! BLOCKS in docs fire too (clippy rewrites code samples verbatim — L4 of
//! the codeblock fixture) and `//!` inner docs and `/** */` block docs
//! fire; regular `//` comments stay SILENT. All mirrored, not invented.
//!
//! ## Guards (the bounded fix-space)
//!
//! - Doc-comment attrs only, collected at item/impl-item/trait-item level
//!   PLUS the file-level inner docs (`//!` — the `File`'s own attrs; the
//!   probe's `tid_inner` row fires, so unlike
//!   `empty_line_after_doc_comments` the inner docs ARE this lint's
//!   territory). A string literal containing tab-bearing doc-shaped text
//!   is not a doc attr and is invisible by construction.
//! - Line docs: the marker must be `///` NOT followed by a fourth `/`
//!   (`////` is a regular comment — the `four_forward_slashes` lint's
//!   territory) or `//!`. Tabs AFTER the marker to end-of-line are edited.
//! - Block docs: `/**` NOT followed by `*` (`/***` is a regular comment,
//!   the same law as `////`). Tabs from after the opening marker to the
//!   closing `*/` are edited.
//! - Statement/expression-position doc comments are out of scope in v1
//!   (the `empty_line_after_doc_comments` collection precedent) — the
//!   documented narrower-than-clippy divergence.
//! - Fixpoint by construction: after the edit no tab remains in the doc.

use syn::visit::Visit;
use syn::{File, ImplItem, Item, TraitItem};

use super::SourceEdit;
use super::helpers::byte_range;

const SPACES: &str = "    ";

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut ranges = Vec::new();
    {
        let mut v = DocRanges {
            ranges: &mut ranges,
        };
        // file-level inner docs (`//!`) live on the File's own attrs
        for a in &file.attrs {
            if a.path().is_ident("doc") {
                v.ranges.push(byte_range(a));
            }
        }
        v.visit_file(file);
    }
    ranges.sort_unstable_by_key(|r| r.start);
    let mut edits = Vec::new();
    for range in ranges {
        let tail = &source[range.start..source.len().min(range.end)];
        let territory: Option<&str> = if tail.starts_with("////") || tail.starts_with("/***") {
            // regular comments wearing doc prefixes
            None
        } else if let Some(rest) = tail.strip_prefix("///") {
            // line doc: after the marker to end of range (the line)
            Some(rest)
        } else if let Some(rest) = tail.strip_prefix("//!") {
            Some(rest)
        } else if let Some(rest) = tail.strip_prefix("/**") {
            // block doc: inside the markers
            Some(&rest[..rest.len().saturating_sub(2)])
        } else {
            // `#[doc = "..."]` attribute form — the string literal's
            // escapes make byte-level tab edits wrong
            None
        };
        let Some(territory) = territory else { continue };
        let marker_len = 3;
        for (off, b) in territory.bytes().enumerate() {
            if b == b'\t' {
                edits.push(SourceEdit {
                    start: range.start + marker_len + off,
                    end: range.start + marker_len + off + 1,
                    replacement: SPACES.to_string(),
                });
            }
        }
    }
    edits
}

struct DocRanges<'a> {
    ranges: &'a mut Vec<std::ops::Range<usize>>,
}

impl<'ast> Visit<'ast> for DocRanges<'_> {
    fn visit_item(&mut self, i: &'ast Item) {
        let attrs: &[syn::Attribute] = match i {
            Item::Const(x) => x.attrs.as_slice(),
            Item::Enum(x) => x.attrs.as_slice(),
            Item::ExternCrate(x) => x.attrs.as_slice(),
            Item::Fn(x) => x.attrs.as_slice(),
            Item::ForeignMod(x) => x.attrs.as_slice(),
            Item::Impl(x) => x.attrs.as_slice(),
            Item::Macro(x) => x.attrs.as_slice(),
            Item::Mod(x) => x.attrs.as_slice(),
            Item::Static(x) => x.attrs.as_slice(),
            Item::Struct(x) => x.attrs.as_slice(),
            Item::Trait(x) => x.attrs.as_slice(),
            Item::TraitAlias(x) => x.attrs.as_slice(),
            Item::Type(x) => x.attrs.as_slice(),
            Item::Union(x) => x.attrs.as_slice(),
            Item::Use(x) => x.attrs.as_slice(),
            _ => &[],
        };
        for a in attrs {
            if a.path().is_ident("doc") {
                self.ranges.push(byte_range(a));
            }
        }
        syn::visit::visit_item(self, i);
    }

    fn visit_impl_item(&mut self, i: &'ast ImplItem) {
        let attrs: &[syn::Attribute] = match i {
            ImplItem::Const(x) => x.attrs.as_slice(),
            ImplItem::Fn(x) => x.attrs.as_slice(),
            ImplItem::Macro(x) => x.attrs.as_slice(),
            ImplItem::Type(x) => x.attrs.as_slice(),
            _ => &[],
        };
        for a in attrs {
            if a.path().is_ident("doc") {
                self.ranges.push(byte_range(a));
            }
        }
        syn::visit::visit_impl_item(self, i);
    }

    fn visit_trait_item(&mut self, i: &'ast TraitItem) {
        let attrs: &[syn::Attribute] = match i {
            TraitItem::Const(x) => x.attrs.as_slice(),
            TraitItem::Fn(x) => x.attrs.as_slice(),
            TraitItem::Macro(x) => x.attrs.as_slice(),
            TraitItem::Type(x) => x.attrs.as_slice(),
            _ => &[],
        };
        for a in attrs {
            if a.path().is_ident("doc") {
                self.ranges.push(byte_range(a));
            }
        }
        syn::visit::visit_trait_item(self, i);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const SEED: &str = "///\tA doc comment with a leading tab.\npub fn f() -> u64 {\n    1\n}";

    #[test]
    fn seed_fires_and_rewrites() {
        let out = propose_fix("tabs_in_doc_comments", SEED).expect("matched");
        assert!(out.starts_with("///    A doc"), "tab not spaced: {out:?}");
        assert!(!out.contains('\t'), "a tab survived: {out:?}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mid_text_tab() {
        let src = "/// A doc comment with a\tmid-text tab.\npub fn f() -> u64 {\n    2\n}";
        let out = propose_fix("tabs_in_doc_comments", src).expect("matched");
        assert!(out.contains("with a    mid"), "mid tab not spaced: {out:?}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn two_tabs_double_spaces() {
        // probe: two tabs -> eight spaces (two per-tab replacements)
        let src = "///\t\tTwo tabs.\npub fn f() -> u64 {\n    3\n}";
        let out = propose_fix("tabs_in_doc_comments", src).expect("matched");
        assert!(
            out.starts_with("///        Two"),
            "double tab not doubled: {out:?}"
        );
    }

    #[test]
    fn inner_doc_fires() {
        // probe tid_inner: `//!` IS this lint's territory (unlike
        // empty_line_after_doc_comments, whose scope is outer docs only)
        let src = "//!\tModule doc with a tab.\npub fn f() -> u64 {\n    4\n}";
        let out = propose_fix("tabs_in_doc_comments", src).expect("matched");
        assert!(
            out.starts_with("//!    Module"),
            "inner tab not spaced: {out:?}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn block_doc_fires() {
        // probe tid_block: `/** */` fires; edits stay inside the markers
        let src = "/**\tBlock doc with a tab. */\npub fn f() -> u64 {\n    5\n}";
        let out = propose_fix("tabs_in_doc_comments", src).expect("matched");
        assert!(
            out.starts_with("/**    Block"),
            "block tab not spaced: {out:?}"
        );
        assert!(out.contains(" */"), "closing marker damaged: {out:?}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn code_block_tab_fires() {
        // probe tid_codeblock: clippy rewrites tabs inside fenced code
        // samples too — mirrored verbatim
        let src = "/// Example:\n/// ```\n///\tlet x = 1;\n/// ```\npub fn f() -> u64 {\n    6\n}";
        let out = propose_fix("tabs_in_doc_comments", src).expect("matched");
        assert!(
            out.contains("///    let x = 1;"),
            "codeblock tab not spaced: {out:?}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn regular_comment_silent() {
        // probe: regular `//` comments are SILENT — mirrored
        let src = "//\tA regular comment with a tab.\npub fn f() -> u64 {\n    7\n}";
        assert!(propose_fix("tabs_in_doc_comments", src).is_none());
    }

    #[test]
    fn four_slashes_is_regular() {
        // `////` is a regular comment (the four_forward_slashes law) — the
        // marker guard refuses before any edit
        let src = "////\tFour slashes.\npub fn f() -> u64 {\n    8\n}";
        assert!(propose_fix("tabs_in_doc_comments", src).is_none());
    }

    #[test]
    fn string_literal_tab_silent() {
        // a string literal carrying a tab is not a doc comment — invisible
        // by construction (the doc-attr collection law); the REAL doc tab
        // beside it still fires
        let src = "pub fn f() -> &'static str {\n    \"\ttab in a string\"\n}\n///\treal doc\npub fn g() -> u64 {\n    1\n}";
        let out = propose_fix("tabs_in_doc_comments", src).expect("matched");
        assert!(
            out.contains("\ttab in a string"),
            "string tab touched: {out:?}"
        );
        assert!(
            out.contains("///    real doc"),
            "doc tab not spaced: {out:?}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn attr_form_doc_refuses() {
        // `#[doc = "..."]` attribute form: the literal's escapes make
        // byte-level tab edits wrong — refused (v1 divergence)
        let src = "#[doc = \"\\ttab via escape\"]\npub fn f() -> u64 {\n    9\n}";
        assert!(propose_fix("tabs_in_doc_comments", src).is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let first = propose_fix("tabs_in_doc_comments", SEED).expect("matched");
        assert!(propose_fix("tabs_in_doc_comments", &first).is_none());
    }
}
