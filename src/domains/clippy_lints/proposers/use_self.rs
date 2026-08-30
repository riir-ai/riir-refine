//! `clippy::use_self` — unnecessary structure-name repetition inside its own
//! inherent `impl` (Batch 89; the Plan 049 T0.1 fired histogram: 130 real
//! instances).
//!
//! Probe-verified firing shapes (clippy 1.97, all-groups JSON; every
//! suggestion `MachineApplicable`, replacing the name segment with `Self`):
//! return types `-> S`, struct literals `S { .. }`,
//! associated paths `S::K`/`S::new()`, ctor calls, and the same inside
//! GENERIC impls (`impl<T> G<T> { fn f() -> G<T> }` — args must match the
//! impl head's). Trait impls are skipped by real clippy's report on our
//! probe shape and conservatively skipped here in v1.
//!
//! Parse-only soundness guards (why this is FIXABLE despite being a
//! path-rewrite lint):
//! 1. **Inherent impls only** — `imp.trait_.is_none()`; the self type must be
//!    a single-segment path (qualified `some_mod::S` self-types are skipped).
//! 2. **Nested items are never entered** (`fn`/`mod`/`use` declared inside a
//!    method body cannot see `Self` — rewriting there would not compile).
//! 3. **Bounds, attributes, and macro bodies are never entered** (`T: S`
//!    bound rewrites are invalid; `#[doc = "S"]` is not code; macro tokens
//!    are opaque).
//! 4. **Generic-argument identity** — `G<..>` is rewritten only when its
//!    argument source text equals the impl head's argument text
//!    (`impl<T> G<T>` fires `G<T>`, never `G<String>`; `impl G<u32>` fires
//!    `G<u32>`). Bare `G<..>` inside a non-generic impl of `G` is invalid
//!    code and never fires.
//! 5. **The value-namespace ambiguity** — a bare single-segment path in
//!    *expression* position could resolve to a `const S` / a CamelCase `fn
//!    S` instead of the type (both legal, both style-lint-flagged, and
//!    silent semantic changes if rewritten). Skipped: bare expr paths,
//!    bare call-callee paths, and bare pattern paths whose ident carries
//!    no lowercase letter (`SCREAMING_SNAKE` const-pattern territory).
//!    `S { .. }` literals, `S(..)`-callee multi-segment paths, and every
//!    TYPE-position path are unambiguous and fire.

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{File, ImplItem, Item, Path, PathArguments, Pat, Type, TypePath};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    let mut stack: Vec<&Item> = file.items.iter().collect();
    while let Some(item) = stack.pop() {
        match item {
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    stack.extend(inner.iter());
                }
            }
            Item::Impl(imp) => {
                if imp.trait_.is_some() {
                    continue;
                }
                collect_impl(source, imp, &mut edits);
            }
            _ => {}
        }
    }
    edits
}

fn collect_impl(source: &str, imp: &syn::ItemImpl, edits: &mut Vec<SourceEdit>) {
    let Type::Path(tp) = &*imp.self_ty else { return };
    if tp.qself.is_some() || tp.path.segments.len() != 1 || tp.path.leading_colon.is_some() {
        return;
    }
    let seg = tp.path.segments.last().expect("len checked");
    if seg.ident.to_string().starts_with("r#") {
        return;
    }
    let self_args: Option<String> = match &seg.arguments {
        PathArguments::None => None,
        PathArguments::AngleBracketed(ab) => {
            let mut parts = Vec::with_capacity(ab.args.len());
            for a in &ab.args {
                match src(source, a) {
                    Some(t) => parts.push(t),
                    // broken span bookkeeping — skip the whole impl
                    None => return,
                }
            }
            Some(parts.join(", "))
        }
        PathArguments::Parenthesized(_) => return,
    };
    let mut c = Collector {
        source,
        name: seg.ident.to_string(),
        self_args,
        generic_params: impl_type_param_idents(imp),
        edits: Vec::new(),
    };
    for item in &imp.items {
        // A method/const that redeclares an impl-level type-param name would
        // make `G<T>` refer to ITS param, not the impl's — text comparison
        // cannot see through the shadow, so the whole item is skipped.
        if shadows_impl_params(&c.generic_params, item) {
            continue;
        }
        c.visit_impl_item(item);
    }
    edits.append(&mut c.edits);
}

