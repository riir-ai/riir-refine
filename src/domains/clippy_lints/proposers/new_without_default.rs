//! `clippy::new_without_default` — `impl W { pub fn new() -> Self { .. } }`
//! with no `Default` → INSERT `impl Default for W { fn default() -> Self {
//! Self::new() } }` (Batch 168 / Plan 159 — the coverage-report re-arm pack
//! XII; the family fixture verified to fire under the plain oracle,
//! 2026-09-21, `scripts/plan159_pack12_probe.py`).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! The edit is purely ADDITIVE: no existing byte moves, the inserted impl
//! is byte-for-byte clippy's own "try adding this" suggestion shape. Adding
//! a `Default` impl cannot change the meaning of code that already
//! compiled — every use of `Default` was previously a compile error
//! (unsatisfied bound / unresolved path), so the impl only turns broken
//! code into working code. Inherent methods shadow trait methods for both
//! `.default()` calls and `W::default()` paths, so the one ambiguity shape
//! (an inherent `default` fn) is refused up front, not hoped away.
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - Inherent impls only (`trait_` none). The self type must be a plain
//!   ident (`W`) — generic impls (`impl<T> X<T>`) refuse in v1.
//! - The `new` shape: zero inputs, no generics, not `unsafe`, returns
//!   exactly `Self`, and is `pub` (clippy's firing premise is effective
//!   exportation; plain `pub` is the conservative reading). `const fn new`
//!   is admitted — a const fn is callable from `default()`.
//! - No `impl Default for W` anywhere in the FILE, no `#[derive(Default)]`
//!   on any same-named type, and no inherent `default` fn in any `W` impl.
//!   A Default impl living in ANOTHER module/file is invisible to a
//!   parse-only proposer — the compile gate catches the conflicting-impls
//!   revert (the documented cross-file divergence, E0119-class, never
//!   silent: `--verify` reverts).
//! - The inserted block carries the inherent impl's own line indentation;
//!   multiple candidates in one file each get their own block (distinct
//!   types — duplicate `new` on one type is E0592, unreachable).

use std::collections::BTreeSet;

use quote::ToTokens;
use syn::visit::Visit;
use syn::{File, ImplItem, ItemImpl, ReturnType, Type, Visibility};

use super::helpers::byte_range;
use super::SourceEdit;

/// One admissible `new`-bearing inherent impl: the type to name in the
/// inserted `impl Default for …` plus where/how to insert it.
struct Candidate {
    type_name: String,
    impl_end: usize,
    indent: String,
}

#[derive(Default)]
struct Scan<'a> {
    source: &'a str,
    new_impls: Vec<Candidate>,
    default_impls: BTreeSet<String>,
    inherent_default_fns: BTreeSet<String>,
    derive_defaults: BTreeSet<String>,
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

/// Any `Ident` token exactly `Default` inside a `#[derive(...)]` list —
/// recursive, because the list lives inside the bracket GROUP of the
/// attribute's token stream (a top-level iteration sees only `#` + the
/// group; measured by the debug probe before this fix shipped).
fn tokens_list_default(ts: proc_macro2::TokenStream) -> bool {
    ts.into_iter().any(|tt| match tt {
        proc_macro2::TokenTree::Ident(i) => i == "Default",
        proc_macro2::TokenTree::Group(g) => tokens_list_default(g.stream()),
        _ => false,
    })
}

fn derive_lists_default(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("derive") && tokens_list_default(a.to_token_stream()))
}

/// The zero-arg `pub fn new() -> Self` shape clippy's suggestion composes.
fn is_zero_arg_new(f: &syn::ImplItemFn) -> bool {
    if !matches!(f.vis, Visibility::Public(_)) || !f.sig.inputs.is_empty() {
        return false;
    }
    if !f.sig.generics.params.is_empty() || f.sig.unsafety.is_some() {
        return false;
    }
    matches!(
        &f.sig.output,
        ReturnType::Type(_, ty)
            if matches!(**ty, Type::Path(ref p)
                if p.qself.is_none()
                    && p.path.segments.len() == 1
                    && p.path.segments[0].ident == "Self"
                    && p.path.segments[0].arguments.is_none())
    )
}

