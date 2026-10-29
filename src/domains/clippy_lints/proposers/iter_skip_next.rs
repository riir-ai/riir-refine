//! `clippy::iter_skip_next` — `x.skip(n).next()` → `x.nth(n)`, and on
//! SIGNATURE-PROVEN slice-container receivers the oracle-clean inverse
//! `v.iter().skip(n).next()` → `v.get(n)`
//! (Batch 157 / Plan 148 — the frontier-lead mining pack III; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery, 2026-09-18).
//!
//! TWO arms, ladder-ordered:
//!
//! 1. **The slice arm** — when the receiver chain is exactly
//!    `<binding>.iter()` and the binding is a signature-proven
//!    `&[T]`/`&[T; N]`/`&Vec<T>` param (`helpers::slice_param_heads`, never
//!    rebound in the body), emit `<binding>.get(n)`. This is ONE STEP BETTER
//!    than clippy's own `.nth(n)` suggestion, which fires `clippy::iter_nth`
//!    on slice iterators (measured, the Batch 156 round-3 probe) — the
//!    oracle-clean inverse. `slice/Vec::get(i)` ≡ `iter().skip(i).next()`
//!    exactly: both `Option<&T>`, both `None` out of range, neither panics,
//!    both borrow the receiver shared.
//! 2. **The generic arm** — any other `X.skip(A).next()` → `X.nth(A)`:
//!    `Iterator::nth(n)` IS `skip(n).next()` by std semantics for every
//!    iterator (both consume, both return the element at index n), so the
//!    rewrite is identical whenever the input compiled — the parse-only
//!    method-name match is the `iter_nth_zero`/`filter_next` corpus
//!    precedent. Documented divergence: an unproven-but-actually-slice
//!    receiver heals to `.nth(n)` (clippy's own suggestion), which may fire
//!    `iter_nth` on that receiver — the derivation narrows arm 1, never
//!    widens it.
//!
//! Guards: attrs/turbofish on any link, `.skip` arg count ≠ 1, `.next` with
//! args, and comments in the two DELETED ranges (`.skip(` and `).next()`)
//! all refuse — the skip argument's text rides VERBATIM (copied, so
//! comments inside it survive).

use std::collections::BTreeSet;

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, ImplItemFn, ItemFn};

use super::helpers::{byte_range, drops_interior_comment, slice_param_heads_over};
use super::{SourceEdit, edit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        proven: BTreeSet::new(),
        saved: BTreeSet::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Signature-proven slice-container binding idents of the enclosing fn.
    proven: BTreeSet<String>,
    /// The set to restore on fn exit (one-deep save).
    saved: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        let heads = slice_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.proven = heads.into_keys().collect();
        self.saved = saved;
        syn::visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        let heads = slice_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.proven = heads.into_keys().collect();
        self.saved = saved;
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = self.try_rewrite(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() || m.method != "next" {
            return None;
        }
        if !m.args.is_empty() {
            return None;
        }
        let skip = match unwrap_parens(&m.receiver) {
            Expr::MethodCall(s)
                if s.method == "skip" && s.turbofish.is_none() && s.attrs.is_empty() =>
            {
                s
            }
            _ => return None,
        };
        if skip.args.len() != 1 {
            return None;
        }
        let arg_range = byte_range(&skip.args[0]);
        let m_range = byte_range(m);
        let arg_text = self.source.get(arg_range.clone())?;
        // The trailing deleted range `).next()` is shared by both arms.
        let del_tail = arg_range.end..m_range.end;
        if drops_interior_comment(self.source, del_tail, &[]) {
            return None;
        }

        // Arm 1 — `<binding>.iter().skip(A).next()` → `<binding>.get(A)`
        // (signature-proven slice container only; the oracle-clean inverse).
        if let Some((path_text, path_range)) = self.proven_iter_path(skip) {
            let del_head = path_range.end..arg_range.start;
            if !drops_interior_comment(self.source, del_head, &[]) {
                return Some(format!("{path_text}.get({arg_text})"));
            }
        }

        // Arm 2 — `X.skip(A).next()` → `X.nth(A)` (clippy's own suggestion,
        // identical for every iterator). The receiver text is COPIED — it
        // was already valid in method-receiver position, comments included.
        let recv_range = byte_range(&skip.receiver);
        let recv_text = self.source.get(recv_range.clone())?;
        let del_head = recv_range.end..arg_range.start;
        if drops_interior_comment(self.source, del_head, &[]) {
            return None;
        }
        Some(format!("{recv_text}.nth({arg_text})"))
    }

    /// `(binding text, binding span)` when the skip receiver is exactly
    /// `<proven-binding>.iter()`.
    fn proven_iter_path(&self, skip: &ExprMethodCall) -> Option<(String, std::ops::Range<usize>)> {
        let iter = match unwrap_parens(&skip.receiver) {
            Expr::MethodCall(i)
                if i.method == "iter" && i.turbofish.is_none() && i.attrs.is_empty() =>
            {
                i
            }
            _ => return None,
        };
        if !iter.args.is_empty() {
            return None;
        }
        let Expr::Path(p) = unwrap_parens(&iter.receiver) else {
            return None;
        };
        if p.qself.is_some()
            || p.path.leading_colon.is_some()
            || p.path.segments.len() != 1
            || !p.path.segments[0].arguments.is_none()
        {
            return None;
        }
        let ident = p.path.segments[0].ident.to_string();
        if !self.proven.contains(&ident) {
            return None;
        }
        let range = byte_range(p);
        Some((self.source.get(range.clone())?.to_string(), range))
    }
}

