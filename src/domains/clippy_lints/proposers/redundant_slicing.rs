//! `clippy::redundant_slicing` — `f(&s[..])` → `f(s)` when `s` is provably
//! already a `&str`/`&[T]` binding (Batch 155 / Plan 146 — the frontier-lead
//! mining pack; family fixture verified to fire exactly this lint under
//! all+pedantic+nursery, 2026-09-18).
//!
//! Pattern: a call argument (or method-call argument) of the shape
//! `&<ident>[..]` where `<ident>` resolves — parse-only — to a same-file fn
//! param of type `&str` or `&[T]`. For such a binding the full-range slice
//! re-borrows what the binding already is, so passing the binding directly
//! is type-identical.
//!
//! Resolution (the FIXABLE-tier justification, the `receiver_kind` precedent):
//! the file's fn params are collected up front; the ident must name at least
//! one param typed `&str`/`&[T]` AND no param of any OTHER type — a name
//! reused across fns with different types (e.g. a `String` param elsewhere)
//! is unresolvable parse-only and refuses. Locals and fields are invisible
//! to this resolver and also refuse (an owned `String` local named the same
//! is the E0308 case real clippy's types would have caught).
//!
//! Guards: the borrow must be immutable and un-annotated (`&mut s[..]`,
//! `&'a s[..]` refuse); the base must be a bare path (field/index/call bases
//! refuse — they are not param-resolvable); `Vec`-derived locals (`&v[..]`
//! where `v: Vec<T>` is the REAL clippy fix shape too — but type-blind for
//! us) refuse via the resolver.

use super::{edit, for_each_fn, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprIndex, ExprReference, File, RangeLimits, Signature, Type};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    let resolver = SliceBindingResolver::collect(file);
    let mut v = Collector {
        source,
        resolver: &resolver,
        edits: &mut edits,
    };
    v.visit_file(file);
    edits
}

/// idents that provably name a `&str`/`&[T]` binding (and nothing else).
struct SliceBindingResolver {
    slice_idents: std::collections::BTreeSet<String>,
    other_idents: std::collections::BTreeSet<String>,
}

/// Collect the shadow set: every LOCAL binding name (let pats, closure
/// params) anywhere in the file — a name that also names a local is
/// unresolvable parse-only (the local may shadow the param at the use
/// site, and the local's type is invisible).
struct LocalBindingCollector {
    names: std::collections::BTreeSet<String>,
}

impl<'ast> Visit<'ast> for LocalBindingCollector {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        collect_pat_idents(&l.pat, &mut self.names);
        syn::visit::visit_local(self, l);
    }

    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        for input in &c.inputs {
            collect_pat_idents(input, &mut self.names);
        }
        syn::visit::visit_expr_closure(self, c);
    }
}

/// Every ident bound by a pattern (top-level + sub-patterns — a tuple
/// binding `let (s, n) = ..` shadows BOTH names).
fn collect_pat_idents(pat: &syn::Pat, out: &mut std::collections::BTreeSet<String>) {
    match pat {
        syn::Pat::Ident(p) => {
            out.insert(p.ident.to_string());
            if let Some((_, sub)) = &p.subpat {
                collect_pat_idents(sub, out);
            }
        }
        syn::Pat::Tuple(p) => p.elems.iter().for_each(|e| collect_pat_idents(e, out)),
        syn::Pat::TupleStruct(p) => p.elems.iter().for_each(|e| collect_pat_idents(e, out)),
        syn::Pat::Struct(p) => p
            .fields
            .iter()
            .for_each(|f| collect_pat_idents(&f.pat, out)),
        syn::Pat::Slice(p) => p.elems.iter().for_each(|e| collect_pat_idents(e, out)),
        syn::Pat::Or(p) => p.cases.iter().for_each(|e| collect_pat_idents(e, out)),
        syn::Pat::Reference(p) => collect_pat_idents(&p.pat, out),
        syn::Pat::Paren(p) => collect_pat_idents(&p.pat, out),
        syn::Pat::Type(p) => collect_pat_idents(&p.pat, out),
        _ => {}
    }
}

impl SliceBindingResolver {
    fn collect(file: &File) -> Self {
        let mut r = Self {
            slice_idents: std::collections::BTreeSet::new(),
            other_idents: std::collections::BTreeSet::new(),
        };
        for_each_fn(file, |sig: &Signature, _block| {
            for input in &sig.inputs {
                let syn::FnArg::Typed(arg) = input else {
                    continue;
                };
                let syn::Pat::Ident(pat) = &*arg.pat else {
                    // Non-ident param patterns (tuples) bind names too —
                    // conservatively mark every name they bind as other.
                    let mut names = std::collections::BTreeSet::new();
                    collect_pat_idents(&arg.pat, &mut names);
                    r.other_idents.extend(names);
                    continue;
                };
                let name = pat.ident.to_string();
                if is_shared_slice_or_str(&arg.ty) {
                    r.slice_idents.insert(name);
                } else {
                    r.other_idents.insert(name);
                }
            }
        });
        // The shadow prepass: any LOCAL binding name anywhere blocks the
        // ident (the local may shadow the param at the use site and its
        // type is invisible).
        let mut locals = LocalBindingCollector {
            names: std::collections::BTreeSet::new(),
        };
        locals.visit_file(file);
        r.other_idents.extend(locals.names);
        r
    }

    /// `true` when the ident names a slice/str param and NO other-typed
    /// param anywhere in the file.
    fn resolves(&self, ident: &str) -> bool {
        self.slice_idents.contains(ident) && !self.other_idents.contains(ident)
    }
}

