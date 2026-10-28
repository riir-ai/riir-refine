//! `bool-to-bitpack` — `Vec<bool>` set-bit count → packed `Vec<u64>` + `count_ones()`.
//!
//! Pattern: a fn with a `flags: &[bool]` parameter whose body is the single
//! tail expression `flags.iter().filter(<closure>).count()`. The rewrite
//! changes the parameter to `&[u64]` (packed bits) + the body to
//! `flags.iter().map(|w| w.count_ones() as usize).sum()` — one POPCNT per
//! 64 bools instead of a per-byte comparison (Bench 013: `ParetoDominant`
//! 8-12.5x).
//!
//! **Signature-visible change** (documented per the module caveat): callers
//! must pass packed bits (`pack_bools` at the boundary). **Caller gate
//! (2026-08-18)**: fires ONLY when the fn ident occurs exactly once in the
//! file — a referenced fn (live caller, fn-as-value, macro arg) is refused,
//! because a span-local edit cannot adapt the caller's `&[bool]` argument
//! (the `action_hoare.rs` E0308 lesson). The param name is
//! deliberately preserved (no capture-avoiding rename needed); `&Vec<bool>`
//! params + non-tail count forms are out of scope.
//!
//! Span-preserving: two edits — the param type span → `&[u64]`, and the
//! tail expr span → `<param>.iter().map(|w| w.count_ones() as usize).sum()`.

use syn::{Expr, File, FnArg, Pat, Stmt, Type};

use super::{SourceEdit, edit};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    super::for_each_fn(file, |sig, block| {
        fn_edits(source, sig, block, &mut edits);
    });
    edits
}

/// Push the fn's edits (0 or 2) if it matches.
fn fn_edits(source: &str, sig: &syn::Signature, block: &syn::Block, edits: &mut Vec<SourceEdit>) {
    // Caller gate (the action_hoare.rs E0308 lesson): the rewrite changes the
    // param type — any other reference to this fn in the file (call site,
    // fn-as-value, macro arg) is somewhere a span-local edit cannot adapt.
    if super::count_ident_occurrences(source, &sig.ident.to_string()) > 1 {
        return;
    }
    let Some((param, param_ty)) = bool_slice_param(sig) else {
        return;
    };

    // Body must be exactly the tail chain `<param>.iter().filter(1 arg).count()`.
    if block.stmts.len() != 1 {
        return;
    }
    let Stmt::Expr(expr, None) = &block.stmts[0] else {
        return;
    };
    let Expr::MethodCall(count_mc) = expr else {
        return;
    };
    if count_mc.method != "count" || !count_mc.args.is_empty() {
        return;
    }
    let Expr::MethodCall(filter_mc) = &*count_mc.receiver else {
        return;
    };
    if filter_mc.method != "filter" || filter_mc.args.len() != 1 {
        return;
    }
    let Expr::MethodCall(iter_mc) = &*filter_mc.receiver else {
        return;
    };
    if iter_mc.method != "iter" || !iter_mc.args.is_empty() {
        return;
    }
    let Expr::Path(recv) = &*iter_mc.receiver else {
        return;
    };
    if !recv.path.is_ident(&param) {
        return;
    }

    let tail_replacement = format!("{param}.iter().map(|w| w.count_ones() as usize).sum()");
    edits.push(edit(&param_ty, "&[u64]".to_string()));
    edits.push(edit(expr, tail_replacement));
}

/// Find a `name: &[bool]` (shared-ref slice of bool) parameter — returns
/// (param ident, the type node) so the caller can edit the type's span.
fn bool_slice_param(sig: &syn::Signature) -> Option<(syn::Ident, Type)> {
    for arg in &sig.inputs {
        let FnArg::Typed(pt) = arg else { continue };
        let Pat::Ident(pi) = &*pt.pat else { continue };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        if is_ref_slice_bool(&pt.ty) {
            return Some((pi.ident.clone(), (*pt.ty).clone()));
        }
    }
    None
}

