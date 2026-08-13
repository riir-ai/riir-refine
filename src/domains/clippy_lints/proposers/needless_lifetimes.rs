//! `clippy::needless_lifetimes` — drop explicit `<'a>` lifetimes from fn
//! signatures where elision rules apply.

use syn::{File, ItemFn};

pub(crate) fn rewrite(file: &mut File) -> bool {
    let mut changed = false;
    for item in &mut file.items {
        if let syn::Item::Fn(fn_item) = item {
            changed |= rewrite_fn(fn_item);
        }
    }
    changed
}

fn rewrite_fn(fn_item: &mut ItemFn) -> bool {
    // Collect the distinct lifetime names declared on the fn.
    let lt_names: Vec<String> = fn_item
        .sig
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Lifetime(lp) => Some(lp.lifetime.ident.to_string()),
            _ => None,
        })
        .collect();

    // Conservative: only handle the single-lifetime elision case.
    if lt_names.len() != 1 {
        return false;
    }
    let target = &lt_names[0];

    // Drop lifetime generic params. Punctuated doesn't have `retain` in syn 2.x;
    // rebuild with a filter.
    let before = fn_item.sig.generics.params.len();
    let kept: Vec<syn::GenericParam> = fn_item
        .sig
        .generics
        .params
        .iter()
        .filter(|p| !matches!(p, syn::GenericParam::Lifetime(_)))
        .cloned()
        .collect();
    if kept.len() == before {
        return false; // no lifetime params despite lt_names being non-empty
    }
    fn_item.sig.generics.params = kept.into_iter().collect();

    for input in &mut fn_item.sig.inputs {
        strip_lifetime_in_type(input, target);
    }
    if let syn::ReturnType::Type(_, ty) = &mut fn_item.sig.output {
        strip_lifetime_in_type_ref(ty, target);
    }

    true
}

fn strip_lifetime_in_type(arg: &mut syn::FnArg, target: &str) {
    if let syn::FnArg::Typed(pat) = arg {
        strip_lifetime_in_type_ref(&mut pat.ty, target);
    }
}

fn strip_lifetime_in_type_ref(ty: &mut syn::Type, target: &str) {
    match ty {
        syn::Type::Reference(r) => {
            if let Some(lt) = &r.lifetime {
                if lt.ident == target {
                    r.lifetime = None;
                }
            }
            strip_lifetime_in_type_ref(&mut r.elem, target);
        }
        syn::Type::Paren(p) => strip_lifetime_in_type_ref(&mut p.elem, target),
        syn::Type::Group(g) => strip_lifetime_in_type_ref(&mut g.elem, target),
        syn::Type::Slice(s) => strip_lifetime_in_type_ref(&mut s.elem, target),
        syn::Type::Array(a) => strip_lifetime_in_type_ref(&mut a.elem, target),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(super::super::quote_file(&file))
        } else {
            None
        }
    }

    #[test]
    fn elides_single_lifetime() {
        let input = "fn head<'a>(v: &'a Vec<u8>) -> &'a u8 { &v[0] }";
        let out = propose(input).expect("matched");
        assert!(!out.contains("< 'a >"), "lifetime not stripped: {out}");
        assert!(
            !out.contains('\''),
            "any lifetime quote should be gone: {out}"
        );
    }

    #[test]
    fn no_lifetime_unchanged() {
        let input = "fn head(v: &Vec<u8>) -> &u8 { &v[0] }";
        let out = propose(input);
        assert!(out.is_none(), "should not have changed: {out:?}");
    }

    #[test]
    fn multiple_lifetimes_unchanged() {
        let input = "fn merge<'a, 'b>(x: &'a u8, y: &'b u8) -> &'a u8 { x }";
        let out = propose(input);
        assert!(out.is_none(), "two-lifetime should not be touched: {out:?}");
    }
}
