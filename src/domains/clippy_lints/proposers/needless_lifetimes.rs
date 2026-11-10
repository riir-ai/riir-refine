//! `clippy::needless_lifetimes` — drop explicit `<'a>` lifetimes from fn
//! signatures where elision rules apply.
//!
//! Span-preserving (T6.4): the generics list is re-emitted from the kept
//! params' ORIGINAL source text; each `'a` annotation is deleted together
//! with ONE following space (`&'a u8` → `&u8`).
//!
//! ## Detection (tightened for `--fix` eligibility)
//!
//! Old behavior fired on "exactly one declared lifetime" alone — which is
//! NOT sufficient for elision soundness: `fn pick<'a>(x: &'a u8, y: &'a u8)
//! -> &'a u8` has one declared lifetime but TWO input reference slots, so
//! the elided output lifetime would be ambiguous (the rewrite used to
//! produce non-compiling code; `self_evolve`'s cargo-clippy check caught it,
//! but `--fix` has no type checker).
//!
//! The tightened rule: exactly one declared lifetime, and exactly ONE input
//! reference slot carrying it (output slots may carry it any number of
//! times — elision assigns the single input lifetime to all of them).
//! Fns with a `where` clause or bounds on the lifetime param are skipped
//! (both may reference `'a` in ways the splice cannot see).

use super::{SourceEdit, edit, for_each_fn, src, swallow_following_space};
use syn::{File, GenericParam, Signature, Type};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |sig, _block| {
        collect_fn_edits(source, sig, &mut edits);
    });
    edits
}

fn collect_fn_edits(source: &str, sig: &Signature, edits: &mut Vec<SourceEdit>) {
    // Collect the distinct lifetime names declared on the fn.
    let lt_names: Vec<String> = sig
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Lifetime(lp) => Some(lp.lifetime.ident.to_string()),
            _ => None,
        })
        .collect();

    // Conservative: only handle the single-lifetime elision case.
    if lt_names.len() != 1 {
        return;
    }
    let target = &lt_names[0];

    // A `where` clause may reference the lifetime (`where T: 'a`) — skip.
    if sig.generics.where_clause.is_some() {
        return;
    }
    // Bounds on the lifetime param itself (`<'a: 'static>`) — skip.
    if sig
        .generics
        .params
        .iter()
        .any(|p| matches!(p, GenericParam::Lifetime(lp) if !lp.bounds.is_empty()))
    {
        return;
    }
    // A by-ref receiver (`&self` / `&'a self`) binds elided output lifetimes
    // to SELF — a subtly different signature semantics. Skip; clippy's own
    // type-aware pass handles receivers.
    if matches!(sig.inputs.first(), Some(syn::FnArg::Receiver(r)) if r.reference.is_some()) {
        return;
    }

    // ── Elision soundness (one precise walker) ──
    // Exactly ONE INPUT reference slot in TOTAL (elided `&T` inputs are
    // input lifetimes too — `fn f<'a>(b: &'a B, p: &P)` has two, so the
    // elided output cannot resolve), that slot carries the target, and the
    // target appears ONLY in strippable `&`-annotation positions (see
    // [`elision_walk`] for the full violation list). Output refs don't
    // count toward the slot total — elision assigns them the single input.
    let mut slots = RefSlots {
        total: 0,
        target: 0,
    };
    let mut violating = false;
    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pat) = arg {
            violating |= elision_walk(&pat.ty, target, &mut slots);
        }
    }
    if let syn::ReturnType::Type(_, ty) = &sig.output {
        let mut out_slots = RefSlots {
            total: 0,
            target: 0,
        };
        violating |= elision_walk(ty, target, &mut out_slots);
    }
    if violating || slots.total != 1 || slots.target != 1 {
        return;
    }

    // ── Edit 1: the generics list ──
    let kept: Vec<&GenericParam> = sig
        .generics
        .params
        .iter()
        .filter(|p| !matches!(p, GenericParam::Lifetime(_)))
        .collect();
    if kept.is_empty() {
        // `<'a>` disappears entirely.
        edits.push(edit(&sig.generics, String::new()));
    } else {
        // `<'a, T>` → `<T>` (kept params' ORIGINAL text, comma-joined).
        let texts: Vec<String> = kept.iter().filter_map(|p| src(source, p)).collect();
        if texts.len() != kept.len() {
            return; // span bookkeeping broke — skip this fn entirely
        }
        edits.push(edit(&sig.generics, format!("<{}>", texts.join(", "))));
    }

    // ── Edit 2+: lifetime annotations in inputs + output ──
    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pat) = arg {
            strip_lifetime_in_type(source, &pat.ty, target, edits);
        }
    }
    if let syn::ReturnType::Type(_, ty) = &sig.output {
        strip_lifetime_in_type(source, ty, target, edits);
    }
}

