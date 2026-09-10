//! `constant-time-secret-compare` — secret equality must not ride `==`.
//!
//! A `==` comparison whose operands are plain identifiers matching the
//! secret vocabulary (`secret|token|mac|signature|key|hmac|nonce|tag`, at
//! least ONE side) leaks the matched-prefix length through timing. The
//! bounded fix swaps the operator for `subtle::ConstantTimeEq::ct_eq` —
//! same operand order, same `bool` result via `.into()`.
//!
//! ## Dep gate (the `fxhashmap` pattern)
//!
//! Fires only when the file already carries a `subtle` import (any form —
//! `use subtle::...`, `use subtle as ...`). The import proves the
//! dependency resolves in this compilation unit; the healer never edits a
//! `Cargo.toml`. A crate doing crypto hygiene WITHOUT subtle here is
//! missed (false negative = safe skip).
//!
//! ## Operand gate (strict)
//!
//! - Both operands are **plain path idents** (`provided`, `expected`) —
//!   method calls, field accesses, index exprs stay out (their types are
//!   unprovable parse-only, and `ct_eq` needs `ConstantTimeEq` on BOTH
//!   sides; `subtle` implements it for byte slices/arrays, NOT for
//!   `String`/`&str` — a type mismatch is an E0599 the compile gate
//!   reverts, but every revert is noise, so the shape gate is strict).
//! - The **enclosing fn's name** OR either operand's name hits the secret
//!   vocabulary (word-boundary, case-insensitive, split on `_`). Real code
//!   names fns `verify_token`/`check_mac` with generic operands
//!   (`provided == expected`) — the fn-name signal is the primary one. A
//!   generic `a == b` outside secret-named code never fires.
//! - The comparison is **not** inside its own `assert!`/`assert_eq!`-style
//!   macro guard (test code asserting on tokens is not a timing channel).
//! - `!=` is NOT rewritten (negated form would need `!(a.ct_eq(b))` —
//!   same guard cost, half the frequency; kept out for Phase 2).
//!
//! ## Honest caveats
//!
//! - `ct_eq` needs both operands to implement `ConstantTimeEq` — `subtle`
//!   impls byte slices + arrays + `T: ConstantTimeEq` wrappers, not
//!   `String`. The `--fix --verify` compile gate is the backstop.
//! - A byte-equality that is NOT a secret (a lookup key named `token`)
//!   pays a branch-free compare — correct but marginally slower. The name
//!   gate bounds the blast radius; the review line carries the rule id.
//! - Length differences: `ct_eq` on slices of different length returns
//!   `Choice(0)` immediately (the lengths are public in the types) —
//!   same observable as `==` for slice pairs.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{BinOp, Expr, ExprBinary, File};

use super::SourceEdit;

/// Secret-vocabulary segments (case-insensitive, split on `_`).
const SECRET_WORDS: &[&str] = &[
    "secret",
    "token",
    "mac",
    "signature",
    "sig",
    "key",
    "hmac",
    "nonce",
    "tag",
    "digest",
];

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    // Dep gate: the file must already import `subtle` in some form.
    if !imports_subtle(file) {
        return Vec::new();
    }
    let mut edits = Vec::new();
    let mut walker = Walker {
        source,
        edits: &mut edits,
        in_secret_fn: false,
    };
    walker.visit_file(file);
    edits
}

/// Any `use ... subtle ...` import path proves the dep resolves.
fn imports_subtle(file: &File) -> bool {
    use syn::visit::Visit;
    struct ImportFinder(bool);
    impl<'ast> Visit<'ast> for ImportFinder {
        fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
            let ts: TokenStream = u.tree.to_token_stream();
            if ts
                .to_string()
                .split_whitespace()
                .any(|seg| seg.trim_end_matches(',') == "subtle")
            {
                self.0 = true;
            }
        }
    }
    let mut f = ImportFinder(false);
    f.visit_file(file);
    f.0
}

fn ident_segments(ident: &syn::Ident) -> Vec<String> {
    ident
        .to_string()
        .to_ascii_lowercase()
        .split('_')
        .map(str::to_string)
        .collect()
}

fn is_secret_named(ident: &syn::Ident) -> bool {
    ident_segments(ident)
        .iter()
        .any(|s| SECRET_WORDS.contains(&s.as_str()))
}

/// Both operands must be plain path idents (no fields/methods/index).
fn plain_ident(e: &Expr) -> Option<&syn::Ident> {
    if let Expr::Path(p) = e {
        if p.qself.is_none() && p.path.segments.len() == 1 && p.path.get_ident().is_some() {
            return p.path.get_ident();
        }
    }
    None
}

struct Walker<'a> {
    source: &'a str,
    edits: &'a mut Vec<SourceEdit>,
    /// Whether the current traversal context (enclosing fn) is
    /// secret-named — set by `SkipTests` when it enters an item fn.
    in_secret_fn: bool,
}

