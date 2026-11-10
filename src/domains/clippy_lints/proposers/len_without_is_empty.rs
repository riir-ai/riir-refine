//! `clippy::len_without_is_empty` — an inherent impl with a public
//! `len(&self)` and no `is_empty` anywhere on the type → INSERT
//! `pub fn is_empty(&self) -> bool { self.len() == 0 }` before the impl's
//! closing brace (Batch 175 / Plan 169 — the coverage-report re-arm pack
//! XV; the family fixture verified to fire under the plain oracle,
//! 2026-09-24, `scripts/plan169_pack15_probe.py`).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! The edit is purely ADDITIVE inside an existing impl (the landed
//! `new_without_default` insertion precedent): no existing byte moves, and
//! adding an inherent method cannot change the meaning of code that
//! already compiled — every prior `is_empty` use on the type was a compile
//! error (unresolved method), so the method only turns broken code into
//! working code. The body `self.len() == 0` is clippy's own canonical
//! spelling for the method it asks for.
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - **Inherent impls only** (`trait_` none); a `pub fn len` in a trait
//!   DEFINITION or a trait impl refuses in v1 — inserting a required
//!   method into a trait breaks every downstream impl (the surface-break
//!   class, semantic not measured; the documented narrower-than-clippy
//!   divergence).
//! - The `len` shape: exactly `&self` (by-ref, non-mut, no subpat) + zero
//!   other inputs, no method generics, not `unsafe`, `pub` (plain `pub`
//!   is the conservative reading of the description's "public" term), and
//!   returning a primitive INT (`self.len() == 0` needs `PartialEq` with
//!   `0`; a non-int return would not compile). `const fn len` is admitted
//!   — callable from a non-const `is_empty`.
//! - **No `is_empty` fn anywhere on the type in this file** — same impl,
//!   another inherent impl, or a trait impl (inherent methods shadow trait
//!   methods for method-call syntax, so a trait-`is_empty` sibling is the
//!   conservative refusal). A cross-file sibling is invisible to a
//!   parse-only proposer; the compile gate reverts the E0592-class
//!   duplicate (never silent).
//! - Insertion point: before the impl's CLOSING brace, byte-matched off
//!   the impl span; a single-line impl gets the single-line spelling so
//!   the line never breaks mid-expression. The inserted block carries the
//!   `len` method's own line indentation.

use std::collections::{BTreeSet, HashSet};

use syn::visit::Visit;
use syn::{File, ImplItem, ItemImpl, ReturnType, Type, Visibility};

use super::SourceEdit;
use super::helpers::byte_range;

/// One admissible `len`-bearing inherent impl: where/how to insert.
struct Candidate {
    type_name: String,
    /// Byte offset just BEFORE the impl's closing `}`.
    insert_at: usize,
    /// The impl span contains a newline → multi-line insertion form.
    multiline: bool,
    indent: String,
}

#[derive(Default)]
struct Scan<'a> {
    source: &'a str,
    len_impls: Vec<Candidate>,
    /// Types with an `is_empty` fn in ANY of this file's impls.
    types_with_is_empty: BTreeSet<String>,
    /// Types already holding a Candidate — the second `len` on one type is
    /// E0592-unreachable, but the guard keeps the candidate list unique.
    claimed: HashSet<String>,
}

/// The plain-ident reading of a type (`W`) — generic/qualified types refuse.
fn simple_type_name(ty: &Type) -> Option<String> {
    let Type::Path(p) = ty else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    let seg = &p.path.segments[0];
    if !seg.arguments.is_none() {
        return None;
    }
    Some(seg.ident.to_string())
}

/// The `pub fn len(&self) -> <int>` shape the inserted body can call.
fn is_admissible_len(f: &syn::ImplItemFn) -> bool {
    if !matches!(f.vis, Visibility::Public(_)) || f.sig.ident != "len" {
        return false;
    }
    if !f.sig.generics.params.is_empty() || f.sig.unsafety.is_some() {
        return false;
    }
    // Exactly `&self`: the `FnArg::Receiver` spelling (the standard method
    // form), by-ref, non-mut. By-value / `&mut self` receivers cannot be
    // called through the inserted `&self` receiver (E0507/E0596) —
    // refused, not hoped away.
    if f.sig.inputs.len() != 1 {
        return false;
    }
    match f.sig.inputs.first() {
        Some(syn::FnArg::Receiver(rec)) => {
            if rec.reference.is_none() || rec.mutability.is_some() {
                return false;
            }
        }
        _ => return false,
    }
    // The body `self.len() == 0` needs an int return (PartialEq<usize>).
    matches!(
        &f.sig.output,
        ReturnType::Type(_, ty)
            if matches!(**ty, Type::Path(ref p)
                if p.qself.is_none()
                    && p.path.segments.len() == 1
                    && p.path.segments[0].arguments.is_none()
                    && INT_RETURNS.contains(&p.path.segments[0].ident.to_string().as_str()))
    )
}