/// The elision walker — one precise traversal covering BOTH the reference
/// slot count AND the outside-use check. It walks exactly the shape the
/// stripper recurses (`&`-annotations + Paren/Group/Slice/Array + path
/// generic TYPE args); the target lifetime appearing ANYWHERE else is a
/// violation (it would dangle after the generics are dropped):
///
/// - path generic lifetime args (`Index<'a>`),
/// - trait-object / impl-trait lifetime bounds (`dyn T + 'a`),
/// - lifetimes inside trait-bound arguments (`FnMut(&'a S)` — elision there
///   means higher-ranked, a DIFFERENT signature),
/// - bare-fn types, macros, or any other type carrying the lifetime.
struct RefSlots {
    total: usize,
    target: usize,
}

fn elision_walk(ty: &Type, target: &str, c: &mut RefSlots) -> bool {
    match ty {
        Type::Reference(r) => {
            c.total += 1;
            if let Some(lt) = &r.lifetime {
                if lt.ident != target {
                    return true; // a different explicit lifetime — unexpected; skip
                }
                c.target += 1;
            }
            elision_walk(&r.elem, target, c)
        }
        Type::Paren(p) => elision_walk(&p.elem, target, c),
        Type::Group(g) => elision_walk(&g.elem, target, c),
        Type::Slice(s) => elision_walk(&s.elem, target, c),
        Type::Array(a) => elision_walk(&a.elem, target, c),
        Type::Path(p) => p
            .path
            .segments
            .iter()
            .any(|seg| path_args_violate(&seg.arguments, target, c)),
        Type::TraitObject(t) => bounds_violate(&t.bounds, target, c),
        Type::ImplTrait(t) => bounds_violate(&t.bounds, target, c),
        // Bare-fn types (`fn(&'a u8)`), macro types, verbatim — elision
        // semantics differ or are opaque; any target mention is a skip.
        _ => tokens_mention(ty, target),
    }
}

fn path_args_violate(args: &syn::PathArguments, target: &str, c: &mut RefSlots) -> bool {
    match args {
        syn::PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| match arg {
            syn::GenericArgument::Type(t) => elision_walk(t, target, c),
            syn::GenericArgument::Lifetime(lt) => lt.ident == target,
            syn::GenericArgument::Constraint(b) => bounds_violate(&b.bounds, target, c),
            _ => false,
        }),
        // `Fn(&'a S) -> &'a T` — parenthesized trait arguments.
        syn::PathArguments::Parenthesized(args) => {
            args.inputs.iter().any(|t| elision_walk(t, target, c))
                || matches!(&args.output, syn::ReturnType::Type(_, t) if elision_walk(t, target, c))
        }
        syn::PathArguments::None => false,
    }
}

fn bounds_violate(
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
    target: &str,
    c: &mut RefSlots,
) -> bool {
    bounds.iter().any(|b| match b {
        syn::TypeParamBound::Lifetime(lt) => lt.ident == target,
        syn::TypeParamBound::Trait(tb) => tb
            .path
            .segments
            .iter()
            .any(|seg| path_args_violate(&seg.arguments, target, c)),
        _ => true, // Verbatim — opaque; conservatively skip
    })
}

/// Token-level mentions check for the opaque catch-all arm. May
/// over-match (`'a` prefix of `'abc`) — over-matching only skips a rewrite.
fn tokens_mention<T: quote::ToTokens>(node: &T, target: &str) -> bool {
    let text = node.to_token_stream().to_string();
    text.contains(&format!("'{target} "))
        || text.contains(&format!("'{target},"))
        || text.contains(&format!("'{target}>"))
}

