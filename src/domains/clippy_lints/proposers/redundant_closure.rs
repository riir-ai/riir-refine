//! `clippy::redundant_closure` — `|x| f(x)` → `f` when the closure body is a
//! single call passing the closure's only parameter as the only argument.
//!
//! Span-preserving (T6.4): the closure's byte range is replaced by the
//! ORIGINAL source text of the callee path.
//!
//! E0631 guard (the 2026-08-29 sweep class): a `|w| upload(w)` where
//! `upload`'s parameter type was pinned by an ANNOTATION is NOT redundant —
//! the annotation is what makes the item type deref-coerce at the call
//! (`Option<Vec<f32>>.as_ref().map(|w| upload(w))` with
//! `let upload = |data: &[f32]| ..` breaks as `.map(upload)`: expected
//! `&[f32]`, found `&Vec<f32>`). An UNANNOTATED closure binding is safe: its
//! param type is inferred FROM this call site, so the bare path re-derives
//! the same type. The proposer therefore skips a bare-ident callee that
//! resolves (same file) to a closure binding with any annotated param, or
//! to a `fn` item with any reference-typed parameter (a reference param CAN
//! be a deref-coercion target; by-value params never coerce). Unknown
//! callees (qualified paths, external fns) still fire — a generic or exact
//! match is implied by the original call compiling. VERIFY-tier backstop
//! unchanged: the compile gate reverts anything this guard's heuristics
//! miss.

use super::{SourceEdit, edit, src};
use std::collections::HashSet;
use syn::visit::Visit;
use syn::{Expr, ExprClosure, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let (annotated_closures, ref_param_fns) = collect_callee_kinds(file);
    let mut v = Collector {
        source,
        edits: Vec::new(),
        annotated_closures,
        ref_param_fns,
    };
    v.visit_file(file);
    v.edits
}

/// Same-file callee knowledge: closure `let` bindings with any annotated
/// param, and `fn` items with any reference-typed parameter. Name-based and
/// file-wide — shadowing across scopes is accepted as a conservative skip.
fn collect_callee_kinds(file: &File) -> (HashSet<String>, HashSet<String>) {
    let mut annotated = HashSet::new();
    let mut ref_fns = HashSet::new();
    let mut v = CalleeScanner {
        annotated: &mut annotated,
        ref_fns: &mut ref_fns,
    };
    v.visit_file(file);
    (annotated, ref_fns)
}

struct CalleeScanner<'a> {
    annotated: &'a mut HashSet<String>,
    ref_fns: &'a mut HashSet<String>,
}

impl<'ast> Visit<'ast> for CalleeScanner<'_> {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let syn::Pat::Ident(pi) = &l.pat {
            if let Some(syn::Expr::Closure(c)) = l.init.as_ref().map(|i| &*i.expr) {
                if c.inputs.iter().any(|p| matches!(p, syn::Pat::Type(_))) {
                    self.annotated.insert(pi.ident.to_string());
                }
            }
        }
        syn::visit::visit_local(self, l);
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if has_ref_param(&i.sig) {
            self.ref_fns.insert(i.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        if has_ref_param(&i.sig) {
            self.ref_fns.insert(i.sig.ident.to_string());
        }
        syn::visit::visit_impl_item_fn(self, i);
    }

    fn visit_trait_item_fn(&mut self, i: &'ast syn::TraitItemFn) {
        if has_ref_param(&i.sig) {
            self.ref_fns.insert(i.sig.ident.to_string());
        }
        syn::visit::visit_trait_item_fn(self, i);
    }
}

fn has_ref_param(sig: &syn::Signature) -> bool {
    sig.inputs.iter().any(|arg| {
        matches!(
            arg,
            syn::FnArg::Typed(pt) if matches!(&*pt.ty, syn::Type::Reference(_))
        )
    })
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    annotated_closures: HashSet<String>,
    ref_param_fns: HashSet<String>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_closure(&mut self, c: &'ast ExprClosure) {
        // Depth-first: inner redundant closures collect their own edits (an
        // outer match's replacement region contains no other bare-id use, so
        // overlap selection cannot drop a live inner fix here).
        syn::visit::visit_expr_closure(self, c);
        if let Some(callee) = self.simplify_closure(c) {
            if let Some(text) = src(self.source, callee) {
                self.edits.push(edit(c, text));
            }
        }
    }
}

