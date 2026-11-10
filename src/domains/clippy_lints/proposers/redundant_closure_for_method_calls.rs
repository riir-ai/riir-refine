//! `clippy::redundant_closure_for_method_calls` —
//! `v.iter().map(|s| s.len())` → `v.iter().map(String::len)`
//! (Batch 157 / Plan 148 — the frontier-lead mining pack III; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery, 2026-09-18).
//!
//! Pedantic-group lint (fires under `-W clippy::pedantic`; the bench oracle
//! runs pedantic, so the freeze/score lanes see it).
//!
//! Pattern: a `.map(<closure>)` whose receiver is exactly `<binding>.iter()`,
//! where the binding is a SIGNATURE-PROVEN slice-container param
//! (`helpers::slice_param_heads`: `&[T]` / `&[T; N]` / `&Vec<T>`, shared ref,
//! never rebound in the body) and the closure has one plain param whose body
//! is exactly `p.method()` (0 args). The closure is replaced by
//! `T::method` — the element head's inherent-method path.
//!
//! **FIXABLE tier** — the rewrite is inference-identical BY CONSTRUCTION:
//! the derivation proves the closure param's type is `&T` (the signature
//! fact, not an inference), `p.method()` on `&T` resolves to `T`'s inherent
//! method (inherent beats trait on autoderef), and the emitted fn item
//! `T::method` carries exactly `fn(&T) -> _` — the signature the closure
//! had. The `(head, method)` pair must be in the WHITELIST below (std
//! containers × inherent shared `&self` 0-arg methods), because the emit
//! must be construction-identical for EVERY input that compiles, not merely
//! error-visible on the wrong ones: a local type's `m` could live on a
//! trait or on `&T` itself (the `impl Trait for &T` corner), where
//! `LocalType::m` is a DIFFERENT call that still compiles — the silent-FP
//! class. Whitelisted std heads cannot be redefined (see the shadow guard).
//!
//! Guards (parse-only provable):
//! - file shadow guard (`helpers::file_shadows`): items or non-std
//!   use-aliases named `String`/`Vec`/`HashMap`/`HashSet` refuse the whole
//!   file;
//! - the enclosing fn's generic type params refuse the fire when named like
//!   the resolved head (`fn f<String>(v: &[String])` — the element type is
//!   the GENERIC, and `String::len` would not resolve to std);
//! - attrs/turbofish on the `.map`/`.iter()` links, arg counts ≠ 1, closure
//!   attrs / `move`-capture variants beyond plain `move`, `mut`/`ref`
//!   closure params, multi-segment or generic-arg receivers, method args,
//!   and interior comments in the replaced closure range all refuse.

use std::collections::{BTreeMap, BTreeSet};

use syn::visit::Visit;
use syn::{Expr, ExprClosure, ExprMethodCall, File, ItemFn};

use super::helpers::{byte_range, drops_interior_comment, file_shadows, slice_param_heads_over};
use super::{SourceEdit, edit};

/// Names whose file-level shadowing makes the emit meaning-changing.
const SHADOW_NAMES: &[&str] = &["String", "Vec", "HashMap", "HashSet"];

/// The (element head, method) whitelist: inherent shared-`&self` 0-arg
/// methods of std containers — `len`, `is_empty`, `capacity` on all four.
/// Everything else refuses (a user type's method of the same name may be a
/// trait method or live on `&T`, where the emitted path is a different call
/// that still compiles — the construction-identity bar fails there).
fn whitelisted(head: &str, method: &str) -> bool {
    matches!(head, "String" | "Vec" | "HashMap" | "HashSet")
        && matches!(method, "len" | "is_empty" | "capacity")
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    if file_shadows(file, SHADOW_NAMES) {
        return Vec::new(); // conservatively empty — the guard refused the file
    }
    let mut v = Collector {
        source,
        edits: Vec::new(),
        ctx: None,
        saved: None,
    };
    v.visit_file(file);
    v.edits
}

/// The enclosing-fn context: signature-proven slice bindings.
struct FnCtx {
    bindings: BTreeMap<String, String>,
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    ctx: Option<FnCtx>,
    /// The context to restore on fn exit (one-deep save — fn nesting saves
    /// its own previous on entry).
    saved: Option<FnCtx>,
}

