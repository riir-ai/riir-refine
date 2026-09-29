//! `clippy::mem_replace_with_default` — `std::mem::replace(dst,
//! Default::default())` → `std::mem::take(dst)` (Batch 191 / Plan 188 —
//! the coverage-report re-arm pack XVIII; the firing/silence matrix
//! verified against real clippy 0.1.98, 2026-09-30
//! `scripts/plan188_pack18_probe.py`).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `std::mem::take` is DEFINED as the expansion it replaces — the std
//! source is `pub fn take<T: Default>(dest: &mut T) -> T { replace(dest,
//! T::default()) }`. Substituting the definition's own body with the
//! function is value-identical by construction, and the input's own
//! compilation proves `T: Default` (the `Default::default()` argument
//! type-checked against `replace`'s `src: T`) — zero type-blindness (the
//! std definition-identity class of Batch 160's radix-10 `FromStr` and
//! Batch 162's `bytes().count()`).
//!
//! ## Guards (the bounded fix-space)
//!
//! - The callee is `std::mem::replace` (3 segments) or `mem::replace`
//!   (2 segments, ONLY with a plain `use std::mem;` import present AND no
//!   item/use alias named `mem` in the file — the
//!   `legacy_numeric_constants` shadow-guard class; probe-measured
//!   upstream is SILENT on a local `mem::replace`, so the guard is
//!   parity, not divergence).
//! - Exactly 2 args; the second is EXACTLY the bare `Default::default()`
//!   path call (no args, no turbofish, no attributes).
//! - `<T as Default>::default()` is SILENT upstream (probe-measured) —
//!   parity refusal. `<_>::default()` FIRES upstream but is refused v1 —
//!   the documented narrower-than-clippy divergence (rarity; the identity
//!   holds, the bounded-space call).
//! - Bare `replace(...)` (via `use std::mem::replace;`) FIRES upstream
//!   (probe-measured, `MachineApplicable`) but refuses here: a syntax-only
//!   proposer cannot distinguish the import from a local `fn replace`,
//!   and the rewrite under a local fn is BROKEN code — the
//!   SAFER-than-clippy divergence class.
//! - No attributes on the call; a comment inside the replaced span
//!   refuses (the span re-spells to `<path>take(<arg0>)`).
//!
//! The emitted form keeps the author's own path prefix (`mem::take` stays
//! `mem::take`) where clippy always suggests the full `std::mem::take`
//! spelling — both compile identically (the diverge-from-the-literal-
//! suggestion precedent, Batch 171's `cloned()` over `copied()`).

use syn::visit::Visit;
use syn::{Expr, File};

use super::helpers::byte_range;
use super::{edit, src, SourceEdit};
use crate::edit_emitter::drops_interior_comment;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let imports = MemImports::scan(file);
    let mut v = Collector { source, edits: Vec::new(), imports };
    v.visit_file(file);
    v.edits
}

/// The file-level import state for the 2-segment `mem::replace` spelling:
/// a plain `use std::mem;` must be present, and nothing may shadow the
/// name `mem` (an item, or a use alias/rename binding `mem` to a
/// non-std path). The 3-segment `std::mem::replace` spelling needs no
/// import state — the path is self-proving.
struct MemImports {
    has_std_mem_use: bool,
    mem_shadowed: bool,
}

impl MemImports {
    fn scan(file: &File) -> Self {
        let mut v = Scan { out: MemImports { has_std_mem_use: false, mem_shadowed: false } };
        v.visit_file(file);
        v.out
    }
}