impl Collector<'_> {
    /// If the closure is `|x| f(x)`, return the callee path `f`. Otherwise None.
    fn simplify_closure<'a>(&self, closure: &'a ExprClosure) -> Option<&'a syn::ExprPath> {
        if closure.inputs.len() != 1 {
            return None;
        }
        // The body is `Box<Expr>`; for `|x| f(x)`, the body is the call directly.
        // For `|x| { f(x) }`, the body is an `ExprBlock`. Handle the bare-call form.
        let call_expr = match &*closure.body {
            Expr::Call(_) => &*closure.body,
            Expr::Block(b) if b.block.stmts.len() == 1 => match &b.block.stmts[0] {
                syn::Stmt::Expr(e, None) => e,
                _ => return None,
            },
            _ => return None,
        };
        let Expr::Call(call) = call_expr else {
            return None;
        };
        if call.args.len() != 1 {
            return None;
        }
        let arg = &call.args[0];
        let syn::Pat::Ident(param_pat) = &closure.inputs[0] else {
            return None;
        };
        let arg_ident = match arg {
            Expr::Path(p) => p.path.get_ident(),
            _ => return None,
        };
        let arg_ident = arg_ident?;
        if arg_ident != &param_pat.ident {
            return None;
        }
        let Expr::Path(callee_path) = &*call.func else {
            return None;
        };
        // E0631 guard: a bare-ident callee whose same-file definition pins a
        // type that can diverge from the item type (an annotated closure
        // binding, or a fn item taking a reference) stays a closure — the
        // annotation/reference IS the coercion site.
        if let Some(seg) = callee_path.path.segments.last() {
            let name = seg.ident.to_string();
            if callee_path.path.leading_colon.is_none()
                && callee_path.path.segments.len() == 1
                && (self.annotated_closures.contains(&name) || self.ref_param_fns.contains(&name))
            {
                return None;
            }
        }
        Some(callee_path)
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn simplifies_redundant_closure_in_call() {
        let input = "fn sq(x: u32) -> u32 { x * x }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| sq(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(
            out.contains(".map(sq)"),
            "closure not replaced by bare path: {out}"
        );
        assert!(
            out.contains("fn sq(x: u32) -> u32 { x * x }"),
            "unrelated fn must be byte-identical: {out}"
        );
    }

    #[test]
    fn leaves_non_redundant_closure() {
        let input = "fn go() { let f = |x: i32| x + 1; }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "non-redundant closure touched: {out:?}");
    }

    #[test]
    fn leaves_two_arg_closure() {
        let input = "fn go() { let f = |x: i32, y: i32| add(x, y); }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "two-param closure touched: {out:?}");
    }

    #[test]
    fn leaves_method_call_closure() {
        let input = "fn go() { let f = |x: i32| x.abs(); }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "method-call closure touched: {out:?}");
    }

    #[test]
    fn qualified_path_callee_preserved() {
        let input =
            "fn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| u32::from(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(out.contains("map(u32::from)"), "{out}");
    }

    #[test]
    fn skips_annotated_closure_binding_callee() {
        // THE 2026-08-29 sweep E0631 class: `upload`'s `&[f32]` annotation is
        // the coercion site — `.map(upload)` needs upload: Fn(&Vec<f32>) and
        // fails. Must stay silent.
        let input = "fn run(w: Option<Vec<f32>>) -> Option<u32> {\n    let upload = |data: &[f32]| -> u32 { data.len() as u32 };\n    w.as_ref().map(|x| upload(x))\n}";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "annotated-closure callee fired: {out:?}");
    }

    #[test]
    fn unannotated_closure_binding_callee_still_fires() {
        // Positive control: an unannotated binding infers its param FROM this
        // call site, so the bare path re-derives the same type — safe.
        let input = "fn run(xs: Vec<u32>) -> Vec<u32> {\n    let double = |v| v * 2;\n    xs.iter().map(|n| double(n)).collect()\n}";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(
            out.contains("map(double)"),
            "unannotated callee lost: {out}"
        );
    }

    #[test]
    fn skips_fn_with_reference_param() {
        // `first(v)` coerces &Vec<u32> → &[u32]; `.map(first)` would need
        // Fn(&Vec<u32>) -> u32 — E0631. Reference-typed params can always be
        // a coercion target, so the fn-item form skips too.
        let input = "fn first(s: &[u32]) -> u32 { s[0] }\nfn run(vv: Vec<Vec<u32>>) -> Vec<u32> { vv.iter().map(|v| first(v)).collect() }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "reference-param fn callee fired: {out:?}");
    }

    #[test]
    fn fn_with_value_param_still_fires() {
        // By-value params never deref-coerce — the rewrite is safe.
        let input = "fn inc(x: u32) -> u32 { x + 1 }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| inc(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(out.contains("map(inc)"), "by-value fn callee lost: {out}");
    }
}