/// Peel paren layers (shape checks only — emitted text slices the ORIGINAL
/// node, so parens ride verbatim).
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
    fn proven_slice_receiver_heals_to_get() {
        let out = propose_fix(
            "iter_skip_next",
            "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(n).next() }",
        )
        .expect("matched");
        assert!(out.contains("{ v.get(n) }"), "not rewritten to .get: {out}");
        assert!(!out.contains("skip"), "skip must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn proven_vec_receiver_heals_to_get() {
        let out = propose_fix(
            "iter_skip_next",
            "fn pick(v: &Vec<u8>, k: usize) -> Option<&u8> { v.iter().skip(k).next() }",
        )
        .expect("matched");
        assert!(out.contains("{ v.get(k) }"), "not rewritten to .get: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unproven_receiver_heals_to_nth() {
        // No signature derivation available (iterator from a method chain) —
        // the generic arm, clippy's own suggestion.
        let out = propose_fix(
            "iter_skip_next",
            "fn at(s: &str, n: usize) -> Option<char> { s.chars().skip(n).next() }",
        )
        .expect("matched");
        assert!(
            out.contains("s.chars().nth(n)"),
            "not rewritten to .nth: {out}"
        );
        assert!(!out.contains("skip"), "skip must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unproven_slice_shaped_receiver_falls_to_nth() {
        // `.iter()` chain but the binding is not signature-provable (a let
        // without type ascription) — arm 2 fires, the documented divergence.
        let out = propose_fix(
            "iter_skip_next",
            "fn at(n: usize) -> Option<u8> { let v = [1u8, 2, 3]; v.iter().skip(n).copied().skip(n).next() }",
        );
        assert!(
            out.as_deref()
                .is_some_and(|o| o.contains("v.iter().skip(n).copied().nth(n)")),
            "arm 2 must fire on the unprovable chain: {out:?}"
        );
    }

    #[test]
    fn complex_arg_rides_verbatim() {
        let out = propose_fix(
            "iter_skip_next",
            "fn at(v: &[u8], i: usize) -> Option<&u8> { v.iter().skip(i + 1).next() }",
        )
        .expect("matched");
        assert!(
            out.contains("v.get(i + 1)"),
            "arg text must ride verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rebound_binding_falls_to_nth() {
        // The binding is rebound — the derivation refuses, arm 2 fires.
        let out = propose_fix(
            "iter_skip_next",
            "fn at(v: &[u8], n: usize, flag: bool) -> Option<&u8> { let v: &[u8]; v = if flag { &[] } else { &[] }; v.iter().skip(n).next() }",
        )
        .expect("arm 2 matched");
        assert!(
            out.contains("v.iter().nth(n)"),
            "unproven receiver must heal to .nth: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn skip_without_next_untouched() {
        let out = propose_fix(
            "iter_skip_next",
            "fn go(v: &[u8], n: usize) -> usize { v.iter().skip(n).count() }",
        );
        assert!(out.is_none(), "skip without next must not fire: {out:?}");
    }

    #[test]
    fn next_without_skip_untouched() {
        let out = propose_fix(
            "iter_skip_next",
            "fn go(v: &[u8]) -> Option<&u8> { v.iter().next() }",
        );
        assert!(out.is_none(), "next without skip must not fire: {out:?}");
    }

    #[test]
    fn turbofished_skip_refused() {
        let out = propose_fix(
            "iter_skip_next",
            "fn go(s: &str, n: usize) -> Option<char> { s.chars().skip::<std::str::Chars<'_>>(n).next() }",
        );
        assert!(out.is_none(), "turbofished skip must refuse: {out:?}");
    }

    #[test]
    fn next_with_args_refused() {
        let out = propose_fix(
            "iter_skip_next",
            "fn go(s: &str, n: usize) -> Option<char> { s.chars().skip(n).next_extra(1) }",
        );
        assert!(out.is_none(), "next with args must refuse: {out:?}");
    }

    #[test]
    fn comment_in_deleted_range_refused() {
        let out = propose_fix(
            "iter_skip_next",
            "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(/* idx */ n).next() }",
        );
        assert!(
            out.is_none(),
            "comment in a deleted range must refuse: {out:?}"
        );
    }

    #[test]
    fn trailing_comment_after_arg_refused() {
        // The comment sits AFTER the arg's span, inside the `).next()` range
        // the rewrite deletes — refusing keeps it from being dropped.
        let out = propose_fix(
            "iter_skip_next",
            "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(n /* the index */).next() }",
        );
        assert!(
            out.is_none(),
            "comment in the deleted tail must refuse: {out:?}"
        );
    }

    #[test]
    fn impl_method_receiver_uses_get() {
        let out = propose_fix(
            "iter_skip_next",
            "struct S;\nimpl S {\n    fn at(&self, v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(n).next() }\n}",
        )
        .expect("matched");
        assert!(out.contains("v.get(n)"), "not rewritten to .get: {out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