fn strip_lifetime_in_type(source: &str, ty: &Type, target: &str, edits: &mut Vec<SourceEdit>) {
    match ty {
        Type::Reference(r) => {
            if let Some(lt) = &r.lifetime {
                if lt.ident == target {
                    // Delete `'a ` (lifetime + one following space).
                    edits.push(swallow_following_space(source, edit(lt, String::new())));
                }
            }
            strip_lifetime_in_type(source, &r.elem, target, edits);
        }
        Type::Paren(p) => strip_lifetime_in_type(source, &p.elem, target, edits),
        Type::Group(g) => strip_lifetime_in_type(source, &g.elem, target, edits),
        Type::Slice(s) => strip_lifetime_in_type(source, &s.elem, target, edits),
        Type::Array(a) => strip_lifetime_in_type(source, &a.elem, target, edits),
        // `Option<&'a T>` — strip inside generic arguments too (leaving a
        // dangling `'a` here while dropping the generics would not compile).
        Type::Path(p) => {
            for seg in &p.path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            strip_lifetime_in_type(source, t, target, edits);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn elides_single_lifetime() {
        let input = "fn head<'a>(v: &'a Vec<u8>) -> &'a u8 { &v[0] }";
        let out = propose_fix("needless_lifetimes", input).expect("matched");
        assert_eq!(
            out, "fn head(v: &Vec<u8>) -> &u8 { &v[0] }",
            "span-preserving elision: {out}"
        );
        assert!(
            !out.contains('\''),
            "any lifetime quote should be gone: {out}"
        );
    }

    #[test]
    fn elides_with_mut_ref() {
        let input = "fn fill<'a>(v: &'a mut Vec<u8>) { v.push(0) }";
        let out = propose_fix("needless_lifetimes", input).expect("matched");
        assert!(out.contains("&mut Vec<u8>"), "{out}");
        assert!(!out.contains('\''), "{out}");
    }

    #[test]
    fn keeps_type_params() {
        // `<'a, T>` → `<T>`; the kept param's original text is reused.
        let input = "fn get<'a, T>(v: &'a Vec<T>) -> Option<&'a T> { v.first() }";
        let out = propose_fix("needless_lifetimes", input).expect("matched");
        assert!(out.contains("fn get<T>("), "{out}");
        assert!(
            out.contains("-> Option<&T>"),
            "output lifetime not stripped: {out}"
        );
        assert!(!out.contains('\''), "{out}");
    }

    #[test]
    fn where_clause_unchanged() {
        // `where T: 'a` references the lifetime — not elidable by splice.
        let input = "fn get<'a, T>(v: &'a Vec<T>) -> Option<&'a T> where T: Clone { v.first() }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(
            out.is_none(),
            "where-clause fn must not be touched: {out:?}"
        );
    }

    #[test]
    fn no_lifetime_unchanged() {
        let input = "fn head(v: &Vec<u8>) -> &u8 { &v[0] }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(out.is_none(), "should not have changed: {out:?}");
    }

    #[test]
    fn multiple_lifetimes_unchanged() {
        let input = "fn merge<'a, 'b>(x: &'a u8, y: &'b u8) -> &'a u8 { x }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(out.is_none(), "two-lifetime should not be touched: {out:?}");
    }

    /// The tightened rule: one declared lifetime carried by TWO input slots
    /// is NOT elidable (the output lifetime would be ambiguous). The old
    /// mutation-based proposer fired here and produced non-compiling code.
    #[test]
    fn two_input_slots_unchanged() {
        let input = "fn pick<'a>(x: &'a [u8], y: &'a [u8]) -> &'a u8 { x[0] }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(
            out.is_none(),
            "ambiguous-elision fn must not be touched: {out:?}"
        );
    }

    #[test]
    fn nested_input_refs_unchanged() {
        // `&'a &'a u8` = two input slots — elision output would be ambiguous.
        let input = "fn deref<'a>(r: &'a &'a u8) -> &'a u8 { *r }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(
            out.is_none(),
            "nested-ref input must not be touched: {out:?}"
        );
    }

    /// Elided `&` inputs are input lifetimes too: one explicit `'a` slot +
    /// two elided slots leaves the elided output unresolvable. Found by the
    /// real-file review pass (this repo's own `analyze_index_usage`).
    #[test]
    fn elided_extra_inputs_unchanged() {
        let input = "fn analyze<'a>(b: &'a Block, p: &Path, i: &Ident) -> Index<'a> { f(b, p, i) }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(
            out.is_none(),
            "elided-extra-input fn must not be touched: {out:?}"
        );
    }

    #[test]
    fn output_generic_lifetime_arg_unchanged() {
        // `Index<'a>` uses the lifetime as a generic arg — not an annotation;
        // dropping `<'a>` would dangle it even with a single input slot.
        let input = "fn idx<'a>(v: &'a Vec<u8>) -> Index<'a> { Index::new(v) }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(
            out.is_none(),
            "generic-lifetime output must not be touched: {out:?}"
        );
    }

    #[test]
    fn trait_object_bound_unchanged() {
        // `dyn Tr + 'a` — the bound outlives via the param; not elidable.
        let input = "fn borrow<'a>(v: &'a Vec<u8>) -> Box<dyn Tr + 'a> { Box::new(v[0]) }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(
            out.is_none(),
            "trait-object bound must not be touched: {out:?}"
        );
    }

    #[test]
    fn impl_trait_bound_unchanged() {
        // `impl Iterator<...> + 'a` — found by the real-file review pass
        // (this repo's own `lookup<'a>(&'a self, ..)` — also a receiver case).
        let input = "fn iter<'a>(v: &'a Vec<u8>) -> impl Iterator<Item = &'a u8> + 'a { v.iter() }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(
            out.is_none(),
            "impl-trait bound must not be touched: {out:?}"
        );
    }

    #[test]
    fn by_ref_receiver_unchanged() {
        // Elision with a receiver binds output to SELF — different
        // signature semantics; conservatively out of scope.
        let input = "struct S { v: Vec<u8> }
impl S { fn first<'a>(&'a self) -> &'a u8 { &self.v[0] } }";
        let out = propose_fix("needless_lifetimes", input);
        assert!(out.is_none(), "receiver fn must not be touched: {out:?}");
    }
}
