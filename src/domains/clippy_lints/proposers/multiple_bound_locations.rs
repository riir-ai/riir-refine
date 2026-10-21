//! `clippy::multiple_bound_locations` — a generic param carrying bounds
//! inline (`<T: Default>`) AND in the where clause (`where T: Default`)
//! consolidates into ONE location (Batch 185 / Plan 180 — the
//! coverage-report pack XVII; the Plan-169 staged fresh row. Probe:
//! `scripts/plan180_pack17_probe.py`).
//!
//! Upstream emits NO suggested replacement — both duplicate locations are
//! primary spans (`suggested_replacement: None`, probe-measured) — so the
//! fix is SYNTHESIZED. The sound deletion is subset-driven: only a bound
//! that exists in BOTH locations may disappear from one of them; deleting
//! a bound that lives only in one location would drop a real trait bound
//! (a semantics change). The lint fires on the docs' MIXED case
//! (`F: Debug` + `where F: Sized`) too, and the only sound fix there is a
//! MERGE across locations — a two-location insertion this rule refuses
//! v1 (the documented narrower-than-clippy divergence).
//!
//! ## The two fix directions (both probe-compiled + probe-silent)
//!
//! - **inline deletion** — the inline set ⊆ the where set: the `: bounds`
//!   segment after the ident is deleted and the where arm carries the
//!   full set. `T: Default + Clone` + `where T: Default + Clone` →
//!   `<T>` + the clause.
//! - **where-arm deletion** — the where set ⊆ the inline set: the whole
//!   predicate (plus its trailing comma, or the whole `where` clause when
//!   it was the only predicate) disappears and the inline bounds stand.
//! - Disjoint / partially-overlapping sets refuse (the merge case).
//!
//! ## Guards (the bounded fix-space)
//!
//! - **Fns only** — the probe measured upstream SILENT on trait impls and
//!   struct/impl definitions; this proposer walks fn signatures (item /
//!   trait / impl fns), which is upstream parity, and every other item
//!   kind refuses by construction.
//! - Type params only; where predicates of kind `Type` only; the
//!   predicate subject must be EXACTLY one plain ident (`T`) — generic
//!   paths, references and qualified types refuse.
//! - Whole-set subset logic on the bound TEXTS (each `TraitBound`'s exact
//!   source text, trimmed). A partial inline set (some but not all of the
//!   where arm's bounds inline) refuses — the fix would need a partial
//!   segment deletion the one-span edit cannot express.
//! - Fixpoint: the output carries no param in both locations, and the
//!   `--fix` compile gate reverts any site where consolidation changed a
//!   bound's meaning (identical text on both sides makes that a no-op by
//!   construction).

use std::collections::BTreeSet;

use syn::visit::Visit;
use syn::{File, Generics, ImplItemFn, ItemFn, PredicateType, TraitItemFn, WherePredicate};

use super::helpers::byte_range;
use super::SourceEdit;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

/// Exact source text of one spanned node (trimmed).
fn text_at<'s>(source: &'s str, node: &impl syn::spanned::Spanned) -> &'s str {
    let r = byte_range(node);
    source[r.start..r.end].trim()
}

/// The bounded type's text when it is EXACTLY one plain ident (`T`) — the
/// only where-predicate subject this rule keys on. Generic paths,
/// references, and qualified types refuse.
fn plain_ident_text<'s>(source: &'s str, ty: &syn::Type) -> Option<&'s str> {
    let syn::Type::Path(tp) = ty else { return None };
    if tp.qself.is_some() || tp.path.leading_colon.is_some() || tp.path.segments.len() != 1 {
        return None;
    }
    if !tp.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(text_at(source, &tp.path))
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        syn::visit::visit_item_fn(self, f);
        self.try_rewrite(&f.sig.generics);
    }

    fn visit_trait_item_fn(&mut self, f: &'ast TraitItemFn) {
        syn::visit::visit_trait_item_fn(self, f);
        self.try_rewrite(&f.sig.generics);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        syn::visit::visit_impl_item_fn(self, f);
        self.try_rewrite(&f.sig.generics);
    }
}