fn any_named_is_empty(i: &ItemImpl) -> bool {
    i.items
        .iter()
        .any(|item| matches!(item, ImplItem::Fn(f) if f.sig.ident == "is_empty"))
}

/// The whitespace prefix of the line containing byte offset `at`, if the
/// whole prefix is whitespace (else `None` — the method is not at a line
/// start).
fn line_whitespace_prefix(source: &str, at: usize) -> Option<String> {
    let line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
    let prefix = &source[line_start..at];
    if !prefix.is_empty() && prefix.chars().all(char::is_whitespace) {
        Some(prefix.to_string())
    } else {
        None
    }
}

impl<'ast> Visit<'ast> for Scan<'ast> {
    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let name = simple_type_name(&i.self_ty);
        let Some(n) = name else {
            syn::visit::visit_item_impl(self, i);
            return;
        };
        if any_named_is_empty(i) {
            self.types_with_is_empty.insert(n.clone());
        }
        // Inherent impls only — trait impls only feed the is_empty set.
        if i.trait_.is_none() && !self.claimed.contains(&n) {
            for item in &i.items {
                let ImplItem::Fn(f) = item else { continue };
                if !is_admissible_len(f) {
                    continue;
                }
                let range = byte_range(i);
                let multiline = self.source[range.clone()].contains('\n');
                // Member indent: the len method's own line prefix, else the
                // impl's line prefix + one level, else flush-left.
                let indent = line_whitespace_prefix(self.source, byte_range(f).start)
                    .or_else(|| {
                        line_whitespace_prefix(self.source, range.start)
                            .map(|ind| format!("{ind}    "))
                    })
                    .unwrap_or_default();
                self.len_impls.push(Candidate {
                    type_name: n.clone(),
                    insert_at: range.end.saturating_sub(1),
                    multiline,
                    indent,
                });
                self.claimed.insert(n);
                break;
            }
        }
        syn::visit::visit_item_impl(self, i);
    }
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = Scan {
        source,
        len_impls: Vec::new(),
        types_with_is_empty: BTreeSet::new(),
        claimed: HashSet::new(),
    };
    scan.visit_file(file);
    let mut edits = Vec::new();
    for cand in scan.len_impls {
        let Candidate {
            type_name,
            insert_at,
            multiline,
            indent,
        } = cand;
        if scan.types_with_is_empty.contains(&type_name) {
            continue;
        }
        let ind = &indent;
        let replacement = if multiline {
            // The byte before the impl's `}` is normally the last member's
            // `\n`; only add one when it is not.
            let lead = if insert_at > 0 && source.as_bytes().get(insert_at - 1) == Some(&b'\n') {
                String::new()
            } else {
                "\n".to_string()
            };
            format!(
                "{lead}{ind}pub fn is_empty(&self) -> bool {{\n{ind}    self.len() == 0\n{ind}}}\n"
            )
        } else {
            let lead = if insert_at > 0
                && source
                    .as_bytes()
                    .get(insert_at - 1)
                    .is_some_and(u8::is_ascii_whitespace)
            {
                ""
            } else {
                " "
            };
            format!("{lead}pub fn is_empty(&self) -> bool {{ self.len() == 0 }}")
        };
        edits.push(SourceEdit {
            start: insert_at,
            end: insert_at,
            replacement,
        });
    }
    edits.shrink_to_fit();
    edits
}

