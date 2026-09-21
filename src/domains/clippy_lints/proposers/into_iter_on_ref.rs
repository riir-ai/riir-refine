//! `clippy::into_iter_on_ref` — `(&v).into_iter()` → `v.iter()` (+ the
//! `(&mut v).into_iter()` → `iter_mut()` twin) (Batch 169 / Plan 160 — the
//! coverage-report pack XIII, the Batch-168 staged alternate; envelope
//! verified under the plain oracle 2026-09-21,
//! `scripts/plan160_pack13_probe.py`; the edit form is the SPLICE — the
//! follow-through below).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! For every container in the proven set, `impl IntoIterator for &T`
//! IS `self.iter()` (and `&mut T` IS `self.iter_mut()`) — std defines the
//! trait by delegating to the inherent method, so the rewrite calls
//! the SAME function on the SAME receiver. The receiver-ident's type is
//! proven by the enclosing fn's
//! signature (the `receiver_kind` / `param_heads_over` precedent): only a
//! param annotated with the delegation-law containers (or an array, each
//! behind at most one reference) qualifies, and the never-rebound guard
//! makes the annotation a construction fact.
//!
//! ## The edit form: the SPLICE, not the literal upstream suggestion
//!
//! Clippy's own help is the method rename alone ("call directly: `iter`"
//! — receiver parens KEPT: `(&v).iter()`), and that form MEASURED
//! co-firing `needless_borrow` (the redundant receiver borrow —
//! warn-by-default style, so every plain `--fix` left a NEW warning
//! behind). The born-lint slipped the landing gates by construction: the
//! score-bench created axis and the freeze e2e are FIXABLE-vocabulary-
//! filtered (`needless_borrow` is VERIFY-tier), and the probe measures
//! clippy on BUGGY fixtures, never the fixer's output. The follow-through
//! SPLICES instead — `[receiver.start, method.end)` → `v.iter` — dropping
//! the now-redundant borrow with the parens (the
//! `redundant_closure_for_method_calls` / `string_lit_as_bytes`
//! diverge-from-the-literal-suggestion precedent). The provenance guard
//! makes the splice trivially postfix-safe: the receiver inner is always
//! a proven plain ident.
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The receiver must be the PARENTHESIZED reference expression
//!   `(&e).into_iter()` / `(&mut e).into_iter()` — the parse-visible
//!   spelling. The typeck shapes clippy also catches (`r.into_iter()` on
//!   `r: &Vec<T>`) are out of a parse-only proposer's reach (the staged
//!   scope, Plan 159) and refuse.
//! - The reference target must be a plain ident bound as an enclosing-fn
//!   PARAM typed `Vec<..>`/`[T; N]`/`HashMap<..>`/`HashSet<..>`/
//!   `BTreeMap<..>`/`BTreeSet<..>`/`VecDeque<..>`/`BinaryHeap<..>`
//!   (optionally behind ONE reference) — locals, fields, calls, and
//!   non-container annotations refuse (unproven receivers are the
//!   documented divergence; the `unnecessary_map_or` unknown-receiver
//!   refusal is the precedent). Multi-segment paths (`std::vec::Vec`)
//!   refuse — the conservative direction.
//! - The ident must never be rebound in the body (`body_rebinds` — the
//!   soundness leg shared with the `param_heads_over` family).
//! - Inside a CLOSURE the provenance map is cleared — a closure param of
//!   the same name would shadow silently (refuse, documented).
//! - `for x in (&v).into_iter()` REFUSES: clippy fires there (probe), but
//!   the rename births `for x in (&v).iter()` — `explicit_iter_loop`
//!   territory — a style-for-style trade (documented divergence).
//! - Zero arguments, no turbofish; nested-paren receivers `((&v))` refuse.
//!
//! ## The born-lint axis (priced, not refused)
//!
//! `.count()` tails co-fire `iter_count` upstream (probe: 3 of 5 shapes)
//! and the rename moves them onto `(&v).iter().count()` — clippy's
//! `iter_count` fires on the fixed form exactly as it does on clippy's own
//! suggestion. That is the gen-8 shaped-verdict class (half-credit), never
//! a fixpoint ping: no proposer in this corpus touches a `.count()` tail.
//!
//! Distinct from `iter_nth_zero`/`iter_next_loop` (nth/next-method seams)
//! and from the Batch-167 probe REJECT `needless_borrowed_reference` (a
//! different clippy lint, silent on every natural shape there).

