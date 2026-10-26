//! `ptr-deref-null-check-first` — null-check a raw pointer before deref in
//! Option-returning unsafe fns (`as_ref` + let-else prologue).
//!
//! The buggy form derefs a `*const T` param with no guard — a null caller
//! is instant UB. The bounded fix inserts a prologue that shadows the param
//! with `&T` via `as_ref()`, early-returning `None` on null. The body's
//! `(*p).field` keeps compiling unchanged (deref of the new `&T`).
//!
//! ## Gates (conservative)
//! - **Exactly ONE raw-pointer param** (`*const T` / `*mut T`): multi-param
//!   fns need one prologue per deref'd param — the single-guard fixpoint
//!   would silently half-fix; refuse instead.
//! - **`unsafe fn` returning `Option<_>`**: the prologue's `return None`
//!   only type-checks there, and `as_ref` is unsafe (edition-2024-clean via
//!   the explicit `unsafe {}`).
//! - **Textual `(*p)` present** (the deref shape) and **no existing guard**:
//!   body text containing `as_ref` or `is_null` anywhere means the fn
//!   already handles null — skip the whole fn.
//! - **No caller gate needed**: signature preserved verbatim.
//!
//! ## Honest caveats
//! - `return None` on null CHANGES behavior for callers that relied on UB
//!   (or knew null was impossible). That is the point — the UB becomes a
//!   typed miss — but it is a semantic change in the null case.
//! - The shadowed `p: &T` keeps `(*p)` working, but code reading `p as
//!  *const _` after the prologue now sees a reference — the compile gate
//!   catches those.

use syn::spanned::Spanned;
use syn::{Block, File, Signature, Type};

use super::{SourceEdit, line_indent};

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    walk_fns(&file.items, source, &mut edits);
    edits
}

fn walk_fns(items: &[syn::Item], source: &str, edits: &mut Vec<SourceEdit>) {
    for item in items {
        match item {
            syn::Item::Fn(f) => visit_fn(&f.sig, &f.block, source, edits),
            syn::Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    walk_fns(inner, source, edits);
                }
            }
            _ => {}
        }
    }
}

fn returns_option(output: &syn::ReturnType) -> bool {
    match output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ty) => {
            let syn::Type::Path(tp) = ty.as_ref() else {
                return false;
            };
            tp.path.segments.last().is_some_and(|s| s.ident == "Option")
        }
    }
}

fn visit_fn(sig: &Signature, block: &Block, source: &str, edits: &mut Vec<SourceEdit>) {
    if sig.unsafety.is_none() || !returns_option(&sig.output) {
        return;
    }
    // Exactly one raw-pointer param.
    let raw_params: Vec<&syn::PatType> = sig
        .inputs
        .iter()
        .filter_map(|a| match a {
            syn::FnArg::Typed(pt) => Some(pt),
            syn::FnArg::Receiver(_) => None,
        })
        .filter(|pt| matches!(pt.ty.as_ref(), Type::Ptr(_)))
        .collect();
    if raw_params.len() != 1 {
        return;
    }
    let pt = raw_params[0];
    let Some(ident) = pat_ident(pt) else { return };
    let block_text = &source[block.span().byte_range()];
    let deref_shape = format!("(*{ident})");
    if !block_text.contains(&deref_shape) {
        return;
    }
    // Existing guard → skip.
    if block_text.contains("as_ref") || block_text.contains("is_null") {
        return;
    }
    let r = block.span().byte_range();
    let ind = line_indent(source, r.start);
    // Insert the prologue as the first statement: replace `{` with
    // `{ prologue`. The prologue shadows `p` with `&T`.
    let prologue = format!(
        "{{\n{ind}    let Some({ident}) = (unsafe {{ {ident}.as_ref() }}) else {{ return None }};\n",
    );
    // The replacement covers the whole block; interior preserved by prefix
    // surgery on the original text (block_text starts at '{').
    debug_assert!(block_text.starts_with('{'));
    let replacement = format!("{prologue}{}", &block_text[1..]);
    edits.push(SourceEdit {
        start: r.start,
        end: r.end,
        replacement,
    });
}

fn pat_ident(pt: &syn::PatType) -> Option<String> {
    match pt.pat.as_ref() {
        syn::Pat::Ident(pi) => Some(pi.ident.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(src: &str) -> String {
        let file = syn::parse_file(src).expect("parse");
        let edits = collect_edits(src, &file);
        super::super::apply_edits(src, edits)
    }

    const BUGGY: &str =
        "pub unsafe fn read_len(p: *const Header) -> Option<u32> {\n    Some((*p).len)\n}\n";

    #[test]
    fn inserts_guard_prologue() {
        let out = applied(BUGGY);
        assert!(
            out.contains("let Some(p) = (unsafe { p.as_ref() }) else { return None };"),
            "{out}"
        );
        // The deref body is preserved verbatim.
        assert!(out.contains("Some((*p).len)"));
    }

    #[test]
    fn skips_safe_fn() {
        let src = "pub fn f(p: *const u32) -> Option<u32> { Some(*p) }";
        let file = syn::parse_file(src).unwrap();
        assert!(collect_edits(src, &file).is_empty());
    }

    #[test]
    fn skips_non_option_return() {
        let src = "pub unsafe fn f(p: *const u32) -> u32 { *p }";
        let file = syn::parse_file(src).unwrap();
        assert!(collect_edits(src, &file).is_empty());
    }

    #[test]
    fn skips_already_guarded() {
        let src = "pub unsafe fn f(p: *const u32) -> Option<u32> {\n    if p.is_null() { return None }\n    Some(*p)\n}";
        let file = syn::parse_file(src).unwrap();
        assert!(collect_edits(src, &file).is_empty());
    }

    #[test]
    fn skips_multi_raw_param_fns() {
        // The deref shape IS present ((*a)) — the skip must come from the
        // multi-param gate, not the shape gate.
        let src = "pub unsafe fn f(a: *const u32, b: *const u32) -> Option<u32> { Some((*a)) }";
        let file = syn::parse_file(src).unwrap();
        assert!(collect_edits(src, &file).is_empty());
    }
}
