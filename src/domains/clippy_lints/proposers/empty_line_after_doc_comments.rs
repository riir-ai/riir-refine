//! `clippy::empty_line_after_doc_comments` — `/// docs` + blank line(s) +
//! item → delete the blank run so the doc re-attaches (Batch 168 / Plan 159
//! — the coverage-report re-arm pack XII; the family fixture verified to
//! fire under the plain oracle and the attr-intervened shape verified
//! SILENT, 2026-09-21, `scripts/plan159_pack12_probe.py`).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! The deleted span is pure whitespace by construction (the blank lines
//! between the doc block and the documented item). Comments carry no
//! semantics; the edit restores exactly the attachment the lint asks for
//! ("if the empty line is unintentional, remove it"). Nothing else in the
//! file can move: the edit is one byte range that begins after the doc
//! line's newline and ends at the next non-blank line.
//!
//! ## Guards (the bounded fix-space)
//!
//! - Comment-style OUTER docs only: `///`. Inner docs (`//!`) refuse — a
//!   different lint's territory. `#[doc = "..."]` attribute-form docs and
//!   `/** */` block docs are invisible here (`doc_lines` skips both).
//! - The doc lines are ITEM-LEVEL `#[doc]` attributes only, collected
//!   through `visit_item` / `visit_impl_item` / `visit_trait_item` — a
//!   string literal containing doc-shaped text is not a doc, and a
//!   statement-position stray doc is invisible, by construction.
//! - The blank run must contain at least one blank line and must resolve
//!   to a line that is none of: another doc line, an attribute (`#`), a
//!   closing `}`. The attr case is MEASURED upstream silence (the probe:
//!   `/// docs` + blank + `#[derive]` never fires) — mirrored, not
//!   invented. A blank run to EOF has no item to document — silent.
//! - Multi-line blank runs collapse in one edit (the probe verified the
//!   triple-blank shape fires; one deletion restores attachment).

use syn::visit::Visit;
use syn::{File, ImplItem, Item, TraitItem};

use super::SourceEdit;
use super::helpers::byte_range;

/// Item-position `#[doc]` attribute byte ranges, source order. Item level
/// only: statement-position stray docs and string-literal doc-shaped text
/// are invisible by construction.
struct ItemDocs<'a> {
    ranges: &'a mut Vec<std::ops::Range<usize>>,
}