use std::collections::BTreeSet;

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, ItemFn};

use super::helpers::{body_rebinds, byte_range};
use super::SourceEdit;

/// The std containers whose `IntoIterator for (&|&mut) T` IS
/// `iter()`/`iter_mut()` — the delegation law (std defines the impls by
/// calling the inherent methods; arrays yield the same slice `Iter`).
const CONTAINER_HEADS: &[&str] = &[
    "Vec", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "VecDeque", "BinaryHeap",
];

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = Scan {
        proven: BTreeSet::new(),
        for_iter_ranges: Vec::new(),
        edits: Vec::new(),
    };
    scan.visit_file(file);
    scan.edits
}

struct Scan {
    /// enclosing-fn param idents proven container-typed (cleared in closures)
    proven: BTreeSet<String>,
    /// byte ranges of `for .. in EXPR` iter expressions (refused positions)
    for_iter_ranges: Vec<std::ops::Range<usize>>,
    edits: Vec<SourceEdit>,
}

/// The container head of a provable annotation: a single-segment path in
/// [`CONTAINER_HEADS`] (generic args unconstrained) or an array type,
/// each optionally behind ONE reference.
fn container_head(ty: &syn::Type) -> Option<&'static str> {
    match ty {
        syn::Type::Reference(r) => container_head(&r.elem),
        syn::Type::Array(_) => Some("[T; N]"),
        syn::Type::Path(p) => {
            if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1
            {
                return None;
            }
            let id = p.path.segments[0].ident.to_string();
            CONTAINER_HEADS.iter().find(|c| **c == id).copied()
        }
        _ => None,
    }
}

