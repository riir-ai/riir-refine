//! `clippy::comparison_to_empty` — `v.len() == 0` → `v.is_empty()`; also
//! `!=` (→ `!v.is_empty()`) and the reversed `0 == v.len()` shape.
//!
//! VERIFY-tier: `is_empty` existence is type-blind — a custom type could
//! have `len()` without `is_empty()`; rustc backs the revert under
//! `--verify`.
//!
//! Any integer-suffix spelling of zero fires (`0`, `0usize`).
//! Span-preserving (T6.4): the whole binary expression's byte range is
//! replaced by `<R>.is_empty()` / `!<R>.is_empty()` — no parens after `!`,
//! the method call binds tighter.

use super::helpers::{int_lit, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprMethodCall, File};

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
        // `len()` on one side, literal `0` on the other — either order.
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
    fn is_empty_text(&self, len: &ExprMethodCall, negated: bool) -> Option<String> {
        let recv = src(self.source, &*len.receiver)?;
        let recv = if is_postfix_safe(&len.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        Some(if negated {
            format!("!{recv}.is_empty()")
        } else {
            format!("{recv}.is_empty()")
        })
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
}
