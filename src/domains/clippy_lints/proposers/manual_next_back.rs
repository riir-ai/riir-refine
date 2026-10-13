//! `clippy::manual_next_back` — `it.rev().next()` → `it.next_back()`
//! (Batch 228 / Plan 230 — the coverage-report re-arm pack XIX; the
//! firing/silence matrix verified against real clippy 0.1.98, 2026-10-13,
//! `scripts/plan230_pack19_probe.py` VERDICT PASS; style/warn-by-default,
//! group pinned by the probe's no-flags arm).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `DoubleEndedIterator::next_back` is the reverse-direction counterpart
//! of `next`, and the `Rev` adapter inverts the direction once more —
//! `Rev`'s own std definition IS
//! `fn next(&mut self) -> Option<I::Item> { self.0.next_back() }` (its
//! `next_back` being `self.0.next()`), so `it.rev().next()` is DEFINED as
//! `it.next_back()`. The input's own compilation proves
//! `I: DoubleEndedIterator` — `rev()` is defined only on that trait — so
//! the identity holds for ANY receiver, not just the probe's `iter()`
//! slices (zero type-blindness; the adapter-definition-identity class of
//! Batch 171's `map_clone` by-ref adapters).
//!
//! ## Guards (the bounded fix-space)
//!
//! - The node is a `.next()` call with ZERO args whose receiver is a
//!   `.rev()` call with ZERO args — the rewrite is local to that
//!   two-call tail: everything below `.rev()` (`.iter()`, `.iter_mut()`,
//!   any adapter chain) and everything above `.next()` (`.copied()`, ...)
//!   re-emits untouched (upstream's own suggestion is equally local,
//!   probe-measured on the `iter()` shapes).
//! - The deeper receiver is unconstrained — `x.rev().next() ==
//!   x.next_back()` needs `x: DoubleEndedIterator`, which the original's
//!   compilation proves by method resolution.
//! - Turbofish on either call, arguments on either call, or attributes on
//!   either call refuse; a comment inside the replaced span refuses.
//! - Plain `.next()` without `.rev()`, and `.last()`, are SILENT upstream
//!   (probe-measured) — parity refusals.

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File};

use super::helpers::byte_range;
use super::{edit, src, SourceEdit};
use crate::edit_emitter::drops_interior_comment;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_method_call(&mut self, c: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, c);
        if let Some(replacement) = self.try_rewrite(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    /// `<recv>.rev().next()` → `<recv>.next_back()` — the two-call tail
    /// re-spelled; the receiver below `.rev()` re-emits verbatim.
    fn try_rewrite(&self, c: &ExprMethodCall) -> Option<String> {
        if c.method != "next"
            || !c.args.is_empty()
            || c.turbofish.is_some()
            || !c.attrs.is_empty()
        {
            return None;
        }
        let Expr::MethodCall(rev) = &*c.receiver else { return None };
        if rev.method != "rev"
            || !rev.args.is_empty()
            || rev.turbofish.is_some()
            || !rev.attrs.is_empty()
        {
            return None;
        }
        if drops_interior_comment(self.source, byte_range(c), &[]) {
            return None;
        }
        // The deeper receiver re-emits verbatim (the caller's own
        // formatting, comments excluded — spans cover tokens).
        let head = src(self.source, &rev.receiver)?;
        Some(format!("{head}.next_back()"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "manual_next_back";

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            RULE,
            "fn t(vals: &[u32], sink: &mut Vec<u32>) { sink.push(vals.len()); }",
        )
        .is_none());
    }

    #[test]
    fn rev_next_fires() {
        let out = propose_fix(
            RULE,
            "fn t(vals: &[u32]) -> Option<u32> { vals.iter().rev().next().copied() }",
        )
        .expect("matched");
        assert!(out.contains(".next_back()"), "not rewritten: {out}");
        assert!(!out.contains(".rev()"), "rev not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rev_next_mut_fires() {
        let out = propose_fix(
            RULE,
            "fn t(vals: &mut [u32]) -> Option<&mut u32> { vals.iter_mut().rev().next() }",
        )
        .expect("matched");
        assert!(out.contains(".iter_mut().next_back()"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn standalone_rev_next_fires() {
        // The receiver below `.rev()` is unconstrained — the original's
        // compilation proves `DoubleEndedIterator` by method resolution.
        let out = propose_fix(
            RULE,
            "fn t(it: std::vec::IntoIter<u32>) -> Option<u32> { it.rev().next() }",
        )
        .expect("matched");
        assert!(out.contains("it.next_back()"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn plain_next_refuses() {
        // SILENT upstream (probe-measured) — parity refusal.
        assert!(propose_fix(
            RULE,
            "fn t(vals: &[u32]) -> Option<u32> { vals.iter().next().copied() }",
        )
        .is_none());
    }

    #[test]
    fn last_refuses() {
        // SILENT upstream (probe-measured) — parity refusal.
        assert!(propose_fix(
            RULE,
            "fn t(vals: &[u32]) -> Option<u32> { vals.iter().last().copied() }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            RULE,
            "fn t(vals: &[u32]) -> Option<u32> { vals.iter() /* now */ .rev().next().copied() }",
        )
        .is_none());
    }
}