impl Collector<'_> {
    /// Build + install the fn context; the caller restores `saved` after.
    fn enter_fn<'i>(
        &mut self,
        inputs: impl Iterator<Item = &'i syn::FnArg>,
        generics: &syn::Generics,
        block: &syn::Block,
    ) {
        let saved = self.ctx.take();
        let mut bindings = slice_param_heads_over(inputs, block);
        // The generics guard: a type param named like a resolved head
        // invalidates that head's derivation for THIS fn.
        let generics: BTreeSet<String> = generics
            .params
            .iter()
            .filter_map(|p| match p {
                syn::GenericParam::Type(t) => Some(t.ident.to_string()),
                _ => None,
            })
            .collect();
        bindings.retain(|_, head| !generics.contains(head));
        self.ctx = Some(FnCtx { bindings });
        self.saved = saved;
    }
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        self.enter_fn(f.sig.inputs.iter(), &f.sig.generics, &f.block);
        syn::visit::visit_item_fn(self, f);
        self.ctx = self.saved.take();
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        // Methods get the same derivation as free fns (the `self` receiver
        // is skipped by the Typed-only param scan).
        self.enter_fn(f.sig.inputs.iter(), &f.sig.generics, &f.block);
        syn::visit::visit_impl_item_fn(self, f);
        self.ctx = self.saved.take();
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = self.try_fn_path(m) {
            self.edits.push(edit(&m.args[0], replacement));
        }
    }
}

impl Collector<'_> {
    fn try_fn_path(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "map" {
            return None;
        }
        if m.args.len() != 1 {
            return None;
        }
        // Receiver: exactly `<ident>.iter()`.
        let iter = unwrap_parens(&m.receiver);
        let Expr::MethodCall(iter) = iter else {
            return None;
        };
        if !iter.attrs.is_empty() || iter.turbofish.is_some() || iter.method != "iter" {
            return None;
        }
        if !iter.args.is_empty() {
            return None;
        }
        let Expr::Path(path) = unwrap_parens(&iter.receiver) else {
            return None;
        };
        if path.qself.is_some()
            || path.path.leading_colon.is_some()
            || path.path.segments.len() != 1
            || !path.path.segments[0].arguments.is_none()
        {
            return None;
        }
        let ident = path.path.segments[0].ident.to_string();
        let ctx = self.ctx.as_ref()?;
        let head = ctx.bindings.get(&ident)?;

        // The closure: one plain param, body exactly `p.method()`.
        let Expr::Closure(c) = &m.args[0] else {
            return None;
        };
        let (_, method) = closure_body_method(c)?;
        if !whitelisted(head, &method) {
            return None;
        }
        let range = byte_range(&m.args[0]);
        if drops_interior_comment(self.source, range.clone(), &[]) {
            return None;
        }
        Some(format!("{head}::{method}"))
    }
}

/// `(param ident, method ident)` when the closure is exactly `|p| p.m()`
/// (one plain input; the body a single 0-arg method call on `p`).
fn closure_body_method(c: &ExprClosure) -> Option<(String, String)> {
    if !c.attrs.is_empty() || c.constness.is_some() || c.movability.is_some() {
        return None;
    }
    if c.lifetimes.is_some() {
        return None;
    }
    if c.inputs.len() != 1 {
        return None;
    }
    let input = &c.inputs[0];
    let syn::Pat::Ident(pi) = input else {
        return None;
    };
    if pi.by_ref.is_some() || pi.subpat.is_some() || pi.mutability.is_some() {
        return None;
    }
    let body = unwrap_parens(&c.body);
    let Expr::MethodCall(call) = body else {
        return None;
    };
    if !call.attrs.is_empty() || call.turbofish.is_some() || !call.args.is_empty() {
        return None;
    }
    let Expr::Path(recv) = unwrap_parens(&call.receiver) else {
        return None;
    };
    if recv.qself.is_some()
        || recv.path.leading_colon.is_some()
        || recv.path.segments.len() != 1
        || !recv.path.segments[0].arguments.is_none()
        || recv.path.segments[0].ident != pi.ident
    {
        return None;
    }
    Some((pi.ident.to_string(), call.method.to_string()))
}

