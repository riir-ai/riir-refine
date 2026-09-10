//! `clippy::chunks_exact_to_as_chunks` — `for c in data.chunks_exact(4) { }`
//! → `for c in data.as_chunks::<4>().0 { }` (and the `_mut` twin).
//!
//! Style lint (new in clippy 1.98): a constant chunk size makes `as_chunks`
//! usable, which yields `&[u8; N]` directly — no `try_into().unwrap()` per
//! chunk. Real clippy fires on both `chunks_exact` and `chunks_exact_mut`
//! with a LITERAL or named-const size, and stays silent when the loop
//! consumes the iterator's `remainder`/`into_remainder` (our shape gate
//! refuses those independently: the iterable must be the direct method
//! call — `for c in it.by_ref()` never fires here).
//!
//! Tier: **VERIFY**. Two type-blind classes the compile gate reverts:
//! - a loop body that consumes the chunk in a position where `&[u8; N]`
//!   does not coerce (e.g. a `Vec<u8>` parameter) — parse-only cannot see
//!   callee signatures;
//! - a NON-const `Path` size argument (a plain variable `n` compiles as
//!   `chunks_exact(n)` but not as `as_chunks::<n>()`; a named `const` is
//!   indistinguishable from a variable in syn).
//!
//! MSRV note: `slice::as_chunks` stabilized in Rust 1.88. The workspace
//! floats on 1.98.1, and real clippy is MSRV-aware (it never fires on a
//! crate declaring an older `rust-version`, so the `--suggest` path is
//! inherently safe); the AST-driven `--fix` path has no manifest access in
//! `collect_edits` (the `fxhashmap` file-import gate precedent), so the
//! floor is documented here rather than plumbed.

use super::helpers::drops_interior_comment;
use super::{edit, src, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprForLoop, ExprLit, ExprMethodCall, File, Lit};

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
}
