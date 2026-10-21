//! `clippy::bytes_count_to_len` — `x.bytes().count()` → `x.len()` (Batch
//! 162 / Plan 153 — the frontier-lead mining pack VIII; the family fixture
//! verified to fire exactly this lint under all+pedantic+nursery with a
//! clean seed, 2026-09-18 (`scripts/plan152_frontier_probe.py`)).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `str::bytes()` yields exactly the elements of `str::as_bytes()` — the
//! docs define it as that iterator — and `Iterator::count()` on an
//! `ExactSizeIterator` is `len()`. So `s.bytes().count()` IS
//! `s.as_bytes().len()`, which for a `str` receiver IS `s.len()` (the same
//! u8 slice). For a `&[u8]` receiver, `bytes()` re-yields the slice's own
//! `u8` items and `count()` is `len()` likewise. The rewrite is the same
//! value with no side effects on either side — value-identical, and the
//! identity is std's own documented definition (the same construction
//! identity class as Batch 161's `b"abc"` coercion and Batch 160's
//! radix-10 `FromStr`).
//!
//! ## Guards (the bounded fix-space)
//!
//! - The chain must be EXACTLY `.bytes().count()` — two method calls, no
//!   arguments on either, no attributes. `bytes_mut()` (returns
//!   `&mut [u8]`, not an iterator — would not even compile), `chars()`,
//!   `char_indices()`, any adapter between the two calls, and any
//!   additional `.count()`-consuming adapters all refuse.
//! - Both calls must be method-call syntax on the same receiver spine —
//!   UFCS (`bytes::count(x)`) and free-function forms don't match by
//!   construction.
//! - Comments inside the replaced span refuse (the span re-spells to
//!   `<receiver>.len()`).

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File};

use super::helpers::byte_range;
use super::{edit, SourceEdit};
use crate::edit_emitter::drops_interior_comment;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = self.try_rewrite(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    /// The outer `.count()` call; the receiver must be a `.bytes()` call.
    fn try_rewrite(&mut self, count_call: &ExprMethodCall) -> Option<String> {
        if count_call.method != "count"
            || !count_call.args.is_empty()
            || !count_call.attrs.is_empty()
            || count_call.turbofish.is_some()
        {
            return None;
        }
        let Expr::MethodCall(bytes_call) = &*count_call.receiver else {
            return None;
        };
        if bytes_call.method != "bytes"
            || !bytes_call.args.is_empty()
            || !bytes_call.attrs.is_empty()
            || bytes_call.turbofish.is_some()
        {
            return None;
        }
        let call_span = byte_range(count_call);
        if drops_interior_comment(self.source, call_span, &[]) {
            return None;
        }
        // `<receiver>.len()` — the receiver spine re-emits verbatim.
        let receiver_src = self
            .source
            .get(byte_range(&bytes_call.receiver))?
            .to_string();
        Some(format!("{receiver_src}.len()"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "bytes_count_to_len",
            "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.len() }",
        )
        .is_none());
    }

    #[test]
    fn str_receiver_fires() {
        let out = propose_fix(
            "bytes_count_to_len",
            "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes().count() }",
        )
        .expect("matched");
        assert!(out.contains("s.len()"), "not rewritten: {out}");
        assert!(!out.contains("bytes()"), "chain not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn slice_receiver_fires() {
        let out = propose_fix(
            "bytes_count_to_len",
            "fn n(b: &[u8], hits: &mut Vec<u8>) -> usize { hits.push(0); b.bytes().count() }",
        )
        .expect("matched");
        assert!(out.contains("b.len()"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_larger_expr_fires() {
        let out = propose_fix(
            "bytes_count_to_len",
            "fn n(s: &str, hits: &mut Vec<u8>) -> bool { hits.push(0); s.bytes().count() > 3 }",
        )
        .expect("matched");
        assert!(out.contains("s.len() > 3"), "not in place: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn adapter_between_calls_refuses() {
        // An adapter between the two calls is NOT the identity chain.
        assert!(propose_fix(
            "bytes_count_to_len",
            "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes().filter(|b| *b > 0).count() }",
        )
        .is_none());
    }

    #[test]
    fn chars_is_not_the_family() {
        assert!(propose_fix(
            "bytes_count_to_len",
            "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.chars().count() }",
        )
        .is_none());
    }

    #[test]
    fn bytes_mut_refuses() {
        // `bytes_mut` returns &mut [u8] — not the iterator chain at all.
        assert!(propose_fix(
            "bytes_count_to_len",
            "fn n(b: &mut [u8], hits: &mut Vec<u8>) -> usize { hits.push(0); b.bytes_mut().count() }",
        )
        .is_none());
    }

    #[test]
    fn turbofish_refuses() {
        // A turbofish-shaped chain (map before count) is an adapter chain —
        // not the identity; refuse.
        assert!(propose_fix(
            "bytes_count_to_len",
            "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes().map(|b| b).count() }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            "bytes_count_to_len",
            "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes()/* mid */.count() }",
        )
        .is_none());
    }
}
