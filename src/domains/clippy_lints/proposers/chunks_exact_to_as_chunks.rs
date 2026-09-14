//! `clippy::chunks_exact_to_as_chunks` — `for c in data.chunks_exact(4) { }`
//! → `for c in data.as_chunks::<4>().0 { }` (and the `_mut` twin), plus the
//! ITERATOR-CHAIN root shape (P13, 2026-09-14):
//! `data.chunks_exact(4).map(|p| ..)` →
//! `data.as_chunks::<4>().0.iter().map(|p| ..)` — the root call is rewritten
//! wherever it is the RECEIVER of a downstream method (`map`/`all`/`any`/
//! `enumerate`/`for_each`/`sum`/… compose unchanged; clippy's own suggestion for
//! chain sites inserts the same `.0.iter()`).
//!
//! Style lint (new in clippy 1.98): a constant chunk size makes `as_chunks`
//! usable, which yields `&[u8; N]` directly — no `try_into().unwrap()` per
//! chunk. Real clippy fires on both `chunks_exact` and `chunks_exact_mut`
//! with a LITERAL or named-const size, and stays silent when the loop
//! consumes the iterator's `remainder`/`into_remainder` (our shape gate
//! refuses those independently: the iterable must be the direct method
//! call — `for c in it.by_ref()` never fires here; the chain root must be
//! the direct receiver of a method call — a BOUND iterator consumed later
//! can still reach `remainder()`, so `it.map(..)` never fires either).
//!
//! Tier: **VERIFY**. Two type-blind classes the compile gate reverts:
//! - a loop body / closure body that consumes the chunk in a position
//!   where `&[u8; N]` does not coerce (e.g. a `Vec<u8>` parameter) —
//!   parse-only cannot see callee signatures; on chains the closure
//!   param type also shifts `&[u8]` → `&[u8; N]` (const-index bodies are
//!   identical; `p.len()`/typed-position bodies revert);
//! - a NON-const `Path` size argument (a plain variable `n` compiles as
//!   `chunks_exact(n)` but not as `as_chunks::<n>()`; a named `const` is
//!   indistinguishable from a variable in syn).
//!
//! SAFER-than-clippy divergence (measured, 2026-09-14 probe): for the MUT
//! chain root clippy literally suggests `as_chunks_mut::<N>().0.iter()` —
//! but `.iter()` on `&mut [[T; N]]` yields IMMUTABLE `&[T; N]` items, an
//! E0594 on every mutating body (the lint is suggestion-only for exactly
//! this class of reason). We emit `.0.iter_mut()` for the mut twin — the
//! compiling form for both mutating and read-only bodies.