/// The whole-file scanner (the `legacy_numeric_constants` whole-file-guard
/// posture): a `use std::mem;` at ANY depth proves the 2-segment spelling
/// CAN resolve to std in this file; any `mem`-named item, use binding, or
/// extern-crate alias at any depth refuses it (conservative — a
/// fn-scoped shadow we cannot attribute precisely would make the rewrite
/// resolve to the wrong `mem`).
struct Scan {
    out: MemImports,
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_item(&mut self, i: &'ast syn::Item) {
        let name = match i {
            syn::Item::Mod(x) => Some(&x.ident),
            syn::Item::Fn(x) => Some(&x.sig.ident),
            syn::Item::Struct(x) => Some(&x.ident),
            syn::Item::Enum(x) => Some(&x.ident),
            syn::Item::Union(x) => Some(&x.ident),
            syn::Item::Trait(x) => Some(&x.ident),
            syn::Item::Type(x) => Some(&x.ident),
            syn::Item::Const(x) => Some(&x.ident),
            syn::Item::Static(x) => Some(&x.ident),
            syn::Item::ExternCrate(x) => {
                x.rename.as_ref().map(|(_, id)| id).or(Some(&x.ident))
            }
            _ => None,
        };
        if name.is_some_and(|n| n == "mem") {
            self.out.mem_shadowed = true;
        }
        syn::visit::visit_item(self, i);
    }

    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        if is_std_mem_tree(&u.tree) {
            self.out.has_std_mem_use = true;
        } else if binds_name_mem(&u.tree) {
            self.out.mem_shadowed = true;
        }
        syn::visit::visit_item_use(self, u);
    }
}

/// `use std::mem;` — the exact 2-segment plain tree (no rename, no glob).
fn is_std_mem_tree(tree: &syn::UseTree) -> bool {
    let syn::UseTree::Path(p0) = tree else { return false };
    if p0.ident != "std" {
        return false;
    }
    let syn::UseTree::Name(n1) = &*p0.tree else { return false };
    n1.ident == "mem"
}

/// Whether a NON-std use tree binds the bare name `mem` (an alias target
/// or a final segment) — the rename/glob shapes a syntax-only proposer
/// cannot resolve.
fn binds_name_mem(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Name(n) => n.ident == "mem",
        syn::UseTree::Rename(r) => r.rename == "mem",
        syn::UseTree::Glob(_) => true, // a glob could bind anything — refuse
        syn::UseTree::Path(p) => binds_name_mem(&p.tree),
        syn::UseTree::Group(g) => g.items.iter().any(binds_name_mem),
    }
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    imports: MemImports,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        syn::visit::visit_expr_call(self, c);
        if let Some(replacement) = self.try_rewrite(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    /// `std::mem::replace(dst, Default::default())` / the guarded
    /// `mem::replace(..)` spelling.
    fn try_rewrite(&mut self, call: &syn::ExprCall) -> Option<String> {
        if !call.attrs.is_empty() {
            return None;
        }
        let Expr::Path(callee) = &*call.func else {
            return None;
        };
        if callee.qself.is_some() || !callee.attrs.is_empty() {
            return None;
        }
        let segs: Vec<&syn::Ident> =
            callee.path.segments.iter().map(|s| &s.ident).collect();
        let path_ok = match segs.as_slice() {
            [std_seg, mem_seg, replace_seg] => {
                *std_seg == "std" && *mem_seg == "mem" && *replace_seg == "replace"
            }
            [mem_seg, replace_seg] => {
                *mem_seg == "mem"
                    && *replace_seg == "replace"
                    && self.imports.has_std_mem_use
                    && !self.imports.mem_shadowed
            }
            _ => false,
        };
        if !path_ok {
            return None;
        }
        if call.args.len() != 2 {
            return None;
        }
        let mut args = call.args.iter();
        let (dst, default_arg) = (args.next()?, args.next()?);
        if !is_bare_default_call(default_arg) {
            return None;
        }
        let call_span = byte_range(call);
        if drops_interior_comment(self.source, call_span, &[]) {
            return None;
        }
        // `<path-prefix>take(<dst>)` — the author's own prefix re-emits
        // verbatim (only the final `replace` segment swaps for `take`).
        let callee_src = src(self.source, callee)?;
        let prefix_len = callee_src.len().checked_sub("replace".len())?;
        let dst_src = src(self.source, dst)?;
        Some(format!("{}take({dst_src})", &callee_src[..prefix_len]))
    }
}