impl Collector<'_> {
    fn try_rewrite(&mut self, generics: &Generics) {
        let Some(clause) = generics.where_clause.as_ref() else {
            return;
        };
        // Where predicates of kind Type, keyed by the subject's ident text.
        let mut where_preds: Vec<(&str, &PredicateType)> = Vec::new();
        for pred in &clause.predicates {
            let WherePredicate::Type(pt) = pred else {
                continue;
            };
            if let Some(text) = plain_ident_text(self.source, &pt.bounded_ty) {
                where_preds.push((text, pt));
            }
        }
        for param in &generics.params {
            let syn::GenericParam::Type(tp) = param else {
                continue;
            };
            if tp.bounds.is_empty() {
                continue; // inline-only never fires; where-only is the
                          // single-location shape upstream is silent on
            }
            let ident_text = tp.ident.to_string();
            let Some(pt) = where_preds
                .iter()
                .find(|(t, _)| *t == ident_text)
                .map(|(_, pt)| *pt)
            else {
                continue; // inline-only — upstream silent (probe)
            };
            let inline_set: BTreeSet<&str> =
                tp.bounds.iter().map(|b| text_at(self.source, b)).collect();
            let where_set: BTreeSet<&str> =
                pt.bounds.iter().map(|b| text_at(self.source, b)).collect();
            if inline_set.is_empty() || where_set.is_empty() {
                continue;
            }
            if inline_set.is_subset(&where_set) {
                // Drop the inline `: bounds` segment; the clause carries
                // the full set (probe: mbl_inline_dropped compiles+silent).
                let colon = tp.colon_token.as_ref().expect("bounds imply colon");
                let start = byte_range(colon).start;
                let end = byte_range(tp.bounds.last().expect("nonempty")).end;
                self.edits.push(SourceEdit {
                    start,
                    end,
                    replacement: String::new(),
                });
            } else if where_set.is_subset(&inline_set) {
                // Drop the whole where predicate; the inline bounds stand
                // (probe: mbl_where_dropped compiles+silent).
                self.edits.push(self.where_predicate_deletion(clause, pt));
            }
            // Disjoint / partial: the merge case — refused v1 (documented).
        }
    }

    /// Deletion of one type predicate: its span plus the trailing comma,
    /// or the preceding comma + the predicate (last-position), or the
    /// whole `where` clause when it was the only predicate.
    fn where_predicate_deletion(
        &self,
        clause: &syn::WhereClause,
        pt: &PredicateType,
    ) -> SourceEdit {
        if clause.predicates.len() == 1 {
            // The clause dissolves: delete `where` and everything after it
            // (the body braces survive outside the span).
            let start = byte_range(&clause.where_token).start;
            let end = byte_range(clause).end;
            return SourceEdit {
                start,
                end,
                replacement: String::new(),
            };
        }
        let pr = byte_range(pt);
        let bytes = self.source.as_bytes();
        let mut start = pr.start;
        let mut end = pr.end;
        // Leading whitespace (newline + indent) before the predicate is
        // part of the arm's line scaffolding — swallowed in BOTH branches.
        let mut line_start = start;
        while line_start > 0 && (bytes[line_start - 1] == b' ' || bytes[line_start - 1] == b'\t') {
            line_start -= 1;
        }
        if line_start > 0 && bytes[line_start - 1] == b'\n' {
            start = line_start - 1;
        } else {
            start = line_start;
        }
        // Comma AFTER the predicate? It dies with the arm (a clean
        // mid-list splice).
        let mut at = end;
        while at < bytes.len() && (bytes[at] == b' ' || bytes[at] == b'\n' || bytes[at] == b'\r') {
            at += 1;
        }
        if at < bytes.len() && bytes[at] == b',' {
            end = at + 1;
        } else {
            // Last predicate: swallow the comma BEFORE it instead (the
            // line scaffolding is already inside the span).
            let mut back = end;
            while back > 0
                && (bytes[back - 1] == b' ' || bytes[back - 1] == b'\n' || bytes[back - 1] == b'\r')
            {
                back -= 1;
            }
            if back > 0 && bytes[back - 1] == b',' {
                end = back;
            }
        }
        SourceEdit {
            start,
            end,
            replacement: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        // Inline-only is upstream-silent (probe mbl_inline_only).
        assert!(propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default>(t: T) -> T { t }",
        )
        .is_none());
    }

    #[test]
    fn where_only_is_clean() {
        // Where-only is upstream-silent (probe mbl_where_only).
        assert!(propose_fix(
            "multiple_bound_locations",
            "pub fn f<T>(t: T) -> T where T: Default { t }",
        )
        .is_none());
    }

    #[test]
    fn inline_drops_when_where_superset() {
        // Inline ⊆ where: the inline segment deletes (the docs' own
        // "Use instead" direction).
        let out = propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default>(t: T) -> T\nwhere\n    T: Default + Clone,\n{\n    t.clone()\n}",
        )
        .expect("matched");
        assert!(out.contains("pub fn f<T>(t: T)"), "{out}");
        assert!(out.contains("T: Default + Clone"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn equal_sets_consolidate_into_where() {
        // The probe's canonical firing shape: equal sets are subsets both
        // ways; the INLINE direction is checked first and wins.
        let out = propose_fix(
            "multiple_bound_locations",
            "pub fn m<T: Default + Clone>(t: T) -> T\nwhere\n    T: Default + Clone,\n{\n    t.clone()\n}",
        )
        .expect("matched");
        assert!(out.contains("pub fn m<T>(t: T)"), "{out}");
        assert!(out.contains("where"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn where_arm_drops_with_strict_inline_superset() {
        // Where ⊆ inline (strict): the arm deletes, the inline stands.
        let out = propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default + Clone>(t: T) -> T\nwhere\n    T: Default,\n{\n    t.clone()\n}",
        )
        .expect("matched");
        assert!(out.contains("pub fn f<T: Default + Clone>(t: T)"), "{out}");
        assert!(!out.contains("where"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn single_where_predicate_dissolves_the_clause() {
        // Where ⊆ inline and the clause holds ONLY this predicate: the
        // whole `where ...` goes (no dangling keyword).
        let out = propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default + Clone>(t: T) -> T\nwhere\n    T: Default\n{\n    t.clone()\n}",
        )
        .expect("matched");
        assert!(!out.contains("where"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mid_list_where_arm_stays_intact() {
        // Inline {Default} vs where arms {Default},{PartialEq<T>}: the
        // FIRST matching arm is an equal set → the inline direction wins;
        // the clause and BOTH arms survive untouched.
        let out = propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default>(t: T) -> T\nwhere\n    T: Default,\n    T: PartialEq<T>,\n{\n    t == t\n}",
        )
        .expect("matched");
        assert!(out.contains("pub fn f<T>(t: T)"), "{out}");
        assert!(out.contains("where"), "{out}");
        assert!(out.contains("T: PartialEq<T>"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn strict_where_subset_drops_the_arm_mid_list() {
        // Inline {Default, Clone} vs where arms {Default},{Send}: the
        // first matching arm is a strict subset → the where-arm deletion
        // fires WITH its trailing comma, mid-list, cleanly.
        let out = propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default + Clone + Send>(t: T) -> T\nwhere\n    T: Default,\n    T: Send,\n{\n    t.clone()\n}",
        )
        .expect("matched");
        assert!(
            out.contains("pub fn f<T: Default + Clone + Send>(t: T)"),
            "{out}"
        );
        assert!(out.contains("where"), "{out}");
        assert!(out.contains("T: Send"), "{out}");
        assert!(!out.contains("T: Default,"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn disjoint_sets_refuse_the_merge_case() {
        // The docs' mixed firing case: Debug inline, Sized where — the
        // sound fix is a merge (two-location insertion), refused v1
        // (documented narrower-than-clippy divergence).
        assert!(propose_fix(
            "multiple_bound_locations",
            "pub fn ty<F: std::fmt::Debug>(a: F)\nwhere\n    F: Sized,\n{}",
        )
        .is_none());
    }

    #[test]
    fn disjoint_other_direction_refuses() {
        assert!(propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default>(t: T) -> T\nwhere\n    T: Clone,\n{\n    t\n}",
        )
        .is_none());
    }

    #[test]
    fn trait_impl_refuses() {
        // Upstream-silent surface (probe mbl_trait_impl) — parity, not
        // divergence.
        assert!(propose_fix(
            "multiple_bound_locations",
            "pub trait Tr { fn m(&self) -> u8; }\nimpl<T: Default> Tr for T\nwhere\n    T: Default,\n{\n    fn m(&self) -> u8 { 0 }\n}",
        )
        .is_none());
    }

    #[test]
    fn struct_definition_refuses() {
        // Upstream-silent surface (probe mbl_struct).
        assert!(propose_fix(
            "multiple_bound_locations",
            "pub struct S<T: Default>\nwhere\n    T: Default,\n{\n    t: T,\n}",
        )
        .is_none());
    }

    #[test]
    fn non_ident_where_subject_refuses() {
        // A reference subject (`&'static T`) is not the plain-ident
        // subject this rule keys on.
        assert!(propose_fix(
            "multiple_bound_locations",
            "pub fn f<T: Default>(t: T) -> T\nwhere\n    &'static T: Default,\n{\n    t\n}",
        )
        .is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let src = "pub fn f<T: Default>(t: T) -> T\nwhere\n    T: Default,\n{\n    t\n}";
        let first = propose_fix("multiple_bound_locations", src).expect("matched");
        assert!(propose_fix("multiple_bound_locations", &first).is_none());
    }

    #[test]
    fn nested_module_fns_fire() {
        let out = propose_fix(
            "multiple_bound_locations",
            "mod m {\n    pub fn f<T: Default>(t: T) -> T\n    where\n        T: Default,\n    {\n        t\n    }\n}",
        )
        .expect("matched");
        assert!(out.contains("pub fn f<T>(t: T)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