impl Walker<'_> {
    fn visit_file(&mut self, file: &File) {
        use syn::visit::Visit;
        // Skip test modules: assertions on tokens live there by design.
        struct SkipTests<'a, 'b> {
            walker: &'a mut Walker<'b>,
        }
        impl<'ast> Visit<'ast> for SkipTests<'_, '_> {
            fn visit_expr_binary(&mut self, b: &'ast ExprBinary) {
                self.walker.visit_binary(b);
                syn::visit::visit_expr_binary(self, b);
            }
            fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
                let was = self.walker.in_secret_fn;
                if is_secret_named(&f.sig.ident) {
                    self.walker.in_secret_fn = true;
                }
                syn::visit::visit_item_fn(self, f);
                self.walker.in_secret_fn = was;
            }
            fn visit_impl_item_fn(&mut self, m: &'ast syn::ImplItemFn) {
                let was = self.walker.in_secret_fn;
                if is_secret_named(&m.sig.ident) {
                    self.walker.in_secret_fn = true;
                }
                syn::visit::visit_impl_item_fn(self, m);
                self.walker.in_secret_fn = was;
            }
            fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
                // #[cfg(test)] mod tests { ... } — assert-heavy, skip.
                let is_test = m.attrs.iter().any(|a| {
                    a.path().is_ident("cfg") && {
                        let ts: TokenStream = a.meta.to_token_stream();
                        ts.to_string().contains("test")
                    }
                });
                if is_test {
                    return;
                }
                syn::visit::visit_item_mod(self, m);
            }
        }
        SkipTests { walker: self }.visit_file(file);
    }

    fn visit_binary(&mut self, b: &ExprBinary) {
        if !matches!(b.op, BinOp::Eq(_)) {
            return;
        }
        let (Some(l), Some(r)) = (plain_ident(&b.left), plain_ident(&b.right)) else {
            return;
        };
        if !(self.in_secret_fn || is_secret_named(l) || is_secret_named(r)) {
            return;
        }
        let replacement = format!("{l}.ct_eq({r}).into()");
        let range = b.span().byte_range();
        if range.end <= self.source.len() {
            self.edits.push(SourceEdit {
                start: range.start,
                end: range.end,
                replacement,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::collect_edits;

    const IMPORT: &str = "use subtle::ConstantTimeEq;\n";

    fn edits(src: &str) -> Vec<super::SourceEdit> {
        let file = syn::parse_file(src).expect("fixture parses");
        collect_edits(src, &file)
    }

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn secret_eq_rewrites() {
        let src = format!(
            "{IMPORT}\npub fn check_token(provided: &[u8], expected: &[u8]) -> bool {{\n    \
             provided == expected\n}}\n"
        );
        let edits = edits(&src);
        assert_eq!(edits.len(), 1, "one rewrite expected");
        assert_eq!(
            squash(&edits[0].replacement),
            "provided.ct_eq(expected).into()"
        );
        // The fixed form parses.
        let spliced = format!(
            "{}\npub fn check_token(provided: &[u8], expected: &[u8]) -> bool {{\n    \
             {}\n}}\n",
            IMPORT, edits[0].replacement
        );
        assert!(syn::parse_file(&spliced).is_ok());
    }

    #[test]
    fn no_subtle_import_is_silent() {
        // The dep gate: a file without a subtle import never fires (the
        // fxhashmap E0432 class — the rewrite cannot resolve without the dep).
        let src = "pub fn check_token(provided: &[u8], expected: &[u8]) -> bool {\n    \
             provided == expected\n}\n";
        assert!(edits(src).is_empty());
    }

    #[test]
    fn non_secret_names_are_silent() {
        // The vocabulary gate: a generic a == b (or a lookup-key pair with
        // no secret segment) never fires.
        let src = format!("{IMPORT}\npub fn same(a: &[u8], b: &[u8]) -> bool {{\n    a == b\n}}\n");
        assert!(edits(&src).is_empty());
    }

    #[test]
    fn non_plain_operands_are_silent() {
        // Field/method/index operands: the types are unprovable parse-only,
        // and ct_eq needs ConstantTimeEq on both sides — strict skip.
        let src = format!(
            "{IMPORT}\nstruct Pair {{ tok: Vec<u8> }}\npub fn f(p: &Pair, e: &[u8]) -> \
             bool {{\n    p.tok == e\n}}\n"
        );
        assert!(edits(&src).is_empty());
    }

    #[test]
    fn ne_is_not_rewritten() {
        // Phase-2 scope: only Eq; the negated form stays out (half the
        // frequency, full guard cost).
        let src = format!(
            "{IMPORT}\npub fn check_token(provided: &[u8], expected: &[u8]) -> bool {{\n    \
             provided != expected\n}}\n"
        );
        assert!(edits(&src).is_empty());
    }

    #[test]
    fn test_mod_is_skipped() {
        // Assertions on tokens live in #[cfg(test)] mods by design — not a
        // production timing channel.
        let src = format!(
            "{IMPORT}\n#[cfg(test)]\nmod tests {{\n    #[test]\n    fn t() {{\n        \
             let token = b\"x\";\n        assert!(token == b\"x\");\n    }}\n}}\n"
        );
        assert!(edits(&src).is_empty());
    }

    #[test]
    fn word_boundary_respected() {
        // "monkey" contains "key" as a SUBSTRING but not as a _-segment —
        // the vocabulary match is word-boundary (split on _), not substring.
        let src = format!(
            "{IMPORT}\npub fn f(monkey: &[u8], other: &[u8]) -> bool {{\n    monkey == \
             other\n}}\n"
        );
        assert!(edits(&src).is_empty());
    }

    #[test]
    fn aliased_import_enables() {
        // `use subtle as st;` still proves the dep resolves.
        let src = "use subtle as st;\npub fn check_token(provided: &[u8], expected: \
             &[u8]) -> bool {\n    provided == expected\n}\n";
        assert_eq!(edits(src).len(), 1);
    }

    #[test]
    fn string_operands_fire_but_compile_gate_owns_them() {
        // String-typed secrets (a session token) MATCH the vocabulary and
        // the plain-ident gate — but subtle has no ConstantTimeEq for
        // String, so the swap is an E0599 the --fix --verify compile gate
        // reverts. Recorded honestly: the fixture pins the FIRE (the
        // proposer is shape-based); the compile gate is the type backstop.
        let src = format!(
            "{IMPORT}\npub fn check(session_token: &str, expected: &str) -> bool {{\n    \
             session_token == expected\n}}\n"
        );
        assert_eq!(edits(&src).len(), 1);
    }
}
