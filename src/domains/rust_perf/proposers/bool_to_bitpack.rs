//! `bool-to-bitpack` — `Vec<bool>` set-bit count → packed `Vec<u64>` + `count_ones()`.
//!
//! Pattern: a fn with a `flags: &[bool]` parameter whose body is the single
//! tail expression `flags.iter().filter(<closure>).count()`. The rewrite
//! changes the parameter to `&[u64]` (packed bits) + the body to
//! `flags.iter().map(|w| w.count_ones() as usize).sum()` — one POPCNT per
//! 64 bools instead of a per-byte comparison (Bench 013: ParetoDominant
//! 8-12.5x).
//!
//! **Signature-visible change** (documented per the module caveat): callers
//! must pass packed bits (`pack_bools` at the boundary). The param name is
//! deliberately preserved (no capture-avoiding rename needed); `&Vec<bool>`
//! params + non-tail count forms are out of scope.

use syn::{Expr, File, FnArg, Pat, Stmt, Type};

use super::for_each_fn_mut;

pub(crate) fn rewrite(file: &mut File) -> bool {
    let mut changed = false;
    for_each_fn_mut(file, |sig, block| {
        changed |= rewrite_fn(sig, block);
    });
    changed
}

fn rewrite_fn(sig: &mut syn::Signature, block: &mut syn::Block) -> bool {
    let Some(param) = bool_slice_param(sig) else { return false };

    // Body must be exactly the tail chain `<param>.iter().filter(1 arg).count()`.
    if block.stmts.len() != 1 {
        return false;
    }
    let Stmt::Expr(expr, None) = &block.stmts[0] else { return false };
    let Expr::MethodCall(count_mc) = expr else { return false };
    if count_mc.method != "count" || !count_mc.args.is_empty() {
        return false;
    }
    let Expr::MethodCall(filter_mc) = &*count_mc.receiver else { return false };
    if filter_mc.method != "filter" || filter_mc.args.len() != 1 {
        return false;
    }
    let Expr::MethodCall(iter_mc) = &*filter_mc.receiver else { return false };
    if iter_mc.method != "iter" || !iter_mc.args.is_empty() {
        return false;
    }
    let Expr::Path(recv) = &*iter_mc.receiver else { return false };
    if !recv.path.is_ident(&param) {
        return false;
    }

    // Rewrite the param type `&[bool]` → `&[u64]`.
    for arg in &mut sig.inputs {
        if let FnArg::Typed(pt) = arg {
            if matches!(&*pt.pat, Pat::Ident(pi) if pi.ident == param) {
                *pt.ty = syn::parse_quote! { &[u64] };
            }
        }
    }

    // Rewrite the tail to the POPCNT form.
    let new_tail: Expr = syn::parse_quote! {
        #param.iter().map(|w| w.count_ones() as usize).sum()
    };
    block.stmts[0] = Stmt::Expr(new_tail, None);
    true
}

/// Find a `name: &[bool]` (shared-ref slice of bool) parameter.
fn bool_slice_param(sig: &syn::Signature) -> Option<syn::Ident> {
    for arg in &sig.inputs {
        let FnArg::Typed(pt) = arg else { continue };
        let Pat::Ident(pi) = &*pt.pat else { continue };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        if is_ref_slice_bool(&pt.ty) {
            return Some(pi.ident.clone());
        }
    }
    None
}

fn is_ref_slice_bool(ty: &Type) -> bool {
    let Type::Reference(tr) = ty else { return false };
    if tr.mutability.is_some() {
        return false;
    }
    let Type::Slice(ts) = &*tr.elem else { return false };
    let Type::Path(tp) = &*ts.elem else { return false };
    tp.qself.is_none() && tp.path.is_ident("bool")
}

#[cfg(test)]
mod tests {
    use super::super::quote_file;
    use super::*;

    fn propose(input: &str) -> Option<String> {
        let mut file = syn::parse_file(input).ok()?;
        if rewrite(&mut file) {
            Some(quote_file(&file))
        } else {
            None
        }
    }

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose(
            "fn count_true(flags: &[bool]) -> usize {\n    \
             flags.iter().filter(|&&b| b).count()\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("flags: &[u64]")), "{out}");
        assert!(
            squash(&out).contains(&squash("flags.iter().map(|w| w.count_ones() as usize).sum()")),
            "{out}"
        );
    }

    #[test]
    fn param_name_preserved() {
        // A param named like the closure var exercises the shadowing-safety
        // choice (no rename): `w.iter().map(|w| ..)` re-parses + stays
        // correct (the closure shadows within its own body only).
        let out = propose(
            "fn count(w: &[bool]) -> usize {\n    w.iter().filter(|&&b| b).count()\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains(&squash("w: &[u64]")), "{out}");
        let reparsed = syn::parse_file(&out).expect("output must re-parse");
        assert!(!reparsed.items.is_empty());
    }

    #[test]
    fn filter_variants_accepted() {
        // The closure shape varies (`|&b| *b`, `|b| **b`); the chain shape
        // is what we match, not the closure body.
        let out = propose(
            "fn f(flags: &[bool]) -> usize {\n    flags.iter().filter(|&b| *b).count()\n}",
        )
        .expect("pattern matched");
        assert!(squash(&out).contains("count_ones"), "{out}");
    }

    #[test]
    fn vec_param_unchanged() {
        // `&Vec<bool>` is a different type — out of scope.
        assert!(
            propose("fn f(flags: &Vec<bool>) -> usize {\n    \
                     flags.iter().filter(|&&b| b).count()\n}")
                .is_none()
        );
    }

    #[test]
    fn mut_slice_unchanged() {
        assert!(
            propose("fn f(flags: &mut [bool]) -> usize {\n    \
                     flags.iter().filter(|&&b| b).count()\n}")
                .is_none()
        );
    }

    #[test]
    fn extra_body_stmts_unchanged() {
        assert!(
            propose("fn f(flags: &[bool]) -> usize {\n    let n = \
                     flags.iter().filter(|&&b| b).count();\n    n + 1\n}")
                .is_none()
        );
    }

    #[test]
    fn different_method_chain_unchanged() {
        assert!(
            propose("fn f(flags: &[bool]) -> usize {\n    \
                     flags.iter().position(|&&b| b).unwrap_or(0)\n}")
                .is_none()
        );
    }
}