impl<'ast> Visit<'ast> for ItemDocs<'_> {
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

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut ranges = Vec::new();
    {
        let mut v = ItemDocs {
            ranges: &mut ranges,
        };
        v.visit_file(file);
    }
    ranges.sort_unstable_by_key(|r| r.start);
    let mut edits = Vec::new();
    for range in ranges {
        // Comment-style OUTER docs only (`///`). `#[doc = "..."]`/`/** */`
        // attribute spans do not start with the marker; `//!` is an inner
        // (file-level) attr and never reaches this loop.
        if !source[range.start..].starts_with("///") {
            continue;
        }
        let line_end = source[range.start..]
            .find('\n')
            .map_or(source.len(), |i| range.start + i);
        if line_end >= source.len() {
            continue; // the doc line is the last line — nothing follows
        }
        // Walk the lines after the doc line: count the blank run, stop at
        // the first non-blank line (or EOF).
        let mut cursor = line_end + 1;
        let mut blank_found = false;
        let mut next_start = None;
        while cursor < source.len() {
            let nl = source[cursor..]
                .find('\n')
                .map_or(source.len(), |i| cursor + i);
            if source[cursor..nl].trim().is_empty() {
                blank_found = true;
                cursor = nl + 1;
                continue;
            }
            next_start = Some(cursor);
            break;
        }
        let (Some(next_start), true) = (next_start, blank_found) else {
            // No blank before the next line (docs never fire without a
            // gap), or the blank run runs to EOF (no item to document).
            continue;
        };
        let next_trim = source[next_start..]
            .split('\n')
            .next()
            .unwrap_or_default()
            .trim_start();
        if next_trim.starts_with("///")
            || next_trim.starts_with("//!")
            || next_trim.starts_with('#')
            || next_trim.starts_with('}')
        {
            // A second doc block, an attribute-intervened item (measured
            // upstream silence), or a stray in-block doc — all silent.
            continue;
        }
        edits.push(SourceEdit {
            start: line_end + 1,
            end: next_start,
            replacement: String::new(),
        });
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(
            propose_fix(
                "empty_line_after_doc_comments",
                "/// Adds one.\nfn f(hits: &mut Vec<u8>) -> u8 { hits.push(0); 1 }",
            )
            .is_none()
        );
    }

    #[test]
    fn blank_line_collapses() {
        let out = propose_fix(
            "empty_line_after_doc_comments",
            "/// Adds one.\n\nfn f(hits: &mut Vec<u8>) -> u8 { hits.push(0); 1 }",
        )
        .expect("matched");
        assert!(
            out.contains("/// Adds one.\nfn f("),
            "blank not collapsed: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn triple_blank_collapses_in_one_edit() {
        let out = propose_fix(
            "empty_line_after_doc_comments",
            "/// D.\n\n\n\nfn f() -> u8 { 3 }",
        )
        .expect("matched");
        assert!(out.contains("/// D.\nfn f()"), "run not collapsed: {out}");
        assert!(!out.contains("\n\n\n"), "blanks remain: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn attr_after_gap_is_silent_probed_shape() {
        // The probe's measured upstream silence: `/// docs` + blank +
        // `#[derive]` never fires — mirrored, never invented.
        assert!(
            propose_fix(
                "empty_line_after_doc_comments",
                "/// Docs for bar.\n\n#[derive(Debug)]\nstruct B { y: u8 }",
            )
            .is_none()
        );
    }

    #[test]
    fn second_doc_block_refuses() {
        assert!(
            propose_fix(
                "empty_line_after_doc_comments",
                "/// a\n\n/// b\nfn f() -> u8 { 1 }",
            )
            .is_none()
        );
    }

    #[test]
    fn inner_doc_refuses() {
        assert!(
            propose_fix(
                "empty_line_after_doc_comments",
                "//! crate docs\n\nfn f() -> u8 { 1 }",
            )
            .is_none()
        );
    }

    #[test]
    fn string_interior_doc_shape_invisible() {
        // Doc-shaped text inside a string literal is not a doc
        // (`doc_lines` reads real `#[doc]` attrs only).
        assert!(
            propose_fix(
                "empty_line_after_doc_comments",
                "fn s() {\n    let x = \"/// q\\n\\nfn z() {}\";\n    let _ = x;\n}",
            )
            .is_none()
        );
    }

    #[test]
    fn eof_after_blank_refuses() {
        assert!(propose_fix("empty_line_after_doc_comments", "/// D.\n\n").is_none());
    }

    #[test]
    fn mid_block_empty_doc_line_is_not_a_fire() {
        assert!(
            propose_fix(
                "empty_line_after_doc_comments",
                "/// a\n///\n/// b\nfn f() -> u8 { 1 }",
            )
            .is_none()
        );
    }

    #[test]
    fn stray_doc_before_closing_brace_refuses() {
        assert!(
            propose_fix(
                "empty_line_after_doc_comments",
                "fn g() -> u8 {\n    /// stray\n\n    1\n}",
            )
            .is_none()
        );
    }

    #[test]
    fn nested_module_indent_preserved() {
        let out = propose_fix(
            "empty_line_after_doc_comments",
            "mod m {\n    /// D.\n\n    fn f() -> u8 { 1 }\n}",
        )
        .expect("matched");
        assert!(
            out.contains("/// D.\n    fn f()"),
            "item indent lost: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let first = propose_fix(
            "empty_line_after_doc_comments",
            "/// D.\n\nfn f() -> u8 { 1 }",
        )
        .expect("matched");
        assert!(propose_fix("empty_line_after_doc_comments", &first).is_none());
    }
}