fn is_ref_slice_bool(ty: &Type) -> bool {
    let Type::Reference(tr) = ty else {
        return false;
    };
    if tr.mutability.is_some() {
        return false;
    }
    let Type::Slice(ts) = &*tr.elem else {
        return false;
    };
    let Type::Path(tp) = &*ts.elem else {
        return false;
    };
    tp.qself.is_none() && tp.path.is_ident("bool")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn corpus_fixture_rewrites() {
        let out = propose_fix(
            "bool-to-bitpack",
            "fn count_true(flags: &[bool]) -> usize {\n    \
             flags.iter().filter(|&&b| b).count()\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("flags: &[u64]"), "{out}");
        assert!(
            squash(&out).contains(&squash(
                "flags.iter().map(|w| w.count_ones() as usize).sum()"
            )),
            "{out}"
        );
    }

    #[test]
    fn param_name_preserved() {
        // A param named like the closure var exercises the shadowing-safety
        // choice (no rename): `w.iter().map(|w| ..)` re-parses + stays
        // correct (the closure shadows within its own body only). The fn is
        // named `count_bits` (NOT `count`) so the ident occurs exactly once
        // — `.count()` in the body would trip the caller gate.
        let out = propose_fix(
            "bool-to-bitpack",
            "fn count_bits(w: &[bool]) -> usize {\n    w.iter().filter(|&&b| b).count()\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("w: &[u64]"), "{out}");
        let reparsed = syn::parse_file(&out).expect("output must re-parse");
        assert_ne!(reparsed.items, []);
    }

    #[test]
    fn live_caller_blocks_rewrite() {
        // The action_hoare.rs E0308 shape: a private counting helper WITH a
        // live same-file caller must NOT be rewritten — the caller still
        // passes `&[bool]` and a span-local edit cannot adapt it.
        assert!(
            propose_fix(
                "bool-to-bitpack",
                "fn count_satisfied(satisfied: &[bool]) -> usize {\n    \
                 satisfied.iter().filter(|&&s| s).count()\n}\n\nfn reward(pre: &[bool]) -> f32 {\n    count_satisfied(pre) as f32 * 0.5\n}",
            )
            .is_none()
        );
    }

    #[test]
    fn filter_variants_accepted() {
        // The closure shape varies (`|&b| *b`, `|b| **b`); the chain shape
        // is what we match, not the closure body.
        let out = propose_fix(
            "bool-to-bitpack",
            "fn f(flags: &[bool]) -> usize {\n    flags.iter().filter(|&b| *b).count()\n}",
        )
        .expect("pattern matched");
        assert!(out.contains("count_ones"), "{out}");
    }

    #[test]
    fn vec_param_unchanged() {
        // `&Vec<bool>` is a different type — out of scope.
        assert!(
            propose_fix(
                "bool-to-bitpack",
                "fn f(flags: &Vec<bool>) -> usize {\n    \
                 flags.iter().filter(|&&b| b).count()\n}"
            )
            .is_none()
        );
    }

    #[test]
    fn mut_slice_unchanged() {
        assert!(
            propose_fix(
                "bool-to-bitpack",
                "fn f(flags: &mut [bool]) -> usize {\n    \
                 flags.iter().filter(|&&b| b).count()\n}"
            )
            .is_none()
        );
    }

    #[test]
    fn extra_body_stmts_unchanged() {
        assert!(
            propose_fix(
                "bool-to-bitpack",
                "fn f(flags: &[bool]) -> usize {\n    let n = \
                 flags.iter().filter(|&&b| b).count();\n    n + 1\n}"
            )
            .is_none()
        );
    }

    #[test]
    fn different_method_chain_unchanged() {
        assert!(
            propose_fix(
                "bool-to-bitpack",
                "fn f(flags: &[bool]) -> usize {\n    \
                 flags.iter().position(|&&b| b).unwrap_or(0)\n}"
            )
            .is_none()
        );
    }
}
