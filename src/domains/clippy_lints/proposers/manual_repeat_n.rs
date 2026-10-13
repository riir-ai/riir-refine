//! `clippy::manual_repeat_n` — `std::iter::repeat(x).take(n)` →
//! `std::iter::repeat_n(x, n)` (Batch 228 / Plan 230 — the coverage-report
//! re-arm pack XIX; the firing/silence matrix verified against real clippy
//! 0.1.98, 2026-10-13, `scripts/plan230_pack19_probe.py`, VERDICT PASS).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `std::iter::repeat_n` is DEFINED by std as exactly the expansion it
//! replaces — the std source is `pub fn repeat_n<T: Clone>(value: T,
//! count: usize) -> RepeatN<T>`, and `RepeatN`'s `Iterator` impl yields
//! the value `count` times, CLONING per yield: the same element sequence
//! as `repeat(value).take(count)` (which clones per yield too). The
//! element sequence is value-identical by construction, and the input's
//! own compilation proves `T: Clone` (the `repeat(x)` call requires it) —
//! zero type-blindness (the std definition-identity class of Batch 160's
//! radix-10 `FromStr`, Batch 162's `bytes().count()`, Batch 191's
//! `mem::take`).
//!
//! ## Guards (the bounded fix-space)
//!
//! - The outer call is the METHOD `.take(<n>)`; its receiver is EXACTLY a
//!   pure path call `repeat(<x>)` — callee `Expr::Path` only (a method or
//!   field receiver refuses), qself-free, generic-args-free (the
//!   `repeat::<T>(..)` turbofish refuses), exactly 1 arg to each call, no
//!   attributes on either call.
//! - `std::iter::repeat` (3 segments) is self-proving — no import state,
//!   and a local `iter`/`repeat` item must NOT block it.
//! - `iter::repeat` (2 segments) fires ONLY with a plain `use std::iter;`
//!   present AND nothing named `iter` anywhere in the file (item, use
//!   rename, glob — the `legacy_numeric_constants` shadow-guard class).
//! - Bare `repeat` (1 segment) fires ONLY with `use std::iter::repeat;`
//!   present AND nothing named `repeat` anywhere (same class; the plain
//!   import spelling FIRES upstream probe-measured with the
//!   fully-qualified suggestion, so fixing it is parity, not divergence).
//! - The emitted form is ALWAYS the fully-qualified
//!   `std::iter::repeat_n(<x>, <n>)` — upstream's own suggestion at every
//!   posture (probe-measured); emit exactly what upstream suggests, the
//!   divergence-free choice.
//! - A comment inside the replaced span refuses (the span re-spells to
//!   `std::iter::repeat_n(<x>, <n>)`).
//! - The rewrite covers ONLY the `repeat(X).take(N)` chain — the tail
//!   (`.collect()`, `.count()`, …) is preserved, matching upstream's
//!   `MachineApplicable` child-suggestion semantics.
//! - An attribute on a method-call expression is not spellable in surface
//!   Rust — the `attrs` guards are parity armor, untestable by fixture.
//!
//! MSRV: `repeat_n` stabilized in Rust 1.82; the workspace toolchain
//! (1.98) clears the floor (the corpus entry's `context_note` carries it).

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File};

use super::helpers::byte_range;
use super::{edit, src, SourceEdit};
use crate::edit_emitter::drops_interior_comment;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let imports = RepeatImports::scan(file);
    let mut v = Collector { source, edits: Vec::new(), imports };
    v.visit_file(file);
    v.edits
}

/// The file-level import state for the 2-segment `iter::repeat` and
/// bare-`repeat` spellings: the matching plain `use` must be present, and
/// nothing may shadow the name the shortened spelling leans on (an item,
/// or a use alias/rename binding it to a non-std path). The 3-segment
/// `std::iter::repeat` spelling needs no import state — the path is
/// self-proving.
#[allow(clippy::struct_excessive_bools)] // the four flags ARE the guard state: two proving imports + two shadow refusals
struct RepeatImports {
    has_std_iter_use: bool,
    has_repeat_use: bool,
    iter_shadowed: bool,
    repeat_shadowed: bool,
}

impl RepeatImports {
    fn scan(file: &File) -> Self {
        let mut v = Scan {
            out: RepeatImports {
                has_std_iter_use: false,
                has_repeat_use: false,
                iter_shadowed: false,
                repeat_shadowed: false,
            },
        };
        v.visit_file(file);
        v.out
    }
}

