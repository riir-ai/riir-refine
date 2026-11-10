//! `clippy::useless_vec` — `&vec![…]` (a borrow of a temporary `vec!`) →
//! `&[…]` (the array-literal slice).
//!
//! Probe-mapped trigger surface (clippy 1.99.0, 2026-11-10 — the P112
//! intake, the seal-render `mip.rs:121/125` test-fixture instance): the
//! lint fires DEFAULT-ON on the borrow-immediately shape — `&vec![1, 2,
//! 3]` at a call-argument position → "you can use a slice directly:
//! `&[1, 2, 3]`" — and on the repeat form with a literal length
//! (`&vec![0; 8]` → `&[0; 8]`). It stays SILENT on the variable-length
//! repeat (`&vec![0; n]` — an array length must be const), on the
//! let-bound-then-borrowed shape (`let v = vec![..]; f(&v)` — typeck
//! territory at 1.99), on mutated vecs, and on `Vec`-typed returns. Real
//! clippy's suggestions are MACHINE-APPLICABLE (probed: "run `cargo
//! clippy --fix` to apply"), but the P21 oracle lane only consumes
//! suggestions for lints the corpus CARRIES (the P51 corpus-rule
//! precondition) — this corpus rule is what unlocks both the oracle lane
//! and the standalone scan path.
//!
//! The rewrite splices the macro interior VERBATIM between `[` and `]`
//! (elements, separators, comments and interior whitespace survive
//! byte-identically — the interior is located by the FIRST `[` and the
//! macro expression's closing `]`, so a `]` inside a string literal
//! cannot derail the splice). Guards:
//! - `&mut vec![…]` refuses (the `mutability` arm is the documented
//!   divergence — pointless on a temporary);
//! - the repeat length must be a single integer-literal token (any
//!   underscore grouping or integer suffix — both valid const lengths);
//!   identifiers, expressions and everything else refuse (a non-const
//!   length is E0277, but parity with clippy's silence is the rule);
//! - qualified macro paths (`std::vec![…]`) refuse.
//!
//! VERIFY tier (type-blind): the consumer's expected type is invisible to
//! syn — where a callee takes `&Vec<T>` exactly, the array form is E0308.
//! Real clippy is type-aware and stays silent there; every wrong fire is
//! check-visible and the compile gate reverts it (the `ptr_arg` VERIFY
//! precedent — the same `&Vec<T>` param class).

use super::SourceEdit;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, File, MacroDelimiter};

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