/// `&str` / `&[T]` (shared, un-annotated) — nothing else.
fn is_shared_slice_or_str(ty: &Type) -> bool {
    let Type::Reference(r) = ty else {
        return false;
    };
    if r.mutability.is_some() || r.lifetime.is_some() {
        return false;
    }
    match &*r.elem {
        Type::Path(p) => {
            p.qself.is_none()
                && p.path.leading_colon.is_none()
                && p.path.segments.len() == 1
                && p.path.segments[0].ident == "str"
        }
        Type::Slice(_) => true,
        _ => false,
    }
}

struct Collector<'a> {
    source: &'a str,
    resolver: &'a SliceBindingResolver,
    edits: &'a mut Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        syn::visit::visit_expr_call(self, call);
        self.scan_args(&call.args);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        self.scan_args(&m.args);
    }
}

impl Collector<'_> {
    fn scan_args(&mut self, args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>) {
        for arg in args {
            let Expr::Reference(ExprReference {
                mutability: None,
                expr: inner,
                ..
            }) = arg
            else {
                continue;
            };
            if let Some(base) = self.redundant_full_slice_base(inner) {
                if let Some(text) = src(self.source, base) {
                    self.edits.push(edit(arg, text));
                }
            }
        }
    }

    /// The BASE path of `<ident>[..]` when it is a redundant full slice on
    /// a resolvable slice/str param — the binding itself replaces the whole
    /// `&ident[..]` argument.
    fn redundant_full_slice_base<'e>(&self, e: &'e Expr) -> Option<&'e Expr> {
        let Expr::Index(ExprIndex { expr: base, index, .. }) = e else {
            return None;
        };
        let Expr::Path(p) = &**base else {
            return None;
        };
        if p.path.segments.len() != 1 {
            return None;
        }
        let ident = p.path.segments[0].ident.to_string();
        // The full-range index: `..` with neither bound.
        let Expr::Range(r) = &**index else {
            return None;
        };
        if r.start.is_some() || r.end.is_some() || !matches!(r.limits, RangeLimits::HalfOpen(_)) {
            return None;
        }
        if self.resolver.resolves(&ident) {
            Some(base)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn strips_full_slice_on_str_param() {
        let input = "fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(s: &str) -> char { first_char(&s[..]) }";
        let out = propose_fix("redundant_slicing", input).expect("matched");
        assert!(out.contains("first_char(s)"), "slice not stripped: {out}");
        assert!(out.contains("fn first_char(s: &str)"), "signature must stay: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn strips_full_slice_on_slice_param() {
        let input = "fn sum(v: &[u8]) -> u32 { v.iter().sum() }\nfn go(v: &[u8]) -> u32 { sum(&v[..]) }";
        let out = propose_fix("redundant_slicing", input).expect("matched");
        assert!(out.contains("sum(v)"), "slice not stripped: {out}");
    }

    #[test]
    fn method_arg_position_strips() {
        let input = "fn tag(v: &[u8]) -> String { String::from_utf8_lossy(&v[..]).into_owned() }";
        let out = propose_fix("redundant_slicing", input).expect("matched");
        assert!(out.contains("String::from_utf8_lossy(v)"), "method arg not stripped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn owned_string_param_refused() {
        // The resolver sees `s: String` — the &s[..] is a REAL borrow.
        let input = "fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(s: String) -> char { first_char(&s[..]) }";
        let out = propose_fix("redundant_slicing", input);
        assert!(out.is_none(), "String-typed ident must refuse: {out:?}");
    }

    #[test]
    fn local_binding_refused() {
        // A local `let s = ...` is invisible to the resolver — refuse.
        let input = "fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(src: &str) -> char { let s = src.to_uppercase(); first_char(&s[..]) }";
        let out = propose_fix("redundant_slicing", input);
        assert!(out.is_none(), "local binding must refuse: {out:?}");
    }

    #[test]
    fn partial_range_refused() {
        let input = "fn tail_at(s: &str, at: usize) -> &str { &s[at..] }\nfn go(s: &str) -> &str { tail_at(&s[..], 1) }";
        // `&s[at..]`? No — the ARG here is `&s[..]` (full), fine, but a
        // partial-index arg shape must refuse:
        let input2 = "fn tail(s: &str) -> &str { &s[1..] }\nfn go(s: &str) -> &str { tail(&s[1..]) }";
        let out = propose_fix("redundant_slicing", input).expect("matched");
        assert!(out.contains("tail_at(s, 1)"), "full-range arg must strip: {out}");
        let out2 = propose_fix("redundant_slicing", input2);
        assert!(out2.is_none(), "partial range must refuse: {out2:?}");
    }

    #[test]
    fn mut_borrow_refused() {
        let input = "fn take(v: &mut [u8]) {}\nfn go(v: &mut [u8]) { take(&mut v[..]) }";
        let out = propose_fix("redundant_slicing", input);
        assert!(out.is_none(), "&mut form must refuse: {out:?}");
    }

    #[test]
    fn non_path_base_refused() {
        // A field base (`&w.inner[..]`) is not param-resolvable — refuse.
        let input = "struct W { inner: [u8; 2] }\nfn takes(v: &[u8]) -> usize { v.len() }\nfn go(w: &W) -> usize { takes(&w.inner[..]) }";
        let out = propose_fix("redundant_slicing", input);
        assert!(out.is_none(), "field base must refuse: {out:?}");
    }
}