/// The whole-file scanner (the `legacy_numeric_constants` whole-file-guard
/// posture): a plain std use at ANY depth proves the shortened spelling
/// CAN resolve to std in this file; any same-named item, use binding, or
/// extern-crate alias at any depth refuses it (conservative — a fn-scoped
/// shadow we cannot attribute precisely would make the rewrite resolve to
/// the wrong `repeat`).
struct Scan {
    out: RepeatImports,
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
        if name.is_some_and(|n| n == "iter") {
            self.out.iter_shadowed = true;
        }
        if name.is_some_and(|n| n == "repeat") {
            self.out.repeat_shadowed = true;
        }
        syn::visit::visit_item(self, i);
    }

    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        if is_std_iter_tree(&u.tree) {
            self.out.has_std_iter_use = true;
        } else if is_std_iter_repeat_tree(&u.tree) {
            self.out.has_repeat_use = true;
        } else {
            if binds_name(&u.tree, "iter") {
                self.out.iter_shadowed = true;
            }
            if binds_name(&u.tree, "repeat") {
                self.out.repeat_shadowed = true;
            }
        }
        syn::visit::visit_item_use(self, u);
    }
}

/// `use std::iter;` — the exact plain tree proving the 2-segment
/// `iter::repeat` spelling (no rename, no glob, no group).
fn is_std_iter_tree(tree: &syn::UseTree) -> bool {
    let syn::UseTree::Path(p0) = tree else { return false };
    if p0.ident != "std" {
        return false;
    }
    let syn::UseTree::Name(n1) = &*p0.tree else { return false };
    n1.ident == "iter"
}

/// `use std::iter::repeat;` — the exact plain tree proving the bare
/// `repeat` spelling (no rename, no glob, no group).
fn is_std_iter_repeat_tree(tree: &syn::UseTree) -> bool {
    let syn::UseTree::Path(p0) = tree else { return false };
    if p0.ident != "std" {
        return false;
    }
    let syn::UseTree::Path(p1) = &*p0.tree else { return false };
    if p1.ident != "iter" {
        return false;
    }
    let syn::UseTree::Name(n2) = &*p1.tree else { return false };
    n2.ident == "repeat"
}