impl Collector<'_> {
    fn try_borrow_of_vec(&mut self, em: &ExprMacro) {
        // Unqualified `vec` only — `std::vec![..]` and aliases refuse.
        let Some(seg) = em.mac.path.segments.last() else {
            return;
        };
        if em.mac.path.segments.len() != 1 || seg.ident != "vec" || !seg.arguments.is_none() {
            return;
        }
        let MacroDelimiter::Bracket(_) = em.mac.delimiter else {
            return;
        };
        // The repeat form: a `;` token in the interior means the length
        // side follows; it must be exactly ONE integer-literal token (the
        // const-length law — a non-literal is E0277, but clippy's silence
        // on `&vec![0; n]` is the parity rule). The list form (no `;`)
        // splices verbatim.
        let tokens: Vec<_> = em.mac.tokens.clone().into_iter().collect();
        if let Some(semi) = tokens
            .iter()
            .rposition(|t| matches!(t, proc_macro2::TokenTree::Punct(p) if p.as_char() == ';'))
        {
            let len_side = &tokens[semi + 1..];
            let ok =
                len_side.len() == 1 && matches!(len_side[0], proc_macro2::TokenTree::Literal(_));
            if !ok {
                return;
            }
        }
        // Interior = the source text between the FIRST `[` and the macro
        // call's final `]` (the call ends at its delimiter group, so the
        // last byte of the span IS the closing bracket; a `]` inside a
        // string literal sits before it and survives the splice
        // untouched). The replacement covers ONLY the `vec![…]` call —
        // the caller's own `&` survives in place, so the emitted text is
        // the bracket group alone (the Expr::Reference wrapper provides
        // the borrow).
        let mac_span = em.mac.span().byte_range();
        let Some(mac_text) = self.source.get(mac_span.clone()) else {
            return;
        };
        let Some(open) = mac_text.find('[') else {
            return;
        };
        if !mac_text.ends_with(']') {
            return;
        }
        let interior = &mac_text[open + 1..mac_text.len() - 1];
        self.edits.push(SourceEdit {
            start: mac_span.start,
            end: mac_span.end,
            replacement: format!("[{interior}]"),
        });
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_reference(&mut self, e: &'ast syn::ExprReference) {
        // `&mut vec![..]` refuses (the mutability arm); plain `&` walks.
        if e.mutability.is_none() {
            if let Expr::Macro(em) = &*e.expr {
                self.try_borrow_of_vec(em);
            }
        }
        syn::visit::visit_expr_reference(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    // ── the fire arms ──

    #[test]
    fn arg_site_list_form_rewrites() {
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &[i32]) -> i32 { x.iter().sum() }\nfn probe() -> i32 { take(&vec![1, 2, 3]) }",
        )
        .expect("matched");
        assert!(out.contains("take(&[1, 2, 3])"), "{out}");
        assert!(!out.contains("vec!"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn repeat_literal_length_rewrites() {
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &[u8]) -> usize { x.len() }\nfn probe() -> usize { take(&vec![0; 8]) }",
        )
        .expect("matched");
        assert!(out.contains("take(&[0; 8])"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn repeat_underscore_and_suffix_lengths_rewrite() {
        for len in ["1_000", "8usize"] {
            let src = format!(
                "fn take(x: &[u8]) -> usize {{ x.len() }}\nfn probe() -> usize {{ take(&vec![7; {len}]) }}"
            );
            let out = propose_fix("useless_vec", &src)
                .unwrap_or_else(|| panic!("len {len} did not fire"));
            assert!(out.contains(&format!("&[7; {len}]")), "len {len}: {out}");
            syn::parse_file(&out).unwrap_or_else(|e| panic!("len {len}: {e} {out}"));
        }
    }

    #[test]
    fn interior_string_with_bracket_survives() {
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &[&str]) -> usize { x.len() }\nfn probe() -> usize { take(&vec![\"a]b\", \"c\"] ) }",
        )
        .expect("matched");
        assert!(out.contains("&[\"a]b\", \"c\"]"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn annotated_let_rewrites() {
        let out = propose_fix(
            "useless_vec",
            "fn probe() -> usize { let r: &[i32] = &vec![4, 5]; r.len() }",
        )
        .expect("matched");
        assert!(out.contains("&[4, 5]"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    // ── the refusal arms ──

    #[test]
    fn variable_repeat_length_refuses() {
        // Probed: clippy 1.99 stays silent (`[0; n]` needs a const) — the
        // parity rule, not just the compile gate.
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &[u8]) -> usize { x.len() }\nfn probe(n: usize) -> usize { take(&vec![0; n]) }",
        );
        assert!(out.is_none(), "variable length fired: {out:?}");
    }

    #[test]
    fn repeat_expr_length_refuses() {
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &[u8]) -> usize { x.len() }\nfn probe(n: usize) -> usize { take(&vec![0; n + 1]) }",
        );
        assert!(out.is_none(), "expr length fired: {out:?}");
    }

    #[test]
    fn mut_borrow_refuses() {
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &mut [i32]) {}\nfn probe() { take(&mut vec![1, 2]); }",
        );
        assert!(out.is_none(), "&mut fired: {out:?}");
    }

    #[test]
    fn qualified_path_refuses() {
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &[i32]) -> i32 { x.iter().sum() }\nfn probe() -> i32 { take(&std::vec![1, 2, 3]) }",
        );
        assert!(out.is_none(), "qualified path fired: {out:?}");
    }

    #[test]
    fn let_bound_then_borrowed_refuses() {
        // Probed: clippy 1.99 stays silent on this shape (typeck
        // territory) — not this proposer's business.
        let out = propose_fix(
            "useless_vec",
            "fn take(x: &[i32]) -> i32 { x.iter().sum() }\nfn probe() -> i32 { let v = vec![1, 2, 3]; take(&v) }",
        );
        assert!(out.is_none(), "let-bound vec fired: {out:?}");
    }

    #[test]
    fn plain_vec_expr_without_borrow_refuses() {
        let out = propose_fix("useless_vec", "fn probe() -> Vec<i32> { vec![1, 2, 3] }");
        assert!(out.is_none(), "bare vec![..] fired: {out:?}");
    }

    #[test]
    fn nested_borrow_inner_only_rewrites() {
        // The borrow-of-macro is INSIDE the call args; the surrounding
        // call text survives byte-identically.
        let src = "// header\nfn take(x: &[i32]) -> i32 { x.iter().sum() }\nfn probe() -> i32 {\n    // keep\n    take(&vec![1, 2, 3])\n}";
        let out = propose_fix("useless_vec", src).expect("matched");
        assert!(out.contains("// header\n"), "{out}");
        assert!(out.contains("    // keep\n"), "{out}");
        assert!(out.contains("take(&[1, 2, 3])"), "{out}");
        assert_eq!(src.lines().count(), out.lines().count(), "{out}");
    }
}