fn impl_type_param_idents(imp: &syn::ItemImpl) -> Vec<String> {
    imp.generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
            _ => None,
        })
        .collect()
}

fn shadows_impl_params(params: &[String], item: &ImplItem) -> bool {
    if params.is_empty() {
        return false;
    }
    let generics = match item {
        ImplItem::Fn(f) => Some(&f.sig.generics),
        ImplItem::Const(c) => Some(&c.generics),
        _ => None,
    };
    generics.is_some_and(|g| {
        g.params.iter().any(|p| match p {
            syn::GenericParam::Type(tp) => params.contains(&tp.ident.to_string()),
            _ => false,
        })
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    /// Type position — the value namespace does not exist here; always safe.
    Type,
    /// Struct literal `S { .. }` — braces force the struct; safe.
    Struct,
    /// Multi-segment path in any position (`S::K`) — `::` forces the type
    /// namespace (a const cannot be a path prefix); safe.
    Qualified,
    /// Bare single-segment path in expression position — ambiguous with a
    /// `const S` / CamelCase `fn S`; skipped.
    ExprBare,
    /// Bare single-segment path in pattern position — ambiguous only with a
    /// `SCREAMING_SNAKE` const pattern; skipped when the ident has no lowercase.
    PatBare,
}

struct Collector<'a> {
    source: &'a str,
    name: String,
    self_args: Option<String>,
    generic_params: Vec<String>,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    // ── suppression guards (see module docs 2-3) ──
    fn visit_type_param_bound(&mut self, _b: &'ast syn::TypeParamBound) {}
    fn visit_item(&mut self, _i: &'ast Item) {}
    fn visit_attribute(&mut self, _a: &'ast syn::Attribute) {}
    fn visit_macro(&mut self, _m: &'ast syn::Macro) {}

    fn visit_type_path(&mut self, tp: &'ast TypePath) {
        self.try_path(&tp.path, Ctx::Type);
        syn::visit::visit_type_path(self, tp);
    }

    fn visit_expr_path(&mut self, ep: &'ast syn::ExprPath) {
        let bare = ep.path.segments.len() == 1 && matches!(ep.path.segments[0].arguments, PathArguments::None);
        self.try_path(&ep.path, if bare { Ctx::ExprBare } else { Ctx::Qualified });
        syn::visit::visit_expr_path(self, ep);
    }

    fn visit_expr_struct(&mut self, es: &'ast syn::ExprStruct) {
        self.try_path(&es.path, Ctx::Struct);
        syn::visit::visit_expr_struct(self, es);
    }

    fn visit_expr_call(&mut self, ec: &'ast syn::ExprCall) {
        if let syn::Expr::Path(ep) = &*ec.func {
            // `S(..)` bare callee could be a CamelCase fn — only qualified
            // callees (`S::new(..)`) are unambiguous.
            let bare = ep.path.segments.len() == 1;
            self.try_path(&ep.path, if bare { Ctx::ExprBare } else { Ctx::Qualified });
        }
        syn::visit::visit_expr_call(self, ec);
    }

    fn visit_pat(&mut self, p: &'ast Pat) {
        if let syn::Pat::Path(pp) = p {
            let bare = pp.path.segments.len() == 1
                && matches!(pp.path.segments[0].arguments, PathArguments::None);
            self.try_path(&pp.path, if bare { Ctx::PatBare } else { Ctx::Qualified });
        }
        syn::visit::visit_pat(self, p);
    }

    fn visit_pat_struct(&mut self, ps: &'ast syn::PatStruct) {
        self.try_path(&ps.path, Ctx::Struct);
        syn::visit::visit_pat_struct(self, ps);
    }

    fn visit_pat_tuple_struct(&mut self, pts: &'ast syn::PatTupleStruct) {
        self.try_path(&pts.path, Ctx::Struct);
        syn::visit::visit_pat_tuple_struct(self, pts);
    }
}

impl Collector<'_> {
    fn try_path(&mut self, p: &Path, ctx: Ctx) {
        if p.leading_colon.is_some() {
            return;
        }
        let first = &p.segments[0];
        if first.ident != self.name {
            return;
        }
        match (&first.arguments, &self.self_args) {
            // Bare path under a NON-generic impl (`impl V`) — always the
            // type.
            (PathArguments::None, None) => {}
            // Bare path under a generic-identity impl (`impl<T> G<T>`,
            // `impl G<u32>`): a bare `G` infers its arguments from CONTEXT
            // (`G<String>` inside `fn s() -> G<String>`), which syn cannot
            // see — never fire (the `generic_impl_skips_different_args`
            // fixture is exactly this hazard). The angle-bracketed/Paren
            // skip arms below share this `return`.
            (PathArguments::None, Some(_))
            | (PathArguments::AngleBracketed(_), None)
            | (PathArguments::Parenthesized(_), _) => return,
            // `G<..>` with the impl's exact argument text
            (PathArguments::AngleBracketed(ab), Some(args_text)) => {
                let mut parts = Vec::with_capacity(ab.args.len());
                for a in &ab.args {
                    match src(self.source, a) {
                        Some(t) => parts.push(t),
                        None => return,
                    }
                }
                if parts.join(", ") != *args_text {
                    return;
                }
            }
        }
        match ctx {
            Ctx::ExprBare => return,
            Ctx::PatBare => {
                // `const FOO` patterns are SCREAMING_SNAKE by convention; a
                // CamelCase ident (has a lowercase letter) is the type.
                let ident = first.ident.to_string();
                if !ident.chars().any(char::is_lowercase) {
                    return;
                }
            }
            Ctx::Type | Ctx::Struct | Ctx::Qualified => {}
        }
        self.edits.push(edit(&first.ident, "Self".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn return_type_and_struct_literal_fire() {
        let out = propose_fix(
            "use_self",
            "struct V { n: u32 }\nimpl V { fn new() -> V { V { n: 1 } } }",
        )
        .expect("pattern matched");
        assert_eq!(out, "struct V { n: u32 }\nimpl V { fn new() -> Self { Self { n: 1 } } }");
    }

    #[test]
    fn associated_path_fires() {
        let out = propose_fix(
            "use_self",
            "struct S;\nimpl S { const K: u8 = 1; fn make() -> S { S::make() } }",
        )
        .expect("pattern matched");
        assert!(out.contains("fn make() -> Self"), "{out}");
        assert!(out.contains("Self::make()"), "{out}");
        assert!(!out.contains("S::make()"), "{out}");
    }

    #[test]
    fn generic_impl_fires_when_args_match() {
        let out = propose_fix(
            "use_self",
            "struct G<T> { t: Option<T> }\nimpl<T> G<T> { fn new() -> G<T> { G { t: None } } }",
        )
        .expect("pattern matched");
        // The return type's `G<T>` matches the impl head exactly → Self.
        // The bare struct LITERAL stays: a bare `G` under a generic impl
        // infers its arguments from context, which syn cannot see (the
        // `generic_impl_skips_different_args` hazard).
        assert!(out.contains("fn new() -> Self"), "{out}");
        assert!(out.contains("G { t: None }"), "{out}");
    }

    #[test]
    fn generic_impl_skips_different_args() {
        let out = propose_fix(
            "use_self",
            "struct G<T> { t: Option<T> }\nimpl<T> G<T> { fn s() -> G<String> { G { t: None } } }",
        );
        assert!(out.is_none(), "G<String> must not become Self: {out:?}");
    }

    #[test]
    fn concrete_impl_args_match() {
        let out = propose_fix(
            "use_self",
            "struct G<T> { t: Option<T> }\nimpl G<u32> { fn f() -> G<u32> { G { t: None } } }",
        )
        .expect("pattern matched");
        assert!(out.contains("fn f() -> Self"), "{out}");
    }

    #[test]
    fn trait_impl_untouched() {
        let out = propose_fix(
            "use_self",
            "struct S;\ntrait T {}\nimpl T for S { fn f() -> S { S } }\nimpl S { fn g() -> S { S } }",
        )
        .expect("pattern matched");
        // Only the inherent impl's references are rewritten; the bare
        // unit-struct `S` EXPRESSION stays (the ExprBare ambiguity guard).
        assert!(out.contains("impl T for S { fn f() -> S { S } }"), "{out}");
        assert!(out.contains("impl S { fn g() -> Self { S } }"), "{out}");
    }

    #[test]
    fn nested_fn_untouched() {
        // A fn declared inside a method cannot see `Self`.
        let out = propose_fix(
            "use_self",
            "struct S;\nimpl S { fn f() -> S { fn h() -> S { S } h() } }",
        )
        .expect("pattern matched");
        assert!(out.contains("fn h() -> S { S }"), "nested fn rewritten: {out}");
        assert!(out.contains("fn f() -> Self"), "{out}");
    }

    #[test]
    fn bounds_and_attrs_untouched() {
        let out = propose_fix(
            "use_self",
            "struct S;\ntrait Tr {}\nimpl S where S: Tr { fn f() -> S { S } }",
        )
        .expect("pattern matched");
        // The impl-level `where S: Tr` bound must survive (bounds are never
        // entered); the return-type `S` still fires.
        assert!(out.contains("where S: Tr"), "{out}");
        assert!(out.contains("fn f() -> Self"), "{out}");
    }

    #[test]
    fn use_item_inside_nested_mod_untouched() {
        // A `mod` is legal inside a FN BODY (not inside an impl) — the
        // visit_item suppression keeps its contents out of reach.
        let out = propose_fix(
            "use_self",
            "struct S;\nimpl S { fn g() -> S { mod inner { pub fn f() -> u8 { 1 } } inner::f() as u8 } }",
        )
        .expect("pattern matched");
        assert!(out.contains("mod inner"), "{out}");
        assert!(out.contains("fn g() -> Self"), "{out}");
        assert!(out.contains("inner::f()"), "{out}");
    }

    #[test]
    fn qualified_inner_segment_untouched() {
        // `other::S` must not become `other::Self` (or bare `Self`).
        let out = propose_fix(
            "use_self",
            "mod other { pub struct S; }\nstruct S;\nimpl S { fn f() -> other::S { other::S } }",
        );
        if let Some(out) = out {
            assert!(out.contains("other::S"), "qualified path rewritten: {out}");
        }
    }

    #[test]
    fn qualified_callee_and_literal_fire() {
        let out = propose_fix(
            "use_self",
            "struct S { n: u8 }\nimpl S { fn new(n: u8) -> S { S { n } } fn go() -> S { S::new(1) } }",
        )
        .expect("pattern matched");
        assert!(out.contains("Self::new(1)"), "{out}");
        assert!(out.contains("-> Self { Self { n } }"), "{out}");
    }

    #[test]
    fn method_param_shadowing_impl_generic_skips() {
        // The method redeclares `T` — `G<T>` refers to ITS `T`, not the impl's.
        let out = propose_fix(
            "use_self",
            "struct G<T> { t: Option<T> }\nimpl<T> G<T> { fn f<T>(x: G<T>) -> G<T> { x } }",
        );
        assert!(out.is_none(), "shadowed param must not fire: {out:?}");
    }

    #[test]
    fn option_box_inner_types_fire() {
        let out = propose_fix(
            "use_self",
            "struct S;\nimpl S { fn f() -> Option<Box<S>> { None } }",
        )
        .expect("pattern matched");
        assert!(out.contains("Option<Box<Self>>"), "{out}");
    }
}