/// Whether a NON-std use tree binds the bare name `name` (an alias target
/// or a final segment) — the rename/glob shapes a syntax-only proposer
/// cannot resolve.
fn binds_name(tree: &syn::UseTree, name: &str) -> bool {
    match tree {
        syn::UseTree::Name(n) => n.ident == name,
        syn::UseTree::Rename(r) => r.rename == name,
        syn::UseTree::Glob(_) => true, // a glob could bind anything — refuse
        syn::UseTree::Path(p) => binds_name(&p.tree, name),
        syn::UseTree::Group(g) => g.items.iter().any(|t| binds_name(t, name)),
    }
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    imports: RepeatImports,
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
    /// The outer `.take(<n>)`; the receiver must be a pure path call
    /// `repeat(<x>)` in one of the three guarded spellings.
    fn try_rewrite(&mut self, take: &ExprMethodCall) -> Option<String> {
        if take.method != "take"
            || take.args.len() != 1
            || !take.attrs.is_empty()
            || take.turbofish.is_some()
        {
            return None;
        }
        let Expr::Call(repeat_call) = &*take.receiver else {
            return None;
        };
        if !repeat_call.attrs.is_empty() {
            return None;
        }
        // Not a pure path — a method or field receiver refuses.
        let Expr::Path(callee) = &*repeat_call.func else {
            return None;
        };
        if callee.qself.is_some() || !callee.attrs.is_empty() {
            return None;
        }
        // Generic args on ANY segment (`repeat::<u32>(..)` — they live on
        // the last segment, but checking all is the conservative superset).
        if callee
            .path
            .segments
            .iter()
            .any(|s| !matches!(s.arguments, syn::PathArguments::None))
        {
            return None;
        }
        let segs: Vec<&syn::Ident> =
            callee.path.segments.iter().map(|s| &s.ident).collect();
        let spelling_ok = match segs.as_slice() {
            [std_seg, iter_seg, repeat_seg] => {
                *std_seg == "std" && *iter_seg == "iter" && *repeat_seg == "repeat"
            }
            [iter_seg, repeat_seg] => {
                *iter_seg == "iter"
                    && *repeat_seg == "repeat"
                    && self.imports.has_std_iter_use
                    && !self.imports.iter_shadowed
            }
            [repeat_seg] => {
                *repeat_seg == "repeat"
                    && self.imports.has_repeat_use
                    && !self.imports.repeat_shadowed
            }
            _ => false,
        };
        if !spelling_ok {
            return None;
        }
        if repeat_call.args.len() != 1 {
            return None;
        }
        let value = repeat_call.args.first()?;
        let count = take.args.first()?;
        let chain_span = byte_range(take);
        if drops_interior_comment(self.source, chain_span, &[]) {
            return None;
        }
        // `std::iter::repeat_n(<x>, <n>)` — ALWAYS the fully-qualified
        // form (upstream's own suggestion at every posture,
        // probe-measured); the arg texts splice verbatim from their own
        // spans.
        let value_src = src(self.source, value)?;
        let count_src = src(self.source, count)?;
        Some(format!("std::iter::repeat_n({value_src}, {count_src})"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "manual_repeat_n";

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            RULE,
            "fn t(s: &str, hits: &mut Vec<u8>) { hits.push(0); s.bytes().collect(); }",
        )
        .is_none());
    }

    #[test]
    fn std_path_fires() {
        let out = propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u32> { std::iter::repeat(0u32).take(n).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("std::iter::repeat_n(0u32, n)"),
            "not rewritten: {out}"
        );
        assert!(!out.contains(".take("), "chain not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn imported_spelling_fires() {
        let out = propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<String> { use std::iter::repeat; repeat(\"x\").take(n).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("std::iter::repeat_n(\"x\", n)"),
            "not the fully-qualified upstream suggestion: {out}"
        );
        assert!(out.contains("use std::iter::repeat;"), "import dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn two_segment_iter_spelling_fires() {
        let out = propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<char> { use std::iter; iter::repeat('a').take(n).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("std::iter::repeat_n('a', n)"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn const_take_expr_fires() {
        let out = propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u64> { std::iter::repeat(1u64).take(n + 1).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("std::iter::repeat_n(1u64, n + 1)"),
            "take expr not spliced verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn expr_context_preserves_tail() {
        let out = propose_fix(
            RULE,
            "fn t(n: usize, seeds: &[u32]) -> usize { std::iter::repeat(seeds).take(n).count() }",
        )
        .expect("matched");
        assert!(
            out.contains("std::iter::repeat_n(seeds, n).count()"),
            "tail not preserved: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn literal_take_fires() {
        let out = propose_fix(
            RULE,
            "fn t() -> Vec<u32> { std::iter::repeat(7u32).take(3).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("std::iter::repeat_n(7u32, 3)"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn two_args_to_repeat_refuses() {
        // Does NOT compile (repeat takes 1 arg) but PARSES — the arity
        // guard is structural; propose_fix only parses.
        assert!(propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u32> { std::iter::repeat(1u32, 2u32).take(n).collect() }",
        )
        .is_none());
    }

    #[test]
    fn zero_args_refuses() {
        assert!(propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u32> { std::iter::repeat().take(n).collect() }",
        )
        .is_none());
    }

    #[test]
    fn two_args_to_take_refuses() {
        // A method call with 2 args parses (and would not compile) — the
        // arity guard refuses it structurally.
        assert!(propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u32> { std::iter::repeat(0u32).take(n, 5u32).collect() }",
        )
        .is_none());
    }

    #[test]
    fn turbofish_qself_refuses() {
        // The generic args live on the callee path's last segment — not
        // the upstream firing shape (probe-measured silent).
        assert!(propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u32> { std::iter::repeat::<u32>(0u32).take(n).collect() }",
        )
        .is_none());
    }

    #[test]
    fn no_use_bare_repeat_refuses() {
        // Bare `repeat` with NO import — the spelling is unprovable (this
        // would not compile either, but the guard is structural).
        assert!(propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u32> { repeat(0u32).take(n).collect() }",
        )
        .is_none());
    }

    #[test]
    fn local_iter_module_refuses_two_segment() {
        // A local `iter` item shadows the name the 2-segment spelling
        // leans on — the shadow guard refuses (conservative; the rewrite
        // under a shadowed `iter` resolves to the WRONG module).
        assert!(propose_fix(
            RULE,
            "mod iter { pub fn repeat<T: Clone>(value: T) -> impl Iterator<Item = T> { std::iter::repeat(value) } }\nfn t(n: usize) -> Vec<u32> { iter::repeat(0u32).take(n).collect() }",
        )
        .is_none());
    }

    #[test]
    fn local_repeat_fn_refuses_one_segment() {
        // A local `repeat` fn shadows the bare spelling — same shadow
        // guard (the SAFER-than-clippy posture).
        assert!(propose_fix(
            RULE,
            "fn repeat<T: Clone>(value: T) -> impl Iterator<Item = T> { std::iter::repeat(value) }\nfn t(n: usize) -> Vec<u32> { repeat(0u32).take(n).collect() }",
        )
        .is_none());
    }

    #[test]
    fn local_item_does_not_block_std_path() {
        // The 3-segment spelling is self-proving — it fires even when BOTH
        // a local `iter` module and a local `repeat` fn exist (a std path
        // cannot be shadowed by them).
        let out = propose_fix(
            RULE,
            "mod iter { pub fn repeat<T: Clone>(value: T) -> impl Iterator<Item = T> { std::iter::repeat(value) } }\nfn repeat<T: Clone>(value: T) -> impl Iterator<Item = T> { std::iter::repeat(value) }\nfn t(n: usize) -> Vec<u32> { std::iter::repeat(0u32).take(n).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("std::iter::repeat_n(0u32, n)"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            RULE,
            "fn t(n: usize) -> Vec<u32> { std::iter::repeat(/* now */ 0u32).take(n).collect() }",
        )
        .is_none());
    }
}
