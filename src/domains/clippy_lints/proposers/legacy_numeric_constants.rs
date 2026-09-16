//! `clippy::legacy_numeric_constants` — `std::u32::MAX` → `u32::MAX`
//! (Batch 150; Bench 094 default-level evidence: 23 firings across 5
//! crates in the top-10k sample — the strongest multi-crate default-level
//! row in the staged pack).
//!
//! ## Probe matrix (real clippy 0.1.98)
//!
//! FIRES — `std::{T}::{CONST}` expression paths (`MAX`, `MIN` integers;
//! `EPSILON`, `INFINITY`, `NAN`, ... floats), the `::std::`-leading form,
//! and the import form `use std::u32::MAX as LEGACY_MAX;` ("importing a
//! legacy numeric constant"). SILENT — the already-modern `u32::MAX`
//! association-constant form.
//!
//! Clippy's own suggestion is `MaybeIncorrect` (the shadow hazard below),
//! so the oracle lane does not consume it; this proposer generates the
//! rewrite itself with a parse-only SHADOW GUARD standing in for the
//! typeck clippy uses.
//!
//! ⚠ The one semantic hazard: path resolution prefers an in-scope ITEM
//! named like a primitive (`mod u32 {}`, `struct u32;`, `use
//! weird::u32;`, `use foo::bar as u32;`) — the rewrite would then
//! resolve to THAT item and compile with a different meaning. Guard:
//! refuse the whole file when any item or use-alias/last-segment is
//! named like one of the 14 primitives (except `use std::…` re-openings,
//! which preserve semantics by construction). Conservative refusals only
//! ever shrink the fired set; the compile gate cannot arbitrate a
//! meaning-preserving-looking wrong rewrite, so the guard is the
//! defense. Divergences (documented): macro bodies are invisible
//! (syn does not descend), grouped imports
//! (`use std::{u32::MAX, f64::EPSILON};`) are refused, and only the flat
//! `use std::{T}::{CONST} [as X];` tree shape is rewritten.

use super::{src, SourceEdit};
use proc_macro2::TokenTree;
use syn::spanned::Spanned;
use quote::ToTokens;
use syn::visit::Visit;
use syn::{ExprPath, File, ItemUse, Path, UseTree};

/// The 14 primitive numerics whose legacy modules ship constants.
const PRIMS: &[&str] = &[
    "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "i128", "u128", "isize", "usize", "f32",
    "f64",
];

/// The legacy constants that HAVE association-constant counterparts (the
/// rewrite target must exist — `std::f64::MAX` has no `f64::MAX` twin and
/// is not rewritten).
const INT_CONSTS: &[&str] = &["MAX", "MIN"];
const FLOAT_CONSTS: &[&str] = &["EPSILON", "INFINITY", "NEG_INFINITY", "NAN", "MIN_POSITIVE"];

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    if shadows_primitive(file) {
        return v.edits; // conservatively empty — the guard refused the file
    }
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_path(&mut self, e: &'ast ExprPath) {
        syn::visit::visit_expr_path(self, e);
        let Some((t, c)) = legacy_shape(&e.path) else {
            return;
        };
        self.edits.push(SourceEdit {
            start: e.span().byte_range().start,
            end: e.span().byte_range().end,
            replacement: format!("{t}::{c}"),
        });
    }

    /// The import form (`use std::u32::MAX;` / `… as X;`) — flat 3-segment
    /// trees only (grouped trees are refused, see the module doc). The
    /// edit deletes the tree's `std::` prefix; the tail (`u32::MAX as X`)
    /// is already exactly the target text.
    fn visit_item_use(&mut self, u: &'ast ItemUse) {
        syn::visit::visit_item_use(self, u);
        let UseTree::Path(p0) = &u.tree else {
            return;
        };
        if p0.ident != "std" {
            return;
        }
        let UseTree::Path(p1) = &*p0.tree else {
            return;
        };
        let t = p1.ident.to_string();
        if !PRIMS.contains(&t.as_str()) {
            return;
        }
        let leaf_ok = match &*p1.tree {
            UseTree::Name(n) => is_legacy_const(&t, &n.ident),
            UseTree::Rename(r) => is_legacy_const(&t, &r.ident),
            _ => false, // grouped/glob leaf — refuse
        };
        if !leaf_ok {
            return;
        }
        let Some(tree_text) = src(self.source, &u.tree) else {
            return;
        };
        if !tree_text.starts_with("std::") {
            return;
        }
        let range = u.tree.span().byte_range();
        self.edits.push(SourceEdit {
            start: range.start,
            end: range.start + "std::".len(),
            replacement: String::new(),
        });
    }
}

/// `std::{T}::{CONST}` (with or without the leading `::`) → `Some((T, CONST))`.
fn legacy_shape(path: &Path) -> Option<(String, String)> {
    if path.segments.len() != 3 {
        return None;
    }
    let segs: Vec<&syn::Ident> = path.segments.iter().map(|s| &s.ident).collect();
    if segs[0] != "std" {
        return None;
    }
    let t = segs[1].to_string();
    let c = segs[2].to_string();
    if !PRIMS.contains(&t.as_str()) || !is_legacy_const(&t, segs[2]) {
        return None;
    }
    Some((t, c))
}

/// Whether `T::CONST` has an association-constant counterpart (the
/// rewrite target must exist — `std::f64::MAX` has no `f64::MAX` twin).
fn is_legacy_const(t: &str, c: &syn::Ident) -> bool {
    let c = c.to_string();
    match t {
        "f32" | "f64" => FLOAT_CONSTS.contains(&c.as_str()),
        _ => INT_CONSTS.contains(&c.as_str()),
    }
}