/// The primitive int spellings the inserted body can compare with `0`.
const INT_RETURNS: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const BAG: &str = "pub struct Bag { items: Vec<u8> }\nimpl Bag {\n    pub fn len(&self) -> usize {\n        self.items.len()\n    }\n}\n";

    #[test]
    fn seed_is_clean() {
        // The is_empty sibling is present — the firing premise is absent.
        assert!(propose_fix(
            "len_without_is_empty",
            "pub struct W { items: Vec<u8> }\nimpl W {\n    pub fn len(&self) -> usize {\n        self.items.len()\n    }\n    pub fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n}",
        )
        .is_none());
    }

    #[test]
    fn is_empty_is_inserted() {
        let out = propose_fix("len_without_is_empty", BAG).expect("matched");
        assert!(
            out.contains("pub fn is_empty(&self) -> bool"),
            "method missing: {out}"
        );
        assert!(out.contains("self.len() == 0"), "body missing: {out}");
        assert!(
            out.contains("    pub fn len(&self) -> usize"),
            "len moved: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn insertion_lands_before_the_impl_closing_brace() {
        let out = propose_fix("len_without_is_empty", BAG).expect("matched");
        // The impl's `}` stays the LAST brace of the impl block.
        let impl_pos = out.find("impl Bag").expect("impl present");
        let is_empty_pos = out[impl_pos..].find("pub fn is_empty").expect("inserted");
        let closing = out[impl_pos..]
            .rfind("}\n")
            .map_or(out.len(), |p| impl_pos + p);
        assert!(
            is_empty_pos < closing,
            "is_empty must sit inside the impl: {out}"
        );
    }

    #[test]
    fn non_pub_len_refuses() {
        let src = "pub struct W { items: Vec<u8> }\nimpl W {\n    fn len(&self) -> usize {\n        self.items.len()\n    }\n}";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn trait_definition_refuses_v1() {
        // A trait's `pub fn len` gains a REQUIRED method on insertion —
        // every downstream impl would break. The surface-break class is
        // the documented v1 refusal.
        let src = "pub trait T {\n    fn len(&self) -> usize;\n}";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn trait_impl_refuses_v1() {
        let src = "pub trait T { fn len(&self) -> usize; }\npub struct W { items: Vec<u8> }\nimpl T for W {\n    fn len(&self) -> usize {\n        self.items.len()\n    }\n}";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn unsafe_len_refuses() {
        // The inserted body calls len() outside an unsafe block.
        let src = "pub struct W { items: Vec<u8> }\nimpl W { pub unsafe fn len(&self) -> usize { self.items.len() } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn generic_len_refuses() {
        // `self.len()` cannot infer the method's own generic parameter.
        let src = "pub struct W { items: Vec<u8> }\nimpl W { pub fn len<T>(&self) -> usize { self.items.len() } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn non_int_return_refuses() {
        // `self.len() == 0` needs an int (PartialEq<usize>).
        let src = "pub struct W { items: Vec<u8> }\nimpl W { pub fn len(&self) -> String { String::new() } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn by_value_receiver_refuses() {
        // `is_empty(&self)` cannot call a by-value `len` (E0507).
        let src = "pub struct W;\nimpl W { pub fn len(self) -> usize { 0 } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn mut_receiver_refuses() {
        // `is_empty(&self)` cannot call a `&mut self` `len` (E0596).
        let src = "pub struct W { n: usize }\nimpl W { pub fn len(&mut self) -> usize { self.n } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn argful_len_refuses() {
        let src = "pub struct W { items: Vec<u8> }\nimpl W { pub fn len(&self, cap: usize) -> usize { self.items.len() + cap } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn is_empty_in_another_impl_refuses() {
        // A second inherent impl of one type is E0592-unreachable, but the
        // guard is name-scoped so a same-file sibling impl still refuses.
        let src = "pub struct W { items: Vec<u8> }\nimpl W { pub fn len(&self) -> usize { self.items.len() } }\nimpl W { pub fn is_empty(&self) -> bool { true } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn is_empty_in_a_trait_impl_refuses() {
        // The inherent method would shadow the trait method at every call
        // site — the conservative refusal (the probe's with-empty silence
        // covers the same-impl spelling).
        let src = "pub trait E { fn is_empty(&self) -> bool; }\npub struct W { items: Vec<u8> }\nimpl E for W { fn is_empty(&self) -> bool { true } }\nimpl W { pub fn len(&self) -> usize { self.items.len() } }";
        assert!(propose_fix("len_without_is_empty", src).is_none());
    }

    #[test]
    fn sibling_type_with_is_empty_does_not_block() {
        // The refusal is NAME-scoped: another type's is_empty is not ours.
        let src = "pub struct V { n: usize }\nimpl V { pub fn is_empty(&self) -> bool { self.n == 0 } }\npub struct W { items: Vec<u8> }\nimpl W { pub fn len(&self) -> usize { self.items.len() } }";
        let out = propose_fix("len_without_is_empty", src).expect("matched");
        assert!(out.contains("impl W {"), "{out}");
        assert!(out.contains("pub fn is_empty(&self) -> bool"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn single_line_impl_gets_the_single_line_form() {
        let src = "pub struct W { items: Vec<u8> }\nimpl W { pub fn len(&self) -> usize { self.items.len() } }";
        let out = propose_fix("len_without_is_empty", src).expect("matched");
        assert!(
            out.contains("} pub fn is_empty(&self) -> bool { self.len() == 0 }}"),
            "single-line form missing: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn const_len_is_admitted() {
        let src = "pub struct W { items: Vec<u8> }\nimpl W { pub const fn len(&self) -> usize { self.items.len() } }";
        let out = propose_fix("len_without_is_empty", src).expect("matched");
        assert!(out.contains("pub fn is_empty(&self) -> bool"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let first = propose_fix("len_without_is_empty", BAG).expect("matched");
        assert!(propose_fix("len_without_is_empty", &first).is_none());
    }

    #[test]
    fn two_candidates_each_get_their_method() {
        let src = "pub struct A { items: Vec<u8> }\nimpl A { pub fn len(&self) -> usize { self.items.len() } }\npub struct B { n: usize }\nimpl B { pub fn len(&self) -> usize { self.n } }";
        let out = propose_fix("len_without_is_empty", src).expect("matched");
        assert_eq!(
            out.matches("pub fn is_empty(&self) -> bool").count(),
            2,
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}