/// A plain single-segment ident path expression (`v`).
fn plain_ident(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1
        || !p.path.segments[0].arguments.is_none()
    {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

/// True when the ident is proven by `f`'s signature (params only v1).
fn proven_params(inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>, block: &syn::Block) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for arg in inputs {
        let syn::FnArg::Typed(p) = arg else { continue };
        let syn::Pat::Ident(pi) = &*p.pat else { continue };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        if container_head(&p.ty).is_none() {
            continue;
        }
        let name = pi.ident.to_string();
        if body_rebinds(block, &name) {
            continue;
        }
        out.insert(name);
    }
    out
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven.extend(proven_params(&f.sig.inputs, &f.block));
        syn::visit::visit_item_fn(self, f);
        self.proven = saved;
    }

    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        // a closure param of the same name would shadow the fn param —
        // provenance does not cross the boundary (documented refusal)
        let saved = std::mem::take(&mut self.proven);
        syn::visit::visit_expr_closure(self, c);
        self.proven = saved;
    }

    fn visit_expr_for_loop(&mut self, f: &'ast syn::ExprForLoop) {
        self.for_iter_ranges.push(byte_range(&f.expr));
        syn::visit::visit_expr_for_loop(self, f);
        self.for_iter_ranges.pop();
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if m.method != "into_iter" || !m.args.is_empty() || m.turbofish.is_some() {
            return;
        }
        let Expr::Paren(paren) = &*m.receiver else { return };
        let Expr::Reference(r) = &*paren.expr else { return };
        let Some(name) = plain_ident(&r.expr) else { return };
        if !self.proven.contains(&name) {
            return;
        }
        let method_span = byte_range(&m.method);
        if self
            .for_iter_ranges
            .iter()
            .any(|rg| rg.contains(&method_span.start))
        {
            return; // for-head: the rewrite births explicit_iter_loop
        }
        // The SPLICE (Batch 169 follow-through): replace the whole
        // `(&v).into_iter` head with `v.iter` — the parens-kept upstream
        // suggestion MEASURED co-firing `needless_borrow` on the fixed
        // form (the redundant receiver borrow; warn-by-default style, so
        // every plain `--fix` left a NEW warning behind and the
        // vocabulary-filtered created axis could not see it). The splice
        // drops the now-redundant borrow with the parens; the ident is a
        // proven plain ident, so the splice is postfix-safe by the guard
        // set itself.
        let recv_start = byte_range(m.receiver.as_ref()).start;
        let method = if r.mutability.is_some() { "iter_mut" } else { "iter" };
        self.edits.push(SourceEdit {
            start: recv_start,
            end: method_span.end,
            replacement: format!("{name}.{method}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_fires_and_splices() {
        let src = "pub fn f(v: Vec<u64>) -> Option<u64> {\n    (&v).into_iter().next().copied()\n}";
        let out = propose_fix("into_iter_on_ref", src).expect("matched");
        assert!(out.contains("v.iter()"), "splice missing: {out}");
        assert!(!out.contains("(&v)"), "parens survived (the born needless_borrow form): {out}");
        assert!(!out.contains("into_iter"), "old name survived: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mut_twin_splices_iter_mut() {
        let src = "pub fn f(mut v: Vec<u64>) -> usize {\n    (&mut v).into_iter().count()\n}";
        let out = propose_fix("into_iter_on_ref", src).expect("matched");
        assert!(out.contains("v.iter_mut()"), "mut splice missing: {out}");
        assert!(!out.contains("(&mut v)"), "parens survived: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ref_param_container_fires() {
        // v behind one shared reference — head still Vec, `(&*v)` shapes
        // aside, the parenthesized (&v) receiver is the spelling we catch
        let src = "pub fn f(v: &Vec<u64>) -> Option<&u64> {\n    (&v).into_iter().next()\n}";
        let out = propose_fix("into_iter_on_ref", src).expect("matched");
        assert!(out.contains("v.iter()"));
    }

    #[test]
    fn array_param_fires() {
        let src = "pub fn f(a: [u64; 4]) -> Option<&u64> {\n    (&a).into_iter().next()\n}";
        let out = propose_fix("into_iter_on_ref", src).expect("matched");
        assert!(out.contains("a.iter()"));
    }

    #[test]
    fn hashmap_param_fires() {
        let src = "use std::collections::HashMap;\npub fn f(m: HashMap<u64, u64>) -> Option<(&u64, &u64)> {\n    (&m).into_iter().next()\n}";
        let out = propose_fix("into_iter_on_ref", src).expect("matched");
        assert!(out.contains("m.iter()"));
    }

    #[test]
    fn unproven_local_refuses() {
        // `let v = vec![..]` — locals are out of v1 provenance (documented)
        let src = "pub fn f() -> Option<&'static u64> {\n    let v: Vec<u64> = Vec::new();\n    (&v).into_iter().next()\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn non_container_param_refuses() {
        let src = "pub fn f(s: String) -> usize {\n    (&s).into_iter().count()\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn non_paren_receiver_refuses() {
        // the typeck shape (r already a reference) — parse-invisible
        let src = "pub fn f(r: &Vec<u64>) -> usize {\n    r.into_iter().count()\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn field_receiver_refuses() {
        let src = "pub struct S { v: Vec<u64> }\npub fn f(s: &S) -> Option<&u64> {\n    (&s.v).into_iter().next()\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn for_head_refuses() {
        // clippy fires here (probe) — we refuse: the rename births
        // explicit_iter_loop territory (the documented divergence)
        let src = "pub fn f(v: Vec<u64>) -> u64 {\n    let mut s = 0u64;\n    for x in (&v).into_iter() {\n        s += x;\n    }\n    s\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn closure_body_refuses() {
        // provenance does not cross closure boundaries (shadowing guard)
        let src = "pub fn f(v: Vec<u64>) -> Option<&u64> {\n    let g = || (&v).into_iter().next();\n    g()\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn rebinding_refuses() {
        let src = "pub fn f(v: Vec<u64>) -> Option<&u64> {\n    let v = vec![1u64];\n    (&v).into_iter().next()\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn args_and_turbofish_refuse() {
        let src = "pub fn f(v: Vec<u64>) -> Option<&u64> {\n    (&v).into_iter::<&u64>().next()\n}";
        assert!(propose_fix("into_iter_on_ref", src).is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let src = "pub fn f(v: Vec<u64>) -> Option<u64> {\n    (&v).into_iter().next().copied()\n}";
        let first = propose_fix("into_iter_on_ref", src).expect("matched");
        assert!(propose_fix("into_iter_on_ref", &first).is_none());
    }
}