/// The shadow guard: any ITEM (mod/struct/enum/fn/type/const/static/
/// trait/union) or any use-tree ALIAS or non-std LAST SEGMENT named like
/// a primitive refuses the whole file (see the module doc for why — the
/// rewrite is meaning-changing under shadowing and compiles, so the
/// compile gate cannot arbitrate it).
fn shadows_primitive(file: &File) -> bool {
    struct Shadow {
        hit: bool,
    }
    impl<'ast> Visit<'ast> for Shadow {
        fn visit_item(&mut self, i: &'ast syn::Item) {
            let name = match i {
                syn::Item::Mod(x) => Some(&x.ident),
                syn::Item::Struct(x) => Some(&x.ident),
                syn::Item::Enum(x) => Some(&x.ident),
                syn::Item::Union(x) => Some(&x.ident),
                syn::Item::Trait(x) => Some(&x.ident),
                syn::Item::Type(x) => Some(&x.ident),
                syn::Item::Fn(x) => Some(&x.sig.ident),
                syn::Item::Const(x) => Some(&x.ident),
                syn::Item::Static(x) => Some(&x.ident),
                _ => None,
            };
            if name.is_some_and(|n| PRIMS.contains(&n.to_string().as_str())) {
                self.hit = true;
            }
            syn::visit::visit_item(self, i);
        }

        fn visit_item_use(&mut self, u: &'ast ItemUse) {
            // Any leaf/rename ident named like a primitive refuses, EXCEPT
            // a `use std::…` re-opening (same module — semantics preserved).
            if !tree_is_std_reopening(&u.tree) {
                for tt in u.tree.to_token_stream() {
                    if let TokenTree::Ident(id) = tt {
                        if PRIMS.contains(&id.to_string().as_str()) {
                            self.hit = true;
                        }
                    }
                }
            }
            syn::visit::visit_item_use(self, u);
        }
    }

    fn tree_is_std_reopening(tree: &UseTree) -> bool {
        let mut cur = tree;
        loop {
            match cur {
                UseTree::Path(p) => {
                    if p.ident == "std" {
                        return true;
                    }
                    cur = &p.tree;
                }
                _ => return false,
            }
        }
    }

    let mut s = Shadow { hit: false };
    s.visit_file(file);
    s.hit
}

#[cfg(test)]
mod tests {

    fn fix(code: &str) -> Option<String> {
        super::super::propose_fix("legacy_numeric_constants", code)
    }

    #[test]
    fn fires_on_every_probe_firing_shape() {
        for (buggy, fixed) in [
            ("fn f() { let a = std::u32::MAX; }", "fn f() { let a = u32::MAX; }"),
            ("fn f() { let a = std::i64::MIN; }", "fn f() { let a = i64::MIN; }"),
            ("fn f() { let a = std::f64::EPSILON; }", "fn f() { let a = f64::EPSILON; }"),
            ("fn f() { let a = std::f32::NAN; }", "fn f() { let a = f32::NAN; }"),
            ("fn f() { let a = std::usize::MAX; }", "fn f() { let a = usize::MAX; }"),
            ("fn f() { let a = std::f64::INFINITY; }", "fn f() { let a = f64::INFINITY; }"),
            ("fn f() { let a = ::std::u8::MAX; }", "fn f() { let a = u8::MAX; }"),
            (
                "fn f() { let a = std::f64::MIN_POSITIVE; }",
                "fn f() { let a = f64::MIN_POSITIVE; }",
            ),
            (
                "use std::u32::MAX as LEGACY_MAX;\nfn f() { let a = LEGACY_MAX; }",
                "use u32::MAX as LEGACY_MAX;\nfn f() { let a = LEGACY_MAX; }",
            ),
            ("use std::i16::MIN;\nfn main() {}", "use i16::MIN;\nfn main() {}"),
        ] {
            assert_eq!(fix(buggy).as_deref(), Some(fixed), "case: {buggy}");
        }
    }

    #[test]
    fn silent_on_every_non_target_shape() {
        for code in [
            "fn f() { let a = u32::MAX; }", // already modern
            "fn f() { let a = std::u32::MAX_VALUE; }", // no such legacy const
            "fn f() { let a = std::f64::MAX; }", // no assoc-const twin — refuse
            "fn f() { let a = core::u32::MAX; }", // not the std path
            "fn f() { let a = std::string::String::MAX_WRAP; }", // not a prim segment
        ] {
            assert_eq!(fix(code), None, "must stay silent: {code}");
        }
    }

    #[test]
    fn shadow_guard_refuses_the_whole_file() {
        for code in [
            "mod u32 { pub const MAX: u32 = 1; }\nfn f() { let a = std::u32::MAX; }",
            "struct u32;\nfn f() { let a = std::u32::MAX; }",
            "fn u32() {}\nfn g() { let a = std::u32::MAX; }",
            "use weird::u32;\nfn g() { let a = std::u32::MAX; }",
            "use foo::bar as u32;\nfn g() { let a = std::u32::MAX; }",
        ] {
            assert_eq!(fix(code), None, "shadow must refuse: {code}");
        }
    }

    #[test]
    fn std_reopening_use_does_not_trigger_the_guard() {
        // `use std::u32;` re-opens the same module — semantics preserved.
        let code = "use std::u32;\nfn f() { let a = std::u32::MAX; }";
        let out = fix(code).expect("std re-opening is not a shadow");
        assert_eq!(out, "use std::u32;\nfn f() { let a = u32::MAX; }");
    }
}
