//! `clippy::comparison_to_empty` — `s == ""` → `s.is_empty()`; also `!=`
//! (→ `!s.is_empty()`) and the reversed `"" == s` shape (Issue 022: this is
//! the REAL `comparison_to_empty` shape, verified against real clippy).
//! The `.len() == 0` form is real clippy's `len_zero` — still detected +
//! healed here (same fix either way; it stays useful for workspace healing).
//! Type-blind on `is_empty` existence — rustc backs the revert under
//! `--verify` (VERIFY-tier).
//!
//! Self-recursion guard (Issue 031 H1): when the receiver is exactly `self`
//! AND the innermost enclosing function is `is_empty`, the rewrite
//! (`self.len() == 0` → `self.is_empty()`) would be infinite recursion — a
//! runtime stack overflow the `--verify` compile gate is structurally blind
//! to (it compiles fine). Real clippy stays silent on this shape too.
//! `self == ""` inside `fn is_empty` is the same recursion (Form 1).
//! Non-`self` receivers inside `is_empty` still fire (different object);
//! `self.len() == 0` inside any other fn still fires.
//!
//! Any integer-suffix spelling of zero fires (`0`, `0usize`).
//! Span-preserving (T6.4): the whole binary expression's byte range is
//! replaced by `<R>.is_empty()` / `!<R>.is_empty()` — no parens after `!`,
//! the method call binds tighter.

use super::helpers::{int_lit, is_postfix_safe};
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprMethodCall, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        fn_stack: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Innermost-last stack of enclosing function names (Issue 031 H1).
    fn_stack: Vec<String>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        syn::visit::visit_item_fn(self, i);
        self.fn_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, i);
        self.fn_stack.pop();
    }

    fn visit_trait_item_fn(&mut self, i: &'ast syn::TraitItemFn) {
        self.fn_stack.push(i.sig.ident.to_string());
        syn::visit::visit_trait_item_fn(self, i);
        self.fn_stack.pop();
    }
    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        syn::visit::visit_expr_binary(self, b);
        let negated = match b.op {
            BinOp::Eq(_) => false,
            BinOp::Ne(_) => true,
            _ => return, // ordering comparisons (`>= 0`) mean something else
        };
        if !b.attrs.is_empty() {
            return; // attrs would be dropped by the splice
        }
        // Form 1 (real `comparison_to_empty`): empty-string literal on one
        // side, any expression on the other — either order.
        if let Some(operand) = empty_literal_other_side(&b.left, &b.right) {
            if let Some(replacement) = self.expr_is_empty_text(operand, negated) {
                self.edits.push(edit(b, replacement));
            }
            return;
        }
        // Form 2 (real clippy calls this `len_zero`): `len()` on one side,
        // literal `0` on the other — either order.
        let len_call = if let Some(call) = bare_len_call(&b.left) {
            if is_zero(&b.right) {
                call
            } else {
                return;
            }
        } else if let Some(call) = bare_len_call(&b.right) {
            if is_zero(&b.left) {
                call
            } else {
                return;
            }
        } else {
            return;
        };
        if let Some(replacement) = self.is_empty_text(len_call, negated) {
            self.edits.push(edit(b, replacement));
        }
    }
}

impl Collector<'_> {
    /// Issue 031 H1: `self.<expr>` rewritten to `self.is_empty()` inside
    /// `fn is_empty` is infinite recursion — refuse the edit.
    fn would_self_recurse(&self, recv: &str) -> bool {
        recv == "self" && self.fn_stack.last().is_some_and(|n| n == "is_empty")
    }

    fn is_empty_text(&self, len: &ExprMethodCall, negated: bool) -> Option<String> {
        let recv = src(self.source, &len.receiver)?;
        if self.would_self_recurse(&recv) {
            return None;
        }
        Some(finish_is_empty(&recv, &len.receiver, negated))
    }

    fn expr_is_empty_text(&self, operand: &Expr, negated: bool) -> Option<String> {
        let recv = src(self.source, operand)?;
        if self.would_self_recurse(&recv) {
            return None;
        }
        Some(finish_is_empty(&recv, operand, negated))
    }
}

/// Wrap `recv` for the `.is_empty()` replacement — parenthesized unless the
/// operand is postfix-safe.
fn finish_is_empty(recv: &str, operand: &Expr, negated: bool) -> String {
    let recv = if is_postfix_safe(operand) {
        recv.to_string()
    } else {
        format!("({recv})")
    };
    if negated {
        format!("!{recv}.is_empty()")
    } else {
        format!("{recv}.is_empty()")
    }
}

/// The side if it is a plain `.len()` call (0 args, no attrs/turbofish).
fn bare_len_call(e: &Expr) -> Option<&ExprMethodCall> {
    let Expr::MethodCall(m) = e else {
        return None;
    };
    if m.method == "len" && m.args.is_empty() && m.attrs.is_empty() && m.turbofish.is_none() {
        Some(m)
    } else {
        None
    }
}

/// If one side is an empty-string literal (`""`), return the OTHER side
/// (the operand whose `is_empty()` we call) — either order.
fn empty_literal_other_side<'e>(left: &'e Expr, right: &'e Expr) -> Option<&'e Expr> {
    match (is_empty_str_lit(left), is_empty_str_lit(right)) {
        (true, false) => Some(right),
        (false, true) => Some(left),
        _ => None, // neither, or both literals (`"" == ""` — leave alone)
    }
}