use super::helpers::drops_interior_comment;
use super::{edit, src, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprForLoop, ExprLit, ExprMethodCall, ExprParen, File, Lit};

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

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_for_loop(&mut self, f: &'ast ExprForLoop) {
        syn::visit::visit_expr_for_loop(self, f);
        if let Some(replacement) = self.try_as_chunks(f) {
            self.edits.push(edit(&*f.expr, replacement));
        }
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        // The CHAIN shape (P13): this call's receiver IS the chunks_exact
        // call — rewrite the ROOT; every downstream adapter composes
        // unchanged. A bound iterator (`let it = x.chunks_exact(4);
        // it.map(..)`) has a `Path` receiver and never reaches here, which
        // is exactly the remainder()-safety boundary.
        if matches!(m.method.to_string().as_str(), "remainder" | "into_remainder") {
            // Defensive: never rewrite the root under a remainder consumer,
            // even though these shapes need `&mut self` bindings to compile.
            return;
        }
        let target = match &*m.receiver {
            Expr::MethodCall(call) => call,
            Expr::Paren(ExprParen { expr: inner, .. }) => match &**inner {
                Expr::MethodCall(call) => call,
                _ => return,
            },
            _ => return,
        };
        if let Some(replacement) = self.rewrite_chain_root(target) {
            self.edits.push(edit(target, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_as_chunks(&self, f: &ExprForLoop) -> Option<String> {
        // The iterable must be the DIRECT call — `it.by_ref()` / iterator
        // bindings can carry a `remainder()` consumer we cannot see.
        let Expr::MethodCall(m) = &*f.expr else {
            return None;
        };
        self.rewrite_call(m)
    }

    fn rewrite_call(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() {
            return None;
        }
        let as_method = match m.method.to_string().as_str() {
            "chunks_exact" => "as_chunks",
            "chunks_exact_mut" => "as_chunks_mut",
            _ => return None,
        };
        if m.args.len() != 1 {
            return None;
        }
        let arg = m.args.first().unwrap();
        match arg {
            Expr::Lit(ExprLit {
                lit: Lit::Int(_), ..
            })
            | Expr::Path(_) => {}
            _ => return None,
        }
        // Span-replacement — an interior comment (`chunks_exact(/* N */ 4)`)
        // would be displaced.
        if drops_interior_comment(self.source, m.span().byte_range(), &[]) {
            return None;
        }
        let recv = src(self.source, &*m.receiver)?;
        let arg_text = src(self.source, arg)?;
        Some(format!("{recv}.{as_method}::<{arg_text}>().0"))
    }

    /// The chain-root twin: same call rewrite plus the iterator adapter the
    /// slice needs. Non-mut roots take `.iter()` (clippy's own chain
    /// suggestion); the mut root takes `.iter_mut()` — clippy's literal
    /// `.iter()` suggestion is an E0594 on every mutating body (see the
    /// module doc's SAFER-than-clippy note).
    fn rewrite_chain_root(&self, target: &ExprMethodCall) -> Option<String> {
        let base = self.rewrite_call(target)?;
        let iter = if target.method == "chunks_exact_mut" {
            ".iter_mut()"
        } else {
            ".iter()"
        };
        Some(format!("{base}{iter}"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn literal_size_fires() {
        let input = "fn f(bytes: &[u8], out: &mut Vec<u32>) { for chunk in bytes.chunks_exact(4) { out.push(u32::from_le_bytes(chunk.try_into().unwrap())); } }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(
            out.contains("bytes.as_chunks::<4>().0"),
            "no rewrite: {out}"
        );
        // The iterable is the ONLY thing rewritten — the loop body's byte
        // text is untouched (whether it still compiles as `&[u8; N]` is the
        // --verify compile gate's call, not the proposer's).
        assert!(
            out.contains("chunk.try_into().unwrap()"),
            "body must stay: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn const_path_size_fires() {
        let input = "const N: usize = 4;\nfn f(bytes: &[u8]) -> usize { let mut acc = 0; for c in bytes.chunks_exact(N) { acc += c.len(); } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(out.contains("bytes.as_chunks::<N>().0"), "{out}");
    }

    #[test]
    fn mut_twin_fires() {
        let input =
            "fn f(data: &mut [u8]) { for chunk in data.chunks_exact_mut(2) { chunk[0] = 0; } }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(out.contains("data.as_chunks_mut::<2>().0"), "{out}");
    }

    #[test]
    fn variable_size_still_emits_but_is_verify_tier() {
        // A non-const Path arg compiles as chunks_exact(n) but NOT as
        // as_chunks::<n>() — the VERIFY compile gate reverts it. The shape
        // gate accepts it (indistinguishable from a named const).
        let input = "fn f(bytes: &[u8], n: usize) -> usize { let mut acc = 0; for c in bytes.chunks_exact(n) { acc += c.len(); } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(out.contains("bytes.as_chunks::<n>().0"), "{out}");
    }

    #[test]
    fn other_chunk_methods_silent() {
        let input = "fn f(bytes: &[u8]) { for c in bytes.chunks(4) {} }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "chunks() must not fire: {out:?}");
    }

    #[test]
    fn turbofish_or_attrs_silent() {
        let input = "fn f(bytes: &[u8]) { for c in bytes.chunks_exact::<[(); 0]>(4) {} }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "turbofish must not fire: {out:?}");
    }

    #[test]
    fn by_ref_form_silent() {
        // The remainder-consuming shape: the iterator is bound, the loop
        // walks it by ref — refusing is what keeps remainder() safe.
        let input = "fn f(bytes: &[u8]) -> usize { let mut it = bytes.chunks_exact(4); let mut acc = 0; for c in it.by_ref() { acc += c.len(); } acc + it.remainder().len() }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "remainder shape must not fire: {out:?}");
    }

    #[test]
    fn interior_comment_refused() {
        let input = "fn f(bytes: &[u8]) { for c in bytes.chunks_exact(/* w */ 4) {} }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "interior comment must refuse: {out:?}");
    }

    // ── P13 chain-root shapes (2026-09-14) ─────────────────────────────

    #[test]
    fn chain_map_collect_fires() {
        let input =
            "fn firsts(bytes: &[u8]) -> Vec<u8> { bytes.chunks_exact(4).map(|p| p[0]).collect() }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(
            out.contains("bytes.as_chunks::<4>().0.iter().map(|p| p[0]).collect()"),
            "no chain rewrite: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_all_any_enumerate_fire() {
        let input = "fn f(bytes: &[u8]) -> bool { bytes.chunks_exact(4).all(|c| c[0] == 0) && bytes.chunks_exact(4).any(|c| c[1] > 0) }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(out.matches("as_chunks::<4>().0.iter()").count() == 2, "both roots rewritten: {out}");
    }

    #[test]
    fn chain_mut_twin_uses_iter_mut() {
        // The SAFER-than-clippy divergence: clippy suggests `.0.iter()` for
        // the mut root, an E0594 on mutating bodies; we emit `.iter_mut()`.
        let input = "fn f(data: &mut [u8]) { data.chunks_exact_mut(2).for_each(|c| c[0] = 0); }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(
            out.contains("data.as_chunks_mut::<2>().0.iter_mut().for_each(|c| c[0] = 0)"),
            "mut chain must use iter_mut: {out}"
        );
    }

    #[test]
    fn for_over_chain_fires_on_root() {
        // The iterable is a CHAIN — the for-loop visitor refuses it; the
        // chain visitor rewrites the root (clippy's own suggestion for this
        // shape also carries `.0.iter()`, the for consumes the adapter).
        let input = "fn f(bytes: &[u8]) -> usize { let mut acc = 0; for b in bytes.chunks_exact(2).map(|p| p[1]) { acc += b as usize; } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(
            out.contains("for b in bytes.as_chunks::<2>().0.iter().map(|p| p[1])"),
            "for-over-chain root rewrite missing: {out}"
        );
    }

    #[test]
    fn direct_for_loop_still_single_edit() {
        // The two visitors are disjoint on the legacy shape: exactly one
        // rewrite, without an `.iter()` suffix (for consumes the slice
        // directly).
        let input = "fn f(bytes: &[u8], out: &mut Vec<u32>) { for chunk in bytes.chunks_exact(4) { out.push(u32::from_le_bytes(chunk.try_into().unwrap())); } }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert_eq!(out.matches("as_chunks::<4>().0").count(), 1, "exactly one rewrite: {out}");
        assert!(!out.contains(".0.iter()"), "legacy for shape gains no .iter(): {out}");
    }

    #[test]
    fn bound_iterator_chain_still_silent() {
        // The remainder-reachable shape: the iterator is bound, then a
        // method chain consumes the BINDING — receiver is a Path, never
        // fires; `remainder()` later in the same fn stays valid.
        let input = "fn f(bytes: &[u8]) -> usize { let mut it = bytes.chunks_exact(4); let s: usize = it.map(|p| p[0] as usize).sum(); s + it.into_remainder().len() }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "bound-iterator chain must not fire: {out:?}");
    }

    #[test]
    fn remainder_direct_chain_refused() {
        // Defensive arm: the downstream method itself is a remainder
        // consumer (these need &mut bindings to compile, but refuse anyway).
        let input = "fn f(bytes: &[u8]) -> usize { let n = bytes.chunks_exact(4).into_remainder().len(); n }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "remainder chain must refuse: {out:?}");
    }

    #[test]
    fn arg_position_chain_root_silent() {
        // An iterator ADAPTER argument (zip's operand) is not a receiver —
        // real clippy fires there, we deliberately do not (which methods are
        // iterator adapters is a type question; documented divergence).
        let input = "fn f(a: &[u8], b: &[u8]) -> usize { a.chunks_exact(4).zip(b.chunks_exact(4)).count() }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        let fixed = out.expect("the receiver root still fires");
        assert!(
            !fixed.contains("zip(b.as_chunks"),
            "arg-position root must stay untouched: {fixed}"
        );
        assert!(fixed.contains("a.as_chunks::<4>().0.iter()"), "receiver root rewritten: {fixed}");
    }
}