/// Exactly the bare `Default::default()` path call — no args, no
/// turbofish, no attributes, no qualified-self path.
fn is_bare_default_call(e: &Expr) -> bool {
    let Expr::Call(c) = e else { return false };
    if !c.args.is_empty() || !c.attrs.is_empty() {
        return false;
    }
    let Expr::Path(p) = &*c.func else { return false };
    if p.qself.is_some() || !p.attrs.is_empty() {
        return false;
    }
    let segs: Vec<&syn::Ident> = p.path.segments.iter().map(|s| &s.ident).collect();
    segs.len() == 2 && *segs[0] == "Default" && *segs[1] == "default"
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "mem_replace_with_default";

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::take(dst)); }",
        )
        .is_none());
    }

    #[test]
    fn std_path_fires() {
        let out = propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, Default::default())); }",
        )
        .expect("matched");
        assert!(out.contains("std::mem::take(dst)"), "not rewritten: {out}");
        assert!(!out.contains("replace"), "call not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn imported_spelling_fires_and_keeps_prefix() {
        let out = propose_fix(
            RULE,
            "fn t(dst: &mut String, sink: &mut Vec<String>) { use std::mem; sink.push(Vec::new()); sink.push(mem::replace(dst, Default::default())); }",
        )
        .expect("matched");
        assert!(out.contains("mem::take(dst)"), "prefix lost: {out}");
        assert!(out.contains("use std::mem;"), "import dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_larger_expr_fires() {
        let out = propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u8>) -> usize { std::mem::replace(dst, Default::default()).len() }",
        )
        .expect("matched");
        assert!(out.contains("std::mem::take(dst).len()"), "not in place: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn turbofish_default_refuses() {
        // SILENT upstream (probe-measured) — parity refusal.
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut u64, sink: &mut Vec<u64>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, <u64 as Default>::default())); }",
        )
        .is_none());
    }

    #[test]
    fn underscore_default_refuses() {
        // FIRES upstream (probe-measured) but refused v1 — the documented
        // narrower-than-clippy divergence.
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, <_>::default())); }",
        )
        .is_none());
    }

    #[test]
    fn nondefault_second_arg_refuses() {
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, src: Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, src)); }",
        )
        .is_none());
    }

    #[test]
    fn three_args_refuses() {
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, Default::default(), 3)); }",
        )
        .is_none());
    }

    #[test]
    fn bare_replace_refuses() {
        // FIRES upstream (probe-measured, MachineApplicable) but a
        // syntax-only proposer cannot prove the import — the
        // SAFER-than-clippy divergence.
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { use std::mem::replace; sink.push(Vec::new()); sink.push(replace(dst, Default::default())); }",
        )
        .is_none());
    }

    #[test]
    fn local_mem_module_refuses_two_segment() {
        // Upstream is SILENT here (probe-measured) — the shadow guard is
        // parity.
        assert!(propose_fix(
            RULE,
            "mod mem { pub fn replace<T: Clone>(dst: &mut T, src: T) -> T { let out = dst.clone(); *dst = src; out } }\nfn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(mem::replace(dst, Default::default())); }",
        )
        .is_none());
    }

    #[test]
    fn local_mem_module_does_not_block_std_path() {
        // The 3-segment spelling is self-proving — it fires even when a
        // local `mem` item exists (the std path cannot be shadowed by it).
        let out = propose_fix(
            RULE,
            "mod mem { pub fn take<T>(dst: &mut T) -> T where T: Default { let out = dst.clone(); *dst = Default::default(); out } }\nfn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, Default::default())); }",
        )
        .expect("matched");
        assert!(out.contains("std::mem::take(dst)"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn two_segment_without_import_refuses() {
        // No `use std::mem;` — the 2-segment spelling is unprovable.
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(mem::replace(dst, Default::default())); }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            RULE,
            "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, /* now */ Default::default())); }",
        )
        .is_none());
    }
}
