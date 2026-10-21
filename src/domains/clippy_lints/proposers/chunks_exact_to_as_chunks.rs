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
//! Tier: **VERIFY**. The type-blind classes the compile gate reverts:
//! - a loop body / closure body that consumes the chunk in a position
//!   where `&[u8; N]` does not coerce (e.g. a `Vec<u8>` parameter) —
//!   parse-only cannot see callee signatures; on chains the closure
//!   param type also shifts `&[u8]` → `&[u8; N]` (const-index bodies are
//!   identical; `p.len()`/typed-position bodies revert).
//! - an UNKNOWN single-segment Path size (an imported const vs an imported
//!   `static` — the latter is not a valid const argument) and qualified
//!   paths (`Foo::N`). A path that names a runtime binding of THIS file
//!   (a `let`, a param, a loop/match binding) or a file-local `static` is
//!   PROVABLY not a const argument and refuses at the shape gate (the
//!   riir-chain `bytes_per_dim` E0435 class, 2026-09-21 — a runtime `let`
//!   reached `as_chunks::<bytes_per_dim>()` and the verify was blind at
//!   default features on a 124-feature crate). Const-generic params and
//!   `const` items are indistinguishable from variables in syn ALONE —
//!   which is why the guard runs on a file-wide binding/static pre-pass,
//!   not on the path's shape.
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

/// Names that make a single-segment path size argument provably NOT a
/// const argument: every runtime binding of the file (let / fn / closure
/// params / for / match — one `PatIdent` walk covers all binding forms)
/// and every file-local `static` (statics cannot be referenced in const
/// position). A `const` item and a const-generic param appear in NEITHER
/// set — a const generic IS a valid const argument, so only runtime
/// bindings and statics refuse.
#[derive(Default)]
struct NonConstNames {
    bindings: std::collections::HashSet<String>,
    statics: std::collections::HashSet<String>,
}

fn non_const_names(file: &File) -> NonConstNames {
    struct V {
        out: NonConstNames,
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
            self.out.bindings.insert(p.ident.to_string());
            syn::visit::visit_pat_ident(self, p);
        }
        fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
            self.out.statics.insert(i.ident.to_string());
            syn::visit::visit_item_static(self, i);
        }
    }
    let mut v = V {
        out: NonConstNames::default(),
    };
    v.visit_file(file);
    v.out
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        non_const: non_const_names(file),
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    non_const: NonConstNames,
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
        if matches!(
            m.method.to_string().as_str(),
            "remainder" | "into_remainder"
        ) {
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
            }) => {}
            Expr::Path(p) => {
                // A const-generic argument must be a const expression. A
                // SINGLE-segment path naming a runtime binding of this file
                // or a file-local `static` is provably not one — emitting
                // `as_chunks::<n>()` there is E0435 on every compile (the
                // riir-chain `bytes_per_dim` class). A QUALIFIED path
                // (`Foo::N`) can never be a local, and an unknown
                // single-segment path can only be an item (a local is
                // always in this file's binding set) — those keep the
                // VERIFY-tier emit.
                if p.qself.is_none() && p.path.segments.len() == 1 {
                    let ident = p.path.segments[0].ident.to_string();
                    if self.non_const.bindings.contains(&ident)
                        || self.non_const.statics.contains(&ident)
                    {
                        return None;
                    }
                }
            }
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
    fn runtime_param_size_refuses() {
        // A runtime size (a fn param `n`) does not compile as
        // `as_chunks::<n>()` (E0435) — the shape gate refuses it outright
        // now (the riir-chain `bytes_per_dim` class, 2026-09-21): the emit
        // must not depend on the verify gate catching it, because a
        // default-features verify is blind on heavily-gated crates.
        let input = "fn f(bytes: &[u8], n: usize) -> usize { let mut acc = 0; for c in bytes.chunks_exact(n) { acc += c.len(); } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "runtime param size must refuse: {out:?}");
    }

    #[test]
    fn runtime_let_size_refuses() {
        // The live riir-chain shape: `let bytes_per_dim = 32 / dim;` — a
        // runtime let binding reached the const-generic slot.
        let input = "fn f(commitment: &[u8], dim: usize) -> usize { let bytes_per_dim = 32 / dim; let mut acc = 0; for c in commitment.chunks_exact(bytes_per_dim) { acc += c[0] as usize; } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "runtime let size must refuse: {out:?}");
    }

    #[test]
    fn file_local_static_refuses() {
        // A `static` is not a valid const-generic argument either
        // ("statics cannot be referenced in constants").
        let input = "static N: usize = 4;\nfn f(bytes: &[u8]) -> usize { let mut acc = 0; for c in bytes.chunks_exact(N) { acc += c.len(); } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input);
        assert!(out.is_none(), "file-local static size must refuse: {out:?}");
    }

    #[test]
    fn const_generic_param_fires() {
        // `fn f<const N: usize>` — a const-generic param IS a valid const
        // argument; it is not a PatIdent binding, so the pre-pass leaves it
        // alone.
        let input = "fn f<const N: usize>(bytes: &[u8]) -> usize { let mut acc = 0; for c in bytes.chunks_exact(N) { acc += c.len(); } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(out.contains("bytes.as_chunks::<N>().0"), "{out}");
    }

    #[test]
    fn qualified_const_path_fires() {
        // `P::N` — a qualified path can never be a local binding.
        let input = "struct P;\nimpl P { const N: usize = 4; }\nfn f(bytes: &[u8]) -> usize { let mut acc = 0; for c in bytes.chunks_exact(P::N) { acc += c.len(); } acc }";
        let out = propose_fix("chunks_exact_to_as_chunks", input).expect("matched");
        assert!(out.contains("bytes.as_chunks::<P::N>().0"), "{out}");
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
        assert!(
            out.matches("as_chunks::<4>().0.iter()").count() == 2,
            "both roots rewritten: {out}"
        );
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
        assert_eq!(
            out.matches("as_chunks::<4>().0").count(),
            1,
            "exactly one rewrite: {out}"
        );
        assert!(
            !out.contains(".0.iter()"),
            "legacy for shape gains no .iter(): {out}"
        );
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
        assert!(
            fixed.contains("a.as_chunks::<4>().0.iter()"),
            "receiver root rewritten: {fixed}"
        );
    }
}