/// Is this an empty string literal? (Raw/byte-string forms count: `value()`
/// is semantic.)
fn is_empty_str_lit(e: &Expr) -> bool {
    let Expr::Lit(l) = e else {
        return false;
    };
    matches!(&l.lit, syn::Lit::Str(s) if s.value().is_empty())
}

/// Is this the literal `0` (any integer suffix)?
fn is_zero(e: &Expr) -> bool {
    int_lit(e).is_some_and(|(digits, _)| digits == "0")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_equality_to_empty() {
        let input = "fn f(v: &Vec<u8>) -> bool { v.len() == 0 }";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(out.contains("v.is_empty()"), "is_empty not produced: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn converts_empty_str_comparison() {
        // The REAL comparison_to_empty shape (Issue 022; verified against
        // real clippy 1.97).
        let input = "fn f(s: &str) -> bool { s == \"\" }";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(out.contains("s.is_empty()"), "is_empty not produced: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn converts_reversed_empty_str_with_negation() {
        let input = "fn f(s: &str) -> bool { \"\" != s }";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(
            out.contains("!s.is_empty()"),
            "reversed negated form not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nonempty_str_comparison_unchanged() {
        let input = "fn f(s: &str) -> bool { s == \"x\" }";
        let out = propose_fix("comparison_to_empty", input);
        assert!(out.is_none(), "non-empty literal must not fire: {out:?}");
    }

    #[test]
    fn converts_inequality_with_negation() {
        let input = "fn g(v: &Vec<u8>) -> bool { v.len() != 0 }";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(
            out.contains("!v.is_empty()"),
            "negated form not produced: {out}"
        );
    }

    #[test]
    fn converts_reversed_operands() {
        let input = "fn h(v: &Vec<u8>) -> bool { 0 == v.len() }";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(
            out.contains("v.is_empty()"),
            "reversed form not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nonzero_comparison_unchanged() {
        let input = "fn f(v: &Vec<u8>) -> bool { v.len() == 1 }";
        let out = propose_fix("comparison_to_empty", input);
        assert!(out.is_none(), "len() == 1 must not fire: {out:?}");
    }

    #[test]
    fn ordering_comparison_unchanged() {
        let input = "fn g(v: &Vec<u8>) -> bool { v.len() >= 0 }";
        let out = propose_fix("comparison_to_empty", input);
        assert!(out.is_none(), ">= 0 must not fire: {out:?}");
    }

    #[test]
    fn self_recursion_guard_len_form() {
        // Issue 031 H1: `self.len() == 0` inside `fn is_empty` rewritten to
        // `self.is_empty()` is infinite recursion (runtime stack overflow,
        // invisible to the compile gate). Must stay silent.
        let input = "struct S;\nimpl S {\n    pub fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n}";
        let out = propose_fix("comparison_to_empty", input);
        assert!(out.is_none(), "self-recursion shape must not fire: {out:?}");
    }

    #[test]
    fn self_recursion_guard_str_form() {
        // Form 1 twin: `self == ""` inside `fn is_empty` is the same
        // recursion.
        let input =
            "struct S;\nimpl S {\n    fn is_empty(&self) -> bool {\n        self == \"\"\n    }\n}";
        let out = propose_fix("comparison_to_empty", input);
        assert!(
            out.is_none(),
            "self-recursion str form must not fire: {out:?}"
        );
    }

    #[test]
    fn self_recursion_guard_trait_default_body() {
        // Trait default bodies are the same class (visit_trait_item_fn).
        let input =
            "trait T {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n}";
        let out = propose_fix("comparison_to_empty", input);
        assert!(out.is_none(), "trait default body must not fire: {out:?}");
    }

    #[test]
    fn other_receiver_inside_is_empty_still_fires() {
        // Positive control: non-`self` receiver inside `fn is_empty` is a
        // different object — no recursion — and must still fire.
        let input = "struct S;\nimpl S {\n    fn is_empty(&self, other: &Vec<u8>) -> bool {\n        other.len() == 0\n    }\n}";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(
            out.contains("other.is_empty()"),
            "other-receiver form lost: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn self_receiver_outside_is_empty_still_fires() {
        // Positive control: `self` receiver inside a differently-named fn
        // must still fire.
        let input =
            "struct S;\nimpl S {\n    fn check(&self) -> bool {\n        self.len() == 0\n    }\n}";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(
            out.contains("self.is_empty()"),
            "self-receiver outside is_empty lost: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn cast_receiver_parenthesizes_p11() {
        // P11: method calls bind TIGHTER than `as` — `foo() as &str.is_empty()`
        // groups the method into the TYPE position and fails to compile. The
        // shared `is_postfix_safe` excludes casts, so a cast receiver must
        // parenthesize.
        let input = "fn f() -> bool { foo() as &str == \"\" }";
        let out = propose_fix("comparison_to_empty", input).expect("matched");
        assert!(
            out.contains("(foo() as &str).is_empty()"),
            "cast receiver must parenthesize: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}