/// Peel paren layers (shape checks only — emitted text always slices the
/// ORIGINAL node, so parens ride verbatim).
fn unwrap_parens(mut e: &Expr) -> &Expr {
    while let Expr::Paren(p) = e {
        e = &p.expr;
    }
    e
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn rewrites_string_len_over_slice_param() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("v.iter().map(String::len).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rewrites_second_param_name() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn sizes(rows: &[String]) -> Vec<usize> { rows.iter().map(|r| r.len()).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("rows.iter().map(String::len).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rewrites_vec_param_and_is_empty() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn flags(bins: &[Vec<u8>]) -> Vec<bool> { bins.iter().map(|b| b.is_empty()).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("bins.iter().map(Vec::is_empty).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rewrites_hash_map_len() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn sizes(tables: &[HashMap<u8, u8>]) -> Vec<usize> { tables.iter().map(|t| t.len()).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("tables.iter().map(HashMap::len).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn vec_receiver_binding_qualifies() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &Vec<String>) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("v.iter().map(String::len).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_whitelisted_method_refused() {
        // `split` is real clippy surface for this lint, but not in the
        // construction-identity whitelist — silent (documented divergence).
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn words(xs: &[String]) -> Vec<&str> { xs.iter().map(|x| x.split(' ')).collect() }",
        );
        assert!(out.is_none(), "non-whitelisted method must refuse: {out:?}");
    }

    #[test]
    fn non_whitelisted_element_type_refused() {
        // Local `Foo` with (or without) `len` — the emit `Foo::len` is not
        // construction-identical (trait/`&T` corners) — silent.
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "struct Foo;\nimpl Foo { fn len(&self) -> usize { 0 } }\nfn lens(v: &[Foo]) -> Vec<usize> { v.iter().map(|f| f.len()).collect() }",
        );
        assert!(
            out.is_none(),
            "non-whitelisted element head must refuse: {out:?}"
        );
    }

    #[test]
    fn method_with_args_refused() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn cuts(v: &[String]) -> Vec<&str> { v.iter().map(|s| s.get(0..1).unwrap()).collect() }",
        );
        assert!(out.is_none(), "method args must refuse: {out:?}");
    }

    #[test]
    fn non_method_body_refused() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn go(v: &[u8]) -> Vec<u8> { v.iter().map(|x| x + 1).collect() }",
        );
        assert!(out.is_none(), "binary body must refuse: {out:?}");
    }

    #[test]
    fn tuple_closure_param_refused() {
        // `.iter()` over a map's k/v — tuple closure param (and a direct
        // `&HashMap` binding, not a slice-container) — no derivation, silent.
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn keys(m: &HashMap<u8, u8>) -> Vec<u8> { m.iter().map(|(k, _)| *k).collect() }",
        );
        assert!(out.is_none(), "tuple param must refuse: {out:?}");
    }

    #[test]
    fn impl_method_receiver_qualifies() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "struct S;
impl S {
    fn lens(&self, v: &[String]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }
}",
        )
        .expect("matched");
        assert!(
            out.contains("v.iter().map(String::len).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rebound_binding_refused() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &[String], flag: bool) -> Vec<usize> {\n    let v: &[String];\n    v = if flag { &[] } else { &[] };\n    v.iter().map(|s| s.len()).collect()\n}",
        );
        assert!(out.is_none(), "rebound binding must refuse: {out:?}");
    }

    #[test]
    fn shadowing_binding_in_closure_refused() {
        // A closure binding of the same name anywhere in the body refuses
        // (the rebind guard is position-blind, the conservative direction).
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &[String]) -> Vec<usize> {\n    let probe = |v: &str| v.len();\n    let _ = probe(\"\");\n    v.iter().map(|s| s.len()).collect()\n}",
        );
        assert!(
            out.is_none(),
            "same-name closure binding must refuse: {out:?}"
        );
    }

    #[test]
    fn generic_param_named_like_head_refused() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens<String>(v: &[String]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }",
        );
        assert!(out.is_none(), "generic head name must refuse: {out:?}");
    }

    #[test]
    fn file_shadowing_refused() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "mod Vec { pub fn x() {} }\nfn lens(v: &[String]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }",
        );
        assert!(out.is_none(), "mod Vec must trip the shadow guard: {out:?}");
    }

    #[test]
    fn move_closure_still_rewrites() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(move |s| s.len()).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("v.iter().map(String::len).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn interior_comment_in_closure_refused() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(|s| /* s */ s.len()).collect() }",
        );
        assert!(
            out.is_none(),
            "comment in the replaced range must refuse: {out:?}"
        );
    }

    #[test]
    fn mut_param_refused() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(|mut s| s.len()).collect() }",
        );
        assert!(out.is_none(), "`mut` closure param must refuse: {out:?}");
    }

    #[test]
    fn array_param_qualifies() {
        let out = propose_fix(
            "redundant_closure_for_method_calls",
            "fn lens(v: &[String; 4]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }",
        )
        .expect("matched");
        assert!(
            out.contains("v.iter().map(String::len).collect()"),
            "not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}
