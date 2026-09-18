//! `clippy::single_char_add_str` — `s.push_str("!")` → `s.push('!')`
//! (Batch 159 / Plan 150 — the frontier-lead mining pack V; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery AND the
//! default-groups arm, 2026-09-18).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! For a signature-proven `String` receiver (an owned `String` param or an
//! `&mut String` param — the `mut_string_param_heads_over` derivation,
//! never rebound), the ONLY in-scope `push_str` is `String`'s inherent
//! `fn push_str(&mut self, string: &str)` and the ONLY `push` is the
//! inherent `fn push(&mut self, ch: char)` — inherent methods cannot be
//! shadowed by a trait import, and a custom type carrying its own
//! `push_str` is excluded by the derivation itself. `push_str(c)` appends
//! exactly the UTF-8 encoding of `c`; `push(c)` appends the same bytes —
//! identical by construction for any single-`char` string VALUE.
//!
//! The arg must be a plain string literal whose VALUE is exactly one
//! `char` — escapes resolve through `LitStr::value()`, so `"\n"`,
//! `"\u{41}"` (`'A'`), and multi-byte `"\u{e9}"` all qualify, and the
//! EMIT re-escapes for the char-literal grammar via `char::escape_default`
//! (`'`/`\\`/`\n`/`\r`/`\t`/`\0` escaped; ASCII printables verbatim;
//! non-ASCII as `\u{..}` — always VALUE-identical, never textual).
//! Byte strings (`b"!"`), C strings (`c"!"`), raw strings (value-equal —
//! the emit is the escaped form), empty `""`, and multi-char literals
//! refuse.
//!
//! Shape: `receiver.push_str("c")` — a method call with EXACTLY one
//! argument, no turbofish, no attrs. The receiver must be the bare proven
//! ident (paren-wrapped receivers refuse — the emit rides the verbatim
//! text). Comments inside the deleted glue (the method name + the literal)
//! refuse — keep-list = the receiver span. Documented
//! narrower-than-clippy divergences: local `let mut s = String::new()`
//! bindings, non-param receivers (`self.buf`, `v[0]`), and the UFCS form
//! `String::push_str(&mut s, "..")` all refuse pending a local-binding
//! derivation (a custom type with its own `push_str` compiles the
//! original but may lack `push` — the hole the derivation closes).

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, ImplItemFn, ItemFn};

use super::helpers::{byte_range, drops_interior_comment, mut_string_param_heads_over};
use super::{edit, SourceEdit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        proven: BTreeMap::new(),
        saved: BTreeMap::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Signature-proven mutable-String bindings of the enclosing fn.
    proven: BTreeMap<String, String>,
    /// The set to restore on fn exit (one-deep save).
    saved: BTreeMap<String, String>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = mut_string_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = mut_string_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_expr_method_call(&mut self, c: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, c);
        if let Some(replacement) = self.try_rewrite(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, c: &ExprMethodCall) -> Option<String> {
        if c.method != "push_str" || c.args.len() != 1 {
            return None;
        }
        if c.turbofish.is_some() || !c.attrs.is_empty() {
            return None;
        }
        // The receiver must be the bare proven ident — the emit rides the
        // verbatim text, so paren-wrapped forms refuse (the
        // manual_range_contains discipline).
        let recv_text = ident_text(&c.receiver)?;
        if !self.proven.contains_key(recv_text.as_str()) {
            return None;
        }
        // Exactly one char by VALUE (escapes resolved); the emit re-escapes
        // for the char-literal grammar.
        let arg = c.args.first()?;
        let Expr::Lit(lit) = arg else { return None };
        if !lit.attrs.is_empty() {
            return None;
        }
        let syn::Lit::Str(ls) = &lit.lit else { return None };
        let value = ls.value();
        let mut chars = value.chars();
        let (c1, c2) = (chars.next(), chars.next());
        let (Some(ch), None) = (c1, c2) else { return None };
        // Comments inside the deleted glue (method name + literal) refuse;
        // the receiver text rides verbatim — its span is the keep-list.
        let recv_range = byte_range(&*c.receiver);
        if drops_interior_comment(self.source, byte_range(c), &[recv_range]) {
            return None;
        }
        Some(format!(
            "{recv_text}.push('{}')",
            ch.escape_default()
        ))
    }
}