impl<'ast> Visit<'ast> for Scan<'ast> {
    fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
        if derive_lists_default(&s.attrs) {
            self.derive_defaults.insert(s.ident.to_string());
        }
        syn::visit::visit_item_struct(self, s);
    }

    fn visit_item_enum(&mut self, e: &'ast syn::ItemEnum) {
        if derive_lists_default(&e.attrs) {
            self.derive_defaults.insert(e.ident.to_string());
        }
        syn::visit::visit_item_enum(self, e);
    }

    fn visit_item_union(&mut self, u: &'ast syn::ItemUnion) {
        if derive_lists_default(&u.attrs) {
            self.derive_defaults.insert(u.ident.to_string());
        }
        syn::visit::visit_item_union(self, u);
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let name = simple_type_name(&i.self_ty);
        if let Some((_, path, _)) = &i.trait_ {
            if path.segments.last().is_some_and(|s| s.ident == "Default") {
                if let Some(n) = &name {
                    self.default_impls.insert(n.clone());
                }
            }
        } else if let Some(n) = name {
            let range = byte_range(i);
            let mut new_found = false;
            for item in &i.items {
                if let ImplItem::Fn(f) = item {
                    if f.sig.ident == "default" {
                        self.inherent_default_fns.insert(n.clone());
                    }
                    if f.sig.ident == "new" && !new_found && is_zero_arg_new(f) {
                        new_found = true;
                    }
                }
            }
            if new_found {
                // Indent = the whitespace between the impl's line start and
                // the `impl` keyword; a non-whitespace prefix reads as "".
                let line_start = self.source[..range.start].rfind('\n').map_or(0, |i| i + 1);
                let prefix = &self.source[line_start..range.start];
                let indent = if !prefix.is_empty() && prefix.chars().all(char::is_whitespace) {
                    prefix.to_string()
                } else {
                    String::new()
                };
                self.new_impls.push(Candidate {
                    type_name: n,
                    impl_end: range.end,
                    indent,
                });
            }
        }
        syn::visit::visit_item_impl(self, i);
    }
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = Scan {
        source,
        new_impls: Vec::new(),
        default_impls: BTreeSet::new(),
        inherent_default_fns: BTreeSet::new(),
        derive_defaults: BTreeSet::new(),
    };
    scan.visit_file(file);
    let mut edits = Vec::new();
    for cand in scan.new_impls {
        let Candidate {
            type_name,
            impl_end,
            indent,
        } = cand;
        if scan.default_impls.contains(&type_name)
            || scan.inherent_default_fns.contains(&type_name)
            || scan.derive_defaults.contains(&type_name)
        {
            continue;
        }
        let ind = &indent;
        let replacement = format!(
            "\n\n{ind}impl Default for {type_name} {{\n{ind}    fn default() -> Self {{\n{ind}        Self::new()\n{ind}    }}\n{ind}}}"
        );
        edits.push(SourceEdit {
            start: impl_end,
            end: impl_end,
            replacement,
        });
    }
    edits.shrink_to_fit();
    edits
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const WIDG: &str =
        "pub struct Widg { pub x: u8 }\nimpl Widg { pub fn new() -> Self { Widg { x: 1 } } }";

    #[test]
    fn seed_is_clean() {
        // Default already implemented — the firing premise is absent.
        assert!(propose_fix(
            "new_without_default",
            "pub struct W { pub x: u8 }\nimpl W { pub fn new() -> Self { W { x: 1 } } }\nimpl Default for W { fn default() -> Self { W { x: 1 } } }",
        )
        .is_none());
    }

    #[test]
    fn default_impl_is_added() {
        let out = propose_fix("new_without_default", WIDG).expect("matched");
        assert!(out.contains("impl Default for Widg"), "impl missing: {out}");
        assert!(out.contains("Self::new()"), "delegate missing: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn derive_default_refuses() {
        let src = "#[derive(Debug, Default)]\npub struct W { pub x: u8 }\nimpl W { pub fn new() -> Self { W { x: 1 } } }";
        assert!(propose_fix("new_without_default", src).is_none());
    }

    #[test]
    fn derive_of_other_trait_is_not_default() {
        // `DefaultHasher`-style ident text must not read as `Default` —
        // the derive check matches whole idents, not substrings.
        let src = "#[derive(Debug)]\npub struct W { pub x: u8 }\nimpl W { pub fn new() -> Self { W { x: 1 } } }";
        let out = propose_fix("new_without_default", src).expect("matched");
        assert!(out.contains("impl Default for W"));
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn inherent_default_fn_refuses() {
        // An inherent `default` would shadow the trait method — refused
        // up front rather than hoped away.
        let src = "pub struct W { pub x: u8 }\nimpl W { pub fn new() -> Self { W { x: 1 } } pub fn default() -> Self { W { x: 0 } } }";
        assert!(propose_fix("new_without_default", src).is_none());
    }

    #[test]
    fn non_pub_new_refuses() {
        let src = "pub struct W { pub x: u8 }\nimpl W { fn new() -> Self { W { x: 1 } } }";
        assert!(propose_fix("new_without_default", src).is_none());
    }

    #[test]
    fn explicit_return_type_refuses() {
        // `-> W` (not `-> Self`) is outside the lint's own premise.
        let src = "pub struct W { pub x: u8 }\nimpl W { pub fn new() -> W { W { x: 1 } } }";
        assert!(propose_fix("new_without_default", src).is_none());
    }

    #[test]
    fn generic_impl_refuses() {
        let src =
            "pub struct W<T> { pub x: T }\nimpl<T> W<T> { pub fn new(x: T) -> Self { W { x } } }";
        assert!(propose_fix("new_without_default", src).is_none());
    }

    #[test]
    fn argful_new_refuses() {
        let src = "pub struct W { pub x: u8 }\nimpl W { pub fn new(x: u8) -> Self { W { x } } }";
        assert!(propose_fix("new_without_default", src).is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let first = propose_fix("new_without_default", WIDG).expect("matched");
        assert!(propose_fix("new_without_default", &first).is_none());
    }

    #[test]
    fn two_candidates_each_get_their_block() {
        let src = "pub struct A { pub x: u8 }\nimpl A { pub fn new() -> Self { A { x: 1 } } }\npub struct B { pub y: u8 }\nimpl B { pub fn new() -> Self { B { y: 1 } } }";
        let out = propose_fix("new_without_default", src).expect("matched");
        assert!(out.contains("impl Default for A"));
        assert!(out.contains("impl Default for B"));
        syn::parse_file(&out).expect("re-parses");
    }
}