/// The bare ident of a single-segment plain path (the per-file ident
/// helper pattern — `manual_range_contains`'s twin).
fn ident_text(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    if !p.path.segments[0].arguments.is_none() {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn mutable_string_param_fires() {
        let out = propose_fix(
            "single_char_add_str",
            "fn mark(s: &mut String) { s.push_str(\"!\"); }",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('!')"),
            "not rewritten to push: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn owned_string_param_fires() {
        let out = propose_fix(
            "single_char_add_str",
            "fn build(mut s: String) -> String { s.push_str(\"?\"); s }",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('?')"),
            "owned param not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_fires() {
        let out = propose_fix(
            "single_char_add_str",
            "struct W { n: u8 }\nimpl W {\n    fn tag(&self, s: &mut String) { s.push_str(\".\"); }\n}",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('.')"),
            "impl-method param not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn escape_arms_fire_by_value() {
        // "\n" → '\n' (the escape rides as the SAME value).
        let out = propose_fix(
            "single_char_add_str",
            "fn nl(s: &mut String) { s.push_str(\"\\n\"); }",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('\\n')"),
            "escape not value-preserved: {out}"
        );
        // "\u{41}" → 'A' (value-based emit, not textual).
        let out = propose_fix(
            "single_char_add_str",
            "fn uni(s: &mut String) { s.push_str(\"\\u{41}\"); }",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('A')"),
            "unicode escape not resolved by value: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn quote_and_backslash_arms_escape() {
        let out = propose_fix(
            "single_char_add_str",
            "fn q(s: &mut String) { s.push_str(\"'\"); }",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('\\'')"),
            "quote not escaped: {out}"
        );
        let out = propose_fix(
            "single_char_add_str",
            "fn b(s: &mut String) { s.push_str(\"\\\\\"); }",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('\\\\')"),
            "backslash not escaped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn multibyte_char_emits_unicode_escape() {
        // escape_default emits `\u{e9}` for non-ASCII — value-identical to
        // the source's `\u{e9}`/`"\u{e9}"` literal, never textual.
        let out = propose_fix(
            "single_char_add_str",
            "fn accent(s: &mut String) { s.push_str(\"\u{e9}\"); }",
        )
        .expect("matched");
        assert!(
            out.contains("s.push('\\u{e9}')"),
            "non-ASCII not emitted as the \\u escape: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn multi_char_and_empty_refuse() {
        assert!(propose_fix(
            "single_char_add_str",
            "fn two(s: &mut String) { s.push_str(\"ab\"); }",
        )
        .is_none());
        assert!(
            propose_fix(
                "single_char_add_str",
                "fn none(s: &mut String) { s.push_str(\"\"); }",
            )
            .is_none(),
            "empty literal must refuse"
        );
    }

    #[test]
    fn unproven_receiver_refuses() {
        // A custom type's push_str compiles the original but may lack push.
        assert!(propose_fix(
            "single_char_add_str",
            "struct B;\nimpl B { fn push_str(&mut self, s: &str) {} }\nfn go(b: &mut B) { b.push_str(\"!\"); }",
        )
        .is_none());
        // A local binding (String::new) — the documented divergence.
        assert!(propose_fix(
            "single_char_add_str",
            "fn go() { let mut s = String::new(); s.push_str(\"!\"); }",
        )
        .is_none());
        // A field receiver.
        assert!(propose_fix(
            "single_char_add_str",
            "struct W { buf: String }\nimpl W { fn go(&mut self) { self.buf.push_str(\"!\"); } }",
        )
        .is_none());
    }

    #[test]
    fn shared_ref_and_rebound_refuse() {
        // Shared &String is not a mutable view — refused by the derivation.
        assert!(propose_fix(
            "single_char_add_str",
            "fn peek(s: &String) { println!(\"{s}\"); }",
        )
        .is_none());
        // Rebound param — the derivation's soundness leg.
        assert!(propose_fix(
            "single_char_add_str",
            "fn go(s: &mut String) { let s = String::new(); }",
        )
        .is_none());
    }

    #[test]
    fn paren_receiver_and_two_args_refuse() {
        assert!(propose_fix(
            "single_char_add_str",
            "fn go(s: &mut String) { (s).push_str(\"!\"); }",
        )
        .is_none());
        assert!(propose_fix(
            "single_char_add_str",
            "fn go(s: &mut String, t: &str) { s.push_str(t); }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_glue_refuses() {
        assert!(propose_fix(
            "single_char_add_str",
            "fn go(s: &mut String) { s.push_str /* keep */ (\"!\"); }",
        )
        .is_none());
    }
}
